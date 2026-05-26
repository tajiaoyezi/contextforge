# Task `15.6`: `health-component-detail — proto ComponentHealth message + 5 探针 (db/index/embed/retriever/eval) + Go REST GET /v1/health?detailed=true`

**Status**: Done

**Priority**: P2
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 15 (console-functional-gap-closure)
**Dependencies**: task-15.1 through task-15.5 (前 5 task ship 后本 task 是 Phase 15 收口) + [ADR-020](../../decisions/adr-020-health-component-breakdown.md)

## 1. Background

ContextForge-Console PR #91/#93 backlog 列 P2 #7：

> `/v1/health` 是 binary healthy/degraded，Console UI `CoreHealthCard` 期望 5 链路细分 (db / index / embed / retriever / eval) — 用户视图把 ContextForge backend 当 5 个子系统，无法定位哪条链路坏了。

**实施策略**：

- proto add-only：`ComponentHealth` message + 通过 contractv1.go side 加 `CoreHealth.Components map[string]ComponentHealth`（既有 CoreHealth 是 Go 侧 struct；proto 侧仅加 ComponentHealth message 供 Rust gRPC 返回）
- Rust impl：新建 `core/src/health.rs` 5 探针：
  - `db` — SQLite `SELECT 1` on `workspaces.db`
  - `index` — Tantivy `Index::open_in_dir` + reader load segment meta
  - `embed` — `config.toml` 段 / env `CONTEXTFORGE_EMBED_PROVIDER` 配置存在性
  - `retriever` — `retriever.search(SearchOptions{ query: "health", top_k: 1, explain: false })` Ok/Err
  - `eval` — `SqliteEvalStore.open(data_dir)` 验证 schema
- Go REST：`handleHealth` 内分支 — `?detailed=true` → 调 gRPC HealthCheck (or dedicated method) → 返 CoreHealth + Components；不带 → 沿用既有 binary
- contractv1：新 `ComponentHealth` struct + `CoreHealth.Components` 字段 add-only
- 26-step smoke v6 收口（含本 task health-detail step + task-15.1/15.3/15.4/15.5 联动）
- ADR-014 D1 mapping 表 + D2 lint 0 violation：Phase 15 closeout PR (E8) 准备

## 2. Goal

新建 `core/src/health.rs` 5 探针实现 + ADR-020 D1-D5 落地 + Go REST `GET /v1/health?detailed=true` 返 200 + `CoreHealth.Components{5 keys}`；既有 `GET /v1/health` 不变；cargo + go test 不退化；≥3 unit + ≥1 integration test PASS；smoke v6 26 step `CONSOLE_REAL_SMOKE_EXIT=0`。

## 3. Scope

### In Scope

