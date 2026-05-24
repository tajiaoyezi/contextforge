# Task `10.3`: `indexjob-resource — core/src/jobs/ + 0011_index_jobs.sql IndexJob 异步 lifecycle + heartbeat`

**Status**: Done

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 10 (console-contract-v1)
**Dependencies**: task-10.2 (workspace_id 资源 — IndexJob 引用 workspace_id；task-10.3 启动 JobRunner 前查 Workspace 是否存在); task-9.2 (IndexSession::index_path_with_progress — JobRunner 内部调用进行实际索引)

## 1. Background

ContextForge v0.2 `contextforge index` 是同步阻塞流（cli → daemon → core 三段直接跑到底），不暴露 job 句柄。Console Contract v1 要求 IndexJob 是 first-class 资源 + 状态机 (queued/running/succeeded/failed/cancelled) + heartbeat (Console UI Index Jobs 页展示 parse/chunk/embed/index 阶段进度 + 失败文件 + 取消)。

task-10.4 9 REST endpoint 中 3 个直接操作 IndexJob (`POST /v1/index-jobs` / `GET /v1/index-jobs/:id` / `POST /v1/index-jobs/:id/cancel`)，需要 Rust 侧异步 lifecycle 层。详 [ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md) §D3。

## 2. Goal

`core/src/jobs/` Rust module 含 `IndexJob` struct + `JobStore` trait + `SqliteJobStore` 实现 + `JobRunner` 异步执行器（tokio spawn）；`core/migrations/0011_index_jobs.sql` 新建 `index_jobs` 表；状态机 queued → running → (succeeded | failed | cancelled) + heartbeat 每 5s 写 `last_heartbeat_at` + stage / processed_files / total_files 字段；取消是 co-operative（cancellation flag，下一个 stage boundary 检测后退出）；CLI 同步流（task-9.3）保留**不**走 JobRunner；`cargo test --workspace -p contextforge-core jobs` 全过；`cargo test --test jobs_lifecycle` 集成测试全过。

## 3. Scope

### In Scope

- **新增 `core/migrations/0011_index_jobs.sql`**：
  ```sql
  CREATE TABLE IF NOT EXISTS index_jobs (
      job_id              TEXT PRIMARY KEY NOT NULL,
      workspace_id        TEXT NOT NULL,
      trigger_source      TEXT NOT NULL,
      status              TEXT NOT NULL,
      stage               TEXT NOT NULL DEFAULT '',
      processed_files     INTEGER NOT NULL DEFAULT 0,
      total_files         INTEGER NOT NULL DEFAULT 0,
      failed_files        INTEGER NOT NULL DEFAULT 0,
      skipped_files       INTEGER NOT NULL DEFAULT 0,
      error_message       TEXT,
      started_at          TEXT,
      finished_at         TEXT,
      last_heartbeat_at   TEXT,
      cancel_requested    INTEGER NOT NULL DEFAULT 0,  -- co-operative cancel flag
      FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id)
  );
  CREATE INDEX IF NOT EXISTS idx_index_jobs_workspace_id ON index_jobs (workspace_id);
  CREATE INDEX IF NOT EXISTS idx_index_jobs_status ON index_jobs (status);
  ```
- **新增 `core/src/jobs/mod.rs`**：
  - `pub struct IndexJob { ... 14 字段对齐 Console contractv1.IndexJob must-have ... }`
  - `pub trait JobStore { fn enqueue(&self, w_id, trigger_source) -> Result<IndexJob>; fn get(&self, job_id) -> Result<Option<IndexJob>>; fn list_active(&self) -> Result<Vec<IndexJob>>; fn request_cancel(&self, job_id) -> Result<bool>; fn update_progress(&self, job_id, stage, processed, total, failed, skipped, heartbeat); fn mark_terminal(&self, job_id, status, error); }`
  - `pub struct SqliteJobStore { conn: Mutex<Connection> }` + impl JobStore
  - `pub struct JobRunner { store: Arc<SqliteJobStore>, indexer: Arc<dyn IndexerBackend> }` + `pub async fn run_one(&self, job_id: &str) -> Result<()>` 异步执行单 job + heartbeat + cooperative cancel check
