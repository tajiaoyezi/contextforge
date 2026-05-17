# AGENTS.md — 多 Agent 协作约定（Tier: team）

> 本项目 Collaboration Tier = `team`。**任何"非单人"协作场景都用本档**：内部小团队、闭源 SaaS、公开发布、外部贡献的开源项目均适用。
> git 协作严格：worktree 隔离 + feature branch + PR + R6 PR-only。
> 项目可在 `docs/s2v-adapter.md` §Workflow Overrides 微调单一字段（如关闭 multi-reviewer / require-CI-gate）以适配实际团队规模。
>
> **任何 agent 进入本仓库时第一件事：读完本文件 + `docs/s2v-adapter.md`**。
>
> 命名约定：**分支**一律含模块 name 全名（`feat/<phase>-<name>` / `feat/task-<X.Y>-<name>`，与 git 历史一致避免歧义）。**worktree 目录**：phase 级 `ContextForge-wt-<phase-name>/`（主 agent 手动建）；task 级 `ContextForge-wt-task-<X.Y>/`（`/s2v-implement` 步 4.B 自动建 — 用 X.Y 唯一标识，name 已在分支名，二者由 X.Y 关联）。

---

## 章节速查

> 本文件渲染为项目 `AGENTS.md`（数百行长文件）；任何 agent 入仓先按此索引定位。仅列章名不列行号（编辑会漂移 —— 行号锚点仅 `full-standard.md` §0 因 grep 查阅需要而保留）。

- **必守清单** — 所有 tier 必守的 S2V 核心
- **S2V 命令派发** — `/s2v-*` → 执行对应子流程（任何 agent 通用）
- **§0** Adapter 命令读取规则（所有 verification 走 helper）
- **§1** Worktree 拓扑
- **§2** 铁律 R1–R7（违反致代码丢失 / 污染主线）
- **§2.5** Commit 节律
- **§3** Phase / Task 工作流
- **§4** PR 合入流程（PR-only + phase smoke gate） ／ **§4.1** Rebase 同步通知协议
- **§5** 异常处理
- **§6** 给具体外部 agent 的提示
- **§7** 拓扑健康检查
- **§8** 卡住与豁免协议
- **降级到 solo** ／ **参考（项目内自包含）**

---

## 必守清单（所有 tier 必守的 S2V 核心）

S2V 核心方法论（SDD / BDD / TDD Iron Law / §2.5 三段 commit / ADR / Verification / §7 追踪表 / §8 卡住协议）任何 tier 必守。详见 `docs/s2v/standard.md` §4.5.2（项目内快照，由 `/s2v-init` 时复制）。

> `solo` 档放宽的只有 git 协作；本档（team）在 git 协作上是完整严格版。

---

## S2V 命令派发（任何进入本项目的 agent 必读）

本项目已初始化 S2V。当用户输入 `/s2v-implement` / `/s2v-add` / `/s2v-tier`
（或用自然语言要求该子流程）：即便你的工具未注册此 `/` 命令、它只是一条普通消息 ——
你已加载本 `AGENTS.md`，据此执行（命令后的文本即其参数）：

- `/s2v-implement <task-spec 路径>` → 严格执行本文件 **§3 Phase / Task 工作流**
  （项目自包含，无需解析 skill 目录）
- `/s2v-add <类型> <名称>` / `/s2v-tier <目标档>` → 解析 s2v skill 目录
  （`$S2V_SKILL_DIR` 或 `full-standard.md` §22.2 已知默认路径）→ 读并严格执行
  `<skill>/add.md` ｜ `<skill>/tier.md`
- `/s2v-prd` / `/s2v-init` 为建项 / 重建项命令，通常不在已初始化项目内运行；确需时走 skill 目录对应文档

Aider 等无 slash 命令系统的工具：以 `--read AGENTS.md` 加载本文件后本规则同样适用。

---

## 0. Adapter 命令读取规则（所有 verification 命令必须走此 helper）

下面 §3/§4 多处提到的 `s2v_run install` / `s2v_run unit-test` 等都依赖以下 helper。
任何 agent（含外部 Codex/Cursor/CI 脚本）跑 baseline / verification 前，**先 source 一次本块**。

> ⚠️ **必须在 bash 下 source**。helper 顶部有 shell guard：非 bash（zsh 等，macOS Catalina+ 默认 shell 即 zsh）下直接 `source` 会命中 guard 干净退出（提示改用 bash）。agent / CI 须显式 `bash -c '...'` 包裹本块及后续 helper 调用，不要在 zsh 里直接 source。

```bash
# 项目内自包含 helper（由 /s2v-init 步 5.5 / /s2v-tier 步 2.5 同步刷新）
# ⚠️ 须 bash 执行：bash -c 'source docs/s2v/scripts/lib/preflight.sh; source docs/s2v/scripts/lib/verify.sh; ...'
source docs/s2v/scripts/lib/preflight.sh
source docs/s2v/scripts/lib/verify.sh
```

提供的函数：

| 函数 | 用途 |
|---|---|
| `s2v_load_cmd <字段>` | 取 adapter §Commands 字段值（Install / Lint / Typecheck / Unit Test / Integration tests / E2E tests / Coverage / Build / Runtime smoke）|
| `s2v_run <key> [required]` | 执行 verification key；占位 hard-fail / N/A 跳过 / unit-test 自动 required |
| `s2v_extract_verify_keys <task-spec>` | 从 task spec §9 抽取 key（按固定安全执行序）|
| `s2v_verify_full "<keys>"` | 跑全套；空列表 / 缺 unit-test 自动 hard-fail |
| `s2v_read_status <task-spec>` | 读 task spec 顶部 `**Status**:` 字段（多词如 "In Progress" 不被截断）|
| `s2v_preflight_input <path>` | 输入路径形态校验（绝对路径 / ./ 前缀 / 非 docs/specs/tasks/ 拒绝）|
| `s2v_preflight_ready <task-spec>` | Ready Gate 全套；rc=0 OK / rc=1 Draft（本档 §4.5：STOP，通知主 agent/用户审后改 Ready）/ rc=2 硬 STOP |

完整说明 + self-test：见 `docs/s2v/scripts/README.md`。

> **不要硬编码"4 项验证"**：task spec §9 实际列了哪些字段，`s2v_verify_full` 就跑哪些；adapter 字段为 N/A 自动跳过。
>
> **修改 helper 行为**：改全局 skill 的 `scripts/lib/`（路径见 `docs/s2v/standard.md` §22；Claude Code 默认 `~/.claude/skills/s2v/scripts/lib/`，其他 agent 见 §22），跑 `bash scripts/lib/_self-test.sh` 验证，再用 `/s2v-tier` 把项目内快照刷新到最新。**不要直接编辑 `docs/s2v/scripts/`**（会被覆盖）。

---

## 1. Worktree 拓扑

