# Task `29.3`: `lancedb-ann-index-tuning-and-backend-matrix — 在 LanceIndexTuning 参数契约层之上真实建 IVF_PQ/HNSW 索引并实测召回 + 产出多 backend 选择矩阵真实测量 → add-only Amendment 到 ADR-030 D3 / ADR-023 tiers`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 29 (live-vector-recall)
**Dependencies**: task-25.2（`LanceIndexTuning` / `LanceAnnIndex` 参数契约层 + `vector-lancedb` 可构建性 🟢 已确证）/ task-29.1（`select_vector_backend` 工厂 + `LanceDbBackend` 注入热路径）/ ADR-030（production-vector-backend D2 lancedb 可构建性 + D3 选择矩阵，本 task add-only Amendment）/ ADR-023（vector-backend-default D4 lancedb tier，本 task add-only Amendment）/ ADR-028（vector-persistence-strategy，lancedb 磁盘列存持久 seam）/ ADR-004（local-first 0-dep 基线，默认构建仍 0 vector dep）/ ADR-013（禁伪造召回/性能凭据红线）/ ADR-014 D1-D5（第二十次激活）

## 1. Background

task-25.2 把 lancedb 的 ANN 索引调参参数（IVF_PQ 的 `num_partitions` / `num_sub_vectors`、HNSW 的 `m` / `ef_construction`）+ compaction 触发口径收敛为可校验的 `LanceIndexTuning` / `LanceAnnIndex` 结构（`core/src/retriever/vector/lance_db.rs:33-108`），但 `validate()` 是参数契约层——纯函数校验参数范围（partitions>0 / sub_vectors 整除 dim / m>0 / ef>0 / 阈值>0 / metric 受支持），**不建真实索引**。`LanceDbBackend::search`（`lance_db.rs:270-332`）当前走 Lance 默认 flat KNN（`nearest_to` + `DistanceType::Cosine`，无显式 ANN 索引），真实 IVF_PQ/HNSW 建图 + 召回测量随 task-25.2 构建 stop-condition 一并诚实延后 `[SPEC-DEFER:phase-future.lancedb-index-tuning]`（ADR-030 §D2 + D3 + ADR-023 Phase 25 Amendment）。

同时 ADR-030 §D3 的「语料规模 × 部署形态 → 推荐 backend」选择矩阵（`docs/decisions/adr-030-production-vector-backend.md` D3 §:42-44 + Ratification :57-66）是据 Phase 18 合成语料 5 维证据 + 各 phase 推进结论产出的 add-only 指南，lancedb 档至今只有「可构建性 🟢 + 真实性能延后」的 caveat，无真实 in-process ANN 召回/延迟测量；qdrant 档无 live KNN 数；矩阵的真实测量校准仍是缺口。

本 task 在 task-25.2 已确证可构建的 `vector-lancedb` feature 之上，把参数契约层兑现为**真实 IVF_PQ/HNSW 索引建图 + in-process 召回测量**（兑现 `[SPEC-DEFER:phase-future.lancedb-index-tuning]`），再产出一张**多 backend（brute / sqlite-vec / lancedb / qdrant 可跑则跑、不可跑诚实延后）选择矩阵真实测量**，以 add-only Amendment 反哺 ADR-030 D3 + ADR-023 tiers（不溯改其 D 正文，ADR-014 D5）。

## 2. Goal

在 `vector-lancedb` feature 下（in-process，n 仍取适中规模）：(1) 用 `LanceIndexTuning` 真实创建一个 IVF_PQ 与一个 HNSW Lance 索引覆盖一个嵌入式 Lance 数据集，对比无索引 flat KNN 实测召回（recall@5/@10）+ 索引建图耗时 + 查询延迟，**真实跑出后回填**（ADR-013：禁伪造召回/性能数）；(2) 跨 backend（brute-force 默认可跑 / sqlite-vec feature 可跑 / lancedb 本 task 可跑 / qdrant 凭 task-29.2 live server 可达则跑、不可达诚实延后）产出「语料规模 × 部署形态 → 推荐 backend + 真实 caveat」选择矩阵真实测量，反哺 ADR-030 D3 + ADR-023 tier 的 add-only Amendment。

