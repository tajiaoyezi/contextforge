# ADR `034`: `production-vector-live-recall`

**Status**: Accepted（v0.22.0 / task-29.4 ratify；D2 qdrant live-server 维度 honest-defer 部分 ratify）
**Category**: 检索 / 向量 backend / 召回质量
**Date**: 2026-06-02
**Decided By**: 主 agent（ADR-012 自治）；tajiaoyezi ratification at v0.22.0 closeout
**Related**: ADR-030 (production-vector-backend — 本 phase 在其 qdrant 生命周期契约层 / lancedb 索引调参参数之上做真实 live KNN / ANN，选择矩阵以 add-only Amendment 校准，不溯改 D1-D4 正文，ADR-014 D5) / ADR-023 (vector-backend-default — D1-D6 tier 经真实测量以 add-only Amendment 校准) / ADR-028 (vector-persistence — 持久化 seam 复用) / ADR-027 (embedding-provider-selection — 仿 `select_provider` 工厂 pattern) / ADR-004 (local-first-privacy-baseline — 默认构建仍 0 vector dep + BruteForce baseline) / ADR-013 (禁伪造红线 — live-server / 大语料召回数真实跑出后回填，不伪造) / ADR-012 (main-agent-governance-autonomy — tag/release outward-facing 须用户显式授权) / ADR-014 (D1-D5，第二十次激活) / roadmap §3.11

## Context

ContextForge 截至 Phase 25（production-vector-backend, Done）已把生产向量 backend 推进到**契约层 / 参数层**：qdrant 有 connect/health/`decide_ensure`/ensure-create 的生命周期契约层（`core/src/retriever/vector/qdrant.rs:152-270`，真实单测不连 server），lancedb 有可构建性结论 + `LanceIndexTuning`/`LanceAnnIndex` 索引调参参数校验层（`core/src/retriever/vector/lance_db.rs:33-108`，参数校验真实单测不建真实索引）。但三处真实「跑通」仍缺：

