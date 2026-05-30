# Task `19.6`: `adr-023-ratify — 据 task-19.5 真实 SemanticRecall@K flip ADR-023 Status + ADR-006 A1 转正 + ADR-008 embedding crate add-only + Phase 18 §6 AC3/AC4 解决记录（不溯改）`

**Status**: Done

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 19 (vector-retrieval-integration)
**Dependencies**: task-19.5（real dogfood embedding 语料 + `SemanticRecall@5/10` 实测 + `docs/spikes/phase-19-real-recall.md`，本 task 据其数据 ratify）/ task-19.1（real `EmbeddingProvider` 落地，使 19.5 数据非合成）/ task-19.2（默认 backend 接生产 retriever 热路径，使 recall 经真实通路）/ task-18.8（`SemanticRecall@K` 度量 + `MeetsRecallGate` + ADR-006 Amendment A1 provisional 落地）/ ADR-023 vector-backend-default（Phase 18 Proposed，本 task ratify 对象）/ ADR-006 Amendment A1（provisional → active 对象）/ ADR-008 core-library-selection（embedding crate add-only amend 对象）/ ADR-013（no fake-evidence 红线：禁据合成数据 ratify）/ ADR-014 D1-D5 第十次激活

## 1. Background

Phase 18（vector-backend-selection）ship 了向量 backend 基础设施（task-18.1 三 trait + task-18.3–18.6 四 backend spike + task-18.2 harness + task-18.8 `SemanticRecall@K` 度量），并据五维实测产出 `docs/decisions/adr-023-vector-backend-default.md`（**Status: Proposed**，D1-D6 分层选型）。但 Phase 18 closeout 时两点未闭环（见 `docs/s2v-adapter.md` Phase 18 行注）：

- **ADR-023 选型不可 ratify**：四 backend 在合成种子语料上 recall@5/10 均 = 1.0（不可区分，见 `docs/spikes/phase-18-comparison.md`），无法据此 flip D1 默认 backend 为 Accepted。ADR-023 D6 明记 ratify 须待 task-18.8 真实 embedding recall。
- **ADR-006 Amendment A1 为 provisional**：A1.3 记 `SemanticRecall@10 ≥ 0.70` 阈值为 aspirational，仓内无 embedding provider、向量 backend 未接生产 retriever，故门禁不强制 semantic 项。

Phase 19 前序 task 已补齐这两个前提：task-19.1 落 real `EmbeddingProvider`、task-19.2 把选定默认 backend 接 `Retriever` 生产热路径、task-19.5 用 real provider 对 dogfood 真实语料跑出 `SemanticRecall@5/10` 实测数据（`docs/spikes/phase-19-real-recall.md`）。

本 task = Phase 19 的**文档/决策 ratify 收口前置**：据 task-19.5 的**真实**数据，对 ADR-023 做数据驱动的 Status flip（Proposed → Accepted，**或**据实测维持 Proposed + 文档化未决），并把 ADR-006 A1 与 ADR-008 的相应 amendment 转正/补全（均 add-only），同时在 ADR 内**记录** Phase 18 §6 AC3/AC4 已在本 phase 解决——但**不溯改 Phase 18 spec**（ADR-014 D5）。

**红线（ADR-013 no fake-evidence）**：若 task-19.5 因 real embedding provider 平台/模型受阻而未能产出真实 recall（见 phase §7 R1 stop-condition），则**禁止** flip ADR-023 至 Accepted——按实测缺口诚实维持 Proposed + documented 未决，ADR-006 A1 同步维持 provisional。

## 2. Goal

据 task-19.5 `docs/spikes/phase-19-real-recall.md` 的真实 `SemanticRecall@5/10`，完成三处决策文档的数据驱动 amendment（全 add-only，不改既有 Decision/Rationale 正文）：

