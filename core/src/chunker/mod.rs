//! task-2.3 (Phase 2): chunker — chunking + metadata 抽取 + provenance 维护.
//!
//! RED checkpoint: types per §5.3 contract; function bodies are deliberate stubs
//! (return Ok(vec![]) / constant hash) so the 5 RED tests below compile + fail
//! with descriptive assertions. GREEN commit replaces stubs with real impl.
//!
//! 后续 task-2.4 (indexer) 消费 Vec<Chunk>。content_hash v0.1 = std-only FNV-1a-64
//! (§2A 决策)；存储格式 "fnv1a64:<16-hex>"。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::parser::ParsedUnit;

/// Retained for task-1.3 core_skeleton.rs anchor (AC4 wiring). Returns true.
pub fn placeholder_ready() -> bool {
    true
}

/// 检索切片（chunker 产出 → 喂给 indexer）。字段集对应 PRD §Technical Approach
/// Canonical Record v0.1 + AC1 列出的 7 个必含字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub chunk_id: String,
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub language: String,
    pub content: String,
    pub content_hash: String,
    pub kind: Option<String>,
    pub provenance: Vec<Provenance>,
    pub metadata: HashMap<String, String>,
}

/// 来源链（AC2 多来源）。与 PRD §Technical Approach Canonical Record `provenance[]` 对齐。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Provenance {
    pub importer: String,
    pub original_path: String,
    pub imported_at: String,
    pub source_modified_at: String,
}

/// 单语言 chunking 配置（AC3 可配置 + R3 调优）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkConfig {
    pub max_chunk_lines: usize,
    pub overlap_lines: usize,
    pub respect_parsed_units: bool,
}

/// 按语言分组的策略集（AC3：code/markdown/log 分别可调）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkPolicy {
    pub code: ChunkConfig,
    pub markdown: ChunkConfig,
    pub log: ChunkConfig,
    pub text: ChunkConfig,
}

impl Default for ChunkPolicy {
    fn default() -> Self {
        ChunkPolicy {
            code: ChunkConfig { max_chunk_lines: 80, overlap_lines: 0, respect_parsed_units: true },
            markdown: ChunkConfig { max_chunk_lines: 60, overlap_lines: 4, respect_parsed_units: true },
            log: ChunkConfig { max_chunk_lines: 200, overlap_lines: 0, respect_parsed_units: false },
            text: ChunkConfig { max_chunk_lines: 100, overlap_lines: 0, respect_parsed_units: false },
        }
    }
}

#[derive(Error, Debug)]
pub enum ChunkError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: {0}")]
    Parse(String),
    #[error("invalid chunk config: {0}")]
    InvalidConfig(String),
}

/// 主入口：把 parser 产出的解析单元切片为 Chunk。
///
/// RED stub: returns empty Vec → 5 个 RED 测试断言会失败。GREEN commit 替换。
pub fn chunk_units(
    _units: &[ParsedUnit],
    _file_path: &Path,
    _policy: &ChunkPolicy,
    _provenance: Vec<Provenance>,
) -> Result<Vec<Chunk>, ChunkError> {
    Ok(Vec::new())
}

/// 便利入口：直接读文件 + 调 parser + chunk。
///
/// RED stub: 返回空 Vec（GREEN 替换）。
pub fn chunk_file(
    _path: &Path,
    _policy: &ChunkPolicy,
    _provenance: Vec<Provenance>,
) -> Result<Vec<Chunk>, ChunkError> {
    // Suppress unused PathBuf import warning in RED stub
    let _: Option<PathBuf> = None;
    Ok(Vec::new())
}

