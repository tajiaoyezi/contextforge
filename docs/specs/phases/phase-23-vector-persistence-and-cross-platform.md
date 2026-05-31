# Phase 23 · vector-persistence-and-cross-platform

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 解决 Phase 18/19 向量检索落地后两块向量持久化 / 跨平台技术债：**hnsw 图持久化**（避免重启重建——Phase 18 task-18.6 实测 100k 图构建 28.4s，`docs/spikes/phase-18-hnsw.md` / `adr-023:55-60`）与 **sqlite-vec Windows MSVC 跨平台**（`adr-023:101` / `docs/spikes/phase-18-sqlite-vec.md` / `docs/releases/v0.11.0-evidence.md`）；并评估**向量增量索引**（承 Phase 18/19 默认全量 reindex，`phase-19` §2）。v0.16.0 收口。对应 `docs/roadmap.md` §3.4。
>
> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md` §3.4 → `docs/decisions/adr-023-vector-backend-default.md`（D2 hnsw 跨平台 fallback「graph-persistence 未存在」原文 + Consequences/Follow-ups 三个 marker）→ `docs/spikes/phase-18-hnsw.md`（instant-distance 全量建图 + 100k 28.4s 实测 + 重启重建）→ `docs/spikes/phase-18-sqlite-vec.md`（Linux gcc 可构建 + Windows MSVC 受阻凭据）→ `core/src/retriever/vector/hnsw.rs::HnswBackend`（`instant-distance` 全量建图，无序列化）+ `core/src/retriever/vector/sqlite_vec.rs::SqliteVecBackend`（`Connection::open_in_memory` + `vec0` 虚表）+ `core/src/retriever/vector/types.rs::VectorIndexConfig::persistence_path`（已有字段，当前恒 `None`）→ `core/src/server.rs:293-314`（`[SPEC-DEFER:phase-future.hnsw-graph-persistence]` 注释 marker + 按需重建语义路径）→ AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5，第十四次激活）→ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造凭据红线）→ `docs/decisions/adr-008-core-library-selection.md`（依赖选型）。
>
> **ADR 影响面（已识别）**：
> - **ADR-028 vector-persistence-strategy（新，Proposed）**：记 hnsw 图持久化格式（序列化往返 + rebuild-on-load fallback）+ sqlite-vec Windows MSVC 跨平台调查结论（落地或诚实文档化阻断）+ 向量增量索引评估口径。落地后据真实非合成往返 / 真实跨平台构建结果 ratify（ADR-013）。
> - 触及 **ADR-023（vector-backend-default）**：D2 hnsw fallback 的「rebuild-on-restart」前提与 Follow-ups 三 marker（`hnsw-graph-persistence` / `sqlite-vec-cross-platform`）由本 phase 推进——以 add-only Amendment 记录推进结果，不溯改 ADR-023 正文 D1-D6（D5）。
> - 触及 **ADR-008（core-library-selection）**：若 sqlite-vec 跨平台引入替代绑定 / 预编译依赖，按 add-only Amendment 记录（不溯改既有 D 段）。

## 1. 阶段目标

v0.16.0 ship 后，ContextForge 的向量检索具备**持久化能力的 hnsw fallback backend**（`vector-hnsw` feature 下图可序列化到磁盘并在重启后反序列化加载，加载失败时 rebuild-on-load 兜底），并对 **sqlite-vec Windows MSVC 跨平台**给出真实调查结论（可构建路径落地，或在确证受阻时诚实文档化 stop-condition 承 Phase 18 既有结论，不伪造跨平台通过），同时对**向量增量索引**完成评估（最小实现或如实延后）。默认构建仍 0 vector 依赖（ADR-023 D5）、BM25-only baseline 行为不变。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. `vector-hnsw` feature 下 `HnswBackend` 图可序列化到磁盘 + 反序列化加载 + 加载失败时 rebuild-on-load fallback；序列化往返在 feature 下 `cargo test` 可断言（索引→存盘→重载→search 命中等价），默认构建 0 新依赖不退化（AC1）
2. sqlite-vec Windows MSVC 跨平台调查给出真实结论：可构建路径（bundled C amalgamation / 预编译 / 替代绑定之一）落地并在 Windows MSVC 构建通过；或确证仍受阻时诚实文档化 stop-condition（承 `docs/spikes/phase-18-sqlite-vec.md` 既有凭据），按 ADR-013 不伪造跨平台通过（AC2）
3. 向量增量索引评估完成：最小增量实现（单 chunk 追加 / 删除不触发全量 reindex）落地并 deterministic 单测可断言，或确证依赖未明时如实延后并文档化评估口径 `[SPEC-DEFER:phase-future.vector-incremental-index]`（AC3）
4. v0.16.0 release docs + phase §6 闭合 + ADR-028 据真实非合成结果 ratify 或记录维持（AC4）
5. ADR-014 D1-D5（第十四次激活）全通过（AC5）

**v0.x 版本号决策**：v0.16.0 minor release（向量持久化与跨平台债收口；默认构建仍 BM25-only baseline + 0 vector 依赖——持久化 / 增量索引能力均在 feature 下，add-only 不破坏既有客户端）。

## 2. 业务价值

直接推进 ADR-023 Follow-ups 三个 marker 与 Phase 18/19 记录的两块向量持久化 / 跨平台债：

- **hnsw 图持久化**：ADR-023 D2 把 hnsw 定为跨平台 / dev / 小语料 fallback，但明记其「in-memory-only model（rebuild on restart）」与 100k 28.4s 建图为大索引 disqualifying（`adr-023:55-60`）。本 phase 让 `vector-hnsw` 下图可存盘 + 重载，消除重启重建成本，使 hnsw fallback 在中等语料下可用于持久部署（`[SPEC-DEFER:phase-future.hnsw-graph-persistence]`）。
- **sqlite-vec 跨平台**：ADR-023 D1 把 sqlite-vec 定为生产嵌入式推荐默认，但 `docs/spikes/phase-18-sqlite-vec.md` / `docs/releases/v0.11.0-evidence.md` 记录其 Windows MSVC 构建受阻（凭据保留）。本 phase 真实调查 MSVC 可构建路径，落地或诚实定论，缩小 dev/prod backend parity 缺口（`[SPEC-DEFER:phase-future.sqlite-vec-cross-platform]`）。
- **向量增量索引**：Phase 18/19 所有 backend 默认全量 reindex（`core/src/retriever/vector/{hnsw,brute_force,sqlite_vec}.rs` 的 `delete` 均 clear+rebuild 语义）。本 phase 评估单 chunk 追加 / 删除的增量路径，降低大语料重索引成本（`[SPEC-DEFER:phase-future.vector-incremental-index]`，承 `phase-19` §2）。
- **PRD §Constraints（跨平台 + 性能基线）**：dev on Windows MSVC / release on Linux 的 backend parity 与「1 万文件索引 <10min / 单文件增量 <5s」性能基线在向量路径上推进。

**不在本 phase scope**：

- Hybrid scoring（BM25 + Vector 融合）[SPEC-DEFER:phase-future.hybrid-scoring]——v0.14.0 / Phase 21
- Reranker（cross-encoder）[SPEC-DEFER:phase-future.reranker]——v0.14.0 / Phase 21
- Remote embedding provider（OpenAI / Cohere）[SPEC-DEFER:phase-future.embedding-provider-remote]——v0.15.0 / Phase 22
- Embedding 缓存 [SPEC-DEFER:phase-future.embedding-cache]——v0.15.0 / Phase 22
- qdrant server 生命周期 / 部署拓扑 [SPEC-DEFER:phase-future.qdrant-server-lifecycle] / [SPEC-DEFER:phase-future.qdrant-deployment-topology]——长尾 backlog
- lancedb index tuning / schema compaction [SPEC-DEFER:phase-future.lancedb-index-tuning] / [SPEC-DEFER:phase-future.lancedb-schema-compaction]——长尾 backlog

## 3. 涉及模块

### 23.1 hnsw 图持久化（task-23.1）

- 修改 `core/src/retriever/vector/hnsw.rs`——`HnswBackend` 加图序列化（`save` / `load` 到 `VectorIndexConfig.persistence_path`）+ 反序列化加载 + 加载失败时 rebuild-on-load fallback（重建语义承既有全量建图）
- 复用既有 `core/src/retriever/vector/types.rs::VectorIndexConfig::persistence_path`（已有字段，当前恒 `None`）作为存盘路径来源
- 同源 Rust tests（≥2，feature `vector-hnsw` 下：序列化往返 roundtrip——index→save→新实例 load→search 命中等价 + 加载失败 rebuild-on-load fallback 路径）
- `core/Cargo.toml`——`vector-hnsw` feature 如需序列化依赖（如 `instant-distance` 序列化能力 / serde 绑定）按 add-only 评估（R7 经主 agent）

### 23.2 sqlite-vec 跨平台调查（task-23.2）

- 调查 + 修改 `core/Cargo.toml` `vector-sqlite` feature / `core/src/retriever/vector/sqlite_vec.rs`——尝试 Windows MSVC 可构建路径（bundled C amalgamation / 预编译二进制 / 替代绑定之一）
- 新增 `docs/spikes/phase-23-sqlite-vec-cross-platform.md`——记录调查方法 + 真实构建结果（Linux gcc 既有可构建 + Windows MSVC 真实尝试结论），ADR-013 三态如实标（构建通过 / 确证受阻 stop-condition / 部分平台）
- 若可构建：feature 下 Windows MSVC `cargo build` 通过 + 既有 `vector-sqlite` 单测不退化；若受阻：诚实文档化 stop-condition（承 `docs/spikes/phase-18-sqlite-vec.md` 既有凭据），不伪造跨平台通过
- 同源 Rust test（≥1：feature `vector-sqlite` 下既有 sqlite-vec backend 行为不退化的契约测试，Linux 可跑）

### 23.3 向量增量索引 + closeout（task-23.3）

- 评估 + 修改 `core/src/retriever/vector/`（最小增量实现）或文档化评估——单 chunk 追加 / 删除不触发全量 reindex 的增量路径（`[SPEC-DEFER:phase-future.vector-incremental-index]`，最小实现或如实延后）
- 修改 `scripts/console_smoke.sh`——v13：向量持久化 / 跨平台相关 smoke 断言（feature 下 hnsw 持久化往返 smoke 或如实标注受阻），既有 step 不退化
- 新增 `docs/releases/v0.16.0-{evidence,artifacts}.md` + `README.md` v0.16 段 + `RELEASE_NOTES.md` v0.16.0 段
- 修改 `docs/decisions/adr-028-vector-persistence-strategy.md`——据真实结果 Proposed→Accepted 或记录维持 + ADR-023/008 add-only Amendment（推进结果记录，不溯改正文，D5）
- 修改 `docs/s2v-adapter.md`（Phase 23 Draft→Done + Tasks 0→3；ADR-028 状态；ADR-023 Follow-ups 推进记录）

### BDD feature

- 新增 `test/features/phase-23-vector-persistence-and-cross-platform.feature`（≥3 scenario：hnsw 图持久化往返 / sqlite-vec 跨平台调查结论 / 向量增量索引评估）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 23.1 | `core/src/retriever/vector/hnsw.rs` `HnswBackend` 图序列化/反序列化 + rebuild-on-load fallback + roundtrip 测试 | `../tasks/task-23.1-hnsw-graph-persistence.md` |
| 23.2 | `core/Cargo.toml` `vector-sqlite` + `core/src/retriever/vector/sqlite_vec.rs` Windows MSVC 可构建路径调查 + `docs/spikes/phase-23-sqlite-vec-cross-platform.md` | `../tasks/task-23.2-sqlite-vec-cross-platform.md` |
| 23.3 | 向量增量索引评估（最小实现或如实延后）+ smoke v13 + v0.16.0 closeout + ADR-028 ratify | `../tasks/task-23.3-closeout-v0.16.0.md` |

## 5. 依赖关系

- **task-23.1**（hnsw 图持久化）dep Phase 18 task-18.6（`HnswBackend` + `vector-hnsw` feature 已落地）+ `VectorIndexConfig.persistence_path`（task-18.1 已有字段）；可与 23.2 并行（写路径不相交：hnsw.rs vs sqlite_vec.rs/Cargo.toml）。
- **task-23.2**（sqlite-vec 跨平台）dep Phase 18 task-18.3（`SqliteVecBackend` + `vector-sqlite` feature 已落地，Linux gcc 可构建凭据）；调查类任务，结论可能为「落地」或「诚实文档化 stop-condition」。
- **task-23.3**（closeout）dep 23.1 + 23.2 全 Done；向量增量索引评估为本 task 子项。
- 外部：ADR-028（本 phase 新 Proposed）/ ADR-023（vector-backend-default，本 phase 推进其 Follow-ups 三 marker，add-only Amendment）/ ADR-008（core-library-selection，依赖变更 add-only）/ ADR-014 第十四次激活 / ADR-013（禁伪造跨平台凭据）/ Phase 19 语义路径（`Retriever::search_semantic` 已落地，向量 backend 接入热路径）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（每条 AC 含 ADR-014 D3 verified by 显式 owner；Draft 阶段未勾选，实施后逐条置 `[x]`）**：

- [x] **AC1**：`vector-hnsw` feature 下 `HnswBackend` 图序列化到磁盘 + 反序列化加载 + 加载失败 rebuild-on-load fallback；序列化往返在 feature `cargo test` 可断言（index→save→重载→search 命中等价），默认构建 0 新依赖不退化 — verified by task-23.1 §6 AC1-3 + phase-smoke step 1
- [x] **AC2**：sqlite-vec Windows MSVC 跨平台调查给出真实结论——可构建路径落地并 MSVC 构建通过，或确证受阻时诚实文档化 stop-condition（承 Phase 18 既有凭据，禁伪造跨平台通过，ADR-013）— verified by task-23.2 §6 AC1-2 + phase-smoke step 2
- [x] **AC3**：向量增量索引评估完成——最小增量实现 deterministic 单测可断言，或确证依赖未明时如实延后并文档化评估口径（`[SPEC-DEFER:phase-future.vector-incremental-index]`）— verified by task-23.3 §6 AC1 + phase-smoke step 3
- [x] **AC4**：v0.16.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ ADR-028 据真实非合成结果 ratify 或记录维持 + phase §6 闭合 — verified by task-23.3 §6 AC2-3
- [x] **AC5**：ADR-014 cross-validation gate 全套通过（第十四次激活）— D1 mapping + D2 lint `--touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-22 不溯改 — verified by task-23.3 closeout PR body

