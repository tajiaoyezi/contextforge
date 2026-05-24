//! task-10.2 (Phase 10): workspace — Console Contract v1 Workspace resource +
//! workspace_id ↔ collection_id 1:1 映射 + SQLite 持久化.
//!
//! 字段对齐 Console `console-api/internal/coreadapter/contractv1/contractv1.go`
//! Workspace must-have 字段 (workspace_id / name / root_path / status /
//! config_snapshot / allowlist / denylist / created_at / updated_at)。
//!
//! 时间字段：Rust 侧以 Unix epoch 秒（i64）存储，避新增 chrono dep（playbook
//! v0.3 不预期新 dep）；Go REST handler (task-10.4) 在 wire 序列化时通过
//! `time.Unix(sec, 0).UTC()` 转 RFC3339 string 喂 Console JSON。task-10.2 §10
//! trade-off #1 文档化。
//!
//! Refs: ADR-015 §D2 / phase-10 §6 AC2 / task-10.2 §6 AC1-5

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const MIGRATION_SQL: &str = include_str!("../../migrations/0010_workspaces.sql");

const WORKSPACE_ID_MAX_LEN: usize = 64;

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Workspace 资源 (Console Contract v1 must-have 字段镜像 — wire shape 由
/// internal/contractv1/Workspace 表达，本结构是 Rust 侧持久化模型).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Workspace {
    pub workspace_id: String,
    pub name: String,
    pub root_path: String,
    pub status: String,
    pub config_snapshot: String, // JSON-serialized opaque blob (RawMessage 等价)
    pub allowlist: Vec<String>,
    pub denylist: Vec<String>,
    pub created_at_unix: i64,
    pub updated_at_unix: i64,
}

/// Workspace 创建入参 (Console Contract v1 WorkspaceCreate 镜像 + workspace_id
/// 显式提供为 1:1 collection_id; 调用方负责唯一性).
#[derive(Debug, Clone, Default)]
pub struct WorkspaceCreate {
    pub workspace_id: String,
    pub name: String,
    pub root_path: String,
    pub allowlist: Vec<String>,
    pub denylist: Vec<String>,
}

/// WorkspaceStore trait — CRUD 抽象 + soft-delete.
pub trait WorkspaceStore: Send + Sync {
    fn create(&self, req: &WorkspaceCreate) -> Result<Workspace, WorkspaceError>;
    fn list(&self) -> Result<Vec<Workspace>, WorkspaceError>;
    fn get(&self, workspace_id: &str) -> Result<Option<Workspace>, WorkspaceError>;
    fn update_config(
        &self,
        workspace_id: &str,
        allowlist: Vec<String>,
        denylist: Vec<String>,
    ) -> Result<Workspace, WorkspaceError>;
    fn soft_delete(&self, workspace_id: &str) -> Result<(), WorkspaceError>;
}

/// SQLite 实现 — Mutex<Connection> 包装 (rusqlite Connection 非 Send).
pub struct SqliteWorkspaceStore {
    conn: Mutex<Connection>,
    data_dir: PathBuf,
}

impl SqliteWorkspaceStore {
    /// 打开 / 创建 SqliteWorkspaceStore. data_dir 作为 collection 物理目录的根
    /// (`<data_dir>/collections/<workspace_id>/` 在 create() 时自动建).
    /// SQLite 文件落 `<data_dir>/workspaces.db`.
    pub fn open(data_dir: &Path) -> Result<Self, WorkspaceError> {
        std::fs::create_dir_all(data_dir)?;
        let db_path = data_dir.join("workspaces.db");
        let conn = Connection::open(&db_path)?;
        conn.execute_batch(MIGRATION_SQL)?;
        Ok(Self {
            conn: Mutex::new(conn),
            data_dir: data_dir.to_path_buf(),
        })
    }

    /// 物理 collection 目录路径 (workspace_id 直接映射).
    pub fn collection_dir(&self, workspace_id: &str) -> PathBuf {
        self.data_dir.join("collections").join(workspace_id)
    }

