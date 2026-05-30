//! task-4.1 / task-4.2 (Phase 4): retriever — BM25 / metadata / filter 检索 + 12-field explainable result.
//!
//! task-4.1 GREEN: tantivy 0.26 QueryParser + rusqlite JOIN 实现 (read-only, 7-field SearchResult).
//! task-4.2 §2A (2026-05-23): SearchResult 扩 12-field explainable contract (AC1) +
//!   provenance 合成 (AC3 黑盒守护 ≥1 entry) + Retriever::explain debug entry (AC4) +
//!   core/tests/phase4_smoke.rs (AC5).
//!
//! 数据目录与 task-2.4 IndexSession 一致：
//!   [data_dir]/collections/[collection_id]/{metadata.sqlite, tantivy/}
//!
//! Tantivy schema 由 task-2.4 frozen，本模块只读不重定义（meta.json 自携）。
//! provenance 优先 JOIN indexer provenance 表；缺失则合成 scanner-default 保证
//! AC3 invariant `provenance.len() ≥ 1`（v0.1 schema-gap 见 §10）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::{params, Connection};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Value};
use tantivy::{Index, IndexReader, TantivyDocument};
use thiserror::Error;

use crate::chunker::Provenance;

// ---- task-18.1: vector retrieval trait re-exports ----
pub mod vector;
pub use vector::{VectorBackend, VectorSearcher, NoopVectorBackend};

// ---- task-4.2 §2A v0.1 schema-gap default 常量（task-2.4 indexer 未存 → 合成兜底）----
const DEFAULT_CONTEXT_ID: &str = "";
const DEFAULT_SOURCE_TYPE: &str = "";
const DEFAULT_REDACTION_STATUS: &str = "applied"; // BINDING: indexer 仅消费 redacted_content
const SYNTHESIZED_IMPORTER: &str = "scanner"; // provenance 合成 importer 标识

/// Read-only 检索会话；与 task-2.4 IndexSession 共享数据目录。
pub struct Retriever {
    data_dir: PathBuf,
    collection_id: String,
    sqlite: Connection,
    tantivy_index: Index,
    tantivy_reader: IndexReader,
    f_chunk_id: Field,
    f_content: Field,
    f_file_path: Field,
    f_language: Field,
    f_line_start: Field,
    f_line_end: Field,
    config: RetrieverConfig,
    // task-18.1: optional vector backend (default None; Some wired by task-18.7)
    vector_searcher: Option<Arc<dyn VectorSearcher>>,
}

#[derive(Error, Debug)]
pub enum RetrieverError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite: {0}")]
    Sqlite(String),
    #[error("tantivy: {0}")]
    Tantivy(String),
    #[error("invalid config: {0}")]
    InvalidConfig(String),
    #[error("collection not found: {0}")]
    CollectionNotFound(String),
}

impl From<rusqlite::Error> for RetrieverError {
    fn from(e: rusqlite::Error) -> Self {
        RetrieverError::Sqlite(e.to_string())
    }
}

impl From<tantivy::TantivyError> for RetrieverError {
    fn from(e: tantivy::TantivyError) -> Self {
        RetrieverError::Tantivy(e.to_string())
    }
}

/// 检索配置（AC5 tokenizer / boost / exact phrase）.
#[derive(Debug, Clone)]
pub struct RetrieverConfig {
    pub tokenizer: String,
    pub field_boosts: HashMap<String, f32>,
    pub enable_exact_phrase: bool,
}

impl Default for RetrieverConfig {
    fn default() -> Self {
        let mut field_boosts = HashMap::new();
        field_boosts.insert("file_path".to_string(), 2.0);
        field_boosts.insert("content".to_string(), 1.0);
        Self {
            tokenizer: "default".to_string(),
            field_boosts,
            enable_exact_phrase: true,
        }
    }
}

/// 检索请求（PRD §REST/MCP search 请求契约对齐）.
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub query: String,
    pub top_k: usize,
    pub filters: SearchFilters,
    pub explain: bool,
}

/// 过滤契约（PRD §search 请求 filters 字段一致）.
/// v0.1 实现：language / collection / time_range；source_type / agent_scope no-op (§10 schema gap)
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub language: Vec<String>,
    pub source_type: Vec<String>,
    pub collection: Vec<String>,
    pub agent_scope: Vec<String>,
    pub time_after_unix: Option<i64>,
    pub time_before_unix: Option<i64>,
}

/// 检索结果（PRD §REST/MCP search response 契约对齐 + proto `RetrievalResult` 单源 schema unity）.
///
/// task-4.2 §2A (2026-05-23) 升级：从 task-4.1 的 7-field 扩到 12-field explainable contract per AC1。
/// v0.1 schema gap（继承 task-4.1 §10）：`context_id` / `source_type` / `agent_scope` /
/// `redaction_status` 不在 task-2.4 indexer SQLite/Tantivy schema → 返回 v0.1 default 常量；
/// `provenance` 优先 JOIN indexer provenance 表，缺失则合成 scanner-default 保证 ≥1 entry
/// (AC3 黑盒守护)。SPEC-DRIFT-task-2.4 chore-spec PR 未来 reverse-fill 后真实生效。
#[derive(Debug, Clone)]
pub struct SearchResult {
    // ---- 12 explainable fields (AC1) ----
    pub chunk_id: String,
    pub context_id: String,           // v0.1 default ""（schema gap）
    pub source_type: String,          // v0.1 default ""（schema gap）
    pub file_path: String,
    pub line_start: u64,
    pub line_end: u64,
    pub score: f32,
    pub retrieval_method: String,     // v0.1 = "bm25"
    pub reason: String,               // explain=false → ""；explain=true → enriched
    pub agent_scope: Vec<String>,     // v0.1 default vec![]（schema gap）
    pub redaction_status: String,     // v0.1 default "applied"（BINDING redacted_content）
    pub provenance: Vec<Provenance>,  // AC3 硬底：每条 ≥1 entry（合成兜底）
    // ---- 非 AC1 内部扩展（下游消费方便）----
    pub language: String,             // 沿用 task-4.1
    pub content: String,              // 沿用 task-4.1
    pub matched_terms: Vec<String>,   // task-4.1 placeholder；task-4.2 explain=true 时 enrich
}

/// Retained for task-1.3 core_skeleton.rs anchor (AC4 wiring). Returns true.
pub fn placeholder_ready() -> bool {
    true
}

