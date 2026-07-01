//! task-33.3 (Phase 33 / ADR-038 D3): persistent indexing-event store — a
//! replay source for `indexing.*` lifecycle events.
//!
//! Phase 11/26 emit `indexing.progress` / `.cancelled` / `.error` to the
//! in-memory `EventBus` only (best-effort broadcast, lost on restart). The
//! `audit_log` replay source (used by `events::replay_events_from_audit`)
//! cannot carry indexing events because `AuditLogEntry` has no
//! `job_id` / `processed` / `total` columns. This dedicated table mirrors the
//! indexing lifecycle 1:1 so `events::indexing_rows_to_pb_events` can rebuild
//! the `indexing.*` event sequence after a restart.
//!
//! Concurrency: `std::sync::Mutex<Connection>` (mirrors `SqliteTracePersist`).
//! All API are blocking + synchronous; the emit points call them best-effort
//! (a write failure must not block indexing — same contract as the existing
//! best-effort `EventBus::send`).
//!
//! End-to-end restart-then-replay (live daemon + job runner) is
//! `[SPEC-DEFER:phase-future.indexing-replay-e2e]`; this module delivers the
//! persistent source + read API (unit-tested round-trip), and the pure mapper
//! lives in `events::indexing_rows_to_pb_events`.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection};

const MIGRATION_SQL: &str = include_str!("../../migrations/0019_indexing_events.sql");

/// One persisted indexing-lifecycle row (id ASC = chronological). The mapper
/// `events::indexing_rows_to_pb_events` reconstructs `indexing.*` `PbEvent`s
/// from these — `job_id` / `processed` / `total` are taken verbatim (never
/// synthesized, ADR-013).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexingEventRow {
    pub id: i64,
    pub job_id: String,
    /// "indexing" (progress) | "cancelled" | "error".
    pub stage: String,
    pub processed: i64,
    pub total: i64,
    pub message: String,
    pub ts_unix: i64,
}

#[derive(Debug)]
pub enum IndexingEventStoreError {
    Sqlite(rusqlite::Error),
    Poisoned,
    Io(std::io::Error),
}

impl std::fmt::Display for IndexingEventStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexingEventStoreError::Sqlite(e) => write!(f, "sqlite: {e}"),
            IndexingEventStoreError::Poisoned => write!(f, "indexing event store mutex poisoned"),
            IndexingEventStoreError::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for IndexingEventStoreError {}

impl From<rusqlite::Error> for IndexingEventStoreError {
    fn from(e: rusqlite::Error) -> Self {
        IndexingEventStoreError::Sqlite(e)
    }
}

impl From<std::io::Error> for IndexingEventStoreError {
    fn from(e: std::io::Error) -> Self {
        IndexingEventStoreError::Io(e)
    }
}

