# Task `42.3`: `closeout-v0.35.0 — smoke v31→v32[51/51]（REAL 模式 source_type 真实过滤端到端：索引 .rs + .md 混合 fixture，POST /v1/search?source_type=doc 仅返 .md、?source_type=code 仅返 .rs、空 filter 返全部）+ TestTask423 no-regression（[37/37]..[50/50] 不溯改）+ v0.35.0 release docs + ADR-047 据真实测试 ratify + ADR-037 add-only Amendment（source_type no-op supersede / agent_scope no-op 保持）+ roadmap §3.24/§4 add-only + s2v-adapter add-only`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 42 (chunk-source-type-filter)
**Dependencies**: task-42.1（chunk-source-type-derivation-and-filter）+ task-42.2（console-api-source-type-forward）全 Done / 既有 `scripts/console_smoke.sh`（v31[50/50]，Phase 41）+ `internal/cli/smoke_syntax_test.go`（`TestTask413` 范式）/ 既有 release docs 模板（`docs/releases/v0.34.0-{evidence,artifacts}.md`）/ ADR-047（chunk-source-type-filter，本 task ratify Proposed→Accepted）/ ADR-037（add-only Phase-42 Amendment）/ ADR-015 / ADR-024 / ADR-044 / ADR-004 / ADR-008 守线 / ADR-012（tag/release 用户授权）/ ADR-013 / ADR-014 D1-D5（第三十三次激活）

## 1. Background

task-42.1（retriever source_type 派生 + 真实过滤 + populate）+ task-42.2（console 请求侧 source_type forward）合入后，Phase 42 收口：smoke 加 source_type 真实过滤端到端断言、release docs、ADR-047 据真实测试 ratify、ADR-037 add-only Amendment、roadmap/adapter add-only。

- **B1 smoke 真实过滤断言（非保形）**：current smoke v31[50/50]（Phase 41）；本 task v32[51/51] REAL 模式索引 .rs + .md 混合 fixture，`POST /v1/search?source_type=doc` 仅返 .md chunk（`source_file_type="doc"`）、`?source_type=code` 仅返 .rs、空 filter 返全部——distinguishing 断言（证真实过滤生效，非 no-op）。
- **B2 ADR-047 ratify（据真实测试）**：task-42.1/42.2 真实 CI（cargo-test / go-test / lint / spec-lint 四门绿）+ 真实过滤行为后，ADR-047 D1-D4 逐项 Proposed→Accepted（ADR-013 禁据合成 ratify）。
- **B3 ADR-037 add-only Amendment**：task-32.3 把 source_type/agent_scope 定为 documented no-op；本 phase source_type no-op 被真实过滤 supersede（agent_scope no-op 据实保持）——add-only Amendment 记，不溯改 ADR-037 正文（ADR-014 D5）。

## 2. Goal

(1) **smoke v32[51/51]**：`scripts/console_smoke.sh` banner v31→v32 + v32 changelog block + 新 step [51/51]（REAL 模式 source_type 真实过滤端到端：.rs + .md 混合 fixture / `?source_type=doc` 仅返 doc / `?source_type=code` 仅返 code / 空 filter 返全部；不可达则 doc/status）；`internal/cli/smoke_syntax_test.go` 新 `TestTask423`（镜像 `TestTask413`）断言 [51/51] + markers + no-regression（denominators [37/37]..[50/50] 不溯改）。
(2) **v0.35.0 release docs**：`docs/releases/v0.35.0-{evidence,artifacts}.md`（tag SHA / run id / digest angle-bracket backfill marker）+ `README.md` v0.35 段 + `RELEASE_NOTES.md` v0.35.0 段（含「source_type 过滤落地 + `?source_type=` REST + 空 filter byte-equiv + agent_scope 续 memory 层 no-op」）。
(3) **ADR-047 ratify + ADR-037 Amendment + roadmap/adapter**：ADR-047 Status Proposed→Accepted（逐 D 据真实测试）+ `## Ratification（v0.35.0 / task-42.3）`；ADR-037 add-only `## Amendment (Phase 42 / v0.35.0)`（source_type no-op supersede / agent_scope no-op 保持）；`docs/roadmap.md §3.24/§4` add-only（Phase 42 行 + chunk-source-type-filter fulfilled + chunk-agent-scope-filter 续延后 + 新 backlog）；`docs/s2v-adapter.md` Phase 42 / Task / ADR-047 / BDD 行；phase §6 AC 勾选 + Status Done。

