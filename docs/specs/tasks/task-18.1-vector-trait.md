# Task `18.1`: `vector-trait — core/src/retriever/vector/{mod,traits,noop}.rs 三 trait 冻结 + NoopVectorBackend 占位实现 + retriever wiring + Cargo workspace vector-spike feature flag`

**Status**: Draft

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

> 详细文件路径 / 函数签名 / trait method signature / NoopVectorBackend 内部 logic 待 §2A 业务承诺审核期填实（按 phase-18 §3 涉及模块/task-18.1 推导）。下面仅列出 In Scope 边界 — 严禁在实施时超出本边界（如新增真 backend dep）。

<TBD-by-user>

<!-- 渲染规则（§2A 审核期填）：
     - 三 trait method signature 完整列出（Rust trait 块的伪代码或文字描述）
       - VectorBackend: 静态属性 — name() / version() / is_local() / requires_embedding()
       - VectorIndexer: 写路径 — open(config) / index_batch(chunks) / delete(ids) / flush() / close()
       - VectorSearcher: 读路径 — search(query_vec, k, filter) -> Vec<VectorHit> / is_indexed() -> bool
     - 相关 types: VectorHit { chunk_id, score, metadata } / VectorIndexConfig { dim, metric, persistence_path } / VectorScore (f32 newtype with NaN guard)
     - NoopVectorBackend 实现细节: search() -> empty Vec + tracing::debug!; index_batch() -> Ok(0); is_indexed() -> false; flush() -> Ok(()); 等
     - retriever wiring 细节: Retriever::new 接受 Option<Arc<dyn VectorSearcher>> 字段；search() hot path None 时既有 path 不变；Some 时占位调 searcher.search() 返空但 retrieval_method 字段标 "bm25+vector(noop)"
     - Cargo.toml features 块: [features] default = [] / vector-spike = [] (空 feature 占位，task-18.3-18.6 各自添加 dep) / vector-sqlite = ["dep:sqlite-vec"] (示例占位)
     - 模块组织: core/src/retriever/vector/mod.rs (pub mod traits/noop/types) + core/src/retriever/vector/traits.rs + core/src/retriever/vector/types.rs + core/src/retriever/vector/noop.rs + core/src/retriever/vector/tests.rs (或 module-level #[cfg(test)] mod tests)
-->

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

<TBD-by-user>

<!-- 渲染规则（§2A 审核期填）：
     - Rust: std::sync::Arc / std::fmt::Debug / tracing::{debug, info} / serde::{Serialize, Deserialize} / thiserror::Error / async_trait::async_trait (如 trait 含 async method)
     - 既有依赖（不新引）: anyhow / Result type / 既有 retriever module types (ChunkID / RetrievalResult / RetrievalScore 等)
     - 不引入: sqlite-vec / qdrant-client / lancedb / hnsw_rs / instant-distance / fastembed (留 task-18.3-18.6)
     - feature gate: #[cfg(feature = "vector-spike")] 用于后续 backend impl，本 task NoopVectorBackend 不 gate（默认可用）
-->

### 5.3 Function Signatures

<TBD-by-user>

<!-- 渲染规则（§2A 审核期填，关键 trait method 列完整 Rust 伪代码）：
     - trait VectorBackend: Send + Sync { fn name(&self) -> &'static str; fn version(&self) -> &'static str; fn is_local(&self) -> bool; fn requires_embedding(&self) -> bool; }
     - trait VectorIndexer: VectorBackend { fn open(&self, config: VectorIndexConfig) -> Result<(), VectorError>; fn index_batch(&self, chunks: &[VectorChunk]) -> Result<usize, VectorError>; fn flush(&self) -> Result<(), VectorError>; fn delete(&self, ids: &[ChunkId]) -> Result<usize, VectorError>; fn close(&self) -> Result<(), VectorError>; }
     - trait VectorSearcher: VectorBackend { fn search(&self, query_vec: &[f32], k: usize, filter: Option<VectorFilter>) -> Result<Vec<VectorHit>, VectorError>; fn is_indexed(&self) -> bool; }
     - struct VectorHit { chunk_id: ChunkId, score: VectorScore, metadata: VectorHitMetadata }
     - struct VectorIndexConfig { dim: usize, metric: VectorMetric, persistence_path: Option<PathBuf>, ... }
     - enum VectorMetric { Cosine, DotProduct, L2 }
     - struct NoopVectorBackend; impl VectorBackend / VectorIndexer / VectorSearcher for NoopVectorBackend { ... }
     - retriever wiring: Retriever::new(config, ..., vector_searcher: Option<Arc<dyn VectorSearcher>>)
     - 注：sync vs async — 决策由 §2A 拍板（如 trait async-by-default 则后续 SQLite/HNSW backend 用 spawn_blocking；如 sync-by-default 则 Qdrant/LanceDB backend 内部 tokio runtime）；推荐 sync trait + 上层 spawn_blocking
-->

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
| TEST-18.1.1 | traits.rs doctest 三 trait API 示例 | `core/src/retriever/vector/traits.rs` (本 task 新增 doctest) | Not Started |
| TEST-18.1.2 | trait object safety — `Arc<dyn VectorSearcher>` 可构造 | `core/src/retriever/vector/tests.rs::trait_object_safety_test` (本 task 新增) | Not Started |
| TEST-18.1.3 | NoopVectorBackend.search 返空 vec | `core/src/retriever/vector/tests.rs::test_noop_search_returns_empty` (本 task 新增) | Not Started |
| TEST-18.1.4 | NoopVectorBackend.index_batch 返 Ok(0) | `core/src/retriever/vector/tests.rs::test_noop_index_batch_is_noop_ok` (本 task 新增) | Not Started |
| TEST-18.1.5 | NoopVectorBackend.is_indexed 返 false | `core/src/retriever/vector/tests.rs::test_noop_is_indexed_always_false` (本 task 新增) | Not Started |
| TEST-18.1.6 | retriever.search(None searcher) BM25 路径不变 | `core/src/retriever/tests.rs::test_retriever_none_vector_searcher_bm25_unchanged` (本 task 新增) | Not Started |
| TEST-18.1.7 | retriever.search(Some Noop) 返空 vector hits + retrieval_method 保留 | `core/src/retriever/tests.rs::test_retriever_some_noop_vector_searcher_returns_empty_vector_hits` (本 task 新增) | Not Started |
| TEST-18.1.8 | Cargo features vector-spike scaffold (no dep) build PASS | `cargo build --workspace --features vector-spike` (本 task 新增 — CI 段不变；本地实测) | Not Started |
| TEST-18.1.9 | 既有 cargo test --workspace 0 failed (regression) | 全 workspace（既有；本 task 不退化）| Not Started |
| SCEN-18.1.1 | <TBD-by-user> spike harness 后续可消费 trait (placeholder — 由 task-18.2 验证) | `bench/` (task-18.2 ship) | Not Started |

## 8. Risks

<TBD-by-user>

<!-- 渲染规则（§2A 审核期填，按 phase-18 §7 R6 + R7 推导 + 本 task 专属风险）：
     - trait 抽象层动态分派性能损耗（phase-18 R6）：Arc<dyn VectorSearcher> 动态分派 vs 静态泛型 hot path 损耗；缓解：bench 18.2 同时 measure trait 路径 vs 直接调用 baseline；如 ≥5% 损耗 task-18.7 转 enum-based static dispatch
     - trait API 90 天稳定性约束（AC7）：本 task ship 后 90 天内 trait 不能 break；如下游 18.3-18.6 发现 trait 抽象不足以容纳某 backend → §8 卡住协议触发；缓解：trait 设计阶段 cross-check 4 backend 文档（sqlite-vec / qdrant / lancedb / hnsw）确保都能 fit
     - Cargo features 占位但 vector-{sqlite,qdrant,lancedb,hnsw} feature list 当前为空（AC4）：本 task ship 后 task-18.3-18.6 各自 PR 填 dep list；如不同 backend 之间 features 互斥 (如 sqlite-vec FFI 必须 link 但 lancedb 自带 Lance file format) 需 task-18.7 ADR 选定 default 时一并处理
     - retriever wiring None vs Some 行为差异（AC3）：当前 Some 时返空 — 上层 caller 不应假设 vector hits 永远空（task-18.7 wire 真 backend 后会返非空）；缓解：retriever 返结果 schema 保留 vector_hits 字段 + retrieval_method 字段含 vector 部分（task-18.7 wiring 时已就绪）
     - 与 ADR-002 sqlite-tantivy-layered-storage 的兼容性：本 task trait 抽象不强约束存储介质；某些 backend 可能需自带 lance 文件 / qdrant segment / sqlite-vec 二进制扩展，与 ADR-002 分层模型重叠；缓解：task-18.7 决策时如 backend 引入新存储层 → 走 ADR-002 amendment（仿 ADR-022 amend ADR-015 D5 pattern）
-->

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

- **完成日期**：<TBD-after-impl>
- **改动文件**：<TBD-after-impl>
- **commit 列表**：<TBD-after-impl>
- **§9 Verification 结果**（按 §9 实际列出的 key 1:1 展开）：
  - install: <TBD-after-impl>
  - lint: <TBD-after-impl>
  - typecheck: <TBD-after-impl>
  - unit-test: <TBD-after-impl>
  - coverage: <TBD-after-impl>
  - build: <TBD-after-impl>
  - manual: <TBD-after-impl>
- **剩余风险 / 未做项**：<TBD-after-impl>
- **下游 task 影响**：<TBD-after-impl>
