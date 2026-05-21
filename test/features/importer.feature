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
    Then 返回 Name() 为 "fallback" 的 importer，Import 不返回 error，且输出包含 "warning" 的显式降级日志

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
    Given 临时目录下存在 "MEMORY.md" 内容为合法 markdown "# Project memories\n- rule 1"
    When  用 hermes.New() Import 该文件到 collection "default"
    Then  返回 1 条 ContextRecord，Content 原文保留，SchemaVersion="0.1"，CollectionId="default"

  Scenario: SCEN-3.2.2 — 对应 AC2（provider/scope/provenance）
    Given 同上 "MEMORY.md" 内容非空
    When  Import 该文件
    Then  ContextRecord 字段：SourceProvider="hermes"、AgentScope 含 "hermes"、Provenance[0].Importer="hermes-memory"、OriginalPath 等于原始路径、SourceModifiedAt 非空（取文件 mtime）；SourceType="memory"、Language="markdown"、RedactionStatus="pending"（BINDING）

  Scenario: SCEN-3.2.3 — 对应 AC3（只读不写回）
    Given "USER.md" 文件存在，记录 import 前的 mtime 与字节大小
    When  Import 该文件
    Then  Import 完成后文件 mtime 与字节大小不变（仅读取，无 Write/Truncate/Chmod 操作）

  Scenario: SCEN-3.2.4 — 对应 AC4（schema 差异降级）
    Given "MEMORY.md" 文件内容为空（仅含空白字符 / 0 字节）
    When  Import 该文件
    Then  返回 1 条 ContextRecord（来自 task-3.1 NewFileFallbackImporter），不返回 error；log 输出包含 "warning" + "fallback" 字样；不中断整体 import 流程

  # ---
  # Maps to: docs/specs/tasks/task-3.3-importer-openclaw.md
  Scenario: SCEN-3.3.1 — 对应 AC1（workspace 通用导入）
    Given an OpenClaw workspace with markdown, config, log, and memory-like files
    When the OpenClaw importer imports the workspace
    Then each supported file is returned as a ContextRecord through the generic fallback path

  Scenario: SCEN-3.3.2 — 对应 AC2（collection/字段保留）
    Given an OpenClaw workspace named "proj-a" for agent "openclaw"
    When records are imported without an explicit collection id
    Then the collection id is derived from the agent and workspace names, and file_path, source_modified_at, source_type, and agent_scope are preserved

  Scenario: SCEN-3.3.3 — 对应 AC3（不复刻/不写回）
    Given an OpenClaw workspace file with known original contents
    When the OpenClaw importer imports the workspace
    Then the source file contents are unchanged and no OpenClaw backend write path is invoked

  Scenario: SCEN-3.3.4 — 对应 AC4（schema TBD 走 fallback）
    Given an OpenClaw memory-like file whose schema is not recognised
    When the OpenClaw importer imports it
    Then it is imported as a generic ContextRecord and a fallback warning is emitted

  # ---
  # Maps to: docs/specs/tasks/task-3.4-importer-agent-rules.md
  Scenario: SCEN-3.4.1 — 对应 AC1（AGENTS/CLAUDE 导入）
    Given 临时目录下存在 "AGENTS.md" 内容为 "# Project Rules\n\n- Always run tests"
    When 用 AgentRulesImporter 直接导入该文件到 collection "default"
    Then 返回 1 条 ContextRecord，SourceType=agent_rule、provider=claude-code、Content 含规则文本、Tags 含 agent_rule、redaction_status=pending

  Scenario: SCEN-3.4.2 — 对应 AC2（Cursor/Zed rules 导入）
    Given 临时目录下存在 ".cursorrules" 内容为 "# Cursor Rules"
    When 用 AgentRulesImporter 直接 Import（bypass Resolve）
    Then SourceType=agent_rule、provider=cursor、内容被保留为 markdown

  Scenario: SCEN-3.4.3 — 对应 AC3（只读不写回）
    Given 临时目录下存在 "CLAUDE.md" 内容为 "# Memory"
    When 用 AgentRulesImporter 导入
    Then 导入成功且原 CLAUDE.md 文件字节未被修改（只读保证）

  Scenario: SCEN-3.4.4 — 对应 AC4（路径 TBD 走 fallback）
    Given 临时目录下存在 "zed/project-rules.md"（非 AGENTS/CLAUDE 命名）
    When Resolve 该路径
    Then 返回 fallback importer，Import 成功，日志包含 "warning" + "fallback"，record SourceType=file（非 agent_rule）
