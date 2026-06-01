# Task `25.2`: `lancedb-buildability-and-index-tuning — dev box 真实 cargo build --features vector-lancedb 可构建性调查（protoc 前置，仿 task-23.2 sqlite-vec MSVC 调查 pattern；构建通过记真实凭据 / 确证受阻诚实 stop-condition 不伪造）+ core/src/retriever/vector/lance_db.rs 索引调参参数（IVF_PQ/HNSW + compaction 口径校验）+ docs/spikes/phase-25-lancedb-buildability.md 三态如实`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 25 (production-vector-backend)
**Dependencies**: task-18.5（`LanceDbBackend` via `lancedb` 0.30 + `vector-lancedb` feature 已落地，Linux protoc 可构建凭据）/ task-23.2（sqlite-vec Windows MSVC 真实可构建性调查 pattern + 三态如实标）/ task-18.1（`VectorIndexConfig` 字段 + 三 trait freeze）/ ADR-023 D4（lancedb embedded-columnar alternative tier）/ ADR-030 D2（lancedb 可构建性 + 索引调参）/ ADR-008（core-library-selection，依赖变更 add-only）/ ADR-013（禁伪造跨平台构建凭据）/ ADR-014 D1-D5（第十六次激活）

## 1. Background

Phase 18 task-18.5 用 `lancedb` 0.30（`core/src/retriever/vector/lance_db.rs`）实现 `LanceDbBackend`：`new` 建 tokio runtime + `lancedb::connect(LANCEDB_DIR)`（默认 temp_dir 下 `contextforge-lancedb-spike`）；`open` `create_empty_table(TABLE, schema)`（schema = Int32 `id` + FixedSizeList Float32 `vector` dim）；`index_batch` 经 arrow `RecordBatch` + `table.add`；`search` `nearest_to().distance_type(DistanceType::Cosine).limit(k)`（n=5000 走 flat scan，未建 ANN 索引）。ADR-023 D4 把 lancedb 定为「embedded-columnar alternative」（最快写入 50ms + 列存持久 + SQL/metadata 过滤），代价是最重的构建（Lance/DataFusion + protoc 前置）+ 最高单查询延迟。

`docs/spikes/phase-18-lancedb.md` 明记：构建**需 `protoc`**（lance `build.rs`，vendored protoc v35.0 via `PROTOC` env + cmake）+ Lance/DataFusion/Arrow 首次构建约 5 分钟；`arrow-array` pin 到 58 匹配 Lance；n=5000 走 flat scan，「IVF_PQ/HNSW 索引会改变 scale 下延迟」；并把 Lance 索引调参（`[SPEC-DEFER:phase-future.lancedb-index-tuning]`）、数据集 schema 演进/compaction（`[SPEC-DEFER:phase-future.lancedb-schema-compaction]`）、CI protoc 注入（`[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]`）列为 Follow-up。

protoc 前置 + 重 Arrow 栈在某平台（仿 sqlite-vec 当年在 Windows MSVC `cl.exe` 下受阻，`docs/spikes/phase-18-sqlite-vec.md` / 后被 task-23.2 真实构建通过解除）可能成为构建 blocker。本 task 仿 task-23.2 的真实可构建性调查 pattern：在 dev box 上真实 `cargo build --features vector-lancedb`（含 protoc 前置探测/安装），通过则记真实凭据、受阻则诚实 stop-condition，并把 lancedb 索引调参参数收敛为可校验配置结构。

## 2. Goal

