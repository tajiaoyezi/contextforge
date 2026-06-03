# Task `34.1`: `vector-dim-auto-negotiation — factory.rs select_vector_backend(name, dim) 不再静默丢弃 CONTEXTFORGE_VECTOR_DIM（替代 let _ = dim 直接弃用）；镜像 embedding::factory::negotiate_dim 加纯函数 negotiate_vector_dim + VectorBackend::expected_dim() 默认 None；0 新 dep / 0 schema migration；默认 BruteForce dim-agnostic 不强校（ADR-004 byte-equivalent）+ feature-backend live enforce 诚实延后`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 34 (vector-config-completeness)
**Dependencies**: 既有 `core/src/retriever/vector/factory.rs`（task-29.1 `select_vector_backend` 工厂，Phase 29 已交付；task-32.1 env 热路径选 backend + dim，Phase 32 已交付经 `server.rs` `resolve_vector_backend` 注入）/ `core/src/embedding/factory.rs`（task-22.1 `negotiate_dim`，Phase 22 已交付——本 task vector 侧镜像源）/ `core/src/retriever/vector/types.rs`（`VectorError::DimMismatch{expected,got}` 经 task-18.1 已存在 `:83`，本 task 复用不新增变体）/ ADR-037（vector-backend-config-plumbing-and-completeness，dim-negotiation 为 add-only Phase 34 Amendment @ task-34.3 closeout）/ ADR-034（live-vector-recall，工厂 + 默认 BruteForce byte-equivalent 前序）/ ADR-004（local-first-privacy-baseline，默认行为 + 既有契约 + 公共签名不变 + 默认 build 0 新 dep）/ ADR-008（dep add-only，Phase 34 = 0 新 dep）/ ADR-013（禁伪造红线——默认 BruteForce no-op caveat 据实声明，不夸大为已生效强校；feature-backend live enforce 据实延后不预填）/ ADR-012（main-agent-governance-autonomy）/ ADR-014 D1-D5（第二十五次激活）

## 1. Background

Phase 32（task-32.1）已让 `server.rs` `resolve_vector_backend`（`:540`）从 env（`CONTEXTFORGE_VECTOR_BACKEND` / `CONTEXTFORGE_VECTOR_DIM`）解析出 `(backend_name, vec_dim)` 并经 `select_vector_backend(&backend_name, vec_dim)`（`:343` hybrid / `:388` semantic 两热路径）注入工厂，但 **工厂收到 `dim` 后静默丢弃**——本 task 聚焦让 vector 工厂像 embedding 工厂一样真正对 `dim` 做协商，与 embedding 侧对齐：

- **B1 vector 工厂静默丢弃配置 dim**：`core/src/retriever/vector/factory.rs` 的 `select_vector_backend(name, dim)`（`:33-39`）在选 backend 前以 `let _ = dim;`（`:39`）直接弃用入参——`server.rs` 经 `resolve_vector_backend`（`:540`）解析、`:343` / `:388` 传入的 `CONTEXTFORGE_VECTOR_DIM` 被无声丢掉，配置的 dim 既不参与协商、也不在冲突时报错。这与 embedding 侧 `select_provider` 末尾 `negotiate_dim(provider.dim(), dim)?`（`embedding/factory.rs:81`）的真协商行为不对称（vector 侧有缺口）。
- **B2 embedding 侧已有可镜像的纯协商函数**：`core/src/embedding/factory.rs` 的 `negotiate_dim(provider_dim, requested)`（`:88-96`）是纯函数——`requested == 0` ⇒ 用 provider 默认（永不冲突，`:89`）；非零 `requested != provider_dim` ⇒ 硬 `DimMismatch { expected: requested, got: provider_dim }`（`:90-93`），工厂从不静默截断 / 补零。本 task vector 侧镜像该形态加一个纯函数 `negotiate_vector_dim`，把「请求 dim vs backend 声明 dim」的协商抽成可单测 seam。
- **B3 DimMismatch 变体已存在，0 新增**：`core/src/retriever/vector/types.rs` 的 `VectorError::DimMismatch { expected, got }`（`:83`，`#[error("invalid embedding dimension: expected {expected}, got {got}")]`）经 task-18.1 trait freeze 已存在，本 task 直接复用——不新增错误变体、不改 `VectorError` enum（`#[non_exhaustive]` add-only-safe 前提下本 task 连 add 都不需要）。
- **B4 默认 BruteForce dim-agnostic 诚实 caveat（核实后据实声明，非夸大）**：`VectorBackend` 三签名基 trait（`traits.rs:11-16`）无 dim 声明能力——本 task 加 `expected_dim(self) -> Option<usize>` **默认实现返回 `None`**（dim-agnostic）；`BruteForceVectorBackend`（`brute_force.rs:39` `impl VectorBackend`）沿用该默认（`None`），故**默认 build 下协商对任意 dim 放行**（`None` 声明 ⇒ `negotiate_vector_dim` 返回 `Ok`，无强校，与改前 `let _ = dim` 行为 byte-equivalent，ADR-004）。真强校只对**声明 dim 的 feature backend**（qdrant / lancedb / sqlite-vec）生效，其 live 行使须 feature build → 诚实延后 [SPEC-DEFER:phase-future.vector-dim-feature-enforce]（不预填、不夸大为默认已生效，ADR-013）。

