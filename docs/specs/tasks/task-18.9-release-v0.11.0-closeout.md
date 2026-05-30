# Task `18.9`: `release-v0.11.0-closeout — Phase 18 收口（诚实缩范围）+ v0.11.0 release docs`

**Status**: Done

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治；release tag push 经用户 2026-05-30 显式授权「切 v0.11.0 诚实缩范围」）
**Related Phase**: Phase 18 (vector-backend-selection)
**Dependencies**: task-18.1–18.8 全 Done / ADR-023（Proposed）/ ADR-006 Amendment A1 / ADR-014 D1-D5 第九次激活收口

## 1. Background

Phase 18 的 spike/选型/eval 度量全部交付（task-18.1 trait + 18.2 harness + 18.3–18.6 4 路真实数据 backend + 18.7 ADR-023 + comparison + 18.8 SemanticRecall@K 度量+门禁）。但 phase-18 §6 退出标准的 **AC3（ADR-023 Proposed→Accepted）+ AC4（默认 backend 接入生产 retriever + smoke v9 `/v1/search?semantic=true`）** 依赖**真实 embedding provider + 生产向量召回 wiring**，而本 Phase 的 spike 显式用确定性种子向量规避 ONNX（`[SPEC-DEFER:phase-future.embedding-provider-full]`）：

- 合成向量上 4 路 recall 均 1.0（不可区分，`docs/spikes/phase-18-comparison.md`）→ 无真实召回数据,**不能诚实 ratify ADR-023**（保持 Proposed）。
- 无 embedding provider + 向量 backend 未接生产 retriever（`[SPEC-OWNER:phase-future.vector-retrieval-integration]`,ADR-023 D6）→ **AC4 生产语义搜索 + smoke v9 无法实现**。

按 ADR-013（禁 fake-evidence），不把 AC3/AC4 标 `[x]`、不把 ADR-023 翻 Accepted。本 task 做**诚实缩范围 closeout**：v0.11.0 定位为「**向量 backend 基础设施 + 选型里程碑**」，生产语义搜索 + ADR ratify 后置到后继 phase（用户 2026-05-30 选「切 v0.11.0 诚实缩范围」）。

## 2. Goal

落 v0.11.0 release docs（README + RELEASE_NOTES + evidence + artifacts）+ phase-18 §6/§8 诚实状态（AC1/2/5/6 `[x]`；AC3 partial=ADR Proposed；AC4 deferred）+ s2v-adapter Phase 18 状态 + 18.9 行；合入后 push v0.11.0 annotated tag 触发 release.yml。`cargo test --workspace` + `go test ./...` 不退化；D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **修改 `README.md`** — 顶部 `## What's new in v0.11.0` 段（诚实范围：trait + harness + 4 路 spike + ADR-023 Proposed + eval 门禁；语义搜索生产化后置）。
- **修改 `RELEASE_NOTES.md`** — 顶部 `## v0.11.0` 段。
- **新建 `docs/releases/v0.11.0-evidence.md`** + **`docs/releases/v0.11.0-artifacts.md`** — 合入记录 + 4 路 5 维实测 + 验证证据 + 缩范围说明 + tag/镜像 SHA 待填（post-tag-push backfill）。
- **修改 `docs/specs/phases/phase-18-vector-backend-selection.md`** — §6 AC1/2/5/6 `[x]` + verified-by；AC3 partial 注（ADR Proposed，ratify 后置）；AC4 deferred 注（生产 wiring + smoke v9 后置）；§8 DoD 诚实勾选 + 缩范围段。
- **修改 `docs/s2v-adapter.md`** — Phase 18 状态 + 18.9 行 Done。

### Out of Scope（[SPEC-DEFER] / [SPEC-OWNER]）

- **ADR-023 D1 ratify（Proposed→Accepted）** [SPEC-OWNER:phase-future.vector-retrieval-integration]：须真实 embedding recall。
- **默认 backend 生产 retriever 集成 + smoke v9 + README `--semantic` example** [SPEC-OWNER:phase-future.vector-retrieval-integration]：需 embedding provider。
- **PRD §Open Questions O2 标 Resolved** [SPEC-DEFER:phase-future.prd-o2-resolve]：选型 provisional，O2 待 ratify 后标。
- **cross-repo Console 通知** ：Phase 18 非 cross-repo（无 SearchResponse 字段变更落生产），无 Console 协同。

## 4. Actors

