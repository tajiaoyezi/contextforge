# ADR `015`: `console-contract-v1-compatibility`

**Status**: Proposed
**Category**: Integration / 跨仓库契约
**Date**: 2026-05-24
**Decided By**: tajiaoyezi objective + main agent execution
**Related**: ADR-001 (go-rust-dual-binary-architecture) / ADR-003 (cli-rest-mcp-grpc-interfaces) / ADR-004 (local-first-privacy-baseline) / ADR-013 (cli-data-plane-grpc-bridge) / ADR-014 (cross-phase-exit-criteria-validation) / PRD §Open Questions O13 / Console PRD §Technical Approach「Contract v1 must-have 字段」/ Console `console-api/internal/coreadapter/contractv1/contractv1.go`

## Context

ContextForge-Console (后文简称 Console) v1.0 已 ship — 含 `console-web` (Next.js) + `console-api` (Gin BFF) + `internal/coreadapter` 四种 Adapter (HTTP / gRPC / CLI / Mock) + PostgreSQL/Redis 元数据。Console PRD §Technical Approach 显式声明 ContextForge Core 的对接边界是 **Core Integration Contract v1** — 一组版本化 must-have 字段 (single source of truth 落在 Console PRD §Technical Approach「Contract v1 must-have 字段」段 + Console `internal/coreadapter/contractv1/contractv1.go` 镜像)。

**Console 端已就绪**：
- HTTPAdapter (`console-api/internal/coreadapter/http_adapter.go`) — 通过 `/v1/*` REST endpoints 调 Core
- Mock Adapter — 返回 contractv1 假数据，Phase 1-6 不依赖真 Core 即可开发
- fakehttpserver (`console-api/internal/coreadapter/testhelper/fakehttpserver.go`) — Console conformance test 用作 oracle，固化 Console HTTPAdapter 期望的 URL path / request body / response JSON shape / 错误码 mapping
- 19+ endpoints (workspace/index-job/search/source-chunks/memory/eval-runs/observability)

**ContextForge 端缺口**：
- 截至 v0.2.0 (`master` = `1854646`)，ContextForge 暴露的对外接口只有：
  - CLI binary `contextforge init/import/index/search/eval` (Phase 1-9)
  - gRPC `ContextService.{Search, Health, Index}` (Phase 1 + 9, Rust core daemon)
  - REST stub `/v1/search` (task-6.2，limited scope)
  - MCP `contextforge-mcp` (Phase 7)
- **无** Console 期望的 Workspace / IndexJob 资源模型（资源生命周期对 Console 是 first-class，对 ContextForge v0.2 是 implicit — collection 当 workspace 用 + index 同步阻塞跑，不暴露 job 句柄）
- **无** 对齐 Console Contract v1 9-19 endpoint REST 面（health / workspaces / index-jobs / search 嵌套 trace / observability events）
- **无** Go types 镜像 Console `contractv1.go` （即便 task-10.4 写出 REST handler，response shape 散落各处易漂移）

**O13 + Console 端等待的关键动作**：Console 至今只能跑 Mock Adapter (Phase 1-6 内部开发就绪) + 部分 CLI Adapter (Console phase 7 integration 阶段)；HTTPAdapter 在 Console 端实现完整，但**对端 Core HTTP server 不存在** — 任何 Console UI demo 调真 ContextForge 都会撞 404。本 ADR 就此跨仓库 gap 立 v0.3 Phase 10 决策。

**Console 单边契约权威性**：Console PRD §Technical Approach「Contract v1 must-have 字段」+ `contractv1.go` 是单一事实源，ContextForge 不重新定义字段 shape；ContextForge 只在自己仓库内做 **镜像 + 实现 + 满足**。Cross-repo 反向依赖：若 Console 期望字段或行为需要调整（如新增 must-have 字段 / 行为不对齐），由 Console 端 PR 驱动，ContextForge 镜像更新跟进。

## Decision

ContextForge v0.3 (Phase 10) 实施 **Console Contract v1 兼容层**，由 6 个 Decision 段组成。所有 Decision 严格围绕"对齐已存在的 Console contractv1.go single source of truth"，不重新定义字段。

