# Task `36.1`: `qdrant-live-recall-harness — 新增 core/tests/qdrant_live_recall.rs（#![cfg(feature = "vector-qdrant")]，env-gated QDRANT_URL via QdrantConnConfig::from_env），首次以「qdrant HNSW ANN recall@k vs BruteForce 精确 KNN」方法学在 live qdrant 上量真实召回：确定性可复现语料 N×dim → 同时索引进 QdrantBackend(open ensure-create + index_batch) 与 BruteForceVectorBackend → M 个确定性 query 取双方 top-k → recall@k = mean(|qdrant_topk ∩ exact_topk| / k) ≥ documented floor 并 eprintln 真实测得值；health() != Ready 时 eprintln skip notice + 干净 return（honest-defer，无 server 时 CI/本地 skip 不 fail，ADR-013）；0 新 dep / 0 schema migration / 0 默认构建变更（vector-qdrant opt-in，ADR-004/008）`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 36 (qdrant-live-vector-recall)
**Dependencies**: task-25.1（`QdrantBackend` 生命周期契约层已落地：`QdrantConnConfig::from_env`（`qdrant.rs:72-78` 读 `QDRANT_URL` 既有 + 可选 `QDRANT_API_KEY` + url scheme 推断 TLS）/ `connect`（`qdrant.rs:162-181` 懒连接 client + owned current-thread tokio runtime）/ `health()`（`qdrant.rs:184-189` live→`QdrantHealth::Ready`、无 server→`QdrantHealth::Unreachable`，不 panic、不静默成功）/ `decide_ensure`（`qdrant.rs:152-158` 纯函数）+ ensure-create `open`（`qdrant.rs:215-270` reuse/create/error）——真实 live KNN 端到端集成在 task-25.1 §3 范围外记为 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`，本 task 兑现其方法学层 + task-36.2 经 CI service container 永久关闭之）/ task-18.4（`QdrantBackend` via `qdrant-client` 1.18 + `vector-qdrant` feature 起源；`open`/`index_batch`/`search`（`qdrant.rs:330-371` live KNN 读路径：`SearchPointsBuilder` 取 KNN、cosine 直读、`id_map` 把 qdrant 数字 point id 映回 chunk id）本体）/ task-19.3（`BruteForceVectorBackend`，`brute_force.rs` 精确 O(n) cosine searcher，0-dep / 默认可用——本 task 用作 ground-truth 精确 KNN）/ task-29.2（`core/examples/phase29_recall_via_qdrant.rs` production-pipeline honest-defer harness——本 task **不**复用 FastEmbed 模型路径，改用 model-free 确定性可复现语料以保证 ADR-013 可复现性，二者方法学互补）/ ADR-041 D1-D2（live recall harness 方法学 + 真实测得召回数；Status Proposed，ratify @ task-36.3）/ ADR-034 D2（qdrant live-server 端到端 KNN，本 task + task-36.2 兑现其 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`）/ ADR-013（禁伪造红线——召回数真实跑出后回填，无 server 时 honest-defer skip 不伪造通过 / 不预填召回数）/ ADR-004（local-first-privacy-baseline，默认行为 + 默认构建 0 新 vector dep / 0 network）/ ADR-008（dep add-only，本 task = 0 新 dep，`qdrant-client` 自 task-18.4 已 optional）/ ADR-012（main-agent-governance-autonomy）/ ADR-014 D1-D4（第二十七次激活）

## 1. Background

