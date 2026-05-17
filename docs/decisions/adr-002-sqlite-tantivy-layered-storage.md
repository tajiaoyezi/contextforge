# ADR `002`: `sqlite-tantivy-layered-storage`

**Status**: Accepted
**Category**: 数据持久化
**Date**: 2026-05-17
**Decided By**: tajiaoyezi
**Related**: PRD §Decisions Log D2

## Context

ContextForge v0.1 P0 是可解释 BM25/metadata baseline、本地优先、离线可用（PRD §Decisions Log D2、§Constraints）。需要存 metadata/chunk/provenance 并提供全文检索，且不强依赖云/向量后端。

## Decision

分层本地存储：SQLite 存 metadata/chunk/provenance + Tantivy 全文索引；向量后端做 provider 抽象，v0.1 不强依赖。

## Rationale

纯向量库起步会过早把 v0.1 绑定到 embedding/vector pipeline，增加模型/向量维度/重建索引/provider 选择复杂度，而 v0.1 P0 目标是可解释 BM25/metadata baseline；ES/OpenSearch 引入 JVM + 重部署，与单机本地优先冲突；纯 SQLite FTS 解释性与打分能力弱于 Tantivy。

## Alternatives

- **纯向量库（Qdrant/LanceDB）起步**：拒绝 —— 过早绑定 embedding/vector pipeline，复杂度高，P0 不需要。
- **Elasticsearch/OpenSearch**：拒绝 —— JVM + 部署重，违背单机本地优先。
- **纯 SQLite FTS**：拒绝 —— 解释性与打分能力弱于 Tantivy。

## Consequences

> （init agent 初稿，用户审定）

- 正向：本地零云依赖、离线可用；Tantivy 提供可解释打分，SQLite 提供结构化 metadata 查询与单文件可移植。
- 负向/成本：双存储一致性需维护（chunk 同时落 SQLite 与 Tantivy）；向量后端推迟带来 P1 hybrid search 选型风险（R2）。
- 影响面：indexer/retriever/memoryops 均依赖该分层；磁盘索引大小受 1.5x-3x 约束。

## Rollback Or Migration Plan

> （init agent 初稿，用户审定）

向量后端经 provider 抽象隔离，新增/更换后端不动检索 API；若 SQLite 成为瓶颈，可在保持 schema 的前提下迁移底层（评估 sqlx/其他嵌入式库，须新 ADR）。Tantivy 索引可由 SQLite 源记录全量重建（SQLite 为事实源）。

## Follow-ups

- 关联 PRD §Technical Risks R2（向量后端选型悬而未决）—— Phase 5-6 期间 spike 压测后定。
- 关联 PRD §Open Questions O2（向量后端最终选型）。
