# Task `9.1`: `proto-index-rpc — service.proto add-only rpc Index stream + IndexRequest / IndexProgress messages`

> Status=Ready；主 agent §2A 自审通过（ADR-012 主 agent 自治 + 用户 goal §自决规则 6）。本 task 是 Phase 9 cli-pipeline 首个 task — 解锁 9.2/9.3/9.4 实施依赖。

**Status**: Ready

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 9 (cli-pipeline)
**Dependencies**: 无（基于 Phase 1 已 freeze 的 `proto/contextforge/v1/context.proto` 与 `service.proto`）

## 1. Background

v0.1 spec drift：`proto/contextforge/v1/service.proto` 只暴露 `rpc Search` + `rpc Health`，没有 `rpc Index` —— `core/src/indexer/mod.rs::IndexSession::index_path` 实现完整但只能从 Rust 单元测试内部直接调用，CLI `contextforge index` 无法触达。详见 [ADR-013](../../decisions/adr-013-cli-data-plane-grpc-bridge.md) §Context #1。

本 task 是 Phase 9 解锁项：先 add-only 扩 proto 契约，task-9.2 / 9.3 / 9.4 才能在新 RPC 上 wire Rust handler + Go client。

PRD §Technical Risks R1 + ADR-001 / 003 / task-1.1 已定 proto 兼容规则：仅加字段 / 不删 / 不改 tag。service ContextService 新增 RPC method 是合法 add-only 演进（不动现有 Search/Health；新 messages 独立 tag namespace），schema_version 保持 `0.1` 不变。

## 2. Goal

`proto/contextforge/v1/service.proto` 含 `rpc Index(IndexRequest) returns (stream IndexProgress)`；新增 `IndexRequest`（source_path / data_dir / collection_id）+ `IndexProgress`（files_processed / chunks_written / current_file / done / error）messages（独立文件 `proto/contextforge/v1/index.proto` 与 search.proto 命名风格一致）；buf generate 产物（Go `proto/contextforge/v1/index.pb.go` + 更新的 `service.pb.go` / `service_grpc.pb.go` + Rust prost）regen 并 commit；`go vet ./...` + `cargo check --workspace` 全绿。

## 3. Scope

### In Scope

- **新增 `proto/contextforge/v1/index.proto`**：
  ```proto
  syntax = "proto3";
  package contextforge.v1;
  option go_package = "github.com/tajiaoyezi/contextforge/proto/contextforge/v1;contextforgev1";

  // IndexRequest — POST /v1/index request (Phase 9 task-9.1 / ADR-013).
  // SCAN_PATH mode (v0.2 only): server scans source_path filesystem,
  // applies denylist / secret redaction, writes Tantivy + SQLite.
  message IndexRequest {
    string source_path = 1;     // absolute path; required
    string data_dir = 2;        // collection data root (~/.contextforge/ or override)
    string collection_id = 3;   // required; defaults to "default" if empty on server
  }

  // IndexProgress — streaming progress update.
  // Server sends ≥1 message; final message has done=true.
  // On error: sends one message with error set and stream closes.
  message IndexProgress {
    int64 files_processed = 1;
    int64 files_skipped_denied = 2;
    int64 files_skipped_redaction = 3;
    int64 chunks_written = 4;
    string current_file = 5;   // path being processed (last seen); empty on final
    bool done = 6;
    string error = 7;          // empty unless terminal error
  }
  ```
- **修改 `proto/contextforge/v1/service.proto`**：在 `service ContextService` 内 append `rpc Index(IndexRequest) returns (stream IndexProgress);`，紧跟现有 `rpc Search` 之后；`import "contextforge/v1/index.proto";` 加在文件顶部 import 段
- **跑 codegen 更新产物文件**：
  - Go: `proto/contextforge/v1/index.pb.go`（新增）/ `proto/contextforge/v1/service.pb.go`（更新）/ `proto/contextforge/v1/service_grpc.pb.go`（更新 — 增 `ContextServiceClient.Index` + `ContextServiceServer.Index`）
  - Rust: 由 `core/build.rs` tonic-build 自动 regen（task-1.1 已建链路），跑 `cargo check --workspace` 触发
