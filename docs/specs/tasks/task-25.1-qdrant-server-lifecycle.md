# Task `25.1`: `qdrant-server-lifecycle — core/src/retriever/vector/qdrant.rs QdrantBackend 连接配置 + health-probe 入口 + collection ensure-create 决策（reuse/create/error 替代 spike 无脑 drop+create）+ feature vector-qdrant 下不连 live server 的契约测试（config 校验 / health-probe unreachable / ensure-create 三分支）`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 25 (production-vector-backend)
**Dependencies**: task-18.4（`QdrantBackend` via `qdrant-client` + `vector-qdrant` feature 已落地，spike open=drop+create 语义）/ task-18.1（`VectorIndexConfig`（`dim`/`metric`/`collection_id`）字段 + 三 trait freeze）/ ADR-023 D3（qdrant hosted/scale-out tier）/ ADR-030 D1（qdrant 生命周期层）/ ADR-008（core-library-selection，依赖变更 add-only）/ ADR-013（禁伪造 live-server 凭据）/ ADR-014 D1-D5（第十六次激活）

## 1. Background

Phase 18 task-18.4 用 `qdrant-client` 1.18（`core/src/retriever/vector/qdrant.rs`）实现 `QdrantBackend`：`new` 从 `QDRANT_URL`（默认 `http://localhost:6334`）`Qdrant::from_url(&url).build()` 建 gRPC 客户端 + 一个 `new_current_thread` tokio runtime（`block_on` 桥接同步 trait）；`open(config)` 直接 `delete_collection` + `create_collection(...vectors_config(VectorParamsBuilder::new(dim, Distance::Cosine)))`（无脑 drop+create，写死 Cosine）；`index_batch` 经 `UpsertPointsBuilder` 批量 upsert（`UPSERT_BATCH=1000`，`wait(true)`）；`search` 经 `SearchPointsBuilder` 取 KNN。ADR-023 D3 把 qdrant 定为 hosted/multi-agent/scale-out 档（最佳 ANN 吞吐 + server 托管持久/复制/过滤），但代价是打破单二进制模型（外部 server，+166MB）。

`docs/spikes/phase-18-qdrant.md` 明记 qdrant `is_local()==false`——它是**外部 server 进程**，spike 的真实数据在 WSL2 上用手动起的 `qdrant-x86_64-unknown-linux-musl` 静态二进制取得（无 Docker），并把「server lifecycle orchestration（auto-start / health-gate / shutdown）」列为 Follow-up（`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`）+「cluster / replication topology」（`[SPEC-DEFER:phase-future.qdrant-deployment-topology]`）。当前 spike 缺：connect 探活 / health-probe / collection ensure-create（存在则复用，不是无脑 drop+create 丢数据）/ 连接配置（timeout / 可选 api-key / 可选 TLS）。

CI 无在跑的 qdrant server，故真实 KNN over live qdrant 天然受限于「有无 live server」。本 task 把 qdrant 推向可生产化的关键：加一层**契约可验证**（不连 live server 即可单测 shape/config/ensure-create 决策）的生命周期层，真实 KNN over live qdrant 诚实延后。

## 2. Goal

