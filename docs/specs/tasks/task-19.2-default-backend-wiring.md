# Task `19.2`: `default-backend-wiring — core/src/retriever/mod.rs 据 ADR-023 D1/D2 把选定默认 vector backend 接 Retriever::with_vector_searcher 生产热路径 + index/query 过 EmbeddingProvider + BM25 不退化`

**Status**: Pending

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 19 (vector-retrieval-integration)
**Dependencies**: task-19.1（`EmbeddingProvider` trait + `DeterministicEmbeddingProvider` 缺省 provider，`core/src/embedding/`）/ ADR-023 D1/D2/D5（backend 分层选型 + 默认 BM25-only）/ task-18.1（`Vector{Backend,Indexer,Searcher}` 三 trait 冻结 + `Retriever::with_vector_searcher` seam）/ ADR-014 D1-D5 第十次激活

## 1. Background

Phase 18 交付了向量基础设施：task-18.1 冻结三 trait + `Retriever::with_vector_searcher(Arc<dyn VectorSearcher>)` seam，task-18.3–18.6 落地 4 个 backend（sqlite-vec / hnsw / qdrant / lancedb），task-18.8 给出 `SemanticRecall@K` 度量，ADR-023 给出 D1-D6 分层选型（Proposed）。但 ADR-023 §D6 明确「Production runtime wiring of the chosen backend into the `Retriever` hot path requires an embedding pipeline (not yet in the project) and is out-of-scope here `[SPEC-OWNER:phase-future.vector-retrieval-integration]`」——即 Phase 18 只冻 seam，未把 backend 真正接进检索热路径，也没有把文本转向量的 embedding 环节。

task-19.1 补齐了缺失的 embedding 环节：`core/src/embedding/` 下 `EmbeddingProvider` trait（`embed(texts) -> Vec<Vec<f32>>` + `dim()` + `name()`）+ `DeterministicEmbeddingProvider`（hash/seed 派生固定维度向量，无模型依赖，默认构建启用）+ real provider（feature-gated）。

本 task 是 Phase 18 seam 与 task-19.1 embedding 的「合龙」：据 ADR-023 D1/D2 选定生产默认 backend，把它经 `with_vector_searcher` 接进 `Retriever` 热路径，并在 index 端（chunk 文本 → embedding → `index_batch`）与 query 端（query 文本 → embedding → `VectorSearcher::search`）各跑一次 `EmbeddingProvider`。Windows MSVC 约束（ADR-023 §D2：sqlite-vec MSVC 构建受阻，hnsw 是唯一全平台纯 Rust backend）决定本 task 用 **hnsw** 作为 wiring 的默认演示 backend（`vector-hnsw` feature-gated，承 ADR-023 §D5 默认构建仍 BM25-only），生产 Linux 部署可经 feature-select 换 sqlite-vec（D1）。

## 2. Goal

在 `core/src/retriever/mod.rs` 把选定默认 backend（hnsw，ADR-023 D2 全平台跨平台 backend；D1 sqlite-vec 经 feature-select 在 Linux 生产可换）接进 `Retriever` 热路径：新增「embed 后索引」与「embed query 后语义检索」两段真实通路，由 `EmbeddingProvider` 驱动 `VectorChunk.embedding` / `query_vec`。未配 backend 时（`vector_searcher == None`，承 task-18.1 默认）热路径保持 v0.10 BM25-only 不变，retrieval_method 恒 `"bm25"`；配 `NoopVectorBackend` 时 BM25 结果集与 None baseline 逐字节等价（承 TEST-18.1.7）。≥3 个同源 unit test（index→semantic-search roundtrip via embedding 真实召回 / None fallback 保持 BM25 / 选定 backend wiring 经 embedding 端到端）。默认 `cargo test --workspace` + `go test ./...` 不退化；feature build `cargo test -p contextforge-core --features vector-hnsw` 全 PASS；ADR-014 D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **修改 `core/src/retriever/mod.rs`** — wiring seam 落地：
  - `Retriever` 持有可选 `embedder: Option<Arc<dyn EmbeddingProvider>>`（与既有 `vector_searcher: Option<Arc<dyn VectorSearcher>>` 配对）；builder `with_embedder(Arc<dyn EmbeddingProvider>) -> Self`，与既有 `with_vector_searcher` 并列。
  - 新增 `index_chunks_semantic`（或等名）入口：把一批 `(chunk_id, text)` 经 `embedder.embed(texts)` 转 `Vec<VectorChunk>`，调 `VectorIndexer::open` + `index_batch` + `flush`（全量 reindex 语义，承 task-18.1 / hnsw `flush` 建图）。仅在 `embedder` 与 backend 均 `Some` 时执行；任一 `None` → no-op `Ok(())`（BM25-only 不变）。
  - 新增 `search_semantic(query, top_k)`：query 文本经 `embedder.embed(&[query])` 取首向量 → `VectorSearcher::search(&query_vec, top_k, None)` → `Vec<VectorHit>` 映射回 `SearchResult`（`retrieval_method = "vector"`，`score = VectorHit.score.as_f32()`，经 SQLite JOIN by chunk_id 补 file_path / content / line / provenance 等 12-field，复用既有 `read_provenance` + 合成兜底）。`embedder` 或 backend `None` → `Ok(vec![])`（语义路径不可用时空返，caller fallback BM25）。
  - 既有 `search()` 热路径中 task-18.1 留的 `_vector_hits` 空向量探针调用（`search(&[], ...)`）替换为：当 `embedder` 与 backend 均 `Some` 时用 query embedding 喂 `search`；否则维持 None-safe no-op。BM25 结果集合并仍不发生（hybrid fusion 属 `[SPEC-DEFER:phase-future.hybrid-scoring]`），`retrieval_method` 在既有 `search()` 返回值上保持 `"bm25"` 不变（语义结果走独立 `search_semantic` 入口）。
