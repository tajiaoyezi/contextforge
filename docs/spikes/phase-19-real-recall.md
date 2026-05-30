# Phase 19 — real-embedding SemanticRecall@K (task-19.5 → ADR-023 ratify input)

> **Real provider, real ContextForge text, real measurement (ADR-013: no synthetic / deterministic
> / fabricated numbers).** Embeddings produced by the task-19.1 `FastEmbedProvider`
> (`fastembed-all-minilm-l6-v2`, ONNX `all-MiniLM-L6-v2`, **dim 384**) on a **Linux x86_64 host**
> (WSL2 Ubuntu 26.04, rustc 1.96.0), indexed into the default-available `BruteForceVectorBackend`
> (exact cosine), measured by `core/examples/phase19_real_recall.rs`. This resolves the single biggest
> caveat carried out of Phase 18: *synthetic seed vectors gave non-discriminating recall@5/10 = 1.0
> across all four backends* (`docs/spikes/phase-18-comparison.md`). On real-distribution embeddings,
> recall **is** discriminating.

## Why this exists

Phase 18 measured four vector backends on the same synthetic corpus and found recall identical
(`1.0 / 1.0`) at n=5k and n=100k — the seed vectors were too well-separated to differentiate exact
from ANN. ADR-023 D1 (`sqlite-vec` embedded default) was therefore left **PROVISIONAL**, and ADR-006
Amendment A1 (`SemanticRecall@10 ≥ 0.70`) **Proposed/aspirational**, both pending a real-embedding
recall measurement (ADR-023 D6, ADR-006 A1.3). task-19.1 (real `EmbeddingProvider`) + task-19.2
(default backend wired into `Retriever`) + task-19.3 (semantic gRPC path) closed those gaps; this task
produces the real recall data those ratifications depend on.

## Methodology

- **Provider**: `FastEmbedProvider` (`name() = "fastembed-all-minilm-l6-v2"`, `dim() = 384`), real ONNX
  inference — **not** the `DeterministicEmbeddingProvider` hash/seed vectors (which, like the Phase 18
  synthetic seeds, are in the already-proven non-discriminating class and cannot ratify — ADR-013).
- **Corpus** (real ContextForge source/doc text): the 6 golden-question expected files
  (`internal/eval` `BuiltinGoldenQuestions`) + 5 extra real files as distractors. Each file is chunked
  into 40-line windows, **capped at the first 4 windows per file** so no large file (e.g.
  `core/src/retriever/mod.rs`, 39 windows uncapped) dominates the corpus and inflates a file-level
  "any chunk in top-K" hit to a trivial 1.0. Result: **40 chunks across 11 files**.
- **Queries**: the 30 built-in golden questions (6 categories × 5), verbatim from
  `BuiltinGoldenQuestions`. Each query's ground truth = its category's expected file.
- **Retrieval**: every chunk + query embedded by the real provider; chunks indexed into
  `BruteForceVectorBackend` (exact cosine, unit-normalized dot product); each query searched top-10.
- **Metric**: file-level Strong-hit@K — `SemanticRecall@K` = (# queries whose expected **file** has a
  chunk in the top K) / 30, matching the `internal/eval` `SemanticRecallAtK` Strong-hit口径 (task-18.8).
  Supplemented with **top-1 accuracy** and **MRR** of the first expected-file hit — far more
  discriminating than "any chunk in top-K" on a balanced corpus.

> **Backend representativeness**: `BruteForceVectorBackend` is **exact** cosine, so its recall is
> identical to any other exact backend — including the ADR-023 D1 provisional pick `sqlite-vec`
> (also exact) — and is an **upper bound** for the approximate/ANN backends (hnsw). Measuring recall
> with exact cosine is therefore the correct, backend-agnostic read for the D1 / A1 ratification.

## Results (real `all-MiniLM-L6-v2`, exact cosine, 40-chunk balanced corpus, 30 queries)

