# Phase 36 · qdrant-live-vector-recall

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 承 Phase 25（production-vector-backend, Done）/ Phase 29（live-vector-recall, v0.22.0, Done）的 qdrant 生命周期契约层 + live KNN wiring 成果，把 ADR-034 D2 一路 honest-defer 的 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（真实 live-server 端到端 KNN 召回数）**兑现并永久 CI-guarded**。**主题（一句）**：对一台**真实 qdrant server** 跑 qdrant HNSW ANN recall@k vs BruteForce 精确 KNN 的真实召回 harness，并经 CI service container 让该召回在**每次 CI run** 被验证——永久关闭「CI 无 live qdrant server」这一结构性延后。**关键 de-risk 已被 lead 证明**：真实 qdrant + qdrant-client 1.18 端到端 round-trip 已跑通、KNN 正确（query `[1,0,0,0]` 返 `[(a,1.0),(c,0.994)]` 余弦序正确），故本 phase 非「探索能否跑通」而是「把已证明可行的 round-trip 固化为 env-gated harness + CI service」。qdrant backend 自身（`core/src/retriever/vector/qdrant.rs` connect/health/open(ensure-create via `decide_ensure`)/index_batch(upsert)/search(KNN, cosine)/delete）自 Phase 25/29 已**全实现**——本 phase 0 行 backend 逻辑改动、只加 harness + CI 接线。**默认构建不变**：`vector-qdrant` opt-in，默认构建 0 vector dep / 0 网络（ADR-004/008）；**0 新依赖**（`qdrant-client` 自 task-18.4 已是 optional）；0 schema migration；0 默认构建 / 默认行为改动。in-repo 现存的唯一召回数（`core/tests/eval_integration.rs` 的 0.7/0.85）是**合成 fixture 非真实**——本 phase 以真实测量取而代之（ADR-013，A1 synthetic-fixture REJECTED）。v0.29.0 收口。对应 `docs/roadmap.md §3.18`。

> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md §3.18`（live 向量召回 + qdrant 真实兑现路线 + `qdrant-server-lifecycle` 残留 live-server KNN 维度）→ `core/src/retriever/vector/qdrant.rs`（`QdrantConnConfig::from_env()` :72-78 读 `QDRANT_URL` + 可选 `QDRANT_API_KEY`、tls 由 https scheme 推断 / `QdrantBackend::connect` :162-181 / `health()` :184-189 Ready·Unreachable / `open()` :215-270 ensure-create via `decide_ensure` :152-158 / `index_batch()` :272-298 upsert / `search()` :330-371 KNN cosine / `delete()` :300-318）→ `core/src/retriever/vector/brute_force.rs`（`BruteForceVectorBackend` 精确 O(n) cosine ground-truth :27-37 + `index_batch`/`search`）→ `core/src/retriever/vector/types.rs`（`VectorChunk` :46-51 `{ chunk_id: ChunkId(String), embedding: Vec<f32>, metadata: Option<serde_json::Value> }` / `VectorIndexConfig` :54-60 `{ dim, metric: VectorMetric::Cosine, persistence_path: None, collection_id: String }` / `VectorHit` :38-43 / `VectorScore` :22-35）→ `core/examples/phase29_recall_via_qdrant.rs`（Phase 29 harness 范本：`connect(from_env())` + `health()` 守门 + honest-defer）→ `.github/workflows/ci.yml`（`feature-build` job :106-138 protoc 安装 + feature matrix 范本 / `cargo-test` job :13-31 toolchain 1.93 范本）→ AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5，**第二十七次**激活）→ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造红线：真实召回数真实跑出后回填、绝不预填合成数 / CI service container 真实验证证据 = PR 自身 CI run）→ `docs/decisions/adr-034-production-vector-live-recall.md`（D2 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` 本 phase 兑现，以 add-only Phase-36 Amendment 标记其 fulfilled、不溯改 D-body，ADR-014 D5）→ `docs/decisions/adr-004-local-first-privacy-baseline.md`（默认构建 0 vector dep baseline 不变）。

> **ADR 影响面（已识别）**：
> - **ADR-041 qdrant-live-vector-recall（新，Proposed）**：D1 live recall harness（qdrant HNSW ANN recall@k vs BruteForce 精确 KNN 方法学；deterministic 可复现语料；无 server 时 env-gated honest-defer `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`）+ D2 真实测量召回数（placeholder `待回填` 直至 task-36.2 CI run 跑出，ADR-013 不伪造）+ D3 CI service-container 集成（ci.yml `qdrant` service → recall 每次 run 验证；永久关闭「CI 无 server」延后）+ D4 默认 0-vector-dep baseline + 行为不变（`vector-qdrant` opt-in；ADR-004/008；0 新 dep）。Status: Proposed（Draft 阶段不 ratify；ratify 在 task-36.3 closeout，逐 D 据真实数 ratify）。ADR-014 第二十七次激活。
> - 触及 **ADR-034（production-vector-live-recall，v0.22.0 母 ADR）**：其 D2 把 qdrant live-server 端到端 KNN 召回数记为 honest-defer（`## Ratification` D2 🟡 PARTIAL）；本 phase 真实兑现 → 以 **add-only Phase-36 Amendment** 标记 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` D2 维度 fulfilled（live KNN recall 已测 + CI-guarded），**不溯改 D2 D-body / 不溯改其 `## Ratification`**（ADR-014 D5）。
> - 触及 **ADR-004（local-first 0-dep 基线）**：`vector-qdrant` 仍 feature-gated，默认构建仍 0 vector dep / 0 网络；harness 仅 `#![cfg(feature = "vector-qdrant")]`、CI service 仅在新 `qdrant-recall` job 内（守线，非推翻）。
> - 触及 **ADR-008（dep add-only，Phase 36 不增 dep）**：`qdrant-client` 自 task-18.4 已是 optional dep（`vector-qdrant` feature 下），本 phase **0 新 dep**；harness 仅复用既有 `QdrantBackend` / `BruteForceVectorBackend` / `std`，CI 仅加 service container（无源码 dep）。

