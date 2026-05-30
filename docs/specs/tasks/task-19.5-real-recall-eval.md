# Task `19.5`: `real-recall-eval — 真实 dogfood embedding 语料 + SemanticRecall@5/10 实测 + docs/spikes/phase-19-real-recall.md 喂 ADR-023 ratify`

**Status**: Pending

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

- [ ] **AC1**: `test/fixtures/eval/dogfood-embeddings.jsonl` 由 task-19.1 real provider 实跑生成（覆盖 task-18.8 6 类别 expected 文件 chunk + golden-question 子集），每行 `{"chunk_id, embedding}` 合 `load_dogfood`/`Question` 格式；非 deterministic/合成派生 — verified by **TEST-19.5.1**（fixture 行格式 + provider name/dim 元数据校验 + 非合成来源标注）
- [ ] **AC2**: 真实召回实测 — 经 task-19.2 wiring 跑选定 backend 语义检索，`SummarizeHybrid` 产**真实** `SemanticRecall@5/10`（real provider embedding，非伪造），落 `docs/spikes/phase-19-real-recall.md` — verified by **TEST-19.5.2**（real recall run → evidence 数据 + per-category 分解，标注数据源）
- [ ] **AC3**: gate 对照 — `MeetsRecallGate(report)` 据真实 `SemanticRecallAt10` 对 `GateSemanticRecall10Min=0.70` 出 pass/fail 结论，evidence 如实记（达阈 → 喂 task-19.6 A1 转正；未达 → 记实测维持 provisional），不篡改数 — verified by **TEST-19.5.3**（gate 结论 + 实测值并列于 evidence）
- [ ] **AC4**: ADR-013 诚实分支 — real provider deferred（task-19.1 R1 stop）时本 task 记 blocked + recall ratify defer [SPEC-OWNER:phase-future.embedding-provider-full]，绝不以 deterministic 缺省 provider 派生向量假冒真实召回 — verified by **TEST-19.5.4**（evidence 数据源诚实声明：real-run 数字 或 deferred 说明，二者必居其一且互斥）
- [ ] **AC5**: 既有不退化 — `go test ./...` 全 PASS（含 eval 接真实结果路径）；`cargo test --workspace` 不受影响（real provider feature-gated，默认构建不引入）— verified by **TEST-19.5.5**（`go test ./...` + `cargo test --workspace` 0 failed）+ §10 实测
- [ ] **AC6**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched master` PR 触及行 0 未标注命中 — verified by **TEST-19.5.6**（§10 记录的 D2 lint 实跑输出）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-19.5.1 | dogfood real embedding fixture 行格式 + 非合成来源 | `test/fixtures/eval/dogfood-embeddings.jsonl` | Pending |
| TEST-19.5.2 | 真实 SemanticRecall@5/10 实测 + evidence | `docs/spikes/phase-19-real-recall.md` | Pending |
| TEST-19.5.3 | MeetsRecallGate 对 0.70 阈值结论 + 实测并列 | `docs/spikes/phase-19-real-recall.md` | Pending |
| TEST-19.5.4 | ADR-013 数据源诚实声明（real-run 或 deferred 互斥） | `docs/spikes/phase-19-real-recall.md` | Pending |
| TEST-19.5.5 | go test ./... + cargo test --workspace 0 failed | 全 workspace | Pending |
| TEST-19.5.6 | D2 lint --touched master 0 未标注命中 | `scripts/spec_drift_lint.sh` | Pending |

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

- **完成日期**：（实现后填）
- **改动文件**：`test/fixtures/eval/dogfood-embeddings.jsonl`（新增，real provider 生成）、`test/fixtures/eval/golden-semantic.jsonl`（如 O6 需，新增）、`internal/eval/`（如需接真实检索结果）、`docs/spikes/phase-19-real-recall.md`（新增真实召回 evidence）、`docs/s2v-adapter.md`（19.5 行 Done）、`docs/specs/tasks/task-19.5-real-recall-eval.md`（本 spec）
- **commit 列表**：见本 task PR（分支 `feat/task-19.5-real-recall-eval`）；合入后以 merge commit 为准
- **§9 Verification 结果**：（实现后填）—— real provider 实跑真实 `SemanticRecall@5/10` 填入 `docs/spikes/phase-19-real-recall.md`；`go test ./...` + `cargo test --workspace` 绿；D2 lint 0 命中
- **剩余风险 / 未做项**：（实现后填）—— real provider deferred（task-19.1 R1 stop）分支下真实召回 + ratify 后置 [SPEC-OWNER:phase-future.embedding-provider-full]；golden-question 语义口径完整版 [SPEC-OWNER:phase-19.golden-questions-full]；hybrid fusion 召回 [SPEC-DEFER:phase-future.hybrid-scoring]
- **下游 task 影响**：task-19.6（消费本 task 真实 recall 数据 ratify ADR-023 D1 + ADR-006 A1 转正，或据实测维持 Proposed）；task-19.7（closeout 引本 task evidence）