经核 embedding 侧 `negotiate_dim` 已有纯函数协商基线（`embedding/factory.rs:88-96`），本 task vector bound 镜像该形态（纯函数 + DimMismatch），为 code-local 🟢 可单测，0 新 dep（仅既有 `VectorError` 变体）+ 0 schema migration（纯逻辑无表）。

## 2. Goal

(1) **B1/B2**：vector 工厂不再静默丢弃配置 dim——`select_vector_backend`（`:33-39`）把 `let _ = dim;`（`:39`）替换为对纯函数 `negotiate_vector_dim(dim, backend.expected_dim())` 的调用（选出 backend 后协商），冲突时返回 `VectorError::DimMismatch`，从不静默截断 / 补零。纯函数语义镜像 `negotiate_dim`：`requested == 0` ⇒ `Ok`；backend 声明 `None`（dim-agnostic）⇒ `Ok`；声明 `Some(d)` 且 `requested == d` ⇒ `Ok`；声明 `Some(d)` 且非零 `requested != d` ⇒ `DimMismatch { expected: requested, got: d }`。(2) **B3/B4**：`VectorBackend` trait（`traits.rs:11-16`）加 `expected_dim(self) -> Option<usize>` 默认实现返回 `None`（add-only，三既有签名不动，ADR-014 D5）；`BruteForceVectorBackend` 沿用默认（`None`，dim-agnostic）。(3) **caveat 据实**：默认 BruteForce no-op（`expected_dim()==None` ⇒ 协商放行任意 dim，byte-equivalent）+ feature-backend live enforce 延后 [SPEC-DEFER:phase-future.vector-dim-feature-enforce] 据实记入 spec / ADR-039 D1（ADR-013 不夸大为默认已生效强校）。

