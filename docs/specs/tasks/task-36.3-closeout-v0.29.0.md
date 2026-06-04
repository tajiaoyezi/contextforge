# Task `36.3`: `closeout-v0.29.0 — qdrant-live-vector-recall closeout（qdrant HNSW ANN recall@k vs BruteForce exact KNN 真实 live 召回兑现 + CI service-container 永久守护，关闭 [SPEC-DEFER:phase-future.qdrant-server-lifecycle]）+ v0.29.0 closeout（smoke v26 step [45/45] + TestTask363 + release docs（真实召回数 + 真实 CI run link，<backfill>）+ ADR-041 据 D1-D4 ratify + ADR-034 add-only Phase 36 Amendment（D2 qdrant-server-lifecycle fulfilled，不溯改 D-body）+ roadmap §3.18/§4 + adapter）`

**Status**: Draft

**Priority**: P2
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 36 (qdrant-live-vector-recall)
**Dependencies**: task-36.1（qdrant-live-recall-harness — 新增 `core/tests/qdrant_live_recall.rs`，`#![cfg(feature = "vector-qdrant")]`，env-gated 在 `QDRANT_URL`（复用 `QdrantConnConfig::from_env()` 读 `QDRANT_URL` + 可选 `QDRANT_API_KEY`，tls 由 https scheme 推断）；`QdrantBackend::health() != Ready` → `eprintln` skip 提示 + 干净 return（honest-defer，无 server 的 CI/本地干净 skip 不 fail，[SPEC-DEFER:phase-future.qdrant-server-lifecycle]）；构造确定性可复现 N=1000 `VectorChunk` 语料（dim D=64 确定性伪随机单位向量，按 index 种子化，无 randomness / 无时钟，可复现 ADR-013）；同一语料同时索引进 `QdrantBackend`（open ensure-create + index_batch）+ `BruteForceVectorBackend`；M=50 确定性查询向量计算 BruteForce 精确 top-k（ground truth）+ qdrant top-k，`recall@k = mean(|qdrant_topk ∩ exact_topk| / k)`；断言 `recall@k >= floor`（如 k=10 floor=0.90，guard）+ `eprintln` 真实实测值（真实值在 task-36.2/closeout 回填，ADR-013 禁伪造）；TEST-36.1.1（live recall harness env-gated — 真实 recall@k >= floor against live qdrant）+ TEST-36.1.2（确定性语料生成器可复现 — 无 server 跑，断言同 seed ⇒ 同向量）+ TEST-36.1.3（= LAST，D2 lint）；0 新 dep / 0 schema migration / 0 默认构建变更）/ task-36.2（qdrant-recall-ci-service — `.github/workflows/ci.yml` 加 `qdrant-recall` job 用 qdrant SERVICE CONTAINER（`services: qdrant: image: qdrant/qdrant`，ports `6334:6334` + `6333:6333`），Rust toolchain 1.93 + 装 protoc，跑 `QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture`；每次 CI run 对 live service container 跑 harness → recall 永久验证，关闭 [SPEC-DEFER:phase-future.qdrant-server-lifecycle]（CI 现在 HAS live server）；CI-only / add-only / 默认构建 + 行为不变（ADR-004）；TEST-36.2.1（qdrant-recall CI job 对 live 跑 harness 且绿 — 由 PR 自身 CI run = real evidence 验证）+ TEST-36.2.2（= LAST，D2 lint）；CI 配置变更，验证凭据是真实 live CI run，如实记录 ADR-013）全 Done / ADR-041（qdrant-live-vector-recall，本 task ratify）/ ADR-034（production-vector-live-recall，v0.22.0 母 ADR——本 task add-only Phase 36 Amendment：标记其 D2 [SPEC-DEFER:phase-future.qdrant-server-lifecycle] **已兑现**（live KNN recall measured + CI-guarded），不溯改 ADR-034 D-body 正文 ADR-014 D5）/ ADR-030（production-vector-backend，qdrant backend 生命周期契约层母 ADR——本 phase 在其契约层之上真实跑通 live KNN）/ ADR-004（默认行为 / 既有契约不变——vector-qdrant opt-in，默认构建 0-vector-dep / 0-network）/ ADR-008（dep add-only，Phase 36 = 0 new dep——qdrant-client 自 task-18.4 起即 optional）/ ADR-012（tag/release outward-facing 须用户显式授权）/ ADR-013（禁伪造红线——真实召回数 / 真实 CI run 跑出后回填，无 server 时 honest-defer 干净 skip 不伪造 KNN 通过）/ ADR-014 D1-D4（第二十七次激活）

## 1. Background

