//! task-19.1: embedding provider trait + error (Phase 19 vector-retrieval-integration).
//!
//! Mirrors the task-18.1 vector trait style: `Send + Sync + Debug`, `#[non_exhaustive]` error so
//! downstream `match` stays add-only-safe. An `EmbeddingProvider` turns text into fixed-dimension
//! vectors that feed `VectorChunk.embedding` (index path) and the query vector (search path).

use std::fmt::Debug;

use thiserror::Error;

/// Turns text into fixed-dimension embedding vectors. Object-safe (`Arc<dyn EmbeddingProvider>`),
/// so the retriever can swap the deterministic default and a real model behind one seam.
pub trait EmbeddingProvider: Send + Sync + Debug {
    /// Embed a batch of texts. Returns one vector per input, each of length `dim()`.
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError>;
    /// The fixed output dimension every produced vector has.
    fn dim(&self) -> usize;
    /// Provider identity (provenance — feeds the phase-19 `embedding_provider` field).
    fn name(&self) -> &'static str;
}

/// All errors an `EmbeddingProvider` can return.
///
/// `#[non_exhaustive]`: downstream crates must not write exhaustive matches, so new variants stay
/// add-only-safe (the task-18.1 `VectorError` pattern).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EmbeddingError {
    #[error("embedding model load failed: {0}")]
    ModelLoad(String),
    #[error("embedding dimension mismatch: expected {expected}, got {got}")]
    DimMismatch { expected: usize, got: usize },
    #[error("empty input")]
    EmptyInput,
    #[error("embedding backend error: {source}")]
    Backend {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("embedding error: {0}")]
    Other(String),
}
