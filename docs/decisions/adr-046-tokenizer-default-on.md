# ADR `046`: `tokenizer-default-on`

**Status**: Accepted（v0.34.0 / task-41.3 closeout 据真实 CI / 实测 recall delta 逐 D ratify；D1/D2/D3/D4 Accepted——见 §Ratification）

**Category**: 检索质量 / 默认行为演进（首次刻意默认变更）/ tokenizer 默认化 / config env-bridge
**Date**: 2026-06-07
**Decided By**: 主 agent（ADR-012 自治；本批为规划稿 Proposed）；tajiaoyezi ratification at v0.34.0 closeout
**Related**: ADR-029（code-and-cjk-tokenizer-and-eval-hardening — 确立 `code_cjk` 自定义 analyzer + opt-in，§Negative/Follow-ups `[SPEC-DEFER:phase-future.tokenizer-default-on]` adr-029:54，本 ADR D1 兑现默认开启维度）/ ADR-035（cjk-true-segmenter-and-tokenizer-default — D3 评估 tokenizer-default-on 后据「翻默认是产品决策」诚实延后 full default flip + 记 `RetrieverConfig.tokenizer` vestigial + 迁移工具已备，本 ADR 兑现该产品决策）/ ADR-004（local-first-privacy-baseline — 本 ADR 是项目首次**刻意默认行为变更**例外，由本 ADR 显式承接 + opt-out 后 legacy byte-equiv + 不自动迁移用户数据 safety intent 保持）/ ADR-008（dep add-only — Phase 41 = 0 新依赖，默认翻 `code_cjk` 纯 std，jieba `cjk_segmenter` 仍 feature-gated）/ ADR-013（禁伪造红线 — recall delta +0.0909 真实实测非合成、刻意默认变更据实定性非夸大 byte-equiv、jieba 默认不取据实、既有 collection 不自动迁移据实）/ ADR-012（main-agent-governance-autonomy — tag/release outward-facing 须用户显式授权）/ ADR-014（D1-D5，第三十二次激活）/ roadmap §3.23 + §4

## Context

ContextForge 截至 Phase 40（governance-debt-cleanup-3, Done / v0.33.0）已完成三轮治理债清扫。code/CJK 感知 tokenizer `code_cjk`（task-24.1 / ADR-029：camelCase/snake_case/dotted.path/kebab-case 拆子词 + 保留原 token + CJK bigram，**纯 std、0-dep**）自 v0.17.0 已实存，但**仅 opt-in**——其默认化在 Phase 24（ADR-029 §Negative）与 Phase 30（ADR-035 D3）两度被据实延后。grounding 逐维度调研结论：

- **生产默认仍是 `TEXT`（真实，opt-in 未默认化）**：生产索引路径全走 `IndexSession::open(..)`（`core/src/server.rs:141` `CoreService::index` RPC + `core/src/jobs/index_session_backend.rs:151`）→ `open_with_tokenizer(.., DEFAULT_TOKENIZER="default")`（`indexer/mod.rs:183/502`）→ 新建 collection 的 `content` 字段绑 Tantivy 默认 `TEXT` analyzer；**今天 0 tokenizer env/config 接线**（不像 vector/reranker 有 `CONTEXTFORGE_*`）。`RetrieverConfig.tokenizer` 经 ADR-035 D3 核实为 **vestigial（恒不读）**——真实选择由 `open_with_tokenizer` 的 tokenizer 参数在 create 时写入 Tantivy `meta.json` schema 决定。

