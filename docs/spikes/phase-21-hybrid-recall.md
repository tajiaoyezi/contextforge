# Phase 21 spike — hybrid (RRF) + reranked (cross-encoder) recall vs the BM25 baseline (task-21.3)

> **Data source declaration (ADR-013)**: every number below is from a **real model run** — the real
> `FastEmbedProvider` (`all-MiniLM-L6-v2`, dim 384) for the vector component of hybrid, and the real
> `CrossEncoderReranker` (fastembed `BGERerankerBase`) for the reranked pass — routed through the
> **production `Retriever`** (`search` / `search_hybrid` / `search_hybrid` + `with_reranker`) over real
> ContextForge source/doc text. No synthetic, deterministic, or fabricated figures. Reproduce with:
> ```
> cargo run -p contextforge-core --example phase21_hybrid_rerank_recall \
>   --features embedding-fastembed,reranker-fastembed
> ```
> Run platform: Windows MSVC (cygwin bash driver), 2026-05-31 (cf. task-19.1 / phase-21-reranker
> fastembed Win MSVC credential). The default build compiles the example as a no-op (0 fastembed/ort
> dependency); deterministic hybrid/rerank wiring is CI-covered (see §4). Offline note: the run used the
> two models pre-fetched by the phase-20 / phase-21-reranker spikes (HuggingFace was unreachable at run
> time); both `all-MiniLM-L6-v2` + `BGE-reranker-base` ONNX models were served from the local
> `.fastembed_cache` — no number was fabricated to paper over the offline state (ADR-013).

## 1. What this spike measures (vs phase-20)

`phase-20-recall-via-retriever.md` measured **semantic-only** recall through `Retriever::search_semantic`.
task-21.3 extends the same dogfood harness to compare **three retrieval methods over the SAME corpus +
30 golden queries**, to feed the ADR-025 (hybrid fusion) and ADR-026 (reranker provider) ratification:

- **baseline BM25** — `Retriever::search` (Tantivy词面, ADR-002).
- **hybrid RRF** — `Retriever::search_hybrid` (RRF k=60 fusion of BM25 + the vector path, task-21.1 / ADR-025).
- **reranked cross-encoder** — `search_hybrid` top-k re-ordered via `with_reranker(CrossEncoderReranker)`
  (real BGE-reranker-base, task-21.2 / ADR-026 D4: rerank applies once on the fused top-k).

Harness (`core/examples/phase21_hybrid_rerank_recall.rs`): write the 6 golden expected files + 5
distractors into a temp tree → index via the production `IndexSession` (real scanner + chunker → **180
production chunks**) → wire real `FastEmbedProvider` + 0-dep `BruteForceVectorBackend` → for each of the
30 golden queries, file-level recall@5/@10 + top-1 + MRR (first hit whose file_path carries the query's
unique category stem). Same uncapped-chunker file-level inflation caveat as task-20.2 applies to the
recall@K columns; **top-1 / MRR are the discriminating metrics** (single best hit, not chunk-count
inflated).

## 2. Results (real run, ADR-013)

`provider=fastembed-all-minilm-l6-v2 dim=384 production_chunks=180 queries=30 backend=BruteForceVectorBackend(exact-cosine)`

| method | recall@5 | recall@10 | top-1 | MRR | ADR-006 gate (≥0.70) |
|---|---|---|---|---|---|
| baseline BM25 | 0.9000 | 0.9667 | 0.0333 | 0.4095 | **PASS** |
| **hybrid RRF** (ADR-025) | 0.9333 | 0.9667 | **0.6667** | **0.7881** | **PASS** |
| reranked cross-encoder (ADR-026) | **0.9667** | 0.9667 | 0.3333 | 0.6306 | **PASS** |

Deltas vs the BM25 baseline (the discriminating metrics):

- **hybrid RRF**: top-1 **+0.6334** (0.0333 → 0.6667), MRR **+0.3786** (0.4095 → 0.7881), recall@5 +0.0333.
- **reranked cross-encoder**: top-1 **+0.3000** (→ 0.3333), MRR **+0.2211** (→ 0.6306), recall@5 **+0.0667** (best of all three).

## 3. Honest interpretation (ADR-013)

- **BM25 alone almost never ranks the right file FIRST on this corpus** (top-1 = 0.0333 = 1/30) although
  the right file is usually somewhere in the top-10 (recall@10 = 0.9667). The uncapped production chunker
  splits each file into many chunks; BM25's #1 chunk is frequently a term-overlap hit from a distractor
  or sibling file. This is exactly the single-path blind spot ADR-025 set out to cover.
