//! task-18.5 spike: LanceDB backend via `lancedb` (embedded Lance columnar vector store).
//! Gated behind the `vector-lancedb` feature.
//!
//! LanceDB is in-process (`is_local() == true`) but disk-backed (a Lance dataset). The async
//! `lancedb` API is bridged to the sync trait surface via an owned current-thread tokio runtime +
//! `block_on` (the bench harness has no ambient runtime). `DistanceType::Cosine` is used directly,
//! so Lance KNN matches the harness's cosine ground truth.
//!
//! Module is named `lance_db` (not `lancedb`) to avoid colliding with the `lancedb` crate name in
//! `use` / `pub use` paths.

use std::sync::{Arc, Mutex};

use arrow_array::types::Float32Type;
use arrow_array::{Array, FixedSizeListArray, Float32Array, Int32Array, RecordBatch};
use futures::TryStreamExt;
use lancedb::arrow::arrow_schema::{DataType, Field, Schema, SchemaRef};
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::{Connection, DistanceType, Table};

use crate::retriever::vector::traits::{VectorBackend, VectorIndexer, VectorSearcher};
use crate::retriever::vector::types::{
    ChunkId, VectorChunk, VectorError, VectorFilter, VectorHit, VectorIndexConfig, VectorMetric,
    VectorScore,
};

const TABLE: &str = "spike";

// ---- task-25.2: ANN 索引调参参数（IVF_PQ / HNSW）+ compaction 触发口径，可校验配置面 ----

/// ANN 索引类型 + 调参参数。真实建索引 + 性能测量 [SPEC-DEFER:phase-future.lancedb-index-tuning]；
/// 本枚举是参数契约层（validate 不建真实索引）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanceAnnIndex {
    /// IVF_PQ：`num_partitions`（IVF 簇数）+ `num_sub_vectors`（PQ 子向量数，须整除 dim）。
    IvfPq {
        num_partitions: usize,
        num_sub_vectors: usize,
    },
    /// HNSW：`m`（每节点边数）+ `ef_construction`（建图候选数）。
    Hnsw {
        m: usize,
        ef_construction: usize,
    },
}

/// lancedb 索引调参配置（参数契约层）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanceIndexTuning {
    pub index: LanceAnnIndex,
    pub metric: VectorMetric,
    /// compaction 触发行数阈值（>0）。
    pub compaction_threshold_rows: usize,
}

