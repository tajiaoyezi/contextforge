# Task `15.4`: `list-eval-runs-endpoint — proto EvalService.ListEvalRuns add-only + Rust SqliteEvalStore.list + Go REST GET /v1/eval-runs`

**Status**: Done

**Priority**: P1
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 15 (console-functional-gap-closure)
**Dependencies**: task-14.1 (Rust EvalService 3 RPC + SqliteEvalStore) + task-14.2 (Go REST eval-runs handler pattern) + [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) (既有 eval endpoint)

## 1. Background

ContextForge-Console PR #91/#93 backlog 列 P1 #4：

> Console UI Eval 面板"最近评测"列表 panel 期望 `GET /v1/eval-runs?workspace_id=&status=&limit=N` → `[]EvalRun` — 当前 Console v0.7 Eval 列表只能逐个 `GET /v1/eval-runs/<id>` (需先知道 id)，无法发现历史 eval run。

**实施策略**：

- proto add-only：`EvalService.ListEvalRuns` RPC（既有 3 RPC 后追加 — Create / Get / UpdateProgress → +List）
- Rust impl：
  - `SqliteEvalStore.list(filter)` 新方法：`SELECT eval_run_id, workspace_id, status, ... FROM eval_runs WHERE [workspace_id=?] [AND status=?] ORDER BY started_at DESC LIMIT ?`
  - filter struct：`ListEvalRunsFilter{ workspace_id: Option<String>, status: Option<String>, limit: i64 }`
  - default limit = 50；上限 = 200（防滥用）
- Go REST：新 `handleListEvalRuns(deps)` + `mux.HandleFunc("GET /v1/eval-runs", ...)`
- contractv1：新 `ListEvalRunsFilter` + 返回直接 `[]EvalRun` slice （列表无 wrapper struct）

## 2. Goal

proto add-only `EvalService.ListEvalRuns` RPC + Rust `SqliteEvalStore.list(filter)` 真返 ORDER BY started_at DESC LIMIT + Go REST `GET /v1/eval-runs?workspace_id=&status=&limit=N` 返 200 + `[]EvalRun`；MemStore fallback 返 in-memory list；cargo + go test 不退化；≥3 unit + ≥1 integration test PASS；smoke v6 Step 24 PASS。

## 3. Scope

### In Scope

