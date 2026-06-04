# ADR `031`: `observability-hardening`

**Status**: Accepted (2026-06-01；v0.19.0 closeout（task-26.3）据 task-26.1/26.2 真实非合成验证 ratify Proposed→Accepted，ADR-013：禁据合成 / 伪造 ratify。SSE live-server 端到端维度据真实受限「记录维持」不强 ratify。见 §Ratification。)
**Category**: 数据平面 / 可观测性 / 持久化 + 实时事件
**Date**: 2026-05-31
**Decided By**: 主 agent (ADR-012 自治)；tajiaoyezi ratification at v0.19.0 closeout
**Related**: ADR-021 (memory-event-bus-bridge — `EventBus broadcast::channel(1000)` + best-effort emit + `[SPEC-DEFER:phase-future.events-replay-from-audit]` Trade-off 标记) / ADR-002 (sqlite-tantivy-layered-storage — rusqlite bundled SQLite 分层) / ADR-010 (audit-cross-language-unification — `audit_log` 表) / ADR-013 (cli-data-plane-grpc-bridge — 禁伪造凭据红线) / ADR-015 (console-contract-v1-compatibility — add-only) / ADR-004 (local-first-privacy-baseline — 默认 0 新 dep / 0 network) / ADR-008 (core-library-selection — 依赖 add-only) / ADR-014 (D1-D5，第十七次激活) / Phase 16 (v0.9.0-backlog-completion — task-16.1 TraceStore SQLite 持久化 / task-16.2 events 真 long-poll) / Phase 26 (observability-hardening)

## Context

Phase 16 把可观测性的两条信号路径落地为持久化 + 实时两面：

1. **TraceStore SQLite 持久化（task-16.1）**：`core/src/data_plane/search_persist.rs::SqliteTracePersist` 把每次检索的 `RetrievalTrace`（prost 序列化 → base64 TEXT）写进 `<data_dir>/search_traces.db`，schema 在 `core/migrations/0015_search_traces.sql`（`search_traces(query_id, trace_json, workspace_id, ts_unix, created_at)` + `idx_search_traces_ts_desc`）。当前查询面仅 `get(query_id)`（主键命中）+ `list(limit)`（`ORDER BY ts_unix DESC`）+ `load_warm(n)`（暖启动恢复 LRU）。缺口：(a) **无按内容检索**——`RetrievalTrace.query` / 候选步骤等文本只能逐条 `get`，无法「按内容查 trace」；(b) **无清理路径**——`put` 是 `INSERT OR REPLACE`（同 query_id 替换），但不同 query_id 单调增长，`search_traces.db` 无界膨胀，无 VACUUM 回收。

2. **events 真 long-poll（task-16.2）**：`internal/consoleapi/handlers.go::handleEvents`（`GET /v1/observability/events`）把 `?wait=` 真传到 `internal/consoleapi/grpcclient/grpcclient.go::eventsClient.Recent`，经两阶段 long-poll（phase-1 block 首事件 / phase-2 短超时 drain）从 Rust `core/src/data_plane/events.rs::EventBus`（`tokio::sync::broadcast::channel(1000)`）拉事件。缺口：(a) **long-poll 非实时推送**——`handlers.go:655` 自标 `[SPEC-DEFER:task-future.consoleapi-sse]`，每轮 long-poll 要重新订阅 + phase-1/phase-2 之间 ~5ms 窗口的事件被两条流都漏掉（grpcclient.go:419-424 自述）；(b) **不重放历史事件**——ADR-021 Trade-off（`adr-021:115`）明记「不重放历史 audit log 到 EventBus；Console UI 拉 events 从订阅时刻开始（broadcast channel 不存历史）」并标 `[SPEC-DEFER:phase-future.events-replay-from-audit]`；(c) **EventBus 容量 / 分区固定**——`EventBus::new()` 硬编码 `broadcast::channel(1000)`（`events.rs:31`），ADR-021 D4 + Rollback path 已预见「memory 事件高频时挤占 indexing 事件 → 提容量或 partition channel」但未做（`adr-021:118` / `adr-021:153`）。

本 ADR 记录两条信号路径的硬化策略：trace 全文检索 + 周期 VACUUM；事件 SSE 实时推送 + 从 audit log 重放；event-bus 分区 / 容量 / drain 超时配置。全部 local-first（默认构建 0 新 dep / 0 network，ADR-004）。

## Decision

可观测性硬化采用 **复用 bundled SQLite 能力、契约可确定性验证、不破坏既有 long-poll / 22-endpoint 契约** 的策略：