Phase 36（qdrant-live-vector-recall）兑现一个自 Phase 25/29 起被 honest-defer 的真实维度：qdrant `VectorBackend`（`core/src/retriever/vector/qdrant.rs`）的 connect / health / open（ensure-create via `decide_ensure`）/ index_batch（upsert）/ search（KNN, cosine）/ delete 自 Phase 25/29 起即**完整实现**，但**真实 live 端到端 KNN 召回**因 CI 无在跑的 qdrant server 而被诚实延后（ADR-034 D2 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`）；仓内唯一的召回数（`eval_integration.rs` 0.7/0.85）是**合成 fixture** 而非真实测量。Phase 36 关闭它：(1) 建一个 env-gated live recall harness，测 qdrant HNSW ANN `recall@k` vs BruteForce exact KNN（同一嵌入式语料）；(2) 对一台**真实** qdrant server 跑（DE-RISK 已被 lead 证实：真实 qdrant + qdrant-client 1.18 端到端 round-trip 通过、KNN 正确——query `[1,0,0,0]` 返回 `[(a,1.0),(c,0.994)]` 正确 cosine 序）；(3) 经 qdrant SERVICE CONTAINER 接入 CI，每次 CI run 验证召回——**永久关闭**该 defer。默认构建不变：vector-qdrant opt-in，默认构建 0-vector-dep / 0-network（ADR-004/008）；0 新 dep（qdrant-client 自 task-18.4 起即 optional）。

两个实现 task 全 Draft（实施授权另行）：36.1（qdrant-live-recall-harness——新增 `core/tests/qdrant_live_recall.rs`，`#![cfg(feature = "vector-qdrant")]`，env-gated 在 `QDRANT_URL`（复用 `QdrantConnConfig::from_env()`）；`health() != Ready` 时 `eprintln` skip 提示 + 干净 return（无 server 不 fail）；确定性可复现 N=1000 dim=64 单位向量语料（按 index 种子化，无 randomness/时钟）同时索引进 `QdrantBackend` + `BruteForceVectorBackend`；M=50 确定性查询，`recall@k = mean(|qdrant_topk ∩ exact_topk| / k)`，断言 `>= floor`（k=10 floor=0.90）+ `eprintln` 真实实测值）/ 36.2（qdrant-recall-ci-service——`.github/workflows/ci.yml` 加 `qdrant-recall` job 用 qdrant service container 每次 CI run 跑 harness against live service → recall 永久验证，关闭 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`）。

本 task 收口 v0.29.0：smoke v26 + release docs（真实召回数 + 真实 CI run link）+ ADR-041 据真实结果 ratify + ADR-034 add-only Phase 36 Amendment（标 D2 qdrant-server-lifecycle 已兑现，不溯改 D-body）+ roadmap §3.18 推进记录 + §4 add-only backlog + phase §6 闭合 + adapter + feature。

**(A) 真实兑现 vs 合成 fixture（ADR-013 核心价值）**：仓内既有召回数（`eval_integration.rs` 0.7/0.85）是合成 fixture——本 phase 用 **qdrant HNSW ANN recall@k vs BruteForce exact KNN** 这一 model-free / 可复现的真实度量取代之；真实召回数由 task-36.2 的 live CI service container run 跑出后回填（用 `待回填` 标记，绝不预填伪造，ADR-013）。harness 在无 server 时 `eprintln` skip + 干净 return（honest-defer，不伪造 KNN 通过）。

**(B) v0.29.0 closeout**：smoke v25 step `[44/44]`（Phase 35 live）顺接 v26 step `[45/45]`（banner v25→v26，staging `cf-v28-cfg`，offset +2）+ `TestTask363`（mirror `TestTask353`，无回归 `[37/37]`..`[44/44]`）+ `docs/releases/v0.29.0-{evidence,artifacts}.md`（真实 recall 数 + 真实 CI run link + tag/run/digest 用 `<backfill>` 待回填）+ README v0.29 段 + RELEASE_NOTES v0.29.0 段 + ADR-041 Proposed→Accepted（per-D ratify）+ ADR-034 add-only Phase 36 Amendment + roadmap §3.18 + §4 + phase-36 §6 闭合 + adapter + feature。

## 2. Goal

(A) 把 qdrant live KNN 召回从**合成 fixture** 推进到**真实测量 + CI 永久守护**：新增 env-gated harness 测 qdrant HNSW ANN `recall@k` vs BruteForce exact KNN（同一确定性可复现语料），对真实 qdrant server 跑（DE-RISK 已证），并经 qdrant SERVICE CONTAINER 接入 CI，每次 run 验证召回——**永久关闭** ADR-034 D2 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（CI 现在 HAS live server）。真实召回数由 live CI run 跑出后回填，无 server 时 harness `eprintln` skip + 干净 return（honest-defer 不伪造，ADR-013）。0 新 dep / 0 schema migration / 0 默认构建变更。

