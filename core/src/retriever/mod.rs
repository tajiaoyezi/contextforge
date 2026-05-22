//! task-4.1 (Phase 4): retriever — BM25 / metadata / filter 检索（read-only）.
//!
//! RED checkpoint: 公开类型与方法签名按 §5.3 落地；方法体均为 stub
//! (Retriever::search 永远返回 Ok(Vec::new())) 让 5 个 RED 测试 compilable + 描述性失败.
//! GREEN 替换 stub 为 tantivy 0.26 QueryParser + rusqlite JOIN 实现.
//!
//! 数据目录与 task-2.4 IndexSession 一致：
//!   [data_dir]/collections/[collection_id]/{metadata.sqlite, tantivy/}
//!
//! Tantivy schema 由 task-2.4 frozen，本模块只读不重定义（meta.json 自携）.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Value};
use tantivy::{Index, IndexReader, TantivyDocument};
use thiserror::Error;

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

/// 检索结果（PRD §REST/MCP search response 契约对齐）.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub chunk_id: String,
    pub file_path: String,
    pub line_start: u64,
    pub line_end: u64,
    pub language: String,
    pub content: String,
    pub score: f32,
    pub retrieval_method: String,
    pub reason: Option<String>,
    pub matched_terms: Vec<String>,
}

/// Retained for task-1.3 core_skeleton.rs anchor (AC4 wiring). Returns true.
pub fn placeholder_ready() -> bool {
    true
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

            results.push(SearchResult {
                chunk_id,
                file_path,
                line_start: line_start.max(0) as u64,
                line_end: line_end.max(0) as u64,
                language,
                content,
                score,
                retrieval_method: "bm25".to_string(),
                reason: if opts.explain {
                    Some(format!("BM25 match for query '{}'", q_trim))
                } else {
                    None
                },
                matched_terms: Vec::new(), // task-4.2 enriches
            });
        }

        Ok(results)
    }

    pub fn config(&self) -> &RetrieverConfig {
        &self.config
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
}