## 1. 阶段目标

v0.28.0 ship 后，ContextForge 的 qdrant backend 自 Phase 25/29 已**全实现且 wiring 经 honest-defer 证明**（connect/health/ensure-create/upsert/KNN search/delete），唯一缺口是 ADR-034 D2 记的 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`——**真实 live-server 端到端 KNN 召回数**因「CI 无 live qdrant server」一路诚实延后，in-repo 现存召回数（`eval_integration.rs` 0.7/0.85）是**合成 fixture**。本 phase 关闭该缺口：(1) 建一个 env-gated live recall harness，测量 qdrant HNSW ANN recall@k vs BruteForce 精确 KNN（同一内嵌语料 ground truth）；(2) 对一台**真实 qdrant server** 跑出真实召回数（lead 已 de-risk 证明 round-trip 可行 + KNN 正确）；(3) 经 CI **service container** 把它接入每次 CI run → 召回永久被验证。默认构建保持 0 vector dep / 0 网络、`vector-qdrant` opt-in（ADR-004/008），0 新 dep / 0 migration / 0 backend 逻辑改动；既有 `cargo-test` / `go-test` / `lint` / `spec-lint` / `feature-build` 门不退化。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. **qdrant-live-recall-harness（🟢 deterministic generator / 🔴 live server）**：新增 `core/tests/qdrant_live_recall.rs`，`#![cfg(feature = "vector-qdrant")]`，env-gated 于 `QDRANT_URL`（复用 `QdrantConnConfig::from_env()` 读 `QDRANT_URL` + 可选 `QDRANT_API_KEY`、tls 由 https scheme 推断）。`QdrantBackend::health() != Ready` 时 eprintln skip notice + `return`（honest-defer——无 server 的 CI/本地干净跳过，**绝不 fail**，ADR-013）。构建一个 **deterministic 可复现**语料：N（如 1000）个 `VectorChunk`、维度 D（如 64）、伪随机单位向量**由 index 种子化**（无 randomness / 无 clock，per ADR-013 可复现）。把**同一语料**索引进 `QdrantBackend`（open ensure-create + index_batch）**与** `BruteForceVectorBackend`。对 M（如 50）个 deterministic query 向量，从 BruteForce 算精确 top-k（ground truth）+ qdrant top-k；recall@k = mean(|qdrant_topk ∩ exact_topk| / k)。断言 recall@k ≥ 文档化 floor（如 k=10 时 0.90）并 eprintln **实测**数。floor 为护栏、真实数被报告（真实值在 task-36.2/closeout 据 CI run 回填，ADR-013 不伪造）（AC1）
2. **qdrant-recall-ci-service（CI-only / add-only）**：`.github/workflows/ci.yml` 加 `qdrant-recall` job，用 qdrant **service container**（`services: qdrant: image: qdrant/qdrant`，ports `6334:6334` + `6333:6333`），Rust toolchain 1.93 + 装 protoc，跑 `QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture`。该 job 在**每次 CI run** 对 live service container 跑 harness → 召回永久被验证，关闭 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（CI 现**有** live server）。CI-only / add-only / 默认构建 + 行为不变（ADR-004）。**验证证据 = PR 自身的 live CI run（真实，非预填）**，据实记录（ADR-013）（AC2）
3. **v0.29.0 收口**：`scripts/console_smoke.sh` banner v25→v26 + 新 step [45/45]（staging dir `cf-v28-cfg`，offset +2）+ `internal/cli/smoke_syntax_test.go` 新 `TestTask363_SmokeV26QdrantLiveRecallStep`（镜像 `TestTask353`）断言 [45/45] + no-regression（denominators [37/37]..[44/44] 不溯改，ADR-014 D5）；`docs/releases/v0.29.0-{evidence,artifacts}.md`（**真实召回数 + 真实 CI run 链接**，tag SHA / run id / digest 为 angle-bracket `<backfill>` marker）+ README v0.29 段 + RELEASE_NOTES v0.29.0 段；ADR-041 Proposed→Accepted（逐 D 据真实数 ratify，D2 真实召回数据回填）+ ADR-034 add-only Phase-36 Amendment（标 D2 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` fulfilled、不溯改 D-body）+ roadmap §3.18/§4 add-only + s2v-adapter rows（AC3）
4. ADR-014 D1-D5（**第二十七次**激活）全通过（AC4）

**v0.x 版本号决策**：v0.29.0（Phase 36，承 v0.28.0；roadmap §1.1 Phase N→v0.(N-7).0），theme qdrant-live-vector-recall。minor release（真实 live KNN 召回兑现 + CI service-container 永久验证；`vector-qdrant` opt-in、默认构建 0 vector 依赖 + 0 网络不变，0 新依赖（ADR-008，`qdrant-client` 自 task-18.4 已 optional）+ 0 proto / 0 migration / 0 backend 逻辑改动 / 默认行为不变（ADR-004））。

## 2. 业务价值

兑现 ADR-034 D2 + roadmap §3.18 一路刻意延后的 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`——把「qdrant live KNN wiring 经 honest-defer 证明、但真实召回数从未跑出」推进到「真实召回数实测 + 每次 CI run 永久验证」：

