//! task-16.1 (Phase 16 P4 #10): SQLite persistence for the TraceStore that
//! backs `GET /v1/search/{query_id}/trace` and `GET /v1/queries` (task-12.3
//! + task-15.5).
//!
//! Storage model: each row holds a base64-encoded prost-serialized
//! `RetrievalTrace` plus the wrapper metadata (`workspace_id`, `ts_unix`)
//! task-15.5 surfaces via `QueryRecord`. Schema lives in
//! `core/migrations/0015_search_traces.sql`; `Connection::open` triggers
//! the migration on every daemon boot (IF NOT EXISTS, idempotent).
//!
//! Concurrency: `std::sync::Mutex<Connection>` — rusqlite `Connection` is
//! `!Send` by default and the trace store is already `Arc<Mutex<TraceStore>>`,
//! so contention is already bounded by the in-memory LRU lock.
//!
//! task-16.1 §3 trade-off: the schema is **internal** — not exposed via
//! contractv1 / proto (ADR-015 D1 add-only). Console UI keeps consuming
//! the existing `RetrievalTrace` proto message; persistence is invisible.

use std::path::Path;
use std::sync::Mutex;

use base64::Engine as _;
use prost::Message;
use rusqlite::{params, Connection, OptionalExtension};

use crate::pb_console::{QueryRecord as PbQueryRecord, RetrievalTrace as PbRetrievalTrace};

const MIGRATION_SQL: &str = include_str!("../../migrations/0015_search_traces.sql");

/// task-16.1: persistent backing store for `TraceStore` (in `search.rs`).
///
/// Wraps `<data_dir>/search_traces.db`. All API are blocking + synchronous;
/// callers are expected to invoke from a sync context (TraceStore's
/// `Mutex<TraceStore>` already serializes access).
pub struct SqliteTracePersist {
    conn: Mutex<Connection>,
}

#[derive(Debug)]
pub enum SqliteTracePersistError {
    Sqlite(rusqlite::Error),
    Codec(String),
    Poisoned,
    Io(std::io::Error),
}

impl std::fmt::Display for SqliteTracePersistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SqliteTracePersistError::Sqlite(e) => write!(f, "sqlite: {e}"),
            SqliteTracePersistError::Codec(m) => write!(f, "codec: {m}"),
            SqliteTracePersistError::Poisoned => write!(f, "trace persist mutex poisoned"),
            SqliteTracePersistError::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for SqliteTracePersistError {}

impl From<rusqlite::Error> for SqliteTracePersistError {
    fn from(e: rusqlite::Error) -> Self {
        SqliteTracePersistError::Sqlite(e)
    }
}

impl From<std::io::Error> for SqliteTracePersistError {
    fn from(e: std::io::Error) -> Self {
        SqliteTracePersistError::Io(e)
    }
}

