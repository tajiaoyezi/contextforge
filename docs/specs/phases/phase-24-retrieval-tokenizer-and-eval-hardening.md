# Phase 24 · retrieval-tokenizer-and-eval-hardening

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 解决两块直接影响核心代码检索用例可信度的检索质量债：**`content` 字段代码/CJK 分词偏弱**（`core/src/indexer/mod.rs:148` 用默认 `TEXT` analyzer，对 camelCase / snake_case / dotted.path / kebab-case / CJK 切分弱，`docs/roadmap.md` §4 marker `cjk-and-code-tokenizer`，承 `phase-19` §2）与 **eval 标尺未加固**（`internal/eval/eval.go::ValidateDataset` 仅基本校验、golden 数据集无代码/CJK case、`core/src/eval/runner.rs` 为 placeholder，`docs/roadmap.md` §4 三 marker `eval-dataset-validation` / `semantic-golden-dataset` / `rust-native-eval-runner`）。eval 是度量召回的标尺，加固它让召回声明可信。v0.17.0 收口。对应 `docs/roadmap.md` §4。
>
> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md` §4（eval + 检索 tokenizer backlog 四 marker）→ `docs/decisions/adr-029-code-and-cjk-tokenizer-and-eval-hardening.md`（D1 tokenizer opt-in + D2 校验器 + D3 数据集扩充 + D4 runner 评估 + D5 默认不变）→ `core/src/indexer/mod.rs:145-157`（`build_tantivy_schema` — `content` 字段 `TEXT | STORED` 默认 analyzer）+ `core/src/indexer/mod.rs:440-462`（`tantivy_search` — `QueryParser::for_index(&index, vec![content])` 查询侧分词）→ `core/src/retriever/mod.rs:96-115`（`RetrieverConfig.tokenizer` 恒 `"default"` 接入点 + `mod.rs:1131` 「CJK 留接入点 PRD §O11」测试注）→ `internal/eval/eval.go`（`ValidateDataset` ≥30 题 / ≥6 类 / 每类 ≥5 题 / 必填字段 + `Question` 形状 + `BuiltinGoldenQuestions` 30 题 + `SemanticRecallAtK` / `SummarizeHybrid` / `MeetsRecallGate` + gate 阈值 `eval.go:103-108`）+ `internal/eval/eval_test.go`（既有 eval 单测口径）→ `test/fixtures/eval/dogfood-embeddings.jsonl`（task-19.5 real embedding 语料 40 行 dim-384）→ `core/src/eval/runner.rs`（`EvalRunner` placeholder + `[SPEC-DEFER:phase-future.rust-native-eval-runner]` marker）+ `core/src/eval/mod.rs` / `core/src/eval/store.rs` → `docs/spikes/phase-19-real-recall.md`（real recall 度量口径基线）→ AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5，第十五次激活）→ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造凭据红线 / real-data-only）→ `docs/decisions/adr-004-local-first-privacy-baseline.md`（默认 0 新 dep / 0 network，feature/opt-in）→ `docs/decisions/adr-008-core-library-selection.md`（依赖选型 add-only）→ `docs/decisions/adr-006-recall-eval-acceptance-gate.md`（recall gate 阈值）。
>
> **ADR 影响面（已识别）**：
> - **ADR-029 code-and-cjk-tokenizer-and-eval-hardening（新，Proposed）**：记自定义 code/CJK tokenizer 注册 + opt-in 向后兼容口径（D1）+ eval 数据集校验器（D2）+ golden 数据集代码/CJK 扩充（D3）+ rust-native-eval-runner 评估口径（D4）+ 默认构建不变（D5）。落地后据真实非合成 tokenizer 分词单测 + 校验器单测 + 真实 before/after recall delta + runner 评估结论 ratify（ADR-013）。
> - 触及 **ADR-006（recall-eval-acceptance-gate）**：本 phase 加固 eval 标尺但**不改 gate 阈值**（`GateTop5StrongMin` / `GateTop10StrongMin` / `GateSemanticRecall10Min` 不变）；recall delta 度量复用既有口径，以 add-only Amendment 记推进结果（若需），不溯改 ADR-006 正文（D5）。
> - 触及 **ADR-008（core-library-selection）**：若 CJK 分词确证 std 不可行需引入分词 crate，按 add-only Amendment 记录（不溯改既有 D 段）。

## 1. 阶段目标

v0.17.0 ship 后，ContextForge 的全文检索具备 **opt-in 的代码/CJK 感知 tokenizer**（`content` 字段在 opt-in 时按 camelCase / snake_case / dotted.path / kebab-case 拆分并保留原 token + CJK bigram，默认 tokenization 不变以不失效既有索引），并具备 **加固的 eval 标尺**（golden 数据集独立校验器查 schema 良构 / 重复 / 覆盖 + golden 数据集含代码符号 + CJK annotated query case），且对 **rust-native-eval-runner** 完成真实评估（promote 最小 runner 或诚实延后）。tokenizer 改进的真实 before/after recall delta 在 closeout 据扩充 golden 数据集真实跑出（ADR-013）。默认构建仍 0 新 dep、默认 tokenization + eval gate 阈值不变（ADR-004 / ADR-006）。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. `core/src/indexer/mod.rs` 注册自定义 code/CJK `TextAnalyzer`，opt-in 时 `content` 字段按代码符号（camelCase→camel+case / snake_case / dotted.path / kebab-case，保留原 token）+ CJK bigram 分词；默认 tokenization 不变（既有索引不失效，re-index 含义文档化）；deterministic 单测断言代表性代码/CJK 输入拆分正确；默认构建 0 新 dep 不退化（AC1）
2. eval golden 数据集独立校验器落地——查 schema 良构 + 重复（同 query / 同 (query,expected) 对）+ query/answer 覆盖（无悬空 expected）；deterministic 单测断言良构数据集过 / 脏数据被拒；既有 `ValidateDataset` + 30 题 builtin + JSONL roundtrip 不退化（AC2）
3. semantic-golden-dataset 扩充 annotated query 含代码符号 query case（camelCase / snake_case / dotted.path 标识符）+ CJK query case，exercise AC1 tokenizer；扩充数据集过 AC2 校验器；deterministic 校验断言（AC3）
4. tokenizer 真实 before/after recall delta（task-24.1 tokenizer over task-24.2 扩充 golden）实测产出（ADR-013 真实非合成，受阻则诚实延后）+ rust-native-eval-runner 真实评估结论（promote 最小 runner 或诚实延后 `[SPEC-DEFER:phase-future.rust-native-eval-runner]`）（AC4）
5. v0.17.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ console_smoke v14 step + ADR-029 据真实非合成结果 ratify 或记录维持 + phase §6 闭合（AC5）
6. ADR-014 D1-D5（第十五次激活）全通过（AC6）

**v0.x 版本号决策**：v0.17.0 minor release（检索 tokenizer 质量 + eval 加固债收口；默认构建仍 0 新 dep + 默认 tokenization 不变 + eval gate 阈值不变——tokenizer 改进 opt-in，add-only 不破坏既有客户端 / 既有索引）。

## 2. 业务价值

直接推进 `docs/roadmap.md` §4 的检索 tokenizer + eval 四个 backlog marker，提升核心代码检索用例的召回质量与召回声明的可信度：

- **code/CJK tokenizer（marker `cjk-and-code-tokenizer`）**：核心用例是代码 + 文档检索。默认 `TEXT` analyzer 对 `getUserById` / `user_id` / `pkg.module.func` / `配置加载` 这类查询召回偏弱（`core/src/indexer/mod.rs:148` + `core/src/retriever/mod.rs:1131` 「CJK 留接入点」）。本 phase 让 opt-in 的代码/CJK tokenizer 把这些符号拆成可独立命中的子词 + 保留原 token + CJK bigram，提升代码检索召回（`docs/roadmap.md` §4 marker）。
- **eval-dataset-validation（marker）**：召回声明全靠 eval 度量背书；`internal/eval/eval.go::ValidateDataset` 只查 ≥30 题 / ≥6 类 / 必填字段，脏数据（重复 query / 悬空 expected / schema 不良）会静默喂入召回口径污染数字。本 phase 加独立校验器让 eval 标尺可信。
- **semantic-golden-dataset（marker）**：现 golden 30 题为 BM25 口径（task-18.8 §3 + task-19.5 §10 复用 file-level），无代码符号 / CJK query case，无法度量 tokenizer 改进。本 phase 扩充含代码/CJK case 的 annotated query，使 tokenizer 改进可被 eval 持续度量。
- **rust-native-eval-runner（marker）**：`core/src/eval/runner.rs` 是 placeholder（task-14.1 选 Go-side runner，Rust native 标 `[SPEC-DEFER:phase-future.rust-native-eval-runner]`）。本 phase 据真实评估给出 promote 或延后结论，收敛 placeholder 状态。
- **PRD §O11（CJK tokenization）+ §检索质量**：CJK 分词接入点（`core/src/retriever/mod.rs:1131`）落地 + 代码检索召回质量在核心用例上推进。

**不在本 phase scope**：

- CJK 真正分词器（替 bigram 的词典 / 统计分词）[SPEC-DEFER:phase-future.cjk-true-segmenter]——本 phase 用 bigram 务实起步，真正分词器后续
- tokenizer 从 opt-in 转默认开启 + 既有索引迁移工具 [SPEC-DEFER:phase-future.tokenizer-default-on]——本 phase opt-in 不破坏既有索引，默认开启 + 迁移属后续
- golden 数据集 case_results 子表持久化 [SPEC-DEFER:phase-future.case-results-subtable]——`docs/roadmap.md` §4 长尾
- remote embedding provider 真实联调 [SPEC-DEFER:phase-future.embedding-provider-remote]——v0.15.0 / Phase 22 已记
- hybrid scoring / reranker 调参 [SPEC-DEFER:phase-future.hybrid-scoring] / [SPEC-DEFER:phase-future.reranker]——v0.14.0 / Phase 21 已记
- 向量持久化 / 跨平台 [SPEC-DEFER:phase-future.hnsw-graph-persistence] / [SPEC-DEFER:phase-future.sqlite-vec-cross-platform]——v0.16.0 / Phase 23 已记

## 3. 涉及模块

### 24.1 code/CJK tokenizer（task-24.1）

- 修改 `core/src/indexer/mod.rs`——注册自定义 `TextAnalyzer`（code/CJK 感知）；`content` 字段在 opt-in（config）时绑该 analyzer（代码符号拆分 + 保留原 token + CJK bigram），默认时维持 `TEXT` 默认 analyzer（向后兼容，既有索引不失效）；index 侧 analyzer 名与 query 侧 tokenizer 名（`RetrieverConfig.tokenizer` 接入点）对称
- 复用既有 `core/src/retriever/mod.rs:99` `RetrieverConfig.tokenizer` 字段（task-4.1 接入点，当前恒 `"default"`）作为 query 侧 tokenizer 名来源
- 同源 Rust tests（≥3，opt-in 下：代码符号拆分单测——camelCase / snake_case / dotted.path / kebab-case 拆分 + 保留原 token；CJK bigram 单测；默认 tokenization 不变单测）
- `core/Cargo.toml`——若 CJK 分词确证 std 不可行需分词依赖，按 add-only 评估（R7 经主 agent，优先 std-only / Tantivy 自带 analyzer 组合）

### 24.2 eval 数据集加固 + golden 扩充（task-24.2）

- 修改 `internal/eval/eval.go`——加独立校验器（schema 良构 + 重复检测 + query/answer 覆盖），add-only 不改既有 `ValidateDataset` 现有断言语义
- 新增/扩 `test/fixtures/eval/golden-semantic.jsonl`（或等价 annotated query 数据集）——含代码符号 query case（camelCase / snake_case / dotted.path 标识符）+ CJK query case，exercise task-24.1 tokenizer；过新校验器
- 同源 Go tests（≥3：校验器对良构数据集过 / 脏数据被拒 / 扩充数据集含代码/CJK case 且过校验）
- 不产真实 recall 数字（real before/after recall delta 在 task-24.3）

### 24.3 tokenizer recall delta + runner 评估 + closeout（task-24.3）

- 实测 task-24.1 tokenizer over task-24.2 扩充 golden 的真实 before/after recall delta（ADR-013 real 数据，受阻诚实延后）；落 `docs/spikes/phase-24-tokenizer-recall.md`
- 评估 + 修改 `core/src/eval/runner.rs`——promote placeholder 为最小 Rust-native runner（+ deterministic 单测），或诚实延后 `[SPEC-DEFER:phase-future.rust-native-eval-runner]` + 文档化评估口径
- 修改 `scripts/console_smoke.sh`——v14 step：tokenizer / eval 加固相关 smoke 断言（feature 下 tokenizer 分词 smoke 或如实标 feature 层验证），既有 step 不退化
- 新增 `docs/releases/v0.17.0-{evidence,artifacts}.md` + `README.md` v0.17 段 + `RELEASE_NOTES.md` v0.17.0 段
- 修改 `docs/decisions/adr-029-code-and-cjk-tokenizer-and-eval-hardening.md`——据真实结果 Proposed→Accepted 或记录维持 + ADR-006/008 add-only Amendment（若需，不溯改正文 D5）
- 修改 `docs/s2v-adapter.md`（Phase 24 Draft→Done + Tasks 0→3；ADR-029 状态；roadmap §4 四 marker 推进记录）

### BDD feature

- 新增 `test/features/phase-24-retrieval-tokenizer-and-eval-hardening.feature`（≥3 scenario：code/CJK tokenizer 分词 / eval 数据集校验器 + golden 扩充 / tokenizer recall delta + runner 评估 + v0.17.0 收口）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 24.1 | `core/src/indexer/mod.rs` 自定义 code/CJK `TextAnalyzer` + opt-in `content` 绑定 + 默认不变 + 分词单测 | `../tasks/task-24.1-code-and-cjk-tokenizer.md` |
| 24.2 | `internal/eval/eval.go` 数据集校验器 + `test/fixtures/eval/golden-semantic.jsonl` 代码/CJK 扩充 + 校验单测 | `../tasks/task-24.2-eval-dataset-hardening.md` |
| 24.3 | tokenizer 真实 recall delta + `core/src/eval/runner.rs` 评估（promote/延后）+ console_smoke v14 + v0.17.0 closeout + ADR-029 ratify | `../tasks/task-24.3-closeout-v0.17.0.md` |

## 5. 依赖关系

- **task-24.1**（code/CJK tokenizer）dep task-2.4（`build_tantivy_schema` + `content` 字段 + `IndexSession`）+ task-4.1（`RetrieverConfig.tokenizer` 接入点已有字段）；可与 24.2 并行（写路径不相交：`core/src/indexer/mod.rs` vs `internal/eval/eval.go` + fixtures）。
- **task-24.2**（eval 数据集加固）dep task-14.1（eval 模块框架 + `core/src/eval`）+ task-19.5（recall eval + `test/fixtures/eval/` dogfood 语料约定 + golden 数据集口径）+ task-8.1（`internal/eval/eval.go` `ValidateDataset` + `Question` + `BuiltinGoldenQuestions`）；可与 24.1 并行。
- **task-24.3**（closeout）dep 24.1 + 24.2 全 Done；tokenizer recall delta（24.1 over 24.2 数据集）+ runner 评估为本 task 子项。
- 外部：ADR-029（本 phase 新 Proposed）/ ADR-006（recall gate 阈值不变，本 phase 加固标尺不改阈值，add-only Amendment 若需）/ ADR-008（core-library-selection，依赖变更 add-only）/ ADR-004（local-first，默认 0 新 dep / opt-in）/ ADR-014 第十五次激活 / ADR-013（禁伪造 recall / runner 凭据）/ Phase 19 real recall 度量口径（`docs/spikes/phase-19-real-recall.md`）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（每条 AC 含 ADR-014 D3 verified by 显式 owner；Draft 阶段未勾选，实施后逐条置 `[x]`）**：

- [x] **AC1**：`core/src/indexer/mod.rs` 注册自定义 code/CJK `TextAnalyzer`，opt-in 时 `content` 按 camelCase→camel+case / snake_case / dotted.path / kebab-case（保留原 token）+ CJK bigram 分词；默认 tokenization 不变（既有索引不失效，re-index 含义文档化）；deterministic 单测断言代表性代码/CJK 输入拆分正确；默认构建 0 新 dep 不退化 — verified by task-24.1 §6 AC1-4 + phase-smoke step 1
- [x] **AC2**：eval golden 数据集独立校验器落地——schema 良构 + 重复检测（同 query / 同 (query,expected) 对）+ query/answer 覆盖（无悬空 expected）；deterministic 单测断言良构数据集过 / 脏数据被拒；既有 `ValidateDataset` + 30 题 builtin + JSONL roundtrip 不退化 — verified by task-24.2 §6 AC1-2 + phase-smoke step 2
- [x] **AC3**：semantic-golden-dataset 扩充 annotated query 含代码符号 query case + CJK query case（exercise AC1 tokenizer），过 AC2 校验器；deterministic 校验断言 — verified by task-24.2 §6 AC3 + phase-smoke step 2
- [x] **AC4**：tokenizer 真实 before/after recall delta（task-24.1 tokenizer over task-24.2 扩充 golden）实测产出（ADR-013 真实非合成，受阻则诚实延后）+ rust-native-eval-runner 真实评估结论（promote 最小 runner + 单测，或诚实延后 `[SPEC-DEFER:phase-future.rust-native-eval-runner]` + 文档化评估口径）— verified by task-24.3 §6 AC1 + phase-smoke step 3
- [x] **AC5**：v0.17.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ console_smoke v14 step + ADR-029 据真实非合成结果 ratify 或记录维持 + phase §6 闭合 — verified by task-24.3 §6 AC2-3
- [x] **AC6**：ADR-014 cross-validation gate 全套通过（第十五次激活）— D1 mapping + D2 lint `--touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-23 不溯改 — verified by task-24.3 closeout PR body

