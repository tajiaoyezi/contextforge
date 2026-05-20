# language: en
# Maps to:
#   - docs/specs/tasks/task-2.2-parser.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: parser
  In order to parse source files into language-aware units with precise line ranges
  As the index-core pipeline (scanner → parser → chunker)
  I want reliable code/Markdown/log parsing that preserves language + provenance for explainable retrieval

  # Maps to: docs/specs/tasks/task-2.2-parser.md §6 AC1-5 / §7 SCEN-2.2.*

  Scenario: SCEN-2.2.1 — 代码 tree-sitter 解析 (AC1, TEST-2.2.1)
    Given a .rs file containing "fn main() {}" and "struct Foo"
    When parser::parse_content is invoked with language hint "rust"
    Then it returns >=1 ParsedUnit with language="rust", valid line range, and kind present

  Scenario: SCEN-2.2.2 — Markdown 解析 (AC2, TEST-2.2.2)
    Given a .md file with "# Title", a fenced code block, and a paragraph
    When parser::parse_content is invoked with language hint "markdown"
    Then it returns units covering the structure with line_start/line_end and language="markdown"

  Scenario: SCEN-2.2.3 — 日志 / JSONL 解析 (AC3, TEST-2.2.3)
    Given a .log file with timestamped lines and a JSONL record
    When parser::parse_content is invoked with language hint "log"
    Then it returns units with language="log" and line ranges

  Scenario: SCEN-2.2.4 — 未知扩展名降级纯文本 (AC4, TEST-2.2.4)
    Given a file with unknown extension ".bin"
    When parser::parse_content is invoked
    Then it falls back to language="text" without error

  Scenario: SCEN-2.2.5 — language 标签保留 (AC5, TEST-2.2.5)
    Given any supported extension (e.g. .py)
    When parser::parse_content is invoked with matching hint
    Then the returned ParsedUnit carries the exact language label for downstream (chunker/indexer/tokenizer)
