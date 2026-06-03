# ADR `039`: `vector-config-completeness`

**Status**: Proposed（v0.27.0 / task-34.3 closeout 据真实 CI 逐 D ratify Proposed→Accepted；D1/D2 code-local 🟢，D3 get_source_chunk 隔离 already-present verify-only 🟢 grounding-correction，D4 默认 / 0-dep / 0-network / 既有契约不变 🟢；dim-declaring feature backend 的 live dim-enforce 🟡 honest-defer——见 §Ratification）

**Category**: 向量后端配置补全（dim 协商 + config-file 桥接）/ 跨进程 env 接线 / 契约诚实化（grounding-correction）
**Date**: 2026-06-03
**Decided By**: 主 agent（ADR-012 自治）；tajiaoyezi ratification at v0.27.0 closeout
**Related**: ADR-037（vector-backend-config-plumbing-and-completeness — 本 ADR 为其 Phase 34 的收尾补全：Phase 32 起的 env-plumbing 在 dim 协商 + config-file 两点上仍未闭合，本 phase 据实补齐；以 add-only Phase 34 Amendment 记，不溯改其正文，ADR-014 D5）/ ADR-034（vector-store-composition-and-honest-defer — select_vector_backend 工厂 + BruteForce 默认 byte-equiv，本 phase 在工厂处补 dim 协商 seam）/ ADR-023（vector-persistence — dim-declaring feature backend(qdrant/lancedb/sqlite-vec) 才真正约束 dim，默认 BruteForce dim-agnostic）/ ADR-027（embedding-provider-selection — 本 phase 的 `negotiate_vector_dim` 镜像其 `negotiate_dim`(embedding/factory.rs:88-96) 的纯函数协商语义）/ ADR-016（workspace-isolation — get_source_chunk 的 workspace 隔离方向，本 phase 据实校正其「已落」）/ ADR-004（local-first-privacy-baseline — 默认行为 / proto / 既有契约不变 + 0 网络 + BruteForce 默认 dim-agnostic byte-equiv）/ ADR-008（dep add-only — Phase 34 = 0 新依赖；Rust core 无 toml dep，config-file 经 Go→env 桥接而非 Rust 端 toml reader）/ ADR-013（禁伪造红线 — dim-declaring feature backend 的 live dim-enforce 须 feature build 后真实跑出再回填，不预填；get_source_chunk 隔离据实校正为 already-present、不重实现）/ ADR-012（main-agent-governance-autonomy — tag/release outward-facing 须用户显式授权，v0.27.0 本轮已授权）/ ADR-014（D1-D4，第二十五次激活）/ roadmap §3.16 + §4

## Context

ContextForge 截至 Phase 33（governance-debt-cleanup-2, Done / v0.26.0）已完成第二轮治理债清扫。本 Phase 34 是一个**刻意精简**的版本——Phase 31 / 33 双轮清扫后绿色 backlog 已薄，据 ADR-013 取诚实优先于凑量：仅收口 Phase 32 起 vector-backend 配置故事在 dim 协商 + config-file 两点上的真实缺口，外加一处 grounding 校正（survey 高估）。逐维度调研结论（grounded）：

- **`CONTEXTFORGE_VECTOR_DIM` 被工厂静默丢弃（REAL gap）**：`server.rs` `resolve_vector_backend`（:540）解析 env `CONTEXTFORGE_VECTOR_DIM` 并把 dim 传入 `select_vector_backend(name, dim)`（`core/src/retriever/vector/factory.rs:33-39`），但工厂体内 `let _ = dim;`（:39）**直接丢弃**——配置的 dim 从未参与任何协商。对比 embedding 侧 `core/src/embedding/factory.rs:81-96` 在 `select_provider` 末尾调 `negotiate_dim(provider.dim(), dim)`，`requested != 0 && provider_dim != requested` 时返回 `DimMismatch{expected,got}`。vector 侧的 `VectorError::DimMismatch{expected,got}` **已存在**（`core/src/retriever/vector/types.rs:83`）却无人产出。这是 embedding/vector 两条 factory 的对称缺口——embedding 协商 dim、vector 丢弃 dim。

