//! task-18.1: vector retrieval types + error enum.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Chunk identifier. Newtype over String.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkId(pub String);

/// Distance metric for vector similarity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VectorMetric {
    Cosine,
    DotProduct,
    L2,
}

/// Score newtype with NaN/Inf guard (constructed via VectorScore::new).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct VectorScore(f32);

impl VectorScore {
    pub fn new(value: f32) -> Result<Self, VectorError> {
        if value.is_nan() || value.is_infinite() {
            return Err(VectorError::InvalidScore(value));
        }
        Ok(Self(value))
    }

    pub fn as_f32(&self) -> f32 {
        self.0
    }
}

/// Single vector search hit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorHit {
    pub chunk_id: ChunkId,
    pub score: VectorScore,
    pub metadata: Option<serde_json::Value>,
}

/// Chunk + embedding pair for indexing.
#[derive(Debug, Clone)]
pub struct VectorChunk {
    pub chunk_id: ChunkId,
    pub embedding: Vec<f32>,
    pub metadata: Option<serde_json::Value>,
}

/// Index initialization config.
#[derive(Debug, Clone)]
pub struct VectorIndexConfig {
    pub dim: usize,
    pub metric: VectorMetric,
    pub persistence_path: Option<PathBuf>,
    pub collection_id: String,
}

/// Optional search filter (backend-specific extras via opaque JSON).
#[derive(Debug, Clone, Default)]
pub struct VectorFilter {
    pub agent_scope: Option<String>,
    pub source_type: Option<String>,
    pub max_age_days: Option<u32>,
    pub extras: Option<serde_json::Value>,
}

/// All errors backend impls can return.
#[derive(Debug, Error)]
pub enum VectorError {
    #[error("backend not initialized")]
    NotInitialized,
    #[error("invalid embedding dimension: expected {expected}, got {got}")]
    DimMismatch { expected: usize, got: usize },
    #[error("score is NaN or infinite: {0}")]
    InvalidScore(f32),
    #[error("backend I/O error: {0}")]
    Io(String),
    #[error("backend error: {0}")]
    Other(String),
}
