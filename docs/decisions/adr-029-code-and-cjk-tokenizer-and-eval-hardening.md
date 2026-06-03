# ADR `029`: `code-and-cjk-tokenizer-and-eval-hardening`

**Status**: Accepted (2026-05-31 起草 + 同日 v0.17.0 closeout（task-24.3）据 task-24.1/24.2 + 真实 before/after recall delta + rust-native-eval-runner 真实评估结论 ratify Proposed→Accepted，ADR-013 真实非合成。D1-D5 真实依据见 §Ratification。)
**Category**: 检索质量 / 分词 / eval 加固
**Date**: 2026-05-31
**Decided By**: 主 agent (ADR-012 自治)；tajiaoyezi ratification at v0.17.0 closeout
**Related**: ADR-006 (recall-eval-acceptance-gate — Top5/Top10 strong-hit gate + Amendment A1 SemanticRecall@10) / ADR-002 (sqlite+tantivy persistence — `content` TEXT 字段 schema) / ADR-008 (core-library-selection — 依赖选型 add-only) / ADR-004 (local-first-privacy-baseline — 默认 0 新 dep / 0 network) / ADR-013 (cli-data-plane-grpc-bridge — 禁伪造凭据红线 / real-data-only) / ADR-014 (D1-D5 cross-phase exit gate，第十五次激活) / ADR-023 (vector-backend-default — 语义路径) / Phase 2 (task-2.4 indexer Tantivy schema) / Phase 8 (task-8.1 recall harness `internal/eval/eval.go`) / Phase 14 (task-14.1 EvalRunner placeholder `core/src/eval/runner.rs`) / Phase 19 (task-19.5 real recall eval + `test/fixtures/eval/dogfood-embeddings.jsonl`) / Phase 24 (retrieval-tokenizer-and-eval-hardening)

## Context

ContextForge 的核心用例是「让 AI agent 在代码 + 文档语料上检索」。两块直接影响该用例可信度的检索质量债至今未结：

1. **`content` 字段用默认 TEXT tokenizer，对代码符号 / CJK 分词偏弱**：`core/src/indexer/mod.rs:148` 把 Tantivy `content` 字段建为 `sb.add_text_field("content", TEXT | STORED)`——`TEXT` 用 Tantivy 默认 analyzer（`SimpleTokenizer` + lowercase，按非字母数字边界切词）。对代码符号这套切分偏弱：`camelCase` 不拆成 `camel`+`case`、`snake_case` / `dotted.path` / `kebab-case` 的子词无法独立命中、CJK 文本（无空格分隔）被整段当一个 token。`core/src/retriever/mod.rs:99` 的 `RetrieverConfig.tokenizer` 字段恒 `"default"`（task-4.1 §5 留的接入点 + `mod.rs:1131` 测试注「CJK 留接入点 PRD §O11」），查询侧 tokenizer 也未自定义。`docs/roadmap.md` §4 把该项列为 backlog marker `检索 tokenizer: cjk-and-code-tokenizer`（承 `phase-19` §2）。结果：`getUserById` / `user_id` / `配置加载` 这类查询在 BM25 路径上召回偏弱，损害核心代码检索体验。

2. **eval 这把「召回标尺」未加固，召回声明可信度受限**：召回声明全靠 eval 度量背书——recall harness 在 Go（`internal/eval/eval.go`，`ValidateDataset` 仅校验 ≥30 题 / ≥6 类 / 每类 ≥5 题 / 必填字段），golden 数据集为 `BuiltinGoldenQuestions()`（30 题硬编码）+ `test/fixtures/eval/dogfood-embeddings.jsonl`（40 行 dim-384 real embedding 语料，task-19.5）。`core/src/eval/runner.rs` 是 placeholder（`trigger_external` noop，task-14.1 标 `[SPEC-DEFER:phase-future.rust-native-eval-runner]`，真实触发在 Go `runEvalAsync`）。三处局限：(a) golden 数据集无独立校验器查 schema 良构 / 重复 query / query-answer 覆盖完整性，脏数据会静默喂入召回口径；(b) golden 数据集无代码符号 / CJK query case，无法度量第 1 块 tokenizer 改进的真实效果；(c) Rust-native eval runner 是 placeholder，Rust 侧无法独立产召回数（依赖 Go harness）。`docs/roadmap.md` §4 列 `eval-dataset-validation` / `semantic-golden-dataset` / `rust-native-eval-runner` 三 marker。

