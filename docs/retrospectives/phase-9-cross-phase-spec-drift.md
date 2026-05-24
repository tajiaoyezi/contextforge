# Retrospective: Phase 1-8 跨 phase spec drift 击鼓传花机制

**日期**：2026-05-24
**主干 HEAD**：`1854646` (Phase 9 closeout)
**作者**：tajiaoyezi objective + main agent execution
**触发**：ADR-013 §Follow-ups + PRD §Open Questions O12 / `governance-retrospective-cross-phase` chore
**Scope**：Phase 1-8（Phase 9 已由 ADR-013 §Context 单独记录）
**产物锚点**：[ADR-014 cross-phase-exit-criteria-validation](../decisions/adr-014-cross-phase-exit-criteria-validation.md)

---

## 1. 触发事件

v0.1.0 (tag `v0.1.0`, master `ce47d17`, 2026-05-23) 在 spec / 治理 / release contract 层面声明完成；2026-05-24 主 agent 端到端实测发现：

- `contextforge index` 是 manifest 存根（`processed=0 total=0`）
- `contextforge import` 直接返 `not implemented`
- `proto/contextforge/v1/service.proto` 没有 `rpc Index`
- `internal/release/release_test.go::TestTask83_AC2` 是 fake-evidence（`[]StepResult{Status: StepPassed, Evidence: "ok"}` 喂给 validator）

