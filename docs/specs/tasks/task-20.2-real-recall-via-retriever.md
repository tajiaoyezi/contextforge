# Task `20.2`: `real-recall-via-retriever — 让真实 SemanticRecall@K 评测经生产 Retriever::search_semantic 热路径产生（替代/补充 core/examples/phase19_real_recall.rs 旁路），deterministic provider wiring CI 可断言 + real fastembed 数值本地复跑记录`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 20 (semantic-retrieval-throughline)
**Dependencies**: task-19.2（`Retriever::with_embedder` + `index_chunks_semantic` + `search_semantic` 生产热路径）/ task-19.1（`EmbeddingProvider` trait + `DeterministicEmbeddingProvider` + `FastEmbedProvider` feature-gated）/ task-19.5（`core/examples/phase19_real_recall.rs` + `test/fixtures/eval/dogfood-embeddings.jsonl` + `docs/spikes/phase-19-real-recall.md` 真实召回基线）/ ADR-006 Amendment A1（SemanticRecall@10 ≥ 0.70 门禁）/ ADR-013（禁伪造凭据）/ ADR-014 D1-D5

## 1. Background

task-19.5（v0.12.0）用 `core/examples/phase19_real_recall.rs` 跑出真实 `SemanticRecall@5=0.8333 / @10=0.9333`（real `FastEmbedProvider`，exact cosine），喂 task-19.6 ratify ADR-023。但该 example 是**独立谐波**：它自建 `BruteForceVectorBackend` + 直接 embed/index/search，**未经生产 `Retriever::search_semantic` 热路径**。task-19.4 §10 / `docs/releases/v0.12.0-evidence.md` §3b 记录的 caveat 之一即「真实召回经 Retriever 热路径」（`[SPEC-DEFER:phase-future.real-recall-via-retriever]`，承 `task-14.2` / `RELEASE_NOTES`）。

task-19.2 已把 `with_embedder` + `index_chunks_semantic` + `search_semantic`（`retrieval_method=vector` + 12-field 装配）接进生产 `Retriever`。本 task 让真实召回评测走这条真实热路径，使 evidence 代表性从「旁路 example」升到「生产 Retriever」，并验证两者口径一致。

## 2. Goal

