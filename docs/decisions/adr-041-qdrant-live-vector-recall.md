# ADR `041`: `qdrant-live-vector-recall`

**Status**: Accepted（v0.29.0 / task-36.3 closeout 据真实 CI run 26961084355 逐 D ratify；D1 live-recall-harness 方法论 ✅ + D2 真实实测 recall@10=1.0000（N=2000 dim=64 M=50，CI run 26961084355，ADR-013 真实非伪造）✅ + D3 CI service-container 集成永久关闭 CI-no-server defer ✅ + D4 默认 0-vector-dep / 0-network / 既有行为不变 + 0 新 dep ✅；qdrant live KNN recall 真实测量 + CI service-container 永久守护——见 §Ratification）

**Category**: 检索 / 向量 backend / 召回质量（qdrant HNSW ANN recall@k vs BruteForce exact KNN）/ live 端到端兑现 / CI service-container 永久守护
**Date**: 2026-06-04
**Decided By**: 主 agent（ADR-012 自治）；tajiaoyezi ratification at v0.29.0 closeout（对 v0.29.0 tag/release 的显式授权——outward-facing 须用户授权，ADR-012）
**Related**: ADR-034（production-vector-live-recall — 本 ADR 直接**关闭其 D2 的 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`**：ADR-034 D2 在 v0.22.0 ratify 时是 🟡 PARTIAL（connect/health/ensure-create/upsert/KNN wiring 成立、live-recall 维度 honest-defer——CI 无在跑 qdrant server），本 phase 据真实 live server + CI service-container 兑现 live KNN recall 测量并永久守护，以 add-only Phase-36 Amendment 记其 D2 fulfilled，**不溯改 ADR-034 D2 D-body 正文**，ADR-014 D5）/ ADR-030（production-vector-backend — qdrant 生命周期契约层之上首次真实 live recall harness，承其 D1 CI-no-server 结构性约束的识别）/ ADR-006（recall-eval-acceptance-gate — recall@k 作为可验收 gate 的方法论先例，本 phase 以 floor-guard + 真实数报告对齐）/ ADR-028（vector-persistence — qdrant 持久化 seam 复用）/ ADR-004（local-first-privacy-baseline — 默认构建仍 0 vector dep + 0 网络 + BruteForce baseline）/ ADR-008（dep add-only — Phase 36 = **0 新依赖**；`qdrant-client` 自 task-18.4 起已是 optional dep，本 phase 不引入任何新 dep）/ ADR-013（禁伪造红线 — live recall 真实数据真实跑出后回填，floor 是 guard、真实数是报告，不预填不伪造合成召回）/ ADR-012（main-agent-governance-autonomy — tag/release outward-facing 须用户显式授权，v0.29.0 本轮已授权）/ ADR-014（D1-D4，第二十七次激活）/ roadmap §3.11 + §4

## Context

ContextForge 截至 Phase 25（production-vector-backend, Done）/ Phase 29（production-vector-live-recall, Done / v0.22.0）已把 qdrant VectorBackend 推进到**完整实现 + wiring 成立**：`core/src/retriever/vector/qdrant.rs` 有 connect（`qdrant.rs:162`）/ health（`qdrant.rs:184` live 返 `QdrantHealth::Ready`、无 server 返 `Unreachable`，不 panic 不静默成功）/ open（ensure-create via `decide_ensure`，`qdrant.rs:152` + `:242`，保数据不无脑 drop+create）/ index_batch（upsert）/ search（KNN, cosine）/ delete 的全套真实实现，且配 `QdrantConnConfig::from_env()`（`qdrant.rs:72` 读 `QDRANT_URL` + 可选 `QDRANT_API_KEY`，tls 由 https scheme 推断）。但**真实 live 端到端 KNN recall 从未真实测量过**——ADR-034 D2 在 v0.22.0 ratify 时如实记为 🟡 PARTIAL：wiring 成立（connect/health/ensure-create/upsert/KNN 路径成立、health Unreachable 时干净 `exit 0`），但 live-recall 维度 honest-defer 为 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（ADR-034 D2），因为 **CI 无在跑的 qdrant server**（ADR-030 D1 已识别的结构性约束）。

**当前 in-repo 仅有的「召回数」是合成 fixture，不是真实测量（本 phase 要关闭的诚实缺口）**：

- `core/tests/eval_integration.rs:110` 的 `serde_json::json!({"recall@5": 0.7, "recall@10": 0.85})` 是 **SYNTHETIC fixture**（构造 EvalRun 持久化契约用的占位数据），**不是**对 live qdrant 的真实 recall 测量。据 ADR-013，合成 fixture 不能冒充真实召回背书。
- ADR-034 D2 Ratification 如实写：「真实 live KNN 召回数（recall@5/@10 + top-1 + MRR over real server）无 live qdrant server → honest-defer，未跑出、不预填」。

**DE-RISK 已由 lead 真实证实（不是推测——真实端到端 round-trip 已跑通）**：real qdrant + `qdrant-client` 1.18 端到端 round-trip 真实可行，KNN 正确——query `[1,0,0,0]` 返回 `[(a, 1.0), (c, 0.994)]`，cosine 排序正确。这证明 qdrant 读写路径 + cosine KNN 语义在真实 server 上正确，本 phase 的剩余工作是把它系统化为可复现的 recall harness + 永久 CI 守护，而非验证可行性本身。

本 Phase 36 是承 Phase 29（A 向量 live 召回）血脉的**深化与永久守护**——把 ADR-034 D2 honest-deferred 的 qdrant live recall 维度**真实兑现并永久关闭**：(1) 建一个 env-gated live recall harness，测 qdrant HNSW ANN recall@k vs BruteForce EXACT KNN over 同一嵌入语料；(2) 对一台 REAL qdrant server 跑（de-risk 已证）；(3) 经 CI service-container 集成，使 recall 在**每次 CI run** 被验证——永久关闭 CI-no-server defer。全程守 ADR-013：live recall 真实数字一律**真实跑出后回填**（task-36.2 CI run 跑出后填 §Ratification + v0.29.0 evidence），floor 是 guard、真实数是报告，**不预填、不伪造合成召回**。默认构建不变：`vector-qdrant` 仍 opt-in，默认构建 0 vector dep / 0 网络（ADR-004/008）；**0 新依赖**（`qdrant-client` 自 task-18.4 起已是 optional dep）。

## Decision

qdrant live 向量召回采用 **「env-gated live recall harness（qdrant HNSW ANN recall@k vs BruteForce exact KNN + 确定性可复现语料 + 无 server 诚实退出）+ 真实实测召回数（待回填，不伪造）+ CI service-container 永久守护（关闭 CI-no-server defer）+ 默认 0-vector-dep / 既有行为不变」** 策略，分 4 个决策点：

### D1 — live recall harness 方法论（qdrant HNSW ANN recall@k vs BruteForce exact KNN；确定性可复现语料；env-gated 无 server 诚实退出）（task-36.1）

新增 `core/tests/qdrant_live_recall.rs`，`#![cfg(feature = "vector-qdrant")]`，env-gated on `QDRANT_URL`（复用 `QdrantConnConfig::from_env()`，`qdrant.rs:72`，读 `QDRANT_URL` + 可选 `QDRANT_API_KEY`，tls 由 https scheme 推断）。当 `QdrantBackend::health() != Ready`（`qdrant.rs:184`）→ `eprintln` 一条 skip notice + `return`（honest-defer——CI/本机无 server 时干净跳过、**不** fail，ADR-013）。harness 方法论：

