# ADR `016`: `cross-process-rust-go-via-grpc-bridge`

**Status**: Proposed
**Category**: 架构 / 跨进程协议接口
**Date**: 2026-05-25
**Decided By**: tajiaoyezi objective + main agent execution
**Related**: ADR-001 (go-rust-dual-binary-architecture) / ADR-003 (cli-rest-mcp-grpc-interfaces) / ADR-004 (local-first-privacy-baseline) / ADR-013 (cli-data-plane-grpc-bridge) / ADR-014 (cross-phase-exit-criteria-validation) / ADR-015 (console-contract-v1-compatibility) / PRD §Open Questions O14

## Context

ContextForge v0.3 (Phase 10，HEAD `c141a97`) 完成了 [ADR-015](./adr-015-console-contract-v1-compatibility.md) Console Contract v1 兼容层 ——
9 REST endpoint + Go contractv1 17 types + `core/migrations/0010_workspaces.sql` + `core/migrations/0011_index_jobs.sql` + conformance test 反向跑通 + docker compose 集成 smoke。
但 task-10.4 §10 Completion Notes Trade-off #1 + #2 显式记录了 **v0.3 范围内的两处 conscious gap**：

1. **Trade-off #1 (in-memory MemStore)**：v0.3 REST handler 没真接 Rust workspace/jobs 持久化，而是用 `internal/consoleapi/memstore.go` in-memory 模拟，daemon 重启即丢失；
   原由是 Go 进程跨进程直接打开 Rust 写的 SQLite 文件 (workspaces.db) 需要 `mattn/go-sqlite3` / `modernc.org/sqlite` 新 R7 dep + 跨进程 WAL 并发边角 case ——
   被标 `[SPEC-DEFER:task-future.cross-process-sqlite-sharing]`
2. **Trade-off #2 (JobRunner 不真索引)**：v0.3 `POST /v1/index-jobs` 入队列后状态推进 + heartbeat 仍是占位推进（`internal/consoleapi/memstore.go::JobAdapter.runJobAsync` 模拟 200ms tick）；
   Rust 侧 `SqliteJobStore + JobRunner` 单独存在（task-10.3）但未被 REST 路径触发，因此 `POST /v1/search` 没真 indexed 数据；
   Console UI 看到 queued → succeeded 仍是 Go 进程内模拟。

**v0.4 缺口**：上述两处的统一根因是 **Go console-api-serve 与 Rust contextforge-core 在 v0.3 是两条独立进程，没有共享持久化通路** ——
- v0.2 / v0.3 已通过 [ADR-013](./adr-013-cli-data-plane-grpc-bridge.md) 在 `core/proto/contextforge/v1/service.proto` 落了 cli-data-plane gRPC（Phase 9 task-9.1/9.2/9.3 实施），
  端口默认 `:48180`，已建立 Go CLI → Rust core 的 gRPC 桥模式
- v0.3 REST 面没有沿用 ADR-013 模式 —— 它在 v0.3 实施时被 task-10.4 §10 trade-off 显式选择"in-memory + future cross-process-sqlite-sharing"
- 现在 v0.4 要把两个 trade-off 一次性 resolve：**复用 ADR-013 gRPC bridge 模式 + 扩 4 个新 service**，让 console-api-serve 变成 thin REST→gRPC translator

**为什么不直接 Go 打开 SQLite？**
- 跨进程 WAL 并发：`SqliteWorkspaceStore` (rusqlite) 当前用 `journal_mode=WAL`；同时 Go 端用 `modernc.org/sqlite` (或 cgo `mattn/go-sqlite3`) 打开会引入跨进程锁 / journal 文件竞争边角 case，且 v0.2 测试 fixture 完全不覆盖此场景
- schema 双 owner：`core/migrations/` 由 Rust 团队管；Go 端如果直接读 SQLite 即便只读，未来 schema 演进时 Go 仍要追表名 / 列名变更 → schema drift 风险
- 与 ADR-004 local-first 一致：单进程独占持久化通路更简单（fork/spawn race / pid file / file lock 都不需要）
- 与 ADR-001 双二进制架构一致：Go 控制面 + Rust 数据面，数据面包含**所有**持久化（包括 workspace/job 元数据，不只 chunk/index）

