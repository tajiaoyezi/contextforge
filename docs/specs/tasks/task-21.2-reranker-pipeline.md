# Task `21.2`: `reranker-pipeline — core/src/rerank/{mod,traits,identity,cross_encoder}.rs Reranker trait + 确定性 IdentityReranker（默认构建 0 模型依赖，供 CI/测试）+ CrossEncoderReranker（real cross-encoder, feature-gated）+ Retriever::with_reranker builder seam`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 21 (retrieval-quality)
**Dependencies**: task-19.1（`core/src/embedding/{mod,traits}.rs` `EmbeddingProvider` trait 风格 + `DeterministicEmbeddingProvider` 确定性默认 + `FastEmbedProvider` feature-gated 范式）/ task-19.2（`Retriever::with_embedder` / `with_vector_searcher` builder seam 范式 + `search_semantic` top-k 来源）/ ADR-026（reranker-provider，本 phase 新 Proposed）/ ADR-008（core-library-selection，reranker 新 dep add-only amendment）/ ADR-013（禁伪造）/ ADR-014 D1-D5（第十二次激活）

## 1. Background

Phase 19（v0.12.0）落地了语义检索路径（`Retriever::search_semantic`，双塔 embedding 余弦），但未含 reranker。`docs/releases/v0.12.0-artifacts.md:59` 与 `phase-19` §2 记录 reranker（cross-encoder）为 deferred（`[SPEC-DEFER:phase-future.reranker]`）。cross-encoder 对 query×doc 对联合编码打分，比双塔 embedding 余弦更精准，可在初筛 top-k 之上重排提升 top-1 / MRR——但 cross-encoder 需真实模型，其真实质量数值需模型 + 真实 eval 验证。

`core/src/embedding/{mod,traits}.rs`（task-19.1）已确立「trait + 确定性默认实现（默认构建 0 模型依赖）+ real provider（feature-gated，`embedding-fastembed`）」范式：`EmbeddingProvider` trait（`Send + Sync + Debug` + `#[non_exhaustive]` error），`DeterministicEmbeddingProvider`（模型自由缺省），`FastEmbedProvider`（`#[cfg(feature="embedding-fastembed")]`，默认 0 新 crate）。本 task 仿此范式落地 reranker：`Reranker` trait + 确定性 `IdentityReranker`（管道 CI 可验证）+ `CrossEncoderReranker`（real，feature-gated，真实质量如实记录/受阻 defer）。

## 2. Goal

