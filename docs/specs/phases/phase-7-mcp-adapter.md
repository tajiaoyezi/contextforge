# Phase 7 · mcp-adapter

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。`/s2v-init` 生成，Status=Draft。§6 端到端 smoke 由 task-7.1 填实；自动化运行留 task-8.1 eval-harness。

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

**端到端 smoke**（命令骨架；自动化运行留 task-8.1 eval-harness）：

```bash
# Phase 7 MCP adapter smoke (task-7.1 AC5)
set -euo pipefail

ROOT="${TMPDIR:-/tmp}/cf-phase7"
rm -rf "$ROOT"
contextforge init --root "$ROOT"

# Allow one MCP client explicitly. Empty / missing allowlist must reject all.
cat > "$ROOT/mcp-allowlist.json" <<'JSON'
[{"name":"claude-desktop","version":">=0.7.0"}]
JSON
chmod 0600 "$ROOT/mcp-allowlist.json"

# Seed / import fixture data before this point when task-8.1 automates the smoke.
# v0.1 manual gate focuses on the syntactic MCP handshake + 4 tool calls.

INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"claude-desktop","version":"0.8.0"}}}'
READY='{"jsonrpc":"2.0","method":"notifications/initialized"}'
LIST='{"jsonrpc":"2.0","id":2,"method":"tools/list"}'
SEARCH='{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"context_search","arguments":{"query":"fixture query","collections":["default"],"top_k":5}}}'
READ='{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"context_read","arguments":{"chunk_id":"chk_<fixture>_0","collection":"default"}}}'
EXPLAIN='{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"context_explain","arguments":{"query":"fixture query","collections":["default"]}}}'
COLLECTIONS='{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"context_collections","arguments":{}}}'

printf '%s\n%s\n%s\n%s\n%s\n%s\n%s\n' \
  "$INIT" "$READY" "$LIST" "$SEARCH" "$READ" "$EXPLAIN" "$COLLECTIONS" \
  | contextforge mcp --data-dir "$ROOT" --allowlist "$ROOT/mcp-allowlist.json" \
  | tee "$ROOT/mcp-ok.jsonl"

# Expected manual checks:
# - initialize result protocolVersion == "2025-06-18" (newer client negotiated down)
# - tools/list contains context_search/context_read/context_explain/context_collections
# - context_search structuredContent.results uses the same RetrievalResult fields as REST /v1/search
# - context_read returns the requested chunk_id after fixture seeding
# - context_explain includes retrieval_trace with reason/provenance
# - context_collections lists default collection metadata

DENIED='{"jsonrpc":"2.0","id":7,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"not-allowed","version":"0.1.0"}}}'
printf '%s\n' "$DENIED" \
  | contextforge mcp --data-dir "$ROOT" --allowlist "$ROOT/mcp-allowlist.json" \
  | tee "$ROOT/mcp-denied.jsonl"

# Expected denied checks:
# - JSON-RPC error.code == -32000
# - "$ROOT/audit-rest.log" contains endpoint "mcp:initialize" and status 403
# - audit log does not contain full query/tool arguments
```

## 7. 阶段级风险

- 关联 PRD §Technical Risks **R7**（MCP 协议/SDK 漂移，TBD）：mcp-adapter 与核心检索解耦（adapter 仅做协议翻译）；锁定一个已发布 spec 版本并标注兼容范围；协议变更只动 adapter 层。关联 PRD §Open Questions O4。
- 关联 **R9**（MCP client allowlist 设计不当导致越权读取）：MCP client 显式 allowlist；audit log 记录 MCP 访问。

## 8. Phase Definition of Done

- [ ] 本 phase task spec Status=Done 或 Waived
- [ ] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过
- [ ] 关联风险 R7 / R9 缓解措施已落地（adapter 解耦 + 版本锁定 + client allowlist）
- [ ] adapter §Phase 状态索引该行 Status 同步更新
- [ ] team §4 Gate 3 phase smoke gate 通过后方可 merge（本 phase 唯一 task 即最后 task）
