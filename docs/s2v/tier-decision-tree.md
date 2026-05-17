> 📌 **快照来源**：本文件由 `/s2v-init` 在 2026-05-17 从全局 skill `/home/tajiaoyezi/.claude/skills/s2v` 复制。
>
> **请勿直接编辑此文件** — 升级 S2V 规范请改全局 skill 后重跑 `/s2v-init`（或手动 `cp` 覆盖）。

---

# Collaboration Tier 选择决策树

> 帮助用户为新项目选择 S2V Collaboration Tier。被 `/s2v-init` 在初始化阶段调用。
>
> **核心原则**：tier 决定 git 协作严格度，**不**影响 S2V 核心方法论（SDD / BDD / TDD / §2.5 三段 commit / ADR / Verification / 追踪表 / 卡住协议 — 所有 tier 必守）。
>
> **只有两档**：`solo` / `team`。如需关闭 team 内的单字段（如 multi-reviewer / require-CI-gate），用 adapter §Workflow Overrides 微调。

---

## 决策树（1 个核心问题）

### Q：这个项目会有第 2 个开发者 / 外部 agent 协作，或最终公开发布吗？

```
否（仅你自己 / 仅一个固定 agent / 永远不公开）
  → tier = solo

是（任一情况）：
  - 第 2 个人参与开发
  - 调用任何外部 agent（Codex / Cursor / Aider / OpenCode / Kimi）
  - 推到公开仓库（GitHub public / GitLab public）
  - 准备发布 npm / PyPI / cargo / 公开 binary
  - 接受外部贡献者 PR
  → tier = team
```

### 为什么这一个问题就够

- "几个人开发" / "是否公开" / "是否有 CI" 这三件事高度相关，只要**任一个**升级就需要完整的 git 协作约束（worktree / PR / R7 lockfile / phase smoke gate / rebase 通知）
- 实测中没有"中间档"场景能稳定收益 — 要么单人放飞要么完整协作，中间档反而两边不靠
- 如果 team 内某个具体字段（如 multi-reviewer / require-CI-gate / lockfile-protect）不需要，用 adapter §Workflow Overrides 微调即可

### 最终选定

把决策结果写入 `<adapter-path>` 的 `## Workflow` 段：

```markdown
## Workflow

- **Collaboration Tier**: <tier>   # solo | team
```

并把对应 AGENTS.md 模板（`agents-<tier>.md`）渲染到项目根。

---

## 参考案例（按 tier 分类）

### solo 典型项目

- 个人 dotfiles 仓库
- 临时 spike / PoC 验证
- 一次性数据处理脚本
- 个人博客 / 静态站点
- 内部学习项目（不公开 / 单人 / 不调用外部 agent）

### team 典型项目

- 内部业务系统 / 闭源 SaaS（≥2 人或调用外部 agent）
- 公司内部 SDK / 工具库
- 已发布到 GitHub 私有仓库的项目
- npm / PyPI / crates.io 公开发布的库
- 公开 CLI 工具（如本规范的范例项目 `mdtoc`）
- 可被外部贡献的 GitHub public repo
- 需要 multi-reviewer 审核的关键基础设施
- 严格 CI gate + semver / changelog 强制的项目

---

## 边界 case 与 Override

某些场景"已升 team 但想关闭某个具体约束"，可在 adapter 里用 `Overrides` 微调：

```markdown
## Workflow

- **Collaboration Tier**: team
  Overrides:
    - PR-only: true              # 默认 true
    - require-CI-gate: false     # 团队没配 CI，临时关掉
    - worktree-required: false   # 团队规模 2 人不需要并发隔离
    - multi-reviewer: false      # 默认就 false，team 模板不强制多人 review
    - lockfile-protect: true     # 默认 true，强制 R7
    - commit-rhythm-strict: true # 默认 true，所有 tier 强制 §2.5 三段 commit
```

可 override 的字段：
- `PR-only`（true / false）
- `require-CI-gate`（true / false）
- `worktree-required`（true / false）
- `multi-reviewer`（true / false）
- `lockfile-protect`（true / false — R7 强度）
- `commit-rhythm-strict`（true / false — 默认 true，**强烈不建议关**）

> ⚠️ 不可 override S2V 核心：SDD / BDD / TDD / §2.5 / ADR / Verification / 追踪表 / 卡住协议（任何 tier 必守）。

---

## 何时升降档

项目演化触发 tier 调整：

| 触发 | 推荐动作 |
|---|---|
| solo 项目加入第 2 个 agent / 协作者 / 准备公开发布 / 准备接外部贡献 | 跑 `/s2v-tier team` |
| team 项目 archive / 退回个人维护 | 可选 `/s2v-tier solo` |

升降档不会影响 main 上的历史 commit（按 R6.2 baseline 化）。`/s2v-tier` 命令会给"升降档影响清单"。

---

## 默认推荐

如果用户犹豫不决：

- **不知道选什么** → 默认 `team`（中庸，既严格但不重；如有不需要的约束用 Overrides 关掉）
- **明确就一个人 + 永远不公开 + 不调外部 agent** → `solo`
- **明确要做"产品" / 公开发布 / 接外部贡献** → `team`