1. **`docs/decisions/adr-023-vector-backend-default.md`**：以 **Amendment / Ratification** 段记录实测值与判定；`SemanticRecall@10 ≥ 0.70` 且默认 backend 数据成立 → Status `Proposed → Accepted`（D1 默认 backend 转正）；否则（数据 < 阈值 / 不可得）→ **维持 Proposed** + 在新段落 documented 未决（实测值 + 缺口 + 下一步），不动 Status。
2. **`docs/decisions/adr-006-recall-eval-acceptance-gate.md`**：Amendment A1 Status `Proposed/provisional → active`（仅当 `SemanticRecall@10 ≥ 0.70` 真实达阈值；否则记实测值 + 维持 provisional），add-only 追加 ratification 注，不改 A1.1/A1.2/A1.3 既有正文。
3. **`docs/decisions/adr-008-core-library-selection.md`**：add-only Amendment 记 task-19.1 选定的 embedding provider crate 入 Rust 核心库列表（feature-gated optional dep，默认构建 0 新 dep）。

并在 ADR-023 ratification 段**记录** Phase 18 §6 AC3（ADR-023 ratify，原 partial=Proposed）/ AC4（生产向量检索集成，原 deferred）已在 Phase 19 解决——**不溯改** `docs/specs/phases/phase-18-vector-backend-selection.md`（D5）。默认 `cargo test --workspace` + `go test ./...` 不退化（本 task 只触文档，应天然不退化，§9 实跑佐证）；D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **修改 `docs/decisions/adr-023-vector-backend-default.md`** — 新增 **Amendment / Ratification (2026-…, Phase 19 task-19.6)** 段：
  - 引用 task-19.5 `docs/spikes/phase-19-real-recall.md` 的真实 `SemanticRecall@5/10`（provider 名 + corpus 来源 + K + 数值），与 Phase 18 合成 recall=1.0 并列对照（说明真实分布上选型可区分）。
  - **判定分支**：实测 `SemanticRecall@10 ≥ 0.70` 且默认 backend（task-19.2 选定，ADR-023 D1/D2）数据成立 → 顶部 **Status: Proposed → Accepted**，记 D1 默认 backend 转正 + ratify 依据；否则 → **维持 Status: Proposed**，新段记实测值 + 阈值缺口 + 下一步 owner（不动顶部 Status）。
  - 记 Phase 18 §6 AC3（ADR-023 ratify）/ AC4（生产向量检索集成）在 Phase 19 解决 + 指向 task-19.2/19.3/19.5（**不溯改** Phase 18 spec，D5）。
  - 既有 Context/Decision(D1-D6)/Consequences 正文**不改**（add-only）。
- **修改 `docs/decisions/adr-006-recall-eval-acceptance-gate.md`** — Amendment A1 add-only ratification 注：
  - `SemanticRecall@10 ≥ 0.70` 真实达阈值 → A1 Status `Proposed/provisional → active`，记 ratify 依据（task-19.5 真实数据 + real provider + 生产 wiring）；A1.3 provisional 限制以 **superseded-by** 注追加（不删 A1.3 原文）。
  - 否则 → 维持 provisional，新行记实测值 + 维持原因。
  - A1.1（指标）/ A1.2（阈值表）/ A1.3（provisional）既有正文**不改**。
- **修改 `docs/decisions/adr-008-core-library-selection.md`** — add-only **Amendment** 段：task-19.1 选定 embedding provider crate（fastembed / candle / ort，依 19.1 实选）入 Rust 核心库列表，标注 feature-gated optional + 默认构建 0 新 dep + deterministic 缺省/兜底 provider 无模型 dep；既有 Decision/Rationale/Alternatives **不改**。
- **修改 `docs/s2v-adapter.md`** — §ADR 索引 ADR-023 状态行更新（Accepted 或维持 Proposed+注）；Phase 19 表 19.6 行 Pending → Done（本 task 收尾时）；不动 Phase 18 行（D5）。

### Out of Scope（[SPEC-DEFER] / [SPEC-OWNER]）

