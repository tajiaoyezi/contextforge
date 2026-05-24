//! task-10.3 (Phase 10): jobs — Console Contract v1 IndexJob 资源 +
//! 异步 lifecycle (queued/running/succeeded/failed/cancelled) + heartbeat +
//! co-operative cancel.
//!
//! 字段对齐 Console contractv1.IndexJob must-have 字段。
//!
//! 设计 (ADR-015 §D3):
//! - JobStore: SQLite 持久化 (CRUD + 状态转移 + cancel flag)
//! - JobRunner: 异步执行器, 接受 IndexerBackend 注入；run_one 同步 spawn_blocking
//!   跑 indexer，回调内每 N 文件 / 每 5s 心跳 + 检查 cancel
//! - co-operative cancel: cancel_requested SQL 列；indexer 回调检查后返回
//!   Decision::Cancel 让 JobRunner 写 status=cancelled
//!
//! 时间字段：Unix epoch i64 秒（同 task-10.2 trade-off — 避新增 chrono dep）
//!
//! Refs: ADR-015 §D3 / phase-10 §6 AC3 / task-10.3 §6 AC1-5

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const MIGRATION_SQL: &str = include_str!("../../migrations/0011_index_jobs.sql");

const STATUS_QUEUED: &str = "queued";
const STATUS_RUNNING: &str = "running";
const STATUS_SUCCEEDED: &str = "succeeded";
const STATUS_FAILED: &str = "failed";
const STATUS_CANCELLED: &str = "cancelled";

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// IndexJob — Console contractv1.IndexJob 镜像（Rust 侧持久化模型）.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexJob {
    pub job_id: String,
    pub workspace_id: String,
    pub trigger_source: String,
    pub status: String,
    pub stage: String,
    pub processed_files: i64,
    pub total_files: i64,
    pub failed_files: i64,
    pub skipped_files: i64,
    pub error_message: Option<String>,
    pub started_at_unix: Option<i64>,
    pub finished_at_unix: Option<i64>,
    pub last_heartbeat_at_unix: Option<i64>,
}

/// JobStore trait — CRUD + 状态转移 + cancel flag.
pub trait JobStore: Send + Sync {
    fn enqueue(&self, workspace_id: &str, trigger_source: &str) -> Result<IndexJob, JobError>;
    fn get(&self, job_id: &str) -> Result<Option<IndexJob>, JobError>;
    fn list_active(&self) -> Result<Vec<IndexJob>, JobError>;
    fn request_cancel(&self, job_id: &str) -> Result<bool, JobError>;
    fn update_progress(
        &self,
        job_id: &str,
        stage: &str,
        processed: i64,
        total: i64,
        failed: i64,
        skipped: i64,
    ) -> Result<(), JobError>;
    fn mark_running(&self, job_id: &str) -> Result<(), JobError>;
    fn mark_terminal(
        &self,
        job_id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), JobError>;
    fn is_cancel_requested(&self, job_id: &str) -> Result<bool, JobError>;
    fn touch_heartbeat(&self, job_id: &str) -> Result<(), JobError>;
}

/// SQLite 实现 — 假设 workspaces.db 已含 workspaces 表 (FK).
pub struct SqliteJobStore {
    conn: Mutex<Connection>,
}

impl SqliteJobStore {
    /// 打开（共享 workspaces.db SQLite file）+ apply migration.
    pub fn open(data_dir: &Path) -> Result<Self, JobError> {
        std::fs::create_dir_all(data_dir)?;
        let db_path = data_dir.join("workspaces.db");
        let conn = Connection::open(&db_path)?;
        // foreign keys ON (rusqlite 默认 OFF)
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        conn.execute_batch(MIGRATION_SQL)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn new_job_id() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("job-{nanos:x}")
    }

