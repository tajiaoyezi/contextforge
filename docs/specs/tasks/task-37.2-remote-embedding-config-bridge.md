# Task `37.2`: `remote-embedding-config-bridge — Go config.go [remote] 段补 Model 字段（add-only）+ setRemoteEnv 跨进程 env-bridge（镜像 setVectorEnv/setDataDirEnv），把 config.toml 的 [remote] 桥接为 CONTEXTFORGE_REMOTE_ENDPOINT/_MODEL/_PROVIDER 供 spawned core daemon 经既有 factory.rs env 路径拾取；env-wins 覆盖（显式 env 优先，向后兼容）/ 无 [remote] 段 = 不导出 = unset = 默认 provider 行为不变（ADR-004）/ 安全红线：API KEY 永不进 config.toml，仅由用户设 CONTEXTFORGE_REMOTE_API_KEY env（PRD 安全基线）；Rust core 0 toml dep（0 新 dep，ADR-008）`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 37 (embedding-provider-remote-live)
**Dependencies**: 既有 `internal/config/config.go`（task-1.2 手写 TOML 编解码 + `[remote]`/`[embedding]`/`[vector]`/`[[collections]]` 段，Phase 1 起逐步交付；`RemoteProviderConfig{Enabled/Provider/Endpoint}` `:52-56`）/ 既有 `cmd/contextforge/main.go:288-330` `setVectorEnv`（task-34.2 跨进程 env-bridge 范式，已证 spawned core daemon 经 env 拾取）+ `:265-279` `setDataDirEnv`（最初的 env-bridge 范式）/ 既有 `internal/daemon/daemon.go` `launch`（`exec.Command` 继承父进程 env）/ 既有 `core/src/embedding/factory.rs:49-67` `select_provider` `"remote"` arm（task-22.3 已交付，读 `CONTEXTFORGE_REMOTE_ENDPOINT`/`CONTEXTFORGE_REMOTE_MODEL`/`CONTEXTFORGE_REMOTE_PROVIDER`/`CONTEXTFORGE_REMOTE_API_KEY`；`:52` 注 `config plumbing is a follow-up`，本 task 即兑现该 follow-up）/ task-37.1（remote-embedding-live-recall-harness，本 phase 同批；harness 证 real remote 端点端到端召回，config-bridge 把 `[remote]` 配置过桥到同一 factory env 路径）/ ADR-042（embedding-provider-remote-live；本 task = 其 D3 落点）/ ADR-027（embedding-provider-abstraction；本 task = 其 Phase 37 add-only Amendment 落点 @ task-37.3 closeout）/ ADR-004（local-first-privacy-baseline，默认行为 + 既有契约不变；无 `[remote]` 段 / `Enabled=false` = 默认 deterministic provider 行为不变；remote 仍 opt-in）/ ADR-008（dep add-only，Phase 37 = 0 新 dep；Rust core 不引 toml dep）/ ADR-013（禁伪造红线——env-bridge 据实声明同 `CONTEXTFORGE_DATA_DIR`/`CONTEXTFORGE_VECTOR_*` 已证范式，非新 `daemon.Options` 字段重构；API key 仅 env、永不进 config.toml 据实记入安全红线）/ ADR-012（main-agent-governance-autonomy）/ ADR-014 D1-D5（第二十八次激活）

## 1. Background

Phase 22（task-22.3）已让 Rust core 经 `select_provider` 的 `"remote"` arm（`factory.rs:49-67`）从 `CONTEXTFORGE_REMOTE_ENDPOINT` / `CONTEXTFORGE_REMOTE_MODEL` / `CONTEXTFORGE_REMOTE_PROVIDER` / `CONTEXTFORGE_REMOTE_API_KEY` 四个环境变量构造远程 embedding provider，且 `factory.rs:52` 明确记「config plumbing is a follow-up」。task-37.1（harness）已对真实远程端点端到端验证召回（SiliconFlow / Qwen3-Embedding-8B，real vs deterministic 基线对照），但 remote provider 的**配置仍 env-only**——用户无法经持久化的 `config.toml [remote]` 段声明远程 endpoint / model / provider（须每次启动手设环境变量）。本 task 把 remote embedding 的配置补进 Go 侧 `config.toml [remote]` 段并经跨进程 env-bridge 过桥到 core，与既有 `[vector]`（task-34.2 `setVectorEnv`）对齐——它就是 task-34.2 的 embedding 类比：

