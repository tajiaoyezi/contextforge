//! task-2.2: Real parser — tree-sitter (code: go/rs/py/ts/tsx/js/jsx) + pulldown-cmark (Markdown)
//! + line/JSONL (logs). Replaces honest stub post PR#11 (deps). Matches §5.3 contract exactly.
//! AC1-3 implementation; TEST-2.2.1-3 un-ignored in follow-up commit.

use std::collections::HashMap;
use std::path::Path;

/// Retained for task-1.3 core_skeleton.rs test anchor (AC4 wiring). Returns true.
pub fn placeholder_ready() -> bool {
    true
}

// tree-sitter 0.26.8 + language crates (exact pins from core/Cargo.toml post chore/dep-parser-crates)
use tree_sitter::Parser;
use tree_sitter_go;
use tree_sitter_rust;
use tree_sitter_python;
use tree_sitter_typescript;
use tree_sitter_javascript;

// pulldown-cmark 0.13.3
use pulldown_cmark::{Parser as MdParser, Event, Tag, CodeBlockKind, Options};

// thiserror 2.0.18 (direct dep per PR#11; derive form matches task spec §5.3)
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUnit {
    pub language: String,
    pub line_start: usize,
    pub line_end: usize,
    pub content: String,
    pub kind: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported language for {path:?}: {lang}")]
    Unsupported { path: std::path::PathBuf, lang: String },
    #[error("parse failed: {0}")]
    Other(String),
}

/// Single source of truth for language canonicalization (FIX-R2 per PR#6 round-2 review).
/// Both parse_file and parse_content must use this to prevent future drift (AC5 / R8).
fn canonicalize_language(hint: &str) -> String {
    match hint {
        "go" => "go",
        "rs" => "rust",
        "py" => "python",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "md" | "markdown" => "markdown",
        "log" | "jsonl" => "log",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "txt" | "" => "text",
        other => other,
    }.to_string()
}

// -----------------------------------------------------------------------------
// Real implementation (AC1-3) — tree-sitter + pulldown-cmark + log/JSONL
// -----------------------------------------------------------------------------

/// Build 0-based byte offsets for start of each line (for byte→line conversion).
fn build_line_offsets(source: &str) -> Vec<usize> {
    let mut offs = vec![0usize];
    for (i, c) in source.char_indices() {
        if c == '\n' {
            offs.push(i + 1);
        }
    }
    offs
}

/// Map [byte_start, byte_end) to 1-based (line_start, line_end).
fn byte_range_to_line_range(offsets: &[usize], start: usize, end: usize) -> (usize, usize) {
    // partition_point requires Rust 1.52+ (our edition 2021 ok)
    let ls = offsets.partition_point(|&o| o <= start);
    let le = offsets.partition_point(|&o| o <= end.saturating_sub(1));
    let line_start = ls + 1;
    let line_end = le + 1;
    (line_start, line_end.max(line_start))
}

/// Normalize tree-sitter node kind to AC1 expected values ("function", "struct", ...).
fn normalize_code_kind(kind: &str) -> Option<String> {
    let k = kind.to_lowercase();
    if k.contains("function") || k == "function_item" || k.contains("method") {
        Some("function".to_string())
    } else if k.contains("struct") || k == "struct_item" {
        Some("struct".to_string())
    } else if k.contains("class") || k == "class_declaration" {
        Some("class".to_string())
    } else if k.contains("impl") {
        Some("impl".to_string())
    } else {
        None
    }
}

