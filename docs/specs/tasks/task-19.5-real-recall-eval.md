# Task `19.5`: `real-recall-eval — 真实 dogfood embedding 语料 + SemanticRecall@5/10 实测 + docs/spikes/phase-19-real-recall.md 喂 ADR-023 ratify`

**Status**: Done

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 19 (vector-retrieval-integration)
**Dependencies**: task-19.1（real embedding provider + deterministic 缺省 provider，`core/src/embedding/`）/ task-19.2（选定默认 backend 接 `Retriever::with_vector_searcher` + index/query 过 `EmbeddingProvider` 生产 wiring）/ task-18.8（`internal/eval` `SemanticRecallAtK` + `SummarizeHybrid` + `MeetsRecallGate` + `GateSemanticRecall10Min=0.70` 度量已就绪）/ ADR-023（Phase 18 Proposed，本 task 真实数据 feed task-19.6 ratify）/ ADR-006 Amendment A1（SemanticRecall@10 ≥0.70 gate，provisional）/ ADR-013（禁伪造证据）/ ADR-014 D1-D5 第十次激活

## 1. Background

Phase 18 把四个向量 backend 测齐（`docs/spikes/phase-18-comparison.md`）后得到一条决定性结论：**合成种子向量 recall 不可区分**——四个 backend 在 n=100000 仍全 `recall@5/10 = 1.0`，无法据此排序（ADR-023 Context + task-18.7）。task-18.8 因此只落地了 `SemanticRecall@K` **度量 + 门禁 + 单测**（数学正确性），把 live 语义召回值显式 defer 到 `[SPEC-DEFER:phase-future.embedding-provider-full]` —— 因为当时仓内无 embedding provider、向量 backend 未接生产 retriever 热路径，没有真实分布 embedding 就无法产出可区分的召回数。

Phase 19 前序 task 补齐了缺口：task-19.1 落地 `EmbeddingProvider`（deterministic 缺省 provider + real provider feature-gated），task-19.2 把选定默认 backend 接进 `Retriever` 并在 index/query 前过 embedding。**本 task 是 ADR-023 Proposed→Accepted 的数据闭环**：用 task-19.1 的 **real provider** 对真实 ContextForge 源码 chunk（或 golden-question 子集）生成真实分布 embedding 语料，跑选定 backend 的语义检索，用 task-18.8 的 `SemanticRecallAtK`/`SummarizeHybrid` 算出**真实** `SemanticRecall@5/10`，落 `docs/spikes/phase-19-real-recall.md`，供 task-19.6 ratify ADR-023（D1 默认 backend 由 provisional 转 ratified，或据实测 documented 未决）。

**ADR-013 红线**：若 real provider 在两平台均不可构建（task-19.1 R1 stop-condition 触发，real provider deferred），则真实 embedding 语料无法生成 → 本 task **诚实记 blocked，recall ratify defer**，禁用 deterministic 缺省 provider 的派生向量假冒「真实召回」数据（deterministic 向量与 task-18.7 合成种子同属不可区分类，不能 ratify）。该 defer 走 [SPEC-OWNER:phase-future.embedding-provider-full]，不预先宣称 Done/Accepted。

## 2. Goal

用 task-19.1 real provider 把真实 ContextForge 源码 chunk（task-18.8 `BuiltinGoldenQuestions` 6 类别覆盖的 expected 文件 + 必要时 O6 golden-questions 完整版扩充）embed 成真实 dogfood 语料；经 task-19.2 生产 wiring 跑选定默认 backend 的语义检索，用 `internal/eval` 的 `SummarizeHybrid` 双路汇总 + `SemanticRecallAtK(K=5,10)` 算出**真实** `SemanticRecall@5/10`，落 `docs/spikes/phase-19-real-recall.md`（数据源 / provider name+dim / backend / 语料规模 / per-category recall 全标注，喂 task-19.6 ADR-023 ratify）。`go test ./...` + `cargo test --workspace` 不退化；D2 lint 触及行 0 未标注命中。real provider deferred 分支下，本 task 记 blocked + defer，绝不伪造 recall 数。

## 3. Scope

### In Scope