/// 公开：算 content_hash（memoryops 去重锚点；AC5 跨来源一致）。
/// 算法 v0.1 = FNV-1a-64；返回 "fnv1a64:<16-hex>"。
///
/// RED stub: 返回常量 → TEST-2.3.5 "different content → different hash" 会断言失败。
pub fn content_hash(_content: &str) -> String {
    "fnv1a64:0000000000000000".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: 1 个 ParsedUnit
    fn unit(lang: &str, ls: usize, le: usize, body: &str, kind: Option<&str>) -> ParsedUnit {
        ParsedUnit {
            language: lang.to_string(),
            line_start: ls,
            line_end: le,
            content: body.to_string(),
            kind: kind.map(|s| s.to_string()),
            metadata: HashMap::new(),
        }
    }

    // ---- TEST-2.3.1 / SCEN-2.3.1 (AC1) — Chunk 字段完整 ----
    #[test]
    fn test_2_3_1_chunk_required_fields_populated() {
        let units = vec![
            unit("rust", 1, 1, "fn main() {}", Some("function")),
            unit("rust", 3, 3, "struct Foo {}", Some("struct")),
        ];
        let chunks = chunk_units(&units, Path::new("a.rs"), &ChunkPolicy::default(), vec![])
            .expect("chunk_units");
        assert!(!chunks.is_empty(), "AC1: must produce at least 1 Chunk");
        for c in &chunks {
            assert!(!c.chunk_id.is_empty(), "AC1: chunk_id non-empty");
            assert_eq!(c.file_path, "a.rs", "AC1: file_path");
            assert!(c.line_start >= 1, "AC1: line_start ≥ 1 (got {})", c.line_start);
            assert!(c.line_end >= c.line_start, "AC1: line_end ≥ line_start");
            assert_eq!(c.language, "rust", "AC1: language");
            assert!(!c.content.is_empty(), "AC1: content non-empty");
            assert!(
                c.content_hash.starts_with("fnv1a64:"),
                "AC1: content_hash algo-prefixed (got '{}')",
                c.content_hash
            );
            // Hash hex part length: 16 chars (64-bit)
            assert_eq!(c.content_hash.len(), "fnv1a64:".len() + 16, "AC1: hash hex length");
        }
    }

    // ---- TEST-2.3.2 / SCEN-2.3.2 (AC2) — provenance 多来源 ----
    #[test]
    fn test_2_3_2_provenance_multi_source() {
        let units = vec![unit("markdown", 1, 2, "# Title\nbody", Some("heading"))];
        let prov = vec![
            Provenance {
                importer: "local-fs".into(),
                original_path: "/p/a.md".into(),
                imported_at: "2026-05-21T00:00:00Z".into(),
                source_modified_at: "2026-05-21T00:00:00Z".into(),
            },
            Provenance {
                importer: "hermes-memory".into(),
                original_path: "MEMORY.md".into(),
                imported_at: "2026-05-21T01:00:00Z".into(),
                source_modified_at: "2026-05-20T00:00:00Z".into(),
            },
        ];
        let chunks =
            chunk_units(&units, Path::new("a.md"), &ChunkPolicy::default(), prov.clone()).unwrap();
        assert!(!chunks.is_empty(), "AC2: chunks produced");
        for c in &chunks {
            assert_eq!(c.provenance.len(), 2, "AC2: must carry both provenances");
            assert!(
                c.provenance.iter().any(|p| p.importer == "local-fs"),
                "AC2: local-fs preserved"
            );
            assert!(
                c.provenance.iter().any(|p| p.importer == "hermes-memory"),
                "AC2: hermes-memory preserved"
            );
            // 字段透传
            let lf = c.provenance.iter().find(|p| p.importer == "local-fs").unwrap();
            assert_eq!(lf.original_path, "/p/a.md");
            assert_eq!(lf.imported_at, "2026-05-21T00:00:00Z");
            assert_eq!(lf.source_modified_at, "2026-05-21T00:00:00Z");
        }
    }

    // ---- TEST-2.3.3 / SCEN-2.3.3 (AC3) — chunking 可配置 ----
    #[test]
    fn test_2_3_3_chunking_policy_configurable_per_language() {
        // 200 行 go 内容（单个 ParsedUnit）
        let big = (1..=200).map(|i| format!("line{}", i)).collect::<Vec<_>>().join("\n");
        let units = vec![unit("go", 1, 200, &big, None)];

        // policy A: code 50 行/chunk
        let mut policy_a = ChunkPolicy::default();
        policy_a.code = ChunkConfig {
            max_chunk_lines: 50,
            overlap_lines: 0,
            respect_parsed_units: false,
        };
        // policy B: code 100 行/chunk
        let mut policy_b = ChunkPolicy::default();
        policy_b.code = ChunkConfig {
            max_chunk_lines: 100,
            overlap_lines: 0,
            respect_parsed_units: false,
        };

        let a = chunk_units(&units, Path::new("big.go"), &policy_a, vec![]).unwrap();
        let b = chunk_units(&units, Path::new("big.go"), &policy_b, vec![]).unwrap();
        assert!(
            a.len() > b.len(),
            "AC3: 更小 max_chunk_lines 应产更多 chunk (a={}, b={})",
            a.len(),
            b.len()
        );
        assert!(a.len() >= 4, "AC3: 200/50 ≥ 4 chunks (got {})", a.len());
        assert!(b.len() >= 2, "AC3: 200/100 ≥ 2 chunks (got {})", b.len());

        // 改 code 不影响 markdown：100 行 markdown，code=10/md=50 → 走 md（≤3 chunks）
        let md_body =
            (1..=100).map(|i| format!("- item {}", i)).collect::<Vec<_>>().join("\n");
        let md_units = vec![unit("markdown", 1, 100, &md_body, None)];
        let mut policy_c = ChunkPolicy::default();
        policy_c.code = ChunkConfig {
            max_chunk_lines: 10,
            overlap_lines: 0,
            respect_parsed_units: false,
        };
        policy_c.markdown = ChunkConfig {
            max_chunk_lines: 50,
            overlap_lines: 0,
            respect_parsed_units: false,
        };
        let md_chunks = chunk_units(&md_units, Path::new("a.md"), &policy_c, vec![]).unwrap();
        assert!(
            md_chunks.len() <= 3 && !md_chunks.is_empty(),
            "AC3: markdown 走自己的 policy (50 行/chunk → 约 2 chunks, got {})",
            md_chunks.len()
        );
    }

    // ---- TEST-2.3.4 / SCEN-2.3.4 (AC4) — 大文件分块不爆 ----
    #[test]
    fn test_2_3_4_large_file_does_not_explode() {
        // 10k 行 log（合理大文件；scanner 拦截 > 大小上限的超大文件）
        let big_body: String = (1..=10_000)
            .map(|i| format!("line {} of synthetic log entry", i))
            .collect::<Vec<_>>()
            .join("\n");
        let units = vec![unit("log", 1, 10_000, &big_body, None)];

        let mut policy = ChunkPolicy::default();
        policy.log = ChunkConfig {
            max_chunk_lines: 200,
            overlap_lines: 0,
            respect_parsed_units: false,
        };
        let chunks = chunk_units(&units, Path::new("big.log"), &policy, vec![]).unwrap();

        assert!(
            chunks.len() >= 40,
            "AC4: 10000 行 / 200 → ≥40 个 chunk (got {})",
            chunks.len()
        );
        let mut prev_end = 0usize;
        for c in &chunks {
            // 单调递增 + 非重叠（overlap=0）
            assert!(
                c.line_start > prev_end,
                "AC4: 行号单调 (prev_end={}, c.line_start={})",
                prev_end,
                c.line_start
            );
            assert!(c.line_end >= c.line_start, "AC4: 区间良态");
            let n_lines = c.content.lines().count();
            assert!(n_lines <= 200, "AC4: chunk 行数 ≤ max_chunk_lines (got {})", n_lines);
            prev_end = c.line_end;
        }
    }

    // ---- TEST-2.3.5 / SCEN-2.3.5 (AC5) — content_hash 一致性 ----
    #[test]
    fn test_2_3_5_content_hash_consistent_across_sources() {
        let src = "hello world\nthis is content\n";
        let h1 = content_hash(src);
        let h2 = content_hash(src);
        assert_eq!(h1, h2, "AC5: 同内容同 hash");
        assert!(h1.starts_with("fnv1a64:"), "AC5: algo 前缀");

        // CRLF / LF 归一
        let crlf = "hello world\r\nthis is content\r\n";
        assert_eq!(content_hash(crlf), h1, "AC5: CRLF→LF 归一化等价");

        // 行末 trailing whitespace 折叠
        let trailing = "hello world   \nthis is content   \n";
        assert_eq!(content_hash(trailing), h1, "AC5: 行末空白归一化等价");

        // 内容不同 → hash 不同（戳穿 RED stub 常量 hash）
        let different = "totally different content";
        assert_ne!(
            content_hash(different),
            h1,
            "AC5: 不同内容 hash 必不同"
        );

        // 跨 file_path / provenance：同内容 Chunk.content_hash 必同
        let units = vec![unit("text", 1, 2, src, None)];
        let c1 = chunk_units(
            &units,
            Path::new("/path/a.txt"),
            &ChunkPolicy::default(),
            vec![Provenance {
                importer: "imp1".into(),
                original_path: "x".into(),
                imported_at: "t".into(),
                source_modified_at: "t".into(),
            }],
        )
        .unwrap();
        let c2 = chunk_units(
            &units,
            Path::new("/different/b.txt"),
            &ChunkPolicy::default(),
            vec![Provenance {
                importer: "imp2".into(),
                original_path: "y".into(),
                imported_at: "t".into(),
                source_modified_at: "t".into(),
            }],
        )
        .unwrap();
        assert!(!c1.is_empty() && !c2.is_empty(), "AC5: 应产 chunk");
        assert_eq!(
            c1[0].content_hash, c2[0].content_hash,
            "AC5: 同内容跨 file_path / provenance 的 Chunk.content_hash 必相同"
        );
    }
}