pass bar：`negotiate_vector_dim` 纯函数四路经确定性单测验证（`0` ⇒ Ok / `None`-declared ⇒ Ok / matching ⇒ Ok / mismatch ⇒ DimMismatch）（🟢）；默认 BruteForce 路径任意 dim 放行 + byte-equivalent（改前 `let _ = dim` 对默认 build 的可观察行为不变，ADR-004）（🟢）；`VectorBackend` 三既有签名（`name` / `version` / `is_local` / `requires_embedding`）不变 + `expected_dim` 默认 `None`（add-only）；0 新 dep（ADR-008）、0 schema migration、不新增 `VectorError` 变体（复用既有 `:83`）；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `core/src/retriever/vector/traits.rs`——`VectorBackend` 基 trait（`:11-16`）加 `fn expected_dim(&self) -> Option<usize> { None }`（add-only 默认实现，doc 注释记「dim-agnostic backend 返回 None；声明 dim 的 feature backend 覆写返回 Some(d)」）；三既有签名（`name` / `version` / `is_local` / `requires_embedding`）不动（ADR-014 D5）。`BruteForceVectorBackend`（`brute_force.rs:39` `impl VectorBackend`）**不覆写**，沿用默认 `None`（dim-agnostic）。
- 加纯函数 `negotiate_vector_dim(requested: usize, declared: Option<usize>) -> Result<(), VectorError>`——镜像 `embedding/factory.rs:88-96` `negotiate_dim` 形态：`requested == 0` ⇒ `Ok(())`；`declared == None` ⇒ `Ok(())`（dim-agnostic 放行）；`declared == Some(d)` 且 `requested == d` ⇒ `Ok(())`；`declared == Some(d)` 且非零 `requested != d` ⇒ `Err(VectorError::DimMismatch { expected: requested, got: d })`（复用既有 `:83` 变体，不新增）。
- 改 `select_vector_backend`（`:33-39`）——把 `let _ = dim;`（`:39`）替换为：选出 `backend` 后 `negotiate_vector_dim(dim, backend.expected_dim())?;`（默认 build 下 BruteForce `expected_dim()==None` ⇒ 放行，与改前 byte-equivalent；feature backend 声明 `Some(d)` 时冲突报 `DimMismatch`）。公共签名（`name: &str, dim: usize`）**不变**。
- 同源测试：`factory.rs` 同源 test（镜像既有 TEST-29.1.x 风格）断言 `negotiate_vector_dim` 四路 + 默认 BruteForce 路径任意 dim 放行（byte-equivalent）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- feature backend（qdrant / lancedb / sqlite-vec）live dim enforce 行使（须 feature build + 真实 backend 声明 dim 的端到端校验）[SPEC-DEFER:phase-future.vector-dim-feature-enforce]——本 task 仅在默认 build 把协商 seam 接上（BruteForce dim-agnostic no-op），feature backend 覆写 `expected_dim()` 返 `Some(d)` 的 live 强校须 feature build 延后，不预填数值（ADR-013）。
- index-time dim 协商 / 索引内向量维度对齐（与构造期协商正交）——本 task 只接构造期 `select_vector_backend` 协商点，index-time 维度校验非本 task 范围。
- embedding-dim ↔ vector-dim 跨层一致性强校（embedder 实际 dim 与 vector backend 声明 dim 联合校验）——本 task 范围限于 vector 工厂内「请求 dim vs backend 声明 dim」协商，跨层联合校验延后。
- 真实 release tag / run-id / digest（v0.27.0）[SPEC-OWNER:task-34.3-closeout]（ADR-012 用户授权后回填）。

## 4. Actors

