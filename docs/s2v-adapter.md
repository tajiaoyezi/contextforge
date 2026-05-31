# Project Development Adapter

> S2V Development 项目适配层。AI agent 进入项目的第一份必读文件。
> 一旦项目结构、命令、约束发生变化，立即更新本文件（s2v §4.4）。
>
> 本模板由 `/s2v-init` 生成。所有 `<占位符>` 必须被替换为项目真实值。
>
> 与 AGENTS.md 关系（如生成）：本文件定义"项目结构与命令规范"（路径 / 命令 / 测试 / coverage），AGENTS.md 定义"协作行为约束"（worktree / commit / 卡住协议）。两者均必读，加载顺序：AGENTS.md（协作）→ 本文件（结构）→ task spec（业务）。

---

## Project

- **Name**: `ContextForge`
- **Type**: `Infrastructure` <!-- 本地优先的 AI Agent Context / MemoryOps 基础设施：Local CLI + Daemon/API + MCP/Agent Adapter + Rust 检索核心 -->
- **Primary users / actors**: 多 Agent 重度个人/独立开发者 · 3-8 人小型 AI 工具链团队 · 本地优先/隐私敏感开发者
- **Critical workflows**: ① init 本地配置/数据目录 ② import/index 接入代码·文档·日志·Agent memory ③ search 可解释检索（CLI/REST/MCP）④ export/migrate 跨 Agent 上下文迁移

---

## Specification Locations

- **SDD home**: `docs/specs/`
- **Master spec**: `docs/prds/context-forge.prd.md`
- **Phase spec pattern**: `docs/specs/phases/phase-{N}-{name}.md`
- **Task spec pattern**: `docs/specs/tasks/task-{phase}.{seq}-{name}.md`
- **BDD acceptance home**: `test/features/*.feature`
- **ADR home**: `docs/decisions/adr-{N}-{title}.md`

---

## Source And Test Areas

> **路径 list 格式**：所有四类区域使用 **markdown bullet list，每行一个 git pathspec**。下游 `/s2v-implement` 把整个 list 读出后展开为 `git add` 多参数（无需外层引号 / 无需空格分隔）。
>
> **强约束（Source areas / Unit test areas）**：`/s2v-implement` 步 6/7 RED/GREEN 直接当 git pathspec 用 → **禁 `<...>` 占位 + 禁 `N/A`**（占位会触发 `git add` fatal）。
> **弱约束（Integration test areas / E2E test areas）**：当前 `/s2v-implement` / helper **不直接消费** → **允许 `N/A: <原因>`** 或保留 `<...>` 占位（项目无 integration / e2e 测试时合法跳过）；未来引入 integration / e2e 自动化时升级为强约束。

### Source areas

- `cmd/contextforge/`
- `internal/`
- `core/`
- `proto/`
- `go.mod`
- `Cargo.toml`

### Unit test areas

- `cmd/contextforge/`
- `internal/`
- `core/src/`
- `core/tests/`

### Integration test areas

- `<INTEGRATION_TEST_AREAS>` <!-- 弱约束：如 test/integration/ ；无 integration 测试时填 N/A: 无 integration 测试 -->

### E2E test areas

- `<E2E_TEST_AREAS>` <!-- 弱约束：如 test/<scenario>.e2e.test.ts ；无 e2e 测试时填 N/A: 无 e2e 测试 -->

### Other locations

- **BDD feature**: `test/features/*.feature`（与对应 test 文件同名，仅扩展名 `.feature` vs `.test.<ext>`）
- **Fixture areas**: 见下方 §Fixture 约定

### Test File Naming（本项目覆盖 — Go + Rust 双语言）

> S2V 通用规范不强制测试命名。本项目为 Go 控制面 + Rust 数据面双二进制，按各语言习惯覆盖如下：

| 测试类型 | 文件名 | 示例 |
|---|---|---|
| Go 单元测试 | `<module>_test.go` 同包 | `internal/config/loader_test.go` 对应 `internal/config/loader.go` |
| Go 集成测试 | `e2e/<scenario>_test.go` 或 `internal/<m>/<m>_integration_test.go` | `cmd/contextforge/init_integration_test.go` |
| Rust 单元测试 | `#[cfg(test)] mod tests` 同源文件 | `core/src/scanner/mod.rs` 内嵌 `mod tests` |
| Rust 集成测试 | `core/tests/<scenario>.rs` | `core/tests/index_roundtrip.rs` |
| BDD feature | `<module>.feature` | `scanner.feature` |

**默认建议**：保持 Go `_test.go` 同包、Rust `mod tests` 同源 / `core/tests/` 集成的一致性，避免命名漂移。

### Fixture 约定（避免多 agent drift）

| Fixture 大小 / 用途 | 落地位置 | 示例 |
|---|---|---|
| 小 (<20 行) | inline（Go: 字面量 / Rust: `&str` 常量）in test | `let md = "# Title";` |
| 中 (20-100 行) | `test/fixtures/<module>/<case-name>.<ext>` | `test/fixtures/scanner/with-secret.env` |
| 大 (>100 行 / 二进制 / 跨 task 复用) | `test/fixtures/shared/<purpose>.<ext>` | `test/fixtures/shared/golden-openclaw-workspace/` |

**约束**：
- 含 unicode / 特殊字符的 fixture 一律走文件，禁止 inline（diff 噪音 + 编码风险）
- 跨 task 复用 → 必须放 `test/fixtures/shared/` + 在两个 task spec §3 都引用
- fixture 文件名规则：kebab-case + 描述性（**不**写 `case1.<ext>`）
- secret/redaction fixture 含**伪造**凭证样本，禁用真实 key（见 PRD §Constraints 安全基线）

### TEST-ID 落地约定（本项目覆盖 — Go + Rust）

task spec §7 追踪表写 `TEST-X.Y.Z` 等编号，对应代码层落地建议：

```text
Go:   func TestXYZ(t *testing.T) { t.Run("TEST-X.Y.Z: <描述>", func(t *testing.T){...}) }
Rust: #[test] fn test_x_y_z() { /* TEST-X.Y.Z / SCEN-X.Y.Z / AC<N> */ ... }
```

**约定**：
- Go `t.Run` 子测试名 / Rust `#[test]` 函数名上方注释**含 `TEST-X.Y.Z:`**（可 grep 精确匹配）
- 描述上方一行写 `// SCEN-X.Y.Z / AC<N>` 注释（标记追踪表锚点）
- TEST-ID 必须能被 grep 精确匹配 → 配合追踪表实现"声明 → 落地 → 跑过"三段验证

---

## Commands

> 所有命令在项目根目录或对应 worktree 根运行。
>
> **字段语义**（/s2v-implement 与 AGENTS 模板的 helper 按此判读 — `s2v_load_cmd` 取字段后整行字面量交 `s2v_run` 判读）：
> - 真实命令 → **直接填裸值**（如 `pnpm lint`）；**不要加反引号、不要加行尾 `<!-- -->` 注释**
> - 项目暂时不适用 → 写字面量 `N/A: <原因>`
> - **不要留空**
> - 未替换的 `<...>` 占位（**裸形式**，无反引号）→ verify.sh 干净 hard-fail
> - **Unit Test 强制**：§9 Verification 不接受 `N/A` / 留空

- **Install**: go mod download && cargo fetch
- **Lint**: <LINT_COMMANDS>
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit Test**: go test ./... && cargo test --workspace
- **Integration tests**: <INTEGRATION_TEST_COMMANDS>
- **E2E tests**: <E2E_TEST_COMMANDS>
- **Build**: <BUILD_COMMANDS>
- **Coverage**: <COVERAGE_COMMANDS>
- **Runtime smoke**: <RUNTIME_SMOKE_COMMANDS>

<!-- 字段顺序与 s2v_extract_verify_keys 固定执行序一致：install → lint → typecheck → unit-test → integration → e2e → build → coverage → runtime-smoke → manual。Build 在 Coverage 前 -->

> ⚠️ **字段名必须加粗**（`- **Field**:` 形式）且大小写敏感 — `s2v_load_cmd` helper 的 awk 正则按 `^- \*\*Field\*\*:` 匹配。
>
> **§Commands 不收录 Release / Deploy / Manual** — 发布元数据填 `## Constraints` 的 Release constraints；Manual 由 task §9 自由文本列出。
>
> **多工具链应对（Go + Rust）**：本表每类只有一个全局槽位，`s2v_run` 对该槽位整行 `eval`。本项目 Install/Typecheck/Unit Test 已用 `&&` 串联 Go + Rust 两条命令（任一非零即整体失败，不吞错）。后续若需 per-module 命令矩阵，按 init.md "聚合脚本"推荐模式（`bash tools/test-all.sh`）替换槽位。

### Coverage 判读规则（本项目覆盖 — Go + Rust）

- Go: `go test -cover ./...` 输出 `coverage: X.X% of statements` → 判读该百分比
- Rust: `cargo tarpaulin` 输出 `Coverage Results: X/Y (Z%)` → 判读 Z%
- task spec 写 "≥80%" 时，Go/Rust 分别对照各自百分比；聚合阈值在 §9 注明按哪侧为准

### Coverage 未达标处理（主 agent / subagent 行为约束）

