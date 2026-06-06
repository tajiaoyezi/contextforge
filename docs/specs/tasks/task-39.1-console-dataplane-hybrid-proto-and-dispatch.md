# Task `39.1`: `console-dataplane-hybrid-proto-and-dispatch — (A) console_data_plane.proto add-only SearchRequest.hybrid=8（镜像 v1/search.proto:28）+ SearchResultItem.hybrid_score=17（镜像 v1 RetrievalResult.hybrid_score=15，既有字段号 1-7 / 1-16 全冻结，ADR-015 D1 add-only）+ buf generate 重生 Go/Rust 生成代码。(B) core/src/data_plane/search.rs query() 加 hybrid dispatch 分支（let hits = if req.hybrid {..} else if req.semantic {..} else {BM25}），hybrid 分支镜像 server.rs hybrid 路径 + 数据面自身 semantic 分支结构：model-free DeterministicEmbeddingProvider + 0-dep BruteForceVectorBackend + enumerate_chunks + index_chunks_semantic + search_hybrid + 复用 reranker_from_env() opt-in（同 semantic 分支），结果映射加 hybrid_score 填充（镜像 vector_score 条件 :359-363）；默认 hybrid=false → 既有 semantic / BM25 路径字节等价（向后兼容，ADR-004）；0 新 dep / 0 schema migration / 0 默认构建改动`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 39 (console-api-retrieval-signal-forward)
**Dependencies**: 既有 `proto/contextforge/v1/search.proto`（`SearchRequest.hybrid=8` `:28` + `RetrievalResult.hybrid_score=15` `:51`——console_data_plane 镜像之达成 parity，task-21.1 Done）/ 既有 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`（`SearchRequest` `:151-162`——`query=1`..`semantic=7`，本 task add-only `hybrid=8`；`SearchResultItem` `:185-204`——`..vector_score=16`，本 task add-only `hybrid_score=17`，ADR-015 D1 字段冻结契约）/ 既有 `core/src/server.rs` hybrid 路径（`:328-376`——`req.hybrid` `:334` / env-factory backend `:342-343` / reranker opt-in `:351-355` / `search_hybrid` `:363` / `pr.hybrid_score = r.score` `:369`——**hybrid dispatch 范本**，task-21.1 Done）/ 既有 `core/src/data_plane/search.rs` `query()`（`:241`——semantic 分支 `let hits = if req.semantic {..} else {BM25}` `:282-315`，hardcoded `DeterministicEmbeddingProvider::default()` `:283` + `BruteForceVectorBackend::new()` `:284` + reranker opt-in `:291-295`（task-38.2）+ `search_semantic` `:302-304`；结果映射 `:339-368`——`vector_score` 条件 `:359-363`（task-32.3 范本））/ 既有 `core/src/retriever/mod.rs` `search_hybrid`（RRF 融合，task-21.1 Done）+ `with_reranker`（`:630` opt-in seam，task-21.2 Done）+ `reranker_from_env`（task-38.2 Done）/ 既有 `buf` 工具链（proto 重生）/ ADR-044（console-api-retrieval-signal-forward；本 task = 其 D1 落点）/ ADR-025（hybrid-scoring-fusion；本 task 经 console_data_plane proto + 数据面 dispatch 兑现其 `[SPEC-DEFER:phase-future.console-api-hybrid-forward]` 的 core 半，add-only Phase-39 Amendment 落点 @ task-39.3 closeout）/ ADR-015（console-data-plane-proto-contract；本 task add-only 新字段守其 D1 字段冻结 + add-only 演进规则）/ ADR-004（local-first；默认 `hybrid=false` 字节等价 + console 数据面 hybrid 分支 model-free `DeterministicEmbeddingProvider` + 0-dep `BruteForceVectorBackend` 0 网络）/ ADR-008（dep add-only，Phase 39 = 0 新 dep）/ ADR-013（禁伪造红线——`hybrid_score` 端到端携带、非由 score 推断；默认 hybrid=false 字节等价不伪装；console 数据面 hybrid 分支用 hardcoded backend 据实记 console-data-plane-vector-backend-factory 延后）/ ADR-012（main-agent-governance-autonomy）/ ADR-014 D1-D5（第三十次激活）

## 1. Background

Phase 21（task-21.1，ADR-025）已在 core 层落地 hybrid RRF 融合：`core/src/server.rs` 的 hybrid 路径（`:328-376`）经 `req.hybrid`（`:334`）触发——build on-demand 索引 + `search_hybrid(&req.query, top_k)`（`:363`，RRF-fuses BM25 与 vector 两路）→ 命中 `retrieval_method="hybrid"` + `pr.hybrid_score = r.score`（`:369`），`proto/contextforge/v1/search.proto` 的 `SearchRequest.hybrid=8`（`:28`）+ `RetrievalResult.hybrid_score=15`（`:51`）承载之。Phase 38（task-38.2，ADR-043 D3）把 reranker 在生产数据面 opt-in 接线——含 `core/src/data_plane/search.rs` semantic 分支（`reranker_from_env()` → `with_reranker`，`:291-295`）。但**对外 console 数据面通路缺 hybrid 三环**：

- **C1 console_data_plane.proto 无 `hybrid` / `hybrid_score` 字段**：`proto/contextforge/console_data_plane/v1/console_data_plane.proto` 的 `SearchRequest`（`:151-162`）只到 `semantic = 7`、**无 `hybrid` 字段**；`SearchResultItem`（`:185-204`）只到 `vector_score = 16`、**无 `hybrid_score` 字段**。这与 `v1/search.proto`（`SearchRequest.hybrid=8` + `RetrievalResult.hybrid_score=15`）不同——console_data_plane 是 ADR-015 确立的独立检索契约，hybrid parity 是已识别 add-only 演进点（task-20.1 已为 console_data_plane 加 `semantic=7`、task-32.3 已加 `vector_score=16`，hybrid 是同范式下一字段）。
- **C2 console 数据面 `query()` 无 hybrid dispatch 分支**：`core/src/data_plane/search.rs` 的 `query()`（`:241`）只有 `let hits = if req.semantic {..} else {BM25}`（`:282-315`）——**无 hybrid 分支**，即便对外请求 hybrid 也无处分派。`server.rs` 有完整 hybrid 路径（`:328-376`）作范本，但 console 数据面 `query()` 从未镜像它。
- **C3 `hybrid_score` provenance 字段缺 v1 parity**：console_data_plane `SearchResultItem` 有 `vector_score = 16`（task-32.3，semantic 命中的余弦相似度 provenance）却无 `hybrid_score`，对外 hybrid 命中无独立融合分 provenance（与 `v1 RetrievalResult.hybrid_score=15` 不 parity）。

本 task 关闭 console 数据面 hybrid 的 proto + dispatch 半（Go console-api 转发半属 task-39.2），是 ADR-044 D1：

- **B1 proto add-only（既有字段号冻结，ADR-015 D1）**：`SearchRequest` 加 `bool hybrid = 8`（紧随 `semantic = 7`，镜像 `v1/search.proto:28`）+ `SearchResultItem` 加 `float hybrid_score = 17`（紧随 `vector_score = 16`，镜像 `v1 RetrievalResult.hybrid_score=15`）；既有字段号 1-7 / 1-16 **全冻结不动**（既有 client 兼容，不破 wire 契约）。
- **B2 `buf generate` 重生 生成代码**：proto 改动后用既有 `buf` 工具链重生 Go + Rust 生成代码，`SearchRequest.Hybrid` / `SearchResultItem.HybridScore`（Go）+ `req.hybrid` / `pr.hybrid_score`（Rust）随之可用。
- **B3 数据面 hybrid dispatch（镜像 server.rs hybrid 路径 + 数据面 semantic 分支结构）**：`query()` 的 `let hits = if req.semantic {..} else {BM25}` 改为 `if req.hybrid {..} else if req.semantic {..} else {BM25}`；hybrid 分支用 model-free `DeterministicEmbeddingProvider::default()` + 0-dep `BruteForceVectorBackend::new()`（与数据面 semantic 分支 `:283-284` 一致）+ `enumerate_chunks` + `index_chunks_semantic` + `search_hybrid(&req.query, top_k)` + 复用 `reranker_from_env()` opt-in（同 semantic 分支 `:291-295`）；命中 `retrieval_method` 由 `search_hybrid` 标 `"hybrid"`。
- **B4 `hybrid_score` 填充（镜像 vector_score 条件，ADR-013 不伪造）**：结果映射（`:339-368`）加 `hybrid_score: if h.retrieval_method == "hybrid" { h.score } else { 0.0 }`（镜像 `vector_score` `:359-363`）——融合分端到端携带、非由 score 在下游推断。
- **B5 默认 `hybrid=false` 字节等价（向后兼容，ADR-004）**：`req.hybrid` 默认 `false` ⇒ 走既有 `if req.semantic` / BM25 分支、`hybrid_score=0.0` ⇒ 检索结果与当前字节等价；既有 console_data_plane client（不设 `hybrid`）行为不变。
- **B6 console 数据面 hybrid 分支用 hardcoded backend（据实记延后，ADR-013）**：hybrid 分支用 hardcoded `BruteForceVectorBackend`（镜像数据面 semantic 分支 `:283-284`），**非** `server.rs` 的 env-factory backend（`select_vector_backend(resolve_vector_backend())` `:342-343`）——保 console 数据面内部一致性（task-32.1 的 env-factory 只接入 `server.rs`、未接 console 数据面，是既有 asymmetry）；console 数据面接 env-factory backend 续延后 `[SPEC-DEFER:phase-future.console-data-plane-vector-backend-factory]`。

本 task 为 code-local 🟢 可验证（数据面 hybrid dispatch 单测 + proto add-only / `hybrid_score` 填充单测）；0 新 dep（复用 `search_hybrid` / `reranker_from_env` / `BruteForceVectorBackend` / `DeterministicEmbeddingProvider` + `buf` 工具链）；0 schema migration / 0 默认构建改动。

## 2. Goal

(1) **B1 proto add-only**：`proto/contextforge/console_data_plane/v1/console_data_plane.proto` 的 `SearchRequest` add-only `bool hybrid = 8`（紧随 `semantic = 7` `:161`，注释镜像 `v1/search.proto:19/28`：默认 `false` → semantic-only / BM25 向后兼容）+ `SearchResultItem` add-only `float hybrid_score = 17`（紧随 `vector_score = 16` `:203`，注释镜像 `task-32.3` `vector_score` + `v1 RetrievalResult.hybrid_score=15`：RRF 融合分、0 表示 semantic/BM25-only 命中、parity with v1）；既有字段号 1-7 / 1-16 全冻结不动。(2) **B2 `buf generate`**：重生 Go + Rust 生成代码。(3) **B3 数据面 hybrid dispatch**：`core/src/data_plane/search.rs` `query()` 的 `let hits = if req.semantic {..} else {BM25}`（`:282-315`）改为 `if req.hybrid {..} else if req.semantic {..} else {BM25}`；hybrid 分支镜像 `server.rs` hybrid 路径（`:328-376`）+ 数据面 semantic 分支结构——`DeterministicEmbeddingProvider::default()` + `BruteForceVectorBackend::new()` + `enumerate_chunks` + `index_chunks_semantic(backend.as_ref(), &items)` + `search_hybrid(&req.query, top_k)` + 复用 `reranker_from_env()` opt-in（同 `:291-295`）。(4) **B4 `hybrid_score` 填充**：结果映射（`:339-368`）的 `SearchResultItem` 构造加 `hybrid_score: if h.retrieval_method == "hybrid" { h.score } else { 0.0 }`（镜像 `vector_score` `:359-363`）。(5) **B5 默认 `hybrid=false` 字节等价**：`req.hybrid` 默认 `false` ⇒ 走既有 semantic / BM25 分支 + `hybrid_score=0.0` ⇒ 检索结果字节等价无变化（ADR-004 向后兼容）。(6) **B6 console 数据面 hybrid 分支 hardcoded backend**：hybrid 分支用 hardcoded `BruteForceVectorBackend`（镜像数据面 semantic 分支），env-factory backend 续延后 `[SPEC-DEFER:phase-future.console-data-plane-vector-backend-factory]`。(7) **0 dep**：复用 `search_hybrid` / `reranker_from_env` / `BruteForceVectorBackend` / `DeterministicEmbeddingProvider` + `buf`（0 新 Rust dep，ADR-008）。

pass bar：console_data_plane proto `SearchRequest.hybrid=8` + `SearchResultItem.hybrid_score=17` add-only（既有字段号 1-7 / 1-16 冻结，`buf generate` 后 Go / Rust 生成代码 含新字段）（🟢）；数据面 hybrid dispatch 经单测验证（`req.hybrid=true` ⇒ `query()` 走 `search_hybrid`、命中 `retrieval_method="hybrid"` + `hybrid_score` 非零；`req.hybrid=false` + `req.semantic=true` ⇒ 走 semantic 分支字节等价；两者皆 false ⇒ BM25 字节等价）（🟢）；`hybrid_score` 填充经单测验证（hybrid 命中 `hybrid_score=score`、非 hybrid 命中 `hybrid_score=0.0`，非伪造，ADR-013）（🟢）；默认 `hybrid=false` 字节等价（ADR-004）+ 既有契约（console_data_plane proto 既有字段 / `query()` semantic / BM25 分支 / `server.rs` hybrid 路径）不变；0 新 dep；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`——`SearchRequest`（`:151-162`）add-only `bool hybrid = 8`（紧随 `semantic = 7`，注 `task-39.1 add-only: hybrid RRF fusion (BM25+vector). 默认 false → semantic-only / BM25 向后兼容; parity with v1/search.proto:28`）；`SearchResultItem`（`:185-204`）add-only `float hybrid_score = 17`（紧随 `vector_score = 16`，注 `task-39.1 add-only: RRF 融合分 (0 表示 semantic/BM25-only 命中); parity with v1 RetrievalResult.hybrid_score=15`）；既有字段号 1-7 / 1-16 全冻结不动。
- `buf generate`——重生 Go + Rust 生成代码（既有 `buf` 工具链；提交重生后的 generated 文件）。
- 改 `core/src/data_plane/search.rs`——`query()` 的 `let hits = if req.semantic {..} else {BM25}`（`:282-315`）改为 `if req.hybrid {..} else if req.semantic {..} else {BM25}`；hybrid 分支（镜像 `server.rs:334-376` + 数据面 semantic 分支 `:282-304`）：`let embedder = Arc::new(DeterministicEmbeddingProvider::default());` + `let backend = Arc::new(BruteForceVectorBackend::new());` + `let mut wired = retriever.with_embedder(embedder).with_vector_searcher(backend.clone());` + `if let Some(rr) = crate::rerank::reranker_from_env().map_err(|e| Status::internal(format!("reranker: {e}")))? { wired = wired.with_reranker(rr); }` + `enumerate_chunks` + `index_chunks_semantic(backend.as_ref(), &items)` + `wired.search_hybrid(&req.query, top_k)`。
- 改 `core/src/data_plane/search.rs` 结果映射（`:339-368`）——`SearchResultItem` 构造加 `hybrid_score: if h.retrieval_method == "hybrid" { h.score } else { 0.0 }`（紧随 `vector_score` `:359-363`，镜像其条件范式）。
- 同源测试：Rust 同 crate test（`core/src/data_plane/search.rs` 模块内 `#[cfg(test)]` 或 `core/tests/`）——hybrid dispatch（`req.hybrid=true` → `search_hybrid` + `retrieval_method="hybrid"` + `hybrid_score` 非零；`hybrid=false` → semantic / BM25 字节等价）+ proto 字段存在 / 字段号 + `hybrid_score` 填充条件。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- Go console-api `?hybrid` 转发（`contractv1.SearchRequest.Hybrid` + `handleSearch` `?hybrid` OR-merge + `grpcclient.Search` 转发 + `protoToSearchResult` 映射 `HybridScore`）——属 task-39.2（本 phase 同批）；本 task 的 proto add-only 字段 + 数据面 dispatch 是 task-39.2 转发的消费目标（task-39.2 dep 本 task 的 proto 字段 + `buf generate`）。
- console 数据面 hybrid 分支采用 `server.rs` 的 env-factory backend 选型（`select_vector_backend(resolve_vector_backend())`，镜像 `:342-343`）[SPEC-DEFER:phase-future.console-data-plane-vector-backend-factory]——本 task hybrid 分支镜像 console 数据面自身 semantic 分支的 hardcoded `BruteForceVectorBackend`（`:283-284`，保数据面内部一致性）；task-32.1 的 env-factory 只接入 `server.rs`、未接 console 数据面是既有 asymmetry，env-factory backend 接入 console 数据面续延后边界、不在本 task 扩面。
- rerank `reason` provenance 在对外 REST 响应的端到端可见性断言——属 task-39.2 / task-39.3（smoke + Go 测试）；本 task hybrid 分支复用 `reranker_from_env()` opt-in（与 semantic 分支一致），reranker 仍 env 驱动（ADR-043 D3，不加 `?rerank` 参数 [SPEC-DEFER:phase-future.console-api-rerank-forward]，据 ADR-044 D3 重界定为 provenance 可见性）。
- 大语料 hybrid 对外 REST 召回质量基准（NDCG / MRR 大基准）[SPEC-DEFER:phase-future.vector-large-corpus-perf]——本 task 为 hybrid dispatch wiring（`search_hybrid` 正确分派 + `hybrid_score` 携带），hybrid 融合质量 ADR-025 已 ratify、不重测。
- 真实 release tag / run-id / digest（v0.32.0）[SPEC-OWNER:task-39.3-closeout]（ADR-012 用户授权后回填）。

