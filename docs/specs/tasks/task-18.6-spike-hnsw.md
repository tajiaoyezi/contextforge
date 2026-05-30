# Task `18.6`: `spike-hnsw — core/src/retriever/vector/hnsw.rs HnswBackend (instant-distance) + bench 注册表接入 + 5 维 evidence`

**Status**: Done

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 18 (vector-backend-selection)
**Dependencies**: task-18.1（`Vector{Backend,Indexer,Searcher}` 三 trait 冻结）/ task-18.2（spike harness：corpus + measure + runner）/ ADR-008 core-library-selection（新增 instant-distance dep）/ ADR-014 D1-D5 第九次激活

## 1. Background

Phase 18 §2A 候选集第 4 路 = 内嵌 HNSW。本 task 用 **`instant-distance`**（纯 Rust HNSW 近似最近邻，无 C 依赖）实现 `HnswBackend`，接入 task-18.2 的测量台跑 5 维 evidence。

`instant-distance` 是纯 Rust，在 P0 平台（Linux x86_64）与开发机（Windows MSVC）均可构建——区别于 [SPEC-OWNER:task-18.3-spike-sqlite-vec]（sqlite-vec C loadable extension 在 Windows MSVC 构建失败，见 `docs/spikes/phase-18-sqlite-vec.md`，已按 phase-18 §7 R1 [SPEC-DEFER:phase-future.sqlite-vec-cross-platform]）。因此本 task 是首个产出**真实召回数据**的 backend spike。

## 2. Goal

在 `core/src/retriever/vector/hnsw.rs` 落地 `HnswBackend`，实现三 trait（向量单位归一化 + 欧氏距离，与 task-18.2 cosine 真值单调一致）；用 `vector-hnsw` feature gate（默认构建不引入 instant-distance dep）；接入 `bench/src/backends.rs` 注册表；跑出真实 `recall@5/10 + P95 + RSS + cold-start + reindex` 并落 `docs/spikes/phase-18-hnsw.md`。默认 `cargo test --workspace` + `go test ./...` 不退化；D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **新建 `core/src/retriever/vector/hnsw.rs`** — `HnswBackend`：
  - 单位归一化 + 欧氏距离（unit 向量上欧氏与 cosine 单调一致 → HNSW 近邻 == task-18.2 brute-force cosine 真值）
  - `instant-distance` 一次性建图（无增量插入）→ `index_batch` 累积、`flush` 建 `HnswMap`（全量 reindex 语义，承 task-18.1）
  - `Mutex` 内部可变（trait 全 `&self`）
- **修改 `core/Cargo.toml`** — `instant-distance = { version = "0.6", optional = true }` + `vector-hnsw = ["dep:instant-distance"]`（默认不启用）。
- **修改 `core/src/retriever/vector/mod.rs`** — `#[cfg(feature = "vector-hnsw")] pub mod hnsw; pub use hnsw::HnswBackend;`。
- **修改 `bench/Cargo.toml`** — `vector-hnsw = ["contextforge-core/vector-hnsw"]`。
- **修改 `bench/src/backends.rs`** — `#[cfg(feature = "vector-hnsw")] "hnsw"` 注册表分支 + `known_backends`。
- **新建 `docs/spikes/phase-18-hnsw.md`** — 真实 5 维测量 evidence。
- **新建 `docs/spikes/phase-18-sqlite-vec.md`** — sqlite-vec Windows 构建受阻记录（[SPEC-OWNER:task-18.3-spike-sqlite-vec] 的 Linux-first 凭据）。

### Out of Scope（[SPEC-DEFER] / [SPEC-OWNER]）

- **sqlite-vec backend** [SPEC-OWNER:task-18.3-spike-sqlite-vec]：Windows MSVC 构建失败，Linux-first，本 task 仅记录凭据不实现。
- **qdrant-embedded backend** [SPEC-OWNER:task-18.4-spike-qdrant-embedded]：需运行 Qdrant server / embedded segment，不在本 task。
- **lancedb backend** [SPEC-OWNER:task-18.5-spike-lancedb]：Arrow/Lance 重型依赖，不在本 task。
- **默认 backend 选型 + ADR-023** [SPEC-OWNER:task-18.7-decision-adr023]：4 路数据齐后决策。
- **HNSW 图 serde 持久化** [SPEC-DEFER:phase-future.hnsw-graph-persistence]：spike 用内存重建测 cold-start/reindex，落盘持久化后置。
- **增量插入** [SPEC-DEFER:phase-future.vector-incremental-index]：instant-distance 一次性建图，增量后置。
- **非 Linux RSS 采样** [SPEC-DEFER:phase-future.rss-sampling-macos-windows]：承 task-18.2 R3。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`HnswBackend`**：core vector 模块新成员，cfg-gated。
- **bench 注册表**：`run_named("hnsw", ...)` 派发。
- **下游 task-18.7**：消费本 task 的 hnsw 5 维 evidence 做选型。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/tasks/task-18.1-vector-trait.md`（trait 签名）+ `docs/specs/tasks/task-18.2-spike-harness.md`（harness API）
- `core/src/retriever/vector/{traits,types}.rs`（实施对照）
- `docs/decisions/adr-008-core-library-selection.md`（instant-distance 入库 amendment 依据）
- `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5）

### 5.2 Imports（instant-distance 新增 optional dep；默认 0 新 dep）

```rust
use std::sync::Mutex;
use instant_distance::{Builder, HnswMap, Point, Search};
use crate::retriever::vector::traits::{VectorBackend, VectorIndexer, VectorSearcher};
use crate::retriever::vector::types::{ChunkId, VectorChunk, VectorError, VectorFilter, VectorHit, VectorIndexConfig, VectorScore};
```

