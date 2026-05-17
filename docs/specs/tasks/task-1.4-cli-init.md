# Task `1.4`: `cli-init — Go CLI + daemon 骨架 + gRPC client + contextforge init 端到端`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 1 (foundation)
**Dependencies**: 1.1 (proto), 1.2 (config), 1.3 (core-skeleton)

## 1. Background

Phase 1 收口 task：把 proto/config/core-skeleton 串成端到端 `contextforge init`，并打通 Go daemon ↔ Rust core 的 local gRPC（PRD §Implementation Phases Phase 1 / §Technical Risks R1）。这是 Phase 1 的最后一个 task（team §4 Gate 3 phase smoke gate 在此触发）。

## 2. Goal

`contextforge init` 端到端跑通：生成本地配置与数据目录、由 daemon 拉起 `contextforge-core`、Go 经 local gRPC health check Rust core 返回 SERVING；CLI 骨架（cobra）含 init/import/index/search/serve/mcp/eval/export 子命令注册（未实现的返回 not-implemented）。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§User Flow 主流程步 1 / §Technical Approach）
- `docs/specs/phases/phase-1-foundation.md`
- `docs/specs/tasks/task-1.1-proto.md`
- `docs/specs/tasks/task-1.2-config.md`
- `docs/specs/tasks/task-1.3-core-skeleton.md`
- `docs/decisions/adr-001-go-rust-dual-binary-architecture.md`
- `docs/decisions/adr-003-cli-rest-mcp-grpc-interfaces.md`
- `test/features/cli.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §User Flow 主流程步 1): `contextforge init` 生成 `~/.contextforge/` 配置与数据目录（不联网），幂等可重跑。
- [ ] **AC2** (PRD §Implementation Phases Phase 1 Exit Criteria): daemon 能启动 `contextforge-core` 并经 local gRPC health check 返回 SERVING。
- [ ] **AC3** (PRD §Technical Risks R1): core 异常退出时 daemon 能自动重启 + 健康检查（基础版）。
- [ ] **AC4** (PRD §Technical Approach / §Decisions Log D3): cobra CLI 注册 init/import/index/search/serve/mcp/eval/export 子命令；未实现子命令返回明确 not-implemented 提示（非 panic）。
- [ ] **AC5** (本 task 新增): Phase 1 端到端 smoke 可执行（init → core 拉起 → gRPC health SERVING），为 phase spec §6 端到端 smoke 提供落点。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 init 生成配置/目录 | SCEN-1.4.1 | TEST-1.4.1 | - | unit-test | Not Started |
| AC2 daemon 拉起 core+health | SCEN-1.4.2 | TEST-1.4.2 | - | unit-test | Not Started |
| AC3 core 崩溃自动重启 | SCEN-1.4.3 | TEST-1.4.3 | - | unit-test | Not Started |
| AC4 CLI 子命令注册 | SCEN-1.4.4 | TEST-1.4.4 | - | unit-test | Not Started |
| AC5 Phase1 端到端 smoke | SCEN-1.4.5 | TEST-1.4.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R1**（Go↔Rust gRPC 边界 / 进程生命周期）：本 task 端到端验证 R1 缓解；daemon 自动重启 + 健康检查在此落地。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。⚠️ 本 task 为 Phase 1 最后 task：完工/合并前 phase spec §6 端到端 smoke 必须填实（`s2v_preflight_phase` C1 / team §4 Gate 3）。

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
