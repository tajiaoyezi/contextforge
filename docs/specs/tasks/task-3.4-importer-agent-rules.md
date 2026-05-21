# Task `3.4`: `importer-agent-rules — AGENTS.md / CLAUDE.md / Cursor·Zed rules 导入`

> **Status: Done** — RED/GREEN/REFACTOR + §9 验证全绿 + §10 回填完成。

**Status**: Done

**Priority**: P1
**Owner**: grok (per dispatch 2026-05-21; §2A fill)
**Related Phase**: Phase 3 (agent-importers)
**Dependencies**: 3.1 (importer-core)

## 1. Background

项目级规则文件（AGENTS.md / CLAUDE.md / Cursor·Zed rules）是 PRD P0 导入源，作为 agent_rule source 导入。Cursor/Zed 具体路径与格式为 PRD §Open Questions O3 的 TBD，v0.1 当作 project instruction / agent rule source 处理，不做深度语义写回（PRD §Constraints 兼容性 / §Core Capabilities Out of Scope）。

## 2. Goal

`contextforge import agent-rules <path>` 把 AGENTS.md / CLAUDE.md / Cursor·Zed 规则类 Markdown 导入为 source_type=`agent_rule` 的 ContextRecord；不写回这些文件。

## 3. Scope

### In Scope

- `internal/importer/agentrules/` 子包实现 `AgentRulesImporter`（实现 `Importer` 接口）
- `Detect`：对 `AGENTS.md` / `CLAUDE.md`（及大小写变体）返回高 confidence（AC1）
- `Import`：读取文件内容作为 markdown，调用 `buildRecord`（复用 task-3.1）设置 `source_type=agent_rule`、`provider=claude-code|cursor|zed|local`、`tags` 含类型、`redaction_status=pending`（AC1/AC2）
- 支持直接构造 agent-rules importer 导入任意规则类 Markdown（包括 Cursor/Zed TBD 路径），标记为 agent_rule（AC2）
- `Resolve` 未匹配的 Cursor/Zed 路径走 `FileFallbackImporter` + 显式 warning（AC4，复用 3.1 框架）
- 只读导入，`init()` 注册到全局 registry（供未来 CLI/daemon 空白导入触发）

### Out Of Scope

- 深度语义解析规则文件（指令、优先级、tool-use 提取等，PRD Out of Scope）
- 自动发现或硬编码 Cursor/Zed 工作区具体路径（O3 TBD，v0.1 由 `import agent-rules <path>` 手动指定）
- 写回或修改原 AGENTS.md / CLAUDE.md / Cursor·Zed 文件（ADR-005 / AC3）
- 任何新外部依赖（R7，已声明 NEEDS-DEP-task-3.4.md 无增量）

## 4. Users / Actors

- `contextforge import agent-rules <path>` CLI 子命令（Phase 6 消费，直接调用或 Resolve 后的 agent-rules importer）
- Go daemon 导入调度器（未来通过 registry 解析 agent-rules 路径）
- `AgentRulesImporter`（实现方，Name="agent-rules"）
- `FileFallbackImporter`（TBD 路径保底，AC4）
- 下游 `scanner` / `indexer`（消费 record，redaction 标记为 pending 由 2.1/2.4 处理）
- 项目规则维护者（用户提供 AGENTS.md / CLAUDE.md / Cursor rules 路径）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Constraints 兼容性 Claude Code/Cursor/Zed 范围 / §Open Questions O3）
- `docs/specs/phases/phase-3-agent-importers.md`
- `docs/specs/tasks/task-3.1-importer-core.md`
- `docs/decisions/adr-005-readonly-import-draft-export.md`
- `test/features/importer.feature`

### 5.2 Imports

- `internal/importer`（`Importer` 接口、`Register`、`buildRecord`、`recordInput`、`NewFileFallbackImporter` 复用）
- `proto/contextforge/v1`（`ContextRecord`、`Provenance` 生成类型，frozen by 1.1）
- stdlib: `os`, `path/filepath`, `strings`, `log`, `fmt`

### 5.3 函数签名