| # | 目录 | 分支 | 角色 | Owner | 启动门槛 |
|---|---|---|---|---|---|
| 0 | `ContextForge/` (主 repo) | `master` | 协调中心 / merge PR / phase smoke gate | **仅主 agent** | — |
| 1 | `ContextForge-wt-foundation/` | `feat/foundation` | Phase 1 foundation 工作区 | 实施 Phase 1 的 agent | ✅ 立即（无前置）|
| 2 | `ContextForge-wt-index-core/` | `feat/index-core` | Phase 2 index-core 工作区 | 实施 Phase 2 的 agent | ⏳ 等 Phase 1 merge |
| 3 | `ContextForge-wt-agent-importers/` | `feat/agent-importers` | Phase 3 agent-importers 工作区 | 实施 Phase 3 的 agent | ⏳ 等 Phase 1 merge（可与 Phase 2 并行）|
| 4 | `ContextForge-wt-retrieval-explain/` | `feat/retrieval-explain` | Phase 4 retrieval-explain 工作区 | 实施 Phase 4 的 agent | ⏳ 等 Phase 2 merge |
| 5 | `ContextForge-wt-memoryops/` | `feat/memoryops` | Phase 5 memoryops 工作区 | 实施 Phase 5 的 agent | ⏳ 等 Phase 2 + Phase 3 merge（可与 Phase 4 并行）|
| 6 | `ContextForge-wt-cli-api-export/` | `feat/cli-api-export` | Phase 6 cli-api-export 工作区 | 实施 Phase 6 的 agent | ⏳ 等 Phase 4 + Phase 5 merge |
| 7 | `ContextForge-wt-mcp-adapter/` | `feat/mcp-adapter` | Phase 7 mcp-adapter 工作区 | 实施 Phase 7 的 agent | ⏳ 等 Phase 6 merge |
| 8 | `ContextForge-wt-eval-and-reliability/` | `feat/eval-and-reliability` | Phase 8 eval-and-reliability 工作区 | 实施 Phase 8 的 agent | ⏳ 等 Phase 6 + Phase 7 merge |

> ⚠️ **本仓库主干分支 = `master`**（stock `git init` 默认）。本文件下文所有协议命令里出现的 `main` / `origin/main` 均指本仓库主干 `master` —— 按 §2 开头「分支命名约定」note，执行时整体替换为 `master`。

**并发拓扑**：

```text
Phase 1 → {Phase 2 ∥ Phase 3} → {Phase 4 ∥ Phase 5} → Phase 6 → Phase 7 → Phase 8

# Phase 4 仅依赖 Phase 2；Phase 5 依赖 Phase 2 + Phase 3。
# 故 Phase 2 merge 后 Phase 4 可启动；Phase 2 + Phase 3 merge 后 Phase 5 可启动；Phase 4 ∥ Phase 5。
```

**串行锁**（同共享文件的 task 必须串行）：
- 🔒 **Phase 4 ↔ Phase 5**（并行集）：若两者都改 `core/src/indexer/` 或扩展 `proto/contextforge/v1/*.proto` → 必须串行。建议 Phase 4 先冻结 retrieval 读路径与 result schema，Phase 5 再叠加治理写路径（dedup/stale/audit）。
- 🔒 **Phase 2 ↔ Phase 3**（并行集）：仅共享 Phase 1 冻结的 canonical-record / gRPC proto（只读契约，非写锁）；各自分别在 `core/`（Rust）与 `internal/importer/`（Go），无源文件写冲突 → 默认可全程并行，无需串行。
- ⚠️ 跨并行 phase 的 task 若都需改 `proto/`：proto 契约变更走 Phase 1 已定的 proto 兼容规则（仅加字段、不删不改 tag），改 proto 的 task 之间必须串行（proto 是全局单一契约源）。

新增 worktree 命名规则：phase 级 `ContextForge-wt-<phase-name>/` + 分支 `feat/<phase-name>`；task 级 `ContextForge-wt-task-<X.Y>/` + 分支 `feat/task-<X.Y>-<name>`（worktree 目录用 X.Y、分支含 name — 与 `/s2v-implement` 步 4.B 落盘一致）。

---

## 2. 铁律（违反会导致代码丢失或污染主线）

> **⚠️ 分支命名约定（统辖本文全部 git 命令）**：下文所有 `main` 均为**你项目主干分支的占位名**，非字面要求。若主干不是 `main`（stock `git init` 默认 `master`，或团队用 `trunk`/`develop`），请把下文每条 git 命令里的 `main`/`origin/main` 整体替换为你的真实主干名 —— 涉及处包括 R6.1 退化表、R7 依赖 PR、§3 工作流 rebase/fetch、Gate 5「切回主干 + merge」、phase smoke rebase。S2V 不强加分支命名。**主干名判定**：`/s2v-init` 落盘后、创建任何 `feat/*`·`chore/*` 分支前所在分支即主干（彼时 `git branch --show-current` 的输出）；有 remote 时亦可取远端默认分支。

### R1 · 主 repo (main) 上禁止业务 commit

主 repo 是协调中心，仅允许：
- 只读：`git status` / `git log` / `git diff` / `git fetch`
- merge：`git merge --no-ff <feat-branch>`（合 PR）
- worktree 管理：`git worktree add/remove`、`git branch <name>`
- tag / release（v1.0 完成后）

### R2 · 启动其他 agent 前，自己 worktree 必须 clean

`git status` 必须 `nothing to commit, working tree clean`。否则先 `git stash -u` 或 `git commit`。

### R3 · 每次 commit 后立即验证 `[branch]`

```bash
EXPECTED=$(git branch --show-current)
git commit -m "..." | tee /tmp/c.txt
grep -qE "^\[${EXPECTED} " /tmp/c.txt || {
  echo "BRANCH MISMATCH: 期望 [$EXPECTED] 实际 $(grep -oE '^\[[^ ]+' /tmp/c.txt)"
  echo "→ 不要 push, 走 §5 场景 A 安全修复"
  exit 1
}
```

### R4 · 同一 worktree 同时只能有 1 个 agent 写

不同 worktree 可并发，同一 worktree 不可。
若需多 agent 在同 phase，再开新 worktree（不同 task 分支）。

### R5 · agent 不得自创 task spec

实施前必须在 `docs/specs/tasks/task-{phase}.{seq}-{name}.md` 中找到对应 task spec。
- **找到** → 严格按 spec 的 AC / Behavior Contract / Traceability / Verification Plan 执行
- **找不到** → 立刻停下，找主 agent 用 `/s2v-add task <name>` 生成。**禁止自创 task spec**

### R6 · 所有 agent（含主 agent）只能通过 PR 合入 main

❌ **禁止操作**（所有 agent 一律禁止）：
- 在 main 上 `git commit`（含 amend / squash 业务提交）
- 在 main 上 `git rebase` / `cherry-pick` / `reset --hard`
- `git push --force` 到任何已发布分支
- 外部 agent 进入主 repo

✅ **允许操作**（明示放行清单）：

| 类别 | 命令 | 谁可以做 |
|---|---|---|
| 合 PR | `git merge --no-ff <feat-branch>` | 仅主 agent 在主 repo |
| Tag / Release | `git tag` / `git push origin <tag>` | 仅主 agent |
| 创建 branch | `git branch <feat-X>` / `git checkout -b <feat-X>` | 主 agent / 任何 agent 在自己 worktree |
| Worktree 管理 | `git worktree add/remove/list` | 仅主 agent |
| 只读 | `git status` / `git log` / `git diff` / `git show` / `git fetch` | 任何 agent |
| 切换 HEAD（不写）| `git checkout <existing-branch>` | 主 agent 在主 repo / task agent 在 worktree |
| Feature branch 写 | `commit` / `rebase <feat>` / `squash` | 任何 agent，仅在自己分支 |

#### R6.1 · 无 GitHub remote 时的退化协议

