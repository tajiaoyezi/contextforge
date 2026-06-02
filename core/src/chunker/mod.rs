//! task-2.3 (Phase 2): chunker — chunking + metadata 抽取 + provenance 维护.
//!
//! - `chunk_units`: 按 ChunkPolicy.<lang> 切片；respect_parsed_units=true 时尽量按
//!   ParsedUnit 边界保留 1:1 映射；否则按 max_chunk_lines 定长 + overlap 切。
//! - `chunk_file`: parser::parse_file → chunk_units 串接。
//! - `content_hash`: sha256（chore PR #17 加 sha2 v0.11.0；与 task-3.1 importer
//!   一致 — 跨模块 Phase 5 memoryops 去重锚点统一）+ normalize 最小集（CRLF→LF +
//!   行末 trailing whitespace + 整体 trim）→ "sha256:<64-hex>"（algo-prefix 保留
//!   forward-compat）。
//!
//! Rework (2026-05-21): SPEC-DRIFT 裁决后 content_hash 从 FNV-1a-64 → sha256。
//! 后续 task-2.4 (indexer) 消费 Vec<Chunk>。

use std::collections::HashMap;
use std::path::Path;

use sha2::{Digest, Sha256};
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

/// 把 ParsedUnit.language 映射到 ChunkPolicy 的子配置（与 parser 的语言派发一致）。
fn select_config<'a>(lang: &str, policy: &'a ChunkPolicy) -> &'a ChunkConfig {
    match lang {
        "go" | "rust" | "python" | "typescript" | "javascript" => &policy.code,
        "markdown" => &policy.markdown,
        "log" | "json" => &policy.log,
        _ => &policy.text,
    }
}

/// 从 content_hash 构造 chunk_id："chk_<8-hex-prefix>_<ordinal>"。
fn make_chunk_id(hash: &str, ordinal: usize) -> String {
    let prefix = hash
        .split(':')
        .nth(1)
        .map(|h| &h[..8.min(h.len())])
        .unwrap_or("00000000");
    format!("chk_{}_{}", prefix, ordinal)
}

/// 主入口（AC1/AC2/AC3/AC4/AC5）：把 parser 产出的解析单元切片为 Chunk。
pub fn chunk_units(
    units: &[ParsedUnit],
    file_path: &Path,
    policy: &ChunkPolicy,
    provenance: Vec<Provenance>,
) -> Result<Vec<Chunk>, ChunkError> {
    let file_path_str = file_path.to_string_lossy().to_string();
    let mut chunks: Vec<Chunk> = Vec::new();
    let mut ordinal: usize = 0;

    for unit in units {
        let cfg = select_config(&unit.language, policy);
        if cfg.max_chunk_lines == 0 {
            return Err(ChunkError::InvalidConfig(format!(
                "max_chunk_lines must be > 0 (language={})",
                unit.language
            )));
        }

        // 单 ParsedUnit 的行数（按 '\n' 计数；保留 parser 给的 line_start 起点用于 file-relative line 区间）
        let unit_lines: Vec<&str> = unit.content.split('\n').collect();
        let n_lines = unit_lines.len();

        // 路径一：尽量保留 ParsedUnit 边界 → 1 unit = 1 chunk（kind 透传）
        if cfg.respect_parsed_units && n_lines <= cfg.max_chunk_lines {
            let content = unit.content.clone();
            let hash = content_hash(&content);
            chunks.push(Chunk {
                chunk_id: make_chunk_id(&hash, ordinal),
                file_path: file_path_str.clone(),
                line_start: unit.line_start,
                line_end: unit.line_end,
                language: unit.language.clone(),
                content,
                content_hash: hash,
                kind: unit.kind.clone(),
                provenance: provenance.clone(),
                metadata: unit.metadata.clone(),
            });
            ordinal += 1;
            continue;
        }

        // 路径二：按 max_chunk_lines 定长切（含 overlap） — AC3 / AC4
        // step 必 ≥ 1，避免无限循环：overlap < max 时 step = max - overlap，否则 step = max
        let overlap = cfg.overlap_lines.min(cfg.max_chunk_lines.saturating_sub(1));
        let step = cfg.max_chunk_lines - overlap;
        let step = if step == 0 { cfg.max_chunk_lines } else { step };

        let mut i: usize = 0;
        while i < n_lines {
            let end = (i + cfg.max_chunk_lines).min(n_lines);
            let segment = unit_lines[i..end].join("\n");
            // file-relative line numbers (unit.line_start 是该 ParsedUnit 的起始行号)
            let line_start = unit.line_start + i;
            let line_end = unit.line_start + end - 1;
            let hash = content_hash(&segment);
            chunks.push(Chunk {
                chunk_id: make_chunk_id(&hash, ordinal),
                file_path: file_path_str.clone(),
                line_start,
                line_end,
                language: unit.language.clone(),
                content: segment,
                content_hash: hash,
                kind: None, // 拆分后不再承诺 unit.kind 语义
                provenance: provenance.clone(),
                metadata: unit.metadata.clone(),
            });
            ordinal += 1;
            if end >= n_lines {
                break;
            }
            i += step;
        }
    }

    Ok(chunks)
}