### D1 — internal/contractv1/ Go types 镜像包

在 `internal/contractv1/` 新建 Go 包，1:1 镜像 Console `console-api/internal/coreadapter/contractv1/contractv1.go` 中所有 Contract v1 类型（Workspace / IndexJob / SearchRequest / SearchResult / RetrievalTrace / SourceChunk / Citation / MemoryItem / EvalRun / EvalRunCreate / ObservabilityEvent / FieldAvailability / CoreHealth / WorkspaceCreate / CaseResult / AgentScope / MemoryOperation 共 17 类型）。

约束：
- 字段名 / json tag / type 与 Console `contractv1.go` 完全一致（含 `*time.Time` / `*string` / `*int` 等 nullable 表达 — R7 PRD §字段分级原则）
- 包内**禁止**导入任何 ContextForge 内部业务包（保持纯 schema 包，与 Console `contractv1.go` 同款约束）— 仅依赖 `encoding/json` + `time`
- `ContractVersion = "v1"` 常量
- `FieldAvailability` 类型 + `Complete()` / `IsMissing()` helper
- `types_test.go` 跑 JSON marshal/unmarshal roundtrip 验证字段 tag 一致性（task-10.1 §6 AC1-5 + task-10.5 conformance）

### D2 — Workspace 资源模型 + Rust 持久化

ContextForge v0.2 内部用 `collection_id` 作为 namespace；Console Contract v1 用 `workspace_id` 作为顶层资源。本 ADR 决策：

- **workspace_id ↔ collection_id 1:1 映射**（v0.3 简化策略，不引入多 collection per workspace）：workspace 持久化后，其 `workspace_id` 即下层 collection_id 字符串。Workspace 元数据（name / root_path / status / allowlist / denylist / config_snapshot / created_at / updated_at）独立持久化到新 SQLite 表 `workspaces`（migration `0010_workspaces.sql`）
- **Rust 侧落地**：在 `core/src/workspace/` 新增 module，含 `Workspace` struct + `WorkspaceStore` CRUD trait + `SqliteWorkspaceStore` 实现（基于现有 task-1.2 / 1.3 / 2.4 SQLite 链路扩展，**不**引入新 SQLite driver — 仍用 rusqlite）
- **生命周期**：create → ready (synchronously initialize collection dir + chunks DB) → updated (config_snapshot 更新触发) → deleted (软删 + 保留物理目录，soft-delete 行为 v0.4 再细化 [SPEC-DEFER:task-future.workspace-soft-delete])
- 详 task-10.2 §3 / §5

### D3 — IndexJob 资源模型 + 异步 lifecycle

ContextForge v0.2 `contextforge index` 是同步阻塞流（cli/daemon/core 三段一直跑到底），不暴露 job 句柄。Console Contract v1 要求 IndexJob 是 first-class 资源 + 状态机 (queued/running/succeeded/failed/cancelled) + heartbeat。本 ADR 决策：

- **新增异步 lifecycle 层**（不破坏 v0.2 同步 CLI）：在 `core/src/jobs/` 新建 module，含 `IndexJob` struct + `JobStore` trait + `SqliteJobStore` 实现（migration `0011_index_jobs.sql`）+ `JobRunner` 异步执行器（基于 tokio spawn）
- **状态机**：`queued → running → (succeeded | failed | cancelled)`；`running` 状态每 N=5 秒（轻 heartbeat）更新 `last_heartbeat_at` + 当前 stage / processed_files / total_files 字段
- **触发**：HTTP `POST /v1/index-jobs` 入 queued，立即返 IndexJob with job_id；JobRunner 异步消费 queue（v0.3 单 worker 串行，多 worker 留 [SPEC-DEFER:task-future.job-parallelism]）；CLI `contextforge index` 同步流不走 JobRunner，依旧直接调 Index gRPC（v0.2 行为保留）
- **取消**：HTTP `POST /v1/index-jobs/:job_id/cancel` 设置 cancellation flag，JobRunner 在下一个 stage boundary 检测后退出（co-operative cancel，非 hard kill；详 task-10.3 §3）
- 详 task-10.3 §3 / §5

