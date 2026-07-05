# Task `50.4`: `identity-closeout — redeem SPEC-DEFER marker + smoke + docs + phase closeout`

**Status**: Ready
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
- [ ] **AC1**: SPEC-DEFER marker redeemed（3 处源码注释改 fulfilled by task-50.3）— verified by **TEST-50.4.1**
- [ ] **AC2**: README/RELEASE_NOTES v2.0.0-alpha 诚实声明（身份基础交付；RBAC/workspace 延后）— verified by **TEST-50.4.2**
- [ ] **AC3**: smoke gate + phase closeout（roadmap/adapter + smoke pass）— verified by **TEST-50.4.3**
- [ ] **AC4**: ADR-014 D1-D5（第四十二次激活）phase closeout mapping — verified by PR body

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-50.4.1 | SPEC-DEFER marker redeemed（3 处） | grep | Not Started |
| TEST-50.4.2 | README/RELEASE_NOTES v2.0.0-alpha | grep | Not Started |
| TEST-50.4.3 | smoke + phase closeout | smoke + grep | Not Started |

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
**Status**: Ready
1. **完成日期**：<TBD-after-impl>
2. **改动文件**：<TBD-after-impl>
3. **commit 列表**：<TBD-after-impl>
4. **§9 Verification 结果**：<TBD-after-impl>
5. **剩余风险**：<TBD-after-impl>
6. **下游影响**：无（phase closeout；Phase 51+ workspace isolation 路线独立）
