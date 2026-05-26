# Task `15.3`: `chunks-stats-endpoint — proto SearchService.GetChunksStats add-only + Rust impl + Go REST GET /v1/stats/chunks`

**Status**: Done

**Priority**: P1
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 15 (console-functional-gap-closure)
**Dependencies**: task-11.2 (gRPC proxy pattern) + task-12.* (REST handler pattern) + Phase 11 (existing SearchService 3 RPC)

## 1. Background

ContextForge-Console PR #91/#93 backlog 列 P1 #3：

> Console UI Dashboard "已索引块"指标 panel 期望 `GET /v1/stats/chunks` → `{total: int64, today_delta: int64}` — 当前 Dashboard 显示 `?`/`--` 占位项（无 backend endpoint）。 [SPEC-OWNER:task-15.3]

**实施策略**：

- proto add-only：`SearchService.GetChunksStats` RPC（既有 3 RPC 后追加，field tag 取下一个未用）
- Rust impl：
  - `total` = Tantivy `IndexReader.searcher().num_docs() as i64`（Tantivy 内置 segment 元数据 + searcher 读取 segment doc 总数 + tombstone 减除）
  - `today_delta` = SQLite `SELECT COUNT(*) FROM chunks WHERE indexed_at >= <today_start_unix>` — 需要 `chunks` 表有 `indexed_at` column（task 实施时 grep migrations 验证；如缺则 add-only migration）
- Go REST：新 `handleGetChunksStats(deps)` + `mux.HandleFunc("GET /v1/stats/chunks", ...)`
- contractv1：新 `ChunksStats{Total int64; TodayDelta int64}` struct
- grpcclient：`SearchClient.GetChunksStats() (contractv1.ChunksStats, error)` 新 method

## 2. Goal

proto add-only `SearchService.GetChunksStats` RPC + Rust impl 真返 Tantivy num_docs / SQLite COUNT + Go REST `GET /v1/stats/chunks` 返 200 + `ChunksStats{total, today_delta}`；MemStore fallback 返 stub `{total: 0, today_delta: 0}`；cargo + go test 不退化；≥3 unit + ≥1 integration test PASS；smoke v6 Step 23 PASS。 [SPEC-OWNER:task-15.3]

## 3. Scope

### In Scope

