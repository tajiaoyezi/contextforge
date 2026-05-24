# Task `11.3`: `indexjob-real-runner-wiring — JobService.Enqueue 真触发 JobRunner.spawn_blocking(IndexSession::index_path_with_progress)`

**Status**: Ready

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 11 (console-real-data-plane)
**Dependencies**: task-11.1 (`JobServer` 占位 + SqliteJobStore 接通) + task-10.3 (`JobRunner` 框架 + `SqliteJobStore` 已建) + task-2.4 (`IndexSession::index_path_with_progress` API) + task-11.2 (Go gRPC dispatch + sentinel error mapping 不变) + [ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) D1/D3

## 1. Background

task-11.1 落地的 `JobServer` 在 `Enqueue` 仅占位写 `status=queued` 后调 v0.3 的 task-10.3 现有 stub 行为（200ms tick 模拟 status 推进）—— Console UI 看到 status 推进 succeeded 但 Rust 没真索引；这是 [ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md) task-10.4 §10 Trade-off #2 显式记录的 v0.3 conscious gap。本 task 是 [ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) 在 Rust 数据面 wiring 层的 resolve 项 —— 把 `JobRunner.spawn_blocking` 真接 `IndexSession::index_path_with_progress`，让 POST `/v1/index-jobs` 真触发 Rust 索引。

复用 Phase 2/4 已建立的 `IndexSession` API（task-2.4 落地，task-9.2 沿用作 Index gRPC 后端）+ 现有 `SqliteJobStore` heartbeat 字段（task-10.3 `0011_index_jobs.sql` 已含 `processed_files` + `total_files` + `last_heartbeat_at`）。

核心 wiring 路径：

```
POST /v1/index-jobs              # Console REST
   ↓ Go grpcclient (task-11.2)
JobService.Enqueue (Rust)        # task-11.1 占位 stub
   ↓ 本 task 替换
JobRunner.spawn_blocking(closure)
   ↓ 闭包内
IndexSession::index_path_with_progress(workspace.root_path, callback)
   ↓ callback 每 100 files 或 5s
SqliteJobStore.update_progress(processed_files, total_files, last_heartbeat_at)
   ↓ JobOutcome 写回
SqliteJobStore.set_terminal(status, ended_at_unix, error_message)
```

`Arc<AtomicBool>` 作 CancelToken：`JobService.Cancel` 写 true；`IndexSession` 闭包每个文件 batch 头 `cancel_token.load(Ordering::Relaxed)` 判 → if true return `JobError::Cancelled`。

Orphan reaper：daemon 启动时 `SqliteJobStore.list_running()` → 每个标 `failed` + `error_message = "job lost: daemon restart"`（v0.4 简单策略；多实例 daemon leader election [SPEC-DEFER:task-future.multi-daemon-leader-election]）。

## 2. Goal

`core/src/data_plane/job.rs::JobServer::Enqueue` 真调 `JobRunner.spawn_blocking(closure)`，闭包内调 `IndexSession::index_path_with_progress(path, callback)`；heartbeat callback 每 100 files 或 5s 触发 `SqliteJobStore.update_progress(...)`；`Arc<AtomicBool>` CancelToken 真停（IndexSession 闭包内每文件 batch 检查）；`JobOutcome` 写回 `status` (succeeded/failed/cancelled) + `ended_at_unix` + `error_message`；orphan reaper 在 daemon `serve` 启动早期跑（mark all running=failed）；fixture repo `test/fixtures/index-job-real/` ≥5 markdown 文件供集成测试；`cargo test --workspace` 全绿（不破坏 task-10.3 现有 JobRunner 测试 fixture）。

## 3. Scope

### In Scope

