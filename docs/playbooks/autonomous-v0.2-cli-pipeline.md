# Playbook: Autonomous v0.2 CLI Pipeline Ship via `/goal`

> **不入 git** — 本文件保留为本地 untracked，仅作个人操作 playbook。如要团队复用，移到 `docs/decisions/` 或单独 ADR 化。

## 用途

一条 `/goal` 命令启动主 agent 自治跑完：

1. promote PR #58 spec Status: Draft → Ready + 自决合 spec chore PR
2. task-9.1 → 9.2 → {9.3 ∥ 9.4} → 9.5 → 9.6 共 6 task 全程实施 + 自合各自 feature PR
3. Phase 9 closeout（ADR-013 Proposed → Accepted + adapter Phase 9 → Done + PRD §Implementation Phases Phase 9 → Done）
4. v0.2.0 release（RELEASE_NOTES + evidence + artifacts + git tag v0.2.0 + push）

**本质**：这条 /goal 命令是 **user override 当前 ADR-011 红线 #1**（condition 禁含「merged」）+ **R6 merge decision** 自决。启动它等于命令主 agent 把 §2A Ready review + 6 个 task PR merge + v0.2.0 tag push 全部交给 evaluator 判 + 自决。

---

## 启动前 checklist

仓库**任一**符合的基线即可启动（命令本身会自检 + 决定从哪个 E* 起跑）：

| 基线 | 状态特征 | /goal 从哪起 |
|---|---|---|
| **A. spec PR #58 待审** | master 不含 ADR-013 + chore/phase-9-spec-and-adr 分支含 3 commit | 自动从 E1 起 — promote Status + 自合 spec PR |
| **B. spec PR 已合 master** | master 含 ADR-013 + Phase 9 spec + 6 task spec | 自动跳过 E1，从 E2 起 — task-9.1 实施 |
| **C. 部分 task 已完成** | master 含某些 task-9.X merge commit | 自动识别已完成 task，从下一个未完成项起 |

**必须的硬基线**（任一不满足 → /goal STOP 求助）：

- [ ] Claude Code 版本 ≥ v2.1.139（`/goal` 命令可用）
- [ ] git status working tree clean **OR** 仅含规范类未 commit 改动（`docs/` / `_dispatch/` 限定路径；`docs/playbooks/` untracked 允许）
- [ ] master HEAD 至少含 ce47d17 (v0.1.0)
- [ ] `/clear` 让 context 干净（**重要** — 长 /goal 跑期间 token 累计快）
- [ ] 当前分支允许是 `chore/phase-9-spec-and-adr` 或 `master`（前者 E1 起，后者已合 master 后从 E2 起）

**心理预期**：v0.1 跑出来在 task-8.3 撞了 fake-evidence drift。Phase 9 更复杂（Go + Rust + proto + gRPC stream + 真集成测试），最可能停在 E4-E6 之间（task-9.2 rust-grpc-index 或 task-9.3 go-cli-index cargo build 慢 + 真集成 flake）。撞顶不是失败 — 是 best-effort autonomous 的典型边界。**这次特别要警惕 fake-evidence 复发** —— 任何 "AC 通过靠 stub validator 接受 stub 输入" 立即转 §8 不允许 autonomous 自决 Waive。

---

## 启动后注意事项

- ❌ **不要再输任何 prompt**（会清掉 active goal，前功尽弃）
- ❌ 不要切 session（session 关闭 /goal 也清）
- ✅ 静观主 agent surface `◎ /goal active` 指示器跑
- ✅ 撞 STOP 时主 agent 会 surface 完整诊断报告 — 看完决定 resume 路径

---

## 命令（复制下方整段到干净 session）