- **验证向后兼容**：现有 `internal/cli/search.go` + `internal/daemon/search.go` + `core/src/server.rs::CoreService::search` 不需任何修改即可继续工作（go vet / cargo check 全绿即证明 binary compat）
- **不动 message 字段**：不修改 `context.proto` / `search.proto` / `import.proto` / `eval.proto` 现有 message 字段或 tag 编号
- 文件锚点：`proto/contextforge/v1/index.proto`（新增）+ `proto/contextforge/v1/service.proto`（修改，3 行：1 import + 1 rpc + 注释）+ codegen 产物（Go / Rust）

### Out Of Scope

- **Rust `CoreService::index` 业务实现**（task-9.2）— 本 task 只产 proto 契约 + codegen 骨架；Rust 侧服务端实现可暂返 `Status::unimplemented`（codegen 默认 trait method 即 unimplemented）
- **Go `daemon.Index` client wrapper**（task-9.3）— 本 task 不写 Go client 包装；codegen 产生的 `contextforgev1.NewContextServiceClient(conn).Index(...)` 直接可用，daemon 包装留 task-9.3
- **CLI 改造**（task-9.3 / 9.4）— `internal/cli/index.go` / `import.go` 不在本 task 改
- **`rpc Import` / `rpc Eval` 新增** — D1 决策两步式 import 走 Go 离线（不调 daemon）；eval 已通过 task-8.1 `internal/eval/` 直接调 Search RPC 跑通。已有 `ImportRequest` / `EvalRequest` messages（task-1.1 预留）保留不动 — 不 wire RPC 也合法（add-only freeze 容忍未用 message）
- **FEED_RECORDS 模式 / records 字段** — D1 决策两步式不需要 server-side feed-records 模式；如未来需切换到方案 B（单步式），通过新 task 增 `IndexRequest.records repeated Record` 字段（tag 4，add-only）
- **schema_version 版本号变更** — 仍 `0.1`，本 task 不动 freeze 表面
- **proto-gen-go 工具链升级 / buf.gen.yaml 改动** — sticky to current `protoc-gen-go` + `protoc-gen-go-grpc` 版本；如需升 buf 走独立 chore-dep PR

## 4. Users / Actors

- **task-9.2 rust-grpc-index 实施 agent**（下游）：消费本 task 产出的 `ContextServiceServer` trait `Index` method signature 实现 Rust 业务逻辑
- **task-9.3 go-cli-index 实施 agent**（下游）：消费本 task 产出的 `contextforgev1.NewContextServiceClient(conn).Index(...)` Go client API
- **现有 task-6.1 / 7.1 / 8.x 实现**（兼容性接收方）：本 task 必须保证 Search RPC + Health RPC 二进制兼容 — 现有 CLI search / MCP / eval 不修改即可继续工作
- **CI / verify.sh**：本 task §9 `go vet` + `cargo check` + `cargo test --workspace` 必须全绿（codegen 触发的编译验证）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Technical Risks R1 / §Decisions Log D1 D3 / §REST·MCP 最小接口契约草案）
- `docs/specs/phases/phase-9-cli-pipeline.md`
- `docs/decisions/adr-001-go-rust-dual-binary-architecture.md`
- `docs/decisions/adr-003-cli-rest-mcp-grpc-interfaces.md`
- `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`
- `docs/specs/tasks/task-1.1-proto.md`（contract freeze rule 来源）
- `proto/contextforge/v1/context.proto`（CONTRACT FREEZE RULE 注释）
- `proto/contextforge/v1/service.proto`（现有 ContextService 定义）
- `proto/contextforge/v1/search.proto`（命名风格 + import pattern 参考）

### 5.2 Imports

- **proto 内**：
  - `import "contextforge/v1/index.proto";`（service.proto 新增）
  - `index.proto` 自身不需 import（IndexRequest / IndexProgress 仅含 scalar + bool，不需 timestamp / struct）
