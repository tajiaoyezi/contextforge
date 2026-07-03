# Phase 46 · v1.0-docs-and-release-flow

**Status**: Ready

> Phase Spec（s2v full-standard §8.2）。本 phase 是 **v1.0 收口冲刺的第二步**（承 ADR-050 v1.0 定义 D3 + D4 维度）。Phase 45 已交付 D1（能力，已满足）+ D2（API/CLI 冻结）；本 phase 交付 **D3 文档对齐** + **D4 GitHub Release 流程**，为 Phase 47 / v1.0.0 正式发版铺平最后两维。
>
> **Grounding（ADR-013）** 四项实测缺口：(1) **README** 776 行中 **38 个 `## What's new` changelog 段**（v0.3.0→v0.38.0）占 ~85%，访客看不到 Features；末尾残留 `## v0.2 limitations` 过时段（"does not publish a GitHub Release object" 正是 D4 要改的）；"Run the released image" 写死 `v0.28.0`（当前 v0.38.0）；"Where to go next" 引用 v0.1 未实装。(2) **CHANGELOG.md** 不存在（v1.0 需对外标准 changelog；RELEASE_NOTES.md 是内部详档，不可替代）。(3) **ADR 访客索引** 缺（adapter 内部有 50 ADR 表，但 `docs/decisions/` 无面向访客的分类导航 README；50 个决策散在文件名里，无 category / 一句话摘要导航）。(4) **release.yml** 101 行只推 image 到 GHCR + cosign sign/attest，**无 GitHub Release 对象自动创建**（tag push 时）——README 自承 "does not publish a GitHub Release object"。
>
> 本 phase 全程 **0 代码逻辑改动 / 0 新 dep / 0 migration / 0 proto / 0 schema change**（纯文档 + 1 个 CI step）。默认行为 / 既有契约 / 三门不退化。

> **入读顺序**：本 phase spec → ADR-050（v1.0 定义，D3/D4 本 phase ratify）→ roadmap §v1.0 锚点段 → 源码锚点（`README.md`（776 行 / 38 changelog 段 / v0.2 limitations / v0.28.0 pin）+ `RELEASE_NOTES.md`（1734 行内部详档）+ `.github/workflows/release.yml`（101 行无 Release step））→ ADR-014（D1-D5，第三十七次激活）→ ADR-013（D4 缺口真实可验证；README maturity label 不虚标 v1.0）→ ADR-007（分发定义 Amendment）。

## 1. 阶段目标

v0.38.0 ship 后，启动 v1.0 收口冲刺第二步。本 phase（Phase 46 / v0.39.0）交付 ADR-050 剩余两维：**D3 文档对齐**（README 重构 + CHANGELOG.md + ADR 访客索引）+ **D4 GitHub Release 流程**（release.yml 加 Release 对象自动创建）。纯文档 + CI，0 代码逻辑改动。既有三门不退化。

**具体 exit criteria（§6 AC）**：
1. **D3 文档对齐 — README 重构**：删 38 个 `What's new` 段（已在 RELEASE_NOTES.md）+ 删 `v0.2 limitations` 过时段 + 新增 **Features 汇总段** + 刷新版本 pin（v0.28.0→current）+ 加 **maturity label**（"Pre-1.0 收口中"，诚实不虚标 v1.0，ADR-013）（AC1）
2. **D3 文档对齐 — CHANGELOG.md + ADR 索引**：建 `CHANGELOG.md`（Keep a Changelog 格式）+ `docs/decisions/README.md`（50 ADR 分类导航 + 一句话摘要）（AC2）
3. **D4 GitHub Release 流程**：release.yml 加 `softprops/action-gh-release@v2` step（tag push 触发，body 引用 RELEASE_NOTES.md + 标注 cosign/SBOM provenance）；README 同步删 "does not publish a GitHub Release object" 过时声明（AC3）
4. **v0.39.0 closeout**：smoke v35→v36[55/55] + release docs + ADR-050 D3/D4 ratify + roadmap/adapter（AC4）
5. ADR-014 D1-D5（第三十七次激活）全通过（AC5）

