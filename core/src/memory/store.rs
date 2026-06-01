//! task-13.1 SqliteMemoryStore — CRUD + state ops backed by `memory_items`
//! SQLite table (migration `0013_memory_items.sql`).
//!
//! Surface (sequential calls only; rusqlite Connection is wrapped in std Mutex):
//!   - `open(data_dir)` — open/create the DB file + apply migration
//!   - `list(filter)` — filtered by agent_scope/status; soft_deleted excluded by default
//!   - `get(memory_id)` — single lookup; returns Option (None = not found, even for soft_deleted IDs that exist; soft_deleted rows are still gettable by ID)
//!   - `set_pinned(memory_id, pin)` — toggle is_pinned column
//!   - `set_status(memory_id, status)` — drives Deprecate / SoftDelete (CHECK constraint rejects invalid status)
//!   - `seed_for_tests(items)` — bulk-insert helper for unit/integration fixtures

use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, params_from_iter, Connection, Error as RusqliteError};

const MIGRATION_SQL: &str = include_str!("../../migrations/0013_memory_items.sql");
// task-27.1 (ADR-032 D1): add-only pin-actor + pinned-at-timestamp columns.
// Applied via a guarded check in `open` (ALTER ADD COLUMN is not idempotent on
// its own). See 0017_memory_items_add_pin_actor.sql header.
const MIGRATION_PIN_ACTOR_SQL: &str =
    include_str!("../../migrations/0017_memory_items_add_pin_actor.sql");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryItem {
    pub memory_id: String,
    pub agent_scope: String,
    pub content_preview: String,
    pub source_type: String,
    pub source_ref: String,
    pub created_at_unix: i64,
    pub updated_at_unix: i64,
    pub hit_count: i64,
    pub status: String,
    pub is_pinned: bool,
    /// task-27.1 (ADR-032 D1): pin-actor — most-recent pin caller; '' when unpinned.
    pub pinned_by: String,
    /// task-27.1 (ADR-032 D1): pinned-at unix seconds; 0 when unpinned.
    pub pinned_at_unix: i64,
}

#[derive(Debug, Default, Clone)]
pub struct MemoryListFilter {
    pub agent_id: Option<String>,
    pub scope: Option<String>,
    pub namespace: Option<String>,
    pub include_soft_deleted: bool,
}

#[derive(Debug)]
pub enum MemoryStoreError {
    NotFound,
    Invalid(String),
    Sqlite(String),
    Io(std::io::Error),
}

impl std::fmt::Display for MemoryStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryStoreError::NotFound => write!(f, "memory item not found"),
            MemoryStoreError::Invalid(msg) => write!(f, "invalid: {msg}"),
            MemoryStoreError::Sqlite(msg) => write!(f, "sqlite: {msg}"),
            MemoryStoreError::Io(err) => write!(f, "io: {err}"),
        }
    }
}

impl std::error::Error for MemoryStoreError {}

impl From<std::io::Error> for MemoryStoreError {
    fn from(e: std::io::Error) -> Self {
        MemoryStoreError::Io(e)
    }
}

impl From<RusqliteError> for MemoryStoreError {
    fn from(e: RusqliteError) -> Self {
        MemoryStoreError::Sqlite(e.to_string())
    }
}

pub struct SqliteMemoryStore {
    conn: Mutex<Connection>,
}