(B) 据 36.1/36.2 **真实 CI / 实测产物**收口 v0.29.0：ADR-041 `Proposed → Accepted`（逐 D 如实——D1 live recall harness（qdrant HNSW ANN recall@k vs BruteForce exact KNN 方法论；确定性可复现语料；无 server 时 env-gated honest-defer）、D2 真实实测召回数（`待回填` until task-36.2 run，ADR-013 禁伪造）、D3 CI service-container 集成（`ci.yml` 加 qdrant service → 每次 run 验证召回，永久关闭 CI-no-server defer）、D4 默认 0-vector-dep baseline + 行为不变（vector-qdrant opt-in；ADR-004/008；0 新 dep））；ADR-034 add-only Phase 36 Amendment（标其 D2 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` **已兑现**：live KNN recall measured + CI-guarded，不溯改 ADR-034 D-body 正文 ADR-014 D5）；roadmap §3.18（Phase 36 推进记录）+ §4 add-only（新 backlog）；phase-36 §6 AC 置 `[x]` + Status Done；smoke v26 step `[45/45]`；release docs（真实召回数 + 真实 CI run link + tag/run/digest 用 `<backfill>`）；adapter（Phase 36 Done + Tasks 3 + ADR-041 Accepted + feature 行）。**真实 v0.29.0 tag/release 须用户显式授权**（不自行越界 tag，ADR-012）。

pass bar：(A) qdrant live recall harness env-gated 接入 CI service container、真实 recall@k >= floor 经 live CI run 实证（或 honest-defer 干净 skip 如实记录），ADR-034 D2 qdrant-server-lifecycle 标已兑现；(B) smoke `bash -n` 过 + `go test -run TestTask363` 过 + 文档闭合人工核 + ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- qdrant live recall **真实兑现记录**（合成 fixture → 真实 qdrant HNSW ANN recall@k vs BruteForce exact KNN）：(a) task-36.1 env-gated harness（确定性可复现语料 + recall@k 度量 + honest-defer 无 server skip）的真实测试结论；(b) task-36.2 CI service-container 集成（每次 CI run 对 live service 跑 harness）的真实 CI run 凭据；(c) ADR-034 D2 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` 标已兑现（live KNN recall measured + CI-guarded）。**记录于 §范围外 + ADR-041 Context + D1-D3 + ADR-034 add-only Phase 36 Amendment，真实召回数 / CI run 待回填**（ADR-013 禁伪造）。
- `scripts/console_smoke.sh`——banner v25→v26 + v26 changelog 块 + step `[45/45]`（doc/status 断言 qdrant-live-vector-recall baseline：qdrant-live-recall-harness + qdrant-recall-ci-service + qdrant-server-lifecycle defer 关闭；default build init baseline 不变 + denominator 不溯改 ADR-014 D5），staging `cf-v28-cfg`（offset +2）。当前 live 脚本 v25 `[44/44]`（Phase 35）；故 Phase 36 顺接 `[45/45]`。
- `internal/cli/smoke_syntax_test.go`——新增 `TestTask363_SmokeV26QdrantLiveRecallStep`（mirror `TestTask353`，断言 `v26 (task-36.3)` header + `[45/45]` + 标记（`qdrant-live-vector-recall` / `TEST-36.1.` / `TEST-36.2.` / `TEST-36.3.` / `recall@k` / `qdrant_live_recall`）+ 无回归既有 `[37/37]`..`[44/44]`，denominator 不溯改 ADR-014 D5 + `bash -n` 语法）。
- 新增 `docs/releases/v0.29.0-{evidence,artifacts}.md`（真实 recall@k 数 + 真实 `qdrant-recall` CI run link + tag SHA / run id / digest 用 `<backfill>` 待回填）+ `README.md` v0.29 段 + `RELEASE_NOTES.md` v0.29.0 段。
- `docs/decisions/adr-041-qdrant-live-vector-recall.md`——Status Proposed→Accepted（per-D 限定）+ `## Ratification（v0.29.0 / task-36.3）` 节（逐 D 真实依据；D1 harness 方法论 + 确定性可复现语料 + honest-defer、D2 真实实测召回数 `待回填` until task-36.2 live CI run、D3 CI service-container 集成（`ci.yml` qdrant service → 每次 run 验证）、D4 默认 0-vector-dep + 行为不变 + 0 新 dep）。
- add-only Amendment（不溯改正文，ADR-014 D5）：`docs/decisions/adr-034-production-vector-live-recall.md`——`## Amendment (Phase 36 / v0.29.0)`（标其 D2 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` **已兑现**：经 task-36.1 env-gated harness（qdrant HNSW ANN recall@k vs BruteForce exact KNN）+ task-36.2 CI service-container（每次 CI run 对 live qdrant 跑 harness）真实 live KNN recall measured + CI-guarded；不溯改 ADR-034 D2 D-body 正文 + 既有 `## Ratification (v0.22.0)` / `## Amendment (Phase 32)` 正文）。
- `docs/roadmap.md`——§3 新增 §3.18 Phase 36 推进记录 + §4 add-only（标 `qdrant-server-lifecycle` 已兑现/progressed；新 backlog 条目：qdrant-semantic-golden-recall（vs golden semantic labels 需真实 embedding model，`[SPEC-DEFER:phase-future.qdrant-semantic-golden-recall]`）/ qdrant-deployment-topology（集群/复制拓扑，`[SPEC-DEFER:phase-future.qdrant-deployment-topology]`）；add-only 不删旧条目正文）。
- `docs/specs/phases/phase-36-qdrant-live-vector-recall.md`——Status Draft→Done + §6 AC `[x]`（honest per-item：36.1 harness env-gated honest-defer + 确定性可复现 🟢 / 36.2 CI service-container live run 🟢 / 真实召回数据 `待回填` until live CI run 如实标注）。
- `docs/s2v-adapter.md`——§Phase 36 In Progress→Done + Tasks 2→3；§Task +36.3；§ADR 041 Proposed→Accepted；§BDD +phase-36 行。
- `test/features/phase-36-qdrant-live-vector-recall.feature`（已创建）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER] / honest record）

