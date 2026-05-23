# ADR `011`: `single-driver-with-subagents`

**Status**: Proposed
**Category**: Governance / 多 Agent 协作
**Date**: 2026-05-23
**Decided By**: tajiaoyezi
**Related**: AGENTS.md / docs/s2v-adapter.md §Agent Topology / _dispatch/README.md / S2V tier=team / Claude Code `/goal` v2.1.139+ / Phase 1-7 实施全程

## Context

v0.1 ContextForge 自 `/s2v-init` 起选定 S2V tier=`team`，治理拓扑分三层：

- **主 agent（Claude Code）**：在主 repo `ContextForge/` 协调，跑 §4 Gate 0-5 合 PR，仅本层可在 `master` 上写
- **外部 worker 终端**：6 个固定 worker 名册（claude-work1 / codex / grok / droid / agy / kimi）在独立 worktree 跑 task 实施；用户在 worker 终端与主 agent 终端之间复制 prompt + 回报双向中转
- **内部 review subagent**：主 agent 内部用 Agent tool spawn 子 agent 在 context 内评审 PR（2026-05-22 起替代 reviewer 独立终端模式）

Phase 1–7 全程实施观察到 3 个治理摩擦：

1. **worker 槽位实际利用率低**：6 worker 名册理论支持 6 路并行实施，但实际 v0.1 phase 内 task 大多串行依赖（Phase 1 4 task 串行 / Phase 2 4 task 串行 / Phase 5 3 task 串行 / Phase 6 3 task 串行 / Phase 7 1 task），仅 Phase 3 / Phase 4 ↔ 5 有限并行实际用满 ≥3 worker。6 worker 优先级表 + 备选 / 常驻规则给主 agent 增加调度认知负担，但 v0.1 实际并行峰值未达 6。
2. **双向中转成本累计可观**：每个 worker 派工的 latency = (主 agent 写 prompt → 用户复制粘贴到 worker 终端 → worker 跑 `/s2v-implement` → worker 回报粘贴回主 agent 终端)。Phase 1–7 共 19 个 task + 多次 fix 工单 + spec drift PR，单 task 中转开销保守估计 30 min-1h，全 phase 累计 ≥20h 用户中转工作。其中"用户复制" 步骤无技术价值，仅为弥合主 agent / worker 之间无直接通信通道而存在。
3. **Phase 7 已实证内部 subagent 评审 + 主 agent 自合 PR 可独立完成全 phase**：PR #47 (ADR-010) + PR #48 (task-7.1) + PR #49 (phase-7 closeout) + PR #50 (phase-7 spec drift) 共 4 个 PR 全程主 agent 用 Agent tool spawn review subagent 评审 + 主 agent 跑 §4 Gate 0-5 自合，零外部 worker 终端介入 — 治理层面已无技术障碍剥离外部 worker。

同时 Claude Code v2.1.139（2026-05-12 发布）引入 `/goal` 命令：主 agent 设完成条件后跨多轮自治工作，独立 evaluator 模型（默认 Haiku）判完成；与现有"主 agent + 内部 subagent"拓扑天然契合，让单驱动模式不再受限于主 agent 单 turn 容量。

## Decision

v0.1 起项目治理转入 **单驱动 + 内部 subagent 变体**，保留 S2V tier=`team` 骨架（R6 PR-only + R7 lockfile-protect + worktree 拓扑 + ADR governance 不变），仅剥离外部 worker 终端层：

- **唯一驱动**：主 agent（Claude Code 单 session）在主 repo `ContextForge/` 协调 + 在主 repo 或 worktree 实施
- **subagent 调度**：主 agent 用 Agent tool spawn 内部子 agent 完成需隔离 context / 并行执行 / 角色专精的子任务；按需选 `subagent_type`（claude / Explore / Plan / general-purpose / 项目自定义 agent type）；需 worktree 隔离时用 `isolation: "worktree"` 参数
- **长任务自治**：主 agent 用 Claude Code `/goal <condition>` 让自身跨多轮工作至完成条件满足；evaluator 模型独立判定（与做事的主 agent 解耦）
- **外部 worker 终端整套退役**：claude-work1 / codex / grok / droid / agy / kimi 6 worker 名册 + 派工 prompt 落盘规范 + 用户复制粘贴中转流程 全部移除
- **协议名保留 / 载体迁移**：SPEC-DRIFT / NEEDS-DEP / BLOCKED-rebase / BLOCKED-task 4 个协调协议名作为 subagent return 对象类型保留（语义不变 — 标记需主 agent 决策的事件类别），载体从"committed markdown file + 用户口头转达"迁移到"subagent return 结构化对象 + 主 agent 在 context 内可见"。BLOCKED-branch-mismatch.md 例外保留为文件载体（R3/R6 双保险失效需用户决策，必须有文件留痕等用户审）