**为什么不引入 Postgres / 外部 DB？**
- 与 ADR-004 local-first privacy baseline 冲突：外部 DB 引入 deploy 复杂度 + 默认不本地
- v0.4 仍是 single-user 场景；multi-user 留 v1.0

**v0.4 与 v0.3 的 Console 端关系**：
- Console UI 端**无任何改动** —— Contract v1 字段集合在 v0.3 锁定 ([ADR-015](./adr-015-console-contract-v1-compatibility.md) D1)；v0.4 不修改 `internal/contractv1/contractv1.go`
- Console fakehttpserver oracle 不变；v0.3 conformance test (`test/conformance/console_contractv1_test.go`) 在 v0.4 必须仍 PASS（不退化）
- v0.4 仅在 ContextForge 单仓内补完业务层 wiring；cross-repo dep 强度同 v0.3（可选 `$CONSOLE_REPO` 反向读）

**O14 在 PRD §Open Questions 新提出**（本 ADR 自身回答 + 落 resolved by ADR-016）：v0.4 Phase 11 通过 cross-process gRPC bridge 把 task-10.4 §10 两个 trade-off 一次 resolve，给 Console UI 端真持久化业务面。

## Decision

ContextForge v0.4 (Phase 11) 实施 **跨进程 Rust ↔ Go gRPC bridge 业务面**，由 6 个 Decision 段组成。所有 Decision 围绕"Rust 持 SoT + Go 是 thin proxy"，与 [ADR-013](./adr-013-cli-data-plane-grpc-bridge.md) 已建立的 cli-data-plane gRPC 模式同款。

### D1 — Rust 持 SoT (Single Source of Truth)

所有 Workspace / IndexJob / observability event 的 SQLite 持久化只在 Rust 侧；Go console-api-serve **不直接打开任何 SQLite 文件**。
- `core/migrations/*.sql` 单 owner = Rust 团队（schema 变更由 Rust PR 驱动；详 D5）
- `SqliteWorkspaceStore` (`core/src/workspace/mod.rs`，task-10.2 已建) + `SqliteJobStore` (`core/src/jobs/mod.rs`，task-10.3 已建) 只在 Rust 进程内被实例化
- Go 端只持有 gRPC client stub；任何 "Go 写 SQLite" 的 code path 都是 D1 违反，触发 §自决规则 R8 立刻 STOP

**理由**：避免跨进程 WAL 并发边角 case + 避免 schema 双 owner 漂移 + 与 ADR-004 local-first privacy baseline 一致。

### D2 — Rust 暴露 4 个新 gRPC service (Workspace/Job/Search/Events)

在 `core/proto/console_data_plane.proto` 新建 proto 文件（**不**复用 `proto/contextforge/v1/service.proto` 的 IndexService，避免与 Phase 9 cli-data-plane Index gRPC 字段冲突 + 与 Console contractv1 字段命名 1:1 对齐独立演进），新增 4 个 service：

| Service | RPC | 对应 Console REST endpoint |
|---|---|---|
| `WorkspaceService` | Create / Get / List / Delete | POST/GET/GET/DELETE /v1/workspaces[/:id] |
| `JobService` | Enqueue / Get / Cancel / Stream | POST/GET/POST /v1/index-jobs[/:id][/cancel] |
| `SearchService` | Query | POST /v1/search |
| `EventsService` | Subscribe (server stream) | GET /v1/observability/events (long-poll wrap) |

