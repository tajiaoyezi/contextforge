# Phase 47 · v1.0.0-release

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 是 **v1.0 收口冲刺的终点**（承 ADR-050 v1.0 定义完整 ratify）。Phase 45 交付 D1/D2（能力 + API/CLI 冻结）；Phase 46 交付 D3/D4（文档对齐 + GitHub Release 流程，首次实践成功）。本 phase 完成 v1.0.0 正式发版：(1) **README maturity label flip**（Pre-1.0 → **v1.0.0**）；(2) **ADR-050 完整 ratify**（Proposed → Accepted，D1-D4 全真实交付验证）；(3) **v1.0.0 Release notes 列已知限制**（所有 active SPEC-DEFER 按 category 归类为 v1.0 known limitations，ADR-013 honest-defer）；(4) **v1.0.0 tag**（major version bump，用 Phase 46 的 release.yml Release 对象）。
>
> **Grounding（ADR-013）**：v1.0.0 是 major version 里程碑——不是新功能，而是**成熟度声明**。recall@5/@10=1.0 超 PRD 北极星 75%/85%（D1 能力锚点 v0.1 P0 远超）；proto FROZEN + daemon REST 清 501 + CLI 冻结（D2 Phase 45）；README 重构 + CHANGELOG + ADR 索引（D3 Phase 46）；GitHub Release 对象自动创建首次实践成功（D4 Phase 46）。v1.0.0 正式把 "Pre-1.0 收口中" maturity label flip 为 "v1.0.0"——给用户/自己的成熟度信号。**不含**（honest-defer 推 v2.0，ADR-013）：multi-user/认证身份/自动更新/arm64 native。
>
> 本 phase 全程 **0 代码逻辑改动 / 0 新 dep / 0 migration / 0 proto / 0 schema change**（纯文档 + tag）。默认行为 / 既有契约 / 三门不退化。

> **入读顺序**：本 phase spec → ADR-050（v1.0 定义，本 phase 完整 ratify）→ roadmap §v1.0 锚点段 → 源码锚点（`README.md`（maturity label Pre-1.0 line 3）+ ADR-050 §Ratification（Proposed → Accepted flip）+ 所有 active SPEC-DEFER markers）→ ADR-014（D1-D5，第三十八次激活）→ ADR-013（known limitations honest-defer，不伪造完成）。

## 1. 阶段目标

v0.39.0 ship 后，启动 v1.0 收口冲刺终点。本 phase（Phase 47 / v1.0.0）完成 v1.0.0 正式发版：maturity label flip + ADR-050 完整 ratify + v1.0.0 Release notes known-limitations + v1.0.0 tag。纯文档 + tag，0 代码逻辑改动。既有三门不退化。

**具体 exit criteria（§6 AC）**：
1. **README maturity label flip**：Pre-1.0 → **v1.0.0**（诚实里程碑声明，ADR-013）（AC1）
2. **ADR-050 完整 ratify**：Proposed → **Accepted**（D1-D4 全真实交付验证，逐 D 据 Phase 45/46 真实 CI 验证）（AC2）
3. **v1.0.0 Release notes known-limitations**：所有 active SPEC-DEFER 按 category 归类为 v1.0 known limitations（Retrieval quality / Memory / Observability / Release-CI / Interfaces / Platform，ADR-013 honest-defer——不伪造完成，显式列）（AC3）
4. **v1.0.0 closeout**：smoke v36→v37[56/56] + release docs + ADR-050 完整 ratify + roadmap/adapter（AC4）
5. ADR-014 D1-D5（第三十八次激活）全通过（AC5）

**版本号**：v1.0.0（Phase 47，承 v0.39.0），theme v1.0.0-release。**major release**（v1.0 收口终点：完整 ratify + maturity label flip + major version bump）。**不是** minor——v1.0.0 是成熟度里程碑。

## 2. 业务价值

v1.0 收口终点——把 ADR-050 完整 ratify（Proposed→Accepted）+ maturity label flip 为 v1.0.0 + 列已知限制，给用户/自己一个明确的成熟度信号：

### 47.1 v1.0.0-release（🟢 纯文档 + tag）
单聚焦 closeout task：(1) README maturity label Pre-1.0 → v1.0.0 + pin v0.39.0→v1.0.0；(2) ADR-050 §Ratification Proposed → Accepted（D1-D4 全交付验证）；(3) v1.0.0 Release notes（RELEASE_NOTES.md v1.0.0 段 + CHANGELOG [v1.0.0]）列 active SPEC-DEFER 为 known limitations；(4) smoke v37[56/56] + release docs + roadmap/adapter。

**不在本 phase 范围**：multi-user/认证身份/权限/审计合规（v2.0）/ 自动更新（v2.0）/ arm64 native runner（v2.0）/ 任何新核心能力（v1.0 是收口非新功能）。

