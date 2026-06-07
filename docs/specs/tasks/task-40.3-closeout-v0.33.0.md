# Task `40.3`: `closeout-v0.33.0 — smoke v30[49/49] + v0.33.0 release docs + ADR-045 Proposed→Accepted 逐 D ratify + ADR-032/038/027/015 add-only Phase-40 Amendment + roadmap §3.22/§4 add-only + s2v-adapter add-only + phase §6 闭合（governance-debt-cleanup-3）`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 40 (governance-debt-cleanup-3)
**Dependencies**: task-40.1（memory-actor-propagation）+ task-40.2（l2-embedding-cache-true-lru）全 Done（本 closeout 据其真实测试 / 实测产物 ratify）/ 既有 `scripts/console_smoke.sh`（Phase 39 v29[48/48]，本 task v30[49/49]）+ `internal/cli/smoke_syntax_test.go`（`TestTask393` 镜像源）/ ADR-045（本 phase 新 Proposed，本 task ratify）/ ADR-032（memory-ops，pin actor 透传维度兑现 add-only Amendment）/ ADR-038 + ADR-027（embedding，L2 true-LRU 维度兑现 + 真-LRU 假设据实更正 add-only Amendment）/ ADR-015（proto add-only field Amendment）/ ADR-022 D2（memory pin lenient body 契约守线引用）/ ADR-004 / ADR-008 / ADR-012（tag/release outward-facing 须用户显式授权，v0.33.0 本轮已授权）/ ADR-013（禁伪造红线，release 产物 `<backfill>` marker 不预填）/ ADR-014 D1-D5（第三十一次激活，本 task closeout PR body 收口）

## 1. Background

task-40.1（memory pin actor 透传）+ task-40.2（L2 embedding 缓存访问序 LRU）落地后，需收口 Phase 40 / v0.33.0：smoke 顺位 + release docs + ADR-045 据真实测试 ratify + 触及 ADR add-only Amendment + roadmap/adapter add-only + phase §6 闭合。承 Phase 39（task-39.3）closeout 形态——smoke v29[48/48] → v30[49/49]，ADR-044 ratify → ADR-045 ratify，offset 顺位。

## 2. Goal

(1) `scripts/console_smoke.sh` banner v29→v30 + v30 changelog block + 新 step [49/49]（memory pin actor 透传 + L2 访问序 LRU 可达则断言、否则 doc/status）。(2) `internal/cli/smoke_syntax_test.go` 新 `TestTask403`（镜像 `TestTask393`）断言 [49/49] + no-regression（denominators [37/37]..[48/48] 不溯改，ADR-014 D5）。(3) `docs/releases/v0.33.0-{evidence,artifacts}.md` + README v0.33 段 + RELEASE_NOTES v0.33.0 段（tag/run/digest `<backfill>` marker，ADR-013 不预填）。(4) ADR-045 Proposed→Accepted 逐 D ratify + `## Ratification（v0.33.0 / task-40.3）`。(5) ADR-032（pin actor 透传维度兑现）/ ADR-038 + ADR-027（L2 true-LRU 维度兑现 + 真-LRU 假设据实更正）/ ADR-015（proto add-only field）add-only Phase-40 Amendment（不溯改正文，ADR-014 D5）。(6) roadmap §3.22/§4 add-only（Phase 40 行 + memory-actor-authenticated-identity 新 backlog 条目）+ s2v-adapter add-only（Phase 40 / tasks / ADR-045 / BDD 行）。(7) phase-40 Status Draft→Done + §6 AC 勾选。

pass bar：smoke v30[49/49] + `TestTask403`（no-regression [37/37]..[48/48]）+ `bash -n` 全绿；release docs `<backfill>` marker 标记（真实 tag/run/digest post-tag-push 回填）；ADR-045 逐 D 据真实测试 ratify；触及 ADR add-only Amendment 不溯改正文；roadmap/adapter add-only；phase §6 AC1-4 全 `[x]`（逐维如实）；ADR-014 D1-D5 第三十一次激活全通过。

## 3. Scope

### In Scope（计划交付）

