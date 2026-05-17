# ADR `004`: `local-first-privacy-baseline`

**Status**: Accepted
**Category**: 安全
**Date**: 2026-05-17
**Decided By**: tajiaoyezi
**Related**: PRD §Decisions Log D4

## Context

ContextForge 处理高度敏感本地数据（源码/配置/日志/Agent memory/调试记录，可能含 token/key/password/cookie/内部 URL）。目标用户含隐私敏感开发者；v0.1 不追求 GDPR/SOC2/HIPAA 正式合规但必须有工程化隐私基线（PRD §Problem Statement、§Constraints 安全 + Local service security baseline）。

## Decision

本地优先隐私基线 + 默认脱敏 + 远程显式 opt-in：默认本地不上传；denylist 敏感路径；secret 检测 + redaction 后入索引（不改原文件）；远程 provider 显式 opt-in；检索/导出写 audit log。

## Rationale

靠用户自管必泄露（目标用户处理含 key 的代码/日志）；索引前强制全量人工审查破坏"3-5 分钟恢复上下文"体验；全库加密作为 v0.1 默认能力过重（密钥管理/恢复/备份/性能复杂度）。denylist+redaction+audit+explicit remote opt-in 性价比最高。

## Alternatives

- **不做 secret 处理靠用户自管**：拒绝 —— 必泄露。
- **索引前强制全量人工审查**：拒绝 —— 破坏核心体验。
- **全库加密**：拒绝（v0.1）—— 默认能力过重，可作 v0.2+ 增强安全能力评估。

## Consequences

> （init agent 初稿，用户审定）

- 正向：隐私敏感用户可放心接入；secret 不以明文进索引/导出；行为可审计。
- 负向/成本：redaction 漏检/误报风险（R4）；denylist 过严可能漏索引有效内容（需 allowlist override + 显式确认）；audit log 自身需脱敏。
- 影响面：scanner（denylist/secret）、memoryops（redaction_status/audit）、daemon/MCP（监听限制/token/allowlist）、exporter（二次扫描）。

## Rollback Or Migration Plan

> （init agent 初稿，用户审定）

隐私基线为加法式安全（默认更安全）；如需放宽（如团队互信场景）经 adapter/config 显式开关，不改默认。全库加密作为 v0.2+ 增强项可叠加而非替换本基线（新 ADR）。

## Follow-ups

- 关联 PRD §Technical Risks R4（secret redaction 漏检/误报）、R9（本地 daemon/MCP 暴露面）。
- 关联 PRD §Open Questions O7（v0.1 威胁模型边界）/ O10（本地 API/MCP 安全边界）。
