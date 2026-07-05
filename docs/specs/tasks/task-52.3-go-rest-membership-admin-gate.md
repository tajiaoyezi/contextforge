# Task `52.3`: `go-rest-membership-admin-gate — Go REST membership + roleMiddleware + admin-gate`

**Status**: Ready
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
- [ ] **AC1**: POST member（admin can add；non-admin 403）— verified by **TEST-52.3.1**
- [ ] **AC2**: member can search but not hard-delete（403）— verified by **TEST-52.3.2**
- [ ] **AC3**: trusted-network byte-equivalent（all admin）— verified by **TEST-52.3.3**

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-52.3.1 | admin add member / non-admin 403 | go test | Not Started |
| TEST-52.3.2 | member search ok / hard-delete 403 | go test | Not Started |
| TEST-52.3.3 | trusted-network byte-equiv | go test | Not Started |

## 9. Verification
```bash
go test ./internal/consoleapi/ -run TestTask523 -v
go test ./internal/cli/ # no-regression
```

## 10. Completion Notes
**Status**: Ready
1. **完成日期**：<TBD-after-impl>
2. **改动文件**：<TBD-after-impl>
3. **commit 列表**：<TBD-after-impl>
4. **§9 Verification 结果**：<TBD-after-impl>
5. **剩余风险**：<TBD-after-impl>
6. **下游影响**：task-52.4（workspace create auto-admin 据本 task membership wiring）
