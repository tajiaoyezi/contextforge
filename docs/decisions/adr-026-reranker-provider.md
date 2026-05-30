# ADR `026`: `reranker-provider`

**Status**: Proposed (2026-05-30; Phase 21 task-21.2 起草。reranker 选型 = `Reranker` trait + 确定性 `IdentityReranker` 默认 + real `CrossEncoderReranker` feature-gated；real 模型真实质量数值据 task-21.3 真实 eval ratify Proposed→Accepted，受阻则如实记录维持 Proposed，ADR-013 禁据合成/伪造 ratify。)
**Category**: 数据平面 / 向量检索 / 检索质量 / 重排
**Date**: 2026-05-30
**Decided By**: 主 agent (ADR-012 自治)；tajiaoyezi ratification at v0.14.0 closeout
**Related**: ADR-008 (core-library-selection，reranker 新 dep add-only amendment) / ADR-004 (local-first-privacy-baseline，本地优先 + 远程/模型 opt-in) / ADR-006 (recall-eval-acceptance-gate) / ADR-023 (vector-backend-default，确定性默认 + feature-gated tier 范式) / ADR-013 (禁伪造) / ADR-014 (D1-D5) / Phase 19 task-19.1 (`EmbeddingProvider` trait + 确定性默认 + real feature-gated 范式) / Phase 21 (retrieval-quality) / task-21.2 (reranker-pipeline) / task-21.3 (closeout-v0.14.0 ratify) / ADR-025 (hybrid-scoring-fusion)

## Context

Phase 19（v0.12.0）落地的语义检索用**双塔 embedding 余弦**（query 与 doc 各自独立编码后算相似度）。cross-encoder reranker 把 query×doc **对**联合编码打分，比双塔更精准，可在初筛 top-k 之上重排提升 top-1 / MRR——但 cross-encoder 需真实模型，其真实质量数值需模型 + 真实 eval 验证（`docs/releases/v0.12.0-artifacts.md:59` / `phase-19` §2 记 `[SPEC-DEFER:phase-future.reranker]`）。

`core/src/embedding/{mod,traits}.rs`（task-19.1）已确立可复用范式，本 ADR 仿照之：

- **trait**：`EmbeddingProvider`（`Send + Sync + Debug`，object-safe `Arc<dyn ...>`，`#[non_exhaustive]` error 让下游 match add-only-safe）。
- **确定性默认实现**：`DeterministicEmbeddingProvider`（模型自由，默认构建可用，供 CI/smoke/test/wiring）。
- **real provider（feature-gated）**：`FastEmbedProvider`（`#[cfg(feature="embedding-fastembed")]`，默认构建 0 新 crate）。

reranker 的设计点：选什么 trait 抽象、确定性默认怎么做（让 CI 能验证重排管道而无需模型）、real cross-encoder 怎么 feature-gate、real 真实质量如何在 ADR-013 下诚实记录。这与 ADR-023「确定性 0-dep 默认（brute-force）+ feature-gated 真实 backend tier」一脉相承。

## Decision

ContextForge reranker 采用 **`Reranker` trait + 确定性 `IdentityReranker` 默认（0 模型依赖）+ real `CrossEncoderReranker`（feature-gated）**，仿 task-19.1 embedding provider 范式：

### D1 — `Reranker` trait（仿 `EmbeddingProvider`）

`core/src/rerank/traits.rs` 定义 `Reranker` trait：`Send + Sync + Debug`，`rerank(query: &str, candidates: &[SearchResult]) -> Result<Vec<SearchResult>, RerankError>`（输出按重排分降序的候选子集）+ `name(&self) -> &'static str`（provenance）。`RerankError` `#[non_exhaustive]`（仿 `EmbeddingError`），下游 match add-only-safe。object-safe（`Arc<dyn Reranker>`）以供 `Retriever::with_reranker` 注入。

### D2 — 确定性默认实现：`IdentityReranker`（0 模型依赖）