以下经 grounding 校正为 **honest-defer / 真实数据待回填，不在 spec 预填伪造**（须在 spec 与 ADR-041 如实记录）：

- **真实实测 recall@k 数 = 真实 live CI run 跑出后回填（不预填，ADR-013）**：harness 断言的是 floor（k=10 floor=0.90 = guard），真实实测值（`recall@10` 等）由 task-36.2 的 qdrant service-container live CI run 跑出后回填到 ADR-041 D2 + `docs/releases/v0.29.0-evidence.md`，spec / ADR body 用 `待回填` 标记，绝不预填伪造数。无 server（本地无 `QDRANT_URL`）时 harness `eprintln` skip + 干净 return（honest-defer，不伪造 KNN 通过）。
- 真实 v0.29.0 tag push + release run（cosign 真签 + GHCR 推送）[SPEC-OWNER:user-authorized-release]——outward-facing 不可逆须用户显式授权（ADR-012）；post-tag-push backfill 填实 tag SHA / run id / digest，本 task body 不预填真实凭据。
- recall vs golden semantic labels（需真实 embedding model 的语义 golden 召回）[SPEC-DEFER:phase-future.qdrant-semantic-golden-recall]——qdrant-vs-exact-KNN 度量是 model-free + 可复现的干净主度量（ADR-041 A3）；语义 golden 召回需真实 embedding model + 标注语料，超本 phase 范围 honest-defer。
- qdrant 集群 / 复制部署拓扑（多节点 / sharding / replication 召回）[SPEC-DEFER:phase-future.qdrant-deployment-topology]——本 phase live recall 针对单节点 service container baseline；集群/复制拓扑超本 phase 范围 honest-defer（承 ADR-034 Follow-ups）。
- 多 backend 生产化（lancedb / sqlite-vec 的等价 live CI service container 召回守护）[SPEC-DEFER:phase-future.multi-backend-production]——本 phase 聚焦 qdrant live recall + CI service container；其余 backend 的等价 CI live 守护超本 phase 范围 honest-defer。

## 4. Actors

