# Phase 26 · observability-hardening

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 硬化 Phase 16 落地的两条可观测性信号路径：**TraceStore 持久化**（`core/src/data_plane/search_persist.rs::SqliteTracePersist`，task-16.1 — `search_traces` 表无按内容检索 + 无清理路径无界膨胀）与 **events 实时面**（`internal/consoleapi` `GET /v1/observability/events` long-poll，task-16.2 + `core/src/data_plane/events.rs::EventBus`，ADR-021）。两面硬化：trace 全文检索（FTS5）+ 周期 VACUUM；events SSE 实时推送（替代 long-poll 重订阅）+ 从 audit log 重放漏失事件；event-bus 分区 / 容量 / drain 超时配置。全部 local-first（默认 0 新 dep / 0 network，ADR-004）。v0.19.0 收口。对应 `docs/roadmap.md`。
>
> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md` → `docs/decisions/adr-031-observability-hardening.md`（D1-D6 + Proposed→ratify 口径）→ `docs/decisions/adr-021-memory-event-bus-bridge.md`（`EventBus broadcast::channel(1000)` + best-effort emit D4 + Trade-off `[SPEC-DEFER:phase-future.events-replay-from-audit]` `adr-021:115` + Rollback path「提容量 / partition channel」`adr-021:153`）→ `core/src/data_plane/search_persist.rs::SqliteTracePersist`（`put`/`get`/`list`/`load_warm` + `core/migrations/0015_search_traces.sql` schema）→ `internal/consoleapi/handlers.go::handleEvents`（`GET /v1/observability/events` long-poll + `:655` `[SPEC-DEFER:task-future.consoleapi-sse]` 标记）→ `internal/consoleapi/grpcclient/grpcclient.go::eventsClient.Recent`（两阶段 long-poll + `:419-424` phase 间隙漏事件自述）→ `core/src/data_plane/events.rs::EventBus`（`broadcast::channel(1000)` + `with_capacity` seam + `subscribe`）→ `core/src/memoryops/audit.rs::AuditSink`（`audit_log` 表 + `list()`/`record()` — events 重放源）→ AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5，第十七次激活）→ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造凭据红线）→ `docs/decisions/adr-004-local-first-privacy-baseline.md`（默认 0 新 dep / 0 network）→ `docs/decisions/adr-008-core-library-selection.md`（依赖选型 add-only）。
>
> **ADR 影响面（已识别）**：
> - **ADR-031 observability-hardening（新，Proposed）**：记 trace FTS5 全文检索 + 周期 VACUUM（D1/D2）+ events SSE 实时推送（D3）+ 从 audit log 重放（D4）+ event-bus 分区 / 容量 / drain 配置（D5）+ 默认 0-dep 不变（D6）。落地后据真实非合成 FTS 往返 / VACUUM 回收 / SSE 帧契约 / 重放顺序契约 ratify（ADR-013）。
> - 触及 **ADR-021（memory-event-bus-bridge）**：兑现其 `[SPEC-DEFER:phase-future.events-replay-from-audit]`（重放，`adr-021:115`）+ Rollback path 预见的「提容量 / partition channel」（event-bus 配置，`adr-021:153`）——以 add-only Amendment 记录推进结果，不溯改 ADR-021 正文 D1-D4（ADR-014 D5）。
> - 触及 **ADR-015（console-contract-v1-compatibility）**：SSE endpoint 为 add-only 新增（既有 long-poll endpoint + 22-endpoint 契约不动），按 add-only 记录。

## 1. 阶段目标

v0.19.0 ship 后，ContextForge 的可观测性两条信号路径具备**持久化硬化的 TraceStore**（`search_traces.db` 支持 FTS5 按内容检索 + 周期 VACUUM 抑制无界膨胀）与**实时硬化的 events 面**（SSE 实时推送替代 long-poll 重订阅 + 从 audit log 重放订阅前漏失事件 + event-bus 容量 / 分区 / drain 超时可配）。默认构建仍 0 新依赖 / 0 network（ADR-004）、既有 long-poll endpoint + 22-endpoint 契约 + `put`/`get`/`list`/`load_warm` 签名不退化（ADR-015 add-only）。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. `SqliteTracePersist` 支持 FTS5 按内容检索（`search_fts(query_text, limit)` 命中含该文本的 trace）+ 周期 VACUUM（`vacuum()` / 保留策略回收空间且数据不破坏）；deterministic 单测可断言（index trace → FTS-search 命中 / 插入→删除→vacuum→数据完好）；既有 `put`/`get`/`list`/`load_warm` 签名与语义不变，默认构建 0 新依赖（FTS5 / VACUUM 复用 rusqlite bundled SQLite）（AC1）
2. `internal/consoleapi` 加 SSE 实时推送 endpoint（`text/event-stream`，旁挂既有 long-poll，add-only）+ 从 audit log 重放漏失事件；SSE 帧编码 + 重放顺序经 deterministic 契约测试可断言（注入确定事件序 → 断言 SSE 帧格式 + 重放 audit `id ASC` 顺序 + 拼接边界不重复 / 不乱序），不依赖实时 timing flakiness（ADR-013）（AC2）
3. event-bus 分区 + 容量 + drain 超时可配（`event-bus-partition` / `event-bus-capacity` / `events-drain-timeout-config`），带保守默认使既有行为默认不变（容量默认 1000 / 不分区 / drain 默认 ~100ms）；deterministic 单测可断言配置生效 + 默认等价（AC3）
4. v0.19.0 release docs + `console_smoke.sh` v16 + phase §6 闭合 + ADR-031 据真实非合成结果 ratify 或记录维持 + ADR-021/015 add-only Amendment（AC4）
5. ADR-014 D1-D5（第十七次激活）全通过（AC5）

**v0.x 版本号决策**：v0.19.0 minor release（可观测性 trace + events 硬化收口；默认构建仍 0 新依赖 / 0 network + 既有契约不退化——FTS5 / VACUUM / SSE / 重放 / event-bus 配置均 add-only 或 opt-in，不破坏既有客户端）。

## 2. 业务价值

直接硬化 Phase 16 落地的两条可观测性信号路径 + 兑现 ADR-021 两处预留：

- **trace 全文检索 + VACUUM**：task-16.1 `SqliteTracePersist` 当前仅 `get(query_id)` 主键命中 + `list(limit)` 时间序——无法「按内容查 trace」（如查所有命中某关键词的检索），且 `search_traces` 单调增长无清理（`put` 是 `INSERT OR REPLACE` 仅同 query_id 替换，不同 query_id 无界膨胀）。本 phase 加 FTS5 影子表（按内容检索）+ 周期 VACUUM（抑制膨胀），让 trace 持久面可查询、可维护（D1/D2，`docs/decisions/adr-031-observability-hardening.md`）。
- **events SSE + 重放**：task-16.2 events 是两阶段 long-poll（`handlers.go:655` 自标 `[SPEC-DEFER:task-future.consoleapi-sse]` + `grpcclient.go:419-424` 自述 phase 间隙漏事件），每轮重订阅；ADR-021 Trade-off 明记不重放历史（`adr-021:115` `[SPEC-DEFER:phase-future.events-replay-from-audit]`）。本 phase 加 SSE 实时推送（持续流，消除重订阅 + 间隙漏失）+ 从 audit log 重放订阅前事件（兑现 ADR-021 预留），让实时面无遗漏（D3/D4）。
- **event-bus 配置**：ADR-021 D4 + Rollback path 预见「memory 事件高频挤占 indexing 事件 → 提容量或 partition channel」（`adr-021:118` / `adr-021:153`）但 `EventBus::new()` 仍硬编码 `broadcast::channel(1000)`（`events.rs:31`）。本 phase 加容量 / 分区 / drain 超时配置（复用既有 `with_capacity` seam，`events.rs:35`），兑现 ADR-021 Rollback path 预见（D5）。
- **PRD §Constraints（local-first + 可观测性）**：trace 可查询 + events 无遗漏 + 默认 0 新 dep / 0 network（ADR-004），推进单用户 local-first 部署的可观测性基线。

**不在本 phase scope**：

- events 重放扩展到 `indexing.*` 类事件（需 indexing 事件持久化源；audit_log 当前仅持久 memory state-op）[SPEC-DEFER:phase-future.indexing-event-persistence]
- SSE 多客户端 fan-out 背压 / 压力调优 [SPEC-DEFER:phase-future.sse-backpressure-tuning]
- trace FTS5 跨库 schema 迁移 / 重建 [SPEC-DEFER:phase-future.tracestore-fts-schema-migration]
- 跨进程 / 多节点事件广播（Kafka/NATS 类替换属 ADR-004 local-first 红线外）[SPEC-DEFER:phase-future.distributed-event-bus]
- trace 内容脱敏 / 二次审计（audit log 既有脱敏由 ADR-010 覆盖）[SPEC-DEFER:phase-future.trace-content-redaction]

## 3. 涉及模块

### 26.1 TraceStore FTS + VACUUM（task-26.1）

- 修改 `core/src/data_plane/search_persist.rs`——`SqliteTracePersist` 加 `search_fts(query_text, limit)`（FTS5 影子表按内容检索）+ `vacuum()` / 可选 `prune_older_than(ts_unix)`（周期回收空间）；`put` 时同步写 FTS 影子表（触发器或显式同步）；既有 `put`/`get`/`list`/`load_warm` 签名与语义不变（add-only）
- 新增 `core/migrations/0016_*.sql`——FTS5 影子虚表（`search_traces_fts`）+ 触发器（承 `0015_search_traces.sql` 编号序，`IF NOT EXISTS` 幂等，旧库 boot 时回填）
- 同源 Rust tests（≥3：FTS index→search 命中 / FTS miss 不命中 / VACUUM 后数据完好 + row_count 一致）
- FTS5 / VACUUM 复用 `rusqlite = { features = ["bundled"] }`（`core/Cargo.toml:70`）bundled SQLite——0 新依赖（ADR-004 / ADR-008，无 Cargo.toml 改动）

### 26.2 events SSE 推送 + 重放（task-26.2）

- 修改 `internal/consoleapi`（`handlers.go` + `router.go` + `grpcclient/grpcclient.go`）——加 SSE 实时推送 endpoint（`text/event-stream`，`http.Flusher` 持续 flush，旁挂既有 `GET /v1/observability/events` long-poll，add-only）+ 从 audit log 重放（`?replay=` / `?since_ts=` 参 → 经 Rust 查 `audit_log` 重建漏失事件序）
- 修改 / 新增 Rust 重放查询面（`core/src/data_plane/events.rs` 或邻接 — 从 `core/src/memoryops/audit.rs::AuditSink::list()` audit `id ASC` 重建 `ObservabilityEvent`）
- 同源 Go contract tests（≥2：SSE 帧编码 — 注入确定事件序断言 `id:`/`event:`/`data:` 帧格式；重放顺序 — audit `id ASC` 升序 + 与实时流拼接边界不重复 / 不乱序，deterministic 不依赖墙钟）
- SSE 用 Go 标准库 `net/http` `http.Flusher`——0 新依赖（ADR-004）

### 26.3 event-bus 配置 + closeout（task-26.3）

- 修改 `core/src/data_plane/events.rs` + consoleapi——`event-bus-capacity`（替换硬编码 `broadcast::channel(1000)`，复用 `with_capacity` seam `events.rs:35`）+ `event-bus-partition`（`memory.*` / `indexing.*` 分区可选）+ `events-drain-timeout-config`（grpcclient phase-2 drainTimeout 可配）；保守默认使既有行为默认不变
- 修改 `scripts/console_smoke.sh`——v16：events SSE / trace FTS / event-bus 配置相关 smoke 断言（既有 step 不退化）
- 新增 `docs/releases/v0.19.0-{evidence,artifacts}.md` + `README.md` v0.19 段 + `RELEASE_NOTES.md` v0.19.0 段
- 修改 `docs/decisions/adr-031-observability-hardening.md`——据真实结果 Proposed→Accepted 或记录维持 + ADR-021/015 add-only Amendment（推进结果记录，不溯改正文，ADR-014 D5）
- 修改 `docs/s2v-adapter.md`（Phase 26 Draft→Done + Tasks 0→3；ADR-031 状态；ADR-021 预留兑现记录）

### BDD feature

- 新增 `test/features/phase-26-observability-hardening.feature`（≥3 scenario：trace FTS + VACUUM / events SSE + 重放 / event-bus 配置 + 收口）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 26.1 | `core/src/data_plane/search_persist.rs` FTS5 全文检索 + VACUUM + `core/migrations/0016_*.sql` + roundtrip 测试 | `../tasks/task-26.1-tracestore-fts-and-vacuum.md` |
| 26.2 | `internal/consoleapi` SSE 实时推送 endpoint + 从 audit log 重放 + Rust 重放查询面 + contract 测试 | `../tasks/task-26.2-events-sse-push-and-replay.md` |
| 26.3 | event-bus 分区 / 容量 / drain 配置 + smoke v16 + v0.19.0 closeout + ADR-031 ratify | `../tasks/task-26.3-closeout-v0.19.0.md` |

## 5. 依赖关系

- **task-26.1**（trace FTS + VACUUM）dep Phase 16 task-16.1（`SqliteTracePersist` + `0015_search_traces.sql` 已落地）；可与 26.2 并行（写路径不相交：`search_persist.rs` vs `internal/consoleapi`）。
- **task-26.2**（events SSE + 重放）dep Phase 16 task-16.2（events long-poll + `grpcclient.eventsClient.Recent` 已落地）+ Phase 11 task-11.4（`EventBus` + `EventsService.Subscribe` server-stream）+ ADR-021（memory event bus + audit_log 重放源）；可与 26.1 并行。
- **task-26.3**（closeout）dep 26.1 + 26.2 全 Done；event-bus 配置为本 task 子项（依赖 `events.rs::EventBus` `with_capacity` seam）。
- 外部：ADR-031（本 phase 新 Proposed）/ ADR-021（memory-event-bus-bridge，本 phase 兑现其重放预留 + Rollback path 容量 / 分区预见，add-only Amendment）/ ADR-015（console-contract，SSE add-only）/ ADR-004（local-first 0 新 dep / 0 network 红线）/ ADR-008（依赖变更 add-only）/ ADR-014 第十七次激活 / ADR-013（禁伪造凭据）/ ADR-002（rusqlite bundled SQLite 分层 — FTS5 / VACUUM 来源）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（每条 AC 含 ADR-014 D3 verified by 显式 owner；Draft 阶段未勾选，实施后逐条置 `[x]`）**：

- [x] **AC1**：`SqliteTracePersist` FTS5 按内容检索（`search_fts` 命中含文本的 trace）+ 周期 VACUUM（`vacuum()` / 保留策略回收空间且数据完好）；deterministic 单测可断言（index→FTS-search 命中 / 插入→删除→vacuum→数据完好）；既有 `put`/`get`/`list`/`load_warm` 签名与语义不变；默认构建 0 新依赖（FTS5 / VACUUM 复用 rusqlite bundled） — verified by task-26.1 §6 AC1-5 + phase-smoke step 1（10/10 search_persist 单测）
- [x] **AC2**：`internal/consoleapi` SSE 实时推送 endpoint（`text/event-stream`，旁挂 long-poll，add-only）+ 从 audit log 重放漏失事件；SSE 帧编码 + 重放顺序经 deterministic 契约测试可断言（注入确定事件序 → SSE 帧格式 + 重放 audit `id ASC` 顺序 + 拼接边界不重复 / 不乱序），不依赖实时 timing flakiness（ADR-013） — verified by task-26.2 §6 AC1-4 + phase-smoke step 2（Rust replay 2/2 + Go SSE 4 契约；live e2e 诚实延后）
- [x] **AC3**：event-bus 分区 + 容量 + drain 超时可配（`event-bus-partition` / `event-bus-capacity` / `events-drain-timeout-config`），保守默认使既有行为默认不变（容量默认 1000 / 不分区 / drain 默认 ~100ms）；deterministic 单测可断言配置生效 + 默认等价 — verified by task-26.3 §6 AC1 + phase-smoke step 3（events 6/6 + drain 5/5）
- [x] **AC4**：v0.19.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ `console_smoke.sh` v16 + ADR-031 据真实非合成结果 ratify 或记录维持 + ADR-021/015 add-only Amendment + phase §6 闭合 — verified by task-26.3 §6 AC2-3
- [x] **AC5**：ADR-014 cross-validation gate 全套通过（第十七次激活）— D1 mapping + D2 lint `--touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-25 不溯改 — verified by task-26.3 closeout PR body