- **真实 recall 数据生产**（real provider + dogfood 语料 + `SemanticRecall@K` 实跑）[SPEC-OWNER:task-19.5-real-recall-eval]：本 task **消费** 19.5 数据做 ratify，不产出数据。
- **默认 backend 选型决策 + 生产 wiring** [SPEC-OWNER:task-19.2-default-backend-wiring]：D1/D2 默认 backend 接 retriever 热路径由 19.2 完成，本 task 仅据其结果在 ADR 记录。
- **embedding provider 选型 + 落地** [SPEC-OWNER:task-19.1-spike-embedding-provider]：crate 选定由 19.1，本 task 仅 add-only 记入 ADR-008。
- **Phase 19 closeout + v0.12.0 release + tag** [SPEC-OWNER:task-19.7-closeout-v0.12.0]：adapter §Phase 索引 flip / release docs / tag push 由 19.7。
- **溯改 Phase 18 spec §6 AC3/AC4 勾选状态** [SPEC-DEFER:phase-future.no-retro-edit-d5]：ADR-014 D5 禁溯改历史 phase；本 task 在 Phase 19 侧记录解决，不动 Phase 18 文件。
- **ADR-013 flip to Accepted** [SPEC-DEFER:phase-future.adr-013-no-flip-without-real-recall]：无真实 recall 数据不得据本 task flip 任何 Status 至 Accepted——若 19.5 数据不可得则全部维持 Proposed/provisional。
- **remote embedding provider ADR** [SPEC-DEFER:phase-future.embedding-provider-remote]：承 phase §不在 scope，本地 provider only。

## 4. Actors

- **主 agent**：实施 + PR 主理（文档/决策 amendment）。
- **task-19.5 `docs/spikes/phase-19-real-recall.md`**：真实 `SemanticRecall@K` 数据源，本 task 唯一 ratify 依据。
- **`docs/decisions/adr-023-vector-backend-default.md`**：ratify 主对象（Status flip 或 documented 未决）。
- **`docs/decisions/adr-006-recall-eval-acceptance-gate.md`** Amendment A1：provisional → active 对象。
- **`docs/decisions/adr-008-core-library-selection.md`**：embedding crate add-only 入库对象。
- **下游 task-19.7**：消费本 task 的 ADR-023 终态做 adapter §Phase 索引 flip + release docs。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-19-vector-retrieval-integration.md` §1 exit criteria / §3.6 模块 / §5 依赖 / §6 AC5 / §7 R1-R2 / §8 DoD（ratify 条款 + ADR-013 禁据合成 ratify）
- `docs/specs/tasks/task-19.5-real-recall-eval.md` §6 AC（真实 `SemanticRecall@K` 产出契约）+ `docs/spikes/phase-19-real-recall.md`（数据本体）
- `docs/specs/tasks/task-19.1-spike-embedding-provider.md`（选定 crate 名，feed ADR-008 amend）+ `docs/specs/tasks/task-19.2-default-backend-wiring.md`（选定默认 backend，feed ADR-023 D1/D2 ratify）
- `docs/decisions/adr-023-vector-backend-default.md`（D1-D6 + D6 ratify 前提）+ `docs/spikes/phase-18-comparison.md`（合成 recall=1.0 不可区分背景）
- `docs/decisions/adr-006-recall-eval-acceptance-gate.md` Amendment A1（A1.1 指标 / A1.2 阈值表 / A1.3 provisional）+ `docs/specs/tasks/task-18.8-eval-semantic-recall.md`（度量 + `MeetsRecallGate` + `GateSemanticRecall10Min = 0.70`）
- `docs/decisions/adr-008-core-library-selection.md`（add-only amend 落点）
- `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5，尤 D5 不溯改历史 phase）+ `scripts/spec_drift_lint.sh`（D2 lint 口径）

### 5.2 Amendment 不改既有正文（全 add-only）

本 task 触三份 ADR 均**仅追加**新段落 / 仅改顶部 `**Status**` 行（ADR-023 在 ratify 分支命中时），**不删改** Decision / Rationale / Alternatives / Consequences / 既有 Amendment 正文。仿 ADR-006 Amendment A1（task-18.8 落地）+ ADR-015/022 add-only contract pattern。

### 5.3 关键判定逻辑（ratify gate）

