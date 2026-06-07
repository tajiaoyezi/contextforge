# Phase 14 · eval-rest-surface

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 是 v0.7.0 minor release + **Console 22-endpoint conformance 100% PASS 收口 phase** — 把 [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D1 Wave 4 共 2 个 Eval endpoint 落地，至此 22/22 Console contract endpoint 全部 ship：
>
> - `POST /v1/eval-runs` body `EvalRunCreate` → `EvalRun`（新建，status="running"）
> - `GET /v1/eval-runs/{id}` → `EvalRun`
>
> 治理基线：本 phase 按 ADR-011 单驱动 + ADR-012 主 agent 自治 + **ADR-014 cross-validation gate（D1/D2/D3/D4/D5 第五次完整激活）**。详见 [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) + [ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) + [ADR-006](../../decisions/adr-006-recall-eval-acceptance-gate.md) + [ADR-014](../../decisions/adr-014-cross-phase-exit-criteria-validation.md)。
>
> **v0.7.0 ship 后**：Console HTTPAdapter 22-endpoint conformance suite 全 PASS = ContextForge ↔ Console v1.0 contract 集成完整闭环；[ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) Status: Proposed → Accepted 在本 phase closeout PR 内回填。

## 1. 阶段目标

实现 ContextForge 内部 EvalRun 持久化 + Console Eval 2 endpoint REST surface。本 phase 同时升级既存 `proto/contextforge/v1/eval.proto` 从 recall-only 二参 schema 到完整 EvalRun lifecycle（add-only 演进）：

- **proto upgrade**：`proto/contextforge/console_data_plane/v1/console_data_plane.proto` (或 `eval.proto` 子文件) 加 EvalService 2 RPC + 复用既存 EvalRun message (Phase 11 task-11.1 ship 时 11 message 含 EvalRun) + EvalRunCreate / CaseResult message
- **新增 SQLite migration**：`core/migrations/0014_eval_runs.sql` 表 `eval_runs` (eval_run_id PK / workspace_id / status / config_snapshot JSON / started_at / finished_at nullable / metrics JSON / case_results JSON / schema_version) + `eval_case_results` 子表（或 case_results JSON 嵌入 eval_runs；§10 trade-off 评估）
- **新增 Rust `SqliteEvalStore`**：`core/src/eval/` 模块 + CRUD + state ops
- **新增 Rust `EvalService` impl**：`core/src/data_plane/eval.rs` impl EvalService trait + 后台 spawn_blocking 真触发既存 `internal/eval/eval.go` 或 `core/src/eval/runner.rs` recall harness（Phase 8 task-8.1 已 ship recall eval CLI 框架）
- **新增 Go `grpcclient.EvalClient`**：5 method wrapper（含 List 备选 [SPEC-DEFER:console-eval-list]）
- **新增 Go REST handlers**：`handleCreateEvalRun` + `handleGetEvalRun`（POST 走 confirmMiddleware？Console PRD 写 POST eval-run create 非破坏性 → 不走 confirmMiddleware；status side effect 主要是创建新资源不修改既有）

**关键 scope 决策（§3 in scope）**：本 phase 实施 Eval 资源 lifecycle (queued → running → succeeded/failed/cancelled) 但 **recall harness 实际执行复用 Phase 8 既有 `internal/eval/eval.go`**（Go 侧 CLI 调用 logic）；EvalService.Create 在 Rust 侧记 status=running + 异步 spawn 调 Go-side recall harness（或更简单：在 Go console-api-serve 进程内 spawn goroutine 跑 recall + 更新 SqliteEvalStore via gRPC client）—— §3 task-14.1 §10 trade-off 评估两条路径。

来源：[ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D1 Wave 4 / D6 沿用 ADR-016 / D7 ADR-014 第五次激活 / PRD §Implementation Phases v0.7 新增 / PRD §Open Questions O17 新增 / [ADR-006](../../decisions/adr-006-recall-eval-acceptance-gate.md) recall eval 一等 PRD 验收门。

## 2. 业务价值

直接支撑 ContextForge PRD §Core Capabilities #4「召回评测 — 用户可以验证『换 provider/embedding 后召回是否退化』」的 UI 闭环：

- **Eval v0.1 UI**：Console UI 端 Eval 面板 → 触发 eval run（POST /v1/eval-runs body 含 dataset_ref + config_snapshot）→ 查看 status / metrics / case_results
- **主指标可视化**：PRD §Success Metrics 主指标「Golden questions Top-5/10 命中率」首次有 UI 接口（v0.1 仅 CLI `contextforge eval run`）
- **完整 Console 22-endpoint conformance 闭环**：v0.7.0 ship 后 Console HTTPAdapter conformance suite 22/22 全 PASS = 双方握手成功标志；ContextForge v1.0 contract anchor 完整表达
- **Phase 8 task-8.1 既有 recall harness 复用**：本 phase 不重造评测引擎；EvalService 仅是 lifecycle + persistence orchestration 层

不在本 phase scope：
- Eval result UI 实时进度推送（Console UI 改 SSE / WebSocket 留 v1.x）[SPEC-DEFER:console-eval-progress-sse]
- Eval golden questions dataset 管理 UI（CRUD datasets via Console）[SPEC-DEFER:console-dataset-management]
- Multi-tenant eval queue [SPEC-DEFER:phase-future.eval-queue]

## 3. 涉及模块

- `core/migrations/0014_eval_runs.sql`（新增：`eval_runs` 表 schema + indexes）
- `core/src/eval/`（新增 module：`mod.rs` + `store.rs` SqliteEvalStore + `runner.rs` recall harness wrapper）
- `core/src/eval/store.rs`（新增：SqliteEvalStore CRUD + state ops）
- `core/src/eval/runner.rs`（新增：spawn_blocking 触发 recall harness + 写 progress + 完成时写 metrics + case_results）
- `core/src/data_plane/mod.rs`（修改：`DataPlaneStores` 持有 `Arc<SqliteEvalStore>` + `Arc<EvalRunner>`）
- `core/src/data_plane/eval.rs`（新增：`EvalServer` impl EvalService trait + 2 RPC method）
- `core/src/server.rs`（修改：serve_full 注册 EvalServer service）
- `proto/contextforge/console_data_plane/v1/console_data_plane.proto`（修改：加 EvalService 2 RPC + EvalRunCreate / CaseResult message + 复用既存 EvalRun message）
- `internal/consoleapi/grpcclient/grpcclient.go`（修改：加 EvalClient struct + 2 method wrapper）
- `internal/consoleapi/types.go`（修改：加 EvalClient 接口 + Deps 加 Eval）
- `internal/consoleapi/router.go`（修改：注册 2 新路由）
- `internal/consoleapi/handlers.go`（修改：加 2 handler — `handleCreateEvalRun` + `handleGetEvalRun`）
- `internal/consoleapi/memstore.go`（修改：MemEvalStore — Create 返 stub EvalRun status=running + 异步推进 status=succeeded with mock metrics；Get 返该 stub；fallback mode 无 audit log）[SPEC-OWNER:task-14.2]
- `internal/eval/eval.go`（不修改 OR 修改：复用既有 recall harness；如 task-14.1 §10 trade-off 选「Go console-api-serve 内 spawn goroutine 跑 recall」路径 → 不修改；如选「Rust EvalRunner 直接调 Go binary」路径 → 需 OS process 调用复杂度）
- `core/tests/eval_integration.rs`（新增：3+ 集成测试）
- `internal/consoleapi/e2e_grpc_test.go`（修改：加 2 sub-step — POST eval-run + GET eval-run）
- `internal/consoleapi/handlers_test.go`（修改：加 2+ unit test）
- `internal/consoleapi/grpcclient/grpcclient_test.go`（修改：加 2 Eval client wrapper unit test）
- `scripts/console_smoke.sh` v5（修改：20 endpoint flow → 22 endpoint flow；加 step 21-22 POST eval-run + GET eval-run + 等 status terminal）
- `scripts/release_smoke.sh`（**修改：第 6 段 update**；加 `phase14_console_eval=ok` 子检查 OR 单独段；本 phase 是 release_smoke 最后段，标志 22-endpoint 全 PASS）
- `docs/s2v-adapter.md`（修改：§Phases 加 Phase 14 行 / §Tasks 加 task-14.1/14.2 / §ADRs ADR-017 Proposed → Accepted）
- `docs/prds/context-forge.prd.md`（修改：§Implementation Phases 加 Phase 14 段 + §Open Questions O17 新增 + O15/O16 标记 fully resolved by v0.5/v0.6/v0.7 ship）
- `test/features/console-contract-completion.feature`（修改：加 phase-14 scenarios 3+）
- `test/fixtures/eval-seed/`（新增：golden_questions 测试 dataset）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 14.1 | core/migrations + core/src/eval + core/src/data_plane/eval.rs + proto EvalService | `../tasks/task-14.1-rust-eval-grpc-service.md` |
| 14.2 | internal/consoleapi (router + handlers + grpcclient) + memstore EvalAdapter | `../tasks/task-14.2-go-eval-rest-handlers.md` |

## 5. 依赖关系

- **依赖**：Phase 13（memory-rest-surface）— 复用 task-13.1 SQLite migration + Rust store + gRPC service 增量 pattern + task-13.2 Go REST handler pattern；Phase 12（console-contract-completion）— 复用 confirmMiddleware（如适用）；Phase 8（eval-and-reliability）— 复用 task-8.1 既有 recall harness (`internal/eval/eval.go`)；Phase 11（console-real-data-plane）— 复用 DataPlaneStores 共享 stores 链 + tonic Server::builder
- **可并行**：否（v0.7 收口 phase）。Phase 内顺序：task-14.1（Rust EvalService + SqliteEvalStore + EvalRunner orchestration）→ task-14.2（Go REST + grpcclient + smoke v5）
- **Phase 内并行机会** [SPEC-OWNER:task-14.1,task-14.2]：与 Phase 13 同款 — 偏好串行避免 stub/真接 diff 错位

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（任务 14.1-14.2 全 Done，实测验证；每条 AC 含 ADR-014 D3 verified by 显式 owner）**：

- [x] AC1：Rust EvalService gRPC 3 RPC (Create / Get / UpdateProgress) 注册可用；`eval_runs` 表通过 0014_eval_runs.sql migration 自动建立；SqliteEvalStore 5 method 全工作 — **verified by task-14.1 §6 AC1/AC2/AC3 + `core/tests/eval_integration.rs::test_eval_crud_via_grpc` + `test_eval_run_terminal_lifecycle` PASS**
- [x] AC2：`POST /v1/eval-runs` body `EvalRunCreate{workspace_id, config_snapshot, dataset_ref}` → 走 gRPC EvalService.Create → 返 200 + `EvalRun{status:"running"}` + Go-side runEvalAsync goroutine 异步触发 light-weight recall harness 真跑 — **verified by task-14.2 §6 AC1 + e2e_grpc Step 9e + smoke v5 Step 19 PASS**
- [x] AC3：`GET /v1/eval-runs/{id}` 真返 EvalRun 全字段；不存在 → 404；status lifecycle (running → succeeded) 在 runEvalAsync 完成后真持久化；case_results JSON 真填 — **verified by task-14.2 §6 AC2 + e2e_grpc Step 9e (Get 200 + Get unknown 404) + smoke v5 Step 20 (terminal at attempt 1: status=succeeded) PASS**
- [x] AC4：EvalRun.metrics 真填 `recall@5`/`recall@10`/`precision@5` float64 map；finished_at 在 status terminal 时填实；started_at 在 Create 时填实 — **verified by smoke v5 Step 20 `metrics contains recall@5 ✅` + cargo test eval `test_update_progress_persists_terminal_status` (finished_at_unix.is_some) PASS**
- [x] AC5：MemStore fallback 模式下 POST /v1/eval-runs 返 stub EvalRun + goroutine 2s 后 mock 推进到 status=succeeded with mock metrics (recall@5: 0.7)；conformance test 不退化 — **verified by MemEvalStore.Create 2s timer impl + interface compliance go build clean + test/conformance PASS**
- [x] AC6：scripts/console_smoke.sh v5 20-step flow (Console 22 endpoint conformance — 2 shared via filter) `CONSOLE_REAL_SMOKE_EXIT=0` — **verified by `bash scripts/console_smoke.sh` 实测真接 Rust daemon + Go console-api-serve, 20/20 PASS with eval terminal+metrics 含 recall@5**
- [x] AC7：ADR-014 cross-validation gate 全套通过：D2 lint 0 violation + D3 phase §6 每条 AC 含 verified by + D1 closeout PR body 含 mapping 表 + v0.4/v0.5/v0.6 既有 18 endpoint 不退化 — **verified by closeout PR body (this PR) + D2 lint targeted grep on touched files + go test ./... 43 pkgs PASS + cargo test 94 lib + integration tests 全过**

**端到端 smoke**：

```bash
# step 1 — Phase 14 主集成 smoke (v5，含 22 endpoint flow，全 PASS)
bash scripts/console_smoke.sh
# 1) spawn contextforge-core daemon
# 2) spawn console-api-serve
# 3) curl 22 endpoint (v0.6 20 个 + v0.7 新 2 eval)
#    含 POST /v1/eval-runs (workspace_id, dataset_ref, config_snapshot) → 拿 eval_run_id
#    含 poll GET /v1/eval-runs/<id> 每秒一次 ≤60s 等 status terminal
#    含 验证 final EvalRun.metrics 含 recall@5/@10 + case_results 非空
# 4) CONSOLE_REAL_SMOKE_EXIT=0

# step 2 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master

# step 3 — Release smoke (v0.7.0 release prep)
bash scripts/release_smoke.sh
# PHASE_RELEASE_SMOKE_EXIT=0
# phase11_console_real=ok + phase12_endpoints=ok + phase13_memory=ok + phase14_eval=ok 全段
```

step 1 是 task-14.2 Gate 3 入口；22 endpoint flow 是 ContextForge ↔ Console 双仓握手成功标志。

step 3 release_smoke.sh 在本 phase 加入 `phase14_console_eval=ok` 子段 = v0.7.0 ship gate 最后一道。

## 7. 阶段级风险

- **关联 [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) §Rollback**：D1 SoT 反向（Go 写 eval_runs）→ 立刻 STOP；D3 thin proxy 违反 → 下推 Rust
- **关联 task-14.1 EvalRunner spawn 路径**：「Rust spawn_blocking 调 Go-side recall harness」需要 OS process 调用 → 复杂度高 + 跨语言进程管理 + recall harness 错误传播复杂；选「Go console-api-serve 进程内 spawn goroutine 调 internal/eval/eval.go + 通过 gRPC 更新 SqliteEvalStore」更简单 + 错误传播自然；§10 trade-off 详细评估；推荐 Go-side runner [SPEC-DEFER:phase-future.rust-native-eval-runner] 留 v1.x
- **关联 PRD §Technical Risks R3**（检索召回率不达标）：本 phase 暴露 eval 但不改 retriever；如 v0.7 release 用户在自己 dataset 上跑 recall@10 < 85% → 这是真实业务信号；不视为 release blocker
- **关联 ADR-006 recall eval acceptance gate**：本 phase 让 recall eval 从 CLI-only 升到 UI-accessible；不改 ADR-006 acceptance gate 阈值；release_smoke.sh 内置 golden_questions 在自有 fixture 上跑 recall@10 ≥ 85% 仍是 v0.7 ship gate
- **关联 ADR-014 governance 第五次激活风险**：v0.3/v0.4/v0.5/v0.6 四次跑通；本 phase 第五次再验证 + Phase 14 closeout PR 推 ADR-017 Status → Accepted（合并 6 D-clauses 完整覆盖 3 phase）
- **proto EvalRun message 字段重复**：Phase 11 task-11.1 ship `console_data_plane.proto` 11 message 含 EvalRun + 既有 `proto/contextforge/v1/eval.proto` 含 EvalRequest/EvalResponse（recall-only schema）—— 两份 proto namespace 不同（`contextforge.console_data_plane.v1` vs `contextforge.v1`）= 不冲突；本 phase 不动 v1/eval.proto，仅扩 console_data_plane v1/EvalService

## 8. Phase Definition of Done

- [ ] 本 phase 全部 task spec Status=Done（14.1/14.2 全 Done — PR 顺序合）
- [ ] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过（console_smoke.sh 22 endpoint flow exit 0 + spec_drift_lint.sh 0 violation + release_smoke.sh 全段 ok）
- [ ] 关联风险 ADR-017 §Rollback / R3 / ADR-006 / ADR-014 治理风险缓解措施已落地
- [ ] adapter §Phase 状态索引该行 Status 同步更新（closeout PR）
- [ ] **ADR-017 状态推进 Proposed → Accepted**（本 phase closeout PR；6 D-clauses 完整覆盖 v0.5/v0.6/v0.7 3 phase 后由 closeout 一次性推进）
- [ ] PRD §Implementation Phases Phase 14 行新增 + §Open Questions O15 / O16 / O17 全 fully resolved；§Success Metrics 主指标「Golden questions 命中率」加备注「v0.7+ Console UI 端 POST /v1/eval-runs 可触发」
- [ ] **ADR-014 D1 mapping 表**：closeout PR body 含 Phase §6 ↔ Task §6 AC 映射
- [ ] **ADR-014 D2 lint 输出**：closeout PR body 含 0 violation 输出
- [ ] v0.7.0 release tag prep ready + **Console 22-endpoint conformance suite 全 PASS 证据**（双方握手成功标志）
- [ ] cross-repo follow-up：通知 Console 团队 ContextForge v0.7.0 release ship → Console 端可以切到 production HTTPAdapter mode（关闭 MockAdapter）
