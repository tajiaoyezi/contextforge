# Task `23.2`: `sqlite-vec-cross-platform — 调查 core/Cargo.toml vector-sqlite + core/src/retriever/vector/sqlite_vec.rs 的 Windows MSVC 可构建路径（bundled C amalgamation / 预编译 / 替代绑定）；落地或诚实文档化 stop-condition（承 phase-18 既有结论，禁伪造跨平台通过）+ docs/spikes/phase-23-sqlite-vec-cross-platform.md`

**Status**: Draft

**Priority**: P2
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 23 (vector-persistence-and-cross-platform)
**Dependencies**: task-18.3（`SqliteVecBackend` via `sqlite-vec` 0.1.9 `vec0` + `vector-sqlite` feature，Linux gcc 可构建实测 + Windows MSVC 受阻凭据保留）/ task-18.1（三 trait freeze）/ ADR-023 D1（sqlite-vec 嵌入式推荐默认 + Windows-MSVC-build-blocked 记录）/ ADR-008（core-library-selection，依赖变更 add-only）/ ADR-013（禁伪造跨平台凭据）/ ADR-014 D1-D5（第十四次激活）

## 1. Background

Phase 18 task-18.3 用 `sqlite-vec` 0.1.9 的 `vec0` 虚表实现 `SqliteVecBackend`（`core/src/retriever/vector/sqlite_vec.rs`）：经 `rusqlite::ffi::sqlite3_auto_extension` 把 `sqlite3_vec_init` 注册进 rusqlite 的 bundled SQLite，`open` 建 `CREATE VIRTUAL TABLE vec_items USING vec0(embedding float[N])`，KNN 经 `embedding MATCH ? ORDER BY distance`。`core/Cargo.toml:82` 把 `sqlite-vec = { version = "=0.1.9", optional = true }` pin 在 `vector-sqlite` feature 下（0.1.10-alpha.4 缺 `sqlite-vec-diskann.c` C amalgamation）。

task-18.3 实测：**Linux x86_64 gcc 可构建并跑出真实 5 维数据**（recall@5/10=1.0、P95 0.167ms 等，`docs/spikes/phase-18-sqlite-vec.md`）；但 **Windows MSVC 构建受阻**——`sqlite-vec` 的 C amalgamation 经 gcc 编译，MSVC 工具链下受阻，凭据保留（`docs/releases/v0.11.0-evidence.md` / adapter task-18.3 行：`Windows MSVC 受阻 [SPEC-DEFER:phase-future.sqlite-vec-cross-platform] 凭据保留`）。ADR-023 D1 据此把 sqlite-vec 定为生产嵌入式推荐默认（架构与 ADR-002 最契合），但 Consequences 明记「the recommended default (sqlite-vec) does not build on the Windows dev box, so dev/prod backend parity is imperfect」，并把跨平台移植列为 Follow-up（`[SPEC-DEFER:phase-future.sqlite-vec-cross-platform]`）。

本 task 真实调查 sqlite-vec 在 Windows MSVC 的可构建路径，缩小 dev/prod backend parity 缺口；调查类任务（🔴 受阻平台），结论可能为「某路径落地构建通过」或「确证仍受阻 → 诚实文档化 stop-condition」，按 ADR-013 不伪造跨平台通过。

## 2. Goal

