# ContextForge Phase 11 Fixture File 3 — CancelToken Semantics

`task-11.3` requires co-operative cancellation: `JobService.Cancel` sets
`cancel_requested=1` in SQLite, and the indexer's progress callback returns
`ProgressDecision::Cancel` when it observes the flag. The `JobRunner.run_one`
then writes `status=cancelled` to the job row.

Why co-operative rather than hard kill?

- Rust's structured concurrency model: `tokio::spawn_blocking` cannot be
  externally cancelled without dropping the JoinHandle (which doesn't
  actually stop the worker thread)
- file-level granularity is enough: a fresh `IndexSession` will pick up the
  partial state via incremental reindex on next enqueue
- contextforge's local-first ADR-004 baseline means there's no remote IO
  that would otherwise stall the worker indefinitely

The `test_cancel_truly_stops` integration test asserts that after Cancel,
within 5 seconds the job's `status=cancelled` and the worker is no longer
processing files.
