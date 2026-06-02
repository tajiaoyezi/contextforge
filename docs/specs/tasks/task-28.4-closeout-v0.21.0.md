# Task `28.4`: `closeout-v0.21.0 — smoke v18 step 37 + v0.21.0 release docs + ADR-033 据 D1-D4 真实 ratify（D1 arm64 DEFERRED / D2 cosign 机制验证·真签@release / D3 lint 门绿）+ ADR-007 add-only Amendment + phase-28 §6 闭合`

**Status**: Done

**Priority**: P2
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 28 (release-ci-hardening)
**Dependencies**: task-28.1（匿名 pull + arm64 DEFERRED，#188）/ task-28.2（cosign 签名 + SBOM + provenance，#189）/ task-28.3（CI 强 lint，#190）全 Done / ADR-033（release-ci-hardening，本 task ratify）/ ADR-007（minimal-tarball-distribution，本 task add-only Amendment）/ ADR-004（镜像运行时 / 默认 baseline 不变）/ ADR-012（tag/release outward-facing 须用户显式授权）/ ADR-013（禁伪造凭据红线）/ ADR-014 D1-D5（第十九次激活）

## 1. Background

Phase 28 三个实现 task 全 Done：28.1（verify-image 匿名 pull 守护 + arm64 multi-arch 实测不可行延后）/ 28.2（cosign keyless sign + SBOM attest + provenance；GitHub 原生 attestation 私有仓库不可用→改 cosign）/ 28.3（ci.yml lint job clippy+gofmt+go vet 卡红 + 存量修到全绿）。本 task 收口 v0.21.0：smoke v18 + release docs + ADR-033 据真实结果 ratify + ADR-007 Amendment + phase §6 闭合 + adapter + feature。

## 2. Goal

据 28.1/28.2/28.3 **真实 CI / release run 产物**收口 v0.21.0：ADR-033 `Proposed → Accepted`（逐 D 项如实——D1 arm64 DEFERRED + anon-pull 达成、D2 cosign 机制验证·真签于已授权 release run、D3 lint 门绿、D4 baseline 不变）；ADR-007 add-only Amendment（部署发布面扩展，不溯改正文 D5）；phase-28 §6 AC1-5 置 `[x]` + Status Done；smoke v18 step 37（发布硬化状态，default build baseline intact）；release docs（evidence/artifacts/README/RELEASE_NOTES，tag/run/digest 用 backfill 待回填）；adapter（Phase 28 Done + Tasks 4 + ADR-033 Accepted + feature 行）。**真实 v0.21.0 tag/release 须用户显式授权**（不自行 tag，ADR-012）。

## 3. Scope

### In Scope（实际交付）

- `scripts/console_smoke.sh`——banner v17→v18 + v18 changelog 块 + step 37（`[37/37]`，发布硬化状态 + default build init baseline 不变；既有 step 不退化 + denominator 不溯改 ADR-014 D5）。
- `internal/cli/smoke_syntax_test.go`——`TestTask284_SmokeV18ReleaseCiHardeningStep`（断言 `[37/37]` + 标记 + 无回归既有 `[33/33]`..`[36/36]`）。
- 新增 `docs/releases/v0.21.0-{evidence,artifacts}.md`（tag/run/digest 用 `<backfill>` 待回填）+ `README.md` v0.21 段 + `RELEASE_NOTES.md` v0.21.0 段。
- `docs/decisions/adr-033-release-ci-hardening.md`——Status Proposed→Accepted（per-D 限定）+ `## Ratification` 节（逐 D 真实依据）+ `### ADR-007 add-only Amendment` 子节。
- `docs/decisions/adr-007-minimal-tarball-distribution.md`——append `## Amendment (Phase 28 / v0.21.0)`（不溯改正文）。
- `docs/specs/phases/phase-28-release-ci-hardening.md`——Status Draft→Done + §6 AC1-5 `[x]`（AC1 arm64 DEFERRED / AC2 cosign 真签@release 如实标注）。
- `docs/s2v-adapter.md`——§Phase 28 In Progress→Done + Tasks 3→4；§Task +28.4；§ADR 033 Proposed→Accepted；§BDD +phase-28 行。
- 新增 `test/features/phase-28-release-ci-hardening.feature`（≥4 scenario）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- 真实 v0.21.0 tag push + release run（cosign 真签 + GHCR 推送）[SPEC-OWNER:user-authorized-release]——outward-facing 不可逆须用户授权（ADR-012）；post-tag-push backfill 填实 tag SHA / run / digest。
- multi-arch arm64 原生 runner `[SPEC-DEFER:phase-future.multi-arch-native-runner]` / GitHub 原生 attestation `[SPEC-DEFER:phase-future.github-native-attestation]` / 签名密钥管理 `[SPEC-DEFER:phase-future.signing-key-management]` / lint 存量清零 `[SPEC-DEFER:phase-future.lint-backlog-cleanup]`。

