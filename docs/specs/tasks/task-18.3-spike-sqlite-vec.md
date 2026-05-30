# Task `18.3`: `spike-sqlite-vec — core/src/retriever/vector/sqlite_vec.rs SqliteVecBackend (rusqlite bundled + sqlite-vec vec0) + bench 注册表接入 + 5 维 evidence`

**Status**: Done

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 18 (vector-backend-selection)
**Dependencies**: task-18.1（`Vector{Backend,Indexer,Searcher}` 三 trait 冻结）/ task-18.2（spike harness：corpus + measure + runner）/ task-18.6（hnsw spike 先行 + sqlite-vec 构建受阻凭据）/ ADR-008 core-library-selection（新增 sqlite-vec dep）/ ADR-014 D1-D5 第十次激活

## 1. Background

Phase 18 §2A 候选集第 1 路 = SQLite `vec0` 扩展（[sqlite-vec](https://github.com/asg017/sqlite-vec)）。task-18.6 记录了 `sqlite-vec = 0.1.10-alpha.4` 在 Windows MSVC 构建受阻（`docs/spikes/phase-18-sqlite-vec.md`，已 [SPEC-DEFER:phase-future.sqlite-vec-cross-platform]）。

本 task 在 Linux x86_64（phase-18 §7 R1 P0 平台，gcc 15.2.0）复跑后定位到两点：

- `0.1.10-alpha.4` 预发布版打包不完整（`sqlite-vec.c:3772` `#include "sqlite-vec-diskann.c"` 但该源文件未随 crate 发布）→ 任何平台均无法编译，与工具链无关。
- **稳定版 `sqlite-vec = 0.1.9`** 在 Linux gcc 下 `cc-rs` 编译通过；`rusqlite`（bundled SQLite）+ `sqlite3_auto_extension(sqlite3_vec_init)` 注册扩展 + `vec0` 虚表 KNN 查询经一次性探针证实可用。

因此本 task 用 **稳定版 0.1.9** 在 Linux 落地 `SqliteVecBackend` 并产出真实 5 维 evidence。Windows MSVC 受阻结论维持（凭据保留），Linux 为本 spike 平台。

## 2. Goal

在 `core/src/retriever/vector/sqlite_vec.rs` 落地 `SqliteVecBackend`，实现三 trait（向量单位归一化 + `vec0` 默认 L2 距离，与 task-18.2 cosine 真值单调一致）；用 `vector-sqlite` feature gate（默认构建不引入 sqlite-vec dep）；接入 `bench/src/backends.rs` 注册表；跑出真实 `recall@5/10 + P95 + RSS + cold-start + reindex` 并落 `docs/spikes/phase-18-sqlite-vec.md`（替换原构建受阻记录为实测数据，Windows 受阻结论以附注保留）。默认 `cargo test --workspace` + `go test ./...` 不退化；D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **新建 `core/src/retriever/vector/sqlite_vec.rs`** — `SqliteVecBackend`：
  - `rusqlite`（既有 bundled dep）`Connection::open_in_memory` + `std::sync::Once` 守护的 `sqlite3_auto_extension(sqlite3_vec_init)` 进程级注册（避免重复注册）
  - `open` 建 `CREATE VIRTUAL TABLE vec_items USING vec0(embedding float[dim])`
  - 单位归一化 + `vec0` 默认 L2 距离（unit 向量上 L2 与 cosine 单调一致 → KNN == task-18.2 brute-force cosine 真值）
  - `index_batch` 以 `id_map` Vec 下标作 rowid 直插（embedding 序列化为 JSON 文本）；`chunk_id`(String) ↔ rowid(i64) 经 `id_map` 互映
  - `delete` 清空（全量 reindex 语义，承 task-18.1）；`Mutex<Connection>` 内部可变（trait 全 `&self`，`Mutex<Connection>` 为 Send+Sync）
- **修改 `core/Cargo.toml`** — `sqlite-vec = { version = "0.1.9", optional = true }` + `vector-sqlite = ["dep:sqlite-vec"]`（默认不启用）。
- **修改 `core/src/retriever/vector/mod.rs`** — `#[cfg(feature = "vector-sqlite")] pub mod sqlite_vec; pub use sqlite_vec::SqliteVecBackend;`。
- **修改 `bench/Cargo.toml`** — `vector-sqlite = ["contextforge-core/vector-sqlite"]`。
- **修改 `bench/src/backends.rs`** — `#[cfg(feature = "vector-sqlite")] "sqlite-vec"` 注册表分支 + `known_backends`。
- **重写 `docs/spikes/phase-18-sqlite-vec.md`** — Linux 真实 5 维测量 evidence + Windows MSVC 受阻附注。
- **修改 `scripts/spike_vector_backends.sh`** — `BACKENDS` 加 `sqlite-vec`（注释引导）。
- **修改 `docs/s2v-adapter.md`** — Phase 18 表 18.3 行 Deferred → Done。

### Out of Scope（[SPEC-DEFER] / [SPEC-OWNER]）

- **Windows MSVC 下的 sqlite-vec** [SPEC-DEFER:phase-future.sqlite-vec-cross-platform]：MSVC 构建受阻，Linux 为 spike 平台，承 task-18.6 凭据。
- **qdrant backend** [SPEC-OWNER:task-18.4-spike-qdrant-embedded]：需运行 server，不在本 task。
- **lancedb backend** [SPEC-OWNER:task-18.5-spike-lancedb]：Arrow/Lance 重型依赖，不在本 task。
- **默认 backend 选型 + ADR-023** [SPEC-OWNER:task-18.7-decision-adr023]：4 路数据齐后决策。
- **`vec0` 落盘持久化** [SPEC-DEFER:phase-future.sqlite-vec-on-disk]：spike 用内存库测 cold-start/reindex，落盘后置。
- **embedding 二进制 blob 编码** [SPEC-DEFER:phase-future.sqlite-vec-blob-encoding]：spike 用 JSON 文本喂 `vec0`，紧凑 float blob 后置优化。
- **非 Linux RSS 采样** [SPEC-DEFER:phase-future.rss-sampling-macos-windows]：承 task-18.2 R3。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`SqliteVecBackend`**：core vector 模块新成员，cfg-gated。
- **bench 注册表**：`run_named("sqlite-vec", ...)` 派发。
- **下游 task-18.7**：消费本 task 的 sqlite-vec 5 维 evidence 做选型。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/tasks/task-18.1-vector-trait.md`（trait 签名）+ `docs/specs/tasks/task-18.2-spike-harness.md`（harness API）+ `docs/specs/tasks/task-18.6-spike-hnsw.md`（先行 backend 模式 + 受阻凭据）
- `core/src/retriever/vector/{traits,types}.rs`（实施对照）+ `core/src/retriever/vector/hnsw.rs`（同构实现参考）
- `docs/decisions/adr-008-core-library-selection.md`（sqlite-vec 入库 amendment 依据）
- `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5）

### 5.2 Imports（sqlite-vec 新增 optional dep；默认 0 新 dep）

```rust
use std::sync::{Mutex, Once};
use rusqlite::{ffi::sqlite3_auto_extension, Connection};
use sqlite_vec::sqlite3_vec_init;
use crate::retriever::vector::traits::{VectorBackend, VectorIndexer, VectorSearcher};
use crate::retriever::vector::types::{ChunkId, VectorChunk, VectorError, VectorFilter, VectorHit, VectorIndexConfig, VectorScore};
```

### 5.3 关键设计

- 单位归一化（`normalize`）索引/查询向量；`vec0` 默认 L2，unit 向量上与 cosine 单调一致 → KNN == task-18.2 brute-force cosine 真值。
- `SqliteVecBackend { conn: Mutex<Connection>, id_map: Mutex<Vec<String>>, dim: Mutex<usize> }`；`Once` 守护扩展注册。
- `open`：`DROP TABLE IF EXISTS` + `CREATE VIRTUAL TABLE vec_items USING vec0(embedding float[dim])`；清 `id_map`、记 `dim`。
- `index_batch`：rowid = `id_map.len()`，`INSERT INTO vec_items(rowid, embedding) VALUES (?, ?)`（embedding = 归一化后 `serde_json::to_string`）；`id_map.push(chunk_id)`；维度不符返 `VectorError::DimMismatch`。
- `search`：`SELECT rowid, distance FROM vec_items WHERE embedding MATCH ? ORDER BY distance LIMIT ?`；rowid → chunk_id；score = `1 - distance/2`（L2∈[0,2] → sim∈[0,1]）。
- `flush` no-op（`vec0` 插入即可查）；`delete` `DELETE FROM vec_items` + 清 `id_map`（全量 reindex 语义）；`is_indexed` = `id_map` 非空。

## 6. Acceptance Criteria

- [x] **AC1**: `SqliteVecBackend` 实现 `VectorBackend`/`VectorIndexer`/`VectorSearcher` 三 trait；`cargo build -p contextforge-bench --features vector-sqlite` 在 Linux gcc 下 exit 0；默认构建（无 feature）不引入 sqlite-vec（cfg-gated）— verified by **TEST-18.3.1**（feature build PASS + 默认 `cargo build` 无 sqlite-vec 编译）
- [x] **AC2**: 真实召回 — `spike --backend sqlite-vec` 产出 `recall@5/10`（`vec0` KNN 对 task-18.2 brute-force cosine 真值）非伪造，记录于 `docs/spikes/phase-18-sqlite-vec.md` — verified by **TEST-18.3.2**（spike run JSON + evidence）
- [x] **AC3**: cosine 一致性 — 单位归一化 + L2 使 `vec0` KNN 匹配 cosine 真值（recall 高，非 0）— verified by **TEST-18.3.3**（spike recall@5 ≥ 0.9）
- [x] **AC4**: harness 端到端真 backend — runner 返完整 `MeasureReport`（P95 / RSS / cold-start / reindex 记录），无 panic — verified by **TEST-18.3.4**（release spike exit 0 + 5 维字段全填，Linux RSS 经 /proc 实采）
- [x] **AC5**: 既有不退化 — 默认 `cargo test --workspace` 全 PASS（vector-sqlite 默认不启用，gated path 不入默认编译）；`go test ./...` 全 PASS — verified by **TEST-18.3.5**（`cargo test --workspace` 0 failed）+ §10 实测
- [x] **AC6**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched master` PR 触及行 0 未标注命中 — verified by §10 记录的 D2 lint 实跑输出

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-18.3.1 | feature build PASS + 默认无 sqlite-vec | `cargo build -p contextforge-bench --features vector-sqlite` | Done |
| TEST-18.3.2 | sqlite-vec spike 真实 recall + evidence | `docs/spikes/phase-18-sqlite-vec.md` | Done |
| TEST-18.3.3 | cosine 一致性 recall@5 ≥ 0.9 | spike run | Done |
| TEST-18.3.4 | runner 真 backend 5 维全填无 panic（Linux RSS 实采） | release spike JSON | Done |
| TEST-18.3.5 | 默认 cargo test --workspace 0 failed | 全 workspace | Done |

## 8. Risks

- **R1（中）alpha 版打包损坏**：`0.1.10-alpha.4` 缺 `sqlite-vec-diskann.c` 源文件无法编译；本 task pin **稳定版 0.1.9**。
  - **缓解**：`= 0.1.9` 精确 pin；Cargo.lock 锁定；版本升级回归由后续 task 验证。
- **R2（中）合成种子向量 recall 偏理想**：承 task-18.6 R1；真实分布 recall 见 dogfood + Linux 大 n 跑批。
  - **缓解**：evidence 标注数据来源；task-18.7 横向对比在 Linux release 大 n 复跑。
- **R3（低）Windows MSVC 不可构建**：sqlite-vec C 扩展在 MSVC 受阻（task-18.6 凭据）；本 task Linux-only spike。
  - **缓解**：`vector-sqlite` feature 默认关闭，默认构建/CI（含 Windows dev 机）不受影响；跨平台落地后置。

## 9. Verification Plan

```bash
# Linux x86_64 (gcc)
cargo build -p contextforge-bench --features vector-sqlite
cargo run --release -q -p contextforge-bench --features vector-sqlite -- --backend sqlite-vec --n 5000 --dim 64 --seed 1 --m 500 --out docs/spikes/phase-18-sqlite-vec.md
cargo test --workspace        # 默认 feature，sqlite-vec gated 不入编译
go test ./...
bash scripts/spec_drift_lint.sh --touched master
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**：2026-05-30
- **改动文件**：`core/src/retriever/vector/sqlite_vec.rs`（新增）、`core/src/retriever/vector/mod.rs`（cfg-gated export）、`core/Cargo.toml`（sqlite-vec optional + vector-sqlite）、`bench/Cargo.toml`（vector-sqlite feature）、`bench/src/backends.rs`（sqlite-vec arm）、`docs/spikes/phase-18-sqlite-vec.md`（重写为实测）、`scripts/spike_vector_backends.sh`（BACKENDS）、`docs/s2v-adapter.md`（18.3 行 Done）
- **commit 列表**：见本 task PR（分支 `feat/task-18.3-spike-sqlite-vec`）；合入后以 merge commit 为准
- **§9 Verification 结果**：见 PR 描述与 `docs/spikes/phase-18-sqlite-vec.md`（Linux gcc 实测填充）
- **剩余风险 / 未做项**：`vec0` 落盘持久化 / blob 编码后置；Windows MSVC 跨平台见 [SPEC-DEFER]；qdrant(18.4)/lancedb(18.5) 见各自 [SPEC-OWNER]
- **下游 task 影响**：task-18.7（消费 sqlite-vec 5 维 evidence + 其余 backend 数据做 ADR-023 选型）