### D1 — TraceStore 全文检索：rusqlite bundled FTS5 影子表 + 按内容查询（`search_persist.rs`，task-26.1）

`SqliteTracePersist` 加全文检索：在既有 `search_traces` 主表旁建 FTS5 影子虚表（`search_traces_fts`，索引 `query` + trace 可读文本投影），经 SQLite 触发器或 `put` 时同步写入；新增 `search_fts(query_text, limit)` 返回命中的 `QueryRecord` 序（按 FTS rank / `ts_unix`）。FTS5 是 `rusqlite = { features = ["bundled"] }`（`core/Cargo.toml:70`）bundled SQLite 自带的全文模块——**0 新依赖、0 network**（ADR-004 满足）。新增 migration（`core/migrations/0016_*.sql`，承 0015 编号序，`IF NOT EXISTS` 幂等）建 FTS 影子表 + 触发器；旧库 boot 时迁移幂等回填。`put` / `get` / `list` / `load_warm` 既有签名与语义不变（add-only 方法）。

### D2 — TraceStore 周期 VACUUM：阈值触发 + 显式 API（`search_persist.rs`，task-26.1）

`SqliteTracePersist` 加 `vacuum()`（执行 SQLite `VACUUM` 回收 page，重建紧凑库文件）+ 可选保留策略 `prune_older_than(ts_unix)`（按 `ts_unix < cutoff` 删行后 VACUUM）。触发口径：显式调用 + 可由调用方按行数 / 时间阈值驱动（不在 hot path 同步阻塞——VACUUM 需独占库，调用方在维护窗口或 boot 时调）。VACUUM 后 `get` / `list` / FTS 行为不变（仅回收空间）。deterministic 单测：插入 N 行 → 删除 → `vacuum()` → `row_count` 与库可用性断言（VACUUM 不破坏数据）。

### D3 — events SSE 实时推送：SSE framing endpoint（add-only，旁挂 long-poll，`internal/consoleapi`，task-26.2）

`internal/consoleapi` 加 SSE（Server-Sent-Events）实时推送 endpoint（如 `GET /v1/observability/events/stream`，`text/event-stream`），与既有 `GET /v1/observability/events` long-poll endpoint **并存**（add-only，ADR-015 D1；不改既有 long-poll 路由 / 契约）。SSE handler 经 `grpcclient` 订阅 Rust `EventBus` server-stream，把每个 `ObservabilityEvent` 编码为 SSE 帧（`id:` / `event:` / `data:` JSON）持续 flush，消除 long-poll「重订阅 + phase-1/phase-2 间隙漏事件」语义。SSE 帧编码 / 顺序契约用 deterministic 测试断言（注入确定事件序 → 断言 SSE wire 帧格式 + 顺序），不依赖实时 timing flakiness（ADR-013：契约可验证、不预判墙钟）。

### D4 — events 重放：从 audit log 重建漏失事件（`internal/consoleapi` + Rust，task-26.2）

兑现 ADR-021 `[SPEC-DEFER:phase-future.events-replay-from-audit]`（`adr-021:115`）：SSE / events 订阅支持 `?replay=` 参（如 `?last_event_id=` / `?since_ts=`），从持久化 audit log（`core/src/memoryops/audit.rs::AuditSink` 的 `audit_log` 表 — memory state-op 历史；`memory_pin` / `memory_deprecate` / `memory_soft_delete` 等）重建订阅时刻之前的 `ObservabilityEvent` 序，按 audit `id` / `timestamp` 升序回放，再无缝接续实时流。重放顺序 deterministic（audit `id ASC` 单调），用契约测试断言重放序 + 与实时流的拼接边界，不重复 / 不乱序。重放是 best-effort 历史补偿（audit 是持久主路径，event 是实时副路径，ADR-021 D1 不变）。

### D5 — event-bus 分区 + 容量 + drain 超时配置（`core/src/data_plane/events.rs` + consoleapi，task-26.3）

兑现 ADR-021 Rollback path「提容量或 partition channel」预见（`adr-021:153`）：`EventBus` 加可配置容量（`event-bus-capacity`，替换硬编码 `broadcast::channel(1000)`；`EventBus::with_capacity` 已存在 seam，`events.rs:35`）+ 可选按命名空间分区（`event-bus-partition`，如 `memory.*` / `indexing.*` 分独立 broadcast channel，避免 memory 高频挤占 indexing — ADR-021 D4 预见的丢事件场景）+ events drain 超时配置（`events-drain-timeout-config`，把 grpcclient phase-2 硬编码 `~100ms` drainTimeout 提为可配）。全部带保守默认（容量默认仍 1000 / 不分区 / drain 默认 ~100ms），既有行为默认不变（ADR-021 / task-16.2 默认语义保留）；配置仅 opt-in。