- **翻默认对既有 collection 自动安全（真实，schema-driven 对称）**：tokenizer 绑定的**唯一真相源是 `meta.json`**——`open_with_tokenizer` 仅在 create 时（`meta.json` 不存在）用传入 tokenizer 建 schema，**open 既有 collection 走 `Index::open_in_dir` 读回持久化 schema、忽略传入值**（`indexer/mod.rs:528-535`）；query 侧（`Retriever::open_with_config`）据 schema 字段绑定派生 analyzer（schema-driven 对称，`register_code_cjk` 无条件注册 task-24.1 R4）。**故翻默认对既有 collection 自动安全**（既有 `TEXT` collection 保持持久化 analyzer、index/query 仍对称、不被静默失效），仅**新建** collection 绑新默认 `code_cjk`。既有 collection 升级到 `code_cjk` 由用户经既有迁移工具 `IndexSession::reindex_with_tokenizer`（`indexer/mod.rs:920-981`，Phase 30 已备）主动触发。

- **Phase 30 延后的是「产品决策」非「技术能力」（真实）**：ADR-035 D3 明记「迁移工具已备 + schema-driven 对称已文档化（方案 B），默认仍 opt-in，full default flip 据『翻默认是产品决策』诚实延后 `[SPEC-DEFER:phase-future.tokenizer-default-on]`」。本 phase 即做出该产品决策。

- **真实收益（Phase 24 实测，非合成）**：`docs/spikes/phase-24-tokenizer-recall.md` / `core/examples/phase24_tokenizer_recall.rs` 实测 `code_cjk` over default `TEXT` recall delta **+0.0909**（default 0.9091 → code/CJK 1.0000，over task-24.2 golden）。Phase 30 另实测 jieba 真分词 `cjk_segmenter` vs bigram `code_cjk` delta=**+0.0000**（小语料无增益）→ 默认取 0-dep `code_cjk`、不取重词典 jieba。

本 ADR 把「production tokenizer 从 opt-in 翻为新建 collection 默认 `code_cjk` + opt-out 通道」收敛为处理策略。**关键定性（ADR-013）**：这是项目史上**首次刻意默认行为变更**（新建 collection 倒排词项 `TEXT`→`code_cjk`，**非 byte-equivalent**），区别于历来「默认 byte-equiv」红线——由本 ADR 显式承接，以三重安全 + 一处实测收益为据。改动 🟢 可单测（resolve_tokenizer 矩阵 + 绑定断言 + config round-trip）/ 🟡 本地 real recall delta。0 新依赖（ADR-008）+ 0 网络。

## Decision

tokenizer 默认化采用 **「production 默认翻 code_cjk（env-resolution + 既有 collection 安全）+ env/config opt-out + 0-dep 守线 + 刻意默认变更显式承接」** 策略，分 4 个决策点：

### D1 — production tokenizer 默认翻 `code_cjk`（新建 collection；既有 collection schema-driven 安全；Phase 24 实测 +0.0909 justify）（task-41.1）🟢 / 🟡

`core/src/server.rs` add `resolve_tokenizer() -> String`（镜像 `resolve_data_dir`/`resolve_vector_backend` 的 env-resolution）读 `CONTEXTFORGE_TOKENIZER`：**unset/"" → `CODE_CJK_TOKENIZER`（翻默认）**；`"default"` → `DEFAULT_TOKENIZER`（opt-out 回 legacy `TEXT`）；`"code_cjk"` → `CODE_CJK_TOKENIZER`；`"cjk_segmenter"` → feature `cjk-segmenter` 在则 `CJK_SEGMENTER_TOKENIZER`、缺则 stderr WARN + `CODE_CJK_TOKENIZER`；其余未知值 → stderr WARN + `CODE_CJK_TOKENIZER`（best-effort 不静默落 `TEXT`，镜像 Phase 35 surfacing）。生产索引两调用点（`server.rs:141` `CoreService::index` + `jobs/index_session_backend.rs:151`）由 `IndexSession::open(..)` 改 `open_with_tokenizer(.., &resolve_tokenizer())`。`IndexSession::open`（库便捷入口）/ `DEFAULT_TOKENIZER` 常量**不动**（向后兼容库调用方 + 既有 indexer/retriever 单测）。既有 collection 经 `open_in_dir` 保持持久化 `TEXT` analyzer（不被静默失效）；仅新建 collection 绑 `code_cjk`。

