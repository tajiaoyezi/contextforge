//! task-14.1 (ADR-017 D1 Wave 4): `EvalServer` impl `EvalService` trait.
//!
//! 3 RPC: Create / Get / UpdateProgress → 真走 `SqliteEvalStore` (task-14.1).
//! UpdateProgress is the Go-side EvalRunner goroutine callback channel
//! (`internal/consoleapi/eval_runner.go::runEvalAsync` in task-14.2) — not
//! exposed in Console contract v1 22-endpoint surface; the Go REST handler
//! goroutine drives it after the recall harness finishes.
//!
//! Error mapping (ADR-016 §D3):
//!   - `EvalStoreError::NotFound` → `tonic::Status::not_found`
//!   - `EvalStoreError::Invalid` → `tonic::Status::invalid_argument`
//!   - `EvalStoreError::Sqlite|Json|Io` → `tonic::Status::internal`

use std::collections::HashMap;
use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::eval::{
    CaseResult as RustCaseResult, EvalRun as RustEvalRun, EvalRunCreate, EvalStoreError,
    ListEvalRunsFilter,
};
use crate::pb_console::eval_service_server::EvalService;
use crate::pb_console::{
    CaseResult as PbCaseResult, CreateEvalRunRequest, EvalRun as PbEvalRun, GetEvalRunRequest,
    ListEvalRunsRequest, ListEvalRunsResponse, UpdateEvalRunProgressRequest,
    UpdateEvalRunProgressResponse,
};

use super::DataPlaneStores;

pub struct EvalServer {
    stores: Arc<DataPlaneStores>,
}

impl EvalServer {
    pub fn new(stores: Arc<DataPlaneStores>) -> Self {
        Self { stores }
    }
}

fn case_to_pb(c: RustCaseResult) -> PbCaseResult {
    PbCaseResult {
        case_id: c.case_id,
        query: c.query,
        expected_chunks: c.expected_chunks,
        actual_chunks: c.actual_chunks,
        score: c.score,
        passed: c.passed,
    }
}

fn case_from_pb(c: PbCaseResult) -> RustCaseResult {
    RustCaseResult {
        case_id: c.case_id,
        query: c.query,
        expected_chunks: c.expected_chunks,
        actual_chunks: c.actual_chunks,
        score: c.score,
        passed: c.passed,
    }
}

fn run_to_pb(r: RustEvalRun) -> PbEvalRun {
    let metrics_json = serde_json::to_string(&r.metrics).unwrap_or_else(|_| "{}".into());
    PbEvalRun {
        eval_run_id: r.eval_run_id,
        workspace_id: r.workspace_id,
        status: r.status,
        config_snapshot_json: r.config_snapshot_json,
        started_at_unix: r.started_at_unix,
        finished_at_unix: r.finished_at_unix,
        metrics_json,
        case_results: r.case_results.into_iter().map(case_to_pb).collect(),
        schema_version: r.schema_version,
        dataset_ref: r.dataset_ref,
        error_message: r.error_message,
    }
}

fn eval_err_to_status(e: EvalStoreError) -> Status {
    match e {
        EvalStoreError::NotFound => Status::not_found("eval run not found"),
        EvalStoreError::Invalid(m) => Status::invalid_argument(m),
        EvalStoreError::Sqlite(m) => Status::internal(format!("sqlite: {m}")),
        EvalStoreError::Json(m) => Status::internal(format!("json: {m}")),
        EvalStoreError::Io(e) => Status::internal(format!("io: {e}")),
    }
}