- **确定性可复现语料**：构造 N（如 1000）条 `VectorChunk` 的确定性伪随机单位向量，dim D（如 64），由 index 作种子（**无随机 / 无时钟**，per-seed 完全可复现，ADR-013）。`VectorChunk { chunk_id: ChunkId(String), embedding: Vec<f32>, metadata: Option<serde_json::Value> }`。
- **同一语料双索引**：同一语料同时索引进 `QdrantBackend`（open ensure-create + index_batch）**与** `BruteForceVectorBackend`。`VectorIndexConfig { dim, metric: VectorMetric::Cosine, persistence_path: None, collection_id: String }`。
- **recall@k 测量**：对 M（如 50）条确定性 query 向量，从 BruteForce 算 exact top-k（ground truth）+ qdrant top-k；`recall@k = mean(|qdrant_topk ∩ exact_topk| / k)`。断言 `recall@k >= 一个文档化的 floor`（如 k=10 时 0.90）并 `eprintln` 出 **ACTUAL 实测数**。floor 是 guard、真实数是报告（真实值在 task-36.2 / closeout 据真实 CI run 填入，ADR-013 不伪造）。

**0 新依赖、0 schema migration、0 proto 改动、0 默认构建变更**。Tests：TEST-36.1.1（live recall harness，env-gated——对 live qdrant 实测 `recall@k >= floor`）+ TEST-36.1.2（确定性语料生成器可复现性——**无 server** 也跑，断言 same seed ⇒ same vectors）+ TEST-36.1.3（= LAST，D2 lint）。

