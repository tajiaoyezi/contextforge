# Task `34.2`: `vector-backend-config-file — Go config.go [vector] 段（Backend/Dim）+ setVectorEnv 跨进程 env-bridge（镜像 setDataDirEnv），把 config.toml 的 [vector] 桥接为 CONTEXTFORGE_VECTOR_BACKEND/_DIM 供 spawned core daemon 经既有 resolve_vector_backend env 路径拾取；env-wins 覆盖（显式 env 优先，向后兼容）/ 无 [vector] 段 = 不导出 = unset = BruteForce 字节等价（ADR-004）；Rust core 0 toml dep（0 新 dep，ADR-008）`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 34 (vector-config-completeness)
**Dependencies**: 既有 `internal/config/config.go`（task-1.2 手写 TOML 编解码 + `[remote]`/`[embedding]`/`[[collections]]` 段，Phase 1 已交付）/ 既有 `cmd/contextforge/main.go:254-268` `setDataDirEnv`（跨进程 env-bridge 范式，已证 spawned core daemon 经 `os.LookupEnv("CONTEXTFORGE_DATA_DIR")` 拾取）/ 既有 `internal/daemon/daemon.go:201-209` `launch`（`exec.Command` 继承父进程 env）/ 既有 `core/src/server.rs:540-545` `resolve_vector_backend`（task-32.1 已交付，读 `CONTEXTFORGE_VECTOR_BACKEND`/`CONTEXTFORGE_VECTOR_DIM`）/ task-34.1（vector-dim-auto-negotiation，本 phase 同批；config-file 与 dim-negotiation 共同补齐 Phase 32 起的 env-plumbing）/ ADR-037（vector-backend-config-plumbing-and-completeness；本 task = 其 Phase 34 add-only Amendment 落点 @ task-34.3 closeout，dim-negotiation + config-file 完成 Phase 32 起的 env-plumbing 故事）/ ADR-004（local-first-privacy-baseline，默认行为 + 既有契约不变；无 [vector] 段 = 默认 BruteForce 字节等价）/ ADR-008（dep add-only，Phase 34 = 0 新 dep；Rust core 不引 toml dep）/ ADR-039 §D2（本 task 即其原文实现）/ ADR-013（禁伪造红线——env-bridge 据实声明同 `CONTEXTFORGE_DATA_DIR` 已证范式，非新 daemon.Options 字段重构）/ ADR-012（main-agent-governance-autonomy）/ ADR-014 D1-D5（第二十五次激活）

## 1. Background

Phase 32（task-32.1，ADR-037）已让 Rust core 经 `resolve_vector_backend`（`server.rs:540-545`）从 `CONTEXTFORGE_VECTOR_BACKEND` / `CONTEXTFORGE_VECTOR_DIM` 两个环境变量在搜索热路径选 vector backend + dim，但**配置仅 env-only**——用户无法经持久化的 `config.toml` 声明 vector backend，须每次启动手设环境变量。本 task 把 vector backend 配置补进 Go 侧 `config.toml`，与既有 `[remote]`/`[embedding]` 段对齐：

