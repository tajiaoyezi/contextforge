# Task `22.1`: `provider-config-selection — internal/config 加 add-only [embedding] 配置（provider/dim）+ core/src/embedding 工厂按配置选择 provider（deterministic/fastembed/remote）+ dim 协商校验 + core/src/server.rs 语义路径改用工厂`

**Status**: Draft

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 22 (embedding-provider-completion)
**Dependencies**: task-19.1（`EmbeddingProvider` trait + `DeterministicEmbeddingProvider` + `FastEmbedProvider` feature-gated）/ task-19.2（`Retriever::with_embedder` + `search_semantic`）/ task-19.3（`core/src/server.rs` 语义路径硬编码 `DeterministicEmbeddingProvider::default()`）/ ADR-027（embedding-provider-abstraction，本 phase 新 Proposed）/ ADR-004（local-first）/ ADR-008（core-library + embedding crate Amendment）/ ADR-013（禁伪造凭据）/ ADR-014 D1-D5（第十三次激活）

## 1. Background

Phase 19（v0.12.0）落地 `EmbeddingProvider` trait（`core/src/embedding/traits.rs:13`）与两个实现：`DeterministicEmbeddingProvider`（`core/src/embedding/deterministic.rs`，`DEFAULT_DIM=384`，默认构建可用）与 `FastEmbedProvider`（`core/src/embedding/fastembed_provider.rs`，`embedding-fastembed` feature，`all-MiniLM-L6-v2` dim 384）。

但 provider **选择是硬编码的**：`core/src/server.rs:299-324` 的语义路径恒构造 `DeterministicEmbeddingProvider::default()`（line 301）。无运行时配置选择、无维度协商校验。`internal/config/config.go` 的 `Config` 有 `[remote]` 段（`RemoteProviderConfig{Enabled, Provider, Endpoint}`），但**没有** `[embedding]` 段，无法表达「用哪个 embedding provider / 哪个维度」。

本 task 闭合 `[SPEC-OWNER:phase-future.embedding-provider-full]` 的「provider 经配置选择 + dim 协商」最小子集：加 add-only `[embedding]` 配置，在 Rust 侧加工厂按配置选择 provider 并校验维度协商，语义路径改用工厂（缺省仍确定性 identity 实现，行为不变）。

## 2. Goal

