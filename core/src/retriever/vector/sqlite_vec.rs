//! task-18.3 spike: sqlite-vec backend via the `vec0` SQLite extension (asg017/sqlite-vec).
//! Gated behind the `vector-sqlite` feature.
//!
//! Vectors are unit-normalized and compared by the `vec0` default L2 distance, which is monotonic
//! with cosine similarity for unit vectors — so `vec0` KNN matches the cosine ground truth the
//! harness uses (the same approach as the hnsw backend).

use std::sync::{Mutex, Once};

use rusqlite::{ffi::sqlite3_auto_extension, Connection};
use sqlite_vec::sqlite3_vec_init;

use crate::retriever::vector::traits::{VectorBackend, VectorIndexer, VectorSearcher};
use crate::retriever::vector::types::{
    ChunkId, VectorChunk, VectorError, VectorFilter, VectorHit, VectorIndexConfig, VectorScore,
};

static REGISTER: Once = Once::new();

/// Register sqlite-vec's `vec0` virtual table for every subsequent SQLite connection.
/// `Once` guards against double-registration when several backend instances are created.
fn register_extension() {
    REGISTER.call_once(|| unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    });
}

fn normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        return v.to_vec();
    }
    v.iter().map(|x| x / norm).collect()
}

fn to_backend_err<E: std::error::Error + Send + Sync + 'static>(e: E) -> VectorError {
    VectorError::Backend { source: Box::new(e) }
}

/// sqlite-vec backend: an in-memory SQLite connection holding a `vec0` virtual table.
/// `Mutex<Connection>` keeps the backend `Send + Sync` (rusqlite `Connection` is `Send`, and the
/// trait surface is all `&self`). `id_map` maps the `vec0` integer rowid back to the chunk id.
pub struct SqliteVecBackend {
    conn: Mutex<Connection>,
    id_map: Mutex<Vec<String>>,
    dim: Mutex<usize>,
}

impl SqliteVecBackend {
    pub fn new() -> Result<Self, VectorError> {
        register_extension();
        let conn = Connection::open_in_memory().map_err(to_backend_err)?;
        Ok(Self {
            conn: Mutex::new(conn),
            id_map: Mutex::new(Vec::new()),
            dim: Mutex::new(0),
        })
    }
}

impl std::fmt::Debug for SqliteVecBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SqliteVecBackend")
    }
}

impl VectorBackend for SqliteVecBackend {
    fn name(&self) -> &'static str {
        "sqlite-vec"
    }
    fn version(&self) -> &'static str {
        "0.1.9"
    }
    fn is_local(&self) -> bool {
        true
    }
    fn requires_embedding(&self) -> bool {
        true
    }
}

impl VectorIndexer for SqliteVecBackend {
    fn open(&self, config: VectorIndexConfig) -> Result<(), VectorError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("DROP TABLE IF EXISTS vec_items;")
            .map_err(to_backend_err)?;
        conn.execute(
            &format!(
                "CREATE VIRTUAL TABLE vec_items USING vec0(embedding float[{}])",
                config.dim
            ),
            [],
        )
        .map_err(to_backend_err)?;
        *self.dim.lock().unwrap() = config.dim;
        self.id_map.lock().unwrap().clear();
        Ok(())
    }

    fn index_batch(&self, chunks: &[VectorChunk]) -> Result<usize, VectorError> {
        let dim = *self.dim.lock().unwrap();
        let conn = self.conn.lock().unwrap();
        let mut id_map = self.id_map.lock().unwrap();
        let tx = conn.unchecked_transaction().map_err(to_backend_err)?;
        {
            let mut stmt = tx
                .prepare_cached("INSERT INTO vec_items(rowid, embedding) VALUES (?, ?)")
                .map_err(to_backend_err)?;
            for c in chunks {
                if c.embedding.len() != dim {
                    return Err(VectorError::DimMismatch {
                        expected: dim,
                        got: c.embedding.len(),
                    });
                }
                let rowid = id_map.len() as i64;
                let nv = normalize(&c.embedding);
                let json = serde_json::to_string(&nv).map_err(to_backend_err)?;
                stmt.execute(rusqlite::params![rowid, json])
                    .map_err(to_backend_err)?;
                id_map.push(c.chunk_id.0.clone());
            }
        }
        tx.commit().map_err(to_backend_err)?;
        Ok(chunks.len())
    }

    fn delete(&self, _ids: &[ChunkId]) -> Result<usize, VectorError> {
        // vec0 spike semantics = full reindex: clear the table and the id map.
        let conn = self.conn.lock().unwrap();
        let mut id_map = self.id_map.lock().unwrap();
        let n = id_map.len();
        conn.execute("DELETE FROM vec_items", [])
            .map_err(to_backend_err)?;
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

impl VectorSearcher for SqliteVecBackend {
    fn search(
        &self,
        query_vec: &[f32],
        k: usize,
        _filter: Option<&VectorFilter>,
    ) -> Result<Vec<VectorHit>, VectorError> {
        let conn = self.conn.lock().unwrap();
        let id_map = self.id_map.lock().unwrap();
        if id_map.is_empty() {
            return Ok(vec![]);
        }
        let q = normalize(query_vec);
        let json = serde_json::to_string(&q).map_err(to_backend_err)?;
        let mut stmt = conn
            .prepare_cached(
                "SELECT rowid, distance FROM vec_items WHERE embedding MATCH ? ORDER BY distance LIMIT ?",
            )
            .map_err(to_backend_err)?;
        let rows = stmt
            .query_map(rusqlite::params![json, k as i64], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?))
            })
            .map_err(to_backend_err)?;
        let mut hits = Vec::with_capacity(k);
        for row in rows {
            let (rowid, dist) = row.map_err(to_backend_err)?;
            let id = match id_map.get(rowid as usize) {
                Some(s) => s.clone(),
                None => continue,
            };
            // L2 distance over unit vectors is in [0, 2]; map to a [0, 1] similarity score.
            let sim = 1.0 - (dist as f32) / 2.0;
            let score = VectorScore::new(sim).unwrap_or_else(|_| VectorScore::new(0.0).unwrap());
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