- **qdrant 真实召回数从未跑出**：Phase 29（ADR-034 D2 🟡 PARTIAL）确立 qdrant connect→ensure-create→upsert→KNN wiring 成立（无 server 时 honest-defer exit 0），但真实 live-server 召回数（recall@5/@10 over real server）因 CI 无 server 一路诚实未跑出 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`。本 phase 对真实 server 跑出真实 recall@k，使「qdrant 真能高质量召回」从 wiring 假设变为实测事实（lead de-risk 已证明 round-trip 可行 + KNN 余弦序正确）。
- **in-repo 召回数是合成 fixture**：`core/tests/eval_integration.rs` 的 0.7/0.85 是合成 fixture（非真实跑出），不构成 qdrant 召回质量的真实证据。本 phase 以真实测量取而代之（ADR-013，合成-fixture 召回 A1 REJECTED）。
- **召回从未在 CI 被验证**：Phase 29 的 honest-defer 是结构性约束（CI 无 server）。本 phase 经 CI service container 让 qdrant recall 在**每次 CI run** 被验证 → 任何破坏 qdrant 召回质量的回归会被 CI 卡红，**永久关闭**「CI 无 server」结构性约束 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（A2 一次性本地 live recall 无 CI REJECTED——非永久护栏）。
- **0 backend 改动、纯 harness + CI 接线**：qdrant backend 自 Phase 25/29 已全实现，本 phase 不改 1 行 backend 逻辑，只加 env-gated harness（复用既有 backend）+ CI service（无源码 dep），是最 surgical 的「兑现而非重写」。

### 36.1 qdrant-live-recall-harness（qdrant-live-recall-harness，🟢 generator / 🔴 live server）

- **新增 `core/tests/qdrant_live_recall.rs`**：`#![cfg(feature = "vector-qdrant")]`，env-gated 于 `QDRANT_URL`（复用 `QdrantConnConfig::from_env()` :72-78 读 `QDRANT_URL` + 可选 `QDRANT_API_KEY`、tls 由 https scheme 推断）。`QdrantBackend::connect(&QdrantConnConfig::from_env())` 后 `health()`（:184-189）守门：`!= Ready`（Unreachable）→ `eprintln!` skip notice + `return`（honest-defer——无 server 的 CI/本地干净跳过、**绝不 fail**，ADR-013）。
- **deterministic 可复现语料**：N（如 1000）个 `VectorChunk { chunk_id: ChunkId(format!(...)), embedding: <dim-D 伪随机单位向量>, metadata: None }`，维度 D（如 64），伪随机单位向量**由 index 种子化**（如 splitmix64/LCG 喂 index，**无 `rand` crate / 无 clock**，per ADR-013 可复现：同 seed ⇒ 同向量）。向量归一化为单位长（cosine = dot）。
- **双索引同语料 ground truth**：把**同一语料**索引进 `QdrantBackend`（`open(VectorIndexConfig { dim: D, metric: VectorMetric::Cosine, persistence_path: None, collection_id: <唯一名> })` ensure-create + `index_batch`）**与** `BruteForceVectorBackend`（精确 O(n) cosine，ground truth）。
- **recall@k 测量**：对 M（如 50）个 deterministic query 向量，从 BruteForce 算精确 top-k（ground truth）+ qdrant `search()` top-k；recall@k = mean(|qdrant_topk ∩ exact_topk| / k)。断言 recall@k ≥ 文档化 floor（如 k=10 时 **0.90**）并 `eprintln!` **实测**数。**floor 是护栏、真实数被报告**（真实值在 task-36.2 CI run / task-36.3 closeout 据真实跑出回填，**绝不预填**，ADR-013）。
- 0 新 dep / 0 schema migration / 0 默认构建改动；harness 仅复用既有 `QdrantBackend` / `BruteForceVectorBackend` / `std`。
- **同源验证（≥2，🔴 live server + 🟢 generator）**：TEST-36.1.1（live recall harness，env-gated——对 live qdrant 实测 recall@k ≥ floor；无 server 时 honest-defer 干净跳过、不 fail、零召回数输出）+ TEST-36.1.2（deterministic 语料 generator 可复现性——**无 server 也跑**，断言同 seed ⇒ 同向量，per ADR-013）+ TEST-36.1.3（= LAST，D2 lint）。

