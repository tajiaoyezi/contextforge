# _dispatch — review subagent template + 历史归档

> 本目录现状（2026-05-23 起 / ADR-011 + ADR-012）：
> - `reviewer__per-PR.md`：review subagent prompt template（**tracked**，主 agent 内部 Agent tool spawn 时引用，仍在用）
> - `sessions/`：**仅本地** + **历史归档**。新流程不再产出此目录内容；存量 `sessions/archive/` 内是 2026-05-23 前 team 多终端模式下的 worker 派工 prompt 留档，供历史复盘
>
> Ignore 机制：`.gitignore` 含 `_dispatch/sessions/`（项目级 — 所有 dev clone 后自动生效；fresh clone `git check-ignore _dispatch/sessions/anything.md` exit 0，而 `_dispatch/README.md` + `_dispatch/reviewer__per-PR.md` 不被忽略）

## 单驱动变体下本目录的作用

[ADR-011](../docs/decisions/adr-011-single-driver-with-subagents.md) 决策后，项目治理转入"主 agent + 内部 subagent"单驱动模式，外部 worker 终端整套退役。[ADR-012](../docs/decisions/adr-012-main-agent-governance-autonomy.md) 进一步把 §2A Ready review、R6 merge decision、R7 dep chore PR、§8 Waive 的有锚点决策交给主 agent 自决。本目录原"worker 派工 prompt 留痕"作用消失，仅保留两类内容：

1. **`reviewer__per-PR.md`** — 主 agent 用 Agent tool spawn review subagent 时引用的 prompt template（review subagent 调用本身不落盘，Agent tool log 已审计；落盘的是模板）
2. **`sessions/archive/`** — 历史归档。Phase 1–7 实施期间的 worker 派工 prompt 留档，仅本地，作为历史复盘 / 派工模板参考保留；不入 git。如确认不需要可直接 `rm -rf` 清理

## Review subagent 使用规范（主 agent 内部，2026-05-22 起延用）

**Reviewer 不再是独立终端**；主 agent 在 context 内用 **Agent tool** spawn 子 agent 完成评审。

**主 agent 操作流程**：

1. 接收 PR ready 信号后（subagent return ready 对象 / 主 agent 自身完成实施），按 PR 复杂度决定 subagent 数量：
   - 简单 PR：1 个 subagent
   - 复杂 PR（多模块 / 多维度）：2–3 个并行 subagent
   - 多 PR 同时评：N 个并行 subagent（一对一）
2. 主 agent 用 Agent tool spawn 子 agent，prompt 内容引用 `_dispatch/reviewer__per-PR.md`（含角色 + 步骤 + 输出格式）+ PR 特定增量（PR# / 预期 head / 特殊核对点）
3. subagent 在主 agent context 内跑：load context → 临时 clone 跑验证 → 写结构化 review object → return 给主 agent
4. **subagent 不发 PR 评论 / 不调 gh API**（与历史 reviewer 终端模式相反）— 评论由主 agent 评判后决定是否发
5. 主 agent 收 review object → 评判 → 自做小修 / 重 spawn subagent 修 / 决定 merge

**省掉的环节**（vs 2026-05-22 之前的 reviewer 终端模式）：

- ❌ 主 agent 写 reviewer 派工 prompt → 用户复制 → reviewer 终端跑 → 用户复制回报 → 主 agent 接收
- ✅ 主 agent → Agent tool subagent → return → 主 agent 评判（in-context，零用户中转）

**修复路径**（单驱动变体下进一步简化）：

主 agent 评判完 review → 如有修复 → 在主 agent context 内直接改 / 或重 spawn 实施 subagent → 再次 spawn review subagent → 决定 merge。**全程 in-context，无用户中转**。

## 现有结构

```
_dispatch/
├── README.md                                 # 本文件（目录说明 + Review subagent 规范）
├── reviewer__per-PR.md                       # review subagent prompt template（tracked，仍在用）
└── sessions/                                 # 仅本地，不入 git
    └── archive/                              # 历史归档（2026-05-23 前 team 多终端模式遗留）
```

## 主 agent 自驱流程（典型 task）

1. 主 agent 读 task spec（含 §5 Required Reading）→ 决定实施方式：
   - **直接实施**：主 agent 在自己 context 内 / 切 feature branch（在主 repo 或自建 worktree）
   - **spawn subagent 实施**：Agent tool spawn `claude` / 项目自定义 agent type，`isolation: "worktree"` 隔离 — subagent 完成后 return ready 对象
   - **长任务自驱**：主 agent 设 `/goal <condition>` 跨多轮自治至完成态
2. 实施完成 → 主 agent spawn 1+ review subagent 评 PR（引用 `reviewer__per-PR.md`）
3. review 过 → 主 agent 跑 AGENTS.md §4 Gate 0-5 并按 ADR-012 自决 merge；review 未过 → 主 agent 决定修复路径继续

## 边界

- 治理骨架不变：R6 PR-only / R7 subagent lockfile-protect / worktree 拓扑 / ADR 制度 / AGENTS §4 Gate 0-5 全保留
- 主 agent 自治：§2A Ready review / R6 merge decision / R7 dep chore PR / §8 Waive 在有 PRD/spec/ADR/用户目标锚点时由主 agent 自决
- subagent 行为约束：不能 cd 主 repo / 不能 push 到 main / 不能 merge 自己的 PR / 不能改 lockfile（R7：return needs-dep 对象）
- 长任务红线：`/goal` condition 禁含「merged」字面（merge 仍是主 agent 显式动作）
- branch mismatch：R3/R6 物理保险不放松，必须保留 `BLOCKED-branch-mismatch.md` 事故载体；仅确定且非破坏性恢复可由主 agent 自决
- 详细决策与历史背景：[ADR-011](../docs/decisions/adr-011-single-driver-with-subagents.md) / [ADR-012](../docs/decisions/adr-012-main-agent-governance-autonomy.md)
