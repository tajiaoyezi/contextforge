# ADR `035`: `cjk-true-segmenter-and-tokenizer-default`

**Status**: Proposed

**Category**: 检索 / 分词 / 召回质量
**Date**: 2026-06-02
**Decided By**: 主 agent（ADR-012 自治）；tajiaoyezi ratification at v0.23.0 closeout
**Related**: ADR-029 (code-and-cjk-tokenizer-and-eval-hardening — 本 ADR 承其 CJK bigram analyzer，以 add-only Amendment 记录真分词升级 + tokenizer-default-on 评估结论，不溯改正文) / ADR-004 (local-first-privacy-baseline — cjk-segmenter feature-gated，默认构建仍 0 新 dep / 0 network) / ADR-008 (core-library-selection — 真分词器 jieba-rs/lindera 为 optional dep add-only，实施时 R7 chore) / ADR-006 (recall-eval-acceptance-gate — recall delta 度量口径) / ADR-002 (sqlite+tantivy persistence — analyzer 绑定持久化 meta.json) / ADR-013 (real-data-only / 禁伪造凭据红线 — recall 数字真实跑出后回填) / ADR-014 (D1-D5 cross-phase exit gate，第二十一次激活) / roadmap §3.12

## Context

ContextForge 的核心用例是「让 AI agent 在代码 + 文档语料上检索」。Phase 24（retrieval-tokenizer-and-eval-hardening，ADR-029 Accepted）兑现了 opt-in 的 code/CJK 感知 tokenizer，但 CJK 处理刻意采务实起步形态——**overlapping bigram**，非真正的词分词器。截至 v0.22.0，实测现状（`core/src/indexer/mod.rs`）：

- 自定义 `CodeCjkTokenizer`（`tokenize_code_cjk:282-322`，bigram loop `290-308`，`is_cjk:186-196`）对 CJK 字符段发**重叠 bigram**：`配置加载` → `配置` / `置加` / `加载`。这是子串近似命中，非语义词边界——真正的词分词器（jieba-rs / lindera）应输出 `配置` / `加载`（丢掉跨词的 `置加` 噪声 token）。bigram 的精度受粒度限制，ADR-029 §Negative 已如实记录「非真正分词器，长尾精度优化属后续」，并以 `[SPEC-DEFER:phase-future.cjk-true-segmenter]`（ADR-029:54）留 follow-up。
- analyzer seam：`build_code_cjk_analyzer():364-369` 构建 analyzer，`register_code_cjk(index):373-377` 注册到 tokenizer manager。该名（`CODE_CJK_TOKENIZER = "code_cjk"`，`:183`）须在 **index 站点** `IndexSession::open_with_tokenizer:442` 与 **query 站点** `Retriever::open_with_config`（`retriever/mod.rs:250`）**两处**注册——否则 query 解析静默失败 → 召回退化（task-24.1 R4）。对称性由 schema 字段绑定（meta.json）+ 双站点注册共同驱动。
- tokenizer 仍是 **opt-in**：默认 `content` 字段走 Tantivy 默认 analyzer（`DEFAULT_TOKENIZER = "default"`，`:181`；默认分支 `build_tantivy_schema:162`，opt-in 分支 `:155`）。ADR-029 §Negative 记录「opt-in 切换需 re-index 才生效」，并以 `[SPEC-DEFER:phase-future.tokenizer-default-on]`（ADR-029:54）留 tokenizer 默认开启 + 索引迁移 follow-up。
- `RetrieverConfig.tokenizer`（`core/src/retriever/mod.rs:99`，Default `:110`）**vestigial（恒不读）**：search 路径 `QueryParser::for_index`（`retriever/mod.rs:325-328`）从 **schema 字段绑定** 派生 analyzer，不读 config。若要 config 驱动选择，必须真接线 `config.tokenizer` 路由到对应 register fn，或文档化 schema-driven 对称口径。
- recall delta 度量：`docs/spikes/phase-24-tokenizer-recall.md` + harness `core/examples/phase24_tokenizer_recall.rs` over `test/fixtures/eval/golden-semantic.jsonl`（11 题 = 6 code-symbol + 5 cjk）；Go 校验器 `internal/eval/eval.go::ValidateGoldenSemantic:231-280` + `knownCategories:214-223`（含 cjk）。Phase 24 真实 delta = +0.0909，**由单个 cjk case 驱动**——语料小（11 q / 12 files），不足以背书真分词器的真实增益，须扩 CJK case 才有意义 delta。

