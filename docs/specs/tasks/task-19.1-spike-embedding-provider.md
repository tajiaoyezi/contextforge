# Task `19.1`: `spike-embedding-provider — core/src/embedding/{mod,traits}.rs EmbeddingProvider trait + DeterministicEmbeddingProvider（无模型缺省）+ real provider（feature-gated）+ 候选评估 evidence`

**Status**: Done

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 19 (vector-retrieval-integration)
**Dependencies**: Phase 18 infra（task-18.1 `Vector{Backend,Indexer,Searcher}` 三 trait 冻结 + `VectorChunk.embedding: Vec<f32>` 契约 / task-18.8 `SemanticRecall@K` 度量 / ADR-023 Proposed）/ ADR-008 core-library-selection（embedding crate add-only amendment 依据）/ ADR-014 D1-D5 第十一次激活

## 1. Background

Phase 18 交付了向量 backend 基础设施（trait + 4 backend spike + harness + `SemanticRecall@K` 度量 + ADR-023 Proposed），但所有召回评测都喂的是合成种子向量 —— `docs/spikes/phase-18-comparison.md` 明确记录：4 个 backend 在 n=100k 仍 recall@5/10 = 1.0，**合成向量不可区分 ANN 与 exact**，真正有区分度的召回排名必须来自真实分布 embedding。要产出真实 embedding 就需要一个 embedding provider，而 Phase 18 把它整体留作 [SPEC-DEFER:phase-future.embedding-provider-full]。

本 task = Phase 19 首项，解锁 task-19.2（backend wiring 需要 embedding 把文本变向量）。核心张力是 **平台门槛**：fastembed-rs / candle / ort(ONNX) 都依赖 native 运行时 + 模型获取，phase-18 已实证 sqlite-vec 在 Windows MSVC 受阻（[SPEC-DEFER:phase-future.sqlite-vec-cross-platform]），real embedding provider 在 Windows MSVC / CI 同样存在 native + 模型下载受阻风险（phase-19 §7 R1 高风险）。

因此本 task 采取**双轨**：先 spike 评估 3 路候选在 Linux + Windows MSVC 的构建/运行/模型获取，选定一路落 feature-gated real provider；同时落一个**无模型依赖的 deterministic 缺省/兜底 provider**（hash/seed 派生固定维度向量），它进默认构建（0 新 dep），供 CI / smoke / test / wiring 跑通。ADR-013 红线：若 real provider 两平台均不可构建，deterministic 缺省 provider 继续跑通 wiring/smoke（诚实标注），real-model 真实召回 + ADR ratify 据实测延后 [SPEC-OWNER:phase-future.embedding-provider-full]，禁据合成数据预先 claim Done/Accepted。

## 2. Goal

在 `core/src/embedding/` 落地 embedding 抽象层：`EmbeddingProvider` trait（`core/src/embedding/{mod,traits}.rs`）+ `DeterministicEmbeddingProvider`（`core/src/embedding/deterministic.rs`，hash/seed 派生，无模型 dep，默认构建启用，供 CI/smoke/test）+ 选定的 real provider（`core/src/embedding/fastembed_provider.rs`，`embedding-fastembed` feature-gated，模型 lazy load）。spike 评估 3 路候选（fastembed-rs / candle / ort）在 Linux + Windows MSVC 的构建/运行/模型获取并选定一路，evidence 落 `docs/spikes/phase-19-embedding-{candidates,fastembed}.md`。同源 `mod tests` ≥3 unit test（deterministic 确定性 + dim 一致 + trait 契约）。默认 `cargo test --workspace` + `go test ./...` 不退化；D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **新建 `core/src/embedding/mod.rs`** — 模块根：`pub mod traits; pub mod deterministic;` + cfg-gated `#[cfg(feature = "embedding-fastembed")] pub mod fastembed_provider;` + re-export（`pub use traits::EmbeddingProvider; pub use deterministic::DeterministicEmbeddingProvider;` + cfg-gated real provider）+ `#[cfg(test)] mod tests;`。
- **新建 `core/src/embedding/traits.rs`** — `EmbeddingProvider` trait（`Send + Sync + Debug`，与 task-18.1 三 trait 同风格）：
  - `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError>` — 批量文本 → 每文本一向量，长度恒 = `dim()`
  - `fn dim(&self) -> usize` — 固定输出维度
  - `fn name(&self) -> &'static str` — provider 标识（provenance，对接 phase-19 `embedding_provider` 字段）
  - `EmbeddingError` enum（`thiserror`，`#[non_exhaustive]` 承 task-18.1 `VectorError` add-only pattern）：`ModelLoad(String)` / `DimMismatch { expected, got }` / `EmptyInput` / `Backend { #[source] ... }` / `Other(String)`。
