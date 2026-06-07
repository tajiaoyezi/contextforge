# Task `32.1`: `vector-backend-config-plumbing — server.rs hybrid + semantic 两热路径经 env/config 选 vector backend（factory 接线，替代硬编码 select_vector_backend("",0)）；未设/"" → BruteForce byte-equivalent（ADR-004 默认行为不变 + 0 新 dep）`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 32 (vector-backend-config-plumbing-and-completeness)
**Dependencies**: 既有 `core/src/retriever/vector/factory.rs`（task-29.1 `select_vector_backend(name,dim)` 工厂，Phase 29 已交付，ADR-034）/ `core/src/server.rs`（CoreService `data_dir` 注入 + `resolve_data_dir` env pattern + hybrid / semantic 两热路径，Phase 6 / 19 / 29 已交付）/ ADR-034（production-vector-live-recall，factory 为既有契约，本 task 把 default 选择改为 env 可配置，sqlite-vec arm 补全在 task-32.2）/ ADR-023（vector-backend，0-vector-dep baseline 守线）/ ADR-004（local-first-privacy-baseline，默认行为 + 既有契约不变）/ ADR-016（Go thin proxy + Rust SoT 两进程布局，env 注入不改拓扑）/ ADR-013（禁伪造红线——真实 recall/latency 数值不预填）/ ADR-012（main-agent-governance-autonomy）/ ADR-014 D1-D5（第二十三次激活）

## 1. Background

Phase 29（ADR-034 D1）已落地 `select_vector_backend(name, dim)` 工厂集中 backend 选择，但 `server.rs` 热路径**仅注入 default**——backend 名仍硬编码 `""`，env / config 无法选 qdrant / lancedb / sqlite-vec。本 task 聚焦把 backend 选择从硬编码 default 提升为 env 可配置，default 保形：

- **B1 hybrid 热路径硬编码 default**：`core/src/server.rs:340` 的 `select_vector_backend("", 0)`（块 `:334-341`，注释 `:337-339` 已记「No vector config is plumbed to the server yet, so default args ("", 0)」）——hybrid 检索路径恒走 `""` → `BruteForceVectorBackend`，无法据部署选 qdrant / lancedb / sqlite-vec。
- **B2 semantic 热路径硬编码 default**：`core/src/server.rs:382` 的 `select_vector_backend("", 0)`（块 `:376-383`，注释 `:380-381` 同记「default "", 0 → byte-equivalent to the hardcoded BruteForceVectorBackend::new()」）——semantic 检索路径同样恒走 default backend。
- **B3 factory `dim` 参数未用**：`core/src/retriever/vector/factory.rs:37` 的 `let _ = dim;`（doc `:29-30`「`dim` mirrors `select_provider`'s signature for later embedder-dim negotiation」）——`dim` 自 task-29.1 起即为预留形参，BruteForce arm 对任意 dim 工作、feature backend 于 index time 协商 dim，本 task 沿用此契约（dim 自动协商不在本 task 范围 [SPEC-DEFER:phase-future.vector-dim-auto-negotiation]）。
- **B4 已有 env 注入 pattern 可复用**：`core/src/server.rs:504-525` 的 `resolve_data_dir` 已建立「cmd-arg / `$CONTEXTFORGE_DATA_DIR`（`:511`）/ home / cwd」四级 env 解析 pattern，CoreService 持 `data_dir: PathBuf`（`:52`，`new` `:57` 注入）；vector backend 选择可镜像该 env pattern（`CONTEXTFORGE_VECTOR_BACKEND`），无须新机制。

经核 task-29.1 已为 default 选择建立 byte-equivalence 测试基线（`factory.rs:76` TEST-29.1.1 default/"brute" → "brute-force"），本 task 范围内的两热路径接线 + default byte-equiv 为 code-local 🟢 可单测，0 新 dep（仅 env 读取 + 既有工厂调用）。

## 2. Goal

(1) **B1/B2**：`server.rs` hybrid（`:340`）+ semantic（`:382`）两热路径的 backend 名由硬编码 `""` 改为经 env（`CONTEXTFORGE_VECTOR_BACKEND`）解析后传入 `select_vector_backend`——未设 / 空 → `""` → `BruteForceVectorBackend`（与改前 byte-equivalent）；设为 `qdrant` / `lancedb` / `sqlite-vec` 经工厂选对应 backend（feature 未开 → 工厂 honest Err 浮出为 `Status::internal`，不静默回落）。(2) **B3**：`dim` 沿用 task-29.1 契约（占位形参，env 可选携带 dim 透传，BruteForce arm 不约束 dim）；dim 自动协商不在本 task 范围 [SPEC-DEFER:phase-future.vector-dim-auto-negotiation]。(3) env 解析镜像 `resolve_data_dir`（`:504-525`）pattern，default 保形。

