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

/// task-11.1 (Phase 11): tonic/prost generated bindings for
/// `package contextforge.console_data_plane.v1` (ADR-016 §D2).
/// Separated from `pb` to keep Phase 9 Index gRPC contract frozen.
pub mod pb_console {
    tonic::include_proto!("contextforge.console_data_plane.v1");
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

// task-10.2 (Phase 10): Console Contract v1 Workspace resource + SQLite 持久化.
pub mod workspace;

// task-10.3 (Phase 10): Console Contract v1 IndexJob 异步 lifecycle + heartbeat.
pub mod jobs;

// task-11.1 (Phase 11, ADR-016): Console data plane gRPC services
// (WorkspaceService / JobService / SearchService / EventsService).
pub mod data_plane;