**理由**：ADR-029 / ADR-035 D3 把 tokenizer 默认化作为已识别 follow-up 延后（`[SPEC-DEFER:phase-future.tokenizer-default-on]`），延后理由是「翻默认是产品决策」而非技术受阻（迁移工具 + schema-driven 对称已备）。`resolve_tokenizer` env-resolution 在生产调用点（而非改 `DEFAULT_TOKENIZER` 常量）是最 surgical 的翻默认 + opt-out：库 API `IndexSession::open` / 既有单测不破，且 `CONTEXTFORGE_TOKENIZER` 提供 opt-out 通道。既有 collection 经 `open_in_dir` 读回持久化 schema → 翻默认对其零影响（不被静默失效）。**默认翻 `code_cjk`（纯 std）非 jieba `cjk_segmenter`**：守 0-dep baseline（ADR-008）+ Phase 30 实测 jieba vs bigram delta=0（小语料无增益）。Phase 24 实测 `code_cjk` over `TEXT` recall delta +0.0909 是翻默认的真实收益依据（小 golden，大语料续 SPEC-DEFER）。**诚实定性（ADR-013）**：本项使新建 collection 倒排词项 `TEXT`→`code_cjk`——**非 byte-equivalent**，是首次刻意默认变更，spec / ADR 据实记、不夸大为 byte-equiv。备选「改 `DEFAULT_TOKENIZER` 常量」破库 API + 无 opt-out 通道，不取（见 §A2）；备选「默认 jieba」破 0-dep，不取（见 §A3）。

### D2 — `CONTEXTFORGE_TOKENIZER` env opt-out/override + Go `[retrieval] tokenizer` config 桥（task-41.1 env + task-41.2 config）🟢

opt-out / override 通道：(a) env `CONTEXTFORGE_TOKENIZER`（task-41.1 `resolve_tokenizer` 消费）——`"default"` 回 legacy `TEXT`、`"cjk_segmenter"` 升 jieba（feature 在时）；(b) Go config（task-41.2）：`internal/config/config.go` add-only `RetrievalConfig{Tokenizer string}` + `Config.Retrieval` + `[retrieval]` 段 encode/decode round-trip（镜像 `VectorConfig`/`[vector]`）+ `cmd/contextforge/main.go` `setTokenizerEnv`（镜像 `setVectorEnv`：`[retrieval] tokenizer` 非空且 `CONTEXTFORGE_TOKENIZER` 未设 → `os.Setenv`，**env-wins**，无段 / 空值 → 不导出 → Rust `resolve_tokenizer` 默认 `code_cjk`）接线 doServe/doMCP。Rust core 0 toml dep（复用既有跨进程 env-bridge）。

**理由**：翻默认的语义在 **Rust 默认**（`resolve_tokenizer` unset → `code_cjk`）；Go config 仅作 **opt-out / override 通道**——与 vector/reranker 桥同构（Rust env 路径是消费方、Go 桥 config→env、env-wins）。无 `[retrieval]` 段 → 不导出 → 默认 `code_cjk`（翻默认生效）；显式 `[retrieval] tokenizer = "default"` → opt-out 回 `TEXT`（legacy byte-equiv）。tokenizer 非密钥（不涉 API key 安全 baseline，与 remote/reranker 桥不同——无 key 排除）。备选「无 config opt-out 只 env」减用户回退面，不取（见 §A5）。

### D3 — recall delta 复测 + honest-defer 边界（task-41.1 复测 + all tasks）🟡 / 🟢

