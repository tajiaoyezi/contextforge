# Task `29.2`: `qdrant-live-knn-and-recall-harness — phase29 真实端到端召回 harness（clone phase20_recall_via_retriever，BruteForce→QdrantBackend::connect(QdrantConnConfig::from_env())，feature vector-qdrant + embedding-fastembed gated），首次真实兑现 [SPEC-DEFER:phase-future.qdrant-server-lifecycle]：connect→ensure-create→upsert→KNN over live qdrant；无 server 时 health()==Unreachable honest-defer（eprintln + exit 0，ADR-013 禁伪造）+ 单节点部署基线文档化（集群/复制延后）`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 29 (live-vector-recall)
**Dependencies**: task-25.1（`QdrantBackend` 生命周期契约层已落地：`QdrantConnConfig::from_env` / `connect` / `health()` / `decide_ensure` + ensure-create `open`，真实 live KNN 标 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`）/ task-29.1（`select_vector_backend` 工厂 + server.rs 热路径注入；本 task 经 harness 直接 `QdrantBackend::connect` 与工厂并行验证 backend 真实 KNN）/ task-18.4（`QdrantBackend` via `qdrant-client` 1.18 + `vector-qdrant` feature 起源；`open`/`index_batch`/`search` 本体）/ `core/examples/phase20_recall_via_retriever.rs`（task-20.2 production-pipeline 召回 harness，本 task clone 为 phase29 harness）/ ADR-034 D2（qdrant live-server 端到端 KNN + honest-defer）/ ADR-030 D1（qdrant 生命周期）/ ADR-023 D3（qdrant hosted/scale-out tier）/ ADR-013（禁伪造 live-server 凭据 / 召回数值）/ ADR-014 D1-D5（第二十次激活）

## 1. Background

Phase 25 task-25.1 把 `core/src/retriever/vector/qdrant.rs` 的 `QdrantBackend` 推到**可生产化的契约层**：`QdrantConnConfig::from_env`（`qdrant.rs:72-78` 读 `QDRANT_URL` 既有 + 可选 `QDRANT_API_KEY` + url scheme 推断 TLS）+ `validate`（纯函数校验，不连 server）+ `connect`（`qdrant.rs:162-181`，懒连接 client + owned current-thread tokio runtime）+ `health()`（`qdrant.rs:184-189`，live→`QdrantHealth::Ready`，无 server→`QdrantHealth::Unreachable`，不 panic、不静默成功）+ `decide_ensure`（`qdrant.rs:152-158` 纯函数）+ ensure-create `open`（`qdrant.rs:215-270`，reuse/create/error 替代 spike 无脑 drop+create）。这些都是 **deterministic 契约**——`vector-qdrant` feature 下不连 live server 即可单测（TEST-25.1.1~25.1.4 已 Done）。

但 task-25.1 §3 范围外明记：**真实连一个 live qdrant server 跑 connect→ensure-create→upsert→KNN 的端到端集成** `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`——CI 无在跑的 qdrant server，`is_local()==false`（外部 server 进程），spike 真实数据曾在 WSL2 用手动 musl 二进制取得。qdrant 的 live KNN 读路径已在 `qdrant.rs:330-371`（`VectorSearcher::search`：`SearchPointsBuilder` 取 KNN，cosine 直读，`id_map` 把 qdrant 数字 point id 映回 chunk id）实现，但从未在真实 live server 上端到端跑过——契约成立、live 召回未兑现。

本 task 是该 deferral 的**首次真实兑现**：clone task-20.2 的 production-pipeline 召回 harness（`core/examples/phase20_recall_via_retriever.rs`，经真实 scanner+chunker+FastEmbed 走 `Retriever::search_semantic` 热路径量召回），把其 0-dep `BruteForceVectorBackend` 换成 `QdrantBackend::connect(QdrantConnConfig::from_env())`，feature `vector-qdrant` + `embedding-fastembed` gated，对一个真实 qdrant server 跑 connect→ensure-create→upsert→KNN 并量真实召回。CI 无 server，故 harness 在 `health()==Unreachable` 时 **honest-defer**（eprintln 说明 + `exit 0`，不伪造召回、不伪造 live-server 通过，ADR-013）。

## 2. Goal

新增 `core/examples/phase29_recall_via_qdrant.rs`（clone 自 `phase20_recall_via_retriever.rs`）：feature `vector-qdrant` + `embedding-fastembed` 同时开时，经真实 production pipeline 索引 6 golden + 5 distractor 语料 → `QdrantBackend::connect(QdrantConnConfig::from_env())` 构造 backend → **先 `health()` 探活**：若 `Unreachable` 则 eprintln 说明 + `exit 0`（honest-defer，不伪造）；若 `Ready` 则 `open`（ensure-create）→ `index_chunks_semantic`（经 backend `index_batch` upsert）→ 30 golden query 各 `search_semantic` top-10 → 量真实 file-level SemanticRecall@5/@10 + top-1 + MRR over **live qdrant KNN**。任一 feature 缺省时 harness 编译为 no-op（eprintln 说明，default `cargo build --workspace` / `cargo test --workspace` 不受影响）。

单节点部署基线（手动起 server / `QDRANT_URL` 指向 / dim-metric 一致）在本 task spec + harness 注释文档化；集群 / 复制 / 部署拓扑 `[SPEC-DEFER:phase-future.qdrant-deployment-topology]` 诚实延后。

pass bar：(1) 真实 live qdrant server 上（手动 / dev-box）connect→ensure-create→upsert→KNN 跑通，真实召回数 **真实跑出后回填** 到 §10 + v0.22.0 evidence（绝不预填，ADR-013）；无 server 时 honest-defer 干净退出 exit 0。(2) harness 在 feature 开但无 server 时 deterministic 编译 + honest-defer（证明 wiring 不伪造召回）。(3) 单节点部署基线文档化、集群/复制诚实延后。(4) D2 lint 触及行 0 未标注命中。默认构建 0 新 vector dep、行为不变（ADR-004 / ADR-023 D5）。

## 3. Scope

### In Scope（计划交付）

- **新增 `core/examples/phase29_recall_via_qdrant.rs`**（clone 自 `core/examples/phase20_recall_via_retriever.rs`，结构镜像：6 golden 类别 + 5 distractor 写入临时源树 → `IndexSession` 真实 scanner+chunker 索引 → `Retriever` + `FastEmbedProvider` + backend → `enumerate_chunks` + `index_chunks_semantic` → 30 query `search_semantic` 量召回）。差异：把 `BruteForceVectorBackend::new()` 换成 `QdrantBackend::connect(&QdrantConnConfig::from_env())?`；`#[cfg(all(feature = "vector-qdrant", feature = "embedding-fastembed"))]` 双 gate；构造 backend 后 **先 `backend.health()`**——`QdrantHealth::Unreachable` → eprintln（说明需 live qdrant + `QDRANT_URL`）+ `return Ok(())`（exit 0，honest-defer）；`QdrantHealth::Ready` → 继续 ensure-create + upsert + KNN + 量召回。任一 feature 缺省 → no-op `main`（eprintln 说明）。
- **harness 注释 + 本 spec §5.2 文档化单节点部署基线**：手动起单节点 qdrant server（spike musl 二进制 / 本地 Docker / dev-box）；`QDRANT_URL` env 指向（默认 `http://localhost:6334`）；collection dim 须与 FastEmbed all-MiniLM-L6-v2 dim 384 一致（ensure-create 决策保 dim/metric 匹配，不静默丢数据）。
- **不改 `core/src/retriever/vector/qdrant.rs` 本体**：harness 复用既有 `connect`/`health`/`open`/`index_batch`/`search`（`qdrant.rs:330-371` live KNN 读路径），不重写 backend。
- **§10 + v0.22.0 evidence 真实召回回填位**：真实跑出 SemanticRecall@5/@10 + top-1 + MRR + run 环境（qdrant 版本 / server 部署形态）**真实跑出后回填**（绝不预填）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **CI 内常驻 qdrant server / CI 自动跑 live KNN** `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`：CI 无在跑的 qdrant server；本 task harness 在 CI（无 server）honest-defer exit 0，真实 live KNN 在手动 / dev-box 跑，禁伪造 live-server 通过（ADR-013）。
- **qdrant 集群 / 复制 / 部署拓扑 / auto-start / shutdown 编排** `[SPEC-DEFER:phase-future.qdrant-deployment-topology]`：本 task 仅文档化单节点基线，hosted 运维硬化（多节点 / 复制因子 / sharding）承 `docs/spikes/phase-18-qdrant.md` Follow-up，诚实延后。
- **`QdrantBackend` 的 `index_batch` / `search` 本体重写** [SPEC-OWNER:task-18.4-spike-qdrant]：本 task 经 harness 真实驱动既有 KNN 读路径（`qdrant.rs:330-371`），不重写 upsert/KNN。
- **`select_vector_backend` 工厂 + server.rs 热路径注入** [SPEC-OWNER:task-29.1]：本 task 经 harness 直接 `QdrantBackend::connect` 验证真实 KNN 召回，工厂化注入由 task-29.1 落地（二者并行：harness 量召回、工厂量热路径接线）。
- **lancedb 真实 ANN 索引调参 + 多 backend 选择矩阵实测** [SPEC-OWNER:task-29.3]：本 task 仅做 qdrant live KNN harness。
- **v0.22.0 closeout / smoke / ADR ratify / Amendment** [SPEC-OWNER:task-29.4]：本 task 交付 qdrant live harness + 真实召回证据，收口在 29.4。

