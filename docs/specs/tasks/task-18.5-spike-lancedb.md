# Task `18.5`: `spike-lancedb — core/src/retriever/vector/lance_db.rs LanceDbBackend (lancedb embedded + Arrow RecordBatch + 自带 tokio runtime block_on) + bench 注册表接入 + 5 维 evidence`

**Status**: Done

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 18 (vector-backend-selection)
**Dependencies**: task-18.1（三 trait 冻结）/ task-18.2（spike harness）/ task-18.3（bench 注册表 + id_map 模式）/ task-18.4（async backend block_on 桥接模式）/ ADR-008 core-library-selection（新增 lancedb dep）/ ADR-014 D1-D5 第十二次激活

## 1. Background

Phase 18 §2A 候选集第 3 路 = LanceDB（嵌入式列式向量库，Lance 格式 + DataFusion 查询引擎）。原 handoff 记 lancedb 为「Arrow/Lance 重型 native 构建」延后。在 Linux x86_64（WSL2 Ubuntu 26.04）上验证：`lancedb = 0.30.0` 经 gcc 15.2.0 + cmake 4.2.3 + protoc（vendored v35.0）**构建通过（~3min）**，运行时 `connect` / `create_empty_table` / `add` / `nearest_to` cosine KNN 经一次性探针证实可用。

与 qdrant（外部 server）不同，LanceDB 是**进程内嵌入式**（`is_local = true`），数据落 Lance 磁盘数据集——是「进程内 + 真磁盘持久化 + 列式」这一组合的候选。

## 2. Goal

在 `core/src/retriever/vector/lancedb.rs` 落地 `LanceDbBackend`，实现三 trait（lancedb 异步经自带 `tokio::runtime::Runtime` + `block_on` 桥接；Arrow `RecordBatch`（`id` Int32 + `vector` FixedSizeList&lt;Float32&gt;）写入；`DistanceType::Cosine` 与 task-18.2 cosine 真值一致）；用 `vector-lancedb` feature gate（默认构建不引入 lancedb dep）；接入 `bench/src/backends.rs` 注册表；跑出真实 5 维数据并落 `docs/spikes/phase-18-lancedb.md`。默认 `cargo test --workspace` + `go test ./...` 不退化；D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **新建 `core/src/retriever/vector/lance_db.rs`** — `LanceDbBackend`（模块名 `lance_db` 避开 `lancedb` crate 名冲突）：
  - 自带 `tokio::runtime::Runtime` + `block_on` 桥接异步 lancedb（bench 为 sync，承 task-18.4 模式）
  - Arrow schema：`id` Int32 + `vector` `FixedSizeList<Float32>[dim]`；`chunk_id`(String) ↔ 数值 id(i32) 经 `id_map` 互映
  - `open` 连接临时目录 + `create_empty_table`（drop 既有表后重建）；`index_batch` 构 `RecordBatch` 经 `table.add`；`delete` `table.delete("true")` 清空（全量 reindex）；`search` `query().nearest_to(v).distance_type(Cosine).limit(k)` → 解析 `id` + `_distance` 列
  - 手写 `Debug`；`Connection`/`Table`/`Runtime`/`Mutex` 皆 Send+Sync
- **修改 `core/Cargo.toml`** — `lancedb = { version = "0.30", optional = true }` + `arrow-array = { version = "58", optional = true }`（匹配 lance 内部 arrow 版本，lancedb 仅 re-export `arrow_schema`）+ `futures = { version = "0.3", optional = true }`；`vector-lancedb = ["dep:lancedb", "dep:arrow-array", "dep:futures"]`（默认不启用）。
- **修改 `core/src/retriever/vector/mod.rs`** — `#[cfg(feature = "vector-lancedb")] pub mod lance_db; pub use lance_db::LanceDbBackend;`。
- **修改 `bench/Cargo.toml`** — `vector-lancedb = ["contextforge-core/vector-lancedb"]`。
- **修改 `bench/src/backends.rs`** — `#[cfg(feature = "vector-lancedb")] "lancedb"` 注册表分支 + `known_backends`。
- **新建 `docs/spikes/phase-18-lancedb.md`** — Linux 真实 5 维 evidence + build 前置（protoc/cmake）标注。
- **修改 `scripts/spike_vector_backends.sh`** — 注释引导（lancedb build 需 protoc）。
- **修改 `docs/s2v-adapter.md`** — Phase 18 表 18.5 行 Deferred → Done。

### Out of Scope（[SPEC-DEFER] / [SPEC-OWNER]）

