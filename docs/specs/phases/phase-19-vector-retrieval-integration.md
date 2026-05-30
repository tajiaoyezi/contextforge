# Phase 19 · vector-retrieval-integration

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。本 phase 把 Phase 18 交付的**向量 backend 基础设施**（trait + 4 backend spike + harness + `SemanticRecall@K` 度量 + ADR-023 Proposed）推进到**生产语义检索**：补 embedding provider、把选定默认 backend 接生产 retriever 热路径、真实召回评测、ratify ADR-023。解决 Phase 18 遗留 [SPEC-OWNER:phase-future.vector-retrieval-integration] + [SPEC-DEFER:phase-future.embedding-provider-full]。v0.12.0 收口。
>
> **入读顺序（必读）**：本 phase spec → `docs/decisions/adr-023-vector-backend-default.md`（Proposed，D1-D6 分层选型）→ `docs/spikes/phase-18-comparison.md`（4 backend 实测）→ `docs/specs/tasks/task-18.8-eval-semantic-recall.md`（SemanticRecall@K 度量）→ `docs/specs/phases/phase-18-vector-backend-selection.md` §6 AC3/AC4 deferred 说明 → AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5，第十次激活）→ `docs/decisions/adr-002-sqlite-tantivy-layered-storage.md` + `docs/decisions/adr-006-recall-eval-acceptance-gate.md`（Amendment A1）+ `docs/decisions/adr-008-core-library-selection.md`（本 phase amend）。
>
> **§2A Decisions Log（待 task-19.1 spike 后锁）**：
> 1. **embedding provider 候选**：fastembed-rs（ort/ONNX）/ candle（HF Rust ML）/ ort 直调，本地嵌入式（PRD §Anti-metrics 本地优先）；task-19.1 spike 在 Linux + Windows MSVC 评估构建/运行/模型获取后选定。
> 2. **deterministic-provider 兜底**：仓内必有一个**无模型依赖**的确定性 provider（hash/seed 派生 embedding），供 CI / smoke / test / 默认构建（0 新 dep）；real provider feature-gated（模型 lazy download）。
> 3. **生产默认 backend**：据 ADR-023 D1/D2 + Windows MSVC 约束（sqlite-vec MSVC 受阻）选定——很可能 **hnsw**（纯 Rust 全平台）或 feature-select；task-19.2 锁。
> 4. **ratify 前提**：ADR-023 Proposed→Accepted **必须**经 task-19.5 真实 embedding recall 数据，禁据合成数据 ratify（ADR-013）。
>
> **ADR 影响面（已识别）**：
> - **ADR-023 ratify**：Proposed → Accepted（task-19.6），据 task-19.5 真实 SemanticRecall@K。
> - **ADR-006 Amendment A1 转正**：SemanticRecall@10 ≥0.70 gate 由 provisional 转 active（若真实数据达阈值；否则记实测值 + 维持 provisional）。
> - **ADR-008 amendment**：embedding provider crate（fastembed/candle/ort）入 Rust 库列表（add-only）。
> - **可能 ADR-015/022 pattern**：若 SearchResponse 加 `vector_score` / `embedding_provider` 字段落生产 → add-only contract 演进（评估 Console 通知）。

## 1. 阶段目标

v0.12.0 ship 后 ContextForge 自带**端到端语义检索**：embedding provider（确定性兜底 + 真实模型 feature-gated）+ 选定默认 backend 接生产 retriever + `/v1/search?semantic=true` 通路 + 真实召回评测 + ADR-023 ratify，把 Phase 18 的「基础设施 + 选型(Proposed)」推到「生产可用 + 选型 ratify」。

**具体可观测的 phase exit criteria（对应 §6 6 条 AC）**：

1. `EmbeddingProvider` trait + deterministic 缺省 provider（默认构建 0 新 dep）+ real provider（feature-gated）落地，spike evidence 文档（AC1）
2. 选定默认 backend 接 `Retriever::with_vector_searcher` 生产热路径 + embedding on index/query，既有 BM25 不退化（AC2）
3. `/v1/search?semantic=true` Go→Rust gRPC 语义检索通路通（字段变更 add-only）（AC3）
4. smoke v9 30-step（既有 28 + semantic search + eval `--semantic`）全 PASS（AC4）
5. 真实 dogfood embedding 语料 `SemanticRecall@K` 实测数据 + ADR-023 ratify（Proposed→Accepted）或据实测 documented 未决（AC5）
6. ADR-014 D1-D5 第十次激活全通过（AC6）

