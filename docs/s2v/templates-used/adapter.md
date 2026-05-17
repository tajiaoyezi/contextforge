> 📌 **快照来源**：本文件由 `/s2v-init` 在 2026-05-17 从全局 skill `/home/tajiaoyezi/.claude/skills/s2v` 复制。
>
> **请勿直接编辑此文件** — 升级 S2V 规范请改全局 skill 后重跑 `/s2v-init`（或手动 `cp` 覆盖）。

---

# Project Development Adapter

> S2V Development 项目适配层。AI agent 进入项目的第一份必读文件。
> 一旦项目结构、命令、约束发生变化，立即更新本文件（s2v §4.4）。
>
> 本模板由 `/s2v-init` 生成。所有 `<占位符>` 必须被替换为项目真实值。
>
> 与 AGENTS.md 关系（如生成）：本文件定义"项目结构与命令规范"（路径 / 命令 / 测试 / coverage），AGENTS.md 定义"协作行为约束"（worktree / commit / 卡住协议）。两者均必读，加载顺序：AGENTS.md（协作）→ 本文件（结构）→ task spec（业务）。

---

## Project

- **Name**: `<PROJECT_NAME>`
- **Type**: `<PROJECT_TYPE>` <!-- Web / API / CLI / Mobile / Desktop / Data Pipeline / Infrastructure 等 -->
- **Primary users / actors**: `<USER_ROLES>`
- **Critical workflows**: `<CRITICAL_WORKFLOWS>` <!-- 列 2-4 个核心用户流程；与 standard.md §4.3 同 schema -->

---

## Specification Locations