### 36.2 qdrant-recall-ci-service（qdrant-recall-ci-service，CI-only / add-only）

- **`.github/workflows/ci.yml` 加 `qdrant-recall` job**：用 qdrant **service container**（`services: qdrant: image: qdrant/qdrant`，ports `6334:6334`（gRPC）+ `6333:6333`（REST）），`runs-on: ubuntu-22.04`，Rust toolchain **1.93**（与既有 job 一致）+ 装 protoc（`qdrant-client` build 需系统 protoc，仿 `feature-build` job :127-128），跑 `QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture`。
- 该 job 在**每次 CI run** 对 live service container 跑 harness → 召回永久被验证，**关闭** `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（CI 现**有** live server，结构性约束解除）。
- **CI-only / add-only / 默认构建 + 行为不变**（ADR-004）：新增 job 不动既有 5 job，不改默认 `cargo-test`（仍默认构建 0 vector dep），仅在 `qdrant-recall` job 内启 `vector-qdrant` + service container。
- **验证证据 = PR 自身的 live CI run**（真实，非预填）：该 job 绿 = 真实 live recall 验证证据，run id / 链接据实记录（**绝不预填**，ADR-013）。NOTE：本 task 是 **CI config 改动**，验证证据是 live CI run、据实记录（ADR-013）。
- **同源验证（≥1，CI-only）**：TEST-36.2.1（`qdrant-recall` CI job 对 live service container 跑 harness 且绿——经 **PR 自身 CI run** = 真实证据验证）+ TEST-36.2.2（= LAST，D2 lint）。

### 36.3 closeout-v0.29.0（closeout-v0.29.0，🟢）

- **(a) v0.29.0 收口**：`scripts/console_smoke.sh` banner v25→v26 + v26 changelog block + 新 step [45/45]（staging dir `cf-v28-cfg`，offset +2：v23→`cf-v25-cfg` / v24→`cf-v26-cfg` / v25→`cf-v27-cfg` / v26→`cf-v28-cfg`；current Phase 35 [44/44] → Phase 36 顺位 [45/45]）；`internal/cli/smoke_syntax_test.go` 新 `TestTask363_SmokeV26QdrantLiveRecallStep`（镜像 `TestTask353`）断言 [45/45] + no-regression（denominators [37/37]..[44/44] 不溯改，ADR-014 D5）。
- **(b) release docs（真实数 + 真实 CI run 链接）**：`docs/releases/v0.29.0-{evidence,artifacts}.md`（**真实 recall@k 数 + 真实 `qdrant-recall` CI run 链接**；tag SHA / run id / digest 为 angle-bracket `<backfill>` marker 直至 post-tag-push）+ `README.md` v0.29 段 + `RELEASE_NOTES.md` v0.29.0 段。
- **(c) ADR ratify + Amendment**：ADR-041 Proposed→Accepted（逐 D 据真实数 ratify；**D2 真实召回数据回填**——floor-only 直至 CI run 跑出真实数）+ ADR-034 add-only **Phase-36 Amendment**（标其 D2 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` fulfilled：live KNN recall 已测 + CI-guarded，**不溯改 D2 D-body / 不溯改 `## Ratification`**，ADR-014 D5）+ roadmap §3.18/§4 add-only（`qdrant-server-lifecycle` progressed → fulfilled）+ s2v-adapter Phase 36 + tasks + ADR-041 + BDD rows。
- ADR-014 第二十七次激活。

**不在本 phase 范围**：

- qdrant 集群 / 副本 / 分片部署拓扑（CI service 仅单节点 baseline）[SPEC-DEFER:phase-future.qdrant-deployment-topology]
- recall vs golden 语义标签（需真实 embedding 模型）的语义召回——qdrant-vs-exact-KNN metric 为干净 primary（model-free + 可复现），语义 golden 召回延后 [SPEC-DEFER:phase-future.qdrant-semantic-golden-recall]
- 多 backend（lancedb / sqlite-vec）的同款 CI service live recall（本 phase 仅 qdrant；其余 backend live recall 延后）[SPEC-DEFER:phase-future.multi-backend-production]
- 大语料（百万级）qdrant 性能基准与调优 [SPEC-DEFER:phase-future.vector-large-corpus-perf]

## 3. 涉及模块

### 36.1 qdrant-live-recall-harness（task-36.1）

