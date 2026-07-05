//! task-50.1 (Phase 50 / ADR-051 D2): SqliteUserStore — per-user identity table for verified
//! actor propagation. Closes `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`.
//!
//! Surface (sequential calls only; rusqlite Connection is wrapped in std Mutex):
//!   - `open(data_dir)` — open/create `users.db` + apply migration 0020 (idempotent)
//!   - `create(user)` — insert; fails on duplicate id or token (UNIQUE)
//!   - `get_by_token(token)` — bearer resolution; returns Option (None = unknown token)
//!   - `list()` — all users (created_at ascending then id; admin/list endpoint)
//!
//! Token is plaintext (local-first compromise; hash storage deferred to Phase 51+,
//! `[SPEC-DEFER:phase-future.token-hash-storage]` — needs salt + HMAC evaluation).

use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, Error as RusqliteError};

const MIGRATION_SQL: &str = include_str!("../../migrations/0020_users.sql");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: String,
    pub name: String,
    pub token: String,
    pub created_at_unix: i64,
}

#[derive(Debug)]
pub enum UserStoreError {
    Duplicate(String),
    Sqlite(String),
    Io(std::io::Error),
}

impl std::fmt::Display for UserStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserStoreError::Duplicate(what) => write!(f, "duplicate {what}"),
            UserStoreError::Sqlite(msg) => write!(f, "sqlite: {msg}"),
            UserStoreError::Io(err) => write!(f, "io: {err}"),
        }
    }
}

impl std::error::Error for UserStoreError {}

impl From<std::io::Error> for UserStoreError {
    fn from(err: std::io::Error) -> Self {
        UserStoreError::Io(err)
    }
}

impl From<RusqliteError> for UserStoreError {
    fn from(err: RusqliteError) -> Self {
        // rusqlite exposes the constraint name via `sqlite_constraint` code; the message text
        // carries the column name (e.g. "UNIQUE constraint failed: users.token").
        if err.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
            let msg = err.to_string();
            let what = if msg.contains("users.token") {
                "token"
            } else if msg.contains("users.id") || msg.contains("users.PRIMARY") {
                "id"
            } else {
                "row"
            };
            UserStoreError::Duplicate(what.to_string())
        } else {
            UserStoreError::Sqlite(err.to_string())
        }
    }
}

pub struct SqliteUserStore {
    conn: Mutex<Connection>,
}

impl SqliteUserStore {
    /// Open/create the user store DB inside `data_dir/users.db` and apply migration 0020.
    /// Idempotent (CREATE TABLE IF NOT EXISTS).
    pub fn open(data_dir: &Path) -> Result<Self, UserStoreError> {
        std::fs::create_dir_all(data_dir)?;
        let db_path = data_dir.join("users.db");
        let conn = Connection::open(&db_path)?;
        conn.execute_batch(MIGRATION_SQL)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Create a new user. The caller supplies `id`, `name`, and `token`; `created_at_unix`
    /// defaults to now when 0. Returns `Duplicate` on id/token collision.
    pub fn create(&self, mut user: User) -> Result<User, UserStoreError> {
        if user.created_at_unix == 0 {
            user.created_at_unix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
        }
        let conn = self.conn.lock().map_err(|e| UserStoreError::Sqlite(format!("lock: {e}")))?;
        conn.execute(
            "INSERT INTO users (id, name, token, created_at_unix) VALUES (?, ?, ?, ?)",
            params![user.id, user.name, user.token, user.created_at_unix],
        )?;
        Ok(user)
    }

    /// Resolve a bearer token to a user. Returns `None` when the token is unknown (no row).
    /// This is the verified-identity seam: the Go REST layer calls this via gRPC and uses
    /// the returned `User.id` as the authoritative actor (overriding any caller-declared value).
    pub fn get_by_token(&self, token: &str) -> Result<Option<User>, UserStoreError> {
        let conn = self.conn.lock().map_err(|e| UserStoreError::Sqlite(format!("lock: {e}")))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, token, created_at_unix FROM users WHERE token = ?",
        )?;
        let mut rows = stmt.query(params![token])?;
        match rows.next()? {
            Some(row) => Ok(Some(User {
                id: row.get(0)?,
                name: row.get(1)?,
                token: row.get(2)?,
                created_at_unix: row.get(3)?,
            })),
            None => Ok(None),
        }
    }

    /// List all users (created_at ascending then id) — backs the admin list endpoint.
    pub fn list(&self) -> Result<Vec<User>, UserStoreError> {
        let conn = self.conn.lock().map_err(|e| UserStoreError::Sqlite(format!("lock: {e}")))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, token, created_at_unix FROM users ORDER BY created_at_unix ASC, id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                token: row.get(2)?,
                created_at_unix: row.get(3)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn fresh_store() -> SqliteUserStore {
        let seq = SEQ.fetch_add(1, Ordering::SeqCst);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cf-user-{}-{}-{}", std::process::id(), nanos, seq));
        SqliteUserStore::open(&dir).expect("open ok")
    }

    fn user(id: &str, name: &str, token: &str) -> User {
        User {
            id: id.to_string(),
            name: name.to_string(),
            token: token.to_string(),
            created_at_unix: 0,
        }
    }

    // TEST-50.1.1 / AC1: migration 0020 幂等 + users 表 schema 正确
    #[test]
    fn test_50_1_1_migration_idempotent_and_schema() {
        let s = fresh_store();
        // re-open the same dir → migration IF NOT EXISTS must be idempotent
        let dir = std::env::temp_dir().join(format!(
            "cf-user-reopen-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::SeqCst)
        ));
        {
            let _ = SqliteUserStore::open(&dir).expect("first open");
        }
        let _ = SqliteUserStore::open(&dir).expect("second open (idempotent)");
        // schema: insert + select round-trip proves columns exist
        let u = s.create(user("u1", "alice", "tok-1")).expect("create");
        assert_eq!(u.id, "u1");
        assert_eq!(u.name, "alice");
        assert_eq!(u.token, "tok-1");
        assert!(u.created_at_unix > 0, "created_at_unix defaults to now when 0");
    }

    // TEST-50.1.2 / AC2: UserStore create/get-by-token/list + dup-token err
    #[test]
    fn test_50_1_2_crud_get_by_token_dup_err() {
        let s = fresh_store();
        s.create(user("u1", "alice", "tok-a")).expect("create u1");
        s.create(user("u2", "bob", "tok-b")).expect("create u2");

        // get-by-token round-trip
        let got = s.get_by_token("tok-a").expect("get tok-a").expect("some");
        assert_eq!(got.id, "u1");
        assert_eq!(got.name, "alice");
        let none = s.get_by_token("unknown").expect("get unknown ok");
        assert!(none.is_none(), "unknown token returns None");

        // list returns both, ascending by created_at then id
        let list = s.list().expect("list");
        assert_eq!(list.len(), 2, "list has both users");

        // duplicate token errors
        let dup = s.create(user("u3", "carol", "tok-a"));
        match dup {
            Err(UserStoreError::Duplicate(what)) => assert_eq!(what, "token"),
            other => panic!("expected Duplicate(token), got {other:?}"),
        }
        // duplicate id errors
        let dup_id = s.create(user("u1", "again", "tok-c"));
        match dup_id {
            Err(UserStoreError::Duplicate(_)) => {}
            other => panic!("expected Duplicate(id), got {other:?}"),
        }
    }
}
