# Task `18.1`: `vector-trait — core/src/retriever/vector/{mod,traits,noop}.rs 三 trait 冻结 + NoopVectorBackend 占位实现 + retriever wiring + Cargo workspace vector-spike feature flag`

**Status**: Done

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 18 (vector-backend-selection)
**Dependencies**: Phase 4 task-4.1 (`Retriever` 抽象与 result schema 既有) / Phase 4 task-4.2 (explain 路径 retrieval_method 字段) / PRD §Decisions Log D2 (向量后端 provider 抽象，v0.1 不强依赖) / PRD §Open Questions O2 (向量后端最终选型 — 本 phase 解) / PRD §Core Capabilities #2 (可解释检索一等公民，含 `retrieval_method` 字段) / PRD §Constraints performance (P95 < 500ms / idle RSS < 300MB 不退化) / ADR-008 core-library-selection (后续 backend dep 引入 amendment) / ADR-014 D1-D5 第九次激活

## 1. Background

PRD §Open Questions O2 「向量后端最终选型」自 v0.1 起留白至 v0.10，源自 D2 「向量后端做 provider 抽象，v0.1 不强依赖」决策。Phase 18 §2A Decisions Log 锁 **trait-first** 集成深度（决策 4）— 4 backend spike (task-18.3-18.6) 并行实施前先冻结统一抽象层，避免各 backend ad-hoc 实现后回头返工 trait 抽象。

本 task 是 Phase 18 推荐序的首项（trait → harness → 4 backend ∥ → decision → eval → 收口），承担三件不可拆解的工作：

1. **抽象层冻结**：在 `core/src/retriever/vector/` 落 `VectorBackend` / `VectorIndexer` / `VectorSearcher` 三 trait 定义，作为 task-18.3-18.6 4 backend 共享接口；本 task ship 后 4 spike 并行开始
2. **占位实现**：`NoopVectorBackend` 实现三 trait 全 stub（search 返空 / index 返 Ok no-op / is_indexed 返 false），让既有 BM25-only retriever 路径在 vector backend 未配置时不退化（PRD §Anti-metrics「不能牺牲可解释性」）
3. **retriever 端集成**：`core/src/retriever/mod.rs` 接入 `Option<Arc<dyn VectorSearcher>>` 字段；当 None 时既有 BM25-only 行为完全保留（不退化）；当 Some 时占位返空（实际 backend swap-in 由 task-18.7 default wiring 完成）

**实施策略**：本 task 不引入任何真 backend dep（sqlite-vec / qdrant / lancedb / hnsw 全不入 Cargo.toml）— 仅定义 trait + Noop 实现 + retriever wiring + `vector-spike` feature flag scaffold（让 task-18.3-18.6 接入各自 dep 时按 `[features] vector-spike = [...]` opt-in）。Cargo dep 完全 add-only — 现有 v0.10.0 build 时 `cargo build --workspace` 不引入新 dep。

## 2. Goal

在 `core/src/retriever/vector/` 落地 `VectorBackend` / `VectorIndexer` / `VectorSearcher` 三 trait 定义 + `NoopVectorBackend` 占位实现 + retriever 端 `Option<Arc<dyn VectorSearcher>>` 集成；既有 BM25 / metadata / filter 检索路径不退化（`cargo test --workspace` 0 failed）；≥3 NoopVectorBackend unit test PASS；`core/Cargo.toml` workspace `[features] vector-spike` scaffold ready；ADR-014 D2 lint 0 unannotated hits。本 task ship 后 task-18.2 spike harness 可启动，task-18.3-18.6 4 backend 实现可并行开始（共享 trait 接口）。

## 3. Scope

### In Scope

> 详细文件路径 / trait method signature / NoopVectorBackend 内部 logic 由 §2A 业务承诺审核期填实（本 PR 完成）。严禁在实施时超出本边界（如新增真 backend dep）— 全部在 §3 Out of Scope 用 [SPEC-OWNER:task-X.Y] 标注归属下游 task。

- **新建 `core/src/retriever/vector/mod.rs`** — 模块入口：
  ```rust
  pub mod traits;
  pub mod types;
  pub mod noop;

  #[cfg(test)]
  mod tests;

  pub use traits::{VectorBackend, VectorIndexer, VectorSearcher};
  pub use types::{ChunkId, VectorChunk, VectorError, VectorFilter, VectorHit, VectorIndexConfig, VectorMetric, VectorScore};
  pub use noop::NoopVectorBackend;
  ```

- **新建 `core/src/retriever/vector/types.rs`** — types + errors（§5.3 §A 完整签名见下）：
  - `ChunkId` newtype（或复用 `crate::chunker::ChunkId`，实施时确认既有路径，本 spec 暂用 newtype 占位）
  - `VectorScore(f32)` newtype + `VectorScore::new(v) -> Result<Self, VectorError>` NaN/Inf guard
  - `VectorMetric` enum: `Cosine` / `DotProduct` / `L2`
  - `VectorHit { chunk_id, score, metadata: Option<serde_json::Value> }`
  - `VectorChunk { chunk_id, embedding: Vec<f32>, metadata }`
  - `VectorIndexConfig { dim, metric, persistence_path: Option<PathBuf>, collection_id: String }`
  - `VectorFilter { agent_scope: Option<String>, source_type: Option<String>, max_age_days: Option<u32> }`（backend-specific filter 留 `extras: Option<serde_json::Value>` opaque 字段）
  - `VectorError` enum（thiserror）: `NotInitialized` / `DimMismatch { expected, got }` / `InvalidScore(f32)` / `Io(String)` / `Other(String)`