- **修改 `core/Cargo.toml`** — `[dev-dependencies]` 不引入新 dep；`vector-hnsw` 既有 feature 复用（task-18.6 已定义 `vector-hnsw = ["dep:instant-distance"]`）。默认 feature 维持 `default = []`（ADR-023 D5：默认构建 0 vector dep + BM25-only）。embedding 缺省 provider 由 task-19.1 在默认构建启用（无模型 dep）。
- **修改 `core/src/embedding/mod.rs`（如 task-19.1 未导出）** — 确保 `EmbeddingProvider` + `DeterministicEmbeddingProvider` 经 `pub use` 在 `crate::embedding::` 可达，供 retriever wiring import（若 task-19.1 已导出则本 task 不动）。
- **新增 retriever `mod tests`（≥3）** — TEST-19.2.1/2/3（见 §6/§7）：deterministic embedding + hnsw backend 经 fixture index→search roundtrip 真实召回 / None fallback 保持 BM25（承 TEST-18.1.6 风格，等价 baseline）/ 选定 backend wiring 经 embedder 端到端非空命中。`vector-hnsw` feature-gated（`#[cfg(feature = "vector-hnsw")]` 守护需 hnsw 的测试；deterministic provider 测试默认构建可跑）。
- **修改 `docs/s2v-adapter.md`** — Phase 19 任务表 19.2 行 Pending → Done（实施后）。

### Out of Scope（[SPEC-DEFER] / [SPEC-OWNER]）

- **proto SearchRequest `semantic` flag + Go `/v1/search?semantic=true` 通路** [SPEC-OWNER:task-19.3-semantic-search-api]：本 task 仅 Rust core 层 wiring + Rust public API（`search_semantic`），跨语言 contract 演进归 19.3。
- **smoke v9 30-step + eval `--semantic` CLI** [SPEC-OWNER:task-19.4-smoke-v9]：端到端 smoke 与 eval CLI 归 19.4。
- **真实 dogfood embedding SemanticRecall@K 实测 + ADR-023 ratify** [SPEC-OWNER:task-19.5-real-recall-eval]：本 task 用 deterministic 缺省 provider 跑 wiring 正确性（确定性向量 → 同义文本近邻可断言），真实模型召回数据归 19.5；ADR-013 禁据合成数据 ratify ADR-023。
- **BM25 + Vector 混合打分融合（hybrid fusion）** [SPEC-DEFER:phase-future.hybrid-scoring]：本 task 语义路径经独立 `search_semantic` 入口，BM25 结果集不被语义命中扰动（承 Phase 18 §不在 scope）。
- **hnsw 图持久化 / rebuild-on-load** [SPEC-DEFER:phase-future.hnsw-graph-persistence]：承 Phase 18，wiring 用内存图 + 索引时建图（hnsw `flush` 语义），落盘后置。
- **生产 Linux 切 sqlite-vec(D1) 默认** [SPEC-DEFER:phase-future.sqlite-vec-cross-platform]：sqlite-vec MSVC 受阻（ADR-023 D2），本 task wiring 默认演示 backend 用全平台 hnsw；Linux 生产经 `vector-sqlite` feature-select 换 D1。
- **vector index 增量更新** [SPEC-DEFER:phase-future.vector-incremental-index]：承 Phase 18，默认全量 reindex（hnsw 一次性建图）。

