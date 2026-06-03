//! task-29.1: vector backend selection factory (Phase 29 live-vector-recall).
//!
//! `select_vector_backend` maps a config backend name + requested embedding dim to a concrete
//! `Arc<dyn VectorSearcher>`, mirroring `embedding::factory::select_provider` and centralizing the
//! choice the `server.rs` hot paths (`:302` hybrid / `:341` semantic) used to hardcode as
//! `Arc::new(BruteForceVectorBackend::new())`. The default (`""` / `"brute"`) is byte-equivalent to
//! that hardcoded backend — the swap is behavior-preserving (ADR-034 D1). `qdrant` / `lancedb` /
//! `sqlite-vec` stay feature-gated (ADR-004 local-first: the default build pulls 0 new dependency); a disabled feature
//! surfaces as an explicit `VectorError` — never a silent fallback to BruteForce, never a fabricated
//! success (ADR-013).

use std::sync::Arc;

use crate::retriever::vector::traits::VectorStore;
use crate::retriever::vector::types::VectorError;
use crate::retriever::vector::BruteForceVectorBackend;

/// Select a vector backend by config name + requested embedding dim.
///
/// - `""` / `"brute"` → `BruteForceVectorBackend` (always available, 0-dep).
/// - `"qdrant"` → `QdrantBackend` behind the `vector-qdrant` feature; an explicit
///   feature-not-enabled error otherwise (no panic, no silent fallback).
/// - `"lancedb"` → `LanceDbBackend` behind the `vector-lancedb` feature; an explicit
///   feature-not-enabled error otherwise.
/// - `"sqlite-vec"` → `SqliteVecBackend` behind the `vector-sqlite` feature; an explicit
///   feature-not-enabled error otherwise.
/// - any other name → an explicit unknown-backend error.
///
/// Returns `Arc<dyn VectorStore>` (both indexer + searcher) so the `server.rs` hot path can index
/// then search through one handle; it upcasts to `Arc<dyn VectorSearcher>` at `with_vector_searcher`.
/// task-34.1 (ADR-039 D1): after constructing the backend, `dim` is reconciled against the backend's
/// declared `expected_dim()` via [`negotiate_vector_dim`] (mirrors `embedding::factory::negotiate_dim`)
/// — a configured `CONTEXTFORGE_VECTOR_DIM` is no longer silently discarded. The default BruteForce
/// backend is dim-agnostic (`expected_dim() == None`) so the default build accepts any dim and stays
/// byte-equivalent (ADR-004); enforcement bites only for dim-declaring feature backends.
pub fn select_vector_backend(
    name: &str,
    dim: usize,
) -> Result<Arc<dyn VectorStore>, VectorError> {
    let backend: Arc<dyn VectorStore> = match name {
        "" | "brute" => Arc::new(BruteForceVectorBackend::new()),
        "qdrant" => {
            #[cfg(feature = "vector-qdrant")]
            {
                Arc::new(crate::retriever::vector::QdrantBackend::new()?)
            }
            #[cfg(not(feature = "vector-qdrant"))]
            {
                return Err(VectorError::Other(
                    "vector backend 'qdrant' requires the vector-qdrant feature".into(),
                ));
            }
        }
        "lancedb" => {
            #[cfg(feature = "vector-lancedb")]
            {
                Arc::new(crate::retriever::vector::LanceDbBackend::new()?)
            }
            #[cfg(not(feature = "vector-lancedb"))]
            {
                return Err(VectorError::Other(
                    "vector backend 'lancedb' requires the vector-lancedb feature".into(),
                ));
            }
        }
        "sqlite-vec" => {
            #[cfg(feature = "vector-sqlite")]
            {
                Arc::new(crate::retriever::vector::SqliteVecBackend::new()?)
            }
            #[cfg(not(feature = "vector-sqlite"))]
            {
                return Err(VectorError::Other(
                    "vector backend 'sqlite-vec' requires the vector-sqlite feature".into(),
                ));
            }
        }
        other => {
            return Err(VectorError::Other(format!("unknown vector backend {other:?}")));
        }
    };
    // task-34.1 (ADR-039 D1): reconcile the configured dim with the backend's declared dim.
    negotiate_vector_dim(dim, backend.expected_dim())?;
    Ok(backend)
}