本 ADR 记录三块的处理策略：CJK 真分词器的注册方式（feature-gated，守 0-dep fallback）+ analyzer seam 对称口径 + tokenizer 默认开启评估（含既有索引迁移工具 + config 路由接线）+ 扩充 CJK golden 的真实 recall delta 度量口径。**默认构建仍 0 新 dep + 默认 tokenization 不变**（ADR-004），真实 recall 数字真实跑出后回填、不预填（ADR-013）。

## Decision

CJK 真分词 + tokenizer 默认开启采用 **feature-gated 真分词器（守 0-dep bigram fallback）+ 双站点对称注册 + 默认开启诚实评估 + 扩充 CJK golden 真实 delta 背书** 策略：

### D1 — CJK 真分词器 behind `cjk-segmenter` feature；默认 0-dep（task-30.1）

在新 core feature `cjk-segmenter`（默认 off → 0 新 dep，镜像 `core/Cargo.toml` 既有 `vector-lancedb`/`embedding-remote` gating 配方：`[features]` 默认 `default = []`、optional dep + `cfg(feature = "cjk-segmenter")` gate）后挂一个 **真正的 CJK 词分词器** analyzer，对多字 CJK 短语切**词边界**（`配置加载` → `配置` / `加载`），区别于 bigram（`配置` / `置加` / `加载`）。分词库选型 jieba-rs（纯 Rust 词典，较轻 🟡）vs lindera（内嵌 IPADIC/ko-dic 词典，较重 🔴）于 task-30.1 据真实编译 / 体积权衡定，optional dep 经主 agent R7 chore + ADR-008 add-only 引入（**本 ADR 为规划，仅记录此路径，不加 dep**）。真分词 token stream 由 deterministic 单测断言真实词边界。

**理由**：bigram 务实但非真分词，跨词 `置加` 噪声 token 损害精度；真分词器对 CJK 召回精度的提升须真实 delta 背书。feature-gated 守 ADR-004——默认构建不编译该 feature、仍 0 新 dep；dep add-only 走 ADR-008（实施时 R7 chore，非 subagent 自改）。备选「in-place 替 bigram 于 `build_code_cjk_analyzer:364-369`」会丢掉 0-dep fallback，故不取（见 D2）。

### D2 — analyzer seam：PARALLEL analyzer name + 双站点注册对称，保 bigram 作 0-dep fallback（task-30.1）

采**并行 analyzer name**（如 `CJK_SEGMENTER_TOKENIZER = "cjk_segmenter"`）+ 新 build analyzer fn + register fn，**在 index 站点 `open_with_tokenizer:442` 与 query 站点 `open_with_config`（retriever/mod.rs）`:250` 两处注册**——保留既有 bigram（`code_cjk`）作默认 0-dep fallback、真分词作 feature 升级；而非在 `build_code_cjk_analyzer:364-369` 内 in-place 替换。

**理由**：parallel name 让 bigram 仍是 feature 关闭时的默认 0-dep fallback、真分词作可选升级，两者并存不互斥；新 analyzer name 必须双站点注册，否则 query 解析（`QueryParser::for_index`）从 schema 绑定派生时找不到该 analyzer → 静默失败 → 召回退化（task-24.1 R4 的复现红线）。对称性是 schema 绑定（meta.json）+ 双注册共同保证，单站点注册不足。

### D3 — tokenizer-default-on 评估 + 既有索引 reindex/migration 工具 + `RetrieverConfig.tokenizer` 路由接线（task-30.2）

评估把 tokenizer 从 opt-in 翻到 **默认开启**：(a) 默认 analyzer 绑定变更须 **re-index**（绑定持久化于 tantivy `meta.json`），故须提供既有索引 reindex/migration 工具；(b) `config.tokenizer` 现 vestigial（`retriever/mod.rs:99` 恒不读），若要 config 驱动选择须**真接线**到对应 register fn 路由，**或** 文档化 schema-driven 对称口径（search 从 schema 字段绑定派生 analyzer）；(c) 既有默认索引不得被破坏（向后兼容）。**若全量 default-on 迁移过重，则诚实延后 default flip、保留 opt-in + migration 工具** `[SPEC-DEFER:phase-future.tokenizer-default-on]`。

**理由**：默认开启的真实代价是既有索引须 re-index（schema 绑定持久化口径，非配置开关即生效）；config.tokenizer vestigial 是真实事实，要么真接线要么文档化 schema-driven，不可假装 config 已驱动。迁移面过重时诚实延后 default flip 优于强行翻默认破坏既有索引（ADR-013：受阻不伪造）。