**理由**：qdrant-vs-exact-KNN 是 model-free + reproducible 的干净 primary 召回指标——不需要真实 embedding model（A3 那种语义 golden labels 引入模型依赖与不确定性），BruteForce exact top-k 是数学上的 ground truth，qdrant HNSW ANN top-k 与其交集比即 ANN 近似质量。确定性种子语料（无随机 / 无时钟）保 per-seed 完全可复现（ADR-013）；env-gated honest-defer（health 不 Ready → 干净 `return`，不 fail）使 harness 在无 server 环境（本机 / 未配 service 的 CI）干净跳过、不伪造 KNN 通过。floor 作 guard（防 ANN 配置回退）、真实数作报告（据实记录实际近似质量）——两者职责分离，呼应 ADR-006 recall-eval-acceptance-gate 的 gate 方法论。

### D2 — 真实实测召回数（待回填 until task-36.2 run；不伪造，ADR-013）（task-36.1 harness 产出 / task-36.2 run 回填）

harness（D1）跑出的真实 recall@k 数字（recall@10 + 可附 recall@5 / top-1 over live qdrant）是本 phase 的真实召回证据。据 ADR-013，这些数字**待回填**——在 task-36.2 的 CI service-container run（或等价真实 live run）跑出后，填入 §Ratification + `docs/releases/v0.29.0-evidence.md`，**绝不预填、绝不伪造、绝不以合成 fixture（`eval_integration.rs:110` 的 0.7/0.85）冒充真实测量**。floor（如 k=10 时 0.90）是 spec 写定的 guard，真实数是 run 跑出后据实报告——真实数 ≥ floor 是 AC，真实数本身是 evidence。

**理由**：ADR-013 禁伪造红线——召回数必须真实跑出后回填，不在 ADR / spec 预填。本 phase 的 de-risk（real qdrant + qdrant-client 1.18 round-trip 已证 KNN 正确）证明可行性，但**可行性不等于实测数**——具体 recall@10 over N=1000/D=64/M=50 语料的数字须由 harness 真实跑出。在 task-36.2 的 CI run 跑通前，本 D 的真实数字一律记 `待回填`（floor-only），ratify 时据真实 CI run 填实。

### D3 — CI service-container 集成（qdrant service in ci.yml → recall 每次 run 验证；永久关闭 CI-no-server defer）（task-36.2）

在 `.github/workflows/ci.yml` 加一个 `qdrant-recall` job，用 qdrant SERVICE CONTAINER（`services: qdrant: image: qdrant/qdrant`，ports `6334:6334` + `6333:6333`），Rust toolchain 1.93 + install protoc，跑 `QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture`。这使 harness 对 live service container 在**每次 CI run** 跑——recall 永久被验证，**关闭 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`**（ADR-034 D2 honest-defer 的结构性根因「CI 无在跑 qdrant server」now 解除——CI now HAS a live server）。CI-only / add-only / 默认构建 + 行为不变（ADR-004）。

**0 新依赖、0 schema migration、0 默认构建变更**。Tests：TEST-36.2.1（`qdrant-recall` CI job 对 live service container 跑 harness 且绿——由 PR 自身的 CI run 验证 = 真实证据）+ TEST-36.2.2（= LAST，D2 lint）。NOTE：这是 CI config 改动，验证证据是 live CI run，据实记录（ADR-013）——`qdrant-recall` job 绿 + harness `eprintln` 出的真实 recall 数即 D2 / D3 的真实回填来源。

**理由**：ADR-034 D2 的 honest-defer 根因是结构性约束「CI 无在跑 qdrant server」（ADR-030 D1）——A2（一次性本机 live recall 无 CI）能跑出数但**不永久守护**（回退无人发现）。CI service-container 是 GitHub Actions 原生 services 机制，把 live qdrant 接进 CI 拓扑，使 harness 每次 run 跑——既兑现 live recall 测量、又永久守护（ANN 配置回退会令 recall < floor → CI 红）。这是把 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` 从「结构性延后」推进到「永久关闭」的忠实手段。验证证据是 PR 自身 CI run（真实 green + 真实 recall 数），据 ADR-013 据实记录、不预填。

### D4 — 默认 0-vector-dep baseline + 既有行为不变 + 0 新依赖（all tasks）

所有改动保持默认构建 0 vector dep + 0 网络 + 既有行为 / proto / 既有契约不变（ADR-004）+ 0 新依赖（ADR-008）：

