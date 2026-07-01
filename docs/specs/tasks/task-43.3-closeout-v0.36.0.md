# Task `43.3`: `closeout-v0.36.0 — smoke v32→v33[52/52]（indexing replay splice 可达则断言 since_ts>0 订阅者收到 indexing replay 事件序列、否则 doc/status 归因单测）+ TestTask433 no-regression（[37/37]..[51/51] 不溯改）+ v0.36.0 release docs + ADR-048 据真实测试 ratify + ADR-038 add-only Amendment（indexing-replay-e2e splice 维度兑现）+ roadmap §3.25/§4 add-only + s2v-adapter add-only`

**Status**: Ready

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 43 (governance-debt-cleanup-4)
**Dependencies**: task-43.1（indexing-replay-splice）Done / 既有 `scripts/console_smoke.sh`（v32[51/51]，Phase 42）+ `internal/cli/smoke_syntax_test.go`（`TestTask423` 范式）/ 既有 release docs 模板（`docs/releases/v0.35.0-{evidence,artifacts}.md`）/ ADR-048（indexing-replay-splice，本 task ratify Proposed→Accepted）/ ADR-038（add-only Phase-43 Amendment）/ ADR-031 / ADR-021 / ADR-004 / ADR-008 守线 / ADR-012（tag/release 用户授权）/ ADR-013 / ADR-014 D1-D5（第三十四次激活）

## 1. Background

task-43.1（indexing replay splice 接进 live subscribe）合入后，Phase 43 收口：smoke 加 indexing replay splice 可达断言、release docs、ADR-048 据真实测试 ratify、ADR-038 add-only Amendment、roadmap/adapter add-only。

- **B1 smoke splice 可达断言（诚实归因）**：current smoke v32[51/51]（Phase 42）；本 task v33[52/52] REAL 模式（`MODE=real` && daemon succeeded）断言 indexing replay splice 可达——`since_ts>0` 订阅者经 `SubscribeEvents` 收到 indexing replay 事件序列（`evt-idx-*`）；不可达 / non-real 模式 → echo doc/status（诚实归因到 unit 级 TEST-43.1.2，不伪造 smoke 数值）。live daemon restart-then-replay e2e 🟡 honest-defer（ADR-013，本 smoke 不跨 restart 双窗口）。
- **B2 ADR-048 ratify（据真实测试）**：task-43.1 真实 CI（cargo-test / go-test / lint / spec-lint 四门绿）+ splice 时序单测后，ADR-048 D1-D4 逐项 Proposed→Accepted（ADR-013 禁据合成 ratify；live daemon e2e 🟡 据已达 unit 级 splice ratify + 如实记录受阻）。
- **B3 ADR-038 add-only Amendment**：Phase 33 D3 标 `[SPEC-DEFER:phase-future.indexing-replay-e2e]`（mapper 🟢 已达 / e2e 🟡 未跑）；本 phase splice 维度兑现（mapper 接进 live subscribe + since_ts 时序）——add-only Amendment 记，不溯改 ADR-038 正文（ADR-014 D5）。live daemon e2e 续 `[SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]` honest-defer。

## 2. Goal

(1) **smoke v33[52/52]**：`scripts/console_smoke.sh` banner v32→v33 + v33 changelog block + 新 step [52/52]（indexing replay splice 可达则断言 `since_ts>0` 订阅者收到 indexing replay 事件序列、否则 doc/status 归因 unit TEST-43.1.2）；`internal/cli/smoke_syntax_test.go` 新 `TestTask433`（镜像 `TestTask423`）断言 [52/52] + markers + no-regression（denominators [37/37]..[51/51] 不溯改）。
(2) **v0.36.0 release docs**：`docs/releases/v0.36.0-{evidence,artifacts}.md`（tag SHA / run id / digest angle-bracket backfill marker）+ `README.md` v0.36 段 + `RELEASE_NOTES.md` v0.36.0 段（含「indexing replay splice 落地 + since_ts 时序对齐 audit + 默认 byte-equiv + live daemon e2e 🟡 honest-defer + memory-actor 据实延后」）。
(3) **ADR-048 ratify + ADR-038 Amendment + roadmap/adapter**：ADR-048 Status Proposed→Accepted（逐 D 据真实测试）+ `## Ratification（v0.36.0 / task-43.3）`；ADR-038 add-only `## Amendment (Phase 43 / v0.36.0)`（indexing-replay-e2e splice 维度兑现，live daemon e2e 续延后）；`docs/roadmap.md §3.25/§4` add-only（Phase 43 行 + indexing-replay-e2e splice fulfilled + indexing-replay-daemon-e2e 新 backlog + memory-actor-all-rpc 新 backlog）；`docs/s2v-adapter.md` Phase 43 / Task / ADR-048 / BDD 行；phase §6 AC 勾选 + Status Done。

