# Task `13.2`: `go-memory-rest-handlers — Console REST 5 memory endpoint + grpcclient.MemoryClient + confirmMiddleware on deprecate/soft-delete`

**Status**: Ready

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 13 (memory-rest-surface)
**Dependencies**: task-13.1 (Rust MemoryService gRPC 5 RPC + SqliteMemoryStore 已 ship) + task-12.1 (confirmMiddleware 已 ship + WorkspaceClient.Update + JobClient.ListActive grpcclient pattern) + [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D1 Wave 3 / D2

## 1. Background

task-13.1 已在 Rust 侧 ship 完整 MemoryService gRPC（5 RPC: List / Get / Pin / Deprecate / SoftDelete）+ `memory_items` 表 + audit hooks。本 task 在 Go 侧补完 REST 表面 + grpcclient wrapper + MemStore fallback：

- 5 个 REST endpoint：
  - `GET /v1/memory?agent_id=&scope=&namespace=` → `[]MemoryItem`（不走 confirmMiddleware）
  - `GET /v1/memory/{id}` → `MemoryItem`（不走 confirmMiddleware）
  - `POST /v1/memory/{id}/pin` → 204（pin 非破坏性，不走 confirmMiddleware）
  - `POST /v1/memory/{id}/deprecate` → 204（破坏性，走 task-12.1 confirmMiddleware；缺 X-Confirm 返 412）
  - `POST /v1/memory/{id}/soft-delete` → 204（破坏性，走 confirmMiddleware）

`internal/consoleapi/grpcclient/grpcclient.go` 加 `MemoryClient` struct + 5 method wrapper；`internal/consoleapi/types.go` 加 MemoryClient 接口 + Deps 加 Memory 字段；`internal/consoleapi/memstore.go` MemStore 加 MemoryAdapter（fallback 模式下 list/get 用 in-memory map seed；pin/deprecate/soft-delete 返 ErrDataPlaneUnavailable）；`scripts/console_smoke.sh` 升 v4（15 endpoint → 20 endpoint）含 memory 5 step。

## 2. Goal

Go `internal/consoleapi/` 加 5 个新 REST endpoint；router 注册 5 路由（2 个走 confirmMiddleware）；grpcclient.MemoryClient 5 method wrapper + error mapping；MemStore fallback 模式下 list/get 工作；`go test ./internal/consoleapi/...` 全绿（task-11.x / task-12.x 既有不退化）；conformance test (TestConsoleContractV1Conformance) 不退化；`bash scripts/console_smoke.sh` v4 REAL mode 20 endpoint flow `CONSOLE_REAL_SMOKE_EXIT=0`；≥6 单元测试 + ≥1 集成测试 + ≥5 smoke sub-step PASS。

## 3. Scope

### In Scope

- **修改 `internal/consoleapi/types.go`**：
  - 加 `MemoryClient` 接口 5 method：
    ```go
    type MemoryClient interface {
        List(filter MemoryListFilter) ([]contractv1.MemoryItem, error)
        Get(memoryID string) (*contractv1.MemoryItem, error)  // nil if not found
        Pin(memoryID string, pin bool) error                   // pin=false = unpin
        Deprecate(memoryID string) error
        SoftDelete(memoryID string) error
    }

    type MemoryListFilter struct {
        AgentID, Scope, Namespace string
        IncludeSoftDeleted bool
    }
    ```
  - `Deps` 加 `Memory MemoryClient` 字段
- **修改 `internal/consoleapi/router.go`**：
  - 路由注册：
    ```go
    mux.HandleFunc("GET /v1/memory", handleListMemory(deps))
    mux.HandleFunc("GET /v1/memory/{id}", handleGetMemory(deps))
    mux.HandleFunc("POST /v1/memory/{id}/pin", handleMemoryPin(deps))  // not confirm-gated
    mux.HandleFunc("POST /v1/memory/{id}/deprecate", confirmMiddleware(handleMemoryDeprecate(deps)))
    mux.HandleFunc("POST /v1/memory/{id}/soft-delete", confirmMiddleware(handleMemorySoftDelete(deps)))
    ```
- **修改 `internal/consoleapi/handlers.go`**：
  - 新增 5 handler：
    - `handleListMemory(deps)`: parse `?agent_id=`/`?scope=`/`?namespace=`/`?include_soft_deleted=true` query → `deps.Memory.List(filter)` → 返 200 + `[]MemoryItem`（空集 → `[]`）
    - `handleGetMemory(deps)`: PathValue id → `deps.Memory.Get(id)` → 200 + MemoryItem / 404 + ErrorBody
    - `handleMemoryPin(deps)`: PathValue id → `deps.Memory.Pin(id, true)` → 204 No Content (no body)；404 if not found
    - `handleMemoryDeprecate(deps)`: PathValue id → `deps.Memory.Deprecate(id)` → 204 / 404
    - `handleMemorySoftDelete(deps)`: PathValue id → `deps.Memory.SoftDelete(id)` → 204 / 404
- **修改 `internal/consoleapi/grpcclient/grpcclient.go`**：
  - 加 `MemoryClient` struct + `proto.MemoryServiceClient` 字段
  - 加 5 method:
    - `List(filter MemoryListFilter)`: 调 `proto.MemoryService.List(ctx, &proto.ListMemoryRequest{AgentId, Scope, Namespace, IncludeSoftDeleted})` → protoToMemoryList(resp.Items) → 返
    - `Get(id)`: → MemoryItem / nil if NotFound
    - `Pin(id, pin)`: → empty / NotFound mapping
    - `Deprecate(id)` / `SoftDelete(id)`: 同
  - `New(addr, opts...)` 加 `memory: proto.NewMemoryServiceClient(conn)` 字段
  - 加 `protoToMemoryItem(*proto.MemoryItem) contractv1.MemoryItem` helper
- **修改 `internal/consoleapi/memstore.go`**：
  - 加 `MemMemoryStore` struct + `items map[string]contractv1.MemoryItem`
  - 实现 `List(filter)`: in-memory filter + 返 slice
  - 实现 `Get(id)`: 返 *MemoryItem or nil
  - 实现 `Pin(id, pin)`: 更新 in-memory + 返 nil
  - 实现 `Deprecate(id)` / `SoftDelete(id)`: 更新 status 字段 + 返 nil（fallback 模式下不写 audit）
  - 注：fallback 模式下 5 method 都返 success 行为，区别仅在「重启即丢」+「不写 audit log」；这与 ADR-016 D4 fallback 语义一致（degraded 但功能可用 for demo）
  - 在 `console-api-serve` 启动时 if `CONSOLE_API_FALLBACK_INMEM=1` → seed 5 个 fixture memory_items（hardcoded in MemStore for demo）
- **修改 `internal/cli/console_api_serve.go`**：
  - `buildDeps` helper 加 Memory client 构造：grpc mode → `grpcclient.New(...).Memory`；fallback mode → `&memstore.Memory{items: seedFixtures()}`
- **修改 `scripts/console_smoke.sh`** v4：
  - step 15 → step 20 共 5 新 step:
    - step 16: pre-step — seed 5 memory items via `sqlite3 $DATA_DIR/memory.db < test/fixtures/memory-seed/seed.sql`（[SPEC-DEFER:dev-mode-seed] 或 daemon `--seed-fixtures` flag）
    - step 17: GET /v1/memory → 验证返 5 items
    - step 18: GET /v1/memory/<id> → 验证返单条
    - step 19: POST /v1/memory/<id>/pin → 验证 204 + GET 返 is_pinned=true
    - step 20: POST /v1/memory/<id>/deprecate WITHOUT X-Confirm → 验证 412；带 -H "X-Confirm: yes" → 204 + GET 返 status="deprecated"
    - step 21: POST /v1/memory/<id>/soft-delete WITH X-Confirm: yes → 204 + GET /v1/memory 默认不返该项；GET /v1/memory?include_soft_deleted=true 返该项
- **修改 `test/fixtures/memory-seed/seed.sql`** (新增)：5 个 INSERT 语句 + agent_scope 分布覆盖测试
- **单元测试 ≥6**（`internal/consoleapi/handlers_test.go` + `grpcclient/grpcclient_test.go`）：
  - `TestListMemory_Filter_Combinations` (agent_id / scope / namespace 单 / 组合 → 调 grpcclient.List with proper filter)
  - `TestGetMemory_404_when_missing`
  - `TestMemoryPin_204_no_body`
  - `TestMemoryDeprecate_412_when_missing_confirm` (sanity check; confirmMiddleware 行为复用 task-12.1 unit test)
  - `TestMemoryDeprecate_204_with_confirm_header`
  - `TestMemoryDeprecate_204_with_confirm_query`
  - `TestMemorySoftDelete_412_then_204_then_excluded_from_default_list`
  - `TestMemoryClient_Maps_Errors` (NotFound → ErrNotFound / Unavailable → ErrDataPlaneUnavailable)
- **集成测试 ≥1**：
  - `internal/consoleapi/e2e_grpc_test.go::TestMemoryEndpoints_E2E_GrpcBacked` (spawn Rust daemon + Go console-api-serve + seed via SQL + REST flow 5 endpoint)
- **文件锚点**：`internal/consoleapi/{types,router,handlers,memstore}.go` + `internal/consoleapi/grpcclient/grpcclient.go` + `internal/cli/console_api_serve.go` + `scripts/console_smoke.sh` v4 + `test/fixtures/memory-seed/seed.sql`
- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **Rust SqliteMemoryStore + MemoryService gRPC** [SPEC-OWNER:task-13.1]：本 task 仅 Go REST 层
- **importer 改造写入 memory_items** [SPEC-DEFER:phase-15.import-to-memory-items]
- **memory create REST endpoint**：Console 22-endpoint 不含
- **POST /v1/memory/{id}/unpin** (本 task `Pin(id, false)` API 已支持但 Console UI 端 v1.0 只有 `/pin` endpoint；如 Console 需要 separate `/unpin` 路径 → cross-repo amendment [SPEC-DEFER:console-memory-unpin])
- **Eval endpoints** [SPEC-OWNER:phase-14]

## 4. Users / Actors

- **Console UI 端**（下游，via cross-repo）：Memory 管理面板 list / detail / 三按钮 (pin / deprecate / soft-delete)
- **task-14.1/14.2 eval 实施 agent**（下游 phase）：复用本 task 5 endpoint 模板 + confirmMiddleware 模式

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/decisions/adr-017-console-contract-completion-22-endpoint.md` §D1 Wave 3 / §D2 / §D6
- `docs/decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md` §D3 / §D4
- `docs/specs/phases/phase-13-memory-rest-surface.md` §3 / §6
- `docs/specs/tasks/task-13.1-rust-memory-grpc-service.md` (MemoryService 5 RPC 接口)
- `docs/specs/tasks/task-12.1-quick-win-rest-endpoints.md` (confirmMiddleware pattern + 204 cancel 改造模式)
- `H:/devlopment/code/contextforge/internal/contractv1/contractv1.go::MemoryItem` (9 字段)

### 5.2 Imports

- **Go**: 现有 stdlib `net/http` + `encoding/json`；现有 `internal/consoleapi/grpcclient/`；现有 `google.golang.org/grpc`
- **不引入新依赖**：R7 不触发；`go.mod` 不动

### 5.3 Routes 新增形状

```go
// internal/consoleapi/router.go
mux.HandleFunc("GET /v1/memory", handleListMemory(deps))                                  // NEW (task-13.2)
mux.HandleFunc("GET /v1/memory/{id}", handleGetMemory(deps))                              // NEW
mux.HandleFunc("POST /v1/memory/{id}/pin", handleMemoryPin(deps))                         // NEW (pin 非破坏性)
mux.HandleFunc("POST /v1/memory/{id}/deprecate", confirmMiddleware(handleMemoryDeprecate(deps)))   // NEW (破坏性)
mux.HandleFunc("POST /v1/memory/{id}/soft-delete", confirmMiddleware(handleMemorySoftDelete(deps)))  // NEW (破坏性)
```

## 6. Acceptance Criteria

- [ ] AC1：`GET /v1/memory` 走 gRPC MemoryService.List + filter；query params (agent_id / scope / namespace / include_soft_deleted) 各组合工作；空集 → 200 + `[]` — **verified by unit-test `TestListMemory_Filter_Combinations` + integration `TestMemoryEndpoints_E2E_GrpcBacked` step list PASS**
- [ ] AC2：`GET /v1/memory/{id}` 真返 MemoryItem 9 字段；不存在 → 404；`POST /v1/memory/{id}/pin` → 204 no body + 后续 GET 返 is_pinned=true (Rust 持久化) — **verified by unit-test 3 cases + integration step get/pin PASS**
- [ ] AC3：`POST /v1/memory/{id}/deprecate` 缺 X-Confirm → 412 PRECONDITION_FAILED + ErrorBody；`X-Confirm: yes` header **或** `?confirm=true` query 任一 → 204 + Rust 持久化 status="deprecated" + AuditSink 写入一条 op_type="deprecate" — **verified by unit-test `TestMemoryDeprecate_*` 3 cases + integration step deprecate PASS**
- [ ] AC4：`POST /v1/memory/{id}/soft-delete` 同款 412/204 行为 + Rust 持久化 status="soft_deleted"；list endpoint 默认不返该项；`include_soft_deleted=true` 返该项 — **verified by unit-test + integration `test_soft_delete_then_excluded_from_default_list` PASS**
- [ ] AC5：MemStore fallback 模式（`CONSOLE_API_FALLBACK_INMEM=1`）seed 5 fixture items + list/get/pin/deprecate/soft-delete 全工作（in-memory，不写 audit；重启即丢）；conformance test (TestConsoleContractV1Conformance) 在两种模式下都不退化 — **verified by go test fallback mode + conformance suite PASS**
- [ ] AC6：v0.4 + v0.5 既有 15 endpoint test 不退化；scripts/console_smoke.sh v4 20 endpoint flow `CONSOLE_REAL_SMOKE_EXIT=0` — **verified by §9 verify run all-green + smoke exit 0**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | GET /v1/memory 带 filter | router.go + handlers.go (handleListMemory) + grpcclient.Memory.List + tests | Ready |
| AC2 | GET /v1/memory/{id} + POST /pin → 204 | router.go + handlers.go + grpcclient + tests | Ready |
| AC3 | POST /deprecate + X-Confirm 412/204 + audit | router.go (confirmMiddleware) + handlers.go + tests | Ready |
| AC4 | POST /soft-delete + X-Confirm 412/204 + exclude from default list | router.go + handlers.go + tests | Ready |
| AC5 | MemStore fallback works for memory 5 method | memstore.go + console_api_serve.go (seed fixtures) + tests | Ready |
| AC6 | scripts/console_smoke.sh v4 20 endpoint exit 0 | scripts/console_smoke.sh + integration | Ready |

## 8. Risks

- **`POST /pin` 是否非破坏性？**：Console PRD spec 写「pin is 非破坏性（不走 X-Confirm）」；ContextForge 端按这条不加 confirmMiddleware；如未来 Console UI 改为 pin 也要 confirm → cross-repo amendment [SPEC-DEFER:console-pin-confirm]
- **`include_soft_deleted=true` query param 语义**：Console v1.0 contract 没明确这个 query param；ContextForge 端按 default false 隐藏 soft_deleted；提供 query param 允许 UI 调试 view soft-deleted；如 Console 不需要 → 删除（不破坏 default 行为）；缓解 task implementation 第一步 cross-repo 确认
- **MemStore fallback 模式下 pin/deprecate/soft-delete 是否真持久化？**：本 task §3 选 in-memory 持久化（仍写 in-memory map），与 ADR-016 D4 fallback 「degraded but functional」语义一致；不写 audit log；重启即丢；trade-off 接受
- **seed fixture 注入路径**：scripts/console_smoke.sh 内 sqlite3 CLI 直接 INSERT memory_items 表 vs daemon 启动 `--seed-fixtures` flag；本 task 选 sqlite3 CLI（不污染 daemon binary；smoke-only 用途）；[SPEC-DEFER:dev-mode-seed] 留 v0.6.x dev mode 注入
- **MemoryListFilter 在 MemStore 实现差异**：fallback mode in-memory filter vs gRPC mode Rust 服务端 filter；行为一致性测试覆盖 (AC5 conformance not regress) 兜底

## 9. Verification Plan

- **install**: `go mod download`
- **lint**: `gofmt -l internal/consoleapi/`
- **typecheck**: `go build ./...`
- **unit-test**: `go test -v ./internal/consoleapi/... ./internal/consoleapi/grpcclient/...`（≥6 新单测 + 既有不退化）
- **integration**: `go test -v -run TestMemoryEndpoints_E2E_GrpcBacked ./internal/consoleapi/...` + `go test -v -run TestConsoleContractV1Conformance ./test/conformance/...` (env-gated CONSOLE_REPO)
- **e2e**: 通过 integration + `bash scripts/console_smoke.sh` v4 REAL mode 20 endpoint flow
- **build**: `go build ./cmd/contextforge`
- **coverage**: 不强制
- **runtime-smoke**: `bash scripts/console_smoke.sh` REAL mode end-to-end 20 endpoint + manual curl 验证 412/204
- **manual**: curl `GET /v1/memory` + curl `POST /pin` 验证 204 + curl `POST /deprecate` 不带 X-Confirm 验证 412 + 带验证 204

## 10. Completion Notes

<!-- 完工时按 standard.md §8.3 6 项 schema 回填 -->

- **完成日期**：<待填>
- **改动文件**：
  - `internal/consoleapi/types.go` (修改 — MemoryClient 接口 + MemoryListFilter struct + Deps 加 Memory)
  - `internal/consoleapi/router.go` (修改 — 5 路由 + 2 走 confirmMiddleware)
  - `internal/consoleapi/handlers.go` (修改 — 5 新 handler)
  - `internal/consoleapi/memstore.go` (修改 — MemMemoryStore + 5 method + seed fixtures)
  - `internal/consoleapi/grpcclient/grpcclient.go` (修改 — MemoryClient struct + 5 method wrapper)
  - `internal/cli/console_api_serve.go` (修改 — buildDeps 加 Memory client 构造)
  - `internal/consoleapi/router_test.go` (修改 — 加 6+ unit test)
  - `internal/consoleapi/handlers_test.go` (修改 — 同)
  - `internal/consoleapi/grpcclient/grpcclient_test.go` (修改 — 加 1 unit test)
  - `internal/consoleapi/e2e_grpc_test.go` (修改 — TestMemoryEndpoints_E2E_GrpcBacked)
  - `scripts/console_smoke.sh` (修改 v4 — 15 → 20 endpoint flow + seed step + 5 memory step)
  - `test/fixtures/memory-seed/seed.sql` (新增 — 5 INSERT memory_items)
  - `docs/specs/tasks/task-13.2-go-memory-rest-handlers.md` (本 spec §6 / §7 / §10 / Status 推进)
- **commit 列表**：
  - feat(consoleapi+grpcclient): task-13.2 — 5 memory REST endpoint + grpcclient.MemoryClient + confirmMiddleware on deprecate/soft-delete + memstore fallback + smoke v4
  - docs(spec): task-13.2 §6/§7/§10 / Status → Done
- **§9 Verification 结果**：<待填>
- **剩余风险 / 未做项**：
  - importer 写入路径 [SPEC-DEFER:phase-15.import-to-memory-items]
  - daemon `--seed-fixtures` dev mode [SPEC-DEFER:dev-mode-seed]
  - POST /unpin separate endpoint (本 task `Pin(id, false)` 已支持; 留 Console 端确认是否需要 separate route)
- **下游 task 影响**：task-14.1/14.2 phase-14 复用 5 endpoint pattern + confirmMiddleware 模式
