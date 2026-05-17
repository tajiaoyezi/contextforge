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

/// Honest stub (pre-NEEDS-DEP rebase per PR#6 review).
/// - parse_file: always returns **one** unit with **actual** file content + **actual** line count (no fabricated content/line numbers/kinds).
/// - Language normalized to canonical names per §5.3 (e.g. "rs" → "rust").
/// - Real tree-sitter/pulldown-cmark will replace this after the chore-dep PR + rebase.
/// - This is the correct provenance for task-2.3 until then.
pub fn parse_file(path: &Path) -> Result<Vec<ParsedUnit>, ParseError> {
    // Basic size guard (FIX-6)
    const MAX_SIZE: u64 = 100 * 1024 * 1024; // 100MB
    let meta = std::fs::metadata(path)?;
    if meta.len() > MAX_SIZE {
        return Err(ParseError::Other(format!("file too large for parser stub (> {} bytes), skipped", MAX_SIZE)));
    }

    let content = std::fs::read_to_string(path)?;
    let line_count = content.lines().count().max(1);

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let language = match ext {
        "go" => "go".to_string(),
        "rs" => "rust".to_string(),
        "py" => "python".to_string(),
        "ts" | "tsx" => "typescript".to_string(),
        "js" | "jsx" => "javascript".to_string(),
        "md" | "markdown" => "markdown".to_string(),
        "log" | "jsonl" => "log".to_string(),
        "json" => "json".to_string(),
        "yaml" | "yml" => "yaml".to_string(),
        "toml" => "toml".to_string(),
        "txt" | "" => "text".to_string(),
        other => other.to_string(),
    };

    Ok(vec![ParsedUnit {
        language,
        line_start: 1,
        line_end: line_count,
        content,
        kind: Some("file".to_string()),
        metadata: HashMap::new(),
    }])
}

/// parse_content kept for test convenience (no disk IO). Also normalizes language.
pub fn parse_content(_path: &Path, source: &str, language_hint: &str) -> Result<Vec<ParsedUnit>, ParseError> {
    let line_count = source.lines().count().max(1);
    // Normalize common aliases
    let language = match language_hint {
        "rs" => "rust",
        "py" => "python",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        other => other,
    }.to_string();

    Ok(vec![ParsedUnit {
        language,
        line_start: 1,
        line_end: line_count,
        content: source.to_string(),
        kind: Some("content".to_string()),
        metadata: HashMap::new(),
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    // Honest state per PR#6 review (FIX-2/3/5).
    // Strict AC1/AC2/AC3 tests are ignored until real parser lands.

    // TEST-2.2.1 / SCEN-2.2.1 (AC1)
    #[test]
    #[ignore = "pending NEEDS-DEP-task-2.2 tree-sitter/pulldown-cmark rebase (PR#6 review)"]
    fn test_2_2_1_code_parses_with_language_and_range() {
        let src = "fn main() {}\nstruct Foo {}";
        let units = parse_content(std::path::Path::new("main.rs"), src, "rust").expect("parse");
        assert!(!units.is_empty());
        assert!(units.len() > 1, "AC1: real parser must split >1 units");
        assert!(units.iter().any(|u| u.kind.as_deref() == Some("function")), "AC1: must detect function");
    }

    // TEST-2.2.2 / SCEN-2.2.2 (AC2)
    #[test]
    #[ignore = "pending NEEDS-DEP-task-2.2 tree-sitter/pulldown-cmark rebase (PR#6 review)"]
    fn test_2_2_2_markdown_detects_structure() {
        let src = "# Title\n\n```rust\nfn x(){}\n```\n\npara";
        let units = parse_content(std::path::Path::new("doc.md"), src, "markdown").expect("parse");
        assert!(units.iter().any(|u| u.kind.as_deref() == Some("heading")), "AC2: must detect heading");
        assert!(units.iter().any(|u| u.kind.as_deref() == Some("code_block")), "AC2: must detect code_block");
    }

    // TEST-2.2.3 / SCEN-2.2.3 (AC3)
    #[test]
    #[ignore = "pending NEEDS-DEP-task-2.2 tree-sitter/pulldown-cmark rebase (PR#6 review)"]
    fn test_2_2_3_log_and_jsonl() {
        let src = "2026-05-18 ERROR something\n{\"level\":\"info\"}";
        let units = parse_content(std::path::Path::new("app.log"), src, "log").expect("parse");
        assert!(units.len() >= 2, "AC3: real parser must split into >=2 records");
    }

    // TEST-2.2.4 / SCEN-2.2.4 (AC4) — passes with honest stub
    #[test]
    fn test_2_2_4_unknown_ext_falls_back_to_text() {
        let src = "random content";
        let units = parse_content(std::path::Path::new("data.bin"), src, "text").expect("fallback");
        assert_eq!(units[0].language, "text");
    }

    // TEST-2.2.5 / SCEN-2.2.5 (AC5) + parse_file coverage (FIX-5)
    #[test]
    fn test_2_2_5_language_label_is_retained() {
        let src = "hello";
        let units_c = parse_content(std::path::Path::new("x.py"), src, "python").unwrap();
        assert_eq!(units_c[0].language, "python");

        // Exercise the main entry parse_file (was missing coverage)
        use std::io::Write;
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("cf-test-{}.py", std::process::id()));
        { let mut f = std::fs::File::create(&tmp).unwrap(); f.write_all(src.as_bytes()).unwrap(); }
        let units_f = parse_file(&tmp).expect("parse_file");
        assert_eq!(units_f[0].language, "python", "AC5 + FIX-5: parse_file must return canonical name");
        let _ = std::fs::remove_file(&tmp);
    }

    // Explicit parse_file error path test (FIX-5)
    #[test]
    fn test_parse_file_io_error() {
        let bad = std::path::Path::new("/no/such/path/ever.rs");
        let e = parse_file(bad).unwrap_err();
        match e {
            ParseError::Io(_) => {}
            _ => panic!("expected Io error"),
        }
    }
}
