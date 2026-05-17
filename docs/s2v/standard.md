> 📌 **快照来源**：本文件由 `/s2v-init` 在 2026-05-17 从全局 skill `/home/tajiaoyezi/.claude/skills/s2v` 复制。
>
> **请勿直接编辑此文件** — 升级 S2V 规范请改全局 skill 后重跑 `/s2v-init`（或手动 `cp` 覆盖）。

---

# S2V Development｜规格到验证驱动开发规范

> **Spec-to-Verification Development Standard**
> 面向任意软件项目的通用开发协议。
> 不绑定语言、技术栈、工具链或目录约定。每个项目通过「项目适配层」声明实际路径、命令与约束。

---

## §0 章节速查

> 22 章主规范。**跨章引用的权威锚点是章节标题本身**（`## N.` / `## N.M`），用 `grep -nE "^## " full-standard.md` 即时定位。
> 文件内的 `## Project / ## Commands / ## Context` 等是 §4 适配层 / §8.3 Task Spec / §16 ADR / §19 交付说明等模板的**内嵌子章节**，不出现在本目录中。

| 章节 | 标题 |
|---|---|
| §1 | 规范定位 |
| §2 | 核心目标 |
| §2.5 | 三段 Commit 节律 |
| §3 | 适用范围 |
| §4 | 项目适配层（含 §4.5 Collaboration Tier） |
| §5 | 分层模型 |
| §6 | 基本原则 |
| §7 | 标准开发生命周期 |
| §8 | SDD 规范（含 §8.3 Task Spec 模板 / §8.3.1 Task 颗粒度判据） |
| §9 | BDD 规范 |
| §10 | TDD 规范 |
| §11 | 集成、E2E 和运行时验证 |
| §12 | 追踪机制（含 §12.2 Traceability Status / §12.3 Waiver 流程） |
| §13 | Definition of Ready |
| §14 | Definition of Done |
| §15 | AI Vibecoding 执行协议 |
| §16 | ADR 规范 |
| §17 | 变更类型流程 |
| §18 | Review 标准 |
| §19 | 交付说明模板 |
| §20 | 新项目落地步骤 |
| §21 | 最终执行口径 |
| §22 | Installation Paths by Agent（各 agent 工具的 skill 安装路径速查）|

> **特别注意**：§8.3 Task Spec 模板内部还有一个 "## 10. Completion Notes"（**Task Spec 模板的 §10**，**不是**本规范第 10 章 TDD）。两个 §10 共存是 Task Spec 复用相同编号的设计，agent 跨引用时按上下文区分（"Task spec §10" / "standard §10"）。
>
> **定位方式（本表不再标行号 —— 根除行号漂移类）**：早期标行号且自称 grep 锚点，结果每次编辑都漂移、需手工重抓且反复失准（该 DEFECT 链已两次复发，连"已逐项核对"后又被同批次后续编辑顶歪）。**权威定位一律用章节标题**：`grep -nE "^## " full-standard.md`（或 `grep -nE "^## 8\\." full-standard.md`），即时、永不漂移。跨文件引用只用 `§N` / `## N. 标题`，**禁止裸行号**。

---

## 1. 规范定位

S2V Development 定义一套可复用的项目开发方法，而不是某个项目的目录模板、技术栈模板或工具链模板。

名称含义：

- **S**pec —— 起点是可判定的规格（SDD 层）。
- **to** —— 过程是把规格逐层转译为业务可读的场景（BDD 层）和可执行的测试（TDD 层）。
- **V**erification —— 终点是真实使用路径的验证（Integration / E2E / Runtime 层）。

它解决的问题：在 AI 辅助开发和 vibecoding 过程中，如何避免「直接开写、需求漂移、验收缺失、测试滞后、文档与实现脱节」。

本规范只规定：

- 工作流。
- 产物之间的关系。
- 质量门禁。
- 追踪机制。

本规范不规定：

- 用什么语言、框架、测试工具。
- 项目目录如何命名。
- 命令长什么样。

每个项目必须通过「项目适配层」（见 §4）声明这些项目特有信息。规范本身永远保持干净。

---

## 2. 核心目标

1. 用 SDD 明确需求、边界、约束和验收。
2. 用 BDD 把需求转成业务或用户可理解的场景。
3. 用 TDD 把关键行为转成可执行测试，再驱动实现。
4. 用集成、E2E、运行时验证确认真实使用路径。
5. 用追踪表把需求、场景、测试、实现和验证结果绑定起来。
6. 用固定门禁约束人和 AI agent 的执行顺序。
7. 让每次开发都有可恢复的上下文、可审查的证据和可复用的记录。

---

## 2.5 三段 Commit 节律

S2V 把每个 task 的 git history 强制拆成三类 commit，让 TDD 履迹可被审计、回放和驳回。这条规则**所有 tier 必守**（与 git 协作严格度无关），因此放在通用章节而非 §4.5 tier 差异化中。

### 2.5.1 三类 commit 必含

| 阶段 | type | 触发条件 | 必含动作 |
|---|---|---|---|
| **RED** | `test` | task 实现前 | 加入失败测试，本地运行确认它们 **fail for the right reason**（不是语法错 / 找不到文件等无意义失败） |
| **GREEN** | `feat` 或 `fix` | RED commit 之后 | 加入最小实现使 RED 测试全部通过，禁止改测试本身使其通过 |
| **REFACTOR**（可选） | `refactor` | GREEN 之后 | 在测试保护下改善结构。无需重构时省略此 commit |
| **§10 回填** | `docs` | task done 前 | 回写 task spec §10 Completion Notes，按 §8.3 的 6 项中文 schema（不要在此摘要里复制字段清单 — 字段以 §8.3 为唯一权威；team merge gate 4 / CI / 各实施 agent 都按 §8.3 grep 字段名）|

> **编译型语言（Java / Go / Rust / Kotlin / TS-strict 等）的 RED 桥接**：在编译型语言里，
> 测试引用一个**尚不存在**的生产类型 = **编译失败** —— 这正落在上面 "不是语法错 / 找不到
> 文件等无意义失败" 的禁止之列，**不是合法 RED**。合法做法：RED commit 里**随失败测试一并
> 提交 task spec §5.3 的可编译空骨架**（类/函数签名存在但 `throw UnsupportedOperationException` /
> 返回错值），使测试**因断言失败而红（功能未实现型），而非因编译失败而红**。**无异常语言（Go / Rust / C 等）**：等价的合法 feature-absent 标记是骨架内**刻意** `panic("unimplemented: <fn>")`（Go）/ `unimplemented!()`（Rust）—— 测试因该刻意标记而红，等同 `throw UnsupportedOperationException`，属合法 RED；但**优先净断言失败或上述刻意显式 panic**，避免骨架仅"返回零值"致测试**附带** nil 解引用 panic / 超时（那种红成因模糊、近似被禁的"无意义失败"，"fail for the right reason" 无法判定）—— 须重塑骨架或测试，使红**明确归因于功能缺失**。这是 §5.3 签名
> 在 RED 阶段（而非仅步 7 GREEN）即落地的唯一合法理由，不算"RED+GREEN 混"。解释型语言
> （Python 等）`ModuleNotFoundError` 等运行期失败本身即合法 RED，无需此桥接。配套的
> RED commit `git add` 范围见 implement.md 步 6（须含 §5.3 骨架 + 复现红测试所需的最小构建配置）。

### 2.5.2 命名约定

- **Scope** 取模块名：`parser` / `cli` / `auth` / `spec` / `agents` / `adapter` / `adr` 之一
- **示例**：
  - `test(parser): 加 SCEN-2.1.1 ~ 2.1.5 的 5 个 RED 测试`
  - `feat(parser): 实现 extractHeadings 通过全部测试`
  - `refactor(parser): 提取 walkTokens helper`
  - `docs(spec): 回填 task-2.1 §10 Completion Notes`

### 2.5.3 禁止反模式

- ❌ 一个 commit 同时包含 RED + GREEN（无法审计 TDD 履迹）—— 例外：编译型语言 RED 随失败测试一并提交 §5.3 可编译空骨架（仍因断言失败而红、非 GREEN），见 §2.5.1「编译型语言 RED 桥接」，不算本项
- ❌ commit message 写「实现 + 测试」而 diff 里测试是后补的（伪 TDD）
- ❌ GREEN 时改 RED 测试使其通过（除非测试本身确实写错，且必须 amend RED commit 而非新加 fix）
- ❌ 跳过 §10 回填直接进下一个 task（追踪表会断链）

