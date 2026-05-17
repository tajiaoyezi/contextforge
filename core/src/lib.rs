//! contextforge-core — ContextForge Rust data-plane crate.
//!
//! task-1.1 scope: expose the frozen proto / canonical-record contract
//! surface ([`contract`]) consumed by the conformance tests. Later phases
//! add scan/parse/chunk/index/retrieve modules behind this same crate.

pub mod contract;