| 实测 vs 阈值 | 应当 | 禁止 |
|---|---|---|
| ≥ 阈值 | ✅ 直接通过 | — |
| 差距 ≤ 2 行 | 检查 Uncovered → 补**真实** TEST-X.Y.Z | ❌ 凑数断言；❌ `// nolint`/`#[cfg(not(test))]` 跳过 |
| 差距 > 2 行 / 路径无法测试 | 走 §卡住协议（AGENTS.md §8）→ 主 agent 决策 | ❌ subagent 自行修改 task spec 阈值（违反 R6）|

---

## Constraints

- **Runtime target**: `<RUNTIME_TARGET>` <!-- PRD §Constraints 运行时：Go toolchain (建议 Go 1.22+) + Rust stable (建议 1.75+, cargo)；双二进制 contextforge / contextforge-core；无 JVM/Node；CPU-only 须可完成基础索引与检索 -->
- **Supported platforms**: `<SUPPORTED_PLATFORMS>` <!-- PRD §Constraints 平台：v0.1 P0 = Linux x86_64 (Ubuntu 22.04/24.04/26.04 / WSL2)；macOS arm64/x86_64 源码构建 nice-to-have；Windows v0.3 preview。注：PR#15 后 Windows native 测试套可跑通（go vet + go test ./internal/... 全绿），仅作 nice-to-have 开发者体验改进；P0 release gate 仍按 PRD = Linux/WSL2，0600/0700 安全基线在 Linux/WSL2 硬断言、Windows ACL 等价仍待 Phase 8 / v0.3 -->
- **Security requirements**: `<SECURITY_REQUIREMENTS>` <!-- PRD §Constraints 安全 + Local service security baseline：默认本地不上传 / 远程 provider 显式 opt-in / denylist + secret redaction / daemon 默认 127.0.0.1 或 unix socket、禁 0.0.0.0 / REST 本地随机 token (0600) / MCP client allowlist / audit log 脱敏 -->
- **Performance requirements**: `<PERFORMANCE_REQUIREMENTS>` <!-- PRD §Constraints 性能：10 万 chunk BM25/metadata/filter P95 <500ms（不含 embedding/reranker/远程）/ 1 万文件索引 <10min / 单文件增量 <5s / daemon idle <300MB -->
- **Compatibility requirements**: `<COMPATIBILITY_REQUIREMENTS>` <!-- PRD §Constraints 兼容性：只读导入 + 导出 draft/bundle 不写回；P0 导入源见 PRD；OpenClaw/Cursor/Zed schema、MCP 版本 TBD -->
- **Release constraints**: `<RELEASE_CONSTRAINTS>` <!-- PRD §Constraints 发布：v0.1 GitHub Release Linux x86_64 tarball + 源码 self-host + Docker Compose；回退上一 release tag -->

---

## Workflow

- **Collaboration Tier**: `team`
  <!-- 必填值：solo | team。决定 git 协作严格度。详见 s2v full-standard.md §4.5 -->
  <!-- 重要：Tier 仅影响 git 协作层（branch / PR / worktree / merge gate），
       不影响 S2V 核心（SDD / BDD / TDD / §2.5 三段 commit / ADR / Verification / 追踪表 / 卡住协议）—— 所有 tier 必守 -->
  Overrides:
    - **adr-014-cross-validation-gate**（2026-05-24，[ADR-014](decisions/adr-014-cross-phase-exit-criteria-validation.md) Accepted 后生效）：
      Phase 10 起新 phase / task spec PR 适用 ADR-014 D1-D5 制度——
      D1 closeout PR body 含 phase §6 ↔ task §6 AC mapping 表 + D2 lint 输出；
      D2 跑 `bash scripts/spec_drift_lint.sh --touched origin/master`（PR 增量模式），
      未标注 anti-pattern 命中须加 `[SPEC-DEFER:<name>]` 或 `[SPEC-OWNER:<task>]`；
      D3 phase spec §6 每条 AC 必须显式 `verified by ...` owner；
      D4 缺 D1/D2 输出 → 主 agent 不自决合，降级用户审或转 §8 STOP；
      D5 Phase 1-9 历史不溯改。详 AGENTS.md §3.4.4 / §4 Gate 4.5。
    - **phase23-start-gate = contract-frozen**（2026-05-17，主 agent + 用户签字）：
      AGENTS §1 worktree 表字面写 Phase 2/3 启动门槛 = "等 Phase 1 merge"。
      本 override 将其**重释为"Phase 1 契约已冻结并 merged"**即可启动 Phase 2/3 ——
      判据 = task-1.1(proto 冻结)/1.2(config)/1.3(core-skeleton) 均已 merge 到 master
      （PR #1/#2/#3）。理由：Phase 2(scanner)/Phase 3(importer) 实质只依赖 task-1.1
      冻结的 canonical-record/gRPC proto + task-1.2 denylist/allowlist，**不消费**
      task-1.4 `contextforge init`；2.1(Rust)/2.2(Rust)/3.1(Go) 写路径互不相交。
      **硬约束**：早启动的 Phase 2/3 task **只读消费**冻结契约，**禁止修改**
      `proto/contextforge/v1/*`；若实施中发现确需改 proto/config 契约 → subagent 立即 STOP
      → return spec-drift 对象给主 agent，主 agent 串行化处理（proto 仅 add-only，
      影响有界）。task-1.4 仍照常走 AGENTS §4 Gate 3 phase-1 §6 端到端 smoke，
      Phase 1 仍按正常流程正式收口（本 override 不豁免 §4 任何 gate）。

### Agent Topology（单驱动 + 内部 subagent 自治，2026-05-23 起；前身 Agent Roster 见 ADR-011）

本项目治理拓扑（[ADR-011](decisions/adr-011-single-driver-with-subagents.md) / [ADR-012](decisions/adr-012-main-agent-governance-autonomy.md) 决策）：

- **唯一驱动**：主 agent（Claude Code 单 session）在主 repo `ContextForge/` 协调 + 实施
- **subagent 调度**：主 agent 用 **Agent tool** spawn 内部子 agent 完成需隔离 context / 并行执行 / 角色专精的子任务；`subagent_type` 按任务选：
  - `Explore` — 只读探索 / 定位文件 / 跨多目录 grep
  - `Plan` — 实施前设计 + 验证方案 + 边界讨论
  - `general-purpose` — 通用多步研究 / 搜索（适合不确定 scope 的任务）
  - `code-reviewer` / `code-simplifier` / 项目自定义 agent type — 角色化任务
  - `claude` — 默认 catch-all（不确定时用）
- **worktree 隔离**：需写隔离时用 Agent tool `isolation: "worktree"` 参数 — 自动建 `../ContextForge-wt-task-<X.Y>` + `feat/task-<X.Y>-<name>` 分支；subagent 完成后主 agent 收回 worktree
- **长任务自治**：主 agent 用 Claude Code `/goal <condition>` 让自身跨多轮工作至完成条件满足 — 完整规范见 [AGENTS.md §3.5](../AGENTS.md) / [ADR-011](decisions/adr-011-single-driver-with-subagents.md)
- **治理自治**：主 agent 对 §2A Ready review / R6 merge decision / R7 dep chore PR / §8 Waive 可按 ADR-012 自决；R3/R6 物理保险、subagent lockfile 禁写、`BLOCKED-branch-mismatch.md` 留痕不放松。

#### Review subagent 协议（主 agent 内部，2026-05-22 起延用）

- 主 agent 用 Agent tool spawn 子 agent 完成 PR 评审，PR 复杂度 / 并行需要决定 subagent 数量（简单 PR 1 个；多模块 PR 多维度可 2–3 个并行；多 PR 同时评可 N 个一对一）
- subagent 跑 review → return 结构化结论给主 agent → 主 agent 直接评判 + 决策（merge / 自做小修 / 继续打磨）
- 引用 prompt template：`_dispatch/reviewer__per-PR.md`
- **硬约束：review subagent 不得再 spawn 子 subagent** — 必须直接亲自评审（亲自跑 temp clone verify + 读 spec + 写 review object），嵌套 spawn 会失控且信息二手转述损失。该硬约束写在 `_dispatch/reviewer__per-PR.md` 第 28-29 行（"角色"段尾），与本处单一源

#### 与既有协议的关系

- 所有 subagent 工作产出仍走 R6 PR-only + AGENTS §4 PR 合入流程
- Gate 0-5 全绿后的 merge 决策由主 agent 按 ADR-012 自决，不再要求额外用户确认
- subagent 实施结果 / 卡住 / 需新 dep / 发现 spec drift → 通过 **return 结构化对象** 给主 agent（旧 worker 终端模式下的 `NEEDS-DEP-task-X.Y.md` / `BLOCKED-task-X.Y.md` / `READY-FOR-MERGE-task-X.Y.md` / `SPEC-DRIFT-task-X.Y.md` 文件载体已退役）
- review subagent 调用是主 agent context 内行为，**不落盘**（Agent tool log 已审计）
- subagent 不得自走：**主 agent → subagent** 单一决策链；subagent 完成 / 卡住后 return 即结束，由主 agent 决定下一步

---

## Phase 状态索引

> 与 Master Spec §Implementation Phases 同步。开始一个 phase 时更新此处。
>
> **Status 取值**：与 spec 顶部 Status 共用 standard.md §10.5.1 状态机 — 合法值 `Draft / Ready / In Progress / Done / Blocked / Waived`。

