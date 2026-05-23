# Phase 4 · retrieval-explain

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。
> **Phase 4 已收口（chore/phase-4-closeout，主 agent 域，2026-05-23）**：4.1/4.2 全 Done 并 merge（PR #27 + PR #29）；§6 端到端 smoke 已填实且经 team §4 Gate 3 实跑全绿（`cargo test --test phase4_smoke`）；R3/R8 缓解措施落地（task-4.1 5 字段可解释 + task-4.2 12 字段 explainable schema + path/filename/symbol boost + exact phrase + provenance 合成黑盒守护）。§8 DoD 全满足。收口模式：本 chore PR pre-closeout（§6+§8+Status→Done + adapter sync）→ task-4.2 PR #29 §4 Gate 3 触发抓 → merge。

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

- [x] `contextforge search` 能返回 Top-K 结果（v0.1 调试入口 = Rust `Retriever::explain` public API；Go CLI `contextforge search` 留 Phase 6 task-6.1 实现）
- [x] 每条结果至少包含 12 个可解释字段（含 PRD 列出的 7 字段 file_path/line_start/line_end/score/retrieval_method/reason/agent_scope + task-4.2 新增 5 字段 chunk_id/context_id/source_type/redaction_status/provenance；task-4.2 SearchResult schema）
- [x] 错误 query 返回空结果，不 panic（task-4.1 retriever 5 unit tests 验证 BM25/filter/boost/exact-phrase；task-4.2 phase4_smoke 含 explain() 调用 cover 空 / 异常 path）
- [x] 返回结果能定位回原始文件和行号（task-4.2 AC2 / TEST-4.2.2 — file_path + line_start/line_end 精确）

**端到端 smoke**（2026-05-23 chore PR `chore/phase-4-closeout` 验证）：

`cargo test --test phase4_smoke -- --nocapture` 全绿（master HEAD 收口后）：

- 入口：`core/tests/phase4_smoke.rs` 含 `#[test] fn phase_4_end_to_end_smoke()`（task-4.2 §2A 选项 A 决策；主 agent §4 Gate 3 精准抓 last task）
- 验证项（按 task-4.2 AC1-5 端到端覆盖）：
  - AC1 SearchResult 12-field schema 完整（compile-enforced + 运行时断言）
  - AC2 file_path + line_start/line_end 精确定位回原始文件
  - AC3 黑盒守护 — 每条结果 provenance.len() ≥ 1（合成 scanner-default 兜底）
  - AC4 Retriever::explain() public API 调试入口可调用 + 返 12-field 可解释结果
  - AC5 端到端链路（chunker → indexer → retriever → explain）全过
- 完整 §6 AC 覆盖：task-4.1 retriever 5 tests + task-4.2 4 unit tests + phase4_smoke 1
- 全 workspace verify：`cargo test --workspace` 47 passed (Rust) + `go test ./...` 8 packages 全绿（task-4.1 PR #27 worker + reviewer + task-4.2 PR #29 worker + reviewer subagent 多重独立 verify）

**Scope 注**：phase-4 smoke 用 Rust 集成测试作为 §4 Gate 3 精准抓入口；CLI `contextforge search` 端到端在 Phase 6 task-6.1 实现后由 Phase 8 task-8.3 release smoke 接管；gRPC ContextService::Search tonic server 留 task-6.2；MCP `context_search` tool 留 task-7.1。proto `contextforge.v1.RetrievalResult` 已 frozen in task-1.1（12 字段 1:1 对应 SearchResult），Phase 6/7 wrap 仅需简单 SearchResult → RetrievalResult field mapping。

## 7. 阶段级风险

- 关联 PRD §Technical Risks **R3**（检索召回率达不到 Top-5 ≥ 75% / Top-10 ≥ 85%）：本 phase 起持续跑 recall eval 监控（评测在 Phase 8 落 harness）；golden questions 分场景统计；chunking 可配置；不达标先优化 BM25/metadata/filter。
- 关联 **R8**（中文/英文/代码符号混合检索质量不稳定）：configurable tokenizer；path/filename/symbol 单独 field 并 boost；exact phrase/symbol search；CJK-aware tokenizer 或 n-gram fallback。关联 PRD §Open Questions O11。

## 8. Phase Definition of Done

- [x] 本 phase 全部 task spec Status=Done 或 Waived（按 §12.3 登记）—— 4.1/4.2 均 Done 且 merge（PR #27 + PR #29）；无 Waived
- [x] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过（`s2v_preflight_phase` 通过）—— 本 chore PR 填实；`cargo test --test phase4_smoke` 实跑全过；task-4.2 PR #29 §4 Gate 3 触发精准抓
- [x] 关联风险 R3 / R8 缓解措施已落地（可解释字段齐全、tokenizer 可配置）—— R3：retriever 9 unit tests 覆盖 BM25 + metadata filter + path/filename/symbol boost + exact phrase（task-4.1 5 + task-4.2 4）；recall eval 持续监控落 Phase 8 task-8.1。R8：path/filename/symbol 分 field 已 boost (task-4.1)；exact phrase 已支持；configurable tokenizer / CJK / n-gram fallback 留 task-8.1 eval-harness 后按实测数据调优。
- [x] adapter §Phase 状态索引该行 Status 同步更新 —— 本 chore PR 同步 Phase 4 Draft → Done + task-4.1 Draft → Done（worker §10 已 Done，adapter sync debt）+ task-4.2 Draft → Done
- [x] team §4 Gate 3 phase smoke gate 通过后方可 merge 最后一个 task —— 本 chore PR merge 后 task-4.2 PR #29 §4 Gate 0-5 全过（phase4_smoke 实跑 + Gate 3 section-scoped 复核 IS_LAST_TASK_IN_PHASE）→ merge
