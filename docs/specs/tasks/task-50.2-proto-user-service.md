# Task `50.2`: `proto-user-service — proto add-only UserService + Rust gRPC handler`

**Status**: Done
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 50 (identity-foundation)
**Dependencies**: task-50.1（UserStore）/ ADR-015（proto FROZEN，add-only）/ ADR-016（gRPC bridge）

## 1. Background
UserStore（task-50.1）是 Rust 内部。Go 需通过 gRPC 访问（ADR-016 D1：Go 不能直接查 SQLite）。本 task 新增 proto UserService + Rust gRPC handler。

## 2. Goal
proto add-only 新增 `UserService`（CreateUser / GetUserByToken / ListUsers）+ Rust handler 调 UserStore。0 现有字段号变更（ADR-015 FROZEN）。

## 3. Scope
- 改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`：add-only UserService + messages（User / CreateUserRequest / GetUserByTokenRequest / ListUsersRequest / ListUsersResponse）+ service UserService {}
- 新增 `core/src/data_plane/user.rs`：UserService handler（调 UserStore）
- 改 `core/src/data_plane/mod.rs`（pub mod user）+ server wiring（register UserService）
- tonic 生成代码（cargo build 自动）
- 单测：create → get-by-token round-trip via gRPC

## 6. AC
- [x] **AC1**: proto add-only（0 现有字段号变更；新 UserService + 3 RPC）— verified by **TEST-50.2.1**（git diff proto 仅 add-only）
- [x] **AC2**: Rust UserService gRPC 单测 PASS（create → get-by-token round-trip）— verified by **TEST-50.2.2**

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-50.2.1 | proto add-only（UserService + 3 RPC，0 现有字段变更） | git diff | Done |
| TEST-50.2.2 | Rust UserService gRPC round-trip | cargo test | Done |

## 9. Verification
```bash
cargo test -p contextforge-core -run user
cargo build -p contextforge-core # tonic 生成 + 编译
# proto add-only 验证
git diff origin/master -- proto/ | grep -E '^-' | grep -v '^---' # 应为空（无删除）
```

## 10. Completion Notes
**Status**: Done
1. **完成日期**：2026-07-05
2. **改动文件**：
   - proto/contextforge/console_data_plane/v1/console_data_plane.proto（add-only UserService + User/CreateUserRequest/GetUserByTokenRequest/ListUsersRequest/ListUsersResponse messages）
   - proto/contextforge/console_data_plane/v1/console_data_plane.pb.go + console_data_plane_grpc.pb.go（buf generate 重新生成）
   - core/src/data_plane/user.rs（新增，UserServer handler + 3 RPC）
   - core/src/data_plane/mod.rs（+pub mod user + register_services + server_with_services 注册 UserService）
3. **commit 列表**：
   - <GREEN> feat(identity): task-50.2 proto add-only UserService + Rust gRPC handler
4. **§9 Verification 结果**：
   - proto add-only：0 现有字段号变更（git diff 仅 + UserService/messages）✅
   - cargo test: 3 passed / 0 failed（test_50_2_2 roundtrip + test_50_2_2b dup-already-exists + test_50_2_2c empty-data-dir-failed-precondition）
   - full lib no-regression: 239 passed / 0 failed ✅
   - go build ./... ✅（pb.go 重新生成编译通过）
5. **剩余风险**：UserServer 每调用开 store（cheap IF NOT EXISTS）；低 QPS admin 服务可接受；若 future 需高 QPS 可加 Arc<SqliteUserStore> 缓存
6. **下游影响**：task-50.3（Go grpcclient 调 UserService Create/GetByToken/List）
