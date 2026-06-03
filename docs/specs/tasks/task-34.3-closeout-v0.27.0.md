# Task `34.3`: `closeout-v0.27.0 — get_source_chunk workspace-isolation VERIFY-ONLY guard test（已 task-12.2 交付 search.rs:421-423；grounding 校正 survey overstatement，交付守护测试不写新代码）+ v0.27.0 closeout（smoke v24 step [43/43] + TestTask343 + release docs + ADR-039 据 D1-D5 ratify + ADR-037 add-only Phase 34 Amendment + roadmap §3.16+§4 + adapter）`

**Status**: Draft

**Priority**: P2
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 34 (vector-config-completeness)
**Dependencies**: task-34.1（vector-dim-auto-negotiation — `factory.rs` `select_vector_backend` 不再静默丢弃配置 dim：加 `expected_dim(self) -> Option<usize>` 默认 trait 方法 + pure-fn `negotiate_vector_dim` 协商 seam）/ task-34.2（vector-backend-config-file — Go `[vector]` 段 → `CONTEXTFORGE_VECTOR_BACKEND`/`CONTEXTFORGE_VECTOR_DIM` env 跨进程桥，env-wins，无 `[vector]` 段 → 不导出 → BruteForce byte-equiv）全 Done / ADR-039（vector-config-completeness，本 task ratify）/ ADR-037（vector-backend-config-plumbing-and-completeness，本 task add-only Phase 34 Amendment：dim-negotiation + config-file 补全 Phase 32 起的 env-plumbing）/ ADR-004（默认行为 / proto 既有字段 / 既有契约不变——默认 BruteForce dim-agnostic + 无 `[vector]` 段 byte-equivalent）/ ADR-008（dep add-only，Phase 34 = 0 new dep）/ ADR-012（tag/release outward-facing 须用户显式授权；本轮已授权 v0.27.0）/ ADR-013（禁伪造红线——真实 tag/run/digest 不预填；survey overstatement grounding 校正如实记录）/ ADR-014 D1-D5（第二十五次激活）

## 1. Background

Phase 34（vector-config-completeness）是一个刻意**小**的版本——Phase 31/33 治理债清理后绿色 backlog 偏薄，故据 ADR-013「诚实优于充数」只收两处真实补全（task-34.1 / task-34.2）+ 本 closeout（task-34.3）；它把 Phase 32（vector-backend-config-plumbing-and-completeness，ADR-037）起的 vector backend 配置故事**收口**：env→server.rs 两热路径接线（Phase 32 D1）+ sqlite-vec 工厂臂（Phase 32 D2）已就位，Phase 34 补两块剩余拼图——配置 dim 不再被工厂静默丢弃 + backend 可经 config 文件（非仅 env）选择。

两个实现 task 全 Draft（实施授权另行）：34.1（vector-dim-auto-negotiation——`core/src/retriever/vector/factory.rs:33-39` `select_vector_backend(name, dim)` 当前 `let _ = dim;` **静默丢弃** `server.rs` `resolve_vector_backend`（`server.rs:540`）解析后传入的 `CONTEXTFORGE_VECTOR_DIM`；本 task 仿 `core/src/embedding/factory.rs:81-96` `negotiate_dim(provider_dim, requested)`——加 `expected_dim(self) -> Option<usize>` 到 `VectorBackend` trait（`core/src/retriever/vector/traits.rs:11-16`，**DEFAULT impl 返回 `None`** 表示 dim-agnostic，`BruteForceVectorBackend` 保留 `None`），`factory.rs` 用 pure-fn `negotiate_vector_dim(dim, backend.expected_dim())` 代替 `let _ = dim`（requested==0 OR declared==None → `Ok`；非零 requested != `Some(declared)` → `VectorError::DimMismatch{expected,got}`，该 variant 已存在 `core/src/retriever/vector/types.rs:83`）。HONEST CAVEAT：默认 BruteForce dim-agnostic（`expected_dim()=None`）→ 默认构建协商**接受任何 dim**（无强制、与改前 byte-equivalent、ADR-004）；真正强制只对声明 dim 的 feature backend（qdrant/lancedb/sqlite-vec）生效，其 live 行使 `[SPEC-DEFER:phase-future.vector-dim-feature-enforce]`（须 feature build））/ 34.2（vector-backend-config-file——vector backend 当前 env-only（`CONTEXTFORGE_VECTOR_BACKEND`/`CONTEXTFORGE_VECTOR_DIM` 由 Rust `server.rs` 读）；Rust core 有 serde/serde_json 但**无 toml dep**（Rust 侧加 toml reader 会破 0-dep，ADR-008），而 Go `internal/config/config.go` 已解析 `config.toml` 的 `[collections]`/`[remote]`/`[embedding]` 段，spawn 的 Rust daemon **继承 Go 进程 env**（`internal/daemon/daemon.go:201-208` `exec.Command` 继承 env；`cmd/contextforge/main.go:254-268` `setDataDirEnv` 同法导出 `CONTEXTFORGE_DATA_DIR`）；本 task 加 `[vector]` 段到 Go `config.Config`（`Backend string` toml `backend` tag + `Dim int` toml `dim` tag）+ `setVectorEnv` helper（仿 `setDataDirEnv`）：当 `[vector]` 存在**且**对应 env 未被显式设置时导出 `CONTEXTFORGE_VECTOR_BACKEND`/`CONTEXTFORGE_VECTOR_DIM`，让 spawn 的 core daemon 经既有 `resolve_vector_backend` env 路径拾取。ENV WINS：显式设置的 env 覆盖 config 文件（向后兼容）；无 `[vector]` 段 → 不导出 → unset → BruteForce byte-equivalent（ADR-004 默认不变）。复用与 `CONTEXTFORGE_DATA_DIR` 同一已验证跨进程 env 桥，**非** deferred `daemon.Options.DataDir` 字段重构（其仍 `[SPEC-DEFER:phase-future.daemon-options-datadir]`））。

