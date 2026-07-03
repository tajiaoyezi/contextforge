# Task `47.1`: `v1.0.0-release — README maturity label flip Pre-1.0→v1.0.0 + ADR-050 完整 ratify Proposed→Accepted + SPEC-DEFER known-limitations catalog + v1.0.0 closeout`

**Status**: Done
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 47 (v1.0.0-release)
**Dependencies**: Phase 45（D1/D2 已 ratify）+ Phase 46（D3/D4 已 ratify，D4 Release 对象首次实践成功）/ ADR-050（完整 ratify）/ ADR-007（分发定义）/ ADR-013（known limitations honest-defer）/ ADR-014（第三十八次激活）

## 1. Background
Phase 45/46 已交付 ADR-050 全 4 维度（D1 能力/D2 API-CLI 冻结/D3 文档对齐/D4 GitHub Release 流程），ADR-050 处于 "部分 ratify D1/D2/D3/D4" 状态。v1.0.0 是把 ADR-050 完整 ratify（Proposed→Accepted）+ maturity label flip + 列已知限制的终点。项目自承 Pre-1.0（README maturity label），v1.0.0 正式声明成熟度里程碑。

## 2. Goal
(1) README maturity label Pre-1.0 → **v1.0.0** + pin v0.39.0→v1.0.0。
(2) ADR-050 §Ratification Proposed → **Accepted**（D1-D4 全真实交付验证，逐 D 据 Phase 45/46 CI 验证）。
(3) v1.0.0 Release notes（RELEASE_NOTES.md v1.0.0 段 + CHANGELOG [v1.0.0]）列 active SPEC-DEFER 按 category 归类为 v1.0 known limitations（Retrieval quality / Memory / Observability / Release-CI / Interfaces / Platform，ADR-013 honest-defer）。
(4) smoke v36→v37[56/56] + TestTask471 + release docs + roadmap/adapter。

## 3. Scope
- 改 `README.md`：maturity label Pre-1.0 → **v1.0.0**（line 3 Status）+ pin v0.39.0→v1.0.0（Run the released image + Latest 段）
- 改 `docs/decisions/adr-050-v1.0-definition.md`：Status Proposed → **Accepted**；§Ratification 加完整 ratify 记录（D1-D4 全 ✅）
- 改 `RELEASE_NOTES.md`：加 v1.0.0 段（What shipped = v1.0 收口终点 + ADR-050 完整 ratify + **Known limitations** 6 category catalog + 凭据）
- 改 `CHANGELOG.md`：加 [v1.0.0] 段（major release — v1.0 maturity milestone + known limitations 指向）
- 新增 `docs/releases/v1.0.0-evidence.md` + `v1.0.0-artifacts.md`
- 改 `docs/roadmap.md`：§3.29 + §v1.0 锚点段（v1.0.0 完整 ratify Accepted）
- 改 `docs/s2v-adapter.md`：Phase 47 行 + ADR-050 Accepted + task 行
- 改 `scripts/console_smoke.sh`：v36→v37，step [55/55]→[56/56]
- 加 `internal/cli/smoke_syntax_test.go`：`TestTask471`（maturity label v1.0.0 + ADR-050 Accepted + known limitations + no-regression denominator）
- 新增 `test/features/phase-47-v1.0.0-release.feature`

## 6. AC
- [x] **AC1**（README maturity label flip）: README Pre-1.0 → **v1.0.0** + pin v0.39.0→v1.0.0 — verified by **TEST-47.1.1**
- [x] **AC2**（ADR-050 完整 ratify）: ADR-050 Proposed → **Accepted**（D1-D4 全 ✅） — verified by **TEST-47.1.2**
- [x] **AC3**（known-limitations catalog）: RELEASE_NOTES v1.0.0 段 + CHANGELOG [v1.0.0] 列 active SPEC-DEFER 6 category — verified by **TEST-47.1.3**
- [x] **AC4**（v1.0.0 closeout）: smoke v37[56/56] + release docs + ADR-050 完整 ratify + roadmap/adapter — verified by **TEST-47.1.4**
- [x] **AC5**（ADR-014 cross-validation gate）: D1-D5（第三十八次激活） — verified by PR body + LAST TEST

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-47.1.1 | README maturity label v1.0.0（无 Pre-1.0）+ pin v1.0.0 | docs grep | Done |
| TEST-47.1.2 | ADR-050 Status Accepted + D1-D4 全交付 | docs grep | Done |
| TEST-47.1.3 | RELEASE_NOTES v1.0.0 known limitations 6 category + CHANGELOG [v1.0.0] | docs grep | Done |
| TEST-47.1.4 | smoke v37[56/56] + TestTask471 PASS + release docs + ADR-050 Accepted | smoke + docs grep | Done |

## 9. Verification
```bash
# README maturity label v1.0.0（无 Pre-1.0）
grep -q "v1\.0\.0" README.md && ! grep -q "Pre-1\.0" README.md
# pin v1.0.0
grep -q "v1\.0\.0" README.md
# ADR-050 Accepted
grep -q "^\*\*Status\*\*: Accepted" docs/decisions/adr-050-v1.0-definition.md
# known limitations 6 category
grep -q "Known limitations" RELEASE_NOTES.md
# smoke
bash scripts/console_smoke.sh   # v37[56/56]
go test ./internal/cli/ -run TestTask471
```

## 10. Completion Notes
**Status**: Done

1. **完成日期**：2026-07-03
2. **改动文件**：
   - README.md（maturity label Pre-1.0→v1.0.0 + pin v0.39.0→v1.0.0）
   - docs/decisions/adr-050-v1.0-definition.md（Status Proposed→Accepted + 完整 ratify 段 D1-D4 全 ✅）
   - RELEASE_NOTES.md（v1.0.0 段 + Known limitations 6 category catalog）
   - CHANGELOG.md（[v1.0.0] 段 major release + known limitations）
   - docs/releases/v1.0.0-evidence.md + v1.0.0-artifacts.md（新增）
   - docs/roadmap.md（§3.29 推进记录 + §v1.0 锚点 v1.0.0 ratify）
   - docs/s2v-adapter.md（Phase 47 Done + ADR-050 Accepted + task 行）
   - scripts/console_smoke.sh（v36→v37[56/56]）
   - internal/cli/smoke_syntax_test.go（TestTask471：maturity label + ADR-050 Accepted + known limitations + no-regression）
3. **commit 列表**：- `c6e626c` feat(v1.0.0): task-47.1 v1.0.0-release — ADR-050 完整 ratify Accepted + maturity label flip + known limitations + smoke v37[56/56]
4. **§9 Verification 结果**：
   - lint: N/A（纯文档 + smoke，gofmt 不涉；CI lint 全绿）
   - typecheck: N/A
   - unit-test: go test ./internal/cli/ 全过（TestTask471 + TestTask463 no-regression PASS）
   - docs grep: ✅ README maturity label v1.0.0（无 Pre-1.0）+ pin v1.0.0 / ADR-050 Status Accepted + D1-D4 全 ✅ / RELEASE_NOTES Known limitations 6 category / smoke v37[56/56] + bash -n PASS
5. **剩余风险 / 未做项**：无（v1.0.0 是成熟度声明——D1-D4 全 CI 验证，known limitations 诚实列）
6. **下游 task 影响**：v2.0 路线（multi-user/认证/自动更新/arm64 native + large-corpus benchmarks + 其余 SPEC-DEFER backlog）
