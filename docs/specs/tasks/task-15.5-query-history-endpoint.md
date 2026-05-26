# Task `15.5`: `query-history-endpoint — proto SearchService.ListQueries add-only + Rust TraceStore.list + Go REST GET /v1/queries`

**Status**: Ready

**Priority**: P1
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 15 (console-functional-gap-closure)
**Dependencies**: task-12.3 (TraceStore in-memory ring buffer) + task-11.2 (gRPC proxy pattern) + Phase 12 (SearchService 既有)

## 1. Background

ContextForge-Console PR #91/#93 backlog 列 P1 #5：

> Console UI Dashboard "最近查询"列表 panel 期望 `GET /v1/queries?limit=N` → `[]QueryRecord` — 当前 Console 只能逐个 `GET /v1/search/<query_id>/trace`（需先知道 query_id），无法发现历史查询。

**实施策略**：

- proto add-only：`SearchService.ListQueries` RPC（既有 4 RPC 后追加 — Query / GetSourceChunk / GetSearchTrace / GetChunksStats → +ListQueries）
- Rust impl：
  - `TraceStore.list(limit)` 新方法 — 既有 `TraceStore` 是 `HashMap<String, PbRetrievalTrace>` + `VecDeque<String>` order LRU；list 按 order 取最近 N 个（DESC by insertion order = reverse VecDeque iteration）
  - 返回 `Vec<QueryRecord>` — 从 PbRetrievalTrace 抽 `query_id` + `query` + 时序信息
  - default limit = 20；上限 = 100
- Go REST：新 `handleListQueries(deps)` + `mux.HandleFunc("GET /v1/queries", ...)`
- contractv1：新 `QueryRecord{query_id, query, ts_unix, workspace_id}` struct

## 2. Goal

proto add-only `SearchService.ListQueries` RPC + Rust `TraceStore.list(limit)` 真返 in-memory ring buffer 按时序 + Go REST `GET /v1/queries?limit=N` 返 200 + `[]QueryRecord`；MemStore fallback 返 in-memory query history；cargo + go test 不退化；≥3 unit + ≥1 integration test PASS；smoke v6 Step 25 PASS。

## 3. Scope

### In Scope