- **B1 vector backend 配置 env-only，无 config.toml 落点**：`config.toml` 经 `internal/config/config.go` 解析，已有 `[collections]` / `[remote]`（`:298-320`）/ `[embedding]`（`:322-338`）段，但**无 `[vector]` 段**——vector backend 选择仅经 `CONTEXTFORGE_VECTOR_BACKEND` / `CONTEXTFORGE_VECTOR_DIM` 环境变量进 Rust core（`server.rs:540-545`）。用户在 `config.toml` 里没有声明 vector backend 的对称落点（与 `[embedding] provider/dim` 不对称）。
- **B2 Rust core 无 toml dep（不能在 Rust 侧读 config.toml）**：Rust core 持 `serde` / `serde_json` 但**无 `toml` 依赖**——在 Rust 侧加 config.toml reader 须引 `toml` crate，破 0 新 dep（ADR-008）。故配置桥接须在已解析 `config.toml` 的 **Go 侧**完成，不动 Rust core 依赖面。
- **B3 已证的跨进程 env-bridge 范式（`CONTEXTFORGE_DATA_DIR`）**：`cmd/contextforge/main.go:254-268` 的 `setDataDirEnv` 把 data-dir 经 `os.Setenv("CONTEXTFORGE_DATA_DIR", ...)` 导出为父进程环境变量；spawned core daemon 经 `internal/daemon/daemon.go:201-209` `launch` 的 `exec.Command`（继承父进程 env）启动，于 Rust 侧经 `os.LookupEnv` 等价路径拾取。本 task **复用同一已证范式**——加一个 `setVectorEnv`（镜像 `setDataDirEnv`），把 `[vector]` 段桥接为 `CONTEXTFORGE_VECTOR_BACKEND` / `CONTEXTFORGE_VECTOR_DIM`，spawned core daemon 经既有 `resolve_vector_backend`（`server.rs:540-545`）env 路径拾取。**这是同一跨进程 env-bridge，不是延后的 `daemon.Options.DataDir` 字段重构**（后者 [SPEC-DEFER:phase-future.daemon-options-datadir]）。
- **B4 env-wins 向后兼容 + 无 [vector] 段 = BruteForce 字节等价**：显式设置的环境变量**优先于** config 文件（与 `setDataDirEnv` 的「`dataDir != ""` 才 Setenv，保留旧值」一致的「env 已设则不覆盖」语义）——保证既有 env-only 工作流向后兼容。无 `[vector]` 段 ⇒ 不导出任何变量 ⇒ 两个环境变量 unset ⇒ `resolve_vector_backend` 收 `("", 0)` ⇒ BruteForce 默认（`server.rs:538` 注：byte-equivalent to `select_vector_backend("", 0)`），默认行为不变（ADR-004）。

本 task 为 code-local 🟢 可单测（Go config TOML round-trip + `setVectorEnv` env-wins 单测），0 新 dep（沿用 task-1.2 手写 TOML codec + 既有 `os` 标准库）；Rust core 依赖面零变化（不引 toml dep）。

## 2. Goal

(1) **B1**：为 Go `config.Config` 加 `[vector]` 段——`Vector VectorConfig` 字段，`VectorConfig{ Backend string（toml `backend`）; Dim int（toml `dim`）}`，经既有手写 TOML codec（`encodeTOML` `:183-204` / `decodeTOML` `:206-264`）对称编解码，与 `[remote]` / `[embedding]` 段并列（add-only，absent 段 ⇒ zero value）。(2) **B2/B3**：加 `setVectorEnv` helper（镜像 `setDataDirEnv` `:254-268`）——当 `[vector]` 段存在（字段非零）**且对应环境变量未显式设置**时，`os.Setenv` 导出 `CONTEXTFORGE_VECTOR_BACKEND` / `CONTEXTFORGE_VECTOR_DIM`，供 spawned core daemon 经既有 `resolve_vector_backend`（`server.rs:540-545`）env 路径拾取；返回 restore 闭包（镜像 `setDataDirEnv` 返回 `func()`）。(3) **B4**：env-wins——显式已设的环境变量不被 config 覆盖（向后兼容）；无 `[vector]` 段 ⇒ 不导出 ⇒ unset ⇒ BruteForce 字节等价（ADR-004 默认不变）。(4) **0 dep**：Go 侧沿用手写 TOML codec + `os` 标准库；Rust core **不引 toml dep**（0 新 dep，ADR-008）。