- **vector backend 配置仅 env、无 config-file 路径（REAL gap）**：`CONTEXTFORGE_VECTOR_BACKEND` / `CONTEXTFORGE_VECTOR_DIM` 仅由 Rust `server.rs` 读 env。Rust core 有 serde / serde_json 但**无 toml dep**——在 Rust 端写 config reader 会破 0-dep（ADR-008）。而 Go `internal/config/config.go` 已解析 `config.toml`（含 `[collections]` / `[remote]` / `[embedding]` sections），且 spawned Rust daemon **继承 Go 进程 env**（`internal/daemon/daemon.go:202` `exec.Command` 继承 env；`cmd/contextforge/main.go:255` `setDataDirEnv` 以同样方式导出 `CONTEXTFORGE_DATA_DIR`）。故 config-file → core 的最干净桥接是 Go 端加 `[vector]` section + 一个 `setVectorEnv`（镜像 `setDataDirEnv`）跨进程 env-bridge，复用既有 `resolve_vector_backend` env 路径，Rust core 保持 0-dep。

- **`get_source_chunk` workspace 隔离 = HONEST 非问题（already-present，grounding 校正 / verify-only）**：survey 把 `get_source_chunk` 的 workspace 隔离记为 gap，但 grounding 复核 `core/src/data_plane/search.rs:421-423` 自 **task-12.2（ADR-016/017 D1 Wave 2）起已落**——`req.workspace_id` 非空时 `candidates = vec![req.workspace_id.clone()]`（仅该 workspace），空时 probe 全 workspace（aggregate-all）。**无新代码**——交付一个 verify-only 守护测试如实记录已存在的隔离（workspace_id 非空仅返该 workspace chunk / 跨 workspace chunk_id not_found / 空走 aggregate）。这正是 ADR-013 的价值：grounding 校正掉 survey 高估，据实记 already-present 而非伪造为新工作。

本 ADR 把上述「dim-negotiation seam / config-file env-bridge / get_source_chunk 隔离 already-present 校正」收敛为一个精简补全 + 诚实化 Phase 的处理策略。改动**全为 code-local 🟢 可单测**（Rust 纯函数协商 seam + Go TOML round-trip / setVectorEnv 单测）；dim-declaring feature backend(qdrant/lancedb/sqlite-vec) 的 live dim-enforce 须 feature build → 🟡 honest-defer，不伪造（ADR-013）。全部改动遵守 ADR-004 默认行为 / proto / 既有契约不变 + 0 网络 + ADR-008 0 新依赖（Rust core 不引 toml）+ ADR-013 受阻 / 非问题项诚实分级不伪造。

## Decision

向量配置补全采用 **「factory dim 协商 seam + Go→env config-file 桥接 + get_source_chunk 隔离据实校正 + 默认零依赖 / dim-agnostic 守线」** 策略，分 4 个决策点：

### D1 — vector-dim-auto-negotiation（factory negotiate + `expected_dim`；default BruteForce no-op honest-caveat；feature-enforce honest-defer）（task-34.1）🟢

为 `select_vector_backend` 补 dim 协商 seam（镜像 embedding 侧 `negotiate_dim`）：

- 在 `VectorBackend` trait（`core/src/retriever/vector/traits.rs`）加 `expected_dim(&self) -> Option<usize>`，**DEFAULT impl 返回 `None`**（dim-agnostic）；`BruteForceVectorBackend` 沿用默认 `None`（不约束 dim）。该 default impl 使新方法 add-only——既有 backend impl 零改动（ADR-014 D5）。
- 加纯函数 `negotiate_vector_dim(requested: usize, declared: Option<usize>) -> Result<(), VectorError>`：`requested == 0`（用默认）**或** `declared == None`（dim-agnostic）→ `Ok`；非零 `requested != Some(declared)` → `VectorError::DimMismatch{expected: requested, got: declared}`（既存 variant，types.rs:83）。
- `factory.rs:39` 的 `let _ = dim;` 替换为 `negotiate_vector_dim(dim, backend.expected_dim())?` 调用。

