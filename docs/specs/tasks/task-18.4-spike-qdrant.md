# Task `18.4`: `spike-qdrant — core/src/retriever/vector/qdrant.rs QdrantBackend (qdrant-client gRPC + 自带 tokio runtime block_on) + bench 注册表接入 + 5 维 evidence`

**Status**: Done

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 18 (vector-backend-selection)
**Dependencies**: task-18.1（`Vector{Backend,Indexer,Searcher}` 三 trait 冻结）/ task-18.2（spike harness）/ task-18.3（sqlite-vec 先行 + bench 注册表模式）/ ADR-008 core-library-selection（新增 qdrant-client dep）/ ADR-014 D1-D5 第十一次激活

## 1. Background

Phase 18 §2A 候选集第 2 路 = Qdrant（外部向量数据库，gRPC）。原 handoff 记 qdrant 在 Windows 无法无人值守起服务而延后。在 Linux x86_64（WSL2 Ubuntu 26.04）上，Qdrant **v1.18.1 静态 musl 单二进制**可后台直接运行（REST 6333 healthz pass / gRPC 6334 listen），`qdrant-client = 1.18.0` 经一次性探针证实可连接 + `Distance::Cosine` KNN 正确（id 1 score 0.9986 / id 3 0.9983 / 正交 id 2 0.0526）。

与 hnsw / sqlite-vec（进程内）不同，Qdrant 是**独立 server 进程**（`is_local = false`，需运行 server）。因此本 task 用 `qdrant-client` 直连本机 Qdrant 跑真实 5 维 evidence；并记录 qdrant 是「外部服务」这一选型关键差异。

## 2. Goal

在 `core/src/retriever/vector/qdrant.rs` 落地 `QdrantBackend`，实现三 trait（`qdrant-client` 异步经自带 `tokio::runtime::Runtime` + `block_on` 桥接到 sync trait；`Distance::Cosine` 直接 cosine，与 task-18.2 cosine 真值一致）；用 `vector-qdrant` feature gate（默认构建不引入 qdrant-client dep）；接入 `bench/src/backends.rs` 注册表；跑出真实 `recall@5/10 + P95 + RSS + cold-start + reindex` 并落 `docs/spikes/phase-18-qdrant.md`（qdrant **server** 进程 RSS 经 /proc 外采，与 harness client RSS 区分标注）。默认 `cargo test --workspace` + `go test ./...` 不退化；D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **新建 `core/src/retriever/vector/qdrant.rs`** — `QdrantBackend`：
  - `Qdrant::from_url`（默认 `http://localhost:6334`，可经 `QDRANT_URL` 覆盖）+ 自带 `tokio::runtime::Runtime`，每个 trait 方法内 `rt.block_on(async { ... })` 桥接异步 client（bench harness 为 sync）
  - `Distance::Cosine` 原生 cosine（无需归一化）；`chunk_id`(String) ↔ 数值 `PointId`(u64) 经 `id_map` 互映
  - `open` `delete_collection` + `create_collection`（dim + Cosine）；`index_batch` 分批 `upsert_points(wait=true)`；`delete` 重建集合（全量 reindex 语义）；`search` `search_points` → `VectorHit`
  - 手写 `Debug`（client 不实现 Debug）；`Qdrant`/`Runtime`/`Mutex` 皆 Send+Sync
- **修改 `core/Cargo.toml`** — `qdrant-client = { version = "1.18", optional = true }` + `vector-qdrant = ["dep:qdrant-client"]`（默认不启用）。
- **修改 `core/src/retriever/vector/mod.rs`** — `#[cfg(feature = "vector-qdrant")] pub mod qdrant; pub use qdrant::QdrantBackend;`。
- **修改 `bench/Cargo.toml`** — `vector-qdrant = ["contextforge-core/vector-qdrant"]`。
- **修改 `bench/src/backends.rs`** — `#[cfg(feature = "vector-qdrant")] "qdrant"` 注册表分支 + `known_backends`。
- **新建 `docs/spikes/phase-18-qdrant.md`** — Linux 真实 5 维 evidence + qdrant server 进程 RSS 外采标注。
- **修改 `scripts/spike_vector_backends.sh`** — 注释引导（qdrant 需先起 server，不入默认 BACKENDS）。
- **修改 `docs/s2v-adapter.md`** — Phase 18 表 18.4 行 Deferred → Done。

### Out of Scope（[SPEC-DEFER] / [SPEC-OWNER]）

