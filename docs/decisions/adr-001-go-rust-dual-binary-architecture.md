# ADR `001`: `go-rust-dual-binary-architecture`

**Status**: Accepted
**Category**: 架构
**Date**: 2026-05-17
**Decided By**: tajiaoyezi
**Related**: PRD §Decisions Log D1

## Context

ContextForge 是本地优先的多 Agent 上下文基础设施，需同时具备成熟的 CLI/服务/MCP 编排生态与高性能的扫描/解析/索引/检索能力（PRD §Problem Statement 痛点 1-5、§Technical Approach）。单一语言难以同时最优满足"系统怎么被使用"与"上下文怎么被高效处理"。

## Decision

采用控制面/数据面分离的双二进制架构：Go 控制面（CLI/daemon/REST/MCP/编排）+ Rust 数据面（scan/parse/chunk/index/retrieve），经 local gRPC 通信，不用 FFI/cgo。

## Rationale

纯 Go：tree-sitter·tantivy 级检索解析生态与性能弱于 Rust；纯 Rust：CLI/MCP/配置编排生态不如 Go 成熟、迭代慢；FFI/cgo：引入内存归属/panic 边界/构建复杂度，v0.1 不值得用这复杂度换那点性能。gRPC 边界清晰，Go/Rust 可独立开发调试。

## Alternatives

- **纯 Go 单体**：拒绝 —— 检索/解析生态与性能不足。
- **纯 Rust 单体**：拒绝 —— CLI/MCP/配置编排生态不成熟、迭代慢。
- **Go+Rust FFI/cgo**：拒绝 —— 内存归属/panic 边界/构建复杂度，v0.1 收益不抵成本（后期性能瓶颈明确后再评估）。

## Consequences

> （init agent 初稿，用户审定）

- 正向：Go/Rust 各取所长，进程边界清晰，可独立测试与替换；契约化（proto）降低耦合。
- 负向/成本：双工具链（Go + Rust）构建与 CI 复杂度上升；gRPC 序列化有额外开销；需维护 proto 契约与版本化（见 R1）。
- 影响面：所有 phase 都跨越或依赖该边界；adapter §Commands 用 `&&` 串联双语言命令。

## Rollback Or Migration Plan

> （init agent 初稿，用户审定）

若 gRPC 边界成为确证瓶颈：先以同 proto 契约切换传输（Unix socket 调优 / 共享内存），仍不足再评估 FFI/cgo（须新开 ADR 取代本 ADR）。proto 契约版本化（仅加字段、不删不改 tag）保证回退/迁移不破坏既有数据。

## Follow-ups

- 关联 PRD §Technical Risks R1（Go↔Rust gRPC 边界复杂度）—— Phase 1 task 1.1 冻结契约。
- 关联 PRD §Open Questions O9（canonical record schema/版本/兼容策略冻结）。