本 task 兼两职：(A) 一处经 grounding 校正为 **HONEST NON-ISSUE / verify-only** 的项——`get_source_chunk` workspace 隔离；(B) 收口 v0.27.0：smoke v24 + release docs + ADR-039 据真实结果 ratify + ADR-037 add-only Phase 34 Amendment + roadmap §3.16 推进记录 + §4 add-only backlog + phase §6 闭合 + adapter + feature。

**(A) `get_source_chunk` workspace-isolation = HONEST NON-ISSUE / verify-only（grounding 校正）**：survey 把它表述为 gap，**经核 `core/src/data_plane/search.rs:421-423` 自 task-12.2（ADR-017 D1 Wave 2）起即已把候选 scope 到 `req.workspace_id`**——非空 `workspace_id` → `vec![req.workspace_id.clone()]`（仅该 workspace），空 → `workspace_store.list()` 聚合全 workspace 探测（`:423-431`），命中即返回（`:443-458`）。隔离已在位、无新代码。本 task 据 ADR-013 交付一个 **verify-only 守护测试** 对称记录已在位的隔离（`workspace_id` 设 → 仅该 workspace 的 chunk；跨 workspace `chunk_id` → not_found；空 → 聚合），并把 survey 的 overstatement 作为 grounding 校正记于 ADR-039 D3——不写新代码（symmetry honestly）。

**(B) v0.27.0 closeout**：smoke v23 step `[42/42]`（Phase 33 live）顺接 v24 step `[43/43]`（banner v23→v24，staging `cf-v26-cfg`，offset +2）+ `TestTask343`（mirror `TestTask334`，无回归 `[37/37]`..`[42/42]`）+ `docs/releases/v0.27.0-{evidence,artifacts}.md`（`<backfill>` 待回填）+ README v0.27 段 + RELEASE_NOTES v0.27.0 段 + ADR-039 Proposed→Accepted（per-D ratify）+ ADR-037 add-only Phase 34 Amendment + roadmap §3.16 + §4 + phase-34 §6 闭合 + adapter + feature。

## 2. Goal

(A) 交付 `get_source_chunk` workspace-isolation **verify-only 守护测试**（`core/src/data_plane/search.rs:421-423` 已 task-12.2 在位的隔离的对称记录）：`workspace_id` 设非空 → 仅返回该 workspace 的 chunk；跨 workspace 的 `chunk_id`（属另一 workspace）→ `Status::not_found`；`workspace_id` 空 → 聚合全 workspace 探测（既有 `:423-431` 行为）。**不写新代码**——隔离已自 task-12.2 在位（survey overstatement 校正即本 task 的 ADR-013 价值，须在 spec 与 ADR-039 D3 如实记录）。0 新 dep。

(B) 据 34.1/34.2 **真实 CI / 实测产物**收口 v0.27.0：ADR-039 `Proposed → Accepted`（逐 D 如实——D1 vector-dim-auto-negotiation（factory negotiate + `expected_dim`）达成 + 默认 BruteForce no-op honest-caveat + feature-enforce `[SPEC-DEFER:phase-future.vector-dim-feature-enforce]`、D2 vector-backend-config-file（Go `[vector]`→env 桥，env-wins，无 `[vector]` 段=byte-equiv，Rust 0-dep 保持）达成、D3 `get_source_chunk` isolation already-present verify-only（grounding 校正）+ dropped/honest-defer 边界、D4 默认行为 + 0-dep + 0-network + 既有契约不变（ADR-004/008））；ADR-037 add-only Phase 34 Amendment（dim-negotiation + config-file 补全 Phase 32 起的 env-plumbing，不溯改 D1-D5 正文 ADR-014 D5）；roadmap §3.16（Phase 34 推进记录）+ §4 add-only（新 backlog）；phase-34 §6 AC 置 `[x]` + Status Done；smoke v24 step `[43/43]`；release docs（tag/run/digest 用 `<backfill>`）；adapter（Phase 34 Done + Tasks 3 + ADR-039 Accepted + feature 行）。**真实 v0.27.0 tag/release 须用户显式授权**（本轮用户已授权 v0.27.0；不自行越界 tag，ADR-012）。

