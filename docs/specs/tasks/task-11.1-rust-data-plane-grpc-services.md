# Task `11.1`: `rust-data-plane-grpc-services — core/proto/console_data_plane.proto 4 service + core/src/data_plane/ tonic server`

**Status**: Ready

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 11 (console-real-data-plane)
**Dependencies**: task-10.2 (SqliteWorkspaceStore 已建) + task-10.3 (SqliteJobStore + JobRunner 框架已建) + task-9.1/9.2 (tonic + prost + `:48180` 端口模式) + [ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) D1/D2/D5

## 1. Background

[ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md) Phase 10 task-10.4 §10 Trade-off #1 显式记录 v0.3 console-api-serve 用 in-memory MemStore 模拟持久化（daemon 重启即丢失），原因是 Go 进程跨进程直接打开 Rust 写的 SQLite 文件需要新 R7 dep + 跨进程 WAL 边角 case 多。本 task 是 [ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) D2 在 ContextForge 仓内的解锁项 —— 先建 Rust 4 个新 gRPC service (`WorkspaceService` / `JobService` / `SearchService` / `EventsService`)，task-11.2 才能在其上实现 Go thin REST→gRPC translator。

复用 [ADR-013](../../decisions/adr-013-cli-data-plane-grpc-bridge.md) Phase 9 task-9.1/9.2 已建立的 tonic + prost + `:48180` 端口模式；新 service 注册到现有 `tonic::transport::Server::builder()` 链；proto 文件命名 `core/proto/console_data_plane.proto` 与 `proto/contextforge/v1/service.proto` (Phase 9 cli-data-plane Index gRPC) 分离 —— 避免与 Phase 9 已 freeze 的 Index gRPC 字段冲突 + 让 Console business plane 独立演进。

本 task 的 `JobService.Enqueue` 仅占位写 `status=queued` + 调 `JobRunner` 的现有 v0.3 stub（task-10.3 内 200ms tick 模拟）；真触发 `IndexSession::index_path_with_progress` 在 task-11.3 [SPEC-OWNER:task-11.3]。`SearchService.Query` 占位返 empty results；真接 retriever 在 task-11.4 [SPEC-OWNER:task-11.4]。`EventsService.Subscribe` 占位返 keepalive only；真接 EventBus broadcast 在 task-11.4 [SPEC-OWNER:task-11.4]。

## 2. Goal

`core/proto/console_data_plane.proto` 含 4 service × 14 RPC + 11 message 类型，1:1 镜像 Go `internal/contractv1/contractv1.go` JSON tag (snake_case)；`core/src/data_plane/` Rust module 含 4 个 tonic service trait 实现 + 接 `SqliteWorkspaceStore` (task-10.2) + `SqliteJobStore` + `JobRunner` 框架 (task-10.3 现有 v0.3 stub 行为)；contextforge-core daemon `serve` 子命令启动时把 4 service `add_service` 到现有 `:48180` tonic Server；`cargo test --workspace` 全绿（不破坏 task-10.3 现有 JobRunner 测试）；≥6 单元测试 + ≥2 集成测试（真 `tonic::transport::Server::bind` 真 TCP + tonic `Channel::from_static` client）。

## 3. Scope

### In Scope

- **新增 `core/proto/console_data_plane.proto`**：
  - 包声明 `package contextforge.console_data_plane.v1;`（与 Phase 9 `contextforge.v1` namespace 分离）
  - 4 service × 14 RPC：
    - `WorkspaceService` (4 RPC): `Create(CreateWorkspaceRequest) returns (Workspace)` / `Get(GetWorkspaceRequest) returns (Workspace)` / `List(ListWorkspacesRequest) returns (ListWorkspacesResponse)` / `Delete(DeleteWorkspaceRequest) returns (DeleteWorkspaceResponse)`
    - `JobService` (4 RPC): `Enqueue(EnqueueJobRequest) returns (IndexJob)` / `Get(GetJobRequest) returns (IndexJob)` / `Cancel(CancelJobRequest) returns (CancelJobResponse)` / `Stream(StreamJobsRequest) returns (stream IndexJob)`（reserved for v0.4.x; 本 task 占位实现返单条 keepalive）
    - `SearchService` (1 RPC): `Query(SearchRequest) returns (SearchResponse)`（SearchResponse = `{result: SearchResult, trace: RetrievalTrace}`，与 Console contractv1 嵌套约定一致）
    - `EventsService` (1 RPC): `Subscribe(SubscribeEventsRequest) returns (stream ObservabilityEvent)`（v0.4.x extension reserved 3 RPC: Recent/Filter/Replay）
    - Health (1 RPC): `Health(HealthRequest) returns (HealthResponse)` reused at top-level package（或单独 nested HealthService；spec 内允许两种 — task implementation 自决）
  - 11 message 类型：`Workspace` / `WorkspaceCreate` / `IndexJob` / `SearchRequest` / `SearchResult` / `RetrievalTrace` / `SourceChunk` / `Citation` / `ObservabilityEvent` / `CoreHealth` / `FieldAvailability`
  - 字段 snake_case + int64 for Unix epoch + string for enum-like status (`"queued"|"running"|"succeeded"|"failed"|"cancelled"`，**不**用 proto enum —— 与 Go `contractv1.IndexJob.Status` string 类型对齐)
  - 包注释引用 ADR-016 D2 + Console contractv1.go 单一事实源路径
