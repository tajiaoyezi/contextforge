# ADR `027`: `embedding-provider-abstraction`

**Status**: Accepted (2026-05-30 Proposed；2026-05-31 task-22.4 据 task-22.1/22.2/22.3 真实非合成验证 ratify Proposed→Accepted，ADR-013。D1-D5 抽象经真实 Go config round-trip + Rust factory/dim/cache 单测 + 远程契约测试（fixture，不打网络）+ 默认 0 网络 dep 验证；远程 provider 真实网络联调 / 召回质量 + health 远程探针真实命中如实 defer，受阻不伪造 ratify。见 §Ratification Amendment。)
**Category**: 数据面 / embedding provider 层 / 本地优先
**Date**: 2026-05-30
**Decided By**: 主 agent (ADR-012 自治)；tajiaoyezi ratification at v0.15.0 closeout
**Related**: ADR-004 (local-first-privacy-baseline) / ADR-008 (core-library-selection，含 2026-05-30 embedding crate Amendment) / ADR-002 (sqlite-tantivy-layered-storage) / ADR-006 (recall-eval-acceptance-gate) / ADR-020 (health-component-breakdown，`probe_embed` config-only) / ADR-013 (禁伪造凭据) / Phase 19 task-19.1 (`EmbeddingProvider` trait + `DeterministicEmbeddingProvider` + `FastEmbedProvider`) / Phase 22 (embedding-provider-completion)

## Context

Phase 19（v0.12.0）落地了 `EmbeddingProvider` trait（`core/src/embedding/traits.rs`）与两个实现：

1. `DeterministicEmbeddingProvider`（`core/src/embedding/deterministic.rs`，`DEFAULT_DIM=384`）—— 无模型缺省，默认构建可用，供 CI / smoke / wiring（无语义结构，不测真实召回）。
2. `FastEmbedProvider`（`core/src/embedding/fastembed_provider.rs`，`embedding-fastembed` feature，`all-MiniLM-L6-v2` dim 384）—— real 模型，feature-gated（默认构建 0 dep）。

但 provider **选择是硬编码的**：`core/src/server.rs:299-324` 的语义路径恒构造 `DeterministicEmbeddingProvider::default()` + `BruteForceVectorBackend`。无运行时配置选择、无 dim 协商校验、无重复内容缓存、无远程 provider 通路。`docs/roadmap.md` §3.3 把 4 个真实 marker 聚合到 Phase 22：

- `[SPEC-OWNER:phase-future.embedding-provider-full]`（task-19.1 / task-19.5 / adr-006）—— 完整 provider 层（选择 / 配置 / 缓存 / 远程）。
- `[SPEC-DEFER:phase-future.embedding-provider-remote]`（adr-008:56 + phase-19 §2）—— OpenAI / Cohere 远程 provider。
- `[SPEC-DEFER:phase-future.embedding-cache]`（phase-19-embedding spike）—— content-hash → embedding 缓存。
- `[SPEC-DEFER:phase-future.embed-remote-probe]`（adr-020:103）—— health 探针含远程可达性。

约束硬底（ADR-004）：本地优先 / 隐私敏感是 ContextForge 的产品红线 —— 远程 provider 必须显式 opt-in、默认构建不拉网络 dep、密钥不入库不入日志。

## Decision

引入 **可配置的 embedding provider 抽象层**，在保持 `EmbeddingProvider` trait 不变（add-only）的前提下增加「配置选择 + 维度协商 + 缓存包装 + 远程骨架」四个能力，本地优先为不可妥协红线：

### D1 — `[embedding]` 配置 + 工厂选择

`internal/config.Config` 加 `Embedding EmbeddingConfig{Provider, Dim}`（add-only TOML `[embedding]` 段，仿既有 `[remote]` 段）。Rust 侧新增工厂 `select_provider(provider_name, dim)`：`"deterministic"`（缺省）/ `"fastembed"`（feature 下 real，未编入时明确报「feature 未启用」）/ `"remote"`（D3 骨架）。缺省 `provider="deterministic"` → 既有语义路径行为逐字不变（向后兼容）。

### D2 — dim 协商校验

工厂选出 provider 后校验其 `dim()` 与配置 / 向量索引 `VectorIndexConfig.dim` 一致；不一致即返回 `EmbeddingError::DimMismatch{expected, got}`（既有变体），**不静默截断 / pad**。缺省 dim 仍 384（`DEFAULT_DIM`），既有索引不受影响。

### D3 — content-hash 缓存（包装器）

