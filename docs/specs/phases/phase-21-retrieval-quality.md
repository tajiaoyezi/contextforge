# Phase 21 · retrieval-quality

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。本 phase 在 Phase 19（v0.12.0）落地的「BM25 单路 / 语义单路 + BM25 fallback」之上，提供 **hybrid scoring（BM25 + 向量分数融合）** 与 **reranker（cross-encoder）**，提升 top-k 排序质量。融合函数与 reranker 的**管道 + 确定性 identity 实现**在默认构建（0 模型依赖）下可无人值守 CI 验证；real cross-encoder 模型的**真实质量数值**经 feature-gated provider 如实记录、受阻如实 defer（ADR-013）。v0.14.0 收口。对应 `docs/roadmap.md` §3.2。
>
> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md` §3.2（范围 / marker 来源 / 可验证性分级）→ `core/src/retriever/mod.rs`（`SearchResult` 12-field struct + `search()` BM25 路径 + `search_semantic()` 语义路径 + 行内 `[SPEC-DEFER:phase-future.hybrid-scoring]` 标记 @ 450/640）→ `core/src/embedding/{mod,traits}.rs`（`EmbeddingProvider` trait 风格：`Send + Sync + Debug` + `#[non_exhaustive]` error，供 `Reranker` trait 仿照）→ `core/src/retriever/vector/{traits,brute_force}.rs`（`VectorSearcher` + 0-dep `BruteForceVectorBackend` 确定性默认范式）→ `internal/eval/eval.go`（`Report` / `SummarizeHybrid` / `MeetsRecallGate`，hybrid/reranked 列扩展面）→ `internal/cli/eval.go`（`--semantic` flag 范式，供 `--rerank` 仿照）→ `core/Cargo.toml` features（`embedding-fastembed` feature-gate 范式，供 reranker feature 仿照）→ AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5）→ `docs/decisions/adr-023-vector-backend-default.md`（数据驱动 ratify 范式）+ `docs/decisions/adr-006-recall-eval-acceptance-gate.md`（recall gate）。
>
> **ADR 影响面（已识别）**：
> - **ADR-025 hybrid-scoring-fusion（新，Proposed）**：记融合策略选型（RRF 加权常数 vs 加权归一），数据驱动 ratify——确定性融合序可 CI 验证，真实召回对比数值经 task-21.3 真实 eval 跑出后于 v0.14.0 closeout ratify（仿 ADR-023 / ADR-006 数据驱动 ratify 范式）。
> - **ADR-026 reranker-provider（新，Proposed）**：记 reranker 选型——`Reranker` trait + 确定性 identity 实现（默认构建 0 模型依赖）+ real cross-encoder provider（feature-gated）；real 模型真实质量数值如实 defer，受阻文档化 stop-condition（ADR-013）。
> - 可能触及 **ADR-008（core-library-selection）**：reranker real provider 引入新 feature-gated crate，以 add-only amendment 记录（不溯改正文，D5）。

## 1. 阶段目标

v0.14.0 ship 后，ContextForge 在已有 BM25 / 语义双路基础上，提供两种排序质量增强：

- **hybrid scoring**：把 BM25 词面分数与向量语义分数融合为单一 hybrid 排序（融合函数 + `retrieval_method = "hybrid"` + add-only `hybrid_score` 字段），覆盖「纯词面或纯语义任一单路都漏召」的查询。
- **reranker**：在初筛 top-k 之上用 cross-encoder 重排序（`Reranker` trait + 确定性 identity 实现默认可用 + real cross-encoder provider feature-gated）。

二者均 opt-in，默认构建（0 模型依赖、0 新 crate）行为不变：默认仍为 BM25 baseline，hybrid 与 rerank 经显式请求字段开启。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. hybrid scoring 融合函数落地（RRF 或加权归一）+ `retrieval_method = "hybrid"` + add-only `hybrid_score` 字段；固定 BM25/vector 分数 → 期望融合序确定性单测可断言（AC1）
2. `Reranker` trait + 确定性 identity-reranker（默认构建 0 模型依赖）落地，确定性重排序管道 CI 可断言；real cross-encoder provider feature-gated，真实质量数值经真实 eval 记录或受阻如实 defer（ADR-013）（AC2）
3. eval 报告加 hybrid / reranked 召回列 + smoke 升级（hybrid / rerank opt-in 路径真实断言）+ 既有 step 不退化（AC3）
4. v0.14.0 release docs + phase §6 闭合 + ADR-025 / ADR-026 据真实数据 ratify 或如实记录维持 Proposed（AC4）
5. ADR-014 D1-D5（第十二次激活）全通过（AC5）