本 ADR 记录两者的处理策略：自定义 code/CJK tokenizer 的注册方式 + 向后兼容口径 + eval 数据集校验器 + golden 数据集扩充 + tokenizer 真实 before/after recall delta 度量口径 + rust-native-eval-runner 的评估结论。

## Decision

检索 tokenizer 与 eval 加固采用 **opt-in、向后兼容、真实召回背书、不破坏默认 baseline** 的策略：

### D1 — 自定义 code/CJK tokenizer：注册自定义 analyzer + opt-in config（向后兼容）

`core/src/indexer/mod.rs` 注册一个自定义 Tantivy `TextAnalyzer`（code/CJK 感知，task-24.1），对 `content` 字段在 **opt-in 时** 生效；分词规则：

- **代码符号拆分**：`camelCase` → `camel` + `case`（+ 保留原 token `camelCase`）；`snake_case` → `snake` + `case`（+ 原 token）；`dotted.path` → `dotted` + `path`（+ 原 token）；`kebab-case` → `kebab` + `case`（+ 原 token）。**保留原 token** 让原样查询不退化。
- **CJK 处理**：CJK 字符段用 bigram（如 `配置加载` → `配置` / `置加` / `加载`），让无空格 CJK 文本可子串命中。
- **向后兼容（ADR-004）**：默认 tokenization **不变**（`content` 仍走默认 `TEXT` analyzer），自定义 analyzer **opt-in via config** 初始启用——既有 collection 索引不被静默失效。opt-in 切换会改变倒排词项，需 **re-index** 才生效，该 re-index 含义在 task-24.1 spec + release docs 明确文档化。index 侧 analyzer 与 query 侧 tokenizer 名一致（`RetrieverConfig.tokenizer` 接入点）以保对称分词。

`save`/`load` 与持久化不在本 ADR——本 ADR 改的是倒排词项分词，磁盘格式由 Tantivy 既有 schema 承载。tokenizer 改进的真实 before/after recall delta 在 closeout（task-24.3）据真实非合成数据度量，不在 task-24.1 内预判跨语料召回数值（ADR-013）。

### D2 — eval 数据集校验器：schema 良构 + 重复检测 + query/answer 覆盖

eval golden 数据集加独立校验器（task-24.2），在既有 `internal/eval/eval.go::ValidateDataset`（≥30 题 / ≥6 类 / 每类 ≥5 题 / 必填字段）之上补强：(a) **schema 良构**——每条 question 的字段类型 / 必填项 / 枚举（category 在已知集）well-formed；(b) **重复检测**——同一 `query` 文本重复 / 同一 `(query, expected)` 对重复被识别；(c) **query/answer 覆盖**——每个声明的 expected 文件 / chunk_id 在数据集口径内可被覆盖检查（无悬空 expected）。校验器以 deterministic 单测可断言（已知良构数据集过、已知脏数据被拒）。校验器是 add-only：不改既有 `ValidateDataset` 的现有断言语义（既有 30 题 builtin + JSONL roundtrip 不退化）。

### D3 — golden 数据集扩充：代码符号 + CJK annotated query case

semantic-golden-dataset 扩充 annotated query（task-24.2），**含代码符号 query case（如 camelCase / snake_case / dotted.path 标识符查询）+ CJK query case**，使其能 exercise task-24.1 tokenizer 的代码/CJK 分词改进。扩充的 query 带 expected 标注（`query` + `expected_file_path` / `expected_chunk_id` + `category`，沿用 `internal/eval` `Question` JSON 形状）。扩充数据集过 D2 校验器。**真实 recall 数字不在 task-24.2 产出**——task-24.2 只产「可校验、可 exercise tokenizer」的扩充数据集 + 校验器；真实 before/after recall delta 在 closeout（task-24.3）据真实非合成跑出。