```
data = read(docs/spikes/phase-19-real-recall.md)   # task-19.5 真实 SemanticRecall@5/10
if data.is_real (real provider + dogfood corpus, 非合成 / 非 deterministic-only):
    if data.SemanticRecall@10 >= 0.70 and default_backend(task-19.2) data 成立:
        ADR-023.Status = Proposed -> Accepted          # D1 默认 backend ratify
        ADR-006.A1.Status = provisional -> active        # gate 转正
    else:
        ADR-023.Status = Proposed (维持) + documented 未决（实测值 + 缺口 + 下一步）
        ADR-006.A1.Status = provisional (维持) + 实测值记录
else:   # 真实数据不可得（phase §7 R1 stop-condition 命中）
    ADR-023.Status = Proposed (维持) + 诚实记缺口   # ADR-013 禁据合成/deterministic ratify
    ADR-006.A1.Status = provisional (维持)
ADR-008: add-only 记 task-19.1 embedding crate（与 ratify 分支无关，恒执行）
ADR-023/Phase19 侧: 记 Phase 18 §6 AC3/AC4 已解决（不溯改 Phase 18 spec, D5）
```

- **ratify 与否取决于真实数据**：唯一信源是 task-19.5 spike 文件；无真实数据 → 不 flip（ADR-013）。
- **ADR-008 amend 恒执行**：embedding crate add-only 入库与 recall 阈值无关（19.1 已选定 crate）。
- **Phase 18 AC3/AC4 解决记录**：写在 ADR-023 ratification 段 + 本 task §10，**不**回写 Phase 18 spec（D5）。

## 6. Acceptance Criteria

- [x] **AC1**: ADR-023 Status flip 据真实数据 — task-19.5 `docs/spikes/phase-19-real-recall.md` 真实 `SemanticRecall@10 = 0.9333 ≥ 0.70`（exact-cosine，代表 D1 sqlite-vec/实现默认 brute-force）→ `docs/decisions/adr-023-vector-backend-default.md` 顶部 `**Status**` Proposed → **Accepted** + Amendment/Ratification 段。据真实非合成数据 ratify（ADR-013：spike 数据源声明 real `FastEmbedProvider` run）— verified by **TEST-19.6.1**
- [x] **AC2**: ADR-023 amendment add-only + 数据引用 — Amendment/Ratification 段引用真实 `SemanticRecall@5=0.8333/@10=0.9333`（provider `all-MiniLM-L6-v2` + corpus 40-chunk + K + top1/MRR）+ 与 Phase 18 合成 1.0 对照表；既有 Context/Decision(D1-D6)/Consequences 正文逐字不改（仅追加段 + 顶部 Status 行）— verified by **TEST-19.6.2**
- [x] **AC3**: ADR-006 Amendment A1 转正据阈值 — `SemanticRecall@10=0.9333 ≥ 0.70` 真实达阈 → A1 Status Proposed/provisional → **Active**（A1 头部 Status 行 + 新增 A1.4 ratification 段，A1.3 provisional 由 A1.4 superseded 不删原文）；A1.1/A1.2/A1.3 既有正文不改 — verified by **TEST-19.6.3**
- [x] **AC4**: ADR-008 embedding crate add-only — `docs/decisions/adr-008-core-library-selection.md` 新增 Amendment 记 task-19.1 选定 `fastembed`（all-MiniLM-L6-v2 dim 384，feature-gated `embedding-fastembed` optional + 默认构建 0 新 dep + rustls 跨平台 + deterministic 缺省 provider 无模型 dep）；既有 Decision/Rationale/Alternatives 不改 — verified by **TEST-19.6.4**
- [x] **AC5**: Phase 18 AC3/AC4 解决记录且不溯改（D5）— ADR-023 Amendment/Ratification 段记 Phase 18 §6 AC3（ratify）/ AC4（生产向量检索集成）在 Phase 19 解决并指向 task-19.2/19.3/19.5；`docs/specs/phases/phase-18-vector-backend-selection.md` 未被本 task 触碰（git diff --stat master 为空）— verified by **TEST-19.6.5**
- [x] **AC6**: 既有不退化 + D2 lint — 本 task 只触文档（零代码改动）；`bash scripts/spec_drift_lint.sh --touched master` PR 触及行 0 未标注命中（CI spec-lint gate 复核；cargo-test/go-test gate 天然不退化）— verified by **TEST-19.6.6**

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-19.6.1 | ADR-023 Status Proposed→Accepted 据真实 recall@10=0.9333（非合成） | `docs/decisions/adr-023-vector-backend-default.md` | Done |
| TEST-19.6.2 | ADR-023 Amendment/Ratification 段引用 19.5 真实数据 + 对照表 + 既有 D1-D6 正文 add-only | `docs/decisions/adr-023-vector-backend-default.md` | Done |
| TEST-19.6.3 | ADR-006 A1 Proposed/provisional→Active + A1.4 add-only（A1.1-A1.3 原文不改） | `docs/decisions/adr-006-recall-eval-acceptance-gate.md` | Done |
| TEST-19.6.4 | ADR-008 embedding crate（fastembed）add-only 入库 | `docs/decisions/adr-008-core-library-selection.md` | Done |
| TEST-19.6.5 | Phase 18 AC3/AC4 解决记录 + Phase 18 spec diff 空（D5） | ADR-023 段 + `git diff docs/specs/phases/phase-18-*`（空，见 §10） | Done |
| TEST-19.6.6 | 零代码改动（cargo/go 天然不退化）+ D2 lint 0 unannotated | `scripts/spec_drift_lint.sh` + CI gate | Done |

