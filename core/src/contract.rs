//! Frozen ContextForge proto / canonical-record contract (task-1.1),
//! Rust data-plane side.
//!
//! RED skeleton: every accessor is a deliberate, explicit `unimplemented!()`
//! so the conformance tests fail for the right reason — feature absent —
//! rather than by a compile error (s2v §2.5.1 compiled-language RED bridge).
//! GREEN replaces these with the real implementation that parses
//! `proto/contextforge/v1/*.proto` and uses the tonic/prost generated bindings.

/// Frozen canonical-record schema version ("0.1").
pub fn schema_version() -> &'static str {
    unimplemented!("contract::schema_version — proto contract not yet frozen (task-1.1 RED)")
}

/// Whether the proto documents the versioning freeze rule
/// ("only add fields, never delete or renumber tags").
pub fn freeze_rule_documented() -> bool {
    unimplemented!("contract::freeze_rule_documented — task-1.1 RED")
}

/// Proto field names of the given canonical message.
pub fn message_fields(_msg: &str) -> Vec<String> {
    unimplemented!("contract::message_fields — task-1.1 RED")
}

/// Proves the Rust bindings were code-generated (tonic/prost, no FFI) by
/// constructing a generated message. Returns `Ok(())` when codegen is wired.
pub fn generated_rust_smoke() -> Result<(), String> {
    unimplemented!("contract::generated_rust_smoke — task-1.1 RED")
}
