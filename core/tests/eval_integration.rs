//! task-14.1 (ADR-017 D1 Wave 4) — EvalService end-to-end via tonic client.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tonic::transport::Server;
use tonic::Request;

use contextforge_core::data_plane::{eval::EvalServer, DataPlaneStores};
use contextforge_core::eval::SqliteEvalStore;
use contextforge_core::jobs::SqliteJobStore;
use contextforge_core::pb_console::eval_service_client::EvalServiceClient;
use contextforge_core::pb_console::eval_service_server::EvalServiceServer;
use contextforge_core::pb_console::{
    CaseResult, CreateEvalRunRequest, GetEvalRunRequest, UpdateEvalRunProgressRequest,
};
use contextforge_core::workspace::SqliteWorkspaceStore;

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = std::env::temp_dir().join(format!(
        "cf-eval-int-{name}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

async fn spawn_server() -> (String, Arc<SqliteEvalStore>, tokio::task::JoinHandle<()>) {
    let dir = temp_dir("e2e");
    let ws = Arc::new(SqliteWorkspaceStore::open(&dir).unwrap());
    let js = Arc::new(SqliteJobStore::open(&dir).unwrap());
    let ev = Arc::new(SqliteEvalStore::open(&dir).unwrap());
    let stores = DataPlaneStores::with_eval(ws, js, ev.clone());
    let server = EvalServer::new(stores);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{}", addr);

    let handle = tokio::spawn(async move {
        let incoming = tonic::transport::server::TcpIncoming::from_listener(listener, true, None)
            .expect("incoming");
        let _ = Server::builder()
            .add_service(EvalServiceServer::new(server))
            .serve_with_incoming(incoming)
            .await;
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    (url, ev, handle)
}

#[tokio::test]
async fn test_eval_crud_via_grpc() {
    let (url, _store, _h) = spawn_server().await;
    let mut client = EvalServiceClient::connect(url).await.expect("connect");

    // Create
    let resp = client
        .create(Request::new(CreateEvalRunRequest {
            eval_run_id: "er-int-1".into(),
            workspace_id: "ws-x".into(),
            config_snapshot_json: "{\"k\":1}".into(),
            dataset_ref: "/tmp/ds".into(),
        }))
        .await
        .unwrap();
    let run = resp.into_inner();
    assert_eq!(run.eval_run_id, "er-int-1");
    assert_eq!(run.status, "running");
    assert!(run.finished_at_unix.is_none());

    // Get
    let resp = client
        .get(Request::new(GetEvalRunRequest {
            eval_run_id: "er-int-1".into(),
        }))
        .await
        .unwrap();
    let got = resp.into_inner();
    assert_eq!(got.eval_run_id, "er-int-1");

    // Get missing → 404
    let err = client
        .get(Request::new(GetEvalRunRequest {
            eval_run_id: "ghost".into(),
        }))
        .await
        .expect_err("expect not_found");
    assert_eq!(err.code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn test_eval_run_terminal_lifecycle() {
    let (url, _store, _h) = spawn_server().await;
    let mut client = EvalServiceClient::connect(url).await.expect("connect");
    client
        .create(Request::new(CreateEvalRunRequest {
            eval_run_id: "er-life".into(),
            workspace_id: "ws".into(),
            config_snapshot_json: "{}".into(),
            dataset_ref: "".into(),
        }))
        .await
        .unwrap();
    let metrics = serde_json::json!({"recall@5": 0.7, "recall@10": 0.85}).to_string();
    client
        .update_progress(Request::new(UpdateEvalRunProgressRequest {
            eval_run_id: "er-life".into(),
            status: "succeeded".into(),
            metrics_json: metrics,
            case_results: vec![CaseResult {
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
    let resp = client
        .get(Request::new(GetEvalRunRequest {
            eval_run_id: "er-life".into(),
        }))
        .await
        .unwrap();
    let run = resp.into_inner();
    assert_eq!(run.status, "succeeded");
    assert!(run.finished_at_unix.is_some());
    assert!(run.metrics_json.contains("recall@5"));
    assert_eq!(run.case_results.len(), 1);
    assert!(run.case_results[0].passed);
}
