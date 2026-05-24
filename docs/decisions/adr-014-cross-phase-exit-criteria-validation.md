# ADR `014`: `cross-phase-exit-criteria-validation`

**Status**: Proposed
**Category**: Governance / 治理流程
**Date**: 2026-05-24
**Decided By**: tajiaoyezi objective + main agent execution
**Related**: ADR-011 (single-driver-with-subagents) / ADR-012 (main-agent-governance-autonomy) / ADR-013 (cli-data-plane-grpc-bridge) §Follow-ups / [retrospective phase-9-cross-phase-spec-drift](../retrospectives/phase-9-cross-phase-spec-drift.md) / AGENTS.md §3 §4 / docs/s2v-adapter.md §Workflow Overrides / PRD §Open Questions O12

## Context

[retrospective phase-9-cross-phase-spec-drift](../retrospectives/phase-9-cross-phase-spec-drift.md) §4 总结：v0.1.0 (ce47d17, 2026-05-23) ship 后实测发现 CLI 数据通路断 + import 子命令未实现 + task-8.3 fake-evidence 三类 spec drift（详 ADR-013 §Context），跨 Phase 1 / 2 / 6 / 8 击鼓传花链一路无 §2A / merge / closeout gate 截住。retrospective 调研补充 5 处新 phase ↔ task AC mismatch + 50+ anti-pattern 命中 + 8 个 phase closeout PR 全部缺少 cross-validation 证据。

**根因（retrospective §4）**：Phase Definition of Done (§8) 要求 "§6 阶段级 AC 全部满足"，但 "满足" 未被操作化为 "每条 §6 AC cross-reference 到拥有 task §6 AC + evidence 链接"。实际操作链：
1. phase §6 看自身文字是否 `[x]`
2. phase-level smoke 跑过，但 smoke 验证对象（如 Rust API）可能与 §6 字面承诺（如 CLI 能力）脱节
3. 每个 child task §6 AC 自圆其说；task §3 OOS 把未做的部分推给虚构下一 phase
4. 下一 phase task §3 In Scope 不主动接管前 phase OOS 部分（无 owner）
5. 击鼓传花链在某 phase task §3 OOS 处终结，但同 task §6 AC 仍 `[x]`
6. closeout PR 机械 Status flip，不审 phase §6 ↔ task §6 字面对齐

**ADR-012 自治盲区**：ADR-012 §自治范围把 §2A / R6 merge / R7 dep / §8 Waive 交给主 agent；主 agent 在**单 task / 单 phase** 视角内执行 Gate 0-5 充分，但**跨 phase spec drift 检测**在单 task 视角下结构性看不到。本 ADR 不 supersede ADR-012，而是在 phase closeout 维度**叠加**跨 phase 视角的检测制度。

## Decision

引入 **Phase Exit Criteria ↔ Task §6 AC 双向 cross-check 制度**，自下一新 phase（Phase 10）起强制执行。Phase 1-9 历史不溯改（Phase 9 已通过 ADR-013 闭环修复，本 ADR 是其 follow-up）。制度由 5 条约束 + 1 个工具脚本组成：

### D1 — Phase closeout PR 必含 cross-validation 表

phase closeout PR 的 PR body **必须**包含一张 "Phase §6 ↔ Task §6 AC mapping" 表格，每行 4 字段：

| Phase §6 AC 编号 + 字面 | 拥有 task / 验证方式 | task §6 AC 编号 + 字面 | Evidence 链接 |

- 空 mapping 或字段未填 → closeout PR review 自动阻塞
- "phase-level smoke 验证" 是合法的 "拥有 task / 验证方式" 值，但 evidence 必须指向 smoke 脚本退出码 + 验证对象的具体命令（如 `scripts/phase_N_smoke.sh` 退出 0 + 真跑 CLI binary，**不接受**仅跑 Rust API 内部测试作为 CLI 能力字面承诺的证据）
- "无对应 task / 仅 phase-level 验证" 必须显式声明，且 phase §6 字面不得包含 "CLI" / "user-facing endpoint" / "外部可调用" 等用户能力词

### D2 — 击鼓传花条款 lint（`scripts/spec_drift_lint.sh`）

新建 `scripts/spec_drift_lint.sh`：grep `docs/specs/` 中 anti-pattern 关键词，输出每个命中的 `file:line` + 强制要求 spec 作者就近标注：

**A 类标注 — 合法延后**：必须含 `[SPEC-DEFER:<name>]` 命名 marker + 命名 target phase/task；closeout 时可被 lint 验真（grep target phase 是否真接管）。例：
```markdown
- MCP HTTP/SSE transport [SPEC-DEFER:task-7.1.transport-http]：留 future task-7.x 实施
```