## 4. Actors

- 主 agent（ADR-012 自治）
- `console_data_plane.proto SearchRequest`（`:151-162`，本 task add-only `bool hybrid = 8`，镜像 `v1/search.proto:28`）
- `console_data_plane.proto SearchResultItem`（`:185-204`，本 task add-only `float hybrid_score = 17`，镜像 `v1 RetrievalResult.hybrid_score=15`）
- `buf`（既有 proto 工具链，本 task 重生 Go + Rust 生成代码）
- `core/src/data_plane/search.rs` `query()`（`:241`，本 task 加 hybrid dispatch 分支 + `hybrid_score` 填充）
- `search_hybrid`（`core/src/retriever/mod.rs`，既有 RRF 融合，task-21.1 交付，本 task 数据面首次消费）
- `reranker_from_env` / `with_reranker`（task-38.2 / `core/src/retriever/mod.rs:630`，既有 opt-in seam，本 task hybrid 分支复用，不改其签名）
- `DeterministicEmbeddingProvider` / `BruteForceVectorBackend`（model-free / 0-dep，本 task hybrid 分支复用，与数据面 semantic 分支一致）
- 对外 console-api 调用方（经 task-39.2 的 `POST /v1/search` `?hybrid=true` 抵达本 task 的数据面 hybrid 分支；本 task 不含 Go 转发）