| # | Phase | Phase Spec | Status | Tasks | Worktree（仅 team）|
|---|---|---|---|---|---|
| 1 | `foundation` | `docs/specs/phases/phase-1-foundation.md` | Done | 4 | `../ContextForge-wt-foundation` |
| 2 | `index-core` | `docs/specs/phases/phase-2-index-core.md` | Done | 4 | `../ContextForge-wt-index-core` |
| 3 | `agent-importers` | `docs/specs/phases/phase-3-agent-importers.md` | Done | 4 | `../ContextForge-wt-agent-importers` |
| 4 | `retrieval-explain` | `docs/specs/phases/phase-4-retrieval-explain.md` | Done | 2 | `../ContextForge-wt-retrieval-explain` |
| 5 | `memoryops` | `docs/specs/phases/phase-5-memoryops.md` | Done | 3 | `../ContextForge-wt-memoryops` |
| 6 | `cli-api-export` | `docs/specs/phases/phase-6-cli-api-export.md` | Done | 3 | `../ContextForge-wt-cli-api-export` |
| 7 | `mcp-adapter` | `docs/specs/phases/phase-7-mcp-adapter.md` | Done | 1 | `../ContextForge-wt-mcp-adapter` |
| 8 | `eval-and-reliability` | `docs/specs/phases/phase-8-eval-and-reliability.md` | Done | 3 | `../ContextForge-wt-eval-and-reliability` |
| 9 | `cli-pipeline` | `docs/specs/phases/phase-9-cli-pipeline.md` | Done | 6 | `../ContextForge-wt-cli-pipeline` |
| 10 | `console-contract-v1` | `docs/specs/phases/phase-10-console-contract-v1.md` | Done | 6 | `../ContextForge-wt-console-contract-v1` |
| 11 | `console-real-data-plane` | `docs/specs/phases/phase-11-console-real-data-plane.md` | Done | 4 | `../ContextForge-wt-console-real-data-plane` |
| 12 | `console-contract-completion` | `docs/specs/phases/phase-12-console-contract-completion.md` | Done | 3 | `../ContextForge-wt-console-contract-completion` |
| 13 | `memory-rest-surface` | `docs/specs/phases/phase-13-memory-rest-surface.md` | Done | 2 | `../ContextForge-wt-memory-rest-surface` |
| 14 | `eval-rest-surface` | `docs/specs/phases/phase-14-eval-rest-surface.md` | Done | 2 | `../ContextForge-wt-eval-rest-surface` |
| 15 | `console-functional-gap-closure` | `docs/specs/phases/phase-15-console-functional-gap-closure.md` | Done | 6 | `../ContextForge-wt-console-functional-gap-closure` |
| 16 | `v0.9.0-backlog-completion` | `docs/specs/phases/phase-16-v0.9.0-backlog-completion.md` | Done | 4 | `../ContextForge-wt-v0.9.0-backlog-completion` |
| 17 | `is-pinned-amendment` | `docs/specs/phases/phase-17-is-pinned-amendment.md` | Done | 1 | `../ContextForge-wt-is-pinned-amendment` |
| 18 | `vector-backend-selection` | `docs/specs/phases/phase-18-vector-backend-selection.md` | Done | 9 | `../ContextForge-wt-vector-backend-selection`（v0.11.0 closeout 缩范围：AC1/2/5/6 met；AC3 partial=ADR-023 Proposed；AC4 deferred=生产集成 [SPEC-OWNER:phase-future.vector-retrieval-integration]）。**→ Phase 19 (v0.12.0) 解除**：AC3 ADR-023 ratify Accepted（task-19.6）+ AC4 生产集成 live（task-19.2/19.3/19.5）；记录于 ADR-023 Amendment，Phase 18 spec 正文未溯改（D5） |
| 19 | `vector-retrieval-integration` | `docs/specs/phases/phase-19-vector-retrieval-integration.md` | Done | 7 | master（v0.12.0：端到端语义检索 live + ADR-023 ratify Accepted。解 Phase 18 AC3/AC4 + [SPEC-OWNER:phase-future.vector-retrieval-integration] **已解除**；real recall @10=0.9333≥0.70（task-19.5），默认构建 0-dep deterministic+brute-force 语义路径，real fastembed provider feature-gated） |
| 20 | `semantic-retrieval-throughline` | `docs/specs/phases/phase-20-semantic-retrieval-throughline.md` | Done | 3 | master（v0.13.0：console-api `?semantic=true` 经 console_data_plane gRPC 语义分派真生效（task-20.1，闭 console/core proto 分离 drift）+ 真实召回经生产 Retriever 热路径（task-20.2 real fastembed @5=0.9667/@10=1.0/top1=0.7333/MRR=0.8367 gate PASS）+ smoke v10（task-20.3）。闭合 v0.12.0 evidence §3b 两条 caveat；ADR-024 Accepted。tag 待用户授权后 push） |
| 21 | `retrieval-quality` | `docs/specs/phases/phase-21-retrieval-quality.md` | Done | 3 | master（v0.14.0：hybrid scoring（RRF k=60 BM25+向量融合，task-21.1）+ reranker（Reranker trait + 确定性 IdentityReranker + feature-gated CrossEncoderReranker，task-21.2）+ eval/smoke/release 收口（task-21.3）。真实 dogfood eval：hybrid top-1 0.0333→0.6667 / MRR 0.4095→0.7881 vs BM25 baseline → ADR-025 Accepted；real cross-encoder run（D5 未触发，top-1/MRR vs baseline uplift + 最高 recall@5，诚实 caveat：本小型代码语料不及 hybrid）→ ADR-026 Accepted。默认构建 0 新 dep、BM25 baseline 不变；tag 待用户授权后 push） |
| 22 | `embedding-provider-completion` | `docs/specs/phases/phase-22-embedding-provider-completion.md` | Done | 4 | master（v0.15.0：provider 配置选择 + dim 协商（`select_provider` 工厂 + `negotiate_dim`→`DimMismatch`，task-22.1）+ content-hash 缓存（`CachingEmbeddingProvider` 内存 L1 + 可选 SQLite L2，task-22.2）+ 远程 OpenAI/Cohere HTTP 骨架（`RemoteEmbeddingProvider` ureq rustls feature-gated + 契约测试不打网络，task-22.3）+ health opt-in 远程探针 + smoke v12 + 收口（task-22.4）。ADR-027 据 D1-D5 真实非合成验证（Go config round-trip + Rust factory/dim/cache 单测 + 远程契约 fixture + 默认 0 网络 dep）Proposed→Accepted；默认构建 0 模型 / 0 网络 dep（deterministic 缺省，fastembed/remote feature-gated + opt-in，ADR-004）；远程真实联调/密钥/召回质量 + 远程探针真实命中如实 defer（ADR-013）；tag 无人值守授权下主 agent 自主 push） |
| 23 | `vector-persistence-and-cross-platform` | `docs/specs/phases/phase-23-vector-persistence-and-cross-platform.md` | Done | 3 | master（v0.16.0：hnsw 图持久化往返（路径 B 输入集 serialize + load 重建 + rebuild-on-load，task-23.1，3/3 PASS）+ sqlite-vec Windows MSVC 跨平台（task-23.2 真实在 x86_64-pc-windows-msvc 构建+运行通过，**解除 Phase 18 MSVC-blocked stop-condition**，0 源码改动）+ 向量增量索引评估（brute-force/sqlite-vec 行级追加，hnsw 增量延后，task-23.3）+ smoke v13 + 收口。ADR-028 据 D1-D4 真实非合成验证 Proposed→Accepted；ADR-023 add-only Amendment 推进 Follow-ups（rebuild-on-restart 前提解除 / MSVC parity 缩小，不溯改正文 D5）；默认构建 0-vector-dep BM25 baseline 不变（ADR-023 D5）；tag 无人值守授权下主 agent 自主 push） |
| 24 | `retrieval-tokenizer-and-eval-hardening` | `docs/specs/phases/phase-24-retrieval-tokenizer-and-eval-hardening.md` | Done | 3 | master（v0.17.0：opt-in code/CJK tokenizer（`CodeCjkTokenizer` 纯 std 代码符号拆分 + 保留原 token + CJK bigram，默认 tokenization 不变，task-24.1 #173）+ eval 数据集校验器 `ValidateGoldenSemantic` + golden-semantic.jsonl 代码/CJK 扩充（task-24.2 #174）+ 真实 before/after recall delta 0.9091→1.0000 (+0.0909) over task-24.2 golden + rust-native-eval-runner 诚实延后 + smoke v14 + 收口（task-24.3）。ADR-029 据 D1-D3/D5 真实非合成验证 Proposed→Accepted（D4 runner 真实评估诚实延后）；默认构建 0 新 dep + 默认 tokenization + eval gate 阈值不变（ADR-004/006，无 ADR-006/008 Amendment）；opt-in 经 config 须 re-index；tag 无人值守授权下主 agent 自主 push） |
| 25 | `production-vector-backend` | `docs/specs/phases/phase-25-production-vector-backend.md` | Draft | 0 | 规划 Draft（roadmap §3.6 / v0.18.0；3 task spec Draft）；实现期逐 task 建 `../ContextForge-wt-task-25.x` |
| 26 | `observability-hardening` | `docs/specs/phases/phase-26-observability-hardening.md` | Draft | 0 | 规划 Draft（roadmap §3.7 / v0.19.0；3 task spec Draft）；实现期逐 task 建 `../ContextForge-wt-task-26.x` |
| 27 | `memory-ops-hardening` | `docs/specs/phases/phase-27-memory-ops-hardening.md` | Draft | 0 | 规划 Draft（roadmap §3.8 / v0.20.0；3 task spec Draft）；实现期逐 task 建 `../ContextForge-wt-task-27.x` |

