//! Audit logging for MemoryOps privacy-sensitive operations.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOperation {
    Import,
    Search,
    Export,
    Redact,
}

impl AuditOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            AuditOperation::Import => "import",
            AuditOperation::Search => "search",
            AuditOperation::Export => "export",
            AuditOperation::Redact => "redact",
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

pub struct AuditSink;

impl AuditSink {
    pub fn open(_data_dir: impl AsRef<Path>, _collection: &str) -> Result<Self, AuditError> {
        Ok(Self)
    }

    pub fn record(&mut self, _event: AuditEvent) -> Result<AuditLogEntry, AuditError> {
        Err(AuditError::InvalidEvent(
            "audit record storage not implemented".to_string(),
        ))
    }

    pub fn list(&self) -> Result<Vec<AuditLogEntry>, AuditError> {
        Ok(Vec::new())
    }

    pub fn count_by_operation(&self, _operation: AuditOperation) -> Result<u64, AuditError> {
        Ok(0)
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
