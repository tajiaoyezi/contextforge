# Phase 30 · cjk-true-segmenter

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 承 Phase 24(retrieval-tokenizer-and-eval-hardening, Done)：现 `code_cjk` analyzer 对 CJK 采**重叠 bigram**（`配置加载` → `配置`/`置加`/`加载`，非真分词），把这一务实起步**升级为真分词器**（true word segmenter，`配置加载` → `配置`/`加载`），并评估把 tokenizer 由 opt-in 翻为**默认开启**（含既有索引 reindex/migration 工具），在扩展后的 CJK golden 上量出**真实召回 delta**。真分词器为 **feature-gated 升级**（新 `cjk-segmenter` feature，默认 off → 默认构建仍 0 新 dep，守 ADR-004），bigram 保留作默认 0-dep fallback。三类验证诚实分层：🟢 deterministic 分词单测（CI 可验，无 live dep）/ 🟡 真实小语料 recall delta（须 feature 构建 + 本地真实跑，ADR-013 不预填）/ 🔴 重词典 dep（jieba-rs / lindera，dep 经主 agent R7 chore + ADR-008 add-only，受阻如实记录）。全部限于 `core/src/indexer/mod.rs` analyzer seam + `core/Cargo.toml` feature-gating + eval golden/harness；默认 analyzer + 6-field schema + 既有索引不变（ADR-004）；既有 `cargo-test` / `go-test` / `spec-lint` 三门不退化。对应 `docs/roadmap.md §3.12`。
>
> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md §3.12`（cjk-true-segmenter + tokenizer-default-on 候选 + 两 marker 出处）→ `core/src/indexer/mod.rs`（analyzer seam：`is_cjk` :186-196 / `tokenize_code_cjk` bigram loop :282-322 / `build_code_cjk_analyzer` :364-369 / `register_code_cjk` :373-377 / `open_with_tokenizer` :416-458 注册站点 :442；常量 `DEFAULT_TOKENIZER`=`default` :181 / `CODE_CJK_TOKENIZER`=`code_cjk` :183）→ `core/src/retriever/mod.rs`（`RetrieverConfig.tokenizer` :99 + Default :110 **vestigial 现状**——search 路径 `QueryParser::for_index` :325-328 据 schema 字段绑定解析 analyzer 不读 config；query 站点 `register_code_cjk` :250）→ `core/Cargo.toml`（feature-gating recipe：`[features]` :115-132 / `default = []` :116 / `vector-lancedb = [dep:lancedb,...]` :120 / `embedding-remote = [dep:ureq]` :131 / optional dep `ureq` :107）→ `docs/spikes/phase-24-tokenizer-recall.md` + `core/examples/phase24_tokenizer_recall.rs`（recall harness）+ `test/fixtures/eval/golden-semantic.jsonl`（11 行：6 code-symbol + 5 cjk）+ `internal/eval/eval.go ValidateGoldenSemantic` :231-280 / `knownCategories` :214-223 → AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-035-cjk-true-segmenter-and-tokenizer-default.md`（D1-D5）→ `docs/decisions/adr-029-code-and-cjk-tokenizer-and-eval-hardening.md`（:54 Follow-ups + :66 ratification scope，本 phase add-only Amendment）→ `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5，第二十一次激活）→ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造红线：真实分词单测 / 真实 recall delta，受阻如实记录不伪造）。
>
> **ADR 影响面（已识别）**：
> - **ADR-035 cjk-true-segmenter-and-tokenizer-default（新，Proposed）**：记真分词器 feature-gated 升级（D1）+ analyzer seam 并行命名 + 双站点注册对称（D2）+ tokenizer-default-on 评估 + 既有索引 reindex/migration + `RetrieverConfig.tokenizer` 路由接线（D3）+ 扩展 CJK golden + 真实 recall delta（D4，ADR-013 不预填）+ 默认构建 default tokenization 不变（D5）。落地后据真实分词单测 / 真实 recall delta ratify；重词典 dep / 小语料受阻维度据「已达维度 ratify + 受阻维度如实记录」处理，不强 ratify、不伪造。
> - 触及 **ADR-029（code-and-cjk-tokenizer-and-eval-hardening）**：真分词替/补 bigram + tokenizer-default-on 评估属 ADR-029:54 Follow-ups + :66 ratification scope 所留 marker——以 add-only Amendment 记录升级结果，不溯改 ADR-029 §Decision 正文 D1-D5（ADR-014 D5）。
> - 触及 **ADR-008（dependency-policy）**：真分词器引入新 optional dep（jieba-rs / lindera）属 add-only——实施时经主 agent R7 chore + ADR-008 add-only Amendment（**非 subagent 自编辑**；本 phase 为规划，仅 NOTE 不加 dep）。
> - 触及 **ADR-004（local-first-privacy-baseline）**：`cjk-segmenter` feature 默认 off → 默认构建仍 0 新 dep / 0-network；真分词为 feature 升级（守线，非推翻）。

## 1. 阶段目标

Phase 24 ship 后，ContextForge 的 `code_cjk` analyzer 对 CJK 采**重叠 bigram**（务实 0-dep 起步），非真分词；本 phase 把它**升级为真词分词器**（feature-gated，默认 0-dep）、评估 tokenizer 默认开启（含既有索引迁移/重建工具）、并在扩展后的 CJK golden 上量出真实召回 delta。真分词器经新 `cjk-segmenter` feature 守护（默认 off → 默认构建 0 新 dep，镜像 ADR-004），bigram 保留为 0-dep fallback；默认 analyzer + 6-field schema + 既有默认索引不变；既有 `cargo-test` / `go-test` / `spec-lint` 三门不退化。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. 新 `cjk-segmenter` feature 下，真分词 analyzer 把多字 CJK 短语切成**词**（`配置加载` → `配置`/`加载`，区别于 bigram `配置`/`置加`/`加载`）；新 analyzer 名在**索引站点** `open_with_tokenizer:442` 与**查询站点** `open_with_config:250` 双注册对称，opt-in collection round-trip 查询可解析 + 命中（无静默召回退化，task-24.1 R4）；默认构建（无 `cjk-segmenter`）default tokenization + 6-field schema + 0 新 dep 不变；重词典 dep（jieba-rs / lindera）经主 agent R7 chore + ADR-008 add-only（本 phase 规划仅 NOTE）（AC1）
2. 评估 tokenizer 默认开启：提供既有索引 reindex/migration 工具（默认 analyzer 绑定持久化于 tantivy `meta.json`，翻默认须 re-index）+ 把 vestigial `RetrieverConfig.tokenizer`:99 真接线为路由（选哪个 register fn）**或**文档化 schema-driven 对称；在扩展后的 CJK golden（经 Go `ValidateGoldenSemantic` 校验）上量出 default vs bigram vs 真分词的**真实 recall delta**（真实跑出后回填，小语料不外推，ADR-013）；既有默认索引不破坏（向后兼容）（AC2）
3. 默认构建 0-dep / default tokenization 维持不变 + v0.23.0 closeout：release docs + `scripts/console_smoke.sh` step `[39/39]`（default build init baseline + default tokenization 不变断言 + 既有 step 不退化）+ phase §6 闭合 + ADR-035 据真实分词单测 / 真实 recall delta ratify（受阻维度如实记录）+ ADR-029 add-only Amendment（AC3）
4. ADR-014 D1-D5（第二十一次激活）全通过（AC4）

**v0.x 版本号决策**：v0.23.0 minor release（Phase 30，承 v0.22.0；roadmap §1.1 Phase N→v0.(N-7).0；新 `cjk-segmenter` feature opt-in，默认构建 0 新 dep + 0 网络、default analyzer + 6-field schema 不变、不破坏既有 v0.6-v0.22 client 与既有默认索引）。

## 2. 业务价值

兑现 ADR-029:54 Follow-ups + phase-24 spec :41/:42 / task-24.1 :35/:36 / task-24.3 :39/:40 一路刻意延后的两项 marker，补齐 CJK 检索精度与默认开启缺口：

- **cjk-true-segmenter**：Phase 24 为守 0-dep 选了重叠 bigram（`配置加载` → `配置`/`置加`/`加载`），跨词边界的伪 token（`置加`）稀释精度、真词（`配置`/`加载`）与 bigram 不完全对齐。真分词器（jieba-rs / lindera）输出真词边界，提升 CJK 召回精度——以 feature-gated 方式提供，守 0-dep 默认构建（`[SPEC-DEFER]` 已留于 ADR-029:54）。
- **tokenizer-default-on**：现 `code_cjk` 为 opt-in（默认 collection 不享代码/CJK 子词命中）。评估默认开启需既有索引 re-index（schema 绑定持久化 meta.json）+ `RetrieverConfig.tokenizer` 现 vestigial 须真接线。本 phase 评估并提供迁移工具；若全量默认翻转太重则诚实保留 opt-in + 迁移工具（phase-24 spec :42 已留 marker）。
- **真实 recall delta**：现 golden 仅 5 CJK case、Phase 24 实测 delta（+0.0909）由**单个** cjk case 驱动、语料仅 11 q / 12 file——扩 CJK case 才有意义 delta。本 phase 扩展 CJK golden 并量真实 delta（不外推、不预填，ADR-013）。

**不在本 phase scope**：

- 真分词器多语言扩展（日文/韩文 lindera ko-dic / ja-dic 全量）[SPEC-DEFER:phase-future.cjk-multilingual-segmenter]
- 用户自定义词典 / 领域词扩充 [SPEC-DEFER:phase-future.cjk-user-dictionary]
- 向量侧（embedding）CJK 分词协同（本 phase 仅倒排 BM25 analyzer）[SPEC-DEFER:phase-future.cjk-embedding-tokenization]
- 大语料（>1k q）recall 评测台 [SPEC-DEFER:phase-future.large-corpus-eval-harness]
- 若全量 default-on 迁移过重则 default flip 本身延后 [SPEC-DEFER:phase-future.tokenizer-default-on]

## 3. 涉及模块

### 30.1 cjk-true-segmenter：feature-gated 真分词 analyzer（task-30.1）

- 新增 `core/Cargo.toml` `[features]` `cjk-segmenter`（默认 off，镜像 `vector-lancedb` :120 gating recipe）+ optional dep（jieba-rs / lindera 二选一，`optional = true`，镜像 `ureq` :107）——**dep 添加经主 agent R7 chore + ADR-008 add-only，本 task 规划仅 NOTE，不自加 dep**
- 修改 `core/src/indexer/mod.rs`——并行 analyzer 名 `CJK_SEGMENTER_TOKENIZER = cjk_segmenter`（区别 `CODE_CJK_TOKENIZER` :183）+ 新 `build_cjk_segmenter_analyzer()` + `register_cjk_segmenter(index)`，经 `#[cfg(feature = "cjk-segmenter")]` 门控；bigram `build_code_cjk_analyzer` :364-369 保留作 0-dep fallback
- 双站点注册：新 analyzer 名须在索引站点 `open_with_tokenizer:442` 与查询站点 `open_with_config`（`retriever/mod.rs`）:250 **同时**注册（否则 query 解析静默失败 → 召回退化，task-24.1 R4）
- deterministic 分词单测（`#[cfg(feature = "cjk-segmenter")]`）：真词边界（`配置加载` → `配置`/`加载`），与 bigram（`配置`/`置加`/`加载`）token stream 显式区分
- 同源验证（≥2，🟢 CI 可验：`cargo test --features cjk-segmenter` 真分词 token stream + opt-in round-trip 命中 / `cargo test --workspace`（无 feature）默认不变）

