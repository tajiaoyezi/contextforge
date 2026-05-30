# Task `19.4`: `smoke-v9 — scripts/console_smoke.sh 28→30 step（step 29 = /v1/search?semantic=true roundtrip；step 30 = eval --semantic）+ internal/cli/eval.go --semantic CLI flag 接 task-18.8 SummarizeHybrid + MeetsRecallGate`

**Status**: Done

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 19 (vector-retrieval-integration)
**Dependencies**: task-19.3（`SearchRequest.semantic` add-only proto 字段 + Rust gRPC semantic 路径 + Go `/v1/search?semantic=true` query param 通路）/ task-19.2（默认 backend 接 `Retriever::with_vector_searcher` 生产热路径）/ task-19.1（`EmbeddingProvider` trait + deterministic 缺省 provider）/ task-18.8（`internal/eval` `SummarizeHybrid` + `MeetsRecallGate` + `SemanticRecallAtK` 度量）/ ADR-006 Amendment A1（SemanticRecall@10 ≥ 0.70 门禁）/ ADR-013（禁伪造凭据）/ ADR-014 D1-D5 第十次激活

## 1. Background

Phase 19 把 Phase 18 的向量 backend 基础设施推到生产语义检索（phase-19 §1）。task-19.3 已把 `semantic` 加成 `SearchRequest` 的 add-only 字段并把 `/v1/search?semantic=true` query param 通到 Rust gRPC semantic 路径；task-19.2 已把选定默认 backend 接进 `Retriever` 热路径。但端到端 smoke（`scripts/console_smoke.sh`）当前止于 v8 step 28（`[28/28]` task-17.1 is_pinned roundtrip），尚未覆盖语义检索通路；CLI eval 入口（`internal/cli/eval.go`，phase-19 §3.4 模块标号写作 `cmd/contextforge/eval.go`，真实代码路径是 `internal/cli/eval.go` + 生产 daemon 注入在 `cmd/contextforge/main.go`）只调用 `evalpkg.Summarize`（BM25 单路），未消费 task-18.8 落地的 `SummarizeHybrid` + `MeetsRecallGate` 双路度量。

task-18.8 已把 `SemanticRecall@K` 度量 + `Report` semantic 字段 + `SummarizeHybrid(bm25, semantic []Result)` + `MeetsRecallGate(report)` 门禁落进 `internal/eval/eval.go`（数学正确性 + 单测齐备），但 CLI live 路径当时标 [SPEC-OWNER:phase-future.vector-retrieval-integration] 后置（task-18.8 §3 R3）。本 phase 该 owner 已解锁：task-19.1/19.2/19.3 提供 embedding provider + 生产向量召回 + semantic gRPC 通路。

因此本 task 做两件事：(a) `scripts/console_smoke.sh` v9 把 28 step 扩到 30 step——step 29 验 `/v1/search?semantic=true` REST→gRPC 语义通路 roundtrip，step 30 验 `contextforge eval run --semantic` 端到端跑通；(b) `internal/cli/eval.go` 加 `--semantic` flag，BM25 路径之外再跑一遍 semantic 检索（`SearchRequest.Semantic=true`），用 `SummarizeHybrid` 双路汇总 + `MeetsRecallGate` 门禁判定，把 task-18.8 的 library 度量真正接到 CLI 入口。

ADR-013 红线：本 task 是 smoke + CLI wiring，不产出召回数值结论。真实 `SemanticRecall@K` 实测 + ADR-023 ratify 是 task-19.5/19.6 的职责。deterministic 缺省 provider 在两平台均可跑通 wiring/smoke（无模型 dep）；真实 recall 数值若受平台/模型门槛影响，由 task-19.5 诚实记录，本 task 的 smoke step 30 只断言 CLI 退出码 + 双路 Report 字段成形，不预判阈值通过。

## 2. Goal