`core/src/rerank/identity.rs` 的 `IdentityReranker` 是默认构建可用的**确定性 identity 实现**（非占位）：按候选既有分数（`hybrid_score` > 0 取之，否则 `score`）降序 + `chunk_id` 升序稳定 tie-break 重排，不改候选内容、不丢候选，可在 `reason` 标注重排来源。它让 CI/测试无模型即可验证 **重排管道 wiring 正确性**（与 ADR-023 的 0-dep `BruteForceVectorBackend` 同精神——默认构建有一个真实可跑的确定性实现，把 real 模型留 feature-gated）。

### D3 — real provider feature-gated：`CrossEncoderReranker`

`core/src/rerank/cross_encoder.rs` 的 `CrossEncoderReranker` 是 real cross-encoder provider，`#[cfg(feature="reranker-<impl>")]`（具体 crate + feature 名由 task-21.2 候选评估选定，承 `embedding-fastembed` feature-gate 范式）。默认构建不编译（0 新 crate）。引入 real reranker crate 是 add-only dep（ADR-008 amendment，R7 走主 agent chore PR）。本地优先红线（ADR-004）：reranker 是 opt-in 增强，非运行前提；远程/模型不在默认路径。

### D4 — `Retriever::with_reranker` opt-in seam

`Retriever` add-only `with_reranker(Arc<dyn Reranker>)` builder（仿 `with_embedder` / `with_vector_searcher`）。`search_semantic` / `search_hybrid` 取 top-k 后，`Some(reranker)` → 应用重排；`None` → 不变（既有路径逐字段向后兼容）。rerank 不改既有两路/融合的默认行为。

### D5 — 真实质量据真实 eval ratify，受阻如实 defer

确定性 `IdentityReranker` 重排管道可 CI 验证（序稳定 + 候选不丢）；real `CrossEncoderReranker` 的 top-1/MRR 提升真实质量数值需模型 + 真实 dogfood eval 本地复跑（task-21.3）。ADR-013：真实质量只在 real 模型 run 下记录；确定性测试不冒充真实质量。stop-condition：real reranker provider 两平台均不可构建 / 模型不可得 → 确定性管道跑通 + 真实质量数值如实 defer（`[SPEC-DEFER:phase-future.reranker-real-quality]`），本 ADR 维持 Proposed，不伪造质量数值翻 Accepted。

## Consequences

- **Positive**: trait 抽象让 reranker 可换实现而不动检索核心；确定性 `IdentityReranker` 让默认构建有可跑的重排管道 + CI 验证 wiring（无模型依赖）；real cross-encoder feature-gated 隔离默认构建（0 新 crate）；本地优先红线守线（ADR-004，模型 opt-in）；与 task-19.1 embedding 范式 + ADR-023 tier 范式一致，认知负担低。
- **Negative / open**: real cross-encoder 真实质量是开放点——需模型 + 真实 eval，受阻平台不可得时如实 defer（D5 stop-condition），本 ADR 在真实质量数据到位前维持 Proposed；reranker 重排在 top-k 之上加一层计算，real 模型路径延迟随模型而定（默认 `IdentityReranker` 路径 0 模型成本）。
- **Ratification**: 本 ADR **Proposed**。task-21.2 落地 trait + `IdentityReranker` + feature-gated `CrossEncoderReranker` + task-21.3 真实 dogfood eval 跑出 real cross-encoder top-1/MRR 提升数据后，于 v0.14.0 closeout 据真实非合成数据 ratify Proposed→Accepted；若 real 模型质量受阻不可得，则诚实记录维持 Proposed（确定性管道仍闭环，但 real 质量这条如实 defer，ADR-013）。
- **Follow-ups**: real cross-encoder 真实质量在受阻平台复跑 `[SPEC-DEFER:phase-future.reranker-real-quality]`；reranker crate dep 引入 `[SPEC-OWNER:phase-future.reranker-dep-chore]`（主 agent chore PR）；console-api `?rerank=true` 转发 `[SPEC-DEFER:phase-future.console-api-rerank-forward]`（承 Phase 20 范式）；Console UI 重排 explain `[SPEC-OWNER:phase-future.console-semantic-explain]`（跨仓库 Console 领域）；hybrid scoring 融合 `[SPEC-OWNER:task-21.1-hybrid-scoring]`（ADR-025）。