`core/src/retriever/vector/qdrant.rs` 的 `QdrantBackend` 自 Phase 25 / Phase 29 起已**完整实现**：`connect` / `health` / `open`（经 `decide_ensure` ensure-create）/ `index_batch`（upsert）/ `search`（KNN，cosine）/ `delete`。但**真实 live 端到端 KNN 召回**在 task-25.1 §3 / ADR-034 D2 据实记为 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`——CI 无在跑的 qdrant server（`is_local()==false`，外部 server 进程）。当前 repo 内**仅有的召回数字**（`core/tests/eval_integration.rs:110` 的 `{"recall@5": 0.7, "recall@10": 0.85}`）是**合成 fixture**（EvalService CRUD 测试的预置 metrics 值），**不是真实测得的召回**。Phase 36 关闭这条 gap，本 task 是第一步——以一个 model-free、确定性可复现的「qdrant HNSW ANN recall@k vs BruteForce 精确 KNN」方法学 harness 在 live qdrant 上量真实召回：

- **B1 方法学 = qdrant ANN recall@k vs BruteForce 精确 KNN（model-free，clean primary）**：在**同一**嵌入语料上，把确定性语料同时索引进 `QdrantBackend`（HNSW ANN）与 `BruteForceVectorBackend`（精确 O(n) cosine，ground truth），对 M 个确定性 query 各取双方 top-k，`recall@k = mean(|qdrant_topk ∩ exact_topk| / k)`。这是一个**不依赖任何 embedding 模型**、纯向量空间、完全可复现的召回指标——ANN 索引（qdrant HNSW）相对精确 KNN（BruteForce）的召回度量。区别于 task-29.2 的 production-pipeline harness（FastEmbed all-MiniLM-L6-v2 经 `Retriever::search_semantic` 量语义召回，需真实模型、首跑下载）——本 task 的 model-free 度量是 clean primary（可复现 + CI 友好）。
- **B2 确定性可复现语料（ADR-013 可复现性核心）**：构造 N（如 1000）个 `VectorChunk`，每个的 embedding 是 dim D（如 64）的**确定性伪随机单位向量、以 index 为 seed**（无 `rand` crate、无时钟、无随机源）——同一 seed 必产同一向量（可复现）。M（如 50）个 query 向量同样确定性生成。**绝不**用 `std::time` / `rand` / clock 作种（ADR-013 禁不可复现造数）。
- **B3 honest-defer 守门（health() != Ready → skip 干净退出，不 fail）**：harness 构造 `QdrantBackend::connect(QdrantConnConfig::from_env())` 后**第一步** `health()`——`!= Ready`（无 server）则 `eprintln!` 一条 skip notice + `return`（测试**干净通过**，**不** fail）。这样无 server 的本地 / CI 环境**干净 skip 而非红**（honest-defer，ADR-013）；真实 server 上（task-36.2 CI service container / dev-box）`health()==Ready` 才进真实召回度量。
- **B4 floor 是 guard、真实值是报告（ADR-013 禁伪造）**：harness 断言 `recall@k >= documented floor`（如 k=10 floor 0.90）作为不退化 guard，并 `eprintln!` **真实测得**的召回数。floor 是守门；真实数字在 task-36.2 CI service-container run 真实跑出后回填 §10 + v0.29.0 evidence（Draft 阶段 **待回填**，绝不预填、绝不伪造，ADR-013）。
- **B5 de-risk 已由 lead 验证（真实非合成）**：真实 qdrant + `qdrant-client` 1.18 round-trip 已端到端跑通、KNN 正确——query `[1,0,0,0]` 返回 `[(a,1.0),(c,0.994)]`、cosine 序正确（lead 实证）。本 task 把这条 de-risked 路径制度化为可复现的 recall harness。

经核 `qdrant-client` 自 task-18.4 已 optional（`vector-qdrant` feature），`BruteForceVectorBackend` 默认可用——本 task **0 新 dep** + **0 schema migration**（纯新增 test 文件，无表）+ **0 默认构建变更**（`#![cfg(feature = "vector-qdrant")]`，默认 build 0-vector-dep / 0-network 不变，ADR-004/008）。

## 2. Goal

(1) **新增 `core/tests/qdrant_live_recall.rs`**：`#![cfg(feature = "vector-qdrant")]`（默认构建不编译此文件，0-vector-dep / 0-network 不变）；env-gated 经 `QdrantConnConfig::from_env()`（读 `QDRANT_URL` + 可选 `QDRANT_API_KEY`，TLS 由 https scheme 推断）。(2) **honest-defer 守门**：`QdrantBackend::connect(...)` 后第一步 `health()`——`!= QdrantHealth::Ready` → `eprintln!` skip notice（说明需 live qdrant + `QDRANT_URL`）+ `return`（测试**干净通过不 fail**，无 server 的本地 / CI 干净 skip，ADR-013）。(3) **确定性可复现语料**：N（如 1000）个 dim D（如 64）确定性伪随机单位向量、以 index 为 seed（无 randomness / 无 clock，可复现，ADR-013）的 `VectorChunk`。(4) **同语料双索引 + recall@k**：同一语料同时 `index_batch` 进 `QdrantBackend`（`open` ensure-create + `index_batch` upsert）**与** `BruteForceVectorBackend`；M（如 50）个确定性 query 向量各取 BruteForce 精确 top-k（ground truth）+ qdrant top-k；`recall@k = mean(|qdrant_topk ∩ exact_topk| / k)`。(5) **floor guard + 真实值报告**：断言 `recall@k >= documented floor`（如 k=10 floor 0.90）并 `eprintln!` 真实测得值——floor 守门、真实值待回填（task-36.2 CI run 真实跑出后回填 §10 + v0.29.0 evidence，绝不预填，ADR-013）。

