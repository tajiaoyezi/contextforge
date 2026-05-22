//! task-2.4 (Phase 2): indexer — Tantivy 0.26 全文索引 + rusqlite 0.39 SQLite 存储 + 增量更新.
//!
//! 数据目录布局（PRD §Local data directory v0.1）：
//!   <data_dir>/collections/<collection_id>/{metadata.sqlite, tantivy/}
//!
//! 同步策略 (ADR-002): SQLite 真值源（chunks/files/provenance 三表）+ Tantivy 全文倒排
//! （5 字段 schema：chunk_id PK / content TEXT / file_path STRING / language STRING /
//! line_start, line_end I64）。Tantivy 失败时 SQLite 仍 truth，可重建。
//!
//! 链路上游：scanner → parser → chunker → indexer
//!   - scanner 已做 denylist 跳过 + secret redact (BINDING: redaction_status="pending" 触发警告)
//!   - indexer 只消费 ScannedFile.redacted_content（不触原始 secret）

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use rusqlite::{params, Connection};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, Value, FAST, INDEXED, STORED, STRING, TEXT};
use tantivy::{doc, Index, IndexWriter, ReloadPolicy, TantivyDocument, Term};
use thiserror::Error;

use crate::chunker::{chunk_units, content_hash, Chunk, ChunkPolicy, Provenance};
use crate::parser::parse_content;
use crate::scanner::{scan_file, scan_path, RedactionStatus, ScanOptions};

/// Retained for task-1.3 core_skeleton.rs anchor (AC4 wiring). Returns true.
pub fn placeholder_ready() -> bool {
    true
}

/// Tantivy IndexWriter memory budget (50 MB) — tantivy 推荐默认下限.
const TANTIVY_WRITER_BUDGET: usize = 50_000_000;

