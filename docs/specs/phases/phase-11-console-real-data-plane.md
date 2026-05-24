# Phase 11 · console-real-data-plane

**Status**: Ready

> Phase Spec（s2v full-standard §8.2）。本 phase 是 v0.4.0 minor release 收口 phase — 把 v0.3 Phase 10 ([ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md)) 在 task-10.4 §10 显式记录的两个 Trade-off (`[SPEC-DEFER:task-future.cross-process-sqlite-sharing]` + JobRunner 不真索引) 一次性 resolve：
>
> 通过 [ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) cross-process Rust ↔ Go gRPC bridge 把 Console REST 9 endpoint 真接通 Rust 数据面 —— Workspace / IndexJob 真持久化跨 daemon 重启 + IndexJob 真触发 Rust `JobRunner.spawn_blocking(IndexSession::index_path_with_progress)` + Search 真接 retriever (Tantivy + SQLite chunks) + Events 真接 progress server stream。
>
> 治理基线：本 phase 按 ADR-011 单驱动 + ADR-012 主 agent 自治 + **ADR-014 cross-validation gate（D1/D2/D3/D4/D5 第二次完整激活）**；§2A Ready review 由主 agent 自审（本 phase 不涉及 cross-repo 字段变更 —— Console contractv1 字段集合 v0.3 锁定，建议主 agent 自审 + 用户复核选项）。详见 [ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) + [ADR-014](../../decisions/adr-014-cross-phase-exit-criteria-validation.md) + [ADR-013](../../decisions/adr-013-cli-data-plane-grpc-bridge.md)。

## 1. 阶段目标

实现 ContextForge 内部 Rust ↔ Go cross-process gRPC bridge：`core/proto/console_data_plane.proto` 4 个新 service (WorkspaceService / JobService / SearchService / EventsService) + `core/src/data_plane/` tonic server 实现复用 [ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md) D2/D3 落地的 `SqliteWorkspaceStore` + `SqliteJobStore`；Go `internal/consoleapi/grpcclient/` 实现 4 个 client wrapper + 替换 v0.3 `internal/consoleapi/memstore.go` MemStore 为默认 gRPC-backed；MemStore 降级为 env-gated fallback (`CONSOLE_API_FALLBACK_INMEM=1`)；`JobService.Enqueue` 真触发 `JobRunner.spawn_blocking(IndexSession::index_path_with_progress)` + heartbeat 每 100 files 或 5s + co-operative cancel via `CancelToken Arc<AtomicBool>` + `JobOutcome` 写回 status + error_message；`SearchService.Query` 真接 existing retriever (Tantivy + SqliteChunkStore) + `RetrievalTrace.retrieved_chunks` 真填 (score + source_file + content snippet ≤200 字)；`EventsService.Subscribe` 真接 `JobRunner` progress callback 经 tokio broadcast channel；Go `/v1/observability/events` 改 long-poll wrap (30s timeout / 100 evt batch)；`scripts/console_smoke.sh` v2 REAL mode 默认 + `CONSOLE_REAL_SMOKE_EXIT=0`；`scripts/release_smoke.sh` 第 5 段更新为 REAL 模式 + `PHASE_RELEASE_SMOKE_EXIT=0`。来源：[ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) D1-D6 / task-10.4 §10 Completion Notes Trade-off #1 + #2 / PRD §Open Questions O14 / PRD §Implementation Phases v0.4 新增 (见 PRD §Implementation Phases Phase 11 行)。

## 2. 业务价值

闭环 v0.3 Phase 10 收口时显式留下的两处 conscious gap（task-10.4 §10 Trade-off #1 + #2）。直接支撑：

- **Console UI 端真业务面**：workspace 跨 daemon 重启不丢；POST `/v1/index-jobs` 真启动 Rust 索引；POST `/v1/search` 真返回 indexed 分块；GET `/v1/observability/events` 真流 progress 事件 — 不再是 v0.3 demo 状态机 in-memory 模拟
- **Rust 持 SoT 制度落地**：ADR-016 D1/D5 把 SQLite schema 单 owner = Rust 团队 固化下来；为后续 Console endpoint expansion (`/v1/memory*` / `/v1/eval-runs*` 等) 提供 cross-process gRPC bridge 复用模板 [SPEC-DEFER:console-endpoint-expansion]
- **复用 ADR-013 cli-data-plane gRPC**：Phase 9 已建立的 tonic + prost + `:48180` 模式延伸到 business plane，不引入新端口/auth 边界
- **MemStore 降级为 env-gated fallback**：v0.3 集成测试 fixture (`internal/consoleapi/e2e_test.go` 等) 不破坏；运维 degraded 模式有明确信号 (`/v1/health` `missing=["data_plane"]`)

