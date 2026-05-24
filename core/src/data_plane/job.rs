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
use crate::pb_console::job_service_server::JobService;
use crate::pb_console::{
    CancelJobRequest, CancelJobResponse, EnqueueJobRequest, GetJobRequest, IndexJob as PbIndexJob,
    StreamJobsRequest,
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
        let job = self
            .stores
            .job_store
            .enqueue(&req.workspace_id, &trigger)
            .map_err(job_err_to_status)?;
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
        ws.create(&WorkspaceCreate {
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
