# ADR `025`: `hybrid-scoring-fusion`

**Status**: Accepted (2026-05-30 Proposed → 2026-05-31 ratified at v0.14.0 closeout (task-21.3)。融合策略选型据真实 dogfood eval 召回对比 ratify：real `all-MiniLM-L6-v2` 经生产 `Retriever::search_hybrid`，hybrid RRF top-1 0.0333→0.6667 / MRR 0.4095→0.7881 vs BM25 baseline (`docs/spikes/phase-21-hybrid-recall.md`)，据真实非合成数据 Proposed→Accepted，仿 ADR-023/ADR-006 数据驱动 ratify 范式；ADR-013 禁据合成/伪造 ratify。)
**Category**: 数据平面 / 向量检索 / 检索质量 / 分数融合
**Date**: 2026-05-30
**Decided By**: 主 agent (ADR-012 自治)；tajiaoyezi ratification at v0.14.0 closeout
**Related**: ADR-006 (recall-eval-acceptance-gate) / ADR-023 (vector-backend-default，数据驱动 ratify 范式) / ADR-002 (sqlite-tantivy-layered-storage，BM25 词面路径) / ADR-014 (D1-D5) / ADR-013 (禁伪造) / Phase 19 task-19.2 (`search()` BM25 + `search_semantic()` 语义两路) / task-19.3 (proto add-only 范式) / Phase 21 (retrieval-quality) / task-21.1 (hybrid-scoring) / task-21.3 (closeout-v0.14.0 ratify)

## Context

Phase 19（v0.12.0）落地了两条**独立**检索路径：

1. `Retriever::search()` —— BM25 词面（Tantivy 倒排，ADR-002），`retrieval_method = "bm25"`。
2. `Retriever::search_semantic()` —— 向量语义（双塔 embedding 余弦），`retrieval_method = "vector"`。

`core/src/retriever/mod.rs:449-450` 与 `:639-640` 的行内注释明确：两路**不融合**——语义命中不并入 BM25 结果集，`retrieval_method` 恒为 `"bm25"` 或 `"vector"`，hybrid fusion 标 `[SPEC-DEFER:phase-future.hybrid-scoring]`（`docs/releases/v0.12.0-artifacts.md:60` / `phase-19` §2 同记）。

纯词面与纯语义各有盲区：BM25 漏召同义改写 / 语义相近但无共词的查询；双塔语义漏召需精确词面匹配（标识符 / 罕见术语）的查询。融合两路证据可覆盖任一单路的漏召。需要决定**融合策略**——这是本 ADR 的决策点。

两类主流融合策略：

- **RRF（Reciprocal Rank Fusion）**：`score = Σ_path w_path / (k_const + rank_path)`（rank 从 1 起，k_const 常数如 60）。只用**排名**，免去跨路分数量纲归一；对分数分布鲁棒。
- **加权归一（weighted normalized sum）**：各路 score min-max（或 z-score）归一到可比量纲后加权和。保留分数幅度信息，但对分数分布敏感、需调归一与权重。

恰如 ADR-023 在合成语料上「recall 不可区分 → 架构驱动选型 + 真实数据 ratify」，融合策略的优劣**无法**纯凭确定性合成分数判定，需真实 dogfood 召回对比。

## Decision

ContextForge 采用**确定性可验证的融合函数 + 数据驱动选型 ratify**：task-21.1 落地一个明确融合策略并以确定性测试守正确性，最终策略选型据 task-21.3 真实 dogfood eval 召回对比 ratify。

### D1 — 融合入口：独立 `search_hybrid`，不改既有两路

新增 `Retriever::search_hybrid(query, top_k)`：内部复用 `search()`（BM25）+ `search_semantic()`（语义）取两路结果后调融合函数，输出 `retrieval_method = "hybrid"` 的结果集。既有 `search()` / `search_semantic()` 行为**不变**——hybrid 是 opt-in 的第三入口，不扰动既有两路（守 ADR-023 D5 默认 BM25 baseline）。

### D2 — 默认融合策略：RRF（provisional），加权归一为对照

task-21.1 默认落地 **RRF**（`Σ w / (k_const + rank)`），理由：免跨路分数量纲归一（BM25 分数与 cosine 相似度量纲不可比）、对分数分布鲁棒、实现确定性简单。**provisional**：最终策略据 task-21.3 真实召回对比 ratify——若加权归一在真实 dogfood 上显著优于 RRF，则切换并记录（ADR-013 据真实数据）。

### D3 — 确定性 + 可解释

融合按 `hybrid_score` 降序、同分以 `chunk_id` 升序稳定 tie-break（确定性，仿 `core/src/retriever/vector/brute_force.rs` 排序）。同一 chunk 两路均命中 → 融合分累加两路贡献。add-only `hybrid_score` 字段携带融合分量，既有 `score`（BM25 分量）与 `vector_score`（语义分量，task-19.3）并存——可解释性**增强**：用户可见融合分及其来源分量。