- **修改 `core/src/data_plane/job.rs::JobServer::Enqueue`**：
  - 把 task-11.1 占位 stub 替换为真 wiring：
    1. 写 `status=queued` 到 `SqliteJobStore` (复用 task-11.1 既有 path)
    2. 构造 `CancelToken = Arc::new(AtomicBool::new(false))`
    3. clone `Arc<SqliteJobStore>` + `Arc<DataPlaneStores.workspace_store>` + `CancelToken` 进闭包
    4. `runner.spawn_blocking(move || { run_index_job(job_id, workspace_id, stores, cancel_token) })`
    5. 返 `IndexJob { status: "queued", ... }`（同步返；真索引在 spawn_blocking 内异步跑）
  - 新增 `core/src/data_plane/job.rs::run_index_job` 函数：
    1. 读 workspace.root_path from `SqliteWorkspaceStore`
    2. `SqliteJobStore.set_running(job_id, started_at_unix)`
    3. 构造 IndexSession + `index_path_with_progress(root_path, progress_callback)`
    4. `progress_callback(processed: u64, total: u64) -> Result<(), JobError>`：
       - 每 100 files 或 5s 调 `SqliteJobStore.update_progress(...)`
       - 检查 `cancel_token.load(Ordering::Relaxed)` → if true return `Err(JobError::Cancelled)`
       - 同时 emit `ObservabilityEvent { event_type: "indexing.progress", payload: {job_id, processed_files, total_files, ts_unix} }` 到 EventBus broadcast channel（EventBus impl 在 task-11.4，本 task 占位 `if let Some(eb) = &stores.event_bus { eb.send(...) }` 容错路径）
    5. completion：
       - Ok(()) → `SqliteJobStore.set_terminal(job_id, "succeeded", ended_at_unix, None)`
       - Err(JobError::Cancelled) → `set_terminal(job_id, "cancelled", ended_at_unix, Some("user requested cancel"))`
       - Err(JobError::Other(e)) → `set_terminal(job_id, "failed", ended_at_unix, Some(e.to_string()))`
- **修改 `core/src/data_plane/job.rs::JobServer::Cancel`**：
  - 从 in-memory `Arc<DashMap<JobId, CancelToken>>` 取该 job 的 CancelToken → `store(true, Ordering::Relaxed)`
  - 同时 `SqliteJobStore.set_cancel_requested(job_id, true)`（用于 daemon 重启后 reaper 识别 in-progress + cancel-requested → mark cancelled 而非 failed）
  - 返 `CancelJobResponse { ok: true }`；若 job 已 terminal → `tonic::Status::failed_precondition("job already terminal")`
- **新增 `core/src/data_plane/job.rs::orphan_reaper`**：
  - daemon `serve` 启动早期（在 `register_services` 之前）调
  - `SqliteJobStore.list_running()` → 每个调 `set_terminal(job_id, "failed", now_unix, Some("job lost: daemon restart"))`
  - log info `"orphan reaper: marked N jobs as failed"`
- **修改 `core/src/bin/contextforge_core.rs`** (或 daemon serve 子命令入口)：
  - 在 `register_services` 调用前 调 `orphan_reaper(&stores.job_store).await?`
- **新增 fixture `test/fixtures/index-job-real/`**：≥5 markdown 文件 (file1.md ~ file5.md)，每个 ≥10 行非平凡内容（含 word "contextforge" 至少 2 次，供 task-11.4 search 真返回测试用）；fixture **必须真有内容**（非 `echo > file` 占位）—— 与 §自决规则 R9 一致
- **集成测试 `core/tests/indexjob_real_runner.rs`**：
  - `test_enqueue_starts_running`：POST 后 ≤1s 内 status queued → running（spawn_blocking 启动 + set_running 已写）
  - `test_job_succeeds_real_index`：fixture repo (≥5 files) → 等 status=succeeded（≤30s）→ processed_files == total_files == 5 + 无 error_message
  - `test_cancel_truly_stops`：fixture repo (≥20 files for cancel window) → Enqueue → 等 200ms → Cancel → ≤5s 内 status=cancelled + cancel_token.load == true + processed_files < total_files
  - `test_orphan_job_reaper`：直接 `SqliteJobStore.insert(status=running, ...)` 模拟 orphan job + 调 `orphan_reaper(&store)` → 该 job status=failed + error_message="job lost: daemon restart"
  - `test_heartbeat_persists_every_100_files_or_5s`：fixture 200 files → 跑 5s + 观察 SqliteJobStore.processed_files 真更新
- **单元测试**：
  - `test_cancel_token_arc_atomic_visibility` (两 thread + Arc<AtomicBool>)
  - `test_job_error_to_terminal_mapping`
