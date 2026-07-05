# ADR `052`: `workspace-ownership`

**Status**: Accepted（per-D ratify，Phase 51 task-51.1 交付存储基础；task-51.2-51.3 交付 proto/Go 透传）
**Category**: 身份验证 / 安全 / v2.0 multi-user workspace ownership
**Date**: 2026-07-03
**Decided By**: 主 agent（ADR-012 自治）；用户方向锚定
**Related**: ADR-051（identity-foundation, per-user verified identity）/ ADR-016（cross-process gRPC bridge, Rust sole SQLite owner）/ ADR-015（console-contract-v1 FROZEN）/ ADR-032（memory-ops-hardening, pin_actor 列 + guarded ALTER 同 pattern）/ ADR-045（memory pin actor 透传）

## Context

ADR-051 交付了 per-user verified identity（身份层），并把 workspace owner / per-user workspace access control 显式 defer 到 Phase 51+：

> workspace owner / per-user workspace access control 延后 `[SPEC-DEFER:phase-future.workspace-user-isolation]` Phase 51+

当前（v2.0.0-alpha）状态：

- **身份已 verified**：Go REST 层覆写 `X-Actor` 为 verified `user_id`（ADR-051 D3）
- **workspace 表无 owner 列**：`workspaces(workspace_id, name, root_path, status, config_snapshot, allowlist, denylist, created_at_unix, updated_at_unix)`（migration 0010）——任何 verified user 都能看 / 改 / 删任意 workspace
- **无 per-user 隔离**：list/get/update/delete 不按 user 过滤

本 ADR 定义 Phase 51 第一步：**workspace ownership 存储模型**——把 owner_id 落库到 workspaces 表，为 per-user access control（list/get 边界）提供数据基础，但**不引入 ACL/RBAC 权限系统**。

## Decision

### D1 — owner_id 单列 owner 模型（不做 ACL 表 / 多 owner / 组）

ownership 模型：每个 workspace 至多一个 owner（`owner_id TEXT`，可 NULL）。

- 列：`workspaces.owner_id TEXT`（migration 0021，guarded ALTER 同 0017 pattern）
- NULL owner_id = "unowned"（trusted-network 模式 + 任何 verified user 都可见；既有数据 backfill 为 NULL）
- 一个 verified user 创建的 workspace 设 `owner_id = user_id`
- 不做多 owner（join 表）/ 组 ownership / 共享 ownership
- 理由：最小增量；单 owner 覆盖 v2.0 单用户隔离需求；多 owner / 组是 ACL 工程的一部分（D4）

### D2 — 访问控制边界：owned-by-user ∪ unowned（不做 deny 列表 / 细粒度）

访问边界：verified user 可见 `owner_id = self OR owner_id IS NULL` 的 workspace。

- `list_owned(userID)`：`WHERE status != 'deleted' AND (owner_id = ? OR owner_id IS NULL)`
- `get_if_owned(id, userID)`：`WHERE workspace_id = ? AND (owner_id = ? OR owner_id IS NULL)`；非 owner 且非 unowned → None
- `create_owned(req)`：写入 `req.owner_id`
- unowned（NULL）对任何 verified user 开放（向后兼容既有 trusted-network 数据）
- 理由：二值边界（owned ∪ unowned）足够覆盖 v2.0；细粒度 deny / role 留 Phase 52 RBAC

### D3 — byte-equivalent 默认（向后兼容）

既有 create/list/get 行为不变（owner_id 不写 / 不过滤）。

- 现有 `create()` 保持原样（不写 owner_id → NULL；返回 `owner_id: None`）——byte-equivalent
- 现有 `list()` / `get()` 加 owner_id 到 SELECT + 填充字段，但**不做 owner 过滤**（返回所有非 deleted 行；既有调用方看到 `owner_id: Option=None` 对旧行，backward compatible）
- 新方法 `create_owned` / `list_owned` / `get_if_owned` 是**新增**路径（proto/Go 在 task-51.2/51.3 接入），不替换现有方法
- 理由：不破坏 v1.x / v2.0-alpha 部署；既有调用方零改动；渐进式采纳（trusted-network 模式仍用 create/list/get）