- **server.rs 热路径仍硬编码 BruteForce**：语义路径（`core/src/server.rs:341`）与 hybrid 路径（`core/src/server.rs:302`）均直接 `BruteForceVectorBackend::new()`，无工厂注入——`select_provider`（embedding）已工厂化（`core/src/embedding/factory.rs:27-30` + `server.rs:339`），向量侧对称的 `select_vector_backend` **缺**。真实 backend（qdrant/lancedb）即便 feature 编译通过也进不了生产热路径，`[SPEC-DEFER:phase-future.vector-retrieval-integration]`（phase-25 spec line 44）未兑现。
- **qdrant live KNN 从未真实跑过**：`qdrant.rs:330-371` 的 live search 读路径在契约层之上，但 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` 的端到端 connect→ensure-create→upsert→KNN over **live server** 仍属诚实延后（CI 无在跑的 qdrant server，ADR-030 D1）。
- **lancedb 真实 ANN 索引 + 召回从未实测**：`lance_db.rs:270-332` live search 在参数层之上，但 `[SPEC-DEFER:phase-future.lancedb-index-tuning]` 的真实 IVF_PQ/HNSW 建图 + 召回测量未做（ADR-030 D2 ratify 时如实延后）。
- **生产 backend 选择矩阵尚无真实测量校准**：ADR-030 D3（`adr-030:42-44` matrix + `:57-66` Ratification）与 ADR-023 D1-D6 tier 当前据 tier 推理 + 可构建性结论给出，未据**真实跨 backend 召回/延迟测量**校准。

本 ADR 记录把上述延后的契约层 / 参数层**真实跑通为 live 向量召回**、并把真实 backend 接入生产热路径的策略。全程守 ADR-013：live-server / 大语料的真实召回 / 延迟数字一律**真实跑出后回填**到 §10 + v0.22.0 evidence，不在 spec / ADR 预填。默认构建仍 0 vector dep + BruteForce 语义 baseline（ADR-004 / ADR-023 D5），feature-gated backend 默认不编译不引入供应链面。

## Decision

生产向量 live 召回采用 **工厂化热路径注入 + qdrant live KNN 真实兑现（无 server 诚实延后）+ lancedb 真实 ANN 索引调参 + 真实测量校准选择矩阵（add-only Amendment）+ 默认构建零依赖守线** 策略：

### D1 — vector backend factory + server.rs 热路径注入（task-29.1）

新增 `select_vector_backend(name, dim) -> Result<Arc<dyn VectorSearcher>, VectorError>` 工厂，对称仿 `core/src/embedding/factory.rs::select_provider`（`factory.rs:27-30`）：`""`/`"brute"` → `BruteForceVectorBackend`（始终可用，默认 0-dep）；`"qdrant"` → `QdrantBackend`（`vector-qdrant` feature 下，否则返回可识别 `Err`，不静默成功）；`"lancedb"` → `LanceDbBackend`（`vector-lancedb` feature 下，否则可识别 `Err`）。把它接入 `core/src/server.rs` 替换 `server.rs:302`（hybrid 路径）+ `server.rs:341`（语义路径）的硬编码 `BruteForceVectorBackend::new()`。默认构建（无 vector feature）语义 + hybrid 仍经 BruteForce 工作，`cargo test --workspace` 不受影响。

**理由**：消除 `server.rs:302/341` 硬编码 BruteForce，把真实 backend 经工厂注入生产热路径，兑现 `[SPEC-DEFER:phase-future.vector-retrieval-integration]`（phase-25 spec line 44）。仿 `select_provider` 工厂 pattern 与 embedding 侧对称、最 surgical（不改 `VectorSearcher` trait 签名 `traits.rs:38-46`）；默认仍 BruteForce 守 ADR-004 0-dep。备选「server.rs 直接 `#[cfg]` 分支」会把 feature-gate 逻辑散在热路径、与 embedding 侧不对称且难单测，故不取——工厂集中决策、可 deterministic 单测（无 feature 默认分支 / 缺 feature 诚实 Err）。

### D2 — qdrant live-server 端到端 KNN + 真实召回 harness；无 server 诚实延后（task-29.2）

