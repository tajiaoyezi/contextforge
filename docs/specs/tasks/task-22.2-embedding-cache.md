# Task `22.2`: `embedding-cache — core/src/embedding/cache.rs CachingEmbeddingProvider（content-hash Sha256(text)→embedding 缓存；内存缺省 + 可选 SQLite 持久化承 ADR-002）+ 确定性命中/失效单测`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 22 (embedding-provider-completion)
**Dependencies**: task-22.1（`select_provider` 工厂 — `CachingEmbeddingProvider` 包裹工厂选出的 provider）/ task-19.1（`EmbeddingProvider` trait）/ ADR-002（sqlite-tantivy-layered-storage — SQLite 持久化基线）/ ADR-027（embedding-provider-abstraction D3 缓存包装器）/ ADR-013（禁伪造凭据）/ ADR-014 D1-D5（第十三次激活）

## 1. Background

Phase 19 的 `EmbeddingProvider`（`core/src/embedding/traits.rs:13`）每次 `embed(&[String])` 都重新计算 —— 对 `DeterministicEmbeddingProvider`（`Sha256`→splitmix64）开销小，但对 real `FastEmbedProvider`（ONNX 推理）或远程 provider（HTTP 往返）开销大。重复内容（同一 chunk 多次 reindex、相同 query 多次检索）会重复 embed，浪费算力 / 网络。

`docs/roadmap.md` §3.3 把 `[SPEC-DEFER:phase-future.embedding-cache]`（phase-19-embedding spike）排入本 phase。`internal/eval`（`internal/eval/eval.go`）的召回评测对同一 question 集多趟跑（BM25 + semantic），缓存能稳定其 embedding 输入。

本 task 落 content-hash → embedding 缓存：以 `Sha256(text)` 为 key，命中跳过底层 embed，内容变更（hash 变化）即失效；内存缺省 + 可选 SQLite 持久化（承 ADR-002）。

## 2. Goal

新增 `core/src/embedding/cache.rs::CachingEmbeddingProvider`——包裹任意 `Arc<dyn EmbeddingProvider>`、本身实现 `EmbeddingProvider` trait（装饰器）。`embed(&[String])` 时：对每个 text 计算 `Sha256(text)` key，命中缓存则直接返回缓存向量（不调底层），未命中则调底层 `embed` 后写入缓存。`dim()` / `name()` 透传底层（`name()` 可加 `"cached:"` 前缀标识 provenance）。缓存后端：内存（`HashMap<String, Vec<f32>>`，`Mutex` 保护）缺省 + 可选 SQLite 持久化（独立表 / 独立文件，承 ADR-002 分层，复用 `rusqlite` bundled，feature / 配置 opt-in）。内容变更（hash 不同）即未命中 → 失效语义。≥3 Rust 测试全 PASS（用确定性 identity 实现做底层、计数底层调用次数断言命中跳过）；`cargo test --workspace` 不退化；D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **新增 `core/src/embedding/cache.rs`**（`core/src/embedding/mod.rs` `pub mod cache;` + re-export `CachingEmbeddingProvider`）：
  - `pub struct CachingEmbeddingProvider { inner: Arc<dyn EmbeddingProvider>, mem: Mutex<HashMap<String, Vec<f32>>>, store: Option<...> }`
  - `impl CachingEmbeddingProvider { pub fn new(inner: Arc<dyn EmbeddingProvider>) -> Self; pub fn with_sqlite(inner, path) -> Result<Self, EmbeddingError> }`
  - `impl EmbeddingProvider for CachingEmbeddingProvider`：`embed` 逐 text 算 `Sha256(text)` hex key（复用既有 `sha2::Sha256`，与 `deterministic.rs` 同 crate）→ 命中返缓存 / 未命中调 `inner.embed` 后 put；`dim()` → `inner.dim()`；`name()` → `"cached"` provenance 标识（透传底层 name 或前缀）