**适用范围**：v0.1 Phase 8 起的全部 task 派工 + 所有未来 chore PR + 所有未来 ADR / spec drift PR。Phase 1–7 已完成 task 的 §10 Completion Notes 历史 audit trail 不溯改（保留"worker"字面作为实施历史的真实记录）。

## Rationale

- **实际并行峰值 ≤3，6 worker 名册过度配置**：Phase 1–7 全程并行峰值出现在 Phase 3 (3 importer task) + Phase 4↔5 (4.1/4.2 ↔ 5.1/5.2/5.3 集合)，最多 ≤3 worker 同时活跃；6 worker 优先级 / 备选规则的认知开销与实际并行收益不匹配。Agent tool subagent 在主 agent context 内可并行 spawn（单 message 多 tool use call），并行能力上限取决于主 agent token 预算 / Anthropic API 并发限额，足够覆盖 v0.1 + v0.2 可见的并行峰值。
- **双向中转开销转为零中转**：用户复制粘贴动作消失 → latency 改善幅度按 Phase 1–7 累计估算 ≥20h；主 agent 直接读 subagent return object（结构化），失败模式 / 回报内容标准化（与 worker 终端文本回报的自由格式相比，return object 字段固定可机器校验）。
- **/goal 让主 agent 自身可承担长任务**：之前长任务（如 task-2.2 parser 多轮 fix）必须靠 worker 终端跨 session 推进；/goal + auto mode 可让主 agent 跨多轮自治到完成条件满足，evaluator 独立判完成防自我蒙混。
- **Phase 7 已实证可行**：PR #47 / #48 / #49 / #50 全程主 agent + 内部 subagent 完成，包含 ADR 调研 + 新增模块实施 + closeout + spec drift fix 4 类典型工作，无技术阻塞。
- **保留 R6 / R7 / worktree / ADR 骨架**：单驱动不是 solo 档 — solo 档放弃 PR-only + lockfile-protect + worktree 隔离 + ADR 制度。v0.1 公开仓库 + 多人观察 / 未来外部贡献可能 / 治理审计需求都要求保留 team 档骨架。"单驱动 + 内部 subagent" 是 team 档的实施层变体，不是 tier 降级。

## Alternatives

- **选项 A — 维持 team 档原拓扑（弃）**：保留 6 worker 名册 + 派工 prompt 落盘 + 用户中转。
  - Pros：与 Phase 1–7 实施流程 1:1；外部 contributor onboarding 路径清晰（用 codex / grok 等熟悉的工具直接接入）；并行峰值有上限 buffer。
  - Cons：Phase 1–7 实测并行峰值 ≤3，6 名册过度配置；双向中转累计开销 ≥20h；/goal 难以接入（worker 终端不在主 agent context 内，evaluator 看不到 worker 输出）。
- **选项 B（本 ADR 选定）— 单驱动 + 内部 subagent + /goal**：仅主 agent + Agent tool subagent；外部 worker 退役。
  - Pros：零中转；/goal 长任务自治可用；并行通过 Agent tool 单 message 多 spawn 实现，覆盖 v0.1 并行峰值；治理骨架保留（R6 / R7 / worktree / ADR 不变）；Phase 7 实证可行。
  - Cons：subagent token 计入主 session（单 session 上限是约束，需在主 agent 调度时控）；evaluator 模型 billing（默认 Haiku，相对主 turn 通常可忽略）；外部 contributor onboarding 需重写（不再直接派给 codex / grok 等）— 但 v0.1 未来 12 个月内无外部 contributor 计划，不构成阻塞。