- **不破坏 task-10.3 现有 JobRunner 测试**：v0.3 task-10.3 `core/src/jobs/*_test.rs` 仍 PASS（行为兼容：v0.3 stub callback path 现在 fallback 到 in-memory cancel_token + spawn_blocking 仍然存在；测试若依赖 stub 状态机推进周期 → 改为依赖 set_terminal 真写）
- **不引入新 SQLite migration**：复用 task-10.3 `0011_index_jobs.sql` 既有字段 (`processed_files` / `total_files` / `last_heartbeat_at` / `cancel_requested` / `error_message` / `ended_at_unix`)；若某字段不存在（task-10.3 历史 scope 不全）→ 本 task 单独 `0012_index_jobs_progress_extension.sql` add-only migration（[ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) D5 Rust 单 owner）
- **文件锚点**：`core/src/data_plane/job.rs` (Enqueue/Cancel/run_index_job/orphan_reaper) + `core/src/bin/contextforge_core.rs` (daemon serve 早期 orphan reaper) + `test/fixtures/index-job-real/file{1..5}.md` + `core/tests/indexjob_real_runner.rs`
- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **task-11.4 SearchService 真接 retriever** [SPEC-OWNER:task-11.4]：本 task 仅 JobRunner ↔ IndexSession wiring；search 仍占位 task-11.1 empty results
- **task-11.4 EventsService 真接 EventBus** [SPEC-OWNER:task-11.4]：本 task emit progress event 路径已 wire 到 `Option<EventBus>`，但 EventBus impl 在 task-11.4；本 task fixture 路径 `if let Some(eb) = &stores.event_bus` 容错 None
- **多实例 daemon leader election** [SPEC-DEFER:task-future.multi-daemon-leader-election]：v0.4 单 daemon；orphan reaper 假设 single-writer
- **真 hard kill cancel（非 co-operative）** [SPEC-DEFER:task-future.hard-cancel]：v0.4 仅 co-operative（IndexSession 自身需循环检查 cancel_token，对长跑 single-file parse 不可中断）
- **新增 SQLite migration 0012_***：默认不引入；若 task-10.3 0011 字段不全 → fallback 加 add-only 0012；本 task scope 内允许
- **gRPC streaming JobService.Stream 全实现** [SPEC-OWNER:task-11.4]：本 task 仍 keepalive only

## 4. Users / Actors

- **Console UI 用户**（end-user）：触发 POST `/v1/index-jobs` 后期望真 indexing + progress reflection + cancel 真停
- **task-11.4 实施 agent**（下游 / 同 phase）：依赖本 task progress event emission 路径 wire 到 EventBus，task-11.4 落 EventBus impl 后真流
- **运维**：通过 orphan reaper 让 daemon 重启后状态干净（无 forever-running job）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md` §D1 / §D3
- `docs/specs/phases/phase-11-console-real-data-plane.md`
- `docs/specs/tasks/task-11.1-rust-data-plane-grpc-services.md` (JobServer 占位现状)
- `docs/specs/tasks/task-10.3-indexjob-resource.md` (SqliteJobStore + JobRunner 框架 + 0011 schema)
- `docs/specs/tasks/task-2.4-indexer.md` (IndexSession::index_path_with_progress API)
- `core/src/index.rs` (IndexSession 当前签名)
- `core/src/jobs/mod.rs` (SqliteJobStore + JobRunner 现有方法)

### 5.2 Imports

- **Rust**: 现有 `tokio` (spawn_blocking + sync::atomic) + `rusqlite` + `dashmap`（task-10.3 已引或 stdlib HashMap+RwLock 等价）
- **不引入新依赖**：R7 不触发；`Cargo.toml` 不动（若需 `dashmap` 而当前无 → 单独 chore PR 引入；本 task scope 不动）

### 5.3 函数签名

```rust
// core/src/data_plane/job.rs

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct JobServer {
    stores: Arc<DataPlaneStores>,
    cancel_tokens: Arc<dashmap::DashMap<String, Arc<AtomicBool>>>,  // job_id -> CancelToken
}

#[tonic::async_trait]
impl proto::job_service_server::JobService for JobServer {
    async fn enqueue(
        &self,
        req: tonic::Request<proto::EnqueueJobRequest>,
    ) -> Result<tonic::Response<proto::IndexJob>, tonic::Status> {
        let workspace_id = req.into_inner().workspace_id;
        let job_id = self.stores.job_store.insert_queued(&workspace_id)?;
        let cancel_token = Arc::new(AtomicBool::new(false));
        self.cancel_tokens.insert(job_id.clone(), cancel_token.clone());

        let stores = self.stores.clone();
        self.stores.job_runner.spawn_blocking({
            let job_id = job_id.clone();
            let cancel_token = cancel_token.clone();
            move || run_index_job(job_id, workspace_id, stores, cancel_token)
        });

        let job = self.stores.job_store.get(&job_id)?;
        Ok(tonic::Response::new(workspace_to_proto(job)))
    }