    fn validate_id(id: &str) -> Result<(), WorkspaceError> {
        if id.is_empty() || id.len() > WORKSPACE_ID_MAX_LEN {
            return Err(WorkspaceError::Invalid(format!(
                "workspace_id must be 1..={WORKSPACE_ID_MAX_LEN} chars; got len={}",
                id.len()
            )));
        }
        if !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(WorkspaceError::Invalid(
                "workspace_id must match ^[a-zA-Z0-9_-]+$".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_create(req: &WorkspaceCreate) -> Result<(), WorkspaceError> {
        Self::validate_id(&req.workspace_id)?;
        if req.name.trim().is_empty() {
            return Err(WorkspaceError::Invalid("name must not be empty".into()));
        }
        if req.root_path.trim().is_empty() {
            return Err(WorkspaceError::Invalid("root_path must not be empty".into()));
        }
        let path = Path::new(&req.root_path);
        if !path.is_absolute() {
            return Err(WorkspaceError::Invalid(format!(
                "root_path must be absolute; got {}",
                req.root_path
            )));
        }
        Ok(())
    }

    fn row_to_workspace(row: &rusqlite::Row<'_>) -> rusqlite::Result<Workspace> {
        let allowlist_json: Option<String> = row.get("allowlist")?;
        let denylist_json: Option<String> = row.get("denylist")?;
        let allowlist: Vec<String> = allowlist_json
            .as_deref()
            .map(|s| serde_json::from_str(s).unwrap_or_default())
            .unwrap_or_default();
        let denylist: Vec<String> = denylist_json
            .as_deref()
            .map(|s| serde_json::from_str(s).unwrap_or_default())
            .unwrap_or_default();
        Ok(Workspace {
            workspace_id: row.get("workspace_id")?,
            name: row.get("name")?,
            root_path: row.get("root_path")?,
            status: row.get("status")?,
            config_snapshot: row.get("config_snapshot")?,
            allowlist,
            denylist,
            created_at_unix: row.get("created_at_unix")?,
            updated_at_unix: row.get("updated_at_unix")?,
        })
    }
}

impl WorkspaceStore for SqliteWorkspaceStore {
    fn create(&self, req: &WorkspaceCreate) -> Result<Workspace, WorkspaceError> {
        Self::validate_create(req)?;
        let conn = self.conn.lock().expect("workspace conn mutex poisoned");
        let existing: Option<String> = conn
            .query_row(
                "SELECT workspace_id FROM workspaces WHERE workspace_id = ?1",
                params![req.workspace_id],
                |r| r.get(0),
            )
            .ok();
        if existing.is_some() {
            return Err(WorkspaceError::Invalid(format!(
                "workspace_id already exists: {}",
                req.workspace_id
            )));
        }
        let now = now_unix();
        let allowlist_json = serde_json::to_string(&req.allowlist)?;
        let denylist_json = serde_json::to_string(&req.denylist)?;
        let config_snapshot = "{}".to_string();
        conn.execute(
            "INSERT INTO workspaces (workspace_id, name, root_path, status, config_snapshot, allowlist, denylist, created_at_unix, updated_at_unix)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                req.workspace_id,
                req.name,
                req.root_path,
                "ready",
                config_snapshot,
                allowlist_json,
                denylist_json,
                now,
                now,
            ],
        )?;
        drop(conn);
        // create physical collection dir (1:1 mapping per ADR-015 §D2)
        let collection_dir = self.collection_dir(&req.workspace_id);
        std::fs::create_dir_all(&collection_dir)?;
        Ok(Workspace {
            workspace_id: req.workspace_id.clone(),
            name: req.name.clone(),
            root_path: req.root_path.clone(),
            status: "ready".to_string(),
            config_snapshot: "{}".to_string(),
            allowlist: req.allowlist.clone(),
            denylist: req.denylist.clone(),
            created_at_unix: now,
            updated_at_unix: now,
        })
    }

    fn list(&self) -> Result<Vec<Workspace>, WorkspaceError> {
        let conn = self.conn.lock().expect("workspace conn mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT workspace_id, name, root_path, status, config_snapshot, allowlist, denylist, created_at_unix, updated_at_unix
             FROM workspaces WHERE status != 'deleted' ORDER BY created_at_unix",
        )?;
        let rows = stmt.query_map([], Self::row_to_workspace)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    fn get(&self, workspace_id: &str) -> Result<Option<Workspace>, WorkspaceError> {
        Self::validate_id(workspace_id)?;
        let conn = self.conn.lock().expect("workspace conn mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT workspace_id, name, root_path, status, config_snapshot, allowlist, denylist, created_at_unix, updated_at_unix
             FROM workspaces WHERE workspace_id = ?1",
        )?;
        let mut rows = stmt.query(params![workspace_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::row_to_workspace(row)?))
        } else {
            Ok(None)
        }
    }

    fn update_config(
        &self,
        workspace_id: &str,
        allowlist: Vec<String>,
        denylist: Vec<String>,
    ) -> Result<Workspace, WorkspaceError> {
        Self::validate_id(workspace_id)?;
        let allowlist_json = serde_json::to_string(&allowlist)?;
        let denylist_json = serde_json::to_string(&denylist)?;
        let now = now_unix();
        let conn = self.conn.lock().expect("workspace conn mutex poisoned");
        let affected = conn.execute(
            "UPDATE workspaces SET allowlist = ?1, denylist = ?2, updated_at_unix = ?3, status = 'ready'
             WHERE workspace_id = ?4 AND status != 'deleted'",
            params![allowlist_json, denylist_json, now, workspace_id],
        )?;
        if affected == 0 {
            return Err(WorkspaceError::Invalid(format!(
                "workspace not found or deleted: {workspace_id}"
            )));
        }
        let mut stmt = conn.prepare(
            "SELECT workspace_id, name, root_path, status, config_snapshot, allowlist, denylist, created_at_unix, updated_at_unix
             FROM workspaces WHERE workspace_id = ?1",
        )?;
        let mut rows = stmt.query(params![workspace_id])?;
        let row = rows
            .next()?
            .ok_or_else(|| WorkspaceError::Invalid(format!("workspace vanished: {workspace_id}")))?;
        Self::row_to_workspace(row).map_err(Into::into)
    }

    fn soft_delete(&self, workspace_id: &str) -> Result<(), WorkspaceError> {
        Self::validate_id(workspace_id)?;
        let now = now_unix();
        let conn = self.conn.lock().expect("workspace conn mutex poisoned");
        let affected = conn.execute(
            "UPDATE workspaces SET status = 'deleted', updated_at_unix = ?1 WHERE workspace_id = ?2",
            params![now, workspace_id],
        )?;
        if affected == 0 {
            return Err(WorkspaceError::Invalid(format!(
                "workspace not found: {workspace_id}"
            )));
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("invalid workspace: {0}")]
    Invalid(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    fn fresh_store() -> (PathBuf, SqliteWorkspaceStore) {
        let unique = format!(
            "cfg-ws-test-{}-{}",
            std::process::id(),
            now_unix_nano()
        );
        let dir = env::temp_dir().join(unique);
        let _ = fs::remove_dir_all(&dir);
        let store = SqliteWorkspaceStore::open(&dir).expect("open");
        (dir, store)
    }

    fn abs_root_for_test(id: &str) -> String {
        env::temp_dir()
            .join(format!("cfg-ws-root-{}-{}", id, now_unix_nano()))
            .to_string_lossy()
            .into_owned()
    }

    fn now_unix_nano() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    }

    #[test]
    fn migration_applies() {
        let (dir, store) = fresh_store();
        let req = WorkspaceCreate {
            workspace_id: "demo".to_string(),
            name: "demo".to_string(),
            root_path: abs_root_for_test("demo"),
            allowlist: vec![],
            denylist: vec![],
        };
        let w = store.create(&req).expect("create");
        assert_eq!(w.workspace_id, "demo");
        assert_eq!(w.status, "ready");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn create_triggers_collection_dir() {
        let (dir, store) = fresh_store();
        let req = WorkspaceCreate {
            workspace_id: "demo".to_string(),
            name: "demo".to_string(),
            root_path: abs_root_for_test("demo"),
            ..Default::default()
        };
        store.create(&req).expect("create");
        let coll = dir.join("collections").join("demo");
        assert!(coll.exists(), "collection dir must be physically created");
        assert!(coll.is_dir());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn crud_happy_path() {
        let (dir, store) = fresh_store();
        let req = WorkspaceCreate {
            workspace_id: "alpha".to_string(),
            name: "alpha-name".to_string(),
            root_path: abs_root_for_test("alpha"),
            allowlist: vec!["*.md".to_string()],
            denylist: vec![".env".to_string()],
        };
        let created = store.create(&req).expect("create");
        assert_eq!(created.allowlist, vec!["*.md"]);
        let got = store.get("alpha").expect("get").expect("present");
        assert_eq!(got.name, "alpha-name");
        let listed = store.list().expect("list");
        assert_eq!(listed.len(), 1);
        let updated = store
            .update_config(
                "alpha",
                vec!["*.txt".to_string()],
                vec![".env".to_string(), ".ssh".to_string()],
            )
            .expect("update");
        assert_eq!(updated.allowlist, vec!["*.txt"]);
        assert_eq!(updated.denylist.len(), 2);
        store.soft_delete("alpha").expect("delete");
        let post = store.get("alpha").expect("get post-delete");
        assert!(post.is_some(), "soft-delete preserves row");
        assert_eq!(post.unwrap().status, "deleted");
        let listed_post = store.list().expect("list post-delete");
        assert!(
            listed_post.is_empty(),
            "soft-deleted rows excluded from default list"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_input_returns_invalid_error() {
        let (dir, store) = fresh_store();
        let req = WorkspaceCreate {
            workspace_id: "x".to_string(),
            name: "".to_string(),
            root_path: abs_root_for_test("x"),
            ..Default::default()
        };
        let err = store.create(&req).expect_err("empty name should fail");
        assert!(matches!(err, WorkspaceError::Invalid(_)));
        let req2 = WorkspaceCreate {
            workspace_id: "y".to_string(),
            name: "y".to_string(),
            root_path: "relative/path".to_string(),
            ..Default::default()
        };
        let err2 = store.create(&req2).expect_err("non-absolute should fail");
        assert!(matches!(err2, WorkspaceError::Invalid(_)));
        let req3 = WorkspaceCreate {
            workspace_id: "dup".to_string(),
            name: "dup".to_string(),
            root_path: abs_root_for_test("dup"),
            ..Default::default()
        };
        store.create(&req3).expect("first create");
        let err3 = store.create(&req3).expect_err("duplicate should fail");
        assert!(matches!(err3, WorkspaceError::Invalid(_)));
        let req4 = WorkspaceCreate {
            workspace_id: "with spaces!".to_string(),
            name: "x".to_string(),
            root_path: abs_root_for_test("ws"),
            ..Default::default()
        };
        let err4 = store.create(&req4).expect_err("invalid id chars should fail");
        assert!(matches!(err4, WorkspaceError::Invalid(_)));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn status_transitions() {
        let (dir, store) = fresh_store();
        let req = WorkspaceCreate {
            workspace_id: "t".to_string(),
            name: "t".to_string(),
            root_path: abs_root_for_test("t"),
            ..Default::default()
        };
        let w = store.create(&req).expect("create");
        assert_eq!(w.status, "ready");
        store
            .update_config("t", vec![], vec![".env".to_string()])
            .expect("update");
        let w2 = store.get("t").expect("get").expect("present");
        assert_eq!(w2.status, "ready");
        store.soft_delete("t").expect("delete");
        let w3 = store.get("t").expect("get").expect("present");
        assert_eq!(w3.status, "deleted");
        let _ = fs::remove_dir_all(&dir);
    }
}
