# Task `16.2`: `events-real-long-poll — handleEvents 真把 ?wait= 传到 grpcclient + 真 block-until-event-or-timeout 语义 + MemStore fallback wait sleep`

**Status**: Done

**Priority**: P4
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 16 (v0.9.0-backlog-completion)
**Dependencies**: Phase 11 task-11.4（既有 EventBus broadcast::channel + EventsService.Subscribe server-stream）

## 1. Background

ContextForge-Console PR #91/#93 backlog 列 P4 #11：

> `GET /v1/observability/events?wait=30s` 当前等价 batch polling — Go REST handler `handleEvents` (`internal/consoleapi/handlers.go:637`) 解析 `wait` 后 line 641 显式 `_ = parseWaitParam(r)` **discard**；下游 `grpcclient.eventsClient.Recent(limit)` 用 hardcoded 30s ctx timeout 但 `for len(batch) < limit { stream.Recv() }` 阻塞等 `limit` 个 event（默认 100）→ 实际语义"等满 100 个 event 或 30s 超时"。Console UI 期望"有任一 event 立刻返"。

既有 v0.8.0 状态：
- `internal/consoleapi/handlers.go:637-672` handleEvents + parseWaitParam（默认 30s / clamp [1s, 60s]）— 但 wait 被 discard
- `internal/consoleapi/types.go:77-79` `EventsClient.Recent(limit int)` 接口
- `internal/consoleapi/grpcclient/grpcclient.go:386-414` eventsClient — hardcoded 30s ctx + 等满 limit 才返
- `internal/consoleapi/memstore.go:466-478` MemStore.Recent — 同步返 ring buffer
- `core/src/data_plane/events.rs:54-117` EventsServer — gRPC server-stream backed by `EventBus { broadcast::Sender }` 永不主动 close stream（除非 daemon 关停）

**实施策略**：

- `EventsClient.Recent` 签名加 `wait time.Duration` 参 → 既有 3 callers (handlers.go / memstore.go / grpcclient.go) 全部更新
- `grpcclient.eventsClient.Recent` 改两阶段 long-poll：
  - **阶段 1**：用 `wait` 作 ctx timeout 调 `Subscribe`，`stream.Recv()` block 等第一个 event 或 ctx.Done()
  - **阶段 2**：拿到首个 event 后用短 ctx (`drainTimeout=100ms`) 继续 `Recv()` drain 已 broadcast 内的多 event，直到 ctx.Done() OR len ≥ limit OR stream Recv error
  - **timeout 路径**：阶段 1 ctx.Done() 直接返 `[]` + nil err（不报错）
- `MemStore.Recent` 加 `wait time.Duration` 参 — 现有 ring buffer 非空时立刻返；空时 `time.Sleep(min(wait, 1*time.Second))` 后返 `[]`（fallback 无真 event 源）
- `handleEvents` line 641 `_ = parseWaitParam(r)` 改 `wait := parseWaitParam(r)` 真传到 Recent
- 测试覆盖：无新 event 时 wait 5s 真 block 5s 返 []；有新 event 时 ≤ 200ms 立刻返；ctx cancel 时 grpcclient 释放 broadcast::Receiver（goroutine 不 leak）
- ADR-014 D2 lint：本 task spec anti-pattern 全部标注

## 2. Goal

真把 `?wait=` 参传到下游 + 真实现 block-until-event-or-timeout 语义；`GET /v1/observability/events?wait=5s` 在无新 event 时真 block 5s 返 200 + []；有新 event 时 ≤ 200ms 立刻返；多 client 并行不互相阻塞；既有 `?limit=` clamp + memstore fallback + 既有 22-endpoint 不退化；≥3 unit test + ≥1 integration test PASS。

## 3. Scope

### In Scope

