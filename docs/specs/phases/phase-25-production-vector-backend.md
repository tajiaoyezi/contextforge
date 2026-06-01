# Phase 25 · production-vector-backend

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 把 ADR-023 列为生产规模 ANN 两档的 **qdrant**（外部 gRPC server，D3 hosted/scale-out）与 **lancedb**（嵌入式列存，D4 embedded-columnar）从 Phase 18 spike 态推向生产：qdrant 加 connect/health-probe/collection ensure-create/连接配置的**生命周期层**（契约层真实可验证，不需 live server），lancedb 做**真实可构建性调查**（dev-box `cargo build --features vector-lancedb`，protoc 前置，仿 task-23.2 sqlite-vec MSVC 调查 pattern）+ 索引调参参数，并产出**生产 backend 选择矩阵**。ADR-013 关键：qdrant 需 live server（CI 无）、lancedb 需 protoc 且可能在某平台受阻——二者均以诚实 stop-condition 处理，不伪造跨环境通过。v0.18.0 收口。对应 `docs/roadmap.md` §3.6。
>
> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md` §3.6 → `docs/decisions/adr-030-production-vector-backend.md`（本 phase 新 Proposed，D1 qdrant 生命周期 / D2 lancedb 可构建性 / D3 选择矩阵 / D4 默认 0-dep）→ `docs/decisions/adr-023-vector-backend-default.md`（D3 qdrant「hosted/scale-out」原文 + D4 lancedb「embedded-columnar alternative」原文 + D5 默认 0-vector-dep + Consequences/Follow-ups）→ `docs/decisions/adr-028-vector-persistence-strategy.md`（Phase 23 嵌入式/fallback 两档推进结论 + Amendment pattern）→ `docs/spikes/phase-18-qdrant.md`（external server `is_local()==false` + server RSS≈104.8MB + server-lifecycle Follow-up）→ `docs/spikes/phase-18-lancedb.md`（protoc 前置 + Lance/DataFusion ~5min 构建 + index-tuning/schema-compaction Follow-up）→ `docs/spikes/phase-23-sqlite-vec-cross-platform.md`（真实可构建性调查 pattern + 三态如实标）→ `core/src/retriever/vector/qdrant.rs::QdrantBackend`（`Qdrant::from_url` + `block_on` + `open` drop+create + `Distance::Cosine`）+ `core/src/retriever/vector/lance_db.rs::LanceDbBackend`（`lancedb::connect` + `create_empty_table` + `nearest_to().distance_type(Cosine)`）+ `core/src/retriever/vector/types.rs::VectorIndexConfig`（`dim`/`metric`/`collection_id`/`persistence_path`）+ `core/src/retriever/vector/traits.rs`（三 trait freeze）+ `core/Cargo.toml`（`vector-qdrant=["dep:qdrant-client"]` / `vector-lancedb=["dep:lancedb","dep:arrow-array","dep:futures"]`，全 optional）→ AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5，第十六次激活）→ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造凭据红线）→ `docs/decisions/adr-008-core-library-selection.md`（依赖选型 add-only）→ `docs/decisions/adr-004-local-first-privacy-baseline.md`（默认 0 dep / 0 network）。
>
> **ADR 影响面（已识别）**：
> - **ADR-030 production-vector-backend（新，Proposed）**：记 qdrant server 生命周期层（connect/health-probe/ensure-create/config）+ lancedb 真实可构建性调查结论（通过或诚实 stop-condition）+ 索引调参参数 + 生产 backend 选择矩阵。落地后据真实非合成契约单测 / 真实 dev-box 构建结果 ratify（ADR-013）。
> - 触及 **ADR-023（vector-backend-default）**：D3 qdrant「hosted/scale-out」与 D4 lancedb「embedded-columnar alternative」两 tier 由本 phase 推进——以 add-only Amendment 记录推进结果，不溯改 ADR-023 正文 D1-D6（D5）。
> - 触及 **ADR-008（core-library-selection）**：若 lancedb 索引调参 / qdrant 生命周期引入新 crate，按 add-only Amendment 记录（不溯改既有 D 段）；qdrant-client / lancedb / arrow-array / futures 均为 task-18.4/18.5 既有 optional dep，本 phase 不新增 direct dep 为基线。

## 1. 阶段目标

v0.18.0 ship 后，ContextForge 的生产规模向量 backend 两档具备可生产化的基础层：**qdrant** 在 `vector-qdrant` feature 下有 connect/health-probe/collection ensure-create/连接配置的生命周期层（契约层 shape/config/ensure-create 决策在不连 live server 下 `cargo test` 可断言；真实 KNN over live qdrant 因 CI 无 server 诚实延后），**lancedb** 在 `vector-lancedb` feature 下给出真实 dev-box 可构建性结论（构建通过则记真实凭据 + 索引调参参数校验可单测，确证受阻则诚实文档化 stop-condition 承 protoc-prereq 先例，不伪造跨平台构建通过），并产出**生产 backend 选择矩阵**（语料规模 × 部署形态 → 推荐 backend + caveat）。默认构建仍 0 vector 依赖（ADR-023 D5）、BM25-only baseline 行为不变。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. `vector-qdrant` feature 下 `QdrantBackend` 有生命周期层——连接配置校验（url/dim/collection 名）+ health-probe 入口（live 时 readiness / 无 server 时可识别 unreachable，不 panic）+ collection ensure-create 决策（reuse/create/error）；契约层 shape/config/ensure-create 决策在不连 live server 下 `cargo test --features vector-qdrant` 可断言；真实 KNN over live qdrant 因 CI 无 server 诚实延后 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`；默认构建 0 新依赖不退化（AC1）
2. lancedb 真实 dev-box 可构建性给出结论：`cargo build --features vector-lancedb`（含 protoc 前置）在 dev box 真实构建通过则记真实凭据（rustc/protoc 版本/耗时）+ feature 下既有 lancedb 契约不退化 + 索引调参参数（IVF_PQ/HNSW 参数 + compaction 口径）校验可单测；或确证受阻时诚实文档化 stop-condition（承 `docs/spikes/phase-18-lancedb.md` protoc-prereq + sqlite-vec MSVC 先例），按 ADR-013 不伪造跨平台构建通过；产出 `docs/spikes/phase-25-lancedb-buildability.md` 三态如实标（AC2）
3. 生产 backend 选择矩阵产出（dev/小语料→hnsw / 单机嵌入式→sqlite-vec / 大语料列存→lancedb / hosted scale-out→qdrant，每档含 caveat）；`scripts/console_smoke.sh` v15 通过 `bash -n` + 向量生产 backend 状态 smoke 断言 + 既有 step 不退化（AC3）
4. v0.18.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ ADR-030 据真实非合成结果 ratify 或记录维持 + ADR-023/008 add-only Amendment + phase §6 闭合（AC4）
5. ADR-014 D1-D5（第十六次激活）全通过（AC5）

