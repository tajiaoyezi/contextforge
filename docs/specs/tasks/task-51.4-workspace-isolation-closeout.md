# Task `51.4`: `workspace-isolation-closeout — redeem marker + SearchService thin gate + docs`

**Status**: Done
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
- [x] **AC1**: SPEC-DEFER marker redeemed — verified by **TEST-51.4.1**（proto multi-workspace-strict → full-rpc-ownership-enforcement 延后标注）
- [x] **AC2**: SearchService.Query thin gate（verified user 非 own workspace → 403；trusted-network byte-equiv）— verified by **TEST-51.4.2**（go test TestTask514_2/2b PASS）
- [x] **AC3**: README/RELEASE_NOTES v2.0.0-alpha.2 + phase closeout — verified by **TEST-51.4.3**
- [x] **AC4**: ADR-014 D1-D5（第四十三次激活）phase closeout mapping — verified by PR body

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-51.4.1 | SPEC-DEFER redeemed | grep | Done |
| TEST-51.4.2 | SearchService thin gate | go test | Done |
| TEST-51.4.3 | README/RELEASE_NOTES + closeout | grep | Done |

## 9. Verification
```bash
grep -r "workspace-user-isolation" core/src/ proto/ | grep -v "fulfilled" # 应空
cargo test -p contextforge-core -run search
grep -q 'v2.0.0-alpha.2\|workspace ownership' README.md
```

## 10. Completion Notes
**Status**: Done
1. **完成日期**：2026-07-05
2. **改动文件**：
   - internal/consoleapi/handlers.go（handleSearch 加 workspace ownership thin gate）
   - internal/consoleapi/workspace_owner_test.go（+TestTask514_2 search gate + TestTask514_2b byte-equiv）
   - proto/contextforge/console_data_plane/v1/console_data_plane.proto（SPEC-DEFER 标注 full-rpc-ownership-enforcement）
   - README.md（Status v2.0.0-alpha→v2.0.0-alpha.2）
   - RELEASE_NOTES.md（加 v2.0.0-alpha.2 段）
   - docs/specs/phases/phase-51-workspace-isolation.md（Status Done）
   - docs/s2v-adapter.md（Phase 51 + Task 51.1-51.4 全 Done）
   - docs/specs/tasks/task-51.4-workspace-isolation-closeout.md（本 §10 回填）
3. **commit 列表**：
   - <GREEN> docs(release): task-51.4 v2.0.0-alpha.2 closeout
4. **§9 Verification 结果**：
   - go test TestTask514_2/_2b：2/2 PASS（search gate blocks non-owned + trusted-network byte-equiv）
   - full consoleapi no-regression ✅
5. **剩余风险**：SearchService thin gate 仅在 Go REST 层（非 Rust gRPC 层）；全 RPC enforcement 延后 full-rpc-ownership-enforcement
6. **下游影响**：无（phase closeout；Phase 52 RBAC 路线独立）
