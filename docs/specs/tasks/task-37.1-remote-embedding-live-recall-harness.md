# Task `37.1`: `remote-embedding-live-recall-harness — 新增 core/tests/remote_embedding_recall.rs（#![cfg(feature = "embedding-remote")]，env-gated CONTEXTFORGE_REMOTE_API_KEY via factory.rs env 路径），首次以「real remote embedding 语义召回 vs deterministic（model-free）基线」方法学在 live remote 端点上量真实召回：作者手工标注语义集（15 case / 16 doc，覆盖 英文复述 / 代码概念 / CJK / 跨语言，含故意近义干扰）→ 同一标注集 + 同一 BruteForceVectorBackend 精确余弦路径上比较 real 模型 vs deterministic 基线 → recall@1 / recall@3 = mean(命中 / N) → 先 eprintln 真实测得值再 assert（floor r3>=0.70 且 remote recall@1 > deterministic recall@1）；CONTEXTFORGE_REMOTE_API_KEY 未设时 eprintln skip notice + 干净 return（honest-defer，CI 无密钥时 skip 不 fail，ADR-013，api_key 永不记录）；另有非网络 well-formed 守护测试（doc id 唯一 / 每 relevant id 存在 / case 数>=12）无 key 也总跑；0 新 dep / 0 schema migration / 0 默认构建变更（embedding-remote opt-in，ADR-004/008）`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 37 (embedding-provider-remote-live)
**Dependencies**: task-22.3（`RemoteEmbeddingProvider` 契约层已落地：`build_request_body`（`remote_provider.rs:60-69` OpenAI/Cohere 风格请求体纯函数构造，`dim==0` 省略 `dimensions` 字段）/ `parse_response`（`remote_provider.rs:74-102` `{"data":[{"embedding":[...]}]}` → 有序向量纯函数解析，malformed/empty/missing 显式 `EmbeddingError` 不 panic）/ `embed`（`remote_provider.rs:104-123` ureq POST + Bearer header + parse；feature-gated `embedding-remote`）/ `Debug` impl（`remote_provider.rs:47-56` 只打 endpoint/model，**永不**打 api_key）——真实 live 端点端到端联调 + 真实召回在 task-22.3 §3 范围外记为 `[SPEC-DEFER:phase-future.embedding-provider-remote]`（「Real network reachability / API keys / real recall quality are deferred — CI has no credentials」，`remote_provider.rs:8-9`），本 task 兑现其真实联调 + 真实召回方法学层）/ task-22.1（`select_provider` 工厂 + dim 协商：`factory.rs:49-74` `"remote"` 分支自 env 读 `CONTEXTFORGE_REMOTE_ENDPOINT/_MODEL/_PROVIDER/_API_KEY`、api_key 永不记录，`factory.rs:52` 注释 "config plumbing is a follow-up"——config-bridge 由 task-37.2 兑现；`negotiate_dim`（`factory.rs:88-96`）dim 协商 `DimMismatch` 不静默截断）/ task-19.1（`DeterministicEmbeddingProvider`，`deterministic.rs` Sha256-seeded splitmix64 → 单位向量，model-free 默认可用、**无语义结构**——本 task 用作 model-free 对照基线，量 real 模型相对它买到的语义召回 delta）/ task-19.3（`BruteForceVectorBackend`，`brute_force.rs` 精确 O(n) cosine searcher，0-dep / 默认可用——本 task 两个 provider 共用同一精确余弦 ground-truth 路径，apples-to-apples）/ ADR-042 D1-D2（remote-embedding-live-recall harness 方法学 + 真实测得召回数；Status Proposed，ratify @ task-37.3）/ ADR-027 D1-D5（embedding-provider-abstraction：`select_provider` 工厂 + dim 协商；本 task + task-37.2 经 add-only Phase-37 Amendment 标记 `embedding-provider-remote` 真实联调 + 真实召回兑现，不溯改 D-body）/ ADR-013（禁伪造红线——召回数真实跑出后回填，无 key 时 honest-defer skip 不伪造通过 / 不预填召回数 / api_key 永不记录）/ ADR-004（local-first-privacy-baseline，默认行为 + 默认构建 0 新 network dep / 0 network；api_key env-only 永不进 config.toml）/ ADR-008（dep add-only，本 task = 0 新 dep，`ureq` 自 task-22.3 已 optional）/ ADR-012（main-agent-governance-autonomy）/ ADR-014 D1-D4（第二十八次激活）

## 1. Background

`core/src/embedding/remote_provider.rs` 的 `RemoteEmbeddingProvider` 自 Phase 22 起其**契约层 / 纯函数构造已完整实现**：`build_request_body`（OpenAI/Cohere 风格请求体）/ `parse_response`（响应 → 有序向量）/ `embed`（ureq POST live HTTP）/ `Debug`（永不泄 api_key）。但**真实 live 端点端到端联调 + 真实语义召回**在 task-22.3 §3 据实记为 `[SPEC-DEFER:phase-future.embedding-provider-remote]`——`remote_provider.rs:8-9` 明记「Real network reachability / API keys / real recall quality are deferred — CI has no credentials」，仓库内此前**只有** deterministic（model-free）缺省 provider + feature-gated fastembed + remote 的纯函数契约测试（`remote_provider.rs:142-202` 对 fixture 断言 `build_request_body`/`parse_response`，**从不**触网），**从未**对 live remote 端点测过真实语义召回。Phase 37 关闭这条 gap，本 task 是第一步——以一个「real remote embedding 语义召回 vs deterministic（model-free）基线」方法学 harness 在 live remote 端点上量真实召回：

