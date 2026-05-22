# Task `5.1`: `dedup — content/source hash 去重 + provenance 合并`

> **Status: Ready** — §2A 前置审核已完成，按 v0.1 MemoryOps 边界仅做 exact duplicate 去重。

**Status**: In Progress

**Priority**: P0
**Owner**: codex
**Related Phase**: Phase 5 (memoryops)
**Dependencies**: Phase 2 (索引产物), Phase 3 (importer 产出 record)

## 1. Background

MemoryOps 治理核心：同一事实跨多 Agent source 重复时去重并保留 provenance 链（PRD §User Flow 边界场景 / §Core Capabilities #3）。能力边界严格按 PRD「v0.1 MemoryOps 能力边界」：仅 normalized content hash / source hash / exact duplicate 去重。

## 2. Goal

`memoryops` 能基于 normalized content hash / source hash 检出 exact duplicate 并去重；provenance 链合并保留多个来源（不丢原始来源）；不做语义相似去重（边界外）。

## 3. Scope

### In Scope

- 新增 Go 子包 `internal/memoryops/dedup`，面向 Go 控制面消费 canonical `ContextRecord`。
- 基于现有 `ContentHash` 字段识别 exact duplicate；优先消费 task-2.3 chunker 的 `sha256:<64-hex>` hash，不重新计算内容 hash。
- 支持同一内容跨多 importer/source 的 provenance 链合并，保留 importer / original_path / source_modified_at。
- 保留 first-seen canonical record 作为代表记录，合并重复记录的 provenance、tags、agent_scope、security_labels 等集合字段。
- 显式测试语义相同但字面不同 / content_hash 不同的记录不会被去重。

### Out Of Scope

- LLM/embedding/向量语义相似去重。
- 语义冲突检测、stale 标记、生命周期策略、审计事件写入（task-5.2 / task-5.3）。
- 修改 chunker / indexer / importer / proto 契约。
- 重新计算、替换或迁移 content_hash；本 task 只消费上游已写入的 hash。
- SQLite/Tantivy 写路径集成与 CLI/API wiring。

## 4. Users / Actors

- MemoryOps 调度器（后续调用 dedup 后再进入 lifecycle / audit）
- Importer / indexer 产出的 canonical `ContextRecord` 列表（输入）
- Exporter / retriever / lifecycle 下游消费者（消费去重后的代表记录和 provenance 链）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Core Capabilities #3 + v0.1 MemoryOps 能力边界 / §User Flow 边界场景）
- `docs/specs/phases/phase-5-memoryops.md`
- `docs/specs/tasks/task-2.3-chunker.md`
- `docs/specs/tasks/task-3.1-importer-core.md`
- `docs/decisions/adr-005-readonly-import-draft-export.md`
- `test/features/memoryops.feature`

### 5.2 Imports

- `github.com/tajiaoyezi/contextforge/proto/contextforge/v1`
- stdlib: `sort`

### 5.3 函数签名

```go
// Result is the output of exact duplicate deduplication.
type Result struct {
    Records    []*contextforgev1.ContextRecord
    Duplicates []Duplicate
}

// Duplicate describes a record merged into a first-seen representative.
type Duplicate struct {
    RepresentativeID string
    DuplicateID      string
    ContentHash      string
}

// Records merges exact duplicate ContextRecords by ContentHash.
func Records(records []*contextforgev1.ContextRecord) Result
```

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Implementation Phases Phase 5 Exit Criteria): exact duplicate 能被去重（normalized content hash / source hash）。
- [ ] **AC2** (PRD §Implementation Phases Phase 5 Exit Criteria / §User Flow 边界场景): provenance 链能合并并保留多个来源，不丢失原始来源。
- [ ] **AC3** (PRD §Core Capabilities v0.1 MemoryOps 能力边界): **不做** LLM 语义相似去重 / 语义冲突判断（边界外，仅 exact duplicate）。
- [ ] **AC4** (本 task 新增): 去重锚点为 task 2.3 chunker 产出的 content_hash，保证同内容跨来源 hash 一致可去重。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 exact duplicate 去重 | SCEN-5.1.1 | TEST-5.1.1 | - | unit-test | Verified |
| AC2 provenance 链合并 | SCEN-5.1.2 | TEST-5.1.2 | - | unit-test | Verified |
| AC3 不做语义去重(边界) | SCEN-5.1.3 | TEST-5.1.3 | - | unit-test | Verified |
| AC4 content_hash 锚点 | SCEN-5.1.4 | TEST-5.1.4 | - | unit-test | Verified |

## 8. Risks

- 关联 PRD §Technical Risks **R5**：provenance 与 importer 解耦（content_hash 锚点）。关联 PRD §Open Questions **O5 / O9**（schema 无损承载边界）。

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