pass bar：Go config `[vector]` 段 TOML round-trip 经确定性单测验证（含 / 不含 `[vector]` 段双向；既有 `[collections]`/`[remote]`/`[embedding]` 段不受影响）（🟢）；`setVectorEnv` env-wins 经单测验证（段存在 ⇒ env 导出；env 已显式设 ⇒ 不覆盖（env-wins）；无段 ⇒ 不导出）（🟢）；Rust core 0 toml dep（依赖面零变化）；无 `[vector]` 段 = BruteForce 字节等价（ADR-004） + 既有契约（`config.Config` 既有字段 / `daemon.Options` / `resolve_vector_backend`）不变；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `internal/config/config.go`——加 `VectorConfig` 结构（`Backend string` toml 标签 `backend` / `Dim int` toml 标签 `dim`，doc 注：`""` ⇒ unset ⇒ core 默认 BruteForce；`Dim 0` ⇒ unset ⇒ core 默认 dim 协商）+ `Config` 加 `Vector VectorConfig` 字段（add-only，与 `Embedding EmbeddingConfig` `:39` 并列）。
- `encodeTOML`（`:183-204`）加 `\n[vector]\n` 段输出（`backend` / `dim` 两行，镜像 `[embedding]` 段 `:194-196`）；`decodeTOML`（`:206-264`）`switch` 加 `case line == "[vector]"`（镜像 `[embedding]` 段 `:219-221`）+ `assignVector`（镜像 `assignEmbedding` `:322-338`：`backend` 走 `parseTOMLString`，`dim` 走 `strconv.Atoi`）。
- 加 `setVectorEnv` helper 于 `cmd/contextforge/main.go`（镜像 `setDataDirEnv` `:254-268`）——入参 vector backend / dim（取自 `config.Config.Vector`），对 `CONTEXTFORGE_VECTOR_BACKEND`（backend 非空且 env 未设时 Setenv）/ `CONTEXTFORGE_VECTOR_DIM`（dim 非零且 env 未设时 Setenv）逐变量 `os.LookupEnv` 探测 + `os.Setenv` 导出 + 返回 restore 闭包（env-wins：已显式设的变量不覆盖）。
- 同源测试：`internal/config` 同包 test（`[vector]` 段 TOML round-trip + 既有段不受影响）+ `cmd/contextforge` 同包 test（`setVectorEnv` env-wins：段存在导出 / env 已设不覆盖 / 无段不导出）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- 把 vector 配置经 `daemon.Options` 结构字段（而非环境变量）传给 spawned core daemon [SPEC-DEFER:phase-future.daemon-options-datadir]——本 task 复用已证的跨进程 env-bridge（同 `CONTEXTFORGE_DATA_DIR` 范式），不引入 `daemon.Options.DataDir`/`daemon.Options.Vector` 字段重构（该重构属另一延后边界，本 task 不依赖、不实现）。
- 在 Rust core 侧直接读 `config.toml`（须引 `toml` crate）[SPEC-DEFER:phase-future.rust-core-toml-reader]——破 0 新 dep（ADR-008），故配置桥接限于已解析 `config.toml` 的 Go 侧 + 经环境变量过桥，Rust core 依赖面不动。
- vector backend 的 live feature 构建 / 真实 dim 强约束兑现（qdrant/lancedb/sqlite-vec 声明 dim 时的强约束触发）[SPEC-DEFER:phase-future.vector-dim-feature-enforce]（task-34.1 §3 已记）——本 task 仅做配置桥接，dim 协商的 default-build no-op honest-caveat 见 task-34.1。
- config.toml 的 `[vector]` 段语义校验（未知 backend 名在 Go 侧拒绝 / 提示）——本 task 仅做桥接，backend 名的合法性由 Rust core `select_vector_backend`（`factory.rs:33-39`，未知名 explicit error）权威裁决，Go 侧透传，honest-defer 不在 Go 侧重复校验逻辑 [SPEC-DEFER:phase-future.go-side-vector-name-validate]。
- 真实 release tag / run-id / digest（v0.27.0）[SPEC-OWNER:task-34.3-closeout]（ADR-012 用户授权后回填）。

## 4. Actors

- 主 agent（ADR-012 自治）
- `config.Config`（`internal/config/config.go:31-40`，本 task 加 `Vector VectorConfig` 字段，与 `Embedding` `:39` 并列）
- `VectorConfig`（新结构，镜像 `EmbeddingConfig` `:61-64`：`Backend string` / `Dim int`）
- `encodeTOML` / `decodeTOML` / `assignVector`（`internal/config/config.go:183-204` / `:206-264`，本 task 加 `[vector]` 段对称编解码，镜像 `[embedding]` 路径）
- `setVectorEnv`（新 helper 于 `cmd/contextforge/main.go`，镜像 `setDataDirEnv` `:254-268`，跨进程 env-bridge）
- spawned core daemon（`internal/daemon/daemon.go:201-209` `launch` `exec.Command` 继承父进程 env）+ `resolve_vector_backend`（`core/src/server.rs:540-545`，既有 env 拾取点，本 task 不改）
- 运维 / 部署者（经 `config.toml [vector]` 声明 vector backend，无须每次手设环境变量）

## 5. Behavior Contract

### 5.1 Required Reading

