//! task-1.3 integration tests — TEST-1.3.1 ~ TEST-1.3.4 (SCEN-1.3.*).
//!
//! Drives the public `contextforge_core::server` surface (§5.3) + the AC4
//! module placeholders. RED skeleton: every call hits `unimplemented!()`
//! and panics -> tests fail functionally (not by compile error, §2.5.1).

use std::net::SocketAddr;
use std::time::Duration;

use contextforge_core::pb::context_service_client::ContextServiceClient;
use contextforge_core::pb::{HealthRequest, SearchRequest};
use contextforge_core::server::{
    resolve_listen_addr, resolve_listen_addr_with_opts, serve, ListenAddr,
};

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
    // Drive the deterministic _with_opts variant so this test is immune to a
    // future env-var leak from a parallel test setting
    // CONTEXTFORGE_ALLOW_WILDCARD_BIND.
    assert!(
        resolve_listen_addr_with_opts(Some("0.0.0.0:50051"), false).is_err(),
        "0.0.0.0 bind must be rejected (security baseline)"
    );
    assert!(
        resolve_listen_addr_with_opts(Some("127.0.0.1:50551"), false).is_ok(),
        "loopback bind must be accepted"
    );
}

// task-16.4 SCEN-16.4.1: explicit opt-in via _with_opts(allow_wildcard=true)
// — docker / k8s deployments where container network isolation makes 0.0.0.0
// safe (production fallback deny per ADR-018 + bridge network DNS).
#[tokio::test(flavor = "multi_thread")]
async fn test_1_3_1b_wildcard_allowed_with_opt_in() {
    assert!(
        resolve_listen_addr_with_opts(Some("0.0.0.0:50051"), true).is_ok(),
        "0.0.0.0 bind must be accepted with explicit opt-in (docker/k8s)"
    );
    assert!(
        resolve_listen_addr_with_opts(Some("[::]:50051"), true).is_ok(),
        ":: bind must be accepted with explicit opt-in (IPv6 wildcard)"
    );
    // Opt-in still rejects malformed addrs and still routes unix sockets.
    assert!(
        resolve_listen_addr_with_opts(Some("not-an-address"), true).is_err(),
        "opt-in must not bypass malformed-addr rejection"
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

// SCEN-1.3.3 / AC3: tonic codegen wired; Search reachable through the tonic
// transport. task-1.3 originally asserted `Status::unimplemented`; task-6.1
// (§2A 决策 A) replaced that placeholder with the real Retriever wire, so
// an empty-collections request now returns `InvalidArgument` per task-6.1
// §5.3 error mapping. AC3's core claim (tonic codegen wired) is still
// satisfied: the request traverses tonic transport → ContextServiceServer
// → CoreService::search, just with a richer error envelope.
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
        .expect_err("Search with default (empty-collections) request must error");
    // task-6.1 §5.3: collections empty → InvalidArgument (was Unimplemented
    // in task-1.3 baseline before §2A decision A replaced the placeholder).
    assert_eq!(
        err.code(),
        tonic::Code::InvalidArgument,
        "Search must return InvalidArgument for empty collections (task-6.1 §5.3)"
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
