# Task `8.3`: `release-smoke — Linux x86_64 release 打包 + smoke test + 性能基准`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 8 (eval-and-reliability)
**Dependencies**: 8.1 (eval-harness), 8.2 (reliability)

## 1. Background

v0.1 收口 task：产出可安装的 Linux x86_64 release 包并通过 smoke test，验证 v0.1 七项技术闭环在 Linux/WSL2 端到端跑通（PRD §Implementation Phases Phase 8 / §Decisions Log D7）。这是 Phase 8 也是整个 v0.1 的最后一个 task（team §4 Gate 3 触发，phase spec §6 端到端 smoke = v0.1 七项闭环）。

## 2. Goal

产出 `contextforge-linux-amd64.tar.gz`（含 `contextforge` + `contextforge-core` + `contextforge.example.toml` + README + LICENSE）；Linux/WSL2 release smoke test 通过（解包→init→import→index→search/MCP→export→eval run）；10 万 chunk BM25/metadata/filter P95 < 500ms 基准达标；README 快速启动可复现。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Decisions Log D7 / §Constraints 发布 / §Implementation Phases Phase 8 Exit Criteria / §Success Metrics）
- `docs/specs/phases/phase-8-eval-and-reliability.md`
- `docs/specs/tasks/task-8.1-eval-harness.md`
- `docs/specs/tasks/task-8.2-reliability.md`
- `docs/decisions/adr-007-minimal-tarball-distribution.md`
- `test/features/release.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Decisions Log D7 / §Constraints 发布): 产出 `contextforge-linux-amd64.tar.gz`，含 contextforge + contextforge-core + contextforge.example.toml + README + LICENSE。
- [ ] **AC2** (PRD §Implementation Phases Phase 8 Exit Criteria): Linux / WSL2 release smoke test 通过（解包→init→import→index→search/MCP→export→eval run 端到端）。
- [ ] **AC3** (PRD §Implementation Phases Phase 8 Exit Criteria / §Success Metrics 次指标): 10 万 chunk 内 BM25/metadata/filter 检索 P95 < 500ms 基准达标。
- [ ] **AC4** (PRD §Implementation Phases v0.1 七项技术闭环): v0.1 七项闭环在 Linux/WSL2 端到端跑通（导入/索引/CLI·API 搜索/MCP/可解释检索/recall eval/可靠运行）。
- [ ] **AC5** (本 task 新增 / C1): 本 task = Phase 8 与 v0.1 最后 task，phase spec §6 端到端 smoke（= v0.1 七项闭环 release smoke 序列）必须在合并前填实并全过。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 tarball 产物完整 | SCEN-8.3.1 | TEST-8.3.1 | - | unit-test | Not Started |
| AC2 release smoke 通过 | SCEN-8.3.2 | TEST-8.3.2 | - | unit-test | Not Started |
| AC3 P95<500ms 基准 | SCEN-8.3.3 | TEST-8.3.3 | - | unit-test | Not Started |
| AC4 v0.1 七项闭环跑通 | SCEN-8.3.4 | TEST-8.3.4 | - | unit-test | Not Started |
| AC5 phase §6 端到端 smoke | SCEN-8.3.5 | TEST-8.3.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R6**（大仓库性能/资源回归）+ **R2**（向量后端选型应已结论）+ **R3**（召回率达标判定）。关联 PRD §Open Questions **O2/O6**。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。⚠️ 本 task 为 Phase 8 与 v0.1 最后 task：完工/合并前 phase spec §6 端到端 smoke（v0.1 七项闭环 release smoke）必须填实且全过（C1 / team §4 Gate 3）。

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
