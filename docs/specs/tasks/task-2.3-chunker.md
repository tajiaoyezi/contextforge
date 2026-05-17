# Task `2.3`: `chunker — chunking + metadata 抽取 + provenance 维护`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 2 (index-core)
**Dependencies**: 2.2 (parser)

## 1. Background

把 parser 产出的解析单元切成检索用 `Chunk`，抽取 metadata，并维护 provenance（来源链）。chunking 策略需可配置以支撑 PRD §Technical Risks R3（不达标时按 code/markdown/log 分别调参）。

## 2. Goal

`chunker` 产出 `Chunk`（含 chunk_id / file_path / line_start / line_end / language / content / content_hash），并写入 `provenance`（importer/original_path/imported_at/source_modified_at）；chunking 策略可配置（按 code/markdown/log 分别策略）。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Technical Approach Canonical Record schema / §Technical Risks R3）
- `docs/specs/phases/phase-2-index-core.md`
- `docs/specs/tasks/task-1.1-proto.md`
- `docs/specs/tasks/task-2.2-parser.md`
- `docs/decisions/adr-002-sqlite-tantivy-layered-storage.md`
- `test/features/chunker.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Technical Approach Canonical Record v0.1): 每个 `Chunk` 含 chunk_id / file_path / line_start / line_end / language / content / content_hash。
- [ ] **AC2** (PRD §Technical Approach Canonical Record v0.1): `provenance[]` 写入 importer / original_path / imported_at / source_modified_at，可承载多来源。
- [ ] **AC3** (PRD §Technical Risks R3): chunking 策略可配置，对 code / markdown / log 分别可调参。
- [ ] **AC4** (PRD §User Flow 边界场景): 超大文件分块不爆内存（与 scanner 流式协同）。
- [ ] **AC5** (本 task 新增): content_hash 为后续 memoryops 去重锚点（normalized content hash），保证同内容跨来源 hash 一致。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 Chunk 字段完整 | SCEN-2.3.1 | TEST-2.3.1 | - | unit-test | Not Started |
| AC2 provenance 多来源 | SCEN-2.3.2 | TEST-2.3.2 | - | unit-test | Not Started |
| AC3 chunking 可配置 | SCEN-2.3.3 | TEST-2.3.3 | - | unit-test | Not Started |
| AC4 大文件分块不爆内存 | SCEN-2.3.4 | TEST-2.3.4 | - | unit-test | Not Started |
| AC5 content_hash 一致性 | SCEN-2.3.5 | TEST-2.3.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R3**（召回率）：chunking 策略直接影响召回，须可配置可回归。
- 关联 **R5**：provenance 与 importer 解耦（content_hash 锚点）。

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
