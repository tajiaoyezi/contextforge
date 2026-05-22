//! Phase 2 端到端 smoke — TEST-2.4.5 (SCEN-2.4.5 / AC5).
//!
//! 主 agent §4 Gate 3 phase-2 smoke gate 触发点（task spec §9 + §6 AC5）：
//!
//!   cargo test --test phase2_smoke -- --nocapture
//!
//! 一个 `#[test] fn phase_2_end_to_end_smoke()` 跑完整链路：
//!   scanner.scan_path → parser → chunker → indexer (SQLite + Tantivy)
//! 并断言：
//!   * SQLite chunks 表行数 > 0 (AC2 truth)
//!   * Tantivy 全文命中正常 token (AC2 inverted)
//!   * denylisted (.env) 内容不入索引 (AC3)
//!   * 原始 secret 内容不入索引 (AC3 redaction)
//!   * IndexStats.files_indexed > 0 (AC1)
//!
//! v0.1 用合成 fixture（不依赖 test/fixtures/shared/），Phase 8 真实大仓库压测另起。

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use contextforge_core::chunker::ChunkPolicy;
use contextforge_core::indexer::IndexSession;
use contextforge_core::scanner::{default_denylist, ScanOptions};

fn temp_root(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "contextforge-phase2-smoke-{name}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write(path: &PathBuf, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

/// AC5: Phase 2 端到端 smoke — 主 agent §4 Gate 3 调用入口。
#[test]
fn phase_2_end_to_end_smoke() {
    let src_root = temp_root("src");
    let data_dir = temp_root("data");

    // ---- 合成 fixture：normal / denylisted / secret ----
    write(
        &src_root.join("README.md"),
        "# ContextForge\n\nThis is a small fixture for the phase-2 smoke. \
         A unique searchable token: phase2smokemarkerz3q1.\n",
    );
    write(
        &src_root.join("src/lib.md"),
        "# Lib\n\nSecond document with another body line.\n",
    );
    // denylisted (.env 在 default_denylist) — scanner 应跳过
    write(
        &src_root.join(".env"),
        "DB_PASSWORD=plaintext_smoke_password_should_be_skipped\n",
    );
    // 含 AWS key 模式的非 denylisted 文件 — scanner 应 redact
    write(
        &src_root.join("config.md"),
        "# Config\nAWS key: AKIAIOSFODNN7EXAMPLE — should be redacted.\n",
    );

    let scan_opts = ScanOptions {
        denylist: default_denylist(),
        allowlist: Vec::new(),
        allow_denylist_override: false,
        dry_run: false,
        max_file_bytes: 10 * 1024 * 1024,
    };

    let mut sess = IndexSession::open(&data_dir, "smoke-collection").expect("open session");

    let stats = sess
        .index_path(&src_root, &scan_opts, &ChunkPolicy::default(), vec![])
        .expect("index_path");

    sess.commit().expect("commit");

    // ---- AC1: 至少索引到 normal/lib/config 三个文件（denied 跳过）----
    assert!(
        stats.files_indexed >= 3,
        "AC1 (smoke): 应索引 ≥3 normal files (denied 跳过), got {}",
        stats.files_indexed
    );

    // ---- AC2 truth: SQLite chunks 表有数据 ----
    let chunk_count = sess.sqlite_chunk_count().expect("sqlite count");
    assert!(
        chunk_count > 0,
        "AC2 (smoke): SQLite chunks 表应 >0, got {}",
        chunk_count
    );

    // ---- AC2 inverted: Tantivy 全文命中已知 token ----
    let hits = sess
        .tantivy_search("phase2smokemarkerz3q1", 10)
        .expect("tantivy search");
    assert!(
        !hits.is_empty(),
        "AC2 (smoke): Tantivy 应命中 unique token 'phase2smokemarkerz3q1'"
    );

    // ---- AC3 denylist: .env 内容不入索引 ----
    let env_hits = sess
        .tantivy_search("plaintext_smoke_password_should_be_skipped", 10)
        .expect("tantivy search");
    assert!(
        env_hits.is_empty(),
        "AC3 (smoke): .env 内容不应入索引（denylist 在 scanner 已跳过），但命中 {} 个",
        env_hits.len()
    );

    // ---- AC3 redaction: 原始 AWS key 不应被搜到 ----
    let secret_hits = sess
        .tantivy_search("AKIAIOSFODNN7EXAMPLE", 10)
        .expect("tantivy search");
    assert!(
        secret_hits.is_empty(),
        "AC3 (smoke): 原始 secret 不应入索引（scanner 已 redact），但命中 {} 个",
        secret_hits.len()
    );

    eprintln!(
        "[phase-2 smoke] data_dir={:?} collection={} stats={:?} chunk_count={}",
        sess.data_dir(),
        sess.collection_id(),
        stats,
        chunk_count
    );
}