**0 新依赖、0 schema migration、0 proto 改动**。交付物 = `CONTEXTFORGE_VECTOR_DIM` 不再被静默丢弃 + 一个纯函数协商 seam 单测。Tests：TEST-34.1.1（`negotiate_vector_dim` 纯函数：0 → Ok / None-declared → Ok / matching → Ok / mismatch → `DimMismatch`）+ TEST-34.1.2（BruteForce default path：任意 dim 被接受、byte-equivalent）。

**理由**：embedding factory 协商 dim 而 vector factory 丢弃 dim（`let _ = dim;`）是真实对称缺口，且 `VectorError::DimMismatch` 已存在却无人产出。镜像 embedding 侧 `negotiate_dim`（纯函数 seam + trait 声明 dim）最 surgical：`expected_dim` 用 default-impl-None 使其 add-only（既有 backend 零改）、纯函数可单测、复用既存 error variant、0 新 dep。**HONEST CAVEAT（ADR-013）**：默认 BruteForce 是 dim-agnostic（`expected_dim()=None`），故 DEFAULT build 下协商接受**任意** dim（无强制、default 行为 byte-equivalent，ADR-004）——真正的 dim 强制只对 dim-declaring feature backend(qdrant/lancedb/sqlite-vec) 生效，其 live 行使须 feature build → honest-defer `[SPEC-DEFER:phase-future.vector-dim-feature-enforce]`，不在默认 build 伪造强制证据。备选「在 BruteForce 上强制 dim」否决：BruteForce 设计上 dim-agnostic，强制 dim 会破其语义 + 默认行为（见 A2）。

### D2 — vector-backend-config-file（Go `[vector]`→env 桥接；env-wins；no-section = byte-equiv；Rust 0-dep 保持）（task-34.2）🟢

为 vector backend 加 config-file 路径，经 Go→env 跨进程桥接（**不在 Rust 端引 toml**）：

- Go `config.Config`（`internal/config/config.go`）加 `[vector]` section：`Backend string`（toml `backend` tag）+ `Dim int`（toml `dim` tag），镜像既有 `[embedding]`。
- 加 `setVectorEnv` helper（镜像 `setDataDirEnv`，`cmd/contextforge/main.go:255`）：当 `[vector]` present **且**对应 env **未**已设置时，导出 `CONTEXTFORGE_VECTOR_BACKEND` / `CONTEXTFORGE_VECTOR_DIM`，使 spawned core daemon 经既有 `resolve_vector_backend` env 路径拾取（daemon.go:202 继承 env）。
- **ENV WINS**：显式设的 env var 覆盖 config file（back-compat）。无 `[vector]` section → 不导出 → unset → BruteForce byte-equivalent（ADR-004 默认不变）。

这是与 `CONTEXTFORGE_DATA_DIR` 同形的**已验证跨进程 env-bridge**（**非** deferred 的 `daemon.Options.DataDir` field 重构——后者保持 `[SPEC-DEFER:phase-future.daemon-options-datadir]`，承 ADR-038 D4）。**0 新依赖**。Tests：TEST-34.2.1（Go config `[vector]` TOML round-trip：有 / 无 section、既有 `[collections]` / `[remote]` / `[embedding]` 不受影响）+ TEST-34.2.2（`setVectorEnv`：section present → env 导出 / env 已设 → 不覆盖 env-wins / 无 section → 不导出）。

