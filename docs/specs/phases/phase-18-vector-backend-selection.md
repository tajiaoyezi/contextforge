# Phase 18 · vector-backend-selection

**Status**: Ready

> Phase Spec（s2v full-standard §8.2）。本 phase 解决 PRD §Open Questions **O2 向量后端最终选型**（源自 D2 / 技术 TBD：SQLite vec ext / Qdrant local / LanceDB / 内嵌 HNSW，需核心开发在 Phase 5-6 期间做 spike 压测后定）。v0.11.0 收口。
>
> **入读顺序（必读）**：本 phase spec → `docs/prds/context-forge.prd.md` §Open Questions O2 + §Decisions Log D2 + §Constraints Performance/Compatibility + §Success Metrics + §Anti-metrics → AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（自 Phase 10 起本 phase 必须激活 D1-D5 cross-validation gate，第九次激活）→ `docs/decisions/adr-002-sqlite-tantivy-layered-storage.md` + `docs/decisions/adr-006-recall-eval-acceptance-gate.md` + `docs/decisions/adr-008-core-library-selection.md`（本 phase 可能 amendment）。
>
> **§2A Decisions Log（已锁，2026-05-30 主 agent + 用户确认）**：
> 1. **候选集**：锁 4 路 — SQLite vec ext / Qdrant local (embedded) / LanceDB / 内嵌 HNSW (hnswlib-rs 或 instant-distance) — 按 PRD D2 原貌
> 2. **评测口径**：**must** = `recall@5` + `recall@10` + `P95 latency` + `单机内存 RSS`（idle + 索引时）；**nice-to-have** = `cold-start time` + `索引重建耗时`
> 3. **判据排序**：性能 (recall + P95) > 单机资源 (RSS) > 嵌入门槛 (依赖/编译复杂度) > 单文件可移植 — 锚 PRD §Anti-metrics 「不能为速度牺牲可解释性 / secret redaction / 本地优先」
> 4. **集成深度**：**trait-first** — 18.1 冻结 `Vector{Backend,Indexer,Searcher}` 三 trait + `NoopVectorBackend` 占位 → 18.2 spike harness → 18.3-18.6 四 backend 实现并行 → 18.7 决策 ADR-023 → 18.8 eval 接入 → 18.9 收口
> 5. **数据集来源**：合成 100k chunk + ContextForge 自身代码仓 dogfood corpus — O6 golden questions 完整版（≥30 条 6 类）独立留 Phase 19，本 phase **不**依赖 O6
>
> **ADR 影响面（已识别）**：
> - **新增 ADR-023**：spike 决策结论 — 选定默认 backend + 4 路 trade-off matrix；task-18.7 ship 时 Proposed → Accepted（收口 PR）
> - **可能 ADR-002 amendment**：若选定 backend 引入新存储层（如 LanceDB Lance file format 与 ADR-002 SQLite+Tantivy 分层冲突）→ 走 ADR-002 amendment 路径（仿 ADR-022 amend ADR-015 D5 pattern）
> - **可能 ADR-008 amendment**：default backend 选定后加 Cargo dep → ADR-008 §Rust 库列表追加
> - **可能 ADR-006 amendment**：recall-eval-acceptance-gate 由 BM25-only 扩为 BM25 + Semantic → ADR-006 §Acceptance Threshold 子节追加 SemanticRecall@K 阈值

## 1. 阶段目标

v0.11.0 ship 后 ContextForge 自带**向量召回 trait 抽象层** + **1 个 spike 数据驱动选定的默认 backend 实现** + **4 backend spike evidence 文档** + **recall eval gate 含语义召回路径**，从根本上解 PRD §Open Questions O2 + D2「provider 抽象，v0.1 不强依赖」从抽象推到实装。

**具体可观测的 phase exit criteria（对应 §6 6 条 AC）**：

1. `core/src/retriever/vector/` 三 trait + NoopVectorBackend 落地 ✅ 既有 BM25 检索不退化（AC1）
2. `scripts/spike_vector_backends.sh` + `bench/` 跑通 4 backend × 5 维测量 → `docs/spikes/phase-18-{sqlite-vec,qdrant-embedded,lancedb,hnsw}.md` 4 份 evidence（AC2）
3. `docs/decisions/adr-023-<chosen-backend>-default.md` Status: Accepted + 含 4 backend trade-off matrix（AC3）
4. 默认 backend retriever 端集成 + smoke v9 30-step (既有 28 + 2 vector search step)（AC4）
5. `internal/eval/eval.go` SemanticRecall@K + recall gate D6 阈值（AC5）
6. ADR-014 D1-D5 第九次激活全通过（AC6）

