# ADR `023`: `vector-backend-default`

**Status**: Accepted (2026-05-30; ratified in Phase 19 task-19.6 on task-19.5 real-embedding recall — see the **Amendment / Ratification** section below. Originally Proposed/provisional pending real-embedding recall.)
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

## Amendment / Ratification (2026-05-30, Phase 19 task-19.6)

> Add-only ratification. The Context / Decision (D1–D6) / Consequences above are **unchanged**; this
> section records the real-embedding recall that D6 deferred and flips the top **Status** to Accepted.

**Ratification basis — task-19.5 real-embedding SemanticRecall@K** (`docs/spikes/phase-19-real-recall.md`):
the real `FastEmbedProvider` (`all-MiniLM-L6-v2`, dim 384) was run over real ContextForge text (the 6
golden-question expected files + 5 distractor files → a 40-chunk balanced corpus, 30 built-in golden
queries) and indexed into an **exact-cosine** backend. Result: **SemanticRecall@5 = 0.8333,
SemanticRecall@10 = 0.9333** (top-1 = 0.60, MRR = 0.70) — clearing the ADR-006 Amendment A1 gate
(`SemanticRecall@10 ≥ 0.70`).

| corpus | recall@5 | recall@10 | discriminating? |
|---|---|---|---|
| Phase 18 synthetic seed vectors | 1.0 | 1.0 | **no** — non-discriminating (the D6 blocker) |
| Phase 19 real `all-MiniLM-L6-v2` (exact cosine) | 0.8333 | 0.9333 | **yes** — top-1 0.60, MRR 0.70, per-category 0.40–1.0 |

The recall was measured on **exact cosine**, so it is representative of any exact backend — including the
D1 provisional pick `sqlite-vec` — and is an upper bound for the ANN tiers (hnsw). It resolves the single
open dimension (D6): recall is discriminating on real distributions and the gate passes (ADR-013: real
`FastEmbedProvider` run, no synthetic / deterministic / fabricated figures).

**Status: Proposed → Accepted.** The tiered D1–D5 strategy is ratified. Note on the *implemented*
default: task-19.3 wired the semantic hot path with the **0-dependency `BruteForceVectorBackend`** (exact
cosine) as the default-available searcher — honoring **D5** (no vector dependency in the default build) —
with `sqlite-vec` (D1, embedded-persistence) / `hnsw` (D2, cross-platform) / `qdrant` (D3) / `lancedb`
(D4) remaining the feature-gated tiers. The recall ratification applies to the exact-cosine class
(brute-force + sqlite-vec alike); the D1–D4 tier ranking continues to rest on the Phase 18
latency / RSS / cold-start evidence, which this recall result does not disturb.

**Phase 18 §6 AC3 / AC4 resolved in Phase 19** (recorded here per ADR-014 D5 — the Phase 18 spec is
**not** retro-edited): AC3 (ADR-023 ratify, was partial/Proposed) is resolved by this ratification; AC4
(production vector-retrieval integration, was deferred) is resolved by task-19.2 (default backend wired
into `Retriever`) + task-19.3 (`/v1/search?semantic=true` → core gRPC semantic path) + task-19.5 (real
recall measured through that path).

## Amendment (Phase 23 / v0.16.0, 2026-05-31 — add-only, D1–D6 正文不溯改)

Phase 23（ADR-028 vector-persistence-strategy）以 add-only 方式推进本 ADR 的两个 Follow-up，**不溯改 D1–D6 / Consequences / Follow-ups 正文**（ADR-014 D5）：

- **D2 hnsw「in-memory-only / rebuild-on-restart」前提 → 解除**：task-23.1 让 `HnswBackend` 在 `vector-hnsw` feature 下支持 `save`/`load` 图持久化（路径 B：序列化 `(normalized embedding, chunk_id)` 输入集 + load 重建）+ rebuild-on-load fallback（`cargo test --features vector-hnsw` 3/3 PASS）。原 D2 把 hnsw 列为「rebuild on restart」的 disqualifying 项之一现已不成立——hnsw 可作中等语料持久部署 fallback（仍需重建图但消除冷启动 SQLite 枚举 + 重 embed 成本）。
- **D1「sqlite-vec Windows-MSVC-build-blocked / dev-prod parity imperfect」→ 缩小**：task-23.2 真实调查确证 sqlite-vec 0.1.9 在 `x86_64-pc-windows-msvc`（rustc 1.95.0）`cargo build --features vector-sqlite` + 契约测试**真实构建+运行通过**（解除 Phase 18 task-18.3 的 MSVC-blocked stop-condition；工具链演进，0 源码改动）。原 Consequences「the recommended default (sqlite-vec) does not build on the Windows dev box」在本机已不成立。**诚实 caveat**：单台 MSVC dev box 真实凭据，CI 默认不构建该 feature，跨 CI MSVC 持续守护属后续（`docs/spikes/phase-23-sqlite-vec-cross-platform.md`）。

依赖变更：task-23.1 路径 B（`serde`/`serde_json` 已 direct）+ task-23.2 路径 (a)（维持 `sqlite-vec = "=0.1.9"`）均 **0 新 dep** → 无 ADR-008 依赖变更 Amendment。详见 ADR-028 + `docs/releases/v0.16.0-evidence.md`。

