# Phase 20 spike — real SemanticRecall@K through the production `Retriever` hot path (task-20.2)

> **Data source declaration (ADR-013)**: every number below is from a **real `FastEmbedProvider`
> run** (`all-MiniLM-L6-v2`, dim 384) over real ContextForge source/doc text, routed through the
> **production `Retriever::search_semantic` hot path** — no synthetic, deterministic, or fabricated
> figures. Reproduce with:
> ```
> cargo run -p contextforge-core --example phase20_recall_via_retriever --features embedding-fastembed
> ```
> Run platform: WSL2 Ubuntu / rustc stable, 2026-05-31. The default build compiles the example as a
> no-op (0 fastembed/ort dependency); deterministic hot-path wiring is CI-covered (see §4).

## 1. Why this spike (vs task-19.5)

task-19.5 (`docs/spikes/phase-19-real-recall.md`) measured real recall against a **standalone**
`BruteForceVectorBackend` with a **controlled corpus** (40-line windows, `MAX_CHUNKS_PER_FILE = 4` →
40 chunks) — it proved the *embeddings* discriminate, but did **not** exercise the production
retrieval pipeline.

task-20.2 closes the v0.12.0 caveat (`v0.12.0-evidence.md` §3b / `task-19.4` §10): it routes recall
through the **real production path** — the same one `core/src/server.rs` (CoreService) and
`core/src/data_plane/search.rs` (console-api, task-20.1) use at request time:

1. Write the 6 golden expected files + 5 distractor files into a temp source tree.
2. Index them with the production **`IndexSession`** (real scanner + chunker) → a real collection.
3. Open a `Retriever`, wire the real `FastEmbedProvider` + the 0-dep default `BruteForceVectorBackend`,
   build the on-demand semantic index from the collection's own chunks via `enumerate_chunks` +
   `index_chunks_semantic`, then `Retriever::search_semantic` top-10 per golden query.
4. File-level SemanticRecall@5/@10 + top-1 accuracy + MRR (first hit whose file_path carries the
   query's expected category stem).

The production chunker is **uncapped** (unlike task-19.5's `MAX_CHUNKS_PER_FILE`), producing **175
chunks** from the 11 files.

## 2. Results (real `all-MiniLM-L6-v2`, via `Retriever::search_semantic`)

| metric | task-20.2 (production Retriever, 175 chunks) | task-19.5 (standalone backend, 40 capped chunks) |
|---|---|---|
| SemanticRecall@5 | **0.9667** (29/30) | 0.8333 |
| SemanticRecall@10 | **1.0000** (30/30) | 0.9333 |
| top-1 accuracy | **0.7333** | 0.60 |
| MRR | **0.8367** | 0.70 |
| gate (≥0.70) | **PASS** | PASS |

Per-category (n=5 each): config-location / error-reproduction / historical-decision / code-location /
agent-memory-rule all **recall@5=@10=1.0**; log-troubleshooting **@5=0.80 / @10=1.0**.

## 3. Honest interpretation (ADR-013)

- **The real path performs well, but recall@10=1.0 is partly chunk-count inflation.** With the
  uncapped production chunker, each expected file yields *many* chunks (175 total across 11 files),
  so "any chunk from the expected file in top-K" is mechanically easier to satisfy than on
  task-19.5's 4-chunk-per-file corpus. This is the same file-level inflation task-19.5 deliberately
  suppressed with `MAX_CHUNKS_PER_FILE`.
- **The discriminating metrics rule out pure inflation.** top-1 accuracy (0.7333) and MRR (0.8367)
  are *not* inflated by chunk count (top-1 is the single best hit), and both are **higher** than
  task-19.5's 0.60 / 0.70 — the production path genuinely ranks the right file first more often.
- **The two numbers are not directly comparable** (different corpora + chunking). Both clear the
  ADR-006 A1 gate (`SemanticRecall@10 ≥ 0.70`); task-20.2 is the **representative** measurement
  (production pipeline), task-19.5 is the **controlled** one (discrimination floor). Neither is
  fabricated.

## 4. Deterministic CI verification

The real run above needs the `embedding-fastembed` feature + model download and is **not** run in
CI. The production hot-path *wiring* (Retriever + the 0-dep default `BruteForceVectorBackend` +
`enumerate_chunks` + `index_chunks_semantic` + `search_semantic`, retrieval_method `"vector"`,
provenance floor) is covered in the **default build** by:

- `core/src/retriever/mod.rs::test_20_2_recall_via_retriever_brute_force_default_build` (deterministic
  embeddings, exact-text query hits its own chunk through `Retriever::search_semantic`).
- `core/src/retriever/mod.rs::test_19_2_semantic_roundtrip_hits_target_chunk` (hnsw-feature variant).
- `core/src/data_plane/search.rs::test_20_1_query_semantic_dispatches_vector_path` (console-api
  `SearchService.Query` semantic dispatch).

Deterministic embeddings carry no semantics — they prove plumbing, not recall (ADR-013); the recall
numbers come only from the real `FastEmbedProvider` run.
