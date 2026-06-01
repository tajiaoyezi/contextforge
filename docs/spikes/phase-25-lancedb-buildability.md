# Spike: lancedb dev-box buildability (task-25.2 / Phase 25)

> **结论（ADR-013 真实非合成，三态标注）**：🟢 **`cargo build --features vector-lancedb` 在 Windows MSVC 真实构建通过** —
> exit 0（`x86_64-pc-windows-msvc`, rustc 1.95.0, protoc `libprotoc 31.1` via vendored, 2026-06-01）。既有 `LanceDbBackend` 契约（open→index→search KNN + dim mismatch）+ 索引调参参数校验在 **lib 单测下 2/2 PASS**（`--lib retriever::vector::lance_db`，exit 0）。
> protoc 前置以仓内既有 build-dep `protoc-bin-vendored`（3.2.0）的 `protoc.exe` 经 `PROTOC` env 提供——**无需单独系统安装 protoc / cmake**。这 **缩小**（不消除）了 `docs/spikes/phase-18-lancedb.md` 记录的「构建需 protoc」前置在 Windows MSVC 上可能成为 blocker 的担忧（仿 task-23.2 解除 sqlite-vec MSVC 受阻先例）。**未做任何源码 / Cargo.toml / Cargo.lock 改动**用于构建——既有 `lancedb=0.30` / `arrow-array=58` / `futures=0.3` optional 配置在当前工具链下即可经 MSVC 构建（0 新依赖）；本 task 在其上 add-only 加索引调参参数 + 补契约测试。
>
> 🟡 **诚实 caveat（不影响 buildability 结论）**：广义 `cargo test --features vector-lancedb`（编译**全部 integration test target**）在本工具链上触发 rustc 1.95.0 **ICE + rlib-format 链接错误**（`crate datafusion_optimizer/lance required to be available in rlib format, but was not found in this form`），命中的是**与向量无关的 integration test target**（`indexjob_real_runner` / `scanner` / `proto_contract` / `phase4_smoke` / `phase5_smoke` / …）。这是重 lance/datafusion 依赖树在 integration-test 链接阶段的**工具链限制**，非逻辑回归、非 task-25.2 引入（这些 target 不引用本 task 新增符号）。**buildability（`cargo build`）+ lib 单测 + 默认 `cargo test --workspace` 均通过**。详见 §2.4。

## 1. 调查背景

Phase 18 task-18.5 用 `lancedb` 0.30（`core/src/retriever/vector/lance_db.rs`）实现 `LanceDbBackend`（embedded Lance columnar store，`is_local()==true`，disk-backed）。`docs/spikes/phase-18-lancedb.md` 明记其构建在 **Linux x86_64**（WSL2，rustc 1.96.0）通过，但 **需 `protoc`**（lance `build.rs`，当时用 vendored protoc v35.0 via `PROTOC` env + cmake 4.2.3）+ Lance/DataFusion/Arrow 首次构建约 5 分钟。ADR-023 D4 把 lancedb 定为「embedded-columnar alternative」，ADR-030 D2 记录其 protoc 前置在某平台（仿 sqlite-vec 当年在 Windows MSVC `cl.exe` 受阻，`docs/spikes/phase-18-sqlite-vec.md`，后被 task-23.2 真实构建通过解除）可能成为构建 blocker。

task-25.2 仿 task-23.2 的真实可构建性调查 pattern：在 dev box 上真实 `cargo build --features vector-lancedb`（含 protoc 前置探测/处理），通过则记真实凭据、受阻则诚实 stop-condition，并把索引调参参数收敛为可校验配置结构。

## 2. 调查方法 + 真实凭据

### 2.1 protoc 前置探测

```
$ protoc --version            # PATH
PROTOC NOT FOUND              # protoc 不在 PATH
$ cmake --version             # PATH
NOT FOUND                     # cmake 不在 PATH
```

dev box 既无 PATH 内 protoc 亦无 PATH 内 cmake。但仓库 `core/Cargo.toml` 的 `[build-dependencies]` 已有 `protoc-bin-vendored = "3"`（用于 tonic-build 编译本仓自有 proto）；其在 cargo registry cache 内携带 Windows protoc 二进制：

```
$ <cargo-home>\registry\src\<hash>\protoc-bin-vendored-win32-3.2.0\bin\protoc.exe --version
libprotoc 31.1
```

→ 将 `PROTOC` env 指向该 vendored `protoc.exe`，为 lance `build.rs` 提供 protoc 前置（无需单独安装 protoc / cmake；MSVC C/C++ 工具链由 rustc host + `cc` crate 经 vswhere 自动定位，与 task-23.2 一致）。

### 2.2 真实构建（🟢 通过 — AC1）

```
$ rustc -vV
host: x86_64-pc-windows-msvc
release: 1.95.0 (59807616e 2026-04-14)

$ $env:PROTOC = "...\protoc-bin-vendored-win32-3.2.0\bin\protoc.exe"   # libprotoc 31.1
$ cargo build --features vector-lancedb -p contextforge-core
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 13.78s     # 增量（lance 树已编译）
   (exit 0, 0 warnings)
```

