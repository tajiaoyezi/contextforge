# Phase 41 · tokenizer-default-on

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase **做出 Phase 30（cjk-true-segmenter, Done / v0.23.0）经 ADR-035 §D3 据「翻默认是产品决策」诚实延后的那个产品决策**：把 code/CJK 感知 tokenizer（`code_cjk`，纯 std、0-dep bigram + 代码符号拆分，task-24.1 / ADR-029）从 **opt-in** 翻为**新建 collection 的生产默认**。grounding 真实状态：(a) 生产索引路径全走 `IndexSession::open(..)`（`core/src/server.rs:141` `CoreService::index` RPC + `core/src/jobs/index_session_backend.rs:151`）→ `open_with_tokenizer(.., DEFAULT_TOKENIZER="default")`（`core/src/indexer/mod.rs:183/502`），即默认仍走 Tantivy 默认 `TEXT` analyzer；**今天 0 tokenizer env/config 接线**（不像 vector/reranker 有 `CONTEXTFORGE_*`）。(b) tokenizer 绑定的**唯一真相源是 Tantivy `meta.json`**——`open_with_tokenizer` 仅在 **create 时**（`meta.json` 不存在）用传入 tokenizer 建 schema，**open 既有 collection 时走 `Index::open_in_dir` 读回持久化 schema、忽略传入值**（`indexer/mod.rs:528-535`）；query 侧（`Retriever::open_with_config`）据 schema 字段绑定派生 analyzer（schema-driven 对称，ADR-035 D3）。**故翻默认对既有 collection 自动安全**（既有 collection 保持其持久化的 `TEXT` analyzer、index/query 仍对称、不被静默失效），仅**新建** collection 绑新默认 `code_cjk`。(c) 既有索引迁移工具 `IndexSession::reindex_with_tokenizer`（`indexer/mod.rs:920-981`）+ schema-driven 对称口径自 Phase 30 已备；本 phase 只补「翻默认 + opt-out 通道」。**关键诚实定性（ADR-013，本 phase 核心）**：这是项目史上**首次刻意改默认行为**（新建 collection 的倒排词项由 `TEXT` 变 `code_cjk`，非 byte-equivalent）——区别于历来「默认 byte-equiv」红线。该刻意默认变更由**新 ADR-046 显式承接**，以三重安全 + 一处实测收益为据：① 既有 collection 不受影响（持久化 schema，仍可检索）；② `CONTEXTFORGE_TOKENIZER=default` env + Go `[retrieval] tokenizer` config 提供 opt-out 回退到 legacy `TEXT`（byte-equiv）；③ 既有 collection 升级到 `code_cjk` 由用户经既有 `reindex_with_tokenizer` 主动触发（不自动迁移用户数据）；④ Phase 24 已实测 `code_cjk` over default `TEXT` recall delta **+0.0909**（real，非预填）justify 翻默认。`code_cjk` 是纯 std → **0 新依赖**（ADR-008）；jieba 真分词 `cjk_segmenter` 仍 feature-gated opt-in（Phase 30 实测 jieba vs bigram delta=+0.0000 → 默认不取重词典 dep）。

> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md §3.23 + §4 backlog` → 源码锚点（`core/src/indexer/mod.rs:183`（`DEFAULT_TOKENIZER = "default"`）+ `:185/:189`（`CODE_CJK_TOKENIZER = "code_cjk"` / `CJK_SEGMENTER_TOKENIZER = "cjk_segmenter"`）+ `:499-556`（`open` → `open_with_tokenizer`，:528-535 create-vs-open meta.json 分支）+ `:920-981`（`reindex_with_tokenizer` 迁移工具）/ `core/src/server.rs:141`（`CoreService::index` RPC 生产索引入口 `IndexSession::open`）+ `:521-560`（`resolve_data_dir` env-resolution 范式）+ `:551-560`（`resolve_vector_backend` env 范式镜像源）/ `core/src/jobs/index_session_backend.rs:151`（session backend 生产索引 `IndexSession::open`）/ `core/src/retriever/mod.rs:98-110`（`RetrieverConfig.tokenizer` vestigial + Default）+ `:250-260`（query 侧 analyzer 注册站点，schema-driven 对称）/ `internal/config/config.go:96-99`（`VectorConfig` add-only 段范式）+ `:238-240`（encode `[vector]`）+ `:269/:305-308/:435-451`（decode `[vector]`）/ `cmd/contextforge/main.go:304-346`（`setVectorEnv` env-bridge 镜像源）+ `:108-118` / `:150-160`（doServe / doMCP 接线点）） → AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5，**第三十二次**激活）→ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造红线：recall delta +0.0909 真实实测非合成 / 刻意默认变更据实定性非夸大 byte-equiv / jieba 默认不取据实 / 既有 collection 不自动迁移据实）。

> **ADR 影响面（已识别）**：
> - **ADR-046 tokenizer-default-on（新，Proposed）**：记 production 默认翻 `code_cjk`（resolve_tokenizer env-resolution + 既有 collection schema-driven 安全 + Phase 24 实测 +0.0909 justify，D1）+ `CONTEXTFORGE_TOKENIZER` env opt-out/override + Go `[retrieval] tokenizer` config 桥（D2）+ recall delta 复测 + honest-defer 边界（jieba 默认不取 / 既有 collection 用户主动 reindex，D3）+ 刻意默认变更由本 ADR 承接 + 0-dep / 0-network + opt-out 保 legacy byte-equiv（D4）。Status: Proposed（Draft 阶段不 ratify；ratify 在 task-41.3 closeout）。
> - 触及 **ADR-029（code-and-cjk-tokenizer-and-eval-hardening）**：其 §Negative / Follow-ups 以 `[SPEC-DEFER:phase-future.tokenizer-default-on]`（adr-029:54）记「tokenizer 默认开启 + 索引迁移工具」延后——本 phase 经 add-only Amendment 记其**默认开启维度兑现**（生产默认翻 `code_cjk` + opt-out + 既有 collection 安全），不溯改 ADR-029 正文（ADR-014 D5）。
> - 触及 **ADR-035（cjk-true-segmenter-and-tokenizer-default）**：其 D3 评估 tokenizer-default-on 后据「翻默认是产品决策」诚实延后 full default flip（`[SPEC-DEFER:phase-future.tokenizer-default-on]`），并记 `RetrieverConfig.tokenizer` vestigial + 迁移工具已备——本 phase 经 add-only Amendment 记该产品决策兑现（默认翻 `code_cjk`，`cjk_segmenter` jieba 仍 feature opt-in），不溯改 ADR-035 正文（ADR-014 D5）。
> - 触及 **ADR-004（默认行为 + 既有契约不变）→ 本 phase 刻意例外，由 ADR-046 承接**：本 phase 是项目首次**刻意改默认行为**（新建 collection 倒排词项 `TEXT`→`code_cjk`，非 byte-equiv）。ADR-046 显式承接该例外，以「既有 collection 不受影响（持久化 schema）+ `CONTEXTFORGE_TOKENIZER=default` / `[retrieval]` opt-out 保 legacy byte-equiv + 既有 collection 用户主动 reindex + Phase 24 实测 +0.0909 justify」为据；ADR-004 的「opt-out 后 legacy byte-equiv」+「既有用户数据零感知（不自动迁移）」safety intent 保持。
> - 触及 **ADR-008（dep add-only）**：本 phase = **0 新依赖**（默认翻 `code_cjk` 纯 std；jieba `cjk_segmenter` 仍 feature-gated 不进默认构建）。
> - 触及 **ADR-024（console-api-semantic-forward）/ ADR-025（hybrid）→ 不触及**：本 phase 改的是索引 analyzer 默认绑定，检索 RPC / proto / console-api 契约零 delta。

## 1. 阶段目标

v0.33.0 ship 后，ContextForge 做出 Phase 30 / ADR-035 D3 诚实延后的产品决策：把 code/CJK 感知 tokenizer `code_cjk`（task-24.1，纯 std bigram + 代码符号拆分，0-dep）从 opt-in 翻为**新建 collection 的生产默认**，使全体用户**默认**享受 Phase 24 实测的 recall 增益（+0.0909 over 默认 `TEXT`），而既有 collection 不受影响、并提供 opt-out 回退。具体：(1) `core/src/server.rs` 加 `resolve_tokenizer()` env-resolution（镜像 `resolve_data_dir`/`resolve_vector_backend`）读 `CONTEXTFORGE_TOKENIZER`：unset/"" → `CODE_CJK_TOKENIZER`（**翻默认**）；`"default"` → `DEFAULT_TOKENIZER`（opt-out legacy `TEXT`）；`"code_cjk"`/`"cjk_segmenter"`（后者须 feature）passthrough；其余 / feature 缺失 → stderr WARN + 回落 `code_cjk`（best-effort，镜像 Phase 35 surfacing）。生产索引两调用点（`server.rs:141` + `jobs/index_session_backend.rs:151`）由 `IndexSession::open(..)` 改 `open_with_tokenizer(.., &resolve_tokenizer())`；`IndexSession::open` / `DEFAULT_TOKENIZER` 库 API + 常量**不动**（向后兼容库调用方 + 既有单测）。(2) Go `internal/config/config.go` add-only `RetrievalConfig{Tokenizer}` + `[retrieval]` 段 + `setTokenizerEnv`（镜像 `setVectorEnv`，env-wins、无段不导出 → Rust 默认 `code_cjk`）接线 doServe/doMCP。(3) 经 `phase24_tokenizer_recall` harness 复测 default `TEXT` vs `code_cjk` 真实 recall delta（+0.0909 已 Phase 24 实测，本 phase 复确认默认化后此增益成为出厂基线）。**关键诚实定性（ADR-013）**：本 phase 是项目首次**刻意改默认行为**（新建 collection 倒排词项变更、非 byte-equiv），由 ADR-046 显式承接 + 三重安全 + 实测收益为据；既有 collection 不自动迁移（用户经既有 `reindex_with_tokenizer` 主动升级）；jieba `cjk_segmenter` 默认不取（feature-gated，0-dep baseline + Phase 30 实测 jieba vs bigram delta=+0.0000）。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. **production tokenizer 默认翻 `code_cjk` + env opt-out（🟢 / 🟡 真实 delta）**：`core/src/server.rs` add `resolve_tokenizer()`（读 `CONTEXTFORGE_TOKENIZER`，unset → `CODE_CJK_TOKENIZER`、`"default"` → `DEFAULT_TOKENIZER` opt-out、known passthrough、unknown/feature-off → stderr WARN + `code_cjk`）+ 生产索引两调用点（`server.rs:141` + `jobs/index_session_backend.rs:151`）改用 `open_with_tokenizer(.., &resolve_tokenizer())`；`IndexSession::open` / `DEFAULT_TOKENIZER` 不动；既有 collection 经 `open_in_dir` 保持持久化 `TEXT` analyzer（不被静默失效）；Phase 24 harness 复测 default vs `code_cjk` 真实 recall delta（+0.0909 实测，🟡 本地）（AC1）
2. **Go `[retrieval] tokenizer` config 桥（🟢）**：`internal/config/config.go` add-only `RetrievalConfig{Tokenizer string}` + `Config.Retrieval` + `[retrieval]` 段 encode/decode round-trip + `cmd/contextforge/main.go` add `setTokenizerEnv`（镜像 `setVectorEnv`：`[retrieval] tokenizer` 非空且 `CONTEXTFORGE_TOKENIZER` 未设 → 导出，env-wins，无段不导出 → Rust 默认 `code_cjk`）接线 doServe/doMCP；API key 无关（tokenizer 非密钥）；Rust core 0 toml dep（AC2）
3. **刻意默认变更承接 + 0-dep 守线 + honest-defer 边界 + v0.34.0 closeout**：刻意默认变更（新建 collection `TEXT`→`code_cjk` 非 byte-equiv）由 ADR-046 显式承接（既有 collection 安全 + opt-out legacy byte-equiv + 不自动迁移 + Phase 24 实测 +0.0909 justify）；0 新依赖（`code_cjk` 纯 std；jieba `cjk_segmenter` 仍 feature opt-in，ADR-008）+ 0 网络；honest-defer：jieba 默认开启 `[SPEC-DEFER:phase-future.cjk-segmenter-default-on]`（Phase 30 实测 vs bigram delta=0、重词典 dep）/ 既有 collection 自动迁移 `[SPEC-DEFER:phase-future.tokenizer-auto-reindex-on-upgrade]`（不自动改用户数据）/ 大语料 recall `[SPEC-DEFER:phase-future.tokenizer-large-corpus-recall]`；v0.34.0 release docs + `scripts/console_smoke.sh` v31[50/50] + ADR-046 据真实测试 ratify + ADR-029/035 add-only Amendment + roadmap §3.23/§4 add-only + phase §6 闭合（AC3）
4. ADR-014 D1-D5（**第三十二次**激活）全通过（AC4）

**v0.x 版本号决策**：v0.34.0（Phase 41，承 v0.33.0；roadmap §1.1 Phase N→v0.(N-7).0），theme tokenizer-default-on。minor release（首次刻意默认行为变更——code/CJK tokenizer 从 opt-in 翻为新建 collection 默认，由 ADR-046 显式承接 + 三重安全 + Phase 24 实测 +0.0909 justify；既有 collection 不受影响 + opt-out 保 legacy byte-equiv + 不自动迁移；默认构建 0 新依赖（ADR-008，`code_cjk` 纯 std）+ 0 网络）。

## 2. 业务价值

把 Phase 24 已实测但 opt-in 的 code/CJK recall 增益（+0.0909）变为**出厂默认**，让全体用户默认获得更好的代码符号 / CJK 检索召回，且对既有用户零破坏、可 opt-out：

### 41.1 production tokenizer 默认翻 code_cjk + env opt-out（tokenizer-default-on，🟢 / 🟡 真实 delta）

- grounding 真实状态：生产索引全走 `IndexSession::open(..)`（`server.rs:141` `CoreService::index` RPC + `jobs/index_session_backend.rs:151`）→ `open_with_tokenizer(.., DEFAULT_TOKENIZER="default")` → 新建 collection 的 `content` 字段绑 Tantivy 默认 `TEXT` analyzer。code/CJK `code_cjk` analyzer（task-24.1：camelCase/snake_case/dotted.path/kebab-case 拆子词 + 保留原 token + CJK bigram，纯 std）自 v0.17.0 已**实存但 opt-in**（须 `RetrieverConfig.tokenizer="code_cjk"` —— 而该字段经 ADR-035 D3 核实为 vestigial，真实选择由 `open_with_tokenizer` 的 tokenizer 参数在 create 时写入 `meta.json` schema 决定）。
- 本 phase 加 `resolve_tokenizer()`（`server.rs`，镜像 `resolve_data_dir`/`resolve_vector_backend` 的 env-resolution）：读 `CONTEXTFORGE_TOKENIZER`，**unset/"" → `CODE_CJK_TOKENIZER`（翻默认）**；`"default"` → `DEFAULT_TOKENIZER`（opt-out 回 legacy `TEXT`）；`"code_cjk"` → `CODE_CJK_TOKENIZER`；`"cjk_segmenter"` → feature 在则 `CJK_SEGMENTER_TOKENIZER`、feature 缺则 stderr WARN + `CODE_CJK_TOKENIZER`；其余未知值 → stderr WARN + `CODE_CJK_TOKENIZER`（best-effort 不静默落 `TEXT`，镜像 Phase 35 surfacing）。生产索引两调用点改 `open_with_tokenizer(.., &resolve_tokenizer())`。`IndexSession::open`（库便捷入口）/ `DEFAULT_TOKENIZER` 常量**不动**（向后兼容库调用方 + 既有 indexer/retriever 单测 `content_tokenizer_name == DEFAULT_TOKENIZER` 不破）。
- **既有 collection 自动安全**：`open_with_tokenizer` 对 `meta.json` 存在的 collection 走 `Index::open_in_dir`（读回持久化 schema，**忽略传入 tokenizer**）→ 既有 `TEXT` collection 仍按 `TEXT` 索引 / 检索（query 侧 schema-driven 对称）；仅 `meta.json` 不存在的**新建** collection 绑 `code_cjk`。新建 `code_cjk` collection 的 query 侧（`Retriever::open_with_config`）`register_code_cjk` 无条件注册（task-24.1 R4，index/query 对称）。
- **真实收益（ADR-013，不预填）**：Phase 24（`docs/spikes/phase-24-tokenizer-recall.md` / `phase24_tokenizer_recall.rs`）实测 `code_cjk` over default `TEXT` recall delta **+0.0909**（default 0.9091 → code/CJK 1.0000，over task-24.2 golden）。本 phase 经同一 harness 复测确认默认化后此增益成为出厂基线（真实数值待实施回填，小语料 caveat，大语料续 `[SPEC-DEFER:phase-future.tokenizer-large-corpus-recall]`）。
- **HONEST DEFINITION（刻意默认变更，不夸大为 byte-equiv，ADR-013）**：本项使**新建** collection 的 `content` 倒排词项由 `TEXT` 变 `code_cjk`——**非 byte-equivalent**，是项目首次刻意默认行为变更，由 ADR-046 显式承接。安全边界：既有 collection 不受影响；`CONTEXTFORGE_TOKENIZER=default` opt-out 回 legacy `TEXT`（byte-equiv）；既有 collection 升级到 `code_cjk` 由用户经 `reindex_with_tokenizer` 主动触发（不自动迁移用户数据）。

### 41.2 Go `[retrieval] tokenizer` config 桥（tokenizer-config-bridge，🟢）

- 今天 0 tokenizer config/env 接线——`resolve_tokenizer`（task-41.1）令 `CONTEXTFORGE_TOKENIZER` env 可控默认/opt-out，但 Go config.toml 尚无对应段。本 phase add-only `RetrievalConfig{Tokenizer string}` + `Config.Retrieval` + `[retrieval]` 段（encode/decode round-trip，镜像 `VectorConfig`/`[vector]`）+ `setTokenizerEnv`（`cmd/contextforge/main.go`，镜像 `setVectorEnv`）：`[retrieval] tokenizer` 非空且 `CONTEXTFORGE_TOKENIZER` 未设 → `os.Setenv("CONTEXTFORGE_TOKENIZER", cfg.Retrieval.Tokenizer)`，**env-wins**（显式 env 覆盖 config），无 `[retrieval]` 段 / 空值 → 不导出 → Rust `resolve_tokenizer` 默认 `code_cjk`。接线 doServe/doMCP（镜像 setVectorEnv/setRerankerEnv 接线点）。
- **设计定性**：翻默认的语义在 **Rust 默认**（`resolve_tokenizer` unset → `code_cjk`）；Go config 仅作 **opt-out / override 通道**（如 `[retrieval] tokenizer = "default"` 回 legacy `TEXT`、`"cjk_segmenter"` 升 jieba）——与 vector/reranker 桥同构（Rust env 路径是消费方，Go 桥 config→env）。tokenizer 非密钥（不涉及 API key 安全 baseline）。Rust core 0 toml dep（复用既有跨进程 env-bridge）。

**不在本 phase 范围**：

- jieba 真分词 `cjk_segmenter` 默认开启（须重词典 dep 破 0-dep baseline；Phase 30 实测 jieba vs bigram delta=+0.0000 无增益）[SPEC-DEFER:phase-future.cjk-segmenter-default-on]
- 既有 collection 升级时自动 reindex 到 `code_cjk`（不自动改用户数据；用户经既有 `reindex_with_tokenizer` 主动触发）[SPEC-DEFER:phase-future.tokenizer-auto-reindex-on-upgrade]
- 大语料 tokenizer recall 基准（小 golden 实测 +0.0909，大规模续测）[SPEC-DEFER:phase-future.tokenizer-large-corpus-recall]
- `RetrieverConfig.tokenizer` vestigial 字段的真路由（ADR-035 D3 已定性为 schema-driven 对称，本 phase 不改其 vestigial 状态）[SPEC-DEFER:phase-future.retriever-config-tokenizer-routing]
- 其余治理 / 检索 marker 据实保持延后（`vector-dim-feature-enforce` 须 feature build / `chunk-source-type-filter` 须 import-path schema migration）

## 3. 涉及模块

### 41.1 tokenizer-default-on（task-41.1）

- 修改 `core/src/server.rs`——add `resolve_tokenizer() -> String`（读 `CONTEXTFORGE_TOKENIZER`，镜像 `resolve_data_dir:521-549`/`resolve_vector_backend:551-560` 的 env-resolution；unset/"" → `CODE_CJK_TOKENIZER`、`"default"` → `DEFAULT_TOKENIZER`、`"code_cjk"`/`"cjk_segmenter"`(feature) passthrough、unknown/feature-off → stderr WARN + `CODE_CJK_TOKENIZER`）+ `CoreService::index` RPC `IndexSession::open(&data_dir, &collection_id)`（:141）→ `IndexSession::open_with_tokenizer(&data_dir, &collection_id, &resolve_tokenizer())`
- 修改 `core/src/jobs/index_session_backend.rs`——`IndexSession::open(data, workspace_id)`（:151）→ `open_with_tokenizer(data, workspace_id, &resolve_tokenizer())`（复用 `server::resolve_tokenizer` 或同 crate 共享 helper）
- **不改** `core/src/indexer/mod.rs` `IndexSession::open`（:502，库便捷入口续走 `DEFAULT_TOKENIZER`）/ `DEFAULT_TOKENIZER` 常量（:183）/ `open_with_tokenizer`（:511，create-vs-open meta.json 分支已对既有 collection 安全）/ `reindex_with_tokenizer`（:920，迁移工具已备）
- 复测（不改）`core/examples/phase24_tokenizer_recall.rs` + `docs/spikes/phase-24-tokenizer-recall.md`——确认 default `TEXT` vs `code_cjk` recall delta +0.0909 默认化后成出厂基线
- 同源验证（≥2，🟢 / 🟡：`resolve_tokenizer` env 矩阵（unset→code_cjk / "default"→default / "code_cjk"→code_cjk / unknown→WARN+code_cjk）（TEST-41.1.1）+ 生产路径新建 collection 默认绑 `code_cjk`（`content_tokenizer_name`）/ opt-out env → 绑 `default` / 既有 `TEXT` collection 经生产路径 open 保持 `TEXT`（不被静默改）（TEST-41.1.2）+ Phase 24 harness 真实 recall delta +0.0909 复测（TEST-41.1.3，🟡 本地）)

### 41.2 tokenizer-config-bridge（task-41.2）

- 修改 `internal/config/config.go`——add-only `RetrievalConfig struct { Tokenizer string }`（toml `tokenizer`）+ `Config.Retrieval RetrievalConfig`（镜像 `VectorConfig`/`Config.Vector` :96-99/:40）+ `encodeTOML` `[retrieval]` 段（镜像 `[vector]` :238-240）+ `decodeTOML` `case line == "[retrieval]"`（镜像 :269）+ `assignRetrieval`（镜像 `assignVector` :435-451）
- 修改 `cmd/contextforge/main.go`——add `setTokenizerEnv(dataDir string) func()`（镜像 `setVectorEnv:304-346`：load config best-effort、`setIfAbsent("CONTEXTFORGE_TOKENIZER", cfg.Retrieval.Tokenizer)`、env-wins、missing config 静默 / 真 parse-err stderr WARN）+ doServe（:108-118）/ doMCP（:150-160）接线（`restoreTok := setTokenizerEnv(opts.DataDir)` + defer restore，镜像 setVectorEnv 接线）
- 修改 `internal/config/config_test.go`——`[retrieval] tokenizer` round-trip（Save→Load 等价，镜像既有 vector/reranker round-trip test）
- 同源验证（≥2，🟢：config `[retrieval] tokenizer` Save/Load round-trip（TEST-41.2.1）+ `setTokenizerEnv` env-wins / 无段不导出 / 非空导出 `CONTEXTFORGE_TOKENIZER`（TEST-41.2.2，镜像 setVectorEnv test 形态）)

### 41.3 closeout（task-41.3）

- 修改 `scripts/console_smoke.sh`——banner v30→v31 + v31 changelog block + 新 step [50/50]（REAL 模式：索引含 camelCase 符号 `getUserProfile` 的片段、search 子词 `profile` → 默认 `code_cjk` 命中（证翻默认生效）；`CONTEXTFORGE_TOKENIZER=default` opt-out → 新建 collection → `profile` miss（证 opt-out 回 `TEXT`）；不可达则 doc/status；current Phase 40 [49/49] → Phase 41 顺位 [50/50]）
- 修改 `internal/cli/smoke_syntax_test.go`——新 `TestTask413`（镜像 `TestTask403`）断言 [50/50] + markers（tokenizer-default-on / code_cjk / CONTEXTFORGE_TOKENIZER / X-Actor 等不溯改）+ no-regression（denominators [37/37]..[49/49] 不溯改，ADR-014 D5）
- 新增 `docs/releases/v0.34.0-evidence.md` + `v0.34.0-artifacts.md`（tag SHA / run id / digest 为 angle-bracket backfill marker）+ `README.md` v0.34 段 + `RELEASE_NOTES.md` v0.34.0 段（含「default tokenizer 翻 code_cjk + opt-out via CONTEXTFORGE_TOKENIZER / [retrieval] tokenizer + 既有 collection 不受影响 / reindex 升级」Upgrade 段）
- 修改 `docs/decisions/adr-046-tokenizer-default-on.md`——Status Proposed→Accepted（逐 D 如实）+ 新 `## Ratification（v0.34.0 / task-41.3）`
- add-only Amendment（非正文改，ADR-014 D5）：`adr-029`（code-and-cjk-tokenizer，默认开启维度兑现 add-only）/ `adr-035`（cjk-true-segmenter-and-tokenizer-default，D3 产品决策兑现 add-only）；`docs/roadmap.md §3.23/§4` add-only（Phase 41 行 + 新 backlog 条目 cjk-segmenter-default-on / tokenizer-auto-reindex-on-upgrade）
- 修改 `docs/specs/phases/phase-41-tokenizer-default-on.md`——Status Draft→Done + §6 AC 勾选（逐维如实）
- 修改 `docs/s2v-adapter.md`——Phase 41 行 + Task 行 + ADR-046 行 + BDD 行