- **SQLite 持久化（可选）**：缓存表（如 `embedding_cache(content_hash TEXT PRIMARY KEY, dim INTEGER, vector BLOB, provider TEXT)`）独立文件 / 独立表（承 ADR-002，add-only schema，不动既有 `metadata.sqlite` / migrations）；向量以字节序列化（复用既有 `base64` 或裸 BLOB）；feature / 构造参数 opt-in，缺省内存不落盘。
- **修改 `core/src/embedding/tests.rs`（或 cache.rs 内 `#[cfg(test)]`）**：≥3 测试——（a）相同 text 第二次 `embed` 命中缓存、底层 `embed` 不再被调（用计数 wrapper provider 断言调用次数）；（b）不同 text（不同 hash）未命中、底层被调；（c）SQLite 持久化往返（写入后新建 `CachingEmbeddingProvider` 从同一文件读回命中，底层 0 调用）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **缓存淘汰策略（LRU / 容量上限）** [SPEC-DEFER:phase-future.cache-lru]：本 task 内存缓存无上限淘汰（小语料够用）；LRU / 容量配置留后续版本。
- **`cache-cap-configurable` 容量配置** [SPEC-DEFER:phase-future.cache-cap-configurable]：roadmap §4 长尾，本 task 不涉及。
- **provider 配置选择 + 工厂** [SPEC-OWNER:task-22.1-provider-config-selection]：工厂在 22.1，本 task 包裹工厂选出的 provider。
- **`RemoteEmbeddingProvider` HTTP 骨架** [SPEC-OWNER:task-22.3-remote-provider-skeleton]：远程 provider 在 22.3；本 task 用确定性 identity 实现做底层测试，不依赖远程。
- **health 探针 + smoke + closeout** [SPEC-OWNER:task-22.4-closeout-v0.15.0]：收口 task。
- **缓存命中率真实数值（real provider）** [SPEC-DEFER:phase-future.embedding-provider-remote]：本 task 用确定性 identity 实现断言命中 / 失效逻辑；real / 远程 provider 的真实命中率不在本 task 数值化（ADR-013）。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`core/src/embedding/cache.rs::CachingEmbeddingProvider`**：本 task 新增的缓存装饰器，实现 `EmbeddingProvider`。
- **底层 `Arc<dyn EmbeddingProvider>`**：被包裹的 provider（确定性 identity 实现 / fastembed / remote），本 task 不改其实现。
- **SQLite（rusqlite bundled）**：可选持久化后端，承 ADR-002 分层。
- **下游 task-22.4**：smoke v12 断言缓存命中可观测；closeout。

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/embedding/traits.rs:13-42`（`EmbeddingProvider` trait — `embed` / `dim` / `name`）
- `core/src/embedding/deterministic.rs:10-16`（`sha2::Sha256` 用法 + `DEFAULT_DIM`，作底层测试 provider）
- `core/src/embedding/mod.rs`（re-export pattern）
- `core/Cargo.toml:59`（`sha2 = "0.11.0"` 已是 direct dep）+ `core/Cargo.toml:70`（`rusqlite bundled`）+ `core/Cargo.toml:74`（`base64`）
- `docs/decisions/adr-002-sqlite-tantivy-layered-storage.md`（SQLite 分层基线）+ `docs/decisions/adr-027-embedding-provider-abstraction.md`（D3 缓存包装器）

### 5.2 关键设计 — content-hash key + 装饰器 + 失效语义

- key = `Sha256(text)` hex（与 `deterministic.rs` 同 hash，避免新 dep）；同文本 → 同 key → 命中；文本变更 → 不同 key → 未命中（= 失效，无需显式 invalidate）。
- 装饰器：`CachingEmbeddingProvider` 实现 `EmbeddingProvider`，可被 `Retriever::with_embedder` / 工厂透明接入（trait object 兼容）。
- 批量 `embed(&[String])`：逐 text 查缓存，仅对未命中的 text 调底层 `embed`，结果按输入顺序组装（保持 trait 契约「一向量 per 输入」）。
- SQLite 持久化 opt-in：缺省内存（进程内）；构造时显式给路径才落盘（承 ADR-002，add-only 独立表）。

### 5.3 不变量

- `EmbeddingProvider` trait 不变（add-only，本 task 不改 trait）；缓存对 caller 透明。
- 缓存命中返回的向量与底层直接 embed 逐字节相同（确定性 identity 实现下可严格断言）。
- 默认构建 0 新 dep（`sha2` / `rusqlite` / `base64` 均已是 direct dep）。
- SQLite 缓存 add-only schema，不动既有 `metadata.sqlite` / migrations（ADR-002 分层）。

## 6. Acceptance Criteria

- [ ] **AC1**: `CachingEmbeddingProvider` 包裹 `DeterministicEmbeddingProvider`：相同 text 第二次 `embed` 命中缓存、底层 `embed` 不再被调（计数 wrapper 断言底层调用次数）；返回向量与底层直接 embed 逐字节相同 — verified by **TEST-22.2.1**
- [ ] **AC2**: 失效语义 — 不同 text（不同 `Sha256` hash）未命中、底层被调；批量 `embed` 混合命中 / 未命中时仅对未命中 text 调底层，结果按输入顺序正确组装 — verified by **TEST-22.2.2**
- [ ] **AC3**: SQLite 持久化往返（可选）— 给定路径写入缓存后，新建 `CachingEmbeddingProvider::with_sqlite` 从同一文件读回命中（底层 0 调用）；缺省（无路径）内存缓存不落盘 — verified by **TEST-22.2.3**
- [ ] **AC4**: `dim()` / `name()` 透传底层（`dim()==inner.dim()`，`name()` 携 `"cached"` provenance 标识）；缓存装饰器可作 `Arc<dyn EmbeddingProvider>` 接入 `Retriever::with_embedder` — verified by **TEST-22.2.4**
- [ ] **AC5**: 既有不退化 — `cargo test --workspace` 全 PASS（含既有 embedding / retriever 测试）；`go test ./...` 不受影响（本 PR 零 Go delta）；D2 lint `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-22.2.5** + §10 记录

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-22.2.1 | 相同 text 命中缓存 + 底层不再被调 + 向量逐字节相同 | `core/src/embedding/cache.rs` `#[cfg(test)]` 或 `core/src/embedding/tests.rs` | Planned |
| TEST-22.2.2 | 失效语义：不同 text 未命中 + 批量混合命中顺序正确 | `core/src/embedding/cache.rs` `#[cfg(test)]` | Planned |
| TEST-22.2.3 | SQLite 持久化往返命中 + 缺省内存不落盘 | `core/src/embedding/cache.rs` `#[cfg(test)]` | Planned |
| TEST-22.2.4 | `dim()`/`name()` 透传 + 可作 `Arc<dyn EmbeddingProvider>` 接入 | `core/src/embedding/cache.rs` `#[cfg(test)]` | Planned |
| TEST-22.2.5 | `cargo test --workspace` 0 failed + D2 lint 0 未标注命中 | 全 Rust + `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（中）底层调用计数测试的 wrapper provider**：断言「命中跳过底层」需观察底层 `embed` 调用次数。
  - **缓解**：测试内定义计数 wrapper（实现 `EmbeddingProvider`，`embed` 内 `AtomicUsize` 自增后委托确定性 identity 实现），断言计数；纯 std 不引外部测试替身框架。
- **R2（低）SQLite 向量序列化精度**：`f32` 向量序列化往返需精确还原。
  - **缓解**：以原始字节（`f32::to_le_bytes`）存 BLOB，读回 `from_le_bytes`，逐字节往返；测试断言往返向量逐字节相等。
- **R3（低）内存缓存无上限**：长跑进程缓存可能增长。
  - **缓解**：本 task 小语料够用，无淘汰；LRU / 容量上限 `[SPEC-DEFER:phase-future.cache-lru]` 后续版本接入（§3 范围外）。

## 9. Verification Plan

```bash
# Rust：缓存命中/失效 + SQLite 往返 + 透传 + 既有不退化
cargo test -p contextforge-core embedding::cache -- --nocapture
cargo test -p contextforge-core embedding -- --nocapture
cargo test --workspace

# Go 不退化（本 PR 零 Go delta，CI go-test gate 复核）
go test ./...

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **Status**: 待实施（Draft）。实施完成后按以下 6 项回填：完成日期 / 改动文件 / commit 列表 / §9 Verification 结果 / 设计取舍 / 剩余风险 + 下游影响。
