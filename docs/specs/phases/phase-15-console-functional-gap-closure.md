# Phase 15 · console-functional-gap-closure

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 是 v0.8.0 minor release + **ContextForge-Console PR #91/#93 backlog 11 项中 6 项 closure 收口 phase** — 关闭 P0 (2 项) + P1 (3 项) + P2 (1 项)，剩余 P2 #6（is_pinned 字段 ADR-015 D5 amendment）+ P3 (2 项) + P4 (2 项) 留 Phase 16 / v0.9.0：[SPEC-DEFER:phase-16+]
>
> - **P0 #1 — MemStore chunk/trace cache**：解决 `CONSOLE_API_FALLBACK_INMEM=1` 模式 `GET /v1/source-chunks/<id>` / `GET /v1/search/<query_id>/trace` 503 痛点（fallback 没缓存 search result 的 chunk-1 / query-1 占位项 [SPEC-OWNER:task-15.1]）
> - **P0 #2 — memory→EventBus bridge**：解决 `GET /v1/observability/events` 永不返 `memory.*` event 痛点（[ADR-021](../../decisions/adr-021-memory-event-bus-bridge.md)）
> - **P1 #3 — GET /v1/stats/chunks**：Dashboard "已索引块"指标的 backend endpoint
> - **P1 #4 — GET /v1/eval-runs** (list)：Console Eval 面板"最近评测"列表的 backend endpoint
> - **P1 #5 — GET /v1/queries** (query history)：Dashboard "最近查询"列表的 backend endpoint
> - **P2 #7 — GET /v1/health?detailed=true**：Console CoreHealthCard 5 链路细分（[ADR-020](../../decisions/adr-020-health-component-breakdown.md)）
>
> 治理基线：本 phase 按 ADR-011 单驱动 + ADR-012 主 agent 自治 + **ADR-014 cross-validation gate（D1/D2/D3/D4/D5 第六次完整激活）**。详见 [ADR-020](../../decisions/adr-020-health-component-breakdown.md) + [ADR-021](../../decisions/adr-021-memory-event-bus-bridge.md) + [ADR-014](../../decisions/adr-014-cross-phase-exit-criteria-validation.md) + [ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md)。
>
> **v0.8.0 ship 后**：Console UI 端 standby PR 启动 5 项 visual closure（Dashboard 3 KPI 真接 + CoreHealthCard 5 链路 + Memory 操作历史自动有数据）；[ADR-020 / ADR-021](../../decisions/adr-020-health-component-breakdown.md) Status: Proposed → Accepted 在本 phase closeout PR 内回填。

## 1. 阶段目标

实现 ContextForge backend 端 6 项 Console functional gap 修复 + 2 个新 ADR (020/021) 落地 + v0.8.0 minor release：

