# Playbook: Autonomous v0.1 Ship via `/goal`

> **不入 git** — 本文件保留为本地 untracked，仅作个人操作 playbook。如要团队复用，移到 `docs/decisions/` 或单独 ADR 化。

## 用途

一条 `/goal` 命令启动主 agent 自治跑完：

1. 扩写规范到 single-driver-autonomous 变体（新增 ADR-012）
2. 自合 refactor PR（ADR-011 + ADR-012 + 全部规范改动）
3. 跑完 Phase 8（task-8.1 / 8.2 / 8.3 + closeout）
4. v0.1.0 release（构建 + tag + push）

**本质**：这条 /goal 命令是 **user override 当前 ADR-011 红线 #1**（condition 禁含「merged」）。启动它等于命令主 agent 先去写 ADR-012 把这条红线删掉，然后按 ADR-012 跑。

---

## 启动前 checklist

仓库**任一**符合的基线即可启动（命令本身会自检 + 决定从哪个 E* 起跑）：

| 基线 | 状态特征 | /goal 从哪起 |
|---|---|---|
| **A. refactor 已 merge** | master 含 ADR-011 三段 commit + ADR-012 不存在 + working tree clean | 自动切新 chore 分支 → 从 E1 起 |
| **B. refactor 在 feature 分支** | 任意 feature branch 含 ADR-011 commit + ADR-012 不存在 | 在当前分支续 → 从 E1 起 |
| **C. refactor + ADR-012 均已 merge** | master 含 ADR-011 + ADR-012 + working tree clean | 跳过 E1-E5 → 从 E6 起 |
| **D. Phase 8 部分完成** | master 含已完成 task-8.x merge commit | 自动识别已完成 E*，从下一个未完成项起 |

**必须的硬基线**（任一不满足 → /goal STOP 求助）：

- [ ] Claude Code 版本 ≥ v2.1.139（`/goal` 命令可用）
- [ ] git status working tree clean **OR** 仅含规范类未 commit 改动（`AGENTS.md` / `docs/` / `_dispatch/` / `.gitignore` 限定路径）
- [ ] master HEAD 至少含 ADR-011（即 commit `d422bec` 或其后续 merge）— 如果 master 还没有 ADR-011，必须先合本次 refactor PR
- [ ] `/clear` 让 context 干净（**重要** — 长 /goal 跑期间 token 累计快）

**心理预期**：最可能停在 E7/E8 之间（task-8.2 或 8.3 中段撞 token 顶）。撞顶不是失败 — 是 best-effort autonomous 的典型边界。

---

## 启动后注意事项

- ❌ **不要再输任何 prompt**（会清掉 active goal，前功尽弃）
- ❌ 不要切 session（session 关闭 /goal 也清）
- ✅ 静观主 agent surface `◎ /goal active` 指示器跑
- ✅ 撞 STOP 时主 agent 会 surface 完整诊断报告 — 看完决定 resume 路径

---

## 命令（复制下方整段到干净 session）

```text
/goal 自治完成本项目 v0.1 ship。

[阶段 0] 第一轮 surface git diagnostic（status / log -10 / branch / ls docs/decisions/）+ docs/s2v-adapter.md 当前 Phase / Task 索引 + 最新 ADR 编号。基于诊断从下方 E* 决定起跑（已完成项标 skip）。基线异常 → STOP 求助。

[完成条件] surface 以下全到位即完成（命令输出 / 文件片段，不接受自述）：

E1 新写治理升级 ADR（取下一个序号），放宽 §2A / R6 merge / R7 dep / §8 Waive 用户审为主 agent 自决，保留 R3/R6 物理保险 + BLOCKED-branch-mismatch.md
E2 AGENTS.md / docs/s2v-adapter.md / _dispatch/README.md 对齐新 ADR
E3 上述 refactor commits 已合 master
E4 docs/s2v-adapter.md Task 总索引中所有非 Done 业务 task 全部闭环：spec Status=Done + §10 6 项齐 + §9 真绿 + master 含 merge commit
E5 所有非 Done phase 全部 closeout：phase spec Status=Done + §8 DoD 全 [x] + adapter Phase 行 Status=Done
E6 v0.1.0 release：git ls-remote --tags origin 含 v0.1.0 + RELEASE_NOTES.md 进 master + 构建产物清单（按 ADR-007）

[自决规则]
1. branch mismatch → reflog + cherry-pick 复原
2. context < 30k → /compact + 重 load 当前阶段关键 spec
3. rate limit → 60s 退避，5 次后 Agent tool spawn subagent 续跑
4. secret 泄露 → 自动 redact + 写 docs/security/INCIDENT-*.md
5. trade-off 无锚点 → 保守优先（backward compat > spec 字面 > 最小改动），§10 注明
6. tag push → 自审 release notes + 产物完整性后自动 push

[硬 STOP] 阶段 0 基线异常 / token < 10% 且 /compact 用尽 / merge rebase 真冲突 / §9 verify 3 轮 systematic-debugging 仍红 / git 状态完全错乱

[硬约束] 每阶段 §9 真跑（surface 命令 + 退出码）/ R3 commit 落分支 grep 全程 / §4 Gate 0-5 物理流程不跳 / 每 commit 调 TaskUpdate 同步 / STOP 必 surface 完整诊断 + 续跑建议

stop after 100 turns（surface 完整进度）
```

