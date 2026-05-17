//! task-1.3 integration tests — TEST-1.3.1 ~ TEST-1.3.4 (SCEN-1.3.*).
//!
//! Drives the public `contextforge_core::server` surface (§5.3) + the AC4
//! module placeholders. RED skeleton: every call hits `unimplemented!()`
//! and panics -> tests fail functionally (not by compile error, §2.5.1).

use std::net::SocketAddr;
use std::time::Duration;

use contextforge_core::pb::context_service_client::ContextServiceClient;
use contextforge_core::pb::{HealthRequest, SearchRequest};
use contextforge_core::server::{resolve_listen_addr, serve, ListenAddr};

/// Pick a free loopback port (listener dropped immediately so `serve` can bind).
fn ephemeral_addr() -> SocketAddr {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}

async fn spawn_server(addr: SocketAddr) {
    tokio::spawn(async move {
        let _ = serve(ListenAddr::Tcp(addr)).await;
    });
    tokio::time::sleep(Duration::from_millis(400)).await; // let it bind
}

// SCEN-1.3.1 / AC1: resolve_listen_addr — safe default, reject 0.0.0.0.
#[tokio::test(flavor = "multi_thread")]
async fn test_1_3_1_listen_addr_rejects_wildcard() {
    // TEST-1.3.1: 默认非 0.0.0.0；显式 0.0.0.0 绑定被拒；loopback 合法
    let def = resolve_listen_addr(None).expect("default listen addr must resolve");
    match def {
        ListenAddr::Tcp(s) => {
            assert!(!s.ip().is_unspecified(), "default must not bind 0.0.0.0");
        }
        ListenAddr::Unix(_) => {}
    }
    assert!(
        resolve_listen_addr(Some("0.0.0.0:50051")).is_err(),
        "0.0.0.0 bind must be rejected (security baseline)"
    );
    assert!(
        resolve_listen_addr(Some("127.0.0.1:50551")).is_ok(),
        "loopback bind must be accepted"
    );
}

// SCEN-1.3.2 / AC2: gRPC ContextService.Health -> SERVING.
#[tokio::test(flavor = "multi_thread")]
async fn test_1_3_2_health_serving() {
    // TEST-1.3.2
    let addr = ephemeral_addr();
    spawn_server(addr).await;

    let mut client = ContextServiceClient::connect(format!("http://{addr}"))
        .await
        .expect("gRPC client must connect to contextforge-core");
    let resp = client
        .health(HealthRequest {})
        .await
        .expect("Health RPC must succeed")
        .into_inner();
    assert_eq!(resp.status, "SERVING", "Health must report SERVING");
}

// SCEN-1.3.3 / AC3: tonic codegen wired; Search -> Status::unimplemented.
#[tokio::test(flavor = "multi_thread")]
async fn test_1_3_3_search_unimplemented() {
    // TEST-1.3.3
    let addr = ephemeral_addr();
    spawn_server(addr).await;

    let mut client = ContextServiceClient::connect(format!("http://{addr}"))
        .await
        .expect("gRPC client must connect");
    let err = client
        .search(SearchRequest::default())
        .await
        .expect_err("Search must be unimplemented in task-1.3 skeleton");
    assert_eq!(
        err.code(),
        tonic::Code::Unimplemented,
        "Search must return gRPC Unimplemented"
    );
}

// SCEN-1.3.4 / AC4: Phase 2+ module placeholders compile + are reachable.
#[test]
fn test_1_3_4_module_placeholders() {
    // TEST-1.3.4
    assert!(contextforge_core::scanner::placeholder_ready());
    assert!(contextforge_core::parser::placeholder_ready());
    assert!(contextforge_core::chunker::placeholder_ready());
    assert!(contextforge_core::indexer::placeholder_ready());
    assert!(contextforge_core::retriever::placeholder_ready());
    assert!(contextforge_core::memoryops::placeholder_ready());
}