### D4 — REST API 9 endpoints (Phase 10 必有，覆盖 Console Mock→HTTP 迁移最小集)

`internal/consoleapi/` 新建 Go package，含 9 个 REST handler，路径 / shape 严格对齐 Console HTTPAdapter 期望（见 task-10.4 §5.3 单一事实源）：

1. `GET  /v1/health` → CoreHealth (`contract_version: "v1"` 必含)
2. `POST /v1/workspaces` (body WorkspaceCreate) → Workspace
3. `GET  /v1/workspaces` → []Workspace
4. `GET  /v1/workspaces/:workspace_id` → Workspace (404 → ErrNotFound)
5. `POST /v1/index-jobs` (body `{workspace_id}`) → IndexJob (status: "queued")
6. `GET  /v1/index-jobs/:job_id` → IndexJob (404 → ErrNotFound)
7. `POST /v1/index-jobs/:job_id/cancel` → 200 OK / 409 if terminal
8. `POST /v1/search` (body SearchRequest) → `{result: SearchResult, trace: RetrievalTrace}` (Console HTTPAdapter 嵌套约定，见 fakehttpserver oracle)
9. `GET  /v1/observability/events` → []ObservabilityEvent (long-poll，**非** SSE — Console HTTPAdapter v1.0 不消费 SSE)

约束：
- 错误码 mapping：404 / 409 / 5xx 对齐 Console `ErrNotFound` / `ErrConflict` / `ErrCoreUnavailable` 句柄约定（见 Console `internal/coreadapter/http_adapter.go` error mapping）
- 认证：默认 `trusted-network`（无 Authorization header 要求）+ 可选 bearer token（env `CONTEXTFORGE_CONSOLEAPI_AUTH_TOKEN` 启用时强制 `Authorization: Bearer <token>`）— 与 Console `CONSOLE_API_CORE_AUTH_MODE=token` 模式对接
- daemon bind: 默认 `127.0.0.1:48181` (R4 local-first)；CORS 默认收敛同 task-6.2 现有 daemon REST
- OpenAPI yaml 落 `docs/consoleapi/openapi.yaml`（task-10.4 AC4 产物）
- 其它 endpoint (`/v1/memory*` / `/v1/eval-runs*` / `/v1/source-chunks/:id` / `/v1/search/:query_id/trace` / `/v1/workspaces/:id/config` PATCH 等 10 个) **不在** Phase 10 scope (`[SPEC-DEFER:task-future.consoleapi-extension]` — Console Mock Adapter 覆盖直到 v0.4 才需要)

### D5 — Conformance test 反向依赖 Console fakehttpserver

ContextForge `test/conformance/console_contractv1_test.go` 反向取 Console fakehttpserver 设定的 oracle，验证 ContextForge REST endpoints 输出 JSON 能被 Console HTTPAdapter 正确解析：

- 测试方式：spawn ContextForge daemon (含 D4 9 endpoints) → 用 Console HTTPAdapter Go client (`console-api/internal/coreadapter/http_adapter.go`) 调本地 ContextForge → 断言返回的 Contract v1 类型 unmarshal 完整 + FieldAvailability.Missing 为期望集合
- Cross-repo 依赖：CI 跑 conformance test 时需 fetch Console 仓库 (read-only) — 在 `test/conformance/` 的 README 文档化路径假设 (`$CONSOLE_REPO=H:/devlopment/code/ContextForge-Console`) + 提供 skip 机制（env 未设时 SKIP 不 fail，避免 fork 环境 broken）
- **不**修改 Console 任何文件（ADR-014 D4 + playbook §自决规则 #8 cross-repo 写硬约束）
- 详 task-10.5 §3 / §5

### D6 — Docker compose 端到端集成 smoke

`scripts/console_smoke.sh` 启动：
1. `docker compose -f deploy/console-stack.yml up -d` (新 yml; 含 Console v1.0 docker image + ContextForge daemon + Postgres + Redis)
2. 健康检查（poll Console UI `:3000/healthz` + ContextForge `:48181/v1/health`）
3. `curl http://localhost:3000/api/workspaces` 通过 Console BFF 调真 ContextForge → 真返回 workspace 列表（非 Mock 数据）
4. `CONSOLE_SMOKE_EXIT=0` final marker