`core/src/retriever/vector/qdrant.rs` 的 `QdrantBackend` 在 `vector-qdrant` feature 下新增生命周期层：(a) **连接配置**——把 url / 连接 timeout / 可选 api-key / 可选 TLS 收敛为一个可校验的连接配置结构，从环境（`QDRANT_URL` 既有）或显式入参构造，配置校验（url 非空、dim>0、collection 名非空、metric 受支持）在不连 server 时即可单测；(b) **health-probe**——暴露一个健康探活入口（如 `health()`），live 时返回 readiness、无 server 时返回可识别 `unreachable` 状态（不 panic、不静默成功）；(c) **collection ensure-create**——把 `open` 的「无脑 drop+create」改为 ensure-create 决策：给定一个 collection-describe 响应（或 absent），决定 reuse（存在且 dim/metric 匹配）/ create（不存在）/ error（存在但 dim/metric 不匹配，可识别错误不静默重建丢数据），决策逻辑在喂入构造的响应下可单测。≥3 Rust 测试（feature `vector-qdrant` 下，**不连 live server**）全 PASS：连接配置校验 + health-probe 无 server 返 unreachable 不 panic + collection ensure-create 决策 reuse·create·error 三分支。真实 KNN over live qdrant 诚实延后 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（CI 无 server，禁伪造 live-server 通过，ADR-013）。默认构建（无 `vector-qdrant`）0 新依赖、行为不变；`cargo test --workspace` 不退化。D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **修改 `core/src/retriever/vector/qdrant.rs`**：`QdrantBackend` 加连接配置结构（url / 连接 timeout / 可选 api-key / 可选 TLS 字段 + 校验方法）；加 health-probe 入口（`health()` / `is_reachable()`——live 时打 readiness 端点返回 ready/notready，无 server 时返回可识别 `unreachable` 错误态，不 panic）；把 `open` 的 drop+create 改为 ensure-create 决策（given collection 状态 → reuse/create/error），ensure-create 的决策函数与 gRPC 调用分离以便不连 server 单测。
- **复用 `core/src/retriever/vector/types.rs::VectorIndexConfig`**（`dim` / `metric` / `collection_id`）作为 ensure-create 的期望值来源（dim/metric/名匹配判定）。
- **复用既有 `QDRANT_URL` env**（`QdrantBackend::new`）作连接来源，按需扩展可选 api-key / TLS 配置入参（不破坏既有 `new` 行为）。
- **新增同源 Rust 单测（`core/src/retriever/vector/qdrant.rs` 内 `#[cfg(test)] mod tests`，feature `vector-qdrant` gated，不连 live server）**：(a) 连接配置校验——合法配置 Ok、url 空 / dim=0 / collection 名空 → 可识别 Err；(b) health-probe——无 server 时 `health()` 返可识别 unreachable 不 panic；(c) collection ensure-create 决策——given「存在且匹配」→ reuse、「不存在」→ create、「存在但 dim/metric 不匹配」→ error 三分支，喂入构造的 describe 响应断言。
- **可选修改 `core/Cargo.toml`**：`vector-qdrant` feature 若需生命周期相关 crate 面——按 add-only 评估，依赖变更经主 agent R7 chore（subagent 不自改 Cargo.toml）；qdrant-client 1.18 已 optional。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **真实连一个 live qdrant server 跑 connect→ensure-create→upsert→KNN 的端到端集成** [SPEC-DEFER:phase-future.qdrant-server-lifecycle]：CI 无在跑的 qdrant server（`docs/spikes/phase-18-qdrant.md` 用手动 musl 二进制取真实数据）；本 task 落契约层（shape/config/ensure-create 决策可单测），live-server 集成 / KNN over live qdrant 诚实延后，禁伪造 live-server 通过（ADR-013）。
- **qdrant 集群 / 复制 / 部署拓扑 / auto-start / shutdown 编排** [SPEC-DEFER:phase-future.qdrant-deployment-topology]：hosted 运维硬化项，承 `docs/spikes/phase-18-qdrant.md` Follow-up。
- **`QdrantBackend` 的 `index_batch` / `search` 本体** [SPEC-OWNER:task-18.4-spike-qdrant]：本 task 在其上加生命周期层，不重写 upsert/KNN。
- **lancedb 可构建性 / 索引调参** [SPEC-OWNER:task-25.2-lancedb-buildability-and-index-tuning]：本 task 仅做 qdrant 生命周期。
- **把 qdrant 接进 `core/src/server.rs` 语义热路径** [SPEC-DEFER:phase-future.vector-retrieval-integration]：本 task 落 backend 层生命周期能力 + 单测；热路径接入后续。
- **生产 backend 选择矩阵 / smoke v15 / v0.18.0 closeout** [SPEC-OWNER:task-25.3-closeout-v0.18.0]：本 task 交付 qdrant 生命周期层，矩阵/收口在 25.3。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`core/src/retriever/vector/qdrant.rs::QdrantBackend`**：task-18.4 qdrant backend，本 task 加连接配置 + health-probe + ensure-create。
- **`core/src/retriever/vector/types.rs::VectorIndexConfig`**：`dim`/`metric`/`collection_id` 字段，本 task ensure-create 决策的期望值来源。
- **`qdrant_client::Qdrant` + `CreateCollectionBuilder`**：gRPC 客户端 + collection 管理 API，本 task 在其上加 ensure-create 决策（决策函数与 gRPC 调用分离以便不连 server 单测）。
- **live qdrant server**：外部 server 进程（`is_local()==false`）；本 task 不依赖其在 CI 存在——真实 KNN 集成诚实延后。
- **下游 task-25.3**：closeout 据本 task 生命周期层评估选择矩阵 qdrant 档 caveat + smoke v15。

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/retriever/vector/qdrant.rs`（`QdrantBackend` / `new`（`QDRANT_URL` + `Qdrant::from_url` + tokio runtime）/ `open`（drop+create + `Distance::Cosine`）/ `index_batch`（`UpsertPointsBuilder` + `UPSERT_BATCH`）/ `search`（`SearchPointsBuilder`）/ `is_local()==false`）
- `core/src/retriever/vector/types.rs::VectorIndexConfig`（`dim` / `metric: VectorMetric` / `collection_id: String` / `persistence_path`）+ `VectorChunk` / `VectorHit` / `VectorError`（`Backend{source}` / `Other` 变体）
- `core/src/retriever/vector/traits.rs`（`VectorBackend` / `VectorIndexer` / `VectorSearcher` 三 trait 接口，不改签名）
- `core/src/retriever/vector/mod.rs`（`#[cfg(feature="vector-qdrant")] pub use qdrant::QdrantBackend`）
- `docs/spikes/phase-18-qdrant.md`（external server `is_local()==false` + server RSS≈104.8MB + 真实数据经手动 musl 二进制 + server-lifecycle/topology Follow-up）+ `docs/decisions/adr-023-vector-backend-default.md` D3 + Consequences/Follow-ups
- `docs/decisions/adr-030-production-vector-backend.md` D1（qdrant 生命周期）+ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造 live-server）+ `docs/decisions/adr-008-core-library-selection.md`（依赖 add-only）+ `qdrant-client` 1.18 文档（`Qdrant::from_url` 连接参数 / `health_check` / `collection_exists` / `collection_info` API 面核实）

