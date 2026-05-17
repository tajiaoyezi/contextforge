# language: en
# Maps to:
#   - docs/specs/tasks/task-8.1-eval-harness.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: eval
  In order to <TBD-by-user>
  As <TBD-by-user>
  I want golden questions 加载 + recall eval（Top-5/10 命中率/延迟/错误召回）+ eval dataset 导出

  # ---
  # Maps to: docs/specs/tasks/task-8.1-eval-harness.md
  Scenario: SCEN-8.1.1 — 对应 AC1（golden ds ≥30/每类≥5）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-8.1.2 — 对应 AC2（Strong/Weak/Miss 规则）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-8.1.3 — 对应 AC3（eval run 输出报告）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-8.1.4 — 对应 AC4（延迟不含远程）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-8.1.5 — 对应 AC5（导出 eval JSONL）
    Given <TBD>
    When <TBD>
    Then <TBD>