### 30.2 tokenizer-default-on 评估 + 既有索引迁移 + 扩展 CJK recall delta（task-30.2）

- 评估 tokenizer 默认开启：默认 analyzer 绑定持久化于 tantivy `meta.json`，翻默认须既有索引 **re-index** → 提供 reindex/migration 工具（既有默认 collection 重建到新 analyzer 绑定，向后兼容）
- 接线 `RetrieverConfig.tokenizer`（`retriever/mod.rs:99`，现 vestigial 从不被 search 读）为**路由**——据 config 选调哪个 register fn / analyzer 名，**或**文档化 schema-driven 对称（search 据 schema 字段绑定解析 analyzer，非 config）
- 扩展 `test/fixtures/eval/golden-semantic.jsonl` CJK case（经 Go `ValidateGoldenSemantic` :231-280 / `knownCategories` :214-223 校验 schema/dup/category）
- 经 phase24-style harness（`core/examples/phase24_tokenizer_recall.rs`）量 default vs bigram vs 真分词的**真实 recall delta**（真实跑出后回填，小语料不外推，ADR-013）
- 若全量 default-on 迁移过重则诚实保留 opt-in + 迁移工具 `[SPEC-DEFER:phase-future.tokenizer-default-on]`
- 同源验证（≥2：🟡 真实 recall delta 本地跑 + 🟢 迁移工具 round-trip / 既有默认索引向后兼容 / Go validator pass）

