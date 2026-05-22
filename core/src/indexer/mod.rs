//! task-2.4 (Phase 2): indexer — Tantivy 全文索引 + SQLite metadata/chunk 存储 + 增量.
//!
//! RED checkpoint: types per §5.3 contract; method bodies are deliberate stubs
//! (返回 IndexStats::default() / 空 Vec) so the 4 unit tests + AC5 integration smoke
//! compile + fail with descriptive assertions. GREEN commit replaces stubs with real
//! tantivy 0.26 + rusqlite 0.39 impl (per chore PR #23).
//!
//! 数据目录布局（PRD §Local data directory v0.1）：
//!   <data_dir>/collections/<collection_id>/{metadata.sqlite, tantivy/}
//!
//! 同步策略 (ADR-002): SQLite 真值源 + Tantivy 全文倒排（best-effort，可重建）。

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::chunker::{ChunkPolicy, Provenance};
use crate::scanner::{RedactionStatus, ScanOptions};

/// Retained for task-1.3 core_skeleton.rs anchor (AC4 wiring). Returns true.
pub fn placeholder_ready() -> bool {
    true
}

/// 索引会话：单 collection 单数据目录。生命周期管理 SQLite 连接 + Tantivy IndexWriter.
pub struct IndexSession {
    data_dir: PathBuf,
    collection_id: String,
    // GREEN: 真实字段 — rusqlite::Connection + tantivy::Index + tantivy::IndexWriter
    // + tantivy field handles。RED stub 仅记录 data_dir/collection_id。
}

#[derive(Error, Debug)]
pub enum IndexError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite: {0}")]
    Sqlite(String),
    #[error("tantivy: {0}")]
    Tantivy(String),
    #[error("scan: {0}")]
    Scan(String),
    #[error("parse: {0}")]
    Parse(String),
    #[error("chunk: {0}")]
    Chunk(String),
    #[error("redaction status unsafe for indexing: {0:?}")]
    UnsafeRedaction(RedactionStatus),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexStats {
    pub files_indexed: usize,
    pub files_skipped_denied: usize,
    pub files_skipped_redaction: usize,
    pub chunks_written: usize,
    pub chunks_updated: usize,
    pub chunks_deleted: usize,
}

