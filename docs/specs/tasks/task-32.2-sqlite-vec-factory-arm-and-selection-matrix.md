# Task `32.2`: `sqlite-vec-factory-arm-and-selection-matrix — factory 加 "sqlite-vec" arm 镜像 qdrant/lancedb（feat vector-sqlite on→SqliteVecBackend / off→honest Err naming vector-sqlite）+ in-process 选择矩阵 wiring 🟢；矩阵 recall/latency CELL 须本机 MSVC feature build → 🟡 honest-defer`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 32 (vector-backend-config-plumbing-and-completeness)
**Dependencies**: 既有 `core/src/retriever/vector/factory.rs`（task-29.1 `select_vector_backend`，Phase 29 已交付）/ `core/src/retriever/vector/sqlite_vec.rs`（task-18.3 spike + task-23.2 Windows MSVC 真实构建通过，`SqliteVecBackend`）/ `core/src/retriever/vector/mod.rs:40`（`#[cfg(feature = "vector-sqlite")] pub use sqlite_vec::SqliteVecBackend`）/ `core/Cargo.toml:82,127`（`sqlite-vec = "=0.1.9"` optional + `vector-sqlite = ["dep:sqlite-vec"]`）/ ADR-037（vector-backend-config-plumbing-and-completeness §D2，本 task 即其原文实现）/ ADR-034（production-vector-live-recall，sqlite-vec arm 补全 factory 后端覆盖，add-only Amendment 落点 @ task-32.4）/ ADR-023（vector-backend，0-dep baseline 守线）/ ADR-004（local-first-privacy-baseline，默认 0-vector-dep / 默认行为不变）/ ADR-008（dep add-only，本 task 加 0 新 dep——`sqlite-vec` 既已 optional 在树）/ ADR-013（禁伪造红线——矩阵 recall/latency CELL 须本机 MSVC feature build 真实跑出才记数，不预填）/ ADR-014 D1-D5（第二十三次激活）

## 1. Background

`select_vector_backend(name, dim)`（`factory.rs:31-69`）今天有四条 arm：`""`/`"brute"` → `BruteForceVectorBackend`（0-dep 默认）、`"qdrant"`（feat `vector-qdrant`）、`"lancedb"`（feat `vector-lancedb`）、其余 → honest `VectorError`。但 `SqliteVecBackend`（`sqlite_vec.rs:43`，feat `vector-sqlite`，`name()="sqlite-vec"`）虽已：

- 实现 `VectorBackend + VectorIndexer + VectorSearcher` → 即 `VectorStore`（与 qdrant/lancedb 同形）；
- 经 `mod.rs:40` re-export `SqliteVecBackend`；
- 在 task-23.2（Phase 23）于 `x86_64-pc-windows-msvc` 真实构建 + 运行通过（TEST-23.2.3 open→index→KNN 契约绿）；
- dep 已 optional 在 `Cargo.toml:82` + feature 已声明 `Cargo.toml:127`；

**却没有对应的 factory arm**——`select_vector_backend("sqlite-vec", dim)` 当前落入 `other =>` 分支返回 `unknown vector backend "sqlite-vec"`，与 qdrant/lancedb 的「feature 缺失则诚实 Err」语义不一致：sqlite-vec 是一个**已存在、已在树、可构建**的后端，却被工厂当作未知名拒绝。这是工厂后端覆盖的一处缺口（ADR-034 D 矩阵覆盖不全）。

本 task 聚焦补全该 arm，使 factory 后端覆盖与 `mod.rs` 已 re-export 的后端集合一致：

- **D2a（arm gating 🟢）**：factory 加 `"sqlite-vec"` arm，**镜像 qdrant/lancedb 的双半 cfg 模式**（feat on → `Arc::new(SqliteVecBackend::new()?)` / feat off → 显式 `VectorError` 含 `sqlite-vec` 名 + `vector-sqlite` feature 名，绝不静默回落 BruteForce、绝不伪造成功）。这是确定性 code-local，feat-off 半在默认 build 可直接单测，feat-on 半在 `--features vector-sqlite` build 下单测。
- **D2b（in-process 选择矩阵 wiring 🟢 / cell 🟡）**：补 arm 后，三/四后端经 `select_vector_backend` 的 in-process 选择路径已 wiring 完整（brute / sqlite-vec / [qdrant] / [lancedb] 由 name 选中、各返回正确 `name()`）——这一 **wiring 维度 🟢** 可经确定性单测验证。但矩阵的 **recall@k / latency CELL** 须在本机 `x86_64-pc-windows-msvc` 以 `--features vector-sqlite` 真实 feature build + 真实语料跑出 → 🟡 honest-defer `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`，真实数值真实跑出才回填，**不预填任何 recall/latency 数字**（ADR-013）。

