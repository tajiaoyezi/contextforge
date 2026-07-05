//! task-52.1 (Phase 52 / ADR-053 D2): SqliteMembershipStore — workspace membership
//! table for 3-role RBAC. Closes `[SPEC-DEFER:phase-future.rbac-roles-permissions]`
//! for the storage layer.
//!
//! Surface (sequential calls only; rusqlite Connection is wrapped in std Mutex):
//!   - `open(data_dir)` — open/create `membership.db` + apply migration 0022 (idempotent)
//!   - `add_member(workspace_id, user_id, role)` — INSERT; Duplicate on PK collision; CHECK fail → Invalid
//!   - `remove_member(workspace_id, user_id)` — DELETE; Ok(()) even when absent (idempotent)
//!   - `list_members(workspace_id)` — SELECT WHERE workspace_id=? ORDER BY created_at
//!   - `get_role(workspace_id, user_id)` — SELECT role WHERE PK match → Option<Role>
//!
//! Pattern mirrors `core/src/identity/store.rs` (SqliteUserStore): a separate
//! `membership.db` SQLite file with `Mutex<Connection>` (ADR-016 D1 single-owner-
//! per-DB). No FK to workspaces/users (cross-DB); role CHECK at the DB layer is
//! the authoritative enum guard.

use std::path::Path;
use std::str::FromStr;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, types::Type, Connection, Error as RusqliteError};

const MIGRATION_SQL: &str = include_str!("../../migrations/0022_workspace_members.sql");

/// The 3 fixed RBAC roles (ADR-053 D1). Stored verbatim in the `role` column
/// (`admin` / `member` / `viewer`); the DB CHECK constraint is the authoritative
/// enum guard, `FromStr` is the app-side mirror.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Admin,
    Member,
    Viewer,
}

impl Role {
    /// Wire/storage representation (matches the migration CHECK enum verbatim).
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Member => "member",
            Role::Viewer => "viewer",
        }
    }
}

impl FromStr for Role {
    type Err = MembershipStoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "admin" => Ok(Role::Admin),
            "member" => Ok(Role::Member),
            "viewer" => Ok(Role::Viewer),
            other => Err(MembershipStoreError::Invalid(format!(
                "unknown role: {other}"
            ))),
        }
    }
}

/// One membership row: a user bound to a workspace with a fixed role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub workspace_id: String,
    pub user_id: String,
    pub role: Role,
    pub created_at_unix: i64,
}

#[derive(Debug)]
pub enum MembershipStoreError {
    /// PK(workspace_id, user_id) collision — the user is already a member.
    Duplicate(String),
    /// CHECK constraint failure (e.g. role not in admin/member/viewer) or a bad
    /// role string parsed at the app boundary.
    Invalid(String),
    Sqlite(String),
    Io(std::io::Error),
}

impl std::fmt::Display for MembershipStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MembershipStoreError::Duplicate(what) => write!(f, "duplicate {what}"),
            MembershipStoreError::Invalid(msg) => write!(f, "invalid: {msg}"),
            MembershipStoreError::Sqlite(msg) => write!(f, "sqlite: {msg}"),
            MembershipStoreError::Io(err) => write!(f, "io: {err}"),
        }
    }
}

impl std::error::Error for MembershipStoreError {}

impl From<std::io::Error> for MembershipStoreError {
    fn from(err: std::io::Error) -> Self {
        MembershipStoreError::Io(err)
    }
}

impl From<RusqliteError> for MembershipStoreError {
    fn from(err: RusqliteError) -> Self {
        // Distinguish PK collision (Duplicate) from CHECK failure (Invalid). rusqlite
        // exposes constraint kind via the extended result code embedded in the message;
        // both surface as ConstraintViolation, so we disambiguate on the message text.
        if err.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
            let msg = err.to_string();
            if msg.contains("CHECK constraint failed") {
                MembershipStoreError::Invalid(msg)
            } else {
                // PRIMARY KEY collision — the (workspace_id, user_id) pair already exists.
                MembershipStoreError::Duplicate("membership".to_string())
            }
        } else {
            MembershipStoreError::Sqlite(err.to_string())
        }
    }
}

pub struct SqliteMembershipStore {
    conn: Mutex<Connection>,
}