pass bar：两热路径 env 接线经确定性单测 / 集成测试验证（🟢）；未设 env → default `""` → BruteForce byte-equivalent（既有语义 / hybrid 行为 + TEST-29.1.x 不退化，🟢）；默认行为 / proto / 既有契约不变（ADR-004）+ 0 新 dep（ADR-008/ADR-023 守线）；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `core/src/server.rs`——加一个镜像 `resolve_data_dir`（`:504-525`）的 env 解析 helper（如 `resolve_vector_backend()` 读 `CONTEXTFORGE_VECTOR_BACKEND`，未设 / 空 → `""`；可选 `CONTEXTFORGE_VECTOR_DIM` 解析为 `usize`，未设 / 非法 → `0`）；hybrid 热路径（`:340`）+ semantic 热路径（`:382`）的 `select_vector_backend("", 0)` 改为传入解析结果（`select_vector_backend(&backend_name, dim)`）。feature 未开时工厂返回的 honest Err 沿既有 `.map_err(|e| Status::internal(...))`（`:341` / `:383`）浮出，不静默回落到 BruteForce（ADR-013）。
- 改两热路径注释（`:337-339` / `:380-381`）——「No vector config is plumbed」描述随实现更新为「backend name from env CONTEXTFORGE_VECTOR_BACKEND；unset/"" → BruteForce byte-equiv」（兑现注释承诺）。
- `factory.rs` 不改逻辑——`select_vector_backend(name, dim)` 既有签名 / arm（`:38-67`）即本 task 调用契约；`dim` 沿用 `:37` `let _ = dim` 预留契约（本 task 范围内仅把 env 解析的 dim 透传，不在工厂内消费 [SPEC-DEFER:phase-future.vector-dim-auto-negotiation]）。
- 同源测试：server.rs 同源 / 集成测试断言两热路径 env 接线（设 env → 工厂收到该 name）+ default byte-equiv（未设 env → `""` → BruteForce，既有行为不变）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- factory sqlite-vec arm 补全（feature `vector-sqlite` → `SqliteVecBackend`）[SPEC-OWNER:task-32.2]——本 task 仅接线 env→两热路径，sqlite-vec arm 在 task-32.2 加。
- embedder-dim 自动协商（`dim` 据 provider 真实维度协商而非透传）[SPEC-DEFER:phase-future.vector-dim-auto-negotiation]——`dim` 沿用 task-29.1 占位契约。
- real per-user / per-collection vector config UI（console-api 表单选 backend）[SPEC-DEFER:phase-future.per-user-vector-config-ui]——本 task 为进程级 env 配置，不含 UI / per-request 配置。
- qdrant / lancedb / sqlite-vec 的真实 recall / latency 选择矩阵 cell（须 MSVC feature build）[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]——矩阵在 task-32.2，真实数值据真实跑出回填（ADR-013 不伪造）。
- 真实 release tag / run-id / digest（v0.25.0）[SPEC-OWNER:task-32.4-closeout]（ADR-012 用户授权后回填）。

## 4. Actors