**端到端 smoke（C1 集成兜底）**：(1) trace FTS5 按内容检索命中 + VACUUM 回收（feature 无关，默认构建可跑）；(2) events SSE 帧契约 + 从 audit log 重放顺序（contract 层 deterministic）；(3) event-bus 容量 / 分区 / drain 配置生效 + 默认等价 全 PASS。

## 7. 阶段级风险

- **R1（中）FTS5 影子表 + 触发器使 `put` 写放大**：每次 `put` 多写一份 FTS 倒排索引；高频写场景成本上升。
  - **缓解**：trace `put` 是检索后写、非高频热路径（单用户 local-first）；FTS5 是 bundled SQLite 内建（无外部成本）；写放大限于 trace 持久面，不触及检索热路径。stop-condition：若 FTS5 在 bundled SQLite 中不可用（极少见，bundled 默认含 FTS5）则记录受阻态，AC1 不标 `[x]` 并按 ADR-013 如实记录。
- **R2（高）SSE 实时测试 timing flakiness**：SSE 是实时流，墙钟断言易 flaky。
  - **缓解**：契约测试断言 SSE **wire 帧格式 + 事件顺序**（注入确定事件序 → 断言 `id:`/`event:`/`data:` 帧 + 顺序），不断言墙钟到达时延（ADR-013：契约可确定性验证）；重放顺序断言 audit `id ASC` 单调序，纯确定性。SSE 与既有 long-poll 并存（add-only），long-poll 路径不退化兜底。
