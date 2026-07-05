# Task `50.1`: `adr-migration-userstore — ADR-051 + SQLite users migration + Rust UserStore`

**Status**: Done
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
- [x] **AC1**: migration 0020 幂等（IF NOT EXISTS）+ users 表 schema 正确 — verified by **TEST-50.1.1**
- [x] **AC2**: UserStore create/get-by-token/list 单测 PASS + dup-token 返错 — verified by **TEST-50.1.2**
- [x] **AC3**: ADR-051 Accepted（per-D ratify：D1 per-user token / D2 SQLite local-first / D3 Go 覆写 / D4 不做 RBAC-Postgres-OIDC）— verified by ADR file Status

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-50.1.1 | migration 0020 幂等 + schema | cargo test | Done |
| TEST-50.1.2 | UserStore CRUD + dup-token err | cargo test | Done |

## 9. Verification
```bash
cargo test -p contextforge-core -run identity
cargo test -p contextforge-core # no-regression
```

## 10. Completion Notes
**Status**: Done
1. **完成日期**：2026-07-05
2. **改动文件**：
   - core/migrations/0020_users.sql（新增，users 表 + token index）
   - core/src/identity/mod.rs + store.rs（新增，SqliteUserStore）
   - core/src/lib.rs（+pub mod identity）
   - docs/decisions/adr-051-identity-foundation.md（新增）
   - docs/decisions/README.md（+ADR-051 行 + count 49→50）
3. **commit 列表**：
   - <GREEN> feat(identity): task-50.1 ADR-051 + SQLite users migration + Rust UserStore
4. **§9 Verification 结果**：
   - cargo test: 2 passed / 0 failed（test_50_1_1 migration 幂等 + test_50_1_2 CRUD/dup）+ full lib no-regression ✅
   - ADR-051 Accepted（D1-D5 per-task ratify）
5. **剩余风险**：token 明文存（local-first 妥协；hash 存 `[SPEC-DEFER:phase-future.token-hash-storage]` Phase 51+）
6. **下游影响**：task-50.2（proto UserService 调 UserStore）/ task-50.3（Go bearer 匹配 users.token）