/// AC1: tree-sitter code parsing for P0 languages. Top-level named children only.
fn parse_code(source: &str, language: &str) -> Result<Vec<ParsedUnit>, ParseError> {
    let mut parser = Parser::new();
    let ts_lang = match language {
        "rust" => tree_sitter_rust::LANGUAGE.into(),
        "go" => tree_sitter_go::LANGUAGE.into(),
        "python" => tree_sitter_python::LANGUAGE.into(),
        "typescript" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "javascript" => tree_sitter_javascript::LANGUAGE.into(),
        _ => return parse_text_fallback(source, language),
    };
    parser.set_language(&ts_lang).map_err(|e| ParseError::Other(format!("set_language: {}", e)))?;
    let tree = parser.parse(source, None).ok_or_else(|| ParseError::Other("tree-sitter parse failed".into()))?;
    let root = tree.root_node();
    let mut units = Vec::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if !child.is_named() { continue; }
        if let Some(kind) = normalize_code_kind(child.kind()) {
            let start = child.start_position();
            let end = child.end_position();
            let (ls, le) = (start.row + 1, end.row + 1);
            let content = source[child.byte_range()].to_string();
            units.push(ParsedUnit {
                language: language.to_string(),
                line_start: ls,
                line_end: le,
                content,
                kind: Some(kind),
                metadata: HashMap::new(),
            });
        }
    }
    if units.is_empty() {
        // graceful: at least one unit for the whole source (prevents empty results on edge files)
        let line_count = source.lines().count().max(1);
        units.push(ParsedUnit {
            language: language.to_string(),
            line_start: 1,
            line_end: line_count,
            content: source.to_string(),
            kind: Some("code".to_string()),
            metadata: HashMap::new(),
        });
    }
    Ok(units)
}

/// Crude JSONL top-level key extractor (no serde_json dep — R7 compliance).
/// Returns comma-joined keys on success, None on non-object or parse failure.
fn extract_json_top_keys(line: &str) -> Option<String> {
    let t = line.trim();
    if !t.starts_with('{') || !t.ends_with('}') { return None; }
    let mut keys = Vec::new();
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            let mut k = String::new();
            let mut closed = false;
            while let Some(nc) = chars.next() {
                if nc == '"' { closed = true; break; }
                k.push(nc);
            }
            if !closed { continue; }
            // skip whitespace + expect ':'
            while let Some(&nc) = chars.peek() {
                if nc == ':' {
                    if !k.is_empty() { keys.push(k.clone()); }
                    break;
                }
                if nc.is_whitespace() { chars.next(); continue; }
                break;
            }
        }
    }
    if keys.is_empty() { None } else { Some(keys.join(",")) }
}

/// AC3: .log/.txt → per-line log_entry; .jsonl → try json key extract + degrade to line on fail.
fn parse_log(source: &str, language: &str) -> Result<Vec<ParsedUnit>, ParseError> {
    let mut units = Vec::new();
    for (i, line) in source.lines().enumerate() {
        let lnum = i + 1;
        if line.trim().is_empty() { continue; }
        let mut md = HashMap::new();
        if line.trim_start().starts_with('{') {
            if let Some(ks) = extract_json_top_keys(line) {
                md.insert("json_keys".to_string(), ks);
            }
        }
        units.push(ParsedUnit {
            language: language.to_string(),
            line_start: lnum,
            line_end: lnum,
            content: line.to_string(),
            kind: Some("log_entry".to_string()),
            metadata: md,
        });
    }
    if units.is_empty() {
        units.push(ParsedUnit {
            language: language.to_string(),
            line_start: 1,
            line_end: 1,
            content: source.to_string(),
            kind: Some("log_entry".to_string()),
            metadata: HashMap::new(),
        });
    }
    Ok(units)
}