约束：
- v0.3 仅 Linux/WSL2 跑（macOS 应能跑但不在 §6 AC）；Windows skip with override
- Console docker image 取 Console 仓库已 ship 版本（拉取或本地 build；smoke script 文档化两种路径）
- Deploy yml 落 `deploy/console-stack.yml`（task-10.6 AC2 产物）
- 详 task-10.6 §3 / §5

## Rationale

- **不重新定义 Contract v1**：Console PRD + contractv1.go 是 single source of truth；ContextForge 重新定义会引入 drift 风险（ADR-014 D2 lint 也会捕获 "重新定义" 的 anti-pattern）。镜像策略 → cross-repo 字段对齐 verifiable
- **workspace_id 1:1 collection_id**：v0.2 现有 collection 概念 + Console workspace 概念语义近似（都是 "数据集"），1:1 映射避免双层 namespace；多 collection per workspace [SPEC-DEFER:task-future.multi-collection]
- **异步 IndexJob 不破坏 v0.2 同步 CLI**：CLI 同步流性能更优（无 worker queue overhead）+ 行为已稳定；Console 异步流另起 JobRunner，REST handler 单独走，两条路径共享底层 IndexSession::index_path API
- **9 endpoints 最小集**：Console Mock→HTTP 迁移 happy path 所需最少集（workspace 三段 CRUD + index job lifecycle + search + health + observability）；其它 10+ endpoint (`/v1/memory*` / `/v1/eval-runs*` / etc.) 在 Mock Adapter 覆盖期间不阻塞，留 v0.4 task
- **REST 不引入新 web framework**：复用 task-6.2 现有 daemon REST 已用的 net/http + chi router（Phase 6 已选 chi 且过 R7 dep gate）；Authentication 复用 task-6.2 local random token 模式但扩展为 bearer header 形式
- **Conformance test 反向 fakehttpserver**：Console fakehttpserver 是 Console 仓库自我 oracle，ContextForge 反向引用它验证"对端会怎么调我"是最低假设成本的对齐手段
- **Docker compose 是 Console v1.0 主发布形态**（Console PRD §Constraints "发布" 段）；ContextForge v0.3 集成 smoke 直接落 docker compose 与 Console 同款部署模式

## Alternatives

- **A. ContextForge 重新定义 Contract v1（不镜像 Console contractv1.go）**：会与 Console 长期 drift；Console 任何字段变更都需 ContextForge 同步审；废弃
- **B. ContextForge 只暴露 gRPC，不暴露 REST（让 Console gRPC Adapter 调）**：Console HTTPAdapter 已实现完整 + Console PRD §Technical Approach 4 种 Adapter 平行，HTTP 是用户最熟悉的对接形态；只走 gRPC 会阻塞 Console 任何使用 HTTPAdapter 的部署；废弃
- **C. ContextForge 在 daemon 现有 `:48181/v1/search` stub 之上一次性扩 19+ endpoint 全套**：scope 过大（19 endpoint 含 memory / eval / 等仍未完整实现的业务），v0.3 时间表 (Phase 10 单 phase) 装不下；切 9 endpoint 必有 + 10 endpoint defer 是 scope 平衡
- **D. workspace_id 独立 from collection_id (两层 namespace)**：增加迁移复杂度（v0.2 已有的 collection 数据需重新关联 workspace）+ 模型冗余；1:1 映射是最小可演进路径
- **E. IndexJob 用现有 v0.2 manifest store 复用 (不新建 jobs 表)**：manifest 是 reliability checkpoint（task-8.2），语义为"已写入哪些 chunks 防 resume 重复"，不是"job 生命周期"；混用语义会引入 schema drift；新建 `0011_index_jobs.sql` 是正确边界

## Consequences

