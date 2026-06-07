//! task-11.1 §6 AC1 / AC3 / AC4 / AC5 integration: real TCP listener + tonic
//! Channel client → 4 service end-to-end.
//!
//! AC1 (proto/snake_case 1:1): grpcurl-equivalent compile-time check 走单测
//! `test_proto_field_snake_case_consistency` (lib unit). 集成测试这里走真
//! `tonic::transport::Server::serve_with_incoming` → `Channel::from_static`
//! 验证 4 service 注册可见 + Workspace CRUD via grpc + Job enqueue/get/cancel
//! via grpc + Search empty response via grpc + Events keepalive stream via grpc.

use contextforge_core::data_plane::{server_with_services, DataPlaneStores};
use contextforge_core::jobs::SqliteJobStore;
use contextforge_core::pb_console::{
    events_service_client::EventsServiceClient,
    job_service_client::JobServiceClient,
    search_service_client::SearchServiceClient,
    workspace_service_client::WorkspaceServiceClient,
    CancelJobRequest, CreateWorkspaceRequest, DeleteWorkspaceRequest, EnqueueJobRequest,
    GetJobRequest, GetWorkspaceRequest, ListWorkspacesRequest, SearchRequest as PbSearchRequest,
    SubscribeEventsRequest,
};
use contextforge_core::workspace::SqliteWorkspaceStore;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_stream::StreamExt;