- `internal/config/config.go:31-40`（`Config` 根结构——`:39` `Embedding EmbeddingConfig` 是 `Vector VectorConfig` 并列声明的对齐点）+ `:57-64`（`EmbeddingConfig` `Provider string` / `Dim int` + doc——`VectorConfig` 镜像形态源）
- `internal/config/config.go:183-204`（`encodeTOML`——`:194-196` `[embedding]` 段输出，`[vector]` 段镜像）+ `:206-264`（`decodeTOML`——`:219-221` `case line == "[embedding]"` 段头分派，`[vector]` 镜像）+ `:322-338`（`assignEmbedding`——`backend` 走 `parseTOMLString` / `dim` 走 `strconv.Atoi`，`assignVector` 镜像）
- `cmd/contextforge/main.go:254-268`（`setDataDirEnv`——`os.LookupEnv` 探旧值 + `dataDir != ""` 才 `os.Setenv` + 返回 restore 闭包；`setVectorEnv` 镜像此范式 + env-wins 语义）
- `internal/daemon/daemon.go:201-209`（`launch` `exec.Command(d.opts.CoreBinPath, d.opts.ListenAddr)` 继承父进程 env——env-bridge 跨进程生效的依据）
- `core/src/server.rs:533-545`（`resolve_vector_backend` 读 `CONTEXTFORGE_VECTOR_BACKEND` / `CONTEXTFORGE_VECTOR_DIM`，`:538` 注 unset/blank → `("", 0)` byte-equivalent to `select_vector_backend("", 0)` → BruteForce 默认）——本 task 桥接的下游拾取点，不改
- `docs/decisions/adr-037-vector-backend-config-plumbing-and-completeness.md`（本 task = 其 Phase 34 add-only Amendment 落点 @ task-34.3 closeout）+ `docs/decisions/adr-039-vector-config-completeness.md §D2`（本 task 即其原文实现）+ ADR-004（默认 BruteForce 字节等价）/ ADR-008（Rust core 0 toml dep）

### 5.2 关键设计 — Go [vector] 段 + setVectorEnv 跨进程 env-bridge（env-wins / 无段字节等价 / Rust 0-dep）

- **B1 Go `[vector]` 段（镜像 `[embedding]`）**：`VectorConfig{ Backend string; Dim int }`（`Backend` toml 标签 `backend`，`Dim` toml 标签 `dim`），`Config.Vector` add-only 字段；`encodeTOML`（`:183-204`）输出 `\n[vector]\nbackend = "..."\ndim = N\n`（镜像 `[embedding]` `:194-196`）；`decodeTOML`（`:206-264`）`case line == "[vector]"` 切段 + `assignVector`（`backend`→`parseTOMLString`，`dim`→`strconv.Atoi`，镜像 `assignEmbedding` `:322-338`）。**absent `[vector]` 段 ⇒ `Config.Vector` 为 zero value（`Backend ""` / `Dim 0`）**，与既有 `[embedding]` absent 行为一致（task-1.2 手写 codec 容忍缺段）。
- **B2/B3 `setVectorEnv` 跨进程 env-bridge（镜像 `setDataDirEnv`）**：在 `cmd/contextforge/main.go` 加 `setVectorEnv(backend string, dim int) (func(), error)`，逐变量复用 `setDataDirEnv`（`:254-268`）范式——对 `CONTEXTFORGE_VECTOR_BACKEND`：`old, hadOld := os.LookupEnv(...)`；**仅当 env 未显式设（`!hadOld`）且 `backend != ""` 时** `os.Setenv`；对 `CONTEXTFORGE_VECTOR_DIM` 同理（`!hadOld` 且 `dim != 0` 时 `os.Setenv(..., strconv.Itoa(dim))`）；返回 restore 闭包（恢复每个变量的旧值 / unset，镜像 `setDataDirEnv` 返回 `func()`）。spawned core daemon 经 `daemon.go:201-209` `exec.Command`（继承父进程 env）启动 → Rust 侧 `resolve_vector_backend`（`server.rs:540-545`）拾取，与 `CONTEXTFORGE_DATA_DIR` 同跨进程范式。
- **B4 env-wins（向后兼容）**：`setVectorEnv` 对每个变量先 `os.LookupEnv` 探测；**若该变量已显式设置（`hadOld == true`）则不 `os.Setenv` 覆盖**——显式 env 优先于 config 文件（既有 env-only 工作流向后兼容）。这是与 `setDataDirEnv`「保留旧值」语义一致的「env-wins」实现（`setDataDirEnv` 经 restore 闭包恢复旧值；本 task 进一步在 set 前判 `!hadOld` 避免覆盖已设值）。
- **无 `[vector]` 段 = unset = BruteForce 字节等价（ADR-004）**：absent `[vector]` ⇒ `Config.Vector` zero value ⇒ `setVectorEnv("", 0)` ⇒ 两变量均不 `os.Setenv`（`backend == ""` / `dim == 0` 守卫）⇒ 两环境变量 unset ⇒ `resolve_vector_backend` 收 `("", 0)`（`server.rs:537-539`）⇒ BruteForce 默认（byte-equivalent to `select_vector_backend("", 0)`）。默认 build 行为不变。
- **Rust core 0 toml dep（ADR-008）**：配置桥接全程在 Go 侧（已解析 `config.toml`）+ 环境变量过桥完成；Rust core **不引 `toml` crate**（依赖面零变化）；下游拾取点 `resolve_vector_backend`（`server.rs:540-545`）为 task-32.1 既有代码，本 task 不改。
- **非 `daemon.Options` 字段重构**：本 task 复用已证的跨进程 env-bridge（同 `CONTEXTFORGE_DATA_DIR`），**不**引入 `daemon.Options.DataDir` / `daemon.Options.Vector` 结构字段把配置经 `daemon.Options` 传入 [SPEC-DEFER:phase-future.daemon-options-datadir]（该重构属另一延后边界，本 task 不依赖、不实现，ADR-013 据实声明用已证范式而非新建结构）。