pass bar：feature `vector-qdrant` 下 `core/tests/qdrant_live_recall.rs` 编译通过；**无 server 时**（本地 / 本 CI run 无 service container）`health()!=Ready` → eprintln skip notice + 干净通过 exit 0（**不 fail**，honest-defer）；**有 live server 时**（task-36.2 service container / dev-box）经同语料双索引量真实 `recall@k`、断言 `>= floor`、eprintln 真实值（真实数字 task-36.2 run 真实跑出后回填，绝不预填）；确定性语料生成器**无 server 即可**复现性自测（同 seed 必产同一向量，TEST-36.1.2 不连 server）；0 新 dep（ADR-008）+ 0 schema migration + 0 默认构建变更（默认 0-vector-dep / 0-network，ADR-004）；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- **新增 `core/tests/qdrant_live_recall.rs`**（`#![cfg(feature = "vector-qdrant")]`）：
  - **确定性语料生成器**：`fn deterministic_unit_vector(seed: u64, dim: usize) -> Vec<f32>`——以 `seed`（chunk index）确定性派生 dim D 个分量（如 splitmix64 / 简单 LCG 等 std-only 确定性 PRNG，**无** `rand` crate），再单位归一化；**无时钟 / 无随机源**，同 seed 必产同一向量（可复现，ADR-013）。语料 = N 个 `VectorChunk { chunk_id: ChunkId(format!("chunk-{i}")), embedding: deterministic_unit_vector(i, D), metadata: None }`；M 个 query 同法以独立 seed 偏移生成。
  - **honest-defer 守门**：`let conn = QdrantConnConfig::from_env(); let be = QdrantBackend::connect(&conn)?;` 后**第一步** `if be.health() != QdrantHealth::Ready { eprintln!("qdrant_live_recall: no live qdrant at {} (health != Ready); skipping live recall — set QDRANT_URL to a running qdrant to measure real recall (honest-defer per ADR-013).", conn.url); return; }`（测试干净通过、不 fail）。
  - **同语料双索引 + recall@k 度量**：`VectorIndexConfig { dim: D, metric: VectorMetric::Cosine, persistence_path: None, collection_id: "phase36-live-recall".to_string() }`；`QdrantBackend::open(cfg.clone())`（ensure-create）+ `index_batch(&corpus)`（upsert）；`BruteForceVectorBackend::open(cfg)` + `index_batch(&corpus)`（精确 ground truth）；对 M 个 query 各 `qdrant.search(q, k, None)` + `brute.search(q, k, None)`，取 chunk_id 集合交集，`recall@k = mean(|qdrant_topk ∩ exact_topk| / k)`。
  - **floor guard + 真实值 eprintln**：`assert!(recall_at_k >= FLOOR, "recall@{k}={recall_at_k} below floor {FLOOR}")`（如 `k=10`、`FLOOR=0.90`）；`eprintln!("qdrant_live_recall: measured recall@{k} = {recall_at_k} (N={N}, dim={D}, M={M}, floor={FLOOR})")`——真实值由 `-- --nocapture` 可见、待 task-36.2 run 回填（绝不预填，ADR-013）。
- **TEST-36.1.1**（live recall harness，env-gated）：有 live qdrant 时真实 `recall@k >= floor`（无 server honest-defer 干净 skip）。
- **TEST-36.1.2**（确定性语料生成器复现性，**无 server**）：断言同 seed → 同向量（`deterministic_unit_vector(7, D) == deterministic_unit_vector(7, D)`、且每个为单位向量），**不连 server** 即可跑（可复现性 ADR-013 守线）。
- **TEST-36.1.3**（= LAST，D2 lint）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **把 `qdrant-recall` job 加进 `.github/workflows/ci.yml`（service container）** [SPEC-OWNER:task-36.2-qdrant-recall-ci-service]——本 task 仅交付 harness；service-container 集成（每次 CI run 跑 harness 永久关闭 CI-no-server defer）由 task-36.2 落地。
- **真实测得召回数字 / 真实 CI run 链接（v0.29.0 evidence）回填** [SPEC-OWNER:task-36.3-closeout-v0.29.0]（task-36.2 CI service-container run 真实跑出后回填，ADR-013 不预填）。
- **recall vs golden 语义标签（需真实 embedding 模型）的召回度量** [SPEC-DEFER:phase-future.qdrant-semantic-golden-recall]——qdrant-vs-精确-KNN 度量是 model-free + 可复现的 clean primary（ADR-041 A3），语义 golden 召回需真实模型、不可复现地依赖模型权重，诚实延后（task-29.2 production-pipeline harness 已覆盖 model-based 语义维度的 honest-defer）。
- **qdrant 集群 / 复制 / 部署拓扑 / sharding** [SPEC-DEFER:phase-future.qdrant-deployment-topology]——本 task 仅量单节点 / service-container 的 ANN 召回，多节点拓扑诚实延后（承 task-29.2 §3）。
- **多 backend（lancedb / sqlite-vec / hnsw）live 召回矩阵** [SPEC-DEFER:phase-future.multi-backend-production]——本 task 聚焦 qdrant；其余 backend 的 live 召回矩阵诚实延后。
- **改 `core/src/retriever/vector/qdrant.rs` / `brute_force.rs` 本体** [SPEC-OWNER:task-18.4-spike-qdrant]——本 task harness 是消费方，复用既有 `connect`/`health`/`open`/`index_batch`/`search`（task-25.1 / task-18.4 freeze）、`BruteForceVectorBackend`（task-19.3），不重写 backend。

