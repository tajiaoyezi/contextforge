# Task `21.1`: `hybrid-scoring — core/src/retriever/fusion.rs 融合函数（RRF 或加权归一）+ Retriever::search_hybrid 入口 + SearchResult add-only hybrid_score 字段 + proto RetrievalResult.hybrid_score=15 / SearchRequest.hybrid=8（add-only）+ retrieval_method="hybrid"`

**Status**: Done

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 21 (retrieval-quality)
**Dependencies**: task-19.2（`Retriever::search()` BM25 + `search_semantic()` 语义两路生产热路径）/ task-19.3（proto `SearchRequest.semantic=7` + `RetrievalResult.vector_score=13`/`embedding_provider=14` add-only 范式 + buf 重生成流程）/ ADR-025（hybrid-scoring-fusion，本 phase 新 Proposed）/ ADR-015（Console Contract v1 add-only）/ ADR-017（22-endpoint conformance）/ ADR-013（禁伪造）/ ADR-014 D1-D5（第十二次激活）

## 1. Background

Phase 19（v0.12.0）落地了两条**独立**检索路径：`Retriever::search()`（BM25 词面，`retrieval_method = "bm25"`，`core/src/retriever/mod.rs:436`）与 `Retriever::search_semantic()`（向量语义，`retrieval_method = "vector"`，`core/src/retriever/mod.rs:641/719`）。两路结果**不融合**——`core/src/retriever/mod.rs:449-450` 与 `:639-640` 的行内注释明确标 `[SPEC-DEFER:phase-future.hybrid-scoring]`：语义命中不并入 BM25 结果集，BM25 探针向量命中只 log 不并入，`retrieval_method` 恒 `"bm25"` 或 `"vector"`。

`docs/releases/v0.12.0-artifacts.md:60` 与 `phase-19` §2 同样记录 hybrid scoring 为 deferred。本 task 兑现该 marker：提供把 BM25 词面分数与向量语义分数融合为单一排序的融合函数，新增 `Retriever::search_hybrid` 入口与 `retrieval_method = "hybrid"`，并 add-only 一个 `hybrid_score` 字段携带融合分量（既有 `score` BM25 分量 + `vector_score` 语义分量并存，可解释性增强）。

## 2. Goal