**B 类标注 — 静默延后**：必须显式 `[SPEC-OWNER:<task>]` 指向已存在或同 spec PR 中新建的 task。例：
```markdown
- 完整 import REST endpoint [SPEC-OWNER:task-9.4]：本 task 仅 stub
```

无标注的命中 → lint exit 1，closeout PR / spec PR 阻塞。识别词表（可扩）：
- `留给 Phase` / `留 Phase X+` / `推给 task-` / `Phase X+1`
- `本 task 仅` / `仅 scope` / `out of scope` / `OOS`
- `历史 gap` / `历史 drift` / `历史问题`
- `留 future` / `留 v0.X+` / `v0.X.x` / `future task`
- `not implemented` / `unimplemented` / `stub`（spec text 中，code 不算）
- `占位` / `scaffold` / `mock`（spec text 中）

历史 spec 含违规命中不报错（保留向后兼容）；新增 / 修改 spec PR 中触及行强制满足 lint。

### D3 — Phase §6 验证对象一致性 gate

phase spec §6 每条 Exit Criteria 必须在 spec 内显式声明 verification owner，使用以下两种形式之一：

```markdown
- [x] AC N：`<字面承诺>` — verified by phase-smoke `scripts/phase_N_smoke.sh` step M (cmd: `<具体命令>`)
- [x] AC N：`<字面承诺>` — verified by task-<X.Y> §6 AC M (file:line)
```

不允许 `[x]` 无 verification owner。已 Done 的 Phase 1-8 历史 spec 不溯改；Phase 10 起强制。

### D4 — 主 agent 自治补丁（ADR-012 §自治范围叠加约束）

ADR-012 §2A / R6 merge / §8 Waive 的主 agent 自治范围**在 phase closeout PR 维度**叠加以下约束：

- closeout PR 必须在 commit message 或 PR body 含 D1 mapping 表 + `scripts/spec_drift_lint.sh` 输出（截图或粘贴）
- 缺 D1 / D2 输出 → 视为 §2A 未满足，主 agent 不得自决合 PR；必须降级到用户审或升级到 §8 STOP
- §2A Ready review 对 spec PR：新建 / 修改的 spec 文件触及 §3 OOS / §6 AC 段时，主 agent 必须 surface lint 输出 + 标注分类（A / B）；分类不明确 → 同样降级用户审

### D5 — 历史 spec 不溯改 + 适用范围

- **不适用**：Phase 1-9 已 closeout 的 phase / task spec 不重审（包括其 §6 AC 和 §3 OOS 标注）。Phase 9 已通过 ADR-013 单独修复 v0.1 实现层 drift
- **适用**：Phase 10 起所有新建 phase spec + task spec 适用 D1 / D2 / D3 / D4
- **特例**：v0.2.0 后若新增对 Phase 1-9 范围的修补 task（如 task-6.2 GetChunk RPC future 兑现），新 task spec 自身适用 D2 / D3；其所属 phase 不重新 closeout

## Rationale

- **不 supersede ADR-012**：单驱动 + 主 agent 自治本身不是 spec drift 根因（Phase 1-8 大部分实施期是 team worker 多 agent 拓扑）。本 ADR 是在 ADR-012 之上**叠加跨 phase 视角的检测制度**，主 agent 单 task 自治权利不变
- **D1 表是最低成本可执行的强制制度**：retrospective §3.3 显示 8 个 phase closeout PR 全部缺 cross-validation 证据；主 agent 写 mapping 表的边际成本远低于 v0.1 ship 后实测发现的修复成本（Phase 9 全程 6 task + 1 ADR + 1 phase spec + 1 closeout PR）
- **D2 lint 用现存 anti-pattern 词表足够**：retrospective §3.2 已实测 50+ 命中可被 grep 捕获，词表来自 ADR-013 §Context + Explore subagent 调研，覆盖率充分。`[SPEC-DEFER]` / `[SPEC-OWNER]` 标注是写 spec 时的 1 行附加成本
- **D3 verification owner 显式化**：retrospective §4 根因是 "satisfy" 未被操作化。verified by 强制声明把 "smoke 跑过" 与 "字面承诺" 的对齐推到 spec 撰写期（前移），不是 closeout 时再回溯
- **D4 不削弱 ADR-012**：主 agent 在单 task 视角自治权不变；仅 phase closeout 维度加约束，且约束以工具产出（lint 输出 / mapping 表）为客观证据，不依赖主 agent 主观判断
- **D5 历史不溯改避免 churn**：v0.1 → v0.2 历史 spec 即便有违规命中（如 task-1.4 / 2.4 / 6.2 多处），重写收益低（已通过 Phase 9 实施层修复 + 本 retrospective 留痕）；强制溯改会增加 spec drift（修 spec 又跟实施不同步）

## Alternatives

