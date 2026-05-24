# Playbook: Autonomous v0.3 Console Contract v1 Ship via `/goal`

> **不入 git** — 本文件保留为本地 untracked，仅作个人操作 playbook。如要团队复用，移到 `docs/decisions/` 或单独 ADR 化。
>
> 同系列：[autonomous-v0.1-ship.md](autonomous-v0.1-ship.md) / [autonomous-v0.2-cli-pipeline.md](autonomous-v0.2-cli-pipeline.md)

## 用途

一条 `/goal` 命令启动主 agent 自治跑完：

1. ADR-014 §Follow-ups prelude — `scripts/spec_drift_lint.sh` + AGENTS.md §3/§4 cross-validation gate + adapter §Workflow Overrides 更新 + 自合独立 chore PR
2. ADR-015 console-contract-v1-compatibility + Phase 10 顶层 phase spec + 6 task spec (10.1～10.6) 起草 + 各 spec Status Draft→Ready (按 ADR-014 D3 强制 verified by 显式声明) + 自合 spec PR
3. task-10.1 → {10.2 ∥ 10.3} → 10.4 → 10.5 → 10.6 共 6 task 全程实施 + 自合各自 feature PR
4. Phase 10 closeout（按 ADR-014 D1 mapping 表 + D2 lint 输出 + ADR-015 Proposed→Accepted + adapter Phase 10→Done + PRD §Implementation Phases Phase 10 新增）
5. v0.3.0 release（RELEASE_NOTES + evidence + artifacts + git tag v0.3.0 + push）

**本质**：本 /goal 命令是 **user override 当前 ADR-011 红线 #1**（condition 禁含「merged」）+ **R6 merge decision** 自决 + **首次激活 ADR-014 制度全套约束**（D1/D2/D3/D4）。启动它等于命令主 agent 把 §2A Ready review + prelude/spec/6 个 task PR/closeout merge + v0.3.0 tag push 全部交给 evaluator 判 + 自决。

**与 v0.2 playbook 的关键差异**：
- v0.3 涉及 **cross-repo file read**：`H:/devlopment/code/ContextForge-Console/docs/prds/contextforge-console.prd.md` 是 Contract v1 must-have 字段 single source of truth；`H:/devlopment/code/ContextForge-Console/console-api/internal/coreadapter/contractv1/contractv1.go` 是 Go types 镜像基线
- v0.3 首次激活 ADR-014：每个 task spec §6 AC 必须含 `verified by phase-smoke step M` 或 `verified by task-<X.Y> §6 AC M`；spec 内 anti-pattern 词必须 `[SPEC-DEFER:<name>]` 或 `[SPEC-OWNER:<task>]` 标注
- v0.3 引入新资源模型（Workspace + IndexJob）+ SQLite migration + REST endpoints + conformance test，是迄今最复杂的 phase

---

## 启动前 checklist

仓库**任一**符合的基线即可启动（命令本身会自检 + 决定从哪个 E* 起跑）：

| 基线 | 状态特征 | /goal 从哪起 |
|---|---|---|
| **A. 仅 ADR-014 已合（当前 2026-05-24 状态）** | master 含 ADR-014 Accepted；不含 `scripts/spec_drift_lint.sh`；不含 ADR-015；不含 phase-10 spec | 自动从 E1 起 — prelude chore PR |
| **B. prelude 已合** | master 含 `scripts/spec_drift_lint.sh` + AGENTS.md cross-validation gate；不含 ADR-015 | 自动跳 E1，从 E2 起 — spec PR |
| **C. spec PR 已合** | master 含 ADR-015 + phase-10-console-contract-v1.md + 6 task spec | 自动跳 E1/E2，从 E3 起 — task-10.1 实施 |
| **D. 部分 task 已完成** | master 含某些 task-10.X merge commit | 自动识别已完成 task，从下一个未完成项起 |

**必须的硬基线**（任一不满足 → /goal STOP 求助）：

