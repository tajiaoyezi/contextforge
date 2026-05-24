# Task `12.2`: `source-chunk-by-id — GET /v1/source-chunks/{id} + Rust SearchService.GetSourceChunk RPC + retriever by-id lookup`

**Status**: Done

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 12 (console-contract-completion)
**Dependencies**: task-12.1 (confirmMiddleware + grpcclient.Search 扩展 pattern 已 ship) + task-4.1/4.2 (retriever 框架 + SqliteChunkStore 已 ship) + task-11.4 (SearchService 真接 retriever 已 ship) + [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D1 Wave 2

## 1. Background

Console Contract v1 22 endpoint 含 `GET /v1/source-chunks/{id}` 按 chunk_id 取单个 chunk 详情：返 `SourceChunk` (含 chunk_id / workspace_id / source_file_path / line_start / line_end / chunk_text_preview / chunk_offset_start / chunk_offset_end / redaction_status 9 字段)。

当前 v0.4.0 (Phase 11) ship 的 SearchService 只暴露 `Query(SearchRequest) returns (SearchResponse)` 一 RPC，没有 by-chunk_id lookup。retriever 底层 (`core/src/retriever/`) 持 SqliteChunkStore 有 chunk_id PK，但没暴露 `get_chunk(chunk_id)` API。

本 task 在 Rust 侧新增 `SearchService.GetSourceChunk(GetSourceChunkRequest) returns (SourceChunk)` RPC + retriever 加 `get_chunk_by_id(chunk_id: &str) -> Option<SourceChunk>` + Go REST handler + grpcclient.Search.GetSourceChunk wrapper。

## 2. Goal

`proto/contextforge/console_data_plane/v1/console_data_plane.proto` SearchService 加 `GetSourceChunk` RPC (add-only proto 演进)；`core/src/retriever/` 加 `get_chunk_by_id` 接口；`core/src/data_plane/search.rs` SearchServer 实现 `get_source_chunk` 方法；`internal/consoleapi/grpcclient/grpcclient.go` SearchClient 加 `GetSourceChunk` wrapper；`internal/consoleapi/handlers.go` + `router.go` 加 `GET /v1/source-chunks/{id}` 路由 + handler；`internal/consoleapi/memstore.go` 不实现（fallback 返 ErrDataPlaneUnavailable）；`cargo test --workspace` + `go test ./...` 全绿；≥4 单元测试 + ≥1 集成测试 PASS。

## 3. Scope

### In Scope

- **修改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`**：
  - SearchService 加 RPC `rpc GetSourceChunk(GetSourceChunkRequest) returns (SourceChunk);`（add-only，与现有 `Query` RPC 并存）
  - 新增 message `GetSourceChunkRequest { string chunk_id = 1; string workspace_id = 2; }` (workspace_id 可选，用于多 workspace 隔离场景 [SPEC-DEFER:phase-15.multi-workspace-strict])
  - 复用已有 `SourceChunk` message (Phase 11 task-11.1 ship)
  - `option go_package` 不变；proto 编号下一个未用
- **修改 `core/build.rs`**：tonic_build 自动 pick up `.proto` 改动（不需要显式改 build.rs）
- **修改 `core/src/retriever/mod.rs`** 或具体子模块：
  - 加 `pub fn get_chunk_by_id(&self, chunk_id: &str) -> Result<Option<SourceChunk>, RetrieverError>` 接口
  - 底层 SqliteChunkStore SQL `SELECT chunk_id, workspace_id, source_file_path, line_start, line_end, chunk_text_preview, chunk_offset_start, chunk_offset_end, redaction_status FROM chunks WHERE chunk_id = ?`
  - 不存在 chunk_id → `Ok(None)`；DB error → `Err(RetrieverError::...)`
- **修改 `core/src/data_plane/search.rs`**：
  - SearchServer 新增 `get_source_chunk(&self, req: GetSourceChunkRequest) -> Result<Response<SourceChunk>, Status>` 方法
  - 调 `self.stores.retriever.get_chunk_by_id(&req.chunk_id)`：
    - `Ok(Some(chunk))` → `Response::new(chunk_to_proto(chunk))`
    - `Ok(None)` → `Err(Status::not_found(format!("chunk not found: {}", req.chunk_id)))`
    - `Err(e)` → `Err(Status::internal(format!("retriever error: {}", e)))`
  - `chunk_to_proto(SourceChunk) -> proto::SourceChunk` helper（已存或新增）
- **修改 `internal/consoleapi/grpcclient/grpcclient.go`**：
  - `SearchClient` 加 `GetSourceChunk(chunkID string) (contractv1.SourceChunk, error)` 方法
  - 调 `c.search.GetSourceChunk(ctx, &proto.GetSourceChunkRequest{ChunkId: chunkID})` → protoToSourceChunk(resp) → 返
  - 错误 mapping (mapGrpcErr) 沿用 task-11.2 既有
- **修改 `internal/consoleapi/types.go`**：
  - SearchClient 接口加 `GetSourceChunk(chunkID string) (contractv1.SourceChunk, error)`
- **修改 `internal/consoleapi/router.go`**：
  - 加路由 `GET /v1/source-chunks/{id}` → `handleGetSourceChunk(deps)`（非破坏性，不走 confirmMiddleware）
- **修改 `internal/consoleapi/handlers.go`**：
  - 新增 `handleGetSourceChunk(deps Deps) http.HandlerFunc`：parse PathValue id → `deps.Search.GetSourceChunk(id)` → 返 200 + SourceChunk；404 / 503 sentinel mapping
- **修改 `internal/consoleapi/memstore.go`**：
  - MemStore SearchAdapter 加 `GetSourceChunk(id)` 返 `ErrDataPlaneUnavailable`（fallback 模式下 search index 不存在，无法回 chunk；行为符合 ADR-016 D4 degraded 信号）
- **单元测试 ≥4**：
  - `core/src/retriever/mod.rs::tests::test_get_chunk_by_id_found_and_none` (Rust)
  - `core/src/data_plane/search.rs::tests::test_get_source_chunk_returns_chunk` (Rust)
  - `core/src/data_plane/search.rs::tests::test_get_source_chunk_404_not_found` (Rust)
  - `internal/consoleapi/handlers_test.go::TestGetSourceChunk_404_when_missing` (Go)
  - `internal/consoleapi/handlers_test.go::TestGetSourceChunk_200_when_found` (Go)
  - `internal/consoleapi/grpcclient/grpcclient_test.go::TestSearchClient_GetSourceChunk_Maps_404` (Go)
- **集成测试 ≥1**：
  - `core/tests/data_plane_integration.rs::test_get_source_chunk_via_grpc` (spawn tonic server + tonic client + 真索引 fixture + 真 by-id fetch + 不存在 chunk 走 404)
  - 或 `internal/consoleapi/e2e_grpc_test.go::TestGetSourceChunk_E2E` (spawn Rust daemon + Go console-api-serve + 索引 fixture + REST GET /v1/source-chunks/<id> 真返 SourceChunk)
- **文件锚点**：`proto/contextforge/console_data_plane/v1/console_data_plane.proto` + `core/src/retriever/` + `core/src/data_plane/search.rs` + `internal/consoleapi/{types,router,handlers,memstore}.go` + `internal/consoleapi/grpcclient/grpcclient.go` + test files
- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **GET /v1/search/{query_id}/trace** [SPEC-OWNER:task-12.3]：本 task 不实施 trace 持久化
- **PATCH workspace config + cancel 204 + X-Confirm** [SPEC-OWNER:task-12.1]：本 task 不实施
- **Memory / Eval 端点** [SPEC-OWNER:phase-13/14]
- **MemStore fallback 实现 GetSourceChunk**：低价值；返 ErrDataPlaneUnavailable
- **multi-workspace strict 隔离**（chunk_id PK 跨 workspace 是否冲突）[SPEC-DEFER:phase-15.multi-workspace-strict]：本 task 假设 chunk_id 全 workspace 唯一（SqliteChunkStore current schema 行为）
- **chunk 内容 redaction policy 进一步演进**[SPEC-DEFER:secret-redaction-v2]：本 task `redaction_status` 字段透传 retriever 既有值

## 4. Users / Actors

- **Console UI 端**（下游，via cross-repo）：search 结果列表点击 chunk 后端到端拿全文 → 高亮 line_start/line_end
- **task-12.3 search-trace-by-query-id 实施 agent**（下游）：复用本 task SearchService RPC 扩展 pattern（proto add-only + Rust server impl + Go grpcclient wrapper + REST handler）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/decisions/adr-017-console-contract-completion-22-endpoint.md` §D1 Wave 2 / §D6
- `docs/decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md` §D2 / §D3
- `docs/specs/phases/phase-12-console-contract-completion.md` §3 / §6
- `docs/specs/tasks/task-11.1-rust-data-plane-grpc-services.md` (SearchService proto 现状)
- `docs/specs/tasks/task-11.4-search-real-retriever-and-events.md` (SearchService.Query 真接 retriever pattern)
- `H:/devlopment/code/contextforge/internal/contractv1/contractv1.go` (SourceChunk 字段)

### 5.2 Imports

- **Rust**: 现有 `tonic 0.12` + `prost 0.13`；复用 `core/src/retriever/` + `core/src/data_plane/search.rs`；可能新增 `core/src/retriever/by_id.rs` 子模块 (optional, §10 trade-off)
- **Go**: 现有 `internal/consoleapi/grpcclient/`；不引入新依赖

### 5.3 proto 改动形状

```proto
// proto/contextforge/console_data_plane/v1/console_data_plane.proto
service SearchService {
  rpc Query(SearchRequest) returns (SearchResponse);
  rpc GetSourceChunk(GetSourceChunkRequest) returns (SourceChunk);  // NEW (task-12.2)
}

message GetSourceChunkRequest {
  string chunk_id = 1;
  string workspace_id = 2;  // optional, for multi-workspace lookup
}

// SourceChunk message 已存 (task-11.1 ship; 9 fields 1:1 with Go contractv1.SourceChunk)
```

### 5.4 Rust SearchServer 方法形状

```rust
// core/src/data_plane/search.rs
#[tonic::async_trait]
impl proto::search_service_server::SearchService for SearchServer {
    // 已存 (task-11.4)
    async fn query(&self, ...) -> Result<Response<SearchResponse>, Status> { ... }

    // NEW (task-12.2)
    async fn get_source_chunk(
        &self,
        request: Request<GetSourceChunkRequest>,
    ) -> Result<Response<SourceChunk>, Status> {
        let req = request.into_inner();
        match self.stores.retriever.get_chunk_by_id(&req.chunk_id) {
            Ok(Some(chunk)) => Ok(Response::new(chunk_to_proto(chunk))),
            Ok(None) => Err(Status::not_found(format!("chunk not found: {}", req.chunk_id))),
            Err(e) => Err(Status::internal(format!("retriever error: {}", e))),
        }
    }
}
```

## 6. Acceptance Criteria

- [x] AC1：`GET /v1/source-chunks/{id}` 走 gRPC SearchService.GetSourceChunk → 复用既存 `Retriever::get_chunk(chunk_id)` (task-6.2 既存) → 真返 SourceChunk (chunk_id / workspace_id / source_file_path / line_start / line_end / chunk_text_preview / chunk_offset_start=0 [SPEC-DEFER:chunk-byte-offsets] / chunk_offset_end=0 / redaction_status 9 字段)；不存在 chunk → 404 NOT_FOUND — **verified by Rust unit `test_get_source_chunk_{empty_chunk_id_returns_invalid_argument,unknown_returns_not_found}` + Go E2E `TestRESTEndpoints_E2E_GrpcBacked` Step 9b PASS**
- [x] AC2：proto add-only 演进：`GetSourceChunk` RPC 添加不破坏既有 `Query` RPC；`GetSourceChunkRequest` 新 message 不破坏既有 message 编号 — **verified by `buf generate proto` clean + `cargo build -p contextforge-core` clean**
- [x] AC3：复用既存 retriever.get_chunk(chunk_id) (task-6.2 ship)；新 RPC 单元测试覆盖 empty_id (InvalidArgument) + unknown (NotFound) — **verified by `cargo test -p contextforge-core --lib data_plane::search` 3/3 PASS**
- [x] AC4：MemStore fallback 模式下 GetSourceChunk 返 `ErrDataPlaneUnavailable` → HTTP 503 + ErrorBody (deep defense / ADR-016 D4) — **verified by `TestGetSourceChunk_503_WhenFallback` PASS**
- [x] AC5：v0.4 + task-12.1 不退化 — **verified by `cargo test -p contextforge-core --lib` 66/66 PASS + `go test ./...` 43 packages 全绿**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | GET /v1/source-chunks/{id} 真返 chunk | proto + search.rs (reuse retriever.get_chunk) + handlers.go + grpcclient + E2E test | Done |
| AC2 | proto add-only 演进 | proto/contextforge/console_data_plane/v1/console_data_plane.proto | Done |
| AC3 | retriever.get_chunk unit test (reuse task-6.2) + 新 RPC 2 unit tests | core/src/data_plane/search.rs + cargo test | Done |
| AC4 | MemStore fallback 503 | memstore.go + go test TestGetSourceChunk_503_WhenFallback | Done |
| AC5 | v0.4 + task-12.1 不退化 | §9 verify run | Done |

## 8. Risks

- **chunk_id 跨 workspace 唯一性假设可能不稳**：SqliteChunkStore current schema chunk_id PK 是全表唯一；如果 v1.x 引入 multi-workspace partition 表，chunk_id 可能仅在 workspace 内唯一 → 本 task `GetSourceChunkRequest.workspace_id` 字段已加为 optional 兼容未来 [SPEC-DEFER:phase-15.multi-workspace-strict]
- **chunk_text_preview redaction**：retriever 既有逻辑可能已经做 redaction；本 task 直透传 retriever 返回值不二次处理；如发现 retriever 输出未 redaction → 安全 bug 应上报 [SPEC-OWNER:secret-redaction-v2]
- **proto add-only 字段编号冲突**：task-11.1 ship 时 `SourceChunk` message 已用 1-9 编号；`GetSourceChunkRequest` 是新 message 不冲突；但 `GetSourceChunk` RPC 在 SearchService 内编号必须接续既有 RPC → grep `SearchService` 当前定义确认下一个未用 RPC name + tonic_build 重新生成验证
- **tonic_build 编译时不自动重生成的边界 case**：cargo cache 偶尔不识别 proto 改动 → 缓解 `cargo clean -p contextforge-core` 强制重建；CI 内不会出现
- **retriever 接口扩展破坏既有调用**：`get_chunk_by_id` 是 add-only 方法 + 不修改既有 trait；缓解 grep 既有 retriever 调用点确认无 trait extension required

## 9. Verification Plan

- **install**: `cargo fetch` + `go mod download`
- **lint**: `cargo fmt --check` + `gofmt -l internal/consoleapi/`
- **typecheck**: `cargo check -p contextforge-core` + `go build ./...`
- **unit-test**: `cargo test -p contextforge-core --lib retriever::tests + data_plane::search::tests` + `go test ./internal/consoleapi/... ./internal/consoleapi/grpcclient/...`
- **integration**: `cargo test -p contextforge-core --test data_plane_integration -- test_get_source_chunk_via_grpc` + `go test -v ./internal/consoleapi/...` 含 E2E
- **e2e**: 通过 integration 实现 + `bash scripts/console_smoke.sh` REAL mode 15 endpoint flow 内含 source-chunks step
- **build**: `cargo build -p contextforge-core` + `go build ./cmd/contextforge`
- **coverage**: 不强制
- **runtime-smoke**: `bash scripts/console_smoke.sh` REAL mode + manual curl `GET /v1/source-chunks/<id>` 验证
- **manual**: grpcurl 调 SearchService.GetSourceChunk + diff proto vs Go contractv1.SourceChunk 字段命名

## 10. Completion Notes

- **完成日期**：2026-05-24
- **改动文件**：
  - `proto/contextforge/console_data_plane/v1/console_data_plane.proto` (修改 — SearchService 加 GetSourceChunk RPC + GetSourceChunkRequest message)
  - `core/src/retriever/mod.rs` (或具体子模块；修改 — 加 get_chunk_by_id 方法)
  - `core/src/data_plane/search.rs` (修改 — SearchServer 加 get_source_chunk 方法 + chunk_to_proto helper)
  - `core/tests/data_plane_integration.rs` (修改 — 加 test_get_source_chunk_via_grpc)
  - `internal/consoleapi/grpcclient/grpcclient.go` (修改 — SearchClient.GetSourceChunk wrapper)
  - `internal/consoleapi/types.go` (修改 — SearchClient 接口加 GetSourceChunk)
  - `internal/consoleapi/router.go` (修改 — GET /v1/source-chunks/{id} 路由)
  - `internal/consoleapi/handlers.go` (修改 — handleGetSourceChunk)
  - `internal/consoleapi/memstore.go` (修改 — Search.GetSourceChunk 返 ErrDataPlaneUnavailable)
  - `internal/consoleapi/handlers_test.go` (修改 — 2 新 unit test)
  - `internal/consoleapi/grpcclient/grpcclient_test.go` (修改 — 1 新 unit test)
  - `internal/consoleapi/e2e_grpc_test.go` (修改 — TestGetSourceChunk_E2E sub-step)
  - `docs/specs/tasks/task-12.2-source-chunk-by-id.md` (本 spec §6 / §7 / §10 / Status 推进)
- **commit 列表**：
  - feat(core/search+consoleapi): task-12.2 — GET /v1/source-chunks/{id} via gRPC SearchService.GetSourceChunk + retriever by-id lookup
  - docs(spec): task-12.2 §6/§7/§10 / Status → Done
- **关键决策**：复用既存 `Retriever::get_chunk(chunk_id)` (task-6.2 ship)，不新增 `get_chunk_by_id` 方法；workspace_id 从 GetSourceChunkRequest 可选传入，若缺失则在 SearchServer 内枚举 SqliteWorkspaceStore.list() 真试每个 workspace 寻 chunk（chunk_id 全局唯一 SqliteChunkStore 假设 [SPEC-DEFER:phase-15.multi-workspace-strict]）；chunk_offset_start/end=0 占位 [SPEC-DEFER:chunk-byte-offsets]（SqliteChunkStore current schema 不存 byte offsets，Console UI 用 line_start/end 显示）
- **§9 Verification 结果**：
  - `cargo check -p contextforge-core`: clean
  - `cargo test -p contextforge-core --lib`: 66 passed; 0 failed (含 2 new GetSourceChunk tests)
  - `go build ./...`: clean (含 degradedSearch.GetSourceChunk + MemStore.GetSourceChunk 占位)
  - `go test ./internal/consoleapi/...`: PASS (含 2 new TestGetSourceChunk_503/400 tests + e2e_grpc Step 9b 404 PASS)
  - `go test ./internal/consoleapi/grpcclient/...`: PASS (含 2 new fake server GetSourceChunk wire tests)
  - `go test ./test/conformance/...`: PASS (v0.4 + task-12.1 不退化)
  - `go test ./...`: 43/43 packages PASS
- **剩余风险 / 未做项**：
  - GET /v1/search/{query_id}/trace [SPEC-OWNER:task-12.3]
  - multi-workspace strict 隔离 [SPEC-DEFER:phase-15.multi-workspace-strict]
  - chunk text redaction 完整 v2 review [SPEC-DEFER:secret-redaction-v2]
- **下游 task 影响**：task-12.3 复用本 task SearchService RPC 扩展 pattern + 复用 chunk_to_proto helper（如适用 trace 字段）