`scripts/console_smoke.sh` 升 v9：现有 28 step 不退化 + 新增 step 29（`/v1/search?semantic=true` roundtrip：REAL 模式断言响应为 `{result, trace}` 嵌套且 `retrieval_method` 反映语义路径，semantic 通路通）+ step 30（`contextforge eval run --semantic`：构建 Go 二进制后跑 eval，断言 stdout 同时含 BM25 与 semantic 双路汇总行 + gate 判定行，退出码 0）。`internal/cli/eval.go` 加 `--semantic` flag：开启时对每题再发一次 `SearchRequest{Semantic:true}` 检索，`SummarizeHybrid(bm25Results, semanticResults)` 双路汇总，`MeetsRecallGate(report)` 判定门禁并打印 `gate=pass|fail` + failing checks；未开启时维持 BM25-only 既有行为（`Summarize`，向后兼容 task-8.1）。≥3 Go 测试（CLI flag parse + eval semantic 双路路径 + smoke `bash -n` 语法）全 PASS。默认 `go test ./...` + `cargo test --workspace` 不退化；D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **修改 `internal/cli/eval.go`**：
  - `evalRunOpts` 加 `Semantic bool` 字段。
  - `parseEvalRunOpts` 加 `--semantic`（默认 false）flag 解析。
  - `runEval`：BM25 路径循环（既有）产 `bm25Results`；当 `opts.Semantic` 为真时，对同一题集再循环一次发 `SearchRequest{Query, Collections, TopK, Explain:true, Semantic:true}` 产 `semanticResults`。
  - 汇总改走 `evalpkg.SummarizeHybrid(bm25Results, semanticResults)`（`--semantic` 关闭时 `semanticResults` 为 nil → `SummarizeHybrid` 退回 BM25-only，与既有 `Summarize` 输出等价）。
  - 打印：既有 BM25 行不变；`report.SemanticEvaluated` 为真时追加 `semantic_recall_at_5=` / `semantic_recall_at_10=` / `semantic_strong_hits_top5=` / `semantic_strong_hits_top10=` 行；末尾追加 `gate=pass`（或 `gate=fail` + 每条 failing check 一行）由 `evalpkg.MeetsRecallGate(report)` 给出。
- **修改 `internal/cli/eval_test.go`**：新增 ≥2 Go 单测——(1) `parseEvalRunOpts` 解析 `--semantic` 置 `Semantic=true`，缺省为 false；(2) `runEval --semantic` 用注入的 `fetchSearchResults`（按 `req.Semantic` 分别返回 BM25/semantic fake 结果）跑通，stdout 含 `semantic_recall_at_10=` + `gate=` 行，且 `fetchSearchResults` 被调用 60 次（30 题 × 2 路）。
- **修改 `scripts/console_smoke.sh`**：v9 注释段 + step header 从 `[N/28]` 迁到 `[N/30]`（含 20-endpoint 流头部与 v6/v7/v8 段标号同步）+ 新增 step 29（semantic search roundtrip）+ step 30（eval `--semantic` 端到端）+ 终态 marker 之前插入两 step。
  - step 29（REAL 模式）：`POST /v1/search?semantic=true`（query param）断言响应仍为 `{result, trace}` 嵌套（add-only 不破坏既有 shape）；非 REAL 模式打印 SKIP 说明（deterministic 缺省 provider 下空 trace 亦可接受）。
  - step 30（REAL 模式）：复用 step 99-103 构建的 Go 二进制（`$GO_BIN`）跑 `"$GO_BIN" eval run --semantic --collection=default`，断言 stdout 含 `total=` + `semantic_recall_at_10=` + `gate=` 行且退出码 0（ADR-013：不断言具体召回阈值，只断言双路成形 + CLI 通路）。
- **新增 `internal/cli/smoke_syntax_test.go`**（或并入 `eval_test.go` 同包 Go 测试）：用 `exec.Command("bash", "-n", "scripts/console_smoke.sh")` 跑 bash 语法检查，断言退出码 0；`bash` 不在 PATH 时 `t.Skip`（Windows dev 机 Git Bash 可选）。

### Out of Scope（[SPEC-DEFER] / [SPEC-OWNER]）