## 5. Behavior Contract

### 5.1 Required Reading

- `proto/contextforge/console_data_plane/v1/console_data_plane.proto:151-162`（`SearchRequest`——`query=1`..`config_snapshot=6`..`semantic=7`，`hybrid=8` add-only 紧随）+ `:185-204`（`SearchResultItem`——`..score=10`..`vector_score=16`，`hybrid_score=17` add-only 紧随）
- `proto/contextforge/v1/search.proto:18-29`（**范本**：`SearchRequest` `semantic=7` + `hybrid=8` `:28` 注释风格）+ `:31-52`（`RetrievalResult` `vector_score=13` + `hybrid_score=15` `:51` 注释风格）
- `core/src/server.rs:328-376`（**hybrid dispatch 范本**：`if req.hybrid` `:334` / env-factory backend `:342-343`（本 task **不**镜像此项，用 hardcoded backend，见 §5.2 B6）/ `with_embedder().with_vector_searcher()` `:345-347` / reranker opt-in `:351-355` / `enumerate_chunks` `:357` / `index_chunks_semantic` `:360` / `search_hybrid` `:363` / `pr.hybrid_score = r.score` `:369`）
- `core/src/data_plane/search.rs:241`（`query()` 入口）+ `:282-315`（semantic 分支 `let hits = if req.semantic {..} else {BM25}`——hardcoded `DeterministicEmbeddingProvider::default()` `:283` + `BruteForceVectorBackend::new()` `:284` + `with_embedder().with_vector_searcher()` `:285-287` + reranker opt-in `:291-295` + `enumerate_chunks` `:296-298` + `index_chunks_semantic` `:299-301` + `search_semantic` `:302-304`；BM25 else `:305-315`）+ `:339-368`（结果映射 `SearchResultItem` 构造——`retrieval_method: h.retrieval_method.clone()` `:355` + `vector_score: if h.retrieval_method == "vector" { h.score } else { 0.0 }` `:359-363`——`hybrid_score` 镜像之）
- `core/src/retriever/mod.rs`（`search_hybrid`——RRF 融合 BM25 + vector 两路，命中标 `retrieval_method="hybrid"`，task-21.1 Done）+ `:630`（`with_reranker(mut self, reranker: Arc<dyn Reranker>) -> Self` opt-in seam，task-21.2 Done）+ `reranker_from_env`（读 `CONTEXTFORGE_RERANKER_PROVIDER`，task-38.2 Done）
- `docs/decisions/adr-044-console-api-retrieval-signal-forward.md §D1`（本 task 即其原文实现）+ `docs/decisions/adr-025-hybrid-scoring-fusion.md`（hybrid 母 ADR，本 task = 其 `console-api-hybrid-forward` core 半兑现，Phase-39 add-only Amendment 落点 @ task-39.3）+ `docs/decisions/adr-015-console-data-plane-proto-contract.md`（proto add-only 字段守其 D1 字段冻结规则）/ ADR-004（默认 hybrid=false 字节等价 / 0 网络）/ ADR-008（0 新 dep）/ ADR-013（`hybrid_score` 端到端携带非推断 / console 数据面 hardcoded backend 据实记延后）

