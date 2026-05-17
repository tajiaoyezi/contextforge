# language: en
# Maps to:
#   - docs/specs/tasks/task-2.1-scanner.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: scanner
  In order to keep sensitive local files out of the index
  As a local-first ContextForge user
  I want 文件扫描 + denylist/allowlist 过滤 + secret 扫描/redaction（不改原文件）

  # ---
  # Maps to: docs/specs/tasks/task-2.1-scanner.md
  Scenario: SCEN-2.1.1 — 对应 AC1（denylist 默认跳过）
    Given a project tree containing source files and default-denylisted paths
    When the scanner walks the project with default options
    Then denylisted paths are skipped and do not appear in scan results

  Scenario: SCEN-2.1.2 — 对应 AC2（allowlist 模型）
    Given a project tree and a collection allowlist
    When the scanner walks the project
    Then only allowlisted paths are scanned and denylist override requires explicit confirmation

  Scenario: SCEN-2.1.3 — 对应 AC3（secret redact 不改原文件）
    Given a file containing API key, Bearer token, private key, AWS, GitHub token, password, and cookie samples
    When the scanner scans the file
    Then it returns redacted content with typed redaction labels and leaves the source file unchanged

  Scenario: SCEN-2.1.4 — 对应 AC4（scan --dry-run 预检）
    Given a file that would be redacted
    When the scanner runs in dry-run mode
    Then it lists redaction hits without producing indexable redacted content

  Scenario: SCEN-2.1.5 — 对应 AC5（超大文件流式保护）
    Given a file larger than the configured maximum file size
    When the scanner evaluates it
    Then it records a TooLarge skip reason and does not read the file into memory