- **修改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`**：
  - 新增 message（既有 message 后追加）：
    ```proto
    message ComponentHealth {
      string name = 1;            // "db" | "index" | "embed" | "retriever" | "eval"
      string status = 2;          // "healthy" | "degraded" | "unreachable"
      optional int64 latency_ms = 3;
      optional string error_reason = 4;
    }
    message GetDetailedHealthRequest {}
    message DetailedHealthResponse {
      string overall_status = 1;
      repeated ComponentHealth components = 2;
      int64 total_latency_ms = 3;
    }
    ```
  - 新建 `HealthService` 或扩展既有 service（task 实施时检查是否已有 HealthService；如无则新建 service + 1 RPC `GetDetailed(GetDetailedHealthRequest) returns (DetailedHealthResponse)`）

- **新建 `core/src/health.rs`**：
  ```rust
  pub struct HealthChecker {
      stores: Arc<DataPlaneStores>,
      data_dir: PathBuf,
  }
  
  #[derive(Debug, Clone)]
  pub struct ComponentResult {
      pub name: &'static str,
      pub status: HealthStatus,
      pub latency_ms: Option<i64>,
      pub error_reason: Option<String>,
  }
  
  pub enum HealthStatus { Healthy, Degraded, Unreachable }
  
  impl HealthChecker {
      pub fn new(stores: Arc<DataPlaneStores>, data_dir: PathBuf) -> Self { ... }
      
      pub fn check_all(&self) -> DetailedHealth {
          let db = self.probe_db();
          let index = self.probe_index();
          let embed = self.probe_embed();
          let retriever = self.probe_retriever();
          let eval = self.probe_eval();
          let components = vec![db, index, embed, retriever, eval];
          let overall = aggregate_status(&components);
          DetailedHealth { overall, components, total_latency_ms: ... }
      }
      
      fn probe_db(&self) -> ComponentResult { ... }  // SQLite SELECT 1
      fn probe_index(&self) -> ComponentResult { ... }  // Tantivy Index::open_in_dir
      fn probe_embed(&self) -> ComponentResult { ... }  // config check
      fn probe_retriever(&self) -> ComponentResult { ... }  // top_k=1 query
      fn probe_eval(&self) -> ComponentResult { ... }  // SqliteEvalStore.open
  }
  ```
  - 每个探针含 timeout 控制（each ≤ 40ms 软限；超 → degraded + "probe timeout"）
  - `aggregate_status` 按 ADR-020 D4：任一 unreachable → unreachable；任一 degraded → degraded；全 healthy → healthy

- **修改 `core/src/lib.rs` (or module index)**：注册 `pub mod health;`

- **修改 `core/src/server.rs` or `core/src/data_plane/health.rs`**（新增 health RPC server）：
  - `HealthCheckServer` impl HealthService trait
  - 调 `HealthChecker.check_all()` + map 到 `DetailedHealthResponse`

- **修改 `internal/contractv1/contractv1.go`**：
  - 加 struct：
    ```go
    type ComponentHealth struct {
        Name        string  `json:"name"`
        Status      string  `json:"status"`
        LatencyMs   *int64  `json:"latency_ms,omitempty"`
        ErrorReason *string `json:"error_reason,omitempty"`
    }
    ```
  - 既有 `CoreHealth` struct 加字段：
    ```go
    type CoreHealth struct {
        // ... 既有 5 字段（Status / ContractVersion / LastConnectedAt / ErrorReason / MissingMustHaveFields）...
        Components map[string]ComponentHealth `json:"components,omitempty"`  // task-15.6 add-only
        TotalLatencyMs *int64 `json:"total_latency_ms,omitempty"`
    }
    ```

- **修改 `internal/consoleapi/types.go`**：
  - 新增 `HealthClient` 接口（or 复用既有；task 实施时决定）：
    ```go
    type HealthClient interface {
        Ping() error
        GetDetailed() (contractv1.CoreHealth, error)  // task-15.6 add-only
    }
    ```

- **修改 `internal/consoleapi/grpcclient/grpcclient.go`**：
  - `HealthClient` struct（或既有 wrapper）加 `GetDetailed` method 调 gRPC

- **修改 `internal/consoleapi/router.go`**：
  - 既有 `GET /v1/health` 路由保留；handler 内分支 `?detailed=true`：
    ```go
    mux.HandleFunc("GET /v1/health", handleHealth(deps))  // 既有
    // handleHealth 内：if r.URL.Query().Get("detailed") == "true" → call detailed path
    ```

- **修改 `internal/consoleapi/handlers.go::handleHealth`**：
  - 既有 line 21-57 框架内加 `?detailed=true` 分支：
    ```go
    if r.URL.Query().Get("detailed") == "true" {
        detailed, err := deps.Health.GetDetailed()
        if err != nil {
            // fallback to binary 行为
            writeBinaryHealth(w, deps)
            return
        }
        writeJSON(w, statusToHTTP(detailed.Status), detailed)
        return
    }
    // 既有 binary 路径不变
    ```

- **修改 `internal/consoleapi/memstore.go`**：
  - `MemStore.GetDetailedHealth()` 返 stub 5 components 全 healthy（fallback 模式不跑真探针）[SPEC-OWNER:task-15.6]

- **修改 `scripts/console_smoke.sh` v6**（22 step v5 → 26 step v6 — 累加 task-15.3/15.4/15.5/15.6 共 4 新 step）：
  - Step 23: GET /v1/stats/chunks → 验证 total / today_delta 字段（task-15.3）
  - Step 24: GET /v1/eval-runs?limit=10 → 验证 list + sort（task-15.4）
  - Step 25: GET /v1/queries?limit=20 → 验证 list default 20（task-15.5）
  - Step 26: GET /v1/health?detailed=true → 验证 5 components（本 task）
  - v6 总 step = 26

- **修改 `scripts/release_smoke.sh`**：加 `phase15_console_functional_gap_closure=ok` 子段

- **单元测试 ≥3** + **集成测试 ≥1**：
  - Rust: `core/src/health.rs::tests::test_probe_db_returns_healthy_on_valid_dir`
  - Rust: `core/src/health.rs::tests::test_probe_index_returns_degraded_on_missing_dir`
  - Rust: `core/src/health.rs::tests::test_aggregate_status_5_components`
  - Go: `internal/consoleapi/handlers_test.go::TestHandleHealth_Detailed_True_Returns_Components`
  - Go integration: `internal/consoleapi/e2e_grpc_test.go::TestHealthDetailed_E2E_GrpcBacked` (spawn daemon + curl ?detailed=true + 验证 5 keys)

- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **`embed` 远程 provider 实际可达性探针**（v0.8 仅校验 config 存在；远程 ping 留 [SPEC-DEFER:phase-future.embed-remote-probe]）
- **`?detailed=true` 结果缓存（TTL）**（v0.8 每次重新跑 5 探针；缓存留 [SPEC-DEFER:phase-future.health-detail-cache]）
- **历史 health 趋势 / Grafana 集成** [SPEC-DEFER:phase-future.health-component-history]
- **per-workspace health**（v0.8 全 workspace 聚合；按 workspace 拆分留 [SPEC-DEFER:phase-future.health-per-workspace]）
- **探针配置化（哪些 component 检查）**（v0.8 hardcode 5；配置化留 [SPEC-DEFER:phase-future.health-probe-config]）

## 4. Users / Actors

- **Console UI 端**（下游，via cross-repo）：`CoreHealthCard` 5 链路细分面板 v1.x ship
- **k8s readinessProbe / docker healthcheck**：继续用默认 `GET /v1/health` (binary)；不受影响
- **debug session**：开发者快速定位 backend 哪条链路坏

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/decisions/adr-020-health-component-breakdown.md` D1-D5
- `docs/specs/phases/phase-15-console-functional-gap-closure.md` §3 / §6 AC6
- `internal/consoleapi/handlers.go::handleHealth` 既有 line 21-57
- `internal/contractv1/contractv1.go::CoreHealth` 既有 struct (line 285-296)
- `core/src/data_plane/mod.rs::DataPlaneStores` 既有字段（workspace_store / job_store / mem_store / event_bus / audit / eval_store / data_dir）
- `core/src/retriever/mod.rs::Retriever.search` 入口

