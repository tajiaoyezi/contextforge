# Task `43.1`: `indexing-replay-splice — core/src/data_plane/indexing_events.rs add list_since(limit, since_ts)（since_ts 时序过滤镜像 replay_events_from_audit）+ DataPlaneStores add indexing_event_store: Option<Arc<SqliteIndexingEventStore>> 字段 + full() 加第 10 参数（既有 constructor 补 None byte-equiv）+ server.rs serve_full 传入 Some(indexing_event_store.clone())（store 已在 :756 构造）+ events.rs subscribe replay 段 splice indexing replay（since_ts>0 时 list_since + indexing_rows_to_pb_events，audit replay 后、live forward 前；store None / lock 失败 unwrap_or_default 空）；0 新 dep / 0 schema migration（复用 Phase 33 migration 0019）/ 0 proto 改动；默认 byte-equiv（since_ts<=0 / store=None 两条退化路径）`

**Status**: Ready

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 43 (governance-debt-cleanup-4)
**Dependencies**: 既有 `core/src/data_plane/indexing_events.rs`（`SqliteIndexingEventStore` task-33.3 已在 / `list(limit)` :111 缺 since_ts 本 task 加 list_since 不改 list / `append` :92 写路径已在 / `IndexingEventRow` :34 / `test_33_3_2` :172 store round-trip 镜像源）/ 既有 `core/src/data_plane/mod.rs`（`DataPlaneStores` :43-74 字段列表缺 indexing_event_store / `full()` :156-178 constructor 本 task 加第 10 参数 / `new`/`with_eval`/`with_memory`/`with_runner`/`with_runner_and_bus` 既有 constructor 本 task 补 None）/ 既有 `core/src/server.rs`（`serve_full` :756-762 indexing_event_store 局部构造 + 传 IndexSessionBackend 写路径已在 / `:788-798 DataPlaneStores::full()` 9 参数本 task 加第 10）/ 既有 `core/src/data_plane/events.rs`（`subscribe` :223-308 replay 段 :241-250 只接 audit 本 task splice indexing / `replay_events_from_audit` :394-423 since_ts 范式镜像源 / `indexing_rows_to_pb_events` :438-474 mapper 已写好本 task 调用）/ ADR-048（indexing-replay-splice，本 task 即其 D1/D2/D3 原文实现）/ ADR-038（governance-debt-cleanup-2 D3，indexing-replay-e2e splice 维度兑现 add-only Amendment @ task-43.3）/ ADR-031（observability-hardening，replay 范式源 task-26.2 引用）/ ADR-021（memory-event-bridge，audit replay splice 镜像源引用）/ ADR-004（默认 byte-equiv + 既有契约不变）/ ADR-008（dep add-only，Phase 43 = 0 新 dep）/ ADR-013（禁伪造红线——splice 真实接入非合成、since_ts 时序单测守护；live daemon e2e 🟡 honest-defer 不预填）/ ADR-012 / ADR-014 D1-D5（第三十四次激活）

## 1. Background

Phase 33 task-33.3（ADR-038 D3）交付了 indexing event 的**持久化**（add-only migration `0019_indexing_events` + `SqliteIndexingEventStore` + 4 emit 点 best-effort 持久写）+ **replay mapper**（`indexing_rows_to_pb_events`，`events.rs:438`，真实 job_id/processed/total 取持久行不合成，`test_33_3_2` 守护）。但 mapper **从未在 live subscribe 路径被调用**——4 个拼接缺口（grounding 已亲自核实）：

- **B1 `list(limit)` 缺 since_ts（真实，决定方案）**：`SqliteIndexingEventStore::list(limit)`（`indexing_events.rs:111`）只接受 `limit`，SQL 是无条件 `SELECT ... ORDER BY id ASC LIMIT ?1`（:115-116）。而 replay 须按 since_ts 过滤——镜像 `replay_events_from_audit`（`events.rs:394-423`）的 `ts < since_ts → skip`（:401-403）。缺 since_ts 参数 = replay 无法只取"自 since_ts 起 missed"的事件。
- **B2 `DataPlaneStores` 无 indexing_event_store 字段（真实）**：`DataPlaneStores`（`mod.rs:43-74`）字段列表有 `workspace_store`/`job_store`/`job_runner`/`data_dir`/`event_bus`/`memory`/`audit`/`eval`/`trace_persist` 9 个，**无** `indexing_event_store`。events subscribe 路径经 `self.stores` 读 store，无字段 = 不可达。
- **B3 `serve_full` 未传入 store（真实）**：`serve_full`（`server.rs:756-762`）**局部已构造** `indexing_event_store` 并传给 `IndexSessionBackend::with_event_bus_and_indexing_store`（写路径 OK），但 `DataPlaneStores::full()`（`server.rs:788-798`，9 参数）**未传入**该 store（读路径不可达）。
- **B4 subscribe replay 只接 audit（真实）**：`EventsServer::subscribe`（`events.rs:223-308`）的 replay 段（`:241-250`）：`if req.since_ts > 0 { self.stores.audit.as_ref()...map(|entries| replay_events_from_audit(&entries, req.since_ts))... }`——只 splice 了 memory audit replay，**未接 indexing replay**。mapper `indexing_rows_to_pb_events`（:438）写好了却从不在 live 路径调用。