## 4. Actors

- 主 agent（ADR-012 自治）：实施 harness + 在有 server 的 CI service container（task-36.2）/ dev-box 跑真实召回回填证据。
- `core/tests/qdrant_live_recall.rs`（新增 integration test，`#![cfg(feature = "vector-qdrant")]`）：确定性语料 → qdrant ANN vs BruteForce 精确 KNN recall@k + honest-defer skip。
- `core/src/retriever/vector/qdrant.rs::QdrantBackend`（task-25.1 生命周期层 + task-18.4 KNN 本体）：harness 真实驱动 `connect`（`:162-181`）/ `health`（`:184-189`）/ `open` ensure-create（`:215-270`）/ `index_batch` upsert（`:272-298`）/ `search` live KNN（`:330-371`）。
- `core/src/retriever/vector/brute_force.rs::BruteForceVectorBackend`（task-19.3）：精确 O(n) cosine searcher，提供 ground-truth top-k（`search` `brute_force.rs:84-118`，cosine 降序 + chunk_id 破并列确定性序）。
- `QdrantConnConfig::from_env`（`qdrant.rs:72-78`）：env gate 来源（`QDRANT_URL` + 可选 `QDRANT_API_KEY` + https scheme 推断 TLS）。
- live qdrant server：外部 server 进程（`is_local()==false`）；无 server → `health()!=Ready` → honest-defer skip（本 task 不伪造）；真实召回经 task-36.2 service container（CI）/ dev-box 跑出。
- 下游 task-36.2 / task-36.3：36.2 加 `qdrant-recall` CI service-container job 跑本 harness（每次 CI run 验证召回，永久关闭 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`）；36.3 closeout 据真实召回数 ratify ADR-041 + add-only ADR-034 Phase-36 Amendment。

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/retriever/vector/qdrant.rs:72-78`（`QdrantConnConfig::from_env`：`QDRANT_URL` + 可选 `QDRANT_API_KEY` + https scheme 推断 TLS——本 task 的 env gate 来源）+ `:162-181`（`connect`：懒连接 client + owned current-thread tokio runtime）+ `:184-189`（`health()`：live→`QdrantHealth::Ready` / 无 server→`QdrantHealth::Unreachable`，不 panic——honest-defer 守门点）+ `:215-270`（ensure-create `open`，`decide_ensure` reuse/create/error，`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` anchor `:219`）+ `:272-298`（`index_batch` upsert，dim guard + `id_map` 映射）+ `:330-371`（`VectorSearcher::search` live KNN 读路径：`SearchPointsBuilder` 取 KNN + cosine 直读 + `id_map` 映回 chunk id）
- `core/src/retriever/vector/brute_force.rs:84-118`（`BruteForceVectorBackend::search` 精确 O(n) cosine：单位归一化 → dot → cosine 降序 + chunk_id 破并列确定性序——ground-truth top-k 来源）+ `:54-82`（`open` clear + `index_batch` append）
- `core/src/retriever/vector/types.rs:9-10`（`ChunkId(pub String)`）+ `:46-51`（`VectorChunk { chunk_id, embedding: Vec<f32>, metadata: Option<serde_json::Value> }`）+ `:53-60`（`VectorIndexConfig { dim, metric, persistence_path, collection_id }`）+ `:13-18`（`VectorMetric::Cosine`）+ `:37-43`（`VectorHit { chunk_id, score, metadata }`）
- `core/src/retriever/vector/traits.rs:28-35`（`VectorIndexer::open` / `index_batch`）+ `:48-56`（`VectorSearcher::search` / `is_indexed`）
- `core/tests/eval_integration.rs:110`（`{"recall@5": 0.7, "recall@10": 0.85}` 合成 fixture metrics——本 task 据实记其为 EvalService CRUD 预置值、**非**真实召回，本 task 的真实 recall@k 取代「仓库内仅有的召回数字是合成」之缺）
- `core/Cargo.toml`（`vector-qdrant = ["dep:qdrant-client"]` 自 task-18.4 已 optional——本 task 0 新 dep；`BruteForceVectorBackend` 默认可用，无 feature gate）
- `docs/decisions/adr-041-qdrant-live-vector-recall.md`（D1 live recall harness 方法学 / D2 真实测得召回数 / D3 CI service-container 集成 / D4 默认 0-vector-dep baseline；Status Proposed，ratify @ task-36.3）+ `docs/decisions/adr-034-production-vector-live-recall.md` D2（qdrant live-server 端到端 KNN，本 task + task-36.2 兑现其 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`）+ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造：无 server honest-defer skip 不 fail / 召回数真实跑出后回填不预填 / 确定性语料禁不可复现造数）+ `docs/specs/tasks/task-25.1-qdrant-server-lifecycle.md` §3 范围外（`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` 出处）

### 5.2 关键设计 — qdrant ANN recall@k vs BruteForce 精确 KNN harness + honest-defer + 确定性可复现语料（0 dep / 0 migration / 默认构建不变）

- **B1 方法学 model-free 可复现**：在**同一**确定性语料上量 qdrant HNSW ANN 相对 BruteForce 精确 KNN 的 `recall@k`——纯向量空间、**不依赖任何 embedding 模型**。BruteForce（`brute_force.rs:84-118`，cosine 降序 + chunk_id 破并列确定性序）给 ground-truth top-k；qdrant（`qdrant.rs:330-371` live KNN，cosine 直读）给 ANN top-k；`recall@k = mean(|qdrant_topk ∩ exact_topk| / k)` over M query。
  ```rust
  #![cfg(feature = "vector-qdrant")]
  // 确定性伪随机单位向量（std-only PRNG，无 rand crate / 无 clock；同 seed 必产同一向量）
  fn deterministic_unit_vector(seed: u64, dim: usize) -> Vec<f32> {
      let mut s = seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(1);
      let mut v: Vec<f32> = (0..dim).map(|_| {
          s ^= s >> 12; s ^= s << 25; s ^= s >> 27;          // xorshift64*（确定性）
          ((s.wrapping_mul(0x2545F4914F6CDD1D) >> 33) as f32 / (1u64 << 31) as f32) - 1.0
      }).collect();
      let n: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
      if n > 0.0 { for x in &mut v { *x /= n; } }
      v
  }
  ```
- **B2 honest-defer 守门（health() != Ready → skip 不 fail，ADR-013）**：harness 第一步 `health()`——`!= QdrantHealth::Ready` → `eprintln!` skip notice + `return`（测试**干净通过**）：
  ```rust
  let conn = QdrantConnConfig::from_env();
  let be = QdrantBackend::connect(&conn).expect("connect builds lazy client");
  if be.health() != QdrantHealth::Ready {
      eprintln!("qdrant_live_recall: no live qdrant at {} (health != Ready); skipping live recall — \
                 set QDRANT_URL to a running qdrant to measure real recall (honest-defer per ADR-013).", conn.url);
      return; // 干净 skip，不 fail —— 本地 / 无 service container 的 CI run 不变红
  }
  ```
  无 server 的本地 / 本 CI run（service container 由 task-36.2 加）走此分支干净 skip（**不** fail），证明 wiring 成立而不伪造召回；`Ready` 才进真实度量。
- **B3 同语料双索引 + recall@k**：N 个确定性 `VectorChunk` 经**同一**语料分别 `index_batch` 进 `QdrantBackend`（`open` ensure-create + upsert）与 `BruteForceVectorBackend`；M 个确定性 query 各取双方 top-k chunk_id 集合交集；`recall@k = mean(|qdrant_topk ∩ exact_topk| / k)`。collection 名 `phase36-live-recall`、dim D、metric Cosine（与 `search` cosine 直读 + BruteForce 一致），ensure-create dim/metric 匹配（不静默丢数据，task-25.1 决策）。
- **B4 floor 是 guard、真实值是报告（ADR-013）**：`assert!(recall_at_k >= FLOOR)` 作不退化 guard（如 k=10 floor 0.90——de-risk 已证 qdrant HNSW + 小语料 KNN 正确，floor 是保守下界）；`eprintln!` 真实测得值（`-- --nocapture` 可见）。真实数字在 task-36.2 CI service-container run 真实跑出后回填 §10 + v0.29.0 evidence——Draft 阶段 **待回填**，绝不预填、绝不伪造（ADR-013）。
- **B5 测试矩阵据实**：TEST-36.1.1 是 env-gated live test（有 server 量真实召回；无 server honest-defer 干净 skip 不 fail）；TEST-36.1.2 是**纯确定性语料生成器复现性**自测（**不连 server**，断言同 seed → 同向量 + 单位向量），保证语料可复现（ADR-013）——即使无 server 也跑、也绿，守住 harness 的可复现地基。

### 5.3 不变量

- **默认构建 0-vector-dep / 0-network 不变（ADR-004/008）**：`core/tests/qdrant_live_recall.rs` `#![cfg(feature = "vector-qdrant")]`——默认 `cargo test --workspace` **不编译** 此文件、不引入 `qdrant-client`、不连网；`vector-qdrant` opt-in，默认行为 / 默认构建 dep 集不变。
- **0 新代码依赖（ADR-008）**：`qdrant-client` 自 task-18.4 已 optional、`BruteForceVectorBackend` 默认可用、确定性 PRNG 是 std-only（无 `rand` crate）——本 task **0 新 Cargo 依赖**、无 `Cargo.lock` 变化。
- **0 schema migration**：纯新增 test 文件，无表 / 无持久化结构变更，不加列、不 `ALTER`、不新增编号 migration。
- **honest-defer 不伪造（ADR-013）**：`health()!=Ready` → eprintln skip + `return`（测试干净通过、**不** fail、**不**输出召回数、**不**当成功召回）；真实召回仅在 `health()==Ready` 的真实 server 上产生且经 §10 回填。
- **可复现性（ADR-013）**：语料 / query 向量确定性（同 seed 必产同一向量，无 `rand` / 无 clock）；TEST-36.1.2 断言之、**不连 server** 即可复现——召回度量在固定语料上可复现（同 server / 同 qdrant 版本下可比）。
- **不改 backend 本体**：`qdrant.rs` / `brute_force.rs` 三 trait 签名与 `connect`/`health`/`open`/`index_batch`/`search` 不动——harness 是消费方，复用既有 API（task-25.1 / task-18.4 / task-19.3 freeze）。
- **召回数真实跑出后回填**：真实 `recall@k` + run 环境（qdrant 版本 / service-container 形态）绝不预填——task-36.2 run 真实跑出后回填（**待回填**，ADR-013）。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [x] **AC1**（live recall harness env-gated + honest-defer skip 🔴 live / 🟢 wiring）: 新增 `core/tests/qdrant_live_recall.rs`（`#![cfg(feature = "vector-qdrant")]`），env-gated 经 `QdrantConnConfig::from_env()`；构造 `QdrantBackend::connect(...)` 后第一步 `health()`——`!= Ready`（无 server）→ eprintln skip notice + `return`（测试**干净通过不 fail**）；`Ready` 时同确定性语料同时索引进 `QdrantBackend`（`open` ensure-create + `index_batch` upsert）与 `BruteForceVectorBackend`（精确 ground truth），M 个确定性 query 取双方 top-k，`recall@k = mean(|qdrant_topk ∩ exact_topk| / k) >= FLOOR`（如 k=10 floor 0.90）并 eprintln 真实测得值（真实数字 **真实跑出后回填**，无 server 时 honest-defer skip，绝不预填，ADR-013）；**0 新 dep + 0 schema migration + 0 默认构建变更**（默认 0-vector-dep / 0-network） — verified by **TEST-36.1.1**（env-gated：有 live qdrant 时真实 `recall@k >= floor`；无 server honest-defer 干净 skip 不 fail）
- [x] **AC2**（确定性语料生成器复现性，无 server 🟢）: `deterministic_unit_vector(seed, dim)` 同 seed 必产同一向量（无 `rand` / 无 clock）且每个为单位向量——**不连 server** 即可复现性自测，守住 harness 可复现地基（ADR-013） — verified by **TEST-36.1.2**（无 server，断言同 seed → 同向量 + 单位向量）
- [x] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-36.1.3**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-36.1.1 | 🔴 live / 🟢 wiring：env-gated（`QdrantConnConfig::from_env`）live recall harness——`health()==Ready` 时同确定性语料双索引（qdrant ensure-create+upsert / BruteForce 精确），M query 取双方 top-k，`recall@k >= FLOOR`（如 k=10 floor 0.90）+ eprintln 真实值（真实数字 真实跑出后回填，不预填）；`health()!=Ready`（无 server）→ eprintln skip + return 干净通过（**不** fail，honest-defer ADR-013） | `core/tests/qdrant_live_recall.rs` | Done |
| TEST-36.1.2 | 🟢 确定性语料生成器复现性（**无 server**）：`deterministic_unit_vector(seed, dim)` 同 seed → 同向量（无 `rand` / 无 clock）+ 每个为单位向量——不连 server 即可跑、即绿，守 harness 可复现地基（ADR-013） | `core/tests/qdrant_live_recall.rs` | Done |
| TEST-36.1.3 | ADR-014 D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（高）真实 live KNN recall 需 live qdrant server，本 CI run 无**（承 phase-29 / task-25.1 R1）：`is_local()==false`，本 CI run 无在跑的 qdrant server（service container 由 task-36.2 加）。
  - **缓解**：harness 经 `health()` 守门——`!=Ready` → honest-defer eprintln skip + `return`（测试干净通过、**不** fail、不伪造召回，ADR-013）；真实召回经 task-36.2 service container（CI 每次 run）/ dev-box 跑出后回填 §10 + v0.29.0 evidence。AC1 的 live 维度 honest-defer 时如实记，不强 ratify、不预填（ADR-013/ADR-014）。stop-condition：harness 在无 server 时若 **fail（变红）而非干净 skip** 则 AC1 不标 `[x]`。