## 4. Actors

- **主 agent**：实施 + PR 主理（ADR-012 自治）。
- **`Retriever`（core）**：wiring 宿主——持有 `embedder` + `vector_searcher` 两可选注入点，暴露 `with_embedder` / `index_chunks_semantic` / `search_semantic` 公开 API。
- **`EmbeddingProvider`（task-19.1）**：文本 → 向量；本 task 测试用 `DeterministicEmbeddingProvider`（确定性，可断言近邻）。
- **`HnswBackend`（task-18.6）**：选定默认 wiring backend（ADR-023 D2 全平台），实现 `VectorIndexer` + `VectorSearcher`。
- **下游 task-19.3**：消费本 task 的 `search_semantic` Rust API，wrap 进 gRPC semantic 路径。
- **下游 task-19.5**：把本 task 的 wiring 接 real provider 跑真实 SemanticRecall@K。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/tasks/task-19.1-spike-embedding-provider.md`（`EmbeddingProvider` trait 签名 + `DeterministicEmbeddingProvider` 确定性契约）
- `docs/specs/tasks/task-18.1-vector-trait.md`（`Vector{Backend,Indexer,Searcher}` 签名 + `with_vector_searcher` seam）+ `docs/specs/tasks/task-18.6-spike-hnsw.md`（hnsw backend 实现 + 全平台凭据）
- `core/src/retriever/mod.rs`（既有 `vector_searcher: Option<Arc<dyn VectorSearcher>>` + `with_vector_searcher` + `search()` 中 task-18.1 探针调用 + 12-field `SearchResult` JOIN 装配 + `read_provenance` 合成兜底）
- `core/src/retriever/vector/{traits,types}.rs`（`VectorChunk` / `VectorHit` / `VectorScore` / `VectorIndexConfig` / `VectorMetric` 形状）+ `core/src/retriever/vector/hnsw.rs`（`HnswBackend::new` + open/index_batch/flush/search 语义，normalize + L2 单调 cosine）
- `docs/decisions/adr-023-vector-backend-default.md`（D1 sqlite-vec 生产 / D2 hnsw 全平台 / D5 默认 BM25-only / D6 wiring forward-ref 本 task 解除）
- `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5）

### 5.2 Imports（默认 0 新 dep；embedding/hnsw 经既有 crate 内模块 + feature-gate）

```rust
use std::sync::Arc;
use crate::embedding::EmbeddingProvider;                    // task-19.1
use crate::retriever::vector::{VectorSearcher, VectorIndexer}; // task-18.1
use crate::retriever::vector::types::{ChunkId, VectorChunk, VectorIndexConfig, VectorMetric};
// 测试侧（#[cfg(feature = "vector-hnsw")]）:
// use crate::retriever::vector::HnswBackend;
// use crate::embedding::DeterministicEmbeddingProvider;
```

### 5.3 关键设计