- **新增 `test/fixtures/eval/dogfood-embeddings.jsonl`** —— 真实 dogfood embedding 语料：对 ContextForge 真实源码 chunk（覆盖 task-18.8 `BuiltinGoldenQuestions` 6 类别 expected 文件 + golden-question 子集对应 chunk）用 task-19.1 real provider 生成 embedding，每行 `{"chunk_id": "...", "embedding": [...]}`（沿用 `bench/src/corpus.rs` `load_dogfood` 已识别的 JSONL 行格式 + `test/fixtures/spike/dogfood-contextforge.jsonl` 既有约定）。语料由 real provider 实跑生成，非手写。
- **新增/扩 `test/fixtures/eval/golden-semantic.jsonl`（如 O6 需）** —— golden-questions 完整版语义口径子集 [SPEC-OWNER:phase-19.golden-questions-full]：承 task-18.8 §Out of Scope 的 `phase-future.semantic-golden-dataset` forward-ref，每行一个 `eval.Question`（`query` + `expected_chunk_id`/`expected_file_path` + `category`，沿用 `internal/eval` `Question` JSON tag），供语义近邻判定。仅在本 phase 真实召回需要语义标注超出现 30 题 BM25 口径时新增。
- **修改 `internal/eval/`（如需 runner 接真实检索结果）** —— 把 real provider embedding 语料 + 选定 backend 的语义检索结果喂入既有 `EvaluateQuestion` → `SummarizeHybrid(bm25, semantic)`，复用 task-18.8 已落地的 `SemanticRecallAtK`/`MeetsRecallGate`（度量本身不改，仅接 real 检索结果到 semantic `[]Result`）。
- **新增 `docs/spikes/phase-19-real-recall.md`** —— 真实 `SemanticRecall@5/10` evidence：数据源（real provider name + dim + 语料 chunk 数 + question 数）、整体 recall@5/10、per-category 分解、`MeetsRecallGate` 结论（是否过 `GateSemanticRecall10Min=0.70`）、与 ADR-006 A1 阈值对照；明确标注「real provider 实跑，非 deterministic/合成」。该数据 feed task-19.6 ADR-023 ratify。
- **修改 `docs/s2v-adapter.md`** —— Phase 19 表 19.5 行 Pending → Done（real provider 可用分支）。

### Out of Scope（[SPEC-DEFER] / [SPEC-OWNER]）

- **real embedding provider 实现本身** [SPEC-OWNER:task-19.1-spike-embedding-provider]：本 task 消费 task-19.1 的 provider，不实现它。
- **选定默认 backend 接生产 retriever 热路径** [SPEC-OWNER:task-19.2-default-backend-wiring]：本 task 消费 task-19.2 的 wiring 跑检索，不改 wiring。
- **real provider 两平台均不可构建（task-19.1 R1 stop-condition）下的真实召回 + ADR-023 ratify** [SPEC-OWNER:phase-future.embedding-provider-full]：embedding provider 受阻则本 task blocked，记实测不可得 + recall ratify defer，不以 deterministic 缺省 provider 派生向量假冒真实召回（ADR-013）。
- **ADR-023 Status Proposed→Accepted 落笔** [SPEC-OWNER:task-19.6-adr-023-ratify]：本 task 产数据，task-19.6 据数据改 ADR Status + ADR-006 A1 转正。
- **CLI `--semantic` flag + smoke v9 step 30** [SPEC-OWNER:task-19.4-smoke-v9]：本 task 跑 library 度量产 evidence，CLI/smoke 接入在 task-19.4。
- **remote embedding provider（OpenAI/Cohere）** [SPEC-DEFER:phase-future.embedding-provider-remote]：承 Phase 19 §不在 scope，仅本地 provider。
- **hybrid BM25+Vector fusion 召回** [SPEC-DEFER:phase-future.hybrid-scoring]：本 task 语义路径单独测，BM25 + Vector 融合后置。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **task-19.1 real `EmbeddingProvider`**：对真实源码 chunk 产真实分布 embedding（语料源头）。
- **task-19.2 生产 wiring + 选定 backend**：跑语义检索，产 per-question semantic 结果。
- **`internal/eval`（task-18.8）**：`EvaluateQuestion` → `SummarizeHybrid` → `SemanticRecallAtK`/`MeetsRecallGate`，算真实召回。
- **`docs/spikes/phase-19-real-recall.md`**：真实召回 evidence，本 task 产出。
- **下游 task-19.6**：消费本 task 真实 recall 数据 ratify ADR-023 D1 + ADR-006 A1 转正。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/tasks/task-19.1-spike-embedding-provider.md`（real provider name/dim/`embed` 契约 + R1 stop-condition）+ `docs/specs/tasks/task-19.2-default-backend-wiring.md`（生产 wiring index/query 过 embedding 的接口）
- `docs/specs/tasks/task-18.8-eval-semantic-recall.md` + `internal/eval/eval.go`（`SemanticRecallAtK` / `SummarizeHybrid` / `MeetsRecallGate` / `GateSemanticRecall10Min` / `Question`/`Result` 形状）
- `bench/src/corpus.rs` `load_dogfood` + `test/fixtures/spike/dogfood-contextforge.jsonl`（dogfood JSONL 行格式约定）
- `docs/decisions/adr-023-vector-backend-default.md`（D1 PROVISIONAL + D6 ratify 前提）+ `docs/decisions/adr-006-recall-eval-acceptance-gate.md`（Amendment A1 provisional gate）
- `docs/decisions/adr-013-*`（禁伪造证据红线）+ `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5）
- `docs/spikes/phase-18-comparison.md`（合成 recall 不可区分的前因，本 task 解之）

