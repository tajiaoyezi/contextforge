# Task `11.4`: `search-real-retriever-and-events — SearchService.Query 真接 retriever + EventsService.Subscribe 真接 EventBus + Go long-poll wrap`

**Status**: Ready

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 11 (console-real-data-plane)
**Dependencies**: task-11.1 (`SearchServer` + `EventsServer` 占位) + task-11.2 (Go grpcclient + handler thin proxy) + task-11.3 (`JobRunner` 真 emit `indexing.progress` event 到 EventBus 路径已 wire) + task-4.1 / 4.2 (retriever 真 impl Tantivy + RetrievalTrace) + [ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) D2/D3

## 1. Background

task-11.1 落地 `SearchServer` 占位返 empty `SearchResult` + `RetrievalTrace`；`EventsServer` 占位返 keepalive only。本 task 是 Phase 11 收口 task —— 把 SearchService 真接现有 retriever (`core/src/retriever/`，task-4.1/4.2 落地 Tantivy + RetrievalTrace)、`EventsService` 真接 tokio broadcast channel-backed `EventBus`、`JobRunner` progress emit 路径（task-11.3 已 wire `Option<EventBus>` 容错）激活、Go REST `/v1/observability/events` handler 改 long-poll wrap (30s timeout / 100 evt batch)。

`SearchService.Query` 接 retriever 后：`RetrievalTrace.retrieved_chunks` 真填（score from Tantivy + source_file from chunk.path + content snippet from `chunk.content[..min(200, len)]`，与 Console contractv1.RetrievalTrace.RetrievedChunks 字段对齐）。

`EventsService.Subscribe` 接 `tokio::sync::broadcast::channel(1000)`（容量 1000，溢出 drop oldest，与 v0.3 内部 evt 约定一致；overflow 替代 Kafka/NATS 与 ADR-004 local-first 一致）。`JobRunner` 在 task-11.3 progress 闭包内已调 `eb.send(...)`，本 task 落 `eb` 实例化 + 注入 `DataPlaneStores.event_bus = Some(Arc<EventBus>)`。

Go `/v1/observability/events` long-poll wrap：handler 收 GET → 调 `EventsClient.Recent(limit=100)` → grpcclient 内部调 `EventsService.Subscribe` server stream → select loop：`{recv => append batch + return if len >= 100; timeout 30s => return current batch}`；空批次也返 200 + `[]`（不返 204；Console adapter v1.0 期望 200 + maybe-empty）。

## 2. Goal

`core/src/data_plane/search.rs::SearchServer::Query` 真接 `core/src/retriever/`（Tantivy + SqliteChunkStore）+ `RetrievalTrace.retrieved_chunks` 真填；`core/src/data_plane/events.rs::EventsServer::Subscribe` 真接 `tokio::sync::broadcast::channel(1000)` EventBus；`DataPlaneStores` 注入 `event_bus: Arc<EventBus>` 让 task-11.3 progress emit 路径激活；Go `internal/consoleapi/handlers.go::handleEvents` 改 long-poll wrap (30s timeout / 100 evt batch)；fixture repo (task-11.3 既建 `test/fixtures/index-job-real/`) → 索引完成后 POST `/v1/search` 真返回 ≥1 SourceChunk + score>0 + source_file 匹配；GET `/v1/observability/events` 在 index 跑期间真返回 ≥1 `indexing.progress` evt 含 `job_id` + `processed_files` + `total_files`；`cargo test --workspace` + `go test ./...` 全绿。

## 3. Scope

### In Scope

- **修改 `core/src/data_plane/search.rs::SearchServer::Query`**：
  - 替换 task-11.1 占位空响应为真调 `core/src/retriever/`（task-4.1/4.2 既有 `Retriever::search(query, top_k, filters)` API）
  - 构造 `SearchResult { items: Vec<SourceChunk> }`：每个 hit 转 `SourceChunk { id, score, source_file, content, line_start, line_end, ... }`（字段 1:1 镜像 Go contractv1.SourceChunk）
  - 构造 `RetrievalTrace { query_id, ts_unix, retrieved_chunks: Vec<RetrievedChunkEntry>, /* ... */ }`；`RetrievedChunkEntry { chunk_id, score, source_file, content_snippet }`；`content_snippet = chunk.content[..min(200, len)]`（UTF-8 boundary-safe 截断 —— 不在 multi-byte 字符中间切断）
