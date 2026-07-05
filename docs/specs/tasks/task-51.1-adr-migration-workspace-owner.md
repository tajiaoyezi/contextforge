# Task `51.1`: `adr-migration-workspace-owner — ADR-052 + migration 0021 + WorkspaceStore owner 支持`

**Status**: Ready
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
- [ ] **AC1**: migration 0021 guarded 幂等（PRAGMA table_info 检查）+ owner_id 列 schema — verified by **TEST-51.1.1**
- [ ] **AC2**: WorkspaceStore create_owned/list_owned/get_if_owned 单测 PASS（owned filter / NULL owner visible / 非 owner 不可见）— verified by **TEST-51.1.2**
- [ ] **AC3**: ADR-052 Accepted（D1 owner 列 / D2 访问边界 / D3 byte-equiv / D4 不做 ACL）— verified by ADR file Status

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-51.1.1 | migration 0021 guarded 幂等 + schema | cargo test | Not Started |
| TEST-51.1.2 | WorkspaceStore owner CRUD + NULL/unowned filter | cargo test | Not Started |

## 9. Verification
```bash
cargo test -p contextforge-core workspace
cargo test -p contextforge-core # no-regression
```

## 10. Completion Notes
**Status**: Ready
1. **完成日期**：<TBD-after-impl>
2. **改动文件**：<TBD-after-impl>
3. **commit 列表**：<TBD-after-impl>
4. **§9 Verification 结果**：<TBD-after-impl>
5. **剩余风险**：<TBD-after-impl>
6. **下游影响**：task-51.2（proto WorkspaceService 调 owner store）/ task-51.3（Go handler 传 owner）
