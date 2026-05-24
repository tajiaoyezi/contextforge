# ContextForge Phase 11 Fixture File 5 — End-to-End Verification

When all 5 files in this directory are indexed by `IndexSession`, the
resulting chunks contain the word "contextforge" at least 2× per file,
giving `Retriever::search("contextforge", top_k=5)` enough text mass to
return ≥1 SourceChunk per file.

The Phase 11 §6 AC4 phase-level smoke checks:

1. POST `/v1/index-jobs` with `workspace_id` pointing at this fixture
2. Within 1s status transitions queued → running
3. Within 30s status transitions running → succeeded
4. `processed_files == 5` and `total_files == 5`
5. POST `/v1/search` returns ≥1 chunk whose `source_file_path` matches one
   of the files in this fixture

`test_job_succeeds_real_index` is the task-11.3 integration test owner of
the first 4 steps; `test_search_real_chunks` (task-11.4) owns step 5.

Together they prove ADR-016 D1/D2/D3 are honoured end-to-end —
contextforge persistence is single-source-of-truth Rust, the Go REST
layer is a thin protocol translator, and the data plane gRPC services
truly drive the index pipeline.
