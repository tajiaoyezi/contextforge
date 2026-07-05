# ADR `053`: `rbac-roles-permissions`

**Status**: Accepted（per-D ratify，Phase 52 task-52.1 交付存储基础；task-52.2-52.4 交付 proto/Go 透传 + admin-gate + auto-admin）
**Category**: 身份验证 / 安全 / v2.0 multi-user RBAC（3-role 扇平 + workspace_members）
**Date**: 2026-07-03
**Decided By**: 主 agent（ADR-012 自治）；用户方向锚定
**Related**: ADR-052（workspace-ownership, owner_id 单列）/ ADR-051（identity-foundation, per-user verified identity）/ ADR-016（cross-process gRPC bridge, Rust sole SQLite owner）/ ADR-015（console-contract-v1 FROZEN）/ ADR-018（fallback-inmem-default-reversal, byte-equivalent 默认的先例）

## Context

ADR-052 交付了 workspace ownership 存储模型（D1 owner_id 单列 + D2 owned ∪ unowned 访问边界），并把 RBAC / roles / permissions 显式 defer 到 Phase 52+：

> RBAC / roles / permissions 延后 `[SPEC-DEFER:phase-future.rbac-roles-permissions]` Phase 52+

当前（v2.0.0-alpha.2）状态：

- **identity 已 verified**：Go REST 层覆写 `X-Actor` 为 verified `user_id`（ADR-051 D3）
- **workspace 有 owner_id**：单 owner 模型（ADR-052 D1），但所有 owner 权限相同——无 destructive op 保护、无 read-only user
- **无 membership / role**：一个 workspace 只能由 owner 访问；多 user 共享、role 区分都不存在
- **destructive ops 无 gate**：4 个 destructive endpoint（reindex / pin / unpin / hard-delete）+ user 管理端点对任何 verified user 开放

本 ADR 定义 Phase 52：**3-role 扇平 RBAC**——引入 `workspace_members` 表把多 user 绑定到 workspace 并赋予固定 role，admin-gate 限制 destructive + user 管理 ops，byte-equivalent 保证 trusted-network 行为不变。

## Decision

### D1 — 3-role 扇平模型（admin / member / viewer；不做 custom role / permission table）

role 模型：固定 3-role 枚举，无 custom role、无 permission table。

- `admin`：全权（读写 + destructive + user 管理 + 成员管理）
- `member`：读写（非 destructive）
- `viewer`：只读
- role 存储为 TEXT（`'admin'` / `'member'` / `'viewer'`），DB CHECK constraint 是权威枚举守卫
- 不做 custom role（命名 role）/ permission table（role × permission 映射）/ 细粒度 per-field 权限
- 理由：3-role 扇平覆盖 v2.0 multi-user 团队部署需求（admin 管成员 + viewer 只读）；custom role / permission table 是 ACL 工程的一部分（独立 phase），塞进来会让收口无限延期（ADR-050 / 051 / 052 "分步走" 教训）

### D2 — workspace_members 表（migration 0022；多 user 共享）

membership 模型：每个 (workspace_id, user_id) 对一行，带一个 role。

- 表：`workspace_members(workspace_id, user_id, role, created_at_unix)`，PK(workspace_id, user_id)
- role CHECK IN ('admin','member','viewer')（权威枚举守卫）
- 索引：`idx_workspace_members_user(user_id)`（按 user 反查 workspace 列表）
- 无 FK 到 workspaces / users（跨 DB：membership.db 独立于 workspaces.db + users.db，per ADR-016 D1 single-owner-per-DB）——app-level join
- MembershipStore（CRUD）：`add_member` / `remove_member`（幂等）/ `list_members(workspace_id)` / `get_role(workspace_id, user_id) → Option<Role>`
- 独立 `membership.db` 文件（同 SqliteUserStore pattern，Mutex<Connection>）
- owner_id（ADR-052）与 membership 并存：workspace create 时 owner auto-gets admin membership（task-52.4）；owner_id 作为 fallback（向后兼容），membership 为 primary
- 理由：join 表是多 user 共享的最小模型；PK 防重复 membership；CHECK 把枚举下沉到 DB 层（不可绕过）；独立 DB 符合 ADR-016 单 owner-per-DB 架构

### D3 — admin-gate 范围（4 destructive + user 管理；不做全 28 路由细粒度 gate）

AuthZ 边界：destructive ops + user 管理限 admin role；其余读写 ops 对 member/admin 开放；viewer 全只读。

- admin-gate 覆盖端点（6 个，最高价值风险面）：
  - 4 destructive：reindex / pin / unpin / hard-delete（已确认 endpoint 列表见 task-52.3）
  - user 管理：`/v1/users` POST（create）/ DELETE（remove）
- 非覆盖端点：list / get / search / memory ops 等 22 路由不做 member/viewer 细粒度 gate（留 Phase 52.x `[SPEC-DEFER:phase-future.full-rpc-ownership-enforcement]`）
- gate 实现：Go `roleMiddleware`（task-52.3）每请求调 `GetMyRole` gRPC（task-52.2）→ 拿 context role → 非 admin 拒（403）
- 理由：6 端点是 irreversible / privilege-escalation 风险面；全覆盖 28 路由细粒度 gate 是独立工程（每路由 role × op 矩阵），违反分步走教训；先收最高风险面，细粒度 gate 留 Phase 52.x

