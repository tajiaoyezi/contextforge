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
use tantivy::schema::{
    Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions, Value, FAST, INDEXED, STORED,
    STRING, TEXT,
};
use tantivy::tokenizer::{LowerCaser, RemoveLongFilter, TextAnalyzer, Token, TokenStream, Tokenizer};
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

/// task-9.2 §5.3: per-file progress snapshot 喂给 `index_path_with_progress` 回调。
/// 独立于 `crate::pb::IndexProgress`（保 indexer 模块不依赖 proto package）。
/// 调用方（如 `server.rs::CoreService::index`）按需 map 到 proto 消息。
#[derive(Debug, Clone, Copy)]
pub struct IndexProgressSnapshot<'a> {
    pub files_processed: usize,
    pub files_skipped_denied: usize,
    pub files_skipped_redaction: usize,
    pub chunks_written: usize,
    pub current_file: Option<&'a Path>,
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

fn build_tantivy_schema(tokenizer_name: &str) -> (Schema, [Field; 6]) {
    let mut sb = Schema::builder();
    let chunk_id = sb.add_text_field("chunk_id", STRING | STORED);
    // task-24.1：opt-in 时 content 绑自定义 code/CJK analyzer；其余（含 "default"）维持
    // `TEXT | STORED`（向后兼容）。opt-in 的索引选项与 `TEXT` 等价（WithFreqsAndPositions
    // + fieldnorms + stored），仅 tokenizer 名不同 → 不改字段集 / 字段类型，只改 analyzer 绑定。
    let content = if tokenizer_name == CODE_CJK_TOKENIZER {
        let indexing = TextFieldIndexing::default()
            .set_tokenizer(tokenizer_name)
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let opts = TextOptions::default().set_indexing_options(indexing).set_stored();
        sb.add_text_field("content", opts)
    } else {
        sb.add_text_field("content", TEXT | STORED)
    };
    let file_path = sb.add_text_field("file_path", STRING | STORED);
    let language = sb.add_text_field("language", STRING | STORED);
    let line_start = sb.add_i64_field("line_start", STORED | INDEXED | FAST);
    let line_end = sb.add_i64_field("line_end", STORED | INDEXED | FAST);
    (
        sb.build(),
        [chunk_id, content, file_path, language, line_start, line_end],
    )
}

// ---- task-24.1: 自定义 code/CJK aware tokenizer ----
//
// 设计（spec §5.2）：自定义 `TextAnalyzer` = CodeCjkTokenizer（代码符号拆分 + 保留原 token
// + CJK bigram）+ RemoveLongFilter(40) + LowerCaser（与 Tantivy "default" 的 filter 链一致，
// 仅替换底层 tokenizer）。opt-in 时绑 `content`，默认时维持 `TEXT`。0 新 dep（纯 std）。

/// Tantivy 默认 analyzer 名（`TEXT` 字段恒用此名）— opt-in 之外维持现状.
pub(crate) const DEFAULT_TOKENIZER: &str = "default";
/// 自定义 code/CJK analyzer 注册名 — opt-in 时 `content` 字段绑此名.
pub(crate) const CODE_CJK_TOKENIZER: &str = "code_cjk";

/// 判定 char 是否属于按 bigram 切分的 CJK / CJK-adjacent 表意区段（无空格分隔的脚本）.
fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3400..=0x4DBF      // CJK Unified Ideographs Ext A
        | 0x4E00..=0x9FFF    // CJK Unified Ideographs
        | 0xF900..=0xFAFF    // CJK Compatibility Ideographs
        | 0x3040..=0x309F    // Hiragana
        | 0x30A0..=0x30FF    // Katakana
        | 0xAC00..=0xD7AF    // Hangul Syllables
        | 0x20000..=0x2A6DF  // CJK Unified Ideographs Ext B
    )
}

/// 代码标识符 char：字母数字（非 CJK）或子词分隔符 `_ . -`.
fn is_code_char(c: char) -> bool {
    (c.is_alphanumeric() && !is_cjk(c)) || matches!(c, '_' | '.' | '-')
}

