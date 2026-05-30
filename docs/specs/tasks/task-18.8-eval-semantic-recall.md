# Task `18.8`: `eval-semantic-recall — internal/eval SemanticRecall@K 度量 + 双路 SummarizeHybrid + recall gate + ADR-006 amendment`

**Status**: Done

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 18 (vector-backend-selection)
**Dependencies**: task-18.1（vector trait）/ task-18.7（ADR-023 选型）/ ADR-006（recall-eval-acceptance-gate，本 task amend）/ ADR-014 D1-D5 第十四次激活 / Phase 8 task-8.1（既有 BM25 eval harness）

## 1. Background

ADR-006 的 recall-eval 验收门当前为 **BM25-only**（Strong/Weak/Miss + Top-5/10 命中率，`internal/eval/eval.go`，Phase 8 task-8.1）。Phase 18 让 retriever 具备向量召回（task-18.1 trait + task-18.3–18.6 backend + ADR-023 选型），故 eval 口径需扩为 **BM25 + Semantic 双路**，并定义 `SemanticRecall@10 ≥ 0.70` 门禁（phase-18 §AC5 + ADR-006 amendment）。

关键限制：仓内无 embedding provider（`[SPEC-DEFER:phase-future.embedding-provider-full]`），向量 backend 未接入生产 retriever（`[SPEC-OWNER:phase-future.vector-retrieval-integration]`，ADR-023 D6），合成种子向量 recall 不可区分（task-18.7）。因此本 task 落地**度量 + 门禁 + 单测**（数学正确性），live 语义召回值与正式 ratify 待真实 embedding provider 后置（ADR-013 不伪造）。

## 2. Goal

`internal/eval/eval.go` 加 `SemanticRecall@K`（K=5,10）度量 + `Report` semantic 字段 + `SummarizeHybrid` 双路汇总 + `MeetsRecallGate` 门禁（BM25 恒检 + SemanticRecall@10 仅在 semantic 路径有结果时检）；`eval_test.go` ≥3 单测验证数学 + 门禁 + 空 semantic 退 BM25；ADR-006 add-only amendment 定义 SemanticRecall@10 ≥ 0.70 门禁（provisional）。`go test ./...` + `cargo test --workspace` 不退化；D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **修改 `internal/eval/eval.go`**：
  - `Report` 加 `SemanticEvaluated bool / SemanticStrongHits{5,10} int / SemanticWeakHits int / SemanticMisses int / SemanticRecallAt{5,10} float64`。
  - 门禁常量 `GateTop5StrongMin=0.75 / GateTop10StrongMin=0.85 / GateSemanticRecall10Min=0.70`。
  - `SemanticRecallAtK(results, k) float64`（top-K strong 命中率）。
  - `SummarizeHybrid(bm25, semantic []Result) Report`（双路汇总；无 semantic → BM25-only `SemanticEvaluated=false`）。
  - `MeetsRecallGate(report) (bool, []string)`（BM25 恒检；SemanticRecall@10 仅 `SemanticEvaluated` 时检）。
- **修改 `internal/eval/eval_test.go`**：4 单测（SemanticRecall@K 数学 K=5/10 / SummarizeHybrid 双路 / 空 semantic 退 BM25 + 门禁 / 门禁阈值 BM25+semantic）。
- **修改 `docs/decisions/adr-006-recall-eval-acceptance-gate.md`**：add-only `## Amendment A1` 段（SemanticRecall@K 度量 + 阈值表 + provisional 限制），不改既有 Decision。
- **修改 `docs/s2v-adapter.md`**：Phase 18 表 18.8 行 → Done。

### Out of Scope（[SPEC-DEFER] / [SPEC-OWNER]）

- **真实 embedding provider** [SPEC-DEFER:phase-future.embedding-provider-full]：fastembed/candle/ONNX 不在本 task；live 语义召回值待其落地。
- **向量 backend 接入生产 retriever 热路径 + CLI `--semantic` live 路径** [SPEC-OWNER:phase-future.vector-retrieval-integration]（ADR-023 D6）：需 embedding pipeline；本 task 仅度量 + 门禁 + 单测，CLI live 路径后置。
- **ADR-023 D1 默认 backend ratify** [SPEC-OWNER:phase-future.vector-retrieval-integration]：须真实 embedding 复测后。
- **golden questions 语义标注扩充** [SPEC-DEFER:phase-future.semantic-golden-dataset]：现 30 题为 BM25 口径，语义近邻标注后置。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`internal/eval`**：recall-eval harness，本 task 加语义双路。
- **ADR-006**：验收门 source of truth，本 task add-only amend。
- **下游 phase-future.vector-retrieval-integration**：接入真实 embedding + 生产向量召回 → 用本 task 度量 ratify ADR-023。

## 5. Behavior Contract

### 5.1 Required Reading

- `internal/eval/eval.go` + `eval_test.go`（既有 BM25 harness）
- `docs/decisions/adr-006-recall-eval-acceptance-gate.md`（amend 对象）+ `adr-023-vector-backend-default.md`（D6 wiring 后置）
- `docs/specs/phases/phase-18-vector-backend-selection.md`（§AC5 SemanticRecall gate）