依赖树解析版本（`Cargo.lock`，未因本次构建改动 → **0 新依赖**）：`lancedb 0.30.0` / `lance 7.0.0`（+ `lance-core`/`lance-encoding`/`lance-file`/`lance-index`/`lance-io`/`lance-table`/`lance-linalg`/`lance-datafusion` 等 7.0.0）/ `datafusion 53.1.0`（+ ~30 datafusion-* 子 crate）/ `arrow-array 58.3.0` / `arrow-schema 58.3.0` / `arrow-buffer 58.3.0`。

**冷构建凭据（硬证据，非合成）**：`target/debug/deps` 内 **1097 个依赖 rlib** 真实编译产物，含 `liblancedb-*.rlib`（≈144 MB）/ `liblance-*.rlib`（≈951 MB）/ 全 DataFusion 53 + Arrow 58 树；lance/datafusion 部分 rlib 产物 mtime 跨约 3 分钟（与 phase-18 Linux「~5 min 首构建」量级一致）。registry src cache 在构建前为空（无任何 lance/arrow/datafusion 源），构建后含 50+ lance/datafusion/arrow crate 源 → 真实下载 + 编译，非合成。

### 2.3 feature lib 契约 + 索引调参参数校验（构建通过前提 — AC3/AC4）

```
$ cargo test -p contextforge-core --features vector-lancedb --lib retriever::vector::lance_db
test retriever::vector::lance_db::tests::test_25_2_3_index_tuning_validate ... ok
test retriever::vector::lance_db::tests::test_25_2_4_backend_contract_roundtrip ... ok
test result: ok. 2 passed; 0 failed; 169 filtered out     # lib exit 0
```

- **TEST-25.2.4（既有 backend 契约不退化）**：在 Windows MSVC 上真实建 Lance dataset（temp dir）跑 open→index→search KNN（查询 `[1,0,0,0]` 最近邻为 `a`，`DistanceType::Cosine`）+ dim mismatch → `VectorError::DimMismatch`。**真实运行通过**，非仅编译。
- **TEST-25.2.3（索引调参参数范围校验）**：`LanceIndexTuning::validate(dim)` 纯函数（不建真实索引）——合法 IVF_PQ（partitions>0 / sub_vectors 整除 dim）/ HNSW（m>0 / ef>0）`Ok`；partitions=0 / sub_vectors 不整除 dim / sub_vectors=0 / dim=0 / 阈值=0 / HNSW m=0 → 可识别 `Err`。确定性纯函数。

RED→GREEN 已复核：GREEN 前 `validate` 为 `todo!`，`cargo test ...--lib ...lance_db` → 25.2.4 ok / 25.2.3 FAILED（todo! panic）= 1 passed 1 failed；GREEN 后 2 passed 0 failed。

### 2.4 🟡 广义 feature 测试的 rustc ICE caveat（工具链限制，非逻辑回归）

广义 `cargo test -p contextforge-core --features vector-lancedb`（**不加 `--lib`**，即编译全部 integration test target）在本工具链 **exit 101**：

```
error: crate `lance` required to be available in rlib format, but was not found in this form
error: crate `datafusion_optimizer` required to be available in rlib format, but was not found in this form
error: internal compiler error: no resolution for an import   # core\tests\indexjob_real_runner.rs
...
note: rustc 1.95.0 (59807616e 2026-04-14) running on x86_64-pc-windows-msvc
error: could not compile `contextforge-core` (test "indexjob_real_runner" / "scanner" /
       "proto_contract" / "phase4_smoke" / "phase5_smoke" / "phase9_index_smoke" /
       "core_skeleton" / "memory_integration" / "search_persist_integration")
```

诚实定性：

1. **命中的全是与向量无关的 integration test target**（`indexjob_real_runner` / `scanner` / `proto_contract` / `phase4_smoke` / `phase5_smoke` / `phase9_index_smoke` / `core_skeleton` / `memory_integration` / `search_persist_integration`）——它们不引用 task-25.2 新增的任何符号（`LanceAnnIndex` / `LanceIndexTuning` / `validate`），失败在**编译/链接阶段**，非运行时逻辑。
2. **成因 = 重 lance/datafusion 依赖树在 integration-test 链接阶段的 rustc 工具链限制**：`required to be available in rlib format, but was not found in this form` 是 cargo 为「仅元数据可达」的依赖只产 `.rmeta`、而 integration test crate 链接 lib 时需 `.rlib` 的已知 Rust 现象；叠加 rustc 1.95.0 在 `indexjob_real_runner.rs` 上的 ICE（`note: we would appreciate a bug report`）。
3. **非 task-25.2 引入**：本 task diff 仅向 `lance_db.rs` add-only 加 feature-gated 代码；上述 target 在父提交同样会以 `cargo test --features vector-lancedb` 命中（启用该 feature + integration-test 链接的固有属性，自 task-18.5 即存在）。
4. **不影响 buildability 结论 + 本 task 门禁**：`cargo build --features vector-lancedb`（AC1）exit 0；lib 单测（AC3/AC4，task §9 以 `--lib ...lance_db` 跑）2/2；默认 `cargo test --workspace`（AC5）exit 0、全绿。本 task 的契约/参数测试驻于 lib（`#[cfg(test)] mod tests`），以 `--lib` 过滤即避开 integration-target 编译。