### 5.2 关键设计 — console_data_plane proto add-only + 数据面 hybrid dispatch（既有字段号冻结 / 镜像 server.rs hybrid 路径 + 数据面 semantic 分支结构 / 默认 hybrid=false 字节等价 / hardcoded backend 保内部一致）

- **B1 proto add-only（镜像 v1/search.proto，既有字段号冻结，ADR-015 D1）**：`SearchRequest` 加 `bool hybrid = 8`（紧随 `semantic = 7`）；`SearchResultItem` 加 `float hybrid_score = 17`（紧随 `vector_score = 16`）。既有字段号 1-7 / 1-16 **全冻结不动**——新字段用下一个未用编号（8 / 17），既有 wire 契约不破、既有 client 兼容。注释镜像 `v1/search.proto` + `task-32.3 vector_score` 风格（add-only / 默认值 / parity 引用）。
- **B2 `buf generate` 重生 生成代码**：proto 改动后用既有 `buf` 工具链重生 Go + Rust generated code（提交重生文件）；`SearchRequest.Hybrid`（Go）/ `req.hybrid`（Rust）+ `SearchResultItem.HybridScore`（Go）/ `pr.hybrid_score`（Rust）随之可用。**仅 add-only 字段重生**——既有 generated 字段不变（diff 只增 hybrid / hybrid_score 相关行）。
- **B3 数据面 hybrid dispatch（镜像 server.rs hybrid 路径 + 数据面 semantic 分支结构）**：`query()` 的 `let hits = if req.semantic {..} else {BM25}` 改为三分支 `if req.hybrid {..} else if req.semantic {..} else {BM25}`。hybrid 分支（新）镜像 `server.rs:334-376` 的 build-index + `search_hybrid` + reranker opt-in，并在装配上对齐数据面 semantic 分支（`:282-304`）：`DeterministicEmbeddingProvider::default()` + `BruteForceVectorBackend::new()` + `with_embedder().with_vector_searcher()` + `reranker_from_env()` opt-in（`if let Some(rr) = ... { wired = wired.with_reranker(rr) }`）+ `enumerate_chunks` + `index_chunks_semantic(backend.as_ref(), &items)` + `search_hybrid(&req.query, top_k)`。`search_hybrid` 命中标 `retrieval_method="hybrid"`。
- **B4 `hybrid_score` 填充（镜像 vector_score 条件，ADR-013 不伪造）**：结果映射（`:339-368`）的 `SearchResultItem` 构造加 `hybrid_score: if h.retrieval_method == "hybrid" { h.score } else { 0.0 }`（紧随 `vector_score` `:359-363`）——hybrid 命中携带其融合 score（`search_hybrid` 的 RRF 融合分，与 `server.rs:369` `pr.hybrid_score = r.score` 一致），非 hybrid 命中（vector / bm25）为 `0.0`（不伪造融合分）。
- **B5 默认 `hybrid=false` 字节等价（ADR-004 向后兼容）**：`req.hybrid` 默认 `false`（proto bool 默认）⇒ `if req.hybrid` 不进 ⇒ 走既有 `else if req.semantic` / BM25 分支（与当前 `if req.semantic {..} else {BM25}` 等价）+ `hybrid_score` 为 `0.0`（非 hybrid 命中）⇒ 检索结果字节等价；既有 console_data_plane client（不设 `hybrid`）行为不变。
- **B6 console 数据面 hybrid 分支 hardcoded backend（据实记延后，ADR-013）**：hybrid 分支用 hardcoded `BruteForceVectorBackend::new()`（镜像数据面 semantic 分支 `:284`），**非** `server.rs:342-343` 的 env-factory backend（`select_vector_backend(resolve_vector_backend())`）——task-32.1 的 env-factory 只接入 `server.rs`、console 数据面 semantic 分支至今用 hardcoded backend（既有 asymmetry）；hybrid 分支镜像数据面 semantic 分支保内部一致性，console 数据面接 env-factory backend 续延后 `[SPEC-DEFER:phase-future.console-data-plane-vector-backend-factory]`（既有 asymmetry 的统一扩面，非本 task scope）。