**理由**：vector backend 当前仅 env、无 config-file 路径是真实缺口；Rust core 无 toml dep，在 Rust 端写 config reader 会破 0-dep（ADR-008）。Go 已解析 config.toml + spawned daemon 继承 env + `CONTEXTFORGE_DATA_DIR` 已证此桥接可行 → Go 端 `[vector]`→env 桥接是最 surgical 且保 Rust 0-dep 的路径。**env-wins** 保 back-compat（显式 env 覆盖文件）；**no-section = byte-equiv** 保默认 BruteForce 行为不变（ADR-004）。复用 `setDataDirEnv` 同形的跨进程 bridge，**不**触 deferred 的 `daemon.Options.DataDir` field 重构（须接 child `cmd.Env` + 改 spawn 契约，承 ADR-038 D4 honest-defer）。

### D3 — get_source_chunk workspace 隔离 = HONEST 非问题（already-present，grounding 校正 / verify-only）+ honest-defer 边界（task-34.3）🟢 / non-issue

`get_source_chunk` 的 workspace 隔离经 grounding 复核**自 task-12.2（ADR-016/017 D1 Wave 2）起已落**——`search.rs:421-423`：`req.workspace_id` 非空 → `candidates = vec![req.workspace_id.clone()]`（仅该 workspace），空 → probe 全 workspace（aggregate-all probe，chunk_id 全局唯一）。survey 把它高估为 gap → **grounding 校正**：据实记为 already-present，**无新代码**。交付物 = 一个 **verify-only 守护测试**（如实记录已存在的隔离对称：workspace_id set → 仅该 workspace chunk / 跨 workspace 的 chunk_id → not_found / empty → aggregate），使隔离行为被显式守护、未来回归会令其失败。

honest-defer 边界（据 ADR-013 如实记录，不伪造、不夸大）：
- **vector-dim feature-enforce**：D1 的 dim 强制对 dim-declaring feature backend(qdrant/lancedb/sqlite-vec) 的 live 行使须 feature build → honest-defer `[SPEC-DEFER:phase-future.vector-dim-feature-enforce]`（默认 BruteForce dim-agnostic no-op 据实标注，纯函数 seam 🟢 已达 / feature live 未跑、不预填）。
- **daemon.Options.DataDir field 重构**：D2 用 `setVectorEnv` 同 `setDataDirEnv` 的跨进程 env-bridge，**不**触 `daemon.Options.DataDir` field 重构（须接 child `cmd.Env` + 改 spawn 契约）→ 承 ADR-038 D4 honest-defer `[SPEC-DEFER:phase-future.daemon-options-datadir]`。
- **get_source_chunk 隔离 = already-present**（非 gap）→ verify-only 守护测试，不重实现（grounding 校正）。

**理由**：据 ADR-013，对非问题 / 受阻项诚实记录边界、不伪造完成、不夸大缺口。get_source_chunk 隔离经 grounding 校正确认 task-12.2 已落（search.rs:421-423）——写「新隔离代码」是把 already-present 伪造为新工作（违 Simplicity-First + ADR-013）；据实交付 verify-only 守护测试守护已存在的对称行为。D1 的 feature-enforce / D2 的 daemon.Options.DataDir 据实 defer 并打 SPEC-DEFER tag，不在默认 build / 当前桥接伪造其证据。

### D4 — 默认行为 + 0-dep + 0-network + 既有契约不变（all tasks）🟢

所有改动保持默认行为 / proto / 既有契约不变 + 0 网络（ADR-004）+ 0 新依赖（ADR-008，Phase 34 = 0 dep，Rust core 不引 toml）：

- D1 `expected_dim` 为 default-impl-None add-only trait 方法（既有 backend 零改）+ `negotiate_vector_dim` 纯函数（默认 BruteForce dim-agnostic → 任意 dim 接受、byte-equivalent）；0 proto / 0 migration。
- D2 `[vector]` 为 add-only Go config section（无 section → 零值 → 不导出 → unset → BruteForce byte-equiv）+ `setVectorEnv` env-wins（显式 env 覆盖文件，back-compat）；Rust core 0-dep（config-file 经 Go→env 桥接，非 Rust toml reader）。
- D3 get_source_chunk 隔离为 already-present verify-only 测试（无行为改）。
- 既有 `cargo-test` / `go-test` / `lint` / `spec-lint` 四门不退化。