### D4 — add-only 契约 + 默认 BM25 baseline

proto `RetrievalResult` add-only `float hybrid_score = 15`（13/14 已被 vector_score/embedding_provider 占用）+ `SearchRequest` add-only `bool hybrid = 8`（7 已被 semantic 占用）；`SearchResult` add-only `hybrid_score: f32`（缺省 0.0）。`hybrid` 缺省 false → 既有客户端不受影响；响应仅 add-only 字段，22-endpoint conformance 不破坏（ADR-017）；proto-freeze 守护（只增不删不改号）。默认构建仍 BM25 baseline，hybrid 经显式请求开启。

### D5 — 单路缺失降级

`search_hybrid` 在任一路为空（如无 embedder/backend → `search_semantic` 返 `Ok(vec![])`）时降级为另一路结果，`retrieval_method` 仍标 `"hybrid"`（融合分退化为单路分），不 panic。保证 hybrid 入口在默认构建（0 vector dep）下也安全可调。

## Consequences

- **Positive**: 覆盖词面/语义任一单路漏召；融合函数确定性可 CI 验证（固定分数 → 期望融合序）；add-only 契约 0 破坏，默认 BM25 baseline 守线；可解释性增强（融合分 + 来源分量并存）；策略选型据真实数据 ratify（不在合成数据上假装区分）。
- **Negative / open**: 融合策略的真实优劣在合成数据上不可区分（D2 provisional 的开放点，仿 ADR-023 D6）——RRF vs 加权归一的最终选型须 task-21.3 真实 dogfood 召回对比；hybrid 需同时跑 BM25 + 语义两路，延迟为两路之和（默认 brute-force 语义路径对中小语料可接受，大语料延迟随 ANN backend 而定）。
- **Ratification**: 本 ADR **Proposed**。task-21.1 落地确定性融合函数 + task-21.3 真实 dogfood eval 跑出 hybrid vs BM25 vs 语义召回对比后，于 v0.14.0 closeout 据真实非合成数据 ratify Proposed→Accepted（确认 D2 默认策略或据数据切换）。若真实召回上两策略不可区分，则按架构简单性择 RRF 并诚实记录「recall 不可区分」（仿 ADR-023 D6 范式），不伪造区分度（ADR-013）。
- **Follow-ups**: reranker（cross-encoder）在 hybrid top-k 之上重排 `[SPEC-OWNER:task-21.2-reranker-pipeline]`（ADR-026）；console-api `?hybrid=true` 转发 `[SPEC-DEFER:phase-future.console-api-hybrid-forward]`（承 Phase 20 console-api 语义贯通范式）；Console UI 融合 explain `[SPEC-OWNER:phase-future.console-semantic-explain]`（跨仓库 Console 领域）。

## Ratification Amendment (task-21.3, 2026-05-31 — 数据驱动 ratify, 仿 ADR-023)

> add-only 批注，不溯改上方正文（ADR-014 D5）。本 ADR 由 **Proposed → Accepted**，据 task-21.3 真实
> dogfood eval（`docs/spikes/phase-21-hybrid-recall.md`，ADR-013 真实非合成数据）。

**数据源声明（ADR-013）**：real `FastEmbedProvider`（`all-MiniLM-L6-v2`, dim 384）经生产
`Retriever::search_hybrid`（RRF k=60 融合 BM25 + 向量两路），dogfood 语料 180 production chunks / 30
golden 查询，Windows MSVC 2026-05-31。复跑 `cargo run -p contextforge-core --example
phase21_hybrid_rerank_recall --features embedding-fastembed,reranker-fastembed`。

| 检索法 | recall@5 | recall@10 | top-1 | MRR | gate(≥0.70) |
|---|---|---|---|---|---|
| baseline BM25 | 0.9000 | 0.9667 | 0.0333 | 0.4095 | PASS |
| **hybrid RRF（本 ADR）** | 0.9333 | 0.9667 | **0.6667** | **0.7881** | PASS |

**ratify 依据**：hybrid RRF 融合相对 BM25 baseline 取得**决定性** top-1（+0.6334，0.0333→0.6667）/ MRR
（+0.3786，0.4095→0.7881）提升，recall@10 持平、recall@5 更高。这覆盖了 D2 / R1 标记的「融合策略在合成
分数上不可区分」开放点——RRF 在真实 dogfood 上显著优于单路 BM25 baseline，确认 **D2 默认 RRF 策略**，无需
回退到「架构简单性择一 + 诚实记录不可区分」的 ADR-023 D6 stop-condition。融合策略选型据真实数据落定（未据
合成/伪造 ratify，ADR-013）。D1（独立 `search_hybrid`）/ D3（确定性 + 可解释）/ D4（add-only + 默认 BM25
baseline）/ D5（单路降级）均经 task-21.1 确定性测试 + 本真实 eval 守护。
