# Task `3.3`: `importer-openclaw — OpenClaw workspace 导入（通用 file/md/config/log）`

> **Status: Ready** — §2A 前置审核已完成，按 PRD O3 裁定仅做通用 workspace 导入。

**Status**: In Progress

**Priority**: P0
**Owner**: codex
**Related Phase**: Phase 3 (agent-importers)
**Dependencies**: 3.1 (importer-core)

## 1. Background

OpenClaw 是 PRD 列出的 P0 导入源。OpenClaw 具体 memory schema 为 PRD §Open Questions O3 的 TBD，故 v0.1 仅承诺 workspace 通用 file/markdown/config/log 导入（PRD §Constraints 兼容性 / §Core Capabilities Out of Scope「不完整复刻 OpenClaw memory engine」）。

## 2. Goal

`contextforge import openclaw <ws>` 按 workspace 通用方式导入 Markdown/config/log/memory-like 文件为 ContextRecord（source_provider=openclaw，按 agent/workspace name 建 collection），保留 file_path/source_modified_at/source_type/agent_scope；不复刻 OpenClaw 内部 memory engine、不写回。

## 3. Scope

### In Scope

- 新增 `internal/importer/openclaw` Go 子包，实现 OpenClaw workspace importer。
- 复用 task-3.1 `Importer` 抽象与 `FileFallbackImporter`，按通用 file / markdown / config / log / memory-like 文件导入为 `ContextRecord`。
- 设置 `source_provider=openclaw`、OpenClaw agent scope、workspace-derived collection id，并保留 `file_path`、`source_modified_at`、`source_type`。
- 对 OpenClaw schema-aware 解析保持 TBD：无法识别的 schema 走通用 fallback 并输出 warning。

### Out Of Scope

- OpenClaw 内部 memory schema-aware 解析、版本私有字段语义解析。
- 复刻、替换或写回 OpenClaw memory backend / workspace 文件。
- 修改 canonical proto 契约、lockfile 或新增依赖。
- CLI wiring / daemon API / indexer 端到端写入（后续 Phase 6 / Phase 4 集成）。

## 4. Users / Actors

- `contextforge import openclaw <ws>` CLI 调用方（后续接入）
- Go daemon 导入调度器（后续接入）
- `openclaw` importer 子包（本 task 实现方）
- task-3.1 fallback importer（本 task 复用方）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Constraints 兼容性 OpenClaw 范围 / §Core Capabilities Out of Scope / §Open Questions O3）
- `docs/specs/phases/phase-3-agent-importers.md`
- `docs/specs/tasks/task-3.1-importer-core.md`
- `docs/decisions/adr-005-readonly-import-draft-export.md`
- `test/features/importer.feature`

### 5.2 Imports

- `github.com/tajiaoyezi/contextforge/internal/importer`
- `github.com/tajiaoyezi/contextforge/proto/contextforge/v1`
- stdlib: `fmt`, `log`, `os`, `path/filepath`, `sort`, `strings`
- protobuf: `google.golang.org/protobuf/types/known/timestamppb`

### 5.3 函数签名

```go
// NewImporter creates an OpenClaw workspace importer.
func NewImporter(agentName string) importer.Importer

// CollectionID derives the default collection id from agent name and workspace name.
func CollectionID(workspacePath string, agentName string) string

// ImportWorkspace imports an OpenClaw workspace with the default importer.
func ImportWorkspace(path string, collectionID string, agentName string) ([]*contextforgev1.ContextRecord, error)
```

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Implementation Phases Phase 3 Exit Criteria): OpenClaw workspace 至少能按通用 file/markdown/config/log 方式导入为 ContextRecord。
- [ ] **AC2** (PRD §Constraints 兼容性): 按 agent name / workspace name 建 collection，保留 file_path/source_modified_at/source_type/agent_scope。
- [ ] **AC3** (PRD §Core Capabilities Out of Scope): 不复刻 OpenClaw 内部 memory engine、不替换其 backend、不自动写回 workspace。
- [ ] **AC4** (PRD §Technical Risks R5 / §Open Questions O3): OpenClaw 具体 memory schema 标 TBD；schema-aware 解析为后续增量增强，v0.1 不识别即走通用 fallback + warning。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 workspace 通用导入 | SCEN-3.3.1 | TEST-3.3.1 | - | unit-test | Test Red |
| AC2 collection/字段保留 | SCEN-3.3.2 | TEST-3.3.2 | - | unit-test | Test Red |
| AC3 不复刻/不写回 | SCEN-3.3.3 | TEST-3.3.3 | - | unit-test | Test Red |
| AC4 schema TBD 走 fallback | SCEN-3.3.4 | TEST-3.3.4 | - | unit-test | Test Red |

## 8. Risks

- 关联 PRD §Technical Risks **R5**（OpenClaw 版本漂移，概率高）：v0.1 只承诺通用导入；schema-aware 为增量。关联 PRD §Open Questions **O3**（需基于实测版本 + 真实 workspace 样本收集 fixture）。

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
