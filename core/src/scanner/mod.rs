//! Phase 2 scanner: file discovery, denylist/allowlist filtering, and
//! secret redaction before parser/chunker/indexer consume local content.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

pub const DEFAULT_MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanOptions {
    pub denylist: Vec<String>,
    pub allowlist: Vec<PathBuf>,
    pub allow_denylist_override: bool,
    pub dry_run: bool,
    pub max_file_bytes: u64,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            denylist: default_denylist(),
            allowlist: Vec::new(),
            allow_denylist_override: false,
            dry_run: false,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanReport {
    pub root: PathBuf,
    pub files: Vec<ScannedFile>,
    pub skipped: Vec<SkippedPath>,
    pub redaction_hits: Vec<SecretMatch>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFile {
    pub path: PathBuf,
    pub original_size_bytes: u64,
    pub content: Option<String>,
    pub redacted_content: Option<String>,
    pub redaction_status: RedactionStatus,
    pub redactions: Vec<SecretMatch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionStatus {
    None,
    Partial,
    Full,
}

impl RedactionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RedactionStatus::None => "none",
            RedactionStatus::Partial => "partial",
            RedactionStatus::Full => "full",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecretKind {
    ApiKey,
    BearerToken,
    PrivateKey,
    AwsAccessKey,
    AwsSecretKey,
    GithubToken,
    Password,
    Cookie,
}

impl SecretKind {
    fn label(self) -> &'static str {
        match self {
            SecretKind::ApiKey => "[REDACTED:API_KEY]",
            SecretKind::BearerToken => "[REDACTED:BEARER_TOKEN]",
            SecretKind::PrivateKey => "[REDACTED:PRIVATE_KEY]",
            SecretKind::AwsAccessKey => "[REDACTED:AWS_ACCESS_KEY]",
            SecretKind::AwsSecretKey => "[REDACTED:AWS_SECRET_KEY]",
            SecretKind::GithubToken => "[REDACTED:GITHUB_TOKEN]",
            SecretKind::Password => "[REDACTED:PASSWORD]",
            SecretKind::Cookie => "[REDACTED:COOKIE]",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretMatch {
    pub kind: SecretKind,
    pub path: Option<PathBuf>,
    pub line: usize,
    pub start: usize,
    pub end: usize,
    pub redaction: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedPath {
    pub path: PathBuf,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    Denylisted(String),
    NotAllowlisted,
    TooLarge { size: u64, max: u64 },
    NotUtf8,
    Symlink,
}

#[derive(Debug)]
pub enum ScanError {
    Io { path: PathBuf, source: io::Error },
    DenylistOverrideRequired,
    NotAllowlisted,
    FileTooLarge { path: PathBuf, size: u64, max: u64 },
    Symlink { path: PathBuf },
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScanError::Io { path, source } => {
                write!(f, "scanner I/O error at {}: {source}", path.display())
            }
            ScanError::DenylistOverrideRequired => {
                write!(f, "denylist override requires explicit confirmation")
            }
            ScanError::NotAllowlisted => write!(f, "path is outside configured allowlist"),
            ScanError::FileTooLarge { path, size, max } => write!(
                f,
                "scanner file too large at {}: {size} bytes exceeds {max} bytes",
                path.display()
            ),
            ScanError::Symlink { path } => write!(f, "scanner refuses symlink {}", path.display()),
        }
    }
}

impl Error for ScanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ScanError::Io { source, .. } => Some(source),
            ScanError::DenylistOverrideRequired
            | ScanError::NotAllowlisted
            | ScanError::FileTooLarge { .. }
            | ScanError::Symlink { .. } => None,
        }
    }
}

pub fn default_denylist() -> Vec<String> {
    [
        ".env",
        ".env.*",
        "*.pem",
        "*.key",
        "*.p12",
        "*.pfx",
        "id_rsa",
        "id_ed25519",
        ".ssh/",
        ".git/objects/",
        "node_modules/",
        "target/",
        "dist/",
        "build/",
        ".cache/",
        "vendor/",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

pub fn scan_path(root: impl AsRef<Path>, options: &ScanOptions) -> Result<ScanReport, ScanError> {
    let root = root.as_ref().to_path_buf();
    let mut report = ScanReport {
        root: root.clone(),
        files: Vec::new(),
        skipped: Vec::new(),
        redaction_hits: Vec::new(),
        dry_run: options.dry_run,
    };
    walk(&root, options, &mut report)?;
    report.files.sort_by(|a, b| a.path.cmp(&b.path));
    report.skipped.sort_by(|a, b| a.path.cmp(&b.path));
    report.redaction_hits.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.line.cmp(&b.line))
            .then(a.start.cmp(&b.start))
            .then(a.kind.cmp(&b.kind))
    });
    Ok(report)
}

