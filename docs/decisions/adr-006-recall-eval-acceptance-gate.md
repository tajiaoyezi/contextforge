# ADR `006`: `recall-eval-acceptance-gate`

**Status**: Accepted
**Category**: 测试工具链
**Date**: 2026-05-17
**Decided By**: tajiaoyezi
**Related**: PRD §Decisions Log D6

## Context

ContextForge 的核心价值之一是回答"换 provider/embedding/参数后召回是否退化"（PRD §Problem Statement 痛点 5、§Core Capabilities #4）。普通单测无法覆盖多 Agent 上下文召回/provenance/迁移保真/本地索引质量的回归。

## Decision

recall eval 作为 PRD 级一等验收门：Go `go test` + Rust `cargo test`；内建 `contextforge eval run`（golden questions → Top-5/10 命中率/延迟/错误召回）作为 PRD 级验收。

## Rationale

仅单测无法回答"换 provider/embedding 后召回是否退化"（核心价值）；外部 RAG eval 框架多为 Python 生态、增加运行时/工程复杂度，且评测对象（多 Agent 上下文召回/provenance/迁移保真/本地索引质量）通用框架不能完全覆盖；人工抽检不可回归。

## Alternatives

- **仅单元测试不做 recall eval**：拒绝 —— 无法回归召回质量。
- **外部 RAG eval 框架（ragas 等）**：拒绝 —— Python 生态/复杂度，且覆盖不全；v0.1 内建轻量 eval，后续可导出数据兼容外部工具。
- **纯人工抽检**：拒绝 —— 不可回归。

## Consequences

> （init agent 初稿，用户审定）

- 正向：召回质量可回归、可横向对比 provider；与 Go+Rust 本地优先栈一致，无 Python/云依赖。
- 负向/成本：需构建并维护 golden questions 数据集（标注成本、防过拟合，O6）；eval 口径需严格定义（Strong/Weak/Miss、延迟不含远程）。
- 影响面：Phase 8 task 8.1 落 harness；Phase 4 起持续监控召回。

## Rollback Or Migration Plan

> （init agent 初稿，用户审定）

eval dataset 可导出 JSONL，便于回归与未来兼容外部评测工具；若内建 eval 不足，可在保留数据集的前提下叠加外部框架（新 ADR），数据不丢失。

## Follow-ups

- 关联 PRD §Technical Risks R3（检索召回率不达标）—— 分场景统计先达标再看总分。
- 关联 PRD §Open Questions O6（golden questions 数据集构建与维护）。
