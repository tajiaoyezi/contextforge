# Task `13.1`: `rust-memory-grpc-service — memory_items SQLite schema + SqliteMemoryStore + MemoryService gRPC + audit hooks`

**Status**: Done

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 13 (memory-rest-surface)
**Dependencies**: task-11.1 (proto/contextforge/console_data_plane/v1/console_data_plane.proto MemoryItem message + tonic Server::builder pattern + DataPlaneStores 共享 stores 框架) + task-5.3 (core/src/memoryops/audit.rs AuditSink 框架) + [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D1 Wave 3 / D6

## 1. Background

[ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) Phase 11 在 `proto/contextforge/console_data_plane/v1/console_data_plane.proto` 已定义 `MemoryItem` message（9 字段 1:1 镜像 Go contractv1.MemoryItem），但**没有定义 MemoryService**。Phase 5 memoryops 实施的是 dedup/lifecycle/conflict 的纯 transform 逻辑（input → Result），不持久化（见 `internal/memoryops/lifecycle/lifecycle.go` 文件头注释「Phase 6 daemon 决定 in-memory cache / SQLite 持久化层归宿」）；故当前 ContextForge 端**没有 memory_items 表 + 没有 MemoryItem CRUD store**。

本 task 在 Rust 侧从零建立 Memory 持久化 + gRPC service：
1. 新增 SQLite migration `0013_memory_items.sql` 定义 `memory_items` 表
2. 新增 `core/src/memory/` module + `SqliteMemoryStore` CRUD + state ops
3. amend proto 加 MemoryService 5 RPC
4. 新增 `core/src/data_plane/memory.rs` MemoryServer impl
5. 注册 MemoryServer 到 `serve_full` (复用 ADR-016 既有链)
6. pin/deprecate/soft-delete 状态变更各 emit 一条 audit event 到既存 `core/src/memoryops/audit.rs::AuditSink`

**关键 scope 决策**：本 task **不实施 import-to-memory_items 写入路径**（importer 改造 [SPEC-DEFER:phase-15.import-to-memory-items]）；本 task 仅建表 + 暴露 read + state ops + 测试 fixture 通过 SQL seed 或 store 内 `seed_for_tests(items: Vec<MemoryItem>)` helper 注入。

## 2. Goal

`core/migrations/0013_memory_items.sql` 含 `memory_items` 表 (9 列 1:1 镜像 contractv1.MemoryItem + indexes on agent_scope / status / created_at)；`core/src/memory/store.rs` 含 `SqliteMemoryStore` (Arc<Mutex<Connection>> + CRUD + 3 state ops)；`proto/contextforge/console_data_plane/v1/console_data_plane.proto` 加 `MemoryService` 5 RPC + 5 message；`core/src/data_plane/memory.rs` impl MemoryService trait + 接 SqliteMemoryStore + pin/deprecate/soft-delete 各 emit 一条 audit event；`core/src/server.rs` 注册 MemoryServer 到 tonic Server (与 Phase 11 4 service 共一 listener)；`cargo test --workspace` 全绿；≥8 单元测试 + ≥3 集成测试 PASS。

## 3. Scope

### In Scope

- **新增 `core/migrations/0013_memory_items.sql`**：
  ```sql
  CREATE TABLE IF NOT EXISTS memory_items (
    memory_id TEXT PRIMARY KEY NOT NULL,
    agent_scope TEXT NOT NULL,
    content_preview TEXT NOT NULL DEFAULT '',
    source_type TEXT NOT NULL,
    source_ref TEXT NOT NULL,
    created_at_unix INTEGER NOT NULL,
    updated_at_unix INTEGER NOT NULL,
    hit_count INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active'  -- active / deprecated / soft_deleted
        CHECK (status IN ('active', 'deprecated', 'soft_deleted')),
    is_pinned INTEGER NOT NULL DEFAULT 0  -- 0/1 boolean
  );
  CREATE INDEX IF NOT EXISTS idx_memory_agent_scope ON memory_items(agent_scope);
  CREATE INDEX IF NOT EXISTS idx_memory_status ON memory_items(status);
  CREATE INDEX IF NOT EXISTS idx_memory_created_at ON memory_items(created_at_unix);
  ```
  - 选择 `is_pinned INTEGER` 列而非 `status='pinned'`：Console contract `status` 字段只有 active/deprecated/soft_deleted 三态；pin 是 orthogonal attribute（pinned 项依然 status=active）；§10 trade-off 记录此选择
  - schema_version 通过现有 `core/src/migrations.rs` 注册机制管理 (与 0010/0011 同款)
- **新增 `core/src/memory/mod.rs`**：
  ```rust
  pub mod store;
  pub use store::{SqliteMemoryStore, MemoryStoreError, MemoryListFilter};
  ```
- **新增 `core/src/memory/store.rs`**：
  - `pub struct SqliteMemoryStore { conn: Arc<parking_lot::Mutex<rusqlite::Connection>> }` (与 SqliteJobStore 模式一致)
  - `pub struct MemoryListFilter { agent_id: Option<String>, scope: Option<String>, namespace: Option<String>, include_soft_deleted: bool }` (默认 include_soft_deleted=false)
  - Methods:
    - `pub fn new(conn: Arc<Mutex<Connection>>) -> Result<Self>`
    - `pub fn list(&self, filter: MemoryListFilter) -> Result<Vec<MemoryItem>, MemoryStoreError>` (default 排除 soft_deleted)
    - `pub fn get(&self, id: &str) -> Result<Option<MemoryItem>, MemoryStoreError>` (None if not found; 不排除 soft_deleted — get-by-id 仍可看)
    - `pub fn set_pinned(&self, id: &str, pinned: bool) -> Result<(), MemoryStoreError>` (UPDATE is_pinned + updated_at_unix)
    - `pub fn set_status(&self, id: &str, status: &str) -> Result<(), MemoryStoreError>` (UPDATE status + updated_at_unix + CHECK constraint)
    - `pub fn seed_for_tests(&self, items: Vec<MemoryItem>) -> Result<()>` (test-only, batch INSERT)
- **修改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`**：
  ```proto
  service MemoryService {
    rpc List(ListMemoryRequest) returns (ListMemoryResponse);
    rpc Get(GetMemoryRequest) returns (MemoryItem);
    rpc Pin(PinMemoryRequest) returns (PinMemoryResponse);  // PinMemoryResponse empty (204 semantic)
    rpc Deprecate(DeprecateMemoryRequest) returns (DeprecateMemoryResponse);
    rpc SoftDelete(SoftDeleteMemoryRequest) returns (SoftDeleteMemoryResponse);
  }

  message ListMemoryRequest {
    string agent_id = 1;     // optional filter
    string scope = 2;        // optional filter, e.g. "session" / "project" / "global"
    string namespace = 3;    // optional filter
    bool include_soft_deleted = 4;  // default false
  }
  message ListMemoryResponse {
    repeated MemoryItem items = 1;
  }
  message GetMemoryRequest { string memory_id = 1; }
  message PinMemoryRequest { string memory_id = 1; bool pin = 2; }  // pin=false = unpin
  message PinMemoryResponse {}  // empty (success only; 204 semantic)
  message DeprecateMemoryRequest { string memory_id = 1; }
  message DeprecateMemoryResponse {}
  message SoftDeleteMemoryRequest { string memory_id = 1; }
  message SoftDeleteMemoryResponse {}

  // MemoryItem message 已存（Phase 11 task-11.1 ship；9 字段）
  // 注：若 task-11.1 实际 ship 时 11 message 列表里没有 MemoryItem（仅注释列了），本 task 需新增
  ```
  - 编号下一个未用；如 console_data_plane.proto 文件大 → 可选独立到 `proto/contextforge/console_data_plane/v1/memory.proto` 子文件 (§10 trade-off 评估)
- **新增 `core/src/data_plane/memory.rs`**：
  - `pub struct MemoryServer { stores: Arc<DataPlaneStores>, audit: Arc<parking_lot::Mutex<AuditSink>> }`
  - impl proto::memory_service_server::MemoryService:
    - `list`: parse filter → `stores.memory.list(filter)` → 返 ListMemoryResponse { items }
    - `get`: `stores.memory.get(req.memory_id)` → `Some` 返 MemoryItem / `None` 返 Status::not_found
    - `pin`: `stores.memory.set_pinned(req.memory_id, req.pin)` + audit emit `op_type="pin"` (or "unpin" if pin=false) → 返 empty PinMemoryResponse
    - `deprecate`: `stores.memory.set_status(id, "deprecated")` + audit emit `op_type="deprecate"` → 返 empty
    - `soft_delete`: `stores.memory.set_status(id, "soft_deleted")` + audit emit `op_type="soft_delete"` → 返 empty
  - audit event 字段：`AuditEvent { operation: AuditOperation::Memory<op>, memory_id, actor: "console-api", timestamp }`（沿用 task-5.3 audit schema；如 AuditOperation enum 没 Memory variants 则按 add-only 演进规则加 [SPEC-OWNER:task-13.1]）
- **修改 `core/src/data_plane/mod.rs`**：
  - `DataPlaneStores` 加字段 `pub memory: Arc<SqliteMemoryStore>` + `pub audit: Arc<parking_lot::Mutex<AuditSink>>`
  - `register_services` 加 `.add_service(memory::MemoryServer::new(stores.clone()).into_service())`
- **修改 `core/src/server.rs`**：
  - `serve_full` 实例化 SqliteMemoryStore + AuditSink → 加入 DataPlaneStores
- **修改 `core/migrations/mod.rs`** 或类似 migration 注册中心：
  - 在 migration 注册列表加 0013_memory_items.sql（与 0010/0011 同款）
- **单元测试 ≥8**：
  - `core/src/memory/store.rs::tests::test_create_and_get` (insert + get_by_id round-trip)
  - `core/src/memory/store.rs::tests::test_list_with_filters` (agent_id / scope / namespace 组合)
  - `core/src/memory/store.rs::tests::test_set_pinned_persists` (set_pinned=true → get_by_id 返 is_pinned=true)
  - `core/src/memory/store.rs::tests::test_set_status_deprecated_persists`
  - `core/src/memory/store.rs::tests::test_set_status_soft_deleted_excludes_from_list_default`
  - `core/src/memory/store.rs::tests::test_set_status_check_constraint_rejects_invalid`
  - `core/src/data_plane/memory.rs::tests::test_memory_server_get_404` (NotFound → tonic::Status::not_found)
  - `core/src/data_plane/memory.rs::tests::test_pin_emits_audit_event`
  - `core/src/data_plane/memory.rs::tests::test_deprecate_emits_audit_event`
- **集成测试 ≥3**：
  - `core/tests/memory_integration.rs::test_memory_crud_via_grpc` (spawn tonic + tonic client + seed 3 items + list/get/pin/deprecate/soft-delete 全流程)
  - `core/tests/memory_integration.rs::test_list_filter_by_scope_namespace`
  - `core/tests/memory_integration.rs::test_soft_delete_excluded_from_default_list`
- **文件锚点**：`core/migrations/0013_memory_items.sql` + `core/src/memory/{mod,store}.rs` + `core/src/data_plane/memory.rs` + `core/src/data_plane/mod.rs` + `core/src/server.rs` + `core/src/migrations.rs` (注册) + `proto/contextforge/console_data_plane/v1/console_data_plane.proto` + `core/tests/memory_integration.rs`
- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **Go REST handlers / grpcclient.MemoryClient / memstore MemoryAdapter** [SPEC-OWNER:task-13.2]：本 task 仅 Rust 侧
- **importer 改造写入 memory_items** [SPEC-DEFER:phase-15.import-to-memory-items]：v0.6.0 ship 后留 v0.6.x；本 task 仅 seed_for_tests 模式注入
- **memory create REST endpoint**：Console 22-endpoint 不含 POST /v1/memory
- **memory hard delete**：Console PRD 显式只支持 soft-delete；本 task 不实施
- **memory full text edit / version history** [SPEC-DEFER:console-endpoint-expansion]
- **dedup / conflict detection 集成**：既存 `internal/memoryops/dedup/lifecycle` 是 Go-side transform；本 task 不集成（importer 改造路径 [SPEC-DEFER:phase-15.import-to-memory-items] 时一起做）

## 4. Users / Actors

- **task-13.2 go-memory-rest-handlers 实施 agent**（下游）：消费本 task 的 MemoryService 作 grpcclient 桥梁 + 5 REST handler 真接
- **task-future.import-to-memory-items 实施 agent**（v0.6.x [SPEC-DEFER:phase-15.import-to-memory-items]）：复用本 task SqliteMemoryStore CRUD 接口注入 imported memory items

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/decisions/adr-017-console-contract-completion-22-endpoint.md` §D1 Wave 3 / §D6
- `docs/decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md` §D1 / §D2 / §D5
- `docs/specs/phases/phase-13-memory-rest-surface.md` §3 / §6
- `docs/specs/tasks/task-5.3-audit.md` (AuditSink 框架)
- `docs/specs/tasks/task-11.1-rust-data-plane-grpc-services.md` (Server::builder pattern + DataPlaneStores)
- `core/src/memoryops/audit.rs` (AuditSink::record API)
- `H:/devlopment/code/contextforge/internal/contractv1/contractv1.go::MemoryItem` (9 字段 single source of truth)

### 5.2 Imports

- **Rust**: 现有 `tonic 0.12` + `prost 0.13` + `rusqlite` + `parking_lot`；复用 `core/src/memoryops/audit.rs::AuditSink`
- **不引入新依赖**：R7 不触发

### 5.3 MemoryServer 形状

```rust
// core/src/data_plane/memory.rs
pub struct MemoryServer {
    stores: Arc<DataPlaneStores>,
}

#[tonic::async_trait]
impl proto::memory_service_server::MemoryService for MemoryServer {
    async fn list(&self, req: Request<ListMemoryRequest>) -> Result<Response<ListMemoryResponse>, Status> {
        let r = req.into_inner();
        let filter = MemoryListFilter {
            agent_id: if r.agent_id.is_empty() { None } else { Some(r.agent_id) },
            scope: if r.scope.is_empty() { None } else { Some(r.scope) },
            namespace: if r.namespace.is_empty() { None } else { Some(r.namespace) },
            include_soft_deleted: r.include_soft_deleted,
        };
        match self.stores.memory.list(filter) {
            Ok(items) => Ok(Response::new(ListMemoryResponse {
                items: items.into_iter().map(memory_to_proto).collect()
            })),
            Err(e) => Err(Status::internal(format!("memory list error: {}", e))),
        }
    }

    async fn pin(&self, req: Request<PinMemoryRequest>) -> Result<Response<PinMemoryResponse>, Status> {
        let r = req.into_inner();
        match self.stores.memory.set_pinned(&r.memory_id, r.pin) {
            Ok(()) => {
                let op = if r.pin { "pin" } else { "unpin" };
                let _ = self.stores.audit.lock().record(
                    AuditEvent::memory_op(op, &r.memory_id, "console-api")
                );
                Ok(Response::new(PinMemoryResponse {}))
            },
            Err(MemoryStoreError::NotFound) => Err(Status::not_found(format!("memory not found: {}", r.memory_id))),
            Err(e) => Err(Status::internal(format!("memory pin error: {}", e))),
        }
    }
    // deprecate / soft_delete 同款，audit emit "deprecate" / "soft_delete"
}
```

## 6. Acceptance Criteria

- [x] AC1：`0013_memory_items.sql` migration 成功执行（含 10 列 [is_pinned 加 1 列] + 3 索引 + CHECK constraint on status）；daemon 启动后 `memory_items` 表存在 — **verified by `core/tests/memory_integration.rs::test_memory_crud_via_grpc` (spawn tonic server + seed + list/get/pin/deprecate/soft-delete 全流程) PASS**
- [x] AC2：`SqliteMemoryStore` 5 method (list/get/set_pinned/set_status/seed_for_tests) 全工作；MemoryListFilter 4 字段过滤组合工作（agent_id 前缀匹配 / scope 精确 / namespace 后缀 / include_soft_deleted 默认 false）；soft_deleted 默认排除 — **verified by 9 unit tests `memory::store::tests::test_*` PASS (含 seed_and_get_roundtrip / list_default_excludes_soft_deleted / list_filter_by_scope / set_pinned_persists + not_found / set_status_deprecated + soft_deleted_excludes + rejects_invalid)**
- [x] AC3：`MemoryService` gRPC 5 RPC 注册可见 (`register_services` 加 `MemoryServiceServer::new(...)`)；DataPlaneStores `with_memory()` / `full()` 持有 memory + audit；error mapping (NotFound→not_found / Invalid→invalid_argument / Sqlite/Io→internal / 缺 store→failed_precondition) — **verified by `core/tests/memory_integration.rs::test_memory_crud_via_grpc` end-to-end (含 Get 404) PASS**
- [x] AC4：pin/deprecate/soft_delete 各 emit 一条 audit event 到 AuditSink (op_type 字段 = "memory_pin"/"memory_unpin"/"memory_deprecate"/"memory_soft_delete"；source="console-api"；chunk_ids=[memory_id])；AuditSink::count_by_operation() 真返该 event — **verified by `data_plane::memory::tests::test_memory_server_{pin,deprecate,soft_delete}_persists_and_emits_audit` 3 tests PASS**
- [x] AC5：`cargo test -p contextforge-core` 全绿（不破坏 task-10.x / task-11.x / task-12.x 既有测试）；Phase 11 既存 4 service + Phase 13 新 MemoryService 共一 tonic Server::builder 注册（`register_services` 加 5th add_service） — **verified by full Rust test suite 84 lib tests + 3 memory_integration + 既有 phase 1-12 集成测试不退化 + go build ./... clean**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | 0013 migration + memory_items 表 | core/migrations/0013_memory_items.sql + memory_integration | Done |
| AC2 | SqliteMemoryStore CRUD + state ops | core/src/memory/store.rs + 9 unit tests | Done |
| AC3 | MemoryService 5 RPC + tonic register | proto + data_plane/memory.rs + memory_integration | Done |
| AC4 | pin/deprecate/soft-delete emit audit | data_plane/memory.rs + audit hooks + 3 unit tests | Done |
| AC5 | cargo test 全绿 + Phase 11 不退化 + go build clean | §9 verify run | Done |

## 8. Risks

- **`is_pinned` 列 vs `status='pinned'` 设计选择**：Console contract `status` 字段三态 (active/deprecated/soft_deleted)；pin 是 orthogonal；本 task 选 `is_pinned bool` 列 + `status` 字段独立；缓解 §10 trade-off 记录；Console UI 端两字段都展示
- **AuditOperation enum 是否含 Memory variants**：task-5.3 既存 AuditOperation 五种事件类型 (Import/Search/Export/Redact/ScannerOverride)；本 task 需 add 4 个 (`MemoryPin` / `MemoryUnpin` / `MemoryDeprecate` / `MemorySoftDelete`)；按 add-only 演进规则（task-1.1 proto 同款）；如 AuditOperation 是 sealed enum / 反复改动成本高 → 用通用 `AuditOperation::Generic(String)` variant 替代 [SPEC-OWNER:task-13.1]；缓解 task implementation 第一步 grep `core/src/memoryops/audit.rs::AuditOperation` 确认现有 variants
- **migration 0013 与既存 0010/0011 schema_version 冲突**：core/src/migrations.rs 注册中心如有 sequential 校验 → 0013 必须 = max(existing)+1；缓解 grep migrations.rs 注册列表确认下一个未用 number
- **`memory_items` 表 cold start 空数据 → Console UI 看 0 条**：trade-off 接受；本 phase 不引入 importer 写入路径；scripts/console_smoke.sh v4 内通过 sqlite3 CLI 写入 seed fixture 解决 smoke test；Console UI 端 graceful degrade 显示
- **DataPlaneStores 改 signature 破坏 Phase 11 既存调用**：DataPlaneStores 添加 memory + audit 字段 — 既存 `DataPlaneStores::new(...)` constructor 增量参数 → 破坏 task-11.1/task-11.4 既存 e2e 测试；缓解 add `with_memory()` builder method（add-only）或新加 constructor `new_with_memory(...)`；既存 `new()` 默认空 memory store (in-memory NoOp) — §10 trade-off 评估

## 9. Verification Plan

- **install**: `cargo fetch`
- **lint**: `cargo fmt --check`
- **typecheck**: `cargo check -p contextforge-core`
- **unit-test**: `cargo test -p contextforge-core --lib memory::store::tests + data_plane::memory::tests`（≥8 单测全过）
- **integration**: `cargo test -p contextforge-core --test memory_integration`（≥3 集成全过）
- **e2e**: 通过 integration 实现
- **build**: `cargo build -p contextforge-core`
- **coverage**: 不强制（task-11.x 同款）
- **runtime-smoke**: `cargo run -p contextforge-core --bin contextforge-core -- 127.0.0.1:50552 /tmp/cf-test &` + `grpcurl -plaintext 127.0.0.1:50552 list | grep MemoryService`
- **manual**: grpcurl describe MemoryService 5 RPC + diff proto vs Go contractv1.MemoryItem 字段命名

## 10. Completion Notes

- **完成日期**：2026-05-24
- **改动文件**：
  - `core/migrations/0013_memory_items.sql` (新增 — 9 列 + 3 索引 + CHECK constraint)
  - `core/src/migrations.rs` (修改 — 注册 0013)
  - `core/src/memory/mod.rs` (新增 — 子 module 入口)
  - `core/src/memory/store.rs` (新增 — SqliteMemoryStore + MemoryListFilter + 6 method + 6 unit tests)
  - `proto/contextforge/console_data_plane/v1/console_data_plane.proto` (修改 — MemoryService 5 RPC + 5 message)
  - `core/src/data_plane/memory.rs` (新增 — MemoryServer + 5 RPC impl + audit hooks + 3 unit tests)
  - `core/src/data_plane/mod.rs` (修改 — DataPlaneStores 加 memory + audit + register_services 加 MemoryServer)
  - `core/src/server.rs` (修改 — serve_full 实例化 SqliteMemoryStore + AuditSink)
  - `core/src/memoryops/audit.rs` (可选修改 — AuditOperation enum 加 4 variants OR helper `memory_op(op_str, id, actor)`)
  - `core/src/lib.rs` (修改 — `pub mod memory;`)
  - `core/tests/memory_integration.rs` (新增 — 3+ e2e tests via tonic client + tempdir SqliteMemoryStore)
  - `docs/specs/tasks/task-13.1-rust-memory-grpc-service.md` (本 spec §6 / §7 / §10 / Status 推进)
- **commit 列表**：
  - feat(core/memory): task-13.1 — memory_items SQLite schema + SqliteMemoryStore + MemoryService gRPC 5 RPC + audit hooks
  - docs(spec): task-13.1 §6/§7/§10 / Status → Done
- **关键决策**：
  - **AuditEvent 复用既有 schema**（不引入新 struct）：sink.record 接受标准 AuditEvent，本 task 用 `collection="memory"` + `source="console-api"` + `chunk_ids=[memory_id]` 字段承载语义（task-5.3 audit schema 与 Console UI Memory 语义不完全对齐但 deltaable）
  - **DataPlaneStores 用 Option<memory> + Option<audit>**：Phase 11 既有 4-service tests 不受影响（new() 默认 None；with_memory() / full() opt-in）；MemoryServer.list/get/pin/etc 缺 store → failed_precondition (清晰错误)
  - **agent_id 前缀 / namespace 后缀 LIKE 匹配**：v0.6 contractv1.MemoryItem 仅有 agent_scope 字段 (无 agent_id / namespace 独立列)；按惯例 `agent_scope = "{agent_id}:{namespace}"` 形式存储；importer 改造 [SPEC-DEFER:phase-15.import-to-memory-items] 时正式实现
  - **不引入新 dep**: 复用 std::sync::Mutex + rusqlite + 既有 audit 框架；R7 不触发
- **§9 Verification 结果**：
  - `cargo check -p contextforge-core`: clean
  - `cargo test -p contextforge-core --lib memory::`: 14 passed (9 store + 5 server)
  - `cargo test -p contextforge-core --test memory_integration`: 3 passed (CRUD via gRPC + list filter + soft_delete excluded)
  - `cargo test -p contextforge-core`: 84 lib + 17 test groups all PASS, 0 failed
  - `cargo build -p contextforge-core`: clean (serve_full wires SqliteMemoryStore + AuditSink to DataPlaneStores::full)
  - `go build ./...`: clean (proto regen via `buf generate proto`; no Go-side break)
- **剩余风险 / 未做项**：
  - Go REST handlers + grpcclient.MemoryClient [SPEC-OWNER:task-13.2]
  - importer 改造写入 memory_items [SPEC-DEFER:phase-15.import-to-memory-items]
  - memory hard delete (Console PRD 不要)
- **下游 task 影响**：task-13.2 用本 task MemoryService 作 grpcclient 桥梁 + 实现 5 REST handler