首次真实兑现 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`：克隆 `core/examples/phase20_recall_via_retriever.rs` 为 phase29 harness，把 BruteForce 换为 `QdrantBackend::connect(QdrantConnConfig::from_env())`，guard 在 `vector-qdrant` + `embedding-fastembed` feature 下，对一台 **live qdrant server** 跑 connect→ensure-create→upsert→KNN（live 读路径 `qdrant.rs:330-371`）。CI 无 server → 当 `backend.health() == Unreachable`（`qdrant.rs:184-189`）时诚实延后（`eprintln` + `exit 0`，不伪造 KNN 通过，ADR-013）。文档化单机部署 baseline；集群/复制拓扑 → `[SPEC-DEFER:phase-future.qdrant-deployment-topology]`。

**理由**：这是 qdrant 契约层（Phase 25 已真实单测 `decide_ensure`/health/ensure-create）之上**首次真实 live KNN**——契约正确不等于 live 跑通，需对真实 server 兑现。CI 无 server 是结构性约束（ADR-030 D1 已识别），honest-defer（health 探活 → 可达才跑，不可达干净退出）既证明 wiring 又不伪造召回，真实召回数 manual/dev-box 跑出后回填 §10 + v0.22.0 evidence。备选「CI spin up qdrant container」超出本 phase 范围（引入 CI service 编排面），单机 baseline 先兑现、拓扑延后是诚实的渐进。

### D3 — lancedb 真实 ANN 索引调参建图 + 性能（task-29.3）

用 `LanceIndexTuning`/`LanceAnnIndex` 参数契约（`lance_db.rs:33-108`）在一个嵌入式 Lance 数据集上真实建 IVF_PQ/HNSW 索引并测量召回（`vector-lancedb` feature，in-process，n 仍 modest），兑现 `[SPEC-DEFER:phase-future.lancedb-index-tuning]`。lancedb feature 构建 caveat：broad `cargo test` 触 rustc ICE → 用 `cargo build` + `--lib` scoped 测试 `[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]`（承 ADR-030 D2 真实凭据 + Phase 23 sqlite-vec MSVC 先例）。lancedb compaction 执行 → `[SPEC-DEFER:phase-future.lancedb-schema-compaction]`（很可能诚实延后）。

**理由**：在参数契约层（Phase 25 真实单测 `validate`）之上真实建 IVF_PQ/HNSW 索引并实测召回，是「参数可校验」到「索引真能召回」的兑现。大语料性能受 toolchain（rustc 1.95.0 ICE on broad test target，ADR-030 D2 已记）+ 资源限，故 n 取 modest、broad test → scoped `--lib`，受阻维度（compaction / 大语料 / CI 构建）如实记录不伪造。备选「合成召回数充门」违 ADR-013，故真实跑出后回填。

### D4 — 多 backend 选择矩阵真实测量 → ADR-030 D3 / ADR-023 tier 的 add-only Amendment（task-29.3）

产出一份**真实**多 backend 选择矩阵测量（brute / sqlite-vec / lancedb / qdrant，可跑的真测、不可跑的诚实延后），用真实召回/延迟数据校准选择矩阵，feed 给 ADR-030 D3（`adr-030:42-44`）+ ADR-023 tier（`adr-023:44-82` D1-D4）的 **add-only Amendment**（不编辑其 D 正文，ADR-014 D5）。

**理由**：ADR-030 D3 / ADR-023 tier 当前据 tier 推理 + 可构建性结论给出，用真实跨 backend 测量校准提升可信度。add-only Amendment 守 ADR-014 D5——既往 ratify 过的 D 正文不溯改，只追加「Phase 29 真实测量校准」结果。某 backend 不可跑（qdrant 无 server / lancedb 平台受阻）则该格如实延后，矩阵据已达格校准，不伪造全格。

### D5 — 默认构建不变：0 vector dep + BruteForce 语义 baseline（all tasks）

`select_vector_backend` 默认分支 + 缺 feature Err 路径不引入任何 vector 依赖；`QdrantBackend`/`LanceDbBackend` 仍各自 `vector-qdrant`（`core/Cargo.toml:119`）/ `vector-lancedb`（`Cargo.toml:120`）feature 下编译；默认构建语义 + hybrid 路径仍经 0-dep `BruteForceVectorBackend`（ADR-023 D5）。harness 在 `vector-qdrant` + `embedding-fastembed`（`Cargo.toml:123`）/ `vector-lancedb` feature 下，默认 `cargo test --workspace` 不编译它们。本 ADR 不改 `VectorBackend`/`VectorIndexer`/`VectorSearcher` 三 trait 签名（`traits.rs:11-46`）。

**理由**：ADR-004 local-first——默认构建 0-network / 0 新依赖 / 0 供应链面是不可让渡的 baseline。feature-gated backend 是「按需启用的生产能力」而非「默认引入的成本」，与 D1-D4 的真实兑现正交：工厂注入 + live KNN + 真实索引都在 feature 边界内，默认路径与 Phase 19 语义 baseline 字节等价。

## Consequences

- **Positive**: server.rs 热路径从硬编码 BruteForce 推进到工厂注入真实 backend（兑现 `[SPEC-DEFER:phase-future.vector-retrieval-integration]`）；qdrant 契约层首次真实 live KNN 兑现（有 server 真跑、无 server 诚实退出）；lancedb 参数层首次真实 ANN 索引建图 + 召回测量；生产 backend 选择矩阵据真实测量校准（add-only，不溯改 ADR-030/023 正文）；默认构建 0 vector dep + BruteForce 语义 baseline 不变（ADR-004 / ADR-023 D5），`cargo test --workspace` 不退化，三 trait 签名不变。
- **Negative / open**（受阻维度如实，不伪造）：qdrant live KNN 依赖 live server，CI 无 server → D2 集成维度诚实延后（health Unreachable 时 `exit 0`，真实召回 manual/dev-box 跑出后回填，绝不预填）；lancedb feature 构建受 rustc 1.95.0 broad-test ICE 限（→ `cargo build` + `--lib` scoped，CI 默认不构建该 feature `[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]`），大语料性能 + compaction 受 toolchain/资源限如实延后；选择矩阵不可跑格（qdrant 无 server / lancedb 平台受阻）据已达格校准，不伪造全格；qdrant 集群/复制拓扑超本 phase 范围延后。
- **Ratification**: 本 ADR Proposed。task-29.1..29.3 通过后于 v0.22.0 closeout 据真实 CI / 实测产物 ratify；live-server / 大语料受阻维度据已达维度 ratify + 如实记录，不强 ratify。
- **Follow-ups**: qdrant 集群/复制部署拓扑 `[SPEC-DEFER:phase-future.qdrant-deployment-topology]`；lancedb feature 在 CI 真实构建（toolchain ICE 解除后）`[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]`；CI 内置 qdrant service 跑 live KNN 回归 `[SPEC-DEFER:phase-future.qdrant-ci-service]`；ADR-030 D3 选择矩阵 + ADR-023 D1-D6 tier 经真实测量以 add-only Amendment 记录（task-29.3 已落地，不溯改其正文，ADR-014 D5）。注：`[SPEC-DEFER:phase-future.lancedb-schema-compaction]` 已由 task-29.3 真实 `compact()` 兑现，不再延后。

## Ratification (v0.22.0 / task-29.4)

本 ADR 于 v0.22.0 closeout（task-29.4）据 task-29.1/29.2/29.3 的**真实非合成验证** ratify **Proposed → Accepted**（ADR-013：禁据合成/伪造 ratify）。逐 D 真实依据：

- **D1（vector backend factory + server.rs 热路径注入）→ ✅ Accepted**：task-29.1（PR #197）落 `select_vector_backend(name, dim) -> Result<Arc<dyn VectorStore>, VectorError>`（`core/src/retriever/vector/factory.rs`）。**实现精化**：Decision D1 原拟返 `Arc<dyn VectorSearcher>`，但 `server.rs` 热路径用同一 backend 对象既 `index_chunks_semantic(&dyn VectorIndexer)` 又 `with_vector_searcher(Arc<dyn VectorSearcher>)`，故加 add-only 组合 trait `VectorStore: VectorIndexer + VectorSearcher`（IS-A VectorSearcher，契约真超集；三 base trait 签名零改动，ADR-014 D5），调用点经 rustc 1.86+ trait-upcasting 自动 upcast。`server.rs:302`（hybrid）/ `:341`（semantic）经工厂注入（默认 `""` byte-equivalent BruteForce）。`cargo test -p contextforge-core --lib retriever::vector::factory` **4 passed**（TEST-29.1.1 默认臂 BruteForce / 29.1.2 feature 关闭诚实 Err + 未知名 Err）+ `cargo test --workspace` **191 lib + 全集成 0 failed**（默认 semantic+hybrid 经工厂走 BruteForce 不退化）。兑现 `[SPEC-DEFER:phase-future.vector-retrieval-integration]`（phase-25 line 44）。
- **D2（qdrant live-server 端到端 KNN + honest-defer）→ 🟡 PARTIAL（wiring + honest-defer Accepted；live-recall 维度 honest-defer）**：task-29.2（PR #198）`core/examples/phase29_recall_via_qdrant.rs`（双 gate `vector-qdrant`+`embedding-fastembed`）经 `QdrantBackend::connect(QdrantConnConfig::from_env())` + `health()` 守门。`cargo build --features vector-qdrant,embedding-fastembed` exit 0；`cargo run`（CI / dev box 无 server）→ **实测 `health=Unreachable ... Exiting 0.` EXIT=0，零召回数输出**——connect/health/ensure-create/upsert/KNN wiring 成立而**不伪造召回**（ADR-013）。**真实 live KNN 召回数（recall@5/@10 + top-1 + MRR over real server）无 live qdrant server → honest-defer，未跑出、不预填**，待手动 / dev-box 单节点 qdrant 跑出后回填 v0.22.0-evidence §7。单节点部署基线文档化，集群/复制 `[SPEC-DEFER:phase-future.qdrant-deployment-topology]`。
- **D3（lancedb 真实 ANN 索引调参建图 + 性能）→ ✅ Accepted**：task-29.3（PR #199）`LanceDbBackend::create_ann_index`（Lance `create_index` 真实建 `Index::IvfPq` / `Index::IvfHnswSq`）+ `compact()`（`OptimizeAction::All`）。`cargo test --features vector-lancedb --lib retriever::vector::lance_db` **4 passed**（`--lib` scoped 规避 broad-test rustc 1.95.0 ICE）。实测（n=1024 dim=384 clustered，单次代表值）：IVF_PQ recall@10≈0.41–0.46 build~2.8 s query~51 ms；IVF_HNSW_SQ recall@10≈0.90 build~0.25 s query~3.5 ms。**真实 compaction 兑现**：1536 行 6 fragment → count_rows=1536 不丢（兑现 `[SPEC-DEFER:phase-future.lancedb-index-tuning]` + `[SPEC-DEFER:phase-future.lancedb-schema-compaction]`）。大语料拐点 `[SPEC-DEFER:phase-future.vector-large-corpus-perf]` 延后。
- **D4（多 backend 选择矩阵真实测量 → ADR-030/023 add-only Amendment）→ ✅ Accepted**：task-29.3 同语料真实矩阵——brute-force exact（recall 1.0, ~5.5 ms）/ lancedb flat（1.0, ~7.6 ms）/ IVF_PQ / IVF_HNSW_SQ（上）实测；qdrant honest-defer（无 server）/ sqlite-vec 本 pass 未跑 in-process 测量。已写入 ADR-030 `## Amendment (Phase 29 / v0.22.0)` D3 矩阵 + ADR-023 `## Amendment (Phase 29 / v0.22.0)` tier（add-only，不溯改 D 正文，ADR-014 D5）。**真实结论**：modest n 下 brute-force exact 既最准又最快，实测背书 ADR-023 D5 默认 0-dep 档。
- **D5（默认构建不变：0 vector dep + BruteForce 语义 baseline）→ ✅ Accepted**：默认 `cargo test --workspace` 0 failed + `cargo clippy --workspace --all-targets -- -D warnings` 0 warning；`core/Cargo.toml` / `Cargo.lock` 未改（0 新 direct dep）；qdrant/lancedb 真实索引全在各自 feature 下，默认 semantic+hybrid 经 0-dep BruteForce；三 base trait 签名不变（`VectorStore` 是 add-only 组合 trait）。

