# language: en
# Maps to:
#   - docs/specs/tasks/task-4.1-retriever.md
#   - docs/specs/tasks/task-4.2-explain.md
#
# 轻量 BDD（s2v §9.2）；module=retriever 跨 task 4.1/4.2，本文件追加各 task 的 Scenario 组。
# 占位场景由 task agent 实施时填 Given/When/Then。

Feature: retriever
  In order to <TBD-by-user>
  As <TBD-by-user>
  I want BM25/metadata/filter 检索 + explainable retrieval trace + 可解释 result schema

  # ---
  # Maps to: docs/specs/tasks/task-4.1-retriever.md
  Scenario: SCEN-4.1.1 — 对应 AC1（BM25+metadata Top-K）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-4.1.2 — 对应 AC2（filter 契约一致）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-4.1.3 — 对应 AC3（空/错误 query 不 panic）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-4.1.4 — 对应 AC4（性能 P95<500ms）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-4.1.5 — 对应 AC5（tokenizer/boost/exact）
    Given <TBD>
    When <TBD>
    Then <TBD>

  # ---
  # Maps to: docs/specs/tasks/task-4.2-explain.md
  Scenario: SCEN-4.2.1 — 对应 AC1（可解释字段完整）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-4.2.2 — 对应 AC2（定位回原文行号）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-4.2.3 — 对应 AC3（覆盖率≥90%/禁黑盒）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-4.2.4 — 对应 AC4（gRPC/CLI 调试入口）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-4.2.5 — 对应 AC5（Phase4 端到端 smoke）
    Given <TBD>
    When <TBD>
    Then <TBD>