- **B1 方法学 = real remote embedding 语义召回 vs deterministic（model-free）基线（同一标注集 + 同一精确余弦路径）**：在**同一**作者手工标注语义集上，用 `select_provider("remote", DIM)` 经 live 端点取 real 嵌入、`select_provider("deterministic", DIM)` 取 model-free 基线嵌入，二者各把语料索引进**同一** `BruteForceVectorBackend`（精确 O(n) cosine），对每个标注 query 取 top-k 看其单一 relevant doc 是否命中，`recall@1 = mean(top-1 命中)`、`recall@3 = mean(top-3 命中)`。delta 即「真实嵌入相对 model-free hash 向量买到的语义」——deterministic 向量**无语义结构**（hash 派生）故近随机，real 模型应把语义对排进 top-k。区别于 task-29.2 / task-36.1 的 model-free 可复现「ANN vs 精确 KNN」度量（量索引近似精度）——本 task 量的是 **embedding 语义质量**（real 模型 vs 无语义基线），二者维度互补。
- **B2 作者手工标注语义集（诚实范围，非大基准）**：作者手工标注 15 个 case / 16 个文档，覆盖 英文复述 / 代码概念 / CJK / 跨语言四类；语料故意埋近义干扰项（`config_save` vs `config_load`、`bm25` vs `hybrid`、`cjk_index` vs `cjk_vector`），使 top-k 命中**非词面 trivial**——必须靠语义而非共享词。每个 query 是其单一 relevant doc 的复述 / 跨语言 / 概念重述。这是**小型手工标注集**（诚实范围），证明真实模型把明显语义对排在近义干扰之上，**不是**大型标准基准（ADR-013）。
- **B3 honest-defer 守门（CONTEXTFORGE_REMOTE_API_KEY 未设 → skip 干净退出，不 fail）**：live 召回测试**第一步**读 `CONTEXTFORGE_REMOTE_API_KEY`——未设（CI 无密钥）则 `eprintln!` 一条 skip notice（说明需 live remote 端点 + 设 `CONTEXTFORGE_REMOTE_ENDPOINT/_MODEL/_API_KEY`）+ `return`（测试**干净通过**，**不** fail）；factory 另自 env 读 `CONTEXTFORGE_REMOTE_ENDPOINT/_MODEL/_PROVIDER`，api_key **永不**记录。这样无密钥的本地 / CI 环境**干净 skip 而非红**（honest-defer，ADR-013）；设密钥的 dev-box 才进真实召回度量。
- **B4 floor 是 guard、真实值是报告（ADR-013 禁伪造）**：harness 先 `eprintln!` **真实测得**的 deterministic vs remote recall@1/@3 + delta，再 `assert!`：floor `r3 >= 0.70`（不退化下界）**且** `remote recall@1 > deterministic recall@1`（real 嵌入必须胜过 model-free 基线，否则真实嵌入买不到东西）。floor 是守门；真实数字在设密钥的真实 run 跑出后回填 §10 + v0.30.0 evidence（Draft 阶段 **待回填**，绝不预填、绝不伪造，ADR-013）。
- **B5 de-risk 已由主 agent 真实验证（真实非合成）**：SiliconFlow（`https://api.siliconflow.cn/v1/embeddings`，OpenAI-compatible）+ 模型 `Qwen/Qwen3-Embedding-8B` 端到端 round-trip 已跑通——native dim=4096，OpenAI 风格 `dimensions` 参数被接受并生效（MRL，本 phase 请求 1024），CJK 输入正常；Rust `RemoteEmbeddingProvider` → ureq → `parse_response` 路径在 Windows MSVC `--features embedding-remote` 真实编译并跑通、返回 1024 维向量（主 agent 实证）。本 task 把这条 de-risked 路径制度化为可复现的 recall harness。

经核 `ureq` 自 task-22.3 已 optional（`embedding-remote` feature），`BruteForceVectorBackend` / `DeterministicEmbeddingProvider` 默认可用——本 task **0 新 dep** + **0 schema migration**（纯新增 test 文件，无表）+ **0 默认构建变更**（`#![cfg(feature = "embedding-remote")]`，默认 build 0-network-dep / 0-network 不变，ADR-004/008）。

## 2. Goal

(1) **新增 `core/tests/remote_embedding_recall.rs`**：`#![cfg(feature = "embedding-remote")]`（默认构建不编译此文件，0-network-dep / 0-network 不变）；env-gated 经 `CONTEXTFORGE_REMOTE_API_KEY`（factory 另读 `CONTEXTFORGE_REMOTE_ENDPOINT/_MODEL/_PROVIDER`，api_key 永不记录）。(2) **honest-defer 守门**：live 召回测试第一步 `std::env::var("CONTEXTFORGE_REMOTE_API_KEY")`——未设 → `eprintln!` skip notice（说明需 live remote 端点 + 设密钥）+ `return`（测试**干净通过不 fail**，无密钥的本地 / CI 干净 skip，ADR-013）。(3) **作者手工标注语义集**：15 个 case / 16 个文档，覆盖 英文复述 / 代码概念 / CJK / 跨语言，含故意近义干扰项（`config_save`/`config_load`、`bm25`/`hybrid`、`cjk_index`/`cjk_vector`），使 top-k 命中非词面 trivial。(4) **同标注集 real vs deterministic 对照 + recall@k**：同一标注集 + 同一 `BruteForceVectorBackend` 精确余弦路径上，`select_provider("remote", DIM)`（live 嵌入）与 `select_provider("deterministic", DIM)`（model-free 基线）各索引语料；每个 query 取 top-k 看其单一 relevant 是否命中；`recall@1 = mean(top-1 命中)`、`recall@3 = mean(top-3 命中)`。(5) **真实值 eprintln + floor/delta guard**：先 `eprintln!` 真实测得 deterministic vs remote recall@1/@3 + delta，再 `assert!`（`r3 >= 0.70` 且 `remote recall@1 > deterministic recall@1`）——真实值待回填（设密钥真实 run 跑出后回填 §10 + v0.30.0 evidence，绝不预填，ADR-013）。(6) **非网络 well-formed 守护测试**：断言 doc id 唯一 / 每个 relevant id 存在于语料 / case 数 >= 12——**无 key 也总是跑**（即使 live 测试 honest-defer 也守住标注集逻辑地基）。