- **B1 `[remote]` 段缺 `Model` 字段、且无 model/endpoint/provider 的 config→core env 过桥**：`config.toml [remote]` 段经 `internal/config/config.go` 解析，已有 `RemoteProviderConfig{ Enabled / Provider / Endpoint }`（`:52-56`），但**无 `Model` 字段**——而 `factory.rs:55` 的 remote provider 构造需 `CONTEXTFORGE_REMOTE_MODEL`。用户在 `config.toml` 里没有声明远程 model 的落点，且 `[remote]` 段的 `Provider`/`Endpoint` 当前也未被桥接为 core 的 `CONTEXTFORGE_REMOTE_*` 环境变量（Go 侧只读 `Enabled` 控本进程 opt-in，未把 endpoint/model/provider 过桥给 spawned core daemon）。
- **B2 Rust core 无 toml dep（不能在 Rust 侧读 config.toml）**：Rust core 持 `serde` / `serde_json` 但**无 `toml` 依赖**——在 Rust 侧加 config.toml reader 须引 `toml` crate，破 0 新 dep（ADR-008）。故配置桥接须在已解析 `config.toml` 的 **Go 侧**完成，不动 Rust core 依赖面；core 经既有 `factory.rs:54-59` env 读取路径接收（本 task 不改 `factory.rs`）。
- **B3 已证的跨进程 env-bridge 范式（`CONTEXTFORGE_DATA_DIR` / `CONTEXTFORGE_VECTOR_*`）**：`cmd/contextforge/main.go:265-279` 的 `setDataDirEnv` 与 `:288-330` 的 `setVectorEnv`（task-34.2）把配置经 `os.Setenv(...)` 导出为父进程环境变量；spawned core daemon 经 `internal/daemon/daemon.go` `launch` 的 `exec.Command`（继承父进程 env）启动，于 Rust 侧经 env 读取等价路径拾取。本 task **复用同一已证范式**——加一个 `setRemoteEnv`（镜像 `setVectorEnv`），把 `[remote]` 段桥接为 `CONTEXTFORGE_REMOTE_ENDPOINT` / `CONTEXTFORGE_REMOTE_MODEL` / `CONTEXTFORGE_REMOTE_PROVIDER`，spawned core daemon 经既有 `select_provider` remote arm（`factory.rs:54-59`）env 路径拾取。**这是同一跨进程 env-bridge，不是延后的 `daemon.Options` 字段重构**（后者 [SPEC-DEFER:phase-future.daemon-options-datadir]）。
- **B4 env-wins 向后兼容 + 无 `[remote]` 段 = 默认 provider 行为不变**：显式设置的环境变量**优先于** config 文件（与 `setVectorEnv` / `setDataDirEnv` 的「env 已设则不覆盖」语义一致）——保证既有 env-only 工作流向后兼容。无 `[remote]` 段（或字段为空）⇒ 不导出任何变量 ⇒ 三个环境变量保持调用方原状 ⇒ 默认 deterministic provider 行为不变（ADR-004，remote 仍 opt-in）。
- **B5 安全红线：API KEY 永不进 config.toml，仅 env**：`CONTEXTFORGE_REMOTE_API_KEY` 是密钥，**永不**写入 `config.toml`、**永不**由 `setRemoteEnv` 处理或导出——`setRemoteEnv` 只过桥非密钥的 endpoint / model / provider；密钥仅由用户显式设 `CONTEXTFORGE_REMOTE_API_KEY` 环境变量，core 经 `factory.rs:59` 读取且永不记录（PRD 安全基线 / ADR-004 opt-in / ADR-013 据实记安全红线）。

本 task 为 code-local 🟢 可单测（Go config TOML round-trip + `setRemoteEnv` env-wins 单测），0 新 dep（沿用 task-1.2 手写 TOML codec + 既有 `os` 标准库）；Rust core 依赖面零变化（不引 toml dep，不改 `factory.rs`）。

## 2. Goal

(1) **B1**：为 Go `RemoteProviderConfig` add-only 补 `Model string`（toml `model`）字段（与既有 `Enabled` / `Provider` / `Endpoint` 并列），经既有手写 TOML codec（`encodeTOML` `:196-219` / `decodeTOML` `:222-287` / `assignRemote` `:321-343`）对称编解码（toml round-trip 保真；absent ⇒ zero value）。(2) **B2/B3**：加 `setRemoteEnv` helper（镜像 `setVectorEnv` `:288-330`）——当 `[remote]` 段对应字段非空 **且对应环境变量未显式设置**时，`os.Setenv` 导出 `CONTEXTFORGE_REMOTE_ENDPOINT`（取 `Endpoint`）/ `CONTEXTFORGE_REMOTE_MODEL`（取 `Model`）/ `CONTEXTFORGE_REMOTE_PROVIDER`（取 `Provider`），供 spawned core daemon 经既有 `select_provider` remote arm（`factory.rs:54-59`）env 路径拾取；返回 restore 闭包（镜像 `setVectorEnv` 返回 `func()`）。(3) **B4**：env-wins——显式已设的环境变量不被 config 覆盖（向后兼容）；无 `[remote]` 段 / 字段空 ⇒ 不导出 ⇒ 默认 provider 行为不变（ADR-004）。(4) **B5 安全红线**：`setRemoteEnv` 不处理 / 不导出 `CONTEXTFORGE_REMOTE_API_KEY`；API KEY 永不进 `config.toml`，仅由用户设 env（PRD 安全基线）。(5) **0 dep**：Go 侧沿用手写 TOML codec + `os` 标准库；Rust core **不引 toml dep**（0 新 dep，ADR-008），不改 `factory.rs`。