### D4 — byte-equivalent 默认（trusted-network → admin；向后兼容）

trusted-network 模式（空 token，无 verified user）下，所有请求视为 admin role（跳过 gate）。

- trusted-network 判定：空 token → admin（v2.0.0-alpha.2 行为：destructive ops 无 gate）
- 非 trusted-network：必须 verified user + GetMyRole 返回 admin 才放行 destructive + user mgmt
- 现有部署（trusted-network 单用户）零行为变化——admin gate 对 admin role 透明
- 理由：不破坏 v1.x / v2.0-alpha 部署；既有 trusted-network 调用方零改动；渐进式采纳（多租户部署才需要 verified user + role gate）；同 ADR-018 byte-equivalent 默认的先例

## Trade-offs / Conscious limitations

- **3-role 固定（无 custom role）**：不支持命名 role / 继承 / 组合。复杂权限层级场景留独立 phase（需 permission table + role 管理 UI）
- **admin-gate 仅 6 端点**：22 非覆盖路由对 member 开放读写、对 viewer 不强制只读（middleware 不全）。多租户严格隔离需 Phase 52.x 全路由 gate
- **owner_id + membership 并存（冗余）**：两套数据源（owner_id fallback / membership primary）。task-52.4 auto-admin 保证一致；删除 workspace 时两处都需清理（留 Phase 52.x 一致性收敛）
- **role TEXT（非 FK / 非 enum 表）**：DB CHECK 是权威守卫，但无 referential integrity 到独立 enum 表。role 枚举扩展（加第 4 role）需 migration + CHECK 更新（add-only）
- **每请求 GetMyRole gRPC（性能）**：roleMiddleware 每请求一次 gRPC round-trip。无 cache，高 QPS 场景需 Phase 52.x 加 role cache
- **无 FK 到 workspaces/users（跨 DB）**：membership.db 独立，user/workspace 删除时 membership 行变悬空（留 Phase 52.x 一致性清理）

## Alternatives considered

- **custom role + permission table（role × permission 映射）**：拒，独立 ACL 工程（需 role 管理 UI + permission 评估引擎 + 继承），违反 ADR-050/051/052 "分步走" 教训；3-role 扇平足够 v2.0
- **owner_id 替代 membership（不加表）**：拒，单 owner 模型不支持多 user 共享（ADR-052 D4 已论证）；membership 是多 user 协作前提
- **membership FK 到 workspaces/users**：拒，跨 DB（membership.db vs workspaces.db vs users.db），SQLite 跨 DB FK 不可行（ADR-016 D1）；app-level join 足够
- **全 28 路由 member/viewer 细粒度 gate**：拒，每路由 role × op 矩阵是独立工程；先收 6 端点最高风险面（D3），细粒度 gate 留 Phase 52.x
- **role 存为 INTEGER enum（非 TEXT）**：拒，TEXT + CHECK 可读性好（DB 直接看 role 字符串）+ 与 UserStore pattern 一致；INT enum 需额外映射层
- **替换 destructive endpoint（而非加 gate）**：拒，破坏 byte-equivalent（既有 endpoint 路径不变）；加 middleware gate 是 add-only（endpoint 不变，仅加 role 检查）

## Consequences

- ✅ workspace_members 表（migration 0022，PK + CHECK）——为多 user 共享 + role 区分提供存储基础
- ✅ MembershipStore 暴露 add_member / remove_member / list_members / get_role（D2 CRUD）
- ✅ 3-role 扇平（admin/member/viewer）固定枚举（D1），DB CHECK 权威守卫
- ✅ byte-equivalent 默认（trusted-network → admin，D4；既有部署零行为变化）
- ⚠️ admin-gate 仅 6 端点（22 路由无 member/viewer 细粒度 gate；留 Phase 52.x）
- ⚠️ owner_id + membership 并存（冗余；task-52.4 auto-admin 保证一致，删除一致性留 Phase 52.x）
- ⚠️ 每请求 GetMyRole gRPC（无 cache；高 QPS 需 Phase 52.x role cache）
- ⚠️ role TEXT 无 FK（user/workspace 删除时 membership 悬空；留 Phase 52.x）

## Ratification

- D1-D4 据 task-52.1 真实交付 ratify（非纸面）：
  - task-52.1：ADR-053 + migration 0022 + MembershipStore（D1 3-role 枚举 + D2 表/CRUD + D3 admin-gate 范围定义 + D4 byte-equiv 承诺）
  - task-52.2（proto）：MembershipService add-only RPC + Rust handler 调 store（D2 访问路径 proto 层）
  - task-52.3（Go）：roleMiddleware + admin-gate + trusted-network → admin（D3 gate 落地 + D4 byte-equiv）
  - task-52.4（closeout）：workspace create auto-admin + redeem SPEC-DEFER（D2 owner-membership 一致性收敛）
