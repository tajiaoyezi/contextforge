# Task `26.2`: `events-sse-push-and-replay — internal/consoleapi 加 SSE 实时推送 endpoint（text/event-stream，旁挂既有 long-poll，add-only）+ 从 audit log 重放订阅前漏失事件 + deterministic 契约测试（SSE 帧编码 + 重放顺序，不依赖实时 timing）`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 26 (observability-hardening)
**Dependencies**: task-16.2（`handleEvents` 真 long-poll + `grpcclient.eventsClient.Recent` 两阶段）/ task-11.4（`EventBus broadcast::channel(1000)` + `EventsService.Subscribe` server-stream）/ ADR-021（memory-event-bus-bridge — `audit_log` 重放源 + `[SPEC-DEFER:phase-future.events-replay-from-audit]` `adr-021:115`）/ ADR-031 D3/D4（SSE + 重放）/ ADR-015（console-contract-v1-compatibility，SSE add-only）/ ADR-004（local-first 0 新 dep / 0 network）/ ADR-013（禁伪造凭据）/ ADR-014 D1-D5（第十七次激活）

## 1. Background

Phase 16 task-16.2 把 `internal/consoleapi/handlers.go::handleEvents`（`GET /v1/observability/events`）的 `?wait=` 真传到 `internal/consoleapi/grpcclient/grpcclient.go::eventsClient.Recent`，经两阶段 long-poll（phase-1 block 首事件用 `wait` 作 ctx timeout / phase-2 短超时 `~100ms` drain）从 Rust `core/src/data_plane/events.rs::EventBus`（`tokio::sync::broadcast::channel(1000)`）拉事件，`EventsService.Subscribe` server-stream 出。

两块缺口由代码自标：

- **long-poll 非实时推送**：`handlers.go:655` 显式 `[SPEC-DEFER:task-future.consoleapi-sse]`——「Console HTTPAdapter v1.0 expects 200 + maybe-empty array (NOT 204)」是 long-poll 契约；`grpcclient.go:419-424` 自述 phase-1 返回与 phase-2 重订阅之间 `~5ms` 窗口的事件被两条流都漏掉（「Events emitted in the ~5ms gap ... are missed by both streams」）。每轮 long-poll 重新订阅 broadcast channel。
- **不重放历史事件**：ADR-021 Trade-off（`adr-021:115`）明记「不重放历史 audit log 到 EventBus；Console UI 拉 events 从订阅时刻开始（broadcast channel 不存历史）；想看历史需查 audit log」并标 `[SPEC-DEFER:phase-future.events-replay-from-audit]`。`core/src/data_plane/events.rs` 的 broadcast channel 不存历史（新订阅者只收订阅后事件 — `events.rs:48` `subscribe`）。

持久 audit 历史在 `core/src/memoryops/audit.rs::AuditSink`（`audit_log` 表，`record()` 写 + `list()` 返 `id ASC` 序），持久 memory state-op（`memory_pin`/`memory_unpin`/`memory_deprecate`/`memory_soft_delete` 等 `AuditOperation`），是 ADR-021 D1 桥接到 `EventBus` 的同源审计路径。

ADR-031 D3/D4 记录硬化策略：SSE 实时推送（旁挂 long-poll，add-only）+ 从 audit log 重放（兑现 ADR-021 预留）。SSE 用 Go 标准库 `net/http` `http.Flusher`——0 新依赖（ADR-004）。本 task 落这两块 + deterministic 契约测试（断言 SSE wire 帧 + 重放顺序，不依赖实时 timing flakiness，ADR-013）。

## 2. Goal