Phase 24 harness（`phase24_tokenizer_recall.rs`）复测 default `TEXT` vs `code_cjk` 真实 recall delta（+0.0909 已实测，本 phase 复确认默认化后此增益成出厂基线）——🟡 本地、小 golden caveat、真实数不预填（ADR-013）。honest-defer 边界：jieba `cjk_segmenter` 默认开启 `[SPEC-DEFER:phase-future.cjk-segmenter-default-on]`（重词典 dep 破 0-dep + Phase 30 实测 delta=0）/ 既有 collection 升级时自动 reindex `[SPEC-DEFER:phase-future.tokenizer-auto-reindex-on-upgrade]`（不自动改用户数据，用户经 `reindex_with_tokenizer` 主动触发）/ 大语料 tokenizer recall `[SPEC-DEFER:phase-future.tokenizer-large-corpus-recall]` / `RetrieverConfig.tokenizer` vestigial 字段真路由 `[SPEC-DEFER:phase-future.retriever-config-tokenizer-routing]`（ADR-035 D3 已定性 schema-driven 对称，本 phase 不改其 vestigial 状态）。其余检索 marker 据实保持延后（`vector-dim-feature-enforce` 须 feature build / `chunk-source-type-filter` 须 import-path schema migration）。

**理由**：recall delta 是翻默认的真实依据，须真实实测（Phase 24 已测 +0.0909）而非预填；小 golden caveat 据实记、大语料续 SPEC-DEFER。jieba 默认不取（0-dep + 无增益）、既有 collection 不自动迁移（用户数据零感知）是诚实的范围边界——焦点版本不强行扩面（ADR-013 honest over padding）。

### D4 — 刻意默认变更由本 ADR 承接 + 0-dep / 0-network + opt-out 保 legacy byte-equiv（all tasks）🟢

本 phase 是项目**首次刻意默认行为变更**（新建 collection `content` 倒排词项 `TEXT`→`code_cjk`，**非 byte-equivalent**）——由本 ADR D1 **显式承接**该例外，ADR-004 的 safety intent 据实保持：① 既有 collection 不受影响（持久化 schema，仍可检索）；② `CONTEXTFORGE_TOKENIZER=default` env / `[retrieval] tokenizer = "default"` config opt-out 回 legacy `TEXT`（byte-equiv）；③ 既有 collection 升级到 `code_cjk` 由用户主动 reindex（不自动迁移用户数据）；④ Phase 24 实测 +0.0909 justify。**0 新依赖**（`code_cjk` 纯 std；jieba `cjk_segmenter` 仍 feature-gated 不进默认构建，ADR-008）+ 0 网络。既有 `cargo-test` / `go-test` / `spec-lint` 三门不退化。

**理由**：ADR-004 local-first + 默认行为不变是 baseline，但 tokenizer 默认化的**目的**就是改新建 collection 默认——不能既「翻默认」又「byte-equiv」，故诚实定性为刻意例外，由本 ADR 承接 + 三重安全把破坏面降到「仅新建 collection、且可 opt-out / 不自动迁移」。ADR-008 0-dep 不可让渡（默认翻纯 std `code_cjk`）。受阻 / 另一层 marker 据 ADR-013 honest-defer、不强行扩面。

## Consequences

