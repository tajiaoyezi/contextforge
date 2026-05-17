# Task `5.3`: `audit — 审计事件 + audit log`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 5 (memoryops)
**Dependencies**: 5.1 (dedup)

## 1. Background

可审计性是 PRD 隐私基线一部分（PRD §Constraints 安全 + Local service security baseline / §Decisions Log D4）。本 task 实现 import/search/export/redact 等关键事件写 audit log，且 audit log 不记录完整 secret/导出内容。是 Phase 5 末批 task（与 5.2 并列）。

## 2. Goal

`memoryops` 能为 import / search / export / redact / delete 关键事件产出审计事件并写入 collection `audit.log`；默认记录 operation/collection/source/result_count/redaction_count/timestamp，**不**记录完整 query content / 完整 secret / 完整导出内容。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Constraints 安全 + Local service security baseline / §Decisions Log D4）
- `docs/specs/phases/phase-5-memoryops.md`
- `docs/specs/tasks/task-5.1-dedup.md`
- `docs/specs/tasks/task-2.1-scanner.md`
- `docs/decisions/adr-004-local-first-privacy-baseline.md`
- `test/features/memoryops.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Implementation Phases Phase 5 Exit Criteria): import / search / export / redact 事件能写入 collection `audit.log`。
- [ ] **AC2** (PRD §Constraints Local service security baseline): audit log 默认记录 operation/collection/source/result_count/redaction_count/timestamp，**不**默认记录完整 query content。
- [ ] **AC3** (PRD §Constraints 安全): audit log **不**记录完整 secret、**不**记录完整导出内容。
- [ ] **AC4** (PRD §Technical Risks R4): scanner secret override（task 2.1 AC4 关联）发生时必须写 audit log（可追溯）。
- [ ] **AC5** (本 task 新增): Phase 5 端到端 smoke 可执行（导入含重复事实 fixture → 去重+provenance 合并 + stale 可标记可检索 + audit.log 含四类事件且无完整 secret），为 phase spec §6 端到端 smoke 提供落点。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 四类事件写 audit.log | SCEN-5.3.1 | TEST-5.3.1 | - | unit-test | Not Started |
| AC2 默认字段不含 query 全文 | SCEN-5.3.2 | TEST-5.3.2 | - | unit-test | Not Started |
| AC3 不记录完整 secret/导出 | SCEN-5.3.3 | TEST-5.3.3 | - | unit-test | Not Started |
| AC4 secret override 写 audit | SCEN-5.3.4 | TEST-5.3.4 | - | unit-test | Not Started |
| AC5 Phase5 端到端 smoke | SCEN-5.3.5 | TEST-5.3.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R4**（redaction 漏检/误报）：audit log 提供可追溯性但本身不得泄露 secret。关联 PRD §Open Questions **O7 / O10**（威胁模型 / API 安全边界）。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。⚠️ 本 task 与 5.2 为 Phase 5 末批：Phase 5 最后合并的 task 完工前 phase spec §6 端到端 smoke 必须填实（C1 / team §4 Gate 3）。

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