**v0.x 版本号决策**：v0.11.0 minor release（含 trait 抽象 + 1 backend default + eval 接入；breaking change risk 仅在 ADR-006 amendment 时严格判定 — 默认 backend 添加为 add-only 不破坏 BM25-only baseline）。

## 2. 业务价值

直接对接 PRD §Core Capabilities #1（可解释召回）+ §Success Metrics 主指标 + §Decisions Log D2：

- **PRD §Success Metrics 主指标提升**：Top-5 ≥ 75% / Top-10 ≥ 85% 现有 BM25 baseline 满足；加入语义召回后，跨语义近邻问题（"如何配置 X" / "X 报错怎么办" 等自然语言查询）召回从 weak hit 提升到 strong hit ≥10% 绝对值；预期 SemanticRecall@10 ≥70% (Hybrid 后 Top-10 ≥90%)
- **PRD §Success Metrics 工程指标维持**：本 phase 不破坏 P95 < 500ms / idle RSS < 300MB 硬约束；spike 判据 #2 即此
- **PRD D2「provider 抽象，v0.1 不强依赖」延伸**：v0.1-v0.10 抽象层留白，本 phase ship trait + 1 默认实现，把 "抽象但不实装" 推到 "trait 已冻结 + 1 默认 ship + 其他 backend swap-in friendly"
- **跨 backend swap-in friendly**：trait-first 决策让未来用户/团队按场景 swap default backend（嵌入式选 SQLite vec / 性能优先选 LanceDB / 已部署 Qdrant 仓库选 qdrant-client remote 模式）无 retriever 端代码改动
- **PRD §Anti-metrics 自觉**：本 phase 严守 3 条反指标 — (a) recall 提升不能牺牲可解释性 → vector search 结果 schema 保留 `vector_score` + `embedding_provider` provenance 字段；(b) 不能牺牲 secret redaction → embedding 输入路径走 task-2.1 scanner denylist + secret 检测；(c) 不能牺牲本地优先 → 默认 backend 必须本地嵌入式（remote provider opt-in 严格保留）

**不在本 phase scope**：

- CJK + 代码符号 tokenizer（O11）[SPEC-DEFER:phase-future.cjk-and-code-tokenizer]
- O6 golden questions 完整版（≥30 条 6 类）[SPEC-DEFER:phase-19.golden-questions-full] — Phase 18 用合成 + dogfood corpus 双轨即够
- Reranker (cross-encoder) [SPEC-DEFER:phase-future.reranker]
- Hybrid scoring (BM25 + Vector fusion) [SPEC-DEFER:phase-future.hybrid-scoring] — Phase 18 仅 ship vector 路径单独，hybrid 留后续
- Console UI 端语义召回 explain panel visual changes（cross-repo Console 主仓领域）
- Multi-collection vector index（跨 collection 共享 vector index）[SPEC-DEFER:phase-future.multi-collection-vector-index]
- Vector index incremental update（增量更新策略）[SPEC-DEFER:phase-future.vector-incremental-index] — Phase 18 spike 默认全量 reindex
- Embedding provider abstraction 完整化（OpenAI / Cohere / local sentence-transformers）[SPEC-DEFER:phase-future.embedding-provider-full] — Phase 18 spike 仅依赖 1 个 local embedding provider（默认 fastembed-rs 或 candle 本地 ONNX）

## 3. 涉及模块

### 18.1 vector retrieval trait（task-18.1）