- 主 agent（ADR-012 自治；真实 release 须用户授权 ADR-012）
- `core/tests/qdrant_live_recall.rs` harness（`#![cfg(feature = "vector-qdrant")]` env-gated 在 `QDRANT_URL`——task-36.1 落地：health != Ready → eprintln skip + 干净 return（honest-defer），确定性可复现 N=1000 dim=64 语料同时索引进 `QdrantBackend` + `BruteForceVectorBackend`，M=50 查询 `recall@k = mean(|qdrant_topk ∩ exact_topk| / k)` 断言 >= floor + eprintln 实测值，本 closeout 经 ADR-041 D1/D2 ratify）
- `QdrantBackend` connect / health / open（ensure-create via `decide_ensure`）/ index_batch（upsert）/ search（KNN, cosine）（`core/src/retriever/vector/qdrant.rs`——Phase 25/29 已实现，本 phase 首次真实 live KNN recall 测量经 harness 兑现，本 closeout 经 ADR-041 D1 ratify）
- `qdrant-recall` CI job（`.github/workflows/ci.yml` qdrant service container `image: qdrant/qdrant` ports `6334:6334`+`6333:6333`，Rust 1.93 + protoc，跑 `QDRANT_URL=http://localhost:6334 cargo test ... --test qdrant_live_recall`——task-36.2 落地：每次 CI run 对 live service 跑 harness，本 closeout 经 ADR-041 D3 ratify + 真实 CI run link 回填）
- ADR-034 D2 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（v0.22.0 母 ADR 的 honest-defer——本 closeout 经 add-only Phase 36 Amendment 标已兑现：live KNN recall measured + CI-guarded，不溯改 D-body）
- closeout 文档集（smoke / release docs / ADR-041 ratify / ADR-034 add-only Phase 36 Amendment / roadmap §3.18+§4 / phase spec / adapter / feature）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/retriever/vector/qdrant.rs`（`QdrantBackend` connect / `health()`（Ready / Unreachable）/ open（ensure-create via `decide_ensure`）/ `index_batch`（upsert）/ `search`（KNN, cosine）/ delete——Phase 25/29 已实现，task-36.1 harness 经其 live 读/写路径测真实 recall@k；`QdrantConnConfig::from_env()` 读 `QDRANT_URL` + 可选 `QDRANT_API_KEY`，tls 由 https scheme 推断——harness env-gate 锚点）
- `core/src/retriever/vector/brute_force.rs`（`BruteForceVectorBackend` exact KNN——harness ground-truth top-k 来源，同语料同时索引）
- `core/src/retriever/vector/traits.rs`（`VectorChunk { chunk_id: ChunkId(String), embedding: Vec<f32>, metadata: Option<serde_json::Value> }` + `VectorIndexConfig { dim, metric: VectorMetric::Cosine, persistence_path: None, collection_id: String }`——harness 语料/索引配置类型契约，本 ADR 不改三 trait 签名）
- `core/examples/phase29_recall_via_qdrant.rs`（task-29.2 的 connect→ensure-create→upsert→KNN over live server + health 守门 honest-defer wiring——task-36.1 harness 的方法论先例 + honest-defer 锚点）
- `core/Cargo.toml`（`vector-qdrant` feature 下 qdrant-client optional dep，自 task-18.4 起即 optional——Phase 36 = 0 新 dep，ADR-008；默认构建不编译该 feature，ADR-004）
- `.github/workflows/ci.yml`（既有 jobs——task-36.2 加 `qdrant-recall` job 用 qdrant service container（`services: qdrant: image: qdrant/qdrant`，ports `6334:6334`+`6333:6333`），Rust toolchain 1.93 + 装 protoc，跑 `QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture`——CI-only / add-only / 默认构建 + 行为不变锚点）
- `core/tests/eval_integration.rs`（既有合成 fixture 召回数 0.7/0.85——本 phase 用真实 qdrant HNSW ANN recall@k vs BruteForce exact KNN 取代合成度量的 grounding 锚点，ADR-013）
- `docs/specs/tasks/task-36.1-qdrant-live-recall-harness.md §10` + `task-36.2-qdrant-recall-ci-service.md §10`（真实测试结果 + 真实 CI run 凭据——ADR-041 ratify 依据）
- `docs/decisions/adr-041-qdrant-live-vector-recall.md`（§D1-D4 + Consequences Ratification 条款）
- `docs/decisions/adr-034-production-vector-live-recall.md`（§Decision D2 + `## Ratification (v0.22.0 / task-29.4)` D2 partial + Follow-ups `[SPEC-DEFER:phase-future.qdrant-ci-service]`——本 task add-only Phase 36 Amendment 落点：标 D2 qdrant-server-lifecycle 已兑现，不溯改 D-body）+ `docs/decisions/adr-030-production-vector-backend.md`（qdrant backend 生命周期契约层母 ADR——本 phase 在其契约层之上真实跑通 live KNN）
- `internal/cli/smoke_syntax_test.go`（`TestTask353_SmokeV25ObservabilityHardeningStep`——本 task `TestTask363` mirror 源）+ `scripts/console_smoke.sh`（v25 `[44/44]` 块 + banner，cf-v27-cfg → 本 task cf-v28-cfg offset +2）
- `docs/releases/v0.28.0-{evidence,artifacts}.md`（release docs 模板）

### 5.2 关键设计 — qdrant HNSW ANN recall@k vs BruteForce exact KNN + 诚实 per-D ratify + backfill 待回填

