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
}
