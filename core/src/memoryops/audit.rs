//! Audit logging for MemoryOps privacy-sensitive operations.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOperation {
    Import,
    Search,
    Export,
    Redact,
    // task-13.1 (ADR-017 D1 Wave 3): memory state-op audit events emitted by
    // `data_plane::memory::MemoryServer` on Pin / Deprecate / SoftDelete.
    MemoryPin,
    MemoryUnpin,
    MemoryDeprecate,
    MemorySoftDelete,
}

impl AuditOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            AuditOperation::Import => "import",
            AuditOperation::Search => "search",
            AuditOperation::Export => "export",
            AuditOperation::Redact => "redact",
            AuditOperation::MemoryPin => "memory_pin",
            AuditOperation::MemoryUnpin => "memory_unpin",
            AuditOperation::MemoryDeprecate => "memory_deprecate",
            AuditOperation::MemorySoftDelete => "memory_soft_delete",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    pub operation: AuditOperation,
    pub collection: String,
    pub source: String,
    pub result_count: u64,
    pub redaction_count: u64,
    pub query: Option<String>,
    pub redacted_terms: Vec<String>,
    pub chunk_ids: Vec<String>,
    pub export_total_byte_count: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditLogEntry {
    pub id: i64,
    pub operation: String,
    pub collection: String,
    pub source: String,
    pub result_count: u64,
    pub redaction_count: u64,
    pub timestamp: String,
    pub query_hash: Option<String>,
    pub query_length: Option<u64>,
    pub redacted_terms: Vec<String>,
    pub chunk_ids: Vec<String>,
    pub export_total_byte_count: Option<u64>,
}

#[derive(Debug)]
pub enum AuditError {
    Io(std::io::Error),
    Sqlite(String),
    InvalidEvent(String),
}

impl fmt::Display for AuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditError::Io(err) => write!(f, "audit io: {err}"),
            AuditError::Sqlite(err) => write!(f, "audit sqlite: {err}"),
            AuditError::InvalidEvent(err) => write!(f, "invalid audit event: {err}"),
        }
    }
}

impl std::error::Error for AuditError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AuditError::Io(err) => Some(err),
            AuditError::Sqlite(_) | AuditError::InvalidEvent(_) => None,
        }
    }
}

impl From<std::io::Error> for AuditError {
    fn from(err: std::io::Error) -> Self {
        AuditError::Io(err)
    }
}

impl From<rusqlite::Error> for AuditError {
    fn from(err: rusqlite::Error) -> Self {
        AuditError::Sqlite(err.to_string())
    }
}

pub struct AuditSink {
    collection_dir: PathBuf,
    sqlite: Connection,
}