- **qdrant HNSW ANN recall@k vs BruteForce exact KNN（model-free / 可复现真实度量，ADR-013 核心价值）**：harness（`core/tests/qdrant_live_recall.rs`，`#![cfg(feature = "vector-qdrant")]`）env-gated 在 `QDRANT_URL`（复用 `QdrantConnConfig::from_env()`）；构造确定性可复现 N=1000 `VectorChunk` 语料——dim D=64 确定性伪随机单位向量，按 index 种子化（无 randomness / 无时钟，同 seed ⇒ 同向量，可复现 ADR-013）；同一语料同时索引进 `QdrantBackend`（open ensure-create + index_batch upsert）+ `BruteForceVectorBackend`；M=50 确定性查询向量计算 BruteForce 精确 top-k（ground truth）+ qdrant top-k，`recall@k = mean(|qdrant_topk ∩ exact_topk| / k)`；断言 `recall@k >= floor`（k=10 floor=0.90 = guard）+ `eprintln` 真实实测值。`health() != Ready` 时 `eprintln` skip 提示 + 干净 return（honest-defer——无 server 的本地/CI 干净 skip 不 fail，不伪造 KNN 通过，ADR-013）。DE-RISK 已被 lead 证实（真实 qdrant + qdrant-client 1.18 round-trip 通过、KNN 正确：query `[1,0,0,0]` → `[(a,1.0),(c,0.994)]` 正确 cosine 序）。pass bar：harness env-gated + 确定性可复现 + recall@k >= floor（经 live CI run 实证）+ 真实实测值回填。0 新 dep。
- ADR-041 ratify **逐 D 项据真实结果**：D1（live recall harness——qdrant HNSW ANN recall@k vs BruteForce exact KNN 方法论；确定性可复现语料（同 seed ⇒ 同向量，TEST-36.1.2 无 server 跑实证）；无 server 时 env-gated honest-defer（health != Ready → eprintln skip + 干净 return）🟢）/ D2（真实实测召回数——`recall@k` 真实值 `待回填` until task-36.2 的 qdrant service-container live CI run 跑出，断言 floor（k=10 floor=0.90）是 guard、真实数是报告值，ADR-013 禁伪造）/ D3（CI service-container 集成——`ci.yml` `qdrant-recall` job 用 qdrant service container（`image: qdrant/qdrant`，ports `6334:6334`+`6333:6333`），每次 CI run 对 live service 跑 harness → recall 永久验证，**关闭** CI-no-server defer；TEST-36.2.1 由 PR 自身 CI run = real evidence 验证，真实 CI run link `待回填` 🟢）/ D4（默认 0-vector-dep baseline + 行为不变——vector-qdrant opt-in，默认构建 0-vector-dep / 0-network；ADR-004/008；0 新 dep（qdrant-client 自 task-18.4 起即 optional）；0 proto / 0 migration / 三 trait 签名不变 🟢）。各 D 真实测试 / 实测结果待 36.1-36.2 实施 + live CI run 跑出后回填，不为「全 Accepted」预填伪造召回数（ADR-013）。
- ADR-034 add-only Phase 36 Amendment 为 **add-only 注记**（不删/不改 ADR-034 D-body 正文 + 既有 `## Ratification (v0.22.0 / task-29.4)` / `## Amendment (Phase 32 / v0.25.0)` 正文）：标其 D2 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` **已兑现**——经 task-36.1 env-gated harness（qdrant HNSW ANN recall@k vs BruteForce exact KNN）+ task-36.2 CI service-container（每次 CI run 对 live qdrant 跑 harness）真实 live KNN recall measured + CI-guarded。ADR-034 D2 v0.22.0 ratify 时记为「🟡 PARTIAL（wiring + honest-defer Accepted；live-recall 维度 honest-defer）」+ Follow-ups `[SPEC-DEFER:phase-future.qdrant-ci-service]`，Phase 36 把该 live-recall 维度真实兑现 + CI 永久守护（不溯改 D-body，ADR-014 D5）。
- tag SHA / release run id / 镜像 digest 在 release docs 用 `<backfill: ...>` 待回填——真实 v0.29.0 tag/release 是 closeout 合入后的**用户授权步**（ADR-012），post-tag-push backfill PR 填实（承 v0.8–v0.28 pattern）。真实 recall@k 数 + 真实 `qdrant-recall` CI run link 亦 `待回填` until task-36.2 live CI run（ADR-013 禁预填伪造）。
- smoke step `[45/45]` 为文档/状态步：验 default build init baseline 不变（ADR-004，vector-qdrant opt-in 默认不编译）+ 文档化三 task 状态（qdrant-live-recall-harness + qdrant-recall-ci-service + qdrant-server-lifecycle defer 关闭），staging `cf-v28-cfg`（offset +2）。

### 5.3 不变量

- 默认行为不变（ADR-004）：vector-qdrant opt-in——默认构建（无 `vector-qdrant` feature）不编译 harness / 不连任何 server / 0-vector-dep / 0-network；`cargo test --workspace` 不受影响；harness env-gated 在 `QDRANT_URL`，无 server 时 `eprintln` skip + 干净 return（不 fail）；语义 + hybrid 路径默认仍经 0-dep BruteForce baseline。
- closeout 0 行为变更 / 0 新依赖（Phase 36 = 0 new dep，ADR-008——qdrant-client 自 task-18.4 起即 optional，harness 复用 `QdrantBackend` + `BruteForceVectorBackend` 既有 API；0 proto / 0 migration；smoke 既有 step + denominator 不溯改 ADR-014 D5；`ci.yml` 加 job 是 CI 配置 add-only 不改默认构建/行为）。
- honest-defer 守恒（ADR-013）：harness 在 `health() != Ready` 时 `eprintln` skip + 干净 return（无 server 不伪造 KNN 通过）；真实 recall@k 数 / 真实 CI run link 由 task-36.2 live CI run 跑出后回填，spec / ADR body 用 `待回填` 标记绝不预填；断言的 floor（k=10 floor=0.90）是 guard、真实数是报告值。
- ADR-014 D5：历史 Phase 1-35 spec 不溯改；ADR-034 add-only Phase 36 Amendment 不改 D-body 正文 + 既有 Ratification / Amendment 正文（仅标 D2 qdrant-server-lifecycle 已兑现）；roadmap §4 新 backlog 为 add-only 条目不删旧条目正文。
- add-only 兑现（新增 `core/tests/qdrant_live_recall.rs` + `ci.yml` `qdrant-recall` job 不改既有生产代码 / 不改 `QdrantBackend` 实现 / 不改三 trait 签名）不破既有契约（ADR-004/008）；`eval_integration.rs` 既有合成 fixture 测试不删（add-only 用真实度量补强）。
- 真实 tag/release 经用户授权后执行（ADR-012）；release docs tag/run/digest + 真实 recall@k 数 + 真实 CI run link backfill 待回填，不预填伪造凭据/数（ADR-013）。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [ ] **AC1**（qdrant live recall 真实兑现 + ADR-034 D2 qdrant-server-lifecycle 已兑现 🟢）: task-36.1 env-gated harness（`core/tests/qdrant_live_recall.rs`，确定性可复现 N=1000 dim=64 语料 + qdrant HNSW ANN recall@k vs BruteForce exact KNN + health != Ready honest-defer skip）+ task-36.2 CI service-container（`ci.yml` `qdrant-recall` job 每次 CI run 对 live qdrant 跑 harness，recall@k >= floor 经 live CI run 实证）兑现 live KNN recall measured + CI-guarded；ADR-034 add-only Phase 36 Amendment 标 D2 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` 已兑现（不溯改 D-body，ADR-014 D5）；真实 recall@k 数 + 真实 CI run link `待回填`（ADR-013 禁伪造）；0 新 dep / 0 默认构建变更 — verified by **TEST-36.3.1**。
- [ ] **AC2**（v0.29.0 closeout 🟢）: smoke banner v25→v26 + step `[45/45]`（qdrant-live-vector-recall baseline + default build baseline intact，staging `cf-v28-cfg` offset +2）+ `TestTask363_SmokeV26QdrantLiveRecallStep`（含无回归既有 `[37/37]`..`[44/44]`，denominator 不溯改）；v0.29.0 release docs（`v0.29.0-{evidence,artifacts}.md` 真实 recall@k 数 + 真实 CI run link + `<backfill>` + README v0.29 段 + RELEASE_NOTES v0.29.0 段）+ ADR-041 per-D ratify `Proposed→Accepted`（D1 harness 方法论 + 确定性可复现 + honest-defer；D2 真实实测召回数 `待回填` until live CI run；D3 CI service-container 集成关闭 CI-no-server defer；D4 默认 0-vector-dep + 行为不变 + 0 新 dep）+ ADR-034 add-only Phase 36 Amendment（标 D2 qdrant-server-lifecycle 已兑现）+ roadmap §3.18 推进记录 + §4 add-only 新 backlog + phase-36 §6 AC `[x]` + Status Done + adapter（Phase 36 Done/Tasks 3/ADR-041 Accepted）+ feature — verified by **TEST-36.3.2**。
- [ ] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中（CI spec-lint 权威）— verified by **TEST-36.3.2**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-36.3.1 | qdrant live recall 真实兑现 + ADR-034 D2 qdrant-server-lifecycle 已兑现：task-36.1 env-gated harness（确定性可复现 N=1000 dim=64 语料 + qdrant HNSW ANN recall@k vs BruteForce exact KNN + health != Ready honest-defer skip）+ task-36.2 CI service-container（`ci.yml` `qdrant-recall` job 每次 CI run 对 live qdrant 跑 harness）兑现 live KNN recall measured + CI-guarded；ADR-034 add-only Phase 36 Amendment 标 D2 qdrant-server-lifecycle 已兑现（不溯改 D-body）；真实 recall@k 数 + 真实 CI run link `待回填`（ADR-013 禁伪造）；0 新 dep | `docs/specs/tasks/task-36.3-closeout-v0.29.0.md`（§范围外）+ `docs/decisions/adr-041-qdrant-live-vector-recall.md`（Context + D1-D3）+ `docs/decisions/adr-034-production-vector-live-recall.md`（Amendment Phase 36） | Draft |
| TEST-36.3.2 | smoke v26 step `[45/45]`（qdrant-live-vector-recall baseline + qdrant-live-recall-harness/qdrant-recall-ci-service/qdrant-server-lifecycle-closed 标记 `qdrant-live-vector-recall`/`TEST-36.1.`/`TEST-36.2.`/`TEST-36.3.`/`recall@k`/`qdrant_live_recall` + 无回归既有 denominator，staging `cf-v28-cfg`）+ `bash -n` 过 + `go test -run TestTask363` 过 + v0.29.0 release docs（真实 recall@k 数 + 真实 CI run link）+ ADR-041 per-D ratify Accepted（D1 harness 方法论 + 确定性可复现 + honest-defer / D2 真实召回数 `待回填` / D3 CI service-container 关闭 defer）+ ADR-034 add-only Phase 36 Amendment（D2 qdrant-server-lifecycle 已兑现）+ roadmap §3.18+§4 + phase-36 §6 闭合 + adapter + feature + D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/console_smoke.sh` + `internal/cli/smoke_syntax_test.go` + release/ADR-041/ADR-034/roadmap/phase/adapter/feature + `scripts/spec_drift_lint.sh` | Draft |

## 8. Risks

- **R1（低）真实召回数 / 真实 CI run 预填伪造**：本 task 的真实 recall@k 数 + 真实 `qdrant-recall` CI run link 须由 task-36.2 的 qdrant service-container live CI run 跑出后回填，若预填伪造数则破 ADR-013 红线。
  - **缓解**：spec / ADR body 用 `待回填` 标记，断言 floor（k=10 floor=0.90）是 guard、真实数是报告值；harness 无 server 时 `eprintln` skip + 干净 return 不伪造 KNN 通过；真实 recall vs golden semantic labels `[SPEC-DEFER:phase-future.qdrant-semantic-golden-recall]`。stop-condition：真实 recall@k 数 + 真实 CI run link 经 live CI run 跑出回填则 AC1 标 `[x]`。
- **R2（低）ADR-034 D2 误溯改 D-body / 误报 qdrant-server-lifecycle 已兑现但 CI 未实证**：诚实风险——D2 已兑现须有真实 live CI run 凭据（TEST-36.2.1 = PR 自身 CI run real evidence），不溯改 ADR-034 D-body 正文。
  - **缓解**：ADR-034 仅追加 `## Amendment (Phase 36 / v0.29.0)` 段标 D2 qdrant-server-lifecycle 已兑现，不删/不改 D2 D-body 正文 + 既有 Ratification / Amendment 正文（D5）；「已兑现」须有 task-36.2 真实 live CI run 凭据（`待回填` until run），否则标受阻维度 / backfill。stop-condition：「D2 qdrant-server-lifecycle 已兑现」须有真实 live CI run 实证，否则 backfill。