ratify 范围 = vector backend 工厂 + server.rs 热路径注入（D1）+ qdrant live KNN wiring/honest-defer（D2 partial）+ lancedb 真实 IVF_PQ/IVF_HNSW_SQ 索引 + compaction + 召回（D3）+ 多 backend 选择矩阵真实测量（D4）+ 默认 0-dep baseline（D5）。**qdrant live-server 真实召回数据据「已达维度 ratify + 受阻维度如实记录」honest-defer，不伪造**（ADR-013）。证据见 `docs/releases/v0.22.0-evidence.md` §3。

## Amendment (Phase 32 / v0.25.0)

add-only 校准，不编辑 D1-D5 正文 / 不溯改上方 `## Ratification (v0.22.0 / task-29.4)` 及任何 Phase 29 校准（ADR-014 D5）。

D4 的多 backend 选择矩阵在 v0.22.0 ratify 时，sqlite-vec 格记为「本 pass 未跑 in-process 测量」（`## Ratification` D4）。Phase 32（task-32.2，PR #213，squash `76a3137`）为 `select_vector_backend` 工厂补上 `"sqlite-vec"` 臂（`vector-sqlite` feature double-half cfg gating，仿 qdrant/lancedb：feature on → `SqliteVecBackend::new()`，`name()="sqlite-vec"`；feature off → 可识别 honest `Err`，点名 sqlite-vec + vector-sqlite，不静默成功，ADR-013）。故 sqlite-vec 现可经 `select_vector_backend` 选择 + 由 task-32.1 完成 config-plumbing（`CONTEXTFORGE_VECTOR_BACKEND`），D4 矩阵此前未测的 sqlite-vec 槽位现已 **factory-selectable + config-plumbed**，工厂 backend 覆盖补齐为 brute / qdrant / lancedb / sqlite-vec。