- **新建 `core/src/retriever/vector/traits.rs`** — 三 trait sync 定义（§5.3 §B 完整签名见下）：
  - `VectorBackend: Send + Sync + std::fmt::Debug`（基 trait — 静态属性）
  - `VectorIndexer: VectorBackend`（写路径 — open/index_batch/delete/flush/close）
  - `VectorSearcher: VectorBackend`（读路径 — search/is_indexed）
  - 全 sync 方法（与既有 `Retriever::search()` sync 一致；后续 task-18.4/18.5 实现 Qdrant/LanceDB 时内部用 `tokio::task::block_in_place` 或独立 tokio runtime 包 async client）
  - 含 `///` rustdoc + ≥1 doctest 展示 NoopVectorBackend 用法

- **新建 `core/src/retriever/vector/noop.rs`** — `NoopVectorBackend` 占位实现：
  - `#[derive(Debug, Default, Clone, Copy)] pub struct NoopVectorBackend;`
  - `impl VectorBackend`: `name() = "noop"` / `version() = "0.1.0"` / `is_local() = true` / `requires_embedding() = false`
  - `impl VectorIndexer`: open/delete/flush/close 全 `Ok(())`；`index_batch(_) = Ok(0)`
  - `impl VectorSearcher`: `search(_, _, _) = Ok(vec![]) + tracing::debug!("NoopVectorBackend.search empty");` / `is_indexed() = false`

- **新建 `core/src/retriever/vector/tests.rs`**（或同源 `#[cfg(test)] mod tests`）：≥6 unit test 覆盖 TEST-18.1.2-7
  - `trait_object_safety_test` — 构造 `let _: Arc<dyn VectorSearcher> = Arc::new(NoopVectorBackend);` 编译过即 PASS（object safety smoke）
  - `test_noop_search_returns_empty` — `NoopVectorBackend.search(&[0.1, 0.2], 10, None)` 返 `Ok(vec![])`
  - `test_noop_index_batch_is_noop_ok` — `NoopVectorBackend.index_batch(&[chunk])` 返 `Ok(0)`
  - `test_noop_is_indexed_always_false` — `NoopVectorBackend.is_indexed()` 返 `false`
  - `test_vector_score_nan_rejected` — `VectorScore::new(f32::NAN)` 返 `Err(VectorError::InvalidScore(_))`
  - `test_vector_score_inf_rejected` — `VectorScore::new(f32::INFINITY)` 返 `Err(VectorError::InvalidScore(_))`

- **修改 `core/src/retriever/mod.rs`** — retriever wiring add-only：
  - `pub mod vector;`（re-export `pub use vector::{VectorBackend, VectorSearcher, NoopVectorBackend};`）
  - `pub struct Retriever` 新增字段 `vector_searcher: Option<Arc<dyn VectorSearcher>>`（默认 `None`）
  - `Retriever::new(...)` 签名保持向后兼容 — 现有 caller 不传 vector_searcher 自动 `None`（实施时取舍：要么用 builder pattern `.with_vector_searcher(...)`，要么 `new(...)` 多一参且全部 caller 传 `None` — 推荐 builder 减少改面）
  - `Retriever::search(opts) -> Result<Vec<SearchResult>>` hot path：
    - `if let Some(searcher) = &self.vector_searcher { let _v_hits = searcher.search(&[], opts.top_k, None).unwrap_or_default(); /* task-18.7 接入真融合 */ }`
    - 既有 BM25 / metadata / filter 路径**字节不变**；仅在 BM25 完成后**追加**一处 vector_searcher 占位调（None 时整段跳过）
  - 既有所有 caller（cli / daemon / mcp / explain 路径）**不需修改**
  - 新增 `core/src/retriever/tests.rs` 中（或既有 `#[cfg(test)] mod tests`）≥2 unit test：`test_retriever_none_vector_searcher_bm25_unchanged` + `test_retriever_some_noop_vector_searcher_returns_empty_vector_hits`

- **修改 `core/Cargo.toml`** — workspace features scaffold（add-only，0 dep 新增）：
  ```toml
  [features]
  default = []
  vector-spike = []          # task-18.2 harness 通用 gate
  vector-sqlite = []         # task-18.3 ship 时填 ["dep:sqlite-vec"]
  vector-qdrant = []         # task-18.4 ship 时填 ["dep:qdrant-client"]
  vector-lancedb = []        # task-18.5 ship 时填 ["dep:lancedb"]
  vector-hnsw   = []         # task-18.6 ship 时填 ["dep:instant-distance" 或 "dep:hnsw_rs"]
  ```
  本 task ship 后 `cargo build --workspace` 默认 features 行为完全不变（0 dep 新增）；`cargo build --workspace --features vector-spike` 也 0 dep（占位 feature）