## 4. Actors

- **主 agent**：实施 + harness 主理 + 在有 server 的 dev-box / 手动环境跑真实召回回填证据（ADR-012 自治）。
- **`core/examples/phase29_recall_via_qdrant.rs`**（新增 harness）：clone 自 phase20，经 production pipeline 量 live qdrant KNN 召回 + honest-defer。
- **`core/examples/phase20_recall_via_retriever.rs`**（clone 源）：task-20.2 production-pipeline 召回 harness（real scanner+chunker+FastEmbed→`Retriever::search_semantic`）。
- **`core/src/retriever/vector/qdrant.rs::QdrantBackend`**：task-25.1 生命周期层 + task-18.4 KNN 本体；本 task 经 harness 真实驱动 `connect`/`health`/`open`/`index_batch`/`search`。
- **`QdrantConnConfig::from_env`**（`qdrant.rs:72-78`）：连接来源（`QDRANT_URL` + 可选 `QDRANT_API_KEY` + TLS scheme 推断）。
- **live qdrant server**：外部 server 进程（`is_local()==false`）；本 task 真实 KNN 依赖其在手动 / dev-box 存在——CI 无 server → honest-defer，不伪造。
- **下游 task-29.3 / task-29.4**：29.3 把 qdrant 纳入多 backend 选择矩阵实测；29.4 closeout 据本 task 真实召回（或 honest-defer 维度）ratify ADR-034 D2。

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/retriever/vector/qdrant.rs:152-158`（`decide_ensure` 纯函数 reuse/create/error）+ `:162-181`（`connect`：懒连接 client + tokio runtime）+ `:184-189`（`health()`：live→Ready / 无 server→Unreachable，不 panic）+ `:215-270`（ensure-create `open`，SPEC-DEFER anchor `:219`）+ `:330-371`（`VectorSearcher::search` live KNN 读路径：`SearchPointsBuilder` + cosine 直读 + `id_map` 映回 chunk id）+ `:72-78`（`QdrantConnConfig::from_env`）+ `:393-462`（既有 feature-gated 契约单测）
- `core/examples/phase20_recall_via_retriever.rs`（clone 源：6 golden 类别 + 5 distractor + `IndexSession` 真实索引 + `Retriever`/`FastEmbedProvider`/`with_vector_searcher` + `enumerate_chunks` + `index_chunks_semantic` + 30 query `search_semantic` 量 recall@5/@10/top-1/MRR + no-op default-build main）
- `core/src/retriever/vector/traits.rs:38-46`（`VectorSearcher::search` live-KNN surface）+ `:11-25`（`VectorBackend` / `VectorIndexer`）
- `core/src/retriever/mod.rs:592-595`（`with_vector_searcher`）+ `:628-665`（`index_chunks_semantic`，`persistence_path:None` `:656`）+ `:684-713`（`search_semantic_raw`）
- `core/Cargo.toml:119`（`vector-qdrant = ["dep:qdrant-client"]`）+ `:123`（`embedding-fastembed = ["dep:fastembed"]`）
- `docs/decisions/adr-034-production-vector-live-recall.md` D2（qdrant live-server 端到端 KNN + honest-defer）+ `docs/decisions/adr-023-vector-backend-default.md` D3（qdrant hosted/scale-out tier）+ `docs/decisions/adr-030-production-vector-backend.md` D1（qdrant 生命周期）+ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造 live-server 凭据 / 召回数值）
- `docs/spikes/phase-18-qdrant.md`（external server `is_local()==false` + 真实数据经手动 musl 二进制 + server-lifecycle / deployment-topology Follow-up）+ `docs/specs/tasks/task-25.1-qdrant-server-lifecycle.md` §3 范围外（`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` 出处）

### 5.2 关键设计 — live qdrant KNN harness + honest-defer + 单节点部署基线

- **harness 经 production pipeline 量真实召回**：clone phase20 的结构（real scanner+chunker→`IndexSession`→`Retriever`+`FastEmbedProvider`(all-MiniLM-L6-v2, dim 384)→`enumerate_chunks`+`index_chunks_semantic`→30 query `search_semantic` top-10→file-level recall@5/@10/top-1/MRR），唯一 backend 差异：`with_vector_searcher(Arc::new(QdrantBackend::connect(&QdrantConnConfig::from_env())?))`。这样真实召回经 `core/src/server.rs` 同款热路径（`Retriever::search_semantic` → backend `search`，`qdrant.rs:330-371` live KNN），不是合成数。
- **honest-defer 守门（ADR-013 核心）**：构造 backend 后**第一步** `match backend.health()`——`QdrantHealth::Unreachable` → `eprintln!("phase29_recall_via_qdrant: no live qdrant at QDRANT_URL (health=Unreachable); honest-defer per ADR-013 — set QDRANT_URL to a running single-node qdrant to measure real recall.")` + `return Ok(())`（exit 0）。**不**伪造召回、**不**把 honest-defer 当成功召回、**不**预填任何召回数。`QdrantHealth::Ready` 才进 ensure-create → upsert → KNN → 量召回。CI（无 server）走 Unreachable 分支干净退出，证明 wiring 成立而不伪造（区别于真实 server 上的 Ready 分支）。
- **ensure-create dim/metric 一致**：collection dim 须 = FastEmbed dim 384、metric = Cosine（与 `search` cosine 直读一致）。`open` 经 `decide_ensure`（`qdrant.rs:152-158`）保 reuse（既有匹配）/ create（不存在）/ error（dim/metric 不匹配，可识别不静默丢数据）。harness 用唯一 collection 名（如 `phase29-recall-via-qdrant`）避免与既有数据冲突。
- **单节点部署基线（文档化）**：手动起一个单节点 qdrant server——spike 的 `qdrant-x86_64-unknown-linux-musl` 静态二进制（无 Docker，WSL2）/ 本地 `qdrant/qdrant` 容器 / dev-box；gRPC 默认端口 6334；`QDRANT_URL` env 指向（`from_env` 默认 `http://localhost:6334`，可选 `QDRANT_API_KEY`）。集群 / 复制因子 / sharding / 部署拓扑 `[SPEC-DEFER:phase-future.qdrant-deployment-topology]` 诚实延后——本 task 只兑现单节点 live KNN 召回基线。
- **ADR-013 真实召回回填**：真实 SemanticRecall@5/@10 + top-1 + MRR + qdrant 版本 + server 部署形态在真实跑通后回填 §10 + v0.22.0 evidence；CI / 无 server 维度如实标 honest-defer，不强造数（待实测回填）。