**端到端 smoke（C1 集成兜底）**：(1) `vector-hnsw` feature 下 hnsw 图持久化往返 roundtrip；(2) sqlite-vec 跨平台调查结论（构建通过 / 受阻如实标）；(3) 向量增量索引评估结论（最小实现 smoke 或如实延后）全 PASS。

## 7. 阶段级风险

- **R1（中）hnsw 序列化能力依赖第三方 crate 支持**：`instant-distance` 0.6 是否暴露 `HnswMap` 序列化（serde 派生 / 自定义格式）需先核实；若不支持则需自定义图格式或 fallback 到「存 embedding + 重载时重建」。
  - **缓解**：task-23.1 先核实 `instant-distance` 序列化面；若 crate 不支持原生序列化，则持久化「输入 embedding + id」并在 load 时重建图（rebuild-on-load 即此兜底语义），仍消除「冷启动从 SQLite 重新枚举 + 重 embed」成本。stop-condition：若序列化与重建均不可行则记录受阻态，AC1 不标 `[x]`。
- **R2（高）sqlite-vec Windows MSVC 仍受阻**：Phase 18 已记录 MSVC 构建阻断（`docs/spikes/phase-18-sqlite-vec.md` 凭据保留）；调查可能确证仍受阻。
  - **缓解**：task-23.2 真实尝试 bundled / 预编译 / 替代绑定三路径；任一通过即落地，全部受阻则诚实文档化 stop-condition（承既有结论），按 ADR-013 不伪造跨平台通过——AC2 在「确证受阻」态下以「真实调查 + stop-condition 文档」满足，不标伪造 `[x]`。本 phase 不因 sqlite-vec 受阻阻塞 23.1 / 23.3（hnsw fallback 与增量索引独立推进）。