impl AuditSink {
    pub fn open(data_dir: impl AsRef<Path>, collection: &str) -> Result<Self, AuditError> {
        if collection.trim().is_empty() {
            return Err(AuditError::InvalidEvent(
                "collection must not be empty".to_string(),
            ));
        }
        let collection_dir = data_dir.as_ref().join("collections").join(collection);
        fs::create_dir_all(&collection_dir)?;
        let sqlite = Connection::open(collection_dir.join("metadata.sqlite"))?;
        sqlite.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                operation TEXT NOT NULL,
                collection TEXT NOT NULL,
                source TEXT NOT NULL,
                result_count INTEGER NOT NULL,
                redaction_count INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                query_hash TEXT,
                query_length INTEGER,
                redacted_terms TEXT NOT NULL DEFAULT '',
                chunk_ids TEXT NOT NULL DEFAULT '',
                export_total_byte_count INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_audit_log_operation ON audit_log(operation);
            CREATE INDEX IF NOT EXISTS idx_audit_log_collection ON audit_log(collection);
            "#,
        )?;
        Ok(Self {
            collection_dir,
            sqlite,
        })
    }

    pub fn record(&mut self, event: AuditEvent) -> Result<AuditLogEntry, AuditError> {
        validate_event(&event)?;
        let timestamp = now_unix_string();
        let query_hash = event.query.as_deref().map(sha256_hex);
        let query_length = event.query.as_ref().map(|query| query.len() as u64);
        let redacted_terms = event
            .redacted_terms
            .iter()
            .map(|term| sanitize_redaction_label(term))
            .collect::<Vec<_>>();
        let chunk_ids = event
            .chunk_ids
            .iter()
            .map(|chunk_id| sanitize_chunk_id(chunk_id))
            .filter(|chunk_id| !chunk_id.is_empty())
            .collect::<Vec<_>>();

        self.sqlite.execute(
            "INSERT INTO audit_log
                (operation, collection, source, result_count, redaction_count, timestamp,
                 query_hash, query_length, redacted_terms, chunk_ids, export_total_byte_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                event.operation.as_str(),
                event.collection,
                event.source,
                u64_to_i64(event.result_count, "result_count")?,
                u64_to_i64(event.redaction_count, "redaction_count")?,
                timestamp,
                query_hash,
                query_length
                    .map(|value| u64_to_i64(value, "query_length"))
                    .transpose()?,
                serialize_list(&redacted_terms),
                serialize_list(&chunk_ids),
                event
                    .export_total_byte_count
                    .map(|value| u64_to_i64(value, "export_total_byte_count"))
                    .transpose()?,
            ],
        )?;

        let id = self.sqlite.last_insert_rowid();
        self.get(id)
    }

    pub fn list(&self) -> Result<Vec<AuditLogEntry>, AuditError> {
        let mut stmt = self.sqlite.prepare(
            "SELECT id, operation, collection, source, result_count, redaction_count,
                    timestamp, query_hash, query_length, redacted_terms, chunk_ids,
                    export_total_byte_count
             FROM audit_log ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], row_to_entry)?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }
        Ok(entries)
    }

    pub fn count_by_operation(&self, operation: AuditOperation) -> Result<u64, AuditError> {
        let count: i64 = self.sqlite.query_row(
            "SELECT COUNT(*) FROM audit_log WHERE operation = ?1",
            params![operation.as_str()],
            |row| row.get(0),
        )?;
        Ok(count.max(0) as u64)
    }

    fn get(&self, id: i64) -> Result<AuditLogEntry, AuditError> {
        Ok(self.sqlite.query_row(
            "SELECT id, operation, collection, source, result_count, redaction_count,
                    timestamp, query_hash, query_length, redacted_terms, chunk_ids,
                    export_total_byte_count
             FROM audit_log WHERE id = ?1",
            params![id],
            row_to_entry,
        )?)
    }

    pub fn collection_dir(&self) -> &Path {
        &self.collection_dir
    }
}

pub fn import_event(
    collection: &str,
    source: &str,
    result_count: u64,
    redaction_count: u64,
) -> AuditEvent {
    AuditEvent {
        operation: AuditOperation::Import,
        collection: collection.to_string(),
        source: source.to_string(),
        result_count,
        redaction_count,
        query: None,
        redacted_terms: Vec::new(),
        chunk_ids: Vec::new(),
        export_total_byte_count: None,
    }
}

pub fn search_event(
    collection: &str,
    source: &str,
    query: &str,
    result_count: u64,
    redaction_count: u64,
) -> AuditEvent {
    AuditEvent {
        operation: AuditOperation::Search,
        collection: collection.to_string(),
        source: source.to_string(),
        result_count,
        redaction_count,
        query: Some(query.to_string()),
        redacted_terms: Vec::new(),
        chunk_ids: Vec::new(),
        export_total_byte_count: None,
    }
}

pub fn export_event(
    collection: &str,
    source: &str,
    chunk_ids: Vec<String>,
    total_byte_count: u64,
    redaction_count: u64,
) -> AuditEvent {
    AuditEvent {
        operation: AuditOperation::Export,
        collection: collection.to_string(),
        source: source.to_string(),
        result_count: chunk_ids.len() as u64,
        redaction_count,
        query: None,
        redacted_terms: Vec::new(),
        chunk_ids,
        export_total_byte_count: Some(total_byte_count),
    }
}

pub fn redact_event(
    collection: &str,
    source: &str,
    redacted_terms: Vec<String>,
    redaction_count: u64,
) -> AuditEvent {
    AuditEvent {
        operation: AuditOperation::Redact,
        collection: collection.to_string(),
        source: source.to_string(),
        result_count: 0,
        redaction_count,
        query: None,
        redacted_terms,
        chunk_ids: Vec::new(),
        export_total_byte_count: None,
    }
}