pass bar：smoke v32[51/51] `bash -n` 通过 + REAL source_type 真实过滤端到端断言（distinguishing）；`TestTask423` 断言 [51/51] + markers + no-regression（[37/37]..[50/50] 不溯改）；ADR-047 据真实 CI/实测 ratify（禁伪造）；ADR-037 add-only Amendment（不溯改正文）；roadmap/adapter add-only；ADR-014 D2 lint 0 未标注命中；ADR-012 tag/release 用户授权后回填真实产物。

## 3. Scope

### In Scope（计划交付）

- 改 `scripts/console_smoke.sh`——banner v31→v32 + v32 changelog block + 新 step [51/51]（REAL source_type 真实过滤端到端）
- 改 `internal/cli/smoke_syntax_test.go`——新 `TestTask423`（[51/51] + markers chunk-source-type-filter/source_type/classify/TEST-42.1./TEST-42.2. + no-regression [37/37]..[50/50] 不溯改 + `bash -n`）
- 新增 `docs/releases/v0.35.0-evidence.md` + `v0.35.0-artifacts.md`（镜像 v0.34.0，tag/run/digest backfill marker）
- 改 `README.md`（v0.35 段）+ `RELEASE_NOTES.md`（v0.35.0 段）
- 改 `docs/decisions/adr-047-chunk-source-type-filter.md`——Status Proposed→Accepted + `## Ratification（v0.35.0 / task-42.3）`
- 改 `docs/decisions/adr-037-*.md`——add-only `## Amendment (Phase 42 / v0.35.0)`（source_type no-op supersede / agent_scope no-op 保持，不溯改正文）
- 改 `docs/roadmap.md`——§3.24 推进记录 + §4 backlog（chunk-source-type-filter fulfilled / chunk-agent-scope-filter 续延后 + 新 backlog 条目）add-only
- 改 `docs/s2v-adapter.md`——Phase 42 行 Draft→Done + Tasks 0→3 + ADR-047 Proposed→Accepted + BDD 行
- 改 `docs/specs/phases/phase-42-chunk-source-type-filter.md`——Status Draft→Done + §6 AC 勾选
- 改 task-42.1/42.2 spec——Status Done + AC 勾选 + 追踪表 Done + §10 真实证据

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- chunk-level agent_scope 过滤 [SPEC-DEFER:phase-future.chunk-agent-scope-filter]
- importer 显式 source_type 打标 [SPEC-DEFER:phase-future.chunk-importer-source-type-tagging]
- semantic 路径 retriever-内过滤 [SPEC-DEFER:phase-future.semantic-path-source-type-filter]
- 真实 v0.35.0 tag push（ADR-012 须用户显式授权；本 task 出 release docs + backfill marker，授权后 post-tag-push 回填真实 tag SHA / run-id / digest / tlog）

## 4. Actors

- 主 agent（ADR-012 自治；tag/release 须用户授权）
- `scripts/console_smoke.sh`（v32[51/51] REAL source_type 真实过滤端到端）
- `internal/cli/smoke_syntax_test.go`（`TestTask423` 守护 smoke step + no-regression）
- ADR-047（本 task ratify Proposed→Accepted）/ ADR-037（本 task add-only Amendment）
- 用户（v0.35.0 tag/release 授权方，ADR-012）

## 5. Behavior Contract

### 5.1 Required Reading

