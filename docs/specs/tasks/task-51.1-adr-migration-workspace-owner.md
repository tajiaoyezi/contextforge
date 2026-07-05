# Task `51.1`: `adr-migration-workspace-owner — ADR-052 + migration 0021 + WorkspaceStore owner 支持`

**Status**: Done
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 51 (workspace-isolation)
**Dependencies**: v2.0.0-alpha（已 ship，Phase 50 verified identity）/ ADR-016（D1 Rust sole SQLite owner）/ ADR-014（第四十三次激活）

## 1. Background
workspace 表无 owner 列。Phase 51 加 owner_id 列 + WorkspaceStore owner 支持，为 per-user access control 提供存储基础。

## 2. Goal
(1) ADR-052：owner 列模型 / 访问控制边界 / byte-equiv fallback / 不做 ACL。
(2) migration 0021：`ALTER TABLE workspaces ADD COLUMN owner_id TEXT`（guarded PRAGMA，同 0017）。
(3) WorkspaceStore：create_owned / list_owned(userID) / get_if_owned(id, userID)。

## 3. Scope
- 新增 `docs/decisions/adr-052-workspace-ownership.md`
- 新增 `core/migrations/0021_workspaces_owner.sql`
- 改 `core/src/workspace/mod.rs`：Workspace struct 加 owner_id；WorkspaceCreate 加 owner_id；WorkspaceStore trait 加 create_owned/list_owned/get_if_owned；open() 加 ensure_owner_column()（PRAGMA guard，同 ensure_pin_actor_columns pattern）；现有 create/list/get 保留（byte-equiv）
- 单测：create_owned → list_owned → get_if_owned round-trip；NULL owner backfill；unowned visible

## 6. AC
- [x] **AC1**: migration 0021 guarded 幂等（PRAGMA table_info 检查）+ owner_id 列 schema — verified by **TEST-51.1.1**
- [x] **AC2**: WorkspaceStore create_owned/list_owned/get_if_owned 单测 PASS（owned filter / NULL owner visible / 非 owner 不可见）— verified by **TEST-51.1.2**
- [x] **AC3**: ADR-052 Accepted（D1 owner 列 / D2 访问边界 / D3 byte-equiv / D4 不做 ACL）— verified by ADR file Status

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-51.1.1 | migration 0021 guarded 幂等 + schema | cargo test | PASS |
| TEST-51.1.2 | WorkspaceStore owner CRUD + NULL/unowned filter | cargo test | PASS |

## 9. Verification
```bash
cargo test -p contextforge-core workspace
cargo test -p contextforge-core # no-regression
```

## 10. Completion Notes
**Status**: Done
1. **完成日期**：2026-07-03
2. **改动文件**：
   - 新增 `core/migrations/0021_workspaces_owner.sql`（guarded ALTER TABLE ADD COLUMN owner_id TEXT）
   - 新增 `docs/decisions/adr-052-workspace-ownership.md`（D1 owner 列 / D2 访问边界 / D3 byte-equiv / D4 不做 ACL）
   - 改 `core/src/workspace/mod.rs`：MIGRATION_OWNER_SQL const；Workspace/WorkspaceCreate 加 `owner_id: Option<String>`；`ensure_owner_column()`（PRAGMA guard，同 ensure_pin_actor_columns pattern）；WorkspaceStore trait 加 create_owned/list_owned/get_if_owned；SqliteWorkspaceStore impl 三方法；现有 list/get/update_config SELECT 加 owner_id 列；create() 返回 owner_id: None（byte-equiv）
   - 改 `core/src/data_plane/workspace.rs`：gRPC create handler 显式 `owner_id: None`（byte-equiv，proto 透传留 task-51.2）
   - 改 `core/src/jobs/index_session_backend.rs` / `core/src/data_plane/job.rs` / `core/src/data_plane/search.rs`：测试 fixture 加 `..Default::default()`（owner_id None）
   - 改 `docs/decisions/README.md`：新增 "Identity & Access" 分类 + ADR-052 行；总数 50 → 51
3. **commit 列表**：见 git log（本 task 单 commit）
4. **§9 Verification 结果**：
   - `cargo test -p contextforge-core --lib workspace` → 20 passed; 0 failed（含新增 test_51_1_1 / test_51_1_2）
   - `cargo test -p contextforge-core --lib`（no-regression）→ 241 passed; 0 failed
5. **剩余风险**：
   - owner_id 非 FK（user 删除时悬空，留 Phase 52 RBAC 处理）
   - NULL = unowned 对任何 verified user 可见（过渡期；多租户需 Phase 52 收敛）
   - create_owned/create 双路径并存（短期 API surface 增大；proto/Go task-51.2/51.3 接入后收敛）
6. **下游影响**：task-51.2（proto WorkspaceService 调 create_owned/list_owned/get_if_owned）/ task-51.3（Go handler 传 verified owner → WorkspaceCreate.owner_id）
