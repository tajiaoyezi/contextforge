//! task-19.1: embedding provider abstraction (Phase 19 vector-retrieval-integration).
//!
//! `EmbeddingProvider` turns text into fixed-dimension vectors. `DeterministicEmbeddingProvider`
//! is the model-free default (CI/smoke/test/wiring); `FastEmbedProvider` is the real model behind
//! the `embedding-fastembed` feature (off by default — 0 new dep at default features).

pub mod traits;
pub mod deterministic;
pub mod factory;

#[cfg(feature = "embedding-fastembed")]
pub mod fastembed_provider;

#[cfg(test)]
mod tests;

pub use deterministic::{DeterministicEmbeddingProvider, DEFAULT_DIM};
pub use factory::select_provider;
pub use traits::{EmbeddingError, EmbeddingProvider};

#[cfg(feature = "embedding-fastembed")]
pub use fastembed_provider::FastEmbedProvider;