工厂臂 wiring 经真实验证（非仅结构）：默认构建 TEST-32.2.1 feature-off honest-Err + TEST-32.2.2 selection-matrix wiring → factory 6/6；feature-on 经 **真实 x86_64-pc-windows-msvc `cargo test --features vector-sqlite`** 构建通过 → `sqlite_vec_with_feature_returns_sqlite_vec_backend` + 矩阵 feature-on 分支成立。0 新依赖（sqlite-vec 在 `Cargo.toml` 已是 optional）。

sqlite-vec 的 **in-process recall/latency 矩阵 CELL（真实 KNN recall@k + 延迟 over 语料）仍 honest-deferred** `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`——需本机 MSVC feature 构建 + 真实语料跑出，绝不预填伪造数（ADR-013）。证据见 `docs/releases/v0.25.0-evidence.md`。

## Amendment (Phase 36 / v0.29.0)

add-only 校准，不编辑 D1-D5 正文 / 不溯改 `## Ratification (v0.22.0 / task-29.4)` 及 `## Amendment (Phase 32 / v0.25.0)`（ADR-014 D5）。

D2（qdrant live KNN）在 v0.22.0 ratify 时是 🟡 PARTIAL：connect/health/ensure-create/upsert/KNN wiring 成立、但**真实 live-server 召回数因 CI 无在跑 qdrant server 一路 honest-defer** `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`。Phase 36（ADR-041 qdrant-live-vector-recall）**兑现并永久关闭**该维度：

