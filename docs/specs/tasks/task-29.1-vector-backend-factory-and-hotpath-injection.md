# Task `29.1`: `vector-backend-factory-and-hotpath-injection — core/src/retriever/vector 新增 select_vector_backend(name, dim) 工厂（仿 embedding/factory.rs::select_provider）+ 把 core/src/server.rs:302/341 硬编码 BruteForceVectorBackend::new() 替换为工厂注入（默认仍 BruteForce 0-dep；qdrant/lancedb feature-gated 否则诚实 Err）+ feature 不连 live server 的 deterministic 工厂契约测试`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 29 (live-vector-recall)
**Dependencies**: task-25.1（`QdrantBackend` 生命周期层 `connect`/`health`/`decide_ensure`，Done）/ task-25.2（`LanceDbBackend` 可构建性 + `LanceIndexTuning`/`LanceAnnIndex` 参数契约，Done）/ task-18.1（`VectorSearcher`/`VectorBackend`/`VectorIndexer` 三 trait freeze + `VectorError`）/ task-22.1（`core/src/embedding/factory.rs::select_provider` 工厂范式——本 task 镜像之）/ ADR-034 D1（vector backend factory + server.rs 热路径注入，本 task 即其原文实现）/ ADR-030 D1（生产 backend 生命周期层）/ ADR-023 D5（默认 0-dep BruteForce 基线）/ ADR-004（local-first 0-dep）/ ADR-008（core-library-selection，依赖变更 add-only）/ ADR-013（禁伪造 live-server 通过 / 召回数值）/ ADR-014 D1-D5（第二十次激活）

## 1. Background

`core/src/server.rs` 的语义路径（`server.rs:341`）与 hybrid 路径（`server.rs:302`）当前都**硬编码** `let backend = Arc::new(BruteForceVectorBackend::new());`——无论部署是否配置了 qdrant / lancedb，热路径永远只用 0-dep 的 BruteForceVectorBackend。embedding 侧已在 task-22.1 经 `select_provider`（`core/src/embedding/factory.rs:27-30`）工厂化（`server.rs:339` 调 `select_provider("deterministic", 0)`），vector 侧却仍是 Phase 19 遗留的硬编码常量，**无对称工厂**。

Phase 25 已把 qdrant（task-25.1：`connect`/`health`/`decide_ensure` 契约层）与 lancedb（task-25.2：可构建性 + `LanceIndexTuning` 参数契约）推到「backend 生命周期 / 可构建性先行」状态，并把「把 qdrant/lancedb 接进 `core/src/server.rs` 语义热路径」明确列为 `[SPEC-DEFER:phase-future.vector-retrieval-integration]`（phase-25 spec 第 44 行：「backend 生命周期/可构建性先行，热路径接入后续」）。本 task 兑现该延后项的**注入维度**：在 vector 侧补一个与 `select_provider` 对称的 `select_vector_backend(name, dim)` 工厂，把 `server.rs:302/341` 两处硬编码替换为工厂注入。

本 task 是 🟢 deterministic 维度——工厂的「默认名 → BruteForce」「feature 关闭名 → 诚实 Err」「热路径经工厂、默认构建语义+hybrid 仍走 BruteForce」三条契约都可在 CI 不连任何 live server 下单测。真实 qdrant live KNN（task-29.2）与 lancedb real ANN 调参（task-29.3）是后续 task 在本工厂之上的 live / feature 维度。

## 2. Goal

`core/src/retriever/vector`（工厂落地于该模块，如 `factory.rs` 或 `mod.rs`）新增 `select_vector_backend(name: &str, dim: usize) -> Result<Arc<dyn VectorSearcher>, VectorError>`，镜像 `core/src/embedding/factory.rs::select_provider` 的范式：

- `""` / `"brute"` → `BruteForceVectorBackend`（始终可用，0-dep，对应 `server.rs` 现硬编码语义，行为保持）；
- `"qdrant"` → `QdrantBackend`（`#[cfg(feature = "vector-qdrant")]`）；feature 关闭时返回可识别 `VectorError`（明确「需 vector-qdrant feature」，不 panic、不静默 fallback、不伪造成功）；
- `"lancedb"` → `LanceDbBackend`（`#[cfg(feature = "vector-lancedb")]`）；feature 关闭时返回可识别 `VectorError`；
- 其他名 → 可识别 unknown-backend `VectorError`。