经核：`sqlite-vec` dep 已 optional 在 `Cargo.toml:82`、feature `vector-sqlite` 已声明 `Cargo.toml:127`、`SqliteVecBackend` 已 re-export `mod.rs:40`——本 task **加 0 新 dep**（feature-gated，默认 build 0 vector-dep，ADR-004 / ADR-008 守线），仅在 factory 加一条 arm + 单测。

## 2. Goal

(1) **D2a**：`select_vector_backend` 加 `"sqlite-vec"` arm，镜像 `"qdrant"`/`"lancedb"` 的 `#[cfg(feature=…)]` / `#[cfg(not(feature=…))]` 双半结构：feat `vector-sqlite` on → `Arc::new(SqliteVecBackend::new()?)`；feat off → `return Err(VectorError::Other("vector backend 'sqlite-vec' requires the vector-sqlite feature".into()))`（错误文案须含后端名 `sqlite-vec` + feature 名 `vector-sqlite`，与 qdrant/lancedb 文案同形）。(2) **D2b**：补 arm 后，in-process 选择矩阵的 **wiring** 完整——`select_vector_backend("sqlite-vec", dim)`（feat on）返回 `name()=="sqlite-vec"` 的后端；这一 wiring 维度 🟢 经单测断言。矩阵的 recall/latency CELL 🟡 据实延后 `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`，须本机 MSVC feature build 真实跑出后回填。

pass bar：D2a feat-off 半 + arm 镜像形状经默认 build 单测（🟢，镜像 TEST-29.1.2 双半）；feat-on 半 + wiring 经 `--features vector-sqlite` build 单测（🟢）；矩阵 recall/latency CELL 🟡 `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`（真实跑出才回填，ADR-013 不伪造）；默认 build 0 vector-dep / 默认行为不变（ADR-004）；0 新 dep（ADR-008，`sqlite-vec` 既已 optional 在树）；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `core/src/retriever/vector/factory.rs`——在 `match name` 内 `"lancedb"` arm 之后、`other =>` 之前插入 `"sqlite-vec"` arm，镜像 qdrant/lancedb 的双半 cfg 模式：
  - feat on：`#[cfg(feature = "vector-sqlite")] { Arc::new(crate::retriever::vector::SqliteVecBackend::new()?) }`（`SqliteVecBackend::new()` 返回 `Result<Self, VectorError>` → 用 `?`，与 qdrant/lancedb arm 同形）；
  - feat off：`#[cfg(not(feature = "vector-sqlite"))] { return Err(VectorError::Other("vector backend 'sqlite-vec' requires the vector-sqlite feature".into())); }`。
  - 同步更新 `select_vector_backend` 的 rustdoc（`factory.rs:18-30`）枚举：add-only 列出 `"sqlite-vec"` → `SqliteVecBackend` behind the `vector-sqlite` feature；既有 `""`/`brute`/`qdrant`/`lancedb`/unknown 描述不动（surgical）。
- 加单测（同源 `factory.rs` `#[cfg(test)] mod tests`，镜像 TEST-29.1.2 既有 qdrant/lancedb 双半测试）：
  - feat-off 半（默认 build）：`#[cfg(not(feature = "vector-sqlite"))]` 断言 `select_vector_backend("sqlite-vec", 0)` 为 Err，msg 含 `sqlite-vec` + `vector-sqlite`；
  - feat-on 半（`--features vector-sqlite` build）：`#[cfg(feature = "vector-sqlite")]` 断言返回后端 `name()=="sqlite-vec"`（in-process wiring 🟢）。