- 端口复用 ADR-013 cli-data-plane gRPC `:48180`（不引入新端口/auth 边界）
- 复用 task-9.* 已建立的 `tonic 0.12` + `prost 0.13` + `tonic-build 0.12` 工具链（Cargo.toml `tonic-build` 已注册；本 ADR 仅扩 `build.rs` 编译列表）
- proto 文件命名 `core/proto/console_data_plane.proto` 与现有 `proto/contextforge/v1/service.proto` 文件分离（Index gRPC 在 v0.2/v0.3 已 freeze；Console business plane 独立演进）
- gRPC method 与 Console REST endpoint 1:1 对应（同字段集合、同命名约定）

**理由**：避免引入额外端口/auth 边界；复用 task-9.* 已建立的 tonic + prost 工具链；与 Phase 9 Rust gRPC bridge 模式一致。

### D3 — Go console-api-serve = thin protocol translator

`internal/consoleapi/` handler 收到 Console REST 请求 → 调对应 gRPC method → 把 gRPC response 转 Console contractv1 JSON 返回。
- **禁止**在 Go 侧做业务逻辑（status 推进 / 字段补全 / 时间戳生成 / 校验）
- **禁止**在 Go 侧引入字段映射代码 —— `.proto` 字段命名必须与 Go contractv1 JSON tag 1:1 对应（snake_case 一致）；handler 直 `protojson.Unmarshal` 后 `json.Marshal` 同字段，**不**写 `func toProto(req Workspace) *proto.Workspace` 这种逐字段映射
- bearer auth 仍在 Go middleware 层（不下沉 gRPC）—— 认证是 REST 层职责，与 v0.3 一致
- 错误映射沿用 v0.3 sentinel：gRPC `NotFound` → `ErrNotFound` → HTTP 404；`FailedPrecondition` → `ErrJobAlreadyTerminal` → HTTP 409；`Internal` / `Unavailable` → `ErrCoreUnavailable` → HTTP 503

**理由**：保 D1 SoT + 让 Go 侧可被独立替换/版本化 + 减少跨语言 bug surface。

### D4 — in-memory MemStore 降级为 env-gated fallback

v0.3 task-10.4 引入的 `internal/consoleapi/memstore.go` MemStore + WorkspaceAdapter + JobAdapter **不删除**，但默认禁用：
- 默认行为：`console-api-serve` 启动时尝试连 `127.0.0.1:48180` gRPC；连不上 → `/v1/health` 返回 `status="degraded"` + `missing=["data_plane"]` + HTTP 503；所有业务 endpoint 返回 503
- env-gated fallback：仅当 `CONSOLE_API_FALLBACK_INMEM=1` 设置时启用 MemStore + log warning `"console-api: using in-memory fallback store (data plane unreachable)"` + health endpoint 返回 `degraded=true` + `store="inmem-fallback"`
- v0.3 集成测试可继续以 in-memory 模式跑（不依赖 Rust daemon spawn）—— 不破坏 v0.3 集成测试 fixture

**理由**：保 v0.3 集成测试可继续跑（不依赖 Rust daemon spawn）+ 给运维一个明确的 degraded 信号；不悄悄成默认 —— §自决规则 R8 enforce。

### D5 — schema 变更 single owner = Rust

所有 `core/migrations/00XX_*.sql` 由 Rust 团队 owner：
- Go 团队不创建 migration、不引入新 SQLite 表
- Go 侧任何持久化需求 → 走 gRPC method 加字段，由 Rust 实现存储
- v0.4 评估：**Phase 11 不新增 migration**（复用 task-10.2 `0010_workspaces.sql` + task-10.3 `0011_index_jobs.sql`）；JobRunner 真接索引时的 progress 字段已在 0011 内（processed_files / total_files / last_heartbeat_at）

**理由**：与 D1 一致；避免 schema 漂移。

### D6 — 复用 ADR-014 D1-D5 cross-validation gate

v0.4 不引入新 governance ADR；Phase 11 §6 5 AC × 4 task mapping 表 + D2 lint + D3 verified-by + D4 main-agent self-merge + D5 历史不动 全沿用 v0.3 模式。
- ADR-014 v0.3 首次激活已跑通；v0.4 第二次激活验证制度稳定性
- 引入新 governance ADR 会扩大 scope 超出 ship-first 选项

