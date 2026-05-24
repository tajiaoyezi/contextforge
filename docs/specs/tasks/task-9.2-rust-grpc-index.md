# Task `9.2`: `rust-grpc-index — CoreService::index 流式实现 wrap IndexSession::index_path`

> Status=Draft；主 agent 待用户 §2A Ready review 后推进。本 task 依赖 task-9.1 codegen 产物。

**Status**: Draft

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 9 (cli-pipeline)
**Dependencies**: 9.1 (proto-index-rpc)

## 1. Background

task-9.1 在 proto 层声明 `rpc Index(IndexRequest) returns (stream IndexProgress)` 后，Rust `core/src/server.rs::CoreService` 自动获得 trait method `index` 默认实现（`Status::unimplemented`）。本 task 实施真实业务：把已实现的 `core/src/indexer/mod.rs::IndexSession::index_path` 包成 tonic stream handler，按文件粒度上报 `IndexProgress`，对 Go CLI client（task-9.3）暴露可消费的进度流。

PRD §User Flow 异常流明确要求"索引中断（大仓库 10 万文件）→ 进入长任务模式，进度显示、可中断"。task-8.2 reliability 已落 resume manifest 但 indexer 实测从未跑过 CLI 路径；本 task 是把 indexer 真实接到 CLI 路径的桥梁。

## 2. Goal

`CoreService::index` 在 `core/src/server.rs` 流式实现：消费 `IndexRequest{source_path, data_dir, collection_id}` → 打开 `IndexSession::open(data_dir, collection_id)` → 调 `IndexSession::index_path_with_progress(root, ..., callback)` 按文件粒度 emit `IndexProgress` 到 tonic stream → 最终 emit `done=true` 消息后关流。错误映射到 `tonic::Status`（按 task-6.1 §5.3 已建立的 `RetrieverError` 5 变种 → Status mapping 同款逻辑应用到 `IndexError`）；新增 `IndexSession::index_path_with_progress(..., callback)` 方法（保持原 `index_path` 兼容签名作为 thin wrapper）；`core/tests/phase9_index_smoke.rs` 集成测试覆盖 stream 端到端（含 SCAN_PATH 真扫描临时 fixture）。

## 3. Scope

### In Scope

- **修改 `core/src/indexer/mod.rs`**：
  - 新增 `pub fn index_path_with_progress<F>(&mut self, root: &Path, scan_options: &ScanOptions, policy: &ChunkPolicy, provenance: Vec<Provenance>, on_progress: F) -> Result<IndexStats, IndexError>` 其中 `F: FnMut(&IndexProgressSnapshot)`
  - `IndexProgressSnapshot` 结构体含 `files_processed / files_skipped_denied / files_skipped_redaction / chunks_written / current_file: Option<&Path>`（独立于 proto `IndexProgress` 类型 — 保持 indexer 模块不依赖 proto package）
  - 回调时机：每处理完一个 ScannedFile（含 skip 情况）触发一次；初始时机 + 终态时机由 caller (server.rs) 决定何时 emit proto IndexProgress
  - 原 `pub fn index_path` 改为 `index_path_with_progress(..., |_| {})` 的 thin wrapper，签名不变（向后兼容 task-2.4 现有调用方）
- **修改 `core/src/server.rs`**：
  - 在 trait `ContextService` impl 内加 `async fn index` method（task-9.1 codegen 后必然要求实现）
  - 新增 `type IndexStream = tokio_stream::wrappers::ReceiverStream<Result<IndexProgress, tonic::Status>>;` associated type
  - 实现逻辑：
    1. 校验 `req.source_path` 非空 + 路径存在 → 否则 `Status::invalid_argument`
    2. 解析 `data_dir`（空 fallback `self.data_dir`）+ `collection_id`（空 fallback `"default"`）
    3. 创建 `(tx, rx) = tokio::sync::mpsc::channel(32)`
    4. spawn `tokio::task::spawn_blocking` 跑同步 `IndexSession::index_path_with_progress`，回调内 `tx.blocking_send(Ok(IndexProgress { ... }))`
    5. 完成时（无论成功/失败）send 最后一条 `IndexProgress { done: true, error: <err.to_string() if Err else "">, ... }` 然后 close tx
    6. return `Ok(Response::new(ReceiverStream::new(rx)))`
  - 错误映射：`IndexError::Io(NotFound)` / `IndexError::Sqlite(*)` / `IndexError::Tantivy(*)` / `IndexError::UnsafeRedaction(*)` → 通过 final IndexProgress.error 字段传递（不用 tonic::Status — stream 已建立后用 in-band error 更友好；client 看到 done=true && error != "" 即知失败）
  - 校验阶段错误（path 不存在 / collection_id 非法）→ `Err(Status::invalid_argument(...))` 在 stream 建立前抛出（client 不会收到 stream）