- `vector-qdrant` 仍 opt-in（`core/Cargo.toml` feature gate）：`qdrant_live_recall.rs` `#![cfg(feature = "vector-qdrant")]`，默认 `cargo test --workspace`（无 vector feature）**不编译它**，默认 semantic + hybrid 路径仍经 0-dep `BruteForceVectorBackend`（ADR-023 D5）。
- **0 新依赖**：`qdrant-client` 自 task-18.4 起已是 optional dep（`core/Cargo.toml` `vector-qdrant` feature 下）；本 phase 不引入任何新 direct dep，`Cargo.lock` 默认构建段不变。
- `qdrant-recall` CI job 是 add-only（新 job，不改既有 job）；默认构建 + 既有行为 + proto + migration 零变更。
- 既有 `cargo-test` / `go-test` / `lint` / `spec-lint` 四门不退化；新增 `qdrant-recall` job 是 add-only 第五个验证面（CI-only）。

**理由**：ADR-004 local-first + ADR-008 dep add-only——默认构建 0 vector dep / 0 网络 / 0 新依赖是不可让渡 baseline。feature-gated qdrant 是「按需启用的生产能力」而非「默认引入的成本」，与 D1-D3 的 live recall 兑现正交：harness + CI service 都在 `vector-qdrant` feature 边界 / CI service 边界内，默认 `cargo test --workspace` 与 Phase 19 语义 baseline 字节等价。CI service-container 是 CI 拓扑改动（GitHub Actions 原生 services），不进生产二进制、不改默认行为。

## Consequences

- **Positive**: qdrant live KNN recall 首次真实测量（D1 env-gated harness：qdrant HNSW ANN recall@k vs BruteForce exact KNN over 确定性可复现语料，floor-guard + 真实数报告，TEST-36.1.1/36.1.2，0 dep / 0 proto / 0 migration）；真实召回数据据真实 CI run 回填（D2 待回填，floor-only until task-36.2 run，ADR-013 不伪造、不以合成 fixture 冒充）；CI service-container 把 live qdrant 接进 CI 拓扑使 recall 每次 run 验证（D3 `qdrant-recall` job，**永久关闭 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`**——CI now HAS a live server，TEST-36.2.1 由 PR 自身 CI run 验证）；全部默认构建 0 vector dep / 0 网络 / 0 新依赖（D4 `vector-qdrant` opt-in + `qdrant-client` 早已 optional，ADR-004 / ADR-008），既有四门不退化、`qdrant-recall` 是 add-only 第五验证面。**直接关闭 ADR-034 D2 的 honest-defer**：其 v0.22.0 时的 🟡 PARTIAL（wiring 成立、live-recall honest-defer）now 推进为 live KNN recall measured + CI-guarded（以 add-only Phase-36 Amendment 记 ADR-034 D2 fulfilled，不溯改其 D2 D-body，ADR-014 D5）。
- **Negative / open**（受阻 / 延后项如实，不伪造、不夸大）：真实 recall@k 数字在 task-36.2 CI run 跑出前一律 `待回填`（floor-only，ADR-013，**绝不预填**）；recall vs golden semantic labels（需真实 embedding model）→ honest-defer `[SPEC-DEFER:phase-future.qdrant-semantic-golden-recall]`——qdrant-vs-exact-KNN 指标是 model-free + reproducible 的干净 primary，semantic-golden 引入模型依赖与不确定性、非本 phase 的 model-free scope；qdrant 集群/复制/生产部署拓扑超本 phase 范围 → honest-defer `[SPEC-DEFER:phase-future.qdrant-deployment-topology]`（CI service-container 是单节点测试拓扑，非生产拓扑背书）；多 backend production 拓扑选型 → honest-defer `[SPEC-DEFER:phase-future.multi-backend-production]`；env-gated harness 在无 server 环境（本机 / 未配 service 的 fork CI）干净 skip（health 不 Ready → `eprintln` + `return`，不 fail）——这是 by-design honest-defer 不是缺口——以上据 ADR-013 如实分级、不伪造完成、不夸大缺口。
- **Ratification**: 本 ADR **Proposed**。task-36.1（harness）/ 36.2（CI service-container run）通过后于 v0.29.0 closeout（task-36.3）据真实 CI run / 实测 recall 逐 D ratify Proposed→Accepted（见 §Ratification）；真实 recall 数据据 task-36.2 真实 CI run 回填，绝不预填（ADR-013）。
- **Follow-ups**: recall vs golden semantic labels（需真实 embedding model）`[SPEC-DEFER:phase-future.qdrant-semantic-golden-recall]`；qdrant 集群/复制/生产部署拓扑 `[SPEC-DEFER:phase-future.qdrant-deployment-topology]`；多 backend production 拓扑选型 `[SPEC-DEFER:phase-future.multi-backend-production]`。ADR-034 D2 的 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` 经本 phase D3（CI service-container）**关闭**，以 add-only Phase-36 Amendment 记其 fulfilled、不溯改 ADR-034 D2 D-body 正文（ADR-014 D5）；ADR-030 / ADR-006 / ADR-028 / ADR-004 / ADR-008 / ADR-013 引用均不溯改其正文。