完整证据链见 [ADR-013 §Context](../decisions/adr-013-cli-data-plane-grpc-bridge.md#context)。Phase 9 (`feat/phase-9-cli-pipeline` / PR #58-65) 已通过 task-9.1～9.6 + closeout 在 v0.2.0 修复实现层 spec drift。

**本 retrospective 不修复实现** — Phase 9 已修。本 retrospective 回答 **为什么 spec drift 一路跨 Phase 1 / 2 / 6 / 8 没被任何 closeout / §2A / merge gate 截住，直到 v0.1.0 已 ship 才被实测发现**。

---

## 2. 击鼓传花链（既有证据，引用 ADR-013）

ADR-013 §Context "Spec drift 形成原因（retrospective）" 段已列出 4 站击鼓传花：

| 站 | 来源 | 推给 | 该站说辞 |
|---|---|---|---|
| 1 | task-1.4 §3 OOS | Phase 2+ / 6 / 7 / 8 | "仅注册 not-implemented 骨架"，CLI 非 init 子命令业务实现全推 |
| 2 | task-2.4 §3 OOS | Phase 6 / 7 | "REST/MCP/gRPC 暴露 indexer (Phase 6 / 7 — 本 task 仅 Rust API + Rust smoke)" |
| 2.5 | phase-2-index-core §6 注 | Phase 6 task-6.1 → Phase 8 task-8.3 | "CLI `contextforge index` 端到端在 Phase 6 task-6.1 实现后由 Phase 8 task-8.3 release smoke 接管" |
| 3 | task-6.1 §3 In Scope | (未规划目的地) | 只覆盖 CLI `search` + Rust `CoreService::search` wire；CLI `index` / `import` wire 或 proto `rpc Index` **未规划** — 击鼓传花链在此断链未续 |
| 4 | task-8.3 §3 OOS | (终点宣布 OOS) | "修复所有历史产品 gap（如完整 import CLI 体验）；本 task 只建立 release smoke 可判定门" |

**第 4 站终结却仍 AC `[x]`**：task-8.3 §6 AC2 字面声称 `[x]` 通过 "解包→init→import→index→search/MCP→export→eval run 端到端"，但同 task §3 已自承 import/index 实现不在 scope。AC2 通过靠 validator 接受 stub 输入（`Evidence: "ok"`），不实际跑 CLI binary —— **AC2 是假 Done**。

---

## 3. 新增证据（本 retrospective 调研补充）

主 agent spawn Explore subagent 跨 `docs/specs/` 做 anti-pattern 关键词扫描 + Phase Exit Criteria ↔ task §6 AC 对照审计 + 历史 closeout PR 模式审计，补充 ADR-013 §Context 未覆盖的系统性证据。

### 3.1 不在 ADR-013 §Context 的 phase ↔ task AC mismatch（5 处）

| # | Phase Exit Criteria 字面 | Task §6 AC 实际 | Gap |
|---|---|---|---|
| G1 | phase-2-index-core §6: "`contextforge index ./sample_project` 能索引 ≥1000 文件" | task-2.4 §6 AC1 同字面；smoke 由 `core/tests/phase2_smoke.rs` 跑 `IndexSession` 直接调 Rust API | Phase 顶层字面是 CLI 能力，task 实际跑 Rust API；CLI 不存在直到 Phase 6 task-6.1，且 task-6.1 只 wire `search` 不 wire `index`。Phase 2 §6 实际验证对象与字面承诺脱节 |
| G2 | phase-3-agent-importers §6: "`contextforge import openclaw/hermes/agent-rules` 把外部源转为 canonical record（与 Phase 2 集成后端到端入索引）" | phase-3-agent-importers §6 smoke 注："phase smoke 用 Go test 端到端验证 `importer.Resolve()` + `Import()` flow（**CLI `contextforge import` 子命令在 Phase 6 task-6.1 实现**，本 phase 不依赖）" | Phase 顶层 §6 主文字面是 CLI 能力；同 phase §6 注脚自我修正为 API-level；下游 Phase 6 task-6.1 §3 In Scope 又只覆盖 `search` — 字面承诺与注脚 / 下游实施串联断裂 |
| G3 | phase-6-cli-api-export §6: "REST `/v1/search` 可用" + 隐含 5 endpoint | task-6.2 §3 OOS: "完整 POST /v1/import 实现：本 task 仅 stub 501；真实 import 留 future" + "gRPC `ContextService::GetChunk` RPC...新 RPC 留 future SPEC-DRIFT-task-6.2+" + "TLS / mTLS / OAuth：留 Phase 7+ / v0.3" | Phase 6 顶层未显式声明 import/chunk REST 已实现，但 PRD §User Flow 字面承诺。task-6.2 自承 stub 但 AC 不显式标 "not implemented in v0.1"。closeout PR 未捕获 stub 与 PRD 字面承诺的 cross-phase gap |
| G4 | task-8.3 §6 AC2: "Release smoke includes ordered steps: unpack → init → import → index → search → mcp → export → eval (verified by TestTask83_AC2)" | task-8.3 §3 OOS: "修复所有历史产品 gap（如完整 import CLI 体验）；本 task 只建立 release smoke 可判定门" | 同 task 内 §6 AC2 字面与 §3 OOS 字面互斥。`TestTask83_AC2` 喂 `Evidence: "ok"` hardcoded fake 给 validator，AC2 `[x]` 通过靠 §3 自承未做的功能 — 已由 ADR-013 §Context #2 / #3 标注，本表对齐到 phase ↔ task AC 视角 |
| G5 | phase-1-foundation §6: "Go daemon 能通过 local gRPC health check Rust core" + task-1.4 AC2/AC3 实现 health check | task-6.2 §3 In Scope: "daemon 新增 Listen/Serve 编排；local random token 0600"；REST `/v1/search` 假设 `ContextService.Search` proto 存在 | `ContextService.Search` proto 实际存在（task-1.1 已加），但 task-1.3 §3 注 "ContextService.Search 留 Status::unimplemented stub (业务属 Phase 2+, 本 task 仅骨架)"。即 proto field tag 占位但实现 stub；Phase 6 REST `/v1/search` 实施时 task-6.1 / 6.2 自己接通 `ContextService.Search` (而非依赖 task-2.4)，绕过 Phase 2 indexer 真实数据通路。这是 task-1.3 §3 备注向 Phase 6 推 + Phase 6 未关注 indexer wire 真实性的次级 drift。Phase 9 task-9.2 才真正补齐 |

### 3.2 Anti-pattern 关键词扫描（按风险类型分桶）

Explore subagent grep 跨 `docs/specs/` 共找 50+ 命中，按类型分桶：

**Type A — 合法延后（有明确 follow-up + 命名 target）**：
- task-7.1 `SPEC-DRIFT-task-7.1.*` 系列：MCP HTTP/SSE/WebSocket transport / 2025-11-25 spec bump / 完整 semver parser — 均有显式 marker
- task-9.1 (Phase 9 自身)：补齐 Phase 2 deferred `rpc Index` — 闭环完成

**Type B — 静默延后（无 owner / 链断 / closeout 未追踪）**：
- task-1.3:27 "`ContextService.Search` 留 `Status::unimplemented` stub" → 推 Phase 2+；Phase 2 把 CLI 推 Phase 6 / 7；最终 Phase 9 task-9.2 才补
- task-1.4 所有非 `init` 子命令 "Phase 2+/6/7/8" 注册 not-implemented → 多 phase 循环推
- task-2.4:42 "REST/MCP/gRPC 暴露 indexer (Phase 6/7)" → Phase 6 task-6.1 §3 不覆盖；断链直到 Phase 9
- task-6.2:79-84 "POST /v1/import: 本 task 仅 stub 501; 真实 import 留 future phase 8 chore / task-8.x backlog" — Phase 8 task-8.1/8.2/8.3 均未规划 import；断链直到 Phase 9 task-9.4
- task-6.2 "GetChunk RPC 留 future SPEC-DRIFT-task-6.2+" — 至今未规划
- task-6.2 "TLS / mTLS / OAuth：留 Phase 7+ / v0.3" — 至今未规划

**Type C — 闭环验收（推到下一 phase 且真在该 phase 做了）**：
- Phase 9 task-9.1 / 9.2 / 9.3 / 9.4 / 9.5 / 9.6 全是 Phase 1-8 各种 deferral 的闭环兑现

**模式判定**：合法延后（Type A）有 `SPEC-DRIFT-<task>.*` 命名 marker + closeout 时可 grep 验真；静默延后（Type B）只有"留给 Phase X+"自由文本 + 无 owner + closeout 不查 — 这是击鼓传花链的滋生土壤。

### 3.3 Closeout PR 纪律审计

检查 master 历史 merge commits（`git log --merges --oneline master | head -30`），列出全部 phase closeout PR：

| PR | Phase | commit message 核心 |
|---|---|---|
| #22 (f64a537) | Phase 3 closeout | (Status flip + adapter sync) |
| #25 (0767710) | Phase 2 closeout | "§6/§8 + adapter Status sync" |
| #30 (fbdf0a8) | Phase 4 closeout | "§6/§8 + Status→Done" |
| #35 (55e7499) | Phase 5 closeout | "§6/§8 + Status→Done" |
| #45 (cd7df58) | Phase 6 pre-closeout | "adapter Status sync + 3 task post-merge nits + audit Go vs Rust spec contractual 修正" |
| #49 (f5770c1) | Phase 7 closeout | "Status sync" |
| #50 (a2b80b2) | Phase 6/7 spec-drift | "PRD O4 + task-7.1 AC + Phase 6/7 §8 DoD" |
| #65 (1854646) | Phase 9 closeout | "phase-9 Status=Done + ADR-013 Accepted + adapter Phase 9=Done" |

**统一模式 — closeout PR 做 3 件事**：
1. 所有 child task Status=Done 检查 ✅
2. Phase §6 / §8 文本 / Status 更新 ✅
3. Adapter index sync ✅

**统一缺失 — 4. Phase 顶层 Exit Criteria 字面 ↔ child task §6 AC 字面 cross-validation ❌**：
- 无任何 closeout PR commit message 含 "cross-validate" / "字面对齐审计" / phase AC ↔ task AC 映射表
- 无任何 closeout PR 含 grep anti-pattern (`留给 Phase` / `本 task 仅` / `out of scope` / `stub`) 的产物

例外：PR #45 (Phase 6 pre-closeout) 含 "audit Go vs Rust spec contractual 修正" — 是 **reviewer 在 PR review 阶段反应式发现的 Go vs Rust spec 不对称**，不是 closeout checklist 主动驱动。这佐证 cross-validation 偶发出现于 reviewer 手动审视，但**未被制度化**。

---

## 4. 系统性机制（最关键发现）

Phase Definition of Done (§8) 要求 "§6 阶段级 AC 全部满足"。但**满足**未被操作化定义为 "每条 §6 AC 都 cross-reference 到拥有 task 的 §6 AC + 链接 evidence"。

实际操作链：
1. 主 agent / reviewer 看 phase §6 时只检查 phase 自身文字是否 `[x]`
2. phase §6 通过靠 phase-level smoke test 跑过；但 smoke 实际验证对象（如 Rust API）可能与 §6 字面承诺（如 CLI 能力）脱节
3. 每个 child task §6 AC 自圆其说；task §3 OOS 把 "未做的部分" 推给虚构的下一 phase
4. 下一 phase task §3 In Scope 不主动接管前 phase OOS 的部分（无 owner）
5. 击鼓传花链在某 phase task §3 OOS 处终结，但同 task §6 AC 仍 `[x]`
6. closeout PR 机械 Status flip，不审 phase §6 ↔ task §6 字面对齐

**单 task 视角的 §2A Ready review / merge gate / Waive 评估都看不到这个链**。需跨 task / 跨 phase 视角才能发现。

ADR-012 §自治范围把 §2A / R6 merge / R7 dep / §8 Waive 交给主 agent；主 agent 在**单 task / 单 phase** 视角内执行 Gate 0-5 充分，但**跨 phase spec drift 检测**在单 task 视角下结构性看不到 — ADR-012 治理自治在跨 phase 维度有覆盖盲区。

---

## 5. 改进提议（指向 ADR-014）

提议引入 **Phase Exit Criteria ↔ Task §6 AC 双向 cross-check 制度**，作为 ADR-014 主决策。细化要点：

1. **Phase closeout PR 必含 cross-validation 表**：以列表/表格形式 surface "phase §6 AC N → 拥有 task <X.Y> §6 AC M → evidence 链接（PR / test 文件 / smoke 脚本退出码）"。空 mapping 或 unmapped AC = closeout PR review 阻塞
2. **击鼓传花条款识别清单（anti-pattern lint）**：写 `scripts/spec_drift_lint.sh` grep `docs/specs/` 中 anti-pattern 词（`留给 Phase` / `本 task 仅` / `历史 gap` / `留 future` / `推给 task-X` 等），输出每个命中的 file:line + 强制要求 spec 作者标注 `[A]` 合法延后（有 SPEC-DRIFT marker + 命名 target）或 `[B]` 静默延后（必须开新 task 拥有）。closeout PR 跑 lint 并 surface 输出
3. **Phase §6 字面承诺的验证对象一致性 gate**：每条 Phase §6 AC 必须显式声明是否由"phase-level smoke"还是"child task §6"承担 verification；后者必须命名具体 task §6 AC 编号
4. **主 agent 自治补丁**：ADR-012 §自治范围在 phase closeout 时增加约束 — closeout PR 必须 surface 上述 #1 表 + #2 lint 输出；缺则视为 §2A 未满足，需用户审或升级到 §8 STOP
5. **历史已 Done phase 不溯改**：Phase 1-8 已 closeout 不重审；ADR-014 在 Phase 10 起新建 phase 强制执行（Phase 9 自身已通过 ADR-013 闭环修复，本 retrospective 即是其 follow-up）

---

## 6. 不在本 retrospective scope

- **不重审 Phase 1-8 task spec 内容本身**：retrospective 是过程审视，不是 spec 重做。已 Done phase 的实施债（如 task-6.2 GetChunk / TLS / OAuth）若仍需要应在新 phase / 新 task 单独立项
- **不评估 ADR-011 / ADR-012 是否需要回退**：单驱动 + 主 agent 自治本身不是 spec drift 根因（Phase 1-8 大部分实施期是 team worker 多 agent 拓扑）。ADR-014 是在 ADR-012 之上**叠加 cross-phase 视角的检测制度**，不是 supersede ADR-012
- **不直接修复 Console 对接缺失**：v0.3 Phase 10 对接 Console Contract v1 是 ADR-014 制度的首个适用场景，不是本 retrospective 范围
- **不审 ADR 之间 cross-link 完整性**：另立 chore

---

## 7. 引用证据来源

- [ADR-013 §Context](../decisions/adr-013-cli-data-plane-grpc-bridge.md#context)：3 类 spec drift 实测证据 + 4 站击鼓传花链原始记录
- [ADR-013 §Follow-ups](../decisions/adr-013-cli-data-plane-grpc-bridge.md#follow-ups)：本 retrospective 的直接源头（O12 新增）
- [ADR-012](../decisions/adr-012-main-agent-governance-autonomy.md)：单驱动主 agent 自治范围（本 retrospective 提议在 closeout 维度叠加约束）
- [ADR-011](../decisions/adr-011-single-driver-with-subagents.md)：单驱动拓扑基线
- [Phase 9 cli-pipeline spec](../specs/phases/phase-9-cli-pipeline.md)：闭环修复实施记录
- 调研工具：`Grep` 跨 `docs/specs/` anti-pattern 关键词扫描 + `git log --merges` master 历史 closeout PR commit message 审计
- 本 retrospective 由主 agent 单 turn spawn Explore subagent 调研 + 整合产出，全部证据 file:line 可溯
