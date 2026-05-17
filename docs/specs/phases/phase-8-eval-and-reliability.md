# Phase 8 · eval-and-reliability

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。`/s2v-init` 生成，Status=Draft。§6 端到端 smoke 留 `<TBD-by-user>`，本 phase 最后一个 task 完工/合并前必须填实（`s2v_preflight_phase` C1）。

## 1. 阶段目标

`contextforge eval run` 输出 Top-5/Top-10 命中率、延迟、错误召回报告；v0.1 七项技术闭环在 Linux/WSL2 端到端跑通；完成长任务/中断恢复/资源占用/secret redaction/export 的可靠性硬化；产出可安装的 Linux x86_64 release 包并通过 smoke test。来源：PRD §Implementation Phases Phase 8。

## 2. 业务价值

实现 PRD 核心能力 #4（召回评测）并把 v0.1 收口为可交付物。直接支撑主指标（Golden questions 命中率 + 上下文重建时间）、次指标（检索性能 P95）与 PRD §Decisions Log D6（recall eval 作为 PRD 级一等验收门）。这是 v0.1「做对了」的最终判定 phase。

## 3. 涉及模块

- `eval`（Go+Rust）：golden questions 加载、检索调用、命中率/延迟/错误召回统计（`contextforge eval run`）
- 可靠性硬化：长任务/中断恢复/资源占用 + secret redaction/export 回归
- release：Linux x86_64 release 打包 + smoke test + 性能基准
- 文件锚点：`internal/eval/` · `core/src/`（性能/恢复）· `core/tests/` · `cmd/contextforge/` · 全链路集成测试

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 8.1 | eval | `../tasks/task-8.1-eval-harness.md` |
| 8.2 | reliability | `../tasks/task-8.2-reliability.md` |
| 8.3 | release | `../tasks/task-8.3-release-smoke.md` |

## 5. 依赖关系

- **依赖**：Phase 6（cli-api-export）+ Phase 7（mcp-adapter）
- **可并行**：否
- **Phase 内顺序**：8.1 eval-harness ∥ 8.2 reliability → 8.3 release-smoke（dep 8.1/8.2，最终收口 task）

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（参考 — PRD §Implementation Phases Phase 8 Exit Criteria + §Success Metrics Eval Measurement Protocol，用户审定后落实）**：

- `contextforge eval run` 输出 Top-5 / Top-10、latency、miss cases（Strong/Weak/Miss 命中规则见 PRD §Success Metrics）
- golden questions 数据集 ≥ 30 条，每类 ≥ 5 条（配置定位/错误复现/历史决策/日志排查/Agent memory·rule/代码位置）
- Linux / WSL2 release smoke test 通过
- 10 万 chunk 内 BM25 / metadata / filter 检索 P95 < 500ms（不含 embedding/reranker/远程）
- secret redaction / export / audit log 回归测试通过
- 大仓库长任务中断后可恢复或安全重建

**端到端 smoke**：`<TBD-by-user>`（本 phase 最后一个 task=8.3 完工/合并前填实，即 v0.1 七项技术闭环 Linux/WSL2 端到端 release smoke：解包 tarball → init → import → index → search/MCP → export → eval run → 校验 P95 与命中率门的可执行序列）

## 7. 阶段级风险

- 关联 PRD §Technical Risks **R3**（召回率不达标）：本 phase 落 recall eval harness，分场景统计先达标再看总分。关联 PRD §Open Questions O6（golden questions 数据集构建与维护）。
- 关联 **R6**（大仓库性能/资源回归）+ **R2**（向量后端选型 spike，PRD 定 Phase 5-6 期间做，本 phase 前应已有结论）。关联 O2。

## 8. Phase Definition of Done

- [ ] 本 phase 全部 task spec Status=Done 或 Waived
- [ ] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过（v0.1 七项技术闭环跑通）
- [ ] 关联风险 R3 / R6 / R2 缓解措施已落地（eval harness + 性能回归 + 向量后端结论）
- [ ] adapter §Phase 状态索引该行 Status 同步更新
- [ ] team §4 Gate 3 phase smoke gate 通过后方可 merge 最后一个 task（v0.1 收口）