`internal/consoleapi` 加 SSE 实时推送 endpoint（如 `GET /v1/observability/events/stream`，`Content-Type: text/event-stream`），经 `grpcclient` 订阅 Rust `EventsService.Subscribe`，把每个 `ObservabilityEvent` 编码为 SSE 帧（`id:` event_id / `event:` event_type / `data:` JSON）经 `http.Flusher` 持续 flush，消除 long-poll 重订阅 + phase 间隙漏事件；与既有 `GET /v1/observability/events` long-poll endpoint 并存（add-only，ADR-015 D1，既有路由 / 契约不动）。SSE / events 订阅支持 `?since_ts=` / `?last_event_id=` 重放参——从持久 `audit_log`（`AuditSink::list()` audit `id ASC`）重建订阅时刻之前的 `ObservabilityEvent` 序（memory state-op 类），按 audit `id` 升序回放，再无缝接续实时流。SSE 帧编码 + 重放顺序经 deterministic 契约测试可断言（注入确定事件序 → 断言 SSE wire 帧格式 + 顺序 / 重放 audit `id ASC` 升序 + 拼接边界不重复不乱序），不依赖墙钟到达时延。既有 long-poll endpoint + 22-endpoint 契约不退化（add-only）。≥2 Go contract test + 重放 Rust 查询面单测全 PASS。默认构建 0 新依赖（SSE 用标准库 `http.Flusher`）/ 0 network。D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **修改 `internal/consoleapi/router.go`**：注册 SSE 路由（如 `mux.HandleFunc("GET /v1/observability/events/stream", handleEventsStream(deps))`），与既有 `GET /v1/observability/events`（`router.go:32`）并存（add-only）。
- **修改 `internal/consoleapi/handlers.go`**：加 `handleEventsStream(deps)`——set `Content-Type: text/event-stream` + `Cache-Control: no-cache` header，经 `deps.Events`（或新增 streaming 订阅入口）持续读事件并以 SSE 帧写 `http.ResponseWriter` + `Flush()`；client 断开（`r.Context().Done()`）时退出 + 释放 gRPC 订阅（不泄漏 goroutine，承 task-16.2 goroutine-leak 测试思路）。`?since_ts=` / `?last_event_id=` 重放参 → 先回放 audit 历史帧，再接续实时流。
- **修改 `internal/consoleapi/grpcclient/grpcclient.go` + `internal/consoleapi/types.go`**：加 streaming 订阅入口（如 `EventsClient.Stream(ctx, opts) (<-chan contractv1.ObservabilityEvent, error)` 或等价），复用既有 `EventsService.Subscribe` server-stream（不改既有 `Recent(limit, wait)` long-poll 签名）；重放参经此入口下传。
- **新增 / 修改 Rust 重放查询面（`core/src/data_plane/events.rs` 或邻接模块）**：从 `core/src/memoryops/audit.rs::AuditSink::list()`（audit `id ASC`）重建 memory state-op 类 `ObservabilityEvent` 序（按 `adr-021:79-91` D3 字段映射约定：`event_type` `memory.*` / `severity` `info` / `payload_json` `{memory_id, op}` 等），供 `Subscribe` 在 `?since_ts=` 时先回放再接实时。
- **新增同源 Go contract tests（`internal/consoleapi/events_test.go` 或邻接，承既有 events_test.go pattern）**：(a) SSE 帧编码——注入确定事件序（如 immediateEventsClient 风格测试替身）→ 断言响应体含正确 SSE 帧（`id:`/`event:`/`data:` 行 + 空行分隔）+ 顺序；(b) 重放顺序——给定 audit 历史 + 实时事件 → 断言重放段按 audit `id ASC` 升序 + 与实时流拼接边界不重复 / 不乱序，deterministic（不 `time.Sleep` 断言墙钟）。
- **新增 Rust 重放查询面单测**：audit `id ASC` 重建 `ObservabilityEvent` 序的顺序 + 字段映射断言（deterministic）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **既有 `handleEvents` long-poll + `eventsClient.Recent` 两阶段语义** [SPEC-OWNER:task-16.2-events-real-long-poll]：本 task 旁挂 SSE（add-only），不改既有 long-poll 路径 / 契约。
- **`EventBus` 容量 / 分区 / drain 超时配置** [SPEC-OWNER:task-26.3-closeout-v0.19.0]：本 task 用既有 `EventBus`；容量 / 分区配置在收口 task。
- **TraceStore FTS / VACUUM** [SPEC-OWNER:task-26.1-tracestore-fts-and-vacuum]：本 task 仅做 events 实时面。
- **重放扩展到 `indexing.*` 类事件**（需 indexing 事件持久化源；`audit_log` 当前仅持久 memory state-op）[SPEC-DEFER:phase-future.indexing-event-persistence]：本 task 重放 scope 限于 audit_log 已持久的 memory state-op 事件序。
- **SSE 多客户端 fan-out 背压 / 压力调优** [SPEC-DEFER:phase-future.sse-backpressure-tuning]：本 task 落单客户端 SSE 帧契约 + 重放顺序；多客户端背压属后续。
- **真实 daemon 起服 SSE 端到端 curl 验证**（需 live server）[SPEC-DEFER:phase-future.sse-live-server-e2e]：本 task 用 contract 层 deterministic 测试（注入事件序）断言 SSE 帧 + 重放顺序；真实起服端到端 smoke 由 task-26.3 据 console_smoke 合规环境评估，受阻则诚实 stop-condition（ADR-013，不伪造 live 通过）。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`internal/consoleapi/handlers.go::handleEventsStream`**：本 task 新增的 SSE handler（旁挂既有 `handleEvents`）。
- **`internal/consoleapi/grpcclient/grpcclient.go::eventsClient`**：task-16.2 long-poll client，本 task 加 streaming 订阅入口（不改既有 `Recent`）。
- **`core/src/data_plane/events.rs::EventsServer` / `EventBus`**：task-11.4 broadcast 事件源，本 task 在 `Subscribe` 路径加 audit 重放接续。
- **`core/src/memoryops/audit.rs::AuditSink`**：`audit_log` 持久历史（`list()` `id ASC`），本 task 作 events 重放源。
- **下游 task-26.3**：closeout 据本 task SSE / 重放能力评估 smoke v16 断言（真实起服 SSE 或如实标 stop-condition）。

