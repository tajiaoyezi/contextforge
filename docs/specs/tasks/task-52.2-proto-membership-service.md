# Task `52.2`: `proto-membership-service — proto add-only MembershipService + Rust handler`

**Status**: Done
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
- [x] **AC1**: proto add-only（0 现有字段号变更；新 MembershipService + 4 RPC）— verified by **TEST-52.2.1**
- [x] **AC2**: Rust MembershipService gRPC 单测 PASS — verified by **TEST-52.2.2**

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-52.2.1 | proto add-only | git diff | Pass — 0 deletion lines vs origin/master |
| TEST-52.2.2 | Rust gRPC round-trip | cargo test | Pass — 5 tests (data_plane::membership) green |

## 9. Verification
```bash
cargo test -p contextforge-core --lib data_plane::membership
cargo build -p contextforge-core
buf generate proto
git diff origin/master -- proto/ | grep -E '^-' | grep -v '^---'
```

## 10. Completion Notes
**Status**: Done
1. **完成日期**：2026-07-03
2. **改动文件**：
   - `proto/contextforge/console_data_plane/v1/console_data_plane.proto` — add-only append: `Member` / `AddMemberRequest` / `RemoveMemberRequest` / `RemoveMemberResponse` / `ListMembersRequest` / `ListMembersResponse` / `GetMyRoleRequest` / `GetMyRoleResponse` + `service MembershipService { AddMember / RemoveMember / ListMembers / GetMyRole }`（ADR-015 FROZEN；0 existing field numbers changed）
   - `proto/contextforge/console_data_plane/v1/console_data_plane.pb.go` / `console_data_plane_grpc.pb.go` — regenerated via `buf generate proto`
   - `core/src/data_plane/membership.rs` — NEW Rust handler（`MembershipServer` + `with_store` lazy-open + 4 RPCs；store err → tonic Status：Duplicate→AlreadyExists、Invalid→InvalidArgument、other→Internal）
   - `core/src/data_plane/mod.rs` — `pub mod membership;` + import + register in `register_services` + `server_with_services`
3. **commit 列表**：见 git log（本 commit）
4. **§9 Verification 结果**：
   - `cargo test -p contextforge-core --lib data_plane::membership` → 5 passed / 0 failed
   - `cargo test -p contextforge-core --lib`（no-regression）→ 249 passed / 0 failed
   - `cargo clippy -p contextforge-core --tests -- -D warnings` → clean（`#[allow(clippy::result_large_err)]` on `with_store` + `impl MembershipService`）
   - `go build ./...` → exit 0
   - proto add-only check：`git diff origin/master -- proto/...proto | grep '^-' | grep -v '^---'` → 无删除行（add-only confirmed）
5. **剩余风险**：
   - 无 RBAC 强制（enforcement 留给 task-52.3 Go `roleMiddleware` + ADR-053 D3 admin-gate）
   - 仍无 workspace/user FK（跨 DB；app-level join；与 task-52.1 一致）
   - `data_dir` 空 → `failed_precondition`（与 UserServer 一致，task-11.1 baseline / 单测）
6. **下游影响**：task-52.3（Go grpcclient 调 MembershipService / roleMiddleware 走 `GetMyRole` admin-gate）；task-52.4（auto-admin on workspace create）
