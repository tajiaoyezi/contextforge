//! task-11.1 §6 AC4: `JobServer` impl `JobService` trait.
//!
//! 4 RPC: Enqueue / Get / Cancel / Stream → 真走 `SqliteJobStore` (task-10.3).
//! `Enqueue` 写 status=queued via `SqliteJobStore.enqueue`；本 task 不真触发
//! `IndexSession::index_path_with_progress` (留 task-11.3 [SPEC-OWNER:task-11.3])。
//! `Stream` 本 task 仅占位返单条 keepalive 后 close (完整 multi-job streaming
//! 留 [SPEC-OWNER:task-11.4])。
//!
//! Error mapping (ADR-016 §D3):
//!   - `JobError::WorkspaceNotFound` → `tonic::Status::not_found`
//!   - `JobError::InvalidState("job not found: ...")` → `tonic::Status::not_found`
//!   - `JobError::InvalidState(其它)` → `tonic::Status::failed_precondition`
//!   - `JobError::Sqlite / Io / Indexer` → `tonic::Status::internal`

use std::sync::Arc;

use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use crate::jobs::{IndexJob as RustIndexJob, JobError, JobStore};
use crate::workspace::WorkspaceStore;
use crate::pb_console::job_service_server::JobService;
use crate::pb_console::{
    CancelJobRequest, CancelJobResponse, EnqueueJobRequest, GetJobRequest, IndexJob as PbIndexJob,
    ListJobsRequest, ListJobsResponse, StreamJobsRequest,
};

use super::DataPlaneStores;

pub struct JobServer {
    stores: Arc<DataPlaneStores>,
}

impl JobServer {
    pub fn new(stores: Arc<DataPlaneStores>) -> Self {
        Self { stores }
    }
}

fn job_to_pb(j: RustIndexJob) -> PbIndexJob {
    PbIndexJob {
        job_id: j.job_id,
        workspace_id: j.workspace_id,
        trigger_source: j.trigger_source,
        status: j.status,
        stage: j.stage,
        processed_files: j.processed_files,
        total_files: j.total_files,
        failed_files: j.failed_files,
        skipped_files: j.skipped_files,
        error_message: j.error_message,
        started_at_unix: j.started_at_unix,
        finished_at_unix: j.finished_at_unix,
        last_heartbeat_at_unix: j.last_heartbeat_at_unix,
    }
}

fn job_err_to_status(e: JobError) -> Status {
    match e {
        JobError::WorkspaceNotFound(id) => {
            Status::not_found(format!("workspace not found: {id}"))
        }
        JobError::InvalidState(msg) => {
            if msg.contains("job not found") {
                Status::not_found(msg)
            } else {
                Status::failed_precondition(msg)
            }
        }
        JobError::Sqlite(err) => Status::internal(format!("sqlite: {err}")),
        JobError::Io(err) => Status::internal(format!("io: {err}")),
        JobError::Indexer(msg) => Status::internal(format!("indexer: {msg}")),
    }
}

/// task-11.1 server stream associated type. Capacity 8 is sufficient for the
/// keepalive-only placeholder; task-11.4 may grow this when wiring true
/// multi-job streaming.
const JOB_STREAM_CAPACITY: usize = 8;

#[tonic::async_trait]
impl JobService for JobServer {
    async fn enqueue(
        &self,
        req: Request<EnqueueJobRequest>,
    ) -> Result<Response<PbIndexJob>, Status> {
        let req = req.into_inner();
        let trigger = if req.trigger_source.is_empty() {
            "console-api".to_string()
        } else {
            req.trigger_source
        };
        let workspace_id = req.workspace_id.clone();
        let job = self
            .stores
            .job_store
            .enqueue(&workspace_id, &trigger)
            .map_err(job_err_to_status)?;
        let job_id = job.job_id.clone();

        // task-11.3 §6 AC1: spawn the real JobRunner when configured. Without
        // a JobRunner (task-11.1 unit tests / in-memory dev mode) we keep the
        // job at status=queued and let callers move it manually.
        if let Some(runner) = &self.stores.job_runner {
            if let Some(workspace) = self
                .stores
                .workspace_store
                .get(&workspace_id)
                .map_err(|e| Status::internal(format!("workspace lookup: {e}")))?
            {
                let source = std::path::PathBuf::from(workspace.root_path);
                let data = self.stores.data_dir.clone();
                let runner = runner.clone();
                let job_id_owned = job_id.clone();
                tokio::spawn(async move {
                    // Honor the task-11.3 §6 AC1 contract: queued → running
                    // ≤1s. JobRunner.run_one marks running, then completes
                    // via JobOutcome → mark_terminal. Any error is recorded
                    // back into SqliteJobStore so the caller can poll Get().
                    if let Err(e) = runner.run_one(&job_id_owned, &source, &data).await {
                        eprintln!("WARN job {} run_one failed: {e}", job_id_owned);
                    }
                });
            }
        }

        Ok(Response::new(job_to_pb(job)))
    }