- **R2（中）确定性 PRNG 误用 clock / `rand` 致不可复现**：若误以 `std::time` / `rand` 作种，语料不可复现、recall 不可比（破 ADR-013）。
  - **缓解**：`deterministic_unit_vector(seed, dim)` 仅以 `seed`（chunk index）派生（std-only xorshift/splitmix，无 `rand` crate、无 clock）；TEST-36.1.2 断言同 seed → 同向量（**不连 server** 可跑）。stop-condition：TEST-36.1.2 显示同 seed 产不同向量则 AC2 不标 `[x]`。
- **R3（中）qdrant collection dim/metric 与语料 dim D·Cosine 不一致致 ensure-create error 或 KNN 不可比**：`open` `decide_ensure` 对 dim/metric 不匹配返回可识别 error（不静默重建）。
  - **缓解**：harness 用唯一 collection 名 `phase36-live-recall` + 显式 dim D / metric Cosine（与 `search` cosine 直读 + BruteForce 一致）；若复用既有不匹配 collection → ensure-create error 可识别（不静默丢数据），harness 如实报错而非伪造召回。stop-condition：dim/metric mismatch 致 KNN 不可比则不标 `[x]`。
- **R4（低）floor 设过高致 flaky 红 / 设过低致无 guard 价值**：ANN 召回随 N/D/k/HNSW 参数浮动，floor 设过高会 flaky、过低无意义。
  - **缓解**：floor 设为保守下界（如 k=10 floor 0.90，de-risk 已证小语料 KNN 正确）作不退化 guard；真实测得值经 eprintln 报告（floor 是地板、真实数才是结论，真实跑出后回填——绝不以 floor 充当真实值，ADR-013）。floor 调参 / per-k 多 floor 矩阵 `[SPEC-DEFER:phase-future.recall-floor-tuning-matrix]` 诚实延后。stop-condition：floor 误被当作「真实召回数」写入 evidence 则 review 退回。
