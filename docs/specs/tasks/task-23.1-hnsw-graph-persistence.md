# Task `23.1`: `hnsw-graph-persistence — core/src/retriever/vector/hnsw.rs HnswBackend 图序列化/反序列化到磁盘（VectorIndexConfig.persistence_path）+ 加载失败时 rebuild-on-load fallback + feature vector-hnsw 下序列化往返 roundtrip 测试`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 23 (vector-persistence-and-cross-platform)
**Dependencies**: task-18.6（`HnswBackend` via `instant-distance` + `vector-hnsw` feature 已落地，全量建图语义）/ task-18.1（`VectorIndexConfig.persistence_path` 字段 + 三 trait freeze）/ ADR-023 D2（hnsw 跨平台 fallback + 「rebuild-on-restart」前提）/ ADR-008（core-library-selection，依赖变更 add-only）/ ADR-013（禁伪造凭据）/ ADR-014 D1-D5（第十四次激活）

## 1. Background

Phase 18 task-18.6 用 `instant-distance`（纯 Rust HNSW，`core/src/retriever/vector/hnsw.rs`）实现 `HnswBackend`：`index_batch` 把 `(normalize(embedding), chunk_id)` 累积进 `pending`，`flush` 用 `Builder::default().build(points, values)` 一次性建整图存进 `map: Mutex<Option<HnswMap<HnswPoint, String>>>`（全量建图语义，无增量插入）。ADR-023 D2 把 hnsw 定为跨平台 / dev / 小语料 fallback，但 `adr-023:55-60` 明记其 disqualifying 项之一是「**in-memory-only model（rebuild on restart）**」+ 100k 图构建实测 28.4s（`docs/spikes/phase-18-hnsw.md`），并把 hnsw 图持久化列为 Follow-up（`[SPEC-DEFER:phase-future.hnsw-graph-persistence]`）。

`core/src/server.rs:293-296` 的语义路径注释亦标记该 marker：semantic 路径当前对每次请求「按需从 SQLite 枚举 chunk + 重新 embed + 重建 hnsw/brute-force 索引」（`no persistence yet — [SPEC-DEFER:phase-future.hnsw-graph-persistence]`）。`core/src/retriever/vector/types.rs::VectorIndexConfig` 已有 `persistence_path: Option<PathBuf>` 字段（task-18.1 预留），但当前 `Retriever::index_chunks_semantic`（`core/src/retriever/mod.rs:625`）构造 config 时恒填 `None`——持久化 seam 已留但未接通。

本 task 让 `vector-hnsw` feature 下 `HnswBackend` 的图可序列化到磁盘、重启后反序列化加载，并在加载失败（文件缺失 / 格式不兼容 / 损坏）时 rebuild-on-load 兜底，消除「每次重启从零枚举 + 重 embed + 重建图」成本。

## 2. Goal