**v0.x 版本号决策**：v0.18.0 minor release（生产规模向量 backend 两档生命周期 + 可构建性收口；默认构建仍 BM25-only baseline + 0 vector 依赖——qdrant 生命周期层 / lancedb 可构建性均在 feature 下，add-only 不破坏既有客户端）。

## 2. 业务价值

直接推进 ADR-023 D3/D4 两个生产规模 tier 与 task-18.4/18.5 spike 记录的 Follow-up：

- **qdrant 生命周期**：ADR-023 D3 把 qdrant 定为 hosted/multi-agent/scale-out 档，但 `core/src/retriever/vector/qdrant.rs` 的 `open` 是 spike 级「drop+create」、无 connect 探活/health/ensure-create/连接配置。`docs/spikes/phase-18-qdrant.md` 把 server-lifecycle orchestration 列为 Follow-up（`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`）。本 phase 加可契约验证的生命周期层（不连 live server 即可单测 shape/config/ensure-create 决策），让 qdrant 从 spike 推向可生产化——真实 KNN over live qdrant 因 CI 无 server 诚实延后。
- **lancedb 可构建性**：ADR-023 D4 把 lancedb 定为「embedded-columnar alternative」（最快写入 + 列存持久），但 `docs/spikes/phase-18-lancedb.md` 记录其构建**需 protoc** + Lance/DataFusion ~5min。protoc 前置在某平台（仿 sqlite-vec 当年 MSVC 受阻）可能成为构建 blocker。本 phase 做真实 dev-box 可构建性调查（仿 task-23.2 pattern），落地真实凭据或诚实 stop-condition，缩小或如实记录构建缺口。
- **生产 backend 选择矩阵**：ADR-023 D1-D4 给了 tier 排序但无「按语料规模/部署形态选 backend」的可操作矩阵。本 phase 把四档（hnsw/sqlite-vec/lancedb/qdrant）按部署场景收敛成一张可查矩阵 + 每档 caveat，降低选型门槛。
- **PRD §Constraints（local-first + 生产规模）**：dev-box 可构建性 + hosted scale-out backend 生命周期在向量路径上推进，同时默认 0-dep / 0-network baseline（ADR-004）不破坏。