- **Positive**: code/CJK tokenizer 从 opt-in 翻为新建 collection 出厂默认（`resolve_tokenizer` env-resolution + 生产两调用点接线），全体用户默认获 Phase 24 实测的 +0.0909 recall 增益（代码符号 camelCase/snake_case/dotted.path/kebab-case 子词 + CJK bigram 命中）；既有 collection 经 `open_in_dir` schema-driven 自动安全（不被静默失效）；`CONTEXTFORGE_TOKENIZER` env + Go `[retrieval] tokenizer` config 提供 opt-out / override（env-wins，无段默认 code_cjk）；既有迁移工具 `reindex_with_tokenizer` 供用户主动升级既有 collection；**0 新依赖**（`code_cjk` 纯 std）+ 0 网络；既有三门不退化。
- **Negative / open**（受阻 / 另一层项如实，不伪造、不夸大）：本项**非 byte-equivalent**（新建 collection 倒排词项 `TEXT`→`code_cjk`，首次刻意默认变更，由本 ADR 承接）；既有 collection 升级到 `code_cjk` 须用户主动 reindex（不自动迁移用户数据）→ `[SPEC-DEFER:phase-future.tokenizer-auto-reindex-on-upgrade]`；jieba 真分词 `cjk_segmenter` 默认不取（重 dep + Phase 30 实测 delta=0）→ `[SPEC-DEFER:phase-future.cjk-segmenter-default-on]`；recall delta +0.0909 系小 golden 实测、大语料续 `[SPEC-DEFER:phase-future.tokenizer-large-corpus-recall]`；`RetrieverConfig.tokenizer` vestigial 状态不改（ADR-035 D3 schema-driven 对称）→ `[SPEC-DEFER:phase-future.retriever-config-tokenizer-routing]`。
- **Ratification**: 本 ADR **Proposed**。task-41.1/41.2 通过后于 v0.34.0 closeout（task-41.3）据真实 CI / 实测产物（resolve_tokenizer 矩阵 + 绑定断言 + config round-trip + Phase 24 harness real recall delta + smoke v31[50/50]）逐 D ratify Proposed→Accepted（ADR-013：禁据合成 / 伪造 ratify）。
- **Follow-ups**: jieba `cjk_segmenter` 默认开启 `[SPEC-DEFER:phase-future.cjk-segmenter-default-on]`；既有 collection 升级自动 reindex `[SPEC-DEFER:phase-future.tokenizer-auto-reindex-on-upgrade]`；大语料 recall `[SPEC-DEFER:phase-future.tokenizer-large-corpus-recall]`；`RetrieverConfig.tokenizer` 真路由 `[SPEC-DEFER:phase-future.retriever-config-tokenizer-routing]`。ADR-029（默认开启维度兑现）/ ADR-035（D3 产品决策兑现）以 add-only Amendment 于 task-41.3 记录（不溯改正文，ADR-014 D5）；ADR-004（刻意默认变更例外承接 + safety intent 保持）/ ADR-008 / ADR-013 引用均不溯改其正文。

## Ratification（v0.34.0 / task-41.3）

本 ADR 于 v0.34.0 closeout（task-41.3）据 task-41.1/41.2 真实 CI（cargo-test / go-test / lint / spec-lint 四门绿）+ phase24 harness 实测 recall delta 逐 D ratify Proposed→Accepted。各 D 真实依据：