pass bar：Go config `[remote]` 段 `Model` 字段 TOML round-trip 经确定性单测验证（含 / 不含 `model` 双向；既有 `Enabled`/`Provider`/`Endpoint` + 既有 `[embedding]`/`[vector]`/`[[collections]]` 段不受影响）（🟢）；`setRemoteEnv` env-wins 经单测验证（字段非空 ⇒ env 导出；env 已显式设 ⇒ 不覆盖（env-wins）；字段空 ⇒ 不导出；**API key 永不被 `setRemoteEnv` 导出**）（🟢）；Rust core 0 toml dep（依赖面零变化，`factory.rs` 不改）；无 `[remote]` 段 = 默认 provider 行为不变（ADR-004） + 既有契约（`config.Config` 既有字段 / `daemon.Options` / `select_provider`）不变；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `internal/config/config.go`——`RemoteProviderConfig` add-only 补 `Model string`（toml 标签 `model`，doc 注：`""` ⇒ unset ⇒ core 默认 model；**API key 不在此结构、永不进 config.toml，仅 env**），与既有 `Enabled` / `Provider` / `Endpoint`（`:52-56`）并列。
- `encodeTOML`（`:196-219`）`[remote]` 段加 `model = "..."` 输出行（紧随既有 `endpoint` 行 `:206`，镜像 `provider`/`endpoint` 行）；`assignRemote`（`:321-343`）`switch` 加 `case "model"`（走 `parseTOMLString`，镜像 `case "endpoint"`）；`decodeTOML` `[remote]` 段头分派（`:232-234`）不变（既有 `case line == "[remote]"`）。
- 加 `setRemoteEnv` helper 于 `cmd/contextforge/main.go`（镜像 `setVectorEnv` `:288-330`）——best-effort `config.Load(dataDir)`（load 失败非致命，missing config 静默、parse/read 真失败 stderr WARN，镜像 `setVectorEnv` task-35.1 行为），逐变量对 `CONTEXTFORGE_REMOTE_ENDPOINT`（取 `Remote.Endpoint`）/ `CONTEXTFORGE_REMOTE_MODEL`（取 `Remote.Model`）/ `CONTEXTFORGE_REMOTE_PROVIDER`（取 `Remote.Provider`）经 `setIfAbsent`（env-wins 守卫：非空 + env 未设才 `os.Setenv`）导出 + 返回 restore 闭包。**不处理 `CONTEXTFORGE_REMOTE_API_KEY`**（安全红线）。
- 接线：`setRemoteEnv` 接入 `doServe`（`:108` `setVectorEnv` 后）+ `doMCP`（`:142` `setVectorEnv` 后）两 daemon-up 路径（镜像 `setVectorEnv` 接线点）。
- 同源测试：`internal/config` 同包 test（`[remote] model` 段 TOML round-trip + 既有字段/段不受影响）+ `cmd/contextforge` 同包 test（`setRemoteEnv` env-wins：字段非空导出 / env 已设不覆盖 / 字段空不导出 / **API key 永不被导出**）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- 把 remote 配置经 `daemon.Options` 结构字段（而非环境变量）传给 spawned core daemon [SPEC-DEFER:phase-future.daemon-options-datadir]——本 task 复用已证的跨进程 env-bridge（同 `CONTEXTFORGE_DATA_DIR` / `CONTEXTFORGE_VECTOR_*` 范式），不引入 `daemon.Options` 字段重构（该重构属另一延后边界，本 task 不依赖、不实现）。
- 在 Rust core 侧直接读 `config.toml`（须引 `toml` crate）[SPEC-DEFER:phase-future.rust-core-toml-reader]——破 0 新 dep（ADR-008），故配置桥接限于已解析 `config.toml` 的 Go 侧 + 经环境变量过桥，Rust core 依赖面不动、`factory.rs` 不改。
- 把 API key 经 config.toml 落盘或经 `setRemoteEnv` 过桥 [SPEC-DEFER:phase-future.remote-api-key-secret-store]——安全红线：API key 永不进 config.toml、永不由 `setRemoteEnv` 处理，仅由用户设 `CONTEXTFORGE_REMOTE_API_KEY` env（PRD 安全基线）；密钥的专用 secret-store / keyring 集成不在本 task 范围。
- config.toml 的 `[remote]` 段语义校验（未知 provider 名 / 非法 endpoint URL 在 Go 侧拒绝 / 提示）——本 task 仅做桥接，provider 名 / endpoint 的合法性由 Rust core `select_provider`（`factory.rs:27-83`，未知名 explicit error）+ 远程端点 round-trip 权威裁决，Go 侧透传，honest-defer 不在 Go 侧重复校验逻辑 [SPEC-DEFER:phase-future.go-side-remote-name-validate]。
- remote provider 的大语料语义召回质量断言 [SPEC-DEFER:phase-future.embedding-large-corpus-recall]（task-37.1 §10 已记）——本 task 仅做配置桥接，召回数值由 task-37.1 的作者标注小集 harness 据实测得（诚实小集 caveat 见 task-37.1）。
- 真实 release tag / run-id / digest（v0.30.0）[SPEC-OWNER:task-37.3-closeout]（ADR-012 用户授权后回填）。