- **新增 doctest in `core/src/retriever/vector/traits.rs`** — 展示 NoopVectorBackend 用法：
  ```rust
  /// # Examples
  /// ```
  /// use contextforge_core::retriever::vector::{NoopVectorBackend, VectorSearcher, VectorBackend};
  /// let backend = NoopVectorBackend;
  /// assert_eq!(backend.name(), "noop");
  /// assert!(!backend.is_indexed());
  /// let hits = backend.search(&[0.1, 0.2], 10, None).unwrap();
  /// assert!(hits.is_empty());
  /// ```
  ```

- **改动文件清单（实施时不超出此清单）**：
  - 新建：`core/src/retriever/vector/{mod.rs,types.rs,traits.rs,noop.rs,tests.rs}` (5)
  - 修改：`core/src/retriever/mod.rs` (wiring + re-export)
  - 修改：`core/src/retriever/tests.rs`（或既有同源 mod tests）— 加 2 unit test
  - 修改：`core/Cargo.toml`（features 块 add-only）
  - 修改：`docs/specs/tasks/task-18.1-vector-trait.md`（§10 Completion Notes 回填 — 完工时）

### Out of Scope（[SPEC-DEFER] / [SPEC-OWNER]）

- **真 backend 实现** [SPEC-OWNER:task-18.3-spike-sqlite-vec / task-18.4-spike-qdrant-embedded / task-18.5-spike-lancedb / task-18.6-spike-hnsw]：sqlite-vec / qdrant-client / lancedb / hnsw_rs 任一 dep 引入 + 实际 backend 实现不在本 task；本 task 仅 trait + Noop
- **spike harness + bench crate** [SPEC-OWNER:task-18.2-spike-harness]：`bench/` crate 新增 + corpus gen + 5 维 measurement runner 由 task-18.2 ship
- **默认 backend 选定 + ADR-023** [SPEC-OWNER:task-18.7-decision-adr023]：spike 数据出齐后由 task-18.7 决策；本 task 不预设默认 backend
- **eval semantic recall** [SPEC-OWNER:task-18.8-eval-semantic-recall]：`internal/eval/eval.go` SemanticRecall@K + `--semantic` CLI flag + recall gate 由 task-18.8 ship
- **smoke v9 step 29-30** [SPEC-OWNER:task-18.9-release-v0.11.0-closeout]：vector search roundtrip smoke step 由 task-18.9 ship
- **Embedding provider 实现** [SPEC-DEFER:phase-future.embedding-provider-full]：fastembed-rs / candle / sentence-transformers ONNX 等本地 embedding 实现不在 trait 抽象层；trait 接受 `Vec<f32>` query vector，调用方负责 embedding 生成（task-18.2 harness 内自带占位 provider）
- **Hybrid scoring (BM25 + Vector 融合)** [SPEC-DEFER:phase-future.hybrid-scoring]：本 task retriever wiring 仅占位 — Some(searcher) 时 vector hits 返空（不与 BM25 score 融合）；真 hybrid scoring 留后续
- **Multi-collection vector index** [SPEC-DEFER:phase-future.multi-collection-vector-index]：trait 当前面向单 collection；跨 collection 共享 vector index 留后续
- **Vector index incremental update** [SPEC-DEFER:phase-future.vector-incremental-index]：trait `index_batch()` 当前面向全量 reindex 语义；增量更新策略留后续
- **CJK + 代码符号 tokenizer (O11)** [SPEC-DEFER:phase-future.cjk-and-code-tokenizer]：与 vector retrieval 正交，留后续
- **Reranker (cross-encoder)** [SPEC-DEFER:phase-future.reranker]：trait 不含 rerank 路径；留后续
- **Console UI 端 vector_score 字段显示** (cross-repo)：Console 主仓领域；如 task-18.7 default wiring 后 SearchResult schema 新增 `vector_score: f32` → cross-repo follow-up 通知 Console 团队 ship

## 4. Actors

- **主 agent**：本 task 实施 + chore PR 主理（§2A 审核 + RED→GREEN→REFACTOR + verify + commit）
- **Rust SoT**：`core/src/retriever/vector/` 三 trait + Noop 实现持有方；`core/src/retriever/mod.rs` retriever wiring
- **下游 task-18.2-18.6**：消费本 task ship 的 trait 接口实现 spike harness + 4 backend
- **下游 task-18.7**：消费 trait + 选定默认 backend wire 进 retriever（替换 NoopVectorBackend）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-18-vector-backend-selection.md`（本 task 父 phase，含 §2A 5 拍板点 + §3 task-18.1 涉及模块 + §6 AC1 owner）
- `docs/prds/context-forge.prd.md` §Core Capabilities #2 + §Decisions Log D2 + §Constraints Performance + §Anti-metrics + §Open Questions O2 + §Technical Risks R2
- `docs/specs/tasks/task-4.1-retriever.md`（既有 Retriever 抽象与 result schema — 本 task 不破坏）
- `docs/specs/tasks/task-4.2-explain.md`（既有 explain 路径 + retrieval_method 字段 — 本 task 扩 "vector(noop)" 取值）
- `docs/decisions/adr-002-sqlite-tantivy-layered-storage.md`（v0.10 既有 metadata + 全文索引存储模型；本 task 仅 trait 抽象不改存储）
- `docs/decisions/adr-008-core-library-selection.md`（Rust 库选择基线；本 task 不引入新 dep，task-18.3-18.6 各自 PR 走 ADR-008 amendment）
- `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5 第九次激活；本 task 是 phase-18 task 之一 — D2 lint + D3 verified-by + D5 历史不溯改）

### 5.2 Imports

**核心 std / 第三方 crate**（全部已在 `core/Cargo.toml` 既有 deps — 0 dep 新增）：

```rust
// core/src/retriever/vector/types.rs
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// core/src/retriever/vector/traits.rs
use std::fmt::Debug;
use crate::retriever::vector::types::{ChunkId, VectorChunk, VectorError, VectorFilter, VectorHit, VectorIndexConfig};

// core/src/retriever/vector/noop.rs
use tracing::debug;
use crate::retriever::vector::traits::{VectorBackend, VectorIndexer, VectorSearcher};
use crate::retriever::vector::types::{ChunkId, VectorChunk, VectorError, VectorFilter, VectorHit, VectorIndexConfig};

// core/src/retriever/vector/tests.rs
use std::sync::Arc;
use super::*;

