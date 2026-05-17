# Task `5.2`: `lifecycle — stale 标记 + 基础冲突检测`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 5 (memoryops)
**Dependencies**: 5.1 (dedup)

## 1. Background

MemoryOps 生命周期治理：过期标记 + 基础冲突检测，避免过期/冲突上下文污染 Agent（PRD §Core Capabilities #3 / §Problem Statement 痛点 4）。能力边界按 PRD「v0.1 MemoryOps 能力边界」：stale = expires_at / source deleted / source modified；冲突仅检测同 key/path/tag 明显冲突，不做 LLM 语义判断。

## 2. Goal

`memoryops` 支持 stale 标记（`expires_at` 到期 / source deleted / source modified）可被设置与检索；基础冲突检测（同一 key / path / tag 下明显冲突给出提示），不做语义冲突判断。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Core Capabilities v0.1 MemoryOps 能力边界 / §Problem Statement 痛点 4）
- `docs/specs/phases/phase-5-memoryops.md`
- `docs/specs/tasks/task-5.1-dedup.md`
- `docs/decisions/adr-005-readonly-import-draft-export.md`
- `test/features/memoryops.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Implementation Phases Phase 5 Exit Criteria): stale 标记可被设置和检索（`expires_at` 到期 / source deleted / source modified 三种触发）。
- [ ] **AC2** (PRD §Core Capabilities v0.1 MemoryOps 能力边界): 基础冲突检测仅覆盖同一 key / path / tag 下明显冲突并给提示。
- [ ] **AC3** (PRD §Core Capabilities v0.1 MemoryOps 能力边界): **不做** LLM 语义冲突判断（边界外）。
- [ ] **AC4** (PRD §Problem Statement 痛点 4 / 本 task 新增): 检索可选择排除/标注 stale 记录，避免过期上下文污染召回。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 stale 三触发可设/检索 | SCEN-5.2.1 | TEST-5.2.1 | - | unit-test | Not Started |
| AC2 基础冲突检测提示 | SCEN-5.2.2 | TEST-5.2.2 | - | unit-test | Not Started |
| AC3 不做语义冲突(边界) | SCEN-5.2.3 | TEST-5.2.3 | - | unit-test | Not Started |
| AC4 检索可排除 stale | SCEN-5.2.4 | TEST-5.2.4 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R5**：source modified/deleted 判定依赖 provenance.source_modified_at 准确性。关联 PRD §Open Questions **O5**。

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