/// 索引会话：单 collection 单数据目录。生命周期管理 SQLite 连接 + Tantivy IndexWriter.
pub struct IndexSession {
    data_dir: PathBuf,
    collection_id: String,
    sqlite: Connection,
    tantivy_index: Index,
    tantivy_writer: Mutex<IndexWriter>,
    f_chunk_id: Field,
    f_content: Field,
    f_file_path: Field,
    f_language: Field,
    f_line_start: Field,
    f_line_end: Field,
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

impl From<rusqlite::Error> for IndexError {
    fn from(e: rusqlite::Error) -> Self {
        IndexError::Sqlite(e.to_string())
    }
}

impl From<tantivy::TantivyError> for IndexError {
    fn from(e: tantivy::TantivyError) -> Self {
        IndexError::Tantivy(e.to_string())
    }
}

impl From<tantivy::query::QueryParserError> for IndexError {
    fn from(e: tantivy::query::QueryParserError) -> Self {
        IndexError::Tantivy(format!("query: {}", e))
    }
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

/// SQLite 3 表 schema — chunks / files / provenance. §5.3 已冻结。
const SQL_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS chunks (
    chunk_id      TEXT PRIMARY KEY,
    file_path     TEXT NOT NULL,
    line_start    INTEGER NOT NULL,
    line_end      INTEGER NOT NULL,
    language      TEXT,
    content       TEXT NOT NULL,
    content_hash  TEXT NOT NULL,
    kind          TEXT,
    collection_id TEXT NOT NULL,
    indexed_at    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_chunks_file_path    ON chunks(file_path);
CREATE INDEX IF NOT EXISTS idx_chunks_content_hash ON chunks(content_hash);

CREATE TABLE IF NOT EXISTS files (
    file_path    TEXT PRIMARY KEY,
    content_hash TEXT NOT NULL,
    mtime_unix   INTEGER NOT NULL,
    indexed_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS provenance (
    chunk_id           TEXT NOT NULL,
    importer           TEXT NOT NULL,
    original_path      TEXT NOT NULL,
    imported_at        TEXT NOT NULL,
    source_modified_at TEXT,
    FOREIGN KEY (chunk_id) REFERENCES chunks(chunk_id) ON DELETE CASCADE
);
"#;

fn build_tantivy_schema() -> (Schema, [Field; 6]) {
    let mut sb = Schema::builder();
    let chunk_id = sb.add_text_field("chunk_id", STRING | STORED);
    let content = sb.add_text_field("content", TEXT | STORED);
    let file_path = sb.add_text_field("file_path", STRING | STORED);
    let language = sb.add_text_field("language", STRING | STORED);
    let line_start = sb.add_i64_field("line_start", STORED | INDEXED | FAST);
    let line_end = sb.add_i64_field("line_end", STORED | INDEXED | FAST);
    (
        sb.build(),
        [chunk_id, content, file_path, language, line_start, line_end],
    )
}

// FIX-2 (PR #24 reviewer): 名实不符 — 实际返回 unix epoch seconds (decimal string),
// 不是 RFC3339。改名 indexed_at_now_str() 避免误导.
fn indexed_at_now_str() -> String {
    let now = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", now.as_secs())
}

fn lang_hint_from_path(p: &Path) -> &'static str {
    match p.extension().and_then(|s| s.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("go") => "go",
        Some("rs") => "rust",
        Some("py") => "python",
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") => "javascript",
        Some("md") => "markdown",
        Some("log") | Some("jsonl") => "log",
        Some("json") => "json",
        Some("yaml") | Some("yml") => "yaml",
        Some("toml") => "toml",
        _ => "text",
    }
}

impl IndexSession {
    /// 打开（或创建）索引会话；建 SQLite schema + 打开/创建 Tantivy index + 持久化 IndexWriter.
    pub fn open(data_dir: &Path, collection_id: &str) -> Result<Self, IndexError> {
        let coll_dir = data_dir.join("collections").join(collection_id);
        fs::create_dir_all(&coll_dir)?;
        let sqlite_path = coll_dir.join("metadata.sqlite");
        let tantivy_dir = coll_dir.join("tantivy");
        fs::create_dir_all(&tantivy_dir)?;

        let sqlite = Connection::open(&sqlite_path)?;
        // FIX-1 (PR #24 reviewer): rusqlite 默认 PRAGMA foreign_keys=OFF — 显式打开让
        // provenance 的 FOREIGN KEY ... ON DELETE CASCADE 生效，无需手动级联清理.
        sqlite.execute_batch("PRAGMA foreign_keys = ON;")?;
        sqlite.execute_batch(SQL_SCHEMA)?;

        let (schema, fields) = build_tantivy_schema();
        // 用 meta.json 存在与否判断 — tantivy::Index::exists 需要 Directory trait + 错误转换繁琐
        let meta = tantivy_dir.join("meta.json");
        let index = if meta.exists() {
            Index::open_in_dir(&tantivy_dir)?
        } else {
            Index::create_in_dir(&tantivy_dir, schema.clone())?
        };
        let writer: IndexWriter = index.writer(TANTIVY_WRITER_BUDGET)?;

        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            collection_id: collection_id.to_string(),
            sqlite,
            tantivy_index: index,
            tantivy_writer: Mutex::new(writer),
            f_chunk_id: fields[0],
            f_content: fields[1],
            f_file_path: fields[2],
            f_language: fields[3],
            f_line_start: fields[4],
            f_line_end: fields[5],
        })
    }

    /// 全量索引：scan root → for ScannedFile → parse → chunk → 写双存储。
    pub fn index_path(
        &mut self,
        root: &Path,
        scan_options: &ScanOptions,
        policy: &ChunkPolicy,
        provenance: Vec<Provenance>,
    ) -> Result<IndexStats, IndexError> {
        let report = scan_path(root, scan_options).map_err(|e| IndexError::Scan(e.to_string()))?;
        let mut stats = IndexStats::default();
        stats.files_skipped_denied = report.skipped.len();

        for sf in &report.files {
            // BINDING (task-3.1 §10 Waiver): consume only redacted_content; original
            // secrets must not enter the index. Scanner sets redacted_content when any
            // redaction happened; otherwise content holds the (unredacted, secret-free) source.
            let body: &str = match (sf.redacted_content.as_ref(), sf.content.as_ref()) {
                (Some(r), _) => r.as_str(),
                (None, Some(c)) => c.as_str(),
                (None, None) => {
                    stats.files_skipped_redaction += 1;
                    continue;
                }
            };

            let chunks = self.parse_and_chunk(&sf.path, body, policy, &provenance)?;
            if chunks.is_empty() {
                continue;
            }

            self.write_chunks(&sf.path, body, &chunks)?;
            stats.chunks_written += chunks.len();
            stats.files_indexed += 1;
        }

        Ok(stats)
    }