- 矩阵 recall/latency CELL：在 §10 / ADR-037 D4 以 `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]` 记 honest-defer 边界——须本机 MSVC `--features vector-sqlite` build + 真实语料跑出，真实数值真实跑出才回填，本 task body **不预填**（ADR-013）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- sqlite-vec in-process 选择矩阵的 recall@k / latency 数值 cell [SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]——须本机 `x86_64-pc-windows-msvc` `--features vector-sqlite` 真实 feature build + 真实语料跑出，🟡 honest-defer，真实数值真实跑出才回填（ADR-013 不伪造）。
- server.rs 两热路径经 config 选 sqlite-vec backend 的端到端注入——属 task-32.1 D1 config plumbing 职责；本 task 范围为补 factory arm（factory 是 plumbing 的被选项之一，端到端注入由 task-32.1 交付）。
- sqlite-vec 持久化（当前 `SqliteVecBackend` 为 in-memory `Connection::open_in_memory()`，`sqlite_vec.rs:52`）落盘 / `persistence_path` 兑现 [SPEC-DEFER:phase-future.sqlite-vec-on-disk-persistence]——本 task 仅补工厂选择，不改后端存储语义。
- sqlite-vec 行级增量索引（当前 `delete` 为 full-reindex 语义，`sqlite_vec.rs:128-137`）[SPEC-DEFER:phase-future.sqlite-vec-incremental-index]（task-23.3 §3 已据实记录）。
- CI 默认构建 `vector-sqlite` feature（`vec0` C amalgamation 须 MSVC/gcc 工具链）[SPEC-DEFER:phase-future.sqlite-vec-ci-default-build]——默认 CI build 仍 0 vector-dep（ADR-004），feature build 单机 opt-in。
- ADR-034 add-only Amendment（sqlite-vec arm 补全 factory）正文回填 [SPEC-OWNER:task-32.4-closeout]（Phase closeout 落点，非本 task body）。

## 4. Actors

- 主 agent（ADR-012 自治）
- `select_vector_backend`（`core/src/retriever/vector/factory.rs`，加 `"sqlite-vec"` arm）
- `SqliteVecBackend`（`core/src/retriever/vector/sqlite_vec.rs`，feat `vector-sqlite`，被工厂选中的后端）
- `cargo test`（默认 build feat-off 半）/ `cargo test --features vector-sqlite`（feat-on 半 + wiring）
- 本机 `x86_64-pc-windows-msvc` MSVC 工具链（feature build + 矩阵 cell 真实跑出的前提，🟡 据实延后 `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/retriever/vector/factory.rs:31-69`（`select_vector_backend` 全体——`match name` 四 arm + `dim` 现 `let _ = dim`；qdrant arm `:40-51` / lancedb arm `:52-63` 即本 task 镜像的双半 cfg 模板；`other =>` `:64-66`）+ `:18-30`（rustdoc 后端枚举——本 task add-only 加 `sqlite-vec` 行）+ `:84-101`（既有 TEST-29.1.2 qdrant/lancedb 双半测试——本 task 镜像之）
- `core/src/retriever/vector/sqlite_vec.rs`（`SqliteVecBackend`——`:43-59` 结构 + `new() -> Result<Self, VectorError>`（`:50`，故 arm 用 `?`）+ `:68-70` `name()="sqlite-vec"` + `:67-194` 实现 `VectorBackend/VectorIndexer/VectorSearcher` 即 `VectorStore`）
- `core/src/retriever/vector/mod.rs:40`（`#[cfg(feature = "vector-sqlite")] pub use sqlite_vec::SqliteVecBackend`——arm 引用路径 `crate::retriever::vector::SqliteVecBackend`，与 qdrant/lancedb re-export `:43,:46` 同形）
- `core/Cargo.toml:82`（`sqlite-vec = { version = "=0.1.9", optional = true }`——dep 既已在树，0 新 dep）+ `:127`（`vector-sqlite = ["dep:sqlite-vec"]`——feature 既已声明）
- `docs/decisions/adr-037-*.md §D2`（vector-backend-config-plumbing；本 task 即其原文实现）+ `§D4`（据实延后边界——矩阵 cell `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`）+ `docs/decisions/adr-034-*.md`（production-vector-live-recall；sqlite-vec arm 补全 factory 的 add-only Amendment 落点 @ task-32.4）

