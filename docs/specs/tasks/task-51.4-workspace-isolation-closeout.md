# Task `51.4`: `workspace-isolation-closeout — redeem marker + SearchService thin gate + docs`

**Status**: Ready
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 51 (workspace-isolation)
**Dependencies**: task-51.1-51.3 全部完成 / ADR-013 / ADR-014（第四十三次激活）

## 1. Background
task-51.3 实测 verified owner 贯穿后，redeem `[SPEC-DEFER:phase-future.workspace-user-isolation]`。加 SearchService.Query thin gate。phase closeout。

## 2. Goal
(1) redeem SPEC-DEFER marker。
(2) SearchService.Query thin gate：verified user 传非 own/unowned workspace_id → 403。
(3) README + RELEASE_NOTES v2.0.0-alpha.2 段。
(4) phase closeout。

## 3. Scope
- redeem `[SPEC-DEFER:phase-future.workspace-user-isolation]`（proto 注释 + 源码注释改 fulfilled）
- 改 `core/src/data_plane/search.rs`：Query handler 加 ownership thin gate（verified user + 非 own workspace → 403）；需要从 gRPC 拿 verified userID（proto 加 caller_user_id 字段 or metadata——本 task 评估最小路径）
- 改 `README.md`：workspace ownership 交付声明
- 改 `RELEASE_NOTES.md`：v2.0.0-alpha.2 段
- 改 roadmap/adapter/CHANGELOG
- 改 phase-51 spec Status Done + AC [x]

## 6. AC
- [ ] **AC1**: SPEC-DEFER marker redeemed — verified by **TEST-51.4.1**
- [ ] **AC2**: SearchService.Query thin gate（verified user 非 own workspace → 403；trusted-network byte-equiv）— verified by **TEST-51.4.2**
- [ ] **AC3**: README/RELEASE_NOTES v2.0.0-alpha.2 + phase closeout — verified by **TEST-51.4.3**
- [ ] **AC4**: ADR-014 D1-D5（第四十三次激活）phase closeout mapping — verified by PR body

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-51.4.1 | SPEC-DEFER redeemed | grep | Not Started |
| TEST-51.4.2 | SearchService thin gate | cargo test | Not Started |
| TEST-51.4.3 | README/RELEASE_NOTES + closeout | grep | Not Started |

## 9. Verification
```bash
grep -r "workspace-user-isolation" core/src/ proto/ | grep -v "fulfilled" # 应空
cargo test -p contextforge-core -run search
grep -q 'v2.0.0-alpha.2\|workspace ownership' README.md
```

## 10. Completion Notes
**Status**: Ready
1. **完成日期**：<TBD-after-impl>
2. **改动文件**：<TBD-after-impl>
3. **commit 列表**：<TBD-after-impl>
4. **§9 Verification 结果**：<TBD-after-impl>
5. **剩余风险**：<TBD-after-impl>
6. **下游影响**：无（phase closeout；Phase 52 RBAC 路线独立）