- **codegen 工具链**（已在仓库，不引入新依赖）：
  - `protoc-gen-go`（buf.gen.yaml 已配）
  - `protoc-gen-go-grpc`（buf.gen.yaml 已配）
  - `core/build.rs` 用 `tonic-build`（task-1.1 已配，自动 regen Rust 绑定）
- **不引入新依赖**：R7 不触发；`go.mod` / `Cargo.toml` 不动

### 5.3 函数签名

> 本 task 产出 proto 契约 + codegen 文件；签名由 proto 定义自动生成，下面列出 codegen 后的关键 Go / Rust 接口契约（消费方约定）。

**Go**（codegen 自动生成在 `proto/contextforge/v1/service_grpc.pb.go`）：

```go
// ContextServiceClient interface 新增方法（生成）:
type ContextServiceClient interface {
    Search(ctx context.Context, in *SearchRequest, opts ...grpc.CallOption) (*SearchResponse, error)
    Health(ctx context.Context, in *HealthRequest, opts ...grpc.CallOption) (*HealthResponse, error)
    Index(ctx context.Context, in *IndexRequest, opts ...grpc.CallOption) (ContextService_IndexClient, error)  // 新增
}

// ContextService_IndexClient 流式 client（生成）:
type ContextService_IndexClient interface {
    Recv() (*IndexProgress, error)
    grpc.ClientStream
}

// ContextServiceServer interface 新增方法（生成）:
type ContextServiceServer interface {
    Search(context.Context, *SearchRequest) (*SearchResponse, error)
    Health(context.Context, *HealthRequest) (*HealthResponse, error)
    Index(*IndexRequest, ContextService_IndexServer) error  // 新增
    mustEmbedUnimplementedContextServiceServer()
}

// IndexRequest / IndexProgress messages（生成在 index.pb.go）:
type IndexRequest struct {
    SourcePath   string `protobuf:"bytes,1,opt,name=source_path,json=sourcePath,proto3"`
    DataDir      string `protobuf:"bytes,2,opt,name=data_dir,json=dataDir,proto3"`
    CollectionId string `protobuf:"bytes,3,opt,name=collection_id,json=collectionId,proto3"`
}

type IndexProgress struct {
    FilesProcessed        int64  `protobuf:"varint,1,opt,name=files_processed,json=filesProcessed,proto3"`
    FilesSkippedDenied    int64  `protobuf:"varint,2,opt,name=files_skipped_denied,json=filesSkippedDenied,proto3"`
    FilesSkippedRedaction int64  `protobuf:"varint,3,opt,name=files_skipped_redaction,json=filesSkippedRedaction,proto3"`
    ChunksWritten         int64  `protobuf:"varint,4,opt,name=chunks_written,json=chunksWritten,proto3"`
    CurrentFile           string `protobuf:"bytes,5,opt,name=current_file,json=currentFile,proto3"`
    Done                  bool   `protobuf:"varint,6,opt,name=done,proto3"`
    Error                 string `protobuf:"bytes,7,opt,name=error,proto3"`
}
```

**Rust**（codegen 自动生成在 `core/src/proto/contextforge.v1.rs` 等同 task-1.1 模式）：

```rust
// trait ContextService 新增方法（tonic 生成）:
#[async_trait]
pub trait ContextService: Send + Sync + 'static {
    async fn search(&self, request: tonic::Request<SearchRequest>) -> Result<tonic::Response<SearchResponse>, tonic::Status>;
    async fn health(&self, request: tonic::Request<HealthRequest>) -> Result<tonic::Response<HealthResponse>, tonic::Status>;

    // 新增 — 关联类型支持 stream return:
    type IndexStream: futures_core::Stream<Item = Result<IndexProgress, tonic::Status>> + Send + 'static;
    async fn index(&self, request: tonic::Request<IndexRequest>) -> Result<tonic::Response<Self::IndexStream>, tonic::Status>;
}
```

> ⚠️ Rust tonic stream return 用 associated type — task-9.2 实施时具体类型（`tokio_stream::wrappers::ReceiverStream<...>` 等）由 9.2 决定，本 task 不绑定。

