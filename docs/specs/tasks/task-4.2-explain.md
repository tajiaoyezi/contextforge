# Task `4.2`: `explain — explainable retrieval trace + result schema`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 4 (retrieval-explain)
**Dependencies**: 4.1 (retriever)

## 1. Background

可解释检索是 ContextForge 一等公民与核心差异（PRD §Core Capabilities #2 / §Vision 关键差异）。本 task 在 retriever 之上产出可解释 result：每条结果带来源/位置/打分/召回方式/理由/scope，并产出 retrieval trace。是 Phase 4 最后一个 task（team §4 Gate 3 触发）。

## 2. Goal

检索结果按 PRD §Technical Approach search response 契约带 `chunk_id/context_id/source_type/file_path/line_start/line_end/score/retrieval_method/reason/agent_scope/redaction_status/provenance`；可输出 retrieval trace（为何召回：命中词/方式/分数）；可经内部 gRPC Search API / `contextforge search` 调试入口验证。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Core Capabilities #2 / §Technical Approach REST/MCP search response / §Success Metrics 可解释性覆盖率）
- `docs/specs/phases/phase-4-retrieval-explain.md`
- `docs/specs/tasks/task-4.1-retriever.md`
- `docs/specs/tasks/task-1.1-proto.md`
- `docs/decisions/adr-003-cli-rest-mcp-grpc-interfaces.md`
- `test/features/retriever.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Technical Approach REST/MCP search response): 每条结果含 chunk_id/context_id/source_type/file_path/line_start/line_end/score/retrieval_method/reason/agent_scope/redaction_status/provenance。
- [ ] **AC2** (PRD §Implementation Phases Phase 4 Exit Criteria): 结果能定位回原始文件和行号（file_path + line_start/line_end 精确）。
- [ ] **AC3** (PRD §Success Metrics 次指标 / 反指标): 可解释性覆盖率 ≥ 90% 结果含全部可解释字段；禁止返回无 provenance 的"黑盒高分"结果。
- [ ] **AC4** (PRD §Implementation Phases Phase 4 Exit Criteria): 可经内部 gRPC Search API / `contextforge search` 调试入口返回上述可解释结果。
- [ ] **AC5** (本 task 新增): Phase 4 端到端 smoke 可执行（索引 fixture → 一组 query 校验每条结果 7+ 可解释字段 + 空 query 不 panic），为 phase spec §6 端到端 smoke 提供落点。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 可解释字段完整 | SCEN-4.2.1 | TEST-4.2.1 | - | unit-test | Not Started |
| AC2 定位回原文行号 | SCEN-4.2.2 | TEST-4.2.2 | - | unit-test | Not Started |
| AC3 覆盖率≥90%/禁黑盒 | SCEN-4.2.3 | TEST-4.2.3 | - | unit-test | Not Started |
| AC4 gRPC/CLI 调试入口 | SCEN-4.2.4 | TEST-4.2.4 | - | unit-test | Not Started |
| AC5 Phase4 端到端 smoke | SCEN-4.2.5 | TEST-4.2.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R3**（召回率）：reason/trace 为调参与回归提供依据。反指标硬约束：可解释性不可为命中率牺牲。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。⚠️ 本 task 为 Phase 4 最后 task：完工/合并前 phase spec §6 端到端 smoke 必须填实（C1 / team §4 Gate 3）。

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