## 3. 涉及模块

- **47.1**：`README.md`（maturity label flip + pin v1.0.0）+ `docs/decisions/adr-050-v1.0-definition.md`（完整 ratify）+ `RELEASE_NOTES.md`（v1.0.0 段 + known limitations）+ `CHANGELOG.md`（[v1.0.0] 段）+ `docs/releases/v1.0.0-evidence.md` + `v1.0.0-artifacts.md`（新增）+ `docs/roadmap.md`（§3.29 + §v1.0 锚点段 v1.0.0 ratify）+ `docs/s2v-adapter.md`（Phase 47 行 + ADR-050 Accepted）+ `scripts/console_smoke.sh`（v36→v37[56/56]）+ `internal/cli/smoke_syntax_test.go`（TestTask471）
- BDD：`test/features/phase-47-v1.0.0-release.feature`

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 47.1 | v1.0.0 closeout：maturity label flip + ADR-050 full ratify + SPEC-DEFER known-limitations catalog + smoke v37[56/56] + v1.0.0 release docs | `../tasks/task-47.1-v1.0.0-release.md` |

## 5. 依赖关系

- 47.1 dep Phase 45（D1/D2 已 ratify）+ Phase 46（D3/D4 已 ratify，D4 Release 对象首次实践成功）。
- ADR-050（完整 ratify Proposed→Accepted @ task-47.1）/ ADR-007（v1.0 分发定义，v1.0.0 正式发版）/ ADR-014（第三十八次激活）/ ADR-013（known limitations honest-defer，不伪造完成）/ ADR-004/008（守 0 dep baseline）守线。

## 6. 阶段级验收标准 + 端到端 smoke

  - [x] **AC1**（README maturity label flip 🟢 纯文档）: README Pre-1.0 → **v1.0.0** + pin v0.39.0→v1.0.0 — verified by **TEST-47.1.1**（maturity label v1.0.0 + 无 Pre-1.0 + pin v1.0.0）
  - [x] **AC2**（ADR-050 完整 ratify 🟢 纯文档）: ADR-050 Proposed → **Accepted**（D1-D4 全真实交付验证） — verified by **TEST-47.1.2**（Status Accepted + D1-D4 全 ✅）
  - [x] **AC3**（v1.0.0 known-limitations catalog 🟢 纯文档）: RELEASE_NOTES.md v1.0.0 段 + CHANGELOG [v1.0.0] 列 active SPEC-DEFER 按 category（Retrieval/Memory/Observability/Release-CI/Interfaces/Platform） — verified by **TEST-47.1.3**（known limitations 段在场 + 6 category）
  - [x] **AC4**（v1.0.0 closeout）: smoke v37[56/56] + release docs + ADR-050 完整 ratify + roadmap/adapter — verified by **TEST-47.1.4**
  - [x] **AC5**（ADR-014 cross-validation gate）: D1-D5（第三十八次激活）— verified by task-47.1 PR body + LAST TEST

## 7. 阶段级风险

- **R1（中）v1.0.0 maturity label 过早**：flip 为 v1.0.0 是否过早？
  - **缓解**：ADR-050 D1-D4 全真实交付验证（D1 recall@5/@10=1.0 超 PRD 北极星；D2 API/CLI 冻结；D3 文档对齐；D4 Release 流程）。v1.0.0 是成熟度声明不是功能声明——known limitations 诚实列出。stop-condition：无（D1-D4 已验证，v1.0.0 是诚实里程碑）。
- **R2（低）known-limitations catalog 遗漏**：~180 SPEC-DEFER markers 按类归列可能遗漏。
  - **缓解**：按 ADR-050 不含清单（multi-user/auth/auto-update/arm64）+ retrieval quality（large-corpus benchmarks）+ memory（actor auth/deprecate-harddelete）+ observability（metrics facility）+ release-CI（arm64 native/multi-os）6 大类，每类列关键 marker，不穷举（ADR-013——有意义归类非 marker 堆砌）。详档指向 roadmap §4 backlog。

## 8. Definition of Done

- 1 task spec 顶部 Status Done；§6 AC1-5 全 [x]；smoke 全 PASS。
- ADR-050 完整 ratify（Proposed → **Accepted**，D1-D4 全真实交付验证）；roadmap §v1.0 锚点段（v1.0.0 ratify）+ §3.29 + adapter。
- release：v1.0.0-{evidence,artifacts}.md + RELEASE_NOTES v1.0.0 段（known limitations）+ CHANGELOG [v1.0.0] + README（maturity label v1.0.0 + pin v1.0.0）。
- smoke：v37[56/56] + TestTask471。
- follow-up：v2.0 路线（multi-user/认证身份/自动更新/arm64 native + large-corpus benchmarks + 其余 SPEC-DEFER backlog）。