把 `core/src/server.rs:302`（hybrid 路径）与 `server.rs:341`（语义路径）的 `Arc::new(BruteForceVectorBackend::new())` 替换为 `select_vector_backend(...)` 调用（默认参数与现硬编码 byte-equivalent，未把 vector backend 配置 plumb 进 server 故默认名）。兑现 `[SPEC-DEFER:phase-future.vector-retrieval-integration]`（phase-25 spec 第 44 行）的注入维度。

pass bar：feature `vector-qdrant` / `vector-lancedb` 下工厂返回对应 backend；feature 关闭下返回诚实 `Err`（非成功）；默认构建（无任何 vector feature）`select_vector_backend("", 0)` 返 BruteForce，`server.rs` 语义+hybrid 路径经工厂仍走 BruteForce、`cargo test --workspace` 不退化、0 新依赖。真实 live KNN / 召回质量诚实延后给 task-29.2/29.3（禁伪造 live-server 通过 / 召回数值，ADR-013）。D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- **新增 `select_vector_backend(name, dim) -> Result<Arc<dyn VectorSearcher>, VectorError>` 工厂**（落地 `core/src/retriever/vector` 模块，工厂函数 + 文档注释 + 与 `select_provider` 对称的分支结构）：`""`/`"brute"` → `BruteForceVectorBackend`（始终可用）；`"qdrant"` → `QdrantBackend`（feature-gated，关闭返诚实 Err）；`"lancedb"` → `LanceDbBackend`（feature-gated，关闭返诚实 Err）；未知名 → 诚实 Err。`dim` 参数沿用 `select_provider` 的契约形态以便后续与 embedder dim 协商（本 task BruteForce 不强约束 dim；保留入参对称性）。
- **修改 `core/src/server.rs:302`（hybrid 路径）+ `server.rs:341`（语义路径）**：把硬编码 `Arc::new(BruteForceVectorBackend::new())` 替换为 `select_vector_backend(...)`（默认名 / dim，与现硬编码 byte-equivalent），error 经现有 `Status::internal` 映射（与 `server.rs:339` 的 `select_provider` 错误映射对称）。既有 `with_vector_searcher`（`core/src/retriever/mod.rs:592-595`）注入点 / `index_chunks_semantic` / `search_semantic` / `search_hybrid` 调用链不动。
- **新增工厂 deterministic 契约测试**（同源 `#[cfg(test)] mod tests`，不连 live server）：(a) `select_vector_backend("", 0)` / `("brute", 0)` 返 BruteForce backend（默认构建即可，无 feature）；(b) feature 关闭时 `select_vector_backend("qdrant", _)` / `("lancedb", _)` 返可识别 `Err`（非成功，不伪造）；feature 开启分支的对应正向断言在该 feature 下单测；(c) `server.rs` 热路径经工厂——默认构建语义+hybrid 仍走 BruteForce、`cargo test --workspace` 不受影响（经 server 集成测试或 retriever 注入链断言）。
- **可选 `core/src/retriever/vector/mod.rs` re-export**：`pub use` 工厂函数（若落地于独立 `factory.rs`），与 `embedding::factory` 的 re-export 形态对称；不改三 trait 签名、不改 `BruteForceVectorBackend` / `QdrantBackend` / `LanceDbBackend` 本体。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **真实 qdrant live server 端到端 connect→ensure-create→upsert→KNN + real-recall harness** [SPEC-OWNER:task-29.2-qdrant-live-knn-and-recall-harness]：本 task 只做工厂 + 注入，live KNN 在 29.2；CI 无 live server 故 29.2 诚实延后（ADR-013）。
- **lancedb real ANN 索引调参（IVF_PQ/HNSW build + 实测召回）+ 多 backend 选择矩阵真实测量** [SPEC-OWNER:task-29.3-lancedb-ann-index-tuning-and-backend-matrix]：本 task 不建真索引、不测召回。
- **把 vector backend 名 / dim 从配置（env / proto）plumb 进 `core/src/server.rs`** [SPEC-DEFER:phase-future.vector-backend-config-plumbing]：本 task 工厂以默认名 byte-equivalent 替换硬编码；从外部配置选 backend 的 plumbing 后续（与 embedding 侧「config 未 plumb 进 server」现状对称）。
- **sqlite-vec backend 接入工厂** [SPEC-DEFER:phase-future.sqlite-vec-factory-arm]：task-23.2 已证 Windows MSVC 可构建但默认 / CI 不构建该 feature；本 task 工厂先覆盖 brute/qdrant/lancedb 三臂，sqlite-vec 臂随 29.3 选择矩阵评估后续。
- **smoke v19 / v0.22.0 closeout / ADR ratify + amendment** [SPEC-OWNER:task-29.4-closeout-v0.22.0]：本 task 交付工厂 + 注入，收口在 29.4。