- **新建 `core/src/embedding/deterministic.rs`** — `DeterministicEmbeddingProvider`：
  - `new(dim)` 构造；hash(text)（sha2，既有 dep）→ seed → 逐分量派生 f32 → 单位归一化（与 task-18.x backend normalize 对齐，cosine 距离 well-defined）
  - 相同 text 恒产相同向量（确定性，可复现 recall）；不同 text 高概率不同向量
  - 无模型 / 无网络 / 无新 dep；默认构建启用；`Debug`/`Default`（默认 dim 取 real provider 同维，便于 wiring 切换）
- **新建 `core/src/embedding/fastembed_provider.rs`** — 选定的 real provider（fastembed/candle/ort 之一，依 §spike 结果定 `fastembed`）：
  - 实现 `EmbeddingProvider`；模型 lazy load（首次 `embed` 触发 init，避免构造即下载）
  - `embedding-fastembed` feature gate；默认构建不引入该 dep（cfg-gated）
- **新建 `core/src/embedding/tests.rs`**（同源 `mod tests`）— ≥3 unit test：
  - deterministic 确定性（同 text 两次 embed 逐分量相等）
  - dim 一致（输出每向量 `.len() == dim()`，多文本批量恒定）
  - trait 契约（`Arc<dyn EmbeddingProvider>` 对象安全 + `name()`/`dim()` 稳定 + 空输入 / 单位范数 invariant）
- **修改 `core/src/lib.rs`** — 加 `pub mod embedding;`（承 §lib.rs 既有模块声明风格）。
- **修改 `core/Cargo.toml`** — `fastembed = { version = "...", optional = true }` real-provider dep + `embedding-fastembed = ["dep:fastembed", ...]` feature（默认不启用，承 vector-* feature pattern）。
- **新建 `docs/spikes/phase-19-embedding-candidates.md`** — 3 路候选评估矩阵（fastembed-rs / candle / ort）：Linux + Windows MSVC 构建结果、模型获取方式（下载/打包/缓存路径）、API 形状、native 运行时门槛、选定理由。
- **新建 `docs/spikes/phase-19-embedding-fastembed.md`** — 选定 provider 的真实 embed 样例 evidence（输入文本 → 输出向量维度/范数/前若干分量、模型来源、首次 lazy load 行为、平台构建实证）。
- **修改 `docs/s2v-adapter.md`** — §Phase 索引 Phase 19 Tasks 计数 +1（19.1 行登记）。

### Out of Scope（[SPEC-DEFER] / [SPEC-OWNER]）

- **default backend wiring（embedding 接生产 retriever 热路径）** [SPEC-OWNER:task-19.2-default-backend-wiring]：本 task 只落 provider 抽象 + 实现，index/query 路径接入是 19.2。
- **proto `embedding_provider` / `vector_score` 字段 + semantic API** [SPEC-OWNER:task-19.3-semantic-search-api]：provenance 字段落 proto 在 19.3。
- **真实 dogfood embedding `SemanticRecall@K` 实测** [SPEC-OWNER:task-19.5-real-recall-eval]：real provider 喂 dogfood 语料跑真实召回是 19.5。
- **ADR-023 Proposed→Accepted ratify** [SPEC-OWNER:task-19.6-adr-023-ratify]：据 19.5 真实数据 ratify，本 task 不动 ADR-023 Status。
- **Remote embedding provider（OpenAI / Cohere）** [SPEC-DEFER:phase-future.embedding-provider-remote]：本 phase 仅本地 provider，承 phase-19 §不在 scope。
- **real provider 两平台均不可构建时的 real-model 召回闭环** [SPEC-OWNER:phase-future.embedding-provider-full]：stop-condition 触发则 deterministic 缺省 provider 续跑 wiring/smoke，real recall + ADR ratify 据实测延后（ADR-013 禁据合成预先 claim）。
- **embedding 结果缓存 / 增量重嵌** [SPEC-DEFER:phase-future.embedding-cache]：spike 每次 `embed` 直算，缓存层后置优化。
- **CJK + 代码符号 tokenizer 对 embedding 输入的特化** [SPEC-DEFER:phase-future.cjk-and-code-tokenizer]：承 phase-19 §不在 scope。

