//! task-18.1: vector retrieval trait abstraction + NoopVectorBackend placeholder.
//!
//! Three sync traits (VectorBackend / VectorIndexer / VectorSearcher) +
//! related types + NoopVectorBackend stub. No real backend deps — those
//! come in task-18.3-18.6.

pub mod traits;
pub mod types;
pub mod noop;
// task-19.3: default-available exact-cosine searcher (0 dep) for the opt-in semantic path.
pub mod brute_force;
// task-29.1: vector backend selection factory (mirrors embedding::factory::select_provider).
pub mod factory;

#[cfg(feature = "vector-hnsw")]
pub mod hnsw;

#[cfg(feature = "vector-sqlite")]
pub mod sqlite_vec;

#[cfg(feature = "vector-qdrant")]
pub mod qdrant;

#[cfg(feature = "vector-lancedb")]
pub mod lance_db;

#[cfg(test)]
mod tests;

pub use traits::{VectorBackend, VectorIndexer, VectorSearcher};
pub use types::{ChunkId, VectorChunk, VectorError, VectorFilter, VectorHit, VectorIndexConfig, VectorMetric, VectorScore};
pub use noop::NoopVectorBackend;
pub use brute_force::BruteForceVectorBackend;
pub use factory::select_vector_backend;

#[cfg(feature = "vector-hnsw")]
pub use hnsw::HnswBackend;

#[cfg(feature = "vector-sqlite")]
pub use sqlite_vec::SqliteVecBackend;

#[cfg(feature = "vector-qdrant")]
pub use qdrant::QdrantBackend;

#[cfg(feature = "vector-lancedb")]
pub use lance_db::LanceDbBackend;