## Alternatives

- **A1（synthetic-fixture recall）**：用合成 fixture（如 `eval_integration.rs:110` 的 `recall@5: 0.7 / recall@10: 0.85`）充当 recall 背书。否决：合成 fixture **不是真实测量**（违 ADR-013 禁伪造红线）——它是构造 EvalRun 持久化契约的占位数据，不能冒充对 live qdrant 的真实 KNN recall；据 D1/D2，真实召回须由 harness 对 live server 真实跑出后报告，floor 是 guard、真实数是 evidence。
- **A2（一次性本机 live recall 无 CI）**：在本机 / dev-box 对 live qdrant 跑一次 recall harness、记下数字、不接 CI。否决：**不永久守护**（ANN 配置回退 / qdrant 升级回退会令 recall 退化而无人发现）；据 D3，CI service-container 把 live qdrant 接进 CI 拓扑使 recall 每次 run 验证，才能永久关闭 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`——一次性本机跑兑现了「测过一次」却没兑现「永久守护」。
- **A3（recall vs golden semantic labels，需真实 embedding model）**：以真实语义 golden labels（需真实 embedding model 生成语义相关性标注）为 ground truth 测 recall。延后 `[SPEC-DEFER:phase-future.qdrant-semantic-golden-recall]`：qdrant-vs-exact-KNN 指标是 model-free + reproducible 的干净 primary——BruteForce exact top-k 是数学 ground truth、确定性种子语料无随机 / 无时钟可复现；semantic-golden 引入真实 embedding model 依赖 + 标注主观性 + 不确定性，非本 phase model-free scope，作 follow-up 延后而非本 phase primary。

## 触及 ADR 关系

- **ADR-034（production-vector-live-recall）→ 直接关闭其 D2 defer + add-only Phase-36 Amendment（不溯改）**：ADR-034 D2 在 v0.22.0 ratify 时是 🟡 PARTIAL（connect/health/ensure-create/upsert/KNN wiring 成立、live-recall 维度 honest-defer 为 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`——CI 无在跑 qdrant server）。本 phase D1（harness）+ D3（CI service-container）兑现 live KNN recall 测量 + 永久守护，关闭该 defer。以 add-only `## Amendment (Phase 36 / v0.29.0)` 记其 D2 fulfilled（live KNN recall measured + CI-guarded），**不溯改 ADR-034 D2 D-body 正文 + 不溯改其 Ratification (v0.22.0) 任何 Phase 29 校准**（ADR-014 D5）。
- **ADR-030（production-vector-backend）→ 据实引用（不溯改）**：ADR-030 D1 已识别「CI 无在跑 qdrant server」是结构性约束，本 phase D3 经 CI service-container 解除该约束，承其方向、不溯改其正文。
- **ADR-006（recall-eval-acceptance-gate）→ 方法论对齐引用（不溯改）**：recall@k 作可验收 gate 的方法论先例，本 phase 以 floor-guard（防回退）+ 真实数报告（据实记录）对齐，不溯改其正文。
- **ADR-028（vector-persistence）→ 据实引用（不溯改）**：qdrant 持久化 seam 复用（open ensure-create + index_batch），承其方向、不溯改其正文。
- **ADR-004（local-first-privacy-baseline）→ 守线**：默认构建 0 vector dep + 0 网络 + 既有行为 / proto / 既有契约不变（D4）守 ADR-004 baseline；CI service-container 是 CI 拓扑改动不进生产二进制 / 不改默认行为。
- **ADR-008（dep add-only）→ 守线**：本 phase 加 **0 新依赖**——`qdrant-client` 自 task-18.4 起已是 optional dep（`vector-qdrant` feature 下），不引入任何新 direct dep，`Cargo.lock` 默认构建段不变。
- **ADR-013（禁伪造红线）→ 守线**：live recall 真实数据真实跑出后回填（D2 待回填 until task-36.2 run，floor 是 guard、真实数是报告，绝不预填、绝不以合成 fixture（`eval_integration.rs:110`）冒充真实测量）；确定性种子语料无随机 / 无时钟可复现（D1）；env-gated honest-defer（health 不 Ready → 干净 `return` 不 fail，D1）据实分级、不伪造 KNN 通过；CI run 验证证据据实记录、不预填（D3）。
- **ADR-014（cross-phase-exit-criteria-validation）→ 第二十七次激活**：D1-D4 mapping + 各 task LAST D2 lint（TEST-36.1.3 / 36.2.2，touched 行 0 未标注命中）+ D1 verified-by（TEST-36.1.1 live harness / 36.1.2 reproducibility）+ D3 verified-by（TEST-36.2.1 live CI run）+ D4 自治 + D5 历史 Phase 1-35 不溯改（ADR 改动 add-only Phase-36 Amendment、不溯改 ADR-034 D2 D-body）；本 ADR ratify 在 task-36.3 closeout，Proposed 阶段不 ratify。