- **修改 `core/src/data_plane/events.rs::EventsServer::Subscribe`**：
  - 接受 `subscriber = event_bus.subscribe()` (broadcast::Receiver)
  - server stream loop：`recv.recv().await` → `tonic::Status::ok` + 转 proto `ObservabilityEvent` + send to client；client cancel → drop subscriber
  - 错误 mapping：broadcast `RecvError::Lagged` → log warning + skip (subscriber 漏 evt 但不 break stream)；`RecvError::Closed` → end stream gracefully
- **新增 `core/src/data_plane/events.rs::EventBus`**：
  - `pub struct EventBus { tx: tokio::sync::broadcast::Sender<ObservabilityEvent> }`
  - `pub fn new() -> Arc<Self>` (cap=1000)
  - `pub fn send(&self, evt: ObservabilityEvent) -> Result<usize, broadcast::error::SendError>` (允许 0 subscriber 时 silent drop)
  - `pub fn subscribe(&self) -> broadcast::Receiver<ObservabilityEvent>`
- **修改 `core/src/data_plane/mod.rs::DataPlaneStores`**：新增 `pub event_bus: Arc<EventBus>` 字段；daemon serve 启动时 `EventBus::new()` 实例化并注入
- **修改 `core/src/bin/contextforge_core.rs`** (或 daemon serve 入口)：构造 `EventBus + Arc::new(...)` + 注入到 DataPlaneStores
- **修改 `internal/consoleapi/handlers.go::handleEvents`**：
  - 替换 v0.3 简单返 `Deps.Events.Recent(100)` 为 long-poll wrap：
    1. parse `?wait=30s` query param (默认 30s, max 60s)
    2. 调 `Deps.Events.RecentLongPoll(ctx, limit=100, timeout=wait)`（新接口方法；EventsClient 实现需对应扩展）
    3. 返 200 + JSON []ObservabilityEvent
  - 如 ctx canceled mid-poll → 返 已收 batch (可能 empty)；不返 4xx
- **修改 `internal/consoleapi/types.go::EventsClient` 接口**：
  - 新增 `RecentLongPoll(ctx context.Context, limit int, timeout time.Duration) ([]contractv1.ObservabilityEvent, error)`
  - 保留 v0.3 `Recent(limit int)` 方法不变（用于 fallback-inmem 模式无 ctx + 无 timeout）
- **修改 `internal/consoleapi/grpcclient/grpcclient.go::eventsClient::RecentLongPoll`**：
  - 调 `EventsService.Subscribe` server stream + select loop：
    - 收到 evt → append batch + `if len(batch) >= limit { close stream; return batch, nil }`
    - timer fires (timeout) → close stream + return batch (maybe empty)
    - ctx canceled → close stream + return batch + ctx.Err()
- **集成测试**：
  - `core/tests/search_real_retriever.rs::test_search_real_chunks`：fixture repo (task-11.3 `index-job-real/`) → index 真跑完（复用 task-11.3 wiring）→ POST SearchRequest `{query: "contextforge", top_k: 5}` → 返 ≥1 SourceChunk + score > 0 + source_file ∈ fixture file 列表
  - `core/tests/search_real_retriever.rs::test_retrieval_trace_fields`：同上 → 断言 `RetrievalTrace.retrieved_chunks[0]` 含 chunk_id + score + source_file + content_snippet (len ≤ 200 + UTF-8 boundary safe)
  - `core/tests/events_real_eventbus.rs::test_progress_event_emitted`：daemon 启动 + Enqueue fixture index → Subscribe stream → 期望收到 ≥1 `indexing.progress` evt 含 `job_id`/`processed_files`/`total_files`
  - `internal/consoleapi/handlers_test.go::TestHandleEvents_LongPoll30s`：mock EventsClient 5s 后 emit 1 evt → handler 返 200 + 1 evt 含期望字段
  - `internal/consoleapi/handlers_test.go::TestHandleEvents_TimeoutEmptyBatch`：mock EventsClient never emit → handler 等 30s 后返 200 + []
  - `internal/consoleapi/handlers_test.go::TestHandleEvents_Batch100Caps`：mock emit 200 evt → handler 收 100 evt 后立即返
