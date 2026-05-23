# language: en
# Maps to:
#   - docs/specs/tasks/task-6.3-exporter.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: exporter
  In order to migrate governed context between agents without writing back
  As a local-first ContextForge user
  I want canonical JSONL, Markdown bundle, and Agent draft export with a second secret scan

  # ---
  # Maps to: docs/specs/tasks/task-6.3-exporter.md
  Scenario: SCEN-6.3.1 — 对应 AC1（jsonl/md-bundle 导出）
    Given a collection with canonical ContextRecord entries
    When I export it as jsonl and markdown-bundle
    Then jsonl contains one ContextRecord per line and markdown-bundle contains .md files plus manifest.json

  Scenario: SCEN-6.3.2 — 对应 AC2（agent-draft 不写回）
    Given canonical records with memory, user, agents, and claude scopes
    When I export them as agent-draft to a user-selected directory
    Then MEMORY.md, USER.md, AGENTS.md, and CLAUDE.md are created and protected agent home paths are rejected

  Scenario: SCEN-6.3.3 — 对应 AC3（export 二次 secret scan）
    Given serialized export bytes with common token, key, private-key, and password patterns
    When the exporter runs its Go inline sanity scan
    Then hits are reported and the export is refused before writing

  Scenario: SCEN-6.3.4 — 对应 AC4（迁移保真率≥80%）
    Given fixture records with all 23 ContextRecord fields populated
    When I calculate fidelity for jsonl, markdown-bundle, and agent-draft output
    Then jsonl and markdown-bundle score at least 80 percent and agent-draft scores at least 60 percent

  Scenario: SCEN-6.3.5 — 对应 AC5（Phase6 端到端 smoke）
    Given the task-6.1 search path is available through daemon.Search
    When contextforge export runs all three format flags through the CLI
    Then it pseudo full-scans with query "*" and produces the requested output artifacts