/// 便利入口（AC1/AC2/AC3）：parser::parse_file → chunk_units 串接。
pub fn chunk_file(
    path: &Path,
    policy: &ChunkPolicy,
    provenance: Vec<Provenance>,
) -> Result<Vec<Chunk>, ChunkError> {
    let units = crate::parser::parse_file(path).map_err(|e| ChunkError::Parse(e.to_string()))?;
    chunk_units(&units, path, policy, provenance)
}

/// AC5：normalize content for hashing — CRLF→LF + 行末 trailing whitespace + 整体 trim。
/// 不归一化 leading whitespace / 内部空行 — 这些影响代码语义；行末空白与 CRLF 不影响。
fn normalize_for_hash(s: &str) -> String {
    // 1. CRLF → LF
    let s = s.replace("\r\n", "\n");
    // 2. 行末 trailing whitespace（每行）
    let trimmed_lines: Vec<&str> = s.split('\n').map(|l| l.trim_end()).collect();
    let joined = trimmed_lines.join("\n");
    // 3. 整体 trim（首尾空白）
    joined.trim().to_string()
}

/// 公开：算 content_hash（memoryops 去重锚点；AC5 跨来源一致）。
///
/// 算法 = sha256（与 task-3.1 importer `internal/importer/record.go:80`
/// `sha256.Sum256` 一致，跨模块 Phase 5 memoryops 去重锚点统一）。返回
/// `sha256:<64-hex>`（algo-prefix 保留 forward-compat — Phase 5 memoryops 按前缀
/// 分流即可与任一模块对接）。
pub fn content_hash(content: &str) -> String {
    let normalized = normalize_for_hash(content);
    let digest = Sha256::digest(normalized.as_bytes());
    // sha2 0.11 / digest 0.11 — 输出 hex 用逐字节格式化（不依赖 hex crate）
    let mut hex_str = String::with_capacity(64);
    for byte in digest.iter() {
        use std::fmt::Write;
        write!(hex_str, "{:02x}", byte).expect("write to String never fails");
    }
    format!("sha256:{}", hex_str)
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
                c.content_hash.starts_with("sha256:"),
                "AC1: content_hash algo-prefixed (got '{}')",
                c.content_hash
            );
            // Hash hex part length: 64 chars (256-bit sha256)
            assert_eq!(c.content_hash.len(), "sha256:".len() + 64, "AC1: hash hex length");
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
        let policy_a = ChunkPolicy {
            code: ChunkConfig {
                max_chunk_lines: 50,
                overlap_lines: 0,
                respect_parsed_units: false,
            },
            ..Default::default()
        };
        // policy B: code 100 行/chunk
        let policy_b = ChunkPolicy {
            code: ChunkConfig {
                max_chunk_lines: 100,
                overlap_lines: 0,
                respect_parsed_units: false,
            },
            ..Default::default()
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
        let policy_c = ChunkPolicy {
            code: ChunkConfig {
                max_chunk_lines: 10,
                overlap_lines: 0,
                respect_parsed_units: false,
            },
            markdown: ChunkConfig {
                max_chunk_lines: 50,
                overlap_lines: 0,
                respect_parsed_units: false,
            },
            ..Default::default()
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

        let policy = ChunkPolicy {
            log: ChunkConfig {
                max_chunk_lines: 200,
                overlap_lines: 0,
                respect_parsed_units: false,
            },
            ..Default::default()
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
        assert!(h1.starts_with("sha256:"), "AC5: algo 前缀");
        // sha256 hex length = 64 + "sha256:".len() (7) = 71
        assert_eq!(h1.len(), "sha256:".len() + 64, "AC5: sha256 hex 长度 = 64");

        // CRLF / LF 归一
        let crlf = "hello world\r\nthis is content\r\n";
        assert_eq!(content_hash(crlf), h1, "AC5: CRLF→LF 归一化等价");

        // 行末 trailing whitespace 折叠
        let trailing = "hello world   \nthis is content   \n";
        assert_eq!(content_hash(trailing), h1, "AC5: 行末空白归一化等价");

        // 内容不同 → hash 不同
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