    fn row_to_job(row: &rusqlite::Row<'_>) -> rusqlite::Result<IndexJob> {
        Ok(IndexJob {
            job_id: row.get("job_id")?,
            workspace_id: row.get("workspace_id")?,
            trigger_source: row.get("trigger_source")?,
            status: row.get("status")?,
            stage: row.get("stage")?,
            processed_files: row.get("processed_files")?,
            total_files: row.get("total_files")?,
            failed_files: row.get("failed_files")?,
            skipped_files: row.get("skipped_files")?,
            error_message: row.get("error_message")?,
            started_at_unix: row.get("started_at_unix")?,
            finished_at_unix: row.get("finished_at_unix")?,
            last_heartbeat_at_unix: row.get("last_heartbeat_at_unix")?,
        })
    }
}

impl JobStore for SqliteJobStore {
    fn enqueue(&self, workspace_id: &str, trigger_source: &str) -> Result<IndexJob, JobError> {
        if workspace_id.is_empty() {
            return Err(JobError::InvalidState("workspace_id must not be empty".into()));
        }
        // verify workspace exists
        let conn = self.conn.lock().expect("jobs conn mutex poisoned");
        let ws_exists: Option<String> = conn
            .query_row(
                "SELECT workspace_id FROM workspaces WHERE workspace_id = ?1 AND status != 'deleted'",
                params![workspace_id],
                |r| r.get(0),
            )
            .ok();
        if ws_exists.is_none() {
            return Err(JobError::WorkspaceNotFound(workspace_id.into()));
        }
        let job_id = Self::new_job_id();
        conn.execute(
            "INSERT INTO index_jobs (job_id, workspace_id, trigger_source, status, stage, processed_files, total_files, failed_files, skipped_files, cancel_requested)
             VALUES (?1, ?2, ?3, ?4, '', 0, 0, 0, 0, 0)",
            params![job_id, workspace_id, trigger_source, STATUS_QUEUED],
        )?;
        let mut stmt = conn.prepare("SELECT * FROM index_jobs WHERE job_id = ?1")?;
        let mut rows = stmt.query(params![job_id])?;
        let row = rows
            .next()?
            .ok_or_else(|| JobError::InvalidState(format!("just-inserted job vanished: {job_id}")))?;
        Self::row_to_job(row).map_err(Into::into)
    }

