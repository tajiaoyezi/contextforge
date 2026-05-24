//! task-14.1 (ADR-017 D1 Wave 4): Eval persistence + orchestration module.
//!
//! Owns the `eval_runs` SQLite table (migration `0014_eval_runs.sql`) and
//! exposes a `SqliteEvalStore` whose surface is consumed by
//! [`crate::data_plane::eval::EvalServer`] (3 gRPC RPCs: Create / Get /
//! UpdateProgress). Triggering the recall harness itself is the Go side's
//! responsibility (task-14.2); this module only persists state.

pub mod runner;
pub mod store;

pub use runner::EvalRunner;
pub use store::{CaseResult, EvalRun, EvalRunCreate, EvalStoreError, SqliteEvalStore};