新增 `core/src/rerank/` 模块：`Reranker` trait（仿 `EmbeddingProvider`：`Send + Sync + Debug`，`fn rerank(&self, query, candidates) -> Result<Vec<...>, RerankError>` + `fn name(&self) -> &'static str`；`#[non_exhaustive]` error）；`IdentityReranker`（确定性 identity 实现：按确定性规则稳定重排——保持输入相对序 / 以候选既有 score + chunk_id 稳定排序，默认构建 0 模型依赖，供 CI/测试）；`CrossEncoderReranker`（real cross-encoder provider，`#[cfg(feature="reranker-<impl>")]`，默认构建 0 新 crate）。`core/Cargo.toml` add-only feature `reranker-<impl>`（optional dep；R7 走主 agent chore PR）。`Retriever` add-only `with_reranker(Arc<dyn Reranker>)` builder（仿 `with_embedder`）+ rerank 在 `search_semantic` / `search_hybrid` 返回的 top-k 之上 opt-in 应用。确定性 `IdentityReranker` 重排管道在 `cargo test` 可断言（序稳定 + 不丢候选）；real `CrossEncoderReranker` 真实质量数值经 feature-gated 入口本地复跑记录到 spike doc，CI（默认 feature）不构建模型。≥2 Rust 测试全 PASS；默认 `cargo test --workspace` 不退化；D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **新增 `core/src/rerank/mod.rs`**：模块导出 `Reranker` trait + `RerankError` + `IdentityReranker`（默认）；`#[cfg(feature="reranker-<impl>")] pub mod cross_encoder; pub use cross_encoder::CrossEncoderReranker;`（仿 `embedding/mod.rs` 结构）。
- **新增 `core/src/rerank/traits.rs`**：`Reranker` trait（`Send + Sync + Debug`，`rerank(query: &str, candidates: &[SearchResult]) -> Result<Vec<SearchResult>, RerankError>`，输出按重排分降序的候选子集 + `name()`）+ `RerankError`（`#[non_exhaustive]`，仿 `EmbeddingError`）。
- **新增 `core/src/rerank/identity.rs`**：`IdentityReranker`——确定性 identity 实现，按候选既有 `score`（或 `hybrid_score`/`vector_score`）降序 + `chunk_id` 稳定 tie-break 重排（不引入模型，不改候选内容；可解释 `reason` 标注重排来源）；默认构建可用。
- **新增 `core/src/rerank/cross_encoder.rs`（feature-gated）**：`CrossEncoderReranker`——real cross-encoder provider（feature `reranker-<impl>`；候选 crate 由 ADR-026 选型，承 `embedding-fastembed` feature-gate 范式），query×doc 对联合打分重排；默认构建不编译（0 新 crate）。
- **修改 `core/Cargo.toml`**：add-only `[features]` 项 `reranker-<impl> = ["dep:<crate>"]`（optional dep default 不含；具体 crate 由 ADR-026 选型，R7 dep 引入走主 agent chore PR，本 task spec 不私改 Cargo.toml dep 行 → return needs-dep）。
- **修改 `core/src/retriever/mod.rs`**：add-only `reranker: Option<Arc<dyn Reranker>>` 字段 + `with_reranker(mut self, r: Arc<dyn Reranker>) -> Self` builder（仿 `with_embedder` @ 587）；`search_semantic` / `search_hybrid` 在取得 top-k 后，若 `reranker` 为 `Some` 则 opt-in 应用重排（None → 不变，向后兼容）。
- **同源 Rust 单测（`core/src/rerank/identity.rs` 内 `mod tests` + `mod.rs` `mod tests`）**：（a）`IdentityReranker` 在固定候选 fixture 下确定性重排序（序稳定 + 候选不丢 + 内容不改）；（b）`Retriever::with_reranker(IdentityReranker)` 后 `search_semantic`/`search_hybrid` 经重排管道返回重排后序，None reranker 时序不变（向后兼容）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **real cross-encoder 模型的真实质量数值** [SPEC-DEFER:phase-future.reranker-real-quality]：本 task 落 real `CrossEncoderReranker` 管道（feature-gated）+ 确定性 `IdentityReranker` CI 可验证；real 模型 top-1/MRR 提升的真实质量数值需模型 + 真实 eval 本地复跑，受阻平台如实 defer（ADR-013，不伪造）。
- **reranker 选型最终 ratify（真实 eval 质量对比）** [SPEC-OWNER:task-21.3-closeout-v0.14.0]：本 task 落地 trait + 两实现，ADR-026 据真实质量数据 ratify 在收口 task。
- **hybrid scoring 融合函数** [SPEC-OWNER:task-21.1-hybrid-scoring]：本 task reranker 在 `search_hybrid` 返回的 top-k 上重排，不实现融合本身。
- **eval reranked 召回列 + smoke + v0.14.0 release docs** [SPEC-OWNER:task-21.3-closeout-v0.14.0]：本 task 落 rerank 管道；eval/smoke/release 在收口 task。
- **reranker crate 的 Cargo.toml dep 行落地** [SPEC-OWNER:phase-future.reranker-dep-chore]：R7 dep 引入由主 agent chore PR 承接（subagent 禁改 Cargo.toml dep → return needs-dep）。
- **console-api `?rerank=true` 转发** [SPEC-DEFER:phase-future.console-api-rerank-forward]：本 task 落 core 数据面 rerank seam；console-api Go 转发承 Phase 20 范式，后续版本贯通。
- **Console UI 重排 explain 面板** [SPEC-OWNER:phase-future.console-semantic-explain]：跨仓库 Console 领域。

## 4. Actors