### 5.3 不变量

- 默认构建（无 `vector-qdrant` / 无 `embedding-fastembed`）harness 编译为 no-op、0 新 vector dep、`cargo build --workspace` / `cargo test --workspace` 行为不变（ADR-023 D5 / ADR-004）。
- honest-defer 不伪造：`health()==Unreachable` → eprintln + exit 0，绝不输出召回数 / 绝不当成功召回（ADR-013）。真实召回仅在 `health()==Ready` 的真实 server 上产生且经 §10 回填。
- harness 经 **production hot path**（`Retriever::search_semantic` → backend `search`）量召回，不旁路、不合成——与 `core/src/server.rs` 同款检索接线。
- 不改 `qdrant.rs` 本体三 trait 签名 / `connect`/`health`/`open`/`search`——harness 是消费方，复用既有 API（task-25.1 / task-18.4 freeze）。
- ensure-create 不静默丢数据：dim/metric 不匹配 → 可识别 error（沿用 task-25.1 决策）。
- 召回数值 / run 环境绝不预填——真实跑出后回填（待实测回填，ADR-013）。

## 6. Acceptance Criteria

- [ ] AC1（🔴 live qdrant 端到端 KNN，honest-defer）: feature `vector-qdrant` + `embedding-fastembed` 下，`phase29_recall_via_qdrant` 对一个真实 live qdrant server（手动 / dev-box）跑 connect→ensure-create→upsert→KNN，经 `Retriever::search_semantic` 热路径量真实 file-level SemanticRecall@5/@10 + top-1 + MRR over live qdrant KNN（`qdrant.rs:330-371`）；无 server 时 `health()==Unreachable` → eprintln + exit 0（honest-defer，绝不伪造召回 / live-server 通过，ADR-013）。真实召回数 **真实跑出后回填** §10 + v0.22.0 evidence，绝不预填 — verified by TEST-29.2.1
- [ ] AC2（🟢 wiring deterministic）: harness 在 feature 开但无 server 时 deterministic 编译 + honest-defer 干净退出（exit 0，eprintln 说明），证明 connect/health/ensure-create/upsert/KNN 接线成立而不伪造召回；默认构建（feature 缺省）harness no-op、`cargo build --workspace` / `cargo test --workspace` 不退化、0 新 vector dep — verified by TEST-29.2.2
- [ ] AC3（🟢 doc / 🔴 real）: qdrant 单节点部署基线（手动起 server / `QDRANT_URL` 指向 / dim-metric 384·Cosine 一致）在本 spec §5.2 + harness 注释文档化；集群 / 复制 / 部署拓扑诚实延后 [SPEC-DEFER:phase-future.qdrant-deployment-topology]（真实多节点拓扑 待实测回填，不伪造） — verified by TEST-29.2.3
- [ ] AC4（ADR-014 D2 lint）: bash scripts/spec_drift_lint.sh --touched origin/master PR 触及行 0 未标注命中 — verified by TEST-29.2.4

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-29.2.1 | 🔴 live qdrant connect→ensure-create→upsert→KNN over real server，经 `search_semantic` 热路径量真实 recall@5/@10/top-1/MRR；无 server honest-defer exit 0（真实召回数 真实跑出后回填，不预填） | `core/examples/phase29_recall_via_qdrant.rs` + §10 / v0.22.0 evidence | Planned |
| TEST-29.2.2 | 🟢 harness feature 开无 server deterministic 编译 + honest-defer 干净退出（exit 0）；默认构建 no-op + `cargo build/test --workspace` 不退化 + 0 新 vector dep | `core/examples/phase29_recall_via_qdrant.rs` + 全 workspace | Planned |
| TEST-29.2.3 | 🟢 doc / 🔴 real：单节点部署基线文档化（spec §5.2 + harness 注释）；集群/复制/拓扑诚实延后 `[SPEC-DEFER:phase-future.qdrant-deployment-topology]` | 本 spec + `core/examples/phase29_recall_via_qdrant.rs` 注释 | Planned |
| TEST-29.2.4 | ADR-014 D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威） | `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（高）真实 live KNN 需 live qdrant server，CI 无**（承 phase-29 §7 + task-25.1 R1）：`is_local()==false`，CI 无在跑的 qdrant server。
  - **缓解**：harness 经 `health()` 守门——无 server → honest-defer eprintln + exit 0（不伪造召回 / live-server 通过，ADR-013）；真实召回在手动 / dev-box（单节点基线）跑出后回填 §10 + v0.22.0 evidence。AC1 的 live 维度 honest-defer 时如实记，不强 ratify、不预填（ADR-014 / ADR-013）。AC2 在 feature 开无 server 时 deterministic 证明 wiring 成立。
- **R2（中）qdrant collection dim/metric 与 FastEmbed dim 384·Cosine 不一致致 ensure-create error 或 KNN 不可比**：`open` `decide_ensure` 对 dim/metric 不匹配返回可识别 error（不静默重建）。
  - **缓解**：harness 用唯一 collection 名 + 显式 dim 384 / metric Cosine（与 `search` cosine 直读一致）；若复用既有不匹配 collection → ensure-create error 可识别（不静默丢数据），harness 如实报错而非伪造召回。
- **R3（中）`embedding-fastembed` 首跑下载 ONNX 模型 / 离线环境无缓存**：FastEmbed all-MiniLM-L6-v2 首跑下载模型。
  - **缓解**：harness 注释记首跑下载（沿用 phase20 注释）；离线从 `.fastembed_cache` 服务（cwd 含缓存目录）；无模型 / 无网时 harness 因 embedder 构造失败如实报错，不伪造召回（ADR-013）。
- **R4（低）harness 引入新 crate 面**：default build 须 0 新 vector dep。
  - **缓解**：harness 仅复用既有 optional `qdrant-client`（`vector-qdrant`）+ `fastembed`（`embedding-fastembed`），双 `#[cfg(all(...))]` gate；默认构建 no-op、0 新 dep、无 Cargo.lock 变化（ADR-023 D5 / ADR-004）。

