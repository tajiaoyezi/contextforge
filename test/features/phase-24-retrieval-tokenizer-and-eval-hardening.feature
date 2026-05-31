# language: en
# Maps to:
#   - docs/specs/phases/phase-24-retrieval-tokenizer-and-eval-hardening.md
#   - docs/specs/tasks/task-24.1-code-and-cjk-tokenizer.md
#   - docs/specs/tasks/task-24.2-eval-dataset-hardening.md
#   - docs/specs/tasks/task-24.3-closeout-v0.17.0.md
#
# 轻量 BDD（s2v §9.2）；Phase 24 retrieval-tokenizer-and-eval-hardening。Scenario ID 在各 task spec §7 追踪表映射到具体测试。

Feature: phase-24-retrieval-tokenizer-and-eval-hardening
  In order to 提升核心代码检索的代码符号/CJK 召回，并加固 eval 标尺让召回声明可信
  As Phase 24 内核（code/CJK tokenizer opt-in + eval 数据集校验器 + golden 代码/CJK 扩充 + tokenizer recall delta + rust-native-eval-runner 评估 + v0.17.0 收口）
  I want content 字段在 opt-in 时按 camelCase/snake_case/dotted.path/kebab-case 拆分（保留原 token）+ CJK bigram，且默认 tokenization 不变（既有索引不失效）、eval 数据集脏数据被校验器拒、tokenizer recall delta 真实非合成记录不伪造

  # ---
  # Maps to: docs/specs/tasks/task-24.1-code-and-cjk-tokenizer.md (TEST-24.1.1/24.1.2/24.1.3/24.1.4)
  Scenario: SCEN-24.1.1 — 对应 AC1（code/CJK tokenizer opt-in 分词 + 默认不变 + index/query 对称）
    Given core/src/indexer/mod.rs 注册自定义 code/CJK TextAnalyzer（代码符号 splitter 保留原 token + CJK bigram + lowercase）+ content 字段 opt-in 绑定（默认仍 TEXT 默认 analyzer）+ RetrieverConfig.tokenizer 接入点
    When  opt-in 下对 camelCase/getUserById/user_id/pkg.module.func/kebab-case-name + CJK 输入（配置加载）分词，或未 opt-in 走默认 analyzer，或 opt-in 索引后代码符号子词查询
    Then  代码符号拆出子 token（camel+case 等）且保留原 token（TEST-24.1.1）；CJK 拆出 bigram 序 配置/置加/加载（TEST-24.1.2）；未 opt-in 时 tantivy_search 命中不退化 + schema 结构不变（TEST-24.1.3）；index 侧 analyzer 名 = query 侧 tokenizer 名 + opt-in 子词查询命中（TEST-24.1.4）；默认构建 0 新 dep（std-only/Tantivy 自带组合）

  # ---
  # Maps to: docs/specs/tasks/task-24.2-eval-dataset-hardening.md (TEST-24.2.1/24.2.2/24.2.3)
  Scenario: SCEN-24.2.1 — 对应 AC2/AC3（eval 数据集校验器 + golden 代码/CJK 扩充）
    Given internal/eval/eval.go 独立校验器（schema 良构 + 重复检测 + query/answer 覆盖，add-only 不改 ValidateDataset）+ test/fixtures/eval/golden-semantic.jsonl 扩充 annotated query
    When  校验器跑良构数据集（BuiltinGoldenQuestions + 扩充 golden）或脏数据（重复 query / 重复 (query,expected) 对 / 悬空 expected / category 不在已知集 / line_range start>end），且扩充 golden 含代码符号 + CJK query case
    Then  良构数据集过 + schema 不良/悬空被拒（TEST-24.2.1）；重复 query + 重复对被拒 + 既有 ValidateDataset/30 题 builtin/JSONL roundtrip 不退化（TEST-24.2.2）；golden-semantic.jsonl 经 LoadJSONL 含代码符号 query（getUserById/user_id/pkg.module.func）+ CJK query case + 路径真实 exercise task-24.1 tokenizer + 过校验（TEST-24.2.3）；本 task 零真实 recall 数字（真实 delta 在 task-24.3，ADR-013）

  # ---
  # Maps to: docs/specs/tasks/task-24.3-closeout-v0.17.0.md (TEST-24.3.1/24.3.2/24.3.3/24.3.5)
  Scenario: SCEN-24.3.1 — 对应 AC1/AC3/AC5（tokenizer recall delta + runner 评估 + smoke v14 + v0.17.0 收口 + ADR-029 ratify）
    Given task-24.1 tokenizer over task-24.2 扩充 golden 的真实 before/after recall delta + core/src/eval/runner.rs rust-native-eval-runner 评估 + scripts/console_smoke.sh v14 step 33 + v0.17.0 release docs + ADR-029（code-and-cjk-tokenizer-and-eval-hardening）
    When  default analyzer（before）vs opt-in code/CJK analyzer（after）over 扩充 golden 跑 recall@5/10 delta；runner promote 最小 runner（+单测）或诚实延后；smoke v14 文档化 Phase 24 状态；ADR-029 据 task-24.1/24.2 + delta + runner 评估真实结果 ratify
    Then  真实 before/after recall delta 实测落 docs/spikes/phase-24-tokenizer-recall.md（小语料 delta 不显著则如实记录不夸大，ADR-013）；runner promote 或 [SPEC-DEFER:phase-future.rust-native-eval-runner] 诚实延后 + 文档化评估口径（TEST-24.3.1）；smoke 既有 step 不退化 + bash -n exit 0；ADR-029 据真实非合成验证 Proposed→Accepted（受阻维度如实记录不强 ratify）+ ADR-006/008 add-only Amendment（gate 阈值不变，不溯改正文）；phase-24 §6 全 met；ADR-014 D1-D5（第十五次激活）全通过
