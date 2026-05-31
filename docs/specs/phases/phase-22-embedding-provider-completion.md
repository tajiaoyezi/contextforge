# Phase 22 · embedding-provider-completion

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 把 Phase 19（v0.12.0）落地的「`DeterministicEmbeddingProvider` 缺省 + 单一 `FastEmbedProvider` real provider（feature-gated）」扩成**完整的 embedding provider 层**：运行时经配置选择 provider、embedding 缓存、远程 provider（OpenAI / Cohere）HTTP 骨架，以及 health 远程探针。v0.15.0 收口。对应 `docs/roadmap.md` §3.3。
>
> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md` §3.3 → `core/src/embedding/mod.rs` + `core/src/embedding/traits.rs`（`EmbeddingProvider` trait + `EmbeddingError`）+ `core/src/embedding/deterministic.rs`（`DEFAULT_DIM=384` 缺省 provider）+ `core/src/embedding/fastembed_provider.rs`（real provider，`embedding-fastembed` feature）→ `core/src/server.rs:293-324`（语义路径当前硬编码 `DeterministicEmbeddingProvider::default()` + `BruteForceVectorBackend` 的工厂点）→ `core/src/retriever/mod.rs::with_embedder` / `index_chunks_semantic` / `search_semantic`（embedder 消费方）→ `core/src/retriever/vector/types.rs`（`VectorIndexConfig.dim`）→ `internal/config/config.go`（`Config` + 既有 `[remote]` 表）→ `core/src/health.rs::probe_embed`（ADR-020 config-only 探针）→ AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5）→ `docs/decisions/adr-004-local-first-privacy-baseline.md`（本地优先 / 远程显式 opt-in）→ `docs/decisions/adr-008-core-library-selection.md`（含 2026-05-30 embedding crate Amendment）→ `docs/decisions/adr-002-sqlite-tantivy-layered-storage.md`（SQLite 持久化基线）。
>
> **ADR 影响面（已识别）**：
> - **ADR-027 embedding-provider-abstraction（新，Proposed）**：记 provider 层（确定性 / fastembed / remote 经配置选择）+ 远程 opt-in + 本地优先红线，承 ADR-004 local-first / ADR-008 core-library。落地后据真实非合成验证 ratify（ADR-013）。
> - 可能触及 **ADR-020（health-component-breakdown）**：`probe_embed` 从 config-only 扩到可选远程探针——以 add-only amendment / 记录方式确认 `[SPEC-DEFER:phase-future.embed-remote-probe]` 的真实命中如实 defer，不溯改 ADR-020 正文（D5）。

## 1. 阶段目标

v0.15.0 ship 后，ContextForge 的 embedding 层从「Phase 19 硬编码确定性缺省 + 单一 feature-gated fastembed」升级为**可经配置选择的 provider 层**：运行时按 `[embedding]` 配置在「确定性 identity 实现 / fastembed real / 远程 HTTP」之间选择，带 dim 协商校验；重复内容经 content-hash 缓存避免重复 embed；远程 provider（OpenAI / Cohere）以 feature-gated HTTP 骨架 + 契约级确定性测试存在（请求构造 / 响应解析 / 错误路径不打真实网络）；health 探针在配置远程时可选做真实可达性探测。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. `[embedding]` 配置（`provider` / `dim`）+ Rust 侧工厂按配置在确定性 / fastembed / 远程之间选择 provider + dim 协商校验（provider `dim()` 与配置 / 向量索引 `VectorIndexConfig.dim` 不一致即明确 `DimMismatch`，不静默）（AC1）
2. content-hash → embedding 缓存（内存 + 可选 SQLite 持久化，承 ADR-002）：命中跳过重复 embed，内容变更（hash 变化）失效，确定性命中 / 失效单测可断言（AC2）
3. `RemoteEmbeddingProvider`（OpenAI / Cohere HTTP，feature-gated）+ 契约级确定性测试（请求体构造 / 响应 JSON 解析 / HTTP 错误路径，不打真实网络）；真实联调 + 密钥如实 defer（AC3）
4. health `probe_embed` 在配置远程 provider 时可选做远程可达性探针（feature / 显式 opt-in）；config-only 缺省行为不变（AC4）
5. v0.15.0 release docs + phase §6 闭合 + ADR-027 ratify 或据实测记录 + smoke（AC5）
6. ADR-014 D1-D5（第十三次激活）全通过（AC6）

**v0.x 版本号决策**：v0.15.0 minor release（embedding provider 层完整化；默认构建仍 0 模型 / 0 网络 dep——确定性 identity 实现为缺省，fastembed / 远程均 feature-gated + 显式 opt-in，承 ADR-004 local-first）。

## 2. 业务价值

承接 `docs/roadmap.md` §3.3 聚合的 4 个真实 marker，闭合 embedding 层从 spike 缺省到生产可配置的差距：

- **provider 经配置选择 + dim 协商**：Phase 19 把 provider 选择硬编码在 `core/src/server.rs:301`（恒 `DeterministicEmbeddingProvider::default()`）。本 phase 让运行时经 `[embedding]` 配置选择 provider 并校验维度协商，兑现 `[SPEC-OWNER:phase-future.embedding-provider-full]`（task-19.1 / task-19.5 / adr-006）。
- **embedding 缓存**：相同内容（hash 相同）重复 embed 浪费算力（real provider 尤甚）。content-hash 缓存避免重复 embed，兑现 `[SPEC-DEFER:phase-future.embedding-cache]`（phase-19-embedding spike）。
- **远程 provider 骨架**：在不违本地优先（ADR-004）前提下，为 OpenAI / Cohere 等远程 embedding 提供 feature-gated HTTP 骨架 + 契约测试通路，兑现 `[SPEC-DEFER:phase-future.embedding-provider-remote]`（adr-008:56 + phase-19 §2）。
- **health 远程探针**：把 ADR-020 留下的 config-only embed 探针扩到可选真实可达性探测，兑现 `[SPEC-DEFER:phase-future.embed-remote-probe]`（adr-020:103）。
- **PRD §Constraints 安全基线**：远程 provider 显式 opt-in、不在默认构建拉入网络 dep、密钥不入库不入日志——可解释性 / 隐私不退化。

**不在本 phase scope**：

- Hybrid scoring（BM25 + Vector 融合）[SPEC-DEFER:phase-future.hybrid-scoring]——v0.14.0 / Phase 21
- Reranker（cross-encoder）[SPEC-DEFER:phase-future.reranker]——v0.14.0 / Phase 21
- 向量索引持久化 / hnsw 图持久化 [SPEC-DEFER:phase-future.hnsw-graph-persistence]——v0.16.0 / Phase 23
- sqlite-vec Windows MSVC 跨平台 [SPEC-DEFER:phase-future.sqlite-vec-cross-platform]——v0.16.0 / Phase 23
- 远程 embedding provider 的真实联调 / 密钥 / 真实召回质量 [SPEC-DEFER:phase-future.embedding-provider-remote]——本 phase 落 feature-gated 骨架 + 契约测试；真实网络命中如实 defer（ADR-013）
- 缓存淘汰策略（LRU / 容量上限）[SPEC-DEFER:phase-future.cache-lru]——长尾 backlog（roadmap §4）

## 3. 涉及模块

### 22.1 provider 配置 + 工厂选择 + dim 协商（task-22.1）

- 修改 `internal/config/config.go`——`Config` 加 `Embedding EmbeddingConfig`（add-only），`EmbeddingConfig{Provider string, Dim int}`；TOML codec 加 `[embedding]` 段（仿既有 `[remote]` 段的 encode / decode / assign 三处）
- 新增 `core/src/embedding/factory.rs`（或扩 `core/src/embedding/mod.rs`）——`select_provider(provider_name, dim) -> Result<Arc<dyn EmbeddingProvider>, EmbeddingError>`：`"deterministic"`（确定性 identity 实现）/ `"fastembed"`（feature 下 real，未编入 feature 时明确报「未启用」错误）/ `"remote"`（task-22.3 骨架）；dim 协商校验（provider `dim()` 与请求 dim / `VectorIndexConfig.dim` 不一致 → `EmbeddingError::DimMismatch`）
- 修改 `core/src/server.rs:299-324`——语义路径从硬编码 `DeterministicEmbeddingProvider::default()` 改为经工厂按配置选择（缺省仍确定性 identity 实现，行为不变）
- 同源 Go + Rust tests（Go：`[embedding]` round-trip + 缺省；Rust：工厂选择 ≥3 路 + dim 协商 mismatch 报错 + 缺省不退化）

### 22.2 embedding 缓存（task-22.2）

- 新增 `core/src/embedding/cache.rs`——`CachingEmbeddingProvider`（包裹任意 `Arc<dyn EmbeddingProvider>`，实现 `EmbeddingProvider` trait）：以 `Sha256(text)` 为 key 缓存 embedding；命中跳过底层 embed，未命中 embed 后写入；内容变更（hash 变化）即未命中（失效）
- 缓存后端：内存（`HashMap`）缺省 + 可选 SQLite 持久化（承 ADR-002 SQLite 分层；feature / 配置 opt-in，复用 `rusqlite` bundled）
- 同源 Rust tests（≥3：相同文本第二次命中不调底层 / 不同文本未命中 / SQLite 持久化往返；用确定性 identity 实现做底层 provider 断言命中失效）

### 22.3 远程 provider 骨架（task-22.3）

- 新增 `core/src/embedding/remote_provider.rs`——`RemoteEmbeddingProvider`（feature-gated，如 `embedding-remote`）：OpenAI / Cohere 风格 HTTP embedding，实现 `EmbeddingProvider` trait；请求体构造（model / input / dim）+ 响应 JSON 解析（`data[].embedding`）+ HTTP / 解析错误路径映射到 `EmbeddingError`
- 默认构建不编入（feature off → 0 网络 dep；承 ADR-004 local-first + ADR-008 D5）
- 契约级确定性测试：请求构造 / 响应解析用**固定 fixture JSON**断言，错误路径用构造的错误响应断言——**不打真实网络**（ADR-013）
- 修改 `core/Cargo.toml`——加 `embedding-remote` feature + HTTP client optional dep（如 `reqwest` rustls，承 fastembed 既有 rustls 口径，避 OpenSSL）；默认构建不拉
- 同源 Rust tests（≥3：OpenAI 请求体构造 / 响应解析 / 错误路径；全 fixture，无网络）

### 22.4 health 远程探针 + smoke + closeout（task-22.4）

- 修改 `core/src/health.rs::probe_embed`——配置远程 provider 时可选做远程可达性探针（feature / 显式 opt-in）；config-only 缺省行为不变（ADR-020 D1）
- 修改 `scripts/console_smoke.sh`——v12：`[embedding]` 配置选择 + 缓存命中可观测断言（确定性路径，非真实网络）
- 新增 `docs/releases/v0.15.0-{evidence,artifacts}.md` + `README.md` v0.15 段 + `RELEASE_NOTES.md` v0.15.0 段
- 修改 `docs/decisions/adr-027-embedding-provider-abstraction.md`——据实测 Proposed→Accepted 或记录维持
- 修改 `docs/s2v-adapter.md`（Phase 22 Draft→Done + Tasks 0→4；ADR-027 状态；BDD feature 行）

### BDD feature

- 新增 `test/features/phase-22-embedding-provider-completion.feature`（≥4 scenario：provider 经配置选择 / embedding 缓存命中失效 / 远程 provider 契约构造解析 / health 远程探针 opt-in）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 22.1 | `internal/config` `[embedding]` 配置 + `core/src/embedding/factory.rs` 工厂选择 + dim 协商 + `core/src/server.rs` 改用工厂 | `../tasks/task-22.1-provider-config-selection.md` |
| 22.2 | `core/src/embedding/cache.rs` `CachingEmbeddingProvider`（content-hash → embedding，内存 + 可选 SQLite） | `../tasks/task-22.2-embedding-cache.md` |
| 22.3 | `core/src/embedding/remote_provider.rs` `RemoteEmbeddingProvider`（OpenAI/Cohere HTTP，feature-gated）+ 契约级确定性测试 | `../tasks/task-22.3-remote-provider-skeleton.md` |
| 22.4 | `core/src/health.rs` 远程探针 + smoke v12 + v0.15.0 closeout + ADR-027 ratify | `../tasks/task-22.4-closeout-v0.15.0.md` |

## 5. 依赖关系

- **task-22.1**（provider 配置 + 工厂）= 首项，提供 provider 选择 seam；解锁 22.2 / 22.3 经工厂接入。
- **task-22.2**（缓存）dep 22.1 工厂（`CachingEmbeddingProvider` 包裹工厂选出的 provider）；可与 22.3 并行（cache.rs vs remote_provider.rs 写路径不相交）。
- **task-22.3**（远程骨架）dep 22.1 工厂（`"remote"` 选项落地）；可与 22.2 并行。
- **task-22.4**（closeout）dep 22.1 + 22.2 + 22.3 全 Done。
- 外部：ADR-027（本 phase 新 Proposed）/ ADR-004（local-first 红线）/ ADR-008（core-library + embedding crate Amendment）/ ADR-002（SQLite 分层）/ ADR-020（health embed 探针）/ ADR-014 第十三次激活 / Phase 19 `EmbeddingProvider` trait + `FastEmbedProvider`（已落地）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（每条 AC 含 ADR-014 D3 verified by 显式 owner；Draft 阶段未勾选，实施后逐条置 `[x]`）**：

- [x] **AC1**：`[embedding]` 配置（`provider` / `dim`）+ Rust 工厂按配置在确定性 identity 实现 / fastembed / 远程之间选择 provider + dim 协商校验（不一致即 `EmbeddingError::DimMismatch`，不静默）；缺省仍确定性 identity 实现，语义路径行为不变 — verified by task-22.1 §6 AC1-4 + phase-smoke step 1
- [x] **AC2**：content-hash → embedding 缓存（内存 + 可选 SQLite 持久化，承 ADR-002）：命中跳过底层 embed，内容变更（hash 变化）失效，确定性命中 / 失效单测可断言 — verified by task-22.2 §6 AC1-3 + phase-smoke step 2
- [x] **AC3**：`RemoteEmbeddingProvider`（OpenAI / Cohere HTTP，feature-gated）+ 契约级确定性测试（请求构造 / 响应解析 / 错误路径，不打真实网络）；默认构建 0 网络 dep；真实网络联调 + 密钥按 ADR-013 如实 defer，受阻不伪造（禁伪造，stop-condition 见 §7 R2）— verified by task-22.3 §6 AC1-4 + phase-smoke step 3
- [x] **AC4**：health `probe_embed` 在配置远程 provider 时可选做远程可达性探针（feature / 显式 opt-in）；config-only 缺省行为不变（ADR-020 D1）；真实远程命中按 ADR-013 如实 defer — verified by task-22.4 §6 AC1 + phase-smoke step 4
- [x] **AC5**：v0.15.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ ADR-027 据实测 ratify 或记录 + phase §6 闭合 — verified by task-22.4 §6 AC2-3
- [x] **AC6**：ADR-014 cross-validation gate 全套通过（第十三次激活）— D1 mapping + D2 lint `--touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-21 不溯改 — verified by task-22.4 closeout PR body