impl SqliteMemoryStore {
    /// Open/create the memory store DB inside `data_dir/memory.db` and apply
    /// the 0013 migration. Idempotent (CREATE TABLE IF NOT EXISTS).
    pub fn open(data_dir: &Path) -> Result<Self, MemoryStoreError> {
        std::fs::create_dir_all(data_dir)?;
        let db_path = data_dir.join("memory.db");
        let conn = Connection::open(&db_path)?;
        conn.execute_batch(MIGRATION_SQL)?;
        // task-27.1 (ADR-032 D1): idempotent add-only pin-actor/timestamp columns
        // (guarded — ALTER ADD COLUMN errors if re-run, so skip when present).
        ensure_pin_actor_columns(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// List memory items matching the supplied filter. By default soft_deleted
    /// rows are excluded; pass `include_soft_deleted=true` to include them.
    /// Returned in insertion order (created_at_unix ascending then memory_id).
    pub fn list(&self, filter: MemoryListFilter) -> Result<Vec<MemoryItem>, MemoryStoreError> {
        let conn = self.conn.lock().map_err(|e| MemoryStoreError::Invalid(format!("lock: {e}")))?;
        let mut sql = String::from(
            "SELECT memory_id, agent_scope, content_preview, source_type, source_ref, \
             created_at_unix, updated_at_unix, hit_count, status, is_pinned, \
             pinned_by, pinned_at_unix \
             FROM memory_items WHERE 1=1",
        );
        let mut args: Vec<String> = Vec::new();
        if let Some(scope) = filter.scope.as_deref() {
            sql.push_str(" AND agent_scope = ?");
            args.push(scope.to_string());
        }
        // agent_id and namespace are not stored as dedicated columns in v0.6
        // (the Console contract's MemoryItem schema captures only agent_scope);
        // they are accepted via the filter for forward-compat with the gRPC
        // request shape and treated as exact-match suffixes on agent_scope:
        //   agent_scope == "{agent_id}:{namespace}" — convention defer to
        //   [SPEC-DEFER:phase-15.import-to-memory-items].
        if let Some(agent_id) = filter.agent_id.as_deref() {
            sql.push_str(" AND agent_scope LIKE ?");
            args.push(format!("{}%", agent_id));
        }
        if let Some(ns) = filter.namespace.as_deref() {
            sql.push_str(" AND agent_scope LIKE ?");
            args.push(format!("%{}", ns));
        }
        if !filter.include_soft_deleted {
            sql.push_str(" AND status != 'soft_deleted'");
        }
        sql.push_str(" ORDER BY created_at_unix ASC, memory_id ASC");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(args.iter()), Self::row_to_item)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Get a single memory item by id. Returns Ok(None) if missing (soft_deleted
    /// rows are still returned — get-by-id is the "show all detail" path).
    pub fn get(&self, memory_id: &str) -> Result<Option<MemoryItem>, MemoryStoreError> {
        let conn = self.conn.lock().map_err(|e| MemoryStoreError::Invalid(format!("lock: {e}")))?;
        let mut stmt = conn.prepare(
            "SELECT memory_id, agent_scope, content_preview, source_type, source_ref, \
             created_at_unix, updated_at_unix, hit_count, status, is_pinned, \
             pinned_by, pinned_at_unix \
             FROM memory_items WHERE memory_id = ? LIMIT 1",
        )?;
        let mut rows = stmt.query(params![memory_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::row_to_item(row)?))
        } else {
            Ok(None)
        }
    }

    /// Toggle is_pinned (true → 1, false → 0). Also bumps updated_at_unix.
    /// Returns NotFound when the memory_id does not exist.
    pub fn set_pinned(&self, memory_id: &str, pinned: bool) -> Result<(), MemoryStoreError> {
        let conn = self.conn.lock().map_err(|e| MemoryStoreError::Invalid(format!("lock: {e}")))?;
        let now = now_unix();
        let n = conn.execute(
            "UPDATE memory_items SET is_pinned = ?, updated_at_unix = ? WHERE memory_id = ?",
            params![pinned as i64, now, memory_id],
        )?;
        if n == 0 {
            Err(MemoryStoreError::NotFound)
        } else {
            Ok(())
        }
    }

    /// task-27.1 (ADR-032 D1): actor-aware pin. pin=true writes `pinned_by=actor`
    /// + `pinned_at_unix=now`; pin=false clears both to defaults (`''` / 0).
    /// Also bumps `updated_at_unix` and toggles `is_pinned` (superset of
    /// `set_pinned`, which delegates here with an empty actor). NotFound when
    /// the memory_id does not exist.
    pub fn set_pinned_with_actor(
        &self,
        memory_id: &str,
        pinned: bool,
        actor: &str,
    ) -> Result<(), MemoryStoreError> {
        let _ = (memory_id, pinned, actor);
        todo!("task-27.1 GREEN: UPDATE is_pinned + pinned_by/pinned_at_unix write-through")
    }

    /// Set status to one of {active, deprecated, soft_deleted}; bumps updated_at_unix.
    /// CHECK constraint in the schema rejects other values (rusqlite surfaces it
    /// as a SqliteFailure which we map to `Invalid`).
    pub fn set_status(&self, memory_id: &str, status: &str) -> Result<(), MemoryStoreError> {
        if !matches!(status, "active" | "deprecated" | "soft_deleted") {
            return Err(MemoryStoreError::Invalid(format!(
                "status must be one of active/deprecated/soft_deleted; got {status}"
            )));
        }
        let conn = self.conn.lock().map_err(|e| MemoryStoreError::Invalid(format!("lock: {e}")))?;
        let now = now_unix();
        let n = conn.execute(
            "UPDATE memory_items SET status = ?, updated_at_unix = ? WHERE memory_id = ?",
            params![status, now, memory_id],
        )?;
        if n == 0 {
            Err(MemoryStoreError::NotFound)
        } else {
            Ok(())
        }
    }

    /// Bulk-insert helper used by unit + integration test fixtures.
    pub fn seed_for_tests(&self, items: Vec<MemoryItem>) -> Result<(), MemoryStoreError> {
        let conn = self.conn.lock().map_err(|e| MemoryStoreError::Invalid(format!("lock: {e}")))?;
        for item in items {
            conn.execute(
                "INSERT INTO memory_items \
                 (memory_id, agent_scope, content_preview, source_type, source_ref, \
                  created_at_unix, updated_at_unix, hit_count, status, is_pinned, \
                  pinned_by, pinned_at_unix) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    item.memory_id,
                    item.agent_scope,
                    item.content_preview,
                    item.source_type,
                    item.source_ref,
                    item.created_at_unix,
                    item.updated_at_unix,
                    item.hit_count,
                    item.status,
                    item.is_pinned as i64,
                    item.pinned_by,
                    item.pinned_at_unix,
                ],
            )?;
        }
        Ok(())
    }

    fn row_to_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryItem> {
        Ok(MemoryItem {
            memory_id: row.get(0)?,
            agent_scope: row.get(1)?,
            content_preview: row.get(2)?,
            source_type: row.get(3)?,
            source_ref: row.get(4)?,
            created_at_unix: row.get(5)?,
            updated_at_unix: row.get(6)?,
            hit_count: row.get(7)?,
            status: row.get(8)?,
            is_pinned: row.get::<_, i64>(9)? != 0,
            pinned_by: row.get(10)?,
            pinned_at_unix: row.get(11)?,
        })
    }
}

/// task-27.1 (ADR-032 D1): idempotently add the pin-actor / pinned-at columns to
/// an existing `memory_items` table. ALTER ADD COLUMN is not IF-NOT-EXISTS-able,
/// so check `PRAGMA table_info` first and only run the 0017 migration when the
/// column is absent (fresh DBs created by the 0013 CREATE TABLE go through here
/// too — they lack the columns until this adds them).
fn ensure_pin_actor_columns(conn: &Connection) -> Result<(), MemoryStoreError> {
    let mut has_pinned_by = false;
    {
        let mut stmt = conn.prepare("PRAGMA table_info(memory_items)")?;
        let cols = stmt.query_map([], |r| r.get::<_, String>(1))?;
        for c in cols {
            if c? == "pinned_by" {
                has_pinned_by = true;
                break;
            }
        }
    }
    if !has_pinned_by {
        conn.execute_batch(MIGRATION_PIN_ACTOR_SQL)?;
    }
    Ok(())
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn fresh_store() -> SqliteMemoryStore {
        let seq = SEQ.fetch_add(1, Ordering::SeqCst);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cf-mem-{}-{}-{}", std::process::id(), nanos, seq));
        SqliteMemoryStore::open(&dir).expect("open ok")
    }

    fn mem(id: &str, scope: &str, status: &str) -> MemoryItem {
        let now = now_unix();
        MemoryItem {
            memory_id: id.into(),
            agent_scope: scope.into(),
            content_preview: format!("preview for {id}"),
            source_type: "test".into(),
            source_ref: format!("file:{id}.md"),
            created_at_unix: now,
            updated_at_unix: now,
            hit_count: 0,
            status: status.into(),
            is_pinned: false,
            pinned_by: String::new(),
            pinned_at_unix: 0,
        }
    }

    #[test]
    fn test_seed_and_get_roundtrip() {
        let s = fresh_store();
        s.seed_for_tests(vec![mem("mem-1", "agent-a", "active")])
            .expect("seed");
        let got = s.get("mem-1").expect("get ok");
        assert!(got.is_some());
        let item = got.unwrap();
        assert_eq!(item.memory_id, "mem-1");
        assert_eq!(item.status, "active");
        assert!(!item.is_pinned);
    }

    #[test]
    fn test_list_default_excludes_soft_deleted() {
        let s = fresh_store();
        s.seed_for_tests(vec![
            mem("a", "scope-x", "active"),
            mem("b", "scope-x", "deprecated"),
            mem("c", "scope-x", "soft_deleted"),
        ])
        .unwrap();
        let items = s.list(MemoryListFilter::default()).unwrap();
        assert_eq!(items.len(), 2, "soft_deleted must be excluded by default");
        let ids: Vec<_> = items.iter().map(|i| i.memory_id.as_str()).collect();
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"b"));
        assert!(!ids.contains(&"c"));
    }

    #[test]
    fn test_list_with_include_soft_deleted() {
        let s = fresh_store();
        s.seed_for_tests(vec![
            mem("a", "scope-x", "active"),
            mem("b", "scope-x", "soft_deleted"),
        ])
        .unwrap();
        let items = s
            .list(MemoryListFilter {
                include_soft_deleted: true,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_list_filter_by_scope() {
        let s = fresh_store();
        s.seed_for_tests(vec![
            mem("a", "scope-x", "active"),
            mem("b", "scope-y", "active"),
        ])
        .unwrap();
        let items = s
            .list(MemoryListFilter {
                scope: Some("scope-x".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].memory_id, "a");
    }

    #[test]
    fn test_set_pinned_persists() {
        let s = fresh_store();
        s.seed_for_tests(vec![mem("p", "scope", "active")]).unwrap();
        s.set_pinned("p", true).unwrap();
        let got = s.get("p").unwrap().unwrap();
        assert!(got.is_pinned);
        // Unpin too
        s.set_pinned("p", false).unwrap();
        assert!(!s.get("p").unwrap().unwrap().is_pinned);
    }

    /// task-17.1 / ADR-022 AC3: List SELECT must project is_pinned per row.
    #[test]
    fn test_list_returns_is_pinned_column() {
        let s = fresh_store();
        s.seed_for_tests(vec![
            mem("p1", "scope", "active"),
            mem("p2", "scope", "active"),
        ])
        .unwrap();
        s.set_pinned("p1", true).unwrap();
        let items = s.list(MemoryListFilter::default()).unwrap();
        assert_eq!(items.len(), 2);
        let p1 = items.iter().find(|i| i.memory_id == "p1").unwrap();
        let p2 = items.iter().find(|i| i.memory_id == "p2").unwrap();
        assert!(p1.is_pinned, "p1 list row should reflect set_pinned(true)");
        assert!(!p2.is_pinned, "p2 list row should default to is_pinned=false");
    }

    /// TEST-27.1.2: set_pinned_with_actor(true) writes pinned_by=actor +
    /// pinned_at_unix>0; (false) clears to defaults; get+list project both.
    #[test]
    fn test_set_pinned_with_actor_writes_through_and_clears() {
        let s = fresh_store();
        s.seed_for_tests(vec![mem("pa", "scope", "active")]).unwrap();
        // pin=true → actor + timestamp written.
        s.set_pinned_with_actor("pa", true, "console-api").unwrap();
        let got = s.get("pa").unwrap().unwrap();
        assert!(got.is_pinned);
        assert_eq!(got.pinned_by, "console-api");
        assert!(got.pinned_at_unix > 0, "pinned_at_unix set on pin");
        // list projects the fields too.
        let listed = s.list(MemoryListFilter::default()).unwrap();
        let row = listed.iter().find(|i| i.memory_id == "pa").unwrap();
        assert_eq!(row.pinned_by, "console-api");
        assert!(row.pinned_at_unix > 0);
        // pin=false → cleared to defaults.
        s.set_pinned_with_actor("pa", false, "console-api").unwrap();
        let after = s.get("pa").unwrap().unwrap();
        assert!(!after.is_pinned);
        assert_eq!(after.pinned_by, "", "unpin clears actor");
        assert_eq!(after.pinned_at_unix, 0, "unpin clears timestamp");
    }

    /// TEST-27.1.2b: pinned_at_unix is independent of updated_at_unix — a later
    /// deprecate/soft_delete update bumps updated_at but does not touch pin fields.
    #[test]
    fn test_pinned_at_independent_of_updated_at() {
        let s = fresh_store();
        s.seed_for_tests(vec![mem("pi", "scope", "active")]).unwrap();
        s.set_pinned_with_actor("pi", true, "actor-x").unwrap();
        let pinned = s.get("pi").unwrap().unwrap();
        let pin_ts = pinned.pinned_at_unix;
        // A non-pin update (deprecate) bumps updated_at but leaves pin fields.
        s.set_status("pi", "deprecated").unwrap();
        let after = s.get("pi").unwrap().unwrap();
        assert_eq!(after.pinned_by, "actor-x", "deprecate must not clear pin actor");
        assert_eq!(after.pinned_at_unix, pin_ts, "deprecate must not change pinned_at");
    }

    /// TEST-27.1: bare set_pinned still works (delegates with empty actor),
    /// preserving the task-17.1 signature + behavior.
    #[test]
    fn test_set_pinned_delegates_backward_compatible() {
        let s = fresh_store();
        s.seed_for_tests(vec![mem("bc", "scope", "active")]).unwrap();
        s.set_pinned("bc", true).unwrap();
        assert!(s.get("bc").unwrap().unwrap().is_pinned);
        s.set_pinned("bc", false).unwrap();
        assert!(!s.get("bc").unwrap().unwrap().is_pinned);
    }

    #[test]
    fn test_set_pinned_not_found() {
        let s = fresh_store();
        let err = s.set_pinned("missing", true).expect_err("expect NotFound");
        assert!(matches!(err, MemoryStoreError::NotFound));
    }

    #[test]
    fn test_set_status_deprecated_persists() {
        let s = fresh_store();
        s.seed_for_tests(vec![mem("d", "scope", "active")]).unwrap();
        s.set_status("d", "deprecated").unwrap();
        let got = s.get("d").unwrap().unwrap();
        assert_eq!(got.status, "deprecated");
    }

    #[test]
    fn test_set_status_soft_deleted_excludes_from_list_default() {
        let s = fresh_store();
        s.seed_for_tests(vec![mem("x", "scope", "active")]).unwrap();
        s.set_status("x", "soft_deleted").unwrap();
        let items = s.list(MemoryListFilter::default()).unwrap();
        assert!(items.is_empty(), "soft_deleted excluded by default list");
        // but get-by-id still finds it
        assert!(s.get("x").unwrap().is_some());
    }

    #[test]
    fn test_set_status_rejects_invalid() {
        let s = fresh_store();
        s.seed_for_tests(vec![mem("y", "scope", "active")]).unwrap();
        let err = s.set_status("y", "garbage").expect_err("expect Invalid");
        assert!(matches!(err, MemoryStoreError::Invalid(_)));
    }
}
