# Task `49.1`: `golden-retrieval-jsonl — 大语料 retrieval golden（6 cats × ~20 题 ≈120 题）`

**Status**: Ready
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 49 (eval-hardening)
**Dependencies**: v1.0.2（已 ship）/ ADR-006 / ADR-013 / ADR-014（第四十一次激活）
**Required Reading**: phase-49-eval-hardening.md / internal/eval/eval.go（ValidateDataset:183-209 + knownCategories:214-223）/ 现有 BuiltinGoldenQuestions()（:111-181，6 cats × 5 题模板）

## 1. Background
现有 CLI eval 只能跑 `ValidateDataset`（≥30 题 / ≥6 categories / ≥5 每 category），但 `golden-semantic.jsonl`（16 题 2 cat）过不了 → CLI 只能用 30-question builtin。v1.1 要用更大 golden 验证 recall，需新增一个过 `ValidateDataset` 的大文件。

## 2. Goal
新增 `test/fixtures/eval/golden-retrieval.jsonl`：6 个 builtin categories（config-location / error-reproduction / historical-decision / log-troubleshooting / agent-memory-rule / code-location），每类 ~20 题，总 ~120 题。查询基于真实 contextforge 源码设计（internal/ / core/src/ / docs/decisions/），覆盖比 builtin 5 题/类更多文件和更细粒度。

## 3. Scope
- 新增 `test/fixtures/eval/golden-retrieval.jsonl`：~120 行 JSONL
- 每行 schema 同 `Question` struct（eval.go:29-37）：`{query, expected_file_path, expected_line_range:{start,end}, category, notes?, expected_sources?, expected_chunk_id?}`
- 6 categories 全部 ∈ `knownCategories`（无需扩集，用现有 6 个 builtin name）
- 每类 ~20 题，line_range 用 `{start:0,end:0}`（整文件匹配，与现有 golden-semantic 一致）或具体行范围
- 加 `internal/eval/eval_test.go`：`TestTask491_*`（ValidateDataset 通过 + category 分布 + 无 dup query）
- RED→GREEN：先写测试断言文件存在且过 ValidateDataset（RED：文件不存在 fail），再填 golden（GREEN）

## 4.1 设计约束（诚实）
- **expected_file_path 必须 ground-truth 准确**：每题的 expected_file_path 必须真实存在于仓库（grep 可验）。争议 case（多个候选文件）在 notes 标注选择理由
- **查询多样性**：避免同义重复（如 "where is config" / "config location" / "find config" 算 3 题但太近）—— 每题应测不同检索维度（不同文件 / 不同 code path / 不同概念）
- **不伪造 recall**：本 task 只产 golden 数据，不跑 recall（task-49.4 跑）。expected 是 ground-truth 不是预测 recall

## 6. AC
- [ ] **AC1**: golden-retrieval.jsonl ≥120 题 / 6 categories / 每类 ≥5 → `ValidateDataset` 通过 — verified by **TEST-49.1.1**
- [ ] **AC2**: 所有 category ∈ knownCategories（config-location/error-reproduction/historical-decision/log-troubleshooting/agent-memory-rule/code-location）— verified by **TEST-49.1.2**
- [ ] **AC3**: 无 duplicate query；每题 expected_file_path 非空且文件真实存在 — verified by **TEST-49.1.3**

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-49.1.1 | golden-retrieval.jsonl 过 ValidateDataset（≥120/6cat/≥5） | go test | Not Started |
| TEST-49.1.2 | category ∈ knownCategories（6 builtin） | go test | Not Started |
| TEST-49.1.3 | 无 dup query + expected_file_path 真实存在 | go test + grep | Not Started |

## 9. Verification
```bash
# ValidateDataset 通过
go test ./internal/eval/ -run TestTask491 -v
# category 分布
go test ./internal/eval/ -run TestTask491_CategoryDistribution -v
# expected_file_path 真实存在（每题的文件 grep 得到）
# （在测试里 os.Stat 或 git ls-files 验证）
# go vet + gofmt
go vet ./internal/eval/ && gofmt -l internal/eval/
```

## 10. Completion Notes
**Status**: Ready

1. **完成日期**：<TBD-after-impl>
2. **改动文件**：<TBD-after-impl>
3. **commit 列表**：<TBD-after-impl>
4. **§9 Verification 结果**：<TBD-after-impl>
5. **剩余风险**：<TBD-after-impl>
6. **下游影响**：task-49.3（CLI dispatch 用此文件测降级路径反向）/ task-49.4（spike 读此文件）