`internal/config.Config` 加 `Embedding EmbeddingConfig`（add-only），`EmbeddingConfig{Provider string, Dim int}`，TOML codec 加 `[embedding]` 段（仿既有 `[remote]` 段的 encode / decode / assign 三处），缺省 `Provider=""`（→ 视作 `"deterministic"`）/ `Dim=0`（→ 视作 `DEFAULT_DIM` 384）。Rust 侧新增工厂 `select_provider(provider_name: &str, dim: usize) -> Result<Arc<dyn EmbeddingProvider>, EmbeddingError>`：`"deterministic"` / `""` → `DeterministicEmbeddingProvider`；`"fastembed"` → feature 下 `FastEmbedProvider`，未编入 feature 时返回明确 `EmbeddingError`（feature 未启用）；`"remote"` → task-22.3 骨架 seam（本 task 留工厂分支 + 明确错误，骨架实现在 task-22.3）。dim 协商校验：工厂选出 provider 后，若其 `dim()` 与请求 `dim`（非 0 时）不一致 → 返回 `EmbeddingError::DimMismatch{expected, got}`（既有变体），不静默截断 / pad。`core/src/server.rs:299-324` 语义路径改为经工厂按配置选择（缺省仍确定性 identity 实现，逐字节行为不变）。≥3 Go + Rust 测试全 PASS；`go test ./...` + `cargo test --workspace` 不退化；D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **修改 `internal/config/config.go`**：`Config` 加 `Embedding EmbeddingConfig` 字段（add-only，置于既有字段后）；新增 `EmbeddingConfig struct { Provider string; Dim int }`；TOML codec 加 `[embedding]` 段——`encodeTOML` 加 `[embedding]` 段（仿 `[remote]` 段写 `provider` / `dim`）、`decodeTOML` 加 `"embedding"` section 分派、新增 `assignEmbedding(e *EmbeddingConfig, key, raw string)`（`provider` 走 `parseTOMLString`，`dim` 走 `strconv.Atoi`）。
- **新增 `core/src/embedding/factory.rs`**（并在 `core/src/embedding/mod.rs` `pub mod factory;` + re-export `select_provider`）：`pub fn select_provider(provider_name: &str, dim: usize) -> Result<Arc<dyn EmbeddingProvider>, EmbeddingError>`——`"deterministic"` / `""` → `Arc::new(DeterministicEmbeddingProvider::new(if dim==0 {DEFAULT_DIM} else {dim}))`；`"fastembed"` → `#[cfg(feature="embedding-fastembed")]` 返回 `FastEmbedProvider`、否则 `Err(EmbeddingError::Other("provider 'fastembed' requires the embedding-fastembed feature".into()))`；`"remote"` → 返回明确 `EmbeddingError`（remote 骨架 [SPEC-OWNER:task-22.3-remote-provider-skeleton] 落地，本 task 留工厂分支 + 明确错误）；未知 name → `EmbeddingError::Other`。dim 协商：选出 provider 后 `if dim != 0 && provider.dim() != dim { return Err(EmbeddingError::DimMismatch{expected: dim, got: provider.dim()}); }`。
- **修改 `core/src/server.rs:299-324`**：语义路径 `let embedder = Arc::new(DeterministicEmbeddingProvider::default());`（line 301）改为经 `select_provider(<配置 provider>, <配置 dim>)`；无配置可读时缺省 `("deterministic", 0)` → 行为与现状逐字节等价。
- **修改 `core/src/embedding/tests.rs`（或同源 `#[cfg(test)]`）+ `internal/config/config_test.go`**：Go 测试断言 `[embedding]` round-trip（含 / 不含段均合法，缺省 `Provider=""` / `Dim=0`）；Rust 测试断言工厂 `"deterministic"` / `""` 返确定性 provider、`"remote"` / 未知 name 返明确 `Err`、dim 协商 mismatch 返 `DimMismatch`、`dim=0` 不触发 mismatch（用 `DEFAULT_DIM`）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **`RemoteEmbeddingProvider` HTTP 骨架实现 + 契约测试** [SPEC-OWNER:task-22.3-remote-provider-skeleton]：本 task 仅在工厂留 `"remote"` 分支 + 明确错误，骨架实现在 task-22.3。
- **`CachingEmbeddingProvider` content-hash 缓存** [SPEC-OWNER:task-22.2-embedding-cache]：缓存包装在 task-22.2，本 task 不实现缓存。
- **health 远程探针 + smoke v12 + v0.15.0 release docs** [SPEC-OWNER:task-22.4-closeout-v0.15.0]：本 task 落配置 + 工厂；探针 / smoke / closeout 在收口 task。
- **远程 provider 真实网络联调 / 密钥 / 真实召回质量** [SPEC-DEFER:phase-future.embedding-provider-remote]：本 task 是配置 + 工厂 wiring，不打真实网络、不产出召回数值（ADR-013）。
- **缓存淘汰策略（LRU / 容量上限）** [SPEC-DEFER:phase-future.cache-lru]：roadmap §4 长尾，本 task 不涉及。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`internal/config.Config` / `EmbeddingConfig`**：本地配置根，本 task add-only 加 `[embedding]` 段。
- **`core/src/embedding/factory.rs::select_provider`**：本 task 新增的 provider 选择 seam。
- **`core/src/embedding/{deterministic,fastembed_provider}.rs`**：工厂选出的具体 provider（Phase 19 已落地，本 task 不改其实现）。
- **`core/src/server.rs` 语义路径**：本 task 把硬编码 provider 改为经工厂选择。
- **下游 task-22.2 / 22.3 / 22.4**：22.2 包裹工厂选出的 provider 做缓存；22.3 落 `"remote"` 分支骨架；22.4 探针 + closeout。

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/embedding/traits.rs:13-42`（`EmbeddingProvider` trait + `EmbeddingError` 含 `DimMismatch{expected, got}`）
- `core/src/embedding/deterministic.rs`（`DeterministicEmbeddingProvider::new(dim)` + `DEFAULT_DIM=384`）
- `core/src/embedding/fastembed_provider.rs`（`FastEmbedProvider` + `FASTEMBED_DIM=384`，`embedding-fastembed` feature）
- `core/src/embedding/mod.rs`（既有 re-export + feature-gated `pub mod fastembed_provider`）
- `core/src/server.rs:293-324`（语义路径硬编码 `DeterministicEmbeddingProvider::default()` 工厂点）
- `core/src/retriever/vector/types.rs:53-60`（`VectorIndexConfig.dim` — dim 协商对象）
- `internal/config/config.go:48-54`（`RemoteProviderConfig` + `[remote]` 段 codec 范例：`assignRemote` / `encodeTOML` / `decodeTOML`）
- `docs/decisions/adr-004-local-first-privacy-baseline.md`（本地优先）+ `docs/decisions/adr-027-embedding-provider-abstraction.md`（D1 配置选择 + D2 dim 协商）

### 5.2 关键设计 — 工厂选择 + dim 协商 + 缺省不退化

- `select_provider("", 0)` ≡ `select_provider("deterministic", 0)` → `DeterministicEmbeddingProvider::new(DEFAULT_DIM)`：与 Phase 19 `DeterministicEmbeddingProvider::default()` 逐字节等价，缺省语义路径行为不变。
- dim 协商：`dim=0` 表示「不指定，用 provider 缺省」→ 不触发 mismatch；`dim != 0` 且与 provider `dim()` 不一致 → `EmbeddingError::DimMismatch{expected: dim, got: provider.dim()}`（不静默截断 / pad，避免破坏既有 384 向量索引）。
- `"fastembed"` 在未编入 `embedding-fastembed` feature 时返回明确 `EmbeddingError`（而非 panic / silent fallback），让 caller 知情。
- `"remote"` 分支本 task 返回明确 `EmbeddingError`（骨架由 task-22.3 落地）；本 task 不引入网络 dep（ADR-004 + ADR-008 D5）。

### 5.3 不变量

- `EmbeddingProvider` trait 不变（add-only，本 task 不改 trait 签名）；既有 `with_embedder` / `index_chunks_semantic` / `search_semantic` 消费方零改动。
- 默认构建 0 新 dep（工厂纯 Rust；fastembed / remote 仍 feature-gated）。
- 缺省配置（无 `[embedding]` 段 / `Provider=""`）→ 确定性 identity 实现 + dim 384，语义路径行为逐字节不变（ADR-027 D1 向后兼容）。
- 远程 provider 不在本 task 激活（ADR-004 本地优先红线）。

## 6. Acceptance Criteria

- [ ] **AC1**: `internal/config.Config` 加 add-only `Embedding EmbeddingConfig{Provider, Dim}`，TOML `[embedding]` 段 round-trip 正确（含 / 不含段均合法，缺省 `Provider=""` / `Dim=0`，既有 `[remote]` / `[[collections]]` 段不受影响）— verified by **TEST-22.1.1**
- [ ] **AC2**: `select_provider("deterministic", 0)` 与 `select_provider("", 0)` 均返回 `DeterministicEmbeddingProvider`（`name()=="deterministic-sha256"`，`dim()==384`）；与 Phase 19 `DeterministicEmbeddingProvider::default()` 行为等价 — verified by **TEST-22.1.2**
- [ ] **AC3**: dim 协商 — `select_provider("deterministic", 128)` 返回 dim=128 的 provider（无 mismatch）；`select_provider("deterministic", 0)` 用 `DEFAULT_DIM`（不触发 mismatch）；构造 provider `dim()` 与请求 dim 不一致的场景返回 `EmbeddingError::DimMismatch{expected, got}`（不静默）— verified by **TEST-22.1.3**
- [ ] **AC4**: `select_provider("remote", ...)` 与未知 provider name 返回明确 `EmbeddingError`（不 panic、不 silent fallback）；`"fastembed"` 在未编入 feature 时返回明确 feature-未启用 `EmbeddingError` — verified by **TEST-22.1.4**
- [ ] **AC5**: `core/src/server.rs` 语义路径改用 `select_provider` 后，缺省 `("deterministic", 0)` 下既有语义检索（`?semantic=true`）行为不退化 — verified by **TEST-22.1.5** + §10 实测
- [ ] **AC6**: 既有不退化 — `go test ./...` + `cargo test --workspace` 全 PASS；D2 lint `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-22.1.6** + §10 记录

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-22.1.1 | `[embedding]` 配置 TOML round-trip（含/缺省 + 既有段不受影响） | `internal/config/config_test.go` | Planned |
| TEST-22.1.2 | `select_provider("deterministic"/"" )` 返确定性 provider（name/dim） | `core/src/embedding/tests.rs` | Planned |
| TEST-22.1.3 | dim 协商：指定 dim 生效 / dim=0 用默认 / mismatch 返 `DimMismatch` | `core/src/embedding/tests.rs` | Planned |
| TEST-22.1.4 | `"remote"` / 未知 name / fastembed-未启用 返明确 `EmbeddingError` | `core/src/embedding/tests.rs` | Planned |
| TEST-22.1.5 | `server.rs` 语义路径改用工厂后缺省行为不退化 | `core/src/server.rs` `#[cfg(test)]` 或 `core/tests/` | Planned |
| TEST-22.1.6 | `go test ./...` + `cargo test --workspace` 0 failed + D2 lint 0 未标注命中 | 全 Go + 全 Rust + `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（中）dim 协商边界（dim=0 语义）**：`dim=0` 需明确表示「用 provider 缺省」而非「dim 必须为 0」。
  - **缓解**：工厂中 `dim==0` 跳过 mismatch 校验、确定性 provider 用 `DEFAULT_DIM` 构造；测试覆盖 dim=0 / 指定 dim / mismatch 三路。
- **R2（低）`server.rs` 改动触及语义热路径**：工厂替换硬编码 provider 不应改变缺省行为。
  - **缓解**：缺省 `("deterministic", 0)` 与 `DeterministicEmbeddingProvider::default()` 等价；TEST-22.1.5 断言缺省语义检索不退化；既有 task-19.3 语义测试复跑守护。
- **R3（低）Go TOML codec `dim` 整数解析**：既有 codec 只处理 string / bool / string-array，新增 int 字段需 `strconv.Atoi`。
  - **缓解**：`assignEmbedding` 中 `dim` 走 `strconv.Atoi`（解析失败返 error，与既有 `allow_denylist_override` 的 `ParseBool` 同模式）；`encodeTOML` 用 `strconv.Itoa`；round-trip 测试覆盖。

## 9. Verification Plan

```bash
# Go：[embedding] 配置 round-trip + 既有不退化
go vet ./internal/...
go test ./internal/config/... -run 'TestTask221' -v
go test ./...

# Rust：工厂选择 + dim 协商 + server 语义路径不退化
cargo test -p contextforge-core embedding::tests -- --nocapture
cargo test --workspace
# feature 下 fastembed 分支（本地可选）
cargo test --features embedding-fastembed -p contextforge-core embedding::tests

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **Status**: 待实施（Draft）。实施完成后按以下 6 项回填：完成日期 / 改动文件 / commit 列表 / §9 Verification 结果 / 设计取舍 / 剩余风险 + 下游影响。
