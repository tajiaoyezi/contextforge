# Task `10.5`: `conformance-test — test/conformance/console_contractv1_test.go 反向跑 Console fakehttpserver oracle`

**Status**: Done

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 10 (console-contract-v1)
**Dependencies**: task-10.1 (Go types 镜像) + task-10.4 (9 REST endpoint 真实现)

## 1. Background

task-10.4 9 REST endpoint 实现后，需验证其输出能被 Console HTTPAdapter 正确解析 — Cross-repo Contract v1 字段对齐 verifiable 的关键 gate。Console 端已有自己的 conformance test（用 fakehttpserver 作为 Mock oracle）；ContextForge 端反向使用同款 oracle 验证自己的实现。详 [ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md) §D5。

## 2. Goal

`test/conformance/console_contractv1_test.go` 用 Console HTTPAdapter Go client + Console fakehttpserver 设定的期望（路径 / shape / 错误码），反向跑 ContextForge daemon (含 task-10.4 9 endpoint) 端到端：
1. 启 ContextForge daemon
2. 创建 Console HTTPAdapter client 指向 ContextForge daemon URL
3. 跑 9 endpoint 调用：创建 workspace → 列出 workspace → 触发 index job → poll job 直到 terminal → search → cancel job 等典型 flow
4. 断言每个返回的 contractv1 类型 unmarshal 完整 + FieldAvailability.Missing 为期望集合（v0.3 missing 应为空，因为 9 endpoint 都返回完整 must-have 字段）
5. Cross-repo dependency: env `$CONSOLE_REPO=H:/devlopment/code/ContextForge-Console` 时跑全套；未设时 SKIP（CI 友好）

## 3. Scope

### In Scope

- **新增 `test/conformance/console_contractv1_test.go`**：
  - `TestConsoleContractV1Conformance` — 主入口
  - Setup: spawn ContextForge daemon (cargo build + go build + spawn 子进程) + 等 `/v1/health` 200 OK + contract_version="v1"
  - Cross-repo dependency setup: 检查 env `$CONSOLE_REPO` 设置 → 若未设 t.Skip("CONSOLE_REPO env required for cross-repo conformance test")；若设了 → import 模拟 Console HTTPAdapter (内嵌简化版 HTTPAdapter Go 代码到 test，或通过 go.mod replace + vendor 拉 Console adapter 包 — v0.3 选择内嵌避免新依赖)
  - 9 endpoint flow:
    1. GET /v1/health → 验证 CoreHealth 字段 + contract_version="v1"
    2. POST /v1/workspaces (sample WorkspaceCreate) → 验证返回 Workspace 字段完整 + status="ready"
    3. GET /v1/workspaces → 验证列表含上一步创建的 workspace
    4. GET /v1/workspaces/:id → 验证按 id 取回
    5. GET /v1/workspaces/non-existent → 验证 404 → ErrNotFound mapping
    6. POST /v1/index-jobs body `{workspace_id}` → 验证 IndexJob status="queued"
    7. Poll GET /v1/index-jobs/:job_id 直到 status in (succeeded, failed) (timeout 30s)
    8. POST /v1/search body SearchRequest → 验证返回 `{result, trace}` 嵌套 + 字段完整
    9. POST /v1/index-jobs/:terminal_job_id/cancel → 验证 409 Conflict mapping
    10. GET /v1/observability/events → 验证返回 ObservabilityEvent 列表 (>=1 event from above operations)
  - Teardown: kill daemon + clean staging dir
- **错误码 mapping 验证**：404 → ErrNotFound / 409 → ErrConflict / 5xx → ErrCoreUnavailable (注意 Console HTTPAdapter 用 sentinel error，ContextForge test 用 errors.Is 比对)
- **FieldAvailability 验证**：v0.3 应所有 must-have 字段都填充 → Missing=[]；test 断言所有返回类型的 FieldAvailability.Complete() == true
- **CI skip 机制**：env 未设 → t.Skip + 退出码 0；env 设但 Console 仓库不可读 → t.Fatal
- 文件锚点：`test/conformance/console_contractv1_test.go` + `test/conformance/README.md` (文档 env 假设 + 跑法)

### Out Of Scope