- 新增 `core/src/retriever/vector/mod.rs`（pub mod traits + noop）
- 新增 `core/src/retriever/vector/traits.rs`（`VectorBackend` / `VectorIndexer` / `VectorSearcher` 三 trait）
- 新增 `core/src/retriever/vector/noop.rs`（`NoopVectorBackend` 占位实现，返空 hits + log warning）
- 修改 `core/src/retriever/mod.rs`（接入 vector backend `Option<Arc<dyn VectorSearcher>>` 字段；默认 `None` → BM25-only 路径不退化）
- 修改 `core/Cargo.toml`（workspace 加 `[features] default = []; vector-spike = ["dep:..."]`，spike crate optional）
- 新增 `core/src/retriever/vector/tests.rs` 或同源 `mod tests`（NoopVectorBackend + trait 契约 ≥3 unit test）
- 不引入任何真 backend dep

### 18.2 spike harness（task-18.2）

- 新增 `bench/` Rust crate（cargo workspace 新 member；含 main.rs + lib.rs；不入 core/）
- 新增 `bench/src/{corpus,measure,backends,runner}.rs`（corpus 加载 + 5 维测量 + backend trait runner + summary writer）
- 新增 `scripts/spike_vector_backends.sh`（shell wrapper：gen corpus → run bench → write evidence md）
- 新增 `tools/gen_synthetic_corpus.sh`（合成 100k chunk fixture 生成 — embedding 用 deterministic seed 保 reproducible）
- 新增 `test/fixtures/spike/synthetic-100k.jsonl.gz`（gen 完后 commit 入 fixture；< 50MB 可接受；超 50MB 转 git-lfs 或 gen-on-demand）
- 新增 `test/fixtures/spike/dogfood-contextforge.jsonl`（ContextForge 自身代码仓索引 dump；< 5MB 直 commit）
- evidence template `docs/spikes/_template.md`（5 维 measurement schema + trade-off discussion 段）

### 18.3-18.6 backend spike（4 task 并行）

- task-18.3: `core/src/retriever/vector/sqlite_vec.rs`（sqlite-vec ext loadable extension）+ `core/Cargo.toml [dependencies] sqlite-vec = { version = "*", optional = true }` + `core/migrations/0018_vector_index.sql`（可选）
- task-18.4: `core/src/retriever/vector/qdrant_embedded.rs`（`qdrant-client` embedded mode 或 `qdrant_segment` 直接库调用）+ Cargo dep
- task-18.5: `core/src/retriever/vector/lancedb.rs`（`lancedb` Rust crate）+ Cargo dep + `core/data/vectors/` Lance file path
- task-18.6: `core/src/retriever/vector/hnsw.rs`（`instant-distance` 或 `hnsw_rs` 嵌入式 HNSW + 自定义 SQLite metadata 持久化）+ Cargo dep
- 每 task 同源 `mod tests`（≥3 unit test：build / search / persistence reload）
- 每 task evidence `docs/spikes/phase-18-<backend>.md`（5 维测量 + 排除/选定理由 + Open Questions）

### 18.7 decision ADR-023（task-18.7）

- 新增 `docs/decisions/adr-023-<chosen-backend>-default.md`（s2v full-standard §16.2 模板 + 4 backend trade-off matrix + 选定理由 + 排除理由 + 反对意见预期）
- 修改 `core/src/retriever/mod.rs`（默认 `Some(<chosen>)` wiring + feature flag `default = ["vector-<chosen>"]`）
- 可能修改 `docs/decisions/adr-002-sqlite-tantivy-layered-storage.md`（amendment 段加 vector layer）
- 可能修改 `docs/decisions/adr-008-core-library-selection.md`（Rust 库列表加 chosen backend crate）
- 修改 `docs/s2v-adapter.md` §ADR 索引（追加 ADR-023 行 + 可能 ADR-002/008 备注 amended-by-ADR-023）
- 修改 `docs/prds/context-forge.prd.md` §Decisions Log（D2 行追加 resolved by ADR-023 引用）

### 18.8 eval semantic recall（task-18.8）

- 修改 `internal/eval/eval.go`（`EvalResult` struct 加 `SemanticStrongHits int / SemanticWeakHits int / SemanticMisses int`；harness 跑 BM25 + Vector 两路计算分别 hit rate）
- 修改 `internal/eval/eval_test.go`（≥3 unit test：SemanticRecall@K 计算 + 阈值断言 + 空 vector backend fallback BM25-only）
- 修改 `cmd/contextforge/eval.go`（CLI flag `--semantic [bool]` default true 当 vector backend 已配置；false 时 BM25-only baseline）
- 可能修改 `docs/decisions/adr-006-recall-eval-acceptance-gate.md`（amendment 段加 SemanticRecall@10 ≥70% gate）
- 修改 `scripts/release_smoke.sh`（加 `phase18_vector_backend_selection=ok` 子段 — curl `/v1/search?semantic=true&q=<test-query>` 实测）

