# language: en
# Maps to:
#   - docs/specs/tasks/task-1.2-config.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: config
  In order to manage local-first ContextForge configuration safely (denylist defaults, opt-in remote providers)
  As a local-first ContextForge user
  I want TOML 配置 + 默认 denylist/allowlist + collection/agent scope + 远程 provider opt-in 管理

  # ---
  # Maps to: docs/specs/tasks/task-1.2-config.md
  Scenario: SCEN-1.2.1 — 对应 AC1（默认配置/目录生成）
    Given a non-existent local configuration root directory
    When the config Init function is called
    Then a default config.toml and data-dir directories are created, and config round-trip remains consistent

  Scenario: SCEN-1.2.2 — 对应 AC2（默认 denylist 完整）
    Given the default security configuration settings
    When loading the default denylist paths
    Then it must contain all sixteen security-sensitive patterns including env files, private keys, and git objects

  Scenario: SCEN-1.2.3 — 对应 AC3（allowlist 导入模型）
    Given a custom collection configuration with allowlist paths
    When saving and loading the collection configuration with denylist override set to true
    Then the allowlist patterns and agent scope are fully preserved, and the override flag is successfully persisted

  Scenario: SCEN-1.2.4 — 对应 AC4（文件权限 0600）
    Given the local-first security sandbox requirements on Linux
    When the configuration file is initialized on disk
    Then the config.toml and token files must have 0600 permissions, and the data directory must have 0700 permissions

  Scenario: SCEN-1.2.5 — 对应 AC5（远程 provider 默认关）
    Given the default privacy-first configuration
    When checking the remote provider setting or enabling it explicitly
    Then the remote provider is disabled by default, and can only be enabled via explicit user opt-in