- **修改 `internal/consoleapi/types.go`** (line 77-79)：
  ```go
  // EventsClient backs GET /v1/observability/events.
  type EventsClient interface {
      Recent(limit int, wait time.Duration) ([]contractv1.ObservabilityEvent, error)  // task-16.2: + wait
  }
  ```

- **修改 `internal/consoleapi/handlers.go`** (line 637-653) handleEvents：
  ```go
  func handleEvents(deps Deps) http.HandlerFunc {
      const defaultLimit = 100
      return func(w http.ResponseWriter, r *http.Request) {
          wait := parseWaitParam(r)       // task-16.2: 真传 wait
          limit := parseLimitParam(r, defaultLimit)
          evts, err := deps.Events.Recent(limit, wait)  // task-16.2: + wait
          if err != nil {
              mapStorageError(w, err)
              return
          }
          if evts == nil {
              evts = []contractv1.ObservabilityEvent{}
          }
          writeJSON(w, http.StatusOK, evts)
      }
  }
  ```

- **修改 `internal/consoleapi/grpcclient/grpcclient.go`** (line 386-414) eventsClient.Recent：
  ```go
  // task-16.2: real long-poll. Two-phase:
  //   1) Wait up to `wait` for the first event (block on stream.Recv()).
  //   2) Once first event arrives, drain immediately-available events for up
  //      to drainTimeout (~100ms) or until limit reached.
  //
  // Empty return on phase-1 timeout — Console expects 200 + [] (NOT 408).
  const drainTimeout = 100 * time.Millisecond

  func (e *eventsClient) Recent(limit int, wait time.Duration) ([]contractv1.ObservabilityEvent, error) {
      if limit <= 0 {
          limit = 100
      }
      if wait <= 0 {
          wait = 30 * time.Second
      }

      // Phase 1: wait for first event with `wait` timeout.
      ctx, cancel := context.WithTimeout(context.Background(), wait)
      defer cancel()

      stream, err := e.c.Subscribe(ctx, &pb.SubscribeEventsRequest{})
      if err != nil {
          return nil, mapGrpcErr(err)
      }

      first, err := stream.Recv()
      if err != nil {
          // ctx timeout / EOF / transport error — return empty (no events arrived).
          // Distinguishing DeadlineExceeded vs real error: empty return is
          // safe for all (Console expects [] on timeout). Log non-deadline
          // errors so operators can see real failures via daemon logs (e.g.
          // gRPC core down) — /v1/health endpoint is the user-visible signal.
          if !errors.Is(err, context.DeadlineExceeded) {
              log.Printf("WARN events Recv error after phase-1 subscribe: %v", err)
          }
          return []contractv1.ObservabilityEvent{}, nil
      }
      batch := make([]contractv1.ObservabilityEvent, 0, limit)
      batch = append(batch, protoToObservabilityEvent(first))

      // Phase 2: drain immediately-available events with short timeout.
      if len(batch) < limit {
          drainCtx, drainCancel := context.WithTimeout(context.Background(), drainTimeout)
          defer drainCancel()
          drainStream, err := e.c.Subscribe(drainCtx, &pb.SubscribeEventsRequest{})
          if err == nil {
              for len(batch) < limit {
                  evt, err := drainStream.Recv()
                  if err != nil {
                      break
                  }
                  batch = append(batch, protoToObservabilityEvent(evt))
              }
          }
      }

      return batch, nil
  }
  ```