`CachingEmbeddingProvider` 包裹任意 `Arc<dyn EmbeddingProvider>`、本身实现 `EmbeddingProvider` trait（装饰器）：以 `Sha256(text)` 为 key 缓存 embedding，命中跳过底层 embed，内容变更（hash 变化）即未命中（失效）。后端：内存缺省 + 可选 SQLite 持久化（承 ADR-002 SQLite 分层，独立表 / 文件，add-only schema）。

### D4 — 远程 provider 骨架（feature-gated，本地优先红线）

`RemoteEmbeddingProvider`（OpenAI / Cohere 风格 HTTP，`embedding-remote` feature）实现 `EmbeddingProvider` trait：请求体构造 + 响应 JSON 解析（`data[].embedding`）+ HTTP / 解析错误路径映射 `EmbeddingError`。**默认构建不编入**（feature off → 0 网络 dep，承 ADR-004 + ADR-008 D5）；运行时**显式 opt-in**（配置 + 既有 `RemoteProviderConfig.Enabled`）；密钥从环境 / 配置读，不入库不入日志。契约正确性用固定 fixture 的确定性测试断言（请求构造 / 响应解析 / 错误路径，不打真实网络）。

### D5 — 本地优先红线（不可妥协）

embedding 层的缺省与默认构建恒为本地、无网络、无模型 dep：缺省 provider = 确定性 identity 实现；real（fastembed）与远程（remote）均 feature-gated + 显式 opt-in。任何远程激活必须经用户显式配置；无配置 / 默认构建下绝不发起网络请求（health 远程探针亦 opt-in）。此红线优先于任何 provider 能力扩展。

## Consequences

- **Positive**: provider 经配置选择（确定性 / fastembed / 远程）+ dim 协商不静默；重复内容缓存省算力（real provider 尤甚）；远程 provider 通路就绪但本地优先不破（默认 0 网络 dep + 显式 opt-in）；`EmbeddingProvider` trait 不变（add-only），既有 `with_embedder` / `search_semantic` 消费方零改动。
- **Negative / open**: provider 矩阵变大（确定性 / fastembed / 远程 × 缓存包装），配置组合需测试覆盖；远程 provider 真实可达性 / 召回质量 CI 不可验证（需密钥 + 网络），契约骨架与真实行为间存在验证缺口。
- **Ratification**: 本 ADR **Proposed**。task-22.1（配置 + 工厂 + dim 协商）/ task-22.2（缓存）/ task-22.3（远程骨架契约测试）落地 + task-22.4 smoke v12 通过后，于 v0.15.0 closeout 据真实非合成验证 ratify Proposed→Accepted（ADR-013：禁据合成 / 伪造 ratify）。远程 provider 真实网络联调 / 真实召回质量按 `[SPEC-DEFER:phase-future.embedding-provider-remote]` 如实 defer，受阻则文档化 stop-condition、不据无网络环境伪造 ratify。
- **Follow-ups**: 缓存淘汰策略（LRU / 容量上限）`[SPEC-DEFER:phase-future.cache-lru]`（roadmap §4 长尾）；远程 provider 真实联调 + 密钥管理 `[SPEC-DEFER:phase-future.embedding-provider-remote]`；health 远程探针真实命中 `[SPEC-DEFER:phase-future.embed-remote-probe]`（task-22.4）；rust-native eval runner 真实远程召回 `[SPEC-DEFER:phase-future.rust-native-eval-runner]`（roadmap §4）。

## Ratification Amendment (v0.15.0 / task-22.4, 2026-05-31)

本 ADR 于 v0.15.0 closeout 据 task-22.1/22.2/22.3 的**真实非合成验证** ratify **Proposed → Accepted**（ADR-013：禁据合成 / 无网络伪造 ratify）。D1–D5 各项的真实验证依据：