**v0.x 版本号决策**：v0.14.0 minor release（检索质量：hybrid + reranker；默认构建仍 BM25-only baseline——hybrid 与 rerank 均 opt-in，add-only 请求/响应字段不破坏既有客户端）。

## 2. 业务价值

直接兑现 `docs/roadmap.md` §3.2 的两条 marker，承 PRD §Core Capabilities 检索质量目标：

- **hybrid scoring**（`[SPEC-DEFER:phase-future.hybrid-scoring]`，来源 `core/src/retriever/mod.rs:450/640` 行内标记 + `phase-19` §2 + `v0.12.0-artifacts` §7）：v0.12.0 语义路径与 BM25 路径**各自独立**（语义命中不并入 BM25 结果集，`retrieval_method` 恒 `"bm25"` 或 `"vector"`），无融合。本 phase 提供融合，让两路证据互补，提升对「词面与语义任一单路漏召」查询的召回。
- **reranker**（`[SPEC-DEFER:phase-future.reranker]`，来源 `phase-19` §2 + `v0.12.0-artifacts` §7）：cross-encoder 对 query×doc 对联合编码打分，比双塔 embedding 的余弦更精准，可在初筛 top-k 上重排提升 top-1 / MRR。本 phase 提供 reranker 管道与确定性默认实现，real 模型质量经 feature-gated provider 真实 eval 验证。
- **PRD §Core Capabilities #1（可解释召回）**：hybrid 结果保留 `vector_score`（语义分量）与 BM25 `score`（词面分量）并新增 `hybrid_score`（融合分量），可解释性增强而非退化；reranker 结果保留初筛分与重排分双轨。

**不在本 phase scope**：

- Remote embedding provider（OpenAI / Cohere）[SPEC-DEFER:phase-future.embedding-provider-remote]——v0.15.0 / Phase 22
- Embedding 缓存 [SPEC-DEFER:phase-future.embedding-cache]——v0.15.0 / Phase 22
- 完整 embedding provider 配置 / 选择层 [SPEC-OWNER:phase-future.embedding-provider-full]——v0.15.0 / Phase 22
- hnsw 图持久化 [SPEC-DEFER:phase-future.hnsw-graph-persistence]——v0.16.0 / Phase 23
- sqlite-vec Windows MSVC 跨平台 [SPEC-DEFER:phase-future.sqlite-vec-cross-platform]——v0.16.0 / Phase 23
- Console UI 语义 / 重排 explain 面板（cross-repo Console 领域）[SPEC-OWNER:phase-future.console-semantic-explain]
- real cross-encoder 模型的真实质量数值在受阻平台的复跑 [SPEC-DEFER:phase-future.reranker-real-quality]——如实 defer，受阻文档化 stop-condition（ADR-013）

## 3. 涉及模块

### 21.1 hybrid scoring（task-21.1）

- 新增 `core/src/retriever/fusion.rs`（或 `core/src/retriever/mod.rs` 内函数）——融合函数（RRF 加权常数 / 加权归一，二选一由 ADR-025 选型），输入 BM25 结果集 + 向量结果集 → 输出按 hybrid 分数排序的 `SearchResult` 集，`retrieval_method = "hybrid"`，`hybrid_score` 填实
- 修改 `core/src/retriever/mod.rs`——新增 `search_hybrid(query, top_k)` 入口（复用 `search()` BM25 + `search_semantic()` 语义两路结果后融合）；`SearchResult` add-only `hybrid_score: f32` 字段（缺省 0.0）
- 修改 `proto/contextforge/v1/search.proto`——`RetrievalResult` add-only `float hybrid_score = 15`（field 13/14 已被 vector_score/embedding_provider 占用，15 为下一空号）+ `SearchRequest` add-only `bool hybrid = 8`（buf 重生成）
- 同源 Rust tests（≥3：固定 BM25/vector 分数 → 期望融合序确定性断言 + `retrieval_method="hybrid"` + `hybrid_score` 填实 + 单路缺失时降级）

### 21.2 reranker-pipeline（task-21.2）