```text
/goal 自治完成 Phase 9 cli-pipeline + v0.2.0 ship。

[阶段 0] 第一轮 surface git diagnostic（status / log -10 / branch / ls docs/specs/tasks/task-9.* / ls docs/decisions/adr-013*）+ docs/s2v-adapter.md 当前 Phase 9 / Task 9.X Status + ADR-013 Status。基于诊断从下方 E* 决定起跑（已完成项标 skip）。基线异常 → STOP 求助。

[完成条件] surface 以下全到位即完成（命令输出 / 文件片段 / grep 结果，不接受自述）：

E1 PR #58 spec chore 已合 master：master HEAD 含 ADR-013 + phase-9-cli-pipeline.md + 6 task spec + cli-pipeline.feature；且各 spec 顶部 Status 从 Draft 推 Ready（主 agent §2A 自审基于 ADR-013 §Decision 锚点）

E2 task-9.1 proto-index-rpc 全 [x]：proto/contextforge/v1/service.proto 含 "rpc Index(IndexRequest) returns (stream IndexProgress);" 字面（grep 命中）+ proto/contextforge/v1/index.proto 含 IndexRequest 3 字段 + IndexProgress 7 字段 + Go codegen 产物 commit + Rust 绑定 cargo check 全绿 + task-9.1 §6 AC1-5 全 [x] + §9 verify 全绿 + §10 6 项齐 + master 含 merge commit

E3 task-9.2 rust-grpc-index 全 [x]：IndexSession::index_path_with_progress 实现 + CoreService::index trait method 实现 + core/tests/phase9_index_smoke.rs 通过 (cargo test --test phase9_index_smoke 退出 0) + task-9.2 §6 AC1-5 全 [x] + §9 verify 全绿 + §10 6 项齐 + master 含 merge commit

E4 task-9.3 go-cli-index 全 [x]：internal/daemon/index.go 新增 + internal/cli/index.go 重写调真实 gRPC + integration test TestCliIndex_E2E_RealCore 通过 (go test ./internal/cli -run TestCliIndex_E2E_RealCore 退出 0) + task-9.3 §6 AC1-5 全 [x] + §9 verify 全绿 + §10 6 项齐 + master 含 merge commit

E5 task-9.4 go-cli-import 全 [x]：internal/cli/import.go 实现三子命令 + recordToMarkdown helper + cli.go dispatch case "import" wire 真实 + import_test.go 5 AC 全过 + task-9.4 §6 AC1-5 全 [x] + §9 verify 全绿 + §10 6 项齐 + master 含 merge commit

E6 task-9.5 release-smoke-real 全 [x]：internal/release/release_test.go 删除 TestTask83_AC2 / TestTask83_AC4 函数（grep "Status: StepPassed, Evidence:" 命中 0）+ TestTask83_AC1 重写为真 go build + cargo build + 真 binary tarball + TestPhase9ReleaseSmoke_EndToEnd 通过（go test ./internal/release -run TestPhase9ReleaseSmoke_EndToEnd -timeout 180s 退出 0）+ scripts/release_smoke.sh 含 phase 9 段 + task-9.5 §6 AC1-5 全 [x] + §9 verify 全绿 + §10 6 项齐 + master 含 merge commit

E7 task-9.6 readme-quickstart-verified 全 [x]：examples/quickstart/sample-project/ ≥5 .md + .env + .yaml + .go + .log + examples/quickstart/hermes-memory/MEMORY.md/USER.md + scripts/quickstart_smoke.sh 退出 0 + 最末输出 QUICKSTART_SMOKE_EXIT=0 + README.md Quick Start 段含 examples/quickstart/ 引用 + RELEASE_NOTES.md v0.2.0 章节 + docs/releases/v0.2.0-evidence.md + v0.2.0-artifacts.md + task-9.6 §6 AC1-5 全 [x] + §9 verify 全绿 + §10 6 项齐 + master 含 merge commit

E8 Phase 9 closeout 已合 master：phase-9-cli-pipeline.md Status=Done + §6 阶段级 AC 全 [x] + §8 DoD 全 [x] + adapter §Phase 索引 Phase 9 行 Status=Done + ADR-013 Status: Proposed → Accepted + PRD §Implementation Phases Phase 9 行 Status=Done

E9 v0.2.0 release：git ls-remote --tags origin 含 v0.2.0 + RELEASE_NOTES.md v0.2.0 章节进 master + docs/releases/v0.2.0-evidence.md HEAD commit SHA 填实（非 <待 chore PR 合后填> 占位）+ docs/releases/v0.2.0-artifacts.md 按 ADR-007 产物清单完整

[自决规则]
1. branch mismatch → reflog + cherry-pick 复原；写 BLOCKED-branch-mismatch.md 留痕（ADR-011 物理保险）
2. context < 30k → /compact + 重 load 当前阶段关键 spec
3. rate limit → 60s 退避，5 次后 Agent tool spawn subagent 续跑实施部分（评审 / merge 仍主 agent）
4. secret 泄露 → 自动 redact + 写 docs/security/INCIDENT-*.md + 不 push 含 secret 的 commit
5. trade-off 无锚点 → 保守优先（backward compat > spec 字面 > 最小改动），§10 注明；如涉及安全 / 隐私 / release integrity 削弱 → 转 §8 STOP（不 autonomous Waive）
6. 6 task PR Gate 0-5 全绿后主 agent 自决合（ADR-012 + 本 user override）
7. v0.2.0 tag push 自审 release notes + 产物完整性后自动 push（ADR-012 §自决规则 #6）

[fake-evidence 警戒线 — 本 phase 因 ADR-013 §Context 特别强调]
- 每个 AC 通过判定必须基于"真命令真退出码真输出"而非"validator 接受 stub 输入"
- task-9.5 §6 AC2 grep "Status: StepPassed, Evidence:" 在 internal/release/ 必须 0 命中 — 否则视为 fake-evidence 复发，转 §8 STOP
- task-9.6 §6 AC3 scripts/quickstart_smoke.sh 必须真跑七步 CLI binary（含 cargo build + go build + 真索引 ≥1 文件 + 真 search 返回 ≥1 结果），不接受 mock / skip
- 任何 task §9 verify 出现 "SKIP" / "TBD" / "等用户" → 不允许 autonomous 推 Status=Done，转 §8 STOP
- 任何 task §10 Completion Notes 出现 "fake" / "stub" / "假证据" / "fixture-only" 等词 → autonomous 检测必须立即 STOP

[硬 STOP] 阶段 0 基线异常 / token < 10% 且 /compact 用尽 / merge rebase 真冲突 / §9 verify 3 轮 systematic-debugging 仍红 / git 状态完全错乱 / 检测到 fake-evidence 复发倾向（见警戒线）

[硬约束] 每阶段 §9 真跑（surface 命令 + 退出码）/ R3 commit 落分支 grep 全程 / §4 Gate 0-5 物理流程不跳 / 每 commit 调 TaskUpdate 同步 / STOP 必 surface 完整诊断 + 续跑建议 / fake-evidence 警戒线全程激活

stop after 200 turns（surface 完整进度 + 已完成 E* + 未完成 E* + token 余量 + 当前阻塞）
```

