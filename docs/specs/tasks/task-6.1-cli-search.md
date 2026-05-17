# Task `6.1`: `cli-search — contextforge search 命令`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 6 (cli-api-export)
**Dependencies**: Phase 4 (retrieval-explain), Phase 5 (memoryops)

## 1. Background

把可解释检索对外暴露为用户最常用入口 `contextforge search`（PRD §User Flow 主流程步 3 / §Core Capabilities #2）。Phase 6 首个 task，6.2/6.3 依赖其命令骨架。

## 2. Goal

`contextforge search "<query>" [--collections --agent-scope --top-k --filters --explain]` 经 Go CLI → daemon → Rust retriever 返回带可解释字段的结果（CLI 人类可读 + `--json`）；命中 stale/去重后的治理结果（Phase 5 协同）。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§User Flow 主流程 / §Technical Approach REST/MCP search 契约 / §Success Metrics）
- `docs/specs/phases/phase-6-cli-api-export.md`
- `docs/specs/tasks/task-4.2-explain.md`
- `docs/specs/tasks/task-5.2-lifecycle.md`
- `docs/decisions/adr-003-cli-rest-mcp-grpc-interfaces.md`
- `test/features/cli.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Implementation Phases Phase 6 Exit Criteria): `contextforge search "<query>"` 可用并返回 Top-K 可解释结果。
- [ ] **AC2** (PRD §Technical Approach REST/MCP 契约): 支持 `--collections / --agent-scope / --top-k / --filters / --explain`，语义与 search 请求契约一致。
- [ ] **AC3** (PRD §Core Capabilities #2): 结果含全部可解释字段，CLI 人类可读输出 + `--json` 结构化输出二选一。
- [ ] **AC4** (PRD §Constraints 安全): 结果默认不展示完整 secret（redaction_status 透传，复用 scanner/explain 行为）。
- [ ] **AC5** (PRD §User Flow 主流程 5 步): search 与后续 export 命令共享检索结果模型，为 6.3 export search-result 提供接口。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 search 返回 Top-K | SCEN-6.1.1 | TEST-6.1.1 | - | unit-test | Not Started |
| AC2 flags 契约一致 | SCEN-6.1.2 | TEST-6.1.2 | - | unit-test | Not Started |
| AC3 可解释字段+--json | SCEN-6.1.3 | TEST-6.1.3 | - | unit-test | Not Started |
| AC4 不展示完整 secret | SCEN-6.1.4 | TEST-6.1.4 | - | unit-test | Not Started |
| AC5 与 export 共享结果模型 | SCEN-6.1.5 | TEST-6.1.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R9**（本地暴露面）：CLI 经 daemon 走本地 gRPC，遵守 daemon 监听限制（task 6.2）。

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