- 主 agent（ADR-012 自治）
- `select_vector_backend`（`core/src/retriever/vector/factory.rs:33-39`，本 task 把 `:39` `let _ = dim;` 替换为 `negotiate_vector_dim(dim, backend.expected_dim())?` 调用点）
- `negotiate_vector_dim`（`core/src/retriever/vector/factory.rs` 新增纯函数，镜像 `embedding/factory.rs:88-96` `negotiate_dim`，可单测 seam）
- `VectorBackend` 基 trait（`core/src/retriever/vector/traits.rs:11-16`，本 task 加 `expected_dim` 默认 `None`）
- `BruteForceVectorBackend`（`core/src/retriever/vector/brute_force.rs:39` `impl VectorBackend`，沿用默认 `expected_dim()==None`，dim-agnostic）
- `VectorError::DimMismatch`（`core/src/retriever/vector/types.rs:83`，本 task 复用既有变体，不新增）
- `resolve_vector_backend` / `server.rs` 两热路径（`core/src/server.rs:540` / `:343` / `:388`，env 解析 + dim 注入来源，本 task 不改，仅令其传入的 dim 不再被丢弃）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/retriever/vector/factory.rs:33-39`（`select_vector_backend(name, dim)` 签名 + `:39` `let _ = dim;` 静默丢弃点——本 task 替换为 `negotiate_vector_dim(dim, backend.expected_dim())?`）
- `core/src/embedding/factory.rs:88-96`（`negotiate_dim(provider_dim, requested)` 纯函数——`:89` `requested != 0 && provider_dim != requested` ⇒ `:90-93` `DimMismatch`；本 task vector 侧 `negotiate_vector_dim` 的镜像源）+ `:81`（`select_provider` 末尾 `negotiate_dim(provider.dim(), dim)?` 调用形态——本 task `select_vector_backend` 镜像该调用点）
- `core/src/retriever/vector/types.rs:83`（`VectorError::DimMismatch { expected, got }` `#[error("invalid embedding dimension: expected {expected}, got {got}")]`——本 task 复用既有变体，`VectorError` `#[non_exhaustive]` `:78` 不改）
- `core/src/retriever/vector/traits.rs:11-16`（`VectorBackend` 基 trait 三既有签名 `name` / `version` / `is_local` / `requires_embedding`——本 task add-only 加 `expected_dim` 默认 `None`，既有签名不动，ADR-014 D5）
- `core/src/retriever/vector/brute_force.rs:39`（`impl VectorBackend for BruteForceVectorBackend`——沿用 `expected_dim` 默认 `None`，dim-agnostic，默认 build no-op 协商点）
- `core/src/server.rs:540`（`resolve_vector_backend` env 解析 `CONTEXTFORGE_VECTOR_BACKEND` / `CONTEXTFORGE_VECTOR_DIM`）+ `:343` / `:388`（hybrid / semantic 两热路径 `select_vector_backend(&backend_name, vec_dim)` 注入点——本 task 令其传入的 dim 不再被丢弃）
- `docs/decisions/adr-037-*.md`（vector-backend-config-plumbing-and-completeness；dim-negotiation 为 add-only Phase 34 Amendment 落点 @ task-34.3 closeout）+ `docs/decisions/adr-039-vector-config-completeness.md §D1`（本 task 即其原文实现）+ `docs/decisions/adr-034-live-vector-recall.md`（工厂 + 默认 BruteForce byte-equivalent 前序）

### 5.2 关键设计 — 纯函数 dim 协商 seam + expected_dim 默认 None（0 dep / 0 migration / 默认 byte-equivalent）

- **B1/B2 纯函数 `negotiate_vector_dim` 镜像 `negotiate_dim`**：新增 `pub(crate) fn negotiate_vector_dim(requested: usize, declared: Option<usize>) -> Result<(), VectorError>`，逻辑镜像 `embedding/factory.rs:88-96`：(a) `requested == 0` ⇒ `Ok(())`（用 backend 默认 dim，永不冲突）；(b) `declared == None` ⇒ `Ok(())`（dim-agnostic backend 放行任意 dim）；(c) `declared == Some(d)` 且 `requested == d` ⇒ `Ok(())`；(d) `declared == Some(d)` 且非零 `requested != d` ⇒ `Err(VectorError::DimMismatch { expected: requested, got: d })`。`select_vector_backend`（`:33-39`）选出 `backend` 后调 `negotiate_vector_dim(dim, backend.expected_dim())?;` 替换 `:39` `let _ = dim;`，从不静默截断 / 补零（与 embedding 侧对称）。纯函数无 I/O、无锁、可独立单测（pass bar 四路）。
- **B3 复用既有 DimMismatch 不新增变体**：`VectorError::DimMismatch { expected, got }`（`types.rs:83`）已存在，本 task 直接用——`expected = requested`、`got = declared 的 d`（与 `negotiate_dim` 的 `expected: requested, got: provider_dim` 同语义）。`VectorError` enum（`#[non_exhaustive]` `:78`）不改，0 错误变体增量。
- **B4 expected_dim 默认 None（add-only trait 方法）**：`VectorBackend`（`traits.rs:11-16`）加 `fn expected_dim(&self) -> Option<usize> { None }`——**带默认实现**故对既有所有 impl 源码兼容（`BruteForceVectorBackend` / `NoopVectorBackend` 等不需任何改动即沿用 `None`）；三既有签名（`name` / `version` / `is_local` / `requires_embedding`）不动（task-18.1 trait freeze + ADR-014 D5 add-only）。声明 dim 的 feature backend（qdrant / lancedb / sqlite-vec）**可**覆写返 `Some(d)`，其 live 强校须 feature build → [SPEC-DEFER:phase-future.vector-dim-feature-enforce]。
- **默认 BruteForce no-op 诚实 caveat**：默认 build 下 `BruteForceVectorBackend::expected_dim() == None` ⇒ `negotiate_vector_dim(dim, None)` 对任意 `dim` 返 `Ok` ⇒ **默认 build 无 dim 强校**，对默认 build 可观察行为与改前 `let _ = dim`（同样放行任意 dim）byte-equivalent（ADR-004）。spec / ADR-039 D1 据实记此 caveat：本 task 在默认路径接上协商 seam（不再静默丢弃 + 可单测），真强校只在 feature backend 声明 `Some(d)` 时生效（ADR-013 不夸大为默认已生效强校）。
- **协商点位置镜像 embedding 侧**：协商在 `match name { ... }` 选出 `backend` **之后**（`negotiate_dim` 在 `select_provider` 末尾 `:81` 同样选出 provider 后才协商）——故未知 backend / feature-off 仍先返各自既有 honest Err（TEST-29.1.2 / TEST-32.2.1 不退化），dim 协商只在 backend 成功选出后进行。

