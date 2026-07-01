# ADR `048`: `indexing-replay-splice`

**Status**: Proposed（Phase 43 规划阶段；ratify 在 task-43.3 closeout 据真实 CI 逐 D）

**Category**: 治理债清理（第四轮）/ observability replay 拼接 / 默认 byte-equiv 守线
**Date**: 2026-07-01
**Decided By**: 主 agent（ADR-012 自治）
**Related**: ADR-038（governance-debt-cleanup-2 — D3 indexing event 持久化 + replay mapper 已交付，本 ADR 兑现其 `[SPEC-DEFER:phase-future.indexing-replay-e2e]` 的 splice 拼接维度，add-only Amendment 不溯改正文）/ ADR-031（observability-hardening — replay 范式源 task-26.2，本 ADR 复用 since_ts 守护 + best-effort unwrap_or_default 模式）/ ADR-021（memory-event-bridge — audit replay splice 是镜像源，本 ADR 把 indexing replay 对称接进同一 subscribe 路径）/ ADR-004（local-first-privacy-baseline — 默认行为不变 + 0 网络；since_ts<=0 / store=None byte-equiv）/ ADR-008（dep add-only — Phase 43 = 0 新依赖，复用既有 migration 0019 + mapper）/ ADR-013（禁伪造红线 — splice 真实接入非合成、since_ts 时序单测守护；live daemon restart e2e 🟡 honest-defer 不预填）/ ADR-012（main-agent-governance-autonomy — tag/release outward-facing 须用户显式授权）/ ADR-014（D1-D5，第三十四次激活）/ roadmap §3.25 + §4

## Context

ContextForge 截至 Phase 42（chunk-source-type-filter, Done / v0.35.0）已完成 42 个 phase。Phase 33 task-33.3（ADR-038 D3）交付了 indexing event 的**持久化 + replay mapper**：

- **已交付（Phase 33 task-33.3）**：add-only migration `0019_indexing_events` + `SqliteIndexingEventStore`（`core/src/data_plane/indexing_events.rs`，`append`/`list`）+ 4 个 emit 点 best-effort 持久写（`jobs/index_session_backend.rs` 经 `IndexSessionBackend::with_event_bus_and_indexing_store`）+ 纯 mapper `indexing_rows_to_pb_events`（`events.rs:438`，真实 `job_id`/`processed`/`total` 取持久行不合成，确定性 `evt-idx-{id}`）。`test_33_3_2` 守护 store round-trip + mapper rebuild。
- **延后的拼接缺口（本 ADR 的目标）**：mapper 已写好并验证，但**未接进 live subscribe 路径**。具体 4 个缺口（grounding 已亲自核实，非转述）：
  1. `SqliteIndexingEventStore::list(limit)`（`indexing_events.rs:111`）只接受 `limit`，**缺 `since_ts` 参数**——而 replay 须按 since_ts 过滤（与 `replay_events_from_audit` 的 `ts < since_ts → skip` 对齐）。
  2. `DataPlaneStores`（`core/src/data_plane/mod.rs:43-74`）结构体**无** `indexing_event_store` 字段。
  3. `serve_full`（`server.rs:756-762`）**局部已构造** `indexing_event_store` 并传给 `IndexSessionBackend`（写路径 OK），但 `DataPlaneStores::full()`（`server.rs:788-798`，9 参数）**未传入**该 store。
  4. `EventsServer::subscribe`（`events.rs:241-250`）的 replay splice **只接了 `self.stores.audit`**（memory audit replay），**未接 indexing replay**。

结果：`since_ts > 0` 的订阅者能收到 missed 的 memory `*.pin`/`*.deprecate`/`*.soft_delete` 事件（audit replay），但**收不到** missed 的 `indexing.progress`/`.cancelled`/`.error` 事件——indexing replay mapper 写好了却从不在 live 路径被调用。这是 ADR-038 D3 明确标注的 `[SPEC-DEFER:phase-future.indexing-replay-e2e]` 的"最后一公里"。

**诚实定性（ADR-013）**：本 ADR 交付 splice **拼接**（接进 live subscribe 路径 + since_ts 时序），🟢 纯本地单测可验证时序正确性 + 拼接 + 默认 byte-equiv。**live daemon restart-then-replay 端到端 e2e**（真起进程 + 跨 restart 双窗口断言）须 running daemon（须 console 跨进程），🟡 honest-defer `[SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]`——本 ADR 交付 unit 级 splice + 时序单测，不预填 e2e。

## Decision

第四轮治理债清扫采用 **「indexing replay splice 接进 live subscribe 路径 + since_ts 时序对齐 audit + 默认 byte-equiv 守线」** 策略，分 4 个决策点：

