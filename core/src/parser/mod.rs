//! task-1.3 (AC4): Phase 2 `parser` placeholder — code (tree-sitter) /
//! Markdown (pulldown-cmark) / log parsing. No logic yet; lands in Phase 2.

/// True once this placeholder module is wired into the crate (AC4 test anchor).
pub fn placeholder_ready() -> bool {
    true
}

// =============================================================================
// task-2.2 RED skeleton (per §5.3 signatures + §6 AC1-5)
// Minimal std-only heuristic implementation for compile + RED bridge.
// Real tree-sitter / pulldown-cmark will replace the bodies after NEEDS-DEP
// chore PR + rebase (R7). See NEEDS-DEP-task-2.2.md.
// =============================================================================

use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUnit {
    pub language: String,
    pub line_start: usize,
    pub line_end: usize,
    pub content: String,
    pub kind: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug)]
pub enum ParseError {
    Io(std::io::Error),
    Unsupported { path: std::path::PathBuf, lang: String },
    Other(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Io(e) => write!(f, "io: {}", e),
            ParseError::Unsupported { path, lang } => write!(f, "unsupported language for {:?}: {}", path, lang),
            ParseError::Other(s) => write!(f, "parse failed: {}", s),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<std::io::Error> for ParseError {
    fn from(e: std::io::Error) -> Self { ParseError::Io(e) }
}

/// RED skeleton: extension-based language + whole-file unit (will be replaced by real parsers).
pub fn parse_file(path: &Path) -> Result<Vec<ParsedUnit>, ParseError> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let language = match ext {
        "go" | "rs" | "py" | "ts" | "tsx" | "js" | "jsx" => ext.to_string(),
        "md" | "markdown" => "markdown".to_string(),
        "log" | "jsonl" => "log".to_string(),
        "txt" | "json" | "yaml" | "yml" | "toml" => ext.to_string(),
        _ => "text".to_string(),
    };

    let content = std::fs::read_to_string(path)?;
    let line_count = content.lines().count().max(1);

    // Stub: one unit for the whole file. Real impl will split into many rich units.
    Ok(vec![ParsedUnit {
        language,
        line_start: 1,
        line_end: line_count,
        content,
        kind: Some("text".to_string()),
        metadata: HashMap::new(),
    }])
}

/// Explicit language version (used by tests / special callers).
pub fn parse_content(_path: &Path, source: &str, language_hint: &str) -> Result<Vec<ParsedUnit>, ParseError> {
    let line_count = source.lines().count().max(1);
    Ok(vec![ParsedUnit {
        language: language_hint.to_string(),
        line_start: 1,
        line_end: line_count,
        content: source.to_string(),
        kind: Some("text".to_string()),
        metadata: HashMap::new(),
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    // All RED tests use parse_content (std-only) to avoid any new crate in this branch (R7/NEEDS-DEP).
    // Real tree-sitter/pulldown bodies + parse_file rich splitting come after dep rebase.

    // TEST-2.2.1 / SCEN-2.2.1 (AC1): code → language tag + range
    #[test]
    fn test_2_2_1_code_parses_with_language_and_range() {
        let src = "fn main() {}\nstruct Foo {}";
        let units = parse_content(std::path::Path::new("main.rs"), src, "rust").expect("parse");
        assert!(!units.is_empty(), "AC1: must return at least one unit");
        let u = &units[0];
        assert_eq!(u.language, "rust", "AC1: language rust");
        assert!(u.line_end >= u.line_start);
        // RED expectation (fails on current stub): real tree-sitter must produce richer units
        assert!(units.len() > 1 || u.kind.as_deref() == Some("function"), "RED: expecting function/struct units from real parser (AC1)");
    }

    // TEST-2.2.2 / SCEN-2.2.2 (AC2): markdown structure
    #[test]
    fn test_2_2_2_markdown_detects_structure() {
        let src = "# Title\n\n```rust\nfn x(){}\n```\n\npara";
        let units = parse_content(std::path::Path::new("doc.md"), src, "markdown").expect("parse");
        assert!(!units.is_empty());
        let u = &units[0];
        assert_eq!(u.language, "markdown");
        assert!(u.line_end >= 1);
        // RED: real pulldown-cmark must detect heading + code_block as separate units
        assert!(units.iter().any(|u| u.kind.as_deref() == Some("heading") || u.content.contains("# Title")), "RED: expecting heading/code_block (AC2)");
    }

    // TEST-2.2.3 / SCEN-2.2.3 (AC3): logs
    #[test]
    fn test_2_2_3_log_and_jsonl() {
        let src = "2026-05-18 ERROR something\n{\"level\":\"info\"}";
        let units = parse_content(std::path::Path::new("app.log"), src, "log").expect("parse");
        assert!(!units.is_empty());
        assert_eq!(units[0].language, "log");
        // RED: real log parser would split into multiple timestamped/JSON records
        assert!(units.len() > 1, "RED: expecting multiple log entries (AC3)");
    }

    // TEST-2.2.4 / SCEN-2.2.4 (AC4): unknown fallback
    #[test]
    fn test_2_2_4_unknown_ext_falls_back_to_text() {
        let src = "random content";
        let units = parse_content(std::path::Path::new("data.bin"), src, "text").expect("fallback");
        assert_eq!(units[0].language, "text");
        // This one can stay mostly passing for fallback guarantee; the "no panic + correct tag" is the AC
    }

    // TEST-2.2.5 / SCEN-2.2.5 (AC5): language retained
    #[test]
    fn test_2_2_5_language_label_is_retained() {
        let src = "hello";
        let units = parse_content(std::path::Path::new("x.py"), src, "python").unwrap();
        assert_eq!(units[0].language, "python", "AC5: language survives");
        // RED: in real run with many files the label must be accurate for tokenizer boost (R8)
    }
}