- **主 agent**：实施 + PR 主理 + reranker crate 选型 chore PR。
- **`core/src/rerank/traits.rs::Reranker`**：本 task 新增 trait（仿 `EmbeddingProvider`）。
- **`core/src/rerank/identity.rs::IdentityReranker`**：确定性 identity 实现，默认构建可用，CI 验证重排管道。
- **`core/src/rerank/cross_encoder.rs::CrossEncoderReranker`（feature-gated）**：real cross-encoder，真实质量本地复跑。
- **`core/src/retriever/mod.rs::Retriever`（add-only `with_reranker`）**：本 task 加 reranker seam。
- **`core/src/embedding/{mod,traits}.rs`（参考）**：task-19.1 已确立的「trait + 确定性默认 + real feature-gated」范式，本 task 仿照。
- **下游 task-21.3**：closeout 在 eval / smoke 验证 rerank + 据真实质量数据 ratify ADR-026。

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/embedding/mod.rs`（模块结构：`pub mod traits/deterministic` + `#[cfg(feature)] pub mod fastembed_provider` + re-export）
- `core/src/embedding/traits.rs`（`EmbeddingProvider` trait `Send + Sync + Debug` + `#[non_exhaustive] EmbeddingError` 范式，供 `Reranker` / `RerankError` 仿照）
- `core/src/embedding/deterministic.rs`（`DeterministicEmbeddingProvider` 确定性默认实现范式，供 `IdentityReranker` 仿照）
- `core/src/retriever/mod.rs`（`with_embedder` @ 587 / `with_vector_searcher` @ 581 builder seam 范式 + `search_semantic` @ 641 top-k 来源 + `SearchResult` @ 140）
- `core/Cargo.toml`（`embedding-fastembed = ["dep:fastembed"]` feature-gate 范式 @ 99/115，供 reranker feature 仿照）
- `docs/decisions/adr-026-reranker-provider.md`（本 phase ADR：reranker 选型 + 确定性默认 + real feature-gated）+ `docs/decisions/adr-008-core-library-selection.md`（dep add-only amendment）+ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md` 与 ADR-013 禁伪造

### 5.2 关键设计 — 仿 embedding 范式的 trait + 确定性默认 + real feature-gated

- `Reranker` trait 与 `EmbeddingProvider` 同风格：object-safe（`Arc<dyn Reranker>`），`#[non_exhaustive]` error 让下游 match add-only-safe。
- `IdentityReranker`：确定性——按候选既有分数（`hybrid_score` > 0 取之，否则 `score`）降序 + `chunk_id` 升序稳定 tie-break；不改候选内容、不丢候选、可在 `reason` 标注「reranked by identity」。它是默认构建可用的确定性 identity 实现（非空壳实现），供 CI/测试验证 rerank 管道 wiring（管道接线正确性可断言，不预判真实质量）。
- `CrossEncoderReranker`：feature-gated；real cross-encoder 模型对 query×doc 联合打分；默认 feature 下整模块不编译（0 新 crate，仿 `FastEmbedProvider` `#[cfg(feature="embedding-fastembed")]`）。
- `Retriever::with_reranker`：opt-in seam；`search_semantic`/`search_hybrid` 取 top-k 后，`Some(reranker)` → 应用重排，`None` → 不变（既有路径向后兼容）。

### 5.3 不变量

- 默认 `cargo test --workspace` 不退化（`CrossEncoderReranker` feature-gated，默认 0 新 crate；`IdentityReranker` 0 模型依赖）。
- `None` reranker → `search_semantic`/`search_hybrid` 序与本 task 前逐字段等价（add-only seam，向后兼容）。
- ADR-013：real cross-encoder 真实质量数值只在真实模型 run 下记录；确定性 `IdentityReranker` 测试只断言管道 wiring + 序确定性，不冒充真实质量。

## 6. Acceptance Criteria

