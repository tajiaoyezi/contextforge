# Phase 29 · live-vector-recall

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。本 phase 承 Phase 25（production-vector-backend，Done）的 qdrant / lancedb 契约层 + 参数校验层成果，把一路 `[SPEC-DEFER]` 延后的「契约 / 参数」层兑现为**真实 live 向量召回**，并把真实 backend 工厂化注入生产热路径 `core/src/server.rs`（现 :302 hybrid / :341 semantic 仍硬编码 `BruteForceVectorBackend`）。本 phase 的两层诚实分界明确：工厂化注入 + 默认 BruteForce 语义热路径 + lancedb 参数→真实 IVF_PQ/HNSW 索引构建为 🟢 deterministic / 🟡 feature build 可在受控环境验证；qdrant live-server 端到端 KNN（connect→ensure-create→upsert→search）依赖外部真实 server，CI 无 server → **🔴 诚实延后**（health Unreachable 时 honest-defer + exit 0，绝不伪造召回数字，ADR-013）。默认构建仍 0 vector 依赖、BruteForce 语义基线不变（ADR-004）。v0.22.0 收口。对应 `docs/roadmap.md §3.11`。
>
> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md §3.11`（live 向量召回候选 + 生产 backend 热路径注入 + qdrant/lancedb 真实兑现路线）→ `core/src/server.rs`（热路径：`:302` hybrid + `:341` semantic 硬编码 `BruteForceVectorBackend`、`:339` `select_provider` 工厂调用样板）→ `core/src/retriever/vector/`（`traits.rs:38-46` `VectorSearcher::search` live-KNN surface / `qdrant.rs` connect·health·ensure-create·live-search / `lance_db.rs` `LanceIndexTuning`·`LanceAnnIndex`·validate·live-search / `mod.rs:592-595` `with_vector_searcher`、`:628-665` `index_chunks_semantic`、`:684-713` `search_semantic_raw`）→ AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5，第二十次激活）→ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造红线：真实 live KNN / 真实索引召回 / 真实选择矩阵测量，受阻如实记录不伪造）→ `docs/decisions/adr-030-production-vector-backend.md`（选择矩阵 D3 / Ratification）→ `docs/decisions/adr-023-vector-backend-default.md`（默认 backend tier）→ `docs/decisions/adr-004-local-first-privacy-baseline.md`（默认构建 0 vector dep baseline 不变）。
>
> **ADR 影响面（已识别）**：
> - **ADR-034 production-vector-live-recall（新，Proposed）**：记 vector backend 工厂 + server.rs 热路径注入（D1）+ qdrant live-server 端到端 KNN + 真实召回 harness（无 server 诚实延后，D2）+ lancedb 真实 ANN 索引构建 + 实测召回（D3）+ 多 backend 选择矩阵真实测量 → ADR-030/023 add-only Amendment（D4）+ 默认构建 0 vector dep + BruteForce 语义基线不变（D5）。落地后据真实 live KNN / 真实索引召回 / 真实矩阵测量产物 ratify；live-server / 大语料受阻维度据「已达维度 ratify + 受阻维度如实记录」处理，不强 ratify、不伪造（ADR-013）。
> - 触及 **ADR-030（production-vector-backend）**：add-only Amendment（选择矩阵据真实测量校准；不溯改正文 D1-D4，ADR-014 D5）。
> - 触及 **ADR-023（vector-backend-default）**：D1-D6 tier add-only Amendment（默认 BruteForce tier 之上据真实测量补 lancedb/qdrant tier 定位，不溯改正文）。
> - 触及 **ADR-004（local-first 0-dep 基线）**：qdrant / lancedb 均 feature-gated，默认构建仍 0 vector dep、不引入供应链面（守线，非推翻）。

## 1. 阶段目标

Phase 25 ship 后，ContextForge 的生产向量 backend 完成了 **qdrant 生命周期契约层**（connect/health/ensure-create）与 **lancedb 索引参数校验层**（`LanceIndexTuning`/`LanceAnnIndex` validate），但二者均停留在「契约 / 参数」层、未做真实 live 召回，且生产热路径 `server.rs` 仍硬编码 `BruteForceVectorBackend`。本 phase 把这些延后层兑现为真实 live 向量召回：工厂化 backend 选择并注入热路径、对真实 qdrant server 跑首次端到端 KNN（无 server 诚实延后）、在 lancedb 参数契约之上真实建 IVF_PQ/HNSW 索引并实测召回、用真实测量校准多 backend 选择矩阵。默认构建保持 0 vector 依赖、BruteForce 语义基线不变（ADR-004），既有 `cargo-test` / `go-test` / `spec-lint` 三门不退化。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. 新增 `select_vector_backend(name, dim) -> Result<Arc<dyn VectorSearcher>, VectorError>` 工厂，仿 `core/src/embedding/factory.rs::select_provider`（`factory.rs:27-30`）：默认 `""`/`"brute"` → `BruteForceVectorBackend`（始终可用）、`"qdrant"` → feature `vector-qdrant` 下 `QdrantBackend` 否则诚实 Err、`"lancedb"` → feature `vector-lancedb` 下 `LanceDbBackend` 否则诚实 Err；并注入 `server.rs:302`（hybrid）+ `server.rs:341`（semantic）替换硬编码 `BruteForceVectorBackend::new()`；默认构建（无 vector feature）semantic+hybrid 仍经 BruteForce 工作、`cargo test --workspace` 不受影响（兑现 `[SPEC-DEFER:phase-future.vector-retrieval-integration]`，phase-25 spec line 44）（AC1）
2. 首次真实兑现 qdrant 契约层之上的 **live 端到端 KNN**（connect→ensure-create→upsert→search，`qdrant.rs:330-371`），克隆 `core/examples/phase20_recall_via_retriever.rs` 为 phase29 harness，feature `vector-qdrant` + `embedding-fastembed` gate，`health()==Unreachable` 时 honest-defer（eprintln + exit 0）不伪造召回；真实召回数字真实跑出后回填 §10 + v0.22.0 evidence（AC2）
3. 用 lancedb 参数契约（`lance_db.rs:33-108`）在内嵌 Lance dataset 上真实建 **IVF_PQ/HNSW 索引**并实测召回（feature `vector-lancedb`，进程内运行，n 仍 modest），兑现 `[SPEC-DEFER:phase-future.lancedb-index-tuning]`；并产出**真实多 backend 选择矩阵测量**（brute / sqlite-vec / lancedb / qdrant 可跑则跑、不可跑诚实延后）→ ADR-030 D3 + ADR-023 tier add-only Amendment（AC3）
4. v0.22.0 release docs + `scripts/console_smoke.sh` v19（默认构建 init 基线完整断言 + 既有 step 不退化）+ phase §6 闭合 + ADR-034 据真实产物 ratify（live-server / 大语料受阻维度如实）+ ADR-030/023 add-only Amendment（不溯改正文 D5）（AC4）
5. ADR-014 D1-D5（第二十次激活）全通过（AC5）

**v0.x 版本号决策**：v0.22.0（Phase 29，承 v0.21.0；roadmap §1.1 Phase N→v0.(N-7).0）minor release（live 向量召回兑现 + 生产 backend 工厂化注入；feature-gated backends 默认不编译、不破坏既有 v0.6-v0.21 client、默认构建 0 vector 依赖 + 0 网络、BruteForce 语义基线不变）。

## 2. 业务价值

兑现 roadmap §3.11 + phase-25 spec line 44 + ADR-030 §Ratification 一路刻意延后的 live 向量召回 marker，把「契约 / 参数」层成果接到真实生产召回：

- **生产热路径仍硬编码 BruteForce**：`server.rs:302`（hybrid）+ `:341`（semantic）当前硬编码 `BruteForceVectorBackend::new()`，Phase 25 建好的 qdrant / lancedb backend 无法被生产路径选用（`[SPEC-DEFER:phase-future.vector-retrieval-integration]`，phase-25 spec line 44）。本 phase 工厂化注入，使大语料用户可经配置切到可扩展 backend，默认用户仍 0-dep BruteForce。
- **qdrant 契约层从未跑过真实 KNN**：Phase 25 只验了 connect/health/ensure-create 决策（`qdrant.rs:152-270`），live search 读路径（`qdrant.rs:330-371`）从未对真实 server 端到端跑过。本 phase 做首次真实 connect→upsert→KNN，使「qdrant 真能召回」从契约假设变为实测事实（无 server 诚实延后，不伪造）。
- **lancedb 参数层从未建过真实索引**：`LanceIndexTuning`/`LanceAnnIndex` validate（`lance_db.rs:48-108`）只校验参数、未真建索引。本 phase 在内嵌 dataset 上真建 IVF_PQ/HNSW 并实测召回，使索引参数从「校验通过」变为「真实可召回」。
- **选择矩阵缺真实测量数据**：ADR-030 D3 选择矩阵（`:42-44`）+ ADR-023 tier 当前据设计推断，无真实跨 backend 实测。本 phase 产出真实测量校准矩阵（add-only Amendment，不溯改正文）。

**不在本 phase scope**：

- qdrant 集群 / 副本 / 分片部署拓扑（仅文档化单节点基线）[SPEC-DEFER:phase-future.qdrant-deployment-topology]
- lancedb schema compaction 真实执行（likely 诚实延后）[SPEC-DEFER:phase-future.lancedb-schema-compaction]
- lancedb feature 全量 `cargo test` 的 rustc ICE 规避入 CI 默认（本 phase 用 `cargo build` + `--lib` scoped 测试）[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]
- 向量增量索引行级追加（Phase 23 已延后）[SPEC-DEFER:phase-future.vector-incremental-index]
- 大语料（百万级）性能基准与调优 [SPEC-DEFER:phase-future.vector-large-corpus-perf]

## 3. 涉及模块

### 29.1 vector backend 工厂 + server.rs 热路径注入（task-29.1）

- 新增 `core/src/retriever/vector/` 工厂 `select_vector_backend(name, dim) -> Result<Arc<dyn VectorSearcher>, VectorError>`，仿 `core/src/embedding/factory.rs::select_provider`（`factory.rs:27-30`）：`""`/`"brute"` → `BruteForceVectorBackend`（始终可用、0-dep）；`"qdrant"` → feature `vector-qdrant` 下 `QdrantBackend` 否则显式 feature-not-enabled Err（不 panic / 不静默 fallback）；`"lancedb"` → feature `vector-lancedb` 下 `LanceDbBackend` 否则显式 Err；未知 name → 显式 unknown-backend Err
- 修改 `core/src/server.rs`——`:302`（hybrid 路径）+ `:341`（semantic 路径）以工厂调用替换硬编码 `BruteForceVectorBackend::new()`（仿 `:339` `select_provider` 样板）；默认参数令默认构建仍选 BruteForce
- 兑现 `[SPEC-DEFER:phase-future.vector-retrieval-integration]`（phase-25 spec line 44）
- 同源验证（≥2，deterministic：默认 name 返回 BruteForce / feature off 时 qdrant·lancedb 返回诚实 Err / 默认构建 semantic+hybrid 经 BruteForce 仍工作 `cargo test --workspace` 不受影响）

### 29.2 qdrant live KNN + 真实召回 harness（task-29.2）

- 克隆 `core/examples/phase20_recall_via_retriever.rs` 为 phase29 qdrant harness，把 `BruteForceVectorBackend` 换为 `QdrantBackend::connect(QdrantConnConfig::from_env())`，feature `vector-qdrant` + `embedding-fastembed` gate
- 对**真实 qdrant server** 跑 connect→ensure-create→upsert→KNN（live search 读路径 `qdrant.rs:330-371`）；`backend.health()==Unreachable` 时 honest-defer（eprintln + exit 0），CI 无 server 不伪造召回（ADR-013）
- 文档化 qdrant 单节点部署基线；集群 / 副本 / 分片 → `[SPEC-DEFER:phase-future.qdrant-deployment-topology]`
- 同源验证（🔴 真实 server：live KNN 召回数字真实跑出后回填 §10 + v0.22.0 evidence / 🟢 deterministic：harness 编译 + 无 server 干净 honest-defer 证明 wiring 不伪造召回）

### 29.3 lancedb 真实 ANN 索引调优 + 多 backend 选择矩阵（task-29.3）

- 用 `LanceIndexTuning`/`LanceAnnIndex` 参数契约（`lance_db.rs:33-108`）在内嵌 Lance dataset 上真实建 **IVF_PQ/HNSW 索引**并实测召回（feature `vector-lancedb`，进程内运行，n 仍 modest），兑现 `[SPEC-DEFER:phase-future.lancedb-index-tuning]`
- 产出**真实多 backend 选择矩阵测量**（brute / sqlite-vec / lancedb / qdrant 可跑则跑、不可跑诚实延后）→ ADR-030 D3（`:42-44`）+ ADR-023 tier add-only Amendment（不编辑其 D-body，ADR-014 D5）
- lancedb schema compaction 执行 → `[SPEC-DEFER:phase-future.lancedb-schema-compaction]`（likely 诚实延后）
- lancedb feature build caveat：broad `cargo test` 触 rustc ICE → 用 `cargo build` + `--lib` scoped 测试 `[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]`
- 同源验证（🟡 feature build：真实索引召回数字真实跑出后回填 / 🔴 大语料 / compaction 真实执行或诚实延后）

### 29.4 v0.22.0 closeout（task-29.4）

- 修改 `scripts/console_smoke.sh`——v19 banner + v19 changelog 块 + 新 step（默认构建 init 基线完整断言；live-vector 在 CI 无 console-api 运行时面，取文档 / 状态 step）+ 既有 step 不退化；`internal/cli/smoke_syntax_test.go` 新 Test 断言新 step + 既有 step 无回归（既有 smoke 分母不溯改，ADR-014 D5）
- 新增 `docs/releases/v0.22.0-{evidence,artifacts}.md`（tag SHA / run id / image digest 以尖括号 backfill marker 写，由后续 post-tag-push backfill PR 回填）+ `README.md` v0.22 段 + `RELEASE_NOTES.md` v0.22.0 段
- 修改 `docs/decisions/adr-034-production-vector-live-recall.md`——Status Proposed→Accepted（逐 D，如实：live-server / 大语料延后维度据真实证据部分 ratify）+ 新增 `## Ratification（v0.22.0 / task-29.4）` 段
- 修改 `docs/decisions/adr-030-production-vector-backend.md`（`## Amendment (Phase 29 / v0.22.0)`）+ `docs/decisions/adr-023-vector-backend-default.md`（tier add-only Amendment），不编辑其正文（ADR-014 D5）
- 修改 `docs/specs/phases/phase-29-live-vector-recall.md`——Status Draft→Done + §6 AC 逐维如实勾选
- 修改 `docs/s2v-adapter.md`（Phase 29 row + Task rows + ADR-034 row + BDD row）