- **SDD home**: `<SPEC_HOME>` <!-- 如 docs/specs/ -->
- **Master spec**: `<MASTER_SPEC>` <!-- 如 docs/prds/<name>.prd.md -->
- **Phase spec pattern**: `<PHASE_SPEC_PATTERN>` <!-- 如 docs/specs/phases/phase-{N}-{name}.md -->
- **Task spec pattern**: `<TASK_SPEC_PATTERN>` <!-- 如 docs/specs/tasks/task-{phase}.{seq}-{name}.md -->
- **BDD acceptance home**: `<ACCEPTANCE_HOME>` <!-- 如 test/features/*.feature -->
- **ADR home**: `<DECISION_HOME>` <!-- 如 docs/decisions/adr-{N}-{title}.md -->

---

## Source And Test Areas

> **路径 list 格式**：所有四类区域使用 **markdown bullet list，每行一个 git pathspec**。下游 `/s2v-implement` 把整个 list 读出后展开为 `git add` 多参数（无需外层引号 / 无需空格分隔）。
>
> **强约束（Source areas / Unit test areas）**：`/s2v-implement` 步 6/7 RED/GREEN 直接当 git pathspec 用 → **禁 `<...>` 占位 + 禁 `N/A`**（占位会触发 `git add` fatal）。
> **弱约束（Integration test areas / E2E test areas）**：当前 `/s2v-implement` / helper **不直接消费** → **允许 `N/A: <原因>`** 或保留 `<...>` 占位（项目无 integration / e2e 测试时合法跳过）；未来引入 integration / e2e 自动化时升级为强约束。

### Source areas

- `<SOURCE_AREAS>` <!-- 如 src/ ；Go 多目录每行一个：cmd/、internal/、pkg/ ；禁 ./...（Go package pattern 非 git pathspec）-->

### Unit test areas

- `<UNIT_TEST_AREAS>` <!-- 如 test/ ；Go 同包测试用 .（项目根全包含）；多目录每行一个，与 Source areas 同款 -->

### Integration test areas

- `<INTEGRATION_TEST_AREAS>` <!-- 弱约束：如 test/integration/ ；无 integration 测试时填 N/A: 无 integration 测试 -->

### E2E test areas

- `<E2E_TEST_AREAS>` <!-- 弱约束：如 test/<scenario>.e2e.test.ts ；无 e2e 测试时填 N/A: 无 e2e 测试 -->

### Other locations

- **BDD feature**: `<ACCEPTANCE_HOME>`（与对应 test 文件同名，仅扩展名 `.feature` vs `.test.<ext>`）
- **Fixture areas**: 见下方 §Fixture 约定

### Test File Naming（Default Profile，可覆盖）

> ⚠️ 这是 `/s2v-init` 生成的**默认建议**（适合 TS / JS / Bun 生态）。S2V 通用规范本身**不强制**测试命名 — 项目可在此文档覆盖此节以匹配语言习惯（如 Go 用 `_test.go`、Python 用 `test_<module>.py`、Rust 用 `mod tests`）。

| 测试类型 | 文件名 | 示例（TS/JS 默认） |
|---|---|---|
| 单元测试 | `<module>.test.<ext>` | `parser.test.ts` 对应 `src/parser.ts` |
| 集成测试 | `<module>.integration.test.<ext>` | `cli.integration.test.ts` |
| 端到端测试 | `<scenario>.e2e.test.<ext>` | `dogfood.e2e.test.ts` |
| BDD feature | `<module>.feature` | `parser.feature` |

**默认建议**：保持团队内命名一致（避免同时混用 `.spec` 和 `.test`、避免大小写漂移、avoid 不必要的子目录嵌套）。

**覆盖示例**（其他语言生态）：
- Go: `<module>_test.go` 同包；E2E 用 `e2e/<scenario>_test.go`
- Python: `test_<module>.py` 用 pytest；E2E 用 `tests/e2e/test_<scenario>.py`
- Rust: 单元放 `#[cfg(test)] mod tests` 同源文件；集成放 `tests/<scenario>.rs`

### Fixture 约定（避免多 agent drift）

| Fixture 大小 / 用途 | 落地位置 | 示例 |
|---|---|---|
| 小 (<20 行) | inline template literal in test file | `const md = \`# Title\`;` |
| 中 (20-100 行) | `test/fixtures/<module>/<case-name>.<ext>` | `test/fixtures/parser/setext.md` |
| 大 (>100 行 / 二进制 / 跨 task 复用) | `test/fixtures/shared/<purpose>.<ext>` | `test/fixtures/shared/golden-readme.md` |

**约束**：
- 含 unicode / 特殊字符的 fixture 一律走文件，禁止 inline（diff 噪音 + 编码风险）
- 跨 task 复用 → 必须放 `test/fixtures/shared/` + 在两个 task spec §3 都引用
- fixture 文件名规则：kebab-case + 描述性（**不**写 `case1.<ext>`）

### TEST-ID 落地约定（Default Profile，可覆盖）

> ⚠️ 这是默认建议，目的是让追踪表锚点能被 grep 精确匹配。具体语法可由项目按测试框架习惯调整，只要**保留可 grep 的稳定 TEST-ID**即可。

task spec §7 追踪表写 `TEST-X.Y.Z` 等编号，对应代码层落地建议：

```text
// SCEN-X.Y.Z / AC<N>
it/test "TEST-X.Y.Z: <描述>"  → 验证 grep "TEST-X.Y.Z" 能精确匹配
```

**默认建议**：
- `it()` / `test()` 描述**以 `TEST-X.Y.Z:` 开头**（冒号 + 空格）
- 描述上方一行写 `// SCEN-X.Y.Z / AC<N>` 注释（标记追踪表锚点）
- TEST-ID 必须能被 grep 精确匹配 → 配合追踪表实现"声明 → 落地 → 跑过"三段验证

**覆盖示例**（其他测试框架）：
- Go: `func TestXYZ(t *testing.T) { t.Run("TEST-X.Y.Z: ...", ...) }`
- Python pytest: `def test_xyz():` + 文档字符串 `"TEST-X.Y.Z: ..."`
- Rust: `#[test] fn test_x_y_z() { /* TEST-X.Y.Z */ ... }`

---

## Commands

> 所有命令在项目根目录或对应 worktree 根运行。
>
> **字段语义**（/s2v-implement 与 AGENTS 模板的 helper 按此判读 — `s2v_load_cmd` 取字段后整行字面量交 `s2v_run` 判读）：
> - 真实命令 → **直接填裸值**（如 `pnpm lint`）；**不要加反引号、不要加行尾 `<!-- -->` 注释** — `s2v_load_cmd` 返回整行字段值，含反引号 / 注释会被 `s2v_run` 的 `eval` 误执行（命令替换乱套）且使占位检测失效
> - 项目暂时不适用 → 写字面量 `N/A: <原因>`，如 `N/A: 待 v1.1 引入 ESLint`
> - **不要留空** — 留空会被 helper 报告为"未配置"并影响 §10 Verification 结果记录可读性
> - 未替换的 `<...>` 占位（**裸形式**，无反引号）→ verify.sh 干净 hard-fail（"❌ adapter §Commands - <field> 仍是未替换占位"，明确指引编辑 adapter）
> - **Unit Test 强制**：§9 Verification 不接受 `N/A` / 留空（unit-test 自动 required；其余字段 N/A 时跳过执行但保留审计痕迹）
> - 字段示例：Install 如 `pnpm install`（无依赖写 `N/A: 无依赖`）/ Typecheck 如 `pnpm tsc --noEmit` / Coverage 如 `pnpm test --coverage`
> - **JS/TS/Node Unit Test 注意**：`node --test` 的目录模式发现行为在 Node 18/20/22 各 major 间不一致（test file glob / 默认递归范围有差异），同一命令换 Node 版本可能跑到不同测试集。**Unit Test 命令应固定一个稳定 runner**（显式列出 test glob，或用 vitest / jest），并把确切 Node 版本记到 §9 Verification / §Constraints `Runtime target` —— `node --test` directory-mode discovery differs across Node 18/20/22, so pin an explicit runner and record the exact Node version.

- **Install**: <INSTALL_COMMANDS>
- **Lint**: <LINT_COMMANDS>
- **Typecheck**: <TYPECHECK_COMMANDS>
- **Unit Test**: <UNIT_TEST_COMMANDS>
- **Integration tests**: <INTEGRATION_TEST_COMMANDS>
- **E2E tests**: <E2E_TEST_COMMANDS>
- **Build**: <BUILD_COMMANDS>
- **Coverage**: <COVERAGE_COMMANDS>
- **Runtime smoke**: <RUNTIME_SMOKE_COMMANDS>

<!-- 字段顺序与 s2v_extract_verify_keys 固定执行序一致：install → lint → typecheck → unit-test → integration → e2e → build → coverage → runtime-smoke → manual。Build 在 Coverage 前（先编译通过再算覆盖率） -->

> ⚠️ **字段名必须加粗**（`- **Field**:` 形式）且大小写敏感 — `s2v_load_cmd` helper 的 awk 正则按 `^- \*\*Field\*\*:` 匹配，去掉加粗或改大小写后 helper 全部读不到，所有 verification 命令被当空值跳过（unit-test 还会 hard-fail）。如需扩展字段，与 implement.md / AGENTS.md §0 helper 同步更新。
>
> **§Commands 不收录 Release / Deploy** — 发布/部署是跨 task 的项目级动作（一般由 CI/CD、release 流水线或专门命令管），不属于 task 级 §9 verification。如需描述发布相关元数据（窗口、灰度、回滚），填到下方 `## Constraints` 段的 `<RELEASE_CONSTRAINTS>`。
>
> **§Commands 不收录 Manual** — Manual 是 task 级自由文本步骤（由 task spec §9 直接列出"用什么 checklist / 如何核验"），不需在 adapter 声明命令。`s2v_run` helper 对 `manual` key 走交互式 ack 路径（不读 adapter），见 `agents-team.md` / `agents-solo.md` §0。
>
> **多工具链 / 前后端分离项目（每类只有一个全局槽位的应对）**：本表每类（Install / Unit Test / Build / Typecheck …）只有**一个全局槽位**，`s2v_run` 对该槽位整行 `eval`；当前版本**不支持 per-module / per-area 命令矩阵**（见 init.md "当前版本只完整支持 default profile"）。前后端分离 / 多语言（如后端 `mvn test` + 前端 `vitest`）**推荐模式**：在项目内写一个聚合脚本（如 `tools/test-all.sh`：内部按目录存在性依次跑后端必跑 + 前端尽力而为，任一真失败即非零退出），槽位填该脚本路径（如 `Unit Test: bash tools/test-all.sh`）。聚合脚本须：① 任一子工具链失败 → 整体非零（不可吞错）；② 对尚未生成的模块目录（如前端在后续 phase 才建）自动跳过而非报错。此为当前版本的官方推荐绕过，per-area 命令矩阵为后续版本目标。

