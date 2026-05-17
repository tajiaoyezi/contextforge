# ADR `007`: `minimal-tarball-distribution`

**Status**: Accepted
**Category**: 部署发布
**Date**: 2026-05-17
**Decided By**: tajiaoyezi
**Related**: PRD §Decisions Log D7

## Context

ContextForge 是 Go + Rust 混合双二进制产物，v0.1 优先验证价值闭环而非铺开分发渠道（PRD §Constraints 发布、§Decisions Log D7）。

## Decision

v0.1 极简分发：GitHub Release Linux x86_64 tarball（contextforge + contextforge-core + example.toml）+ 源码 self-host + Docker Compose。

## Rationale

单一语言包管理器（cargo/go/npm）无法干净分发 Go+Rust 混合产物；立即多平台+签名+自动更新在价值未验证前过早；仅 Docker 对本地 CLI/MCP 工作流不便。tarball + 源码 + Docker Compose 覆盖 v0.1 验证场景且成本最低。

## Alternatives

- **单一语言包管理器（cargo/go/npm）分发**：拒绝 —— 混合产物无法干净分发。
- **立即多平台 + 签名 + 自动更新**：拒绝 —— 价值未验证前过早。
- **仅 Docker**：拒绝 —— 对本地 CLI/MCP 工作流不便。

## Consequences

> （init agent 初稿，用户审定）

- 正向：分发实现成本最低，聚焦 v0.1 价值闭环；Linux x86_64/WSL2 覆盖目标开发环境。
- 负向/成本：macOS/Windows 用户 v0.1 需源码构建（nice-to-have，非官方 tarball）；无签名/自动更新（v1.0 目标）。
- 影响面：Phase 8 task 8.3 release-smoke 产出 tarball + smoke test。

## Rollback Or Migration Plan

> （init agent 初稿，用户审定）

回滚策略：tarball 版本化，出问题回退上一 release tag + README 标注已知问题。后续路线（v0.2 macOS tarball + Homebrew、v0.3 Windows preview、v1.0 多平台+签名+自更新）为加法式演进，不破坏 v0.1 分发（演进时新开 ADR）。

## Follow-ups

- 关联 PRD §Constraints 发布后续路线（v0.2/v0.3/v1.0）。
- 关联 PRD §Implementation Phases Phase 8（release smoke test）。