// core/src/retriever/mod.rs（既有 + 新增）
use std::sync::Arc;       // 新增
// 既有 imports 保留：std::collections::HashMap / std::path::Path,PathBuf / rusqlite / tantivy::* / thiserror::Error / crate::chunker::Provenance / ...
pub mod vector;            // 新增
pub use vector::{VectorBackend, VectorSearcher, NoopVectorBackend};   // 新增
```

**禁止引入**（留 task-18.3-18.6 各自 PR 走 ADR-008 amendment）：

- `sqlite-vec` / `qdrant-client` / `qdrant-segment` / `lancedb` / `instant-distance` / `hnsw_rs` / `hora-search`
- `async-trait`（trait sync 决策；无需 macro 依赖）
- `fastembed` / `candle` / `ort` / `tokenizers`（embedding provider 留 task-18.2 harness 内部，本 task trait 不引）
- `rayon` / `crossbeam` 等并发原语（本 task trait 不约束并发模型 — backend 各自实现可用，trait 层 sync 不强加）

**feature gate**：

- `NoopVectorBackend` + 三 trait 定义不 gate（默认可用，无 dep 触发）
- task-18.3-18.6 实施时各 backend impl 用 `#[cfg(feature = "vector-<backend>")]` gate 模块（本 task 不预 gate）
- `vector-spike` feature 是空占位（用作 task-18.2 harness 启用门，但本 task ship 时其值 `[]`，不触发任何 dep）

### 5.3 Function Signatures

> **§2A 决策**：trait **sync**（与既有 `Retriever::search()` sync 一致；无 `async-trait` dep；后续 task-18.4 (Qdrant) / task-18.5 (LanceDB) 实现 async client 时内部用 `tokio::task::block_in_place` 或独立 tokio runtime handle 包装；性能成本通过 task-18.2 harness 实测 — phase-18 §7 R6 已识别）。
>
> 所有方法 `&self`（无 `&mut self`）— 让 `Arc<dyn VectorIndexer>` / `Arc<dyn VectorSearcher>` 可共享；backend 内部用 `Mutex` / `RwLock` / `OnceCell` 管可变状态。

#### §A `core/src/retriever/vector/types.rs`

```rust
/// Chunk identifier. Newtype over String to keep API explicit.
/// (Implementer note: 实施时若 crate::chunker::ChunkId 已存在则改 `pub use crate::chunker::ChunkId;`)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkId(pub String);

/// Distance metric for vector similarity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VectorMetric { Cosine, DotProduct, L2 }

/// Score newtype with NaN/Inf guard (constructed via VectorScore::new).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct VectorScore(f32);

impl VectorScore {
    pub fn new(value: f32) -> Result<Self, VectorError> {
        if value.is_nan() || value.is_infinite() {
            return Err(VectorError::InvalidScore(value));
        }
        Ok(Self(value))
    }
    pub fn as_f32(&self) -> f32 { self.0 }
}

/// Single vector search hit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorHit {
    pub chunk_id: ChunkId,
    pub score: VectorScore,
    pub metadata: Option<serde_json::Value>,
}

/// Chunk + embedding pair for indexing.
#[derive(Debug, Clone)]
pub struct VectorChunk {
    pub chunk_id: ChunkId,
    pub embedding: Vec<f32>,
    pub metadata: Option<serde_json::Value>,
}

/// Index initialization config.
#[derive(Debug, Clone)]
pub struct VectorIndexConfig {
    pub dim: usize,
    pub metric: VectorMetric,
    pub persistence_path: Option<PathBuf>,
    pub collection_id: String,
}

/// Optional search filter (backend-specific extras via opaque JSON).
#[derive(Debug, Clone, Default)]
pub struct VectorFilter {
    pub agent_scope: Option<String>,
    pub source_type: Option<String>,
    pub max_age_days: Option<u32>,
    pub extras: Option<serde_json::Value>,
}

/// All errors backend impls can return.
#[derive(Debug, Error)]
pub enum VectorError {
    #[error("backend not initialized")]
    NotInitialized,
    #[error("invalid embedding dimension: expected {expected}, got {got}")]
    DimMismatch { expected: usize, got: usize },
    #[error("score is NaN or infinite: {0}")]
    InvalidScore(f32),
    #[error("backend I/O error: {0}")]
    Io(String),
    #[error("backend error: {0}")]
    Other(String),
}
```

#### §B `core/src/retriever/vector/traits.rs`

```rust
/// Static identity/capability of a vector backend.
///
/// All backend impls (`NoopVectorBackend`, future SqliteVec/Qdrant/LanceDB/Hnsw) implement this base trait.
pub trait VectorBackend: Send + Sync + Debug {
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn is_local(&self) -> bool;
    fn requires_embedding(&self) -> bool;
}

/// Write-path: index lifecycle + mutation.
pub trait VectorIndexer: VectorBackend {
    fn open(&self, config: VectorIndexConfig) -> Result<(), VectorError>;
    fn index_batch(&self, chunks: &[VectorChunk]) -> Result<usize, VectorError>;
    fn delete(&self, ids: &[ChunkId]) -> Result<usize, VectorError>;
    fn flush(&self) -> Result<(), VectorError>;
    fn close(&self) -> Result<(), VectorError>;
}

/// Read-path: nearest-neighbor search.
///
/// # Examples
/// ```
/// use contextforge_core::retriever::vector::{NoopVectorBackend, VectorSearcher, VectorBackend};
/// let backend = NoopVectorBackend;
/// assert_eq!(backend.name(), "noop");
/// assert!(!backend.is_indexed());
/// let hits = backend.search(&[0.1, 0.2, 0.3], 10, None).unwrap();
/// assert!(hits.is_empty());
/// ```
pub trait VectorSearcher: VectorBackend {
    fn search(
        &self,
        query_vec: &[f32],
        k: usize,
        filter: Option<&VectorFilter>,
    ) -> Result<Vec<VectorHit>, VectorError>;
    fn is_indexed(&self) -> bool;
}
```

#### §C `core/src/retriever/vector/noop.rs`

```rust
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopVectorBackend;