pub fn scanner_override_event(
    collection: &str,
    source: &str,
    redacted_terms: Vec<String>,
    redaction_count: u64,
) -> AuditEvent {
    redact_event(collection, source, redacted_terms, redaction_count)
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuditLogEntry> {
    let result_count: i64 = row.get(4)?;
    let redaction_count: i64 = row.get(5)?;
    let query_length: Option<i64> = row.get(8)?;
    let export_total_byte_count: Option<i64> = row.get(11)?;
    Ok(AuditLogEntry {
        id: row.get(0)?,
        operation: row.get(1)?,
        collection: row.get(2)?,
        source: row.get(3)?,
        result_count: result_count.max(0) as u64,
        redaction_count: redaction_count.max(0) as u64,
        timestamp: row.get(6)?,
        query_hash: row.get(7)?,
        query_length: query_length.map(|value| value.max(0) as u64),
        redacted_terms: deserialize_list(&row.get::<_, String>(9)?),
        chunk_ids: deserialize_list(&row.get::<_, String>(10)?),
        export_total_byte_count: export_total_byte_count.map(|value| value.max(0) as u64),
    })
}

fn validate_event(event: &AuditEvent) -> Result<(), AuditError> {
    if event.collection.trim().is_empty() {
        return Err(AuditError::InvalidEvent(
            "collection must not be empty".to_string(),
        ));
    }
    if event.source.trim().is_empty() {
        return Err(AuditError::InvalidEvent(
            "source must not be empty".to_string(),
        ));
    }
    Ok(())
}

fn now_unix_string() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    now.as_secs().to_string()
}

fn sha256_hex(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let mut out = String::with_capacity("sha256:".len() + 64);
    out.push_str("sha256:");
    for byte in digest.iter() {
        use std::fmt::Write;
        write!(out, "{byte:02x}").expect("write to String never fails");
    }
    out
}

fn sanitize_redaction_label(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.starts_with("[REDACTED:")
        && trimmed.ends_with(']')
        && trimmed.chars().all(|ch| {
            ch.is_ascii_uppercase() || ch.is_ascii_digit() || matches!(ch, '[' | ']' | ':' | '_')
        })
    {
        trimmed.to_string()
    } else {
        "[REDACTED:UNKNOWN]".to_string()
    }
}

fn sanitize_chunk_id(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.'))
        .collect()
}

fn serialize_list(values: &[String]) -> String {
    values.join("\n")
}