- **选项 C — 转 solo 档**：调用 `/s2v-tier solo` 重生 AGENTS.md 为简化版，删除 worktree / PR / 主 agent gate / R7 等协作约束。
  - Pros：最大简化；零治理负担。
  - Cons：放弃 R6 PR-only（仓库直接在 master commit），治理审计链断；放弃 R7 lockfile-protect（无供应链审计）；放弃 worktree 隔离（并行 task 写互相覆盖风险）；放弃 ADR 制度（决策无溯源）— 对公开仓库 + 未来可能的外部贡献 / 治理审计需求都是 regression。
- **选项 D — 团队-轻量档 / 新 tier 名**：在 S2V standard 新增 `team-lite` / `team-single-driver` 等 tier 值。
  - Pros：在方法论层显式区分。
  - Cons：S2V tier 词汇表（`solo` / `team` / `enterprise`）是上游 standard 单一事实源，本项目不应私自扩展 — 会污染 standard。"team 档单驱动变体" 措辞已能在不改 standard 的前提下表达本变体，无需新 tier。
- **选项 E — 混合：主 agent + 1 外部 worker（claude-work1 仅）**：保留 1 worker 槽位作为重 token 任务 offload 通道。
  - Pros：单 session token 预算上限的兜底；外部 contributor onboarding 有最小路径。
  - Cons：双向中转开销不消除（用户仍要复制粘贴）；/goal 仍难接入；治理规范仍要维护 worker 名册段 — 不彻底，治理收益少于一半。

## Consequences

- **正向**：
  - 单决策链 / 零中转 → Phase 8 起每个 task 实施 latency 改善估算 30 min-1h；
  - `/goal` 长任务自治可用 → task-8.1 eval-harness / task-8.2 reliability 等多轮 fix 任务可设 condition 后自驱跑；
  - subagent return 结构化对象 → 失败模式 / 回报内容标准化，主 agent 不需要 parse worker 自由格式文本；
  - 治理骨架不变 → R6 PR-only / R7 lockfile-protect / worktree 拓扑 / ADR 制度全部保留，未来外部 contributor 可参考 AGENTS.md 与 ADR-011 理解项目治理；
  - `_dispatch/sessions/` 不再产出 → 减少本地 git status 噪音；archive/ 历史保留供复盘。
- **负向 / 成本**：
  - subagent token 计入主 session → 单 session token 上限是新约束（subagent 嵌套 spawn 深度需主 agent 显式控制，避免失控）；
  - `/goal` evaluator 模型 billing → 默认 Haiku，相对主 turn 通常可忽略，但长任务下累计需 watch；
  - 外部 contributor onboarding 路径需要重写 → 12 个月内无外部贡献计划，暂不构成实际阻塞，但若 v0.2+ 走开源化路线需重审；
  - `/goal` condition 必须严格 R6 兼容 → condition 禁含「merged」字面（merge 是主 agent 显式动作，不能交 evaluator 判），需主 agent 在写 condition 时遵守红线；
  - Phase 1–7 已完成 task 的 §10 Completion Notes 保留 "worker" 字面 → 与新规范字面不一致（历史 audit trail 与活规范字面分离），需未来 contributor 理解"§10 是历史 / AGENTS.md 是活规范"。
- **影响面**：
  - 直接：`AGENTS.md`（治理活规范）/ `docs/s2v-adapter.md` §Agent Topology（adapter 层）/ `_dispatch/README.md`（dispatch 规范）/ `_dispatch/reviewer__per-PR.md`（历史说明加 1 行）/ `.gitignore`（删 STATUS-MAIN.md 行）；
  - 间接消费者：未来所有 task 派工流程 / 所有 chore PR / spec drift PR；
  - 不影响：所有 Go / Rust 源码 + 测试 + 业务 spec + 业务 ADR（adr-001–010）+ S2V 方法论基线（`docs/s2v/standard.md` / `docs/s2v/templates-used/`）；
  - 关联 ADR：与 ADR-001 ~ ADR-010 全部正交（业务 / 跨语言架构 ADR，治理变体不动业务）。

## Rollback Or Migration Plan

**触发条件**（任一即可启动回退至 team 档原拓扑）：

1. v0.2+ 开源化路线落定，外部 contributor onboarding 需要明确 worker 接入路径；
2. 主 agent 单 session token 上限频繁触顶（如连续 ≥3 个 task 被 token 上限切断），且 subagent 嵌套 / `/goal` 长任务无法缓解；
3. Anthropic API 并发 / rate limit 调整导致 Agent tool 并行 spawn 受限，影响实际并行峰值 < Phase 1–7 已达到的 ≤3；
4. 用户体感（不是技术指标）— 主 agent 单驱动后用户参与感 / 控制感显著下降，希望恢复 worker 派工的 explicit dispatch 节奏。