pass bar：(A) `get_source_chunk` workspace-isolation verify-only 守护测试 🟢（`workspace_id` 设 → 仅该 workspace / 跨 workspace `chunk_id` → not_found / 空 → 聚合）；(B) smoke `bash -n` 过 + `go test -run TestTask343` 过 + 文档闭合人工核 + ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- `core/src/data_plane/search.rs`（或同源 test 模块）——`get_source_chunk` workspace-isolation **verify-only 守护测试**：(a) `workspace_id` 设非空、目标 chunk 属该 workspace → 返回该 chunk（`PbSourceChunk.workspace_id == req.workspace_id`）；(b) `workspace_id` 设非空但 `chunk_id` 属**另一** workspace → `Status::not_found`（候选 scope 到 `vec![req.workspace_id]`，`:421-422`，不跨 workspace 命中）；(c) `workspace_id` 空 → 聚合全 workspace 探测（`workspace_store.list()`，`:423-431`），任一 workspace 命中即返回。**断言已在位隔离行为，不改 `search.rs` 生产代码**。
- `scripts/console_smoke.sh`——banner v23→v24 + v24 changelog 块 + step `[43/43]`（doc/status 断言 vector-config-completeness baseline：vector-dim-auto-negotiation + vector-backend-config-file + get_source_chunk isolation verify-only；default build init baseline 不变 + denominator 不溯改 ADR-014 D5），staging `cf-v26-cfg`（offset +2）。当前 live 脚本 v23 `[42/42]`（Phase 33）；故 Phase 34 顺接 `[43/43]`。
- `internal/cli/smoke_syntax_test.go`——新增 `TestTask343_SmokeV24VectorConfigCompletenessStep`（mirror `TestTask334`，断言 `v24 (task-34.3)` header + `[43/43]` + 标记（`vector-config-completeness` / `TEST-34.1.` / `TEST-34.2.` / `TEST-34.3.` / `expected_dim` / `[vector]` / `workspace_id`）+ 无回归既有 `[37/37]`..`[42/42]`，denominator 不溯改 ADR-014 D5 + `bash -n` 语法）。
- 新增 `docs/releases/v0.27.0-{evidence,artifacts}.md`（tag SHA / run id / digest 用 `<backfill>` 待回填）+ `README.md` v0.27 段 + `RELEASE_NOTES.md` v0.27.0 段。
- `docs/decisions/adr-039-vector-config-completeness.md`——Status Proposed→Accepted（per-D 限定）+ `## Ratification（v0.27.0 / task-34.3）` 节（逐 D 真实依据；D1 默认 BruteForce no-op honest-caveat / feature-enforce honest-defer、D2 env-wins + 无段 byte-equiv + Rust 0-dep 保持、D3 isolation already-present verify-only grounding 校正、D4 默认行为 + 0-dep + 0-network 不变）。
- add-only Amendment（不溯改正文，ADR-014 D5）：`docs/decisions/adr-037-vector-backend-config-plumbing-and-completeness.md`——`## Amendment (Phase 34 / v0.27.0)`（vector-dim-auto-negotiation——factory negotiate seam + `expected_dim` 默认 trait 方法，承 Phase 32 D1 env→config 接线；vector-backend-config-file——Go `[vector]`→env 跨进程桥，env-wins，承 ADR-037 Follow-ups `[SPEC-DEFER:phase-future.vector-backend-config-file]`；不溯改 D1-D5 正文 + 既有 Amendment 正文）。
- `docs/roadmap.md`——§3 新增 §3.16 Phase 34 推进记录 + §4 add-only（新 backlog 条目：vector-dim-feature-enforce（feature build）/ daemon-options-datadir（承既有 defer）/ vector-config-file-rust-native（超 Go→env 桥的 Rust 原生结构化读，须 toml dep）；add-only 不删旧条目正文）。
- `docs/specs/phases/phase-34-vector-config-completeness.md`——Status Draft→Done + §6 AC `[x]`（honest per-dim：feature-enforce live / config-file-rust-native 🟡 如实标注）。
- `docs/s2v-adapter.md`——§Phase 34 In Progress→Done + Tasks 2→3；§Task +34.3；§ADR 039 Proposed→Accepted；§BDD +phase-34 行。
- `test/features/phase-34-vector-config-completeness.feature`（已创建）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER] / DROPPED honest record）

以下经 grounding 校正为 **DROPPED / verify-only / honest-defer，不实现新代码**（survey overstatement 校正即本 task 的 ADR-013 价值，须在 spec 与 ADR-039 D3 如实记录）：

- **`get_source_chunk` workspace 隔离 = ALREADY-PRESENT verify-only（不写新代码）**：survey 把它表述为 gap——经核 `core/src/data_plane/search.rs:421-423` 自 task-12.2（ADR-017 D1 Wave 2）即已把候选 scope 到 `req.workspace_id`（非空 → 仅该 workspace；空 → 聚合全 workspace 探测），隔离已在位。本 task 据 ADR-013 交付 verify-only 守护测试对称记录已在位的隔离（`workspace_id` 设 → 仅该 workspace / 跨 workspace `chunk_id` → not_found / 空 → 聚合），grounding 校正记于 ADR-039 D3——不写新生产代码。