> 该索引由 `/s2v-add phase <name>` 自动追加；手动修改时保持一致。

## Task 总索引

> 全部 task spec 应通过 `/s2v-add task <name>` 创建；agent 进 worktree 后**禁止自创 task spec**（违反 s2v R5 单一事实源）。
>
> **Status 取值**：同 §Phase 状态索引（standard.md §10.5.1：`Draft / Ready / In Progress / Done / Blocked / Waived`）。

| Task | 模块 | Spec 文件 | Status | 依赖 / Phase 内顺序 | Worktree（仅 team）|
|---|---|---|---|---|---|
| 1.1 | proto | docs/specs/tasks/task-1.1-proto.md | Done | Phase1 #1 | `../ContextForge-wt-foundation` |
| 1.2 | config | docs/specs/tasks/task-1.2-config.md | Done | Phase1 #2（dep 1.1）| `../ContextForge-wt-foundation` |
| 1.3 | core | docs/specs/tasks/task-1.3-core-skeleton.md | Done | Phase1 #3（dep 1.1）| `../ContextForge-wt-foundation` |
| 1.4 | cli | docs/specs/tasks/task-1.4-cli-init.md | Done | Phase1 #4（dep 1.1,1.2,1.3）| `../ContextForge-wt-foundation` |
| 2.1 | scanner | docs/specs/tasks/task-2.1-scanner.md | Done | Phase2 #1 | `../ContextForge-wt-index-core` |
| 2.2 | parser | docs/specs/tasks/task-2.2-parser.md | Done | Phase2 #2 | `../ContextForge-wt-index-core` |
| 2.3 | chunker | docs/specs/tasks/task-2.3-chunker.md | Done | Phase2 #3（dep 2.2）| `../ContextForge-wt-index-core` |
| 2.4 | indexer | docs/specs/tasks/task-2.4-indexer.md | Done | Phase2 #4（dep 2.1,2.3）| `../ContextForge-wt-index-core` |
| 3.1 | importer | docs/specs/tasks/task-3.1-importer-core.md | Done | Phase3 #1 | `../ContextForge-wt-agent-importers` |
| 3.2 | importer | docs/specs/tasks/task-3.2-importer-hermes.md | Done | Phase3 #2（dep 3.1）| `../ContextForge-wt-agent-importers` |
| 3.3 | importer | docs/specs/tasks/task-3.3-importer-openclaw.md | Done | Phase3 #3（dep 3.1）| `../ContextForge-wt-agent-importers` |
| 3.4 | importer | docs/specs/tasks/task-3.4-importer-agent-rules.md | Done | Phase3 #4（dep 3.1）| `../ContextForge-wt-agent-importers` |
| 4.1 | retriever | docs/specs/tasks/task-4.1-retriever.md | Done | Phase4 #1 | `../ContextForge-wt-retrieval-explain` |
| 4.2 | retriever | docs/specs/tasks/task-4.2-explain.md | Done | Phase4 #2（dep 4.1）| `../ContextForge-wt-retrieval-explain` |
| 5.1 | memoryops | docs/specs/tasks/task-5.1-dedup.md | Done | Phase5 #1 | `../ContextForge-wt-memoryops` |
| 5.2 | memoryops | docs/specs/tasks/task-5.2-lifecycle.md | Done | Phase5 #2（dep 5.1）| `../ContextForge-wt-memoryops` |
| 5.3 | memoryops | docs/specs/tasks/task-5.3-audit.md | Done | Phase5 #3（dep 5.1）| `../ContextForge-wt-memoryops` |
| 6.1 | cli | docs/specs/tasks/task-6.1-cli-search.md | Done | Phase6 #1 | `../ContextForge-wt-cli-api-export` |
| 6.2 | daemon | docs/specs/tasks/task-6.2-rest-api.md | Done | Phase6 #2（dep 6.1）| `../ContextForge-wt-cli-api-export` |
| 6.3 | exporter | docs/specs/tasks/task-6.3-exporter.md | Done | Phase6 #3（dep 6.1）| `../ContextForge-wt-cli-api-export` |
| 7.1 | mcp-adapter | docs/specs/tasks/task-7.1-mcp-server.md | Done | Phase7 #1 | `../ContextForge-wt-mcp-adapter` |
| 8.1 | eval | docs/specs/tasks/task-8.1-eval-harness.md | Done | Phase8 #1 | `../ContextForge-wt-eval-and-reliability` |
| 8.2 | reliability | docs/specs/tasks/task-8.2-reliability.md | Done | Phase8 #2 | `../ContextForge-wt-eval-and-reliability` |
| 8.3 | release | docs/specs/tasks/task-8.3-release-smoke.md | Done | Phase8 #3（dep 8.1,8.2）| `../ContextForge-wt-eval-and-reliability` |
| 9.1 | proto | docs/specs/tasks/task-9.1-proto-index-rpc.md | Done | Phase9 #1 | `../ContextForge-wt-cli-pipeline` |
| 9.2 | core/server | docs/specs/tasks/task-9.2-rust-grpc-index.md | Done | Phase9 #2（dep 9.1）| `../ContextForge-wt-cli-pipeline` |
| 9.3 | cli/index | docs/specs/tasks/task-9.3-go-cli-index.md | Done | Phase9 #3（dep 9.2）| `../ContextForge-wt-cli-pipeline` |
| 9.4 | cli/import | docs/specs/tasks/task-9.4-go-cli-import.md | Done | Phase9 #4（dep 9.2，可 ∥ 9.3）| `../ContextForge-wt-cli-pipeline` |
| 9.5 | release | docs/specs/tasks/task-9.5-release-smoke-real.md | Done | Phase9 #5（dep 9.3,9.4）| `../ContextForge-wt-cli-pipeline` |
| 9.6 | release/readme | docs/specs/tasks/task-9.6-readme-quickstart-verified.md | Done | Phase9 #6（dep 9.5，收口）| `../ContextForge-wt-cli-pipeline` |
| 10.1 | contractv1 | docs/specs/tasks/task-10.1-contractv1-types.md | Done | Phase10 #1 | `../ContextForge-wt-console-contract-v1` |
| 10.2 | core/workspace | docs/specs/tasks/task-10.2-workspace-resource.md | Done | Phase10 #2（dep 10.1）| `../ContextForge-wt-console-contract-v1` |
| 10.3 | core/jobs | docs/specs/tasks/task-10.3-indexjob-resource.md | Done | Phase10 #3（dep 10.2，可 ∥ 10.2 部分阶段）| `../ContextForge-wt-console-contract-v1` |
| 10.4 | internal/consoleapi | docs/specs/tasks/task-10.4-rest-endpoints.md | Done | Phase10 #4（dep 10.1,10.2,10.3）| `../ContextForge-wt-console-contract-v1` |
| 10.5 | test/conformance | docs/specs/tasks/task-10.5-conformance-test.md | Done | Phase10 #5（dep 10.4）| `../ContextForge-wt-console-contract-v1` |
| 10.6 | scripts/console_smoke | docs/specs/tasks/task-10.6-console-integration-smoke.md | Done | Phase10 #6（dep 10.5，收口）| `../ContextForge-wt-console-contract-v1` |
| 11.1 | core/proto + core/src/data_plane | docs/specs/tasks/task-11.1-rust-data-plane-grpc-services.md | Done | Phase11 #1 | `../ContextForge-wt-console-real-data-plane` |
| 11.2 | internal/consoleapi/grpcclient | docs/specs/tasks/task-11.2-go-rest-to-grpc-proxy.md | Done | Phase11 #2（dep 11.1）| `../ContextForge-wt-console-real-data-plane` |
| 11.3 | core/src/data_plane/job + IndexSession wiring | docs/specs/tasks/task-11.3-indexjob-real-runner-wiring.md | Done | Phase11 #3（dep 11.1,11.2）| `../ContextForge-wt-console-real-data-plane` |
| 11.4 | core/src/data_plane/search + events | docs/specs/tasks/task-11.4-search-real-retriever-and-events.md | Done | Phase11 #4（dep 11.1,11.2,11.3，收口）| `../ContextForge-wt-console-real-data-plane` |
| 12.1 | internal/consoleapi (router + handlers + grpcclient + confirmMiddleware) | docs/specs/tasks/task-12.1-quick-win-rest-endpoints.md | Done | Phase12 #1 | `../ContextForge-wt-console-contract-completion` |
| 12.2 | core/src/retriever + core/src/data_plane/search.rs + Go REST | docs/specs/tasks/task-12.2-source-chunk-by-id.md | Done | Phase12 #2（dep 12.1）| `../ContextForge-wt-console-contract-completion` |
| 12.3 | core/src/data_plane/search.rs (trace persistence) + Go REST | docs/specs/tasks/task-12.3-search-trace-by-query-id.md | Done | Phase12 #3（dep 12.1,12.2，收口）| `../ContextForge-wt-console-contract-completion` |
| 13.1 | core/migrations + core/src/memory + core/src/data_plane/memory.rs + proto MemoryService | docs/specs/tasks/task-13.1-rust-memory-grpc-service.md | Done | Phase13 #1 | `../ContextForge-wt-memory-rest-surface` |
| 13.2 | internal/consoleapi (router + handlers + grpcclient) + memstore MemoryAdapter | docs/specs/tasks/task-13.2-go-memory-rest-handlers.md | Done | Phase13 #2（dep 13.1，收口）| `../ContextForge-wt-memory-rest-surface` |
| 14.1 | core/migrations + core/src/eval + core/src/data_plane/eval.rs + proto EvalService | docs/specs/tasks/task-14.1-rust-eval-grpc-service.md | Done | Phase14 #1 | `../ContextForge-wt-eval-rest-surface` |
| 14.2 | internal/consoleapi (router + handlers + grpcclient) + memstore EvalAdapter + eval_runner.go | docs/specs/tasks/task-14.2-go-eval-rest-handlers.md | Done | Phase14 #2（dep 14.1，收口）| `../ContextForge-wt-eval-rest-surface` |
| 15.1 | internal/consoleapi/memstore.go (chunkCache + traceCache) | docs/specs/tasks/task-15.1-memstore-chunk-trace-cache.md | Done | Phase15 #1 | `../ContextForge-wt-console-functional-gap-closure` |
| 15.2 | core/src/data_plane/memory.rs (emit EventBus) | docs/specs/tasks/task-15.2-memory-event-bus-bridge.md | Done | Phase15 #2 | `../ContextForge-wt-console-functional-gap-closure` |
| 15.3 | proto + core/src/data_plane/search.rs + Go REST GET /v1/stats/chunks | docs/specs/tasks/task-15.3-chunks-stats-endpoint.md | Done | Phase15 #3（dep 15.2 后实施 — 复用 EventBus 已稳定）| `../ContextForge-wt-console-functional-gap-closure` |
| 15.4 | proto + core/src/eval/store.rs + Go REST GET /v1/eval-runs | docs/specs/tasks/task-15.4-list-eval-runs-endpoint.md | Done | Phase15 #4（dep 15.3 完成 — 串行 proto 修改）| `../ContextForge-wt-console-functional-gap-closure` |
| 15.5 | proto + core/src/data_plane/search.rs (TraceStore.list) + Go REST GET /v1/queries | docs/specs/tasks/task-15.5-query-history-endpoint.md | Done | Phase15 #5（dep 15.4 完成 — 串行 proto）| `../ContextForge-wt-console-functional-gap-closure` |
| 15.6 | proto + core/src/health.rs + Go REST GET /v1/health?detailed=true | docs/specs/tasks/task-15.6-health-component-detail.md | Done | Phase15 #6（dep 15.5，收口含 smoke v6 + ADR-014 D2 lint + closeout）| `../ContextForge-wt-console-functional-gap-closure` |
| 16.1 | core/migrations/0015_search_traces.sql + core/src/data_plane/search_persist.rs + core/src/data_plane/search.rs write-through | docs/specs/tasks/task-16.1-tracestore-sqlite-persistence.md | Done | Phase16 #1 | `../ContextForge-wt-v0.9.0-backlog-completion` |
| 16.2 | internal/consoleapi (handlers + types + grpcclient + memstore) Recent(limit, wait) | docs/specs/tasks/task-16.2-events-real-long-poll.md | Done | Phase16 #2（dep 16.1，串行 ship 便于 review；无文件级冲突）| `../ContextForge-wt-v0.9.0-backlog-completion` |
| 16.3 | .github/workflows/release.yml + ci.yml | docs/specs/tasks/task-16.3-ghcr-image-push-ci.md | Done | Phase16 #3（可与 16.4 并行 — 纯 ops）| `../ContextForge-wt-v0.9.0-backlog-completion` |
| 16.4 | deploy/docker-compose.production.yml + .env.production.example + docs/deploy/production.md + smoke v7 + release_smoke.sh phase16 段 | docs/specs/tasks/task-16.4-compose-production-example.md | Done | Phase16 #4（dep 16.3 image push；收口含 smoke v7 + ADR-014 D2 lint + closeout）| `../ContextForge-wt-v0.9.0-backlog-completion` |
| 17.1 | proto MemoryItem.is_pinned + memory_to_pb mapper + internal/contractv1/contractv1.go::MemoryItem.IsPinned + grpcclient.protoToMemoryItem + internal/consoleapi/memstore.go is_pinned wiring + handleMemoryPin body parse + smoke v8 step 28 (migration 0017 not needed — column already in 0013) | docs/specs/tasks/task-17.1-memory-is-pinned-field.md | Done | Phase17 #1（dep [ADR-022](decisions/adr-022-memory-is-pinned-field-amendment.md) D4 cross-repo signal resolved 2026-05-28 — Console master @ 415ee30 ships MemoryItem.IsPinned）| `../ContextForge-wt-is-pinned-amendment` |
| 18.1 | core/src/retriever/vector/{mod,traits,noop}.rs 三 trait + NoopVectorBackend 占位 + retriever wiring Option&lt;Arc&lt;dyn VectorSearcher&gt;&gt; + Cargo workspace vector-spike feature scaffold | docs/specs/tasks/task-18.1-vector-trait.md | Done | Phase18 #1（trait-first 决策首项；ship 后 task-18.2 spike harness 启动 + task-18.3-18.6 4 backend 并行可启）| `../ContextForge-wt-vector-backend-selection` |
| 18.2 | bench/ crate（确定性 corpus 生成 + measure 5 维 + runner over trait + Noop smoke）+ scripts/spike_vector_backends.sh + docs/spikes/_template.md + dogfood fixture | docs/specs/tasks/task-18.2-spike-harness.md | Done | Phase18 #2（测量台；ship 后 task-18.3-18.6 接入真 backend 跑 evidence、task-18.7 消费 5 维数据选型）| `../ContextForge-wt-vector-backend-selection` |
| 18.3 | core/src/retriever/vector/sqlite_vec.rs SqliteVecBackend (rusqlite bundled + sqlite-vec 0.1.9 vec0) + vector-sqlite feature + bench 注册表接入 + 5 维 evidence（Linux gcc 实测 recall@5/10=1.0 P95 0.167ms cold-start 36.8ms idle/index RSS 6.0/8.5MB） | docs/specs/tasks/task-18.3-spike-sqlite-vec.md | Done | Phase18 #3（Linux x86_64 实测真实数据；Windows MSVC 受阻 [SPEC-DEFER:phase-future.sqlite-vec-cross-platform] 凭据保留）| `../ContextForge-wt-vector-backend-selection` |
| 18.6 | core/src/retriever/vector/hnsw.rs HnswBackend (instant-distance 纯 Rust HNSW) + vector-hnsw feature + bench 注册表接入 + 5 维 evidence | docs/specs/tasks/task-18.6-spike-hnsw.md | Done | Phase18 #4（首个真实召回数据 backend；release n=5000/dim=64 recall@5/10=1.0 P95 0.23ms；跨平台可构建）| `../ContextForge-wt-vector-backend-selection` |
| 18.4 | core/src/retriever/vector/qdrant.rs QdrantBackend (qdrant-client 1.18 gRPC + 自带 tokio runtime block_on) + vector-qdrant feature + bench 注册表接入 + 5 维 evidence（Linux 本机 Qdrant v1.18.1 实测 recall@5/10=1.0 P95 0.650ms server RSS ~104.8MB cold-start 30.9ms） | docs/specs/tasks/task-18.4-spike-qdrant.md | Done | Phase18 #5（Linux 实测真实数据；外部 server 进程 is_local=false，server RSS ~10x 进程内 backend，gRPC 往返延迟）| `../ContextForge-wt-vector-backend-selection` |
| 18.5 | core/src/retriever/vector/lance_db.rs LanceDbBackend (lancedb 0.30 embedded Lance + Arrow RecordBatch + 自带 tokio runtime block_on) + vector-lancedb feature + bench 注册表接入 + 5 维 evidence（Linux 实测 recall@5/10=1.0 P95 1.551ms idle/index RSS 30.5/50.9MB cold-start 7.4ms） | docs/specs/tasks/task-18.5-spike-lancedb.md | Done | Phase18 #6（Linux 实测真实数据；嵌入式 is_local=true 磁盘持久化列式；最重进程内 RSS + 最快写入；build 需 protoc）| `../ContextForge-wt-vector-backend-selection` |
| 18.7 | 4 路 backend 5 维横向对比（n=5000+100000）+ ADR-023 默认 backend 选型（Proposed，分层 D1-D6）+ comparison 文档 + hnsw evidence 补 Linux RSS/100k + known_backends unused_mut 清理 | docs/specs/tasks/task-18.7-decision-adr023.md | Done | Phase18 #7（合成 recall 不可区分 → 架构驱动选型：D1 sqlite-vec 嵌入式默认 provisional / D2 hnsw 跨平台 fallback / D3 qdrant scale-out / D4 lancedb 列式；ratify 待 task-18.8 真实 embedding recall）| `../ContextForge-wt-vector-backend-selection` |
| 18.8 | internal/eval SemanticRecall@K 度量 + Report semantic 字段 + SummarizeHybrid 双路 + MeetsRecallGate（BM25 恒检 + SemanticRecall@10≥0.70 仅 semantic 时检）+ 4 单测 + ADR-006 add-only Amendment A1 | docs/specs/tasks/task-18.8-eval-semantic-recall.md | Done | Phase18 #8（度量+门禁+单测落地；live 语义召回值 + ratify 待真实 embedding provider [SPEC-OWNER:phase-future.vector-retrieval-integration]）| `../ContextForge-wt-vector-backend-selection` |
| 18.9 | Phase 18 closeout（诚实缩范围）+ v0.11.0 release docs（README + RELEASE_NOTES + evidence + artifacts）+ phase-18 §6/§8 诚实状态（AC1/2/5/6 met / AC3 partial / AC4 deferred）+ v0.11.0 tag | docs/specs/tasks/task-18.9-release-v0.11.0-closeout.md | Done | Phase18 #9（v0.11.0 = 向量 backend 基础设施+选型里程碑；生产语义搜索 + ADR ratify 后置；用户授权切版）| `../ContextForge-wt-vector-backend-selection` |
| 19.1 | core/src/embedding/{mod,traits,deterministic,fastembed_provider}.rs EmbeddingProvider trait + DeterministicEmbeddingProvider（无模型缺省，默认构建）+ FastEmbedProvider（fastembed-rs rustls，all-MiniLM-L6-v2 dim 384，feature-gated）+ 候选评估 evidence | docs/specs/tasks/task-19.1-spike-embedding-provider.md | Done | Phase19 #1（embedding 首项；fastembed 跨平台可构建 Linux 30s + Win MSVC 1m11s，无 stop-condition；解锁 19.2 wiring）| master |
| 19.2 | core/src/retriever/mod.rs embedder 字段 + with_embedder + index_chunks_semantic + search_semantic（retrieval_method=vector + 12-field 装配）+ search() 探针改用 query embedding；hnsw 默认 wiring backend（ADR-023 D2 全平台）；None→BM25 不退化 | docs/specs/tasks/task-19.2-default-backend-wiring.md | Done | Phase19 #2（dep 19.1 + ADR-023 D2；hnsw + deterministic provider index→search roundtrip 实测命中；默认构建 0 vector dep）| master |
| 19.3 | proto SearchRequest.semantic=7 + RetrievalResult.vector_score=13/embedding_provider=14（add-only，buf 重生成）+ CoreService::search semantic 分派（DeterministicEmbeddingProvider + 新增 0-dep BruteForceVectorBackend 按需建索）+ Retriever.enumerate_chunks + Go handleSearch ?semantic=true + 3 测试 | docs/specs/tasks/task-19.3-semantic-search-api.md | Done | Phase19 #3（contract add-only，22-endpoint conformance + proto freeze 守护 PASS；默认构建语义路径可用经 brute-force 0-dep searcher，ADR-023 D5 默认 BM25 行为不变）| master |
| 19.4 | scripts/console_smoke.sh v9 30-step（step 29 /v1/search?semantic=true 合约保形 + step 30 eval --semantic 双路）+ internal/cli/eval.go --semantic CLI flag（evalSearchPass 双趟 + SummarizeHybrid + MeetsRecallGate gate 行）+ 3 Go 测试 | docs/specs/tasks/task-19.4-smoke-v9.md | Done | Phase19 #4（dep 19.3；接 task-18.8 SummarizeHybrid/MeetsRecallGate；ADR-013 step 30 仅断言双路成形不预判召回；console-api semantic 转发 follow-up 留 task-19.5）| master |
| 19.5 | core/examples/phase19_real_recall.rs（feature-gated real fastembed 谐波）+ test/fixtures/eval/dogfood-embeddings.jsonl（40 行 dim-384 real 语料）+ docs/spikes/phase-19-real-recall.md（真实 SemanticRecall@5=0.83/@10=0.93 gate PASS + top1/MRR 区分度 + per-category）+ bench fixture 测试 | docs/specs/tasks/task-19.5-real-recall-eval.md | Done | Phase19 #5（real provider R1 未触发；balanced corpus 修正 artifact 后可区分真实召回；@10=0.9333≥0.70 喂 19.6 ratify；ADR-013 数据源诚实声明 real-run）| master |
| 19.6 | ADR-023 Proposed→**Accepted**（据 task-19.5 真实 recall@10=0.9333≥0.70 exact-cosine）+ ADR-006 A1 Proposed/provisional→**Active**（A1.4 ratification 注）+ ADR-008 add-only embedding crate（fastembed feature-gated）+ 记 Phase 18 §6 AC3/AC4 在 Phase 19 解决（不溯改 Phase 18 spec，D5）| docs/specs/tasks/task-19.6-adr-023-ratify.md | Done | Phase19 #6（dep 19.5；全 add-only 不改既有 ADR 正文；据真实非合成数据 ratify，ADR-013 守线；实现默认 backend = 0-dep brute-force exact 经 D5）| master |
| 19.7 | Phase 19 closeout（端到端语义检索 ship，路径 A）+ v0.12.0 release docs（README/RELEASE_NOTES/evidence/artifacts）+ phase-19 §6 AC1-6 [x] + Status Done + adapter Phase 0→7/forward-ref 解除 + tag（用户授权 push @ `dcbe09b`，release.yml run 26685041851 success，ghcr v0.12.0+latest @ sha256:6f0ae8…d2990）+ backfill | docs/specs/tasks/task-19.7-closeout-v0.12.0.md | Done | Phase19 #7（dep 19.1-19.6；v0.12.0 = 端到端语义检索 live + ADR-023 Accepted；tag 经用户授权已 push + 镜像已发布）| master |
| 20.1 | console_data_plane SearchRequest add-only semantic=7（buf 重生成）+ Rust SearchServer::query 语义分派（仿 server.rs CoreService）+ internal/contractv1 SearchRequest.Semantic + handleSearch ?semantic=true OR-merge + grpcclient 透传 | docs/specs/tasks/task-20.1-console-api-semantic-forward.md | Done | Phase20 #1（闭合 task-19.4 §10 console-api 未转发 caveat；**实施期发现 console_data_plane proto 与 core contextforge/v1 proto 分离**——初版误称 0 delta，真实 scope 扩为 proto+Rust 分派+Go，spec §10 记 drift；deterministic embeddings 证 plumbing，真实召回 [SPEC-OWNER:task-20.2]） | master |
| 20.2 | core/examples/phase20_recall_via_retriever.rs 经生产 Retriever::search_semantic 热路径跑真实召回 + core/src/retriever/mod.rs test_20_2 默认构建确定性 hot-path + docs/spikes/phase-20-recall-via-retriever.md | docs/specs/tasks/task-20.2-real-recall-via-retriever.md | Done | Phase20 #2（real run @ production 175 chunks：recall@5=0.9667/@10=1.0/top1=0.7333/MRR=0.8367 gate PASS；@10=1.0 部分含 uncapped-chunk 膨胀但 top1/MRR 高于 19.5 证真实路径；确定性 test_20_2 守默认构建 wiring，ADR-013）| master |
| 20.3 | scripts/console_smoke.sh v10 console-api ?semantic=true 真实语义断言（grep vector-bruteforce）+ v0.13.0 release docs + ADR-024 Proposed→Accepted + phase-20 §6 闭合 + adapter | docs/specs/tasks/task-20.3-closeout-v0.13.0.md | Done | Phase20 #3（dep 20.1+20.2；smoke v10 step29 升真实语义断言 + TEST-20.3.1；release docs 备齐；tag push 待用户明确授权，stop-condition c）| master |
| 21.1 | core/src/retriever/fusion.rs RRF 融合（k=60）+ Retriever::search_hybrid + proto SearchRequest.hybrid=8/RetrievalResult.hybrid_score=15（add-only）+ server.rs CoreService req.hybrid 分派（retrieval_method="hybrid"，hybrid_score 从 score 装配）| docs/specs/tasks/task-21.1-hybrid-scoring.md | Done | Phase21 #1（RRF 确定性融合序 CI 可断言（test_21_1×4 PASS）；实现决策:不加 SearchResult 字段（融合分入 score，避全库字面量 churn）+ 分派在 server.rs CoreService 非 data_plane（console hybrid 转发 defer）；策略选型据真实 eval ratify ADR-025@task-21.3）| master |
| 21.2 | core/src/rerank/{mod,traits,identity,cross_encoder}.rs Reranker trait + 确定性 IdentityReranker（默认 0 模型依赖）+ CrossEncoderReranker（feature-gated）+ Retriever::with_reranker seam | docs/specs/tasks/task-21.2-reranker-pipeline.md | Done | Phase21 #2（dep 19.1 EmbeddingProvider trait 范式；可与 21.1 并行（fusion.rs vs rerank/ 新模块）；real 模型质量 ADR-013 如实 defer，受阻 stop-condition）| master |
| 21.3 | internal/eval Report 加 hybrid/reranked 列 + internal/cli/eval.go --hybrid/--rerank flag + scripts/console_smoke.sh v11 hybrid/rerank 真实断言 + core/examples/phase21_hybrid_rerank_recall.rs 真实 dogfood eval + v0.14.0 release docs + ADR-025/026 ratify(Accepted) + phase-21 §6 闭合 + adapter | docs/specs/tasks/task-21.3-closeout-v0.14.0.md | Done | Phase21 #3（dep 21.1+21.2；SummarizePasses add-only + rerankIdentity（eval 层确定性，console rerank forward [SPEC-DEFER]）；真实 eval hybrid/reranker vs baseline 驱动 ADR-025/026 Accepted（ADR-026 诚实 hybrid caveat）；tag push 经用户授权；承 task-19.7/20.3 closeout 模式）| master |
| 22.1 | internal/config `[embedding]`（provider/dim，add-only TOML 段仿 `[remote]`）+ core/src/embedding/factory.rs `select_provider`（deterministic/fastembed/remote 工厂选择 + dim 协商 `DimMismatch`）+ core/src/server.rs 语义路径改用工厂（缺省确定性 identity 实现行为不变）| docs/specs/tasks/task-22.1-provider-config-selection.md | Done | Phase22 #1（首项；提供 provider 选择 seam 解锁 22.2/22.3；0 网络 dep；缺省向后兼容承 ADR-027 D1/D2）| master |
| 22.2 | core/src/embedding/cache.rs `CachingEmbeddingProvider`（content-hash Sha256(text)→embedding 缓存装饰器；内存缺省 + 可选 SQLite 持久化承 ADR-002）+ 确定性命中/失效单测（计数 wrapper 断言底层跳过）| docs/specs/tasks/task-22.2-embedding-cache.md | Done | Phase22 #2（dep 22.1 工厂；可与 22.3 并行 cache.rs vs remote_provider.rs 写路径不相交；sha2/rusqlite/base64 已 direct dep 0 新 dep）| master |
| 22.3 | core/src/embedding/remote_provider.rs `RemoteEmbeddingProvider`（OpenAI/Cohere HTTP，embedding-remote feature-gated，rustls 承 fastembed 口径）+ build_request_body/parse_response 纯函数 + 契约级确定性测试（请求构造/响应解析/错误路径，不打真实网络）+ Cargo.toml embedding-remote feature | docs/specs/tasks/task-22.3-remote-provider-skeleton.md | Done | Phase22 #3（dep 22.1 工厂 remote 分支；可与 22.2 并行；默认构建 0 网络 dep 承 ADR-004/ADR-008 D5；真实联调+密钥 🔴 如实 defer，§8 R1 stop-condition，ADR-013）| master |
| 22.4 | core/src/health.rs probe_embed 远程可达性探针（opt-in，config-only 缺省承 ADR-020 D1）+ scripts/console_smoke.sh v12（配置选择+缓存命中确定性断言）+ v0.15.0 release docs + ADR-027 ratify + phase-22 §6 闭合 + adapter | docs/specs/tasks/task-22.4-closeout-v0.15.0.md | Done | Phase22 #4（dep 22.1+22.2+22.3 全 Done；承 task-19.7 closeout 模式；tag push 经用户授权；远程探针真实命中 [SPEC-DEFER:phase-future.embed-remote-probe] 如实 defer）| master |
| 23.1 | core/src/retriever/vector/hnsw.rs HnswBackend 图序列化/反序列化到磁盘（VectorIndexConfig.persistence_path 既有字段首次消费）+ rebuild-on-load fallback + feature vector-hnsw 序列化往返 roundtrip 测试 | docs/specs/tasks/task-23.1-hnsw-graph-persistence.md | Done | Phase23 #1（dep task-18.6 HnswBackend + task-18.1 persistence_path；可与 23.2 并行，hnsw.rs vs sqlite_vec.rs/Cargo.toml 写路径不相交；管道 🟢 / feature 真实持久化往返 🟡）| master |
| 23.2 | core/Cargo.toml vector-sqlite + core/src/retriever/vector/sqlite_vec.rs Windows MSVC 可构建路径调查（bundled amalgamation / 预编译扩展 / 替代绑定三路径）+ docs/spikes/phase-23-sqlite-vec-cross-platform.md（落地或诚实文档化 stop-condition，禁伪造跨平台通过）| docs/specs/tasks/task-23.2-sqlite-vec-cross-platform.md | Done | Phase23 #2（dep task-18.3 SqliteVecBackend Linux gcc 凭据；🔴 受阻平台调查类；结论=落地或 stop-condition；受阻不阻塞 23.1/23.3）| master |
| 23.3 | 向量增量索引评估（最小实现或如实延后 [SPEC-DEFER:phase-future.vector-incremental-index]）+ scripts/console_smoke.sh v13 向量持久化/跨平台 smoke + v0.16.0 release docs + ADR-028 ratify + ADR-023/008 add-only Amendment + phase-23 §6 闭合 + adapter | docs/specs/tasks/task-23.3-closeout-v0.16.0.md | Done | Phase23 #3（dep 23.1+23.2 全 Done；tag push 经用户授权；承 task-19.7/18.9 closeout 模式）| master |
| 24.1 | core/src/indexer/mod.rs 自定义 code/CJK TextAnalyzer（opt-in + 代码符号拆分保留原 token + CJK bigram + 默认不变）| docs/specs/tasks/task-24.1-code-and-cjk-tokenizer.md | Done | Phase24 #1（dep task-2.4 indexer schema + task-4.1 RetrieverConfig.tokenizer 接入点；可与 24.2 并行）| master（#173；TEST-24.1.1-4，0 新 dep，纯 std opt-in via config）|
| 24.2 | internal/eval/eval.go 数据集校验器（schema/重复/覆盖）+ test/fixtures/eval/golden-semantic.jsonl 代码/CJK 扩充 | docs/specs/tasks/task-24.2-eval-dataset-hardening.md | Done | Phase24 #2（dep task-8.1 ValidateDataset + task-19.5 golden 口径；可与 24.1 并行）| master（#174；TEST-24.2.1-4，ValidateGoldenSemantic add-only，gate 阈值不变）|
| 24.3 | tokenizer 真实 before/after recall delta + core/src/eval/runner.rs 评估（promote/延后）+ console_smoke v14 + v0.17.0 closeout + ADR-029 ratify | docs/specs/tasks/task-24.3-closeout-v0.17.0.md | Done | Phase24 #3（dep 24.1+24.2；收口）| master（recall delta +0.0909 实测 + runner 诚实延后 + smoke v14 + ADR-029 Accepted）|
| 25.1 | core/src/retriever/vector/qdrant.rs 生命周期层（connection-config validate + health-probe + decide_ensure 纯函数契约层 deterministic 单测）| docs/specs/tasks/task-25.1-qdrant-server-lifecycle.md | Draft | Phase25 #1（dep task-18.4 QdrantBackend spike；契约层不需 live server；可与 25.2 并行）| 规划 Draft |
| 25.2 | core/src/retriever/vector/lance_db.rs 真实可构建性调查（dev-box cargo build protoc 前置三态）+ 索引调参参数校验 | docs/specs/tasks/task-25.2-lancedb-buildability-and-index-tuning.md | Draft | Phase25 #2（dep task-18.5 LanceDbBackend spike；仿 task-23.2 sqlite-vec MSVC 调查 pattern；可与 25.1 并行）| 规划 Draft |
| 25.3 | 生产 backend 选择矩阵 + console_smoke v15 + v0.18.0 closeout + ADR-030 ratify + ADR-023 add-only Amendment | docs/specs/tasks/task-25.3-closeout-v0.18.0.md | Draft | Phase25 #3（dep 25.1+25.2；收口）| 规划 Draft |
| 26.1 | core/src/data_plane/search_persist.rs TraceStore FTS5 shadow 表 + prune_older_than/VACUUM（migration 0016 add-only，bundled SQLite 0 新 dep）| docs/specs/tasks/task-26.1-tracestore-fts-and-vacuum.md | Draft | Phase26 #1（dep Phase16 task-16.1 SqliteTracePersist；可与 26.2 并行）| 规划 Draft |
| 26.2 | events SSE 推送（GET /v1/observability/events/stream，Go http.Flusher add-only）+ audit_log 重放 + event-bus 容量/drain 配置 | docs/specs/tasks/task-26.2-events-sse-push-and-replay.md | Draft | Phase26 #2（dep Phase16 task-16.2 long-poll + Phase11 task-11.4 EventBus + ADR-021；可与 26.1 并行）| 规划 Draft |
| 26.3 | console_smoke v16 + v0.19.0 closeout + ADR-031 ratify + ADR-021/015 add-only Amendment | docs/specs/tasks/task-26.3-closeout-v0.19.0.md | Draft | Phase26 #3（dep 26.1+26.2；收口）| 规划 Draft |
| 27.1 | proto add-only pinned_by(string)/pinned_at_unix(int64) + core/src/memory/store.rs 写穿 + audit 回填 | docs/specs/tasks/task-27.1-memory-pin-actor-and-timestamp.md | Draft | Phase27 #1（dep Phase13 task-13.1 MemoryService + Phase17 task-17.1 is_pinned + ADR-022 §Trade-offs 三 marker；proto add-only 不破冻结）| 规划 Draft |
| 27.2 | proto add-only Unpin/HardDelete RPC + Pin/Unpin 显式拆分 + hard-delete X-Confirm（复用 confirmMiddleware，ADR-017 D2）| docs/specs/tasks/task-27.2-memory-pin-unpin-split-and-hard-delete.md | Draft | Phase27 #2（dep 27.1；串行 proto add-only）| 规划 Draft |
| 27.3 | console_smoke v17 + v0.20.0 closeout + ADR-032 ratify | docs/specs/tasks/task-27.3-closeout-v0.20.0.md | Draft | Phase27 #3（dep 27.1+27.2；收口）| 规划 Draft |