### 5.2 关键设计

- **真实分布是核心**：语料 embedding **必须**来自 task-19.1 real provider（HF/ONNX 模型推理），不可用 task-18.7 合成种子或 task-19.1 deterministic 缺省 provider 的 hash/seed 派生向量——后两者属 ADR-023 已证「不可区分」类，无法 ratify。
- **语料构造**：对 task-18.8 6 类别 `BuiltinGoldenQuestions` 的 expected 文件 chunk + 同库其余真实源码 chunk 构成检索语料底；每条 golden question 的 `query` 文本经 real provider embed 为 query 向量，对语料底跑选定 backend KNN top-K。
- **召回判定复用既有口径**：检索结果转 `[]*contextforgev1.RetrievalResult` → `EvaluateQuestion(q, results, latency)` → semantic `[]Result` → `SummarizeHybrid(bm25, semantic)`；`SemanticRecallAtK` 用既有 Strong-hit@K 判定（`Outcome==Strong && 1<=MatchedRank<=K`），weak 不计。
- **gate 对照**：`MeetsRecallGate(report)` 仅在 `SemanticEvaluated` 时检 `SemanticRecallAt10 >= 0.70`；evidence 记是否过门 + per-category 分解，不因未过门而篡改数。
- **stop-condition 诚实分支**：task-19.1 R1 触发（real provider 两平台均不可构建）→ 真实 embedding 语料不可生成 → evidence 记「real provider deferred，真实 SemanticRecall 实测不可得」，recall ratify defer [SPEC-OWNER:phase-future.embedding-provider-full]；本 task 不产 recall 数字，AC 据该分支以 documented 未决收口（仿 Phase 18 closeout pattern）。

## 6. Acceptance Criteria