其余范围外：
- 真实 v0.27.0 tag push + release run（cosign 真签 + GHCR 推送）[SPEC-OWNER:user-authorized-release]——outward-facing 不可逆已获本轮用户授权（ADR-012）；post-tag-push backfill 填实 tag SHA / run id / digest，本 task body 不预填真实凭据。
- vector dim feature-backend live enforcement（qdrant/lancedb/sqlite-vec 声明 dim 与 requested 真实协商 → live DimMismatch）[SPEC-DEFER:phase-future.vector-dim-feature-enforce]——task-34.1 默认 BruteForce dim-agnostic（`expected_dim()=None`）→ 默认协商接受任何 dim（byte-equiv，无强制）；真正强制只对声明 dim 的 feature backend 生效，须 feature build 行使，honest-caveat 不伪造已验。
- `daemon.Options.DataDir` 字段重构（替代 `CONTEXTFORGE_DATA_DIR` 跨进程 env 桥）[SPEC-DEFER:phase-future.daemon-options-datadir]——task-34.2 复用与 `CONTEXTFORGE_DATA_DIR` 同一已验证 env 桥（`setVectorEnv` 仿 `setDataDirEnv`），**非** Options 字段重构（承 task-33.4 既有 defer，本 task 不实现）。
- Rust 原生结构化 vector config 读（超 Go→env 桥的 Rust core 直读 config 文件）[SPEC-DEFER:phase-future.vector-config-file-rust-native]——Rust core 无 toml dep（加之破 0-dep，ADR-008），task-34.2 走 Go `[vector]`→env 跨进程桥据实记录；Rust 原生读须新 dep，honest-defer 新 backlog。

## 4. Actors

