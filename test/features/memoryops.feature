# language: en
# Maps to:
#   - docs/specs/tasks/task-5.1-dedup.md
#   - docs/specs/tasks/task-5.2-lifecycle.md
#   - docs/specs/tasks/task-5.3-audit.md
#
# 轻量 BDD（s2v §9.2）；module=memoryops 跨 task 5.1-5.3，本文件追加各 task 的 Scenario 组。
# 占位场景由 task agent 实施时填 Given/When/Then。

Feature: memoryops
  In order to avoid duplicate memory pollution while preserving source traceability
  As a multi-agent ContextForge user
  I want 去重 / 冲突检测 / 过期标记 / provenance 合并 / 审计事件（v0.1 能力边界内）

  # ---
  # Maps to: docs/specs/tasks/task-5.1-dedup.md
  Scenario: SCEN-5.1.1 — 对应 AC1（exact duplicate 去重）
    Given two ContextRecords with the same normalized content_hash from different sources
    When MemoryOps dedup runs on the records
    Then only one representative record remains and the duplicate is reported

  Scenario: SCEN-5.1.2 — 对应 AC2（provenance 链合并）
    Given duplicate ContextRecords with different importer, original_path, and source_modified_at provenance entries
    When MemoryOps dedup merges them
    Then the representative record keeps all distinct provenance entries

  Scenario: SCEN-5.1.3 — 对应 AC3（不做语义去重 边界）
    Given two records that are semantically similar but have different literal content_hash values
    When MemoryOps dedup runs
    Then both records remain because v0.1 does not perform semantic deduplication

  Scenario: SCEN-5.1.4 — 对应 AC4（content_hash 锚点）
    Given records carrying chunker-produced content_hash values in sha256-prefixed format
    When MemoryOps dedup groups records
    Then it uses the provided content_hash as the dedup anchor without recalculating content

  # ---
  # Maps to: docs/specs/tasks/task-5.2-lifecycle.md
  Scenario: SCEN-5.2.1 — 对应 AC1（stale 三触发可设/检索）
    Given a set of ContextRecords where one has past expires_at, one has a provenance.original_path that no longer exists, and one whose fs mtime is newer than its provenance.source_modified_at
    When MemoryOps lifecycle Mark runs with a deterministic Oracle (clock + filesystem)
    Then each affected record gets a StaleMark with the correct reason (expired / source-deleted / source-modified) and healthy records are not marked

  Scenario: SCEN-5.2.2 — 对应 AC2（基础冲突检测提示）
    Given two records sharing the same source_uri with different content_hash, and two other records sharing the same file_path with different content_hash
    When MemoryOps lifecycle Mark runs
    Then it emits one ConflictReport per source_uri group and one per file_path group, each listing the participating record ids deterministically

  Scenario: SCEN-5.2.3 — 对应 AC3（不做语义冲突 边界）
    Given two records that are semantically similar but have different content_hash AND non-overlapping source_uri AND non-overlapping file_path
    When MemoryOps lifecycle Mark runs
    Then it does NOT report them as conflicting (proves no LLM / embedding semantic analysis is performed — v0.1 边界硬约束)

  Scenario: SCEN-5.2.4 — 对应 AC4（检索可排除 stale）
    Given a list of records and a list of StaleMarks naming a subset of those records
    When the caller invokes lifecycle.FilterStale(records, marks)
    Then the returned list omits every record whose id is in marks, preserves the original order of the remaining records, and does not mutate the input slice

  # ---
  # Maps to: docs/specs/tasks/task-5.3-audit.md
  Scenario: SCEN-5.3.1 — 对应 AC1（四类事件写 audit.log）
    Given a collection audit sink
    When import, search, export, and redact operations are recorded
    Then audit.log contains one event for each operation

  Scenario: SCEN-5.3.2 — 对应 AC2（默认字段不含 query 全文）
    Given a search operation with sensitive raw query content
    When the search audit event is recorded
    Then audit.log stores query hash and length but not the raw query text

  Scenario: SCEN-5.3.3 — 对应 AC3（不记录完整 secret/导出）
    Given redact and export operations containing sensitive source material
    When the audit events are recorded
    Then audit.log keeps redaction labels and chunk ids without full secret or export content

  Scenario: SCEN-5.3.4 — 对应 AC4（secret override 写 audit）
    Given scanner secret override is explicitly confirmed
    When the override is translated into an audit event
    Then audit.log records a redact event with only redaction labels

  Scenario: SCEN-5.3.5 — 对应 AC5（Phase5 端到端 smoke）
    Given duplicate facts, a redacted secret, and a Phase 5 smoke collection
    When chunker, indexer, retrieval, stale marking, and audit logging run
    Then provenance is preserved, stale state is retrievable, four audit operations exist, and no full secret appears in audit.log