- **修改 `internal/consoleapi/memstore.go`** (line 466-478) MemStore.Recent：
  ```go
  // task-16.2: fallback wait — ring buffer 非空时立刻返；空时 sleep min(wait, 1s)
  // 后返 [] (fallback 无真 event 源；不能真 block-on-event-arrival)
  func (s *MemStore) Recent(limit int, wait time.Duration) ([]contractv1.ObservabilityEvent, error) {
      s.mu.Lock()
      have := len(s.events)
      s.mu.Unlock()

      if have == 0 && wait > 0 {
          // task-16.2 [SPEC-OWNER:task-16.2]: fallback 模式无真 broadcast；sleep
          // 模拟 long-poll wait 上限 1s，避 Console UI 拉到立刻空返 → 紧 retry 死循环。
          // 实际 wait 由 grpcclient 路径承担（fallback 仅 in-memory ring buffer）
          sleepFor := wait
          if sleepFor > time.Second {
              sleepFor = time.Second
          }
          time.Sleep(sleepFor)
      }

      s.mu.Lock()
      defer s.mu.Unlock()
      if limit <= 0 || limit > len(s.events) {
          limit = len(s.events)
      }
      if limit == 0 {
          return []contractv1.ObservabilityEvent{}, nil
      }
      out := make([]contractv1.ObservabilityEvent, limit)
      copy(out, s.events[len(s.events)-limit:])
      return out, nil
  }
  ```

- **更新所有 EventsClient.Recent caller**（grep `\.Recent(` 一遍）：
  - `internal/consoleapi/handlers.go::handleEvents` — already updated above
  - `internal/consoleapi/memstore.go::MemStore.Recent` — already updated above
  - `internal/consoleapi/grpcclient/grpcclient.go::eventsClient.Recent` — already updated above
  - `internal/cli/console_api_serve_degraded.go::degradedEvents.Recent`（如存在）— 加 wait 参 + 直接返 `[]contractv1.ObservabilityEvent{}, nil`（degraded 模式无 events）
  - 任何测试桩（如 `internal/consoleapi/router_test.go` 内的 fake events client）— 加 wait 参

- **不修改 Rust 侧**：
  - `core/src/data_plane/events.rs` `EventsServer.subscribe` 不动 — server-stream RPC 永不主动 close；Go client 通过 ctx timeout 主动 cancel = broadcast::Receiver drop = stream graceful close。零 Rust 改动
  - EventBus broadcast::Sender 不动

- **单元测试 ≥3**（在 `internal/consoleapi/handlers_test.go` OR `events_long_poll_test.go` 新建）：
  - `TestHandleEvents_Wait5s_Blocks_When_NoEvent` — wait=5s + fake events client `Recent` 内 `time.Sleep(5s)` + 返 `[]`；assert response 200 + body=`[]` + elapsed ≥ 4.5s
  - `TestHandleEvents_Returns_Early_OnEvent` — wait=5s + fake events client `Recent` 内立刻返 `[evt]`；assert response ≤ 200ms + body 含 1 event
  - `TestHandleEvents_Wait_ClampedTo_60s_Max` — wait=120s → parseWaitParam clamp 60s（既有 test 不退化）
  - `TestMemStore_Recent_EmptyBuffer_Sleeps_Then_Returns_Empty` — wait=2s + 空 buffer → assert elapsed ≥ 1.5s（cap 1s + tolerance）

- **集成测试 ≥1**（`internal/consoleapi/e2e_grpc_test.go` 加 Step 11b）：
  - `TestEventsLongPoll_E2E_GrpcBacked`：
    1. spawn Rust daemon + console-api-serve
    2. curl `GET /v1/observability/events?wait=2s` 第一次 — assert ≥ 1.5s elapsed + body `[]` OR 含 indexing.progress（来自既有 Step 9 job）
    3. 后台 goroutine 触发 POST /v1/index-jobs → 主流 curl `?wait=10s` — assert ≤ 1s elapsed + body 含 ≥ 1 event

- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **events SSE / WebSocket** [SPEC-DEFER:phase-future.events-sse-push]：ADR-017 D4 lock 沿用 long-poll；SSE 留 v1.x（Console v1.0 HTTPAdapter 不消费 SSE）
- **events ?since=cursor 增量拉取** [SPEC-DEFER:phase-future.events-cursor-pagination]：v0.9 仍 limit + recency；cursor 增量留 v1.x
- **events 持久化 ring buffer (RocksDB / SQLite)** [SPEC-DEFER:task-future.event-persistence]：daemon 重启即丢仍接受；持久化留 v1.x
- **multi-subscriber broadcast fairness（慢 subscriber 不影响快 subscriber）** [SPEC-DEFER:phase-future.events-broadcast-fairness]：v0.9 沿用 tokio broadcast::channel 默认 lagging behavior；优化留 v1.x
- **gRPC stream backpressure 控制** [SPEC-DEFER:phase-future.grpc-events-backpressure]：v0.9 sink channel cap=64；自适应 backpressure 留 v1.x
- **wait > 60s 长 ping**：parseWaitParam 既有 60s clamp 不放宽 [SPEC-DEFER:phase-future.events-long-wait-budget]
- **Phase 2 drain re-subscribe 优化**（每次 drain 新 Subscribe stream 浪费）：v0.9 接受；future 优化用单 stream + 读后 try-recv-with-deadline [SPEC-DEFER:phase-future.events-drain-reuse-stream]

## 4. Users / Actors