fn push_token(out: &mut Vec<Token>, pos: &mut usize, text: &str, from: usize, to: usize) {
    out.push(Token {
        offset_from: from,
        offset_to: to,
        position: *pos,
        text: text.to_string(),
        position_length: 1,
    });
    *pos += 1;
}

/// delimiter-free 段按 camelCase 边界切分，返回段内相对 byte range 列表.
/// 边界：lower/digit→upper（`camelCase`）；acronym 尾（`HTMLParser`→`HTML`/`Parser`）。
fn camel_ranges(word: &str) -> Vec<(usize, usize)> {
    let cv: Vec<(usize, char)> = word.char_indices().collect();
    let n = cv.len();
    let mut ranges = Vec::new();
    if n == 0 {
        return ranges;
    }
    let mut start = 0usize; // index into cv
    for i in 1..n {
        let prev = cv[i - 1].1;
        let cur = cv[i].1;
        let boundary = (!prev.is_uppercase() && cur.is_uppercase())
            || (prev.is_uppercase()
                && cur.is_uppercase()
                && i + 1 < n
                && cv[i + 1].1.is_lowercase());
        if boundary {
            ranges.push((cv[start].0, cv[i].0));
            start = i;
        }
    }
    ranges.push((cv[start].0, word.len()));
    ranges
}

/// 在某 delimiter-free word 上拆 camelCase 子词，push 到 out（跳过空 / 与整段相同的子词）.
fn emit_word_subwords(
    word: &str,
    word_base: usize,
    seg: &str,
    pos: &mut usize,
    out: &mut Vec<Token>,
) {
    for (cs, ce) in camel_ranges(word) {
        let sub = &word[cs..ce];
        if sub.is_empty() || sub == seg {
            continue;
        }
        push_token(out, pos, sub, word_base + cs, word_base + ce);
    }
}

/// 一个 code 段（alnum + `_ . -`）：保留原 token + 拆 `_ . -` + 拆 camelCase 子词.
fn emit_code_segment(seg: &str, base: usize, pos: &mut usize, out: &mut Vec<Token>) {
    if !seg.chars().any(|c| c.is_alphanumeric()) {
        return; // 纯分隔符段（如 "..."）不产 token
    }
    // 保留原 token（整段，原样查询不退化）
    push_token(out, pos, seg, base, base + seg.len());
    // 拆 `_ . -` → word，再拆 camelCase
    let mut word_start: Option<usize> = None;
    for (b, c) in seg.char_indices() {
        if matches!(c, '_' | '.' | '-') {
            if let Some(ws) = word_start.take() {
                emit_word_subwords(&seg[ws..b], base + ws, seg, pos, out);
            }
        } else if word_start.is_none() {
            word_start = Some(b);
        }
    }
    if let Some(ws) = word_start.take() {
        emit_word_subwords(&seg[ws..], base + ws, seg, pos, out);
    }
}

/// 把输入文本切成 code/CJK 感知的 token 序（确定性；offset/position 真实可用于 phrase 查询）.
fn tokenize_code_cjk(text: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    let cv: Vec<(usize, char)> = text.char_indices().collect();
    let n = cv.len();
    let mut i = 0usize;
    while i < n {
        let (bstart, c) = cv[i];
        if is_cjk(c) {
            // CJK run → bigram（单字 → unigram）
            let mut j = i;
            while j < n && is_cjk(cv[j].1) {
                j += 1;
            }
            let run = &cv[i..j];
            if run.len() == 1 {
                let (bs, ch) = run[0];
                push_token(&mut out, &mut pos, &ch.to_string(), bs, bs + ch.len_utf8());
            } else {
                for k in 0..run.len() - 1 {
                    let (bs, ch0) = run[k];
                    let (b1, ch1) = run[k + 1];
                    let s: String = [ch0, ch1].iter().collect();
                    push_token(&mut out, &mut pos, &s, bs, b1 + ch1.len_utf8());
                }
            }
            i = j;
        } else if is_code_char(c) {
            let mut j = i;
            while j < n && is_code_char(cv[j].1) {
                j += 1;
            }
            let seg_end = if j < n { cv[j].0 } else { text.len() };
            emit_code_segment(&text[bstart..seg_end], bstart, &mut pos, &mut out);
            i = j;
        } else {
            i += 1; // 分隔符（空白 / 标点）跳过
        }
    }
    out
}