- **真实 `SemanticRecall@K` 数值实测 + dogfood embedding 语料** [SPEC-OWNER:task-19.5-real-recall-eval]：本 task 的 step 30 与 CLI `--semantic` 只验证双路 Report 成形 + 门禁判定执行 + CLI 退出码；真实召回数值由 task-19.5 用 real provider + dogfood 语料产出。
- **ADR-023 Proposed→Accepted ratify** [SPEC-OWNER:task-19.6-adr-023-ratify]：须 task-19.5 真实数据（ADR-013 禁据合成 ratify）。
- **proto `SearchRequest.semantic` 字段 + Rust gRPC semantic 路径 + Go `/v1/search?semantic=true` handler** [SPEC-OWNER:task-19.3-semantic-search-api]：本 task 消费该通路并在 smoke 验证，不实现它。
- **默认 backend 生产 wiring + index/query embedding** [SPEC-OWNER:task-19.2-default-backend-wiring]：本 task 依赖其落地。
- **`EmbeddingProvider` trait + deterministic 缺省 provider + real provider** [SPEC-OWNER:task-19.1-spike-embedding-provider]：本 task 的 smoke/CLI 在该 provider 之上跑。
- **golden questions 语义近邻标注扩充** [SPEC-DEFER:phase-future.semantic-golden-dataset]：现 30 题为 BM25 口径（承 task-18.8 §3）；语义标注后置不影响本 task 的双路 wiring。
- **Phase 19 收口 v0.12.0 release docs + smoke v9 final 定稿** [SPEC-OWNER:task-19.7-closeout-v0.12.0]：本 task 落 smoke v9 30-step；release evidence + README/RELEASE_NOTES v0.12 段在 closeout。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`internal/cli/eval.go`（`runEval` / `parseEvalRunOpts`）**：CLI eval 入口，本 task 加 `--semantic` 双路。
- **`internal/eval`（`SummarizeHybrid` / `MeetsRecallGate` / `SemanticRecallAtK`）**：task-18.8 落地的度量库，本 task 由 CLI 首次消费。
- **`scripts/console_smoke.sh`**：端到端 C1 集成兜底 smoke，本 task 升 v9 30-step。
- **`fetchSearchResults`（`internal/cli/search.go` 注入 hook）**：CLI 检索 backend；生产由 `cmd/contextforge/main.go` `SetSearchBackend(searchViaDaemon)` 注入，测试由 `eval_test.go` 直接覆写。
- **上游 task-19.1/19.2/19.3**：提供 embedding provider + 生产向量召回 + semantic gRPC 通路。
- **下游 task-19.5/19.7**：task-19.5 在本 CLI/smoke 之上跑真实 recall；task-19.7 收口 smoke v9 final。

## 5. Behavior Contract

### 5.1 Required Reading