**不在本 phase scope**：

- qdrant 真实 live-server 集成 / KNN over live qdrant [SPEC-DEFER:phase-future.qdrant-server-lifecycle]——CI 无在跑的 qdrant server，契约层可验证、live 集成延后
- qdrant 集群 / 复制 / 部署拓扑 [SPEC-DEFER:phase-future.qdrant-deployment-topology]——hosted 运维硬化项
- lancedb ANN 索引真实性能 / 大索引建图 [SPEC-DEFER:phase-future.lancedb-index-tuning]——构建通过后的集成验证
- lancedb 数据集 compaction / schema 演进 [SPEC-DEFER:phase-future.lancedb-schema-compaction]——承 `docs/spikes/phase-18-lancedb.md` Follow-up
- CI 注入 protoc / 跨 CI lancedb 构建持续守护 [SPEC-DEFER:phase-future.lancedb-build-prereq-ci]——承 `docs/spikes/phase-18-lancedb.md` Follow-up
- 把 qdrant/lancedb 接进 `core/src/server.rs` 语义热路径 [SPEC-DEFER:phase-future.vector-retrieval-integration]——backend 生命周期/可构建性先行，热路径接入后续
- hybrid / reranker / remote embedding provider [SPEC-DEFER:phase-future.hybrid-scoring] / [SPEC-DEFER:phase-future.reranker] / [SPEC-DEFER:phase-future.embedding-provider-remote]——已在 Phase 21/22 落地或属其他版本

## 3. 涉及模块

### 25.1 qdrant server 生命周期层（task-25.1）

- 修改 `core/src/retriever/vector/qdrant.rs`——`QdrantBackend` 加连接配置（url/timeout/可选 api-key/可选 TLS 收敛为可校验结构）+ health-probe 入口（live readiness / 无 server unreachable 可识别态）+ collection ensure-create（存在且 dim/metric 匹配→reuse / 不存在→create / 不匹配→可识别 error，替代 spike 的无脑 drop+create）
- 复用 `core/src/retriever/vector/types.rs::VectorIndexConfig`（`dim` / `metric` / `collection_id`）作为 ensure-create 期望值来源
- 复用既有 `QDRANT_URL` env（`core/src/retriever/vector/qdrant.rs::QdrantBackend::new`）作连接来源，按需扩展配置入参
- 同源 Rust tests（≥3，feature `vector-qdrant` 下，**不连 live server**：连接配置校验 / health-probe 在无 server 时返 unreachable 不 panic / collection ensure-create 决策 reuse·create·error 三分支）
- `core/Cargo.toml`——`vector-qdrant` feature 如需生命周期相关 crate 面按 add-only 评估（R7 经主 agent；qdrant-client 1.18 已 optional）

### 25.2 lancedb 可构建性 + 索引调参（task-25.2）

- 调查 `core/Cargo.toml` `vector-lancedb` feature / `core/src/retriever/vector/lance_db.rs`——dev box 真实 `cargo build --features vector-lancedb`（含 protoc 前置探测/安装），仿 task-23.2 sqlite-vec MSVC 调查 pattern
- 修改 `core/src/retriever/vector/lance_db.rs`——加索引调参参数（IVF_PQ/HNSW 的 num_partitions/num_sub_vectors/metric + compaction 触发口径收敛为可校验配置结构；参数范围校验在不建真实索引下可单测）
- 新增 `docs/spikes/phase-25-lancedb-buildability.md`——记录调查方法 + 真实构建结果（dev box rustc/protoc 版本/耗时 + 三态如实标：🟢 构建通过 / 🔴 确证受阻 stop-condition / 🟡 部分平台·caveat）
- 若构建通过：feature 下 dev box `cargo build` 通过 + 既有 `vector-lancedb` backend 契约不退化 + 索引调参参数校验单测；若受阻：诚实文档化 stop-condition（承 `docs/spikes/phase-18-lancedb.md` protoc-prereq + sqlite-vec MSVC 先例），不伪造跨平台构建通过
- 同源 Rust test（≥2：feature `vector-lancedb` 下既有 lancedb backend 契约不退化 + 索引调参参数范围校验）

### 25.3 生产 backend 选择矩阵 + closeout（task-25.3）