/// 自定义 code/CJK tokenizer（产 owned `Vec<Token>`，token stream 不借用输入文本）.
#[derive(Clone, Default)]
pub(crate) struct CodeCjkTokenizer;

pub(crate) struct CodeCjkTokenStream {
    tokens: Vec<Token>,
    idx: usize,
    token: Token,
}

impl Tokenizer for CodeCjkTokenizer {
    type TokenStream<'a> = CodeCjkTokenStream;
    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        CodeCjkTokenStream {
            tokens: tokenize_code_cjk(text),
            idx: 0,
            token: Token::default(),
        }
    }
}

impl TokenStream for CodeCjkTokenStream {
    fn advance(&mut self) -> bool {
        if self.idx < self.tokens.len() {
            self.token = self.tokens[self.idx].clone();
            self.idx += 1;
            true
        } else {
            false
        }
    }
    fn token(&self) -> &Token {
        &self.token
    }
    fn token_mut(&mut self) -> &mut Token {
        &mut self.token
    }
}

/// 构建自定义 code/CJK `TextAnalyzer`（CodeCjkTokenizer + RemoveLongFilter(40) + LowerCaser）.
pub(crate) fn build_code_cjk_analyzer() -> TextAnalyzer {
    TextAnalyzer::builder(CodeCjkTokenizer)
        .filter(RemoveLongFilter::limit(40))
        .filter(LowerCaser)
        .build()
}