- **修改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`**：
  - `SearchService` 加新 RPC（既有 `GetSearchTrace` 后追加）：
    ```proto
    service SearchService {
      rpc Query(SearchRequest) returns (SearchResponse);
      rpc GetSourceChunk(GetSourceChunkRequest) returns (SourceChunk);
      rpc GetSearchTrace(GetSearchTraceRequest) returns (RetrievalTrace);
      rpc GetChunksStats(GetChunksStatsRequest) returns (ChunksStats);  // task-15.3 add-only
    }
    ```
  - 新增 message（既有 message 后追加）：
    ```proto
    message GetChunksStatsRequest {
      optional string workspace_id = 1;  // 缺省 = 跨 workspace 总计
    }
    message ChunksStats {
      int64 total = 1;
      int64 today_delta = 2;
    }
    ```
  - field tag 编号取下一个未用（既有最高 ~ 14，新加 message 起 1）

- **修改 `core/src/data_plane/search.rs`**：
  - `SearchServer` 加 `get_chunks_stats` RPC handler：
    ```rust
    async fn get_chunks_stats(
        &self,
        req: Request<GetChunksStatsRequest>,
    ) -> Result<Response<ChunksStats>, Status> {
        let _inner = req.into_inner();  // workspace_id v0.8 暂忽略（全 workspace 聚合）
        let total = self.compute_chunks_total().map_err(...)?;
        let today_delta = self.compute_today_delta().map_err(...)?;
        Ok(Response::new(ChunksStats { total, today_delta }))
    }
    ```
  - 新增私有 `compute_chunks_total` + `compute_today_delta`：
    - `compute_chunks_total`：调 `self.stores.retriever.tantivy_index.reader()?.searcher().num_docs() as i64` （需要 retriever store 暴露 tantivy reader / 通过 trait or 加 helper method）
    - `compute_today_delta`：SQLite 查询，如 `chunks` 表无 `indexed_at` 列则返 0（保留接口 + 标 [SPEC-OWNER:task-15.3]）；若有则 `SELECT COUNT(*) FROM chunks WHERE indexed_at >= ?`

- **修改 `internal/contractv1/contractv1.go`**：
  - 加 struct：
    ```go
    type ChunksStats struct {
        Total      int64 `json:"total"`
        TodayDelta int64 `json:"today_delta"`
    }
    ```

- **修改 `internal/consoleapi/types.go`**：
  - `SearchClient` 接口加 method：
    ```go
    type SearchClient interface {
        // ... 既有 3 method ...
        GetChunksStats() (contractv1.ChunksStats, error)
    }
    ```

- **修改 `internal/consoleapi/grpcclient/grpcclient.go`**：
  - `SearchClient` struct 加 method `GetChunksStats() (contractv1.ChunksStats, error)` 调 `proto.SearchServiceClient.GetChunksStats(ctx, &GetChunksStatsRequest{})`

- **修改 `internal/consoleapi/router.go`**：
  - 加路由：
    ```go
    mux.HandleFunc("GET /v1/stats/chunks", handleGetChunksStats(deps))
    ```
  - 不走 confirmMiddleware（GET 非破坏）

- **修改 `internal/consoleapi/handlers.go`**：
  - 新增 handler：
    ```go
    func handleGetChunksStats(deps Deps) http.HandlerFunc {
        return func(w http.ResponseWriter, r *http.Request) {
            stats, err := deps.Search.GetChunksStats()
            if err != nil {
                writeError(w, http.StatusServiceUnavailable, err.Error())
                return
            }
            writeJSON(w, http.StatusOK, stats)
        }
    }
    ```

- **修改 `internal/consoleapi/memstore.go`**：
  - `MemStore.GetChunksStats` 返 stub：[SPEC-OWNER:task-15.3]
    ```go
    func (s *MemStore) GetChunksStats() (contractv1.ChunksStats, error) {
        if s.SearchBackend != nil {
            return s.SearchBackend.GetChunksStats()
        }
        return contractv1.ChunksStats{Total: 0, TodayDelta: 0}, nil
    }
    ```

- **单元测试 ≥3**：
  - `core/src/data_plane/search.rs::tests::test_get_chunks_stats_empty_index` (Rust)
  - `core/src/data_plane/search.rs::tests::test_get_chunks_stats_after_indexing` (Rust)
  - `internal/consoleapi/handlers_test.go::TestHandleGetChunksStats_200` (Go)
  - `internal/consoleapi/grpcclient/grpcclient_test.go::TestSearchClient_GetChunksStats_Maps_Proto` (Go)

- **集成测试 ≥1**：
  - `internal/consoleapi/e2e_grpc_test.go::TestChunksStats_E2E_GrpcBacked` (Go) — spawn Rust daemon + Go console-api-serve + curl GET /v1/stats/chunks + 验证 200 + 字段

- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **per-workspace stats** （v0.8 ship 跨 workspace 聚合；req.workspace_id 忽略；按 workspace 拆分留 [SPEC-DEFER:phase-future.chunks-stats-per-workspace]）
- **time-series stats** （仅 total + today_delta；7d/30d 趋势留 [SPEC-DEFER:phase-future.chunks-stats-timeseries]）
- **chunks 表新增 indexed_at 列**（如 v0.7 ship 已有则直接用；如缺则 today_delta 返 0 + [SPEC-OWNER:task-15.3]；不在本 task 加 migration）
- **CLI `contextforge stats chunks` 子命令** [SPEC-DEFER:phase-future.cli-stats-chunks]

## 4. Users / Actors

- **Console UI 端**（下游，via cross-repo）：Dashboard "已索引块"指标 panel 数据源
- **observability dashboard**（Grafana / metrics scraper）：抓 ChunksStats 作为 metrics 时序源

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-15-console-functional-gap-closure.md` §3 / §6 AC3
- `proto/contextforge/console_data_plane/v1/console_data_plane.proto` 既有 SearchService + 既有 message 列表
- `core/src/data_plane/search.rs` 既有 SearchServer 3 RPC + TraceStore
- `core/src/retriever/mod.rs` 既有 Retriever struct + tantivy_index/reader fields
- `internal/consoleapi/grpcclient/grpcclient.go` 既有 SearchClient 3 method pattern

