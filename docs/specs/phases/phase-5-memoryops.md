# Phase 5 · memoryops

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。
> **Phase 5 已收口（chore/phase-5-closeout，主 agent 域，2026-05-23）**：5.1/5.2/5.3 全 Done 并 merge（PR #26 + PR #34 + PR #31）；§6 端到端 smoke 已填实且经 team §4 Gate 3 实跑全绿（`cargo test --test phase5_smoke`）；R4/R5 缓解措施落地（task-5.1 content_hash sha256 跨模块统一 + task-5.2 stale 三触发 + 基础冲突检测 (无 LLM 语义) + task-5.3 SQLite audit_log 4 类事件 + 脱敏策略 + scanner override audit helper）。§8 DoD 全满足。收口模式：本 chore PR pre-closeout（§6+§8+Status→Done + adapter Phase 5 / task-5.2 Status sync；task-5.3 codex 已在 PR #31 顺手 sync 自己一行）→ task-5.3 PR #31 §4 Gate 3 触发抓 → merge。

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

- [x] exact duplicate 能被去重（normalized content hash / source hash）（task-5.1 / PR #26 — content_hash sha256 跨模块统一 + provenance 链合并 + memoryops/dedup unit tests）
- [x] provenance 链能合并并保留多个来源（task-5.1 / PR #26 — multi-importer provenance merge with dedup by importer key + ordering preservation）
- [x] redaction 状态能写入 ContextRecord（`redaction_status` 字段）（task-3.1 importer-core §10 BINDING — pending 跨 4 importer 全设；task-2.1 scanner / task-2.4 indexer 实际脱敏路径）
- [x] stale 标记可被设置和检索（task-5.2 / PR #34 — stale 三触发 expires_at / source deleted / source modified via Oracle abstraction + FilterStale pre-filter API）
- [x] import / search / export / redact 事件能写入 audit log（task-5.3 / PR #31 — SQLite audit_log 4 类事件 + 默认脱敏字段策略 + scanner override audit helper；audit log 不记完整 secret/query/export）

**端到端 smoke**（2026-05-23 chore PR `chore/phase-5-closeout` 验证）：

`cargo test --test phase5_smoke -- --nocapture` 全绿（master HEAD 收口后）：

- 入口：`core/tests/phase5_smoke.rs` 含 `#[test] fn phase_5_end_to_end_smoke()`（task-5.3 §2A 选项 A 决策；主 agent §4 Gate 3 精准抓 last task）
- 验证项（按 task-5.3 AC1-5 端到端覆盖）：
  - AC1 SQLite audit_log 表 4 类事件（import / search / export / redact）写入
  - AC2 默认字段（operation/collection/source/result_count/redaction_count/timestamp）— 不记 query content
  - AC3 反指标 negative — db bytes 实读断言不含完整 secret/export content；含 `[REDACTED:<TYPE>]` 标签
  - AC4 scanner override audit helper 可调用（wiring 由调用方负责 — Phase 6 daemon/CLI 接入）
  - AC5 端到端链路（chunker → indexer → retriever → audit + db bytes 断言无 SECRET）全过；stale 检查用测试内 stub（task-5.2 lifecycle 是 Go package，跨语言 seam 推到 Phase 6 daemon gRPC）
- 完整 §6 AC 覆盖：task-5.1 dedup memoryops/dedup unit tests + task-5.2 lifecycle 4 unit tests + task-5.3 audit 4 unit tests + phase5_smoke 1
- 全 workspace verify：`cargo test --workspace` 52 passed (Rust) + `go test ./...` 全 Go 10 包 ok（task-5.1 PR #26 + task-5.2 PR #34 + task-5.3 PR #31 worker 实测 + reviewer subagent 三轮独立 verify）

**Scope 注**：phase-5 smoke 用 Rust 集成测试作为 §4 Gate 3 精准抓入口；Go memoryops/lifecycle stale API 与 Rust core 跨语言 wiring 推到 Phase 6 daemon（task-6.x via gRPC + proto extension）；CLI `contextforge memoryops *` 端到端在 Phase 6 task-6.1 实现后由 Phase 8 task-8.3 release smoke 接管。MemoryOps 能力严格不超 PRD「v0.1 MemoryOps 能力边界」（不做 LLM 语义冲突判断 / 不做完整 event sourcing — task-5.2 AC3 反指标 statically guaranteed via import 闭合）。

## 7. 阶段级风险

- 关联 PRD §Technical Risks **R5**（外部 Agent schema 漂移影响 provenance 合并准确性）：provenance 与 importer 解耦；以 content_hash 为去重锚点。
- 关联 **R4**（redaction 状态正确写入）：redaction_status 必须随 ContextRecord 持久化；audit log 脱敏（不记录完整 secret）。关联 PRD §Open Questions O5（canonical record schema 无损承载边界）/ O9。

## 8. Phase Definition of Done

- [x] 本 phase 全部 task spec Status=Done 或 Waived（按 §12.3 登记）—— 5.1/5.2/5.3 均 Done 且 merge（PR #26 + PR #34 + PR #31）；无 Waived
- [x] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过（`s2v_preflight_phase` 通过）—— 本 chore PR 填实；`cargo test --test phase5_smoke` 实跑全过；task-5.3 PR #31 §4 Gate 3 触发精准抓
- [x] MemoryOps 能力严格不超 PRD「v0.1 MemoryOps 能力边界」（不做 LLM 语义冲突判断 / 不做完整 event sourcing）—— task-5.2 AC3 反指标 statically guaranteed (imports 闭合 4 项 — os/sort/time/contextforgev1，无 LLM client / http / embedding)；task-5.3 audit_log 不做 event sourcing 仅 append-only 4 类事件
- [x] adapter §Phase 状态索引该行 Status 同步更新 —— 本 chore PR 同步 Phase 5 Draft → Done + task-5.2 Draft → Done；task-5.3 已由 codex PR #31 自带 sync（worker 顺手 — 工程接受，建议未来规范化 AGENTS §Workflow 明示）
- [x] team §4 Gate 3 phase smoke gate 通过后方可 merge 最后一个 task —— 本 chore PR merge 后 task-5.3 PR #31 codex fix-rebase + §4 Gate 0-5 全过（phase5_smoke 实跑 + Gate 3 section-scoped 复核 IS_LAST_TASK_IN_PHASE — 受益于本会话 PR #32 Gate 3 fix）→ merge