## 4. Actors

- **主 agent**：spike 评估 + 实施 + PR 主理（ADR-012 自治）。
- **`EmbeddingProvider` trait**：core embedding 模块新 seam，下游 retriever / eval 经此抽象消费 embedding。
- **`DeterministicEmbeddingProvider`**：默认构建成员，CI / smoke / test / wiring 的无模型缺省 provider。
- **real provider（`fastembed`）**：cfg-gated 成员，dev / Linux 跑真实 embedding（喂 task-19.5 真实召回）。
- **下游 task-19.2**：消费 `EmbeddingProvider` 把 index/query 文本嵌入后接 vector searcher。
- **下游 task-19.5**：用 real provider 对 dogfood 语料生成真实 embedding 跑 `SemanticRecall@K`。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-19-vector-retrieval-integration.md`（§3 涉及模块 19.1 / §4 任务清单 / §5 依赖关系 / §6 AC1 / §7 R1）
- `docs/specs/tasks/task-18.1-vector-trait.md`（trait 风格 + `VectorChunk.embedding: Vec<f32>` 契约——embedding 输出喂此字段）+ `docs/specs/tasks/task-18.8-eval-semantic-recall.md`（`SemanticRecall@K` 度量——本 provider 喂其真实输入）
- `core/src/retriever/vector/{traits,types}.rs`（trait/error 同风格对照——`#[non_exhaustive]` error、`Send + Sync + Debug` bound、newtype guard）+ `core/src/retriever/vector/hnsw.rs`（`normalize` 单位归一化参考，deterministic provider 复用同语义）
- 兄弟 task：`../tasks/task-19.2-default-backend-wiring.md`（下游 wiring 如何消费 provider）/ `../tasks/task-19.5-real-recall-eval.md`（real provider 的真实召回去向）
- `docs/decisions/adr-008-core-library-selection.md`（embedding crate 入库 add-only amendment 依据）+ `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5）+ `docs/decisions/adr-013-*`（禁伪造证据红线，stop-condition 诚实缩范围依据）

### 5.2 Imports（real provider 新增 optional dep；默认构建 0 新 dep）

```rust
// traits.rs / deterministic.rs — 仅既有 dep（sha2 / thiserror / std），默认构建 0 新 dep
use std::fmt::Debug;
use sha2::{Digest, Sha256};
use thiserror::Error;
use crate::embedding::traits::{EmbeddingProvider, EmbeddingError};

// fastembed.rs — feature-gated real provider（示意，fastembed 依 spike 选定）
// use fastembed::{...};   // 经 embedding-fastembed feature 引入，lazy model init
```

### 5.3 关键设计

- **trait 与 task-18.1 三 trait 同构**：`EmbeddingProvider: Send + Sync + Debug`；`embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError>`；`dim()` / `name()` 与 `VectorBackend::name()` 同风格（`&'static str`）。对象安全（`Arc<dyn EmbeddingProvider>`）让 retriever 持有 trait object 在 deterministic / real 间切换。
- **deterministic 派生**：`embed` 对每 text 取 `Sha256(text)` 摘要作种子，按 `dim` 逐分量展开为 f32（摘要字节 → 稳定伪随机分量），末尾单位归一化（复用 `retriever/vector/hnsw.rs` 同 normalize 语义 → cosine 距离 well-defined，与 backend 真值一致）。相同 text 恒产相同向量（可复现 recall，进 CI/smoke 不引非确定性）。
- **维度恒定**：`embed` 输出 `Vec<Vec<f32>>` 外层长 = `texts.len()`，每内层长恒 = `dim()`；空输入返 `Ok(vec![])` 或 `EmbeddingError::EmptyInput`（依 §spike 与 wiring 约定择一，evidence 记定）。
- **real provider lazy load**：构造不触发模型下载/加载；首次 `embed` 经 `OnceCell`/`Once` 守护 init（避免默认路径意外拉模型）；模型本地缓存（本地优先，PRD §Anti-metrics）。`name()` 返模型/runtime 标识入 provenance。
- **feature gate**：`embedding-fastembed` 默认关闭；默认 `cargo build` 不编译 `fastembed.rs`、不引入 `fastembed` dep（cfg-gated，承 vector-sqlite/vector-hnsw pattern）。
- **stop-condition（ADR-013）**：spike 实证 real provider 在 Linux + Windows MSVC 的构建结果记 candidates evidence。若两平台均不可构建 → deterministic 缺省 provider 落地 + 跑通 §9，real provider 实现降级为「最接近可构建一路的实测受阻凭据」记 evidence，真实召回 + ADR ratify 据实测延后 [SPEC-OWNER:phase-future.embedding-provider-full]，AC5/AC6 据实诚实标注（不预先 claim）。

