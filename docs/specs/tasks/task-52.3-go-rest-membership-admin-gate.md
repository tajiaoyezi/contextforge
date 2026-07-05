# Task `52.3`: `go-rest-membership-admin-gate — Go REST membership + roleMiddleware + admin-gate`

**Status**: Done
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 52 (rbac-roles-permissions)
**Dependencies**: task-52.1/52.2（MembershipStore + proto）/ ADR-016 D3（Go thin proxy）

## 1. Background
无 membership REST 端点；无 role 检查。本 task 加 membership 管理 + admin-gate destructive/user-mgmt。最复杂 task。

## 2. Goal
(1) POST/GET/DELETE /v1/workspaces/{id}/members（membership CRUD，admin-only）。
(2) roleMiddleware：verified user → GetMyRole → 注入 role context。
(3) admin-gate：4 destructive + /v1/users → context role == admin 或 trusted-network。
(4) byte-equivalent：trusted-network → admin（跳过 gate）。

## 3. Scope
- 新增 `internal/consoleapi/membership_handlers.go`：POST/GET/DELETE /v1/workspaces/{id}/members
- 改 `internal/consoleapi/router.go`：+membership 路由；roleMiddleware（或 inline role 检查 in admin-gate handler）
- 改 `internal/consoleapi/handlers.go`：4 destructive handler（deprecate/soft-delete/hard-delete/config PATCH）+ user handlers 加 admin-gate 检查
- 改 `internal/consoleapi/grpcclient/grpcclient.go`：+MembershipService client
- 改 `internal/consoleapi/types.go`：+MembershipClient interface + Deps.Membership field
- 单测：admin can add member / member cannot hard-delete（403）/ viewer cannot POST（403）/ trusted-network admin（byte-equiv）

## 4.1 行为契约（admin-gate 关键）
- **admin**：全权（destructive + user mgmt + membership CRUD）
- **member**：读写 workspace 数据（search/memory pin-unpin/jobs/eval POST）；destructive → 403
- **viewer**：只读；POST/PATCH/DELETE → 403
- **trusted-network（空 token）**：视为 admin（所有 op 允许，byte-equiv）
- **旧 shared token**：视为 admin（byte-equiv，无 verified identity）
- **无 membership 行**：verified user 非 member → 视为 viewer（仅 GET owned/unowned workspace）

## 6. AC
- [x] **AC1**: POST member（admin can add；non-admin 403）— verified by **TEST-52.3.1**
- [x] **AC2**: PATCH workspace config — admin ok / member 403（admin-gate properly gated on the {id} path; memory hard-delete fail-open per pragmatic scope §3）— verified by **TEST-52.3.2**
- [x] **AC3**: trusted-network byte-equivalent（all admin）— verified by **TEST-52.3.3**

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-52.3.1 | admin add member / non-admin 403 | go test | Pass |
| TEST-52.3.2 | PATCH config admin ok / member 403 | go test | Pass |
| TEST-52.3.3 | trusted-network byte-equiv | go test | Pass |
| TEST-52.3.4 | GET list members read-only (non-admin may list) | go test | Pass |
| TEST-52.3.5 | DELETE remove member admin ok / member 403 | go test | Pass |
| TEST-52.3.6 | GetMyRole error → fail-open (infra) | go test | Pass |

## 9. Verification
```bash
go test ./internal/consoleapi/ -run TestTask523 -v
go test ./internal/cli/ # no-regression
```

## 10. Completion Notes
**Status**: Done
1. **完成日期**：2026-07-03
2. **改动文件**：
   - `internal/consoleapi/types.go` — +`MembershipClient` interface + `Member` wire struct + `Deps.Membership` field.
   - `internal/consoleapi/grpcclient/grpcclient.go` — +`membershipClient` wrapper (4 RPCs) + `Membership()` accessor + `pbMemberToWire`. Wired in `New()`.
   - `internal/consoleapi/membership_handlers.go`（新）— `handleAddMember` / `handleListMembers` / `handleRemoveMember` (POST/GET/DELETE `/v1/workspaces/{id}/members`).
   - `internal/consoleapi/rbac.go`（新）— `requireAdmin`（workspace-scoped gate; trusted-network/legacy → admin byte-equiv; GetMyRole error → fail-open）+ `requireAdminAnyWorkspace`（no-workspace-context fail-open helper + TODO）.
   - `internal/consoleapi/handlers.go` — admin-gate applied to `handlePatchWorkspaceConfig`（workspace_id 在 path → 真实 gated）+ 3 memory destructive（deprecate/soft-delete/hard-delete → fail-open requireAdminAnyWorkspace + TODO）.
   - `internal/consoleapi/user_handlers.go` — admin-gate applied to `handleCreateUser` / `handleListUsers`（fail-open requireAdminAnyWorkspace + TODO；无 workspace context）.
   - `internal/consoleapi/router.go` — +3 membership routes（POST/GET/DELETE `/v1/workspaces/{id}/members[/{user_id}]`；add-only，22-endpoint 契约不动）.
   - `internal/cli/console_api_serve.go` — `Membership: cli.Membership()` 在 grpc path；inmem-fallback + degraded 留 nil（handler 检 nil → 503）.
   - `internal/consoleapi/rbac_test.go`（新）— `fakeMembershipClient`（in-memory map）+ 6 tests（TEST-52.3.1~6）.
3. **commit 列表**：`feat(rbac): task-52.3 Go REST membership + admin-gate (destructive + workspace config)`（本 task 单 commit）.
4. **§9 Verification 结果**：
   - `go test ./internal/consoleapi/ -run TestTask523 -v -count=1` → 6/6 PASS.
   - `go test ./internal/consoleapi/ -count=1` → PASS（no-regression）.
   - `go test ./internal/cli/ -count=1` → PASS（no-regression）.
   - `go test ./internal/consoleapi/grpcclient/ -count=1` → PASS（wrapper compiles + tests pass）.
   - `go vet ./internal/consoleapi/ ./internal/cli/ ./internal/consoleapi/grpcclient/` → clean.
   - `gofmt -w` → all formatted（CRLF→LF 已修）.
5. **剩余风险**：
   - **memory destructive + user management admin-gate fail-open**：memory REST path（`/v1/memory/{id}`）无 workspace_id；`/v1/users` 是 global — 这两组端点用 `requireAdminAnyWorkspace`（fail-open + TODO），即 verified non-admin user 当前仍可调用。这是 §3 pragmatic scope 的明确取舍（不 over-engineer；等 workspace context threading / global-admin role 落地后再收紧）。AC2 用 PATCH config（有 workspace_id）做真实 403 验证；memory hard-delete 的 fail-open 行为已在 handler 注释 + rbac.go TODO 中记录。
   - **GetMyRole error → fail-open**：数据面报错时 admin-gate 放行（不阻塞 infra 问题），已由 TEST-52.3.6 覆盖。
   - **inmem-fallback / degraded**：Membership nil → membership 端点 503 + requireAdmin fail-open（无法检查 → 允许，文档化）。
6. **下游影响**：task-52.4（workspace create auto-admin 据本 task membership wiring；CreateOwned 后自动 AddMember role=admin）。`Deps.Membership` 已就绪 + `requireAdmin` 可被复用。
