//! task-19.1: real embedding provider via fastembed-rs (ort/ONNX runtime, `all-MiniLM-L6-v2`,
//! dim 384). Gated behind the `embedding-fastembed` feature — the default build does not compile
//! this module or pull fastembed/ort.
//!
//! The model is lazy-loaded on the first `embed` call (a `OnceLock`-guarded init), so constructing
//! the provider never triggers a model download. fastembed is built with rustls (not OpenSSL) so it
//! needs no system OpenSSL/pkg-config. Module is named `fastembed_provider` (not `fastembed`) to
//! avoid colliding with the `fastembed` crate name in `use` paths.

use std::sync::OnceLock;

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use crate::embedding::traits::{EmbeddingError, EmbeddingProvider};

/// Output dimension of `all-MiniLM-L6-v2` (matches `deterministic::DEFAULT_DIM`).
pub const FASTEMBED_DIM: usize = 384;

/// Real embedding provider backed by fastembed-rs + ONNX `all-MiniLM-L6-v2`.
pub struct FastEmbedProvider {
    model: OnceLock<TextEmbedding>,
}

impl FastEmbedProvider {
    pub fn new() -> Self {
        Self {
            model: OnceLock::new(),
        }
    }

    fn model(&self) -> Result<&TextEmbedding, EmbeddingError> {
        if let Some(m) = self.model.get() {
            return Ok(m);
        }
        let m = TextEmbedding::try_new(InitOptions::new(EmbeddingModel::AllMiniLML6V2))
            .map_err(|e| EmbeddingError::ModelLoad(e.to_string()))?;
        // A concurrent caller may win the race; either stored model is equivalent.
        let _ = self.model.set(m);
        Ok(self.model.get().expect("model set above"))
    }
}

impl Default for FastEmbedProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for FastEmbedProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FastEmbedProvider")
    }
}

impl EmbeddingProvider for FastEmbedProvider {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        let model = self.model()?;
        model
            .embed(texts.to_vec(), None)
            .map_err(|e| EmbeddingError::Other(e.to_string()))
    }

    fn dim(&self) -> usize {
        FASTEMBED_DIM
    }

    fn name(&self) -> &'static str {
        "fastembed-all-minilm-l6-v2"
    }
}
