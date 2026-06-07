# Task `41.3`: `closeout-v0.34.0 — smoke v30→v31[50/50]（production 默认 code_cjk + CONTEXTFORGE_TOKENIZER=default opt-out 端到端断言，TestTask413 镜像 TestTask403 无 [37/37]..[49/49] 回归，bash -n）+ v0.34.0 release docs（tag/run/digest <backfill> marker）+ ADR-046 据 D1-D4 ratify + ADR-029 add-only Phase-41 Amendment（标 默认开启维度 fulfilled）+ ADR-035 add-only Phase-41 Amendment（标 D3 产品决策 fulfilled）+ ADR-004/008 守线引用（刻意默认变更例外承接 + 0-dep）+ roadmap §3.23/§4 + adapter + defer marker 更新 + phase §6 闭合`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 41 (tokenizer-default-on)
**Dependencies**: task-41.1（tokenizer-default-on，Done）+ task-41.2（tokenizer-config-bridge，Done）/ 既有 `scripts/console_smoke.sh`（v30[49/49]，Phase 40 task-40.3 已在）+ `internal/cli/smoke_syntax_test.go`（`TestTask403` 镜像源）/ ADR-046（tokenizer-default-on，本 task ratify Proposed→Accepted）/ ADR-029（code-and-cjk-tokenizer，默认开启维度 add-only Amendment）/ ADR-035（cjk-true-segmenter-and-tokenizer-default，D3 产品决策 add-only Amendment）/ ADR-004（刻意默认变更例外由 ADR-046 承接 + opt-out byte-equiv safety intent）/ ADR-008（0 新 dep 守线）/ ADR-012（tag/release 须用户授权）/ ADR-014 D1-D5（第三十二次激活）/ ADR-013（禁伪造红线——真实测试 / 实测产物 ratify、tag/digest 不预填）

## 1. Background

task-41.1（production 默认翻 `code_cjk` + env opt-out）+ task-41.2（Go `[retrieval] tokenizer` config 桥）落地后，本 task 收口 v0.34.0：端到端 smoke + release docs + ADR-046 据真实测试 ratify + ADR-029/035 add-only Amendment + roadmap/adapter add-only + phase §6 闭合。

- **B1 smoke 顺位**：current `scripts/console_smoke.sh` v30[49/49]（Phase 40）→ Phase 41 v31[50/50]（banner v30→v31，staging 顺位 offset）。新 step REAL 模式断言翻默认端到端可观测：索引含 camelCase 符号 `getUserProfile` 的片段、search 子词 `profile` → 默认 `code_cjk` 命中（`code_cjk` 拆 camelCase 子词 → `profile` token；证翻默认生效）；`CONTEXTFORGE_TOKENIZER=default` opt-out → 新建 collection → search `profile` miss（`TEXT` 单 token `getuserprofile`；证 opt-out 回 legacy）；不可达则 doc/status。
- **B2 ADR ratify**：ADR-046 Proposed → Accepted（据 task-41.1/41.2 真实 CI 逐 D：D1 resolve_tokenizer + 生产绑定 / D2 env+config 桥 / D3 recall delta +0.0909 复测 + honest-defer / D4 刻意默认变更承接 + 0-dep）。
- **B3 add-only Amendment（不溯改正文 ADR-014 D5）**：ADR-029（默认开启维度兑现）+ ADR-035（D3 产品决策兑现）+ ADR-004（刻意默认变更例外承接守线引用）+ ADR-008（0-dep 守线引用）。
- **B4 release docs**：v0.34.0 evidence/artifacts（tag/run/digest `<backfill>`）+ README v0.34 段 + RELEASE_NOTES v0.34.0 段（Upgrade 段记 default tokenizer 翻 code_cjk + opt-out + 既有 collection 不受影响 + reindex 升级）。

## 2. Goal

(1) `scripts/console_smoke.sh` banner v30→v31 + v31 changelog block + 新 step [50/50]（production 默认 code_cjk + CONTEXTFORGE_TOKENIZER=default opt-out 端到端断言或 doc/status）。(2) `internal/cli/smoke_syntax_test.go` `TestTask413`（镜像 `TestTask403`）断言 [50/50] + markers（tokenizer-default-on / code_cjk / CONTEXTFORGE_TOKENIZER）+ no-regression（denominators [37/37]..[49/49] 不溯改）+ `bash -n`。(3) v0.34.0 release docs（evidence/artifacts/README/RELEASE_NOTES，tag/run/digest `<backfill>`）。(4) ADR-046 据 D1-D4 真实 ratify Proposed→Accepted + `## Ratification（v0.34.0 / task-41.3）`。(5) ADR-029/035 add-only Phase-41 Amendment（不溯改正文 D5）+ ADR-004/008 守线引用。(6) roadmap §3.23/§4 add-only（Phase 41 行 + 新 backlog 条目 cjk-segmenter-default-on / tokenizer-auto-reindex-on-upgrade）+ s2v-adapter add-only（Phase 41 行 Draft→Done + Tasks 0→3 + ADR-046 行 + BDD 行）。(7) phase §6 AC1-4 逐维勾选 + 3 task spec Status Draft→Done。