### D1 — `SqliteIndexingEventStore::list_since(limit, since_ts)` 时序过滤（task-43.1）🟢

给 `SqliteIndexingEventStore`（`indexing_events.rs`）add `pub fn list_since(&self, limit: usize, since_ts: i64) -> Result<Vec<IndexingEventRow>, IndexingEventStoreError>`：`since_ts > 0` 时 `WHERE ts_unix >= ?since ORDER BY id ASC LIMIT ?`（镜像 `replay_events_from_audit` 的 `ts < since_ts → skip` 语义，含等号边界）；`since_ts <= 0` 时不过滤（返全量 limit，与既有 `list()` 行为一致）。既有 `list(limit)` 保留不动（其他调用方不受影响）。同源单测 TEST-43.1.1 守护 since_ts 过滤 + id ASC 时序。

**理由**：replay 须让订阅者收到"自 since_ts 起 missed 的事件"。audit replay 已用 `ts < since_ts → skip`（`events.rs:401-403`）建立范式；indexing 复用同语义（`ts_unix >= since`）最 surgical。既有 `list(limit)` 保留（`test_33_3_2` 等既有调用方不破）。0 新 dep、0 schema migration（复用 migration 0019 既有 `ts_unix` 列）。

### D2 — `DataPlaneStores` 加 `indexing_event_store` 字段 + `serve_full` 接线（task-43.1）🟢

(a) `DataPlaneStores`（`mod.rs:43-74`）add `pub indexing_event_store: Option<Arc<SqliteIndexingEventStore>>` 字段；`full()` constructor（`mod.rs:156-178`）加第 10 参数；所有既有 constructor（`new`/`with_eval`/`with_memory`/`with_runner`/`with_runner_and_bus`）补 `indexing_event_store: None`（既有调用方 byte-equiv）。
(b) `serve_full`（`server.rs:756-798`）：`indexing_event_store` 已在 `:756` 局部构造并传给 `IndexSessionBackend`（写路径已在），本 D 仅 clone 一份传入 `DataPlaneStores::full(..., Some(indexing_event_store.clone()))`（第 10 参数）——读路径（subscribe replay）也可达。

**理由**：`DataPlaneStores` 是所有 data plane service 共享的 store 注入点（mod.rs doc），events subscribe 路径经 `self.stores` 读 store。store 已在 serve_full 局部构造（写路径已用），clone 进 DataPlaneStores 使读路径（subscribe replay）对称可达，0 新构造、0 双开。既有 constructor 补 `None` 保既有调用方 byte-equiv（单测、`with_runner` 等不接 indexing replay，退化到现状）。

### D3 — `EventsServer::subscribe` 加 indexing replay splice（task-43.1）🟢

`subscribe`（`events.rs:223-308`）的 replay 段（`:241-250`）在既有 audit replay 之后、live forward（`:251` spawn）之前，加 indexing replay：`since_ts > 0` 时 `self.stores.indexing_event_store.as_ref().and_then(|s| s.list_since(REPLAY_LIMIT, req.since_ts).ok()).map(|rows| indexing_rows_to_pb_events(&rows)).unwrap_or_default()`；与 audit replay 合并进同一个 `replay: Vec<PbEvent>`（indexing 在前 / audit 在后，两类均 id ASC / ts ASC 内部有序；客户端按 `event_id`（`evt-idx-{id}` vs `evt-audit-{id}`）dedup splice 边界）。store None / lock 失败 / 空 → `unwrap_or_default()` 空切片（best-effort，镜像 audit `:245-247`）。`REPLAY_LIMIT` 常量与 audit replay 同限（既有内存上限，防爆）。同源单测 TEST-43.1.2 守护：subscribe 带 since_ts → 先收到 indexing replay 再 audit replay 再 live；since_ts<=0 → 无 replay byte-equiv；store None → 无 indexing replay。

**理由**：indexing replay mapper（`indexing_rows_to_pb_events`）已写好并经 `test_33_3_2` 验证，但从未在 live 路径调用——这是"最后一公里"债。splice 进既有 replay 段（audit replay 之后、live forward 之前）保证：(1) subscribe-first（`:235` subscribe_all 在 replay 构造之前）不丢 live 事件（镜像 task-26.2 既有模式）；(2) 两类 replay 都在 live 之前发送，客户端可按 event_id dedup。0 新 dep、0 migration、0 proto 改动（纯内部 read 路径 splice）。默认 byte-equiv：since_ts<=0 → 无 indexing replay（与现状一致）；store None → 无 indexing replay（与现状一致）。