## 4. Actors

- 主 agent（ADR-012 自治）
- `RemoteProviderConfig`（`internal/config/config.go:52-56`，本 task add-only 补 `Model string` 字段，与 `Endpoint` 并列）
- `config.Config`（`internal/config/config.go:31-41`，既有 `Remote RemoteProviderConfig` 字段 `:38`，本 task 不改根结构）
- `encodeTOML` / `assignRemote`（`internal/config/config.go:196-219` / `:321-343`，本 task `[remote]` 段加 `model` 行对称编解码，镜像 `endpoint` 路径）
- `setRemoteEnv`（新 helper 于 `cmd/contextforge/main.go`，镜像 `setVectorEnv` `:288-330`，跨进程 env-bridge；**不处理 API key**）
- spawned core daemon（`internal/daemon/daemon.go` `launch` `exec.Command` 继承父进程 env）+ `select_provider` remote arm（`core/src/embedding/factory.rs:49-67`，既有 env 拾取点，本 task 不改）
- 运维 / 部署者（经 `config.toml [remote]` 声明远程 endpoint / model / provider，无须每次手设环境变量；API key 仍仅由用户设 `CONTEXTFORGE_REMOTE_API_KEY` env）

## 5. Behavior Contract

### 5.1 Required Reading

- `internal/config/config.go:50-56`（`RemoteProviderConfig` `Enabled` / `Provider` / `Endpoint` + doc——`Model` 是 add-only 补字段的并列点）+ `:31-41`（`Config` 根结构 `Remote` 字段 `:38`）
- `internal/config/config.go:196-219`（`encodeTOML`——`:203-206` `[remote]` 段输出 `enabled`/`provider`/`endpoint`，`model` 行紧随 `endpoint` 镜像）+ `:222-287`（`decodeTOML`——`:232-234` `case line == "[remote]"` 段头分派，本 task 不改头）+ `:321-343`（`assignRemote`——`case "endpoint"` 走 `parseTOMLString`，`case "model"` 镜像）
- `cmd/contextforge/main.go:288-330`（`setVectorEnv`——best-effort `config.Load` + missing 静默 / parse 失败 stderr WARN + `setIfAbsent`（env-wins 守卫）+ 返回 restore 闭包；`setRemoteEnv` 镜像此范式 + env-wins 语义 + **API key 不处理** 安全红线）+ `:265-279`（`setDataDirEnv` 最初范式）+ `:108-109` / `:142-143`（`setVectorEnv` 在 doServe / doMCP 的接线点——`setRemoteEnv` 镜像接线）
- `internal/daemon/daemon.go` `launch`（`exec.Command` 继承父进程 env——env-bridge 跨进程生效的依据）
- `core/src/embedding/factory.rs:49-67`（`select_provider` remote arm 读 `CONTEXTFORGE_REMOTE_ENDPOINT` `:54` / `CONTEXTFORGE_REMOTE_MODEL` `:55` / `CONTEXTFORGE_REMOTE_PROVIDER` `:57` / `CONTEXTFORGE_REMOTE_API_KEY` `:59`，`:52` 注 `config plumbing is a follow-up`，`:53` 注 `api_key is read here and never logged`）——本 task 桥接的下游拾取点，不改
- `core/tests/remote_embedding_recall.rs`（task-37.1 harness——同一 factory env 路径的 real-vs-deterministic 召回验证；config-bridge 把 `[remote]` 配置过桥到同一路径）
- `docs/decisions/adr-042-embedding-provider-remote-live.md §D3`（本 task 即其原文实现）+ `docs/decisions/adr-027-embedding-provider-abstraction.md`（本 task = 其 Phase 37 add-only Amendment 落点 @ task-37.3 closeout）+ ADR-004（默认 deterministic provider 不变 / remote opt-in）/ ADR-008（Rust core 0 toml dep）

### 5.2 关键设计 — Go [remote] Model add-only + setRemoteEnv 跨进程 env-bridge（env-wins / API key env-only / Rust 0-dep）

