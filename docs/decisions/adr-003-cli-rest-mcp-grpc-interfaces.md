# ADR `003`: `cli-rest-mcp-grpc-interfaces`

**Status**: Accepted
**Category**: 协议接口
**Date**: 2026-05-17
**Decided By**: tajiaoyezi
**Related**: PRD §Decisions Log D3

## Context

ContextForge 既要服务人类开发者（脚本化/调试），又要服务程序化 Agent 调用与多 Agent MCP 工作流（PRD §User Flow、§Technical Approach REST/MCP 最小接口契约草案）。同时内部需 Go↔Rust 双进程通信。

## Decision

三对外接口 + 一内部 RPC：对外 CLI / 本地 REST `/v1/*` / MCP tools（context_search/read/explain/collections）；内部 Go↔Rust 用 local gRPC。

## Rationale

仅 CLI 无法服务 Agent 程序化调用；仅 MCP 不能脚本化/调试；内部用 stdin·stdout JSON-RPC 在长任务/流式进度/并发语义上不如 gRPC 清晰。三对外接口覆盖人类 + 程序 + 多 Agent 三类消费者。

## Alternatives

- **仅 CLI**：拒绝 —— 无法服务 Agent 程序化调用。
- **仅 MCP**：拒绝 —— 不能脚本化/调试。
- **内部 stdin·stdout JSON-RPC 代替 gRPC**：拒绝 —— 长任务/流式进度/并发语义不如 gRPC 清晰。

## Consequences

> （init agent 初稿，用户审定）

- 正向：人类/程序/Agent 三类消费者各有合适入口；MCP 与 REST 返回可解释字段一致，迁移成本低。
- 负向/成本：三对外接口需维护契约一致性（result schema 单一源）；MCP spec/SDK 漂移风险（R7）。
- 影响面：Phase 1（proto/gRPC）、Phase 6（CLI/REST）、Phase 7（MCP）。

## Rollback Or Migration Plan

> （init agent 初稿，用户审定）

result schema 单一源（proto 定义），任一对外接口可独立演进而不破坏其他；MCP adapter 仅做协议翻译，与核心检索解耦，MCP spec 变更只动 adapter 层（R7 缓解），必要时可临时下线 MCP 而 CLI/REST 不受影响。

## Follow-ups

- 关联 PRD §Technical Risks R7（MCP 协议/SDK 漂移）—— Phase 7 启动前锁定版本。
- 关联 PRD §Open Questions O4（MCP 协议/SDK 目标版本）/ O10（本地 API/MCP 安全边界）。
