# language: en
# Maps to:
#   - docs/specs/tasks/task-6.2-rest-api.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: daemon
  In order to <TBD-by-user>
  As <TBD-by-user>
  I want 本地 REST API server (/v1/*) + 长任务调度 + 本地监听/token 安全基线

  # ---
  # Maps to: docs/specs/tasks/task-6.2-rest-api.md
  Scenario: SCEN-6.2.1 — 对应 AC1（/v1/search 契约一致）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-6.2.2 — 对应 AC2（其余 /v1/* 可用）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-6.2.3 — 对应 AC3（默认本地监听禁 0.0.0.0）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-6.2.4 — 对应 AC4（token 0600）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-6.2.5 — 对应 AC5（无 token 拒绝 + 审计）
    Given <TBD>
    When <TBD>
    Then <TBD>
