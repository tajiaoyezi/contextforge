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
| 25 | `production-vector-backend` | `docs/specs/phases/phase-25-production-vector-backend.md` | Done | 3 | master（v0.18.0：qdrant server 生命周期契约层（`QdrantConnConfig` validate + `health()` probe + `decide_ensure` ensure-create，不连 live server 4/4，task-25.1）+ lancedb 真实可构建性 🟢（`cargo build --features vector-lancedb` exit 0 @ x86_64-pc-windows-msvc，protoc via 仓内 protoc-bin-vendored，0 新 dep）+ 索引调参参数 `LanceIndexTuning::validate`（task-25.2）+ 生产 backend 选择矩阵（语料规模 × 部署形态 → hnsw/sqlite-vec/lancedb/qdrant + caveat）+ smoke v15 + 收口（task-25.3）。ADR-030 据 D1-D4 真实非合成验证 Proposed→Accepted（qdrant live KNN / lancedb 真实 ANN 索引性能诚实延后）；ADR-023 D3/D4 tier add-only Amendment（不溯改 D1-D6）；默认构建 0 vector 依赖 + BM25-only baseline 不变（ADR-023 D5，无 ADR-008 Amendment）；tag 无人值守授权下主 agent 自主 push） |
| 26 | `observability-hardening` | `docs/specs/phases/phase-26-observability-hardening.md` | Done | 3 | master（v0.19.0：TraceStore FTS5 内容检索 `search_fts` + 周期 VACUUM/prune（migration 0016 add-only，rusqlite bundled 0 新 dep，旧 0015-only 库 boot 回填，task-26.1 #178）+ events SSE 实时推送 `GET /v1/observability/events/stream`（Go http.Flusher add-only 旁挂 long-poll）+ 从 audit_log 重放漏失 memory state-op 事件（proto add-only since_ts/last_event_id，replay_events_from_audit id ASC + D3 映射，task-26.2 #179）+ event-bus 容量/分区/drain 配置（EventBus::from_config + CF_EVENT_BUS_CAPACITY/PARTITION + CONSOLE_EVENTS_DRAIN_TIMEOUT，保守默认 1000/不分区/100ms 行为不变）+ smoke v16 step 35 + 收口（task-26.3）。ADR-031 据 D1-D6 真实非合成验证 Proposed→Accepted（SSE live-server e2e 维度记录维持 `[SPEC-DEFER:phase-future.sse-live-server-e2e]`）；ADR-021 add-only Amendment 兑现 events-replay-from-audit + event-bus 容量/分区 Rollback path 预见；ADR-015 SSE add-only；默认构建 0 新 dep / 0 network + 既有 long-poll/22-endpoint/put/get/list/load_warm 不退化（ADR-004/015）；tag 无人值守授权下主 agent 自主 push） |
| 27 | `memory-ops-hardening` | `docs/specs/phases/phase-27-memory-ops-hardening.md` | Done | 3 | master（v0.20.0：pin-actor + pinned-at-timestamp（`MemoryItem` add-only proto field 11/12 + migration 0017 guarded ALTER + `set_pinned_with_actor` 写穿，task-27.1 #181）+ Pin/Unpin 显式拆分 + hard-delete（add-only `Unpin`/`HardDelete` RPC + `store.hard_delete` 物理删除 + `AuditOperation::MemoryHardDelete` + console-api unpin/hard-delete X-Confirm gated，task-27.2 #183）+ is_pinned audit backfill（`reconcile_is_pinned_from_audit` last-event-wins opt-in，task-27.3）+ smoke v17 step 36 + 收口。ADR-032 据 D1-D4 真实非合成验证 Proposed→Accepted；ADR-022 add-only Amendment 推进 §Trade-offs 三条 marker（pin_actor/pinned-at-timestamp/is-pinned-backfill，不溯改正文 D5）；proto 全 add-only（proto-freeze guard 过）+ 默认构建 0 新 dep / 0 network + 既有 5 RPC 不退化（ADR-004/032 D4）；tag 无人值守授权下主 agent 自主 push） |
| 28 | `release-ci-hardening` | `docs/specs/phases/phase-28-release-ci-hardening.md` | Done | 4 | master（v0.21.0：发布 / CI 硬化 — 匿名可拉取守护 + multi-arch（arm64 emulation 实测不可行 run 26757640892→延后原生 runner，task-28.1）+ 供应链证明（cosign keyless sign + cosign attest SBOM + build-push provenance:max，GitHub 原生 attestation 私有仓库不可用 run 26789731232→改 cosign 即 ADR-033 §D2 原文，机制 run 26799480280 verified·真签@release，task-28.2）+ CI 强 lint（实测存量 gofmt **15 真实**/go vet 0/clippy ~33 全修到全绿 → ci.yml lint job 三阻断，task-28.3）+ smoke v18 step 37 + 收口（task-28.4）。ADR-033 据 D1-D4 真实 ratify Proposed→Accepted（D1 arm64 DEFERRED / D2 机制验证·真签在 release / D3 lint 门绿 / D4 baseline 不变，逐维如实 ADR-013）；ADR-007 add-only Amendment（部署发布面扩展 cosign 签名 OCI，arm64 延后，不溯改正文 D5）；纯 `.github/workflows/*` + lint 修复层，镜像运行时 + 默认 0-network/0-dep baseline 不变（ADR-004）；ADR-014 第十九次激活；tag/release outward-facing 须用户显式授权（ADR-012，真实 v0.21.0 release 待授权）） |
| 29 | `live-vector-recall` | `docs/specs/phases/phase-29-live-vector-recall.md` | Done | 4 | master（v0.22.0：承 Phase 25 把 qdrant/lancedb 契约层 / 参数层兑现为真实 live 向量召回 — vector backend 工厂 `select_vector_backend`（返 `Arc<dyn VectorStore>`，add-only 组合 trait）+ server.rs:302/341 热路径注入替换硬编码 BruteForce（task-29.1 #197，兑现 `[SPEC-DEFER:phase-future.vector-retrieval-integration]`）+ qdrant live KNN 真实 harness（无 server `health()==Unreachable` honest-defer 实测 exit 0，task-29.2 #198，🔴 live / 🟢 wiring）+ lancedb 真实 IVF_PQ/IVF_HNSW_SQ 索引 + compaction + 多 backend 选择矩阵真实测量 → ADR-030/023 add-only Amendment（task-29.3 #199，IVF_HNSW_SQ recall@10≈0.90 / IVF_PQ≈0.44）+ smoke v19 step 38 + closeout（task-29.4）。ADR-034 Accepted（per-D，D2 live-server honest-defer 部分 ratify）；默认构建 0 vector dep + BruteForce 语义 baseline 不变（ADR-004/023 D5）；qdrant live-server 真实召回 honest-defer 真实跑出后回填（ADR-013，不预填）；ADR-014 第二十次激活；4/4 Done 三门绿自主合，tag/release 待用户授权 ADR-012） |
| 30 | `cjk-true-segmenter` | `docs/specs/phases/phase-30-cjk-true-segmenter.md` | Done | 3 | master（v0.23.0：承 Phase 24 把 CJK 重叠 bigram 升级为真分词器 — `cjk-segmenter` feature（jieba-rs 0.7.4，默认 off 0-dep）+ 并行 `cjk_segmenter` analyzer（`配置加载`→`配置`/`加载` vs bigram `配置`/`置加`/`加载`）+ 双站点对称注册、bigram 保留 0-dep fallback（task-30.1 #202）+ `IndexSession::reindex_with_tokenizer` 迁移工具 + `RetrieverConfig.tokenizer` schema-driven 对称（方案 B vestigial）+ 扩 CJK golden（11→16）真实 recall delta（task-30.2 #203，seg−bigram=+0.0000 诚实零）+ smoke v20 step 39 + closeout（task-30.3）。ADR-035 Accepted（per-D，D3 default flip honest-defer）；jieba-rs 经主 agent R7 chore + ADR-008 add-only；默认构建 0 新 dep + 默认 tokenization 不变（ADR-004）；ADR-014 第二十一次激活；3/3 Done 三门绿自主合，tag/release 待用户授权 ADR-012） |
| 31 | `governance-debt-cleanup` | `docs/specs/phases/phase-31-governance-debt-cleanup.md` | Done | 4 | master（v0.24.0：清跨 Phase 治理债 — Go fallback `MemMemoryStore` memory 变更 emit `memory.*` event 对齐 workspace/job + Rust 路径 + event-bus partition/capacity **经核 Phase 26 已交付** verify-only + roadmap §4 add-only 更正（task-31.1，🟢，不重复实现 ADR-013）+ embedding-cache LRU + Go memstore cap 可配置 + compose `mem_limit`/`cpus`/可选 TLS proxy（task-31.2，🟢 / 🟡 真实 cert）+ eval case-results 子表（add-only migration 0018）+ exporter `content=""` 经新 `ListAllChunks` RPC 真实全文 + 3 MCP nits + C2/C3/C4 诚实延后重申（task-31.3，🟢）+ smoke v21 step 40 + closeout（task-31.4）。ADR-036 Accepted（per-D，D2 真实 cert / D4 native-runner·attestation honest-defer）；ADR-021/027/029/033 add-only Amendment（不溯改正文 D5）；默认行为 + 既有契约（proto/migration add-only、cache 默认值、compose 可选）不变（ADR-004）；ADR-014 第二十二次激活；4/4 Done 三门绿自主合（#206-#208 + closeout），tag/release 待用户授权 ADR-012） |
| 32 | `vector-backend-config-plumbing-and-completeness` | `docs/specs/phases/phase-32-vector-backend-config-plumbing-and-completeness.md` | Done | 4 | plan/phase-32-vector-config-plumbing（v0.25.0 规划：承 Phase 29 把 `select_vector_backend` 工厂从「仅默认接线」补全 — backend config plumbing 两热路径（task-32.1，env→server.rs hybrid/semantic，未设/"" → BruteForce byte-equivalent）+ factory sqlite-vec arm + in-process 选择矩阵 wiring（task-32.2，🟢 wiring / 🟡 矩阵 recall·latency cell 须 MSVC feature build honest-defer）+ console_data_plane `SearchResultItem` add-only `vector_score=16` provenance（parity v1 search proto）+ retrieval-filter 契约诚实化（task-32.3，`mod.rs:325` 误导性 WARN → 准确 no-op + real chunk filter 新 backlog）+ closeout（task-32.4）。ADR-037 Accepted（per-D，D2 sqlite-vec 矩阵 cell honest-defer）；默认 0-vector-dep baseline + 既有契约不变（ADR-004/023）；sqlite-vec 矩阵 cell / real chunk source_type·agent_scope filter feature honest-defer（ADR-013）；实现 + tag/release 经用户授权 ADR-012） |
| 33 | `governance-debt-cleanup-2` | `docs/specs/phases/phase-33-governance-debt-cleanup-2.md` | Done | 4 | master（v0.26.0：第二轮治理债清扫，镜像 Phase 31 — L2 SQLite embedding-cache rowid-FIFO 有界（task-33.1，#218，0-dep/0-migration，opt-in-path 防御）+ memstore FIFO→access-order LRU + hard-delete no-dangling-ref 不变式（task-33.2，#219，cascade 经核非问题 honest-defer / handleMemoryPin lenient ADR-022 D2 据实不改）+ observability indexing.* 持久化(migration 0019)+replay mapper + TraceStore 多 workspace 隔离 add-only proto + drain verify-only（task-33.3，#220，🟢/🟡）+ export --timeout + closeout（task-33.4，smoke v23 [42/42]；%v→%w non-bug / tracestore-fts 已修复 / datadir env→Options honest-defer）。ADR-038 Accepted（per-D，D3 indexing-replay-e2e + tracestore-isolation-e2e / D4 dropped-nits·datadir 🟡 honest-defer）；ADR-031/027 add-only Amendment（Phase 33，不溯改正文 D5）；默认行为+契约+0-dep 不变（ADR-004/008）；grounding 据实下修多处 survey 过陈（ADR-013）；4/4 Done 三门绿自主合，tag/release 待用户授权 ADR-012） |
| 34 | `vector-config-completeness` | `docs/specs/phases/phase-34-vector-config-completeness.md` | Done | 3 | master（v0.27.0：承 Phase 32 把 env-plumbing 补全为 dim 真实协商 + config 文件选 backend，刻意小版本（Phase 31/33 后绿区 backlog 已薄，honest over padding ADR-013）— vector-dim-auto-negotiation（task-34.1，`factory.rs` `negotiate_vector_dim` 替 `let _ = dim` + `VectorBackend::expected_dim` DEFAULT None，默认 BruteForce dim-agnostic no-op honest-caveat byte-equivalent，feature backend 真实强制续 `[SPEC-DEFER:phase-future.vector-dim-feature-enforce]`）+ vector-backend-config-file（task-34.2，Go `config.toml` `[vector]` 段 → `setVectorEnv` 跨进程 env-bridge 仿 `CONTEXTFORGE_DATA_DIR`，env-wins、无段=byte-equiv、Rust 0-dep 保留，daemon.Options.DataDir 字段重构续 `[SPEC-DEFER:phase-future.daemon-options-datadir]`）+ closeout（task-34.3，smoke v24 [43/43] + TestTask343 + grounding 校正 `get_source_chunk` workspace 隔离经核已实存自 task-12.2 verify-only 不变式记录、survey 高估为 gap）。ADR-039 Accepted（D1 dim 协商 / D2 config-file env bridge / D3 get_source_chunk 已实存 verify-only grounding 校正 / D4 默认 0-dep/0-network + 既有契约不变 ADR-004/008）；ADR-037 add-only Phase 34 Amendment（不溯改正文 D5）；ADR-014 第二十五次激活；实现 + tag/release 经用户授权 ADR-012） |
| 35 | `observability-hardening` | `docs/specs/phases/phase-35-observability-hardening.md` | Done | 3 | master（v0.28.0：承 Phase 31/33 治理债血脉，把热路径中被静默吞掉的真实错误显式化，刻意小版本（第三轮债清理、边际递减，honest over padding ADR-013）— rust-silent-failure-surfacing（task-35.1，`index_session_backend.rs:201` store.append `let _=` 持久化真实错误 + `retriever/mod.rs:415` `Err(_)=>continue` Tantivy/SQLite desync 经 `eprintln!` WARN 显式化镜像 `search.rs:109`，best-effort 保持，`eb.send:193` LEAVE AS-IS no-subscribers）+ go-silent-failure-surfacing（task-35.2，`setVectorEnv` config.Load/Setenv 经 `fmt.Fprintf(os.Stderr)` 显式化镜像 `daemon/rest.go:110`，stderr-capture RED→GREEN，`memstore.go:579` nil-sink 🟡 impl-grounding）+ closeout（task-35.3，smoke v25 [44/44] + TestTask353 + grounding 校正诚实 7→3-4：`search.rs:109`/`server.go:298`(task-31.3)/`allowlist.go:31`(POSIX-only)/`eb.send:193`(no-subscribers) DROP/LEAVE 不改代码）。ADR-040 Accepted（D1 rust surfacing / D2 go surfacing + memstore nil-sink honest non-issue grounding 校正 / D3 grounding 校正 7→3-4 / D4 默认 0-dep/0-network + best-effort 不转 fail-fast ADR-004/008）；ADR-031 add-only Phase 35 Amendment（承 stderr/best-effort surfacing，不溯改正文 D5）；ADR-014 第二十六次激活；实现 + tag/release 经 AskUserQuestion 2026-06-04 用户授权 ADR-012） |
| 36 | `qdrant-live-vector-recall` | `docs/specs/phases/phase-36-qdrant-live-vector-recall.md` | Done | 3 | master（v0.29.0：兑现 ADR-034 D2 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` 真实 live qdrant KNN 召回 — task-36.1 env-gated live recall harness（`core/tests/qdrant_live_recall.rs`，qdrant HNSW ANN recall@k vs BruteForce 精确 KNN，确定性可复现语料 N=2000 dim=64，`health()!=Ready` honest-defer skip 不 fail）+ task-36.2 CI service-container（`ci.yml` `qdrant-recall` job 每次 run 对 live qdrant 验证 recall，永久关闭 CI-no-server defer）+ task-36.3 closeout（smoke v26[45/45] + 真实召回数 release docs + ADR-041 ratify + ADR-034 add-only Phase 36 Amendment 标 D2 fulfilled）。de-risk 已证明 round-trip 可行 + KNN 余弦序正确；qdrant backend 自 Phase 25/29 已全实现 0 行改动只加 harness+CI；默认构建 0 vector dep + 0 新 dep（ADR-004/008）。**CI run 26961084355 实测 recall@10=1.0000**（live KNN==brute-force exact ground truth，N=2000 dim=64 M=50；recall=1.0 因 qdrant 低于 HNSW indexing_threshold 服务精确 KNN 的 live 正确性证明，HNSW 近似域大语料续 `[SPEC-DEFER:phase-future.vector-large-corpus-perf]`）。ADR-041 Accepted；ADR-034 add-only Phase-36 Amendment（标 D2 qdrant-server-lifecycle fulfilled，不溯改 D-body D5）；ADR-014 第二十七次激活；经 AskUserQuestion 2026-06-04 用户授权 ADR-012（C 解锁高价值项 → qdrant live 召回自起 docker + 规划+实现+发版 无人值守），3/3 Done 三门绿合入 master） |
| 37 | `embedding-provider-remote-live` | `docs/specs/phases/phase-37-embedding-provider-remote-live.md` | Done | 3 | master（v0.30.0：兑现 ADR-027 `[SPEC-DEFER:phase-future.embedding-provider-remote]` 真实远程 embedding 端点端到端联调 + 实测语义召回 — task-37.1 env-gated live recall harness（`core/tests/remote_embedding_recall.rs`，`embedding-remote` feature，作者手工标注集 15 case/16 doc 含故意近义干扰，real 模型 vs deterministic 基线同一 `BruteForceVectorBackend` 精确余弦路径 recall@1/@3，`CONTEXTFORGE_REMOTE_API_KEY` 未设 honest-defer skip 不 fail，非网络 well-formed 守护无 key 也跑）+ task-37.2 remote-embedding-config-bridge（Go `RemoteProviderConfig` add-only `Model` + `setRemoteEnv` 跨进程 env-bridge 镜像 setVectorEnv，env-wins，API key env-only 永不进 config，Rust 0 toml dep）+ task-37.3 closeout（smoke v27[46/46] + release docs + ADR-042 ratify + ADR-027 add-only Phase-37 Amendment）。de-risk 已由主 agent 本机真实证明（SiliconFlow + Qwen3-Embedding-8B round-trip + Windows MSVC `--features embedding-remote` 编译跑通；本机实测 remote recall@1=0.8667–0.9333（跨 run 波动）/recall@3=1.0000（稳定）vs deterministic 0.0000/0.0667）；embedding provider 抽象自 Phase 22 已全实现 0 行 provider 核心改动只加 harness+config-bridge+closeout；默认构建 0 网络 / 0 新 dep（`ureq` 自 task-22.3 已 optional，ADR-004/008）。ADR-042 Accepted；ADR-027 add-only Phase-37 Amendment（兑现 embedding-provider-remote，不溯改 D-body D5）；ADR-014 第二十八次激活；经 AskUserQuestion 2026-06-06 用户授权 ADR-012（解锁高价值项 → 远程 embedding live 召回 + 完整 S2V phase + 发版 v0.30.0 无人值守）。3 task 全 Done 三门绿合入 master（#242 harness + #243 config-bridge + 本 closeout）；recall@1 跨 run 波动诚实记录、recall@3 稳定；CI honest-defer（remote 付费 API 无免费 service container，与 qdrant 差异）；v0.30.0 发版经用户授权 ADR-012） |
| 38 | `embedding-remote-reranker-live` | `docs/specs/phases/phase-38-embedding-remote-reranker-live.md` | Done | 3 | master（v0.31.0 已发布：实测 remote MRR=1.0000 recall@1=1.0000（3 runs stable）vs identity MRR=0.4762 recall@1=0.0000、delta_MRR=+0.5238（Qwen3-VL-Reranker-8B via SiliconFlow /v1/rerank，14 case 作者标注集）；#247 task-38.1 + #248 task-38.2 + 本 closeout 三门绿合入 master，ADR-043 Accepted + ADR-026/042 add-only Phase-38 Amendment 标 remote reranker 维度 fulfilled。原规划 plan/phase-38-embedding-remote-reranker-live（v0.31.0 规划：兑现 `[SPEC-DEFER:phase-future.embedding-remote-reranker-live]`（ADR-042 / phase-37 spec §不在范围 / roadmap §3.19 follow-up）真实远程 reranker（cross-encoder over HTTP）端到端联调 + 实测 rerank 质量 + 首次把 reranker 从 config 在生产数据面路径 opt-in 接通 — task-38.1 remote-reranker-provider-and-live-recall（**构建** `core/src/rerank/remote_provider.rs` `RemoteRerankerProvider`（`build_rerank_request_body`/`parse_rerank_response` 纯函数 + ureq POST，镜像 `RemoteEmbeddingProvider` + `CrossEncoderReranker` by-index 映射，Debug 不打印 api_key）+ `core/src/rerank/factory.rs` `select_reranker(name)` 工厂（镜像 `embedding/factory.rs:27-96`，feature-off 显式 Err）+ 新 feature `reranker-remote = ["dep:ureq"]`（0 新 dep）+ `core/tests/remote_rerank_recall.rs`（`#![cfg(feature = "reranker-remote")]`，env-gated `CONTEXTFORGE_RERANKER_API_KEY` honest-defer skip，作者手工标注 query×candidate 集含故意近义干扰，real cross-encoder vs `IdentityReranker` no-semantic-signal 基线 MRR/recall@1 floor MRR_remote>=0.70 且 remote>identity，非网络契约 + well-formed 守护无 key 也跑））+ task-38.2 reranker-config-bridge-and-data-plane-wiring（Go `RerankerConfig`（Enabled/Provider/Endpoint/Model，toml round-trip，无 api-key 字段）+ `setRerankerEnv` 跨进程 env-bridge 镜像 `setRemoteEnv`/`setVectorEnv`，env-wins、无段不导出、API key env-only 永不进 config + Rust 数据面 `reranker_from_env()` 在 `server.rs` hybrid:334/semantic:376 + `data_plane/search.rs` semantic:282 **三处生产路径首次 opt-in 接线**，默认 `CONTEXTFORGE_RERANKER_PROVIDER` unset 字节等价无 rerank 向后兼容，Rust 0 toml dep）+ task-38.3 closeout（smoke v28[47/47] + release docs + ADR-043 ratify + ADR-026 add-only Phase-38 Amendment + ADR-042 add-only Phase-38 Amendment）。与 Phase 37 核心差异：本 phase **构建** provider（`RemoteEmbeddingProvider` 自 Phase 22 已全实现，但 `RemoteRerankerProvider` / `select_reranker` 工厂从无）且**数据面首次 opt-in 接线**（三处此前只调 `.with_embedder().with_vector_searcher()`、从不调 `.with_reranker()`，reranker 仅 Phase 21 builder seam 仅测试用）。de-risk 已由主 agent 本机真实证明（SiliconFlow `/v1/rerank` + `Qwen/Qwen3-VL-Reranker-8B` round-trip，`config_save relevance_score=0.7356` 排 #1 约 46x 区分度，HTTP 200，排序语义正确）；默认构建 0 新 dep（`ureq` 自 task-22.3 已 optional）+ 0 network + 0 proto + 0 migration（ADR-004/008）。ADR-043 Proposed（per-D，D2 真实 MRR/recall 本机已认证 run honest-defer，CI honest-defer 复用 `embedding-remote-ci-credential`）；ADR-026 add-only Phase-38 Amendment（兑现 remote reranker 维度，不溯改 D-body D5）+ ADR-042 add-only Phase-38 Amendment（标 follow-up fulfilled）；大语料 rerank 质量续 `[SPEC-DEFER:phase-future.reranker-large-corpus-quality]`、多 provider rerank live 续 `[SPEC-DEFER:phase-future.embedding-multi-provider-live]`；ADR-014 第二十九次激活；实现 + tag/release 经用户授权 ADR-012） |
| 39 | `console-api-retrieval-signal-forward` | `docs/specs/phases/phase-39-console-api-retrieval-signal-forward.md` | Done | 3 | -（v0.32.0 已落地（#252/#253/closeout，master）：兑现 ADR-025 `[SPEC-DEFER:phase-future.console-api-hybrid-forward]`（对外 console-api `?hybrid=true` REST 转发）并据 ADR-043 D3 **重界定** `[SPEC-DEFER:phase-future.console-api-rerank-forward]` — 把已存在于内核但对外 REST 不可达/不可见的 hybrid 融合（Phase 21 `server.rs` hybrid 路径 + `search_hybrid` + `hybrid_score`）+ rerank `reason` provenance（Phase 38 数据面 opt-in）经 console_data_plane proto + 数据面 dispatch + Go console-api 转发**首次贯通到对外 `POST /v1/search`** — task-39.1 console-dataplane-hybrid-proto-and-dispatch（proto add-only `SearchRequest.hybrid=8` + `SearchResultItem.hybrid_score=17` 既有字段号冻结 ADR-015 D1 + `buf generate` + `data_plane/search.rs` `query()` hybrid dispatch 分支镜像 `server.rs` hybrid 路径 + 数据面 semantic 分支结构 + `hybrid_score` 填充镜像 `vector_score`）+ task-39.2 console-api-hybrid-forward-and-rerank-visibility（`contractv1` add-only `Hybrid`/`HybridScore` 镜像 `Semantic`/`VectorScore` + `handleSearch` `?hybrid` OR-merge 镜像 `?semantic` + `grpcclient` 转发/映射 + rerank `reason` provenance 对外 REST 可见）+ task-39.3 closeout（smoke v29[48/48] + release docs + README:350 措辞替换 + ADR-044 ratify + ADR-025/043 add-only Phase-39 Amendment + roadmap §3.21/§4 + adapter + defer marker 更新）。**0 backend 算法改动「贯通而非重写」**：复用 `search_hybrid`/`reranker_from_env`/`?semantic` 范式/`vector_score` provenance 范式；默认 `hybrid=false` 字节等价 + reranker unset 字节等价 + proto add-only 既有契约不变 + 0 新 dep（ADR-004/008/015）。**诚实校正（ADR-013）**：`?rerank=true` per-request 与 ADR-043 D3 env 驱动冲突 → 记为 superseded、不实现，改交付 rerank provenance 可见性。ADR-044 Proposed；ADR-025/043 add-only Phase-39 Amendment；ADR-014 第三十次激活。Phase 39 实现 + 发版须另行 ADR-012 授权（本批为规划稿，Draft/Proposed）） |
| 40 | `governance-debt-cleanup-3` | `docs/specs/phases/phase-40-governance-debt-cleanup-3.md` | Done | 3 | master（v0.33.0 已落地：第三轮治理债清扫，镜像 Phase 31/33 — 清两组 code-local 真实治理 marker：**memory pin actor 透传**（`pin()` 硬编码 `"console-api"`，因 `PinMemoryRequest` 无 actor field / Go `MemoryStore.Pin` 无 actor 参数 / `handleMemoryPin` 不读调用方；`set_pinned_with_actor` store 层本就接受 actor task-27.1/ADR-032 D1，仅入口透传链缺）+ **L2 embedding 缓存访问序 LRU**（Phase 33 D1 给 L2 加 rowid-FIFO 插入序驱逐但 `sqlite_get` 命中不重排）— task-40.1 memory-actor-propagation（`PinMemoryRequest` add-only `actor=3` 既有 memory_id=1/pin=2 字段号冻结 ADR-015 D1 + buf generate + Go `MemoryStore.Pin(id,pin)` → `Pin(id,pin,actor)` interface + `memoryClient.Pin`/`MemMemoryStore.Pin` 两实现 + `grpcclient` 填 `pb.PinMemoryRequest.Actor` + `handleMemoryPin` 读 `r.Header.Get("X-Actor")` 缺省空串 + Rust `pin()` `set_pinned_with_actor(.., if req.actor.is_empty() { "console-api" } else { req.actor.as_str() })` 空回落 byte-equiv；认证身份 honest-defer `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`；ADR-022 D2 宽松 body 契约不改）+ task-40.2 l2-embedding-cache-true-lru（`core/src/embedding/cache.rs` `sqlite_get` 命中时仅 `l2_cap>0` `INSERT OR REPLACE` 原样回写命中行 bump 隐式 rowid 到表尾 → 既有 `sqlite_put` rowid 序驱逐由插入序 FIFO 升访问序 LRU，cap==0 不 bump，复用既有隐式 rowid 0 schema migration，据实更正 Phase 33「真 LRU 须加 created_at 列+ALTER」假设、与 Go memstore move-to-front 同技法；命中 bump 写放大 + `with_sqlite` 无生产调用点现网零影响据实记）+ task-40.3 closeout（smoke v30[49/49] + release docs + ADR-045 ratify + ADR-032 add-only Phase-40 Amendment（pin actor 透传维度兑现）+ ADR-038/027 add-only Phase-40 Amendment（L2 true-LRU 维度兑现 + 真-LRU 假设据实更正）+ ADR-015 add-only Amendment + roadmap §3.22/§4 + adapter + defer marker 更新）。**0 新依赖、复用既有范式**：actor 透传复用 `set_pinned_with_actor` 显式 actor 参数 + `r.Header.Get` 范式；L2 访问序 LRU 复用既有隐式 rowid + Go memstore move-to-front 技法；默认 pin actor proto/Go 参数 add-only 空回落 byte-equiv + L2 命中 bump 仅有限 cap 生效 cap==0 byte+perf-equiv + proto add-only 既有契约不变 0 migration（ADR-004/008/015）。诚实校正（ADR-013）：pin actor 调用方透传 vs 认证身份 honest-defer；L2 真-LRU 据实更正 Phase 33 假设 + 写放大/opt-in-path 现网零影响据实记；其余 marker（vector-dim-feature-enforce 须 feature build / tracestore-multi-workspace-strict 余下读路径 / chunk-source-type-filter 须 import-path migration）据实保持延后不强行扩面。ADR-045 Accepted（per-D ratify）；ADR-032/038/027/015 add-only Phase-40 Amendment；ADR-014 第三十一次激活。经用户 AskUserQuestion 2026-06-07 授权 ADR-012（C 第三轮治理债清扫 + 规划+实现+发版无人值守）；#256 规划 + #257 task-40.1 + #258 task-40.2 + 本 closeout 三门绿合入 master（68046c3 / 08e8db6），多 agent 对抗审查 4 维度 × 每 finding 3 skeptic 核实 0 真实缺陷；TEST-40.1.1-4 + TEST-40.2.1-2 全绿；真实 v0.33.0 tag/release 经用户授权 push（ADR-012），tag SHA/digest/tlog post-tag-push 回填 ADR-013 不预填） |
| 42 | `chunk-source-type-filter` | `docs/specs/phases/phase-42-chunk-source-type-filter.md` | Draft | 0 | plan/phase-42-chunk-source-type-filter（v0.35.0 规划：把 chunk 检索的 `source_type` 过滤从 Phase 32（task-32.3 / ADR-037）据实记的 documented no-op 落地为真实过滤 — task-42.1 chunk-source-type-derivation-and-filter（`core/src/retriever/mod.rs` add `classify_source_type(file_path) -> &'static str` 扩展名确定性桶 code/doc/config/other 镜像 `indexer::lang_hint_from_path` + 三构造点 `search()` BM25/`get_chunk`/`search_semantic` `source_type` 由 `DEFAULT_SOURCE_TYPE=""` 改真实派生 + `search()` BM25 加 source_type post-filter 镜像 `:386` language post-filter 空 filter byte-equiv + `agent_scope` 续 documented no-op（窄化 no-op 块仅 agent_scope）；**0 schema migration**（source_type 由 file_path 派生、chunks/files/provenance §5.3 保持 FROZEN）；v1 `server.rs:440-453` 已映射 proto `filters.source_type` → retriever 真实过滤后 v1 gRPC/REST body 立即生效；据真契约改写 TEST-32.3.2）+ task-42.2 console-api-source-type-forward（`console_data_plane.proto` `SearchRequest` add-only `repeated string source_type = 9` 既有字段 1-8 冻结 ADR-015 + buf generate + `data_plane/search.rs` 按 `req.source_type` 对 populate 后 hit post-filter 覆盖 BM25/semantic/hybrid 一致 + Go `contractv1.SearchRequest` add-only `SourceType []string` + `handleSearch` `?source_type=` query/body 并集 forward 镜像 `?semantic`/`?hybrid` + grpcclient 映射；console 响应 `source_file_type=5` 响应侧已就绪 populate 后立即显示真实值）+ task-42.3 closeout（smoke v32[51/51] REAL source_type 真实过滤端到端 + ADR-047 ratify + ADR-037 add-only Phase-42 Amendment + roadmap §3.24/§4 + adapter）。**关键诚实校正（ADR-013）**：`chunk-source-type-filter` 与 `chunk-agent-scope-filter` 经 grounding **不对称**——source_type 可由 file_path 确定性派生（0 migration）真实落地；`agent_scope` 是 memory 层概念（`memory_items` 0013 / `ListMemory` scope / `memstore.go:629-635`）、chunks 无 agent 关联、无可派生维度 → 续 documented no-op + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`（不伪造，镜像 Phase 32/34/35 grounding 校正手法）。空 filter byte-equiv + proto add-only 既有契约不变 + 0 新 dep（`classify_source_type` 纯 std）+ 0 网络 + 0 schema migration（ADR-004/008/015）。source_type value 由空串变真实派生值系填补 task-4.2 §2A v0.1 schema gap（契约本意）由 ADR-047 据实记。ADR-047 Proposed；ADR-037 add-only Phase-42 Amendment（source_type no-op supersede / agent_scope no-op 保持）；ADR-014 第三十三次激活。Phase 42 实现 + 发版须另行 ADR-012 授权（本批为规划稿，Draft/Proposed）） |
| 41 | `tokenizer-default-on` | `docs/specs/phases/phase-41-tokenizer-default-on.md` | Done | 3 | master（v0.34.0 已实现待发版：做出 Phase 30 / ADR-035 D3 据「翻默认是产品决策」诚实延后的产品决策——把 code/CJK tokenizer `code_cjk`（task-24.1，纯 std 0-dep bigram + 代码符号拆分）从 opt-in 翻为**新建 collection 生产默认**，全体用户默认获实测 recall 增益 — task-41.1 tokenizer-default-on（`core/src/server.rs` add `resolve_tokenizer()` env-resolution 镜像 `resolve_data_dir`/`resolve_vector_backend`：unset→`code_cjk` 翻默认 / `"default"`→opt-out 回 legacy `TEXT` / `"code_cjk"`/`"cjk_segmenter"`(feature) passthrough / unknown·feature-off→stderr WARN+`code_cjk` 不静默落 TEXT + 生产索引两调用点 `server.rs:141` CoreService::index + `jobs/index_session_backend.rs:151` 改 `open_with_tokenizer(.., &resolve_tokenizer())`；`IndexSession::open`/`DEFAULT_TOKENIZER` 库 API+常量不动；既有 collection 经 `open_in_dir` 读回持久化 schema 自动安全不被静默失效；Phase 24 harness 复测真实 recall delta +0.0909）+ task-41.2 tokenizer-config-bridge（Go `internal/config/config.go` add-only `RetrievalConfig{Tokenizer}` + `[retrieval]` 段 round-trip 镜像 `VectorConfig`/`[vector]` + `cmd/contextforge/main.go` `setTokenizerEnv` 镜像 `setVectorEnv`：`[retrieval] tokenizer` 非空且 env 未设→导出 `CONTEXTFORGE_TOKENIZER`，env-wins、无段不导出→Rust 默认 `code_cjk`，tokenizer 非密钥，Rust core 0 toml dep）+ task-41.3 closeout（smoke v31[50/50] production 默认 code_cjk + `CONTEXTFORGE_TOKENIZER=default` opt-out 端到端 + ADR-046 ratify + ADR-029/035 add-only Phase-41 Amendment + roadmap §3.23/§4 + adapter）。**关键诚实定性（ADR-013）**：项目**首次刻意改默认行为**（新建 collection 倒排词项 `TEXT`→`code_cjk` 非 byte-equiv）——由 ADR-046 显式承接 + 三重安全（既有 collection 不受影响 / `CONTEXTFORGE_TOKENIZER=default`·`[retrieval]` opt-out 回 legacy byte-equiv / 既有 collection 不自动迁移用户经 `reindex_with_tokenizer` 主动）+ Phase 24 实测 +0.0909 justify；jieba `cjk_segmenter` 默认不取（0-dep baseline + Phase 30 实测 jieba vs bigram delta=+0.0000）。0 新 dep（`code_cjk` 纯 std）+ 0 network（ADR-004/008）。ADR-046 Accepted（per-D ratify）；ADR-029/035 add-only Phase-41 Amendment；ADR-014 第三十二次激活。#262 task-41.1（35bb421）+ #263 task-41.2（2cead8b）+ closeout 三门绿合入 master；**实测 recall delta +0.1250 recall@5/@10**（default 0.8750 → code_cjk 1.0000 over 当前 16-题 golden，与 ADR-035 Amendment D4 一致；Phase 24 原始 11-题 golden 为 +0.0909）；TEST-41.1.1/.2 + TEST-41.2.1/.2 全绿；smoke v31[50/50]（REAL camel 子词 `runner`(of JobRunner) 经 code_cjk 命中、TEXT 会 miss，distinguishing）。真实 v0.34.0 tag/release 经用户授权 push（ADR-012），tag SHA/digest/tlog post-tag-push 回填（ADR-013 不预填）） |

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
| 25.1 | core/src/retriever/vector/qdrant.rs 生命周期层（connection-config validate + health-probe + decide_ensure 纯函数契约层 deterministic 单测）| docs/specs/tasks/task-25.1-qdrant-server-lifecycle.md | Done | Phase25 #1（dep task-18.4 QdrantBackend spike；契约层不需 live server；可与 25.2 并行）| master（config/health/decide_ensure 契约层 4/4 不连 live server；open() ensure-create 重写；live KNN 诚实延后 [SPEC-DEFER:phase-future.qdrant-server-lifecycle]；0 新 dep）|
| 25.2 | core/src/retriever/vector/lance_db.rs 真实可构建性调查（dev-box cargo build protoc 前置三态）+ 索引调参参数校验 | docs/specs/tasks/task-25.2-lancedb-buildability-and-index-tuning.md | Done | Phase25 #2（dep task-18.5 LanceDbBackend spike；仿 task-23.2 sqlite-vec MSVC 调查 pattern；可与 25.1 并行）| master（🟢 cargo build --features vector-lancedb exit 0 @ x86_64-pc-windows-msvc，protoc via 仓内 protoc-bin-vendored，1097 rlib，0 新 dep；LanceIndexTuning::validate + backend 契约 lib 2/2；广义 feature 全 target 测试 rustc ICE caveat；真实 ANN 索引性能延后）|
| 25.3 | 生产 backend 选择矩阵 + console_smoke v15 + v0.18.0 closeout + ADR-030 ratify + ADR-023 add-only Amendment | docs/specs/tasks/task-25.3-closeout-v0.18.0.md | Done | Phase25 #3（dep 25.1+25.2；收口）| master（生产 backend 选择矩阵（hnsw/sqlite-vec/lancedb/qdrant + caveat）+ smoke v15 step 34 + v0.18.0 release docs + ADR-030 Accepted + ADR-023 D3/D4 add-only Amendment + phase-25 §6 闭合）|
| 26.1 | core/src/data_plane/search_persist.rs TraceStore FTS5 shadow 表 + prune_older_than/VACUUM（migration 0016 add-only，bundled SQLite 0 新 dep）| docs/specs/tasks/task-26.1-tracestore-fts-and-vacuum.md | Done | Phase26 #1（dep Phase16 task-16.1 SqliteTracePersist；可与 26.2 并行）| master（#178：search_fts quoted-phrase MATCH + vacuum/prune_older_than + open 旧库回填 + put FTS 同步；10/10 search_persist 单测；0 新 dep）|
| 26.2 | events SSE 推送（GET /v1/observability/events/stream，Go http.Flusher add-only）+ audit_log 重放 + event-bus 容量/drain 配置 | docs/specs/tasks/task-26.2-events-sse-push-and-replay.md | Done | Phase26 #2（dep Phase16 task-16.2 long-poll + Phase11 task-11.4 EventBus + ADR-021；可与 26.1 并行）| master（#179：proto add-only since_ts/last_event_id + replay_events_from_audit id ASC + handleEventsStream SSE 帧 + grpcclient.Stream；Rust 2/2 + Go SSE 4 契约；live e2e 诚实延后）|
| 26.3 | console_smoke v16 + v0.19.0 closeout + ADR-031 ratify + ADR-021/015 add-only Amendment | docs/specs/tasks/task-26.3-closeout-v0.19.0.md | Done | Phase26 #3（dep 26.1+26.2；收口）| master（event-bus EventBus::from_config 容量/分区/drain 配置 events 6/6 + drain 5/5 + smoke v16 step 35 + v0.19.0 release docs + ADR-031 Accepted + ADR-021 add-only Amendment + phase-26 §6 闭合）|
| 27.1 | proto add-only pinned_by(string)/pinned_at_unix(int64) + core/src/memory/store.rs 写穿 + audit 回填 | docs/specs/tasks/task-27.1-memory-pin-actor-and-timestamp.md | Done | Phase27 #1（dep Phase13 task-13.1 MemoryService + Phase17 task-17.1 is_pinned + ADR-022 §Trade-offs 三 marker；proto add-only 不破冻结）| master（#181：MemoryItem field 11/12 + 0017 guarded ALTER + set_pinned_with_actor + console_message_fields freeze guard；store 15/15 + data_plane 14/14 + proto_contract 6/6）|
| 27.2 | proto add-only Unpin/HardDelete RPC + Pin/Unpin 显式拆分 + hard-delete X-Confirm（复用 confirmMiddleware，ADR-017 D2）| docs/specs/tasks/task-27.2-memory-pin-unpin-split-and-hard-delete.md | Done | Phase27 #2（dep 27.1；串行 proto add-only）| master（#183：Unpin/HardDelete RPC + store.hard_delete 物理删除 + MemoryHardDelete audit + console-api 412→204→404；store 14/14 + data_plane 14/14 + go consoleapi）|
| 27.3 | console_smoke v17 + v0.20.0 closeout + ADR-032 ratify | docs/specs/tasks/task-27.3-closeout-v0.20.0.md | Done | Phase27 #3（dep 27.1+27.2；收口）| master（is_pinned audit backfill reconcile_is_pinned_from_audit + smoke v17 step 36 + v0.20.0 release docs + ADR-032 Accepted + ADR-022 add-only Amendment + phase-27 §6 闭合）|
| 28.1 | `verify-image.yml` 未鉴权（logout 后）匿名 pull 守护（守 v0.10.0 PRIVATE→403，run 26788773926 verified）+ multi-arch（arm64）emulation 实测不可行（run 26757640892 45min 超时）→ 延后原生 runner，release.yml 净零回退 | docs/specs/tasks/task-28.1-multi-arch-image-and-anonymous-pull.md | Done | Phase28 #1 | master |
| 28.2 | `release.yml` cosign keyless sign（签 digest）+ cosign attest SPDX SBOM（syft）+ build-push SLSA provenance:max + `verify-image.yml` cosign verify + verify-attestation（GitHub 原生 attestation 因私有仓库不可用 run 26789731232→改 cosign，ADR-033 §D2 原文；机制 run 26799480280 verified，真签 @ v0.21.0 release） | docs/specs/tasks/task-28.2-image-signing-sbom-provenance.md | Done | Phase28 #2 | master |
| 28.3 | `ci.yml` 加 lint job（clippy -D warnings + gofmt + go vet 三阻断）；实测存量 gofmt **15 真实**(CI/LF 暴露；本机 96=15 真实+81 CRLF，初判误断 0 被 CI 纠正)/go vet 0/clippy ~33 → 全修到全绿（gofmt -w+strip管道 / clippy fix+手动+2 targeted allow）+ cargo test 不退化 | docs/specs/tasks/task-28.3-ci-strict-lint.md | Done | Phase28 #3 | master |
| 28.4 | v0.21.0 closeout：smoke v18 step 37 + release docs（evidence/artifacts/README/RELEASE_NOTES）+ ADR-033 据 D1-D4 真实 ratify Accepted（D1 arm64 DEFERRED/D2 cosign 机制验证·真签@release/D3 lint 门绿）+ ADR-007 add-only Amendment + phase-28 §6 闭合 + feature；真实 v0.21.0 tag/release 待用户授权 | docs/specs/tasks/task-28.4-closeout-v0.21.0.md | Done | Phase28 #4（dep 28.1+28.2+28.3；收口）| master |
| 29.1 | `core/src/retriever/vector` `select_vector_backend(name, dim)` 工厂（仿 `embedding/factory.rs::select_provider`：默认/空→BruteForce、qdrant/lancedb feature-gated 否则诚实 Err）+ `server.rs:302`(hybrid)/`:341`(semantic) 替换硬编码 BruteForce；兑现 `[SPEC-DEFER:phase-future.vector-retrieval-integration]`（🟢 deterministic 不连 server）| docs/specs/tasks/task-29.1-vector-backend-factory-and-hotpath-injection.md | Done | Phase29 #1（dep task-25.1/25.2 backend + task-22.1 工厂范式 + ADR-034 D1）| master（#197；factory 4/4 + workspace 191+ 0 failed）|
| 29.2 | qdrant live 端到端 KNN harness（克隆 `phase20_recall_via_retriever.rs`，`QdrantBackend::connect(from_env)`，feature vector-qdrant+embedding-fastembed）+ 无 server `health()==Unreachable` honest-defer（不伪造召回 ADR-013）+ 单节点部署基线文档；首次兑现 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` live 维度（🔴 live server / 🟢 wiring）| docs/specs/tasks/task-29.2-qdrant-live-knn-and-recall-harness.md | Done | Phase29 #2（dep 29.1 工厂 qdrant 臂 + Phase25 qdrant 契约层 + 真实 server 或 honest-defer）| master（#198；feature build + honest-defer exit 0 实测；live-recall 维度延后）|
| 29.3 | lancedb 真实 IVF_PQ/HNSW 索引建图 + 实测召回（`LanceIndexTuning`，feature vector-lancedb，兑现 `[SPEC-DEFER:phase-future.lancedb-index-tuning]`）+ 多 backend 选择矩阵真实测量 → ADR-030 D3/ADR-023 tier add-only Amendment；compaction/大语料/CI 构建 ICE 诚实延后（🟡 feature build / 🔴 大语料）| docs/specs/tasks/task-29.3-lancedb-ann-index-tuning-and-backend-matrix.md | Done | Phase29 #3（dep 29.1 工厂 lancedb 臂 + Phase25 lancedb 参数层 + vector-lancedb 可构建环境）| master（#199；--lib 4/4：真实 IVF_PQ/IVF_HNSW_SQ + compaction + 矩阵）|
| 29.4 | v0.22.0 closeout：smoke v19 + release docs + ADR-034 据 D1-D5 真实 ratify（live-server/大语料受阻维度据已达维度 ratify 如实）+ ADR-030/023 add-only Amendment + phase-29 §6 闭合 + adapter + feature；真实 v0.22.0 tag/release 待用户授权 | docs/specs/tasks/task-29.4-closeout-v0.22.0.md | Done | Phase29 #4（dep 29.1+29.2+29.3；收口）| master（this PR；smoke v19 step 38 + release docs + ADR-034 per-D ratify）|
| 30.1 | `core/Cargo.toml` `cjk-segmenter` feature + optional dep（jieba-rs/lindera，NOTE 经主 agent R7 chore + ADR-008 add-only）+ `core/src/indexer/mod.rs` 并行 `cjk_segmenter` analyzer（真词边界 `配置加载`→`配置`/`加载` vs bigram）+ 双站点注册（index :442 + query retriever :250）对称；bigram 保留 0-dep fallback；deterministic 真词边界单测（🟢 分词单测 / 🔴 重词典 dep）| docs/specs/tasks/task-30.1-cjk-true-segmenter.md | Done | Phase30 #1（dep Phase24 analyzer seam + Cargo feature recipe + ADR-008 dep add-only）| master（#202；jieba 真分词 2/2 + 双站点 round-trip + 默认 0-dep）|
| 30.2 | tokenizer-default-on 评估 + 既有索引 reindex/migration 工具 + `RetrieverConfig.tokenizer`(:99 现 vestigial) 路由接线或文档化 schema-driven + 扩展 CJK golden（Go `ValidateGoldenSemantic` 校验）+ phase24-harness 量 default vs bigram vs 真分词真实 recall delta（不预填 ADR-013）；迁移过重则诚实延后 default flip `[SPEC-DEFER:phase-future.tokenizer-default-on]`（🟡 recall / 🟢 wiring）| docs/specs/tasks/task-30.2-tokenizer-default-on-and-cjk-recall-delta.md | Done | Phase30 #2（dep 30.1 真分词 analyzer + golden/harness + Go validator）| master（#203；reindex 工具 + 真实 recall delta seg−bigram=+0.0000 诚实零；default flip 延后）|
| 30.3 | v0.23.0 closeout：smoke v20 step + release docs + ADR-035 据 D1-D5 真实 ratify（重词典 dep/小语料/default-on 受阻维度据已达维度 ratify 如实）+ ADR-029 add-only Amendment（真分词升级 + tokenizer-default-on 结论）+ ADR-008 dep note + phase-30 §6 闭合 + adapter + feature；真实 v0.23.0 tag/release 待用户授权 | docs/specs/tasks/task-30.3-closeout-v0.23.0.md | Done | Phase30 #3（dep 30.1+30.2；收口）| master（this PR；smoke v20 step 39 + release docs + ADR-035 per-D ratify + ADR-029 Amendment）|
| 31.1 | Go fallback `MemMemoryStore`（`internal/consoleapi/memstore.go:590-657`）Pin/Deprecate/SoftDelete/Unpin/HardDelete 经 `emitEvent`(:100-115) emit `memory.*` event（对齐 workspace/job + Rust `data_plane/memory.rs:52-106` 已 emit）+ event-bus partition/capacity **经核 Phase 26/ADR-031 D5 已交付**（`events.rs:24-203` + `server.rs:602-603` + TEST-26.3.1a/b/c）verify-only + roadmap §4 add-only 更正（🟢，不重复实现）| docs/specs/tasks/task-31.1-observability-memstore-event-parity.md | Done | Phase31 #1（dep 既有 memstore.go emitEvent + Phase26 event-bus；Rust 侧不动）| master（#206；memstore parity test + cargo events 6 passed verify-only）|
| 31.2 | embedding-cache LRU/cap（`core/src/embedding/cache.rs:23` 无界 HashMap）+ Go memstore cap 可配置（`memstore.go:49` 硬编码 256→config/env）+ compose `mem_limit`/`cpus`（`docker-compose.production.yml`）+ 可选 TLS-terminating proxy（caddy/traefik，真实 cert `[SPEC-DEFER:phase-future.compose-tls-auto-cert]`）（🟢 / 🟡 真实 cert）| docs/specs/tasks/task-31.2-cache-and-deploy-hardening.md | Done | Phase31 #2（dep 既有 cache.rs + memstore.go + deploy compose）| master（#207；cache 5/5 + cap config + compose config parse OK；真实 cert honest-defer）|
| 31.3 | eval case-results 子表 `eval_case_results`（add-only migration 0018，`store.rs` 双写）+ exporter `content=""`（`source.go:85` 根因 v1 search proto 无全文）经新 add-only `ListAllChunks` RPC 真实全文 + 真实 ContentHash + 3 MCP nits（`server.go:187` protocolVersion 白名单 / `:270` audit.Write err 不吞 / `allowlist.go` 文件 mode warn）+ C2/C3/C4（rust-native-eval-runner / multi-arch-native-runner / github-native-attestation）诚实延后重申（🟢）| docs/specs/tasks/task-31.3-eval-exporter-and-mcp-nits.md | Done | Phase31 #3（dep 既有 eval/store.rs + exporter + mcpadapter + proto add-only RPC）| master（#208；eval 12/12 + exporter ListAllChunks 真实 content + 3 MCP nits；C2/C3/C4 honest-defer 重申）|
| 31.4 | v0.24.0 closeout：smoke v21 step + release docs + ADR-036 据 D1-D5 真实 ratify（TLS cert/native runner/attestation 受阻维度据已达维度 ratify 如实）+ ADR-021/027/029/033 add-only Amendment + roadmap §4 event-bus 已交付更正 + phase-31 §6 闭合 + adapter + feature；真实 v0.24.0 tag/release 待用户授权 | docs/specs/tasks/task-31.4-closeout-v0.24.0.md | Done | Phase31 #4（dep 31.1+31.2+31.3；收口）| master（this PR；smoke v21 step 40 + release docs + ADR-036 per-D ratify + ADR-021/027/029/033 Amendment）|
| 32.1 | `core/src/server.rs` hybrid（`:340`）+ semantic（`:382`）两热路径经 env（`CONTEXTFORGE_VECTOR_BACKEND`，仿 `resolve_data_dir` pattern）选 backend，替代硬编码 `select_vector_backend("", 0)`；未设/"" → BruteForce byte-equivalent（默认行为不变 ADR-004 + 0 新 dep） | docs/specs/tasks/task-32.1-vector-backend-config-plumbing.md | Done | Phase32 #1（dep task-29.1 工厂 + server.rs 两热路径 + `resolve_data_dir` env pattern）| plan/phase-32-vector-config-plumbing |
| 32.2 | `core/src/retriever/vector/factory.rs` `select_vector_backend` 加 `"sqlite-vec"` arm（feature `vector-sqlite` 双半 gating，镜像 qdrant/lancedb）+ in-process 选择矩阵 wiring 🟢；矩阵 recall/latency cell 🟡 honest-defer `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`（须本机 MSVC feature build，不伪造数值 ADR-013）；0 新 dep（`sqlite-vec` 既 optional 在树）| docs/specs/tasks/task-32.2-sqlite-vec-factory-arm-and-selection-matrix.md | Done | Phase32 #2（dep task-18.3/23.2 `SqliteVecBackend` + `mod.rs:40` re-export + factory arm pattern）| plan/phase-32-vector-config-plumbing |
| 32.3 | `console_data_plane.proto` `SearchResultItem` add-only `vector_score=16`（parity v1 search proto `vector_score=13`）+ grpcclient wiring 携 provenance + `core/src/retriever/mod.rs:325` 误导性 WARN → 准确 no-op 契约 + 新 backlog `[SPEC-DEFER:phase-future.chunk-source-type-filter]` + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`（chunks 表 §5.3 FROZEN 无该列、`agent_scope` 属 memory 层，real filter 系 import-path feature 非 nit）| docs/specs/tasks/task-32.3-console-provenance-and-retrieval-filter-honesty.md | Done | Phase32 #3（dep console_data_plane proto + v1 search proto parity + retriever filter struct/WARN）| plan/phase-32-vector-config-plumbing |
| 32.4 | v0.25.0 closeout：smoke v22 step [41/41] + release docs + ADR-037 据 D1-D5 真实 ratify（D2 sqlite-vec 矩阵 cell / D4 real chunk filter feature honest-defer 部分 ratify）+ ADR-034 add-only Amendment（sqlite-vec arm 补全工厂）+ ADR-023 守线引用 + roadmap §3.14/§4 add-only + phase-32 §6 闭合 + adapter + feature；真实 v0.25.0 tag/release 经用户授权 | docs/specs/tasks/task-32.4-closeout-v0.25.0.md | Done | Phase32 #4（dep 32.1+32.2+32.3；收口）| plan/phase-32-vector-config-plumbing |
| 33.1 | L2 SQLite embedding-cache rowid-FIFO 有界（`core/src/embedding/cache.rs` sqlite_put L2 `INSERT OR REPLACE` 无界 → COUNT+DELETE ORDER BY rowid，0-dep/0-migration）；`with_sqlite` test-only opt-in-path 防御；true-LRU 须 ALTER `[SPEC-DEFER:phase-future.l2-cache-true-lru]` | docs/specs/tasks/task-33.1-l2-embedding-cache-bound.md | Done | Phase33 #1（dep Phase31 L1 BoundedCache + embedding_cache CREATE TABLE）| plan/phase-33-governance-debt-cleanup-2 |
| 33.2 | console-api memstore FIFO→access-order LRU（`memstore.go` cacheChunk/Trace + read-path move-to-front）+ memory hard-delete no-dangling-ref 不变式测试（cascade 经核非问题 `[SPEC-DEFER:phase-future.memory-harddelete-cascade]`）；剔除 handleMemoryPin strict-400（ADR-022 D2 lenient 据实不改）| docs/specs/tasks/task-33.2-memstore-lru-and-harddelete-invariant.md | Done | Phase33 #2（dep Phase31 memstore cap + Phase27 hard_delete）| plan/phase-33-governance-debt-cleanup-2 |
| 33.3 | observability：indexing.* 事件持久化 add-only migration 0019 + replay mapper 扩展（Phase26 仅 memory.*；mapper 🟢/e2e 🟡 `[SPEC-DEFER:phase-future.indexing-replay-e2e]`）+ TraceStore 多 workspace 严格隔离（add-only `workspace_id` proto 字段 + SQL WHERE，空=aggregate-all 兼容）+ events-drain-timeout verify-only（Phase26 已交付）| docs/specs/tasks/task-33.3-observability-indexing-replay-and-trace-isolation.md | Done | Phase33 #3（dep Phase26 events/replay + search_persist + audit）| plan/phase-33-governance-debt-cleanup-2 |
| 33.4 | `internal/cli/export.go` add-only `--timeout` flag（默认 60s，task-31.3 后两次 daemon spawn）+ v0.26.0 closeout（smoke v23[42/42]+release+ADR-038 ratify+ADR-031/027 Amendment+roadmap/adapter）；剔除 %v→%w（non-bug）/tracestore-fts（已修复）/datadir env→Options honest-defer `[SPEC-DEFER:phase-future.daemon-options-datadir]` | docs/specs/tasks/task-33.4-export-timeout-and-closeout-v0.26.0.md | Done | Phase33 #4（dep 33.1+33.2+33.3；收口）| plan/phase-33-governance-debt-cleanup-2 |
| 34.1 | `core/src/retriever/vector/factory.rs:33-39` `negotiate_vector_dim(dim, backend.expected_dim())` 纯函数替 `let _ = dim`（仿 `embedding/factory.rs:81-96 negotiate_dim`，0=Ok/None-declared=Ok/match=Ok/mismatch=`VectorError::DimMismatch{expected,got}`(types.rs:83 已实存)）+ `VectorBackend` trait（`traits.rs`）add `expected_dim(self)->Option<usize>` DEFAULT None（dim-agnostic）+ `BruteForceVectorBackend` 保 None；honest-caveat 默认 BruteForce 接受任意 dim byte-equivalent（ADR-004），feature backend 真实强制 `[SPEC-DEFER:phase-future.vector-dim-feature-enforce]` | docs/specs/tasks/task-34.1-vector-dim-auto-negotiation.md | Done | Phase34 #1（dep task-29.1 工厂 + task-22.1 `negotiate_dim` 范式 + ADR-039 D1；可与 34.2 并行 Rust vs Go 写路径不相交）| master |
| 34.2 | Go `internal/config/config.go` add-only `[vector]` 段（`Backend string`/`Dim int` toml 标签）+ `setVectorEnv` helper（仿 `cmd/contextforge/main.go:255 setDataDirEnv` 跨进程 env-bridge：`[vector]` 存在且 env 未设时 export `CONTEXTFORGE_VECTOR_BACKEND`/`CONTEXTFORGE_VECTOR_DIM`，spawned core daemon 经既有 `resolve_vector_backend` env 路径接收）；ENV WINS（显式 env 覆盖 config）+ 无段=不 export=unset=BruteForce byte-equiv（ADR-004）；Rust 0-dep 保留（复用 `CONTEXTFORGE_DATA_DIR` 跨进程 env-bridge，非 `daemon.Options.DataDir` 重构 `[SPEC-DEFER:phase-future.daemon-options-datadir]`） | docs/specs/tasks/task-34.2-vector-backend-config-file.md | Done | Phase34 #2（dep 既有 config.go `[collections]`/`[remote]`/`[embedding]` 段 + setDataDirEnv env-bridge + ADR-039 D2；可与 34.1 并行）| master |
| 34.3 | v0.27.0 closeout：grounding 校正 `get_source_chunk` workspace 隔离经核已实存（`core/src/data_plane/search.rs:421-423` 自 task-12.2 按 `req.workspace_id` scope，空=aggregate-all）→ verify-only guard 不变式测试（workspace_id 设=仅该 workspace / 跨 workspace chunk_id=not_found / 空=aggregate）无新代码 + smoke v24 step [43/43]（banner v23→v24，staging cf-v26-cfg，offset +2）+ TestTask343（镜像 TestTask334 无 [37/37]..[42/42] 回归）+ release docs + README v0.27 段 + RELEASE_NOTES v0.27.0 段 + ADR-039 据 D1-D4 真实 ratify + ADR-037 add-only Phase 34 Amendment（不溯改正文 D5）+ roadmap/adapter add-only + feature；真实 v0.27.0 tag/release 经用户授权 | docs/specs/tasks/task-34.3-closeout-v0.27.0.md | Done | Phase34 #3（dep 34.1+34.2；收口）| master |
| 35.1 | rust-silent-failure-surfacing：`core/src/jobs/index_session_backend.rs:201` `let _ = store.append(...)`（indexing-event 持久化真实错误：磁盘满/锁）→ `if let Err(e) { eprintln!("WARN ...: {e}") }`（best-effort 不阻断 indexing）+ `core/src/retriever/mod.rs:415` `Err(_)=>continue`（Tantivy/SQLite desync 静默跳过命中）→ `Err(e) => { eprintln!("WARN retriever: ... desync, skipping: {e}"); continue }`（skip 保持）；镜像 `search.rs:108-113`；`eb.send:193` LEAVE AS-IS（no-subscribers intentional）；Rust eprintln! 输出仓库不断言 → guard/behavior-preservation 测试不伪造 stderr-assert（ADR-013）；0 新 dep | docs/specs/tasks/task-35.1-rust-silent-failure-surfacing.md | Done | Phase35 #1（dep 既有 store.append/retriever match row + search.rs eprintln! 范式 + ADR-040 D1；可与 35.2 并行 Rust vs Go 写路径不相交）| master |
| 35.2 | go-silent-failure-surfacing：`cmd/contextforge/main.go:297` `setVectorEnv` `config.Load` 错误 + `:308` `os.Setenv` 失败静默吞 → `fmt.Fprintf(os.Stderr)` 显式化（镜像 `daemon/rest.go:110`，best-effort env-only 路径失败时不变）；`memstore.go:579` `emitMemoryEvent` nil-sink no-op 🟡 实施期 grounding：production-wired→一次性 `sync.Once` WARN / fallback-only by-design→honest non-issue `[SPEC-DEFER:phase-future.memstore-degraded-observability-warn]`；stderr-capture（`os.Pipe`）RED→GREEN；0 新 dep | docs/specs/tasks/task-35.2-go-silent-failure-surfacing.md | Done | Phase35 #2（dep 既有 setVectorEnv task-34.2 + daemon/rest.go stderr 范式 + ADR-040 D2；可与 35.1 并行）| master |
| 35.3 | closeout-v0.28.0：observability-hardening 7→3-4 grounding 校正如实记录（`search.rs:109` already-surfaced+core 无 metrics facility / `mcpadapter/server.go:298` task-31.3 already-done / `mcpadapter/allowlist.go:31` 有意 POSIX-only / `index_session_backend.rs:193` eb.send 有意 no-subscribers DROP/LEAVE 不改代码，不引新 metrics facility）+ smoke v25 [44/44]（banner v24→v25，staging cf-v27-cfg，offset +2）+ TestTask353（镜像 TestTask343 无 [37/37]..[43/43] 回归）+ v0.28.0 release docs + README v0.28 段 + RELEASE_NOTES v0.28.0 段 + ADR-040 据 D1-D4 真实 ratify + ADR-031 add-only Phase 35 Amendment（不溯改正文 D5）+ roadmap §3.17/§4 add-only + adapter；真实 v0.28.0 tag/release 经用户授权（AskUserQuestion 2026-06-04 ADR-012） | docs/specs/tasks/task-35.3-closeout-v0.28.0.md | Done | Phase35 #3（dep 35.1+35.2；收口）| master |
| 36.1 | qdrant-live-recall-harness：新增 `core/tests/qdrant_live_recall.rs`（`#![cfg(feature = "vector-qdrant")]`，env-gated `QDRANT_URL` 复用 `QdrantConnConfig::from_env()`；`health()!=Ready`→honest-defer skip 不 fail；确定性可复现语料 N=2000 dim=64 index-seeded 单位向量无 `rand`/无 clock；双索引 `QdrantBackend`（ensure-create+index_batch）vs `BruteForceVectorBackend` 精确 ground truth；M=50 query recall@k=mean(\|∩\|/k) 断言 ≥ floor(k=10→0.90) + eprintln 实测数，真实数真实跑出后回填绝不预填 ADR-013）；0 新 dep / 0 migration / 0 默认构建变更 | docs/specs/tasks/task-36.1-qdrant-live-recall-harness.md | Done | Phase36 #1（dep task-25.1 qdrant 生命周期 + task-18.4 backend + task-19.3 BruteForce ground truth + ADR-041 D1；可独立先行 generator 无 server 也跑）| master |
| 36.2 | qdrant-recall-ci-service：`.github/workflows/ci.yml` 加 `qdrant-recall` job（qdrant service container `image: qdrant/qdrant` ports 6334:6334+6333:6333 + toolchain 1.93 + protoc + `QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture`）→ 每次 CI run 对 live service container 验证 recall、永久关闭 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`；CI-only/add-only/默认构建+行为不变；验证证据=PR 自身 live CI run 据实记录 ADR-013 | docs/specs/tasks/task-36.2-qdrant-recall-ci-service.md | Done | Phase36 #2（dep 36.1 harness 文件 + ci.yml feature-build protoc 范式；建议 36.1 先 merge）| master |
| 36.3 | closeout-v0.29.0：smoke v26[45/45]（banner v25→v26，staging cf-v28-cfg，offset +2）+ TestTask363（镜像 TestTask353 无 [37/37]..[44/44] 回归）+ release docs（真实召回数 + 真实 CI run link，`<backfill>`）+ README/RELEASE_NOTES v0.29 段 + ADR-041 据 D1-D4 ratify + ADR-034 add-only Phase 36 Amendment（标 D2 qdrant-server-lifecycle fulfilled，不溯改 D-body D5）+ roadmap §3.18/§4 add-only + adapter + feature；真实 v0.29.0 tag/release 经用户授权 ADR-012 | docs/specs/tasks/task-36.3-closeout-v0.29.0.md | Done | Phase36 #3（dep 36.1+36.2；收口）| master |
| 37.1 | remote-embedding-live-recall-harness：新增 `core/tests/remote_embedding_recall.rs`（`#![cfg(feature = "embedding-remote")]`，env-gated `CONTEXTFORGE_REMOTE_API_KEY`——未设 honest-defer skip 不 fail，api_key 永不记录；作者手工标注语义集 15 case/16 doc 含故意近义干扰；同一 `BruteForceVectorBackend` 精确余弦路径 real remote vs deterministic 基线 recall@1/@3，先 eprintln 再 assert floor r3≥0.70 且 remote@1>det@1；非网络 well-formed 守护无 key 也跑；0 新 dep / 0 migration / 0 默认构建变更；本机真实 run 实测 remote recall@1=0.8667/recall@3=1.0000 vs deterministic 0.0000/0.0667）| docs/specs/tasks/task-37.1-remote-embedding-live-recall-harness.md | Draft | Phase37 #1（dep task-22.3 RemoteEmbeddingProvider + task-22.1 select_provider + task-19.1 Deterministic 基线 + task-19.3 BruteForce ground truth + ADR-042 D1；可独立先行守护无 key 也跑）| - |
| 37.2 | remote-embedding-config-bridge：`internal/config/config.go` `RemoteProviderConfig` add-only `Model` 字段（toml round-trip）+ 新 `setRemoteEnv` 跨进程 env-bridge（镜像 Phase 34 `setVectorEnv`/`setDataDirEnv`：`[remote]` 段存在且 env 未设 → 导出 `CONTEXTFORGE_REMOTE_ENDPOINT/_MODEL/_PROVIDER`，env-wins，无段不导出）接线 doServe+doMCP；API key env-only 永不进 config.toml（setRemoteEnv 不处理密钥）；Rust core 0 toml dep（复用既有 factory env 读取，0 改动 factory.rs）；0 新 dep | docs/specs/tasks/task-37.2-remote-embedding-config-bridge.md | Draft | Phase37 #2（dep setVectorEnv/setDataDirEnv 范式 task-34.2 + RemoteProviderConfig task-1.2 + doServe/doMCP 接线点 + ADR-042 D3；与 37.1 文件解耦可并行）| - |
| 37.3 | closeout-v0.30.0：smoke v27[46/46]（banner v26→v27，staging cf-v29-cfg，offset +2）+ TestTask373（镜像 TestTask363 无 [37/37]..[45/45] 回归）+ release docs（真实 recall 数 + 诚实记 CI honest-defer：remote 付费外部 API 无免费 service container、召回由本机已认证 run 实测，与 qdrant 差异，`<backfill>`）+ README/RELEASE_NOTES v0.30 段 + ADR-042 据 D1-D4 ratify + ADR-027 add-only Phase-37 Amendment（标 embedding-provider-remote fulfilled，不溯改 D-body D5）+ roadmap §3.19/§4 + adapter + feature；真实 v0.30.0 tag/release 经用户授权 ADR-012 | docs/specs/tasks/task-37.3-closeout-v0.30.0.md | Draft | Phase37 #3（dep 37.1+37.2；收口）| - |
| 38.1 | remote-reranker-provider-and-live-recall：**构建** `core/src/rerank/remote_provider.rs` `RemoteRerankerProvider`（`build_rerank_request_body`/`parse_rerank_response` 纯函数 + ureq POST，请求 `{model, query, documents:[字符串], top_n, return_documents:false}` / 响应 `{results:[{index, relevance_score}], meta}`，映射回 `candidates[index]`、set `score=relevance_score`、annotate reason，镜像 `RemoteEmbeddingProvider` + `CrossEncoderReranker` by-index 映射，Debug 不打印 api_key）+ `core/src/rerank/factory.rs` `select_reranker(name)` 工厂（镜像 `embedding/factory.rs:27-96`，"remote" 分支从 env 读 `CONTEXTFORGE_RERANKER_ENDPOINT/_MODEL/_PROVIDER/_API_KEY`、feature-off 显式 Err 不静默）+ 新 feature `reranker-remote = ["dep:ureq"]`（复用既有 ureq，0 新 dep）+ 新增 `core/tests/remote_rerank_recall.rs`（`#![cfg(feature = "reranker-remote")]`，env-gated `CONTEXTFORGE_RERANKER_API_KEY`——未设 honest-defer skip 不 fail，api_key 永不记录；作者手工标注 query×candidate 集（每 query 一个已知相关文档 + 故意近义干扰）；候选喂入统一 / 无相关性先验 score（`IdentityReranker` no-semantic-signal 基线 ≈ chance）；real remote cross-encoder vs identity MRR=mean(1/rank_of_relevant) / recall@1，先 eprintln 再 assert floor MRR_remote>=0.70 且 MRR_remote>MRR_identity；非网络契约 + well-formed 守护（build/parse fixture + select_reranker 路由 + doc id 唯一 / relevant id 存在 / case 数>=12）无 key 也跑；0 新 dep / 0 migration / 0 proto / 0 默认构建变更；真实 MRR/recall 真实跑出后回填绝不预填 ADR-013，de-risk 探针 `config_save relevance_score=0.7356` 排 #1 可引用为可行性证据）| docs/specs/tasks/task-38.1-remote-reranker-provider-and-live-recall.md | Done | Phase38 #1（dep ADR-026 `Reranker` trait + `IdentityReranker` + `CrossEncoderReranker` by-index 映射 + task-22.3 `RemoteEmbeddingProvider` 范式 + `embedding/factory.rs` select_provider 范式 + ADR-043 D1；可独立先行守护无 key 也跑）| - |
| 38.2 | reranker-config-bridge-and-data-plane-wiring：`internal/config/config.go` 新增 `RerankerConfig`（`Enabled`/`Provider`/`Endpoint`/`Model` toml round-trip，无 api-key 字段）+ 新 `setRerankerEnv` 跨进程 env-bridge（镜像 Phase 37 `setRemoteEnv` / Phase 34 `setVectorEnv`：`[reranker]` 段存在且 env 未设 → 导出 `CONTEXTFORGE_RERANKER_ENDPOINT/_MODEL/_PROVIDER`，env-wins，无段不导出）接线 doServe+doMCP；API key env-only 永不进 config.toml（`setRerankerEnv` 不处理密钥）+ Rust 数据面新增 `reranker_from_env()`（读 `CONTEXTFORGE_RERANKER_PROVIDER` → 非空 / 非 none 时 `select_reranker` → `with_reranker`）在 `server.rs` hybrid `:334` / semantic `:376` + `data_plane/search.rs` semantic `:282` **三处生产路径首次 opt-in 接线**（此前只调 `.with_embedder().with_vector_searcher()`、从不调 `.with_reranker()`）；`CONTEXTFORGE_RERANKER_PROVIDER`=identity → provenance marker（`IDENTITY_RERANK_REASON`）/ unset → 无 marker 向后兼容字节等价无 rerank；Rust core 0 toml dep；0 proto / 0 migration / 0 新 dep | docs/specs/tasks/task-38.2-reranker-config-bridge-and-data-plane-wiring.md | Done | Phase38 #2（dep setRemoteEnv/setVectorEnv 范式 task-37.2/34.2 + doServe/doMCP 接线点 + Phase 21 `with_reranker` builder seam + task-38.1 `select_reranker` + ADR-043 D3；config 桥与数据面接线同一 task 端到端，避免「桥而不消费」不诚实）| - |
| 38.3 | closeout-v0.31.0：smoke v28[47/47]（banner v27→v28，staging cf-v30-cfg，offset +2）+ TestTask383（镜像 TestTask373 无 [37/37]..[46/46] 回归）+ release docs（真实 MRR/recall 数 + 诚实记 CI honest-defer：remote 付费外部 API 无免费 service container、rerank 质量由本机已认证 run 实测，复用 `[SPEC-DEFER:phase-future.embedding-remote-ci-credential]`，与 qdrant 差异，`<backfill>`）+ README/RELEASE_NOTES v0.31 段 + ADR-043 据 D1-D4 ratify + ADR-026 add-only Phase-38 Amendment（标 remote reranker 维度 fulfilled，不溯改 D-body D5）+ ADR-042 add-only Phase-38 Amendment（标 `embedding-remote-reranker-live` follow-up fulfilled）+ roadmap §3.20/§4 + adapter + feature；大语料 rerank 质量续 `[SPEC-DEFER:phase-future.reranker-large-corpus-quality]`；真实 v0.31.0 tag/release 经用户授权 ADR-012 | docs/specs/tasks/task-38.3-closeout-v0.31.0.md | Done | Phase38 #3（dep 38.1+38.2；收口）| - |
| 39.1 | console-dataplane-hybrid-proto-and-dispatch：`proto/contextforge/console_data_plane/v1/console_data_plane.proto` add-only `SearchRequest.hybrid=8`（镜像 `v1/search.proto:28`）+ `SearchResultItem.hybrid_score=17`（镜像 `v1 RetrievalResult.hybrid_score=15`，既有字段号 1-7 / 1-16 冻结 ADR-015 D1）+ `buf generate` 重生 Go/Rust stub + `core/src/data_plane/search.rs` `query()` 的 `if req.semantic {..} else {BM25}` 改 `if req.hybrid {..} else if req.semantic {..} else {BM25}`，hybrid 分支镜像 `server.rs` hybrid 路径（:328-376）+ 数据面 semantic 分支结构（hardcoded `DeterministicEmbeddingProvider` + `BruteForceVectorBackend` + `search_hybrid` + `retrieval_method="hybrid"` + 复用 `reranker_from_env` opt-in）+ 结果映射 `hybrid_score: if h.retrieval_method=="hybrid" { h.score } else { 0.0 }`（镜像 `vector_score` :359-363）；默认 `hybrid=false` 字节等价；console 数据面 env-factory backend 续 `[SPEC-DEFER:phase-future.console-data-plane-vector-backend-factory]`；0 新 dep / 0 migration | docs/specs/tasks/task-39.1-console-dataplane-hybrid-proto-and-dispatch.md | Done | Phase39 #1（dep task-21.1 `search_hybrid` + `server.rs` hybrid 路径范本 + task-38.2 `reranker_from_env` + task-32.3 `vector_score` 填充范式 + ADR-015 proto 契约 + ADR-044 D1；可独立先行；#252 合入 @ e5b1172，TEST-39.1.1/39.1.2 PASS）| - |
| 39.2 | console-api-hybrid-forward-and-rerank-visibility：`internal/contractv1/contractv1.go` add-only `SearchRequest.Hybrid bool`（json `hybrid`，镜像 `Semantic` :125）+ `SearchResult.HybridScore float32`（json `hybrid_score`，镜像 `VectorScore` :153）+ `internal/consoleapi/handlers.go` `handleSearch` `if r.URL.Query().Get("hybrid")=="true" { body.Hybrid=true }`（镜像 `?semantic` :452-454）+ `internal/consoleapi/grpcclient/grpcclient.go` `Search` 加 `Hybrid: req.Hybrid`（镜像 `Semantic` :372）+ `protoToSearchResult` 加 `HybridScore: p.HybridScore`（镜像 `VectorScore` :623）；对外 `POST /v1/search`（body `{"hybrid":true}` 或 `?hybrid=true`）贯通 core hybrid 路径；rerank `reason` provenance 经既有 `Reason: p.Reason`（:624）链路对外 REST 可见（reranker 保持 env 驱动、不做 per-request，`?rerank` 据 ADR-044 D3 superseded `[SPEC-DEFER:phase-future.console-api-rerank-forward]`）；默认 hybrid=false 字节等价；0 新 dep / 0 proto 再改 | docs/specs/tasks/task-39.2-console-api-hybrid-forward-and-rerank-visibility.md | Done | Phase39 #2（dep task-39.1 proto 字段 + `buf generate` + task-20.1 `?semantic` 转发范式 + task-32.3 `VectorScore` cross-repo add-only 范式 + ADR-024 + ADR-044 D2/D3；dep 39.1 先在；#253 合入 @ a9cc6bc，TEST-39.2.1/39.2.2 PASS）| - |
| 39.3 | closeout-v0.32.0：smoke v28→v29[48/48]（staging 顺位 offset；端到端断言 `?hybrid=true` → `retrieval_method="hybrid"` / `hybrid_score` + `CONTEXTFORGE_RERANKER_PROVIDER=identity` → rerank `reason` marker 对外 REST 可见，镜像 step [29] 风格）+ TestTask393（镜像 TestTask383 无 [37/37]..[47/47] 回归，`bash -n`）+ release docs（hybrid 贯通 + rerank provenance 可见证据 + README:350「in a later release」措辞替换 + RELEASE_NOTES v0.32 段，tag/run/digest `<backfill>` marker）+ ADR-044 据 D1-D4 ratify + ADR-025 add-only Phase-39 Amendment（标 console-api-hybrid-forward fulfilled，不溯改 D-body D5）+ ADR-043 add-only Phase-39 Amendment（标 console-api-rerank-forward 重界定为 provenance 可见性 fulfilled + `?rerank` per-request superseded，不溯改 D-body D5）+ roadmap §3.21/§4 add-only + adapter + defer marker（console_smoke.sh:49-50 / smoke_syntax_test.go:705-706 / README:350）据实更新 + phase §6 闭合；真实 v0.32.0 tag/release 经用户授权 ADR-012 | docs/specs/tasks/task-39.3-closeout-v0.32.0.md | Done | Phase39 #3（dep 39.1+39.2；收口本 PR；smoke v29[48/48] + TestTask393 + release docs + ADR-044 ratify + ADR-025/043 Amendment；tag/digest post-tag-push 回填）| - |
| 40.1 | memory-actor-propagation：`PinMemoryRequest`（proto:336-339）add-only `string actor = 3`（既有 memory_id=1 / pin=2 字段号冻结，ADR-015 D1）+ `buf generate` 重生 Go/Rust binding + Go `MemoryStore.Pin(id,pin)` → `Pin(id,pin,actor)`（interface + `memoryClient.Pin` grpcclient.go:724-726 / `MemMemoryStore.Pin` memstore.go:653 两实现）+ `grpcclient` 填 `pb.PinMemoryRequest{MemoryId,Pin,Actor}` + `handleMemoryPin`（handlers.go:525-549）读 `r.Header.Get("X-Actor")` 缺省空串透传 + Rust `pin()`（memory.rs:225-229）`set_pinned_with_actor(.., if req.actor.is_empty() { "console-api" } else { req.actor.as_str() })` 空回落 byte-equiv（既有 client / 无 header = 既有硬编码值）；认证身份 honest-defer `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`；ADR-022 D2 宽松 body 契约不改；0 新 dep / proto add-only |  docs/specs/tasks/task-40.1-memory-actor-propagation.md | Done |DONE Phase40 #1（dep ADR-032 D1 `set_pinned_with_actor` store actor 字段 + `r.Header.Get` header 范式 router.go:71 + buf generate + ADR-015 proto 契约 + ADR-045 D1；可独立先行）| - |
| 40.2 | l2-embedding-cache-true-lru：`core/src/embedding/cache.rs` `sqlite_get`（:140-150）命中时仅 `l2_cap > 0` `INSERT OR REPLACE INTO embedding_cache`（同 `(content_hash,provider,dim,vector)`）原样回写命中行 bump 其隐式 rowid 到表尾 → 既有 `sqlite_put`（:153-195）rowid 序驱逐由插入序 FIFO（Phase 33 D1）升访问序 LRU（驱逐最久未用 = 最小 rowid）；cap==0 不 bump（保插入序、零额外写）；复用既有隐式 rowid、0 新 dep / 0 schema migration；据实更正 Phase 33（ADR-038 A2/D4）「真 LRU 须加 created_at 列 + ALTER」假设——与 Go memstore 命中 move-to-front（task-33.2）同技法；命中 bump 写放大 = 访问序 LRU 固有代价 + `with_sqlite` 无生产调用点（Phase 33 D1 已标 opt-in-path）现网零影响据实记 |  docs/specs/tasks/task-40.2-l2-embedding-cache-true-lru.md | Done |DONE Phase40 #2（dep task-33.1 L2 rowid-FIFO + `with_sqlite_capacity` + `DEFAULT_L2_EMBEDDING_CACHE_CAP` + TEST-33.1.1 镜像源 + ADR-038 D1 + ADR-045 D2；与 40.1 并行无依赖）| - |
| 40.3 | closeout-v0.33.0：smoke v29→v30[49/49]（staging 顺位 offset，pin actor 透传 + L2 访问序 LRU；TestTask403 镜像 TestTask393 无 [37/37]..[48/48] 回归，`bash -n`）+ release docs（tag/run/digest `<backfill>` marker）+ ADR-045 据 D1-D3 ratify + ADR-032 add-only Phase-40 Amendment（标 memory-actor-propagation fulfilled + 认证身份续延后）+ ADR-038/027 add-only Phase-40 Amendment（标 l2-cache-true-lru fulfilled + 真-LRU 假设据实更正）+ ADR-015 add-only Amendment（proto add-only field）+ roadmap §3.22/§4 + adapter + defer marker 更新 + phase §6 闭合；真实 v0.33.0 tag/release 经用户授权 ADR-012 |  docs/specs/tasks/task-40.3-closeout-v0.33.0.md | Done |DONE Phase40 #3（dep 40.1+40.2；收口）| - |
| 41.1 | tokenizer-default-on：`core/src/server.rs` add `resolve_tokenizer() -> String`（镜像 `resolve_data_dir` :521-549 / `resolve_vector_backend` :551-560 env-resolution；读 `CONTEXTFORGE_TOKENIZER`：unset/""→`CODE_CJK_TOKENIZER` 翻默认 / `"default"`→`DEFAULT_TOKENIZER` opt-out 回 legacy `TEXT` / `"code_cjk"`→`code_cjk` / `"cjk_segmenter"`→feature 在则 `cjk_segmenter`·缺则 stderr WARN+`code_cjk` / unknown→stderr WARN+`code_cjk` 不静默落 TEXT）+ 生产索引两调用点（`server.rs:141` `CoreService::index` + `jobs/index_session_backend.rs:151`）由 `IndexSession::open(..)` 改 `open_with_tokenizer(.., &resolve_tokenizer())`；`IndexSession::open`（:502 库便捷入口）/ `DEFAULT_TOKENIZER`（:183 常量）不动（向后兼容库调用方 + 既有 indexer/retriever 单测）；既有 collection 经 `open_in_dir`（:528-535）读回持久化 schema 自动安全（不被静默失效）；Phase 24 harness（`phase24_tokenizer_recall.rs`）复测 default `TEXT` vs `code_cjk` 真实 recall delta +0.0909（首次刻意默认变更非 byte-equiv，由 ADR-046 D1/D4 承接，真实数回填不预填）；0 新 dep（`code_cjk` 纯 std；jieba `cjk_segmenter` 仍 feature opt-in） | docs/specs/tasks/task-41.1-tokenizer-default-on.md | Done | Phase41 #1（#262 35bb421；dep task-24.1 `code_cjk`+`open_with_tokenizer` create-vs-open 安全语义 + `resolve_data_dir`/`resolve_vector_backend` env 范式 + Phase 24 harness + ADR-046 D1/D3；TEST-41.1.1/.2 PASS，实测 recall delta +0.1250）| - |
| 41.2 | tokenizer-config-bridge：`internal/config/config.go` add-only `RetrievalConfig{Tokenizer string}` + `Config.Retrieval` + `[retrieval]` 段 encode/decode round-trip（镜像 `VectorConfig` :96-99 / `[vector]` :238-240 / `assignVector` :435-451）+ `cmd/contextforge/main.go` add `setTokenizerEnv`（镜像 `setVectorEnv` :304-346：`[retrieval] tokenizer` 非空且 `CONTEXTFORGE_TOKENIZER` 未设→导出，env-wins、missing config 静默 / 真 parse-err stderr WARN、无段/空值不导出→Rust `resolve_tokenizer` 默认 `code_cjk`）接线 doServe :108-118 / doMCP :150-160；tokenizer 非密钥（无 api-key 字段，与 remote/reranker 桥不同）；Rust core 0 toml dep | docs/specs/tasks/task-41.2-tokenizer-config-bridge.md | Done | Phase41 #2（#263 2cead8b；dep `VectorConfig`/`setVectorEnv` 范式 task-34.2 + doServe/doMCP 接线点 + task-41.1 `CONTEXTFORGE_TOKENIZER` 消费方 + ADR-046 D2；TEST-41.2.1/.2 PASS，Rust core 0 toml dep）| - |
| 41.3 | closeout-v0.34.0：smoke v30→v31[50/50]（staging 顺位 offset；REAL `getUserProfile` 片段 index → 子词 `profile` 默认 `code_cjk` 命中 / `CONTEXTFORGE_TOKENIZER=default` opt-out → 新建 collection `profile` miss 端到端；TestTask413 镜像 TestTask403 无 [37/37]..[49/49] 回归，`bash -n`）+ release docs（tag/run/digest `<backfill>` marker；Upgrade 记翻默认 + opt-out + 既有 collection 不受影响 + reindex 升级，非 byte-equiv 据实）+ ADR-046 据 D1-D4 ratify + ADR-029 add-only Phase-41 Amendment（标默认开启维度 fulfilled）+ ADR-035 add-only Phase-41 Amendment（标 D3 产品决策 fulfilled）+ ADR-004/008 守线引用 + roadmap §3.23/§4 + adapter + defer marker 更新 + phase §6 闭合；真实 v0.34.0 tag/release 经用户授权 ADR-012 | docs/specs/tasks/task-41.3-closeout-v0.34.0.md | Done | Phase41 #3（dep 41.1+41.2；收口：smoke v31[50/50] + TestTask413 + release docs + ADR-046 ratify + ADR-029/035 Amendment + roadmap/adapter；tag/release 待 ADR-012 授权）| - |

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
| 030 | production-vector-backend | Accepted | docs/decisions/adr-030-production-vector-backend.md |
| 031 | observability-hardening | Accepted | docs/decisions/adr-031-observability-hardening.md |
| 032 | memory-ops-hardening | Accepted | docs/decisions/adr-032-memory-ops-hardening.md |
| 033 | release-ci-hardening | Accepted | docs/decisions/adr-033-release-ci-hardening.md |
| 034 | production-vector-live-recall | Accepted | docs/decisions/adr-034-production-vector-live-recall.md |
| 035 | cjk-true-segmenter-and-tokenizer-default | Accepted | docs/decisions/adr-035-cjk-true-segmenter-and-tokenizer-default.md |
| 036 | governance-debt-cleanup | Accepted | docs/decisions/adr-036-governance-debt-cleanup.md |
| 037 | vector-backend-config-plumbing-and-completeness | Accepted | docs/decisions/adr-037-vector-backend-config-plumbing-and-completeness.md |
| 038 | governance-debt-cleanup-2 | Accepted | docs/decisions/adr-038-governance-debt-cleanup-2.md |
| 039 | vector-config-completeness | Accepted | docs/decisions/adr-039-vector-config-completeness.md |
| 040 | observability-hardening | Accepted | docs/decisions/adr-040-observability-hardening.md |
| 041 | qdrant-live-vector-recall | Accepted | docs/decisions/adr-041-qdrant-live-vector-recall.md |
| 042 | embedding-provider-remote-live | Accepted | docs/decisions/adr-042-embedding-provider-remote-live.md |
| 043 | embedding-remote-reranker-live | Accepted | docs/decisions/adr-043-embedding-remote-reranker-live.md |
| 044 | console-api-retrieval-signal-forward | Accepted | docs/decisions/adr-044-console-api-retrieval-signal-forward.md |
| 045 | governance-debt-cleanup-3 | Proposed | docs/decisions/adr-045-governance-debt-cleanup-3.md |
| 046 | tokenizer-default-on | Accepted | docs/decisions/adr-046-tokenizer-default-on.md |
| 047 | chunk-source-type-filter | Proposed | docs/decisions/adr-047-chunk-source-type-filter.md |

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
| 28.1 / 28.2 / 28.3 / 28.4 | test/features/phase-28-release-ci-hardening.feature |
| 29.1 / 29.2 / 29.3 / 29.4 | test/features/phase-29-live-vector-recall.feature |
| 30.1 / 30.2 / 30.3 | test/features/phase-30-cjk-true-segmenter.feature |
| 31.1 / 31.2 / 31.3 / 31.4 | test/features/phase-31-governance-debt-cleanup.feature |
| 32.1 / 32.2 / 32.3 / 32.4 | test/features/phase-32-vector-backend-config-plumbing-and-completeness.feature |
| 33.1 / 33.2 / 33.3 / 33.4 | test/features/phase-33-governance-debt-cleanup-2.feature |
| 34.1 / 34.2 / 34.3 | test/features/phase-34-vector-config-completeness.feature |
| 35.1 / 35.2 / 35.3 | test/features/phase-35-observability-hardening.feature |
| 36.1 / 36.2 / 36.3 | test/features/phase-36-qdrant-live-vector-recall.feature |
| 37.1 / 37.2 / 37.3 | test/features/phase-37-embedding-provider-remote-live.feature |
| 38.1 / 38.2 / 38.3 | test/features/phase-38-embedding-remote-reranker-live.feature |
| 39.1 / 39.2 / 39.3 | test/features/phase-39-console-api-retrieval-signal-forward.feature |
| 40.1 / 40.2 / 40.3 | test/features/phase-40-governance-debt-cleanup-3.feature |
| 41.1 / 41.2 / 41.3 | test/features/phase-41-tokenizer-default-on.feature |
| 42.1 / 42.2 / 42.3 | test/features/phase-42-chunk-source-type-filter.feature |

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