    async fn cancel(
        &self,
        req: tonic::Request<proto::CancelJobRequest>,
    ) -> Result<tonic::Response<proto::CancelJobResponse>, tonic::Status> {
        let job_id = req.into_inner().job_id;
        if let Some(tok) = self.cancel_tokens.get(&job_id) {
            tok.store(true, Ordering::Relaxed);
        }
        self.stores.job_store.set_cancel_requested(&job_id, true)?;
        Ok(tonic::Response::new(proto::CancelJobResponse{ ok: true }))
    }
    // get / stream 同 task-11.1
}

fn run_index_job(
    job_id: String,
    workspace_id: String,
    stores: Arc<DataPlaneStores>,
    cancel_token: Arc<AtomicBool>,
) -> Result<(), JobError> {
    stores.job_store.set_running(&job_id, now_unix())?;
    let workspace = stores.workspace_store.get(&workspace_id)?;
    let root_path = workspace.root_path;

    let session = IndexSession::new(/* ... */);
    let mut last_persist = std::time::Instant::now();
    let mut last_persist_count = 0u64;

    session.index_path_with_progress(&root_path, |processed, total| {
        if cancel_token.load(Ordering::Relaxed) {
            return Err(JobError::Cancelled);
        }
        let elapsed = last_persist.elapsed();
        if processed - last_persist_count >= 100 || elapsed >= std::time::Duration::from_secs(5) {
            stores.job_store.update_progress(&job_id, processed, total, now_unix())?;
            if let Some(eb) = &stores.event_bus {
                let _ = eb.send(ObservabilityEvent::indexing_progress(&job_id, processed, total));
            }
            last_persist = std::time::Instant::now();
            last_persist_count = processed;
        }
        Ok(())
    })?;

    stores.job_store.set_terminal(&job_id, "succeeded", now_unix(), None)?;
    Ok(())
}