pass bar：smoke v33[52/52] `bash -n` 通过 + indexing replay splice 可达断言（不可达诚实归因 unit）；`TestTask433` 断言 [52/52] + markers + no-regression（[37/37]..[51/51] 不溯改）；ADR-048 据真实 CI/实测 ratify（禁伪造，live daemon e2e 🟡 据实延后不强 ratify）；ADR-038 add-only Amendment（不溯改正文）；roadmap/adapter add-only；ADR-014 D2 lint 0 未标注命中；ADR-012 tag/release 用户授权后回填真实产物。

## 3. Scope

### In Scope（计划交付）

- 改 `scripts/console_smoke.sh`——banner v32→v33 + v33 changelog block + 新 step [52/52]（indexing replay splice 可达断言）
- 改 `internal/cli/smoke_syntax_test.go`——新 `TestTask433`（[52/52] + markers indexing-replay/splice/list_since/TEST-43.1. + no-regression [37/37]..[51/51] 不溯改 + `bash -n`）
- 新增 `docs/releases/v0.36.0-evidence.md` + `v0.36.0-artifacts.md`（镜像 v0.35.0，tag/run/digest backfill marker）
- 改 `README.md`（v0.36 段）+ `RELEASE_NOTES.md`（v0.36.0 段）
- 改 `docs/decisions/adr-048-indexing-replay-splice.md`——Status Proposed→Accepted + `## Ratification（v0.36.0 / task-43.3）`
- 改 `docs/decisions/adr-038-governance-debt-cleanup-2.md`——add-only `## Amendment (Phase 43 / v0.36.0)`（indexing-replay-e2e splice 维度兑现，live daemon e2e 续延后，不溯改正文）
- 改 `docs/roadmap.md`——§3.25 推进记录 + §4 backlog（indexing-replay-e2e splice fulfilled / indexing-replay-daemon-e2e 新 backlog / memory-actor-all-rpc 新 backlog）add-only
- 改 `docs/s2v-adapter.md`——Phase 43 行 Draft→Done + Tasks 0→2 + ADR-048 Proposed→Accepted + BDD 行
- 改 `docs/specs/phases/phase-43-governance-debt-cleanup-4.md`——Status Draft→Done + §6 AC 勾选
- 改 task-43.1 spec——Status Done + AC 勾选 + 追踪表 Done + §10 真实证据

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- live daemon restart-then-replay 端到端 e2e [SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]
- memory-actor-all-rpc 四 RPC（独立 phase）[SPEC-DEFER:phase-future.memory-actor-all-rpc]
- memory actor 认证身份 [SPEC-DEFER:phase-future.memory-actor-authenticated-identity]
- 真实 v0.36.0 tag push（ADR-012 须用户显式授权；本 task 出 release docs + backfill marker，授权后 post-tag-push 回填真实 tag SHA / run-id / digest / tlog）

## 4. Actors

- 主 agent（ADR-012 自治；tag/release 须用户授权）
- `scripts/console_smoke.sh`（v33[52/52] indexing replay splice 可达断言）
- `internal/cli/smoke_syntax_test.go`（`TestTask433` 守护 smoke step + no-regression）
- ADR-048（本 task ratify Proposed→Accepted）/ ADR-038（本 task add-only Amendment）
- 用户（v0.36.0 tag/release 授权方，ADR-012）

## 5. Behavior Contract

### 5.1 Required Reading

- `scripts/console_smoke.sh`（v32[51/51] banner + changelog + step 范式，Phase 42）
- `internal/cli/smoke_syntax_test.go`（`TestTask423` 范式：denominator + markers + no-regression 断言）
- `docs/releases/v0.35.0-{evidence,artifacts}.md`（release docs 模板，backfill marker 约定）
- `docs/decisions/adr-047-chunk-source-type-filter.md`（ADR ratify + Ratification section 范式）+ `adr-038-governance-debt-cleanup-2.md`（待 add-only Amendment）+ `adr-048-indexing-replay-splice.md`（待 ratify）
- `docs/decisions/adr-014-*.md`（D1-D5，第三十四次激活；D5 历史 Phase 1-42 不溯改）

### 5.2 关键设计 — closeout（splice 可达 smoke / 据真实 ratify / add-only Amendment）