### 5.3 不变量

- 默认行为不变（ADR-004）：无 `[vector]` 段 ⇒ `Config.Vector` zero value ⇒ `setVectorEnv` 不导出 ⇒ 环境变量 unset ⇒ `resolve_vector_backend` 收 `("", 0)` ⇒ BruteForce（byte-equivalent to `select_vector_backend("", 0)`，`server.rs:538`）；既有 `config.toml`（无 `[vector]` 段的旧文件）Load 后 `Vector` 为 zero value，Save 后新增 `[vector]` 段（zero 值 `backend = ""` / `dim = 0`，语义等同 unset）。
- 既有契约不变：`config.Config` 既有字段（`SchemaVersion` / `DataDir` / `Denylist` / `Collections` / `Remote` / `Embedding`）+ 既有段（`[remote]` / `[embedding]` / `[[collections]]`）编解码不变（add-only `Vector` 字段 + `[vector]` 段，既有 round-trip 不退化）；`daemon.Options` 结构 / `daemon.go` `launch` 不变；`resolve_vector_backend`（`server.rs:540-545`）/ `select_vector_backend`（`factory.rs:33-39`）契约不动。
- env-wins（向后兼容）：显式设置的 `CONTEXTFORGE_VECTOR_BACKEND` / `CONTEXTFORGE_VECTOR_DIM` 优先于 `config.toml [vector]`——`setVectorEnv` 仅在变量未设（`!hadOld`）时导出，既有 env-only 工作流行为不变。
- 0 新代码依赖（ADR-008）：Go 侧沿用 task-1.2 手写 TOML codec + `os` / `strconv` 标准库，无第三方依赖增量；**Rust core 不引 `toml` crate**（依赖面零变化）。
- 跨进程范式据实（ADR-013）：env-bridge 同 `CONTEXTFORGE_DATA_DIR` 已证范式（`setDataDirEnv` + `exec.Command` 继承 env），非延后的 `daemon.Options` 字段重构 [SPEC-DEFER:phase-future.daemon-options-datadir]；据实声明用已证范式，不夸大为新装配机制。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [ ] **AC1**（Go config `[vector]` 段 TOML round-trip + `setVectorEnv` env-wins 🟢）: `VectorConfig{Backend,Dim}` + `Config.Vector` add-only 字段；`encodeTOML`/`decodeTOML` 对称编解码 `[vector]` 段（含 / 不含段双向 round-trip 恒等），既有 `[collections]`/`[remote]`/`[embedding]` 段不受影响（TEST-1.2.* 不退化）；`setVectorEnv`：段存在（backend/dim 非零）⇒ `CONTEXTFORGE_VECTOR_BACKEND`/`CONTEXTFORGE_VECTOR_DIM` 经 `os.Setenv` 导出；env 已显式设 ⇒ 不覆盖（env-wins）；无段（`"",0`）⇒ 不导出；restore 闭包恢复旧值；**Rust core 0 toml dep** — verified by **TEST-34.2.1**（config round-trip）+ **TEST-34.2.2**（setVectorEnv env-wins）
- [ ] **AC2**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-34.2.3**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-34.2.1 | Go config `[vector]` 段 TOML round-trip：含 `[vector]`（`backend`/`dim`）Save→Load 恒等 + 不含 `[vector]` 段 Load 得 zero value（`Backend ""`/`Dim 0`）+ 既有 `[collections]`/`[remote]`/`[embedding]` 段编解码不受影响（不退化） | `internal/config/config_test.go`（同包 test） | Planned |
| TEST-34.2.2 | `setVectorEnv` env-wins：段存在（backend/dim 非零）⇒ `CONTEXTFORGE_VECTOR_BACKEND`/`CONTEXTFORGE_VECTOR_DIM` 经 `os.Setenv` 导出（spawned core daemon 经 env 路径拾取）；env 已显式设 ⇒ 不被 config 覆盖（env-wins）；无段（`"",0`）⇒ 不导出（unset ⇒ BruteForce 字节等价）；restore 闭包恢复旧值 / unset | `cmd/contextforge/main_test.go`（同包 test） | Planned |
| TEST-34.2.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（中）`[vector]` 段加入破既有 config.toml round-trip**：加 `[vector]` 段输出 / 解析可能影响既有 `[remote]`/`[embedding]`/`[[collections]]` 段的编解码顺序或解析。
  - **缓解**：`[vector]` 段紧随 `[embedding]` 段输出（`encodeTOML` `:194-196` 后），`decodeTOML` `switch` 加独立 `case`（不改既有 case），`assignVector` 独立函数（不改 `assignEmbedding`）；TEST-34.2.1 断言既有段 round-trip 不退化 + 既有 TEST-1.2.* 全绿。stop-condition：既有段 round-trip 单测退化则 AC1 不标 `[x]`。