pub fn scan_file(path: impl AsRef<Path>, options: &ScanOptions) -> Result<ScannedFile, ScanError> {
    let path = path.as_ref();
    let metadata = fs::metadata(path).map_err(|source| ScanError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.len() > options.max_file_bytes {
        return Ok(ScannedFile {
            path: path.to_path_buf(),
            original_size_bytes: metadata.len(),
            content: None,
            redacted_content: None,
            redaction_status: RedactionStatus::None,
            redactions: Vec::new(),
        });
    }

    let content = fs::read_to_string(path).map_err(|source| ScanError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(scan_content(
        path.to_path_buf(),
        metadata.len(),
        content,
        options.dry_run,
    ))
}

pub fn detect_secrets(content: &str) -> Vec<SecretMatch> {
    let (_, _, matches) = redact_content(content);
    matches
}

pub fn redact_content(content: &str) -> (String, RedactionStatus, Vec<SecretMatch>) {
    let spans = find_secret_spans(content);
    if spans.is_empty() {
        return (content.to_string(), RedactionStatus::None, Vec::new());
    }

    let mut redacted = content.to_string();
    for span in spans.iter().rev() {
        redacted.replace_range(span.start..span.end, span.kind.label());
    }
    let matches = spans
        .into_iter()
        .map(|span| SecretMatch {
            kind: span.kind,
            path: None,
            line: span.line,
            start: span.line_start,
            end: span.line_end,
            redaction: span.kind.label(),
        })
        .collect();
    (redacted, RedactionStatus::Partial, matches)
}

pub fn is_denied(path: impl AsRef<Path>, denylist: &[String]) -> Option<String> {
    let path = path.as_ref();
    let normalized = normalize_path(path);
    let components = path_components(path);
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    for pattern in denylist {
        let p = pattern.trim();
        if p.is_empty() {
            continue;
        }
        if p.ends_with('/') {
            let dir = p.trim_end_matches('/');
            if dir.contains('/') {
                if contains_component_sequence(&components, &path_components(Path::new(dir))) {
                    return Some(pattern.clone());
                }
            } else if components.iter().any(|c| c == dir) {
                return Some(pattern.clone());
            }
            continue;
        }
        if let Some(suffix) = p.strip_prefix('*') {
            if file_name.ends_with(suffix) {
                return Some(pattern.clone());
            }
            continue;
        }
        if p.ends_with(".*") {
            let prefix = p.trim_end_matches('*');
            if file_name.starts_with(prefix) {
                return Some(pattern.clone());
            }
            continue;
        }
        if file_name == p || normalized == p || normalized.ends_with(&format!("/{p}")) {
            return Some(pattern.clone());
        }
    }
    None
}

pub fn is_allowlisted(path: impl AsRef<Path>, allowlist: &[PathBuf]) -> bool {
    if allowlist.is_empty() {
        return true;
    }
    let path = normalize_lexical(path.as_ref());
    allowlist
        .iter()
        .map(|p| normalize_lexical(p))
        .any(|allowed| path == allowed || path.starts_with(&allowed))
}

/// Backward-compatible task-1.3 test anchor.
pub fn placeholder_ready() -> bool {
    true
}

fn walk(path: &Path, options: &ScanOptions, report: &mut ScanReport) -> Result<(), ScanError> {
    let mut entries = fs::read_dir(path)
        .map_err(|source| ScanError::Io {
            path: path.to_path_buf(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| ScanError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let p = entry.path();
        let metadata = entry.metadata().map_err(|source| ScanError::Io {
            path: p.clone(),
            source,
        })?;

        if let Some(pattern) = is_denied(&p, &options.denylist) {
            let explicit_override =
                !options.allowlist.is_empty() && is_allowlisted(&p, &options.allowlist);
            if explicit_override && !options.allow_denylist_override {
                return Err(ScanError::DenylistOverrideRequired);
            }
            if explicit_override && options.allow_denylist_override {
                // explicit override: continue through normal file/dir handling
            } else {
                mark_tree_skipped(&p, SkipReason::Denylisted(pattern), report)?;
                continue;
            }
        }

        if !options.allowlist.is_empty() && !is_allowlisted(&p, &options.allowlist) {
            if metadata.is_dir() && is_ancestor_of_allowlist(&p, &options.allowlist) {
                walk(&p, options, report)?;
            } else {
                mark_tree_skipped(&p, SkipReason::NotAllowlisted, report)?;
            }
            continue;
        }

        if metadata.is_dir() {
            walk(&p, options, report)?;
            continue;
        }
        if !metadata.is_file() {
            continue;
        }
        if metadata.len() > options.max_file_bytes {
            report.skipped.push(SkippedPath {
                path: p,
                reason: SkipReason::TooLarge {
                    size: metadata.len(),
                    max: options.max_file_bytes,
                },
            });
            continue;
        }

        match fs::read_to_string(&p) {
            Ok(content) => {
                let scanned = scan_content(p, metadata.len(), content, options.dry_run);
                report
                    .redaction_hits
                    .extend(scanned.redactions.iter().cloned());
                report.files.push(scanned);
            }
            Err(err) if err.kind() == io::ErrorKind::InvalidData => {
                report.skipped.push(SkippedPath {
                    path: p,
                    reason: SkipReason::NotUtf8,
                })
            }
            Err(source) => return Err(ScanError::Io { path: p, source }),
        }
    }
    Ok(())
}

fn scan_content(
    path: PathBuf,
    original_size_bytes: u64,
    content: String,
    dry_run: bool,
) -> ScannedFile {
    let (redacted, status, mut redactions) = redact_content(&content);
    for m in &mut redactions {
        m.path = Some(path.clone());
    }
    let (content, redacted_content) = match (dry_run, status) {
        (true, _) => (None, None),
        (false, RedactionStatus::None) => (Some(content), None),
        (false, _) => (None, Some(redacted)),
    };

    ScannedFile {
        path,
        original_size_bytes,
        content,
        redacted_content,
        redaction_status: status,
        redactions,
    }
}

fn mark_tree_skipped(
    path: &Path,
    reason: SkipReason,
    report: &mut ScanReport,
) -> Result<(), ScanError> {
    let metadata = fs::metadata(path).map_err(|source| ScanError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.is_file() {
        report.skipped.push(SkippedPath {
            path: path.to_path_buf(),
            reason,
        });
        return Ok(());
    }
    if !metadata.is_dir() {
        return Ok(());
    }

    let mut entries = fs::read_dir(path)
        .map_err(|source| ScanError::Io {
            path: path.to_path_buf(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| ScanError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    entries.sort_by_key(|e| e.path());
    if entries.is_empty() {
        report.skipped.push(SkippedPath {
            path: path.to_path_buf(),
            reason,
        });
        return Ok(());
    }
    for entry in entries {
        mark_tree_skipped(&entry.path(), reason.clone(), report)?;
    }
    Ok(())
}

fn is_ancestor_of_allowlist(path: &Path, allowlist: &[PathBuf]) -> bool {
    let path = normalize_lexical(path);
    allowlist
        .iter()
        .map(|p| normalize_lexical(p))
        .any(|allowed| allowed.starts_with(&path))
}

fn normalize_path(path: &Path) -> String {
    path_components(path).join("/")
}

fn path_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|c| match c {
            Component::Normal(s) => s.to_str().map(str::to_string),
            _ => None,
        })
        .collect()
}

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            _ => out.push(component.as_os_str()),
        }
    }
    out
}

fn contains_component_sequence(have: &[String], want: &[String]) -> bool {
    !want.is_empty() && have.windows(want.len()).any(|w| w == want)
}

#[derive(Debug, Clone, Copy)]
struct SecretSpan {
    kind: SecretKind,
    start: usize,
    end: usize,
    line: usize,
    line_start: usize,
    line_end: usize,
}

fn find_secret_spans(content: &str) -> Vec<SecretSpan> {
    let mut spans = Vec::new();
    find_private_key(content, &mut spans);

    let mut offset = 0usize;
    for (idx, line) in content.split_inclusive('\n').enumerate() {
        let trimmed = line.strip_suffix('\n').unwrap_or(line);
        find_line_secret(trimmed, offset, idx + 1, &mut spans);
        offset += line.len();
    }
    spans.sort_by_key(|s| (s.start, s.end));
    dedup_overlapping(spans)
}

fn find_private_key(content: &str, spans: &mut Vec<SecretSpan>) {
    let mut search = 0;
    while let Some(rel_start) = content[search..].find("-----BEGIN PRIVATE KEY-----") {
        let start = search + rel_start;
        let end = content[start..]
            .find("-----END PRIVATE KEY-----")
            .map(|rel| start + rel + "-----END PRIVATE KEY-----".len())
            .unwrap_or_else(|| start + "-----BEGIN PRIVATE KEY-----".len());
        let (line, line_start) = line_for_offset(content, start);
        spans.push(SecretSpan {
            kind: SecretKind::PrivateKey,
            start,
            end,
            line,
            line_start,
            line_end: line_start + (end - start),
        });
        search = end;
    }
}

fn find_line_secret(line: &str, line_offset: usize, line_no: usize, spans: &mut Vec<SecretSpan>) {
    let lower = line.to_ascii_lowercase();

    if let Some(pos) = lower.find("bearer ") {
        let start = pos + "bearer ".len();
        push_token_span(
            line,
            line_offset,
            line_no,
            start,
            SecretKind::BearerToken,
            spans,
        );
    }
    if let Some(pos) = find_token_prefix(line, "AKIA", 20) {
        spans.push(span(
            line_offset,
            line_no,
            pos,
            pos + 20,
            SecretKind::AwsAccessKey,
        ));
    }
    if let Some(pos) = find_token_prefix(line, "ghp_", 20) {
        let end = token_end(line, pos);
        spans.push(span(
            line_offset,
            line_no,
            pos,
            end,
            SecretKind::GithubToken,
        ));
    }
    if let Some(start) = value_start_after_key(&lower, "aws_secret_access_key") {
        push_value_span(
            line,
            line_offset,
            line_no,
            start,
            SecretKind::AwsSecretKey,
            spans,
        );
    } else if let Some(start) = value_start_after_key(&lower, "api_key") {
        push_value_span(line, line_offset, line_no, start, SecretKind::ApiKey, spans);
    } else if let Some(start) = value_start_after_key(&lower, "apikey") {
        push_value_span(line, line_offset, line_no, start, SecretKind::ApiKey, spans);
    }
    if let Some(start) = value_start_after_key(&lower, "password") {
        push_value_span(
            line,
            line_offset,
            line_no,
            start,
            SecretKind::Password,
            spans,
        );
    }
    if let Some(pos) = lower.find("cookie:") {
        let start = skip_space(line, pos + "cookie:".len());
        push_value_span(line, line_offset, line_no, start, SecretKind::Cookie, spans);
    }
}

fn value_start_after_key(lower: &str, key: &str) -> Option<usize> {
    let key_pos = lower.find(key)?;
    let rest = &lower[key_pos + key.len()..];
    let sep_rel = rest.find(['=', ':'])?;
    Some(skip_space(lower, key_pos + key.len() + sep_rel + 1))
}

fn push_token_span(
    line: &str,
    line_offset: usize,
    line_no: usize,
    start: usize,
    kind: SecretKind,
    spans: &mut Vec<SecretSpan>,
) {
    let end = token_end(line, start);
    if end > start {
        spans.push(span(line_offset, line_no, start, end, kind));
    }
}

fn push_value_span(
    line: &str,
    line_offset: usize,
    line_no: usize,
    start: usize,
    kind: SecretKind,
    spans: &mut Vec<SecretSpan>,
) {
    let start = trim_opening_quote(line, start);
    let mut end = token_end(line, start);
    end = trim_closing_quote(line, start, end);
    if end > start {
        spans.push(span(line_offset, line_no, start, end, kind));
    }
}

fn span(
    line_offset: usize,
    line_no: usize,
    start: usize,
    end: usize,
    kind: SecretKind,
) -> SecretSpan {
    SecretSpan {
        kind,
        start: line_offset + start,
        end: line_offset + end,
        line: line_no,
        line_start: start,
        line_end: end,
    }
}

fn find_token_prefix(line: &str, prefix: &str, min_len: usize) -> Option<usize> {
    let mut search = 0;
    while let Some(rel) = line[search..].find(prefix) {
        let start = search + rel;
        let end = token_end(line, start);
        if end - start >= min_len {
            return Some(start);
        }
        search = end;
    }
    None
}

fn token_end(line: &str, start: usize) -> usize {
    line[start..]
        .char_indices()
        .find_map(|(i, ch)| {
            if ch.is_whitespace() || matches!(ch, '"' | '\'' | ';' | ',' | ')' | '(' | '<' | '>') {
                Some(start + i)
            } else {
                None
            }
        })
        .unwrap_or(line.len())
}

fn skip_space(s: &str, mut i: usize) -> usize {
    while i < s.len() && s.as_bytes()[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn trim_opening_quote(line: &str, start: usize) -> usize {
    if start < line.len() && matches!(line.as_bytes()[start], b'"' | b'\'') {
        start + 1
    } else {
        start
    }
}

fn trim_closing_quote(line: &str, start: usize, mut end: usize) -> usize {
    if end > start && matches!(line.as_bytes()[end - 1], b'"' | b'\'') {
        end -= 1;
    }
    end
}

fn line_for_offset(content: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut line_start = 0;
    for (i, ch) in content.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            line_start = i + 1;
        }
    }
    (line, offset - line_start)
}

fn dedup_overlapping(spans: Vec<SecretSpan>) -> Vec<SecretSpan> {
    let mut out: Vec<SecretSpan> = Vec::new();
    for span in spans {
        if out
            .last()
            .is_some_and(|prev| span.start < prev.end && span.end <= prev.end)
        {
            continue;
        }
        out.push(span);
    }
    out
}