- **D1（配置选择 + 工厂）**：`go test ./internal/config/ -run TestTask221` `[embedding]` TOML round-trip PASS（含/不含段 + 既有 `[remote]`/`[[collections]]` 不受影响）；`cargo test embedding::tests` `select_provider("deterministic"/"")` 等价 Phase 19 `default()`（字节相同 embed）+ `server.rs` 语义路径走工厂后 `test_22_1_5` + 既有 `test_19_3` 仍 PASS。**真实验证、向后兼容**。
- **D2（dim 协商）**：`negotiate_dim(384,128)` → `DimMismatch{expected:128,got:384}`（默认构建单测）+ feature 构建 `select_provider("fastembed",128)` → `DimMismatch`（network-free，仅读 `dim()`）。**不静默截断/pad，真实验证**。
- **D3（content-hash 缓存）**：`cargo test embedding::cache` 4/4 PASS（命中跳底层计数断言 + 字节相同 / 失效 + 批量顺序 / SQLite 往返 inner 0 调用 + 内存缺省不落盘）。**确定性真实验证**。
- **D4（远程骨架，feature-gated）**：`cargo test --features embedding-remote embedding::remote_provider` 4/4 PASS（`build_request_body` / `parse_response` / 错误路径 / factory 分支，全 fixture，**不打真实网络**）；ureq 2.12.1 Windows MSVC 编译通过。**契约真实验证**。
- **D5（本地优先红线）**：默认构建 `cargo tree -p contextforge-core | grep ureq` **空** + deterministic 缺省 → 0 网络 / 0 模型 dep；`probe_embed` 默认 config-only（`TEST-22.4.1` 守 opt-in inert）。**红线真实守护**。

**如实 defer（ADR-013，未据无网络伪造 ratify）**：远程 provider 真实网络联调 / 真实 API 密钥 / 真实召回质量 `[SPEC-DEFER:phase-future.embedding-provider-remote]` + health 远程探针真实命中 `[SPEC-DEFER:phase-future.embed-remote-probe]`——CI / 无人值守环境无密钥 + 无网络，骨架 + 契约测试达标即视抽象层验证通过，真实命中**未**标、不伪造。ratify 范围 = provider **抽象层**（配置选择 / dim 协商 / 缓存 / 远程骨架 / 本地优先），不含远程真实集成质量。证据见 `docs/releases/v0.15.0-evidence.md` §3。

## Amendment (Phase 31 / v0.24.0, 2026-06-03 — add-only, 正文不溯改)

Phase 31（ADR-036 D2）以 add-only 方式给 `CachingEmbeddingProvider` 的 L1 缓存加容量上界，**不溯改正文**（ADR-014 D5）：

- **L1 缓存有界化（cache-lru）**：task-31.2（PR #207）`core/src/embedding/cache.rs` 的 L1 由无界 `Mutex<HashMap>` 改为 `BoundedCache`（map + VecDeque，FIFO-on-insert 驱逐，0 新 dep）+ `DEFAULT_EMBEDDING_CACHE_CAP=50_000` + `with_capacity` 构造（`new`/`with_sqlite` 签名兼容，默认 cap 不破现有命中）。长跑 daemon 的 L1 内存随唯一文本数单调无界增长的风险解除。`cargo test embedding::cache` 5 passed（含既有 22.2.* + 新 cap 驱逐测试）。L2 SQLite 上限延后 `[SPEC-DEFER:phase-future.l2-cache-eviction]`。

依赖变更：手写 LRU 0 新 dep。详见 ADR-036 Ratification + `docs/releases/v0.24.0-evidence.md`。

## Amendment (Phase 33 / v0.26.0, 2026-06-03 — add-only, 正文不溯改)

Phase 33（ADR-038 D1）以 add-only 方式给 `CachingEmbeddingProvider` 的 **L2 SQLite 缓存**加容量上界，承 Phase 31 的 L1 `BoundedCache`，**不溯改正文 + 不溯改 Phase 31 Amendment**（ADR-014 D5）：

- **L2 缓存有界化（l2-cache-bound）**：task-33.1（PR #218）`core/src/embedding/cache.rs` 的 L2 `sqlite_put` 在 `INSERT OR REPLACE` 后做 row-count cap + rowid-FIFO 驱逐（`DELETE ... WHERE provider=? AND dim=? AND rowid NOT IN (SELECT rowid ... ORDER BY rowid DESC LIMIT cap)`），`DEFAULT_L2_EMBEDDING_CACHE_CAP=50_000` + add-only `with_sqlite_capacity` 构造（`with_sqlite` 委派，签名兼容）。用隐式 rowid 作 FIFO 序 → **0 新 dep / 0 schema migration**。`cargo test embedding::cache` 7 passed（含既有 + `test_33_1_1_l2_cap_evicts_oldest_fifo` / `test_33_1_2_default_l2_cap_keeps_modest_workload`）。
- **opt-in-path caveat（ADR-013 据实）**：`with_sqlite`/`with_sqlite_capacity` 无生产调用点（test-only），shipped daemon 走 memory-only L1 → 这是 **opt-in 路径的纵深防御**，据实不夸大为 live leak。带 `created_at` 列的真 LRU 须 `ALTER` 既有用户文件（破 0-migration）→ honest-defer `[SPEC-DEFER:phase-future.l2-cache-true-lru]`。

