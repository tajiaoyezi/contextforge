# ADR `029`: `code-and-cjk-tokenizer-and-eval-hardening`

**Status**: Proposed (2026-05-31 起草；v0.17.0 closeout（task-24.3）据 task-24.1/24.2 真实非合成验证 ratify Proposed→Accepted，ADR-013：真实 before/after recall delta + rust-native-eval-runner 真实评估结论；某维度受阻则据「已达维度 ratify + 受阻维度如实记录」处理，不强 ratify。见 §Ratification 占位段。)
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

## Ratification (v0.17.0 / task-24.3, 待回填)

本 ADR 计划于 v0.17.0 closeout 据 task-24.1/24.2 的真实非合成验证 ratify **Proposed → Accepted**（ADR-013：禁据合成 / 伪造 ratify）。D1–D5 各项的真实验证依据由 task-24.3 据实回填（实施完成时填实，Draft 阶段不预填）：

- **D1（code/CJK tokenizer）**：`cargo test --features <tokenizer-feature> ... indexer` tokenizer 分词单测对代码符号 / CJK 输入拆分正确 + opt-in 时 `content` 走自定义 analyzer / 默认 tokenization 不变；real before/after recall delta（task-24.3 据扩充 golden 跑）。
- **D2（eval 数据集校验器）**：`go test ./internal/eval/...` 校验器对良构数据集过 / 脏数据（重复 / 悬空 expected / schema 不良）被拒；既有 `ValidateDataset` + 30 题 builtin 不退化。
- **D3（golden 数据集扩充）**：扩充数据集含代码符号 + CJK query case 过 D2 校验器 + exercise D1 tokenizer。
- **D4（rust-native-eval-runner）**：真实评估结论（promote 最小 runner + 单测，或诚实延后 `[SPEC-DEFER:phase-future.rust-native-eval-runner]` + 文档化评估口径）。
- **D5（默认构建不变）**：默认 `cargo test --workspace` + `go test ./...` 0 新 dep、默认 tokenization + eval gate 阈值不变。

ratify 范围 = code/CJK tokenizer + eval 加固**策略**（D1-D5 经真实 cargo build/test + recall 度量验证）；tokenizer 默认开启 + 索引迁移 + CJK 真正分词器属后续。证据见 `docs/releases/v0.17.0-evidence.md`（待 task-24.3 产出）。
