# ADR `035`: `cjk-true-segmenter-and-tokenizer-default`

**Status**: Accepted（v0.23.0 / task-30.3 ratify；D3 default flip honest-defer 部分 ratify）

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
- **Follow-ups**: 真分词器词典自定义 / 用户词典加载 `[SPEC-DEFER:phase-future.cjk-segmenter-user-dict]`；tokenizer 默认开启 default flip 续 `[SPEC-DEFER:phase-future.tokenizer-default-on]`（迁移工具已备，翻默认是产品决策）；扩充 golden 至跨语料规模（破小语料局限）`[SPEC-DEFER:phase-future.cjk-golden-corpus-expansion]`；多语种分词器（日 / 韩，lindera ko-dic 路径）`[SPEC-DEFER:phase-future.multilang-segmenter]`。ADR-029 以 add-only Amendment 记录真分词升级 + tokenizer-default-on 结论（task-30.3，不溯改正文，ADR-014 D5）；optional dep jieba-rs 经主 agent R7 chore + ADR-008 add-only（task-30.1）。

## Ratification (v0.23.0 / task-30.3)

本 ADR 于 v0.23.0 closeout（task-30.3）据 task-30.1/30.2 的**真实分词单测 / 真实扩 CJK golden recall delta** ratify **Proposed → Accepted**（ADR-013：禁据合成/伪造 ratify）。逐 D 真实依据：

- **D1（CJK 真分词器 behind `cjk-segmenter`，默认 0-dep）→ ✅ Accepted**：task-30.1（PR #202）经主 agent R7 chore（ADR-008 add-only）引入 optional `jieba-rs 0.7.4`（pure-Rust，无 C/build 前置；选 jieba 而非 lindera 因更轻）+ `cjk-segmenter` feature（默认 off）。`tokenize_cjk_segmenter` 经 `jieba.cut(run, false)` 对 CJK run 真分词；`cargo test --features cjk-segmenter --lib test_30_1` **2 passed**——实测 `配置加载 → [配置, 加载]`（真词边界），与 bigram `[配置, 置加, 加载]` `assert_ne!` 显式区分。jieba 0.7.4 在 Windows MSVC（rustc 1.95.0）build exit 0，纯 Rust 无构建受阻（未触发 lindera 重词典 🔴 风险）。
- **D2（parallel analyzer name + 双站点注册对称，保 bigram fallback）→ ✅ Accepted**：`CJK_SEGMENTER_TOKENIZER = "cjk_segmenter"` 并列 `code_cjk`，新 `build_cjk_segmenter_analyzer`/`register_cjk_segmenter` 在 index 站点（`open_with_tokenizer`）+ query 站点（`open_with_config`）双注册（feature-gated）；TEST-30.1.2 双站点 round-trip（IndexSession 写 + Retriever 查 `配置` 命中）PASS。bigram `code_cjk` 保留作 0-dep fallback。
- **D3（tokenizer-default-on 评估 + reindex 工具 + config 路由）→ 🟡 PARTIAL（迁移工具 Accepted；default flip honest-defer）**：task-30.2（PR #203）`IndexSession::reindex_with_tokenizer` 真实迁移工具（读 SQLite chunk + drop/重建 Tantivy 绑定 new analyzer + 重加，向后兼容）——TEST-30.2.2（default→code_cjk）+ 30.2.2b（→cjk_segmenter）PASS。config 采**方案 B schema-driven 对称**——`RetrieverConfig.tokenizer` doc 注明 vestigial（search 据 schema/`meta.json` 派生）。**default flip 本身据「迁移工具已备 + 翻默认是产品决策」诚实延后 `[SPEC-DEFER:phase-future.tokenizer-default-on]`，默认仍 opt-in，不伪造已翻**（ADR-013）。
- **D4（扩 CJK golden + 真实 recall delta）→ ✅ Accepted（含诚实零 delta 结论）**：task-30.2 扩 `golden-semantic.jsonl` +5 CJK case（11→16，经 Go `ValidateGoldenSemantic`）；phase24-style harness 实测（16 q / 14 file）：**default 0.8750/0.8750 → bigram 1.0/1.0 → segmenter 1.0/1.0**。**delta(seg−bigram)=+0.0000 全指标**——小语料 file-level 召回真分词与 bigram 持平（两者均完整召回 CJK case）；delta(seg−default)=+0.1250 recall。**诚实结论：小语料下真分词相对 bigram 无 file-level 召回提升**（真分词价值在 token 洁净/精度非此规模召回，bigram 0-dep fallback 仍有价值），如实记录零 delta、不外推、不伪造（ADR-013）。
- **D5（默认构建默认 tokenization 不变）→ ✅ Accepted**：默认 `cargo test --workspace` 0 failed + `cargo clippy --workspace --all-targets -- -D warnings` 0 warning；`cjk-segmenter` 默认 off → 不编译 jieba（0 新 dep at default features）；默认 `content` tokenization + 6-field schema 不变；bigram `code_cjk` opt-in 保留。

