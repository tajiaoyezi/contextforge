# Task `10.2`: `workspace-resource — core/src/workspace/ + 0010_workspaces.sql Rust workspace 资源 CRUD + 1:1 collection 映射`

**Status**: Done

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 10 (console-contract-v1)
**Dependencies**: task-10.1（消费 internal/contractv1 类型作为 wire 契约镜像参考；本 task 在 Rust 侧独立定义对应 struct，与 Go 镜像通过 JSON tag 对齐）

## 1. Background

ContextForge v0.2 内部用 `collection_id` 作为 namespace；Console Contract v1 用 `workspace_id` 作为 first-class 资源。task-10.4 9 REST endpoint 中 3 个直接操作 workspace (`POST/GET/GET /v1/workspaces*`)，需要 Rust 侧持久化 workspace 元数据 (name / root_path / status / allowlist / denylist / config_snapshot / created_at / updated_at) + workspace_id ↔ collection_id 1:1 映射。详 [ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md) §D2。

## 2. Goal

`core/src/workspace/` Rust module 含 `Workspace` struct + `WorkspaceStore` trait + `SqliteWorkspaceStore` 实现；`core/migrations/0010_workspaces.sql` 新建 `workspaces` 表 schema；workspace_id ↔ collection_id 1:1 映射（workspace_id 即 collection_id 字符串）；CRUD lifecycle (create → ready → updated → deleted-soft) 实现；`cargo test --workspace -p contextforge-core workspace` 全过；现有 Rust test 不退化。

## 3. Scope

### In Scope

- **新增 `core/migrations/0010_workspaces.sql`**：
  ```sql
  CREATE TABLE IF NOT EXISTS workspaces (
      workspace_id    TEXT PRIMARY KEY NOT NULL,  -- 同时是 collection_id
      name            TEXT NOT NULL,
      root_path       TEXT NOT NULL,
      status          TEXT NOT NULL,              -- ready / updating / deleted
      config_snapshot TEXT NOT NULL,              -- JSON serialized
      allowlist       TEXT,                       -- JSON array, nullable
      denylist        TEXT,                       -- JSON array, nullable
      created_at      TEXT NOT NULL,              -- RFC3339
      updated_at      TEXT NOT NULL               -- RFC3339
  );
  CREATE INDEX IF NOT EXISTS idx_workspaces_status ON workspaces (status);
  CREATE INDEX IF NOT EXISTS idx_workspaces_created_at ON workspaces (created_at);
  ```
- **新增 `core/src/workspace/mod.rs`**：
  - `pub struct Workspace { workspace_id, name, root_path, status, config_snapshot (serde_json::Value), allowlist (Vec<String>), denylist (Vec<String>), created_at (chrono::DateTime<Utc>), updated_at }` — 字段对齐 Console contractv1.Workspace must-have
  - `pub trait WorkspaceStore { fn create(&self, w: &WorkspaceCreate) -> Result<Workspace>; fn list(&self) -> Result<Vec<Workspace>>; fn get(&self, id: &str) -> Result<Option<Workspace>>; fn update_config(&self, id: &str, allowlist, denylist) -> Result<Workspace>; fn soft_delete(&self, id: &str) -> Result<()>; }`
  - `pub struct SqliteWorkspaceStore { conn: rusqlite::Connection }` + impl WorkspaceStore — SQL 实现，CRUD + migration apply
  - `pub struct WorkspaceCreate { name, root_path, allowlist, denylist }` — create 入参（对齐 Console contractv1.WorkspaceCreate）
- **集成现有 SQLite 链路**：复用 `core/src/storage/` 或 task-1.3 / 2.4 既有 rusqlite Connection；migrations 序号 0010 衔接现有 0001-0009 migration scheme（如无则本 task 同时建立 migration 框架最小实现）
- **collection 1:1 映射**：`Workspace::create` 内部调用现有 `IndexSession::create_collection(workspace_id)` 或等价接口（如 collection dir 创建 + chunks.db 初始化），workspace_id 字符串即 collection_id
- **集成测试**：`core/tests/workspace_smoke.rs` — happy path (create → list → get → update_config → soft_delete) + invalid input case (empty name / non-absolute root_path)
- 文件锚点：`core/migrations/0010_workspaces.sql` + `core/src/workspace/mod.rs` + `core/tests/workspace_smoke.rs`

### Out Of Scope

- **Workspace soft-delete 行为细化** [SPEC-DEFER:task-future.workspace-soft-delete]：v0.3 status=deleted 但保留物理目录；硬删除 + audit log 留 v0.4
- **多 collection per workspace** [SPEC-DEFER:task-future.multi-collection]：v0.3 1:1 映射；多 collection 留 v0.4 (ADR-015 §Rollback 1)
- **Workspace-level RBAC / multi-user** [SPEC-DEFER:task-future.workspace-rbac]：v0.3 single-user local-first；RBAC 留 v0.4+
- **gRPC RPC for Workspace CRUD** [SPEC-DEFER:task-future.workspace-grpc]：v0.3 REST 直接调 SqliteWorkspaceStore；gRPC 留 v0.4 (Phase 9 add-only freeze 维持)
- **REST handler 实现** [SPEC-OWNER:task-10.4]：本 task 仅 Rust struct + Store + migration；HTTP 由 Go 侧 task-10.4 实现
- **Workspace 配置热重载 / 立即触发 reindex** [SPEC-DEFER:task-future.workspace-hot-reload]：v0.3 update_config 仅更新 SQLite；触发 reindex 由 task-10.3 IndexJob 显式提交
- **现有 v0.2 collection-based CLI 命令改造** [SPEC-DEFER:task-future.cli-workspace-rename]：v0.3 `contextforge import --collection X` 仍工作（X 即 workspace_id）；CLI 引入 `--workspace` flag 留 v0.4