- **选项 A — 不立 ADR-014，仅在 AGENTS.md §4 closeout 章节加 checklist 说明**（弃）：retrospective §3.3 显示 closeout PR 全部缺 cross-validation 已是事实，说明非强制 checklist 实际不被执行。不立 ADR + 不上 lint 工具的方案预期复用 Phase 1-9 模式，Phase 10+ drift 风险无降低
- **选项 B — 引入 cross-validation 但用 reviewer subagent 手动审，不上 lint 脚本**（弃）：retrospective §3.3 Example PR #45 显示 reviewer 偶发反应式发现 spec 不对称，但未制度化；纯人工 review 不可回归。lint 脚本可在 CI 跑 + 本地预跑，是更可靠的物理保险
- **选项 C（本 ADR 选定）— ADR-014 强制 D1 + D2 + D3 + D4，Phase 10 起适用**：D2 lint 是物理 gate；D1 mapping 表是 closeout 物理 gate；D3 是 spec 撰写期前移；D4 把约束嵌入 ADR-012 自治范围，不依赖主 agent 主观判断
- **选项 D — 同上但同时溯改 Phase 1-9 已 Done spec**（弃）：churn 大、收益低（已通过 Phase 9 实施层修复 + 本 retrospective 留痕），且违反 standard.md §16.2 ADR 不可变性同款的 phase spec 锁定原则

## Consequences

**正面**：
- Phase 10+ 击鼓传花链在 spec 撰写期被 D2 lint 拦截，或在 closeout 期被 D1 mapping 表暴露 — 不再依赖 v0.1 ship 后实测才发现
- ADR-012 主 agent 自治范围保持，但 closeout 自治受 D4 客观证据约束 — 不削弱日常工作流，仅在跨 phase 维度加保险
- `scripts/spec_drift_lint.sh` 是可执行工具，CI 可集成（PR check）；本地可预跑（spec 作者 self-validate）
- D5 历史不溯改 → 落地成本仅在 Phase 10 实施期吸收，不需要回填工作

**负面 / 成本**：
- 写 D1 mapping 表对 phase closeout 增加 30-60 min 工作（取决于 phase task 数）— 与 Phase 9 修复成本相比小
- `scripts/spec_drift_lint.sh` 需新建 + 维护词表 + 集成 CI（如已有 CI 框架）— 一次性 1-2 task 工作
- D2 标注（`[SPEC-DEFER]` / `[SPEC-OWNER]`）改变 spec 写作风格，可能引发短期 spec PR review 摩擦（直到团队习惯）
- D4 加约束意味着主 agent 在 closeout 维度更频繁可能降级用户审；预期不影响日常 task 实施 / merge 节奏

**对 ContextForge-Console 对接的影响**：
- 本 ADR-014 是 Phase 10 console-contract-v1 phase 启动前的治理前置 — D1 / D2 / D3 / D4 在 Phase 10 全程适用，预期降低跨 phase 漂移风险（Console Contract v1 涉及 Workspace / IndexJob / REST endpoint 多模块，是 D2 lint 高价值场景）

## Rollback Or Migration Plan

如 ADR-014 实施中发现：

1. **D2 lint 词表误报率过高**（合法用法被识别为 anti-pattern）：补充词表白名单 / 优化匹配规则（仅 grep `docs/specs/` 不含已有 `[SPEC-DEFER]` / `[SPEC-OWNER]` 标注的行）；继续保留 D2 但降低噪音
2. **D1 mapping 表撰写成本超预期**（phase 内 task 多导致表过长）：保留 D1 但简化字段（只要求 verified by 链接，不强制完整字面引用）
3. **D4 主 agent 降级用户审频率过高**（影响 Phase 10 实施节奏）：审视具体 case 调整 D4 阈值，但 D1 / D2 / D3 物理 gate 不变
4. **整体不可行**：发起新 ADR 取代本 ADR；不允许直接修改本 ADR `Decision` 字段（standard.md §16.2 ADR 不可变性）

Rollback 通过新 ADR superseding 完成。

## Follow-ups

- **本 ADR 合入后**：Status: Proposed → Accepted 在本 PR closeout / merge commit 内回填
- **Phase 10 启动前**：实现 `scripts/spec_drift_lint.sh`（可独立 chore PR 或 Phase 10 task-10.0 prelude）+ 在 AGENTS.md §3 / §4 加 cross-validation gate 引用 + adapter §Workflow Overrides 添加（如适用）
- **Phase 10 closeout 后**：首次 D1 / D2 / D3 / D4 全套落地的 phase；retrospective 评估制度有效性，决定是否需要 ADR-014 v2 调整
- **可选 follow-up**：建立 `docs/retrospectives/` 制度化产出（每 phase closeout 后产 mini-retrospective），不在本 ADR scope
- **关联 PRD §Open Questions O12**：本 ADR Accepted 后可 mark O12 resolved by ADR-014（PRD update 在 closeout PR 内或独立 chore PR）