## 4. Actors

- **主 agent**：实施 + PR 主理（ADR-012 自治）。
- **`core/src/retriever/vector` 工厂（`select_vector_backend`）**：新增，本 task 的核心交付物；镜像 `embedding::factory::select_provider`。
- **`core/src/server.rs` 语义路径（`:341`）+ hybrid 路径（`:302`）**：本 task 把两处硬编码 `BruteForceVectorBackend::new()` 替换为工厂调用。
- **`core/src/embedding/factory.rs::select_provider`（`:27-30`）**：工厂范式参照（分支结构 / feature gate / 诚实 Err / `Arc<dyn _>` 返回类型 / dim 入参）。
- **`BruteForceVectorBackend`（`core/src/retriever/vector/mod.rs:31` re-export）/ `QdrantBackend`（`:40`，feature `vector-qdrant`）/ `LanceDbBackend`（`:43`，feature `vector-lancedb`）**：工厂三分支的目标 backend；本 task 不改其本体。
- **`VectorSearcher` trait（`core/src/retriever/vector/traits.rs:38-46`）**：工厂返回类型 `Arc<dyn VectorSearcher>` 的 trait 面（`search` / `is_indexed`）。
- **下游 task-29.2 / 29.3**：在本工厂之上做 qdrant live KNN（29.2）/ lancedb real ANN + 选择矩阵（29.3）。

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/embedding/factory.rs:27-30`（`select_provider(provider_name, dim) -> Result<Arc<dyn EmbeddingProvider>, EmbeddingError>` 工厂签名 + `:31-48` 分支结构：`""`/`"deterministic"` 默认臂、feature-gated 臂 + `#[cfg(not(feature=...))]` 诚实 Err 臂——本 task `select_vector_backend` 镜像此范式）
- `core/src/server.rs:302`（hybrid 路径 `let backend = Arc::new(BruteForceVectorBackend::new());`——本 task 替换点）+ `server.rs:341`（语义路径同款硬编码——本 task 替换点）+ `server.rs:339`（`select_provider("deterministic", 0)` 既有工厂调用 + `Status::internal` 错误映射，对称参照）
- `core/src/retriever/vector/traits.rs:38-46`（`VectorSearcher::search(query_vec, k, filter)` + `is_indexed()`——工厂返回 `Arc<dyn VectorSearcher>` 的 trait 面）+ `traits.rs:11-25`（`VectorBackend` / `VectorIndexer` 基 trait，工厂不改其签名）
- `core/src/retriever/mod.rs:592-595`（`with_vector_searcher(searcher: Arc<dyn VectorSearcher>)` 注入点——工厂产物经此注入，调用链不动）
- `core/src/retriever/vector/mod.rs:31`（`pub use brute_force::BruteForceVectorBackend;`）+ `:40`（`#[cfg(feature="vector-qdrant")] pub use qdrant::QdrantBackend;`）+ `:43`（`#[cfg(feature="vector-lancedb")] pub use lance_db::LanceDbBackend;`）——工厂三分支引用的 re-export
- `core/src/retriever/vector/types.rs::VectorError`（工厂诚实 Err 的承载类型——`Other` / `Backend` 变体）
- `core/Cargo.toml:119`（`vector-qdrant = ["dep:qdrant-client"]`）+ `:120`（`vector-lancedb = ["dep:lancedb", ...]`）——feature gate 名核实
- `docs/decisions/adr-034-production-vector-live-recall.md` D1（vector backend factory + server.rs 热路径注入——本 task 即其原文实现）+ `docs/specs/phases/phase-25-production-vector-backend.md` 第 44 行（`[SPEC-DEFER:phase-future.vector-retrieval-integration]` 延后来源）

### 5.2 关键设计 — vector backend 工厂 + server.rs 热路径注入（镜像 select_provider）