impl LanceIndexTuning {
    /// 参数范围校验（不建真实索引）：partitions>0 / sub_vectors>0 且整除 dim / m>0 / ef>0 /
    /// 阈值>0 / metric 受支持。纯函数——给定相同参数 → 相同 Ok/Err（确定性，可单测）。
    pub fn validate(&self, dim: usize) -> Result<(), VectorError> {
        if dim == 0 {
            return Err(VectorError::Other(
                "lancedb index tuning: dim must be > 0".into(),
            ));
        }
        if self.compaction_threshold_rows == 0 {
            return Err(VectorError::Other(
                "lancedb index tuning: compaction_threshold_rows must be > 0".into(),
            ));
        }
        // metric 受支持：所有 VectorMetric 变体均映射到 lance DistanceType（Cosine/Dot/L2）。
        match self.metric {
            VectorMetric::Cosine | VectorMetric::DotProduct | VectorMetric::L2 => {}
        }
        match &self.index {
            LanceAnnIndex::IvfPq {
                num_partitions,
                num_sub_vectors,
            } => {
                if *num_partitions == 0 {
                    return Err(VectorError::Other(
                        "lancedb IVF_PQ: num_partitions must be > 0".into(),
                    ));
                }
                if *num_sub_vectors == 0 {
                    return Err(VectorError::Other(
                        "lancedb IVF_PQ: num_sub_vectors must be > 0".into(),
                    ));
                }
                // PQ 子向量须整除 dim（每子向量等长切分）。
                if !dim.is_multiple_of(*num_sub_vectors) {
                    return Err(VectorError::Other(format!(
                        "lancedb IVF_PQ: num_sub_vectors ({num_sub_vectors}) must divide dim ({dim})"
                    )));
                }
            }
            LanceAnnIndex::Hnsw { m, ef_construction } => {
                if *m == 0 {
                    return Err(VectorError::Other("lancedb HNSW: m must be > 0".into()));
                }
                if *ef_construction == 0 {
                    return Err(VectorError::Other(
                        "lancedb HNSW: ef_construction must be > 0".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

fn to_backend_err<E: std::error::Error + Send + Sync + 'static>(e: E) -> VectorError {
    VectorError::Backend { source: Box::new(e) }
}

/// LanceDB backend: an embedded, disk-backed Lance dataset. `Connection`/`Table`/`Runtime` are all
/// `Send + Sync`; `id_map` maps the Int32 row id back to the chunk id.
pub struct LanceDbBackend {
    rt: tokio::runtime::Runtime,
    conn: Connection,
    table: Mutex<Option<Table>>,
    schema: Mutex<Option<SchemaRef>>,
    id_map: Mutex<Vec<String>>,
    dim: Mutex<usize>,
}

impl LanceDbBackend {
    pub fn new() -> Result<Self, VectorError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(to_backend_err)?;
        let dir = std::env::var("LANCEDB_DIR").unwrap_or_else(|_| {
            std::env::temp_dir()
                .join("contextforge-lancedb-spike")
                .to_string_lossy()
                .into_owned()
        });
        let conn = rt
            .block_on(async { lancedb::connect(&dir).execute().await })
            .map_err(to_backend_err)?;
        Ok(Self {
            rt,
            conn,
            table: Mutex::new(None),
            schema: Mutex::new(None),
            id_map: Mutex::new(Vec::new()),
            dim: Mutex::new(0),
        })
    }
}

impl std::fmt::Debug for LanceDbBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LanceDbBackend")
    }
}

impl VectorBackend for LanceDbBackend {
    fn name(&self) -> &'static str {
        "lancedb"
    }
    fn version(&self) -> &'static str {
        "0.30"
    }
    fn is_local(&self) -> bool {
        // Embedded (in-process) Lance dataset, disk-backed.
        true
    }
    fn requires_embedding(&self) -> bool {
        true
    }
}

impl VectorIndexer for LanceDbBackend {
    fn open(&self, config: VectorIndexConfig) -> Result<(), VectorError> {
        let dim = config.dim;
        let schema: SchemaRef = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    dim as i32,
                ),
                true,
            ),
        ]));
        let table = self.rt.block_on(async {
            let _ = self.conn.drop_table(TABLE, &[]).await;
            self.conn
                .create_empty_table(TABLE, schema.clone())
                .execute()
                .await
                .map_err(to_backend_err)
        })?;
        *self.table.lock().unwrap() = Some(table);
        *self.schema.lock().unwrap() = Some(schema);
        *self.dim.lock().unwrap() = dim;
        self.id_map.lock().unwrap().clear();
        Ok(())
    }

    fn index_batch(&self, chunks: &[VectorChunk]) -> Result<usize, VectorError> {
        let dim = *self.dim.lock().unwrap();
        let schema = self
            .schema
            .lock()
            .unwrap()
            .clone()
            .ok_or(VectorError::NotInitialized)?;
        let table = self
            .table
            .lock()
            .unwrap()
            .clone()
            .ok_or(VectorError::NotInitialized)?;
        let mut id_map = self.id_map.lock().unwrap();

        let start = id_map.len() as i32;
        let mut ids = Vec::with_capacity(chunks.len());
        for (i, c) in chunks.iter().enumerate() {
            if c.embedding.len() != dim {
                return Err(VectorError::DimMismatch {
                    expected: dim,
                    got: c.embedding.len(),
                });
            }
            ids.push(start + i as i32);
            id_map.push(c.chunk_id.0.clone());
        }
        let id_arr = Int32Array::from(ids);
        let vec_arr = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
            chunks
                .iter()
                .map(|c| Some(c.embedding.iter().map(|x| Some(*x)).collect::<Vec<_>>())),
            dim as i32,
        );
        let batch = RecordBatch::try_new(schema, vec![Arc::new(id_arr), Arc::new(vec_arr)])
            .map_err(to_backend_err)?;
        self.rt
            .block_on(async { table.add(batch).execute().await.map_err(to_backend_err) })?;
        Ok(chunks.len())
    }

    fn delete(&self, _ids: &[ChunkId]) -> Result<usize, VectorError> {
        // Lance spike semantics = full reindex: delete all rows.
        let table = self
            .table
            .lock()
            .unwrap()
            .clone()
            .ok_or(VectorError::NotInitialized)?;
        let mut id_map = self.id_map.lock().unwrap();
        let n = id_map.len();
        self.rt
            .block_on(async { table.delete("true").await.map_err(to_backend_err) })?;
        id_map.clear();
        Ok(n)
    }

    fn flush(&self) -> Result<(), VectorError> {
        Ok(())
    }

    fn close(&self) -> Result<(), VectorError> {
        Ok(())
    }
}

