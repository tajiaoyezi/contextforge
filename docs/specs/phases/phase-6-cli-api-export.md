# Phase 6 · cli-api-export

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。`/s2v-init` 生成，Status=Draft。§6 端到端 smoke 留 `<TBD-by-user>`，本 phase 最后一个 task 完工/合并前必须填实（`s2v_preflight_phase` C1）。

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

**端到端 smoke**：`<TBD-by-user>`（本 phase 最后一个 task 完工/合并前填实，例：索引 fixture → `contextforge search` + `curl /v1/search` 返回一致可解释结果 → `contextforge export` 三格式产出 + 二次 secret scan 命中、字段保真率 ≥ 80% 的 smoke 序列）

## 7. 阶段级风险

- 关联 PRD §Technical Risks **R9**（本地 daemon/MCP 暴露面风险）：daemon 默认只监听 127.0.0.1 或 Unix socket、禁默认 0.0.0.0；REST 本地随机 token（0600）。关联 PRD §Open Questions O10。
- 关联 **R4**（export 二次扫描漏检）：export 前二次 secret scan；结果默认不展示完整 secret。

## 8. Phase Definition of Done

- [ ] 本 phase 全部 task spec Status=Done 或 Waived
- [ ] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过
- [ ] 关联风险 R9 / R4 缓解措施已落地（监听限制 + token + export 二次扫描）
- [ ] adapter §Phase 状态索引该行 Status 同步更新
- [ ] team §4 Gate 3 phase smoke gate 通过后方可 merge 最后一个 task