### 5.2 关键设计 — factory sqlite-vec arm 镜像 qdrant/lancedb（双半 gating，默认 0-dep 不变）

- **D2a arm 镜像（确定性）**：`"sqlite-vec"` arm 与 `"qdrant"`（`factory.rs:40-51`）/ `"lancedb"`（`:52-63`）**逐字同形**——唯一差异是 feature 名（`vector-sqlite`）、后端类型（`SqliteVecBackend`）、错误文案中的后端名（`'sqlite-vec'`）。feat-on 半返回 `Arc<dyn VectorStore>`（`SqliteVecBackend` impl 三 trait → `VectorStore`，与 qdrant/lancedb 同 upcast 路径）；feat-off 半返回 `VectorError::Other`，文案含 `sqlite-vec` + `vector-sqlite`——**honest Err，绝不静默回落 BruteForce、绝不伪造成功**（ADR-013 / ADR-034 D1 既定语义）。
- **D2b in-process 选择矩阵 wiring（🟢） vs cell（🟡）**：
  - **wiring 维度 🟢**：补 arm 后，`select_vector_backend(name, dim)` 对 `name ∈ {"", "brute", "sqlite-vec"(feat on), "qdrant"(feat on), "lancedb"(feat on)}` 各返回正确后端（`name()` 断言）——这是「选哪个后端」的确定性 wiring，feat-on 半在 `--features vector-sqlite` build 单测可验。
  - **cell 维度 🟡**：矩阵的 recall@k / latency 数值（「sqlite-vec 在某真实语料上 recall@10 = ?、p50 latency = ?」）须本机 `x86_64-pc-windows-msvc` `--features vector-sqlite` 真实 build + 真实语料跑出 → `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]` honest-defer，**真实跑出才回填，本 task body 不预填任何数字**（ADR-013）。pass bar 仅就 wiring 维度 🟢 标 `[x]`，cell 维度 honest-defer 记录。
- **dim 参数沿用**：`select_vector_backend` 的 `dim`（`factory.rs:33`）当前 `let _ = dim`（`:37`，预留 embedder-dim 协商）——sqlite-vec arm 与既有 arm 一致**不在构造期约束 dim**（`SqliteVecBackend` 在 `open(config)` 期由 `config.dim` 建 `vec0` vtable，`sqlite_vec.rs:83-98`，非构造期）；本 task 不改 `dim` 语义（surgical）。

### 5.3 不变量

- 默认 build 0 vector-dep（ADR-004 / ADR-023 守线）：默认 feature set（`Cargo.toml:125` `default = []`）不含 `vector-sqlite`，默认 build 不拉 `sqlite-vec` C amalgamation；`select_vector_backend("sqlite-vec", _)` 在默认 build 返回 honest Err（含 feature 名），非静默成功。
- 默认行为不变（ADR-004）：既有 arm（`""`/`brute`/`qdrant`/`lancedb`/unknown）行为逐字不变（既有 TEST-29.1.1 / TEST-29.1.2 / TEST-29.1.3 全绿）；新增 arm 仅扩展 `match` 覆盖（add-only），不改既有分支。
- 既有契约不变：`select_vector_backend` 签名（`name: &str, dim: usize -> Result<Arc<dyn VectorStore>, VectorError>`）不变；调用方（server.rs 两热路径、task-32.1 plumbing）不破。
- 0 新代码依赖（ADR-008）：`sqlite-vec` 既已 optional 在 `Cargo.toml:82` + feature 已声明 `:127`——本 task **加 0 新 dep / 0 新 feature**，仅在 factory 加一条 arm + 单测。
- honest Err 优先于静默回落：feat-off 半绝不回落 BruteForce、绝不伪造成功（ADR-013）。
- 矩阵 recall/latency cell 真实跑出才记数（ADR-013）：本 task body 不预填任何 recall/latency 数字，🟡 `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]` 据真实跑出后回填。

## 6. Acceptance Criteria