### 2.5.4 与 git 协作 tier 的关系

- `solo` tier：三段 commit 都直接落 main
- `team` tier：三段 commit 落 feature branch，之后通过 PR 合入 main（按 §4.5.3 / AGENTS.md §4）

无论哪档 tier，**三段节律本身不可省略**——这是 S2V 核心方法论的一部分，不是协作偏好。

---

## 3. 适用范围

### 3.1 适合完整套用

- 新功能开发。
- Bug 修复。
- 重构。
- UI / UX 优化。
- API 或协议变更。
- 数据模型或持久化变更。
- 数据处理流程。
- 自动化脚本。
- 基础设施变更。
- 测试体系建设。
- 依赖或工具链变更。
- 架构演进。

### 3.2 不适合完整套用

- 一次性临时实验。
- 纯文本错别字修正。
- 无行为变化的格式化或重排。
- 明确不进入主线的探索代码。

这类场景可以走轻量流程（见 §15.4），但仍需说明范围、变更和验证方式。

---

## 4. 项目适配层

### 4.1 适配层的作用

通用规范不能假设固定路径，也不能假设固定工具。每个项目必须维护一份适配层，告诉本规范的执行方（人或 AI agent）：

- 项目用什么命名规范放规格、场景、决策、源码、测试。
- 项目用什么命令做 lint、类型检查、各级测试、构建、运行时 smoke。
- 项目有哪些语言、平台、安全、性能、合规、发布约束。

适配层让通用规范不被项目细节污染，也让 AI agent 一进项目就能拿到所有上下文，不必猜。

> 本规范中存在两类占位符：
>
> 1. **适配层占位符**（如 `<PROJECT_NAME>`、`<SPEC_HOME>`、`<UNIT_TEST_COMMANDS>`）：项目级配置，必须在适配层中声明实际值。
> 2. **模板填空占位符**（如 `<MAIN_FLOW>`、`<ACTOR>`、`<ACCEPTANCE_CRITERION>`）：模板内部按位填写的字段，由产物作者根据当下任务填入具体内容，无需写入适配层。

### 4.2 适配层必须声明的内容

| 配置项 | 含义 |
|---|---|
| `<PROJECT_NAME>` | 项目名称 |
| `<PROJECT_TYPE>` | 项目类型（如 Web、API、CLI、Mobile、Desktop、Data Pipeline、Infrastructure 等） |
| `<USER_ROLES>` | 主要使用者或调用方 |
| `<CRITICAL_WORKFLOWS>` | 核心业务或系统流程 |
| `<SPEC_HOME>` | SDD 产物所在位置 |
| `<MASTER_SPEC>` | 项目总规格入口 |
| `<PHASE_SPEC_PATTERN>` | 阶段规格命名或组织方式 |
| `<TASK_SPEC_PATTERN>` | 单任务规格命名或组织方式 |
| `<ACCEPTANCE_HOME>` | BDD 场景所在位置 |
| `<DECISION_HOME>` | ADR 或决策记录所在位置 |
| `<SOURCE_AREAS>` | 主要源码区域 |
| `<UNIT_TEST_AREAS>` | 单元测试所在区域 |
| `<INTEGRATION_TEST_AREAS>` | 集成测试所在区域 |
| `<E2E_TEST_AREAS>` | 端到端测试所在区域 |
| `<INSTALL_COMMANDS>` | 依赖安装命令（首次进入项目 + 锁文件变更后跑）|
| `<LINT_COMMANDS>` | 静态检查命令 |
| `<TYPECHECK_COMMANDS>` | 类型检查命令（如有） |
| `<UNIT_TEST_COMMANDS>` | 单元测试命令 |
| `<INTEGRATION_TEST_COMMANDS>` | 集成测试命令 |
| `<E2E_TEST_COMMANDS>` | 端到端测试命令 |
| `<BUILD_COMMANDS>` | 构建命令（执行序在 E2E 后、Coverage 前 — 先编译通过再算覆盖率）|
| `<COVERAGE_COMMANDS>` | 覆盖率命令（与 task spec §9 阈值对照判读，详见 `templates/adapter.md` Coverage 判读规则）|
| `<RUNTIME_SMOKE_COMMANDS>` | 运行时 smoke 验证命令 |
| `<RUNTIME_TARGET>` | 运行时目标（如目标 OS、运行时版本、容器、编排平台） |
| `<SUPPORTED_PLATFORMS>` | 目标平台与版本范围 |
| `<SECURITY_REQUIREMENTS>` | 鉴权、权限、加密、合规约束 |
| `<PERFORMANCE_REQUIREMENTS>` | 延迟、吞吐、资源约束 |
| `<COMPATIBILITY_REQUIREMENTS>` | 向前 / 向后兼容性约束 |
| `<RELEASE_CONSTRAINTS>` | 发布窗口、灰度、回滚约束 |
| `<COLLABORATION_TIER>` | 协作模式档位（`solo` / `team`），决定 git 协作严格度。详见 §4.5 |

未使用的条目可以留空，但不得删除字段，以免后续 agent 误以为「该项无要求」。

> **重要**：`<COLLABORATION_TIER>` 是必填字段。它**只**影响 git 协作层（branch / PR / worktree / merge gate），**不**影响 S2V 核心（SDD / BDD / TDD / Iron Law / §2.5 三段 commit / ADR / Verification / 追踪表 / 卡住协议）—— 详见 §4.5。

### 4.3 适配层模板

```markdown
# Project Development Adapter

## Project

- **Name**: `<PROJECT_NAME>`
- **Type**: `<PROJECT_TYPE>`
- **Primary users / actors**: `<USER_ROLES>`
- **Critical workflows**: `<CRITICAL_WORKFLOWS>`

## Specification Locations

- **SDD home**: `<SPEC_HOME>`
- **Master spec**: `<MASTER_SPEC>`
- **Phase spec pattern**: `<PHASE_SPEC_PATTERN>`
- **Task spec pattern**: `<TASK_SPEC_PATTERN>`
- **BDD acceptance home**: `<ACCEPTANCE_HOME>`
- **ADR home**: `<DECISION_HOME>`

## Source And Test Areas

> Source / Unit test / Integration test / E2E test 区域使用 markdown bullet list，**每行一个 git pathspec**。下游 `/s2v-implement` 把整个 list 展开为 `git add` 多参数。
>
> **强约束（Source areas / Unit test areas）**：`/s2v-implement` 步 6/7 直接消费 → 禁 `<...>` 占位 + 禁 `N/A`（占位触发 `git add` fatal）。**弱约束（Integration test areas / E2E test areas）**：当前 implement / helper 不直接消费 → 允许 `N/A: <原因>` 或保留占位（项目无对应测试时合法跳过；未来引入自动化时升级为强约束）。完整约定详见 `templates/adapter.md` §Source And Test Areas。

### Source areas

- `<SOURCE_AREAS>`

### Unit test areas

- `<UNIT_TEST_AREAS>`

### Integration test areas

- `<INTEGRATION_TEST_AREAS>`

### E2E test areas

- `<E2E_TEST_AREAS>`

## Commands

- **Install**: <INSTALL_COMMANDS>
- **Lint**: <LINT_COMMANDS>
- **Typecheck**: <TYPECHECK_COMMANDS>
- **Unit Test**: <UNIT_TEST_COMMANDS>
- **Integration tests**: <INTEGRATION_TEST_COMMANDS>
- **E2E tests**: <E2E_TEST_COMMANDS>
- **Build**: <BUILD_COMMANDS>
- **Coverage**: <COVERAGE_COMMANDS>
- **Runtime smoke**: <RUNTIME_SMOKE_COMMANDS>

> 字段语义：未填 → 留空 / `N/A: <原因>` / 真实命令；`<...>` 占位未替换会被 implement.md 与 AGENTS.md §0 helper hard-fail。
> Unit Test 是 §9 强制门槛，不允许 N/A / 留空；其余字段 N/A 时跳过执行但保留审计痕迹。
>
> ⚠️ **字段名必须加粗**（`- **Field**:` 形式）— `s2v_load_cmd` helper 的 awk 正则按 `^- \*\*Field\*\*:` 匹配，去掉加粗后 helper 全部读不到，所有 verification 命令会被当成空值跳过（unit-test 还会 hard-fail）。
>
> **如需按本地语言习惯改字段名，需同步更新 4 文件 10 处副本**：
> 1. `templates/adapter.md` — 字段名 ×1
> 2. `templates/agents-team.md` §0 — `s2v_load_cmd` ×1 + `s2v_run` 内 case ×1 + `s2v_extract_verify_keys` awk ×1（共 3 处）
> 3. `templates/agents-solo.md` §0 — 同上 3 处
> 4. `implement.md` — `s2v_load_cmd` 步 3 ×1 + `key_to_field` 函数 ×1 + `s2v_extract_verify_keys` awk 步 9 ×1（共 3 处）
>
> §Commands 不收录 Release / Deploy。发布/部署是跨 task 的项目级动作（一般由 CI/CD、release 流水线或专门命令管），不属于 task 级 §9 verification；如需描述发布元数据（窗口、灰度、回滚），填到下方 `## Constraints` 段的 `<RELEASE_CONSTRAINTS>`。