### 5.3 不变量

- 默认行为不变（ADR-004）：`req.hybrid` 默认 `false` ⇒ 走既有 semantic / BM25 分支 + `hybrid_score=0.0` ⇒ 检索结果字节等价于当前；既有 console_data_plane client（不设 `hybrid`）+ 既有 `if req.semantic` / BM25 路径不变。
- 既有契约不变（ADR-015 D1 + ADR-004）：console_data_plane proto 既有字段号 1-7（`SearchRequest`）/ 1-16（`SearchResultItem`）全冻结不动（add-only `hybrid=8` / `hybrid_score=17`，既有 wire 契约不破、既有 client 兼容）；`query()` semantic / BM25 分支 + 结果映射既有字段不变（add-only `hybrid_score` 填充）；`server.rs` hybrid 路径 / `search_hybrid` / `reranker_from_env` / `with_reranker` 签名不改。
- `hybrid_score` 据实（ADR-013）：hybrid 命中 `hybrid_score = h.score`（`search_hybrid` RRF 融合分，端到端携带）；非 hybrid 命中（vector / bm25）`hybrid_score = 0.0`（不伪造融合分，镜像 `vector_score` 范式）。
- reranker 仍 env 驱动（ADR-043 D3）：hybrid 分支复用 `reranker_from_env()` opt-in（与 semantic 分支一致）——`CONTEXTFORGE_RERANKER_PROVIDER` unset ⇒ 无 rerank 字节等价；本 task 不加 `?rerank` 参数（per-request 转发据 ADR-044 D3 重界定）。
- 0 新代码依赖（ADR-008）：复用 `search_hybrid`（task-21.1）/ `reranker_from_env`（task-38.2）/ `with_reranker`（task-21.2）/ `BruteForceVectorBackend` / `DeterministicEmbeddingProvider` + `buf` 工具链；无新 Rust direct dep（`Cargo.lock` 默认构建段不变）。
- console 数据面内部一致（ADR-013 据实）：hybrid 分支用 hardcoded `BruteForceVectorBackend`（镜像数据面 semantic 分支），env-factory backend 据实记延后 `[SPEC-DEFER:phase-future.console-data-plane-vector-backend-factory]`，不夸大为「console 数据面已接 env-factory backend」。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [ ] **AC1**（console_data_plane proto add-only + 数据面 hybrid dispatch + 默认字节等价 🟢）: `SearchRequest.hybrid=8` + `SearchResultItem.hybrid_score=17` add-only（既有字段号 1-7 / 1-16 冻结，`buf generate` 后 Go / Rust 生成代码 含新字段）；`core/src/data_plane/search.rs` `query()` 三分支 dispatch（`if req.hybrid {..} else if req.semantic {..} else {BM25}`）；`req.hybrid=true` ⇒ 走 `search_hybrid`、命中 `retrieval_method="hybrid"` + `hybrid_score` 非零（复用 `reranker_from_env` opt-in）；`req.hybrid=false` + `req.semantic=true` ⇒ semantic 分支字节等价；两者皆 false ⇒ BM25 字节等价 — verified by **TEST-39.1.1**（数据面 hybrid dispatch）
- [ ] **AC2**（`hybrid_score` 填充据实 + proto 字段号 🟢）: `SearchResultItem.hybrid_score` 字段号 = 17、`SearchRequest.hybrid` 字段号 = 8（既有字段号不动）；结果映射 `hybrid_score: if h.retrieval_method == "hybrid" { h.score } else { 0.0 }`——hybrid 命中 `hybrid_score=score`、非 hybrid 命中 `hybrid_score=0.0`（非伪造，ADR-013）；console 数据面 hybrid 分支用 hardcoded `BruteForceVectorBackend`（据实记延后）；0 新 dep — verified by **TEST-39.1.2**（proto add-only + `hybrid_score` 填充）
- [ ] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-39.1.3**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-39.1.1 | 数据面 hybrid dispatch：`query()` 三分支（`if req.hybrid {..} else if req.semantic {..} else {BM25}`）——`req.hybrid=true` ⇒ 走 `search_hybrid`、命中 `retrieval_method="hybrid"` + `hybrid_score` 非零（复用 `reranker_from_env` opt-in，同 semantic 分支）；`req.hybrid=false` + `req.semantic=true` ⇒ semantic 分支命中 `retrieval_method="vector"` 字节等价；两者皆 false ⇒ BM25 字节等价（向后兼容，ADR-004） | `core/src/data_plane/search.rs` 同 crate test（或 `core/tests/`） | Draft |
| TEST-39.1.2 | proto add-only + `hybrid_score` 填充：`buf generate` 后 `SearchRequest.hybrid` 字段号 = 8 / `SearchResultItem.hybrid_score` 字段号 = 17（既有字段号 1-7 / 1-16 不动）；结果映射 hybrid 命中 `hybrid_score = h.score`、非 hybrid 命中（vector / bm25）`hybrid_score = 0.0`（镜像 `vector_score` 条件，非伪造，ADR-013）；console 数据面 hybrid 分支用 hardcoded `BruteForceVectorBackend`（据实记 `[SPEC-DEFER:phase-future.console-data-plane-vector-backend-factory]`） | `core/src/data_plane/search.rs` 同 crate test + proto 字段断言 | Draft |
| TEST-39.1.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Draft |