/// task-34.1 (ADR-039 D1): reconcile a configured embedding dim with a backend's declared dim,
/// mirroring [`crate::embedding::factory::negotiate_dim`]. `requested == 0` ("unset" — use whatever
/// the backend/embedder produces) and a dim-agnostic backend (`declared == None`, e.g. BruteForce)
/// never mismatch; a non-zero `requested` that differs from a backend's declared dim is a hard
/// `DimMismatch` — the factory never silently truncates, pads, or discards the configured dim.
pub(crate) fn negotiate_vector_dim(requested: usize, declared: Option<usize>) -> Result<(), VectorError> {
    if let Some(d) = declared {
        if requested != 0 && requested != d {
            return Err(VectorError::DimMismatch {
                expected: requested,
                got: d,
            });
        }
    }
    Ok(())
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

    // TEST-32.2.1 (default-build half): sqlite-vec feature off → honest Err naming both the backend
    // ("sqlite-vec") and the feature ("vector-sqlite"), never a silent BruteForce fallback. Mirrors
    // the qdrant/lancedb feature-off honest-Err tests above.
    #[cfg(not(feature = "vector-sqlite"))]
    #[test]
    fn sqlite_vec_without_feature_is_honest_err() {
        let err = select_vector_backend("sqlite-vec", 0).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("sqlite-vec"), "err should name sqlite-vec: {msg}");
        assert!(msg.contains("vector-sqlite"), "err should name the feature: {msg}");
    }

    // TEST-32.2.1 (feature-on half): sqlite-vec feature on → factory returns the sqlite-vec backend.
    #[cfg(feature = "vector-sqlite")]
    #[test]
    fn sqlite_vec_with_feature_returns_sqlite_vec_backend() {
        let backend =
            select_vector_backend("sqlite-vec", 0).expect("sqlite-vec feature on → backend");
        assert_eq!(backend.name(), "sqlite-vec");
    }

    // TEST-32.2.2: in-process selection-matrix wiring — the factory dispatches each name to the
    // right backend. Default build stays 0-vector-dep: "" / "brute" → brute-force; "sqlite-vec" →
    // honest Err naming the feature (no silent fallback). The matrix's recall/latency CELL needs a
    // local MSVC `--features vector-sqlite` build + real corpus and is honest-deferred
    // [SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix] (no fabricated numbers, ADR-013).
    #[test]
    fn selection_matrix_wiring_dispatches_by_name() {
        assert_eq!(select_vector_backend("", 0).unwrap().name(), "brute-force");
        assert_eq!(select_vector_backend("brute", 0).unwrap().name(), "brute-force");
        #[cfg(not(feature = "vector-sqlite"))]
        {
            assert!(
                select_vector_backend("sqlite-vec", 0).is_err(),
                "default build must not silently fall back to brute-force for sqlite-vec"
            );
        }
        #[cfg(feature = "vector-sqlite")]
        {
            assert_eq!(
                select_vector_backend("sqlite-vec", 0).unwrap().name(),
                "sqlite-vec"
            );
        }
    }

    // TEST-34.1.1 (ADR-039 D1): negotiate_vector_dim pure-function — four paths. requested==0
    // ("unset") or a dim-agnostic backend (declared==None) never mismatch; a non-zero requested
    // that differs from a declared dim is a hard DimMismatch (reusing the existing variant).
    #[test]
    fn negotiate_vector_dim_four_paths() {
        // requested 0 → always Ok (use whatever the backend/embedder produces).
        assert!(negotiate_vector_dim(0, None).is_ok());
        assert!(negotiate_vector_dim(0, Some(128)).is_ok());
        // dim-agnostic backend (None) → any requested dim accepted.
        assert!(negotiate_vector_dim(384, None).is_ok());
        // declared matches requested → Ok.
        assert!(negotiate_vector_dim(384, Some(384)).is_ok());
        // declared differs from a non-zero requested → DimMismatch{expected:requested, got:declared}.
        match negotiate_vector_dim(384, Some(128)) {
            Err(VectorError::DimMismatch { expected, got }) => {
                assert_eq!(expected, 384);
                assert_eq!(got, 128);
            }
            other => panic!("expected DimMismatch, got {other:?}"),
        }
    }

    // TEST-34.1.2 (ADR-039 D1): the default BruteForce backend is dim-agnostic
    // (expected_dim()==None), so select_vector_backend accepts ANY configured dim and stays
    // byte-equivalent to the pre-34.1 behavior (the dim is no longer silently discarded, but the
    // default path imposes no constraint, ADR-004).
    #[test]
    fn brute_force_default_path_accepts_any_dim() {
        for dim in [0usize, 128, 384, 1536] {
            let backend = select_vector_backend("", dim)
                .unwrap_or_else(|e| panic!("brute-force should accept dim={dim}: {e}"));
            assert_eq!(backend.name(), "brute-force");
            assert_eq!(backend.expected_dim(), None, "BruteForce stays dim-agnostic");
        }
    }
}