### 5.3 不变量

- 默认行为不变（ADR-004）：默认 build（无 feature）下 `select_vector_backend("", dim)` / `select_vector_backend("brute", dim)` 对任意 `dim` 仍返 BruteForce 成功（`expected_dim()==None` ⇒ 协商放行），对默认 build 可观察行为与改前 `let _ = dim` byte-equivalent；既有 TEST-29.1.1 / TEST-29.1.2 / TEST-32.2.1 / TEST-32.2.2 不退化。
- 既有契约不变：`select_vector_backend` 公共签名（`name: &str, dim: usize`）兼容；`VectorBackend` 三既有签名（`name` / `version` / `is_local` / `requires_embedding`）不动，`expected_dim` 为带默认实现的 add-only 方法（既有 impl 源码兼容，ADR-014 D5）；`VectorError` enum（`#[non_exhaustive]`）不改、不新增变体（复用既有 `DimMismatch` `:83`）；`VectorIndexer` / `VectorSearcher` / `VectorStore` 行为不变。
- 0 新代码依赖（ADR-008）：仅既有 `VectorError::DimMismatch` + 纯逻辑，无 Cargo 依赖增量；Rust core 默认 build 仍 0 vector dep（ADR-004 local-first）。
- 0 schema migration：纯协商逻辑无表 / 无持久化，不加列、不 `ALTER`、不新增编号 migration。
- dim 不再被静默丢弃：`server.rs` 经 `resolve_vector_backend`（`:540`）注入（`:343` / `:388`）的 `CONTEXTFORGE_VECTOR_DIM` 现进入 `negotiate_vector_dim` 协商（默认 BruteForce 放行；feature backend 声明 `Some(d)` 时冲突报 `DimMismatch`），不再 `let _ = dim` 弃用。
- 默认 no-op 诚实边界（ADR-013）：默认 BruteForce dim-agnostic（`expected_dim()==None`）⇒ 默认 build 无 dim 强校，本 task 在默认路径接上协商 seam（不再静默丢弃 + 可单测），不夸大为默认已生效强校；feature backend live enforce → [SPEC-DEFER:phase-future.vector-dim-feature-enforce] 据实延后，不预填。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [ ] **AC1**（`negotiate_vector_dim` 纯函数四路 + 默认 BruteForce byte-equivalent 🟢）: 纯函数 `negotiate_vector_dim(requested, declared)` ——`requested == 0` ⇒ `Ok`；`declared == None` ⇒ `Ok`；`declared == Some(d)` 且 `requested == d` ⇒ `Ok`；`declared == Some(d)` 且非零 `requested != d` ⇒ `Err(VectorError::DimMismatch { expected: requested, got: d })`（复用既有 `:83`，不新增变体）。`select_vector_backend`（`:33-39`）`:39` `let _ = dim;` 替换为 `negotiate_vector_dim(dim, backend.expected_dim())?`；默认 BruteForce 路径（`expected_dim()==None`）任意 dim 放行（byte-equivalent，ADR-004）；`VectorBackend` 加 `expected_dim` 默认 `None`（三既有签名不动，add-only）；**0 新 dep + 0 schema migration** — verified by **TEST-34.1.1**（纯函数四路）+ **TEST-34.1.2**（默认 BruteForce 路径任意 dim 放行 byte-equivalent）
- [ ] **AC2**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-34.1.3**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-34.1.1 | `negotiate_vector_dim` 纯函数四路：`0` ⇒ Ok / `None`-declared ⇒ Ok / matching `Some(d)==requested` ⇒ Ok / mismatch 非零 `requested != Some(d)` ⇒ `DimMismatch { expected: requested, got: d }`（复用既有 `types.rs:83`，0 新 dep + 0 schema migration） | `core/src/retriever/vector/factory.rs`（同源 test） | Planned |
| TEST-34.1.2 | 默认 BruteForce 路径：`select_vector_backend("", dim)` / `("brute", dim)` 对任意 `dim`（含非零）仍返 brute-force 成功（`expected_dim()==None` ⇒ 协商放行），与改前 `let _ = dim` byte-equivalent（ADR-004 默认行为不变）；既有 TEST-29.1.x / TEST-32.2.x 不退化 | `core/src/retriever/vector/factory.rs` | Planned |
| TEST-34.1.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（中）`expected_dim` 加 trait 方法破既有 impl 编译**：`VectorBackend` 是基 trait，多 backend（BruteForce / Noop / qdrant / lancedb / sqlite-vec）impl 之，新增无默认实现的方法会破全部 impl。
  - **缓解**：`expected_dim` **带默认实现** `{ None }` → 所有既有 impl 不需任何改动即沿用 `None`（add-only 源码兼容，ADR-014 D5）；feature backend 选择性覆写（live 强校延后）。stop-condition：`cargo build --workspace`（默认 build）+ feature build 编译不过则 AC1 不标 `[x]`。
