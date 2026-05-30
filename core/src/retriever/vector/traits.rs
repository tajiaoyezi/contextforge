//! task-18.1: vector retrieval traits (sync, per §5.3 §E decision).

use std::fmt::Debug;

use crate::retriever::vector::types::{ChunkId, VectorChunk, VectorError, VectorFilter, VectorHit, VectorIndexConfig};

/// Static identity/capability of a vector backend.
///
/// All backend impls (`NoopVectorBackend`, future SqliteVec/Qdrant/LanceDB/Hnsw)
/// implement this base trait.
pub trait VectorBackend: Send + Sync + Debug {
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn is_local(&self) -> bool;
    fn requires_embedding(&self) -> bool;
}

/// Write-path: index lifecycle + mutation.
pub trait VectorIndexer: VectorBackend {
    fn open(&self, config: VectorIndexConfig) -> Result<(), VectorError>;
    fn index_batch(&self, chunks: &[VectorChunk]) -> Result<usize, VectorError>;
    fn delete(&self, ids: &[ChunkId]) -> Result<usize, VectorError>;
    fn flush(&self) -> Result<(), VectorError>;
    fn close(&self) -> Result<(), VectorError>;
}

/// Read-path: nearest-neighbor search.
///
/// # Examples
/// ```
/// use contextforge_core::retriever::vector::{NoopVectorBackend, VectorSearcher, VectorBackend};
/// let backend = NoopVectorBackend;
/// assert_eq!(backend.name(), "noop");
/// assert!(!backend.is_indexed());
/// let hits = backend.search(&[0.1, 0.2, 0.3], 10, None).unwrap();
/// assert!(hits.is_empty());
/// ```
pub trait VectorSearcher: VectorBackend {
    fn search(
        &self,
        query_vec: &[f32],
        k: usize,
        filter: Option<&VectorFilter>,
    ) -> Result<Vec<VectorHit>, VectorError>;
    fn is_indexed(&self) -> bool;
}
