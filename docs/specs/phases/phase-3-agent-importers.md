# Phase 3 · agent-importers

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。**Status=Done（2026-05-21 收口）**；§6 端到端 smoke 已填实并跑过，4 importer task 全 Done（task-3.1 PR #7 / task-3.2 PR #21 / task-3.3 PR #20 / task-3.4 PR #19 / chore phase-3-closeout）。

## 1. 阶段目标

`contextforge import openclaw/hermes/agent-rules` 把外部源转为 canonical record（与 Phase 2 集成后端到端入索引）。来源：PRD §Implementation Phases Phase 3。

## 2. 业务价值

实现 PRD 核心能力 #5（跨 Agent 上下文迁移）的导入侧与 #1 的多 Agent 接入：把分散在 OpenClaw workspace / Hermes MEMORY.md·USER.md / AGENTS.md·CLAUDE.md 的上下文统一成 canonical record。支撑成功指标「跨 Agent 迁移保真 ≥ 2 种导入源」。

## 3. 涉及模块

- `agent-importer`（Go）：Agent 适配编排（openclaw-workspace / hermes-memory / agent-rules importer）+ canonical record 映射
- 文件锚点：`internal/importer/`（core / hermes / openclaw / agentrules 子包）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 3.1 | importer | `../tasks/task-3.1-importer-core.md` |
| 3.2 | importer | `../tasks/task-3.2-importer-hermes.md` |
| 3.3 | importer | `../tasks/task-3.3-importer-openclaw.md` |
| 3.4 | importer | `../tasks/task-3.4-importer-agent-rules.md` |

## 5. 依赖关系

- **依赖**：Phase 1（canonical record schema + proto 契约）
- **可并行**：是 —— 可与 Phase 2（index-core）并行；与 Phase 2 集成后端到端入索引
- **Phase 内顺序**：3.1 importer-core（框架+映射+fallback）先行 → 3.2 hermes ∥ 3.3 openclaw ∥ 3.4 agent-rules（均 dep 3.1）

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（参考 — PRD §Implementation Phases Phase 3 Exit Criteria，用户审定后落实）**：

- [x] Hermes `MEMORY.md` / `USER.md` 能导入为 canonical record（task-3.2 / PR #21）
- [x] OpenClaw workspace 至少能按通用 file / markdown / config / log 方式导入（task-3.3 / PR #20）
- [x] `AGENTS.md` / `CLAUDE.md` 能作为 agent_rule source 导入（task-3.4 / PR #19）
- [x] 不识别的 schema 会降级为通用文件导入并提示 warning，不中断导入（task-3.1 fallback 框架 / 3.2/3.3/3.4 AC4 复用）

**端到端 smoke**（2026-05-21 chore PR `chore/phase-3-closeout` 验证）：

`go test -count=1 ./internal/importer/...` 全绿（master HEAD `6908084`）：

- `internal/importer/` (task-3.1 core): tests pass — buildRecord / Resolve fallback / SCEN-3.1.1~3.1.5
- `internal/importer/hermes/` (task-3.2): 4 tests pass — SCEN-3.2.1~3.2.4
- `internal/importer/openclaw/` (task-3.3): 5 tests pass — SCEN-3.3.1~3.3.4 + FIX-1 RedactionStatus assert
- `internal/importer/agentrules/` (task-3.4): 4 tests pass — SCEN-3.4.1~3.4.4

跨 importer 一致性 verify：
- ContextRecord.redaction_status="pending" 跨 4 importer 全设（task-3.1 §10 Waiver BINDING；下游 scanner/indexer 实际脱敏）
- content_hash 用 sha256（与 task-2.3 chunker 跨 module 算法一致，实证 64-hex 字面相同）
- provenance.importer 唯一标识各 source provider（fallback / hermes / openclaw / agent-rules）

**Scope 注**：phase smoke 用 Go test 端到端验证 `importer.Resolve()` + `Import()` flow（CLI `contextforge import` 子命令在 Phase 6 task-6.1 实现，本 phase 不依赖）。完整 CLI 端到端 smoke 留 Phase 8 task-8.3 release smoke 处理。

## 7. 阶段级风险

- 关联 PRD §Technical Risks **R5**（外部 Agent schema 不稳定 / 版本漂移，概率高）：importer 分层 —— 通用 file/markdown fallback 永远可用，schema-aware 解析为增量增强；不识别降级+警告不中断；每 importer 带版本探测 + 样本 fixture 回归；canonical record 与 importer 解耦。关联 PRD §Open Questions O3。

## 8. Phase Definition of Done

- [x] 本 phase 全部 task spec Status=Done 或 Waived（3.1/3.2/3.3/3.4 全 Done）
- [x] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过
- [x] 关联风险 R5 缓解措施已落地（分层 importer + task-3.1 fallback 框架 + 3.2/3.3/3.4 通过 Resolve 复用 fallback；schema-aware 留 follow-up 增量）
- [x] adapter §Phase 状态索引该行 Status 同步更新（chore PR `chore/phase-3-closeout`）
- [x] team §4 Gate 3 phase smoke gate **后置补偿**（实际 merge 顺序：PR #21/#19/#20 → 本 chore PR 收口；smoke 已通过不影响业务，已 audit）
