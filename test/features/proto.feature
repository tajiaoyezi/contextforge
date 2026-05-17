# language: en
# Maps to:
#   - docs/specs/tasks/task-1.1-proto.md
#
# 轻量 BDD（s2v §9.2）：本文件为业务可读场景文档；Scenario ID 在 task spec §7 追踪表映射到 TEST。
# /s2v-init 生成的占位场景 —— task agent 实施时按对应 AC 填 Given/When/Then。

Feature: proto
  In order to <TBD-by-user>
  As <TBD-by-user>
  I want gRPC + canonical-record proto 契约（context/search/import/eval）冻结并可 Go/Rust 双侧 codegen

  # ---
  # Maps to: docs/specs/tasks/task-1.1-proto.md
  Scenario: SCEN-1.1.1 — 对应 AC1（ContextRecord 最小字段）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-1.1.2 — 对应 AC2（四类对象 proto）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-1.1.3 — 对应 AC3（search 契约一致）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-1.1.4 — 对应 AC4（Go+Rust codegen 无 FFI）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-1.1.5 — 对应 AC5（schema 版本化冻结）
    Given <TBD>
    When <TBD>
    Then <TBD>