**理由**：ADR-004 local-first + ADR-008 dep add-only——默认行为 / proto / 既有契约不变 + 0 网络 + 0 新依赖（含 Rust core 不引 toml）是不可让渡 baseline。本 phase 为配置补全 + 诚实化——非默认行为演进。default-impl-None trait 方法（既有 backend 零改）/ dim-agnostic BruteForce（任意 dim byte-equiv）/ add-only Go `[vector]`（无 section byte-equiv）/ env-wins（显式 env 覆盖文件）/ Go→env 桥接（Rust 0-dep）使既有用户与既有契约零感知。

## Consequences

- **Positive**: `CONTEXTFORGE_VECTOR_DIM` 不再被工厂静默丢弃（D1 `negotiate_vector_dim` 纯函数 seam + `expected_dim` default-impl-None trait 方法，镜像 embedding 侧 `negotiate_dim`，复用既存 `VectorError::DimMismatch`，0 新 dep / 0 proto / 0 migration，embedding/vector 两 factory 协商对称）；vector backend 获 config-file 路径（D2 Go `[vector]`→env 桥接，env-wins back-compat，无 section byte-equiv，复用 `setDataDirEnv` 同形跨进程 bridge，Rust core 保持 0-dep 不引 toml）；get_source_chunk workspace 隔离经 grounding 校正据实记为 already-present（task-12.2 起 search.rs:421-423 已落）+ verify-only 守护测试显式守护对称行为（无新代码）；全部 0-dep / 0-network / default 保形（BruteForce dim-agnostic 任意 dim byte-equiv / 无 `[vector]` section byte-equiv），默认行为 / proto / 既有契约不变（ADR-004 / ADR-008），既有四门不退化。
- **Negative / open**（受阻 / 非问题项如实，不伪造、不夸大）：vector-dim feature-enforce（D1 dim 强制对 dim-declaring feature backend(qdrant/lancedb/sqlite-vec) 的 live 行使须 feature build）→ 🟡 honest-defer `[SPEC-DEFER:phase-future.vector-dim-feature-enforce]`（默认 BruteForce dim-agnostic no-op 据实标注、纯函数 seam 🟢 已达 / feature live 未跑、不预填）；daemon.Options.DataDir field 重构（D2 用 `setVectorEnv` 跨进程 env-bridge，不触 field 重构，须接 child `cmd.Env` + 改 spawn 契约）→ 承 ADR-038 D4 honest-defer `[SPEC-DEFER:phase-future.daemon-options-datadir]`；get_source_chunk workspace 隔离 = HONEST 非问题（already-present，grounding 校正）→ verify-only 守护测试，不重实现——以上据 ADR-013 如实分级、不伪造完成、不夸大缺口。
- **Ratification**: 本 ADR **Proposed**。task-34.1 / 34.2 通过后于 v0.27.0 closeout（task-34.3）据真实 CI / 实测产物逐 D ratify Proposed→Accepted（ADR-013：禁据合成 / 伪造 ratify）；vector-dim feature-enforce live（须 feature build）等 🟡 受阻维度据已达维度（纯函数 seam / BruteForce default path 🟢）ratify + 如实记录受阻，不强 ratify。
- **Follow-ups**: vector-dim feature-enforce（dim-declaring feature backend(qdrant/lancedb/sqlite-vec) feature build + live dim mismatch 真实行使后）`[SPEC-DEFER:phase-future.vector-dim-feature-enforce]`；datadir / vector env→`daemon.Options` field（接 child `cmd.Env` + 改 spawn 契约后）`[SPEC-DEFER:phase-future.daemon-options-datadir]`。ADR-037（dim-negotiation + config-file 补全 env-plumbing）以 add-only Phase 34 Amendment 于 task-34.3 记录（不溯改正文，ADR-014 D5）；ADR-034 / ADR-023 / ADR-027 / ADR-016 / ADR-004 / ADR-008 / ADR-013 引用均不溯改其正文。