**B5 audit replay since_ts 范式是镜像源（真实）**：`replay_events_from_audit`（`:394-423`）已建立 since_ts 过滤（`ts < since_ts → skip` :401-403）+ best-effort（`audit.as_ref().and_then(...).unwrap_or_default()` :242-247）+ subscribe-first（`:235 subscribe_all()` 在 replay 构造前保证不丢 live）模式。本 task 镜像此模式接 indexing replay。

本 task 补 4 缺口：`list_since(limit, since_ts)` + `DataPlaneStores` 加字段 + `full()` 加参数 + `serve_full` 传入 + `subscribe` splice indexing replay。code-local 🟢 可单测，0 新 dep + 0 schema migration（复用 0019）+ 0 proto 改动。

## 2. Goal

(1) **B1 list_since**：`core/src/data_plane/indexing_events.rs` add `pub fn list_since(&self, limit: usize, since_ts: i64) -> Result<Vec<IndexingEventRow>, IndexingEventStoreError>`：
   - `since_ts > 0` 时 `SELECT ... WHERE ts_unix >= ?since ORDER BY id ASC LIMIT ?limit`（镜像 `replay_events_from_audit` 的 `ts < since_ts → skip` 语义，含等号边界——`>=` 取 since_ts 当刻及之后）
   - `since_ts <= 0` 时不过滤（`SELECT ... ORDER BY id ASC LIMIT ?limit`，与既有 `list()` 行为一致）
   - 既有 `list(limit)` 保留不动（其他调用方不破）
(2) **B2 DataPlaneStores 字段**：`core/src/data_plane/mod.rs` `DataPlaneStores` add `pub indexing_event_store: Option<Arc<crate::data_plane::indexing_events::SqliteIndexingEventStore>>` 字段。
(3) **B2 full() 加参数 + 既有 constructor 补 None**：`full()` constructor（:156-178）加第 10 参数 `indexing_event_store: Option<Arc<...>>`；所有既有 constructor（`new`/`with_eval`/`with_memory`/`with_runner`/`with_runner_and_bus`）补 `indexing_event_store: None`（byte-equiv，既有调用方不接 indexing replay）。
(4) **B3 serve_full 传入**：`core/src/server.rs` `serve_full` `DataPlaneStores::full(...)` 第 10 参数传 `Some(indexing_event_store.clone())`（store 已在 `:756` 构造，clone 进 DataPlaneStores 读路径；写路径 `IndexSessionBackend` 仍持原 Arc）。
(5) **B4 subscribe splice**：`core/src/data_plane/events.rs` `subscribe` replay 段（:241-250）after audit replay 加 indexing replay：`since_ts > 0` 时 `self.stores.indexing_event_store.as_ref().and_then(|s| s.list_since(REPLAY_LIMIT, req.since_ts).ok()).map(|rows| indexing_rows_to_pb_events(&rows)).unwrap_or_default()`；合并进 `replay: Vec<PbEvent>`（indexing 在前 / audit 在后，两类各 id ASC / ts ASC 内部有序；客户端按 event_id dedup）。store None / lock 失败 → `unwrap_or_default()` 空切片（best-effort 镜像 audit :245-247）。