真实调查 sqlite-vec 在 Windows MSVC 工具链下的可构建路径（bundled C amalgamation 编译选项调整 / 预编译二进制扩展加载 / 替代 Rust 绑定三路径之一）。**若任一路径通过**：在 `core/Cargo.toml` `vector-sqlite` feature / `core/src/retriever/vector/sqlite_vec.rs` 落地该路径，使 `cargo build --features vector-sqlite` 在 Windows MSVC 通过，且既有 Linux `vector-sqlite` backend 行为不退化。**若确证仍受阻**：诚实文档化 stop-condition（承 `docs/spikes/phase-18-sqlite-vec.md` 既有 gcc-only 凭据 + 本 task 真实尝试的失败凭据），不伪造跨平台通过（ADR-013）。两种结论均产出 `docs/spikes/phase-23-sqlite-vec-cross-platform.md`（调查方法 + 真实构建结果 + ADR-013 三态如实标）。≥1 Rust 测试（feature `vector-sqlite` 下既有 sqlite-vec backend 契约不退化，Linux 可跑）全 PASS。默认构建（无 `vector-sqlite`）0 新依赖、行为不变。D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **调查 + 记录 `docs/spikes/phase-23-sqlite-vec-cross-platform.md`（新增）**：按 spike 模板记录三路径真实尝试——(a) bundled C amalgamation 在 MSVC 下的编译选项 / build 脚本调整；(b) sqlite-vec 预编译 `vec0` 扩展二进制 + 运行时 `load_extension` 加载；(c) 替代 Rust 绑定 / 同等 SQLite KNN 扩展。每路径记真实 `cargo build` 输出（通过 / 失败凭据），ADR-013 三态如实标（构建通过 / 确证受阻 stop-condition / 部分平台）。
- **若某路径通过 → 修改 `core/Cargo.toml`（`vector-sqlite` feature）+ `core/src/retriever/vector/sqlite_vec.rs`**：落地可在 Windows MSVC 构建的路径（依赖 / build 配置 / 加载方式调整），保持既有 Linux gcc 路径不退化；依赖变更经主 agent R7 chore（subagent 不自改 Cargo.toml）。
- **若确证受阻 → `docs/spikes/phase-23-sqlite-vec-cross-platform.md` 文档化 stop-condition**：承 Phase 18 既有结论 + 本 task 真实尝试凭据，明确「Windows MSVC 经调查仍受阻」+ 受阻成因 + 推荐 dev 用 hnsw fallback（ADR-023 D2）；不在源码伪造跨平台构建通过。
- **新增/扩同源 Rust 测试（`core/src/retriever/vector/sqlite_vec.rs` 内 `#[cfg(test)] mod tests`，feature `vector-sqlite` gated）**：既有 sqlite-vec backend 契约不退化的 deterministic 测试（open→index→search 命中 + dim mismatch 错误路径），Linux gcc 下可跑；若 MSVC 路径落地则补 MSVC 可构建断言。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **sqlite-vec backend 本体（`vec0` 虚表 + KNN）** [SPEC-OWNER:task-18.3-spike-sqlite-vec]：本 task 在其上做跨平台构建调查，不重写 backend 逻辑。
- **sqlite-vec on-disk 持久化 / blob 编码细化** [SPEC-DEFER:phase-future.sqlite-vec-on-disk]：本 task 聚焦 MSVC 可构建路径，on-disk 编码细化属独立 marker。
- **hnsw 图持久化** [SPEC-OWNER:task-23.1-hnsw-graph-persistence]：本 task 与 23.1 写路径不相交（sqlite_vec.rs/Cargo.toml vs hnsw.rs）。
- **向量增量索引** [SPEC-DEFER:phase-future.vector-incremental-index]：评估在 task-23.3。
- **v0.16.0 release docs + smoke v13 + ADR-028 ratify** [SPEC-OWNER:task-23.3-closeout-v0.16.0]：本 task 产出调查结论；closeout 引用它。
- **qdrant / lancedb 跨平台构建** [SPEC-DEFER:phase-future.qdrant-deployment-topology] / [SPEC-DEFER:phase-future.lancedb-build-prereq-ci]：本 task 仅调查 sqlite-vec。

## 4. Actors