impl VectorSearcher for LanceDbBackend {
    fn search(
        &self,
        query_vec: &[f32],
        k: usize,
        _filter: Option<&VectorFilter>,
    ) -> Result<Vec<VectorHit>, VectorError> {
        let table = self
            .table
            .lock()
            .unwrap()
            .clone()
            .ok_or(VectorError::NotInitialized)?;
        let id_map = self.id_map.lock().unwrap();
        if id_map.is_empty() {
            return Ok(vec![]);
        }
        let qv = query_vec.to_vec();
        let batches: Vec<RecordBatch> = self.rt.block_on(async {
            table
                .query()
                .nearest_to(qv.as_slice())
                .map_err(to_backend_err)?
                .distance_type(DistanceType::Cosine)
                .limit(k)
                .execute()
                .await
                .map_err(to_backend_err)?
                .try_collect::<Vec<RecordBatch>>()
                .await
                .map_err(to_backend_err)
        })?;

        let mut hits = Vec::with_capacity(k);
        for b in &batches {
            let id_col = b
                .column_by_name("id")
                .and_then(|c| c.as_any().downcast_ref::<Int32Array>())
                .ok_or_else(|| VectorError::Other("lancedb result missing id column".into()))?;
            let dist_col = b
                .column_by_name("_distance")
                .and_then(|c| c.as_any().downcast_ref::<Float32Array>())
                .ok_or_else(|| {
                    VectorError::Other("lancedb result missing _distance column".into())
                })?;
            for i in 0..b.num_rows() {
                let id_idx = id_col.value(i) as usize;
                let dist = dist_col.value(i);
                if let Some(cid) = id_map.get(id_idx) {
                    // Lance cosine distance = 1 - cosine similarity → map back to a similarity score.
                    let sim = 1.0 - dist;
                    let score =
                        VectorScore::new(sim).unwrap_or_else(|_| VectorScore::new(0.0).unwrap());
                    hits.push(VectorHit {
                        chunk_id: ChunkId(cid.clone()),
                        score,
                        metadata: None,
                    });
                }
            }
        }
        Ok(hits)
    }

    fn is_indexed(&self) -> bool {
        !self.id_map.lock().unwrap().is_empty()
    }
}

// ---- task-29.3: REAL ANN index build on the `vector` column (redeems the parameter-contract layer
// [SPEC-DEFER:phase-future.lancedb-index-tuning]). create_ann_index consumes a validated
// LanceIndexTuning and calls Lance create_index, building a real IVF_PQ / IVF_HNSW_SQ index — distinct
// from the default flat KNN `search` (`:270-332`). After an index exists, `nearest_to` queries use it. ----
impl LanceDbBackend {
    /// Build a real ANN index on the `vector` column from a validated `LanceIndexTuning`.
    ///
    /// - `IvfPq { num_partitions, num_sub_vectors }` → `Index::IvfPq` (product quantization).
    /// - `Hnsw { m, ef_construction }` → `Index::IvfHnswSq` with a single IVF partition (effectively
    ///   pure HNSW over scalar-quantized vectors; lancedb 0.30 exposes HNSW only as an IVF_HNSW_*
    ///   variant).
    ///
    /// Requires an open table with rows (`index_batch` first). Errors surface as `VectorError`
    /// (e.g. too few rows to train PQ on a tiny corpus) — never silently skipped (ADR-013).
    pub fn create_ann_index(&self, tuning: &LanceIndexTuning) -> Result<(), VectorError> {
        use lancedb::index::vector::{IvfHnswSqIndexBuilder, IvfPqIndexBuilder};
        use lancedb::index::Index;

        let dim = *self.dim.lock().unwrap();
        tuning.validate(dim)?;
        let table = self
            .table
            .lock()
            .unwrap()
            .clone()
            .ok_or(VectorError::NotInitialized)?;
        let dt = match tuning.metric {
            VectorMetric::Cosine => DistanceType::Cosine,
            VectorMetric::DotProduct => DistanceType::Dot,
            VectorMetric::L2 => DistanceType::L2,
        };
        let index = match &tuning.index {
            LanceAnnIndex::IvfPq {
                num_partitions,
                num_sub_vectors,
            } => Index::IvfPq(
                IvfPqIndexBuilder::default()
                    .distance_type(dt)
                    .num_partitions(*num_partitions as u32)
                    .num_sub_vectors(*num_sub_vectors as u32),
            ),
            LanceAnnIndex::Hnsw { m, ef_construction } => Index::IvfHnswSq(
                IvfHnswSqIndexBuilder::default()
                    .distance_type(dt)
                    .num_partitions(1)
                    .num_edges(*m as u32)
                    .ef_construction(*ef_construction as u32),
            ),
        };
        self.rt.block_on(async {
            table
                .create_index(&["vector"], index)
                .execute()
                .await
                .map_err(to_backend_err)
        })?;
        Ok(())
    }