pass bar：feature `embedding-remote` 下 `core/tests/remote_embedding_recall.rs` 编译通过；**无密钥时**（本地 / 本 CI run 无 credentials）`CONTEXTFORGE_REMOTE_API_KEY` 未设 → eprintln skip notice + 干净通过 exit 0（**不** fail，honest-defer）；**设密钥 + live remote 端点时**（dev-box）经同标注集 real vs deterministic 对照量真实 `recall@1/@3`、eprintln 真实值 + delta、断言 `r3 >= 0.70` 且 `remote@1 > det@1`（真实数字 设密钥真实 run 跑出后回填，绝不预填）；well-formed 守护测试**无 key 即可**跑（doc id 唯一 / relevant 存在 / case>=12，不触网）；api_key **永不**记录（factory `Debug` impl 只打 endpoint/model）；0 新 dep（ADR-008）+ 0 schema migration + 0 默认构建变更（默认 0-network-dep / 0-network，ADR-004）；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- **新增 `core/tests/remote_embedding_recall.rs`**（`#![cfg(feature = "embedding-remote")]`）：
  - **作者手工标注语义集**：`docs() -> Vec<(&'static str, &'static str)>`（16 个 `(id, text)` 文档，含近义干扰 `config_save`/`config_load`、`bm25`/`hybrid`、`cjk_index`/`cjk_vector`、CJK 文本）+ `cases() -> Vec<Case>`（15 个 `Case { query, relevant, category }`，category ∈ {`en-paraphrase`, `code-concept`, `cjk`, `cross-lingual`}），每个 query 是其单一 relevant doc 的复述 / 跨语言 / 概念重述，命中靠语义非词面。
  - **honest-defer 守门**：live 召回测试第一步 `if std::env::var("CONTEXTFORGE_REMOTE_API_KEY").is_err() { eprintln!("SKIP ...: CONTEXTFORGE_REMOTE_API_KEY unset (honest-defer, ADR-013; set CONTEXTFORGE_REMOTE_ENDPOINT/_MODEL/_API_KEY with a real OpenAI-compatible embedding endpoint to run)"); return; }`（测试干净通过、不 fail）；factory 另自 env 读 endpoint/model/provider；api_key 经 factory 读后**永不**记录。
  - **同标注集 real vs deterministic 对照 + recall@k 度量**：`select_provider("deterministic", DIM)` 与 `select_provider("remote", DIM)`（`DIM=1024`，Qwen3-Embedding-8B native 4096 经 OpenAI 风格 `dimensions` 参数 MRL 取 1024，两 provider 同 dim apples-to-apples）；各把 `docs()` 经 `provider.embed(...)` 嵌入后索引进**同一** `BruteForceVectorBackend`（精确 cosine ground truth）；对每个 `Case` 取 `backend.search(q, 3, None)` top-3，`recall@1 = mean(top-1 == relevant)`、`recall@3 = mean(relevant ∈ top-3)`。
  - **真实值 eprintln + floor/delta guard**：先 `eprintln!("REMOTE-EMBED semantic recall over {n} labeled cases (dim={DIM}) | deterministic: recall@1={d1} recall@3={d3} | remote: recall@1={r1} recall@3={r3} | delta@1={..} delta@3={..}")`；再 `assert!(r3 >= 0.70, ...)`（real 模型把明显语义对排进 top-3 的不退化 floor）+ `assert!(r1 > d1, ...)`（real 嵌入必须胜过 model-free 基线）——真实值由 `-- --nocapture` 可见、待设密钥真实 run 回填（绝不预填，ADR-013）。
  - **非网络 well-formed 守护**：`test_labeled_set_well_formed`——断言 doc id 唯一（sort+dedup 后 len 不变）/ 每个 `Case.relevant` 存在于 `docs()` / `cases().len() >= 12`，**无 key 也总跑**、不触网，守标注集逻辑地基。
- **TEST-37.1.1**（live semantic recall harness，env-gated）：设 `CONTEXTFORGE_REMOTE_API_KEY` + live remote 端点时真实 `recall@3 >= 0.70` 且 `remote recall@1 > deterministic recall@1`（无 key honest-defer 干净 skip）。
- **TEST-37.1.2**（标注集 well-formed，**无 key**）：断言 doc id 唯一 / 每 relevant 存在 / case 数 >= 12，**不触网** 即可跑（标注集逻辑地基守线）。
- **TEST-37.1.3**（= LAST，D2 lint）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **Go `[remote]` 段 Model 字段 add-only + `setRemoteEnv` env-bridge** [SPEC-OWNER:task-37.2-remote-embedding-config-bridge]——本 task 仅交付 harness（消费既有 factory env 路径）；Go config → spawned core 的 `CONTEXTFORGE_REMOTE_*` env 桥接（兑现 `factory.rs:52` "config plumbing is a follow-up"）由 task-37.2 落地。
- **真实测得召回数字 / 真实 run 证据（v0.30.0 evidence）回填** [SPEC-OWNER:task-37.3-closeout-v0.30.0]（设密钥真实 run 跑出后回填，ADR-013 不预填）；smoke v27 step + `TestTask373` + release docs 亦由 task-37.3。
- **CI 每次 run 跑 live remote 召回（service container）** [SPEC-DEFER:phase-future.embedding-remote-ci-credential]——与 qdrant 不同：qdrant 有免费 OSS service container（task-36.2 经 `qdrant-recall` job 兑现），remote embedding 是**付费外部 API、无免费 service container**，CI 据实 honest-defer（无 credentials），真实召回由设密钥的已认证 run 实测——这一诚实差异由 ADR-042 D2 记载。
- **大语料 / 大型标准基准上的 embedding 语义质量度量** [SPEC-DEFER:phase-future.embedding-large-corpus-recall]——本 task 是小型作者手工标注集（证明 real 模型把明显语义对排在近义干扰之上），大语料语义质量 / 标准基准诚实延后（harness header `remote_embedding_recall.rs:18-19` 据实记）。
- **多 remote provider（cohere / 其它 OpenAI-compatible）/ reranker 端点 live 召回矩阵** [SPEC-DEFER:phase-future.embedding-multi-provider-live]——本 task 聚焦 OpenAI-compatible embedding 端点（de-risk 用 SiliconFlow Qwen3-Embedding-8B）；其余 provider / reranker 端点的 live 召回矩阵诚实延后。
- **改 `core/src/embedding/remote_provider.rs` / `factory.rs` / `deterministic.rs` / `brute_force.rs` 本体** [SPEC-OWNER:task-22.3-remote-provider]——本 task harness 是消费方，复用既有 `select_provider`（task-22.1 freeze）/ `RemoteEmbeddingProvider`（task-22.3 freeze）/ `DeterministicEmbeddingProvider`（task-19.1）/ `BruteForceVectorBackend`（task-19.3），不重写 provider / backend。