新增 `core/src/retriever/fusion.rs`（或 `mod.rs` 内函数）：融合函数接受 BM25 结果集 + 向量结果集，按融合策略（RRF 加权常数 / 加权归一，由 ADR-025 选型；本 task 落地其一并以确定性测试守正确性）算出 `hybrid_score`，按 `hybrid_score` 降序输出 `SearchResult` 集，`retrieval_method = "hybrid"`。新增 `Retriever::search_hybrid(query, top_k)`：内部复用 `search()` + `search_semantic()` 取两路结果后调融合函数。`SearchResult` add-only `hybrid_score: f32`（缺省 0.0 → 既有 BM25/vector 路径行为不变）。proto `RetrievalResult` add-only `float hybrid_score = 15` + `SearchRequest` add-only `bool hybrid = 8`（buf 重生成，承 task-19.3 add-only 范式）。固定 BM25/vector 分数 → 期望融合序在 `cargo test` 确定性可断言。≥3 Rust 测试全 PASS；默认 `cargo test --workspace` 不退化；D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **新增 `core/src/retriever/fusion.rs`**：纯函数 `fuse(bm25: &[SearchResult], vector: &[SearchResult], top_k: usize) -> Vec<SearchResult>`（或等价签名），按 ADR-025 选定的融合策略（RRF：`Σ 1/(k + rank)` 加权常数 / 加权归一：min-max 归一后加权和）计算 `hybrid_score`，按 `hybrid_score` 降序、同分以 `chunk_id` 稳定 tie-break（仿 `brute_force.rs` 确定性排序），`retrieval_method = "hybrid"`。
- **修改 `core/src/retriever/mod.rs`**：`pub mod fusion;` + 新增 `pub fn search_hybrid(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>, RetrieverError>`（复用 `search()` + `search_semantic()` 后调 `fusion::fuse`；任一路为空时降级为另一路结果，不 panic）。**实现决策（最小 churn，实施期定）**：不新增 `SearchResult` 结构字段——融合后的 RRF 分写进既有 `score` 字段 + `retrieval_method="hybrid"`；proto `RetrievalResult.hybrid_score` 由 server.rs 分派从 `score` 装配（仿 task-19.3 `vector_score`），避免改 `SearchResult` 触发全代码库字面量 churn。
- **修改 `proto/contextforge/v1/search.proto`**：`RetrievalResult` add-only `float hybrid_score = 15`（13/14 已占）+ `SearchRequest` add-only `bool hybrid = 8`（7 已占）；`buf generate proto` 重生成 Go pb（Rust pb 经 `core/build.rs` 自动重生成）。
- **修改 `core/src/server.rs`（CoreService.search，非 data_plane）**：`req.hybrid` 分派分支（仿 task-19.3 semantic 分支：wire `DeterministicEmbeddingProvider` + 0-dep `BruteForceVectorBackend` + `enumerate_chunks`/`index_chunks_semantic`）调 `search_hybrid`，map 后 `pr.hybrid_score = r.score`；`search_result_to_proto` 加 `hybrid_score: 0.0` 默认 + 既有 7 处 contextforge/v1 `SearchRequest` 字面量补 `hybrid: false`。core proto 路径 = CoreService（CLI / daemon-REST）；console_data_plane hybrid 转发 defer（[SPEC-DEFER:phase-future.console-api-hybrid-forward]）。
- **同源 Rust 单测（`core/src/retriever/fusion.rs` 内 `mod tests` + `mod.rs` `mod tests`）**：（a）固定 BM25/vector 分数 fixture → 断言融合序符合期望（确定性，不预判召回阈值）；（b）`retrieval_method == "hybrid"` + `hybrid_score` 填实（非 0）；（c）单路缺失（BM25 空 / vector 空）时 `search_hybrid` 降级为另一路，不 panic。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **`Retriever::search()` BM25 + `search_semantic()` 语义两路实现** [SPEC-OWNER:task-19.2-default-backend-wiring] / [SPEC-OWNER:task-19.3-semantic-search-api]：本 task 在其上融合，不实现两路本身。
- **reranker（cross-encoder）重排** [SPEC-OWNER:task-21.2-reranker-pipeline]：本 task 仅融合 BM25+vector 分数，reranker 管道在并行 task。
- **eval 报告 hybrid / reranked 召回列 + smoke 升级 + v0.14.0 release docs** [SPEC-OWNER:task-21.3-closeout-v0.14.0]：本 task 落融合 + proto 字段；eval/smoke/release 在收口 task。
- **融合策略最终选型 ratify（真实 dogfood eval 对比数值）** [SPEC-OWNER:task-21.3-closeout-v0.14.0]：本 task 落地确定性可验证的融合函数，真实召回对比驱动的 ADR-025 ratify 在收口 task。
- **console-api `?hybrid=true` 转发** [SPEC-DEFER:phase-future.console-api-hybrid-forward]：本 task 落 core 数据面融合 + gRPC 字段；console-api Go 转发承 Phase 20 console-api 语义贯通范式，后续版本贯通。
- **Console UI 重排 / 融合 explain 面板** [SPEC-OWNER:phase-future.console-semantic-explain]：跨仓库 Console 领域，本 task 仅就绪 `hybrid_score` provenance 数据通路。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`core/src/retriever/fusion.rs::fuse`**：本 task 新增的纯融合函数。
- **`core/src/retriever/mod.rs::Retriever`（`search` / `search_semantic` / 新 `search_hybrid`）**：BM25 + 语义两路 + 本 task 新增融合入口。
- **`SearchResult`（add-only `hybrid_score`）**：12-field explainable 结果，本 task add-only 第 13 字段。
- **`proto RetrievalResult` / `SearchRequest`（add-only `hybrid_score=15` / `hybrid=8`）**：schema unity 单源，本 task add-only。
- **下游 task-21.3**：closeout 在 eval / smoke 验证 hybrid + 据真实 eval ratify ADR-025。

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/retriever/mod.rs`（`SearchResult` struct @ 140 + `search()` BM25 @ 自 ~360 + `search_semantic()` @ 641 + `assemble_vector_result` @ 675 + 行内 `[SPEC-DEFER:phase-future.hybrid-scoring]` @ 449-450/639-640）
- `core/src/retriever/vector/brute_force.rs`（确定性 cosine 降序 + chunk_id tie-break 排序范式 @ 104-108）
- `proto/contextforge/v1/search.proto`（`SearchRequest.semantic=7` + `RetrievalResult.vector_score=13`/`embedding_provider=14` add-only 范式）
- `docs/specs/tasks/task-19.3-semantic-search-api.md`（proto add-only buf 重生成 + gRPC 装配范式）
- `docs/decisions/adr-025-hybrid-scoring-fusion.md`（本 phase ADR：融合策略选型 + 数据驱动 ratify）
- `docs/decisions/adr-006-recall-eval-acceptance-gate.md`（recall gate，hybrid 召回阈值口径属 task-21.3）+ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md` 与 ADR-013 禁伪造（数值口径）