### 30.3 v0.23.0 closeout（task-30.3）

- 修改 `scripts/console_smoke.sh`——banner v19→v20 + v20 changelog 块 + step `[39/39]`（doc/status step：default build init baseline + default tokenization 不变断言，`cjk-segmenter` feature-gated 无 console-api 运行时面 → 文档/状态步；既有 step 不退化 + denominator 不溯改 ADR-014 D5）
- 修改 `internal/cli/smoke_syntax_test.go`——新 Test 断言 `[39/39]` + 既有 step 无回归
- 新增 `docs/releases/v0.23.0-{evidence,artifacts}.md`（tag SHA / run id / digest 用 angle-bracket backfill 待回填）+ `README.md` v0.23 段 + `RELEASE_NOTES.md` v0.23.0 段
- 修改 `docs/decisions/adr-035-cjk-true-segmenter-and-tokenizer-default.md`——Status Proposed→Accepted（per-D 限定，诚实：重词典 dep / 小语料受阻维度据真实证据部分 ratify）+ `## Ratification（v0.23.0 / task-30.3）` 节
- 修改 `docs/decisions/adr-029-code-and-cjk-tokenizer-and-eval-hardening.md`——append `## Amendment (Phase 30 / v0.23.0)`（记真分词升级 + tokenizer-default-on 结果，不溯改正文 D1-D5）+ 若加 dep 则 ADR-008 add-only note
- 修改 `docs/specs/phases/phase-30-cjk-true-segmenter.md`——Status Draft→Done + §6 AC 逐维诚实勾选
- 修改 `docs/s2v-adapter.md`——Phase 30 行 + Task 行 + ADR-035 行 + BDD 行