- **新增 `core/tests/phase9_index_smoke.rs`**：
  - `#[test] fn phase_9_index_grpc_end_to_end_smoke()`：建临时 data_dir + 临时 source_path（≥3 .md + 1 .env denied + 1 secret-redacted .yaml） → 起 tonic in-process server (类似 phase4_smoke / phase6_smoke pattern) → 调 client.index() 消费 stream → assert 收到 ≥4 个 IndexProgress 消息（每文件一次 + final done） + final files_processed ≥3 + chunks_written > 0 + error == "" + .env skipped + .yaml redacted；assert SQLite chunks 表 row > 0 + Tantivy 搜索某 fixture marker 命中
  - 错误路径：source_path = "/nonexistent" → client.index() 立即返回 Status::InvalidArgument（流未建立）
- **不动其它**：现有 `health` / `search` method 不修改；其它 indexer / scanner / parser / chunker / retriever 模块不改
- 文件锚点：`core/src/indexer/mod.rs`（加 `index_path_with_progress` + `IndexProgressSnapshot`）+ `core/src/server.rs`（加 `index` method impl）+ `core/tests/phase9_index_smoke.rs`（新增）

### Out Of Scope

- **Go 侧 daemon.Index client wrapper**（task-9.3 实施）
- **CLI 改造**（task-9.3 / 9.4）
- **task-8.2 reliability resume manifest 与 stream 集成**（task-9.3 决策 — manifest 仍在 Go CLI 侧维护，gRPC stream 不感知 resume；如未来要 server-side resume 走新 task）
- **Tantivy commit 频率优化 / 增量 stream 性能调优**（v0.2 baseline 用 `index_path` 现有 batch commit 即可；优化留 future task）
- **修改 `Cargo.toml` / `Cargo.lock`**（R7 严格通道；本 task 无新 dep — `tokio` / `tokio-stream` / `tonic` 已在）
- **修改 proto / schema_version**（task-9.1 已 freeze）

## 4. Users / Actors

- **task-9.3 go-cli-index 实施 agent**（下游）：消费本 task 暴露的 gRPC Index stream，包装为 Go CLI 用户体验
- **`internal/release/release_test.go::TestPhase9ReleaseSmoke_EndToEnd`**（task-9.5 下游）：本 task `phase9_index_smoke.rs` 是其 Rust 侧子集；release smoke 复用本 task 验证好的 indexer-via-gRPC 路径
- **PRD §User Flow 异常流"索引中断"用户**（间接）：本 task `IndexProgress` 的 `current_file` + `files_processed` 字段提供了 CLI 进度显示所需数据
- **task-2.4 indexer 现有调用方**（兼容性）：现有 `IndexSession::index_path` 调用方（仅 phase2_smoke.rs / phase6_smoke.rs 测试代码）不需修改，因为本 task 将 `index_path` 改为 thin wrapper

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§User Flow 异常流"索引中断" / §Decisions Log D3）
- `docs/specs/phases/phase-9-cli-pipeline.md`
- `docs/specs/tasks/task-9.1-proto-index-rpc.md`
- `docs/specs/tasks/task-2.4-indexer.md`（`IndexSession` 现有 API）
- `docs/specs/tasks/task-6.1-cli-search.md`（`CoreService` 现有 wire 模式 + `RetrieverError` → `Status` 映射先例）
- `docs/decisions/adr-001-go-rust-dual-binary-architecture.md`
- `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`
- `core/src/indexer/mod.rs`（`IndexSession::index_path` 当前实现）
- `core/src/server.rs`（`CoreService` 当前 trait impl）
- `core/tests/phase2_smoke.rs` + `core/tests/phase6_smoke.rs`（tonic in-process server smoke pattern）
- `test/features/cli-pipeline.feature`