pass bar：IVF_PQ/HNSW 真实索引建图 + 召回测量经 `vector-lancedb` feature 真实跑出（值实施时回填）；多 backend 矩阵每档记真实测量或诚实延后理由；ADR-030 D3 / ADR-023 tier add-only Amendment（不溯改正文）；compaction 执行真跑或诚实延后；默认构建 0 vector dep / BruteForce 基线不退化（ADR-004）；D2 lint 0 未标注命中。

**feature 构建 caveat（承 task-25.2）**：`vector-lancedb` 广义 `cargo test --features vector-lancedb`（全 integration test target）受 rustc 1.95.0 ICE + rlib-format 链接限制（向量无关 target 的工具链项，非逻辑回归）；本 task 用 `cargo build --features vector-lancedb` + `--lib retriever::vector::lance_db` scoped 测试规避 `[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]`，CI 默认不构建该 feature。

## 3. Scope

### In Scope（计划交付）

- 在 `vector-lancedb` feature 下，于 `core/src/retriever/vector/lance_db.rs` 既有 `LanceDbBackend` 之上加一条**真实建 ANN 索引**的路径：消费 `LanceIndexTuning`（IVF_PQ / HNSW）经 Lance `create_index` API 在 Lance table 上建真实索引（区别于现 `search` 的 flat KNN，`lance_db.rs:270-332`），兑现 `[SPEC-DEFER:phase-future.lancedb-index-tuning]`。索引参数取自 `LanceIndexTuning`（已 `validate(dim)` 通过的参数）。
- 一个 in-process 召回测量驱动（克隆/仿 `core/examples/phase20_recall_via_retriever.rs` 或落 `--lib` 测试/example），guard 在 `vector-lancedb`（+ `embedding-fastembed` 取真实向量），对同一嵌入式 Lance 数据集分别测 flat KNN / IVF_PQ / HNSW 的 recall@5/@10 + 建图耗时 + 查询延迟，结果实施时回填 §10。
- 多 backend 选择矩阵真实测量：brute-force（默认 0-dep，always）/ sqlite-vec（`vector-sqlite` feature）/ lancedb（本 task）/ qdrant（task-29.2 live server 可达则测、不可达诚实延后），统一语料 + 统一召回口径产出「语料规模 × 部署形态 → 推荐 backend + 真实 caveat」表。
- ADR-030 `## Amendment` 段落更新本 task 的真实测量喂入 D3 矩阵（add-only，不溯改 D1-D4 正文）+ ADR-023 tier add-only Amendment 喂入 lancedb（D4）/ qdrant（D3）真实数（不溯改 D1-D6 正文，ADR-014 D5）。Amendment 正文于 task-29.4 closeout 据真实测量回填。
- compaction 执行：尝试在超 `compaction_threshold_rows` 行数后真实触发 Lance 数据集 compaction（`LanceIndexTuning::compaction_threshold_rows`，`lance_db.rs:52-53`）；若 toolchain/资源限不可真跑则诚实延后 `[SPEC-DEFER:phase-future.lancedb-schema-compaction]`。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- 大语料（n≫ 适中规模）lancedb 性能 baseline `[SPEC-DEFER:phase-future.lancedb-large-corpus-perf]`——in-process 适中 n 实测；大语料受 toolchain/资源限如实记录，不伪造。
- `vector-lancedb` 广义 feature 全 target CI 构建门 `[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]`——受 rustc 1.95.0 ICE 限，本 task 用 `cargo build` + `--lib` scoped 规避，CI 默认不构建该 feature。
- lancedb 数据集 compaction 真实执行（若不可真跑）`[SPEC-DEFER:phase-future.lancedb-schema-compaction]`（承 ADR-030 D2 + `docs/spikes/phase-18-lancedb.md` Follow-up）。
- qdrant live KNN 真实召回 [SPEC-OWNER:task-29.2]（本 task 矩阵只消费 task-29.2 已跑出的 qdrant 数；qdrant live server 召回归 task-29.2）。
- ADR-030 / ADR-023 D 正文修改——本 task 只 add-only Amendment（ADR-014 D5）。
- 真实 tag / release [SPEC-OWNER:task-29.4]（outward-facing，归 closeout）。

