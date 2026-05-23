# Phase 6 · cli-api-export

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。`/s2v-init` 生成，Status=Draft。§6 端到端 smoke 初始留待用户审定，本 phase 最后一个 task 完工/合并前必须填实（`s2v_preflight_phase` C1）。

## 1. 阶段目标

`contextforge search` / REST `/v1/search` / `contextforge export` 可用；导出 canonical JSONL / Markdown bundle / agent draft，迁移字段保真 ≥ 80%。来源：PRD §Implementation Phases Phase 6。

## 2. 业务价值

把前 5 个 phase 的能力对外暴露为可用工作流，实现 PRD 核心能力 #5（跨 Agent 上下文迁移）的导出侧。直接支撑主指标「上下文重建时间 ≤ 3-5 分钟」与次指标「跨 Agent 迁移保真 ≥ 80% 结构化字段」。

## 3. 涉及模块

- `cli`（Go）：`contextforge search` / `contextforge export` 命令
- `daemon`（Go）：本地 REST API server（`/v1/search` 等，契约见 PRD §Technical Approach）
- `exporter`（Go）：canonical JSONL / Markdown bundle / agent draft 导出 + export 二次 secret scan
- 文件锚点：`internal/cli/` · `internal/daemon/` · `internal/exporter/`

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 6.1 | cli | `../tasks/task-6.1-cli-search.md` |
| 6.2 | daemon | `../tasks/task-6.2-rest-api.md` |
| 6.3 | exporter | `../tasks/task-6.3-exporter.md` |

## 5. 依赖关系

- **依赖**：Phase 4（retrieval-explain）+ Phase 5（memoryops）
- **可并行**：否
- **Phase 内顺序**：6.1 cli-search 先行 → 6.2 rest-api（dep 6.1）∥ 6.3 exporter（dep 6.1）

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（参考 — PRD §Implementation Phases Phase 6 Exit Criteria，用户审定后落实）**：

- `contextforge search` 可用
- REST `/v1/search` 可用（请求/响应契约对齐 PRD §Technical Approach REST/MCP 最小接口契约草案）
- `contextforge export --format jsonl` 可导出 canonical JSONL
- `contextforge export --format markdown-bundle` 可导出 Markdown bundle
- export 前执行二次 secret scan
- 迁移字段保真率可通过 fixture 计算（目标 ≥ 80%）

**端到端 smoke**（task-6.3 AC5 落点；自动化运行留 task-8.1 eval-harness）：

```bash
set -euo pipefail

CF_ROOT="$(mktemp -d)"
CF_SRC="$(mktemp -d)"
CF_PORT="${CF_PORT:-18086}"

cat > "$CF_SRC/README.md" <<'EOF'
# Phase 6 smoke

fixture query phase6exportmarker
EOF

contextforge init --root "$CF_ROOT"
contextforge import "$CF_SRC" --collection default --data-dir "$CF_ROOT"
contextforge serve --data-dir "$CF_ROOT" --addr "127.0.0.1:${CF_PORT}" &
SERVE_PID="$!"
trap 'kill "$SERVE_PID" 2>/dev/null || true' EXIT
sleep 2

contextforge search "phase6exportmarker" --collections default --data-dir "$CF_ROOT" --json \
  | tee "$CF_ROOT/search-cli.json"

curl -fsS \
  -H "Authorization: Bearer $(cat "$CF_ROOT/token")" \
  -H "Content-Type: application/json" \
  -X POST "http://127.0.0.1:${CF_PORT}/v1/search" \
  -d '{"query":"phase6exportmarker","collections":["default"],"top_k":5,"explain":true}' \
  | tee "$CF_ROOT/search-rest.json"

contextforge export --collection default --data-dir "$CF_ROOT" \
  --format jsonl --output "$CF_ROOT/export.jsonl" \
  | tee "$CF_ROOT/export-jsonl.log"
contextforge export --collection default --data-dir "$CF_ROOT" \
  --format markdown-bundle --output "$CF_ROOT/export.tar.gz" \
  | tee "$CF_ROOT/export-md-bundle.log"
contextforge export --collection default --data-dir "$CF_ROOT" \
  --format agent-draft --output "$CF_ROOT/export-draft" \
  | tee "$CF_ROOT/export-agent-draft.log"

test -s "$CF_ROOT/export.jsonl"
test -s "$CF_ROOT/export.tar.gz"
test -s "$CF_ROOT/export-draft/MEMORY.md"
grep -E 'fidelity=0\.[89]|fidelity=1\.000' "$CF_ROOT/export-jsonl.log"
grep -E 'fidelity=0\.[89]|fidelity=1\.000' "$CF_ROOT/export-md-bundle.log"
grep -E 'fidelity=0\.[6-9]|fidelity=1\.000' "$CF_ROOT/export-agent-draft.log"
```

## 7. 阶段级风险

- 关联 PRD §Technical Risks **R9**（本地 daemon/MCP 暴露面风险）：daemon 默认只监听 127.0.0.1 或 Unix socket、禁默认 0.0.0.0；REST 本地随机 token（0600）。关联 PRD §Open Questions O10。
- 关联 **R4**（export 二次扫描漏检）：export 前二次 secret scan；结果默认不展示完整 secret。

## 8. Phase Definition of Done

- [x] 本 phase 全部 task spec Status=Done 或 Waived —— task-6.1/6.2/6.3 全 Done @ PR #41/#44/#43；adapter §Task 索引同步
- [x] §6 阶段级 AC 全部满足、端到端 smoke 已填实（PR #45 phase-6 closeout 填实 §6 命令骨架；自动化运行留 task-8.1 eval-harness — 与 phase-7 同模式）
- [x] 关联风险 R9 / R4 缓解措施已落地（task-6.2 默认 127.0.0.1 监听 + secret-token mode 0600；task-6.3 export 二次 sanity secret scan + 保护路径拒写）
- [x] adapter §Phase 状态索引该行 Status 同步更新 —— PR #45 phase-6 closeout
- [x] team §4 Gate 3 phase smoke gate 通过后方可 merge 最后一个 task —— PR #44 task-6.2 phase-last Gate 3 通过 + PR #45 closeout 后 master Done