- **R3（中）向量增量索引依赖 backend 能力差异**：brute-force / hnsw 全量建图语义下增量追加需重建；sqlite-vec `vec0` 支持行级 insert/delete。
  - **缓解**：task-23.3 评估各 backend 增量可行性；最小实现优先在支持行级增量的 backend（如 sqlite-vec / brute-force 追加）落地 deterministic 单测；建图类 backend（hnsw）增量受 crate 限制则如实延后 `[SPEC-DEFER:phase-future.vector-incremental-index]`，AC3 以「评估完成 + 最小实现或诚实延后」满足。

## 8. Definition of Done

- 3 task spec（23.1-23.3）顶部 `**Status**: Done`
- §6 阶段级 AC1-5 全 `[x]`（受阻态按 ADR-013 如实记录，不伪造）
- 端到端 smoke 3 step 全 PASS（含受阻态如实标注）
- **ADR**：ADR-028 `Proposed → Accepted`（据真实非合成往返 / 真实跨平台构建结果）或据实测记录维持 + 文档化；ADR-023 / ADR-008 add-only Amendment 记录推进结果（不溯改正文，D5）
- **adapter**：§Phase 索引 Phase 23 `Draft → Done` + `Tasks 0 → 3`；§ADR 索引 ADR-028；§BDD 追加 phase-23 feature 行；ADR-023 Follow-ups 三 marker 推进记录
- **spike evidence**：`docs/spikes/phase-23-sqlite-vec-cross-platform.md`（跨平台调查）+ hnsw 持久化往返证据（task-23.1 spike 或 spec §10）
- **release**：`docs/releases/v0.16.0-{evidence,artifacts}.md` + `RELEASE_NOTES.md` v0.16 段 + README v0.16 段
- **follow-up**：向量增量索引若延后则 `[SPEC-DEFER:phase-future.vector-incremental-index]` 留 backlog；qdrant / lancedb 细化项继续 backlog
