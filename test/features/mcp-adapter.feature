# language: en
# Maps to:
#   - docs/specs/tasks/task-7.1-mcp-server.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: mcp-adapter
  In order to reuse governed ContextForge context from MCP-capable agents
  As a local-first developer using Claude Desktop, Cursor, Zed, OpenClaw, or Hermes
  I want MCP server 暴露 context_search/context_read/context_explain/context_collections + client allowlist

  # ---
  # Maps to: docs/specs/tasks/task-7.1-mcp-server.md
  Scenario: SCEN-7.1.1 — 对应 AC1（context_search 一致字段）
    Given an allowlisted MCP client and an indexed ContextForge collection
    When the client calls the context_search tool with a query and collection
    Then the tool returns RetrievalResult fields matching REST /v1/search

  Scenario: SCEN-7.1.2 — 对应 AC2（read/explain/collections）
    Given an allowlisted MCP client and available collection directories
    When the client calls context_read, context_explain, and context_collections
    Then each tool returns a real result instead of a stub or not-implemented error

  Scenario: SCEN-7.1.3 — 对应 AC3（client allowlist 拒绝 + 审计）
    Given an empty or non-matching mcp-allowlist.json
    When a client sends the initialize JSON-RPC request
    Then the server returns JSON-RPC error -32000, closes stdio, and writes an mcp:initialize audit event

  Scenario: SCEN-7.1.4 — 对应 AC4（adapter 解耦 + 版本锁定）
    Given a client that supports a newer MCP protocol version
    When the client initializes over newline-delimited stdio JSON-RPC
    Then the server negotiates down to MCP 2025-06-18 and exposes only tool capabilities

  Scenario: SCEN-7.1.5 — 对应 AC5（Phase7 端到端 smoke）
    Given the contextforge mcp CLI subcommand with fake stdin and stdout
    When the command is dispatched with a data directory and allowlist path
    Then the MCP backend receives the stdio stream and emits JSON-RPC responses