- **R2（中）默认协商误改默认 build 可观察行为**：协商 seam 接上后若默认 BruteForce 误返 `Some(d)` 或非零 dim 误报 `DimMismatch`，会破默认 build byte-equivalence。
  - **缓解**：BruteForce 沿用默认 `expected_dim()==None` → `negotiate_vector_dim(dim, None)` 对任意 dim 恒 `Ok`，与改前 `let _ = dim`（放行任意 dim）可观察行为一致；TEST-34.1.2 断言默认路径任意 dim 放行；TEST-29.1.x / TEST-32.2.x 不退化。stop-condition：默认 build 任一既有 vector 工厂测试退化则不标 `[x]`。
- **R3（低）默认 no-op caveat 被误读为默认已生效强校**：默认 BruteForce dim-agnostic 无强校，易被夸大为默认 build 已对 dim 强校。
  - **缓解**：spec §1 B4 / §5.2 B4 / §5.3 + ADR-039 D1 据实记「默认 BruteForce no-op（byte-equivalent），真强校只在 feature backend 声明 `Some(d)` 时生效」；feature-backend live enforce → [SPEC-DEFER:phase-future.vector-dim-feature-enforce]（ADR-013 不夸大、不预填）。
- **R4（低）协商点位置致 honest Err 顺序变化**：dim 协商若放在选 backend 之前，会令 feature-off / unknown-name 的既有 honest Err 被 dim 协商抢先。
  - **缓解**：协商放在 `match name` 选出 `backend` **之后**（镜像 `negotiate_dim` 在 `select_provider` 末尾 `:81`）→ 未知 backend / feature-off 仍先返各自既有 honest Err（TEST-29.1.2 / TEST-32.2.1 顺序不退化），dim 协商只在 backend 成功选出后进行。