    /// 增量：单文件 partial reindex（AC4）— 比对 files.content_hash → 不同则删旧 chunks 重插.
    pub fn reindex_file(
        &mut self,
        path: &Path,
        scan_options: &ScanOptions,
        policy: &ChunkPolicy,
        provenance: Vec<Provenance>,
    ) -> Result<IndexStats, IndexError> {
        let sf = scan_file(path, scan_options).map_err(|e| IndexError::Scan(e.to_string()))?;
        let mut stats = IndexStats::default();

        let body: &str = match (sf.redacted_content.as_ref(), sf.content.as_ref()) {
            (Some(r), _) => r.as_str(),
            (None, Some(c)) => c.as_str(),
            (None, None) => {
                stats.files_skipped_redaction = 1;
                return Ok(stats);
            }
        };

        let new_file_hash = content_hash(body);
        let file_path_str = sf.path.to_string_lossy().to_string();

        let prev: Option<String> = self
            .sqlite
            .query_row(
                "SELECT content_hash FROM files WHERE file_path = ?1",
                params![&file_path_str],
                |row| row.get(0),
            )
            .ok();

        if prev.as_deref() == Some(&new_file_hash) {
            // No-op: file unchanged
            return Ok(stats);
        }

        // 删旧 chunks（SQLite + Tantivy）
        let deleted = self.delete_chunks_for_file(&file_path_str)?;
        stats.chunks_deleted = deleted;

        // 重插新 chunks
        let chunks = self.parse_and_chunk(&sf.path, body, policy, &provenance)?;
        if !chunks.is_empty() {
            self.write_chunks(&sf.path, body, &chunks)?;
        }
        stats.chunks_written = chunks.len();
        stats.chunks_updated = chunks.len(); // 视作 update (deleted + written)
        stats.files_indexed = 1;
        Ok(stats)
    }

    /// 提交 Tantivy IndexWriter pending writes + reload reader 让搜索看到最新数据.
    pub fn commit(&mut self) -> Result<(), IndexError> {
        let mut w = self
            .tantivy_writer
            .lock()
            .map_err(|e| IndexError::Tantivy(format!("writer lock poisoned: {}", e)))?;
        w.commit()?;
        Ok(())
    }

    /// SQLite chunks 表行数（AC2 testing helper）.
    pub fn sqlite_chunk_count(&self) -> Result<u64, IndexError> {
        let count: i64 = self
            .sqlite
            .query_row("SELECT COUNT(*) FROM chunks", [], |row| row.get(0))?;
        Ok(count.max(0) as u64)
    }