## 5. Behavior Contract

### 5.1 Required Reading

- `internal/consoleapi/handlers.go`（`handleEvents` long-poll + `:655` `[SPEC-DEFER:task-future.consoleapi-sse]` + `parseWaitParam` / `parseLimitParam`）
- `internal/consoleapi/router.go`（路由表 + `:32` `GET /v1/observability/events` 注册点 — SSE 旁挂处）
- `internal/consoleapi/grpcclient/grpcclient.go`（`eventsClient.Recent` 两阶段 long-poll + `:419-424` phase 间隙漏事件自述 + `:434` 签名 + goroutine 释放 pattern）
- `internal/consoleapi/types.go:86`（`EventsClient interface { Recent(limit, wait) }` — streaming 入口加在此）+ `internal/consoleapi/events_test.go`（`immediateEventsClient` / `sleepingEventsClient` 测试替身 pattern — contract 测试风格）
- `core/src/data_plane/events.rs`（`EventBus broadcast::channel(1000)` + `subscribe` + `EventsServer.Subscribe` server-stream + `build_*_event` 字段填充 pattern）
- `core/src/memoryops/audit.rs::AuditSink`（`audit_log` schema + `record()`/`list()` `id ASC` + `AuditOperation` 枚举 `MemoryPin`/`MemoryUnpin`/`MemoryDeprecate`/`MemorySoftDelete`）
- `docs/decisions/adr-021-memory-event-bus-bridge.md`（D1 桥接 + D3 字段映射 `:79-91` + Trade-off `:115` 重放预留 + D4 best-effort）+ `docs/decisions/adr-031-observability-hardening.md`（D3 SSE + D4 重放）
- `docs/decisions/adr-015-console-contract-v1-compatibility.md`（add-only）+ `docs/decisions/adr-004-local-first-privacy-baseline.md`（0 新 dep / 0 network）+ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造）

### 5.2 关键设计 — SSE framing + audit 重放接续

