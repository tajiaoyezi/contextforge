# ADR `030`: `production-vector-backend`

**Status**: Proposed (2026-05-31)
**Category**: 数据平面 / 向量检索 / 生产规模 backend 生命周期 + 可构建性
**Date**: 2026-05-31
**Decided By**: 主 agent (ADR-012 自治)；tajiaoyezi ratification at v0.18.0 closeout
**Related**: ADR-023 (vector-backend-default — D3 qdrant「hosted/scale-out」/ D4 lancedb「embedded-columnar alternative」/ D5 默认 0-vector-dep / Follow-ups) / ADR-028 (vector-persistence-strategy — hnsw 持久化 + sqlite-vec 跨平台) / ADR-002 (sqlite+tantivy persistence) / ADR-008 (core-library-selection — 依赖选型 add-only) / ADR-004 (local-first-privacy-baseline — 默认 0 dep / 0 network，远程/重构件 opt-in) / ADR-014 (D1-D5，第十六次激活) / ADR-013 (禁伪造凭据 — real-data-only honest defer) / Phase 18 (vector-backend-selection — task-18.4 qdrant spike / task-18.5 lancedb spike) / Phase 25 (production-vector-backend)

## Context

ADR-023（vector-backend-default）以真实 5 维证据把四个向量 backend 分层：sqlite-vec（D1 嵌入式推荐默认）/ hnsw（D2 跨平台 fallback）/ **qdrant（D3 hosted/scale-out）** / **lancedb（D4 embedded-columnar alternative）**。Phase 18 把全部四个落为 feature-gated spike backend（task-18.4 qdrant / task-18.5 lancedb），Phase 23（ADR-028）已把 sqlite-vec / hnsw 两个嵌入式/fallback 档推到生产可用（hnsw 图持久化 + sqlite-vec Windows MSVC 真实构建）。生产规模 ANN 的两档——qdrant（外部 gRPC server）与 lancedb（嵌入式列存，构建需 protoc）——仍停在 task-18.4/18.5 的 spike 态：

1. **qdrant 生命周期未成型**：`core/src/retriever/vector/qdrant.rs::QdrantBackend`（`vector-qdrant` feature；`qdrant-client` 1.18 optional dep 已在 `core/Cargo.toml`）用 `Qdrant::from_url(QDRANT_URL || http://localhost:6334)` + 一个 current-thread tokio runtime `block_on` 桥接同步 trait。`open` 直接 `delete_collection` + `create_collection`（写死 `Distance::Cosine`），无 connect 探活 / health-probe / collection ensure-create（存在则复用）/ 连接配置（timeout / api-key / TLS）等生产生命周期层。`is_local() == false`——qdrant 是外部 server 进程，CI 无在跑的 qdrant server（`docs/spikes/phase-18-qdrant.md` 用 WSL2 上手动起的 musl 静态二进制取真实数据），故真实 KNN 集成天然受限于「有无 live server」。

2. **lancedb 可构建性 + 索引调参未定**：`core/src/retriever/vector/lance_db.rs::LanceDbBackend`（`vector-lancedb` feature；`lancedb` 0.30 + `arrow-array` 58 + `futures` 0.3 optional deps 已在 `core/Cargo.toml`）用 `lancedb::connect(LANCEDB_DIR)` + `create_empty_table` + `nearest_to().distance_type(Cosine)`。`docs/spikes/phase-18-lancedb.md` 记录其构建**需 `protoc`**（lance `build.rs`）+ Lance/DataFusion/Arrow 首次构建约 5 分钟，且 n=5000 走 flat scan（未建 ANN 索引）。protoc 前置在某些平台（如 Windows MSVC dev box，仿 sqlite-vec 当年的 MSVC 受阻）可能成为构建 blocker；ANN 索引（IVF_PQ / HNSW）调参 + 数据集 compaction 是 `docs/spikes/phase-18-lancedb.md` 显式列的 Follow-up（`[SPEC-DEFER:phase-future.lancedb-index-tuning]` / `[SPEC-DEFER:phase-future.lancedb-schema-compaction]`）。

3. **生产 backend 选择缺指南**：ADR-023 D1-D4 给了 tier 排序，但没有「按语料规模 / 部署形态选哪个 backend」的可操作矩阵。dev/小语料、单机嵌入式、大语料持久、hosted/multi-agent 各档的推荐路径分散在四个 spike 文件 + ADR-028 里。

