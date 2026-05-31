# Phase 21 spike — reranker pipeline (task-21.2)

> **Data source declaration (ADR-013)**: this doc records the reranker pipeline across three honestly
> labelled tiers — (a) **deterministic identity** (`IdentityReranker`, CI-verifiable, 0 model), (b)
> **real cross-encoder compile** (`CrossEncoderReranker` behind `reranker-fastembed`, compiles
> against the real fastembed API), and (c) **real cross-encoder quality numbers** (top-1 / MRR uplift
> on a dogfood corpus). Tier (a)+(b) are confirmed here. Tier (c) — the substantive recall/MRR uplift
> measurement — is owned by `task-21.3` closeout dogfood eval; nothing here is synthetic or fabricated.

## 1. What the reranker pipeline is

Phase 19 semantic retrieval scores query and doc *independently* (dual-encoder cosine,
`Retriever::search_semantic`). A **cross-encoder** scores the (query, doc) **pair** jointly — more
precise — so reranking the initial top-k can lift top-1 / MRR. task-21.2 lands the pipeline:

- **`Reranker` trait** (`core/src/rerank/traits.rs`) — `Send + Sync + Debug`, object-safe
  (`Arc<dyn Reranker>`), `#[non_exhaustive] RerankError` (mirrors the task-19.1 `EmbeddingProvider`
  pattern; downstream `match` stays add-only-safe).
- **`IdentityReranker`** (`core/src/rerank/identity.rs`) — the model-free **deterministic default**
  (like ADR-023's 0-dep `BruteForceVectorBackend`): re-orders candidates by existing relevance score
  (desc) + `chunk_id` (asc) stable tie-break, drops/changes no candidate, annotates `reason` with
  provenance. It exists to make the **rerank-pipeline wiring** CI-verifiable with no model dependency.
- **`CrossEncoderReranker`** (`core/src/rerank/cross_encoder.rs`, `#[cfg(feature="reranker-fastembed")]`)
  — real cross-encoder via fastembed-rs `TextRerank` (BGE-reranker-base). Off by default (0 new crate
  — reuses the already-present optional `fastembed` dep; default build/CI never compiles it).
- **`Retriever::with_reranker(Arc<dyn Reranker>)`** seam — `search_semantic` / `search_hybrid`
  re-order their assembled top-k through it; `None` → both paths return their native order unchanged
  (backward compatible). For `search_hybrid`, the reranker applies **once** on the fused top-k (the
  vector component is fused raw, so fusion ranks reflect retrieval, not rerank).

## 2. Deterministic CI verification (tier a) — confirmed

Default build (`cargo test --workspace`, 0 model dep):

- `core/src/rerank/identity.rs::test_21_2_1_…` — `IdentityReranker` re-orders a fixed candidate set
  by score desc (chunk_id asc tie-break), drops no candidate, mutates no content, annotates `reason`,
  and is byte-identical across re-runs (determinism). **PASS**
- `core/src/retriever/mod.rs::test_21_2_2_…` — `with_reranker` seam: a wired `IdentityReranker`
  re-orders `search_semantic` + `search_hybrid` top-k (observable via the provenance marker); `None`
  reranker leaves the path unchanged (backward compat). Uses the default 0-dep `BruteForceVectorBackend`
  + `DeterministicEmbeddingProvider`. **PASS**

Deterministic results assert pipeline **wiring correctness + order determinism**, NOT real rerank
quality (ADR-013) — quality needs a real cross-encoder model.

## 3. Real cross-encoder (tiers b + c)

### Tier b — `CrossEncoderReranker` compiles against the real fastembed API — confirmed

`cargo check -p contextforge-core --features reranker-fastembed` → **PASS** (fastembed v4.9.1 + ort
v2.0.0-rc.9 compiled; 0 errors). The provider binds the real `TextRerank` /
`RerankInitOptions::new(RerankerModel::BGERerankerBase)` / `rerank(query, docs, false, None)` API and
maps `RerankResult { index, score }` back onto the candidates by index. Run platform: Windows MSVC,
2026-05-31 (cf. task-19.1 fastembed cross-platform credential).

### Tier c — real cross-encoder run (qualitative relevance) — confirmed

`cargo test -p contextforge-core --features reranker-fastembed test_21_2_3 -- --nocapture` →
**PASS** (1 passed; ran 393.67s = real BGE-reranker-base model download + ONNX inference). Run
platform: Windows MSVC, 2026-05-31.

On a real model run, `CrossEncoderReranker` correctly ranks by **joint** (query, doc) relevance: for
the query *"what does a panda eat?"* over three candidates `{a: "the giant panda is a bear species
endemic to china", b: "rust is a systems programming language", c: "pandas eat bamboo shoots and
leaves in the wild"}`, the bamboo-eating doc `c` outranks the unrelated systems-programming doc `b`,
the output is sorted by cross-encoder score descending, and every result carries the
`reranked:cross-encoder` provenance marker. No candidate dropped. This is a **real model run** (ADR-013
real-run tier) — not synthetic, deterministic, or fabricated.

### Tier c (quantitative) — recall / MRR uplift on the dogfood corpus — owned by task-21.3

The substantive **top-1 / MRR uplift numbers** (real cross-encoder rerank vs the dual-encoder
baseline over the dogfood eval corpus) are produced by the `task-21.3` closeout dogfood eval — the
same `[SPEC-OWNER:task-21.3-closeout-v0.14.0]` boundary the phase-21 spec sets, and the data that
ratifies ADR-026 Proposed→Accepted. task-21.2 lands the pipeline + confirms the real model reranks by
relevance (above); the uplift quantification is deliberately deferred to closeout
(`[SPEC-DEFER:phase-future.reranker-real-quality]` for any receiving-platform blocker, per ADR-026 D5
— not triggered here: the model built + ran on Windows MSVC).

## 4. Verdict

- Pipeline (trait + deterministic default + feature-gated real provider + retriever seam): **landed**.
- Deterministic CI verification (tier a): **PASS** (TEST-21.2.1 / TEST-21.2.2).
- Real cross-encoder compile (tier b): **PASS** (`reranker-fastembed`, fastembed v4.9.1).
- Real cross-encoder run, qualitative relevance (tier c): **PASS** (TEST-21.2.3, real BGE model).
- Real cross-encoder quantitative uplift (tier c numbers) + ADR-026 ratify: **owned by task-21.3**.
- ADR-026 stays **Proposed** until task-21.3 ratifies on real uplift data (ADR-013 — no premature
  Accept on qualitative-only evidence).