无 `origin` remote 时（判定：`git remote -v | grep -q "^origin"` 为假 — 与 `references/r6-pr-protocol.md` 单一事实源同口径；fork 仓库有 `upstream` 无 `origin` 亦走此分支）退化为纯本地 PR 模拟：

| 步骤 | 有 remote | 无 remote |
|---|---|---|
| task agent 同步 main | `git fetch origin && git rebase origin/main` | `git rebase main`（worktree 共享 .git）|
| task agent 完成产出 | `git push -u origin <branch>` + 开 PR | 写 `READY-FOR-MERGE-task-X.Y.md` + commit + 用户口头转达 |
| 主 agent fetch PR branch | `git fetch origin pull/N/head:feat/X` | `git checkout feat/X`（本地分支已存在）|
| 主 agent merge | `git merge --no-ff feat/X` + `git push origin main` | `git merge --no-ff feat/X`（无 push）|
| 通知 worktree rebase | comment + GitHub UI | §4.1 STATUS-MAIN.md + 用户转达 |

#### R6.2 · 生效起点

- R6 / R7 自本 AGENTS.md merge 入 main 之后对所有未来 commit 生效
- main 上 R6 加入前的历史 commit 视为 baseline，不需回溯重做

### R7 · 禁止 agent 自行修改 lockfile（`package.json` / `bun.lock` / `requirements.txt` / `Cargo.lock` / 等）

依赖管理仅由主 agent 在专门 PR 中执行。任何 task agent 想加新依赖：

1. 写 `NEEDS-DEP-task-<X.Y>.md`（包名 / 版本范围 / 用途 / 替代方案考虑）
2. commit 该文件 + 在 PR 标 `[needs-dep]`
3. 主 agent 走完整 R6 PR 流程加依赖：
   ```bash
   git checkout -b chore/dep-<package-name>
   <add-dep-command>          # 改 lockfile
   git commit ...             # 在 chore branch 上 commit
   git checkout main
   git merge --no-ff chore/dep-<package-name>
   ```
4. 通知 task agent rebase（按 §4.1）

> **可 override**：小团队完全互信、无供应链审计需求 → adapter §Workflow Overrides 设 `lockfile-protect: false`（不推荐，仅在主 agent 与所有 task agent 同人时考虑）。

---

## 2.5 Commit 节律

每个 task agent 在 feature branch 上至少产出：

| 阶段 | type | 示例 |
|---|---|---|
| RED | `test` | `test(parser): 加 SCEN-2.1.1 ~ 2.1.9 的 9 个 RED 测试` |
| GREEN | `feat` | `feat(parser): 实现 extractHeadings 通过全部 9 个测试` |
| REFACTOR（如有）| `refactor` | `refactor(parser): 提取 walkTokens helper` |
| 文档回填 | `docs` | `docs(spec): 回填 task-2.1 §10 Completion Notes` |

Scope 取值统一为模块名（`<module-1>` / `<module-2>` / `spec` / `agents` / `adapter` / `adr`）。

每次 commit 后立即跑 R3 grep 校验。

---

## 3. Phase / Task 工作流

```bash
# 0. 验证不在主 repo
test "$(basename "$PWD")" != "ContextForge" || { echo "ERROR: 不要在主 repo"; exit 1; }

# 1. 验证当前在预期 worktree
EXPECTED_BRANCH=$(git branch --show-current)

# 2. 同步 main（按 R6.1 区分有/无 remote）
if git remote -v | grep -q "^origin"; then
  git fetch origin main
  git rebase origin/main || { echo "rebase 冲突 → 写 BLOCKED-rebase.md"; exit 1; }
else
  git rebase main || { echo "rebase 冲突 → 写 BLOCKED-rebase.md"; exit 1; }
fi

# 3. 基线绿（单一实现 = scripts/lib/verify.sh s2v_baseline_green；冷启动判定先于门禁：
#    greenfield（prune 依赖/docs 后无非脚手架文件）自动跳过 install+typecheck+unit-test，
#    否则三者全跑；排除式+安全偏置非白名单，不靠 runner 退出码。<UNIT_TEST_AREAS>：读
#    adapter §Source And Test Areas > Unit test areas bullet list，空格分隔多 pathspec，无外层引号）
s2v_baseline_green "<UNIT_TEST_AREAS>" || { echo "❌ 基线非绿 - 先解决遗留"; exit 1; }

# 4. 读规格（按顺序）
#    a. AGENTS.md
#    b. docs/s2v-adapter.md
#    c. <task-spec-path>
#    d. 该 spec §5.1 Required Reading 列出的所有上游 spec
#    e. 对应 .feature 文件
#    f. 相关 ADR

# 4.5. PREFLIGHT — Ready Gate（不通过禁止进 RED）
#      复用 §0 已 source 的 preflight.sh（与 /s2v-implement 步 2 同一 Ready Gate —
#      含 Status 多词解析 / <TBD-by-user> / §6 AC 非空 / §7 SCEN-TEST 非空 全套检查；
#      手写 inline 会漏 §6/§7 空检查，已统一改用 helper）
TASK_SPEC="<task-spec-path>"   # agent 替换为 docs/specs/tasks/task-X.Y-<name>.md 真实路径
s2v_preflight_ready "$TASK_SPEC"
case $? in
  0) : ;;                        # Ready / In Progress，可进 RED
  1) echo "🛑 STOP: $TASK_SPEC Status=Draft — 通知主 agent / 用户审 §3 Scope / §5 Behavior Contract / §6 AC，把 Status 改成 Ready 再来"
     exit 1 ;;
  *) exit 2 ;;                   # 硬性 STOP（§6 AC 空 / §7 无 SCEN-TEST / 非法 Status / 残留 <TBD-by-user> — 详因已写 stderr）
esac

# 5. RED → GREEN → REFACTOR 三段 commit（按 §2.5）
#    每次 commit 后跑 R3 grep 校验

# 5.5. task done 前跑 §9 Verification → 回填 §10 → 推进 Status
#      §10 schema 见 standard.md §8.3 6 项；推进规则见 §10.5.1 状态机。
#      不依赖 /s2v-implement skill — 外部 agent / 手动实施都必须执行此步。
#      用 §0 helper 自动从 task spec §9 抽取实际列出的 verification key（不要手动维护清单）
VERIFY_KEYS="$(s2v_extract_verify_keys "$TASK_SPEC")"
s2v_verify_full "$VERIFY_KEYS" || exit 1
# C4：覆盖率阈值契约门（声明阈值但 Coverage 命令不自我强制 → STOP）
s2v_coverage_threshold_guard "$TASK_SPEC" || exit 1
#      回填 §10 Completion Notes 6 项中文 schema（agent 自行编辑 task spec）
#      字段名速查（权威以 docs/s2v/standard.md §8.3 为准；本注释仅供 agent 减少跳查）：
#        1. **完成日期**：YYYY-MM-DD
#        2. **改动文件**：- src/<file>（新增/修改）...
#        3. **commit 列表**：- <short-hash> <message>
#        4. **§9 Verification 结果**：按本 task §9 实际列出的 key 逐行展开
#             - install: ✅ / skipped: <reason> / N/A
#             - lint: ✅ / skipped / N/A
#             - typecheck: ✅
#             - unit-test: N passed / 0 failed   <!-- 强制：不允许 skipped -->
#             - integration / e2e / build: ✅ / skipped / N/A
#             - coverage: NN.N% / 阈值 NN%
#             - runtime-smoke: ✅ <evidence: 端口/截图/日志>
#             - manual: ✅ <证据/截图/确认者>
#        5. **剩余风险 / 未做项**：一句话或「无」
#        6. **下游 task 影响**：受影响 task ID 列表或「无」
#      （Waived AC 时另加第 7 项 "Waiver 登记"，按 §12.3 五项填，详见 standard §10）
# 推进 Spec Status: Ready / In Progress → Done（portable perl，BSD/macOS + GNU/Linux 通用）
# ⚠️ 不要用 `sed -i ''`（BSD-only，CI/Linux 容器会失败）
perl -i -pe 's/^\*\*Status\*\*: (Ready|In Progress)$/\*\*Status\*\*: Done/' "$TASK_SPEC"
grep -qE "^\*\*Status\*\*: Done$" "$TASK_SPEC" \
  || { echo "🛑 Status 推进失败 — 检查 $TASK_SPEC 顶部"; exit 1; }
git add "docs/specs/tasks/task-<X.Y>-"*.md   # 双引号让 <X.Y> 不被 bash 当输入重定向
git commit -m "docs(spec): 回填 task-<X.Y> §10 Completion Notes + Status → Done"

# 6. task done → push branch（无 remote 时跳过）
if git remote -v | grep -q "^origin"; then
  git push -u origin "$EXPECTED_BRANCH"
fi

# 7. 通知主 agent
#    有 remote：开 PR（与 references/r6-pr-protocol.md 同款）
#      gh pr create --base main --title "task-<X.Y>: <一句话>" \
#        --body "实现 task-<X.Y>，详见 docs/specs/tasks/task-<X.Y>-*.md"
#      （若 gh 未安装：在 GitHub/GitLab Web 上从 $EXPECTED_BRANCH 发起 PR 到 main）
#    无 remote：写 READY-FOR-MERGE-task-<X.Y>.md + commit + 用户转达
```