- `scripts/console_smoke.sh`（当前 v8 step 结构：20-endpoint 流 `[N/20]` + v6/v7/v8 段 `[N/28]`，终态 marker `CONSOLE_REAL_SMOKE_EXIT=0`）
- `internal/eval/eval.go`（task-18.8 `SummarizeHybrid` / `MeetsRecallGate` / `SemanticRecallAtK` / `Report` semantic 字段 / `GateSemanticRecall10Min=0.70`）+ `internal/eval/eval_test.go`
- `internal/cli/eval.go` + `internal/cli/eval_test.go`（既有 `runEval` / `parseEvalRunOpts` / `evalRunOpts`）
- `internal/cli/search.go`（`fetchSearchResults` 注入 hook + `SearchBackend` 签名）+ `cmd/contextforge/main.go`（`searchViaDaemon` 生产注入）
- `proto/contextforge/v1/search.pb.go`（`SearchRequest` 字段；task-19.3 追加 `Semantic bool`）
- `internal/consoleapi/handlers.go`（`handleSearch` @ `/v1/search`，task-19.3 接 `semantic` query param）
- `docs/specs/tasks/task-18.8-eval-semantic-recall.md`（度量来源）+ `docs/specs/tasks/task-19.3-semantic-search-api.md`（semantic 通路）+ `docs/specs/tasks/task-19.2-default-backend-wiring.md`（backend wiring）
- `docs/decisions/adr-006-recall-eval-acceptance-gate.md` Amendment A1（门禁阈值）+ `docs/decisions/adr-013-*.md`（禁伪造）+ `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5）

### 5.2 关键设计 — CLI `--semantic` 双路

- `runEval` 既有 BM25 循环不动：每题发 `SearchRequest{Query, Collections, TopK, Explain:true}`（无 semantic flag），收 `bm25Results []evalpkg.Result`。
- `opts.Semantic` 为真时，第二循环每题发 `SearchRequest{Query, Collections, TopK, Explain:true, Semantic:true}`，收 `semanticResults []evalpkg.Result`；为假时 `semanticResults` 留 nil。
- `report := evalpkg.SummarizeHybrid(bm25Results, semanticResults)`：nil semantic → `SemanticEvaluated=false`，输出与既有 `Summarize` 等价（向后兼容 task-8.1 既有测试 `TestTask81_AC3_AC5`）。
- `ok, failures := evalpkg.MeetsRecallGate(report)`：BM25 两阈值（0.75/0.85）恒检；SemanticRecall@10（0.70）仅 `SemanticEvaluated` 时检（task-18.8 设计）。打印 `gate=pass` 或 `gate=fail` + 每条 failing check。
- ADR-013：CLI 退出码不绑定 gate 结果（gate fail 仍 exit 0，结果打印供人判读）；smoke step 30 只断言 CLI 退出 0 + 双路字段成形，不预判阈值通过——真实数值由 task-19.5 评测。

### 5.3 关键设计 — smoke v9 step 29 / step 30

- **step 29**（`[29/30]`，REAL 模式）：`search_body=$(curl -sf -X POST "$BASE/v1/search?semantic=true" -d '{"query":"contextforge","workspace_id":"...","top_k":5,"agent_scope":"session"}')`；断言响应仍嵌套 `"result"` + `"trace"`（task-19.3 add-only 不破坏既有 22-endpoint contract）。非 REAL 模式打印 SKIP（deterministic 缺省 provider 下空 trace 可接受）。
- **step 30**（`[30/30]`，REAL 模式）：`"$GO_BIN" eval run --semantic --collection=default`（`$GO_BIN` 由现有 `[3/4] go build` 步构建）；断言 stdout 含 `total=` + `semantic_recall_at_10=` + `gate=` 且退出码 0。失败时 tail core.log/api.log 便于诊断。非 REAL 模式打印 SKIP。
- step header 全量迁号：`[N/20]` 流头部说明 + v6 段 `[21/28]`…`[24/28]` + v7 段 `[25/28]`…`[27/28]` + v8 段 `[28/28]` 一律改成 `/30` 分母；终态 marker `CONSOLE_REAL_SMOKE_EXIT=0` / `CONSOLE_SMOKE_EXIT=0` 保留在最末。

### 5.4 不变量

- `--semantic` 关闭时 `runEval` 行为与现状逐字节等价（既有 `TestTask81_AC3_AC5` 仍绿，30 calls 不变）。
- smoke 既有 28 step 断言不改语义，仅迁分母 + 追加两 step。
- proto / REST contract 由 task-19.3 守 add-only；本 task 不新增 proto 字段。

## 6. Acceptance Criteria

- [x] **AC1**: `internal/cli/eval.go` `parseEvalRunOpts` 解析 `--semantic` flag（默认 false，给定置 true）；`evalRunOpts.Semantic` 字段正确填充 — verified by **TEST-19.4.1**（flag 缺省 false + `--semantic` → true）
- [x] **AC2**: `runEval --semantic` 对每题发 BM25 + semantic 两次检索（`fetchSearchResults` 调 60 次 = 30 题 × 2 路），走 `evalpkg.SummarizeHybrid` 双路汇总，stdout 含 `semantic_recall_at_10=` + `gate=` 行；`--semantic` 关闭时维持 BM25-only（30 calls，输出与既有 `Summarize` 等价） — verified by **TEST-19.4.2**（注入 fake backend 按 `req.Semantic` 分流 + 输出断言 + 向后兼容）
- [x] **AC3**: `runEval --semantic` 调 `evalpkg.MeetsRecallGate(report)` 并打印 `gate=pass`/`gate=fail`（+ failing checks）；CLI 退出码不绑定 gate 结果（ADR-013：结果供判读，不预判阈值） — verified by **TEST-19.4.2**（gate 行出现 + gate-fail 仍 exit 0）
- [x] **AC4**: `scripts/console_smoke.sh` v9 通过 `bash -n` 语法检查（exit 0）；step header 全量迁 `[N/28]`→`[N/30]`，新增 step 29（`/v1/search?semantic=true` roundtrip）+ step 30（`eval run --semantic` 端到端） — verified by **TEST-19.4.3**（`bash -n` exit 0 + step 标号 grep）
- [x] **AC5**: 既有不退化 — `go test ./...` 全 PASS（含新增 3 Go 测试 + 既有 `TestTask81_AC3_AC5` 仍绿）；`cargo test --workspace` 不受影响（本 PR 零 Rust delta，CI cargo-test gate 复核） — verified by **TEST-19.4.4**（`go test ./...` 0 failed）+ §10 实测
- [x] **AC6**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched master` PR 触及行 0 未标注命中 — verified by **TEST-19.4.5**（D2 lint 实跑输出）+ §10 记录

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-19.4.1 | `--semantic` flag parse（缺省 false / 给定 true） | `internal/cli/eval_test.go` | Done（`TestTask194_AC1_ParseSemanticFlag` PASS） |
| TEST-19.4.2 | `runEval --semantic` 双路（60 calls + SummarizeHybrid 输出 + gate 行 + BM25-only 向后兼容） | `internal/cli/eval_test.go` | Done（`TestTask194_AC2_AC3_RunEvalSemanticDualPath` PASS） |
| TEST-19.4.3 | smoke v9 `bash -n` 语法 + step 29/30 标号 | `internal/cli/smoke_syntax_test.go` | Done（`TestTask194_AC4_SmokeV9SyntaxAndSteps` PASS，`bash -n` exit 0） |
| TEST-19.4.4 | `go test ./...` 0 failed（含既有 task-8.1 eval 测试不退化） | 全 Go | Done（`go test ./...` 全 PASS，`TestTask81_AC3_AC5` 仍绿） |
| TEST-19.4.5 | D2 lint `--touched master` 0 未标注命中 | `scripts/spec_drift_lint.sh` | Done（见 §10） |