- **不破坏 v0.3 conformance**：`go test ./test/conformance/... -run TestConsoleContractV1Conformance` 仍 PASS（v0.3 fakehttpserver oracle 对 `/v1/observability/events` 期望 200 + []ObservabilityEvent）
- **不引入新 R7 dep**：现有 `tokio::sync::broadcast` (tokio std feature) + `tonic` server stream；Go 端 stdlib `context` + `time`
- **文件锚点**：`core/src/data_plane/search.rs` + `core/src/data_plane/events.rs` + `core/src/data_plane/mod.rs` + `core/src/bin/contextforge_core.rs` + `internal/consoleapi/handlers.go` + `internal/consoleapi/types.go` + `internal/consoleapi/grpcclient/grpcclient.go` + `core/tests/{search_real_retriever,events_real_eventbus}.rs` + `internal/consoleapi/handlers_test.go`
- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **search filters (workspace_id / file_type / date range)** [SPEC-DEFER:console-endpoint-expansion]：v0.4 仅 query string + top_k；filter struct 字段 reserved
- **event types extension (workspace.created / workspace.deleted)** [SPEC-DEFER:console-endpoint-expansion]：v0.4 仅 `indexing.progress` / `indexing.cancelled` / `indexing.error`；其它 event type v0.4.x 增量
- **真 SSE (Server-Sent Events) / WebSocket** [SPEC-DEFER:task-future.consoleapi-sse]：v0.4 仍 long-poll（与 v0.3 Console fakehttpserver oracle 约定一致；ADR-015 D5 沿用）
- **event ring buffer 持久化** [SPEC-DEFER:task-future.event-persistence]：v0.4 broadcast channel volatile（daemon 重启即丢；与 ADR-004 local-first 一致；Console UI 持久化 evt 在 console-api 端）
- **search 反向 retriever cross-validation (eval 集 hit rate)** [SPEC-DEFER:task-future.search-eval-integration]：v0.4 仅功能 wiring；recall eval 仍走 v0.1 `contextforge eval run` (Phase 8 task-8.1)
- **多 EventBus subscriber filter (since=event_id / since_unix)** [SPEC-DEFER:console-endpoint-expansion]：v0.4 仅 from-now subscribe；replay 留 v0.4.1
- **task-11.3 JobRunner 真接 IndexSession** [SPEC-OWNER:task-11.3]：本 task 依赖 task-11.3 progress emit 路径已 wire；不重做 wiring

## 4. Users / Actors

- **Console UI 用户**（end-user）：POST `/v1/search` 真返回 indexed 内容；GET `/v1/observability/events` 真流 progress
- **task-11.3 progress emit 路径消费方**：本 task EventBus 实例化后 task-11.3 `Option<EventBus>=Some(...)` 自动激活
- **运维**：通过 GET `/v1/observability/events` 实时观察索引进度（无需 polling job status）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md` §D2 / §D3
- `docs/specs/phases/phase-11-console-real-data-plane.md`
- `docs/specs/tasks/task-11.1-rust-data-plane-grpc-services.md` (SearchServer / EventsServer 占位现状)
- `docs/specs/tasks/task-11.2-go-rest-to-grpc-proxy.md` (EventsClient 接口现状)
- `docs/specs/tasks/task-11.3-indexjob-real-runner-wiring.md` (JobRunner progress emit 路径 wire 现状)
- `docs/specs/tasks/task-4.1-retriever.md` (Retriever::search API)
- `docs/specs/tasks/task-4.2-explain.md` (RetrievalTrace 字段)
- `core/src/retriever/` (Retriever 当前签名)
- `internal/contractv1/contractv1.go` (SourceChunk / RetrievalTrace / ObservabilityEvent JSON tag)
- `tokio::sync::broadcast` 文档 (broadcast channel semantics + Lagged 处理)

### 5.2 Imports

- **Rust**: 现有 `tokio` (sync::broadcast) + `tonic` stream；现有 `core/src/retriever/` API；不引入新 dep
- **Go**: stdlib `context` + `time`；现有 `google.golang.org/grpc` stream
- **不引入新依赖**：R7 不触发；`go.mod` / `Cargo.toml` 不动

### 5.3 函数签名

```rust
// core/src/data_plane/events.rs

use tokio::sync::broadcast;
use std::sync::Arc;