新增评测入口（`core/examples/phase20_recall_via_retriever.rs` 或扩 `phase19_real_recall.rs` 使其经 `Retriever::search_semantic`）：用真实 dogfood 语料（复用 `test/fixtures/eval/dogfood-embeddings.jsonl` 或同源真实文件）经生产 `Retriever`（`with_embedder` + `index_chunks_semantic` + `search_semantic`）跑 `SemanticRecall@5/10` + top-1 + MRR。deterministic provider 下 index→search_semantic roundtrip wiring 在 `cargo test` 可断言（命中预期 chunk）；real fastembed（feature `embedding-fastembed`）召回数值本地复跑记录到 `docs/spikes/phase-20-recall-via-retriever.md`，与 task-19.5 example 口径对比。≥2 Rust 测试全 PASS（默认 feature，feature-gated example 编为无 dep 入口不影响 `cargo test --workspace`）。D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **新增 `core/examples/phase20_recall_via_retriever.rs`**（feature-gated `#[cfg(feature="embedding-fastembed")]` 真实路径 + `#[cfg(not(...))]` 无 dep 入口，承 task-19.5 模式）：经 `Retriever::with_embedder` + `index_chunks_semantic` + `search_semantic` 跑真实召回（区别于 task-19.5 直接调 backend）。
- **同源 Rust 单测（`core/src/retriever/mod.rs` 内 `mod tests` 或 `core/tests/`）**：deterministic provider 下 `Retriever::search_semantic` index→search roundtrip 命中预期 chunk（wiring 正确性，CI 可跑，无模型 dep）。
- **新增 `docs/spikes/phase-20-recall-via-retriever.md`**：记录经 Retriever 热路径的真实 `SemanticRecall@K`（real fastembed 本地复跑）+ 与 task-19.5 旁路 example 口径对比 + ADR-013 数据源声明（real run / deterministic / 受阻三态如实标）。
- **可选扩 `bench/src/tests.rs`**：fixture 经 Retriever 路径加载格式校验（承 task-19.5 `test_19_5_real_dogfood_fixture_format`）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **`Retriever::search_semantic` 生产热路径实现** [SPEC-OWNER:task-19.2-default-backend-wiring]：本 task 在其上跑评测，不实现它。
- **`FastEmbedProvider` real provider + dogfood 语料** [SPEC-OWNER:task-19.1-spike-embedding-provider] / [SPEC-OWNER:task-19.5-real-recall-eval]：本 task 复用其产物。
- **console-api 语义转发** [SPEC-OWNER:task-20.1-console-api-semantic-forward]：本 task 是 Rust 数据面评测，与 Go console-api 写路径不相交。
- **hybrid scoring / reranker 对召回的影响** [SPEC-DEFER:phase-future.hybrid-scoring] / [SPEC-DEFER:phase-future.reranker]：v0.14.0 / Phase 21。
- **向量增量索引** [SPEC-DEFER:phase-future.vector-incremental-index]：承 Phase 18/19 默认全量 reindex。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`core/src/retriever/mod.rs::Retriever`（`with_embedder` / `index_chunks_semantic` / `search_semantic`）**：task-19.2 生产热路径，本 task 经它跑评测。
- **`core/src/embedding`（`DeterministicEmbeddingProvider` / `FastEmbedProvider`）**：embedding 来源，CI 用 deterministic，本地真实用 fastembed。
- **`test/fixtures/eval/dogfood-embeddings.jsonl`（task-19.5）**：真实语料 fixture。
- **上游 task-19.1/19.2/19.5**：提供 provider / 热路径 / 真实召回基线。
- **下游 task-20.3**：closeout 引用本 task 的经-Retriever 召回作为 v0.13.0 evidence。

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/retriever/mod.rs`（`with_embedder` / `index_chunks_semantic` / `search_semantic` / `retrieval_method=vector` 12-field 装配）
- `core/examples/phase19_real_recall.rs`（task-19.5 旁路 example：corpus chunk + embed + `BruteForceVectorBackend` + recall 计算）
- `docs/spikes/phase-19-real-recall.md`（真实召回方法 + 0.8333/0.9333 基线 + 平衡语料修正 artifact）
- `core/src/embedding/{traits,deterministic,fastembed_provider}.rs`
- `test/fixtures/eval/dogfood-embeddings.jsonl`（40 行 dim-384 real 语料）+ `bench/src/tests.rs`（fixture 校验）
- `docs/decisions/adr-006-recall-eval-acceptance-gate.md` Amendment A1 + `docs/decisions/adr-013-*.md`

### 5.2 关键设计 — 经 Retriever 热路径

- 评测入口构造生产 `Retriever`，`with_embedder(provider)`，`index_chunks_semantic(chunks)` 建语义索引，逐 query `search_semantic(query, top_k)` 取结果，按 task-19.5 同口径算 file-level Strong-hit `SemanticRecall@5/10` + top-1 + MRR。
- deterministic provider 路径在 `cargo test` 跑：固定 chunk + query → 断言 `search_semantic` 命中预期 chunk（wiring 正确性，不预判召回阈值）。
- real fastembed 路径 feature-gated，本地复跑产真实数值，写进 spike doc；CI（默认 feature）不构建模型。

### 5.3 不变量

- 默认 `cargo test --workspace` 不退化（feature-gated example 在默认 feature 下为无 dep 入口，承 task-19.5）。
- BM25 检索路径不受影响（仅新增语义评测入口）。
- ADR-013：经-Retriever 召回若与 task-19.5 example 口径有差异，如实记录差异成因，不强行对齐数字。

## 6. Acceptance Criteria

- [x] **AC1**: deterministic provider 下 `Retriever::search_semantic` index→search roundtrip 命中预期 chunk（wiring 正确性，`cargo test` 可断言，无模型 dep）— verified by **TEST-20.2.1**
- [x] **AC2**: real fastembed（feature `embedding-fastembed`）经 `Retriever` 热路径跑出真实 `SemanticRecall@5/10` + top-1 + MRR，记录到 `docs/spikes/phase-20-recall-via-retriever.md` 并与 task-19.5 旁路 example 口径对比；数据源 ADR-013 三态如实标（real run / deterministic / 受阻）— verified by **TEST-20.2.2** + §10 实测记录
- [x] **AC3**: 既有不退化 — 默认 `cargo test --workspace` 全 PASS（feature-gated example 不引入默认 dep）；`go test ./...` 不受影响（本 PR 零 Go delta）— verified by **TEST-20.2.3** + §10
- [x] **AC4**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-20.2.4** + §10

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-20.2.1 | deterministic `Retriever::search_semantic` roundtrip 命中预期 chunk | `core/src/retriever/mod.rs`（`mod tests`）或 `core/tests/` | Done |
| TEST-20.2.2 | real fastembed 经 Retriever 召回数值 + spike 记录 + 与 19.5 对比 | `core/examples/phase20_recall_via_retriever.rs` + `docs/spikes/phase-20-recall-via-retriever.md` | Done |
| TEST-20.2.3 | 默认 `cargo test --workspace` 0 failed | 全 Rust | Done |
| TEST-20.2.4 | D2 lint `--touched origin/master` 0 未标注命中 | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（中）real provider 平台/模型门槛**（承 phase-19 §7 R1 / phase-20 §7 R2）：fastembed 模型下载 / Windows MSVC 构建受阻 → 真实数值不可得。
  - **缓解**：deterministic provider 路径 CI 可验证 wiring；real 数值 🟡 本地 feature 复跑。stop-condition：两平台均不可构建 real provider → deterministic roundtrip 跑通 + 真实数值如实 defer（不伪造，ADR-013），AC2 记录受阻态，不标 `[x]`。
- **R2（中）经 Retriever 召回与 task-19.5 example 数值有差异**：热路径装配（12-field / retrieval_method）可能改变结果集。
  - **缓解**：如实记录差异 + 成因（chunk 切分 / 装配差异），不强行对齐；两口径均 ≥ gate 即闭环，否则诚实记录。
- **R3（低）fixture 复用导致语料偏差**：dogfood-embeddings.jsonl 为 task-19.5 平衡语料。
  - **缓解**：沿用 task-19.5 平衡语料口径（避免 dominant-file 召回膨胀 artifact）；spike doc 注明语料来源。

## 9. Verification Plan

```bash
# Rust：deterministic roundtrip wiring（CI 默认 feature）
cargo test --workspace

