# language: en
# Maps to:
#   - docs/specs/tasks/task-1.3-core-skeleton.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: core
  In order to <TBD-by-user>
  As <TBD-by-user>
  I want contextforge-core (Rust) 数据面骨架 + tonic gRPC server + health + 模块占位

  # ---
  # Maps to: docs/specs/tasks/task-1.3-core-skeleton.md
  Scenario: SCEN-1.3.1 — 对应 AC1（core 可启动监听 local gRPC）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-1.3.2 — 对应 AC2（gRPC health SERVING）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-1.3.3 — 对应 AC3（tonic codegen 无 FFI）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-1.3.4 — 对应 AC4（模块占位编译通过）
    Given <TBD>
    When <TBD>
    Then <TBD>