impl SqliteMembershipStore {
    /// Open/create the membership store DB inside `data_dir/membership.db` and apply
    /// migration 0022. Idempotent (CREATE TABLE IF NOT EXISTS).
    pub fn open(data_dir: &Path) -> Result<Self, MembershipStoreError> {
        std::fs::create_dir_all(data_dir)?;
        let db_path = data_dir.join("membership.db");
        let conn = Connection::open(&db_path)?;
        conn.execute_batch(MIGRATION_SQL)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Add a member to a workspace with the given role. `created_at_unix` defaults to
    /// now when 0. Returns `Duplicate` on PK(workspace_id, user_id) collision and
    /// `Invalid` if the role violates the DB CHECK (the `Role` enum makes this
    /// unreachable from safe code, but the guard stays authoritative).
    pub fn add_member(
        &self,
        workspace_id: &str,
        user_id: &str,
        role: Role,
    ) -> Result<Member, MembershipStoreError> {
        let created_at_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let conn = self
            .conn
            .lock()
            .map_err(|e| MembershipStoreError::Sqlite(format!("lock: {e}")))?;
        conn.execute(
            "INSERT INTO workspace_members (workspace_id, user_id, role, created_at_unix) \
             VALUES (?, ?, ?, ?)",
            params![workspace_id, user_id, role.as_str(), created_at_unix],
        )?;
        Ok(Member {
            workspace_id: workspace_id.to_string(),
            user_id: user_id.to_string(),
            role,
            created_at_unix,
        })
    }

    /// Remove a member from a workspace. Idempotent: returns `Ok(())` even when the
    /// (workspace_id, user_id) row is absent (DELETE matches zero rows).
    pub fn remove_member(
        &self,
        workspace_id: &str,
        user_id: &str,
    ) -> Result<(), MembershipStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MembershipStoreError::Sqlite(format!("lock: {e}")))?;
        conn.execute(
            "DELETE FROM workspace_members WHERE workspace_id = ? AND user_id = ?",
            params![workspace_id, user_id],
        )?;
        Ok(())
    }

    /// List all members of a workspace, ordered by `created_at_unix` ascending then
    /// `user_id` (deterministic ordering for the membership list endpoint).
    pub fn list_members(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<Member>, MembershipStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MembershipStoreError::Sqlite(format!("lock: {e}")))?;
        let mut stmt = conn.prepare(
            "SELECT workspace_id, user_id, role, created_at_unix FROM workspace_members \
             WHERE workspace_id = ? ORDER BY created_at_unix ASC, user_id ASC",
        )?;
        let rows = stmt.query_map(params![workspace_id], |row| {
            let role_str: String = row.get(2)?;
            // role came from a CHECK-constrained column, so parse is infallible here;
            // fall back to Invalid (defensive) rather than panicking inside the closure.
            let role = Role::from_str(&role_str).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(2, Type::Text, Box::new(e))
            })?;
            Ok(Member {
                workspace_id: row.get(0)?,
                user_id: row.get(1)?,
                role,
                created_at_unix: row.get(3)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Resolve a user's role in a workspace. Returns `None` when the user is not a
    /// member. This is the AuthZ seam: the Go `roleMiddleware` (task-52.3) calls this
    /// via gRPC and gates destructive ops on `Role::Admin` (ADR-053 D3).
    pub fn get_role(
        &self,
        workspace_id: &str,
        user_id: &str,
    ) -> Result<Option<Role>, MembershipStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MembershipStoreError::Sqlite(format!("lock: {e}")))?;
        let mut stmt = conn.prepare(
            "SELECT role FROM workspace_members WHERE workspace_id = ? AND user_id = ?",
        )?;
        let mut rows = stmt.query(params![workspace_id, user_id])?;
        match rows.next()? {
            Some(row) => {
                let role_str: String = row.get(0)?;
                let role = Role::from_str(&role_str)?;
                Ok(Some(role))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn fresh_store() -> SqliteMembershipStore {
        let seq = SEQ.fetch_add(1, Ordering::SeqCst);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "cf-membership-{}-{}-{}",
            std::process::id(),
            nanos,
            seq
        ));
        SqliteMembershipStore::open(&dir).expect("open ok")
    }

    // TEST-52.1.1 / AC1: migration 0022 幂等 + workspace_members schema (PK + CHECK)
    #[test]
    fn test_52_1_1_migration_idempotent_and_schema() {
        let s = fresh_store();
        // re-open the same dir → migration IF NOT EXISTS must be idempotent
        let dir = std::env::temp_dir().join(format!(
            "cf-membership-reopen-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::SeqCst)
        ));
        {
            let _ = SqliteMembershipStore::open(&dir).expect("first open");
        }
        let _ = SqliteMembershipStore::open(&dir).expect("second open (idempotent)");

        // schema proof: valid add round-trips through list (columns exist, types ok)
        s.add_member("ws-1", "u1", Role::Admin)
            .expect("add admin");
        let list = s.list_members("ws-1").expect("list");
        assert_eq!(list.len(), 1, "one member after add");
        assert_eq!(list[0].workspace_id, "ws-1");
        assert_eq!(list[0].user_id, "u1");
        assert_eq!(list[0].role, Role::Admin);
        assert!(list[0].created_at_unix > 0, "created_at_unix defaults to now");

        // CHECK constraint: an invalid role string must be rejected at the DB layer.
        // We bypass the typed API and insert raw to exercise the migration's CHECK.
        {
            let conn = s.conn.lock().expect("lock for raw insert");
            let bad = conn.execute(
                "INSERT INTO workspace_members (workspace_id, user_id, role, created_at_unix) \
                 VALUES (?, ?, ?, ?)",
                params!["ws-1", "u-bad", "superuser", 0],
            );
            assert!(bad.is_err(), "CHECK rejects role outside enum");
            let msg = format!("{}", bad.unwrap_err());
            assert!(
                msg.contains("CHECK constraint failed"),
                "expected CHECK failure, got: {msg}"
            );
        }

        // PK constraint: duplicate (workspace_id, user_id) rejected at DB layer.
        {
            let conn = s.conn.lock().expect("lock for raw dup");
            let dup = conn.execute(
                "INSERT INTO workspace_members (workspace_id, user_id, role, created_at_unix) \
                 VALUES (?, ?, ?, ?)",
                params!["ws-1", "u1", "member", 0],
            );
            assert!(dup.is_err(), "PK rejects duplicate membership");
        }
    }

    // TEST-52.1.2 / AC2: MembershipStore add/list/get_role round-trip + remove idempotent
    // + duplicate PK → Duplicate error + invalid role → Invalid error.
    #[test]
    fn test_52_1_2_crud_get_role_dup_and_check_errors() {
        let s = fresh_store();

        // add → list → get_role round-trip across all 3 roles
        s.add_member("ws-1", "u-admin", Role::Admin)
            .expect("add admin");
        s.add_member("ws-1", "u-member", Role::Member)
            .expect("add member");
        s.add_member("ws-1", "u-viewer", Role::Viewer)
            .expect("add viewer");

        let list = s.list_members("ws-1").expect("list ws-1");
        assert_eq!(list.len(), 3, "three members");
        assert_eq!(s.get_role("ws-1", "u-admin").expect("role admin"), Some(Role::Admin));
        assert_eq!(s.get_role("ws-1", "u-member").expect("role member"), Some(Role::Member));
        assert_eq!(s.get_role("ws-1", "u-viewer").expect("role viewer"), Some(Role::Viewer));
        // non-member → None
        assert_eq!(s.get_role("ws-1", "u-none").expect("role none"), None);
        // empty workspace → empty list
        assert!(s.list_members("ws-empty").expect("list empty").is_empty());

        // duplicate PK → Duplicate error (via typed API)
        let dup = s.add_member("ws-1", "u-admin", Role::Member);
        match dup {
            Err(MembershipStoreError::Duplicate(what)) => assert_eq!(what, "membership"),
            other => panic!("expected Duplicate(membership), got {other:?}"),
        }

        // remove_member is idempotent: existing row then absent row both Ok
        s.remove_member("ws-1", "u-viewer").expect("remove existing");
        assert_eq!(s.get_role("ws-1", "u-viewer").expect("role after remove"), None);
        s.remove_member("ws-1", "u-viewer").expect("remove absent (idempotent)");
        assert_eq!(s.list_members("ws-1").expect("list after remove").len(), 2);

        // Role::from_str mirrors the DB enum: valid parses, invalid → Invalid
        assert_eq!("admin".parse::<Role>().unwrap(), Role::Admin);
        assert_eq!("member".parse::<Role>().unwrap(), Role::Member);
        assert_eq!("viewer".parse::<Role>().unwrap(), Role::Viewer);
        match "root".parse::<Role>() {
            Err(MembershipStoreError::Invalid(_)) => {}
            other => panic!("expected Invalid for bad role, got {other:?}"),
        }
    }
}
