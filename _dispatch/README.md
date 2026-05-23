# _dispatch — worker 派工 prompt 留痕 + review subagent template

> 顶层 `README.md`（本文件）+ `reviewer__per-PR.md` **tracked 入库**作为规范文档（项目级 source-of-truth）；`sessions/` 子目录 **仅本地**（worker 派工 prompts 动态内容，每个 session 自维护）。
>
> Ignore 机制：`.gitignore` 含 `_dispatch/sessions/`（项目级 — 所有 dev clone 后自动生效；fresh clone `git check-ignore _dispatch/sessions/anything.md` exit 0，而 `_dispatch/README.md` 不被忽略）。
>
> 主 agent 本地可能在 `.git/info/exclude` 也有 `_dispatch/` 旧规则（2026-05-22 之前残留）— 对 tracked 文件无效，可保留可清理；不影响他人 clone。

## 规范：worker 派工 prompt 必须落盘

每次 session 开始后，**所有由主 agent 指派给外部 worker 终端的 prompt 必须以 `.md` 落盘到本目录**。
这样主 agent / 用户 / 后续 session 都能复盘当时派工的全部上下文，避免靠对话流回看。

**Review subagent 调用不落盘**（2026-05-22 起 — 详见下方 §Review subagent 使用规范）：主 agent 内部 Agent tool spawn 行为，Agent tool log 已审计。

**落盘位置约定**（仅 worker 派工）：

| 类型 | 位置 | 例 |
|---|---|---|
| 通用模板（跨 session 复用、不绑定具体 task） | 顶层 `_dispatch/` | `reviewer__per-PR.md`（review subagent prompt template）|
| session-specific 派工（绑定具体 task / 时段） | `sessions/<YYYY-MM-DD>-<topic>/` 子目录 | `sessions/2026-05-22-task-2.4-indexer/01-claude-work1__task-2.4-indexer.md` |

**文件命名**：`<seq>-<worker名>__<task-名>.md`（同 session 多发时用 `01/02/...` 标顺序；单发可省序号）。

### `sessions/` 子目录组织规则（详细）

`_dispatch/sessions/` 仅本地（`.gitignore` 已加），用于留痕当前进行中 session 的全部 worker 派工 prompt。命名规则：

**子目录**：`sessions/<YYYY-MM-DD>[-<可选-topic>]/`
- `<YYYY-MM-DD>` = session 启动当天日期（按主 agent 终端日历）
- `<可选-topic>` = 该子目录开始时主要 task 或主题，便于人类识别（如 `task-5.2-5.3-parallel` / `phase-4-closeout` / `task-2.4-indexer`），可省略只用日期
- **同日 / 同 session 合并约定**（2026-05-23 用户纠偏）：同一日期或同一 session 下的所有 worker 派工（含初始 task dispatch / fix 工单 / chore PR dispatch / 跨 task 后续追加）**统一进入同一文件夹**，用文件名 seq 区分，**禁止**为每个 task / 每个 chore 单建子目录
- **历史兼容**：archive 内已有的 `task-5.2-5.3-parallel` / `task-2.4-indexer` / `task-4.2-explain` 等命名为合法已落档，不 retro 重组；新建子目录按本约定走

**文件名**：`<NN>-<worker>__<task-name>.md`
- `<NN>` = 子目录内**全局**派工序号（`01` / `02` / ...），**跨 task 跨 worker 跨 chore 共用一组序号**（不按 worker 分隔），便于按时间线复盘
- `<worker>` = 实际 worker 名（`claude-work1` / `codex` / `grok` / `droid` / `agy` / `kimi` — 与 §Agent Roster 一致）
- `<task-name>` = 该 prompt 的 task 主题（同子目录的 task-id 可不重复，可填子主题如 `nit-cleanup` / `gate-fix`）
- 命名示例：`01-claude-work1__task-2.4-indexer.md` / `01-codex__task-5.3-audit.md` / `01-droid__chore-cleanup.md`

