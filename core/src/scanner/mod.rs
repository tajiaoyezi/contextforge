//! Phase 2 scanner: file discovery, denylist/allowlist filtering, and
//! secret redaction before parser/chunker/indexer consume local content.

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

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
}

#[derive(Debug)]
pub enum ScanError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    DenylistOverrideRequired,
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
        }
    }
}

impl Error for ScanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ScanError::Io { source, .. } => Some(source),
            ScanError::DenylistOverrideRequired => None,
        }
    }
}

pub fn default_denylist() -> Vec<String> {
    unimplemented!("task-2.1 RED skeleton: default_denylist")
}

pub fn scan_path(root: impl AsRef<Path>, options: &ScanOptions) -> Result<ScanReport, ScanError> {
    let _ = (root.as_ref(), options);
    unimplemented!("task-2.1 RED skeleton: scan_path")
}

pub fn scan_file(path: impl AsRef<Path>, options: &ScanOptions) -> Result<ScannedFile, ScanError> {
    let _ = (path.as_ref(), options);
    unimplemented!("task-2.1 RED skeleton: scan_file")
}

pub fn detect_secrets(content: &str) -> Vec<SecretMatch> {
    let _ = content;
    unimplemented!("task-2.1 RED skeleton: detect_secrets")
}

pub fn redact_content(content: &str) -> (String, RedactionStatus, Vec<SecretMatch>) {
    let _ = content;
    unimplemented!("task-2.1 RED skeleton: redact_content")
}

pub fn is_denied(path: impl AsRef<Path>, denylist: &[String]) -> Option<String> {
    let _ = (path.as_ref(), denylist);
    unimplemented!("task-2.1 RED skeleton: is_denied")
}

pub fn is_allowlisted(path: impl AsRef<Path>, allowlist: &[PathBuf]) -> bool {
    let _ = (path.as_ref(), allowlist);
    unimplemented!("task-2.1 RED skeleton: is_allowlisted")
}

/// Backward-compatible task-1.3 test anchor.
pub fn placeholder_ready() -> bool {
    true
}