---

## 撞 STOP 后的 resume 指南

resume 时**重新输入完全相同的 /goal 命令** — 阶段 0 状态识别会自动判出新起点（已 merge 的 E* 标 skip，从下一个未完成项继续）。

| STOP 原因 | resume 策略 |
|---|---|
| token < 10% / context 爆 | 开新 session → `/clear` → 重新粘贴 /goal 命令 |
| rate limit 持续 | 等 1h 后同上 resume |
| 阶段 0 落入案例 E（基线不对）| 看主 agent surface 的诊断 + 补救建议；人工修复基线（如把 refactor PR 合 master）后 resume |
| git 状态错乱 | **不要再 /goal** — 先用 `git reflog` / `git status` 人工评估，必要时 hard reset 到已知良好 commit，再决定续跑路径 |
| merge rebase 失败 | 人工解冲突 → commit → 再 /goal resume |
| §9 verify 持续红 | 人工审 task spec — 是 spec 写错、实现真漏功能、还是真要 Waive？拍板后 resume |

---

## 与 ADR-011 红线的关系（重要）

| ADR-011 红线 | 本 /goal 命令的处理 |
|---|---|
| condition 禁含「merged」 | ❌ **本命令 condition 含 E5/E6/E7/E8 的 "merged" 字面** — 是 user override（你已决定接受 ADR-012） |
| /goal 不用于 PR merge 本身 | ❌ **本命令让主 agent 自决合 PR** — 同上 user override |
| §2A Draft→Ready 用户审 | ❌ **本命令让主 agent 自决** — ADR-012 放宽 |
| R7 dep 用户审 | ❌ **本命令让主 agent 自决** — ADR-012 放宽 |
| §8 Waive 用户审 | ✅ **保留 STOP**（自决规则 #7） — 实施失败自己 Waive 蒙混风险高 |
| R3 commit 落分支硬 grep | ✅ 保留（物理层） |
| R6 PR 物理流程 | ✅ 保留（执行硬约束 c） |
| BLOCKED-branch-mismatch.md 文件载体 | ✅ 保留（R3/R6 双保险失效仍需用户决策） |

---

## 预测（基于自决规则全到位）

| 阶段 | 撞 STOP 概率 | 主要原因 |
|---|---|---|
| E1–E5 改规范 + 合 refactor PR | 5% | 单纯文档 + git，自决规则覆盖 |
| E6 task-8.1 | 35% | §9 verify 红 3 轮 / rebase 失败 |
| E7 task-8.2 | 50% | 累计 token / 同 E6 |
| E8 task-8.3 | 70% | token cap |
| E9 phase-8 closeout | 75% | token 几乎肯定爆 |
| E10 v0.1.0 release | 80% | 同 E9 + tag 不可逆动作即使主 agent 不犹豫还是要审 release notes |

**最可能停在 E7/E8 之间**。即使全自决到位，**100 turn 内跑完整个 v0.1 release 的概率 < 15%**。但跑到 E5/E6 的概率 > 70% — 至少能把规范升级 + refactor PR 自合 + task-8.1 推进一截。

---

## 不可逆动作清单（启动前再确认一次）

| 动作 | 不可逆程度 | 主 agent 自决吗 |
|---|---|---|
| 在 master 上 commit | 中（可 revert）| 不会（R6 物理保留） |
| merge --no-ff 到 master | 中（可 revert）| **是**（ADR-012 放开）|
| push origin master | 高（公开历史）| **是**（ADR-012 放开）|
| git tag v0.1.0 + push | **极高**（GitHub release tag 删了仍有镜像/缓存）| **是**（自决规则 #8）|
| 自动 redact secret | 中（可看 INCIDENT.md 复盘）| **是**（自决规则 #4）|
| Waive AC | 中（spec 可改回）| 否（保留 STOP）|

如要把任一动作改回保守（STOP 求助），改 /goal command 的对应规则后再启动。

---

## 历史关联

- ADR-011 single-driver-with-subagents 是本 playbook 的治理基线
- 本 playbook 的 /goal 命令会触发主 agent 自动写 ADR-012 single-driver-autonomous（在 ADR-011 基础上进一步放宽）
- 启动失败 / 取消后，仓库状态仍是 ADR-011 stable baseline，不损坏既有治理