- **主 agent**：实施 + PR 主理 + 调查结论裁决（落地 vs stop-condition）。
- **`core/src/retriever/vector/sqlite_vec.rs::SqliteVecBackend`**：task-18.3 sqlite-vec backend，本 task 调查其跨平台构建。
- **`core/Cargo.toml` `vector-sqlite` feature（`sqlite-vec = "=0.1.9"` / `rusqlite bundled`）**：构建配置面，本 task 据调查结论调整或维持。
- **`sqlite-vec` crate C amalgamation + MSVC 工具链**：受阻面，本 task 真实尝试三路径。
- **下游 task-23.3**：closeout 引用本 task 调查结论作 v0.16.0 evidence + ADR-028 sqlite-vec 跨平台决策依据。

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/retriever/vector/sqlite_vec.rs`（`register_extension` / `sqlite3_auto_extension` / `vec0` 虚表 / `open`/`index_batch`/`search` / dim mismatch 错误路径）
- `core/Cargo.toml:70`（`rusqlite = { version = "0.39.0", features = ["bundled"] }`）+ `:82`（`sqlite-vec = { version = "=0.1.9", optional = true }` + 0.1.10-alpha.4 缺 diskann.c 注释）+ `[features] vector-sqlite`
- `docs/spikes/phase-18-sqlite-vec.md`（Linux gcc 可构建实测 + Windows MSVC 受阻成因凭据）
- `docs/releases/v0.11.0-evidence.md`（v0.11.0 平台矩阵 + sqlite-vec gcc-only 记录）
- `docs/decisions/adr-023-vector-backend-default.md` D1（sqlite-vec 嵌入式默认 + Windows-MSVC-build-blocked）+ Consequences（dev/prod parity imperfect）+ Follow-ups（`sqlite-vec-cross-platform`）
- `docs/decisions/adr-008-core-library-selection.md`（依赖选型 add-only）+ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造跨平台凭据红线）

### 5.2 关键设计 — 三路径调查 + 诚实定论

- **路径 (a) bundled C amalgamation MSVC 编译**：调查 `sqlite-vec` 0.1.9 的 C amalgamation 在 MSVC `cl.exe` 下的编译失败成因（C 标准 / 内建函数 / 链接），尝试 build 脚本 / 编译标志调整使其经 MSVC 通过。
- **路径 (b) 预编译扩展运行时加载**：调查不静态编译 C，改为分发预编译 `vec0` 动态扩展 + rusqlite 运行时 `load_extension` 加载；评估其与 `sqlite3_auto_extension` 注册路径的取舍 + 安全基线（扩展加载默认禁用语义）。
- **路径 (c) 替代绑定**：调查同等 SQLite KNN 能力的替代 Rust 绑定 / crate，在保持 `VectorSearcher` 契约下替换底层（ADR-008 依赖选型 add-only Amendment）。
- **诚实定论（ADR-013）**：任一路径在 Windows MSVC `cargo build --features vector-sqlite` 真实通过 → 落地 + 记录真实凭据；三路径全部确证受阻 → 文档化 stop-condition（承 Phase 18 gcc-only 既有结论 + 本 task 失败凭据），推荐 dev 用 hnsw fallback（ADR-023 D2），不在源码伪造跨平台构建通过。
- **本 task 不阻塞 phase**：sqlite-vec 受阻不阻 task-23.1（hnsw 持久化）/ task-23.3（增量索引 + closeout）——三者写路径与依赖独立。

### 5.3 不变量

- 默认构建（无 `vector-sqlite` feature）0 新依赖、`SqliteVecBackend` 不编译、行为逐字节不变（ADR-023 D5）。
- 既有 Linux gcc `vector-sqlite` 路径不退化（无论 MSVC 路径是否落地）。
- 调查结论诚实：不伪造 Windows MSVC 构建通过；受阻态以「真实尝试凭据 + stop-condition 文档」如实记录，AC2 在受阻态下据此满足而非伪造 `[x]`。
- 不改 `VectorBackend` / `VectorIndexer` / `VectorSearcher` 三 trait 签名（task-18.1 freeze）。

## 6. Acceptance Criteria

- [ ] **AC1**: sqlite-vec Windows MSVC 三路径（bundled amalgamation / 预编译扩展 / 替代绑定）真实调查完成，每路径真实 `cargo build` 凭据记录到 `docs/spikes/phase-23-sqlite-vec-cross-platform.md`（ADR-013 三态如实标）— verified by **TEST-23.2.1** + §10 实测记录
- [ ] **AC2**: 调查给出真实结论——某路径在 Windows MSVC `cargo build --features vector-sqlite` 通过则落地（`core/Cargo.toml` / `sqlite_vec.rs`）+ 既有 Linux 路径不退化；或确证三路径受阻则诚实文档化 stop-condition（承 Phase 18 既有结论，禁伪造跨平台通过，ADR-013）— verified by **TEST-23.2.2** + §10
- [ ] **AC3**: feature `vector-sqlite` 下既有 sqlite-vec backend 契约不退化（open→index→search 命中 + dim mismatch 错误路径），Linux gcc 下 deterministic 可断言；不破坏 task-18.1 三 trait 签名 — verified by **TEST-23.2.3**
- [ ] **AC4**: 既有不退化 — 默认 `cargo test --workspace`（无 vector feature）全 PASS + 0 新依赖；`go test ./...` 不受影响（本 PR 零 Go delta）— verified by **TEST-23.2.4** + §10
- [ ] **AC5**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-23.2.5** + §10 记录

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-23.2.1 | 三路径 MSVC 构建真实调查凭据 + spike 记录 | `docs/spikes/phase-23-sqlite-vec-cross-platform.md` | Planned |
| TEST-23.2.2 | 调查结论（路径落地 MSVC 构建通过 / 或 stop-condition 文档化） | `core/Cargo.toml` + `core/src/retriever/vector/sqlite_vec.rs` 或 spike doc | Planned |
| TEST-23.2.3 | feature `vector-sqlite` 既有 backend 契约不退化（Linux） | `core/src/retriever/vector/sqlite_vec.rs`（`mod tests`） | Planned |
| TEST-23.2.4 | 默认 `cargo test --workspace` 0 failed + 0 新依赖 | 全 Rust | Planned |
| TEST-23.2.5 | D2 lint `--touched origin/master` 0 未标注命中 | `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（高）Windows MSVC 经调查仍受阻**（承 phase-23 §7 R2 / task-18.3 既有凭据）：三路径可能全部确证受阻。
  - **缓解**：真实尝试 bundled / 预编译 / 替代绑定三路径；全部受阻则诚实文档化 stop-condition（ADR-013），推荐 dev 用 hnsw 跨平台 fallback（ADR-023 D2）——AC2 在受阻态以「真实调查 + stop-condition 文档」满足，不标伪造 `[x]`。受阻不阻塞 phase（23.1 / 23.3 独立）。