### 5.2 关键设计

- `SemanticRecall@K` 复用既有 Strong-hit 判定（`Result.Outcome==Strong && MatchedRank<=K`），对向量路径 `[]Result` 计算；weak 不计入 recall。
- 双路：`SummarizeHybrid` 先 `Summarize(bm25)` 填 BM25 字段，再填 semantic 字段；空 semantic 即 BM25-only。
- 门禁分离：BM25 两阈值恒检；SemanticRecall@10 仅 `SemanticEvaluated` 时检 → 生产 BM25-only 现状不被语义门误伤。

## 6. Acceptance Criteria

- [x] **AC1**: `SemanticRecallAtK(results, k)` 计算 top-K strong 命中率正确（weak/miss 不计；空集 0）— verified by **TEST-18.8.1**（K=5 → 2/5；K=10 → 3/5；nil → 0）
- [x] **AC2**: `SummarizeHybrid` 同时填 BM25 + semantic 字段；`Report` semantic 字段（strong@5/10 / recall@5/10 / Evaluated）正确 — verified by **TEST-18.8.2**
- [x] **AC3**: 空 semantic 结果 → `SemanticEvaluated=false`（BM25-only 退回）；门禁不强制 semantic 项 — verified by **TEST-18.8.3**
- [x] **AC4**: `MeetsRecallGate` BM25 两阈值（0.75/0.85）恒检 + SemanticRecall@10（0.70）仅 `SemanticEvaluated` 时检 — verified by **TEST-18.8.4**
- [x] **AC5**: ADR-006 add-only amendment 定义 SemanticRecall@10 ≥ 0.70 门禁 + provisional 限制（embedding provider 后置）— verified by `docs/decisions/adr-006-*.md` §Amendment A1
- [x] **AC6**: 既有不退化 — `go test ./...` 全 PASS（含新 4 单测）；`cargo test --workspace` 不受影响；D2 lint `--touched master` 0 未标注命中 — verified by §10 实测

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-18.8.1 | SemanticRecall@K 数学（K=5/10/空） | `internal/eval/eval_test.go` | Done |
| TEST-18.8.2 | SummarizeHybrid 双路字段 | `internal/eval/eval_test.go` | Done |
| TEST-18.8.3 | 空 semantic 退 BM25-only + 门禁 | `internal/eval/eval_test.go` | Done |
| TEST-18.8.4 | MeetsRecallGate BM25 + semantic 阈值 | `internal/eval/eval_test.go` | Done |
| TEST-18.8.5 | go test ./... 0 failed | 全 Go | Done |

## 8. Risks

- **R1（高）live 语义召回值不可得**：无 embedding provider，向量 backend 未接生产 retriever。
  - **缓解**：本 task 仅度量 + 门禁 + 单测（数学），阈值 aspirational；live + ratify 后置 [SPEC-OWNER:phase-future.vector-retrieval-integration]；ADR-006 amendment 明记 provisional。
- **R2（中）ADR-006 amend 触历史 ADR**：ADR-006 为 Phase 5/8 期。
  - **缓解**：add-only `## Amendment A1` 段（不改既有 Decision），承 phase-18 spec 明文授权 + adr-022 amend 先例；docs/decisions 不入 D2 lint 范围。
- **R3（低）CLI `--semantic` 未落**：phase-18 module 列了 CLI flag，但 `cmd/contextforge/eval.go` 不存在且 live 路径需生产向量召回。
  - **缓解**：CLI live 路径 [SPEC-OWNER:phase-future.vector-retrieval-integration] 后置；本 task 落 library 度量 + 门禁（CLI 接入时直接调用）。

## 9. Verification Plan

```bash
go vet ./internal/eval/...
go test ./internal/eval/... -run TestTask188 -v
go test ./...
cargo test --workspace
bash scripts/spec_drift_lint.sh --touched master
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**：2026-05-30
- **改动文件**：`internal/eval/eval.go`（semantic 度量 + 门禁）、`internal/eval/eval_test.go`（4 单测）、`docs/decisions/adr-006-recall-eval-acceptance-gate.md`（add-only Amendment A1）、`docs/s2v-adapter.md`（18.8 行 Done）、`docs/specs/tasks/task-18.8-eval-semantic-recall.md`（本 spec）
- **commit 列表**：见本 task PR（分支 `feat/task-18.8-eval-semantic-recall`）；合入后以 merge commit 为准
- **§9 Verification 结果**：`go test ./internal/eval` 4 新单测 PASS + 全 `go test ./...` 绿；`cargo test --workspace` 不受影响；D2 lint 0 命中
- **剩余风险 / 未做项**：live 语义召回 + ratify + CLI `--semantic` 后置（embedding provider + 生产向量召回 wiring）
- **下游 task 影响**：task-18.9（Phase 18 closeout 引本 task + ADR-023）；phase-future.vector-retrieval-integration（用本度量 ratify）
