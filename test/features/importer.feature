# language: en
# Maps to:
#   - docs/specs/tasks/task-3.1-importer-core.md
#   - docs/specs/tasks/task-3.2-importer-hermes.md
#   - docs/specs/tasks/task-3.3-importer-openclaw.md
#   - docs/specs/tasks/task-3.4-importer-agent-rules.md
#
# 轻量 BDD（s2v §9.2）；module=importer 跨 task 3.1-3.4，本文件追加各 task 的 Scenario 组。
# 占位场景由 task agent 实施时填 Given/When/Then。

Feature: importer
  In order to <TBD-by-user>
  As <TBD-by-user>
  I want Agent 适配编排（openclaw/hermes/agent-rules）只读导入 + canonical record 映射 + 分层 fallback

  # ---
  # Maps to: docs/specs/tasks/task-3.1-importer-core.md
  Scenario: SCEN-3.1.1 — 对应 AC1（Importer 抽象只读）
    Given 全局 importer 注册表为空
    When 注册名为 "mock-test" 的 importer 并 Resolve 路径 "/any/path/mock-test.txt"
    Then 返回的 importer Name() 等于 "mock-test"

  Scenario: SCEN-3.1.2 — 对应 AC2（通用 fallback 保底）
    Given 临时目录下存在文件 "note.md" 内容为 "# Hello"
    When 用 FileFallbackImporter 导入该文件到 collection "default"
    Then 返回 1 条 ContextRecord，Content 等于 "# Hello"

  Scenario: SCEN-3.1.3 — 对应 AC3（未识别降级 + warning）
    Given 临时目录下存在文件 "weird.xyz" 内容为 "data"
    When Resolve 该路径
    Then 返回 Name() 为 "fallback" 的 importer，Import 不返回 error

  Scenario: SCEN-3.1.4 — 对应 AC4（映射核心字段完整）
    Given 临时目录下存在文件 "config.yaml" 内容为 "key: val"
    When 用 FileFallbackImporter 导入到 collection "proj-a"
    Then ContextRecord 的 SchemaVersion、CollectionId、SourceType、SourceProvider、SourceUri、FilePath、Language、ContentHash、Provenance 均非空，且 LineStart 等于 1

  Scenario: SCEN-3.1.5 — 对应 AC5（importer/record 解耦）
    Given 注册两个 mock importer "low"(confidence 0.3) 和 "high"(confidence 0.9)
    When Resolve 路径 "/any/path/high.txt"
    Then 返回 "high" importer；且 low 与 high 产生的 ContextRecord SchemaVersion 相同

  # ---
  # Maps to: docs/specs/tasks/task-3.2-importer-hermes.md
  Scenario: SCEN-3.2.1 — 对应 AC1（Hermes 导入为 record）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-3.2.2 — 对应 AC2（provider/scope/provenance）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-3.2.3 — 对应 AC3（只读不写回）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-3.2.4 — 对应 AC4（schema 差异降级）
    Given <TBD>
    When <TBD>
    Then <TBD>

  # ---
  # Maps to: docs/specs/tasks/task-3.3-importer-openclaw.md
  Scenario: SCEN-3.3.1 — 对应 AC1（workspace 通用导入）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-3.3.2 — 对应 AC2（collection/字段保留）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-3.3.3 — 对应 AC3（不复刻/不写回）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-3.3.4 — 对应 AC4（schema TBD 走 fallback）
    Given <TBD>
    When <TBD>
    Then <TBD>

  # ---
  # Maps to: docs/specs/tasks/task-3.4-importer-agent-rules.md
  Scenario: SCEN-3.4.1 — 对应 AC1（AGENTS/CLAUDE 导入）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-3.4.2 — 对应 AC2（Cursor/Zed rules 导入）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-3.4.3 — 对应 AC3（只读不写回）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-3.4.4 — 对应 AC4（路径 TBD 走 fallback）
    Given <TBD>
    When <TBD>
    Then <TBD>
