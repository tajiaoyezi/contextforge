# Task `11.2`: `go-rest-to-grpc-proxy — internal/consoleapi/grpcclient/ + Deps gRPC backed + MemStore env-gated fallback`

**Status**: Ready

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 11 (console-real-data-plane)
**Dependencies**: task-11.1 (`core/proto/console_data_plane.proto` 4 service + Rust tonic server) + task-10.4 (Go `internal/consoleapi/` 9 REST handler + `Deps` 接口 + `MemStore` + bearer middleware) + [ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) D3/D4

## 1. Background

task-11.1 落地 Rust 4 个 gRPC service 后，Go console-api-serve 需要从 v0.3 in-memory MemStore 默认行为切到 gRPC-backed Deps —— REST handler 收请求 → 调对应 gRPC method → 把 gRPC response 转 Console contractv1 JSON 返回（[ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) D3 thin proxy）。

[ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) D4 显式规定：v0.3 `internal/consoleapi/memstore.go` MemStore + WorkspaceAdapter + JobAdapter **不删除**，但默认禁用 —— 仅当 `CONSOLE_API_FALLBACK_INMEM=1` 环境变量设置时启用 + log warning + `/v1/health` 返回 `degraded=true` + `store="inmem-fallback"`。理由：保 v0.3 集成测试 `internal/consoleapi/router_test.go` + `e2e_test.go` 可继续以 in-memory 模式跑（不依赖 Rust daemon spawn），同时给运维 degraded 信号。

D3 thin proxy 关键约束：handler **禁止** 任何业务逻辑（status 推进 / 字段补全 / 时间戳生成 / 校验）—— 全交 Rust；**禁止** handler 内字段映射代码 —— `.proto` 字段命名 (task-11.1) 与 Go contractv1 JSON tag 必须 snake_case 1:1，handler 直 `protojson.Unmarshal` / `json.Marshal` 同字段。

## 2. Goal

`internal/consoleapi/grpcclient/grpcclient.go` 含 `New(addr string, opts ...grpc.DialOption) (*Client, error)` + 4 个 client wrapper impl 现有 `WorkspaceClient` / `JobClient` / `SearchClient` / `EventsClient` 接口；`internal/consoleapi/handlers.go` 重构为 thin proxy（不引入字段映射代码 + 不引入业务逻辑）；`internal/cli/console_api_serve.go` 新增 `--grpc-addr` flag (默认 `127.0.0.1:48180`) + `--fallback-inmem` flag (别名 env `CONSOLE_API_FALLBACK_INMEM`)；`internal/consoleapi/memstore.go` 保留不变但仅 env-gated 启用；v0.3 test suite 全绿：`go test ./internal/consoleapi/... -v` + `go test ./test/conformance/... -run TestConsoleContractV1Conformance` 不退化。

## 3. Scope

### In Scope

- **新增 `internal/consoleapi/grpcclient/grpcclient.go`**：
  - `type Client struct { workspace WorkspaceClient; job JobClient; search SearchClient; events EventsClient }` —— 持有 4 个 gRPC client wrapper [SPEC-OWNER:task-11.1]
  - `func New(addr string, opts ...grpc.DialOption) (*Client, error)`：dial → 返 4 client wrapper struct
  - `WorkspaceClient` wrapper impl `consoleapi.WorkspaceClient` 接口 (`Create` / `List` / `Get`)：直 dispatch 到 tonic-generated `WorkspaceServiceClient.Create` / `.List` / `.Get`；gRPC `tonic::Status` ↔ Go sentinel error mapping：`codes.NotFound` → `consoleapi.ErrNotFound`；`codes.FailedPrecondition` → `consoleapi.ErrJobAlreadyTerminal`；其它 → `fmt.Errorf("gRPC %v: %s", st.Code(), st.Message())`
  - `JobClient` wrapper impl `consoleapi.JobClient` 接口 (`Enqueue` / `Get` / `Cancel`)：直 dispatch
  - `SearchClient` wrapper impl `consoleapi.SearchClient` 接口 (`Search`)：gRPC Query 返 `SearchResponse{result, trace}` → 直 unpack 返 `(SearchResult, RetrievalTrace, error)`
  - `EventsClient` wrapper impl `consoleapi.EventsClient` 接口 (`Recent(limit int)`)：v0.4 实现：调 `EventsService.Subscribe` server stream，30s timeout 或满 `limit` evt 后 close stream + return slice；long-poll wrap 完整实现在 [SPEC-OWNER:task-11.4]，本 task 仅占位调 Subscribe stream 取最近 N evt
  - **不引入字段映射代码**：proto-generated Go types 字段命名（protoc-go 默认 PascalCase + protojson 输出 snake_case）→ contractv1 Go struct (json tag snake_case) 直 `protojson.Marshal` then `json.Unmarshal` 即可（**或** 在 grpcclient wrapper 内 protoc-go struct 字段一一赋给 contractv1 struct 同字段 —— 自决选项；目标是 wrapper 内代码 ≤ 简单赋值）