pass bar：`list_since` since_ts 过滤 + id ASC 时序（TEST-43.1.1 🟢）；subscribe 带 since_ts → 先收到 indexing replay 再 audit replay 再 live + since_ts<=0 无 replay byte-equiv + store=None 退化（TEST-43.1.2 🟢）；0 新 dep（ADR-008）/ 0 schema migration（复用 0019）/ 0 proto 改动 / 默认 byte-equiv；既有 events / data_plane 单测不退化；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `core/src/data_plane/indexing_events.rs`——add `pub fn list_since(&self, limit: usize, since_ts: i64) -> Result<Vec<IndexingEventRow>, IndexingEventStoreError>`（since_ts>0 时 `WHERE ts_unix >= ?` ORDER BY id ASC LIMIT，since_ts<=0 不过滤）；既有 `list(limit)` 不动；TEST-43.1.1（list_since 时序过滤：append 3 行 ts=100/200/300 → since_ts=150 返 ts=200/300 两行 id ASC；since_ts<=0 返全量）
- 改 `core/src/data_plane/mod.rs`——`DataPlaneStores` add `pub indexing_event_store: Option<Arc<crate::data_plane::indexing_events::SqliteIndexingEventStore>>` 字段 + `full()` 加第 10 参数 + `new`/`with_eval`/`with_memory`/`with_runner`/`with_runner_and_bus` 补 `indexing_event_store: None`
- 改 `core/src/server.rs`——`serve_full` `DataPlaneStores::full(...)` 第 10 参数 `Some(indexing_event_store.clone())`
- 改 `core/src/data_plane/events.rs`——`subscribe` replay 段 after audit replay 加 indexing replay splice（since_ts>0 时 list_since + mapper，合并进 replay Vec）+ TEST-43.1.2（subscribe splice 时序 + 默认 byte-equiv）
- **不改**：既有 `list(limit)`（:111）/ migration 0019（已落地）/ `indexing_rows_to_pb_events` mapper（:438 已写好）/ proto（0 改动）/ `replay_events_from_audit`（:394 镜像源不改）/ 既有 audit replay splice 逻辑（:241-250 不改，仅在其后加 indexing）
- 同源测试：TEST-43.1.1（list_since 时序过滤）+ TEST-43.1.2（subscribe splice 时序 indexing→audit→live + since_ts<=0 byte-equiv + store=None 退化）

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- live daemon restart-then-replay 端到端 e2e（真起进程 + 跨 restart 双窗口断言，须 running daemon / console 跨进程）[SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]——本 task 交付 unit 级 splice + 时序单测
- memory-actor-all-rpc 四 RPC（Deprecate/SoftDelete 7 层改动 + 新 schema migration / HardDelete 须 audit 层重设计）[SPEC-DEFER:phase-future.memory-actor-all-rpc]——本 task 单聚焦 indexing-replay 不扩面
- REPLAY_LIMIT 提为可配置常量（现 inline 100，本 task 用具名常量，不增 env config）——若需 env config 另立 task
- 真实 release tag / run-id / digest（v0.36.0）[SPEC-OWNER:task-43.3-closeout]（ADR-012 用户授权后回填）

## 4. Actors