## 4. Actors

- 主 agent（ADR-012 自治）
- `core/src/retriever/vector/lance_db.rs`（`LanceDbBackend` + `LanceIndexTuning` 真实建索引路径 + 召回测量）
- `LanceIndexTuning` / `LanceAnnIndex`（IVF_PQ / HNSW 参数契约层，`lance_db.rs:33-108`）
- in-process 召回测量驱动（仿 `core/examples/phase20_recall_via_retriever.rs`；`vector-lancedb` + `embedding-fastembed` guard）
- 多 backend 选择矩阵（brute / sqlite-vec / lancedb / qdrant，消费 task-29.2 qdrant 数）
- `docs/decisions/adr-030-production-vector-backend.md`（D3 矩阵 add-only Amendment 宿主）+ `docs/decisions/adr-023-vector-backend-default.md`（tier add-only Amendment 宿主）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/retriever/vector/lance_db.rs:33-45`（`LanceAnnIndex`：IVF_PQ `num_partitions`/`num_sub_vectors` + HNSW `m`/`ef_construction`）+ `:48-54`（`LanceIndexTuning`：index + metric + `compaction_threshold_rows`）+ `:59-108`（`validate(dim)` 参数范围校验纯函数——partitions>0 / sub_vectors 整除 dim / m>0 / ef>0 / 阈值>0 / metric 受支持）
- `core/src/retriever/vector/lance_db.rs:270-332`（`VectorSearcher::search` 现 flat KNN 读路径——`nearest_to` + `DistanceType::Cosine` + cosine distance→similarity 映射；本 task 在其之上加真实 ANN 索引建图）
- `core/src/retriever/vector/traits.rs:38-46`（`VectorSearcher::search` live-KNN 契约面）+ `:11-25`（`VectorBackend` / `VectorIndexer`）
- `docs/decisions/adr-030-production-vector-backend.md` D3 §:42-44（选择矩阵口径——dev/小语料→hnsw / 单机嵌入式→sqlite-vec / 大语料列存→lancedb / hosted→qdrant）+ Ratification §:57-66（task-25.2 D2 lancedb 可构建性 🟢 + `[SPEC-DEFER:phase-future.lancedb-index-tuning]` 真实性能延后由本 task 兑现）
- `docs/decisions/adr-023-vector-backend-default.md` §44-89（D1-D6 tier 排序）+ §149-156（Phase 25 / v0.18.0 add-only Amendment：D4 lancedb tier 可构建性确证，本 task 续加真实召回）
- `core/examples/phase20_recall_via_retriever.rs`（production pipeline 召回测量驱动——克隆为本 task 多 backend 矩阵驱动的基线）+ sibling `core/examples/phase19_real_recall.rs`
- `core/Cargo.toml:120`（`vector-lancedb` feature）+ `:119`（`vector-qdrant`）+ `:123`（`embedding-fastembed`）

### 5.2 关键设计 — 真实 ANN 索引建图 + 多 backend 矩阵真实测量（不伪造召回/性能）

参数契约层（`LanceIndexTuning::validate`）已确定性可单测（task-25.2，`--lib retriever::vector::lance_db` 2/2 PASS）；本 task 把它兑现为真实索引：

- **真实建索引**：在 `LanceDbBackend` 上加路径，把 `validate(dim)` 通过的 `LanceIndexTuning` 经 Lance `create_index` 在 table 上建真实 IVF_PQ / HNSW 索引（非现 `search` 的 flat KNN）。索引建成后 `search` 走 ANN 路径，对比 flat 的 recall/latency 差。
- **召回测量真实跑出**：用 `embedding-fastembed` 真实向量（仿 `phase20_recall_via_retriever.rs` 的 production pipeline 口径，离线从 `.fastembed_cache` 服务）在适中 n 上测 flat / IVF_PQ / HNSW 的 recall@5/@10 + 建图耗时 + 查询延迟，**真实跑出后回填 §10 + v0.22.0 evidence**，绝不预填（ADR-013）。
- **多 backend 矩阵真实测量**：统一语料 + 统一召回口径，brute-force（默认）/ sqlite-vec（`vector-sqlite`）/ lancedb（本 task）/ qdrant（消费 task-29.2 已跑出的 live 数；task-29.2 honest-defer 则矩阵 qdrant 档诚实延后）跑出真实 recall + latency + caveat（feature 构建前置 / live-server 依赖 / 平台限制），喂入 ADR-030 D3 + ADR-023 tier 的 add-only Amendment。
- **add-only 校准（ADR-014 D5）**：真实测量只以 ADR-030 `## Amendment` + ADR-023 tier Amendment 段记录，**不溯改** ADR-030 D1-D4 / ADR-023 D1-D6 正文（task-25.2 已用此模式记 D2/D3，本 task 续加真实召回/延迟）。