- **修改 `internal/consoleapi/types.go`**：保留 `Deps` 接口 + 4 client interface 签名不动（D3 接口稳定）；新增 `ErrDataPlaneUnavailable` sentinel for gRPC connect-refused mapping → HTTP 503
- **修改 `internal/consoleapi/router.go`**：
  - `Deps.HealthCheck()` 新方法（或 router 内调 grpcclient `WaitForReady`）：启动 `console-api-serve` 时若 grpc dial 失败 + fallback-inmem 未启 → log warning + 注册 503 handler 兜底（health endpoint 永远工作返 `degraded=true` + `missing=["data_plane"]`；其它 endpoint 返 503 + ErrCoreUnavailable）
  - 启用 fallback-inmem 时 log warning `"console-api: using in-memory fallback store (data plane unreachable); set CONSOLE_API_FALLBACK_INMEM=0 + start contextforge-core daemon to use real persistence"` + `/v1/health` payload `degraded=true` + `store="inmem-fallback"`
- **修改 `internal/consoleapi/handlers.go`** (重构)：
  - handler 内**禁止**任何 status 推进 / 字段补全 / 时间戳生成 / 校验 —— 全 dispatch 到 Deps.Job/Workspace/Search/Events
  - 错误映射沿用 v0.3 sentinel: `ErrNotFound` → 404 / `ErrJobAlreadyTerminal` → 409 / `ErrDataPlaneUnavailable` → 503 / 其它 → 500 (`ErrorBody{Code, Message}`)
  - bearer auth middleware 不动（task-10.4 既有 `bearerAuthMiddleware` 保留）
- **修改 `internal/cli/console_api_serve.go`** (或 daemon serve 子命令的 console-api 子段)：
  - 新增 `--grpc-addr string` flag (默认 `127.0.0.1:48180`)
  - 新增 `--fallback-inmem bool` flag (默认 false；别名读 env `CONSOLE_API_FALLBACK_INMEM=1`)
  - 启动顺序：
    1. parse flags + env
    2. if `--fallback-inmem` set: 用 `memstore.New(...)` Deps 实例化 router；log warning
    3. else: `grpcclient.New(--grpc-addr)` 实例化 Deps；dial 失败 → router 用 degraded Deps wrapper (返 ErrDataPlaneUnavailable for all) + health endpoint 永远返 degraded [SPEC-OWNER:task-11.2]
- **保留 `internal/consoleapi/memstore.go` 不变** + 改为 conditional 仅 fallback-inmem 启用
- **集成测试**：
  - `internal/consoleapi/e2e_test.go::TestRESTEndpoints_E2E` 已存在（v0.3 task-10.4）；本 task 新增 `TestRESTEndpoints_E2E_GrpcBacked` [SPEC-OWNER:task-11.2] —— spawn `contextforge-core` Rust daemon + console-api-serve gRPC connect → curl 9 endpoint 全过 + workspace 持久化跨重启
  - `internal/consoleapi/grpcclient/grpcclient_test.go::TestFallbackInmemWhenNoDaemon`：仅启 console-api-serve 不启 daemon + `--fallback-inmem=true` → curl `/v1/health` → assert `degraded=true` + `store="inmem-fallback"` + POST workspaces 200 created
  - `internal/consoleapi/grpcclient/grpcclient_test.go::TestNoDaemonNoFallback_503`：仅启 console-api-serve 不启 daemon + `--fallback-inmem=false` → curl `/v1/health` → assert HTTP 503 + `degraded=true` + `missing=["data_plane"]`
