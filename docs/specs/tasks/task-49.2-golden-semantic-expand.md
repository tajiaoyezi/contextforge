# Task `49.2`: `golden-semantic-expand — golden-semantic.jsonl 扩展（code-symbol + cjk 16→~80 题）`

**Status**: Done
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 49 (eval-hardening)
**Dependencies**: task-49.1（同 phase 内，无强 dep 但建议先做 49.1 摸清 golden 设计范式）/ ADR-029 / ADR-035 / ADR-046 / ADR-013 / ADR-014
**Required Reading**: phase-49-eval-hardening.md / internal/eval/eval.go（ValidateGoldenSemantic:231-280 + knownCategories:214-223）/ test/fixtures/eval/golden-semantic.jsonl（现状 16 题）/ docs/spikes/phase-19-real-recall.md（CJK recall 范式）

## 1. Background
现有 `golden-semantic.jsonl` 16 题（6 code-symbol + 10 cjk）。CJK 10 题全是"bigram vs true-seg 对抗 case"（adversarial），缺多词短语/CJK+ASCII 混合/自然语言长句/code-CJK 标识符覆盖。ADR-035 task-30.2 实测 bigram vs true-seg delta=0（小语料无差别），但更大 CJK 语料可能暴露真实差别。code-symbol 6 题覆盖 snake_case/PascalCase/dotted.path 不够全。

## 2. Goal
扩展 `golden-semantic.jsonl` 16 → ~80 题（~40 code-symbol + ~40 cjk），保持 code-symbol + cjk 两类（语义纯度，不强加 ≥6 categories）。过 `ValidateGoldenSemantic`（count-agnostic + knownCategories 闭集，无需扩集——code-symbol/cjk 已在集内）。

## 3. Scope
- 改 `test/fixtures/eval/golden-semantic.jsonl`：16 → ~80 行
- **CJK 扩展（10→~40）**：
  - 多词 CJK 短语（`向量检索后端工厂` / `控制面数据面分离`）
  - CJK+ASCII 混合（`向量检索backend` / `gRPC跨进程`）
  - 自然语言长句（`为什么选择双 binary 架构` / `如何配置向量后端`）
  - code-CJK 标识符（`IndexSession` 旁注中文 / `BruteForceVectorBackend`）
- **code-symbol 扩展（6→~40）**：
  - 更多 snake_case（`resolve_vector_backend` / `select_vector_backend` / `build_tantivy_schema`）
  - PascalCase（`RetrieverConfig` / `VectorIndexConfig` / `DataPlaneStores`）
  - dotted.path（`contextforge.eval.run` / `retriever.vector.traits`）
  - kebab-case（`code-cjk` / `vector-backend`）
- 加 `internal/eval/eval_test.go`：`TestTask492_*`（ValidateGoldenSemantic 通过 + code-symbol/cjk 分布 + CJK 扩展覆盖新维度）

## 4.1 设计约束（诚实）
- **保持 knownCategories 闭集不扩**：code-symbol + cjk 已在集内，本 task 不加新 category（避免污染闭集语义）
- **CJK 新维度覆盖**：每题 notes 标注测的是什么维度（多词/混合/长句/标识符），便于 task-49.4 spike 分析
- **expected_file_path ground-truth**：同 task-49.1，文件必须真实存在
- **不跑 recall**：本 task 只产数据，CJK recall delta 由 task-49.4 spike 实测

## 6. AC
- [x] **AC1**: golden-semantic.jsonl ~80 题（~40 code-symbol + ~40 cjk）→ `ValidateGoldenSemantic` 通过 — verified by **TEST-49.2.1**
- [x] **AC2**: CJK 扩展覆盖新维度（多词短语 / CJK+ASCII 混合 / 自然语言长句 / code-CJK 标识符，每维度 ≥3 题）— verified by **TEST-49.2.2**
- [x] **AC3**: 无 dup query；每题 expected_file_path 真实存在 — verified by **TEST-49.2.3**

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-49.2.1 | golden-semantic.jsonl ~80 题过 ValidateGoldenSemantic | go test | Done |
| TEST-49.2.2 | CJK 新维度覆盖（多词/混合/长句/标识符 各≥3） | go test | Done |
| TEST-49.2.3 | 无 dup + expected_file_path 真实存在 | go test + grep | Done |

## 9. Verification
```bash
go test ./internal/eval/ -run TestTask492 -v
go vet ./internal/eval/ && gofmt -l internal/eval/
```

## 10. Completion Notes
**Status**: Done

1. **完成日期**：2026-07-05
2. **改动文件**：
   - test/fixtures/eval/golden-semantic.jsonl（扩展 16→76 题）
   - internal/eval/eval_test.go（+TestTask492_AC1/AC2/AC3 三测试 + strings import）
3. **commit 列表**：
   - `f350ed2` test(eval): task-49.2 RED — TestTask492_AC1/AC2/AC3
   - <GREEN> feat(eval): task-49.2 GREEN — golden-semantic.jsonl 扩展 76 题（code-symbol 35 + cjk 41）
4. **§9 Verification 结果**：
   - lint: ✅（gofmt clean on eval_test.go）
   - typecheck: N/A（go vet ✅）
   - unit-test: 3 passed / 0 failed（TestTask492_AC1/AC2/AC3 全 PASS）+ full eval package no-regression ✅
5. **剩余风险**：CJK 扩展覆盖 4 新维度（multi-word 5 / cjk-ascii-mix 7 / natural-lang 4 / code-cjk-id 4），但 recall delta 实测在 task-49.4；若 bigram vs true-seg 在大语料仍 delta=0 说明 bigram 默认正确
6. **下游影响**：task-49.4（CJK spike 读此文件对比 bigram vs true-seg）