`core/src/retriever/vector/hnsw.rs` 的 `HnswBackend` 在 `vector-hnsw` feature 下新增持久化能力：(a) `save(path)`——把已建图（或其重建所需的 `(embedding, chunk_id)` 输入集）序列化到磁盘；(b) `load(path)`——反序列化加载已存图，使 `search` 无需重建即命中等价结果；(c) 加载失败时 rebuild-on-load fallback——`load` 在文件缺失 / 格式不兼容 / 损坏时返回可识别状态，调用方据此走全量重建（既有 `flush` 建图语义），不 panic、不静默吞错。持久化路径来源为 `VectorIndexConfig.persistence_path`（既有字段）；`None` 时维持现状（纯内存，行为不变）。≥2 Rust 测试（feature `vector-hnsw` 下）全 PASS：序列化往返 roundtrip（index→save→新实例 load→search 命中等价）+ 加载失败 rebuild-on-load fallback 路径。默认构建（无 `vector-hnsw`）0 新依赖、行为不变；`cargo test --workspace` 不退化。D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **修改 `core/src/retriever/vector/hnsw.rs`**：`HnswBackend` 加持久化方法（`save` / `load` 或等价 `persist` / `restore`），把图（或重建所需的 `(unit-normalized embedding, chunk_id)` 输入集）序列化到 `VectorIndexConfig.persistence_path` 指向的磁盘文件，并支持反序列化加载；加载失败（文件缺失 / 格式版本不兼容 / 损坏）返回 `Result` 错误或可识别空态，使调用方走全量重建 fallback（复用既有 `flush` 全量建图语义）。
- **接通持久化路径**：`HnswBackend::open`（或 index 生命周期）读 `VectorIndexConfig.persistence_path`——`Some(path)` 时尝试 load，失败则 rebuild-on-load；`None` 时维持纯内存现状（向后兼容）。
- **新增同源 Rust 单测（`core/src/retriever/vector/hnsw.rs` 内 `#[cfg(test)] mod tests` 或 `core/tests/`，feature `vector-hnsw` gated）**：(a) 序列化往返——`HnswBackend` index 一组确定 `(embedding, chunk_id)` → save 到临时路径 → 新 `HnswBackend` 实例 load 同路径 → `search(query, k)` 命中与原实例等价的 chunk_id 序；(b) 加载失败 rebuild-on-load fallback——load 缺失 / 损坏文件路径 → 触发全量重建 → search 仍正确（不 panic）。
- **可选修改 `core/Cargo.toml`**：`vector-hnsw` feature 若需序列化依赖（`instant-distance` 序列化能力 / serde 绑定 / `bincode` 等编码 crate）——按 add-only 评估，依赖变更经主 agent R7 chore（subagent 不自改 Cargo.toml）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **`HnswBackend` 全量建图 + `instant-distance` 集成本体** [SPEC-OWNER:task-18.6-spike-hnsw]：本 task 在其上加持久化，不重写建图。
- **sqlite-vec / brute-force / qdrant / lancedb 的持久化** [SPEC-OWNER:task-23.2-sqlite-vec-cross-platform]（sqlite-vec 跨平台）/ sqlite-vec 已天然磁盘持久（`vec0` on-disk，ADR-023 D1）：本 task 仅做 hnsw 图持久化。
- **向量增量索引（单 chunk 追加/删除不全量重建）** [SPEC-DEFER:phase-future.vector-incremental-index]：本 task 是图持久化往返，增量索引评估在 task-23.3。
- **把持久化接进 `core/src/server.rs` 语义热路径（重启复用已存图替代按需重建）** [SPEC-OWNER:task-23.3-closeout-v0.16.0]：本 task 落 backend 层持久化能力 + 单测；热路径接入 / smoke 在收口 task 据实评估。
- **持久化格式跨版本迁移 / schema 演进** [SPEC-DEFER:phase-future.vector-incremental-index]：本 task 仅做单版本格式往返 + 不兼容时 rebuild-on-load 兜底。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`core/src/retriever/vector/hnsw.rs::HnswBackend`**：task-18.6 hnsw backend，本 task 加图序列化/反序列化 + rebuild-on-load。
- **`core/src/retriever/vector/types.rs::VectorIndexConfig::persistence_path`**：task-18.1 预留的持久化路径字段，本 task 首次消费。
- **`instant-distance::HnswMap`**：图数据结构，本 task 核实其序列化面（原生 serde / 重建输入持久化二选一）。
- **下游 task-23.3**：closeout 据本 task 持久化能力评估是否接进 `server.rs` 语义热路径 + smoke v13。

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/retriever/vector/hnsw.rs`（`HnswBackend` / `pending` / `map: Mutex<Option<HnswMap<HnswPoint, String>>>` / `flush` 全量建图 / `index_batch` / `search`）
- `core/src/retriever/vector/types.rs::VectorIndexConfig`（`persistence_path: Option<PathBuf>` 既有字段）+ `VectorChunk` / `VectorHit`
- `core/src/retriever/vector/traits.rs`（`VectorBackend` / `VectorIndexer` / `VectorSearcher` 三 trait 接口）
- `core/src/retriever/mod.rs:592-634`（`index_chunks_semantic` 当前 `persistence_path: None` 构造点）
- `core/src/server.rs:293-314`（语义路径按需重建 + `[SPEC-DEFER:phase-future.hnsw-graph-persistence]` marker）
- `docs/spikes/phase-18-hnsw.md`（100k 28.4s 建图实测 + in-mem-only 模型）+ `docs/decisions/adr-023-vector-backend-default.md` D2 + Follow-ups
- `instant-distance` 0.6 crate 文档（核实 `HnswMap` 是否暴露序列化 / serde 派生）+ `docs/decisions/adr-008-core-library-selection.md`（依赖选型 add-only）+ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造）

### 5.2 关键设计 — 图持久化 + rebuild-on-load fallback

- **持久化策略二选一（task-23.1 先核实 `instant-distance` 序列化面）**：
  - **路径 A（原生序列化）**：若 `HnswMap` 暴露 serde 派生 / 可序列化，直接把已建图编码到磁盘（如 `bincode` / `serde_json`），load 时反序列化恢复完整图——重载后 `search` 无需重建。
  - **路径 B（重建输入持久化，确定性 identity 兜底）**：若 `HnswMap` 不暴露原生序列化，则持久化 `(unit-normalized embedding, chunk_id)` 输入集（已是 `pending` 内容），load 时反序列化输入并复用 `flush` 全量建图——仍消除「从 SQLite 重新枚举 + 重 embed」成本（embed 是 hnsw 路径外的昂贵步骤）。
- **rebuild-on-load fallback**：`load(path)` 在文件缺失 / 格式版本不兼容 / 反序列化失败时返回可识别状态（`Result` 错误或 `Ok(None)` 空态），调用方据此走全量重建（既有 `flush` 语义），不 panic、不静默成功。格式带版本头，跨版本不兼容时归入 rebuild-on-load 路径。
- **`persistence_path` 语义**：`Some(path)` → 启用持久化（open 时尝试 load + rebuild-on-load 兜底，flush 后 save）；`None` → 纯内存现状（行为逐字节不变，向后兼容）。
- **ADR-013**：持久化往返断言「重载后 search 命中等价」是 deterministic feature 测试可验证项（🟡 feature 下真实持久化往返）；不预判跨语料召回数值。

### 5.3 不变量

- 默认构建（无 `vector-hnsw` feature）0 新依赖、`HnswBackend` 不编译、行为逐字节不变（ADR-023 D5）。
- `persistence_path: None` 时 hnsw 路径与现状等价（纯内存全量建图）。
- 持久化往返语义：相同输入 index → save → load → 同 query search → 命中等价 chunk_id 序（确定性，hnsw 近邻在固定输入下稳定）。
- rebuild-on-load 不静默吞错：load 失败可被调用方识别并触发重建，不伪造「加载成功」。
- 不改三 trait 签名（`VectorBackend` / `VectorIndexer` / `VectorSearcher`）——持久化方法为 `HnswBackend` inherent method 或经既有 `open`/`flush` 生命周期接入，不破坏 task-18.1 trait freeze。

## 6. Acceptance Criteria

- [ ] **AC1**: feature `vector-hnsw` 下 `HnswBackend` 序列化往返——index 一组确定 `(embedding, chunk_id)` → save 到磁盘 → 新实例 load 同路径 → `search(query, k)` 命中与原实例等价的 chunk_id 序（🟡 feature 下真实持久化往返）— verified by **TEST-23.1.1**
- [ ] **AC2**: 加载失败 rebuild-on-load fallback——`load` 在文件缺失 / 损坏 / 格式不兼容时返回可识别状态，调用方走全量重建，`search` 仍正确且不 panic（不静默吞错）— verified by **TEST-23.1.2**
- [ ] **AC3**: `persistence_path: None` 时 hnsw 路径与现状等价（纯内存全量建图，行为不变）；持久化方法不破坏 task-18.1 三 trait 签名 — verified by **TEST-23.1.3**
- [ ] **AC4**: 既有不退化 — 默认 `cargo test --workspace`（无 vector feature）全 PASS + 0 新依赖；`cargo test --workspace --features vector-hnsw` 既有 hnsw 测试不退化；`go test ./...` 不受影响（本 PR 零 Go delta）— verified by **TEST-23.1.4** + §10 实测
- [ ] **AC5**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-23.1.5** + §10 记录

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-23.1.1 | feature `vector-hnsw` 序列化往返 index→save→load→search 命中等价 | `core/src/retriever/vector/hnsw.rs`（`mod tests`）或 `core/tests/` | Planned |
| TEST-23.1.2 | 加载失败 rebuild-on-load fallback 路径 search 正确不 panic | `core/src/retriever/vector/hnsw.rs`（`mod tests`）或 `core/tests/` | Planned |
| TEST-23.1.3 | `persistence_path: None` 纯内存等价 + 不破坏三 trait 签名 | `core/src/retriever/vector/hnsw.rs`（`mod tests`） | Planned |
| TEST-23.1.4 | 默认 `cargo test --workspace` 0 failed + `--features vector-hnsw` 不退化 | 全 Rust | Planned |
| TEST-23.1.5 | D2 lint `--touched origin/master` 0 未标注命中 | `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（中）`instant-distance` 不暴露 `HnswMap` 原生序列化**（承 phase-23 §7 R1）：crate 可能不提供 serde 派生。
  - **缓解**：先核实 `instant-distance` 0.6 序列化面；不支持则走路径 B（持久化 `(embedding, chunk_id)` 输入集 + load 时复用 `flush` 重建）——仍消除昂贵的 SQLite 枚举 + 重 embed。stop-condition：若原生序列化与输入持久化均不可行，记录受阻态，AC1 不标 `[x]`（ADR-013 不伪造往返通过）。