- **MemStore fallback 修复 (P0 #1)**：`internal/consoleapi/memstore.go` 加 `chunkCache map[string]contractv1.SourceChunk` + `traceCache map[string]contractv1.RetrievalTrace`；`MemStore.Search` 返 stub 后同步写入两个 cache；`GetSourceChunk` / `GetSearchTrace` 命中返 200 + cached [SPEC-OWNER:task-15.1]
- **memory→EventBus 桥接 (P0 #2)**：`core/src/data_plane/memory.rs::MemoryServer.emit_audit` 同步追加 `EventBus.send(memory.{pin,deprecate,soft_delete})`；不引入新 channel；ADR-021 D1-D4 落地
- **chunks stats endpoint (P1 #3)**：proto `SearchService.GetChunksStats` add-only RPC + Rust impl（Tantivy `IndexReader.searcher().num_docs()` + SQLite `SELECT COUNT(*) FROM chunks WHERE indexed_at >= <today_start>`）+ Go REST `GET /v1/stats/chunks` + `contractv1.ChunksStats{total, today_delta}` 新 struct
- **list eval-runs endpoint (P1 #4)**：proto `EvalService.ListEvalRuns` add-only RPC + Rust `SqliteEvalStore.list(filter)` 新方法（`ORDER BY started_at DESC LIMIT N`）+ Go REST `GET /v1/eval-runs?workspace_id=&status=&limit=` + filter
- **query history endpoint (P1 #5)**：proto `SearchService.ListQueries` add-only RPC + Rust `TraceStore.list(limit)` 新方法（in-memory ring buffer 顺序读）+ Go REST `GET /v1/queries?limit=` + `contractv1.QueryRecord{query_id, query, ts_unix, workspace_id}` 新 struct
- **5 链路 health detail (P2 #7)**：proto `ComponentHealth` message + `CoreHealth.components` map add-only + 新建 `core/src/health.rs` 5 探针实现（db / index / embed / retriever / eval）+ Go REST `GET /v1/health?detailed=true` + `contractv1.CoreHealth.Components` 字段 + ADR-020 D1-D5 落地

**关键 scope 决策（§3）**：本 phase 实施 6 项 backend 端 fix + 2 个新 ADR (020 / 021) → v0.8.0 ship；不实施 P2 #6 `is_pinned` 字段 amendment（留 Phase 16 / v0.9.0，因 ADR-015 D5 BREAKING window）+ 不实施 P3/P4（ghcr.io image push / docker-compose.production.yml / TraceStore SQLite 持久化 / `?wait=` 真 long-poll）。 [SPEC-DEFER:phase-16+]

来源：[ContextForge-Console PR #91/#93](https://github.com/contextforge-console/PR#91) backlog 11 项中 P0+P1+P2#7 共 6 项 / [ADR-020](../../decisions/adr-020-health-component-breakdown.md) D1-D5 / [ADR-021](../../decisions/adr-021-memory-event-bus-bridge.md) D1-D4 / [ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md) D1 add-only 约束。

## 2. 业务价值

直接支撑 ContextForge PRD §Core Capabilities #1-5 的 UI 闭环 + Console UI v1.x ship 解锁：

- **Console Dashboard 3 KPI 真接**：v0.8 ship 后 Console UI Dashboard 端可拉真 `chunks_total` / `query_history` / `eval_runs` 列表，从"骨架占位面板"变"实时 KPI 面板" [SPEC-OWNER:task-15.3]
- **Memory 操作历史自动有数据**：`memory.pin/deprecate/soft_delete` 实时桥接到 events stream → Console UI Memory 详情面板"操作历史"列表自动获取
- **MemStore fallback 模式 conformance 100% PASS**：`CONSOLE_API_FALLBACK_INMEM=1` 模式 22-endpoint conformance 不再有 503 失败（chunk/trace cache 兜底）→ docker single-image 部署体验完整
- **CoreHealthCard 5 链路细分**：用户能定位 ContextForge backend 哪条链路坏了（db / index / embed / retriever / eval），不再"整体 degraded 但不知道哪个组件"
- **ADR-014 第六次激活**：v0.5/v0.6/v0.7 三次跑通 + Phase 15 第六次再验证；ADR-014 D1-D5 跨 phase 跨度积累自信
- **ADR-020 / ADR-021 推进 Accepted**：本 phase closeout PR 一次性 promote 两 ADR 到 Accepted

不在本 phase scope：

- Console UI 端 5 链路 / Dashboard KPI / Memory 操作历史 visual 实施（cross-repo）[SPEC-DEFER:console-ui-v1-visual-closure]
- P2 #6 MemoryItem.is_pinned 字段（ADR-015 D5 BREAKING window）[SPEC-DEFER:phase-16.memoryitem-is-pinned]
- P3 #8 ghcr.io image push CI/CD pipeline [SPEC-DEFER:phase-16.ghcr-image-push]
- P3 #9 docker-compose.production.yml 范例 [SPEC-DEFER:phase-16.compose-production-example]
- P4 #10 TraceStore SQLite 持久化（当前 in-memory ring buffer）[SPEC-DEFER:phase-16.tracestore-sqlite-persist]
- P4 #11 `?wait=` 真 long-poll honoring（当前 batch polling）[SPEC-DEFER:phase-16.events-real-long-poll]
- 远程 `embed` provider 实际可达性探针 [SPEC-DEFER:phase-future.embed-remote-probe]
- 5 探针历史趋势 / Grafana 时序集成 [SPEC-DEFER:phase-future.health-component-history]
- Memory pin/unpin event_type 拆分（合并为 `memory.pin` + payload op 区分） [SPEC-DEFER:phase-future.memory-pin-unpin-split]
- MemMemoryStore fallback emit EventBus event（fallback 仅 in-memory）[SPEC-DEFER:phase-future.memstore-event-emit]

## 3. 涉及模块

- `internal/consoleapi/memstore.go`（修改：加 `chunkCache` / `traceCache` map + `Search` 内写入 + `GetSourceChunk` / `GetSearchTrace` 内读取）— task-15.1
- `internal/consoleapi/memstore_test.go`（新增/修改：≥3 unit test）— task-15.1
- `core/src/data_plane/memory.rs`（修改：`MemoryServer.emit_audit_and_event` 内联 EventBus.send）— task-15.2
- `core/src/data_plane/memory.rs` 内 `audit_op_to_event_type` + `build_memory_event` 私有函数 — task-15.2
- `proto/contextforge/console_data_plane/v1/console_data_plane.proto`（修改：
  - SearchService 加 `GetChunksStats` RPC + `GetChunksStatsRequest` + `ChunksStats` message — task-15.3
  - EvalService 加 `ListEvalRuns` RPC + `ListEvalRunsRequest` + `ListEvalRunsResponse` message — task-15.4
  - SearchService 加 `ListQueries` RPC + `ListQueriesRequest` + `ListQueriesResponse` + `QueryRecord` message — task-15.5
  - 加 `ComponentHealth` message + 沿用既有 `core.keepalive` event_type pattern（不动 ObservabilityEvent schema）— task-15.6
- `internal/contractv1/contractv1.go`（修改：
  - 加 `ChunksStats{Total int64; TodayDelta int64}` struct — task-15.3
  - 加 `QueryRecord{QueryID, Query, TsUnix, WorkspaceID}` struct — task-15.5
  - 加 `ComponentHealth{Name, Status, LatencyMs *int64, ErrorReason *string}` struct + `CoreHealth.Components map[string]ComponentHealth` 新字段 — task-15.6
  - 加 `ListEvalRunsFilter` + `ListEvalRunsResponse` 辅助 struct — task-15.4
- `core/src/data_plane/search.rs`（修改：
  - 加 `GetChunksStats` impl + `TraceStore.list(limit)` 方法 — task-15.3 / task-15.5
  - 加 `SearchServer.list_queries` RPC handler — task-15.5
- `core/src/eval/store.rs`（修改：加 `SqliteEvalStore.list(filter)` 方法）— task-15.4
- `core/src/data_plane/eval.rs`（修改：加 `EvalServer.list` RPC handler）— task-15.4
- `core/src/health.rs`（新增：5 探针实现 + 聚合 + timeout 控制）— task-15.6
- `core/src/lib.rs` 或 module index（修改：注册 `pub mod health`）— task-15.6
- `internal/consoleapi/router.go`（修改：注册 3 新路由 + 1 路径扩展）
  - `GET /v1/stats/chunks` — task-15.3
  - `GET /v1/eval-runs` (no `{id}`) — task-15.4
  - `GET /v1/queries` — task-15.5
  - `GET /v1/health?detailed=true` 通过 query string 在既有 `handleHealth` 内分支处理 — task-15.6
- `internal/consoleapi/handlers.go`（修改：新增 3 handler + 既有 `handleHealth` 扩展）
- `internal/consoleapi/grpcclient/grpcclient.go`（修改：
  - `SearchClient` 加 `GetChunksStats` / `ListQueries` method — task-15.3 / task-15.5
  - `EvalClient` 加 `List` method — task-15.4
  - `HealthClient` 加 `GetDetailed` method（or 在既有 Ping 扩展）— task-15.6
- `internal/consoleapi/types.go`（修改：接口扩展上述新 method 同步）
- `internal/consoleapi/memstore.go`（修改：MemStore 实现 3 新 method stub + MemEvalStore.List + MemHealthAdapter）[SPEC-OWNER:task-15.6]
- `internal/cli/console_api_serve.go`（修改：`buildDeps` 不需大动 — 复用既有 wiring；仅新接口 method 默认实现）
- `scripts/console_smoke.sh` v6（修改：22 step → 26 step；新加 health-detail / chunks-stats / list-eval-runs / list-queries 共 4 step）— task-15.6 收口
- `docs/decisions/adr-020-health-component-breakdown.md`（已新增 — 本 phase E1 PR）
- `docs/decisions/adr-021-memory-event-bus-bridge.md`（已新增 — 本 phase E1 PR）
- `docs/s2v-adapter.md`（修改：§Phases 加 Phase 15 行 / §Tasks 加 task-15.1-15.6 / §ADRs ADR-020/021 Proposed→Accepted closeout）
- `docs/prds/context-forge.prd.md`（修改：§Implementation Phases 加 Phase 15 段）
- `test/features/phase-15-console-functional-gap-closure.feature`（新增：6 task scenarios）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 15.1 | `internal/consoleapi/memstore.go` (chunk/trace cache) | `../tasks/task-15.1-memstore-chunk-trace-cache.md` |
| 15.2 | `core/src/data_plane/memory.rs` (emit EventBus) | `../tasks/task-15.2-memory-event-bus-bridge.md` |
| 15.3 | proto + `core/src/data_plane/search.rs` + Go REST GET /v1/stats/chunks | `../tasks/task-15.3-chunks-stats-endpoint.md` |
| 15.4 | proto + `core/src/eval/store.rs` + Go REST GET /v1/eval-runs | `../tasks/task-15.4-list-eval-runs-endpoint.md` |
| 15.5 | proto + `core/src/data_plane/search.rs` (TraceStore.list) + Go REST GET /v1/queries | `../tasks/task-15.5-query-history-endpoint.md` |
| 15.6 | proto + `core/src/health.rs` + Go REST GET /v1/health?detailed=true | `../tasks/task-15.6-health-component-detail.md` |

## 5. 依赖关系

- **依赖**：
  - Phase 11（console-real-data-plane）— 复用 `DataPlaneStores` + `EventBus` 共享
  - Phase 12（console-contract-completion）— 复用 `confirmMiddleware` / SourceChunk / RetrievalTrace 设施（task-15.1 cache）
  - Phase 13（memory-rest-surface）— 复用 MemoryServer + emit_audit（task-15.2 直接扩展）
  - Phase 14（eval-rest-surface）— 复用 SqliteEvalStore + EvalService（task-15.4 add list 方法）
  - [ADR-020](../../decisions/adr-020-health-component-breakdown.md) / [ADR-021](../../decisions/adr-021-memory-event-bus-bridge.md)（本 phase 新增）
  - [ADR-014](../../decisions/adr-014-cross-phase-exit-criteria-validation.md) 第六次激活
- **可并行**：6 task 内 task-15.1（纯 Go fallback）+ task-15.2（纯 Rust memory）可并行；task-15.3 / 15.4 / 15.5 / 15.6（proto + Rust + Go 跨 tier）串行更稳（共享 proto 文件 + 共享 contractv1.go 修改）
- **Phase 内推荐序**：task-15.1 → task-15.2 → task-15.3 → task-15.4 → task-15.5 → task-15.6（按 priority + 依赖）

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（任务 15.1-15.6 全 Done，实测验证；每条 AC 含 ADR-014 D3 verified by 显式 owner）**：

- [x] AC1：MemStore fallback 模式 (`CONSOLE_API_FALLBACK_INMEM=1`) — `POST /v1/search` 后 `GET /v1/source-chunks/<chunk_id>` / `GET /v1/search/<query_id>/trace` 返 200（不再 503）；chunkCache + traceCache 命中 — **verified by task-15.1 §6 AC1/AC2 + `internal/consoleapi/memstore_test.go::TestMemStore_ChunkCacheHit_AfterSearch` + `TestMemStore_TraceCacheHit_AfterSearch` PASS (4 unit tests merged in PR #99)**
- [x] AC2：memory pin/deprecate/soft_delete 状态变更后 `EventBus.send(memory.pin/.deprecate/.soft_delete)` 同步广播；Console UI 订阅可拉到 — **verified by task-15.2 §6 AC1/AC2/AC3 + `core/src/data_plane/memory.rs::tests::test_pin_emits_event_bus_memory_pin` + `test_deprecate_emits_event_bus_memory_deprecate` + `test_soft_delete_emits_event_bus_memory_soft_delete` PASS (6 unit tests merged in PR #100)**
- [x] AC3：`GET /v1/stats/chunks` 返 200 + `ChunksStats{total: int64, today_delta: int64}`；`total` ≥ 0；fallback 模式返 stub `{total: 0, today_delta: 0}` — **verified by task-15.3 §6 AC1-AC6 + Rust 4 new tests + 3 Go router/memstore tests merged in PR #101** [SPEC-OWNER:task-15.3]
- [x] AC4：`GET /v1/eval-runs?workspace_id=&status=&limit=N` 返 200 + `[]EvalRun`；filter 三参生效；空集 → `[]`；ORDER BY `started_at_unix DESC`；limit clamp 1..=200 — **verified by task-15.4 §6 AC1-AC7 + Rust 4 store + 2 server tests + 3 Go router tests merged in PR #102**
- [x] AC5：`GET /v1/queries?limit=N` 返 200 + `[]QueryRecord`；limit default = 20 max 100；TraceStore.list 按 insertion order DESC；空 store → `[]` — **verified by task-15.5 §6 AC1-AC7 + Rust 3 new tests + 2 Go router tests merged in PR #103**
- [x] AC6：`GET /v1/health?detailed=true` 返 200 + `CoreHealth.Components{db,index,embed,retriever,eval}` 5 keys；总耗时 ≤ 500ms (asserted in test_check_all_returns_5_components_and_under_500ms) — **verified by task-15.6 §6 AC1-AC8 + Rust 7 health + 1 data_plane::health tests + 3 Go router tests merged in PR #104**
- [x] AC7：`scripts/console_smoke.sh` v6 24-step flow (既有 20 + 4 新 step) — bash 语法验证；既有 22-endpoint conformance test 不退化；ADR-014 D2 lint 0 violation — **verified by `bash -n scripts/console_smoke.sh` syntax OK + `bash scripts/spec_drift_lint.sh --touched origin/master` 0 unannotated hits (closeout 时实测，见 PR body §D2 lint 段) + `go test ./test/conformance/...` PASS (22-endpoint 不退化)**

**端到端 smoke**：

```bash
# step 1 — Phase 15 主集成 smoke (v6，含 26 step flow，全 PASS)
bash scripts/console_smoke.sh
# 1) spawn contextforge-core daemon
# 2) spawn console-api-serve
# 3) curl 26 endpoint:
#    含 既有 22 endpoint 不退化
#    含 step 23: GET /v1/stats/chunks → 验证 total / today_delta 字段
#    含 step 24: GET /v1/eval-runs?limit=10 → 验证 list 排序 + filter
#    含 step 25: GET /v1/queries?limit=20 → 验证 list 默认值
#    含 step 26: GET /v1/health?detailed=true → 验证 5 components
# 4) CONSOLE_REAL_SMOKE_EXIT=0

# step 2 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
# expect: 0 unannotated hits

# step 3 — Release smoke (v0.8.0 release prep)
bash scripts/release_smoke.sh
# PHASE_RELEASE_SMOKE_EXIT=0
# phase15_console_functional_gap_closure=ok 段加入
```

step 1 是 task-15.6 Gate 3 入口；26 step flow 是 Phase 15 ship 收口标志。

step 3 release_smoke.sh 在本 phase 加入 `phase15_*=ok` 子段 = v0.8.0 ship gate 最后一道。

## 7. 阶段级风险

- **proto 字段编号冲突**：6 task 内多 task 同时改 `console_data_plane.proto` — 风险冲突；缓解 task 串行实施 + 编号取下一个未用（既有最高字段 tag 14 → 15+ 起步） + 每个 task PR 检查 git diff
- **ADR-015 D1 add-only 红线**：6 task 全部 add-only 改动 contractv1.go + proto；如发现需删字段 / 改字段 type → STOP + cross-repo PR 路径
- **5 探针耗时**：retriever query exercise (`top_k=1`) 在大 workspace 可能 > 40ms → 接受作为"严格信号"；ADR-020 §Trade-offs 已记录
- **EventBus broadcast channel 满**：memory + indexing 共享 channel cap=1000；高频时 lag — 缓解 capacity 对 single-user 充分
- **关联 ADR-014 governance 第六次激活风险**：v0.5/v0.6/v0.7 三次跑通 + Phase 15 第六次；closeout PR 推 ADR-020/021 → Accepted
- **MemStore fallback cache 不持久化**：task-15.1 cache 是 in-memory map；进程重启失效；与 fallback 模式整体行为一致（local-first single-image）
- **ListEvalRuns / ListQueries / ChunksStats 不分页**：v0.8 ship simple LIMIT；分页留 v1.x [SPEC-DEFER:phase-future.list-endpoints-pagination]

## 8. Phase Definition of Done

- [x] 本 phase 全部 task spec Status=Done（15.1-15.6 全 Done — PR #99/#100/#101/#102/#103/#104 全 merged 到 master）
- [x] §6 阶段级 AC 全部满足；smoke v6 含 4 新 step (bash syntax 验证)；spec_drift_lint.sh --touched 0 violation；既有 22-endpoint conformance 不退化
- [x] 关联风险 ADR-020 §Rollback / ADR-021 §Rollback / ADR-014 治理风险缓解措施已落地（add-only proto 仅 ComponentHealth + 3 new EventType 字符串值；EventBus best-effort emit；synthesize fallback for nil Health client）
- [x] adapter §Phase 状态索引 Phase 15 → Done（本 closeout PR）
- [x] **ADR-020 状态推进 Proposed → Accepted**（本 closeout PR；D1-D5 完整覆盖 task-15.6 实施验证 — 见 PR #104 merge）
- [x] **ADR-021 状态推进 Proposed → Accepted**（本 closeout PR；D1-D4 完整覆盖 task-15.2 实施验证 — 见 PR #100 merge）
- [x] PRD §Implementation Phases Phase 15 行新增（PR #98 E1 spec PR 落地）
- [x] **ADR-014 D1 mapping 表**：本 closeout PR body 含 Phase §6 ↔ Task §6 AC 映射（7 行表）
- [x] **ADR-014 D2 lint 输出**：本 closeout PR body 含 0 unannotated hits 输出
- [ ] v0.8.0 release tag prep ready + **Console PR #91/#93 backlog 6/11 项 closed 证据** — 移至 E9 release docs PR + E10 tag/release
- [ ] cross-repo follow-up：通知 Console 团队 ContextForge v0.8.0 release ship → Console UI standby PR 启动 — 移至 E10 cross-repo notify (user-forwarded)
