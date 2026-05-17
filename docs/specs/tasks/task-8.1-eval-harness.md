# Task `8.1`: `eval-harness — golden questions + recall eval (contextforge eval run)`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 8 (eval-and-reliability)
**Dependencies**: Phase 6 (cli-api-export), Phase 7 (mcp-adapter)

## 1. Background

召回评测是 ContextForge 核心能力 #4 与 PRD §Decisions Log D6（recall eval 作为 PRD 级一等验收门）。本 task 实现 `contextforge eval run`，按 PRD §Success Metrics Eval Measurement Protocol 口径产出报告。

## 2. Goal

`contextforge eval run` 加载 golden questions 数据集（≥ 30 条，每类 ≥ 5：配置定位/错误复现/历史决策/日志排查/Agent memory·rule/代码位置），对索引跑检索，按 Strong/Weak/Miss 规则输出 Top-5/Top-10 主命中率（仅 Strong）、延迟（不含 embedding/远程）、错误召回样例；可导出 eval dataset JSONL。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Success Metrics Eval Measurement Protocol / §Decisions Log D6 / §Implementation Phases Phase 8 Exit Criteria）
- `docs/specs/phases/phase-8-eval-and-reliability.md`
- `docs/specs/tasks/task-4.2-explain.md`
- `docs/specs/tasks/task-6.1-cli-search.md`
- `docs/decisions/adr-006-recall-eval-acceptance-gate.md`
- `test/features/eval.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Success Metrics Eval Measurement Protocol): golden questions 数据集 ≥ 30 条、每类 ≥ 5 条（6 类），每条含 query/expected_sources/expected_file_path/expected_line_range|expected_chunk_id/category/notes。
- [ ] **AC2** (PRD §Success Metrics 命中规则): 按 Strong/Weak/Miss 判定；Top-5/Top-10 主命中率只统计 Strong hit，Weak hit 单独报告。
- [ ] **AC3** (PRD §Implementation Phases Phase 8 Exit Criteria): `contextforge eval run` 输出 Top-5/Top-10、latency、miss cases。
- [ ] **AC4** (PRD §Success Metrics 主指标计算方式): 延迟指标不含 embedding/reranker/远程 provider API 调用时间。
- [ ] **AC5** (PRD §Constraints 兼容性导出): 可导出 Eval dataset JSONL，便于回归与外部工具兼容。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 golden ds ≥30/每类≥5 | SCEN-8.1.1 | TEST-8.1.1 | - | unit-test | Not Started |
| AC2 Strong/Weak/Miss 规则 | SCEN-8.1.2 | TEST-8.1.2 | - | unit-test | Not Started |
| AC3 eval run 输出报告 | SCEN-8.1.3 | TEST-8.1.3 | - | unit-test | Not Started |
| AC4 延迟不含远程 | SCEN-8.1.4 | TEST-8.1.4 | - | unit-test | Not Started |
| AC5 导出 eval JSONL | SCEN-8.1.5 | TEST-8.1.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R3**（召回率）：分场景统计先达标再看总分。关联 PRD §Open Questions **O6**（golden questions 数据集构建与维护：谁标注/覆盖场景/防过拟合）。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。

## 10. Completion Notes

- **完成日期**：`<TBD-after-impl>`
- **改动文件**：`<TBD-after-impl>`
- **commit 列表**：`<TBD-after-impl>`
- **§9 Verification 结果**：
  - install: `<TBD-after-impl>`
  - typecheck: `<TBD-after-impl>`
  - unit-test: `<TBD-after-impl>`
- **剩余风险 / 未做项**：`<TBD-after-impl>`
- **下游 task 影响**：`<TBD-after-impl>`