- 主 agent（ADR-012 自治；真实 release 本轮已获用户授权）
- `get_source_chunk`（`core/src/data_plane/search.rs:403-458`，本 task verify-only 守护测试目标——`:421-423` task-12.2 已在位的 `workspace_id` scope 隔离）
- `select_vector_backend` / `negotiate_vector_dim` / `expected_dim`（task-34.1 落地，`core/src/retriever/vector/factory.rs:33-39` + `traits.rs:11-16`，本 closeout 经 ADR-039 D1 ratify）
- Go `[vector]` config 段 + `setVectorEnv`（task-34.2 落地，`internal/config/config.go` + `cmd/contextforge/main.go`，本 closeout 经 ADR-039 D2 ratify）
- closeout 文档集（smoke / release docs / ADR-039 ratify / ADR-037 add-only Phase 34 Amendment / roadmap §3.16+§4 / phase spec / adapter / feature）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/data_plane/search.rs:403-458`（`get_source_chunk`——`:408-410` chunk_id 空校验 + `:411-416` 空 data_dir → not_found + **`:421-423` task-12.2 `workspace_id` scope 隔离锚点**（非空 → `vec![req.workspace_id.clone()]` 仅该 workspace；空 → `:423-431` `workspace_store.list()` 聚合）+ `:443-458` 命中返回——本 task verify-only 守护测试锚点，断言已在位隔离不写新代码）
- `core/src/retriever/vector/factory.rs:33-39`（`select_vector_backend(name, dim)` `let _ = dim;` 静默丢弃锚点——task-34.1 替换为 `negotiate_vector_dim` 协商 seam，本 closeout ratify ADR-039 D1）+ `core/src/embedding/factory.rs:81-96`（`negotiate_dim(provider_dim, requested)` mirror 源——requested==0 never mismatch / 非零 differ → `DimMismatch`）+ `core/src/retriever/vector/traits.rs:11-16`（`VectorBackend` trait——task-34.1 add `expected_dim(self) -> Option<usize>` 默认 `None`）+ `core/src/retriever/vector/types.rs:83`（`VectorError::DimMismatch{expected,got}` 已存在）
- `internal/config/config.go:31-64`（`Config` struct + `EmbeddingConfig`——task-34.2 add `[vector]` 段 mirror 源）+ `:206-264` decodeTOML / `:183-204` encodeTOML（TOML codec round-trip）+ `cmd/contextforge/main.go:254-268`（`setDataDirEnv`——`setVectorEnv` mirror 源，env-wins via `LookupEnv` 既有/不覆盖）+ `internal/daemon/daemon.go:201-208`（`exec.Command` 子进程继承 env——跨进程 env 桥论据）
- `core/src/server.rs:533-559`（`resolve_vector_backend` / `parse_vector_backend`——读 `CONTEXTFORGE_VECTOR_BACKEND`/`CONTEXTFORGE_VECTOR_DIM` 既有 env 路径，task-34.2 Go `[vector]`→env 经此被 core 拾取）
- `docs/specs/tasks/task-34.1-vector-dim-auto-negotiation.md §10` + `task-34.2-vector-backend-config-file.md §10`（真实测试结果 + 结论——ADR-039 ratify 依据）
- `docs/decisions/adr-039-vector-config-completeness.md`（§D1-D4 + Consequences Ratification 条款）
- `docs/decisions/adr-037-vector-backend-config-plumbing-and-completeness.md §Decision D1-D5 + §Follow-ups`（vector backend config 文件化 `[SPEC-DEFER:phase-future.vector-backend-config-file]` 已记于 Follow-ups——本 task add-only Phase 34 Amendment 落点：dim-negotiation + config-file 补全 Phase 32 起 env-plumbing）
- `internal/cli/smoke_syntax_test.go:344-376`（`TestTask334_SmokeV23GovernanceDebtCleanup2Step`——本 task `TestTask343` mirror 源）+ `scripts/console_smoke.sh`（v23 `[42/42]` 块 + banner，cf-v25-cfg → 本 task cf-v26-cfg offset +2）
- `docs/releases/v0.26.0-{evidence,artifacts}.md`（release docs 模板）

### 5.2 关键设计 — get_source_chunk isolation verify-only + 诚实 per-D ratify + backfill 待回填

- **`get_source_chunk` isolation verify-only（grounding 校正，不写新代码）**：隔离自 task-12.2（ADR-017 D1 Wave 2）即在 `search.rs:421-423`——`!req.workspace_id.is_empty()` → 候选 = `vec![req.workspace_id.clone()]`（仅该 workspace 开 collection），else → `workspace_store.list()` 聚合全 workspace 探测（chunk_id global-unique per `SqliteChunkStore` schema，任一开 collection 命中即正确，`:418-420` 注释）。verify-only 守护测试断言三态：(a) `workspace_id` 设非空、目标在该 workspace → 命中 `PbSourceChunk.workspace_id == req.workspace_id`；(b) `workspace_id` 设非空但 `chunk_id` 属另一 workspace → `Status::not_found`（候选不含该 workspace，不跨命中）；(c) `workspace_id` 空 → 聚合探测命中。**survey 把它表述为 gap = overstatement，本 task 据 ADR-013 校正为 already-present、交付守护测试不写生产代码**（symmetry honestly），grounding 校正记 ADR-039 D3。pass bar 守护测试三态绿。0 新 dep。
- ADR-039 ratify **逐 D 项据真实结果**：D1（vector-dim-auto-negotiation——factory `negotiate_vector_dim(dim, backend.expected_dim())` + `expected_dim` 默认 trait 方法 `None` 达成 🟢 pure-fn 单测；**默认 BruteForce dim-agnostic（`expected_dim()=None`）→ 默认构建协商接受任何 dim，无强制、与改前 `let _ = dim` byte-equivalent honest-caveat**；feature-backend live enforce 🟡 `[SPEC-DEFER:phase-future.vector-dim-feature-enforce]`）/ D2（vector-backend-config-file——Go `[vector]`→env 跨进程桥达成 🟢 TOML round-trip + `setVectorEnv` 单测；**env-wins**（显式 env 覆盖 config）+ 无 `[vector]` 段=不导出=unset=BruteForce byte-equiv + **Rust 0-dep 保持**（无 toml dep，ADR-008）；`daemon.Options.DataDir` 字段重构 honest-defer 承既有 `[SPEC-DEFER:phase-future.daemon-options-datadir]`）/ D3（`get_source_chunk` isolation already-present verify-only——grounding 校正：survey overstated as gap，隔离自 task-12.2 `search.rs:421-423` 在位，交付守护测试不写新代码 🟢）/ D4（默认行为 + 0-dep + 0-network + 既有契约不变 ADR-004/008——默认 BruteForce dim-agnostic + 无 `[vector]` 段 byte-equiv，0 新 dep，0 网络）。各 D 真实测试 / 实测结果待 34.1-34.2 实施后跑出再回填，不为「全 Accepted」伪造 feature-backend live DimMismatch 已验（ADR-013）。
- ADR-037 add-only Phase 34 Amendment 为 **add-only 注记**（不删/不改 ADR-037 D1-D5 正文 + 既有 Amendment 正文）：vector-dim-auto-negotiation（factory negotiate seam + `expected_dim` 默认 trait 方法，承 Phase 32 D1 env→config 接线把配置 dim 真正喂进协商而非静默丢弃）+ vector-backend-config-file（Go `[vector]`→env 跨进程桥，env-wins，承 ADR-037 §Follow-ups `[SPEC-DEFER:phase-future.vector-backend-config-file]` 兑现）。Phase 32 起 env-plumbing 经 Phase 34 收口完整（dim 不再丢弃 + backend 可经 config 文件选）。
- tag SHA / release run id / 镜像 digest 在 release docs 用 `<backfill: ...>` 待回填——真实 v0.27.0 tag/release 是 closeout 合入后的**用户授权步**（本轮已授权），post-tag-push backfill PR 填实（承 v0.8–v0.26 pattern）。
- smoke step `[43/43]` 为文档/状态步：验 default build init baseline 不变（ADR-004）+ 文档化三 task 状态（vector-dim-auto-negotiation + vector-backend-config-file + get_source_chunk isolation verify-only），staging `cf-v26-cfg`（offset +2）。

### 5.3 不变量

- 默认行为不变（ADR-004）：默认 BruteForce dim-agnostic（`expected_dim()=None`）→ `negotiate_vector_dim` 接受任何 dim → 与改前 `let _ = dim` byte-equivalent；无 `[vector]` config 段 → `setVectorEnv` 不导出 → env unset → `resolve_vector_backend` → `("", 0)` → BruteForce byte-equivalent；`get_source_chunk` 隔离行为不变（task-12.2 已在位，本 closeout 交付 verify-only 守护测试）。
- closeout 0 行为变更 / 0 新依赖（Phase 34 = 0 new dep，ADR-008——34.1 复用既有 `VectorError::DimMismatch` + 标准 trait 默认方法；34.2 复用 Go 既有 TOML codec + 标准 `os.Setenv`/`os.LookupEnv`；Rust 侧 0 toml dep；smoke 既有 step + denominator 不溯改 ADR-014 D5）。
- ENV WINS（向后兼容，ADR-004）：显式设置的 `CONTEXTFORGE_VECTOR_BACKEND`/`CONTEXTFORGE_VECTOR_DIM` env 覆盖 config 文件 `[vector]` 段（`setVectorEnv` 经 `LookupEnv` 既有不覆盖，仿 `setDataDirEnv`）。
- ADR-014 D5：历史 Phase 1-33 spec 不溯改；ADR-037 add-only Phase 34 Amendment 不改 D1-D5 正文 + 既有 Amendment 正文；roadmap §4 新 backlog 为 add-only 条目不删旧条目正文。
- add-only trait 默认方法（`expected_dim(self) -> Option<usize>` 默认 `None`，dim-agnostic backend 零改）+ add-only Go `[vector]` 段（无段=零值=byte-equiv）+ Rust 0-dep 保持（无 toml）不破既有契约（ADR-004/008）。
- honest 守线（ADR-013）：`get_source_chunk` isolation already-present（survey overstatement）如实记录于 §范围外 + ADR-039 D3，**verify-only 不写新代码**；feature-backend live dim enforce 🟡 honest-defer，不伪造已验。
- 真实 tag/release 经用户授权后执行（本轮已授权，ADR-012）；release docs tag/run/digest backfill 待回填，不预填伪造凭据。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [ ] **AC1**（get_source_chunk workspace-isolation verify-only 守护测试 🟢）: `core/src/data_plane/search.rs`（同源 test）verify-only 守护测试断言已自 task-12.2 在位的隔离（`:421-423`）：`workspace_id` 设非空、目标在该 workspace → 命中 `PbSourceChunk.workspace_id == req.workspace_id`；`workspace_id` 设非空但 `chunk_id` 属另一 workspace → `Status::not_found`（候选 scope 不跨 workspace 命中）；`workspace_id` 空 → 聚合全 workspace 探测命中（`workspace_store.list()`）；**不改 `search.rs` 生产代码**（survey overstated as gap，grounding 校正 ADR-013）；0 新 dep — verified by **TEST-34.3.1**。
- [ ] **AC2**（v0.27.0 closeout 🟢🟡）: smoke banner v23→v24 + step `[43/43]`（vector-config-completeness baseline + default build baseline intact，staging `cf-v26-cfg` offset +2）+ `TestTask343_SmokeV24VectorConfigCompletenessStep`（含无回归既有 `[37/37]`..`[42/42]`，denominator 不溯改）；v0.27.0 release docs（`v0.27.0-{evidence,artifacts}.md` `<backfill>` + README v0.27 段 + RELEASE_NOTES v0.27.0 段）+ ADR-039 per-D ratify `Proposed→Accepted`（D1 default BruteForce no-op honest-caveat + feature-enforce honest-defer；D2 env-wins + 无段 byte-equiv + Rust 0-dep；D3 isolation already-present verify-only grounding 校正；D4 默认行为 + 0-dep + 0-network）+ ADR-037 add-only Phase 34 Amendment（dim-negotiation + config-file）+ roadmap §3.16 推进记录 + §4 add-only 新 backlog + phase-34 §6 AC `[x]` + Status Done + adapter（Phase 34 Done/Tasks 3/ADR-039 Accepted）+ feature — verified by **TEST-34.3.2**。
- [ ] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中（CI spec-lint 权威）— verified by **TEST-34.3.3**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-34.3.1 | `get_source_chunk` workspace-isolation verify-only 守护测试：`workspace_id` 设 → 仅该 workspace chunk（`PbSourceChunk.workspace_id` 一致）；跨 workspace `chunk_id` → `Status::not_found`；空 → 聚合全 workspace 探测命中；断言 task-12.2 `search.rs:421-423` 已在位隔离，不写新生产代码（survey overstatement grounding 校正，ADR-013）；0 新 dep | `core/src/data_plane/search.rs`（同源 test 模块） | Draft |
| TEST-34.3.2 | smoke v24 step `[43/43]`（vector-config-completeness baseline + vector-dim-auto-negotiation/vector-backend-config-file/get_source_chunk-isolation 标记 + 无回归既有 denominator，staging `cf-v26-cfg`）+ `bash -n` 过 + `go test -run TestTask343` 过 + v0.27.0 release docs + ADR-039 per-D ratify Accepted（D1 default no-op honest-caveat / feature-enforce honest-defer + D3 isolation already-present 如实）+ ADR-037 add-only Phase 34 Amendment + roadmap §3.16+§4 + phase-34 §6 闭合 + adapter + feature | `scripts/console_smoke.sh` + `internal/cli/smoke_syntax_test.go` + release/ADR-039/ADR-037/roadmap/phase/adapter/feature | Draft |
| TEST-34.3.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Draft |

## 8. Risks

- **R1（低）get_source_chunk 守护测试误写新生产代码**：本 task 是 verify-only，隔离已自 task-12.2 在位（`search.rs:421-423`），若误改 `search.rs` 生产逻辑则越界。
  - **缓解**：守护测试仅断言已在位三态行为（`workspace_id` 设 → 仅该 workspace / 跨 workspace → not_found / 空 → 聚合），不触 `search.rs` 生产代码；grounding 校正记 ADR-039 D3（survey overstated as gap）。stop-condition：守护测试三态绿且 `search.rs` 生产代码 0 改动则 AC1 标 `[x]`。
- **R2（低）closeout 误报 isolation 为本 task 新修 / 误报 feature-enforce 为已验**：诚实风险。
  - **缓解**：§范围外 + ADR-039 D3 逐项如实——`get_source_chunk` isolation already-present（task-12.2，verify-only 不写新代码）；D1 默认 BruteForce dim-agnostic no-op honest-caveat（默认协商接受任何 dim、无强制）+ feature-backend live dim enforce 🟡 `[SPEC-DEFER:phase-future.vector-dim-feature-enforce]`，不伪造 feature live DimMismatch 已验（ADR-013）。stop-condition：任何「本 task 新修 isolation」/「feature-enforce 已验」表述须有真实凭据，否则标受阻维度 / backfill。
- **R3（低）smoke denominator 误溯改 / staging offset 错位**：新 step 须 `[43/43]`、staging `cf-v26-cfg`（offset +2），既有 `[37/37]`..`[42/42]` 不动。
  - **缓解**：`TestTask343` 无回归断言守护（mirror `TestTask334`）；ADR-014 D5；staging dir `cf-v26-cfg` 顺接 v25→v26（offset +2）。
- **R4（低）ADR-037 Amendment 误溯改 D1-D5 正文 / 既有 Amendment 正文**：须 add-only 追加 `## Amendment (Phase 34 / v0.27.0)` 不删既有正文（D5）。
  - **缓解**：仅追加 Phase 34 Amendment 段（dim-negotiation factory seam + `expected_dim` / config-file Go→env 桥），不改 ADR-037 D1-D5 正文 + 既有 Phase 32 Amendment 正文 + §Follow-ups（其 `[SPEC-DEFER:phase-future.vector-backend-config-file]` 经本 phase 兑现，Amendment 记兑现不删原条目）。