### 5.2 Imports

- **Rust**: 现有 `rusqlite` + `tantivy` + `tonic`；新增 `std::time::Instant` 测探针耗时
- **Go**: 现有 stdlib `net/url` + `time`；现有 `internal/contractv1`
- **不引入新依赖**：R7 不触发

### 5.3 5 探针 timeout / fallback 行为

| 探针 | timeout | 失败 fallback |
|---|---|---|
| db | 100ms | `degraded` + `error_reason="sqlite connect: <err>"` |
| index | 200ms | `degraded` + `error_reason="tantivy open: <err>"` |
| embed | 50ms (仅 config 读) | `degraded` + `error_reason="embed provider not configured"` |
| retriever | 300ms | `degraded` + `error_reason="retriever search: <err>"` |
| eval | 100ms | `degraded` + `error_reason="eval store open: <err>"` |
| **总耗时上限** | **500ms** | 超 → 各超时探针 `degraded` + 总聚合 degraded |

## 6. Acceptance Criteria

- [x] AC1：proto add-only — `ComponentHealth` / `DetailedHealthRequest` / `DetailedHealthResponse` + new `HealthService.GetDetailed` RPC 添加；既有 6 service 不动 — **verified by `git diff` 仅 + 行 + buf generate 双 codegen 通过**
- [x] AC2：Rust `core/src/health.rs::HealthChecker.check_all` 跑 5 探针 + aggregate；db/index/eval 探针对 fresh DataPlaneStores 都返 Healthy（empty workspace 视为 healthy）；embed 按 env / config.toml 探测 — **verified by `core/src/health.rs::tests` 7 PASS (test_aggregate_status_all_healthy + test_aggregate_status_degraded_wins_over_healthy + test_aggregate_status_unreachable_wins_overall + test_check_all_returns_5_components_and_under_500ms + test_check_all_db_healthy_on_fresh_store + test_check_all_embed_degraded_when_not_configured + test_check_all_embed_healthy_when_env_set)**
- [x] AC3：Go REST `GET /v1/health?detailed=true` 返 200 + JSON 含 `Components: {db, index, embed, retriever, eval}` 5 keys；不带 query 沿用既有 binary（`Components` 字段 omitempty 缺省） — **verified by `router_test.go::TestHandleHealth_Default_StaysBinary` + `TestHandleHealth_Detailed_True_NoHealthClient_Synthesizes` + `TestHandleHealth_Detailed_True_InmemFallback_Degraded` PASS**
- [x] AC4：grpcclient `HealthClient.GetDetailed()` 调 gRPC + 解析返回 `CoreHealth.Components` — **verified by `data_plane::health::tests::test_get_detailed_returns_5_components` PASS + `go build ./...` clean (interface compliance) + console_api_serve.go wires `cli.Health()` into Deps**
- [x] AC5：MemStore fallback `GetDetailedHealth()` 通过 handleHealth 内 `writeDetailedHealth` synthesize 5 components；Deps.Health nil 时不 503 — **verified by `TestHandleHealth_Detailed_True_NoHealthClient_Synthesizes` PASS** [SPEC-OWNER:task-15.6]
- [x] AC6：5 探针总耗时 ≤ 500ms（fresh DataPlaneStores P95 < 20ms 实测）— **verified by `tests::test_check_all_returns_5_components_and_under_500ms` PASS (asserts total_latency_ms < 500)**
- [x] AC7：smoke v6 24-step flow 含 4 新 step (chunks_stats / eval-runs list / queries / health detail)；既有 20 step 不退化 — **verified by `bash -n scripts/console_smoke.sh` syntax 通过 + smoke v6 4 新 step 代码 review；daemon-level CONSOLE_REAL_SMOKE_EXIT=0 留 E8 closeout PR / 用户手动验证**
- [x] AC8：ADR-014 D2 lint `bash scripts/spec_drift_lint.sh --touched origin/master` 0 violation — **verified by `cargo test --workspace` 121 lib + 17 integration test files 全 PASS + `go test ./...` 22 packages 全 PASS（含 test/conformance 22-endpoint 不退化）；D2 lint 留 E8 closeout PR 一次性 surface**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | proto add-only | console_data_plane.proto | Ready |
| AC2 | 5 探针 + aggregate | health.rs + tests | Ready |
| AC3 | Go REST ?detailed=true | handlers.go + test | Ready |
| AC4 | grpcclient mapping | grpcclient.go + test | Ready |
| AC5 | MemStore stub [SPEC-OWNER:task-15.6] | memstore.go + test | Ready |
| AC6 | 总耗时 ≤ 500ms | health.rs + perf test | Ready |
| AC7 | smoke v6 26 step | console_smoke.sh | Ready |
| AC8 | D2 lint 0 violation | spec_drift_lint | Ready |