- **SSE endpoint（add-only，旁挂 long-poll）**：`GET /v1/observability/events/stream` set `Content-Type: text/event-stream` + `Cache-Control: no-cache`，经 `grpcclient` streaming 订阅 `EventsService.Subscribe`，每事件写 SSE 帧并 `http.Flusher.Flush()` 持续推；client 断开（`r.Context().Done()`）退出 + 释放 gRPC 订阅。既有 `GET /v1/observability/events` long-poll endpoint + 路由 + 22-endpoint 契约不动（ADR-015 D1 add-only）。
- **SSE 帧编码契约**：每事件一帧——`id: <event_id>\n` + `event: <event_type>\n` + `data: <JSON ObservabilityEvent>\n` + `\n`（空行结束帧）。`id:` 用 `event_id`（供客户端 `Last-Event-ID` 续传）。帧格式 + 顺序经注入确定事件序的 contract 测试断言（wire-level，不依赖墙钟）。
- **audit 重放接续**：`?since_ts=<ts>` / `?last_event_id=<id>` 时，先经 Rust 从 `AuditSink::list()`（`id ASC`）筛 `since_ts` 之后的 memory state-op 条目，按 `adr-021:79-91` D3 映射重建 `ObservabilityEvent`（`event_type` `memory.*` / `severity` `info` / `source` `contextforge-core` / `payload_json` `{memory_id, op}`），按 audit `id` 升序回放为 SSE 帧，再接续实时 broadcast 流——拼接边界以 `event_id` / `ts_unix` 去重（不重复回放已实时收到的事件）。
- **重放 scope（诚实记录）**：`audit_log` 持久 memory state-op（`AuditOperation` 含 `MemoryPin`/`MemoryUnpin`/`MemoryDeprecate`/`MemorySoftDelete`），故重放覆盖 `memory.*` 事件；`indexing.*` 事件无 audit 持久源 → 不在重放 scope [SPEC-DEFER:phase-future.indexing-event-persistence]，如实记录于 §8。
- **ADR-013（契约可验证、不预判墙钟）**：SSE 帧契约 + 重放顺序是 deterministic（注入确定事件序 → 断言 wire 帧 + audit `id ASC` 顺序）；真实 daemon 起服 SSE 端到端属 live-server 验证 [SPEC-DEFER:phase-future.sse-live-server-e2e]，由 task-26.3 据合规环境评估，受阻则诚实 stop-condition（不伪造 live 通过）。

### 5.3 不变量

- 默认构建 0 新依赖（SSE 用 Go 标准库 `net/http` `http.Flusher`；重放查既有 `audit_log`；ADR-004 / ADR-008）+ 0 network。
- 既有 `handleEvents` long-poll endpoint + `eventsClient.Recent(limit, wait)` 签名 + 路由 + 22-endpoint 契约逐字节不退化（SSE 是 add-only 新增，ADR-015 D1）。
- SSE 帧格式确定：每事件 `id:`/`event:`/`data:` + 空行；`data:` 是合法 JSON `ObservabilityEvent`（既有 contractv1 struct，字段不变）。
- 重放顺序确定：audit `id ASC` 升序回放；拼接边界以 `event_id` / `ts_unix` 去重，不重复 / 不乱序。
- client 断开释放 gRPC 订阅（不泄漏 goroutine，承 task-16.2 goroutine-leak 防护思路）。
- 重放仅覆盖 audit_log 已持久的 memory state-op 事件（不新引入持久字段；indexing 事件重放受 audit 源限制如实延后）。

## 6. Acceptance Criteria