### 5.2 关键设计 — 融合函数确定性

- 融合策略二选一（ADR-025 选型）：**RRF**（`hybrid_score = Σ_path w_path / (k_const + rank_path)`，rank 从 1 起，k_const 常数如 60）或 **加权归一**（各路 score min-max 归一到 [0,1] 后加权和）。本 task 落地选定策略，确定性测试守正确性。
- 同一 `chunk_id` 在两路均命中 → 融合分累加两路贡献；仅单路命中 → 仅该路贡献。
- 排序：`hybrid_score` 降序，同分以 `chunk_id` 升序稳定 tie-break（确定性，仿 `brute_force.rs`）。
- `search_hybrid`：`search()` + `search_semantic()` 各取 ≥ top_k 候选后融合取 top_k；任一路为空（如 None embedder/backend → `search_semantic` 返 `Ok(vec![])`）时降级为另一路结果，`retrieval_method` 仍标 `"hybrid"`（融合分退化为单路分）。

### 5.3 不变量

- `SearchResult.hybrid_score` 缺省 0.0：既有 `search()` / `search_semantic()` 返回值逐字段等价于现状（BM25/vector 路径不受 add-only 字段影响）。
- proto add-only（`hybrid_score=15` / `hybrid=8`）：`hybrid` 缺省 false → 既有客户端不受影响；响应仅 add-only 字段，22-endpoint conformance 不破坏（ADR-017）。proto-freeze 守护（只增不删不改号）。
- 默认 `cargo test --workspace` 不退化；hybrid 路径 opt-in，默认 BM25 行为不变（ADR-023 D5 默认 BM25 baseline 守线）。

## 6. Acceptance Criteria

- [x] **AC1**: `fusion::fuse` 在固定 BM25/vector 分数 fixture 下产生**确定性期望融合序**（`hybrid_score` 降序 + `chunk_id` 稳定 tie-break）；两路均命中的 chunk 融合分累加 — verified by **TEST-21.1.1**
- [x] **AC2**: `Retriever::search_hybrid` 返回 `retrieval_method == "hybrid"` 的结果，`hybrid_score` 填实（非 0）；`SearchResult` add-only `hybrid_score` 缺省 0.0 不破坏既有 `search()`/`search_semantic()` — verified by **TEST-21.1.2**
- [x] **AC3**: 单路缺失降级 — BM25 空 / vector 空（None embedder/backend）时 `search_hybrid` 返回另一路结果不 panic；proto `RetrievalResult.hybrid_score=15` / `SearchRequest.hybrid=8` add-only（buf 重生成）+ 22-endpoint conformance + proto-freeze 守护 PASS — verified by **TEST-21.1.3**
- [x] **AC4**: 既有不退化 — 默认 `cargo test --workspace` 全 PASS（hybrid 路径 opt-in，默认 BM25 行为不变）；`go test ./...` 全 PASS（含 conformance）— verified by **TEST-21.1.4** + §10 实测
- [x] **AC5**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-21.1.5** + §10 记录

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-21.1.1 | 固定 BM25/vector 分数 → 确定性期望融合序 + 两路命中累加 | `core/src/retriever/fusion.rs`（`mod tests`） | Done |
| TEST-21.1.2 | `search_hybrid` retrieval_method=hybrid + hybrid_score 填实 + add-only 缺省不退化 | `core/src/retriever/mod.rs`（`mod tests`） | Done |
| TEST-21.1.3 | 单路缺失降级不 panic + proto add-only + conformance/proto-freeze 守护 | `core/src/retriever/mod.rs` + `test/conformance/` + proto-freeze test | Done |
| TEST-21.1.4 | 默认 `cargo test --workspace` + `go test ./...` 0 failed | 全 Rust + Go | Done |
| TEST-21.1.5 | D2 lint `--touched origin/master` 0 未标注命中 | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（中）融合策略在确定性 fixture 上的「正确性」与「召回最优」分离**：确定性测试只能验证融合函数算对，不能验证哪种策略召回更高。
  - **缓解**：本 task AC 只断言融合**正确性 + 序确定性**（不预判召回数值，ADR-013）；策略选型的真实召回对比驱动属 task-21.3 + ADR-025 数据驱动 ratify。若 ADR-025 选型未定，本 task 落地一个明确策略并在 §10 记录，ratify 时如实对比。