### D4 — rust-native-eval-runner：真实评估后 promote 或诚实延后

`core/src/eval/runner.rs`（placeholder，`[SPEC-DEFER:phase-future.rust-native-eval-runner]`）在 closeout（task-24.3）据真实评估二选一：(a) 若评估表明可行且收益清晰，把 placeholder promote 为最小 Rust-native runner（Rust 侧能独立对一组 question + 检索结果算召回，复用既有 `internal/eval` 召回口径的 Rust 等价 / 或最小子集），落 deterministic 单测；(b) 若评估表明 Go harness 仍是更务实路径（跨语言进程管理 / 收益不足），则诚实延后 `[SPEC-DEFER:phase-future.rust-native-eval-runner]` + 文档化评估口径（不在 placeholder 上伪造「已实现」）。**ADR-013 红线：不在源码 / 文档伪造 rust-native runner 已落地或召回已产**；评估结论以真实依据如实记录。

### D5 — 默认构建不变：0 新 dep + 默认 tokenization 不变 + eval 口径不变

tokenizer opt-in（D1）/ 校验器（D2）/ 数据集扩充（D3）/ runner 评估（D4）**全部不破坏默认 baseline**：默认 `content` tokenization 不变（既有索引不失效）；eval 既有 `ValidateDataset` + 30 题 builtin + recall gate 阈值（`GateTop5StrongMin=0.75` / `GateTop10StrongMin=0.85` / `GateSemanticRecall10Min=0.70`，`eval.go:103-108`）不变。tokenizer / 校验器若需新依赖（如 CJK 分词 crate），仅在评估确证 std 不可行时经主 agent R7 chore + ADR-008 add-only 引入（优先 std-only，ADR-004）。本 ADR 不改 `internal/eval` 召回度量函数（`SemanticRecallAtK` / `SummarizeHybrid` / `MeetsRecallGate`）签名。

## Consequences

- **Positive**: 核心代码检索用例的代码符号 / CJK 召回经 opt-in tokenizer 改进，且改进有真实 before/after recall delta 背书（ADR-013）；eval 标尺经数据集校验器加固，召回声明不再被脏数据静默污染；golden 数据集首次含代码/CJK query case，使 tokenizer 改进可被持续度量；rust-native-eval-runner 经真实评估给出可行结论（promote 或诚实延后），placeholder 状态收敛；默认构建保持 0 新 dep + 默认 tokenization 不变（既有 collection 不失效）+ eval 口径不变。
- **Negative / open**: tokenizer opt-in 切换需 re-index 才生效（既有索引不自动重建——该取舍在 release docs 文档化）；CJK bigram 分词的精度受 bigram 粒度限制（非真正分词器，长尾精度优化属后续）；tokenizer 改进的 recall delta 取决于扩充 golden 数据集的代码/CJK case 覆盖度（小语料 delta 可能偏弱，如实记录不夸大）；rust-native-eval-runner 可能经评估仍延后（D4 受阻态如实记录，Go harness 续用）。
- **Ratification**: 本 ADR **Proposed**。task-24.1 真实 tokenizer 分词单测（代码/CJK 输入拆分正确）+ task-24.2 真实校验器单测 + 扩充数据集 + task-24.3 真实 before/after recall delta（tokenizer over 扩充 golden）+ rust-native-eval-runner 真实评估结论通过后，于 v0.17.0 closeout（task-24.3）据真实非合成验证 ratify Proposed→Accepted（ADR-013：禁据合成 / 伪造 ratify）；某维度受阻（如 recall delta 因小语料不显著 / runner 经评估延后）则据「已达维度 ratify + 受阻维度如实记录」处理，不强 ratify。
- **Follow-ups**: CJK 真正分词器（替 bigram）的精度优化 `[SPEC-DEFER:phase-future.cjk-true-segmenter]`；tokenizer 默认开启（从 opt-in 转 default + 索引迁移工具）`[SPEC-DEFER:phase-future.tokenizer-default-on]`；rust-native-eval-runner 若 D4 评估延后则续 `[SPEC-DEFER:phase-future.rust-native-eval-runner]`（roadmap §4）；golden 数据集 case_results 子表 `[SPEC-DEFER:phase-future.case-results-subtable]`（roadmap §4）。

