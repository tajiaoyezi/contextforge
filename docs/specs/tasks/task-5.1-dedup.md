# Task `5.1`: `dedup — content/source hash 去重 + provenance 合并`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 5 (memoryops)
**Dependencies**: Phase 2 (索引产物), Phase 3 (importer 产出 record)

## 1. Background

MemoryOps 治理核心：同一事实跨多 Agent source 重复时去重并保留 provenance 链（PRD §User Flow 边界场景 / §Core Capabilities #3）。能力边界严格按 PRD「v0.1 MemoryOps 能力边界」：仅 normalized content hash / source hash / exact duplicate 去重。

## 2. Goal

`memoryops` 能基于 normalized content hash / source hash 检出 exact duplicate 并去重；provenance 链合并保留多个来源（不丢原始来源）；不做语义相似去重（边界外）。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Core Capabilities #3 + v0.1 MemoryOps 能力边界 / §User Flow 边界场景）
- `docs/specs/phases/phase-5-memoryops.md`
- `docs/specs/tasks/task-2.3-chunker.md`
- `docs/specs/tasks/task-3.1-importer-core.md`
- `docs/decisions/adr-005-readonly-import-draft-export.md`
- `test/features/memoryops.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Implementation Phases Phase 5 Exit Criteria): exact duplicate 能被去重（normalized content hash / source hash）。
- [ ] **AC2** (PRD §Implementation Phases Phase 5 Exit Criteria / §User Flow 边界场景): provenance 链能合并并保留多个来源，不丢失原始来源。
- [ ] **AC3** (PRD §Core Capabilities v0.1 MemoryOps 能力边界): **不做** LLM 语义相似去重 / 语义冲突判断（边界外，仅 exact duplicate）。
- [ ] **AC4** (本 task 新增): 去重锚点为 task 2.3 chunker 产出的 content_hash，保证同内容跨来源 hash 一致可去重。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 exact duplicate 去重 | SCEN-5.1.1 | TEST-5.1.1 | - | unit-test | Not Started |
| AC2 provenance 链合并 | SCEN-5.1.2 | TEST-5.1.2 | - | unit-test | Not Started |
| AC3 不做语义去重(边界) | SCEN-5.1.3 | TEST-5.1.3 | - | unit-test | Not Started |
| AC4 content_hash 锚点 | SCEN-5.1.4 | TEST-5.1.4 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R5**：provenance 与 importer 解耦（content_hash 锚点）。关联 PRD §Open Questions **O5 / O9**（schema 无损承载边界）。

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