## 8. Risks

- **R1（中）真实召回数值受 embedding provider 平台/模型门槛**（承 phase-19 §7 R1）：real provider 在 Windows MSVC / CI 可能受阻 → smoke step 30 与 CLI `--semantic` 在 deterministic 缺省 provider 下跑通通路，但召回数值不具评测意义。
  - **缓解**：step 30 与 CLI 只断言双路 Report 成形 + CLI 退出码，不预判 gate 阈值通过（ADR-013）；真实数值 [SPEC-OWNER:task-19.5-real-recall-eval] 用 real provider 产出。stop-condition：semantic 通路在 deterministic provider 下亦不通 → 回看 task-19.2/19.3 wiring，不在本 task 强造结果。
- **R2（中）semantic 检索使每题双发请求，30 题 eval 延时翻倍**：CLI `--semantic` 跑 60 次检索。
  - **缓解**：`--semantic` 默认关闭（既有 30-call BM25 路径不变）；smoke step 30 复用已 warm 的 daemon，30 题 fixture 体量小，延时可控；大语料评测属 task-19.5。
- **R3（低）smoke step 迁号触及既有 28 行断言文本**：批量改 `/28`→`/30` 易误伤断言逻辑。
  - **缓解**：仅改 step header 的分母字符串与注释，不动 curl 断言体；`bash -n` + REAL smoke 复跑守语义不退化；既有 step 26 daemon restart / step 28 is_pinned 断言逐行核对。
- **R4（低）`bash -n` 测试在无 bash 的 Windows dev 机不可跑**：本机 Git Bash 可选。
  - **缓解**：测试 `bash` 不在 PATH 时 `t.Skip`；CI（Linux）必跑该语法检查 → 不漏网。

## 9. Verification Plan