- **R2（中）旧 config.toml（无 `[vector]` 段）兼容**：既有用户的 `config.toml` 无 `[vector]` 段，Load 须得 zero value（不报错）；Save 后新增 `[vector]` 段须语义等同 unset。
  - **缓解**：手写 codec 缺段 ⇒ `Config.Vector` zero value（与 `[embedding]` 缺段一致）；Save 出的 `backend = ""` / `dim = 0` 经 `setVectorEnv` 守卫（`backend != ""` / `dim != 0` 才导出）⇒ 不导出 ⇒ unset ⇒ BruteForce 字节等价。TEST-34.2.1 含「不含 `[vector]` 段 Load 得 zero value」断言。
- **R3（中）env-wins 语义被误实现为 config-wins**：若 `setVectorEnv` 无条件 `os.Setenv` 会让 config 覆盖已显式设的 env（破向后兼容）。
  - **缓解**：`setVectorEnv` 对每个变量先 `os.LookupEnv`，仅 `!hadOld` 时 `os.Setenv`；TEST-34.2.2 含「env 已显式设 ⇒ 不被 config 覆盖」断言（env-wins）。stop-condition：env-wins 单测不过则 AC1 不标 `[x]`。
- **R4（低）测试改进程全局 env 致并行测试串扰**：`setVectorEnv` / 测试 `os.Setenv` 改进程全局环境变量，并行测试可能相互干扰。
  - **缓解**：测试用 `t.Setenv`（Go test 自动恢复 + 标记不可 `t.Parallel`）或显式 restore 闭包恢复；断言后还原；与既有 `setDataDirEnv` 测试同惯例。
- **R5（低）跨进程 env-bridge 被误读为新装配机制**：env-bridge 易被误读为新建的 daemon 配置传递机制，而非复用 `CONTEXTFORGE_DATA_DIR` 已证范式。
  - **缓解**：spec §1 B3 / §5.2 B2-B3 / §5.3 据实记「同 `CONTEXTFORGE_DATA_DIR` 已证范式，非 `daemon.Options` 字段重构 [SPEC-DEFER:phase-future.daemon-options-datadir]」（ADR-013 不夸大）；Rust core 0 toml dep 保依赖面零变化。

