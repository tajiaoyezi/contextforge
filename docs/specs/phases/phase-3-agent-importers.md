# Phase 3 · agent-importers

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。`/s2v-init` 生成，Status=Draft。§6 端到端 smoke 留 `<TBD-by-user>`，本 phase 最后一个 task 完工/合并前必须填实（`s2v_preflight_phase` C1）。

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

- Hermes `MEMORY.md` / `USER.md` 能导入为 canonical record
- OpenClaw workspace 至少能按通用 file / markdown / config / log 方式导入
- `AGENTS.md` / `CLAUDE.md` 能作为 agent_rule source 导入
- 不识别的 schema 会降级为通用文件导入并提示 warning，不中断导入

**端到端 smoke**：`<TBD-by-user>`（本 phase 最后一个 task 完工/合并前填实，例：对 `test/fixtures/shared/golden-hermes-memory/` + `golden-openclaw-workspace/` 跑 `contextforge import` → 校验产出 canonical record 字段 + 与 Phase 2 索引端到端贯通的 smoke 序列）

## 7. 阶段级风险

- 关联 PRD §Technical Risks **R5**（外部 Agent schema 不稳定 / 版本漂移，概率高）：importer 分层 —— 通用 file/markdown fallback 永远可用，schema-aware 解析为增量增强；不识别降级+警告不中断；每 importer 带版本探测 + 样本 fixture 回归；canonical record 与 importer 解耦。关联 PRD §Open Questions O3。

## 8. Phase Definition of Done

- [ ] 本 phase 全部 task spec Status=Done 或 Waived
- [ ] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过
- [ ] 关联风险 R5 缓解措施已落地（分层 importer + fallback + fixture 回归）
- [ ] adapter §Phase 状态索引该行 Status 同步更新
- [ ] team §4 Gate 3 phase smoke gate 通过后方可 merge 最后一个 task