**多 worker 双轨 / 多轨 session**：所有 worker prompt 落同一子目录（按日期 + 可选 topic 分组），不再每 worker 单建子目录。例：
```
sessions/2026-05-23-task-5.2-5.3-parallel/
├── 01-claude-work1__task-5.2-lifecycle.md
├── 02-codex__task-5.3-audit.md
├── 03-grok__chore-agents-gate-defects.md
├── 04-droid__chore-post-merge-nit-cleanup.md
├── 05-grok__chore-agents-fix-blocker.md
├── 06-codex__task-5.3-fix-rebase.md
├── 07-claude-work1__task-6.1-cli-search.md
├── 08-agy__chore-bdd-phase-1-backfill.md
├── 09-droid__chore-adr-009-provenance-ts.md
└── 10-grok__chore-dispatch-readme-consolidate-by-date.md
```

**注意 seq 跨 task/chore/worker 全局递增**（不按类型分组也不按 worker 分组 — 纯时间线）；该例展示了同一日期下 task 派工 + fix 工单 + chore 派工共 10 个 prompt 全部入同一文件夹。

**fix 工单** 写在同一子目录、序号递增：`02-codex__task-5.3-fix.md` / `03-codex__task-5.3-fix2.md`。**fix 工单 / chore 派工 / 新 task dispatch 全部共用同一组 seq，不分配独立子目录**。

归档约定见上方现有结构段。

## worker 回报输出规范（所有 worker 派工 prompt 强制）

worker（claude-work1 / codex / grok / droid / agy / kimi）回报时**必须明确写出**：

1. **作用对象**：PR 编号（如 `PR #6`）+ 完整 PR 链接（如 `https://github.com/tajiaoyezi/contextforge/pull/6`）+ task ID（如 `task-2.2`）
2. **作用结果**：本次做了什么（commit hash + headline / 触发的状态变化）
3. **作用验证**：§9 真绿 / 测试数字 / 是否引入回归

**理由**：worker 的回报常被原样剪贴转给下游（主 agent、其他 worker）。如果 PR# / 链接只在标题或开头一笔带过，下游拿到截断部分时可能识别不出作用对象，导致跑错 PR / 漏 PR / 无法 cross-verify。

**派工 prompt 写法**：每份发给 worker 的派工 prompt 末尾 "回报清单" 段**必须包含**：
- "新 head SHA + PR # / PR URL"（不能只给 SHA）
- "本次操作的 PR 编号 + 链接"（即使全文已多次提，也要在结尾再明示一次便于剪贴）

**示例回报顶部应有**：
```
✅ PR #6 (https://github.com/tajiaoyezi/contextforge/pull/6) task-2.2 第二轮 FIX-1 后半补齐完成
   新 head SHA: 6d0fe863...
   修复内容: ...
```

而不是把 PR# 只放标题里（标题被剪贴吞掉就丢失了上下文）。

## Review subagent 使用规范（主 agent 内部 Agent tool spawn，2026-05-22 起）

**Reviewer 不再是独立终端**；改为主 agent 在 context 内用 **Agent tool** spawn 子 agent 完成评审。

**主 agent 操作流程**：
1. 接收 worker push PR 回报后，按 PR 复杂度决定 subagent 数量：
   - 简单 PR：1 个 subagent
   - 复杂 PR（多模块 / 多维度）：2-3 个并行 subagent
   - 多 PR 同时评：N 个并行 subagent（一对一）
2. 主 agent 用 Agent tool spawn 子 agent，prompt 内容引用 `_dispatch/reviewer__per-PR.md`（含角色 + 步骤 + 输出格式）+ PR 特定增量（PR# / 预期 head / 特殊核对点）
3. subagent 在主 agent context 内跑：load context → 临时 clone 跑验证 → 写结构化 review object → return 给主 agent
4. **subagent 不发 PR 评论 / 不调 gh API**（与之前 reviewer 终端模式相反）— 评论由主 agent 评判后决定是否发
5. 主 agent 收 review object → 评判 → 派 worker fix（落盘 worker fix prompt）/ 决定 merge

**省掉的环节**（vs 之前 reviewer 终端模式）：
- ❌ 主 agent 写 reviewer 派工 prompt → 用户复制 → reviewer 终端跑 → 用户复制回报 → 主 agent 接收
- ✅ 主 agent → Agent tool subagent → return → 主 agent 评判（in-context，零用户中转）