## 9. Verification Plan

```bash
# AC1 — get_source_chunk workspace-isolation verify-only 守护测试（已在位隔离三态）
cargo test -p contextforge-core get_source_chunk

# AC2 — smoke 语法 + syntax test
bash -n scripts/console_smoke.sh
go test ./internal/cli/ -run TestTask343

# AC2 — 文档闭合人工核（ADR-039 Accepted + per-D / ADR-037 add-only Phase 34 Amendment /
#        roadmap §3.16 + §4 新 backlog / phase §6 [x] / adapter Done / feature 存在）
# AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master

# 既有不退化（closeout 文档+smoke 不影响 workspace；34.1 trait 默认方法 / 34.2 Go config add-only）
cargo test --workspace && go test ./...
```

> **outward-facing 红线**：真实 v0.27.0 tag push + release run（cosign 真签 + GHCR 推送）是 closeout 合入后的**用户授权步**（本轮已授权，ADR-012）；本 task body 不预填真实凭据，release docs 的 tag/run/digest 用 `<backfill>` 待 post-tag-push backfill 填实 [SPEC-OWNER:user-authorized-release]。
>
> **honest-defer / verify-only 边界**：本 closeout 交付范围限于 `get_source_chunk` workspace-isolation verify-only 守护测试（🟢 已在位隔离 task-12.2，不写新代码）+ v0.27.0 closeout 文档/smoke；§范围外 grounding 校正（isolation already-present / feature-backend dim enforce `[SPEC-DEFER:phase-future.vector-dim-feature-enforce]` / daemon-options-datadir `[SPEC-DEFER:phase-future.daemon-options-datadir]` / rust-native config `[SPEC-DEFER:phase-future.vector-config-file-rust-native]`）**不实现新代码**，据 ADR-013 如实记录于 §范围外 + ADR-039 D3。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Draft（实施后回填 Done + 真实实证）