### 18.9 收口（task-18.9）

- 修改 `scripts/console_smoke.sh`（v9：28 step → 30 step；step 29 = vector search via `/v1/search?semantic=true`；step 30 = `/v1/eval/run --semantic` smoke）
- 新增 `docs/releases/v0.11.0-evidence.md` + `docs/releases/v0.11.0-artifacts.md`
- 修改 `docs/prds/context-forge.prd.md`（§Implementation Phases 加 Phase 18 段落；§Open Questions O2 标 `[x] Resolved by Phase 18 closeout`）
- 修改 `docs/s2v-adapter.md`（§Phase 索引 Phase 18 Draft → Done + Tasks 0 → 9）
- 修改 `README.md`（v0.11 Quick start 段加语义召回 example）

### BDD feature

- 新增 `test/features/phase-18-vector-backend-selection.feature`（≥3 scenario：trait 抽象层 / spike harness 跑通 / 默认 backend semantic search roundtrip）
- 修改 `docs/s2v-adapter.md` §BDD Feature 索引 追加行

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 18.1 | `core/src/retriever/vector/{mod,traits,noop}.rs` + retriever wiring + NoopVectorBackend | `../tasks/task-18.1-vector-trait.md` |
| 18.2 | `bench/` crate + `scripts/spike_vector_backends.sh` + synthetic-100k + dogfood corpus | `../tasks/task-18.2-spike-harness.md` |
| 18.3 | `core/src/retriever/vector/sqlite_vec.rs` + `sqlite-vec` Cargo dep + spike evidence | `../tasks/task-18.3-spike-sqlite-vec.md` |
| 18.4 | `core/src/retriever/vector/qdrant_embedded.rs` + `qdrant-*` Cargo dep + spike evidence | `../tasks/task-18.4-spike-qdrant-embedded.md` |
| 18.5 | `core/src/retriever/vector/lancedb.rs` + `lancedb` Cargo dep + spike evidence | `../tasks/task-18.5-spike-lancedb.md` |
| 18.6 | `core/src/retriever/vector/hnsw.rs` + `instant-distance`/`hnsw_rs` Cargo dep + spike evidence | `../tasks/task-18.6-spike-hnsw.md` |
| 18.7 | `docs/decisions/adr-023-<chosen>-default.md` + default wiring + ADR-002/006/008 amendment | `../tasks/task-18.7-decision-adr023.md` |
| 18.8 | `internal/eval/eval.go` SemanticRecall@K + CLI `--semantic` flag + recall gate | `../tasks/task-18.8-eval-semantic-recall.md` |
| 18.9 | `scripts/console_smoke.sh` v9 + RELEASE_NOTES v0.11.0 + closeout（含 ADR-014 D1/D2/D3/D5）| `../tasks/task-18.9-release-v0.11.0-closeout.md` |

**Phase 内推荐序**：

```text
18.1 (trait 冻结，1 PR ~3 天) → 18.2 (harness 就绪，1 PR ~3 天) → {18.3, 18.4, 18.5, 18.6} ∥ (4 backend spike，可并行，~4-6 天) → 18.7 (decision + 默认 wiring + ADR amendments，1 PR ~2 天) → 18.8 (eval 接入，1 PR ~2 天) → 18.9 (收口 + release prep，1 PR ~2 天)

总预估 ~18-21 天（含 review + 修 backlog；4 backend 并行节省 ~6 天）
```

**并发 worktree 规划**：

- Phase 18 worktree: `../ContextForge-wt-vector-backend-selection`（主 agent 手动建，承载 18.1/18.2/18.7/18.8/18.9 串行 task）
- 4 backend spike task 各独立 task worktree（Agent tool `isolation: "worktree"` 自动建 `../ContextForge-wt-task-18.{3,4,5,6}`）
- 串行锁：18.7 改 `core/src/retriever/mod.rs` default wiring 时所有 18.3-18.6 必须 merged（无 backend 实施未 merge）