#[tonic::async_trait]
impl EvalService for EvalServer {
    async fn create(
        &self,
        req: Request<CreateEvalRunRequest>,
    ) -> Result<Response<PbEvalRun>, Status> {
        let req = req.into_inner();
        let store = self
            .stores
            .eval
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("eval store not configured"))?;
        if req.eval_run_id.is_empty() {
            return Err(Status::invalid_argument("eval_run_id must not be empty"));
        }
        let create_req = EvalRunCreate {
            eval_run_id: req.eval_run_id,
            workspace_id: req.workspace_id,
            config_snapshot_json: req.config_snapshot_json,
            dataset_ref: if req.dataset_ref.is_empty() {
                None
            } else {
                Some(req.dataset_ref)
            },
        };
        let run = store.create(create_req).map_err(eval_err_to_status)?;
        Ok(Response::new(run_to_pb(run)))
    }

    async fn get(
        &self,
        req: Request<GetEvalRunRequest>,
    ) -> Result<Response<PbEvalRun>, Status> {
        let id = req.into_inner().eval_run_id;
        if id.is_empty() {
            return Err(Status::invalid_argument("eval_run_id must not be empty"));
        }
        let store = self
            .stores
            .eval
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("eval store not configured"))?;
        match store.get(&id).map_err(eval_err_to_status)? {
            Some(r) => Ok(Response::new(run_to_pb(r))),
            None => Err(Status::not_found(format!("eval run not found: {id}"))),
        }
    }

    async fn update_progress(
        &self,
        req: Request<UpdateEvalRunProgressRequest>,
    ) -> Result<Response<UpdateEvalRunProgressResponse>, Status> {
        let req = req.into_inner();
        let store = self
            .stores
            .eval
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("eval store not configured"))?;
        let metrics: HashMap<String, f64> = if req.metrics_json.is_empty() {
            HashMap::new()
        } else {
            serde_json::from_str(&req.metrics_json)
                .map_err(|e| Status::invalid_argument(format!("metrics_json: {e}")))?
        };
        store
            .update_metrics(&req.eval_run_id, metrics)
            .map_err(eval_err_to_status)?;
        let cases: Vec<RustCaseResult> = req.case_results.into_iter().map(case_from_pb).collect();
        store
            .update_case_results(&req.eval_run_id, cases)
            .map_err(eval_err_to_status)?;
        if matches!(req.status.as_str(), "succeeded" | "failed" | "cancelled") {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let err_msg = if req.error_message.is_empty() {
                None
            } else {
                Some(req.error_message)
            };
            store
                .mark_finished(&req.eval_run_id, &req.status, now, err_msg)
                .map_err(eval_err_to_status)?;
        }
        Ok(Response::new(UpdateEvalRunProgressResponse {}))
    }

    /// task-15.4 (Phase 15 P1 #4): list eval runs filtered by workspace_id /
    /// status with ORDER BY started_at DESC. `limit=0` falls back to default 50;
    /// values are clamped 1..=200 inside SqliteEvalStore::list.
    async fn list(
        &self,
        req: Request<ListEvalRunsRequest>,
    ) -> Result<Response<ListEvalRunsResponse>, Status> {
        let req = req.into_inner();
        let store = self
            .stores
            .eval
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("eval store not configured"))?;
        let workspace_id = if req.workspace_id.is_empty() {
            None
        } else {
            Some(req.workspace_id)
        };
        let status = if req.status.is_empty() {
            None
        } else {
            Some(req.status)
        };
        let limit = if req.limit <= 0 { 50 } else { req.limit as i64 };
        let runs = store
            .list(ListEvalRunsFilter {
                workspace_id,
                status,
                limit,
            })
            .map_err(eval_err_to_status)?;
        let pb_runs: Vec<PbEvalRun> = runs.into_iter().map(run_to_pb).collect();
        Ok(Response::new(ListEvalRunsResponse { runs: pb_runs }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::SqliteEvalStore;
    use crate::jobs::SqliteJobStore;
    use crate::workspace::SqliteWorkspaceStore;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(name: &str) -> PathBuf {
        let seq = SEQ.fetch_add(1, Ordering::SeqCst);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "cf-eval-server-{name}-{}-{nanos}-{seq}",
            std::process::id()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn fresh_server() -> (EvalServer, Arc<SqliteEvalStore>) {
        let dir = temp_dir("base");
        let ws = Arc::new(SqliteWorkspaceStore::open(&dir).unwrap());
        let js = Arc::new(SqliteJobStore::open(&dir).unwrap());
        let ev = Arc::new(SqliteEvalStore::open(&dir).unwrap());
        let stores = DataPlaneStores::with_eval(ws, js, ev.clone());
        (EvalServer::new(stores), ev)
    }

    #[tokio::test]
    async fn test_eval_server_create_returns_running() {
        let (server, _) = fresh_server();
        let resp = server
            .create(Request::new(CreateEvalRunRequest {
                eval_run_id: "er-create-1".into(),
                workspace_id: "ws-x".into(),
                config_snapshot_json: "{\"k\":1}".into(),
                dataset_ref: "/tmp/ds".into(),
            }))
            .await
            .expect("create ok");
        let run = resp.into_inner();
        assert_eq!(run.eval_run_id, "er-create-1");
        assert_eq!(run.status, "running");
        assert!(run.finished_at_unix.is_none());
    }

    #[tokio::test]
    async fn test_eval_server_get_404() {
        let (server, _) = fresh_server();
        let err = server
            .get(Request::new(GetEvalRunRequest {
                eval_run_id: "missing".into(),
            }))
            .await
            .expect_err("expect not_found");
        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    // task-15.4 (Phase 15 P1 #4): EvalServer.list RPC tests.
    #[tokio::test]
    async fn test_eval_server_list_returns_empty_when_no_rows() {
        let (server, _) = fresh_server();
        let resp = server
            .list(Request::new(ListEvalRunsRequest {
                workspace_id: "".into(),
                status: "".into(),
                limit: 10,
            }))
            .await
            .expect("list ok");
        assert!(resp.into_inner().runs.is_empty());
    }

    #[tokio::test]
    async fn test_eval_server_list_filters_by_workspace_id() {
        let (server, _) = fresh_server();
        for (id, ws) in &[("a1", "ws-a"), ("b1", "ws-b"), ("a2", "ws-a")] {
            server
                .create(Request::new(CreateEvalRunRequest {
                    eval_run_id: format!("er-{id}"),
                    workspace_id: (*ws).into(),
                    config_snapshot_json: "{}".into(),
                    dataset_ref: "".into(),
                }))
                .await
                .unwrap();
        }
        let resp = server
            .list(Request::new(ListEvalRunsRequest {
                workspace_id: "ws-a".into(),
                status: "".into(),
                limit: 0, // exercise default
            }))
            .await
            .expect("list ok");
        let runs = resp.into_inner().runs;
        assert_eq!(runs.len(), 2);
        for r in &runs {
            assert_eq!(r.workspace_id, "ws-a");
        }
    }

    #[tokio::test]
    async fn test_update_progress_persists_terminal_status() {
        let (server, ev_store) = fresh_server();
        server
            .create(Request::new(CreateEvalRunRequest {
                eval_run_id: "er-up".into(),
                workspace_id: "ws".into(),
                config_snapshot_json: "{}".into(),
                dataset_ref: "".into(),
            }))
            .await
            .unwrap();
        let metrics = serde_json::json!({"recall@5": 0.75}).to_string();
        server
            .update_progress(Request::new(UpdateEvalRunProgressRequest {
                eval_run_id: "er-up".into(),
                status: "succeeded".into(),
                metrics_json: metrics,
                case_results: vec![PbCaseResult {
                    case_id: "c-1".into(),
                    query: "hello".into(),
                    expected_chunks: vec!["chk-1".into()],
                    actual_chunks: vec!["chk-1".into()],
                    score: 1.0,
                    passed: true,
                }],
                error_message: "".into(),
            }))
            .await
            .unwrap();
        let got = ev_store.get("er-up").unwrap().unwrap();
        assert_eq!(got.status, "succeeded");
        assert!(got.finished_at_unix.is_some());
        assert_eq!(got.metrics.get("recall@5").copied(), Some(0.75));
        assert_eq!(got.case_results.len(), 1);
        assert!(got.case_results[0].passed);
    }
}
