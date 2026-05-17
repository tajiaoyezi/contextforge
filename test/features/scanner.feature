# language: en
# Maps to:
#   - docs/specs/tasks/task-2.1-scanner.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: scanner
  In order to <TBD-by-user>
  As <TBD-by-user>
  I want 文件扫描 + denylist/allowlist 过滤 + secret 扫描/redaction（不改原文件）

  # ---
  # Maps to: docs/specs/tasks/task-2.1-scanner.md
  Scenario: SCEN-2.1.1 — 对应 AC1（denylist 默认跳过）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-2.1.2 — 对应 AC2（allowlist 模型）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-2.1.3 — 对应 AC3（secret redact 不改原文件）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-2.1.4 — 对应 AC4（scan --dry-run 预检）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-2.1.5 — 对应 AC5（超大文件流式保护）
    Given <TBD>
    When <TBD>
    Then <TBD>