```go
// AgentRulesImporter 实现 project instruction / agent rule 文件的只读 importer。
// Detect 仅匹配稳定文件名 AGENTS.md / CLAUDE.md（高 confidence）；
// Cursor/Zed 路径 TBD → 不匹配（Resolve 走 fallback + warning，AC4）。
// 直接构造时支持任意规则 Markdown 并标记 source_type=agent_rule（AC2）。
type AgentRulesImporter struct{ /* ... */ }

// NewAgentRulesImporter 创建 agent-rules importer 实例（init() 自动 Register）。
func NewAgentRulesImporter() Importer

// 满足 Importer 接口：
func (a *AgentRulesImporter) Name() string
func (a *AgentRulesImporter) Detect(path string) (confidence float64, ok bool)
func (a *AgentRulesImporter) Import(path string, collectionID string) ([]*contextforgev1.ContextRecord, error)

// init 触发注册（供 CLI/daemon 侧 `_ "github.com/tajiaoyezi/contextforge/internal/importer/agentrules"` 激活）
func init() { importer.Register(NewAgentRulesImporter()) }
```

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [x] **AC1** (PRD §Implementation Phases Phase 3 Exit Criteria): `AGENTS.md` / `CLAUDE.md` 能作为 `agent_rule` source 导入为 ContextRecord。
- [x] **AC2** (PRD §Constraints 兼容性): Cursor / Zed 规则类 Markdown 能导入（路径/格式 TBD → 走通用 markdown + agent_rule 标记）。
- [x] **AC3** (PRD §Decisions Log D5 / §Core Capabilities Out of Scope): 只读导入，不写回 AGENTS.md/CLAUDE.md/Cursor·Zed rules，不做深度语义写回。
- [x] **AC4** (PRD §Open Questions O3): Cursor/Zed 具体路径与格式标 TBD，v0.1 不识别即走通用 fallback + warning，不中断。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 AGENTS/CLAUDE 导入 | SCEN-3.4.1 | TEST-3.4.1 | - | unit-test | Done |
| AC2 Cursor/Zed rules 导入 | SCEN-3.4.2 | TEST-3.4.2 | - | unit-test | Done |
| AC3 只读不写回 | SCEN-3.4.3 | TEST-3.4.3 | - | unit-test | Done |
| AC4 路径 TBD 走 fallback | SCEN-3.4.4 | TEST-3.4.4 | - | unit-test | Done |

## 8. Risks

- 关联 PRD §Technical Risks **R5**：Cursor/Zed 规则文件格式漂移。关联 PRD §Open Questions **O3**（需各工具当前版本实测）。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。⚠️ 本 task 为 Phase 3 最后 task（之一，与 3.2/3.3 并列末批）：Phase 3 最后合并的 task 完工前 phase spec §6 端到端 smoke 必须填实（C1 / team §4 Gate 3）。

## 10. Completion Notes

- **完成日期**：2026-05-21
- **改动文件**：
  - `internal/importer/agentrules/agentrules.go`（新增：AgentRulesImporter + Detect/Import + build + init 注册）
  - `internal/importer/agentrules/agentrules_test.go`（新增：TEST-3.4.1~3.4.4 覆盖 4 AC / SCEN）
  - `test/features/importer.feature`（更新：SCEN-3.4.1~3.4.4 Given/When/Then 填实）
  - `docs/specs/tasks/task-3.4-importer-agent-rules.md`（§2A 填实 §3/§4/§5.2/§5.3/Owner/Status Ready→Done + §6 [x] + §7 追踪 Done + §10 回填）
  - `NEEDS-DEP-task-3.4.md`（新增：R7 声明无新依赖）
- **commit 列表**：
  - `c2c8684` test(agentrules): 加 SCEN-3.4.1~3.4.4 4 个 RED 测试 + stub 骨架；§2A 填实 + NEEDS-DEP + feature
  - `6172278` feat(agentrules): 实现 AgentRulesImporter 通过全部 4 个测试 (AC1-4)
- **§9 Verification 结果**：
  - install: ✅ (go mod download && cargo fetch)
  - typecheck: ✅ (go vet ./... && cargo check --workspace)
  - unit-test: 4 passed / 0 failed (agentrules: TEST-3.4.1~3.4.4) / 全量 go test ./... 0 failed（零回归）
- **剩余风险 / 未做项**：
  - buildRecord 逻辑在 subpkg 重复（因 3.1 未导出构造函数 + 并行任务禁改 core）；未来 core 导出后可去重（低风险）。
  - Phase 3 §6 端到端 smoke 暂未填实，待主 agent 在最后合并 task 的 §4 Gate 3 处理（避开任何 <...> 形式 token）。
- **下游 task 影响**：无（Phase 3 最后批次 3.2/3.3/3.4 并行，无后续 3.x 依赖本实现细节）