- 主 agent（ADR-012 自治）
- `CoreService`（`core/src/server.rs:44-58`，持 `data_dir`，本 task 加 vector backend env 解析）
- `select_vector_backend`（`core/src/retriever/vector/factory.rs:31`，既有工厂，本 task 调用方）
- hybrid 检索热路径（`core/src/server.rs:334-341`）+ semantic 检索热路径（`core/src/server.rs:376-383`）
- `resolve_data_dir`（`core/src/server.rs:504-525`，env 解析 pattern 镜像源）
- 运维 / 部署者（据部署经 `CONTEXTFORGE_VECTOR_BACKEND` 选 backend）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/server.rs:334-341`（hybrid 热路径——`:340` `select_vector_backend("", 0)` + `:341` `.map_err` honest Err 浮出 + `:337-339` 「no vector config plumbed」注释）
- `core/src/server.rs:376-383`（semantic 热路径——`:382` `select_vector_backend("", 0)` + `:383` `.map_err` + `:380-381` 同注释）
- `core/src/server.rs:44-58`（CoreService doc `:44-46` + `data_dir: PathBuf` `:52` + `new(data_dir)` `:57`）+ `:504-525`（`resolve_data_dir` 四级 env 解析 pattern，`$CONTEXTFORGE_DATA_DIR` `:511`——env 解析镜像源）
- `core/src/retriever/vector/factory.rs:31-69`（`select_vector_backend(name, dim)` 签名 + arm：`""`/"brute" `:38-39`、qdrant `:40-51`、lancedb `:52-63`、unknown honest Err `:64-66`）+ `:37`（`let _ = dim` 预留契约 [SPEC-DEFER:phase-future.vector-dim-auto-negotiation]）+ `:76`（TEST-29.1.1 default byte-equiv 基线）
- `docs/decisions/adr-034-*.md`（production-vector-live-recall；factory 既有契约，本 task env 接线为 add-only，sqlite-vec arm 补全 Amendment 落点 @ task-32.4）+ `docs/decisions/adr-037-vector-backend-config-plumbing-and-completeness.md §D1`（本 task 即其原文实现）+ `docs/decisions/adr-023-*.md`（0-vector-dep baseline 守线）

### 5.2 关键设计 — env→两热路径 backend 选择（default 保形）

- **B1/B2 env→两热路径接线**：加 env 解析 helper（镜像 `resolve_data_dir` 四级 pattern 的 env 维度）读 `CONTEXTFORGE_VECTOR_BACKEND`（trim 后空 / 未设 → `""`）；hybrid（`:340`）+ semantic（`:382`）两热路径调用 `select_vector_backend(&name, dim)`。pass bar 测试：(a) 未设 env → 解析得 `""` → 工厂返回 `BruteForceVectorBackend`（`name()=="brute-force"`），与改前 `select_vector_backend("", 0)` byte-equivalent，既有 hybrid / semantic 行为不变；(b) 设 `CONTEXTFORGE_VECTOR_BACKEND=nope`（unknown）→ 工厂 honest Err（`factory.rs:64-66`）→ 热路径 `.map_err` 浮出 `Status::internal`（含 backend 名），不静默回落 BruteForce。env 读取经 `std::env::var`，0 新 dep。
- **B3 dim 透传 + 占位契约沿用**：可选 `CONTEXTFORGE_VECTOR_DIM` 解析为 `usize`（未设 / 解析失败 → `0`）透传给 `select_vector_backend(name, dim)`；工厂内 `dim` 仍为占位（`:37` `let _ = dim`，BruteForce arm 对任意 dim 工作）——本 task 不在工厂内消费 dim，dim 自动协商 [SPEC-DEFER:phase-future.vector-dim-auto-negotiation]。pass bar：设 dim env → 工厂收到该值（不 panic、default 路径行为不变）。
- **env 解析镜像既有 pattern**：解析 helper 镜像 `resolve_data_dir`（`:504-525`）的 trim / 空回落语义（`:506-509` arg trim、`:511-514` env trim 非空），保持代码风格一致；CoreService 不新增持久字段（env 于热路径就地解析，或经构造期解析存字段——按既有 `data_dir` 注入风格择一，默认就地解析最小改动）。
- **honest Err 不静默回落**：feature 未开（如 `CONTEXTFORGE_VECTOR_BACKEND=qdrant` 而无 `vector-qdrant` feature）→ 工厂返回 explicit Err（`factory.rs:45-50`）→ 热路径浮出 `Status::internal`，**绝不**静默回落 BruteForce（ADR-013：不伪造成功、不隐藏受阻）。

### 5.3 不变量

- 默认行为不变（ADR-004）：未设 `CONTEXTFORGE_VECTOR_BACKEND` 时两热路径解析得 `""` → `BruteForceVectorBackend`，与改前硬编码 `select_vector_backend("", 0)` byte-equivalent；既有 hybrid / semantic 检索结果、proto、`SearchResponse` shape 不变。
- 既有契约不变：`select_vector_backend(name, dim)` 工厂签名 / arm 不改（本 task 为调用方接线）；CoreService 公共构造（`new(data_dir)` `:57`）兼容（env 就地解析或 add-only 字段，调用方不破）；两进程拓扑不变（env 注入不改 Go thin proxy + Rust SoT 布局，ADR-016）。
- 0 新代码依赖（ADR-008 / ADR-023 守线）：仅 `std::env::var` 读取 + 既有 `select_vector_backend` 调用，无 Cargo 依赖增量；默认构建仍 0-vector-dep（feature backend 仍 feature-gated）。
- honest Err 守线（ADR-013）：feature 未开 / unknown backend → explicit Err 浮出，不静默回落 / 不伪造成功；真实 recall / latency 数值不在本 task 产出（属 task-32.2 矩阵，🟡 据真实跑出回填）。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [x] **AC1**（config plumbing 两热路径 🟢）: `server.rs` hybrid（`:340`）+ semantic（`:382`）两热路径经 env（`CONTEXTFORGE_VECTOR_BACKEND` + 可选 `CONTEXTFORGE_VECTOR_DIM`）解析后传入 `select_vector_backend(&name, dim)`（替代硬编码 `("", 0)`）；env 解析 helper 镜像 `resolve_data_dir`（`:504-525`）pattern；设 unknown backend → 工厂 honest Err 浮出 `Status::internal`，不静默回落 — verified by **TEST-32.1.1**
- [x] **AC2**（default 未设/"" → BruteForce byte-equiv 默认行为不变 🟢）: 未设 `CONTEXTFORGE_VECTOR_BACKEND` → 两热路径解析得 `""` → `BruteForceVectorBackend`（`name()=="brute-force"`），与改前 byte-equivalent；既有 hybrid / semantic 行为 + `factory.rs` TEST-29.1.1/.2/.3 不退化；默认行为 / proto / 既有契约不变（ADR-004）+ 0 新 dep（ADR-023 baseline）— verified by **TEST-32.1.2**
- [x] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-32.1.3**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-32.1.1 | env→两热路径接线：设 `CONTEXTFORGE_VECTOR_BACKEND` → hybrid（`:340`）+ semantic（`:382`）经工厂选该 backend；env 解析 helper 镜像 `resolve_data_dir`；unknown name → honest Err 浮出 `Status::internal`，不静默回落 | `core/src/server.rs`（同源 / 集成 test） | Done |
| TEST-32.1.2 | default byte-equiv：未设 env → 两热路径 `""` → `BruteForceVectorBackend`（`name()=="brute-force"`），既有 hybrid / semantic 行为 + TEST-29.1.x 不退化；0 新 dep | `core/src/server.rs` + `core/src/retriever/vector/factory.rs` | Done |
| TEST-32.1.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（中）env 就地解析 vs CoreService 字段注入的最小改动取舍**：env 于热路径就地解析（每请求读 `std::env::var`）最小改动但每请求一次系统调用；构造期解析存字段需改 `CoreService::new` 签名 / 测试。
  - **缓解**：默认就地解析（与 `select_provider("deterministic", 0)` `:378` 既有热路径就地构造风格一致，env 读开销可忽略）；若选字段注入须保 `new(data_dir)` `:57` 兼容（add-only 字段 / Default）。stop-condition：default byte-equiv 单测（AC2）不过则 AC1 不标 `[x]`。
- **R2（中）feature 未开时 honest Err 浮出 vs 静默回落的诚实边界**：设 `CONTEXTFORGE_VECTOR_BACKEND=qdrant` 而无 `vector-qdrant` feature，须 explicit Err 而非静默 BruteForce。
  - **缓解**：沿用工厂既有 honest Err（`factory.rs:45-50`）经 `.map_err`（`:341` / `:383`）浮出 `Status::internal`；单测断言 unknown backend → Err（不静默回落）。据 ADR-013 不伪造成功 / 不隐藏受阻。stop-condition：honest Err 不浮出（静默回落）则 AC1 不标 `[x]`。
- **R3（低）默认行为回归**：env 解析改动致未设 env 时 backend 名非 `""`（如误把 `None` 映射为 `"brute"` 之外），破 byte-equivalence。
  - **缓解**：解析 helper 未设 / 空一律回落 `""`（与改前硬编码 `""` 一致，非 `"brute"`——二者 byte-equivalent 但取 `""` 保 TEST-29.1.3 路径）；AC2 单测断言 `name()=="brute-force"` + 既有 hybrid / semantic 行为不变。
- **R4（低）dim 透传与预留契约一致性**：本 task 透传 dim 但工厂仍 `let _ = dim`（`:37`），易误读为 dim 已生效 [SPEC-DEFER:phase-future.vector-dim-auto-negotiation]。
  - **缓解**：注释 / spec 明记 dim 沿用 task-29.1 预留契约，dim 自动协商 [SPEC-DEFER:phase-future.vector-dim-auto-negotiation]；本 task 范围内透传不消费，default 路径（dim=0）行为不变。

## 9. Verification Plan

```bash
# 1. AC1 — env→两热路径接线（设 env → 工厂选该 backend；unknown → honest Err 浮出）
cargo test -p contextforge-core server::