- **修改 `core/build.rs`**：tonic_build 编译列表追加 `console_data_plane.proto`（复用 ADR-013 既有 pattern；不引入新 R7 dep）
- **新增 `core/src/data_plane/mod.rs`**：
  - `pub mod workspace; pub mod job; pub mod search; pub mod events;`
  - 顶层 `register_services(server: tonic::transport::Server, stores: Arc<DataPlaneStores>) -> tonic::transport::Server` helper 把 4 service `add_service` 到链
  - `DataPlaneStores` struct 持有 `Arc<SqliteWorkspaceStore>` + `Arc<SqliteJobStore>` + `Arc<JobRunner>` 引用，spawn 在 daemon 启动早期 + share across 4 service impl
- **新增 `core/src/data_plane/workspace.rs`**：
  - `pub struct WorkspaceServer { store: Arc<SqliteWorkspaceStore> }` impl tonic-generated `WorkspaceService` trait
  - Create/Get/List/Delete 4 method 真调 `SqliteWorkspaceStore` (复用 task-10.2 已实现的 CRUD)
  - 错误映射：`StoreError::NotFound` → `tonic::Status::not_found(...)`; `StoreError::Conflict` → `Status::failed_precondition(...)`; 其它 → `Status::internal(...)`
- **新增 `core/src/data_plane/job.rs`**：
  - `pub struct JobServer { store: Arc<SqliteJobStore>, runner: Arc<JobRunner> }` impl `JobService` trait
  - `Enqueue` 写 `status=queued` + 调 `JobRunner.spawn_blocking(stub_callback)` —— **本 task 仅占位使用 task-10.3 现有 v0.3 stub 行为**（真接 `IndexSession::index_path_with_progress` 在 [SPEC-OWNER:task-11.3]）
  - `Get` 真读 `SqliteJobStore`；`Cancel` 真设 `cancel_requested=true`；`Stream` 本 task 实现 keepalive only（每 1s emit current job state 然后 break；真完整 stream 在 [SPEC-OWNER:task-11.4]）
- **新增 `core/src/data_plane/search.rs`**：
  - `pub struct SearchServer { /* 占位 */ }` impl `SearchService` trait
  - `Query` 本 task 返 `SearchResult { items: vec![] }` + `RetrievalTrace { retrieved_chunks: vec![] }`（真接 retriever 在 [SPEC-OWNER:task-11.4]）
  - **必须有显式 TODO 注释 + verified by task-11.4 §6 AC1 锚点**
- **新增 `core/src/data_plane/events.rs`**：
  - `pub struct EventsServer { /* 占位 broadcast channel sender */ }` impl `EventsService` trait
  - `Subscribe` 本 task 返 keepalive only（每 5s emit 1 个 `ObservabilityEvent { event_type: "core.keepalive", ts_unix: now }` 然后 break）；真接 `JobRunner` progress 在 [SPEC-OWNER:task-11.4]
- **修改 `core/src/bin/contextforge_core.rs`** 或 daemon `serve` 子命令入口：
  - 启动时实例化 `Arc<SqliteWorkspaceStore>` + `Arc<SqliteJobStore>` + `Arc<JobRunner>`（复用 task-10.2/10.3 既有 init 链）
  - 调 `data_plane::register_services(server, stores)` 把 4 service 注册到 `:48180` tonic Server（与 Phase 9 Index gRPC 同一 Server::builder 链）
