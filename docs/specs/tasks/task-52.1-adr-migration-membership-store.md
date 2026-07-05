# Task `52.1`: `adr-migration-membership-store — ADR-053 + migration 0022 + MembershipStore`

**Status**: Done
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 52 (rbac-roles-permissions)
**Dependencies**: v2.0.0-alpha.2（已 ship）/ ADR-016（D1 Rust sole SQLite owner）/ ADR-014（第四十四次激活）

## 1. Background
无 membership 表。Phase 52 加 workspace_members + MembershipStore，为 role-based access control 提供存储基础。

## 2. Goal
(1) ADR-053：3-role 扇平 / workspace_members / admin-gate 范围 / byte-equiv。
(2) migration 0022：`workspace_members(workspace_id, user_id, role CHECK IN admin/member/viewer, created_at_unix)` PK(workspace_id, user_id)。
(3) MembershipStore：add_member / remove_member / list_members / get_role。

## 3. Scope
- 新增 `docs/decisions/adr-053-rbac-roles-permissions.md`
- 新增 `core/migrations/0022_workspace_members.sql`
- 新增 `core/src/membership/{mod,store}.rs`：MembershipStore（CRUD + role CHECK）
- 改 `core/src/lib.rs`（pub mod membership）
- 单测：add → list → get_role round-trip；role CHECK reject invalid；remove

## 6. AC
- [x] **AC1**: migration 0022 幂等 + workspace_members schema（PK + CHECK constraint）— verified by **TEST-52.1.1**
- [x] **AC2**: MembershipStore add/remove/list/get_role 单测 PASS + invalid role CHECK reject — verified by **TEST-52.1.2**
- [x] **AC3**: ADR-053 Accepted（D1 3-role / D2 workspace_members / D3 admin-gate / D4 byte-equiv）— verified by ADR file Status

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-52.1.1 | migration 幂等 + schema + CHECK | cargo test | PASS |
| TEST-52.1.2 | MembershipStore CRUD + role CHECK | cargo test | PASS |

## 9. Verification
```bash
cargo test -p contextforge-core --lib membership
cargo test -p contextforge-core --lib # no-regression
```

## 10. Completion Notes
**Status**: Done
1. **完成日期**：2026-07-03
2. **改动文件**：
   - 新增 `core/migrations/0022_workspace_members.sql`（workspace_members 表 + idx_workspace_members_user）
   - 新增 `core/src/membership/mod.rs`（模块声明 + re-export）
   - 新增 `core/src/membership/store.rs`（Role enum + Member struct + MembershipStoreError + SqliteMembershipStore + 2 单测）
   - 改 `core/src/lib.rs`（`pub mod membership;`）
   - 新增 `docs/decisions/adr-053-rbac-roles-permissions.md`（D1 3-role / D2 workspace_members / D3 admin-gate / D4 byte-equiv）
   - 改 `docs/decisions/README.md`（ADR-053 入 Identity & Access + count 51→52）
3. **commit 列表**：`feat(rbac): task-52.1 ADR-053 + migration 0022 workspace_members + MembershipStore`
4. **§9 Verification 结果**：
   - `cargo test -p contextforge-core --lib membership` → 2 passed (test_52_1_1, test_52_1_2)
   - `cargo test -p contextforge-core --lib` → 244 passed; 0 failed（242 基线 + 2 新增，无回归）
   - `cargo clippy -p contextforge-core --tests -- -D warnings` → clean
5. **剩余风险**：
   - role 存 TEXT（非 FK），user/workspace 删除时 membership 行悬空（[SPEC-DEFER:phase-future.membership-orphan-cleanup] 留 Phase 52.x 一致性清理）
   - owner_id（ADR-052）与 membership 并存冗余，task-52.4 auto-admin 收敛一致性
   - admin-gate 范围（D3）仅定义，落地在 task-52.3 Go roleMiddleware
6. **下游影响**：task-52.2（proto MembershipService 调 store）/ task-52.3（Go roleMiddleware 调 get_role）/ task-52.4（workspace create auto-admin）
