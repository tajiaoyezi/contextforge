# language: en
# Maps to:
#   - docs/specs/tasks/task-8.3-release-smoke.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: release
  In order to <TBD-by-user>
  As <TBD-by-user>
  I want Linux x86_64 release 打包 + smoke test + 性能基准 + v0.1 七项闭环端到端

  # ---
  # Maps to: docs/specs/tasks/task-8.3-release-smoke.md
  Scenario: SCEN-8.3.1 — 对应 AC1（tarball 产物完整）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-8.3.2 — 对应 AC2（release smoke 通过）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-8.3.3 — 对应 AC3（P95<500ms 基准）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-8.3.4 — 对应 AC4（v0.1 七项闭环跑通）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-8.3.5 — 对应 AC5（phase §6 端到端 smoke）
    Given <TBD>
    When <TBD>
    Then <TBD>