## 4. Actors

- 主 agent（ADR-012 自治）：实施 harness + 在设密钥的 dev-box（SiliconFlow Qwen3-Embedding-8B）跑真实召回回填证据。
- `core/tests/remote_embedding_recall.rs`（新增 integration test，`#![cfg(feature = "embedding-remote")]`）：作者手工标注集 → real remote vs deterministic 语义 recall@k + honest-defer skip + 非网络 well-formed 守护。
- `core/src/embedding/factory.rs::select_provider`（task-22.1）：harness 经其 `"remote"` 分支（`:49-74` 自 env 读 endpoint/model/provider/api_key，api_key 永不记录）与 `"deterministic"` 分支（`:32-36`）取两 provider；`negotiate_dim`（`:88-96`）dim 协商。
- `core/src/embedding/remote_provider.rs::RemoteEmbeddingProvider`（task-22.3）：harness 真实驱动 `embed`（`:104-123` ureq POST live HTTP + Bearer + `parse_response`）；`build_request_body`（`:60-69`）/ `parse_response`（`:74-102`）纯函数构造；`Debug`（`:47-56`）永不泄 api_key。
- `core/src/embedding/deterministic.rs::DeterministicEmbeddingProvider`（task-19.1）：model-free 对照基线（Sha256-seeded splitmix64 → 单位向量，**无语义结构** `:7-8`），与 real 模型同 dim 同精确余弦路径对照。
- `core/src/retriever/vector/brute_force.rs::BruteForceVectorBackend`（task-19.3）：精确 O(n) cosine searcher，两 provider 共用作 ground-truth top-k（`search` `brute_force.rs:84-118`，cosine 降序 + chunk_id 破并列确定性序）。
- `CONTEXTFORGE_REMOTE_API_KEY` / `CONTEXTFORGE_REMOTE_ENDPOINT/_MODEL/_PROVIDER`（env）：env gate 来源；API KEY **永不**进 config.toml、永不记录（PRD 安全基线 / ADR-004）；无 key → honest-defer skip（本 task 不伪造）；真实召回经设密钥的 dev-box 跑出。
- 下游 task-37.2 / task-37.3：37.2 加 Go `[remote]` Model add-only + `setRemoteEnv` env-bridge（兑现 `factory.rs:52` config plumbing follow-up）；37.3 closeout 据真实召回数 ratify ADR-042 + add-only ADR-027 Phase-37 Amendment + smoke v27 + release docs。

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/embedding/factory.rs:27-83`（`select_provider`：`""`/`"deterministic"` 分支 `:32-36`、`"remote"` 分支 `:49-74` 自 env 读 `CONTEXTFORGE_REMOTE_ENDPOINT/_MODEL/_PROVIDER/_API_KEY`、`factory.rs:52` "config plumbing is a follow-up" 注释、api_key `:59` 读后永不记录、`negotiate_dim` `:81`）+ `:88-96`（`negotiate_dim` dim 协商 `DimMismatch` 不静默截断）
- `core/src/embedding/remote_provider.rs:8-9`（`[SPEC-DEFER:phase-future.embedding-provider-remote]` 出处：「Real network reachability / API keys / real recall quality are deferred — CI has no credentials」）+ `:47-56`（`Debug` impl 只打 endpoint/model、**永不** api_key——安全基线）+ `:60-69`（`build_request_body` `dim==0` 省略 `dimensions`、非零发 `dimensions`——MRL 取 1024 来源）+ `:74-102`（`parse_response` `data[].embedding` → 有序向量、malformed/empty/missing 显式 `EmbeddingError`）+ `:104-123`（`embed` ureq POST live HTTP + Bearer header + parse）
- `core/src/embedding/deterministic.rs:7-8`（**无语义结构** caveat：「these vectors carry no semantic structure — NOT to measure real recall」——本 task model-free 基线的诚实定位）+ `:44-65`（`embed_one` Sha256-seeded splitmix64 → 单位向量，可复现 model-free）+ `:16`（`DEFAULT_DIM = 384`）
- `core/src/retriever/vector/brute_force.rs:84-118`（`BruteForceVectorBackend::search` 精确 O(n) cosine：单位归一化 → dot → cosine 降序 + chunk_id 破并列确定性序——ground-truth top-k 来源）+ `:54-82`（`open` clear + `index_batch` append）
- `core/src/retriever/vector/types.rs`（`ChunkId(pub String)` / `VectorChunk { chunk_id, embedding: Vec<f32>, metadata: Option<serde_json::Value> }` / `VectorIndexConfig { dim, metric, persistence_path, collection_id }` / `VectorMetric::Cosine` / `VectorHit { chunk_id, score, metadata }`）+ `core/src/retriever/vector/traits.rs`（`VectorIndexer::open` / `index_batch`、`VectorSearcher::search`）
- `core/src/embedding/traits.rs:13-20`（`EmbeddingProvider::embed` / `dim` / `name`，object-safe `Arc<dyn EmbeddingProvider>`）+ `:26-42`（`EmbeddingError` `#[non_exhaustive]`，`DimMismatch` / `Backend` / `Other`）
- `core/Cargo.toml`（`embedding-remote` feature → `dep:ureq` 自 task-22.3 已 optional——本 task 0 新 dep；`DeterministicEmbeddingProvider` / `BruteForceVectorBackend` 默认可用，无 feature gate）
- `docs/decisions/adr-042-embedding-provider-remote-live.md`（D1 remote-embedding-live-recall harness 方法学 / D2 真实测得召回数 + CI honest-defer 因付费外部 API 无免费 service container（与 qdrant 诚实差异）/ D3 remote-embedding-config-bridge / D4 默认 0-network + 0 新 dep baseline；Status Proposed，ratify @ task-37.3）+ `docs/decisions/adr-027-embedding-provider-abstraction.md`（`select_provider` 工厂 + dim 协商；本 task + task-37.2 经 add-only Phase-37 Amendment 标记 `embedding-provider-remote` 真实联调 + 真实召回兑现，不溯改 D-body，ADR-014 D5）+ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造：无 key honest-defer skip 不 fail / 召回数真实跑出后回填不预填 / api_key 永不记录）+ `docs/specs/tasks/task-22.3-remote-embedding-provider.md` §3 范围外（`[SPEC-DEFER:phase-future.embedding-provider-remote]` 出处）

