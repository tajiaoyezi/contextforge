# language: en
# Maps to:
#   - docs/specs/tasks/task-2.2-parser.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: parser
  In order to <TBD-by-user>
  As <TBD-by-user>
  I want 代码(tree-sitter)/Markdown(pulldown-cmark)/日志解析，保留 language 与行号区间

  # ---
  # Maps to: docs/specs/tasks/task-2.2-parser.md
  Scenario: SCEN-2.2.1 — 对应 AC1（代码 tree-sitter 解析）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-2.2.2 — 对应 AC2（Markdown 解析）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-2.2.3 — 对应 AC3（日志解析）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-2.2.4 — 对应 AC4（未知类型降级纯文本）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-2.2.5 — 对应 AC5（language 标签保留）
    Given <TBD>
    When <TBD>
    Then <TBD>