- **task-36.1**（PR #236）新增 env-gated live recall harness `core/tests/qdrant_live_recall.rs`（qdrant HNSW ANN recall@k vs BruteForce exact KNN，确定性可复现语料 N=2000 dim=64，`health()!=Ready` honest-defer skip 不 fail）。
- **task-36.2**（PR #237）新增 `qdrant-recall` CI job（qdrant **service container** + Wait-for-ready + harness），令 recall 每次 CI run 对 live qdrant 验证。
- **真实实测（CI run 26961084355）**：`recall@10=1.0000`（N=2000 dim=64 M=50，qdrant LIVE KNN == brute-force exact ground truth），取代此前唯一的合成 fixture（`eval_integration.rs:110` 的 0.7/0.85）。诚实判读（ADR-013）：recall=1.0 因 qdrant 在 N 低于其 HNSW indexing_threshold 时服务**精确** KNN → 这是 live KNN **正确性**真实证明；HNSW 近似域大语料真实 ANN recall 续 honest-defer `[SPEC-DEFER:phase-future.vector-large-corpus-perf]`。

故 D2 的 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` **fulfilled**（live KNN recall measured + CI-guarded，CI now HAS a live server）。**不溯改 D2 D-body 正文 / Ratification (v0.22.0)**（ADR-014 D5）。证据见 `docs/releases/v0.29.0-evidence.md` + ADR-041 §Ratification。