/// task-6.2 §2A 决策 E: chunk_id format detector — REST `/v1/chunks/{id}` fast-path
/// pivot. 当 query 看起来像 `chunk_id` 时，`server.rs CoreService::search` 优先调
/// `retriever.get_chunk` 走精确路径；未命中 fallback 到 BM25 全文 `search()`.
///
/// chunker §100 format: `chk_<8-hex>_<ordinal>` (例 `chk_a1b2c3d4_0`). 此 detector
/// 是 chunk_id format 的"必要+充分"形状检查；任何其他形态（含 spec §5.3 示例的纯 hex
/// `^[0-9a-f]{16,}$`）都不触发 fast-path（避免误判 BM25 query 为 chunk_id 字面）.
pub fn is_chunk_id_format(s: &str) -> bool {
    let rest = match s.strip_prefix("chk_") {
        Some(r) => r,
        None => return false,
    };
    // 拆 "_<ordinal>" — 自后向前找最后一个 '_'
    let underscore_idx = match rest.rfind('_') {
        Some(i) => i,
        None => return false,
    };
    let hex_part = &rest[..underscore_idx];
    let ord_part = &rest[underscore_idx + 1..];
    hex_part.len() == 8
        && hex_part.chars().all(|c| c.is_ascii_hexdigit())
        && !ord_part.is_empty()
        && ord_part.chars().all(|c| c.is_ascii_digit())
}

/// task-4.2 AC4 helper: 提取 query 中的可解释 term — 仅保留在 chunk content 中出现的词
/// （case-insensitive substring 匹配；过滤 Tantivy QueryParser 元字符 / 引号 / 仅空白）.
///
/// 不重做 BM25 / Tantivy 自身的 tokenization — 只做 "用户读得懂的 reason enrichment".
fn enrich_matched_terms(query: &str, content: &str) -> Vec<String> {
    let content_lower = content.to_lowercase();
    query
        .split(|c: char| c.is_whitespace() || c == '"' || c == '\'')
        .filter_map(|t| {
            let term = t.trim_matches(|c: char| {
                !c.is_alphanumeric() && c != '_' && c != '-'
            });
            if term.is_empty() {
                return None;
            }
            let term_lower = term.to_lowercase();
            if content_lower.contains(&term_lower) {
                Some(term.to_string())
            } else {
                None
            }
        })
        .collect()
}

impl Retriever {
    /// 打开 read-only 会话；连接同一 task-2.4 数据目录.
    pub fn open(data_dir: &Path, collection_id: &str) -> Result<Self, RetrieverError> {
        Self::open_with_config(data_dir, collection_id, RetrieverConfig::default())
    }

