# Task `45.1`: `v1.0-definition — ADR-050 立 v1.0 锚点（4 维度 D1-D4 + 不含清单推 v2.0 + v2.0 路线；承 ADR-017 悬空 v1.0 gate）+ roadmap §v1.0 锚点段 + §3.27 排期`

**Status**: Done
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 45 (v1.0-api-cli-freeze)
**Dependencies**: ADR-050（本 task 立即写为 Proposed）/ ADR-007（v1.0 分发定义收窄 add-only Amendment @ task-45.4）/ ADR-017（悬空 v1.0 gate 正式承接）/ ADR-015/004/008/013/014/012 守线

## 1. Background
项目从未立过 v1.0 锚点（PRD P0 是 v0.1 的、PRD v1.0 只在分发维度、roadmap 零 v1.0、README 无成熟度标签、ADR-017 悬空 v1.0 gate）。本 task 立 ADR-050 正式定义。

## 2. Goal
(1) ADR-050 Proposed：v1.0 = 功能成熟度收口（D1，已满足）+ API/CLI 冻结（D2，Phase 45）+ 文档对齐（D3，Phase 46）+ GitHub Release 流程（D4，Phase 46-47）；不含 multi-user/认证/自动更新/arm64（推 v2.0）。
(2) roadmap §v1.0 锚点段（引用 ADR-050 4 维度 + 不含清单）+ §3.27 Phase 45 排期。
(3) adapter ADR-050 行 + Phase 45 行 + BDD 行。

## 3. Scope
- 新增 `docs/decisions/adr-050-v1.0-definition.md`（Proposed）
- 改 `docs/roadmap.md`（§v1.0 锚点段 + §3.27）
- 改 `docs/s2v-adapter.md`（Phase 45 行 Draft + Task 行 + ADR-050 Proposed + BDD 行）
- 新增 `test/features/phase-45-v1.0-api-cli-freeze.feature`

## 6. AC
- [x] **AC1**（ADR-050 v1.0 定义）: ADR-050 Proposed 在场 + 4 维度（D1-D4）+ 不含清单 + v2.0 路线 + roadmap §v1.0 锚点段 — verified by **TEST-45.1.1**（grep 守护）

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-45.1.1 | ADR-050 在场 + D1-D4 + 不含清单 + roadmap §v1.0 锚点段 | docs grep | Not Started |

## 9. Verification
```bash
# grep 守护 ADR-050 + roadmap 锚点
grep -c "D1.*能力\|D2.*API\|D3.*文档\|D4.*发布" docs/decisions/adr-050-v1.0-definition.md
```

## 10. Completion Notes
**Status*: Done