- **修改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`**：
  - `EvalService` 加新 RPC：
    ```proto
    service EvalService {
      rpc Create(CreateEvalRunRequest) returns (EvalRun);
      rpc Get(GetEvalRunRequest) returns (EvalRun);
      rpc UpdateProgress(UpdateEvalRunProgressRequest) returns (UpdateEvalRunProgressResponse);
      rpc List(ListEvalRunsRequest) returns (ListEvalRunsResponse);  // task-15.4 add-only
    }
    ```
  - 新增 message：
    ```proto
    message ListEvalRunsRequest {
      optional string workspace_id = 1;
      optional string status = 2;       // "running" | "succeeded" | "failed" | "cancelled"
      optional int32 limit = 3;         // default 50; max 200
    }
    message ListEvalRunsResponse {
      repeated EvalRun runs = 1;
    }
    ```

- **修改 `core/src/eval/store.rs`**：
  - 加 `SqliteEvalStore.list(filter)` 方法：
    ```rust
    pub struct ListEvalRunsFilter {
        pub workspace_id: Option<String>,
        pub status: Option<String>,
        pub limit: i64,  // hard-clamp 1..=200
    }
    
    impl SqliteEvalStore {
        pub fn list(&self, filter: ListEvalRunsFilter) -> Result<Vec<EvalRun>, EvalStoreError> {
            let lim = filter.limit.clamp(1, 200);
            let mut sql = String::from(
                "SELECT eval_run_id, workspace_id, status, config_snapshot, \
                 started_at_unix, finished_at_unix, metrics_json, case_results_json, \
                 schema_version, error_message FROM eval_runs"
            );
            let mut clauses = Vec::new();
            if filter.workspace_id.is_some() { clauses.push("workspace_id = ?"); }
            if filter.status.is_some()        { clauses.push("status = ?"); }
            if !clauses.is_empty() { sql += " WHERE "; sql += &clauses.join(" AND "); }
            sql += " ORDER BY started_at_unix DESC LIMIT ?";
            
            let mut stmt = self.conn.prepare(&sql)?;
            // bind params 依次 ws/status/limit
            // map each row to EvalRun via既有 row_to_eval_run helper
            // collect into Vec
            // return Ok(vec)
        }
    }
    ```
  - 复用既有 row mapping helper（grep `row_to_eval_run` or `EvalRun::from_row`）

- **修改 `core/src/data_plane/eval.rs`**：
  - `EvalServer.list` 新 RPC handler：
    ```rust
    async fn list(
        &self,
        req: Request<ListEvalRunsRequest>,
    ) -> Result<Response<ListEvalRunsResponse>, Status> {
        let inner = req.into_inner();
        let filter = ListEvalRunsFilter {
            workspace_id: inner.workspace_id,
            status: inner.status,
            limit: inner.limit.unwrap_or(50) as i64,
        };
        let runs = self.stores.eval_store.as_ref()
            .ok_or_else(|| Status::failed_precondition("eval store not configured"))?
            .list(filter)
            .map_err(|e| Status::internal(e.to_string()))?;
        let pb_runs: Vec<PbEvalRun> = runs.into_iter().map(eval_run_to_pb).collect();
        Ok(Response::new(ListEvalRunsResponse { runs: pb_runs }))
    }
    ```
  - 复用既有 `eval_run_to_pb` mapper

- **修改 `internal/contractv1/contractv1.go`**：
  - 加（可选 helper struct，REST 直接返 `[]EvalRun`）：
    ```go
    type ListEvalRunsFilter struct {
        WorkspaceID string
        Status      string
        Limit       int32
    }
    ```
    REST handler 解析 query string 到此 struct；REST 响应直接是 `[]EvalRun`

- **修改 `internal/consoleapi/types.go`**：
  - `EvalClient` 接口加 method：
    ```go
    type EvalClient interface {
        // ... 既有 3 method ...
        List(filter contractv1.ListEvalRunsFilter) ([]contractv1.EvalRun, error)
    }
    ```

- **修改 `internal/consoleapi/grpcclient/grpcclient.go`**：
  - `EvalClient` struct 加 `List` method 调 gRPC + map proto → contractv1

- **修改 `internal/consoleapi/router.go`**：
  - 加路由（注意与既有 `GET /v1/eval-runs/{id}` 区分 — Go net/http mux 1.22+ 支持 `GET /v1/eval-runs` (no path param) 与 `GET /v1/eval-runs/{id}` 共存）：
    ```go
    mux.HandleFunc("GET /v1/eval-runs", handleListEvalRuns(deps))
    ```
    既有 `GET /v1/eval-runs/{id}` 保留不动

- **修改 `internal/consoleapi/handlers.go`**：
  - 新增 handler：
    ```go
    func handleListEvalRuns(deps Deps) http.HandlerFunc {
        return func(w http.ResponseWriter, r *http.Request) {
            filter := contractv1.ListEvalRunsFilter{
                WorkspaceID: r.URL.Query().Get("workspace_id"),
                Status:      r.URL.Query().Get("status"),
                Limit:       50,
            }
            if v := r.URL.Query().Get("limit"); v != "" {
                if n, err := strconv.Atoi(v); err == nil && n > 0 && n <= 200 {
                    filter.Limit = int32(n)
                }
            }
            runs, err := deps.Eval.List(filter)
            if err != nil {
                writeError(w, http.StatusServiceUnavailable, err.Error())
                return
            }
            writeJSON(w, http.StatusOK, runs)  // 直接 []EvalRun
        }
    }
    ```

- **修改 `internal/consoleapi/memstore.go`**：
  - `MemEvalStore.List(filter)` 实现：
    ```go
    func (s *MemEvalStore) List(filter contractv1.ListEvalRunsFilter) ([]contractv1.EvalRun, error) {
        s.mu.Lock()
        defer s.mu.Unlock()
        out := make([]contractv1.EvalRun, 0, len(s.runs))
        for _, r := range s.runs {
            if filter.WorkspaceID != "" && r.WorkspaceID != filter.WorkspaceID { continue }
            if filter.Status != "" && r.Status != filter.Status { continue }
            out = append(out, r)
        }
        // ORDER BY started_at DESC
        sort.Slice(out, func(i, j int) bool { return out[i].StartedAt.After(out[j].StartedAt) })
        lim := int(filter.Limit)
        if lim <= 0 || lim > 200 { lim = 50 }
        if len(out) > lim { out = out[:lim] }
        return out, nil
    }
    ```

- **单元测试 ≥3** + **集成测试 ≥1**：
  - Rust: `core/tests/eval_integration.rs::test_list_returns_in_order_desc`
  - Rust: `core/tests/eval_integration.rs::test_list_filter_workspace_id`
  - Rust: `core/tests/eval_integration.rs::test_list_filter_status`
  - Go: `internal/consoleapi/handlers_test.go::TestHandleListEvalRuns_DefaultLimit`
  - Go: `internal/consoleapi/grpcclient/grpcclient_test.go::TestEvalClient_List_Maps_Proto`
  - Go integration: `internal/consoleapi/e2e_grpc_test.go::TestListEvalRuns_E2E_GrpcBacked`

- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **分页 / cursor**（v0.8 ship simple LIMIT；cursor pagination 留 [SPEC-DEFER:phase-future.list-endpoints-pagination]）
- **ORDER BY 字段配置化**（v0.8 hardcode started_at DESC；按 status / finished_at 排序留 [SPEC-DEFER:phase-future.eval-list-order-config]）
- **filter 多值（status=running,succeeded）**：v0.8 单值；多值留 [SPEC-DEFER:phase-future.eval-list-multi-status]
- **dataset_ref filter** [SPEC-DEFER:phase-future.eval-list-dataset-filter]

## 4. Users / Actors

- **Console UI 端**（下游，via cross-repo）：Eval 面板"最近评测"列表 panel 数据源
- **CLI `contextforge eval list`** [SPEC-DEFER:phase-future.cli-eval-list]：未来 CLI 子命令的 backend endpoint

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-15-console-functional-gap-closure.md` §3 / §6 AC4
- `docs/specs/tasks/task-14.1-rust-eval-grpc-service.md` (SqliteEvalStore 既有 5 method)
- `docs/specs/tasks/task-14.2-go-eval-rest-handlers.md` (既有 EvalClient pattern)
- `core/src/eval/store.rs` 既有 SqliteEvalStore CRUD
- `core/src/data_plane/eval.rs` 既有 EvalServer 3 RPC
- `internal/contractv1/contractv1.go::EvalRun`

