# ADR `023`: `vector-backend-default`

**Status**: Proposed (2026-05-30; data-driven recommendation from the Phase 18 spikes — **provisional pending task-18.8 real-embedding recall**; to be ratified at Phase 18 closeout)
**Category**: 数据平面 / 向量检索 / backend 选型
**Date**: 2026-05-30
**Decided By**: 主 agent (ADR-012 自治) on the task-18.3–18.6 5-dimension evidence; tajiaoyezi ratification at closeout
**Related**: ADR-001 (dual-binary) / ADR-002 (sqlite+tantivy persistence) / ADR-008 (core-library-selection) / ADR-014 (D1-D5) / Phase 18 (vector-backend-selection) / task-18.1 (vector trait freeze) / task-18.2 (spike harness) / task-18.3–18.6 (4 backend spikes) / task-18.8 (eval SemanticRecall@K)

## Context

Phase 18 evaluates four embedded/external vector backends behind the task-18.1 `Vector{Backend,
Indexer,Searcher}` traits, to choose a default for ContextForge's semantic retrieval (augmenting the
ADR-002 BM25/Tantivy path). All four were implemented and measured on the **same Linux x86_64 host**
by the task-18.2 harness at n=5000 and n=100000 — full data in `docs/spikes/phase-18-comparison.md`
and the per-backend `docs/spikes/phase-18-<backend>.md` files.

Key facts from the evidence (n=100000, dim=64):

| backend | model | recall@5/10 | P95 (ms) | index RSS (MB) | cold-start (ms) | platform |
|---|---|---|---|---|---|---|
| sqlite-vec | embedded + disk, exact | 1.0 / 1.0 | 3.198 | 90.7 | 760 | Linux/gcc (**Windows MSVC blocked**) |
| hnsw | in-mem ANN, pure Rust | 1.0 / 1.0 | 0.871 | 180.0 | **28432** | **all incl. Windows MSVC** |
| qdrant | external server, ANN | 1.0 / 1.0 | 0.947 | 91.6 (+~166 server) | 385 | external server |
| lancedb | embedded + disk, flat | 1.0 / 1.0 | 10.893 | 90.8 | 50.4 | Linux (+protoc build) |

Decisive observations:

- **recall is non-discriminating on the synthetic corpus** — all four hold recall@5/10 = 1.0 even at
  100k. The selection therefore **cannot** be made on recall from this data; the real ranking needs
  dogfood-distribution embeddings, which is **task-18.8 (eval SemanticRecall@K)**.
- **ContextForge is local-first, single-binary, SQLite-based (ADR-002), cross-platform** (dev on
  Windows MSVC, release on Linux/containers). These constraints — not recall — drive the choice.
- No single backend dominates: sqlite-vec is the lightest + best-aligned with ADR-002 but is
  Windows-MSVC-build-blocked; hnsw builds everywhere with zero native deps but has a 28 s graph build
  at 100k, no persistence, and the heaviest memory; qdrant needs an external server; lancedb has the
  fastest writes + durability but the heaviest build (protoc) and highest query latency.

## Decision

A **tiered, feature-gated** backend strategy — no backend compiled by default — with a recommended
embedded default and explicit per-deployment alternatives. **Provisional**: the embedded-default pick
(D1) is ratified only after task-18.8 confirms recall on real embeddings.

### D1 — Recommended embedded default (production / Linux): `sqlite-vec` — PROVISIONAL

For ContextForge's production target (Linux containers, per release.yml + the docker-compose
deployment), **sqlite-vec** is the recommended default: it is the most architecturally coherent with
ADR-002 (the vector index lives in the **same on-disk SQLite store** as the rest of the data plane —
no separate format, no rebuild-on-restart, transactional consistency), the lightest footprint
(90 MB at 100k), and exact recall. Its exact O(n) query latency (3.2 ms at 100k) is well within the
PRD P95 < 500 ms. This pick is **provisional pending task-18.8** real-embedding recall.

### D2 — Cross-platform / dev / small-corpus fallback: `hnsw`

**hnsw** (instant-distance, pure Rust, 0 native deps) is the only backend that builds on the Windows
MSVC dev box and everywhere else. It is the fallback for development, cross-platform builds, and small
corpora where the graph-build cost is low. It is **not** the production default at scale: its 28 s
build at 100k, in-memory-only model (rebuild on restart), and 180 MB footprint are disqualifying for
large persisted indexes until a graph-persistence layer exists
(`[SPEC-DEFER:phase-future.hnsw-graph-persistence]`).

### D3 — Hosted / scale-out: `qdrant`

**qdrant** is reserved for hosted / multi-agent / horizontal-scale deployments where an external
vector database is acceptable. It has the best ANN throughput and server-managed durability,
replication, and filtering — at the cost of breaking the single-binary model (external server,
+166 MB).

### D4 — Embedded-columnar alternative: `lancedb`

**lancedb** is the alternative when fast bulk ingest (50 ms writes) + on-disk columnar durability +
SQL/metadata filtering matter more than per-query latency. It carries the heaviest build (Lance/
DataFusion + a `protoc` prerequisite).

### D5 — Default build is BM25-only; all backends feature-gated

The default build ships **no** vector backend — `NoopVectorBackend` keeps the hot path BM25-only
(task-18.1). Each backend is gated behind its `vector-<backend>` feature, so the default build (incl.
the Windows dev box and CI) pulls **zero** new dependencies. Selecting a backend is a build/deploy-time
feature choice, not a default.

### D6 — Ratification + runtime wiring deferred

This ADR is **Proposed**. Final ratification (and any promotion of D1 to an `Accepted` default)
follows **task-18.8** (real-embedding SemanticRecall@K on the dogfood corpus), at the Phase 18
closeout. Production runtime wiring of the chosen backend into the `Retriever` hot path requires an
embedding pipeline (not yet in the project) and is **out of scope** here
(`[SPEC-OWNER:phase-future.vector-retrieval-integration]`) — task-18.1 already provides the
`Retriever::with_vector_searcher(Arc<dyn VectorSearcher>)` seam for that future integration.

## Consequences

- **Positive**: the decision is grounded in real 5-dimension data across four backends on one host;
  the trait abstraction (task-18.1) means any tier can be wired without touching the retrieval core;
  the default build stays dependency-free and cross-platform.
- **Negative / open**: the recall dimension is unresolved on synthetic data — D1 is provisional until
  task-18.8. The recommended default (sqlite-vec) does not build on the Windows dev box, so dev/prod
  backend parity is imperfect (mitigated by hnsw for local dev + vector search being opt-in).
- **Follow-ups**: task-18.8 (real-embedding recall → ratify D1); hnsw graph persistence
  `[SPEC-DEFER:phase-future.hnsw-graph-persistence]`; sqlite-vec Windows MSVC port
  `[SPEC-DEFER:phase-future.sqlite-vec-cross-platform]`; production embedding + retrieval wiring
  `[SPEC-OWNER:phase-future.vector-retrieval-integration]`.
