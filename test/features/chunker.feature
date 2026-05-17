# language: en
# Maps to:
#   - docs/specs/tasks/task-2.3-chunker.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: chunker
  In order to <TBD-by-user>
  As <TBD-by-user>
  I want 文档/代码 chunking + metadata 抽取 + provenance 维护 + content_hash 去重锚点

  # ---
  # Maps to: docs/specs/tasks/task-2.3-chunker.md
  Scenario: SCEN-2.3.1 — 对应 AC1（Chunk 字段完整）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-2.3.2 — 对应 AC2（provenance 多来源）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-2.3.3 — 对应 AC3（chunking 可配置）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-2.3.4 — 对应 AC4（大文件分块不爆内存）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-2.3.5 — 对应 AC5（content_hash 一致性）
    Given <TBD>
    When <TBD>
    Then <TBD>
