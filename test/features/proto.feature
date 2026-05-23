# language: en
# Maps to:
#   - docs/specs/tasks/task-1.1-proto.md
#
# 轻量 BDD（s2v §9.2）：本文件为业务可读场景文档；Scenario ID 在 task spec §7 追踪表映射到 TEST。
# /s2v-init 生成的占位场景 —— task agent 实施时按对应 AC 填 Given/When/Then。

Feature: proto
  In order to keep the Go/Rust dual-binary contract frozen and bidirectionally generatable
  As a ContextForge maintainer
  I want gRPC + canonical-record proto 契约（context/search/import/eval）冻结并可 Go/Rust 双侧 codegen

  # ---
  # Maps to: docs/specs/tasks/task-1.1-proto.md
  Scenario: SCEN-1.1.1 — 对应 AC1（ContextRecord 最小字段）
    Given the frozen schema_version "0.1" proto contract
    When a Go or Rust caller constructs a ContextRecord
    Then the minimal required fields are present and tagged identically across both languages

  Scenario: SCEN-1.1.2 — 对应 AC2（四类对象 proto）
    Given the canonical data-plane schema definition
    When the proto contract is fully parsed in Go or Rust
    Then the four objects SourceRecord, ContextRecord, Chunk, and RetrievalResult are defined with non-empty fields

  Scenario: SCEN-1.1.3 — 对应 AC3（search 契约一致）
    Given the search-related proto definitions
    When a search request is parsed or a retrieval result is populated
    Then SearchRequest and RetrievalResult fields match the PRD schema draft exactly

  Scenario: SCEN-1.1.4 — 对应 AC4（Go+Rust codegen 无 FFI）
    Given the buf/protoc configuration and tonic/prost generators
    When the dual-language codegen pipeline is executed
    Then the Go and Rust gRPC code is generated successfully without FFI runtime dependencies

  Scenario: SCEN-1.1.5 — 对应 AC5（schema 版本化冻结）
    Given the proto contract v0.1 definitions
    When checking the schema version and documentation
    Then the version must be exactly "0.1" and the add-only tag freeze rule must be documented
