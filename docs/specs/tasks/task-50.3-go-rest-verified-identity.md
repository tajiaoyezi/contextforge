# Task `50.3`: `go-rest-verified-identity — Go REST 注册 + bearer 解析 verified identity + actor 覆写`

**Status**: Done
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 50 (identity-foundation)
**Dependencies**: task-50.1/50.2（UserStore + UserService gRPC）/ ADR-016 D3（Go thin proxy）

## 1. Background
当前 bearer middleware 是单一共享 token；X-Actor 是 caller 自填未验证。本 task 让 bearer 匹配 users.token → 注入 verified userID 到 context → handler 覆写 actor。最复杂 task。

## 2. Goal
(1) POST /v1/users 注册（返回 token）+ GET /v1/users list。
(2) bearerAuthMiddleware 扩展：token ∈ users.token → 注入 userID context；旧 shared-token → 不注入（向后兼容）；空 → trusted-network（byte-equivalent）。
(3) handlers.go:559,625 actor 从 context 读 verified userID（覆写 X-Actor）；trusted-network 回落 `"console-api"`。

## 3. Scope
- 新增 `internal/consoleapi/user_handlers.go`：POST /v1/users（name → create → 返 token）/ GET /v1/users（list）
- 改 `internal/consoleapi/router.go` bearerAuthMiddleware：调 grpcclient.UserService.GetUserByToken；匹配 → context.WithValue(userID)；不匹配 → 检查旧 shared-token；都失败 → 401
- 改 `internal/consoleapi/handlers.go:559,625`：actor = ctx userID（有）/ 否则 X-Actor（向后兼容旧路径）/ 否则 `"console-api"`（trusted-network）
- 改 `internal/consoleapi/grpcclient/grpcclient.go`：+UserService client（CreateUser/GetUserByToken/ListUsers）
- 改 `internal/consoleapi/types.go`：Deps +UserService client
- 单测：注册 → 用 token 调 pin → actor=verified userID；trusted-network byte-equiv；旧 shared-token 仍工作

## 4.1 行为契约（关键：byte-equivalent）
- **token ∈ users.token**：middleware 注入 userID context；handler actor=userID
- **token = 旧 shared-token（CONTEXTFORGE_CONSOLEAPI_AUTH_TOKEN）**：不注入 context；handler actor=X-Actor 声明值（旧行为）
- **空 token（trusted-network）**：不注入 context；handler actor=X-Actor 或回落 `"console-api"`（byte-equivalent）
- **POST /v1/users**：trusted-network 或 admin token 可注册；普通 user token 不可（防止任意 user 创建）—— 初始实现：trusted-network 或任何有效 token 都可注册（简化；admin 分级 `[SPEC-DEFER:phase-future.rbac-roles-permissions]` 留 Phase 52 RBAC）

## 6. AC
- [x] **AC1**: POST /v1/users 注册 → 返 token → 用该 token 调 pin → actor=verified userID — verified by **TEST-50.3.1**
- [x] **AC2**: trusted-network（空 token）byte-equivalent（actor 仍 `"console-api"` 回落）— verified by **TEST-50.3.2**
- [x] **AC3**: 旧 shared token（CONTEXTFORGE_CONSOLEAPI_AUTH_TOKEN）仍工作（actor=X-Actor 声明值，旧行为）— verified by **TEST-50.3.3**

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-50.3.1 | 注册→token→pin actor=verified userID | go test | Done |
| TEST-50.3.2 | trusted-network byte-equivalent | go test | Done |
| TEST-50.3.3 | 旧 shared token 向后兼容 | go test | Done |

## 9. Verification
```bash
go test ./internal/consoleapi/ -run TestTask503 -v
go test ./internal/cli/ # no-regression
```

## 10. Completion Notes
**Status**: Done
1. **完成日期**：2026-07-05
2. **改动文件**：
   - internal/consoleapi/types.go（+UserClient interface +User struct +ErrConflict +Deps.User field）
   - internal/consoleapi/user_handlers.go（新增，handleCreateUser/handleListUsers + verifiedActor helper）
   - internal/consoleapi/router.go（+/v1/users 路由 + bearerAuthMiddleware 加 UserClient 参数 + verifiedUserIDKey context + verified identity 解析逻辑 + context import）
   - internal/consoleapi/handlers.go（pin/unpin actor 用 verifiedActor 覆写 X-Actor）
   - internal/consoleapi/grpcclient/grpcclient.go（+userClient impl + Client.user field + New wiring + User() accessor + AlreadyExists→ErrConflict mapping + codes/status import 已有）
   - internal/cli/console_api_serve.go（Deps.User wiring from grpcclient）
   - internal/consoleapi/user_identity_test.go（新增，TestTask503_AC1/AC2/AC3）
3. **commit 列表**：
   - <GREEN> feat(identity): task-50.3 Go REST 注册 + bearer verified identity + actor 覆写
4. **§9 Verification 结果**：
   - go test: 3 passed / 0 failed（TestTask503_AC1 verified-actor-override + AC2 byte-equiv + AC3 legacy-compat）+ full consoleapi package no-regression ✅ + full cli ✅
   - go vet ✅ / gofmt clean
5. **剩余风险**：bearer middleware 对 per-user token 多一次 GetByToken gRPC 调用（低 QPS 可接受；高 QPS 可加 token cache）；trusted-network + 旧 shared token byte-equivalent 验证通过（AC2/AC3）
6. **下游影响**：task-50.4（redeem marker 据本 task 实测 verified actor 贯穿；本 task AC1 证明 X-Actor 被覆写）