- [x] **AC1**: SSE 实时推送 endpoint（`text/event-stream`）注入确定事件序 → 响应体含正确 SSE 帧（`id:`/`event:`/`data:` 行 + 空行分隔）+ 顺序与注入序一致；`data:` 是合法 JSON `ObservabilityEvent` — verified by **TEST-26.2.1**
- [x] **AC2**: SSE endpoint 是 add-only——既有 `GET /v1/observability/events` long-poll endpoint + 路由 + `eventsClient.Recent(limit, wait)` 签名不退化；既有 22-endpoint 契约 + events_test 不退化 — verified by **TEST-26.2.2**
- [x] **AC3**: 从 audit log 重放——给定 audit 历史（memory state-op `id ASC`）+ `?since_ts=` → 重放段按 audit `id` 升序重建 `ObservabilityEvent`（`event_type` `memory.*` 等 D3 映射）+ 与实时流拼接边界以 `event_id`/`ts_unix` 去重不重复不乱序，deterministic（不依赖墙钟） — verified by **TEST-26.2.3**
- [x] **AC4**: client 断开释放 gRPC 订阅 + 重放 scope 诚实——`r.Context().Done()` 退出 SSE handler 不泄漏 goroutine；重放覆盖 audit 已持久 memory state-op，indexing 事件重放如实延后（`[SPEC-DEFER:phase-future.indexing-event-persistence]`） — verified by **TEST-26.2.4**
- [x] **AC5**: 既有不退化 + 0 新依赖 — 默认 `go test ./...` 全 PASS + SSE 用标准库 `http.Flusher`（0 新 dep）；`cargo test --workspace`（重放查询面单测）不退化；D2 lint `--touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-26.2.5** + §10 实测

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-26.2.1 | SSE 帧编码 — 注入确定事件序断言 `id:`/`event:`/`data:` 帧 + 顺序 + data JSON 合法 | `internal/consoleapi/events_test.go` | Done |
| TEST-26.2.2 | SSE add-only — 既有 long-poll endpoint + Recent 签名 + 22-endpoint 不退化 + nil-safe 503 | `internal/consoleapi/events_test.go` | Done |
| TEST-26.2.3 | audit 重放顺序 — `id ASC` 升序重建 + 拼接边界去重不重复不乱序 + `?since_ts=` 透传（deterministic） | `internal/consoleapi/events_test.go` + `core/src/data_plane/events.rs`（`mod tests`） | Done |
| TEST-26.2.4 | client 断开释放订阅不泄漏 goroutine + 重放 scope 诚实（memory state-op 覆盖） | `internal/consoleapi/events_test.go` | Done |
| TEST-26.2.5 | 默认 `go test ./...` + `cargo test --workspace` 0 failed + 0 新依赖 + D2 lint 0 未标注 | 全 Go + Rust + `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（高）SSE 实时测试 timing flakiness**（承 phase-26 §7 R2）：SSE 是实时流，墙钟断言易 flaky。
  - **缓解**：contract 测试断言 SSE **wire 帧格式 + 事件顺序**（注入确定事件序 → 断言 `id:`/`event:`/`data:` 帧 + 顺序），不断言墙钟到达时延（ADR-013 契约可验证）；重放顺序断言 audit `id ASC` 单调序纯确定性。SSE 与既有 long-poll 并存（add-only），long-poll 路径不退化兜底。stop-condition：真实 daemon 起服 SSE 端到端 [SPEC-DEFER:phase-future.sse-live-server-e2e] 由 task-26.3 据合规环境评估，受阻则诚实记录不伪造 live 通过。
- **R2（中）audit log 重放仅覆盖 memory state-op 事件**（承 phase-26 §7 R3）：`audit_log` 不持久 `indexing.*` 事件。
  - **缓解**：重放 scope 限 audit_log 已持久的 memory state-op（`AuditOperation` `MemoryPin`/`Unpin`/`Deprecate`/`SoftDelete`）；indexing 事件重放需 indexing 持久化源 [SPEC-DEFER:phase-future.indexing-event-persistence]，如实记录；AC3 以「memory state-op 重放 deterministic 可断言」满足，不伪造 indexing 重放。
- **R3（中）broadcast channel lag 致重放与实时流接续边界丢事件**：`broadcast::channel(1000)` 满时 drop oldest（task-11.4 既有 `RecvError::Lagged`）。
  - **缓解**：重放从持久 `audit_log` 补订阅前历史 + 实时流接续；拼接边界以 `event_id`/`ts_unix` 去重；channel lag 是 task-11.4 既有行为，task-26.3 加 `event-bus-capacity` 配置缓解 [SPEC-OWNER:task-26.3-closeout-v0.19.0]。AC3 断言重放 + 实时拼接的确定序，lag 缓解归 26.3。
- **R4（低）SSE 与 long-poll 双 endpoint 维护表面**：两条事件通路并存。
  - **缓解**：SSE 是 add-only 新增 endpoint，long-poll 路径 / 契约不动（ADR-015 D1）；两路共享同一 `EventBus` 源 + 同一 contractv1 `ObservabilityEvent` 编码，维护表面增量小。

## 9. Verification Plan