- **单元测试 ≥6**：
  - `test_proto_field_snake_case_consistency` (grep `core/src/data_plane/workspace.rs` 字段名 vs Go `contractv1.go` JSON tag)
  - `test_workspace_server_create_via_service` (in-process tonic + SqliteWorkspaceStore in tempdir)
  - `test_workspace_server_get_404` (NotFound → tonic::Status::not_found)
  - `test_job_server_enqueue_writes_queued` (SqliteJobStore in tempdir + assert status="queued")
  - `test_job_server_cancel_sets_flag` (cancel_requested=true)
  - `test_search_server_empty_response` (返 empty items + empty retrieved_chunks)
  - `test_events_server_keepalive` (stream emit 1 keepalive 然后 close)
  - `test_register_services_adds_4_services` (Server::builder() 状态检查)
- **集成测试 ≥2**（真 TCP listener + tonic Channel）：
  - `core/tests/data_plane_integration.rs::test_workspace_crud_via_grpc`：spawn `tonic::transport::Server::serve_with_incoming` 真 net listener → tonic Channel::from_static → Create/Get/List/Delete 真调
  - `core/tests/data_plane_integration.rs::test_job_enqueue_get_cancel`：同上 + JobService 4 method 真走 SqliteJobStore（status=queued + Get 返 same job + Cancel 后 cancel_requested=true）
- **文件锚点**：`core/proto/console_data_plane.proto` + `core/src/data_plane/{mod,workspace,job,search,events}.rs` + `core/src/bin/contextforge_core.rs` 入口修改 + `core/tests/data_plane_integration.rs`
- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **JobService.Enqueue 真触发 IndexSession::index_path_with_progress** [SPEC-OWNER:task-11.3]：本 task 仅占位用 task-10.3 现有 v0.3 stub
- **SearchService.Query 真接 retriever** [SPEC-OWNER:task-11.4]：本 task 仅占位返 empty results；显式 TODO 注释
- **EventsService.Subscribe 真接 JobRunner progress** [SPEC-OWNER:task-11.4]：本 task 仅占位返 keepalive
- **Go client / handler 改动** [SPEC-OWNER:task-11.2]：本 task 仅 Rust 侧；Go grpcclient + handler 重构在 task-11.2
- **新增 SQLite migration** [SPEC-DEFER:task-future.console-endpoint-expansion]：复用 task-10.2 `0010_workspaces.sql` + task-10.3 `0011_index_jobs.sql`；本 task 不引入新 schema
- **gRPC streaming filters / replay / since=event_id** [SPEC-DEFER:console-endpoint-expansion]：v0.4.1 增量
- **JobService.Stream 完整 multi-job server stream** [SPEC-OWNER:task-11.4]：本 task 仅占位 keepalive 路径

## 4. Users / Actors

- **task-11.2 go-rest-to-grpc-proxy 实施 agent**（下游）：消费本 task 产出的 4 个 service stub 作为 Go grpcclient 桥梁
- **task-11.3 indexjob-real-runner-wiring 实施 agent**（下游）：在本 task 的 `JobServer` 基础上把 `Enqueue` 真接 `IndexSession::index_path_with_progress`
- **task-11.4 search-real-retriever-and-events 实施 agent**（下游）：在本 task 的 `SearchServer` + `EventsServer` 基础上真接 retriever + EventBus broadcast channel

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md` §D1 / §D2 / §D5
- `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md` （`:48180` 端口模式 + tonic + prost 工具链）
- `docs/specs/phases/phase-11-console-real-data-plane.md`
- `docs/specs/tasks/task-10.2-workspace-resource.md` (SqliteWorkspaceStore 接口)
- `docs/specs/tasks/task-10.3-indexjob-resource.md` (SqliteJobStore + JobRunner 框架)
- `docs/specs/tasks/task-9.1-proto-index-rpc.md` (tonic + prost 工具链已建)
- `docs/specs/tasks/task-9.2-rust-grpc-index.md` (Server::builder pattern)
- `H:/devlopment/code/contextforge/internal/contractv1/contractv1.go` （字段命名 single source of truth）

### 5.2 Imports

- **Rust**: 现有 `tonic = "0.12"` + `prost = "0.13"` + `prost-types = "0.13"` + `tonic-build = "0.12"`（Cargo.toml 已有；ADR-013 引入）；复用现有 `tokio` + `rusqlite` + `core/src/workspace/` + `core/src/jobs/`
- **不引入新依赖**：R7 不触发；`Cargo.toml` 不动

### 5.3 函数签名

```rust
// core/src/data_plane/mod.rs
pub mod workspace;
pub mod job;
pub mod search;
pub mod events;