## 8. Risks

- **R1（高）数据面 hybrid 分支破默认无 hybrid 字节等价（向后兼容）**：若 `query()` 改 dispatch 时影响既有 semantic / BM25 分支（如误改条件顺序），会改变默认（`hybrid=false`）检索结果。
  - **缓解**：三分支 `if req.hybrid {..} else if req.semantic {..} else {BM25}`——`hybrid` 在最前、`hybrid=false` 时完全走既有 `else if req.semantic` / BM25 分支（与当前 `if req.semantic {..} else {BM25}` 等价）；TEST-39.1.1 含「`req.hybrid=false` + `req.semantic=true` ⇒ semantic 字节等价 / 两者皆 false ⇒ BM25 字节等价」断言。stop-condition：默认字节等价单测退化则 AC1 不标 `[x]`。
- **R2（中）proto 既有字段号被误动（破 wire 契约）**：若加 `hybrid=8` / `hybrid_score=17` 时误改既有字段号，破既有 client 兼容。
  - **缓解**：`hybrid=8` 紧随 `semantic=7`（下一未用编号）、`hybrid_score=17` 紧随 `vector_score=16`；既有字段号 1-7 / 1-16 全冻结不动；TEST-39.1.2 含「既有字段号不动 + 新字段号 = 8 / 17」断言。stop-condition：既有字段号变动则 AC1/AC2 不标 `[x]`。