- **R5（低）默认构建被 harness 污染**：harness 须不进默认构建（0-vector-dep / 0-network 不变）。
  - **缓解**：`#![cfg(feature = "vector-qdrant")]` 整文件 gate——默认 `cargo test --workspace` 不编译此文件、不引 `qdrant-client`、不连网；0 新 dep（`qdrant-client` 自 task-18.4 optional）。stop-condition：默认构建编译此文件 / 引入 vector dep 则不标 `[x]`。

## 9. Verification Plan

```bash
# 0. 默认构建（无 vector-qdrant feature）：harness 不编译、0 新 vector dep、不退化（AC1 默认维度 + R5）
cargo test --workspace
cargo build --workspace

# 1. AC2 — 确定性语料生成器复现性（无 server 即可跑、即绿；feature 开但不依赖 server）
cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture
echo "exit=$?  # 期望 0：TEST-36.1.2 复现性 PASS；TEST-36.1.1 无 server → honest-defer 干净 skip 不 fail"

# 2. AC1 — 真实 live qdrant 端到端 recall@k（task-36.2 service container / dev-box，本地需先起 qdrant）
#    起单节点 qdrant（本地 docker：docker run -p 6333:6333 -p 6334:6334 qdrant/qdrant），再指向 QDRANT_URL：
#    QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core \
#      --features vector-qdrant --test qdrant_live_recall -- --nocapture
#    → health()==Ready → 同语料双索引 → recall@k >= floor + eprintln 真实测得值
#    真实数字 task-36.2 CI service-container run 真实跑出后回填 §10 + v0.29.0 evidence（绝不预填，ADR-013）

# 3. clippy（feature 开 + 默认）
cargo clippy -p contextforge-core --features vector-qdrant --tests -- -D warnings
cargo clippy --workspace --all-targets -- -D warnings

# 4. AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.qdrant-live-recall-harness-defer-note]：本 task 仅交付 `core/tests/qdrant_live_recall.rs`（`#![cfg(feature = "vector-qdrant")]`，env-gated `QDRANT_URL`）的 qdrant ANN recall@k vs BruteForce 精确 KNN 方法学 harness——确定性可复现语料 + honest-defer（`health()!=Ready` → eprintln skip + return 干净通过不 fail）。真实召回数仅在 `health()==Ready` 的真实 server 上产生（task-36.2 CI service container / dev-box），**真实跑出后回填** §10 + v0.29.0 evidence（绝不预填、绝不伪造，ADR-013）。`qdrant-recall` CI service-container job（每次 CI run 跑 harness、永久关闭 CI-no-server defer）→ [SPEC-OWNER:task-36.2-qdrant-recall-ci-service]；语义 golden 召回（需真实模型）→ [SPEC-DEFER:phase-future.qdrant-semantic-golden-recall]（model-free 的 qdrant-vs-精确-KNN 是 clean primary，ADR-041 A3）；集群 / 复制 / 部署拓扑 → [SPEC-DEFER:phase-future.qdrant-deployment-topology]；多 backend live 召回矩阵 → [SPEC-DEFER:phase-future.multi-backend-production]；floor 调参矩阵 → [SPEC-DEFER:phase-future.recall-floor-tuning-matrix]。floor 是地板 guard、真实测得值才是结论（绝不以 floor 充真实值，ADR-013）。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done