```bash
# Go：CLI --semantic 双路 + flag parse + smoke 语法
go vet ./internal/cli/...
go test ./internal/cli/... -run 'TestTask194|TestTask81' -v
go test ./...

# smoke v9 语法 + 标号
bash -n scripts/console_smoke.sh
grep -c '\[29/30\]\|\[30/30\]' scripts/console_smoke.sh   # 期望 ≥ 2

# 端到端 REAL smoke（Linux / WSL2 / Git Bash）— 需 cargo + go 工具链
bash scripts/console_smoke.sh        # 期望末行 CONSOLE_REAL_SMOKE_EXIT=0

# Rust 不退化 + D2 lint
cargo test --workspace
bash scripts/spec_drift_lint.sh --touched master
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**：2026-05-30
- **改动文件**：`internal/cli/eval.go`（`--semantic` flag + `evalSearchPass` 抽出 BM25/semantic 单趟 + 双路 `SummarizeHybrid` + `MeetsRecallGate` gate 行）、`internal/cli/eval_test.go`（`TestTask194_AC1_ParseSemanticFlag` flag parse + `TestTask194_AC2_AC3_RunEvalSemanticDualPath` 60-call 双路 + gate=fail 仍 exit 0 + BM25-only 30-call 向后兼容）、`internal/cli/smoke_syntax_test.go`（新增 `TestTask194_AC4_SmokeV9SyntaxAndSteps`：`bash -n` 语法 + `[29/30]`/`[30/30]` 标号 + 无残留 `/28]`）、`scripts/console_smoke.sh`（v9 30-step：header 段 + 8 个 `[N/28]`→`[N/30]` 迁号 + step 29 `/v1/search?semantic=true` 合约保形 + step 30 `eval run --semantic` 双路报告）、`docs/specs/tasks/task-19.4-smoke-v9.md`（本 spec）、`docs/s2v-adapter.md`（19.4 行 Done）
- **commit 列表**：见本 task PR（分支 `feat/task-19.4-smoke-v9`）；合入后以 merge commit 为准
- **§9 Verification 结果**：`go vet ./internal/cli/...` clean；`go test ./internal/cli -run 'TestTask194|TestTask81' -v` 4/4 PASS（含既有 `TestTask81_AC3_AC5` 不退化）；全 `go test ./...` 0 failed；`bash -n scripts/console_smoke.sh` exit 0（经 `TestTask194_AC4` + 本地 Git Bash 双跑）；`grep -c '\[29/30\]\|\[30/30\]'` ≥ 2；本 PR 仅改 Go + bash（零 Rust delta），`cargo test --workspace` 不受影响（CI cargo-test gate 复核）；D2 lint `--touched master` 0 未标注命中（见下）。端到端 REAL smoke（`bash scripts/console_smoke.sh` 末行 `CONSOLE_REAL_SMOKE_EXIT=0`）需 cargo+go 工具链的 WSL/Linux 环境，由 CI / task-19.7 closeout 复跑定稿。
- **设计取舍（诚实记录）**：smoke 用 `console-api-serve`（`internal/consoleapi`），其 `handleSearch` 仅解码 JSON body、**不转发** `?semantic=true` query param（task-19.3 只接了 `internal/daemon/rest.go` 那条 REST surface）。故 step 29 按 spec §5.3 仅断言 add-only query param **不破坏** `{result, trace}` 22-endpoint 合约（保形），**不声称**语义检索经 console-api 真正生效。真正的语义通路由 step 30 的 CLI `eval run --semantic` 走（`searchViaDaemon` → proto `SearchRequest{Semantic:true}` → core gRPC semantic 分支，绕过 console-api）。ADR-013：step 30 跑空 transient 索引 → 召回数值无意义，仅断言双路报告成形 + gate 行 + exit 0。
- **剩余风险 / 未做项**：(1) console-api `/v1/search` 转发 `?semantic=true` 到 gRPC `SearchRequest.Semantic`（+ `contractv1.SearchRequest` add `Semantic` 字段）未做——经 console-api 的真实语义 REST 召回属 [SPEC-OWNER:task-19.5-real-recall-eval]（task-19.5 真实 recall 评测会需要这条通路或直接走 CLI）。(2) 真实 `SemanticRecall@K` 数值 + ADR-023 ratify 见 [SPEC-OWNER:task-19.5-real-recall-eval] / [SPEC-OWNER:task-19.6-adr-023-ratify]。(3) smoke v9 final 定稿 + release evidence 见 [SPEC-OWNER:task-19.7-closeout-v0.12.0]。
- **下游 task 影响**：task-19.5（在本 CLI `--semantic` + smoke v9 之上跑真实 recall 评测；并补 console-api semantic 转发 if 经 REST 评测）；task-19.7（Phase 19 closeout 收口 smoke v9 final + v0.12.0 release docs）