## Constraints

- **Runtime target**: `<RUNTIME_TARGET>`
- **Supported platforms**: `<SUPPORTED_PLATFORMS>`
- **Security requirements**: `<SECURITY_REQUIREMENTS>`
- **Performance requirements**: `<PERFORMANCE_REQUIREMENTS>`
- **Compatibility requirements**: `<COMPATIBILITY_REQUIREMENTS>`
- **Release constraints**: `<RELEASE_CONSTRAINTS>`

## Workflow

- **Collaboration Tier**: `<COLLABORATION_TIER>`   # solo | team — 详见 §4.5
```

适配层放在哪个文件、哪个目录由项目决定，规范不强制。常见做法是在项目入口文档中维护一份独立适配层文件并引用。

### 4.4 适配层使用规则

- 项目专属路径、命令、工具链永远写在适配层，不写进通用规范。
- 适配层是 agent 进入项目的「第一份必读文件」。
- 适配层一旦变化（如新增测试命令、迁移规格目录），立即更新；不允许长期落后于真实状态。
- 同一项目只维护一份适配层；多团队 / 多分支差异通过适配层内分节体现。

### 4.5 Collaboration Tier（协作模式档位）

`<COLLABORATION_TIER>` 字段是适配层必填项。它把"项目氛围"翻译为机器可读的硬约定，决定 git 协作严格度，但**不动 S2V 核心方法论**。

#### 4.5.1 两档预设

| 档位 | 适用场景 | 典型项目 |
|---|---|---|
| `solo` | 单人 / 快速迭代 / spike / 内部脚本 | 个人 dotfiles、临时工具、PoC |
| `team` | **任何"非单人"协作场景**：内部小团队、闭源 SaaS、公开发布 / 外部贡献的开源项目 | 内部业务系统、公开 SDK / npm 包 / 公共 CLI / 含外部贡献的开源项目 |

#### 4.5.2 所有 tier 必守（S2V 核心，不可降级）

| 维度 | 强制度 | 触发判定 |
|---|---|---|
| SDD（master / phase / task spec） | ✅ 任何 tier 必写 | 任何代码改动 → 必有对应 task spec |
| BDD（`.feature` 文件） | ✅ 任何 tier **必评估并记录结论** | 有用户/外部系统可见行为 → 必写场景；无可见行为 → 在追踪表 §7 标 N/A 并写明原因 |
| TDD Iron Law（先写失败测试） | ✅ 任何 tier 必守 | 关键逻辑 → 必写测试；遗留代码无法测 → 按 §10.4 标 Waived |
| §2.5 三段 commit 节律（RED → GREEN → REFACTOR） | ✅ 任何 tier 必守 — TDD 履迹的可验证性 | 任何 task |
| ADR（架构决策记录） | ✅ 任何 tier **必评估并记录结论** | 命中 §16.1 八类决策之一（字面取值见 §16.1「8 类决策类别」表）→ 必写 ADR；无相关决策 → 在 task §10 Completion Notes 写「无 ADR 触发」即可 |
| Verification（typecheck / test / coverage） | ✅ 任何 tier 必跑 | adapter §Commands 列出的命令 task done 前全跑 |
| §12 追踪表（AC ↔ SCEN ↔ TEST） | ✅ 任何 tier 必维护 | 每个 AC 必须有对应 BDD/TDD/Verification 行（含 N/A）|
| 卡住协议（BLOCKED 文件求助） | ✅ 任何 tier 必走 | AC 失败 ≥3 次 → 必写 BLOCKED 文件 |

> **关键澄清**：「必写」不是「凡事都生成 .feature / ADR 文件」，而是「必评估是否需要 + 把结论留痕」。
>
> - 如某 task 是纯内部重构（无外部可见行为）→ BDD 标 N/A，无需 .feature
> - 如某 task 是修一个 typo（无任何技术决策）→ ADR 标「无触发」，无需 ADR 文件
> - 但**评估和留痕动作本身不可省略** — 这是追踪表完整性的基础
>
> 任何 tier 都不得借此跳过 S2V 核心。`solo` 不是"测试 / spec / ADR 的免责证书"，只是"git 协作姿态宽松"。

#### 4.5.3 Tier 差异化对照（仅 git 协作层）

| 维度 | `solo` | `team` |
|---|---|---|
| Worktree 隔离 | ❌ 不需要 | ✅ 多 agent 时强制 |
| Feature branch | 可选 | ✅ 必须 |
| 直接 push main | ✅ 允许 | ❌ |
| PR 合入 | ❌ 不需要 | ✅（有 remote 走平台 PR；无 remote 走本地 PR 模拟，按 R6.1）|
| 主 agent gate | 自审 | ✅ 主 agent 5 步 phase smoke gate |
| Rebase 同步通知协议（§4.1）| ❌ 不需要 | ✅ 多 worktree 并发时强制 |
| Lockfile 保护（R7）| ❌ 谁开发谁加 | ✅ 走专门 chore branch + PR |
| AGENTS.md | 简化版（含 S2V 必守清单 + task SOP + BLOCKED 模板）| 完整版（含 R1-R7 / worktree / PR / phase gate / 通知协议 / BLOCKED 完整模板 / 主 agent 决策矩阵）|

> 早期 `open-source` 档已合并入 `team`：实测它独有的字段（R7 / R6.1 / R6.2 / §4.1 / 5 步 gate）全部都是"凡是协作就需要"的硬约束，没有一项是开源专属。

#### 4.5.4 Tier 字段值示例

```markdown
## Workflow

- **Collaboration Tier**: team
  Overrides:
    - PR-only: true
    - require-CI-gate: true
    - multi-reviewer: false      # 团队规模小，跳过多人 review
    - lockfile-protect: true     # 显式启用 R7（团队默认 true，写出来便于审计）
