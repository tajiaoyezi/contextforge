# Task `51.3`: `go-rest-workspace-owner — Go REST workspace handler 用 verified identity`

**Status**: Done
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 51 (workspace-isolation)
**Dependencies**: task-51.1/51.2（WorkspaceStore + proto owner）/ ADR-016 D3（Go thin proxy）

## 1. Background
workspace handler 不读 verified identity。本 task 让 POST/GET /v1/workspaces 从 context 读 verified userID 传给 grpcclient。

## 2. Goal
(1) POST /v1/workspaces：verified user → owner_id = verified userID；trusted-network → owner_id = ""（unowned）。
(2) GET /v1/workspaces：verified user → 仅自己 own 的 + unowned；trusted-network → all（byte-equiv）。
(3) GET /v1/workspaces/{id}：verified user → 仅自己 own 的或 unowned；否则 403；trusted-network → all。

## 3. Scope
- 改 `internal/consoleapi/handlers.go`：handleCreateWorkspace/handleListWorkspaces/handleGetWorkspace 读 verifiedUserIDKey context
- 改 `internal/consoleapi/grpcclient/grpcclient.go`：workspaceClient.Create 加 owner 参数；List/Get 加 owner filter 参数
- 改 `internal/consoleapi/types.go`：WorkspaceClient interface 加 owner 参数（或新方法 ListOwned/GetIfOwned）
- 单测：POST with per-user token → owner_id = userID；GET list → 仅自己 + unowned；trusted-network → all

## 4.1 行为契约（byte-equivalent 关键）
- **trusted-network（空 token）**：owner_id = ""（unowned）；list/get → all（v2.0.0-alpha 行为不变）
- **旧 shared token**：同 trusted-network（无 verified identity 注入）
- **per-user token**：owner_id = verified userID；list → 自己 own + unowned；get → 403 if 非 own 且非 unowned

## 6. AC
- [x] **AC1**: POST workspace with per-user token → owner_id = verified userID — verified by **TEST-51.3.1**
- [x] **AC2**: GET workspaces with per-user token → 仅自己 own 的 + unowned — verified by **TEST-51.3.2**
- [x] **AC3**: trusted-network byte-equivalent（all visible，owner_id=""）— verified by **TEST-51.3.3**

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-51.3.1 | POST workspace owner_id=verified userID | go test | Pass |
| TEST-51.3.2 | GET list 仅自己 own + unowned | go test | Pass |
| TEST-51.3.3 | trusted-network byte-equivalent | go test | Pass |
| TEST-51.3.4 | legacy shared token byte-equivalent（额外覆盖）| go test | Pass |
| TEST-51.3.5 | GET not-owned → 403 Forbidden（GetIfOwned dispatch）| go test | Pass |

## 9. Verification
```bash
go test ./internal/consoleapi/ -run TestTask513 -v
go test ./internal/cli/ # no-regression
```

## 10. Completion Notes
**Status**: Done
1. **完成日期**：2026-07-03
2. **改动文件**：
   - `internal/consoleapi/types.go` — `WorkspaceClient` 接口加 3 个 owner-scoped 方法（CreateOwned / ListOwned / GetIfOwned），保留既有 Create/List/Get/Update byte-equivalent。
   - `internal/contractv1/contractv1.go` — `Workspace` wire struct 加 `OwnerID string \`json:"owner_id,omitempty"\``（omitempty → unowned workspace JSON byte-equivalent v1.x）。
   - `internal/consoleapi/grpcclient/grpcclient.go` — `workspaceClient` 加 CreateOwned/ListOwned/GetIfOwned 三个 gRPC wrapper（调 task-51.2 新增 RPC）；`protoToWorkspace` 携带 `OwnerId`。
   - `internal/consoleapi/memstore.go` — `WorkspaceAdapter` 加 3 个 owner-scoped 方法（fallback 不跟踪 owner_id，delegate 到非 owner 方法 — 真实归属判定在 Rust WorkspaceStore task-51.1）。
   - `internal/cli/console_api_serve_degraded.go` — `degradedWorkspace` 加 3 个 owner-scoped 方法（均返回 503 byte-equivalent）。
   - `internal/consoleapi/handlers.go` — handleCreateWorkspace / handleListWorkspaces / handleGetWorkspace 读 `verifiedUserIDKey{}` context：verified user → owner-scoped 方法；trusted-network / legacy shared token → 既有方法 byte-equivalent。handleGetWorkspace 在 GetIfOwned 返 nil 时 403 Forbidden（非 own）。handlePatchWorkspaceConfig 保持 byte-equivalent（config 更新本 phase 不查 owner）。
   - `internal/consoleapi/workspace_owner_test.go` — 新增 fake `ownerCapturingWorkspace` + 5 个测试（TestTask513_1..5）。
3. **commit 列表**：见本 task 最终 commit（feat(workspace): task-51.3 ...）。
4. **§9 Verification 结果**：
   - `go test ./internal/consoleapi/ -run TestTask513 -v -count=1` → PASS（5/5）
   - `go test ./internal/consoleapi/ -count=1` → PASS（no-regression）
   - `go test ./internal/cli/ -count=1` → PASS（no-regression）
   - `go vet ./internal/consoleapi/ ./internal/cli/` → clean
   - `gofmt -w` → applied（all modified .go files）
5. **剩余风险**：
   - handlePatchWorkspaceConfig 仍 byte-equivalent（不查 owner）— config 更新的 owner gate 留 v1.x（[SPEC-DEFER:phase-future.workspace-config-owner-gate]）。
   - MemStore fallback owner-scoped 方法 delegate 到非 owner 方法（fallback 本就单用户本地优先，不强制归属 — ADR-016 §D4）；真实 owner 过滤在 Rust WorkspaceStore task-51.1 经 grpcclient 路径生效。
6. **下游影响**：task-51.4（SearchService thin gate 据本 task verified identity 贯穿）