## Ratification (v0.17.0 / task-24.3 — Accepted 2026-05-31)

本 ADR 于 v0.17.0 closeout（task-24.3）据 task-24.1/24.2 + 真实 before/after recall delta + rust-native-eval-runner 真实评估结论 ratify **Proposed → Accepted**（ADR-013：真实非合成）。D1–D5 真实验证依据：

- **D1（code/CJK tokenizer）— ✅ 已达**：`cargo test -p contextforge-core --lib indexer::tests::test_24_1` 4 passed（TEST-24.1.1-4：camelCase/snake_case/dotted.path/kebab-case 拆子词 + 保留原 token + CJK bigram 确定性 token-stream 断言；opt-in `content` 走自定义 analyzer；默认 tokenization 不变）。**修订口径**：tokenizer 为 **opt-in via `RetrieverConfig.tokenizer="code_cjk"`（config，非 feature-gate）**，默认构建即含（0 新 dep，纯 std），不需 `--features`。真实 before/after recall delta（task-24.3，BM25 file-level over task-24.2 golden）= **+0.0909**（default 0.9091 → code/CJK 1.0000），由真实 CJK bigram 命中（`语义检索`）驱动；sub-token 判别力由 TEST-24.1.4 确定性背书。证据 `docs/spikes/phase-24-tokenizer-recall.md`。
- **D2（eval 数据集校验器）— ✅ 已达**：`go test ./internal/eval/...` ok（TEST-24.2.1-4：`ValidateGoldenSemantic` 对良构过 / 脏数据（重复 query / 重复 (query,expected) 对 / 悬空 expected / 未知 category / line_range start>end）被拒；既有 `ValidateDataset` + 30 题 builtin + JSONL roundtrip 不退化，add-only）。
- **D3（golden 数据集扩充）— ✅ 已达**：`test/fixtures/eval/golden-semantic.jsonl` 11 题（6 `code-symbol` + 5 `cjk`，query→真实文件路径经核实），过 D2 校验器，exercise D1 tokenizer（TEST-24.2.3）。
- **D4（rust-native-eval-runner）— 🟡 诚实延后**：真实评估结论 = **延后** `[SPEC-DEFER:phase-future.rust-native-eval-runner]`。Go harness（`internal/eval/eval.go`）仍是召回口径单一事实源（task-14.1），Rust-native runner 会跨语言重复 `SemanticRecallAtK`/gate 逻辑 → drift 风险且无现消费方；ad-hoc Rust 召回量测由 `core/examples/phase24_tokenizer_recall.rs` 覆盖。placeholder + marker 保留，理由见 spike §4 + `core/src/eval/runner.rs`（ADR-013：不伪造已实现）。
- **D5（默认构建不变）— ✅ 已达**：`cargo test --workspace` + `go test ./...` 0 failed；0 新 dep（`core/Cargo.toml` 未改，纯 std tokenizer）；默认 tokenization 不变（既有索引不失效）；eval gate 阈值（`GateTop5StrongMin=0.75` / `GateTop10StrongMin=0.85` / `GateSemanticRecall10Min=0.70`）不变（TEST-24.2.4 守护）。

ratify 范围 = code/CJK tokenizer + eval 加固**策略**（D1-D3/D5 经真实 cargo/go test + recall 度量验证；D4 经真实评估诚实延后）。tokenizer 默认开启 + 索引迁移 [SPEC-DEFER:phase-future.tokenizer-default-on] + CJK 真正分词器 [SPEC-DEFER:phase-future.cjk-true-segmenter] + rust-native-eval-runner [SPEC-DEFER:phase-future.rust-native-eval-runner] 属后续。ADR-006（gate 阈值不变）/ ADR-008（tokenizer std-only，无依赖变更）无需 amendment。证据见 `docs/releases/v0.17.0-evidence.md` + `docs/spikes/phase-24-tokenizer-recall.md`。