ratify 范围 = 真分词器 feature-gated（D1）+ parallel name 双站点对称（D2）+ reindex 迁移工具（D3 partial）+ 真实 recall delta 含诚实零 delta（D4）+ 默认 baseline 不变（D5）。**tokenizer default flip 据「已达维度 ratify + 受阻维度如实记录」honest-defer，不伪造**（ADR-013）。证据见 `docs/releases/v0.23.0-evidence.md` §3。

## Amendment (Phase 41 / v0.34.0, 2026-06-07 — tokenizer-default-on，add-only，正文 / D1-D5 / Ratification 不溯改)

本 ADR D3 把 **tokenizer default flip 据「迁移工具已备 + 翻默认是产品决策」诚实延后** `[SPEC-DEFER:phase-future.tokenizer-default-on]`（默认仍 opt-in）。**Phase 41（ADR-046，Accepted）做出该产品决策、兑现 default flip**，**不溯改本 ADR 正文 / D1-D5 / Ratification**（ADR-014 D5）：

- **D3 的 `[SPEC-DEFER:phase-future.tokenizer-default-on]` → 🟢 兑现（default flip）**：`code_cjk`（D1 的纯 std 0-dep bigram analyzer）从 opt-in 翻为**新建 collection 的生产默认**——task-41.1 `core/src/server.rs` `resolve_tokenizer()`（unset → `code_cjk` 翻默认 / `CONTEXTFORGE_TOKENIZER=default` opt-out 回 legacy `TEXT`）+ 生产索引两调用点 `open_with_tokenizer(.., &resolve_tokenizer())`；`IndexSession::open`/`DEFAULT_TOKENIZER` 不动。既有 collection 经 `open_in_dir` 读回持久化 schema **自动安全**（不被静默失效，TEST-41.1.2 守护）；既有 collection 升级由用户经 D3 的 `reindex_with_tokenizer` 主动触发（不自动迁移）。task-41.2 加 Go `[retrieval] tokenizer` config + `setTokenizerEnv` env 桥（opt-out / override 通道，env-wins，无段 → core 默认 `code_cjk`，Rust core 0 toml dep）。`RetrieverConfig.tokenizer` 仍按 D3 方案 B schema-driven 对称（vestigial 状态不改，续 `[SPEC-DEFER:phase-future.retriever-config-tokenizer-routing]`）。
- **默认翻 `code_cjk` 非 jieba `cjk_segmenter`（守 D5 0-dep baseline）**：默认翻纯 std `code_cjk` bigram；jieba `cjk_segmenter`（D1）仍 feature-gated opt-in——据本 ADR D4 实测「真分词 vs bigram delta=+0.0000」（小语料无增益）+ ADR-008 0-dep baseline，默认不取重词典 dep。default→code_cjk 实测 recall delta **+0.1250 recall@5/@10**（与 D4 测量 delta(seg−default)=+0.1250 一致）。
- **首次刻意默认变更由 ADR-046 承接**：新建 collection `TEXT`→`code_cjk` 非 byte-equiv，由 ADR-046 显式承接（三重安全 + 实测 justify）；本 ADR D5「默认构建默认 tokenization 不变」正文不溯改——Phase 41 的默认变更由 ADR-046 owned，本 amendment 仅 add-only 标 default flip 维度由 Phase 41 兑现。

依赖变更：0 新 dep（`code_cjk` 纯 std；jieba `cjk_segmenter` 仍 feature-gated 不进默认构建）。详见 ADR-046 Ratification + `docs/releases/v0.34.0-evidence.md`。