- **R3（低）smoke denominator 误溯改 / staging offset 错位**：新 step 须 `[45/45]`、staging `cf-v28-cfg`（offset +2），既有 `[37/37]`..`[44/44]` 不动。
  - **缓解**：`TestTask363` 无回归断言守护（mirror `TestTask353`）；ADR-014 D5；staging dir `cf-v28-cfg` 顺接 v26→cf-v28（offset +2）。
- **R4（低）默认构建 / 0-dep 受 feature 污染 / CI job 误改默认行为**：harness `#![cfg(feature = "vector-qdrant")]` 默认不编译；`ci.yml` `qdrant-recall` job 须 CI-only / add-only 不改默认构建/行为。
  - **缓解**：harness feature-gated（默认 `cargo test --workspace` 不编译 / 不连 server）；qdrant-client 自 task-18.4 起即 optional（0 新 dep，ADR-008）；`qdrant-recall` job add-only（既有 jobs 不动），默认构建语义 + hybrid 仍经 0-dep BruteForce baseline（ADR-004），三 trait 签名不变（ADR-013 honest-defer 无 server skip）。

## 9. Verification Plan

```bash
# AC1 — qdrant live recall 真实兑现（人工核 §范围外 + ADR-041 Context+D1-D3 + ADR-034 Amendment Phase 36）
# harness env-gated 在 QDRANT_URL（无 server eprintln skip + 干净 return）；确定性可复现语料（TEST-36.1.2 无 server 跑）
# CI service-container（ci.yml qdrant-recall job）每次 run 对 live qdrant 跑 harness → recall@k >= floor 经 live CI run 实证
# ADR-034 add-only Phase 36 Amendment 标 D2 qdrant-server-lifecycle 已兑现（不溯改 D-body）；真实 recall@k 数 + 真实 CI run link 待回填

# AC2 — smoke 语法 + syntax test
bash -n scripts/console_smoke.sh
go test ./internal/cli/ -run TestTask363

# AC2 — 文档闭合人工核（ADR-041 Accepted + per-D / ADR-034 add-only Phase 36 Amendment /
#        roadmap §3.18 + §4 新 backlog / phase §6 [x] / adapter Done / feature 存在）
# AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master

# 既有不退化（closeout 文档+smoke+CI job add-only 不影响默认热路径；harness feature-gated 默认不编译）
cargo test --workspace && go test ./...
```