**版本号**：v0.39.0（Phase 46，承 v0.38.0），theme v1.0-docs-and-release-flow。minor release（v1.0 收口第二步：文档对齐 + 发布流程；纯文档 + CI，无 breaking change）。

## 2. 业务价值

v1.0 收口第二步——把 ADR-050 剩余 D3/D4 两维落地，为 Phase 47 / v1.0.0 正式发版铺平：

### 46.1 readme-restructure（🟢 纯文档）
README 从 "38 段 changelog 污染 + 过时 v0.2 limitations + 写死 v0.28.0" 重构为 "**Features 汇总优先 + maturity label + 当前版本 pin**" 的访客友好结构。删的 38 段已在 RELEASE_NOTES.md（内部详档，1734 行），README 只保留最新 1-2 版要点 + 指向 RELEASE_NOTES.md。

### 46.2 changelog-and-adr-index（🟢 纯文档）
- 建 `CHANGELOG.md`（Keep a Changelog 1.1.0 格式，从 RELEASE_NOTES.md 提炼 v0.1→v0.38.0 关键里程碑——非全文搬运，是对外简表）。
- 建 `docs/decisions/README.md`（50 ADR 按 category 分组：Architecture / Storage / Retrieval / Release / Governance；每条一句话摘要 + status + 链接）。与 adapter 内部表格互补（adapter 是 s2v 治理表，此 README 是访客导航）。

### 46.3 release-flow-and-closeout（🟢 CI + 文档）
- release.yml 加 GitHub Release 对象自动创建（`softprops/action-gh-release@v2`，tag push 触发，body 从 RELEASE_NOTES.md 对应版本段拼接 + 标注 cosign/SBOM provenance 链接）。
- README 同步删 "does not publish a GitHub Release object" 过时声明（task-46.1 删 v0.2 limitations 时已触，本 task 确保 Release 流程落地后声明一致）。
- closeout：smoke v36[55/55] + release docs + ADR-050 D3/D4 ratify + roadmap/adapter。

**不在本 phase 范围**：v1.0 正式发版（Phase 47；含 maturity label Pre-1.0→v1.0.0 flip + v1.0.0 tag + 所有 SPEC-DEFER 列已知限制）/ multi-user/认证身份/自动更新/arm64 native（v2.0）。

## 3. 涉及模块

- **46.1**：`README.md`（重构：删 38 changelog 段 + v0.2 limitations + 加 Features 段 + maturity label + 刷新 pin）
- **46.2**：`CHANGELOG.md`（新增，Keep a Changelog 格式）+ `docs/decisions/README.md`（新增，ADR 访客索引）
- **46.3**：`.github/workflows/release.yml`（加 Release 对象 step）+ `README.md`（同步 Release 声明）+ smoke v35→v36[55/55] + TestTask463 + release docs + ADR-050 D3/D4 ratify + roadmap/adapter
- BDD：`test/features/phase-46-v1.0-docs-and-release-flow.feature`

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 46.1 | README 重构（删 38 changelog + v0.2 limitations + Features 汇总 + maturity label + pin 刷新）| `../tasks/task-46.1-readme-restructure.md` |
| 46.2 | CHANGELOG.md（Keep a Changelog）+ docs/decisions/README.md（ADR 访客索引）| `../tasks/task-46.2-changelog-and-adr-index.md` |
| 46.3 | release.yml GitHub Release 对象 + smoke v36[55/55] + v0.39.0 closeout + ADR-050 D3/D4 ratify | `../tasks/task-46.3-release-flow-and-closeout.md` |

## 5. 依赖关系

- 46.1（README 重构）无 dep，可先行。
- 46.2（CHANGELOG + ADR 索引）无 dep，与 46.1 并行无冲突。
- 46.3（release.yml + closeout）dep 46.1（README Release 声明同步需 task-46.1 先删 v0.2 limitations）+ 46.2（CHANGELOG 首版就绪供 Release body 引用）。closeout dep 全部。
- ADR-050（D3/D4 ratify @ task-46.3）/ ADR-007（分发定义 Amendment @ task-46.3，D4 落地后 GitHub Release 对象成为分发物之一）/ ADR-014（第三十七次激活）/ ADR-013（maturity label 不虚标）/ ADR-004/008（守 0 dep baseline）守线。