**§9 Verification 实证**（real evidence，待实施后本地全绿回填）：
- `cargo test -p contextforge-core get_source_chunk`——`get_source_chunk` workspace-isolation verify-only 守护测试三态（`workspace_id` 设 → 仅该 workspace chunk / 跨 workspace `chunk_id` → `Status::not_found` / 空 → 聚合全 workspace 探测命中），断言 task-12.2 `search.rs:421-423` 已在位隔离，0 生产代码改动（survey overstatement grounding 校正，ADR-013）。
- `bash -n scripts/console_smoke.sh` exit 0 + `go test ./internal/cli/ -run TestTask343` PASS——smoke v24 step `[43/43]`（vector-config-completeness + TEST-34.1.*/34.2.*/34.3.* + `expected_dim`/`[vector]`/`workspace_id` 标记 + 无回归 `[37/37]`..`[42/42]`，staging `cf-v26-cfg` offset +2）。
- `cargo test --workspace`（lib + 全 integration）+ `go test ./...` 全 PASS（既有不退化）；`cargo clippy --workspace --all-targets -- -D warnings` 0 warning。
- `bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威）。
- ADR-039 ratify 逐 D 据 34.1-34.2 真实测试 / 实测结果——待实测回填；D1 默认 BruteForce dim-agnostic no-op honest-caveat（默认协商接受任何 dim、无强制、byte-equiv）+ feature-backend live dim enforce 🟡 `[SPEC-DEFER:phase-future.vector-dim-feature-enforce]`、D2 env-wins + 无 `[vector]` 段 byte-equiv + Rust 0-dep 保持（无 toml dep）、D3 `get_source_chunk` isolation already-present verify-only（grounding 校正 survey overstated as gap）、D4 默认行为 + 0-dep + 0-network + 既有契约不变（ADR-004/008）如实记录，不强 ratify（ADR-013）。
- ADR-037 add-only `## Amendment (Phase 34 / v0.27.0)`（vector-dim-auto-negotiation factory negotiate seam + `expected_dim` 默认 trait 方法 / vector-backend-config-file Go `[vector]`→env 跨进程桥 env-wins）——据真实落地后回填，不溯改 D1-D5 正文 + 既有 Phase 32 Amendment 正文（ADR-014 D5）。
- roadmap §3.16 Phase 34 推进记录 + §4 add-only 新 backlog（vector-dim-feature-enforce / daemon-options-datadir / vector-config-file-rust-native）——add-only 落地后回填。
- 真实 v0.27.0 tag/release（cosign 真签 + GHCR 推送）经用户授权（本轮已授权）→ post-tag-push backfill 填实 evidence/artifacts 待回填（tag SHA / run id / digest，承 v0.8–v0.26 pattern，不预填伪造凭据 ADR-013）[SPEC-OWNER:user-authorized-release]。