```

> ⚠️ `**Collaboration Tier**` 字段名必须加粗 — `tier.md` / `add.md` 的 grep 模式按 `^- \*\*Collaboration Tier\*\*:` 严格匹配，去粗后读不到当前 tier，幂等检查与 tier-aware commit 流程都失效。

`Overrides:` 是可选小节，允许微调单一字段以适配实际团队规模或场景，不必为此自创新 tier。

#### 4.5.5 Tier 配套产物

每档 tier 配套一份 AGENTS.md 模板（位于 `${S2V_SKILL_DIR}/templates/agents-<tier>.md`；`S2V_SKILL_DIR` 由 `/s2v-init` 步 0 `_s2v_skill_dir` resolver 解析，Claude Code 默认 `~/.claude/skills/s2v/templates/`，其他 agent 见 §22），由 `/s2v-init` 命令在项目初始化时按 tier 自动生成。

后续如需调整 tier，使用 `/s2v-tier <new-tier>` 命令重新生成 AGENTS.md（含升降档影响清单）。

---

## 5. 分层模型

| 层级 | 解决的问题 | 主要产物 | 成功标准 |
|---|---|---|---|
| SDD | 做什么、为什么做、边界在哪里 | Master Spec、Phase Spec、Task Spec | 范围清楚，验收标准可判定 |
| BDD | 用户或业务如何感知结果 | Feature 文件、Scenario、验收场景表 | 场景可读，覆盖主流程、异常流、边界流 |
| TDD | 代码行为是否正确 | 单元测试、组件测试、服务测试、模块测试 | 关键行为先有失败测试或明确豁免 |
| Integration | 模块之间是否协作正确 | 集成测试、契约测试、API 测试 | 跨边界行为可验证 |
| E2E / Runtime | 真实使用路径是否成立 | 端到端测试、运行时 smoke、手工验收记录 | 用户路径或系统路径可证实 |
| ADR | 为什么做这个技术决策 | 决策记录 | 背景、选择、替代方案、影响可追溯 |

> 表中产物是测试**类型**或文档**角色**，不是具体工具。具体工具由适配层声明。

---

## 6. 基本原则

### 6.1 单一事实源

每个功能、修复或重构都必须有一个 SDD 入口作为事实源。实现过程中发现需求变化，必须先更新 SDD，再同步 BDD、测试和实现。

禁止让聊天记录、临时注释、口头约定或未归档的 TODO 成为长期事实源。

### 6.2 先契约，后实现

非平凡改动进入实现前，必须明确：

1. 目标。
2. 范围。
3. 非范围。
4. 验收标准。
5. 测试策略。
6. 风险。
7. 验证方式。

没有可判定的验收标准时，不进入实现。

### 6.3 测试与风险匹配

测试强度由风险决定，不由目录或代码量决定。

高风险改动需要更多验证：

- 影响核心流程。
- 影响数据一致性。
- 影响鉴权、权限、安全。
- 影响支付、计费、生产数据、用户隐私。
- 涉及并发、缓存、异步任务、外部系统。
- 改变公共 API、协议或持久化结构。

低风险改动可以走轻量验证（见 §15.4），但不能没有验证说明。

### 6.4 不强绑工具链

本规范不规定必须使用某个测试框架、语言、构建工具或目录结构。

项目可以使用任何合适工具，但必须在适配层声明：

- 用什么工具。
- 命令是什么。
- 哪些测试覆盖哪些风险。
- 无法自动化的部分如何验证。

### 6.5 渐进落地

不要求一次性引入全部测试层级。允许按 SDD → BDD → TDD → Integration → E2E 的顺序分阶段引入，但每个新增层级都必须在适配层登记，并在 Task Spec 的追踪表中体现。

---

## 7. 标准开发生命周期

```text
Idea
  -> SDD
  -> BDD
  -> Test Strategy
  -> TDD Red
  -> Implementation Green
  -> Refactor
  -> Integration / E2E / Runtime Verification
  -> Documentation Backfill
  -> Review
  -> Merge / Release
```

### 7.1 阶段门禁

| 阶段 | 进入条件 | 退出条件 |
|---|---|---|
| Idea | 出现需求、问题或改动意图 | 目标、优先级、范围初步明确 |
| SDD | 可以描述问题和价值 | Task Spec 完成，验收标准可判定 |
| BDD | 有用户路径、业务流程或外部可见行为 | 主流程、异常流、边界场景完成 |
| Test Strategy | 已知影响面 | 单元 / 集成 / E2E / 手工验证的选择已记录 |
| TDD Red | 关键行为可测试 | 失败测试存在，或记录豁免 |
| Implementation Green | 有测试或验收策略 | 最小实现完成，相关测试通过 |
| Refactor | 行为已受测试保护 | 结构优化完成，测试仍通过 |
| Verification | 功能可运行 | 自动化或手工验证有结果 |
| Backfill | 实现和验证完成 | SDD、BDD、追踪表、ADR 更新 |
| Review | 产物完整 | 发现项处理，剩余风险明确 |

---

## 8. SDD 规范

### 8.1 Master Spec

Master Spec 是项目总入口，描述项目级事实。

必须包含：

1. 背景。
2. 目标。
3. 范围和非范围。
4. 用户或使用者。
5. 核心流程。
6. 技术约束。
7. 质量标准。
8. 风险登记册。
9. 阶段规划。
10. 决策记录索引。

存放位置由适配层 `<MASTER_SPEC>` 指定。

### 8.2 Phase Spec

Phase Spec 描述一个阶段或里程碑。

必须包含：

1. 阶段目标。
2. 业务价值。
3. 涉及模块。
4. 任务清单（表格 Spec 列路径**必须**用 phase 文件相对路径 `../tasks/task-X.Y-name.md`，便于 IDE 内点击跳转；adapter §Task 总索引保持项目根路径，两者分工不同：phase §4 是 phase 视角、adapter 是项目根视角）。
5. 依赖关系。
6. 阶段级验收标准。
7. 阶段级风险。
8. 阶段级 Definition of Done。

命名与组织方式由适配层 `<PHASE_SPEC_PATTERN>` 指定。

> **§6 是受门禁约束的集成兜底（C1）**：阶段级验收标准 + 端到端 smoke 不是可选说明文字。
> S2V 对每个 task 隔离严格 TDD，但"task 拼起来能否集成"只此一层兜底；§6 留空 = 集成层
> 无人把关（三轮黑盒互证最强缺口）。phase 的最后一个 task 完工/合并前，
> `scripts/lib/preflight.sh` 的 `s2v_preflight_phase` 强制 §6 已填实（非空、无 `<TBD>`
> 占位）且 phase Status 为 §10.5.1 合法值 —— solo 在实施收尾（AGENTS.md SOP 步 7）、
> team 在 §4 Gate 3 执行。

### 8.3 Task Spec

Task Spec 是最小执行单元。简单任务可以写在 Phase Spec 内；复杂任务必须拆为单独文件。

模板：

````markdown
# Task `<TASK_ID>`: `<TASK_NAME>`

> ⚠️ **Status: Draft** — 此 spec 含**两类待处理项**，**禁止进入实施阶段**（包括 `/s2v-implement` 或任何写代码的 agent）。
>
> **进入实施前必做（两类操作分开）**：
>
> 1. **填空（必填）**：清零所有 `<TBD-by-user>` 和 `// TBD` 占位 — 这些是 PRD 没说、必须由你定的字段（典型：§3 Scope 文件清单 / §4 Actors / §5.2 Imports / §5.3 函数签名）。**不填完不能 Ready**。
> 2. **审核（review）**：检查已由 init/add 推导给值的字段（典型：§6 AC 由 PRD 推导带 `(PRD §X)` 引用 / §7 追踪表 ID 编号 / §9 Verification 命令来自 adapter）。发现偏差直接修改本节内容；通过后**无需删除**"由 init 推导"的标记注释。
> 3. **改状态**：把上方 Status 字段从 `Draft` 改为 `Ready`。
>
> Agent 检测到 `Status: Draft` 必须**主动停下并提示用户**，不可继续。详见 `full-standard.md` §10.5.1 状态机。
>
> ⚠️ **禁止混合占位写法**：`- [ ] <TBD-by-user> AC1: bun install 退出码 0` 这种"既给了答案又挂了占位"形式造成 review 者陷入"删 prefix vs 重写整条"的伪决策疲劳。init/add 必须二选一：**模式 A 完整给值**（典型 §6 AC，详见下方 §6 渲染规则）或**模式 B 完全留白 `<TBD-by-user>`**（典型 §3 / §4 / §5）。

**Status**: Draft

> Allowed values: `Draft` · `Ready` · `In Progress` · `Blocked` · `Waived` · `Done`（详见 `full-standard.md` §10.5.1 状态机）。
> **Status 行必须只有一个枚举值**，不要保留管道符列表 — 否则 agent / CI 的 grep 检查会误判（例如 `grep "Status: Draft"` 在枚举行也命中）。

**Priority**: P0

> Allowed values: `P0` · `P1` · `P2` · `P3`
**Owner**: `<OWNER>`
**Related Phase**: `<PHASE_ID>`
**Dependencies**: `<DEPENDENCIES>`

## 1. Background

为什么需要这次改动。

## 2. Goal

任务完成后应该成立的事实。

## 3. Scope

### In Scope

- ...

### Out Of Scope

- ...

## 4. Users / Actors

- `<ACTOR>`：`<HOW_THEY_INTERACT>`

## 5. Behavior Contract

描述外部可观察行为、API 契约、数据契约或系统契约。

### 5.1 Required Reading

- `<上游 task spec / ADR 路径>` + 对应 BDD `test/features/<module>.feature`（`/s2v-init` / `/s2v-add` 渲染时自动列出上游 task / ADR + `.feature`；`/s2v-implement` 步 0 按本节链路读取所有引用）

### 5.2 Imports

- `<TBD-by-user>`（本 task 实现需引入 / 依赖的模块、包、内部符号；Draft 期留 `<TBD-by-user>`，Ready 前用户填实）

### 5.3 函数签名