## 6. Acceptance Criteria

- [x] **AC1**: `EmbeddingProvider` trait（`embed` + `dim` + `name`）+ `EmbeddingError`（`#[non_exhaustive]`）落 `core/src/embedding/{mod,traits}.rs`；`Arc<dyn EmbeddingProvider>` 对象安全；默认 `cargo build -p contextforge-core` exit 0 且不引入 real-provider dep（cfg-gated）— verified by **TEST-19.1.1**（trait 对象安全 + 默认 build 无 `fastembed` 编译）
- [x] **AC2**: `DeterministicEmbeddingProvider` 确定性 — 同 text 两次 `embed` 逐分量相等，无模型 / 无网络 / 无新 dep，默认构建启用 — verified by **TEST-19.1.2**（确定性单测 PASS）
- [x] **AC3**: dim 一致 — `embed` 输出每向量 `.len() == dim()`，多文本批量恒定，单位范数 invariant — verified by **TEST-19.1.3**（dim 一致 + 范数单测 PASS）
- [x] **AC4**: 候选评估 evidence — `docs/spikes/phase-19-embedding-candidates.md` 记 3 路（fastembed-rs/candle/ort）Linux + Windows MSVC 构建/模型获取/API/选定理由（非伪造，stop-condition 据实记）；`docs/spikes/phase-19-embedding-fastembed.md` 记选定 provider 真实 embed 样例或实测受阻凭据 — verified by **TEST-19.1.4**（两 evidence 文档存在 + 内容实证）
- [x] **AC5**: 既有不退化 — 默认 `cargo test --workspace` 全 PASS（embedding-fastembed 默认不启用，gated path 不入默认编译）；`go test ./...` 全 PASS — verified by **TEST-19.1.5**（`cargo test --workspace` 0 failed + `go test ./...` 0 failed）+ §10 实测
- [x] **AC6**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched master` PR 触及行 0 未标注命中 — verified by **TEST-19.1.6**（§10 记录的 D2 lint 实跑输出）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-19.1.1 | trait 对象安全 + 默认 build 无 real-provider dep | `core/src/embedding/tests.rs` + `cargo build -p contextforge-core` | Done |
| TEST-19.1.2 | deterministic 确定性（同 text 逐分量相等） | `core/src/embedding/tests.rs` | Done |
| TEST-19.1.3 | dim 一致 + 单位范数 invariant | `core/src/embedding/tests.rs` | Done |
| TEST-19.1.4 | 候选评估 + 选定 provider evidence 实证 | `docs/spikes/phase-19-embedding-{candidates,fastembed}.md` | Done |
| TEST-19.1.5 | 默认 cargo test --workspace + go test ./... 0 failed | 全 workspace | Done |
| TEST-19.1.6 | D2 lint --touched master 0 未标注命中 | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（高）real provider 平台/模型门槛**：fastembed(ort)/candle native runtime + 模型下载在 Windows MSVC / CI 受阻（承 phase-19 §7 R1，类比 phase-18 sqlite-vec MSVC 受阻凭据）。
  - **缓解**：deterministic 缺省 provider 兜底（无模型 / 无网络 / 默认构建 0 新 dep，供 CI/smoke/test/wiring）；real provider `embedding-fastembed` feature 默认关闭。stop-condition（ADR-013）：两平台均不可构建 → deterministic 续跑 wiring/smoke（诚实标注），real-model 召回 + ADR ratify 据实测延后 [SPEC-OWNER:phase-future.embedding-provider-full]。
- **R2（中）deterministic 向量非语义相关**：hash 派生向量无语义结构，不能替代真实召回评测（合成不可区分，承 phase-18 §comparison caveat）。
  - **缓解**：deterministic provider 明确定位为 CI/smoke/test/wiring 缺省，evidence 标注其非语义；真实召回唯由 real provider 在 task-19.5 dogfood 语料产出，ADR-013 禁据 deterministic 数据 ratify。
- **R3（中）模型下载非确定 / 网络依赖**：real provider 首次 lazy load 拉模型，CI 无网或慢则不可复现。
  - **缓解**：lazy load 经 `OnceCell` 守护、模型本地缓存（本地优先）；CI / smoke 默认走 deterministic provider（无网）；real provider 跑批限 dev / Linux 有网环境，evidence 记模型来源与缓存路径。
- **R4（低）batch 维度 / 空输入边界**：批量 `embed` 多文本维度一致性 + 空输入语义需契约稳定，否则下游 wiring 越界。
  - **缓解**：`dim()` 固定 + 单测守 batch 每向量长恒定 + 空输入显式语义（`Ok(vec![])` 或 `EmptyInput`，evidence 记定）；`#[non_exhaustive]` error 让下游 match add-only-safe。