### 5.2 Imports

- **Rust 标库**：`std::path::Path / PathBuf` / `std::sync::Arc`
- **tokio 生态**（已在 Cargo.toml — task-1.3）：`tokio::sync::mpsc` / `tokio::task::spawn_blocking` / `tokio_stream::wrappers::ReceiverStream`
- **tonic**（已在 task-1.1）：`tonic::Request` / `tonic::Response` / `tonic::Status` / `#[tonic::async_trait]`
- **内部**：`crate::indexer::{IndexSession, IndexStats, IndexError}` + `crate::scanner::ScanOptions` + `crate::chunker::ChunkPolicy / Provenance` + `crate::pb::{IndexRequest, IndexProgress, ...}`（task-9.1 codegen 产）
- **测试侧**：`tempfile::TempDir`（已在 dev-deps，task-2.4）+ `tonic::transport::{Server, Channel}`（已在）+ `tokio::runtime::Runtime`
- **不引入**：R7 严格 — `tokio-stream` 在 `core/Cargo.toml` 检查（如未直接列在 deps 但作为 tonic transitive 已可见 → 需独立 chore-dep PR 显式加 dev-deps 或 deps）。本 task §2A 时主 agent 必须 verify `tokio-stream` 已可作为直接依赖使用；如未，本 task 阻塞 → return needs-dep 对象给主 agent

### 5.3 函数签名

```rust
// core/src/indexer/mod.rs 新增 + 重构 ----

pub struct IndexProgressSnapshot<'a> {
    pub files_processed: usize,
    pub files_skipped_denied: usize,
    pub files_skipped_redaction: usize,
    pub chunks_written: usize,
    pub current_file: Option<&'a Path>,
}

impl IndexSession {
    /// 全量索引带 per-file progress 回调（Phase 9 task-9.2）。
    /// 现有 `index_path` 作为 thin wrapper 调本方法传 |_| {} no-op callback.
    pub fn index_path_with_progress<F>(
        &mut self,
        root: &Path,
        scan_options: &ScanOptions,
        policy: &ChunkPolicy,
        provenance: Vec<Provenance>,
        mut on_progress: F,
    ) -> Result<IndexStats, IndexError>
    where
        F: FnMut(&IndexProgressSnapshot<'_>);

    /// 现有签名保持不变（task-2.4 调用方零改动）— 实现改为 thin wrapper.
    pub fn index_path(
        &mut self,
        root: &Path,
        scan_options: &ScanOptions,
        policy: &ChunkPolicy,
        provenance: Vec<Provenance>,
    ) -> Result<IndexStats, IndexError> {
        self.index_path_with_progress(root, scan_options, policy, provenance, |_| {})
    }
}

// core/src/server.rs 新增 ----

use tokio_stream::wrappers::ReceiverStream;

#[tonic::async_trait]
impl ContextService for CoreService {
    // ... 现有 health / search 不变 ...

    type IndexStream = ReceiverStream<Result<IndexProgress, tonic::Status>>;

    async fn index(
        &self,
        request: tonic::Request<IndexRequest>,
    ) -> Result<tonic::Response<Self::IndexStream>, tonic::Status>;
}
```

- SCEN/TEST-9.2.1 → `index_path_with_progress` 回调按文件粒度触发 ≥3 次（≥3 normal files fixture），最终 IndexStats 正确（AC1）
- SCEN/TEST-9.2.2 → `index_path` thin wrapper 与原行为等价（task-2.4 phase2_smoke.rs 不回归）（AC2）
- SCEN/TEST-9.2.3 → `CoreService::index` 校验阶段拒非法 source_path → `Status::InvalidArgument`（AC3）
- SCEN/TEST-9.2.4 → `CoreService::index` 成功路径 stream 含 ≥4 条消息（每文件 1 + final done）+ final files_processed = N + chunks_written > 0 + error == ""（AC4）
- SCEN/TEST-9.2.5 → `phase9_index_smoke.rs` 端到端：tonic in-process server + client.index() consume stream + SQLite chunks row > 0 + Tantivy fixture marker 命中（AC5）