- **B1 Go `[remote]` 段 `Model` add-only（镜像 `Endpoint`）**：`RemoteProviderConfig` 加 `Model string`（toml 标签 `model`），与既有 `Enabled` / `Provider` / `Endpoint` 并列；`encodeTOML`（`:196-219`）`[remote]` 段输出 `model = "..."`（紧随 `endpoint` `:206`，镜像 `provider`/`endpoint` 行）；`assignRemote`（`:321-343`）加 `case "model"`（`model`→`parseTOMLString`，镜像 `case "endpoint"`）；`decodeTOML` 段头分派 `case line == "[remote]"`（`:232-234`）不变。**absent `model` 行 ⇒ `Remote.Model` 为 zero value（`""`）**，与既有 `[remote]` 缺字段行为一致（task-1.2 手写 codec 容忍缺字段）。
- **B2/B3 `setRemoteEnv` 跨进程 env-bridge（镜像 `setVectorEnv`）**：在 `cmd/contextforge/main.go` 加 `setRemoteEnv(dataDir string) func()`，best-effort `config.Load(dataDir)`（load 失败非致命；missing config 静默、`errors.Is(err, os.ErrNotExist)` 守护；real parse/read 失败 stderr WARN，镜像 `setVectorEnv` task-35.1 行为）；逐变量复用 `setIfAbsent`（env-wins 守卫）——对 `CONTEXTFORGE_REMOTE_ENDPOINT`（取 `cfg.Remote.Endpoint`）/ `CONTEXTFORGE_REMOTE_MODEL`（取 `cfg.Remote.Model`）/ `CONTEXTFORGE_REMOTE_PROVIDER`（取 `cfg.Remote.Provider`）：**仅当值非空且 env 未显式设（`!hadOld`）时** `os.Setenv`；返回 restore 闭包（恢复每个变量的旧值 / unset，镜像 `setVectorEnv` 返回 `func()`）。spawned core daemon 经 `daemon.go` `exec.Command`（继承父进程 env）启动 → Rust 侧 `select_provider` remote arm（`factory.rs:54-59`）拾取，与 `CONTEXTFORGE_DATA_DIR` / `CONTEXTFORGE_VECTOR_*` 同跨进程范式。接线 `doServe`（`:108` `setVectorEnv` 后）+ `doMCP`（`:142` `setVectorEnv` 后）。
- **B4 env-wins（向后兼容）**：`setRemoteEnv` 对每个变量先 `os.LookupEnv` 探测；**若该变量已显式设置（`hadOld == true`）则不 `os.Setenv` 覆盖**——显式 env 优先于 config 文件（既有 env-only 工作流向后兼容）。这是与 `setVectorEnv` `setIfAbsent`（`!had` 才 Setenv）一致的「env-wins」实现。
- **B5 安全红线：API KEY 永不进 config.toml / 仅 env**：`setRemoteEnv` **只过桥** `CONTEXTFORGE_REMOTE_ENDPOINT` / `CONTEXTFORGE_REMOTE_MODEL` / `CONTEXTFORGE_REMOTE_PROVIDER` 三个非密钥变量；**绝不处理 / 绝不导出 `CONTEXTFORGE_REMOTE_API_KEY`**——`RemoteProviderConfig` 不含 API key 字段、`encodeTOML` 不写 API key、`config.toml` 永不含密钥。API key 仅由用户显式设 `CONTEXTFORGE_REMOTE_API_KEY` env，core 经 `factory.rs:59` 读取且永不记录（`factory.rs:53` 注 `never logged`）。PRD 安全基线 / ADR-004 opt-in / ADR-013 据实记安全红线（[SPEC-DEFER:phase-future.remote-api-key-secret-store] 专用 secret-store 不在本 task）。
- **无 `[remote]` 段 / 字段空 = 默认 provider 行为不变（ADR-004）**：absent `[remote]` 段 / 字段空 ⇒ `cfg.Remote.{Endpoint,Model,Provider}` 为 `""` ⇒ `setRemoteEnv` 三变量均不 `os.Setenv`（`val == ""` 守卫）⇒ 三环境变量保持调用方原状 ⇒ `select_provider` 默认仍走 deterministic（remote 须显式 `EmbeddingConfig.Provider == "remote"` + `--features embedding-remote` 才激活，仍 opt-in）。默认 build 行为不变。
- **Rust core 0 toml dep（ADR-008）+ `factory.rs` 不改**：配置桥接全程在 Go 侧（已解析 `config.toml`）+ 环境变量过桥完成；Rust core **不引 `toml` crate**（依赖面零变化）；下游拾取点 `select_provider` remote arm（`factory.rs:54-59`）为 task-22.3 既有代码，本 task 不改（兑现 `factory.rs:52` 的 `config plumbing is a follow-up` 注，无须改 Rust 读取逻辑）。
- **非 `daemon.Options` 字段重构**：本 task 复用已证的跨进程 env-bridge（同 `CONTEXTFORGE_DATA_DIR` / `CONTEXTFORGE_VECTOR_*`），**不**引入 `daemon.Options` 结构字段把配置经 `daemon.Options` 传入 [SPEC-DEFER:phase-future.daemon-options-datadir]（该重构属另一延后边界，本 task 不依赖、不实现，ADR-013 据实声明用已证范式而非新建结构）。

