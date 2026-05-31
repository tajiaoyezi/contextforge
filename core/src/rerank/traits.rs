//! task-21.2: reranker trait + error (Phase 21 retrieval-quality).
//!
//! Mirrors the task-19.1 `EmbeddingProvider` style: `Send + Sync + Debug`, `#[non_exhaustive]`
//! error so downstream `match` stays add-only-safe. A `Reranker` re-orders an initial top-k
//! candidate list by a joint (query, candidate) relevance score. A cross-encoder scores the
//! query×doc *pair* jointly — more precise than the dual-encoder cosine `search_semantic` uses —
//! so reranking the initial top-k can lift top-1 / MRR. Object-safe (`Arc<dyn Reranker>`) so the
//! retriever can swap the deterministic default and a real model behind one seam.

use std::fmt::Debug;

use thiserror::Error;

use crate::retriever::SearchResult;

/// Re-orders an initial candidate list by joint (query, candidate) relevance.
///
/// Implementations MUST NOT fabricate or drop candidates — the output is a re-ordering of the
/// input (each output element corresponds to exactly one input candidate), in descending
/// rerank-relevance order. Object-safe so it injects as `Arc<dyn Reranker>` via
/// `Retriever::with_reranker`.
pub trait Reranker: Send + Sync + Debug {
    /// Re-rank `candidates` against `query`, returning them in descending rerank-score order.
    /// The output is a permutation of `candidates` (no candidate added or dropped). An empty
    /// `candidates` slice returns an empty `Vec`.
    fn rerank(
        &self,
        query: &str,
        candidates: &[SearchResult],
    ) -> Result<Vec<SearchResult>, RerankError>;

    /// Provider identity (provenance). Implementations may surface it in the reranked-result `reason`.
    fn name(&self) -> &'static str;
}

/// All errors a `Reranker` can return.
///
/// `#[non_exhaustive]`: downstream crates must not write exhaustive matches, so new variants stay
/// add-only-safe (the task-19.1 `EmbeddingError` pattern).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RerankError {
    #[error("rerank model load failed: {0}")]
    ModelLoad(String),
    #[error("rerank backend error: {source}")]
    Backend {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("rerank error: {0}")]
    Other(String),
}
