//! task-13.1 (ADR-017 D1 Wave 3) — MemoryService end-to-end via tonic
//! client → MemoryServer → SqliteMemoryStore.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tonic::transport::Server;
use tonic::Request;

use contextforge_core::data_plane::{memory::MemoryServer, DataPlaneStores};
use contextforge_core::jobs::SqliteJobStore;
use contextforge_core::memory::{MemoryItem, SqliteMemoryStore};
use contextforge_core::memoryops::audit::AuditSink;
use contextforge_core::pb_console::memory_service_client::MemoryServiceClient;
use contextforge_core::pb_console::memory_service_server::MemoryServiceServer;
use contextforge_core::pb_console::{
    DeprecateMemoryRequest, GetMemoryRequest, ListMemoryRequest, PinMemoryRequest,
    SoftDeleteMemoryRequest,
};
use contextforge_core::workspace::SqliteWorkspaceStore;

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = std::env::temp_dir().join(format!(
        "cf-memory-int-{name}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn mem(id: &str, scope: &str, status: &str) -> MemoryItem {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    MemoryItem {
        memory_id: id.into(),
        agent_scope: scope.into(),
        content_preview: format!("preview for {id}"),
        source_type: "test".into(),
        source_ref: format!("file:{id}.md"),
        created_at_unix: now,
        updated_at_unix: now,
        hit_count: 0,
        status: status.into(),
        is_pinned: false,
        pinned_by: String::new(),
        pinned_at_unix: 0,
    }
}

async fn spawn_server() -> (String, Arc<SqliteMemoryStore>, tokio::task::JoinHandle<()>) {
    let dir = temp_dir("e2e");
    let ws = Arc::new(SqliteWorkspaceStore::open(&dir).unwrap());
    let js = Arc::new(SqliteJobStore::open(&dir).unwrap());
    let mem_store = Arc::new(SqliteMemoryStore::open(&dir).unwrap());
    let audit = Arc::new(Mutex::new(AuditSink::open(&dir, "memory").unwrap()));
    let stores = DataPlaneStores::with_memory(ws, js, mem_store.clone(), audit);
    let server = MemoryServer::new(stores);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{}", addr);

    let handle = tokio::spawn(async move {
        let incoming = tonic::transport::server::TcpIncoming::from_listener(listener, true, None)
            .expect("incoming");
        let _ = Server::builder()
            .add_service(MemoryServiceServer::new(server))
            .serve_with_incoming(incoming)
            .await;
    });
    // Give the server a moment to start.
    tokio::time::sleep(Duration::from_millis(50)).await;
    (url, mem_store, handle)
}

#[tokio::test]
async fn test_memory_crud_via_grpc() {
    let (url, mem_store, _h) = spawn_server().await;
    mem_store
        .seed_for_tests(vec![mem("m1", "agent-x", "active"), mem("m2", "agent-x", "active")])
        .unwrap();
    let mut client = MemoryServiceClient::connect(url).await.expect("connect");

    // List
    let resp = client
        .list(Request::new(ListMemoryRequest {
            agent_id: "".into(),
            scope: "".into(),
            namespace: "".into(),
            include_soft_deleted: false,
        }))
        .await
        .unwrap();
    let items = resp.into_inner().items;
    assert_eq!(items.len(), 2);

    // Get hit
    let resp = client
        .get(Request::new(GetMemoryRequest {
            memory_id: "m1".into(),
        }))
        .await
        .unwrap();
    assert_eq!(resp.into_inner().memory_id, "m1");

    // Get miss → 404
    let err = client
        .get(Request::new(GetMemoryRequest {
            memory_id: "ghost".into(),
        }))
        .await
        .expect_err("expect not_found");
    assert_eq!(err.code(), tonic::Code::NotFound);

    // Pin
    client
        .pin(Request::new(PinMemoryRequest {
            memory_id: "m1".into(),
            pin: true,
            actor: String::new(),
        }))
        .await
        .unwrap();
    assert!(mem_store.get("m1").unwrap().unwrap().is_pinned);

    // Deprecate
    client
        .deprecate(Request::new(DeprecateMemoryRequest {
            memory_id: "m2".into(),
        }))
        .await
        .unwrap();
    assert_eq!(mem_store.get("m2").unwrap().unwrap().status, "deprecated");

    // Soft delete + verify list excludes by default
    client
        .soft_delete(Request::new(SoftDeleteMemoryRequest {
            memory_id: "m1".into(),
        }))
        .await
        .unwrap();
    let resp = client
        .list(Request::new(ListMemoryRequest {
            agent_id: "".into(),
            scope: "".into(),
            namespace: "".into(),
            include_soft_deleted: false,
        }))
        .await
        .unwrap();
    // m1 is now soft_deleted (excluded), m2 is deprecated (included)
    let items = resp.into_inner().items;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].memory_id, "m2");
}