## 6. Acceptance Criteria

- [ ] **AC1** (本 task 新增 / ADR-013 §Decision #2): `IndexSession::index_path_with_progress` 实现按 ScannedFile 粒度触发 `on_progress` 回调；签名按 §5.3；对 ≥3 文件 fixture 触发 ≥3 次回调；累计 files_processed 与 IndexStats 一致
- [ ] **AC2** (PRD §Decisions Log D1 backward compatibility): 原 `IndexSession::index_path` 签名不变；现有 `core/tests/phase2_smoke.rs` + `core/tests/phase6_smoke.rs` 不修改即可继续通过（baseline 不回归）
- [ ] **AC3** (PRD §Decisions Log D3 / §REST·MCP 接口契约): `CoreService::index` 对 source_path 空 / 不存在 → 流建立前返回 `Status::InvalidArgument`；data_dir 空 → fallback `self.data_dir`；collection_id 空 → fallback `"default"`
- [ ] **AC4** (PRD §User Flow 异常流"索引中断"进度显示): `CoreService::index` 成功路径流含 ≥N+1 条 IndexProgress 消息（N = 处理文件数），按文件粒度 emit + final done=true 消息；error 字段 in-band 传递 indexer 内部错误（不通过 tonic::Status 中断 stream）；client 收 done=true 后通道关闭
- [ ] **AC5** (Phase 9 §6 端到端 smoke 落点 / 本 task 新增): `core/tests/phase9_index_smoke.rs::phase_9_index_grpc_end_to_end_smoke` 通过 — 临时 data_dir + 临时 source_path（≥3 .md + 1 .env denied + 1 secret-redacted .yaml） + tonic in-process server + client.index() stream consume + SQLite chunks > 0 + Tantivy 搜索 fixture marker 命中 + .env skipped + .yaml redacted（secret 不入索引）

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 index_path_with_progress 回调 | SCEN-9.2.1 | TEST-9.2.1 | - | unit-test | - |
| AC2 index_path thin wrapper 兼容 | SCEN-9.2.2 | TEST-9.2.2 | phase2_smoke / phase6_smoke 不回归 | unit-test | - |
| AC3 校验阶段错误映射 | SCEN-9.2.3 | TEST-9.2.3 | - | unit-test | - |
| AC4 stream 进度上报 | SCEN-9.2.4 | TEST-9.2.4 | - | unit-test | - |
| AC5 phase9_index_smoke 端到端 | SCEN-9.2.5 | TEST-9.2.5 | phase9_index_smoke.rs | unit-test (cargo test --test phase9_index_smoke) | - |

## 8. Risks

- 关联 PRD §Technical Risks **R1**（Go↔Rust gRPC 边界 / 进程生命周期）：本 task 引入 stream RPC 增加并发面 — mpsc channel 容量 32 / spawn_blocking 资源管理需测试；client cancel mid-stream 时 server-side spawn_blocking 任务不立即取消（spawn_blocking 不可被 cancel），但 channel send 失败后 indexer 仍跑完当前文件再退出，影响有限（不丢数据；SQLite + Tantivy 写入是 file-grained atomic）。
- 关联 PRD §Technical Risks **R6**（大仓库性能 / 资源不达标）：per-file progress emit 增加少量 channel send 开销；预期 < 1ms / file 不影响 P95；如未来发现回归（task-9.5 release smoke 100k chunk benchmark），降级为每 N 文件 emit 一次（batch emit）。
- 关联 **R4**（secret redaction 漏检 / 误报）：本 task 不引入 secret 处理新逻辑，纯 wrap 已 verified 的 `IndexSession::index_path`；phase9_index_smoke.rs AC5 含 secret redaction 回归断言。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位）。本 task `phase9_index_smoke.rs` 是 Rust 集成测试，被 `cargo test --workspace` 自动收纳；主 agent §4 Gate 3 可 `cargo test --test phase9_index_smoke` 精准抓。

## 10. Completion Notes

> 待 task 完成后回填。