### BDD feature

- 新增 `test/features/phase-41-tokenizer-default-on.feature`（≥3 scenario：production 默认翻 `code_cjk`（新建 collection 绑 code_cjk + 既有 TEXT collection 不受影响 + CONTEXTFORGE_TOKENIZER=default opt-out 回 TEXT）/ Go `[retrieval] tokenizer` config 桥（env-wins / 无段默认 code_cjk）/ v0.34.0 收口 + 刻意默认变更承接 + 0-dep 守线）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 41.1 | `core/src/server.rs` add `resolve_tokenizer()`（env-resolution，unset→`code_cjk` 翻默认 / `"default"`→opt-out `TEXT` / known passthrough / unknown→WARN+code_cjk）+ 生产索引两调用点（`server.rs:141` + `jobs/index_session_backend.rs:151`）改 `open_with_tokenizer(.., &resolve_tokenizer())`；`IndexSession::open`/`DEFAULT_TOKENIZER` 不动；既有 collection schema-driven 安全；Phase 24 harness 复测真实 recall delta +0.0909（刻意默认变更非 byte-equiv，由 ADR-046 承接） | `../tasks/task-41.1-tokenizer-default-on.md` |
| 41.2 | `internal/config/config.go` add-only `RetrievalConfig{Tokenizer}` + `[retrieval]` 段 round-trip + `cmd/contextforge/main.go` `setTokenizerEnv`（镜像 `setVectorEnv`，env-wins、无段不导出 → Rust 默认 code_cjk）接线 doServe/doMCP；Rust 0 toml dep | `../tasks/task-41.2-tokenizer-config-bridge.md` |
| 41.3 | smoke v31[50/50] + v0.34.0 closeout + ADR-046 ratify + ADR-029/035 add-only Amendment + roadmap §3.23/§4 add-only + s2v-adapter add-only | `../tasks/task-41.3-closeout-v0.34.0.md` |