    /// Tantivy 全文查询（AC2 testing helper）；返回命中 chunk_id 列表.
    pub fn tantivy_search(&self, query: &str, limit: usize) -> Result<Vec<String>, IndexError> {
        let reader = self
            .tantivy_index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        let searcher = reader.searcher();
        let qp = QueryParser::for_index(&self.tantivy_index, vec![self.f_content]);
        let parsed = qp.parse_query(query)?;
        // tantivy 0.26: TopDocs::with_limit().order_by_score() → impl Collector
        let collector = TopDocs::with_limit(limit).order_by_score();
        let top = searcher.search(&parsed, &collector)?;
        let mut out = Vec::with_capacity(top.len());
        for (_score, addr) in top {
            let doc: TantivyDocument = searcher.doc(addr)?;
            if let Some(v) = doc.get_first(self.f_chunk_id) {
                if let Some(s) = v.as_str() {
                    out.push(s.to_string());
                }
            }
        }
        Ok(out)
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
    pub fn collection_id(&self) -> &str {
        &self.collection_id
    }

    // ---- 私有 helpers ----

    fn parse_and_chunk(
        &self,
        path: &Path,
        body: &str,
        policy: &ChunkPolicy,
        provenance: &[Provenance],
    ) -> Result<Vec<Chunk>, IndexError> {
        let hint = lang_hint_from_path(path);
        let units = parse_content(path, body, hint).map_err(|e| IndexError::Parse(e.to_string()))?;
        let chunks = chunk_units(&units, path, policy, provenance.to_vec())
            .map_err(|e| IndexError::Chunk(e.to_string()))?;
        Ok(chunks)
    }

    fn write_chunks(&mut self, path: &Path, body: &str, chunks: &[Chunk]) -> Result<(), IndexError> {
        let file_path_str = path.to_string_lossy().to_string();
        let file_hash = content_hash(body);
        let mtime_unix: i64 = fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let now_iso = indexed_at_now_str();

        let tx = self.sqlite.unchecked_transaction()?;
        // chunks insert (use INSERT OR REPLACE to make incremental retries idempotent)
        for c in chunks {
            tx.execute(
                "INSERT OR REPLACE INTO chunks
                    (chunk_id, file_path, line_start, line_end, language, content,
                     content_hash, kind, collection_id, indexed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    c.chunk_id,
                    file_path_str,
                    c.line_start as i64,
                    c.line_end as i64,
                    c.language,
                    c.content,
                    c.content_hash,
                    c.kind,
                    self.collection_id,
                    now_iso,
                ],
            )?;
            // provenance rows (CASCADE delete via FK when chunks deleted)
            for p in &c.provenance {
                tx.execute(
                    "INSERT INTO provenance
                        (chunk_id, importer, original_path, imported_at, source_modified_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        c.chunk_id,
                        p.importer,
                        p.original_path,
                        p.imported_at,
                        if p.source_modified_at.is_empty() {
                            None
                        } else {
                            Some(p.source_modified_at.clone())
                        },
                    ],
                )?;
            }
        }
        // files (AC4 锚点 — partial reindex 用)
        tx.execute(
            "INSERT OR REPLACE INTO files (file_path, content_hash, mtime_unix, indexed_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![file_path_str, file_hash, mtime_unix, now_iso],
        )?;
        tx.commit()?;

        // Tantivy add_document (commit 延迟到 self.commit())
        let writer = self
            .tantivy_writer
            .lock()
            .map_err(|e| IndexError::Tantivy(format!("writer lock poisoned: {}", e)))?;
        for c in chunks {
            writer.add_document(doc!(
                self.f_chunk_id => c.chunk_id.clone(),
                self.f_content => c.content.clone(),
                self.f_file_path => file_path_str.clone(),
                self.f_language => c.language.clone(),
                self.f_line_start => c.line_start as i64,
                self.f_line_end => c.line_end as i64,
            ))?;
        }
        Ok(())
    }

    /// 删某 file_path 的所有 chunks（SQLite + Tantivy）。返回删除的 SQLite 行数.
    ///
    /// FIX-1 (PR #24 reviewer): IndexSession::open 现在 `PRAGMA foreign_keys=ON`，
    /// provenance 通过 `FOREIGN KEY ... ON DELETE CASCADE` 由 SQLite 自动级联清理.
    fn delete_chunks_for_file(&mut self, file_path: &str) -> Result<usize, IndexError> {
        let n: usize = self.sqlite.execute(
            "DELETE FROM chunks WHERE file_path = ?1",
            params![file_path],
        )?;
        // Tantivy delete by file_path term
        let writer = self
            .tantivy_writer
            .lock()
            .map_err(|e| IndexError::Tantivy(format!("writer lock poisoned: {}", e)))?;
        let term = Term::from_field_text(self.f_file_path, file_path);
        writer.delete_term(term);
        Ok(n)
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

        let count = sess.sqlite_chunk_count().expect("sqlite count");
        assert!(count >= 1, "AC2: SQLite chunks 表应有 ≥1 行, got {}", count);

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

        fs::write(
            src_root.join("notes.md"),
            "# Notes\nordinarytokenz8q4 body\n",
        )
        .unwrap();
        fs::write(src_root.join(".env"), "API_KEY=plain_secret\n").unwrap();
        fs::write(
            src_root.join("config.md"),
            "# Config\nAWS key: AKIAIOSFODNN7EXAMPLE referenced here\n",
        )
        .unwrap();

        let mut sess = IndexSession::open(&data_dir, "test-coll").unwrap();
        sess.index_path(&src_root, &make_scan_options(), &ChunkPolicy::default(), vec![])
            .unwrap();
        sess.commit().unwrap();

        let normal_hits = sess.tantivy_search("ordinarytokenz8q4", 10).expect("tantivy");
        assert!(
            !normal_hits.is_empty(),
            "AC3 sanity: 正常 token 应被命中"
        );

        let env_hits = sess.tantivy_search("plain_secret", 10).expect("tantivy");
        assert!(
            env_hits.is_empty(),
            "AC3 denylist: .env 内容不应入索引, 但命中 {} 个",
            env_hits.len()
        );

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

        fs::write(&fp, "# Doc v2\nnewtokenq8r9s0\n").unwrap();

        let stats_inc = sess
            .reindex_file(&fp, &make_scan_options(), &ChunkPolicy::default(), vec![])
            .unwrap();
        sess.commit().unwrap();

        let changed = stats_inc.chunks_updated > 0
            || (stats_inc.chunks_deleted > 0 && stats_inc.chunks_written > 0);
        assert!(
            changed,
            "AC4: reindex_file 应有 chunks 变动, got stats={:?}",
            stats_inc
        );

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