- **R2（中）预编译扩展运行时加载触及安全基线**（ADR-004 local-first / 扩展加载默认禁用）：路径 (b) 需 enable extension loading。
  - **缓解**：路径 (b) 评估须记录扩展加载的安全取舍（仅加载随发行物分发的可信 `vec0` 扩展）；若与安全基线冲突则降优先级，记录于 spike doc 取舍段。
- **R3（中）替代绑定引入新供应链表面 / 行为差异**（ADR-008 依赖选型）：路径 (c) 换底层 crate。
  - **缓解**：替代绑定须保持 `VectorSearcher` 契约 + 既有 Linux 行为不退化；依赖变更经主 agent R7 chore + ADR-008 add-only Amendment 记录，subagent 不自改 Cargo.toml。

## 9. Verification Plan

```bash
# Rust：默认构建（无 vector feature）0 新依赖 + 不退化
cargo test --workspace

# feature 下 sqlite-vec 契约（Linux gcc 可构建）
cargo test --workspace --features vector-sqlite

# Windows MSVC 构建真实尝试（本 task 调查核心；通过则记录凭据，受阻则记录 stop-condition）
cargo build --features vector-sqlite   # on Windows MSVC toolchain

# Go 不退化（本 PR 零 Go delta）
go test ./...

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **Status**: 待实施（Draft）。实施完成后按以下 6 项回填：完成日期 / 改动文件 / commit 列表 / §9 Verification 结果（含 Windows MSVC `cargo build` 真实凭据）/ 设计取舍（三路径真实尝试结论 + 落地路径或 stop-condition 裁决，ADR-013 数据源声明）/ 剩余风险 + 下游影响（ADR-028 sqlite-vec 跨平台决策依据）。