impl SqliteTracePersist {
    /// Open or create `<data_dir>/search_traces.db`. Runs the migration
    /// idempotently. Caller is responsible for ensuring `data_dir` is
    /// writable.
    pub fn open(data_dir: &Path) -> Result<Self, SqliteTracePersistError> {
        std::fs::create_dir_all(data_dir)?;
        let path = data_dir.join("search_traces.db");
        let conn = Connection::open(&path)?;
        conn.execute_batch(MIGRATION_SQL)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// INSERT OR REPLACE one trace. Same key → row replaced (preserves the
    /// in-memory LRU "refresh recency" semantic on duplicate puts).
    pub fn put(
        &self,
        key: &str,
        trace: &PbRetrievalTrace,
        workspace_id: &str,
        ts_unix: i64,
    ) -> Result<(), SqliteTracePersistError> {
        let trace_json = encode_trace(trace);
        let conn = self.conn.lock().map_err(|_| SqliteTracePersistError::Poisoned)?;
        conn.execute(
            "INSERT OR REPLACE INTO search_traces (query_id, trace_json, workspace_id, ts_unix) \
             VALUES (?1, ?2, ?3, ?4)",
            params![key, trace_json, workspace_id, ts_unix],
        )?;
        Ok(())
    }

    /// Get one trace by query_id. Returns `Ok(None)` on miss.
    pub fn get(&self, key: &str) -> Result<Option<PbRetrievalTrace>, SqliteTracePersistError> {
        let conn = self.conn.lock().map_err(|_| SqliteTracePersistError::Poisoned)?;
        let row: Option<String> = conn
            .query_row(
                "SELECT trace_json FROM search_traces WHERE query_id = ?1",
                params![key],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        match row {
            Some(s) => decode_trace(&s).map(Some),
            None => Ok(None),
        }
    }

    /// List the most-recent N records as `QueryRecord` (matches the
    /// `RetrievalTrace.query` + workspace_id/ts_unix metadata Rust-side
    /// per task-15.5). `limit` clamped 1..=100 to bound result size.
    pub fn list(&self, limit: usize) -> Result<Vec<PbQueryRecord>, SqliteTracePersistError> {
        let conn = self.conn.lock().map_err(|_| SqliteTracePersistError::Poisoned)?;
        let lim = limit.clamp(1, 100) as i64;
        let mut stmt = conn.prepare(
            "SELECT query_id, trace_json, workspace_id, ts_unix \
             FROM search_traces ORDER BY ts_unix DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![lim], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })?;
        let mut out = Vec::with_capacity(lim as usize);
        for r in rows {
            let (key, trace_json, workspace_id, ts_unix) = r?;
            let trace = decode_trace(&trace_json)?;
            out.push(PbQueryRecord {
                query_id: key,
                query: trace.query,
                ts_unix,
                workspace_id,
            });
        }
        Ok(out)
    }

    /// task-16.1 warm restore — pull the most-recent N records and reverse
    /// to oldest-first so the caller can re-insert into the LRU VecDeque
    /// in insertion-order (newest ends up at the back, matching real-time
    /// `put` behavior).
    ///
    /// Real-time use case: insertion order == ts_unix DESC, so reverse +
    /// re-insert restores the LRU exactly. Backfill / out-of-order import
    /// edge case is deferred — see task-16.1 §8.
    pub fn load_warm(
        &self,
        n: usize,
    ) -> Result<Vec<(String, PbRetrievalTrace, String, i64)>, SqliteTracePersistError> {
        let conn = self.conn.lock().map_err(|_| SqliteTracePersistError::Poisoned)?;
        let lim = n.min(1000) as i64;
        let mut stmt = conn.prepare(
            "SELECT query_id, trace_json, workspace_id, ts_unix \
             FROM search_traces ORDER BY ts_unix DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![lim], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })?;
        let mut out: Vec<(String, PbRetrievalTrace, String, i64)> = Vec::new();
        for r in rows {
            let (key, trace_json, workspace_id, ts_unix) = r?;
            let trace = decode_trace(&trace_json)?;
            out.push((key, trace, workspace_id, ts_unix));
        }
        // Reverse: load_warm returns newest-first from SQL, but the caller
        // wants oldest-first so the LRU back position lands on the newest
        // after re-insertion.
        out.reverse();
        Ok(out)
    }

    /// Count rows — testing aid; not used by the production hot path.
    #[cfg(test)]
    pub(crate) fn row_count(&self) -> Result<i64, SqliteTracePersistError> {
        let conn = self.conn.lock().map_err(|_| SqliteTracePersistError::Poisoned)?;
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM search_traces", [], |r| r.get(0))?;
        Ok(n)
    }
}

fn encode_trace(t: &PbRetrievalTrace) -> String {
    let bytes = t.encode_to_vec();
    base64::engine::general_purpose::STANDARD.encode(&bytes)
}

fn decode_trace(s: &str) -> Result<PbRetrievalTrace, SqliteTracePersistError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| SqliteTracePersistError::Codec(format!("base64 decode: {e}")))?;
    PbRetrievalTrace::decode(bytes.as_slice())
        .map_err(|e| SqliteTracePersistError::Codec(format!("prost decode: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "cf-trace-persist-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn fixture_trace(query: &str) -> PbRetrievalTrace {
        PbRetrievalTrace {
            trace_id: format!("trace-{query}"),
            query: query.to_string(),
            expanded_query: None,
            candidate_generation_steps: vec!["tantivy-bm25".to_string()],
            lexical_candidates_count: 3,
            vector_candidates_count: 0,
            rerank_steps: vec![],
            scope_filter_result: "no-op".to_string(),
            final_context_count: 3,
            retrieved_chunks: vec![],
        }
    }

    /// AC1 verification: open + migration creates the table + index.
    #[test]
    fn test_open_creates_search_traces_table() {
        let dir = temp_dir("open");
        let persist = SqliteTracePersist::open(&dir).expect("open ok");
        // INSERT must succeed against the newly-created table.
        persist
            .put("qry-1", &fixture_trace("hello"), "ws-1", 1_700_000_000)
            .expect("put after open ok");
        assert_eq!(persist.row_count().unwrap(), 1);
        // Idempotent re-open against same file → no schema corruption.
        drop(persist);
        let persist2 = SqliteTracePersist::open(&dir).expect("re-open ok");
        assert_eq!(persist2.row_count().unwrap(), 1);
    }

    /// AC2 unit verification: put + get roundtrip preserves trace fields.
    #[test]
    fn test_put_then_get_roundtrip_preserves_trace() {
        let dir = temp_dir("get");
        let persist = SqliteTracePersist::open(&dir).expect("open ok");
        let t = fixture_trace("hello world");
        persist
            .put("qry-rt", &t, "ws-rt", 1_700_000_100)
            .expect("put ok");
        let got = persist.get("qry-rt").expect("get ok").expect("present");
        assert_eq!(got.trace_id, t.trace_id);
        assert_eq!(got.query, t.query);
        assert_eq!(
            got.candidate_generation_steps,
            t.candidate_generation_steps
        );
        assert_eq!(got.lexical_candidates_count, t.lexical_candidates_count);
        assert_eq!(got.final_context_count, t.final_context_count);
        // Miss → None.
        let missed = persist.get("qry-nope").expect("get ok");
        assert!(missed.is_none());
    }

    /// AC2 unit verification: list returns most-recent first by ts_unix DESC,
    /// honors the `limit` clamp, and projects to PbQueryRecord shape.
    #[test]
    fn test_put_then_list_returns_desc_by_ts_clamped_to_100() {
        let dir = temp_dir("list");
        let persist = SqliteTracePersist::open(&dir).expect("open ok");
        // 5 traces at ts 100 / 200 / 300 / 400 / 500.
        for i in 0..5i64 {
            let key = format!("qry-{i}");
            persist
                .put(&key, &fixture_trace(&key), "ws-list", 100 * (i + 1))
                .unwrap();
        }
        // Default-ish limit 3 → newest 3 (ts 500, 400, 300).
        let got = persist.list(3).expect("list ok");
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].query_id, "qry-4");
        assert_eq!(got[0].ts_unix, 500);
        assert_eq!(got[1].query_id, "qry-3");
        assert_eq!(got[1].ts_unix, 400);
        assert_eq!(got[2].query_id, "qry-2");
        assert_eq!(got[2].ts_unix, 300);
        // Clamp test: 0 → at least 1; 200 → at most 100. (Empty store
        // separately verifies the bound; here we just exercise clamp upper.)
        let zero_lim = persist.list(0).expect("list ok");
        assert!(!zero_lim.is_empty(), "limit 0 clamps to 1");
        // limit upper-bound: synthesize 110 rows then assert clamp 100.
        for i in 100..210i64 {
            let key = format!("qry-{i}");
            persist
                .put(&key, &fixture_trace(&key), "ws-list", i)
                .unwrap();
        }
        let upper = persist.list(200).expect("list ok");
        assert_eq!(upper.len(), 100, "clamp upper to 100");
    }

    /// AC3 unit verification: load_warm returns oldest-first so a downstream
    /// LRU re-insert lands the newest at the back.
    #[test]
    fn test_load_warm_returns_recent_n_oldest_first() {
        let dir = temp_dir("warm");
        let persist = SqliteTracePersist::open(&dir).expect("open ok");
        // 5 traces ts 100, 200, 300, 400, 500.
        for i in 0..5i64 {
            let key = format!("qry-{i}");
            persist
                .put(&key, &fixture_trace(&key), "ws-warm", 100 * (i + 1))
                .unwrap();
        }
        let warm = persist.load_warm(3).expect("warm ok");
        assert_eq!(warm.len(), 3);
        // After internal reverse: oldest-first order across the top-3 newest.
        // SQL ORDER BY ts_unix DESC LIMIT 3 → [500, 400, 300]; reverse →
        // [300, 400, 500].
        assert_eq!(warm[0].3, 300, "oldest of top-3 first");
        assert_eq!(warm[1].3, 400);
        assert_eq!(warm[2].3, 500, "newest at back for LRU push_back");
        // Bound test: load_warm caps at 1000.
        let huge = persist.load_warm(2000).expect("warm ok");
        assert!(huge.len() <= 1000);
    }

    /// Smoke test: API surface returns Result and does not panic on
    /// degenerate input (empty strings, ts_unix = 0). The schema does not
    /// NOT NULL-constrain text columns beyond the `NOT NULL` declaration
    /// itself, so empty strings INSERT successfully — this test verifies
    /// the API contract, not error-path handling.
    ///
    /// **Note**: The TraceStore-level AC4 invariant (hot cache intact even
    /// when persist errors) lives in `search.rs::tests::test_trace_store_put_
    /// hot_cache_intact_even_after_persist_failure` — see PR #110 review
    /// follow-up for the actual error-path coverage.
    #[test]
    fn test_put_with_degenerate_inputs_does_not_panic() {
        let dir = temp_dir("degen");
        let persist = SqliteTracePersist::open(&dir).expect("open ok");
        // Empty strings + ts=0 → schema accepts (no CHECK constraint
        // beyond NOT NULL). API returns Ok; no panic.
        persist
            .put("", &fixture_trace(""), "", 0)
            .expect("degenerate put ok");
        // Re-put under the same (empty) key → INSERT OR REPLACE replaces row.
        persist
            .put("", &fixture_trace("again"), "ws-2", 1)
            .expect("re-put same key ok");
        assert_eq!(persist.row_count().unwrap(), 1, "same key replaced not duplicated");
    }
}