| metric | value |
|---|---|
| **SemanticRecall@5** | **0.8333** (25/30) |
| **SemanticRecall@10** | **0.9333** (28/30) |
| top-1 accuracy | 0.6000 (18/30) |
| MRR (first expected-file hit) | 0.7029 |
| **ADR-006 A1 gate (`SemanticRecall@10 ≥ 0.70`)** | **PASS** |

### Per-category breakdown (n=5 each)

| category | expected file | recall@5 | recall@10 |
|---|---|---|---|
| config-location | `internal/config/config.go` | 1.0000 | 1.0000 |
| error-reproduction | `internal/daemon/daemon.go` | 1.0000 | 1.0000 |
| historical-decision | `docs/decisions/adr-007-minimal-tarball-distribution.md` | 1.0000 | 1.0000 |
| log-troubleshooting | `internal/memoryops/audit/audit.go` | 1.0000 | 1.0000 |
| code-location | `core/src/retriever/mod.rs` | 0.6000 | 0.8000 |
| agent-memory-rule | `docs/s2v-adapter.md` | 0.4000 | 0.8000 |

## What the data says

- **Recall is now discriminating** (the Phase 18 caveat is resolved). On real embeddings the headline
  is `0.83 / 0.93` (not `1.0 / 1.0`), top-1 is `0.60`, MRR is `0.70`, and per-category recall@5 ranges
  `0.40 – 1.00`. The variance is interpretable: queries over distinctive prose/config
  (`config`, `error`, `historical`, `log`) retrieve their file perfectly; queries over one-of-many
  similar code/doc sections (`code-location` among many `.rs` files; `agent-memory-rule` over the large
  `s2v-adapter.md`) are harder, especially at K=5.
- **The A1 gate passes**: `SemanticRecall@10 = 0.9333 ≥ 0.70`. This is the real-embedding measurement
  ADR-006 A1.3 and ADR-023 D6 were waiting on.
- **Honest scope of the claim**: this validates that the real embedding model + exact-cosine retrieval
  achieve the A1 gate on a real-text golden set. It does **not** rank the four ADR-023 backends against
  each other (all exact backends share this recall; ANN is bounded above by it) — backend selection
  stays on the Phase 18 latency/RSS/cold-start evidence (ADR-023 D1–D5), which this recall result does
  not disturb.

## Reproduce

```bash
cargo run -p contextforge-core --example phase19_real_recall --features embedding-fastembed
# Linux x86_64 (WSL2 Ubuntu 26.04, rustc 1.96.0) — downloads the ONNX model on first run.
# Writes test/fixtures/eval/dogfood-embeddings.jsonl (40 real-embedding lines) and prints the report above.
```

The model is deterministic for a given input, so reruns reproduce these counts; the committed
`test/fixtures/eval/dogfood-embeddings.jsonl` pins the exact 384-d vectors. The default build
(`cargo build/test --workspace`, no `embedding-fastembed`) compiles the example as a no-op stub — zero
new dependency, no impact on the default workspace build.

## Feeds

- **task-19.6** — ratify ADR-023 (D1 `sqlite-vec` provisional → the recall blocker is cleared; the gate
  passes) and promote ADR-006 Amendment A1 from Proposed/aspirational to ratified
  (`SemanticRecall@10 ≥ 0.70` met at 0.9333). Record Phase 18 AC3/AC4 as resolved **without editing the
  Phase 18 spec** (ADR-014 D5; add-only ADR amendments).
- **task-19.7** — Phase 19 closeout cites this evidence for the v0.12.0 release notes.

## Data-source declaration (ADR-013)

Every number on this page is from a **real `FastEmbedProvider` run** (real ONNX `all-MiniLM-L6-v2`
inference over real ContextForge text). No synthetic seed vectors, no `DeterministicEmbeddingProvider`
hash vectors, no hand-authored or fabricated recall figures. The real provider built and ran on the
Linux x86_64 host above (the task-19.1 R1 stop-condition did **not** trigger), so the honest-defer
branch (blocked + ratify-deferred) does not apply — real recall was produced.