### 5.2 关键设计 — real remote 语义召回 vs deterministic 基线 harness + honest-defer + 作者标注集（0 dep / 0 migration / 默认构建不变）

- **B1 方法学 real vs model-free 基线（同标注集 + 同精确余弦路径）**：在**同一**作者手工标注集上量 real remote 嵌入相对 deterministic（model-free）基线的语义 `recall@1/@3`——两 provider 同 `DIM` 同 `BruteForceVectorBackend`（`brute_force.rs:84-118` cosine 降序 + chunk_id 破并列确定性序）精确余弦路径，apples-to-apples。deterministic（`deterministic.rs:7-8` 无语义结构）近随机给基线；real 模型应把语义对排进 top-k；`recall@1 = mean(top-1 命中)` / `recall@3 = mean(relevant ∈ top-3)` over N=15 case。
  ```rust
  #![cfg(feature = "embedding-remote")]
  const DIM: usize = 1024; // Qwen3-Embedding-8B native 4096 → OpenAI dimensions 参数 MRL 取 1024
  struct Case { query: &'static str, relevant: &'static str, category: &'static str }
  // 同一标注集索引进同一 BruteForceVectorBackend；两 provider 仅嵌入来源不同
  fn index_with(provider: &Arc<dyn EmbeddingProvider>) -> BruteForceVectorBackend { /* embed docs() → index_batch */ }
  fn measure(label: &str, provider: Arc<dyn EmbeddingProvider>) -> (f32, f32) { /* recall@1, recall@3 over cases() */ }
  ```
- **B2 honest-defer 守门（API_KEY 未设 → skip 不 fail，ADR-013）**：live 召回测试第一步读 `CONTEXTFORGE_REMOTE_API_KEY`——未设 → `eprintln!` skip notice + `return`（测试**干净通过**）：
  ```rust
  if std::env::var("CONTEXTFORGE_REMOTE_API_KEY").is_err() {
      eprintln!("SKIP test_remote_embedding_semantic_recall: CONTEXTFORGE_REMOTE_API_KEY unset \
                 (honest-defer, ADR-013; set CONTEXTFORGE_REMOTE_ENDPOINT/_MODEL/_API_KEY with a \
                 real OpenAI-compatible embedding endpoint to run)");
      return; // 干净 skip，不 fail —— 本地 / 无 credentials 的 CI run 不变红
  }
  ```
  无密钥的本地 / CI run（remote 是付费外部 API 无免费 service container）走此分支干净 skip（**不** fail），证明 wiring 成立而不伪造召回；设密钥才进真实度量。factory 另自 env 读 endpoint/model/provider；api_key 经 `factory.rs:59` 读后由 `RemoteEmbeddingProvider`（`Debug` `:47-56` 只打 endpoint/model）**永不**记录。
- **B3 同标注集 real vs deterministic + recall@k**：`docs()`（16 doc 含近义干扰）经**同一**标注集分别由 `select_provider("deterministic", DIM)` 与 `select_provider("remote", DIM)` 嵌入后索引进各自 `BruteForceVectorBackend`；`cases()`（15 query）各取 top-3 看其单一 `relevant` 是否命中；`recall@1`/`recall@3` 双值。近义干扰（`config_save`/`config_load`、`bm25`/`hybrid`、`cjk_index`/`cjk_vector`）使命中靠语义非词面（`brute_force` cosine 与 real 模型一致）。
- **B4 floor 是 guard、真实值是报告（ADR-013）**：先 `eprintln!` 真实测得 deterministic vs remote recall@1/@3 + delta（`-- --nocapture` 可见），再 `assert!(r3 >= 0.70)`（real 模型把明显语义对排进 top-3 的不退化 floor）+ `assert!(r1 > d1)`（real 嵌入必须胜过 model-free 基线，否则真实嵌入买不到东西）。真实数字在设密钥真实 run 跑出后回填 §10 + v0.30.0 evidence——Draft 阶段 **待回填**，绝不预填、绝不伪造（ADR-013）。
- **B5 测试矩阵据实**：TEST-37.1.1 是 env-gated live test（设密钥量真实语义召回 + delta；无 key honest-defer 干净 skip 不 fail）；TEST-37.1.2 是**纯非网络标注集 well-formed** 守护（**不触网**，断言 doc id 唯一 / 每 relevant 存在 / case 数 >= 12），即使无 key 也跑、也绿，守住标注集逻辑地基（ADR-013）。

### 5.3 不变量