### Coverage 判读规则（Default Profile，可覆盖）

> ⚠️ 以下假设是 Bun / Vitest / Jest 类工具的 ASCII 表格输出。**项目可在此文档覆盖** — 只要明确「task spec 阈值对应哪一列」即可。

`<COVERAGE_COMMANDS>` 的 stdout 末尾通常会输出 ASCII 表格，每个 src 文件一行 `% Funcs / % Lines / Uncovered`。
**判读对象 = `% Lines` 列**。task spec 写 "≥80%" 即对照该列。

**覆盖示例**（其他工具）：
- Go: `go test -cover` 输出 `coverage: X.X% of statements` → 判读那个百分比
- Python pytest-cov: `pytest --cov` 输出表格的 `Cover` 列
- Rust tarpaulin: `Coverage Results: X/Y (Z%)` → 判读 Z%
- Jacoco (Java): 读 `target/site/jacoco/index.html` 的 LINE 列

如工具输出格式不同，在此文档补充判读规则示例。

### Coverage 未达标处理（task agent 行为约束）

| 实测 vs 阈值 | 应当 | 禁止 |
|---|---|---|
| ≥ 阈值 | ✅ 直接通过 | — |
| 差距 ≤ 2 行 | 检查 Uncovered → 补**真实** TEST-X.Y.Z（标新 ID 跟追踪表对齐）| ❌ `expect(true).toBe(true)` 凑数；❌ `/* istanbul ignore */` 跳过 |
| 差距 > 2 行 / 路径无法测试 | 走 §卡住协议（AGENTS.md §8 / s2v §8）→ 主 agent 决策 | ❌ 自行修改 task spec 阈值（违反 R5 / R6 — 视 tier 而定） |