在 dev box 上真实调查 lancedb 可构建性并据结果落地或诚实定论：(a) **可构建性**——真实 `cargo build --features vector-lancedb`（含 protoc 前置探测/安装），构建通过则记真实凭据（rustc / protoc 版本 / 构建耗时）+ feature 下既有 `LanceDbBackend` 契约不退化；确证受阻（protoc 缺失不可补 / Lance·DataFusion·Arrow 栈在该平台构建受阻）则诚实文档化 stop-condition（承 `docs/spikes/phase-18-lancedb.md` protoc-prereq + sqlite-vec MSVC 先例），**不伪造跨平台构建通过**（ADR-013）；(b) **索引调参参数**——把 lancedb ANN 索引调参参数（IVF_PQ / HNSW 的 `num_partitions` / `num_sub_vectors` / metric）+ compaction 触发口径收敛为一个可校验配置结构，参数范围校验在不建真实索引下可单测。产出 `docs/spikes/phase-25-lancedb-buildability.md`（三态如实标：🟢 构建通过 / 🔴 确证受阻 stop-condition / 🟡 部分平台·caveat）。≥2 Rust 测试（feature `vector-lancedb` 下，构建通过前提下）：既有 lancedb backend 契约不退化 + 索引调参参数范围校验。默认构建（无 `vector-lancedb`）0 新依赖、行为不变；`cargo test --workspace` 不退化。D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **调查 `core/Cargo.toml` `vector-lancedb` feature / `core/src/retriever/vector/lance_db.rs`**：在 dev box 工具链上真实 `cargo build --features vector-lancedb -p contextforge-core`（含 protoc 前置探测/安装，仿 task-23.2 sqlite-vec MSVC 调查），记录真实凭据（rustc / protoc 版本 / 构建耗时 / 平台 arch）。
- **修改 `core/src/retriever/vector/lance_db.rs`**（构建通过前提下）：加索引调参配置结构——IVF_PQ / HNSW 调参参数（`num_partitions` / `num_sub_vectors` / metric）+ compaction 触发口径（如行数阈值），`validate()` 参数范围校验（partitions>0 / sub_vectors>0 且整除 dim / metric 受支持）在不建真实索引下纯函数可单测。
- **新增 `docs/spikes/phase-25-lancedb-buildability.md`**：记录调查方法（protoc 前置 / dev-box 构建命令 / 真实凭据）+ 真实构建结果，ADR-013 三态如实标（🟢 构建通过 / 🔴 确证受阻 stop-condition 承 protoc-prereq + sqlite-vec MSVC 先例 / 🟡 部分平台·caveat），仿 `docs/spikes/phase-23-sqlite-vec-cross-platform.md` 结构。
- **新增同源 Rust 单测（`core/src/retriever/vector/lance_db.rs` 内 `#[cfg(test)] mod tests`，feature `vector-lancedb` gated，构建通过前提下）**：(a) 既有 lancedb backend 契约不退化——open→index→search KNN（temp dir Lance dataset）+ dim mismatch error 路径；(b) 索引调参参数范围校验——合法参数 Ok、partitions=0 / sub_vectors 不整除 dim → 可识别 Err（纯函数，不建真实索引）。
- **可选修改 `core/Cargo.toml`**：`vector-lancedb` feature 若索引调参需新 crate 面——按 add-only 评估，依赖变更经主 agent R7 chore（subagent 不自改 Cargo.toml）；lancedb 0.30 / arrow-array 58 / futures 0.3 已 optional。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **lancedb ANN 索引真实建图 + 大语料性能测量** [SPEC-DEFER:phase-future.lancedb-index-tuning]：本 task 落参数校验配置结构（不建真实大索引）；真实建 IVF_PQ/HNSW 索引 + 性能属构建通过后的集成验证，承 `docs/spikes/phase-18-lancedb.md` Follow-up。
- **lancedb 数据集 schema 演进 / compaction 真实执行** [SPEC-DEFER:phase-future.lancedb-schema-compaction]：本 task 落 compaction 触发口径（参数），真实 compaction 执行后续。
- **CI 注入 protoc / 跨 CI lancedb 构建持续守护** [SPEC-DEFER:phase-future.lancedb-build-prereq-ci]：本 task 在 dev box 单机真实凭据，跨 CI 持续守护承 `docs/spikes/phase-18-lancedb.md` Follow-up。
- **`LanceDbBackend` 的 open/index/search 本体** [SPEC-OWNER:task-18.5-spike-lancedb]：本 task 在其上加索引调参参数 + 可构建性调查，不重写 columnar 读写。
- **qdrant server 生命周期** [SPEC-OWNER:task-25.1-qdrant-server-lifecycle]：本 task 仅做 lancedb。
- **生产 backend 选择矩阵 / smoke v15 / v0.18.0 closeout** [SPEC-OWNER:task-25.3-closeout-v0.18.0]：本 task 交付可构建性结论 + 索引调参参数，矩阵/收口在 25.3。

## 4. Actors