**§9 Verification 实证**（real evidence，本地全绿 + 真实 live 召回）：
- **AC1 真实 live 召回**：`QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture`（对真实 `qdrant/qdrant` docker 容器）→ `test_36_1_1_qdrant_live_recall_at_k` PASS，实测 `PHASE36 qdrant LIVE recall@10 vs brute-force exact KNN | N=2000 dim=64 M=50 => recall@10=1.0000`。honest-defer 分支：`env -u QDRANT_URL cargo test ...` → `test_36_1_1` 干净 SKIP（eprintln "QDRANT_URL unset (honest-defer)" + return，**不 fail**）、`test_36_1_2` PASS（2 passed / 0 failed）。
- **AC2 复现性**：`test_36_1_2_deterministic_corpus_reproducible` PASS（同 seed → byte-identical 向量 / 异 seed → 不同 / 单位长 norm≈1.0），**无 server 也跑**。
- 默认构建不退化：`cargo test -p contextforge-core --lib` 212 passed / 0 failed（harness `#![cfg(feature = "vector-qdrant")]` 不进默认构建）；`cargo clippy -p contextforge-core --features vector-qdrant --tests -- -D warnings` 0 warning。
- AC3：D2 lint `--touched origin/master`（CI spec-lint 权威）。

**诚实判读（ADR-013，关键）**：实测 recall@10=**1.0000**——这是「qdrant LIVE KNN 与 brute-force 精确 ground truth 逐一吻合」的真实 live-server 召回数（取代合成 fixture `eval_integration.rs` 0.7/0.85），真实关闭 ADR-034 D2「真实 live recall 数从未测过」。**为何 1.0**：qdrant 在 upsert 后即时对低于其 HNSW `indexing_threshold`（默认 ~10000）/ 未经后台 optimizer 建图的段服务**精确** KNN，故 N=2000 下 qdrant 返回值即等于精确 top-k → recall 1.0。这是 live-server KNN **正确性**的真实证明；压测 HNSW **近似域**（语料 > indexing_threshold 且 optimizer 建图后）的真实 ANN recall（预期 <1.0）需大语料 + optimizer-wait、timing 敏感 → honest-defer `[SPEC-DEFER:phase-future.vector-large-corpus-perf]`（不夸大为「已压测 HNSW 近似」，ADR-013）。floor=0.90 为不退化 guard（若 qdrant 召回错误/漏检则 <0.90 红），真实测得 1.0000 留足余量。