**端到端 smoke（C1 集成兜底）**：(1) opt-in 下 code/CJK tokenizer 分词正确（feature/config 下代码符号 + CJK 拆分 smoke 或如实标 feature 层验证）；(2) eval 数据集校验器对扩充 golden（含代码/CJK case）通过 + 脏数据被拒；(3) tokenizer recall delta 实测结论 + runner 评估结论（promote 或如实延后）全 PASS。

## 7. 阶段级风险

- **R1（中）CJK 分词无合适 std 路径 → 需新依赖**：Tantivy 自带 analyzer 未必含 CJK bigram；纯 std bigram 实现可行但需自写。
  - **缓解**：task-24.1 优先自写 std-only CJK bigram（Rust `char` 迭代识别 CJK Unicode 区段 + 滑窗 bigram，0 新 dep）+ 组合 Tantivy 既有 token filter；确证 std 不可行才经主 agent R7 chore + ADR-008 add-only 引入分词 crate。stop-condition：若 opt-in tokenizer 注册与分词单测均不可行则记录受阻态，AC1 不标 `[x]`（ADR-013 不伪造分词通过）。
- **R2（中）tokenizer 改进 recall delta 在小语料不显著**：扩充 golden 仍是小语料（承 task-19.5 §10 小语料 caveat），代码/CJK case 数有限，before/after delta 可能偏弱甚至打平。
  - **缓解**：task-24.3 如实记真实 before/after delta（不篡改 / 不夸大），evidence 标注语料规模 + case 数 + per-case 分解；delta 不显著时如实记录「opt-in tokenizer 落地 + 分词正确性单测背书 + recall delta 在本小语料 <X」，AC4 以「真实 delta 实测 + 诚实记录」满足，不为正 delta 改语料/口径（ADR-013）。
