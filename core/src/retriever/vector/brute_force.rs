//! task-19.3: brute-force exact-cosine vector backend — **0 dependencies, default-available**.
//!
//! The semantic search path needs a `VectorSearcher` even in the default build, where the optional
//! ADR-023 ANN backends (hnsw / sqlite-vec / qdrant / lancedb) are feature-gated off. This exact
//! O(n) cosine searcher fills that role: correct, dependency-free, fine for small/medium corpora.
//! The ANN backends remain the feature-gated scalable options. ADR-023 D5 (no vector *deps* by
//! default) is preserved — this is pure Rust with no new dependency; and the default *behavior*
//! stays BM25 because the semantic path is opt-in (`SearchRequest.semantic`).
//!
//! Vectors are unit-normalized so the dot product equals cosine similarity.

use std::sync::Mutex;

use crate::retriever::vector::traits::{VectorBackend, VectorIndexer, VectorSearcher};
use crate::retriever::vector::types::{
    ChunkId, VectorChunk, VectorError, VectorFilter, VectorHit, VectorIndexConfig, VectorScore,
};

fn normalize(v: &[f32]) -> Vec<f32> {
    let n: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n == 0.0 {
        return v.to_vec();
    }
    v.iter().map(|x| x / n).collect()
}

/// Exact-cosine brute-force vector backend (in-memory, full-reindex semantics).
#[derive(Debug, Default)]
pub struct BruteForceVectorBackend {
    rows: Mutex<Vec<(Vec<f32>, String)>>, // (unit-normalized vector, chunk_id)
}

impl BruteForceVectorBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

impl VectorBackend for BruteForceVectorBackend {
    fn name(&self) -> &'static str {
        "brute-force"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    fn is_local(&self) -> bool {
        true
    }
    fn requires_embedding(&self) -> bool {
        true
    }
}

impl VectorIndexer for BruteForceVectorBackend {
    fn open(&self, _config: VectorIndexConfig) -> Result<(), VectorError> {
        self.rows.lock().unwrap().clear();
        Ok(())
    }

    fn index_batch(&self, chunks: &[VectorChunk]) -> Result<usize, VectorError> {
        let mut rows = self.rows.lock().unwrap();
        for c in chunks {
            rows.push((normalize(&c.embedding), c.chunk_id.0.clone()));
        }
        Ok(chunks.len())
    }

    fn delete(&self, _ids: &[ChunkId]) -> Result<usize, VectorError> {
        let mut rows = self.rows.lock().unwrap();
        let n = rows.len();
        rows.clear();
        Ok(n)
    }

    fn flush(&self) -> Result<(), VectorError> {
        Ok(())
    }

    fn close(&self) -> Result<(), VectorError> {
        Ok(())
    }
}

impl VectorSearcher for BruteForceVectorBackend {
    fn search(
        &self,
        query_vec: &[f32],
        k: usize,
        _filter: Option<&VectorFilter>,
    ) -> Result<Vec<VectorHit>, VectorError> {
        let rows = self.rows.lock().unwrap();
        if rows.is_empty() {
            return Ok(vec![]);
        }
        let q = normalize(query_vec);
        let mut scored: Vec<(f32, &String)> = rows
            .iter()
            .map(|(v, id)| {
                let dot: f32 = q.iter().zip(v.iter()).map(|(a, b)| a * b).sum();
                (dot, id)
            })
            .collect();
        // cosine descending; ties broken by chunk_id for deterministic ordering.
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.cmp(b.1))
        });
        Ok(scored
            .into_iter()
            .take(k)
            .map(|(sim, id)| VectorHit {
                chunk_id: ChunkId(id.clone()),
                score: VectorScore::new(sim).unwrap_or_else(|_| VectorScore::new(0.0).unwrap()),
                metadata: None,
            })
            .collect())
    }

    fn is_indexed(&self) -> bool {
        !self.rows.lock().unwrap().is_empty()
    }
}