- 主 agent（ADR-012 自治）
- `SqliteIndexingEventStore`（`core/src/data_plane/indexing_events.rs`，task-33.3 已在——本 task 加 `list_since` 读 API）
- `DataPlaneStores`（`core/src/data_plane/mod.rs`，store 注入点——本 task 加 `indexing_event_store` 字段）
- `serve_full`（`core/src/server.rs:714`，生产 wiring——本 task 把已构造 store clone 进 DataPlaneStores）
- `EventsServer::subscribe`（`core/src/data_plane/events.rs:223`，replay splice 入口——本 task 加 indexing replay splice）
- `indexing_rows_to_pb_events`（`core/src/data_plane/events.rs:438`，task-33.3 mapper——本 task 首次在 live 路径调用）
- 订阅者（gRPC `SubscribeEventsRequest{since_ts}`——本 task 后 since_ts>0 可收到 indexing replay）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/data_plane/indexing_events.rs:76-135`（`SqliteIndexingEventStore` impl：`open`/`append`/`list(limit)`——本 task 加 `list_since` 不改 `list`）+ `:34-43`（`IndexingEventRow` 字段）+ `:172-206`（`test_33_3_2` store round-trip 镜像源）
- `core/src/data_plane/mod.rs:43-74`（`DataPlaneStores` 字段列表——本 task 加 `indexing_event_store`）+ `:156-178`（`full()` constructor——本 task 加第 10 参数）+ `:80-200`（既有 constructor 本 task 补 None）
- `core/src/server.rs:714-812`（`serve_full`：`:756-762` indexing_event_store 局部构造 + 传 IndexSessionBackend；`:788-798` DataPlaneStores::full() 9 参数——本 task 加第 10）
- `core/src/data_plane/events.rs:223-308`（`subscribe`：`:235` subscribe_all subscribe-first / `:241-250` replay 段只接 audit——本 task splice indexing / `:251-288` live forward spawn）+ `:394-423`（`replay_events_from_audit` since_ts 范式镜像源）+ `:438-474`（`indexing_rows_to_pb_events` mapper task-33.3 已写好——本 task 首次 live 调用）
- `docs/decisions/adr-038-governance-debt-cleanup-2.md` §D3（indexing event 持久化 + replay mapper + `[SPEC-DEFER:phase-future.indexing-replay-e2e]`，本 task 兑现 splice 维度 add-only Amendment @ task-43.3）+ `adr-048-indexing-replay-splice.md §D1/D2/D3`（本 task 即其原文实现）

### 5.2 关键设计 — splice 时序 + 默认 byte-equiv + best-effort（0 migration / 0 proto / since_ts 对齐 audit）

- **B1 list_since 时序过滤（镜像 audit since_ts）**：`list_since(limit, since_ts)` 的 `since_ts > 0` 时 `WHERE ts_unix >= ?since`（含等号边界——取 since_ts 当刻及之后；镜像 `replay_events_from_audit` 的 `ts < since_ts → skip`，即保留 `ts >= since_ts`）；`since_ts <= 0` 不过滤返全量 limit（与 `list()` 一致）。id ASC 内部有序（mapper 依赖）。既有 `list(limit)` 保留（`test_33_3_2` 等既有调用方不破）。
- **B2/B3 DataPlaneStores 接线（store 已在，clone 读路径）**：`indexing_event_store` 已在 `serve_full` `:756` 构造并传 `IndexSessionBackend`（写路径），本 task clone 一份进 `DataPlaneStores`（读路径 subscribe replay）。写路径 `IndexSessionBackend` 仍持原 Arc，读路径 DataPlaneStores 持 clone Arc——共享同一 SQLite 文件句柄（`SqliteIndexingEventStore` 内部 `Mutex<Connection>`，clone Arc 不复制 Connection）。既有 constructor 补 `None`（`new`/`with_eval`/`with_memory`/`with_runner`/`with_runner_and_bus`）byte-equiv（单测、非 serve_full 路径不接 indexing replay，退化到现状）。
- **B4 subscribe splice（audit 后、live 前）**：splice 严格在 audit replay 之后、live forward（`:251` spawn）之前。两类 replay 各 id ASC / ts ASC 内部有序；indexing `evt-idx-{id}` 与 audit `evt-audit-{id}` 命名空间独立，客户端按 event_id dedup splice 边界。subscribe-first（`:235 subscribe_all()` 在 replay 构造前）保证不丢 live 事件（镜像 task-26.2 既有模式）。store None / lock 失败 / 空 → `unwrap_or_default()` 空切片（best-effort 镜像 audit :245-247）。
- **默认 byte-equiv（两条退化路径）**：(1) `since_ts <= 0`（订阅首连无 since_ts）：indexing replay 返空（`req.since_ts > 0` 守护，与既有 audit replay :241 同分支）→ 行为与现状 byte-identical；(2) `indexing_event_store == None`（旧 constructor / 单测不设）：indexing replay 返空 → 退化到现状。仅 `serve_full` 生产路径把 store 传入新字段。

### 5.3 不变量

- 默认 byte-equiv（ADR-004）：`since_ts<=0` → 无 indexing replay（与现状 byte-identical）；`store=None` → 无 indexing replay（退化现状）。仅 `serve_full` 生产路径 `since_ts>0` + `store=Some` 时 indexing replay 生效。
- 0 schema migration（复用 Phase 33 0019）：`indexing_events` 表 schema 不变；`list_since` 复用既有 `ts_unix` 列。
- 0 新代码依赖（ADR-008）：`list_since` 纯 rusqlite（既有 dep）；splice 复用既有 `indexing_rows_to_pb_events` mapper。无 Cargo 依赖增量。
- 0 proto 改动：纯内部 read 路径 splice，无 proto field 增减。
- 0 网络：splice 是本地 subscribe 内部 read。
- subscribe-first 不丢 live（镜像 task-26.2）：`subscribe_all()` 在 replay 构造前。
- best-effort（镜像 audit）：store None / lock 失败 / 空 → 空切片，不阻断 subscribe。
- live daemon e2e 据实 honest-defer（ADR-013）：本 task 交付 unit 级 splice + 时序单测；live daemon restart-then-replay e2e `[SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]` 不预填。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [ ] **AC1**（list_since 时序过滤 🟢）: `SqliteIndexingEventStore::list_since(limit, since_ts)` since_ts>0 时 `WHERE ts_unix >= ?` ORDER BY id ASC LIMIT（镜像 audit），since_ts<=0 不过滤返全量；既有 `list(limit)` 不动 — verified by **TEST-43.1.1**
- [ ] **AC2**（subscribe splice 时序 + 默认 byte-equiv 🟢）: `DataPlaneStores` add `indexing_event_store` 字段 + `full()` 加参数（既有 constructor 补 None byte-equiv）+ `serve_full` 传入 + `subscribe` splice indexing replay（since_ts>0 时 list_since + mapper，audit 后、live 前）；subscribe 带 since_ts → 先 indexing replay 再 audit replay 再 live；since_ts<=0 无 replay byte-equiv；store=None 退化；0 新 dep / 0 migration / 0 proto — verified by **TEST-43.1.2**
- [ ] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-43.1.3**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-43.1.1 | `list_since` 时序过滤：append 3 行 ts=100/200/300（id ASC）→ `list_since(100, 150)` 返 ts=200/300 两行 id ASC（since_ts>0 过滤）；`list_since(100, 0)` / `list_since(100, -1)` 返全量 3 行（since_ts<=0 不过滤，与 `list()` 一致）；既有 `list(100)` 不受影响仍返全量 | `core/src/data_plane/indexing_events.rs`（同源 test） | Not Started |
| TEST-43.1.2 | subscribe splice 时序 + 默认 byte-equiv：构造 DataPlaneStores 含 audit + indexing_event_store；append audit entries + indexing rows（ts > since_ts）；subscribe(since_ts=T) → 先收到 indexing replay（evt-idx-*，ts ASC）再 audit replay（evt-audit-*）再 live（eb.send 触发）；subscribe(since_ts=0) → 无 replay（byte-equiv 现状）；store=None（DataPlaneStores::with_runner 不设）→ 无 indexing replay 退化 | `core/src/data_plane/events.rs`（同源 test） | Not Started |
| TEST-43.1.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Not Started |

## 8. Risks

- **R1（中）splice 时序错误**：replay batch 内 indexing/audit/live 顺序若错，订阅者看到乱序或 dedup 失败。
  - **缓解**：splice 严格 audit 后、live forward 前；两类 replay 各 id ASC / ts ASC；event_id 命名空间独立（evt-idx vs evt-audit）；TEST-43.1.2 断言时序。stop-condition：时序错乱则 AC2 不标 `[x]`。
- **R2（中）默认行为回归**：splice 若在 since_ts<=0 或 store=None 时仍发 indexing replay，破默认 byte-equiv。
  - **缓解**：splice 仅 since_ts>0 生效（镜像 audit :241 守护）；store=None `unwrap_or_default()` 空；TEST-43.1.2 断言两条退化路径 byte-equiv。stop-condition：退化路径非 byte-equiv 则 AC2 不标 `[x]`。
- **R3（低）Arc clone 误读为双开 Connection**：`indexing_event_store.clone()` clone Arc（共享 Mutex<Connection>），非新开 SQLite 文件。
  - **缓解**：`SqliteIndexingEventStore` 内部 `Mutex<Connection>`，Arc clone 共享同一 Connection；写路径 IndexSessionBackend 与读路径 DataPlaneStores 共享同一 store 实例（经 Arc）。stop-condition：若误新开 Connection 则违共享语义。
- **R4（低）live daemon e2e 被误读为已交付**：本 task 交付 unit 级 splice，live daemon e2e 未跑。
  - **缓解**：spec §5.2 + §10 据实记「splice 🟢 / live daemon e2e 🟡 honest-defer」；closeout 据已达维度 ratify。stop-condition：若把 unit splice 夸大为 live e2e 则越界。

## 9. Verification Plan

```bash
# 1. AC1 — list_since 时序过滤
cargo test -p contextforge-core --lib data_plane::indexing_events::test_43_1_1

# 2. AC2 — subscribe splice 时序 + 默认 byte-equiv
cargo test -p contextforge-core --lib data_plane::events::test_43_1_2

# 3. 不退化（全量）
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# 4. AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]：本 task 交付 indexing replay splice 接进 live subscribe（list_since + DataPlaneStores 字段 + serve_full 接线 + subscribe splice），🟢 可单测，0 新 dep（ADR-008）+ 0 schema migration（复用 0019）+ 0 proto 改动。live daemon restart-then-replay 端到端 e2e（真起进程 + 跨 restart 双窗口断言）须 running daemon / console 跨进程 → 🟡 honest-defer 不预填（ADR-013）。memory-actor-all-rpc 据实延后留独立 phase。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Ready（待实施回填）