### D4 — 不做 RBAC / ACL / 共享 / transfer（Phase 52+）

本 ADR 仅定义 ownership（D1 列 + D2 边界），不做授权（AuthZ）。

- RBAC / roles / permissions 延后 `[SPEC-DEFER:phase-future.rbac-roles-permissions]` Phase 52+
- workspace 共享（share to another user）/ 转移（transfer ownership）延后 `[SPEC-DEFER:phase-future.workspace-sharing-transfer]` Phase 53+
- 细粒度 per-field 权限 / deny 列表延后 Phase 52 RBAC
- 理由：ownership 是 AuthZ 的最小前置（无 owner 无法做 "deny non-owner"）；塞 RBAC 进本 phase 会让收口无限延期（ADR-050 / ADR-051 D4 教训）

## Trade-offs / Conscious limitations

- **单 owner 模型**：不支持 workspace 被多 user 共享。多 user 协作场景留 Phase 53+（需 share/transfer + 邀请流）
- **NULL = unowned（非 "everyone"）**：语义上 unowned 对任何 verified user 可见，与 ACL "public" 等价但**不建模为显式标记**。生产多租户场景需 Phase 52 RBAC 把 NULL 收敛为显式 policy
- **既有 trusted-network 数据 owner_id = NULL**：迁移后所有旧行变 unowned（任何 verified user 可见）。这是过渡期妥协——多租户部署需手动 backfill owner 或等 Phase 52 RBAC
- **create_owned 与 create 并存（非替换）**：两条路径共存，proto/Go 自行选择。短期增加 API surface，但保证 byte-equivalent（create 不变）
- **owner_id 是 user_id 字符串（非 FK）**：不建外键约束到 users 表（Rust sole owner，跨表 FK 在 SQLite 跨 migration 复杂；user 删除时 owner_id 变悬空留 Phase 52 处理）

## Alternatives considered

- **ACL 权限表（workspace_id × user_id × role）**：拒，独立工程（需 role 枚举 + 共享流 + 权限评估），违反 ADR-050/051 "分步走" 教训；留 Phase 52 RBAC
- **多 owner（join 表 workspace_owners）**：拒，超出 v2.0 单用户隔离需求；增加查询复杂度（每 list 需 join）；单 owner 足够
- **owner_id NOT NULL（强制 owner）**：拒，破坏 byte-equivalent（既有 NULL 行无法 backfill 真实 owner；trusted-network 模式创建的 workspace 无 verified user）；NULL 兼容过渡
- **替换 create/list/get（而非新增 create_owned/...）**：拒，破坏 byte-equivalent；既有调用方（Go REST v1.x 路径）会因新增 owner 过滤而漏数据；新增方法保证零回归
- **Postgres / 外部权限系统**：拒，破坏 ADR-004/016 local-first；与现有 SQLite 架构不一致

## Consequences

- ✅ workspaces 表获得 owner_id 列（migration 0021，guarded 幂等）——为 per-user 隔离提供存储基础
- ✅ WorkspaceStore 暴露 create_owned / list_owned / get_if_owned（D2 访问边界）
- ✅ byte-equivalent 默认（既有 create/list/get 不变；旧行 owner_id NULL）
- ✅ 关闭 `[SPEC-DEFER:phase-future.workspace-user-isolation]` 的存储部分（proto/Go 接入留 task-51.2/51.3）
- ⚠️ 单 owner（不支持共享 / 转移；留 Phase 53+）
- ⚠️ NULL = unowned 对任何 verified user 可见（过渡期；多租户需 Phase 52 RBAC 收敛）
- ⚠️ owner_id 非 FK（user 删除时悬空；留 Phase 52）

## Ratification

- D1-D4 据 task-51.1 真实交付 ratify（非纸面）：
  - task-51.1：migration 0021 + WorkspaceStore owner 方法（D1 列 + D2 边界 + D3 byte-equiv + D4 不做 ACL 的诚实记录）
  - task-51.2（proto）：WorkspaceService 调 owner store（D2 访问路径 proto 层）
  - task-51.3（Go）：REST handler 传 verified owner（D2 Go 层透传 + D3 byte-equiv 选择路径）