- **修改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`**：
  - `SearchService` 加新 RPC：
    ```proto
    service SearchService {
      rpc Query(SearchRequest) returns (SearchResponse);
      rpc GetSourceChunk(GetSourceChunkRequest) returns (SourceChunk);
      rpc GetSearchTrace(GetSearchTraceRequest) returns (RetrievalTrace);
      rpc GetChunksStats(GetChunksStatsRequest) returns (ChunksStats);  // task-15.3
      rpc ListQueries(ListQueriesRequest) returns (ListQueriesResponse);  // task-15.5 add-only
    }
    ```
  - 新增 message：
    ```proto
    message ListQueriesRequest {
      optional int32 limit = 1;  // default 20; max 100
    }
    message QueryRecord {
      string query_id = 1;
      string query = 2;
      int64 ts_unix = 3;
      optional string workspace_id = 4;
    }
    message ListQueriesResponse {
      repeated QueryRecord records = 1;
    }
    ```

- **修改 `core/src/data_plane/search.rs`**：
  - `TraceStore` 加 `list(limit)` 方法：
    ```rust
    impl TraceStore {
        pub fn list(&self, limit: usize) -> Vec<QueryRecord> {
            let lim = limit.clamp(1, 100);
            // iterate self.order in reverse (most recent first), take limit
            let mut out = Vec::with_capacity(lim);
            for key in self.order.iter().rev().take(lim) {
                if let Some(trace) = self.map.get(key) {
                    out.push(QueryRecord {
                        query_id: trace.query_id.clone(),
                        query: trace.query.clone(),
                        ts_unix: trace.ts_unix.unwrap_or(0),
                        workspace_id: trace.workspace_id.clone(),
                    });
                }
            }
            out
        }
    }
    ```
    注意：既有 TraceStore.put 时 value 是 PbRetrievalTrace；需要 trace 携带 `ts_unix` + `workspace_id` 信息。如既有 PbRetrievalTrace 缺这些字段 → task 实施时检查 + add-only 加（[SPEC-OWNER:task-15.5]）；如已有则直接读
  - `SearchServer.list_queries` 新 RPC handler：
    ```rust
    async fn list_queries(
        &self,
        req: Request<ListQueriesRequest>,
    ) -> Result<Response<ListQueriesResponse>, Status> {
        let inner = req.into_inner();
        let limit = inner.limit.unwrap_or(20) as usize;
        let trace_store = self.trace_store.lock().map_err(|_| Status::internal("trace store lock poisoned"))?;
        let records = trace_store.list(limit);
        Ok(Response::new(ListQueriesResponse { records }))
    }
    ```

- **修改 `internal/contractv1/contractv1.go`**：
  - 加 struct：
    ```go
    type QueryRecord struct {
        QueryID     string `json:"query_id"`
        Query       string `json:"query"`
        TsUnix      int64  `json:"ts_unix"`
        WorkspaceID string `json:"workspace_id,omitempty"`
    }
    ```

- **修改 `internal/consoleapi/types.go`**：
  - `SearchClient` 接口加 method：
    ```go
    type SearchClient interface {
        // ... 既有 + GetChunksStats（task-15.3）...
        ListQueries(limit int) ([]contractv1.QueryRecord, error)
    }
    ```

- **修改 `internal/consoleapi/grpcclient/grpcclient.go`**：
  - `SearchClient` struct 加 `ListQueries` method 调 gRPC + map proto → contractv1

- **修改 `internal/consoleapi/router.go`**：
  - 加路由：
    ```go
    mux.HandleFunc("GET /v1/queries", handleListQueries(deps))
    ```

- **修改 `internal/consoleapi/handlers.go`**：
  - 新增 handler：
    ```go
    func handleListQueries(deps Deps) http.HandlerFunc {
        return func(w http.ResponseWriter, r *http.Request) {
            limit := 20
            if v := r.URL.Query().Get("limit"); v != "" {
                if n, err := strconv.Atoi(v); err == nil && n > 0 && n <= 100 {
                    limit = n
                }
            }
            records, err := deps.Search.ListQueries(limit)
            if err != nil {
                writeError(w, http.StatusServiceUnavailable, err.Error())
                return
            }
            writeJSON(w, http.StatusOK, records)
        }
    }
    ```

- **修改 `internal/consoleapi/memstore.go`**：
  - `MemStore.ListQueries(limit)` 实现：MemStore 已有 traceCache (task-15.1) 可作为来源；从 traceCache 抽 query+query_id+ts 转 QueryRecord
    ```go
    func (s *MemStore) ListQueries(limit int) ([]contractv1.QueryRecord, error) {
        if s.SearchBackend != nil {
            return s.SearchBackend.ListQueries(limit)
        }
        s.mu.Lock()
        defer s.mu.Unlock()
        out := make([]contractv1.QueryRecord, 0, len(s.traceCache))
        for qid, trace := range s.traceCache {
            out = append(out, contractv1.QueryRecord{
                QueryID:     qid,
                Query:       trace.Query,
                TsUnix:      0,  // stub timestamp（fallback 不存 ts） [SPEC-OWNER:task-15.5]
                WorkspaceID: "",
            })
        }
        // 不排序（fallback 简化）
        if limit > 0 && limit < len(out) { out = out[:limit] }
        return out, nil
    }
    ```

- **单元测试 ≥3** + **集成测试 ≥1**：
  - Rust: `core/src/data_plane/search.rs::tests::test_trace_store_list_returns_recent_first`
  - Rust: `core/src/data_plane/search.rs::tests::test_list_queries_rpc_default_limit_20`
  - Go: `internal/consoleapi/handlers_test.go::TestHandleListQueries_DefaultLimit20`
  - Go: `internal/consoleapi/grpcclient/grpcclient_test.go::TestSearchClient_ListQueries_Maps_Proto`
  - Go integration: `internal/consoleapi/e2e_grpc_test.go::TestListQueries_E2E_GrpcBacked`

- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **TraceStore SQLite 持久化**（v0.8 ship in-memory ring buffer 不变；持久化留 [SPEC-DEFER:phase-16.tracestore-sqlite-persist]）
- **分页 / cursor**（v0.8 simple LIMIT；cursor 留 [SPEC-DEFER:phase-future.list-endpoints-pagination]）
- **workspace_id filter**（v0.8 全部 query 列出；按 workspace 过滤留 [SPEC-DEFER:phase-future.queries-list-workspace-filter]）
- **时间窗口 filter (since=/until=)** [SPEC-DEFER:phase-future.queries-list-time-window]
- **query 全文搜索（grep query 内容）** [SPEC-DEFER:phase-future.queries-list-fulltext-grep]

## 4. Users / Actors

- **Console UI 端**（下游，via cross-repo）：Dashboard "最近查询"列表 panel 数据源
- **debug session**：开发者快速看最近 N 个查询用于调参 / debug retrieval

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-15-console-functional-gap-closure.md` §3 / §6 AC5
- `docs/specs/tasks/task-12.3-search-trace-by-query-id.md` (TraceStore 既有)
- `core/src/data_plane/search.rs` 既有 TraceStore + SearchServer
- `internal/contractv1/contractv1.go::RetrievalTrace`
- `proto/contextforge/console_data_plane/v1/console_data_plane.proto` 既有 RetrievalTrace message + SearchService

