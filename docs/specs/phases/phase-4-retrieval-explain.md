# Phase 4 · retrieval-explain

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。`/s2v-init` 生成，Status=Draft。§6 端到端 smoke 留 `<TBD-by-user>`，本 phase 最后一个 task 完工/合并前必须填实（`s2v_preflight_phase` C1）。

## 1. 阶段目标

检索链路跑通；可通过内部 gRPC Search API / `contextforge search` 调试入口返回带 `file_path/line/score/retrieval_method/last_modified/reason/agent_scope` 的可解释结果。来源：PRD §Implementation Phases Phase 4。

## 2. 业务价值

实现 PRD 核心能力 #2（可解释检索，一等公民）—— ContextForge 的核心差异点。直接支撑主指标「Golden questions 命中率 Top-5 ≥ 75% / Top-10 ≥ 85%」与次指标「可解释性覆盖率 ≥ 90%」，以及反指标「不能为命中率牺牲可解释性」。

## 3. 涉及模块

- `retriever`（Rust）：BM25 / metadata / filter 检索 + explainable retrieval trace + explainable result schema
- 文件锚点：`core/src/retriever/` · `core/tests/`

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 4.1 | retriever | `../tasks/task-4.1-retriever.md` |
| 4.2 | retriever | `../tasks/task-4.2-explain.md` |

## 5. 依赖关系

- **依赖**：Phase 2（index-core：Tantivy + SQLite 索引产物）
- **可并行**：是 —— 可在 Phase 2 + Phase 3 完成后与 Phase 5（memoryops）并行（Phase 4 走检索读路径，Phase 5 走治理写路径）。串行锁见 AGENTS.md §1（若都改 `core/src/indexer/` 或扩展 proto 须串行）
- **Phase 内顺序**：4.1 retriever（BM25/metadata/filter）先行 → 4.2 explain（trace + result schema，dep 4.1）

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（参考 — PRD §Implementation Phases Phase 4 Exit Criteria，用户审定后落实）**：

- `contextforge search` 能返回 Top-K 结果
- 每条结果至少包含 `file_path`、`line_start`、`line_end`、`score`、`retrieval_method`、`reason`、`agent_scope`（对齐 PRD §Technical Approach REST/MCP search result 契约）
- 错误 query 返回空结果，不 panic
- 返回结果能定位回原始文件和行号

**端到端 smoke**：`<TBD-by-user>`（本 phase 最后一个 task=4.2 完工/合并前填实，例：索引 fixture 后对一组 query 跑 `contextforge search` → 校验每条结果含全部 7 个可解释字段、空 query 不 panic 的 smoke 序列）

## 7. 阶段级风险

- 关联 PRD §Technical Risks **R3**（检索召回率达不到 Top-5 ≥ 75% / Top-10 ≥ 85%）：本 phase 起持续跑 recall eval 监控（评测在 Phase 8 落 harness）；golden questions 分场景统计；chunking 可配置；不达标先优化 BM25/metadata/filter。
- 关联 **R8**（中文/英文/代码符号混合检索质量不稳定）：configurable tokenizer；path/filename/symbol 单独 field 并 boost；exact phrase/symbol search；CJK-aware tokenizer 或 n-gram fallback。关联 PRD §Open Questions O11。

## 8. Phase Definition of Done

- [ ] 本 phase 全部 task spec Status=Done 或 Waived
- [ ] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过
- [ ] 关联风险 R3 / R8 缓解措施已落地（可解释字段齐全、tokenizer 可配置）
- [ ] adapter §Phase 状态索引该行 Status 同步更新
- [ ] team §4 Gate 3 phase smoke gate 通过后方可 merge 最后一个 task