- **主 agent**：实施 + PR 主理 + 可构建性结论决策（构建通过 vs 诚实 stop-condition）。
- **`core/src/retriever/vector/lance_db.rs::LanceDbBackend`**：task-18.5 lancedb backend，本 task 加索引调参参数 + 可构建性调查对象。
- **`core/src/retriever/vector/types.rs::VectorIndexConfig`**：`dim`/`metric` 字段，本 task 索引调参参数校验（sub_vectors 整除 dim）的来源。
- **`lancedb` 0.30 + `arrow-array` 58 + protoc**：构建链（lance `build.rs` 需 protoc），本 task 真实调查其在 dev box 可构建性。
- **dev box 工具链**：本 task 真实构建发生地（rustc / protoc / cmake 版本如实记录）。
- **下游 task-25.3**：closeout 据本 task 可构建性结论评估选择矩阵 lancedb 档 caveat。

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/retriever/vector/lance_db.rs`（`LanceDbBackend` / `new`（`LANCEDB_DIR` + `lancedb::connect`）/ `open`（`create_empty_table` + Int32 id + FixedSizeList Float32 vector schema）/ `index_batch`（arrow `RecordBatch` + `table.add`）/ `search`（`nearest_to().distance_type(Cosine).limit(k)` + `_distance` 列 → 1-dist similarity）/ `is_local()==true`）
- `core/src/retriever/vector/types.rs::VectorIndexConfig`（`dim` / `metric: VectorMetric`）+ `VectorChunk` / `VectorHit` / `VectorError`
- `core/Cargo.toml`（`vector-lancedb = ["dep:lancedb", "dep:arrow-array", "dep:futures"]`；`lancedb = "0.30"` / `arrow-array = "58"` / `futures = "0.3"` 全 optional）
- `docs/spikes/phase-18-lancedb.md`（protoc 前置 + vendored protoc v35.0 via PROTOC env + cmake + ~5min 首构建 + arrow 58 pin + flat scan + index-tuning/schema-compaction/build-prereq-ci Follow-up）+ `docs/decisions/adr-023-vector-backend-default.md` D4
- `docs/spikes/phase-23-sqlite-vec-cross-platform.md`（真实可构建性调查 pattern + 三态如实标 + 单机 caveat 口径，本 task 复用其结构）+ `docs/spikes/phase-18-sqlite-vec.md`（sqlite-vec MSVC 受阻先例）
- `docs/decisions/adr-030-production-vector-backend.md` D2（lancedb 可构建性 + 索引调参）+ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造跨平台）+ `docs/decisions/adr-008-core-library-selection.md`（依赖 add-only）+ `lancedb` 0.30 文档（`Table::create_index` / IVF_PQ/HNSW 参数 / `optimize`·compaction API 面核实）

### 5.2 关键设计 — 可构建性调查 + 索引调参参数

- **可构建性真实调查（仿 task-23.2 三态）**：在 dev box 上真实 `cargo build --features vector-lancedb`——先探测/安装 protoc（lance `build.rs` 前置）；构建通过 → 记真实凭据（rustc / protoc / cmake 版本、构建耗时、平台 arch）+ feature 下既有契约测试通过；确证受阻（protoc 不可得 / Arrow·DataFusion 栈在该平台编译受阻）→ 诚实文档化 stop-condition（承 protoc-prereq + sqlite-vec MSVC 先例），不伪造跨平台构建通过。三态如实标进 spike。
- **索引调参参数结构**：把 lancedb ANN 索引调参参数（IVF_PQ：`num_partitions` / `num_sub_vectors`；HNSW：`m` / `ef_construction`；metric）+ compaction 触发口径（行数阈值）收敛为可校验配置结构。`validate()`（partitions>0 / sub_vectors>0 且整除 dim / metric 受支持 / 阈值>0）纯函数在不建真实索引下可单测——这是参数面的契约，真实建索引性能是后续集成。
- **ADR-013**：可构建性是真实 dev-box 构建（🟢 通过 / 🔴 stop-condition / 🟡 caveat 三态如实，非合成）；索引调参参数校验是 deterministic feature 测试可验证项（🟡 feature 下不建真实大索引）；真实 ANN 索引性能不预判、不伪造。

### 5.3 不变量

- 默认构建（无 `vector-lancedb` feature）0 新依赖、`LanceDbBackend` 不编译、行为逐字节不变（ADR-023 D5 / ADR-004）。
- 索引调参参数校验纯函数：given 相同参数 → 相同 Ok/Err（确定性，可单测）。
- 可构建性结论据真实 dev-box 构建凭据（ADR-013：通过记真实凭据 / 受阻诚实 stop-condition，不伪造跨平台通过）。
- 既有 Linux protoc `vector-lancedb` 路径不退化（无破坏性源码改动；索引调参为 add-only 配置面）。
- 不改三 trait 签名（`VectorBackend` / `VectorIndexer` / `VectorSearcher`）——索引调参为 `LanceDbBackend` inherent 配置，不破坏 task-18.1 trait freeze。

## 6. Acceptance Criteria

- [x] **AC1**: lancedb 真实 dev-box 可构建性给出结论——`cargo build --features vector-lancedb`（含 protoc 前置）构建通过记真实凭据（rustc/protoc/cmake 版本 + 耗时 + arch），或确证受阻时诚实文档化 stop-condition（承 protoc-prereq + sqlite-vec MSVC 先例，禁伪造跨平台构建通过，ADR-013）— verified by **TEST-25.2.1**（🟢 x86_64-pc-windows-msvc / rustc 1.95.0 / protoc 31.1 vendored / `cargo build` exit 0 / 0 新依赖）
- [x] **AC2**: `docs/spikes/phase-25-lancedb-buildability.md` 产出 + 三态如实标（🟢 构建通过 / 🔴 确证受阻 stop-condition / 🟡 部分平台·caveat）+ 单机 caveat 口径（仿 `docs/spikes/phase-23-sqlite-vec-cross-platform.md`）— verified by **TEST-25.2.2**（🟢 buildability + 🟡 广义 feature 全 target 测试 rustc ICE caveat 如实标）
- [x] **AC3**: 索引调参参数范围校验——`validate()` 合法参数（partitions>0 / sub_vectors 整除 dim / metric 受支持 / 阈值>0）Ok；非法（partitions=0 / sub_vectors 不整除 dim）→ 可识别 Err（纯函数，不建真实索引）— verified by **TEST-25.2.3**
- [x] **AC4**: 既有 lancedb backend 契约不退化（构建通过前提）——feature 下 open→index→search KNN + dim mismatch error 路径正确；真实 ANN 索引性能 / compaction 执行诚实延后（`[SPEC-DEFER:phase-future.lancedb-index-tuning]` / `[SPEC-DEFER:phase-future.lancedb-schema-compaction]`）— verified by **TEST-25.2.4**
- [x] **AC5**: 既有不退化 + D2 lint — 默认 `cargo test --workspace`（无 vector feature）全 PASS + 0 新依赖；`go test ./...` 不受影响（本 PR 零 Go delta）；`bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-25.2.5** + §10 实测

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-25.2.1 | dev-box 真实 `cargo build --features vector-lancedb` 可构建性结论（通过记凭据 / 受阻 stop-condition） | `docs/spikes/phase-25-lancedb-buildability.md` + §10 | Done（🟢 `cargo build` exit 0） |
| TEST-25.2.2 | spike 三态如实标 + 单机 caveat 口径（仿 phase-23 sqlite-vec spike） | `docs/spikes/phase-25-lancedb-buildability.md` | Done（🟢 + 🟡 ICE caveat） |
| TEST-25.2.3 | 索引调参参数范围校验（合法 Ok / partitions=0·sub_vectors 不整除 dim Err，纯函数） | `core/src/retriever/vector/lance_db.rs`（`mod tests`） | Done |
| TEST-25.2.4 | feature `vector-lancedb` 既有 backend 契约不退化（open→index→search KNN + dim mismatch） | `core/src/retriever/vector/lance_db.rs`（`mod tests`） | Done |
| TEST-25.2.5 | 默认 `cargo test --workspace` 0 failed + 0 新依赖 + D2 lint 0 未标注命中 | 全 Rust + `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（高）lancedb 在 dev box 因 protoc/Arrow 栈构建受阻**（承 phase-25 §7 R2）：`docs/spikes/phase-18-lancedb.md` 记构建需 protoc + ~5min Arrow 栈；某平台可能受阻（仿 sqlite-vec 当年 MSVC）。
  - **缓解**：真实尝试 dev-box 构建（含 protoc 前置探测/安装）；通过即记真实凭据 + 契约不退化，受阻则诚实文档化 stop-condition（承 protoc-prereq + sqlite-vec MSVC 先例），按 ADR-013 不伪造跨平台构建通过——AC1 在「确证受阻」态下以「真实调查 + stop-condition 文档」满足，不标伪造 `[x]`。
- **R2（中）lancedb 0.30 索引调参 API 面与设计假设不符**（IVF_PQ/HNSW 参数名 / compaction API）：参数结构需核实真实 API。
  - **缓解**：先核实 `lancedb` 0.30 的 `create_index` / IVF_PQ/HNSW 参数 / `optimize`(compaction) API 面；参数校验纯函数只依赖参数语义（partitions/sub_vectors/metric/阈值），与具体 API 调用解耦——API 变化只影响建索引调用，校验层稳定可单测；真实建索引延后。
- **R3（低）索引调参引入新 crate 面**：default build 须 0 新依赖。
  - **缓解**：优先复用 lancedb 0.30 既有 `create_index` API（IVF_PQ/HNSW 已在 crate 内）；如需新 crate 仅在 `vector-lancedb` feature 下引入，经主 agent R7 chore（subagent 不自改 Cargo.toml），默认构建 0 新 dep（ADR-023 D5 / ADR-004）。

## 9. Verification Plan

```bash
# 真实 dev-box 可构建性调查（含 protoc 前置；仿 task-23.2 pattern）
protoc --version            # lance build.rs 前置探测
cargo build --features vector-lancedb -p contextforge-core

