# Phase 52 · rbac-roles-permissions (B1 第三步：3-role 扇平 RBAC)

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 是 **v2.0 multi-user 第三步**——完整 AuthZ 层：3-role 扇平（admin/member/viewer）+ workspace_members 多 user 共享 + admin-gate destructive/user-management。关闭 `[SPEC-DEFER:phase-future.rbac-roles-permissions]`。
>
> Phase 50/51 交付了 verified identity + workspace ownership（单 owner_id）。Phase 52 让多 user 共享 workspace 并按 role 控制 destructive ops + user 管理。
>
> **方向锚点**（用户已定）：3-role 扇平 / workspace_members 表 / admin-gate destructive + user mgmt。
>
> **入读顺序**：本 phase spec → 4 task spec → 源码锚点（`core/migrations/0021_workspaces_owner.sql` owner_id / `core/src/identity/store.rs` UserStore / `core/src/workspace/mod.rs` WorkspaceStore / `internal/consoleapi/router.go` confirmMiddleware + verifiedUserIDKey / `internal/consoleapi/handlers.go` 4 destructive endpoints）→ ADR-052（D4 deferral）/ ADR-053（本 phase 新增）。

## 1. 阶段目标

引入完整 AuthZ 层。具体：

1. **3-role 扇平**：admin（全权）/ member（读写）/ viewer（只读）—— 固定 role 枚举
2. **workspace_members 表**：migration 0022 + MembershipStore
3. **admin-gate**：4 destructive endpoints + /v1/users 限 admin role
4. **byte-equivalent**：trusted-network → 所有 user 视为 admin（v2.0.0-alpha.2 行为不变）

**具体 exit criteria（§6 AC）**：
1. task-52.1: ADR-053 + migration 0022 + MembershipStore
2. task-52.2: proto add-only MembershipService + Rust handler
3. task-52.3: Go REST membership + roleMiddleware + admin-gate
4. task-52.4: redeem SPEC-DEFER + workspace create auto-admin + closeout
5. ADR-014 D1-D5（第四十四次激活）

**版本号**：v2.0.0-alpha.3（task-52.4 closeout 定）。

## 2. 业务价值

**完整 AuthZ**：Phase 51 让 verified user 有了 workspace 边界（owner/non-owner），但所有 owner 权限相同——没有 destructive op 保护，没有 read-only user。Phase 52 让 admin 能管理 workspace 成员 + 限制 destructive ops + 邀请 viewer 只读访问。这是 multi-user 团队部署的必要前提。

### 52.1 ADR-053 + migration + MembershipStore（🟢 Rust）
### 52.2 proto + Rust gRPC（🟢 proto add-only + Rust）
### 52.3 Go REST + roleMiddleware + admin-gate（🟢 Go，最复杂）
### 52.4 closeout（🟢 文档 + auto-admin）

**不在本 phase 范围**（诚实 OOS）：workspace sharing/transfer UI（Phase 53+ `[SPEC-DEFER:phase-future.workspace-sharing-transfer]`）/ 全 28 路由 member/viewer 细粒度 gate（Phase 52.x——本 phase 聚焦 admin-gate 最高价值风险面）/ custom role + permission table（不做，3-role 扇平足够）/ OAuth/OIDC（Phase 53+）/ token hash（Phase 51+）。

## 3. 涉及模块
- **52.1**: `core/migrations/0022_workspace_members.sql`（新增）+ `core/src/membership/{mod,store}.rs`（新增）+ `docs/decisions/adr-053-rbac-roles-permissions.md`（新增）
- **52.2**: `proto/.../console_data_plane.proto`（add-only MembershipService）+ `core/src/data_plane/membership.rs`（新增）+ buf generate
- **52.3**: `internal/consoleapi/membership_handlers.go`（新增）+ `router.go`（roleMiddleware + admin-gate）+ `handlers.go`（admin-gate 检查）+ `grpcclient.go`（MembershipService client）
- **52.4**: workspace create auto-admin + redeem marker + README/RELEASE_NOTES + roadmap/adapter

## 5. Behavior Contract
- migration 0022 幂等（CREATE TABLE IF NOT EXISTS）
- MembershipStore：add_member(workspace_id, user_id, role) / remove_member / list_members(workspace_id) / get_role(workspace_id, user_id) → Option<role>
- role ∈ {admin, member, viewer}（CHECK constraint reject invalid）
- admin-gate：destructive + user mgmt 端点 → context role == admin 或 trusted-network（视为 admin）
- byte-equivalent：trusted-network（空 token）→ admin role（跳过 gate）

## 6. AC（Phase 级）
- [ ] AC1: ADR-053 + migration 0022 + MembershipStore 单测 — verified by task-52.1 §6
- [ ] AC2: proto add-only MembershipService + Rust gRPC 单测 — verified by task-52.2 §6
- [ ] AC3: Go REST membership + roleMiddleware + admin-gate + byte-equivalent — verified by task-52.3 §6
- [ ] AC4: workspace create auto-admin + SPEC-DEFER redeemed + closeout — verified by task-52.4 §6
- [ ] AC5: ADR-014 D1-D5（第四十四次激活）全通过

## 8. Risks
- **roleMiddleware 性能**：每请求 GetMyRole gRPC → 可加 cache（Phase 52.x）
- **byte-equiv**：trusted-network → admin（AC 强制验证）
- **enforcement 不全**：admin-gate 仅 6 端点；member/viewer 细粒度 gate `[SPEC-DEFER:phase-future.full-rpc-ownership-enforcement]` 留 Phase 52.x
- **owner_id + membership 冗余**：并存（owner_id fallback，membership primary；向后兼容）

## 9. Phase smoke gate
task-52.4 跑：cargo test -p contextforge-core + go test ./internal/consoleapi/ ./internal/cli/ + spec_drift_lint。