- [x] AC1（sqlite-vec arm feature 双半 gating）: `select_vector_backend` 加 `"sqlite-vec"` arm，镜像 qdrant/lancedb 双半 cfg 模式——feat `vector-sqlite` on → `Arc::new(SqliteVecBackend::new()?)`（`name()=="sqlite-vec"`）；feat off → honest `VectorError` 含 `sqlite-vec` + `vector-sqlite`（绝不静默回落 BruteForce）；rustdoc add-only 列出该 arm；既有 arm（TEST-29.1.1/.2/.3）逐字不变 — verified by TEST-32.2.1（镜像 TEST-29.1.2 双半）
- [x] AC2（in-process 选择矩阵 wiring 🟢 + 矩阵 cell honest-defer 🟡）: 补 arm 后选择矩阵 wiring 完整——`select_vector_backend("sqlite-vec", dim)`（feat on）返回 `name()=="sqlite-vec"` 后端（wiring 🟢 单测断言）；矩阵 recall@k / latency CELL 须本机 `x86_64-pc-windows-msvc` `--features vector-sqlite` 真实 build + 真实语料跑出 → 🟡 `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`（真实数值真实跑出才回填，本 task **不预填**，ADR-013）；默认 build 0 vector-dep（ADR-004）+ 0 新 dep（ADR-008） — verified by TEST-32.2.2
- [x] AC3（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by TEST-32.2.3（LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-32.2.1 | factory `"sqlite-vec"` arm 双半 gating（镜像 TEST-29.1.2）：feat off → honest Err（msg 含 `sqlite-vec` + `vector-sqlite`，不回落 BruteForce）；feat on（`--features vector-sqlite`）→ 后端 `name()=="sqlite-vec"`；既有 arm 不变 | `core/src/retriever/vector/factory.rs`（同源 test） | Done |
| TEST-32.2.2 | in-process 选择矩阵 wiring 🟢：`select_vector_backend("sqlite-vec", dim)`（feat on）选中正确后端（`name()` 断言）+ 默认 build 0 vector-dep；矩阵 recall/latency CELL 🟡 `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`（须本机 MSVC feature build 真实跑出才回填，不预填） | `core/src/retriever/vector/factory.rs`（同源 test） | Done（wiring 🟢；矩阵 recall/latency CELL 🟡 honest-defer 续延后） |
| TEST-32.2.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）— LAST | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（低）arm 镜像形状偏差**：sqlite-vec arm 若未逐字镜像 qdrant/lancedb 双半 cfg（错误文案缺 feature 名 / feat-off 半误回落 BruteForce），破 honest-Err 契约。
  - **缓解**：以 `factory.rs:40-51`（qdrant）/ `:52-63`（lancedb）为逐字模板，仅替换 feature 名 / 类型 / 文案后端名；TEST-32.2.1 feat-off 半断言 msg 同时含 `sqlite-vec` + `vector-sqlite`。stop-condition：双半 gating 单测不过则 AC1 不标 `[x]`。
- **R2（中→🟡）矩阵 recall/latency cell 须本机 MSVC feature build**：`vec0` C amalgamation 须 MSVC/gcc 工具链；recall/latency 数值须真实语料跑出，本环境 CI 默认不构建该 feature。
  - **缓解**：wiring 维度 🟢（`name()` 选中断言）本 task 验；recall/latency cell `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]` honest-defer，据 ADR-013 不伪造数值，真实跑出后回填。stop-condition：矩阵 cell 未实测则 AC2 仅就 wiring 维度标 `[x]` + cell honest-defer 记录，不预填任何数字。
- **R3（低）feat-on 半单测须 `--features vector-sqlite` build**：默认 `cargo test` 不覆盖 feat-on 半（`#[cfg(feature = "vector-sqlite")]` 编出）。
  - **缓解**：feat-on 半单测 `#[cfg(feature = "vector-sqlite")]` 标注（与既有 TEST-29.1.2 feat-on 半 `factory.rs:109-122` 同形），§9 Verification 显式列 `cargo test -p contextforge-core --features vector-sqlite` 命令；本机 MSVC 跑该命令验 feat-on 半 + wiring。
- **R4（低）误引入新 dep / 新 feature**：sqlite-vec arm 若误声明新 feature 或新 dep，破 ADR-008 add-only / ADR-004 0-dep baseline。
  - **缓解**：`sqlite-vec` dep 既已 optional 在 `Cargo.toml:82`、feature `vector-sqlite` 既已声明 `:127`——本 task 只引用既有 feature，**不动 `Cargo.toml`**；§10 记 0 新 dep / 0 新 feature。