- **Qdrant 集群 / 生产部署拓扑** [SPEC-DEFER:phase-future.qdrant-deployment-topology]：spike 用本机单节点，集群后置。
- **lancedb backend** [SPEC-OWNER:task-18.5-spike-lancedb]：不在本 task。
- **默认 backend 选型 + ADR-023** [SPEC-OWNER:task-18.7-decision-adr023]：4 路数据齐后决策。
- **embedded / in-process 模式** [SPEC-DEFER:phase-future.qdrant-embedded-mode]：Qdrant 无原生 Rust embedded；spike 用本机 server 进程。
- **server 自动拉起编排** [SPEC-DEFER:phase-future.qdrant-server-lifecycle]：spike 假定 server 已运行（手动/脚本起 binary），编排后置。
- **非 Linux RSS 采样** [SPEC-DEFER:phase-future.rss-sampling-macos-windows]：承 task-18.2 R3。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`QdrantBackend`**：core vector 模块新成员，cfg-gated，`is_local=false`。
- **Qdrant server**：本机 v1.18.1 musl binary（gRPC 6334）。
- **bench 注册表**：`run_named("qdrant", ...)` 派发。
- **下游 task-18.7**：消费本 task 的 qdrant 5 维 evidence + 外部服务差异做选型。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/tasks/task-18.1-vector-trait.md` + `task-18.2-spike-harness.md` + `task-18.3-spike-sqlite-vec.md`（bench 注册表 + id_map 模式）
- `core/src/retriever/vector/{traits,types}.rs` + `core/src/retriever/vector/sqlite_vec.rs`（同构实现参考）
- `docs/decisions/adr-008-core-library-selection.md` + `adr-014-cross-phase-exit-criteria-validation.md`

### 5.2 Imports（qdrant-client 新增 optional dep；默认 0 新 dep）

```rust
use std::sync::Mutex;
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{CreateCollectionBuilder, Distance, VectorParamsBuilder, UpsertPointsBuilder, PointStruct, SearchPointsBuilder};
use qdrant_client::Payload;
use crate::retriever::vector::traits::{VectorBackend, VectorIndexer, VectorSearcher};
use crate::retriever::vector::types::{ChunkId, VectorChunk, VectorError, VectorFilter, VectorHit, VectorIndexConfig, VectorScore};
```

### 5.3 关键设计

- 异步→同步桥接：`QdrantBackend { client: Qdrant, rt: tokio::runtime::Runtime, id_map: Mutex<Vec<String>>, collection: Mutex<String>, dim: Mutex<usize> }`；trait 方法内 `self.rt.block_on(...)`（bench 无 ambient runtime，`block_on` 安全）。
- `Distance::Cosine` 原生 cosine；qdrant `point.score` 即 cosine 相似度（降序，最佳在前）→ 与 cosine 真值排序一致。
- `open`：`delete_collection`（忽略 not-found）+ `create_collection(dim, Cosine)`；记 collection/dim、清 id_map。
- `index_batch`：rowid = `id_map.len()` 起的 u64；分批（每 1000）`upsert_points(... .wait(true))`；`id_map.push(chunk_id)`；维度不符返 `DimMismatch`。
- `search`：`search_points(collection, query, k)` → `point.id`(Num) 经 id_map → chunk_id；score = `point.score`。
- `delete` 重建集合（全量 reindex）；`flush` no-op（upsert wait=true 已落）；`is_indexed` = id_map 非空；`is_local()=false`。
- **RSS 区分**：harness `sample_rss_mb()` 采的是 bench **client** 进程 RSS（非 qdrant server）；qdrant **server** 进程 RSS 经 `/proc/<pid>/status` VmRSS 外采，evidence 中明确区分标注（ADR-013 不伪造）。

## 6. Acceptance Criteria

- [x] **AC1**: `QdrantBackend` 实现三 trait；`cargo build -p contextforge-bench --features vector-qdrant` exit 0；默认构建（无 feature）不引入 qdrant-client（cfg-gated）— verified by **TEST-18.4.1**（feature build PASS + 默认 `cargo build` 无 qdrant-client 编译）
- [x] **AC2**: 真实召回 — `spike --backend qdrant`（连本机 Qdrant server）产出 `recall@5/10`（qdrant Cosine KNN 对 task-18.2 brute-force cosine 真值）非伪造，记录于 `docs/spikes/phase-18-qdrant.md` — verified by **TEST-18.4.2**（spike run JSON + evidence）
- [x] **AC3**: cosine 一致性 — `Distance::Cosine` 使 qdrant KNN 匹配 cosine 真值（recall 高，非 0）— verified by **TEST-18.4.3**（spike recall@5 ≥ 0.9）
- [x] **AC4**: harness 端到端真 backend — runner 返完整 `MeasureReport`（P95 / cold-start / reindex 记录），无 panic；qdrant server 进程 RSS 外采记入 evidence — verified by **TEST-18.4.4**（release spike exit 0 + server VmRSS 实采）
- [x] **AC5**: 既有不退化 — 默认 `cargo test --workspace` 全 PASS（vector-qdrant 默认不启用）；`go test ./...` 全 PASS — verified by **TEST-18.4.5**（`cargo test --workspace` 0 failed）+ §10 实测
- [x] **AC6**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched master` PR 触及行 0 未标注命中 — verified by §10 记录的 D2 lint 实跑输出

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-18.4.1 | feature build PASS + 默认无 qdrant-client | `cargo build -p contextforge-bench --features vector-qdrant` | Done |
| TEST-18.4.2 | qdrant spike 真实 recall + evidence | `docs/spikes/phase-18-qdrant.md` | Done |
| TEST-18.4.3 | cosine 一致性 recall@5 ≥ 0.9 | spike run | Done |
| TEST-18.4.4 | runner 真 backend 5 维全填无 panic + server RSS 外采 | release spike JSON + /proc VmRSS | Done |
| TEST-18.4.5 | 默认 cargo test --workspace 0 failed | 全 workspace | Done |