**计划改动文件**：
- `core/src/data_plane/search.rs`（同源 test 模块）——`get_source_chunk` workspace-isolation verify-only 守护测试（`workspace_id` 设 → 仅该 workspace / 跨 workspace → not_found / 空 → 聚合），断言 task-12.2 `:421-423` 已在位隔离，0 生产代码改动。
- `scripts/console_smoke.sh`——banner v23→v24 + v24 changelog 块 + step `[43/43]`（vector-config-completeness baseline：vector-dim-auto-negotiation + vector-backend-config-file + get_source_chunk isolation verify-only；default build init baseline 不变，staging `cf-v26-cfg` offset +2）。
- `internal/cli/smoke_syntax_test.go`——`TestTask343_SmokeV24VectorConfigCompletenessStep`（mirror `TestTask334`，断言 `[43/43]` + 标记 + 无回归既有 `[37/37]`..`[42/42]`，denominator 不溯改）。
- `docs/releases/v0.27.0-{evidence,artifacts}.md`（新，tag/run/digest `<backfill>` 待回填）+ `README.md` v0.27 段 + `RELEASE_NOTES.md` v0.27.0 段。
- `docs/decisions/adr-039-vector-config-completeness.md`——Status Proposed→Accepted（per-D 限定）+ `## Ratification（v0.27.0 / task-34.3）` 节。
- add-only Amendment：`docs/decisions/adr-037-vector-backend-config-plumbing-and-completeness.md`——`## Amendment (Phase 34 / v0.27.0)`（dim-negotiation factory seam + `expected_dim` / config-file Go→env 桥），不溯改 D1-D5 正文 + 既有 Phase 32 Amendment 正文。
- `docs/roadmap.md`——§3 新增 §3.16 Phase 34 推进记录 + §4 add-only 新 backlog。
- `docs/specs/phases/phase-34-vector-config-completeness.md`——Status Draft→Done + §6 AC `[x]`（honest per-dim）。
- `docs/s2v-adapter.md`——Phase 34 Done + Tasks 3 + ADR-039 Accepted + BDD 行。
- `test/features/phase-34-vector-config-completeness.feature`（已创建）。