# feature 下 lancedb 契约 + 索引调参参数校验（构建通过前提）
cargo test -p contextforge-core --features vector-lancedb retriever::vector::lance_db

# 默认构建（无 vector feature）0 新依赖 + 不退化
cargo test --workspace

# Go 不退化（本 PR 零 Go delta）
go test ./...

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**: 2026-06-01
- **改动文件**:
  - `core/src/retriever/vector/lance_db.rs`（修改，feature `vector-lancedb` gated）— 新增 `LanceAnnIndex`（`IvfPq{num_partitions,num_sub_vectors}` / `Hnsw{m,ef_construction}`）+ `LanceIndexTuning`（`index`/`metric`/`compaction_threshold_rows`）+ `LanceIndexTuning::validate(dim)` 参数契约层纯函数 + `import VectorMetric`；新增 2 个 feature-gated lib 单测（TEST-25.2.3 参数校验 / TEST-25.2.4 backend 契约往返）
  - `docs/spikes/phase-25-lancedb-buildability.md`（新增）— 真实 dev-box 可构建性调查（protoc 前置 / 构建命令 / 真实凭据 / 三态如实标 🟢 buildability + 🟡 广义 feature 全 target 测试 rustc ICE caveat），仿 `docs/spikes/phase-23-sqlite-vec-cross-platform.md`
  - `docs/specs/tasks/task-25.2-...md`（本文件）— Status→Done + AC1-5 勾选 + TEST 表 + §10