- 新增 `core/src/rerank/{mod,traits,identity}.rs`——`Reranker` trait（仿 `EmbeddingProvider` trait 风格：`Send + Sync + Debug` + `#[non_exhaustive]` error）+ `IdentityReranker`（确定性 identity 实现：保持输入序 / 确定性稳定排序，默认构建 0 模型依赖，供 CI/测试）
- 新增 `core/src/rerank/cross_encoder.rs`——`CrossEncoderReranker`（real cross-encoder provider，feature-gated `reranker-fastembed` 或同类，承 `core/Cargo.toml` `embedding-fastembed` feature-gate 范式；默认构建 0 新 crate）
- 修改 `core/Cargo.toml`——add-only feature `reranker-<impl>`（optional dep，default 不含；R7 dep 走主 agent chore PR）
- 修改 `core/src/retriever/mod.rs`——`with_reranker(Arc<dyn Reranker>)` builder seam（仿 `with_embedder` / `with_vector_searcher`）+ rerank 在 `search_semantic` / `search_hybrid` top-k 之上 opt-in
- 同源 Rust tests（≥2：`IdentityReranker` 确定性重排序管道 CI 可断言 + 确定性 fixture 下 rerank 序稳定；real provider 经 feature-gated 入口本地复跑数值，CI 不构建模型）

### 21.3 closeout-v0.14.0（task-21.3）

- 修改 `internal/eval/eval.go`——`Report` add-only hybrid / reranked 召回列（仿既有 `SemanticRecall@K` add-only 字段 + `SummarizeHybrid` 扩展）+ `internal/cli/eval.go` add-only `--rerank` flag（仿 `--semantic` flag）
- 修改 `scripts/console_smoke.sh`——升级版：hybrid / rerank opt-in 路径真实断言（响应 `retrieval_method` 反映 hybrid 路径 / `hybrid_score` provenance），既有 step 不退化
- 新增 `docs/releases/v0.14.0-{evidence,artifacts}.md` + `README.md` v0.14 段 + `RELEASE_NOTES.md` v0.14.0 段
- 修改 `docs/decisions/adr-025-hybrid-scoring-fusion.md` + `docs/decisions/adr-026-reranker-provider.md`——据真实 eval 数据 Proposed→Accepted 或如实记录维持
- 修改 `docs/s2v-adapter.md`（Phase 21 Draft→Done + Tasks 0→3；ADR-025 / ADR-026 状态；BDD phase-21 feature 行）

### BDD feature

- 新增 `test/features/phase-21-retrieval-quality.feature`（≥3 scenario：hybrid 融合序确定性 / reranker 确定性重排管道 / eval+smoke hybrid 真实断言）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 21.1 | `core/src/retriever/fusion.rs` 融合函数 + `Retriever::search_hybrid` + `SearchResult.hybrid_score` + proto add-only `hybrid_score=15`/`hybrid=8` | `../tasks/task-21.1-hybrid-scoring.md` |
| 21.2 | `core/src/rerank/{mod,traits,identity,cross_encoder}.rs` `Reranker` trait + `IdentityReranker`（确定性默认）+ `CrossEncoderReranker`（feature-gated）+ `Retriever::with_reranker` | `../tasks/task-21.2-reranker-pipeline.md` |
| 21.3 | eval hybrid/reranked 列 + smoke 升级 + v0.14.0 release docs + ADR-025/026 ratify + phase-21 §6 闭合 + adapter | `../tasks/task-21.3-closeout-v0.14.0.md` |

## 5. 依赖关系

- **task-21.1**（hybrid scoring）dep Phase 19 `search()` + `search_semantic()`（已落地）；首项，提供融合入口。
- **task-21.2**（reranker pipeline）dep Phase 19 `EmbeddingProvider` trait 风格（仿照）+ `Retriever` builder seam 范式；可与 21.1 并行（写路径基本不相交：`fusion.rs` vs `rerank/` 新模块；proto add-only 字段如同 PR 串行化以避免 field 号竞争）。
- **task-21.3**（closeout）dep 21.1 + 21.2 全 Done。
- 外部：ADR-025 / ADR-026（本 phase 新 Proposed）/ ADR-006（recall gate）/ ADR-008（core-library-selection，reranker 新 dep add-only amendment）/ ADR-013（禁伪造）/ ADR-014 第十二次激活 / Phase 19 task-19.2/19.3（`search_semantic` + proto add-only 范式）/ Phase 20 task-20.1（console-api 语义贯通，rerank/hybrid 经同一 console-api 通路可达）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（每条 AC 含 ADR-014 D3 verified by 显式 owner；Draft 阶段未勾选，实施后逐条置 `[x]`）**：