**grounding 校正（实施期，ADR-013）**：
- generator 函数名实为 `det_unit_vec(seed, dim)`（spec §7 草拟名 `deterministic_unit_vector`）+ splitmix64 派生（无 `rand`/无 clock）；collection 名实为 `phase36_live_recall`（下划线，spec 连字符）——均机械命名差异，行为同 spec。
- 真实 CI live run（每次 CI run 对 service container 跑、永久关闭 CI-no-server defer）由 **task-36.2** `qdrant-recall` job 兑现 `[SPEC-OWNER:task-36.2-qdrant-recall-ci-service]`；本 task 的 live 证据 = 本地对真实 qdrant 容器跑出的 recall@10=1.0000（真实非预填）。

**实际改动文件**：
- 新增 `core/tests/qdrant_live_recall.rs`（`#![cfg(feature = "vector-qdrant")]`，env-gated `QDRANT_URL` + `health()!=Ready` honest-defer skip；splitmix64 确定性单位向量 N=2000 dim=64；双索引 `QdrantBackend`（ensure-create+index_batch）vs `BruteForceVectorBackend` 精确 ground truth；M=50 query recall@k=mean(|∩|/k) 断言 ≥ 0.90 + eprintln 实测值；TEST-36.1.1 live + TEST-36.1.2 复现性）。
- 0 backend 改动 / 0 新 dep / 0 schema migration / 0 network / 0 默认构建变更（默认 0-vector-dep，ADR-004/008）。ADR-041 D1 ratify 依据（@ task-36.3 closeout）。