- `<TBD-by-user>`（本 task 新增 / 修改的关键函数 / 接口签名骨架；Draft 期留 `<TBD-by-user>`，Ready 前用户填实）

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：
     - init/add 基于 PRD 推导出 AC 内容，**完整写出**（不挂 <TBD-by-user> 前缀）
     - 每条 AC 加引用：`- [ ] **AC<N>** (PRD §<reference>): <内容>`
       - PRD 已写明 → 引用精确章节，例 `(PRD §AC.1)` / `(PRD §Behavior Contract)`
       - PRD 没写、由 task 推导 → 标 `(本 task 新增)`
     - 用户 review 阶段：发现偏差直接改 AC 内容；review 通过**无需删除本注释**
     - **严禁** `- [ ] <TBD-by-user> AC<N>: 内容` 混合写法（伪决策疲劳源）
-->

- [ ] **AC1** (PRD §`<reference>`): `<ACCEPTANCE_CRITERION_1>`
- [ ] **AC2** (PRD §`<reference>`): `<ACCEPTANCE_CRITERION_2>`

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| `<CRITERION>` | `<SCENARIO_ID>` | `<TEST_ID>` | `<FLOW_TEST_ID>` | `<COMMAND_OR_MANUAL_CHECK>` | Not Started |

## 8. Risks

- ...

## 9. Verification Plan

- **Install**: `<INSTALL_COMMANDS>`  <!-- 与 adapter §Commands `Install` 一致 -->
- **Lint**: `<LINT_COMMANDS>`
- **Typecheck**: `<TYPECHECK_COMMANDS>`
- **Unit**: `<UNIT_TEST_COMMANDS>`  <!-- 强制：实施 agent 不允许 N/A -->
- **Integration**: `<INTEGRATION_TEST_COMMANDS>`
- **E2E**: `<E2E_TEST_COMMANDS>`
- **Build**: `<BUILD_COMMANDS>`  <!-- 与 adapter §Commands `Build` 一致；执行序在 E2E 之后、Coverage 之前 -->
- **Coverage**: `<COVERAGE_COMMANDS>`
- **Runtime smoke**: `<RUNTIME_SMOKE_COMMANDS>`
- **Manual**: `<MANUAL_VERIFICATION_STEPS>`

> 字段名加粗与 §4.3 adapter §Commands 模板风格统一。`s2v_extract_verify_keys` 的 awk 模式用 `(\*\*)?` 可选匹配，所以 task §9 字段加粗或非加粗都能识别；推荐按本模板加粗以保持一致。

## 10. Completion Notes

> **结构层级（避免数字混淆）**：
>
> - **顶层 6 项 outline**（本节定义的权威 schema）：完成日期 / 改动文件 / commit 列表 / §9 Verification 结果 / 剩余风险 / 下游 task 影响
> - **outline 内部展开**：其中第 4 项 `§9 Verification 结果` 内部按本 task §9 实际执行项**逐行展开**（最多 10 行：install / lint / typecheck / unit-test / integration / e2e / build / coverage / runtime-smoke / manual）。其他 5 项是单段文本/列表。
> - **第 7 项 Waiver 登记**：条件性字段。本 task 没有任何 Waived AC 时**不出现**；有任意 Waived AC 时**必填**，每个 Waive 按 §12.3 五项展开。
>
> **gate 识别口径**：team merge gate 4 / CI 检查器 / 外部 agent 都**按 6 个 outline 字段名 grep**；"§9 Verification 结果"内部子项数应等于本 task §9 实际列出的字段数（参见本节示例与 implement.md 步 10）。
>
> `/s2v-implement` 在 task done 时按此格式回填；`/s2v-add task` 在 init 时填占位 `<TBD-after-impl>`。

- **完成日期**：YYYY-MM-DD
- **改动文件**：
  - `<source-file-1>`（新增/修改）
  - `<source-file-2>`
  - `<test-file-1>`
- **commit 列表**：
  - `<hash1>` test: 加 RED 测试
  - `<hash2>` feat: 实现
  - `<hash3>` refactor:（如有）
- **§9 Verification 结果**：（按本 task §9 实际执行项逐项记录；§9 没列的删除该行；执行了的填实际结果，避免"验证已执行但审计记录缺失"）
  - install: ✅ / skipped: <reason> / N/A
  - lint: ✅ / skipped: <reason> / N/A
  - typecheck: ✅ / skipped: <reason> / N/A
  - unit-test: N passed / 0 failed   <!-- 强制：unit-test 不允许 skipped -->
  - integration: ✅ / skipped: <reason> / N/A
  - e2e: ✅ / skipped: <reason> / N/A
  - build: ✅ / skipped: <reason> / N/A
  - coverage: NN.N% / 阈值 NN%
  - runtime-smoke: ✅ <evidence: 端口/截图/日志> / skipped: <reason>
  - manual: ✅ <证据/截图/确认者> / N/A: <reason>
- **剩余风险 / 未做项**：`<RISK_OR_NONE>`
- **下游 task 影响**：`<DOWNSTREAM_OR_NONE>`
- **Waiver 登记**（仅在本 task 含 Waived AC 时出现 — §12.3 五项展开，每个 Waive 一段）：
  - **AC<N> Waived**：
    - 豁免对象：`<具体 AC 描述或 SCEN-X.Y.N>`
    - 原因：`<技术 / 业务 / 时间约束>`
    - 替代验证：`<命令 / 手工 checklist / 验证证据>`
    - 补齐条件：`<何时 / 触发条件 / 升级路径>`
    - 负责人：`<主 agent / 用户 / 关联 ADR / Issue 链接>`
````

> Waiver 五项规范见 §12.3；本节是其在 Task Spec 中的**唯一承载位置**（不要新建 §11 / §12 段，也不要混入"剩余风险"自由文本里）。`/s2v-implement` 步 11.B 卡住路径选 Waive 时按此格式追加；team merge gate 4 检测到 Status = Waived 但 §10 缺 Waiver 登记会 BLOCK。

#### 8.3.1 Task 颗粒度判据

一个 Task Spec 的拆分粒度必须同时满足：

- **可循环**：单次 TDD Red → Green → Refactor 循环内可完成
- **可提交**：一次 commit 可收尾，不留中间态
- **可验证**：§9 Verification Plan 必须包含 Unit / unit-test 条目，且 unit-test 可在 Task 完成时执行并通过；其他验证项按 task 实际风险列入并记录。
  - unit-test 是 §9 强制门槛；见 adapter Commands、task spec §9、`/s2v-implement` 步 9 与 AGENTS `s2v_verify_full`。不允许 N/A、留空或 skipped。

任一条不满足，必须再拆。不以时间为粒度判据。

### 8.4 SDD 编写规则

SDD 应该写：

- 用户价值。
- 可观察行为。
- 输入输出契约。
- 数据约束。
- 权限和安全约束。
- 性能和兼容性要求。
- 明确不做什么。
- 验证方式。

SDD 不应该写：

- 未确认的具体实现路径。
- 随意承诺的依赖。
- 不能判定的验收标准。
- 无 owner 的 TODO。
- 没有边界的「优化」「增强」「完善」。
- 项目专属硬编码路径或命令（这些只能进适配层）。

### 8.5 Spec / Plan 提交前自审清单

Spec 或 Plan 进入 Implementation 阶段前，必须按顺序完成 3 步自审：

1. **验收标准映射检查**：§6 Acceptance Criteria 每条必须在 §7 Traceability 表找到 BDD Scenario / TDD Test / Verification 三栏归宿。
2. **占位符扫描**：按 §10.5.1 反例对照表扫描，不允许任何 TODO / TBD / 模糊动词。
3. **跨 Task 一致性检查**：类型名、方法名、接口名、Actor 名在 Master / Phase / Task Spec 之间命名一致。

任意一步不通过，退回 SDD 阶段补齐，禁止进入 Implementation。

---

## 9. BDD 规范

### 9.1 BDD 的定位

BDD 用业务或用户语言描述外部可感知的行为。它关心「外部如何感知系统」，不关心内部如何实现。

适用场景：

- 用户界面流程。
- API 使用流程。
- CLI 命令流程。
- 数据处理流程。
- 异步任务流程。
- 权限和错误流程。
- 跨系统交互流程。

### 9.2 轻量 BDD

`.feature` 文件可以作为业务可读的场景文档，是否引入 step definitions 由项目自行决定：

- 轻量项目可以只把 `.feature` 当文档，不绑定执行框架，由对应的执行测试在追踪表中引用 Scenario ID 即可。
- 需要严格自动化的项目可以引入 step definitions 工具链，让 `.feature` 直接驱动执行。