### BDD feature

- 新增 `test/features/phase-29-live-vector-recall.feature`（≥4 scenario：工厂 + 热路径注入 / qdrant live KNN 诚实延后 / lancedb 真实索引 + 选择矩阵 / 默认构建 0-dep 基线不变）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 29.1 | `core/src/retriever/vector/` `select_vector_backend` 工厂（仿 `embedding/factory.rs`）+ `server.rs:302`/`:341` 热路径注入替换硬编码 BruteForce | `../tasks/task-29.1-vector-backend-factory-and-hotpath-injection.md` |
| 29.2 | qdrant live connect→ensure-create→upsert→KNN harness（克隆 `phase20_recall_via_retriever.rs`）+ 无 server honest-defer + 单节点部署基线文档 | `../tasks/task-29.2-qdrant-live-knn-and-recall-harness.md` |
| 29.3 | lancedb 真实 IVF_PQ/HNSW 索引构建 + 实测召回 + 多 backend 选择矩阵真实测量 → ADR-030/023 add-only Amendment | `../tasks/task-29.3-lancedb-ann-index-tuning-and-backend-matrix.md` |
| 29.4 | smoke v19 + v0.22.0 closeout + ADR-034 ratify + ADR-030/023 add-only Amendment + adapter + feature | `../tasks/task-29.4-closeout-v0.22.0.md` |

