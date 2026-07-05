# Task `51.2`: `proto-workspace-owner — proto add-only owner 字段 + Rust WorkspaceService handler`

**Status**: Done
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
- [x] **AC1**: proto add-only（0 现有字段号变更；Workspace + CreateWorkspaceRequest 加 owner_id）— verified by **TEST-51.2.1**
- [x] **AC2**: Rust WorkspaceService gRPC 单测 PASS（create with owner → list_owned → get_if_owned round-trip）— verified by **TEST-51.2.2**

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-51.2.1 | proto add-only（owner_id 字段，0 现有变更） | git diff | Pass |
| TEST-51.2.2 | Rust WorkspaceService owner round-trip | cargo test | Pass |

## 9. Verification
```bash
cargo test -p contextforge-core data_plane::workspace
cargo build -p contextforge-core
buf generate proto
git diff origin/master -- proto/ | grep -E '^-' | grep -v '^---' # 应空
```

## 10. Completion Notes
**Status**: Done
1. **完成日期**：2026-07-03
2. **改动文件**：
   - `proto/contextforge/console_data_plane/v1/console_data_plane.proto` — add-only: `Workspace.owner_id=10`、`CreateWorkspaceRequest.owner_id=6`、`ListOwnedWorkspacesRequest{owner_id=1}`、`GetIfOwnedWorkspaceRequest{workspace_id=1, owner_id=2}`、`WorkspaceService.ListOwned`/`GetIfOwned` 2 new RPC（ADR-015 FROZEN：0 现有字段号变更）
   - `proto/contextforge/console_data_plane/v1/console_data_plane.pb.go` + `_grpc.pb.go` — `buf generate` 重生（Go 新 OwnerId 字段 + ListOwned/GetIfOwned client/server/handler）
   - `core/src/data_plane/workspace.rs` — `Create` 读 `req.owner_id`（empty→None）走 `create_owned`；新增 `list_owned`/`get_if_owned` handler（None→`not_found`）；`workspace_to_pb` map `owner_id`；新增 `test_51_2_2` round-trip 单测；既有 4 个测试加 `owner_id: String::new()`
   - `core/src/data_plane/mod.rs` — `test_proto_field_snake_case_consistency` 加 `owner_id` 字段
   - `core/tests/data_plane_integration.rs` + `core/tests/indexjob_real_runner.rs` — 既有 CreateWorkspaceRequest 字面量加 `owner_id: String::new()`（unowned byte-equivalent）
3. **commit 列表**：`feat(workspace): task-51.2 proto add-only owner field + Rust WorkspaceService handler`
4. **§9 Verification 结果**：
   - `cargo test -p contextforge-core --lib data_plane::workspace` → 6 passed / 0 failed（含 test_51_2_2）
   - `cargo test -p contextforge-core --lib` → 242 passed / 0 failed（no-regression）
   - `cargo test -p contextforge-core --test data_plane_integration` → 5 passed / 0 failed
   - `cargo clippy -p contextforge-core --tests -- -D warnings` → clean（无 warning/error）
   - `go build ./...` → exit 0（pb.go 重生编译通过）
   - `git diff origin/master -- proto/ | grep '^-' | grep -v '^---'` → empty（TEST-51.2.1 add-only 验证通过）
5. **剩余风险**：
   - GetIfOwned 非 owner / 不存在都映射 `not_found`（不在 wire 上区分，防 enumeration；与 Get 一致）。
   - `workspace_to_pb` owner_id `Option<String> → String`：None（unowned/legacy）序列化为 proto3 默认空串，round-trip-safe（空串 ⇄ None 在 wire 边界）。
6. **下游影响**：task-51.3（Go grpcclient 调 owner RPC — ListOwned/GetIfOwned 已在 pb.go 就绪）。
