# Task `52.1`: `adr-migration-membership-store — ADR-053 + migration 0022 + MembershipStore`

**Status**: Ready
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
- [ ] **AC1**: migration 0022 幂等 + workspace_members schema（PK + CHECK constraint）— verified by **TEST-52.1.1**
- [ ] **AC2**: MembershipStore add/remove/list/get_role 单测 PASS + invalid role CHECK reject — verified by **TEST-52.1.2**
- [ ] **AC3**: ADR-053 Accepted（D1 3-role / D2 workspace_members / D3 admin-gate / D4 byte-equiv）— verified by ADR file Status

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-52.1.1 | migration 幂等 + schema + CHECK | cargo test | Not Started |
| TEST-52.1.2 | MembershipStore CRUD + role CHECK | cargo test | Not Started |

## 9. Verification
```bash
cargo test -p contextforge-core --lib membership
cargo test -p contextforge-core --lib # no-regression
```

## 10. Completion Notes
**Status**: Ready
1. **完成日期**：<TBD-after-impl>
2. **改动文件**：<TBD-after-impl>
3. **commit 列表**：<TBD-after-impl>
4. **§9 Verification 结果**：<TBD-after-impl>
5. **剩余风险**：<TBD-after-impl>
6. **下游影响**：task-52.2（proto MembershipService 调 store）/ task-52.3（Go roleMiddleware 调 get_role）/ task-52.4（workspace create auto-admin）