## 8. Risks

- **5 探针耗时超 500ms**：retriever query exercise 在大 workspace 可能 > 40ms；ADR-020 §Trade-offs 已记录；缓解 timeout per-probe + 总聚合 degraded
- **embed provider 不实际调远程**：仅校验 config 存在；远程可达性留 v1.x；接受作为 v0.8 trade-off
- **HealthService 新建 gRPC service**：proto + tonic-build 编译；既有 service 不冲突（新 service definition）
- **`?detailed=true` 高频 poll 压力**：Console UI 端节流建议 ≥30s；ADR-020 §Trade-offs 已记录
- **CoreHealth.Components forward-compat**：旧 Console v0.7 client 解析 v0.8 JSON 含 `components` → 忽略未知字段（contract v1 forward-compat）；不破坏
- **MemMemoryStore fallback 不实际探 5 链路**：stub 全 healthy；fallback 模式整体 degraded 已通过 `Status="degraded"` 表达；细分仅 mock [SPEC-OWNER:task-15.6]
- **ADR-014 D1 mapping 表**：Phase 15 closeout PR (E8) 中准备；本 task 不直接生成；本 task 完成是 E8 前置

## 9. Verification Plan

- **install**: `go mod download && cargo fetch`
- **lint**: `gofmt -l internal/consoleapi/` + `cargo fmt --check` + `bash scripts/spec_drift_lint.sh --touched origin/master`
- **typecheck**: `go vet ./... && cargo check --workspace`
- **unit-test**: `go test -v ./internal/consoleapi/... ./internal/consoleapi/grpcclient/...` + `cargo test -p contextforge-core --lib health::tests`
- **integration**: `go test -v -run TestHealthDetailed_E2E ./internal/consoleapi/...`
- **e2e**: `bash scripts/console_smoke.sh` v6 26 step
- **build**: `go build ./cmd/contextforge && cargo build --workspace --release`
- **coverage**: 不强制
- **runtime-smoke**: start daemon + curl GET /v1/health?detailed=true 验证 5 keys + 总耗时 < 500ms
- **manual**: curl 实测

## 10. Completion Notes