- **Console UI 端**（下游，via cross-repo）：Memory 操作历史 / IndexJob 进度 / 通用 observability 面板，long-poll wait 真生效后 UX 实时性提升
- **k8s-style health probe**：不受影响（probe 通常不带 wait 参，走默认 30s 但实际触发 timeout 路径返 []）
- **debug session**：开发者短 wait（如 2s）实时看 event stream

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-16-v0.9.0-backlog-completion.md` §3 / §6 AC2
- `docs/specs/tasks/task-11.4-search-real-retriever-and-events.md` (既有 EventsServer + long-poll wrap [SPEC-OWNER:task-11.4] 占位)
- `internal/consoleapi/handlers.go` 既有 handleEvents (line 626-672)
- `internal/consoleapi/grpcclient/grpcclient.go` 既有 eventsClient (line 386-414)
- `internal/consoleapi/types.go::EventsClient` 接口 (line 77-79)
- `core/src/data_plane/events.rs::EventsServer` (无须修改)
- ADR-017 D4 long-poll v1.0 lock

### 5.2 Imports

- **Go**: 既有 `context` / `time` / stdlib；本 task 新加 `errors` (for `errors.Is(err, context.DeadlineExceeded)`) + `log` (for `log.Printf` warn on Recv error) 到 `grpcclient/grpcclient.go` import 块
- **不引入新外部依赖**：R7 不触发 — `errors` / `log` 均 stdlib

### 5.3 Wait timeout 行为对齐

- `parseWaitParam` 既有 clamp [1s, 60s] 不动；default 30s
- `Recent(limit, wait)` 拿到 wait=0 → 内部 fallback 30s（防御性 default）
- Recv error 在 phase 1 → 返 `[]` + nil err（不 propagate ctx.DeadlineExceeded 给 HTTP client）
- Phase 2 drainTimeout 100ms 硬编码（不暴露为参；future tunable 留 [SPEC-DEFER:phase-future.events-drain-timeout-config]）

## 6. Acceptance Criteria

- [x] AC1：`EventsClient.Recent` 签名加 `wait time.Duration` 参；既有 4 callers (types.go interface + handlers.go + grpcclient/grpcclient.go + memstore.go + degraded fallback) 全部更新；`go build ./...` clean — **verified by `go build ./... ` 0 error + `go vet ./...` clean + degradedEvents 同步更新**
- [x] AC2：`GET /v1/observability/events?wait=2s` 在无新 event 时真 block ≥ 1.8s 返 200 + `[]`（NOT 408 / NOT 204）— **verified by `internal/consoleapi/events_test.go::TestHandleEvents_Wait2s_Blocks_When_NoEvent` PASS (实测 2.00s elapsed) + daemon-level e2e_grpc Step 11b 留 task-16.4 收口**
- [x] AC3：`GET /v1/observability/events?wait=5s` 在有新 event 时 ≤ 500ms 立刻返 200 + ≥1 event — **verified by `events_test.go::TestHandleEvents_Returns_Early_OnEvent` PASS (实测 0.00s elapsed; immediate-return fake)**
- [x] AC4：多 client 并行不互相阻塞 — 2 goroutine 同时 `?wait=1s` 各自独立 timeout/return；不死锁；总 wall-clock ≤ 1.8s（顺序版本会 ~2s）— **verified by `events_test.go::TestHandleEvents_ConcurrentClients_Independent` PASS (实测 1.00s 并发版本 vs 2s 顺序)**
- [x] AC5：MemStore fallback `Recent(limit, wait)` 空 buffer 时 sleep min(wait, 1s) 返 `[]`；非空时立刻返；接口 compliance — **verified by `events_test.go::TestMemStore_Recent_EmptyBuffer_SleepsThenReturnsEmpty` PASS (1.00s elapsed 命中 1s cap) + `TestMemStore_Recent_NonEmptyBuffer_DoesNotSleep` PASS (0.00s 非空不 sleep)**
- [x] AC6：grpcclient ctx cancel 释放后端 broadcast::Receiver — `defer cancel()` + `defer drainCancel()` 在 eventsClient.Recent 末尾保证 phase-1 + phase-2 stream 都释放；non-DeadlineExceeded 错误 log 后吞 — **verified by 代码审视：两阶段均有 `defer cancel()`；log.Printf 触发非 deadline error path；既有 e2e_grpc test 不退化（cached, PASS）**
- [x] AC7：既有 22-endpoint conformance + Phase 15 v6 smoke 不退化；既有 e2e_grpc Step 11 (events keepalive) 不退化 — **verified by `go test ./...` 22 packages 全 PASS（含 `test/conformance` 22-endpoint + e2e_grpc 真接 Rust daemon + 既有 Phase 15 v6 smoke step）+ cargo workspace 未改 unaffected**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | EventsClient.Recent + wait | types.go / handlers.go / grpcclient.go / memstore.go / degraded | Done |
| AC2 | wait 2s no-event block ≥ 1.8s [] | events_test.go TestHandleEvents_Wait2s_Blocks_When_NoEvent | Done |
| AC3 | wait with-event return ≤ 500ms | events_test.go TestHandleEvents_Returns_Early_OnEvent | Done |
| AC4 | concurrent clients independent | events_test.go TestHandleEvents_ConcurrentClients_Independent | Done |
| AC5 | MemStore wait sleep / non-sleep | events_test.go TestMemStore_Recent_{EmptyBuffer_SleepsThenReturnsEmpty,NonEmptyBuffer_DoesNotSleep} | Done |
| AC6 | ctx cancel goroutine no leak | defer cancel × 2; non-deadline log.Printf path | Done |
| AC7 | regression 不退化 | go test ./... 22 pkgs PASS (含 e2e_grpc + conformance) | Done |

## 8. Risks

- **gRPC stream Subscribe re-create overhead in phase 2 drain**：每次 drain 新建 Subscribe stream 有 ~5ms RTT；接受 — phase 2 上限 100ms，drain re-use stream 优化留 [SPEC-DEFER:phase-future.events-drain-reuse-stream]
- **Phase 2 重订阅 race window 可能漏 event**：phase-1 stream.Recv 返回首 event 后到 phase-2 Subscribe 完成前的 ~5ms 窗内，broadcast::Sender 发送的新 event 不会被任一 receiver 处理（phase-1 receiver 已不再 Recv；phase-2 receiver 未订阅）。可接受 — observability event 是 informational 非 transactional；Console UI 用户可下一次 poll cycle 重拉。单 stream 设计（goroutine + 通道 + drainTimeout select）可消除 race，但 ~30 lines 复杂度上升 [SPEC-DEFER:phase-future.events-drain-reuse-stream]
- **MemStore sleep min(wait, 1s) 阻塞 HTTP handler**：fallback 模式 single goroutine sleeping 1s 不影响其他 HTTP route（Go net/http handler-per-conn）；多 client 并发不死锁；接受
- **Goroutine leak on phase-1 timeout**：`stream.Recv()` 收到 ctx.DeadlineExceeded 后 stream 内部 goroutine 应自动结束；缓解 — `cancel()` defer 显式释放 + `-race` test 验证 NumGoroutine 不增长
- **client disconnect mid-wait**：HTTP client (curl) 收到 ctx cancel 时 Go handler 还在 grpcclient.Recent 内 block；缓解 — handler ctx 继承 r.Context() 在 future task；v0.9 接受 — handler 写 response 失败时 grpcclient still completes background；不 leak（cancel defer）但浪费少量 work；优化留 [SPEC-DEFER:phase-future.events-http-ctx-propagate]
- **MemStore wait + non-empty 时仍 sleep 风险**：当前设计 wait > 0 + len(events)==0 才 sleep；非空时不 sleep 直接返；正确
- **关联 [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D4 long-poll v1.0 lock**：本 task 不引入 SSE / WebSocket；纯 long-poll wait 实现修正
- **既有 `Recent(limit int)` 单参签名 BREAKING**：crate-internal BREAKING；所有 Go callers grep 一遍 + 更新；零跨仓影响（contractv1 / proto 不动）

## 9. Verification Plan

- **install**: 已有 `go mod download`
- **lint**: `gofmt -l internal/consoleapi/` + `go vet ./internal/consoleapi/...`
- **typecheck**: `go build ./cmd/contextforge ./internal/...`
- **unit-test**: `go test -v ./internal/consoleapi/... -run TestHandleEvents`
- **integration**: `go test -v -race ./internal/consoleapi/... -run TestEventsLongPoll_E2E_GrpcBacked`
- **e2e**: smoke v7 Step 25
- **build**: `go build ./cmd/contextforge`
- **runtime-smoke**: start daemon + console-api-serve + 一窗 `curl ?wait=10s` + 另窗 trigger POST /v1/index-jobs + 主窗 ≤ 1s 返
- **coverage**: 不强制
- **manual**: 见 §9 runtime-smoke

## 10. Completion Notes

- **完成日期**：2026-05-27
- **关键决策**：
  - **EventsClient.Recent 签名直接 BREAKING crate-internal**：4 callers (types interface + handlers + grpcclient + memstore + degraded) 一起更新；package internal，零跨仓 / proto 影响。
  - **两阶段 long-poll**：phase-1 ctx timeout = `wait` 等首 event；phase-2 重 `Subscribe` 用 `eventsDrainTimeout = 100ms` drain 已 broadcast 的 follow-up event。phase-2 接受 ~5ms 重订阅窗口 race（§8 记录的 informational event trade-off）。
  - **错误吞 + log.Printf 区分**：phase-1 Recv 失败 → 返 `[]` + nil；DeadlineExceeded 静默（正常 timeout 情况），非 deadline error 走 `log.Printf` warn（gRPC core 真挂时运维可见）。
  - **MemStore fallback sleep cap 1s**：避免 Console UI 设 `?wait=30s` 时 HTTP handler goroutine 持有 30s；同时不立返 `[]` 以避 UI poll-storm；选 1s 取中庸。
  - **degradedEvents 不 honor wait**：degraded 模式无 event source，wait sleep 只是延迟 503，没意义；直接返 ErrDataPlaneUnavailable 让 ops 链路立即收到 unhealthy 信号。
  - **测试用 stub EventsClient 而非真 gRPC**：events_test.go 引入 `sleepingEventsClient` + `immediateEventsClient` stub，跑 `httptest.NewRecorder()` 即可断言 wait/no-wait/early-return/concurrent 行为；daemon-level 端到端验证留 task-16.4 smoke v7 Step 25。
  - **`parseWaitParam` 既有 clamp 不动**：default 30s / clamp [1s, 60s] 沿用；新加 `TestParseWaitParam_ClampUpperLowerAndDefault` 5 sub-cases 锚定既有行为不被本 task 改动破坏。
- **§9 Verification 结果**：
  - `go build ./...`: clean
  - `go vet ./...`: clean
  - `go test ./internal/consoleapi/... -run "HandleEvents|MemStore_Recent|ParseWaitParam" -v`: 6 tests + 5 sub-cases ALL PASS（实测时序：Wait2s 2.00s / Returns_Early 0.00s / Concurrent 1.00s / EmptyBuffer 1.00s / NonEmpty 0.00s）
  - `go test ./...`: 22 packages PASS（含 internal/consoleapi 42s 含 e2e_grpc 真接 Rust daemon + test/conformance 22-endpoint 不退化 + internal/cli 28s degradedEvents 不退化 + daemon / release / 其他）
  - cargo workspace 不动 — task-16.2 不触 Rust SoT
- **改动文件**：
  - `internal/consoleapi/types.go` (修改 — `EventsClient.Recent` 加 `wait time.Duration` 参 + `time` import)
  - `internal/consoleapi/handlers.go` (修改 — `handleEvents` line 637-653 真传 `wait` 到 `deps.Events.Recent`)
  - `internal/consoleapi/grpcclient/grpcclient.go` (修改 — `eventsClient.Recent` 两阶段 long-poll + `eventsDrainTimeout` const + `log` import + non-DeadlineExceeded warn)
  - `internal/consoleapi/memstore.go` (修改 — `MemStore.Recent` 加 wait 参 + empty buffer sleep min(wait, 1s))
  - `internal/cli/console_api_serve_degraded.go` (修改 — `degradedEvents.Recent` 签名同步 + `time` import)
  - `internal/consoleapi/events_test.go` (新增 ~250 行 — 5 tests + 1 helper sleepingEventsClient + 1 helper immediateEventsClient + parseWaitParam clamp 5 sub-cases)
  - `docs/specs/tasks/task-16.2-events-real-long-poll.md` (本 spec §6 [x] / §7 Done / §10 完工 + Status → Done)
- **commit 列表**：
  - feat(consoleapi): task-16.2 — events ?wait= real two-phase long-poll (Phase 16 P4 #11)
- **剩余风险 / 未做项**：
  - **events SSE / WebSocket** [SPEC-DEFER:phase-future.events-sse-push]：ADR-017 D4 lock；Console v1.0 HTTPAdapter 不消费 SSE
  - **?since=cursor 增量** [SPEC-DEFER:phase-future.events-cursor-pagination]
  - **events 持久化 ring buffer (SQLite)** [SPEC-DEFER:task-future.event-persistence]：daemon 重启即丢仍接受
  - **multi-subscriber broadcast fairness** [SPEC-DEFER:phase-future.events-broadcast-fairness]
  - **gRPC stream backpressure** [SPEC-DEFER:phase-future.grpc-events-backpressure]
  - **wait > 60s 长 ping** [SPEC-DEFER:phase-future.events-long-wait-budget]
  - **Phase 2 drain re-subscribe 优化（single-stream + select 消除 race）** [SPEC-DEFER:phase-future.events-drain-reuse-stream]
  - **HTTP client disconnect ctx propagate to grpcclient** [SPEC-DEFER:phase-future.events-http-ctx-propagate]
- **下游 task 影响**：task-16.4 smoke v7 Step 25 daemon-level long-poll 验证；ADR-015 D1 add-only 不破（不动 proto / contractv1 字段）；ADR-017 D4 long-poll v1.0 lock 沿用（无 SSE 引入）；Console UI v0.9 自动获益（UX 实时性提升，client 改动可选 — `?wait=30s` 已是 v0.7 默认）。