---

## Constraints

- **Runtime target**: `<RUNTIME_TARGET>` <!-- 如 Node 20+ / Bun 1.x / Python 3.12 -->
- **Supported platforms**: `<SUPPORTED_PLATFORMS>` <!-- 如 macOS arm64 / Linux x64 / cross-platform -->
- **Security requirements**: `<SECURITY_REQUIREMENTS>`
- **Performance requirements**: `<PERFORMANCE_REQUIREMENTS>`
- **Compatibility requirements**: `<COMPATIBILITY_REQUIREMENTS>`
- **Release constraints**: `<RELEASE_CONSTRAINTS>`

---

## Workflow

- **Collaboration Tier**: `<COLLABORATION_TIER>`
  <!-- 必填值：solo | team。决定 git 协作严格度。详见 s2v full-standard.md §4.5 -->
  <!-- 重要：Tier 仅影响 git 协作层（branch / PR / worktree / merge gate），
       不影响 S2V 核心（SDD / BDD / TDD / §2.5 三段 commit / ADR / Verification / 追踪表 / 卡住协议）—— 所有 tier 必守 -->
  Overrides:
    - <override-key>: <override-value>   <!-- 可选；用于微调 tier 默认值，如 PR-only: false -->

---

## Phase 状态索引

> 与 Master Spec §Implementation Phases 同步。开始一个 phase 时更新此处。
>
> **Status 取值**：与 spec 顶部 Status 共用 standard.md §10.5.1 状态机 — 合法值 `Draft / Ready / In Progress / Done / Blocked / Waived`。
> ⚠️ 不要写 "not started"、"TODO"、"待开始" 等 — 不在状态机内，会让 PREFLIGHT Ready Gate 误判。

| # | Phase | Phase Spec | Status | Tasks | Worktree（仅 team）|
|---|---|---|---|---|---|
| 1 | `<phase-1-name>` | `docs/specs/phases/phase-1-<name>.md` | Draft | <count> | `<worktree-path>` |
| 2 | `<phase-2-name>` | `docs/specs/phases/phase-2-<name>.md` | Draft | <count> | `<worktree-path>` |
| ... | ... | ... | ... | ... | ... |

