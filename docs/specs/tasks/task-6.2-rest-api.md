# Task `6.2`: `rest-api — daemon 本地 REST API (/v1/*)`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 6 (cli-api-export)
**Dependencies**: 6.1 (cli-search)

## 1. Background

Agent 程序化调用需要本地 REST API（PRD §Decisions Log D3 / §Technical Approach REST/MCP 最小接口契约草案）。本地服务安全基线严格（PRD §Constraints Local service security baseline / §Technical Risks R9）。

## 2. Goal

daemon 暴露 `POST /v1/search` / `GET /v1/chunks/{id}` / `POST /v1/import` / `POST /v1/eval/run` / `GET /v1/collections`，请求/响应契约与 PRD §Technical Approach 草案一致；默认只监听 127.0.0.1 或 Unix socket（禁 0.0.0.0），启用本地随机 token（文件 0600）。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Technical Approach REST/MCP 最小接口契约草案 / §Constraints Local service security baseline / §Technical Risks R9）
- `docs/specs/phases/phase-6-cli-api-export.md`
- `docs/specs/tasks/task-6.1-cli-search.md`
- `docs/specs/tasks/task-1.2-config.md`
- `docs/decisions/adr-003-cli-rest-mcp-grpc-interfaces.md`
- `test/features/daemon.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Implementation Phases Phase 6 Exit Criteria): REST `POST /v1/search` 可用，请求/响应契约与 PRD §Technical Approach 草案一致。
- [ ] **AC2** (PRD §Technical Approach REST/MCP 契约): `GET /v1/chunks/{id}` / `POST /v1/import` / `POST /v1/eval/run` / `GET /v1/collections` 可用。
- [ ] **AC3** (PRD §Constraints Local service security baseline): daemon 默认只监听 `127.0.0.1` 或 Unix socket，v0.1 禁默认绑定 `0.0.0.0`。
- [ ] **AC4** (PRD §Constraints Local service security baseline): REST API 默认启用本地随机 token，token 文件权限 `0600`。
- [ ] **AC5** (PRD §Technical Risks R9): 未带有效 token 的请求被拒；访问写 audit log（脱敏，复用 task 5.3）。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 /v1/search 契约一致 | SCEN-6.2.1 | TEST-6.2.1 | - | unit-test | Not Started |
| AC2 其余 /v1/* 可用 | SCEN-6.2.2 | TEST-6.2.2 | - | unit-test | Not Started |
| AC3 默认本地监听禁0.0.0.0 | SCEN-6.2.3 | TEST-6.2.3 | - | unit-test | Not Started |
| AC4 token 0600 | SCEN-6.2.4 | TEST-6.2.4 | - | unit-test | Not Started |
| AC5 无 token 拒绝+审计 | SCEN-6.2.5 | TEST-6.2.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R9**（本地 daemon/MCP 暴露面）：监听限制 + token + audit 为核心缓解。关联 PRD §Open Questions **O10**。

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