**端到端 smoke（C1 集成兜底）**：(1) `[embedding]` 配置选择经工厂生效 + dim 协商；(2) embedding 缓存命中 / 失效可观测；(3) 远程 provider 契约级构造 / 解析（fixture）；(4) health 远程探针 opt-in 全 PASS。

## 7. 阶段级风险

- **R1（中）dim 协商与既有索引不一致**：切换 provider 后 `dim()` 与已建向量索引 / `VectorIndexConfig.dim` 不一致会破坏检索。
  - **缓解**：task-22.1 dim 协商校验 — provider `dim()` 与配置 / 索引 dim 不一致即 `EmbeddingError::DimMismatch`（不静默截断 / pad）；测试覆盖 mismatch 报错路径。缺省 dim 仍 384（`DEFAULT_DIM`），既有索引不受影响。
- **R2（高）远程 provider 真实联调需密钥 / 网络，CI 不验证**：OpenAI / Cohere 真实可达性 + 真实召回质量需密钥 + 网络。
  - **缓解**：task-22.3 做 feature-gated 骨架 + 契约级确定性测试（fixture 断言请求构造 / 响应解析 / 错误路径，不打真实网络），CI 可验证骨架正确性；真实网络命中 / 密钥 / 召回质量 🔴 如实 defer（ADR-013 不伪造）。**stop-condition**：远程 provider 无密钥 / 网络受阻 → 契约测试跑通即视为骨架达标，真实联调如实记录 defer，不标 `[x]` 真实命中、不伪造响应，继续 closeout。