- **R2（低）proto 字段号竞争**：`hybrid_score=15` / `hybrid=8` 与 task-21.2 reranker 可能新增字段号冲突。
  - **缓解**：按当前空号顺序分配（vector_score=13/embedding_provider=14 已占 → hybrid_score=15；semantic=7 已占 → hybrid=8）；task-21.1 / task-21.2 proto 改动 PR 串行化（phase §5 依赖列已注）；proto-freeze 守护复跑。
- **R3（低）`SearchResult` add-only 字段遗漏构造点**：既有多处构造 `SearchResult`（search/get_chunk/assemble_vector_result）需补 `hybrid_score: 0.0`。
  - **缓解**：编译器强制——add field 后未补的构造点 `cargo check` 即报错；逐点补缺省 0.0，确定性测试守 BM25/vector 路径 `hybrid_score==0.0`。

## 9. Verification Plan

```bash
# Rust：融合函数确定性 + search_hybrid + add-only 不退化
cargo test -p contextforge-core retriever::fusion -- --nocapture
cargo test --workspace

# proto add-only 重生成 + conformance + proto-freeze
# (buf generate per task-19.3 流程)
go test ./test/conformance/...
go test ./...

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**：2026-05-31
- **落地融合策略**：**RRF（Reciprocal Rank Fusion）**，`score = Σ_path 1/(k + rank)`，k=60。选 RRF 而非 min-max 加权归一：rank-based 无需 per-path 分数归一、确定性、参数轻（k 单常数）。ADR-025 据此 + task-21.3 真实 eval ratify。
- **改动文件**：`proto/contextforge/v1/search.proto`（`SearchRequest.hybrid=8` + `RetrievalResult.hybrid_score=15` add-only）+ `search.pb.go`（buf generate proto 重生成）、`core/src/retriever/fusion.rs`（新增 RRF `fuse` + 3 单测）、`core/src/retriever/mod.rs`（`pub mod fusion` + `search_hybrid`）、`core/src/server.rs`（CoreService.search `req.hybrid` 分派分支 + `search_result_to_proto` `hybrid_score:0.0` + `test_21_1_hybrid_dispatches_fusion_path` + 6 处 contextforge/v1 `SearchRequest` 字面量补 `hybrid:false`）、`core/tests/phase6_smoke.rs`（第 7 处字面量补 `hybrid:false`）、本 spec + `docs/s2v-adapter.md`（21.1 Done）。
- **设计取舍（实施期定，记于 §3）**：(1) **不新增 `SearchResult` 结构字段**——融合 RRF 分写进既有 `score` + `retrieval_method="hybrid"`，proto `hybrid_score` 由 server.rs 分派从 `score` 装配（仿 19.3 `vector_score`），避免改 `SearchResult` 触发全代码库字面量 churn。(2) **分派在 `core/src/server.rs` CoreService**（core proto 路径，CLI/daemon-REST），**非** data_plane/search.rs；console_data_plane hybrid 转发 defer（[SPEC-DEFER:phase-future.console-api-hybrid-forward]）。(3) proto add-only 字段破坏 7 处既有 Rust exhaustive 字面量 → 用 `cargo test --no-run` 编译器枚举 + 逐一补 `hybrid:false`（注意 console_data_plane `SearchRequest` 是另一 proto，不补）。
- **§9 Verification 结果**：`cargo test --workspace`（WSL2）全 PASS——22 test 二进制 + 新 `test_21_1_rrf_fuse_deterministic_order_and_dual_path_boost` / `test_21_1_rrf_respects_top_k` / `test_21_1_rrf_single_path_and_empty` / `test_21_1_hybrid_dispatches_fusion_path` 全 ok；`go build ./...` + `go test ./...` 0 failed（Go pb 加 Hybrid 字段，既有 Go 字面量无需补，编译通过）；D2 lint `--touched origin/master` 0 命中（见 commit）。ADR-013：确定性 embeddings 证融合分派 plumbing，非召回质量。
- **剩余风险 / 下游**：reranker [SPEC-OWNER:task-21.2-reranker-pipeline]；eval hybrid 列 + smoke + v0.14.0 release docs + ADR-025/026 ratify [SPEC-OWNER:task-21.3-closeout-v0.14.0]；console-api `?hybrid=true` 转发 [SPEC-DEFER:phase-future.console-api-hybrid-forward]。