# 2. AC2 — default byte-equiv（未设 env → "" → BruteForce）+ 既有工厂基线不退化
cargo test -p contextforge-core retriever::vector::factory

# 3. 不退化（全量）
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# 4. AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界**：本 task 仅交付 env→两热路径 backend 选择接线（🟢 可单测）+ default byte-equiv（🟢）；sqlite-vec factory arm 补全 [SPEC-OWNER:task-32.2]、embedder-dim 自动协商 [SPEC-DEFER:phase-future.vector-dim-auto-negotiation]、真实 recall / latency 选择矩阵 cell [SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix] 均不在本 task 范围；据 ADR-013 不预填真实数值（属 task-32.2，真实跑出后回填）。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done

**§9 Verification（PR #212 / squash commit `c7358ed`，v0.25.0 impl，真实证据）**：
- AC1：`cargo test -p contextforge-core`（server backend resolve/parse）PASS —— TEST-32.1.1 `parse_vector_backend`/`resolve_vector_backend` 读 `CONTEXTFORGE_VECTOR_BACKEND` env name + 可选 `CONTEXTFORGE_VECTOR_DIM`（parse/trim/blank→0），unknown name → 工厂 honest Err；server.rs hybrid（`:340`）+ semantic（`:382`）两热路径经 `select_vector_backend(&name, dim)` 接线（镜像 `resolve_data_dir` env pattern）；unknown/feature-off 经 `Status::internal` 诚实暴露（无静默回退，ADR-013）。
- AC2：`cargo test -p contextforge-core`（factory）PASS —— TEST-32.1.2 env 未设 → `("", 0)` → BruteForce 路径与 v0.24.0 字节等价（`name()=="brute-force"`，默认行为不变）；既有 hybrid / semantic 行为 + `factory.rs` TEST-29.1.1/.2/.3 byte-equiv 基线不退化；0 新 dep（默认构建仍 0-vector-dep，ADR-023）。
- AC3：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威；PR #212 触及行 0 未标注命中）。
- 全量不退化：`cargo test --workspace` 199 passed / 0 failed + `cargo clippy --workspace --all-targets -- -D warnings` exit 0 + `go test ./...` 全过；CI 四门（cargo-test / go-test / spec-lint / lint）PASS（v0.25.0-evidence §4）。0 新 dep / 默认行为不变 / 既有契约不变 / honest Err 不静默回落（ADR-004/008/013/023）。