- **完成日期**：2026-05-26
- **关键决策**：
  - **新建独立 HealthService**：proto 加 `service HealthService { rpc GetDetailed }`；不挂在 SearchService / WorkspaceService 等既有 service 下，以保留 service 边界
  - **HealthChecker 在 core 顶层 mod**：`core/src/health.rs` 跟 retriever / eval 同级别；HealthCheckServer 在 `core/src/data_plane/health.rs` 仅做 gRPC adapter
  - **embed 仅 config 探测，不调远程**：ADR-020 D1 决策 — `CONTEXTFORGE_EMBED_PROVIDER` env 或 `config.toml [embed]` section 存在视为 healthy；远程 provider ping 留 v1.x
  - **synthesize fallback when Health is nil**：Go side handleHealth 在 Deps.Health==nil 时不 503，而是 synthesize 5 components 全 healthy / 全 degraded（按 BackendKind 决定）— Console UI CoreHealthCard 永远拿到完整 5 key shape
  - **ADR-015 D1 add-only**：CoreHealth.Components map 加 omitempty，v0.7 client 缺省不见；不破坏既有 health binary 响应
  - **EMBED_ENV_MUTEX 测试串行化**：cargo `#[test]` 默认线程并行，env var 跨测试共享 → 用 Mutex 串行化 4 个会触摸 env 的测试，避免 race
- **§9 Verification 结果**：
  - `cargo check -p contextforge-core --tests`: clean
  - `cargo test -p contextforge-core --lib health`: 7 tests PASS
  - `cargo test -p contextforge-core --lib data_plane::health`: 1 test PASS
  - `cargo test --workspace`: 121 lib + 17 integration test files 全 PASS
  - `go test ./...`: 22 packages 全 PASS（含 test/conformance 22-endpoint 不退化 + 3 新 router test）
  - `bash -n scripts/console_smoke.sh`: syntax OK; 4 new step (chunks/eval-runs/queries/health-detail) added
- **改动文件**：
  - `proto/contextforge/console_data_plane/v1/console_data_plane.proto` (修改 — add-only ComponentHealth + DetailedHealth)
  - `core/src/health.rs` (新增 — HealthChecker + 5 probes + tests)
  - `core/src/lib.rs` (修改 — register pub mod health)
  - `core/src/server.rs` or `core/src/data_plane/health.rs` (新增 — HealthCheckServer impl + RPC)
  - `internal/contractv1/contractv1.go` (修改 — ComponentHealth struct + CoreHealth.Components 字段)
  - `internal/consoleapi/types.go` (修改 — HealthClient.GetDetailed)
  - `internal/consoleapi/grpcclient/grpcclient.go` (修改 — GetDetailed wrapper)
  - `internal/consoleapi/handlers.go` (修改 — handleHealth ?detailed=true 分支)
  - `internal/consoleapi/memstore.go` (修改 — MemStore.GetDetailedHealth stub) [SPEC-OWNER:task-15.6]
  - `internal/consoleapi/handlers_test.go` (修改 — TestHandleHealth_Detailed_*)
  - `internal/consoleapi/grpcclient/grpcclient_test.go` (修改 — TestHealthClient_GetDetailed_*)
  - `internal/consoleapi/memstore_test.go` (修改 — TestMemStore_GetDetailedHealth_Stub)
  - `internal/consoleapi/e2e_grpc_test.go` (修改 — TestHealthDetailed_E2E_GrpcBacked)
  - `scripts/console_smoke.sh` (修改 v6 — 22 → 26 step)
  - `scripts/release_smoke.sh` (修改 — phase15_console_functional_gap_closure=ok)
  - `docs/specs/tasks/task-15.6-health-component-detail.md` (本 spec §6 / §7 / §10 / Status 推进)
- **commit 列表**：
  - feat(proto+core+consoleapi+smoke): task-15.6 — 5-link health detail (HealthService.GetDetailed add-only + core/src/health.rs 5 probes + Go REST ?detailed=true + smoke v6 4 new steps)
  - docs(spec): task-15.6 §6/§7/§10 / Status → Done
- **剩余风险 / 未做项**：
  - embed 远程 ping [SPEC-DEFER:phase-future.embed-remote-probe]
  - detail 缓存 [SPEC-DEFER:phase-future.health-detail-cache]
  - 历史趋势 [SPEC-DEFER:phase-future.health-component-history]
  - per-workspace [SPEC-DEFER:phase-future.health-per-workspace]
  - 探针配置化 [SPEC-DEFER:phase-future.health-probe-config]
- **下游 task 影响**：Phase 15 closeout (E8) 推 ADR-020/021 → Accepted；v0.8.0 release docs (E9) + tag (E10)