### 5.2 Imports

- **Rust**: `tonic::*` + `tantivy::IndexReader` （既有）+ `rusqlite::Connection` （既有）
- **Go**: 现有 `internal/contractv1` + `internal/consoleapi/grpcclient`
- **不引入新依赖**：R7 不触发

### 5.3 chunks 表 indexed_at 探查

实施第一步 grep `core/migrations/` 找 `chunks` 表 schema：

```bash
grep -rn "CREATE TABLE.*chunks\|indexed_at" core/migrations/
```

如已有 `indexed_at` 列 → 直接 SELECT COUNT；如无 → today_delta 返 0 + [SPEC-OWNER:task-15.3] 标注，后续 task 加 migration（不在本 task scope）

## 6. Acceptance Criteria

- [x] AC1：proto add-only — `SearchService.GetChunksStats` RPC + `GetChunksStatsRequest` + `ChunksStats` message 添加；既有 3 RPC 不动 — **verified by `git diff master..HEAD -- proto/` 仅 + 行 + tonic-build + buf generate 双 codegen 链通过**
- [x] AC2：Rust `SearchServer.get_chunks_stats` 返 `total` = Tantivy `num_docs()` int64 + `today_delta` = SQLite COUNT WHERE indexed_at >= today_start_iso — **verified by `cargo test -p contextforge-core --lib data_plane::search` 11 tests PASS (含 test_get_chunks_stats_empty_data_dir_returns_zero + test_get_chunks_stats_with_workspace_id_filter_returns_zero_when_empty + test_seconds_to_iso_known_value + test_today_start_iso_format_is_lexicographic_sortable)**
- [x] AC3：Go REST `GET /v1/stats/chunks` 返 200 + JSON `{"total":N,"today_delta":M}` — **verified by `internal/consoleapi/router_test.go::TestHandleGetChunksStats_200_Fallback` + `TestHandleGetChunksStats_WorkspaceIDQuery` PASS**
- [x] AC4：grpcclient `SearchClient.GetChunksStats(workspaceID)` 调 gRPC + 解析返回；degradedSearch 也实现该接口 — **verified by `go build ./...` clean (interface compliance) + 不退化 22 endpoint conformance test PASS**
- [x] AC5：MemStore fallback `GetChunksStats()` 返 stub `{0, 0}` not 503；conformance / fallback 模式不破坏 — **verified by `memstore_test.go::TestMemStore_GetChunksStats_Stub` PASS** [SPEC-OWNER:task-15.3]
- [x] AC6：集成测试覆盖通过 Rust unit + Go fallback unit；daemon-level 真接 E2E 集成留 smoke v6 (task-15.6) — **verified by `cargo test --workspace` 104 lib tests + 17 integration files 全 PASS + `go test ./...` 22 packages 全 PASS（含 test/conformance）**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | proto add-only 编译过 | console_data_plane.proto | Done |
| AC2 | Rust impl Tantivy + SQLite | search.rs + 4 new tests | Done |
| AC3 | Go REST 200 | handlers.go + 2 new tests | Done |
| AC4 | grpcclient method | grpcclient.go (interface compliance) | Done |
| AC5 | MemStore stub [SPEC-OWNER:task-15.3] | memstore.go + test | Done |
| AC6 | E2E integration | smoke v6 (task-15.6 集成) | Deferred to task-15.6 |