- 新增 `core/tests/qdrant_live_recall.rs`——`#![cfg(feature = "vector-qdrant")]`；env-gated 复用 `QdrantConnConfig::from_env()`（`qdrant.rs:72-78`，读 `QDRANT_URL` + 可选 `QDRANT_API_KEY`、tls 由 https scheme 推断）；`QdrantBackend::connect` + `health()`（:184-189）`!= Ready` → `eprintln!` skip + `return`（honest-defer，绝不 fail，ADR-013）
- deterministic 可复现语料 generator——N（如 1000）个 `VectorChunk { chunk_id: ChunkId(String), embedding: Vec<f32>(dim D，如 64), metadata: None }`，伪随机单位向量**由 index 种子化**（splitmix64/LCG 喂 index，**无 `rand` crate / 无 clock**，同 seed ⇒ 同向量，ADR-013 可复现）；归一化单位长
- 双索引同语料——`QdrantBackend.open(VectorIndexConfig { dim: D, metric: VectorMetric::Cosine, persistence_path: None, collection_id: <唯一名> })` + `index_batch` / `BruteForceVectorBackend`（精确 O(n) cosine ground truth）
- recall@k 测量——M（如 50）deterministic query：BruteForce 精确 top-k（ground truth）vs qdrant `search()` top-k；recall@k = mean(|∩| / k)；断言 ≥ floor（k=10 → 0.90）+ `eprintln!` 实测数（floor 护栏 / 真实数回填，绝不预填，ADR-013）
- 0 新 dep / 0 schema migration / 0 默认构建改动；仅复用既有 backend + `std`
- 同源验证（≥2：TEST-36.1.1 live recall harness（env-gated，live qdrant 实测 recall@k ≥ floor；无 server honest-defer 干净跳过不 fail、零召回数输出）/ TEST-36.1.2 deterministic generator 可复现性（**无 server 也跑**，同 seed ⇒ 同向量）；真实召回数真实跑出后回填、不伪造，ADR-013）

### 36.2 qdrant-recall-ci-service（task-36.2）

- 修改 `.github/workflows/ci.yml`——加 `qdrant-recall` job：`runs-on: ubuntu-22.04`；`services: qdrant: image: qdrant/qdrant`（ports `6334:6334` gRPC + `6333:6333` REST）；Rust toolchain `1.93`（与既有 job 一致）+ 装 protoc（仿 `feature-build` :127-128）+ cache cargo；跑 `QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture`
- CI-only / add-only——不动既有 `cargo-test` / `go-test` / `lint` / `spec-lint` / `feature-build` 5 job，默认 `cargo-test` 仍默认构建 0 vector dep（ADR-004 行为不变）；`vector-qdrant` + service container 仅在新 job 内
- 关闭 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（CI 现有 live server，结构性约束解除）；**验证证据 = PR 自身 live CI run**（据实记录 run id / 链接，绝不预填，ADR-013）
- 同源验证（≥1：TEST-36.2.1 `qdrant-recall` CI job 对 live service container 跑 harness 且绿——经 **PR 自身 CI run** = 真实证据验证；CI config 改动、验证证据为 live CI run 据实记录，ADR-013）

### 36.3 closeout-v0.29.0（task-36.3）

- 修改 `scripts/console_smoke.sh`——banner v25→v26 + v26 changelog block + 新 step [45/45]（staging dir `cf-v28-cfg`，offset +2；qdrant live recall verify 可达则断言、否则 doc/status；current Phase 35 [44/44] → Phase 36 顺位 [45/45]）
- 修改 `internal/cli/smoke_syntax_test.go`——新 `TestTask363_SmokeV26QdrantLiveRecallStep`（镜像 `TestTask353`）断言 [45/45] + no-regression（denominators [37/37]..[44/44] 不溯改，ADR-014 D5）
- 新增 `docs/releases/v0.29.0-evidence.md` + `v0.29.0-artifacts.md`（**真实 recall@k 数 + 真实 `qdrant-recall` CI run 链接**；tag SHA / run id / digest 为 angle-bracket `<backfill>` marker）+ `README.md` v0.29 段 + `RELEASE_NOTES.md` v0.29.0 段
- 修改 `docs/decisions/adr-041-qdrant-live-vector-recall.md`——Status Proposed→Accepted（逐 D 据真实数如实，D2 真实召回数据回填——floor-only 直至 CI run 跑出真实数）+ 新 `## Ratification（v0.29.0 / task-36.3）`
- add-only Amendment（非正文改，ADR-014 D5）：`adr-034`（production-vector-live-recall 母 ADR，本 phase 兑现其 D2 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`——标 fulfilled：live KNN recall 已测 + CI-guarded，不溯改 D2 D-body / `## Ratification`）；`docs/roadmap.md §3.18/§4` add-only（Phase 36 行 + `qdrant-server-lifecycle` progressed→fulfilled + qdrant-semantic-golden-recall / multi-backend-production 新/续 backlog 条目）
- 修改 `docs/specs/phases/phase-36-qdrant-live-vector-recall.md`——Status Draft→Done + §6 AC 勾选（逐维如实）
- 修改 `docs/s2v-adapter.md`——Phase 36 行 + Task 行 + ADR-041 行 + BDD 行

### BDD feature

