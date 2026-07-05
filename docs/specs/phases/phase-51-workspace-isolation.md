# Phase 51 · workspace-isolation (B1 第二步：per-user workspace ownership)

**Status**: Ready

> Phase Spec（s2v full-standard §8.2）。本 phase 是 **v2.0 multi-user 第二步**——给 workspace 加 owner 列 + per-user access control。关闭 `[SPEC-DEFER:phase-future.workspace-user-isolation]`（proto 内又称 `phase-15.multi-workspace-strict`）。
>
> Phase 50 交付了 verified identity（actor 从 declared 变 verified），但**数据访问无边界**——任何调用方能读/搜任意 workspace。Phase 51 让 workspace 有了 owner 概念，verified user 只能访问自己 own 的 + unowned 的。
>
> **方向锚点**（承 Phase 50 ADR-051 D4）：option (a) add owner column + filter list/get by verified user——最小增量，为 Phase 52 RBAC 打地基。
>
> **入读顺序**：本 phase spec → 4 task spec → 源码锚点（`core/migrations/0010_workspaces.sql` + `core/src/workspace/mod.rs` WorkspaceStore trait + `core/src/memory/store.rs:351` ensure_pin_actor_columns PRAGMA guard pattern + `internal/consoleapi/router.go:88` verifiedUserIDKey + `internal/consoleapi/handlers.go:70-157` workspace handlers）→ ADR-015（D2 workspace_id=collection_id 1:1）/ ADR-051（D4 deferral）/ ADR-052（本 phase 新增）。

## 1. 阶段目标

让 workspace 有了 owner 概念，verified user 的数据访问有了边界。具体：

1. **owner_id 列**：migration 0021（guarded ALTER TABLE，同 0017 pattern）加 `owner_id TEXT` 到 workspaces 表
2. **WorkspaceStore owner 支持**：create_owned / list_owned(userID) / get_if_owned(id, userID)
3. **Go REST 用 verified identity**：POST/GET /v1/workspaces 从 context 读 verified userID
4. **SearchService thin gate**：verified user 传非 own 的 workspace_id → 403（最小 enforcement）

**具体 exit criteria（§6 AC）**：
1. task-51.1: ADR-052 + migration 0021 + WorkspaceStore owner 支持
2. task-51.2: proto add-only owner 字段 + Rust handler
3. task-51.3: Go REST workspace handler 用 verified identity
4. task-51.4: redeem SPEC-DEFER + SearchService thin gate + closeout
5. ADR-014 D1-D5（第四十三次激活）

**版本号**：v2.0.0-alpha.2（task-51.4 closeout 定）。

## 2. 业务价值

**地基价值**：Phase 50 让身份可验证，但数据访问无边界。Phase 51 让 verified user 的数据访问有了边界——只能访问自己 own 的 + unowned 的 workspace。这是 multi-user 部署的必要前提（否则 verified identity 无意义——任何人能看到所有人的数据）。

### 51.1 ADR-052 + migration + WorkspaceStore（🟢 Rust）
migration 0021 加 owner_id 列 + WorkspaceStore create_owned/list_owned/get_if_owned。

### 51.2 proto + Rust gRPC（🟢 proto add-only + Rust）
proto Workspace message 加 owner_id；WorkspaceService handler 用 owner_id。

### 51.3 Go REST + verified identity（🟢 Go）
POST/GET /v1/workspaces 从 context 读 verified userID 传给 grpcclient。

### 51.4 closeout + thin gate（🟢 Rust gate + 文档）
SearchService.Query thin gate（verified user 非 own workspace → 403）+ redeem marker + docs。

**不在本 phase 范围**（诚实 OOS，均已登记 SPEC-DEFER）：RBAC/roles/permissions（Phase 52+ `[SPEC-DEFER:phase-future.rbac-roles-permissions]`）/ workspace_members 多 user 共享（Phase 52）/ 全 RPC strict enforcement（GetSourceChunk/ListQueries/memory/eval ownership 留 Phase 51.x `[SPEC-DEFER:phase-future.full-rpc-ownership-enforcement]`——这些端点 today 有 empty=aggregate-all 语义需单独 backward-compat 决策）/ token hash 存（Phase 51+ `[SPEC-DEFER:phase-future.token-hash-storage]`）/ OAuth/OIDC（Phase 53+）。

## 3. 涉及模块
- **51.1**: `core/migrations/0021_workspaces_owner.sql`（新增）+ `core/src/workspace/mod.rs`（+owner 支持）+ `docs/decisions/adr-052-workspace-ownership.md`（新增）
- **51.2**: `proto/contextforge/console_data_plane/v1/console_data_plane.proto`（add-only owner 字段）+ `core/src/data_plane/workspace.rs`（handler）+ buf generate
- **51.3**: `internal/consoleapi/handlers.go`（workspace handlers 用 verified identity）+ `grpcclient.go`（+owner 参数）
- **51.4**: `core/src/data_plane/search.rs`（thin gate）+ 源码注释 redeem + `README.md` + `RELEASE_NOTES.md` + roadmap/adapter

## 5. Behavior Contract
- migration 0021 guarded ALTER（PRAGMA table_info 检查 owner_id 是否存在，同 0017）
- WorkspaceStore：create_owned(id, owner) / list_owned(userID) 返回 owner_id=user 或 NULL / get_if_owned(id, userID) 返 Option
- owner 从 context（verifiedUserIDKey）取，不进 WorkspaceCreate body
- trusted-network/shared-token → owner_id = NULL（workspace unowned）+ all visible（byte-equiv）
- NULL owner = unowned → 任何 verified user 可见（transitional）
- SearchService.Query thin gate：verified user + 非 own/unowned workspace_id → 403

## 6. AC（Phase 级）
- [ ] AC1: ADR-052 + migration 0021 + WorkspaceStore owner 单测 — verified by task-51.1 §6
- [ ] AC2: proto add-only owner 字段 + Rust gRPC 单测 — verified by task-51.2 §6
- [ ] AC3: Go REST workspace handler 用 verified identity + byte-equivalent — verified by task-51.3 §6
- [ ] AC4: SPEC-DEFER redeemed + SearchService thin gate + closeout — verified by task-51.4 §6
- [ ] AC5: ADR-014 D1-D5（第四十三次激活）全通过

## 8. Risks
- **ALTER TABLE 幂等**：guarded by PRAGMA table_info（同 0017 pattern，已验证）
- **现有数据 backfill**：NULL owner = unowned（transitional）；不破坏现有部署
- **SearchService thin gate 范围**：仅 Query；全 RPC enforcement `[SPEC-DEFER:phase-future.full-rpc-ownership-enforcement]` 留 Phase 51.x
- **双 DB join**：app-level（workspaces.db + users.db 分离）；owner_id 是 string 非 FK

## 9. Phase smoke gate
task-51.4 跑：cargo test -p contextforge-core + go test ./internal/consoleapi/ ./internal/cli/ + spec_drift_lint。
