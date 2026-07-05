# Task `50.1`: `adr-migration-userstore — ADR-051 + SQLite users migration + Rust UserStore`

**Status**: Ready
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 50 (identity-foundation)
**Dependencies**: v1.1.0（已 ship）/ ADR-016（D1 Rust sole SQLite owner）/ ADR-004（local-first）/ ADR-014（第四十二次激活）

## 1. Background
当前零身份层。Phase 50 第一步：新增 SQLite users 表 + Rust UserStore，为 bearer 解析 verified identity 提供存储基础。ADR-051 记录身份模型决策。

## 2. Goal
(1) ADR-051：per-user token / SQLite / Go 覆写 / 不做 RBAC/Postgres/OIDC。
(2) migration 0020：`users(id TEXT PK, name TEXT, token TEXT UNIQUE NOT NULL, created_at_unix INTEGER)` + token index。
(3) Rust UserStore：create / get-by-token / list。

## 3. Scope
- 新增 `docs/decisions/adr-051-identity-foundation.md`
- 新增 `core/migrations/0020_users.sql`
- 新增 `core/src/identity/mod.rs` + `core/src/identity/store.rs`
- UserStore 单测（create → get-by-token round-trip / list / dup-token err）
- 改 `core/src/lib.rs`（pub mod identity）

## 6. AC
- [ ] **AC1**: migration 0020 幂等（IF NOT EXISTS）+ users 表 schema 正确 — verified by **TEST-50.1.1**
- [ ] **AC2**: UserStore create/get-by-token/list 单测 PASS + dup-token 返错 — verified by **TEST-50.1.2**
- [ ] **AC3**: ADR-051 Accepted（per-D ratify：D1 per-user token / D2 SQLite local-first / D3 Go 覆写 / D4 不做 RBAC-Postgres-OIDC）— verified by ADR file Status

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-50.1.1 | migration 0020 幂等 + schema | cargo test | Not Started |
| TEST-50.1.2 | UserStore CRUD + dup-token err | cargo test | Not Started |

## 9. Verification
```bash
cargo test -p contextforge-core -run identity
cargo test -p contextforge-core # no-regression
```

## 10. Completion Notes
**Status**: Ready
1. **完成日期**：<TBD-after-impl>
2. **改动文件**：<TBD-after-impl>
3. **commit 列表**：<TBD-after-impl>
4. **§9 Verification 结果**：<TBD-after-impl>
5. **剩余风险**：<TBD-after-impl>
6. **下游影响**：task-50.2（proto UserService 调 UserStore）/ task-50.3（Go bearer 匹配 users.token）