- [ ] **AC1**：hybrid scoring 融合函数（RRF 或加权归一）落地；固定 BM25/vector 分数 → 期望融合序确定性可断言；`retrieval_method = "hybrid"` + add-only `hybrid_score` 填实；单路缺失时降级不 panic — verified by task-21.1 §6 AC1-3 + phase-smoke step 1
- [ ] **AC2**：`Reranker` trait + 确定性 `IdentityReranker`（默认构建 0 模型依赖）落地，确定性重排序管道 CI 可断言；real `CrossEncoderReranker`（feature-gated）真实质量数值经真实 eval 记录或受阻如实 defer（禁伪造，ADR-013）— verified by task-21.2 §6 AC1-3 + phase-smoke step 2
- [ ] **AC3**：eval 报告加 hybrid / reranked 召回列（确定性 wiring CI 可断言，real 数值本地复跑记录）+ smoke 升级（hybrid / rerank opt-in 路径真实断言）+ 既有 step 不退化 — verified by task-21.3 §6 AC1-2 + phase-smoke step 3
- [ ] **AC4**：v0.14.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ ADR-025 / ADR-026 据真实数据 ratify 或如实记录维持 Proposed + phase §6 闭合 — verified by task-21.3 §6 AC3
- [ ] **AC5**：ADR-014 cross-validation gate 全套通过（第十二次激活）— D1 mapping + D2 lint `--touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-20 不溯改 — verified by task-21.3 closeout PR body

**端到端 smoke（C1 集成兜底）**：(1) hybrid 融合序确定性 roundtrip；(2) 确定性 reranker 重排管道；(3) eval + smoke hybrid / rerank opt-in 真实断言全 PASS。

## 7. 阶段级风险

- **R1（中）融合策略选型在确定性数据上不可区分**：RRF 与加权归一在固定合成分数上可能产生相近序，难以纯凭确定性测试选定最优策略。
  - **缓解**：确定性测试只断言**融合函数正确性 + 序的确定性**（不预判哪种策略召回更高）；策略选型经 task-21.3 真实 dogfood eval 对比数值驱动（ADR-025 数据驱动 ratify，仿 ADR-023）。stop-condition：若两策略真实召回不可区分，则按架构简单性择一并诚实记录「recall 不可区分」（仿 ADR-023 D6 范式），不伪造区分度。
- **R2（高）real cross-encoder 模型真实质量需模型 + 真实 eval，CI 不验证**：cross-encoder 模型下载 / 平台构建受阻 → 真实质量数值不可得。
  - **缓解**：确定性 `IdentityReranker` 路径 CI 可验证重排管道 wiring；real `CrossEncoderReranker` 质量数值 🔴 需模型 + 真实 eval 本地复跑记录（ADR-013 不伪造）。stop-condition：real reranker provider 两平台均不可构建 / 模型不可得 → 确定性管道跑通 + 真实质量数值如实 defer（`[SPEC-DEFER:phase-future.reranker-real-quality]`），AC2 记录受阻态，不标 `[x]`，ADR-026 维持 Proposed。
- **R3（低）proto add-only 字段号竞争**：`hybrid_score = 15`（RetrievalResult）/ `hybrid = 8`（SearchRequest）与 reranker 可能新增的字段号需协调。
  - **缓解**：task-21.1 / task-21.2 proto add-only 改动 PR 串行化（adapter §依赖列已注）；字段号按当前空号顺序分配（vector_score=13 / embedding_provider=14 已占 → hybrid_score=15）；proto-freeze 守护 + 22-endpoint conformance 复跑确认 add-only 不破坏。

## 8. Definition of Done

- 3 task spec（21.1-21.3）顶部 `**Status**: Done`
- §6 阶段级 AC1-5 全 `[x]`
- 端到端 smoke 3 step 全 PASS
- **ADR**：ADR-025 / ADR-026 `Proposed → Accepted`（据真实 eval 数据）或据真实证据如实记录维持 Proposed（ADR-013 受阻态）
- **adapter**：§Phase 索引 Phase 21 `Draft → Done` + `Tasks 0 → 3`；§ADR 索引 ADR-025 / ADR-026；§BDD 追加 phase-21 feature 行
- **eval evidence**：`docs/spikes/phase-21-hybrid-recall.md`（或同名）记 hybrid / reranked 真实召回对比（real run / deterministic / 受阻三态如实标）
- **release**：`docs/releases/v0.14.0-{evidence,artifacts}.md` + `RELEASE_NOTES.md` v0.14 段 + README v0.14 段
- **cross-repo follow-up**：hybrid / rerank provenance 经 console-api（Phase 20 已贯通）可达后，通知 Console 重排 explain 的数据通路就绪
