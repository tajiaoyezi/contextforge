# Phase 41 · tokenizer-default-on
# 做出 Phase 30 / ADR-035 D3 诚实延后的产品决策：把 code/CJK tokenizer（code_cjk，纯 std 0-dep）
# 从 opt-in 翻为新建 collection 的生产默认。首次刻意默认行为变更（非 byte-equiv），由 ADR-046 承接。
# ADR-046（Proposed→Accepted @ task-41.3）。0 新 dep（code_cjk 纯 std；jieba 仍 feature opt-in）/ 0 网络
# / 既有 collection 不受影响 / opt-out via CONTEXTFORGE_TOKENIZER / [retrieval] tokenizer（ADR-004/008/029/035）。

Feature: tokenizer-default-on — code/CJK tokenizer 翻为新建 collection 生产默认
  作为 ContextForge 维护者
  我希望把 Phase 24 已实测 +0.0909 但 opt-in 的 code/CJK tokenizer 翻为新建 collection 默认
  以便全体用户默认获更好的代码符号 / CJK 检索召回
  且既有 collection 不受影响、可经 env / config opt-out、jieba 默认不取（0-dep）、recall delta 据实不夸大

  # ---- task-41.1: production tokenizer 默认翻 code_cjk + env opt-out（ADR-046 D1）----

  Scenario: resolve_tokenizer unset 翻默认 code_cjk
    Given core/src/server.rs resolve_tokenizer 读 CONTEXTFORGE_TOKENIZER
    When env unset 或为空
    Then 返回 CODE_CJK_TOKENIZER（翻默认，新建 collection 默认获 code/CJK recall）
    And 生产索引两调用点（server.rs:141 CoreService::index + jobs/index_session_backend.rs:151）改 open_with_tokenizer(.., &resolve_tokenizer())

  Scenario: resolve_tokenizer "default" opt-out 回 legacy TEXT
    Given resolve_tokenizer 读 CONTEXTFORGE_TOKENIZER
    When env 为 "default"
    Then 返回 DEFAULT_TOKENIZER（opt-out 回 legacy TEXT，byte-equiv）

  Scenario: resolve_tokenizer 未知值 stderr WARN 回落 code_cjk（不静默落 TEXT）
    Given resolve_tokenizer 读 CONTEXTFORGE_TOKENIZER
    When env 为未知值，或 "cjk_segmenter" 但 cjk-segmenter feature 未编译
    Then stderr WARN 并回落 CODE_CJK_TOKENIZER（best-effort，镜像 Phase 35 surfacing）
    And 不静默落 TEXT（避免翻默认形同未生效）

  Scenario: 生产路径新建 collection 默认绑 code_cjk
    Given 生产索引路径经 open_with_tokenizer(.., &resolve_tokenizer())
    When 新建 collection（meta.json 不存在）且 env unset
    Then content 字段绑 code_cjk analyzer（content_tokenizer_name == "code_cjk"）
    And query 侧 register_code_cjk 无条件注册保 index/query 对称（task-24.1 R4）

  Scenario: 既有 TEXT collection 不受翻默认影响（schema-driven 安全）
    Given 既有 collection 的 meta.json 持久化 TEXT analyzer
    When 经生产路径 open_with_tokenizer(.., "code_cjk")
    Then open_with_tokenizer 走 Index::open_in_dir 读回持久化 schema、忽略传入 tokenizer
    And content_tokenizer_name 仍为 "default"（既有 TEXT collection 不被静默失效）

  Scenario: Phase 24 harness 复测真实 recall delta +0.0909（不预填）
    Given phase24_tokenizer_recall harness 比较 default TEXT vs code_cjk
    When 复测 over task-24.2 golden
    Then recall delta = +0.0909（default 0.9091 → code/CJK 1.0000，Phase 24 实测、本 phase 复确认成出厂基线）
    And 小 golden caveat 据实记、大语料续 [SPEC-DEFER:phase-future.tokenizer-large-corpus-recall]
    And 真实数实施回填、不预填（ADR-013）

  Scenario: 首次刻意默认变更据实定性非 byte-equiv（ADR-013）
    Given 翻默认令新建 collection 倒排词项 TEXT → code_cjk
    When 我据实记录
    Then 这是项目首次刻意默认行为变更、非 byte-equivalent
    And 由 ADR-046 D1/D4 显式承接（既有 collection 安全 + opt-out + 不自动迁移 + Phase 24 +0.0909 justify）
    And 不夸大为 byte-equiv

  # ---- task-41.2: Go [retrieval] tokenizer config 桥（ADR-046 D2）----

  Scenario: config [retrieval] tokenizer Save/Load round-trip
    Given internal/config/config.go add-only RetrievalConfig{Tokenizer} + [retrieval] 段
    When Save 含 tokenizer = "code_cjk" 再 Load
    Then round-trip 等价（镜像 VectorConfig/[vector]）
    And 既有 config.toml 无 [retrieval] 段 → 解码零值 Tokenizer=""（向后兼容）

  Scenario: setTokenizerEnv env-wins + 无段默认 code_cjk
    Given cmd/contextforge/main.go setTokenizerEnv（镜像 setVectorEnv）
    When [retrieval] tokenizer 非空且 CONTEXTFORGE_TOKENIZER 未设
    Then 导出 CONTEXTFORGE_TOKENIZER（env-wins：已设则不覆盖）
    And 无 [retrieval] 段 / 空值 → 不导出 → Rust resolve_tokenizer 默认 code_cjk（翻默认生效）
    And tokenizer 非密钥（无 api-key 字段）+ Rust core 0 toml dep

  # ---- task-41.3: v0.34.0 收口 + 刻意默认变更承接 + 0-dep 守线 + honest-defer 边界 ----

  Scenario: jieba 默认不取 + 既有 collection 不自动迁移据实延后（ADR-013）
    Given jieba cjk_segmenter 是 feature-gated 重词典 dep 且 Phase 30 实测 vs bigram delta=+0.0000
    And 既有 collection 升级须 reindex（不自动改用户数据）
    When 本 phase 据实分级
    Then 默认翻 code_cjk（纯 std 0-dep）、jieba 续 feature opt-in [SPEC-DEFER:phase-future.cjk-segmenter-default-on]
    And 既有 collection 经既有 reindex_with_tokenizer 用户主动升级 [SPEC-DEFER:phase-future.tokenizer-auto-reindex-on-upgrade]

  Scenario: v0.34.0 收口 + 默认零依赖守线
    Given task-41.1 + task-41.2 全 Done
    When task-41.3 收口
    Then scripts/console_smoke.sh v31[50/50]（production 默认 code_cjk + CONTEXTFORGE_TOKENIZER=default opt-out 端到端）+ TestTask413 无 [37/37]..[49/49] 回归
    And ADR-046 据 D1-D4 真实测试 ratify Proposed→Accepted
    And ADR-029 add-only Phase-41 Amendment（默认开启维度兑现）
    And ADR-035 add-only Phase-41 Amendment（D3 产品决策兑现）
    And ADR-004 刻意默认变更例外由 ADR-046 承接 + opt-out byte-equiv safety intent 保持
    And 0 新 dep（code_cjk 纯 std）+ 0 网络（ADR-008）
    And 真实 v0.34.0 tag/run/digest/tlog post-tag-push 回填（ADR-013 不预填）
