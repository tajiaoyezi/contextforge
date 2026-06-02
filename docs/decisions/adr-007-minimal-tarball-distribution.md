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

## Amendment (Phase 28 / v0.21.0 — release-ci-hardening, add-only, 不溯改 §Decision/§Consequences)

Phase 28（release-ci-hardening，v0.21.0）据 `docs/decisions/adr-033-release-ci-hardening.md` 扩展部署发布面。本 Amendment **add-only 记录扩展结果，不溯改本 ADR §Decision/§Rationale/§Consequences 正文**（ADR-014 D5）；§Decision L39「加法式演进，演进时新开 ADR」已预见此演进（ADR-033 即该新 ADR）：

- **发布面扩展**：v0.1 的「最小 x86_64 tarball + 单架构镜像」基线**不变**；在其上加 **cosign keyless 签名 + SPDX SBOM（syft）+ SLSA provenance** 的 OCI 镜像（task-28.2，公共 Sigstore + GHCR OCI 工件，与 repo 私有无关）+ **匿名可拉取守护**（task-28.1，守 v0.10.0 GHCR-PRIVATE→403 回归）。
- **multi-arch 延后**：arm64 多架构镜像经实测 QEMU emulation 不可行（task-28.1，run 26757640892 45min 超时）→ 单架构 amd64 保底 + arm64 延后原生 runner / 交叉编译 `[SPEC-DEFER:phase-future.multi-arch-native-runner]`（如实记录，不伪造）。
- **CI 质量门**：发布配套加 CI 强 lint（clippy + gofmt + go vet 卡红，task-28.3）。

详见 `docs/decisions/adr-033-release-ci-hardening.md §Ratification`。