### D6 — 默认构建不变：0 新 dep + 0 network + 既有契约不退化（ADR-004 / ADR-015）

FTS5 / VACUUM 复用 rusqlite bundled SQLite（0 新 dep）；SSE 用 Go 标准库 `http.Flusher`（0 新 dep）；重放查既有 audit `audit_log` 表（0 新 dep）；event-bus 配置复用既有 `broadcast` / `with_capacity` seam（0 新 dep）。全程 0 network（ADR-004 local-first）。既有 22-endpoint 契约 / long-poll endpoint / `put`/`get`/`list`/`load_warm` 签名不退化（add-only，ADR-015 D1）。本 ADR 不引入新 trait 破坏 / 不改既有 proto enum 类型（`ObservabilityEvent.event_type` 仍 string）。

## Consequences

- **Positive**: trace 可按内容检索（FTS5）+ 周期 VACUUM 抑制 `search_traces.db` 无界膨胀；events 从 long-poll 升级到 SSE 实时推送，消除重订阅 + 间隙漏事件；ADR-021 两个 `[SPEC-DEFER]`（events-replay-from-audit）被兑现，订阅可从 audit log 重建漏失事件；event-bus 容量 / 分区 / drain 可配，兑现 ADR-021 Rollback path 预见；默认构建保持 0 新 dep + 0 network + 既有契约不退化（ADR-004 / ADR-015）。
- **Negative / open**: FTS5 影子表 + 触发器使 `put` 写放大（多写一份倒排索引——单用户 local-first 场景可接受，trace 写非高频热路径）；SSE 与 long-poll 两 endpoint 并存增加 console-api 表面（add-only，但维护两条事件通路）；audit-log 重放仅覆盖 memory state-op 类事件（audit_log 当前不持久 `indexing.*` 事件 → indexing 事件无 audit 源可重放，如实记录于 task-26.2 §8）；event-bus 分区若分得过细可能反增复杂度（默认不分区缓解）。
- **Ratification**: 本 ADR **Proposed**。task-26.1 真实 FTS 检索往返（index trace → FTS-search 命中）+ 真实 VACUUM 回收（deterministic 单测）+ task-26.2 真实 SSE 帧契约 + 重放顺序契约（contract 测试，无实时 flakiness）通过后，于 v0.19.0 closeout（task-26.3）据真实非合成验证 ratify Proposed→Accepted（ADR-013：禁据合成 / 伪造 ratify）；某维度受阻则据「已达维度 ratify + 受阻维度如实记录」处理，不强 ratify。
- **Follow-ups**: events 重放扩展到 `indexing.*` 类（需 indexing 事件持久化源）`[SPEC-DEFER:phase-future.indexing-event-persistence]`；SSE 多客户端 fan-out 压力 / 背压调优 `[SPEC-DEFER:phase-future.sse-backpressure-tuning]`；trace FTS5 跨库 schema 迁移 / 重建 `[SPEC-DEFER:phase-future.tracestore-fts-schema-migration]`；event-bus 跨进程 / 多节点广播（Kafka/NATS 类替换为 OOS per ADR-004 local-first）`[SPEC-DEFER:phase-future.distributed-event-bus]`。

## Ratification（v0.19.0 / task-26.3，2026-06-01）

v0.19.0 closeout（task-26.3）据 task-26.1/26.2/26.3 的**真实非合成验证** ratify `Proposed → Accepted`（ADR-013：禁据合成 / 伪造 ratify）。逐 D 项真实依据：