- **D1（production tokenizer 默认翻 `code_cjk`）→ Accepted 🟢 / 🟡**：task-41.1（PR #262，master @ `35bb421`）落 `core/src/server.rs` `resolve_tokenizer()`（pub fn）+ `parse_tokenizer()`（pub(crate) 纯函数）+ `CoreService::index`（:141）+ `jobs/index_session_backend.rs`（:151）改 `open_with_tokenizer(.., &resolve_tokenizer())`；`IndexSession::open`/`DEFAULT_TOKENIZER` 不动。`test_41_1_1_parse_tokenizer_flips_default_with_opt_out`（TEST-41.1.1，env 矩阵 unset→code_cjk / "default"→TEXT opt-out / unknown→code_cjk 不落 TEXT / cjk_segmenter feature 守护）+ `test_41_1_2_production_default_flip_and_existing_collection_safe`（TEST-41.1.2，生产路径新建 collection 绑 code_cjk camel 子词 'user' 命中 / opt-out 绑 TEXT 子词 miss / 既有 TEXT collection 经翻默认 open 仍保持 TEXT）全绿（lib 220→222）。**实测真实 recall delta（phase24_tokenizer_recall harness，当前 golden 14 files / 16 queries）**：before(default TEXT) recall@5/@10=0.8750 mrr=0.8750 → after(code_cjk) recall@5/@10=1.0000 mrr=0.9375，**delta recall@5/@10=+0.1250 mrr=+0.0625**（ADR-013 据实记录当前 golden 实测，非沿用 Phase 24 旧 golden 的 +0.0909——golden 自 Phase 24 已增长）。
- **D2（env opt-out + Go `[retrieval]` config 桥）→ Accepted 🟢**：task-41.2（PR #263，master @ `2cead8b`）`internal/config/config.go` add-only `RetrievalConfig{Tokenizer}` + `[retrieval]` 段 round-trip + `cmd/contextforge/main.go` `setTokenizerEnv`（镜像 `setVectorEnv`，env-wins，无段不导出）接线 doServe/doMCP；Rust core 0 toml dep。`TestTask412RetrievalConfig`（TEST-41.2.1，round-trip code_cjk/default/cjk_segmenter 保真 + 既有段不受影响 + 旧 config 向后兼容）+ `TestSetTokenizerEnv`（TEST-41.2.2，导出 / env-wins / 空段不导出→core 默认 code_cjk）全绿；`go test ./...` 全过；gofmt 0 diff；go vet clean。
- **D3（recall delta 复测 + honest-defer 边界）→ Accepted 🟡 / 🟢**：实测 +0.1250（小 golden caveat，大语料续 `[SPEC-DEFER:phase-future.tokenizer-large-corpus-recall]`）；jieba `cjk_segmenter` 默认不取据实延后 `[SPEC-DEFER:phase-future.cjk-segmenter-default-on]`（0-dep + Phase 30 delta=0）/ 既有 collection 自动迁移 `[SPEC-DEFER:phase-future.tokenizer-auto-reindex-on-upgrade]` / `RetrieverConfig.tokenizer` 路由 `[SPEC-DEFER:phase-future.retriever-config-tokenizer-routing]` 据实保持延后。
- **D4（首次刻意默认变更承接 + 0-dep / 0-network + opt-out byte-equiv）→ Accepted 🟢**：本 phase 0 新 dep（`code_cjk` 纯 std；jieba feature-gated）+ 0 网络；新建 collection `TEXT`→`code_cjk` 非 byte-equiv 由本 ADR 承接；既有 collection 经 `open_in_dir` 不受影响（TEST-41.1.2 守护）+ `CONTEXTFORGE_TOKENIZER=default`/`[retrieval]` opt-out 回 legacy byte-equiv + 不自动迁移；既有 `cargo test --workspace`（222 lib）+ `go test ./...` 三门不退化。smoke v31[50/50]（REAL 模式 camel 子词 `runner`(of JobRunner) 经 code_cjk 默认命中、legacy TEXT 会 miss，distinguishing）+ TestTask413（无 [37/37]..[49/49] 回归）。

真实 v0.34.0 tag/run/digest/tlog 经用户授权后由 post-tag-push backfill 填实（release docs `<backfill>`，ADR-013 不预填）。

## Alternatives

- **A1（不翻默认，保持 opt-in）**：保留 `code_cjk` 仅 opt-in。否决：ADR-029/035 D3 已把 tokenizer 默认化作为已识别 follow-up（`[SPEC-DEFER:phase-future.tokenizer-default-on]`），延后理由是「产品决策」非技术受阻；Phase 24 实测 +0.0909 真实收益；翻默认安全（既有 collection 不受影响 + opt-out + 不自动迁移）——本 phase 即做出该产品决策。
- **A2（改 `DEFAULT_TOKENIZER` 常量直接翻）**：把 `indexer/mod.rs:183` `DEFAULT_TOKENIZER = "default"` 改为 `"code_cjk"`。否决：会改 `IndexSession::open` 库便捷入口语义（影响所有库调用方）+ 破既有断言 `content_tokenizer_name == DEFAULT_TOKENIZER == "default"` 的单测 + 无 opt-out 通道。`resolve_tokenizer` env-resolution 在生产调用点是更 surgical 的翻默认 + opt-out（库 API / 单测不破）。
- **A3（默认翻 jieba `cjk_segmenter`）**：默认绑 jieba 真分词。否决：jieba 是 feature-gated 重词典 dep，默认化破 ADR-008 0-dep baseline；Phase 30 实测 jieba vs bigram `code_cjk` delta=+0.0000（小语料无增益）。默认取 0-dep `code_cjk` 捕获 +0.0909 收益；jieba 续 feature opt-in。
- **A4（既有 collection 升级时自动 reindex 到 `code_cjk`）**：upgrade 时静默重建既有 collection 索引。否决：静默 reindex 用户数据是惊讶 + 昂贵（Phase 18 记 100k 28s 重建）；既有 collection 保持 `TEXT` 仍可用，用户经既有 `reindex_with_tokenizer` 主动升级 → `[SPEC-DEFER:phase-future.tokenizer-auto-reindex-on-upgrade]`。
- **A5（翻默认但无 config opt-out 只 env）**：只提供 `CONTEXTFORGE_TOKENIZER` env、不加 Go `[retrieval]` config。否决：config.toml 是持久化、声明式的用户回退面（与 vector/reranker 段同构）；`[retrieval] tokenizer` 桥（env-wins）给用户声明式 opt-out / override，与既有 config 范式一致。
- **A6（本轮强行扩面做 jieba 默认 / 自动迁移 / RetrieverConfig.tokenizer 路由 / 大语料基准）**：一并实现所有相关 marker。否决：jieba 默认破 0-dep + 无增益、自动迁移改用户数据、`RetrieverConfig.tokenizer` 路由 ADR-035 D3 已定性 schema-driven 对称、大语料须更大语料工程——据 ADR-013 逐项据实分级、honest-defer，焦点版本不强行扩面（honest over padding）。

