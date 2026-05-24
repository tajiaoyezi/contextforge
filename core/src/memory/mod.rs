//! task-13.1 (ADR-017 D1 Wave 3): Memory persistence + state-ops module.
//!
//! Owns the `memory_items` SQLite table (migration `0013_memory_items.sql`) and
//! exposes a `SqliteMemoryStore` whose surface is consumed by
//! [`crate::data_plane::memory::MemoryServer`] (5 gRPC RPCs: List / Get / Pin /
//! Deprecate / SoftDelete). Pin / Deprecate / SoftDelete each emit a
//! `crate::memoryops::audit` event so the operations trace appears in the
//! existing AuditSink stream.

pub mod store;

pub use store::{MemoryItem, MemoryListFilter, MemoryStoreError, SqliteMemoryStore};