### 5.2 Imports

- **Rust**: 现有 `tonic` + `std::collections`
- **Go**: 现有 stdlib `strconv`；现有 `internal/contractv1`
- **不引入新依赖**：R7 不触发

### 5.3 PbRetrievalTrace 字段补充判定

实施第一步 grep `RetrievalTrace` 在 proto 中的字段：

```bash
grep -A 30 "^message RetrievalTrace" proto/contextforge/console_data_plane/v1/console_data_plane.proto
```

期望字段含 `query_id` + `query` + 可选 `ts_unix` + `workspace_id`。如缺 ts_unix → task 实施时 add-only 加（field tag 取下一未用；属本 task in-scope `[SPEC-OWNER:task-15.5]`）

## 6. Acceptance Criteria

- [ ] AC1：proto add-only — `SearchService.ListQueries` RPC + 3 message 添加；既有 4 RPC 不动 — **verified by `git diff` 仅 + 行 + tonic-build 编译通过**
- [ ] AC2：Rust `TraceStore.list(limit)` 返按 insertion order DESC（最新优先）；limit clamp 1..=100 default 20；空 store → `[]` — **verified by `tests::test_trace_store_list_returns_recent_first` PASS**
- [ ] AC3：Rust `SearchServer.list_queries` RPC 调 TraceStore.list + map 返回 — **verified by `tests::test_list_queries_rpc_default_limit_20` PASS**
- [ ] AC4：Go REST `GET /v1/queries?limit=N` 返 200 + JSON `[]QueryRecord`；不带 limit → default 20 — **verified by `handlers_test.go::TestHandleListQueries_DefaultLimit20` PASS**
- [ ] AC5：grpcclient `SearchClient.ListQueries(limit)` 调 gRPC + 解析返回 — **verified by `grpcclient_test.go::TestSearchClient_ListQueries_Maps_Proto` PASS**
- [ ] AC6：MemStore fallback `ListQueries` 返 traceCache 内容 转 QueryRecord；空 cache → `[]` — **verified by `memstore_test.go::TestMemStore_ListQueries_FromCache` PASS**
- [ ] AC7：集成 `TestListQueries_E2E_GrpcBacked`：连发 3 POST /v1/search → GET /v1/queries → 返 ≥3 项 — **verified by `go test -v -run TestListQueries_E2E ./internal/consoleapi/...` PASS**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | proto add-only | console_data_plane.proto | Ready |
| AC2 | TraceStore.list 排序 | search.rs + test | Ready |
| AC3 | list_queries RPC | search.rs + test | Ready |
| AC4 | Go REST 200 + limit | handlers.go + test | Ready |
| AC5 | grpcclient mapping | grpcclient.go + test | Ready |
| AC6 | MemStore fallback | memstore.go + test | Ready |
| AC7 | E2E integration | e2e_grpc_test.go | Ready |