## 5. 依赖关系

- **task-41.1**（tokenizer-default-on）dep 既有 `core/src/indexer/mod.rs` `open_with_tokenizer`（:511 + create-vs-open meta.json 分支 :528-535 已在）+ `CODE_CJK_TOKENIZER`/`DEFAULT_TOKENIZER`（:185/:183 已在）+ `register_code_cjk`（:379 已在）+ `core/src/server.rs` `resolve_data_dir`/`resolve_vector_backend`（env-resolution 范式 :521-560 已在）+ `CoreService::index`（:141 已在）+ `jobs/index_session_backend.rs`（:151 已在）+ `phase24_tokenizer_recall.rs` harness（已在）；可独立先行（不依赖 41.2）。
- **task-41.2**（tokenizer-config-bridge）dep 既有 `internal/config/config.go` `VectorConfig`/`assignVector`/`[vector]` codec（:96-99/:435-451/:238-240 已在）+ `cmd/contextforge/main.go` `setVectorEnv`（:304-346 已在）+ doServe/doMCP 接线点（:108-118/:150-160 已在）+ task-41.1 `CONTEXTFORGE_TOKENIZER` 消费方（resolve_tokenizer）；config 桥与 Rust 消费方逻辑独立、可与 41.1 并行（Go 桥导出的 env 由 41.1 的 resolve_tokenizer 消费，端到端在 closeout smoke 验证）。
- **task-41.3**（closeout）dep 41.1 + 41.2 全 Done；release docs / smoke v31[50/50] / ADR-046 ratify 据两 task 真实测试 / 实测产物。
- 外部：ADR-046（本 phase 新 Proposed）/ ADR-029（code-and-cjk-tokenizer，默认开启维度兑现 add-only Amendment）/ ADR-035（cjk-true-segmenter-and-tokenizer-default，D3 产品决策兑现 add-only Amendment）/ ADR-004（刻意默认变更例外由 ADR-046 承接，opt-out 后 legacy byte-equiv + 不自动迁移 safety intent 保持）/ ADR-008（dep add-only，Phase 41 = 0 新依赖，`code_cjk` 纯 std）/ ADR-012（tag/release outward-facing 须用户显式授权）/ ADR-014 **第三十二次**激活 / ADR-013（禁伪造红线，recall delta +0.0909 真实实测、刻意默认变更据实定性非夸大、jieba 默认不取据实、既有 collection 不自动迁移据实）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（每条 AC 含 ADR-014 D3 verified by 显式 owner；Draft 阶段未勾选，实施后逐条置 `[x]`）**：