## 5. 依赖关系

- **task-29.1**（工厂 + 热路径注入）dep 既有 `core/src/retriever/vector/traits.rs` `VectorSearcher` trait + `embedding/factory.rs` 工厂样板 + `server.rs:302`/`:341` 热路径；可独立先行（不依赖 29.2/29.3 的 feature build）。
- **task-29.2**（qdrant live KNN）建议 29.1 先 merge（工厂 `"qdrant"` 分支稳定后接 harness）+ dep 既有 `qdrant.rs` connect/health/ensure-create/live-search 契约层（Phase 25 Done）+ 真实 qdrant server（无则 honest-defer）。
- **task-29.3**（lancedb 索引 + 矩阵）建议 29.1 先 merge（工厂 `"lancedb"` 分支）+ dep 既有 `lance_db.rs` 参数契约层（Phase 25 Done）+ feature `vector-lancedb` 可构建环境；选择矩阵测量需 29.2 qdrant 维度（可跑则纳入、不可跑诚实延后）。
- **task-29.4**（closeout）dep 29.1 + 29.2 + 29.3 全 Done；release docs / smoke v19 / ADR-034 ratify 据三 task 真实产物（live KNN / 真实索引召回 / 真实矩阵测量，受阻维度如实）。
- 外部：ADR-034（本 phase 新 Proposed）/ ADR-030（production-vector-backend，本 phase 选择矩阵 add-only Amendment）/ ADR-023（vector-backend-default，tier add-only Amendment）/ ADR-004（本地优先，默认构建 0 vector dep baseline 不变）/ ADR-012（tag/release 主 agent 自治触发，outward-facing 不可逆须用户显式授权）/ ADR-014 第二十次激活 / ADR-013（禁伪造红线，真实 live KNN / 真实索引召回 / 真实矩阵测量，受阻不伪造）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（每条 AC 含 ADR-014 D3 verified by 显式 owner；Draft 阶段未勾选，实施后逐条置 `[x]`）**：