fn deserialize_list(value: &str) -> Vec<String> {
    value
        .split('\n')
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn u64_to_i64(value: u64, field: &str) -> Result<i64, AuditError> {
    i64::try_from(value).map_err(|_| {
        AuditError::InvalidEvent(format!("{field} exceeds SQLite INTEGER positive range"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::redact_content;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    const SECRET: &str = "AKIAIOSFODNN7EXAMPLE";
    const EXPORT_BODY: &str = "full export body must not be persisted";

    fn temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "contextforge-audit-{name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn audit_db_bytes(data_dir: &Path, collection: &str) -> String {
        let db = data_dir
            .join("collections")
            .join(collection)
            .join("metadata.sqlite");
        let bytes = fs::read(db).unwrap_or_default();
        String::from_utf8_lossy(&bytes).to_string()
    }

    // TEST-5.3.1 / SCEN-5.3.1 / AC1: four critical operations are written to audit_log.
    #[test]
    fn test_5_3_1_four_operations_are_written_to_audit_log() {
        let data = temp_root("ac1");
        let collection = "project-a";
        let mut sink = AuditSink::open(&data, collection).expect("open audit sink");

        sink.record(import_event(collection, "importer:hermes", 2, 1))
            .expect("record import");
        sink.record(search_event(
            collection,
            "retriever:bm25",
            "rotate prod token",
            2,
            0,
        ))
        .expect("record search");
        sink.record(export_event(
            collection,
            "exporter:jsonl",
            vec!["chunk-a".to_string(), "chunk-b".to_string()],
            EXPORT_BODY.len() as u64,
            1,
        ))
        .expect("record export");
        sink.record(redact_event(
            collection,
            "scanner:redaction",
            vec!["[REDACTED:AWS_ACCESS_KEY]".to_string()],
            1,
        ))
        .expect("record redact");

        let entries = sink.list().expect("list audit");
        let operations: Vec<&str> = entries.iter().map(|e| e.operation.as_str()).collect();
        assert_eq!(operations, vec!["import", "search", "export", "redact"]);
        assert_eq!(sink.count_by_operation(AuditOperation::Import).unwrap(), 1);
        assert_eq!(sink.count_by_operation(AuditOperation::Search).unwrap(), 1);
        assert_eq!(sink.count_by_operation(AuditOperation::Export).unwrap(), 1);
        assert_eq!(sink.count_by_operation(AuditOperation::Redact).unwrap(), 1);
    }

    // TEST-5.3.2 / SCEN-5.3.2 / AC2: search audit stores query metadata, not raw query text.
    #[test]
    fn test_5_3_2_query_content_is_not_persisted() {
        let data = temp_root("ac2");
        let collection = "project-b";
        let raw_query = "how do I rotate AKIAIOSFODNN7EXAMPLE safely";
        let mut sink = AuditSink::open(&data, collection).expect("open audit sink");

        let entry = sink
            .record(search_event(collection, "retriever:bm25", raw_query, 3, 1))
            .expect("record search");

        assert_eq!(entry.operation, "search");
        assert_eq!(entry.query_length, Some(raw_query.len() as u64));
        assert!(entry.query_hash.is_some(), "query_hash should be present");
        let db_text = audit_db_bytes(&data, collection);
        assert!(
            !db_text.contains(raw_query),
            "audit DB must not contain raw query text"
        );
    }

    // TEST-5.3.3 / SCEN-5.3.3 / AC3: audit does not persist full secrets or export content.
    #[test]
    fn test_5_3_3_secret_and_export_content_are_not_persisted() {
        let data = temp_root("ac3");
        let collection = "project-c";
        let mut sink = AuditSink::open(&data, collection).expect("open audit sink");

        sink.record(redact_event(
            collection,
            "scanner:redaction",
            vec!["[REDACTED:AWS_ACCESS_KEY]".to_string()],
            1,
        ))
        .expect("record redact");
        sink.record(export_event(
            collection,
            "exporter:markdown",
            vec!["chunk-export-1".to_string()],
            EXPORT_BODY.len() as u64,
            1,
        ))
        .expect("record export");

        let db_text = audit_db_bytes(&data, collection);
        assert!(
            !db_text.contains(SECRET),
            "audit DB must not contain the full secret"
        );
        assert!(
            !db_text.contains(EXPORT_BODY),
            "audit DB must not contain full export content"
        );
        assert!(
            db_text.contains("[REDACTED:AWS_ACCESS_KEY]"),
            "audit DB should preserve redaction label evidence"
        );
        assert!(
            db_text.contains("chunk-export-1"),
            "audit DB should preserve chunk id evidence"
        );
    }

    // TEST-5.3.4 / SCEN-5.3.4 / AC4: scanner override writes a redact audit event.
    #[test]
    fn test_5_3_4_scanner_secret_override_writes_audit_event() {
        let data = temp_root("ac4");
        let collection = "project-d";
        let (_redacted, _status, matches) =
            redact_content("AWS key AKIAIOSFODNN7EXAMPLE was overridden for local import");
        let labels: Vec<String> = matches.iter().map(|m| m.redaction.to_string()).collect();
        let mut sink = AuditSink::open(&data, collection).expect("open audit sink");

        let entry = sink
            .record(scanner_override_event(
                collection,
                "scanner:override",
                labels.clone(),
                labels.len() as u64,
            ))
            .expect("record scanner override");

        assert_eq!(entry.operation, "redact");
        assert_eq!(entry.redaction_count, labels.len() as u64);
        assert_eq!(entry.redacted_terms, labels);
        let db_text = audit_db_bytes(&data, collection);
        assert!(
            !db_text.contains(SECRET),
            "override audit must redact secret"
        );
    }
}
