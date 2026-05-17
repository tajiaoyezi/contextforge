# language: en
# Maps to:
#   - docs/specs/tasks/task-2.4-indexer.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: indexer
  In order to <TBD-by-user>
  As <TBD-by-user>
  I want Tantivy 全文索引 + SQLite metadata/chunk 存储 + 基础增量 + contextforge index

  # ---
  # Maps to: docs/specs/tasks/task-2.4-indexer.md
  Scenario: SCEN-2.4.1 — 对应 AC1（索引 ≥1000 文件）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-2.4.2 — 对应 AC2（SQLite+Tantivy 可查）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-2.4.3 — 对应 AC3（denylist+redaction 生效）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-2.4.4 — 对应 AC4（基础增量更新）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-2.4.5 — 对应 AC5（Phase2 端到端 smoke）
    Given <TBD>
    When <TBD>
    Then <TBD>