- [ ] **AC1**（vector backend 工厂 + server.rs 热路径注入；🟢 deterministic）：新增 `select_vector_backend(name, dim) -> Result<Arc<dyn VectorSearcher>, VectorError>`（仿 `embedding/factory.rs::select_provider`）——默认 `""`/`"brute"` → `BruteForceVectorBackend`、`"qdrant"`/`"lancedb"` feature off 时返回诚实 Err（不伪造成功）；`server.rs:302`（hybrid）+ `:341`（semantic）经工厂注入替换硬编码 `BruteForceVectorBackend::new()`；默认构建（无 vector feature）semantic+hybrid 仍经 BruteForce 工作、`cargo test --workspace` 不受影响 — verified by task-29.1 §6 + phase-smoke step 1
- [ ] **AC2**（qdrant live 端到端 KNN；🔴 live server / 🟢 wiring）：phase29 qdrant harness 经 `QdrantBackend::connect(QdrantConnConfig::from_env())` 对真实 server 跑 connect→ensure-create→upsert→KNN（`qdrant.rs:330-371`），真实召回数字真实跑出后回填 §10 + v0.22.0 evidence（NEVER 预填）；CI 无 server 时 `health()==Unreachable` honest-defer（eprintln + exit 0）干净通过、不伪造召回（ADR-013）；qdrant 集群 / 副本拓扑 [SPEC-DEFER:phase-future.qdrant-deployment-topology] — verified by task-29.2 §6 + phase-smoke step 2
- [ ] **AC3**（lancedb 真实索引 + 多 backend 选择矩阵；🟡 feature build / 🔴 大语料）：feature `vector-lancedb` 下用 `LanceIndexTuning`/`LanceAnnIndex` 参数契约（`lance_db.rs:33-108`）在内嵌 dataset 上真建 IVF_PQ/HNSW 索引并实测召回（真实跑出后回填，非预填）；真实多 backend 选择矩阵测量（brute / sqlite-vec / lancedb / qdrant 可跑则跑、不可跑诚实延后）→ ADR-030 D3 + ADR-023 tier add-only Amendment（不溯改正文 D5）；compaction 真实执行或诚实延后 [SPEC-DEFER:phase-future.lancedb-schema-compaction]；broad test rustc ICE → `cargo build`+`--lib` scoped [SPEC-DEFER:phase-future.lancedb-build-prereq-ci] — verified by task-29.3 §6 + phase-smoke step 3
- [ ] **AC4**（默认构建 0-dep 基线不变 + v0.22.0 closeout）：默认构建（无 `vector-qdrant`/`vector-lancedb` feature）0 vector 依赖、BruteForce 语义基线不变（ADR-004）；v0.22.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ `scripts/console_smoke.sh` v19（默认构建 init 基线完整断言 + 既有 step 不退化）+ ADR-034 据真实产物 ratify（D1 工厂+热路径达成 / D2 live-server 据真实证据部分·受阻如实 / D3 lancedb 索引达成 / D4 矩阵 Amendment / D5 baseline 不变）+ ADR-030/023 add-only Amendment + phase §6 闭合 — verified by task-29.4 §6
- [ ] **AC5**：ADR-014 cross-validation gate 全套通过（第二十次激活）— D1 mapping + D2 lint `--touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-28 不溯改 — verified by task-29.4 closeout PR body

**端到端 smoke（C1 集成兜底）**：(1) `select_vector_backend` 默认返回 BruteForce + feature off 时 qdrant/lancedb 诚实 Err，且 `server.rs` 热路径经工厂注入、默认构建 `cargo test --workspace` semantic+hybrid 全过；(2) qdrant harness 对真实 server 跑 live KNN 出真实召回（真实跑出后回填），无 server 时 honest-defer exit 0 干净通过（受阻如实标注，不伪造）；(3) lancedb feature build 下真建 IVF_PQ/HNSW 索引 + 实测召回 + 多 backend 选择矩阵真实测量喂 ADR-030/023 add-only Amendment（大语料 / compaction 受阻如实延后）全 PASS（受阻态如实标注）。

## 7. 阶段级风险

- **R1（低）工厂注入改动波及 server.rs 热路径默认行为**：`server.rs:302`/`:341` 现硬编码 BruteForce，工厂化后默认参数须保证默认构建仍选 BruteForce（行为保持）。
  - **缓解**：task-29.1 默认 name（`""`/`"brute"`）严格映射 BruteForce + 默认构建（无 vector feature）`cargo test --workspace` semantic+hybrid deterministic 验过再标 AC1。stop-condition：默认构建语义路径行为变化则不标 `[x]`。
- **R2（高）CI 无真实 qdrant server → live KNN 不可在 CI 验**：qdrant live 端到端 KNN 依赖外部真实 server，CI 无 server。
  - **缓解**：task-29.2 `health()==Unreachable` 时 honest-defer（eprintln + exit 0），真实召回数字仅在真实 server（manual / dev-box）跑出后回填 §10 + v0.22.0 evidence。stop-condition：无 server 时不伪造召回数字 / 不强标 AC2 live 维度为 `[x]`（wiring + honest-defer deterministic 达成则部分 ratify，ADR-013）。
- **R3（中）lancedb feature broad `cargo test` 触 rustc ICE**：Phase 23 已知 lancedb feature 在 broad `cargo test` 下触 rustc ICE。
  - **缓解**：task-29.3 用 `cargo build` + `--lib` scoped 测试规避，CI 默认不构建该 feature `[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]`。stop-condition：scoped 测试仍 ICE 则如实记录受阻、不伪造索引召回数字。
- **R4（中）多 backend 选择矩阵部分 backend 不可跑**：sqlite-vec（MSVC）/ qdrant（无 server）/ lancedb（ICE）在不同环境可跑性不一。
  - **缓解**：task-29.3 矩阵仅纳入真实可跑维度的真实测量，不可跑维度诚实延后 + 标注环境前提；add-only Amendment 仅据真实测量校准（ADR-014 D5）。stop-condition：不为补全矩阵伪造不可跑 backend 的测量数。

## 8. Definition of Done

- 4 task spec（29.1-29.4）顶部 `**Status**: Done`
- §6 阶段级 AC1-5 全 `[x]`（受阻态按 ADR-013 如实记录，不伪造——如 qdrant live 无 server honest-defer / lancedb 大语料受阻 / 矩阵部分 backend 不可跑延后）
- 端到端 smoke 3 step 全 PASS（含受阻态如实标注）
- **ADR**：ADR-034 `Proposed → Accepted`（据真实 live KNN / 真实索引召回 / 真实矩阵测量产物逐 D ratify；live-server / 大语料受阻维度据「已达维度 ratify + 受阻维度如实记录」）；ADR-030 经 `## Amendment (Phase 29 / v0.22.0)` 记录选择矩阵真实测量校准 + ADR-023 tier add-only Amendment（均不溯改正文，ADR-014 D5）
- **adapter**：§Phase 索引 Phase 29 `Draft → Done` + `Tasks 0 → 4`；§ADR 索引 ADR-034；§BDD 追加 phase-29 feature 行；ADR-030/023 Amendment 记录
- **release**：`docs/releases/v0.22.0-{evidence,artifacts}.md` + `RELEASE_NOTES.md` v0.22.0 段 + README v0.22 段
- **smoke**：`scripts/console_smoke.sh` v19（默认构建 init 基线 smoke + 既有 step 不退化）+ `internal/cli/smoke_syntax_test.go` 新 Test 同步（既有 smoke 分母不溯改）
- **feature**：`test/features/phase-29-live-vector-recall.feature` 已于本 phase 创建
- **follow-up**：qdrant 部署拓扑 `[SPEC-DEFER:phase-future.qdrant-deployment-topology]` + lancedb schema compaction `[SPEC-DEFER:phase-future.lancedb-schema-compaction]` + lancedb build CI 前置 `[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]` + 向量增量索引 `[SPEC-DEFER:phase-future.vector-incremental-index]` + 大语料性能 `[SPEC-DEFER:phase-future.vector-large-corpus-perf]` 留 backlog