---

## 撞 STOP 后的 resume 指南

resume 时**重新输入完全相同的 /goal 命令** — 阶段 0 状态识别会自动判出新起点（已 merge 的 E* 标 skip，从下一个未完成项继续）。

| STOP 原因 | resume 策略 |
|---|---|
| token < 10% / context 爆 | 开新 session → `/clear` → 重新粘贴 /goal 命令 |
| rate limit 持续 | 等 1h 后同上 resume |
| 阶段 0 落入异常基线 | 看主 agent surface 的诊断 + 补救建议；人工修复基线后 resume |
| git 状态错乱 | **不要再 /goal** — 先用 `git reflog` / `git status` 人工评估，必要时 hard reset 到已知良好 commit，再决定续跑路径 |
| merge rebase 失败 | 人工解冲突 → commit → 再 /goal resume |
| §9 verify 持续红 | 人工审 task spec — 是 spec 写错、实现真漏功能、还是真要 Waive？拍板后 resume |
| **fake-evidence 警戒线触发** | **不要立即 resume** — 先人工 audit drift 类型，决定是改实施 / 改 spec / 改 AC / 接受为 known limitation 并 §3 OOS 加注。修完再 resume |

---

## 与 ADR-011 / ADR-012 红线的关系（重要）

| 红线 / 规则 | 本 /goal 命令的处理 |
|---|---|
| ADR-011 红线 #1：condition 禁含「merged」 | ❌ **本命令 condition 含 E1/E2/E3/E4/E5/E6/E7/E8 的 "merged" 字面** — 是 user override（你已决定接受 ADR-013 推进 + autonomous 模式风险）|
| ADR-011 红线 #2：/goal 不用于 PR merge 本身 | ❌ **本命令让主 agent 自决合 6 个 task PR + Phase 9 closeout PR** — 同上 user override |
| ADR-012 §2A Draft→Ready 主 agent 自决 | ✅ **本命令显式让主 agent §2A 自审** — ADR-012 §Decision 允许（前提：Phase 9 spec 已基于用户先期 D1/D2/D3 锚点 + ADR-013 §Decision）|
| ADR-012 R7 dep 主 agent 自决 | ✅ 沿用（Phase 9 不预期需新 dep，但 task-9.2 §5.2 警告 tokio-stream 可能需 chore-dep；如触发主 agent 自决走 chore-dep PR）|
| ADR-012 §8 Waive 主 agent 自决 | ⚠️ **保留 STOP**（自决规则 #5）— 安全 / 隐私 / release integrity 削弱仍转 §8 STOP；其它 trade-off 主 agent 自决保守优先 |
| R3 commit 落分支硬 grep | ✅ 保留（物理层 / 自决规则 #1 / BLOCKED-branch-mismatch.md）|
| R6 PR 物理流程 | ✅ 保留（执行硬约束）— 不在 master 上 commit 业务、不 force push、merge 走 --no-ff |
| 本 phase fake-evidence 警戒线 | ✅ **新增 — Phase 9 自创**（ADR-013 §Context #2 经验教训直接落 condition）|