pub struct EventBus {
    tx: broadcast::Sender<ObservabilityEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Arc<Self> {
        let (tx, _) = broadcast::channel(capacity);
        Arc::new(Self { tx })
    }
    pub fn send(&self, evt: ObservabilityEvent) -> Result<usize, broadcast::error::SendError<ObservabilityEvent>> {
        self.tx.send(evt)
    }
    pub fn subscribe(&self) -> broadcast::Receiver<ObservabilityEvent> {
        self.tx.subscribe()
    }
}

pub struct EventsServer { event_bus: Arc<EventBus> }

#[tonic::async_trait]
impl proto::events_service_server::EventsService for EventsServer {
    type SubscribeStream = /* impl Stream<Item = Result<ObservabilityEvent, Status>> */;
    async fn subscribe(
        &self,
        _req: tonic::Request<proto::SubscribeEventsRequest>,
    ) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
        let mut rx = self.event_bus.subscribe();
        let stream = async_stream::stream! {
            loop {
                match rx.recv().await {
                    Ok(evt) => yield Ok(observability_event_to_proto(evt)),
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(missed=n, "events subscriber lagged");
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        };
        Ok(tonic::Response::new(Box::pin(stream)))
    }
}
```

```go
// internal/consoleapi/grpcclient/grpcclient.go
func (e *eventsClient) RecentLongPoll(ctx context.Context, limit int, timeout time.Duration) ([]contractv1.ObservabilityEvent, error) {
    pollCtx, cancel := context.WithTimeout(ctx, timeout)
    defer cancel()
    stream, err := e.c.Subscribe(pollCtx, &pb.SubscribeEventsRequest{})
    if err != nil { return nil, err }

    batch := make([]contractv1.ObservabilityEvent, 0, limit)
    for {
        select {
        case <-pollCtx.Done():
            return batch, nil  // timeout or parent ctx canceled
        default:
        }
        evt, err := stream.Recv()
        if err == io.EOF { return batch, nil }
        if err != nil {
            if errors.Is(pollCtx.Err(), context.DeadlineExceeded) { return batch, nil }
            return batch, err
        }
        batch = append(batch, protoToObservabilityEvent(evt))
        if len(batch) >= limit { return batch, nil }
    }
}
```

## 6. Acceptance Criteria

- [ ] AC1：POST `/v1/search` `{query: "contextforge", top_k: 5}` （fixture repo 已 indexed） → 真返回 ≥1 `SourceChunk` + `score > 0` + `source_file` ∈ fixture file 列表（≥1 path 匹配 `test/fixtures/index-job-real/file*.md`）— **verified by integration-test step `cargo test -p contextforge-core --test search_real_retriever -- test_search_real_chunks`**
- [ ] AC2：`RetrievalTrace.retrieved_chunks[0]` 含 `chunk_id` + `score` + `source_file` + `content_snippet`；`content_snippet` 长度 ≤ 200 字符 + UTF-8 boundary safe (不在 multi-byte 中切) — **verified by unit-test step `cargo test -p contextforge-core --lib data_plane::search -- test_retrieval_trace_fields` + `test_content_snippet_utf8_boundary`**
- [ ] AC3：GET `/v1/observability/events` 在 30s timeout 内若有 `indexing.*` 事件 → 立即返回 200 + batch；空时 30s 后返 200 + `[]`；满 100 evt 时立即返 200 + 100 — **verified by integration-test step `go test ./internal/consoleapi/... -run 'TestHandleEvents_LongPoll30s|TestHandleEvents_TimeoutEmptyBatch|TestHandleEvents_Batch100Caps'`**
- [ ] AC4：`JobRunner` 进度 callback 触发 EventsService stream emit `indexing.progress` 事件含 `{job_id, processed_files, total_files, ts_unix}`；fixture index 跑期间 subscribe → 收到 ≥1 progress evt — **verified by integration-test step `cargo test -p contextforge-core --test events_real_eventbus -- test_progress_event_emitted`**
- [ ] AC5：`cargo test --workspace` + `go test ./...` 全绿；`go test ./test/conformance/... -run TestConsoleContractV1Conformance` 不退化 — **verified by typecheck + unit-test phase smoke + conformance**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | SearchService.Query 真接 retriever | core/src/data_plane/search.rs + test_search_real_chunks | Ready |
| AC2 | RetrievalTrace.retrieved_chunks 真填 (score + source_file + snippet) | search.rs + test_retrieval_trace_fields + test_content_snippet_utf8_boundary | Ready |
| AC3 | Go long-poll wrap 30s / 100 evt batch | handlers.go::handleEvents + grpcclient + 3 handler test | Ready |
| AC4 | EventsService 真接 EventBus + progress emit | events.rs + test_progress_event_emitted | Ready |
| AC5 | 不退化 + conformance 不退化 | cargo test --workspace + go test ./... | Ready |

## 8. Risks

- **`Retriever::search` 返字段不齐**：v0.1/v0.2 Retriever 返 `Vec<RetrievalHit>`，但 contractv1.SourceChunk 含 `line_start` / `line_end` 等位置字段 —— 需 verify `core/src/retriever/` 真返字段集合是否覆盖；若不齐 → task-11.4 §10 加 trade-off T1 "扩展 Retriever 返字段"（add-only）
- **`content_snippet` UTF-8 boundary**：`&str[..200]` 在 multi-byte 中切会 panic；缓解用 `chunk.content.chars().take(200).collect::<String>()` 或 `s.char_indices().nth(200).map(|(i,_)| &s[..i]).unwrap_or(&s)`
- **`tokio::sync::broadcast` capacity 1000 溢出**：单 daemon + 多 client 场景下 long-running index emit 大量 progress evt → 容量 1000 可能不够；缓解 evt 自身已经按 100 files / 5s rate-limited (task-11.3)；溢出 broadcast::Lagged 已 log warning + continue
- **EventsService.Subscribe gRPC stream 半开连接**：client 网络断开但 stream 不知；缓解 stream loop 内 tonic Ping/Pong 默认 keepalive；ContextForge daemon 本地 loopback 不易触发
- **Long-poll handler ctx propagation**：HTTP handler 收到 client cancel → ctx 走 chi → ctx.Done → handler 提前返；缓解 `select { ctx.Done() => return batch, default: stream.Recv }`
- **v0.3 conformance test 不退化**：`/v1/observability/events` v0.3 期望 200 + []ObservabilityEvent；本 task long-poll wrap 仍 200 + []，schema 不变 → conformance 不退化

## 9. Verification Plan

- **install**: `cargo fetch && go mod download`
- **lint**: `cargo fmt --check && gofmt -l internal/consoleapi/`
- **typecheck**: `cargo check -p contextforge-core && go vet ./...`
- **unit-test**: `cargo test -p contextforge-core --lib data_plane::search` + `cargo test -p contextforge-core --lib data_plane::events` + `go test ./internal/consoleapi/... -run 'TestHandle'`
- **integration**: `cargo test -p contextforge-core --test search_real_retriever` + `cargo test -p contextforge-core --test events_real_eventbus` + `go test ./internal/consoleapi/... -run 'TestHandleEvents_'`
- **e2e**: 通过 integration 实现（fixture index → search → events）
- **build**: `cargo build -p contextforge-core && go build ./...`
- **coverage**: 不强制
- **runtime-smoke**: 启 daemon + 启 console-api-serve + curl POST `/v1/index-jobs` → curl GET `/v1/observability/events?wait=30s` 看真返 indexing.progress + curl POST `/v1/search` 看真返 chunks
- **manual**: 索引 fixture 后 `sqlite3 <data_dir>/chunks.db 'SELECT count(*) FROM chunks WHERE source_file LIKE "%index-job-real%"'` > 0

## 10. Completion Notes

<!-- 完工时按 standard.md §8.3 6 项 schema 回填 -->

- **完成日期**：<待回填>
- **改动文件**：<待回填>
- **commit 列表**：<待回填>
- **§9 Verification 结果**：<待回填>
- **剩余风险 / 未做项**：
  - search filters / event types extension [SPEC-DEFER:console-endpoint-expansion]
  - 真 SSE / WebSocket [SPEC-DEFER:task-future.consoleapi-sse]
  - event ring buffer 持久化 [SPEC-DEFER:task-future.event-persistence]
  - search 反向 retriever eval cross-validation [SPEC-DEFER:task-future.search-eval-integration]
  - 多 subscriber filter (since=event_id) [SPEC-DEFER:console-endpoint-expansion]
- **下游 task 影响**：Phase 11 closeout PR；ADR-016 Proposed → Accepted；v0.4.0 release 准备就绪
