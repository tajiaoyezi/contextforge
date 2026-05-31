//! task-18.6 spike: HNSW backend via `instant-distance` (pure-Rust approximate nearest neighbour).
//! Gated behind the `vector-hnsw` feature.
//!
//! Vectors are unit-normalized and compared by euclidean distance, which is monotonic with cosine
//! similarity for unit vectors — so HNSW nearest matches the cosine ground truth the harness uses.

use std::path::Path;
use std::sync::Mutex;

use instant_distance::{Builder, HnswMap, Point, Search};
use serde::{Deserialize, Serialize};

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

/// task-23.1 persistence format version. A mismatch on load → rebuild-on-load (never a silent
/// mis-deserialize of an incompatible file).
const PERSIST_VERSION: u32 = 1;

/// task-23.1 on-disk form (path B): instant-distance's `HnswMap` is not serialized directly; the
/// already-unit-normalized `(embedding, chunk_id)` input set is the source of truth, and the graph
/// is rebuilt from it on load via `flush`. This still eliminates the expensive SQLite-enumerate +
/// re-embed of a cold start (embedding is the costly step, outside the hnsw graph build).
#[derive(Serialize, Deserialize)]
struct PersistedHnsw {
    version: u32,
    entries: Vec<(Vec<f32>, String)>,
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

    /// task-23.1: serialize the indexed `(unit-normalized embedding, chunk_id)` input set to `path`.
    /// Rebuilding the graph from these inputs on load is deterministic and avoids the cold-start
    /// SQLite-enumerate + re-embed cost.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), VectorError> {
        let persisted = {
            let pending = self.pending.lock().unwrap();
            PersistedHnsw {
                version: PERSIST_VERSION,
                entries: pending.clone(),
            }
        };
        let bytes = serde_json::to_vec(&persisted)
            .map_err(|e| VectorError::Backend { source: Box::new(e) })?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| VectorError::Backend { source: Box::new(e) })?;
        }
        std::fs::write(path, bytes).map_err(|e| VectorError::Backend { source: Box::new(e) })?;
        Ok(())
    }

    /// task-23.1: load a persisted input set from `path` and rebuild the graph. Returns `Ok(true)` on
    /// a successful load+rebuild; `Ok(false)` when the file is absent / version-incompatible /
    /// corrupt, signalling the caller to rebuild from scratch (rebuild-on-load fallback — never
    /// panics, never silently reports success on a bad file).
    pub fn load(&self, path: impl AsRef<Path>) -> Result<bool, VectorError> {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(_) => return Ok(false), // absent → rebuild-on-load
        };
        let persisted: PersistedHnsw = match serde_json::from_slice(&bytes) {
            Ok(p) => p,
            Err(_) => return Ok(false), // corrupt → rebuild-on-load
        };
        if persisted.version != PERSIST_VERSION {
            return Ok(false); // version-incompatible → rebuild-on-load
        }
        {
            let mut pending = self.pending.lock().unwrap();
            *pending = persisted.entries;
        }
        self.flush()?; // rebuild the graph from the restored inputs
        Ok(true)
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
    fn open(&self, config: VectorIndexConfig) -> Result<(), VectorError> {
        self.pending.lock().unwrap().clear();
        *self.map.lock().unwrap() = None;
        // task-23.1: persistence_path Some(path) → try to restore a persisted index; absent / corrupt
        // / incompatible → load returns Ok(false) and we stay empty (rebuild-on-load: caller
        // re-indexes). None → in-memory-only, byte-equivalent to the prior behavior.
        if let Some(path) = config.persistence_path.as_ref() {
            let _ = self.load(path)?;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retriever::vector::types::{VectorChunk, VectorMetric};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn tmp_path(name: &str) -> std::path::PathBuf {
        let seq = SEQ.fetch_add(1, Ordering::SeqCst);
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("cf-hnsw-{name}-{}-{nanos}-{seq}.bin", std::process::id()))
    }

    fn mk(id: &str, emb: Vec<f32>) -> VectorChunk {
        VectorChunk { chunk_id: ChunkId(id.into()), embedding: emb, metadata: None }
    }

    fn fixture() -> Vec<VectorChunk> {
        vec![
            mk("a", vec![1.0, 0.0, 0.0, 0.0]),
            mk("b", vec![0.0, 1.0, 0.0, 0.0]),
            mk("c", vec![0.0, 0.0, 1.0, 0.0]),
            mk("d", vec![0.0, 0.0, 0.0, 1.0]),
        ]
    }

    fn ids(hits: &[VectorHit]) -> Vec<String> {
        hits.iter().map(|h| h.chunk_id.0.clone()).collect()
    }

    // TEST-23.1.1 — AC1: index → save → new instance load → search hits equivalent to the original.
    #[test]
    fn test_23_1_1_persist_roundtrip() {
        let path = tmp_path("roundtrip");
        let orig = HnswBackend::new();
        orig.index_batch(&fixture()).unwrap();
        orig.flush().unwrap();
        let query = vec![0.9, 0.1, 0.0, 0.0]; // nearest "a"
        let want = ids(&orig.search(&query, 2, None).unwrap());
        assert!(!want.is_empty(), "original index must return hits");
        orig.save(&path).unwrap();

        let restored = HnswBackend::new();
        let loaded = restored.load(&path).unwrap();
        assert!(loaded, "load of a freshly-saved file must succeed");
        assert!(restored.is_indexed(), "restored backend must have a rebuilt graph");
        let got = ids(&restored.search(&query, 2, None).unwrap());
        assert_eq!(got, want, "restored search must hit the equivalent chunk_id order");

        let _ = std::fs::remove_file(&path);
    }

    // TEST-23.1.2 — AC2: load of an absent / corrupt file returns Ok(false) (rebuild-on-load); the
    // caller rebuilds and search is still correct — no panic, no silent success.
    #[test]
    fn test_23_1_2_rebuild_on_load_fallback() {
        let backend = HnswBackend::new();
        let absent = tmp_path("absent");
        assert!(!backend.load(&absent).unwrap(), "absent file must signal rebuild-on-load");
        let corrupt = tmp_path("corrupt");
        std::fs::write(&corrupt, b"not a valid persisted hnsw").unwrap();
        assert!(
            !backend.load(&corrupt).unwrap(),
            "corrupt file must signal rebuild-on-load (not panic)"
        );
        // caller rebuilds from scratch → search correct
        backend.index_batch(&fixture()).unwrap();
        backend.flush().unwrap();
        let hits = backend.search(&[0.0, 0.0, 0.9, 0.1], 1, None).unwrap();
        assert_eq!(ids(&hits), vec!["c".to_string()], "rebuilt index searches correctly");
        let _ = std::fs::remove_file(&corrupt);
    }

    // TEST-23.1.3 — AC3: persistence_path None → open is in-memory-only (clears, no load); the backend
    // stays usable through the three vector traits (signatures unchanged, task-18.1 freeze).
    #[test]
    fn test_23_1_3_none_path_inmemory_and_trait_objects() {
        let backend = HnswBackend::new();
        backend.index_batch(&fixture()).unwrap();
        let cfg = VectorIndexConfig {
            dim: 4,
            metric: VectorMetric::Cosine,
            persistence_path: None,
            collection_id: "t".into(),
        };
        backend.open(cfg).unwrap();
        assert!(!backend.is_indexed(), "open with None clears the in-memory index (prior behavior)");
        let _b: &dyn VectorBackend = &backend;
        let _i: &dyn VectorIndexer = &backend;
        let _s: &dyn VectorSearcher = &backend;
    }
}