> 该索引由 `/s2v-add phase <name>` 自动追加；手动修改时保持一致。

## Task 总索引

> 全部 task spec 应通过 `/s2v-add task <name>` 创建；agent 进 worktree 后**禁止自创 task spec**（违反 s2v R5 单一事实源）。
>
> **Status 取值**：同 §Phase 状态索引（standard.md §10.5.1：`Draft / Ready / In Progress / Done / Blocked / Waived`）。

| Task | 模块 | Spec 文件 | Status | 依赖 / Phase 内顺序 | Worktree（仅 team）|
|---|---|---|---|---|---|
| ... | ... | ... | ... | ... | ... |

## ADR 索引

> 核心技术决策的独立记录（按 s2v full-standard §16.2 模板）。新增 ADR 用 `/s2v-add adr <title>`。
>
> **Status 取值**：ADR 自有状态机（不同于 spec）— `Proposed / Accepted / Deprecated / Superseded`。

| # | Title | Status | File |
|---|---|---|---|
| ... | ... | ... | ... |

## BDD Feature 索引

> 轻量 BDD（s2v §9.2）：`.feature` 作为业务可读场景文档。Scenario ID 在对应 task spec §7 追踪表中映射到具体测试。

| Task(s) | Feature 文件 |
|---|---|
| ... | ... |

---

### 派工模板（仅 team 档使用）

`solo` 档不需要派工 — 单人 / 单 agent 直接在主 repo 上工作。

`team` 档使用此精确 prompt 格式（避免 agent 自创 task spec + 衔接 PR-only 流程）：

```
[派工目标] task-<X.Y>（spec: docs/specs/tasks/task-<X.Y>-<name>.md）
[Worktree] <worktree-path>（按 AGENTS.md §1 拓扑）
[Branch]   <branch-name>

进入 worktree 后请：
  1. 跑 AGENTS.md §3 step 0-3 的环境校验 + 基线测试
  2. 按顺序读：AGENTS.md / 本 adapter / 该 task spec / 该 spec §5 Required Reading 列出的上游 spec / 对应 .feature 文件
  3. 严格按 task spec §6 AC + §5 Behavior Contract + §7 追踪表执行
  4. RED → GREEN → REFACTOR 三段 commit（按 AGENTS §2.5 节律 + scope 约定）
  5. 每次 commit 后立即跑 R3 grep 校验 [branch]
  6. 完成后 push branch（如有 remote）+ 在 spec §10 Completion Notes 回填 6 项
  7. 通知主 agent 跑 §4 phase smoke gate → merge PR

【硬约束】
- ✅ **允许**修改 task spec 的"流程字段"（按状态机推进）：
  - 顶部 `Status` 行（`Ready → In Progress → Done` / `Blocked` / `Waived`）
  - §7 追踪表的 Status 列（标 `Test Red` / `Verified` / `Done` 等）
  - §10 Completion Notes（完工时按 6 项回填）
- ❌ **禁止**修改 task spec 的"业务契约字段"（这些是主 agent / 用户的领域）：
  - §1 Background / §2 Goal / §3 Scope&Out-of-Scope / §4 Actors
  - §5 Behavior Contract（含 §5.1 Required Reading / §5.2 Imports / §5.3 函数签名）
  - §6 Acceptance Criteria
  - §8 Risks / §9 Verification Plan
  - 如果发现这些字段写错或需要改，**写 SPEC-DRIFT-task-X.Y.md** 让主 agent 决定，不要私自改
- ❌ 禁止新建任何 task spec（要新 task → 让主 agent 跑 `/s2v-add task <name>`）
- ❌ 禁止改 package.json / 锁文件（R7：写 NEEDS-DEP-task-X.Y.md 求助）
- ❌ 禁止 cd 主 repo / push 到 main / 自己 merge PR（R6）
- ⚠️ 卡住 → 写 BLOCKED-task-X.Y.md（AGENTS §8）→ 退出等主 agent
```