pub fn orphan_reaper(store: &SqliteJobStore) -> Result<usize, StoreError> {
    let running = store.list_running()?;
    let count = running.len();
    for job in running {
        let status = if job.cancel_requested { "cancelled" } else { "failed" };
        let msg = if job.cancel_requested {
            "user requested cancel; daemon restarted mid-cancel"
        } else {
            "job lost: daemon restart"
        };
        store.set_terminal(&job.job_id, status, now_unix(), Some(msg.to_string()))?;
    }
    Ok(count)
}
```

## 6. Acceptance Criteria

- [ ] AC1：POST `/v1/index-jobs` → 在 ≤1s 内 status 从 `queued` → `running`（spawn_blocking 真启动 + `set_running` 已写 SQLite） — **verified by integration-test step `cargo test -p contextforge-core --test indexjob_real_runner -- test_enqueue_starts_running`**
- [ ] AC2：fixture `test/fixtures/index-job-real/` (≥5 markdown 文件) 索引完成后 status=succeeded + processed_files == total_files (== 5) + 无 error_message；`IndexSession` 真分块 SQLite chunks 表 +1 row per chunk — **verified by integration-test step `cargo test -p contextforge-core --test indexjob_real_runner -- test_job_succeeds_real_index`**
- [ ] AC3：POST `/v1/index-jobs/<id>/cancel` 后 ≤5s 内 status=`cancelled` + CancelToken.load == true + processed_files < total_files；`indexing.progress` event emission 路径在 `Option<EventBus>=None` 时安全 fallthrough — **verified by integration-test step `cargo test -p contextforge-core --test indexjob_real_runner -- test_cancel_truly_stops`**
- [ ] AC4：daemon 启动早期 `orphan_reaper` 跑过：人工注入 `status=running` 行 + restart → 该 job status=`failed` + error_message="job lost: daemon restart"（cancel_requested=true 的 orphan 则改 status=`cancelled`）— **verified by integration-test step `cargo test -p contextforge-core --test indexjob_real_runner -- test_orphan_job_reaper`**
- [ ] AC5：`cargo test --workspace` 全绿（不破坏 task-10.3 现有 JobRunner 测试）+ `test_heartbeat_persists_every_100_files_or_5s` 验 SqliteJobStore.processed_files 真持续更新 — **verified by typecheck + unit-test phase smoke + integration**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | Enqueue 真 spawn_blocking + queued→running ≤1s | core/src/data_plane/job.rs::Enqueue + test_enqueue_starts_running | Ready |
| AC2 | fixture (≥5 files) → succeeded + processed_files == total_files | run_index_job 闭包 + test_job_succeeds_real_index | Ready |
| AC3 | Cancel 真停 + CancelToken atomic 可见 | JobServer::Cancel + test_cancel_truly_stops | Ready |
| AC4 | orphan reaper marks running → failed/cancelled | job.rs::orphan_reaper + test_orphan_job_reaper | Ready |
| AC5 | 不退化 + heartbeat 真持续更新 | cargo test --workspace + test_heartbeat_persists | Ready |

## 8. Risks

- **`IndexSession::index_path_with_progress` API 不存在 callback variant**：若 task-2.4 / task-9.2 现有 `IndexSession` 只有同步 `index_path()` 无 progress callback → 本 task §10 加 trade-off T1 "扩展 IndexSession API (callback variant)"；扩展为 add-only API（保留 `index_path` 原签名 + 新增 `index_path_with_progress`）
- **spawn_blocking 闭包内 panic**：tokio task crash 但不会更新 status；缓解闭包用 `std::panic::catch_unwind` + 闭包内 Result + `?` 上抛 + spawn_blocking JoinHandle 外层 catch；崩了仍走 `set_terminal(failed, error_message=panic info)`
- **orphan reaper 与新 enqueue race**：reaper 在 `register_services` 前跑（即任何 RPC 收到前），无 new enqueue 风险；缓解 reaper 调用顺序硬编码 daemon serve 入口最早一段
- **Arc<AtomicBool> 跨 spawn_blocking 闭包 visibility**：`AtomicBool` 默认 SeqCst；本 task 用 Ordering::Relaxed 已足（cancel 不是 critical-section 同步原语，仅 cooperative 信号）；缓解单测 `test_cancel_token_arc_atomic_visibility` 用 2 thread + Arc.clone 验
- **heartbeat 写放大**：每 100 files 或 5s 写 SqliteJobStore 引入 IO；100k chunks repo 估算 1000 次写 × ~1ms/写 = 1s overhead per index；task-11.3 §10 trade-off T2 记录 + 默认 5s 触发，不每文件写
- **cancel_token 在 daemon 重启后丢失**：in-memory `Arc<DashMap>` 重启即空；orphan reaper 用 SQLite `cancel_requested` 字段恢复语义（reaper 见 cancel_requested=true → 标 cancelled 而非 failed）

## 9. Verification Plan

- **install**: `cargo fetch`
- **lint**: `cargo fmt --check -- core/src/data_plane/job.rs`
- **typecheck**: `cargo check -p contextforge-core`
- **unit-test**: `cargo test -p contextforge-core --lib data_plane::job` (≥2 单测)
- **integration**: `cargo test -p contextforge-core --test indexjob_real_runner` (5 集成全过)
- **e2e**: 通过 integration 实现
- **build**: `cargo build -p contextforge-core`
- **coverage**: 不强制（核心逻辑 5 集成 + 单测覆盖）
- **runtime-smoke**: 启 daemon + grpcurl `JobService/Enqueue` + grpcurl `JobService/Get` 观察 status 从 queued → running → succeeded
- **manual**: fixture index 跑完后 `sqlite3 <data_dir>/chunks.db "SELECT count(*) FROM chunks"` 真返回 >0 行

## 10. Completion Notes

<!-- 完工时按 standard.md §8.3 6 项 schema 回填 -->

- **完成日期**：<待回填>
- **改动文件**：<待回填>
- **commit 列表**：<待回填>
- **§9 Verification 结果**：<待回填>
- **剩余风险 / 未做项**：
  - SearchService 真接 retriever [SPEC-OWNER:task-11.4]
  - EventsService 真接 EventBus broadcast channel [SPEC-OWNER:task-11.4]
  - 多实例 daemon leader election [SPEC-DEFER:task-future.multi-daemon-leader-election]
  - 真 hard kill cancel [SPEC-DEFER:task-future.hard-cancel]
- **下游 task 影响**：task-11.4 真 EventBus impl 后本 task progress event emission 路径自动激活；Go REST 端通过 `/v1/index-jobs/<id>` 真返回 processed_files / total_files / status