## Ratification（v0.27.0 / task-34.3）

本 ADR 于 v0.27.0 closeout（task-34.3）据 task-34.1 / 34.2 / 34.3 真实 CI（四门绿：cargo-test / go-test / lint / spec-lint）逐 D ratify Proposed→Accepted。各 D 真实依据于 closeout 据真实产物回填（ADR-013 不预填、不据合成 ratify）：

- **D1（vector-dim-auto-negotiation）→ Accepted 🟢（待 closeout 回填真实 PR/SHA）**：task-34.1 落 `VectorBackend::expected_dim` default-impl-None + `negotiate_vector_dim` 纯函数 + `factory.rs` 以 `negotiate_vector_dim(dim, backend.expected_dim())?` 替换 `let _ = dim;`，0 新 dep。TEST-34.1.1（纯函数 4 路径）+ TEST-34.1.2（BruteForce default path 任意 dim byte-equiv）绿。**honest-caveat 据实**：默认 BruteForce dim-agnostic（`expected_dim()=None`）→ 默认 build 无 dim 强制；feature-enforce live honest-defer `[SPEC-DEFER:phase-future.vector-dim-feature-enforce]`。
- **D2（vector-backend-config-file）→ Accepted 🟢（待 closeout 回填真实 PR/SHA）**：task-34.2 Go `config.Config` 加 `[vector]`（Backend / Dim）+ `setVectorEnv` 镜像 `setDataDirEnv`（present 且 env 未设 → 导出 / env 已设 → 不覆盖 env-wins / 无 section → 不导出），Rust core 0-dep 保持。TEST-34.2.1（TOML round-trip，既有 sections 不受影响）+ TEST-34.2.2（setVectorEnv 三路径）绿。daemon.Options.DataDir 重构 honest-defer `[SPEC-DEFER:phase-future.daemon-options-datadir]`。
- **D3（get_source_chunk workspace 隔离 already-present verify-only）→ Accepted 🟢（grounding 校正，非实现）**：task-34.3 据 grounding 复核确认 search.rs:421-423 自 task-12.2 已落隔离，交付 verify-only 守护测试（workspace_id set → 仅该 workspace / 跨 workspace chunk_id → not_found / empty → aggregate）绿，无新代码。survey 高估据实校正为 already-present。
- **D4（默认行为 + 0-dep + 0-network + 既有契约不变）→ Accepted 🟢（待 closeout 回填）**：全 phase 0 新 dep（Rust core 不引 toml）；0 proto / 0 migration；`expected_dim` default-impl-None add-only / `[vector]` add-only section 无 section byte-equiv / BruteForce dim-agnostic 任意 dim byte-equiv；既有 `cargo test --workspace` + `go test ./...` + lint + spec-lint 四门不退化。

真实 v0.27.0 tag/run/digest 经用户授权后由 post-tag-push backfill 填实（release docs `<backfill>`，ADR-013 不预填）。

## Alternatives

- **A1（Rust 端 toml config-file reader）**：在 Rust core 直接读 config.toml 的 `[vector]` section。否决：Rust core 无 toml dep，引 toml 会破 0-dep（ADR-008）；据 D2 复用 Go 已有 toml 解析 + spawned daemon 继承 env 的 `CONTEXTFORGE_DATA_DIR` 同形跨进程 env-bridge（Go `[vector]`→env），Rust core 保持 0-dep，最 surgical。
- **A2（在 BruteForce 上强制 dim）**：给 `BruteForceVectorBackend` 声明固定 `expected_dim` 并强制 dim 匹配。否决：BruteForce 设计上 dim-agnostic（任意 dim 工作，ADR-034 默认 byte-equiv），强制 dim 会破其语义 + 改默认行为（违 ADR-004）；据 D1 `expected_dim` 默认 `None`（dim-agnostic），真正 dim 强制只对 dim-declaring feature backend(qdrant/lancedb/sqlite-vec) 生效，其 live 行使 honest-defer `[SPEC-DEFER:phase-future.vector-dim-feature-enforce]`。
- **A3（把 get_source_chunk 隔离当 gap 实现）**：按 survey 写「新 workspace 隔离代码」。否决：grounding 复核确认 search.rs:421-423 自 task-12.2（ADR-016/017 D1 Wave 2）已落隔离——写新代码是把 already-present 伪造为新工作（违 Simplicity-First + ADR-013）；据 D3 据实校正为 already-present，交付 verify-only 守护测试守护已存在的对称行为，不重实现。