- **默认构建 0-network-dep / 0-network 不变（ADR-004/008）**：`core/tests/remote_embedding_recall.rs` `#![cfg(feature = "embedding-remote")]`——默认 `cargo test --workspace` **不编译** 此文件、不引入 `ureq`、不连网；`embedding-remote` opt-in，默认行为 / 默认构建 dep 集不变。
- **0 新代码依赖（ADR-008）**：`ureq` 自 task-22.3 已 optional、`DeterministicEmbeddingProvider` / `BruteForceVectorBackend` 默认可用——本 task **0 新 Cargo 依赖**、无 `Cargo.lock` 变化。
- **0 schema migration**：纯新增 test 文件，无表 / 无持久化结构变更，不加列、不 `ALTER`、不新增编号 migration。
- **honest-defer 不伪造（ADR-013）**：`CONTEXTFORGE_REMOTE_API_KEY` 未设 → eprintln skip + `return`（测试干净通过、**不** fail、**不**输出召回数、**不**当成功召回）；真实召回仅在设密钥 + live remote 端点上产生且经 §10 回填。
- **api_key 永不记录 / 永不进 config（ADR-004 / PRD 安全基线）**：api_key 仅由用户设 env（`CONTEXTFORGE_REMOTE_API_KEY`），经 `factory.rs:59` 读后由 `RemoteEmbeddingProvider` 持有；`Debug` impl（`remote_provider.rs:47-56`）只打 endpoint/model，eprintln 的 skip notice / 召回报告**绝不**含 api_key；harness 不写任何密钥到磁盘 / config。
- **不改 provider / backend 本体**：`factory.rs` / `remote_provider.rs` / `deterministic.rs` / `brute_force.rs` 签名与 `select_provider`/`embed`/`search` 不动——harness 是消费方，复用既有 API（task-22.1 / task-22.3 / task-19.1 / task-19.3 freeze）。
- **小型手工标注集诚实范围（ADR-013）**：floor 是不退化 regression guard、非质量上界；recall@3=1.0 一类结果证明 real 模型把明显语义对排在近义干扰之上、**非**大基准质量断言；大语料语义质量续 `[SPEC-DEFER:phase-future.embedding-large-corpus-recall]`。
- **召回数真实跑出后回填**：真实 `recall@1/@3` + delta + run 环境（远程端点 / 模型 / dim）绝不预填——设密钥真实 run 跑出后回填（**待回填**，ADR-013）。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [x] **AC1**（live semantic recall harness env-gated + honest-defer skip 🔴 live / 🟢 wiring）: 新增 `core/tests/remote_embedding_recall.rs`（`#![cfg(feature = "embedding-remote")]`），env-gated 经 `CONTEXTFORGE_REMOTE_API_KEY`（factory 另读 `CONTEXTFORGE_REMOTE_ENDPOINT/_MODEL/_PROVIDER`，api_key 永不记录）；live 召回测试第一步读 `CONTEXTFORGE_REMOTE_API_KEY`——未设 → eprintln skip notice + `return`（测试**干净通过不 fail**）；设密钥 + live remote 端点时同一作者手工标注集 + 同一 `BruteForceVectorBackend` 精确余弦路径上 `select_provider("remote", DIM)` 与 `select_provider("deterministic", DIM)` 对照，每个 query 取 top-k，`recall@1 = mean(top-1 命中)` / `recall@3 = mean(relevant ∈ top-3)`，先 eprintln 真实测得值 + delta 再 `assert!(r3 >= 0.70)` 且 `assert!(remote recall@1 > deterministic recall@1)`（真实数字 **真实跑出后回填**，无 key 时 honest-defer skip，绝不预填，ADR-013）；**0 新 dep + 0 schema migration + 0 默认构建变更**（默认 0-network-dep / 0-network） — verified by **TEST-37.1.1**（env-gated：设密钥 + live remote 端点时真实 `recall@3 >= 0.70` 且 `remote@1 > det@1`；无 key honest-defer 干净 skip 不 fail）
- [x] **AC2**（非网络标注集 well-formed 守护，无 key 🟢）: doc id 唯一 / 每个 `Case.relevant` 存在于语料 / `cases().len() >= 12`——**不触网** 即可跑、即绿，守住标注集逻辑地基（ADR-013） — verified by **TEST-37.1.2**（无 key，断言 doc id 唯一 + 每 relevant 存在 + case 数 >= 12）
- [x] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-37.1.3**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-37.1.1 | 🔴 live / 🟢 wiring：env-gated（`CONTEXTFORGE_REMOTE_API_KEY`）live semantic recall harness——设密钥 + live remote 端点时同一作者标注集 + 同一 `BruteForceVectorBackend` 精确余弦路径 `select_provider("remote")` vs `select_provider("deterministic")` 对照，每 query 取 top-k，`recall@1`/`recall@3` + eprintln 真实值 + delta，`assert!(r3 >= 0.70)` 且 `assert!(remote@1 > det@1)`（真实数字 真实跑出后回填，不预填）；`CONTEXTFORGE_REMOTE_API_KEY` 未设 → eprintln skip + return 干净通过（**不** fail，honest-defer ADR-013；api_key 永不记录） | `core/tests/remote_embedding_recall.rs` | Done |
| TEST-37.1.2 | 🟢 非网络标注集 well-formed 守护（**无 key**）：doc id 唯一（sort+dedup len 不变）+ 每个 `Case.relevant` 存在于 `docs()` + `cases().len() >= 12`——不触网 即可跑、即绿，守标注集逻辑地基（ADR-013） | `core/tests/remote_embedding_recall.rs` | Done |
| TEST-37.1.3 | ADR-014 D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（高）真实 live 语义召回需 live remote 端点 + API key，本 CI run 无密钥**（承 task-22.3 §8 R1）：CI 无 credentials；与 qdrant 不同，remote embedding 是**付费外部 API、无免费 service container**（qdrant 有免费 OSS service container，task-36.2 经 `qdrant-recall` job 兑现）。
  - **缓解**：harness 第一步读 `CONTEXTFORGE_REMOTE_API_KEY`——未设 → honest-defer eprintln skip + `return`（测试干净通过、**不** fail、不伪造召回，ADR-013）；真实召回经设密钥的 dev-box（SiliconFlow Qwen3-Embedding-8B）跑出后回填 §10 + v0.30.0 evidence。AC1 的 live 维度 honest-defer 时如实记，不强 ratify、不预填（ADR-013/ADR-014）；CI 据实 honest-defer 这一诚实差异由 ADR-042 D2 记载，CI 每次跑 → `[SPEC-DEFER:phase-future.embedding-remote-ci-credential]`。stop-condition：harness 在无 key 时若 **fail（变红）而非干净 skip** 则 AC1 不标 `[x]`。
