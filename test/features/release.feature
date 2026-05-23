# language: en
# Maps to:
#   - docs/specs/tasks/task-8.3-release-smoke.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: release
  In order to ship a repeatable v0.1 tarball with auditable smoke evidence
  As a v0.1 release owner
  I want Linux x86_64 release 打包 + smoke test + 性能基准 + v0.1 七项闭环端到端

  # ---
  # Maps to: docs/specs/tasks/task-8.3-release-smoke.md
  Scenario: SCEN-8.3.1 — 对应 AC1（tarball 产物完整）
    Given a Linux amd64 tarball candidate
    When the release validator opens the tarball
    Then the required binaries, example config, README, and LICENSE are present with executable binary modes

  Scenario: SCEN-8.3.2 — 对应 AC2（release smoke 通过）
    Given release smoke evidence for unpack, init, import, index, search, MCP, export, and eval run
    When the smoke validator checks the evidence
    Then every required step is present, ordered, and successful

  Scenario: SCEN-8.3.3 — 对应 AC3（P95<500ms 基准）
    Given a benchmark report over at least 100000 chunks
    When the benchmark gate checks BM25, metadata, and filter P95 latency
    Then each P95 is below 500 ms

  Scenario: SCEN-8.3.4 — 对应 AC4（v0.1 七项闭环跑通）
    Given the release smoke evidence covers import, index, CLI/API search, MCP, explainable retrieval, eval, and reliability
    When the v0.1 closure validator checks the step names and outcomes
    Then the seven technical closure areas are represented by passing evidence

  Scenario: SCEN-8.3.5 — 对应 AC5（phase §6 端到端 smoke）
    Given Phase 8 is at its final task
    When the Gate 3 phase smoke command runs
    Then scripts/release_smoke.sh exits 0 and prints release smoke evidence
