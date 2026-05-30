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

## Amendment A1 — SemanticRecall@K gate (2026-05-30, Phase 18 task-18.8)

> Add-only amendment（不改既有 Decision；承 Phase 18 spec "ADR-006 §Acceptance Threshold 追加 SemanticRecall@K 阈值"）。Status：**Active**（ratified 2026-05-30 Phase 19 task-19.6——原 Proposed/provisional pending 真实 embedding provider 已满足，见下 **A1.4**）。

原 ADR-006 验收口径为 **BM25-only**（Strong/Weak/Miss + Top-5/10 命中率）。Phase 18（vector-backend-selection）让 retriever 具备向量召回能力（task-18.1 trait + task-18.3–18.6 backend + ADR-023 选型），故 recall-eval 口径扩展为 **BM25 + Semantic 双路**：

### A1.1 — SemanticRecall@K 指标

`internal/eval` 在既有 BM25 Strong-hit 口径之上，对**向量检索路径**的结果用同一 Strong-hit 判定计算 `SemanticRecall@K`（K=5,10）= top-K 内 strong 命中问题数 / 总问题数。`Report` 加 `SemanticEvaluated / SemanticStrongHits{5,10} / SemanticWeakHits / SemanticMisses / SemanticRecallAt{5,10}`；`SummarizeHybrid(bm25, semantic)` 双路汇总（无 semantic 结果时 `SemanticEvaluated=false`，退回 BM25-only）。

### A1.2 — Gate 阈值

| 指标 | 阈值 | 来源 |
|---|---|---|
| Top-5 strong rate | ≥ 0.75 | ADR-006（BM25，原口径） |
| Top-10 strong rate | ≥ 0.85 | ADR-006（BM25，原口径） |
| **SemanticRecall@10** | **≥ 0.70** | 本 amendment（Phase 18） |

`MeetsRecallGate(report)`：BM25 两项恒检；**SemanticRecall@10 仅在 `SemanticEvaluated` 时检**（即向量路径有结果时）——否则按 BM25-only 门禁，与生产现状一致。常量见 `internal/eval/eval.go`（`GateSemanticRecall10Min = 0.70`）。

### A1.3 — provisional（关键限制）

本 amendment 落地**度量 + 门禁 + 单测**（`SemanticRecall@K` 数学、双路汇总、阈值断言、空 semantic 退 BM25），但 **live 语义召回值尚不可得**：仓内无 embedding provider（`[SPEC-DEFER:phase-future.embedding-provider-full]`），向量 backend 亦未接入生产 retriever 热路径（`[SPEC-OWNER:phase-future.vector-retrieval-integration]`，ADR-023 D6）。故：

- SemanticRecall@10 ≥ 0.70 阈值为 **aspirational**，正式 ratify（含 ADR-023 D1 默认 backend）须待真实 embedding provider 接入后用真实分布语料复测。
- 生产 eval 当前仍为 BM25-only（`SemanticEvaluated=false`），门禁不强制 semantic 项。
- 合成种子向量上 4 路 backend recall 均 1.0（不可区分，见 `docs/spikes/phase-18-comparison.md`），故本 amendment 不据此 ratify 选型。

### A1.4 — Ratification (2026-05-30, Phase 19 task-19.6)

Add-only。A1.1（指标）/ A1.2（阈值表）/ A1.3（provisional）既有正文**不改**；本节据真实 embedding recall 把 A1 Status 从 Proposed/provisional 转为 **Active**，A1.3 的 aspirational/provisional 限制由本节 **superseded**（A1.3 原文保留作历史）。

Phase 19 补齐了 A1.3 列举的全部前提：task-19.1（real `EmbeddingProvider`：`fastembed-all-minilm-l6-v2`，dim 384）+ task-19.2（默认 backend 接 `Retriever` 生产热路径）+ task-19.3（`/v1/search?semantic=true` → core gRPC semantic 分派）。task-19.5 据此用 real provider 对真实 ContextForge 语料实测（`docs/spikes/phase-19-real-recall.md`，exact-cosine）：

| 指标 | 阈值（A1.2） | 实测（task-19.5 real） | 结论 |
|---|---|---|---|
| **SemanticRecall@10** | ≥ 0.70 | **0.9333**（28/30） | **PASS** |
| SemanticRecall@5（参考） | — | 0.8333（25/30） | — |
| top-1 accuracy / MRR（区分度） | — | 0.60 / 0.70 | 真实分布上可区分（非合成 1.0） |

故 A1.2 的 `SemanticRecall@10 ≥ 0.70` 阈值由 **aspirational 转为 active gate**：`MeetsRecallGate(report)` 在 `SemanticEvaluated` 时强制该项（`GateSemanticRecall10Min = 0.70` 不变）。ADR-013：实测来自 real `FastEmbedProvider` 运行，非合成 / 非 deterministic / 非伪造（数据源声明见 spike 文档）。ADR-023 默认 backend 选型同步 ratify（见 `docs/decisions/adr-023-vector-backend-default.md` Amendment / Ratification）。