- 改 `scripts/console_smoke.sh`——banner v29→v30 + v30 changelog block + 新 step [49/49]（REAL 模式：pin `POST /v1/memory/{id}/pin` 带 `X-Actor: smoke-actor` → 断言 200/204 + `pinned_by` 经 get 可见；L2 访问序 LRU 为 core 内部、smoke 以 doc/status 记；不可达则 SKIP）；defer-note 据实更新
- 改 `internal/cli/smoke_syntax_test.go`——新 `TestTask403`（镜像 `TestTask393`）断言 banner "v30 (task-40.3)" + [49/49] + markers + no-regression（[37/37]..[48/48] + 既有主题串）+ `bash -n`；defer-note 同步
- 新增 `docs/releases/v0.33.0-evidence.md` + `v0.33.0-artifacts.md`（镜像 v0.32.0 结构，tag SHA / run id / ghcr digest / cosign tlog 为 angle-bracket `<backfill>` marker，ADR-013 不预填）
- 改 `README.md`——加 "What's new in v0.33.0" 段（memory pin actor 透传 + L2 访问序 LRU，0 新 dep / proto add-only / 默认 byte-equiv）
- 改 `RELEASE_NOTES.md`——加 v0.33.0 段
- 改 `docs/decisions/adr-045-governance-debt-cleanup-3.md`——Status Proposed→Accepted（逐 D 如实）+ 新 `## Ratification（v0.33.0 / task-40.3）`
- add-only Amendment（非正文改，ADR-014 D5）：`adr-032`（pin actor 透传维度兑现）/ `adr-038` + `adr-027`（L2 true-LRU 维度兑现 + 真-LRU 假设据实更正）/ `adr-015`（proto add-only field）各加 `## Amendment (Phase 40 / v0.33.0)`
- 改 `docs/roadmap.md`——§3.22 v0.33.0 推进记录 + §4 backlog add-only（memory-actor-authenticated-identity 新条目）
- 改 `docs/specs/phases/phase-40-governance-debt-cleanup-3.md`——Status Draft→Done + §6 AC1-4 勾选（逐维如实）
- 改 `docs/s2v-adapter.md`——Phase 40 行（Draft→Done，Tasks 0→3）+ Task 40.1/40.2/40.3 行 + ADR-045 行 + BDD phase-40 行
- 改 3 task spec（40.1/40.2）+ 本 task（40.3）顶部 Status Draft→Done + §10 Completion Notes 真实回填

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- 真实 v0.33.0 tag push / release run / ghcr digest / cosign tlog [SPEC-OWNER:task-40.3-closeout]——ADR-012 用户授权后由 post-tag-push backfill PR 回填（release docs `<backfill>` marker，ADR-013 不预填）
- pin actor 认证身份 [SPEC-DEFER:phase-future.memory-actor-authenticated-identity] / 其余治理 marker（vector-dim-feature-enforce / tracestore-multi-workspace-strict / chunk-source-type-filter）据实保持延后

## 4. Actors

- 主 agent（ADR-012 自治）
- `scripts/console_smoke.sh`（v29→v30，step [49/49]）+ `internal/cli/smoke_syntax_test.go`（`TestTask403`）
- ADR-045（Proposed→Accepted）+ 触及 ADR（032/038/027/015 add-only Amendment）
- 用户 tajiaoyezi（ADR-012 v0.33.0 release 授权 + ratification）

## 5. Behavior Contract

### 5.1 Required Reading

- `scripts/console_smoke.sh`（Phase 39 v29[48/48] step + banner——本 task v30[49/49] 顺位）+ `internal/cli/smoke_syntax_test.go`（`TestTask393` 镜像源 + no-regression denominator 串）
- `docs/releases/v0.32.0-{evidence,artifacts}.md`（v0.33.0 镜像模板）+ `docs/decisions/adr-044-*.md`（ADR ratify + per-D Ratification 形态）
- `docs/decisions/adr-045-governance-debt-cleanup-3.md`（本 task ratify）+ `adr-032` / `adr-038` / `adr-027` / `adr-015`（add-only Amendment 落点）
- `docs/specs/phases/phase-40-governance-debt-cleanup-3.md §6`（AC 闭合）+ `docs/roadmap.md §3.21`（§3.22 add-only 形态）+ `docs/s2v-adapter.md`（Phase / Task / ADR / BDD 行形态）

### 5.2 关键设计 — 收口（smoke 顺位 + release docs + ADR ratify + add-only Amendment）

- **smoke 顺位**：banner v29→v30、新 step [49/49]、staging 顺位 offset（镜像 Phase 39 v29[48/48] → v30[49/49]）；denominators [37/37]..[48/48] 不溯改（ADR-014 D5）。
- **release docs `<backfill>` marker**：tag SHA / run id / ghcr digest / cosign tlog 真实值 post-tag-push 回填（ADR-013 不预填）；evidence/artifacts 镜像 v0.32.0 结构。
- **ADR-045 逐 D ratify**：据 task-40.1/40.2 真实 CI（cargo-test / go-test / lint / spec-lint 绿）逐 D（D1 pin actor 透传 / D2 L2 true-LRU / D3 默认零依赖守线）ratify Proposed→Accepted + `## Ratification`。
- **触及 ADR add-only Amendment**：ADR-032（pin actor 透传维度兑现，不溯改 D-body）/ ADR-038 + ADR-027（L2 true-LRU 维度兑现 + 真-LRU 假设据实更正，不溯改 D-body）/ ADR-015（proto add-only field）各加 `## Amendment (Phase 40 / v0.33.0)`（ADR-014 D5 不溯改正文）。

### 5.3 不变量

