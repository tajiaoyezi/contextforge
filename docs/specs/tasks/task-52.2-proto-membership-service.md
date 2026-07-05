# Task `52.2`: `proto-membership-service — proto add-only MembershipService + Rust handler`

**Status**: Ready
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 52 (rbac-roles-permissions)
**Dependencies**: task-52.1（MembershipStore）/ ADR-015（proto FROZEN，add-only）

## 1. Background
MembershipStore（task-52.1）是 Rust 内部。Go 需通过 gRPC 访问。

## 2. Goal
proto add-only 新增 `MembershipService`（AddMember/RemoveMember/ListMembers/GetMyRole）+ Rust handler。0 现有字段号变更（ADR-015 FROZEN）。

## 3. Scope
- 改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`：add-only MembershipService + messages（Member/AddMemberRequest/RemoveMemberRequest/ListMembersRequest/ListMembersResponse/GetMyRoleRequest/GetMyRoleResponse）
- buf generate 重生 pb.go
- 新增 `core/src/data_plane/membership.rs`：handler 调 MembershipStore
- 改 `core/src/data_plane/mod.rs`（pub mod membership + register_services）
- 单测：add → list → get_role round-trip via gRPC

## 6. AC
- [ ] **AC1**: proto add-only（0 现有字段号变更；新 MembershipService + 4 RPC）— verified by **TEST-52.2.1**
- [ ] **AC2**: Rust MembershipService gRPC 单测 PASS — verified by **TEST-52.2.2**

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-52.2.1 | proto add-only | git diff | Not Started |
| TEST-52.2.2 | Rust gRPC round-trip | cargo test | Not Started |

## 9. Verification
```bash
cargo test -p contextforge-core --lib data_plane::membership
cargo build -p contextforge-core
buf generate proto
git diff origin/master -- proto/ | grep -E '^-' | grep -v '^---'
```

## 10. Completion Notes
**Status**: Ready
1. **完成日期**：<TBD-after-impl>
2. **改动文件**：<TBD-after-impl>
3. **commit 列表**：<TBD-after-impl>
4. **§9 Verification 结果**：<TBD-after-impl>
5. **剩余风险**：<TBD-after-impl>
6. **下游影响**：task-52.3（Go grpcclient 调 MembershipService）
