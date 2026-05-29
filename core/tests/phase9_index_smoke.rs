//! Phase 9 task-9.2 §6 AC5 — `core/tests/phase9_index_smoke.rs`.
//!
//! 端到端走 tonic transport 的 `CoreService.Index` server-streaming RPC：
//! 临时 data_dir + 临时 source_path（≥3 .md + 1 .env denied + 1 secret-redacted
//! .yaml）→ 起 tonic in-process Index server → 起 gRPC client → 调
//! `client.index()` consume stream → assert：
//!   * 收到 ≥4 IndexProgress 消息（≥3 normal file 一次 + final done=true）
//!   * final.files_processed ≥ 3 + chunks_written > 0 + error == ""
//!   * .env 计入 files_skipped_denied（scanner denylist 跳过 → 计入 report.skipped）
//!   * SQLite chunks 表 row > 0 + Tantivy 搜索 fixture marker 命中
//!   * secret literal AKIAIOSFODNN7EXAMPLE 不可被 Tantivy 检索到（redaction）
//!
//! 错误路径子测试：source_path = "/nonexistent" → client.index() 立即 Status::InvalidArgument。

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use contextforge_core::indexer::IndexSession;
use contextforge_core::pb::context_service_client::ContextServiceClient;
use contextforge_core::pb::IndexRequest;
use contextforge_core::server;
use tonic::transport::Server;

const PHASE9_MARKER: &str = "phase9smokemarker99x";
const SECRET: &str = "AKIAIOSFODNN7EXAMPLE";

