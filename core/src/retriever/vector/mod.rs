//! task-18.1: vector retrieval trait abstraction + NoopVectorBackend placeholder.
//!
//! Three sync traits (VectorBackend / VectorIndexer / VectorSearcher) +
//! related types + NoopVectorBackend stub. No real backend deps — those
//! come in task-18.3-18.6.

pub mod traits;
pub mod types;
pub mod noop;

#[cfg(feature = "vector-hnsw")]
pub mod hnsw;

#[cfg(test)]
mod tests;

pub use traits::{VectorBackend, VectorIndexer, VectorSearcher};
pub use types::{ChunkId, VectorChunk, VectorError, VectorFilter, VectorHit, VectorIndexConfig, VectorMetric, VectorScore};
pub use noop::NoopVectorBackend;

#[cfg(feature = "vector-hnsw")]
pub use hnsw::HnswBackend;