impl SqliteIndexingEventStore {
    /// Open or create `<data_dir>/indexing_events.db`. Runs the 0019 migration
    /// idempotently (CREATE TABLE IF NOT EXISTS).
    pub fn open(data_dir: &Path) -> Result<Self, IndexingEventStoreError> {
        std::fs::create_dir_all(data_dir)?;
        let path = data_dir.join("indexing_events.db");
        let conn = Connection::open(&path)?;
        conn.execute_batch(MIGRATION_SQL)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Append one lifecycle row, stamping `ts_unix = now`. Returns the new row
    /// id (deterministic replay/dedup key). Called best-effort at the indexing
    /// emit points — a write failure does not block indexing.
    pub fn append(
        &self,
        job_id: &str,
        stage: &str,
        processed: i64,
        total: i64,
        message: &str,
    ) -> Result<i64, IndexingEventStoreError> {
        let conn = self.conn.lock().map_err(|_| IndexingEventStoreError::Poisoned)?;
        conn.execute(
            "INSERT INTO indexing_events (job_id, stage, processed, total, message, ts_unix) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![job_id, stage, processed, total, message, now_unix()],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// List the oldest `limit` rows in id ASC (chronological) order — the input
    /// the replay mapper expects. `limit` is clamped to [1, 10_000].
    pub fn list(&self, limit: usize) -> Result<Vec<IndexingEventRow>, IndexingEventStoreError> {
        let lim = limit.clamp(1, 10_000) as i64;
        let conn = self.conn.lock().map_err(|_| IndexingEventStoreError::Poisoned)?;
        let mut stmt = conn.prepare(
            "SELECT id, job_id, stage, processed, total, message, ts_unix \
             FROM indexing_events ORDER BY id ASC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![lim], |r| {
            Ok(IndexingEventRow {
                id: r.get(0)?,
                job_id: r.get(1)?,
                stage: r.get(2)?,
                processed: r.get(3)?,
                total: r.get(4)?,
                message: r.get(5)?,
                ts_unix: r.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// task-43.1 (ADR-048 D1): list rows with `since_ts` filtering — the
    /// replay input for `EventsServer::subscribe` when a subscriber reconnects
    /// with `since_ts > 0` and wants the indexing lifecycle events it missed.
    /// Mirrors `replay_events_from_audit`'s `ts < since_ts → skip` semantics
    /// (i.e. keeps rows with `ts_unix >= since_ts`, inclusive boundary).
    ///
    /// - `since_ts > 0` → `WHERE ts_unix >= ?since_ts ORDER BY id ASC LIMIT ?lim`
    /// - `since_ts <= 0` → no filter (returns the oldest `limit` rows, identical
    ///   to `list(limit)`) — so first-connect subscribers (no `since_ts`) get no
    ///   replay, byte-equivalent to the pre-task-43.1 behavior.
    ///
    /// `limit` is clamped to [1, 10_000]. Output is id ASC (chronological), the
    /// order `indexing_rows_to_pb_events` expects. 0 new dep / 0 schema migration
    /// (reuses the `ts_unix` column from migration 0019).
    pub fn list_since(
        &self,
        limit: usize,
        since_ts: i64,
    ) -> Result<Vec<IndexingEventRow>, IndexingEventStoreError> {
        let lim = limit.clamp(1, 10_000) as i64;
        let conn = self.conn.lock().map_err(|_| IndexingEventStoreError::Poisoned)?;
        // Single SQL with a `(?1 = 0 OR ts_unix >= ?1)` guard: when `since_ts <= 0`
        // the first disjunct is true (no filter, like `list`); when `since_ts > 0`
        // the second disjunct filters. Keeps one prepared statement + one param
        // layout (`params![since_ts, lim]`) instead of two branches.
        let mut stmt = conn.prepare(
            "SELECT id, job_id, stage, processed, total, message, ts_unix \
             FROM indexing_events WHERE (?1 = 0 OR ts_unix >= ?1) \
             ORDER BY id ASC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![since_ts, lim], |r| {
            Ok(IndexingEventRow {
                id: r.get(0)?,
                job_id: r.get(1)?,
                stage: r.get(2)?,
                processed: r.get(3)?,
                total: r.get(4)?,
                message: r.get(5)?,
                ts_unix: r.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}

/// task-33.3: persistent backing store for indexing lifecycle events.
/// Wraps `<data_dir>/indexing_events.db`.
pub struct SqliteIndexingEventStore {
    conn: Mutex<Connection>,
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::events::indexing_rows_to_pb_events;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("cf-idx-events-{name}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// TEST-33.3.2 (store half): append → list → mapper rebuild round-trip.
    /// Persisted job_id / stage / processed / total survive the round-trip and
    /// the mapper reconstructs the indexing.* events in id ASC order; the 0019
    /// migration is idempotent across re-open.
    #[test]
    fn test_33_3_2_store_roundtrip_and_mapper_rebuild() {
        let dir = temp_dir("roundtrip");
        let store = SqliteIndexingEventStore::open(&dir).expect("open ok");
        let id1 = store.append("job-1", "indexing", 2, 5, "").expect("append progress");
        let id2 = store.append("job-1", "indexing", 5, 5, "").expect("append progress2");
        let id3 = store.append("job-1", "cancelled", 0, 0, "").expect("append cancelled");
        assert!(id1 < id2 && id2 < id3, "ids are monotonic");

        let rows = store.list(100).expect("list ok");
        assert_eq!(rows.len(), 3, "all rows read back");
        // id ASC + fields verbatim (no synthesis).
        assert_eq!(rows[0].id, id1);
        assert_eq!(rows[0].job_id, "job-1");
        assert_eq!(rows[0].stage, "indexing");
        assert_eq!(rows[0].processed, 2);
        assert_eq!(rows[0].total, 5);
        assert!(rows[0].ts_unix > 0, "ts stamped");
        assert_eq!(rows[2].stage, "cancelled");

        // Mapper rebuild from the persisted rows.
        let evs = indexing_rows_to_pb_events(&rows);
        assert_eq!(evs.len(), 3);
        assert_eq!(evs[0].event_id, format!("evt-idx-{id1}"));
        assert_eq!(evs[0].event_type, "indexing.progress");
        assert_eq!(evs[0].job_id, Some("job-1".to_string()));
        assert!(evs[0].payload_json.contains("\"processed_files\":2"));
        assert!(evs[0].payload_json.contains("\"total_files\":5"));
        assert_eq!(evs[2].event_type, "indexing.cancelled");

        // Re-open is idempotent (CREATE TABLE IF NOT EXISTS) — rows preserved.
        drop(store);
        let store2 = SqliteIndexingEventStore::open(&dir).expect("re-open ok");
        assert_eq!(store2.list(100).expect("list2").len(), 3, "re-open preserves rows, no schema error");
    }

    /// TEST-43.1.1 (task-43.1 / ADR-048 D1): `list_since(limit, since_ts)`
    /// since_ts filtering + id ASC ordering + since_ts<=0 no-filter byte-equiv
    /// with `list(limit)`. Mirrors `replay_events_from_audit`'s
    /// `ts < since_ts → skip` semantics (inclusive `>=` boundary).
    #[test]
    fn test_43_1_1_list_since_ts_filter_and_byte_equiv() {
        let dir = temp_dir("list_since");
        let store = SqliteIndexingEventStore::open(&dir).expect("open ok");
        // Append 3 rows; ts is stamped by now_unix() at append time. Sleep a few
        // ms between appends so each row gets a distinct, monotonic ts.
        store.append("job-a", "indexing", 1, 3, "").expect("append r1");
        std::thread::sleep(std::time::Duration::from_millis(1100));
        store.append("job-a", "indexing", 2, 3, "").expect("append r2");
        std::thread::sleep(std::time::Duration::from_millis(1100));
        store.append("job-a", "error", 0, 0, "boom").expect("append r3");

        let all = store.list(100).expect("list all");
        assert_eq!(all.len(), 3, "3 rows appended");
        let ts1 = all[0].ts_unix;
        let ts2 = all[1].ts_unix;
        let ts3 = all[2].ts_unix;
        assert!(ts1 <= ts2 && ts2 <= ts3, "ts monotonic non-strict: {ts1} {ts2} {ts3}");

        // since_ts > 0: keep rows with ts_unix >= since_ts.
        // Use ts2 as the boundary → rows r2 (ts2) and r3 (ts3) kept (inclusive >=).
        let kept = store.list_since(100, ts2).expect("list_since ts2");
        assert_eq!(kept.len(), 2, "since_ts=ts2 keeps r2 + r3 (inclusive boundary)");
        assert_eq!(kept[0].id, all[1].id, "first kept is r2 (id ASC)");
        assert_eq!(kept[1].id, all[2].id, "second kept is r3");
        assert_eq!(kept[1].stage, "error", "fields verbatim");

        // since_ts > max ts → empty.
        let none = store.list_since(100, ts3 + 60).expect("list_since future");
        assert!(none.is_empty(), "since_ts beyond all rows → empty");

        // since_ts <= 0 → no filter, byte-equiv with list(limit).
        let unfilt = store.list_since(100, 0).expect("list_since 0");
        assert_eq!(unfilt.len(), 3, "since_ts=0 no filter returns all");
        assert_eq!(unfilt, all, "since_ts<=0 byte-equiv with list(limit)");
        let unfilt_neg = store.list_since(100, -5).expect("list_since -5");
        assert_eq!(unfilt_neg.len(), 3, "since_ts<0 no filter returns all");

        // limit clamp applies.
        let lim = store.list_since(2, 0).expect("list_since limit 2");
        assert_eq!(lim.len(), 2, "limit clamps result count");

        // 0 schema migration: re-open preserves rows + list_since still works.
        drop(store);
        let store2 = SqliteIndexingEventStore::open(&dir).expect("re-open ok");
        assert_eq!(
            store2.list_since(100, ts2).expect("re-open list_since").len(),
            2,
            "re-open preserves rows, list_since still filters"
        );
    }
}