- [ ] Claude Code 版本 ≥ v2.1.139（`/goal` 命令可用）
- [ ] git status working tree clean **OR** 仅含规范类未 commit 改动（`docs/` 限定路径；`docs/playbooks/` untracked 允许）
- [ ] master HEAD 至少含 a67b370 (ADR-014 Accepted 合入)
- [ ] `/clear` 让 context 干净（**重要** — 长 /goal 跑期间 token 累计快，Phase 10 比 Phase 9 更长）
- [ ] 当前分支 = `master`（所有 E* 都从 master 开 worktree/分支）
- [ ] **ContextForge-Console 仓库可读**：`H:/devlopment/code/ContextForge-Console/` 存在 + 其 `docs/prds/contextforge-console.prd.md` 与 `console-api/internal/coreadapter/contractv1/contractv1.go` 可读（cross-repo 主 agent file read 权限充足）
- [ ] ContextForge-Console 主分支当前为 Console v1.0 已 ship 状态（其 Mock + HTTP/gRPC Adapter scaffold 已就绪 — task-10.5 conformance test 反向跑 Console fakehttpserver 时需要）

**心理预期**：v0.2 跑出来在 task-9.5 撞了 fake-evidence 修复 + cargo build cold cache，最可能停在 E4-E6 之间。Phase 10 更复杂（cross-repo Contract v1 字段对齐 + SQLite migration + REST endpoint OpenAPI + Go ↔ Console 反向 conformance + docker-compose 联调），**最可能停在 E2 spec 阶段（cross-repo Contract v1 must-have 字段识别 + Go types 镜像设计需要主 agent 大量 verify）或 E7 conformance test（需要跨仓库跑 Console fakehttpserver 测试）**。**这次特别要警惕：(a) 把 Console 内部实现细节当成 contract（违反 Console ADR-001 单一边界），(b) ADR-014 D2 lint 自身在 Phase 10 spec 起草期误报 → autonomous 跳过标注**。

---

## 启动后注意事项

- ❌ **不要再输任何 prompt**（会清掉 active goal，前功尽弃）
- ❌ 不要切 session（session 关闭 /goal 也清）
- ❌ 不要在 /goal 跑期间修改 ContextForge-Console 仓库内容（cross-repo 写不在本 /goal scope；只读访问其 PRD + contractv1.go）
- ✅ 静观主 agent surface `◎ /goal active` 指示器跑
- ✅ 撞 STOP 时主 agent 会 surface 完整诊断报告 — 看完决定 resume 路径

---

## 命令（复制下方整段到干净 session）

```text
/goal 自治完成 v0.3 Phase 10 console-contract-v1 + v0.3.0 ship。

[阶段 0] 第一轮 read docs/playbooks/autonomous-v0.3-console-contract-v1.md 全文作为本 /goal 的执行 SOP（E* 详细完成判定 / 自决规则 10 条 / fake-evidence 警戒线 / cross-repo 边界 / ADR-014 D1-D4 制度激活全在该 playbook §命令 段之外的章节）。surface git diagnostic（status / log -5 / 关键 ls：scripts/spec_drift_lint.sh / docs/decisions/adr-015* / docs/specs/phases/phase-10* / docs/specs/tasks/task-10.* / git ls-remote --tags origin | grep v0.3）+ Console 仓库可读自检（ls H:/devlopment/code/ContextForge-Console/docs/prds/）。基于诊断决定从哪个 E* 起跑（已 merge 项 skip）。基线异常（ADR-014 未 Accepted / Console 仓库不可读）→ STOP 求助。

[完成条件 — surface 命令输出 / grep 结果 / commit SHA 证明，不接受自述]

E1 prelude: master 含 scripts/spec_drift_lint.sh 可执行 + self-test 退出 0 + AGENTS.md §3/§4 含 "ADR-014 cross-validation gate" 字面 + merge commit
E2 spec: master 含 docs/decisions/adr-015-*.md (Status: Proposed) + docs/specs/phases/phase-10-*.md (Status: Ready) + docs/specs/tasks/task-10.{1..6}-*.md (Status: Ready, 每个含 D3 verified by 显式声明) + PRD §Implementation Phases Phase 10 行 + adapter ADR/Phase 索引更新 + D2 lint 跑过 0 violation + merge commit
E3 task-10.1: master 含 internal/contractv1/ 包 + 6 must-have struct + types_test.go 跑过 + task-10.1 §6 全 [x] + §9 verify 全绿 + merge commit
E4 task-10.2: master 含 core/src/workspace/ + SQLite migration 0010_workspaces.sql + cargo test workspace 跑过 + task-10.2 §6 全 [x] + merge commit
E5 task-10.3: master 含 core/src/jobs/ + SQLite migration 0011_index_jobs.sql + IndexJob lifecycle (trigger/cancel/heartbeat) + cargo test jobs_lifecycle 跑过 + task-10.3 §6 全 [x] + merge commit
E6 task-10.4: master 含 internal/consoleapi/ + 9 个 REST endpoint + docs/consoleapi/openapi.yaml + bearer auth + go test TestRESTEndpoints_E2E 跑过 + task-10.4 §6 全 [x] + merge commit
E7 task-10.5: master 含 test/conformance/console_contractv1_test.go 真跑 Console fakehttpserver oracle 全过 + task-10.5 §6 全 [x] + merge commit
E8 task-10.6: master 含 scripts/console_smoke.sh 真启动 Console docker compose + ContextForge daemon + curl Console UI 真返回 workspace 列表 + CONSOLE_SMOKE_EXIT=0 + task-10.6 §6 全 [x] + merge commit
E9 closeout: master 含 phase-10 Status=Done + ADR-015 Accepted + adapter Phase 10=Done + PRD Phase 10 行 Status=Done + closeout PR body surface ADR-014 D1 mapping 表 (Phase §6 N 条 AC × 4 字段全填) + D2 lint 输出 0 violation + merge commit
E10 release: git ls-remote --tags origin 含 v0.3.0 + RELEASE_NOTES v0.3.0 章节 + docs/releases/v0.3.0-{evidence,artifacts}.md HEAD SHA 填实 + scripts/release_smoke.sh 第 5 段新增 + PHASE_RELEASE_SMOKE_EXIT=0

[规则与边界全部按 playbook §自决规则 / §fake-evidence 警戒线 / §与 ADR 红线关系 / §不可逆动作清单 执行 — 不在本 /goal 命令重复]

[硬 STOP] 阶段 0 基线异常 / token < 10% 且 /compact 用尽 / merge rebase 真冲突 / §9 verify 3 轮仍红 / git 状态错乱 / fake-evidence 复发 / ADR-014 D1/D2 违规 / Console contract 削弱 / cross-repo 反向依赖触发

stop after 250 turns
```

