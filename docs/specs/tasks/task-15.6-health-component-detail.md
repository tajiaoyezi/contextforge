# Task `15.6`: `health-component-detail — proto ComponentHealth message + 5 探针 (db/index/embed/retriever/eval) + Go REST GET /v1/health?detailed=true`

**Status**: Ready

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
  - `MemStore.GetDetailedHealth()` 返 stub 5 components 全 healthy（fallback 模式不跑真探针）

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

- [ ] AC1：proto add-only — `ComponentHealth` / `GetDetailedHealthRequest` / `DetailedHealthResponse` message 添加；既有 service / message 不动 — **verified by `git diff` 仅 + 行 + tonic-build 编译通过**
- [ ] AC2：Rust `core/src/health.rs::HealthChecker.check_all` 跑 5 探针 + aggregate；空 data_dir → 5 components 全 degraded；正常 data_dir → 全 healthy — **verified by `core/src/health.rs::tests::test_probe_db_*` + `test_probe_index_*` + `test_aggregate_status_5_components` 3 测试 PASS**
- [ ] AC3：Go REST `GET /v1/health?detailed=true` 返 200 + JSON 含 `components: {db, index, embed, retriever, eval}` 5 keys；不带 query 沿用既有 binary — **verified by `handlers_test.go::TestHandleHealth_Detailed_True_Returns_Components` + `TestHandleHealth_Default_Returns_Binary` PASS**
- [ ] AC4：grpcclient `HealthClient.GetDetailed()` 调 gRPC + 解析返回 `CoreHealth.Components` — **verified by `grpcclient_test.go::TestHealthClient_GetDetailed_Maps_Proto` PASS**
- [ ] AC5：MemStore fallback `GetDetailedHealth()` 返 stub 5 components 全 healthy；conformance 不破坏 — **verified by `memstore_test.go::TestMemStore_GetDetailedHealth_Stub` PASS**
- [ ] AC6：5 探针总耗时 ≤ 500ms（P95 测量）— **verified by `tests::test_check_all_under_500ms` PASS**
- [ ] AC7：smoke v6 26-step flow `CONSOLE_REAL_SMOKE_EXIT=0`；Step 23/24/25/26 全 PASS；既有 22 step 不退化 — **verified by `bash scripts/console_smoke.sh` 实测 stdout 含 `CONSOLE_REAL_SMOKE_EXIT=0`**
- [ ] AC8：ADR-014 D2 lint `bash scripts/spec_drift_lint.sh --touched origin/master` 0 violation — **verified by lint stdout**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | proto add-only | console_data_plane.proto | Ready |
| AC2 | 5 探针 + aggregate | health.rs + tests | Ready |
| AC3 | Go REST ?detailed=true | handlers.go + test | Ready |
| AC4 | grpcclient mapping | grpcclient.go + test | Ready |
| AC5 | MemStore stub | memstore.go + test | Ready |
| AC6 | 总耗时 ≤ 500ms | health.rs + perf test | Ready |
| AC7 | smoke v6 26 step | console_smoke.sh | Ready |
| AC8 | D2 lint 0 violation | spec_drift_lint | Ready |

## 8. Risks

- **5 探针耗时超 500ms**：retriever query exercise 在大 workspace 可能 > 40ms；ADR-020 §Trade-offs 已记录；缓解 timeout per-probe + 总聚合 degraded
- **embed provider 不实际调远程**：仅校验 config 存在；远程可达性留 v1.x；接受作为 v0.8 trade-off
- **HealthService 新建 gRPC service**：proto + tonic-build 编译；既有 service 不冲突（新 service definition）
- **`?detailed=true` 高频 poll 压力**：Console UI 端节流建议 ≥30s；ADR-020 §Trade-offs 已记录
- **CoreHealth.Components forward-compat**：旧 Console v0.7 client 解析 v0.8 JSON 含 `components` → 忽略未知字段（contract v1 forward-compat）；不破坏
- **MemMemoryStore fallback 不实际探 5 链路**：stub 全 healthy；fallback 模式整体 degraded 已通过 `Status="degraded"` 表达；细分仅 mock
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

- **完成日期**：<待填>
- **关键决策**：<待填>
- **§9 Verification 结果**：<待填>
- **改动文件**：
  - `proto/contextforge/console_data_plane/v1/console_data_plane.proto` (修改 — add-only ComponentHealth + DetailedHealth)
  - `core/src/health.rs` (新增 — HealthChecker + 5 probes + tests)
  - `core/src/lib.rs` (修改 — register pub mod health)
  - `core/src/server.rs` or `core/src/data_plane/health.rs` (新增 — HealthCheckServer impl + RPC)
  - `internal/contractv1/contractv1.go` (修改 — ComponentHealth struct + CoreHealth.Components 字段)
  - `internal/consoleapi/types.go` (修改 — HealthClient.GetDetailed)
  - `internal/consoleapi/grpcclient/grpcclient.go` (修改 — GetDetailed wrapper)
  - `internal/consoleapi/handlers.go` (修改 — handleHealth ?detailed=true 分支)
  - `internal/consoleapi/memstore.go` (修改 — MemStore.GetDetailedHealth stub)
  - `internal/consoleapi/handlers_test.go` (修改 — TestHandleHealth_Detailed_*)
  - `internal/consoleapi/grpcclient/grpcclient_test.go` (修改 — TestHealthClient_GetDetailed_*)
  - `internal/consoleapi/memstore_test.go` (修改 — TestMemStore_GetDetailedHealth_Stub)
  - `internal/consoleapi/e2e_grpc_test.go` (修改 — TestHealthDetailed_E2E_GrpcBacked)
  - `scripts/console_smoke.sh` (修改 v6 — 22 → 26 step)
  - `scripts/release_smoke.sh` (修改 — phase15_console_functional_gap_closure=ok)
  - `docs/specs/tasks/task-15.6-health-component-detail.md` (本 spec §6 / §7 / §10 / Status 推进)
- **commit 列表**：<待填>
- **剩余风险 / 未做项**：
  - embed 远程 ping [SPEC-DEFER:phase-future.embed-remote-probe]
  - detail 缓存 [SPEC-DEFER:phase-future.health-detail-cache]
  - 历史趋势 [SPEC-DEFER:phase-future.health-component-history]
  - per-workspace [SPEC-DEFER:phase-future.health-per-workspace]
  - 探针配置化 [SPEC-DEFER:phase-future.health-probe-config]
- **下游 task 影响**：Phase 15 closeout (E8) 推 ADR-020/021 → Accepted；v0.8.0 release docs (E9) + tag (E10)
