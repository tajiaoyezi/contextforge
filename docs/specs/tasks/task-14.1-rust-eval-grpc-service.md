# Task `14.1`: `rust-eval-grpc-service — eval_runs SQLite schema + SqliteEvalStore + EvalService gRPC + recall harness orchestration`

**Status**: Ready

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 14 (eval-rest-surface)
**Dependencies**: task-13.1 (proto + Rust gRPC service pattern from MemoryService) + task-8.1 (internal/eval/eval.go recall harness 已 ship) + [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D1 Wave 4 / D6 + [ADR-006](../../decisions/adr-006-recall-eval-acceptance-gate.md)

## 1. Background

[ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) Phase 11 在 `proto/contextforge/console_data_plane/v1/console_data_plane.proto` 已定义 `EvalRun` message（complete schema 1:1 镜像 Go contractv1.EvalRun，含 case_results / metrics / status lifecycle）。Phase 8 task-8.1 ship 完整 recall harness CLI (`contextforge eval run` + `internal/eval/eval.go`)，但**没有 EvalService gRPC + 没有 eval_runs SQLite 表 + recall harness 是 CLI 一次性触发不写 persistent state**。

本 task 在 Rust 侧建立 Eval 持久化 + gRPC service + orchestration layer：
1. 新增 SQLite migration `0014_eval_runs.sql` 定义 `eval_runs` 表
2. 新增 `core/src/eval/` module + `SqliteEvalStore` CRUD + state ops
3. amend proto 加 EvalService 2 RPC
4. 新增 `core/src/data_plane/eval.rs` EvalServer impl
5. 新增 `core/src/eval/runner.rs` EvalRunner orchestration（spawn_blocking + 调既存 Phase 8 recall harness + 写 progress + 完成时写 metrics + case_results）
6. 注册 EvalServer 到 `serve_full`

**关键 scope 决策（§3 + §10 trade-off）**：本 task EvalRunner 实现路径**选「Go console-api-serve 进程内 spawn goroutine 调 internal/eval/eval.go + 通过 gRPC 更新 SqliteEvalStore」**而非「Rust spawn_blocking 调 Go binary as subprocess」—— 前者简单 + 错误传播自然；后者 OS process 管理复杂。这意味着 Rust EvalService.Create 仅做「INSERT eval_runs status=running」+ 不真触发 harness；harness 触发由 Go 侧 task-14.2 实现。但 EvalService.Get 真持久化读 SqliteEvalStore（包含 Go 侧已写入的 metrics / case_results / status）。Trade-off 详 §10。

## 2. Goal

`core/migrations/0014_eval_runs.sql` 含 `eval_runs` 表 (10 列 1:1 镜像 contractv1.EvalRun + indexes)；`core/src/eval/store.rs` 含 `SqliteEvalStore` CRUD + state ops；`proto/contextforge/console_data_plane/v1/console_data_plane.proto` 加 `EvalService` 2 RPC + EvalRunCreate / CaseResult message；`core/src/data_plane/eval.rs` impl EvalService trait + 接 SqliteEvalStore；`core/src/server.rs` 注册 EvalServer service；`cargo test --workspace` 全绿；≥6 单元测试 + ≥2 集成测试 PASS。

## 3. Scope

### In Scope

- **新增 `core/migrations/0014_eval_runs.sql`**：
  ```sql
  CREATE TABLE IF NOT EXISTS eval_runs (
    eval_run_id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running'
        CHECK (status IN ('running', 'succeeded', 'failed', 'cancelled')),
    config_snapshot_json TEXT NOT NULL DEFAULT '{}',  -- JSON serialize
    started_at_unix INTEGER NOT NULL,
    finished_at_unix INTEGER,  -- NULL when running
    metrics_json TEXT NOT NULL DEFAULT '{}',  -- map<string, float64> as JSON
    case_results_json TEXT NOT NULL DEFAULT '[]',  -- array of CaseResult as JSON
    schema_version TEXT NOT NULL DEFAULT 'v1',
    dataset_ref TEXT,  -- optional
    error_message TEXT  -- optional, for status=failed
  );
  CREATE INDEX IF NOT EXISTS idx_eval_runs_workspace ON eval_runs(workspace_id);
  CREATE INDEX IF NOT EXISTS idx_eval_runs_status ON eval_runs(status);
  CREATE INDEX IF NOT EXISTS idx_eval_runs_started_at ON eval_runs(started_at_unix);
  ```
  - case_results 用 JSON text 列嵌入（不另起 eval_case_results 子表）—— Console contract case_results 是 array of typed CaseResult；JSON serialization 简单 + 不引入跨表 JOIN；trade-off 接受
  - status_version 通过现有 `core/src/migrations.rs` 注册
- **新增 `core/src/eval/mod.rs`**：
  ```rust
  pub mod store;
  pub mod runner;
  pub use store::{SqliteEvalStore, EvalStoreError};
  pub use runner::EvalRunner;
  ```
- **新增 `core/src/eval/store.rs`**：
  - `pub struct SqliteEvalStore { conn: Arc<parking_lot::Mutex<rusqlite::Connection>> }` (与 SqliteMemoryStore 模式一致)
  - Methods:
    - `pub fn new(conn: Arc<Mutex<Connection>>) -> Result<Self>`
    - `pub fn create(&self, req: EvalRunCreate) -> Result<EvalRun, EvalStoreError>` (INSERT row + 返 EvalRun{status="running", started_at=now})
    - `pub fn get(&self, id: &str) -> Result<Option<EvalRun>, EvalStoreError>` (None if not found; JSON decode metrics/case_results)
    - `pub fn update_metrics(&self, id: &str, metrics: HashMap<String, f64>) -> Result<()>` (UPDATE metrics_json)
    - `pub fn update_case_results(&self, id: &str, results: Vec<CaseResult>) -> Result<()>`
    - `pub fn mark_finished(&self, id: &str, status: &str, finished_at: i64, error: Option<String>) -> Result<()>` (status="succeeded"/"failed"/"cancelled" + finished_at + error_message)
- **新增 `core/src/eval/runner.rs`**：
  - `pub struct EvalRunner { store: Arc<SqliteEvalStore> }` — **本 task 仅做 Rust 侧 store + 占位 trigger API**；真 recall harness 触发在 Go 侧 task-14.2 [SPEC-OWNER:task-14.2]
  - `pub fn trigger_external(&self, eval_run_id: &str, callback_url: &str)` 占位 (本 task 不实施真 spawn；返 unit) — 真触发由 Go console-api-serve goroutine 跑 recall + 经 gRPC 反向 update store [SPEC-OWNER:task-14.2]
  - `pub fn record_progress(&self, eval_run_id: &str, progress_event: EvalProgressEvent)` 占位 — phase-15 引入 [SPEC-DEFER:phase-15.eval-progress-streaming]
- **修改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`**：
  ```proto
  service EvalService {
    rpc Create(CreateEvalRunRequest) returns (EvalRun);
    rpc Get(GetEvalRunRequest) returns (EvalRun);
    rpc UpdateProgress(UpdateEvalRunProgressRequest) returns (UpdateEvalRunProgressResponse);  // 内部 RPC for Go side runner callback
  }

  message CreateEvalRunRequest {
    string workspace_id = 1;
    string config_snapshot_json = 2;  // JSON string; deserialize Rust side
    string dataset_ref = 3;
  }
  message GetEvalRunRequest { string eval_run_id = 1; }

  message UpdateEvalRunProgressRequest {
    string eval_run_id = 1;
    string status = 2;                          // "running"/"succeeded"/"failed"/"cancelled"
    string metrics_json = 3;                    // map<string, float64> as JSON
    repeated CaseResult case_results = 4;
    string error_message = 5;                   // for status=failed
  }
  message UpdateEvalRunProgressResponse {}

  message CaseResult {
    string case_id = 1;
    string query = 2;
    repeated string expected_chunks = 3;
    repeated string actual_chunks = 4;
    double score = 5;
    bool passed = 6;
  }

  // EvalRun message 已存（Phase 11 task-11.1 ship 11 message 含 EvalRun）
  ```
- **新增 `core/src/data_plane/eval.rs`**：
  - `pub struct EvalServer { stores: Arc<DataPlaneStores> }`
  - impl proto::eval_service_server::EvalService:
    - `create`: parse `CreateEvalRunRequest` → `stores.eval.create(EvalRunCreate{workspace_id, config_snapshot, dataset_ref})` → 返 EvalRun
    - `get`: `stores.eval.get(req.eval_run_id)` → `Some` 返 EvalRun / `None` 返 Status::not_found
    - `update_progress`: `stores.eval.update_metrics + update_case_results + mark_finished if status terminal` —— 这个 RPC 被 Go 侧 task-14.2 EvalRunner 调来反向 update 状态；不在 Console REST 暴露
- **修改 `core/src/data_plane/mod.rs`**：
  - `DataPlaneStores` 加字段 `pub eval: Arc<SqliteEvalStore>`
  - `register_services` 加 `.add_service(eval::EvalServer::new(stores.clone()).into_service())`
- **修改 `core/src/server.rs`**：
  - `serve_full` 实例化 SqliteEvalStore 加入 DataPlaneStores
- **修改 `core/src/migrations.rs`** 或 migration 注册中心：
  - 在注册列表加 0014_eval_runs.sql
- **单元测试 ≥6**：
  - `core/src/eval/store.rs::tests::test_create_and_get_roundtrip`
  - `core/src/eval/store.rs::tests::test_update_metrics_persists` (JSON serialization roundtrip)
  - `core/src/eval/store.rs::tests::test_update_case_results_persists`
  - `core/src/eval/store.rs::tests::test_mark_finished_succeeded_sets_finished_at`
  - `core/src/eval/store.rs::tests::test_status_check_constraint_rejects_invalid`
  - `core/src/data_plane/eval.rs::tests::test_eval_server_create_returns_running`
  - `core/src/data_plane/eval.rs::tests::test_eval_server_get_404`
  - `core/src/data_plane/eval.rs::tests::test_update_progress_persists_terminal_status`
- **集成测试 ≥2**：
  - `core/tests/eval_integration.rs::test_eval_crud_via_grpc` (spawn tonic + tonic client + Create → Get → UpdateProgress flow)
  - `core/tests/eval_integration.rs::test_eval_run_terminal_lifecycle` (Create → UpdateProgress with status=succeeded + metrics + case_results → Get 返完整 EvalRun)
- **文件锚点**：`core/migrations/0014_eval_runs.sql` + `core/src/eval/{mod,store,runner}.rs` + `core/src/data_plane/eval.rs` + `core/src/data_plane/mod.rs` + `core/src/server.rs` + `core/src/migrations.rs` (注册) + `proto/contextforge/console_data_plane/v1/console_data_plane.proto` + `core/tests/eval_integration.rs`
- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **Go REST handlers + grpcclient.EvalClient + EvalRunner Go-side spawn goroutine** [SPEC-OWNER:task-14.2]：本 task 仅 Rust 侧
- **真 recall harness 触发 Rust spawn_blocking 路径** [SPEC-DEFER:phase-future.rust-native-eval-runner]：v0.7 选 Go-side runner；Rust native 留 v1.x
- **Eval progress streaming (SSE / server stream RPC)** [SPEC-DEFER:phase-15.eval-progress-streaming]
- **golden_questions dataset CRUD** [SPEC-DEFER:console-dataset-management]
- **既存 proto/contextforge/v1/eval.proto 改动**：本 task **不动** v1/eval.proto；EvalRequest/EvalResponse recall-only 留 CLI 路径用；console_data_plane v1/EvalService 独立演进
- **Eval cancel REST endpoint**：Console 22-endpoint 不含 POST /v1/eval-runs/{id}/cancel；如未来加 → [SPEC-DEFER:console-eval-cancel]

## 4. Users / Actors

- **task-14.2 go-eval-rest-handlers 实施 agent**（下游）：消费本 task EvalService 作 grpcclient 桥梁 + 实现 2 REST handler + 实现 Go-side EvalRunner goroutine 跑 recall harness + 调 UpdateProgress 反向 update
- **task-future.rust-native-eval-runner**（v1.x）：复用本 task SqliteEvalStore + EvalServer 框架；改 EvalRunner.trigger 为 Rust 真 spawn_blocking 调 recall

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/decisions/adr-017-console-contract-completion-22-endpoint.md` §D1 Wave 4 / §D6
- `docs/decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md` §D1 / §D5
- `docs/decisions/adr-006-recall-eval-acceptance-gate.md`
- `docs/specs/phases/phase-14-eval-rest-surface.md` §3 / §6
- `docs/specs/tasks/task-8.1-eval-harness.md` (Phase 8 recall harness CLI 接口)
- `docs/specs/tasks/task-13.1-rust-memory-grpc-service.md` (proto + Rust gRPC service pattern)
- `H:/devlopment/code/contextforge/internal/contractv1/contractv1.go::EvalRun, EvalRunCreate, CaseResult` (字段集合)
- `H:/devlopment/code/contextforge/internal/eval/eval.go` (recall harness 入口)

### 5.2 Imports

- **Rust**: 现有 `tonic 0.12` + `prost 0.13` + `rusqlite` + `parking_lot` + `serde_json`（既有 dep；用于 JSON serialize metrics/case_results）
- **不引入新依赖**：R7 不触发

### 5.3 EvalServer 形状

```rust
// core/src/data_plane/eval.rs
pub struct EvalServer {
    stores: Arc<DataPlaneStores>,
}

#[tonic::async_trait]
impl proto::eval_service_server::EvalService for EvalServer {
    async fn create(&self, req: Request<CreateEvalRunRequest>) -> Result<Response<EvalRun>, Status> {
        let r = req.into_inner();
        let create_req = EvalRunCreate {
            workspace_id: r.workspace_id,
            config_snapshot: serde_json::from_str(&r.config_snapshot_json).unwrap_or_default(),
            dataset_ref: if r.dataset_ref.is_empty() { None } else { Some(r.dataset_ref) },
        };
        match self.stores.eval.create(create_req) {
            Ok(run) => Ok(Response::new(eval_run_to_proto(run))),
            Err(e) => Err(Status::internal(format!("eval create error: {}", e))),
        }
    }

    async fn get(&self, req: Request<GetEvalRunRequest>) -> Result<Response<EvalRun>, Status> {
        match self.stores.eval.get(&req.into_inner().eval_run_id) {
            Ok(Some(run)) => Ok(Response::new(eval_run_to_proto(run))),
            Ok(None) => Err(Status::not_found("eval run not found")),
            Err(e) => Err(Status::internal(format!("eval get error: {}", e))),
        }
    }

    async fn update_progress(&self, req: Request<UpdateEvalRunProgressRequest>) -> Result<Response<UpdateEvalRunProgressResponse>, Status> {
        let r = req.into_inner();
        let metrics: HashMap<String, f64> = serde_json::from_str(&r.metrics_json).unwrap_or_default();
        let case_results: Vec<CaseResult> = r.case_results.into_iter().map(case_result_from_proto).collect();
        self.stores.eval.update_metrics(&r.eval_run_id, metrics).map_err(...)?;
        self.stores.eval.update_case_results(&r.eval_run_id, case_results).map_err(...)?;
        if matches!(r.status.as_str(), "succeeded" | "failed" | "cancelled") {
            let now = chrono::Utc::now().timestamp();
            let err = if r.error_message.is_empty() { None } else { Some(r.error_message) };
            self.stores.eval.mark_finished(&r.eval_run_id, &r.status, now, err).map_err(...)?;
        }
        Ok(Response::new(UpdateEvalRunProgressResponse {}))
    }
}
```

## 6. Acceptance Criteria

- [ ] AC1：`0014_eval_runs.sql` migration 成功执行（含 10 列 + 3 索引 + CHECK on status）；daemon 启动后 `eval_runs` 表存在 — **verified by integration `test_eval_crud_via_grpc` PASS**
- [ ] AC2：`SqliteEvalStore` 5+ method 全工作；JSON serialization roundtrip 对 metrics (map) + case_results (array) 正确 — **verified by 5 unit tests `core/src/eval/store.rs::tests::*` PASS**
- [ ] AC3：`EvalService` gRPC 3 RPC 注册可见（Create / Get / UpdateProgress）；Create 返 status="running" + started_at=now；Get 不存在 → not_found；UpdateProgress 反向 update store 包含 metrics + case_results + finished_at 在 status terminal 时填实 — **verified by integration `test_eval_run_terminal_lifecycle` PASS**
- [ ] AC4：EvalRun JSON serialization：config_snapshot / metrics / case_results 三个嵌套 JSON 字段 roundtrip 不丢失类型（float64 不变 int / array 不变 object 等）— **verified by unit test `test_json_roundtrip_preserves_types` PASS**
- [ ] AC5：`cargo test --workspace` 全绿（不破坏既有 Phase 11/12/13 测试）；Phase 13 既存 MemoryService + Phase 14 新 EvalService 共一 tonic Server 注册 — **verified by §9 verify run all-green + `test_serve_full_listens_all_planes` 类似集成测试 verify 5 services + 1 internal 全注册**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | 0014 migration + eval_runs 表 | core/migrations/0014_eval_runs.sql + integration | Ready |
| AC2 | SqliteEvalStore CRUD + JSON roundtrip | core/src/eval/store.rs + 5 unit tests | Ready |
| AC3 | EvalService 3 RPC + lifecycle | proto + data_plane/eval.rs + integration | Ready |
| AC4 | JSON nested fields type preserve | unit test | Ready |
| AC5 | cargo test --workspace 全绿 | §9 verify run | Ready |

## 8. Risks

- **`UpdateProgress` 是内部 RPC vs Console REST 暴露**：本 task 选「不在 router.go 暴露 REST endpoint，仅作为 Go-side EvalRunner 反向 callback」；但 Console UI 端 GET /v1/eval-runs/{id} 看到的 metrics / case_results 都来自 UpdateProgress 写入；trade-off 清晰：UpdateProgress 不在 Console contract 22 endpoint 内（Console UI 不调）
- **`config_snapshot` JSON 类型多样**：Console 端 contractv1.EvalRunCreate.ConfigSnapshot 是 `map[string]any`；Rust 端 serde_json::Value 处理任意类型；JSON serialize 不应丢类型；缓解 unit test test_json_roundtrip_preserves_types 真验证 float / int / nested object 等
- **case_results 数组大 → SQLite TEXT 列长度**：SQLite TEXT 上限实际是 1GB；100 case × 1KB/case = 100KB 远低于上限；trade-off 接受；如 v1.x 出现大 dataset → 评估 eval_case_results 子表 [SPEC-DEFER:phase-future.case-results-subtable]
- **CHECK constraint on status 拒绝未来新 status (e.g. "paused")**：本 task 选当前 4 个 status (running/succeeded/failed/cancelled)；如 future 加 paused → 通过 migration ALTER 重建 CHECK；trade-off 接受
- **DataPlaneStores 改 signature 破坏 Phase 11/12/13 既存调用**：与 task-13.1 同款风险；缓解 add `with_eval()` builder method + 既存 `new()` 用 NoOp eval store；§10 trade-off 评估

## 9. Verification Plan

- **install**: `cargo fetch`
- **lint**: `cargo fmt --check`
- **typecheck**: `cargo check -p contextforge-core`
- **unit-test**: `cargo test -p contextforge-core --lib eval::store::tests + data_plane::eval::tests`（≥6 单测全过）
- **integration**: `cargo test -p contextforge-core --test eval_integration`（≥2 集成全过）
- **e2e**: 通过 integration 实现
- **build**: `cargo build -p contextforge-core`
- **coverage**: 不强制
- **runtime-smoke**: `cargo run -p contextforge-core --bin contextforge-core -- 127.0.0.1:50552 /tmp/cf-test &` + `grpcurl -plaintext 127.0.0.1:50552 list | grep EvalService`
- **manual**: grpcurl describe EvalService 3 RPC + diff proto vs Go contractv1.EvalRun 字段命名

## 10. Completion Notes

<!-- 完工时按 standard.md §8.3 6 项 schema 回填 -->

- **完成日期**：<待填>
- **改动文件**：
  - `core/migrations/0014_eval_runs.sql` (新增 — 10 列 + 3 索引 + CHECK constraint)
  - `core/src/migrations.rs` (修改 — 注册 0014)
  - `core/src/eval/mod.rs` (新增 — 子 module 入口)
  - `core/src/eval/store.rs` (新增 — SqliteEvalStore + 5 method + 5 unit tests)
  - `core/src/eval/runner.rs` (新增 — EvalRunner 占位 + trigger_external stub)[SPEC-OWNER:task-14.2]
  - `proto/contextforge/console_data_plane/v1/console_data_plane.proto` (修改 — EvalService 3 RPC + 5 message)
  - `core/src/data_plane/eval.rs` (新增 — EvalServer + 3 RPC impl + 3 unit tests)
  - `core/src/data_plane/mod.rs` (修改 — DataPlaneStores 加 eval + register_services 加 EvalServer)
  - `core/src/server.rs` (修改 — serve_full 实例化 SqliteEvalStore)
  - `core/src/lib.rs` (修改 — `pub mod eval;`)
  - `core/tests/eval_integration.rs` (新增 — 2+ e2e tests)
  - `docs/specs/tasks/task-14.1-rust-eval-grpc-service.md` (本 spec §6 / §7 / §10 / Status 推进)
- **commit 列表**：
  - feat(core/eval): task-14.1 — eval_runs SQLite schema + SqliteEvalStore + EvalService gRPC 3 RPC
  - docs(spec): task-14.1 §6/§7/§10 / Status → Done
- **§9 Verification 结果**：<待填>
- **剩余风险 / 未做项**：
  - Go REST handlers + grpcclient.EvalClient + Go-side EvalRunner goroutine [SPEC-OWNER:task-14.2]
  - Rust native EvalRunner spawn_blocking [SPEC-DEFER:phase-future.rust-native-eval-runner]
  - Eval progress streaming [SPEC-DEFER:phase-15.eval-progress-streaming]
- **下游 task 影响**：task-14.2 用本 task EvalService 作 grpcclient 桥梁 + 实现 2 REST handler + 实现 Go-side goroutine 跑 recall harness + 调 UpdateProgress 反向 update