pass bar：smoke v31[50/50] `bash -n` 通过 + TestTask413 绿（无 [37/37]..[49/49] 回归）；ADR-046 据真实测试逐 D ratify（ADR-013 不据合成 / 伪造）；ADR-029/035 Amendment add-only（不溯改正文）；roadmap/adapter add-only；真实 v0.34.0 tag/run/digest/tlog 经用户授权 post-tag-push 回填（不预填）；ADR-014 D2 lint 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `scripts/console_smoke.sh`——banner v30→v31 + v31 changelog block + 新 step [50/50]（REAL：index `getUserProfile` 片段 → search `profile` 默认 `code_cjk` 命中 / `CONTEXTFORGE_TOKENIZER=default` opt-out → 新建 collection search `profile` miss；不可达 doc/status）；既有 step 不退化（denominators [37/37]..[49/49] 不溯改）
- 改 `internal/cli/smoke_syntax_test.go`——新 `TestTask413`（镜像 `TestTask403`）断言 "v31"/[50/50] + markers（tokenizer-default-on / code_cjk / CONTEXTFORGE_TOKENIZER）+ no-regression + `bash -n`
- 新增 `docs/releases/v0.34.0-evidence.md` + `v0.34.0-artifacts.md`（tag SHA / run id / digest = angle-bracket `<backfill>` marker，ADR-013 不预填）+ `README.md` v0.34 段 + `RELEASE_NOTES.md` v0.34.0 段（Upgrade：default tokenizer 翻 code_cjk + opt-out via `CONTEXTFORGE_TOKENIZER` / `[retrieval] tokenizer` + 既有 collection 不受影响 + reindex 升级；非 byte-equiv 据实记）
- 改 `docs/decisions/adr-046-tokenizer-default-on.md`——Status Proposed→Accepted（逐 D）+ `## Ratification（v0.34.0 / task-41.3）`
- add-only Amendment（不溯改正文，ADR-014 D5）：`adr-029-*.md`（`## Amendment (Phase 41 / v0.34.0)`：默认开启维度兑现）+ `adr-035-*.md`（D3 产品决策兑现）
- 改 `docs/roadmap.md`——§3.23 v0.34.0 推进记录 add-only + §4 新 backlog 条目（cjk-segmenter-default-on / tokenizer-auto-reindex-on-upgrade / tokenizer-large-corpus-recall / retriever-config-tokenizer-routing）add-only
- 改 `docs/specs/phases/phase-41-tokenizer-default-on.md`——Status Draft→Done + §6 AC1-4 勾选
- 改 `docs/s2v-adapter.md`——Phase 41 行 Draft→Done + Tasks 0→3 + Task 41.1/41.2/41.3 行 + ADR-046 行 + BDD 行
- 改 task-41.1 / task-41.2 spec Status Draft→Done（§10 真实证据回填）

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- 真实 v0.34.0 tag push（ADR-012 用户授权后；本 task 仅备 release docs `<backfill>` marker）[SPEC-OWNER:release-v0.34.0]
- jieba 默认开启 / 既有 collection 自动迁移 / 大语料 recall / RetrieverConfig.tokenizer 路由（roadmap §4 backlog，本 task 只记录延后）

## 4. Actors

- 主 agent（ADR-012 自治）
- `console_smoke.sh`（v31[50/50]，本 task 加 step）+ `smoke_syntax_test.go`（`TestTask413`）
- ADR-046（本 task ratify）+ ADR-029/035（本 task add-only Amendment）
- 用户（v0.34.0 tag/release 授权 ADR-012；release 凭据 post-tag-push 回填）

## 5. Behavior Contract

### 5.1 Required Reading

- `scripts/console_smoke.sh`（v30[49/49] Phase 40 → 本 task v31[50/50]；REAL vs SKIP 模式 + staging 顺位 offset）
- `internal/cli/smoke_syntax_test.go`（`TestTask403` 镜像源 + no-regression denominators [37/37]..[49/49]）
- `docs/releases/v0.33.0-{evidence,artifacts}.md`（v0.34.0 release docs 模板镜像 + `<backfill>` marker 约定）
- `docs/decisions/adr-046-tokenizer-default-on.md`（本 task ratify D1-D4）+ `adr-029-*.md §Negative/Follow-ups` + `adr-035-*.md §D3`（本 task add-only Amendment 落点）
- `docs/decisions/adr-014-*.md D5`（历史 Phase 1-40 不溯改——ADR 改动 add-only Amendment）