- 新增 `test/features/phase-36-qdrant-live-vector-recall.feature`（≥3 scenario：qdrant live recall harness（有 server → recall@k ≥ floor + 实测数报告 / 无 server → honest-defer 干净跳过不 fail）/ deterministic 语料 generator 可复现（同 seed ⇒ 同向量、无 server 也跑）/ CI service container 集成（`qdrant-recall` job 每次 run 验证 recall、关闭 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`）+ 默认构建 0-vector-dep 基线不变（`vector-qdrant` opt-in、0 新 dep））

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 36.1 | 新增 `core/tests/qdrant_live_recall.rs`（`#![cfg(feature = "vector-qdrant")]`，env-gated `QDRANT_URL` 复用 `QdrantConnConfig::from_env()`；`health() != Ready` honest-defer skip 不 fail；deterministic 可复现语料 N=1000 dim=64 index-seeded 单位向量；双索引 qdrant vs BruteForce ground truth；recall@k = mean(\|∩\|/k) 断言 ≥ floor(k=10→0.90) + eprintln 实测数；0 新 dep / 0 migration / 0 默认构建改动） | `../tasks/task-36.1-qdrant-live-recall-harness.md` |
| 36.2 | `.github/workflows/ci.yml` 加 `qdrant-recall` job（qdrant service container `image: qdrant/qdrant` ports 6334:6334 + 6333:6333；toolchain 1.93 + protoc；`QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture`）→ 每次 CI run 验证 recall、关闭 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`；CI-only / add-only / 默认构建+行为不变；验证证据 = PR 自身 live CI run 据实记录 | `../tasks/task-36.2-qdrant-recall-ci-service.md` |
| 36.3 | smoke v26[45/45] + v0.29.0 closeout（真实 recall 数 + 真实 CI run 链接）+ ADR-041 ratify + ADR-034 add-only Phase-36 Amendment（标 D2 qdrant-server-lifecycle fulfilled）+ roadmap §3.18/§4 add-only + s2v-adapter add-only | `../tasks/task-36.3-closeout-v0.29.0.md` |

## 5. 依赖关系

- **task-36.1**（qdrant-live-recall-harness）dep 既有 `core/src/retriever/vector/qdrant.rs` 全实现 backend（`QdrantConnConfig::from_env()` :72-78 / `connect` :162-181 / `health` :184-189 / `open` ensure-create :215-270 / `index_batch` :272-298 / `search` :330-371 均 Phase 25/29 Done）+ `brute_force.rs` `BruteForceVectorBackend` 精确 cosine ground truth（Phase 19 Done）+ `types.rs` `VectorChunk`/`VectorIndexConfig`/`VectorHit`（已在）+ Phase 29 harness 范本 `core/examples/phase29_recall_via_qdrant.rs`；可独立先行（不依赖 36.2，generator 部分无 server 也跑）。
- **task-36.2**（qdrant-recall-ci-service）建议 36.1 先 merge（harness 文件稳定后接 CI job）+ dep 既有 `.github/workflows/ci.yml` `feature-build` job protoc 安装范本（:127-128）+ `cargo-test` toolchain 1.93 范本（:13-31）+ GitHub Actions service container 支持；与 36.1 文件解耦（CI 接线）。
- **task-36.3**（closeout）dep 36.1 + 36.2 全 Done；release docs / smoke v26[45/45] / ADR-041 ratify 据两 task 真实产物（真实 recall 数 + 真实 `qdrant-recall` CI run 链接，受阻维度如实）。
- 外部：ADR-041（本 phase 新 Proposed）/ ADR-034（production-vector-live-recall 母 ADR，本 phase 兑现其 D2 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`，add-only Phase-36 Amendment 标 fulfilled、不溯改 D-body）/ ADR-030（production-vector-backend，选择矩阵守线引用）/ ADR-023（vector-backend-default，tier 守线引用）/ ADR-004（local-first，默认构建 0 vector dep baseline + 0 网络 + 行为不变）/ ADR-008（dep add-only，Phase 36 不增 dep、`qdrant-client` 自 task-18.4 已 optional）/ ADR-012（tag/release outward-facing 须用户显式授权）/ ADR-014 **第二十七次**激活 / ADR-013（禁伪造红线：真实 live KNN 召回数真实跑出后回填绝不预填合成数 / CI service container 真实验证证据 = PR 自身 live CI run 据实记录 / 无 server honest-defer 不 fail、零召回数输出不伪造 / 受阻维度如实记录）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（每条 AC 含 ADR-014 D3 verified by 显式 owner；Draft 阶段未勾选，实施后逐条置 `[x]`）**：