- **D1（trace FTS5 内容检索）— Accepted**：`core/src/data_plane/search_persist.rs::search_fts` + `core/migrations/0016_search_traces_fts.sql`（FTS5 影子虚表）真实落地；`cargo test -p contextforge-core --lib data_plane::search_persist` **10 passed / 0 failed**——含 term 的 trace 命中（TEST-26.1.1）/ miss 返空（TEST-26.1.2）/ 旧 0015-only 库 boot 解码 trace_json 回填 FTS 幂等（TEST-26.1.4）。FTS5 复用 rusqlite bundled SQLite（无 Cargo.toml 改动）。
- **D2（周期 VACUUM）— Accepted**：`vacuum()` + `prune_older_than(cutoff)` 真实落地；TEST-26.1.3 插入 N 行 → prune 旧行 → vacuum → `row_count` 与剩余行一致 + 保留行 `get`/`list`/`search_fts` 仍正确（VACUUM 不破坏数据）。
- **D3（events SSE 实时推送）— Accepted（契约层）**：`internal/consoleapi::handleEventsStream`（`text/event-stream` + `http.Flusher`，add-only 旁挂 long-poll）真实落地；`go test ./internal/consoleapi/...` SSE 帧契约（`id:`/`event:`/`data:` + 顺序 + data 合法 JSON）经注入确定事件序断言（TEST-26.2.1），不依赖墙钟。**真实 daemon 起服 SSE 端到端（live curl）据真实受限「记录维持」**（`[SPEC-DEFER:phase-future.sse-live-server-e2e]`；CI 无 running daemon，ADR-013 不伪造 live 通过）。
- **D4（从 audit log 重放）— Accepted**：`core/src/data_plane/events.rs::replay_events_from_audit` + `EventsServer.subscribe` since_ts 接线 + proto add-only `since_ts`/`last_event_id` 真实落地；`cargo test -p contextforge-core --lib data_plane::events` 重放序 id ASC + ADR-021 D3 映射 + since_ts 过滤（TEST-26.2.3 Rust）+ Go SSE 拼接边界以 event_id 去重（TEST-26.2.3 Go）。兑现 ADR-021 `[SPEC-DEFER:phase-future.events-replay-from-audit]`。重放 scope 仅 audit 已持久 memory state-op，`indexing.*` 重放 `[SPEC-DEFER:phase-future.indexing-event-persistence]` 如实延后。
- **D5（event-bus 容量/分区/drain 配置）— Accepted**：`EventBus::from_config(EventBusConfig::from_env())`（容量 `CF_EVENT_BUS_CAPACITY` + 分区 `CF_EVENT_BUS_PARTITION`）+ grpcclient `CONSOLE_EVENTS_DRAIN_TIMEOUT` 真实落地；events 6/6（默认单 channel 等价 / 容量可配 / 分区 memory.·indexing. 路由隔离，TEST-26.3.1）+ drain 5/5 子例。复用 `with_capacity` seam，兑现 ADR-021 Rollback path「提容量 / partition channel」预见（`adr-021:153`）。
- **D6（默认 0 新 dep + 0 network + 既有契约不退化）— Accepted**：FTS5/VACUUM 用 rusqlite bundled、SSE 用 Go 标准库 `http.Flusher`、重放查既有 `audit_log`、event-bus 配置复用 `broadcast`/`with_capacity` seam——`cargo test --workspace` + `go test ./...` 全 PASS、既有 long-poll endpoint + 22-endpoint 契约 + `put`/`get`/`list`/`load_warm` 签名不退化（add-only）。

证据见 `docs/releases/v0.19.0-evidence.md`。

### ADR-021 / ADR-015 add-only Amendment（推进结果，不溯改正文，ADR-014 D5）

- **ADR-021（memory-event-bus-bridge）**：本 phase 兑现其两处预留——(a) `[SPEC-DEFER:phase-future.events-replay-from-audit]`（`adr-021:115`）由 task-26.2 的 `replay_events_from_audit` + SSE `?since_ts=` 重放兑现；(b) Rollback path「memory 高频挤占 indexing → 提容量或 partition channel」（`adr-021:153`）由 task-26.3 的 `event-bus-capacity` + `event-bus-partition` 兑现。以 add-only Amendment 记录推进结果，不溯改 ADR-021 D1-D4 正文（best-effort emit / broadcast(1000) 默认语义仍成立，默认未开启分区）。
- **ADR-015（console-contract-v1-compatibility）**：SSE endpoint `GET /v1/observability/events/stream` 为 add-only 新增（既有 long-poll endpoint + 22-endpoint 契约 + `Recent(limit, wait)` 签名不动），按 ADR-015 D1 add-only 思想记录，不溯改其正文。

## Amendment (Phase 33 / v0.26.0, 2026-06-03 — add-only, 正文不溯改)

Phase 33（ADR-038 D3）以 add-only 方式补 `indexing.*` 事件持久化 + replay，并据实校正 events-drain-timeout 为 verify-only，**不溯改 D1-D6 正文 + 既有 Amendment 正文**（ADR-014 D5）：