**worker 修复工单仍按原流程**：主 agent 评判完 review → 写 worker fix prompt → 落盘 `_dispatch/sessions/...` → 用户转给 worker 终端（这一段保持不变 — 修复仍需 worker 在 worktree 实施）。

## 现有结构

```
_dispatch/
├── README.md                                 # 本文件（目录约定 + 落盘规范）
├── reviewer__per-PR.md                       # review subagent prompt template（主 agent spawn 时引用；保留文件名，内容已改为 subagent 模式）
└── sessions/
    ├── <YYYY-MM-DD>-<topic>/                 # 当前进行中的 session（PR 未 merge 前 worker 派工 + fix prompts）
    └── archive/                              # 已完工 task 的派工留痕归档（可删可保留，PR comments / git log 是主审计源）
```

**归档约定**：当某个 session 关联的 PR 全部 merged 后，把该 session 子目录 mv 到 `sessions/archive/`。原 `sessions/<date>-<topic>/` 路径腾出给新 session。archive 内容不影响新派工查找，仅作为历史复盘 / 派工模板参考保留；如确认不再需要，可直接 `rm -rf _dispatch/sessions/archive/<topic>/` 清理（不入 git，无审计风险）。

## 操作步骤（典型派工流）

1. 主 agent 按 `docs/s2v-adapter.md` §派工模板 写 worker prompt → 落盘到对应 session 子目录（本规范）
2. 用户把 prompt 全文粘到对应 worker 终端（claude-work1 / codex / grok / droid / agy / kimi），回车
3. worker 跑 `/s2v-implement` 的 **§2A 交互审核**（在它自身终端用选择题问用户：AC 接受？Owner？§3/§4/§5 取值？R7 依赖怎么走？）
   - **R7 依赖问题**：一律选「**独立 chore-dep PR**」（不要 fold-in），它会写 `NEEDS-DEP-task-X.Y.md`
4. worker 完成后回报 **PR 链接**（或 `NEEDS-DEP-*` / `SPEC-DRIFT-*` / `BLOCKED-*`）
5. 用户把回报**原样贴回主 agent 终端**：
   - `NEEDS-DEP-*` → 主 agent 开 `chore/dep-*` PR 串行加依赖、merge、通知 worker rebase
   - `SPEC-DRIFT-*` → 主 agent 串行处理 proto / 契约（add-only），通知受影响 worker rebase
   - PR 链接 → 见第 6 步
6. **评审 → 合并**（每个 PR — 新流程 2026-05-22 起）：
   - a. 主 agent 直接用 **Agent tool spawn 1+ 个 review subagent**（数量按 PR 复杂度 + 并行需要决定）— 详见上方 §Review subagent 使用规范
   - b. subagent return 结构化 review object 给主 agent（不发 PR 评论 / 不调 gh API）
   - c. 有 Blocker / Major → 主 agent 写"修复工单" prompt 落盘 → 用户转给对应 worker 终端修（同 feat 分支、TDD、§9 真绿）
   - d. 评审过 → **主 agent 跑 AGENTS §4 Gate 0-5 并 merge**。worker / 用户都不要自己 merge

## 边界（重要）

- 并行 task 写路径互不相交（cli / importer = Go 不同包；scanner / parser = Rust 不同模块），可真并行
- **唯一并行雷区 = 依赖文件**（go.mod / Cargo.toml / *.lock）。R7「依赖只走主 agent 独立 chore-dep PR」消解 — 所以 §2A 里依赖一定选独立 PR、不要 fold-in
- 早启动的 Phase 2/3（2.1/2.2/3.1）**禁改 `proto/`**；需要就 STOP 写 SPEC-DRIFT，主 agent 串行处理。依据：adapter §Workflow Overrides `phase23-start-gate=contract-frozen`（PR #4 已签字落档）
- 合并 / §4 Gate / §6 smoke / 依赖串行 / 契约漂移：全部主 agent 把关，worker 终端只在 §2A 答题 + 回报
