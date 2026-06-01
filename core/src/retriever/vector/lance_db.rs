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
                if dim % *num_sub_vectors != 0 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retriever::vector::traits::{VectorIndexer, VectorSearcher};
    use crate::retriever::vector::types::ChunkId;

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
}