本 ADR 记录把 qdrant / lancedb 两档推向生产规模的处理策略：qdrant server 生命周期/健康/collection 管理层 + lancedb 真实可构建性调查 + 索引调参 + 生产 backend 选择矩阵——全程 ADR-013：qdrant 需 live server（CI 无）、lancedb 需 protoc 且可能在某平台受阻，二者均以诚实 stop-condition 处理，不伪造跨环境通过。

## Decision

生产规模 backend 采用 **feature-gated、诚实定论、不破坏默认 0-dep baseline** 的策略：

### D1 — qdrant server 生命周期层（`vector-qdrant` feature）

`QdrantBackend` 在 `vector-qdrant` feature 下加一层不需要 live server 即可契约验证的生命周期管理（task-25.1）：

- **connect / config**：把 `Qdrant::from_url` 的连接参数（url / timeout / 可选 api-key / 可选 TLS）收敛为一个可验证的连接配置结构，从环境（`QDRANT_URL` 既有 + 可选扩展）或显式入参构造；配置校验（url 非空、dim>0、collection 名非空）在不连服务器时即可单测。
- **health-probe**：暴露一个健康探活入口（如 `health()` / `is_reachable()`）——在有 server 时返回 readiness，在无 server 时返回可识别的 `unreachable` 状态（不 panic、不静默成功）。探活的**请求/响应 shape**（构造探活调用、解析 readiness 字段）可在不连服务器下断言；真实探活打到 live server 属集成验证。
- **collection ensure-create**：把 `open` 的「无脑 drop+create」改为 ensure-create 语义——存在且 dim/metric 匹配则复用，不存在则创建，dim/metric 不匹配则返回可识别错误（不静默重建丢数据）。ensure-create 的**决策逻辑**（given 一个 collection-describe 响应 → 决定 reuse / create / error）可在喂入构造的响应下单测，不需 live server。
- **honest defer**：真实连一个 live qdrant server 跑 connect→ensure-create→upsert→KNN 的端到端集成，因 CI 无在跑的 qdrant server，作诚实 stop-condition `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（task-25.1 §3 范围外，承 `docs/spikes/phase-18-qdrant.md` server-lifecycle Follow-up）。**ADR-013 红线：不在源码/文档伪造「KNN over live qdrant 通过」**——契约层（shape/config/ensure-create 决策）真实单测可断言；live-server 集成如实延后。

### D2 — lancedb 真实可构建性调查 + 索引调参参数（`vector-lancedb` feature）

lancedb 经 task-25.2 在 dev box 上真实 `cargo build --features vector-lancedb`（仿 task-23.2 sqlite-vec MSVC 调查 pattern）：

- **可构建性**：在 dev box 工具链上真实尝试构建（含 `protoc` 前置安装/探测）。构建通过则记录真实凭据（rustc / protoc 版本 / 构建耗时）+ feature 下既有 lancedb backend 契约不退化；若 `protoc` 缺失不可补、或 Lance/DataFusion/Arrow 在该平台构建确证受阻，则诚实文档化 stop-condition `[SPEC-DEFER:phase-future.lancedb-buildability]`（承 `docs/spikes/phase-18-lancedb.md` 的 protoc-prereq 记录 + sqlite-vec 当年 MSVC 受阻先例），**不伪造跨平台构建通过**（ADR-013）。
- **索引调参参数**：把 lancedb 的 ANN 索引调参参数（IVF_PQ / HNSW 的 `num_partitions` / `num_sub_vectors` / metric）+ 数据集 compaction 触发口径，收敛为一个可校验的索引配置结构（参数范围校验在不建真实索引下可单测）；真实建大索引 + compaction 性能属构建通过后的集成验证，未通过时随构建 stop-condition 一并如实延后（`[SPEC-DEFER:phase-future.lancedb-index-tuning]` / `[SPEC-DEFER:phase-future.lancedb-schema-compaction]`，承 `docs/spikes/phase-18-lancedb.md` Follow-up）。
- **spike 证据**：产出 `docs/spikes/phase-25-lancedb-buildability.md`，ADR-013 三态如实标（🟢 构建通过 / 🔴 确证受阻 stop-condition / 🟡 部分平台/有 caveat）。

### D3 — 生产 backend 选择矩阵（task-25.3 收口）

据 ADR-023 D1-D4 tier 排序 + ADR-028 嵌入式/fallback 推进 + 本 phase qdrant/lancedb 推进结果，产出一张「语料规模 × 部署形态 → 推荐 backend」选择矩阵（dev/小语料 → hnsw（D2，含 ADR-028 持久化）；单机嵌入式持久 → sqlite-vec（D1，ADR-028 MSVC 通过）；大语料嵌入式列存 → lancedb（D4，本 phase 可构建性结论）；hosted/multi-agent/scale-out → qdrant（D3，本 phase 生命周期层）），并记录每档的 caveat（live-server 依赖 / protoc 前置 / 平台限制）。矩阵是 add-only 指南，不溯改 ADR-023 D1-D6 tier 排序。

### D4 — 默认构建不变：0 vector 依赖 + BM25-only baseline

qdrant 生命周期层、lancedb 可构建性/调参参数**全部在各自 feature（`vector-qdrant` / `vector-lancedb`）下**，默认构建 0 新 vector 依赖、`QdrantBackend` / `LanceDbBackend` 不编译、语义路径仍经默认 0-dep `BruteForceVectorBackend`（ADR-023 D5）。qdrant-client / lancedb / arrow-array / futures 均已是 `core/Cargo.toml` 既有 optional dep（task-18.4/18.5），本 phase 不新增 direct dep；若调查需新增（如 lancedb 索引调参的 crate 面），经主 agent R7 chore + ADR-008 add-only 记录。本 ADR 不改 task-18.1 三 trait（`VectorBackend`/`VectorIndexer`/`VectorSearcher`）签名。

## Consequences

- **Positive**: qdrant 从「spike open=drop+create」推进到有 connect/health/ensure-create 的生命周期层（契约层真实单测，live-server 集成诚实延后）；lancedb 可构建性据真实 dev-box 构建定论（通过则记真实凭据，受阻则诚实 stop-condition，缩小或如实记录 protoc-prereq 缺口）；生产 backend 选择有了可操作矩阵；默认构建保持 0 vector 依赖 + 跨平台（ADR-023 D5 不破坏）。
- **Negative / open**: qdrant 真实 KNN 集成依赖 live server（CI 无 → D1 集成维度诚实延后）；lancedb 可能经调查在某平台仍因 protoc/Arrow 栈受阻（D2 受阻态如实记录，仿 sqlite-vec MSVC 先例）；lancedb 索引调参的真实性能属构建通过后续。
- **Ratification**: 本 ADR **Proposed**。task-25.1 真实契约单测（shape/config/ensure-create 决策，不连 server）+ task-25.2 真实 dev-box 构建结果（通过或 stop-condition）通过后，于 v0.18.0 closeout（task-25.3）据**真实非合成验证** ratify Proposed→Accepted（ADR-013：禁据合成/伪造 ratify）；某维度受阻（如 lancedb 构建在本平台受阻 / qdrant 无 live server 不能跑 KNN）则据「已达维度 ratify + 受阻维度如实记录」处理，不强 ratify。
- **Follow-ups**: ADR-023 D3/D4 tier 推进结果以 add-only Amendment 记录（不溯改 ADR-023 正文 D1-D6，D5）；qdrant live-server 集成 / KNN `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`；qdrant 集群/复制拓扑 `[SPEC-DEFER:phase-future.qdrant-deployment-topology]`；lancedb 索引调参真实性能 `[SPEC-DEFER:phase-future.lancedb-index-tuning]`；lancedb compaction `[SPEC-DEFER:phase-future.lancedb-schema-compaction]`；若 lancedb 调参引入新 crate 则 ADR-008 add-only 记依赖变更。

## Ratification Amendment (v0.18.0 / task-25.3)

> 本段在 v0.18.0 closeout（task-25.3）据 task-25.1/25.2 的**真实非合成验证**回填，把顶部 **Status** ratify **Proposed → Accepted**（ADR-013：禁据合成/伪造 ratify）。回填时按各维度真实结果记录：D1（qdrant 生命周期）契约层真实单测依据 + live-server 集成延后口径；D2（lancedb 可构建性）真实 dev-box 构建凭据（🟢 通过 / 🔴 受阻 stop-condition）+ 索引调参参数校验；D3（选择矩阵）落地；D4（默认 0-dep 不变）。某维度受阻则据「已达维度 ratify + 受阻维度如实记录」处理，不强 ratify。证据见 `docs/releases/v0.18.0-evidence.md`。Draft 阶段不填本段实测，由 task-25.3 实施时回填。