## 9. Verification Plan

```bash
# 默认构建（无 feature）：deterministic provider + trait，0 新 dep
cargo build -p contextforge-core
cargo test --workspace        # 默认 feature，embedding-fastembed gated 不入编译

# real provider（Linux dev，fastembed 依 spike 选定）：feature build + 真实 embed 探针
cargo build -p contextforge-core --features embedding-fastembed
cargo test -p contextforge-core --features embedding-fastembed -- embedding

# Windows MSVC：real provider 构建实证（结果记 candidates evidence；受阻则 stop-condition）
cargo build -p contextforge-core --features embedding-fastembed

# 既有 Go 控制面不退化
go test ./...

# ADR-014 D2 spec-drift lint
bash scripts/spec_drift_lint.sh --touched master
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**：2026-05-30
- **改动文件**：`core/src/embedding/mod.rs`（新增）、`core/src/embedding/traits.rs`（EmbeddingProvider + EmbeddingError `#[non_exhaustive]`）、`core/src/embedding/deterministic.rs`（DeterministicEmbeddingProvider，Sha256+splitmix64，dim 384，默认构建）、`core/src/embedding/fastembed_provider.rs`（FastEmbedProvider real provider，cfg-gated embedding-fastembed）、`core/src/embedding/tests.rs`（4 unit test：3 deterministic + 1 ignored real fastembed）、`core/src/lib.rs`（`pub mod embedding;`）、`core/Cargo.toml`（fastembed optional + embedding-fastembed feature）、`docs/spikes/phase-19-embedding-{candidates,fastembed}.md`（新增）、`docs/s2v-adapter.md`（19.1 行 Done）
- **commit 列表**：见本 task PR（分支 `feat/task-19.1-spike-embedding-provider`）；合入后以 merge commit 为准
- **§9 Verification 结果**：选定 **fastembed-rs 4.9.1**（rustls，all-MiniLM-L6-v2 dim 384）。Linux build 30.4s + Windows MSVC build 1m11s（均 exit 0，跨平台可构建，区别于 sqlite-vec MSVC 受阻）；默认 build 8.6s 不含 fastembed（0 新 dep）；默认 `cargo test --workspace` 全 PASS（含 3 deterministic 单测）；in-repo real embed `#[ignore]` test PASS（dim 384 真实向量）；`go test ./...` 全 PASS；D2 lint `--touched master` 0 命中。详 `docs/spikes/phase-19-embedding-{candidates,fastembed}.md`
- **剩余风险 / 未做项**：default backend wiring 见 [SPEC-OWNER:task-19.2-default-backend-wiring]；真实 dogfood 召回见 [SPEC-OWNER:task-19.5-real-recall-eval]；ADR-023 ratify 见 [SPEC-OWNER:task-19.6-adr-023-ratify]；real-model 召回闭环 [SPEC-OWNER:phase-future.embedding-provider-full]；remote provider 见 [SPEC-DEFER:phase-future.embedding-provider-remote]
- **下游 task 影响**：task-19.2（消费 EmbeddingProvider 接 index/query 热路径）/ task-19.5（fastembed 喂 dogfood 真实 SemanticRecall@K）/ task-19.3（embedding_provider provenance 源自 trait name()）