## 3. 涉及模块

- `core/proto/console_data_plane.proto`（新增：4 service × 14 RPC + 11 message 类型，1:1 镜像 Go contractv1 JSON tag）
- `core/build.rs`（修改：tonic_build 编译列表新增 `console_data_plane.proto`；复用 ADR-013 既有 pattern）
- `core/src/data_plane/`（新增 module：`mod.rs` + `workspace.rs` + `job.rs` + `search.rs` + `events.rs` 实现 4 个 tonic service trait）
- `core/src/data_plane/workspace.rs`（新增：包 `SqliteWorkspaceStore` 引用 + WorkspaceService trait 实现）
- `core/src/data_plane/job.rs`（新增：包 `SqliteJobStore` + `JobRunner` 真接 `IndexSession::index_path_with_progress` + heartbeat 100 files/5s + CancelToken）
- `core/src/data_plane/search.rs`（新增：包 retriever 引用 + SearchService trait 实现 + RetrievalTrace 真填）
- `core/src/data_plane/events.rs`（新增：包 EventBus tokio broadcast channel + EventsService server stream + 容量 1000）
- `core/src/bin/contextforge_core.rs` 或 daemon `serve` 子命令入口（修改：注册 4 service 到 `tonic::transport::Server::builder().add_service(...)`；监听 `:48180`）
- `internal/consoleapi/grpcclient/`（新增：`grpcclient/grpcclient.go` 含 New(addr, opts...) + 4 client wrapper impl Go Deps 4 接口）
- `internal/consoleapi/types.go` / `router.go` / `handlers.go`（修改：handlers 重构为 thin protocol translator；不引入字段映射代码）
- `internal/consoleapi/memstore.go`（保留但降级：仅当 `CONSOLE_API_FALLBACK_INMEM=1` env 设时启用）
- `internal/cli/console_api_serve.go`（修改：新增 `--grpc-addr` flag 默认 `127.0.0.1:48180` + `--fallback-inmem` flag 别名 env）
- `test/conformance/console_contractv1_test.go`（不修改：v0.3 测试仍 PASS 是 task-11.2 §6 AC5）
- `test/fixtures/index-job-real/`（新增：≥5 文件 markdown fixture 供 task-11.3 真索引测试）
- `scripts/console_smoke.sh`（修改 v2：REAL mode 默认；local-only mode 保留为 `LOCAL_ONLY=1` env；`CONSOLE_REAL_SMOKE_EXIT=0` final marker）
- `scripts/release_smoke.sh`（修改：第 5 段更新为 REAL 模式 + `PHASE_RELEASE_SMOKE_EXIT=0`）
- `docs/s2v-adapter.md`（修改：§Phases 加 Phase 11 行 / §Tasks 加 task-11.1～11.4 / §ADRs 加 ADR-016 / §BDD 加 console-real-data-plane.feature）
- `docs/prds/context-forge.prd.md`（修改：§Implementation Phases 加 Phase 11 段 + §Open Questions O14 新增）
- `test/features/console-real-data-plane.feature`（新增：≥10 scenarios）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 11.1 | core/proto + core/src/data_plane | `../tasks/task-11.1-rust-data-plane-grpc-services.md` |
| 11.2 | internal/consoleapi/grpcclient | `../tasks/task-11.2-go-rest-to-grpc-proxy.md` |
| 11.3 | core/src/data_plane/job + IndexSession wiring | `../tasks/task-11.3-indexjob-real-runner-wiring.md` |
| 11.4 | core/src/data_plane/search + events | `../tasks/task-11.4-search-real-retriever-and-events.md` |

## 5. 依赖关系

