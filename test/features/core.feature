# language: en
# Maps to:
#   - docs/specs/tasks/task-1.3-core-skeleton.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: core
  In order to supervise contextforge-core (Rust data-plane) over local gRPC safely
  As a contextforge daemon supervisor
  I want contextforge-core (Rust) 数据面骨架 + tonic gRPC server + health + 模块占位

  # ---
  # Maps to: docs/specs/tasks/task-1.3-core-skeleton.md
  Scenario: SCEN-1.3.1 — 对应 AC1（core 可启动监听 local gRPC）
    Given the local gRPC server address resolver
    When parsing the listen argument option
    Then wildcard address 0.0.0.0 is rejected, while loopback and Unix socket paths are resolved successfully

  Scenario: SCEN-1.3.2 — 对应 AC2（gRPC health SERVING）
    Given a running contextforge-core gRPC server
    When a health check request is sent to the server
    Then the server responds with a SERVING status

  Scenario: SCEN-1.3.3 — 对应 AC3（tonic codegen 无 FFI）
    Given the codegen assemblies and empty service structures
    When calling the unimplemented search RPC method
    Then the client receives a gRPC Unimplemented status without FFI marshaling issues

  Scenario: SCEN-1.3.4 — 对应 AC4（模块占位编译通过）
    Given the Rust data-plane codebase skeleton
    When the modules are referenced in the crate interface
    Then the scanner, parser, chunker, indexer, retriever, and memoryops module placeholders compile and report ready
