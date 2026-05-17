//! task-1.1 proto / canonical-record contract conformance — Rust data-plane.
//!
//! SCEN/TEST-1.1.x map to docs/specs/tasks/task-1.1-proto.md §7 追踪表 and
//! test/features/proto.feature. Asserts the FROZEN proto / canonical-record
//! contract (PRD §Technical Approach Canonical Record v0.1).

use contextforge_core::contract;

fn assert_superset(have: &[String], want: &[&str], msg: &str) {
    let missing: Vec<&str> = want
        .iter()
        .copied()
        .filter(|w| !have.iter().any(|h| h == w))
        .collect();
    assert!(
        missing.is_empty(),
        "{msg}: missing proto fields {missing:?} (have {have:?})"
    );
}

// AC1 ContextRecord minimal field set (PRD §Technical Approach + task §6 AC1).
const CONTEXT_RECORD_REQUIRED: &[&str] = &[
    "id", "schema_version", "collection_id", "source_type", "source_provider",
    "source_uri", "agent_scope", "content", "content_hash", "redaction_status",
    "language", "file_path", "line_start", "line_end", "tags", "provenance",
    "security_labels", "created_at", "updated_at", "expires_at", "version", "metadata",
];

// TEST-1.1.1 / SCEN-1.1.1 / AC1 — ContextRecord 含全部最小字段。
#[test]
fn test_context_record_minimal_fields() {
    assert_superset(
        &contract::message_fields("ContextRecord"),
        CONTEXT_RECORD_REQUIRED,
        "AC1 ContextRecord minimal schema",
    );
}

// TEST-1.1.2 / SCEN-1.1.2 / AC2 — 额外定义 SourceRecord / Chunk / RetrievalResult。
#[test]
fn test_four_canonical_objects() {
    for m in ["SourceRecord", "Chunk", "RetrievalResult"] {
        assert!(
            !contract::message_fields(m).is_empty(),
            "AC2: proto message {m:?} not defined / has no fields"
        );
    }
}

// TEST-1.1.3 / SCEN-1.1.3 / AC3 — search 请求/响应字段与 PRD 草案一致。
#[test]
fn test_search_contract() {
    assert_superset(
        &contract::message_fields("SearchRequest"),
        &["query", "collections", "agent_scope", "top_k", "filters", "explain"],
        "AC3 SearchRequest",
    );
    assert_superset(
        &contract::message_fields("RetrievalResult"),
        &[
            "chunk_id", "context_id", "source_type", "file_path", "line_start",
            "line_end", "score", "retrieval_method", "reason", "agent_scope",
            "redaction_status", "provenance",
        ],
        "AC3 RetrievalResult",
    );
}

// TEST-1.1.4 / SCEN-1.1.4 / AC4 — Rust 侧 tonic/prost codegen 成功（无 FFI）。
#[test]
fn test_rust_codegen_succeeds() {
    contract::generated_rust_smoke().expect("AC4 Rust (tonic/prost) codegen smoke");
}

// TEST-1.1.5 / SCEN-1.1.5 / AC5 — schema_version="0.1" + 冻结规则文档化。
#[test]
fn test_schema_version_frozen() {
    assert_eq!(
        contract::schema_version(),
        "0.1",
        "AC5: schema_version must be 0.1"
    );
    assert!(
        contract::freeze_rule_documented(),
        "AC5: proto must document the freeze rule (only add fields, never delete/renumber tags)"
    );
}