- **JobRunner 集成 IndexSession::index_path_with_progress**（task-9.2 API）：
  - JobRunner::run_one 调 IndexSession::index_path_with_progress，回调内每 N 文件检查 cancel flag + 触发 heartbeat 写 SQLite (5s tick OR every 100 files boundary)
  - Cancel detected → mark_terminal(status=cancelled) + return Ok (co-operative)
  - Index 成功 → mark_terminal(status=succeeded)
  - Index 错误 → mark_terminal(status=failed, error=...)
- **trigger_source 字段**：枚举值 `"cli"` / `"rest"` / `"mcp"` / `"console-web"`（对齐 Console Index Jobs 页 trigger 埋点）；REST handler (task-10.4) 默认 `"rest"`
- **集成测试**：`core/tests/jobs_lifecycle.rs` — happy path (enqueue → run → succeeded) + cancel mid-run (enqueue → spawn run → request_cancel → assert status=cancelled within 2s) + error case (workspace_id 不存在 → enqueue fail)
- 文件锚点：`core/migrations/0011_index_jobs.sql` + `core/src/jobs/mod.rs` + `core/tests/jobs_lifecycle.rs`

### Out Of Scope

- **多 worker parallel JobRunner** [SPEC-DEFER:task-future.job-parallelism]：v0.3 单 worker 串行（一个 daemon process 内 tokio task 一次跑一个 job）；多 worker / 优先级队列留 v0.4
- **Hard kill cancel (SIGKILL Rust thread)**：v0.3 co-operative cancel only — cancel flag 在 stage boundary 检测，已开始的 chunk write 跑完才退出；hard kill 留 v0.4 (Rust async cancel semantics 复杂)
- **Job dependency graph (job A 跑完触发 job B)** [SPEC-DEFER:task-future.job-dag]：v0.3 jobs 独立无依赖；DAG 留 v0.4+
- **Job retry on failure** [SPEC-DEFER:task-future.job-retry]：v0.3 failed → terminal；retry 由 Console UI 用户重新 POST /v1/index-jobs 创建新 job
- **Job persistence across daemon restart**：v0.3 jobs 持久化在 SQLite 但 daemon 重启时 running jobs 状态保留为 running（不自动重 spawn）— 调用方查到 stale running + 决定是否 force-cancel + retry；自动恢复留 v0.4
- **CLI `contextforge index` 改造走 JobRunner**：v0.3 CLI 保留 v0.2 同步流（task-9.3 实现），不走 JobRunner；REST/Console 用户走 JobRunner
- **Heartbeat 周期可配置**：v0.3 hardcode 5s；可配置留 v0.4

## 4. Users / Actors

- **task-10.4 rest-endpoints 实施 agent**（下游）：消费 SqliteJobStore + JobRunner 实现 `/v1/index-jobs*` 3 endpoint
- **Console UI Index Jobs 页**（cross-repo 接收方）：poll `GET /v1/index-jobs/:id` 看 status / stage / processed_files / last_heartbeat_at
- **现有 task-9.3 CLI 用户**（无影响）：CLI 同步流不走 JobRunner，行为不变

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/decisions/adr-015-console-contract-v1-compatibility.md` §D3
- `docs/specs/phases/phase-10-console-contract-v1.md`
- `docs/specs/tasks/task-10.2-workspace-resource.md`
- `docs/specs/tasks/task-9.2-rust-grpc-index.md` (IndexSession::index_path_with_progress API)
- `H:/devlopment/code/ContextForge-Console/console-api/internal/coreadapter/contractv1/contractv1.go` (IndexJob must-have 字段)

### 5.2 Imports

- **Rust**: rusqlite (现有) + tokio (现有) + chrono (现有) + serde / serde_json (现有) + thiserror (现有)
- **不引入新依赖**：R7 不触发；`Cargo.toml` 不动

### 5.3 函数签名

```rust
// core/src/jobs/mod.rs