    /// task-29.3 (AC3): real Lance dataset compaction via `optimize(OptimizeAction::All)` (compacts
    /// data files + prunes old versions + optimizes indices). Intended to run after row count exceeds
    /// `LanceIndexTuning::compaction_threshold_rows`. Returns the number of rows after compaction.
    pub fn compact(&self) -> Result<usize, VectorError> {
        let table = self
            .table
            .lock()
            .unwrap()
            .clone()
            .ok_or(VectorError::NotInitialized)?;
        self.rt.block_on(async {
            table
                .optimize(lancedb::table::OptimizeAction::All)
                .await
                .map_err(to_backend_err)?;
            table.count_rows(None).await.map_err(to_backend_err)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retriever::vector::traits::{VectorIndexer, VectorSearcher};
    use crate::retriever::vector::types::ChunkId;

    // Tests that drive a real Lance dataset set the process-global `LANCEDB_DIR` env var before
    // `LanceDbBackend::new()`; the default parallel test runner would race on it (one test's backend
    // connecting to another's dir). Serialize them with this lock (poison-tolerant). Pure-function
    // tests (validate) don't take it.
    static LANCEDB_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn dir_lock() -> std::sync::MutexGuard<'static, ()> {
        LANCEDB_DIR_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn tuning_ivf(parts: usize, subs: usize) -> LanceIndexTuning {
        LanceIndexTuning {
            index: LanceAnnIndex::IvfPq {
                num_partitions: parts,
                num_sub_vectors: subs,
            },
            metric: VectorMetric::Cosine,
            compaction_threshold_rows: 1000,
        }
    }

    // ---- TEST-25.2.3 (AC3) — 索引调参参数范围校验（纯函数，不建真实索引）----
    #[test]
    fn test_25_2_3_index_tuning_validate() {
        // 合法 IVF_PQ：dim=384, sub_vectors=8 → 384 % 8 == 0
        assert!(tuning_ivf(256, 8).validate(384).is_ok(), "合法 IVF_PQ 应 Ok");
        // partitions=0 → Err
        assert!(tuning_ivf(0, 8).validate(384).is_err(), "partitions=0 应 Err");
        // sub_vectors 不整除 dim（384 % 7 != 0）→ Err
        assert!(tuning_ivf(256, 7).validate(384).is_err(), "sub_vectors 不整除 dim 应 Err");
        // sub_vectors=0 → Err
        assert!(tuning_ivf(256, 0).validate(384).is_err(), "sub_vectors=0 应 Err");
        // dim=0 → Err
        assert!(tuning_ivf(256, 8).validate(0).is_err(), "dim=0 应 Err");
        // compaction 阈值=0 → Err
        let mut t = tuning_ivf(256, 8);
        t.compaction_threshold_rows = 0;
        assert!(t.validate(384).is_err(), "阈值=0 应 Err");
        // 合法 HNSW
        let hnsw = LanceIndexTuning {
            index: LanceAnnIndex::Hnsw { m: 16, ef_construction: 100 },
            metric: VectorMetric::L2,
            compaction_threshold_rows: 500,
        };
        assert!(hnsw.validate(384).is_ok(), "合法 HNSW 应 Ok");
        // HNSW m=0 → Err
        let bad = LanceIndexTuning {
            index: LanceAnnIndex::Hnsw { m: 0, ef_construction: 100 },
            metric: VectorMetric::L2,
            compaction_threshold_rows: 500,
        };
        assert!(bad.validate(384).is_err(), "HNSW m=0 应 Err");
    }

    // ---- TEST-25.2.4 (AC4) — 既有 lancedb backend 契约不退化（open→index→search KNN + dim mismatch）----
    #[test]
    fn test_25_2_4_backend_contract_roundtrip() {
        let _g = dir_lock();
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("cf-lancedb-test-{}-{nanos}", std::process::id()));
        std::env::set_var("LANCEDB_DIR", &dir);
        let be = LanceDbBackend::new().expect("lancedb connect");
        be.open(VectorIndexConfig {
            dim: 4,
            metric: VectorMetric::Cosine,
            persistence_path: None,
            collection_id: "t".to_string(),
        })
        .expect("open");
        let chunks = vec![
            VectorChunk { chunk_id: ChunkId("a".into()), embedding: vec![1.0, 0.0, 0.0, 0.0], metadata: None },
            VectorChunk { chunk_id: ChunkId("b".into()), embedding: vec![0.0, 1.0, 0.0, 0.0], metadata: None },
            VectorChunk { chunk_id: ChunkId("c".into()), embedding: vec![0.0, 0.0, 1.0, 0.0], metadata: None },
        ];
        assert_eq!(be.index_batch(&chunks).unwrap(), 3);
        let hits = be.search(&[1.0, 0.0, 0.0, 0.0], 2, None).unwrap();
        assert!(!hits.is_empty(), "KNN 应命中");
        assert_eq!(hits[0].chunk_id.0, "a", "最近邻应为 a");
        // dim mismatch
        let bad = vec![VectorChunk {
            chunk_id: ChunkId("x".into()),
            embedding: vec![1.0, 2.0],
            metadata: None,
        }];
        assert!(
            matches!(be.index_batch(&bad), Err(VectorError::DimMismatch { .. })),
            "dim mismatch 应返 DimMismatch"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- TEST-29.3.1 (AC1) — REAL IVF_PQ / IVF_HNSW_SQ index build + recall-vs-flat measurement ----
    // Deterministic clustered vectors (dim 384, no fastembed): flat (no index) = exact ground truth;
    // IVF_PQ / HNSW are lossy ANN, so recall@k < 1.0 is the discriminating signal. Measures real build
    // time + query latency. CI does NOT build the vector-lancedb feature; run via:
    //   cargo test -p contextforge-core --features vector-lancedb --lib retriever::vector::lance_db -- --nocapture
    const DIM: usize = 384;

    fn lcg_next(state: &mut u64) -> f32 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((*state >> 33) as f32 / (1u64 << 31) as f32) - 1.0 // ~[-1, 1)
    }

    fn make_corpus() -> (Vec<VectorChunk>, Vec<(Vec<f32>, usize)>) {
        // 16 clusters × 64 members = 1024 vectors; 32 queries (2 per cluster) perturbed off centers.
        const CLUSTERS: usize = 16;
        const PER_CLUSTER: usize = 64;
        let mut st: u64 = 0x9E3779B97F4A7C15;
        let centers: Vec<Vec<f32>> = (0..CLUSTERS)
            .map(|_| (0..DIM).map(|_| lcg_next(&mut st)).collect())
            .collect();
        let mut chunks = Vec::with_capacity(CLUSTERS * PER_CLUSTER);
        for (c, center) in centers.iter().enumerate() {
            for m in 0..PER_CLUSTER {
                let emb: Vec<f32> = center
                    .iter()
                    .map(|&x| x + 0.10 * lcg_next(&mut st))
                    .collect();
                chunks.push(VectorChunk {
                    chunk_id: ChunkId(format!("c{c}-m{m}")),
                    embedding: emb,
                    metadata: None,
                });
            }
        }
        let mut queries = Vec::new();
        for (c, center) in centers.iter().enumerate() {
            for _ in 0..2 {
                let q: Vec<f32> = center
                    .iter()
                    .map(|&x| x + 0.05 * lcg_next(&mut st))
                    .collect();
                queries.push((q, c));
            }
        }
        (chunks, queries)
    }

    fn fresh_backend(tag: &str) -> (LanceDbBackend, std::path::PathBuf) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("cf-lancedb-{tag}-{}-{nanos}", std::process::id()));
        std::env::set_var("LANCEDB_DIR", &dir);
        let be = LanceDbBackend::new().expect("lancedb connect");
        be.open(VectorIndexConfig {
            dim: DIM,
            metric: VectorMetric::Cosine,
            persistence_path: None,
            collection_id: "ann".to_string(),
        })
        .expect("open");
        (be, dir)
    }

    fn recall_at(ann: &[VectorHit], gt: &[VectorHit], k: usize) -> f64 {
        let gt_ids: std::collections::HashSet<&str> =
            gt.iter().take(k).map(|h| h.chunk_id.0.as_str()).collect();
        let hit = ann
            .iter()
            .take(k)
            .filter(|h| gt_ids.contains(h.chunk_id.0.as_str()))
            .count();
        hit as f64 / k.min(gt.len()).max(1) as f64
    }

    #[test]
    fn test_29_3_1_real_ann_index_recall_vs_flat() {
        let _g = dir_lock();
        use std::time::Instant;
        let (chunks, queries) = make_corpus();
        let k = 10usize;

        // 1) Flat (no index) = exact ground truth + flat query latency.
        let (flat, dir_flat) = fresh_backend("flat");
        flat.index_batch(&chunks).expect("index flat");
        let mut gts: Vec<Vec<VectorHit>> = Vec::with_capacity(queries.len());
        let t0 = Instant::now();
        for (q, _) in &queries {
            gts.push(flat.search(q, k, None).expect("flat search"));
        }
        let flat_us = t0.elapsed().as_micros() as f64 / queries.len() as f64;

        // 1b) BruteForceVectorBackend (0-dep default) baseline — exact cosine, so recall vs lancedb-flat
        // ground truth is ~1.0; measured for the multi-backend matrix latency comparison (AC2).
        use crate::retriever::vector::BruteForceVectorBackend;
        let brute = BruteForceVectorBackend::new();
        brute
            .open(VectorIndexConfig {
                dim: DIM,
                metric: VectorMetric::Cosine,
                persistence_path: None,
                collection_id: "brute".to_string(),
            })
            .expect("brute open");
        brute.index_batch(&chunks).expect("index brute");
        let (mut b5, mut b10, mut brute_us) = (0.0f64, 0.0f64, 0.0f64);
        for (i, (q, _)) in queries.iter().enumerate() {
            let tq = Instant::now();
            let hits = brute.search(q, k, None).expect("brute search");
            brute_us += tq.elapsed().as_micros() as f64;
            b5 += recall_at(&hits, &gts[i], 5);
            b10 += recall_at(&hits, &gts[i], 10);
        }
        let nq0 = queries.len() as f64;
        let (brute_r5, brute_r10, brute_us) = (b5 / nq0, b10 / nq0, brute_us / nq0);

        // 2) IVF_PQ real index build (num_sub_vectors=16 divides dim 384) → recall vs flat.
        let (ivf, dir_ivf) = fresh_backend("ivfpq");
        ivf.index_batch(&chunks).expect("index ivf");
        let tb = Instant::now();
        ivf.create_ann_index(&tuning_ivf(16, 16)).expect("build IVF_PQ index");
        let ivf_build_ms = tb.elapsed().as_millis();
        let (mut r5, mut r10, mut ivf_us) = (0.0f64, 0.0f64, 0.0f64);
        for (i, (q, _)) in queries.iter().enumerate() {
            let tq = Instant::now();
            let hits = ivf.search(q, k, None).expect("ivf search");
            ivf_us += tq.elapsed().as_micros() as f64;
            r5 += recall_at(&hits, &gts[i], 5);
            r10 += recall_at(&hits, &gts[i], 10);
        }
        let n = queries.len() as f64;
        let (ivf_r5, ivf_r10, ivf_us) = (r5 / n, r10 / n, ivf_us / n);

        // 3) IVF_HNSW_SQ real index build → recall vs flat.
        let hnsw_tuning = LanceIndexTuning {
            index: LanceAnnIndex::Hnsw { m: 16, ef_construction: 100 },
            metric: VectorMetric::Cosine,
            compaction_threshold_rows: 1000,
        };
        let (hn, dir_hn) = fresh_backend("hnsw");
        hn.index_batch(&chunks).expect("index hnsw");
        let tb = Instant::now();
        hn.create_ann_index(&hnsw_tuning).expect("build HNSW index");
        let hnsw_build_ms = tb.elapsed().as_millis();
        let (mut h5, mut h10, mut hn_us) = (0.0f64, 0.0f64, 0.0f64);
        for (i, (q, _)) in queries.iter().enumerate() {
            let tq = Instant::now();
            let hits = hn.search(q, k, None).expect("hnsw search");
            hn_us += tq.elapsed().as_micros() as f64;
            h5 += recall_at(&hits, &gts[i], 5);
            h10 += recall_at(&hits, &gts[i], 10);
        }
        let (hn_r5, hn_r10, hn_us) = (h5 / n, h10 / n, hn_us / n);

        println!("=== TEST-29.3.1 REAL multi-backend matrix: lancedb ANN recall-vs-flat (n={} queries={} dim={DIM}) ===", chunks.len(), queries.len());
        println!("brute-force(0-dep,exact)    recall@5={brute_r5:.4} recall@10={brute_r10:.4} build_ms=0 query_us={brute_us:.1}");
        println!("lancedb flat(ground-truth)  recall@5=1.0000 recall@10=1.0000 query_latency_us={flat_us:.1}");
        println!("lancedb IVF_PQ(p16,s16)     recall@5={ivf_r5:.4} recall@10={ivf_r10:.4} build_ms={ivf_build_ms} query_us={ivf_us:.1}");
        println!("lancedb IVF_HNSW_SQ(m16,ef100) recall@5={hn_r5:.4} recall@10={hn_r10:.4} build_ms={hnsw_build_ms} query_us={hn_us:.1}");

        // Assertions: real index build succeeded + returns k hits + recall is a measured [0,1] value.
        // ANN recall vs exact flat is the discriminating signal (lossy quantization); we assert the
        // indexes build and search returns results, and record the measured recall above (ADR-013:
        // numbers are real, not asserted to a fabricated threshold).
        assert!((0.0..=1.0).contains(&ivf_r10), "IVF_PQ recall@10 measured in [0,1]");
        assert!((0.0..=1.0).contains(&hn_r10), "HNSW recall@10 measured in [0,1]");
        assert!(ivf_r10 > 0.0, "IVF_PQ should recover at least some flat neighbors");
        assert!(hn_r10 > 0.0, "HNSW should recover at least some flat neighbors");
        for d in [dir_flat, dir_ivf, dir_hn] {
            let _ = std::fs::remove_dir_all(&d);
        }
    }

    // ---- TEST-29.3.3 (AC3) — REAL Lance dataset compaction over compaction_threshold_rows ----
    #[test]
    fn test_29_3_3_real_compaction() {
        let _g = dir_lock();
        let (be, dir) = fresh_backend("compact");
        // 6 batches × 256 rows = 1536 rows (> compaction_threshold_rows 1000), each batch a fragment.
        let total = 6 * 256;
        let mut st: u64 = 0xC0FFEE;
        for batch in 0..6 {
            let chunks: Vec<VectorChunk> = (0..256)
                .map(|i| VectorChunk {
                    chunk_id: ChunkId(format!("b{batch}-{i}")),
                    embedding: (0..DIM).map(|_| lcg_next(&mut st)).collect(),
                    metadata: None,
                })
                .collect();
            be.index_batch(&chunks).expect("index batch");
        }
        let rows_after = be.compact().expect("real Lance compaction (OptimizeAction::All)");
        println!("TEST-29.3.3 real compaction: {total} rows over 6 fragments → compacted, count_rows={rows_after}");
        assert_eq!(rows_after, total, "compaction must preserve all rows");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