**feature 构建 caveat（承 task-25.2 真实凭据）**：`vector-lancedb` 在 `x86_64-pc-windows-msvc`（rustc 1.95.0，protoc 经仓内 `protoc-bin-vendored` 经 `PROTOC` env）`cargo build` exit 0 已确证；但广义 `cargo test --features vector-lancedb`（全 integration test target）受 rustc 1.95.0 ICE + rlib-format 链接限制——本 task 用 `cargo build --features vector-lancedb` + `--lib retriever::vector::lance_db` scoped 测试 + scoped example 规避，CI 默认不构建该 feature `[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]`。

**stop-condition**：若真实索引建图在本 toolchain 受阻（ICE / 链接 / Lance API 不可用），如实记录受阻维度 + 据已达维度处理，不伪造索引建成 / 召回数（ADR-013）。

### 5.3 不变量

- 默认构建 0 vector dep / BruteForce 语义基线不退化（ADR-004 + ADR-023 D5）；lancedb 真实建索引 + 召回测量全在 `vector-lancedb`（+ `embedding-fastembed`）feature 下，默认 `cargo test --workspace` 不触及。
- 不改 task-18.1 三 trait（`VectorBackend` / `VectorIndexer` / `VectorSearcher`）签名（`traits.rs:11-46`）。
- `LanceIndexTuning::validate` 既有参数契约层语义不变（`lance_db.rs:59-108`），本 task 在其之上加真实建索引，不改其纯函数校验。
- 召回 / 性能数 100% 真实跑出后回填，0 预填 / 0 合成 / 0 伪造（ADR-013）。
- ADR-030 / ADR-023 D 正文不溯改，只 add-only Amendment（ADR-014 D5）。
- 0 新 direct dep（lancedb / arrow-array / futures 自 task-18.5 即 optional；若真实建索引须新增 crate 面则经 R7 chore + ADR-008 add-only 记录）。

## 6. Acceptance Criteria