- **工厂签名对称**：`select_vector_backend(name: &str, dim: usize) -> Result<Arc<dyn VectorSearcher>, VectorError>` 逐项镜像 `select_provider(provider_name: &str, dim: usize) -> Result<Arc<dyn EmbeddingProvider>, EmbeddingError>`——同样的 `(name, dim)` 入参、`Result<Arc<dyn _Trait>, _Error>` 出参、同样的 `match name { 默认臂 => ..., feature 臂 => #[cfg] / #[cfg(not)] 诚实 Err, _ => unknown Err }` 分支骨架。`dim` 入参在本 task BruteForce 臂不强约束（BruteForce 对任意 dim 工作）；保留对称性以便后续与 embedder dim 协商（与 `select_provider` 的 `DimMismatch` 形态呼应，本 task 不实现协商）。
- **默认臂 byte-equivalent**：`""` / `"brute"` → `Arc::new(BruteForceVectorBackend::new())`——与 `server.rs:302/341` 现硬编码逐字节等价（替换是行为保持的，不改默认构建召回路径）。
- **feature-gated 诚实 Err**：`"qdrant"` 臂在 `#[cfg(feature = "vector-qdrant")]` 下构造 `QdrantBackend`、在 `#[cfg(not(feature = "vector-qdrant"))]` 下 `return Err(VectorError::Other("vector backend 'qdrant' requires the vector-qdrant feature".into()))`（仿 `factory.rs:42-47` 的 fastembed 诚实 Err 臂）；`"lancedb"` 臂同构。不 panic、不静默 fallback 到 BruteForce、不伪造成功（ADR-013：feature 关闭即 Err，调用方据此 gate）。
- **server.rs 热路径替换**：`server.rs:302` / `:341` 改为 `let backend = select_vector_backend("", 0).map_err(|e| Status::internal(format!("vector backend: {}", e)))?;`（默认名，与 `server.rs:339` 的 `select_provider` 错误映射风格对称）。backend 配置未 plumb 进 server（与 embedding 侧现状一致）故默认名——从配置选 backend 的 plumbing 是 `[SPEC-DEFER:phase-future.vector-backend-config-plumbing]`。
- **ADR-013 verifiability**：工厂契约（默认臂返 BruteForce / feature 关闭返诚实 Err / 热路径经工厂默认构建仍 BruteForce）是 deterministic、CI 可验证（不连任何 live server）的 🟢 项；真实 qdrant live KNN（29.2）/ lancedb real ANN 召回（29.3）是 live / feature 维度，不在本 task 预判数值、不伪造 live-server 通过。

### 5.3 不变量

- **默认构建逐字节不变（ADR-004 / ADR-023 D5）**：无任何 vector feature 时，`select_vector_backend("", 0)` 返 `BruteForceVectorBackend`，与替换前 `BruteForceVectorBackend::new()` 行为等价；0 新依赖（`core/Cargo.toml` / `Cargo.lock` 不改）；qdrant / lancedb 臂在默认构建下经 `#[cfg(not(feature))]` 编为诚实 Err，不引入供应链面。
- **工厂确定性**：given 相同 `(name, dim)` + 相同 feature 集 → 相同结果（默认臂返同类 backend / 关闭臂返同类 Err），可单测。
- **诚实 Err 不静默**：feature 关闭 / 未知名时返回可识别 `VectorError`，调用方可据此 gate；绝不静默 fallback 到 BruteForce、绝不伪造「backend 构造成功」（ADR-013）。
- **不改三 trait 签名**：`VectorBackend` / `VectorIndexer` / `VectorSearcher`（`traits.rs:11-46`）签名不动；工厂是新增自由函数，返回既有 `Arc<dyn VectorSearcher>`，经既有 `with_vector_searcher`（`mod.rs:592-595`）注入。
- **不改 backend 本体**：`BruteForceVectorBackend` / `QdrantBackend` / `LanceDbBackend` 实现不动；本 task 只新增工厂 + 替换 server.rs 两处构造点。
- **server.rs 调用链不退化**：`enumerate_chunks` / `index_chunks_semantic` / `search_semantic` / `search_hybrid` 链路与 retrieval_method / vector_score / hybrid_score / embedding_provider 输出字段不变（替换的是 backend 来源，非链路）。

## 6. Acceptance Criteria