### 5.3 关键设计

- `HnswPoint(Vec<f32>)` impl `instant_distance::Point`，`distance` = 欧氏；索引/查询前 `normalize`（单位化）。
- `HnswBackend { pending: Mutex<Vec<(Vec<f32>, String)>>, map: Mutex<Option<HnswMap<HnswPoint, String>>> }`。
- `index_batch` 累积归一化向量 → `flush` `Builder::default().build(points, values)` 建图 → `search` 锁图、`map.search(&q, &mut Search::default()).take(k)` → `VectorHit`（score = `1 - dist/2`）。
- `is_indexed` = `map.is_some()`；`delete` 清空（全量 reindex 语义）。

## 6. Acceptance Criteria

- [x] **AC1**: `HnswBackend` 实现 `VectorBackend`/`VectorIndexer`/`VectorSearcher` 三 trait；`cargo build -p contextforge-bench --features vector-hnsw` exit 0；默认构建（无 feature）不引入 instant-distance（cfg-gated）— verified by **TEST-18.6.1**（feature build PASS + 默认 `cargo build` 无 instant-distance 编译）
- [x] **AC2**: 真实召回 — `spike --backend hnsw` 产出 `recall@5/10`（HNSW 近邻对 task-18.2 brute-force cosine 真值）非伪造，记录于 `docs/spikes/phase-18-hnsw.md` — verified by **TEST-18.6.2**（spike run JSON + evidence；实测 n=2000/dim=32 recall@10=1.0）
- [x] **AC3**: cosine 一致性 — 单位归一化 + 欧氏使 HNSW 近邻匹配 cosine 真值（recall 高，非 0）— verified by **TEST-18.6.3**（spike recall@5 ≥ 0.9）
- [x] **AC4**: harness 端到端真 backend — runner 返完整 `MeasureReport`（P95 / cold-start / reindex 记录），无 panic — verified by **TEST-18.6.4**（release spike exit 0 + 5 维字段全填）
- [x] **AC5**: 既有不退化 — 默认 `cargo test --workspace` 全 PASS（vector-hnsw 默认不启用，gated path 不入默认编译）；`go test ./...` 全 PASS — verified by **TEST-18.6.5**（`cargo test --workspace` 0 failed）+ §10 实测
- [x] **AC6**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched master` PR 触及行 0 未标注命中 — verified by §10 记录的 D2 lint 实跑输出

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-18.6.1 | feature build PASS + 默认无 instant-distance | `cargo build -p contextforge-bench --features vector-hnsw` | Done |
| TEST-18.6.2 | hnsw spike 真实 recall + evidence | `docs/spikes/phase-18-hnsw.md` | Done |
| TEST-18.6.3 | cosine 一致性 recall@5 ≥ 0.9 | spike run（实测 1.0） | Done |
| TEST-18.6.4 | runner 真 backend 5 维全填无 panic | release spike JSON | Done |
| TEST-18.6.5 | 默认 cargo test --workspace 0 failed | 全 workspace | Done |

## 8. Risks

- **R1（中）合成种子向量 recall 偏理想**：n=2000/dim=32 上 recall=1.0（向量近正交易分）；真实分布 recall 见 dogfood + Linux 大 n 跑批。
  - **缓解**：evidence 标注数据来源；task-18.7 横向对比在 Linux release 大 n 复跑。
- **R2（低）debug 建图耗时**：debug 下 n=2000 建图 ~2.2s；release 显著降低。
  - **缓解**：evidence 用 release 数；CI 不跑 spike（dev-time）。
- **R3（低）instant-distance 0.6 API 稳定性**：Cargo.lock pin；版本升级回归由后续 task 验证。

## 9. Verification Plan

```bash
cargo build -p contextforge-bench --features vector-hnsw
cargo run --release -q -p contextforge-bench --features vector-hnsw -- --backend hnsw --n 5000 --dim 64 --seed 1 --m 500
cargo test --workspace        # 默认 feature，hnsw gated 不入编译
go test ./...
bash scripts/spec_drift_lint.sh --touched master
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**：2026-05-30
- **改动文件**：`core/src/retriever/vector/hnsw.rs`（新增）、`core/src/retriever/vector/mod.rs`（cfg-gated export）、`core/Cargo.toml`（instant-distance optional + vector-hnsw）、`bench/Cargo.toml`（vector-hnsw feature）、`bench/src/backends.rs`（hnsw arm）、`docs/spikes/phase-18-hnsw.md` + `docs/spikes/phase-18-sqlite-vec.md`（新增）
- **commit 列表**：见本 task PR（分支 `feat/task-18.6-spike-hnsw`）；合入后以 merge commit 为准
- **§9 Verification 结果**：
  - build: ✅ `cargo build -p contextforge-bench --features vector-hnsw` exit 0；默认构建无 instant-distance
  - spike: ✅ hnsw recall@5=1.0 / recall@10=1.0（debug n=2000/dim=32）；release n=5000/dim=64 数见 `docs/spikes/phase-18-hnsw.md`
  - regression: ✅ `cargo test --workspace` 0 failed（默认 feature）；`go test ./...` 全 PASS
  - D2 lint: ✅ `--touched master` 0 未标注命中
- **剩余风险 / 未做项**：HNSW 落盘持久化 / 增量插入后置；sqlite-vec(18.3)/qdrant(18.4)/lancedb(18.5) 见各自 [SPEC-OWNER]，Linux/server 环境后续
- **下游 task 影响**：task-18.7（消费 hnsw 5 维 evidence + 其余 backend 数据做 ADR-023 选型）