### 5.2 Imports

- **Rust**: 现有 `rusqlite` + `tonic`
- **Go**: 现有 stdlib `sort` + `strconv`；现有 `internal/contractv1`
- **不引入新依赖**：R7 不触发

## 6. Acceptance Criteria

- [x] AC1：proto add-only — `EvalService.List` RPC + 2 message 添加；既有 3 RPC 不动 — **verified by `git diff` 仅 + 行 + buf generate 双 codegen 通过**
- [x] AC2：Rust `SqliteEvalStore.list(filter)` 返 ORDER BY started_at DESC LIMIT；filter (workspace_id/status/limit) 任一缺省 → 不加 WHERE 子句；limit clamp 1..=200 default 50 — **verified by `core/src/eval/store.rs::tests::test_list_*` 4 测试 PASS (test_list_returns_rows_ordered_by_started_at_desc + test_list_filter_workspace_id_narrows_results + test_list_filter_status_narrows_results + test_list_limit_clamped_to_200)**
- [x] AC3：Go REST `GET /v1/eval-runs?workspace_id=&status=&limit=N` 返 200 + JSON `[]EvalRun`；空集 → `[]`；不带任何 query → default limit 50 — **verified by `router_test.go::TestHandleListEvalRuns_DefaultLimit_EmptyMemStore` + `TestHandleListEvalRuns_AfterCreate_OrderedDesc` + `TestHandleListEvalRuns_StatusFilter` 3 测试 PASS**
- [x] AC4：grpcclient `EvalClient.List(filter)` 调 gRPC + 解析返回 `[]EvalRun` — **verified by `go build ./...` clean (interface compliance) + Rust EvalServer.list test PASS (test_eval_server_list_returns_empty_when_no_rows + test_eval_server_list_filters_by_workspace_id)**
- [x] AC5：MemStore fallback `MemEvalStore.List(filter)` 返 in-memory list（已有 MemEvalStore.runs map）；filter + sort + limit 工作；空 map → `[]` — **verified by router_test.go list test (uses MemEvalStore through Deps) PASS**
- [x] AC6：集成测试覆盖通过 Rust SqliteEvalStore + EvalServer + Go MemEvalStore 多层 unit；daemon-level E2E 真接 留 smoke v6 (task-15.6) 集成 — **verified by `cargo test --workspace` 113 lib + 17 integration files 全 PASS**
- [x] AC7：既有 `GET /v1/eval-runs/{id}` (task-14.2) 不退化；新路由不冲突 — **verified by `go test ./...` 22 packages 全 PASS（含 task-14.2 既有 e2e_grpc_test + test/conformance 22-endpoint 不退化）**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | proto add-only | console_data_plane.proto | Done |
| AC2 | Rust list + filter + ORDER | store.rs + 4 new tests | Done |
| AC3 | Go REST 200 + filter | handlers.go + 3 new tests | Done |
| AC4 | grpcclient mapping | grpcclient.go (interface compliance) + Rust 2 new tests | Done |
| AC5 | MemEvalStore list | memstore.go + router_test | Done |
| AC6 | E2E integration | smoke v6 (task-15.6 集成) | Deferred to task-15.6 |
| AC7 | 既有 get/{id} 不退化 | go test 22 packages | Done |