- [x] AC1（默认/空名 → BruteForce）: feature 关闭（默认构建）下 `select_vector_backend("", 0)` 与 `select_vector_backend("brute", 0)` 均返回 `BruteForceVectorBackend`（`Arc<dyn VectorStore>`，IS-A `VectorSearcher`，始终可用，0-dep），与 `server.rs:302/341` 替换前 byte-equivalent — verified by TEST-29.1.1（deterministic，不连 server，PASS）
- [x] AC2（feature 关闭 → 诚实 Err）: feature `vector-qdrant` / `vector-lancedb` 关闭时 `select_vector_backend("qdrant", _)` / `("lancedb", _)` 返回可识别 `VectorError`（明确「需对应 feature」），不 panic、不静默 fallback、不伪造成功；未知名同返诚实 Err（ADR-013） — verified by TEST-29.1.2（deterministic，不连 server，PASS）
- [x] AC3（server.rs 热路径经工厂 + 不退化）: `server.rs:302`（hybrid）+ `server.rs:341`（语义）改为 `select_vector_backend("", 0)` 注入；默认构建（无 vector feature）语义+hybrid 路径仍经工厂走 BruteForce，retrieval_method / vector_score / hybrid_score 输出不变；`cargo test --workspace` 191 lib + 全集成 0 failed、0 新依赖 — verified by TEST-29.1.3（deterministic，不连 server，PASS）
- [x] AC4（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by TEST-29.1.4 + §10 记录（CI spec-lint 权威）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-29.1.1 | `select_vector_backend("", 0)` / `("brute", 0)` 返 BruteForce（默认构建，无 feature，deterministic 不连 server） | `core/src/retriever/vector/factory.rs`（同源 `mod tests`） | Done (PASS) |
| TEST-29.1.2 | feature 关闭时 `("qdrant", _)` / `("lancedb", _)` + 未知名返可识别 `VectorError`（非成功，不 panic，ADR-013） | `core/src/retriever/vector/factory.rs`（同源 `mod tests`） | Done (PASS) |
| TEST-29.1.3 | `server.rs:302/341` 经 `select_vector_backend` 注入；默认构建语义+hybrid 走 BruteForce 不退化 + `cargo test --workspace` 不受影响 | `core/src/server.rs` + `cargo test --workspace` | Done (PASS) |
| TEST-29.1.4 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威） | `scripts/spec_drift_lint.sh` | Done (PASS) |

## 8. Risks

- **R1（中）工厂落地位置与 re-export 形态**：工厂可落 `core/src/retriever/vector/factory.rs`（与 `embedding/factory.rs` 对称）或 `mod.rs` 内自由函数；feature-gated 臂的 `#[cfg]` 与 `pub use` 需与现有 vector 模块 re-export（`mod.rs:31/40/43`）协调。
  - **缓解**：镜像 `core/src/embedding/factory.rs` 既有形态（独立 `factory.rs` + `select_provider` 自由函数 + feature gate 在函数内 `#[cfg]` 块）；`pub use` 工厂函数与 `embedding::factory` 对称；feature-gated backend 的引用走既有 `mod.rs` re-export，避免重复 `#[cfg]` 路径。
- **R2（中）feature-gated 臂在默认构建的编译形态**：`"qdrant"` / `"lancedb"` 臂在 `#[cfg(not(feature))]` 下须编为诚实 Err 且不引用未编译的 `QdrantBackend` / `LanceDbBackend` 符号（否则默认构建 break）。
  - **缓解**：仿 `factory.rs:37-48` 的 fastembed 臂——正向构造在 `#[cfg(feature)]` 块、Err 在 `#[cfg(not(feature))]` 块且 `return Err(...)` 不引用 feature-gated 符号；`cargo test --workspace`（默认）+ `--features vector-qdrant` + `--features vector-lancedb` 三态各自编译 + 测试。
- **R3（低）server.rs 错误映射与既有风格不一致**：替换点需与 `server.rs:339` 的 `select_provider` 错误映射（`Status::internal`）对称，不引入新错误语义。
  - **缓解**：复用 `server.rs:339-340` 的 `.map_err(|e| Status::internal(format!(...)))?` 形态；backend 工厂 Err → `Status::internal`，与 embedder 工厂 Err 映射一致；既有语义/hybrid 路径返回 shape 不变。

## 9. Verification Plan

