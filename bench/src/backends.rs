//! Backend registry: resolves a backend name to a measurement run.
//!
//! At task-18.2 only `noop` is wired. task-18.3-18.6 each add an arm
//! (`#[cfg(feature = "vector-<backend>")]`) for their real backend.

use contextforge_core::retriever::vector::{NoopVectorBackend, VectorChunk, VectorError};

use crate::corpus::Query;
use crate::runner::{run, MeasureReport};

/// Run the named backend. Returns `Ok(None)` for an unknown backend name.
pub fn run_named(
    name: &str,
    corpus: &[VectorChunk],
    queries: &[Query],
    dim: usize,
) -> Result<Option<MeasureReport>, VectorError> {
    match name {
        "noop" => Ok(Some(run(&NoopVectorBackend, corpus, queries, dim)?)),
        #[cfg(feature = "vector-hnsw")]
        "hnsw" => Ok(Some(run(
            &contextforge_core::retriever::vector::HnswBackend::new(),
            corpus,
            queries,
            dim,
        )?)),
        #[cfg(feature = "vector-sqlite")]
        "sqlite-vec" => Ok(Some(run(
            &contextforge_core::retriever::vector::SqliteVecBackend::new()?,
            corpus,
            queries,
            dim,
        )?)),
        // task-18.4-18.5 add: "qdrant" | "lancedb" (cfg-gated).
        _ => Ok(None),
    }
}

/// Backend names this harness can currently run.
pub fn known_backends() -> &'static [&'static str] {
    #[cfg(all(feature = "vector-hnsw", feature = "vector-sqlite"))]
    {
        &["noop", "hnsw", "sqlite-vec"]
    }
    #[cfg(all(feature = "vector-hnsw", not(feature = "vector-sqlite")))]
    {
        &["noop", "hnsw"]
    }
    #[cfg(all(feature = "vector-sqlite", not(feature = "vector-hnsw")))]
    {
        &["noop", "sqlite-vec"]
    }
    #[cfg(all(not(feature = "vector-hnsw"), not(feature = "vector-sqlite")))]
    {
        &["noop"]
    }
}
