# Task `3.2`: `importer-hermes — Hermes MEMORY.md / USER.md 导入`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 3 (agent-importers)
**Dependencies**: 3.1 (importer-core)

## 1. Background

Hermes 是 PRD 列出的 P0 导入源之一。本 task 实现 Hermes `MEMORY.md` / `USER.md` → canonical ContextRecord 的只读导入（PRD §Constraints 兼容性 / §Decisions Log D5）。

## 2. Goal

`contextforge import hermes <path>` 把 Hermes `MEMORY.md` / `USER.md` 转为 ContextRecord（source_provider=hermes，agent_scope 含 hermes），保留 provenance（original_path / source_modified_at）；不写回 Hermes memory。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Constraints 兼容性 / §Core Capabilities #5）
- `docs/specs/phases/phase-3-agent-importers.md`
- `docs/specs/tasks/task-3.1-importer-core.md`
- `docs/decisions/adr-005-readonly-import-draft-export.md`
- `test/features/importer.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Implementation Phases Phase 3 Exit Criteria): Hermes `MEMORY.md` / `USER.md` 能导入为 canonical ContextRecord。
- [ ] **AC2** (PRD §Technical Approach Canonical Record v0.1): source_provider=`hermes`、agent_scope 含 `hermes`、provenance.importer=`hermes-memory`、保留 original_path / source_modified_at。
- [ ] **AC3** (PRD §Decisions Log D5): 只读导入，不修改/写回 Hermes `MEMORY.md` / `USER.md`。
- [ ] **AC4** (PRD §Technical Risks R5): Hermes schema 不识别/版本差异时降级通用 markdown 导入 + warning（复用 3.1 fallback），不中断。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 Hermes 导入为 record | SCEN-3.2.1 | TEST-3.2.1 | - | unit-test | Not Started |
| AC2 provider/scope/provenance | SCEN-3.2.2 | TEST-3.2.2 | - | unit-test | Not Started |
| AC3 只读不写回 | SCEN-3.2.3 | TEST-3.2.3 | - | unit-test | Not Started |
| AC4 schema 差异降级 | SCEN-3.2.4 | TEST-3.2.4 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R5**：Hermes schema 漂移 → fixture 回归 + fallback。关联 PRD §Open Questions **O3**（需实测 Hermes 版本与样本）。

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