#[tokio::test]
async fn test_list_filter_by_scope() {
    let (url, mem_store, _h) = spawn_server().await;
    mem_store
        .seed_for_tests(vec![
            mem("a", "scope-x", "active"),
            mem("b", "scope-y", "active"),
            mem("c", "scope-x", "active"),
        ])
        .unwrap();
    let mut client = MemoryServiceClient::connect(url).await.unwrap();
    let resp = client
        .list(Request::new(ListMemoryRequest {
            agent_id: "".into(),
            scope: "scope-x".into(),
            namespace: "".into(),
            include_soft_deleted: false,
        }))
        .await
        .unwrap();
    assert_eq!(resp.into_inner().items.len(), 2);
}

#[tokio::test]
async fn test_soft_delete_excluded_from_default_list() {
    let (url, mem_store, _h) = spawn_server().await;
    mem_store.seed_for_tests(vec![mem("z", "s", "active")]).unwrap();
    let mut client = MemoryServiceClient::connect(url).await.unwrap();
    client
        .soft_delete(Request::new(SoftDeleteMemoryRequest {
            memory_id: "z".into(),
        }))
        .await
        .unwrap();
    let resp = client
        .list(Request::new(ListMemoryRequest {
            agent_id: "".into(),
            scope: "".into(),
            namespace: "".into(),
            include_soft_deleted: false,
        }))
        .await
        .unwrap();
    assert!(resp.into_inner().items.is_empty());
    // include_soft_deleted=true → 1
    let resp2 = client
        .list(Request::new(ListMemoryRequest {
            agent_id: "".into(),
            scope: "".into(),
            namespace: "".into(),
            include_soft_deleted: true,
        }))
        .await
        .unwrap();
    assert_eq!(resp2.into_inner().items.len(), 1);
}

/// task-17.1 / ADR-022 D1 — verify gRPC wire propagates is_pinned end-to-end:
/// SqliteMemoryStore.set_pinned writes the column, MemoryServer.memory_to_pb
/// copies it onto PbMemoryItem, and the gRPC client surfaces it on Get / List.
#[tokio::test]
async fn test_is_pinned_propagates_via_grpc_list_and_get() {
    let (url, mem_store, _h) = spawn_server().await;
    mem_store
        .seed_for_tests(vec![
            mem("pinned", "agent-a", "active"),
            mem("unpinned", "agent-a", "active"),
        ])
        .unwrap();
    mem_store.set_pinned("pinned", true).unwrap();
    let mut client = MemoryServiceClient::connect(url).await.unwrap();

    let list_resp = client
        .list(Request::new(ListMemoryRequest {
            agent_id: "".into(),
            scope: "".into(),
            namespace: "".into(),
            include_soft_deleted: false,
        }))
        .await
        .unwrap();
    let items = list_resp.into_inner().items;
    let pinned = items.iter().find(|i| i.memory_id == "pinned").unwrap();
    let unpinned = items.iter().find(|i| i.memory_id == "unpinned").unwrap();
    assert!(pinned.is_pinned, "List wire response: pinned.is_pinned should be true");
    assert!(!unpinned.is_pinned, "List wire response: unpinned.is_pinned should be false");

    let get_resp = client
        .get(Request::new(GetMemoryRequest {
            memory_id: "pinned".into(),
        }))
        .await
        .unwrap();
    assert!(
        get_resp.into_inner().is_pinned,
        "Get wire response: pinned.is_pinned should be true"
    );
}

/// task-17.1 / ADR-022 D2 — Pin RPC pin=false unpins after an earlier pin=true.
/// Validates the Go-side handleMemoryPin body parsing path equivalence at the
/// proto layer: PinMemoryRequest{pin: false} reverses store state.
#[tokio::test]
async fn test_pin_rpc_unpin_reverses_state() {
    let (url, mem_store, _h) = spawn_server().await;
    mem_store.seed_for_tests(vec![mem("u", "scope", "active")]).unwrap();
    let mut client = MemoryServiceClient::connect(url).await.unwrap();
    client
        .pin(Request::new(PinMemoryRequest {
            memory_id: "u".into(),
            pin: true,
            actor: String::new(),
        }))
        .await
        .unwrap();
    assert!(mem_store.get("u").unwrap().unwrap().is_pinned);
    client
        .pin(Request::new(PinMemoryRequest {
            memory_id: "u".into(),
            pin: false,
            actor: String::new(),
        }))
        .await
        .unwrap();
    assert!(!mem_store.get("u").unwrap().unwrap().is_pinned);
}
