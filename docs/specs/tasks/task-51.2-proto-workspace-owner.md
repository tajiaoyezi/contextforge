# Task `51.2`: `proto-workspace-owner — proto add-only owner 字段 + Rust WorkspaceService handler`

**Status**: Ready
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 51 (workspace-isolation)
**Dependencies**: task-51.1（WorkspaceStore owner 支持）/ ADR-015（proto FROZEN，add-only）

## 1. Background
WorkspaceStore owner 支持（task-51.1）是 Rust 内部。Go 需通过 gRPC 访问。本 task proto 加 owner 字段 + Rust handler。

## 2. Goal
proto add-only Workspace message 加 `owner_id` 字段；WorkspaceService handler 用 owner_id 调 WorkspaceStore owner 方法。0 现有字段号变更（ADR-015 FROZEN）。

## 3. Scope
- 改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`：Workspace message add-only `string owner_id = N`（下一个 free field number）；CreateWorkspaceRequest add-only `string owner_id = N`
- buf generate 重生 pb.go
- 改 `core/src/data_plane/workspace.rs`：Create handler 读 req.owner_id → create_owned；新增 ListOwned/GetIfOwned handler（或 List/Get 加 owner 参数）
- 单测：create with owner → list_owned → get_if_owned round-trip via gRPC

## 6. AC
- [ ] **AC1**: proto add-only（0 现有字段号变更；Workspace + CreateWorkspaceRequest 加 owner_id）— verified by **TEST-51.2.1**
- [ ] **AC2**: Rust WorkspaceService gRPC 单测 PASS（create with owner → list_owned → get_if_owned round-trip）— verified by **TEST-51.2.2**

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-51.2.1 | proto add-only（owner_id 字段，0 现有变更） | git diff | Not Started |
| TEST-51.2.2 | Rust WorkspaceService owner round-trip | cargo test | Not Started |

## 9. Verification
```bash
cargo test -p contextforge-core data_plane::workspace
cargo build -p contextforge-core
buf generate proto
git diff origin/master -- proto/ | grep -E '^-' | grep -v '^---' # 应空
```

## 10. Completion Notes
**Status**: Ready
1. **完成日期**：<TBD-after-impl>
2. **改动文件**：<TBD-after-impl>
3. **commit 列表**：<TBD-after-impl>
4. **§9 Verification 结果**：<TBD-after-impl>
5. **剩余风险**：<TBD-after-impl>
6. **下游影响**：task-51.3（Go grpcclient 调 owner RPC）
