//! task-14.1 EvalRunner — placeholder orchestration layer.
//!
//! task-14.1 §3: this Rust-side type intentionally does NOT spawn the recall
//! harness itself. The recall harness lives in Go (`internal/eval/eval.go`)
//! and is triggered by `internal/consoleapi/eval_runner.go::runEvalAsync`
//! goroutine in task-14.2 [SPEC-OWNER:task-14.2]. That goroutine then calls
//! back into `EvalService.UpdateProgress` to persist metrics + case_results.
//!
//! This type exists for future-compat: when v1.x adopts a Rust-native runner
//! (`[SPEC-DEFER:phase-future.rust-native-eval-runner]`), it will own the
//! tokio::spawn_blocking + harness invocation here.
//!
//! task-24.3 (Phase 24) re-evaluated promoting this placeholder to a real Rust-native runner
//! and **honestly deferred** it (`[SPEC-DEFER:phase-future.rust-native-eval-runner]`): the Go
//! harness (`internal/eval/eval.go`) stays the single source of truth for the recall 口径
//! (task-14.1's deliberate choice), and a Rust-native runner would duplicate `SemanticRecallAtK`
//! / gate logic across two languages → drift risk with no current consumer. Ad-hoc Rust recall
//! measurement (when a spike needs it, e.g. the task-24.3 tokenizer before/after delta) is covered
//! by `core/examples/phase24_tokenizer_recall.rs` without a permanent runner. ADR-013: not faked
//! as implemented. See docs/spikes/phase-24-tokenizer-recall.md §runner-evaluation.

use std::sync::Arc;

use super::store::SqliteEvalStore;

pub struct EvalRunner {
    #[allow(dead_code)]
    store: Arc<SqliteEvalStore>,
}

impl EvalRunner {
    pub fn new(store: Arc<SqliteEvalStore>) -> Self {
        Self { store }
    }

    /// task-14.1: stub. Real triggering happens on the Go side per task-14.2
    /// `runEvalAsync` goroutine [SPEC-OWNER:task-14.2].
    pub fn trigger_external(&self, _eval_run_id: &str, _callback_url: &str) {
        // intentionally noop in v0.7
    }
}