### 5.2 关键设计 — closeout 收口（真实 ratify / add-only / 不预填）

- **smoke v31[50/50]**：banner v30→v31 + 新 step（REAL：`getUserProfile` 片段 index → `profile` 子词查询默认 `code_cjk` 命中 / opt-out env → miss；不可达 doc/status）；既有 step denominators [37/37]..[49/49] 不溯改（ADR-014 D5）；`bash -n` 语法校验。
- **ADR-046 ratify**：据 task-41.1/41.2 真实 CI 逐 D（D1 resolve_tokenizer 矩阵 + 生产绑定 + 既有 collection 安全 / D2 config+env 桥 round-trip + env-wins / D3 recall delta +0.0909 复测 + honest-defer / D4 刻意默认变更承接 + 0-dep）Proposed→Accepted（ADR-013 不据合成 / 伪造 ratify）。
- **add-only Amendment（不溯改正文 D5）**：ADR-029 `## Amendment (Phase 41 / v0.34.0)`（默认开启维度兑现：生产默认翻 code_cjk + opt-out + 既有 collection 安全）+ ADR-035（D3 产品决策兑现：默认翻 code_cjk、jieba 仍 feature opt-in）；ADR-004（刻意默认变更例外由 ADR-046 承接 + opt-out byte-equiv safety intent 保持）/ ADR-008（0-dep）守线引用，不溯改正文。
- **release docs 不预填（ADR-013）**：tag SHA / run id / digest / tlog = angle-bracket `<backfill>` marker，真实 v0.34.0 tag/release 经用户授权 push 后 post-tag-push 回填。
- **roadmap/adapter add-only**：§3.23 v0.34.0 推进记录 + §4 新 backlog；adapter Phase 41 Draft→Done + Tasks 0→3 + Task/ADR/BDD 行。

### 5.3 不变量