- **commit 列表**:
  - `071e81f` test(retriever): TEST-25.2.3~25.2.4 RED — lancedb 索引调参参数校验 + backend 契约（todo!）
  - `b1c7fee` feat(retriever): lancedb 索引调参参数校验（IVF_PQ/HNSW + compaction 口径），通过 TEST-25.2.3~25.2.4
  - （本 commit）docs(spec): 回填 task-25.2 §10 + Status → Done + spike phase-25-lancedb-buildability
- **§9 Verification 结果**（ADR-013 真实非合成，单台 Windows MSVC dev box；详见 `docs/spikes/phase-25-lancedb-buildability.md`）:
  - build: 🟢 `cargo build --features vector-lancedb -p contextforge-core` **exit 0, 0 warnings**（`x86_64-pc-windows-msvc`, rustc 1.95.0, protoc `libprotoc 31.1` via 仓内 `protoc-bin-vendored` 3.2.0 经 `PROTOC` env）；增量 feature 重建 13.78s；冷构建硬证据 = `target/debug/deps` 1097 依赖 rlib（含 `liblancedb`/`liblance`/datafusion 53 + arrow 58 树）；**Cargo.lock 未变 → 0 新依赖**（lancedb/arrow-array/futures 自 task-18.5 即 optional）
  - unit-test（feature lib）: `cargo test -p contextforge-core --features vector-lancedb --lib retriever::vector::lance_db` **2 passed / 0 failed（lib exit 0）**（TEST-25.2.3 参数范围校验纯函数 / TEST-25.2.4 真实 Lance dataset open→index→search KNN + dim mismatch）；RED→GREEN 已复核（RED 25.2.3 FAILED on todo! / 25.2.4 ok → GREEN 2/2）
  - unit-test（默认）: `cargo test --workspace`（无 vector feature）**exit 0，全 binary 0 failed**（169 lib + 全 integration target；lance_db.rs 全 `#[cfg(feature="vector-lancedb")]` gated，默认构建不编译，逐字节不变）
  - integration: `go test ./...` — 本 PR 零 Go delta（改动仅 Rust + docs）
  - lint: D2 `bash scripts/spec_drift_lint.sh --touched origin/master` — 触及行 `[SPEC-DEFER:phase-future.lancedb-index-tuning]` / `[SPEC-DEFER:phase-future.lancedb-schema-compaction]` / `[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]` 已标注 → exit 0（0 未标注命中）
  - 🟡 caveat: 广义 `cargo test --features vector-lancedb`（编译全部 integration test target）exit 101——rustc 1.95.0 ICE + `crate datafusion_optimizer/lance required to be available in rlib format` 链接错误，命中**向量无关 integration test target**（`indexjob_real_runner`/`scanner`/`proto_contract`/`phase4_smoke`/…）。工具链限制（重 lance/datafusion 树在 integration-test 链接阶段），非逻辑回归、非本 task 引入（这些 target 不引用本 task 符号；启用 feature + integration-test 链接的固有属性，自 task-18.5 即存在）。buildability + lib 单测 + 默认 workspace 均通过。详见 spike §2.4。
