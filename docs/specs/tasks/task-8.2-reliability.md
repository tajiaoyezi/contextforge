# Task `8.2`: `reliability — 长任务/中断恢复 + 资源硬化 + secret/export 回归`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 8 (eval-and-reliability)
**Dependencies**: Phase 6 (cli-api-export), Phase 7 (mcp-adapter)

## 1. Background

Phase 2 仅做基础增量，完整长任务恢复与资源硬化推到本 task（PRD §Implementation Phases Phase 2/Phase 8 / §Technical Risks R6）。同时对 secret redaction / export 做回归，保证安全反指标不退化。

## 2. Goal

大仓库长任务中断后可恢复或安全重建（`index --resume` 断点续传）；资源占用满足 PRD §Constraints（daemon idle <300MB、基础索引 <2GB 等工程目标）；secret redaction / export / audit log 回归测试通过（反指标守住）。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Constraints 性能/资源 / §Implementation Phases Phase 8 Exit Criteria / §Technical Risks R6 / §Success Metrics 反指标）
- `docs/specs/phases/phase-8-eval-and-reliability.md`
- `docs/specs/tasks/task-2.4-indexer.md`
- `docs/specs/tasks/task-6.3-exporter.md`
- `docs/decisions/adr-002-sqlite-tantivy-layered-storage.md`
- `test/features/reliability.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Implementation Phases Phase 8 Exit Criteria): 大仓库长任务中断后可恢复或安全重建（`index --resume` 断点续传，不重复全量）。
- [ ] **AC2** (PRD §Constraints 性能/资源): daemon idle 内存 < 300MB、基础索引 < 2GB、单次搜索额外 < 200MB（工程目标，真实大仓库基准）。
- [ ] **AC3** (PRD §Implementation Phases Phase 8 Exit Criteria / §Success Metrics 反指标): secret redaction / export / audit log 回归测试通过（denylist/secret scan 不被性能优化绕过）。
- [ ] **AC4** (PRD §User Flow 异常流): 索引中断进入长任务模式（进度显示/可中断/可恢复），大规模变更自动降级后台任务。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 中断可恢复/续传 | SCEN-8.2.1 | TEST-8.2.1 | - | unit-test | Not Started |
| AC2 资源占用达标 | SCEN-8.2.2 | TEST-8.2.2 | - | unit-test | Not Started |
| AC3 secret/export 回归 | SCEN-8.2.3 | TEST-8.2.3 | - | unit-test | Not Started |
| AC4 长任务模式降级 | SCEN-8.2.4 | TEST-8.2.4 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R6**（大仓库性能/资源不达标）+ **R4**（回归守住 redaction）。关联 PRD §Open Questions **O2**（向量后端选型 spike，PRD 定 Phase 5-6 期间做，本 task 前应有结论以免影响资源基准）。

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