- [x] **AC1**: `test/fixtures/eval/dogfood-embeddings.jsonl` 由 task-19.1 real provider 实跑生成（40 行；覆盖 task-18.8 6 类别 expected 文件 chunk + 5 distractor 真实文件），每行 `{"chunk_id, embedding}` 合 `load_dogfood` 格式（dim 384）；非 deterministic/合成派生 — verified by **TEST-19.5.1**（`bench` `test_19_5_real_dogfood_fixture_format`：load_dogfood 解析 + dim==384 + 非全零 + ≥30 行；spike 数据源声明标注 real provider）
- [x] **AC2**: 真实召回实测 — `core/examples/phase19_real_recall.rs` 经 real `FastEmbedProvider` + 默认 `BruteForceVectorBackend`（exact cosine，代表任意 exact backend，含 ADR-023 D1 sqlite-vec）跑 30 golden 查询，产**真实** `SemanticRecall@5=0.8333 / @10=0.9333`（+ top1=0.60 / MRR=0.70 区分度指标 + per-category 分解），落 `docs/spikes/phase-19-real-recall.md` — verified by **TEST-19.5.2**（real recall run evidence + per-category 表 + 数据源标注）
- [x] **AC3**: gate 对照 — 真实 `SemanticRecall@10=0.9333 ≥ GateSemanticRecall10Min=0.70` → **PASS**，evidence 如实记（达阈 → 喂 task-19.6 A1 转正），未篡改数（headline 0.83/0.93 而非合成 1.0/1.0；artifact 1.0 已经 balanced corpus + top1/MRR 修正） — verified by **TEST-19.5.3**（gate 结论 + 实测值并列于 evidence 表）
- [x] **AC4**: ADR-013 诚实分支 — real provider 两平台均可构建（task-19.1 R1 stop **未触发**），故走 real-run 分支：evidence「Data-source declaration」明确声明全部数字来自 real `FastEmbedProvider` ONNX 推理，非合成/deterministic/伪造；deferred 分支不适用（二者互斥，本 task 取 real-run） — verified by **TEST-19.5.4**（evidence 数据源诚实声明 real-run，互斥 deferred）
- [x] **AC5**: 既有不退化 — `go test ./...` 全 PASS（本 task 零 Go 改动）；`cargo test --workspace` 不受影响（example feature-gated，默认构建编为 no-op 空 main（无 fastembed 依赖），0 新 dep；新增 `bench` fixture 测试 PASS）— verified by **TEST-19.5.5**（`cargo test -p contextforge-bench` PASS + CI cargo-test/go-test gate 复核）+ §10 实测
- [x] **AC6**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched master` PR 触及行 0 未标注命中 — verified by **TEST-19.5.6**（§10 记录的 D2 lint 实跑输出 + CI spec-lint gate）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-19.5.1 | dogfood real embedding fixture 行格式 + dim 384 + 非合成来源 | `test/fixtures/eval/dogfood-embeddings.jsonl` + `bench/src/tests.rs::test_19_5_real_dogfood_fixture_format` | Done（bench 测试 PASS，40 行 dim-384 非全零） |
| TEST-19.5.2 | 真实 SemanticRecall@5=0.8333/@10=0.9333 实测 + per-category + top1/MRR evidence | `core/examples/phase19_real_recall.rs` → `docs/spikes/phase-19-real-recall.md` | Done（real fastembed run，WSL2） |
| TEST-19.5.3 | gate `@10=0.9333 ≥ 0.70` = PASS 结论 + 实测并列 | `docs/spikes/phase-19-real-recall.md` | Done（结果表 + gate 行） |
| TEST-19.5.4 | ADR-013 数据源诚实声明（real-run，互斥 deferred） | `docs/spikes/phase-19-real-recall.md` | Done（Data-source declaration） |
| TEST-19.5.5 | cargo test -p bench PASS（含 fixture 测试）+ go test ./... 不退化 | 全 workspace | Done（bench 7/7 PASS；零 Go 改动；CI gate 复核） |
| TEST-19.5.6 | D2 lint --touched master 0 未标注命中 | `scripts/spec_drift_lint.sh` | Done（见 §10 / CI spec-lint） |

## 8. Risks

- **R1（高）real provider 不可得 → 真实召回不可产**：承 task-19.1 R1 stop-condition（fastembed(ort)/candle 在 Windows MSVC / CI 受阻）；deterministic 缺省 provider 派生向量与 task-18.7 合成同属不可区分类，不能 ratify。
  - **缓解**：诚实分支（§5.2）—— real provider deferred 时记 blocked + recall ratify defer [SPEC-OWNER:phase-future.embedding-provider-full]，绝不伪造（ADR-013）；本 task 据该分支以 documented 未决收口，下游 task-19.6 据「实测维持 Proposed」收。
- **R2（中）真实 recall 未达 0.70 门**：real embedding 上选定 backend recall@10 可能 < `GateSemanticRecall10Min`。
  - **缓解**：如实记实测值（不篡改），evidence 标注；task-19.6 据实测决定 A1 转正或维持 provisional（ADR-006 A1 本就 provisional）；不为过门改语料/口径。
- **R3（中）golden-question 语义标注口径**：现 30 题为 BM25 口径（精确 chunk/行重叠），语义近邻召回判定可能偏严。
  - **缓解**：O6 完整版子集 [SPEC-OWNER:phase-19.golden-questions-full] 按需补语义口径标注；evidence 标注用的判定口径（Strong-hit@K）+ per-category 分解，使口径可复核。
- **R4（低）real provider 非确定性影响可复跑**：模型推理 embedding 可能含微小数值抖动。
  - **缓解**：fixture 一次性生成后入库（语料固定），evidence 记 provider name+version+dim；复跑以入库 fixture 为准，避免每跑重 embed 漂移。

## 9. Verification Plan

```bash
# real provider 可用分支（task-19.1 real provider feature 已落地）
cargo test --workspace                       # real provider feature-gated，默认构建不引入
go test ./internal/eval/... -v               # eval 接真实检索结果路径
go test ./...                                # 全 Go 不退化
# 真实召回数据落 docs/spikes/phase-19-real-recall.md（real provider 实跑，见 PR 描述）
bash scripts/spec_drift_lint.sh --touched master
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**：2026-05-30
- **改动文件**：`core/examples/phase19_real_recall.rs`（新增，feature-gated `embedding-fastembed` real-recall 谐波；默认构建 no-op 空 main，无 fastembed 依赖）、`test/fixtures/eval/dogfood-embeddings.jsonl`（新增，real `FastEmbedProvider` 实跑生成 40 行 dim-384）、`bench/src/tests.rs`（新增 `test_19_5_real_dogfood_fixture_format` 校验 fixture 格式/dim/非全零）、`docs/spikes/phase-19-real-recall.md`（新增真实召回 evidence）、`docs/s2v-adapter.md`（19.5 行 Done）、`docs/specs/tasks/task-19.5-real-recall-eval.md`（本 spec）。注：golden-semantic.jsonl / `internal/eval` 改动**未需要**——复用既有 30 题 golden + file-level Strong-hit@K 口径即可产出可区分真实召回，故未新增（surgical scope）。
- **commit 列表**：见本 task PR（分支 `feat/task-19.5-real-recall-eval`）；合入后以 merge commit 为准
- **§9 Verification 结果**：real `FastEmbedProvider`（all-MiniLM-L6-v2 dim 384）实跑（WSL2 Ubuntu 26.04 / rustc 1.96.0）：`SemanticRecall@5=0.8333 (25/30)`、`SemanticRecall@10=0.9333 (28/30)`、top1=0.6000、MRR=0.7029、gate(≥0.70)=**PASS**；per-category 见 spike doc（config/error/historical/log = 1.0；code-location @5=0.6/@10=0.8；agent-memory-rule @5=0.4/@10=0.8）。`cargo test -p contextforge-bench` 7/7 PASS（含 `test_19_5_real_dogfood_fixture_format`）；本 task 零 Go 改动（`go test ./...` 不退化）；example 默认构建编为 no-op 空 main（`cargo build --example phase19_real_recall` 无 feature 通过，0 新 dep）；D2 lint `--touched master` 0 未标注命中（见下）。CI cargo-test/go-test/spec-lint gate 复核。
- **关键诚实说明（ADR-013）**：首跑（uncapped corpus，124 chunks）得 recall 全 1.0——经判定为**测量 artifact**（`retriever/mod.rs` 39 chunk + `server.rs` 23 chunk 占语料半数，file-level「任一 chunk 入 top-K」被大文件灌成平凡 1.0，与 Phase 18 合成 1.0 同病不同因）。遂 `MAX_CHUNKS_PER_FILE=4` 平衡语料（40 chunk）+ 加 top-1/MRR 区分度指标复跑，得可区分真实值 0.83/0.93/top1 0.60。两跑均为 real fastembed，无伪造；artifact 已诚实修正而非掩盖。
- **剩余风险 / 未做项**：real provider R1 stop **未触发**（fastembed 两平台可构建），故无 deferred 分支。golden-question 语义口径完整版 [SPEC-OWNER:phase-19.golden-questions-full]（现复用 30 题 BM25 口径作 file-level 语义召回，evidence 已标注判定口径）；hybrid BM25+Vector fusion 召回 [SPEC-DEFER:phase-future.hybrid-scoring]；remote embedding provider [SPEC-DEFER:phase-future.embedding-provider-remote]。backend 间排序仍据 Phase 18 latency/RSS 证据（本 recall 不扰 ADR-023 D1-D5）。
- **下游 task 影响**：task-19.6（消费本 task 真实 recall：`@10=0.9333 ≥ 0.70` → ratify ADR-023 D1 recall blocker 清除 + ADR-006 A1 Proposed→转正；记 Phase 18 AC3/AC4 resolved 不溯改 Phase 18 spec）；task-19.7（closeout 引本 evidence 入 v0.12.0 release notes）