依赖变更：rowid-FIFO 0 新 dep。详见 ADR-038 Ratification D1 + `docs/releases/v0.26.0-evidence.md`。

## Amendment (Phase 37 / v0.30.0, 2026-06-06 — add-only, 正文不溯改)

Phase 37（ADR-042 D1/D2/D3）兑现本 ADR v0.15.0 ratify 时记为 honest-defer 的 remote provider live 维度 `[SPEC-DEFER:phase-future.embedding-provider-remote]`（「真实远程端点联调 / API key / 真实召回质量 deferred — CI has no credentials」），**不溯改正文 + 不溯改本 ADR 既有 Ratification (v0.15.0) + Phase 31/33 Amendment**（ADR-014 D5）：

- **`embedding-provider-remote` fulfilled（real live 联调 + 真实语义召回 measured）**：task-37.1（PR #242）新增 env-gated harness `core/tests/remote_embedding_recall.rs`，在作者手工标注集（15 case/16 doc，含故意近义干扰）+ 同一 `BruteForceVectorBackend` 精确余弦路径上对照 real remote 模型 vs deterministic（model-free）基线的 `recall@1`/`recall@3`。主 agent 本机对真实 SiliconFlow 端点（`Qwen/Qwen3-Embedding-8B`，dim=1024，3 次 run）实测：remote recall@1=0.8667–0.9333（跨 run 波动）/ recall@3=1.0000（3 次稳定）vs deterministic 0.0000/0.0667（delta@3=+0.9333）。`RemoteEmbeddingProvider`（`build_request_body`/`parse_response`/ureq `embed`）自 task-22.3 已实现——本 phase 0 行 provider 核心改动。
- **config plumbing follow-up fulfilled**：task-37.2（PR #243）兑现 `core/src/embedding/factory.rs:52` 注释 "config plumbing is a follow-up"——Go `RemoteProviderConfig` add-only `Model` 字段 + `setRemoteEnv` 跨进程 env-bridge（镜像 `setVectorEnv`，`[remote]` 段 → 导出 `CONTEXTFORGE_REMOTE_ENDPOINT`/`_MODEL`/`_PROVIDER`，env-wins），spawned core 经既有 `select_provider` remote arm env 路径拾取；API key env-only 永不进 config.toml（安全红线）；Rust core 0 toml dep。
- **诚实边界（ADR-013）**：小型手工标注集，recall@3=1.0 = real 模型把明显语义对排在近义干扰之上的正确性证明、**非大基准质量断言**（大语料语义质量续 `[SPEC-DEFER:phase-future.embedding-large-corpus-recall]`）；recall@1 跨 run 波动据实记录（remote 模型/服务非完全确定）；CI honest-defer——remote 是付费外部 API、无免费 service container（与 qdrant 不同），召回由本机已认证 run 实测、非每次 CI run 守护 `[SPEC-DEFER:phase-future.embedding-remote-ci-credential]`；多 provider / reranker / health-probe live 续 `[SPEC-DEFER:phase-future.embedding-multi-provider-live]` / `[SPEC-DEFER:phase-future.embedding-remote-reranker-live]` / `[SPEC-DEFER:phase-future.embedding-remote-health-probe]`。

依赖变更：0 新 dep（`ureq` 自 task-22.3 已 optional）。详见 ADR-042 Ratification (v0.30.0) + `docs/releases/v0.30.0-evidence.md`。

## Amendment (Phase 40 / v0.33.0) — L2 embedding 缓存访问序 LRU 增量 (add-only)

> add-only Amendment（不溯改本 ADR D-body / 既有 Amendment，ADR-014 D5）。承 L2 SQLite embedding 缓存有界化（Phase 33 / ADR-038 D1 在本 ADR 之上加 rowid-FIFO bound）。

Phase 40 / v0.33.0（ADR-045 D2）在 L2 缓存有界化（rowid-FIFO，Phase 33）之上补**访问序 LRU**：`core/src/embedding/cache.rs` `sqlite_get` 命中时（仅有限 cap）`INSERT OR REPLACE` 原样回写命中行 bump 隐式 rowid 到表尾，使既有 rowid 序驱逐由插入序 FIFO 升访问序 LRU（驱逐最久未用而非最早插入）。复用既有隐式 rowid、**0 新 dep / 0 schema migration**；默认返回结果不变（bump 原样回写相同字节）。`with_sqlite` 无生产调用点（opt-in-path）→ 现网零影响。验证 TEST-40.2.1/40.2.2。详见 ADR-045 Ratification (v0.33.0)。