- **依赖**：Phase 10（console-contract-v1）— 复用 [ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md) task-10.2 `SqliteWorkspaceStore` (`core/migrations/0010_workspaces.sql`) + task-10.3 `SqliteJobStore` + `JobRunner` 框架 (`core/migrations/0011_index_jobs.sql`) + task-10.4 Go `internal/consoleapi/` 9 REST handler + bearer middleware + sentinel error mapping；Phase 9（cli-pipeline）— 复用 [ADR-013](../../decisions/adr-013-cli-data-plane-grpc-bridge.md) task-9.1 proto + task-9.2 Rust gRPC server pattern + tonic + prost 工具链；Phase 2/4（index-core / retrieval-explain）— 复用 task-2.4 `IndexSession::index_path_with_progress` API + task-4.1/4.2 retriever (Tantivy + SqliteChunkStore)。
- **可并行**：否（v0.4 收口 phase）。Phase 内顺序：task-11.1（proto + tonic server 框架初步实现，service 内部细节由后续 task 替换 [SPEC-OWNER:task-11.3]）→ task-11.2（Go grpcclient + handler thin proxy + MemStore 降级）→ task-11.3（JobService 真触发 JobRunner.spawn_blocking）→ task-11.4（SearchService + EventsService 真接通）。
- **Phase 内并行机会**：task-11.3 (JobRunner ↔ IndexSession wiring) ∥ task-11.4 (Search + Events) 在 task-11.2 完成后可并行 — 两者各自独立 Rust module（`data_plane/job.rs` vs `data_plane/search.rs` + `events.rs`），写路径互不相交；但 task-11.4 EventsService 真接 progress 依赖 task-11.3 JobRunner emit `indexing.progress`，故 task-11.4 §6 AC3 (event 真流) 必须 task-11.3 完成后才能 verify —— 主 agent 选串行实施（v0.4 简化策略，并行收益小）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（任务 11.1-11.4 全 Done，实测验证；每条 AC 含 ADR-014 D3 verified by 显式 owner）**：

- [ ] AC1：Rust contextforge-core daemon 启动后监听 `:48180` gRPC，4 个新 service (`ContextForge.ConsoleDataPlane.WorkspaceService` / `JobService` / `SearchService` / `EventsService`) 注册可用 — **verified by task-11.1 §6 AC1 + phase-smoke step 1 (cmd: `grpcurl -plaintext 127.0.0.1:48180 list | grep ConsoleDataPlane`)**
- [ ] AC2：Go console-api-serve 启动后默认连本机 `:48180` gRPC；所有 9 REST endpoint 走 gRPC proxy；`CONSOLE_API_FALLBACK_INMEM` 未设时 gRPC 不可达 → `/v1/health` 返回 `degraded=true` + `missing=["data_plane"]` + HTTP 503 — **verified by task-11.2 §6 AC4 + phase-smoke step 2 (cmd: 启动 console-api-serve 无 daemon → curl /v1/health → assert status_code=503 + degraded payload)**
- [ ] AC3：POST `/v1/workspaces` 后 daemon 重启 → GET `/v1/workspaces` 仍返回该 workspace（真持久化）— **verified by task-11.2 §6 AC2 + phase-smoke step 3 (curl POST → kill daemon → restart → curl GET)**
- [ ] AC4：POST `/v1/index-jobs` 指向 fixture repo (≥5 markdown 文件) → 等 status=succeeded → POST `/v1/search` 真返回 fixture 文件分块（≥1 SourceChunk + score>0 + source_file 匹配 fixture）— **verified by task-11.3 §6 AC2 + task-11.4 §6 AC1 + phase-smoke step 4 (cmd: `bash scripts/console_smoke.sh` REAL mode 全程跑)**
- [ ] AC5：cancel in-flight job 真停（observed via GET `/v1/index-jobs/<id>.status=cancelled` within 5s）+ events 含 `indexing.progress` 事件流（≥1 evt 含 job_id + processed_files + total_files）— **verified by task-11.3 §6 AC3 + task-11.4 §6 AC3/AC4**
- [ ] AC6：ADR-014 cross-validation gate 全套通过：D2 lint (`bash scripts/spec_drift_lint.sh --touched origin/master` 0 violation) + D3 phase §6 每条 AC 含 verified by + D1 closeout PR body 含 mapping 表 — **verified by phase-smoke step 5 (cmd: `bash scripts/spec_drift_lint.sh --touched origin/master`)**

**端到端 smoke**：

```bash
# step 1 — Phase 11 主集成 smoke (Rust daemon + Go console-api-serve + REAL mode)
bash scripts/console_smoke.sh
# 1) spawn contextforge-core daemon + gRPC :48180
# 2) spawn console-api-serve + REST :48181
# 3) curl 9 endpoint + workspace 持久化跨重启 + index-job 真跑 fixture + search 真返回 + events 真流
# 4) CONSOLE_REAL_SMOKE_EXIT=0

# step 2 — ADR-014 cross-validation gate (D2 lint)
bash scripts/spec_drift_lint.sh --touched origin/master

# step 3 — Release smoke 第 5 段 REAL 模式
bash scripts/release_smoke.sh
# PHASE_RELEASE_SMOKE_EXIT=0
```

step 1 是 task-11.4 Gate 3 入口：spawn 双进程 + curl 9 endpoint + fixture index 真跑完 + search 真返回分块 + events 真流。`CONSOLE_REAL_SMOKE_EXIT=0` 是 final marker。