**实际改动文件**：
- `core/src/server.rs`——加 `resolve_vector_backend`/`parse_vector_backend` env 解析 helper（镜像 `resolve_data_dir` `:504-525` pattern，读 `CONTEXTFORGE_VECTOR_BACKEND` + 可选 `CONTEXTFORGE_VECTOR_DIM`）；hybrid 热路径（`:340`）+ semantic 热路径（`:382`）`select_vector_backend("", 0)` 改为 env 解析结果传入；两热路径注释更新为 env 接线描述。+ 同源测试（TEST-32.1.1 env 接线 + TEST-32.1.2 default byte-equiv）。
- `core/src/retriever/vector/factory.rs`——不改逻辑（既有 `select_vector_backend(name, dim)` 签名 / arm 即调用契约；`dim` 沿用 `:37` 预留契约 [SPEC-DEFER:phase-future.vector-dim-auto-negotiation]）；sqlite-vec arm 补全在 task-32.2（#213，squash `76a3137`）[SPEC-OWNER:task-32.2]。
- ADR-034 sqlite-vec arm add-only Amendment 落点在 task-32.4 closeout（非本 task body）。

**honest-defer 维度（据真实受限如实保持延后，ADR-013，不伪造）**：sqlite-vec in-process recall/latency CELL `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`（属 task-32.2 矩阵，须本机 MSVC feature build + 真实语料）；embedder-dim 自动协商 `[SPEC-DEFER:phase-future.vector-dim-auto-negotiation]`（`dim` 沿用 task-29.1 占位契约，本 task 仅透传不消费）；real per-user/per-collection vector config UI `[SPEC-DEFER:phase-future.per-user-vector-config-ui]`。