**v0.x 版本号决策**：v0.12.0 minor release（端到端语义检索 ship；默认构建仍 BM25-only baseline——real embedding + 非默认 backend 经 feature opt-in，add-only 不破坏既有）。

## 2. 业务价值

直接对接 PRD §Core Capabilities #1（可解释召回）+ §Success Metrics + Phase 18 遗留：

- **解 Phase 18 AC3/AC4 deferred**：Phase 18 ship 了基础设施但语义搜索未生产化 + ADR-023 未 ratify（合成 recall 不可区分）；本 phase 补 embedding + 生产 wiring + 真实召回 → 闭环。
- **PRD §Success Metrics**：语义召回让自然语言查询从 weak hit 升 strong hit；目标 SemanticRecall@10 ≥70%（task-18.8 gate）；不破坏 P95 < 500ms / idle RSS < 300MB。
- **PRD §Anti-metrics 自觉**：(a) 可解释性 → 语义结果保留 `vector_score` + `embedding_provider` provenance；(b) secret redaction → embedding 输入走 scanner denylist；(c) 本地优先 → 默认 embedding/backend 本地嵌入式,real provider 模型本地,remote opt-in。
- **数据驱动 ratify**：真实 embedding recall 让 ADR-023 选型从架构论据升到实测论据。

**不在本 phase scope**：

- Reranker (cross-encoder) [SPEC-DEFER:phase-future.reranker]
- Hybrid scoring (BM25 + Vector fusion) [SPEC-DEFER:phase-future.hybrid-scoring]——本 phase ship 语义路径单独 + BM25 fallback,fusion 后续
- Remote embedding provider（OpenAI / Cohere）[SPEC-DEFER:phase-future.embedding-provider-remote]——本 phase 仅本地 provider
- CJK + 代码符号 tokenizer [SPEC-DEFER:phase-future.cjk-and-code-tokenizer]
- 多 backend 同时生产可用（仅选定 1 默认）[SPEC-DEFER:phase-future.multi-backend-production]
- Vector index 增量更新 [SPEC-DEFER:phase-future.vector-incremental-index]——承 Phase 18,默认全量 reindex
- Console UI 端语义召回 explain panel（cross-repo Console 领域）

## 3. 涉及模块

### 19.1 embedding provider spike（task-19.1）

- 新增 `core/src/embedding/{mod,traits}.rs`——`EmbeddingProvider` trait（`embed(texts) -> Vec<Vec<f32>>` + `dim()` + `name()`）
- 新增 `core/src/embedding/deterministic.rs`——`DeterministicEmbeddingProvider`（hash/seed 派生固定维度向量,无模型 dep,供 CI/smoke/test,默认构建启用）
- 新增 `core/src/embedding/<chosen>.rs`——real provider（fastembed/candle/ort,feature-gated,模型 lazy load）
- 修改 `core/Cargo.toml`——`embedding-<chosen>` feature + optional dep（默认不启用）
- 新增 `docs/spikes/phase-19-embedding-{candidates,<chosen>}.md`——provider 候选评估 + 选定 evidence（构建/平台/模型/API/真实 embed 样例）
- 同源 `mod tests`（≥3 unit test：deterministic provider 确定性 + dim 一致 + trait 契约）

### 19.2 default backend wiring（task-19.2）

- 修改 `core/src/retriever/mod.rs`——据 ADR-023 选定默认 backend,`with_vector_searcher` 生产接入 + index/query 前过 `EmbeddingProvider`
- 修改 `core/Cargo.toml`——默认 feature 含选定 backend（若 hnsw 纯 Rust 跨平台）或保持 feature-select
- 同源 `mod tests`（≥3：index→search roundtrip via embedding + None fallback BM25 + 选定 backend wiring）

### 19.3 semantic-search API（task-19.3）

- 修改 `proto/`——SearchRequest 加 `semantic bool`（add-only field）+ 可能 SearchResponse/RetrievalResult 加 `vector_score f32` + `embedding_provider string`（add-only provenance）
- 修改 `core/src/...`（Rust gRPC server）——semantic 路径分派到 vector searcher
- 修改 Go 控制面 search handler——`/v1/search?semantic=true` query param → gRPC semantic flag
- 同源 tests（Rust gRPC semantic roundtrip + Go handler param parse）+ contract conformance 不破坏

### 19.4 smoke v9 + eval CLI（task-19.4）

- 修改 `scripts/console_smoke.sh`——v9：28→30 step（step 29 = `/v1/search?semantic=true` roundtrip；step 30 = eval `--semantic`）
- 新增/修改 `cmd/contextforge/eval.go`——CLI `--semantic` flag（接 task-18.8 `SummarizeHybrid` + `MeetsRecallGate`）
- 同源 Go tests（CLI flag parse + eval semantic path）

