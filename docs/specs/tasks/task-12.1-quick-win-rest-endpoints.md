# Task `12.1`: `quick-win-rest-endpoints — PATCH workspace/config + GET index-jobs?status=active + cancel 204 + X-Confirm 412 兜底`

**Status**: Done

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 12 (console-contract-completion)
**Dependencies**: task-11.1 (WorkspaceService.Update + JobService.List gRPC 已 ship) + task-11.2 (grpcclient.Workspace/Job 已 ship) + [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D1 Wave 1 / D2 / D3

## 1. Background

[ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) Phase 11 task-11.1 已 ship 完整 Rust gRPC 4 service × 14 RPC（含 `WorkspaceService.Update` + `JobService.List`，见 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`）；task-11.2 已 ship 完整 Go `internal/consoleapi/grpcclient/` 4 client wrapper。但 Go REST 层只暴露了 9 endpoint，PATCH workspace/config + GET index-jobs?status=active 两条因为 Phase 11 §3 OUT scope [SPEC-DEFER:console-endpoint-expansion] 显式 defer 留 v0.4.x。

[ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D1 Wave 1 把这两条 + cancel 204 改造 + X-Confirm 服务端兜底打包成 quick win task — 工作量主要在 Go REST 层（grpcclient.Workspace.Update / Job.ListActive wrapper + router 加路由 + handlers + confirmMiddleware），Rust 端**完全不动**。

## 2. Goal

Go `internal/consoleapi/` 加 2 个新 REST endpoint（PATCH /v1/workspaces/{id}/config + GET /v1/index-jobs?status=active）+ 修改 cancel 返 204 + 引入 `confirmMiddleware` 服务端 X-Confirm/?confirm=true 双因子兜底（缺失返 412 Precondition Failed）；grpcclient WorkspaceClient 接口加 `Update` 方法 + JobClient 接口加 `ListActive` 方法（或 `List(filter ListFilter)`）；MemStore env-gated fallback 同步实现 4 个新行为；`go test ./internal/consoleapi/...` + `./test/conformance/...` 全绿（v0.4 9 endpoint 不退化）；4 个新 e2e sub-test PASS。

## 3. Scope

### In Scope

- **修改 `internal/consoleapi/types.go`**：
  - `WorkspaceClient` 接口加 `Update(workspaceID string, allowlist, denylist []string) (contractv1.Workspace, error)`
  - `JobClient` 接口加 `ListActive() ([]contractv1.IndexJob, error)`（或带 filter struct `List(filter JobListFilter) ([]IndexJob, error)`，最终 task implementation 自决，§10 trade-off 评估）
  - 新增 sentinel `ErrPreconditionRequired = errors.New("X-Confirm header or ?confirm=true query required")` + map 到 412
- **修改 `internal/consoleapi/router.go`**：
  - 路由注册加 `PATCH /v1/workspaces/{id}/config` → `confirmMiddleware(handlePatchWorkspaceConfig(deps))`
  - 路由注册加 `GET /v1/index-jobs` 改 handler 支持 `?status=active` query param（不破坏现有 `GET /v1/index-jobs/{id}`）
  - 路由注册 `POST /v1/index-jobs/{id}/cancel` 改 handler 返 204
  - 新增 `confirmMiddleware(next http.HandlerFunc) http.HandlerFunc`：
    ```go
    // ADR-017 D2 — 破坏性 endpoint 服务端兜底：必须 X-Confirm: yes header 或 ?confirm=true query 任一
    func confirmMiddleware(next http.HandlerFunc) http.HandlerFunc {
        return func(w http.ResponseWriter, r *http.Request) {
            if r.Header.Get("X-Confirm") == "yes" || r.URL.Query().Get("confirm") == "true" {
                next.ServeHTTP(w, r)
                return
            }
            writeError(w, http.StatusPreconditionFailed, "PRECONDITION_FAILED",
                "X-Confirm: yes header or ?confirm=true query required for destructive op (ADR-017 D2)")
        }
    }
    ```
- **修改 `internal/consoleapi/handlers.go`**：
  - 新增 `handlePatchWorkspaceConfig(deps Deps) http.HandlerFunc`：从 PathValue id + body parse `{allowlist:[], denylist:[]}` → `deps.Workspace.Update(id, allowlist, denylist)` → 返更新后 `Workspace`；error mapping (404/412/503/500)
  - 新增 `handleListJobs(deps Deps) http.HandlerFunc`：parse `?status=` query；status="active" → `deps.Job.ListActive()`；空或其它 status → 当前 Console v1.0 不要求全量 list endpoint（22-endpoint 中只有 active filter），返 400 BAD_REQUEST 或 200 + empty array (§10 trade-off 评估)
  - 改 `handleCancelJob`：成功路径 `w.WriteHeader(http.StatusNoContent)` 替换 `StatusOK`
- **修改 `internal/consoleapi/grpcclient/grpcclient.go`**：
  - `WorkspaceClient.Update` 调 `proto.WorkspaceClient.Update` 后 protoTo… helper 转 Go contractv1.Workspace 返
  - `JobClient.ListActive` 调 `proto.JobClient.List` with filter `{statuses: ["queued", "running"]}` 后 map response
  - 新增 `ListJobsRequest` filter 字段 [SPEC-OWNER:task-12.1] — 若 proto 当前 `ListJobsRequest` 没 status filter 字段，须按 ADR-013 add-only 演进规则 amend proto `proto/contextforge/console_data_plane/v1/console_data_plane.proto` ListJobsRequest 加 `repeated string status_filter = N;`（**add-only field，编号下一个未用**；保持向后兼容）
- **修改 `internal/consoleapi/memstore.go`**：
  - `MemStore.Workspace.Update(id, allowlist, denylist)` 实现 in-memory 更新 + UpdatedAt 时间戳推进 + 返更新后 Workspace
  - `MemStore.Job.ListActive()` 实现 filter (queued/running) 返回
  - 这两个新方法在 fallback 模式下也有意义；其它 task-12.2/12.3 新 method（GetSourceChunk / GetSearchTrace）不在 MemStore 实现 [SPEC-OWNER:task-12.2 + task-12.3]
- **修改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`** (如需)：
  - `ListJobsRequest` 加 `repeated string status_filter = N;` (add-only) — 仅 task-12.1 实施时检查 proto，若 task-11.1 ship 时已含 filter 字段则不需要 amend
- **单元测试 ≥6**（`internal/consoleapi/router_test.go` + `handlers_test.go` + `grpcclient_test.go`）：
  - `TestPatchWorkspaceConfig_RequiresConfirm` (412 when missing X-Confirm)
  - `TestPatchWorkspaceConfig_AcceptsHeader` (200 + updated Workspace when X-Confirm: yes)
  - `TestPatchWorkspaceConfig_AcceptsQuery` (200 + updated when ?confirm=true)
  - `TestListJobs_ActiveFilter` (only queued/running returned)
  - `TestCancelJob_Returns_204` (no body, status 204)
  - `TestCancelJob_404_409_unchanged` (sentinel mapping 不退化)
  - `TestGrpcClient_WorkspaceUpdate_Maps_404` (NotFound → ErrNotFound)
  - `TestGrpcClient_JobListActive_Maps_503` (Unavailable → ErrDataPlaneUnavailable)
- **集成测试 ≥1**（`internal/consoleapi/e2e_grpc_test.go` 加 sub-test）：
  - `TestRESTEndpoints_E2E_GrpcBacked` 既有 test fixture 加 sub-step：spawn Rust daemon → POST workspace → PATCH /v1/workspaces/{id}/config with X-Confirm: yes → GET 返更新；POST index-job → GET /v1/index-jobs?status=active 含该 job；POST cancel → 204 验证；PATCH workspace/config without X-Confirm → 412
- **文件锚点**：`internal/consoleapi/{types,router,handlers,memstore}.go` + `internal/consoleapi/grpcclient/grpcclient.go` + `proto/contextforge/console_data_plane/v1/console_data_plane.proto` (条件 amend)
- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **GET /v1/source-chunks/{id}** [SPEC-OWNER:task-12.2]：本 task 不实施 Rust SearchService.GetSourceChunk
- **GET /v1/search/{query_id}/trace** [SPEC-OWNER:task-12.3]：本 task 不实施 Rust trace 持久化
- **Memory 5 endpoint** [SPEC-OWNER:phase-13]：本 task 范围内
- **Eval 2 endpoint** [SPEC-OWNER:phase-14]：本 task 范围内
- **Rust SearchService / EventsService / MemoryService / EvalService 改动** [SPEC-OWNER:task-12.2/12.3/phase-13/phase-14]：本 task 仅 Go REST 层 + 可选 proto add-only amend ListJobsRequest.status_filter
- **新增 SQLite migration** [SPEC-DEFER:task-future.search-trace-sqlite-persistence]：复用 task-10.2/10.3 既有 migrations
- **MemStore 实现 GetSourceChunk / GetSearchTrace**：低价值 (fallback 模式下 search 已废)，留 ErrDataPlaneUnavailable

## 4. Users / Actors

- **task-12.2 source-chunk-by-id 实施 agent**（下游）：依赖本 task confirmMiddleware + handler 模板
- **task-12.3 search-trace-by-query-id 实施 agent**（下游）：依赖本 task confirmMiddleware（不在 trace endpoint 用，但 confirmMiddleware 必须先 ship）
- **task-13.2 go-memory-rest-handlers 实施 agent**（下游 phase）：复用本 task confirmMiddleware 给 memory deprecate / soft-delete 用
- **Console BFF 端** (cross-repo)：本 task ship 后 Console 端可以测 PATCH workspace config + list active jobs + cancel 204 + X-Confirm 412 deep defense

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/decisions/adr-017-console-contract-completion-22-endpoint.md` §D1 / §D2 / §D3 / §D6
- `docs/decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md` §D2 / §D3
- `docs/specs/phases/phase-12-console-contract-completion.md` §3 / §6
- `docs/specs/tasks/task-11.1-rust-data-plane-grpc-services.md` (WorkspaceService.Update + JobService.List proto 字段集合)
- `docs/specs/tasks/task-11.2-go-rest-to-grpc-proxy.md` (grpcclient 4 wrapper 现状)
- `H:/devlopment/code/contextforge/internal/contractv1/contractv1.go` （字段命名 single source of truth；Workspace + IndexJob struct 不动）

### 5.2 Imports

- **Go**: 现有 stdlib `net/http` + `encoding/json` + `errors` + `strings`；现有 `google.golang.org/grpc` + `github.com/tajiaoyezi/contextforge/internal/contractv1`；现有 `internal/consoleapi/grpcclient/`
- **不引入新依赖**：R7 不触发；`go.mod` 不动

### 5.3 Routes 注册形状

```go
// internal/consoleapi/router.go
mux.HandleFunc("GET /v1/health", handleHealth(deps))
mux.HandleFunc("POST /v1/workspaces", handleCreateWorkspace(deps))
mux.HandleFunc("GET /v1/workspaces", handleListWorkspaces(deps))
mux.HandleFunc("GET /v1/workspaces/{id}", handleGetWorkspace(deps))
mux.HandleFunc("PATCH /v1/workspaces/{id}/config", confirmMiddleware(handlePatchWorkspaceConfig(deps)))  // NEW (task-12.1)
mux.HandleFunc("POST /v1/index-jobs", handleEnqueueJob(deps))
mux.HandleFunc("GET /v1/index-jobs", handleListJobs(deps))  // NEW (task-12.1; ?status=active filter)
mux.HandleFunc("GET /v1/index-jobs/{id}", handleGetJob(deps))
mux.HandleFunc("POST /v1/index-jobs/{id}/cancel", handleCancelJob(deps))  // MODIFIED: returns 204 now
mux.HandleFunc("POST /v1/search", handleSearch(deps))
mux.HandleFunc("GET /v1/observability/events", handleEvents(deps))
```

### 5.4 confirmMiddleware 形状

```go
// internal/consoleapi/router.go
func confirmMiddleware(next http.HandlerFunc) http.HandlerFunc {
    return func(w http.ResponseWriter, r *http.Request) {
        if r.Header.Get("X-Confirm") == "yes" || r.URL.Query().Get("confirm") == "true" {
            next.ServeHTTP(w, r)
            return
        }
        writeError(w, http.StatusPreconditionFailed, "PRECONDITION_FAILED",
            "X-Confirm: yes header or ?confirm=true query required for destructive op")
    }
}
```

## 6. Acceptance Criteria

- [x] AC1：`PATCH /v1/workspaces/{id}/config` body `{allowlist:[...], denylist:[...]}` + `X-Confirm: yes` header（**或** `?confirm=true` query）→ 走 gRPC WorkspaceService.UpdateConfig → 返 200 + 更新后 `Workspace`；缺失两者 → 412 Precondition Failed + ErrorBody `{code:"PRECONDITION_FAILED",...}` — **verified by unit-test `TestPatchWorkspaceConfig_{RequiresConfirm,AcceptsHeader,AcceptsQuery,404}` PASS + integration `TestRESTEndpoints_E2E_GrpcBacked` Step 8a PASS (412→200(header)→200(query) flow)**
- [x] AC2：`GET /v1/index-jobs?status=active` 走 gRPC JobService.List + status filter (queued OR running) → 返 200 + JSON array of IndexJob；空集 → 200 + `[]`；missing status filter → 400 [SPEC-DEFER:console-list-all-jobs] — **verified by unit-test `TestListJobs_{ActiveFilter,MissingStatusFilter}` PASS + integration `TestRESTEndpoints_E2E_GrpcBacked` Step 8b PASS**
- [x] AC3：`POST /v1/index-jobs/{id}/cancel` 成功 → 204 No Content (no body)；409 / 404 sentinel mapping 不变 — **verified by unit-test `TestCancelJob_{Returns_204,404_unchanged}` + `TestHandleCancelJob_409` + integration `TestRESTEndpoints_E2E_GrpcBacked` Step 9 PASS**
- [x] AC4：`grpcclient.WorkspaceClient.Update` + `grpcclient.JobClient.ListActive` 实现 + 错误 mapping (NotFound→404 / Unavailable→503) — **verified by `TestGrpcClient_WorkspaceUpdate_{WiresFields,Maps_NotFound}` + `TestGrpcClient_JobListActive_{FiltersAndMaps,Maps_Unavailable}` PASS**
- [x] AC5：v0.4 既有 9 endpoint test 不退化（`go test ./internal/consoleapi/...` 全绿 + `go test ./test/conformance/...` 仍 PASS；MemStore env-gated fallback 模式下 Workspace.Update + Job.ListActive 也工作；其它新 method GetSourceChunk / GetSearchTrace 返 ErrDataPlaneUnavailable [SPEC-OWNER:task-12.2/12.3]）— **verified by `go test ./...` 全绿 (43 packages) + `cargo test -p contextforge-core --lib` 64/64 PASS**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | PATCH workspace/config + X-Confirm 412 兜底 | router.go (confirmMiddleware) + handlers.go (handlePatchWorkspaceConfig) + tests | Done |
| AC2 | GET index-jobs?status=active | router.go + handlers.go (handleListJobs) + grpcclient.Job.ListActive + tests | Done |
| AC3 | cancel 改 204 | handlers.go (handleCancelJob mod) + tests | Done |
| AC4 | grpcclient WorkspaceUpdate + JobListActive wrapper | grpcclient.go + tests | Done |
| AC5 | v0.4 9 endpoint + conformance 不退化 | §9 verify run all-green | Done |

## 8. Risks

- **proto `ListJobsRequest.status_filter` 字段如果 task-11.1 没 ship**：本 task 须按 ADR-013 add-only 演进规则 amend proto + tonic_build 重新生成；缓解 task implementation 第一步 grep `proto/contextforge/console_data_plane/v1/console_data_plane.proto` ListJobsRequest 字段集合
- **Console HTTPAdapter v1.0 cancel 处理 200/204 双 check**：Console repo `console-api/internal/coreadapter/http_adapter.go::CancelIndexJob` 必须 200/204 都接受；若 Console 端 strict 只接 204 → 本 task 改 200→204 完全无 breaking risk；若 strict 只接 200 → 切 204 破坏；缓解 task implementation 第一步 cross-repo grep Console http_adapter.go::CancelIndexJob 确认双 check 存在
- **X-Confirm OR 语义 vs AND 语义混淆**：Console BFF 注入两者，但若服务端误改成 AND 校验（两者都必须存在）→ Console 端 OK；运维 curl 端缺一即 412 → deep defense 反而过度。本 task **强制 OR 语义** + 单元测试 3 cases 覆盖
- **PATCH workspace/config 不允许 None / 部分字段更新**：Console spec 写 body `{allowlist, denylist}` 两字段都必须；本 task 实现时如果 body 缺一字段是否报 400 or treat as empty (覆盖式)？本 task **当前 body 字段必填**；缺一返 400 BAD_REQUEST；如 Console 期望部分更新则 task implementation §10 trade-off 评估 + 提 amendment
- **`ListJobs` 没有 status filter 时 fallback 行为**：Phase 12 设计是 only support `?status=active`；如不传 status 应返什么？task implementation 选 `?status=` 缺失 → 400 BAD_REQUEST 或 200 + 全部 jobs；Console spec 写「list filter」暗示仅 active 是 v1 范围 → 选 400 + message "?status=active required (v1)" [SPEC-DEFER:console-list-all-jobs] 留 v1.x

## 9. Verification Plan

- **install**: `go mod download` + `cargo fetch`
- **lint**: `gofmt -l internal/consoleapi/` (0 diff) + `cargo fmt --check`（proto 改动如需）
- **typecheck**: `go build ./internal/consoleapi/... ./internal/consoleapi/grpcclient/...` + `cargo check -p contextforge-core`
- **unit-test**: `go test -v ./internal/consoleapi/... ./internal/consoleapi/grpcclient/...`（≥6 新单测全过 + v0.4 既有不退化）
- **integration**: `go test -v -run TestRESTEndpoints_E2E_GrpcBacked ./internal/consoleapi/...`（含 4 sub-step：patch_config / list_active / cancel_204 / confirm_412）
- **e2e**: 通过 integration 实现 + `bash scripts/console_smoke.sh` REAL mode 15 endpoint flow
- **build**: `go build ./cmd/contextforge` + `cargo build -p contextforge-core`
- **coverage**: 不强制（task-11.2 同款）
- **runtime-smoke**: `bash scripts/console_smoke.sh` REAL mode end-to-end 15 endpoint
- **manual**: curl PATCH /v1/workspaces/<id>/config 带 / 不带 X-Confirm 验证 412 vs 200；curl GET /v1/index-jobs?status=active 验证 filter；curl POST cancel 验证 204

## 10. Completion Notes

- **完成日期**：2026-05-24
- **改动文件**：
  - `proto/contextforge/console_data_plane/v1/console_data_plane.proto` (修改 — `UpdateWorkspaceConfigRequest` + `WorkspaceService.UpdateConfig` + `ListJobsRequest` + `ListJobsResponse` + `JobService.List` add-only)
  - `proto/contextforge/console_data_plane/v1/console_data_plane.pb.go` (regen via `buf generate proto`)
  - `proto/contextforge/console_data_plane/v1/console_data_plane_grpc.pb.go` (regen via `buf generate proto`)
  - `core/src/data_plane/workspace.rs` (修改 — `UpdateConfig` impl + 2 unit tests)
  - `core/src/data_plane/job.rs` (修改 — `List` impl with status_filter + workspace_id post-filter + 2 unit tests)
  - `internal/consoleapi/types.go` (修改 — `WorkspaceClient.Update` + `JobClient.ListActive` 接口 + `ErrPreconditionRequired` sentinel)
  - `internal/consoleapi/router.go` (修改 — 2 新路由 + `confirmMiddleware` + ErrPreconditionRequired 映射)
  - `internal/consoleapi/handlers.go` (修改 — `handlePatchWorkspaceConfig` + `handleListJobs` + `handleCancelJob` 204)
  - `internal/consoleapi/memstore.go` (修改 — `UpdateWorkspaceConfig` + `ListActiveJobs` in-memory 实现 + adapter delegates)
  - `internal/consoleapi/grpcclient/grpcclient.go` (修改 — `workspaceClient.Update` + `jobClient.ListActive` wrapper)
  - `internal/consoleapi/router_test.go` (修改 — 7 新 unit test + 1 cancel_job 行 200→204)
  - `internal/consoleapi/grpcclient/grpcclient_test.go` (修改 — 4 新 unit test + 2 新 fakeServer stub)
  - `internal/consoleapi/e2e_test.go` (修改 — POST cancel 200→204)
  - `internal/consoleapi/e2e_grpc_test.go` (修改 — Step 8a/8b 加 patch_config + confirm_412 + list_active 子流程 + `doJSONHeaders` helper + Step 9 cancel 200→204)
  - `internal/cli/console_api_serve_degraded.go` (修改 — `degradedWorkspace.Update` + `degradedJob.ListActive` 占位)
  - `docs/specs/tasks/task-12.1-quick-win-rest-endpoints.md` (本 spec §6 / §7 / §10 / Status → Done)
- **commit 列表**：
  - feat(consoleapi): task-12.1 — PATCH workspace/config + GET index-jobs?status=active + cancel 204 + X-Confirm 412 middleware
- **§9 Verification 结果**：
  - `cargo check -p contextforge-core`: clean
  - `cargo test -p contextforge-core --lib`: 64 passed; 0 failed (含 4 new: workspace UpdateConfig + 2 job List)
  - `go build ./...`: clean (含 degradedWorkspace/Job 补 Update/ListActive)
  - `go test ./internal/consoleapi/...`: PASS (含 7 new task-12.1 unit tests + e2e_grpc 4 sub-step PASS with real Rust daemon)
  - `go test ./internal/consoleapi/grpcclient/...`: PASS (含 4 new UpdateConfig + ListActive wire tests)
  - `go test ./test/conformance/...`: PASS (v0.4 9 endpoint 不退化)
  - `go test ./...`: 43/43 packages PASS
- **剩余风险 / 未做项**：
  - GET /v1/source-chunks/{id} [SPEC-OWNER:task-12.2]（不在本 task scope）
  - GET /v1/search/{query_id}/trace [SPEC-OWNER:task-12.3]（不在本 task scope）
  - 真接 daemon e2e cancel→active 异步状态传播测试 racy（cancel_requested → mark_terminal 由 IndexSession 异步驱动，task-11.3 scope）；本 task 仅验 REST 204 契约，不验异步状态收敛
  - MemStore 不实现 GetSourceChunk / GetSearchTrace（fallback 返 ErrDataPlaneUnavailable，trade-off 接受）
- **下游 task 影响**：task-12.2/12.3 复用 router.go confirmMiddleware pattern；phase-13 task-13.2 memory deprecate/soft-delete 复用 confirmMiddleware