## 9. Verification Plan

```bash
# 1. AC1 — negotiate_vector_dim 纯函数四路 + 默认 BruteForce byte-equivalent（确定性单测）
cargo test -p contextforge-core retriever::vector::factory

# 2. 不退化（全量 + 既有 vector 工厂测试 TEST-29.1.x / TEST-32.2.x）
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# 3. AC2 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.vector-dim-negotiation-defer-note]：本 task 仅在默认 build 把 vector 工厂 dim 协商 seam 接上（🟢 纯函数可单测，0 新 dep + 0 schema migration）——`select_vector_backend` 不再静默丢弃 `dim` + `negotiate_vector_dim` 四路 + `VectorBackend::expected_dim` 默认 `None`。默认 BruteForce dim-agnostic（`expected_dim()==None`）⇒ 默认 build 无 dim 强校，对默认 build 可观察行为 byte-equivalent（ADR-004，据实声明非默认已生效强校）；声明 dim 的 feature backend（qdrant / lancedb / sqlite-vec）live dim enforce 行使须 feature build → [SPEC-DEFER:phase-future.vector-dim-feature-enforce]；index-time dim 协商、embedding-dim ↔ vector-dim 跨层联合强校均不在本 task 范围。feature-backend live 强校数值不预填，真实跑出后回填（ADR-013 不伪造）。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Draft

**§9 Verification 计划** (will record real evidence at impl)：
- AC1：`cargo test -p contextforge-core retriever::vector::factory` —— `negotiate_vector_dim` 四路（`0` ⇒ Ok / `None`-declared ⇒ Ok / matching ⇒ Ok / mismatch ⇒ `DimMismatch { expected, got }`，复用既有 `types.rs:83`）+ `select_vector_backend` `:39` `let _ = dim;` 替换为 `negotiate_vector_dim(dim, backend.expected_dim())?` + 默认 BruteForce 路径任意 dim 放行（byte-equivalent，ADR-004）+ `VectorBackend` 加 `expected_dim` 默认 `None`（三既有签名不动 add-only）；0 新 dep + 0 schema migration（真实测试结果待实施回填，ADR-013 不伪造）。
- AC2：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威）。
- 不退化：`cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings` —— 既有 TEST-29.1.x / TEST-32.2.x vector 工厂测试不退化；默认 build 可观察行为 byte-equivalent。真实结果待实施回填。
- 0 新 dep / 0 schema migration / 默认 BruteForce no-op byte-equivalent / 既有契约不变 / feature-backend live enforce 诚实延后 真实结果待实施回填（ADR-013 数值不预填，真实跑出才记数）。

**实际改动文件**（计划，待实施回填）：
- `core/src/retriever/vector/factory.rs`——`select_vector_backend`（`:33-39`）`:39` `let _ = dim;` 替换为 `negotiate_vector_dim(dim, backend.expected_dim())?`（选出 backend 后协商，公共签名不变）；新增纯函数 `negotiate_vector_dim`（镜像 `embedding/factory.rs:88-96` `negotiate_dim`，复用 `VectorError::DimMismatch` `types.rs:83`）。+ 同源 test（TEST-34.1.1 四路 + TEST-34.1.2 默认 BruteForce byte-equivalent）。
- `core/src/retriever/vector/traits.rs`——`VectorBackend`（`:11-16`）加 `fn expected_dim(&self) -> Option<usize> { None }`（add-only 默认实现，三既有签名不动，ADR-014 D5）；`BruteForceVectorBackend`（`brute_force.rs:39`）沿用默认 `None`（dim-agnostic，不覆写）。
- `docs/decisions/adr-037-*.md` dim-negotiation add-only Phase 34 Amendment 落点在 task-34.3 closeout（非本 task body）。