- `scripts/console_smoke.sh`（v31[50/50] banner + changelog + step 范式，Phase 41）
- `internal/cli/smoke_syntax_test.go`（`TestTask413` 范式：denominator + markers + no-regression 断言）
- `docs/releases/v0.34.0-{evidence,artifacts}.md`（release docs 模板，backfill marker 约定）
- `docs/decisions/adr-046-tokenizer-default-on.md`（ADR ratify + Ratification section 范式）+ `adr-037-*.md`（待 add-only Amendment）+ `adr-047-chunk-source-type-filter.md`（待 ratify）
- `docs/decisions/adr-014-*.md`（D1-D5，第三十三次激活；D5 历史 Phase 1-41 不溯改）

### 5.2 关键设计 — closeout（真实过滤 smoke / 据真实 ratify / add-only Amendment）

- **B1 smoke distinguishing 断言**：REAL 模式（`MODE=real` && daemon succeeded）索引 .rs + .md 混合 fixture，`POST /v1/search?source_type=doc` 断言 `grep` 仅 .md chunk（`source_file_type="doc"`）、`?source_type=code` 断言仅 .rs；空 filter 返全部——distinguishing（source_type 过滤真实生效，非 no-op、非保形）；不可达 / non-real 模式 → echo doc/status（诚实归因到单测）。
- **B2 ADR-047 据真实 ratify（禁伪造）**：task-42.1/42.2 真实 CI 四门绿 + 真实过滤行为后，ADR-047 D1（classify_source_type 派生 + 0 migration）/ D2（真实过滤 + populate）/ D3（console forward）/ D4（agent_scope honest-defer）逐项 Proposed→Accepted；真实过滤行为 / 端到端实测数 §Ratification 据实记（ADR-013 禁据合成 ratify）。
- **B3 ADR-037 add-only Amendment（不溯改正文，ADR-014 D5）**：`## Amendment (Phase 42 / v0.35.0)` 记 task-32.3 source_type documented no-op 被本 phase 真实过滤 supersede（agent_scope no-op 据实保持——chunks 无 agent 维度）；不溯改 ADR-037 D-body。
- **B4 release docs backfill marker（ADR-013 不预填）**：v0.35.0 tag SHA / run-id / digest / tlog 为 angle-bracket backfill marker（`<backfill: ...>`），ADR-012 用户授权 tag push 后 post-tag-push 回填真实产物（不预填）。

### 5.3 不变量

- no-regression（ADR-014 D5）：smoke denominators [37/37]..[50/50] + 既有 markers 不溯改；新 step 顺位 [51/51]；`TestTask423` 断言历史 step 不退化。
- 据真实 ratify（ADR-013）：ADR-047 ratify 据 task-42.1/42.2 真实 CI + 实测过滤行为，不据合成 / 预填。
- add-only Amendment（ADR-014 D5）：ADR-037 经 add-only Amendment 记，不溯改正文；闭合 Phase 1-41 spec 不溯改。
- tag/release 用户授权（ADR-012）：v0.35.0 tag push 前停下等用户显式授权；release docs 出 backfill marker，授权后回填。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [ ] **AC1**（smoke v32[51/51] + TestTask423 🟢）: `scripts/console_smoke.sh` banner v31→v32 + 新 step [51/51]（REAL source_type 真实过滤端到端：.rs+.md fixture / `?source_type=doc` 仅 doc / `?source_type=code` 仅 code / 空 filter 全部，distinguishing）+ `internal/cli/smoke_syntax_test.go` `TestTask423`（[51/51] + markers + no-regression [37/37]..[50/50] 不溯改 + `bash -n`） — verified by **TEST-42.3.1**
- [ ] **AC2**（v0.35.0 release docs + ADR-047 ratify + ADR-037 Amendment + roadmap/adapter）: `docs/releases/v0.35.0-{evidence,artifacts}.md` + README/RELEASE_NOTES v0.35 段（backfill marker）+ ADR-047 Proposed→Accepted（逐 D 据真实测试）+ ADR-037 add-only Phase-42 Amendment（source_type no-op supersede / agent_scope no-op 保持，不溯改正文）+ roadmap §3.24/§4 add-only + s2v-adapter Phase 42/Task/ADR-047/BDD 行 + phase §6 AC 勾选 + Status Done — verified by **TEST-42.3.1**（docs 一致性）+ closeout PR review
- [ ] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-42.3.2**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-42.3.1 | smoke v32[51/51]（REAL source_type 真实过滤端到端 distinguishing）+ `TestTask423`（[51/51] + markers chunk-source-type-filter/source_type/classify/TEST-42.1./TEST-42.2. + no-regression [37/37]..[50/50] 不溯改 + `bash -n`）+ release docs / ADR-047 ratify / ADR-037 Amendment / roadmap/adapter add-only 一致 | `scripts/console_smoke.sh` + `internal/cli/smoke_syntax_test.go` + release/ADR/roadmap/adapter docs | Draft |
| TEST-42.3.2 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Draft |