### 5.3 不变量

- 默认行为不变（ADR-004）：无 `[remote]` 段 / 字段空 ⇒ `cfg.Remote.{Endpoint,Model,Provider}` 为 `""` ⇒ `setRemoteEnv` 不导出 ⇒ 环境变量保持调用方原状 ⇒ `select_provider` 默认仍 deterministic（remote 须显式 opt-in + feature-gated）；既有 `config.toml`（无 `model` 行的旧文件）Load 后 `Remote.Model` 为 `""`，Save 后 `[remote]` 段新增 `model = ""` 行（语义等同 unset）。
- 既有契约不变：`config.Config` 既有字段（`SchemaVersion` / `DataDir` / `Denylist` / `Collections` / `Remote`（既有 `Enabled`/`Provider`/`Endpoint`）/ `Embedding` / `Vector`）+ 既有段（`[remote]` 既有字段 / `[embedding]` / `[vector]` / `[[collections]]`）编解码不变（add-only `Model` 字段 + `model` 行，既有 round-trip 不退化）；`daemon.Options` 结构 / `daemon.go` `launch` 不变；`select_provider`（`factory.rs:27-83`）契约 + 依赖面不动。
- env-wins（向后兼容）：显式设置的 `CONTEXTFORGE_REMOTE_ENDPOINT` / `CONTEXTFORGE_REMOTE_MODEL` / `CONTEXTFORGE_REMOTE_PROVIDER` 优先于 `config.toml [remote]`——`setRemoteEnv` 仅在变量未设（`!had`）时导出，既有 env-only 工作流行为不变。
- 安全红线据实（ADR-013）：API key 永不进 `config.toml`、永不由 `setRemoteEnv` 处理 / 导出；`CONTEXTFORGE_REMOTE_API_KEY` 仅由用户显式设 env，core 经 `factory.rs:59` 读取且永不记录（PRD 安全基线 / ADR-004 opt-in）。
- 0 新代码依赖（ADR-008）：Go 侧沿用 task-1.2 手写 TOML codec + `os` / `errors` 标准库，无第三方依赖增量；**Rust core 不引 `toml` crate**（依赖面零变化），`factory.rs` 不改。
- 跨进程范式据实（ADR-013）：env-bridge 同 `CONTEXTFORGE_DATA_DIR` / `CONTEXTFORGE_VECTOR_*` 已证范式（`setVectorEnv` / `setDataDirEnv` + `exec.Command` 继承 env），非延后的 `daemon.Options` 字段重构 [SPEC-DEFER:phase-future.daemon-options-datadir]；据实声明用已证范式，不夸大为新装配机制。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [ ] **AC1**（Go config `[remote] model` 段 TOML round-trip + `setRemoteEnv` env-wins + API key 安全红线 🟢）: `RemoteProviderConfig` add-only `Model string` 字段；`encodeTOML`/`assignRemote` 对称编解码 `model` 行（含 / 不含 `model` 双向 round-trip 恒等），既有 `[remote]` 既有字段 / `[embedding]`/`[vector]`/`[[collections]]` 段不受影响（TEST-1.2.* / TEST-34.2.* 不退化）；`setRemoteEnv`：字段非空（endpoint/model/provider）⇒ `CONTEXTFORGE_REMOTE_ENDPOINT`/`_MODEL`/`_PROVIDER` 经 `os.Setenv` 导出；env 已显式设 ⇒ 不覆盖（env-wins）；字段空 ⇒ 不导出；restore 闭包恢复旧值；**`CONTEXTFORGE_REMOTE_API_KEY` 永不被 `setRemoteEnv` 导出（安全红线）**；**Rust core 0 toml dep + `factory.rs` 不改** — verified by **TEST-37.2.1**（config round-trip）+ **TEST-37.2.2**（setRemoteEnv env-wins + API key never-exported）
- [ ] **AC2**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-37.2.3**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-37.2.1 | Go config `[remote] model` 段 TOML round-trip：含 `model` Save→Load 恒等 + 不含 `model` 行 Load 得 zero value（`Model ""`）+ 既有 `[remote]` `Enabled`/`Provider`/`Endpoint` + `[embedding]`/`[vector]`/`[[collections]]` 段编解码不受影响（不退化） | `internal/config/config_test.go`（同包 test） | Draft |
| TEST-37.2.2 | `setRemoteEnv` env-wins + 安全红线：字段非空（endpoint/model/provider）⇒ `CONTEXTFORGE_REMOTE_ENDPOINT`/`_MODEL`/`_PROVIDER` 经 `os.Setenv` 导出（spawned core daemon 经 env 路径拾取）；env 已显式设 ⇒ 不被 config 覆盖（env-wins）；字段空 ⇒ 不导出；restore 闭包恢复旧值 / unset；**`CONTEXTFORGE_REMOTE_API_KEY` 永不被 `setRemoteEnv` 导出 / 处理（API key env-only 安全红线）** | `cmd/contextforge/main_test.go`（同包 test） | Draft |
| TEST-37.2.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Draft |