### 19.5 real-recall eval（task-19.5）

- 新增/扩 `test/fixtures/eval/`——真实 dogfood embedding 语料（real provider 对 dogfood 代码生成 embedding；或 golden questions 子集）
- 修改 `internal/eval/`（如需 runner 接真实检索结果）+ 跑 `SemanticRecall@K` 实测
- 新增 `docs/spikes/phase-19-real-recall.md`——真实 SemanticRecall@5/10 数据（feed ADR-023 ratify）
- O6 golden questions 完整版（如本 phase 需）[SPEC-OWNER:phase-19.golden-questions-full]（承 Phase 18 §不在 scope 的 forward-ref）

### 19.6 ADR-023 ratify（task-19.6）

- 修改 `docs/decisions/adr-023-vector-backend-default.md`——据 task-19.5 真实 recall:Status Proposed → Accepted（或记实测维持 Proposed + 文档化未决）
- 修改 `docs/decisions/adr-006-recall-eval-acceptance-gate.md`——Amendment A1 SemanticRecall gate provisional → active（若达阈值,add-only）
- 修改 `docs/decisions/adr-008-core-library-selection.md`——embedding provider crate add-only
- closeout 段记 Phase 18 §6 AC3/AC4 已解决（**不溯改 Phase 18 spec,D5**）

### 19.7 收口 v0.12.0（task-19.7）

- 修改 `scripts/console_smoke.sh`（v9 final）+ 新增 `docs/releases/v0.12.0-{evidence,artifacts}.md`
- 修改 `README.md`（v0.12 Quick start 加语义召回 example `contextforge search --semantic "<query>"`）+ `RELEASE_NOTES.md` v0.12.0 段
- 修改 `docs/s2v-adapter.md`（§Phase 索引 Phase 19 Draft → Done + Tasks 0 → 7；§ADR 索引 ADR-023 Accepted；18.x forward-ref 解除）
- v0.12.0 tag push（**经用户授权**）→ release.yml ghcr 镜像 + post-tag-push backfill

### BDD feature

- 新增 `test/features/phase-19-vector-retrieval-integration.feature`（≥3 scenario：embedding provider 确定性 / 默认 backend semantic search roundtrip / eval semantic recall gate）
- 修改 `docs/s2v-adapter.md` §BDD Feature 索引 追加行

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 19.1 | `core/src/embedding/{mod,traits,deterministic,<chosen>}.rs` + EmbeddingProvider trait + spike evidence | `../tasks/task-19.1-spike-embedding-provider.md` |
| 19.2 | `core/src/retriever/mod.rs` default backend wiring + embedding on index/query | `../tasks/task-19.2-default-backend-wiring.md` |
| 19.3 | proto SearchRequest semantic flag + Rust gRPC semantic path + Go `/v1/search?semantic=true` | `../tasks/task-19.3-semantic-search-api.md` |
| 19.4 | `scripts/console_smoke.sh` v9 30-step + `cmd/contextforge/eval.go` `--semantic` CLI | `../tasks/task-19.4-smoke-v9.md` |
| 19.5 | real dogfood embedding corpus + SemanticRecall@K 实测 + `docs/spikes/phase-19-real-recall.md` | `../tasks/task-19.5-real-recall-eval.md` |
| 19.6 | ADR-023 Proposed→Accepted + ADR-006 A1 转正 + ADR-008 amend + Phase 18 AC3/AC4 解决记录 | `../tasks/task-19.6-adr-023-ratify.md` |
| 19.7 | Phase 19 closeout + v0.12.0 release docs + tag + backfill | `../tasks/task-19.7-closeout-v0.12.0.md` |

## 5. 依赖关系

- **task-19.1**（embedding provider）= 首项,解锁 19.2（wiring 需 embedding）。
- **task-19.2**（backend wiring）dep 19.1 + ADR-023 D1/D2;解锁 19.3。
- **task-19.3**（semantic API）dep 19.2;解锁 19.4。
- **task-19.4**（smoke v9）dep 19.3;**task-19.5**（real recall）dep 19.1 real provider + 19.2 wiring。
- **task-19.6**（ratify）dep 19.5 真实数据;**task-19.7**（closeout）dep 19.1-19.6 全 Done。
- 外部:ADR-023（Phase 18 Proposed）/ ADR-006 Amendment A1 / ADR-014 第十次激活 / task-18.1 trait seam / task-18.8 度量。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（C1 集成兜底门强制；每条 AC 含 ADR-014 D3 verified by 显式 owner）**：

