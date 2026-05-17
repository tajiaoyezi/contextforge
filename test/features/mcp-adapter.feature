# language: en
# Maps to:
#   - docs/specs/tasks/task-7.1-mcp-server.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: mcp-adapter
  In order to <TBD-by-user>
  As <TBD-by-user>
  I want MCP server 暴露 context_search/context_read/context_explain/context_collections + client allowlist

  # ---
  # Maps to: docs/specs/tasks/task-7.1-mcp-server.md
  Scenario: SCEN-7.1.1 — 对应 AC1（context_search 一致字段）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-7.1.2 — 对应 AC2（read/explain/collections）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-7.1.3 — 对应 AC3（client allowlist 拒绝 + 审计）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-7.1.4 — 对应 AC4（adapter 解耦 + 版本锁定）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-7.1.5 — 对应 AC5（Phase7 端到端 smoke）
    Given <TBD>
    When <TBD>
    Then <TBD>