## 9. Verification Plan

```bash
# 0. 默认构建（无 vector / 无 fastembed feature）：harness no-op、0 新 vector dep、不退化（AC2 default 维度）
cargo build --workspace
cargo test --workspace

# 1. AC2 — feature 开但无 server：harness deterministic 编译 + honest-defer 干净退出 exit 0（不伪造召回）
#    （CI / 无 server 环境天然走此分支；断言 exit 0 + eprintln 说明 health=Unreachable）
cargo run -p contextforge-core --example phase29_recall_via_qdrant \
  --features vector-qdrant,embedding-fastembed
echo "exit=$?  # 期望 0（honest-defer，无召回数输出）"

# 2. AC1 — 真实 live qdrant 端到端 KNN（手动 / dev-box，单节点基线；CI 不跑）
#    先起单节点 qdrant（spike musl 二进制 / 本地容器），再指向 QDRANT_URL：
#    QDRANT_URL=http://localhost:6334 cargo run -p contextforge-core \
#      --example phase29_recall_via_qdrant --features vector-qdrant,embedding-fastembed
#    → connect→ensure-create→upsert→KNN over live qdrant；真实 recall@5/@10/top-1/MRR
#    真实跑出后回填 §10 + docs/releases/v0.22.0-evidence.md（绝不预填，ADR-013）

# 3. AC3 — 单节点部署基线文档化 + 集群/复制诚实延后（spec §5.2 + harness 注释复核）

# 4. AC4 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **outward-facing 红线**：CI 无常驻 qdrant server——真实 live KNN 召回在手动 / dev-box 的单节点 qdrant 上跑（用户 / 主 agent 环境），CI（无 server）走 `health()==Unreachable` honest-defer exit 0，绝不伪造 live-server 通过或预填召回数（ADR-013 / ADR-014）。真实召回 + run 环境真实跑出后回填 §10 + v0.22.0 evidence。

## 10. Completion Notes (s2v 6 项标准)

- **Status**: Draft（待实施）
- **计划改动文件**：
  - `core/examples/phase29_recall_via_qdrant.rs`（新增）— clone 自 `core/examples/phase20_recall_via_retriever.rs`；`#[cfg(all(feature = "vector-qdrant", feature = "embedding-fastembed"))]` 双 gate；`BruteForceVectorBackend::new()` → `QdrantBackend::connect(&QdrantConnConfig::from_env())?`；构造后先 `backend.health()` honest-defer（Unreachable→eprintln+exit 0）；Ready→ensure-create+upsert+KNN+量召回；feature 缺省 no-op main + 单节点部署基线注释
  - 本 spec §10 + `docs/releases/v0.22.0-evidence.md`（task-29.4 新增）— 真实召回 + run 环境回填位（真实跑出后回填）