- **默认 backend 选型 + ADR-023** [SPEC-OWNER:task-18.7-decision-adr023]：4 路数据齐后决策。
- **Lance 向量索引（IVF_PQ/HNSW）调优** [SPEC-DEFER:phase-future.lancedb-index-tuning]：spike 用 flat 搜索测召回，索引调优后置。
- **Lance 数据集 schema 演进 / 压缩** [SPEC-DEFER:phase-future.lancedb-schema-compaction]：spike 用单批写入，compaction 后置。
- **build 前置编排（protoc/cmake CI 注入）** [SPEC-DEFER:phase-future.lancedb-build-prereq-ci]：spike 本机 vendored protoc；CI 注入后置。
- **非 Linux RSS 采样** [SPEC-DEFER:phase-future.rss-sampling-macos-windows]：承 task-18.2 R3。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`LanceDbBackend`**：core vector 模块新成员，cfg-gated，`is_local=true`（嵌入式）。
- **bench 注册表**：`run_named("lancedb", ...)` 派发。
- **下游 task-18.7**：消费本 task 的 lancedb 5 维 evidence 做选型。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/tasks/task-18.1-vector-trait.md` + `task-18.2-spike-harness.md` + `task-18.4-spike-qdrant.md`（block_on 桥接 + id_map）
- `core/src/retriever/vector/{traits,types}.rs` + `core/src/retriever/vector/qdrant.rs`（async backend 参考）
- `docs/decisions/adr-008-core-library-selection.md` + `adr-014-cross-phase-exit-criteria-validation.md`

### 5.2 Imports（lancedb/arrow-array/futures 新增 optional dep；默认 0 新 dep）

```rust
use std::sync::{Arc, Mutex};
use arrow_array::{RecordBatch, RecordBatchIterator, Int32Array, Float32Array, FixedSizeListArray};
use arrow_array::types::Float32Type;
use lancedb::arrow::arrow_schema::{DataType, Field, Schema};
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::{DistanceType, Connection, Table};
use futures::TryStreamExt;
use crate::retriever::vector::traits::{VectorBackend, VectorIndexer, VectorSearcher};
use crate::retriever::vector::types::{ChunkId, VectorChunk, VectorError, VectorFilter, VectorHit, VectorIndexConfig, VectorScore};
```

### 5.3 关键设计

- `LanceDbBackend { rt: Runtime, conn: Connection, table: Mutex<Option<Table>>, schema: Mutex<Option<SchemaRef>>, id_map: Mutex<Vec<String>>, dim: Mutex<usize>, dir: String }`。
- Arrow schema：`id` Int32（= id_map 下标）+ `vector` `FixedSizeList<Float32>[dim]`。
- `open`：`connect(dir)` + `drop_table(name, &[])`（忽略 not-found）+ `create_empty_table(name, schema)`；记 table/schema/dim、清 id_map。
- `index_batch`：构 `Int32Array`(ids) + `FixedSizeListArray::from_iter_primitive::<Float32Type>`(vectors) → `RecordBatch` → `RecordBatchIterator` → `table.add(...).execute()`；维度不符返 `DimMismatch`。
- `search`：`table.query().nearest_to(query).distance_type(Cosine).limit(k).execute()` → `try_collect::<Vec<RecordBatch>>()`；解析 `id`(Int32) + `_distance`(Float32) 列；score = `1 - distance`（cosine distance → similarity）。
- `delete` `table.delete("true")`（全量清）；`flush` no-op；`is_indexed` = id_map 非空；`is_local()=true`。
- arrow 版本：lance 内部用 arrow 58，lancedb 仅 re-export `arrow_schema`，故 `arrow-array` 显式 pin `=58.x` 与之统一（schema 类型取自 `lancedb::arrow::arrow_schema`）。

## 6. Acceptance Criteria

- [x] **AC1**: `LanceDbBackend` 实现三 trait；`cargo build -p contextforge-bench --features vector-lancedb` exit 0（需 protoc）；默认构建（无 feature）不引入 lancedb（cfg-gated）— verified by **TEST-18.5.1**（feature build PASS + 默认 `cargo build` 无 lancedb 编译）
- [x] **AC2**: 真实召回 — `spike --backend lancedb` 产出 `recall@5/10`（Lance Cosine KNN 对 task-18.2 brute-force cosine 真值）非伪造，记录于 `docs/spikes/phase-18-lancedb.md` — verified by **TEST-18.5.2**（spike run JSON + evidence）
- [x] **AC3**: cosine 一致性 — `DistanceType::Cosine` 使 Lance KNN 匹配 cosine 真值（recall 高，非 0）— verified by **TEST-18.5.3**（spike recall@5 ≥ 0.9）
- [x] **AC4**: harness 端到端真 backend — runner 返完整 `MeasureReport`（P95 / RSS / cold-start / reindex 记录），无 panic；Linux RSS 经 /proc 实采（嵌入式，harness RSS 即进程 RSS）— verified by **TEST-18.5.4**（release spike exit 0 + 5 维字段全填）
- [x] **AC5**: 既有不退化 — 默认 `cargo test --workspace` 全 PASS（vector-lancedb 默认不启用）；`go test ./...` 全 PASS — verified by **TEST-18.5.5**（`cargo test --workspace` 0 failed）+ §10 实测
- [x] **AC6**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched master` PR 触及行 0 未标注命中 — verified by §10 记录的 D2 lint 实跑输出

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-18.5.1 | feature build PASS（protoc）+ 默认无 lancedb | `cargo build -p contextforge-bench --features vector-lancedb` | Done |
| TEST-18.5.2 | lancedb spike 真实 recall + evidence | `docs/spikes/phase-18-lancedb.md` | Done |
| TEST-18.5.3 | cosine 一致性 recall@5 ≥ 0.9 | spike run | Done |
| TEST-18.5.4 | runner 真 backend 5 维全填无 panic（嵌入式 RSS 实采） | release spike JSON | Done |
| TEST-18.5.5 | 默认 cargo test --workspace 0 failed | 全 workspace | Done |