    async fn get(&self, req: Request<GetJobRequest>) -> Result<Response<PbIndexJob>, Status> {
        let id = req.into_inner().job_id;
        match self.stores.job_store.get(&id) {
            Ok(Some(job)) => Ok(Response::new(job_to_pb(job))),
            Ok(None) => Err(Status::not_found(format!("job not found: {id}"))),
            Err(e) => Err(job_err_to_status(e)),
        }
    }

    async fn cancel(
        &self,
        req: Request<CancelJobRequest>,
    ) -> Result<Response<CancelJobResponse>, Status> {
        let id = req.into_inner().job_id;
        match self.stores.job_store.request_cancel(&id) {
            Ok(true) => Ok(Response::new(CancelJobResponse { ok: true })),
            Ok(false) => Err(Status::failed_precondition(format!(
                "job already terminal: {id}"
            ))),
            Err(e) => Err(job_err_to_status(e)),
        }
    }

    async fn list(
        &self,
        req: Request<ListJobsRequest>,
    ) -> Result<Response<ListJobsResponse>, Status> {
        // task-12.1 (ADR-017 D1 Wave 1): v1.0 only supports active filter
        // (queued + running). Go REST layer returns 400 when ?status != "active"
        // [SPEC-DEFER:console-list-all-jobs]. workspace_id filter is post-filter
        // on the active set when set.
        let req = req.into_inner();
        let mut items = self
            .stores
            .job_store
            .list_active()
            .map_err(job_err_to_status)?;
        if let Some(ws) = req.workspace_id.as_deref() {
            if !ws.is_empty() {
                items.retain(|j| j.workspace_id == ws);
            }
        }
        if !req.status_filter.is_empty() {
            let allowed: std::collections::HashSet<&str> =
                req.status_filter.iter().map(String::as_str).collect();
            items.retain(|j| allowed.contains(j.status.as_str()));
        }
        Ok(Response::new(ListJobsResponse {
            items: items.into_iter().map(job_to_pb).collect(),
        }))
    }

    type StreamStream = ReceiverStream<Result<PbIndexJob, Status>>;