- **§9 Verification 计划** (will record real evidence at impl)：
  - AC2 default：`cargo build --workspace` + `cargo test --workspace`（harness no-op、0 新 vector dep、不退化）— 待实测回填
  - AC2 feature 无 server：`cargo run --example phase29_recall_via_qdrant --features vector-qdrant,embedding-fastembed` honest-defer exit 0（无召回输出，证明 wiring 不伪造）— 待实测回填
  - AC1 live：单节点 qdrant 上 `QDRANT_URL=... cargo run --example phase29_recall_via_qdrant --features vector-qdrant,embedding-fastembed` → connect→ensure-create→upsert→KNN 真实 recall@5/@10/top-1/MRR + qdrant 版本 + server 部署形态 — 真实跑出后回填（CI 无 server → honest-defer 维度如实记，不预填、不伪造，ADR-013）
  - AC3：单节点部署基线文档化复核（spec §5.2 + harness 注释）；集群/复制/拓扑 `[SPEC-DEFER:phase-future.qdrant-deployment-topology]` 诚实延后 — 待实测回填
  - AC4 lint：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威）— 待实测回填
- **剩余风险 / 下游影响（计划）**：真实 live KNN 召回受「有无 live qdrant server」限——CI honest-defer，真实数手动 / dev-box 回填；集群/复制/部署拓扑 `[SPEC-DEFER:phase-future.qdrant-deployment-topology]` 诚实延后；下游 task-29.3（多 backend 选择矩阵实测纳入 qdrant 档）+ task-29.4（closeout 据本 task 真实召回 / honest-defer 维度 ratify ADR-034 D2）。