### D4 — 扩充 CJK golden + 真实 recall delta（ADR-013 不预填）（task-30.2）

扩充 `test/fixtures/eval/golden-semantic.jsonl` 的 CJK case（过 Go `ValidateGoldenSemantic:231-280` schema/dup/category 校验），经 phase24-style harness（`core/examples/phase24_tokenizer_recall.rs`）度量 **default vs bigram vs 真分词器** 的真实 before/after recall delta（**数字真实跑出后回填、绝不预填**，ADR-013；小语料 caveat 如实记录、不外推）。

**理由**：现 golden 仅 5 CJK case、Phase 24 delta 由单 case 驱动，不足以背书真分词器的真实增益；扩 CJK case 才有意义 delta。真分词 vs bigram 的真实增益是 Proposed→Accepted ratify 的核心证据，须真实跑出（待实测回填），不强行预设数值。

### D5 — 默认构建默认 tokenization 不变（all tasks）

D1/D2 的真分词器（`cjk-segmenter` feature 默认不编译）、D3 的 default-on 评估、D4 的 golden 扩充 **全部不破坏默认 baseline**：默认 `content` tokenization 不变（既有索引不失效）、默认 6-field schema 不变、默认构建 0 新 dep（`cjk-segmenter` 默认 off → optional dep 不进编译）；`cargo test --workspace` 默认 feature 集不受影响。

**理由**：ADR-004 本地优先 / 隐私 / 0-dep 基线不变是硬约束；feature-gating 让真分词器与重词典 dep 仅在显式 opt-in 时进入，默认用户零成本。default-on 翻默认若发生（D3）须经 re-index 迁移工具承载，不静默失效既有索引。

## Consequences

- **Positive**: CJK 检索从 bigram 子串近似升级到真词边界（feature-gated `cjk-segmenter`），精度增益有真实 before/after recall delta 背书（ADR-013）；parallel analyzer name + 双站点注册保 bigram 作 0-dep fallback、真分词作可选升级，两者并存；tokenizer-default-on 经诚实评估给出可行结论（翻默认 + 迁移工具 或 诚实延后），config.tokenizer vestigial 现状被真接线或文档化收敛；扩充 CJK golden 让真分词增益可持续度量；默认构建保持 0 新 dep + 默认 tokenization 不变（既有 collection 不失效）。
- **Negative / open**: 真分词器引入重词典 dep（lindera 内嵌 IPADIC/ko-dic ~🔴 大；jieba-rs 纯 Rust 词典 ~🟡），即便 feature-gated，opt-in 用户的二进制体积 / 编译时长上升（如实记录真实体积 delta，不夸大）；真实 recall delta 受扩充 CJK golden 覆盖度限制——小语料（现 11 q / 12 files）delta 可能仍偏弱或由少数 case 驱动，如实记录不外推（ADR-013）；tokenizer-default-on 的全量迁移面可能过重 → 诚实延后 default flip（受阻维度如实记录，不伪造「默认已翻」）；真分词器 token 边界依赖词典 recall，词典外新词（OOV）可能切碎，属词典固有局限。
- **Ratification**: 本 ADR Proposed。task-30.1（真分词 token stream 单测 + 双站点对称 + 默认构建不变）+ task-30.2（扩充 CJK golden 真实 recall delta + reindex/migration 工具 + config 路由接线或 schema-driven 文档化）通过后于 v0.23.0 closeout 据真实分词单测 / 真实 recall delta ratify Proposed→Accepted（ADR-013：禁据合成 / 伪造 ratify）；重词典 dep / 小语料 / tokenizer-default-on 等受阻维度据「已达维度 ratify + 受阻维度如实记录」处理，不强 ratify。
- **Follow-ups**: 真分词器词典自定义 / 用户词典加载 `[SPEC-DEFER:phase-future.cjk-segmenter-user-dict]`；若 D3 诚实延后则 tokenizer 默认开启续 `[SPEC-DEFER:phase-future.tokenizer-default-on]`；扩充 golden 至跨语料规模（破小语料局限）`[SPEC-DEFER:phase-future.cjk-golden-corpus-expansion]`；多语种分词器（日 / 韩，lindera ko-dic 路径）`[SPEC-DEFER:phase-future.multilang-segmenter]`。ADR-029（CJK 分词策略）以 add-only Amendment 记录真分词升级 + tokenizer-default-on 结论（task-30.3，不溯改 ADR-029 正文，ADR-014 D5）；若 D1 引入 optional dep，则 ADR-008 add-only Amendment 记录依赖选型（task-30.3）。