> **outward-facing 红线**：真实 v0.29.0 tag push + release run（cosign 真签 + GHCR 推送）是 closeout 合入后的**用户授权步**（ADR-012）；本 task body 不预填真实凭据，release docs 的 tag/run/digest 用 `<backfill>` 待 post-tag-push backfill 填实 [SPEC-OWNER:user-authorized-release]。
>
> **honest-defer / 真实数据待回填边界**：本 closeout 交付范围限于 qdrant live recall 真实兑现记录（🟢 harness + CI service-container 经 live CI run 实证）+ v0.29.0 closeout 文档/smoke；真实 recall@k 数 + 真实 `qdrant-recall` CI run link `待回填` until task-36.2 live CI run（ADR-013 禁预填伪造）；§范围外 honest-defer（qdrant-semantic-golden-recall `[SPEC-DEFER:phase-future.qdrant-semantic-golden-recall]` / qdrant-deployment-topology `[SPEC-DEFER:phase-future.qdrant-deployment-topology]` / multi-backend-production `[SPEC-DEFER:phase-future.multi-backend-production]`）**不实现新代码**，据 ADR-013 如实记录于 §范围外 + ADR-041。

## 10. Completion Notes (s2v 6 项标准)

> **Status**: Draft —— 本节为 Draft 待回填，待 task-36.3 实施 + task-36.2 qdrant service-container live CI run 跑出真实召回数后回填（§9 Verification 实证 / 实际改动文件 / backfill 凭据）。AC 与 §7 追踪表 Status 在实施后逐条置 `[x]` / Done。真实 recall@k 数 + 真实 CI run link + tag/run/digest 经 live CI run / 用户授权 tag/release 后回填（`待回填` / `<backfill>`，ADR-013 不预填伪造）。