    fn get(&self, job_id: &str) -> Result<Option<IndexJob>, JobError> {
        let conn = self.conn.lock().expect("jobs conn mutex poisoned");
        let mut stmt = conn.prepare("SELECT * FROM index_jobs WHERE job_id = ?1")?;
        let mut rows = stmt.query(params![job_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::row_to_job(row)?))
        } else {
            Ok(None)
        }
    }

    fn list_active(&self) -> Result<Vec<IndexJob>, JobError> {
        let conn = self.conn.lock().expect("jobs conn mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT * FROM index_jobs WHERE status IN ('queued', 'running') ORDER BY job_id",
        )?;
        let rows = stmt.query_map([], Self::row_to_job)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    fn request_cancel(&self, job_id: &str) -> Result<bool, JobError> {
        let conn = self.conn.lock().expect("jobs conn mutex poisoned");
        let status: Option<String> = conn
            .query_row(
                "SELECT status FROM index_jobs WHERE job_id = ?1",
                params![job_id],
                |r| r.get(0),
            )
            .ok();
        match status.as_deref() {
            None => Err(JobError::InvalidState(format!("job not found: {job_id}"))),
            Some(STATUS_SUCCEEDED) | Some(STATUS_FAILED) | Some(STATUS_CANCELLED) => Ok(false),
            Some(_) => {
                conn.execute(
                    "UPDATE index_jobs SET cancel_requested = 1 WHERE job_id = ?1",
                    params![job_id],
                )?;
                Ok(true)
            }
        }
    }

    fn update_progress(
        &self,
        job_id: &str,
        stage: &str,
        processed: i64,
        total: i64,
        failed: i64,
        skipped: i64,
    ) -> Result<(), JobError> {
        let conn = self.conn.lock().expect("jobs conn mutex poisoned");
        let now = now_unix();
        let affected = conn.execute(
            "UPDATE index_jobs SET stage = ?1, processed_files = ?2, total_files = ?3, failed_files = ?4, skipped_files = ?5, last_heartbeat_at_unix = ?6
             WHERE job_id = ?7",
            params![stage, processed, total, failed, skipped, now, job_id],
        )?;
        if affected == 0 {
            return Err(JobError::InvalidState(format!("job not found: {job_id}")));
        }
        Ok(())
    }

    fn mark_running(&self, job_id: &str) -> Result<(), JobError> {
        let conn = self.conn.lock().expect("jobs conn mutex poisoned");
        let now = now_unix();
        // only legal transition: queued → running
        let affected = conn.execute(
            "UPDATE index_jobs SET status = ?1, started_at_unix = ?2, last_heartbeat_at_unix = ?2
             WHERE job_id = ?3 AND status = ?4",
            params![STATUS_RUNNING, now, job_id, STATUS_QUEUED],
        )?;
        if affected == 0 {
            return Err(JobError::InvalidState(format!(
                "mark_running: job {job_id} not in {STATUS_QUEUED} state"
            )));
        }
        Ok(())
    }

    fn mark_terminal(
        &self,
        job_id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), JobError> {
        if status != STATUS_SUCCEEDED && status != STATUS_FAILED && status != STATUS_CANCELLED {
            return Err(JobError::InvalidState(format!(
                "mark_terminal: invalid terminal status {status}"
            )));
        }
        let conn = self.conn.lock().expect("jobs conn mutex poisoned");
        let now = now_unix();
        let affected = conn.execute(
            "UPDATE index_jobs SET status = ?1, finished_at_unix = ?2, error_message = ?3
             WHERE job_id = ?4 AND status IN ('queued', 'running')",
            params![status, now, error, job_id],
        )?;
        if affected == 0 {
            return Err(JobError::InvalidState(format!(
                "mark_terminal: job {job_id} already terminal or not found"
            )));
        }
        Ok(())
    }

    fn is_cancel_requested(&self, job_id: &str) -> Result<bool, JobError> {
        let conn = self.conn.lock().expect("jobs conn mutex poisoned");
        let flag: Option<i64> = conn
            .query_row(
                "SELECT cancel_requested FROM index_jobs WHERE job_id = ?1",
                params![job_id],
                |r| r.get(0),
            )
            .ok();
        match flag {
            None => Err(JobError::InvalidState(format!("job not found: {job_id}"))),
            Some(v) => Ok(v != 0),
        }
    }

    fn touch_heartbeat(&self, job_id: &str) -> Result<(), JobError> {
        let conn = self.conn.lock().expect("jobs conn mutex poisoned");
        let now = now_unix();
        let _ = conn.execute(
            "UPDATE index_jobs SET last_heartbeat_at_unix = ?1 WHERE job_id = ?2",
            params![now, job_id],
        )?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("workspace not found: {0}")]
    WorkspaceNotFound(String),
    #[error("invalid state: {0}")]
    InvalidState(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("indexer: {0}")]
    Indexer(String),
}

/// task-11.3 (Phase 11): real `IndexerBackend` impl wrapping `IndexSession::
/// index_path_cancellable`. See `index_session_backend.rs`.
pub mod index_session_backend;
pub use index_session_backend::IndexSessionBackend;

/// IndexerBackend trait — JobRunner 注入式 indexer 接口（实际实现可能包装
/// IndexSession::index_path_with_progress，但本 trait 不依赖 indexer 模块以
/// 便测试 + 解耦）。回调返回 Decision::Cancel 让 indexer 提前退出。
///
/// task-11.4: `job_id` added so implementations can tag emitted EventBus
/// events with the originating job (required by Console UI for per-job
/// progress filtering).
pub trait IndexerBackend: Send + Sync {
    fn index(
        &self,
        source: &Path,
        data: &Path,
        workspace_id: &str,
        job_id: &str,
        on_progress: &mut dyn FnMut(&JobProgressEvent) -> ProgressDecision,
    ) -> Result<JobOutcome, String>;
}