## 8. Risks

- **PbRetrievalTrace 字段不全**：如 proto 缺 ts_unix / workspace_id → add-only 加（field tag 取下一个未用）；接受 BACKWARD COMPAT 风险（add-only 不破坏旧 client）
- **TraceStore 容量**：既有 v0.7 task-12.3 既有 ring buffer cap 是 hardcode（grep 确认大小，预计 256）；ListQueries 不破坏既有 cap 行为
- **proto field tag 冲突**：新加 QueryRecord 是新 message（tags 从 1 开始）；与既有 message 隔离
- **MemStore fallback ts_unix=0**：UI 端拿到 ts=0 可显示 "时间未知" 或 fallback timestamp；接受作为 fallback degradation；可加 `time.Now().Unix()` 在 traceCache write 时（task 实施时决定）
- **既有 RetrievalTrace 已 ship**：modify 已 ship message 加新字段 = add-only safe（proto3 默认值）

## 9. Verification Plan

- **install**: `go mod download && cargo fetch`
- **lint**: `gofmt -l internal/consoleapi/` + `cargo fmt --check`
- **typecheck**: `go vet ./... && cargo check --workspace`
- **unit-test**: `go test -v ./internal/consoleapi/... ./internal/consoleapi/grpcclient/...` + `cargo test -p contextforge-core --lib data_plane::search::tests`
- **integration**: `go test -v -run TestListQueries_E2E ./internal/consoleapi/...`
- **e2e**: `bash scripts/console_smoke.sh` v6 Step 25
- **build**: `go build ./cmd/contextforge && cargo build --workspace --release`
- **coverage**: 不强制
- **runtime-smoke**: start daemon + 3 次 POST /v1/search + curl GET /v1/queries?limit=3 验证
- **manual**: curl 实测

## 10. Completion Notes

- **完成日期**：<待填>
- **关键决策**：<待填>
- **§9 Verification 结果**：<待填>
- **改动文件**：
  - `proto/contextforge/console_data_plane/v1/console_data_plane.proto` (修改 — add-only)
  - `core/src/data_plane/search.rs` (修改 — TraceStore.list + list_queries RPC handler + tests)
  - `internal/contractv1/contractv1.go` (修改 — QueryRecord)
  - `internal/consoleapi/types.go` (修改 — SearchClient.ListQueries)
  - `internal/consoleapi/grpcclient/grpcclient.go` (修改 — ListQueries wrapper)
  - `internal/consoleapi/router.go` (修改 — GET /v1/queries)
  - `internal/consoleapi/handlers.go` (修改 — handleListQueries)
  - `internal/consoleapi/memstore.go` (修改 — MemStore.ListQueries)
  - `internal/consoleapi/handlers_test.go` (修改 — TestHandleListQueries_*)
  - `internal/consoleapi/grpcclient/grpcclient_test.go` (修改 — TestSearchClient_ListQueries_*)
  - `internal/consoleapi/memstore_test.go` (修改 — TestMemStore_ListQueries_FromCache)
  - `internal/consoleapi/e2e_grpc_test.go` (修改 — TestListQueries_E2E_GrpcBacked)
  - `docs/specs/tasks/task-15.5-query-history-endpoint.md` (本 spec §6 / §7 / §10 / Status 推进)
- **commit 列表**：<待填>
- **剩余风险 / 未做项**：
  - SQLite 持久化 [SPEC-DEFER:phase-16.tracestore-sqlite-persist]
  - 分页 [SPEC-DEFER:phase-future.list-endpoints-pagination]
  - workspace filter [SPEC-DEFER:phase-future.queries-list-workspace-filter]
- **下游 task 影响**：task-15.6 smoke v6 Step 25 验证本 task；Console UI Dashboard "最近查询" 真接