## 9. Verification Plan

```bash
# 1. AC1 — Go config [vector] 段 round-trip（含/不含段双向 + 既有段不退化）
go test ./internal/config/...

# 2. AC1 — setVectorEnv env-wins（段存在导出 / env 已设不覆盖 / 无段不导出）
go test ./cmd/contextforge/...

# 3. 不退化（全量 Go + Rust core 0 toml dep 确认）
go test ./...
go vet ./...
cargo test -p contextforge-core
grep -c '^toml ' core/Cargo.toml   # 期望 0（Rust core 不引 toml dep）

# 4. AC2 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.vector-config-file-defer-note]：本 task 仅交付 Go `config.Config` 的 `[vector]` 段（`Backend`/`Dim`）+ `setVectorEnv` 跨进程 env-bridge（镜像 `setDataDirEnv`，env-wins，无段字节等价）+ Rust core 0 toml dep（🟢 Go 侧可单测）；经 `daemon.Options` 结构字段传配置（而非环境变量）[SPEC-DEFER:phase-future.daemon-options-datadir]、Rust core 侧直接读 config.toml（须 toml crate）[SPEC-DEFER:phase-future.rust-core-toml-reader]、vector backend 的 live feature 构建 / 真实 dim 强约束兑现 [SPEC-DEFER:phase-future.vector-dim-feature-enforce]、Go 侧 vector backend 名语义校验 [SPEC-DEFER:phase-future.go-side-vector-name-validate] 均不在本 task 范围。env-bridge 同 `CONTEXTFORGE_DATA_DIR` 已证范式（据实声明，非新装配机制，ADR-013 不夸大）；真实 release tag / run-id / digest（v0.27.0）[SPEC-OWNER:task-34.3-closeout] 实施授权后回填。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Draft

**§9 Verification 计划** (will record real evidence at impl)：
- AC1：`go test ./internal/config/...` —— Go config `[vector]` 段（`backend`/`dim`）含 / 不含段双向 TOML round-trip 恒等 + 不含段 Load 得 zero value + 既有 `[collections]`/`[remote]`/`[embedding]` 段不退化（TEST-34.2.1）；`go test ./cmd/contextforge/...` —— `setVectorEnv` 段存在导出 / env 已显式设不覆盖（env-wins）/ 无段不导出 + restore 闭包恢复（TEST-34.2.2）（真实测试结果待实施回填，ADR-013 不伪造）。
- Rust core 0 toml dep：`grep -c '^toml ' core/Cargo.toml` 期望 0（依赖面零变化，真实结果待实施回填）。
- AC2：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威）。
- 0 新 dep / 默认行为不变（无段 = unset = BruteForce 字节等价，ADR-004） / 既有契约不变 / env-wins 向后兼容 真实结果待实施回填（ADR-013 数值不预填，真实跑出才记）。

**实际改动文件**（计划，待实施回填）：
- `internal/config/config.go`——加 `VectorConfig{Backend string `toml:"backend"`; Dim int `toml:"dim"`}` + `Config.Vector VectorConfig`（与 `Embedding` `:39` 并列）；`encodeTOML`（`:183-204`）加 `[vector]` 段输出（镜像 `[embedding]` `:194-196`）；`decodeTOML`（`:206-264`）加 `case line == "[vector]"` + `assignVector`（镜像 `assignEmbedding` `:322-338`）。
- `cmd/contextforge/main.go`——加 `setVectorEnv(backend string, dim int) (func(), error)`（镜像 `setDataDirEnv` `:254-268`，逐变量 `os.LookupEnv` + `!hadOld` 守卫 `os.Setenv` + restore 闭包，env-wins）。
- `internal/config/config_test.go`——TEST-34.2.1（`[vector]` 段 round-trip + 既有段不退化）。
- `cmd/contextforge/main_test.go`——TEST-34.2.2（`setVectorEnv` env-wins）。
- `docs/decisions/adr-037-*.md` Phase 34 add-only Amendment（dim-negotiation + config-file 完成 Phase 32 起 env-plumbing）落点在 task-34.3 closeout（非本 task body）。
- `core/src/server.rs` / `core/src/retriever/vector/factory.rs` / `core/Cargo.toml` 不改（Rust core 0 toml dep，下游 `resolve_vector_backend` 既有 env 拾取点不动）。