无论选择哪种方式，每个 BDD 场景都必须能在追踪表中映射到一种验证方式。

### 9.3 Scenario 模板

```gherkin
Feature: `<FEATURE_NAME>`
  In order to `<BUSINESS_VALUE>`
  As a `<ACTOR>`
  I want `<CAPABILITY>`

  Background:
    Given `<COMMON_PRECONDITION>`

  Scenario: `<MAIN_FLOW>`
    Given `<STATE>`
    When `<ACTION>`
    Then `<OBSERVABLE_RESULT>`

  Scenario: `<ERROR_FLOW>`
    Given `<ERROR_PRECONDITION>`
    When `<ACTION>`
    Then `<ERROR_OR_PROTECTION_RESULT>`

  Scenario: `<BOUNDARY_FLOW>`
    Given `<BOUNDARY_STATE>`
    When `<ACTION>`
    Then `<EXPECTED_BOUNDARY_RESULT>`
```

存放位置由适配层 `<ACCEPTANCE_HOME>` 指定。

### 9.4 BDD 编写规则

BDD 应该：

- 使用业务语言。
- 描述角色、动作和结果。
- 覆盖主流程、异常流、边界流。
- 说明前置条件。
- 能映射到自动化或手工验收。

BDD 不应该：

- 依赖具体 UI selector。
- 暴露内部函数名。
- 复制实现逻辑。
- 把测试数据准备细节写成业务规则。
- 写成单纯的技术 checklist。

### 9.5 BDD 到执行测试的映射

每个重要 BDD 场景必须在追踪表中映射到一种验证方式：

- 单元测试。
- 集成测试。
- 端到端测试。
- 契约测试。
- 运行时 smoke。
- 手工验收。
- 明确豁免。

不能自动化的场景必须按 §12.3 写明豁免原因和替代验证方式。

---

## 10. TDD 规范

### 10.1 TDD 的定位

TDD 用来锁定关键代码行为。它不是覆盖率表演，也不是 E2E 的替代品。

TDD 优先覆盖：

- 纯逻辑。
- 数据转换。
- 状态机。
- 权限判断。
- 输入校验。
- 错误处理。
- 边界条件。
- 并发控制。
- 缓存策略。
- 协议解析。
- 业务规则。

### 10.2 Red-Green-Refactor

每个 TDD 循环包含：

1. **Red**：写一个会失败的测试，证明当前行为缺失或错误。
2. **Green**：写最小实现让测试通过。
3. **Refactor**：在测试保护下改善结构。

### 10.3 测试粒度选择

| 目标 | 推荐测试 |
|---|---|
| 纯函数行为 | 单元测试 |
| 状态流转 | 单元测试或模块测试 |
| UI 组件行为 | 组件测试或交互测试 |
| API 合约 | 契约测试或集成测试 |
| 数据库读写 | 集成测试 |
| 跨服务流程 | 集成测试或 E2E |
| CLI 行为 | 命令级测试或输出回归测试 |
| 桌面或移动端流程 | UI 自动化或手工验收记录 |
| 基础设施变更 | plan / dry-run / smoke 验证 |
| 数据任务 | 输入输出样本测试和回归数据集 |

> 表中列出的是测试**类型**而非测试**工具**。具体工具由适配层 `<UNIT_TEST_COMMANDS>` 等条目声明，本规范不规定具体框架。

### 10.4 允许的例外

- 遗留系统没有测试入口。
- 外部系统难以模拟。
- UI、硬件或运行时交互暂时无法自动化。
- 现有工具链不支持。

例外必须在追踪表中以「Waived」状态记录，并按 §12.3 写明替代验证方式。

### 10.5 TDD 禁止事项

禁止：

- 先写完整实现，再补弱断言测试。
- 为了通过测试删除关键断言。
- 只测试 happy path。
- 静默吞错误。
- 用 mock 掩盖真实契约变化。
- 把不稳定测试留在主线且没有隔离策略。

#### 10.5.1 Spec Status 状态机（决定何时禁 TBD）

每个 Task Spec / Phase Spec 顶部必须声明 `Status` 字段，状态机如下：

| Status | 是否允许 `<TBD-by-user>` / `// TBD` | 是否可进入实现 | 谁可推进 |
|---|---|---|---|
| **Draft** | ✅ 允许（占位用） | ❌ 禁止进 implementation | `/s2v-init` 自动产 / `/s2v-add` 默认产 |
| **Ready** | ❌ 禁止（必须清零所有 TBD）| ✅ 可启动 RED → GREEN | 用户审核完业务字段后手动改 Draft → Ready |
| **In Progress** | ❌ 禁止新增未跟踪 TBD | ✅ 实施中 | 实施 agent 进 RED 时改 Ready → In Progress（`/s2v-implement` 自动；外部 agent / 手动实施按 AGENTS.md 5 步 SOP，可不写入文件以免无意义 commit） |
| **Done** | ❌ 禁止 | ✅ 已完成 | 实施 agent 在 §10 Completion Notes 回填后改 In Progress → Done（与 §10 回填合并 commit）；**不依赖 `/s2v-implement` skill** |
| **Blocked** | ❌ 禁止 | ⏸ 暂停 | 卡住协议触发（§12.2 状态枚举）|
| **Waived** | ⚠️ 必须按 §12.3 五项填豁免 | ✅ 跳过 | 主 agent 决策 |

**关键铁律**：

- `/s2v-init` 一次性生成的全套文档**默认是 Draft 状态**。这是合法的——init 命令的目标是搭好骨架，业务字段由用户填
- `/s2v-implement <task>` **必须先检查 Status**：如果是 Draft，**拒绝执行**并提示用户先把业务字段填完后改成 Ready
- §10.5.2 的占位符反例对照表**仅在 Ready 及之后状态强制**——Draft 状态下 TBD 是合法的

#### 10.5.2 占位符反例对照（Ready+ 状态强制）

以下反例在 Ready / In Progress / Done 状态的 SDD / BDD / TDD 产物中**均不允许**（Draft 状态可用作占位）：

| ❌ 反例 | ✅ 必须替换为 |
|---|---|
| `// TODO: implement later` | 完整可运行实现 |
| `add error handling` | 具体异常类型 + 处理路径 |
| `// TBD` 或 `<TBD-by-user>` | 具体命令 + 预期输出 |
| `handle edge cases` | 列出每个边界条件 |
| `// fix this later` | 拆为新 Task Spec，禁止留在当前任务 |

---

## 11. 集成、E2E 和运行时验证

### 11.1 目的

单元测试证明局部行为，集成与 E2E 验证真实协作路径。

需要额外验证的情况：

- 多模块协作。
- 用户界面流程。
- API 调用链。
- 数据库、缓存、队列、文件系统、外部服务。
- 权限和鉴权。
- 构建、启动、安装、升级。
- 桌面、移动端、浏览器、硬件或运行时交互。

### 11.2 验证类型

| 类型 | 适用场景 |
|---|---|
| Integration Test | 模块之间有真实依赖 |
| Contract Test | API、协议、事件格式、SDK 对外契约 |
| E2E Test | 用户路径或系统路径 |
| Runtime Smoke | 启动、连接、基础操作可用 |
| Manual Verification | 暂时无法自动化但必须验证 |
| Dry Run / Plan | 基础设施、数据迁移、批处理任务 |

具体执行方式由适配层 `<INTEGRATION_TEST_COMMANDS>`、`<E2E_TEST_COMMANDS>`、`<RUNTIME_SMOKE_COMMANDS>` 等条目声明。

### 11.3 验证记录

每次完成必须记录：

1. 运行了什么命令。
2. 结果是什么。
3. 没有运行什么。
4. 为什么没有运行。
5. 替代检查是什么。
6. 剩余风险是什么。

记录写在 Task Spec 的「Completion Notes」或交付说明（见 §19）中。

---

## 12. 追踪机制

### 12.1 追踪表

追踪表是 SDD、BDD、TDD 与实现之间的核心连接。

```markdown
| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| `<CRITERION>` | `<SCENARIO_ID>` | `<TEST_ID>` | `<FLOW_TEST_ID>` | `<COMMAND_OR_MANUAL_CHECK>` | Not Started |
```

每个 Task Spec 都必须维护一份追踪表。验收标准没有对应行的，视为未规划。

### 12.2 状态枚举