- **Hybrid RRF fusion is the decisive win.** Adding the vector semantic signal to BM25 via RRF lifts the
  right chunk to rank 1 **twenty-fold** (top-1 0.0333 → 0.6667) and nearly doubles MRR (0.4095 → 0.7881),
  with equal recall@10 and better recall@5. This is a clear, real, data-driven validation of the RRF
  fusion strategy — **ratifies ADR-025 (D2 default RRF) Proposed → Accepted**. The fusion strategy
  selection (RRF vs weighted-normalized) is no longer "indistinguishable on synthetic data" (ADR-025
  R1 / ADR-023 D6 stop-condition): RRF demonstrably and substantially beats the BM25 baseline on real
  dogfood data, so RRF is confirmed as the default with no need to fall back to architecture-only
  selection.
- **The real cross-encoder ran (the ADR-026 D5 stop-condition did NOT trigger).** `CrossEncoderReranker`
  (BGE-reranker-base) loaded and reranked the fused top-k on real ContextForge text. It beats the BM25
  baseline on every metric (top-1 +0.30, MRR +0.22) and produces the **best recall@5 of all three
  methods** (0.9667) — the joint (query, doc) cross-encoder scoring promotes a true expected-file chunk
  into the top-5 that fusion ranked 6th–10th.
- **Honest caveat: reranking the hybrid top-k does NOT beat hybrid alone on top-1/MRR over this corpus.**
  reranked top-1 (0.3333) < hybrid (0.6667) and reranked MRR (0.6306) < hybrid (0.7881). Two honest
  reasons: (a) BGE-reranker-base is trained on general/web text, not code+ADR chunks, so its joint
  relevance signal is weaker on this domain than the in-domain dual-encoder + BM25 fusion; (b) the
  reranker re-orders an already-strong fused top-k, so it can only hurt the items fusion already ranked
  well. The reranker's value here is **recall@5** (pulling a missed expected chunk up into the top-5),
  not top-1 on top of an already-good fusion. This is a corpus/model-fit finding, **not** a refutation of
  the pipeline — it informs *when* to enable rerank (opt-in, domain-fit dependent), consistent with the
  opt-in seam design (ADR-026 D4 — default build never reranks).

## 4. Deterministic CI verification

The real run above needs `embedding-fastembed` + `reranker-fastembed` + the ONNX models and is **not**
run in CI. The default-build wiring (0 model dep) is covered by:

- `core/src/server.rs::test_21_1_hybrid_dispatches_fusion_path` — `req.hybrid` dispatches the RRF fusion
  path; `retrieval_method = "hybrid"` + `hybrid_score > 0` (deterministic embeddings prove dispatch).
- `core/src/rerank/identity.rs::test_21_2_1_*` + `core/src/retriever/mod.rs::test_21_2_2_*` —
  `IdentityReranker` deterministic re-order + the `with_reranker` seam (None → unchanged).
- `internal/eval` + `internal/cli` `TestTask213_*` — the eval Report hybrid/reranked columns +
  `--hybrid` / `--rerank` flags + `SummarizePasses` byte-equivalence (Go side).

Deterministic embeddings carry no semantics — they prove plumbing, not recall (ADR-013); the recall /
top-1 / MRR numbers come only from the real `FastEmbedProvider` + `CrossEncoderReranker` run above.

## 5. Verdict (feeds ADR-025 / ADR-026 ratification)

- **ADR-025 hybrid-scoring-fusion → Accepted.** Real dogfood data shows RRF fusion delivers a decisive
  top-1 / MRR uplift over the BM25 baseline (top-1 +0.63, MRR +0.38); RRF is confirmed as the default
  strategy on real (non-synthetic) data, resolving the ADR-025 R1 "indistinguishable on synthetic data"
  open point.
- **ADR-026 reranker-provider → Accepted (with caveat).** The real cross-encoder ran (D5 stop-condition
  not triggered) and provides genuine top-1 / MRR uplift over the BM25 baseline + the best recall@5; the
  trait + deterministic-default + feature-gated provider + opt-in seam architecture is validated end to
  end on a real model. Honest caveat (above): on this small code-centric corpus, reranking the already
  strong hybrid top-k does not beat hybrid on top-1/MRR — rerank is therefore recommended as an opt-in,
  domain-fit-dependent enhancement, never a default (the seam keeps the default build rerank-free).