- `Retriever` 加 `embedder: Option<Arc<dyn EmbeddingProvider>>` 字段，默认 `None`（`open` / `open_with_config` 构造 `None`，与 `vector_searcher: None` 对称）；`with_embedder(self, Arc<dyn EmbeddingProvider>) -> Self` builder 链式注入（仿 `with_vector_searcher`）。
- **index 端 `index_chunks_semantic(&self, items: &[(String, String)])`**：`items` = `(chunk_id, text)`；`texts: Vec<&str>` → `embedder.embed(texts)` → 逐条 `VectorChunk { chunk_id: ChunkId(id), embedding, metadata: None }`；`indexer.open(VectorIndexConfig { dim: embedder.dim(), metric: VectorMetric::Cosine, persistence_path: None, collection_id })` + `index_batch` + `flush`。两注入点任一 `None` → 立即 `Ok(())`（BM25-only 路径不建任何向量索引）。`VectorSearcher` trait object 需同时是 `VectorIndexer` 才能索引——wiring 用同时实现两 trait 的具体 backend（hnsw），故注入点设计上索引侧持具体 `Arc<dyn VectorSearcher>` 并要求其 backend 经 `index_chunks_semantic` 在持有具体类型处建好（详见实现：索引建图发生在持有 `HnswBackend` 实例的调用方，wiring API 接已建好的 `Arc<dyn VectorSearcher>`）。
- **query 端 `search_semantic(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>, RetrieverError>`**：`embedder` 或 `vector_searcher` 为 `None`、或 query trim 空 → `Ok(vec![])`（caller 据空集 fallback BM25）；否则 `embedder.embed(&[query])` 取 `vecs[0]` → `searcher.search(&query_vec, top_k, None)` → 每个 `VectorHit` 经 SQLite JOIN by `chunk_id.0` 装配 12-field `SearchResult`（`retrieval_method = "vector"`，`score = hit.score.as_f32()`，provenance 复用 `read_provenance` + 合成兜底 ≥1 entry，承 AC3 黑盒守护）；JOIN 未命中的 hit skip（Tantivy/SQLite/vector 暂不同步）。
- **既有 `search()` 不退化**：BM25 主路径不变；task-18.1 留的 `searcher.search(&[], ...)` 空向量探针——当 `embedder` 与 backend 均 `Some` 时改用 query embedding 喂探针（结果仍仅 log、不并入 BM25 结果集，hybrid fusion `[SPEC-DEFER:phase-future.hybrid-scoring]`）；任一 `None` 时维持 None-safe no-op。`search()` 返回的 `retrieval_method` 恒 `"bm25"`。
- **score 语义**：hnsw `search` 已把 unit 向量 L2∈[0,2] 映射为 sim∈[0,1]（`1 - distance/2`），`VectorScore::as_f32()` 直填 `SearchResult.score`；deterministic provider 派生向量同义文本近邻可断言 recall（TEST-19.2.1）。

## 6. Acceptance Criteria

- [ ] **AC1**: `Retriever` 暴露 `with_embedder` + `search_semantic` + index 端 wiring 入口；`embedder` 默认 `None`，与 `vector_searcher` 对称；`cargo build -p contextforge-core --features vector-hnsw` 与默认 `cargo build -p contextforge-core` 均 exit 0（ADR-023 D5：默认构建 0 vector dep）— verified by **TEST-19.2.1**（wiring API 存在 + 默认/feature build 双绿）
- [ ] **AC2**: index→semantic-search roundtrip 真实召回 — deterministic 缺省 provider 对一组文本 embed 后经选定 backend 索引；用语义相近 query embed 检索，命中目标 chunk（`retrieval_method == "vector"`，score 有序非伪造）— verified by **TEST-19.2.2**（hnsw + DeterministicEmbeddingProvider roundtrip 命中目标 chunk_id）
- [ ] **AC3**: None fallback 保持 BM25 — 未注入 `embedder` / `vector_searcher`（默认 `Retriever::open`）时，`search()` 结果与 task-18.1 baseline 逐字节等价（chunk_ids / scores / order 一致），`retrieval_method` 恒 `"bm25"`；`search_semantic` 在 None 时返 `Ok(vec![])` 不 Err 不 panic — verified by **TEST-19.2.3**（None baseline 等价 + search_semantic 空返）
- [ ] **AC4**: 选定 backend wiring 端到端 — `with_embedder` + `with_vector_searcher` 注入后 `search_semantic` 走真实 embedding→backend→12-field SearchResult 装配（provenance ≥1 黑盒守护），无 panic；空 query / 维度路径不崩 — verified by **TEST-19.2.4**（wiring 端到端非空命中 + provenance floor + 空 query 空返）
- [ ] **AC5**: 既有不退化 — 默认 `cargo test --workspace` 全 PASS（vector-hnsw 默认不启用，gated 测试不入默认编译）；`cargo test -p contextforge-core --features vector-hnsw` 全 PASS；`go test ./...` 全 PASS — verified by **TEST-19.2.5**（默认 + feature 双 build test 0 failed）+ §10 实测
- [ ] **AC6**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched master` PR 触及行 0 未标注命中 — verified by §10 记录的 D2 lint 实跑输出

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-19.2.1 | wiring API 存在 + 默认/feature build 双绿 | `core/src/retriever/mod.rs` mod tests + `cargo build` ×2 | Pending |
| TEST-19.2.2 | hnsw + deterministic provider index→search roundtrip 命中目标 chunk | `core/src/retriever/mod.rs` mod tests（`#[cfg(feature = "vector-hnsw")]`） | Pending |
| TEST-19.2.3 | None baseline BM25 等价 + search_semantic 空返 | `core/src/retriever/mod.rs` mod tests | Pending |
| TEST-19.2.4 | wiring 端到端非空命中 + provenance ≥1 + 空 query 空返 | `core/src/retriever/mod.rs` mod tests（`#[cfg(feature = "vector-hnsw")]`） | Pending |
| TEST-19.2.5 | 默认 + vector-hnsw feature build test 0 failed | 全 workspace + `cargo test -p contextforge-core --features vector-hnsw` | Pending |