### BDD feature

- 新增 `test/features/phase-30-cjk-true-segmenter.feature`（≥4 scenario：真分词 token stream + 双站点对称 / 默认构建不变 + 0-dep / tokenizer-default-on 评估 + 迁移 + 真实 recall delta / v0.23.0 收口）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 30.1 | `core/Cargo.toml` `cjk-segmenter` feature + optional dep（NOTE，ADR-008 add-only）+ `core/src/indexer/mod.rs` 并行 `cjk_segmenter` analyzer + 双站点注册（:442 + retriever :250）+ deterministic 真词边界单测；bigram 保留 0-dep fallback | `../tasks/task-30.1-cjk-true-segmenter.md` |
| 30.2 | tokenizer-default-on 评估 + 既有索引 reindex/migration 工具 + `RetrieverConfig.tokenizer`:99 路由接线（或文档化 schema-driven）+ 扩展 CJK golden（Go validator）+ phase24-harness 真实 recall delta（不预填，ADR-013） | `../tasks/task-30.2-tokenizer-default-on-and-cjk-recall-delta.md` |
| 30.3 | smoke v20 step `[39/39]` + v0.23.0 closeout + ADR-035 ratify + ADR-029 add-only Amendment | `../tasks/task-30.3-closeout-v0.23.0.md` |

## 5. 依赖关系

- **task-30.1**（真分词 analyzer）dep 既有 `indexer/mod.rs` analyzer seam（`build_code_cjk_analyzer:364-369` / `register_code_cjk:373-377` / 双站点 :442 + retriever :250）+ `core/Cargo.toml` feature-gating recipe（:115-132）；optional dep 经主 agent R7 chore + ADR-008 add-only（前置，本 task 规划 NOTE）；可独立先行。
- **task-30.2**（default-on 评估 + recall delta）建议 30.1 先 merge（真分词 analyzer 落地后才能量真分词 recall delta）+ dep 既有 `RetrieverConfig.tokenizer:99` + golden/harness + Go `ValidateGoldenSemantic`。
- **task-30.3**（closeout）dep 30.1 + 30.2 全 Done；release docs / smoke v20 / ADR-035 ratify 据两 task 真实分词单测 / 真实 recall delta。
- 外部：ADR-035（本 phase 新 Proposed）/ ADR-029（code-and-cjk-tokenizer-and-eval-hardening，本 phase add-only Amendment 记升级结果，不溯改正文）/ ADR-008（新 optional dep add-only，实施时 R7 chore）/ ADR-004（默认构建 0-dep / 0-network baseline 不变）/ ADR-012（tag/release 主 agent 自治触发，outward-facing 不可逆须用户显式授权）/ ADR-014 第二十一次激活 / ADR-013（禁伪造红线，真实分词单测 / 真实 recall delta，受阻不伪造）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（每条 AC 含 ADR-014 D3 verified by 显式 owner；Draft 阶段未勾选，实施后逐条诚实置 `[x]`）**：