- **R2（中）api_key 误入日志 / config / 报告**：若 skip notice / 召回 eprintln / 任何持久化误含 api_key，破 PRD 安全基线 / ADR-004。
  - **缓解**：api_key 仅由用户设 env、经 `factory.rs:59` 读后由 `RemoteEmbeddingProvider` 持有；`Debug` impl（`remote_provider.rs:47-56`）只打 endpoint/model；harness eprintln 只打 category/query/top-k/recall 数，**绝不**含 api_key；harness 不写任何密钥到磁盘 / config。stop-condition：任何输出 / 文件含 api_key 则 review 退回、AC 不标 `[x]`。
- **R3（中）dim 不一致致 `DimMismatch` 或两 provider 不可比**：Qwen3-Embedding-8B native 4096，需经 OpenAI 风格 `dimensions` 参数 MRL 取 `DIM=1024`，且 deterministic 基线须同 `DIM`，否则 `negotiate_dim`（`factory.rs:88-96`）报 `DimMismatch` 或两 provider 不可 apples-to-apples。
  - **缓解**：harness 用统一 `const DIM: usize = 1024`，两 provider 同 `select_provider(name, DIM)`；`build_request_body`（`remote_provider.rs:60-69`）非零 dim 发 `dimensions` 参数（de-risk 已证 SiliconFlow 接受并生效）；`measure` 入口 `assert_eq!(provider.dim(), DIM)`。stop-condition：dim mismatch 致召回不可比 / `DimMismatch` 报错则不标 `[x]`。
- **R4（低）floor 设过高致 flaky 红 / 设过低致无 guard 价值**：小型标注集语义召回随模型 / dim 浮动，floor 设过高会 flaky、过低无意义。
  - **缓解**：floor `r3 >= 0.70` 设为保守不退化下界 + `remote@1 > det@1` 相对 guard（real 嵌入必须胜过 model-free 基线）；真实测得值 + delta 经 eprintln 报告（floor 是地板、真实数才是结论，真实跑出后回填——绝不以 floor 充当真实值，ADR-013）。floor 调参 / per-category 多 floor 矩阵 `[SPEC-DEFER:phase-future.recall-floor-tuning-matrix]` 诚实延后。stop-condition：floor 误被当作「真实召回数」写入 evidence 则 review 退回。
- **R5（低）默认构建被 harness 污染**：harness 须不进默认构建（0-network-dep / 0-network 不变）。
  - **缓解**：`#![cfg(feature = "embedding-remote")]` 整文件 gate——默认 `cargo test --workspace` 不编译此文件、不引 `ureq`、不连网；0 新 dep（`ureq` 自 task-22.3 optional）。stop-condition：默认构建编译此文件 / 引入 network dep 则不标 `[x]`。

## 9. Verification Plan

```bash
# 0. 默认构建（无 embedding-remote feature）：harness 不编译、0 新 network dep、不退化（AC1 默认维度 + R5）
cargo test --workspace
cargo build --workspace

# 1. AC2 — 非网络标注集 well-formed 守护（无 key 即可跑、即绿；feature 开但不依赖网络 / 密钥）
cargo test -p contextforge-core --features embedding-remote --test remote_embedding_recall -- --nocapture
echo "exit=$?  # 期望 0：TEST-37.1.2 well-formed PASS；TEST-37.1.1 无 key → honest-defer 干净 skip 不 fail"

# 2. AC1 — 真实 live remote 端到端语义 recall@k（dev-box，需设密钥 + live OpenAI-compatible 端点）
#    指向真实 remote 端点（de-risk 用 SiliconFlow Qwen3-Embedding-8B）：
#    CONTEXTFORGE_REMOTE_ENDPOINT=https://api.siliconflow.cn/v1/embeddings \
#    CONTEXTFORGE_REMOTE_MODEL=Qwen/Qwen3-Embedding-8B \
#    CONTEXTFORGE_REMOTE_API_KEY=<your-key> \
#      cargo test -p contextforge-core --features embedding-remote --test remote_embedding_recall -- --nocapture
#    → API_KEY 已设 → real vs deterministic 同标注集对照 → recall@3 >= 0.70 且 remote@1 > det@1 + eprintln 真实测得值 + delta
#    真实数字 设密钥真实 run 真实跑出后回填 §10 + v0.30.0 evidence（绝不预填，ADR-013；api_key 永不记录）

# 3. clippy（feature 开 + 默认）
cargo clippy -p contextforge-core --features embedding-remote --tests -- -D warnings
cargo clippy --workspace --all-targets -- -D warnings

# 4. AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.remote-embedding-live-recall-harness-defer-note]：本 task 仅交付 `core/tests/remote_embedding_recall.rs`（`#![cfg(feature = "embedding-remote")]`，env-gated `CONTEXTFORGE_REMOTE_API_KEY`）的 real remote 语义召回 vs deterministic 基线方法学 harness——作者手工标注集 + honest-defer（`CONTEXTFORGE_REMOTE_API_KEY` 未设 → eprintln skip + return 干净通过不 fail，api_key 永不记录）。真实召回数仅在设密钥 + live remote 端点上产生（dev-box，SiliconFlow Qwen3-Embedding-8B），**真实跑出后回填** §10 + v0.30.0 evidence（绝不预填、绝不伪造，ADR-013）。Go `[remote]` Model add-only + `setRemoteEnv` env-bridge → [SPEC-OWNER:task-37.2-remote-embedding-config-bridge]；smoke v27 + release docs 回填 → [SPEC-OWNER:task-37.3-closeout-v0.30.0]；CI 每次跑 live remote 召回（remote 是付费外部 API、无免费 service container，与 qdrant 诚实差异，ADR-042 D2）→ [SPEC-DEFER:phase-future.embedding-remote-ci-credential]；大语料 / 大基准 embedding 语义质量 → [SPEC-DEFER:phase-future.embedding-large-corpus-recall]；多 provider / reranker live 召回矩阵 → [SPEC-DEFER:phase-future.embedding-multi-provider-live]；floor 调参矩阵 → [SPEC-DEFER:phase-future.recall-floor-tuning-matrix]。floor 是地板 guard、真实测得值才是结论（绝不以 floor 充真实值，ADR-013）。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done（本节真实实测数为主 agent 本机真实 run，SiliconFlow `Qwen/Qwen3-Embedding-8B` dim=1024，3 次 run；ADR-042 D2 ratify 在 task-37.3 closeout）