- **R3（中）rust-native-eval-runner promote 收益不足 → 评估延后**：Rust-native runner 需复刻 Go harness 召回口径，跨语言重复 + 收益（task-14.1 已选 Go-side）可能不足。
  - **缓解**：task-24.3 真实评估 promote 可行性 + 收益；可行且收益清晰则落最小 runner + deterministic 单测，否则诚实延后 `[SPEC-DEFER:phase-future.rust-native-eval-runner]` + 文档化评估口径（不在 placeholder 伪造已实现），AC4 以「真实评估 + promote 或诚实延后」满足。
- **R4（低）opt-in tokenizer 切换静默失效既有索引**：opt-in 改倒排词项，既有索引未 re-index 时新旧分词不一致。
  - **缓解**：默认 tokenization 不变（既有索引默认走默认 analyzer，不被动失效）；opt-in 切换的 re-index 含义在 task-24.1 spec + release docs 明确文档化（ADR-004 向后兼容），AC1 含「默认不变」单测覆盖。

## 8. Definition of Done

- 3 task spec（24.1-24.3）顶部 `**Status**: Done`
- §6 阶段级 AC1-6 全 `[x]`（受阻态按 ADR-013 如实记录，不伪造）
- 端到端 smoke 3 step 全 PASS（含受阻态如实标注）
- **ADR**：ADR-029 `Proposed → Accepted`（据真实非合成 tokenizer 分词单测 + 校验器单测 + 真实 recall delta + runner 评估结论）或据实测记录维持 + 文档化；ADR-006 / ADR-008 add-only Amendment 记推进结果（若需，不溯改正文，D5）
- **adapter**：§Phase 索引 Phase 24 `Draft → Done` + `Tasks 0 → 3`；§ADR 索引 ADR-029；§BDD 追加 phase-24 feature 行；roadmap §4 四 marker（`cjk-and-code-tokenizer` / `eval-dataset-validation` / `semantic-golden-dataset` / `rust-native-eval-runner`）推进记录
- **spike evidence**：`docs/spikes/phase-24-tokenizer-recall.md`（tokenizer 真实 before/after recall delta + runner 评估结论）
- **release**：`docs/releases/v0.17.0-{evidence,artifacts}.md` + `RELEASE_NOTES.md` v0.17 段 + README v0.17 段
- **follow-up**：CJK 真正分词器 [SPEC-DEFER:phase-future.cjk-true-segmenter]；tokenizer 默认开启 + 索引迁移 [SPEC-DEFER:phase-future.tokenizer-default-on]；rust-native-eval-runner 若延后则 [SPEC-DEFER:phase-future.rust-native-eval-runner]；case_results 子表 [SPEC-DEFER:phase-future.case-results-subtable] 续 backlog