/// AC2: pulldown-cmark with byte offsets → line ranges via prebuilt table.
/// Emits Heading (with level), CodeBlock (with lang), Paragraph.
fn parse_markdown(source: &str, language: &str) -> Result<Vec<ParsedUnit>, ParseError> {
    let line_offsets = build_line_offsets(source);
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    let parser = MdParser::new_ext(source, opts);
    let mut units = Vec::new();
    for (ev, range) in parser.into_offset_iter() {
        let (kind, meta) = match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                let mut m = HashMap::new();
                m.insert("level".to_string(), (level as u8).to_string());
                (Some("heading".to_string()), m)
            }
            Event::Start(Tag::CodeBlock(cb)) => {
                let mut m = HashMap::new();
                if let CodeBlockKind::Fenced(info) = cb {
                    let lang = info.split_whitespace().next().unwrap_or("").to_string();
                    if !lang.is_empty() { m.insert("lang".to_string(), lang); }
                }
                (Some("code_block".to_string()), m)
            }
            Event::Start(Tag::Paragraph) => (Some("paragraph".to_string()), HashMap::new()),
            _ => continue,
        };
        if let Some(k) = kind {
            let (ls, le) = byte_range_to_line_range(&line_offsets, range.start, range.end);
            let content = source[range].trim().to_string();
            units.push(ParsedUnit {
                language: language.to_string(),
                line_start: ls,
                line_end: le,
                content,
                kind: Some(k),
                metadata: meta,
            });
        }
    }
    if units.is_empty() {
        let lc = source.lines().count().max(1);
        units.push(ParsedUnit {
            language: language.to_string(),
            line_start: 1, line_end: lc,
            content: source.to_string(),
            kind: Some("markdown".to_string()),
            metadata: HashMap::new(),
        });
    }
    Ok(units)
}

/// Fallback for unknown / text (AC4/AC5). Single unit covering whole input.
fn parse_text_fallback(source: &str, language: &str) -> Result<Vec<ParsedUnit>, ParseError> {
    let lc = source.lines().count().max(1);
    Ok(vec![ParsedUnit {
        language: language.to_string(),
        line_start: 1,
        line_end: lc,
        content: source.to_string(),
        kind: Some("text".to_string()),
        metadata: HashMap::new(),
    }])
}

/// Main entry (file path + auto language from ext). Size guard + delegate.
pub fn parse_file(path: &Path) -> Result<Vec<ParsedUnit>, ParseError> {
    const MAX_SIZE: u64 = 100 * 1024 * 1024; // 100MB (FIX-6)
    let meta = std::fs::metadata(path)?;
    if meta.len() > MAX_SIZE {
        return Err(ParseError::Other(format!("file too large (> {} bytes), skipped", MAX_SIZE)));
    }
    let content = std::fs::read_to_string(path)?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    parse_content(path, &content, ext)
}

/// Content entry (for tests + explicit language). Dispatches by canonical language.
pub fn parse_content(_path: &Path, source: &str, language_hint: &str) -> Result<Vec<ParsedUnit>, ParseError> {
    let language = canonicalize_language(language_hint);
    match language.as_str() {
        "rust" | "go" | "python" | "typescript" | "javascript" => parse_code(source, &language),
        "markdown" => parse_markdown(source, &language),
        "log" | "json" => parse_log(source, &language),
        _ => parse_text_fallback(source, &language),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real impl (tree-sitter/pulldown-cmark/log) — AC1-3 now active post PR#11 rebase.
    // All 5 parser tests must pass with 0 ignored.

    // TEST-2.2.1 / SCEN-2.2.1 (AC1)
    #[test]
    fn test_2_2_1_code_parses_with_language_and_range() {
        let src = "fn main() {}\nstruct Foo {}";
        let units = parse_content(std::path::Path::new("main.rs"), src, "rust").expect("parse");
        assert!(!units.is_empty());
        assert!(units.len() > 1, "AC1: real parser must split >1 units");
        assert!(units.iter().any(|u| u.kind.as_deref() == Some("function")), "AC1: must detect function");
    }

    // TEST-2.2.2 / SCEN-2.2.2 (AC2)
    #[test]
    fn test_2_2_2_markdown_detects_structure() {
        let src = "# Title\n\n```rust\nfn x(){}\n```\n\npara";
        let units = parse_content(std::path::Path::new("doc.md"), src, "markdown").expect("parse");
        assert!(units.iter().any(|u| u.kind.as_deref() == Some("heading")), "AC2: must detect heading");
        assert!(units.iter().any(|u| u.kind.as_deref() == Some("code_block")), "AC2: must detect code_block");
    }

    // TEST-2.2.3 / SCEN-2.2.3 (AC3)
    #[test]
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