- 既有 smoke step 不退化（denominators [37/37]..[49/49] 不溯改，ADR-014 D5）；`bash -n` 通过。
- ADR-046 据真实测试 ratify（ADR-013 不据合成 / 伪造）；ADR-029/035 Amendment add-only（不溯改正文，ADR-014 D5）。
- release 凭据不预填（tag/run/digest `<backfill>`，post-tag-push 回填，ADR-013）。
- 默认行为 / 既有契约 baseline：本 phase 是刻意默认变更（非 byte-equiv，由 ADR-046 承接）；release docs Upgrade 段据实记翻默认 + opt-out + 既有 collection 不受影响。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [ ] **AC1**（smoke v31[50/50] + TestTask413 🟢）: `scripts/console_smoke.sh` banner v30→v31 + 新 step [50/50]（production 默认 code_cjk + opt-out 端到端断言或 doc/status）+ 既有 step 不退化（denominators [37/37]..[49/49] 不溯改）+ `bash -n`；`internal/cli/smoke_syntax_test.go` `TestTask413`（镜像 `TestTask403`）断言 [50/50] + markers（tokenizer-default-on / code_cjk / CONTEXTFORGE_TOKENIZER）+ no-regression — verified by **TEST-41.3.1**
- [ ] **AC2**（v0.34.0 release docs + ADR ratify + Amendment + roadmap/adapter 🟢）: `docs/releases/v0.34.0-{evidence,artifacts}.md` + README v0.34 段 + RELEASE_NOTES v0.34.0 段（tag/run/digest `<backfill>`，Upgrade 记翻默认 + opt-out + 既有 collection 不受影响）；ADR-046 Proposed→Accepted（逐 D ratify）+ Ratification 段；ADR-029/035 add-only Phase-41 Amendment（不溯改正文 D5）+ ADR-004/008 守线引用；`docs/roadmap.md §3.23/§4` add-only；`docs/s2v-adapter.md` Phase 41 Draft→Done + Tasks 0→3 + Task/ADR/BDD 行；phase §6 AC1-4 勾选 + 3 task spec Status Draft→Done — verified by **TEST-41.3.1**（同 smoke step 收口校验）
- [ ] **AC3**（ADR-014 D2 lint + D5 不溯改）: `bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中 + 历史 Phase 1-40 ADR/spec 正文不溯改（仅 add-only Amendment） — verified by **TEST-41.3.2**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-41.3.1 | smoke v31[50/50]（`bash -n` + REAL/SKIP）+ `TestTask413`（镜像 `TestTask403`，[50/50] + markers + no-regression [37/37]..[49/49]）+ v0.34.0 release docs（`<backfill>`）+ ADR-046 ratify + ADR-029/035 add-only Amendment + roadmap §3.23/§4 + adapter Phase 41 Draft→Done + phase §6 闭合 | `scripts/console_smoke.sh` / `internal/cli/smoke_syntax_test.go` / `docs/**` | Planned |
| TEST-41.3.2 | D2 lint `--touched origin/master` 0 未标注命中 + D5 历史 Phase 1-40 不溯改（add-only Amendment）（= LAST） | `scripts/spec_drift_lint.sh` / `docs/decisions/**` | Planned |

## 8. Risks

- **R1（中）smoke 新 step 破既有 denominators / bash 语法**：新 step 改既有 [N/N] 或 `bash -n` 失败则 smoke 不可用。
  - **缓解**：denominators [37/37]..[49/49] 不溯改（仅加 [50/50]）；`bash -n` 校验；TestTask413 断言 no-regression。stop-condition：既有 denominators 改 / `bash -n` 失败则 AC1 不标 `[x]`。
- **R2（中）ADR ratify 据合成 / 预填**：若 ADR-046 据未跑出的结果 ratify、或 release 凭据预填则违 ADR-013。
  - **缓解**：ADR-046 据 task-41.1/41.2 真实 CI 逐 D ratify；release 凭据 `<backfill>` post-tag-push 回填。stop-condition：据合成 ratify / 预填 tag/digest 则 AC2 越界。
- **R3（中）Amendment 溯改正文违 D5**：ADR-029/035 Amendment 若改 D-body 而非 add-only 则违 ADR-014 D5。
  - **缓解**：ADR-029/035 仅 append `## Amendment (Phase 41 / v0.34.0)`（不改 D-body）；TEST-41.3.2 D5 check。stop-condition：溯改正文则 AC3 不标 `[x]`。
- **R4（低）release docs Upgrade 段误记 byte-equiv**：本 phase 非 byte-equiv（刻意默认变更），Upgrade 段若沿用「无强制迁移 / byte-equiv」模板措辞则不诚实。
  - **缓解**：Upgrade 段据实记「default tokenizer 翻 code_cjk（新建 collection 非 byte-equiv）+ opt-out via `CONTEXTFORGE_TOKENIZER=default` / `[retrieval] tokenizer` + 既有 collection 不受影响（保持持久化 analyzer）+ 升级既有 collection 经 reindex」。stop-condition：误记 byte-equiv / 漏 opt-out 则 AC2 越界（ADR-013）。

## 9. Verification Plan

```bash
# 1. AC1 — smoke v31[50/50] 语法 + TestTask413
bash -n scripts/console_smoke.sh
go test ./internal/cli/ -run TestTask413

# 2. 不退化（全量）
go test ./...
cargo test --workspace

# 3. AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.closeout-v0.34.0-defer-note]：本 task 收口 v0.34.0（smoke v31[50/50] + release docs + ADR-046 ratify + ADR-029/035 add-only Amendment + roadmap/adapter）。真实 v0.34.0 tag/run/digest/tlog 经用户授权 post-tag-push 回填（`<backfill>` marker，ADR-013 不预填）。jieba 默认开启 `[SPEC-DEFER:phase-future.cjk-segmenter-default-on]` / 既有 collection 自动迁移 `[SPEC-DEFER:phase-future.tokenizer-auto-reindex-on-upgrade]` / 大语料 recall `[SPEC-DEFER:phase-future.tokenizer-large-corpus-recall]` / RetrieverConfig.tokenizer 路由 `[SPEC-DEFER:phase-future.retriever-config-tokenizer-routing]` 续 backlog。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Draft

**§9 Verification 计划** (will record real evidence at impl)：
- AC1：`bash -n scripts/console_smoke.sh` + `go test ./internal/cli/ -run TestTask413` —— smoke v31[50/50] + TestTask413 no-regression（真实结果待实施回填，ADR-013 不伪造）。
- AC2：v0.34.0 release docs（`<backfill>`）+ ADR-046 ratify + ADR-029/035 add-only Amendment + roadmap/adapter add-only + phase §6 闭合。真实结果待实施回填。
- AC3：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中 + D5 历史不溯改。
- 真实 v0.34.0 tag/run/digest/tlog 经用户授权 post-tag-push 回填（ADR-013 不预填）。

**实际改动文件**（计划，待实施回填）：
- `scripts/console_smoke.sh`（v31[50/50]）+ `internal/cli/smoke_syntax_test.go`（`TestTask413`）。
- `docs/releases/v0.34.0-{evidence,artifacts}.md` + `README.md` + `RELEASE_NOTES.md`（v0.34 段）。
- `docs/decisions/adr-046-tokenizer-default-on.md`（ratify）+ `adr-029-*.md` + `adr-035-*.md`（add-only Amendment）。
- `docs/roadmap.md`（§3.23/§4）+ `docs/s2v-adapter.md`（Phase 41 行 + Task/ADR/BDD 行）+ `docs/specs/phases/phase-41-*.md`（§6 闭合）+ task-41.1/41.2 spec（Status Done）。