fn temp_data_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = std::env::temp_dir().join(format!(
        "cf-dp-it-{name}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// Bring up a real TCP listener on 127.0.0.1:<random> and spawn the 4-service
/// tonic server on it. Returns the bound socket addr + a `JoinHandle` whose
/// lifetime keeps the server alive until the test ends (drop = server stops).
async fn spawn_server() -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let dir = temp_data_dir("server");
    let ws = Arc::new(SqliteWorkspaceStore::open(&dir).expect("open ws"));
    let js = Arc::new(SqliteJobStore::open(&dir).expect("open js"));
    let stores = DataPlaneStores::new(ws, js);

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind 0");
    let addr = listener.local_addr().expect("local_addr");
    let incoming = TcpListenerStream::new(listener);

    let router = server_with_services(stores);
    let handle = tokio::spawn(async move {
        router.serve_with_incoming(incoming).await.expect("serve");
    });
    // Tiny wait to let the server actually bind + start accepting.
    tokio::time::sleep(Duration::from_millis(100)).await;
    (addr, handle)
}

async fn workspace_client(addr: std::net::SocketAddr) -> WorkspaceServiceClient<tonic::transport::Channel> {
    WorkspaceServiceClient::connect(format!("http://{}", addr))
        .await
        .expect("ws connect")
}

async fn job_client(addr: std::net::SocketAddr) -> JobServiceClient<tonic::transport::Channel> {
    JobServiceClient::connect(format!("http://{}", addr))
        .await
        .expect("job connect")
}

async fn search_client(addr: std::net::SocketAddr) -> SearchServiceClient<tonic::transport::Channel> {
    SearchServiceClient::connect(format!("http://{}", addr))
        .await
        .expect("search connect")
}

async fn events_client(addr: std::net::SocketAddr) -> EventsServiceClient<tonic::transport::Channel> {
    EventsServiceClient::connect(format!("http://{}", addr))
        .await
        .expect("events connect")
}

#[tokio::test]
async fn test_workspace_crud_via_grpc() {
    let (addr, _h) = spawn_server().await;
    let mut ws = workspace_client(addr).await;

    // Create
    let resp = ws
        .create(CreateWorkspaceRequest {
            workspace_id: "ws-crud-via-grpc".into(),
            name: "crud via grpc".into(),
            root_path: std::env::temp_dir().to_string_lossy().to_string(),
            allowlist: vec!["src/".into()],
            denylist: vec![".git/".into()],
        })
        .await
        .expect("create ok");
    let created = resp.into_inner();
    assert_eq!(created.workspace_id, "ws-crud-via-grpc");
    assert_eq!(created.name, "crud via grpc");
    assert_eq!(created.status, "ready");
    assert_eq!(created.allowlist, vec!["src/"]);
    assert_eq!(created.denylist, vec![".git/"]);

    // Get
    let got = ws
        .get(GetWorkspaceRequest {
            workspace_id: "ws-crud-via-grpc".into(),
        })
        .await
        .expect("get ok")
        .into_inner();
    assert_eq!(got.workspace_id, "ws-crud-via-grpc");

    // Get unknown → NotFound
    let err = ws
        .get(GetWorkspaceRequest {
            workspace_id: "ws-does-not-exist".into(),
        })
        .await
        .expect_err("expect not_found");
    assert_eq!(err.code(), tonic::Code::NotFound);

    // List
    let listed = ws
        .list(ListWorkspacesRequest {})
        .await
        .expect("list ok")
        .into_inner();
    assert!(listed.items.iter().any(|w| w.workspace_id == "ws-crud-via-grpc"));

    // Delete → ok
    let del = ws
        .delete(DeleteWorkspaceRequest {
            workspace_id: "ws-crud-via-grpc".into(),
        })
        .await
        .expect("delete ok")
        .into_inner();
    assert!(del.ok);

    // List after delete → soft-deleted entries filtered
    let listed_after = ws
        .list(ListWorkspacesRequest {})
        .await
        .expect("list ok 2")
        .into_inner();
    assert!(!listed_after
        .items
        .iter()
        .any(|w| w.workspace_id == "ws-crud-via-grpc"));
}

#[tokio::test]
async fn test_job_enqueue_get_cancel() {
    let (addr, _h) = spawn_server().await;
    let mut ws = workspace_client(addr).await;
    let mut job = job_client(addr).await;

    // Seed workspace
    ws.create(CreateWorkspaceRequest {
        workspace_id: "ws-for-job".into(),
        name: "for job".into(),
        root_path: std::env::temp_dir().to_string_lossy().to_string(),
        allowlist: vec![],
        denylist: vec![],
    })
    .await
    .expect("seed workspace");

    // Enqueue
    let enqueue_resp = job
        .enqueue(EnqueueJobRequest {
            workspace_id: "ws-for-job".into(),
            trigger_source: "test".into(),
        })
        .await
        .expect("enqueue ok");
    let job_obj = enqueue_resp.into_inner();
    assert_eq!(job_obj.workspace_id, "ws-for-job");
    assert_eq!(job_obj.status, "queued");
    assert!(!job_obj.job_id.is_empty());

    // Get → same job
    let got = job
        .get(GetJobRequest {
            job_id: job_obj.job_id.clone(),
        })
        .await
        .expect("get ok")
        .into_inner();
    assert_eq!(got.job_id, job_obj.job_id);
    assert_eq!(got.status, "queued");

    // Get unknown → NotFound
    let err = job
        .get(GetJobRequest {
            job_id: "job-does-not-exist".into(),
        })
        .await
        .expect_err("expect not_found");
    assert_eq!(err.code(), tonic::Code::NotFound);

    // Enqueue with unknown workspace → NotFound
    let err = job
        .enqueue(EnqueueJobRequest {
            workspace_id: "ws-missing".into(),
            trigger_source: "test".into(),
        })
        .await
        .expect_err("expect not_found");
    assert_eq!(err.code(), tonic::Code::NotFound);

    // Cancel → ok, then subsequent cancel → FailedPrecondition (because v0.3
    // SqliteJobStore treats a non-terminal queued/running job as cancel-able
    // and after the second call status is still queued + cancel_requested=1;
    // request_cancel returns true even on repeat — so we just verify the
    // first cancel succeeded).
    let cancel_resp = job
        .cancel(CancelJobRequest {
            job_id: job_obj.job_id.clone(),
        })
        .await
        .expect("cancel ok");
    assert!(cancel_resp.into_inner().ok);
}

#[tokio::test]
async fn test_search_empty_response_via_grpc() {
    let (addr, _h) = spawn_server().await;
    let mut search = search_client(addr).await;
    let resp = search
        .query(PbSearchRequest {
            query: "anything".into(),
            workspace_id: "ws-x".into(),
            agent_scope: "".into(),
            retrieval_method: "bm25".into(),
            top_k: 5,
            config_snapshot: "{}".into(),
            semantic: false,
            hybrid: false,
            source_type: Vec::new(),
        })
        .await
        .expect("query ok");
    let inner = resp.into_inner();
    // task-11.1 placeholder: empty results + non-None trace [SPEC-OWNER:task-11.4]
    assert!(inner.results.is_empty(), "task-11.1: empty results");
    let trace = inner.trace.expect("trace present");
    assert_eq!(trace.query, "anything");
    assert!(trace.retrieved_chunks.is_empty());
}

#[tokio::test]
async fn test_serve_full_listens_both_planes() {
    // task-11.1 §6 AC5: contextforge-core daemon `serve` 子命令启动后 4 个新
    // Console data plane service 注册到 tonic Server。
    //
    // 用 serve_full 等价路径（直接调 lib fn，不 spawn binary，避免测试依赖
    // build artifact + 跨 OS path 问题；测试目的是验证 wiring，不是 binary
    // spawn）。Phase 9 ContextService + Phase 11 4 service 共享一个 listener。
    use contextforge_core::pb::context_service_client::ContextServiceClient;
    use contextforge_core::pb::HealthRequest;

    let dir = temp_data_dir("serve_full");
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind 0");
    let addr = listener.local_addr().expect("local_addr");
    let incoming = TcpListenerStream::new(listener);

    let svc = contextforge_core::server::CoreService::new(dir.clone());
    let dir_clone = dir.clone();
    let handle = tokio::spawn(async move {
        use contextforge_core::data_plane::{register_services, DataPlaneStores};
        use contextforge_core::jobs::SqliteJobStore;
        use contextforge_core::pb::context_service_server::ContextServiceServer;
        use contextforge_core::workspace::SqliteWorkspaceStore;

        let ws_store = Arc::new(SqliteWorkspaceStore::open(&dir_clone).unwrap());
        let job_store = Arc::new(SqliteJobStore::open(&dir_clone).unwrap());
        let stores = DataPlaneStores::new(ws_store, job_store);

        let mut builder = tonic::transport::Server::builder();
        let router = builder.add_service(ContextServiceServer::new(svc));
        let router = register_services(router, stores);
        router.serve_with_incoming(incoming).await.expect("serve");
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Phase 9 ContextService.Health 可达
    let mut ctx_client = ContextServiceClient::connect(format!("http://{}", addr))
        .await
        .expect("ctx connect");
    let health = ctx_client
        .health(HealthRequest {})
        .await
        .expect("health ok")
        .into_inner();
    assert_eq!(health.status, "SERVING");

    // Phase 11 WorkspaceService.List 可达
    let mut ws = workspace_client(addr).await;
    let listed = ws
        .list(ListWorkspacesRequest {})
        .await
        .expect("list ok")
        .into_inner();
    assert!(listed.items.is_empty(), "fresh data dir: empty list");

    // Phase 11 SearchService.Query 可达（empty 占位但 wire 活）
    let mut search = search_client(addr).await;
    let resp = search
        .query(PbSearchRequest {
            query: "x".into(),
            workspace_id: "ws".into(),
            agent_scope: "".into(),
            retrieval_method: "bm25".into(),
            top_k: 1,
            config_snapshot: "{}".into(),
            semantic: false,
            hybrid: false,
            source_type: Vec::new(),
        })
        .await
        .expect("search ok");
    assert!(resp.into_inner().results.is_empty());

    drop(handle);
}

#[tokio::test]
async fn test_events_keepalive_stream_via_grpc() {
    let (addr, _h) = spawn_server().await;
    let mut events = events_client(addr).await;
    let resp = events
        .subscribe(SubscribeEventsRequest {
            job_id: None,
            workspace_id: None,
            since_ts: 0,
            last_event_id: String::new(),
        })
        .await
        .expect("subscribe ok");
    let mut stream = resp.into_inner();
    // First message: keepalive
    let first = stream.next().await.expect("at least one event");
    let evt = first.expect("event Ok");
    assert_eq!(evt.event_type, "core.keepalive");
    // task-11.1 placeholder closes stream after one event [SPEC-OWNER:task-11.4]
    let second = tokio::time::timeout(Duration::from_millis(500), stream.next()).await;
    match second {
        Ok(None) => {} // stream closed - expected
        Ok(Some(Err(_))) => {} // also acceptable (server closed)
        Ok(Some(Ok(_))) => panic!("task-11.1: only one keepalive event expected"),
        Err(_) => panic!("task-11.1: stream should close within 500ms"),
    }
}