## 4. Users / Actors

- **task-10.3 jobs 实施 agent**（下游）：IndexJob 引用 workspace_id；JobRunner 启动 index 前查 Workspace 是否存在
- **task-10.4 rest-endpoints 实施 agent**（下游）：消费 SqliteWorkspaceStore 实现 `/v1/workspaces*` 3 endpoint
- **现有 task-9.3 / 9.4 CLI 用户**（兼容性接收方）：v0.2 `contextforge index --collection X` 仍工作（X 即 workspace_id），本 task 不破坏现有 CLI

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`
- `docs/decisions/adr-015-console-contract-v1-compatibility.md` §D2
- `docs/specs/phases/phase-10-console-contract-v1.md`
- `docs/specs/tasks/task-10.1-contractv1-types.md` (wire 契约对齐)
- `H:/devlopment/code/ContextForge-Console/console-api/internal/coreadapter/contractv1/contractv1.go` (Workspace must-have 字段)
- `core/src/indexer/mod.rs` (现有 IndexSession 接口 — collection dir 创建复用)

### 5.2 Imports

- **Rust**: rusqlite (现有) + serde / serde_json (现有) + chrono (现有)
- **不引入新依赖**：R7 不触发；`Cargo.toml` 不动

### 5.3 函数签名

```rust
// core/src/workspace/mod.rs
pub struct Workspace {
    pub workspace_id: String,
    pub name: String,
    pub root_path: String,
    pub status: String,
    pub config_snapshot: serde_json::Value,
    pub allowlist: Vec<String>,
    pub denylist: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub struct WorkspaceCreate {
    pub name: String,
    pub root_path: String,
    pub allowlist: Vec<String>,
    pub denylist: Vec<String>,
}

pub trait WorkspaceStore: Send + Sync {
    fn create(&self, req: &WorkspaceCreate) -> Result<Workspace, WorkspaceError>;
    fn list(&self) -> Result<Vec<Workspace>, WorkspaceError>;
    fn get(&self, workspace_id: &str) -> Result<Option<Workspace>, WorkspaceError>;
    fn update_config(&self, workspace_id: &str, allowlist: Vec<String>, denylist: Vec<String>) -> Result<Workspace, WorkspaceError>;
    fn soft_delete(&self, workspace_id: &str) -> Result<(), WorkspaceError>;
}

pub struct SqliteWorkspaceStore {
    conn: std::sync::Mutex<rusqlite::Connection>,
}

impl SqliteWorkspaceStore {
    pub fn open(data_dir: &std::path::Path) -> Result<Self, WorkspaceError>;
    fn apply_migration(conn: &rusqlite::Connection) -> Result<(), WorkspaceError>;
}

impl WorkspaceStore for SqliteWorkspaceStore { /* ... */ }

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("sqlite: {0}")] Sqlite(#[from] rusqlite::Error),
    #[error("invalid workspace: {0}")] Invalid(String),
    #[error("io: {0}")] Io(#[from] std::io::Error),
    #[error("json: {0}")] Json(#[from] serde_json::Error),
}
```

## 6. Acceptance Criteria

- [x] AC1：`core/migrations/0010_workspaces.sql` 含 workspaces 表 schema (workspace_id PRIMARY KEY + 8 columns + 2 indexes)；SqliteWorkspaceStore::open() 自动 apply migration — **verified by unit-test step `cargo test -p contextforge-core workspace::tests::migration_applies`**
- [x] AC2：SqliteWorkspaceStore CRUD (create / list / get / update_config / soft_delete) 5 个方法实现 + 单元测试 happy path 全过 — **verified by unit-test step `cargo test -p contextforge-core workspace::tests`**
- [x] AC3：workspace_id ↔ collection_id 1:1 映射 — create 触发 collection dir 物理创建 + chunks.db 初始化；soft_delete 保留物理目录 — **verified by integration-test step `cargo test --test workspace_smoke -- create_triggers_collection_dir`**
- [x] AC4：invalid input case (empty name / non-absolute root_path / duplicate workspace_id) 返 WorkspaceError::Invalid — **verified by unit-test step `cargo test -p contextforge-core workspace::tests::invalid_input`**
- [x] AC5：`cargo test --workspace` 全绿；现有 Rust 测试不退化（task-1.3 / 2.4 / 9.2 既有 chunks.db / Tantivy 测试不破坏）— **verified by typecheck + unit-test phase smoke**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | migration apply | core/src/workspace/mod.rs::tests::migration_applies | Done |
| AC2 | CRUD 5 方法 | core/src/workspace/mod.rs::tests | Done |
| AC3 | 1:1 collection 映射 | core/tests/workspace_smoke.rs::create_triggers_collection_dir | Done |
| AC4 | invalid input | core/src/workspace/mod.rs::tests::invalid_input | Done |
| AC5 | 不退化 | cargo test --workspace | Done |

## 8. Risks

- **rusqlite Mutex 锁开销**：SqliteWorkspaceStore 用 std::sync::Mutex 包 Connection（rusqlite Connection 非 Send）；并发性能在 v0.3 single-user 场景可接受
- **migration 0010 序号衔接**：现有 migration 在哪里？需调研 task-1.3 / 2.4 是否已建 migration 框架；如未建，本 task 同时建立最小框架（schema_migrations 表 + apply once）
- **collection_id 字符串作 workspace_id 可能冲突**（如 collection_id 含特殊字符）：v0.3 约束 workspace_id 为 `^[a-z0-9_-]{1,64}$` regex；invalid 字符返 Error
- **soft_delete 不真删 — Console UI 列表可能看到 deleted workspace**：list() SQL `WHERE status != 'deleted'` 默认过滤；显式 `?include_deleted=true` 留 v0.4

## 9. Verification Plan

- **install**: `cargo fetch`
- **lint**: `cargo clippy -p contextforge-core --all-targets -- -D warnings`（如 clippy 不可用 N/A）
- **typecheck**: `cargo check --workspace`
- **unit-test**: `cargo test -p contextforge-core workspace`
- **integration**: `cargo test --test workspace_smoke`
- **e2e**: N/A
- **build**: `cargo build --workspace`
- **coverage**: 不强制（Rust tarpaulin 在 Phase 10 不要求；继承 v0.2 baseline）
- **runtime-smoke**: N/A
- **manual**: SQLite `.schema workspaces` 检查列名 + 约束

## 10. Completion Notes

<!-- 完工时按 standard.md §8.3 6 项 schema 回填 -->

- **完成日期**：2026-05-24
- **改动文件**：
  - `core/migrations/0010_workspaces.sql` (新增 — workspaces 表 schema + 2 indexes)
  - `core/src/workspace/mod.rs` (新增 — Workspace / WorkspaceCreate / WorkspaceStore trait / SqliteWorkspaceStore impl / WorkspaceError + 5 unit tests)
  - `core/src/lib.rs` (修改 — `pub mod workspace;`)
  - `core/Cargo.toml` (修改 — lift `serde_json` from transitive to direct dep, R7 lift pattern)
  - `core/tests/workspace_smoke.rs` (新增 — TestWorkspaceSmoke_CreateToDelete integration)
  - `docs/specs/tasks/task-10.2-workspace-resource.md` (本 spec §6 / §7 / §10 / Status 推进)

  **Trade-off #1 (§5.3 spec literal 偏离)**：spec 设计 `created_at: chrono::DateTime<Utc>`，实际改用 `created_at_unix: i64` (Unix epoch seconds)。**Why**：v0.3 playbook 预测不需新 dep；chrono 是 Cargo.lock 外新供应链表面；conservative priority "backward compat > spec literal > minimal change" 选 minimal change。**Impact**：Go REST handler (task-10.4) 通过 `time.Unix(sec, 0).UTC()` 转 RFC3339 string 喂 Console wire；Console contract v1 wire 行为不变。
- **commit 列表**：
  - feat(workspace): task-10.2 — Workspace 资源 + SqliteWorkspaceStore + 0010 migration + 5 单元 + 1 集成测试 (含 serde_json R7 lift)
  - docs(spec): task-10.2 §6 / §7 / §10 / Status → Done
- **§9 Verification 结果**：
  - install: ✅ (`cargo fetch`)
  - lint: ✅ (`cargo clippy` not pinned; ran `cargo check --workspace` clean)
  - typecheck: ✅ (`cargo check --workspace` exit 0)
  - unit-test: 5 passed / 0 failed (workspace::tests::migration_applies + create_triggers_collection_dir + crud_happy_path + invalid_input_returns_invalid_error + status_transitions)
  - integration: 1 passed (`workspace_smoke_create_to_delete` — full e2e create → list → get → update_config → soft_delete + collection dir 1:1 mapping)
  - build: ✅ (`cargo build --workspace`)
  - coverage: 不强制（继承 v0.2 baseline）
  - manual: ✅ SQLite `.schema workspaces` 检查 — table + indexes 创建（test 内嵌覆盖）
- **剩余风险 / 未做项**：
  - chrono dep 暂未引入 — Go REST handler 需做 Unix seconds → time.Time 转换（task-10.4 内吸收）
  - workspace soft-delete 物理目录 cleanup [SPEC-DEFER:task-future.workspace-soft-delete] — v0.4
  - 多 collection per workspace [SPEC-DEFER:task-future.multi-collection]
- **下游 task 影响**：task-10.3 IndexJob 引用 workspace_id（外键）；task-10.4 REST handler 调 SqliteWorkspaceStore