- [x] **AC1**（qdrant-live-recall-harness；🔴 live server / 🟢 generator）: 新增 `core/tests/qdrant_live_recall.rs`（`#![cfg(feature = "vector-qdrant")]`），env-gated 复用 `QdrantConnConfig::from_env()`；`QdrantBackend::health() != Ready` → `eprintln!` skip + `return`（honest-defer——无 server 干净跳过、**绝不 fail**、零召回数输出，ADR-013）；deterministic 可复现语料（N=1000 dim=64 index-seeded 单位向量、**无 `rand`/无 clock**、同 seed ⇒ 同向量）双索引进 `QdrantBackend`（ensure-create + `index_batch`）与 `BruteForceVectorBackend`（精确 cosine ground truth）；M=50 query recall@k = mean(|∩|/k)、断言 ≥ floor（k=10 → 0.90）+ `eprintln!` **实测**数（floor 护栏 / 真实数回填、绝不预填，ADR-013）；0 新 dep / 0 migration / 0 默认构建改动 — verified by **TEST-36.1.1**（live recall harness env-gated——live qdrant 实测 recall@k ≥ floor / 无 server honest-defer 干净跳过不 fail）+ **TEST-36.1.2**（deterministic generator 可复现——无 server 也跑、同 seed ⇒ 同向量）+ phase-smoke step 1
- [x] **AC2**（qdrant-recall-ci-service；CI-only / add-only）: `.github/workflows/ci.yml` 加 `qdrant-recall` job（qdrant service container `image: qdrant/qdrant`，ports `6334:6334` + `6333:6333`；toolchain 1.93 + protoc；`QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture`）→ 每次 CI run 对 live service container 跑 harness、recall 永久被验证、**关闭** `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（CI 现有 live server）；CI-only / add-only / 默认 `cargo-test` 仍默认构建 0 vector dep（ADR-004 行为不变）；**验证证据 = PR 自身 live CI run**（真实、据实记录 run id / 链接，绝不预填，ADR-013）— verified by **TEST-36.2.1**（`qdrant-recall` CI job 对 live service container 跑 harness 且绿，经 PR 自身 CI run = 真实证据）+ phase-smoke step 2
- [x] **AC3**（v0.29.0 closeout 🟢）: v0.29.0 release docs（evidence/artifacts/README/RELEASE_NOTES，**真实 recall@k 数 + 真实 `qdrant-recall` CI run 链接**）+ `scripts/console_smoke.sh` v26[45/45]（staging `cf-v28-cfg`，offset +2）+ `internal/cli/smoke_syntax_test.go` `TestTask363_SmokeV26QdrantLiveRecallStep` markers 同步（no-regression [37/37]..[44/44] 不溯改）+ ADR-041 据真实数 per-D ratify（D2 真实召回数据回填）+ ADR-034 add-only Phase-36 Amendment（标 D2 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` fulfilled、不溯改 D-body）+ roadmap §3.18/§4 add-only + phase §6 闭合 — verified by **TEST-36.3.1**（smoke v26[45/45] + `bash -n scripts/console_smoke.sh` + `go test -run TestTask363` + release docs + ADR-041 per-D ratify + ADR-034 Amendment + roadmap/adapter add-only）+ phase-smoke step 3
- [x] **AC4**（ADR-014 cross-validation gate）: ADR-014 D1-D5（**第二十七次**激活）全通过 — D1 mapping（Phase 36 → v0.29.0）+ D2 lint `bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-35 不溯改（ADR 改动 add-only Amendment；ADR-034 D2 D-body / `## Ratification` 不溯改；denominators [37/37]..[44/44] 不溯改）— verified by task-36.3 closeout PR body + 各 task LAST D2 lint TEST（**TEST-36.1.3** / **TEST-36.2.2** / **TEST-36.3.2**）

**端到端 smoke（C1 集成兜底）**：(1) `core/tests/qdrant_live_recall.rs` deterministic generator 无 server 也跑（同 seed ⇒ 同向量 TEST-36.1.2 PASS）+ 对 live qdrant 实测 recall@k ≥ floor（k=10 → 0.90）+ `eprintln!` 实测数（真实跑出后回填）/ 无 server 时 `health() != Ready` honest-defer 干净跳过、不 fail、零召回数输出（不伪造，ADR-013）全 PASS；(2) `.github/workflows/ci.yml` `qdrant-recall` job 对 qdrant service container 跑 `cargo test ... --test qdrant_live_recall` 绿（验证证据 = PR 自身 live CI run，据实记录）、默认 `cargo-test` 仍默认构建 0 vector dep 不退化全 PASS；(3) `scripts/console_smoke.sh` v26[45/45]（staging `cf-v28-cfg`）+ `TestTask363_SmokeV26QdrantLiveRecallStep` + ADR-041 ratify（D2 真实召回数据回填）+ ADR-034 Phase-36 Amendment（标 qdrant-server-lifecycle fulfilled）全 PASS。

## 7. 阶段级风险

- **R1（高）CI service container qdrant 启动 / 网络 / 镜像可用性导致 `qdrant-recall` job flake 或 false-red**：service container 拉取 `qdrant/qdrant` 镜像 + 端口映射 + 启动就绪时序若不稳，会令 job flake。
  - **缓解**：task-36.1 harness 经 `QdrantConnConfig::from_env()` + `health()`（:184-189）守门——`!= Ready` 时 honest-defer 干净跳过（不 fail），故即便 service 未就绪也不会 false-red（仅 skip）；task-36.2 用官方 `qdrant/qdrant` 镜像 + 标准 service container 端口映射（6334 gRPC / 6333 REST）+ toolchain 1.93（与既有 job 一致）；验证证据 = PR 自身 live CI run 据实记录（若 job 持续 flake 则如实记录受阻、不伪造绿）。stop-condition：若 `qdrant-recall` job 非确定性 flake / false-red 则 AC2 不标 `[x]`（先稳定 service 就绪或保留 honest-defer skip 语义）。