## Ratification (v0.29.0 / task-36.3)

**Proposed → Accepted**（逐 D 据真实 CI run **26961084355** + 实测 recall，ADR-013 真实非伪造）：

- **D1 live recall harness 方法论 — ✅ Accepted**：`core/tests/qdrant_live_recall.rs`（`#![cfg(feature = "vector-qdrant")]`，env-gated `QDRANT_URL` 复用 `QdrantConnConfig::from_env()`）落地——确定性可复现语料（splitmix64 index-seeded 单位向量 N=2000 dim=64，无 `rand` / 无 clock）双索引进 `QdrantBackend`（ensure-create + index_batch）与 `BruteForceVectorBackend`（精确 ground truth），M=50 query `recall@k=mean(|qdrant_topk ∩ exact_topk|/k)`；`health()!=Ready` honest-defer 干净 skip 不 fail。TEST-36.1.1（live）+ TEST-36.1.2（可复现性，无 server 也跑）本地 + CI 双绿。
- **D2 真实实测召回数 — ✅ Accepted（真实回填）**：CI run **26961084355**（`qdrant-recall` job，service container 日志 `qdrant ready after 1 attempt(s)`）实测 `PHASE36 qdrant LIVE recall@10 vs brute-force exact KNN | N=2000 dim=64 M=50 => recall@10=1.0000`，`test result: ok. 2 passed; 0 failed`。本地对 `qdrant/qdrant` 容器一致复现 recall@10=1.0000。**诚实判读（ADR-013）**：recall=1.0 因 qdrant 在 N=2000（低于其 HNSW indexing_threshold 默认 ~10000）服务**精确** KNN → 这是 live KNN **正确性**真实证明（qdrant == brute-force exact ground truth，取代合成 fixture `eval_integration.rs:110` 的 0.7/0.85）；HNSW **近似域**大语料真实 ANN recall（预期 <1.0）须大语料 + optimizer-wait → honest-defer `[SPEC-DEFER:phase-future.vector-large-corpus-perf]`，不夸大为「已压测 HNSW 近似」。floor=0.90 为不退化 guard，真实 1.0000 留足余量。
- **D3 CI service-container 集成 — ✅ Accepted**：`.github/workflows/ci.yml` `qdrant-recall` job（`services: qdrant/qdrant` 6334+6333 + Rust 1.93 + protoc + Wait-for-ready + harness）每次 CI run 对 live service container 验证 recall；run 26961084355 绿 = 真实证据。**`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（ADR-034 D2）永久关闭**——CI now HAS a live qdrant server。
- **D4 默认 0-vector-dep + 0 新 dep + 既有行为不变 — ✅ Accepted**：`vector-qdrant` opt-in（harness `#![cfg]`-gated，默认 `cargo test --workspace` 不编译）；`qdrant-client` 自 task-18.4 已 optional（0 新 dep）；既有 cargo-test / go-test / lint / spec-lint / feature-build 不退化，`qdrant-recall` 是 add-only 第 6 验证面（run 26961084355 全门绿）。

**ADR-034 D2 关闭**：以 add-only `## Amendment (Phase 36 / v0.29.0)` 记 ADR-034 D2 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` fulfilled（live KNN recall measured + CI-guarded），不溯改 ADR-034 D2 D-body / Ratification (v0.22.0)（ADR-014 D5）。ADR-014 第二十七次激活全 D 通过。
