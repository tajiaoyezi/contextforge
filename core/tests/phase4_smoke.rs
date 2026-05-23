//! Phase 4 端到端 smoke — TEST-4.2.5 (SCEN-4.2.5 / AC5).
//!
//! 主 agent §4 Gate 3 phase-4 smoke gate 触发点（task spec §9 + §6 AC5 / phase-4 spec §6）：
//!
//!   cargo test --test phase4_smoke -- --nocapture
//!
//! 一个 `#[test] fn phase_4_end_to_end_smoke()` 跑完整链路：
//!   scanner.scan_path → parser → chunker → indexer (SQLite + Tantivy) → retriever → explain
//! 并断言（task-4.2 §6 AC1-4 全链路 sanity）：
//!   * AC1: 12-field explainable contract PRESENT 每条 result
//!   * AC2: file_path + line_start/line_end 精确定位回 fixture
//!   * AC3: 每条 result.provenance.len() ≥ 1 （黑盒守护）
//!   * AC4: Retriever::explain() 返 Ok + reason / matched_terms 非空
//!   * 空 query 安全（不 panic，返 Ok(Vec::new()))
//!
//! v0.1 用合成 fixture（不依赖 test/fixtures/shared/），Phase 8 真实大仓库压测另起。
//! Pattern 与 core/tests/phase2_smoke.rs（TEST-2.4.5）一致.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use contextforge_core::chunker::ChunkPolicy;
use contextforge_core::indexer::IndexSession;
use contextforge_core::retriever::{Retriever, SearchFilters, SearchOptions};
use contextforge_core::scanner::{default_denylist, ScanOptions};

fn temp_root(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "contextforge-phase4-smoke-{name}-{}-{nanos}",
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

/// AC5: Phase 4 端到端 smoke — 主 agent §4 Gate 3 调用入口。
///
/// 全链路：scanner → parser → chunker → indexer → retriever → explain.
/// 断言 task-4.2 AC1-4 端到端正确 + AC3 黑盒守护 + AC4 explain debug entry.
#[test]
fn phase_4_end_to_end_smoke() {
    let src_root = temp_root("src");
    let data_dir = temp_root("data");
    let coll_id = "phase4-smoke";

    // ---- 合成 fixture：多文件 + 含 unique trigger token ----
    write(
        &src_root.join("README.md"),
        "# ContextForge\n\nSmoke fixture for phase-4 retrieval.\n\
         Unique searchable token: phase4smokemarker991.\n\
         Another line of body.\n",
    );
    write(
        &src_root.join("docs/guide.md"),
        "# Guide\n\nSecond document body.\n\
         Also contains phase4smokemarker991 for multi-hit.\n",
    );
    write(
        &src_root.join("notes.md"),
        "# Notes\n\nThird doc without trigger token (negative).\n",
    );

    let scan_opts = ScanOptions {
        denylist: default_denylist(),
        allowlist: Vec::new(),
        allow_denylist_override: false,
        dry_run: false,
        max_file_bytes: 10 * 1024 * 1024,
    };

    // ---- 上半段：scanner → parser → chunker → indexer ----
    let mut sess = IndexSession::open(&data_dir, coll_id).expect("phase4 smoke: open indexer");
    let stats = sess
        .index_path(&src_root, &scan_opts, &ChunkPolicy::default(), vec![])
        .expect("phase4 smoke: index_path");
    sess.commit().expect("phase4 smoke: commit");

    assert!(
        stats.files_indexed >= 2,
        "phase4 smoke: 应索引 ≥2 文件 (含 trigger), got {}",
        stats.files_indexed
    );

    // ---- 下半段：retriever 命中 + 12-field explainable result ----
    let retr = Retriever::open(&data_dir, coll_id).expect("phase4 smoke: retriever open");

    let results = retr
        .search(&SearchOptions {
            query: "phase4smokemarker991".into(),
            top_k: 10,
            filters: SearchFilters::default(),
            explain: false,
        })
        .expect("phase4 smoke: search");
    assert!(
        !results.is_empty(),
        "phase4 smoke: 应命中 trigger token, got 0 hits (scanner→indexer→retriever 链路断了？)"
    );

    // AC1 12-field PRESENT (struct 强制；运行时再 sanity 校验默认值)
    for (i, r) in results.iter().enumerate() {
        assert!(!r.chunk_id.is_empty(), "AC1 #{}: chunk_id non-empty", i);
        assert_eq!(r.context_id, "", "AC1 §2A #{}: context_id v0.1 默认 \"\"", i);
        assert_eq!(r.source_type, "", "AC1 §2A #{}: source_type v0.1 默认 \"\"", i);
        assert!(!r.file_path.is_empty(), "AC1 #{}: file_path non-empty", i);
        assert!(r.line_end >= r.line_start, "AC1 #{}: line range valid", i);
        assert!(r.score > 0.0, "AC1 #{}: score > 0", i);
        assert_eq!(r.retrieval_method, "bm25", "AC1 #{}: method=bm25", i);
        assert!(r.agent_scope.is_empty(), "AC1 §2A #{}: agent_scope v0.1 默认 empty", i);
        assert_eq!(
            r.redaction_status, "applied",
            "AC1 §2A #{}: redaction_status v0.1 默认 \"applied\"",
            i
        );
        // AC3 黑盒守护 — 每条 ≥1 provenance entry
        assert!(
            !r.provenance.is_empty(),
            "AC3 黑盒守护 #{}: provenance.len() ≥ 1, got {} (chunk_id={}, file_path={})",
            i,
            r.provenance.len(),
            r.chunk_id,
            r.file_path
        );
    }

    // AC2 file_path + line 精确定位回 fixture
    let any_in_readme = results
        .iter()
        .any(|r| r.file_path.ends_with("README.md") || r.file_path.ends_with("guide.md"));
    assert!(
        any_in_readme,
        "AC2: 至少一条 result.file_path 精确指向 fixture (README.md 或 docs/guide.md)，got file_paths: {:?}",
        results.iter().map(|r| &r.file_path).collect::<Vec<_>>()
    );

    // AC4 Retriever::explain debug entry — reason + matched_terms enriched
    let explained = retr
        .explain(&SearchOptions {
            query: "phase4smokemarker991".into(),
            top_k: 10,
            filters: SearchFilters::default(),
            explain: false, // explain() 内部 force = true
        })
        .expect("AC4 phase4 smoke: Retriever::explain 应返 Ok");
    assert!(!explained.is_empty(), "AC4: explain 应有结果");
    let r0 = &explained[0];
    assert!(
        !r0.reason.is_empty(),
        "AC4: explain() reason 应非空, got: {:?}",
        r0.reason
    );
    assert!(
        !r0.matched_terms.is_empty(),
        "AC4: explain() matched_terms 应非空"
    );

    // 空 query 安全（不 panic）— 复 task-4.1 AC3 行为，phase 端到端再断言一次
    let empty_q = retr
        .search(&SearchOptions {
            query: "".into(),
            top_k: 10,
            filters: SearchFilters::default(),
            explain: false,
        })
        .expect("phase4 smoke: 空 query 应 Ok 不 Err");
    assert!(empty_q.is_empty(), "phase4 smoke: 空 query 返空 Vec");
}