### 5.2 关键设计 — 连接配置 + health-probe + collection ensure-create

- **连接配置结构**：把 `Qdrant::from_url` 的连接参数收敛为可校验结构（url / 连接 timeout / 可选 api-key / 可选 TLS）。`from_env`（读 `QDRANT_URL` 既有 + 可选扩展）与显式入参两条构造路径；`validate()`（url 非空 / dim>0 / collection 名非空 / metric 受支持）在不连 server 下纯函数可单测。
- **health-probe**：`health()` 入口——live 时打 qdrant readiness（`qdrant-client` 健康/版本端点）返回 ready/notready；无 server 时（连接失败）返回可识别 `unreachable` 状态（映射到 `VectorError::Backend{source}` 或专用态），不 panic、不静默成功。无 server 时的 unreachable 路径 deterministic 可单测（连一个不存在的端点）。
- **collection ensure-create 决策**：把决策逻辑（纯函数）与 gRPC 调用分离——`decide_ensure(existing: Option<CollectionDesc>, want: &VectorIndexConfig) -> EnsureAction` 返回 `Reuse` / `Create` / `Error(mismatch)`：存在且 dim+metric 匹配 → `Reuse`（不 drop，保数据）；不存在 → `Create`；存在但 dim/metric 不匹配 → `Error`（可识别，不静默重建丢数据）。`open` 调 `collection_exists`/`collection_info` 取 existing → `decide_ensure` → 据 action 执行（reuse skip / create / 返回 error）。决策函数喂入构造的 `CollectionDesc` 三分支单测，不需 live server。
- **ADR-013**：契约层（config 校验 / health-probe unreachable shape / ensure-create 决策）是 deterministic feature 测试可验证项（🟡 feature 下不连 server 真实契约）；真实 KNN over live qdrant 是 live-server 集成，CI 无 server 故诚实延后，不预判 live 召回数值、不伪造 live-server 通过。

### 5.3 不变量

- 默认构建（无 `vector-qdrant` feature）0 新依赖、`QdrantBackend` 不编译、行为逐字节不变（ADR-023 D5 / ADR-004）。
- `decide_ensure` 纯函数：given 相同 existing+want → 相同 action（确定性，可单测三分支）。
- health-probe 不静默吞错：无 server 时返回可识别 unreachable，调用方可据此 gate，不伪造「连接成功」（ADR-013）。
- ensure-create 不静默丢数据：dim/metric 不匹配返回可识别 error，不无脑 drop+create（替代 spike 语义）。
- 不改三 trait 签名（`VectorBackend` / `VectorIndexer` / `VectorSearcher`）——生命周期方法为 `QdrantBackend` inherent method 或经既有 `open` 生命周期接入，不破坏 task-18.1 trait freeze。
- 既有 `QDRANT_URL` 默认 + `new` 行为向后兼容（扩展配置为可选入参，不破坏既有构造）。

## 6. Acceptance Criteria