/// 在 index 的 tokenizer manager 上注册 code/CJK analyzer（名 = `CODE_CJK_TOKENIZER`）.
/// 默认模式下无字段引用此名 → 无副作用；opt-in 模式下保 index/query 分词对称.
pub(crate) fn register_code_cjk(index: &Index) {
    index
        .tokenizers()
        .register(CODE_CJK_TOKENIZER, build_code_cjk_analyzer());
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
    /// 打开（或创建）默认 tokenizer 的索引会话（向后兼容入口）.
    pub fn open(data_dir: &Path, collection_id: &str) -> Result<Self, IndexError> {
        Self::open_with_tokenizer(data_dir, collection_id, DEFAULT_TOKENIZER)
    }

    /// 打开（或创建）索引会话；建 SQLite schema + 打开/创建 Tantivy index + 持久化 IndexWriter.
    ///
    /// task-24.1：`tokenizer == CODE_CJK_TOKENIZER` 时**新建** collection 的 `content` 字段绑
    /// 自定义 code/CJK analyzer（opt-in）；其余值（含 `"default"`）维持 `TEXT` 默认 analyzer
    /// （向后兼容，既有索引不失效）。opt-in 改倒排词项 → 既有 collection 须 **re-index** 才生效
    /// （旧索引仍可用默认 analyzer 检索，但不享受代码/CJK 子词命中）。
    pub fn open_with_tokenizer(
        data_dir: &Path,
        collection_id: &str,
        tokenizer: &str,
    ) -> Result<Self, IndexError> {
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

        let (schema, fields) = build_tantivy_schema(tokenizer);
        // 用 meta.json 存在与否判断 — tantivy::Index::exists 需要 Directory trait + 错误转换繁琐
        let meta = tantivy_dir.join("meta.json");
        let index = if meta.exists() {
            Index::open_in_dir(&tantivy_dir)?
        } else {
            Index::create_in_dir(&tantivy_dir, schema.clone())?
        };
        // index/query 对称：注册 code/CJK analyzer（默认模式下无字段引用 → 无副作用）.
        register_code_cjk(&index);
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

    /// 全量索引（task-2.4 历史 API）：thin wrapper 调 `index_path_with_progress`
    /// 传 no-op callback；签名 + 行为不变以保 task-2.4 现有调用方（phase2_smoke /
    /// phase6_smoke / server.rs test fixture）零回归。
    pub fn index_path(
        &mut self,
        root: &Path,
        scan_options: &ScanOptions,
        policy: &ChunkPolicy,
        provenance: Vec<Provenance>,
    ) -> Result<IndexStats, IndexError> {
        self.index_path_with_progress(root, scan_options, policy, provenance, |_| {})
    }

    /// task-9.2 §5.3：全量索引带 per-file progress 回调。
    ///
    /// 回调时机：每处理完一个 ScannedFile（含 skip-redaction / empty-chunks 情况）
    /// 触发一次；调用方（如 `server.rs::CoreService::index`）决定何时 emit proto
    /// `IndexProgress`（初始 / 终态由 caller 控制）。回调内 `current_file` 持当前
    /// 处理文件路径；累计计数随处理推进。
    pub fn index_path_with_progress<F>(
        &mut self,
        root: &Path,
        scan_options: &ScanOptions,
        policy: &ChunkPolicy,
        provenance: Vec<Provenance>,
        mut on_progress: F,
    ) -> Result<IndexStats, IndexError>
    where
        F: FnMut(&IndexProgressSnapshot<'_>),
    {
        let report = scan_path(root, scan_options).map_err(|e| IndexError::Scan(e.to_string()))?;
        let mut stats = IndexStats {
            files_skipped_denied: report.skipped.len(),
            ..Default::default()
        };

        for sf in &report.files {
            // BINDING (task-3.1 §10 Waiver): consume only redacted_content; original
            // secrets must not enter the index. Scanner sets redacted_content when any
            // redaction happened; otherwise content holds the (unredacted, secret-free) source.
            let body: &str = match (sf.redacted_content.as_ref(), sf.content.as_ref()) {
                (Some(r), _) => r.as_str(),
                (None, Some(c)) => c.as_str(),
                (None, None) => {
                    stats.files_skipped_redaction += 1;
                    on_progress(&IndexProgressSnapshot {
                        files_processed: stats.files_indexed,
                        files_skipped_denied: stats.files_skipped_denied,
                        files_skipped_redaction: stats.files_skipped_redaction,
                        chunks_written: stats.chunks_written,
                        current_file: Some(sf.path.as_path()),
                    });
                    continue;
                }
            };

            let chunks = self.parse_and_chunk(&sf.path, body, policy, &provenance)?;
            if !chunks.is_empty() {
                self.write_chunks(&sf.path, body, &chunks)?;
                stats.chunks_written += chunks.len();
                stats.files_indexed += 1;
            }

            on_progress(&IndexProgressSnapshot {
                files_processed: stats.files_indexed,
                files_skipped_denied: stats.files_skipped_denied,
                files_skipped_redaction: stats.files_skipped_redaction,
                chunks_written: stats.chunks_written,
                current_file: Some(sf.path.as_path()),
            });
        }

        Ok(stats)
    }

    /// task-11.3 (Phase 11, ADR-016 D3): co-operative cancellable variant of
    /// `index_path_with_progress`. Identical semantics + an extra `cancel_token`
    /// that is checked at file boundaries (between each scanned file). When
    /// `cancel_token.load(Ordering::Relaxed) == true`, the iteration breaks
    /// after the current file finishes its chunk write, and the returned
    /// `IndexStats` reflects the partial work done.
    ///
    /// The caller (`JobRunner` via `IndexerBackend`) uses this to honor the
    /// task-11.3 §6 AC3 contract: POST `/v1/index-jobs/<id>/cancel` → ≤5s
    /// `status=cancelled`.
    ///
    /// API extension justification (task-11.3 §10 trade-off T1): the existing
    /// `index_path_with_progress` callback signature is `FnMut(&Snapshot) ->
    /// ()` and cannot signal a break upstream. Rather than break source
    /// compatibility (would require all callers to be updated), this method is
    /// add-only: existing callers (`server.rs::CoreService::index` from
    /// Phase 9) are unaffected.
    pub fn index_path_cancellable<F>(
        &mut self,
        root: &Path,
        scan_options: &ScanOptions,
        policy: &ChunkPolicy,
        provenance: Vec<Provenance>,
        mut on_progress: F,
        cancel_token: &std::sync::atomic::AtomicBool,
    ) -> Result<(IndexStats, bool), IndexError>
    where
        F: FnMut(&IndexProgressSnapshot<'_>),
    {
        let report = scan_path(root, scan_options).map_err(|e| IndexError::Scan(e.to_string()))?;
        let mut stats = IndexStats {
            files_skipped_denied: report.skipped.len(),
            ..Default::default()
        };

        for sf in &report.files {
            if cancel_token.load(std::sync::atomic::Ordering::Relaxed) {
                return Ok((stats, true));
            }
            let body: &str = match (sf.redacted_content.as_ref(), sf.content.as_ref()) {
                (Some(r), _) => r.as_str(),
                (None, Some(c)) => c.as_str(),
                (None, None) => {
                    stats.files_skipped_redaction += 1;
                    on_progress(&IndexProgressSnapshot {
                        files_processed: stats.files_indexed,
                        files_skipped_denied: stats.files_skipped_denied,
                        files_skipped_redaction: stats.files_skipped_redaction,
                        chunks_written: stats.chunks_written,
                        current_file: Some(sf.path.as_path()),
                    });
                    continue;
                }
            };

            let chunks = self.parse_and_chunk(&sf.path, body, policy, &provenance)?;
            if !chunks.is_empty() {
                self.write_chunks(&sf.path, body, &chunks)?;
                stats.chunks_written += chunks.len();
                stats.files_indexed += 1;
            }

            on_progress(&IndexProgressSnapshot {
                files_processed: stats.files_indexed,
                files_skipped_denied: stats.files_skipped_denied,
                files_skipped_redaction: stats.files_skipped_redaction,
                chunks_written: stats.chunks_written,
                current_file: Some(sf.path.as_path()),
            });
        }

        Ok((stats, false))
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

    // ---- task-24.1: code/CJK tokenizer tests ----

    fn tok_texts(analyzer: &mut tantivy::tokenizer::TextAnalyzer, s: &str) -> Vec<String> {
        use tantivy::tokenizer::TokenStream;
        let mut ts = analyzer.token_stream(s);
        let mut out = Vec::new();
        while ts.advance() {
            out.push(ts.token().text.clone());
        }
        out
    }

    fn content_tokenizer_name(schema: &tantivy::schema::Schema) -> String {
        use tantivy::schema::FieldType;
        let f = schema.get_field("content").unwrap();
        match schema.get_field_entry(f).field_type() {
            FieldType::Str(opts) => opts
                .get_indexing_options()
                .map(|i| i.tokenizer().to_string())
                .unwrap_or_default(),
            _ => String::new(),
        }
    }

    fn write_one_doc(sess: &mut IndexSession, body: &str) {
        let src = temp_root("tok-src");
        fs::write(src.join("doc.md"), format!("# Doc\n{}\n", body)).unwrap();
        sess.index_path(&src, &make_scan_options(), &ChunkPolicy::default(), vec![])
            .unwrap();
    }

    // ---- TEST-24.1.1 (AC1) — 代码符号拆分 + 保留原 token ----
    #[test]
    fn test_24_1_1_code_symbol_split_preserves_original() {
        let mut a = build_code_cjk_analyzer();
        assert_eq!(tok_texts(&mut a, "camelCase"), vec!["camelcase", "camel", "case"]);
        assert_eq!(
            tok_texts(&mut a, "getUserById"),
            vec!["getuserbyid", "get", "user", "by", "id"]
        );
        assert_eq!(tok_texts(&mut a, "user_id"), vec!["user_id", "user", "id"]);
        assert_eq!(
            tok_texts(&mut a, "pkg.module.func"),
            vec!["pkg.module.func", "pkg", "module", "func"]
        );
        assert_eq!(
            tok_texts(&mut a, "kebab-case-name"),
            vec!["kebab-case-name", "kebab", "case", "name"]
        );
        // 单词无分隔/无 camel 边界 → 不产生重复子 token
        assert_eq!(tok_texts(&mut a, "config"), vec!["config"]);
    }

    // ---- TEST-24.1.2 (AC2) — CJK bigram + 混合输入 ----
    #[test]
    fn test_24_1_2_cjk_bigram_and_mixed() {
        let mut a = build_code_cjk_analyzer();
        assert_eq!(tok_texts(&mut a, "配置加载"), vec!["配置", "置加", "加载"]);
        // 混合：非 CJK 段走代码符号切分，CJK 段走 bigram
        assert_eq!(
            tok_texts(&mut a, "getConfig配置"),
            vec!["getconfig", "get", "config", "配置"]
        );
        // 单 CJK 字 → unigram
        assert_eq!(tok_texts(&mut a, "中"), vec!["中"]);
    }

    // ---- TEST-24.1.3 (AC3) — 默认 tokenization 不变 + schema 字段集不变 ----
    #[test]
    fn test_24_1_3_default_tokenization_unchanged() {
        use tantivy::tokenizer::TokenizerManager;
        // 默认 analyzer：camelCase 整体一个（小写）token，CJK 连续段整体一个 token
        let mut def = TokenizerManager::default()
            .get(DEFAULT_TOKENIZER)
            .expect("default analyzer");
        assert_eq!(tok_texts(&mut def, "getUserById"), vec!["getuserbyid"]);
        assert_eq!(tok_texts(&mut def, "配置加载"), vec!["配置加载"]);

        // 默认模式 IndexSession：content 字段 tokenizer == "default"，6 字段 schema 不变
        let data_dir = temp_root("tok-default");
        let sess = IndexSession::open(&data_dir, "c").unwrap();
        let schema = sess.tantivy_index.schema();
        for f in ["chunk_id", "content", "file_path", "language", "line_start", "line_end"] {
            assert!(schema.get_field(f).is_ok(), "schema 应含字段 {f}");
        }
        assert_eq!(content_tokenizer_name(&schema), DEFAULT_TOKENIZER);
    }

    // ---- TEST-24.1.4 (AC4) — index/query 对称 + opt-in 子词命中（默认 miss）----
    #[test]
    fn test_24_1_4_optin_subword_and_cjk_hit_roundtrip() {
        // opt-in：camelCase 拆词 → "user" 命中；CJK bigram → "置加" 命中
        let data_dir = temp_root("tok-optin");
        let mut sess =
            IndexSession::open_with_tokenizer(&data_dir, "c", CODE_CJK_TOKENIZER).unwrap();
        write_one_doc(&mut sess, "fn getUserById() 配置加载");
        sess.commit().unwrap();
        let schema = sess.tantivy_index.schema();
        assert_eq!(content_tokenizer_name(&schema), CODE_CJK_TOKENIZER);
        assert!(
            !sess.tantivy_search("user", 10).unwrap().is_empty(),
            "opt-in: camel 子词 'user' 应命中"
        );
        assert!(
            !sess.tantivy_search("置加", 10).unwrap().is_empty(),
            "opt-in: CJK bigram '置加' 应命中"
        );

        // 默认模式：同内容 → camel 不拆、CJK 整体 → 子词 miss（向后兼容基线）
        let data_dir2 = temp_root("tok-default-miss");
        let mut sess2 = IndexSession::open(&data_dir2, "c").unwrap();
        write_one_doc(&mut sess2, "fn getUserById() 配置加载");
        sess2.commit().unwrap();
        assert!(
            sess2.tantivy_search("user", 10).unwrap().is_empty(),
            "default: camel 子词 'user' 不应命中"
        );
        assert!(
            sess2.tantivy_search("置加", 10).unwrap().is_empty(),
            "default: CJK bigram '置加' 不应命中"
        );
    }
}