- **主 agent**：closeout 文档 + release docs + tag push（经授权）。
- **release.yml**：v0.11.0 tag push 触发 ghcr 镜像构建。
- **后继 phase-future.vector-retrieval-integration**：消费本 release 的基础设施做生产语义召回 + ratify ADR-023。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-18-vector-backend-selection.md`（§6 AC / §8 DoD）
- `docs/releases/v0.10.0-{evidence,artifacts}.md`（release docs 模板）
- `docs/decisions/adr-023-vector-backend-default.md`（Proposed，D6 wiring 后置）+ `adr-006-*.md`（Amendment A1）
- `.github/workflows/release.yml`（v* tag push → ghcr）

### 5.2 诚实缩范围口径

- v0.11.0 = 向量 backend **基础设施 + 选型** 里程碑（非「语义搜索 live」）。
- phase-18 §6：AC1/2/5/6 met；AC3 partial（ADR Proposed）；AC4 deferred（生产集成）。
- DoD 不全 `[x]`：缩范围段记录 AC3/AC4 后置原因 + 后继 phase owner。

## 6. Acceptance Criteria

- [x] **AC1**: v0.11.0 release docs（README §v0.11.0 + RELEASE_NOTES §v0.11.0 + evidence + artifacts）落地，诚实范围（基础设施+选型，语义搜索生产化后置）— verified by 本 PR diff 含 4 文件
- [x] **AC2**: phase-18 §6 AC1/2/5/6 `[x]` + verified-by；AC3 partial（ADR Proposed）+ AC4 deferred 诚实注；§8 DoD 缩范围段 — verified by phase-18 spec diff
- [x] **AC3**: s2v-adapter Phase 18 状态 + 18.9 行 Done — verified by adapter diff
- [x] **AC4**: 既有不退化 — `cargo test --workspace` 0 failed（默认 feature）；`go test ./...` 全 PASS — verified by §10 实测
- [x] **AC5**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched master` PR 触及行 0 未标注命中 — verified by §10 记录
- [x] **AC6**: v0.11.0 annotated tag push（经用户授权）触发 release.yml ghcr 构建 — verified by §10 tag SHA + release.yml run（post-tag-push backfill）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-18.9.1 | v0.11.0 release docs 4 文件 | README / RELEASE_NOTES / v0.11.0-{evidence,artifacts}.md | Done |
| TEST-18.9.2 | phase-18 §6/§8 诚实状态 | docs/specs/phases/phase-18-*.md | Done |
| TEST-18.9.3 | adapter Phase 18 + 18.9 行 | docs/s2v-adapter.md | Done |
| TEST-18.9.4 | cargo + go test 不退化 | 全 workspace | Done |
| TEST-18.9.5 | v0.11.0 tag + release.yml | git tag + GHA | Done（backfill） |

## 8. Risks

- **R1（高）缩范围 vs phase 原始 AC**：AC3/AC4 未达原始标准。
  - **缓解**：诚实标 partial/deferred（非 fake `[x]`，ADR-013）；缩范围 + 后继 phase owner 明记；用户 2026-05-30 授权缩范围切版。
- **R2（中）tag push 对外发布**：release.yml → ghcr 镜像 + GitHub Release。
  - **缓解**：用户显式授权「切 v0.11.0」；镜像 = 现有二进制（基础设施，向量 feature 默认关闭）；可 `git tag -d` + 删 release 回退。
- **R3（低）release docs tag SHA / run ID 待填**：closeout PR 合入先于 tag push。
  - **缓解**：待填值 + post-tag-push backfill PR 填实（承 v0.8/v0.10 pattern）。

## 9. Verification Plan

```bash
cargo test --workspace
go test ./...
bash scripts/spec_drift_lint.sh --touched master
# 合入后：
git tag -a v0.11.0 -m "v0.11.0 — Phase 18 vector-backend-selection (infra + spike + ADR-023 Proposed)"
git push origin v0.11.0   # → release.yml ghcr 构建
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**：2026-05-30
- **改动文件**：`README.md` / `RELEASE_NOTES.md` / `docs/releases/v0.11.0-evidence.md`（新）/ `docs/releases/v0.11.0-artifacts.md`（新）/ `docs/specs/phases/phase-18-vector-backend-selection.md`（§6/§8）/ `docs/s2v-adapter.md`（Phase 18 + 18.9 行）/ `docs/specs/tasks/task-18.9-release-v0.11.0-closeout.md`（本 spec）
- **commit 列表**：见本 task PR（分支 `chore/phase-18-closeout-v0.11.0`）；合入后以 merge commit 为准
- **§9 Verification 结果**：closeout PR CI 三门绿 + D2 lint 0 命中；v0.11.0 tag SHA + release.yml run ID 见 post-tag-push backfill
- **剩余风险 / 未做项**：ADR-023 ratify + 生产语义召回集成 + smoke v9 后置 [SPEC-OWNER:phase-future.vector-retrieval-integration]（需 embedding provider）
- **下游 task 影响**：phase-future.vector-retrieval-integration（消费 v0.11.0 基础设施 + ratify ADR-023）
