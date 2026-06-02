//! task-29.1: vector backend selection factory (Phase 29 live-vector-recall).
//!
//! `select_vector_backend` maps a config backend name + requested embedding dim to a concrete
//! `Arc<dyn VectorSearcher>`, mirroring `embedding::factory::select_provider` and centralizing the
//! choice the `server.rs` hot paths (`:302` hybrid / `:341` semantic) used to hardcode as
//! `Arc::new(BruteForceVectorBackend::new())`. The default (`""` / `"brute"`) is byte-equivalent to
//! that hardcoded backend — the swap is behavior-preserving (ADR-034 D1). `qdrant` / `lancedb` stay
//! feature-gated (ADR-004 local-first: the default build pulls 0 new dependency); a disabled feature
//! surfaces as an explicit `VectorError` — never a silent fallback to BruteForce, never a fabricated
//! success (ADR-013).

use std::sync::Arc;

use crate::retriever::vector::traits::VectorSearcher;
use crate::retriever::vector::types::VectorError;
use crate::retriever::vector::BruteForceVectorBackend;

/// Select a vector backend by config name + requested embedding dim.
///
/// - `""` / `"brute"` → `BruteForceVectorBackend` (always available, 0-dep).
/// - `"qdrant"` → `QdrantBackend` behind the `vector-qdrant` feature; an explicit
///   feature-not-enabled error otherwise (no panic, no silent fallback).
/// - `"lancedb"` → `LanceDbBackend` behind the `vector-lancedb` feature; an explicit
///   feature-not-enabled error otherwise.
/// - any other name → an explicit unknown-backend error.
///
/// `dim` mirrors `select_provider`'s signature for later embedder-dim negotiation; the BruteForce
/// arm works for any dim and does not constrain it in this task.
pub fn select_vector_backend(
    name: &str,
    dim: usize,
) -> Result<Arc<dyn VectorSearcher>, VectorError> {
    // task-29.1 RED: factory not yet implemented (TEST-29.1.1/29.1.2 expected to fail here).
    let _ = (name, dim);
    let _placeholder: Option<Arc<dyn VectorSearcher>> = None;
    let _ = BruteForceVectorBackend::new();
    Err(VectorError::Other(
        "select_vector_backend not yet implemented (task-29.1 RED)".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // TEST-29.1.1: default / empty name → BruteForce (default build, no feature, deterministic).
    #[test]
    fn empty_and_brute_name_return_brute_force() {
        let by_empty = select_vector_backend("", 0).expect("empty name should return a backend");
        assert_eq!(by_empty.name(), "brute-force");
        let by_brute = select_vector_backend("brute", 0).expect("brute name should return a backend");
        assert_eq!(by_brute.name(), "brute-force");
    }

    // TEST-29.1.2 (default-build half): feature off → qdrant/lancedb honest Err; unknown name Err.
    #[cfg(not(feature = "vector-qdrant"))]
    #[test]
    fn qdrant_without_feature_is_honest_err() {
        let err = select_vector_backend("qdrant", 0).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("qdrant"), "err should name qdrant: {msg}");
        assert!(msg.contains("vector-qdrant"), "err should name the feature: {msg}");
    }

    #[cfg(not(feature = "vector-lancedb"))]
    #[test]
    fn lancedb_without_feature_is_honest_err() {
        let err = select_vector_backend("lancedb", 0).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("lancedb"), "err should name lancedb: {msg}");
        assert!(msg.contains("vector-lancedb"), "err should name the feature: {msg}");
    }

    #[test]
    fn unknown_name_is_honest_err() {
        let err = select_vector_backend("nope", 0).unwrap_err();
        assert!(err.to_string().contains("nope"), "err should echo the name");
    }

    // TEST-29.1.2 (feature-on half): qdrant feature on → factory returns the qdrant backend.
    #[cfg(feature = "vector-qdrant")]
    #[test]
    fn qdrant_with_feature_returns_qdrant_backend() {
        let backend = select_vector_backend("qdrant", 0).expect("qdrant feature on → backend");
        assert_eq!(backend.name(), "qdrant");
    }

    #[cfg(feature = "vector-lancedb")]
    #[test]
    fn lancedb_with_feature_returns_lancedb_backend() {
        let backend = select_vector_backend("lancedb", 0).expect("lancedb feature on → backend");
        assert_eq!(backend.name(), "lancedb");
    }
}
