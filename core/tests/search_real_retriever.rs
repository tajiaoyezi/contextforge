//! task-11.4 integration: SearchService.Query 真接 Retriever (Tantivy + SQLite) +
//! EventsService.Subscribe 真接 EventBus broadcast + JobRunner progress emit
//! `indexing.progress` events end-to-end.

use contextforge_core::data_plane::events::EventBus;
use contextforge_core::data_plane::DataPlaneStores;
use contextforge_core::jobs::{IndexSessionBackend, JobRunner, JobStore, SqliteJobStore};
use contextforge_core::pb_console::{
    events_service_client::EventsServiceClient, job_service_client::JobServiceClient,
    search_service_client::SearchServiceClient, EnqueueJobRequest, GetJobRequest,
    SearchRequest as PbSearchRequest, SubscribeEventsRequest,
};
use contextforge_core::workspace::{SqliteWorkspaceStore, WorkspaceCreate, WorkspaceStore};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_stream::StreamExt;

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = std::env::temp_dir().join(format!(
        "cf-s4-it-{name}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn fixture_dir() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest)
        .parent()
        .unwrap()
        .join("test")
        .join("fixtures")
        .join("index-job-real")
}

async fn spawn_full(
    label: &str,
) -> (
    std::net::SocketAddr,
    PathBuf,
    String,
    Arc<EventBus>,
    tokio::task::JoinHandle<()>,
) {
    let data_dir = temp_dir(label);
    let ws_store = Arc::new(SqliteWorkspaceStore::open(&data_dir).expect("ws"));
    let job_store = Arc::new(SqliteJobStore::open(&data_dir).expect("js"));

    let workspace_id = format!("ws-{label}");
    ws_store
        .create(&WorkspaceCreate {
            workspace_id: workspace_id.clone(),
            name: label.into(),
            root_path: fixture_dir().to_string_lossy().to_string(),
            allowlist: vec![],
            denylist: vec![],
        })
        .expect("create ws");

    let event_bus = EventBus::new();
    let indexer = IndexSessionBackend::with_event_bus(event_bus.clone());
    let job_store_dyn: Arc<dyn JobStore> = job_store.clone();
    let runner = Arc::new(JobRunner::new(job_store_dyn, indexer));
    let stores = DataPlaneStores::with_runner_and_bus(
        ws_store,
        job_store,
        runner,
        data_dir.clone(),
        event_bus.clone(),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    let incoming = TcpListenerStream::new(listener);
    let router = contextforge_core::data_plane::server_with_services(stores);
    let handle = tokio::spawn(async move {
        router.serve_with_incoming(incoming).await.expect("serve");
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    (addr, data_dir, workspace_id, event_bus, handle)
}

async fn run_index(addr: std::net::SocketAddr, workspace_id: &str) {
    let mut job = JobServiceClient::connect(format!("http://{addr}"))
        .await
        .expect("job connect");
    let enq = job
        .enqueue(EnqueueJobRequest {
            workspace_id: workspace_id.to_string(),
            trigger_source: "test".into(),
        })
        .await
        .expect("enqueue ok")
        .into_inner();
    // Wait for index to complete (≤15s).
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        let got = job
            .get(GetJobRequest {
                job_id: enq.job_id.clone(),
            })
            .await
            .expect("get ok")
            .into_inner();
        if got.status == "succeeded" || got.status == "failed" {
            assert_eq!(got.status, "succeeded", "index expected succeeded");
            return;
        }
        tokio::time::sleep(Duration::from_millis(80)).await;
    }
    panic!("index did not complete within 15s");
}

// =====================================================================
// AC1: POST /v1/search returns ≥1 SourceChunk with score > 0 + source_file
// matching the fixture repo path.
// =====================================================================
#[tokio::test]
async fn test_search_real_chunks() {
    let (addr, _data, workspace_id, _bus, _h) = spawn_full("search").await;
    run_index(addr, &workspace_id).await;

    let mut search = SearchServiceClient::connect(format!("http://{addr}"))
        .await
        .expect("search connect");
    let resp = search
        .query(PbSearchRequest {
            query: "contextforge".into(),
            workspace_id: workspace_id.clone(),
            agent_scope: "".into(),
            retrieval_method: "bm25".into(),
            top_k: 5,
            config_snapshot: "{}".into(),
            semantic: false,
        })
        .await
        .expect("query ok")
        .into_inner();

    assert!(
        !resp.results.is_empty(),
        "AC1: expected ≥1 SearchResultItem; got 0"
    );
    let first = &resp.results[0];
    assert!(first.score > 0.0, "AC1: expected score > 0; got {}", first.score);
    let fixture_path_str = fixture_dir().to_string_lossy().to_lowercase();
    let first_path = first.source_file_path.to_lowercase();
    assert!(
        first_path.contains("index-job-real"),
        "AC1: source_file_path should reference fixture dir; got {} (fixture={})",
        first.source_file_path,
        fixture_path_str
    );
    assert!(!first.chunk_id.is_empty(), "AC1: chunk_id non-empty");
    assert!(!first.chunk_text_preview.is_empty(), "AC1: chunk_text_preview non-empty");
}

// =====================================================================
// AC2: RetrievalTrace.retrieved_chunks has chunk_id + score + source_file +
// content_snippet ≤ 200 chars (UTF-8 safe).
// =====================================================================
#[tokio::test]
async fn test_retrieval_trace_fields() {
    let (addr, _data, workspace_id, _bus, _h) = spawn_full("trace").await;
    run_index(addr, &workspace_id).await;

    let mut search = SearchServiceClient::connect(format!("http://{addr}"))
        .await
        .expect("search connect");
    let resp = search
        .query(PbSearchRequest {
            query: "contextforge".into(),
            workspace_id: workspace_id.clone(),
            agent_scope: "".into(),
            retrieval_method: "bm25".into(),
            top_k: 5,
            config_snapshot: "{}".into(),
            semantic: false,
        })
        .await
        .expect("query ok")
        .into_inner();

    let trace = resp.trace.expect("trace present");
    assert!(
        !trace.retrieved_chunks.is_empty(),
        "AC2: expected retrieved_chunks; got empty"
    );
    let chunk = &trace.retrieved_chunks[0];
    assert!(!chunk.chunk_id.is_empty(), "AC2: chunk_id present");
    assert!(!chunk.source_file_path.is_empty(), "AC2: source_file_path present");
    assert!(
        chunk.chunk_text_preview.chars().count() <= 200,
        "AC2: content_snippet length should be ≤200 chars; got {}",
        chunk.chunk_text_preview.chars().count()
    );
    // UTF-8 boundary safe — chunk_text_preview is a valid String (compiler
    // guarantee); explicit check would only fail if our truncate broke utf8
    // mid-codepoint (would panic on slice).
}

// =====================================================================
// AC2 helper: utf8_safe_truncate explicitly tested for multi-byte chars.
// =====================================================================
#[test]
fn test_content_snippet_utf8_boundary() {
    use contextforge_core::data_plane::search::utf8_safe_truncate;
    // 3-byte CJK + 1-byte ASCII mixed.
    let s = "abc中文测试xyz";
    // Truncate at 5 chars: "abc中文".
    let out = utf8_safe_truncate(s, 5);
    assert_eq!(out, "abc中文");
    // Truncate at 100 chars (> total): full string.
    let out2 = utf8_safe_truncate(s, 100);
    assert_eq!(out2, s);
}

// =====================================================================
// AC4: JobRunner progress emit `indexing.progress` event observable via
// EventsService.Subscribe.
// =====================================================================
#[tokio::test]
async fn test_progress_event_emitted() {
    let (addr, _data, workspace_id, _bus, _h) = spawn_full("progress").await;

    // Subscribe to events FIRST so we don't miss progress emissions.
    let mut events_client = EventsServiceClient::connect(format!("http://{addr}"))
        .await
        .expect("events connect");
    let mut stream = events_client
        .subscribe(SubscribeEventsRequest {
            job_id: None,
            workspace_id: None,
            since_ts: 0,
            last_event_id: String::new(),
        })
        .await
        .expect("subscribe ok")
        .into_inner();

    // Now enqueue + run index in background.
    let mut job_client = JobServiceClient::connect(format!("http://{addr}"))
        .await
        .expect("job connect");
    let enq = job_client
        .enqueue(EnqueueJobRequest {
            workspace_id: workspace_id.clone(),
            trigger_source: "test".into(),
        })
        .await
        .expect("enqueue")
        .into_inner();

    // Collect events up to 10s OR until we see ≥1 indexing.progress.
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut saw_progress = false;
    while Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(500), stream.next()).await {
            Ok(Some(Ok(evt))) => {
                if evt.event_type == "indexing.progress" {
                    saw_progress = true;
                    assert_eq!(evt.job_id.as_deref(), Some(enq.job_id.as_str()));
                    break;
                }
            }
            Ok(Some(Err(_))) => continue,
            Ok(None) => break,
            Err(_) => continue, // 500ms timeout — retry
        }
    }
    assert!(
        saw_progress,
        "AC4: expected ≥1 indexing.progress event within 10s"
    );
}