- **indexing.* 事件持久化 + replay（兑现 D4 的 `[SPEC-DEFER:phase-future.indexing-event-persistence]`）**：task-33.3（PR #220）此前 `replay_events_from_audit` 仅重放 audit 已持久的 memory state-op，`indexing.*` 无持久源。本 phase 加 **add-only migration `0019_indexing_events`**（专用表，`AuditLogEntry` 无 job_id/processed/total 无法承载）+ `SqliteIndexingEventStore`（append/list，id ASC）+ 四 emit 点（`index_session_backend.rs` progress/error×2/cancelled）best-effort 持久写（不替换既有 `eb.send` 广播）+ `events::indexing_rows_to_pb_events` 纯 mapper（id ASC 重建 indexing.* PbEvent，真实 job_id/processed/total，确定性 `evt-idx-{id}`）。mapper + persist round-trip 单测 🟢（`test_33_3_1` / `test_33_3_2`）；端到端 restart-then-replay（须 running daemon + job runner）🟡 honest-defer `[SPEC-DEFER:phase-future.indexing-replay-e2e]`（ADR-013 不预填）。
- **TraceStore 多 workspace 隔离（承 ADR-016 trace-isolation）**：`GetSearchTraceRequest`/`ListQueriesRequest` add-only `workspace_id=2` + `SqliteTracePersist` get/list/search_fts + in-mem TraceStore + handler `WHERE workspace_id` filter（empty=aggregate-all byte-equiv，ADR-004 back-compat）。SQL/handler 单测 🟢（`test_33_3_3` / `test_33_3_4`）；e2e console 隔离 🟡 `[SPEC-DEFER:phase-future.tracestore-multi-workspace-strict]`。
- **events-drain-timeout = VERIFY-ONLY 校正（承 D5 交付）**：`CONSOLE_EVENTS_DRAIN_TIMEOUT`（`grpcclient.go` `drainTimeoutFromEnv`，默认 100ms）系 Phase 26 D5 既交付，本 phase 据实从「add」改「verify-only」（镜像 Phase 31 event-bus-partition 校正），引证既有 `TestDrainTimeoutFromEnv`（5 子用例绿），不重实现。

依赖变更：migration `include_str!` + proto add-only field 0 新 dep。详见 ADR-038 Ratification D3 + `docs/releases/v0.26.0-evidence.md`。

## Amendment (Phase 35 / v0.28.0, 2026-06-04 — add-only, 正文不溯改)

Phase 35（ADR-040 observability-hardening）承本 ADR 确立的 **stderr / best-effort surfacing 方向**，把它从 TraceStore / events / event-bus 延伸到**热路径中被静默吞掉的真实错误**，**不溯改 D1-D6 正文 + 既有 Phase 33 Amendment 正文**（ADR-014 D5）：

- **热路径静默错误显式化（承 best-effort surfacing 方向）**：task-35.1（PR #229）把 `core/src/jobs/index_session_backend.rs` 的 **4 处** `store.append`（progress/index-error/commit-error/cancelled）`let _ =` 改为 `if let Err(persist_err) { eprintln!("WARN indexing-event persist failed …: {persist_err}") }`（SQLite persist 失败=磁盘满/锁，不再无声吞掉；best-effort 保留，不阻断 indexing）+ `core/src/retriever/mod.rs:415` 的 `Err(_) => continue`（Tantivy/SQLite desync 静默跳过命中）改为 `Err(e) => { eprintln!("WARN retriever: … desync …"); continue }`（skip 行为保留）；`eb.send` 各处保留 as-is（broadcast 无订阅者返 Err 是正常态 intentional，本 ADR D5 既定 best-effort emit 不变）。task-35.2（PR #230）把 `cmd/contextforge/main.go` `setVectorEnv` 的 `config.Load`/`os.Setenv` 失败改为 `fmt.Fprintf(os.Stderr)` 显式化（`errors.Is(os.ErrNotExist)` 守护 missing 静默），镜像 task-31.3（ADR-036）确立的 Go stderr audit-surfacing pattern。
- **不引入新 metrics facility**：core 仅 `eprintln!` / Go 仅 `fmt.Fprintf(os.Stderr)`，severity 为消息前缀（WARN/INFO），无 severity / metrics framework——structured metrics/counter facility honest-defer `[SPEC-DEFER:phase-future.observability-metrics-facility]`（承本 ADR「轻量 stderr surfacing」基线）。
- **7→3-4 grounding 校正（ADR-013）**：survey 7 候选据实收敛 3-4 真静默；4 处 DROP/LEAVE（`search.rs:109` already-surfaced / `mcpadapter/server.go:298` task-31.3 already-done / `allowlist.go:31` 有意 POSIX-only / `eb.send:193` 有意 no-subscribers），不改代码；`memstore.go:579` nil-sink = honest non-issue（生产 sink 总接线）`[SPEC-DEFER:phase-future.memstore-degraded-observability-warn]`。

依赖变更：0 新 dep（add-only eprintln!/Fprintf 旁路，observability-only best-effort 不转 fail-fast）。详见 ADR-040 Ratification + `docs/releases/v0.28.0-evidence.md`。