    async fn stream(
        &self,
        _req: Request<StreamJobsRequest>,
    ) -> Result<Response<Self::StreamStream>, Status> {
        // task-11.1 占位 [SPEC-OWNER:task-11.4]: emit current active jobs once,
        // then close. task-11.4 will replace with true server-stream that
        // subscribes to JobOutcome events.
        let (tx, rx) = tokio::sync::mpsc::channel(JOB_STREAM_CAPACITY);
        match self.stores.job_store.list_active() {
            Ok(active) => {
                for job in active {
                    let _ = tx.send(Ok(job_to_pb(job))).await;
                }
            }
            Err(e) => {
                let _ = tx.send(Err(job_err_to_status(e))).await;
            }
        }
        // drop(tx) → stream closes on receiver next() returning None
        drop(tx);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::SqliteJobStore;
    use crate::workspace::{SqliteWorkspaceStore, WorkspaceCreate, WorkspaceStore};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use std::sync::atomic::{AtomicU64, Ordering};
    static TEST_SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_data_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let seq = TEST_SEQ.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!(
            "cf-job-server-{name}-{}-{nanos}-{seq}",
            std::process::id()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn fresh_server() -> (PathBuf, JobServer, Arc<crate::workspace::SqliteWorkspaceStore>) {
        let seq = TEST_SEQ.load(Ordering::SeqCst);
        let dir = temp_data_dir(&format!("base-{seq}"));
        let ws = Arc::new(SqliteWorkspaceStore::open(&dir).unwrap());
        // seed a workspace so enqueue can pass the FK check
        ws.create(&WorkspaceCreate { owner_id: None,
            workspace_id: "ws-job-test".into(),
            name: "job test".into(),
            root_path: std::env::temp_dir().join("cf-job-fix").to_string_lossy().to_string(),
            allowlist: vec![],
            denylist: vec![],
        })
        .unwrap();
        let js = Arc::new(SqliteJobStore::open(&dir).unwrap());
        let server = JobServer::new(DataPlaneStores::new(ws.clone(), js));
        (dir, server, ws)
    }

    #[tokio::test]
    async fn test_job_server_enqueue_writes_queued() {
        let (_dir, server, _ws) = fresh_server();
        let resp = server
            .enqueue(Request::new(EnqueueJobRequest {
                workspace_id: "ws-job-test".into(),
                trigger_source: "test".into(),
            }))
            .await
            .expect("enqueue ok");
        let job = resp.into_inner();
        assert_eq!(job.workspace_id, "ws-job-test");
        assert_eq!(job.status, "queued");
        assert!(!job.job_id.is_empty());
    }

    #[tokio::test]
    async fn test_job_server_enqueue_unknown_workspace_returns_not_found() {
        let (_dir, server, _ws) = fresh_server();
        let err = server
            .enqueue(Request::new(EnqueueJobRequest {
                workspace_id: "ws-does-not-exist".into(),
                trigger_source: "test".into(),
            }))
            .await
            .expect_err("expect not_found");
        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_job_server_get_404() {
        let (_dir, server, _ws) = fresh_server();
        let err = server
            .get(Request::new(GetJobRequest {
                job_id: "job-does-not-exist".into(),
            }))
            .await
            .expect_err("expect not_found");
        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_job_server_list_returns_queued_jobs() {
        let (_dir, server, _ws) = fresh_server();
        // enqueue two jobs (both queued)
        let job_a = server
            .enqueue(Request::new(EnqueueJobRequest {
                workspace_id: "ws-job-test".into(),
                trigger_source: "test".into(),
            }))
            .await
            .unwrap()
            .into_inner();
        let _job_b = server
            .enqueue(Request::new(EnqueueJobRequest {
                workspace_id: "ws-job-test".into(),
                trigger_source: "test".into(),
            }))
            .await
            .unwrap()
            .into_inner();
        let resp = server
            .list(Request::new(ListJobsRequest {
                status_filter: vec!["queued".into(), "running".into()],
                workspace_id: None,
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(resp.items.len(), 2);
        assert!(resp.items.iter().any(|j| j.job_id == job_a.job_id));
        for j in resp.items.iter() {
            assert!(j.status == "queued" || j.status == "running");
        }
    }

    #[tokio::test]
    async fn test_job_server_list_excludes_terminal_jobs() {
        let (_dir, server, _ws) = fresh_server();
        let job = server
            .enqueue(Request::new(EnqueueJobRequest {
                workspace_id: "ws-job-test".into(),
                trigger_source: "test".into(),
            }))
            .await
            .unwrap()
            .into_inner();
        // mark terminal by request_cancel + then mark_terminal directly via store
        server
            .stores
            .job_store
            .request_cancel(&job.job_id)
            .unwrap();
        // Simulate runner mark_terminal so list_active no longer includes it
        let _ = server.stores.job_store.mark_terminal(
            &job.job_id,
            "cancelled",
            Some("cancelled by test"),
        );
        let resp = server
            .list(Request::new(ListJobsRequest {
                status_filter: vec!["queued".into(), "running".into()],
                workspace_id: None,
            }))
            .await
            .unwrap()
            .into_inner();
        for j in resp.items.iter() {
            assert_ne!(j.job_id, job.job_id, "terminal job must be excluded");
        }
    }

    #[tokio::test]
    async fn test_job_server_cancel_sets_flag() {
        let (_dir, server, _ws) = fresh_server();
        let enqueue_resp = server
            .enqueue(Request::new(EnqueueJobRequest {
                workspace_id: "ws-job-test".into(),
                trigger_source: "test".into(),
            }))
            .await
            .unwrap();
        let job_id = enqueue_resp.into_inner().job_id;
        let cancel_resp = server
            .cancel(Request::new(CancelJobRequest {
                job_id: job_id.clone(),
            }))
            .await
            .expect("cancel ok");
        assert!(cancel_resp.into_inner().ok);

        // verify is_cancel_requested is true now (via store; tests SqliteJobStore wiring)
        assert!(
            server
                .stores
                .job_store
                .is_cancel_requested(&job_id)
                .unwrap(),
            "cancel_requested flag should be true after Cancel RPC"
        );
    }
}