## Amendment (Phase 30 / v0.23.0, 2026-06-03 — add-only, D1–D5 正文不溯改)

Phase 30（ADR-035 cjk-true-segmenter-and-tokenizer-default）以 add-only 方式兑现本 ADR `[SPEC-DEFER]` 所留的两个 follow-up marker，**不溯改 D1–D5 / Consequences / Ratification 正文**（ADR-014 D5）：

- **`[SPEC-DEFER:phase-future.cjk-true-segmenter]` → 🟢 兑现（feature-gated，bigram fallback 保留）**：task-30.1（PR #202）经 `cjk-segmenter` feature（jieba-rs 0.7.4，pure-Rust，主 agent R7 chore + ADR-008 add-only）加并行 `cjk_segmenter` analyzer，对 CJK run 真分词（`配置加载 → 配置/加载`，区别 bigram `配置/置加/加载`），index/query 双站点对称注册。**D1 的 overlapping bigram `code_cjk` 保留作默认 0-dep fallback**（默认构建不编译 jieba，0 新 dep at default features，ADR-004 守线）——非替换、是并行升级。
- **`[SPEC-DEFER:phase-future.tokenizer-default-on]` → 🟡 部分兑现（迁移工具 + 评估；default flip 仍延后）**：task-30.2（PR #203）提供 `IndexSession::reindex_with_tokenizer` 真实迁移工具（analyzer 绑定持久化 `meta.json` ⇒ 切 analyzer 须 re-index，工具读 SQLite chunk 真相源重建）+ `RetrieverConfig.tokenizer` schema-driven 对称文档化（D「vestigial config」现状收敛为方案 B）。**full default flip 据「迁移工具已备 + 翻默认是产品决策」诚实延后**（默认仍 opt-in，`[SPEC-DEFER:phase-future.tokenizer-default-on]` 续留）。
- **D3/D4 recall 度量延伸（真实零 delta 诚实记录）**：扩 `golden-semantic.jsonl` +5 CJK case（11→16），phase24-style harness 实测 default/bigram/真分词——**真分词相对 bigram file-level 召回 delta=+0.0000（小语料持平）**，如实记录、不外推（ADR-013）。本 ADR 的 gate 阈值（`GateTop5StrongMin` 等）+ Go validator 单一事实源不变。

依赖变更：task-30.1 jieba-rs 为 optional dep（经 ADR-008 add-only，默认构建不编译）；task-30.2 reindex 工具复用既有 SQLite/Tantivy 面 0 新 dep。详见 ADR-035 Ratification + `docs/releases/v0.23.0-evidence.md`。

## Amendment (Phase 31 / v0.24.0, 2026-06-03 — add-only, 正文不溯改)

Phase 31（ADR-036 D3）以 add-only 方式把 eval per-case 结果提升为可查询子表，**不溯改正文 + 不溯改 Phase 30 Amendment**（ADR-014 D5）：

- **case-results subtable（可查询）**：task-31.3（PR #208）`core/migrations/0018_eval_case_results.sql`（add-only：CREATE TABLE `eval_case_results` + FK `eval_run_id` + index，不动 `eval_runs`/`case_results_json` 列）；`core/src/eval/store.rs` `update_case_results` 双写（保 `case_results_json` → 既有 `row_to_run` JSON-blob 读路径不变）+ 新 `query_case_results`（per-run）/ `case_pass_ratio`（跨 run 聚合）SQL 查询。per-case 结果从单表 JSON blob 升为可 SQL 过滤/聚合的子表。`cargo test eval::store` 12 passed（含旧 run JSON 读 + 新 run 子表查并存）。本 ADR 的 gate 阈值 + Go validator 单一事实源不变。

依赖变更：复用既有 rusqlite/serde_json，0 新 dep。详见 ADR-036 Ratification + `docs/releases/v0.24.0-evidence.md`。