---

## 4. PR 合入流程（PR-only，主 agent 跑 phase smoke gate）

合并 PR 由仅主 agent 在主 repo 执行。任何 PR 合入 main 前必须通过以下 5 步 gate：

```bash
cd "/home/tajiaoyezi/CodeWorkSpace/ContextForge"   # agent 替换为主 repo 真实路径

# Gate 0：先回收该 task 的 worktree —— 其 commit 已在 feature branch 上（worktree
#         与主 repo 共享 .git），移除 worktree **不丢任何提交**；不先移除则下面 Gate 1
#         的 `git checkout feat/...` 会撞 `fatal: '...' is already checked out at
#         '../ContextForge-wt-task-<X.Y>'`（步 4.B 建的 worktree 仍占用该分支）。
git worktree remove "../ContextForge-wt-task-<X.Y>" 2>/dev/null \
  || { [ -e "../ContextForge-wt-task-<X.Y>" ] && echo "⚠️ Gate 0: worktree 仍存在但 remove 失败（可能有未提交改动）— 处理后 git worktree remove --force 再重跑 gate" >&2; true; }

# Gate 1：切到 PR branch（有 remote 时先 fetch；无 remote 直接 checkout 本地分支）
if git remote -v | grep -q "^origin"; then
  git fetch origin
fi
git checkout "feat/<task-X.Y-name>"

# Gate 2：跑 task §9 Verification Plan 全套
#   §0 helper 自动从 task §9 抽取 key（不要硬编码清单 — 否则需要人工删减，会漏/误执行）
TASK_SPEC="docs/specs/tasks/task-<X.Y>-<name>.md"
VERIFY_KEYS="$(s2v_extract_verify_keys "$TASK_SPEC")"
s2v_verify_full "$VERIFY_KEYS" || exit 1
# C4：覆盖率阈值契约门（声明阈值但 Coverage 命令不自我强制 → BLOCK）
s2v_coverage_threshold_guard "$TASK_SPEC" || { echo "BLOCKED: 覆盖率阈值声明了但 Coverage 命令未自我强制（C4）"; exit 1; }

# Gate 3：Phase 兜底门禁 + 端到端 smoke（按当前 task 在 phase 内位置判定）
#   - phase 内最后一个 task / 跨 phase 集成 task → 必须过 phase 门禁 + 跑 §6 smoke 且全过
#   - phase 内非最后 task → 跳过此 Gate；merge commit 注明 deferred
#   ⚠️ C1：§6 smoke 前先过 s2v_preflight_phase（§6 仍 <TBD>/空 或 phase Status
#      非法 → BLOCK）。"是否最后 task" **机械判定**自 adapter Task 总索引（C1 复审：
#      旧 ${IS_LAST_TASK_IN_PHASE:-1} 默认会把非最后 task 误当最后→§6 仍 <TBD>→
#      误 BLOCK，与下方判定矩阵冲突）。判定不能 → STOP 交人工，不静默放行也不全 BLOCK。
ADAPTER="docs/s2v-adapter.md"        # agent 替换为实际 adapter（如 docs/s2v-adapter.md）
THIS_TASK="<X.Y>"              # agent 替换为本 task id（如 2.3）
PHASE_NO="${THIS_TASK%%.*}"
# 本 phase 内、非本 task、Status≠Done 的 task 行数（adapter §Task 总索引：| Task | 模块 | Spec | Status | … |）
PENDING="$(awk -F'|' -v p="${PHASE_NO}." -v self="$THIS_TASK" '
  /^\| *[0-9]+\.[0-9]+ *\|/ {
    id=$2; gsub(/[[:space:]]/,"",id);
    st=$5; gsub(/^[[:space:]]+|[[:space:]]+$/,"",st);
    if (index(id,p)==1 && id!=self && st!="Done") c++
  }
  END { print c+0 }
' "$ADAPTER" 2>/dev/null)"
if [ -z "$PENDING" ]; then
  echo "BLOCKED: 无法读 adapter §Task 总索引判定本 task 是否 phase 内最后（$ADAPTER）"
  echo "  按下方判定矩阵人工确认后，显式设 IS_LAST_TASK_IN_PHASE=0|1 + IS_CROSS_PHASE_INTEGRATION=0|1 重跑本 gate"
  exit 1
fi
IS_LAST_TASK_IN_PHASE=0; [ "$PENDING" = "0" ] && IS_LAST_TASK_IN_PHASE=1
IS_CROSS_PHASE_INTEGRATION="${IS_CROSS_PHASE_INTEGRATION:-0}"   # 跨 phase 集成 task 无法从索引推断，agent 按矩阵显式设 1
if [ "$IS_LAST_TASK_IN_PHASE" = "1" ] || [ "$IS_CROSS_PHASE_INTEGRATION" = "1" ]; then
  PHASE_SPEC="docs/specs/phases/phase-${PHASE_NO}-<name>.md"   # agent 替换 <name> 为本 phase 名
  s2v_preflight_phase "$PHASE_SPEC" || { echo "BLOCKED: phase §6 未填实 / Status 非法 — 最后 task 不得合并，先补 phase spec §6（C1 集成兜底）"; exit 1; }
  # 通过后按 PHASE_SPEC §6 列出的端到端 smoke 命令执行，必须全过（失败 → block，不 merge）
fi

