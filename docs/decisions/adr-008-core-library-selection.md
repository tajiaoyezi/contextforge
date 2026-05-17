# ADR `008`: `core-library-selection`

**Status**: Accepted
**Category**: 依赖
**Date**: 2026-05-17
**Decided By**: tajiaoyezi
**Related**: PRD §Decisions Log D8

## Context

ContextForge 需在 Go + Rust 两侧选定核心库（PRD §Technical Approach 技术栈、§Decisions Log D8）。避免重复造轮子且契合本地优先/单文件可移植。

## Decision

核心库选成熟生态：Rust = tantivy + tree-sitter + pulldown-cmark + tokio + tonic + rusqlite/sqlx(SQLite，rusqlite 优先，async-heavy 再评估 sqlx)；Go = cobra + chi + grpc-go + slog。

## Rationale

自研全文索引/分词重复造轮子且质量不可控；sled/RocksDB 对结构化 metadata 查询不如 SQLite 直观，且 SQLite 单文件可移植契合本地优先；gin/echo 中间件偏重，chi 轻量贴近 net/http 已足够 v0.1。

## Alternatives

- **自研全文索引/分词**：拒绝 —— 重复造轮子，质量不可控。
- **sled/RocksDB 替 SQLite**：拒绝 —— 结构化 metadata 查询不如 SQLite 直观，SQLite 单文件可移植契合本地优先。
- **Go 侧 gin/echo 替 chi**：拒绝 —— 中间件偏重，chi 轻量已足够 v0.1。

## Consequences

> （init agent 初稿，用户审定）

- 正向：成熟库降低实现/质量风险；tantivy 提供可解释打分，tree-sitter 多语言解析，SQLite 单文件可移植。
- 负向/成本：tantivy/tree-sitter API 演进需跟随；rusqlite vs sqlx 在 async-heavy 场景待评估（D8 已标注 rusqlite 优先）。
- 影响面：Phase 1（tonic/grpc-go/cobra/chi）、Phase 2（tantivy/tree-sitter/pulldown-cmark/SQLite）。

## Rollback Or Migration Plan

> （init agent 初稿，用户审定）

依赖经模块边界隔离（scanner/parser/chunker/indexer/retriever 各自封装第三方库）；单库替换局限在对应模块 + 受 §9 unit-test 保护；R7（lockfile 保护）确保依赖变更走专门 PR。SQLite 客户端 rusqlite→sqlx 切换在保持 schema 前提下可迁移（新 ADR）。

## Follow-ups

- 关联 PRD §Technical Risks R8（中英文/代码符号检索 — tantivy tokenizer 选型）。
- 关联 PRD §Open Questions O11（中英文与代码符号检索策略）。
