# Phase 5 · memoryops

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。`/s2v-init` 生成，Status=Draft。§6 端到端 smoke 留 `<TBD-by-user>`，本 phase 最后一个 task 完工/合并前必须填实（`s2v_preflight_phase` C1）。

## 1. 阶段目标

基于 content hash / source hash 完成重复记录去重并保留 provenance 链；支持 stale 标记；完成基础冲突提示；生成审计事件与 audit log。来源：PRD §Implementation Phases Phase 5。

## 2. 业务价值

实现 PRD 核心能力 #3（MemoryOps 治理）—— 避免 memory 从「增强 Agent」变成「污染 Agent」。支撑次指标「真实接入度 ≥ 20 条 memory/context 治理记录」与反指标「不能为索引速度牺牲 secret redaction」。能力边界严格按 PRD §Core Capabilities「v0.1 MemoryOps 能力边界」。

## 3. 涉及模块

- `memoryops`（Go+Rust）：去重 / 冲突检测 / 过期标记 / provenance 合并 / 审计事件
- 文件锚点：`internal/memoryops/`（Go 编排/审计）· `core/src/memoryops/`（Rust hash/dedup）· `core/tests/`

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 5.1 | memoryops | `../tasks/task-5.1-dedup.md` |
| 5.2 | memoryops | `../tasks/task-5.2-lifecycle.md` |
| 5.3 | memoryops | `../tasks/task-5.3-audit.md` |

## 5. 依赖关系

- **依赖**：Phase 2（索引产物）+ Phase 3（importer 产出的 canonical record）
- **可并行**：是 —— 可在 Phase 2 + Phase 3 完成后与 Phase 4 并行。串行锁见 AGENTS.md §1（Phase 4 ↔ Phase 5 若都改 `core/src/indexer/` 或扩展 proto 须串行；建议 Phase 4 先冻结读路径契约）
- **Phase 内顺序**：5.1 dedup（content/source hash + provenance 合并）先行 → 5.2 lifecycle（stale + 冲突，dep 5.1）∥ 5.3 audit（审计事件 + audit log，dep 5.1）

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（参考 — PRD §Implementation Phases Phase 5 Exit Criteria，用户审定后落实）**：

- exact duplicate 能被去重（normalized content hash / source hash）
- provenance 链能合并并保留多个来源
- redaction 状态能写入 ContextRecord（`redaction_status` 字段）
- stale 标记可被设置和检索（`expires_at` / source deleted / source modified）
- import / search / export / redact 事件能写入 audit log（不记录完整 secret / 完整导出内容）

**端到端 smoke**：`<TBD-by-user>`（本 phase 最后一个 task 完工/合并前填实，例：导入含重复事实的 fixture → 校验去重后 provenance 链合并、stale 可标记可检索、audit.log 含四类事件且无完整 secret 的 smoke 序列）

## 7. 阶段级风险

- 关联 PRD §Technical Risks **R5**（外部 Agent schema 漂移影响 provenance 合并准确性）：provenance 与 importer 解耦；以 content_hash 为去重锚点。
- 关联 **R4**（redaction 状态正确写入）：redaction_status 必须随 ContextRecord 持久化；audit log 脱敏（不记录完整 secret）。关联 PRD §Open Questions O5（canonical record schema 无损承载边界）/ O9。

## 8. Phase Definition of Done

- [ ] 本 phase 全部 task spec Status=Done 或 Waived
- [ ] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过
- [ ] MemoryOps 能力严格不超 PRD「v0.1 MemoryOps 能力边界」（不做 LLM 语义冲突判断 / 不做完整 event sourcing）
- [ ] adapter §Phase 状态索引该行 Status 同步更新
- [ ] team §4 Gate 3 phase smoke gate 通过后方可 merge 最后一个 task
