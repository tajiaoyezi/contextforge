//! Phase 6 cli-api-export AC5 smoke — TEST-6.1.5 (SCEN-6.1.5 / AC5).
//!
//! 端到端走 tonic transport：索引 fixture → 起 tonic Search server →
//! 起 gRPC client → 调 ContextService.Search → 验返回的 RetrievalResult
//! 12 字段 + provenance.len() ≥ 1（AC3 黑盒守护沿 task-4.2）+ ADR-003 单一
//! proto-generated model（task-6.3 exporter 直接消费此 proto）.
//!
//! pattern 同 core/tests/phase4_smoke.rs (TEST-4.2.5) + phase2_smoke.rs
//! (TEST-2.4.5)，但 phase4 走 in-process Retriever 直调；本 smoke 加 tonic
//! transport（启 server + 起 client）覆盖 gRPC wire。

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use contextforge_core::chunker::ChunkPolicy;
use contextforge_core::indexer::IndexSession;
use contextforge_core::pb::context_service_client::ContextServiceClient;
use contextforge_core::pb::SearchRequest;
use contextforge_core::scanner::{default_denylist, ScanOptions};
use contextforge_core::server;
use tonic::transport::Server;

fn temp_root(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "contextforge-phase6-smoke-{name}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write(p: &PathBuf, c: &str) {
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(p, c).unwrap();
}

/// 拿一个临时可用 loopback 端口 — std::TcpListener bind+drop 释放给 server 重用.
/// race window: drop 到 Server::serve 之间极短（μs 级），单线程测试中无碰撞.
fn pick_loopback_addr() -> std::net::SocketAddr {
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let addr = l.local_addr().expect("local_addr");
    drop(l);
    addr
}

/// AC5 端到端 smoke — 主 agent §4 Gate 3 phase-6 smoke gate 候选入口
/// （Phase 6 phase smoke 真正落点在 task-6.3 exporter，本 smoke 仅 AC5 共享 model gate）.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase_6_search_grpc_end_to_end_smoke() {
    let src_root = temp_root("src");
    let data_dir = temp_root("data");
    let coll_id = "phase6-smoke";

    // ---- 1. 合成 fixture (含 unique trigger token) ----
    write(
        &src_root.join("README.md"),
        "# Phase6 smoke fixture\n\nUnique searchable token: phase6smokemarker77z body line.\n",
    );
    write(
        &src_root.join("docs/guide.md"),
        "# Guide\n\nAlso phase6smokemarker77z multi-hit fixture.\n",
    );

    let scan_opts = ScanOptions {
        denylist: default_denylist(),
        allowlist: Vec::new(),
        allow_denylist_override: false,
        dry_run: false,
        max_file_bytes: 10 * 1024 * 1024,
    };
    let mut sess = IndexSession::open(&data_dir, coll_id).expect("indexer open");
    sess.index_path(&src_root, &scan_opts, &ChunkPolicy::default(), vec![])
        .expect("index_path");
    sess.commit().expect("commit");

    // ---- 2. 起 tonic Search server（绑 loopback 临时端口）----
    let addr = pick_loopback_addr();
    let svc = server::context_service_with_data_dir(data_dir.clone());
    let server_handle = tokio::spawn(async move {
        Server::builder()
            .add_service(svc)
            .serve(addr)
            .await
    });

    // ---- 3. 起 gRPC client（重试 connect 等 server 启动 + 调 Search）----
    let endpoint = format!("http://{}", addr);
    let mut client_opt = None;
    for _ in 0..30 {
        match ContextServiceClient::connect(endpoint.clone()).await {
            Ok(c) => {
                client_opt = Some(c);
                break;
            }
            Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
    let mut client = client_opt.expect("AC5: gRPC server 未在 3s 内启动");

    let resp = client
        .search(SearchRequest {
            query: "phase6smokemarker77z".into(),
            collections: vec![coll_id.into()],
            agent_scope: vec![],
            top_k: 10,
            filters: None,
            explain: true,
            semantic: false,
        })
        .await
        .expect("AC5: gRPC Search 调用应 Ok");
    let inner = resp.into_inner();

    // ---- 4. 验 RetrievalResult 12 字段 + provenance ≥ 1 ----
    assert!(
        !inner.results.is_empty(),
        "AC5: end-to-end search 应有命中"
    );
    for (i, r) in inner.results.iter().enumerate() {
        // 12 explainable fields PRESENT (proto-generated struct 强制) + 内容 sanity
        assert!(!r.chunk_id.is_empty(), "AC5 #{}: chunk_id non-empty", i);
        assert_eq!(r.context_id, "", "AC5 #{}: §2A schema gap default", i);
        assert_eq!(r.source_type, "", "AC5 #{}: §2A schema gap default", i);
        assert!(!r.file_path.is_empty(), "AC5 #{}: file_path non-empty", i);
        assert!(r.line_end >= r.line_start, "AC5 #{}: line range valid", i);
        assert!(r.score > 0.0, "AC5 #{}: score > 0", i);
        assert_eq!(r.retrieval_method, "bm25", "AC5 #{}: method=bm25", i);
        assert!(!r.reason.is_empty(), "AC5 #{}: reason 非空 (explain=true)", i);
        assert!(r.agent_scope.is_empty(), "AC5 #{}: §2A default empty", i);
        assert_eq!(
            r.redaction_status, "applied",
            "AC5 #{}: §2A default 'applied'",
            i
        );
        assert!(
            !r.provenance.is_empty(),
            "AC5/AC3 黑盒守护 #{}: provenance.len() ≥ 1, got {}",
            i,
            r.provenance.len()
        );
    }

    server_handle.abort();
    let _ = server_handle.await;
}