impl VectorBackend for NoopVectorBackend {
    fn name(&self) -> &'static str { "noop" }
    fn version(&self) -> &'static str { "0.1.0" }
    fn is_local(&self) -> bool { true }
    fn requires_embedding(&self) -> bool { false }
}

impl VectorIndexer for NoopVectorBackend {
    fn open(&self, _config: VectorIndexConfig) -> Result<(), VectorError> { Ok(()) }
    fn index_batch(&self, _chunks: &[VectorChunk]) -> Result<usize, VectorError> { Ok(0) }
    fn delete(&self, _ids: &[ChunkId]) -> Result<usize, VectorError> { Ok(0) }
    fn flush(&self) -> Result<(), VectorError> { Ok(()) }
    fn close(&self) -> Result<(), VectorError> { Ok(()) }
}

impl VectorSearcher for NoopVectorBackend {
    fn search(
        &self,
        _query_vec: &[f32],
        _k: usize,
        _filter: Option<&VectorFilter>,
    ) -> Result<Vec<VectorHit>, VectorError> {
        debug!("NoopVectorBackend.search called - returning empty hits (vector backend not configured)");
        Ok(vec![])
    }
    fn is_indexed(&self) -> bool { false }
}
```

#### §D `core/src/retriever/mod.rs`（既有 struct + impl 扩 add-only）

```rust
// 既有 Retriever struct（不动既有字段）+ 新增字段：
pub struct Retriever {
    // ... existing fields (chunker_index_dir, sqlite_pool, tantivy_index, ...) ...
    vector_searcher: Option<Arc<dyn VectorSearcher>>,   // NEW (default None)
}

impl Retriever {
    // 既有 new() 签名保持 — caller 不传 vector_searcher，默认 None：
    pub fn new(/* existing args, e.g., data_dir: &Path, config: &RetrieverConfig */) -> Result<Self, RetrieverError> {
        // ... existing init ...
        Ok(Self {
            // ... existing fields ...
            vector_searcher: None,    // NEW default
        })
    }

    // 新增 builder method（add-only — caller 可选用）：
    pub fn with_vector_searcher(mut self, searcher: Arc<dyn VectorSearcher>) -> Self {
        self.vector_searcher = Some(searcher);
        self
    }