- [ ] **AC1**：`EmbeddingProvider` trait + `DeterministicEmbeddingProvider`（无模型,默认构建 0 新 dep,≥3 unit test）+ real provider（feature-gated,模型 lazy）落地;spike evidence `docs/spikes/phase-19-embedding-*.md` — verified by task-19.1 §6 AC1-3 + phase-smoke step 1
- [ ] **AC2**：选定默认 backend 接 `Retriever::with_vector_searcher` 生产热路径 + index/query embedding;既有 BM25 检索不退化（`cargo test --workspace` 0 failed）— verified by task-19.2 §6 AC1-2 + phase-smoke step 2
- [ ] **AC3**：`/v1/search?semantic=true` Go→Rust gRPC 语义通路通;proto 字段变更 add-only（contract conformance 不破坏）— verified by task-19.3 §6 AC1-2 + phase-smoke step 3
- [ ] **AC4**：smoke v9 30-step（既有 28 不退化 + step 29 semantic search + step 30 eval `--semantic`）全 PASS — verified by task-19.4 §6 AC1 + phase-smoke step 4
- [ ] **AC5**：真实 dogfood embedding `SemanticRecall@K` 实测 + ADR-023 ratify（Proposed→Accepted）或据实测 documented 未决（禁据合成 ratify,ADR-013）— verified by task-19.5 §6 AC1-2 + task-19.6 §6 AC1
- [ ] **AC6**：ADR-014 cross-validation gate 全套通过（第十次激活）— D1 mapping table + D2 lint `--touched master` 0 unannotated hits + D3 verified-by 显式 owner + D4 主 agent 自治 + D5 历史 Phase 1-18 不溯改 — verified by task-19.7 closeout PR body

**端到端 smoke（6 step，C1 集成兜底）**：(1) deterministic embedding provider unit;(2) index→semantic search roundtrip via 默认 backend;(3) `/v1/search?semantic=true` gRPC;(4) smoke v9 30-step;(5) real-recall eval SemanticRecall@K;(6) D2 lint 0 hits。

## 7. 阶段级风险

- **R1（高）embedding provider 平台/模型门槛**：fastembed(ort)/candle native + 模型下载在 Windows MSVC / CI 受阻。
  - **缓解**：deterministic provider 兜底（无模型,默认构建,供 CI/smoke）;real provider feature-gated（dev/Linux 跑真实 recall）。stop-condition：两平台均不可构建 real provider → deterministic 跑通 wiring/smoke（标注）,真实 recall + ADR ratify defer,继续其余 task。
- **R2（中）合成 recall 不可区分（承 Phase 18）**：必须真实 embedding 才能 ratify ADR-023。
  - **缓解**：task-19.5 用 real provider + dogfood 真实语料;ADR-013 禁据合成 ratify。
- **R3（中）proto/contract 演进**：SearchResponse 加字段需 add-only + 可能 Console 协同。
  - **缓解**：仿 ADR-015/022 add-only pattern;conformance test 守 22-endpoint 不破坏;cross-repo 通知评估。
- **R4（低）默认 backend 持久化**：hnsw 内存图无持久化（Phase 18 记 28s 重建）。
  - **缓解**：生产 wiring 评估持久化或 rebuild-on-load;[SPEC-DEFER:phase-future.hnsw-graph-persistence] 承 Phase 18。

## 8. Definition of Done

- 7 task spec（19.1-19.7）顶部 `**Status**: Done`
- §6 阶段级 AC1-6 全 `[x]`（AC5 含 ratify 结论或 documented 未决——若 embedding 受阻则诚实缩范围,仿 Phase 18 closeout pattern）
- 端到端 smoke 6 step 全 PASS
- **ADR**：ADR-023 `Proposed → Accepted`（或据实测维持 + 未决记录）+ ADR-006 A1 转正 + ADR-008 amend（add-only）
- **adapter**：§Phase 索引 Phase 19 `Draft → Done` + `Tasks 0 → 7`;§ADR 索引 ADR-023 状态更新;§BDD 追加 phase-19 feature 行;Phase 18 forward-ref（phase-future.vector-retrieval-integration / embedding-provider-full）解除
- **spike evidence**：`docs/spikes/phase-19-{embedding-*,real-recall}.md`
- **release**：`docs/releases/v0.12.0-{evidence,artifacts}.md` + `RELEASE_NOTES.md` v0.12 段 + README v0.12 段
- **cross-repo follow-up**（task-19.6/19.7 §10）：如 SearchResponse 加字段 → 评估通知 Console