- 产出生产 backend 选择矩阵（语料规模 × 部署形态 → 推荐 backend + caveat），写入 release docs + adapter
- 修改 `scripts/console_smoke.sh`——v15：向量生产 backend 状态 smoke 断言（qdrant/lancedb 为 feature 层验证、非 console 热路径 + 默认构建 intact 断言，承 task-23.3 smoke pattern），既有 step 不退化
- 新增 `docs/releases/v0.18.0-{evidence,artifacts}.md` + `README.md` v0.18 段 + `RELEASE_NOTES.md` v0.18.0 段
- 修改 `docs/decisions/adr-030-production-vector-backend.md`——据真实结果 Proposed→Accepted 或记录维持 + ADR-023/008 add-only Amendment（推进 D3/D4 tier 结果记录，不溯改正文，D5）
- 修改 `docs/s2v-adapter.md`（Phase 25 Draft→Done + Tasks 0→3；ADR-030 状态；ADR-023 D3/D4 推进记录）

### BDD feature

- 新增 `test/features/phase-25-production-vector-backend.feature`（≥3 scenario：qdrant 生命周期契约 / lancedb 可构建性调查结论 / 生产 backend 选择矩阵 + 收口）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 25.1 | `core/src/retriever/vector/qdrant.rs` `QdrantBackend` 连接配置 + health-probe + collection ensure-create + 契约测试（不连 live server） | `../tasks/task-25.1-qdrant-server-lifecycle.md` |
| 25.2 | `core/Cargo.toml` `vector-lancedb` + `core/src/retriever/vector/lance_db.rs` 真实可构建性调查 + 索引调参参数 + `docs/spikes/phase-25-lancedb-buildability.md` | `../tasks/task-25.2-lancedb-buildability-and-index-tuning.md` |
| 25.3 | 生产 backend 选择矩阵 + smoke v15 + v0.18.0 closeout + ADR-030 ratify | `../tasks/task-25.3-closeout-v0.18.0.md` |

## 5. 依赖关系

- **task-25.1**（qdrant 生命周期）dep Phase 18 task-18.4（`QdrantBackend` + `vector-qdrant` feature 已落地，qdrant-client 1.18 已 optional）+ `VectorIndexConfig`（task-18.1 字段）；可与 25.2 并行（写路径不相交：qdrant.rs vs lance_db.rs/Cargo.toml）。
- **task-25.2**（lancedb 可构建性）dep Phase 18 task-18.5（`LanceDbBackend` + `vector-lancedb` feature 已落地，Linux protoc 可构建凭据）；调查类任务，结论可能为「构建通过」或「诚实文档化 stop-condition」。
- **task-25.3**（closeout）dep 25.1 + 25.2 全 Done；生产 backend 选择矩阵为本 task 子项。
- 外部：ADR-030（本 phase 新 Proposed）/ ADR-023（vector-backend-default，本 phase 推进 D3/D4 tier，add-only Amendment）/ ADR-028（vector-persistence-strategy，嵌入式/fallback 两档前置结论）/ ADR-008（core-library-selection，依赖变更 add-only）/ ADR-014 第十六次激活 / ADR-013（禁伪造 live-server / 跨平台凭据）/ ADR-004（默认 0 dep / 0 network）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（每条 AC 含 ADR-014 D3 verified by 显式 owner；Draft 阶段未勾选，实施后逐条置 `[x]`）**：