- **设计取舍**:
  - **可构建性结论 🟢（非 🔴/🟡）**：lancedb 在 Windows MSVC `cargo build` 真实通过——protoc 前置以仓内既有 `protoc-bin-vendored` 二进制经 `PROTOC` env 满足（无需系统安装 protoc/cmake），与 task-23.2 解除 sqlite-vec MSVC 受阻同向缩小 ADR-023 D4 dev/prod parity 缺口。**诚实 caveat**：protoc 仍是硬前置（须显式提供 `PROTOC`）→ 担忧「缩小」非「消除」；单机凭据，CI 默认不构建该 feature；广义 feature 全 target 测试受 rustc ICE 限制（工具链项）。
  - **索引调参参数结构**：`LanceIndexTuning` 作参数契约层（`validate` 纯函数不建真实索引）——与具体 `create_index` API 调用解耦（API 变化只影响建索引调用，校验层稳定可单测）；IVF_PQ 的 `num_sub_vectors` 整除 dim 是 PQ 等长切分前提；`compaction_threshold_rows` 为 compaction 触发口径（真实执行延后）。
  - **add-only / 不破坏 trait**：索引调参为 `LanceIndexTuning` inherent 类型，不改 task-18.1 三 trait 签名 / 不重写 `LanceDbBackend` 读写本体（`[SPEC-OWNER:task-18.5-spike-lancedb]`）；`VectorError::Other(String)` 承载校验错误（避免新增 error 变体，`#[non_exhaustive]` add-only 安全）。
  - **真实索引性能延后口径**：本 task 落参数校验（不建真实大索引）；真实 IVF_PQ/HNSW 建图 + 性能 `[SPEC-DEFER:phase-future.lancedb-index-tuning]` / compaction 执行 `[SPEC-DEFER:phase-future.lancedb-schema-compaction]` 承 phase-18 Follow-up。
- **剩余风险 / 未做项**: 真实 ANN 索引建图 + 大语料性能 / compaction 真实执行诚实延后（见 `[SPEC-DEFER]` 标）；CI 注入 protoc + 跨 CI lancedb 构建持续守护 + 广义 feature 全 target 测试 rustc ICE 守护 `[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]`。
- **下游 task 影响**: task-25.3（closeout 据本可构建性结论 🟢 评估生产 backend 选择矩阵 lancedb 档 caveat：protoc 前置 / 重 Arrow 栈 / 单机凭据 / 广义 feature 测试 ICE / 真实索引性能延后；ADR-030 D2 维度据本 spike 真实凭据 ratify）。可与 task-25.1（qdrant，已 Done）并行（写路径不相交）。