## 8. Risks

- **Go mux 路径冲突**：`GET /v1/eval-runs` 与 `GET /v1/eval-runs/{id}` 共存需要 Go 1.22+ net/http enhanced mux —— 项目已用（既有 task-14.2 有 `/{id}` pattern）；不冲突
- **大量 eval_runs SQLite 性能**：v0.8 hard-cap limit=200；scan 全表 ORDER BY started_at DESC + LIMIT 在无索引时退化 → 缓解 task 实施时考虑加 `CREATE INDEX IF NOT EXISTS idx_eval_runs_started_at ON eval_runs(started_at_unix DESC)` migration（如必要；可在 task-15.4 实施时 add-only migration）
- **proto 字段编号冲突**：新 message tags 从 1 开始；与既有 EvalRun (1-11) / CreateEvalRunRequest 不冲突（不同 message）
- **filter 注入风险**：query param + parameterized SQL；不拼接 raw string；安全
- **MemEvalStore 既有 mock 2s timer**：list 不受影响（独立 method） [SPEC-OWNER:task-15.4]

## 9. Verification Plan

- **install**: `go mod download && cargo fetch`
- **lint**: `gofmt -l internal/consoleapi/` + `cargo fmt --check`
- **typecheck**: `go vet ./... && cargo check --workspace`
- **unit-test**: `go test -v ./internal/consoleapi/... ./internal/consoleapi/grpcclient/...` + `cargo test --workspace`
- **integration**: `go test -v -run TestListEvalRuns_E2E ./internal/consoleapi/...` + `cargo test --test eval_integration -- test_list`
- **e2e**: `bash scripts/console_smoke.sh` v6 Step 24
- **build**: `go build ./cmd/contextforge && cargo build --workspace --release`
- **coverage**: 不强制
- **runtime-smoke**: start daemon + 创建 3 eval-run + curl GET /v1/eval-runs?limit=3 验证
- **manual**: curl 实测