- [x] **AC1**：`vector-qdrant` feature 下 `QdrantBackend` 有生命周期层（连接配置校验 + health-probe 入口 + collection ensure-create 决策）；契约层 shape/config/ensure-create 决策在不连 live server 下 `cargo test --features vector-qdrant` 可断言；真实 KNN over live qdrant 因 CI 无 server 诚实延后（`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`，禁伪造 live-server 通过，ADR-013）；默认构建 0 新依赖不退化 — verified by task-25.1 §6 AC1-5 + phase-smoke step 1
- [x] **AC2**：lancedb 真实 dev-box 可构建性给出结论——`cargo build --features vector-lancedb`（含 protoc 前置）构建通过则记真实凭据 + 既有契约不退化 + 索引调参参数校验可单测，或确证受阻时诚实文档化 stop-condition（承 protoc-prereq + sqlite-vec MSVC 先例，禁伪造跨平台构建通过，ADR-013）；`docs/spikes/phase-25-lancedb-buildability.md` 三态如实标 — verified by task-25.2 §6 AC1-4 + phase-smoke step 2
- [x] **AC3**：生产 backend 选择矩阵产出（语料规模 × 部署形态 → 推荐 backend + caveat）；`scripts/console_smoke.sh` v15 `bash -n` exit 0 + 向量生产 backend 状态 smoke 断言 + 既有 step 不退化 — verified by task-25.3 §6 AC1 + phase-smoke step 3
- [x] **AC4**：v0.18.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ ADR-030 据真实非合成结果 ratify 或记录维持 + ADR-023/008 add-only Amendment + phase §6 闭合 — verified by task-25.3 §6 AC2-3
- [x] **AC5**：ADR-014 cross-validation gate 全套通过（第十六次激活）— D1 mapping + D2 lint `--touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-24 不溯改 — verified by task-25.3 closeout PR body

**端到端 smoke（C1 集成兜底）**：(1) `vector-qdrant` feature 下 qdrant 生命周期契约（config/health-probe/ensure-create 决策，不连 live server）；(2) lancedb 可构建性调查结论（构建通过 / 受阻如实标）；(3) 生产 backend 选择矩阵 + smoke v15 `bash -n` 全 PASS（含受阻态如实标注）。

## 7. 阶段级风险

- **R1（高）qdrant 真实 KNN 集成需 live server，CI 无**：`is_local()==false`，CI 无在跑的 qdrant server（`docs/spikes/phase-18-qdrant.md` 用手动起的 musl 二进制取真实数据）。
  - **缓解**：task-25.1 把生命周期层拆为「契约层（config 校验 / health-probe shape / ensure-create 决策逻辑）」与「live-server 集成」两面——契约层在不连 server 下 deterministic 单测可断言（喂入构造的响应 / 校验入参）；真实 KNN over live qdrant 诚实延后 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`，按 ADR-013 不伪造 live-server 通过。AC1 以「契约层真实单测 + live 集成诚实延后」满足。
- **R2（高）lancedb 在 dev box 因 protoc/Arrow 栈构建受阻**：`docs/spikes/phase-18-lancedb.md` 记录构建需 protoc + Lance/DataFusion ~5min；某平台（仿 sqlite-vec 当年 MSVC 受阻）可能受阻。
  - **缓解**：task-25.2 真实尝试 dev-box 构建（含 protoc 前置探测/安装）；构建通过即记真实凭据 + 契约不退化，受阻则诚实文档化 stop-condition（承 protoc-prereq + sqlite-vec MSVC 先例），按 ADR-013 不伪造跨平台构建通过——AC2 在「确证受阻」态下以「真实调查 + stop-condition 文档」满足，不标伪造 `[x]`。本 phase 不因 lancedb 受阻阻塞 25.1 / 25.3（qdrant 生命周期与选择矩阵独立推进）。
- **R3（中）qdrant/lancedb 生命周期/调参引入新 crate 面**：default build 须 0 新 vector 依赖。
  - **缓解**：qdrant-client / lancedb / arrow-array / futures 均为 task-18.4/18.5 既有 optional dep；生命周期/调参优先复用既有 API；如需新 crate 则仅在 feature 下引入，经主 agent R7 chore + ADR-008 add-only 记录（subagent 不自改 Cargo.toml），默认构建 0 新 dep（ADR-023 D5 / ADR-004）。

## 8. Definition of Done

- 3 task spec（25.1-25.3）顶部 `**Status**: Done`
- §6 阶段级 AC1-5 全 `[x]`（受阻态按 ADR-013 如实记录，不伪造）
- 端到端 smoke 3 step 全 PASS（含受阻态如实标注）
- **ADR**：ADR-030 `Proposed → Accepted`（据真实非合成契约单测 / 真实 dev-box 构建结果）或据实测记录维持 + 文档化；ADR-023 / ADR-008 add-only Amendment 记录 D3/D4 tier 推进结果（不溯改正文，D5）
- **adapter**：§Phase 索引 Phase 25 `Draft → Done` + `Tasks 0 → 3`；§ADR 索引 ADR-030；§BDD 追加 phase-25 feature 行；ADR-023 D3/D4 tier 推进记录
- **spike evidence**：`docs/spikes/phase-25-lancedb-buildability.md`（可构建性调查，三态如实）+ qdrant 生命周期契约证据（task-25.1 spec §10）
- **release**：`docs/releases/v0.18.0-{evidence,artifacts}.md` + `RELEASE_NOTES.md` v0.18 段 + README v0.18 段 + 生产 backend 选择矩阵
- **follow-up**：qdrant live-server 集成 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` / lancedb 索引调参真实性能 `[SPEC-DEFER:phase-future.lancedb-index-tuning]` / compaction `[SPEC-DEFER:phase-future.lancedb-schema-compaction]` 留 backlog