- **R3（中）远程 opt-in 误触发违本地优先**：远程 provider 不应在默认构建 / 缺省配置下被激活。
  - **缓解**：远程 provider feature-gated（默认构建不编入，0 网络 dep）+ 配置显式 opt-in（承 ADR-004 + 既有 `RemoteProviderConfig.Enabled`）；缺省 `provider="deterministic"`；密钥从环境 / 配置读，不入库不入日志（承 PRD §Constraints 安全基线）。ADR-027 D-红线明确本地优先。
- **R4（低）SQLite 缓存持久化与既有 schema 冲突**：缓存表与既有 metadata.sqlite / migrations 共存。
  - **缓解**：task-22.2 缓存用独立表 / 独立文件（承 ADR-002 分层），add-only schema；feature / 配置 opt-in，缺省内存缓存不落盘。

## 8. Definition of Done

- 4 task spec（22.1-22.4）顶部 `**Status**: Done`
- §6 阶段级 AC1-6 全 `[x]`
- 端到端 smoke 4 step 全 PASS
- **ADR**：ADR-027 `Proposed → Accepted`（或据实测记录维持 + 文档化）
- **adapter**：§Phase 索引 Phase 22 `Draft → Done` + `Tasks 0 → 4`；§ADR 索引 ADR-027；§BDD 追加 phase-22 feature 行
- **release**：`docs/releases/v0.15.0-{evidence,artifacts}.md` + `RELEASE_NOTES.md` v0.15 段 + README v0.15 段
- **defer 记录**：远程 provider 真实联调 / 密钥 / 召回质量 `[SPEC-DEFER:phase-future.embedding-provider-remote]` + health 远程探针真实命中 `[SPEC-DEFER:phase-future.embed-remote-probe]` 的 stop-condition 状态如实记录（ADR-013）