- [ ] AC1（🟡 真实 ANN 索引建图 + 召回）: 在 `vector-lancedb` feature 下用 `LanceIndexTuning`（IVF_PQ 与 HNSW）经 Lance `create_index` 真实建索引覆盖嵌入式 Lance 数据集，对比 flat KNN 实测 recall@5/@10 + 建图耗时 + 查询延迟，真实跑出后回填 §10（兑现 `[SPEC-DEFER:phase-future.lancedb-index-tuning]`；ADR-013 禁预填） — verified by TEST-29.3.1
- [ ] AC2（🟡 多 backend 选择矩阵真实测量 → add-only Amendment）: brute / sqlite-vec / lancedb / qdrant（task-29.2 可达则测、不可达诚实延后）统一语料真实测量「语料规模 × 部署形态 → 推荐 backend + caveat」矩阵，反哺 ADR-030 D3 + ADR-023 tier add-only Amendment（不溯改其 D 正文，ADR-014 D5） — verified by TEST-29.3.2
- [ ] AC3（🔴 compaction 真跑或诚实延后）: 超 `compaction_threshold_rows` 后尝试真实触发 Lance 数据集 compaction；toolchain/资源限不可真跑则如实记录受阻 `[SPEC-DEFER:phase-future.lancedb-schema-compaction]`（ADR-013 不伪造） — verified by TEST-29.3.3
- [ ] AC4（ADR-014 D2 lint）: bash scripts/spec_drift_lint.sh --touched origin/master PR 触及行 0 未标注命中 — verified by TEST-29.3.4

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-29.3.1 | `vector-lancedb` feature 下 `LanceIndexTuning`（IVF_PQ + HNSW）经 Lance `create_index` 真实建索引 + flat 对比召回/建图耗时/查询延迟（`cargo build` + `--lib` scoped；🟡 feature build；值真实跑出回填） | `core/src/retriever/vector/lance_db.rs` + in-process 召回驱动 | Planned |
| TEST-29.3.2 | 多 backend（brute/sqlite-vec/lancedb/qdrant）选择矩阵真实测量 → ADR-030 D3 + ADR-023 tier add-only Amendment（🟡 feature build；qdrant 凭 task-29.2，不可达诚实延后） | `docs/decisions/adr-030-production-vector-backend.md` + `docs/decisions/adr-023-vector-backend-default.md` | Planned |
| TEST-29.3.3 | lancedb 数据集 compaction 真实执行或诚实延后 `[SPEC-DEFER:phase-future.lancedb-schema-compaction]`（🔴 toolchain/资源限） | `core/src/retriever/vector/lance_db.rs` | Planned |
| TEST-29.3.4 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威） | `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（高）rustc 1.95.0 ICE 限制广义 `vector-lancedb` 测试** — ⚠️ 承 task-25.2 已知：`cargo test --features vector-lancedb`（全 integration test target）受 rustc 1.95.0 ICE + rlib-format 链接限制。
  - **缓解**：用 `cargo build --features vector-lancedb` + `--lib retriever::vector::lance_db` scoped 测试 + scoped example 规避（task-25.2 已证此路可跑）；CI 默认不构建该 feature `[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]`。stop-condition：若真实建索引在 `--lib`/example 路径仍 ICE，AC1 不标 `[x]`，如实记录受阻（ADR-013）。
- **R2（中）真实 ANN 召回在适中语料可能非区分性** — 承 ADR-023 Context：Phase 18 合成语料 recall 全 1.0 非区分。
  - **缓解**：用 `embedding-fastembed` 真实向量（仿 Phase 19/20 dogfood 口径）取区分性语料；IVF_PQ/HNSW 是有损 ANN，相对 flat 的 recall 折损本身即区分信号。真实跑出后如实记录（含「适中 n 下 recall 折损不显著」这类诚实结论）。
- **R3（中）qdrant 矩阵档依赖 task-29.2 live server** — 矩阵 qdrant 列凭 task-29.2 跑出的 live 数。
  - **缓解**：task-29.2 honest-defer（无 server）时本 task 矩阵 qdrant 档诚实延后（标 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` / [SPEC-OWNER:task-29.2]），brute/sqlite-vec/lancedb 三档真实测量仍成立，不因 qdrant 缺失阻塞矩阵主体。
- **R4（中）lancedb compaction 在本 toolchain 不可真跑** — compaction 执行属构建通过后集成验证。
  - **缓解**：尝试真跑；不可真跑则诚实延后 `[SPEC-DEFER:phase-future.lancedb-schema-compaction]`（AC3 据 🔴 维度处理，ADR-013 不伪造）。
- **R5（低）真实建索引引入新 crate 面** — Lance `create_index` 可能拉新 optional dep。
  - **缓解**：优先复用 lancedb 0.30 既有面；若须新增则经主 agent R7 chore + ADR-008 add-only 记依赖变更（Cargo.lock 变更如实记录）。

## 9. Verification Plan