**正面**：
- Console v1.0 已 ship 等真 Core 的 gap 在 Phase 10 闭环 — Console UI 能调真 ContextForge，不只 Mock
- ContextForge 对外 REST 面从 v0.2 单 endpoint 扩到 9，覆盖 Console Mock→HTTP 迁移最小集
- internal/contractv1/ Go types 镜像 + conformance test 反向 fakehttpserver → 跨仓库字段 drift 在 ContextForge CI 阶段就被捕获
- Workspace / IndexJob 资源模型为 ContextForge v0.4+ 多 collection / 多 worker / RBAC 演进留扩展点

**负面 / 成本**：
- 跨仓库依赖：ContextForge conformance test 引入 Console 仓库只读 fetch；CI 需双仓库 checkout
- 镜像维护：Console contractv1.go 任何字段变更都需 ContextForge internal/contractv1/ 同步（playbook §自决规则 #8 触发 STOP — 由用户协调 cross-repo PR）
- IndexJob 异步 lifecycle 引入 SQLite jobs 表 + tokio spawn + heartbeat — Rust async 复杂度上升 (playbook §预测 E5 撞 STOP 概率 55%)
- 9 endpoint REST 增加 internal/daemon 维护面（task-6.2 已有 search stub，task-10.4 重组使 9 endpoint 共享一个 router）
- Docker compose 跨容器网络 + Console image fetch 在 CI 上配置复杂

**对 v0.4+ 的影响**：
- v0.4 可继续 incremental 加 `/v1/memory*` / `/v1/eval-runs*` / `/v1/source-chunks/:id` / `/v1/search/:query_id/trace` 等 Console 期望但 v0.3 OOS 的 endpoint
- 多 worker IndexJob + soft-delete workspace + 多 collection per workspace 等扩展点都在 D2/D3 模型内可演进

## Rollback Or Migration Plan

如 Phase 10 实施中发现：

1. **D2 workspace_id ↔ collection_id 1:1 映射撞用户期望**（如用户期望多 collection per workspace）：保留 D2 模型但加 `default_collection_id` 字段；Console 端继续单 workspace_id 调用，ContextForge 端内部多 collection 落地。新 task-future.multi-collection 跟进。
2. **D3 IndexJob 异步 lifecycle 引发 race condition / heartbeat 间隔不合理**：保留状态机但调整 heartbeat 周期 / 加 stage transition 锁；不回退到同步流（同步流不符合 Console 长任务 UX）
3. **D4 9 endpoint scope 不足**（如 Console 端 Phase 7 integration 验收需更多 endpoint）：增量加 endpoint，每个走独立 task PR（不在 task-10.4 内一次扩，避免 PR 过大）
4. **D5 Console fakehttpserver 与实际 HTTPAdapter 行为不一致**（cross-repo 反向依赖触发）：playbook §自决规则 #8 转 §8 STOP，由用户协调 Console 端 PR；ContextForge 端 conformance test 暂 SKIP 该 case
5. **D6 docker compose 跨容器网络问题 / Console image 拉取失败**：smoke script 加 fallback（用本地源码 build Console image）；Linux/WSL2 优先验证，Windows skip

Rollback 通过新 ADR superseding 完成；Phase 10 已 ship 的 9 endpoint 保持向后兼容（add-only 演进，与 task-1.1 proto 同款规则）。

## Follow-ups

- **本 ADR Accepted in Phase 10 closeout PR**（task-10.6 完成后 + Phase 10 closeout PR 内）— Proposed → Accepted 在 closeout commit 内回填
- **Phase 10 实施后**：v0.4 增量 endpoint task（memory / eval / source-chunks / search trace / workspace config PATCH 等）按 Console 实际 UI 优先级排
- **Cross-repo 治理 follow-up**：ADR-014 D5 的 cross-repo amendment 机制（Console 端字段变更 → ContextForge 镜像更新流程）需 v0.4 governance retrospective 评估是否制度化
- **关联 PRD §Open Questions O13**：本 ADR Accepted 后可 mark O13 resolved by ADR-015（PRD update 在 closeout PR 内）
- **关联 ADR-014 D2 lint**：Phase 10 全程 spec PR 跑 `bash scripts/spec_drift_lint.sh --touched origin/master` + closeout PR 含 D1 mapping 表 + D2 输出 (playbook §自决规则 #9 / #10)