- [x] **AC1**: `Reranker` trait（`Send + Sync + Debug` + `#[non_exhaustive] RerankError`）+ `IdentityReranker` 落地；固定候选 fixture 下 `IdentityReranker.rerank` 确定性重排序（序稳定 + 候选不丢 + 内容不改）— verified by **TEST-21.2.1**
- [x] **AC2**: `Retriever::with_reranker(Arc<dyn Reranker>)` builder seam；`Some(IdentityReranker)` 时 `search_semantic`/`search_hybrid` 经重排管道返回重排序，`None` 时序不变（向后兼容）— verified by **TEST-21.2.2**
- [x] **AC3**: `CrossEncoderReranker`（feature `reranker-<impl>`）real cross-encoder 重排在 feature-gated 入口本地复跑产真实质量数值，记录到 `docs/spikes/phase-21-reranker.md`；数据源 ADR-013 三态如实标（real run / deterministic identity / 受阻 defer）；受阻平台如实 defer 不伪造 — verified by **TEST-21.2.3** + §10 实测记录
- [x] **AC4**: 既有不退化 — 默认 `cargo test --workspace` 全 PASS（`CrossEncoderReranker` feature-gated 不引默认 dep，`IdentityReranker` 0 模型依赖）；`go test ./...` 不受影响（本 PR 零 Go delta）— verified by **TEST-21.2.4** + §10
- [x] **AC5**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-21.2.5** + §10 记录

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-21.2.1 | `IdentityReranker` 固定候选确定性重排序 + 候选不丢/不改 | `core/src/rerank/identity.rs`（`mod tests`） | Done |
| TEST-21.2.2 | `Retriever::with_reranker` seam + None 序不变（向后兼容） | `core/src/retriever/mod.rs`（`mod tests`） | Done |
| TEST-21.2.3 | real `CrossEncoderReranker`（feature）真实质量数值 + spike 记录（三态如实标） | `core/src/rerank/cross_encoder.rs` + `docs/spikes/phase-21-reranker.md` | Done |
| TEST-21.2.4 | 默认 `cargo test --workspace` 0 failed（feature-gated 不引默认 dep） | 全 Rust | Done |
| TEST-21.2.5 | D2 lint `--touched origin/master` 0 未标注命中 | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（高）real cross-encoder 模型 + 真实 eval 平台门槛**（承 phase-21 §7 R2）：cross-encoder 模型下载 / Windows MSVC 或 Linux 构建受阻 → 真实质量数值不可得。
  - **缓解**：确定性 `IdentityReranker` 路径 CI 可验证重排管道 wiring；real `CrossEncoderReranker` 质量数值 🔴 需模型 + 真实 eval 本地复跑。stop-condition：real reranker provider 两平台均不可构建 / 模型不可得 → 确定性管道跑通 + 真实质量数值如实 defer（`[SPEC-DEFER:phase-future.reranker-real-quality]`），AC3 记录受阻态，不标 `[x]`，ADR-026 维持 Proposed（ADR-013 不伪造）。
- **R2（中）reranker crate 选型未定 + R7 dep 引入**：cross-encoder crate（如 fastembed rerank API / 独立 ONNX 模型 crate）需 ADR-026 选型 + 主 agent chore PR 引入。
  - **缓解**：ADR-026 据候选评估选型；feature-gated 隔离默认构建；R7 dep 引入由主 agent chore PR 承接（subagent 禁改 Cargo.toml dep → return needs-dep 对象）；`IdentityReranker` 不依赖任何 crate，即使 real crate 未定也可先落 trait + 确定性默认。
- **R3（低）`with_reranker` seam 改动 `search_semantic`/`search_hybrid` 既有序**：rerank opt-in 不应影响 None 路径。
  - **缓解**：`None` reranker 路径序逐字段等价测试守护（向后兼容）；rerank 仅在 `Some` 分支应用；确定性测试覆盖 Some/None 两组合。

## 9. Verification Plan

```bash
# Rust：Reranker trait + IdentityReranker 确定性 + with_reranker seam（默认 feature，CI）
cargo test -p contextforge-core rerank -- --nocapture
cargo test --workspace

# real cross-encoder rerank（需 reranker-<impl> feature；下载模型，本地复跑）
cargo test -p contextforge-core --features reranker-<impl> rerank::cross_encoder -- --nocapture

# Go 不退化（本 PR 零 Go delta）
go test ./...

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**：2026-05-31
- **改动文件**：
  - `core/src/rerank/traits.rs`（新增）— `Reranker` trait（`Send + Sync + Debug`，object-safe `Arc<dyn Reranker>`）+ `#[non_exhaustive] RerankError`（仿 task-19.1 `EmbeddingProvider`/`EmbeddingError`）
  - `core/src/rerank/identity.rs`（新增）— `IdentityReranker` 确定性默认（0 模型依赖）+ TEST-21.2.1
  - `core/src/rerank/cross_encoder.rs`（新增，`reranker-fastembed` gate）— `CrossEncoderReranker`（fastembed-rs `TextRerank` / BGE-reranker-base）+ TEST-21.2.3
  - `core/src/rerank/mod.rs`（新增）— 模块导出（cross_encoder 经 feature gate）
  - `core/src/lib.rs`（修改）— `pub mod rerank;`
  - `core/src/retriever/mod.rs`（修改）— `reranker` 字段 + `with_reranker` builder + `apply_rerank` helper + `search_semantic`/`search_hybrid` 接线（`search_semantic_raw` 拆分）+ TEST-21.2.2
  - `core/Cargo.toml`（修改）— add-only feature `reranker-fastembed = ["dep:fastembed"]`
  - `docs/spikes/phase-21-reranker.md`（新增）— 三态如实记录（确定性 / real 编译 / real 运行）
  - 本 spec（§6 AC1-5 [x] / §7 Done / §10）+ `docs/s2v-adapter.md`（task 21.2 Done）