- **R3（中）audit log 重放仅覆盖 memory state-op 事件**：`audit_log` 当前持久 memory `pin`/`deprecate`/`soft_delete` 等，不持久 `indexing.*` 事件 → indexing 事件无 audit 源可重放。
  - **缓解**：task-26.2 重放 scope 限于 audit_log 已持久的 memory state-op 事件序，indexing 事件重放需 indexing 持久化源 [SPEC-DEFER:phase-future.indexing-event-persistence]；如实记录于 task-26.2 §8，AC2 以「memory state-op 重放 deterministic 可断言」满足。
- **R4（低）event-bus 分区分得过细反增复杂度**：partition channel 过多 → 订阅 fan-in 复杂。
  - **缓解**：默认不分区（保守默认，既有行为不变）；分区仅 opt-in 且限 `memory.*` / `indexing.*` 两命名空间粗粒度；deterministic 单测断言默认等价 + 配置生效。

## 8. Definition of Done

- 3 task spec（26.1-26.3）顶部 `**Status**: Done`
- §6 阶段级 AC1-5 全 `[x]`（受阻态按 ADR-013 如实记录，不伪造）
- 端到端 smoke 3 step 全 PASS（含受阻态如实标注）
- **ADR**：ADR-031 `Proposed → Accepted`（据真实非合成 FTS 往返 / VACUUM 回收 / SSE 帧契约 / 重放顺序契约）或据实测记录维持 + 文档化；ADR-021 / ADR-015 add-only Amendment 记录推进结果（不溯改正文，ADR-014 D5）
- **adapter**：§Phase 索引 Phase 26 `Draft → Done` + `Tasks 0 → 3`；§ADR 索引 ADR-031；§BDD 追加 phase-26 feature 行；ADR-021 预留兑现记录（events-replay + event-bus 配置）
- **release**：`docs/releases/v0.19.0-{evidence,artifacts}.md` + `RELEASE_NOTES.md` v0.19 段 + README v0.19 段
- **smoke**：`scripts/console_smoke.sh` v16（events SSE / trace FTS / event-bus 配置断言 + 既有 step 不退化）
- **follow-up**：indexing 事件重放 / SSE 背压 / FTS schema 迁移 / 分布式 event-bus 若延后则各 `[SPEC-DEFER:phase-future.*]` 留 backlog