use std::sync::Arc;
use tonic::transport::server::Router;

pub struct DataPlaneStores {
    pub workspace_store: Arc<crate::workspace::SqliteWorkspaceStore>,
    pub job_store: Arc<crate::jobs::SqliteJobStore>,
    pub job_runner: Arc<crate::jobs::JobRunner>,
}

pub fn register_services(
    server: tonic::transport::Server,
    stores: Arc<DataPlaneStores>,
) -> Router {
    server
        .add_service(workspace::WorkspaceServer::new(stores.clone()).into_service())
        .add_service(job::JobServer::new(stores.clone()).into_service())
        .add_service(search::SearchServer::new(stores.clone()).into_service())
        .add_service(events::EventsServer::new(stores).into_service())
}

// core/src/data_plane/workspace.rs
pub struct WorkspaceServer { stores: Arc<DataPlaneStores> }
#[tonic::async_trait]
impl proto::workspace_service_server::WorkspaceService for WorkspaceServer { /* 4 method */ }

// 同理 job.rs / search.rs / events.rs
```

```proto
// core/proto/console_data_plane.proto
syntax = "proto3";
package contextforge.console_data_plane.v1;

service WorkspaceService {
  rpc Create(CreateWorkspaceRequest) returns (Workspace);
  rpc Get(GetWorkspaceRequest) returns (Workspace);
  rpc List(ListWorkspacesRequest) returns (ListWorkspacesResponse);
  rpc Delete(DeleteWorkspaceRequest) returns (DeleteWorkspaceResponse);
}
service JobService {
  rpc Enqueue(EnqueueJobRequest) returns (IndexJob);
  rpc Get(GetJobRequest) returns (IndexJob);
  rpc Cancel(CancelJobRequest) returns (CancelJobResponse);
  rpc Stream(StreamJobsRequest) returns (stream IndexJob);
}
service SearchService {
  rpc Query(SearchRequest) returns (SearchResponse);
}
service EventsService {
  rpc Subscribe(SubscribeEventsRequest) returns (stream ObservabilityEvent);
}