> ⚠️ /goal 命令本身仅列硬证据完成条件 + 阶段 0 read SOP 指令。**详细判定条件 / 自决规则 / fake-evidence 警戒线 / cross-repo 边界 / 不可逆动作清单 全在本 playbook 后续章节** — 主 agent 阶段 0 read 本 playbook 全文加载 SOP，evaluator 按上方简版 condition 判完成。

---

## 撞 STOP 后的 resume 指南

resume 时**重新输入完全相同的 /goal 命令** — 阶段 0 状态识别会自动判出新起点（已 merge 的 E* 标 skip，从下一个未完成项继续）。

| STOP 原因 | resume 策略 |
|---|---|
| token < 10% / context 爆 | 开新 session → `/clear` → 重新粘贴 /goal 命令 |
| rate limit 持续 | 等 1h 后同上 resume |
| 阶段 0 落入异常基线（含 Console 仓库不可读）| 看主 agent surface 的诊断 + 补救建议；人工修复基线后 resume |
| git 状态错乱 | **不要再 /goal** — 先用 `git reflog` / `git status` 人工评估，必要时 hard reset 到已知良好 commit，再决定续跑路径 |
| merge rebase 失败 | 人工解冲突 → commit → 再 /goal resume |
| §9 verify 持续红 | 人工审 task spec — 是 spec 写错、实现真漏功能、还是真要 Waive？拍板后 resume |
| **fake-evidence 警戒线触发** | **不要立即 resume** — 先人工 audit drift 类型，决定是改实施 / 改 spec / 改 AC / 接受为 known limitation 并 §3 OOS 加注。修完再 resume |
| **ADR-014 D1/D2 制度违规** | **不要立即 resume** — 是 spec 起草误用 anti-pattern（修 spec 补标注）还是 lint 规则误报（补 lint 词表白名单）？拍板后 resume |
| **Console contract 反向依赖触发** | **不要立即 resume** — Console 仓库需要调整（如 fakehttpserver 期望行为）才能让 ContextForge 通过 conformance？协调 Console 仓库 PR 后再决定路径 |

---

## 与 ADR-011 / ADR-012 / ADR-014 红线的关系（重要）

