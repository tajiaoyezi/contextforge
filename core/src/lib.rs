//! contextforge-core — ContextForge Rust data-plane crate.
//!
//! task-1.1 scope: expose the frozen proto / canonical-record contract.
//! [`pb`] holds the tonic/prost code-generated bindings; [`contract`] is the
//! conformance surface consumed by the proto-contract tests. Later phases add
//! scan/parse/chunk/index/retrieve modules behind this same crate.

/// tonic/prost generated bindings for `package contextforge.v1`.
pub mod pb {
    tonic::include_proto!("contextforge.v1");
}

pub mod contract;

// task-1.3: tonic gRPC server skeleton + health (AC1/AC2/AC3).
pub mod server;

// task-1.3 (AC4): Phase 2+ data-plane module placeholders (compile, no logic).
pub mod chunker;
pub mod indexer;
pub mod memoryops;
pub mod parser;
pub mod retriever;
pub mod scanner;