## 8. Risks

- **R1（中）smoke 保形而非真实过滤断言**：若 step 仅断言响应 200 而不验证 source_type 过滤真实生效则空洞。
  - **缓解**：REAL 模式 distinguishing 断言（.rs+.md 混合 fixture，`?source_type=doc` grep 仅 .md / `?source_type=code` 仅 .rs），source_type 过滤真实生效才 PASS；不可达 / non-real → echo doc/status 诚实归因单测。stop-condition：step 保形 / 非 distinguishing 则 AC1 不标 `[x]`。
- **R2（中）ADR-047 据合成 / 预填 ratify**：ratify 据合成数据 / 预填违 ADR-013。
  - **缓解**：ADR-047 ratify 据 task-42.1/42.2 真实 CI 四门绿 + 真实过滤行为；真实数 §Ratification 据实记不预填。stop-condition：据合成 / 预填 ratify 则 AC2 不标 `[x]`。
- **R3（中）no-regression denominator 溯改**：新 step 误改既有 [37/37]..[50/50] denominator 破 ADR-014 D5。
  - **缓解**：新 step 顺位 [51/51]，既有 denominators 不动；`TestTask423` 断言历史 step 不退化。stop-condition：溯改既有 denominator 则 AC1 不标 `[x]`。
- **R4（中）backfill marker 误填 / 约定文字误伤**：release docs backfill marker 预填假值 / sed 误伤约定文字（反引号内 `<backfill>` 是约定文字、非待回填 marker）。
  - **缓解**：tag/run/digest 为 angle-bracket backfill marker（`<backfill: 具体descriptor>` 是待回填 marker / `<backfill: ...>` 省略号 + 反引号内约定文字排除）；ADR-012 授权 tag push 后 post-tag-push 回填真实产物。stop-condition：预填假产物则 AC2 不标 `[x]`（ADR-013）。

## 9. Verification Plan

```bash
# 1. AC1 — smoke v32[51/51] 语法 + TestTask423
bash -n scripts/console_smoke.sh
go test ./internal/cli/ -run TestTask423

# 2. 全量不退化
cargo test --workspace
go test ./...
cargo clippy --workspace --all-targets -- -D warnings
go vet ./...

# 3. AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master

# 4. （REAL smoke，本地 daemon 起得来时）source_type 真实过滤端到端
MODE=real bash scripts/console_smoke.sh   # step [51/51] source_type 过滤 distinguishing
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.closeout-v0.35.0-defer-note]：本 task 收口 Phase 42（smoke v32[51/51] source_type 真实过滤端到端 + v0.35.0 release docs + ADR-047 ratify + ADR-037 add-only Amendment + roadmap/adapter），🟢 可单测。chunk-level agent_scope 过滤 `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`、importer source_type 打标 `[SPEC-DEFER:phase-future.chunk-importer-source-type-tagging]`、semantic 路径 retriever-内过滤 `[SPEC-DEFER:phase-future.semantic-path-source-type-filter]` 续 backlog。真实 v0.35.0 tag/run/digest/tlog 经用户授权（ADR-012）后 post-tag-push 回填（release docs `<backfill>` marker，ADR-013 不预填）。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Draft（实施后置 Done + 回填真实 §9 证据 + 真实 v0.35.0 tag/run/digest/tlog）
