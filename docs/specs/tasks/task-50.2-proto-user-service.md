# Task `50.2`: `proto-user-service — proto add-only UserService + Rust gRPC handler`

**Status**: Ready
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
- [ ] **AC1**: proto add-only（0 现有字段号变更；新 UserService + 3 RPC）— verified by **TEST-50.2.1**（git diff proto 仅 add-only）
- [ ] **AC2**: Rust UserService gRPC 单测 PASS（create → get-by-token round-trip）— verified by **TEST-50.2.2**

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-50.2.1 | proto add-only（UserService + 3 RPC，0 现有字段变更） | git diff | Not Started |
| TEST-50.2.2 | Rust UserService gRPC round-trip | cargo test | Not Started |

## 9. Verification
```bash
cargo test -p contextforge-core -run user
cargo build -p contextforge-core # tonic 生成 + 编译
# proto add-only 验证
git diff origin/master -- proto/ | grep -E '^-' | grep -v '^---' # 应为空（无删除）
```

## 10. Completion Notes
**Status**: Ready
1. **完成日期**：<TBD-after-impl>
2. **改动文件**：<TBD-after-impl>
3. **commit 列表**：<TBD-after-impl>
4. **§9 Verification 结果**：<TBD-after-impl>
5. **剩余风险**：<TBD-after-impl>
6. **下游影响**：task-50.3（Go grpcclient 调 UserService）
