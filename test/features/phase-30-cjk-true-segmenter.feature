# language: en
# Maps to:
#   - docs/specs/phases/phase-30-cjk-true-segmenter.md
#   - docs/specs/tasks/task-30.1-cjk-true-segmenter.md
#   - docs/specs/tasks/task-30.2-tokenizer-default-on-and-cjk-recall-delta.md
#   - docs/specs/tasks/task-30.3-closeout-v0.23.0.md
#
# 轻量 BDD（s2v §9.2）；Phase 30 cjk-true-segmenter。Scenario ID 在各 task spec §7 追踪表映射到测试 / 真实 run。

Feature: phase-30-cjk-true-segmenter
  In order to 把 Phase 24 务实的 CJK 重叠 bigram 升级为真分词器（feature-gated，默认 0-dep），评估 tokenizer 默认开启并提供既有索引迁移工具，量出真实可信的 CJK 召回 delta
  As Phase 30 内核（true CJK word segmenter behind cjk-segmenter feature + tokenizer-default-on 评估 + 既有索引 reindex/migration + 扩展 CJK golden recall delta）
  I want 在新 core feature cjk-segmenter 后挂真分词 analyzer（并行 analyzer 名 + 双站点注册 :442/:250，保 bigram 作 0-dep fallback）、把 RetrieverConfig.tokenizer 真接线路由（或文档化 schema-driven 对称）+ 既有索引 reindex 迁移工具、扩 golden-semantic.jsonl CJK case 并据真实 harness 跑出 default vs bigram vs true segmenter 的召回 delta（数值真实跑出后回填、不预填、ADR-013），且默认构建（无 cjk-segmenter feature）默认分词 + 6 字段 schema + 0 新 dep 不变、既有默认索引不失效（向后兼容）

  # ---
  # Maps to: docs/specs/tasks/task-30.1-cjk-true-segmenter.md (TEST-30.1.1)
  Scenario: SCEN-30.1.1 — 对应 AC1（true segmenter 配置加载 → 配置/加载 vs bigram）
    Given core feature cjk-segmenter（默认 off → 0 新 dep，镜像 vector-lancedb feature-gating）下挂真 CJK 词分词 analyzer，并行 analyzer 名 CJK_SEGMENTER_TOKENIZER = cjk_segmenter（保留 build_code_cjk_analyzer 的重叠 bigram 作 0-dep fallback）
    When  在 --features cjk-segmenter 下对多字 CJK 短语 `配置加载` 取真分词 token 流（deterministic 分词单测，无 live dep）
    Then  真分词器按词边界切出 `配置` / `加载` 两 WORD token，区别于 bigram 的 `配置` / `置加` / `加载`（坐实真分词 ≠ 重叠 bigram，TEST-30.1.1，🟢 CI-verifiable under --features cjk-segmenter）；segmenter lib（jieba-rs vs lindera）选型 + optional dep 经主 agent R7 chore + ADR-008 add-only（本 phase 仅规划记录，不加 dep）

  # ---
  # Maps to: docs/specs/tasks/task-30.1-cjk-true-segmenter.md (TEST-30.1.3)
  Scenario: SCEN-30.1.3 — 对应 AC3（default build unchanged 0-dep）
    Given 默认构建（无 cjk-segmenter feature，core/Cargo.toml default = []）—— cjk-segmenter 经 cfg(feature) gate 默认不编译，真分词 lib 经 optional dep 默认不引入
    When  跑 cargo test --workspace（默认 feature 集，无 cjk-segmenter）并核默认 analyzer 绑定 + 6 字段 schema + 依赖图
    Then  默认分词（DEFAULT_TOKENIZER = default）+ 6 字段 schema + 0 新 dep 保持不变，cargo test --workspace 不受影响（默认构建 net-zero / dep-zero baseline，TEST-30.1.3，🟢 CI-verifiable no live dep）；守 ADR-004 默认构建 0 新 dep + ADR-035 D5

  # ---
  # Maps to: docs/specs/tasks/task-30.2-tokenizer-default-on-and-cjk-recall-delta.md (TEST-30.2.1)
  Scenario: SCEN-30.2.1 — 对应 AC1（real recall delta over expanded CJK golden, no prefill）
    Given 扩展后的 test/fixtures/eval/golden-semantic.jsonl（增 CJK case，经 Go ValidateGoldenSemantic 校 schema/dup/category）+ phase24 式 harness（core/examples/phase24_tokenizer_recall.rs 方法论，docs/spikes/phase-24-tokenizer-recall.md）
    When  在扩展 CJK golden 上跑 default vs bigram vs true segmenter 三档召回对比（真实小语料，无外推）
    Then  录出 default / bigram / true segmenter 的 before/after/segmenter 召回 delta —— 真实跑出后回填，绝不预填（ADR-013，小语料 caveat 如实标注、不外推单 case 驱动的虚高 delta，TEST-30.2.1，🟡 needs local real run）；若 default-on 全量迁移过重则诚实延后 default flip 保 opt-in + 迁移工具 [SPEC-DEFER:phase-future.tokenizer-default-on]

  # ---
  # Maps to: docs/specs/tasks/task-30.2-tokenizer-default-on-and-cjk-recall-delta.md (TEST-30.2.2)
  Scenario: SCEN-30.2.2 — 对应 AC2（existing-index migration + config routing, backward-compat）
    Given 既有索引 reindex/migration 工具（改默认 analyzer 绑定须 re-index，因绑定持久化于 tantivy meta.json）+ RetrieverConfig.tokenizer 路由接线（现 vestigial @ retriever/mod.rs:99，search 路径从未读）或文档化 schema-driven 对称
    When  对既有默认索引跑迁移工具 + 经 config.tokenizer 路由选择注册哪个 register fn（或核 schema-driven 对称），并核既有默认索引 round-trip
    Then  既有默认索引不被破坏（向后兼容），config.tokenizer 真接线路由生效（或 schema-driven 对称文档化），迁移工具完成既有索引到新绑定的 re-index（TEST-30.2.2，🟢 CI-verifiable no live dep）；新 analyzer 名须在 index 站点 open_with_tokenizer:442 + query 站点 open_with_config:250 双注册否则 query 解析静默失败 → 召回退化（task-24.1 R4）
