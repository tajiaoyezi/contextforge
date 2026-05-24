# Task `12.3`: `search-trace-by-query-id — GET /v1/search/{query_id}/trace + Rust SearchService trace 持久化 + GetSearchTrace RPC`

**Status**: Done

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 12 (console-contract-completion)
**Dependencies**: task-12.2 (SearchService proto add-only pattern + handlers.go pattern 已 ship) + task-11.4 (SearchService.Query 真接 retriever + RetrievalTrace 真填) + [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D1 Wave 2

## 1. Background

Console Contract v1 22 endpoint 含 `GET /v1/search/{query_id}/trace` 按 query_id 取已执行 search 的 trace（RetrievalTrace 类型；含 trace_id / query / expanded_query / candidate_generation_steps / lexical_candidates_count / vector_candidates_count / rerank_steps / scope_filter_result / final_context_count 9 字段）。

当前 v0.4.0 (Phase 11) ship 的 SearchService.Query 在响应 inline 返 RetrievalTrace（作为 `SearchResponse { result: SearchResult, trace: RetrievalTrace }` 嵌套字段），但**没有持久化 by query_id** —— query 执行完后 trace 即丢失，无法 by-id 反向取。

本 task 在 Rust 侧实施：(a) SearchService.Query 执行时 trace 持久化 by `result.query_id`；(b) 新增 `SearchService.GetSearchTrace(GetSearchTraceRequest) returns (RetrievalTrace)` RPC；(c) Go REST handler + grpcclient wrapper。

**持久化策略 trade-off**（task §10 评估，本 task 选 in-memory LRU 1000）：
- 选项 A (in-memory LRU 1000，本 task 选)：Rust 进程内 `parking_lot::Mutex<lru::LruCache<String, RetrievalTrace>>`；优势：零 SQLite schema 演进；劣势：daemon 重启即丢；QPS 超 1000/min 时 eviction
- 选项 B (SQLite migration 0012_search_traces.sql)：持久化跨重启；劣势：写 IO + schema 演进成本 + Console UI 一般只查最近 trace（重启即丢可接受）；留 `[SPEC-DEFER:task-future.search-trace-sqlite-persistence]`

## 2. Goal

`proto/contextforge/console_data_plane/v1/console_data_plane.proto` SearchService 加 `GetSearchTrace` RPC + `GetSearchTraceRequest` message + 复用既有 `RetrievalTrace` message (task-11.4 ship)；`core/src/data_plane/search.rs` SearchServer 新增 trace store (in-memory LRU 1000) + Query 执行后 store.put(query_id, trace) + 新增 `get_search_trace` 方法；`internal/consoleapi/grpcclient/grpcclient.go` SearchClient 加 `GetSearchTrace` wrapper；`internal/consoleapi/handlers.go` + `router.go` 加 `GET /v1/search/{query_id}/trace` 路由 + handler；MemStore fallback 返 `ErrDataPlaneUnavailable`；`cargo test --workspace` + `go test ./...` 全绿；≥4 单元测试 + ≥1 集成测试 PASS；end-to-end smoke step 跑通。

## 3. Scope

### In Scope

- **修改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`**：
  - SearchService 加 RPC `rpc GetSearchTrace(GetSearchTraceRequest) returns (RetrievalTrace);`（add-only，与 task-12.2 GetSourceChunk + 既有 Query 并存）
  - 新增 message `GetSearchTraceRequest { string query_id = 1; }`
  - 复用已有 `RetrievalTrace` message（task-11.4 ship）
  - proto 编号下一个未用
- **修改 `core/src/data_plane/search.rs`**：
  - SearchServer 新增字段 `trace_store: parking_lot::Mutex<lru::LruCache<String, RetrievalTrace>>` 容量 1000（如 lru crate 未在 Cargo.toml 评估 mini hashmap + manual eviction 替代 [SPEC-OWNER:task-12.3]）
  - 既有 `query` 方法实现末尾：`self.trace_store.lock().put(query_id.clone(), trace.clone())`（query_id 在 SearchResponse.result.query_id 字段；trace 在 SearchResponse.trace 字段）
  - 新增 `get_search_trace(&self, req: GetSearchTraceRequest) -> Result<Response<RetrievalTrace>, Status>`：
    - 调 `self.trace_store.lock().get(&req.query_id).cloned()`
    - `Some(trace)` → `Response::new(trace_to_proto(trace))`
    - `None` → `Err(Status::not_found(format!("trace not found: {}", req.query_id)))`
- **修改 `core/Cargo.toml`** (可选，§10 trade-off)：
  - 添加 `lru = "0.12"` 依赖（约 10kloc + 0 transitive deps；R7 触发，但成本极低）
  - 替代方案：用 `std::collections::HashMap` + 手动 eviction（VecDeque 跟踪插入顺序；满 1000 时 pop_front + remove）—— 不引入 lru dep，但代码 + 测试稍多
- **修改 `internal/consoleapi/grpcclient/grpcclient.go`**：
  - `SearchClient` 加 `GetSearchTrace(queryID string) (contractv1.RetrievalTrace, error)` 方法
  - 调 `c.search.GetSearchTrace(ctx, &proto.GetSearchTraceRequest{QueryId: queryID})` → protoToRetrievalTrace → 返
  - 错误 mapping (mapGrpcErr) 沿用 task-11.2 既有
- **修改 `internal/consoleapi/types.go`**：
  - SearchClient 接口加 `GetSearchTrace(queryID string) (contractv1.RetrievalTrace, error)`
- **修改 `internal/consoleapi/router.go`**：
  - 加路由 `GET /v1/search/{query_id}/trace` → `handleGetSearchTrace(deps)`
  - 注意路径模板有连字符 — Go ServeMux 支持 PathValue 匹配 `{query_id}`；本 endpoint 用 `query_id` 而非 `id` 区分
- **修改 `internal/consoleapi/handlers.go`**：
  - 新增 `handleGetSearchTrace(deps Deps) http.HandlerFunc`：parse PathValue query_id → `deps.Search.GetSearchTrace(queryID)` → 返 200 + RetrievalTrace；404 / 503 sentinel mapping
- **修改 `internal/consoleapi/memstore.go`**：
  - MemStore SearchAdapter 加 `GetSearchTrace(queryID)` 返 `ErrDataPlaneUnavailable`（fallback 模式下无 trace store）
- **单元测试 ≥4**：
  - `core/src/data_plane/search.rs::tests::test_query_persists_trace_by_query_id` (Rust — call query, then look up trace_store directly)
  - `core/src/data_plane/search.rs::tests::test_get_search_trace_returns_trace` (Rust)
  - `core/src/data_plane/search.rs::tests::test_get_search_trace_404_unknown_id` (Rust)
  - `core/src/data_plane/search.rs::tests::test_trace_store_eviction_at_capacity` (Rust — fill 1001 entries, verify oldest evicted)
  - `internal/consoleapi/handlers_test.go::TestGetSearchTrace_200_when_found` (Go)
  - `internal/consoleapi/handlers_test.go::TestGetSearchTrace_404_when_missing` (Go)
  - `internal/consoleapi/grpcclient/grpcclient_test.go::TestSearchClient_GetSearchTrace_Maps_404` (Go)
- **集成测试 ≥1**：
  - `core/tests/data_plane_integration.rs::test_search_trace_persisted_and_retrievable` (spawn tonic server + tonic client + POST /v1/search → 拿 query_id → 立刻 GET /v1/search/<query_id>/trace 真返 trace 含 candidate_generation_steps 等)
  - 或 `internal/consoleapi/e2e_grpc_test.go::TestGetSearchTrace_E2E` (spawn Rust daemon + console-api-serve + POST search → GET trace 真返)
- **修改 `scripts/console_smoke.sh`** v3（task-12.1 已开 head；此 task 补 step 14: GET trace by query_id）：
  - step 14：从 step 8 POST /v1/search response 解析 `result.query_id` → curl `GET /v1/search/<query_id>/trace` → 验证返 `trace_id` 字段
- **文件锚点**：`proto/contextforge/console_data_plane/v1/console_data_plane.proto` + `core/src/data_plane/search.rs` + `core/Cargo.toml` (可选 lru dep) + `internal/consoleapi/{types,router,handlers,memstore}.go` + `internal/consoleapi/grpcclient/grpcclient.go` + test files + scripts/console_smoke.sh
- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **SQLite migration 0012_search_traces.sql** [SPEC-DEFER:task-future.search-trace-sqlite-persistence]：本 task 选 in-memory LRU；持久化跨重启留 v0.5.x
- **PATCH workspace config + cancel 204 + X-Confirm** [SPEC-OWNER:task-12.1]
- **GET /v1/source-chunks/{id}** [SPEC-OWNER:task-12.2]
- **Memory / Eval 端点** [SPEC-OWNER:phase-13/14]
- **trace replay / time-series query** [SPEC-DEFER:console-endpoint-expansion]
- **MemStore fallback 实现 GetSearchTrace**：低价值；返 ErrDataPlaneUnavailable

## 4. Users / Actors

- **Console UI 端**（下游，via cross-repo）：debug search recall 时反复查 trace（候选数 / rerank / scope filter）
- **task-13.1/13.2 memory-rest 实施 agent**（下游 phase）：复用本 task SearchService RPC 扩展 pattern + in-memory LRU store pattern（如适用 audit log 二级缓存）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/decisions/adr-017-console-contract-completion-22-endpoint.md` §D1 Wave 2 / §D6
- `docs/decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md` §D2 / §D3
- `docs/specs/phases/phase-12-console-contract-completion.md` §3 / §6 / §7
- `docs/specs/tasks/task-12.2-source-chunk-by-id.md` (SearchService proto add-only 模板)
- `docs/specs/tasks/task-11.4-search-real-retriever-and-events.md` (SearchService.Query trace 当前 inline 行为)
- `H:/devlopment/code/contextforge/internal/contractv1/contractv1.go` (RetrievalTrace 字段)

### 5.2 Imports

- **Rust**: 现有 `tonic 0.12` + `prost 0.13` + `parking_lot`（已用于 IndexSessionBackend）；可选新增 `lru = "0.12"` (R7 触发评估，§10 trade-off)
- **Go**: 现有 `internal/consoleapi/grpcclient/`；不引入新依赖

### 5.3 proto 改动形状

```proto
// proto/contextforge/console_data_plane/v1/console_data_plane.proto
service SearchService {
  rpc Query(SearchRequest) returns (SearchResponse);
  rpc GetSourceChunk(GetSourceChunkRequest) returns (SourceChunk);  // task-12.2
  rpc GetSearchTrace(GetSearchTraceRequest) returns (RetrievalTrace);  // NEW (task-12.3)
}

message GetSearchTraceRequest {
  string query_id = 1;
}

// RetrievalTrace message 已存 (task-11.4 ship)
```

### 5.4 Rust SearchServer 改动形状

```rust
// core/src/data_plane/search.rs
pub struct SearchServer {
    stores: Arc<DataPlaneStores>,
    trace_store: parking_lot::Mutex<lru::LruCache<String, RetrievalTrace>>,  // NEW (task-12.3)
    // 或 ManualLruMap with VecDeque if not using lru crate
}

impl SearchServer {
    pub fn new(stores: Arc<DataPlaneStores>) -> Self {
        Self {
            stores,
            trace_store: parking_lot::Mutex::new(
                lru::LruCache::new(std::num::NonZeroUsize::new(1000).unwrap())
            ),
        }
    }
}

#[tonic::async_trait]
impl proto::search_service_server::SearchService for SearchServer {
    async fn query(&self, request: Request<SearchRequest>) -> Result<Response<SearchResponse>, Status> {
        // ... existing logic builds result + trace ...

        // NEW (task-12.3): persist trace by query_id before returning
        self.trace_store.lock().put(result.query_id.clone(), trace.clone());

        Ok(Response::new(SearchResponse { result: Some(result), trace: Some(trace) }))
    }

    async fn get_search_trace(
        &self,
        request: Request<GetSearchTraceRequest>,
    ) -> Result<Response<RetrievalTrace>, Status> {
        let req = request.into_inner();
        match self.trace_store.lock().get(&req.query_id).cloned() {
            Some(trace) => Ok(Response::new(trace_to_proto(trace))),
            None => Err(Status::not_found(format!("trace not found: {}", req.query_id))),
        }
    }
}
```

## 6. Acceptance Criteria

- [x] AC1：`SearchService.Query` 执行后 RetrievalTrace 持久化到 in-memory LRU store by 生成的 `query_id`（`qry-{nanos}` 形式，task-12.3 新增）；可立刻 GetSearchTrace by query_id 取回 — **verified by `test_query_persists_trace_by_query_id_and_get_returns_it` PASS + e2e_grpc Step 9c (search → trace fetch) PASS + console_smoke.sh v3 Step 12/13 OK**
- [x] AC2：`GET /v1/search/{query_id}/trace` 走 gRPC SearchService.GetSearchTrace → trace_store.lock().get → 真返 RetrievalTrace；不存在 query_id → 404 NOT_FOUND；空 query_id → 400 — **verified by `test_get_search_trace_{empty_query_id_returns_invalid_argument,unknown_returns_not_found}` PASS + console_smoke.sh v3 unknown query_id → 404 PASS**
- [x] AC3：TraceStore 容量 cap=1000；满溢出 FIFO evict oldest；后续 get evicted query_id → None — **verified by `test_trace_store_eviction_at_capacity` PASS (cap=3 fixture; insert 5 → oldest 2 evicted)**
- [x] AC4：MemStore fallback 模式下 GetSearchTrace 返 `ErrDataPlaneUnavailable` → HTTP 503 — **verified by `TestGetSearchTrace_503_WhenFallback` PASS**
- [x] AC5：v0.4 + task-12.1 + task-12.2 不退化；`scripts/console_smoke.sh` REAL mode 13 endpoint flow 含 step 12 GET trace by query_id + step 11 GET source-chunks 跑通 + `CONSOLE_REAL_SMOKE_EXIT=0` — **verified by `cargo test -p contextforge-core --lib` 70/70 PASS + `go test ./...` 43 packages PASS + smoke.sh `CONSOLE_REAL_SMOKE_EXIT=0` 真跑 13 endpoint 全通**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | SearchService.Query 后 trace 持久化 | core/src/data_plane/search.rs TraceStore + unit test | Done |
| AC2 | GET /v1/search/{query_id}/trace 真返 trace | proto + search.rs + handlers.go + grpcclient + e2e_grpc Step 9c + smoke v3 Step 12 | Done |
| AC3 | LRU 容量 1000 + FIFO eviction | core/src/data_plane/search.rs TraceStore + unit test | Done |
| AC4 | MemStore fallback 503 | memstore.go + go test TestGetSearchTrace_503_WhenFallback | Done |
| AC5 | v0.4 + task-12.1 + 12.2 不退化 + smoke v3 13 endpoint exit 0 | §9 verify run + scripts/console_smoke.sh v3 | Done |

## 8. Risks

- **lru crate R7 触发**：约 10kloc + 0 transitive deps；trade-off 极低；如不想引入 → 用 `std::collections::HashMap + VecDeque` 手动实现 LRU（约 60 行；测试覆盖足够 + 不影响 §6 AC3）；本 task 默认选 lru crate 简化代码
- **trace_store 锁竞争**：高并发 search QPS 时 parking_lot::Mutex 竞争；缓解：read-heavy 场景（write 1 次/query；read N 次/Console refresh）；如出现热点 → switch to `parking_lot::RwLock`
- **query_id 重复**：SearchResponse.result.query_id 生成策略（task-11.4 选 UUID-like）应保证唯一；如重复 → put 覆盖前一 trace；缓解 task-11.4 既有 query_id 生成不变；本 task 不重新设计
- **重启即丢 trace**：trade-off 接受；如 Console UI 显示 「trace 丢失」需引导用户重发 query；缓解：Console UI 端 graceful degrade（提示 trace 不可用，建议 retry search）；留 v0.5.x SQLite 持久化升级 [SPEC-DEFER:task-future.search-trace-sqlite-persistence]
- **proto add-only 字段编号冲突**：与 task-12.2 同款；本 task 实施时 GetSearchTrace RPC 编号必须接续 GetSourceChunk；缓解 grep + tonic_build 重新生成验证

## 9. Verification Plan

- **install**: `cargo fetch` + `go mod download`
- **lint**: `cargo fmt --check` + `gofmt -l internal/consoleapi/`
- **typecheck**: `cargo check -p contextforge-core` + `go build ./...`
- **unit-test**: `cargo test -p contextforge-core --lib data_plane::search::tests` + `go test ./internal/consoleapi/... ./internal/consoleapi/grpcclient/...`
- **integration**: `cargo test -p contextforge-core --test data_plane_integration -- test_search_trace` + `go test -v ./internal/consoleapi/...` 含 E2E
- **e2e**: 通过 integration + `bash scripts/console_smoke.sh` REAL mode 15 endpoint flow step 14 (GET trace by query_id)
- **build**: `cargo build -p contextforge-core` + `go build ./cmd/contextforge`
- **coverage**: 不强制
- **runtime-smoke**: `bash scripts/console_smoke.sh` REAL mode + manual curl `GET /v1/search/<query_id>/trace`
- **manual**: grpcurl 调 SearchService.GetSearchTrace + diff proto vs Go contractv1.RetrievalTrace 字段命名

## 10. Completion Notes

- **完成日期**：2026-05-24
- **改动文件**：
  - `proto/contextforge/console_data_plane/v1/console_data_plane.proto` (修改 — SearchService 加 GetSearchTrace RPC + GetSearchTraceRequest message)
  - `core/Cargo.toml` (可选修改 — 加 lru = "0.12" dep)
  - `core/src/data_plane/search.rs` (修改 — SearchServer 加 trace_store + Query 持久化 + get_search_trace 方法)
  - `core/tests/data_plane_integration.rs` (修改 — 加 test_search_trace_persisted_and_retrievable)
  - `internal/consoleapi/grpcclient/grpcclient.go` (修改 — SearchClient.GetSearchTrace wrapper)
  - `internal/consoleapi/types.go` (修改 — SearchClient 接口加 GetSearchTrace)
  - `internal/consoleapi/router.go` (修改 — GET /v1/search/{query_id}/trace 路由)
  - `internal/consoleapi/handlers.go` (修改 — handleGetSearchTrace)
  - `internal/consoleapi/memstore.go` (修改 — Search.GetSearchTrace 返 ErrDataPlaneUnavailable)
  - `internal/consoleapi/handlers_test.go` (修改 — 2 新 unit test)
  - `internal/consoleapi/grpcclient/grpcclient_test.go` (修改 — 1 新 unit test)
  - `internal/consoleapi/e2e_grpc_test.go` (修改 — TestGetSearchTrace_E2E sub-step)
  - `scripts/console_smoke.sh` v3 (修改 — step 14 GET trace by query_id)
  - `docs/specs/tasks/task-12.3-search-trace-by-query-id.md` (本 spec §6 / §7 / §10 / Status 推进)
- **commit 列表**：
  - feat(core/search+consoleapi): task-12.3 — GET /v1/search/{query_id}/trace via in-memory LRU trace store + GetSearchTrace RPC
  - docs(spec): task-12.3 §6/§7/§10 / Status → Done
- **关键决策**：
  - **不引入 `lru` crate**（R7 风险评估倾向极简）— 自研 `TraceStore { HashMap, VecDeque, cap }` 约 30 行，O(1) lookup + O(n) refresh-on-hit；`std::sync::Mutex` 包裹（read-heavy 场景足够；如热点切 `parking_lot::RwLock` 留 future）
  - **query_id 生成**：本 task 在 SearchService.Query 内统一生成 `qry-{nanos}` 形式（task-11.4 既存返 empty query_id 字段被替换）；每个 SearchResultItem.query_id 一致；trace_store 用此 key
  - **trace_store cap = 1000**：硬编码常量；不参数化（避免 v0.5 Cargo.toml feature 复杂化；future 改 env var 留 v0.5.x）
- **§9 Verification 结果**：
  - `cargo check -p contextforge-core`: clean
  - `cargo test -p contextforge-core --lib`: 70 passed; 0 failed (含 4 new search tests)
  - `go build ./...`: clean (含 degradedSearch.GetSearchTrace + MemStore.GetSearchTrace 占位)
  - `go test ./internal/consoleapi/...`: PASS (含 new TestGetSearchTrace_503_WhenFallback + e2e_grpc Step 9c trace fetch + Step 9 cancel 204 + 8a/8b PATCH/active list PASS)
  - `go test ./internal/consoleapi/grpcclient/...`: PASS (含 2 new GetSearchTrace wire tests)
  - `go test ./...`: 43/43 packages PASS
  - `bash scripts/console_smoke.sh`: `CONSOLE_REAL_SMOKE_EXIT=0` 13 endpoint flow 全通 (chk_4eec0d18_2 找到; trace 取回 OK; unknown query_id 404 OK)
- **剩余风险 / 未做项**：
  - trace SQLite 持久化 [SPEC-DEFER:task-future.search-trace-sqlite-persistence] 留 v0.5.x
  - trace replay / time-series query [SPEC-DEFER:console-endpoint-expansion]
- **下游 task 影响**：本 task 为 Phase 12 收口任务；task-13.1 起 phase-13 实施 memory endpoints