### D4 — 默认 byte-equiv + honest-defer 边界 + 0-dep / 0-migration 守线（all tasks）🟢 / 🟡

所有改动保持默认行为 byte-equiv + 0 网络（ADR-004）+ 0 新依赖 + 0 schema migration（ADR-008，复用 migration 0019）：
- `since_ts <= 0`（订阅首连，无 since_ts）：indexing replay 返空（`req.since_ts > 0` 守护，与既有 audit replay `:241` 同分支）→ 行为与现状 byte-identical。
- `indexing_event_store == None`（旧 constructor / 单测不设）：indexing replay 返空 → 退化到现状。
- 仅 `serve_full` 生产路径（`server.rs:788`）把已构造的 store 传入新字段。
- live daemon restart-then-replay 端到端 e2e（真起进程 + 跨 restart 双窗口）🟡 honest-defer `[SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]`——本 ADR 交付 unit 级 splice + 时序单测，不预填 e2e（ADR-013）。

**理由**：ADR-004 local-first + ADR-008 dep add-only——默认行为 byte-equiv + 0 网络 + 0 新依赖 + 0 migration 是不可让渡 baseline。本 ADR 为治理债拼接——非默认行为演进。`since_ts<=0` / `store=None` 两条退化路径均 byte-equiv，既有用户与既有契约零感知。live daemon e2e 须 running daemon（须 console 跨进程），本 ADR 据 ADR-013 honest-defer 不预填。

## Consequences

- **Positive**: indexing event replay 真实接进 live subscribe 路径——`since_ts > 0` 的订阅者现可收到 missed 的 `indexing.progress`/`.cancelled`/`.error` 生命周期事件（与既有 memory audit replay 对称），兑现 ADR-038 D3 `[SPEC-DEFER:phase-future.indexing-replay-e2e]` 的 splice 维度。mapper 不再是"写好却从不在 live 路径调用"的死代码。since_ts 时序正确性经单测守护；默认行为 byte-equiv（since_ts<=0 / store=None 两条退化路径）；0 新 dep / 0 schema migration / 0 proto 改动 / 0 网络（ADR-004/008）。
- **Negative / open**（受阻维度如实，不伪造、不预填）：live daemon restart-then-replay 端到端 e2e（真起进程 + 跨 restart 双窗口断言）须 running daemon（须 console 跨进程）→ 🟡 `[SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]`；memory-actor-all-rpc（Deprecate/SoftDelete 7 层改动 + 新 schema migration / HardDelete 须 audit 层重设计）据实 honest-defer 留独立 phase（本 ADR 单聚焦 indexing-replay，不强行扩面）。
- **Ratification**: 本 ADR **Proposed**。task-43.1 通过后于 v0.36.0 closeout（task-43.3）据真实 CI 逐 D ratify Proposed→Accepted（ADR-013：禁据合成 / 伪造 ratify）；live daemon e2e 🟡 维度据已达 unit 级 splice ratify + 如实记录受阻，不强 ratify e2e。
- **Follow-ups**: live daemon restart-then-replay e2e `[SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]`；memory-actor-all-rpc（独立 phase）；其余 roadmap §4 backlog 项据实保持延后。

## Alternatives

- **A1（不做 splice，依赖 EventBus live-only）**：保留现状——indexing event 仅 EventBus live 广播，restart 后 missed 事件不可补。否决：mapper 已写好（Phase 33 投入），不接进 live 路径等于让既有代码成为死代码；audit replay 已建立 since_ts splice 范式（task-26.2），indexing 对称拼接是治理对称缺口。0 新 dep / 0 migration 即修。
- **A2（合并 indexing replay 与 audit replay 为统一 replay source）**：把 indexing event 也写进 `audit_log` 表，复用 `replay_events_from_audit`。否决：`AuditLogEntry` 缺 `job_id`/`processed`/`total` 列（ADR-038 D3 已论证），须改 audit schema 破 add-only；Phase 33 已选专用 `indexing_events` 表（migration 0019）更干净。本 ADR 仅在 subscribe 路径 splice 两类 replay，不动 schema。
- **A3（一并做 memory-actor-all-rpc 四 RPC）**：把 memory-actor 四 RPC 也塞进本 phase。否决：grounding 显示 Deprecate/SoftDelete 需 7 层改动 + 新 schema migration（非小债），HardDelete 无法在行上存 actor（须 audit 层重设计）——超"治理债小 phase 刻意小"定位（roadmap §3.17/§3.22 "据实排小不凑数"）。本 ADR 单聚焦 indexing-replay，memory-actor 据实 honest-defer 留独立 phase（ADR-013 不强行扩面）。