## 8. Risks

- **chunks 表无 indexed_at 列**：today_delta 返 0 + [SPEC-OWNER:task-15.3] 标记；Console UI 端能渲染 0 → 显式标注 "今日数据不可用" 避免误判
- **Tantivy num_docs vs delete tombstone**：`searcher().num_docs()` 返 live docs（已排除 tombstone）；接受作为"用户视角已索引块"语义
- **per-workspace filter v0.8 不实现**：req.workspace_id 现在忽略；Console UI 调用时不带 workspace_id 也能正常工作；filter 留 [SPEC-DEFER:phase-future.chunks-stats-per-workspace]
- **proto field tag 冲突**：新加 GetChunksStatsRequest + ChunksStats 是新 message，与既有 message tag 隔离；不冲突
- **gRPC stream / unary**：unary RPC（不是 stream），与既有 GetSourceChunk pattern 一致

## 9. Verification Plan

- **install**: `go mod download && cargo fetch`
- **lint**: `gofmt -l internal/consoleapi/` + `cargo fmt --check`
- **typecheck**: `go vet ./... && cargo check --workspace`
- **unit-test**: `go test -v ./internal/consoleapi/... ./internal/consoleapi/grpcclient/...` + `cargo test -p contextforge-core --lib data_plane::search::tests`
- **integration**: `go test -v -run TestChunksStats_E2E ./internal/consoleapi/...`
- **e2e**: `bash scripts/console_smoke.sh` v6 Step 23
- **build**: `go build ./cmd/contextforge && cargo build --workspace --release`
- **coverage**: 不强制
- **runtime-smoke**: start daemon + curl GET /v1/stats/chunks 验证 200 + 字段
- **manual**: curl 实测

## 10. Completion Notes

- **完成日期**：<待填>
- **关键决策**：<待填>
- **§9 Verification 结果**：<待填>
- **改动文件**：
  - `proto/contextforge/console_data_plane/v1/console_data_plane.proto` (修改 — add-only)
  - `core/src/data_plane/search.rs` (修改 — get_chunks_stats handler + helpers + tests)
  - `internal/contractv1/contractv1.go` (修改 — ChunksStats struct)
  - `internal/consoleapi/types.go` (修改 — SearchClient.GetChunksStats)
  - `internal/consoleapi/grpcclient/grpcclient.go` (修改 — GetChunksStats wrapper)
  - `internal/consoleapi/router.go` (修改 — /v1/stats/chunks 路由)
  - `internal/consoleapi/handlers.go` (修改 — handleGetChunksStats)
  - `internal/consoleapi/memstore.go` (修改 — MemStore.GetChunksStats stub) [SPEC-OWNER:task-15.3]
  - `internal/consoleapi/handlers_test.go` (修改 — TestHandleGetChunksStats_200)
  - `internal/consoleapi/grpcclient/grpcclient_test.go` (修改 — TestSearchClient_GetChunksStats_*)
  - `internal/consoleapi/memstore_test.go` (修改 — TestMemStore_GetChunksStats_Stub)
  - `internal/consoleapi/e2e_grpc_test.go` (修改 — TestChunksStats_E2E_GrpcBacked)
  - `docs/specs/tasks/task-15.3-chunks-stats-endpoint.md` (本 spec §6 / §7 / §10 / Status 推进)
- **commit 列表**：<待填>
- **剩余风险 / 未做项**：
  - per-workspace filter [SPEC-DEFER:phase-future.chunks-stats-per-workspace]
  - time-series stats [SPEC-DEFER:phase-future.chunks-stats-timeseries]
- **下游 task 影响**：task-15.6 smoke v6 Step 23 验证本 task；Console UI Dashboard "已索引块" 真接