step 2 是 ADR-014 D2 lint gate：phase closeout PR 触及行无未标注 anti-pattern；强制 0 violation。

step 3 是 release_smoke.sh 端到端 hand-off：v0.4 第 5 段更新为 REAL mode 后整脚本仍 `PHASE_RELEASE_SMOKE_EXIT=0`，证明 v0.4 ship gate 不退化。

**Scope 注**：本 phase smoke 与 task-8.3 release_smoke.sh + task-9.6 quickstart_smoke.sh + task-10.6 console_smoke.sh v1 互补 — task-8.3 (v0.1) gate tarball + Rust gRPC search smoke；task-9.6 (v0.2) 新增 CLI binary 7-step；task-10.6 (v0.3) 新增 "Console UI 真调 ContextForge" docker compose 段；task-11.4 (v0.4) 升级 console_smoke.sh 为 REAL mode 默认（dropping in-memory 模拟 mode 为 `LOCAL_ONLY=1` env-gated fallback）。四条 smoke 均跑通才允许 v0.4.0 tag。

## 7. 阶段级风险

- **关联 [ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) §Rollback 6 条风险**：
  - D1 SoT 反向（Go 写 SQLite）→ §自决规则 R8 立刻 STOP；revert + 从最近未违 D1 commit 重启该 task
  - D2 4 service 字段命名与 Go contractv1 不齐 → surface diff 表 + 人工拍板；不允许 handler 内加字段映射代码
  - D3 thin proxy 被违反（handler 内业务逻辑）→ 立刻 STOP；下推到 Rust gRPC method
  - D4 MemStore fallback 悄悄成默认（违 env-gated 约束）→ grep router.go + STOP
  - D5 Go 团队创建 migration → STOP；schema 单 owner = Rust
  - D6 governance 第二次激活：ADR-014 D1/D2/D3 应 v0.3 模式延用；若发现制度不稳定 → Phase 11 retrospective 评估
- **关联 task-11.3 IndexSession ↔ JobRunner wiring 边界 case**（最高风险点）：现有 `core/src/index.rs::IndexSession::index_path_with_progress` 必须能在 `spawn_blocking` 闭包内被 callback 驱动；borrow checker 在 `Arc<Mutex<rusqlite::Connection>>` clone 边界 case 上易撞（task-10.3 同坑）；缓解 task-11.3 §10 trade-off T1 加 "扩展 IndexSession API" 路径
- **关联 PRD §Technical Risks R1**（Go↔Rust gRPC 边界）：本 phase 新增 4 service；按 ADR-013 既有 pattern，gRPC 字段 add-only freeze 维持（ADR-001/003 边界不动）
- **关联 PRD §Technical Risks R6**（大仓库索引性能）：task-11.3 heartbeat 每 100 files 或 5s 引入额外 SQLite 写 IO；缓解 batch update + 不每文件写
- **关联 ADR-014 governance 第二次激活风险**：v0.3 首次激活 D1/D2/D3/D4 全套跑通；v0.4 验证制度稳定性；若 D2 lint 词表误报 / D1 mapping 表撰写成本超预期 → Phase 11 retrospective + ADR-014 v2 调整

## 8. Phase Definition of Done

- [ ] 本 phase 全部 task spec Status=Done（11.1/11.2/11.3/11.4 全 Done — PR 顺序合）
- [ ] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过（console_smoke.sh REAL mode 真跑 + spec_drift_lint.sh --touched 0 violation + release_smoke.sh 第 5 段 REAL）
- [ ] 关联风险 ADR-016 §Rollback 6 条 / R1 / R6 / ADR-014 治理风险缓解措施已落地
- [ ] adapter §Phase 状态索引该行 Status 同步更新（closeout PR）
- [ ] ADR-016 状态推进 Proposed → Accepted（closeout PR）
- [ ] PRD §Implementation Phases Phase 11 行新增（含 Status=Done / 描述 / 范围 / 依赖 / 可并行）+ §Open Questions O14 标记 `resolved by ADR-016 (business plane wiring); endpoint expansion 留 v0.4.x` [SPEC-DEFER:console-endpoint-expansion]
- [ ] **ADR-014 D1 mapping 表**：closeout PR body 含 Phase §6 ↔ Task §6 AC 映射（AC1-6 每行 4 字段：phase AC 字面 / 拥有 task or 验证方式 / task §6 AC 字面 / Evidence 链接）
- [ ] **ADR-014 D2 lint 输出**：closeout PR body 含 `bash scripts/spec_drift_lint.sh --touched origin/master` 0 violation 输出
- [ ] §4 Gate 4.5 ADR-014 cross-validation gate 通过 — v0.4.0 release tag prep ready