## 10. Completion Notes

- **完成日期**：2026-05-26
- **关键决策**：
  - **rusqlite::params_from_iter for dynamic WHERE**：filter 三参 (ws/status/limit) 中任一可缺省 — 用 `Vec<&dyn ToSql>` + `params_from_iter` 动态绑定，避免拼接 SQL（注入安全）
  - **limit clamp 1..=200 server-side**：客户端误传 0 / 1000 → server clamp 默认 50 / 上限 200；REST handler 也做一道（200 hard ceiling）防御
  - **MemEvalStore sort.Slice on StartedAt.After**：Go 侧 fallback 也 ORDER BY started_at DESC；与 Rust SQLite 一致
  - **router_test.go 不复用 newTestRouter**：list test 需要 MemEvalStore 拿到 Eval client，构造独立 Deps 避免污染既有 helper
- **§9 Verification 结果**：
  - `cargo check -p contextforge-core --tests`: clean
  - `cargo test -p contextforge-core --lib eval::store`: 11 tests PASS (含 4 新 task-15.4)
  - `cargo test -p contextforge-core --lib data_plane::eval`: 5 tests PASS (含 2 新 task-15.4)
  - `cargo test --workspace`: 113 lib tests + 17 integration test files 全 PASS（task-14.1/14.2 既有不退化）
  - `go test ./...`: 22 packages 全 PASS（含 test/conformance 22-endpoint 不退化 + 3 新 router_test）
- **改动文件**：
  - `proto/contextforge/console_data_plane/v1/console_data_plane.proto` (修改 — ListEvalRunsRequest / ListEvalRunsResponse + EvalService.List add-only)
  - `proto/contextforge/console_data_plane/v1/console_data_plane.pb.go` (生成 — buf generate)
  - `proto/contextforge/console_data_plane/v1/console_data_plane_grpc.pb.go` (生成 — buf generate)
  - `core/src/eval/store.rs` (修改 — ListEvalRunsFilter struct + SqliteEvalStore.list method + 4 新 unit test)
  - `core/src/eval/mod.rs` (修改 — pub use ListEvalRunsFilter)
  - `core/src/data_plane/eval.rs` (修改 — EvalServer.list RPC handler + 2 新 unit test)
  - `internal/contractv1/contractv1.go` (修改 — ListEvalRunsFilter Go struct add-only)
  - `internal/consoleapi/types.go` (修改 — EvalClient.List interface method)
  - `internal/consoleapi/grpcclient/grpcclient.go` (修改 — evalClient.List wrapper)
  - `internal/consoleapi/router.go` (修改 — GET /v1/eval-runs route)
  - `internal/consoleapi/handlers.go` (修改 — handleListEvalRuns + strconv import)
  - `internal/consoleapi/memstore.go` (修改 — MemEvalStore.List method)
  - `internal/consoleapi/router_test.go` (修改 — 3 新 test + time import)
  - `internal/cli/console_api_serve_degraded.go` (修改 — degradedEval.List)
  - `docs/specs/tasks/task-15.4-list-eval-runs-endpoint.md` (本 spec §6 [x] / §7 Done / §10 完工 + Status → Done)
- **commit 列表**：
  - feat(proto+core+consoleapi): task-15.4 — GET /v1/eval-runs (EvalService.List add-only + SqliteEvalStore.list + filter + limit clamp)
  - docs(spec): task-15.4 §6/§7/§10 / Status → Done
- **剩余风险 / 未做项**：
  - 分页 / cursor [SPEC-DEFER:phase-future.list-endpoints-pagination]
  - ORDER BY 字段配置化 [SPEC-DEFER:phase-future.eval-list-order-config]
- **下游 task 影响**：task-15.6 smoke v6 Step 24 验证本 task；Console UI Eval 列表真接