## 8. Risks

- **R1（高）真实 recall 数据不可得 → 无法 ratify（承 phase §7 R1）**：task-19.5 依赖 real embedding provider；若 provider 在 Linux + Windows MSVC 均受阻（phase §7 R1 stop-condition），则无真实 `SemanticRecall@K`，ADR-023 不可 flip。
  - **缓解**：判定逻辑（§5.3）内置「数据不可得 → 维持 Proposed + 诚实记缺口」分支；ADR-013 红线禁据合成/deterministic-only ratify；本 task 仍完成 ADR-008 add-only + Phase 18 AC3/AC4 记录（与 recall 无关部分），ratify 项 documented 未决，不阻塞 task-19.7 收口（仿 Phase 18 closeout 缩范围 pattern）。
- **R2（中）实测 < 0.70 阈值**：真实分布上 `SemanticRecall@10` 可能低于 ADR-006 A1.2 阈值。
  - **缓解**：维持 ADR-023 Proposed + ADR-006 A1 provisional，新段记实测值 + 缺口 + 下一步 owner；不弱化阈值就 flip（避免 self-serving ratify）；阈值/调参后置由后续 phase。
- **R3（中）溯改 Phase 18 spec 违反 D5**：记录「Phase 18 AC3/AC4 解决」易诱导回写 Phase 18 spec 勾选。
  - **缓解**：解决记录只写 ADR-023 ratification 段 + 本 task §10；TEST-19.6.5 断言 Phase 18 spec git diff 为空；D2 lint `--touched master` 守触及行。
- **R4（低）amendment 误改既有 Decision 正文（非 add-only）**：三 ADR amend 可能误触既有段落。
  - **缓解**：仅追加段 / 仅改 ADR-023 顶部 Status 行；TEST-19.6.2/.3/.4 对照既有正文逐字未改；PR diff review。

## 9. Verification Plan