- **修改 Console 仓库任何文件** [SPEC-OWNER:console-team]：cross-repo 写硬约束 (ADR-014 D4 + playbook §自决规则 #8)；任何 Console 端期望与 ContextForge 实际不一致 → 转 §8 STOP 由用户协调 Console PR
- **should-have / optional 字段 conformance** [SPEC-DEFER:task-future.conformance-should-have]：v0.3 仅 must-have；should-have 字段 conformance 留 v0.4
- **Console HTTPAdapter Go 源码直接 import** [SPEC-DEFER:task-future.conformance-vendored-adapter]：v0.3 内嵌简化版 HTTPAdapter；v0.4 评估 go.mod replace 拉 Console adapter 包（涉及跨仓库 Go module 设计）
- **CI runner 设置 $CONSOLE_REPO**：v0.3 CI skip；docs 说明本地 dev 设 env 跑全套
- **Mock 端反向：用 ContextForge fakehttpserver 给 Console 用** [SPEC-DEFER:task-future.contextforge-fakehttpserver]：Console 端有自己的 fakehttpserver；ContextForge 反向 mock 留 v0.4 (Console PR-driven if needed)
- **Performance conformance** (P95 < 200ms etc.)：v0.3 仅功能 conformance；性能基准留 task-10.6 docker compose smoke 内 manual check

## 4. Users / Actors

- **task-10.6 console-integration-smoke 实施 agent**（下游）：本 task PASS 是 task-10.6 docker compose 全套联调的前置
- **Phase 10 closeout PR reviewer**：本 task 是 phase §6 AC5 owner，closeout PR mapping 表引用本 task §6 AC

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/decisions/adr-015-console-contract-v1-compatibility.md` §D5
- `docs/specs/phases/phase-10-console-contract-v1.md`
- `docs/specs/tasks/task-10.1-contractv1-types.md`
- `docs/specs/tasks/task-10.4-rest-endpoints.md`
- `H:/devlopment/code/ContextForge-Console/console-api/internal/coreadapter/http_adapter.go`
- `H:/devlopment/code/ContextForge-Console/console-api/internal/coreadapter/testhelper/fakehttpserver.go`
- `H:/devlopment/code/ContextForge-Console/console-api/internal/coreadapter/conformance_test.go` (Console 端 conformance test 模式参考)

### 5.2 Imports

- **Go**: 内嵌简化版 Console HTTPAdapter（不引入跨仓库 Go module 依赖）；现有 net/http + encoding/json + testing
- **不引入新依赖**：R7 不触发

### 5.3 函数签名

```go
package conformance_test

import (
    "context"
    "encoding/json"
    "fmt"
    "io"
    "net/http"
    "os"
    "os/exec"
    "path/filepath"
    "testing"
    "time"

    "github.com/tajiaoyezi/contextforge/internal/contractv1"
)

func TestConsoleContractV1Conformance(t *testing.T) {
    if os.Getenv("CONSOLE_REPO") == "" {
        t.Skip("CONSOLE_REPO env required for cross-repo conformance test")
    }
    // setup daemon, run 9 endpoint flow, teardown
}

// 内嵌简化 Console HTTPAdapter (Console adapter package 简化版)
type contextForgeHTTPClient struct {
    baseURL string
    authToken string
    httpClient *http.Client
}

func (c *contextForgeHTTPClient) GetHealth(ctx context.Context) (*contractv1.CoreHealth, error)
func (c *contextForgeHTTPClient) CreateWorkspace(ctx context.Context, req contractv1.WorkspaceCreate) (*contractv1.Workspace, error)
func (c *contextForgeHTTPClient) ListWorkspaces(ctx context.Context) ([]contractv1.Workspace, error)
func (c *contextForgeHTTPClient) GetWorkspace(ctx context.Context, id string) (*contractv1.Workspace, error)
func (c *contextForgeHTTPClient) EnqueueIndexJob(ctx context.Context, workspaceID string) (*contractv1.IndexJob, error)
func (c *contextForgeHTTPClient) GetIndexJob(ctx context.Context, jobID string) (*contractv1.IndexJob, error)
func (c *contextForgeHTTPClient) CancelIndexJob(ctx context.Context, jobID string) error
func (c *contextForgeHTTPClient) Search(ctx context.Context, req contractv1.SearchRequest) (*contractv1.SearchResult, *contractv1.RetrievalTrace, error)
func (c *contextForgeHTTPClient) GetEvents(ctx context.Context) ([]contractv1.ObservabilityEvent, error)
```

## 6. Acceptance Criteria

- [x] AC1：`test/conformance/console_contractv1_test.go` 含 TestConsoleContractV1Conformance + 9 endpoint flow + env-based skip 机制 — **verified by manual cat + grep 9 endpoint paths**
- [x] AC2：env `CONSOLE_REPO=$path` 设时 test PASS 端到端 — **verified by integration-test step `CONSOLE_REPO=H:/devlopment/code/ContextForge-Console go test ./test/conformance/... -run TestConsoleContractV1Conformance -v -timeout 180s`**
- [x] AC3：env 未设时 test SKIP (exit 0) 不 fail — **verified by unit-test step `go test ./test/conformance/... -run TestConsoleContractV1Conformance -v`**（无 env）
- [x] AC4：所有返回 contractv1 类型的 FieldAvailability.Complete() == true（v0.3 must-have 字段全部填充） — **verified by integration-test step (AC2 内嵌断言)**
- [x] AC5：错误码 mapping 验证：404 → ErrNotFound / 409 → ErrConflict（v0.3 不验 5xx，daemon 真错很罕见）— **verified by integration-test step (AC2 内嵌 case)**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | test file + flow | test/conformance/console_contractv1_test.go | Done |
| AC2 | env 设 → PASS | test/conformance/console_contractv1_test.go (CONSOLE_REPO 路径) | Done |
| AC3 | env 未设 → SKIP | test/conformance/console_contractv1_test.go | Done |
| AC4 | FieldAvailability complete | test/conformance/console_contractv1_test.go assert | Done |
| AC5 | 错误码 mapping | test/conformance/console_contractv1_test.go assert | Done |

## 8. Risks

- **Console fakehttpserver 与实际 HTTPAdapter 行为不一致**：v0.3 ContextForge 端用 fakehttpserver oracle 验证 wire shape；如发现 Console 实际 HTTPAdapter 行为与 fakehttpserver 不一致 → ADR-014 D4 + playbook §自决规则 #8 转 §8 STOP（用户协调 Console PR）
- **内嵌简化 HTTPAdapter 与 Console 实际实现漂移**：v0.3 内嵌版本仅覆盖 9 endpoint + error mapping；可能漏一些 header (User-Agent / Accept) Console 实际 inject；缓解 internal/consoleapi handler 容忍缺失 header (不 enforce)
- **CI runner CONSOLE_REPO 未设 → AC2 跳过**：v0.3 CI 默认 SKIP AC2；本地 dev 设 env 跑全套；docs/conformance/README.md 文档化
- **daemon spawn race condition**：health check poll 等 30s 超时 + 重试

## 9. Verification Plan

- **install**: `go mod download`
- **lint**: `gofmt -l test/conformance/` (empty)
- **typecheck**: `go vet ./...`
- **unit-test**: `go test ./test/conformance/... -v -short` (SKIP path)
- **integration**: `CONSOLE_REPO=$pwd/../ContextForge-Console go test ./test/conformance/... -run TestConsoleContractV1Conformance -v -timeout 180s`
- **e2e**: 复用 integration
- **build**: `go build ./...`
- **coverage**: N/A (conformance test 不计 coverage)
- **runtime-smoke**: 通过 integration test 实现
- **manual**: 手动 CONSOLE_REPO 设后 go test 跑过

## 10. Completion Notes

<!-- 完工时按 standard.md §8.3 6 项 schema 回填 -->

- **完成日期**：2026-05-24
- **改动文件**：
  - `test/conformance/console_contractv1_test.go` (新增 — TestConsoleContractV1Conformance + 内嵌 minimalConsoleHTTPClient mimicking Console HTTPAdapter)
  - `test/conformance/README.md` (新增 — 跑法 / 设计 / AC 覆盖 / OOS)
  - `docs/specs/tasks/task-10.5-conformance-test.md` (本 spec §6 / §7 / §10 / Status 推进)

  **Trade-off #1 (v0.3 in-process REST server, 非 spawned daemon)**：spec §3 设计 spawn 真 daemon。task-10.4 §10 trade-off #1 决策用 in-memory MemStore (cross-process SQLite 共享 [SPEC-DEFER:task-future.cross-process-sqlite-sharing])，所以 spawn daemon 没有意义 (Rust 集成不通)。本 task 复用 task-10.4 的 startServerE2E 模式 — net.Listen("127.0.0.1:0") + in-process net/http server，跑 Console-style 9 endpoint flow。Wire shape conformance 完全验证；cross-process consistency 留 v0.4。
  **Trade-off #2 (内嵌 minimalConsoleHTTPClient 而非 import Console adapter)**：spec §5.2 选择内嵌简化版避免新 cross-repo Go module dep。Console 仓库未发布 Go module proxies；v0.4 可评估 go.mod replace pull Console adapter 包。
- **commit 列表**：
  - feat(conformance): task-10.5 — Console Contract v1 conformance test + CONSOLE_REPO env-based skip + 9 endpoint Console-style flow + FieldAvailability.Complete() assertions
  - docs(spec): task-10.5 §6 / §7 / §10 / Status → Done
- **§9 Verification 结果**：
  - install: ✅ (`go mod download`)
  - lint: ✅ (`gofmt -l test/conformance/` empty)
  - typecheck: ✅ (`go vet ./...` exit 0)
  - unit-test: PASS — env CONSOLE_REPO 未设 → SKIP (AC3); 设 → PASS (AC1/AC2/AC4/AC5)
  - integration: 复用 unit; full flow 含 9 endpoint + sentinel error mapping + FieldAvailability.Complete()
  - build: ✅ (`go build ./...`)
  - manual: ✅ 跑过 `CONSOLE_REPO=H:/devlopment/code/ContextForge-Console go test ./test/conformance/... -v` PASS
- **剩余风险 / 未做项**：
  - Should-have / optional 字段 conformance [SPEC-DEFER:task-future.conformance-should-have]
  - Live Console HTTPAdapter import [SPEC-DEFER:task-future.conformance-vendored-adapter]
  - Cross-process SQLite consistency (depends on task-future.cross-process-sqlite-sharing)
  - CI runner CONSOLE_REPO env 默认未设 → CI 默认 SKIP；本地 dev 跑全套
- **下游 task 影响**：task-10.6 docker compose smoke 是本 task PASS 后启动 Console UI 真验证的下一步