```bash
# 0. 默认构建不退化（ADR-004 / ADR-023 D5）——不带任何 vector feature
cargo test --workspace
go test ./...

# 1. AC1 — vector-lancedb 真实建索引 + 召回（cargo build + --lib scoped，规避 rustc 1.95.0 ICE）
#    PROTOC 指向仓内 protoc-bin-vendored 的 protoc.exe（承 task-25.2）
cargo build --features vector-lancedb -p contextforge-core
cargo test --features vector-lancedb -p contextforge-core --lib retriever::vector::lance_db
#    in-process 召回驱动（vector-lancedb + embedding-fastembed，离线 .fastembed_cache）：
#    flat / IVF_PQ / HNSW recall@5/@10 + 建图耗时 + 查询延迟 → 真实跑出回填 §10（禁预填，ADR-013）

# 2. AC2 — 多 backend 选择矩阵真实测量（brute / sqlite-vec / lancedb / qdrant）
#    brute（默认可跑）+ sqlite-vec（--features vector-sqlite）+ lancedb（步1）+ qdrant（消费 task-29.2 live 数，不可达诚实延后）
#    统一语料 + 统一召回口径 → ADR-030 D3 + ADR-023 tier add-only Amendment（不溯改 D 正文，ADR-014 D5）

# 3. AC3 — lancedb compaction 真跑或诚实延后
#    超 compaction_threshold_rows 后尝试真实 compaction；不可真跑 → [SPEC-DEFER:phase-future.lancedb-schema-compaction]

# 4. AC4 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **outward-facing 红线**：本 task 无 outward-facing 不可逆动作（in-process 索引 + 文档 Amendment）；真实 tag / release 归 task-29.4（[SPEC-OWNER:task-29.4]，ADR-012 用户授权）。lancedb feature 真实建索引召回数 100% 真实跑出后回填，CI 默认不构建该 feature（`[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]`，ADR-013 禁伪造）。

## 10. Completion Notes (s2v 6 项标准)

- **Status**: Draft（待实施）
- **计划改动文件**：
  - `core/src/retriever/vector/lance_db.rs`——在 `LanceDbBackend` 之上加真实 ANN 索引建图路径（消费 `LanceIndexTuning` 经 Lance `create_index` 建 IVF_PQ / HNSW，区别于现 flat KNN `:270-332`）+ compaction 触发尝试。
  - in-process 召回测量驱动（仿 `core/examples/phase20_recall_via_retriever.rs`；`vector-lancedb` + `embedding-fastembed` guard）——flat / IVF_PQ / HNSW recall@5/@10 + 建图耗时 + 查询延迟 + 多 backend 矩阵真实测量。
  - `docs/decisions/adr-030-production-vector-backend.md`——`## Amendment` 段加 D3 矩阵真实测量（add-only，不溯改 D1-D4）。
  - `docs/decisions/adr-023-vector-backend-default.md`——tier add-only Amendment 加 lancedb（D4）/ qdrant（D3）真实召回/延迟（不溯改 D1-D6，ADR-014 D5）。
- **§9 Verification 计划** (will record real evidence at impl)：
  - AC1：`cargo build --features vector-lancedb` exit 0 + `--lib retriever::vector::lance_db` PASS + 真实 IVF_PQ/HNSW vs flat recall@5/@10 + 建图耗时 + 查询延迟（真实跑出后回填，禁预填）。
  - AC2：brute / sqlite-vec / lancedb / qdrant（可达则测）多 backend 矩阵真实测量值 + 每档 caveat → ADR-030 D3 / ADR-023 tier add-only Amendment（真实跑出后回填）。
  - AC3：compaction 真跑结果或诚实延后理由（toolchain/资源受阻维度如实，ADR-013）。
  - AC4：D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）。
  - 默认构建：`cargo test --workspace` + `go test ./...` 不退化（ADR-004 / ADR-023 D5）。
- **设计取舍 / 受阻维度**（实施时如实回填，不伪造）：`vector-lancedb` 广义全 target 测试受 rustc 1.95.0 ICE 限 → `cargo build` + `--lib` scoped 规避（`[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]`）；适中 n 召回区分性 / 大语料性能受 toolchain/资源限如实记录（`[SPEC-DEFER:phase-future.lancedb-large-corpus-perf]`）；compaction 不可真跑则 `[SPEC-DEFER:phase-future.lancedb-schema-compaction]`；qdrant 矩阵档凭 task-29.2 live 数，honest-defer 时如实记录。
