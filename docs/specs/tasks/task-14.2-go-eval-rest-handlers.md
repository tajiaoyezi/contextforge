# Task `14.2`: `go-eval-rest-handlers — Console REST 2 eval endpoint + grpcclient.EvalClient + Go-side recall harness runner goroutine`

**Status**: Ready

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 14 (eval-rest-surface)
**Dependencies**: task-14.1 (Rust EvalService 3 RPC + SqliteEvalStore 已 ship) + task-13.2 (Go REST handler pattern + grpcclient wrapper pattern) + task-8.1 (internal/eval/eval.go recall harness) + [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D1 Wave 4 + [ADR-006](../../decisions/adr-006-recall-eval-acceptance-gate.md)

## 1. Background

task-14.1 已在 Rust 侧 ship 完整 EvalService gRPC（3 RPC: Create / Get / UpdateProgress）+ `eval_runs` 表 + SqliteEvalStore CRUD。本 task 在 Go 侧补完：
1. 2 个 REST endpoint：
   - `POST /v1/eval-runs` body `EvalRunCreate` → `EvalRun{status:"running"}` (创建新资源，非破坏性，不走 confirmMiddleware)
   - `GET /v1/eval-runs/{id}` → `EvalRun`
2. `grpcclient.EvalClient` 2 method wrapper（Create + Get；UpdateProgress 不在 Console contract，但 EvalRunner 内部调用需要 client）
3. **Go-side EvalRunner goroutine**：POST /v1/eval-runs 后异步 spawn goroutine 调既存 `internal/eval/eval.go` recall harness → 跑完后调 gRPC EvalService.UpdateProgress 反向 update store with metrics + case_results + status="succeeded"/"failed"
4. MemStore fallback：MemEvalStore in-memory + 模拟异步推进 status (mock metrics 直接返 0.5/0.7 之类 stub 值)[SPEC-OWNER:task-14.2]
5. `scripts/console_smoke.sh` v5（20 endpoint → 22 endpoint）含 eval 2 step；`scripts/release_smoke.sh` 加 `phase14_console_eval=ok` 子段
6. **Phase 14 closeout PR 内推 ADR-017 Proposed → Accepted**（6 D-clauses 完整覆盖 v0.5/v0.6/v0.7 3 phase）

至此 22/22 Console contract endpoint 全部 ship；Console HTTPAdapter 22-endpoint conformance suite 全 PASS = ContextForge ↔ Console v1.0 集成完整闭环。

## 2. Goal

Go `internal/consoleapi/` 加 2 个新 REST endpoint + Go-side EvalRunner goroutine；grpcclient.EvalClient 3 method wrapper（Create + Get + UpdateProgress for runner callback）；MemStore fallback 模式工作；`go test ./internal/consoleapi/...` 全绿；conformance test (TestConsoleContractV1Conformance) 全 22 endpoint 不退化；`bash scripts/console_smoke.sh` v5 REAL mode 22 endpoint flow `CONSOLE_REAL_SMOKE_EXIT=0`；release_smoke.sh 加 `phase14_console_eval=ok`；≥4 单元测试 + ≥1 集成测试 + 2 smoke sub-step PASS。

## 3. Scope

### In Scope

- **修改 `internal/consoleapi/types.go`**：
  - 加 `EvalClient` 接口 3 method：
    ```go
    type EvalClient interface {
        Create(req contractv1.EvalRunCreate) (contractv1.EvalRun, error)
        Get(evalRunID string) (*contractv1.EvalRun, error)  // nil if not found
        // UpdateProgress 是内部 RPC，仅 EvalRunner goroutine 调用；不在 Console REST 暴露
        UpdateProgress(evalRunID string, status string, metrics map[string]float64, caseResults []contractv1.CaseResult, errMsg string) error
    }
    ```
  - `Deps` 加 `Eval EvalClient` 字段
- **修改 `internal/consoleapi/router.go`**：
  - 路由注册：
    ```go
    mux.HandleFunc("POST /v1/eval-runs", handleCreateEvalRun(deps))
    mux.HandleFunc("GET /v1/eval-runs/{id}", handleGetEvalRun(deps))
    ```
  - 不走 confirmMiddleware（Eval create 是非破坏性 / 新资源创建）
- **修改 `internal/consoleapi/handlers.go`**：
  - 新增 2 handler：
    - `handleCreateEvalRun(deps)`: parse body `EvalRunCreate` → `deps.Eval.Create(req)` → 拿 EvalRun{status:"running"} → 异步 spawn `go runEvalAsync(deps, run.EvalRunID, req)` → 立刻返 200 + EvalRun
    - `handleGetEvalRun(deps)`: PathValue id → `deps.Eval.Get(id)` → 返 200 + EvalRun / 404
  - 新增 `runEvalAsync(deps Deps, evalRunID string, req EvalRunCreate)` goroutine：
    - 调既存 `internal/eval/eval.go` recall harness with req.config_snapshot + dataset_ref
    - harness 跑完 → 收集 metrics map (`recall@5`, `recall@10`, `precision@5` 等) + case_results array
    - 调 `deps.Eval.UpdateProgress(evalRunID, "succeeded", metrics, caseResults, "")`
    - 异常 → 调 `deps.Eval.UpdateProgress(evalRunID, "failed", nil, nil, err.Error())`
- **修改 `internal/consoleapi/grpcclient/grpcclient.go`**：
  - 加 `EvalClient` struct + `proto.EvalServiceClient` 字段
  - 加 3 method (Create / Get / UpdateProgress)，protoToEvalRun helper
- **修改 `internal/consoleapi/memstore.go`**：
  - 加 `MemEvalStore` struct + `runs map[string]contractv1.EvalRun`
  - `Create(req)` [SPEC-OWNER:task-14.2]: 写 in-memory EvalRun{status:"running", started_at:now} + spawn goroutine 模拟 2s 后改 status="succeeded" + mock metrics + 返 EvalRun
  - `Get(id)`: 返 *EvalRun or nil
  - `UpdateProgress(id, ...)`: in-memory update
- **修改 `internal/cli/console_api_serve.go`**：
  - `buildDeps` helper 加 Eval client 构造（grpc / fallback / degraded modes）
- **修改 `scripts/console_smoke.sh`** v5：
  - step 22 → step 23 共 2 新 step:
    - step 22: POST /v1/eval-runs body `{workspace_id, config_snapshot:{}, dataset_ref:"test/fixtures/eval-seed/golden_questions.jsonl"}` → 拿 eval_run_id
    - step 23: poll GET /v1/eval-runs/<id> 每 1s 直到 status terminal（≤60s）；succeeded → 验证 metrics 含 `recall@5` 字段 + case_results 非空
- **修改 `scripts/release_smoke.sh`**：第 6 段加 `phase14_console_eval=ok` 子检查（runs phase-14 specific smoke step）
- **新增 `test/fixtures/eval-seed/golden_questions.jsonl`**：5 行 fixture golden_questions（query + expected_chunks + category 等）覆盖 task-8.1 recall harness 输入 schema
- **单元测试 ≥4**（`internal/consoleapi/handlers_test.go` + `grpcclient/grpcclient_test.go` + `eval_runner_test.go`）：
  - `TestCreateEvalRun_Returns_200_with_running`
  - `TestGetEvalRun_404_when_missing`
  - `TestEvalRunner_RecallHarness_UpdatesProgress_to_succeeded` (mock recall harness + verify UpdateProgress called with metrics)[SPEC-OWNER:task-14.2]
  - `TestEvalClient_Create_Maps_Errors`
- **集成测试 ≥1**：
  - `internal/consoleapi/e2e_grpc_test.go::TestEvalEndpoints_E2E_GrpcBacked` (spawn Rust daemon + Go console-api-serve + POST eval-run + poll 60s 等 status terminal + verify metrics + case_results)
- **conformance test**：`test/conformance/console_contractv1_test.go` 既有 9 endpoint test 不动；Console 端 22-endpoint conformance suite 反向跑（env CONSOLE_REPO 设时）现在应全 PASS（v0.7.0 ship 的标志）
- **文件锚点**：`internal/consoleapi/{types,router,handlers,memstore}.go` + `internal/consoleapi/eval_runner.go` (新增 — runEvalAsync 实现) + `internal/consoleapi/grpcclient/grpcclient.go` + `internal/cli/console_api_serve.go` + `scripts/console_smoke.sh` v5 + `scripts/release_smoke.sh` + `test/fixtures/eval-seed/`
- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **Rust EvalService impl** [SPEC-OWNER:task-14.1]
- **POST /v1/eval-runs/{id}/cancel** (Console 22-endpoint 不含；如未来加 [SPEC-DEFER:console-eval-cancel])
- **GET /v1/eval-runs** list endpoint (Console 22-endpoint 不含 list；仅 single-get + create) [SPEC-DEFER:console-eval-list]
- **Eval progress SSE / WebSocket 实时推送** [SPEC-DEFER:console-eval-progress-sse]
- **dataset CRUD endpoints** [SPEC-DEFER:console-dataset-management]
- **golden_questions 验证 / dataset_ref 路径校验加固** [SPEC-DEFER:phase-future.eval-dataset-validation]

## 4. Users / Actors

- **Console UI 端**（下游，via cross-repo）：Eval 面板触发 + 查看 metrics / case_results
- **Phase 14 closeout PR 主 agent**：本 task 是 Phase 14 收口任务；下一步 Phase 14 closeout PR 推 ADR-017 Proposed → Accepted + v0.7.0 release tag

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/decisions/adr-017-console-contract-completion-22-endpoint.md` §D1 Wave 4 / §D6
- `docs/decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md` §D3 / §D4
- `docs/decisions/adr-006-recall-eval-acceptance-gate.md`
- `docs/specs/phases/phase-14-eval-rest-surface.md` §3 / §6
- `docs/specs/tasks/task-14.1-rust-eval-grpc-service.md` (EvalService 3 RPC 接口)
- `docs/specs/tasks/task-13.2-go-memory-rest-handlers.md` (REST handler pattern)
- `docs/specs/tasks/task-8.1-eval-harness.md` (recall harness 入口与 schema)
- `H:/devlopment/code/contextforge/internal/contractv1/contractv1.go::EvalRun, EvalRunCreate, CaseResult`
- `H:/devlopment/code/contextforge/internal/eval/eval.go`

### 5.2 Imports

- **Go**: 现有 stdlib `net/http` + `encoding/json` + `context` + `time`；现有 `internal/consoleapi/grpcclient/` + `internal/eval/`；现有 `google.golang.org/grpc`
- **不引入新依赖**：R7 不触发

### 5.3 EvalRunner goroutine 形状

```go
// internal/consoleapi/eval_runner.go
func runEvalAsync(deps Deps, evalRunID string, req contractv1.EvalRunCreate) {
    ctx, cancel := context.WithTimeout(context.Background(), 5*time.Minute)
    defer cancel()

    // 1. Call existing recall harness
    result, err := evalpkg.RunRecall(ctx, evalpkg.Config{
        WorkspaceID: req.WorkspaceID,
        ConfigSnapshot: req.ConfigSnapshot,
        DatasetRef: req.DatasetRef,
    })

    // 2. Convert result + Update progress
    if err != nil {
        _ = deps.Eval.UpdateProgress(evalRunID, "failed", nil, nil, err.Error())
        return
    }
    metrics := map[string]float64{
        "recall@5":  result.Recall5,
        "recall@10": result.Recall10,
        "precision@5": result.Precision5,
    }
    caseResults := convertCaseResults(result.Cases)
    _ = deps.Eval.UpdateProgress(evalRunID, "succeeded", metrics, caseResults, "")
}
```

## 6. Acceptance Criteria

- [ ] AC1：`POST /v1/eval-runs` body `EvalRunCreate{workspace_id, config_snapshot, dataset_ref}` → 走 gRPC EvalService.Create → 立刻返 200 + EvalRun{status:"running", started_at:now}；server-side `runEvalAsync` goroutine 异步 spawn 触发 recall harness — **verified by unit-test `TestCreateEvalRun_Returns_200_with_running` + integration `TestEvalEndpoints_E2E_GrpcBacked` step create PASS**
- [ ] AC2：`GET /v1/eval-runs/{id}` 真返 EvalRun（含 status / metrics / case_results / config_snapshot）；不存在 → 404；status lifecycle 在 recall harness 完成后 (≤60s for small fixture) 真持久化到 succeeded 或 failed — **verified by integration `TestEvalEndpoints_E2E_GrpcBacked` step poll_until_terminal PASS**
- [ ] AC3：`runEvalAsync` goroutine 跑完 recall harness 后调 `deps.Eval.UpdateProgress(...)` 反向 update Rust SqliteEvalStore；metrics map 含 `recall@5` / `recall@10` 等键；case_results 数组每项含 case_id/query/expected_chunks/actual_chunks/score/passed — **verified by unit-test `TestEvalRunner_RecallHarness_UpdatesProgress_to_succeeded` + integration verify GET 后字段填实 PASS**
- [ ] AC4 [SPEC-OWNER:task-14.2]：MemStore fallback 模式（`CONSOLE_API_FALLBACK_INMEM=1`）POST → 返 stub EvalRun + goroutine 2s 后 mock 推进到 succeeded with mock metrics（`recall@5: 0.7`）；GET 返该 stub；conformance test 22 endpoint 全 PASS（含 v0.4/v0.5/v0.6 既有 20 endpoint）— **verified by go test fallback mode + conformance suite PASS**
- [ ] AC5：scripts/console_smoke.sh v5 22 endpoint flow `CONSOLE_REAL_SMOKE_EXIT=0`（含 eval create + poll 60s + GET 验证 metrics 含 recall@5）；scripts/release_smoke.sh `phase14_console_eval=ok` `PHASE_RELEASE_SMOKE_EXIT=0` 全段 — **verified by §9 verify run + smoke exit 0**
- [ ] AC6：v0.4 + v0.5 + v0.6 既有 20 endpoint + task-14.2 新 2 endpoint = **Console 22-endpoint conformance 100% PASS**（双方握手成功标志）— **verified by `CONSOLE_REPO=<path> go test -run TestConsoleContractV1Conformance ./test/conformance/...` 反向跑 22 endpoint 全过 + closeout PR body 引用证据**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | POST /v1/eval-runs → 200 running + spawn goroutine | handlers.go + eval_runner.go + tests | Ready |
| AC2 | GET /v1/eval-runs/{id} + lifecycle | handlers.go + integration | Ready |
| AC3 | runEvalAsync 真调 recall + UpdateProgress | eval_runner.go + tests | Ready |
| AC4 | MemStore fallback works for eval + conformance不退化 | memstore.go + go test | Ready |
| AC5 | scripts/console_smoke.sh v5 22 endpoint exit 0 + release_smoke.sh phase14_console_eval=ok | scripts + integration | Ready |
| AC6 | Console 22-endpoint conformance 100% PASS | test/conformance/ + closeout PR evidence | Ready |

## 8. Risks

- **Go-side goroutine 而非 Rust spawn_blocking 路径**：trade-off task-14.1 §10 选了 Go-side；本 task 实施在 console-api-serve 进程内；console-api-serve crash 时 in-flight eval run 状态丢失（status 永卡 running）→ 缓解：Rust 侧 orphan reaper（与 task-11.3 既有 JobRunner orphan reaper 同款）扫 eval_runs status=running 超 10min 不更新 → mark failed [SPEC-DEFER:phase-15.eval-orphan-reaper]
- **recall harness 内部错误传播**：`internal/eval/eval.go::RunRecall` 错误形式 (golden_questions 路径无效 / workspace 未索引 / dataset 解析失败) 需要规范化到 EvalRun.error_message；缓解 task implementation 第一步 grep `internal/eval/eval.go::RunRecall` 入口签名 + 错误类型；如错误类型多样 → 统一 wrap to string
- **dataset_ref 路径校验**：Console contract `EvalRunCreate.dataset_ref` 是 string；不校验路径存在性 → recall harness 跑时才发现路径无效 → status="failed" + error_message；trade-off 接受（不在 Create 时同步校验避免 REST 阻塞）
- **5min context timeout** in runEvalAsync：5min 对小 dataset 充分；大 dataset (1000+ questions) 可能超时 → 缓解 task implementation 加 `?timeout=<duration>` query param 让 Console UI 端可控；本 task 默认 5min 起步
- **Eval lifecycle 期望「running → succeeded」直接跳过 cancelled / failed 易遗漏**：3 状态都必须可达；缓解 unit test 覆盖 3 状态 transition；mock recall harness 测试时分别返 success / panic / context cancelled [SPEC-OWNER:task-14.2]

## 9. Verification Plan

- **install**: `go mod download`
- **lint**: `gofmt -l internal/consoleapi/`
- **typecheck**: `go build ./...`
- **unit-test**: `go test -v ./internal/consoleapi/... ./internal/consoleapi/grpcclient/...`（≥4 新单测 + 既有不退化）
- **integration**: `go test -v -run TestEvalEndpoints_E2E_GrpcBacked ./internal/consoleapi/...` + `CONSOLE_REPO=<path> go test -v -run TestConsoleContractV1Conformance ./test/conformance/...` (22 endpoint 全 PASS)
- **e2e**: 通过 integration + `bash scripts/console_smoke.sh` v5 REAL mode 22 endpoint flow
- **build**: `go build ./cmd/contextforge`
- **coverage**: 不强制
- **runtime-smoke**: `bash scripts/console_smoke.sh` REAL mode + manual curl POST /v1/eval-runs + poll GET 验证 status terminal
- **manual**: curl POST + 5s 后 GET 验证 metrics 含 recall@5; verify Console 端 conformance suite all PASS

## 10. Completion Notes

<!-- 完工时按 standard.md §8.3 6 项 schema 回填 -->

- **完成日期**：<待填>
- **改动文件**：
  - `internal/consoleapi/types.go` (修改 — EvalClient 接口 3 method + Deps 加 Eval)
  - `internal/consoleapi/router.go` (修改 — 2 路由)
  - `internal/consoleapi/handlers.go` (修改 — 2 新 handler — handleCreateEvalRun + handleGetEvalRun)
  - `internal/consoleapi/eval_runner.go` (新增 — runEvalAsync goroutine)
  - `internal/consoleapi/memstore.go` (修改 — MemEvalStore 3 method + 2s mock goroutine)[SPEC-OWNER:task-14.2]
  - `internal/consoleapi/grpcclient/grpcclient.go` (修改 — EvalClient struct + 3 method wrapper + protoToEvalRun helper)
  - `internal/cli/console_api_serve.go` (修改 — buildDeps 加 Eval client 构造)
  - `internal/consoleapi/router_test.go` (修改 — 加 unit test)
  - `internal/consoleapi/handlers_test.go` (修改 — 加 2+ unit test)
  - `internal/consoleapi/eval_runner_test.go` (新增 — TestEvalRunner_* unit test)
  - `internal/consoleapi/grpcclient/grpcclient_test.go` (修改 — 加 1 unit test)
  - `internal/consoleapi/e2e_grpc_test.go` (修改 — TestEvalEndpoints_E2E_GrpcBacked)
  - `scripts/console_smoke.sh` (修改 v5 — 20 → 22 endpoint + 2 eval steps)
  - `scripts/release_smoke.sh` (修改 — 第 6 段加 phase14_console_eval=ok)
  - `test/fixtures/eval-seed/golden_questions.jsonl` (新增 — 5 行 fixture)
  - `docs/specs/tasks/task-14.2-go-eval-rest-handlers.md` (本 spec §6 / §7 / §10 / Status 推进)
- **commit 列表**：
  - feat(consoleapi+grpcclient): task-14.2 — 2 eval REST endpoint + grpcclient.EvalClient + Go-side runEvalAsync goroutine + recall harness orchestration + smoke v5 + 22-endpoint contract complete
  - docs(spec): task-14.2 §6/§7/§10 / Status → Done
- **§9 Verification 结果**：<待填>
- **剩余风险 / 未做项**：
  - Eval orphan reaper [SPEC-DEFER:phase-15.eval-orphan-reaper]
  - Eval cancel REST endpoint [SPEC-DEFER:console-eval-cancel]
  - Eval list REST endpoint [SPEC-DEFER:console-eval-list]
  - Eval progress SSE [SPEC-DEFER:console-eval-progress-sse]
  - Dataset CRUD [SPEC-DEFER:console-dataset-management]
- **下游 task 影响**：本 task 是 Phase 14 收口 + v0.7.0 release ship 入口；Phase 14 closeout PR 推 ADR-017 Status → Accepted + v0.7.0 tag
