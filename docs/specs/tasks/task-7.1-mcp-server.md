# Task `7.1`: `mcp-server — MCP server (context_search/read/explain/collections) + client allowlist`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 7 (mcp-adapter)
**Dependencies**: Phase 6 (cli-api-export)

## 1. Background

把 ContextForge 接入真实多 Agent 工作流（OpenClaw/Hermes/Claude Code/Cursor/Zed）经 MCP（PRD §Vision / §Technical Approach MCP tools）。MCP 协议/SDK 版本为 PRD §Open Questions O4 的 TBD（需 Phase 7 启动前锁定）。Phase 7 唯一 task（即最后 task，team §4 Gate 3 触发）。

## 2. Goal

MCP server 暴露 `context_search` / `context_read` / `context_explain` / `context_collections`，返回字段与 REST search result 可解释字段一致；MCP client 须显式 allowlist，未授权拒绝；adapter 仅做协议翻译，与核心检索解耦（R7 缓解）。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Technical Approach MCP tools / §Constraints Local service security baseline / §Technical Risks R7,R9 / §Open Questions O4）
- `docs/specs/phases/phase-7-mcp-adapter.md`
- `docs/specs/tasks/task-6.1-cli-search.md`
- `docs/specs/tasks/task-6.2-rest-api.md`
- `docs/decisions/adr-003-cli-rest-mcp-grpc-interfaces.md`
- `test/features/mcp-adapter.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Implementation Phases Phase 7 Exit Criteria): MCP `context_search` 返回可解释结果，字段与 REST search result 一致。
- [ ] **AC2** (PRD §Implementation Phases Phase 7 Exit Criteria): MCP `context_read` 读取指定 chunk/context；`context_explain` 返回召回理由+provenance；`context_collections` 列出可用 collection。
- [ ] **AC3** (PRD §Constraints Local service security baseline / §Technical Risks R9): MCP client 未被 allowlist 时拒绝访问，访问写 audit log。
- [ ] **AC4** (PRD §Technical Risks R7 / §Open Questions O4): mcp-adapter 与核心检索解耦（仅协议翻译）；锁定一个已发布 MCP spec 版本并在 spec 标注兼容范围。
- [ ] **AC5** (本 task 新增): Phase 7 端到端 smoke 可执行（起 MCP server → client 调 4 tool 校验字段与 REST 一致 + 未 allowlist client 被拒），为 phase spec §6 端到端 smoke 提供落点。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 context_search 一致字段 | SCEN-7.1.1 | TEST-7.1.1 | - | unit-test | Not Started |
| AC2 read/explain/collections | SCEN-7.1.2 | TEST-7.1.2 | - | unit-test | Not Started |
| AC3 client allowlist 拒绝+审计 | SCEN-7.1.3 | TEST-7.1.3 | - | unit-test | Not Started |
| AC4 adapter 解耦+版本锁定 | SCEN-7.1.4 | TEST-7.1.4 | - | unit-test | Not Started |
| AC5 Phase7 端到端 smoke | SCEN-7.1.5 | TEST-7.1.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R7**（MCP 协议/SDK 漂移）+ **R9**（MCP client 越权）：adapter 解耦 + 版本锁定 + client allowlist。关联 PRD §Open Questions **O4**（Phase 7 启动前锁定 MCP 目标版本）。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。⚠️ 本 task 为 Phase 7 唯一/最后 task：完工/合并前 phase spec §6 端到端 smoke 必须填实（C1 / team §4 Gate 3）。

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