- **R3（中）`hybrid_score` 伪造（非真实融合分）**：若给非 hybrid 命中也填 `hybrid_score` 或把 `vector_score` / `score` 当融合分，违 ADR-013。
  - **缓解**：`hybrid_score: if h.retrieval_method == "hybrid" { h.score } else { 0.0 }`——仅 hybrid 命中填融合分（`search_hybrid` 的 RRF score），非 hybrid 命中为 `0.0`；镜像 `vector_score` 条件范式；TEST-39.1.2 含「hybrid 命中 `=score` / 非 hybrid `=0.0`」断言。
- **R4（中）`buf generate` 重生引入意外 diff**：proto 改动后 `buf generate` 可能因工具版本 / 配置差异重生超出 hybrid / hybrid_score 的 diff。
  - **缓解**：仅 add-only 两字段、既有字段不动 ⇒ 重生 diff 应只增 hybrid / hybrid_score 相关行；提交前核对 generated diff 仅含新字段（既有 generated 字段不变）；若 buf 版本漂移致大 diff，先对齐工具版本再重生。stop-condition：generated diff 含非 hybrid 既有字段变动则查工具版本。
- **R5（中）console 数据面 hybrid 分支被误接 env-factory backend（与 semantic 分支不一致）**：若 hybrid 分支用 `server.rs` 的 `select_vector_backend(resolve_vector_backend())` 而 semantic 分支仍 hardcoded，引入数据面内部 backend 选型不一致。
  - **缓解**：hybrid 分支镜像数据面 semantic 分支用 hardcoded `BruteForceVectorBackend`（`:284`）；env-factory backend 接入 console 数据面据实记延后 `[SPEC-DEFER:phase-future.console-data-plane-vector-backend-factory]`（既有 asymmetry 的统一扩面，非本 task）；TEST-39.1.2 据实记 hardcoded backend。
