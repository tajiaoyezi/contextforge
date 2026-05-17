# Task `3.1`: `importer-core — canonical record 映射 + importer 框架 + fallback`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 3 (agent-importers)
**Dependencies**: Phase 1（canonical schema + proto）

## 1. Background

Phase 3 框架 task：定义 importer 抽象 + canonical record 映射 + 通用 file/markdown fallback，使 hermes/openclaw/agent-rules importer 共享一致映射并对未识别 schema 安全降级（PRD §Technical Risks R5 / §Decisions Log D5）。

## 2. Goal

`agent-importer` 框架就位：定义 `Importer` 抽象（探测/解析/映射为 ContextRecord），通用 file/markdown/config/log fallback 永远可用；不识别 schema → 降级 fallback + warning，不中断；canonical record 与 importer 解耦。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Decisions Log D5 / §Technical Approach Canonical Record schema / §Technical Risks R5）
- `docs/specs/phases/phase-3-agent-importers.md`
- `docs/specs/tasks/task-1.1-proto.md`
- `docs/decisions/adr-005-readonly-import-draft-export.md`
- `test/features/importer.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Decisions Log D5): 定义 `Importer` 抽象（探测 → 解析 → 映射为 `ContextRecord`），只读导入，不写回任何第三方 Agent memory。
- [ ] **AC2** (PRD §Technical Risks R5): 通用 file/markdown/config/log fallback 永远可用，作为分层 importer 的保底层。
- [ ] **AC3** (PRD §Implementation Phases Phase 3 Exit Criteria): 不识别 schema → 降级为通用文件导入 + 显式 warning，不中断整个导入。
- [ ] **AC4** (PRD §Technical Approach Canonical Record v0.1): 映射产出的 ContextRecord 含 source_type/source_provider/source_uri/agent_scope/provenance 等核心字段，未识别字段进 metadata.extra。
- [ ] **AC5** (PRD §Technical Risks R5 / 本 task 新增): 每个 importer 可声明版本探测钩子，canonical record 与 importer 解耦（更换 importer 不动 record schema）。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 Importer 抽象只读 | SCEN-3.1.1 | TEST-3.1.1 | - | unit-test | Not Started |
| AC2 通用 fallback 保底 | SCEN-3.1.2 | TEST-3.1.2 | - | unit-test | Not Started |
| AC3 未识别降级+warning | SCEN-3.1.3 | TEST-3.1.3 | - | unit-test | Not Started |
| AC4 映射核心字段完整 | SCEN-3.1.4 | TEST-3.1.4 | - | unit-test | Not Started |
| AC5 importer/record 解耦 | SCEN-3.1.5 | TEST-3.1.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R5**（外部 Agent schema 漂移，概率高）：分层 importer + fallback 是核心缓解；本 task 奠定框架。关联 PRD §Open Questions **O3 / O5**。

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
