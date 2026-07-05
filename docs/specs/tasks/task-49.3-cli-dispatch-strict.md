# Task `49.3`: `cli-dispatch-strict — eval CLI dispatch + --strict flag`

**Status**: Done
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 49 (eval-hardening)
**Dependencies**: task-49.1（golden-retrieval.jsonl 已存在测 strict-path）/ task-49.2（golden-semantic.jsonl 已扩展测 fallback-path）/ ADR-014
**Required Reading**: phase-49-eval-hardening.md / internal/cli/eval.go（runEval + parseEvalRunOpts + dispatch 点 :42）/ internal/eval/eval.go（ValidateDataset:183-209 + ValidateGoldenSemantic:231-280）

## 1. Background
`internal/cli/eval.go:42` 无条件调 `ValidateDataset`（≥30/≥6cat/≥5每cat），导致 `--dataset=golden-semantic.jsonl`（16 题 2 cat）CLI 跑不了（fail ≥30 检查）。`ValidateGoldenSemantic`（count-agnostic）只在测试用。v1.1 扩展了两个 golden 文件，需让 CLI 都能跑。

## 2. Goal
(1) 改 `internal/cli/eval.go:42`：`ValidateDataset` 失败时降级 `ValidateGoldenSemantic`，两套都失败才报错（honest error message 说明两套 validator 各自要求）。
(2) 新增 `--strict` flag：强制 `ValidateDataset`（不降级），给 CI/benchmark 场景用。
(3) `--help` 更新说明 dispatch 行为 + `--strict`。

## 3. Scope
- 改 `internal/cli/eval.go`：
  - `parseEvalRunOpts` 加 `strict bool` flag 解析
  - `runEval` dispatch 逻辑：`strict=true` → 只跑 ValidateDataset；`strict=false`（默认）→ ValidateDataset 失败降级 ValidateGoldenSemantic，都失败报错
  - error message 区分两套 validator 的失败原因（帮用户理解为何被拒）
- 改 `internal/cli/eval.go` help 文本：说明 dispatch + `--strict`
- 加 `internal/cli/smoke_syntax_test.go` 或 `eval_test.go`：`TestTask493_*`
  - strict-pass：golden-retrieval.jsonl + `--strict` → 过
  - soft-fallback-pass：golden-semantic.jsonl（无 --strict）→ 降级过
  - soft-fallback-fail-with-strict：golden-semantic.jsonl + `--strict` → fail（行为正确）
  - both-fail：空文件/缺字段 → 两套都 fail

## 4.1 行为契约
- **default（无 --strict）**：`--dataset=X` → 尝试 ValidateDataset → 失败则 ValidateGoldenSemantic → 都失败 exit 1 + error
- **--strict**：`--dataset=X` → 只 ValidateDataset → 失败 exit 1 + error
- **无 --dataset**：仍用 BuiltinGoldenQuestions()（30 题，过 ValidateDataset，行为不变）
- **exit code**：gate 仍软门（runEval 永远 exit 0 when validation passes；validation fail 才 exit 1）—— 本 task 不改 gate 行为，只改 validation dispatch

## 6. AC
- [x] **AC1**: `contextforge eval run --dataset=golden-semantic.jsonl` 现在可跑（之前 fail ≥30）— verified by **TEST-49.3.1**
- [x] **AC2**: `--strict` flag 强制 ValidateDataset；golden-semantic.jsonl + `--strict` 仍 fail（行为正确）— verified by **TEST-49.3.2**
- [x] **AC3**: dispatch 三路径覆盖（strict-pass / soft-fallback-pass / both-fail）— verified by **TEST-49.3.3**

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-49.3.1 | golden-semantic.jsonl 无 --strict 可跑（降级 ValidateGoldenSemantic） | go test | Done |
| TEST-49.3.2 | golden-semantic.jsonl + --strict fail（强制 ValidateDataset） | go test | Done |
| TEST-49.3.3 | dispatch 三路径（strict-pass / fallback-pass / both-fail） | go test | Done |

## 9. Verification
```bash
go test ./internal/cli/ -run TestTask493 -v
go vet ./internal/cli/ && gofmt -l internal/cli/
# 手动验证（如 daemon 可起）
# contextforge eval run --dataset=test/fixtures/eval/golden-semantic.jsonl  # 应过（降级）
# contextforge eval run --dataset=test/fixtures/eval/golden-semantic.jsonl --strict  # 应 fail
```

## 10. Completion Notes
**Status**: Done

1. **完成日期**：2026-07-05
2. **改动文件**：
   - internal/cli/eval.go（evalRunOpts +Strict 字段 / parseEvalRunOpts +--strict flag / runEval dispatch 逻辑 / usage 字符串）
   - internal/cli/eval_test.go（+TestTask493_AC0/AC1/AC2/AC3 四测试）
3. **commit 列表**：
   - `7712cca` test(cli): task-49.3 RED — TestTask493 strict flag + dispatch 三路径测试
   - `7064b7f` feat(cli): task-49.3 GREEN — eval dispatch + --strict flag
4. **§9 Verification 结果**：
   - lint: ✅（gofmt clean / go vet ✅）
   - typecheck: N/A
   - unit-test: 4 passed / 0 failed（TestTask493_AC0/AC1/AC2/AC3 全 PASS）+ full cli package no-regression ✅
5. **剩余风险**：dispatch 降级路径在 ValidateDataset 失败时多一次 ValidateGoldenSemantic 调用（微秒级，性能可忽略）；error message 含两套 validator 的失败原因便于用户诊断
6. **下游影响**：task-49.4（spike 依赖 CLI 能跑 golden 文件验证 wiring）