#[derive(Debug, Clone)]
pub struct JobProgressEvent {
    pub processed_files: i64,
    pub total_files: i64,
    pub failed_files: i64,
    pub skipped_files: i64,
    pub current_file: String,
    pub stage: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressDecision {
    Continue,
    Cancel,
}

#[derive(Debug, Clone, Default)]
pub struct JobOutcome {
    pub processed_files: i64,
    pub total_files: i64,
    pub failed_files: i64,
    pub skipped_files: i64,
    pub cancelled: bool,
}

/// JobRunner — 异步执行器；run_one 同步 spawn_blocking 跑 indexer + 回调内
/// 心跳 + 检查 cancel.
pub struct JobRunner<I: IndexerBackend + 'static> {
    pub store: Arc<dyn JobStore>,
    pub indexer: Arc<I>,
    /// 心跳 / cancel 检查最小间隔（秒）；默认 5s.
    pub heartbeat_interval_secs: i64,
    /// 心跳 / cancel 检查最小文件 boundary 数；默认 100.
    pub heartbeat_every_n_files: i64,
}

impl<I: IndexerBackend + 'static> JobRunner<I> {
    pub fn new(store: Arc<dyn JobStore>, indexer: Arc<I>) -> Self {
        Self {
            store,
            indexer,
            heartbeat_interval_secs: 5,
            heartbeat_every_n_files: 100,
        }
    }

    /// 跑一个 job — 阻塞当前调用者直到 terminal. async 函数（tokio 友好）但
    /// 内部 spawn_blocking 隔离 sync indexer。
    pub async fn run_one(
        &self,
        job_id: &str,
        source: &Path,
        data: &Path,
    ) -> Result<(), JobError> {
        let store = self.store.clone();
        let indexer = self.indexer.clone();
        let job_id_owned = job_id.to_string();
        let job_id_for_closure = job_id_owned.clone();
        let source_owned = source.to_path_buf();
        let data_owned = data.to_path_buf();
        let heartbeat_interval = self.heartbeat_interval_secs;
        let heartbeat_every_n = self.heartbeat_every_n_files;

        // fetch job → get workspace_id
        let job = store
            .get(&job_id_owned)?
            .ok_or_else(|| JobError::InvalidState(format!("job not found: {job_id_owned}")))?;
        let workspace_id = job.workspace_id.clone();

        store.mark_running(&job_id_owned)?;

        let store_for_blocking = store.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut last_heartbeat_unix = now_unix();
            let mut last_heartbeat_files: i64 = 0;
            let mut cancelled = false;

            let mut on_progress = |evt: &JobProgressEvent| -> ProgressDecision {
                // heartbeat condition (task-11.3: every 100 files OR 5s)
                let now = now_unix();
                let time_elapsed = now - last_heartbeat_unix;
                let files_since = evt.processed_files - last_heartbeat_files;
                if time_elapsed >= heartbeat_interval || files_since >= heartbeat_every_n {
                    let _ = store_for_blocking.update_progress(
                        &job_id_for_closure,
                        &evt.stage,
                        evt.processed_files,
                        evt.total_files,
                        evt.failed_files,
                        evt.skipped_files,
                    );
                    last_heartbeat_unix = now;
                    last_heartbeat_files = evt.processed_files;
                }
                // task-11.3 §6 AC3: cancel check is per-file (not only at
                // heartbeat boundary) so small fixtures still observe cancel
                // within the iteration window. The check is a single SELECT
                // — cheaper than the per-file Tantivy chunk write that just
                // ran. Heartbeat SQL update is the throttled write; cancel
                // SELECT is the always-poll signal.
                if let Ok(true) = store_for_blocking.is_cancel_requested(&job_id_for_closure) {
                    cancelled = true;
                    return ProgressDecision::Cancel;
                }
                ProgressDecision::Continue
            };

            let result = indexer.index(&source_owned, &data_owned, &workspace_id, &job_id_for_closure, &mut on_progress);
            (result, cancelled)
        })
        .await
        .map_err(|e| JobError::Indexer(format!("spawn_blocking joined with error: {e}")))?;

        let (indexer_result, cancelled) = result;
        // final mark
        if cancelled {
            self.store.mark_terminal(&job_id_owned, STATUS_CANCELLED, None)?;
            return Ok(());
        }
        match indexer_result {
            Ok(outcome) => {
                let _ = self.store.update_progress(
                    &job_id_owned,
                    "done",
                    outcome.processed_files,
                    outcome.total_files,
                    outcome.failed_files,
                    outcome.skipped_files,
                );
                if outcome.cancelled {
                    self.store.mark_terminal(&job_id_owned, STATUS_CANCELLED, None)?;
                } else {
                    self.store.mark_terminal(&job_id_owned, STATUS_SUCCEEDED, None)?;
                }
                Ok(())
            }
            Err(msg) => {
                self.store
                    .mark_terminal(&job_id_owned, STATUS_FAILED, Some(&msg))?;
                Ok(())
            }
        }
    }
}