message Workspace {
  string workspace_id = 1;
  string name = 2;
  string root_path = 3;
  string status = 4;       // "ready" | "indexing" | "error"
  int64 created_at_unix = 5;
  int64 updated_at_unix = 6;
  // 其余字段 1:1 镜像 Go contractv1.Workspace JSON tag
}
// 同理 11 message 类型
```

## 6. Acceptance Criteria

- [ ] AC1：`core/proto/console_data_plane.proto` 含 4 service × 14 RPC + 11 message 类型；字段命名 snake_case 与 Go `internal/contractv1/contractv1.go` JSON tag 1:1；包声明 `contextforge.console_data_plane.v1` — **verified by unit-test step `cargo test -p contextforge-core --test data_plane_integration -- test_proto_field_snake_case_consistency` + grpcurl describe (cmd: `grpcurl -plaintext 127.0.0.1:48180 describe contextforge.console_data_plane.v1.WorkspaceService`)**
- [ ] AC2：tonic server 启动时 4 service 全注册可见 (`Server::builder().add_service(...)` × 4)；`register_services` helper 真返 `Router` 含 4 service — **verified by unit-test step `cargo test -p contextforge-core --lib data_plane -- test_register_services_adds_4_services`**
- [ ] AC3：`WorkspaceService.Create/Get/List/Delete` 真走 `SqliteWorkspaceStore` 持久化（task-10.2 既有 CRUD）+ 错误映射 `NotFound`→`not_found` / `Conflict`→`failed_precondition` / Internal→`internal` — **verified by integration-test step `cargo test -p contextforge-core --test data_plane_integration -- test_workspace_crud_via_grpc`**
- [ ] AC4：`JobService.Enqueue/Get/Cancel` 真走 `SqliteJobStore` (status=queued / Get 同 job / Cancel 设 cancel_requested=true)；`JobService.Stream` 占位 keepalive only — **verified by integration-test step `cargo test -p contextforge-core --test data_plane_integration -- test_job_enqueue_get_cancel`**
- [ ] AC5：contextforge-core daemon `serve` 子命令启动后 `:48180` 真监听 + 4 service 注册到 tonic Server；`cargo test --workspace` 全绿（不破坏 task-10.3 现有 JobRunner 测试） — **verified by typecheck + unit-test phase smoke + integration `test_daemon_listens_data_plane`**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | proto 4 service × 14 RPC + 11 message | core/proto/console_data_plane.proto + test_proto_field_snake_case_consistency | Ready |
| AC2 | tonic server 注册 + Router helper | core/src/data_plane/mod.rs + test_register_services_adds_4_services | Ready |
| AC3 | WorkspaceService CRUD 真走 SqliteWorkspaceStore | core/src/data_plane/workspace.rs + test_workspace_crud_via_grpc | Ready |
| AC4 | JobService 4 method 真走 SqliteJobStore | core/src/data_plane/job.rs + test_job_enqueue_get_cancel | Ready |
| AC5 | daemon serve 启动 + 不退化 | core/src/bin/contextforge_core.rs + cargo test --workspace | Ready |

## 8. Risks

- **`.proto` 字段命名与 Go contractv1 不齐**：cross-repo D3 thin proxy 破坏；缓解 `test_proto_field_snake_case_consistency` 单测 grep + manual diff Console contractv1.go
- **`Arc<Mutex<rusqlite::Connection>>` clone 进 spawn_blocking 闭包**：task-10.3 同坑；缓解先 `clone Arc` 再 move into closure，不在闭包内 lock
- **proto status 用 string 而非 enum**：Go `contractv1.IndexJob.Status` 是 string；如果用 proto enum 反序列化时 prost 生成 i32 → Go 端需要枚举 cast → 破坏 D3 thin proxy；本 task 选 string，trade-off 接受
- **复用 ADR-013 既有 `:48180` 端口与 Phase 9 Index gRPC 共存**：tonic Server::builder 允许 add_service 多 service；命名空间隔离 (`contextforge.v1` vs `contextforge.console_data_plane.v1`) 避免冲突
- **tonic-build 编译时 proto 路径解析**：`build.rs` 必须正确指向 `core/proto/console_data_plane.proto`；ADR-013 既有 pattern 是 `tonic_build::configure().compile(&["proto/contextforge/v1/*.proto"], &["proto/"])` —— 新文件路径需 separately add

## 9. Verification Plan

- **install**: `cargo fetch`
- **lint**: `cargo fmt --check -- core/src/data_plane/`
- **typecheck**: `cargo check -p contextforge-core`
- **unit-test**: `cargo test -p contextforge-core --lib data_plane`（≥6 单测全过）
- **integration**: `cargo test -p contextforge-core --test data_plane_integration`（≥2 集成全过）
- **e2e**: 通过 integration 实现
- **build**: `cargo build -p contextforge-core`
- **coverage**: 不强制（task-10.3 同款；新 module 单测 + 集成覆盖 + tonic generated code 不计）
- **runtime-smoke**: `cargo run -p contextforge-core --bin contextforge-core -- serve --data-dir /tmp/cf-test &` + `grpcurl -plaintext 127.0.0.1:48180 list | grep ConsoleDataPlane`
- **manual**: grpcurl describe 4 service + diff `core/proto/console_data_plane.proto` 字段命名 vs `internal/contractv1/contractv1.go` JSON tag

## 10. Completion Notes

<!-- 完工时按 standard.md §8.3 6 项 schema 回填 -->

- **完成日期**：<待回填 — task 完工时填 YYYY-MM-DD>
- **改动文件**：<待回填>
- **commit 列表**：<待回填>
- **§9 Verification 结果**：<待回填>
- **剩余风险 / 未做项**：
  - JobService.Enqueue 真触发 IndexSession [SPEC-OWNER:task-11.3]
  - SearchService.Query 真接 retriever [SPEC-OWNER:task-11.4]
  - EventsService.Subscribe 真接 EventBus [SPEC-OWNER:task-11.4]
  - JobService.Stream 完整 multi-job server stream [SPEC-OWNER:task-11.4]
- **下游 task 影响**：task-11.2 用本 task 4 service 作 grpcclient 桥梁；task-11.3 / 11.4 在本 task service stub 基础上替换为真接通