- [x] **AC1**（production tokenizer 默认翻 `code_cjk` + env opt-out 🟢 / 🟡 真实 delta）: `core/src/server.rs` add `resolve_tokenizer()`（读 `CONTEXTFORGE_TOKENIZER`，unset/"" → `CODE_CJK_TOKENIZER` 翻默认 / `"default"` → `DEFAULT_TOKENIZER` opt-out / `"code_cjk"`/`"cjk_segmenter"`(feature) passthrough / unknown/feature-off → stderr WARN + `CODE_CJK_TOKENIZER`）+ 生产索引两调用点（`server.rs:141` `CoreService::index` + `jobs/index_session_backend.rs:151`）改 `open_with_tokenizer(.., &resolve_tokenizer())`；`IndexSession::open`/`DEFAULT_TOKENIZER` 不动；既有 collection 经 `open_in_dir` 保持持久化 `TEXT`（不被静默失效）；Phase 24 harness 复测 default vs `code_cjk` 真实 recall delta +0.0909（🟡 本地，刻意默认变更非 byte-equiv 由 ADR-046 承接） — verified by **TEST-41.1.1**（`resolve_tokenizer` env 矩阵）+ **TEST-41.1.2**（生产路径新建 collection 绑 `code_cjk` / opt-out env 绑 `default` / 既有 `TEXT` collection 保持 `TEXT`）+ **TEST-41.1.3**（Phase 24 harness 真实 recall delta +0.0909 复测）+ phase-smoke step 1
- [x] **AC2**（Go `[retrieval] tokenizer` config 桥 🟢）: `internal/config/config.go` add-only `RetrievalConfig{Tokenizer string}` + `Config.Retrieval` + `[retrieval]` 段 encode/decode round-trip；`cmd/contextforge/main.go` add `setTokenizerEnv`（镜像 `setVectorEnv`：`[retrieval] tokenizer` 非空且 env 未设 → 导出 `CONTEXTFORGE_TOKENIZER`，env-wins，无段不导出 → Rust 默认 `code_cjk`）接线 doServe/doMCP；Rust core 0 toml dep — verified by **TEST-41.2.1**（config `[retrieval] tokenizer` Save/Load round-trip）+ **TEST-41.2.2**（`setTokenizerEnv` env-wins / 无段不导出 / 非空导出）+ phase-smoke step 2
- [x] **AC3**（刻意默认变更承接 + 0-dep 守线 + honest-defer 边界 + v0.34.0 closeout）: 刻意默认变更（新建 collection `TEXT`→`code_cjk` 非 byte-equiv）由 ADR-046 显式承接（既有 collection 安全 + `CONTEXTFORGE_TOKENIZER=default`/`[retrieval]` opt-out 回 legacy byte-equiv + 不自动迁移 + Phase 24 实测 +0.0909 justify）；0 新依赖（`code_cjk` 纯 std；jieba `cjk_segmenter` 仍 feature opt-in，ADR-008）+ 0 网络；honest-defer：`cjk-segmenter-default-on`（jieba 重 dep + Phase 30 delta=0）/ `tokenizer-auto-reindex-on-upgrade`（不自动改用户数据）/ `tokenizer-large-corpus-recall` 据实保持延后；v0.34.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ `scripts/console_smoke.sh` v31[50/50] + `internal/cli/smoke_syntax_test.go` `TestTask413`（no-regression [37/37]..[49/49]）+ ADR-046 据真实测试 ratify + ADR-029/035 add-only Amendment + roadmap §3.23/§4 add-only + phase §6 闭合 — verified by **TEST-41.3.1**（smoke v31[50/50] + smoke_syntax_test + ADR-046 ratify + roadmap/adapter add-only + phase §6 闭合）
- [x] **AC4**（ADR-014 cross-validation gate）: ADR-014 D1-D5（**第三十二次**激活）全通过 — D1 mapping + D2 lint `bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-40 不溯改（ADR 改动 add-only Amendment）— verified by task-41.3 closeout PR body + 各 task LAST TEST（TEST-41.1.4 / TEST-41.2.3 / TEST-41.3.2）

**端到端 smoke（C1 集成兜底）**：(1) `resolve_tokenizer` 默认 `code_cjk` + 生产路径新建 collection 绑 `code_cjk`（子词 `profile` 命中 `getUserProfile`）/ `CONTEXTFORGE_TOKENIZER=default` opt-out 回 `TEXT`（子词 miss）/ 既有 `TEXT` collection 保持 `TEXT` 全 PASS（刻意默认变更据实标注、非 byte-equiv）；(2) Go `[retrieval] tokenizer` config round-trip + `setTokenizerEnv` env-wins / 无段默认 code_cjk 全 PASS；(3) v0.34.0 收口 + 0-dep 守线 + honest-defer 边界全 PASS（jieba 默认不取 / 不自动迁移如实标注）。

## 7. 阶段级风险

- **R1（高）翻默认破既有 collection / 静默失效**：若翻默认令既有 `TEXT` collection 被改用 `code_cjk` 索引或 query 侧不对称，则既有索引召回静默退化。
  - **缓解**：`open_with_tokenizer` 对 `meta.json` 存在的 collection 走 `Index::open_in_dir`（读回持久化 schema、忽略传入 tokenizer），既有 `TEXT` collection 不受影响（已是 indexer 既有行为，本 phase 复用）；query 侧 `register_code_cjk` 无条件注册保对称；TEST-41.1.2 断言既有 `TEXT` collection 经生产路径 open 后 `content_tokenizer_name` 仍 `default`。stop-condition：既有 collection 被静默改 tokenizer / query 侧不对称则 AC1 不标 `[x]`。
- **R2（高）刻意默认变更被误记为 byte-equiv / 越红线**：本 phase 是首次刻意改默认行为，若沿用「默认 byte-equiv」措辞则不诚实，且未由 ADR 承接则违 ADR-004 治理。
  - **缓解**：spec §1/§2/§6 + ADR-046 D1/D4 据实定性「新建 collection `TEXT`→`code_cjk` 非 byte-equiv、首次刻意默认变更」+ ADR-046 显式承接该例外（三重安全 + Phase 24 实测 justify）；ADR-004 的「opt-out 后 legacy byte-equiv + 不自动迁移用户数据」safety intent 据实保持。stop-condition：若把刻意默认变更夸大为 byte-equiv / 未由 ADR-046 承接则越界（ADR-013）。
- **R3（中）recall delta 预填 / 夸大小语料**：+0.0909 是 Phase 24 小 golden 实测，若预填或夸大为大基准则违 ADR-013。
  - **缓解**：spec §2 41.1 + ADR-046 D3 标 +0.0909 为 Phase 24 小 golden 实测、本 phase 复确认；大语料续 `[SPEC-DEFER:phase-future.tokenizer-large-corpus-recall]`；真实复测数 task-41.1 §10 + evidence 真实跑出后回填（不预填）。stop-condition：预填 / 夸大为大基准则 AC1/AC3 不标 `[x]`。
- **R4（中）默认翻到 jieba `cjk_segmenter` 破 0-dep**：若默认翻到 jieba 真分词则引入重词典 dep、破 ADR-008 0-dep baseline。
  - **缓解**：默认翻 **`code_cjk`（纯 std，0-dep）**；jieba `cjk_segmenter` 仍 feature-gated opt-in（`resolve_tokenizer` 仅在 feature 在时 passthrough、缺则 WARN+code_cjk）；ADR-046 A3 据实记 Phase 30 实测 jieba vs bigram delta=+0.0000 → 默认不取重 dep。stop-condition：默认构建引入 jieba dep 则 AC3 不标 `[x]`。
- **R5（低）resolve_tokenizer 未知值静默落 TEXT 抵消翻默认**：未知 env 值若静默落 `TEXT` 则用户以为已 code_cjk 实则没有。
  - **缓解**：未知 / feature-off 值 → stderr WARN + 回落 `code_cjk`（best-effort、不静默落 `TEXT`，镜像 Phase 35 surfacing）；TEST-41.1.1 断言未知值 → WARN + `code_cjk`。stop-condition：未知值静默落 `TEXT` 则 AC1 不标 `[x]`。

## 8. Definition of Done

- 3 task spec（41.1-41.3）顶部 `**Status**: Done`
- §6 阶段级 AC1-4 全 `[x]`（受阻 / 延后态按 ADR-013 如实记录，不伪造——如 jieba 默认开启据实延后 `[SPEC-DEFER:phase-future.cjk-segmenter-default-on]`、既有 collection 自动迁移据实延后）
- 端到端 smoke 3 step 全 PASS（含受阻 / 延后态如实标注）
- **ADR**：ADR-046 `Proposed → Accepted`（据真实测试 / 实测产物逐 D 项 ratify）；ADR-029 经 add-only Amendment 记录（默认开启维度兑现，不溯改正文，ADR-014 D5）；ADR-035 经 add-only Amendment 记录（D3 产品决策兑现，不溯改正文）；ADR-004（刻意默认变更例外由 ADR-046 承接 + opt-out byte-equiv + 不自动迁移 safety intent 保持）/ ADR-008（0 新依赖）守线引用；`docs/roadmap.md §3.23/§4` add-only（Phase 41 行 + 新 backlog 条目）
- **adapter**：§Phase 索引 Phase 41 `Draft → Done` + `Tasks 0 → 3`；§ADR 索引 ADR-046；§BDD 追加 phase-41 feature 行；ADR-029/035 Amendment 记录
- **release**：`docs/releases/v0.34.0-{evidence,artifacts}.md` + `RELEASE_NOTES.md` v0.34 段 + README v0.34 段
- **smoke**：`scripts/console_smoke.sh` v31[50/50]（production 默认 code_cjk + opt-out smoke + 既有 step 不退化，denominators [37/37]..[49/49] 不溯改）+ `internal/cli/smoke_syntax_test.go` `TestTask413` markers 同步
- **follow-up**：jieba 默认开启 `[SPEC-DEFER:phase-future.cjk-segmenter-default-on]` + 既有 collection 自动迁移 `[SPEC-DEFER:phase-future.tokenizer-auto-reindex-on-upgrade]` + 大语料 recall `[SPEC-DEFER:phase-future.tokenizer-large-corpus-recall]` + `RetrieverConfig.tokenizer` 路由 `[SPEC-DEFER:phase-future.retriever-config-tokenizer-routing]` 留 backlog
