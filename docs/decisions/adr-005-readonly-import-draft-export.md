# ADR `005`: `readonly-import-draft-export`

**Status**: Accepted
**Category**: 兼容性
**Date**: 2026-05-17
**Decided By**: tajiaoyezi
**Related**: PRD §Decisions Log D5

## Context

ContextForge 定位为多 Agent 中立的 Context Hub（PRD §Vision），需兼容 OpenClaw/Hermes/Claude Code/Cursor/Zed 等异构、版本不稳定的上下文源（PRD §Technical Risks R5），且不能"悄改用户 Agent memory"。

## Decision

只读导入 + 导出 draft/bundle，不写回第三方 Agent：导入 OpenClaw workspace / Hermes MEMORY.md·USER.md / agent-rules → canonical record；导出 canonical JSONL / Markdown bundle / agent draft；不自动写回。

## Rationale

双向写回风险高（悄改用户 Agent memory）且各 Agent schema 不稳定；仅支持单一 Agent 违背"多 Agent 中立"核心定位；私有格式不做 canonical 会锁死用户，违背"上下文不被锁定"价值主张。

## Alternatives

- **双向同步写回各 Agent 原生 memory**：拒绝 —— 风险高、schema 不稳定。
- **仅支持单一 Agent 格式**：拒绝 —— 违背多 Agent 中立定位。
- **私有格式不做 canonical**：拒绝 —— 锁死用户。

## Consequences

> （init agent 初稿，用户审定）

- 正向：用户数据不被锁定；不会因 ContextForge 误改第三方 Agent memory；canonical record 作为中立交换格式。
- 负向/成本：迁移是"导出 draft → 用户手动应用"，非无缝写回（v0.1 体验取舍）；外部 schema 漂移需 fixture 回归（R5）。
- 影响面：Phase 3（importers 分层 + fallback）、Phase 6（exporter draft/bundle）。

## Rollback Or Migration Plan

> （init agent 初稿，用户审定）

canonical record 为中立中介，新增导入源/导出格式为加法式扩展，不破坏既有；若未来确需写回某 Agent（v0.2+），作为显式 opt-in 能力叠加并新开 ADR，不改 v0.1 只读默认。

## Follow-ups

- 关联 PRD §Technical Risks R5（外部 Agent schema 不稳定）。
- 关联 PRD §Open Questions O3（OpenClaw/Hermes/Cursor/Zed schema 与路径）/ O5（canonical record 无损承载边界）/ O9。
