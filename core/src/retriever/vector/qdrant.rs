//! task-18.4 spike: Qdrant backend via `qdrant-client` (gRPC to a local Qdrant server).
//! Gated behind the `vector-qdrant` feature.
//!
//! Unlike the in-process backends (hnsw / sqlite-vec), Qdrant is an external server process
//! (`is_local() == false`). The async `qdrant-client` is bridged to the sync trait surface via an
//! owned current-thread tokio runtime + `block_on` (the bench harness has no ambient runtime).
//! `Distance::Cosine` is used directly, so Qdrant's KNN matches the harness's cosine ground truth.

use std::sync::Mutex;

use qdrant_client::qdrant::point_id::PointIdOptions;
use qdrant_client::qdrant::{
    CreateCollectionBuilder, Distance, PointStruct, SearchPointsBuilder, UpsertPointsBuilder,
    VectorParamsBuilder,
};
use qdrant_client::{Payload, Qdrant};

use crate::retriever::vector::traits::{VectorBackend, VectorIndexer, VectorSearcher};
use crate::retriever::vector::types::{
    ChunkId, VectorChunk, VectorError, VectorFilter, VectorHit, VectorIndexConfig, VectorScore,
};

const UPSERT_BATCH: usize = 1000;

fn to_backend_err<E: std::error::Error + Send + Sync + 'static>(e: E) -> VectorError {
    VectorError::Backend { source: Box::new(e) }
}

/// Qdrant backend: a gRPC client to an external Qdrant server. `Qdrant` and `tokio::Runtime` are
/// both `Send + Sync`; `id_map` maps Qdrant's numeric point id back to the chunk id.
pub struct QdrantBackend {
    client: Qdrant,
    rt: tokio::runtime::Runtime,
    id_map: Mutex<Vec<String>>,
    collection: Mutex<String>,
    dim: Mutex<usize>,
}

impl QdrantBackend {
    pub fn new() -> Result<Self, VectorError> {
        let url =
            std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6334".to_string());
        let client = Qdrant::from_url(&url).build().map_err(to_backend_err)?;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(to_backend_err)?;
        Ok(Self {
            client,
            rt,
            id_map: Mutex::new(Vec::new()),
            collection: Mutex::new("spike".to_string()),
            dim: Mutex::new(0),
        })
    }
}

impl std::fmt::Debug for QdrantBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("QdrantBackend")
    }
}

impl VectorBackend for QdrantBackend {
    fn name(&self) -> &'static str {
        "qdrant"
    }
    fn version(&self) -> &'static str {
        "1.18"
    }
    fn is_local(&self) -> bool {
        // Qdrant is an external server process, not an in-process library.
        false
    }
    fn requires_embedding(&self) -> bool {
        true
    }
}

impl VectorIndexer for QdrantBackend {
    fn open(&self, config: VectorIndexConfig) -> Result<(), VectorError> {
        let collection = config.collection_id.clone();
        let dim = config.dim as u64;
        self.rt.block_on(async {
            let _ = self.client.delete_collection(&collection).await;
            self.client
                .create_collection(
                    CreateCollectionBuilder::new(&collection)
                        .vectors_config(VectorParamsBuilder::new(dim, Distance::Cosine)),
                )
                .await
                .map_err(to_backend_err)
        })?;
        *self.collection.lock().unwrap() = collection;
        *self.dim.lock().unwrap() = config.dim;
        self.id_map.lock().unwrap().clear();
        Ok(())
    }

    fn index_batch(&self, chunks: &[VectorChunk]) -> Result<usize, VectorError> {
        let dim = *self.dim.lock().unwrap();
        let collection = self.collection.lock().unwrap().clone();
        let mut id_map = self.id_map.lock().unwrap();
        let mut points: Vec<PointStruct> = Vec::with_capacity(chunks.len());
        for c in chunks {
            if c.embedding.len() != dim {
                return Err(VectorError::DimMismatch {
                    expected: dim,
                    got: c.embedding.len(),
                });
            }
            let id = id_map.len() as u64;
            points.push(PointStruct::new(id, c.embedding.clone(), Payload::new()));
            id_map.push(c.chunk_id.0.clone());
        }
        self.rt.block_on(async {
            for batch in points.chunks(UPSERT_BATCH) {
                self.client
                    .upsert_points(UpsertPointsBuilder::new(&collection, batch.to_vec()).wait(true))
                    .await
                    .map_err(to_backend_err)?;
            }
            Ok::<(), VectorError>(())
        })?;
        Ok(chunks.len())
    }

    fn delete(&self, _ids: &[ChunkId]) -> Result<usize, VectorError> {
        // Qdrant spike semantics = full reindex: drop and recreate the collection.
        let collection = self.collection.lock().unwrap().clone();
        let dim = *self.dim.lock().unwrap() as u64;
        let mut id_map = self.id_map.lock().unwrap();
        let n = id_map.len();
        self.rt.block_on(async {
            let _ = self.client.delete_collection(&collection).await;
            self.client
                .create_collection(
                    CreateCollectionBuilder::new(&collection)
                        .vectors_config(VectorParamsBuilder::new(dim, Distance::Cosine)),
                )
                .await
                .map_err(to_backend_err)
        })?;
        id_map.clear();
        Ok(n)
    }

    fn flush(&self) -> Result<(), VectorError> {
        // upsert_points(wait=true) already persisted; nothing to flush.
        Ok(())
    }

    fn close(&self) -> Result<(), VectorError> {
        Ok(())
    }
}

impl VectorSearcher for QdrantBackend {
    fn search(
        &self,
        query_vec: &[f32],
        k: usize,
        _filter: Option<&VectorFilter>,
    ) -> Result<Vec<VectorHit>, VectorError> {
        let collection = self.collection.lock().unwrap().clone();
        let id_map = self.id_map.lock().unwrap();
        if id_map.is_empty() {
            return Ok(vec![]);
        }
        let result = self.rt.block_on(async {
            self.client
                .search_points(SearchPointsBuilder::new(
                    &collection,
                    query_vec.to_vec(),
                    k as u64,
                ))
                .await
                .map_err(to_backend_err)
        })?;
        let mut hits = Vec::with_capacity(k);
        for point in result.result {
            let id_num = match point.id.and_then(|p| p.point_id_options) {
                Some(PointIdOptions::Num(n)) => n as usize,
                _ => continue,
            };
            let id = match id_map.get(id_num) {
                Some(s) => s.clone(),
                None => continue,
            };
            // Qdrant returns the cosine similarity directly (higher = closer, best first).
            let score = VectorScore::new(point.score).unwrap_or_else(|_| VectorScore::new(0.0).unwrap());
            hits.push(VectorHit {
                chunk_id: ChunkId(id),
                score,
                metadata: None,
            });
        }
        Ok(hits)
    }

    fn is_indexed(&self) -> bool {
        !self.id_map.lock().unwrap().is_empty()
    }
}