---

## 预测（基于自决规则全到位）

| 阶段 | 撞 STOP 概率 | 主要原因 |
|---|---|---|
| E1 spec PR 自合 + Status promote | 10% | 简单 docs 操作 + ADR-012 §2A 自审锚点充分 |
| E2 task-9.1 proto 改 + codegen | 25% | buf generate 工具链 / Rust tonic-build 在不同 dev 机版本差异；R7 lockfile 不预期触发 |
| E3 task-9.2 rust-grpc-index | 50% | tokio_stream / spawn_blocking 复杂度 + tonic stream associated type 编译错；可能需 chore-dep PR (tokio-stream) |
| E4 task-9.3 go-cli-index | 60% | cargo build cold cache 60s + 真 daemon 起停 + gRPC stream consume 边界 case + Windows path 差异（如在 WSL2 上跑 ok）|
| E5 task-9.4 go-cli-import | 45% | importer 包 constructor / Register 模式与本 task §5.3 假设不一致（§2A 时主 agent 要 verify）|
| E6 task-9.5 release-smoke-real | 70% | 删除 fake-evidence 测试 + 重写真集成测试 + cargo build cold cache + Windows skip 逻辑 + ValidateSmokeEvidence 真 evidence 适配 |
| E7 task-9.6 readme-quickstart-verified | 50% | quickstart_smoke.sh 七步 e2e + fixture 设计 + RELEASE_NOTES 格式 + evidence/artifacts 模板 |
| E8 Phase 9 closeout | 30% | 多文件 status 推进 + adapter / ADR / PRD 同步 |
| E9 v0.2.0 tag push | 25% | 不可逆动作主 agent 自审 + release notes 终检 |

**最可能停在 E3-E6 之间**。即使全自决到位，**200 turn 内跑完整个 Phase 9 + v0.2.0 release 的概率 < 20%**。但跑到 E2/E3 的概率 > 60% — 至少能把 spec PR 合 + proto 扩展 + Rust grpc-index 推进一截。

---

## 不可逆动作清单（启动前再确认一次）

| 动作 | 不可逆程度 | 主 agent 自决吗 |
|---|---|---|
| 在 master 上 commit | 中（可 revert）| 不会（R6 物理保留） |
| merge --no-ff 到 master | 中（可 revert）| **是**（ADR-012 + 本 override）|
| push origin master | 高（公开历史）| **是**（ADR-012 + 本 override）|
| git tag v0.2.0 + push | **极高**（GitHub release tag 删了仍有镜像/缓存）| **是**（自决规则 #7）|
| 自动 redact secret | 中（可看 INCIDENT.md 复盘）| **是**（自决规则 #4）|
| Waive AC | 中（spec 可改回）| 否（保留 STOP）— fake-evidence 警戒线特别严 |
| chore-dep PR（如 tokio-stream）| 中（lockfile 增 dep）| **是**（ADR-012 R7 自决）|
| 删除 fake-evidence 测试代码 | 低（git 历史保留）| **是**（task-9.5 §3 In Scope 显式）|

如要把任一动作改回保守（STOP 求助），改 /goal command 的对应规则后再启动。

---

## 历史关联

- ADR-011 single-driver-with-subagents 是本 playbook 的治理基线
- ADR-012 main-agent-governance-autonomy 进一步放宽主 agent 自治范围
- ADR-013 cli-data-plane-grpc-bridge 是 Phase 9 的核心决策来源 — 也记录了 v0.1 autonomous 跑出 fake-evidence drift 的经验教训
- autonomous-v0.1-ship.md 是本 playbook 的直接前身 — 同样的 user override 模式 + 同样需警惕 autonomous evaluator 漏检 spec drift
- 启动失败 / 取消后，仓库状态仍是 PR #58 待审 + Phase 9 spec Draft + 6 task spec Draft 不损坏既有治理