## 8. Risks

- **R1（中）`model` 行加入破既有 config.toml round-trip**：加 `[remote] model` 行输出 / 解析可能影响既有 `[remote]` 既有字段或 `[embedding]`/`[vector]`/`[[collections]]` 段的编解码顺序或解析。
  - **缓解**：`model` 行紧随 `[remote]` 段 `endpoint` 行（`encodeTOML` `:206` 后）输出，`assignRemote` 加独立 `case "model"`（不改既有 `case`），段头分派不动；TEST-37.2.1 断言既有段 / 既有 `[remote]` 字段 round-trip 不退化 + 既有 TEST-1.2.* / TEST-34.2.* 全绿。stop-condition：既有段 round-trip 单测退化则 AC1 不标 `[x]`。
- **R2（中）旧 config.toml（无 `model` 行）兼容**：既有用户的 `config.toml [remote]` 段无 `model` 行，Load 须得 zero value（不报错）；Save 后 `[remote]` 段新增 `model = ""` 行须语义等同 unset。
  - **缓解**：手写 codec 缺字段 ⇒ `Remote.Model` zero value（与既有缺字段一致）；Save 出的 `model = ""` 经 `setRemoteEnv` 守卫（`val != ""` 才导出）⇒ 不导出 ⇒ 默认行为不变。TEST-37.2.1 含「不含 `model` 行 Load 得 zero value」断言。
- **R3（中）env-wins 语义被误实现为 config-wins**：若 `setRemoteEnv` 无条件 `os.Setenv` 会让 config 覆盖已显式设的 env（破向后兼容）。
  - **缓解**：`setRemoteEnv` 对每个变量先 `os.LookupEnv`，仅 `!had` 时 `os.Setenv`（复用 `setVectorEnv` `setIfAbsent` 范式）；TEST-37.2.2 含「env 已显式设 ⇒ 不被 config 覆盖」断言（env-wins）。stop-condition：env-wins 单测不过则 AC1 不标 `[x]`。
- **R4（高）API key 误进 config.toml / 误被 setRemoteEnv 导出（安全红线破）**：若给 `RemoteProviderConfig` 加 API key 字段或让 `setRemoteEnv` 处理 `CONTEXTFORGE_REMOTE_API_KEY`，密钥会落盘 / 被 Go 侧搬运，破 PRD 安全基线。
  - **缓解**：`RemoteProviderConfig` **不含** API key 字段（`encodeTOML` 永不写密钥）；`setRemoteEnv` 仅过桥 endpoint/model/provider 三个非密钥变量、**绝不**触及 `CONTEXTFORGE_REMOTE_API_KEY`；TEST-37.2.2 含「`CONTEXTFORGE_REMOTE_API_KEY` 永不被 `setRemoteEnv` 导出 / 处理」断言（设一个哨兵 env 值，调用后断言其未被 `setRemoteEnv` 改动且无新 API-key env 被设）。stop-condition：API-key-never-exported 单测不过则 AC1 不标 `[x]`。
- **R5（低）测试改进程全局 env 致并行测试串扰**：`setRemoteEnv` / 测试 `os.Setenv` 改进程全局环境变量，并行测试可能相互干扰。
  - **缓解**：测试用 `t.Setenv`（Go test 自动恢复 + 标记不可 `t.Parallel`）或显式 restore 闭包恢复；断言后还原；与既有 `setVectorEnv` / `setDataDirEnv` 测试同惯例。
- **R6（低）跨进程 env-bridge 被误读为新装配机制**：env-bridge 易被误读为新建的 daemon 配置传递机制，而非复用 `CONTEXTFORGE_DATA_DIR` / `CONTEXTFORGE_VECTOR_*` 已证范式。
  - **缓解**：spec §1 B3 / §5.2 B2-B3 / §5.3 据实记「同 `CONTEXTFORGE_DATA_DIR`/`CONTEXTFORGE_VECTOR_*` 已证范式，非 `daemon.Options` 字段重构 [SPEC-DEFER:phase-future.daemon-options-datadir]」（ADR-013 不夸大）；Rust core 0 toml dep + `factory.rs` 不改保依赖面零变化。