## 5. 依赖关系

- **依赖**（必须 merged 才能启动 Phase 18 task 实施）：
  - Phase 4（retrieval-explain）— Done ✅ — 复用 retriever 抽象与 result schema
  - Phase 6（cli-api-export）— Done ✅ — 复用 daemon `/v1/search` REST 端点
  - Phase 8（eval-and-reliability）— Done ✅ — 复用 `internal/eval/eval.go` recall harness 框架
  - ADR-002 sqlite-tantivy-layered-storage — Accepted（本 phase 可能 amendment）
  - ADR-006 recall-eval-acceptance-gate — Accepted（本 phase 可能 amendment）
  - ADR-008 core-library-selection — Accepted（本 phase 可能 amendment）
  - ADR-014 cross-phase-exit-criteria-validation — Accepted（第九次激活）
  - PRD §Open Questions O2 — 本 phase 直接 resolve
- **不依赖**（可后置）：
  - O6 golden questions 完整版 — [SPEC-DEFER:phase-19.golden-questions-full]；Phase 18 用合成 + dogfood 双轨即够
  - O11 CJK + 代码符号检索 — [SPEC-DEFER:phase-future.cjk-and-code-tokenizer]
  - ContextForge-Console UI 端语义召回显示 — cross-repo follow-up；Console 团队后续 ship
- **可并行外部信号**：无（本 phase 不需 cross-repo 触发，区别 Phase 17 ADR-022 D5 模式）

**Phase 内推荐序**（重申）：trait 冻结 (18.1) → harness 就绪 (18.2) → 4 spike 任一序 (18.3-18.6 并行) → decision (18.7) → eval 接入 (18.8) → 收口 (18.9)

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（C1 集成兜底门强制；每条 AC 含 ADR-014 D3 verified by 显式 owner）**：

- [ ] **AC1**：vector retrieval trait 抽象层 ship — `VectorBackend` / `VectorIndexer` / `VectorSearcher` 三 trait 落地 `core/src/retriever/vector/`（mod.rs + traits.rs + noop.rs），含 `NoopVectorBackend` 占位实现 ≥3 unit test PASS + 既有 BM25 检索不退化（`cargo test --workspace` 0 failed）— verified by task-18.1 §6 AC1-3（`core/src/retriever/vector/traits.rs` + `core/src/retriever/vector/tests.rs`） + phase-smoke step 1
- [ ] **AC2**：spike harness 跑通 4 backend × 5 维 — 合成 100k chunk + ContextForge dogfood 双数据集；recall@5/10 + P95 + RSS + cold-start + 索引重建耗时 5 维测量；4 backend 各 1 次完整跑通 evidence 落 `docs/spikes/phase-18-{sqlite-vec,qdrant-embedded,lancedb,hnsw}.md` — verified by task-18.2 §6 AC1-2（harness 实现）+ task-18.3/4/5/6 §6 AC1 各（backend spike evidence）+ phase-smoke step 2
- [ ] **AC3**：spike 决策落 ADR-023 — Status: Proposed → Accepted at closeout PR；含 4 backend trade-off matrix（5 维实测对比表）+ 选定的默认 backend 名称 + 排除理由 + 反对意见预期 + 必要时 ADR-002/006/008 amendment 段 — verified by task-18.7 §6 AC1-2 + closeout PR diff 含 ADR-023 + ADR amendment 行变更
- [ ] **AC4**：默认 backend 集成实现 + retriever 端集成 + smoke v9 — `core/src/retriever/mod.rs` default `Some(<chosen>)` wiring；smoke v9 30-step（既有 28 step Phase 16/17 不退化 + step 29-30 加入 vector search roundtrip via `/v1/search?semantic=true`）全 PASS — verified by task-18.9 §6 AC1（smoke v9 实现）+ task-18.7 §6 AC3（default wiring） + phase-smoke step 3
- [ ] **AC5**：eval harness 语义召回评测纳入 — `internal/eval/eval.go` 加 `SemanticRecall@K` 指标 (K=5,10) + CLI `--semantic` flag default true when vector backend configured + recall gate 含 SemanticRecall@10 ≥ 70% D6 阈值 — verified by task-18.8 §6 AC1-2（eval.go 实现 + CLI flag） + phase-smoke step 4
- [ ] **AC6**：ADR-014 cross-validation gate 全套通过（第九次激活）— D1 mapping table (Phase §6 6 条 ↔ Task §6 AC) + D2 lint `scripts/spec_drift_lint.sh --touched origin/master` 0 unannotated hits + D3 verified-by 显式 owner + D4 主 agent 自治补丁 + D5 历史 Phase 1-17 不溯改 — verified by task-18.9 closeout PR body 含 D1 mapping 表 + D2 lint 输出段 + D3 § 6 AC 全含 `verified by ...` + D5 git diff 仅触新加文件

