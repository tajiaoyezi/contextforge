//! task-18.6 spike: HNSW backend via `instant-distance` (pure-Rust approximate nearest neighbour).
//! Gated behind the `vector-hnsw` feature.
//!
//! Vectors are unit-normalized and compared by euclidean distance, which is monotonic with cosine
//! similarity for unit vectors — so HNSW nearest matches the cosine ground truth the harness uses.

use std::sync::Mutex;

use instant_distance::{Builder, HnswMap, Point, Search};

use crate::retriever::vector::traits::{VectorBackend, VectorIndexer, VectorSearcher};
use crate::retriever::vector::types::{
    ChunkId, VectorChunk, VectorError, VectorFilter, VectorHit, VectorIndexConfig, VectorScore,
};

#[derive(Clone)]
struct HnswPoint(Vec<f32>);

impl Point for HnswPoint {
    fn distance(&self, other: &Self) -> f32 {
        let n = self.0.len().min(other.0.len());
        let mut sum = 0.0f32;
        for i in 0..n {
            let d = self.0[i] - other.0[i];
            sum += d * d;
        }
        sum.sqrt()
    }
}

fn normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        return v.to_vec();
    }
    v.iter().map(|x| x / norm).collect()
}

/// HNSW backend. `instant-distance` builds the whole graph at once (no incremental insert), so
/// `index_batch` accumulates and `flush` builds the map (full-reindex semantics, per task-18.1).
pub struct HnswBackend {
    pending: Mutex<Vec<(Vec<f32>, String)>>,
    map: Mutex<Option<HnswMap<HnswPoint, String>>>,
}

impl HnswBackend {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(Vec::new()),
            map: Mutex::new(None),
        }
    }
}

impl Default for HnswBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for HnswBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HnswBackend")
    }
}

impl VectorBackend for HnswBackend {
    fn name(&self) -> &'static str {
        "hnsw"
    }
    fn version(&self) -> &'static str {
        "0.6"
    }
    fn is_local(&self) -> bool {
        true
    }
    fn requires_embedding(&self) -> bool {
        true
    }
}

impl VectorIndexer for HnswBackend {
    fn open(&self, _config: VectorIndexConfig) -> Result<(), VectorError> {
        self.pending.lock().unwrap().clear();
        *self.map.lock().unwrap() = None;
        Ok(())
    }

    fn index_batch(&self, chunks: &[VectorChunk]) -> Result<usize, VectorError> {
        let mut pending = self.pending.lock().unwrap();
        for c in chunks {
            pending.push((normalize(&c.embedding), c.chunk_id.0.clone()));
        }
        Ok(chunks.len())
    }

    fn delete(&self, _ids: &[ChunkId]) -> Result<usize, VectorError> {
        // HNSW has no in-place delete; spike semantics = full reindex, so clear and rebuild.
        let mut pending = self.pending.lock().unwrap();
        let n = pending.len();
        pending.clear();
        *self.map.lock().unwrap() = None;
        Ok(n)
    }

    fn flush(&self) -> Result<(), VectorError> {
        let pending = self.pending.lock().unwrap();
        if pending.is_empty() {
            *self.map.lock().unwrap() = None;
            return Ok(());
        }
        let points: Vec<HnswPoint> = pending.iter().map(|(v, _)| HnswPoint(v.clone())).collect();
        let values: Vec<String> = pending.iter().map(|(_, id)| id.clone()).collect();
        let map = Builder::default().build(points, values);
        *self.map.lock().unwrap() = Some(map);
        Ok(())
    }

    fn close(&self) -> Result<(), VectorError> {
        Ok(())
    }
}

impl VectorSearcher for HnswBackend {
    fn search(
        &self,
        query_vec: &[f32],
        k: usize,
        _filter: Option<&VectorFilter>,
    ) -> Result<Vec<VectorHit>, VectorError> {
        let guard = self.map.lock().unwrap();
        let map = match guard.as_ref() {
            Some(m) => m,
            None => return Ok(vec![]),
        };
        let q = HnswPoint(normalize(query_vec));
        let mut search = Search::default();
        let mut hits = Vec::with_capacity(k);
        for item in map.search(&q, &mut search).take(k) {
            // euclidean distance over unit vectors is in [0, 2]; map to a [0, 1] similarity score.
            let sim = 1.0 - item.distance / 2.0;
            let score = VectorScore::new(sim).unwrap_or_else(|_| VectorScore::new(0.0).unwrap());
            hits.push(VectorHit {
                chunk_id: ChunkId(item.value.clone()),
                score,
                metadata: None,
            });
        }
        Ok(hits)
    }

    fn is_indexed(&self) -> bool {
        self.map.lock().unwrap().is_some()
    }
}
