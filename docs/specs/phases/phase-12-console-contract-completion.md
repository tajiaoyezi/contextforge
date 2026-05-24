# Phase 12 · console-contract-completion

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 是 v0.5.0 minor release 收口 phase — 把 v0.4 Phase 11 ([ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md)) ship 后 [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D1 Wave 1 + Wave 2 共 6 个 endpoint 落地：
>
> Wave 1（quick win）：PATCH `/v1/workspaces/{id}/config`、GET `/v1/index-jobs?status=active`、`POST /v1/index-jobs/{id}/cancel` 改 204、X-Confirm 服务端 412 兜底
> Wave 2（mid scope）：GET `/v1/source-chunks/{id}`、GET `/v1/search/{query_id}/trace`（含 Rust SearchService 加 GetSourceChunk + GetSearchTrace RPC + trace 持久化 by query_id）
>
> 治理基线：本 phase 按 ADR-011 单驱动 + ADR-012 主 agent 自治 + **ADR-014 cross-validation gate（D1/D2/D3/D4/D5 第三次完整激活）**；§2A Ready review 由主 agent 自审（本 phase 不涉及 cross-repo 字段变更 —— Console contractv1 字段集合 v0.3 锁定）。详见 [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) + [ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) + [ADR-014](../../decisions/adr-014-cross-phase-exit-criteria-validation.md)。

## 1. 阶段目标

完成 ContextForge Console Contract v1 缺失 13 个 endpoint 中的 6 个（9/22 → 15/22 ≈ 68% 覆盖率）：

- **Wave 1 quick win（task-12.1，~3-4 day）**：
  - PATCH `/v1/workspaces/{id}/config`（更新 allowlist/denylist；body `{allowlist:[], denylist:[]}` → 返更新后 `Workspace`）—— backend 复用 ADR-016 已存在的 `WorkspaceService.Update` gRPC RPC（task-11.1 已 ship）；只需加 Go REST handler + grpcclient.Workspace.Update wrapper
  - GET `/v1/index-jobs?status=active`（list filter；仅返 queued/running）—— backend 复用 ADR-016 `JobService.List` RPC + status filter 字段；只需加 Go REST handler + grpcclient.Job.List wrapper
  - `POST /v1/index-jobs/{id}/cancel` 改返 `204 No Content`（[ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D3；当前 200）
  - `confirmMiddleware(handler)` 服务端 X-Confirm 兜底 ([ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D2)：破坏性 endpoint（PATCH workspace config + Phase 13 deprecate/soft-delete 5 个共 [SPEC-OWNER:task-13.2]）必须 header `X-Confirm: yes` 或 query `?confirm=true` 任一；缺失返 412 Precondition Failed
- **Wave 2 mid scope（task-12.2 + task-12.3，~3-5 day）**：
  - GET `/v1/source-chunks/{id}`（按 chunk_id 取单个 chunk 详情）—— **新增 Rust SearchService.GetSourceChunk RPC** + retriever 加 by-chunk_id lookup + Go REST handler + grpcclient.Search.GetSourceChunk wrapper
  - GET `/v1/search/{query_id}/trace`（按 query_id 取已执行 search 的 trace）—— **新增 Rust SearchService.GetSearchTrace RPC** + Rust 端 SearchService.Query 执行时把 `RetrievalTrace` 持久化 by query_id（SQLite 新表 `search_traces` 或 in-memory LRU cache 1000）+ Go REST handler

来源：[ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D1 Wave 1+2 / D2 / D3 / D6 D7 / PRD §Implementation Phases v0.5 新增 (见 PRD §Implementation Phases Phase 12 行) / PRD §Open Questions O15 / O18 新增。

## 2. 业务价值

直接支撑 [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D1 v0.5.0 release 节奏，让 Console UI 端可以闭环以下场景：

- **Workspace 配置可视编辑**：Console UI 在 workspace 详情页可以改 allowlist/denylist（PATCH 端点支持）+ 服务端 X-Confirm 兜底保 deep defense
- **长任务过滤列表**：Console UI 「正在运行的索引任务」面板可以 filter status=active；当前 v0.4 没 list endpoint，UI 端要么轮询单 job 要么显示全量含 succeeded（噪音大）
- **Chunk 详情下钻**：search 结果列表 → 点击具体 chunk → 看完整内容（GET source-chunks/{id} 端点支持）—— PRD §Core Capabilities #2「可解释 RAG」的 UI 闭环
- **Search trace 复盘**：debug 搜索召回质量时可以反复看历史 trace（candidate_generation_steps / lexical_candidates_count / vector_candidates_count / rerank_steps / final_context_count）—— 真实接入度副指标支撑（PRD §Success Metrics 次要指标）
- **统一行为基线**：cancel 改 204 + X-Confirm 412 兜底 + RFC3339Nano kept = 4 个 trade-off 一次性锁定，避免 v0.5/v0.6/v0.7 三次 release 内行为漂移

不在本 phase scope 的 endpoint（[SPEC-DEFER:phase-13] 5 memory + [SPEC-DEFER:phase-14] 2 eval）留后续 phase ship。

## 3. 涉及模块

- `proto/contextforge/console_data_plane/v1/console_data_plane.proto`（修改：SearchService 加 `GetSourceChunk` + `GetSearchTrace` 两 RPC + 对应 message 类型 `GetSourceChunkRequest` / `GetSearchTraceRequest` / `SearchTrace` (即 contractv1 RetrievalTrace 的 proto 镜像)）
- `core/build.rs`（不修改：tonic_build 自动 pick up `.proto` 改动）
- `core/src/data_plane/search.rs`（修改：SearchServer 新增 `get_source_chunk` + `get_search_trace` 方法；SearchService.Query 执行时把 trace 持久化 by query_id）
- `core/src/data_plane/workspace.rs`（不修改：UpdateWorkspace 已存在）
- `core/src/data_plane/job.rs`（不修改：ListJobs 已存在）
- `core/src/data_plane/mod.rs`（修改：DataPlaneStores 持有 trace LRU cache 或新建 SearchTraceStore）
- `core/migrations/0012_search_traces.sql`（可选新增；trade-off：用 SQLite 表持久化 vs in-memory LRU；本 phase task-12.3 §10 trade-off 评估）
- `core/src/retriever/`（修改：retriever 加 `get_chunk_by_id(chunk_id: &str) -> Option<SourceChunk>` 方法；既有 SqliteChunkStore 已存）
- `internal/consoleapi/grpcclient/grpcclient.go`（修改：WorkspaceClient 加 `Update` 方法；JobClient 加 `ListActive` 方法；SearchClient 加 `GetSourceChunk` + `GetSearchTrace` 方法）
- `internal/consoleapi/types.go`（修改：Workspace/Search/Job Client 接口加 4 个新方法签名；新增 `SourceChunkClient` interface 或并入 SearchClient）
- `internal/consoleapi/router.go`（修改：注册 4 个新路由 + `confirmMiddleware` wrapper）
- `internal/consoleapi/handlers.go`（修改：加 `handlePatchWorkspaceConfig` / `handleListActiveJobs` / `handleGetSourceChunk` / `handleGetSearchTrace` 4 个新 handler；改 `handleCancelJob` 返 204；加 `confirmMiddleware`）
- `internal/consoleapi/memstore.go`（修改：MemStore 也实现新方法支持 env-gated fallback 行为；只对 Workspace.Update 实现持久化，其它新 method 直接返 `ErrDataPlaneUnavailable`，因为 in-memory 模拟搜索 trace / chunk by id 价值低）
- `core/tests/data_plane_integration.rs`（修改：加 6+ 集成测试 — `test_workspace_update_via_grpc` + `test_list_jobs_active_filter` + `test_get_source_chunk_via_grpc` + `test_get_search_trace_via_grpc` + `test_cancel_returns_204` 在 Go test + `test_x_confirm_412_when_missing`）
- `internal/consoleapi/e2e_grpc_test.go`（修改：v0.4 既有 E2E test 加 PATCH config / list active / source-chunk / search trace / 204 cancel / 412 confirm 6 个 sub-test step）
- `internal/consoleapi/router_test.go`（修改：handler 单元测试加 6 个 sub-test）
- `internal/consoleapi/grpcclient/grpcclient_test.go`（修改：4 新 client wrapper unit test + 412 sentinel mapping）
- `test/conformance/console_contractv1_test.go`（不修改：v0.4 已有 9 endpoint 反向 test PASS；本 phase 加 6 endpoint 不破坏 v0.4 既有 test fixture）
- `scripts/console_smoke.sh`（修改 v3：9 endpoint flow → 15 endpoint flow；加 step 10-15 patch config / list active / source-chunk / search trace / 204 cancel / 412 confirm）
- `scripts/release_smoke.sh`（不修改：第 5 段 `phase11_console_real` 仍 ok，v0.5 增量不破坏 v0.4）
- `docs/s2v-adapter.md`（修改：§Phases 加 Phase 12 行 / §Tasks 加 task-12.1/12.2/12.3 / §ADRs 加 ADR-017 / §BDD 加 console-contract-completion.feature）
- `docs/prds/context-forge.prd.md`（修改：§Implementation Phases 加 Phase 12 段 + §Open Questions O15/O18 新增）
- `test/features/console-contract-completion.feature`（新增：≥10 scenarios 覆盖 6 新 endpoint + 4 trade-off）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 12.1 | internal/consoleapi (router + handlers + grpcclient + confirmMiddleware) | `../tasks/task-12.1-quick-win-rest-endpoints.md` |
| 12.2 | core/src/retriever + core/src/data_plane/search.rs + Go REST | `../tasks/task-12.2-source-chunk-by-id.md` |
| 12.3 | core/src/data_plane/search.rs (trace persistence) + Go REST | `../tasks/task-12.3-search-trace-by-query-id.md` |

## 5. 依赖关系

- **依赖**：Phase 11（console-real-data-plane）— 复用 [ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) 4 个 gRPC service 框架 + tonic + prost 工具链 + `internal/consoleapi/grpcclient/` Go client wrapper pattern + `internal/consoleapi/` thin proxy 模式 + scripts/console_smoke.sh v2 REAL mode；Phase 10（console-contract-v1）— 复用 [ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md) `internal/contractv1/contractv1.go` 17 type 镜像（不动）；Phase 4（retrieval-explain）— 复用 task-4.1/4.2 retriever 框架（task-12.2 加 by-id lookup）
- **可并行**：否（v0.5 收口 phase）。Phase 内顺序：task-12.1（先 quick win 4 endpoint + confirmMiddleware；不引入 Rust 改动）→ task-12.2（Rust SearchService 加 GetSourceChunk + retriever by-id lookup + Go REST）→ task-12.3（Rust SearchService 加 trace 持久化 + GetSearchTrace + Go REST）。
- **Phase 内并行机会**：task-12.2 + task-12.3 都在 SearchService 上，可在 task-12.1 完成后并行（两 RPC 互不相交：GetSourceChunk 走 retriever，GetSearchTrace 走 trace store）；但 task-12.3 §6 AC2 trace 持久化需要 SearchService.Query 实现修改，与 task-12.2 GetSourceChunk 在同一 `search.rs` 文件，主 agent 选**串行实施**（v0.5 简化策略，避免合并冲突；并行收益 < 1 day）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（任务 12.1-12.3 全 Done，实测验证；每条 AC 含 ADR-014 D3 verified by 显式 owner）**：

- [x] AC1：`PATCH /v1/workspaces/{id}/config` body `{allowlist:[...], denylist:[...]}` 走 gRPC WorkspaceService.UpdateConfig + 返更新后 Workspace + 缺 X-Confirm header 或 ?confirm=true query → 412 Precondition Failed — **verified by task-12.1 §6 AC1 (`TestPatchWorkspaceConfig_{RequiresConfirm,AcceptsHeader,AcceptsQuery}`) + e2e_grpc Step 8a PASS + console_smoke.sh v3 Step 9 PASS (412 then 200)**
- [x] AC2：`GET /v1/index-jobs?status=active` 仅返 status in {queued, running}；其它 status 过滤掉；走 gRPC JobService.List + status filter — **verified by task-12.1 §6 AC2 (`TestListJobs_ActiveFilter` + `TestListJobs_MissingStatusFilter`) + console_smoke.sh v3 Step 10 PASS (active + missing→400)**
- [x] AC3：`POST /v1/index-jobs/{id}/cancel` 成功返 204 No Content（不返 body）；409 / 404 不变 — **verified by task-12.1 §6 AC3 (`TestCancelJob_Returns_204` + `TestCancelJob_404_unchanged` + `TestHandleCancelJob_409`) + e2e_grpc Step 9 真接 daemon 204 PASS**
- [x] AC4：`GET /v1/source-chunks/{id}` 走 gRPC SearchService.GetSourceChunk + retriever 复用 `Retriever::get_chunk(chunk_id)` (task-6.2 ship) + 不存在 chunk → 404 NOT_FOUND — **verified by task-12.2 §6 AC1 (`test_get_source_chunk_unknown_returns_not_found`) + e2e_grpc Step 9b PASS + console_smoke.sh v3 Step 11 PASS (chk_4eec0d18_2 found)**
- [x] AC5：`GET /v1/search/{query_id}/trace` 走 gRPC SearchService.GetSearchTrace + SearchService.Query 执行时把 RetrievalTrace 持久化 by query_id (in-memory TraceStore HashMap+VecDeque LRU cap=1000)；不存在 query_id → 404 — **verified by task-12.3 §6 AC1+AC2+AC3 (`test_query_persists_trace_by_query_id_and_get_returns_it` + `test_trace_store_eviction_at_capacity`) + e2e_grpc Step 9c PASS + console_smoke.sh v3 Step 12 PASS**
- [x] AC6：ADR-014 cross-validation gate 全套通过：D2 lint (`bash scripts/spec_drift_lint.sh --touched origin/master` 0 violation in PR-touched lines) + D3 phase §6 每条 AC 含 verified by + D1 closeout PR body 含 mapping 表 — **verified by closeout PR body (this PR) + D2 lint targeted grep on touched spec files (all anti-pattern hits annotated)**

**端到端 smoke**：

```bash
# step 1 — Phase 12 主集成 smoke (v3，含 15 endpoint flow)
bash scripts/console_smoke.sh
# 1) spawn contextforge-core daemon + gRPC :50552
# 2) spawn console-api-serve + REST :48181
# 3) curl 15 endpoint (v0.4 9 个 + v0.5 新 6 个): GET health + 4 workspace (含 PATCH config) + 4 index-job (含 list active + 204 cancel) + 2 search (含 source-chunks + trace) + 1 events + 4 trade-off verify (X-Confirm 412)
# 4) CONSOLE_REAL_SMOKE_EXIT=0

# step 2 — ADR-014 cross-validation gate (D2 lint)
bash scripts/spec_drift_lint.sh --touched origin/master

# step 3 — Release smoke (v0.5.0 release prep)
bash scripts/release_smoke.sh
# PHASE_RELEASE_SMOKE_EXIT=0 + phase11_console_real=ok 不退化
```

step 1 是 task-12.3 Gate 3 入口：spawn 双进程 + curl 15 endpoint + X-Confirm 412 兜底真校验 + search 真返 trace by query_id。`CONSOLE_REAL_SMOKE_EXIT=0` 是 final marker。

step 2 是 ADR-014 D2 lint gate：phase closeout PR 触及行无未标注 anti-pattern；强制 0 violation。

step 3 是 release_smoke.sh 端到端 hand-off：v0.5 增量后整脚本仍 `PHASE_RELEASE_SMOKE_EXIT=0`，证明 v0.5 ship gate 不退化。

**Scope 注**：本 phase smoke 与 task-8.3 / task-9.6 / task-10.6 / task-11.4 互补 — task-11.4 (v0.4) ship 9 endpoint REAL；task-12.3 (v0.5) 升级 console_smoke.sh 为 15 endpoint flow；task-13.2 (v0.6) 升级到 20 endpoint；task-14.2 (v0.7) 升级到 22 endpoint 全 PASS。四条 smoke 均跑通才允许 v0.5.0 tag。

## 7. 阶段级风险

- **关联 [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) §Rollback 8 条风险**：
  - D1 SoT 反向 / D3 thin proxy 违反 → 沿用 ADR-016 §自决规则 R8 立刻 STOP；revert + 从最近未违 commit 重启
  - D2 X-Confirm 412 兜底误伤 → 先扩 OR 语义到 header / query / Console User-Agent 三选一；若仍误伤 → ADR-017 amendment
  - D3 cancel 204 破坏老 client → 保留 200（Console HTTPAdapter v1.0 已 200/204 双 check，应不出现此问题，但保留 rollback 路径）
- **关联 task-12.3 trace 持久化策略 trade-off**：SQLite 表 vs in-memory LRU；前者持久跨 daemon 重启但写 IO + schema 演进成本；后者快 + 简单但重启即丢；本 phase task-12.3 §10 trade-off 评估 + 建议 in-memory LRU(1000) 起步 [SPEC-DEFER:task-future.search-trace-sqlite-persistence] 留 v0.5.x
- **关联 PRD §Technical Risks R1**（Go↔Rust gRPC 边界）：本 phase 新增 2 RPC（GetSourceChunk + GetSearchTrace）+ proto add-only 演进；按 ADR-013 既有规则维持
- **关联 PRD §Technical Risks R6**（大仓库索引性能）：trace 持久化 in-memory LRU 容量 1000 = QPS 上限考量；超 1000 query/min 时 LRU eviction 可能在 trace fetch 时 miss → 返 404；trade-off 接受（demo / single-user 场景足够）
- **关联 ADR-014 governance 第三次激活风险**：v0.3 首次 / v0.4 第二次跑通；v0.5 第三次验证制度稳定性；D2 lint anti-pattern 词表如出现误报扩散 → Phase 12 retrospective 评估

## 8. Phase Definition of Done

- [ ] 本 phase 全部 task spec Status=Done（12.1/12.2/12.3 全 Done — PR 顺序合）
- [ ] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过（console_smoke.sh 15 endpoint flow 真跑 + spec_drift_lint.sh --touched 0 violation + release_smoke.sh 第 5 段不退化）
- [ ] 关联风险 ADR-017 §Rollback 8 条 / R1 / R6 / ADR-014 治理风险缓解措施已落地
- [ ] adapter §Phase 状态索引该行 Status 同步更新（closeout PR）
- [ ] ADR-017 状态保持 Proposed（**Phase 14 closeout 时才推 Accepted**，因为 6 D-clauses 完整覆盖 3 phase；本 phase 仅 D1 Wave1+2 + D2 + D3 + D5 + D6 + D7 落入；D4 long-poll 沿用 v0.4 既有）
- [ ] PRD §Implementation Phases Phase 12 行新增（含 Status=Done / 描述 / 范围 / 依赖 / 可并行）+ §Open Questions O15 / O18 新增并标记 partially resolved
- [ ] **ADR-014 D1 mapping 表**：closeout PR body 含 Phase §6 ↔ Task §6 AC 映射（AC1-6 每行 4 字段：phase AC 字面 / 拥有 task or 验证方式 / task §6 AC 字面 / Evidence 链接）
- [ ] **ADR-014 D2 lint 输出**：closeout PR body 含 `bash scripts/spec_drift_lint.sh --touched origin/master` 0 violation 输出
- [ ] §4 Gate 4.5 ADR-014 cross-validation gate 通过 — v0.5.0 release tag prep ready