- **conformance 不退化**：`go test ./test/conformance/... -run TestConsoleContractV1Conformance` 在 fallback-inmem 模式跑（v0.3 已绿），本 task 不破坏；env `CONSOLE_REPO` 设时 cross-repo 反向跑 Console fakehttpserver 仍 PASS
- **不引入新 R7 dep**：现有 `go.mod` 含 `google.golang.org/grpc` + `google.golang.org/protobuf`（task-9.3 引入）；本 task 不动 `go.mod`
- **文件锚点**：`internal/consoleapi/grpcclient/grpcclient.go` + `grpcclient_test.go` + `internal/consoleapi/handlers.go` + `internal/consoleapi/router.go` + `internal/consoleapi/types.go` + `internal/cli/console_api_serve.go` + proto-go 生成产物 (`internal/consoleapi/proto/` 或 `internal/proto/` 复用既有路径)
- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **task-11.1 proto 起草** [SPEC-OWNER:task-11.1]：本 task 仅消费 task-11.1 已落地的 `.proto` + Rust gRPC server
- **task-11.3 JobRunner 真接 IndexSession** [SPEC-OWNER:task-11.3]：本 task Go 端 dispatch 到 gRPC 即可；JobRunner 行为由 task-11.3 替换
- **task-11.4 真 long-poll wrap on events stream** [SPEC-OWNER:task-11.4]：本 task 占位 EventsClient.Recent 实现简单调 Subscribe → 取 N evt 即返；完整 30s timeout / 100 evt batch 在 task-11.4
- **删除 memstore.go** [SPEC-DEFER:console-endpoint-expansion]：v0.4 保留为 env-gated fallback；删除留 v0.5+ 多实例 daemon 时再评估
- **新增 endpoint** [SPEC-DEFER:console-endpoint-expansion]：v0.4 仅 v0.3 既有 9 endpoint；新 endpoint v0.4.x 增量
- **mTLS / 跨进程 auth** [SPEC-DEFER:task-future.consoleapi-mtls]：v0.4 grpcclient 走 plaintext 本地 loopback（127.0.0.1:48180 + bearer 在 REST 层）
- **gRPC streaming filters / replay** [SPEC-DEFER:console-endpoint-expansion]
- **进程生命周期管理（daemon 自动 spawn / supervisor）** [SPEC-DEFER:task-future.process-supervisor]：v0.4 deploy 仍人工双进程；deploy doc 在 task-11.4 release smoke 段说明

## 4. Users / Actors

- **Console HTTPAdapter**（cross-repo 接收方，不变）：调本 task 改造后的 9 endpoint，行为与 v0.3 一致（路径 / shape / 错误码）
- **task-11.3 实施 agent**（上游 / 同 phase）：依赖本 task gRPC dispatch 正确 + sentinel error mapping 不变，才能让 JobRunner 真触发后的 status / heartbeat / cancel 状态被 Go REST 端透传
- **task-11.4 实施 agent**（同 phase）：依赖本 task EventsClient wrapper 接到 task-11.4 真接 EventBus broadcast 后扩展为 long-poll wrap [SPEC-OWNER:task-11.4]
- **运维**：通过 `/v1/health` 区分三种状态（healthy=daemon 通 / degraded=fallback-inmem / degraded=missing data_plane）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md` §D3 / §D4
- `docs/specs/phases/phase-11-console-real-data-plane.md`
- `docs/specs/tasks/task-11.1-rust-data-plane-grpc-services.md` (proto + Rust gRPC server)
- `docs/specs/tasks/task-10.4-rest-endpoints.md` (Go Deps 接口 + handler + memstore + bearer middleware 现状)
- `docs/specs/tasks/task-10.1-contractv1-types.md` (Go contractv1 JSON tag 字段名)
- `internal/consoleapi/types.go` (现有 Deps + 4 接口 + ErrNotFound / ErrJobAlreadyTerminal sentinel)
- `internal/consoleapi/handlers.go` (现有 9 handler)
- `internal/consoleapi/memstore.go` (现有 in-memory 实现)

### 5.2 Imports

- **Go**: 现有 `google.golang.org/grpc` + `google.golang.org/protobuf/encoding/protojson` (task-9.3 引入)；stdlib `context` + `time`；现有 `github.com/go-chi/chi/v5` (task-6.2/10.4)
- **不引入新依赖**：R7 不触发；`go.mod` 不动

### 5.3 函数签名

```go
package grpcclient

