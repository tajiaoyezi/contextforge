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
    ChunkId, VectorChunk, VectorError, VectorFilter, VectorHit, VectorIndexConfig, VectorMetric,
    VectorScore,
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

// ---- task-25.1: qdrant 生命周期契约层（连接配置 + health-probe + ensure-create 决策）----

/// 连接配置（url / 连接 timeout / 可选 api-key / 可选 TLS）。
#[derive(Debug, Clone)]
pub struct QdrantConnConfig {
    pub url: String,
    pub timeout: Option<std::time::Duration>,
    pub api_key: Option<String>,
    pub tls: bool,
}

impl QdrantConnConfig {
    /// 从环境构造（`QDRANT_URL` 既有 + 可选 `QDRANT_API_KEY`；TLS 由 url scheme 推断）。
    pub fn from_env() -> Self {
        let url =
            std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6334".to_string());
        let api_key = std::env::var("QDRANT_API_KEY").ok().filter(|s| !s.trim().is_empty());
        let tls = url.starts_with("https://");
        Self { url, timeout: None, api_key, tls }
    }

    /// 纯函数校验（不连 server）：url 非空 / dim>0 / collection 名非空 / metric 受支持。
    pub fn validate(&self, want: &VectorIndexConfig) -> Result<(), VectorError> {
        todo!("task-25.1 GREEN")
    }
}

/// health-probe 结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QdrantHealth {
    Ready,
    Unreachable,
}

/// 从 live collection 抽出的描述（dim + metric），用于 ensure-create 决策。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectionDesc {
    pub dim: usize,
    pub metric: VectorMetric,
}

/// ensure-create 决策（替代 spike 无脑 drop+create）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnsureAction {
    Reuse,
    Create,
    Error,
}

/// 纯函数：给定既有 collection 描述与期望配置，决定 reuse / create / error（不连 server）。
pub fn decide_ensure(existing: Option<CollectionDesc>, want: &VectorIndexConfig) -> EnsureAction {
    todo!("task-25.1 GREEN")
}

impl QdrantBackend {
    /// 从连接配置构造（health 探活 / 显式连接参数；client 懒连接，不在此打 server）。
    pub fn connect(conn: &QdrantConnConfig) -> Result<Self, VectorError> {
        let mut builder = Qdrant::from_url(&conn.url);
        builder = builder.api_key(conn.api_key.clone());
        if let Some(t) = conn.timeout {
            builder = builder.timeout(t);
        }
        builder = builder.skip_compatibility_check();
        let client = builder.build().map_err(to_backend_err)?;
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

    /// health-probe：live 返 Ready，无 server 返 Unreachable（不 panic、不静默成功）。
    pub fn health(&self) -> QdrantHealth {
        todo!("task-25.1 GREEN")
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn cfg(dim: usize, coll: &str) -> VectorIndexConfig {
        VectorIndexConfig {
            dim,
            metric: VectorMetric::Cosine,
            persistence_path: None,
            collection_id: coll.to_string(),
        }
    }

    // ---- TEST-25.1.1 (AC1) — 连接配置校验（纯函数，不连 server）----
    #[test]
    fn test_25_1_1_conn_config_validate() {
        let conn = QdrantConnConfig {
            url: "http://localhost:6334".to_string(),
            timeout: None,
            api_key: None,
            tls: false,
        };
        assert!(conn.validate(&cfg(384, "c")).is_ok(), "合法配置应 Ok");
        // url 空
        let mut bad = conn.clone();
        bad.url = "".to_string();
        assert!(bad.validate(&cfg(384, "c")).is_err(), "url 空应 Err");
        // dim=0
        assert!(conn.validate(&cfg(0, "c")).is_err(), "dim=0 应 Err");
        // collection 名空
        assert!(conn.validate(&cfg(384, "")).is_err(), "collection 名空应 Err");
    }

    // ---- TEST-25.1.2 (AC2) — health-probe 无 server 返 Unreachable（不 panic）----
    #[test]
    fn test_25_1_2_health_unreachable_no_server() {
        let conn = QdrantConnConfig {
            url: "http://127.0.0.1:59999".to_string(), // 无 server 监听
            timeout: Some(Duration::from_secs(2)),
            api_key: None,
            tls: false,
        };
        let be = QdrantBackend::connect(&conn).expect("connect 建 client（懒连接）");
        assert_eq!(be.health(), QdrantHealth::Unreachable, "无 server 应返 Unreachable");
    }

    // ---- TEST-25.1.3 (AC3) — ensure-create 决策三分支（纯函数，喂构造 desc）----
    #[test]
    fn test_25_1_3_decide_ensure_three_branches() {
        let want = cfg(384, "c");
        assert_eq!(decide_ensure(None, &want), EnsureAction::Create, "不存在 → Create");
        assert_eq!(
            decide_ensure(Some(CollectionDesc { dim: 384, metric: VectorMetric::Cosine }), &want),
            EnsureAction::Reuse,
            "存在且 dim/metric 匹配 → Reuse"
        );
        assert_eq!(
            decide_ensure(Some(CollectionDesc { dim: 256, metric: VectorMetric::Cosine }), &want),
            EnsureAction::Error,
            "存在但 dim 不匹配 → Error"
        );
        assert_eq!(
            decide_ensure(Some(CollectionDesc { dim: 384, metric: VectorMetric::L2 }), &want),
            EnsureAction::Error,
            "存在但 metric 不匹配 → Error"
        );
    }

    // ---- TEST-25.1.4 (AC4) — 不破坏三 trait 签名（trait object 构造）----
    #[test]
    fn test_25_1_4_trait_objects_construct() {
        let conn = QdrantConnConfig {
            url: "http://127.0.0.1:59999".to_string(),
            timeout: Some(Duration::from_millis(500)),
            api_key: None,
            tls: false,
        };
        let be = QdrantBackend::connect(&conn).unwrap();
        let _b: &dyn VectorBackend = &be;
        let _i: &dyn VectorIndexer = &be;
        let _s: &dyn VectorSearcher = &be;
        assert_eq!(be.name(), "qdrant");
        assert!(!be.is_local(), "qdrant 是外部 server，is_local()==false");
    }
}