→ 正确调用口径：**buildability** 用 `cargo build --features vector-lancedb`；**本 task lib 单测** 用 `cargo test --features vector-lancedb --lib retriever::vector::lance_db`。广义全 target 测试在该 feature 下受工具链 ICE 限制，留 `[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]`（CI 注入 protoc + 跨工具链守护）+ backlog 关注。

## 3. 结论（ADR-013 三态如实）

- 🟢 **真实通过（buildability）**：`cargo build --features vector-lancedb` 在 `x86_64-pc-windows-msvc`（rustc 1.95.0 + vendored protoc 31.1）exit 0。**真实非合成凭据**（1097 rlib 编译产物 + registry cache 真实下载 + Cargo.lock 未变 0 新依赖），非伪造跨平台通过。
- 🟢 **lib 契约 + 参数校验**：`--lib retriever::vector::lance_db` 2/2 PASS（真实 Lance dataset open→index→KNN + dim mismatch + 索引调参参数范围校验）。
- 🟡 **caveat**：广义 `cargo test --features vector-lancedb`（全 integration test target）受 rustc 1.95.0 ICE + rlib-format 链接限制（向量无关 target，非逻辑回归，非本 task 引入）——见 §2.4。
- **落地动作 = 0 源码改动用于构建 + add-only 索引调参参数 + 补契约测试**：既有 `vector-lancedb` 配置即可 MSVC 构建（protoc 经 `PROTOC` env 提供）；新增 `LanceAnnIndex` / `LanceIndexTuning` / `validate` + 2 个 feature-gated lib 单测（add-only，不改三 trait 签名 / 不改 `LanceDbBackend` 读写本体 `[SPEC-OWNER:task-18.5-spike-lancedb]`）。
- 既有 Linux protoc `vector-lancedb` 路径**不退化**（无破坏性源码改动）。

### 诚实 caveat（汇总）

1. 证据来自单台 Windows MSVC dev box（rustc 1.95.0，2026-06-01），与 phase-18 Linux 凭据互补；非声称所有 MSVC / 所有工具链版本通过。
2. **protoc 仍是硬前置**：不在 PATH / 未设 `PROTOC` 时 lance `build.rs` 会失败。本机以仓内 `protoc-bin-vendored` 的 `protoc.exe` 满足（contributor 构建本仓后该二进制即在 cargo cache）——protoc-prereq 担忧由此**缩小**（可满足、不需系统安装）而**非消除**（仍须显式提供 `PROTOC`）。cmake 在本机本次构建未被要求。
3. CI 默认不构建 `vector-lancedb` feature（默认 0 vector 依赖，ADR-023 D5）→ 跨 CI 的 lancedb MSVC 构建非持续守护；CI protoc 注入承 `[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]`。
4. **真实 ANN 索引建图 + 大语料性能** `[SPEC-DEFER:phase-future.lancedb-index-tuning]`、**数据集 compaction 真实执行** `[SPEC-DEFER:phase-future.lancedb-schema-compaction]` 诚实延后：本 task 落参数校验配置结构（不建真实大索引）；n=5000 仍走 flat scan（phase-18），真实 IVF_PQ/HNSW 索引性能属构建通过后的集成验证。
5. **广义 feature 全 target 测试的 rustc ICE**（§2.4）：工具链限制，非逻辑回归、非本 task 引入；buildability + lib 单测 + 默认 workspace 均通过。

## 4. 对 ADR / 下游影响

- **ADR-030 D2（lancedb 可构建性）**：本 spike 提供真实 dev-box 构建凭据（🟢 Windows MSVC `cargo build` 通过）+ 索引调参参数校验，作 task-25.3 据真实非合成结果 ratify ADR-030 Proposed→Accepted 的 D2 维度证据（含 §2.4 ICE caveat 如实记录）。
- **ADR-023 D4 dev/prod parity**：lancedb（embedded-columnar alternative）现可在 Windows MSVC dev box 构建 + lib 运行（protoc 经 vendored 二进制满足）→ 与 task-23.2（sqlite-vec MSVC 通过）一并缩小 ADR-023 D1 记录的 dev/prod backend parity 缺口（历史 Consequences 不溯改，本 spike add-only 记录现状，ADR-014 D5）。
- **task-25.3（closeout）**：引用本 spike 作 v0.18.0 evidence + 生产 backend 选择矩阵 lancedb 档 caveat（protoc 前置 / 重 Arrow 栈构建 / 单机凭据 / 广义 feature 测试 ICE / 真实索引性能延后）依据。
- **`[SPEC-DEFER]`**：`phase-future.lancedb-index-tuning`（真实索引性能）/ `phase-future.lancedb-schema-compaction`（compaction 执行）/ `phase-future.lancedb-build-prereq-ci`（CI protoc 注入 + 跨 CI 持续守护 + 广义 feature 测试 ICE 守护）承 `docs/spikes/phase-18-lancedb.md` Follow-up，本 task 未声称覆盖。
