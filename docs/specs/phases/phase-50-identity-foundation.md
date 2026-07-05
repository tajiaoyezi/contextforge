# Phase 50 · identity-foundation (B1 v2.0 第一步)

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 是 **v2.0 multi-user/auth 的第一步**——per-user 身份验证基础。关闭 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`（actor 从 declared 变 verified），为 Phase 51+（workspace isolation）和 Phase 52+（RBAC）打地基。
>
> **当前零身份层**（v1.1.0 审查确认）：单一共享 bearer token（或 trusted-network 开放）/ `X-Actor` header 是 caller 自填未验证 / proto+SQL 无 user/session/tenant / gRPC loopback-trusted。
>
> **方向锚点**（用户已定）：(a) 身份验证基础（不做 RBAC/workspace ownership）/ SQLite users 表（不引入 Postgres，保 ADR-004/016 local-first）/ Go 覆写 actor（ADR-016 D3 thin proxy，Rust 不改）。
>
> **入读顺序**：本 phase spec → 4 task spec → 源码锚点（`internal/consoleapi/router.go:80-96` bearerAuthMiddleware + `handlers.go:559,625` X-Actor 读取 + `grpcclient.go:728,746` actor→gRPC + `core/migrations/0017` 现有 actor 列 + `core/src/data_plane/memory.rs:247` SPEC-DEFER marker）→ ADR-004 / ADR-015 / ADR-016 / ADR-050 / ADR-051（本 phase 新增）。

## 1. 阶段目标

让 actor 从"caller 自填声明值"变成"verified 身份"。具体：

1. **per-user token → user 映射**：SQLite `users` 表（migration 0020），每 user 一个 token
2. **bearer 解析 verified identity**：middleware 匹配 `users.token` → 注入 user_id 到 request context
3. **Go 覆写 actor**：handler 从 context 读 verified user_id，覆写 X-Actor 声明值；gRPC actor 字段传 verified 值
4. **byte-equivalent 默认**：trusted-network 模式（空 token）行为不变，actor 仍回落 `"console-api"`

**具体 exit criteria（§6 AC）**：
1. task-50.1: ADR-051 + migration 0020 + UserStore
2. task-50.2: proto UserService（add-only）+ Rust gRPC handler
3. task-50.3: Go REST 注册 + bearer 解析 + actor 覆写（向后兼容）
4. task-50.4: redeem SPEC-DEFER + smoke + docs + closeout
5. ADR-014 D1-D5（第四十二次激活）

**版本号**：v2.0.0-alpha（task-50.4 closeout 定；身份基础是 v2.0 首交付，-alpha 标"进行中非完整 multi-user"）。

## 2. 业务价值

**地基价值**：当前任何调用方都能在 `X-Actor` 填任意值冒充他人（pin/unpin audit 归因可伪造）。Phase 50 让 actor 变成 verified 身份，消除冒充风险，并为后续 workspace isolation（Phase 51：per-user workspace access）和 RBAC（Phase 52：roles+permissions）提供 subject 绑定点。

### 50.1 ADR-051 + migration + UserStore（🟢 Rust）
SQLite users 表 + Rust UserStore（CRUD）。ADR-051 记录 per-user token / SQLite / Go 覆写 / 不做 RBAC/Postgres/OIDC 决策。

### 50.2 proto + Rust gRPC（🟢 proto add-only + Rust）
proto 新增 UserService（CreateUser/GetUserByToken/ListUsers），不动现有字段号（ADR-015 FROZEN）。Rust handler 调 UserStore。

### 50.3 Go REST + bearer 解析 + actor 覆写（🟢 Go，最复杂）
POST /v1/users 注册 + bearer middleware 匹配 users.token 注入 context + handler 覆写 actor。

### 50.4 closeout（🟢 文档 + smoke）
redeem SPEC-DEFER marker + README/RELEASE_NOTES + smoke gate。

**不在本 phase 范围**（诚实 OOS，均已登记 SPEC-DEFER）：RBAC/roles/permissions（Phase 52+ `[SPEC-DEFER:phase-future.rbac-roles-permissions]`）/ workspace owner/per-user access control（Phase 51+ `[SPEC-DEFER:phase-future.workspace-user-isolation]`）/ Postgres（破坏 ADR-004/016，不引入）/ OAuth/OIDC 外部 IdP（Phase 53+ `[SPEC-DEFER:phase-future.oauth-oidc-idp]` 委托 Caddy forward_auth）/ gRPC interceptor credentials（loopback-trusted 保持，ADR-016 D2）/ token hash 存（Phase 51，需 salt+HMAC `[SPEC-DEFER:phase-future.token-hash-storage]`）/ token rotation UI（Phase 54+）。

## 3. 涉及模块
- **50.1**: `core/migrations/0020_users.sql`（新增）+ `core/src/identity/mod.rs`+`store.rs`（新增）+ `docs/decisions/adr-051-identity-foundation.md`（新增）
- **50.2**: `proto/contextforge/console_data_plane/v1/console_data_plane.proto`（add-only UserService）+ `core/src/data_plane/user.rs`（新增）+ tonic 生成代码
- **50.3**: `internal/consoleapi/user_handlers.go`（新增）+ `router.go`（bearer 解析扩展）+ `handlers.go:559,625`（actor 覆写）+ `grpcclient.go`（user RPC client）
- **50.4**: 源码注释 redeem（3 处）+ `README.md` + `RELEASE_NOTES.md` + `scripts/console_smoke.sh` + roadmap/adapter

## 5. Behavior Contract
- migration 0020 幂等（IF NOT EXISTS）
- UserStore：create（id/token 唯一）/ get-by-token / list
- proto add-only：新 UserService + 3 RPC，0 现有字段号变更
- bearer：token ∈ users.token → 注入 userID context；token = 旧 shared-token → 不注入（向后兼容）；空 token → trusted-network（byte-equivalent）
- actor 覆写：context 有 userID → actor=userID；否则 trusted-network 回落 `"console-api"`
- gate 仍软门（ADR-013）

## 6. AC（Phase 级）
- [x] AC1: ADR-051 + migration 0020 + UserStore 单测 — verified by task-50.1 §6
- [x] AC2: proto add-only UserService + Rust gRPC 单测 — verified by task-50.2 §6
- [x] AC3: Go REST 注册 + bearer verified identity + actor 覆写 + byte-equivalent — verified by task-50.3 §6
- [x] AC4: SPEC-DEFER redeemed + README/RELEASE_NOTES + smoke + closeout — verified by task-50.4 §6
- [x] AC5: ADR-014 D1-D5（第四十二次激活）全通过

## 8. Risks
- **byte-equivalent 破坏**：trusted-network + 旧 shared token 必须字节等价（AC 强制）——破坏则 BLOCK
- **token 明文存**：local-first 妨协；hash 存延后 `[SPEC-DEFER:phase-future.token-hash-storage]`
- **actor 覆写时机**：bearer 验证后、gRPC 前；context 注入唯一安全路径
- **proto FROZEN**：UserService 必须 add-only，不动现有字段号（ADR-015）

## 9. Phase smoke gate
task-50.4 跑：cargo test -p contextforge-core + go test ./internal/consoleapi/ ./internal/cli/ + console_smoke（加 user 注册 step）+ spec_drift_lint。