impl IndexSession {
    /// 打开（或创建）索引会话；GREEN: 确保 <data_dir>/collections/<id>/ + 建 SQLite schema +
    /// 打开/创建 Tantivy index + 持久化 IndexWriter。RED stub 仅记录路径。
    pub fn open(data_dir: &Path, collection_id: &str) -> Result<Self, IndexError> {
        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            collection_id: collection_id.to_string(),
        })
    }

    /// 全量索引：scan root → for ScannedFile (已 redact + 跳 denylist) → parse → chunk → 写双存储。
    ///
    /// RED stub: 返回 IndexStats::default()（全 0）→ AC1/AC2/AC3 测试失败。
    pub fn index_path(
        &mut self,
        _root: &Path,
        _scan_options: &ScanOptions,
        _policy: &ChunkPolicy,
        _provenance: Vec<Provenance>,
    ) -> Result<IndexStats, IndexError> {
        Ok(IndexStats::default())
    }

    /// 增量：单文件 partial reindex（AC4）— 比对 files.content_hash 决定 skip / 删旧 + 重插。
    ///
    /// RED stub: 返回 IndexStats::default()（全 0）→ AC4 测试失败。
    pub fn reindex_file(
        &mut self,
        _path: &Path,
        _scan_options: &ScanOptions,
        _policy: &ChunkPolicy,
        _provenance: Vec<Provenance>,
    ) -> Result<IndexStats, IndexError> {
        Ok(IndexStats::default())
    }

    /// 提交 Tantivy IndexWriter pending writes（commit）；SQLite 已在事务内自提交。
    pub fn commit(&mut self) -> Result<(), IndexError> {
        Ok(())
    }

    /// 查询 SQLite chunks 表行数（AC2 testing helper）。RED stub: 0.
    pub fn sqlite_chunk_count(&self) -> Result<u64, IndexError> {
        Ok(0)
    }

    /// Tantivy 全文查询（AC2 testing helper）；返回命中 chunk_id 列表。RED stub: 空 Vec.
    pub fn tantivy_search(&self, _query: &str, _limit: usize) -> Result<Vec<String>, IndexError> {
        Ok(Vec::new())
    }

    /// Accessor — 用于诊断 / smoke 输出。
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Accessor — 用于诊断 / smoke 输出。
    pub fn collection_id(&self) -> &str {
        &self.collection_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunker::ChunkPolicy;
    use crate::scanner::{default_denylist, ScanOptions};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Same pattern as scanner tests (core/tests/scanner.rs): self-managed tmp dirs
    // via std::env::temp_dir() — no `tempfile` dep needed.
    fn temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "contextforge-indexer-{name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn make_scan_options() -> ScanOptions {
        ScanOptions {
            denylist: default_denylist(),
            allowlist: Vec::new(),
            allow_denylist_override: false,
            dry_run: false,
            max_file_bytes: 10 * 1024 * 1024,
        }
    }

    // ---- TEST-2.4.1 / SCEN-2.4.1 (AC1) — ≥1000 文件索引 ----
    #[test]
    fn test_2_4_1_indexes_at_least_1000_files() {
        let src_root = temp_root("ac1-src");
        let data_dir = temp_root("ac1-data");

        // 生成 1010 个小文件（>=1000 满足 AC1 + 留 buffer）
        let n_files: usize = 1010;
        for i in 0..n_files {
            let dir = src_root.join(format!("dir{:03}", i / 50));
            fs::create_dir_all(&dir).unwrap();
            let fp = dir.join(format!("note{:05}.md", i));
            fs::write(&fp, format!("# Note {}\nbody line\n", i)).unwrap();
        }

        let mut sess = IndexSession::open(&data_dir, "test-coll").expect("open");
        let stats = sess
            .index_path(&src_root, &make_scan_options(), &ChunkPolicy::default(), vec![])
            .expect("index_path");

        assert!(
            stats.files_indexed >= 1000,
            "AC1: 应索引 ≥1000 文件, got {}",
            stats.files_indexed
        );
        assert!(
            stats.chunks_written > 0,
            "AC1: 至少 1 chunk 写入, got {}",
            stats.chunks_written
        );
    }

    // ---- TEST-2.4.2 / SCEN-2.4.2 (AC2) — SQLite + Tantivy 双查 ----
    #[test]
    fn test_2_4_2_sqlite_and_tantivy_queryable() {
        let src_root = temp_root("ac2-src");
        let data_dir = temp_root("ac2-data");

        fs::write(
            src_root.join("readme.md"),
            "# Project Readme\nUnique token uniquephrasex9k7\n",
        )
        .unwrap();
        fs::write(src_root.join("notes.md"), "# Notes\nanother body line\n").unwrap();

        let mut sess = IndexSession::open(&data_dir, "test-coll").unwrap();
        sess.index_path(&src_root, &make_scan_options(), &ChunkPolicy::default(), vec![])
            .unwrap();
        sess.commit().unwrap();

        // SQLite truth: ≥1 chunk written
        let count = sess.sqlite_chunk_count().expect("sqlite count");
        assert!(count >= 1, "AC2: SQLite chunks 表应有 ≥1 行, got {}", count);

        // Tantivy 全文：查 unique token 应命中
        let hits = sess.tantivy_search("uniquephrasex9k7", 10).expect("tantivy");
        assert!(
            !hits.is_empty(),
            "AC2: Tantivy 应命中 unique token (got 0 hits)"
        );
    }

    // ---- TEST-2.4.3 / SCEN-2.4.3 (AC3) — denylist + redaction 链路守住 ----
    #[test]
    fn test_2_4_3_denylist_and_redaction_in_index_pipeline() {
        let src_root = temp_root("ac3-src");
        let data_dir = temp_root("ac3-data");

        // 正常文件 — 含可搜索独特 token
        fs::write(
            src_root.join("notes.md"),
            "# Notes\nordinarytokenz8q4 body\n",
        )
        .unwrap();
        // denylisted 文件（.env 在 scanner default_denylist 内）
        fs::write(src_root.join(".env"), "API_KEY=plain_secret\n").unwrap();
        // 含 secret pattern 的非 denylisted 文件 — scanner 应 redact AWS key 模式
        fs::write(
            src_root.join("config.md"),
            "# Config\nAWS key: AKIAIOSFODNN7EXAMPLE referenced here\n",
        )
        .unwrap();

        let mut sess = IndexSession::open(&data_dir, "test-coll").unwrap();
        sess.index_path(&src_root, &make_scan_options(), &ChunkPolicy::default(), vec![])
            .unwrap();
        sess.commit().unwrap();

        // 正常 token 可被 Tantivy 命中（确认链路活）
        let normal_hits = sess.tantivy_search("ordinarytokenz8q4", 10).expect("tantivy");
        assert!(
            !normal_hits.is_empty(),
            "AC3 sanity: 正常 token 应被命中"
        );

        // .env 内容（plain_secret）不应入索引（denylist 在 scanner 已跳过）
        let env_hits = sess.tantivy_search("plain_secret", 10).expect("tantivy");
        assert!(
            env_hits.is_empty(),
            "AC3 denylist: .env 内容不应入索引, 但命中 {} 个",
            env_hits.len()
        );

        // 原始 secret 文本（AKIA...）不应在 Tantivy 中可搜（scanner 已 redact 为 [REDACTED:*]）
        let secret_hits = sess
            .tantivy_search("AKIAIOSFODNN7EXAMPLE", 10)
            .expect("tantivy");
        assert!(
            secret_hits.is_empty(),
            "AC3 redaction: 原始 secret 不应入索引, 但命中 {} 个",
            secret_hits.len()
        );
    }

    // ---- TEST-2.4.4 / SCEN-2.4.4 (AC4) — 基础增量更新 ----
    #[test]
    fn test_2_4_4_incremental_reindex_single_file() {
        let src_root = temp_root("ac4-src");
        let data_dir = temp_root("ac4-data");

        let fp = src_root.join("doc.md");
        fs::write(&fp, "# Doc v1\noldtokenx1y2z3\n").unwrap();
        fs::write(src_root.join("other.md"), "# Other\nother content\n").unwrap();

        let mut sess = IndexSession::open(&data_dir, "test-coll").unwrap();
        let stats_full = sess
            .index_path(&src_root, &make_scan_options(), &ChunkPolicy::default(), vec![])
            .unwrap();
        sess.commit().unwrap();
        assert!(
            stats_full.files_indexed >= 2,
            "AC4 setup: 初始索引应处理 ≥2 文件"
        );

        // 修改 doc.md 内容
        fs::write(&fp, "# Doc v2\nnewtokenq8r9s0\n").unwrap();

        // 增量重索引单文件
        let stats_inc = sess
            .reindex_file(&fp, &make_scan_options(), &ChunkPolicy::default(), vec![])
            .unwrap();
        sess.commit().unwrap();

        // AC4: 应至少 chunks_updated > 0 或 (chunks_deleted > 0 AND chunks_written > 0)
        let changed = stats_inc.chunks_updated > 0
            || (stats_inc.chunks_deleted > 0 && stats_inc.chunks_written > 0);
        assert!(
            changed,
            "AC4: reindex_file 应有 chunks 变动, got stats={:?}",
            stats_inc
        );

        // 新 token 可被命中；旧 token 不应再命中
        let new_hits = sess.tantivy_search("newtokenq8r9s0", 10).unwrap();
        assert!(!new_hits.is_empty(), "AC4: 新 token 应入索引");
        let old_hits = sess.tantivy_search("oldtokenx1y2z3", 10).unwrap();
        assert!(
            old_hits.is_empty(),
            "AC4: 旧 token 应从索引删除, 但命中 {} 个",
            old_hits.len()
        );
    }
}
