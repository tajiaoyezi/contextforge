# language: en
# Maps to:
#   - docs/specs/tasks/task-2.4-indexer.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: indexer
  In order to 把 scanner→parser→chunker 产物写入本地可检索的 SQLite + Tantivy 双存储
  As Phase 4 retriever 与 Phase 5 memoryops 的上游消费者（同时为 Phase 2 端到端 smoke 落点）
  I want Tantivy 全文索引 + SQLite metadata/chunk 存储 + 基础增量 + contextforge index

  # ---
  # Maps to: docs/specs/tasks/task-2.4-indexer.md
  Scenario: SCEN-2.4.1 — 对应 AC1（索引 ≥1000 文件）
    Given 临时目录下生成 ≥1000 个 markdown 小文件
    When  IndexSession::open + index_path
    Then  IndexStats.files_indexed ≥ 1000 且 chunks_written > 0

  Scenario: SCEN-2.4.2 — 对应 AC2（SQLite+Tantivy 可查）
    Given 索引一组含独特 token "uniquephrasex9k7" 的小 fixture
    When  index_path + commit
    Then  SQLite chunks 表 ≥1 行；Tantivy 查 "uniquephrasex9k7" 返回 ≥1 命中

  Scenario: SCEN-2.4.3 — 对应 AC3（denylist+redaction 生效）
    Given fixture 含正常 token / .env (denylisted) / AWS key 模式三类文件
    When  index_path + commit
    Then  Tantivy 命中正常 token；不命中 .env 内 plain_secret；不命中原始 AWS key（已 redact 为 [REDACTED:*]）

  Scenario: SCEN-2.4.4 — 对应 AC4（基础增量更新）
    Given 已 index doc.md (含 "oldtokenx1y2z3")
    When  改写 doc.md 内容（含 "newtokenq8r9s0"）→ reindex_file
    Then  IndexStats 有 chunks 变动；新 token 可查；旧 token 已删（不命中）

  Scenario: SCEN-2.4.5 — 对应 AC5（Phase2 端到端 smoke）
    Given core/tests/phase2_smoke.rs 含 `#[test] fn phase_2_end_to_end_smoke()`
    When  cargo test --test phase2_smoke
    Then  端到端跑通：scanner → parser → chunker → indexer 双存储；AC1/AC2/AC3 断言全过；主 agent §4 Gate 3 phase-2 smoke 调用此入口
