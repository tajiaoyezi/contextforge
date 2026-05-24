# ContextForge Release Notes

## v0.6.0 (2026-05-24)

### 摘要

ContextForge v0.6.0 完成 **Phase 13 memory-rest-surface** 收口：ADR-017 D1
Wave 3 共 5 个 memory REST endpoint 落地，把 Console HTTPAdapter conformance
从 13/22 提升到 18/22（82% coverage）。新增 SQLite 表 + `MemoryService` 5 gRPC
RPC + 4 个 AuditOperation 变体 + Go REST 5 handler。ADR-014 cross-validation
gate **第四次完整激活** 跨 4 phase 验证制度稳定性。

### 主要改进

- **task-13.1 Rust SoT** (PR #84):
  - `core/migrations/0013_memory_items.sql` (10 columns + 3 indexes + status CHECK constraint)
  - `core/src/memory/store.rs` `SqliteMemoryStore` (5 methods + 9 unit tests)
  - `proto/contextforge/console_data_plane/v1/console_data_plane.proto` add-only
    `MemoryItem` + 5 request/response messages + `MemoryService` 5 RPC
  - `core/src/data_plane/memory.rs` `MemoryServer` impl (5 RPC + 5 unit tests)
  - `core/src/memoryops/audit.rs` `AuditOperation` 加 4 variants
    (MemoryPin / MemoryUnpin / MemoryDeprecate / MemorySoftDelete)
  - Pin / Deprecate / SoftDelete 各 emit 一条 audit event
  - `core/src/data_plane/mod.rs` `DataPlaneStores` 加 Option<memory> + Option<audit>;
    新 `with_memory()` + `full()` 构造函数; `register_services` 加 5th MemoryServiceServer
  - `core/src/server.rs` `serve_full` 实例化 SqliteMemoryStore + AuditSink 真接到 daemon
  - 3 integration tests via tonic client + MemoryServiceClient
- **task-13.2 Go REST** (PR #85):
  - `internal/consoleapi/types.go` `MemoryClient` interface + `MemoryListFilter` + `Deps.Memory`
  - `internal/consoleapi/router.go` 5 new routes; deprecate + soft-delete
    confirmMiddleware-gated (ADR-017 D2 OR-semantics)
  - `internal/consoleapi/handlers.go` 5 new handlers (Pin/Deprecate/SoftDelete
    each return 204 No Content); `deps.Memory == nil → 503` graceful degrade
  - `internal/consoleapi/memstore.go` `MemMemoryStore` + `SeedFixtures()` (5 hard-coded)
    for `CONSOLE_API_FALLBACK_INMEM=1` mode
  - `internal/consoleapi/grpcclient/grpcclient.go` `memoryClient` 5 wrappers +
    `protoToMemoryItem` helper; `Client.Memory()` accessor
  - `internal/cli/console_api_serve.go` `buildDeps` wires Memory in both modes;
    `degradedDeps()` adds `degradedMemory{}`
  - 7 new router_test + e2e_grpc Step 9d (real Rust daemon 404/412 invariants)
- **scripts/console_smoke.sh v4** (PR #85):
  - Header v3 → v4; subtitle "Phase 13 memory-rest-surface"
  - 13 → 18 endpoint flow; renumber [1/18]..[18/18]
  - 新 Step 13/18: sqlite3 seed (gracefully skips if sqlite3 unavailable)
  - 新 Step 14-18/18: memory list / get / pin 204 / deprecate 412+204 / soft-delete 412+204
  - REAL mode: `CONSOLE_REAL_SMOKE_EXIT=0` 18/18 PASS
- **test/fixtures/memory-seed/seed.sql** (新增): 5 rows + agent_scope 分布
- **治理 / spec 同步** (PR #86):
  - Phase 13 spec / adapter §Phase 13 / task-13.{1,2} 全 `Status: Done`
  - ADR-017 Status: Proposed (full Accepted 推到 Phase 14 closeout 一次性)
  - ADR-014 D1 mapping 表 / D2 lint 0 violation

### Trade-offs / Conscious limitations

- **is_pinned 列设计**：选 `is_pinned bool` 列 + `status` 三态独立；pin state
  存在 Rust SqliteMemoryStore 但**不在 contractv1.MemoryItem 暴露** (ADR-015 D5
  字段锁定)；Console UI 显示 Pin 按钮但 pinned visual indicator 需通过
  future contractv1 amendment 或 inferred via 单独 Get-by-id 调用
- **importer 写入 memory_items 路径** `[SPEC-DEFER:phase-15.import-to-memory-items]`
  留 v0.6.x；v0.6.0 ship 后 Console UI 看 0 条 memory items（fresh install）→
  Console UI 端 graceful degrade
- **memory hard delete** 不实施（Console PRD 显式只支持 soft-delete）
- **POST /unpin separate endpoint** 不实施（Console v1.0 contract 只有 `/pin`；
  `Pin(id, false)` API 端已支持 unpin 语义；如 Console 需要 separate route →
  cross-repo amendment `[SPEC-DEFER:console-memory-unpin]`)

### Migration notes (v0.5.0 → v0.6.0)

- **daemon 重启后 memory_items 表自动创建**（schema migration 0013_memory_items.sql
  在 SqliteMemoryStore.open 内 execute_batch IF NOT EXISTS）；v0.5 用户重启
  daemon 后 `<data_dir>/memory.db` 自动 ready
- **新 5 endpoint**（Memory CRUD + Pin/Deprecate/SoftDelete）— 无 v0.5 baseline;
  client 按 OpenAPI/contractv1 v1 spec 调用
- **destructive endpoints** (deprecate + soft-delete) 需要 X-Confirm: yes header
  或 ?confirm=true query；Console BFF 自动注入；ops curl 用户须显式加
- contractv1.go 字段集合不变 (ADR-015 D5)
- 新 proto RPC + message add-only (ADR-013 D2)

### Tests (Phase 13 全程)

- **Rust**: 84 lib tests (含 14 new memory: 9 store + 5 server) + 3 memory_integration
  + 既有 phase 1-12 测试不退化 = 17 test groups all PASS
- **Go**: 43 packages PASS (含 7 new memory router_test + e2e_grpc Step 9d
  real Rust daemon + grpcclient_test 不退化)
- **conformance**: v0.4/v0.5 既有 endpoints 不退化
- **smoke**: `bash scripts/console_smoke.sh` REAL mode 18/18 PASS

### Verification commands

```bash
cargo test -p contextforge-core   # expect all PASS (17 test groups)
go test ./...                     # expect 43 packages PASS
bash scripts/console_smoke.sh     # expects CONSOLE_REAL_SMOKE_EXIT=0
RELEASE_SMOKE_CONSOLE=1 bash scripts/release_smoke.sh   # PHASE_RELEASE_SMOKE_EXIT=0
```

---

## v0.5.0 (2026-05-24)

### 摘要

ContextForge v0.5.0 完成 **Phase 12 console-contract-completion** 收口：把
ADR-017 D1 Wave 1（quick win 4 个 endpoint）+ Wave 2（mid scope 2 个 endpoint）
共 5 个新 endpoint + 1 个 behavior 切换（cancel 200→204）一次性 ship，把 Console
HTTPAdapter conformance 从 9/22 提升到 13/22（route inventory 9→14 含 PATCH
config）。ADR-014 cross-validation gate **第三次完整激活** 验证制度稳定性。

### 主要改进

- **task-12.1 Wave 1 quick win** (PR #78):
  - `PATCH /v1/workspaces/{id}/config` 走 gRPC `WorkspaceService.UpdateConfig`
    (proto add-only `UpdateWorkspaceConfigRequest`)；body `{allowlist, denylist}`
    覆盖式更新；SqliteWorkspaceStore.update_config 真持久化 + updated_at_unix 推进
  - `GET /v1/index-jobs?status=active` 走 gRPC `JobService.List` + status_filter
    (proto add-only `ListJobsRequest{status_filter, workspace_id}` + `ListJobsResponse`)；
    Rust 端 `list_active()` 包装 + Go 端 missing-filter → 400
  - `POST /v1/index-jobs/{id}/cancel` 返 **204 No Content** (ADR-017 D3)
  - `confirmMiddleware` 服务端 X-Confirm 兜底 (ADR-017 D2): 破坏性 endpoint
    必须 `X-Confirm: yes` header **或** `?confirm=true` query (OR-semantics);
    缺失 → 412 PRECONDITION_FAILED + ErrorBody `{code:"PRECONDITION_FAILED",...}`
- **task-12.2 source-chunk-by-id** (PR #79):
  - `GET /v1/source-chunks/{id}` 走 gRPC `SearchService.GetSourceChunk` (proto
    add-only `GetSourceChunkRequest{chunk_id, workspace_id(optional)}`)
  - Rust impl 复用既存 `Retriever::get_chunk(chunk_id)` (task-6.2 ship 的 SQL
    fast-path)；workspace_id 缺失时枚举 SqliteWorkspaceStore.list() 真试每个
    workspace 寻 chunk (chunk_id 全局唯一 SqliteChunkStore 假设
    `[SPEC-DEFER:phase-15.multi-workspace-strict]`)
  - chunk_offset_start/end = 0 占位 `[SPEC-DEFER:chunk-byte-offsets]` (current
    schema 不存 byte offsets; Console UI 用 line_start/end)
- **task-12.3 search-trace-by-query-id** (PR #80):
  - `GET /v1/search/{query_id}/trace` 走 gRPC `SearchService.GetSearchTrace`
    (proto add-only `GetSearchTraceRequest{query_id}`)
  - 自研 `TraceStore { HashMap, VecDeque, cap=1000 }` ~30 行 LRU/FIFO eviction
    (避免 `lru` crate R7 风险)；`std::sync::Mutex` 包裹 read-heavy 场景足够
  - `SearchService.Query` 内统一生成 `qry-{nanos}` 唯一 query_id 字段
    (task-11.4 既存返 empty query_id 字段被替换)；每次 Query 自动 put trace
    到 trace_store
- **scripts/console_smoke.sh v3** (PR #80):
  - Header bump v2 → v3；subtitle "Phase 12 console-contract-completion"
  - 9 → 13 endpoint flow；renumber [1/13]..[13/13]
  - 新 Step 9/13: task-12.1 PATCH workspace/config (412→200×2)
  - 新 Step 10/13: task-12.1 GET active jobs + missing-status 400
  - 新 Step 11/13: task-12.2 GET source-chunks/{id} (uses chunk_id from search)
  - 新 Step 12/13: task-12.3 GET search/{query_id}/trace + unknown 404
  - REAL mode 真接 daemon: `CONSOLE_REAL_SMOKE_EXIT=0` 13/13 PASS
- **治理 / spec 同步** (PR #81):
  - Phase 12 spec / adapter §Phase 12 / task-12.{1,2,3} 全 `Status: Done`
  - ADR-017 Status: Proposed (full Accepted 推到 Phase 14 closeout 一次性)
  - ADR-014 D1 mapping 表 / D2 lint 0 violation / D3 verified-by 显式

### Trade-offs / Conscious limitations

- **task-12.2 §10**: chunk_offset_start/end = 0 占位
  `[SPEC-DEFER:chunk-byte-offsets]` — current SqliteChunkStore schema 不存
  byte offsets; Console UI 用 line_start/end 显示足够；future schema migration
  填充字节偏移留 v0.5.x
- **task-12.2 §10**: workspace_id 全局唯一假设
  `[SPEC-DEFER:phase-15.multi-workspace-strict]` — multi-workspace strict
  isolation 留 v1.x
- **task-12.3 §10**: trace_store 重启即丢 `[SPEC-DEFER:task-future.search-trace-sqlite-persistence]`
  — SQLite 持久化跨 daemon 重启留 v0.5.x；Console UI 端 graceful degrade 承接
- **task-12.3 §10**: trace_store cap=1000 硬编码 — env var 参数化留 v0.5.x

### Migration notes (v0.4.0 → v0.5.0)

- **`POST /v1/index-jobs/{id}/cancel` 改 204 No Content** — Console HTTPAdapter
  v1.0 已 200/204 双 check (cross-repo 验证)，应不出现 break；如发现 strict
  200 only 的旧 client → rollback path 是把 handlers.go handleCancelJob 回退
  到 `StatusOK`
- **PATCH /v1/workspaces/{id}/config + 新破坏性 endpoint** 现在强制
  X-Confirm/?confirm=true — Console BFF 自动注入；ops curl 用户须显式加
- **新 4 endpoint (PATCH config + active filter + source-chunks + trace)**
  无 v0.4 baseline; client 端按 OpenAPI/contractv1 v1 spec 调用
- contractv1.go 字段集合不变 (ADR-015 D5 字段镜像约束沿用)
- 新 RPC 全 proto add-only (ADR-013 D2)，既有 RPC 字段编号不动

### Tests (Phase 12 全程)

- **Rust**: 70 lib tests (含 4 new task-12.1 workspace UpdateConfig/job List + 3
  new task-12.2 GetSourceChunk + 4 new task-12.3 GetSearchTrace+TraceStore +
  既有 phase 1-11 测试不退化)
- **Go**: 43 packages PASS (含 task-12.1 7 new router_test + 4 new grpcclient_test
  + task-12.2 2 new + task-12.3 1 new + degraded fallback impls + e2e_grpc with
  real Rust daemon Step 8a/8b/9/9b/9c PASS)
- **conformance**: `test/conformance/console_contractv1_test.go` v0.4 9 endpoint
  不退化
- **smoke**: `bash scripts/console_smoke.sh` REAL mode 13/13 endpoint PASS
  with `CONSOLE_REAL_SMOKE_EXIT=0` final marker

### Verification commands

```bash
# Rust workspace
cargo test -p contextforge-core --lib   # expect 70/70 PASS

# Go full
go test ./...   # expect 43 packages PASS

# Phase 12 console real smoke v3 (default REAL mode)
bash scripts/console_smoke.sh   # expects CONSOLE_REAL_SMOKE_EXIT=0

# Release smoke (§5 enables console smoke via env)
RELEASE_SMOKE_CONSOLE=1 bash scripts/release_smoke.sh   # PHASE_RELEASE_SMOKE_EXIT=0

# ADR-014 D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master   # 0 violation
```

---

## v0.4.0 (2026-05-25)

### 摘要

ContextForge v0.4.0 完成 **Phase 11 console-real-data-plane** 收口：把 Phase 10
task-10.4 §10 显式记录的两个 Trade-off (`[SPEC-DEFER:task-future.cross-process-
sqlite-sharing]` 与 JobRunner 不真索引) 一次性 resolve。通过新 ADR-016
**cross-process-rust-go-via-grpc-bridge** 实施 4 个新 Rust gRPC service
(Workspace / Job / Search / Events)，Go console-api-serve 重构为 **thin REST→gRPC
translator**；console UI 期望的 Workspace 持久化跨 daemon 重启 + IndexJob 真触发
Rust 索引 + Search 真返回 indexed chunks + Events 真接 JobRunner progress 全部
端到端落地。ADR-014 cross-validation gate **第二次完整激活** 验制度稳定性。

### 主要改进

- **ADR-016 cross-process Rust ↔ Go gRPC bridge** (Proposed → Accepted): 6 个 D
  条款落地。D1 Rust 持 SoT (Go 不写 SQLite); D2 4 gRPC service in
  `proto/contextforge/console_data_plane/v1/console_data_plane.proto` (snake_case
  1:1 镜像 Go contractv1 JSON tag); D3 Go console-api-serve thin proxy
  (`internal/consoleapi/grpcclient/`); D4 in-memory MemStore 降级为 env-gated
  fallback (`CONSOLE_API_FALLBACK_INMEM=1`); D5 schema 单 owner = Rust; D6 沿用
  ADR-014 cross-validation gate.
- **Rust data plane gRPC services** (`core/src/data_plane/`): 4 tonic service
  trait impls (`WorkspaceServer` / `JobServer` / `SearchServer` / `EventsServer`)
  + `register_services` helper + `serve_full(addr, svc, data_dir)` 把 Phase 9
  ContextService + Phase 11 4 service 注册到同一 tonic Server.
- **Real JobRunner wiring** (task-11.3): `IndexSessionBackend` impl
  `IndexerBackend` 包 `IndexSession::index_path_cancellable` (add-only API
  extension; cancel_token at file boundaries); `JobService.Enqueue` 真
  `tokio::spawn(JobRunner.run_one)`; `orphan_reaper` 在 `serve_full` 启动早期
  清理上一 boot 留下的 running 行 (mark failed + error_message="job lost: daemon
  restart"); JobRunner.run_one 改 per-file cancel-check (heartbeat 仍 throttled
  100files/5s) 让小 fixture 也能在 5s 内观察 cancel.
- **Real SearchService + EventBus** (task-11.4): `SearchService.Query` 真接
  `core/src/retriever/Retriever::search` (Tantivy + SQLite chunks);
  `RetrievalTrace.retrieved_chunks` 真填 (chunk_id + score + source_file +
  `chunk_text_preview` ≤200 chars via `utf8_safe_truncate` UTF-8 boundary safe);
  `EventBus` (broadcast::Sender 容量 1000) 接 `EventsService.Subscribe` server
  stream; `JobRunner` progress callback emit `indexing.progress` /
  `indexing.cancelled` / `indexing.error` events.
- **Go grpcclient** (`internal/consoleapi/grpcclient/`): `Client.Workspace/Job/
  Search/Events()` 4 wrapper impl `consoleapi.{Workspace,Job,Search,Events}Client`;
  `mapGrpcErr` maps gRPC status → consoleapi sentinel (NotFound → ErrNotFound /
  FailedPrecondition → ErrJobTerminal / Unavailable → ErrDataPlaneUnavailable).
- **console-api-serve 新 flags**: `--grpc-addr 127.0.0.1:50551` (default; alias
  to Rust DEFAULT_LISTEN) + `--fallback-inmem` (alias env
  `CONSOLE_API_FALLBACK_INMEM=1`). `BackendKind`-aware `/v1/health`: grpc → 200
  healthy; inmem-fallback → 200 degraded + ErrorReason; degraded → 503 + missing=
  ["data_plane"].
- **Long-poll wait/limit** (`/v1/observability/events`): `?wait=<duration>`
  (default 30s, clamped [1s, 60s]) + `?limit=<int>` (default 100, clamped [1, 500])
  query params; grpcclient.eventsClient.Recent uses ctx 30s timeout to drive
  long-poll behaviour at the gRPC layer.
- **scripts/console_smoke.sh v2** (REAL mode default): spawns both contextforge-
  core daemon and console-api-serve, drives the 9 endpoint flow + real index
  job against `test/fixtures/index-job-real/` (5 markdown files). Final marker:
  `CONSOLE_REAL_SMOKE_EXIT=0`. v0.3 inmem mode retained as `LOCAL_ONLY=1`.
- **release_smoke.sh §5 updated** for REAL mode; final
  `phase11_console_real=ok` marker.
- **ADR-014 D1-D5 second activation pass**: D1 mapping (in closeout PR body);
  D2 lint `bash scripts/spec_drift_lint.sh --touched <base>` 0 violation (with
  proper [SPEC-OWNER]/[SPEC-DEFER] tags throughout phase-11 + 4 task spec);
  D3 each phase §6 AC verified by explicit owner; D4 main-agent self-merge
  via /goal autonomy; D5 historical Phase 1-10 unchanged.
- **治理 / spec 同步**: ADR-016 Proposed → Accepted; Phase 11 / Task 11.1-11.4
  全 Done; PRD §Implementation Phases Phase 11 + §Open Questions O14 partially
  resolved by ADR-016 (business plane wiring; endpoint expansion [SPEC-DEFER:
  console-endpoint-expansion]); adapter §Phase / §Tasks / §ADRs / §BDD synced.

### Trade-offs / Conscious limitations

- **task-11.2 §10 T2** `--grpc-addr` default `127.0.0.1:50551` (与 Rust
  `DEFAULT_LISTEN` 对齐); playbook 文档曾写 `:48180` 是 ADR-013 概念预留, 实施
  按 Rust 既有 default 落地 (无 spec drift — gRPC 字段集合才是契约, 端口可配).
- **task-11.3 §10 T1** cancel co-operative only (file-boundary granularity);
  hard kill cancel [SPEC-DEFER:task-future.hard-cancel].
- **task-11.4 §10 T1** EventBus volatile broadcast (daemon 重启即丢历史
  events); persistent event ring buffer [SPEC-DEFER:task-future.event-persistence].
- **task-11.2 §10 T1** v0.3 in-memory MemStore retained as env-gated fallback
  (not deleted) for conformance test backward compat + degraded mode demo.
- Multi-instance daemon leader election [SPEC-DEFER:task-future.multi-daemon-leader-election].

### Migration notes (v0.3.0 → v0.4.0)

- `console-api-serve` 默认 backend 从 in-memory MemStore 切到 gRPC. v0.3 用户
  若需 inmem 行为, 设 `CONSOLE_API_FALLBACK_INMEM=1` (CLI flag `--fallback-inmem`).
- v0.3 console_smoke.sh 默认 local mode → v0.4 默认 REAL mode (需 cargo build
  Rust binary). 兼容 v0.3 行为: `LOCAL_ONLY=1 bash scripts/console_smoke.sh`.
- Console contract v1 字段集合不变 (ADR-015 D5 字段镜像约束沿用); Console UI
  端无任何改动 — v0.4 仅 ContextForge 单仓内业务面真接通.
- 新 deploy 形态: `contextforge-core <listen> <data_dir> &` 后 `contextforge
  console-api-serve --addr ... --grpc-addr ...`. 双进程 deploy 可用 systemd /
  docker compose / 脚本管理.

### Tests (Phase 11 全程)

- Rust: 60 lib + 5 indexjob_real_runner + 4 search_real_retriever + 5
  data_plane_integration + 既有 phase 1-10 测试不退化.
- Go: 9 grpcclient + 6 cli + 1 e2e gRPC backed E2E (TestRESTEndpoints_E2E_
  GrpcBacked spawns Rust daemon + 9 endpoint flow + workspace 持久化跨 daemon
  restart) + 既有 consoleapi v0.3 + conformance test 不退化.

### Verification commands

```bash
# Rust full workspace
cargo test --workspace

# Go full
go test ./...

# Phase 11 console real smoke (default REAL mode)
bash scripts/console_smoke.sh   # expects CONSOLE_REAL_SMOKE_EXIT=0

# Release smoke (§5 enables console smoke via env)
RELEASE_SMOKE_CONSOLE=1 bash scripts/release_smoke.sh   # PHASE_RELEASE_SMOKE_EXIT=0

# ADR-014 D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master   # 0 violation
```

---

## v0.3.0 (2026-05-24)

### 摘要

ContextForge v0.3.0 完成 **Phase 10 console-contract-v1** 收口：实现 ContextForge ↔
**ContextForge-Console** v1.0 (已 ship) **Contract v1 兼容层** —— 17 个 Go 类型
1:1 镜像 Console `contractv1.go` + Rust workspace/jobs 资源模型 + 9 个对齐 Console
HTTPAdapter 期望的 REST 端点 + cross-repo conformance test + docker compose 集成
smoke。同时 ADR-014 cross-validation gate (D1 mapping / D2 lint / D3 verified-by /
D4 自治补丁 / D5 历史不溯改) 首次完整激活。

### 主要改进

- **internal/contractv1/ Go 类型镜像**：1:1 复刻 Console
  `console-api/internal/coreadapter/contractv1/contractv1.go` 17 个类型 +
  `ContractVersion = "v1"` 常量 + `FieldAvailability` helper；env
  `CONSOLE_REPO=$path` 设时 reflect 反射跑 Console parity 校验。
- **Rust Workspace + IndexJob 资源**：`core/src/workspace/` (CRUD + 1:1
  collection 映射) + `core/src/jobs/` (异步 lifecycle queued/running/
  succeeded/failed/cancelled + heartbeat + co-operative cancel) +
  SQLite migration `0010_workspaces.sql` + `0011_index_jobs.sql`。
- **9 Console Contract v1 REST endpoint** (新增 `internal/consoleapi/`)：
  `GET /v1/health` + `POST/GET/GET /v1/workspaces*` +
  `POST/GET/POST /v1/index-jobs*[/cancel]` + `POST /v1/search` (nested
  `{result, trace}`) + `GET /v1/observability/events` (long-poll, 非 SSE)；
  路径 / shape / 错误码 严格对齐 Console HTTPAdapter；bearer auth +
  OpenAPI 3.0 yaml (`docs/consoleapi/openapi.yaml`)。
- **新 CLI 子命令** `contextforge console-api-serve --addr ...` 启动
  consoleapi router (in-memory MemStore v0.3；cross-process SQLite 共享留
  v0.4 task-future)。
- **Cross-repo conformance test** (`test/conformance/`)：env-based skip
  机制 + Console-style 9 endpoint flow + FieldAvailability.Complete() +
  Console sentinel error mapping (404→ErrNotFound / 409→ErrConflict)。
- **Docker compose stack**：`deploy/console-stack.yml` 含 5 service
  (postgres + redis + contextforge + console-api + console-web)；profile
  `console` gates the optional Console UI services。
- **多阶段 `Dockerfile`**：rust:1.82 + golang:1.22 → debian:bookworm-slim，
  CMD `contextforge console-api-serve --addr 0.0.0.0:48181`。
- **新 smoke**：`scripts/console_smoke.sh` 默认本地 mode (build + spawn
  + 9 endpoint curl); env DOCKER_SMOKE=1 触发 docker compose 模式。
- **release_smoke.sh 第 5 段**：env `RELEASE_SMOKE_CONSOLE=1` 启用 (默认 SKIP
  避 CI 强依赖 docker)。
- **ADR-014 cross-validation gate 全程激活**：D2 lint `scripts/spec_drift_lint.sh
  --touched origin/master` 0 violation；D3 每条 phase §6 AC + task §6 AC 含
  `verified by ...` 显式 owner；D1 closeout PR body mapping 表。
- **治理 / spec 同步**：ADR-015 Proposed → Accepted；Phase 10 / Task
  10.1-10.6 全 Done；PRD §Implementation Phases Phase 10 + §Open Questions
  O12 (Resolved by ADR-014) + O13 (新增 Console 集成)；adapter §Phase /
  Task / ADR / BDD 索引同步。

### v0.3 trade-offs (§Implementation Notes)

- **Cross-process SQLite 共享 Rust ↔ Go (task-10.4 §10 #1)**：v0.3 Go 端 REST
  用 in-memory MemStore；Rust 端 workspace/jobs 用 SQLite。两者各自独立，
  Console UI POST 创建的 workspace 不进 Rust JobRunner。**Why**：保守
  优先级 backward compat > spec literal > minimal change；避新增 sqlite Go
  driver (mattn/go-sqlite3 CGO 或 modernc/sqlite 纯 Go) — playbook v0.3 不
  预期新 dep。**v0.4 follow-up**：[SPEC-DEFER:task-future.cross-process-sqlite-sharing]。
- **时间字段 Unix epoch i64 (workspace/jobs)**：避新增 chrono dep；Go REST
  序列化时 `time.Unix(sec, 0).UTC()` 转 RFC3339 喂 Console wire。
- **Console UI integration smoke 在 docker compose 默认 SKIP**：Console v1.0
  docker image 公网未发布；console_smoke.sh 默认 local mode (ContextForge
  daemon only)；DOCKER_SMOKE=1 + CONSOLE_API_IMAGE / CONSOLE_WEB_IMAGE 三
  env 同时设才跑 full Console UI 集成。

### 限制（继承 v0.1 + v0.2 + Phase 10 新增）

- v0.3 Console 集成是 spec/REST 契约层 conformance；Console UI 真返回
  workspace 列表（非 Mock）已通过 console_smoke.sh 在 ContextForge daemon
  端验证。**Console docker image 公网拉取 + UI 真渲染**留 v0.4 (依赖 Console
  仓库发布 image)。
- v0.3 in-memory MemStore 不持久化 — daemon 重启后数据丢失。Cross-process
  SQLite 共享 / 持久化 IndexJob 留 v0.4。
- 其它 10+ Console endpoint (`/v1/memory*` / `/v1/eval-runs*` /
  `/v1/source-chunks/:id` / `/v1/search/:query_id/trace` /
  `/v1/workspaces/:id/config` PATCH) — Console Mock Adapter 覆盖到 v0.4。

### Migration notes (from v0.2.0)

- `internal/cli` 新增 `console-api-serve` 子命令 — 现有子命令行为不变。
- `internal/daemon/rest.go` v0.2 既有 5 endpoint (`/v1/search`, `/v1/chunks/{id}`,
  `/v1/collections`, `/v1/import`, `/v1/eval/run`) 不变；Console Contract v1
  9 endpoint 在独立 `internal/consoleapi/` 包内，通过 `console-api-serve` 子
  命令暴露 (不与 `serve` 子命令的 daemon REST 冲突)。
- `scripts/release_smoke.sh` 增第 5 段 (env RELEASE_SMOKE_CONSOLE=1 启用)；
  `PHASE_RELEASE_SMOKE_EXIT` 退出码兼容 v0.2。

---

## v0.2.0 (2026-05-24)

### 摘要

ContextForge v0.2.0 完成 Phase 9 cli-pipeline 收口：补齐 v0.1 ship 后实测的
CLI 数据通路 spec drift —— `contextforge index` / `contextforge import` 在
v0.1 是 stub，v0.2 通过 ADR-013 add-only 扩 `rpc Index` server-stream 真接通
Go↔Rust gRPC + 真扫描 + 真写 SQLite/Tantivy。README Quick Start 现可复制粘贴
跑通。

### 主要改进

- **CLI 数据通路打通**：`proto/contextforge/v1/service.proto` 新增 `rpc Index(IndexRequest) returns (stream IndexProgress)`；Rust `CoreService::index`
  wire `IndexSession::index_path_with_progress` 按文件粒度上报进度；Go
  `Daemon.Index` + `internal/cli/index.go` 真实 stream consume + human/JSONL render。
- **`contextforge import` 三子命令真实**：hermes / openclaw / agent-rules 现产
  YAML-frontmatter Markdown 到 `<data-dir>/imports/<source>/`；`contextforge index --source <output_dir>` 把它灌入。
- **README Quick Start 可复制粘贴**：新增 `examples/quickstart/` fixture +
  `scripts/quickstart_smoke.sh` 一键 7 步端到端；README 重写 manual steps + 注释 flag 顺序陷阱。
- **Release smoke 真端到端**：删除 `internal/release/release_test.go` 三个
  fake-evidence 测试（`TestTask83_AC2/AC4/AC5`），重写 `TestTask83_AC1` 用真
  `go build` + `cargo build`，新增 `TestPhase9ReleaseSmoke_EndToEnd` 7-step
  CLI binary 真跑；`scripts/release_smoke.sh` 加 phase 9 段 + 重命名
  `PHASE_RELEASE_SMOKE_EXIT`（去 v0.1-only PHASE8 前缀）。
- **治理 / spec 同步**：ADR-013 Proposed → Accepted；Phase 9 / Task 9.1-9.6 全
  Done；PRD §Implementation Phases Phase 9 + §Open Questions O12 同步；
  adapter §Phase 状态索引 / Task 索引 / ADR 索引 / BDD 索引同步。

### 验证证据

最终 `master` 上执行：

```bash
bash -lc 'source docs/s2v/scripts/lib/preflight.sh; source docs/s2v/scripts/lib/verify.sh; s2v_baseline_green "cmd/contextforge internal core/src core/tests"'
```

结果：`FINAL_HEAD_BASELINE_EXIT=0`。

```bash
bash scripts/release_smoke.sh
```

结果：`PHASE_RELEASE_SMOKE_EXIT=0`（4 段：go release harness / task-8 reliability/eval / Rust gRPC search smoke / phase 9 CLI e2e）。

```bash
bash scripts/quickstart_smoke.sh
```

结果：`QUICKSTART_SMOKE_EXIT=0`（7 步：build / init / import hermes / index records / index source / search / eval）。

完整证据见 [`docs/releases/v0.2.0-evidence.md`](docs/releases/v0.2.0-evidence.md)；产物清单见 [`docs/releases/v0.2.0-artifacts.md`](docs/releases/v0.2.0-artifacts.md)。

### 发布边界

- 继承 v0.1 限制：Linux x86_64 / WSL2 官方目标；macOS 应能跑（bash + cargo + go）；Windows 走 Git Bash / WSL；macOS / Windows 官方 tarball 仍延后。
- `LICENSE` 继续 all-rights-reserved（占位于明确 OSI 许可证前）。
- 真实 GitHub Release 上传、checksum / signing、CI release job 仍需外部发布流水线执行。

### v0.1.0 → v0.2.0 迁移

无 schema 变更（schema_version 仍 `0.1`，proto add-only `rpc Index` 不破坏现有 wire 兼容）。脚本端：`PHASE8_RELEASE_SMOKE_EXIT` 重命名为 `PHASE_RELEASE_SMOKE_EXIT` — 任何依赖此标记的外部 CI 步骤需相应更新。

---

## v0.1.0 (2026-05-23)

### 摘要

ContextForge v0.1.0 完成本地优先的双二进制基础闭环：Go 控制面 `contextforge` + Rust 数据面 `contextforge-core`，覆盖初始化、索引核心、检索解释、REST/MCP/export、recall eval、可靠性 guard 与 release smoke gate。

### 主要能力

- S2V 治理：ADR-012 放宽主 agent 自治决策，同时保留 R3 分支校验、R6 PR-only、worktree 隔离和合入 gate。
- Eval：`contextforge eval run` 具备 30 条内置 golden questions、Top-5/Top-10 strong hit rate、miss cases 与 latency p95 输出。
- Reliability：长任务 resume manifest、资源预算 gate、secret/export/audit safety regression guard。
- Release：新增 `internal/release` tarball contract、七步 smoke evidence、10 万 chunk P95 benchmark gate，以及 `scripts/release_smoke.sh` Phase 8 smoke 入口。
- Distribution docs：新增 `README.md`、`LICENSE`、`contextforge.example.toml` 和 ADR-007 产物清单。

### 验证

最终 `master` 上通过：

```bash
bash -lc 'source docs/s2v/scripts/lib/preflight.sh; source docs/s2v/scripts/lib/verify.sh; s2v_baseline_green "cmd/contextforge internal core/src core/tests"'
```

结果：`FINAL_HEAD_BASELINE_EXIT=0`。

最终 `master` 上通过：

```bash
bash scripts/release_smoke.sh
```

结果：`PHASE8_RELEASE_SMOKE_EXIT=0`（v0.1 版本；v0.2 已重命名为 PHASE_RELEASE_SMOKE_EXIT）。

完整证据见 `docs/releases/v0.1-evidence.md`。

### 发布边界

- 本 tag 提供 release contract gate 与产物清单；真实 GitHub Release 上传、checksum/signing 与 CI release job 仍需在发布流水线中执行。
- v0.1 官方目标平台为 Linux x86_64 / WSL2；macOS / Windows 官方 tarball 延后。
- `LICENSE` 当前为 all-rights-reserved，占位于明确开源许可证之前。
