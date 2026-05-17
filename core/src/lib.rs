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