// Re-export legal terminal status constants for callers.
pub mod status {
    pub const QUEUED: &str = super::STATUS_QUEUED;
    pub const RUNNING: &str = super::STATUS_RUNNING;
    pub const SUCCEEDED: &str = super::STATUS_SUCCEEDED;
    pub const FAILED: &str = super::STATUS_FAILED;
    pub const CANCELLED: &str = super::STATUS_CANCELLED;
}

/// task-11.3 §6 AC4: orphan reaper — at daemon `serve_full` startup, scan
/// every queued/running job left over from a previous boot (the JobRunner
/// owning them is dead) and mark them as failed/cancelled. Returns the count
/// reaped.
///
/// - `cancel_requested == true` → status=cancelled + error_message="user
///   requested cancel; daemon restarted mid-cancel"
/// - else → status=failed + error_message="job lost: daemon restart"
///
/// Must run BEFORE the gRPC server binds, so no fresh Enqueue can see a stale
/// running row from the previous boot.
pub fn orphan_reaper(store: &SqliteJobStore) -> Result<usize, JobError> {
    let active = store.list_active()?;
    let mut count = 0usize;
    for job in active {
        let cancel_requested = store
            .is_cancel_requested(&job.job_id)
            .unwrap_or(false);
        let (status_label, msg) = if cancel_requested {
            (
                STATUS_CANCELLED,
                "user requested cancel; daemon restarted mid-cancel",
            )
        } else {
            (STATUS_FAILED, "job lost: daemon restart")
        };
        store.mark_terminal(&job.job_id, status_label, Some(msg))?;
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{SqliteWorkspaceStore, WorkspaceCreate, WorkspaceStore};
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicI64;
    use std::time::Duration;

    fn unique_data_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        env::temp_dir().join(format!(
            "cfg-jobs-test-{}-{}-{}",
            label,
            std::process::id(),
            nanos
        ))
    }

    fn fresh_workspace_and_jobs(label: &str) -> (PathBuf, SqliteWorkspaceStore, SqliteJobStore) {
        let dir = unique_data_dir(label);
        let _ = fs::remove_dir_all(&dir);
        let ws_store = SqliteWorkspaceStore::open(&dir).expect("ws open");
        let root_path = env::temp_dir()
            .join(format!("cfg-jobs-root-{}", label))
            .to_string_lossy()
            .into_owned();
        ws_store
            .create(&WorkspaceCreate {
                workspace_id: "demo".to_string(),
                name: "demo".to_string(),
                root_path,
                ..Default::default()
            })
            .expect("ws create");
        let job_store = SqliteJobStore::open(&dir).expect("job open");
        (dir, ws_store, job_store)
    }

    #[test]
    fn migration_applies() {
        let (dir, _ws, store) = fresh_workspace_and_jobs("mig");
        let j = store.enqueue("demo", "cli").expect("enqueue");
        assert_eq!(j.status, status::QUEUED);
        assert_eq!(j.processed_files, 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn enqueue_get_list_cancel() {
        let (dir, _ws, store) = fresh_workspace_and_jobs("crud");
        let j = store.enqueue("demo", "rest").expect("enqueue");
        let got = store.get(&j.job_id).expect("get").expect("present");
        assert_eq!(got.workspace_id, "demo");
        let active = store.list_active().expect("list");
        assert_eq!(active.len(), 1);
        let cancelled = store.request_cancel(&j.job_id).expect("cancel");
        assert!(cancelled);
        let flag = store.is_cancel_requested(&j.job_id).expect("flag");
        assert!(flag);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn enqueue_workspace_not_found() {
        let (dir, _ws, store) = fresh_workspace_and_jobs("wsnf");
        let err = store
            .enqueue("non-existent", "cli")
            .expect_err("non-existent ws should fail");
        assert!(matches!(err, JobError::WorkspaceNotFound(_)));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn status_transitions() {
        let (dir, _ws, store) = fresh_workspace_and_jobs("status");
        let j = store.enqueue("demo", "cli").expect("enqueue");
        // illegal: mark_running on already-running
        store.mark_running(&j.job_id).expect("queued → running");
        let bad = store.mark_running(&j.job_id);
        assert!(bad.is_err(), "running → running must fail");
        // succeeded terminal
        store
            .mark_terminal(&j.job_id, status::SUCCEEDED, None)
            .expect("running → succeeded");
        // illegal: terminal → terminal
        let bad2 = store.mark_terminal(&j.job_id, status::FAILED, Some("x"));
        assert!(bad2.is_err(), "terminal → terminal must fail");
        // invalid terminal value
        let j2 = store.enqueue("demo", "cli").expect("enqueue 2");
        store.mark_running(&j2.job_id).expect("running 2");
        let bad3 = store.mark_terminal(&j2.job_id, "bogus", None);
        assert!(bad3.is_err(), "invalid terminal status must fail");
        // request_cancel on terminal returns false
        let cancelled = store.request_cancel(&j.job_id).expect("cancel on terminal");
        assert!(!cancelled, "request_cancel on terminal must return false");
        let _ = fs::remove_dir_all(&dir);
    }

    // ---------------- JobRunner integration ----------------

    struct CountingIndexer {
        total: i64,
        // count down by 1 each callback (simulates per-file work)
        delay_ms: u64,
    }

    impl IndexerBackend for CountingIndexer {
        fn index(
            &self,
            _source: &Path,
            _data: &Path,
            _workspace_id: &str,
            _job_id: &str,
            on_progress: &mut dyn FnMut(&JobProgressEvent) -> ProgressDecision,
        ) -> Result<JobOutcome, String> {
            let mut cancelled = false;
            let mut processed = 0;
            for i in 1..=self.total {
                processed = i;
                let evt = JobProgressEvent {
                    processed_files: i,
                    total_files: self.total,
                    failed_files: 0,
                    skipped_files: 0,
                    current_file: format!("/fake/{i}.md"),
                    stage: "index".to_string(),
                };
                if matches!(on_progress(&evt), ProgressDecision::Cancel) {
                    cancelled = true;
                    break;
                }
                if self.delay_ms > 0 {
                    std::thread::sleep(Duration::from_millis(self.delay_ms));
                }
            }
            Ok(JobOutcome {
                processed_files: processed,
                total_files: self.total,
                failed_files: 0,
                skipped_files: 0,
                cancelled,
            })
        }
    }

    struct FailingIndexer;

    impl IndexerBackend for FailingIndexer {
        fn index(
            &self,
            _: &Path,
            _: &Path,
            _: &str,
            _: &str,
            _on_progress: &mut dyn FnMut(&JobProgressEvent) -> ProgressDecision,
        ) -> Result<JobOutcome, String> {
            Err("simulated indexer failure".to_string())
        }
    }

    #[tokio::test]
    async fn jobrunner_happy_path() {
        let (dir, _ws, store) = fresh_workspace_and_jobs("happy");
        let store_arc: Arc<dyn JobStore> = Arc::new(store);
        let indexer = Arc::new(CountingIndexer { total: 5, delay_ms: 0 });
        let mut runner = JobRunner::new(store_arc.clone(), indexer);
        // make heartbeat boundary easy to hit so update_progress writes
        runner.heartbeat_every_n_files = 1;
        runner.heartbeat_interval_secs = 0;
        let j = store_arc.enqueue("demo", "rest").expect("enqueue");
        let src = env::temp_dir();
        let data = env::temp_dir();
        runner.run_one(&j.job_id, &src, &data).await.expect("run");
        let final_job = store_arc.get(&j.job_id).expect("get").expect("present");
        assert_eq!(final_job.status, status::SUCCEEDED);
        assert_eq!(final_job.processed_files, 5);
        assert_eq!(final_job.total_files, 5);
        assert!(final_job.started_at_unix.is_some());
        assert!(final_job.finished_at_unix.is_some());
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn jobrunner_mid_run_cancel() {
        let (dir, _ws, store) = fresh_workspace_and_jobs("cancel");
        let store_arc: Arc<dyn JobStore> = Arc::new(store);
        let indexer = Arc::new(CountingIndexer { total: 100, delay_ms: 20 });
        let mut runner = JobRunner::new(store_arc.clone(), indexer);
        // request cancel as soon as first heartbeat boundary hit
        runner.heartbeat_every_n_files = 5;
        runner.heartbeat_interval_secs = 0;
        let j = store_arc.enqueue("demo", "rest").expect("enqueue");
        let store_for_cancel = store_arc.clone();
        let job_id_for_cancel = j.job_id.clone();
        // schedule cancel slightly after run starts
        let cancel_task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let _ = store_for_cancel.request_cancel(&job_id_for_cancel);
        });
        let src = env::temp_dir();
        let data = env::temp_dir();
        runner.run_one(&j.job_id, &src, &data).await.expect("run");
        let _ = cancel_task.await;
        let final_job = store_arc.get(&j.job_id).expect("get").expect("present");
        assert_eq!(final_job.status, status::CANCELLED);
        assert!(
            final_job.processed_files < 100,
            "cancel should stop before full 100 — got {}",
            final_job.processed_files
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn jobrunner_indexer_error_marks_failed() {
        let (dir, _ws, store) = fresh_workspace_and_jobs("fail");
        let store_arc: Arc<dyn JobStore> = Arc::new(store);
        let indexer = Arc::new(FailingIndexer);
        let runner = JobRunner::new(store_arc.clone(), indexer);
        let j = store_arc.enqueue("demo", "rest").expect("enqueue");
        let src = env::temp_dir();
        let data = env::temp_dir();
        runner.run_one(&j.job_id, &src, &data).await.expect("run");
        let final_job = store_arc.get(&j.job_id).expect("get").expect("present");
        assert_eq!(final_job.status, status::FAILED);
        assert!(final_job.error_message.is_some());
        assert!(final_job.error_message.unwrap().contains("simulated"));
        let _ = fs::remove_dir_all(&dir);
    }

    // ensure unused; silence dead-code warning on AtomicI64 import for some compilers
    fn _unused_atomic() -> AtomicI64 {
        AtomicI64::new(0)
    }
}