- [ ] **AC1**: feature `vector-qdrant` 下连接配置校验——合法配置（url 非空 / dim>0 / collection 名非空 / metric 受支持）`validate()` Ok；非法（url 空 / dim=0 / collection 名空）→ 可识别 Err（不连 server，纯函数）— verified by **TEST-25.1.1**
- [ ] **AC2**: health-probe 无 server 时返回可识别 unreachable 状态——`health()` 连一个不存在端点返 unreachable（不 panic、不静默成功），可被调用方识别 — verified by **TEST-25.1.2**
- [ ] **AC3**: collection ensure-create 决策三分支——`decide_ensure`：存在且 dim/metric 匹配 → Reuse、不存在 → Create、存在但 dim/metric 不匹配 → Error（喂入构造的 describe 响应，不连 server）— verified by **TEST-25.1.3**
- [ ] **AC4**: 真实 KNN over live qdrant 诚实延后 + 不破坏三 trait 签名——`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` 在 spec/spike 如实标（CI 无 live server，禁伪造 live-server 通过 ADR-013）；生命周期方法为 inherent method 不改三 trait 签名 — verified by **TEST-25.1.4**
- [ ] **AC5**: 既有不退化 + D2 lint — 默认 `cargo test --workspace`（无 vector feature）全 PASS + 0 新依赖；`cargo test --workspace --features vector-qdrant` 既有 qdrant 契约不退化；`go test ./...` 不受影响（本 PR 零 Go delta）；`bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-25.1.5** + §10 实测

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-25.1.1 | feature `vector-qdrant` 连接配置校验（合法 Ok / url 空·dim=0·名空 Err，纯函数不连 server） | `core/src/retriever/vector/qdrant.rs`（`mod tests`） | Planned |
| TEST-25.1.2 | health-probe 无 server 返可识别 unreachable 不 panic | `core/src/retriever/vector/qdrant.rs`（`mod tests`） | Planned |
| TEST-25.1.3 | collection ensure-create 决策 reuse·create·error 三分支（构造 describe 响应） | `core/src/retriever/vector/qdrant.rs`（`mod tests`） | Planned |
| TEST-25.1.4 | live-server 集成诚实延后标注 + 不破坏三 trait 签名（trait object 构造） | `core/src/retriever/vector/qdrant.rs`（`mod tests`）+ spec/spike | Planned |
| TEST-25.1.5 | 默认 `cargo test --workspace` 0 failed + `--features vector-qdrant` 不退化 + D2 lint 0 未标注命中 | 全 Rust + `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（高）真实 KNN 需 live qdrant server，CI 无**（承 phase-25 §7 R1）：`is_local()==false`，CI 无在跑的 qdrant server。
  - **缓解**：把生命周期层拆为契约层（config 校验 / health-probe unreachable shape / ensure-create 决策纯函数）与 live-server 集成；契约层在不连 server 下 deterministic 单测可断言（喂入构造响应 / 校验入参 / 连不存在端点）；真实 KNN over live qdrant 诚实延后 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`，按 ADR-013 不伪造 live-server 通过。AC1-3 在契约层满足，AC4 记延后。
- **R2（中）`qdrant-client` 1.18 API 面与设计假设不符**（如 `collection_info` / health 端点形态）：决策需核实真实 API。
  - **缓解**：先核实 `qdrant-client` 1.18 的 `collection_exists` / `collection_info` / 健康端点 API 面；`decide_ensure` 纯函数只依赖从 API 响应抽出的 (dim, metric, exists) 三元组，与具体 API 形态解耦——API 变化只影响抽取层，决策层稳定可单测。
- **R3（低）生命周期引入新 crate 面**：default build 须 0 新依赖。
  - **缓解**：优先复用 `qdrant-client` 1.18 既有 API（health / collection 管理已在 client 内）；如需新 crate 仅在 `vector-qdrant` feature 下引入，经主 agent R7 chore（subagent 不自改 Cargo.toml），默认构建 0 新 dep（ADR-023 D5 / ADR-004）。

## 9. Verification Plan

```bash
# Rust：默认构建（无 vector feature）0 新依赖 + 不退化
cargo test --workspace

# feature 下 qdrant 生命周期契约（不连 live server）
cargo test --workspace --features vector-qdrant
cargo test -p contextforge-core --features vector-qdrant retriever::vector::qdrant

# Go 不退化（本 PR 零 Go delta）
go test ./...

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **Status**: 待实施（Draft）。实施完成后按 6 项回填：完成日期 / 改动文件 / commit 列表 / §9 Verification 实测结果（ADR-013 真实非合成）/ 设计取舍（连接配置结构 + health-probe unreachable 语义 + ensure-create 决策三分支 + live-server 集成延后口径）/ 剩余风险 + 下游影响。