- **R2（低）序列化依赖引入新供应链表面**（如 `bincode`）：default build 须 0 新依赖。
  - **缓解**：序列化 crate 仅在 `vector-hnsw` optional feature 下引入（承 task-18.x optional dep pattern，default features 0 新 dep）；依赖变更经主 agent R7 chore，subagent 不自改 Cargo.toml。
- **R3（低）持久化格式跨版本不兼容导致静默错配**：旧格式文件被新代码错误反序列化。
  - **缓解**：格式带版本头；版本不匹配 / 反序列化失败归入 rebuild-on-load 路径（重建而非静默错配），AC2 覆盖该路径。

## 9. Verification Plan

```bash
# Rust：默认构建（无 vector feature）0 新依赖 + 不退化
cargo test --workspace

# feature 下持久化往返 + rebuild-on-load fallback（vector-hnsw）
cargo test --workspace --features vector-hnsw

# Go 不退化（本 PR 零 Go delta）
go test ./...

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **Status**: 待实施（Draft）。实施完成后按以下 6 项回填：完成日期 / 改动文件 / commit 列表 / §9 Verification 结果 / 设计取舍（路径 A 原生序列化 vs 路径 B 重建输入持久化的实际选择 + `instant-distance` 序列化面核实结论）/ 剩余风险 + 下游影响（是否接进 server.rs 热路径由 task-23.3 评估）。