**§9 Verification 预期实证**（impl PR 真实回填；以下 live 召回数为主 agent 本机真实 run（SiliconFlow `https://api.siliconflow.cn/v1/embeddings` + `Qwen/Qwen3-Embedding-8B`，dim=1024），非预填、非合成，ADR-013）：
- **AC1 真实 live 召回**（主 agent 本机真实 run）：设 `CONTEXTFORGE_REMOTE_ENDPOINT/_MODEL/_API_KEY` 指向 SiliconFlow Qwen3-Embedding-8B → live semantic recall 测试 PASS，实测（3 次 run）`remote: recall@1=0.8667–0.9333 (13–14/15，跨 run 波动) recall@3=1.0000 (15/15，3/3 稳定)` vs `deterministic: recall@1=0.0000 recall@3=0.0667（稳定）` → `delta@3=+0.9333`。**recall@1 跨 run 波动**（remote 模型/服务非完全确定：同一确定性语料/查询，SiliconFlow 多次 run 得 0.8667 或 0.9333），**recall@3=1.0000 三次均稳定**；波动仅落在故意埋的硬近义干扰对（`config_save`↔`config_load`、`hybrid`↔`bm25`）的 top-1 让位上。harness 护栏（floor `r3>=0.70` + `remote@1 > det@1`）每次 run 均过。honest-defer 分支：未设 `CONTEXTFORGE_REMOTE_API_KEY` → live 测试干净 SKIP（eprintln "CONTEXTFORGE_REMOTE_API_KEY unset (honest-defer)" + return，**不 fail**）。
- **AC2 标注集 well-formed**（无 key 也跑）：well-formed 守护测试 PASS（doc id 唯一 / 每 relevant 存在 / case 数 >= 12），**不触网**。
- 默认构建不退化（impl PR 真实回填）：`cargo test --workspace`（harness `#![cfg(feature = "embedding-remote")]` 不进默认构建）；`cargo clippy -p contextforge-core --features embedding-remote --tests -- -D warnings` 0 warning（impl PR 真实回填）。
- AC3：D2 lint `--touched origin/master`（CI spec-lint 权威，impl PR 真实回填）。

**诚实判读（ADR-013，关键）**：实测 `remote recall@1=0.8667–0.9333（跨 run 波动）/ recall@3=1.0000（3/3 稳定）` vs `deterministic recall@1=0.0000 / recall@3=0.0667`——这是「real remote embedding（Qwen3-Embedding-8B）相对 model-free deterministic 基线在同一作者标注集 + 同一精确余弦路径上买到的真实语义召回」，真实关闭 ADR-027 的 `[SPEC-DEFER:phase-future.embedding-provider-remote]`「real recall quality deferred」。**为何 recall@3=1.0**：在小型作者手工标注集上，real 模型把明显语义 / 跨语言 / 概念重述对排在近义干扰之上——这是 **embedding 语义质量** 的真实证明，**非**大型标准基准的质量断言；大语料 / 标准基准语义质量续 `[SPEC-DEFER:phase-future.embedding-large-corpus-recall]`（不夸大为「已基准测过」，ADR-013）。floor `r3 >= 0.70` + `remote@1 > det@1` 为不退化 guard（real 嵌入若买不到语义则 r3<0.70 或 r1 不胜基线 → 红），真实测得 `r3=1.0000（3/3 稳定）/ delta@1>=+0.8667` 留足余量。CI 据实 honest-defer：remote 是付费外部 API、无免费 service container（与 qdrant 不同——qdrant 有免费 OSS service container，task-36.2 经 `qdrant-recall` job 每次 CI run 实测），故召回由本机已认证 run 实测，ADR-042 D2 记载这一诚实差异。

**grounding（实施期，ADR-013，impl PR 回填确认）**：
- harness live 测试名 `test_remote_embedding_semantic_recall` + 非网络守护测试名 `test_labeled_set_well_formed`（spec §7 草拟 TEST-ID）；标注集 `docs()` 16 doc / `cases()` 15 case，`const DIM: usize = 1024`，collection 名 `remote_embed_recall`——均机械命名，行为同 spec。
- 真实 CI live run（每次 CI run 对端点跑）因 remote 是付费外部 API、无免费 service container 据实 honest-defer → `[SPEC-DEFER:phase-future.embedding-remote-ci-credential]`；本 task 的 live 证据 = 主 agent 本机对真实 SiliconFlow 端点跑出的 `recall@1=0.8667–0.9333（跨 run 波动）/ recall@3=1.0000（稳定）`（真实非预填）。

**实际改动文件**（impl PR 真实回填）：
- 新增 `core/tests/remote_embedding_recall.rs`（`#![cfg(feature = "embedding-remote")]`，env-gated `CONTEXTFORGE_REMOTE_API_KEY` + 未设 honest-defer skip + api_key 永不记录；作者手工标注集 16 doc / 15 case 含近义干扰，`DIM=1024`；同一标注集 + 同一 `BruteForceVectorBackend` 精确余弦路径 `select_provider("remote")` vs `select_provider("deterministic")` 对照，每 query top-k，`recall@1/@3` + eprintln 真实值 + delta，`assert!(r3 >= 0.70)` 且 `assert!(remote@1 > det@1)`；live 测试 TEST-37.1.1 + 非网络 well-formed 守护 TEST-37.1.2）。
- 0 provider / backend 改动 / 0 新 dep / 0 schema migration / 0 默认 network / 0 默认构建变更（默认 0-network-dep，ADR-004/008）。ADR-042 D1 ratify 依据（@ task-37.3 closeout）。