**回退动作 4 步**：

1. 主 agent 新开 `chore/governance-rollback-to-team-multi-terminal` branch → 撤销本 ADR Status: Proposed → 改为 Superseded by ADR-XXX（新 ADR 记录回退决策）；
2. 用 git revert 或 cherry-pick `chore/agents-single-driver-refactor` 之前的 commit（merge 前一个 master HEAD，即本 ADR 引入前的状态）— 恢复 AGENTS.md §6 给具体外部 agent 的提示 / `docs/s2v-adapter.md` §Agent Roster (6 worker 名册) / `_dispatch/README.md` worker 派工规范 / `.gitignore` STATUS-MAIN.md 行；
3. 新增 `MEMORY.md` 索引 + memory 文件记录回退原因（feedback type），并把现存 `feedback_governance-single-driver.md` 改 status / 删除；
4. 通知（如适用）外部 contributor + 在主 README / 顶层文档增补回退说明。

**轻量备选方向 — 混合模式（选项 E 升级）**：如完全回退成本高，可走"主 agent + 1 worker 槽位"折中 — 仅恢复 claude-work1 worker 名册项 + 派工 prompt 落盘约定，删除 grok / droid / agy / kimi 名册项；`/goal` 在主 agent 侧继续可用，worker 仅承担需要独立 session token 预算的 offload 任务。该方向需新增 ADR-XXX 记录决策。

scope 评估：本 ADR refactor 涉及 5 个治理文件改 + 1 ADR 新增 + 1 memory 新增，零业务代码 / 零业务 spec / 零业务 ADR 改动；回退路径技术上是 git revert + 1 个新 ADR，单次工时 ≤4h，可控。

## Follow-ups

- **task-8.1 eval-harness 实测 `/goal`**：task-8.1 / 8.2 / 8.3 Draft → Ready → 实施时用 `/goal "<§6 AC 全 [x] + §9 verification 全绿 + §10 6 项 schema 齐>"` 试跑，记录：evaluator 触发频率 / condition 写法实际经验 / 长任务 token 累计 / 卡死边界（如多少 turn 后 condition 仍未满足需主 agent 干预）。结果反馈到 AGENTS.md §3.5 主 agent 自驱段。
- **subagent return 协议 schema 化**：SPEC-DRIFT / NEEDS-DEP / BLOCKED-rebase / BLOCKED-task 4 类 return object 当前是约定字段（`kind` / `task` / `ac` / `evidence` / `proposal` 等），未来若 Agent tool API 提供 structured return 类型可正式 schema 化。本 ADR Decision 段是单一事实源。
- **`_dispatch/sessions/archive/` 归档清理策略**：v0.1 archive 内容当前作为历史复盘 / 派工模板参考保留；v0.2+ 若磁盘 / 仓库噪音成本超过保留价值，可在新 ADR 中决定是否删除。本 ADR 不动 archive。
- **AGENTS.md §6 "给具体外部 agent 的提示" 段落删除的回溯**：Codex / OpenCode / Cursor / Aider / Claude Code 4 子段历史价值在于"展示多 agent 工具如何接入本治理"。若 v0.2+ 外部 contributor 路线深化，相关内容可在新 ADR + 新章节恢复（不必从历史 commit cherry-pick — 届时的工具生态可能已迭代）。
- **关联 PRD §Vision「多 Agent 一致可追溯」**：v0.1 PRD §Vision 描述的"多 Agent"是 ContextForge 服务的下游 — 即 Claude Desktop / Cursor / Zed / OpenClaw / Hermes 等消费 ContextForge context 的 agent；本 ADR 涉及的"多 Agent"是 ContextForge 项目内的开发治理 — 是上游。两者正交，本 ADR 不影响 PRD §Vision。
- **`/loop` vs `/goal` 选择启发**：长任务两种自治机制 — `/loop` 按时间间隔 / `/goal` 按完成条件。AGENTS.md §3.5 默认推荐 `/goal`（明确完成态优于时间触发）；`/loop` 适用于无明确完成态的轮询任务（如 watch CI / 监控外部状态变化），暂无强用例时不在 §3.5 推广。