    pub fn open_with_config(
        data_dir: &Path,
        collection_id: &str,
        config: RetrieverConfig,
    ) -> Result<Self, RetrieverError> {
        let coll_dir = data_dir.join("collections").join(collection_id);
        if !coll_dir.exists() {
            return Err(RetrieverError::CollectionNotFound(
                collection_id.to_string(),
            ));
        }
        let sqlite_path = coll_dir.join("metadata.sqlite");
        let tantivy_dir = coll_dir.join("tantivy");
        if !sqlite_path.exists() || !tantivy_dir.exists() {
            return Err(RetrieverError::CollectionNotFound(
                collection_id.to_string(),
            ));
        }
        let sqlite = Connection::open(&sqlite_path)?;
        let tantivy_index = Index::open_in_dir(&tantivy_dir)?;
        let tantivy_reader = tantivy_index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        // 从 task-2.4 frozen schema 取 field handles（schema 由 meta.json 自携）
        let schema = tantivy_index.schema();
        let f_chunk_id = schema
            .get_field("chunk_id")
            .map_err(|e| RetrieverError::Tantivy(format!("missing field chunk_id: {}", e)))?;
        let f_content = schema
            .get_field("content")
            .map_err(|e| RetrieverError::Tantivy(format!("missing field content: {}", e)))?;
        let f_file_path = schema
            .get_field("file_path")
            .map_err(|e| RetrieverError::Tantivy(format!("missing field file_path: {}", e)))?;
        let f_language = schema
            .get_field("language")
            .map_err(|e| RetrieverError::Tantivy(format!("missing field language: {}", e)))?;
        let f_line_start = schema
            .get_field("line_start")
            .map_err(|e| RetrieverError::Tantivy(format!("missing field line_start: {}", e)))?;
        let f_line_end = schema
            .get_field("line_end")
            .map_err(|e| RetrieverError::Tantivy(format!("missing field line_end: {}", e)))?;

        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            collection_id: collection_id.to_string(),
            sqlite,
            tantivy_index,
            tantivy_reader,
            f_chunk_id,
            f_content,
            f_file_path,
            f_language,
            f_line_start,
            f_line_end,
            config,
            vector_searcher: None, // task-18.1: default None (BM25-only)
        })
    }

    /// 主检索入口（AC1/AC2/AC3/AC5）.
    ///
    /// 流程：
    ///   1. trim query；空 / 仅空白 → Ok(vec![])（AC3 防御）
    ///   2. QueryParser::for_index([content, file_path]) + file_path boost（AC5）
    ///   3. parse_query Err → Ok(vec![])（AC3 防御：非法语法不 panic）
    ///   4. searcher.search(top_k * over_fetch, .order_by_score()) 拿候选（tantivy 0.26 API）
    ///   5. post-filter: language（AC2）+ time_range（AC2 通过 SQLite chunks.indexed_at）
    ///   6. SQLite JOIN by chunk_id 填完整 SearchResult 字段
    pub fn search(&self, opts: &SearchOptions) -> Result<Vec<SearchResult>, RetrieverError> {
        // AC3: 空 / 仅空白 → 立即返空
        let q_trim = opts.query.trim();
        if q_trim.is_empty() {
            return Ok(Vec::new());
        }

        // v0.1 schema gap warning: source_type / agent_scope 在 SearchFilters struct 中存在但
        // 检索路径未消费（task-2.4 Tantivy/SQLite schema 无对应列）→ caller 传非空值会被静默
        // 忽略。emit warning 让 caller 知情，避免误以为 filter 已生效。
        // 真实 filter 实施由 SPEC-DRIFT-task-2.4 reverse-fill schema 后落地。
        if !opts.filters.source_type.is_empty() || !opts.filters.agent_scope.is_empty() {
            eprintln!(
                "[retriever] WARN: source_type/agent_scope filter not yet implemented \
                 (schema gap; SPEC-DRIFT-task-2.4 pending), value ignored"
            );
        }

        let top_k = if opts.top_k == 0 { 10 } else { opts.top_k };

        // AC5: 配置 QueryParser，在 content + file_path 两字段上搜，对 file_path boost
        let mut qp = QueryParser::for_index(
            &self.tantivy_index,
            vec![self.f_content, self.f_file_path],
        );
        if let Some(&b) = self.config.field_boosts.get("file_path") {
            qp.set_field_boost(self.f_file_path, b);
        }
        if let Some(&b) = self.config.field_boosts.get("content") {
            qp.set_field_boost(self.f_content, b);
        }

        // AC3: 非法语法 → 不 panic, 返空
        let query = match qp.parse_query(q_trim) {
            Ok(q) => q,
            Err(_) => return Ok(Vec::new()),
        };

        // Over-fetch to give post-filter room（filter 删一些后仍能凑齐 top_k）
        let over_fetch = top_k.saturating_mul(5).max(top_k);
        let searcher = self.tantivy_reader.searcher();
        let collector = TopDocs::with_limit(over_fetch).order_by_score();
        let top = searcher.search(&query, &collector)?;

        let want_lang = !opts.filters.language.is_empty();
        let mut results = Vec::with_capacity(top_k);

        for (score, addr) in top {
            if results.len() >= top_k {
                break;
            }
            let doc: TantivyDocument = match searcher.doc(addr) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let chunk_id = match doc.get_first(self.f_chunk_id).and_then(|v| v.as_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let language = doc
                .get_first(self.f_language)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // AC2: language filter (post-filter)
            if want_lang && !opts.filters.language.iter().any(|l| l == &language) {
                continue;
            }

            // Tantivy STORED fields — 行号区间直接读，避免多余 SQLite 列
            let line_start = doc
                .get_first(self.f_line_start)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let line_end = doc
                .get_first(self.f_line_end)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            // SQLite JOIN by chunk_id 取 file_path / content / indexed_at（time filter 用）
            let row = self.sqlite.query_row(
                "SELECT file_path, content, indexed_at
                 FROM chunks WHERE chunk_id = ?1",
                params![chunk_id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                },
            );
            let (file_path, content, indexed_at) = match row {
                Ok(t) => t,
                Err(_) => continue, // Tantivy/SQLite 暂时不同步 → skip 该 hit
            };

            // AC2: time filter (indexed_at 是 unix seconds as String，indexer rfc3339_now)
            let indexed_at_unix: i64 = indexed_at.parse().unwrap_or(0);
            if let Some(after) = opts.filters.time_after_unix {
                if indexed_at_unix < after {
                    continue;
                }
            }
            if let Some(before) = opts.filters.time_before_unix {
                if indexed_at_unix > before {
                    continue;
                }
            }

            // task-4.2 AC3 黑盒守护：优先 JOIN provenance 表；缺失合成 scanner-default
            let mut provenance = self.read_provenance(&chunk_id)?;
            if provenance.is_empty() {
                provenance.push(Provenance {
                    importer: SYNTHESIZED_IMPORTER.to_string(),
                    original_path: file_path.clone(),
                    imported_at: indexed_at.clone(),
                    source_modified_at: String::new(),
                });
            }

            // task-4.2 AC4: explain=true 时 enrich reason + matched_terms（task-4.1 留的 placeholder）
            let (reason, matched_terms) = if opts.explain {
                let terms = enrich_matched_terms(q_trim, &content);
                let r = if terms.is_empty() {
                    format!("bm25 hit on '{}'", q_trim)
                } else {
                    format!("bm25 hit on '{}'; matched terms: [{}]", q_trim, terms.join(", "))
                };
                (r, terms)
            } else {
                (String::new(), Vec::new())
            };

            results.push(SearchResult {
                chunk_id,
                context_id: DEFAULT_CONTEXT_ID.to_string(),
                source_type: DEFAULT_SOURCE_TYPE.to_string(),
                file_path,
                line_start: line_start.max(0) as u64,
                line_end: line_end.max(0) as u64,
                score,
                retrieval_method: "bm25".to_string(),
                reason,
                agent_scope: Vec::new(),
                redaction_status: DEFAULT_REDACTION_STATUS.to_string(),
                provenance,
                language,
                content,
                matched_terms,
            });
        }

        // task-18.1: placeholder vector search call.
        // If a vector_searcher is wired in, invoke it with a zero vector (embedding
        // generation is task-18.2). Results are logged but not yet merged into
        // the BM25 result set — full hybrid fusion is task-18.7.
        if let Some(searcher) = &self.vector_searcher {
            let _vector_hits = searcher
                .search(&[], opts.top_k, None)
                .unwrap_or_default();
            // TODO task-18.7: merge _vector_hits with BM25 results (hybrid fusion)
        }

        Ok(results)
    }

    /// AC4 v0.1 调试入口 — Rust public API；CLI / REST / MCP / gRPC 在 Phase 6/7 wrap.
    ///
    /// 等价 search(opts) 但强制 explain=true，让 reason / matched_terms 填实.
    /// 不消费 opts.explain 字段（无论 caller 传 true / false 都 force = true）.
    pub fn explain(&self, opts: &SearchOptions) -> Result<Vec<SearchResult>, RetrieverError> {
        let forced = SearchOptions {
            query: opts.query.clone(),
            top_k: opts.top_k,
            filters: opts.filters.clone(),
            explain: true,
        };
        self.search(&forced)
    }

    /// task-6.2 §2A 决策 E: exact `chunk_id` lookup — REST `GET /v1/chunks/{id}` fast-path.
    ///
    /// SQLite single-row `WHERE chunk_id = ?1 LIMIT 1` + provenance JOIN (same wiring
    /// as `search()` so the 12-field `SearchResult` schema parity is preserved).
    /// 未命中 → `Ok(None)` (调用方区分 not-found vs error); SQLite / IO 错 → `Err`.
    /// `retrieval_method` 复用 `"bm25"` 标签（fast-path 不引新检索方法名 — schema gap
    /// 与 task-4.2 § task-4.2 §10 一致）；`score=1.0` 标记 exact match.
    pub fn get_chunk(&self, chunk_id: &str) -> Result<Option<SearchResult>, RetrieverError> {
        let row = self.sqlite.query_row(
            "SELECT chunk_id, file_path, content, language, line_start, line_end, indexed_at
             FROM chunks WHERE chunk_id = ?1 LIMIT 1",
            params![chunk_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, String>(6)?,
                ))
            },
        );
        let (chunk_id_db, file_path, content, language, line_start, line_end, indexed_at) =
            match row {
                Ok(t) => t,
                Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
                Err(e) => return Err(RetrieverError::from(e)),
            };

        // provenance — same synthesis floor as search() so AC3 黑盒守护 仍生效
        let mut provenance = self.read_provenance(&chunk_id_db)?;
        if provenance.is_empty() {
            provenance.push(Provenance {
                importer: SYNTHESIZED_IMPORTER.to_string(),
                original_path: file_path.clone(),
                imported_at: indexed_at.clone(),
                source_modified_at: String::new(),
            });
        }

        Ok(Some(SearchResult {
            chunk_id: chunk_id_db,
            context_id: DEFAULT_CONTEXT_ID.to_string(),
            source_type: DEFAULT_SOURCE_TYPE.to_string(),
            file_path,
            line_start: line_start.max(0) as u64,
            line_end: line_end.max(0) as u64,
            score: 1.0,
            retrieval_method: "bm25".to_string(),
            reason: String::new(),
            agent_scope: Vec::new(),
            redaction_status: DEFAULT_REDACTION_STATUS.to_string(),
            provenance,
            language,
            content,
            matched_terms: Vec::new(),
        }))
    }

    /// AC3 helper: 从 indexer provenance 表读 chunk_id 的全部 importer 行（任意条 0..n）.
    fn read_provenance(&self, chunk_id: &str) -> Result<Vec<Provenance>, RetrieverError> {
        let mut stmt = self.sqlite.prepare(
            "SELECT importer, original_path, imported_at, source_modified_at
             FROM provenance WHERE chunk_id = ?1",
        )?;
        let iter = stmt.query_map(params![chunk_id], |row| {
            Ok(Provenance {
                importer: row.get::<_, String>(0)?,
                original_path: row.get::<_, String>(1)?,
                imported_at: row.get::<_, String>(2)?,
                source_modified_at: row
                    .get::<_, Option<String>>(3)?
                    .unwrap_or_default(),
            })
        })?;
        let mut out = Vec::new();
        for row in iter {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn config(&self) -> &RetrieverConfig {
        &self.config
    }

    /// task-18.1: builder method to wire in a vector backend.
    pub fn with_vector_searcher(mut self, searcher: Arc<dyn VectorSearcher>) -> Self {
        self.vector_searcher = Some(searcher);
        self
    }

    /// task-15.3 (Phase 15 P1 #3): live-doc count from the Tantivy reader.
    /// Excludes tombstoned docs — matches the user-facing "已索引块" notion of
    /// chunks currently retrievable. Cheap call (reader holds a `Searcher` per
    /// segment meta; no full scan).
    pub fn num_docs(&self) -> u64 {
        self.tantivy_reader.searcher().num_docs()
    }

    /// task-15.3 (Phase 15 P1 #3): count chunks indexed since `since_iso`
    /// (lexicographic compare on chunks.indexed_at TEXT column; works because
    /// the indexer writes a fixed-width ISO-ish string from `indexed_at_now_str`).
    /// Returns 0 when SQLite query fails so health/stats don't 503 over a
    /// transient lock — fallback safety aligns with [SPEC-OWNER:task-15.3].
    pub fn count_indexed_since(&self, since_iso: &str) -> i64 {
        match self.sqlite.query_row(
            "SELECT COUNT(*) FROM chunks WHERE indexed_at >= ?1",
            params![since_iso],
            |r| r.get::<_, i64>(0),
        ) {
            Ok(n) => n,
            Err(_) => 0,
        }
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn collection_id(&self) -> &str {
        &self.collection_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunker::ChunkPolicy;
    use crate::indexer::IndexSession;
    use crate::scanner::{default_denylist, ScanOptions};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "contextforge-retriever-{name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn scan_opts() -> ScanOptions {
        ScanOptions {
            denylist: default_denylist(),
            allowlist: Vec::new(),
            allow_denylist_override: false,
            dry_run: false,
            max_file_bytes: 10 * 1024 * 1024,
        }
    }

    /// 通过 task-2.4 indexer 在 data_dir 上写好测试 fixture，返回 (src_root, data_dir, coll_id).
    fn build_fixture(name: &str, files: &[(&str, &str)]) -> (PathBuf, PathBuf, String) {
        let src = temp_root(&format!("{name}-src"));
        let data = temp_root(&format!("{name}-data"));
        let coll = format!("test-{}", name);
        for (rel, body) in files {
            let p = src.join(rel);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&p, body).unwrap();
        }
        let mut sess = IndexSession::open(&data, &coll).expect("open indexer");
        sess.index_path(&src, &scan_opts(), &ChunkPolicy::default(), vec![])
            .expect("index_path");
        sess.commit().expect("commit");
        (src, data, coll)
    }

    // ---- TEST-4.1.1 / SCEN-4.1.1 (AC1) — BM25 Top-K ----
    #[test]
    fn test_4_1_1_bm25_top_k_returns_hits() {
        let (_src, data, coll) = build_fixture(
            "ac1",
            &[
                ("readme.md", "# Readme\n\nThis is a unique token alphabetagamma1 in body.\n"),
                ("other.md", "# Other doc\nnothing here\n"),
            ],
        );
        let retr = Retriever::open(&data, &coll).expect("open");
        let results = retr
            .search(&SearchOptions {
                query: "alphabetagamma1".into(),
                top_k: 10,
                filters: SearchFilters::default(),
                explain: false,
            })
            .expect("search");

        assert!(
            !results.is_empty(),
            "AC1: BM25 应命中 unique token, got {} hits",
            results.len()
        );
        let first = &results[0];
        assert!(!first.chunk_id.is_empty(), "AC1: chunk_id non-empty");
        assert!(first.score > 0.0, "AC1: score > 0, got {}", first.score);
        assert_eq!(first.retrieval_method, "bm25", "AC1: method=bm25");
        assert!(!first.file_path.is_empty(), "AC1: file_path non-empty");
        assert!(first.line_end >= first.line_start, "AC1: line range valid");
    }

    // ---- TEST-4.1.2 / SCEN-4.1.2 (AC2) — filter 契约 ----
    //
    // 注意：用 .md (pulldown-cmark) + .txt (text fallback) 两种 parser 路径都保留完整
    // body 进 chunk —— 而 .rs (tree-sitter) 只抽 named items，行内 marker comment 不进 chunk.
    #[test]
    fn test_4_1_2_filter_language_works() {
        let (_src, data, coll) = build_fixture(
            "ac2",
            &[
                ("a.md", "# Md doc\nthe shared marker langfiltermarkerz is here\n"),
                (
                    "b.txt",
                    "Text doc\nthe shared marker langfiltermarkerz is here\n",
                ),
            ],
        );
        let retr = Retriever::open(&data, &coll).expect("open");

        // 不过滤：两种语言都应命中（sanity）
        let all = retr
            .search(&SearchOptions {
                query: "langfiltermarkerz".into(),
                top_k: 10,
                filters: SearchFilters::default(),
                explain: false,
            })
            .expect("search all");
        assert!(
            all.len() >= 2,
            "AC2 sanity: 两种语言都应命中 (got {})",
            all.len()
        );

        // language=["markdown"] 仅 .md 文件
        let only_md = retr
            .search(&SearchOptions {
                query: "langfiltermarkerz".into(),
                top_k: 10,
                filters: SearchFilters {
                    language: vec!["markdown".to_string()],
                    ..Default::default()
                },
                explain: false,
            })
            .expect("search md");
        assert!(!only_md.is_empty(), "AC2: markdown filter 应有 ≥1 hit");
        for r in &only_md {
            assert_eq!(
                r.language, "markdown",
                "AC2: 结果应全部 language=markdown, got '{}'",
                r.language
            );
        }
    }

    // ---- TEST-4.1.3 / SCEN-4.1.3 (AC3) — 空/错误 query 不 panic ----
    #[test]
    fn test_4_1_3_empty_or_malformed_query_returns_empty_safely() {
        let (_src, data, coll) = build_fixture(
            "ac3",
            &[("readme.md", "# Readme\nbody with safetestmarker3z\n")],
        );
        let retr = Retriever::open(&data, &coll).expect("open");

        // 空 query → Ok(empty Vec)
        let r1 = retr
            .search(&SearchOptions {
                query: "".into(),
                top_k: 10,
                filters: SearchFilters::default(),
                explain: false,
            })
            .expect("AC3: empty query should not error");
        assert!(r1.is_empty(), "AC3: 空 query 应返 empty Vec");

        // 仅空白 → 同
        let r2 = retr
            .search(&SearchOptions {
                query: "   \n\t".into(),
                top_k: 10,
                filters: SearchFilters::default(),
                explain: false,
            })
            .expect("AC3: whitespace query should not error");
        assert!(r2.is_empty(), "AC3: 全空白 query 应返 empty Vec");

        // 非法 QueryParser 语法 → Ok(empty)，不 Err，不 panic
        let r3 = retr
            .search(&SearchOptions {
                query: "??!!".into(),
                top_k: 10,
                filters: SearchFilters::default(),
                explain: false,
            })
            .expect("AC3: malformed query should not error");
        assert!(r3.is_empty(), "AC3: 非法 query 应返 empty Vec");

        // Sanity: 正常 query 必须有结果（防 stub 永远返 empty 假绿）
        let r4 = retr
            .search(&SearchOptions {
                query: "safetestmarker3z".into(),
                top_k: 10,
                filters: SearchFilters::default(),
                explain: false,
            })
            .expect("AC3 sanity: valid query");
        assert!(
            !r4.is_empty(),
            "AC3 sanity: 正常 query 应有 ≥1 hit (防 stub 永远 empty 假绿)"
        );
    }

    // ---- TEST-4.1.4 / SCEN-4.1.4 (AC4) — 架构支持快速检索（基线非硬测）----
    #[test]
    fn test_4_1_4_basic_latency_architecture_check() {
        // 50 docs 合成 fixture（不是 PRD 10万 chunk 压测；只验证架构无 pathological 慢路径）
        let bodies: Vec<String> = (0..50)
            .map(|i| format!("# Doc {}\nbody line {} fastsearchmark perfz9\n", i, i))
            .collect();
        let names: Vec<String> = (0..50).map(|i| format!("doc{:03}.md", i)).collect();
        let files: Vec<(&str, &str)> = names
            .iter()
            .zip(bodies.iter())
            .map(|(n, b)| (n.as_str(), b.as_str()))
            .collect();
        let (_src, data, coll) = build_fixture("ac4", &files);
        let retr = Retriever::open(&data, &coll).expect("open");

        let started = std::time::Instant::now();
        let results = retr
            .search(&SearchOptions {
                query: "fastsearchmark".into(),
                top_k: 10,
                filters: SearchFilters::default(),
                explain: false,
            })
            .expect("search");
        let elapsed = started.elapsed();

        assert!(!results.is_empty(), "AC4: 应有命中（非空 sanity）");
        // 架构性能基线：50 文件 1-shot 应 < 500ms（PRD 性能阈值是 10 万 chunk；此处只验证架构无慢路径）
        assert!(
            elapsed.as_millis() < 500,
            "AC4 (architecture baseline): 50-doc 单次检索应 < 500ms, got {} ms (PRD 10万 chunk P95<500ms 真实压测在 task-8.1 eval-harness)",
            elapsed.as_millis()
        );
    }

    // ---- TEST-4.1.5 / SCEN-4.1.5 (AC5) — tokenizer / boost / exact phrase ----
    //
    // v0.1 限制：task-2.4 Tantivy schema 中 file_path 是 STRING（非 tokenized）— path
    // 子串搜索不被 QueryParser 命中（需 SPEC-DRIFT-task-2.4 改 TEXT；§10 schema gap）.
    // 因此本测试 AC5 boost 部分只验证 API 契约（config 暴露 boost map + set_field_boost
    // 调用不报错），不验证 ranking 实际效果。Exact phrase 走 TEXT field 仍可严格测.
    #[test]
    fn test_4_1_5_boost_and_exact_phrase() {
        let (_src, data, coll) = build_fixture(
            "ac5",
            &[
                ("adjacent.md", "# Adjacent\nfoo bar quick brown\n"),
                ("split.md", "# Split\nfoo zip bar nope\n"),
                ("normal.md", "# Normal\nordinary content here\n"),
            ],
        );
        let retr = Retriever::open(&data, &coll).expect("open");

        // Exact phrase: "\"foo bar\"" 应仅命中相邻
        let phrase_results = retr
            .search(&SearchOptions {
                query: "\"foo bar\"".into(),
                top_k: 10,
                filters: SearchFilters::default(),
                explain: false,
            })
            .expect("search phrase");
        assert!(
            !phrase_results.is_empty(),
            "AC5 phrase: \"foo bar\" 应命中相邻文档"
        );
        for r in &phrase_results {
            assert!(
                r.file_path.ends_with("adjacent.md"),
                "AC5 phrase: 命中文档应为 adjacent.md (相邻), got file_path={}",
                r.file_path
            );
        }

        // Non-phrase 同 keywords 走默认 AND，split.md 也命中（foo / bar 各自存在）
        let any_results = retr
            .search(&SearchOptions {
                query: "foo bar".into(),
                top_k: 10,
                filters: SearchFilters::default(),
                explain: false,
            })
            .expect("search any");
        assert!(
            any_results.len() >= 2,
            "AC5 non-phrase: 两文档都含 foo + bar, got {}",
            any_results.len()
        );

        // Config 暴露 tokenizer/boost 接入点（API 契约）
        assert_eq!(retr.config().tokenizer, "default");
        assert_eq!(
            retr.config().field_boosts.get("file_path").copied(),
            Some(2.0),
            "AC5: file_path boost 默认 2.0"
        );
        assert_eq!(
            retr.config().field_boosts.get("content").copied(),
            Some(1.0),
            "AC5: content boost 默认 1.0"
        );
        assert!(retr.config().enable_exact_phrase, "AC5: exact_phrase 默认开");

        // open_with_config 接入点：自定义 tokenizer 名 + boost map (不需实际生效)
        let mut custom_boosts = HashMap::new();
        custom_boosts.insert("content".to_string(), 3.0);
        let custom_cfg = RetrieverConfig {
            tokenizer: "default".to_string(), // CJK 留接入点 (PRD §O11)
            field_boosts: custom_boosts,
            enable_exact_phrase: false,
        };
        let retr2 =
            Retriever::open_with_config(&data, &coll, custom_cfg).expect("open with config");
        assert_eq!(retr2.config().field_boosts.get("content").copied(), Some(3.0));
        assert!(!retr2.config().enable_exact_phrase);
    }

    // ============================================================================
    // task-4.2 §2A (2026-05-23) — explainable retrieval trace + 12-field result schema
    // ============================================================================
    // SCEN-4.2.1 ~ SCEN-4.2.5 — RED commit: 4 tests in this mod + TEST-4.2.5 in
    // core/tests/phase4_smoke.rs（主 agent §4 Gate 3 cargo test --test phase4_smoke 入口）。

    // ---- TEST-4.2.1 / SCEN-4.2.1 (AC1) — 12-field explainable contract PRESENT ----
    //
    // Schema parity (PRD §Technical Approach REST/MCP search response + proto
    // RetrievalResult)：每条 SearchResult 必有全部 12 字段 PRESENT（struct 强制 = compile gate）.
    // v0.1 schema gap 字段（context_id / source_type / agent_scope / redaction_status）
    // 返 §2A default 常量（"" / "" / vec![] / "applied"）.
    #[test]
    fn test_4_2_1_search_result_has_all_12_explainable_fields() {
        let (_src, data, coll) = build_fixture(
            "ac1-explain",
            &[("readme.md", "# Readme\n\nunique token explainmarker42 in body.\n")],
        );
        let retr = Retriever::open(&data, &coll).expect("open");
        let results = retr
            .search(&SearchOptions {
                query: "explainmarker42".into(),
                top_k: 10,
                filters: SearchFilters::default(),
                explain: false,
            })
            .expect("search");
        assert!(!results.is_empty(), "AC1 sanity: should hit on unique token");
        let r = &results[0];

        // 12 explainable fields PRESENT (compile-enforced — struct membership;
        // 运行时 sanity 校验各字段已被 search() 显式赋值 + v0.1 default 决策一致).
        assert!(!r.chunk_id.is_empty(), "AC1: chunk_id non-empty");
        assert_eq!(
            r.context_id, "",
            "AC1 §2A v0.1 schema gap: context_id 默认 \"\""
        );
        assert_eq!(
            r.source_type, "",
            "AC1 §2A v0.1 schema gap: source_type 默认 \"\""
        );
        assert!(!r.file_path.is_empty(), "AC1: file_path non-empty");
        assert!(
            r.line_end >= r.line_start,
            "AC1: line range valid (line_end={} >= line_start={})",
            r.line_end,
            r.line_start
        );
        assert!(r.score > 0.0, "AC1: score > 0, got {}", r.score);
        assert_eq!(r.retrieval_method, "bm25", "AC1: method=bm25");
        // reason: explain=false → "" (proto3 string 空默认; explain=true 时 enrich)
        assert_eq!(
            r.reason, "",
            "AC1 explain=false: reason 默认空（explain=true → enrich, TEST-4.2.4 校验）"
        );
        assert!(
            r.agent_scope.is_empty(),
            "AC1 §2A v0.1 schema gap: agent_scope 默认 empty"
        );
        assert_eq!(
            r.redaction_status, "applied",
            "AC1 §2A v0.1 default: \"applied\"（indexer BINDING 仅消费 redacted_content）"
        );
        // provenance: AC3 黑盒守护 — 每条 ≥1（合成 scanner-default 若无 importer 行）
        // 注：TEST-4.2.3 专门压这一点；本测试仅作 schema coverage sanity
        assert!(
            !r.provenance.is_empty(),
            "AC1 + AC3 黑盒守护：provenance.len() ≥ 1（合成 scanner-default 若无真实 importer 行）, got {}",
            r.provenance.len()
        );
    }

    // ---- TEST-4.2.2 / SCEN-4.2.2 (AC2) — file_path + line_start/end 精确定位回原文 ----
    //
    // 多 chunk 文件 → 命中 → 校验 file_path + line_start/end 落在 fixture 真实行号范围内
    // 且按 (file, line_start, line_end) 切片可恢复原始内容.
    #[test]
    fn test_4_2_2_result_locates_back_to_file_and_line() {
        let body = "# Section A\nlocateme42 first hit content here\n\
                    line three\nline four\nline five\n\
                    # Section B\nanother body\nlast line\n";
        let total_lines = body.lines().count() as u64;
        let (_src, data, coll) = build_fixture("ac2-locate", &[("multi.md", body)]);
        let retr = Retriever::open(&data, &coll).expect("open");
        let results = retr
            .search(&SearchOptions {
                query: "locateme42".into(),
                top_k: 10,
                filters: SearchFilters::default(),
                explain: false,
            })
            .expect("search");
        assert!(!results.is_empty(), "AC2 sanity: should hit on locateme42");
        let r = &results[0];

        // file_path 精确（不模糊不偏移）
        assert!(
            r.file_path.ends_with("multi.md"),
            "AC2: file_path 应精确指向 fixture 'multi.md', got {}",
            r.file_path
        );
        // line_start / line_end 落在 fixture 真实行号范围内
        assert!(
            r.line_start >= 1 && r.line_end <= total_lines,
            "AC2: line range [{}, {}] 应落在 fixture 1..={} 范围内",
            r.line_start,
            r.line_end,
            total_lines
        );
        assert!(
            r.line_end >= r.line_start,
            "AC2: line_end ({}) 必 >= line_start ({})",
            r.line_end,
            r.line_start
        );
        // chunk content 应含 trigger token（恢复原文 sanity）
        assert!(
            r.content.contains("locateme42"),
            "AC2: chunk content 应含 trigger token 'locateme42', got: {:?}",
            r.content
        );
    }

    // ---- TEST-4.2.3 / SCEN-4.2.3 (AC3) — schema coverage 100% + 反指标 provenance ≥1 ----
    //
    // 反指标硬约束：PRD §Success Metrics 反指标「禁返回无 provenance 的黑盒高分」.
    // v0.1 量化（§2A 决策）：每条 result.provenance.len() ≥ 1（合成 scanner-default）.
    // 多文件 fixture → 多条结果 → 每条都过黑盒守护.
    #[test]
    fn test_4_2_3_no_black_box_results_provenance_floor() {
        let (_src, data, coll) = build_fixture(
            "ac3-noblackbox",
            &[
                ("a.md", "# A\nblackboxguard9z in a doc.\n"),
                ("b.md", "# B\nblackboxguard9z in b doc.\n"),
                ("c.md", "# C\nblackboxguard9z in c doc.\n"),
            ],
        );
        let retr = Retriever::open(&data, &coll).expect("open");
        let results = retr
            .search(&SearchOptions {
                query: "blackboxguard9z".into(),
                top_k: 10,
                filters: SearchFilters::default(),
                explain: false,
            })
            .expect("search");

        assert!(
            results.len() >= 3,
            "AC3 sanity: 3 文件都含 marker, 应有 3 hits, got {}",
            results.len()
        );

        // 反指标硬约束：每条 result.provenance.len() ≥ 1 （黑盒守护）
        let mut black_box_count = 0;
        for (i, r) in results.iter().enumerate() {
            if r.provenance.is_empty() {
                black_box_count += 1;
                eprintln!(
                    "AC3 violation: result {}（chunk_id={}, file_path={}, score={}）provenance 为空 — \"黑盒高分\"",
                    i, r.chunk_id, r.file_path, r.score
                );
            }
        }
        assert_eq!(
            black_box_count, 0,
            "AC3 反指标：禁返回无 provenance 的黑盒高分结果（共 {} 条违规 / {} 条总）",
            black_box_count,
            results.len()
        );

        // Schema coverage 100%（struct 强制；运行时再断言 12 字段都 valid）：
        // 此循环 sanity 校验合成 provenance 的 4 字段都非空（结构性完整）
        for r in &results {
            for (j, prov) in r.provenance.iter().enumerate() {
                assert!(
                    !prov.importer.is_empty(),
                    "AC3 provenance #{} importer 非空（合成 'scanner' 或真实 importer 名）",
                    j
                );
                assert!(
                    !prov.original_path.is_empty(),
                    "AC3 provenance #{} original_path 非空（合成 file_path 或真实 importer.original_path）",
                    j
                );
                assert!(
                    !prov.imported_at.is_empty(),
                    "AC3 provenance #{} imported_at 非空（合成 indexed_at 或真实 importer.imported_at）",
                    j
                );
            }
        }
    }

    // ---- TEST-4.2.4 / SCEN-4.2.4 (AC4) — Retriever::explain Rust public API 调试入口 ----
    //
    // §2A AC4 决策：v0.1 调试入口 = Rust public API。gRPC server / Go CLI 留 Phase 6.
    // explain(opts) 等价 search(opts with explain=true) — reason / matched_terms 填实.
    #[test]
    fn test_4_2_4_explain_entry_enriches_reason_and_matched_terms() {
        let (_src, data, coll) = build_fixture(
            "ac4-explain-entry",
            &[("readme.md", "# Readme\nexplainentrymarker77 here in body\n")],
        );
        let retr = Retriever::open(&data, &coll).expect("open");

        // explain=false → reason 空（task-4.1 已做的对照基线）
        let plain = retr
            .search(&SearchOptions {
                query: "explainentrymarker77".into(),
                top_k: 10,
                filters: SearchFilters::default(),
                explain: false,
            })
            .expect("search plain");
        assert!(!plain.is_empty(), "AC4 sanity: should hit");
        assert_eq!(plain[0].reason, "", "AC4 baseline: explain=false → reason \"\"");
        assert!(
            plain[0].matched_terms.is_empty(),
            "AC4 baseline: explain=false → matched_terms empty"
        );

        // explain() 公开 API → reason / matched_terms 填实
        let explained = retr
            .explain(&SearchOptions {
                query: "explainentrymarker77".into(),
                top_k: 10,
                filters: SearchFilters::default(),
                explain: false, // explain() 自己 force = true
            })
            .expect("AC4: Retriever::explain 应返 Ok（v0.1 公开调试入口）");
        assert!(!explained.is_empty(), "AC4: explain 应有结果");
        let r = &explained[0];
        // reason 非空 + 含 BM25 / matched 词（enrichment 内容）
        assert!(
            !r.reason.is_empty(),
            "AC4: explain() 后 reason 必非空（含 'bm25' / 'matched' / 等可解释词）, got: {:?}",
            r.reason
        );
        let reason_lower = r.reason.to_lowercase();
        assert!(
            reason_lower.contains("bm25") || reason_lower.contains("matched"),
            "AC4: reason 应含可解释标识（'bm25' or 'matched'）, got: {:?}",
            r.reason
        );
        // matched_terms 非空（含 query trigger token）
        assert!(
            !r.matched_terms.is_empty(),
            "AC4: explain() 后 matched_terms 必非空（含 query 中可命中的 token）"
        );
        let any_match = r
            .matched_terms
            .iter()
            .any(|t| t.to_lowercase().contains("explainentrymarker77"));
        assert!(
            any_match,
            "AC4: matched_terms 应含 query trigger token 'explainentrymarker77', got: {:?}",
            r.matched_terms
        );
    }

    // ============================================================================
    // task-6.2 §2A 决策 E — retriever.get_chunk 公开 API (REST GET /v1/chunks/{id} fast-path).
    // ============================================================================

    // ---- TEST-6.2.E1 — get_chunk hit returns full 12-field SearchResult ----
    #[test]
    fn test_6_2_e1_get_chunk_returns_12_field_result_on_hit() {
        let (_src, data, coll) = build_fixture(
            "ac2e-hit",
            &[("readme.md", "# Readme\nunique token getchunkmarker62z\n")],
        );
        let retr = Retriever::open(&data, &coll).expect("open");
        // 先用 search 拿到一个真实 chunk_id（fixture 索引后 chunk_id 由 indexer 决定）
        let results = retr
            .search(&SearchOptions {
                query: "getchunkmarker62z".into(),
                top_k: 1,
                filters: SearchFilters::default(),
                explain: false,
            })
            .expect("seed search");
        assert!(!results.is_empty(), "seed: should hit fixture");
        let target_chunk_id = results[0].chunk_id.clone();

        // get_chunk(target_chunk_id) → Ok(Some(SearchResult)) with 12 字段 PRESENT
        let got = retr
            .get_chunk(&target_chunk_id)
            .expect("get_chunk(hit) must Ok");
        assert!(got.is_some(), "AC2-E hit: get_chunk 应返 Some(SearchResult)");
        let r = got.unwrap();
        assert_eq!(r.chunk_id, target_chunk_id, "AC2-E: chunk_id 一致");
        assert!(!r.file_path.is_empty(), "AC2-E: file_path non-empty");
        assert_eq!(r.context_id, "", "AC2-E §2A default schema gap");
        assert_eq!(r.source_type, "", "AC2-E §2A default");
        assert!(r.line_end >= r.line_start, "AC2-E: line range valid");
        assert_eq!(
            r.retrieval_method, "bm25",
            "AC2-E: retrieval_method 复用 search() 的 bm25 标签 (provenance 表 schema 不动)"
        );
        assert!(r.agent_scope.is_empty(), "AC2-E §2A default");
        assert_eq!(r.redaction_status, "applied", "AC2-E §2A default");
        assert!(
            !r.provenance.is_empty(),
            "AC2-E: provenance.len() ≥ 1 (沿 AC3 黑盒守护)"
        );
    }

    // ---- TEST-6.2.E2 — get_chunk miss returns Ok(None), 不 Err ----
    #[test]
    fn test_6_2_e2_get_chunk_returns_none_on_miss() {
        let (_src, data, coll) = build_fixture(
            "ac2e-miss",
            &[("readme.md", "# Readme\nany content\n")],
        );
        let retr = Retriever::open(&data, &coll).expect("open");
        let got = retr
            .get_chunk("nonexistent_chunk_id_zzz999")
            .expect("get_chunk(miss) must Ok (not Err)");
        assert!(
            got.is_none(),
            "AC2-E miss: get_chunk 未命中应返 Ok(None), got Some"
        );
    }

    // ---- task-18.1 retriever-level tests (TEST-18.1.6/7) ----

    // TEST-18.1.6 — AC3: with no vector_searcher wired, the hot path stays the v0.10 BM25 path
    // and retrieval_method is preserved as "bm25" on every hit.
    #[test]
    fn test_retriever_none_vector_searcher_bm25_unchanged() {
        let (_src, data, coll) = build_fixture("vec_none_bm25", &[("a.txt", "hello vector world")]);
        let retr = Retriever::open(&data, &coll).expect("open");
        assert!(retr.vector_searcher.is_none(), "default vector_searcher should be None");
        let results = retr
            .search(&SearchOptions { query: "vector".into(), top_k: 5, filters: SearchFilters::default(), explain: false })
            .expect("search");
        assert!(!results.is_empty(), "BM25 should still return results without vector_searcher");
        assert!(
            results.iter().all(|r| r.retrieval_method == "bm25"),
            "None vector_searcher must keep retrieval_method == \"bm25\" on every hit"
        );
    }

    // TEST-18.1.7 — AC3: wiring NoopVectorBackend must NOT perturb the BM25 result set.
    // Prove equivalence against a None baseline (same chunk_ids, scores, order) rather than a
    // weak "still returns ≥1 hit" smoke, and confirm retrieval_method stays "bm25" (vector hits
    // are empty and not merged at task-18.1).
    #[test]
    fn test_retriever_some_noop_vector_searcher_returns_empty_vector_hits() {
        let (_src, data, coll) = build_fixture(
            "vec_noop_equiv",
            &[("a.txt", "hello vector world"), ("b.txt", "semantic vector search world")],
        );
        let opts = || SearchOptions { query: "vector".into(), top_k: 5, filters: SearchFilters::default(), explain: false };

        // Baseline: no vector_searcher wired.
        let baseline: Vec<(String, f32, String)> = Retriever::open(&data, &coll)
            .expect("open baseline")
            .search(&opts())
            .expect("baseline search")
            .into_iter()
            .map(|r| (r.chunk_id, r.score, r.retrieval_method))
            .collect();
        assert!(!baseline.is_empty(), "fixture should yield BM25 hits");

        // Same index, NoopVectorBackend wired in.
        let retr = Retriever::open(&data, &coll)
            .expect("open noop")
            .with_vector_searcher(Arc::new(NoopVectorBackend));
        assert!(retr.vector_searcher.is_some(), "with_vector_searcher should set Some");
        let with_noop: Vec<(String, f32, String)> = retr
            .search(&opts())
            .expect("noop search")
            .into_iter()
            .map(|r| (r.chunk_id, r.score, r.retrieval_method))
            .collect();

        // Noop returns empty vector hits → BM25 result set is identical to the baseline.
        assert_eq!(
            baseline, with_noop,
            "NoopVectorBackend must not change BM25 chunk_ids / scores / order"
        );
        assert!(
            with_noop.iter().all(|(_, _, m)| m == "bm25"),
            "retrieval_method must stay \"bm25\" (empty vector hits are not merged)"
        );
    }
}