## 6. 阶段级验收标准 + 端到端 smoke

- [ ] **AC1**（D3 README 重构 🟢 纯文档）: README 删 38 个 `What's new` 段 + 删 `v0.2 limitations` + 新增 Features 汇总段 + maturity label（Pre-1.0 收口中）+ 刷新版本 pin（写死 v0.28.0→current） — verified by **TEST-46.1.1**（Features 段在场 + maturity label + pin = current + 无 38 changelog 段）
- [ ] **AC2**（D3 CHANGELOG + ADR 索引 🟢 纯文档）: `CHANGELOG.md`（Keep a Changelog 格式）+ `docs/decisions/README.md`（50 ADR 分类导航 + 一句话摘要） — verified by **TEST-46.2.1**（CHANGELOG 在场 + Keep a Changelog 头）+ **TEST-46.2.2**（ADR 索引 50 条 + category 分组）
- [ ] **AC3**（D4 GitHub Release 流程 🟢 CI + 文档）: release.yml 加 `softprops/action-gh-release@v2` step（tag push 触发）+ README 删 "does not publish a GitHub Release object" 过时声明 — verified by **TEST-46.3.1**（release.yml Release step 在场）+ **TEST-46.3.2**（README 无过时声明）
- [ ] **AC4**（v0.39.0 closeout + ADR-050 D3/D4 ratify）: smoke v36[55/55] + release docs + ADR-050 D3/D4 ratify + roadmap/adapter — verified by **TEST-46.3.3**
- [ ] **AC5**（ADR-014 cross-validation gate）: D1-D5（第三十七次激活）— verified by task-46.3 PR body + LAST TEST

## 7. 阶段级风险

- **R1（低）release.yml Release step 首次实践失败**：`softprops/action-gh-release` 是成熟 action，但首次接入（无 v0.39.0 实跑前无法验证 Release 对象真实创建）。
  - **缓解**：action 配置严格按官方文档（permissions: contents: write 已在 release.yml 顶部声明）；task-46.3 加 yaml 结构 lint；v0.39.0 tag push 时实测（若失败 → v0.39.1 修或 honest-defer Release 对象到 v1.0.0）。stop-condition：action 版本/权限问题导致 Release 对象无法创建 → 转 §8 honest-defer，Release 对象推 v1.0.0 首次实践。
- **R2（低）CHANGELOG 提炼遗漏里程碑**：从 1734 行 RELEASE_NOTES.md 提炼对外简表，可能漏关键版本。
  - **缓解**：按 git tag 历史（v0.1→v0.38.0）逐版本核；task-46.2 grep tag 列表交叉验证。stop-condition：若某版本里程碑无法从 RELEASE_NOTES.md 确认 → 该版本行 honest-defer 或省略（非伪造）。
- **R3（低）spec_drift_lint 反模式词**：CHANGELOG / ADR 索引文档中若误用 spec-lint 禁词（未实装替代词 / 脚手架 / 桩 / 模拟对象类）触发 CI spec-lint。
  - **缓解**：task-46.2 写完后 grep 自检；文档上下文里这些词即使出现也是历史描述（如 "v0.1 的 import/eval 是未实装"），spec_drift_lint 的 `--touched` 模式只 lint PR 增量行，历史引用不在增量内。

## 8. Definition of Done

- 3 task spec 顶部 Status Done；§6 AC1-5 全 [x]；smoke 全 PASS。
- ADR-050 D3/D4 ratify（Proposed → Accepted for D3/D4；完整 ratify 待 Phase 47 v1.0.0）；ADR-007 add-only Amendment（分发定义补 GitHub Release 对象）；roadmap §v1.0 锚点段（Phase 46 落地记录）+ §3.28 + adapter。
- release：v0.39.0-{evidence,artifacts}.md + RELEASE_NOTES + README（重构后 + Features + maturity label）+ CHANGELOG.md（首版）+ docs/decisions/README.md（ADR 索引）。
- smoke：v36[55/55] + TestTask463。
- follow-up：v1.0 正式发版（Phase 47；maturity label flip + v1.0.0 tag + SPEC-DEFER 列已知限制 + ADR-050 完整 ratify）/ v2.0 路线（multi-user/自动更新/arm64）。
