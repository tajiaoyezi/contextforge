# Task `1.3`: `core-skeleton — contextforge-core Rust 骨架 + gRPC server + health`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 1 (foundation)
**Dependencies**: 1.1 (proto)

## 1. Background

数据面二进制 `contextforge-core`（Rust）经 local gRPC 被 Go daemon 拉起与健康检查（PRD §Decisions Log D1 / §Technical Risks R1）。本 task 搭 Rust 侧 tonic gRPC server 骨架 + health，使双进程契约可端到端打通。

## 2. Goal

`contextforge-core` 可独立启动并监听 local gRPC（Unix socket 或 127.0.0.1）；实现 health/SERVING 响应；proto 由 tonic codegen 接入；模块占位（scanner/parser/chunker/indexer/retriever/memoryops）目录就位但不实现。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Technical Approach 架构风格 / 数据流）
- `docs/specs/phases/phase-1-foundation.md`
- `docs/specs/tasks/task-1.1-proto.md`
- `docs/decisions/adr-001-go-rust-dual-binary-architecture.md`
- `docs/decisions/adr-008-core-library-selection.md`
- `test/features/core.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：
     - 完整写出 AC；每条 `- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`
     - review 改内容不删注释；严禁 `<TBD-by-user> AC<N>` 混合写法
-->

- [ ] **AC1** (PRD §Decisions Log D1): `contextforge-core` 二进制可构建并独立启动，监听 local gRPC（Unix socket 或 127.0.0.1，禁默认 0.0.0.0，PRD §Constraints Local service security baseline）。
- [ ] **AC2** (PRD §Implementation Phases Phase 1 Exit Criteria): gRPC health 返回 SERVING；可被 Go daemon health check（task 1.4 端到端验证）。
- [ ] **AC3** (PRD §Decisions Log D8): tonic + tokio + serde 接入，proto 由 tonic codegen，无 FFI/cgo。
- [ ] **AC4** (本 task 新增): scanner/parser/chunker/indexer/retriever/memoryops 在 `core/src/` 建模块占位（编译通过，不实现逻辑），供 Phase 2+ 落地。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 core 可启动监听 | SCEN-1.3.1 | TEST-1.3.1 | - | unit-test | Not Started |
| AC2 gRPC health SERVING | SCEN-1.3.2 | TEST-1.3.2 | - | unit-test | Not Started |
| AC3 tonic codegen 无 FFI | SCEN-1.3.3 | TEST-1.3.3 | - | unit-test / typecheck | Not Started |
| AC4 模块占位编译通过 | SCEN-1.3.4 | TEST-1.3.4 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R1**（进程生命周期 / core 崩溃恢复）：health 必须可靠，为 task 1.4 daemon 自动重启 + 健康检查提供基础。

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
