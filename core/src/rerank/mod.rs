//! task-21.2: reranker abstraction (Phase 21 retrieval-quality).
//!
//! `Reranker` re-orders an initial top-k candidate list by joint (query, candidate) relevance.
//! `IdentityReranker` is the model-free deterministic default (CI/test/wiring — 0 model dependency);
//! `CrossEncoderReranker` is the real cross-encoder behind the `reranker-fastembed` feature (off by
//! default — 0 new crate; it reuses the already-present optional `fastembed` dep). Mirrors the
//! task-19.1 embedding module shape (trait + deterministic default + feature-gated real provider).

pub mod factory;
pub mod traits;
pub mod identity;

#[cfg(feature = "reranker-fastembed")]
pub mod cross_encoder;

// task-38.1: remote reranker provider (OpenAI/Cohere/SiliconFlow HTTP). Feature-gated — the default
// build does not compile it or pull the HTTP client (ADR-004 local-first; reuses optional `ureq`).
#[cfg(feature = "reranker-remote")]
pub mod remote_reranker;

pub use factory::{reranker_from_env, select_reranker};
pub use identity::{IdentityReranker, IDENTITY_RERANK_REASON};
pub use traits::{RerankError, Reranker};

#[cfg(feature = "reranker-fastembed")]
pub use cross_encoder::CrossEncoderReranker;

#[cfg(feature = "reranker-remote")]
pub use remote_reranker::{RemoteRerankerProvider, REMOTE_RERANK_REASON};