## 8. Risks

- **R1（中）`VectorSearcher` trait object 不携 `VectorIndexer`**：`Arc<dyn VectorSearcher>` 只读路径无法直接 `index_batch`（写路径在 `VectorIndexer`）。
  - **缓解**：index 端 wiring 在持有具体 backend 类型（如 `HnswBackend`，同时实现两 trait）处建图后再以 `Arc<dyn VectorSearcher>` 注入 query 端；`index_chunks_semantic` 的索引建图与注入边界在实现 §5.3 明确；测试用具体 `HnswBackend` 走完整 index→search。
- **R2（中）deterministic 缺省 provider 召回偏理想**：hash/seed 派生向量与真实语义分布不同，roundtrip 命中是 wiring 正确性证据，非真实召回质量。
  - **缓解**：TEST-19.2.2 断言「wiring 通路正确 + 同义文本近邻可分」而非召回阈值；真实 SemanticRecall@K 归 [SPEC-OWNER:task-19.5-real-recall-eval]（ADR-013 禁据合成 ratify）。
- **R3（低）hnsw 内存图无持久化**：`Retriever` 进程内建图，重启需重建（Phase 18 记 28s @100k）。
  - **缓解**：wiring 用索引时建图（hnsw `flush`）；落盘 [SPEC-DEFER:phase-future.hnsw-graph-persistence] 承 Phase 18；本 task fixture 规模小，建图廉价。
- **R4（低）默认构建引入 vector dep**：误把 hnsw/embedding real provider 拉进 `default` feature 破坏 ADR-023 D5。
  - **缓解**：`vector-hnsw` 维持 optional + feature-gated；deterministic provider 无模型 dep；TEST-19.2.1 显式校验默认 `cargo build` 不含 instant-distance（cfg-gated path 不入默认编译）。

## 9. Verification Plan

```bash
# 默认构建（BM25-only，0 vector dep — ADR-023 D5）
cargo build -p contextforge-core
cargo test --workspace                 # 默认 feature，vector-hnsw gated 不入编译

# 选定默认 backend wiring（hnsw 全平台 — ADR-023 D2）
cargo build -p contextforge-core --features vector-hnsw
cargo test -p contextforge-core --features vector-hnsw

# Go 控制面不退化
go test ./...

# ADR-014 D2 spec-drift lint（PR 触及行 0 未标注命中）
bash scripts/spec_drift_lint.sh --touched master
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**：（实现后填）
- **改动文件**：`core/src/retriever/mod.rs`（`with_embedder` + `search_semantic` + index wiring + mod tests）、`core/Cargo.toml`（如需 dev-dep 调整；默认 feature 维持 `default = []`）、`core/src/embedding/mod.rs`（如需补 `pub use` 导出，承 task-19.1）、`docs/s2v-adapter.md`（19.2 行 Pending → Done）（实现后据实补全）
- **commit 列表**：（实现后填）见本 task PR（分支 `feat/task-19.2-default-backend-wiring`）；合入后以 merge commit 为准
- **§9 Verification 结果**：（实现后填）默认 `cargo test --workspace` / `cargo test -p contextforge-core --features vector-hnsw` / `go test ./...` / `bash scripts/spec_drift_lint.sh --touched master` 实测输出
- **剩余风险 / 未做项**：（实现后填）proto/Go semantic 通路见 [SPEC-OWNER:task-19.3-semantic-search-api]；smoke v9 + eval CLI 见 [SPEC-OWNER:task-19.4-smoke-v9]；真实召回 + ADR-023 ratify 见 [SPEC-OWNER:task-19.5-real-recall-eval]；hybrid fusion [SPEC-DEFER:phase-future.hybrid-scoring]；hnsw 持久化 [SPEC-DEFER:phase-future.hnsw-graph-persistence]
- **下游 task 影响**：（实现后填）task-19.3（wrap `search_semantic` 进 gRPC semantic 路径）/ task-19.5（接 real provider 跑真实 SemanticRecall@K）
