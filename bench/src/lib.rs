//! `contextforge-bench` (task-18.2): vector backend spike measurement harness.
//!
//! Measures the Phase 18 §2A 5 dimensions — recall@5/10, P95 latency, RSS, cold-start,
//! reindex — for any backend implementing the frozen `VectorIndexer + VectorSearcher` traits.
//! Corpus is generated from a deterministic seed (no external embedding model); the only
//! wired backend at task-18.2 is `NoopVectorBackend` (real backends arrive in task-18.3-18.6).

pub mod backends;
pub mod corpus;
pub mod measure;
pub mod rss;
pub mod runner;

#[cfg(test)]
mod tests;