## ADR 索引

> 核心技术决策的独立记录（按 s2v full-standard §16.2 模板）。新增 ADR 用 `/s2v-add adr <title>`。
>
> **Status 取值**：ADR 自有状态机（不同于 spec）— `Proposed / Accepted / Deprecated / Superseded`。

| # | Title | Status | File |
|---|---|---|---|
| 001 | go-rust-dual-binary-architecture | Accepted | docs/decisions/adr-001-go-rust-dual-binary-architecture.md |
| 002 | sqlite-tantivy-layered-storage | Accepted | docs/decisions/adr-002-sqlite-tantivy-layered-storage.md |
| 003 | cli-rest-mcp-grpc-interfaces | Accepted | docs/decisions/adr-003-cli-rest-mcp-grpc-interfaces.md |
| 004 | local-first-privacy-baseline | Accepted | docs/decisions/adr-004-local-first-privacy-baseline.md |
| 005 | readonly-import-draft-export | Accepted | docs/decisions/adr-005-readonly-import-draft-export.md |
| 006 | recall-eval-acceptance-gate | Accepted | docs/decisions/adr-006-recall-eval-acceptance-gate.md |
| 007 | minimal-tarball-distribution | Accepted | docs/decisions/adr-007-minimal-tarball-distribution.md |
| 008 | core-library-selection | Accepted | docs/decisions/adr-008-core-library-selection.md |
| 009 | provenance-timestamp-placeholder | Accepted | docs/decisions/adr-009-provenance-timestamp-placeholder.md |
| 010 | audit-cross-language-unification | Proposed | docs/decisions/adr-010-audit-cross-language-unification.md |
| 011 | single-driver-with-subagents | Proposed | docs/decisions/adr-011-single-driver-with-subagents.md |
| 012 | main-agent-governance-autonomy | Accepted | docs/decisions/adr-012-main-agent-governance-autonomy.md |
| 013 | cli-data-plane-grpc-bridge | Accepted | docs/decisions/adr-013-cli-data-plane-grpc-bridge.md |
| 014 | cross-phase-exit-criteria-validation | Accepted | docs/decisions/adr-014-cross-phase-exit-criteria-validation.md |
| 015 | console-contract-v1-compatibility | Accepted | docs/decisions/adr-015-console-contract-v1-compatibility.md |
| 016 | cross-process-rust-go-via-grpc-bridge | Accepted | docs/decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md |
| 017 | console-contract-completion-22-endpoint | Accepted | docs/decisions/adr-017-console-contract-completion-22-endpoint.md |
| 018 | fallback-inmem-default-reversal | Accepted | docs/decisions/adr-018-fallback-inmem-default-reversal.md |
| 020 | health-component-breakdown | Accepted | docs/decisions/adr-020-health-component-breakdown.md |
| 021 | memory-event-bus-bridge | Accepted | docs/decisions/adr-021-memory-event-bus-bridge.md |
| 022 | memory-is-pinned-field-amendment | Accepted | docs/decisions/adr-022-memory-is-pinned-field-amendment.md |
| 023 | vector-backend-default | Accepted | docs/decisions/adr-023-vector-backend-default.md |
| 024 | console-api-semantic-forward | Proposed | docs/decisions/adr-024-console-api-semantic-forward.md |
| 025 | hybrid-scoring-fusion | Accepted | docs/decisions/adr-025-hybrid-scoring-fusion.md |
| 026 | reranker-provider | Accepted | docs/decisions/adr-026-reranker-provider.md |
| 027 | embedding-provider-abstraction | Accepted | docs/decisions/adr-027-embedding-provider-abstraction.md |
| 028 | vector-persistence-strategy | Accepted | docs/decisions/adr-028-vector-persistence-strategy.md |
| 029 | code-and-cjk-tokenizer-and-eval-hardening | Accepted | docs/decisions/adr-029-code-and-cjk-tokenizer-and-eval-hardening.md |
| 030 | production-vector-backend | Proposed | docs/decisions/adr-030-production-vector-backend.md |
| 031 | observability-hardening | Proposed | docs/decisions/adr-031-observability-hardening.md |
| 032 | memory-ops-hardening | Proposed | docs/decisions/adr-032-memory-ops-hardening.md |