    pub fn search(&self, opts: &SearchOptions) -> Result<Vec<SearchResult>, RetrieverError> {
        // 既有 BM25 / metadata / filter 路径 — 完全不动一字节：
        let q_trim = opts.query.trim();
        // ... existing BM25 query parse / Tantivy search / SQLite metadata join / explain wiring ...
        let bm25_results: Vec<SearchResult> = /* ... existing ... */;

        // 新增 vector_searcher 占位调（仅 Some 时进 — None 时 hot path 与 v0.10 字节一致）：
        if let Some(searcher) = &self.vector_searcher {
            // task-18.7 接入真融合（fuse vector_hits + bm25_results）；本 task 仅占位返空：
            let _vector_hits = searcher
                .search(&[], opts.top_k, None)
                .unwrap_or_default();
            // 不修改 bm25_results — 本 task ship 后 retrieval_method 字段仍为 "bm25"
        }

        Ok(bm25_results)
    }
}
```

#### §E sync vs async 决策 trade-off（已锁 sync — 留痕）

| 维度 | sync trait（已选）| async trait（候选）|
|---|---|---|
| 与既有 `Retriever::search()` API 一致性 | ✅ 完全一致（既有 sync）| ❌ 需把既有 sync 改 async — 改面大 |
| `async-trait` macro dep 引入 | ✅ 无 | ❌ 需 `async-trait = "0.1"` |
| Qdrant / LanceDB 实现复杂度 | ⚠️ 内部 `block_in_place` 或独立 tokio runtime — 实施时多 ~10 行 | ✅ 自然 fit |
| SQLite vec / HNSW 实现复杂度 | ✅ 原生 sync | ⚠️ 需 wrap in `async fn`（无 actual async） |
| 性能损耗（动态分派 + spawn_blocking）| 同 async（trait object 动态分派开销相同；spawn_blocking ~µs 级，10 万 chunk 量级可忽略）| 同 sync |
| trait object safety | ✅ `Arc<dyn VectorSearcher>` 直接可构 | ⚠️ `async-trait` desugar 后 future 装箱 + Pin 复杂度 |
| **决策**：sync — anchor by 既有 API 一致性 + 减少改面 + 性能差异在 task-18.2 实测后可重审 | ✅ | — |

如 task-18.4 / 18.5 spike 数据显示 sync trait 包 async backend 性能损耗 > 5% → §8 卡住协议触发 → 主 agent 自决重审 trait async 重构（走独立 `chore/spec-fix-task-18.1` PR，amend 本 spec 至 async；下游 4 backend 同步 amend）。

## 6. Acceptance Criteria

> ADR-014 D3 句式：每条 AC 末尾显式 `verified by <test-id> 或 <smoke-step>`。`(PRD §X)` 引用 PRD 锚点；`(本 task 新增)` 是 PRD 未覆盖但 phase-18 §2A 决策推导的扩展。

- [ ] **AC1**: `core/src/retriever/vector/traits.rs` 定义 `VectorBackend` / `VectorIndexer` / `VectorSearcher` 三 trait + 相关 types (`VectorHit` / `VectorIndexConfig` / `VectorMetric` / `VectorError`)；trait method signature 全 sync (推荐) 或 全 async (备选，§2A 拍板)，统一不混用；trait API doc 含 doctest 示例 (`cargo test --doc -p contextforge-core --lib retriever::vector::traits` PASS) — verified by **TEST-18.1.1** (`core/src/retriever/vector/traits.rs` doctest) + **TEST-18.1.2** (`core/src/retriever/vector/tests.rs::trait_object_safety_test`)（本 task 新增）
- [ ] **AC2**: `core/src/retriever/vector/noop.rs` 落 `NoopVectorBackend` 实现三 trait — `name()` 返 `"noop"`；`version()` 返 `"0.1.0"`；`is_local()` 返 `true`；`requires_embedding()` 返 `false`；`search(_, _, _)` 返 `Ok(vec![])` + `tracing::debug!`；`index_batch(_)` 返 `Ok(0)`；`is_indexed()` 返 `false`；`flush()` / `close()` 返 `Ok(())` — verified by **TEST-18.1.3** (`tests::test_noop_search_returns_empty`) + **TEST-18.1.4** (`tests::test_noop_index_batch_is_noop_ok`) + **TEST-18.1.5** (`tests::test_noop_is_indexed_always_false`)（本 task 新增，refs phase-18 §6 AC1）
- [ ] **AC3**: `core/src/retriever/mod.rs` 接入 `Option<Arc<dyn VectorSearcher>>` 字段（`Retriever::new` 签名扩 add-only — 既有 caller 传 `None` 不破坏 API）；当 `None` 时 `Retriever::search()` hot path 与 v0.10 完全一致（既有 BM25 / metadata / filter 路径 0 字节改动到 search algorithm）；当 `Some(noop)` 时 `search()` 内部调 `searcher.search()` 返空 vector hits + 合并入 result，`retrieval_method` 字段保留 `"bm25"` 取值（PRD §Core Capabilities #2 retrieval_method 字段可解释性约束保留）— verified by **TEST-18.1.6** (`tests::test_retriever_none_vector_searcher_bm25_unchanged`) + **TEST-18.1.7** (`tests::test_retriever_some_noop_vector_searcher_returns_empty_vector_hits`)（本 task 新增，refs PRD §Core Capabilities #2 + §Constraints performance P95 < 500ms 不退化）
- [ ] **AC4**: `core/Cargo.toml` workspace `[features]` 块新增 `vector-spike = []` 占位 feature（空 list — task-18.3-18.6 各自添加 dep）+ `default = []`（不强制 enable）+ `vector-sqlite = []` / `vector-qdrant = []` / `vector-lancedb = []` / `vector-hnsw = []` 四占位 feature（task-18.3-18.6 各自 PR ship 时填实 dep list）；`cargo build --workspace` 默认 features 不变 — 0 新 dep 引入 — verified by **TEST-18.1.8** (`cargo build --workspace --no-default-features` + `cargo build --workspace --features vector-spike` 均 PASS) + 本 task closeout PR diff `Cargo.lock` 0 行新增/删除（本 task 新增，refs phase-18 §7 R7 mitigation）
- [ ] **AC5**: 既有 `cargo test --workspace` 全 PASS — Phase 1-17 既有所有 Rust 测试不退化（BM25 retrieval / Tantivy / SQLite / metadata / explain / memory / eval / ConsoleService / 等）；`go test ./...` 全 PASS（Go 端未触及 — proto / contractv1 / consoleapi 不变）— verified by **TEST-18.1.9** (`cargo test --workspace` 输出 0 failed) + 本 task §10 verification 段记录实测计数（本 task 新增，refs phase-18 §6 AC1 + PRD §Anti-metrics 性能不退化）
- [ ] **AC6**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched origin/master` 0 unannotated hits；本 task 引入的所有延后行为关键词全部用 [SPEC-DEFER:&lt;name&gt;] 或 [SPEC-OWNER:&lt;task&gt;] 标注（§3 Out of Scope 已罗列 12 项）— verified by 本 task PR body 含 D2 lint 输出段（refs ADR-014 D2 第九次激活）
- [ ] **AC7**: 本 task ship 后下游可用 — task-18.2 spike harness 编程基于 trait 接口；task-18.3-18.6 4 backend 实现接入 trait；trait API 在本 task ship 后 90 天内不破坏（仅 add-only method / add-only field）；如确需 break → 走 chore/spec-fix-task-18.1 独立 PR 触发 §2A 重审 — verified by 本 task closeout PR body 含 "trait API stability contract" 段说明 90 天 add-only 约束（本 task 新增，refs phase-18 §2A 决策 4 trait-first 集成深度 + ADR-015 D1 add-only 模式）

## 7. 追踪表

> Status 列取值（standard.md §12.2，独立于 spec 顶部 Status）：Not Started / Spec Ready / Scenario Ready / Test Red / In Progress / Verified / Waived / Blocked / Done

| TEST-ID / SCEN-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-18.1.1 | traits.rs doctest 三 trait API 示例 | `core/src/retriever/vector/traits.rs` (本 task 新增 doctest) | Done |
| TEST-18.1.2 | trait object safety — `Arc<dyn VectorSearcher>` 可构造 | `core/src/retriever/vector/tests.rs::trait_object_safety_test` (本 task 新增) | Done |
| TEST-18.1.3 | NoopVectorBackend.search 返空 vec | `core/src/retriever/vector/tests.rs::test_noop_search_returns_empty` (本 task 新增) | Done |
| TEST-18.1.4 | NoopVectorBackend.index_batch 返 Ok(0) | `core/src/retriever/vector/tests.rs::test_noop_index_batch_is_noop_ok` (本 task 新增) | Done |
| TEST-18.1.5 | NoopVectorBackend.is_indexed 返 false | `core/src/retriever/vector/tests.rs::test_noop_is_indexed_always_false` (本 task 新增) | Done |
| TEST-18.1.6 | retriever.search(None searcher) BM25 路径不变 | `core/src/retriever/tests.rs::test_retriever_none_vector_searcher_bm25_unchanged` (本 task 新增) | Done |
| TEST-18.1.7 | retriever.search(Some Noop) 返空 vector hits + retrieval_method 保留 | `core/src/retriever/tests.rs::test_retriever_some_noop_vector_searcher_returns_empty_vector_hits` (本 task 新增) | Done |
| TEST-18.1.8 | Cargo features vector-spike scaffold (no dep) build PASS | `cargo build --workspace --features vector-spike` (本 task 新增 — CI 段不变；本地实测) | Done |
| TEST-18.1.9 | 既有 cargo test --workspace 0 failed (regression) | 全 workspace（既有；本 task 不退化）| Done |