| 红线 / 规则 | 本 /goal 命令的处理 |
|---|---|
| ADR-011 红线 #1：condition 禁含「merged」 | ❌ **本命令 condition 含 E1-E10 的 "merged" 字面** — 是 user override（同 v0.1 / v0.2 playbook） |
| ADR-011 红线 #2：/goal 不用于 PR merge 本身 | ❌ **本命令让主 agent 自决合 8 个 PR（prelude + spec + 6 task）+ Phase 10 closeout PR** — 同上 user override |
| ADR-012 §2A Draft→Ready 主 agent 自决 | ✅ **本命令显式让主 agent §2A 自审** — ADR-012 §Decision 允许（前提：spec 已基于 ADR-015 §Decision + Console PRD Contract v1 must-have 字段对齐 verify） |
| ADR-012 R7 dep 主 agent 自决 | ✅ 沿用（Phase 10 不预期需新 dep，但 task-10.4 REST handler 若引 chi 中间件 v0.X 新版本 / task-10.3 jobs 持久化若引 sqlx 等可能 chore-dep；主 agent 自决走 chore-dep PR） |
| ADR-012 §8 Waive 主 agent 自决 | ⚠️ **保留 STOP**（自决规则 #5）— 安全 / 隐私 / release integrity / Console contract 契约削弱仍转 §8 STOP；其它 trade-off 主 agent 自决保守优先 |
| R3 commit 落分支硬 grep | ✅ 保留（物理层 / 自决规则 #1 / BLOCKED-branch-mismatch.md） |
| R6 PR 物理流程 | ✅ 保留（执行硬约束）— 不在 master 上 commit 业务、不 force push、merge 走 --no-ff |
| **ADR-014 D1 closeout mapping 表** | ✅ **激活**（自决规则 #10 + E9 完成条件强制） |
| **ADR-014 D2 spec_drift_lint.sh** | ✅ **激活**（E1 prelude 实现 + 自决规则 #9 + 全程 spec 起草时跑） |
| **ADR-014 D3 verified by 显式声明** | ✅ **激活**（E2 spec PR 完成条件强制 + 每 task spec §6 AC 含 verified by） |
| **ADR-014 D4 主 agent 自治补丁** | ✅ **激活但 user override**：closeout PR 缺 D1/D2 输出原本须降级用户审，本 /goal user override 改为「surface 完整 D1/D2 输出 → 自决合」；任何 D1/D2 输出空缺仍强制 §8 STOP（自决规则 #9/#10） |
| **cross-repo Console 仓库只读** | ✅ **激活**（自决规则 #8）— 任何 Console 端反向依赖转 §8 STOP，由用户协调跨仓库 PR |

---

## 预测（基于自决规则全到位）

| 阶段 | 撞 STOP 概率 | 主要原因 |
|---|---|---|
| E1 prelude chore（lint + AGENTS.md gate） | 25% | lint script 词表完整性 + AGENTS.md §3/§4 段落插入位置选择 + adapter §Workflow Overrides 字段约束 |
| E2 ADR-015 + Phase 10 spec + 6 task spec | **65%** | **本 phase 最高风险点** — cross-repo Console PRD must-have 字段读取 + Go types 镜像设计 + IndexJob 持久化方案 / REST endpoint OpenAPI / conformance test 反向设计 — 任一字段对齐错或 Console PRD 自身有 ambiguity 触发 §8 STOP；ADR-014 D2 lint 首次激活在 spec 起草期可能高频误报 |
| E3 task-10.1 contract-v1-types | 30% | Go types 镜像 Console contractv1.go 字段对齐 + FieldAvailability 机制实现 + json tag 一致性 verify |
| E4 task-10.2 workspace-resource | 45% | Rust workspace package 新建 + SQLite migration 设计（schema_version 演进策略）+ workspace_id ↔ collection 双向映射 + IndexSession::open_workspace 适配（Phase 2 indexer API 包装） |
| E5 task-10.3 indexjob-resource | **55%** | jobs lifecycle 状态机设计（trigger/heartbeat/cancel/done/error 转移）+ 异步 spawn + 持久化 + tokio 任务管理 — Rust async 复杂度高；与 task-9.3 现有 streaming Index 集成可能撞 API 不对齐 |
| E6 task-10.4 rest-endpoints | 40% | 9 个 endpoint + OpenAPI schema + bearer auth + CORS + audit log 集成；现有 internal/daemon/rest.go 是否复用 / 重构？决策点多 |
| E7 task-10.5 conformance-test | **60%** | 反向跑 Console fakehttpserver 是 cross-repo dependency；fakehttpserver 期望行为可能与 ContextForge 实际响应不对齐（如 error code mapping / FieldAvailability 表达 / pagination 行为）— 若 Console 端期望调整，转自决规则 #8 §8 STOP |
| E8 task-10.6 console-integration-smoke | 45% | docker-compose 跨容器网络 + host.docker.internal 路由 + Console 真启动 + ContextForge daemon 真启动 + Console UI 通过 Console HTTPAdapter 调真返回 — 一步失败链全断 |
| E9 Phase 10 closeout | 35% | ADR-014 D1 mapping 表首次实操（Phase 10 §6 AC 数预期 8-12 条 × 4 字段 = 32-48 cell 填实）+ ADR-015 Accepted + adapter / PRD 同步 |
| E10 v0.3.0 tag push | 25% | 不可逆动作主 agent 自审 + release notes 终检 + cross-repo Console 联调状态记录 |