- **R2（高）真实 recall@k 低于 floor / floor 设置失当**：qdrant HNSW ANN 是近似召回，recall@k 可能低于预设 floor（如 k=10 → 0.90），或 floor 设得过松失去护栏意义。
  - **缓解**：task-36.1 floor 为**文档化护栏**、真实数被 `eprintln!` 报告并据真实跑出回填（ADR-013，绝不预填）；floor 取保守可达值（lead de-risk 已证明 KNN 余弦序正确），N/D/M/k 取 modest 可复现值；若真实 recall < floor 则先据真实数调 floor 或诊断 backend（qdrant HNSW 参数），**不伪造召回数充门**。stop-condition：若真实 recall < floor 且无法据真实数合理校准则如实记录受阻、不强标 AC1 live 维度 `[x]`（generator 维度 TEST-36.1.2 deterministic 达成则部分 ratify，ADR-013）。
- **R3（中）harness 误破默认构建 0-vector-dep / 引入新 dep**：live recall 主题易诱导引 `rand` crate 生成随机向量或在默认构建拉入 qdrant 依赖。
  - **缓解**：task-36.1 `#![cfg(feature = "vector-qdrant")]` 令 harness 默认构建不编译；deterministic 向量用 index-seeded splitmix64/LCG（**无 `rand` crate / 无 clock**，ADR-013 可复现）；`qdrant-client` 自 task-18.4 已 optional（0 新 dep，ADR-008）；默认 `cargo test --workspace` 不含该 test、0 vector dep（ADR-004）。stop-condition：若默认构建引入 vector dep / 新 dep / `rand` crate 则越界、AC 不标 `[x]`。
- **R4（中）误溯改 ADR-034 D2 D-body / `## Ratification`**：兑现 ADR-034 D2 的 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` 时易直接编辑其 D2 正文或 Ratification 段。
  - **缓解**：task-36.3 仅以 **add-only Phase-36 Amendment** 标 D2 维度 fulfilled（live KNN recall 已测 + CI-guarded），**不溯改 D2 D-body / 不溯改 `## Ratification (v0.22.0 / task-29.4)` / 不溯改既有 Phase 32 Amendment**（ADR-014 D5）；ADR-041 为本 phase 新主 ADR。stop-condition：若溯改 ADR-034 任何既有正文 / Ratification 则越界，改为 add-only Amendment。

## 8. Definition of Done

- 3 task spec（36.1-36.3）顶部 `**Status**: Done`
- §6 阶段级 AC1-4 全 `[x]`（受阻 / 延后态按 ADR-013 如实记录，不伪造——如真实 recall < floor 据真实数校准或如实记录 / CI service flake 如实记录 / qdrant 集群拓扑据实延后 `[SPEC-DEFER:phase-future.qdrant-deployment-topology]` / 语义 golden 召回据实延后 `[SPEC-DEFER:phase-future.qdrant-semantic-golden-recall]` / 多 backend live recall 据实延后 `[SPEC-DEFER:phase-future.multi-backend-production]`）
- 端到端 smoke 3 step 全 PASS（含受阻 / 延后态如实标注 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`）
- **ADR**：ADR-041 `Proposed → Accepted`（据真实 recall 数 + 真实 `qdrant-recall` CI run 逐 D 项 ratify，D2 真实召回数据回填——floor-only 直至 CI run 跑出真实数，不强 ratify、不伪造）；ADR-034（production-vector-live-recall 母 ADR）经 add-only **Phase-36 Amendment** 标其 D2 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` fulfilled（live KNN recall 已测 + CI-guarded，不溯改 D2 D-body / `## Ratification`，ADR-014 D5）；ADR-004（默认构建 0 vector dep + 0 网络 + 行为不变）/ ADR-008（Phase 36 不增 dep、`qdrant-client` 自 task-18.4 已 optional）守线引用；`docs/roadmap.md §3.18/§4` add-only（Phase 36 行 + `qdrant-server-lifecycle` progressed→fulfilled + qdrant-semantic-golden-recall / multi-backend-production / qdrant-deployment-topology 新/续 backlog 条目）
- **adapter**：§Phase 索引 Phase 36 `Draft → Done` + `Tasks 0 → 3`；§ADR 索引 ADR-041；§BDD 追加 phase-36 feature 行；ADR-034 Amendment 记录
- **release**：`docs/releases/v0.29.0-{evidence,artifacts}.md`（真实 recall 数 + 真实 CI run 链接）+ `RELEASE_NOTES.md` v0.29 段 + README v0.29 段
- **smoke**：`scripts/console_smoke.sh` v26[45/45]（qdrant live recall verify + 既有 step 不退化，staging `cf-v28-cfg` offset +2，denominators [37/37]..[44/44] 不溯改）+ `internal/cli/smoke_syntax_test.go` `TestTask363_SmokeV26QdrantLiveRecallStep` markers 同步
- **feature**：`test/features/phase-36-qdrant-live-vector-recall.feature` 已于本 phase 创建
- **follow-up**：qdrant 语义 golden 召回 `[SPEC-DEFER:phase-future.qdrant-semantic-golden-recall]` + qdrant 部署拓扑 `[SPEC-DEFER:phase-future.qdrant-deployment-topology]` + 多 backend live recall `[SPEC-DEFER:phase-future.multi-backend-production]` + 大语料性能 `[SPEC-DEFER:phase-future.vector-large-corpus-perf]` 留 backlog