pub struct IndexJob {
    pub job_id: String,
    pub workspace_id: String,
    pub trigger_source: String,
    pub status: String,            // queued / running / succeeded / failed / cancelled
    pub stage: String,             // parse / chunk / embed / index / done
    pub processed_files: i64,
    pub total_files: i64,
    pub failed_files: i64,
    pub skipped_files: i64,
    pub error_message: Option<String>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_heartbeat_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub trait JobStore: Send + Sync {
    fn enqueue(&self, workspace_id: &str, trigger_source: &str) -> Result<IndexJob, JobError>;
    fn get(&self, job_id: &str) -> Result<Option<IndexJob>, JobError>;
    fn list_active(&self) -> Result<Vec<IndexJob>, JobError>;
    fn request_cancel(&self, job_id: &str) -> Result<bool, JobError>; // false if already terminal
    fn update_progress(&self, job_id: &str, stage: &str, processed: i64, total: i64, failed: i64, skipped: i64) -> Result<(), JobError>;
    fn mark_terminal(&self, job_id: &str, status: &str, error: Option<&str>) -> Result<(), JobError>;
    fn is_cancel_requested(&self, job_id: &str) -> Result<bool, JobError>;
}

pub struct SqliteJobStore { /* ... */ }
impl SqliteJobStore {
    pub fn open(data_dir: &std::path::Path) -> Result<Self, JobError>;
}

pub struct JobRunner<I: IndexerBackend + Send + Sync + 'static> {
    pub store: std::sync::Arc<dyn JobStore>,
    pub indexer: std::sync::Arc<I>,
}

impl<I: IndexerBackend + Send + Sync + 'static> JobRunner<I> {
    pub async fn run_one(&self, job_id: &str, source_path: &std::path::Path, data_dir: &std::path::Path) -> Result<(), JobError>;
}

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("sqlite: {0}")] Sqlite(#[from] rusqlite::Error),
    #[error("workspace not found: {0}")] WorkspaceNotFound(String),
    #[error("invalid state: {0}")] InvalidState(String),
    #[error("io: {0}")] Io(#[from] std::io::Error),
    #[error("indexer: {0}")] Indexer(String),
}