## 9. Verification Plan

```bash
# 1. AC1 — Go config [remote] model 段 round-trip（含/不含 model 双向 + 既有段不退化）
go test ./internal/config/...

# 2. AC1 — setRemoteEnv env-wins + API key never-exported（字段非空导出 / env 已设不覆盖 / 字段空不导出 / API key 永不被导出）
go test ./cmd/contextforge/...

# 3. 不退化（全量 Go + Rust core 0 toml dep + factory.rs 不改 确认）
go test ./...
go vet ./...
cargo test -p contextforge-core
grep -c '^toml ' core/Cargo.toml   # 期望 0（Rust core 不引 toml dep）

# 4. AC2 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.remote-config-bridge-defer-note]：本 task 仅交付 Go `RemoteProviderConfig` 的 `Model` add-only 字段（`[remote] model` 行）+ `setRemoteEnv` 跨进程 env-bridge（镜像 `setVectorEnv`，env-wins，无段默认不变，API key env-only 安全红线）+ Rust core 0 toml dep / `factory.rs` 不改（🟢 Go 侧可单测）；经 `daemon.Options` 结构字段传配置（而非环境变量）[SPEC-DEFER:phase-future.daemon-options-datadir]、Rust core 侧直接读 config.toml（须 toml crate）[SPEC-DEFER:phase-future.rust-core-toml-reader]、API key 专用 secret-store / keyring [SPEC-DEFER:phase-future.remote-api-key-secret-store]、Go 侧 remote provider 名 / endpoint 语义校验 [SPEC-DEFER:phase-future.go-side-remote-name-validate]、大语料语义召回质量断言 [SPEC-DEFER:phase-future.embedding-large-corpus-recall] 均不在本 task 范围。env-bridge 同 `CONTEXTFORGE_DATA_DIR`/`CONTEXTFORGE_VECTOR_*` 已证范式（据实声明，非新装配机制，ADR-013 不夸大）；真实 release tag / run-id / digest（v0.30.0）[SPEC-OWNER:task-37.3-closeout] 实施授权后回填。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Draft（实施 + 真实验收在 impl PR；本节实施后回填 real evidence）

实施后须回填：
- AC1：`go test ./internal/config/ -run TestTask372`（TEST-37.2.1：`[remote] model` round-trip + 既有段不退化 + 不含 `model` 行 legacy Load 得 zero value）+ `go test ./cmd/contextforge/ -run TestSetRemoteEnv`（TEST-37.2.2：字段非空导出 + restore unset / 显式 env 不被覆盖 env-wins / 字段空不导出 / **API key 永不被导出** 安全红线）真实结果。
- Rust core 0 toml dep：`grep -c '^toml ' core/Cargo.toml` = 0（依赖面零变化，`factory.rs` 不改，兑现 `factory.rs:52` 的 follow-up 注）。
- AC2：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威）。
- 不退化：`go test ./...` 全 PASS + `go vet ./...` clean + `cargo test -p contextforge-core` 不受影响（无 Rust 改动）。
- 0 新 dep / 默认行为不变（无 `[remote]` 段 / 字段空 = 不导出 = 默认 deterministic provider，ADR-004，remote 仍 opt-in） / 既有契约不变 / env-wins 向后兼容（显式 env 覆盖 config file） / **安全红线（API key 永不进 config.toml、永不由 setRemoteEnv 处理，仅 env）**。

**预期改动文件**（实施后据实回填）：
- `internal/config/config.go`——`RemoteProviderConfig` 加 `Model string`（toml `model`）；`encodeTOML` `[remote]` 段加 `model` 行（紧随 `endpoint`）；`assignRemote` 加 `case "model"`（镜像 `case "endpoint"`）。
- `cmd/contextforge/main.go`——加 `setRemoteEnv(dataDir string) func()`（best-effort `config.Load(dataDir)` → 逐变量 `setIfAbsent`（env-wins 守卫）导出 endpoint/model/provider + restore 闭包，镜像 `setVectorEnv`；**不处理 API key**）；接线 doServe（`setVectorEnv` 后）+ doMCP（`setVectorEnv` 后）。
- `internal/config/config_test.go`——TEST-37.2.1（`[remote] model` 段 round-trip + 既有段不退化）。
- `cmd/contextforge/main_test.go`——TEST-37.2.2（`setRemoteEnv` env-wins + API key never-exported 安全红线）。
- `docs/decisions/adr-027-*.md` Phase 37 add-only Amendment（embedding-provider-remote 真实联调 + 真实召回兑现）落点在 task-37.3 closeout（非本 task body）。
- `core/src/embedding/factory.rs` / `core/Cargo.toml` 不改（Rust core 0 toml dep，下游 `select_provider` remote arm 既有 env 拾取点不动；兑现 `factory.rs:52` follow-up 注）。
