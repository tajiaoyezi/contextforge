# language: en
# Maps to:
#   - docs/specs/tasks/task-8.2-reliability.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: reliability
  In order to ship v0.1 without losing long-running index progress or safety guarantees
  As a local-first ContextForge operator
  I want 长任务/中断恢复 + 资源占用硬化 + secret redaction/export/audit 回归

  # ---
  # Maps to: docs/specs/tasks/task-8.2-reliability.md
  Scenario: SCEN-8.2.1 — 对应 AC1（中断可恢复/续传）
    Given an incomplete index resume manifest
    When contextforge index --resume starts again with the same source and collection
    Then it loads the checkpoint and continues without treating the run as a fresh full index

  Scenario: SCEN-8.2.2 — 对应 AC2（资源占用达标）
    Given measured daemon, indexing, and search memory samples
    When the resource budget checker evaluates them
    Then samples within the PRD budgets pass and samples above a budget fail with the violated budget name

  Scenario: SCEN-8.2.3 — 对应 AC3（secret/export 回归）
    Given redaction, export, and audit safety signals from regression checks
    When the reliability guard evaluates them
    Then any missing redaction/export/audit signal fails the release reliability check

  Scenario: SCEN-8.2.4 — 对应 AC4（长任务模式降级）
    Given a large changed item count
    When contextforge index is invoked
    Then it enters resumable long-task mode and prints the resume manifest location
