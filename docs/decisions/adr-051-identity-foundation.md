# ADR `051`: `identity-foundation`

**Status**: Accepted（per-D ratify，Phase 50 task-50.1 交付存储基础；task-50.2-50.4 交付 proto/Go/覆写）
**Category**: 身份验证 / 安全 / v2.0 multi-user 基础
**Date**: 2026-07-05
**Decided By**: 主 agent（ADR-012 自治）；用户方向锚定
**Related**: ADR-004（local-first-privacy-baseline）/ ADR-015（console-contract-v1 FROZEN）/ ADR-016（cross-process gRPC bridge, Rust sole SQLite owner）/ ADR-045（memory pin actor 透传）/ ADR-050（v1.0 不含清单推 v2.0）

## Context

v1.0/v1.1 的审查确认：当前**零身份层**。

- **AuthN**：单一共享 bearer token（`CONTEXTFORGE_CONSOLEAPI_AUTH_TOKEN`，空 = trusted-network 开放模式）；非 per-user
- **Identity**：`X-Actor` header 是 caller 自填未验证字符串，仅 pin/unpin 2 个 handler 读取；proto/SQL 无 user/session/tenant 字段
- **gRPC bridge**：`:50551` loopback-trusted，无认证（ADR-016 D2）
- **`memory_items.pinned_by`**（migration 0017）：存的是未验证的 declared actor

ADR-050 明确 multi-user/auth 推 v2.0："ADR-051+ 承接"。本 ADR 定义 v2.0 第一步：**per-user 身份验证基础**——让 actor 从 declared 变 verified，为 Phase 51+（workspace isolation）和 Phase 52+（RBAC）提供 subject 绑定点。

## Decision

### D1 — Per-user token → user 映射（不做 OAuth/OIDC）

身份模型：每个 user 有一个 token，bearer middleware 匹配 `users.token` 解析出 verified `user_id`。

- 初始：CLI/配置文件注册（POST /v1/users），非 OAuth/OIDC
- OAuth/OIDC 外部 IdP 延后 `[SPEC-DEFER:phase-future.oauth-oidc-idp]` Phase 53+（委托 Caddy forward_auth）
- 理由：最小增量；外部 IdP 是独立工程（需 OIDC provider 对接 + token 验证 + JWKS）

### D2 — SQLite users 表（不引入 Postgres）

身份存储：Rust-owned SQLite migration（`0020_users.sql`），`users(id, name, token UNIQUE, created_at_unix)`。

- 保持 local-first（ADR-004/016）；Postgres 破坏 local-first baseline 且 ADR-016 明确拒绝
- Rust 是 sole SQLite owner（ADR-016 D1）；Go 通过新 gRPC UserService 访问
- token 明文存（local-first 妥协）；hash 存延后 `[SPEC-DEFER:phase-future.token-hash-storage]` Phase 51+（需 salt + HMAC 评估）
- 理由：与现有架构一致（memory/eval/workspace 都是 SQLite + Rust-owned）

### D3 — Go 覆写 actor（ADR-016 D3 thin proxy）

身份传播：Go REST 层验证身份后，**覆写** `X-Actor` 为 verified `user_id`（caller 声明值丢弃）。gRPC `actor` proto 字段不变（add-only，ADR-015 FROZEN）——Rust 收到的 actor 是 verified 值。

- Rust 不改（信任 Go 断言；loopback gRPC 保持 trusted，ADR-016 D2）
- 不引入 gRPC interceptor credentials（loopback 下收益有限，工程量大）
- 理由：最小改动；符合 ADR-016 D3（Go 是 thin proxy，Rust 信任 Go 断言）

### D4 — 不做 RBAC / workspace ownership / permissions

本 ADR 仅定义身份验证（AuthN + actor verified），不做授权（AuthZ）。

- RBAC / roles / permissions 延后 `[SPEC-DEFER:phase-future.rbac-roles-permissions]` Phase 52+
- workspace owner / per-user workspace access control 延后 `[SPEC-DEFER:phase-future.workspace-user-isolation]` Phase 51+
- 理由：身份是 AuthZ 的前置依赖（无 verified subject 无法绑权限）；塞 RBAC 进本 phase 会让收口无限延期（ADR-050 教训）

### D5 — byte-equivalent 默认（向后兼容）

trusted-network 模式（空 token）+ 旧 shared token 行为不变。

- 空 token → trusted-network（actor 仍回落 `"console-api"`，byte-equivalent）
- 旧 shared token（`CONTEXTFORGE_CONSOLEAPI_AUTH_TOKEN`）→ 不注入 verified identity（actor 仍用 X-Actor 声明值，旧行为）
- 新 per-user token → 注入 verified identity（actor = user_id）
- 理由：不破坏 v1.x 部署；渐进式采纳

## Trade-offs / Conscious limitations

- **token 明文存**：local-first 妥协。生产建议 file permission 0600 + Caddy TLS。hash 存留 Phase 51+
- **admin 分级简化**：初始 POST /v1/users 任何有效 token 都可注册（admin 分级 `[SPEC-DEFER:phase-future.rbac-roles-permissions]` Phase 52 RBAC）
- **gRPC bridge 仍 loopback-trusted**：身份验证只在 REST 层，Rust 不知 user 是谁（actor 是 Go 覆写后的值）。interceptor credentials 延后（收益有限）
- **无 token rotation/revocation UI**：CLI 注册即可，管理 UI 留 Phase 54+

## Alternatives considered

- **Postgres**：拒，破坏 ADR-004/016 local-first；与现有 SQLite 架构不一致
- **OAuth/OIDC 外部 IdP**：拒，独立工程；留 Phase 53+ 委托 Caddy forward_auth
- **gRPC interceptor credentials**：拒，loopback 下收益有限；Go 覆写 actor 更小改动
- **全 RBAC 一步到位**：拒，ADR-050 教训"中型 feature 塞进一个 phase 会让收口无限延期"；身份是 AuthZ 前置依赖，分步走

## Consequences

- ✅ actor 从 declared 变 verified（关闭 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`）
- ✅ 为 Phase 51+ workspace isolation 和 Phase 52+ RBAC 提供 subject 绑定点
- ✅ byte-equivalent 默认（v1.x 部署不破坏）
- ⚠️ token 明文存（生产需 file permission + TLS；hash 存留 Phase 51+）
- ⚠️ gRPC bridge 仍 loopback-trusted（身份只在 REST 层）

## Ratification

- D1-D5 据 task-50.1-50.4 真实交付 ratify（非纸面）：
  - task-50.1：SQLite migration + UserStore（D2 存储）
  - task-50.2：proto UserService + Rust gRPC（D2 访问路径）
  - task-50.3：Go REST + bearer 解析 + actor 覆写（D1 映射 + D3 传播 + D5 byte-equiv）
  - task-50.4：redeem marker + smoke（D4 不做 RBAC 的诚实记录 + 全链验证）