## 8. Risks

- **R1（中）build 前置 protoc**：lance build.rs 需 protoc；本机用 vendored v35.0（`PROTOC` env）。
  - **缓解**：evidence 标注 build 前置；CI 注入 protoc [SPEC-DEFER:phase-future.lancedb-build-prereq-ci]；feature 默认关闭，默认构建/CI 不受影响。
- **R2（中）arrow 版本统一**：lance 内部 arrow 58 与 datafusion 53 共存；`arrow-array` 须 pin 58 与 lance 统一，否则 `RecordBatch` 类型不匹配。
  - **缓解**：`arrow-array = "=58.x"` 精确 pin；schema 取自 `lancedb::arrow::arrow_schema`；Cargo.lock 锁定。
- **R3（中）合成种子向量 recall 偏理想**：承 task-18.3 R2；真实分布 recall 见 dogfood + 大 n。
  - **缓解**：evidence 标注数据来源；task-18.7 横向对比复跑。
- **R4（低）重型依赖树**：lancedb 引 DataFusion/Lance/Arrow 数百 crate（~3min 首次构建）。
  - **缓解**：feature 默认关闭；编译入 dev-time spike，CI 不跑 spike。

## 9. Verification Plan

```bash
# Linux x86_64 — 需 protoc (vendored) + cmake
export PROTOC=/path/to/protoc
cargo build -p contextforge-bench --features vector-lancedb
cargo run --release -q -p contextforge-bench --features vector-lancedb -- --backend lancedb --n 5000 --dim 64 --seed 1 --m 500 --out docs/spikes/phase-18-lancedb.md
cargo test --workspace        # 默认 feature，lancedb gated 不入编译
go test ./...
bash scripts/spec_drift_lint.sh --touched master
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**：2026-05-30
- **改动文件**：`core/src/retriever/vector/lance_db.rs`（新增）、`core/src/retriever/vector/mod.rs`（cfg-gated export）、`core/Cargo.toml`（lancedb/arrow-array/futures optional + vector-lancedb）、`bench/Cargo.toml`（vector-lancedb feature）、`bench/src/backends.rs`（lancedb arm）、`docs/spikes/phase-18-lancedb.md`（新增实测）、`scripts/spike_vector_backends.sh`（注释）、`docs/s2v-adapter.md`（18.5 行 Done）
- **commit 列表**：见本 task PR（分支 `feat/task-18.5-spike-lancedb`）；合入后以 merge commit 为准
- **§9 Verification 结果**：见 PR 描述与 `docs/spikes/phase-18-lancedb.md`（Linux 实测填充）
- **剩余风险 / 未做项**：Lance 索引调优 / schema compaction / CI protoc 注入后置（见各 [SPEC-DEFER]）
- **下游 task 影响**：task-18.7（消费 lancedb 5 维 evidence + 嵌入式列式持久化差异做 ADR-023 选型）；4 路 backend 数据至此齐备