```bash
# 1. AC1+AC2 — 工厂契约（默认构建：默认臂返 BruteForce + 关闭臂返诚实 Err，不连 server）
cargo test -p contextforge-core retriever::vector::factory   # 或工厂落地模块路径

# 2. AC2 feature 开启正向臂 + 不退化（feature 下工厂返对应 backend）
cargo test -p contextforge-core --features vector-qdrant retriever::vector
# lancedb feature 构建 caveat（承 task-25.2）：广义 cargo test 可能 rustc ICE → 用 cargo build + --lib scoped
cargo build -p contextforge-core --features vector-lancedb
cargo test -p contextforge-core --features vector-lancedb --lib retriever::vector

# 3. AC3 — server.rs 热路径经工厂 + 默认构建语义+hybrid 不退化 + 0 新依赖
cargo test --workspace

# 4. AC3 — Go 不退化（本 PR 零 Go delta）
go test ./...

# 5. AC4 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **outward-facing 红线**：本 task 纯内部工厂 + 注入（无 tag / release / 网络面）；真实 qdrant live KNN（task-29.2）/ lancedb real ANN 召回（task-29.3）是 live / feature 维度，本 task 不触发、不预判数值、不伪造 live-server 通过（ADR-013）。

## 10. Completion Notes (s2v 6 项标准)

- **Status**: Done。
- **实际改动文件**：
  - `core/src/retriever/vector/factory.rs`（新增）— `select_vector_backend(name, dim) -> Result<Arc<dyn VectorStore>, VectorError>`：`""`/`"brute"` → `BruteForceVectorBackend`；`"qdrant"` feature-gated（`#[cfg(feature="vector-qdrant")]` 构造 `QdrantBackend::new()?`，关闭返诚实 Err「requires the vector-qdrant feature」）；`"lancedb"` 同构；未知名 `unknown vector backend {name:?}` Err。+ 同源 feature-aware deterministic `#[cfg(test)] mod tests`（TEST-29.1.1 默认臂 / TEST-29.1.2 关闭臂 + 未知名 Err + feature-on 正向臂）。
  - `core/src/retriever/vector/traits.rs`（修改，add-only）— 新增组合 trait `VectorStore: VectorIndexer + VectorSearcher` + blanket `impl<T: VectorIndexer + VectorSearcher> VectorStore for T {}`；三 base trait 签名不动（ADR-014 D5）。
  - `core/src/retriever/vector/mod.rs`（修改）— `pub mod factory;` + `pub use factory::select_vector_backend;` + `pub use traits::...VectorStore`，与 `embedding::factory` re-export 对称。
  - `core/src/server.rs`（修改）— import `BruteForceVectorBackend` → `select_vector_backend`；`:302`（hybrid）+ `:341`（语义）的 `Arc::new(BruteForceVectorBackend::new())` 替换为 `select_vector_backend("", 0).map_err(|e| Status::internal(...))?`（默认名，byte-equivalent）。`backend.clone()` upcast `Arc<dyn VectorStore>`→`Arc<dyn VectorSearcher>`（喂 `with_vector_searcher`）、`backend.as_ref()` upcast `&dyn VectorStore`→`&dyn VectorIndexer`（喂 `index_chunks_semantic`），均经 rustc 1.86+ trait-upcasting coercion 在调用点自动完成。
- **关键设计决断（主 agent 自治，ADR-012）**：spec 原拟工厂返 `Arc<dyn VectorSearcher>`；但 `server.rs` 热路径用**同一** backend 对象既 `index_chunks_semantic(&dyn VectorIndexer)` 又 `with_vector_searcher(Arc<dyn VectorSearcher>)`，单一 `VectorSearcher` trait 对象无法喂 indexer。最小且正确解：新增 add-only 组合 trait `VectorStore`（IS-A `VectorSearcher`，故契约真超集，不退化）。三 base trait 签名零改动。
- **§9 Verification 实测证据**：
  - unit-test：`cargo test -p contextforge-core --lib retriever::vector::factory` → **4 passed; 0 failed**（TEST-29.1.1/29.1.2 默认构建工厂契约 + 未知名 Err，不连 server）。
  - 不退化：`cargo test --workspace` → 191 lib + 全集成 binary **0 failed**（默认无 vector feature，semantic+hybrid 经工厂走 BruteForce）；0 新依赖（`Cargo.toml`/`Cargo.lock` 未改）。
  - lint：`cargo clippy --workspace --all-targets -- -D warnings` → Finished 0 warning；`bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中（CI spec-lint gate 权威）。
  - Go：零 Go delta（`go test`/`go vet`/`gofmt` 不受影响，CI 权威）。
  - RED→GREEN：commit 1 RED（factory 初版返 Err + tests，4 failed）→ commit 2 GREEN（实现 + server.rs 注入，4 passed + workspace 0 failed）。
  - feature 正向臂（`--features vector-qdrant` qdrant 臂 / `--features vector-lancedb` lancedb 臂）经 `#[cfg(feature)]` 测试覆盖；qdrant 真实 live KNN → task-29.2，lancedb 真实 ANN 索引 → task-29.3（本 task 不连 live server、不预判召回数值，ADR-013）。