# Gate 4：检查 §10 Completion Notes 已按统一 6 项 schema 回填
#         schema 见 standard.md §8.3 / s2v-implement 步 10
#
#         注意：不能用 awk '/^## 10\./,/^## /' — 起始行也匹配结束模式，只输出标题行。
#         必须用状态机：起始行后开始捕获，遇到下一个 ## heading 才停。
NOTES=$(awk '
  /^## 10\. Completion Notes/ { in_section=1; next }
  in_section && /^## /        { in_section=0 }
  in_section                  { print }
' "$TASK_SPEC")

# 第 1 道：6 项 outline 字段全检（与 docs/s2v/standard.md §8.3 / s2v-implement 步 10 一致）
echo "$NOTES" | grep -qE "完成日期.*20[0-9]{2}-[0-9]{2}-[0-9]{2}" || { echo "BLOCKED: §10 缺『完成日期』（YYYY-MM-DD）"; echo "  ↳ 若本 task 是 Waive 未实施：§10 仍须规范化 6 项（完成日期填 Waive 当天）+ 清占位 — 见本文件『Waive 后的留痕要求』/ references/blocked-protocol.md"; exit 1; }
echo "$NOTES" | grep -qE "改动文件"                                  || { echo "BLOCKED: §10 缺『改动文件』"; exit 1; }
echo "$NOTES" | grep -qE "commit 列表"                               || { echo "BLOCKED: §10 缺『commit 列表』"; exit 1; }
echo "$NOTES" | grep -qE "Verification 结果|verification 结果"       || { echo "BLOCKED: §10 缺『§9 Verification 结果』"; exit 1; }
echo "$NOTES" | grep -qE "剩余风险"                                  || { echo "BLOCKED: §10 缺『剩余风险』"; exit 1; }
echo "$NOTES" | grep -qE "下游 task 影响|下游影响"                    || { echo "BLOCKED: §10 缺『下游 task 影响』"; exit 1; }

# 第 1.5 道：§10 Verification 段必须按 §9 实际 key 集合 1:1 记录（防 agent 用别 key 凑数过 count gate）
#         §9 keys 复用 Gate 2 已赋值的 $VERIFY_KEYS（s2v_extract_verify_keys 输出，固定执行序 + 去重）
#         §10 keys 从 VERIF_SECTION 用同一标准 key 集合 grep 抽取
#         missing = §9 - §10 → BLOCK；extra = §10 - §9 → warn 不阻断（允许 §10 写额外说明，但提示 §9 漏写风险）
#
# Guard：若 Gate 2 被改流程绕过 → $VERIFY_KEYS 空 → for 循环不进 → 集合校验静默失效。
#        本 guard 强制要求 Gate 2 已赋值（生产路径上 Gate 1→2→3→4 顺序固定，不会触发；防御性兜底）。
[ -z "$VERIFY_KEYS" ] && { echo "BLOCKED: VERIFY_KEYS 未赋值 — Gate 2 应先跑（s2v_extract_verify_keys + s2v_verify_full）"; exit 1; }

VERIF_SECTION=$(echo "$NOTES" | awk '
  /Verification 结果|verification 结果/ { in_verif=1; next }
  in_verif && /^[*-] \*\*(剩余风险|下游)/ { in_verif=0 }
  # 跳过 markdown 围栏内的内容 — 防 agent 把 verification 结果藏在 ``` 块里绕过 grep（P1 修复）
  in_verif && /^[[:space:]]*```/ { in_fence = !in_fence; next }
  in_verif && !in_fence { print }
')
NOTES_KEYS=$(echo "$VERIF_SECTION" \
  | grep -oE "^[[:space:]]+-[[:space:]]+(install|lint|typecheck|unit-test|integration|e2e|build|coverage|runtime-smoke|manual):" \
  | sed -E 's/^[[:space:]]+-[[:space:]]+([a-z-]+):.*$/\1/' \
  | sort -u)

MISSING=""
for k in $VERIFY_KEYS; do
  echo "$NOTES_KEYS" | grep -qx "$k" || MISSING="$MISSING $k"
done
if [ -n "$MISSING" ]; then
  echo "BLOCKED: §10『§9 Verification 结果』段缺少 §9 列出的 key（按 implement.md 步 10 / standard §10 模板必须 1:1 对应）："
  echo "$MISSING" | tr ' ' '\n' | sed '/^$/d; s/^/  - /'
  echo ""
  echo "  §9 列出（按固定执行序）：$VERIFY_KEYS"
  echo "  §10 实际记录       ：$(echo $NOTES_KEYS | tr '\n' ' ')"
  echo "  示例补法：'- unit-test: 12 passed' / '- coverage: 87.3%' / '- build: ✅'"
  exit 1
fi

EXTRA=""
for k in $NOTES_KEYS; do
  case " $VERIFY_KEYS " in *" $k "*) ;; *) EXTRA="$EXTRA $k" ;; esac
done
if [ -n "$EXTRA" ]; then
  echo "⚠️  §10 记录了 §9 没列的 key（不阻断，但请确认是否 §9 漏写）："
  echo "$EXTRA" | tr ' ' '\n' | sed '/^$/d; s/^/  - /'
fi

# 第 2 道：占位拒绝 — §10 内不允许任何 <XXX> 形式的模板 token（防止"字段在但值仍是占位"过 gate）
#   先过 _s2v_strip_retained（§0 已 source preflight.sh，单一源剥除函数）剥掉
#   §8.3 / §10 模板保留的 <!-- --> 注释 / ^> blockquote：§10 schema 指引 blockquote
#   合法含字面 <TBD-after-impl>，不剥则**正确完工**的 task 也被本门误 BLOCK
#   （DEFECT-P3-C，与 DEFECT-1 同根；禁止改回裸 grep，剥除逻辑须单一源 —— 防两处
#   盲点漂移，见 preflight.sh _S2V_STRIP_PREAMBLE）。
#   regex 解释：匹配 <字母_数字_-> 形式的纯标识符 token，能抓 <source-file-1> / <hash1> / <RISK_OR_NONE> / <DOWNSTREAM_OR_NONE> 等真实未替换占位
#   不会误伤 markdown autolink（如 <https://...>，因为 :// 不在字符集内，整个串匹配不上）
PLACEHOLDERS=$(echo "$NOTES" | _s2v_strip_retained | grep -oE "<[A-Za-z_][A-Za-z0-9_-]*>" | sort -u || true)
if [ -n "$PLACEHOLDERS" ]; then
  echo "BLOCKED: §10 仍含未替换的模板占位（应填真实值再 commit）："
  echo "  ↳ 若本 task 是 Waive 未实施：未实施字段填字面量『无（已 Waive，未实施）』并清占位，见本文件『Waive 后的留痕要求』"
  echo "$PLACEHOLDERS" | sed 's/^/  - /'
  echo ""
  echo "   常见示例 → 应替换为："
  echo "   <source-file-1>      → src/parser.ts"
  echo "   <hash1>              → 真实 git short hash（如 abc1234）"
  echo "   <RISK_OR_NONE>       → 一句话风险描述，或字面量「无」"
  echo "   <DOWNSTREAM_OR_NONE> → 受影响下游 task ID 列表，或字面量「无」"
  echo "   <TBD-after-impl>     → 任何 init 时的占位都必须在完工时替换"
  echo ""
  echo "   §10 中**禁止**使用 <...> 形式的尖括号占位 — 即使在反引号内（grep 是纯文本，"
  echo "   不解析 markdown code span，仍会匹配 BLOCK）。如需描述泛型变量名，"
  echo "   改为 'type parameter T' / 'the param value' / 'foo（泛型）' 等无尖括号写法。"
  exit 1
fi

echo "✅ §10 Completion Notes 已按 6 项 schema 完整回填，且无未替换占位"

# 第 3 道：Status 状态机校验（仅检查顶部 Status 合法性 + Blocked 拦截）
SPEC_STATUS=$(grep -E "^\*\*Status\*\*:" "$TASK_SPEC" | head -1 \
  | sed -E 's/^\*\*Status\*\*:[[:space:]]*//; s/[[:space:]]+$//')
case "$SPEC_STATUS" in
  Done|Waived)
    : # 合法终态；Waiver 检查统一在第 4 道触发
    ;;
  Blocked)
    echo "🛑 STOP: Spec Status=Blocked — 不允许合 PR，先解决 BLOCKED-task-*.md"
    exit 1
    ;;
  *)
    echo "🛑 STOP: Spec Status='$SPEC_STATUS'（合 PR 前应为 Done / Waived）"
    exit 1
    ;;
esac

# 第 3.5 道（13 轮 P2-2 修复）：§7 行级状态门 — 顶部 Done 时所有行只能 Done|Waived；
# 防止"顶部状态推进了 / 行级仍 Not Started/Test Red 等" → 假完成 + 破坏 SDD/BDD/TDD 审计链
if [ "$SPEC_STATUS" = "Done" ]; then
  NON_TERMINAL_ROWS=$(awk '/^## 7\. /,/^## 8\./' "$TASK_SPEC" \
    | grep -E "^\|.*\|[[:space:]]*(Not Started|Draft|Ready|Spec Ready|Scenario Ready|Test Red|In Progress|Verified|Blocked)[[:space:]]*\|?[[:space:]]*$" \
    || true)
  if [ -n "$NON_TERMINAL_ROWS" ]; then
    echo "BLOCKED: 顶部 Status=Done 但 §7 追踪表仍有非终态行（破坏 SDD/BDD/TDD 审计链）："
    echo "$NON_TERMINAL_ROWS" | sed 's/^/  /'
    echo ""
    echo "  规则：顶部 Status=Done 时，§7 所有行 Status 只能是 Done 或 Waived。"
    echo "  实施 agent 应在 implement.md 步 10 把通过的行推进为 Done；Waived 行保留原状态。"
    echo "  Verified 视为中间态（已通过测试但未最终完工），合 PR 前应推进到 Done。"
    exit 1
  fi
fi

# 第 4 道：任意 Waived 必须按 §12.3 五项登记 + 每个 Waived 标识独立 block
# P1-5：严格正则 — 只匹配 Status 列单独一格 = "Waived"（前后 | 包围 + 可选空格），避免备注列含字样误触发
WAIVED_ROWS=$(awk '/^## 7\. /,/^## 8\./' "$TASK_SPEC" | grep -E "^\|.*\|[[:space:]]*Waived[[:space:]]*\|?[[:space:]]*$" || true)
WAIVED_ROW_COUNT=0
[ -n "$WAIVED_ROWS" ] && WAIVED_ROW_COUNT=$(echo "$WAIVED_ROWS" | grep -c .)

# 13 轮 P2-1：每个 Waived 行抽标识 — 优先 AC，缺则退化到 SCEN / TEST（兼容 §7 第一列是自然语言 criterion 的项目）
WAIVED_IDS=""
while IFS= read -r row; do
  [ -z "$row" ] && continue
  id=$(echo "$row" | grep -oE "AC-?[0-9.]+" | head -1)
  [ -z "$id" ] && id=$(echo "$row" | grep -oE "SCEN-?[0-9.]+" | head -1)
  [ -z "$id" ] && id=$(echo "$row" | grep -oE "TEST-?[0-9.]+" | head -1)
  [ -n "$id" ] && WAIVED_IDS="$WAIVED_IDS $id"
done <<< "$WAIVED_ROWS"
WAIVED_IDS=$(echo "$WAIVED_IDS" | tr ' ' '\n' | sort -u | grep -v '^$' | tr '\n' ' ')

if [ "$SPEC_STATUS" = "Waived" ] || [ "$WAIVED_ROW_COUNT" -gt 0 ]; then
  ctx=""
  [ "$SPEC_STATUS" = "Waived" ] && ctx="顶部 Status=Waived"
  if [ "$WAIVED_ROW_COUNT" -gt 0 ]; then
    if [ -n "$ctx" ]; then
      ctx="$ctx + §7 行级 Waived $WAIVED_ROW_COUNT 行"
    else
      ctx="§7 行级 Waived $WAIVED_ROW_COUNT 行"
    fi
  fi

  # 13 轮 P2-1：把 §10 切成 Waiver block — 每个"豁免对象：<ID>"行作为 block 起点，每个 block 独立检查五项
  # 旧版只在整段 §10 grep，"AC1 五项齐全 + AC2 只有豁免对象" 的混合场景能绕过
  # 用行级状态机（不用 RS 多字符 — BSD awk 不支持，跨 awk 兼容）
  WAIVER_BLOCKS=$(echo "$NOTES" | awk '
    function emit_block(   m) {
      m = ""
      if (!index(block, "原因"))     m = m " 原因"
      if (!index(block, "替代验证")) m = m " 替代验证"
      if (!index(block, "补齐条件")) m = m " 补齐条件"
      if (!index(block, "负责人"))   m = m " 负责人"
      if (m == "") print "OK:" id
      else print "INCOMPLETE:" id ":" m
    }
    BEGIN { in_block = 0; block = ""; id = "" }
    # P2-B 修复：用列表项锚点，避免"原因：此处对豁免对象要求过严"等散文中含字面量误触发 phantom block
    /^[[:space:]]*-[[:space:]]*\*?\*?豁免对象[\*]*[：:]/ {
      if (in_block) emit_block()
      in_block = 1
      block = $0 "\n"
      id = "未识别"
      if (match($0, /(AC|SCEN|TEST)-?[0-9.]+/)) {
        id = substr($0, RSTART, RLENGTH)
      }
      next
    }
    in_block && /^[[:space:]]*-[[:space:]]*\*\*(剩余风险|下游)/ {
      emit_block(); in_block = 0; next
    }
    in_block && /^## / {
      emit_block(); in_block = 0; next
    }
    in_block { block = block $0 "\n" }
    END { if (in_block) emit_block() }
  ')

  if [ -z "$WAIVER_BLOCKS" ]; then
    echo "BLOCKED: $ctx，但 §10 缺『Waiver 登记』段（standard §10 必填，每个 Waived 标识一个独立 block）"
    exit 1
  fi

  REGISTERED_IDS=$(echo "$WAIVER_BLOCKS" | sed -E 's/^(OK|INCOMPLETE):([^:]+).*$/\2/' | sort -u | tr '\n' ' ')
  INCOMPLETE_BLOCKS=$(echo "$WAIVER_BLOCKS" | grep "^INCOMPLETE:" || true)

  # 1) 每个 §7 Waived ID 都要在 §10 有对应 block
  if [ -n "$WAIVED_IDS" ]; then
    UNREGISTERED=""
    for id in $WAIVED_IDS; do
      case " $REGISTERED_IDS " in *" $id "*) ;; *) UNREGISTERED="$UNREGISTERED $id" ;; esac
    done
    if [ -n "$UNREGISTERED" ]; then
      echo "BLOCKED: §7 Waived 标识在 §10 没有对应 Waiver block："
      echo "  §7 Waived IDs       : $WAIVED_IDS"
      echo "  §10 已登记 IDs      : $REGISTERED_IDS"
      echo "  缺 block 的 IDs     :$UNREGISTERED"
      echo "  每个 Waived 标识一个独立块『- **Waiver 登记 <ID>**:』+ 豁免对象/原因/替代验证/补齐条件/负责人 五项"
      exit 1
    fi
  fi

  # 2) 每个 §10 Waiver block 都必须五项齐全（即使 §7 没列 ID 也要每个 block 自检）
  if [ -n "$INCOMPLETE_BLOCKS" ]; then
    echo "BLOCKED: §10 Waiver block 五项不齐（standard §12.3 — 每个块独立检查）："
    echo "$INCOMPLETE_BLOCKS" | sed -E 's/^INCOMPLETE:([^:]+):(.*)$/  - \1 缺：\2/'
    exit 1
  fi

  echo "✅ Waiver（$ctx）— $(echo $REGISTERED_IDS | wc -w | tr -d ' ') 个 block 全部五项齐全"
fi

# Gate 5：切回 main + merge PR（--no-ff 保留 PR 边界）
git checkout main
git merge "feat/<task-X.Y-name>" --no-ff -m "merge: <task-X.Y> <一句话>"
# ⚠️ 这是 main 上唯一允许的写操作（R6 例外：merge --no-ff）
```

**Phase smoke gate 判定矩阵**：

| 场景 | Phase smoke 命令 | merge 决定 |
|---|---|---|
| Phase 内最后一个 task | 必须全过 | 通过 → merge；失败 → block |
| Phase 内非最后 task | 跳过 | merge 注明 "phase smoke deferred to <last-task>" |
| 跨 phase 集成 task | 必须全过 | 同最后 task |

merge 后清理 feature branch（worktree 已在 Gate 0 回收，不再重复 remove）：

```bash
git branch -d "feat/<task-X.Y-name>"
git push origin --delete "feat/<task-X.Y-name>"   # 远程也删（如有）
```

---

## 4.1 Rebase 同步通知协议

主 agent 每次 merge 一个 PR 到 main 后，必须显式通知所有仍存活的 worktree：

1. 在主 repo 写 `STATUS-MAIN.md`（gitignore 已加，仅本地协调用）：
   ```markdown
   # Main 状态 / 最近一次 merge
   
   - **Commit**: <merge-commit-hash>
   - **Time**: <ISO 8601>
   - **PR / Branch**: feat/<task-X.Y-name>
   - **影响范围**: 列出新增 / 修改的 src/ 文件 + spec 文件
   - **是否影响其他 worktree**: 是 / 否（如是，列出受影响 worktree）
   ```

2. 在每个受影响的存活 worktree 显式发起 prompt：
   > "main 已更新到 <hash>，请在你的 worktree 内：
   > 1. 暂停当前 commit
   > 2. `git fetch origin && git rebase origin/main`（或 `git rebase main` 无 remote 时）
   > 3. 冲突 → 写 `BLOCKED-rebase.md` 求助；无冲突 → 继续工作"

3. task agent 收到通知后优先处理 rebase。

---

## 5. 异常处理

### 场景 A · commit 落错分支（R6 + R3 双保险后理论上不应发生）

> ⚠️ **铁律**：发现 branch mismatch 后**立即停手**。**禁止**任何 agent 自动跑下面的命令——必须先**备份 + 给用户看清**才能动手。

#### A.1 发现：先停 + 备份（agent 自动）

```bash
# R3 校验失败时（[branch] 与 EXPECTED 不一致）：
EXPECTED="<期望分支>"            # 你本以为在哪个分支
WRONG=$(git branch --show-current)   # 实际落在哪个分支
WRONG_HASH=$(git rev-parse HEAD)     # 错 commit 的 hash

# 1. 立刻 push 错分支到远程做 1 份备份（防止本地 reset 丢 commit）
git push origin "${WRONG}":"backup/${WRONG}-mismatch-$(date +%s)" || \
  echo "⚠️ push 备份失败，请手动 git tag backup-${WRONG_HASH:0:7} 后再继续"

# 2. tag 一份本地备份（不依赖 remote）
git tag "backup/${WRONG}-mismatch-${WRONG_HASH:0:7}" "${WRONG_HASH}"

# 3. 写 BLOCKED-branch-mismatch.md 给主 agent / 用户看
cat > BLOCKED-branch-mismatch.md << EOF
# 分支错位事故

- 期望分支：${EXPECTED}
- 实际落于：${WRONG}
- 错 commit：${WRONG_HASH}
- 备份 tag：backup/${WRONG}-mismatch-${WRONG_HASH:0:7}
- 备份 branch（远程）：backup/${WRONG}-mismatch-...

请 **用户审核后** 再决定如何修复（见下方 A.2 选项）。
EOF
git add BLOCKED-branch-mismatch.md && git commit -m "blocked(branch): ${WRONG_HASH:0:7} 落错到 ${WRONG}，已备份"

echo "🛑 STOP。已备份。等用户决定修复方案后再继续。"
exit 1
```

#### A.2 修复（**仅在用户明确确认后** 才执行）

> 用户读完 `BLOCKED-branch-mismatch.md` 后，从下面三个选项里选一个回复 agent。

**选项 1 — 把 commit 移到正确分支，错分支回退到 origin**（最常见）

```bash
# 前置：用户已读 BLOCKED 并明确说"按选项 1 修"
git checkout "${EXPECTED}"             # 切到期望分支（如 feat/<correct>）
git merge --ff-only "${WRONG}"         # fast-forward 把 commit 拉过来
                                       # 如果这步报"non-fast-forward"，停手再问用户

# 此时已不在 ${WRONG} 分支上，可以安全 branch -f
git branch -f "${WRONG}" origin/"${WRONG}"   # 错分支指针回退到远程状态
                                              # 注意：必须确认 origin/${WRONG} 存在且健康
git push -u origin "${EXPECTED}"
```

**选项 2 — 保留错分支历史，把 commit cherry-pick 到正确分支**

```bash
git checkout "${EXPECTED}"
git cherry-pick "${WRONG_HASH}"
# 错分支保持原样（用户后续自行决定是否 revert / 删除）
git push -u origin "${EXPECTED}"
```

**选项 3 — 用户决定保留错落位置**（罕见，例如错分支其实可接受）

由用户手动操作或明确指令 agent，不在此模板内。

#### 红线（永不触碰）

- 🚫 `git reset --hard` —— 全局 dangerous-ops 禁用
- 🚫 `git push --force` 到 main / 共享分支
- 🚫 删除 tag 或 backup branch（直到用户确认修复成功）
- 🚫 任何"agent 自己判断对就上"的修复路径——必须用户在 BLOCKED 文件后明确回复

### 场景 B · rebase 冲突

1. task agent 写 `BLOCKED-rebase.md`（含冲突文件 + 建议解决方向）
2. commit + push branch
3. 主 agent 在 worktree 内手动解冲突 → 让 task agent 接力

### 场景 C · 新增 worktree（仅主 agent）

```bash
# phase 级（主 agent 手动建）：
git branch "feat/<phase-name>"
git worktree add "../ContextForge-wt-<phase-name>" "feat/<phase-name>"
# task 级 worktree 由 /s2v-implement 步 4.B 自动建（../ContextForge-wt-task-<X.Y> + feat/task-<X.Y>-<name>）— 主 agent 通常无需手动建
```

### 场景 D · 卡住 / AC 持续红 → 见 §8

---

## 6. 给具体外部 agent 的提示

> 通用约束：所有外部 agent **严禁进入主 repo `ContextForge/`**，**严禁直接 push main**，**严禁 merge 自己的 PR**。

### Codex
- 默认读取 AGENTS.md
- 启动后必读：`docs/s2v-adapter.md` + 派工 prompt 指定的 task spec + 该 spec §5.1 Required Reading
- commit 用 §2.5 节律 + scope 约定
- 完成 → push branch（如有 remote）+ 开 PR（命令见 §3 步 7：`gh pr create ...`，gh 未装则 Web 发起）/ 无 remote 写 READY-FOR-MERGE 文件

### OpenCode / Cursor / Aider
- 启动前 cd 到具体 worktree 路径，**严禁 cd 到主 repo**
- 启动后第一件事：跑 §3 step 0-2 的 4 行 bash 校验
- 卡住 → 写 BLOCKED 文件（§8）→ 不要硬猜

### Claude Code（主 agent）
- R6 同样适用：自己也只能通过 PR 合入，禁止在 main 上 commit
- 主 agent 自己写代码 / 改 spec → 也开 chore branch → 自 PR 自 merge
- 主 agent 是 phase smoke gate 的执行者（§4）+ 豁免决策者（§8）

---

## 7. 拓扑健康检查

```bash
git worktree list
# 各 worktree HEAD 可不同（在各自 phase 工作中），但都应 ≥ main 的某个 commit
# main 自身应只有 merge commit（R6.2 生效起点之后）
```

---

## 8. 卡住与豁免协议

### 触发条件

任一 task agent 满足以下**全部**条件即视为"卡住"：

1. 同一 AC 连续失败 ≥3 次
2. 已尝试 systematic-debugging 4 阶段（根因 → 模式 → 假设 → 实施）
3. 已检查上游 task spec / ADR 是否有遗漏的契约信息

### 卡住后的标准动作

1. 不要硬猜 / 塞 mock / 改 spec
2. 写 `BLOCKED-task-<X.Y>.md`：

   ```markdown
   # BLOCKED — task-<X.Y>

   ## 卡住的 AC
   - AC<N>: <原文>

   ## 已尝试方案
   1. <尝试 1> → 失败原因
   2. <尝试 2> → 失败原因
   3. <尝试 3> → 失败原因

   ## 当前假设
   - 我认为根因可能是 X，证据是 Y

   ## 主 agent 决策需求
   - 选项 A: 修改 spec（具体改哪行）
   - 选项 B: 给一个提示让我继续
   - 选项 C: Waive 该 AC（按 s2v §12.3 五项填写）

   ## 当前测试 / 代码状态
   - 红测试在 <test-file>:<line>
   - 实施代码在 <src-file>:<line>
   ```

3. `git add BLOCKED-task-<X.Y>.md && git commit -m "blocked(<scope>): task-<X.Y> AC<N> 求助"`
4. push branch + 在 PR 标 `[BLOCKED]`（或在 PR title 加前缀）
5. 退出 worktree，等主 agent 决策

### 主 agent 决策路径

| 决策 | 主 agent 动作 | task agent 后续 |
|---|---|---|
| 选项 A 改 spec | 主 agent 新开 `chore/spec-fix-task-X.Y` branch → 改 task spec → 自 PR 自 merge → 通知 task agent rebase | rebase 后接力 |
| 选项 B 给提示 | 主 agent 在 PR comment 提示 | 删 BLOCKED 文件 + 接力 |
| 选项 C Waive | 主 agent 改 task spec Status → Waived + 在 §10 Completion Notes 追加 **Waiver 登记**（§12.3 五项展开，模板见 standard.md §10）+ **规范化 §10 其余 6 项**（见下方留痕要求）→ 自 PR 自 merge | 删 BLOCKED 文件 + 跳过该 AC |

### Waive 后的留痕要求（s2v §12.3 + §10 Waiver 登记）

任何 Waive 的 AC 必须在 task spec **§10 Completion Notes 的 "Waiver 登记" 子项**追加（这是 Waiver 在 Task Spec 中的**唯一承载位置**，不要新建 §11 / §12 段或写到"剩余风险"自由文本里）：

```markdown
- **Waiver 登记**：
  - **AC<N> Waived**：
    - 豁免对象：<AC 描述或 SCEN-X.Y.N>
    - 原因：<技术 / 业务 / 时间>
    - 替代验证：<命令 / 手工 checklist>
    - 补齐条件：<何时 / 触发条件>
    - 负责人：<主 agent / 用户 / 关联 ADR / Issue 链接>
```

> Gate 4 检测到 task spec 顶部 Status = Waived 但 §10 缺 "Waiver 登记" 子项 → BLOCK。详见 standard.md §10 / §12.3。
>
> ⚠️ **Waive 还须规范化 §10 其余 6 项**（Gate 4 door 1「6 项字段全检」+ door 2「占位拒绝」**无条件先于** Waiver 专用 door 执行，不因 Status=Waived 豁免）。不可达 AC 被 Waive 的 task 通常**从未实施**，§10 仍是 init/add 占位 → 若只加 Waiver 登记就 commit，Gate 4 会在 door 1（缺『完成日期』）即 BLOCK，永远到不了 Waiver 校验。Waive 时主 agent 必须同时：
> - **完成日期** → 填 Waive 当天日期（YYYY-MM-DD）
> - **改动文件 / commit 列表 / §9 Verification 结果 / 剩余风险 / 下游 task 影响** → 未实施填字面量 `无（已 Waive，未实施）`；部分实施填实际值
> - **清除所有 `<TBD-after-impl>` / `<...>` 占位**（door 2 见占位即 BLOCK）
>
> 这样 §10 同时满足 door 1/2（6 项齐 + 无占位）与 door 4（Waiver 五项齐）→ 文档化 Waive 路径才能"自 PR 自 merge"。

---

## 降级到 solo

如项目从协作回归个人维护（archive / 退回到 spike），跑：

```
/s2v-tier solo
```

会重生本 AGENTS.md 为简化版（保留 S2V 核心提示，删 worktree / PR / 主 agent gate / R7 等协作约束）。

降档不影响 main 上历史 commit（R6.2 baseline 化）。

---

## 参考（项目内自包含）

- S2V 完整规范：`docs/s2v/standard.md`（项目快照，由 `/s2v-init` 时复制）
- Tier 详细差异：`docs/s2v/standard.md` §4.5
- Tier 决策树：`docs/s2v/tier-decision-tree.md`
- 模板归档：`docs/s2v/templates-used/`（init 时实际使用的 adapter/agents 模板快照）

> 这些文件是 `/s2v-init` 时从全局 skill（步 0 `_s2v_skill_dir` resolver 解析路径；Claude Code 默认 `~/.claude/skills/s2v/`，其他 agent 见 `docs/s2v/standard.md` §22）复制的快照，让协作者 / CI / 外部 agent 即使没安装全局 skill 也能读到完整规范。