## BDD Feature 索引

> 轻量 BDD（s2v §9.2）：`.feature` 作为业务可读场景文档。Scenario ID 在对应 task spec §7 追踪表中映射到具体测试。

| Task(s) | Feature 文件 |
|---|---|
| 1.1 | test/features/proto.feature |
| 1.2 | test/features/config.feature |
| 1.3 | test/features/core.feature |
| 1.4 / 6.1 | test/features/cli.feature |
| 2.1 | test/features/scanner.feature |
| 2.2 | test/features/parser.feature |
| 2.3 | test/features/chunker.feature |
| 2.4 | test/features/indexer.feature |
| 3.1 / 3.2 / 3.3 / 3.4 | test/features/importer.feature |
| 4.1 / 4.2 | test/features/retriever.feature |
| 5.1 / 5.2 / 5.3 | test/features/memoryops.feature |
| 6.2 | test/features/daemon.feature |
| 6.3 | test/features/exporter.feature |
| 7.1 | test/features/mcp-adapter.feature |
| 8.1 | test/features/eval.feature |
| 8.2 | test/features/reliability.feature |
| 8.3 | test/features/release.feature |
| 9.1 / 9.2 / 9.3 / 9.4 / 9.5 / 9.6 | test/features/cli-pipeline.feature |
| 10.1 / 10.2 / 10.3 / 10.4 / 10.5 / 10.6 | test/features/console-contract-v1.feature |
| 11.1 / 11.2 / 11.3 / 11.4 | test/features/console-real-data-plane.feature |
| 12.1 / 12.2 / 12.3 / 13.1 / 13.2 / 14.1 / 14.2 | test/features/console-contract-completion.feature |
| 15.1 / 15.2 / 15.3 / 15.4 / 15.5 / 15.6 | test/features/phase-15-console-functional-gap-closure.feature |
| 16.1 / 16.2 / 16.3 / 16.4 | test/features/phase-16-v0.9.0-backlog-completion.feature |
| 17.1 | test/features/phase-17-is-pinned-amendment.feature |
| 19.1 / 19.2 / 19.3 / 19.5 | test/features/phase-19-vector-retrieval-integration.feature |
| 20.1 / 20.2 / 20.3 | test/features/phase-20-semantic-retrieval-throughline.feature |
| 21.1 / 21.2 / 21.3 | test/features/phase-21-retrieval-quality.feature |
| 22.1 / 22.2 / 22.3 / 22.4 | test/features/phase-22-embedding-provider-completion.feature |
| 23.1 / 23.2 / 23.3 | test/features/phase-23-vector-persistence-and-cross-platform.feature |
| 24.1 / 24.2 / 24.3 | test/features/phase-24-retrieval-tokenizer-and-eval-hardening.feature |
| 25.1 / 25.2 / 25.3 | test/features/phase-25-production-vector-backend.feature |
| 26.1 / 26.2 / 26.3 | test/features/phase-26-observability-hardening.feature |
| 27.1 / 27.2 / 27.3 | test/features/phase-27-memory-ops-hardening.feature |