fn temp_root(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "contextforge-phase9-smoke-{name}-{}-{nanos}",
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

fn pick_loopback_addr() -> std::net::SocketAddr {
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let addr = l.local_addr().expect("local_addr");
    drop(l);
    addr
}

/// AC5 端到端 smoke — tonic transport + Index stream consumption.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase_9_index_grpc_end_to_end_smoke() {
    let src_root = temp_root("src");
    let data_dir = temp_root("data");
    let coll_id = "phase9-smoke";

    // ---- 1. 合成 fixture ----
    // 3 normal .md files (含 unique marker)
    write(
        &src_root.join("README.md"),
        &format!("# Phase9 smoke\n\nUnique marker: {} body line.\n", PHASE9_MARKER),
    );
    write(
        &src_root.join("docs/guide.md"),
        &format!("# Guide\n\nAlso {} multi-hit fixture.\n", PHASE9_MARKER),
    );
    write(
        &src_root.join("notes/log.md"),
        "# Notes\n\nRegular content; no marker here.\n",
    );

    // 1 .env (denylist — scanner 不会读，files_skipped_denied 计入 report)
    write(
        &src_root.join(".env"),
        "TOKEN=should-not-be-indexed-and-is-denylisted\n",
    );

    // 1 .yaml 含 fake AWS key (secret-redaction — scanner 替换 secret 字符串)
    write(
        &src_root.join("config.yaml"),
        &format!("aws_key: {}\nendpoint: https://api.example.invalid\n", SECRET),
    );

    // ---- 2. 起 tonic Index server (绑 loopback 临时端口) ----
    let addr = pick_loopback_addr();
    let svc = server::context_service_with_data_dir(data_dir.clone());
    let server_handle = tokio::spawn(async move {
        Server::builder().add_service(svc).serve(addr).await
    });

    // ---- 3. 起 gRPC client + 调 Index → consume stream ----
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
        .index(IndexRequest {
            source_path: src_root.to_string_lossy().into_owned(),
            data_dir: data_dir.to_string_lossy().into_owned(),
            collection_id: coll_id.into(),
        })
        .await
        .expect("AC5: gRPC Index 调用应 Ok");

    let mut stream = resp.into_inner();
    let mut messages = Vec::new();
    while let Some(msg) = stream.message().await.expect("stream.message Err") {
        messages.push(msg);
    }

    // ---- 4. assert: 收到 ≥4 messages (3 normal + 1 yaml redaction + final done) ----
    assert!(
        messages.len() >= 4,
        "AC4/AC5: 应收 ≥4 IndexProgress 消息（≥3 normal + final done），got {} 条",
        messages.len()
    );
    let final_msg = messages.last().expect("final message");
    assert!(final_msg.done, "AC4: final message done=true");
    assert!(
        final_msg.error.is_empty(),
        "AC5: final.error 应为空（indexer 成功），got {:?}",
        final_msg.error
    );
    assert!(
        final_msg.files_processed >= 3,
        "AC5: 至少 3 normal .md indexed，got {}",
        final_msg.files_processed
    );
    assert!(
        final_msg.chunks_written > 0,
        "AC5: chunks_written > 0，got {}",
        final_msg.chunks_written
    );
    assert!(
        final_msg.files_skipped_denied >= 1,
        "AC5: .env 至少 1 个 denylist skip，got {}",
        final_msg.files_skipped_denied
    );

    // ---- 5. assert: SQLite + Tantivy 实际有数据 + secret 不可检索 ----
    //
    // Shut down the in-process tonic server BEFORE re-opening IndexSession on
    // the same data_dir. The server-side IndexSession still holds the Tantivy
    // IndexWriter's `directory.lock` until its task is cancelled and dropped;
    // without this explicit shutdown + yield, the line below
    // (IndexSession::open) intermittently failed with
    // `Failed to acquire Lockfile: LockBusy` — observed in CI on PR #118
    // cargo-test (run 26577717038) and PR #121 cargo-test (run 26643146337).
    // The original `server_handle.abort()` at the end of the test was too
    // late for the re-open path. yield_now() gives the cancelled task one
    // poll to run its Drop chain (which releases the Tantivy lock).
    server_handle.abort();
    let _ = server_handle.await;
    tokio::task::yield_now().await;

    let session = IndexSession::open(&data_dir, coll_id).expect("re-open indexer for assertions");
    let chunk_count = session.sqlite_chunk_count().expect("sqlite count");
    assert!(
        chunk_count > 0,
        "AC5: SQLite chunks 表 row > 0，got {}",
        chunk_count
    );

    let marker_hits = session
        .tantivy_search(PHASE9_MARKER, 10)
        .expect("tantivy_search marker");
    assert!(
        !marker_hits.is_empty(),
        "AC5: Tantivy 应命中 fixture marker {}",
        PHASE9_MARKER
    );

    let secret_hits = session
        .tantivy_search(SECRET, 10)
        .expect("tantivy_search secret");
    assert!(
        secret_hits.is_empty(),
        "AC5/R4: 原始 secret 不应入索引（scanner 已 redact），但命中 {} 个",
        secret_hits.len()
    );
}

/// AC3 错误路径 — source_path 不存在 → 流建立前返回 Status::InvalidArgument.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase_9_index_invalid_source_path_returns_invalid_argument() {
    let data_dir = temp_root("data-err");
    let coll_id = "phase9-err";

    let addr = pick_loopback_addr();
    let svc = server::context_service_with_data_dir(data_dir.clone());
    let server_handle = tokio::spawn(async move {
        Server::builder().add_service(svc).serve(addr).await
    });

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
    let mut client = client_opt.expect("server 未启动");

    let err = client
        .index(IndexRequest {
            source_path: format!(
                "{}-nonexistent-phase9",
                std::env::temp_dir().to_string_lossy()
            ),
            data_dir: data_dir.to_string_lossy().into_owned(),
            collection_id: coll_id.into(),
        })
        .await
        .expect_err("AC3: nonexistent source_path 应返 Err 而非 Ok(stream)");

    assert_eq!(
        err.code(),
        tonic::Code::InvalidArgument,
        "AC3: nonexistent source_path 应 InvalidArgument, got {:?}",
        err.code()
    );

    server_handle.abort();
    let _ = server_handle.await;
}
