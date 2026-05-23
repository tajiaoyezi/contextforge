# language: en
# Maps to:
#   - docs/specs/tasks/task-8.1-eval-harness.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: eval
  In order to validate recall quality before v0.1 release
  As a ContextForge release operator
  I want golden questions 加载 + recall eval（Top-5/10 命中率/延迟/错误召回）+ eval dataset 导出

  # ---
  # Maps to: docs/specs/tasks/task-8.1-eval-harness.md
  Scenario: SCEN-8.1.1 — 对应 AC1（golden ds ≥30/每类≥5）
    Given the built-in eval dataset
    When the dataset is validated
    Then it contains 30 golden questions across 6 categories with at least 5 questions per category

  Scenario: SCEN-8.1.2 — 对应 AC2（Strong/Weak/Miss 规则）
    Given a golden question with expected chunk and file line anchors
    When search results contain exact, partial, or unrelated hits
    Then the evaluator classifies them as Strong, Weak, or Miss using Top-5 and Top-10 semantics

  Scenario: SCEN-8.1.3 — 对应 AC3（eval run 输出报告）
    Given a search backend and a valid eval dataset
    When contextforge eval run executes
    Then stdout includes Top-5, Top-10, latency, weak hit, and miss case summary fields

  Scenario: SCEN-8.1.4 — 对应 AC4（延迟不含远程）
    Given evaluation receives measured local search duration
    When the report aggregates latency
    Then it records only the supplied search duration and does not add provider or embedding timing

  Scenario: SCEN-8.1.5 — 对应 AC5（导出 eval JSONL）
    Given the built-in eval dataset
    When the operator passes --export-jsonl
    Then the dataset is written as JSONL and can be loaded back without losing required fields
