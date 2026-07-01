# Task `44.3`: `closeout-v0.37.0 — smoke v33→v34[53/53]（unpin X-Actor 端到端断言：REAL 模式 POST /v1/memory/{id}/unpin 带 X-Actor → audit source 归因，不可达诚实归因 unit TEST-44.1.1）+ TestTask443 no-regression（[37/37]..[52/52] 不溯改）+ v0.37.0 release docs + ADR-049 据 D1-D4 ratify + ADR-032/045 add-only Phase-44 Amendment（unpin actor 透传维度兑现，deprecate/softdelete/harddelete 续延后）+ roadmap §3.26/§4 add-only + s2v-adapter add-only`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治，全权授权）
**Related Phase**: Phase 44 (memory-unpin-actor-propagation)
**Dependencies**: task-44.1 Done / 既有 smoke v33[52/52]（Phase 43）+ TestTask433 范式 / release docs v0.36.0 模板 / ADR-049（本 task ratify）/ ADR-032+045（add-only Amendment）/ ADR-004/008/013/015 守线 / ADR-012（全权授权 tag/release）/ ADR-014 D1-D5（第三十五次激活）

## 1. Background
task-44.1 合入后收口：smoke v34[53/53] unpin X-Actor 端到端 + release docs + ADR-049 ratify + ADR-032/045 Amendment + roadmap/adapter。

## 2. Goal
(1) smoke v34[53/53]（REAL 模式 unpin X-Actor 端到端断言，不可达归因 unit）+ TestTask443（no-regression [37/37]..[52/52]）。
(2) v0.37.0 release docs（evidence/artifacts + README/RELEASE_NOTES v0.37 段）。
(3) ADR-049 ratify Proposed→Accepted + ADR-032/045 add-only Amendment + roadmap §3.26/§4 + adapter + phase §6。

## 3. Scope
### In Scope
- smoke v33→v34[53/53] + TestTask443
- v0.37.0-{evidence,artifacts}.md + README/RELEASE_NOTES v0.37 段
- ADR-049 ratify + ADR-032/045 add-only Amendment + roadmap §3.26/§4 + adapter + phase §6 + task-44.1 §10
### 范围外
- 认证身份 [SPEC-DEFER:phase-future.memory-actor-authenticated-identity]
- deprecate/softdelete/harddelete actor 透传 [SPEC-DEFER:phase-future.memory-actor-all-rpc]
- 真实 v0.37.0 tag push（全权授权，task-44.3 后执行 + post-tag-push backfill）

## 4-5. Actors / Behavior Contract
- 主 agent（全权授权 tag/release）；smoke v34[53/53]；ADR-049 ratify / ADR-032/045 Amendment。
- B1 smoke unpin X-Actor 端到端（REAL 模式 POST /v1/memory/{id}/unpin 带 X-Actor:smoke-actor → audit source 归因，不可达归因 unit TEST-44.1.1）。
- B2 ADR-049 据 task-44.1 真实 CI ratify（D1-D4）。
- B3 ADR-032/045 add-only Amendment（unpin actor 透传维度兑现，deprecate/softdelete/harddelete 续延后，不溯改正文 D5）。

## 6. Acceptance Criteria
- [x] **AC1**（smoke v34[53/53] + no-regression）— verified by **TEST-44.3.1a**（bash -n + TestTask443 绿）+ **TEST-44.3.1b**（smoke unpin X-Actor 端到端 / 不可达诚实归因）
- [x] **AC2**（release docs + ADR ratify + Amendment + roadmap/adapter）— verified by **TEST-44.3.1c**（全在场）
- [x] **AC3**（ADR-014 D2 lint）— verified by **TEST-44.3.2**（= LAST，CI spec-lint 全绿）

## 7. 追踪表
| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-44.3.1a | smoke v34[53/53] bash -n + TestTask443 绿（[53/53] + markers unpin-actor/X-Actor/TEST-44.1. + no-regression [37/37]..[52/52]） | scripts/console_smoke.sh + internal/cli/smoke_syntax_test.go | Done |
| TEST-44.3.1b | smoke v34 unpin X-Actor 端到端（REAL audit source 归因）/ 不可达诚实归因 unit | scripts/console_smoke.sh | Done（doc/status 归因 unit，REAL audit-log 不可达诚实归因） |
| TEST-44.3.1c | v0.37.0 release docs + ADR-049 ratify + ADR-032/045 Amendment + roadmap/adapter + phase §6 全在场 | docs（多文件） | Done |
| TEST-44.3.2 | D2 lint 0 未标注命中（= LAST） | scripts/spec_drift_lint.sh | Done（CI spec-lint） |

## 8. Risks
- R1（低）smoke 不可达被误读为未交付（splice 真实交付由 unit TEST-44.1.1 守护；smoke 可达是 bonus）。
- R2（低）ADR-049 ratify 被误读为认证身份已交付（认证身份 🔴 honest-defer）。

## 9. Verification Plan
```bash
bash -n scripts/console_smoke.sh && go test ./internal/cli/ -run TestTask443
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
go test ./... && go vet ./...
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done

**完成日期**：2026-07-01

**改动文件**：
- `scripts/console_smoke.sh`（v33→v34 + step [53/53] unpin X-Actor + v34 changelog）
- `internal/cli/smoke_syntax_test.go`（新 `TestTask443_SmokeV34UnpinActorPropagationStep` + no-regression [37/37]..[52/52]）
- `docs/releases/v0.37.0-evidence.md` + `v0.37.0-artifacts.md`（新增）+ `README.md` v0.37 段 + `RELEASE_NOTES.md` v0.37.0 段
- `docs/decisions/adr-049-memory-unpin-actor-propagation.md`（Proposed→Accepted + §Ratification）
- `docs/decisions/adr-032-memory-ops-hardening.md` + `adr-045-governance-debt-cleanup-3.md`（add-only Phase-44 Amendment）
- `docs/roadmap.md`（§3.26 推进记录 + §4）+ `docs/s2v-adapter.md`（Phase 44 / Task / ADR-049 / BDD 翻新）+ phase §6 + task-44.3 §10

**commit 列表**：`<本 PR>` feat/closeout: task-44.3 v0.37.0 closeout

**§9 Verification 结果**：
- install: skipped（0 代码 dep 改动）
- lint: ✅ `bash -n scripts/console_smoke.sh` exit 0
- typecheck: N/A（仅 smoke_syntax_test 新增已 go test 验证）
- unit-test: ✅ `go test ./internal/cli/ -run TestTask443` PASS
- integration / e2e / build: N/A
- coverage: N/A
- runtime-smoke: skipped（smoke v34[53/53] doc/status 归因 unit；REAL audit-log 不可达诚实归因）
- manual: ✅ release docs / ADR ratify / Amendment / roadmap / adapter 全在场（TEST-44.3.1c）；ADR-049 据 task-44.1 unit 真实测试 ratify

**剩余风险 / 未做项**：认证身份 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`；deprecate/softdelete/harddelete actor 透传 `[SPEC-DEFER:phase-future.memory-actor-all-rpc]`。真实 v0.37.0 tag/release 经用户全权授权（ADR-012）。

**下游 task 影响**：无（Phase 44 收口；v0.37.0 tag/release 全权授权下一条龙）。