**理由**：v0.3 首次激活后已跑通；v0.4 第二次激活验证制度稳定性；引入新 governance ADR 会扩大 scope 超出 ship-first 选项。

## Rationale

- **不修改 Console contractv1.go**：Contract v1 字段集合 v0.3 锁定（ADR-015 D1 + D5 字段镜像约束沿用）；v0.4 仅补 ContextForge 端真业务面，不动协议
- **复用 ADR-013 gRPC 模式而非新建协议**：Phase 9 已建立 tonic + prost 工具链 + `:48180` 端口；新建独立协议会引入双 RPC 维护成本 + 双 auth 边界
- **Rust 持 SoT 而非 Go 直接读 SQLite**：跨进程 WAL 并发 / schema 双 owner / fork race 都是已知坑，绕开成本远低于 sub-process spawn 复杂度（daemon + console-api-serve 双进程已是 v0.3 既定形态）
- **MemStore 降级为 env-gated fallback 而非删除**：v0.3 集成测试 `internal/consoleapi/router_test.go` + `e2e_test.go` 已经依赖 MemStore；删除会破坏 v0.3 测试 fixture；env-gated fallback 是最小破坏路径
- **不引入新 governance ADR**：v0.4 ship-first；governance retrospective 留 Phase 11 closeout 后评估（与 ADR-014 v0.3 Follow-ups 同款）
- **proto 字段 snake_case 而非 camelCase**：Go contractv1 JSON tag 全用 snake_case (`workspace_id` / `processed_files` / `created_at_unix`)；proto 字段 snake_case 后 prost 自动生成 Rust struct 字段是 `workspace_id`（prost 默认转 snake_case），Go protojson 默认输出 snake_case → 三段一致，handler 不需要做字段重命名

## Alternatives

- **A. Go 直接打开 SQLite (modernc.org/sqlite + WAL)**：拒，破坏 D1 SoT + schema 双 owner + 跨进程 WAL 并发边角 case 多 + 引入新 R7 dep
- **B. Postgres 替代 SQLite**：拒，v0.4 过重 + 破坏 ADR-004 local-first；留 v1.0 multi-user 场景
- **C. 单 Go 进程统一所有逻辑（删除 contextforge-core daemon）**：拒，违反 ADR-001 双二进制架构 + Rust 数据面（tantivy / tree-sitter / rusqlite）无法搬到 Go；本 ADR 维持 ADR-001 边界
- **D. 引入 IPC stdin/stdout JSON-RPC 而非 gRPC**：拒，与 ADR-003 D3 既定 "内部 Go↔Rust 用 local gRPC" 冲突 + 长任务流式进度（IndexJob progress event）gRPC server stream 比 stdin/stdout JSON-RPC 干净
- **E. Console UI 直接调 Rust gRPC（删除 Go console-api-serve thin proxy）**：拒，破坏 ADR-001/003 边界 + Console v1.0 已 ship 的 HTTPAdapter 期望 REST + bearer auth 在 REST 层

## Consequences

**正面**：
- task-10.4 §10 trade-off #1 + #2 一次性 resolve —— Workspace 真持久化跨 daemon 重启 / IndexJob 真触发 Rust 索引 / Search 真返回 indexed 分块 / Events 真接 progress
- Rust 持 SoT，schema 单 owner，与 ADR-004 local-first + ADR-001 双二进制 + ADR-013 cli-data-plane 模式全部一致
- Console UI 端无任何改动；Contract v1 字段集合不动
- in-memory fallback 仍可用于 demo / 集成测试 / 离线运维 degraded 模式