- **B1 smoke 可达断言（诚实归因）**：REAL 模式（`MODE=real` && daemon succeeded）断言 indexing replay splice 可达——构造 DataPlaneStores 含 indexing_event_store + append indexing rows（ts > since_ts）+ `SubscribeEvents(since_ts=T)` 收到 `evt-idx-*` 事件序列；不可达 / non-real 模式 → echo doc/status（诚实归因到 unit 级 TEST-43.1.2，不伪造 smoke 数值，ADR-013）。live daemon restart-then-replay e2e（跨 restart 双窗口）🟡 honest-defer（本 smoke 不跨 restart）。
- **B2 ADR-048 据真实 ratify（禁伪造）**：task-43.1 真实 CI 四门绿 + splice 时序单测后，ADR-048 D1（list_since 时序过滤）/ D2（DataPlaneStores 接线）/ D3（subscribe splice）/ D4（默认 byte-equiv + honest-defer）逐项 Proposed→Accepted；splice 时序 / 默认 byte-equiv §Ratification 据实记（ADR-013 禁据合成 ratify；live daemon e2e 🟡 据已达 unit 级 splice ratify + 如实记录受阻，不强 ratify e2e）。
- **B3 ADR-038 add-only Amendment（不溯改正文，ADR-014 D5）**：`## Amendment (Phase 43 / v0.36.0)` 记 Phase 33 D3 `[SPEC-DEFER:phase-future.indexing-replay-e2e]` 的 splice 维度兑现（mapper 接进 live subscribe + since_ts 时序）；live daemon e2e 续 `[SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]` honest-defer；不溯改 ADR-038 D-body。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [ ] **AC1**（smoke v33[52/52] + no-regression）: `scripts/console_smoke.sh` v33[52/52] indexing replay splice 可达断言（不可达诚实归因 unit）+ `internal/cli/smoke_syntax_test.go` `TestTask433` markers 同步（no-regression [37/37]..[51/51] 不溯改） — verified by **TEST-43.3.1a**（`bash -n scripts/console_smoke.sh` + `TestTask433` 绿）+ **TEST-43.3.1b**（smoke v33[52/52] 可达断言 / 不可达 doc/status 诚实归因）
- [ ] **AC2**（v0.36.0 release docs + ADR-048 ratify + ADR-038 Amendment + roadmap/adapter）: `docs/releases/v0.36.0-{evidence,artifacts}.md` + README/RELEASE_NOTES v0.36 段 + ADR-048 Proposed→Accepted（逐 D 据真实测试）+ ADR-038 add-only Amendment + roadmap §3.25/§4 add-only + s2v-adapter add-only + phase §6 闭合 — verified by **TEST-43.3.1c**（release docs / ADR ratify / Amendment / roadmap / adapter 全在场）
- [ ] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-43.3.2**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-43.3.1a | smoke v33[52/52] `bash -n` 通过 + `TestTask433` 绿（[52/52] + markers indexing-replay/splice/list_since/TEST-43.1. + no-regression [37/37]..[51/51] 不溯改） | `scripts/console_smoke.sh` + `internal/cli/smoke_syntax_test.go` | Not Started |
| TEST-43.3.1b | smoke v33[52/52] indexing replay splice 可达断言（REAL 模式 since_ts>0 订阅者收到 evt-idx-* 事件序列）/ 不可达 doc/status 诚实归因 unit TEST-43.1.2（ADR-013 不伪造） | `scripts/console_smoke.sh` | Not Started |
| TEST-43.3.1c | v0.36.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ ADR-048 Proposed→Accepted Ratification + ADR-038 add-only Amendment + roadmap §3.25/§4 + s2v-adapter + phase §6 闭合 全在场 | docs（多文件） | Not Started |
| TEST-43.3.2 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Not Started |

## 8. Risks

- **R1（中）smoke 不可达被误读为 splice 未交付**：REAL 模式 daemon 起不来或 SubscribeEvents 不可达时，smoke step 不可达 → doc/status，易被误读为 splice 失败。
  - **缓解**：splice 真实交付由 unit TEST-43.1.2 守护（时序 + byte-equiv）；smoke 可达断言是 bonus 集成验证，不可达诚实归因 unit（ADR-013 不伪造 smoke 数值）；release docs / ADR ratify 据 unit 真实测试，不强依赖 smoke 可达。stop-condition：若把不可达 smoke 夸大为已验 splice 则越界。
- **R2（低）ADR-048 ratify 被误读为 live daemon e2e 已验**：本 task 据 unit 级 splice ratify，live daemon e2e 🟡 未跑。
  - **缓解**：ADR-048 §Ratification 据实记「D1-D3 unit 🟢 ratify / D4 live daemon e2e 🟡 honest-defer」；ADR-038 Amendment 标 splice 维度兑现 + live daemon e2e 续延后。stop-condition：若把 unit splice 夸大为 live e2e 已验则越界。
- **R3（低）no-regression 漂移**：denominators [37/37]..[51/51] 须不溯改（ADR-014 D5）。
  - **缓解**：`TestTask433` 断言 no-regression（仅加 [52/52]，不改既有 denominator）；`bash -n` 守 smoke 语法。stop-condition：若溯改既有 denominator 则违 D5。

## 9. Verification Plan

```bash
# 1. AC1 — smoke v33[52/52] + TestTask433
bash -n scripts/console_smoke.sh
go test ./internal/cli/ -run TestTask433

# 2. AC1 — smoke 可达（REAL 模式，可选）
MODE=real bash scripts/console_smoke.sh 2>&1 | grep -E "indexing.replay|52/52"

# 3. AC2 — release docs / ADR ratify / Amendment / roadmap / adapter 在场（file existence + grep）

# 4. 不退化（全量）
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
go test ./... && go vet ./...

# 5. AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]：本 task closeout 据 task-43.1 unit 级 splice + 时序单测 ratify ADR-048（D1-D3 🟢 / D4 live daemon e2e 🟡 honest-defer）；live daemon restart-then-replay e2e 不预填（ADR-013）。memory-actor-all-rpc 据实延后留独立 phase。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Ready（待实施回填）
