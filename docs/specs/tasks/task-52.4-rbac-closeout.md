# Task `52.4`: `rbac-closeout — redeem marker + workspace create auto-admin + docs`

**Status**: Ready
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 52 (rbac-roles-permissions)
**Dependencies**: task-52.1-52.3 全部完成 / ADR-013 / ADR-014（第四十四次激活）

## 1. Background
task-52.3 实测 admin-gate 贯穿后，workspace create 应自动给 owner admin membership。redeem SPEC-DEFER marker。phase closeout。

## 2. Goal
(1) workspace create（Go REST + Rust handler）：owner 自动 add_member(role=admin)。
(2) redeem `[SPEC-DEFER:phase-future.rbac-roles-permissions]`。
(3) README + RELEASE_NOTES v2.0.0-alpha.3 段。
(4) phase closeout。

## 3. Scope
- 改 workspace create path：owner 自动 add_member（Go REST handler 或 Rust WorkspaceStore create_owned 内联）
- redeem SPEC-DEFER marker（ADR-052 D4 + 相关源码注释改 fulfilled）
- 改 `README.md`：RBAC 交付声明
- 改 `RELEASE_NOTES.md`：v2.0.0-alpha.3 段
- 改 roadmap/adapter/CHANGELOG
- 改 phase-52 spec Status Done + AC [x]

## 6. AC
- [ ] **AC1**: workspace create auto-admin membership（owner 自动 add_member admin）— verified by **TEST-52.4.1**
- [ ] **AC2**: SPEC-DEFER redeemed — verified by **TEST-52.4.2**
- [ ] **AC3**: README/RELEASE_NOTES v2.0.0-alpha.3 + phase closeout — verified by **TEST-52.4.3**
- [ ] **AC4**: ADR-014 D1-D5（第四十四次激活）phase closeout mapping — verified by PR body

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-52.4.1 | workspace create auto-admin | go test / cargo test | Not Started |
| TEST-52.4.2 | SPEC-DEFER redeemed | grep | Not Started |
| TEST-52.4.3 | README/RELEASE_NOTES + closeout | grep | Not Started |

## 9. Verification
```bash
grep -r "rbac-roles-permissions" core/src/ docs/ | grep -v "fulfilled" # 应空
grep -q 'v2.0.0-alpha.3\|RBAC' README.md
```

## 10. Completion Notes
**Status**: Ready
1. **完成日期**：<TBD-after-impl>
2. **改动文件**：<TBD-after-impl>
3. **commit 列表**：<TBD-after-impl>
4. **§9 Verification 结果**：<TBD-after-impl>
5. **剩余风险**：<TBD-after-impl>
6. **下游影响**：无（phase closeout；Phase 53 workspace sharing/transfer + OAuth/OIDC 路线独立）