## 触及 ADR 关系

- **ADR-037（vector-backend-config-plumbing-and-completeness）→ add-only Phase 34 Amendment @ task-34.3**：Phase 32 起的 env-plumbing 在 dim 协商（D1）+ config-file（D2）两点上仍未闭合，本 phase 据实补齐。以 `## Amendment (Phase 34 / v0.27.0)` add-only 记 dim-negotiation + config-file 补全，**不溯改 ADR-037 正文**（ADR-014 D5）。
- **ADR-034（vector-store-composition-and-honest-defer）→ 引用（不溯改）**：`select_vector_backend` 工厂 + BruteForce 默认 byte-equiv 是 ADR-034 D1，本 phase 在工厂处补 `negotiate_vector_dim` seam（D1）承其方向，默认 BruteForce dim-agnostic byte-equiv 不变，不溯改其正文。
- **ADR-023（vector-persistence）→ 引用（不溯改）**：dim-declaring feature backend(qdrant/lancedb/sqlite-vec) 才真正约束 dim，默认 BruteForce dim-agnostic（D1 honest-caveat）承其 feature-gated 路径，不溯改其正文。
- **ADR-027（embedding-provider-selection）→ 镜像引用（不溯改）**：本 phase `negotiate_vector_dim`（D1）镜像其 `negotiate_dim`（embedding/factory.rs:88-96）的纯函数协商语义（requested==0 → Ok / mismatch → DimMismatch），把 embedding 侧已有的协商对称补到 vector 侧，不溯改其正文。
- **ADR-016（workspace-isolation）→ 据实校正引用（不溯改）**：get_source_chunk 的 workspace 隔离（D3）经 grounding 复核确认 task-12.2（ADR-016/017 D1 Wave 2）已落（search.rs:421-423），本 phase 据实记 already-present + verify-only 守护，承其隔离方向，不溯改其正文。
- **ADR-038（governance-debt-cleanup-2）→ 承接 honest-defer 引用（不溯改）**：D2 不触 `daemon.Options.DataDir` field 重构，承 ADR-038 D4 的 `[SPEC-DEFER:phase-future.daemon-options-datadir]` honest-defer，不溯改其正文。
- **ADR-004（local-first-privacy-baseline）→ 守线**：默认行为 / proto / 既有契约不变 + 0 网络 + BruteForce 默认 dim-agnostic byte-equiv + 无 `[vector]` section byte-equiv（D4）守 ADR-004 baseline。
- **ADR-008（dep add-only）→ 守线**：本 phase 加 **0 新依赖**——Rust core 不引 toml（config-file 经 Go→env 桥接），`expected_dim` / `negotiate_vector_dim` / Go `[vector]` section / `setVectorEnv` 均 0-dep。
- **ADR-013（禁伪造红线）→ 守线**：vector-dim feature-enforce live（须 feature build）真实跑出后回填、不预填；get_source_chunk 隔离据实校正为 already-present、不重实现（grounding-correction）；BruteForce dim-agnostic no-op 据实标注不夸大为强制（D1 / D3）。
- **ADR-014（cross-phase-exit-criteria-validation）→ 第二十五次激活**：D1-D4 mapping + 各 task LAST D2 lint（touched 行 0 未标注命中）+ D3 verified-by + D4 自治 + D5 历史 Phase 1-33 不溯改（ADR 改动 add-only Phase 34 Amendment）；本 ADR ratify 在 task-34.3 closeout，Proposed 阶段不 ratify。