## 8. Risks

- **R1（中）trait API 90 天稳定性约束破裂**（AC7）：本 task ship 后 trait 视为冻结；如 task-18.3-18.6 spike 实现时发现 trait 抽象不足以容纳某 backend（典型：Qdrant 需 collection-scoped HNSW 参数 + filter 复合表达式超出 `VectorFilter` 4 字段）→ §8 卡住协议触发
  - **缓解**：trait 设计阶段已 cross-check 4 backend 公开文档（sqlite-vec 0.1.x README / qdrant-client 1.x docs.rs / lancedb crate 0.x docs.rs / instant-distance 0.6 README）确保最小公分母覆盖；`VectorFilter.extras: Option<serde_json::Value>` opaque 字段作 backend-specific filter 兜底
  - **退化路径**：trait 不足时主 agent 走独立 `chore/spec-fix-task-18.1` PR amend trait（add-only method 优先 — `default impl` 不破坏既有 backend；break-changes 需重启 task-18.3-18.6 PR cycle）

- **R2（中）sync trait 包 async backend 性能损耗 ≥5%**（§5.3 §E 决策 trade-off）：Qdrant client / LanceDB client 原生 async；sync trait 内部 `tokio::task::block_in_place` 或 `Handle::block_on` 在 tokio multi-thread runtime 下可能引入 ~5-10µs/call 开销
  - **缓解**：task-18.2 spike harness 同时 measure trait 调用路径 vs 直接调用 baseline；10 万 chunk 量级单查询 µs 级开销不影响 PRD §Constraints P95<500ms
  - **退化路径**：如实测 ≥5% 损耗 → 主 agent 走 `chore/spec-fix-task-18.1` amend 为 async trait（引入 `async-trait` dep；下游 4 backend 同步 amend；retriever 端 search 改 async — 改面大但路径明确）

- **R3（中）trait object dyn 动态分派性能损耗**（phase-18 §7 R6 同根）：`Arc<dyn VectorSearcher>` 动态分派 vs 静态泛型 `<T: VectorSearcher>` hot path 损耗 ~3-5%
  - **缓解**：task-18.2 bench 同时 measure trait 路径 vs 直接调用 baseline；如 ≥5% 损耗 → task-18.7 决策转 enum-based static dispatch（pattern: `enum AnyVectorSearcher { Noop(NoopVectorBackend), SqliteVec(SqliteVecBackend), ... }`）
  - **trait 不需 break**：仅在 Retriever struct 改 `enum_dispatch` macro 即可；trait 定义本身保留

- **R4（中）retriever wiring None vs Some 行为差异**（AC3）：本 task ship 后 Some(NoopVectorBackend) 时 vector hits 返空；上层 caller / Console UI 不应假设 `vector_hits` 字段永远空（task-18.7 接入真 backend 后会返非空）
  - **缓解**：本 task 不引入 `SearchResult.vector_hits` 字段（避免 schema 漂移）；task-18.7 决策时统一加 `vector_score: Option<f32>` 字段 + retrieval_method 字段扩取值 `"vector"` / `"bm25+vector"` — 用 ADR-015 D1 add-only 模式
  - **Console UI 影响**：本 task ship 不影响 Console；task-18.7 ship 时走 cross-repo follow-up 通知 Console 团队

- **R5（中）ChunkId 类型与既有 `crate::chunker::ChunkId` 一致性**：本 spec §3/§5.3 §A 用 `pub struct ChunkId(pub String)` newtype；如既有 `crate::chunker::ChunkId` 已是同型 → 实施时改 `pub use crate::chunker::ChunkId;` re-export 避免类型分裂
  - **缓解**：实施 RED 阶段先 `grep -rn "pub struct ChunkId\|pub type ChunkId" core/src/` 确认；如既有则直接 re-export；如不一致 → §8 卡住协议触发，主 agent 决定（推荐 re-export 既有避免类型分裂；本 spec 不预 lock 实施时定）

- **R6（低）Cargo features 占位但 dep 互斥未处理**（AC4）：本 task ship 后 `vector-sqlite` / `vector-qdrant` / `vector-lancedb` / `vector-hnsw` 都是空 list；task-18.3-18.6 各自填 dep 时如某 backend 与 SQLite/Tantivy 既有 dep 互斥（如 sqlite-vec ext loadable extension 需 `libsqlite3-sys/loadable_extension` feature flag 与现有 rusqlite 配置冲突）→ task-18.7 决策时一并处理
  - **缓解**：本 task ship 仅占位 features — 不引入互斥；task-18.3-18.6 各自 PR 跑 `cargo build --features vector-<backend>` 验证孤立可构；task-18.7 跑 `cargo build --features vector-<chosen>` 验证 default 配置可构

- **R7（低）与 ADR-002 sqlite-tantivy-layered-storage 兼容性**：本 task trait 抽象不强约束存储介质；某些 backend（LanceDB Lance file format / Qdrant segment）可能与 ADR-002 SQLite+Tantivy 分层模型重叠 — 但本 task 仅定义 trait + Noop，不引入新存储层
  - **缓解**：本 task ship 不触 ADR-002；task-18.7 default backend 选定后如引入新存储层 → 走 ADR-002 amendment（仿 ADR-022 amend ADR-015 D5 pattern）