---

### subagent spawn 范式（单驱动变体，2026-05-23 起；前身"派工模板"见 ADR-011）

主 agent 用 Agent tool spawn 实施 subagent 时，prompt 应包含：

```
[task 目标]   task-<X.Y>（spec: docs/specs/tasks/task-<X.Y>-<name>.md）
[Worktree]   ../ContextForge-wt-task-<X.Y>（Agent tool isolation: "worktree" 自动建）
[Branch]     feat/task-<X.Y>-<name>
[subagent_type]  按任务选（Explore / Plan / general-purpose / 项目自定义 agent / claude）
[isolation]      worktree（需写隔离）/ 不设（只读探索）

进入 worktree 后请：
  1. 跑 AGENTS.md §3 step 0-3 的环境校验 + 基线测试
  2. 按顺序读：AGENTS.md / 本 adapter / 该 task spec / 该 spec §5 Required Reading / 对应 .feature 文件
  3. 严格按 task spec §6 AC + §5 Behavior Contract + §7 追踪表执行
  4. RED → GREEN → REFACTOR 三段 commit（按 AGENTS §2.5 节律 + scope 约定）
  5. 每次 commit 后立即跑 R3 grep 校验 [branch]
  6. 完成后 push branch（如有 remote）+ 在 spec §10 Completion Notes 回填 6 项
  7. return ready 对象给主 agent（含 branch / commits / verification 结果摘要）

【硬约束】
- ✅ **允许**修改 task spec 的"流程字段"（按状态机推进）：
  - 顶部 `Status` 行（`Ready → In Progress → Done` / `Blocked` / `Waived`）
  - §7 追踪表的 Status 列（标 `Test Red` / `Verified` / `Done` 等）
  - §10 Completion Notes（完工时按 6 项回填）
- ❌ **禁止**修改 task spec 的"业务契约字段"（主 agent / 用户领域）：
  - §1 Background / §2 Goal / §3 Scope&Out-of-Scope / §4 Actors
  - §5 Behavior Contract（含 §5.1 Required Reading / §5.2 Imports / §5.3 函数签名）
  - §6 Acceptance Criteria
  - §8 Risks / §9 Verification Plan
  - 如果发现这些字段写错或需要改，return spec-drift 对象给主 agent，不要私改
- ❌ 禁止新建任何 task spec（要新 task → 主 agent 跑 `/s2v-add task <name>`）
- ❌ 禁止改 go.mod / go.sum / Cargo.toml / Cargo.lock（R7：return needs-dep 对象给主 agent）
- ❌ 禁止 cd 主 repo / push 到 main / 自己 merge PR（R6）
- ⚠️ 卡住 → return blocked 对象给主 agent（AGENTS §8）→ subagent 退出等主 agent 决策
```

长任务自治另外用 `/goal`（主 agent 自身跨多轮）— 见 [AGENTS.md §3.5](../AGENTS.md) / [ADR-011](decisions/adr-011-single-driver-with-subagents.md)。
