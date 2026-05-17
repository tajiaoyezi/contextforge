# Task `2.4`: `indexer — Tantivy 全文索引 + SQLite metadata/chunk 存储 + 增量更新 + contextforge index`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 2 (index-core)
**Dependencies**: 2.1 (scanner), 2.3 (chunker)

## 1. Background

Phase 2 收口 task：把 scanner→parser→chunker 产物写入 Tantivy 全文索引 + SQLite metadata/chunk 存储，并支持基础增量（PRD §Decisions Log D2 / §Implementation Phases Phase 2）。完整长任务恢复在 Phase 8 硬化。本 task 是 Phase 2 最后一个 task（team §4 Gate 3 phase smoke gate 触发）。

## 2. Goal

`contextforge index ./project` 端到端建立本地 Tantivy 索引 + SQLite（metadata/chunk/provenance）；denylist/allowlist + secret redaction 在索引链路生效；单文件变更触发基础增量更新（< 5s 工程目标）。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Decisions Log D2 / §Constraints 性能 / §Implementation Phases Phase 2 Exit Criteria）
- `docs/specs/phases/phase-2-index-core.md`
- `docs/specs/tasks/task-2.1-scanner.md`
- `docs/specs/tasks/task-2.3-chunker.md`
- `docs/decisions/adr-002-sqlite-tantivy-layered-storage.md`
- `test/features/indexer.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Implementation Phases Phase 2 Exit Criteria): `contextforge index ./sample_project` 能索引 ≥ 1000 个文件。
- [ ] **AC2** (PRD §Decisions Log D2): SQLite 存 metadata/chunk/provenance 可查询；Tantivy 全文可搜索到基础结果。
- [ ] **AC3** (PRD §Implementation Phases Phase 2 Exit Criteria): 索引链路尊重 denylist + secret redaction（denylist 路径不入索引、secret 已 redact）。
- [ ] **AC4** (PRD §Constraints 性能 / Phase 2 Exit Criteria): 单文件变更触发基础增量更新（工程目标 < 5s；不重建全量）。
- [ ] **AC5** (本 task 新增): Phase 2 端到端 smoke 可执行（index fixture → SQLite chunk 计数 + Tantivy 命中 + secret fixture 已 redact），为 phase spec §6 端到端 smoke 提供落点。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 索引 ≥1000 文件 | SCEN-2.4.1 | TEST-2.4.1 | - | unit-test | Not Started |
| AC2 SQLite+Tantivy 可查 | SCEN-2.4.2 | TEST-2.4.2 | - | unit-test | Not Started |
| AC3 denylist+redaction 生效 | SCEN-2.4.3 | TEST-2.4.3 | - | unit-test | Not Started |
| AC4 基础增量更新 | SCEN-2.4.4 | TEST-2.4.4 | - | unit-test | Not Started |
| AC5 Phase2 端到端 smoke | SCEN-2.4.5 | TEST-2.4.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R6**（大仓库索引性能/资源）：以真实大仓库基准持续测；超阈值降级后台长任务（完整硬化 Phase 8）。
- 关联 **R4**（redaction 在索引链路不可被绕过）。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。⚠️ 本 task 为 Phase 2 最后 task：完工/合并前 phase spec §6 端到端 smoke 必须填实（C1 / team §4 Gate 3）。

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