## Amendment (Phase 25 / v0.18.0, 2026-06-01 — add-only, D1–D6 正文不溯改)

Phase 25（ADR-030 production-vector-backend）以 add-only 方式推进本 ADR 的两个生产规模 tier（D3 qdrant / D4 lancedb），**不溯改 D1–D6 / Consequences / Follow-ups 正文**（ADR-014 D5）：

- **D3「Hosted / scale-out: qdrant」tier → 生命周期契约层推进**：task-25.1 让 `QdrantBackend` 在 `vector-qdrant` feature 下从 spike 级「`open()` 无脑 drop+create」推进到有 `QdrantConnConfig`（url/timeout/api_key/tls + `validate`）+ `health()` probe + `decide_ensure` collection ensure-create（reuse-if-matching / create / error-on-mismatch）的生命周期契约层（`cargo test --features vector-qdrant retriever::vector::qdrant` 4/4 PASS，不连 live server）。D3 把 qdrant 列为 hosted/scale-out 档的定位不变；本推进是其生产化生命周期层。**诚实 caveat**：真实 KNN over live qdrant server 因 CI 无在跑的 server 诚实延后 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（契约层 shape/config/decide_ensure 真实单测可断言；live-server 集成不伪造，ADR-013）。
- **D4「Embedded-columnar alternative: lancedb」tier → 🟢 可构建性确证（protoc 前置缩小）**：task-25.2 真实调查确证 lancedb 0.30 在 `x86_64-pc-windows-msvc`（rustc 1.95.0）`cargo build --features vector-lancedb` **真实 exit 0**（protoc 经仓内 build-dep `protoc-bin-vendored` 的 `protoc.exe` 经 `PROTOC` env 满足，无需系统安装；0 源码/Cargo 改动，0 新依赖）+ 索引调参参数校验（`LanceIndexTuning::validate`）+ 既有 backend 契约 `--lib` 2/2 PASS。D4 记录的「the heaviest build（Lance/DataFusion + a protoc prerequisite）」前置在本机以 vendored protoc **可满足**——protoc-prereq 担忧由此**缩小**（可满足、不需系统安装）而**非消除**（仍须显式提供 `PROTOC`）。**诚实 caveat**：单台 MSVC dev box 真实凭据，CI 默认不构建该 feature；广义 feature 全 target 测试受 rustc 1.95.0 ICE 限制（工具链项）；真实 ANN 索引性能 `[SPEC-DEFER:phase-future.lancedb-index-tuning]` 延后（`docs/spikes/phase-25-lancedb-buildability.md`）。

依赖变更：task-25.1（qdrant 生命周期层复用 `qdrant-client` 1.18 既有 API）+ task-25.2（索引调参复用 lancedb 0.30 既有面）均 **0 新 dep**（qdrant-client/lancedb/arrow-array/futures 自 task-18.4/18.5 即 optional，`core/Cargo.toml`/`Cargo.lock` 未改）→ 无 ADR-008 依赖变更 Amendment。详见 ADR-030 + `docs/releases/v0.18.0-evidence.md`。

## Amendment (Phase 29 / v0.22.0, 2026-06-03 — add-only, D1–D6 正文不溯改)

Phase 29（ADR-034 live-vector-recall）以**真实跨 backend 测量**校准 tier 定位，**不溯改 D1–D6 / Consequences 正文**（ADR-014 D5）：

- **D5「默认 0-dep BruteForce baseline」→ 实测背书**：task-29.3 同语料矩阵（n=1024, dim=384）实测 brute-force exact recall@10=1.0 + query ~5.5 ms/q，**既最准又最快于 lancedb IVF_PQ（~0.44, ~51 ms）/ 持平 lancedb flat（~7.6 ms）/ 仅略逊 IVF_HNSW_SQ 速度（~3.5 ms）但召回更高**。即 modest 语料下默认 0-dep BruteForce 不仅是「无 dep 的折中」而是**实测最优**——重型 ANN 的索引/查询开销只在大语料才回本。D5 默认档定位由真实测量背书。
- **D4「lancedb embedded-columnar」tier → 真实 ANN 召回/延迟补全**：task-29.3 经 `create_ann_index` 真实建 IVF_PQ / IVF_HNSW_SQ 索引实测——lancedb 档内 **IVF_HNSW_SQ（recall@10 ~0.90, build ~0.25 s, query ~3.5 ms）为首选**，IVF_PQ（recall@10 ~0.44, build ~2.8 s）是重压缩档、modest n 下劣。承 Phase 25 Amendment 的「可构建性确证」，本 phase 续补「真实召回/延迟」。
- **D3「hosted qdrant」tier → 仍 honest-defer**：task-29.2 qdrant live KNN 无 server → honest-defer（`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`），矩阵 qdrant 档真实召回待手动/dev-box server 回填，不伪造（ADR-013）。

依赖变更：task-29.3 复用 lancedb 0.30 既有面 → **0 新 dep** → 无 ADR-008 Amendment。详见 ADR-030 Amendment (Phase 29) + `docs/releases/v0.22.0-evidence.md`。