- **R6（低）测试改进程全局 env（reranker_from_env）致并行串扰**：hybrid 分支复用 `reranker_from_env()` 读 `CONTEXTFORGE_RERANKER_PROVIDER`，测试设 env 可能与并行测试串扰。
  - **缓解**：测试默认不设 `CONTEXTFORGE_RERANKER_PROVIDER`（验 hybrid dispatch 本身，reranker unset ⇒ 无 rerank）；如需验 hybrid + reranker 串测，串行化 env 段 / 断言后还原（同既有 task-38.2 `reranker_env_wiring` 惯例）。

## 9. Verification Plan

```bash
# 1. proto add-only + buf generate（重生 Go + Rust 生成代码；核对 diff 仅含 hybrid / hybrid_score）
buf generate
git diff --stat   # 期望仅 console_data_plane generated + proto 文件，diff 仅增 hybrid / hybrid_score

# 2. AC1 — 数据面 hybrid dispatch（req.hybrid=true → search_hybrid + retrieval_method="hybrid" + hybrid_score；hybrid=false → semantic / BM25 字节等价）
cargo test -p contextforge-core data_plane

# 3. AC2 — hybrid_score 填充 + proto 字段号（hybrid 命中 =score / 非 hybrid =0.0；字段号 8 / 17）
cargo test -p contextforge-core

# 4. 不退化（全量 Rust + 默认 build 无 hybrid 字节等价确认）
cargo test --workspace
cargo clippy --workspace -- -D warnings

# 5. AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.console-dataplane-hybrid-dispatch-defer-note]：本 task 仅交付 (A) console_data_plane proto add-only `SearchRequest.hybrid=8` + `SearchResultItem.hybrid_score=17`（既有字段号冻结，ADR-015 D1）+ `buf generate` 与 (B) `core/src/data_plane/search.rs` `query()` hybrid dispatch 分支（镜像 `server.rs` hybrid 路径 + 数据面 semantic 分支结构，`search_hybrid` + `retrieval_method="hybrid"` + `hybrid_score` 填充 + 复用 `reranker_from_env` opt-in）+ 默认 `hybrid=false` 字节等价（🟢 可单测）；Go console-api `?hybrid` 转发（`contractv1` + `handleSearch` + `grpcclient`）属 task-39.2（消费本 task 的 proto 字段 + `buf generate`）；console 数据面 hybrid 分支接 env-factory backend [SPEC-DEFER:phase-future.console-data-plane-vector-backend-factory]、rerank `?rerank=true` per-request 转发 [SPEC-DEFER:phase-future.console-api-rerank-forward]（据 ADR-044 D3 重界定为 provenance 可见性、reranker 保持 env 驱动）、大语料 hybrid 召回质量基准 [SPEC-DEFER:phase-future.vector-large-corpus-perf]、Console UI hybrid explain 面板 [SPEC-OWNER:phase-future.console-semantic-explain] 均不在本 task 范围。`hybrid_score` 端到端携带、非由 score 推断（ADR-013 不伪造）；真实 release tag / run-id / digest（v0.32.0）[SPEC-OWNER:task-39.3-closeout] 实施授权后回填。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Draft（实施 + §9 真实验证后置 Done，逐条粘 PASS 摘要：`buf generate` diff 仅含 hybrid / hybrid_score / `cargo test -p contextforge-core data_plane` TEST-39.1.1 / `cargo test -p contextforge-core` TEST-39.1.2 / `cargo test --workspace` + clippy / `bash scripts/spec_drift_lint.sh --touched origin/master` 0 命中；未跑不勾 AC）

- **§9 Verification 实证**（实施后回填）：本机真实跑 §9 全部命令、逐条粘 PASS 摘要。
- **实际改动文件**（实施后回填）：`proto/contextforge/console_data_plane/v1/console_data_plane.proto`（add-only `hybrid=8` + `hybrid_score=17`）/ `buf generate` 重生的 Go + Rust generated 文件 / `core/src/data_plane/search.rs`（`query()` hybrid dispatch 分支 + `hybrid_score` 填充）/ Rust 同 crate test（TEST-39.1.1 / TEST-39.1.2）。
- **0 新 dep / 默认行为不变**：`req.hybrid` 默认 `false` = 走既有 semantic / BM25 分支 + `hybrid_score=0.0` = 检索结果字节等价（ADR-004 向后兼容）/ 既有契约不变（proto 既有字段号 1-7 / 1-16 冻结，ADR-015 D1）/ reranker 仍 env 驱动（hybrid 分支复用 `reranker_from_env`，ADR-043 D3）/ `hybrid_score` 端到端携带非推断（ADR-013）/ console 数据面 hybrid 分支 hardcoded backend 据实记延后（ADR-013）。
- **ADR**：本 task = ADR-044 §D1（console_data_plane proto add-only + 数据面 hybrid dispatch）落点；ADR-025（hybrid-scoring-fusion）Phase 39 add-only Amendment（`console-api-hybrid-forward` core 半兑现）落点在 task-39.3 closeout（非本 task body）。
- **复用既有范式**（0 backend 算法改动）：`search_hybrid`（task-21.1）+ `reranker_from_env`（task-38.2）+ `vector_score` 填充范式（task-32.3）+ proto add-only 演进（ADR-015 D1）。