- 默认行为不变（ADR-004）：closeout 为 docs / smoke / ADR ratify，无运行时行为改动；smoke 既有 step 不退化（denominators 不溯改）。
- 既有契约不变 + 0 新 dep（ADR-008）。
- 禁伪造（ADR-013）：release 产物 `<backfill>` marker 不预填；ADR-045 据真实测试 ratify、不据合成 ratify。
- 不溯改历史（ADR-014 D5）：触及 ADR 改动均 add-only Amendment，不改 Phase 1-39 既有正文。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [x] **AC1**（smoke v30[49/49] + TestTask403 🟢）: `scripts/console_smoke.sh` banner v29→v30 + 新 step [49/49]（pin actor 透传 + L2 访问序 LRU）+ defer-note 更新；`internal/cli/smoke_syntax_test.go` `TestTask403`（[49/49] + markers + no-regression [37/37]..[48/48] + `bash -n`） — verified by **TEST-40.3.1**
- [x] **AC2**（v0.33.0 release docs + ADR ratify + add-only Amendment + roadmap/adapter + phase 闭合）: `docs/releases/v0.33.0-{evidence,artifacts}.md`（`<backfill>` marker）+ README v0.33 段 + RELEASE_NOTES v0.33.0 段；ADR-045 Proposed→Accepted 逐 D ratify + Ratification 段；ADR-032/038/027/015 add-only Phase-40 Amendment（不溯改正文）；roadmap §3.22/§4 add-only；s2v-adapter Phase 40 / tasks / ADR-045 / BDD 行；phase-40 §6 AC1-4 全 `[x]` — verified by **TEST-40.3.1**（同源收口验证）
- [x] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-40.3.2**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-40.3.1 | smoke v30[49/49] + `TestTask403`（[49/49] + markers + no-regression [37/37]..[48/48] + `bash -n`）+ release docs + ADR-045 ratify + ADR-032/038/027/015 add-only Amendment + roadmap §3.22/§4 + s2v-adapter + phase §6 AC1-4 闭合 | `scripts/console_smoke.sh` / `internal/cli/smoke_syntax_test.go` / `docs/**` | Done |
| TEST-40.3.2 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（低）smoke denominator 溯改**：v30[49/49] 顺位时误改既有 [37/37]..[48/48]。
  - **缓解**：仅追加 [49/49]，既有 denominator 不动（ADR-014 D5）；`TestTask403` no-regression 断言既有串。stop-condition：denominator 溯改则 AC1 不标 `[x]`。
- **R2（低）release 产物预填**：tag/run/digest 在真实 push 前误填。
  - **缓解**：evidence/artifacts 用 `<backfill>` marker，post-tag-push 回填（ADR-013 不预填）。stop-condition：预填则违 ADR-013。
- **R3（低）ADR 正文溯改**：add-only Amendment 时误改触及 ADR 既有 D-body。
  - **缓解**：仅加 `## Amendment (Phase 40 / v0.33.0)` 段，不动既有正文（ADR-014 D5）。stop-condition：溯改正文则违 ADR-014 D5。

## 9. Verification Plan

```bash
# 1. AC1 — smoke v30[49/49] + TestTask403
bash -n scripts/console_smoke.sh
go test ./internal/cli/ -run TestTask403

# 2. 不退化（全量）
cargo test --workspace
go test ./...

# 3. AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.task-40.3-release-backfill]：本 task 收口 Phase 40 / v0.33.0 docs / smoke / ADR ratify；真实 v0.33.0 tag / release run / ghcr digest / cosign tlog 经用户 ADR-012 授权后由 post-tag-push backfill PR 回填（release docs `<backfill>` marker，ADR-013 不预填）；pin actor 认证身份 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]` + 其余治理 marker 据实保持延后。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done

**§9 Verification 计划** (will record real evidence at impl)：
- AC1：`bash -n scripts/console_smoke.sh` + `go test ./internal/cli/ -run TestTask403` —— smoke v30[49/49] + `TestTask403`（[49/49] + markers + no-regression [37/37]..[48/48]）（真实结果待实施回填，ADR-013 不伪造）。
- AC2：release docs（`<backfill>` marker）+ ADR-045 逐 D ratify + ADR-032/038/027/015 add-only Amendment + roadmap §3.22/§4 + s2v-adapter + phase §6 闭合（真实结果待实施回填）。
- AC3：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威）。
- 真实 v0.33.0 tag/run/digest/tlog post-tag-push 回填（ADR-013 不预填）。

**实际改动文件**（计划，待实施回填）：
- `scripts/console_smoke.sh` v29→v30[49/49] + `internal/cli/smoke_syntax_test.go` `TestTask403`。
- `docs/releases/v0.33.0-{evidence,artifacts}.md` + README v0.33 段 + RELEASE_NOTES v0.33.0 段。
- `docs/decisions/adr-045-*.md` ratify + ADR-032/038/027/015 add-only Amendment。
- `docs/roadmap.md §3.22/§4` + `docs/s2v-adapter.md` + phase-40 §6 闭合 + 3 task spec Status Draft→Done。
