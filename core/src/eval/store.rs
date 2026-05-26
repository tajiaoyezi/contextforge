//! task-14.1 SqliteEvalStore — CRUD + progress updates backed by `eval_runs`
//! SQLite table (migration `0014_eval_runs.sql`).
//!
//! Concurrency: std::sync::Mutex<Connection> (rusqlite Connection is !Send-by-default).
//! JSON serialization: serde_json (already a direct dep).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, Error as RusqliteError};
use serde::{Deserialize, Serialize};

const MIGRATION_SQL: &str = include_str!("../../migrations/0014_eval_runs.sql");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CaseResult {
    pub case_id: String,
    pub query: String,
    pub expected_chunks: Vec<String>,
    pub actual_chunks: Vec<String>,
    pub score: f64,
    pub passed: bool,
}

#[derive(Debug, Clone)]
pub struct EvalRun {
    pub eval_run_id: String,
    pub workspace_id: String,
    pub status: String,
    pub config_snapshot_json: String,
    pub started_at_unix: i64,
    pub finished_at_unix: Option<i64>,
    pub metrics: HashMap<String, f64>,
    pub case_results: Vec<CaseResult>,
    pub schema_version: String,
    pub dataset_ref: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EvalRunCreate {
    pub eval_run_id: String,
    pub workspace_id: String,
    pub config_snapshot_json: String,
    pub dataset_ref: Option<String>,
}

#[derive(Debug)]
pub enum EvalStoreError {
    NotFound,
    Invalid(String),
    Sqlite(String),
    Json(String),
    Io(std::io::Error),
}

impl std::fmt::Display for EvalStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalStoreError::NotFound => write!(f, "eval run not found"),
            EvalStoreError::Invalid(m) => write!(f, "invalid: {m}"),
            EvalStoreError::Sqlite(m) => write!(f, "sqlite: {m}"),
            EvalStoreError::Json(m) => write!(f, "json: {m}"),
            EvalStoreError::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for EvalStoreError {}

impl From<RusqliteError> for EvalStoreError {
    fn from(e: RusqliteError) -> Self {
        EvalStoreError::Sqlite(e.to_string())
    }
}

impl From<serde_json::Error> for EvalStoreError {
    fn from(e: serde_json::Error) -> Self {
        EvalStoreError::Json(e.to_string())
    }
}

impl From<std::io::Error> for EvalStoreError {
    fn from(e: std::io::Error) -> Self {
        EvalStoreError::Io(e)
    }
}

pub struct SqliteEvalStore {
    conn: Mutex<Connection>,
}

impl SqliteEvalStore {
    pub fn open(data_dir: &Path) -> Result<Self, EvalStoreError> {
        std::fs::create_dir_all(data_dir)?;
        let conn = Connection::open(data_dir.join("eval.db"))?;
        conn.execute_batch(MIGRATION_SQL)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// INSERT a new eval_run row with status=running + started_at=now.
    pub fn create(&self, req: EvalRunCreate) -> Result<EvalRun, EvalStoreError> {
        let conn = self.lock()?;
        let now = now_unix();
        let config = if req.config_snapshot_json.is_empty() {
            "{}".to_string()
        } else {
            req.config_snapshot_json.clone()
        };
        conn.execute(
            "INSERT INTO eval_runs \
             (eval_run_id, workspace_id, status, config_snapshot_json, started_at_unix, \
              finished_at_unix, metrics_json, case_results_json, schema_version, dataset_ref, error_message) \
             VALUES (?, ?, 'running', ?, ?, NULL, '{}', '[]', 'v1', ?, NULL)",
            params![
                req.eval_run_id,
                req.workspace_id,
                config,
                now,
                req.dataset_ref,
            ],
        )?;
        Ok(EvalRun {
            eval_run_id: req.eval_run_id,
            workspace_id: req.workspace_id,
            status: "running".into(),
            config_snapshot_json: config,
            started_at_unix: now,
            finished_at_unix: None,
            metrics: HashMap::new(),
            case_results: vec![],
            schema_version: "v1".into(),
            dataset_ref: req.dataset_ref,
            error_message: None,
        })
    }

    /// Get by eval_run_id; returns Ok(None) on miss. JSON columns are decoded.
    pub fn get(&self, eval_run_id: &str) -> Result<Option<EvalRun>, EvalStoreError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT eval_run_id, workspace_id, status, config_snapshot_json, started_at_unix, \
             finished_at_unix, metrics_json, case_results_json, schema_version, dataset_ref, error_message \
             FROM eval_runs WHERE eval_run_id = ? LIMIT 1",
        )?;
        let mut rows = stmt.query(params![eval_run_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row_to_run(row)?))
        } else {
            Ok(None)
        }
    }

    /// UPDATE metrics_json column; bumps nothing else.
    pub fn update_metrics(
        &self,
        eval_run_id: &str,
        metrics: HashMap<String, f64>,
    ) -> Result<(), EvalStoreError> {
        let conn = self.lock()?;
        let metrics_json = serde_json::to_string(&metrics)?;
        let n = conn.execute(
            "UPDATE eval_runs SET metrics_json = ? WHERE eval_run_id = ?",
            params![metrics_json, eval_run_id],
        )?;
        if n == 0 {
            Err(EvalStoreError::NotFound)
        } else {
            Ok(())
        }
    }

    pub fn update_case_results(
        &self,
        eval_run_id: &str,
        results: Vec<CaseResult>,
    ) -> Result<(), EvalStoreError> {
        let conn = self.lock()?;
        let json = serde_json::to_string(&results)?;
        let n = conn.execute(
            "UPDATE eval_runs SET case_results_json = ? WHERE eval_run_id = ?",
            params![json, eval_run_id],
        )?;
        if n == 0 {
            Err(EvalStoreError::NotFound)
        } else {
            Ok(())
        }
    }

    pub fn mark_finished(
        &self,
        eval_run_id: &str,
        status: &str,
        finished_at_unix: i64,
        error_message: Option<String>,
    ) -> Result<(), EvalStoreError> {
        if !matches!(status, "succeeded" | "failed" | "cancelled") {
            return Err(EvalStoreError::Invalid(format!(
                "terminal status must be succeeded/failed/cancelled; got {status}"
            )));
        }
        let conn = self.lock()?;
        let n = conn.execute(
            "UPDATE eval_runs SET status = ?, finished_at_unix = ?, error_message = ? WHERE eval_run_id = ?",
            params![status, finished_at_unix, error_message, eval_run_id],
        )?;
        if n == 0 {
            Err(EvalStoreError::NotFound)
        } else {
            Ok(())
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, EvalStoreError> {
        self.conn
            .lock()
            .map_err(|e| EvalStoreError::Invalid(format!("lock: {e}")))
    }

    /// task-15.4 (Phase 15 P1 #4): list eval runs ordered by started_at DESC,
    /// optionally filtered by workspace_id / status. `limit` is clamped to
    /// [1, 200] with a default of 50 (the caller's None/Some maps to default).
    pub fn list(&self, filter: ListEvalRunsFilter) -> Result<Vec<EvalRun>, EvalStoreError> {
        let limit = filter.limit.clamp(1, 200);
        let mut sql = String::from(
            "SELECT eval_run_id, workspace_id, status, config_snapshot_json, started_at_unix, \
             finished_at_unix, metrics_json, case_results_json, schema_version, dataset_ref, error_message \
             FROM eval_runs",
        );
        let mut clauses: Vec<&'static str> = Vec::new();
        if filter.workspace_id.is_some() {
            clauses.push("workspace_id = ?");
        }
        if filter.status.is_some() {
            clauses.push("status = ?");
        }
        if !clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&clauses.join(" AND "));
        }
        sql.push_str(" ORDER BY started_at_unix DESC LIMIT ?");

        let conn = self.lock()?;
        let mut stmt = conn.prepare(&sql)?;
        // Bind params dynamically in order (ws, status, limit). rusqlite's
        // ParamsFromIter accepts `&[&dyn ToSql]` of mixed types.
        let ws_owned: Option<String> = filter.workspace_id.clone();
        let st_owned: Option<String> = filter.status.clone();
        let mut params_dyn: Vec<&dyn rusqlite::ToSql> = Vec::new();
        if let Some(ws) = ws_owned.as_ref() {
            params_dyn.push(ws);
        }
        if let Some(st) = st_owned.as_ref() {
            params_dyn.push(st);
        }
        params_dyn.push(&limit);
        let mut rows = stmt.query(rusqlite::params_from_iter(params_dyn))?;
        let mut out: Vec<EvalRun> = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(row_to_run(row)?);
        }
        Ok(out)
    }
}

