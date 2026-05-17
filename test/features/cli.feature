# language: en
# Maps to:
#   - docs/specs/tasks/task-1.4-cli-init.md
#   - docs/specs/tasks/task-6.1-cli-search.md
#
# 轻量 BDD（s2v §9.2）；module=cli 跨 task 1.4 / 6.1，本文件追加各 task 的 Scenario 组。
# 占位场景由 task agent 实施时填 Given/When/Then。

Feature: cli
  In order to <TBD-by-user>
  As <TBD-by-user>
  I want CLI 入口——命令解析、配置加载、子命令编排（init/import/index/search/serve/mcp/eval/export）

  # ---
  # Maps to: docs/specs/tasks/task-1.4-cli-init.md
  Scenario: SCEN-1.4.1 — 对应 AC1（init 生成配置/目录）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-1.4.2 — 对应 AC2（daemon 拉起 core + gRPC health）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-1.4.3 — 对应 AC3（core 崩溃自动重启）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-1.4.4 — 对应 AC4（CLI 子命令注册）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-1.4.5 — 对应 AC5（Phase1 端到端 smoke）
    Given <TBD>
    When <TBD>
    Then <TBD>

  # ---
  # Maps to: docs/specs/tasks/task-6.1-cli-search.md
  Scenario: SCEN-6.1.1 — 对应 AC1（search 返回 Top-K）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-6.1.2 — 对应 AC2（flags 契约一致）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-6.1.3 — 对应 AC3（可解释字段 + --json）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-6.1.4 — 对应 AC4（不展示完整 secret）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-6.1.5 — 对应 AC5（与 export 共享结果模型）
    Given <TBD>
    When <TBD>
    Then <TBD>