## 8. Risks

- **R1（中）外部服务依赖**：qdrant 需独立 server 进程（`is_local=false`）；spike 假定本机 server 已运行。
  - **缓解**：evidence 标注「外部服务」差异；server lifecycle 编排 [SPEC-DEFER:phase-future.qdrant-server-lifecycle]；task-18.7 选型须权衡运维成本。
- **R2（中）harness RSS 非 server 内存**：`sample_rss_mb` 采 client 进程，非 qdrant server。
  - **缓解**：qdrant server VmRSS 经 /proc 外采记入 evidence，与 client RSS 区分标注。
- **R3（中）合成种子向量 recall 偏理想**：承 task-18.3 R2；真实分布 recall 见 dogfood + 大 n。
  - **缓解**：evidence 标注数据来源；task-18.7 横向对比复跑。
- **R4（低）gRPC 网络开销计入 cold-start**：cold-start/reindex 含 localhost gRPC 往返（进程内 backend 无此开销）。
  - **缓解**：evidence 明确 cold-start 含网络往返,这是 qdrant 架构的真实成本（选型权衡因子）。

## 9. Verification Plan

```bash
# Linux x86_64 — 先起 qdrant server (v1.18.1 musl binary, gRPC 6334)
QDRANT__SERVICE__GRPC_PORT=6334 ./qdrant &
cargo build -p contextforge-bench --features vector-qdrant
cargo run --release -q -p contextforge-bench --features vector-qdrant -- --backend qdrant --n 5000 --dim 64 --seed 1 --m 500 --out docs/spikes/phase-18-qdrant.md
cat /proc/$(pgrep -f qdrant)/status | grep VmRSS    # server RSS 外采
cargo test --workspace        # 默认 feature，qdrant gated 不入编译
go test ./...
bash scripts/spec_drift_lint.sh --touched master
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**：2026-05-30
- **改动文件**：`core/src/retriever/vector/qdrant.rs`（新增）、`core/src/retriever/vector/mod.rs`（cfg-gated export）、`core/Cargo.toml`（qdrant-client optional + vector-qdrant）、`bench/Cargo.toml`（vector-qdrant feature）、`bench/src/backends.rs`（qdrant arm）、`docs/spikes/phase-18-qdrant.md`（新增实测）、`scripts/spike_vector_backends.sh`（注释）、`docs/s2v-adapter.md`（18.4 行 Done）
- **commit 列表**：见本 task PR（分支 `feat/task-18.4-spike-qdrant`）；合入后以 merge commit 为准
- **§9 Verification 结果**：见 PR 描述与 `docs/spikes/phase-18-qdrant.md`（Linux + 本机 Qdrant v1.18.1 实测填充）
- **剩余风险 / 未做项**：server lifecycle 编排 / 集群拓扑后置；lancedb(18.5) 见 [SPEC-OWNER]
- **下游 task 影响**：task-18.7（消费 qdrant 5 维 evidence + 外部服务差异做 ADR-023 选型）