**负面 / 成本**：
- 每次 REST 请求多 1 跳（HTTP → gRPC → SQLite）：预估 p95 < 5ms (in-memory benchmark) / < 20ms (rusqlite cold cache)；v0.4 不引入 perf benchmark gate，留 v0.4.x retrospective
- Deploy 必须双进程（contextforge-core daemon + console-api-serve）—— 与 v0.3 console_smoke.sh docker compose 模式相同，但 native deploy 需要 systemd / nssm 等服务管理
- gRPC method 必须与 Console REST endpoint 1:1 匹配（Console 加新 endpoint → Rust 加新 gRPC method）—— 同 D5 schema 边界，预期 v0.4.x Console endpoint expansion 时增量演进

**中性**：
- gRPC method 必须与 Console REST endpoint 1:1 匹配（Console 加新 endpoint → Rust 加新 gRPC method）
- v0.4 不引入新 R7 dep（tonic + prost + tonic-build 在 ADR-013 引入；本 ADR 仅扩 `.proto` 编译列表）

**对 v0.4+ 的影响**：
- v0.4.1 Console endpoint expansion (`/v1/memory*` / `/v1/eval-runs*` / `/v1/source-chunks/:id` / `/v1/search/:query_id/trace` / `/v1/workspaces/:id/config` PATCH)：每个新 endpoint 走 D2 模式（Rust 加 gRPC method + Go 加 thin handler）
- v0.5+ 多实例 daemon leader election / cluster：D1 SoT 在多实例语境下需要扩展（留 ADR-018 governance ADR）

## Rollback Or Migration Plan

如 Phase 11 实施中发现：

1. **D1 SoT 反向（Go 写 SQLite）出现**：立刻 STOP（§自决规则 R8）；revert 该 commit；从最近未违 D1 的 commit 重启该 task
2. **D2 4 service 字段命名与 Go contractv1 不齐**：surface diff 表 + 人工拍板（保 1:1 还是接受小 alias）；不允许在 handler 内加字段映射代码（破坏 D3）
3. **D3 thin proxy 被违反（handler 内出现业务逻辑）**：立刻 STOP（§自决规则 R8）；revert + 把业务逻辑下推到 Rust gRPC method
4. **D4 fallback 悄悄成默认**：grep `internal/consoleapi/router.go` 任何无 env check 即用 MemStore 的代码 → STOP；fallback 必须 env-gated
5. **D5 Go 团队尝试创建 migration**：立刻 STOP；migration 一律在 Rust 团队 / `core/migrations/` 下
6. **task-11.3 JobRunner ↔ IndexSession wiring 边界 case 不可解决**：保留 SQLite jobs schema + 把 JobRunner 真接 IndexSession 留 task-11.3.x amendment；in-memory fallback 模式下 v0.4 ship（playbook §不可逆动作清单 #1 评估）

Rollback 通过新 ADR superseding 完成；Phase 11 已 ship 的 gRPC service 保持向后兼容（add-only 演进，与 task-1.1 proto 同款规则）。

## Follow-ups

- **本 ADR Accepted in Phase 11 closeout PR**（task-11.4 完成后 + Phase 11 closeout PR 内）—— Proposed → Accepted 在 closeout commit 内回填
- **Phase 11 实施后**：v0.4.1 增量 endpoint task（memory / eval / source-chunks / search trace / workspace config PATCH 等）按 Console 实际 UI 优先级排
- **Cross-repo 治理 follow-up**：ADR-014 D5 的 cross-repo amendment 机制（Console 端字段变更 → ContextForge 镜像更新流程）在 v0.4 第二次激活后由 Phase 11 retrospective 评估是否制度化
- **关联 PRD §Open Questions O14**：本 ADR Accepted 后 O14 标记 `resolved by ADR-016 (business plane wiring); endpoint expansion 留 v0.4.x` —— v0.4 仅 partial resolve，O14 保 unchecked
- **关联 ADR-014 D2 lint**：Phase 11 全程 spec PR 跑 `bash scripts/spec_drift_lint.sh --touched origin/master` + closeout PR 含 D1 mapping 表 + D2 输出（playbook §自决规则 #9 / #10）