- [x] AC1（真分词器 feature 🟢 分词 / 🔴 dict dep）: 新 `cjk-segmenter` feature（jieba-rs 0.7.4）下真分词 analyzer 把 `配置加载` 切成 `配置`/`加载`（区别 bigram `配置`/`置加`/`加载`，`assert_ne!`）+ 新 analyzer 名双站点注册对称（索引 `open_with_tokenizer` + 查询 `open_with_config`）opt-in round-trip 命中无静默退化（task-24.1 R4）+ 默认构建（无 feature）default tokenization + 6-field schema + 0 新 dep 不变（🟢 deterministic）；jieba-rs 经主 agent R7 chore + ADR-008 add-only（pure-Rust，无重词典构建受阻，未触发 lindera 🔴 风险）— verified by TEST-30.1.1/30.1.2/30.1.3（#202，2+默认 PASS）
- [x] AC2（tokenizer-default-on + migration + 真实 recall delta 🟡）: `IndexSession::reindex_with_tokenizer` 既有索引迁移工具（绑定持久化 meta.json，读 SQLite chunk 重建）+ `RetrieverConfig.tokenizer` schema-driven 对称文档化（方案 B vestigial）+ 既有默认索引向后兼容（reindex round-trip PASS）+ 扩展 CJK golden（11→16）经 Go `ValidateGoldenSemantic` 校验 + phase24-harness 实测 default 0.875 / bigram 1.0 / 真分词 1.0——**delta(seg−bigram)=+0.0000 诚实零**（小语料 file-level 持平，不外推，ADR-013）；full default-on flip 诚实延后 [SPEC-DEFER:phase-future.tokenizer-default-on]（迁移工具已备）— verified by TEST-30.2.1/30.2.2/30.2.2b/30.2.3（#203 PASS）
- [x] AC3（默认 0-dep 不变 + v0.23.0 closeout）: 默认构建 default tokenization + 6-field schema + 0 新 dep 维持不变（ADR-004，`cargo test --workspace` 0 failed）+ v0.23.0 release docs（evidence/artifacts/README/RELEASE_NOTES，tag/run/digest backfill）+ `scripts/console_smoke.sh` step `[39/39]`（default build init baseline + default tokenization 不变 + 既有 step 不退化 ADR-014 D5）+ phase §6 闭合 + ADR-035 据真实分词单测 / 真实 recall delta per-D ratify（D3 default flip honest-defer 如实）+ ADR-029 add-only Amendment（不溯改正文）— verified by TEST-30.3.1/30.3.2
- [x] AC4（ADR-014）: ADR-014 cross-validation gate 全套通过（第二十一次激活）— D1 mapping + D2 lint `--touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-29 不溯改 — verified by TEST-30.3.3 + closeout PR body

**端到端 smoke（C1 集成兜底）**：(1) `cargo test --features cjk-segmenter` 真分词 token stream（`配置加载`→`配置`/`加载`）+ 双站点对称 opt-in round-trip 命中 PASS（🟢）；(2) 扩展 CJK golden 经 Go `ValidateGoldenSemantic` PASS + phase24-harness 在扩展 golden 上量出 default vs bigram vs 真分词真实 recall delta（🟡 本地真实跑，数字真实跑出后回填、不预填）+ 既有索引 reindex/migration 工具 round-trip 向后兼容；(3) `cargo test --workspace`（无 feature）默认 tokenization + 6-field schema + 0 新 dep 不变 + smoke step `[39/39]` default build baseline 不变 全 PASS（受阻态如实标注，如重词典 dep 受阻 / default flip 延后）。

## 7. 阶段级风险

- **R1（高）重词典 dep（lindera / jieba-rs）体量 / 构建成本**：lindera 内嵌 IPADIC / ko-dic 词典体量大（~🔴），jieba-rs 纯 Rust 词典较轻（~🟡）；引入 direct dep 须经 ADR-008 add-only。
  - **缓解**：task-30.1 经 `cjk-segmenter` feature 门控（默认 off → 默认构建 0 新 dep，守 ADR-004）；优先评估较轻的 jieba-rs；dep 添加经主 agent R7 chore + ADR-008 add-only（**非 subagent 自编辑**）。stop-condition：若重词典 dep 在本机 / CI 构建不可行则 AC1 真分词维度不标 `[x]`（feature seam + bigram fallback 达成则部分 ratify，不伪造「真分词成功」，ADR-013）。
- **R2（高）双站点注册不对称致 query 静默召回退化**：新 analyzer 名若只在索引站点 `:442` 注册、查询站点 `:250` 漏注册，则 `QueryParser::for_index` 据 schema 解析 analyzer 时静默 fallback → 召回退化（task-24.1 R4）。
  - **缓解**：task-30.1 deterministic round-trip 测试断言 opt-in collection 查询解析 + 命中（双站点对称）；并行命名 + 双注册同 task-24.1 既有 `register_code_cjk` 模式。stop-condition：round-trip 命中不过则 AC1 对称维度不标 `[x]`。
- **R3（中）tokenizer-default-on 既有索引迁移过重**：翻默认 analyzer 须既有索引 re-index（绑定持久化 meta.json）；`RetrieverConfig.tokenizer:99` 现 vestigial（search 路径从不读）须真接线或文档化。
  - **缓解**：task-30.2 提供 reindex/migration 工具 + 真接线 config 路由 **或** 文档化 schema-driven 对称；若全量 default flip 过重则诚实保留 opt-in + 迁移工具 `[SPEC-DEFER:phase-future.tokenizer-default-on]`（不强翻默认）。stop-condition：既有默认索引向后兼容不过则不标 `[x]`。
- **R4（中）小语料 recall delta 不显著 / 由单 case 驱动**：现 golden 仅 5 CJK case、Phase 24 delta（+0.0909）由单 case 驱动、语料 11 q / 12 file 过小。
  - **缓解**：task-30.2 扩展 CJK golden case（经 Go `ValidateGoldenSemantic` 校验）后再量真实 delta；数字真实跑出后回填、小语料不外推（ADR-013）。stop-condition：delta 不显著 / 负向则如实记录真实数字（不为「真分词更好」伪造正 delta），据真实结果 ratify ADR-035 D4。

## 8. Definition of Done

- 3 task spec（30.1-30.3）顶部 `**Status**: Done`
- §6 阶段级 AC1-4 全 `[x]`（受阻态按 ADR-013 如实记录，不伪造——如重词典 dep 受阻 / default flip 延后 / recall delta 不显著据真实数字）
- 端到端 smoke 3 step 全 PASS（含受阻态如实标注）
- **ADR**：ADR-035 `Proposed → Accepted`（据真实分词单测 / 真实 recall delta：真词边界 token stream / default vs bigram vs 真分词真实 delta）或据实测受阻记录维持 + 文档化；ADR-029 经 add-only Amendment 记录真分词升级 + tokenizer-default-on 结果（不溯改正文，ADR-014 D5）；若加 dep 则 ADR-008 add-only note
- **adapter**：§Phase 索引 Phase 30 `Draft → Done` + `Tasks 0 → 3`；§ADR 索引 ADR-035；§BDD 追加 phase-30 feature 行；ADR-029 Amendment 记录
- **release**：`docs/releases/v0.23.0-{evidence,artifacts}.md` + `RELEASE_NOTES.md` v0.23 段 + README v0.23 段
- **smoke**：`scripts/console_smoke.sh` step `[39/39]`（default build baseline + default tokenization 不变 smoke + 既有 step 不退化）+ `internal/cli/smoke_syntax_test.go` markers 同步
- **follow-up**：CJK 多语言分词 `[SPEC-DEFER:phase-future.cjk-multilingual-segmenter]` + 用户自定义词典 `[SPEC-DEFER:phase-future.cjk-user-dictionary]` + 向量侧 CJK 分词 `[SPEC-DEFER:phase-future.cjk-embedding-tokenization]` + 大语料评测台 `[SPEC-DEFER:phase-future.large-corpus-eval-harness]` + tokenizer-default-on（若延后）`[SPEC-DEFER:phase-future.tokenizer-default-on]` 留 backlog
