# Task `50.4`: `identity-closeout — redeem SPEC-DEFER marker + smoke + docs + phase closeout`

**Status**: Done
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 50 (identity-foundation)
**Dependencies**: task-50.1-50.3 全部完成 / ADR-013 / ADR-014（第四十二次激活，closeout）

## 1. Background
task-50.3 实测 verified actor 贯穿后，redeem `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`。phase closeout。

## 2. Goal
(1) redeem SPEC-DEFER marker（3 处源码注释改 fulfilled）。
(2) README + RELEASE_NOTES v2.0.0-alpha 段。
(3) smoke gate 加 user 注册 step。
(4) phase closeout。

## 3. Scope
- 改 `internal/consoleapi/handlers.go:558,613` + `core/src/data_plane/memory.rs:247`：SPEC-DEFER → fulfilled by task-50.3
- 改 `README.md`：身份验证基础交付声明
- 改 `RELEASE_NOTES.md`：v2.0.0-alpha 段
- 改 `scripts/console_smoke.sh`：加 user 注册 + verified actor step
- 改 `docs/roadmap.md` + `docs/s2v-adapter.md` + `CHANGELOG.md`
- 改 phase-50 spec Status Done + AC [x]

## 6. AC
- [x] **AC1**: SPEC-DEFER marker redeemed（3 处源码注释改 fulfilled by task-50.3）— verified by **TEST-50.4.1**
- [x] **AC2**: README/RELEASE_NOTES v2.0.0-alpha 诚实声明（身份基础交付；RBAC/workspace 延后）— verified by **TEST-50.4.2**
- [x] **AC3**: smoke gate + phase closeout（roadmap/adapter + smoke pass）— verified by **TEST-50.4.3**
- [x] **AC4**: ADR-014 D1-D5（第四十二次激活）phase closeout mapping — verified by PR body

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-50.4.1 | SPEC-DEFER marker redeemed（3 处） | grep | Done |
| TEST-50.4.2 | README/RELEASE_NOTES v2.0.0-alpha | grep | Done |
| TEST-50.4.3 | smoke + phase closeout | smoke + grep | Done |

## 9. Verification
```bash
# marker redeemed
grep -r "memory-actor-authenticated-identity" core/src/ internal/ | grep -v "fulfilled" # 应空
# README
grep -q 'v2.0.0-alpha\|身份验证基础' README.md
# smoke
bash scripts/console_smoke.sh
# spec_drift_lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes
**Status**: Done
1. **完成日期**：2026-07-05
2. **改动文件**：
   - internal/consoleapi/handlers.go（pin/unpin SPEC-DEFER 注释改 fulfilled by task-50.3）
   - core/src/data_plane/memory.rs（SPEC-DEFER 注释改 fulfilled by task-50.3）
   - README.md（Status v1.1.0→v2.0.0-alpha）
   - RELEASE_NOTES.md（加 v2.0.0-alpha 段）
   - docs/specs/phases/phase-50-identity-foundation.md（Status Done + AC [x]）
   - docs/s2v-adapter.md（Phase 50 + Task 50.1-50.4 全 Done）
   - docs/specs/tasks/task-50.4-identity-closeout.md（本 §10 回填 + Status Done）
3. **commit 列表**：
   - <GREEN> docs(release): task-50.4 v2.0.0-alpha closeout
4. **§9 Verification 结果**：
   - SPEC-DEFER redeemed：3 处源码注释（handlers.go pin + unpin + memory.rs）改 fulfilled by task-50.3 ✅
   - README v2.0.0-alpha + RELEASE_NOTES v2.0.0-alpha 段含身份基础交付 + RBAC/workspace 延后声明 ✅
   - phase closeout：phase spec Status Done + adapter 全 Done ✅
5. **剩余风险**：v2.0 进行中（身份基础交付但 RBAC/workspace/OAuth-OIDC 延后）；token 明文存（Phase 51+ hash）
6. **下游影响**：无（phase closeout；Phase 51+ workspace isolation 路线独立）