## 4. Actors

- 主 agent（ADR-012 自治；真实 release 须用户授权）
- closeout 文档集（smoke / release docs / ADR / phase spec / adapter / feature）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-28-release-ci-hardening.md §6/§8`（AC + DoD）
- `docs/decisions/adr-033-release-ci-hardening.md`（§D1-D4 + Consequences Ratification 条款）
- task-28.1/28.2/28.3 §10（真实 run id + 结论）
- `docs/releases/v0.20.0-{evidence,artifacts}.md`（模板）

### 5.2 关键设计 — 诚实 per-D ratify + backfill 待回填

- ADR-033 ratify **逐 D 项据真实结果**：D1 部分（anon-pull run 26788773926 ✅ + arm64 run 26757640892 超时 DEFERRED）/ D2 Accepted（cosign 机制 run 26799480280 ✅；真实 GHCR 签名于授权 release run）/ D3 Accepted（lint 门 PR #190 四门绿）/ D4 Accepted（baseline 不变）。不为「全 Accepted」伪造 arm64 成功或 cosign 真签已发生（ADR-013）。
- tag SHA / release run id / 镜像 digest 在 release docs 用 `<backfill: ...>` 待回填——真实 v0.21.0 tag/release 是 closeout 合入后的**用户授权步**，post-tag-push backfill PR 填实（承 v0.8–v0.20 pattern）。
- smoke step 37 是文档/状态步（发布硬化无 console-api 运行时面）；只验 default build init baseline 不变（ADR-004）+ 文档化三 task 状态。

### 5.3 不变量

- 0 行为变更 / 0 新依赖（closeout 纯文档 + smoke step；smoke 既有 step + denominator 不溯改 D5）。
- ADR-014 D5：历史 Phase 1-27 spec 不溯改；ADR-007 add-only Amendment 不改正文。
- 真实 tag/release 不自行触发（ADR-012）。

## 6. Acceptance Criteria

- [x] **AC1**: smoke v18 step 37（`[37/37]` 发布硬化状态 + default build baseline intact）+ `TestTask284` 断言（含无回归既有 `[33/33]`..`[36/36]`，denominator 不溯改）— verified by **TEST-28.4.1** + §10 实测（`bash -n` + go test）
- [x] **AC2**: v0.21.0 release docs（evidence/artifacts `<backfill>` 待回填 + README + RELEASE_NOTES）+ ADR-033 据 D1-D4 真实 ratify Accepted（逐维如实）+ ADR-007 add-only Amendment + phase-28 §6 AC1-5 `[x]` + Status Done + adapter 闭合 + feature — verified by **TEST-28.4.2** + §10
- [x] **AC3**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-28.4.3** + §10（CI spec-lint 权威）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-28.4.1 | smoke v18 step 37（`[37/37]` + 发布硬化标记 + 无回归既有 denominator）+ `bash -n` 过 + go test TestTask284 过 | `scripts/console_smoke.sh` + `internal/cli/smoke_syntax_test.go` | Done |
| TEST-28.4.2 | release docs + ADR-033 ratify Accepted（per-D 如实）+ ADR-007 Amendment + phase-28 §6 闭合 + adapter + feature | release/ADR/phase/adapter/feature | Done |
| TEST-28.4.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威） | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（低）closeout 误报 arm64 / cosign 真签为已达成**：诚实风险。
  - **缓解**：ADR-033 ratify + release docs + smoke + phase §6 全逐维如实——arm64 DEFERRED、cosign 真签于授权 release run（机制已验证）；不伪造（ADR-013）。stop-condition：任何「multi-arch 成功」/「真实 GHCR 已签名」表述须有真实 run 凭据，否则标 DEFERRED / backfill。
- **R2（低）smoke denominator 误溯改**：新 step 37 须 `[37/37]`，既有 `[33/33]`..`[36/36]` 不动。
  - **缓解**：`TestTask284` 无回归断言守护；ADR-014 D5。

## 9. Verification Plan

```bash
# AC1 — smoke 语法 + syntax test
bash -n scripts/console_smoke.sh
go test ./internal/cli/ -run TestTask284