**端到端 smoke**：

```bash
# step 1 — vector trait + cargo test 不退化
cargo test --workspace
# 期望：既有测试通过 + 新 vector trait 测试 PASS + Noop backend 测试 PASS

# step 2 — spike harness 跑通 4 backend
bash scripts/spike_vector_backends.sh
# 期望：4 份 evidence 落 docs/spikes/phase-18-{sqlite-vec,qdrant-embedded,lancedb,hnsw}.md
# 每份含 5 维测量结果表 + trade-off discussion 段

# step 3 — default backend 集成 smoke v9
bash scripts/console_smoke.sh
# 期望：30 step 全 PASS（含 28 step Phase 17 baseline 不退化 + step 29 vector search + step 30 eval semantic）

# step 4 — eval harness 语义召回评测
contextforge eval run --golden test/fixtures/spike/dogfood-contextforge.jsonl --semantic
# 期望：报告含 SemanticRecall@5 / @10 + 阈值 ≥70% PASS

# step 5 — D2 lint (ADR-014)
bash scripts/spec_drift_lint.sh --touched origin/master
# 期望：0 unannotated hits

# step 6 — release smoke
bash scripts/release_smoke.sh
# 期望：v0.11.0 prep ok + phase18_vector_backend_selection=ok 子段
```

step 1-4 是 task-18.9 phase smoke 入口；step 5-6 是 closeout PR gate。

## 7. 阶段级风险

- **R1（高）**：sqlite-vec ext 跨平台编译困难 — Linux x86_64 .so / macOS arm64 .dylib / Windows .dll 二进制需各自编译；PRD §Constraints Supported platforms 锚 P0 = Linux x86_64 / WSL2，macOS arm64 + Windows v0.3 preview
  - **缓解**：spike 阶段先 Linux x86_64 落地数据（满足 P0 平台）；跨平台留 task-18.7 §3 OOS 显式声明 [SPEC-DEFER:phase-future.sqlite-vec-cross-platform]；如 spike 显示 sqlite-vec 是唯一胜出 backend → 评估转 LanceDB / HNSW 作 P0 默认
- **R2（高）**：Qdrant embedded 资源占用违反 PRD `idle <300MB` — Qdrant 历史是 service-oriented，嵌入式模式 (qdrant_segment) 是新路径
  - **缓解**：spike 18.4 测得 idle RSS 若 > 300MB → 18.7 决策时直接排除 Qdrant；不必勉强保留
- **R3（中）**：LanceDB Rust 原生但社区生态不如 Qdrant 成熟 — API 不稳定 / 文档稀缺 / 跨 minor 版本破坏
  - **缓解**：Cargo.lock 锁版本 + spike 18.5 evidence 记录 API 稳定度 + Lance file format 跨版本兼容性测试（升 lancedb crate 重读旧 Lance file 验证）
- **R4（中）**：内嵌 HNSW (instant-distance / hnsw_rs) 持久化机制弱 — HNSW 内存图结构默认无持久化层，spike 实现需自定义 SQLite metadata blob 持久化
  - **缓解**：spike 18.6 评测 reindex 成本（从 SQLite metadata 复活 HNSW 图）；纳入 cold-start 维度（nice-to-have 排序低）
- **R5（中）**：dogfood corpus 偏小 — ContextForge 自身约 1500 文件、3000 chunk，<1% of 100k；可能不够测出 backend 在大 chunk 下的真实差异
  - **缓解**：合成 100k 测 P95/RSS（量级敏感维度） + dogfood 测 recall（真实分布敏感维度）双轨互补；spike evidence 明确分维度数据来源
