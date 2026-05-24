# ContextForge Phase 11 Fixture File 1

This fixture is consumed by `task-11.3` integration tests that verify
`JobService.Enqueue` truly triggers `JobRunner.spawn_blocking(IndexSession::
index_path_with_progress)`.

## Why "contextforge" appears in every file

The Phase 11 task-11.4 SearchService integration test
(`test_search_real_chunks`) queries for the literal `"contextforge"` token
on the workspace indexed from this fixture directory. Each file in
`test/fixtures/index-job-real/` therefore mentions the project name
multiple times so the retriever produces ≥1 SourceChunk hit per file.

## ADR-016 context

- D1: Rust holds the persistence SoT (this directory's chunks land in the
  Rust-owned SQLite + Tantivy index, not in any Go-side store).
- D2: the gRPC `JobService.Enqueue` RPC arrives via `internal/consoleapi/
  grpcclient` (task-11.2), is dispatched into `core/src/data_plane/job.rs`,
  and runs `IndexSession` through the `IndexerBackend` trait (task-11.3).
- D3: handler is a thin proxy — every business decision (status advance,
  heartbeat persistence, cancel honoring) lives in Rust.

This file should land as ≥1 chunk after indexing.