## 触及 ADR 关系

- **ADR-029（code-and-cjk-tokenizer-and-eval-hardening）→ add-only Amendment @ task-41.3**：其 §Negative / Follow-ups 以 `[SPEC-DEFER:phase-future.tokenizer-default-on]`（adr-029:54）记「tokenizer 默认开启 + 索引迁移工具」延后；本 phase D1 兑现默认开启维度（生产默认翻 `code_cjk` + opt-out + 既有 collection 安全）。以 `## Amendment (Phase 41 / v0.34.0)` add-only 记，**不溯改 ADR-029 正文**（ADR-014 D5）。
- **ADR-035（cjk-true-segmenter-and-tokenizer-default）→ add-only Amendment @ task-41.3**：其 D3 评估 tokenizer-default-on 后据「翻默认是产品决策」诚实延后 full default flip（`[SPEC-DEFER:phase-future.tokenizer-default-on]`）+ 记迁移工具已备 + schema-driven 对称；本 phase 兑现该产品决策（默认翻 `code_cjk`，jieba `cjk_segmenter` 仍 feature opt-in）。以 add-only Amendment 记，**不溯改 ADR-035 正文**（ADR-014 D5）。
- **ADR-004（local-first-privacy-baseline）→ 刻意例外由本 ADR 承接（不溯改）**：本 phase 是首次刻意默认行为变更（新建 collection `TEXT`→`code_cjk` 非 byte-equiv）；本 ADR D1/D4 显式承接该例外，ADR-004 的「opt-out 后 legacy byte-equiv + 不自动迁移用户数据」safety intent 据实保持，不溯改 ADR-004 正文。
- **ADR-008（dep add-only）→ 守线**：本 phase 加 **0 新依赖**（默认翻 `code_cjk` 纯 std；jieba `cjk_segmenter` 仍 feature-gated 不进默认构建）。
- **ADR-013（禁伪造红线）→ 守线**：recall delta +0.0909 真实实测非合成；刻意默认变更据实定性非夸大 byte-equiv；jieba 默认不取据实（0-dep + Phase 30 delta=0）；既有 collection 不自动迁移据实（D1 / D3 / D4）。
- **ADR-014（cross-phase-exit-criteria-validation）→ 第三十二次激活**：D1-D5 mapping + 各 task LAST D2 lint（touched 行 0 未标注命中）+ D3 verified-by + D4 自治 + D5 历史 Phase 1-40 不溯改（ADR 改动 add-only Amendment）；本 ADR ratify 在 task-41.3 closeout，Draft 阶段不 ratify。
