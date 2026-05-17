# Task `3.4`: `importer-agent-rules — AGENTS.md / CLAUDE.md / Cursor·Zed rules 导入`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P1
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 3 (agent-importers)
**Dependencies**: 3.1 (importer-core)

## 1. Background

项目级规则文件（AGENTS.md / CLAUDE.md / Cursor·Zed rules）是 PRD P0 导入源，作为 agent_rule source 导入。Cursor/Zed 具体路径与格式为 PRD §Open Questions O3 的 TBD，v0.1 当作 project instruction / agent rule source 处理，不做深度语义写回（PRD §Constraints 兼容性 / §Core Capabilities Out of Scope）。

## 2. Goal

`contextforge import agent-rules <path>` 把 AGENTS.md / CLAUDE.md / Cursor·Zed 规则类 Markdown 导入为 source_type=`agent_rule` 的 ContextRecord；不写回这些文件。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Constraints 兼容性 Claude Code/Cursor/Zed 范围 / §Open Questions O3）
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

- [ ] **AC1** (PRD §Implementation Phases Phase 3 Exit Criteria): `AGENTS.md` / `CLAUDE.md` 能作为 `agent_rule` source 导入为 ContextRecord。
- [ ] **AC2** (PRD §Constraints 兼容性): Cursor / Zed 规则类 Markdown 能导入（路径/格式 TBD → 走通用 markdown + agent_rule 标记）。
- [ ] **AC3** (PRD §Decisions Log D5 / §Core Capabilities Out of Scope): 只读导入，不写回 AGENTS.md/CLAUDE.md/Cursor·Zed rules，不做深度语义写回。
- [ ] **AC4** (PRD §Open Questions O3): Cursor/Zed 具体路径与格式标 TBD，v0.1 不识别即走通用 fallback + warning，不中断。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 AGENTS/CLAUDE 导入 | SCEN-3.4.1 | TEST-3.4.1 | - | unit-test | Not Started |
| AC2 Cursor/Zed rules 导入 | SCEN-3.4.2 | TEST-3.4.2 | - | unit-test | Not Started |
| AC3 只读不写回 | SCEN-3.4.3 | TEST-3.4.3 | - | unit-test | Not Started |
| AC4 路径 TBD 走 fallback | SCEN-3.4.4 | TEST-3.4.4 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R5**：Cursor/Zed 规则文件格式漂移。关联 PRD §Open Questions **O3**（需各工具当前版本实测）。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。⚠️ 本 task 为 Phase 3 最后 task（之一，与 3.2/3.3 并列末批）：Phase 3 最后合并的 task 完工前 phase spec §6 端到端 smoke 必须填实（C1 / team §4 Gate 3）。

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