- SCEN/TEST-9.1.1 → service.proto 含 `rpc Index(IndexRequest) returns (stream IndexProgress)` 字面（grep 命中）（AC1）
- SCEN/TEST-9.1.2 → index.proto 含 IndexRequest 3 字段 + IndexProgress 7 字段（按上文 5.3 定义）（AC2）
- SCEN/TEST-9.1.3 → `go vet ./...` + `go test ./proto/...` 全绿（codegen 产物可编译）（AC3）
- SCEN/TEST-9.1.4 → `cargo check --workspace` + `cargo test --workspace --no-run` 全绿（Rust 绑定可编译）（AC4）
- SCEN/TEST-9.1.5 → 现有 task-6.1 `internal/cli/search_test.go` + task-7.1 mcp 测试 + task-4.x retriever 测试不回归（baseline green）（AC5）

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (ADR-013 §Decision #1 / PRD §Decisions Log D3 协议接口): `proto/contextforge/v1/service.proto` `service ContextService` 内含 `rpc Index(IndexRequest) returns (stream IndexProgress);` 字面（grep 精确命中）
- [ ] **AC2** (本 task 新增 / ADR-013 §Decision #1): 新建 `proto/contextforge/v1/index.proto`，含 `IndexRequest`（3 字段：source_path / data_dir / collection_id）+ `IndexProgress`（7 字段：files_processed / files_skipped_denied / files_skipped_redaction / chunks_written / current_file / done / error）；schema_version 字面 `0.1` 不动
- [ ] **AC3** (PRD §Technical Risks R1 / task-1.1 contract freeze): `proto/contextforge/v1/index.pb.go` 已生成并 commit；`service.pb.go` / `service_grpc.pb.go` 更新含 `ContextServiceClient.Index` + `ContextServiceServer.Index`；`go vet ./...` 全绿；`go test ./proto/...` 全绿
- [ ] **AC4** (PRD §Technical Risks R1): Rust 绑定 regen（`cargo check --workspace` 触发 tonic-build）；`cargo check --workspace` 全绿；`cargo test --workspace --no-run` 全绿（编译阶段）
- [ ] **AC5** (PRD §Decisions Log D1 R1 add-only freeze): 现有 Search RPC / Health RPC 二进制兼容 — 不修改 `internal/cli/search.go` / `internal/daemon/search.go` / `core/src/server.rs::CoreService::search` 任何代码；`go test ./internal/cli/... ./internal/daemon/...` 全绿（baseline 不回归）；`cargo test --workspace` 全绿

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 service.proto rpc Index | SCEN-9.1.1 | TEST-9.1.1 | - | unit-test (grep + go test ./proto/...) | - |
| AC2 index.proto 7 字段 | SCEN-9.1.2 | TEST-9.1.2 | - | unit-test (grep + go test ./proto/...) | - |
| AC3 Go codegen 全绿 | SCEN-9.1.3 | TEST-9.1.3 | - | typecheck + unit-test | - |
| AC4 Rust codegen 全绿 | SCEN-9.1.4 | TEST-9.1.4 | - | typecheck + unit-test | - |
| AC5 baseline 不回归 | SCEN-9.1.5 | TEST-9.1.5 | - | unit-test | - |

## 8. Risks

- 关联 PRD §Technical Risks **R1**（Go↔Rust gRPC 边界 / 契约演进）：本 task 是 R1 的直接缓解 — 严格 add-only 不动现有 tag，独立新 .proto 文件隔离 message namespace，service method 新增不破坏 client wire 兼容性。
- 风险次：`protoc-gen-go-grpc` 版本如不在仓库稳定可重现 → codegen 产物在不同 dev 机生成的字节序可能不同；缓解：本 task 把 codegen 产物 commit，本地 dev 不再 regen 即可消费；如未来要 regen 必须 sticky 到当前工具链版本。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。本 task 是 Phase 9 首个 task，不触发 §4 Gate 3 phase smoke gate（phase smoke 留 task-9.6）。

## 10. Completion Notes

> 待 task 完成后回填。