| 状态 | 含义 |
|---|---|
| Not Started | 已定义但未开始 |
| Spec Ready | SDD 已完成 |
| Scenario Ready | BDD 已完成 |
| Test Red | 失败测试已存在 |
| In Progress | 实现中 |
| Verified | 自动或手工验证通过（**中间态** — 合 PR 前须推进到 `Done`，team Gate 4 第 3.5 道会拦截 §7 行级残留 `Verified`） |
| Waived | 有明确豁免 |
| Blocked | 存在阻塞 |
| Done | 完成且已回写 |

### 12.3 豁免规则

任何测试或验证豁免必须写清：

1. 豁免对象。
2. 豁免原因。
3. 替代验证。
4. 补齐条件。
5. 负责人或触发条件。

无说明的豁免视为风险，Review 时必须打回。

---

## 13. Definition of Ready

任务进入实现前必须满足：

- [ ] 目标明确。
- [ ] 范围明确。
- [ ] 非范围明确。
- [ ] 验收标准可判定。
- [ ] 关键用户、调用方或系统 actor 已明确。
- [ ] 影响面已初步识别。
- [ ] 测试策略已明确。
- [ ] 依赖和风险已记录。
- [ ] 项目适配层能告诉 agent 应该用哪些路径和命令。

不满足时，应先补规格或更新适配层，不应直接实现。

---

## 14. Definition of Done

任务完成必须满足：

- [ ] SDD 状态已更新。
- [ ] 验收标准已逐项确认。
- [ ] BDD 场景已新增、更新或明确不需要。
- [ ] TDD 测试已新增、更新或记录豁免。
- [ ] 集成、E2E 或运行时验证已完成或记录豁免。
- [ ] 适配层声明的相关命令已运行，或说明不能运行的原因。
- [ ] 错误路径已处理。
- [ ] 边界条件已处理。
- [ ] 用户可见行为或系统外部行为符合验收。
- [ ] 文档、测试和实现没有明显冲突。
- [ ] 没有无 owner 的 TODO、临时 mock、placeholder 留在主线。
- [ ] 剩余风险已记录。

> **Done 由真实 §9 通过把关，非自证（C8）**：canonical 路径（`/s2v-implement` 步 9 /
> solo SOP 步 4 的 `s2v_verify_full ... || exit`）已隐式绑定 —— §9 红则脚本提前退出、
> 到不了 Status→Done，**canonical 路径无额外复跑开销**。仅当 agent **不照脚本跑**、
> 直接手改 Status 时须显式 `s2v_require_green`（复跑 §9，**自动排除 manual**：manual
> 已在 §9 阶段人工确认，复跑会重复 /dev/tty 提示且非交互环境 rc2）；team 另有 §4
> Gate 2 于合并时复核。§10「§9 Verification 结果」是**审计记录**，非放行依据；其手抄
> 数字真伪不被机械校验（需另一次解析，属固有 scope）。

---

## 15. AI Vibecoding 执行协议

### 15.1 每次任务开始时

AI agent 必须：

1. 读取项目适配层。
2. 找到或创建对应 SDD task。
3. 收集相关源码、测试、文档与命令上下文。
4. 明确本次范围与非范围。
5. 识别需要新增或更新的 BDD、TDD、集成、E2E 产物。
6. 给出简短执行计划。

### 15.2 实现前必须回答

1. 这次改动对应哪个 SDD task？
2. 谁会感知这个变化？
3. 主流程是什么？
4. 异常流程是什么？
5. 边界条件是什么？
6. 哪些行为需要 TDD？
7. 哪些路径需要集成或 E2E？
8. 完成后要回写哪些产物？

### 15.3 AI agent 禁止事项

禁止：

- 未读现有上下文就生成新结构。
- 没有验收标准就直接实现。
- 把项目专属目录或命令硬编码进通用规范。
- 在适配层之外写项目特有的路径假设。
- 修改与本次任务无关的文件。
- 静默吞错误。
- 用「应该可以」替代验证证据。
- 留下无主 TODO、临时 mock、placeholder。
- 把无法运行的验证说成已通过。

### 15.4 轻量流程

低风险任务可以走轻量流程：

1. 明确范围。
2. 修改。
3. 运行最小相关验证。
4. 汇报结果与剩余风险。

适用：文案修正、样式微调、单文件 bug 修复、明确的测试修复、文档格式修正。

低风险不等于无验证。轻量流程仍需说明跑了什么、剩余什么风险。

---

## 16. ADR 规范

### 16.1 何时必须写 ADR

- 引入或替换核心依赖。
- 改变架构边界。
- 改变数据模型或持久化方式。
- 改变 API、协议、事件格式。
- 改变鉴权、权限、安全策略。
- 改变测试工具链。
- 改变发布、部署、运行时模式。
- 做不可轻易回滚的技术决策。

#### 8 类决策类别（`类别`列字面取值的**唯一权威**）

PRD §Decisions Log `类别` 列、ADR `Category` 字段、`/s2v-init` 步 9.2「8 类是否都覆盖」审计，
**一律从下列 8 个字面值中选其一**（下游做字符串相等匹配 —— prd.md / init.md 一律引用本表，
不得各自另写同义词，避免"字面对齐"失配）：

| # | 类别（字面值） | 对应上方触发条件 |
|---|---|---|
| 1 | `架构` | 改变架构边界 |
| 2 | `依赖` | 引入或替换核心依赖 |
| 3 | `数据持久化` | 改变数据模型或持久化方式 |
| 4 | `协议接口` | 改变 API、协议、事件格式 |
| 5 | `安全` | 改变鉴权、权限、安全策略 |
| 6 | `测试工具链` | 改变测试工具链 |
| 7 | `部署发布` | 改变发布、部署、运行时模式 |
| 8 | `兼容性` | 做不可轻易回滚的技术决策（OS / runtime / 数据格式向前后兼容）|

### 16.2 ADR 模板

```markdown
# ADR `<ADR_ID>`: `<TITLE>`

**Status**: Proposed | Accepted | Deprecated | Superseded
**Date**: `<DATE>`

## Context

## Decision

## Rationale

## Alternatives

## Consequences

## Rollback Or Migration Plan

## Follow-ups
```

存放位置由适配层 `<DECISION_HOME>` 指定。

---

## 17. 变更类型流程

### 17.1 新功能

完整走：

1. SDD。
2. BDD。
3. 测试策略。
4. TDD。
5. 实现。
6. 集成或 E2E。
7. 回写追踪表。

### 17.2 Bug 修复

必须先复现：

1. 在 Task Spec 中记录问题。
2. 写失败测试或可复现步骤。
3. 修复。
4. 验证不回归。
5. 回写完成记录。

### 17.3 重构

必须满足：

1. 用户可见行为或外部契约不变。
2. 先确认测试保护。
3. 测试不足时先补测试或记录风险。
4. 重构后运行相关验证。

### 17.4 UI / UX 改动

至少考虑：

1. 默认状态。
2. 加载状态。
3. 空状态。
4. 错误状态。
5. 禁用状态。
6. 权限状态。
7. 响应式或平台差异。
8. 可访问性。

### 17.5 API / 协议改动

必须考虑：

1. 兼容性。
2. 调用方影响。
3. 错误码或错误结构。
4. 版本策略。
5. 契约测试。
6. 迁移计划。

### 17.6 数据或迁移改动

必须考虑：

1. 数据备份。
2. 回滚策略。
3. 幂等性。
4. dry-run。
5. 小样本验证。
6. 生产风险。

### 17.7 依赖或工具链改动

必须考虑：

1. 为什么需要。
2. 替代方案。
3. 兼容性。
4. lockfile 或版本锁定。
5. CI 影响。
6. 本地开发影响。
7. 回滚方式。

核心依赖或工具链变更必须同步写 ADR，并更新适配层中相关命令与约束。

---

## 18. Review 标准

Review 优先看行为和风险，不只看代码风格。

检查顺序：

1. 是否有对应 SDD。
2. 验收标准是否被满足。
3. BDD 场景是否覆盖真实流程。
4. 测试是否能发现实际回归。
5. 错误和边界是否处理。
6. 是否有未声明的架构或依赖变化。
7. 是否有无关重构。
8. 验证证据是否可信。
9. 文档、测试和实现是否一致。
10. 剩余风险是否记录。

---

## 19. 交付说明模板

适用于 commit 描述、PR 描述、release notes 或任意需要总结一次交付的场合。