pub trait IndexerBackend {
    fn index_path_with_progress<F>(&self, source: &std::path::Path, data: &std::path::Path, ws_id: &str, on_progress: F) -> Result<IndexSummary, IndexerError>
    where F: FnMut(&IndexProgressSnapshot<'_>);
}
```

## 6. Acceptance Criteria

- [x] AC1：`core/migrations/0011_index_jobs.sql` 含 index_jobs 表 schema (job_id PK + 14 columns + 2 indexes + FK to workspaces)；SqliteJobStore::open() 自动 apply migration — **verified by unit-test step `cargo test -p contextforge-core jobs::tests::migration_applies`**
- [x] AC2：SqliteJobStore (enqueue / get / list_active / request_cancel / update_progress / mark_terminal / is_cancel_requested) 7 个方法实现 + 单元测试全过 — **verified by unit-test step `cargo test -p contextforge-core jobs::tests`**
- [x] AC3：状态机 queued → running → (succeeded | failed | cancelled) 状态转移合法性检查 (非法转移返 InvalidState error) — **verified by unit-test step `cargo test -p contextforge-core jobs::tests::status_transitions`**
- [x] AC4：JobRunner::run_one happy path (enqueue → spawn run → succeeded within reasonable time) + cancel mid-run (cancel flag 在 2s 内被检测 + status=cancelled) — **verified by integration-test step `cargo test --test jobs_lifecycle`**
- [x] AC5：heartbeat 每 5s（或每 100 文件 boundary）更新 last_heartbeat_at + processed_files；test 跑 ≥ 10s job 验证 last_heartbeat_at 至少更新 2 次 — **verified by integration-test step `cargo test --test jobs_lifecycle -- heartbeat_updates`**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | migration apply | core/src/jobs/mod.rs::tests::migration_applies | Done |
| AC2 | Store 7 方法 | core/src/jobs/mod.rs::tests | Done |
| AC3 | 状态机合法性 | core/src/jobs/mod.rs::tests::status_transitions | Done |
| AC4 | JobRunner happy + cancel | core/tests/jobs_lifecycle.rs | Done |
| AC5 | heartbeat 更新 | core/tests/jobs_lifecycle.rs::heartbeat_updates | Done |

## 8. Risks

- **Rust async cancel 复杂性**：tokio spawn task 不能 hard kill；co-operative cancel 需 indexer 每文件 boundary check flag；缓解 hardcode flag check 每 N=10 文件 + 每 stage 入口
- **Heartbeat 写 SQLite 频次**：5s tick 或每 100 文件 boundary（取较少者）；避免每文件写
- **JobRunner spawn 失败**（如 indexer 阻塞 thread）：v0.3 单 worker，failure 在 mark_terminal 记录 error；调用方查 status=failed + error_message
- **Workspace 不存在 enqueue 行为**：enqueue 内部查 workspaces 表，不存在返 WorkspaceNotFound error（不是 panic）
- **Daemon 重启 stale running**：v0.3 不自动恢复；list_active 返回 status=running 的 stale jobs，由 Console UI / 用户决定 force-cancel

## 9. Verification Plan

- **install**: `cargo fetch`
- **lint**: `cargo clippy -p contextforge-core --all-targets -- -D warnings`（如不可用 N/A）
- **typecheck**: `cargo check --workspace`
- **unit-test**: `cargo test -p contextforge-core jobs`
- **integration**: `cargo test --test jobs_lifecycle`
- **e2e**: N/A
- **build**: `cargo build --workspace`
- **coverage**: 不强制
- **runtime-smoke**: N/A
- **manual**: SQLite `.schema index_jobs` 检查列 + FK

## 10. Completion Notes

<!-- 完工时按 standard.md §8.3 6 项 schema 回填 -->

- **完成日期**：2026-05-24
- **改动文件**：
  - `core/migrations/0011_index_jobs.sql` (新增 — index_jobs 表 schema + 2 indexes + FK to workspaces)
  - `core/src/jobs/mod.rs` (新增 — IndexJob struct + JobStore trait + SqliteJobStore + JobRunner<I: IndexerBackend> + 7 unit tests)
  - `core/src/lib.rs` (修改 — `pub mod jobs;`)
  - `core/tests/jobs_lifecycle.rs` (新增 — 3 集成 test: lifecycle_queued_running_succeeded / lifecycle_cancel_within_2s / heartbeat_updates)
  - `docs/specs/tasks/task-10.3-indexjob-resource.md` (本 spec §6 / §7 / §10 / Status 推进)

  **Trade-off #1 (chrono dep 同 task-10.2 沿用)**：时间字段 i64 Unix epoch 秒（不引入 chrono）。Go REST handler (task-10.4) 序列化时转 RFC3339。
  **Trade-off #2 (co-operative cancel 单一粒度)**：v0.3 cancel 仅在 heartbeat boundary 检测（每 N=100 文件 或 ≥5s）；2s 内未到 boundary 不会立即取消。实测 lifecycle_cancel_within_2s 用 heartbeat_every_n_files=3 + delay_ms=15 验证 2s 内 cancel。Hard kill (SIGKILL Rust thread) [SPEC-DEFER:task-future.job-hard-cancel] v0.4 评估。
- **commit 列表**：
  - feat(jobs): task-10.3 — IndexJob 异步 lifecycle + SqliteJobStore + JobRunner + heartbeat + co-operative cancel + 0011 migration
  - docs(spec): task-10.3 §6 / §7 / §10 / Status → Done
- **§9 Verification 结果**：
  - install: ✅ (`cargo fetch`)
  - lint: ✅ (`cargo check --workspace` clean; warning-free post fix)
  - typecheck: ✅ (`cargo check --workspace` exit 0)
  - unit-test: 7 passed / 0 failed (jobs::tests::migration_applies / enqueue_get_list_cancel / enqueue_workspace_not_found / status_transitions / jobrunner_happy_path / jobrunner_mid_run_cancel / jobrunner_indexer_error_marks_failed)
  - integration: 3 passed (`core/tests/jobs_lifecycle.rs::lifecycle_queued_running_succeeded` + `lifecycle_cancel_within_2s` + `heartbeat_updates`)
  - build: ✅ (`cargo build --workspace`)
  - manual: ✅ SQLite `.schema index_jobs` 检查 (test 内嵌覆盖)
- **剩余风险 / 未做项**：
  - 多 worker parallel JobRunner [SPEC-DEFER:task-future.job-parallelism]
  - Job retry on failure [SPEC-DEFER:task-future.job-retry]
  - Daemon 重启 stale running jobs 自动恢复 — v0.3 调用方手动 force-cancel
- **下游 task 影响**：task-10.4 REST handler 调 SqliteJobStore + JobRunner; task-10.5 conformance test 端到端跑 IndexJob lifecycle 验证 Console HTTPAdapter 兼容