```bash
# 真实数据前置（task-19.5 产出，本 task 消费；存在性 + 真实性 gate）
test -f docs/spikes/phase-19-real-recall.md
grep -E 'SemanticRecall@(5|10)' docs/spikes/phase-19-real-recall.md   # 真实数值存在

# ADR-023 ratify 据真实数据（AC1/AC2）：Status 与 19.5 数据一致；既有 D1-D6 正文未改
grep -E '^\*\*Status\*\*' docs/decisions/adr-023-vector-backend-default.md
git -c core.pager=cat diff master -- docs/decisions/adr-023-vector-backend-default.md   # 仅 add-only + Status 行

# ADR-006 A1 转正/维持（AC3）+ ADR-008 embedding crate add-only（AC4）
git -c core.pager=cat diff master -- docs/decisions/adr-006-recall-eval-acceptance-gate.md
git -c core.pager=cat diff master -- docs/decisions/adr-008-core-library-selection.md

# Phase 18 spec 未被触碰（AC5，D5 不溯改）
git -c core.pager=cat diff --stat master -- docs/specs/phases/phase-18-vector-backend-selection.md   # 期望空

# 既有不退化（AC6，本 task 只触文档）
cargo test --workspace
go test ./...

# D2 spec-drift lint（AC6）
bash scripts/spec_drift_lint.sh --touched master
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**：2026-05-30
- **改动文件**：`docs/decisions/adr-023-vector-backend-default.md`（顶部 Status Proposed→Accepted + 新增 Amendment/Ratification 段；Context/Decision D1-D6/Consequences 不改）、`docs/decisions/adr-006-recall-eval-acceptance-gate.md`（A1 头部 Status Proposed/provisional→Active + 新增 A1.4 ratification 段；A1.1/A1.2/A1.3 不改）、`docs/decisions/adr-008-core-library-selection.md`（新增 Amendment：fastembed embedding crate；Decision/Rationale/Alternatives 不改）、`docs/s2v-adapter.md`（§ADR 索引 ADR-023 Accepted + Phase 19 表 19.6 行 Done；Phase 18 行未动）、`docs/specs/tasks/task-19.6-adr-023-ratify.md`（本 spec）。**纯文档 task，零代码改动**。
- **commit 列表**：见本 task PR（分支 `feat/task-19.6-adr-023-ratify`）；合入后以 merge commit 为准
- **§9 Verification 结果**：ratify 据真实数据——task-19.5 `SemanticRecall@10 = 0.9333 ≥ 0.70`（@5=0.8333，top1=0.60，MRR=0.70，exact-cosine real `fastembed-all-minilm-l6-v2`）→ ADR-023 终态 **Status: Accepted**、ADR-006 A1 终态 **Active**。add-only 实证：`git diff master` 仅 2 处删行 = ADR-023 顶部 Status 行 + ADR-006 A1 头部 Status 行（两处允许的 Status flip），其余全为追加段，既有 Decision/正文逐字未改。**D5 实证**：`git diff --stat master -- docs/specs/phases/phase-18-vector-backend-selection.md` = **空**（Phase 18 spec 未被触碰；AC3/AC4 解决记录写在 ADR-023 Amendment 段 + 本 §10）。本 task 零代码改动 → `cargo test --workspace` / `go test ./...` 天然不退化（CI gate 复核）；D2 lint `--touched master` 0 未标注命中（CI spec-lint gate 复核）。
- **Phase 18 §6 AC3/AC4 解决记录（不溯改 D5）**：AC3（ADR-023 ratify，原 partial=Proposed）由本 task ratify 解决；AC4（生产向量检索集成，原 deferred）由 task-19.2（默认 backend 接 `Retriever`）+ task-19.3（`/v1/search?semantic=true` → core gRPC semantic）+ task-19.5（真实召回经该通路实测）解决。记录落 ADR-023 Amendment/Ratification 段，**不回写** Phase 18 spec。
- **剩余风险 / 未做项**：real recall R1 stop 未触发，故无「维持 Proposed」分支（数据达阈，Accepted）。remote embedding provider ADR [SPEC-DEFER:phase-future.embedding-provider-remote]；hnsw graph persistence [SPEC-DEFER:phase-future.hnsw-graph-persistence]（承 ADR-023 follow-up，不在本 ratify scope）。
- **下游 task 影响**：task-19.7（消费 ADR-023/006 终态做 adapter §Phase 索引 Phase 19 flip + v0.12.0 release docs；Phase 18 forward-ref 解除；v0.12.0 tag 需用户授权）
- **ADR-014 D1-D5 第十次激活**：D1 本 task §6 AC1-AC6 ↔ phase-19 §6 AC5（ratify ADR-023）mapping；D2 lint `--touched master` 0 unannotated（CI spec-lint）；D3 每 AC verified-by TEST-19.6.x；D4 主 agent ADR-012 自治据真实数据 ratify（非据合成，ADR-013）；D5 Phase 18 spec git diff 空实证（上记）。