```markdown
## Summary

- ...

## SDD

- Task: `<TASK_ID>`
- Acceptance criteria updated: `<COUNT_OR_LIST>`

## BDD / TDD / Verification

- BDD: `<SCENARIO_IDS>`
- TDD: `<TEST_IDS>`
- Integration / E2E: `<FLOW_TEST_IDS>`
- Manual: `<MANUAL_STEPS>`

## Commands

- `<COMMAND>`: passed | failed | not run（附原因）

## Changed Areas

- Specs: `<SPEC_PATHS>`
- Source: `<SOURCE_PATHS>`
- Tests: `<TEST_PATHS>`
- ADR: `<ADR_PATHS>`

## Risks

- ...

## Follow-ups

- ...
```

---

## 20. 新项目落地步骤

新项目从零接入本规范，按以下顺序落地：

1. 创建或指定项目适配层。
2. 在适配层声明 SDD、BDD、测试、ADR 的实际位置。
3. 在适配层声明 lint、typecheck、各级测试、build、runtime smoke 命令。
4. 建立 Master Spec。
5. 为第一个真实任务写 Task Spec。
6. 为该任务写至少一个 BDD 场景。
7. 为关键行为写 TDD 测试或在追踪表中记录豁免。
8. 实现任务。
9. 运行适配层声明的相关验证。
10. 回写追踪表与 Completion Notes。
11. 在 Review 中检查规范是否真的被执行。

---

## 21. 最终执行口径

以后使用本规范进行 vibecoding，默认口径如下：

1. 没有项目适配层，先建立适配层。
2. 没有 SDD，不进入实现。
3. 没有可判定的验收标准，不进入实现。
4. 有用户或外部系统可见行为，就写 BDD。
5. 有关键逻辑，就写 TDD。
6. 有跨模块或真实运行路径，就做集成、E2E 或 runtime smoke。
7. 有架构、依赖、协议、安全或数据决策，就写 ADR。
8. 完成后必须回写追踪表与 Completion Notes。
9. 不能验证时必须说明原因、替代检查与剩余风险。
10. 项目专属路径、命令、工具链永远由项目适配层决定，不写进通用规范。

本规范的目的不是增加流程负担，而是让 AI 辅助开发从「凭上下文猜测」变成「按契约执行、按证据交付、按记录演进」。

---

## 22. Installation Paths by Agent

> S2V skill 在不同 agent 工具的默认安装路径速查。`/s2v-init` 步 0 的 `_s2v_skill_dir` resolver 按以下优先级解析实际路径，所有跨命令引用全局 skill 资源时统一通过 `${S2V_SKILL_DIR}` 变量访问（不再硬编码 Claude Code 路径）。

### 22.1 解析优先级（resolver 三层 fallback）

| Layer | 来源 | 检测方式 | 优先级 |
|---|---|---|---|
| 1 | `$S2V_SKILL_DIR` 环境变量 | 用户 / CI 显式声明 | **最高** |
| 2 | Agent runtime 注入变量 | `$CLAUDE_SKILL_DIR` / `$SKILL_ROOT` / `$AGENT_SKILL_PATH` | 中 |
| 3 | 已知 agent 工具默认路径 | 按下方表格顺序探测第一个含 `full-standard.md` 的目录 | fallback |

### 22.2 已知 agent 工具默认路径

| Agent | 默认路径 | 备注 |
|---|---|---|
| **Claude Code** | `~/.claude/skills/s2v` | 主要开发 / 测试目标平台（已验证）|
| **Codex CLI** | `~/.codex/skills/s2v` | OpenAI skill-installer 协议（`$CODEX_HOME` 未设置时的默认；已验证）|
| **Cursor** | `~/.cursor/skills/s2v` | Cursor support 官方 native path（已验证）|
| **多 agent 通用** | `~/.agents/skills/s2v` | Cursor / 其他工具共享的 skills 父目录约定（兜底探测路径）|
| **Aider** | — | **无官方 skill 目录协议** — 用 `aider --read full-standard.md` 或 `.aider.conf.yml` 的 `read:` 字段加载规范（不在 Layer 3 探测列表）|

> ⚠️ 探测顺序固定为表中行序（Claude → Codex → Cursor → 多 agent 通用 `~/.agents/skills/s2v`）；第一个含 `full-standard.md` 的目录即返回。Aider 无 skill 目录协议，不参与 Layer 3 探测。如需调整，改 **init.md 步 0 + tier.md 步 0** `_s2v_skill_dir()` Layer 3 循环列表（两处同款 inline 克隆，必须同步 — 见 §22.6）。

### 22.3 环境变量覆盖（CI / 企业部署 / 临时调试）

```bash
export S2V_SKILL_DIR=/path/to/s2v   # 必须含 full-standard.md
/s2v-init                            # 使用显式声明路径，跳过 Layer 2/3 探测
```

### 22.4 非 Claude 用户首次安装

二选一：
- **方案 A**：把 skill 文件夹放到 §22.2 任一默认路径，无需配置 — **仅适用于 Claude Code / Codex CLI / Cursor 三个已验证默认路径**
- **方案 B**：放到任意路径 + 设 `S2V_SKILL_DIR` 环境变量指向 skill 根目录（含 `full-standard.md` 的目录）

> ⚠️ **Aider 用户**：Aider 无官方 skill 目录协议，方案 A 不适用 — 必须用方案 B（`S2V_SKILL_DIR`），或用 `aider --read full-standard.md` / `.aider.conf.yml` 的 `read:` 字段直接加载规范。

> 🔧 **便捷安装（可选）**：`bash scripts/install.sh <target-dir>` 自动化上述方案 A/B —— 忠实拷贝 skill（仅排除 `.git`）+ 校验 `full-standard.md` + 跑 helper self-test + 打印 `S2V_SKILL_DIR` 指引。脚本**刻意不内置 §22.2 路径表**（路径发现仍由运行时 resolver 负责），故**不构成 §22.6 两处 inline resolver 之外的第 3 个路径同步点**；手工 A/B 仍随时可用，本脚本仅为降低首次安装摩擦的可选件。

> 🧩 **`/s2v-*` 跨 agent 命令（两层）**：① 普适层 —— `SKILL.md` 与项目 `AGENTS.md`（由 `templates/agents-*.md` 渲染）内建「命令派发」节，任何读到它的 agent 键入 `/s2v-<x>` 或自然语言要求该子流程即确定性执行对应文档（Aider 经 `--read AGENTS.md`）；无可移植命令标准，这是真正的跨 agent 保证、零配置。② 可选原生 UX 层 —— `bash scripts/install.sh --commands <claude|codex|cursor|opencode|aider>` 把 `commands/<agent>/` 薄委托桩装到该 agent 命令目录，获得原生 `/` 自动补全。桩运行时仍按本章 §22 解析 skill，**不内置 §22.2 路径表**，不构成 §22.6 两处 inline resolver 之外的同步点。

### 22.5 Resolver 实现位置

- **权威规范源点**：`init.md` 步 0 `_s2v_skill_dir()` 函数（resolver 的规范定义；任何逻辑变更以此为准）
- **inline 克隆点**：`tier.md` 步 0 内联一份**逐字等价**的同款 resolver（`/s2v-tier` 是 post-init 命令，但仍需独立 bootstrap，不依赖 `/s2v-init` 缓存）— init.md / tier.md 两处必须保持同步（见 §22.6 维护约束）
- **必须 inline**，不能 source 自全局 skill — 否则陷入 "需要全局 skill 路径才能找到全局 skill 路径" 的 bootstrap 死循环
- 解析结果 `export S2V_SKILL_DIR=<resolved-path>` 后供 init.md / tier.md 全步 + 项目内 `docs/s2v/scripts/lib/*.sh` 间接复用
- 项目内（`docs/s2v/scripts/`）的 helper 不依赖此 resolver — 它们用项目内相对路径

### 22.6 维护约束

- 新增 agent 工具默认路径 → 改 `_s2v_skill_dir()` Layer 3 循环（**init.md 步 0 + tier.md 步 0 两处同步**）+ §22.2 表格同步
- 改路径解析逻辑 / 改优先级 / 改错误信息 → 必须保持 inline，且 **init.md 步 0 与 tier.md 步 0 两份克隆逐字同步**（不可外移到 scripts/lib/ — bootstrap 死循环约束）
- 跨命令引用全局 skill 资源 → 一律用 `${S2V_SKILL_DIR}/...`，不硬编码 `~/.claude/skills/s2v/...`（仅 §22.2 表格 + resolver 错误信息中可作为示范默认值）