## 9. Verification Plan

```bash
# 1. AC1（feat-off 半，默认 build）+ AC2（wiring 默认 build 0-dep + 既有 arm 不变）
cargo test -p contextforge-core retriever::vector::factory

# 2. AC1（feat-on 半）+ AC2（in-process 选择矩阵 wiring 🟢，须本机 MSVC 工具链）
cargo test -p contextforge-core --features vector-sqlite retriever::vector::factory
#    矩阵 recall/latency CELL 🟡 [SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]
#    （须本机 x86_64-pc-windows-msvc + 真实语料真实跑出才回填，不伪造数值——ADR-013）

# 3. 不退化（全量；默认 build 0 vector-dep 守线）
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# 4. AC3 — D2 lint（LAST）
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界**：in-process 选择矩阵的 recall@k / latency 数值 cell 须本机 `x86_64-pc-windows-msvc` 以 `--features vector-sqlite` 真实 feature build + 真实语料跑出（🟡 toolchain-gated），本环境 CI 默认不构建该 feature → wiring 维度（`name()` 选中）本 task 验（🟢），数值 cell `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`，据 ADR-013 不伪造 recall/latency 数值，真实跑出后回填。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done

**§9 Verification（v0.25.0 / impl PR #213 squash commit `76a3137`，真实证据）**：
- AC1：默认构建 `cargo test -p contextforge-core`（factory selection）→ factory **6/6** 全绿——TEST-32.2.1 feat-off 半断言 `select_vector_backend("sqlite-vec", _)` 为 honest Err（msg 含 `sqlite-vec` + `vector-sqlite`，不回落 BruteForce）；feat-on 半经 **真实 `x86_64-pc-windows-msvc` `cargo test --features vector-sqlite` 构建通过**（`sqlite_vec_with_feature_returns_sqlite_vec_backend`：`SqliteVecBackend::new()`，`name()=="sqlite-vec"`）。既有 arm（TEST-29.1.*）逐字不变。
- AC2：in-process 选择矩阵 wiring 🟢——TEST-32.2.2 `select_vector_backend("sqlite-vec", dim)`（feat on）选中正确后端（`name()` 断言），matrix feat-on 分支经真实 MSVC feat build 真实验证（非仅结构性）+ 默认 build 0 vector-dep（ADR-004）+ 0 新 dep（ADR-008，`sqlite-vec` 既已 optional 在 `Cargo.toml:82` + `vector-sqlite` 既声明 `:127`）。矩阵 recall@k / latency CELL 🟡 `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`——须本机 `x86_64-pc-windows-msvc` `--features vector-sqlite` 真实 build + 真实语料跑出，**本 task body 未预填任何 recall/latency 数字，续延后**（ADR-013，不伪造）。
- AC3：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威；本 task PR #213 四门绿——cargo-test / go-test / spec-lint / lint 全 PASS）。
- 0 新 dep / 0 新 feature（`sqlite-vec` 既 optional 在 `Cargo.toml:82` + `vector-sqlite` 既声明 `:127`，未动 `Cargo.toml`）/ 默认行为不变（既有 TEST-29.1.* 全绿）/ 既有契约不变（`select_vector_backend` 签名不变）。

**实际改动文件**（impl PR #213，squash commit `76a3137`）：
- `core/src/retriever/vector/factory.rs`——`select_vector_backend` `match name` 加 `"sqlite-vec"` arm（镜像 qdrant / lancedb 双半 cfg 门控）+ rustdoc add-only 列出该 arm + `#[cfg(test)] mod tests` 加 TEST-32.2.1（feat-off 半 honest Err + feat-on 半 `name()=="sqlite-vec"`）+ TEST-32.2.2（selection-matrix wiring）→ factory 6/6。
- 矩阵 recall/latency cell：honest-defer `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`，须本机 MSVC feature build + 真实语料真实跑出才回填（本 task body 未预填，续延后）。
- ADR-037 据真实测试 per-D ratify Proposed→Accepted（含 D2 sqlite-vec in-process recall/latency CELL honest-defer `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]` 据实延后）+ ADR-034 sqlite-vec arm 补全 factory 的 add-only Amendment（Phase 32）落点在 task-32.4 closeout（v0.25.0，非本 task body）。
