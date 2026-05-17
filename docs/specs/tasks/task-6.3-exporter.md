# Task `6.3`: `exporter — canonical JSONL / Markdown bundle / agent draft 导出 + 二次 secret scan`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 6 (cli-api-export)
**Dependencies**: 6.1 (cli-search)

## 1. Background

跨 Agent 上下文迁移的导出侧（PRD §Core Capabilities #5 / §Decisions Log D5）。导出一律 draft/bundle 不写回；export 前二次 secret scan（PRD §Technical Risks R4 / §Constraints 安全）。是 Phase 6 末批 task（与 6.2 并列）。

## 2. Goal

`contextforge export --format jsonl|markdown-bundle|agent-draft` 把选定 collection 或 search result 导出为 canonical JSONL / Markdown bundle / Agent rule draft；导出前执行二次 secret scan；迁移后结构化字段保真率可经 fixture 计算（目标 ≥ 80%）；不写回任何第三方 Agent。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Core Capabilities #5 / §Decisions Log D5 / §Constraints 兼容性导出格式 / §Success Metrics 跨 Agent 迁移保真）
- `docs/specs/phases/phase-6-cli-api-export.md`
- `docs/specs/tasks/task-6.1-cli-search.md`
- `docs/specs/tasks/task-2.1-scanner.md`
- `docs/decisions/adr-005-readonly-import-draft-export.md`
- `test/features/exporter.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Implementation Phases Phase 6 Exit Criteria): `contextforge export --format jsonl` 导出 canonical JSONL；`--format markdown-bundle` 导出 Markdown bundle。
- [ ] **AC2** (PRD §Constraints 兼容性导出格式): 支持 `--format agent-draft`（Hermes-style MEMORY.md/USER.md / AGENTS.md / CLAUDE.md draft），draft/bundle 一律不写回第三方 Agent。
- [ ] **AC3** (PRD §Technical Risks R4 / §Constraints 安全): export 前执行二次 secret scan，导出物不含完整 secret。
- [ ] **AC4** (PRD §Success Metrics 跨 Agent 迁移保真): 迁移后结构化字段保真率可经 fixture 计算，目标 ≥ 80%。
- [ ] **AC5** (本 task 新增): Phase 6 端到端 smoke 可执行（index fixture → search + curl /v1/search 一致 → export 三格式 + 二次 secret scan 命中 + 字段保真 ≥80%），为 phase spec §6 端到端 smoke 提供落点。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 jsonl/md-bundle 导出 | SCEN-6.3.1 | TEST-6.3.1 | - | unit-test | Not Started |
| AC2 agent-draft 不写回 | SCEN-6.3.2 | TEST-6.3.2 | - | unit-test | Not Started |
| AC3 export 二次 secret scan | SCEN-6.3.3 | TEST-6.3.3 | - | unit-test | Not Started |
| AC4 迁移保真率≥80% | SCEN-6.3.4 | TEST-6.3.4 | - | unit-test | Not Started |
| AC5 Phase6 端到端 smoke | SCEN-6.3.5 | TEST-6.3.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R4**（export 二次扫描漏检）+ **R5**（agent-draft 格式随上游漂移）。关联 PRD §Open Questions **O5**（schema 无损承载边界）。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。⚠️ 本 task 与 6.2 为 Phase 6 末批：Phase 6 最后合并的 task 完工前 phase spec §6 端到端 smoke 必须填实（C1 / team §4 Gate 3）。

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
