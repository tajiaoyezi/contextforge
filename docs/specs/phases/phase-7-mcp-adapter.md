# Phase 7 · mcp-adapter

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。`/s2v-init` 生成，Status=Draft。§6 端到端 smoke 留 `<TBD-by-user>`，本 phase 最后一个 task 完工/合并前必须填实（`s2v_preflight_phase` C1）。

## 1. 阶段目标

Agent 经 MCP 获取一致、可追溯上下文（`context_search` / `context_read` / `context_explain` / `context_collections`）。来源：PRD §Implementation Phases Phase 7。

## 2. 业务价值

把 ContextForge 接入真实多 Agent 工作流（OpenClaw/Hermes/Claude Code/Cursor/Zed），实现 PRD §Vision「Agent 经 MCP 获取一致、可追溯上下文」。MCP tool 返回字段与 REST search result 可解释字段一致（PRD §Technical Approach 要求）。

## 3. 涉及模块

- `mcp-adapter`（Go）：MCP server，暴露 4 个 tool + MCP client allowlist
- 文件锚点：`internal/mcpadapter/`

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 7.1 | mcp-adapter | `../tasks/task-7.1-mcp-server.md` |

## 5. 依赖关系

- **依赖**：Phase 6（cli-api-export：检索/导出对外接口与 result 契约）
- **可并行**：否
- **Phase 内顺序**：单 task（7.1 mcp-server）

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（参考 — PRD §Implementation Phases Phase 7 Exit Criteria，用户审定后落实）**：

- MCP `context_search` 可返回可解释结果（字段与 REST search result 一致）
- MCP `context_read` 可读取指定 chunk / context
- MCP `context_explain` 可返回召回理由和 provenance
- MCP `context_collections` 可列出可用 collection
- MCP client 未被 allowlist 时拒绝访问

**端到端 smoke**：`<TBD-by-user>`（本 phase 唯一 task=7.1 完工/合并前填实，例：起 MCP server → 用 MCP client 调 4 个 tool 校验返回字段与 REST 一致 + 未 allowlist client 被拒的 smoke 序列）

## 7. 阶段级风险

- 关联 PRD §Technical Risks **R7**（MCP 协议/SDK 漂移，TBD）：mcp-adapter 与核心检索解耦（adapter 仅做协议翻译）；锁定一个已发布 spec 版本并标注兼容范围；协议变更只动 adapter 层。关联 PRD §Open Questions O4。
- 关联 **R9**（MCP client allowlist 设计不当导致越权读取）：MCP client 显式 allowlist；audit log 记录 MCP 访问。

## 8. Phase Definition of Done

- [ ] 本 phase task spec Status=Done 或 Waived
- [ ] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过
- [ ] 关联风险 R7 / R9 缓解措施已落地（adapter 解耦 + 版本锁定 + client allowlist）
- [ ] adapter §Phase 状态索引该行 Status 同步更新
- [ ] team §4 Gate 3 phase smoke gate 通过后方可 merge（本 phase 唯一 task 即最后 task）