import (
    "context"
    "fmt"

    "google.golang.org/grpc"
    "google.golang.org/grpc/codes"
    "google.golang.org/grpc/status"

    pb "github.com/tajiaoyezi/contextforge/internal/consoleapi/proto"
    "github.com/tajiaoyezi/contextforge/internal/consoleapi"
    "github.com/tajiaoyezi/contextforge/internal/contractv1"
)

type Client struct {
    conn      *grpc.ClientConn
    Workspace consoleapi.WorkspaceClient
    Job       consoleapi.JobClient
    Search    consoleapi.SearchClient
    Events    consoleapi.EventsClient
}

func New(ctx context.Context, addr string, opts ...grpc.DialOption) (*Client, error) {
    if len(opts) == 0 {
        opts = []grpc.DialOption{grpc.WithTransportCredentials(insecure.NewCredentials())}
    }
    conn, err := grpc.DialContext(ctx, addr, opts...)
    if err != nil { return nil, fmt.Errorf("grpc dial %s: %w", addr, err) }
    return &Client{
        conn:      conn,
        Workspace: &workspaceClient{c: pb.NewWorkspaceServiceClient(conn)},
        Job:       &jobClient{c: pb.NewJobServiceClient(conn)},
        Search:    &searchClient{c: pb.NewSearchServiceClient(conn)},
        Events:    &eventsClient{c: pb.NewEventsServiceClient(conn)},
    }, nil
}

func (c *Client) Close() error { return c.conn.Close() }

// 每个 wrapper struct 实现 consoleapi.<X>Client 接口
type workspaceClient struct{ c pb.WorkspaceServiceClient }
func (w *workspaceClient) Create(req contractv1.WorkspaceCreate) (contractv1.Workspace, error) {
    resp, err := w.c.Create(context.Background(), &pb.CreateWorkspaceRequest{Name: req.Name, RootPath: req.RootPath, /* ... */})
    if err != nil { return contractv1.Workspace{}, mapGrpcErr(err) }
    return protoToWorkspace(resp), nil
}
// 同理其它 wrapper + protoToX helper（赋值即可，无业务逻辑）