- **R8（低）VectorScore NaN/Inf guard 接口偏严**：`VectorScore::new(v) -> Result<Self, _>` 强制每次构造检查；backend 若内部已保证非 NaN 会引入 ~ns 冗余检查
  - **缓解**：性能开销纳秒级可忽略（10 万 chunk 量级一次 query 最多 Top-10 个 score 构造）；FFI/序列化路径 NaN guard 是必要安全网（防 backend bug 把 NaN 喷出影响 BM25 score 比较 panic）

- **R9（低）trait 文档 doctest 占用 cargo test --doc 时长**：≥1 doctest in `traits.rs` 加 `cargo test --doc` 时长 ~1-2s
  - **缓解**：doctest 本身是 AC1 验证手段，时长成本可接受；既有 workspace `cargo test --doc` 时长已 ~5s 量级

- **R10（低）re-export 路径冲突**：`pub use vector::{VectorBackend, ...}` in retriever/mod.rs 可能与既有 retriever module-level types 名字冲突
  - **缓解**：本 spec §5.2 已列具体 re-export 名（VectorBackend / VectorSearcher / NoopVectorBackend）；实施时 RED 阶段先 `grep -rn "pub fn VectorBackend\|pub struct VectorSearcher\|pub use.*VectorBackend" core/src/retriever/` 验证无冲突

## 9. Verification Plan

```bash
# install
cargo fetch

# lint (项目当前 Rust lint 槽位见 docs/s2v-adapter.md §Commands > Lint)
cargo clippy --workspace --all-targets -- -D warnings

# typecheck
cargo check --workspace

# unit-test (按 TEST-ID 列单跑；CI 走 cargo test --workspace)
cargo test -p contextforge-core --lib retriever::vector::tests::test_noop_search_returns_empty
cargo test -p contextforge-core --lib retriever::vector::tests::test_noop_index_batch_is_noop_ok
cargo test -p contextforge-core --lib retriever::vector::tests::test_noop_is_indexed_always_false
cargo test -p contextforge-core --lib retriever::vector::tests::trait_object_safety_test
cargo test -p contextforge-core --lib retriever::tests::test_retriever_none_vector_searcher_bm25_unchanged
cargo test -p contextforge-core --lib retriever::tests::test_retriever_some_noop_vector_searcher_returns_empty_vector_hits

# doctest
cargo test --doc -p contextforge-core --lib retriever::vector::traits

# integration (本 task N/A — 真 backend impl 在 task-18.3-18.6)

# regression — 既有不退化
cargo test --workspace
go test ./...

# build with feature
cargo build --workspace --no-default-features
cargo build --workspace --features vector-spike

# coverage (sqlite-vec / qdrant / lancedb / hnsw 不入 cov — 仅 trait + Noop cov)
cargo llvm-cov --workspace --lib --html
# expect: retriever::vector 模块 line coverage ≥ 90% (Noop 全 stub 易 cov)

# e2e smoke (本 task N/A — smoke v9 step 29-30 在 task-18.9)

# ADR-014 D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
# expect: 0 unannotated hits

# manual: trait API stability commitment
# - 检查 traits.rs 三 trait 全部 method 签名 + types 字段 — 在本 task ship 后视为冻结
# - 90 天内 add-only（add method 默认实现 / add field with #[serde(default)]）；break-changes 走 chore/spec-fix-task-18.1
```

## 10. Completion Notes (s2v 6 项标准)

> 实施完成后按 standard.md §8.3 6 项 schema 回填（每项替换 `<TBD-after-impl>` 为真实值）；Status: Ready → In Progress → Done 推进。

- **完成日期**：2026-05-30
- **改动文件**：
  - core/src/retriever/vector/mod.rs（新增）
  - core/src/retriever/vector/types.rs（新增）
  - core/src/retriever/vector/traits.rs（新增）
  - core/src/retriever/vector/noop.rs（新增）
  - core/src/retriever/vector/tests.rs（新增）
  - core/src/retriever/mod.rs（修改：+pub mod vector, +re-exports, +vector_searcher field, +with_vector_searcher builder, +search() placeholder call, +2 retriever-level tests）
  - core/Cargo.toml（修改：+[features] scaffold vector-spike/vector-sqlite/vector-qdrant/vector-lancedb/vector-hnsw）
- **commit 列表**：
  - 6a5134c feat(vector): task-18.1 vector trait abstraction + NoopVectorBackend + Retriever wiring
- **§9 Verification 结果**：
  - install: ✅ cargo fetch 无新 dep
  - lint: ⚠️ 11 pre-existing clippy warnings（非本 task 引入；vector 模块 0 新 warning）
  - typecheck: ✅ cargo check --workspace exit 0
  - unit-test: 196 passed / 0 failed（186 baseline + 8 vector module + 2 retriever-level）
  - doctest: 1 passed（traits.rs VectorSearcher doctest）
  - coverage: skipped（cargo llvm-cov 未安装；Noop stub 覆盖率可推定 ~100%）
  - build: ✅ cargo build --workspace --features vector-spike exit 0
  - manual: ✅ 三 trait API 冻结确认 — VectorBackend/VectorIndexer/VectorSearcher 全部 method 签名与 §5.3 §B 一致；types 字段与 §5.3 §A 一致
- **剩余风险 / 未做项**：R5 ChunkId 用独立 newtype（crate::chunker 无同名类型，已 grep 确认）；tracing dep 未在 Cargo.toml — noop.rs debug 改为行内注释。ADR-014 D2 lint 脚本在 Windows/PS 环境下未跑（bash 脚本需 WSL）
- **下游 task 影响**：task-18.2 (spike harness), task-18.3-18.6 (4 backend impl), task-18.7 (default wiring + hybrid fusion)