# AC2 — 文档闭合人工核（ADR-033 Accepted + per-D / phase §6 [x] / adapter Done / feature 存在）
# AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master

# 既有不退化（closeout 文档+smoke 不影响 workspace）
cargo test --workspace && go test ./...
```

> **outward-facing 红线**：真实 v0.21.0 tag push + release run（cosign 真签 + 多 GHCR 推送）是 closeout 合入后的**用户授权步**（ADR-012）；本 task 不自行 tag，release docs 的 tag/run/digest 用 `<backfill>` 待回填待 post-tag-push backfill 填实。

## 10. Completion Notes (s2v 6 项标准)

- **Status**: Done（2026-06-02）。
- **完成日期**：2026-06-02。
- **改动文件**：`scripts/console_smoke.sh`（v18 step 37）+ `internal/cli/smoke_syntax_test.go`（TestTask284）+ `docs/releases/v0.21.0-{evidence,artifacts}.md`（新，backfill 待回填）+ `README.md` v0.21 段 + `RELEASE_NOTES.md` v0.21.0 段 + `docs/decisions/adr-033-release-ci-hardening.md`（Accepted + Ratification + ADR-007 Amendment 子节）+ `docs/decisions/adr-007-minimal-tarball-distribution.md`（§Amendment）+ `docs/specs/phases/phase-28-release-ci-hardening.md`（Done + §6 [x]）+ `docs/s2v-adapter.md`（Phase/Task/ADR/BDD）+ `test/features/phase-28-release-ci-hardening.feature`（新）。
- **commit 列表**：`docs(spec): task-28.4 v0.21.0 closeout`（合于一 PR）。
- **§9 Verification 实测结果（ADR-013 真实非合成）**：`bash -n scripts/console_smoke.sh` 过；`go test ./internal/cli/ -run TestTask284` 过；`cargo test --workspace` + `go test ./...` 不退化；D2 lint CI spec-lint 权威。ADR-033 ratify 逐 D 据真实 run（28.1 anon-pull 26788773926 / arm64 26757640892 超时；28.2 cosign 机制 26799480280 / GitHub 原生失败 26789731232；28.3 lint PR #190 26820737785 四门绿）。
- **设计取舍**：(1) **诚实 per-D ratify**——D1 arm64 DEFERRED + anon-pull 达成、D2 cosign 机制验证·真签于授权 release、D3 lint 门绿、D4 baseline 不变，不伪造（ADR-013）。(2) **backfill 待回填**——真实 tag/release 是用户授权步，release docs tag/run/digest 待 post-tag-push backfill。(3) **smoke step 37 文档/状态步**——发布硬化无运行时面，验 default build baseline intact。
- **剩余风险 + 下游影响**：真实 v0.21.0 tag/release（cosign 真签 + GHCR 推送）待用户授权 → post-tag-push backfill 填实 evidence/artifacts 待回填；arm64 multi-arch / GitHub 原生 attestation / 签名密钥管理 / lint 存量清零 / rustfmt / golangci-lint 等 `[SPEC-DEFER:phase-future.*]` 留 backlog。**Phase 28 完结**（4 task 全 Done，v0.21.0 待授权发布）。