func mapGrpcErr(err error) error {
    st, ok := status.FromError(err)
    if !ok { return err }
    switch st.Code() {
    case codes.NotFound:           return consoleapi.ErrNotFound
    case codes.FailedPrecondition: return consoleapi.ErrJobAlreadyTerminal
    case codes.Unavailable:        return consoleapi.ErrDataPlaneUnavailable
    default:                       return fmt.Errorf("gRPC %v: %s", st.Code(), st.Message())
    }
}
```

## 6. Acceptance Criteria

- [ ] AC1：`grpcclient.New(ctx, addr, opts...)` 返回 `*Client` 含 4 个 wrapper 实现 `consoleapi.WorkspaceClient` + `JobClient` + `SearchClient` + `EventsClient` 接口（compile-time 接口 satisfied） — **verified by unit-test step `go test ./internal/consoleapi/grpcclient/... -run TestClientImplementsDeps`**
- [ ] AC2：`console-api-serve --grpc-addr` 默认 `127.0.0.1:48180`；`--addr` 默认 `0.0.0.0:48181` (v0.3 既有 sane default 不变)；新增 `--fallback-inmem` flag (别名 env `CONSOLE_API_FALLBACK_INMEM=1`) — **verified by unit-test step `go test ./internal/cli/... -run TestConsoleApiServeFlags`**
- [ ] AC3：handler 不引入字段映射代码 + 不引入业务逻辑（grep `handlers.go` 无 "if status ==" / "time.Now()" / 业务校验代码）—— **verified by code review checklist (manual diff vs v0.3 handlers.go + grep anti-pattern) + ADR-014 D2 lint anti-pattern "field_map" or "status_advance" scan**
- [ ] AC4：`CONSOLE_API_FALLBACK_INMEM=1` 时 grpc 不可达 → 启用 MemStore + log warning + `/v1/health` 返回 `degraded=true` + `store="inmem-fallback"` + 业务 endpoint 仍 200/201；`CONSOLE_API_FALLBACK_INMEM` 未设 + grpc 不可达 → `/v1/health` 返 503 + `degraded=true` + `missing=["data_plane"]` + 业务 endpoint 全 503 + `ErrCoreUnavailable` — **verified by integration-test step `go test ./internal/consoleapi/grpcclient/... -run TestFallbackInmemWhenNoDaemon` + `TestNoDaemonNoFallback_503`**
- [ ] AC5：v0.3 test suite 全绿（go test ./internal/consoleapi/... + ./test/conformance/...）；`TestRESTEndpoints_E2E_GrpcBacked` 真启 Rust daemon + console-api-serve + 9 endpoint flow 全过 + workspace 持久化跨 daemon 重启 — **verified by §9 verify run + integration `TestRESTEndpoints_E2E_GrpcBacked`**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | grpcclient.New + 4 wrapper impl 接口 | internal/consoleapi/grpcclient/grpcclient.go + TestClientImplementsDeps | Ready |
| AC2 | CLI flags + env override | internal/cli/console_api_serve.go + TestConsoleApiServeFlags | Ready |
| AC3 | handlers 无业务逻辑 / 无字段映射 | internal/consoleapi/handlers.go + manual review + D2 lint | Ready |
| AC4 | fallback-inmem env-gated + 503 sans daemon | grpcclient_test.go + TestFallbackInmemWhenNoDaemon + TestNoDaemonNoFallback_503 | Ready |
| AC5 | v0.3 不退化 + e2e gRPC backed | internal/consoleapi/e2e_test.go::TestRESTEndpoints_E2E_GrpcBacked | Ready |

## 8. Risks

- **proto-go 生成路径**：`internal/consoleapi/proto/` 还是 `internal/proto/` 还是 task-9.3 既有路径？需 verify task-9.3 `proto/contextforge/v1/*.proto` 生成产物位置 + Makefile gen-proto target 复用还是新建
- **gRPC dial 失败 vs gRPC connect-on-first-call**：grpc-go `DialContext` 默认 lazy；首个 RPC 才真 connect。需 `WaitForReady(true)` + 启动期主动 `client.Health(...)` 探测
- **bearer auth 仍在 Go middleware 层**：D3 thin proxy 不下沉 auth 到 gRPC；缓解 middleware 顺序 router.go 内：bearer → grpcclient dispatch
- **handler 重构破坏 v0.3 e2e test**：v0.3 `TestRESTEndpoints_E2E` 用 MemStore；本 task 需 router.go 注入 Deps 时切 grpcclient (实测 daemon) 或 MemStore (fallback)；v0.3 test 改为 fallback-inmem 模式跑或保留两 variant
- **conformance test 不退化**：`go test ./test/conformance/...` 用 in-memory Deps（v0.3 模式）；本 task 应让 conformance test 默认仍走 in-memory（设 `CONSOLE_API_FALLBACK_INMEM=1`），避免 conformance 测试必须 spawn daemon

## 9. Verification Plan

- **install**: `go mod download`
- **lint**: `gofmt -l internal/consoleapi/grpcclient/ internal/consoleapi/`
- **typecheck**: `go vet ./...`
- **unit-test**: `go test ./internal/consoleapi/grpcclient/... ./internal/consoleapi/... -v` (全绿)
- **integration**: `go test ./internal/consoleapi/grpcclient/... -run TestFallbackInmemWhenNoDaemon` + `TestNoDaemonNoFallback_503` + `TestRESTEndpoints_E2E_GrpcBacked` (真 cargo build + spawn daemon)
- **e2e**: 通过 integration 实现
- **build**: `go build ./...`
- **coverage**: 不强制（task-10.4 同款；新 grpcclient 包单测覆盖 wrapper + error mapping）
- **runtime-smoke**: 启 contextforge-core daemon + console-api-serve → `curl -i http://127.0.0.1:48181/v1/health` → 返 `status=healthy + contract_version=v1`
- **manual**: `CONSOLE_API_FALLBACK_INMEM=1 console-api-serve` 单进程 → `curl /v1/health` 返 `degraded=true + store=inmem-fallback`

## 10. Completion Notes

<!-- 完工时按 standard.md §8.3 6 项 schema 回填 -->

- **完成日期**：<待回填>
- **改动文件**：<待回填>
- **commit 列表**：<待回填>
- **§9 Verification 结果**：<待回填>
- **剩余风险 / 未做项**：
  - Long-poll wrap on events stream [SPEC-OWNER:task-11.4]
  - 进程生命周期管理 [SPEC-DEFER:task-future.process-supervisor]
  - mTLS / 跨进程 auth [SPEC-DEFER:task-future.consoleapi-mtls]
- **下游 task 影响**：task-11.3 / 11.4 不再动 Go 端；本 task 后 task-11.3 JobRunner 真接 IndexSession 直接由 gRPC 透传到 Go REST 端