**最可能停在 E2 / E5 / E7 之间**。即使全自决到位，**250 turn 内跑完整个 Phase 10 + v0.3.0 release 的概率 < 15%**（v0.2 < 20% 基础上更复杂）。但跑到 E2/E3 的概率 > 70% — 至少能把 prelude + spec + contract types 推进一截。

---

## 不可逆动作清单（启动前再确认一次）

| 动作 | 不可逆程度 | 主 agent 自决吗 |
|---|---|---|
| 在 master 上 commit | 中（可 revert）| 不会（R6 物理保留） |
| merge --no-ff 到 master | 中（可 revert）| **是**（ADR-012 + 本 override） |
| push origin master | 高（公开历史）| **是**（ADR-012 + 本 override） |
| git tag v0.3.0 + push | **极高**（GitHub release tag 删了仍有镜像/缓存）| **是**（自决规则 #7） |
| 自动 redact secret | 中（可看 INCIDENT.md 复盘）| **是**（自决规则 #4） |
| Waive AC | 中（spec 可改回）| 否（保留 STOP）— fake-evidence + ADR-014 警戒线特别严 |
| chore-dep PR（如 sqlx）| 中（lockfile 增 dep）| **是**（ADR-012 R7 自决） |
| 新建 SQLite migration（0010 / 0011）| 中（数据库 schema 演进，可回滚但需逆向 migration）| **是**（task-10.2 / 10.3 §3 In Scope 显式） |
| 创建 docs/releases/v0.3.0-*.md | 低（文档可修）| **是**（E10 完成条件强制） |
| **修改 Console 仓库任何文件** | 高（cross-repo 协调）| **否**（自决规则 #8 硬约束转 §8 STOP） |
| **ADR-014 lint 词表白名单调整** | 中（影响后续 spec PR lint 行为）| **是**（lint script self-test 通过即自决 commit） |
| **PRD §Implementation Phases 新增 Phase 10 行** | 中（PRD 修改）| **是**（E2 完成条件强制） |

如要把任一动作改回保守（STOP 求助），改 /goal command 的对应规则后再启动。

---

## 历史关联

- ADR-011 single-driver-with-subagents 是本 playbook 的治理基线
- ADR-012 main-agent-governance-autonomy 进一步放宽主 agent 自治范围
- **ADR-014 cross-phase-exit-criteria-validation 是本 playbook 首次完整激活的治理制度**（D1 closeout mapping / D2 lint / D3 verified by / D4 主 agent 自治补丁）
- ADR-015 console-contract-v1-compatibility（待本 /goal E2 起草）是 Phase 10 的核心决策来源
- autonomous-v0.1-ship.md 是本 playbook 的远祖 — v0.1 跑出来撞 fake-evidence drift，触发 ADR-013 + Phase 9 修复
- autonomous-v0.2-cli-pipeline.md 是本 playbook 的直接前身 — 同样的 user override 模式；其 fake-evidence 经验直接落 ADR-014
- ContextForge-Console 仓库 (H:/devlopment/code/ContextForge-Console/) 是本 playbook 的反向依赖目标：Console v1.0 已 ship + 等真 Core；Console Contract v1 must-have 字段是本 phase 实施的 single source of truth
- 启动失败 / 取消后，仓库状态仍是 ADR-014 已合 + Phase 10 未启 不损坏既有治理（v0.2.0 + Console mock mode 仍可用）