/// task-15.4 filter struct for `SqliteEvalStore::list`. None = no constraint
/// on that column; non-None = exact match. `limit` clamped to [1, 200]; pass
/// 50 for the default.
#[derive(Debug, Clone)]
pub struct ListEvalRunsFilter {
    pub workspace_id: Option<String>,
    pub status: Option<String>,
    pub limit: i64,
}

fn row_to_run(row: &rusqlite::Row<'_>) -> Result<EvalRun, EvalStoreError> {
    let metrics_json: String = row.get(6)?;
    let case_json: String = row.get(7)?;
    let metrics: HashMap<String, f64> = serde_json::from_str(&metrics_json)?;
    let case_results: Vec<CaseResult> = serde_json::from_str(&case_json)?;
    Ok(EvalRun {
        eval_run_id: row.get(0)?,
        workspace_id: row.get(1)?,
        status: row.get(2)?,
        config_snapshot_json: row.get(3)?,
        started_at_unix: row.get(4)?,
        finished_at_unix: row.get(5)?,
        metrics,
        case_results,
        schema_version: row.get(8)?,
        dataset_ref: row.get(9)?,
        error_message: row.get(10)?,
    })
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

    fn fresh_store() -> SqliteEvalStore {
        let seq = SEQ.fetch_add(1, Ordering::SeqCst);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cf-eval-{}-{}-{}", std::process::id(), nanos, seq));
        SqliteEvalStore::open(&dir).expect("open ok")
    }

    fn make_create(id: &str) -> EvalRunCreate {
        EvalRunCreate {
            eval_run_id: id.into(),
            workspace_id: "ws-x".into(),
            config_snapshot_json: "{\"k\":1}".into(),
            dataset_ref: Some("/tmp/dataset".into()),
        }
    }

    #[test]
    fn test_create_and_get_roundtrip() {
        let s = fresh_store();
        let created = s.create(make_create("er-1")).unwrap();
        assert_eq!(created.status, "running");
        let got = s.get("er-1").unwrap().expect("present");
        assert_eq!(got.eval_run_id, "er-1");
        assert_eq!(got.status, "running");
        assert_eq!(got.config_snapshot_json, "{\"k\":1}");
        assert_eq!(got.dataset_ref.as_deref(), Some("/tmp/dataset"));
        assert!(got.finished_at_unix.is_none());
    }

    #[test]
    fn test_update_metrics_persists() {
        let s = fresh_store();
        s.create(make_create("er-m")).unwrap();
        let mut m = HashMap::new();
        m.insert("recall@5".to_string(), 0.7);
        m.insert("recall@10".to_string(), 0.85);
        s.update_metrics("er-m", m.clone()).unwrap();
        let got = s.get("er-m").unwrap().unwrap();
        assert_eq!(got.metrics.get("recall@5").copied(), Some(0.7));
        assert_eq!(got.metrics.get("recall@10").copied(), Some(0.85));
    }

    #[test]
    fn test_update_case_results_persists() {
        let s = fresh_store();
        s.create(make_create("er-c")).unwrap();
        let cases = vec![CaseResult {
            case_id: "c-1".into(),
            query: "hello".into(),
            expected_chunks: vec!["chk-1".into()],
            actual_chunks: vec!["chk-1".into(), "chk-2".into()],
            score: 0.95,
            passed: true,
        }];
        s.update_case_results("er-c", cases.clone()).unwrap();
        let got = s.get("er-c").unwrap().unwrap();
        assert_eq!(got.case_results.len(), 1);
        assert_eq!(got.case_results[0].case_id, "c-1");
        assert!(got.case_results[0].passed);
    }

    #[test]
    fn test_mark_finished_succeeded_sets_finished_at() {
        let s = fresh_store();
        s.create(make_create("er-f")).unwrap();
        s.mark_finished("er-f", "succeeded", 1700000000, None).unwrap();
        let got = s.get("er-f").unwrap().unwrap();
        assert_eq!(got.status, "succeeded");
        assert_eq!(got.finished_at_unix, Some(1700000000));
    }

    #[test]
    fn test_mark_finished_rejects_invalid_status() {
        let s = fresh_store();
        s.create(make_create("er-bad")).unwrap();
        let err = s
            .mark_finished("er-bad", "garbage", 1700000000, None)
            .expect_err("expect Invalid");
        assert!(matches!(err, EvalStoreError::Invalid(_)));
    }

    #[test]
    fn test_update_metrics_not_found() {
        let s = fresh_store();
        let err = s
            .update_metrics("missing", HashMap::new())
            .expect_err("expect NotFound");
        assert!(matches!(err, EvalStoreError::NotFound));
    }

    #[test]
    fn test_json_roundtrip_preserves_types() {
        let s = fresh_store();
        s.create(make_create("er-j")).unwrap();
        let mut m = HashMap::new();
        m.insert("int_like".to_string(), 100.0);
        m.insert("frac".to_string(), 0.3333333333333333);
        s.update_metrics("er-j", m.clone()).unwrap();
        let got = s.get("er-j").unwrap().unwrap();
        assert_eq!(got.metrics.get("int_like").copied(), Some(100.0));
        assert!((got.metrics.get("frac").copied().unwrap() - 0.3333333333333333).abs() < 1e-15);
    }

    // task-15.4 (Phase 15 P1 #4) — SqliteEvalStore.list tests.

    fn create_with_workspace(s: &SqliteEvalStore, id: &str, ws: &str) {
        s.create(EvalRunCreate {
            eval_run_id: id.to_string(),
            workspace_id: ws.to_string(),
            config_snapshot_json: "{}".to_string(),
            dataset_ref: None,
        })
        .unwrap();
    }

    #[test]
    fn test_list_returns_rows_ordered_by_started_at_desc() {
        let s = fresh_store();
        // 3 sequential creates → ascending started_at_unix; list returns DESC.
        create_with_workspace(&s, "er-1", "ws-a");
        std::thread::sleep(std::time::Duration::from_millis(1100));
        create_with_workspace(&s, "er-2", "ws-a");
        std::thread::sleep(std::time::Duration::from_millis(1100));
        create_with_workspace(&s, "er-3", "ws-a");
        let runs = s
            .list(ListEvalRunsFilter {
                workspace_id: None,
                status: None,
                limit: 10,
            })
            .unwrap();
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].eval_run_id, "er-3");
        assert_eq!(runs[1].eval_run_id, "er-2");
        assert_eq!(runs[2].eval_run_id, "er-1");
    }

    #[test]
    fn test_list_filter_workspace_id_narrows_results() {
        let s = fresh_store();
        create_with_workspace(&s, "er-a1", "ws-a");
        create_with_workspace(&s, "er-b1", "ws-b");
        create_with_workspace(&s, "er-a2", "ws-a");
        let runs = s
            .list(ListEvalRunsFilter {
                workspace_id: Some("ws-a".into()),
                status: None,
                limit: 10,
            })
            .unwrap();
        assert_eq!(runs.len(), 2);
        for r in &runs {
            assert_eq!(r.workspace_id, "ws-a");
        }
    }

    #[test]
    fn test_list_filter_status_narrows_results() {
        let s = fresh_store();
        create_with_workspace(&s, "er-x", "ws");
        create_with_workspace(&s, "er-y", "ws");
        // Mark er-x as succeeded; er-y stays running.
        s.mark_finished("er-x", "succeeded", 1_700_000_000, None)
            .unwrap();
        let succeeded = s
            .list(ListEvalRunsFilter {
                workspace_id: None,
                status: Some("succeeded".into()),
                limit: 10,
            })
            .unwrap();
        assert_eq!(succeeded.len(), 1);
        assert_eq!(succeeded[0].eval_run_id, "er-x");
        let running = s
            .list(ListEvalRunsFilter {
                workspace_id: None,
                status: Some("running".into()),
                limit: 10,
            })
            .unwrap();
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].eval_run_id, "er-y");
    }

    #[test]
    fn test_list_limit_clamped_to_200() {
        let s = fresh_store();
        for i in 0..3 {
            create_with_workspace(&s, &format!("er-{i}"), "ws");
        }
        // Even though we pass 500, server should clamp to 200 (here total only 3).
        let runs = s
            .list(ListEvalRunsFilter {
                workspace_id: None,
                status: None,
                limit: 500,
            })
            .unwrap();
        assert_eq!(runs.len(), 3);
        // Limit 0 / negative — clamp to 1 (returns most recent only).
        let one = s
            .list(ListEvalRunsFilter {
                workspace_id: None,
                status: None,
                limit: 0,
            })
            .unwrap();
        assert_eq!(one.len(), 1);
    }
}
