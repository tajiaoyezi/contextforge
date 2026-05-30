//! task-18.1: NoopVectorBackend — placeholder stub returning empty results.

use crate::retriever::vector::traits::{VectorBackend, VectorIndexer, VectorSearcher};
use crate::retriever::vector::types::{ChunkId, VectorChunk, VectorError, VectorFilter, VectorHit, VectorIndexConfig};

#[derive(Debug, Default, Clone, Copy)]
pub struct NoopVectorBackend;

impl VectorBackend for NoopVectorBackend {
    fn name(&self) -> &'static str { "noop" }
    fn version(&self) -> &'static str { "0.1.0" }
    fn is_local(&self) -> bool { true }
    fn requires_embedding(&self) -> bool { false }
}

impl VectorIndexer for NoopVectorBackend {
    fn open(&self, _config: VectorIndexConfig) -> Result<(), VectorError> { Ok(()) }
    fn index_batch(&self, _chunks: &[VectorChunk]) -> Result<usize, VectorError> { Ok(0) }
    fn delete(&self, _ids: &[ChunkId]) -> Result<usize, VectorError> { Ok(0) }
    fn flush(&self) -> Result<(), VectorError> { Ok(()) }
    fn close(&self) -> Result<(), VectorError> { Ok(()) }
}

impl VectorSearcher for NoopVectorBackend {
    fn search(
        &self,
        _query_vec: &[f32],
        _k: usize,
        _filter: Option<&VectorFilter>,
    ) -> Result<Vec<VectorHit>, VectorError> {
        // NoopVectorBackend.search: returning empty hits (vector backend not configured)
        Ok(vec![])
    }
    fn is_indexed(&self) -> bool { false }
}