- **R6（中）**：trait 抽象层动态分派性能损耗 — `Arc<dyn VectorSearcher>` vs 静态泛型 `<T: VectorSearcher>` 在 hot path 上有 ~3-5% 损耗
  - **缓解**：bench 18.2 同时 measure trait 路径 vs 直接调用 baseline；如 ≥5% 损耗 → 18.7 决策时评估转 enum-based static dispatch（pattern：`enum AnyBackend { SqliteVec(SqliteVecBackend), Lance(LanceBackend), ... }`）
- **R7（中）**：4 backend Cargo dep 引入 lockfile 膨胀 + 编译时间增长 — 全 features 编译 v0.11 可能比 v0.10 多 2-3 倍
  - **缓解**：feature flag `[features] default = ["vector-<chosen>"]` 让 spike crate optional；release build 仅含 default backend；spike crate 仅 dev-build 用
- **R8（中）**：ADR-002 / ADR-008 / ADR-006 amendment 跨多 ADR 串行化复杂度 — 4 ADR 同时变更易遗漏依赖
  - **缓解**：task-18.7 一次性 PR 收口所有 ADR 变更（含 ADR-023 新增 + ADR-002/006/008 amendment 段），avoid 分散 PR
- **R9（低）**：local embedding provider 选择延迟 phase 18 启动 — 当前 ContextForge 无生产环境 embedding provider；spike 需 fastembed-rs（本地 ONNX）或 candle（本地 transformer）
  - **缓解**：task-18.2 §3 把 embedding provider 选择 + dep 安装作为 18.2 第 1 子步；spike harness 自带 fixed embedding provider 减外部变量；embedding provider 完整抽象留 [SPEC-DEFER:phase-future.embedding-provider-full]
- **R10（低）**：v0.11 ship 时间线 — 4 backend spike + decision + integration 18-21 天预估若 spike 全失败（4 路全不满足判据）→ 退化为 D2 "provider 抽象但不实装" + 18.7 撤回为 ADR-023 "继续观察"
  - **缓解**：18.7 决策 PR 准入条件文档化 — 至少 1 backend 满足 P95<500ms + RSS<300MB + recall@10 ≥70% 3 维同时达标；若 4 路全失败 → §8 卡住协议触发，启动 user 决策路径（可能拓展候选 / 调整阈值 / 延期 v0.11 minor 转 v0.12）

## 8. Definition of Done

- 9 task spec（18.1-18.9）顶部 `**Status**: Done`
- §6 阶段级 AC1-6 全 `[x]`
- 端到端 smoke 6 step 全 PASS（cargo test / spike harness / smoke v9 / eval semantic / D2 lint / release smoke）
- **ADR**：
  - ADR-023 `Status: Proposed → Accepted`（spike 决策落地）
  - 必要时 ADR-002 / ADR-006 / ADR-008 amendment 段已 ship
- **PRD**：
  - `§Open Questions O2` 标 `[x] Resolved by Phase 18 closeout`
  - `§Implementation Phases` 加 Phase 18 段落
  - `§Decisions Log D2` 行追加 `resolved by ADR-023` 引用
- **adapter**：
  - `§Phase 状态索引` Phase 18 `Status: Draft → Done` + `Tasks: 0 → 9`
  - `§Task 总索引` 9 行追加（task-18.1-18.9 全 Status: Done）
  - `§ADR 索引` 追加 ADR-023 + ADR-002/006/008 amendment 注
  - `§BDD Feature 索引` 追加 `phase-18-vector-backend-selection.feature` 行
- **spike evidence**：`docs/spikes/phase-18-{sqlite-vec,qdrant-embedded,lancedb,hnsw}.md` ≥4 份 + `docs/spikes/_template.md`
- **release**：`docs/releases/v0.11.0-evidence.md` + `docs/releases/v0.11.0-artifacts.md` + `RELEASE_NOTES.md` v0.11 段
- **README**：Quick start 加语义召回 example（`contextforge search --semantic "<query>"`）
- **cross-repo follow-up**（task-18.9 §10）：
  - 如 SearchResponse 加 `vector_score: f32` 字段 → 通知 Console 团队（Console contractv1.go SearchResult 加同名字段 add-only）
  - 如 `/v1/search?semantic=true` 是新 query param → Console HTTPAdapter 端 review