```bash
# Go：SSE 帧契约 + 重放顺序 + add-only 不退化（deterministic，不依赖墙钟）
go test ./internal/consoleapi/... -run 'Events|SSE|Replay|Stream' -v

# 全 Go 默认不退化（22-endpoint 契约 + 既有 long-poll）
go test ./...

# Rust：audit 重放查询面单测 + 默认不退化（0 新依赖）
cargo test -p contextforge-core data_plane::events
cargo test --workspace

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **Status**: Done（2026-06-01）。
- **完成日期**：2026-06-01。
- **改动文件**：
  - `proto/contextforge/console_data_plane/v1/console_data_plane.proto`——`SubscribeEventsRequest` add-only `int64 since_ts = 3` + `string last_event_id = 4`（既有 field 1/2 不动；regen Go pb.go + Rust prost）。
  - `core/src/data_plane/events.rs`——`replay_events_from_audit(entries, since_ts)`（从 `AuditSink::list()` id ASC 重建 memory state-op `ObservabilityEvent` 序）+ `audit_op_str_to_event` 字符串映射 + `EventsServer.subscribe` 接线（since_ts>0 先回放 audit 再接 live；先 subscribe 后建回放批避免漏 live）+ Rust 重放序单测。
  - `internal/consoleapi/types.go`——`StreamOptions` + `EventsStreamer` 接口 + `Deps.EventsStream`（optional）。
  - `internal/consoleapi/handlers.go`——`handleEventsStream`（`text/event-stream` 帧 + `http.Flusher` 持续推 + event_id 拼接边界去重 + `r.Context().Done()` 释放；nil streamer → 503）+ `parseSinceTSParam` / `streamLastEventID`。
  - `internal/consoleapi/router.go`——add-only `GET /v1/observability/events/stream` 路由（既有 long-poll endpoint 不动）。
  - `internal/consoleapi/grpcclient/grpcclient.go`——`eventsClient.Stream`（订阅 `Subscribe` 转 channel，ctx 释放）+ `Client.EventsStream()` 访问器。
  - `internal/cli/console_api_serve.go`——gRPC Deps 接 `EventsStream: cli.EventsStream()`（fallback/degraded 留 nil → SSE 503）。
  - `internal/consoleapi/events_test.go`——4 SSE 契约测试 + helper（fakeStreamer / parseSSEFrames）。
  - `core/tests/{data_plane_integration,search_real_retriever}.rs`——`SubscribeEventsRequest` 字段补全（add-only 字段连带）。
  - 0 新依赖（SSE 用 Go 标准库 `http.Flusher`；重放查既有 `audit_log`）。
- **commit 列表（RED→GREEN）**：
  - RED `test(events): TEST-26.2.1~26.2.4 RED`（proto+regen + replay `todo!()` + handler 返 501 + 4 Go 测试 + Rust 测试；Rust replay panic + 4 Go SSE FAIL）。
  - GREEN `feat(events): events SSE 实时推送 + 从 audit log 重放`（replay 实现 + subscribe 接线 + 真实 SSE handler + grpcclient.Stream）。
- **§9 Verification 实测结果（ADR-013 真实非合成）**：
  - `cargo test -p contextforge-core --lib data_plane::events` → **2 passed**（replay 序 + keepalive 不退化）。
  - `go test ./internal/consoleapi/...` → consoleapi + grpcclient 全 PASS（4 SSE 契约：帧编码 / add-only nil-safe / 重放拼接去重 + since_ts 透传 / 断开释放）。
  - `cargo test --workspace` + `go test ./...` → 0 failed（既有 long-poll + 22-endpoint + keepalive 不退化）。
  - D2 lint 本机 scoped 触及行 0 未标注命中（CI spec-lint 权威）。
- **设计取舍**：(1) SSE 帧 `id:`/`event:`/`data:`（data = JSON `ObservabilityEvent`）+ 空行结束帧；(2) 重放在 Rust `subscribe` 内**先 subscribe live channel 后建 audit 回放批**，确保回放与 live 之间无 live 事件丢失；(3) 重放 event_id = `evt-audit-{audit_id}` 确定性，SSE handler 以 event_id 去重拼接边界（seen set 随会话内 distinct id 增长，单用户 local-first 可接受）；(4) **重放 scope 诚实**：仅覆盖 audit_log 已持久的 memory state-op（pin/unpin/deprecate/soft_delete），`indexing.*` 无 audit 持久源 → 重放延后 `[SPEC-DEFER:phase-future.indexing-event-persistence]`；(5) **真实起服 SSE 端到端**（running daemon curl）属 live-server 验证，本 task 用 contract 层 deterministic 测试（注入事件序 + 构造 audit）断言帧 + 重放顺序，live e2e 诚实延后 `[SPEC-DEFER:phase-future.sse-live-server-e2e]`，由 task-26.3 据合规环境评估，不伪造 live 通过。
- **剩余风险 + 下游影响**：`indexing.*` 重放 / SSE 多客户端背压 `[SPEC-DEFER:phase-future.sse-backpressure-tuning]` / event-bus 容量缓解归 task-26.3；SSE handler dedup seen-set 长会话内存随 distinct event_id 增长（缓解归背压调优）。