- **commit 列表**：
  - `4435159` docs(spec): Status Draft → In Progress
  - `c1d1256` test(rerank): TEST-21.2.1/21.2.2 RED + Reranker trait/RerankError + IdentityReranker 骨架 + seam
  - `7d98f4a` feat(rerank): IdentityReranker 确定性重排 + CrossEncoderReranker（feature-gated）GREEN
  - `a64e83d` docs(spec): 回填 §10 + Status → Done + phase-21 reranker spike
  - `b194513` docs(adapter): 标记 task-21.2 为 Done
  - （本提交）test(rerank): adversarial review 跟进 — 强化 TEST-21.2.2 seam 序断言（ReverseReranker）+ trait/hybrid doc 修订
- **设计取舍（实施期定）**：
  - (1) **reranker crate = 复用既有 `fastembed` optional dep**（add-only feature `reranker-fastembed`，0 新 crate、Cargo.lock 无变更）——fastembed v4.9.1 自带 `TextRerank` cross-encoder API，无需引新 crate，故不触发 R7 needs-dep（仅加 `[features]` 行）。
  - (2) **`IdentityReranker` 重排规则**：按候选既有 `score` 降序 + `chunk_id` 升序稳定 tie-break（仿 `fusion.rs`），不丢/不改候选内容，`reason` 标注 `reranked:identity` 让重排来源可观测（ADR-026 D2）。`SearchResult` 无 `hybrid_score` 字段（task-21.1 把融合分写进 `score`），故评分取 `score`。
  - (3) **`search_hybrid` 用 `search_semantic_raw`（未经 rerank）的向量结果做 RRF 融合**，reranker 仅在融合后 top-k 上应用一次（避免向量分量被重排扭曲 RRF rank + 避免二次重排）。`None` reranker → `search_semantic`/`search_hybrid` 逐字段等价于本 task 前（向后兼容）。
- **§9 Verification 结果**：
  - 默认 `cargo test --workspace`：全 PASS（22 test 二进制，新增 `test_21_2_1_identity_rerank_deterministic_order_no_drop_no_content_change` + `test_21_2_2_with_reranker_seam_applies_and_none_unchanged` ok；0 failed）（AC4）。
  - `cargo check -p contextforge-core --features reranker-fastembed`：PASS（fastembed v4.9.1 + ort v2.0.0-rc.9 编译，0 error）——`CrossEncoderReranker` 编译于 real fastembed `TextRerank` API。
  - real run `cargo test -p contextforge-core --features reranker-fastembed test_21_2_3 -- --nocapture`：**1 passed**（393.67s，真实 BGE-reranker-base 下载+ONNX 推理，Windows MSVC 2026-05-31）——real cross-encoder 正确按 query 相关性重排（bamboo 文档高于无关文档），score 降序 + 来源标注（详 `docs/spikes/phase-21-reranker.md`，ADR-013 real-run 数据源声明）（AC3）。
  - `go test ./...`：24 包 ok，0 failed（本 PR 零 Go delta）（AC4）。
  - D2 lint `bash scripts/spec_drift_lint.sh --touched origin/master`：0 未标注命中（AC5）。
  - adversarial 多维 review（correctness / spec-adr / compat-honesty / reuse）：0 critical/major；4 minor/nit 全部跟进——TEST-21.2.2 加 `ReverseReranker` 断言「seam 输出序由 reranker 决定」（闭合「序回归不可检」gap）+ `search_hybrid` doc 改引 `search_semantic_raw` + `Reranker::name()` doc 软化为「may surface」。
- **剩余风险 / 未做项**：real cross-encoder 的 top-1/MRR 定量提升数值 + ADR-026 Proposed→Accepted ratify 由收口承接 [SPEC-OWNER:task-21.3-closeout-v0.14.0]；受阻平台真实质量复跑 [SPEC-DEFER:phase-future.reranker-real-quality]（本 task 未触发——模型已在 Windows MSVC 构建+运行）。
- **下游 task 影响**：task-21.3 消费本 task 的 `Reranker` / `with_reranker` seam + spike 真实运行结果做 eval reranked 列 + ADR-026 ratify；console-api `?rerank=true` 转发 [SPEC-DEFER:phase-future.console-api-rerank-forward]。