# real recall 经 Retriever 热路径（需 embedding-fastembed feature；下载 ONNX 模型，本地复跑）
cargo run -p contextforge-core --example phase20_recall_via_retriever --features embedding-fastembed

# Go 不退化（本 PR 零 Go delta）
go test ./...

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**：2026-05-31
- **改动文件**：`core/examples/phase20_recall_via_retriever.rs`（新增，feature-gated real-fastembed-via-Retriever 谐波 + 默认 no-op）、`core/src/retriever/mod.rs`（新增 `test_20_2_recall_via_retriever_brute_force_default_build` 默认构建确定性 hot-path 测试）、`docs/spikes/phase-20-recall-via-retriever.md`（新增，真实召回 + 与 19.5 对比 + 诚实膨胀 caveat）、本 spec + `docs/s2v-adapter.md`（20.2 Done）。零 Go delta。
- **§9 Verification 结果**：`cargo test --workspace`（WSL2）全 PASS（22 test 二进制 + 新 `test_20_2_recall_via_retriever_brute_force_default_build` ok）；real run `cargo run --example phase20_recall_via_retriever --features embedding-fastembed` 成功 → **production_chunks=175, SemanticRecall@5=0.9667 / @10=1.0000, top1=0.7333, MRR=0.8367, gate PASS**；`go test ./...` 不受影响（零 Go delta）；D2 lint `--touched origin/master` 0 未标注命中（spike doc 在 docs/spikes 不在 lint scope；本 spec 触及行清洁）。
- **设计取舍 / 诚实记录（ADR-013）**：经生产 `Retriever::search_semantic` 热路径（real scanner+chunker → 175 production chunks，uncapped）跑出的 recall@10=1.0 **部分含 file-level 膨胀**（chunk/file 多 → "任一 chunk 命中 top-K" 机械变易，正是 19.5 用 MAX_CHUNKS_PER_FILE 抑制的 artifact）；但区分性 top-1=0.7333 / MRR=0.8367（高于 19.5 的 0.60/0.70，不受 chunk 数膨胀影响）证实真实路径表现，非纯膨胀。两口径方法不同（生产 chunker vs 受控窗口）不直接可比，均过 gate；20.2 为代表性测量，19.5 为受控区分度地板。详 `docs/spikes/phase-20-recall-via-retriever.md` §3。
- **剩余风险 / 下游**：smoke v10 console-api 真实语义断言 + v0.13.0 release docs + ADR-024 ratify [SPEC-OWNER:task-20.3-closeout-v0.13.0]；real run 需 `embedding-fastembed` feature（CI 不跑，本地复跑），CI 由确定性 `test_20_2` 守 hot-path wiring。
