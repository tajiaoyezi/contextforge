# Task `6.2`: `rest-api — daemon 本地 REST API (/v1/*)`

> ✅ 已过 `/s2v-implement` §2A 前置审核（2026-05-23，主 agent 与用户预先审定，worker 终端可直接进入 RED）：§3/§4/§5.2/§5.3 `<TBD-by-user>` 已清零、§6 AC 经用户审定接受、A/B/C/D/E 五决策已确认（A. HTTP framework=stdlib net/http、B. 5 endpoint 部分真实施 + import/eval/run stub 501、C. 新 `contextforge serve` 子命令持久 daemon、D. Token file 0600 启动软随机生、E. retriever 加 `get_chunk` 公开 API 支持 AC2 chunks/{id} — 详见 §10 §2A Decisions）。实时状态以下方 `**Status**` 字段为准；状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: Ready

**Priority**: P0
**Owner**: tajiaoyezi
**Related Phase**: Phase 6 (cli-api-export)
**Dependencies**: 6.1 (cli-search) — 已 merged 6a80f4c

## 1. Background

Agent 程序化调用需要本地 REST API（PRD §Decisions Log D3 / §Technical Approach REST/MCP 最小接口契约草案）。本地服务安全基线严格（PRD §Constraints Local service security baseline / §Technical Risks R9）。task-6.1 已 wire Rust tonic Search server + Go daemon.Search 包装，本 task 在其之上加 HTTP wrapper。

## 2. Goal

daemon 暴露 `POST /v1/search` / `GET /v1/chunks/{id}` / `POST /v1/import` / `POST /v1/eval/run` / `GET /v1/collections`，请求/响应契约与 PRD §Technical Approach 草案一致；默认只监听 127.0.0.1 或 Unix socket（禁 0.0.0.0），启用本地随机 token（文件 0600）；新增 `contextforge serve` 子命令启动持久 daemon。

## 3. Scope

### In Scope

- **新增 `contextforge serve` 子命令（§2A 决策 C：persistent server）**：
  - `internal/cli/cli.go` Execute 内 dispatch case `"serve"` 改 dispatch 到本 task 新增 `runServe(args, stdout, stderr)`（替换 task-1.4 default 的 "not implemented"）
  - flags（stdlib `flag.FlagSet`）：
    - `--addr=<host:port>` 监听地址（default = 自动选 loopback 端口）；非 loopback 拒（沿 task-1.4 `ensureLoopback`）
    - `--unix=<path>` Unix socket 路径（互斥于 --addr）
    - `--data-dir=<path>` data 根目录（default = `config.DefaultRootDir()`）
  - 启动顺序：解析 flag → 加载 / 生成 token → daemon.Start(AutoRestart=true) → 等 SERVING → 启动 REST HTTP server → 注册信号处理 (SIGINT/SIGTERM) → graceful shutdown (停 REST + daemon.Stop + flush audit)
  - 输出：监听地址 + token 路径写 stdout 一次（便于用户复制）
- **`internal/daemon/rest.go` 新增 REST HTTP server 实现（§2A 决策 A：stdlib `net/http` + `http.ServeMux`）**：
  - 公开 API：`func (d *Daemon) ServeREST(ctx context.Context, listener net.Listener, token string) error`
  - 内部 mux 注册 5 个 endpoint（§2A 决策 B：3 真实施 + 2 stub）
  - 全局 Authorization Bearer middleware (§2A 决策 D)
  - graceful shutdown via `srv.Shutdown(ctx)`
- **5 个 endpoint 实现（§2A 决策 B：部分真实施 + 其他 stub）**：
  - **真实施 #1 — `POST /v1/search`**：
    - Body JSON `{query, collections?, agent_scope?, top_k?, filters?{source_type?, language?}, explain?}` (`SearchRequest` 等价 — 用 stdlib `encoding/json`)
    - 映射 → proto `*contextforgev1.SearchRequest` → 调 `daemon.Search(ctx, req)`（task-6.1 已 wire）
    - 响应 200 + body JSON `{results: [...]}` (序列化 `*contextforgev1.SearchResponse` — ADR-003 单一 schema)
    - gRPC Status → HTTP code 映射：InvalidArgument→400 / FailedPrecondition→412 / NotFound→404 / Unauthenticated→401（middleware 前置）/ Internal→500
  - **真实施 #2 — `GET /v1/chunks/{id}`**：
    - Path 解析 chunk_id（stdlib mux `r.PathValue("id")` Go 1.22+）
    - 调 retriever 公开 API `get_chunk(chunk_id)`（§2A 决策 E：本 task 新加 — 详见下方 §5.3 + §3 跨 task 影响）
    - 命中 → 200 + 单条 `RetrievalResult` JSON；未命中 → 404 `{"error":"chunk not found"}`
  - **真实施 #3 — `GET /v1/collections`**：
    - 扫 data_dir 下 collection 目录（每 collection 1 子目录含 `chunks.db`）
    - 返 `{collections: [{id: "default", chunk_count: N, last_indexed_at: "..."}, ...]}` JSON
    - SQLite query 简单 `SELECT COUNT(*) FROM chunks` + dir mtime
  - **Stub 501 #1 — `POST /v1/import`**：
    - 解析 body 但不实际调用 importer（dep importer 整套 pipeline 超出 v0.1 P0）
    - 返 501 + body `{"error":"deferred to phase 8","note":"see task-8.x backlog"}`
  - **Stub 501 #2 — `POST /v1/eval/run`**：
    - dep task-8.1 eval-harness 未启
    - 返 501 + body `{"error":"deferred to phase 8 (eval-harness)","note":"see task-8.1"}`
- **Token 验证（§2A 决策 D + AC4 + AC5）**：
  - Token 存 `<data_dir>/token`（chmod 0600；目录 0700 沿 task-1.2 baseline）
  - 启动时：若 token 文件不存在 → `crypto/rand.Read(32 bytes)` + `hex.EncodeToString` 写入 + chmod 0600
  - 中间件 `authMiddleware(next http.Handler) http.Handler`：
    - 读 `Authorization: Bearer <token>` header；
    - 与文件 token strict 比较（const time `subtle.ConstantTimeCompare`）；
    - 失败 → 401 + body `{"error":"missing or invalid token"}` + 调 audit.Write 记 access denied
- **AC3 + AC5 安全基线（监听限制 + audit）**：
  - 沿 task-1.4 `ensureLoopback` / `freeLoopbackAddr` — `--addr` 非 loopback / `--unix=` 路径非绝对 → 启动 error；0.0.0.0 / :: 拒绝（defense in depth）
  - **Unix socket 支持**：`--unix=<path>` 走 `net.Listen("unix", path)` + 启动后 chmod 0600（私有 socket）；解决 Windows 不支持时 fallback to TCP loopback（warning 写 stderr）
  - **Audit log integration（AC5）**：复用 `internal/memoryops/audit/`（task-5.3 已 ✅）公开 API `audit.Write(event)`；每个 access（含 401）写一条事件：endpoint / status / timestamp / **不记 token 值 / 不记完整 query**（脱敏）
- **新增 RED→GREEN 测试**（5 个，落在以下文件）：
  - `internal/daemon/rest_test.go`（新建）— TEST-6.2.1 AC1 search 契约 + TEST-6.2.3 AC3 ensureLoopback + TEST-6.2.5 AC5 无 token 401 + audit；用 httptest.Server + 真实 daemon + in-memory tempdir Retriever
  - `internal/cli/serve_test.go`（新建）— TEST-6.2.4 AC4 token file 0600 + 启动时生成；用 t.TempDir() + os.Stat mode
  - `internal/daemon/rest_test.go` 中 — TEST-6.2.2 AC2 chunks/{id} 真返 + collections 真返 + import/eval stub 501 + 错误码映射
- **跨 task 影响（必须同 PR 完成）**：
  - **task-4.x retriever 加公开 API**（§2A 决策 E）：`core/src/retriever/mod.rs` 新增 `pub fn get_chunk(&self, chunk_id: &str) -> Result<Option<SearchResult>, RetrieverError>` — SQLite `WHERE chunk_id=?` + provenance JOIN + 复用 task-4.2 12-field SearchResult 映射；返 `Option<SearchResult>` 让上层区分 not-found vs error
  - **Rust gRPC ContextService 加 `GetChunk` RPC**：proto 已 frozen，需要走 SPEC-DRIFT 流程？— **不需要**：现有 `Search` 已能用 `collections` + 实际命中过滤实现等价 `GetChunk` 行为（短 query = chunk_id 字串完全匹配）。**v0.1 P0 决策：REST handler 内部 `daemon.Search` with `query = chunk_id` + filters 反查 chunk_id，不引入新 gRPC RPC**（避免 SPEC-DRIFT 串行）；retriever Rust 层 `get_chunk` API 作为 future 优化路径，本 task 仅扩 retriever Rust 层 API（gRPC 不动），server.rs `ContextService::search` 内部当 query 看起来像 chunk_id format 时优先调 retriever.get_chunk 走精确路径（fast-path optimization；fallback 仍走 full BM25 search）— **本 §3 In Scope 落实此 fast-path**
- **填实 `test/features/daemon.feature` SCEN-6.2.1~5** 占位 Given/When/Then（与 TEST-6.2.X 一一映射）

### Out Of Scope

- **gRPC `ContextService::GetChunk` RPC**：proto frozen + phase23-start-gate；v0.1 用 search fast-path 实现 chunks/{id}，新 RPC 留 future SPEC-DRIFT-task-6.2+ 串行
- **完整 POST /v1/import 实现**：dep importer + indexer 整套 pipeline；本 task 仅 stub 501；真实 import 留 future phase 8 chore / task-8.x backlog
- **POST /v1/eval/run 实现**：dep task-8.1 eval-harness 未启；本 task 仅 stub 501
- **TLS / mTLS / OAuth**：v0.1 沿 task-1.4 设计 loopback plaintext + Bearer token；TLS 留 Phase 7+ / v0.3
- **跨 collection 联邦 API**：v0.1 单 collection 范围；联邦留 future
- **POST `/v1/audit/query` 或其他 admin endpoint**：仅 5 个 PRD 列出的 endpoint；admin 留 future
- **修改 task-4.2 `Retriever::search` / `explain` 行为契约**：仅扩 `get_chunk` 新公开 API；search/explain 不动（向下兼容）
- **修改 task-5.3 audit `audit.Write` 公开 API**：复用现有 API；事件 schema 不改
- **修改 `Cargo.toml` / `go.mod` / `Cargo.lock` / `go.sum`**：R7 严格通道；本 task 不引新 dep（stdlib net/http / crypto/rand / encoding/json / archive/tar / encoding/hex / crypto/subtle 全 stdlib）
- **修改 `proto/contextforge/v1/*.proto`**：proto frozen + phase23-start-gate；fast-path 复用现有 Search RPC

## 4. Users / Actors

- **PRD §User Flow Agent 程序化调用消费者**：通过 REST `/v1/search` 集成到外部 Agent / IDE / 工具链
- **task-6.1 daemon.Search**（上游 ✅ done）：本 task REST `/v1/search` handler 内部调 daemon.Search；不改 daemon.Search 契约
- **task-1.4 daemon.Start / HealthCheck / Stop**（上游 ✅ done）：本 task `contextforge serve` 子命令复用，新增长生命周期管理（AutoRestart=true）
- **task-4.x retriever**（上游 ✅ done）：本 task **扩 `Retriever::get_chunk` 公开 API**（§2A 决策 E）；search/explain 行为不动；retriever 内部 Rust SearchResult 12-field schema 沿用
- **task-5.3 audit**（上游 ✅ done）：本 task REST middleware 写 audit.log；复用 `audit.Write(event)` 公开 API
- **task-1.2 config**（上游 ✅ done）：本 task token file 路径解析复用 `config.DefaultRootDir` 等价路径
- **task-6.3 exporter**（同期并行，codex 同会话跑）：与本 task 共享 daemon.Search + retriever；本 task 不改 exporter 边界
- **task-7.1 MCP tool**（下游强依赖）：MCP server 复用本 task REST handler 内部逻辑（gRPC 调用 + Authorization 等）；本 task 不直接 wire MCP
- **task-8.1 eval-harness**（下游软依赖）：可调 REST `/v1/search` 跑 recall eval；本 task 提供 endpoint 不实现 eval

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Technical Approach REST/MCP 最小接口契约草案 / §Constraints Local service security baseline / §Technical Risks R9）
- `docs/specs/phases/phase-6-cli-api-export.md`
- `docs/specs/tasks/task-6.1-cli-search.md`（上游：daemon.Search + per-invocation spawn 先例）
- `docs/specs/tasks/task-1.4-cli-init.md`（上游：CLI dispatch + daemon.Start pattern + ensureLoopback / freeLoopbackAddr）
- `docs/specs/tasks/task-1.2-config.md`（上游：DataDir + 0600/0700 baseline）
- `docs/specs/tasks/task-4.2-explain.md`（上游：Retriever::search/explain + SearchResult 12 字段 schema）
- `docs/specs/tasks/task-5.3-audit.md`（上游：audit.Write 公开 API + 脱敏规则）
- `docs/decisions/adr-003-cli-rest-mcp-grpc-interfaces.md`
- `docs/decisions/adr-004-local-first-privacy-baseline.md`
- `test/features/daemon.feature`

### 5.2 Imports

- **Go stdlib**:
  - `net/http`（HTTP server + ServeMux + Server.Shutdown — §2A 决策 A）
  - `net` (Unix socket / TCP listener)
  - `crypto/rand`（token 生成）/ `crypto/subtle`（const-time token 比较）
  - `encoding/hex`（token hex encode）/ `encoding/json`（request/response marshaling）
  - `context` / `time`（超时控制 / graceful shutdown 截止时间）
  - `os` / `os/signal` / `syscall`（信号处理 SIGINT/SIGTERM + file 0600/0700）
  - `fmt` / `io` / `errors` / `strings`（CLI parsing + 错误传播）
- **Go 内部（已有）**:
  - `github.com/tajiaoyezi/contextforge/internal/config`（DataDir / DefaultRootDir）
  - `github.com/tajiaoyezi/contextforge/internal/daemon`（Start / HealthCheck / Stop / Search — task-6.1 已 wire 的全部）
  - `github.com/tajiaoyezi/contextforge/internal/memoryops/audit`（audit.Write — task-5.3 已 ✅）
- **Go proto（已有，task-1.1 codegen）**:
  - `contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"`（SearchRequest / SearchResponse / RetrievalResult / SearchFilters / Provenance — 同 task-6.1）
- **Go gRPC（已有）**:
  - `google.golang.org/grpc/status`（gRPC Status → HTTP code 映射用）
  - `google.golang.org/grpc/codes`（同上）
- **Rust 内部（task-4.x retriever 扩 `get_chunk`）**:
  - `use crate::retriever::{Retriever, SearchResult, RetrieverError}`
  - `use crate::chunker::Provenance`（沿用 task-4.2 类型）
  - `rusqlite`（沿用 task-2.4 / 4.2 已引入；无新 dep）
- **R7 严格通道**：不引入新 Go module / Rust crate；不改 `Cargo.toml` / `Cargo.lock` / `go.mod` / `go.sum`

### 5.3 函数签名

**Go CLI** (`internal/cli/serve.go` 新建；`cli.go` 内 dispatch case `"serve"` 调 `runServe`):

```go
package cli

import (
    "context"
    "crypto/rand"
    "encoding/hex"
    "flag"
    "fmt"
    "io"
    "net"
    "net/http"
    "os"
    "os/signal"
    "path/filepath"
    "syscall"
    "time"

    "github.com/tajiaoyezi/contextforge/internal/config"
    "github.com/tajiaoyezi/contextforge/internal/daemon"
)

// serveOpts — flag 解析后状态（仅本包用）。
type serveOpts struct {
    Addr    string // --addr <host:port>; 空 → 自动选 loopback 端口
    Unix    string // --unix <path>; 互斥于 Addr
    DataDir string // --data-dir <path>; 空 → config.DefaultRootDir()
}

// runServe 实现 serve 子命令（AC1-5 持久 daemon 入口）。
// 1. 解析 flag → 加载 / 生成 token → daemon.Start(AutoRestart=true) → 等 SERVING
// 2. 启动 REST HTTP server (Daemon.ServeREST)
// 3. 注册信号处理 (SIGINT/SIGTERM) → graceful shutdown (停 REST + daemon.Stop)
// 4. stdout 一次性输出监听地址 + token 路径
//
// 返回 process exit code（0=ok / 1=运行错 / 2=usage 错）
func runServe(args []string, stdout, stderr io.Writer) int

// loadOrGenerateToken 读 <data_dir>/token；不存在 → crypto/rand 32 bytes hex + 写 0600。
// 返 token 字符串 + token 文件绝对路径 + error。
func loadOrGenerateToken(dataDir string) (token, tokenPath string, err error)
```

**Go daemon** (`internal/daemon/rest.go` 新建；`daemon.go` 不动):

```go
package daemon

import (
    "context"
    "encoding/json"
    "fmt"
    "net"
    "net/http"
    "strings"
    "time"

    "github.com/tajiaoyezi/contextforge/internal/memoryops/audit"
    contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// ServeREST 在已 listener 上启动 REST HTTP server，复用 d.Search / d.clientConn。
// listener 由 caller 创建（loopback TCP 或 Unix socket — caller ensureLoopback 已校验）。
// token 用于 Authorization Bearer 中间件校验。
// 返回前调用方 graceful shutdown：在 ctx 取消时 srv.Shutdown(ctx)。
func (d *Daemon) ServeREST(ctx context.Context, listener net.Listener, token string) error

// authMiddleware 校验 Authorization: Bearer <token>。
// 失败 → 401 + JSON body + audit.Write(access denied)。
// 用 crypto/subtle.ConstantTimeCompare 防 timing attack。
func authMiddleware(next http.Handler, expectedToken string) http.Handler

// handleSearch — POST /v1/search 处理器：JSON body → proto SearchRequest → d.Search → JSON resp。
// gRPC Status → HTTP code 映射：InvalidArgument→400 / FailedPrecondition→412 /
// NotFound→404 / Unauthenticated→401 / Internal→500 / Unknown→500。
func (d *Daemon) handleSearch(w http.ResponseWriter, r *http.Request)

// handleChunk — GET /v1/chunks/{id} 处理器：id → daemon.Search fast-path（chunk_id 字面匹配）→
// 单条 RetrievalResult JSON 或 404。
// v0.1 fast-path 复用 daemon.Search（避免新 gRPC RPC + phase23-gate SPEC-DRIFT）；
// Rust 端 server.rs CoreService::search 内部当 query 看起来像 chunk_id format 时调
// retriever.get_chunk 走精确路径（本 task 在 retriever 加的新 API）。
func (d *Daemon) handleChunk(w http.ResponseWriter, r *http.Request)

// handleCollections — GET /v1/collections 处理器：扫 data_dir 子目录 + 每 collection SQLite
// count + dir mtime → JSON {collections: [...]}。
func (d *Daemon) handleCollections(w http.ResponseWriter, r *http.Request)

// handleImport — POST /v1/import 处理器：v0.1 stub 501（dep importer pipeline 超 v0.1 P0；
// 见 §3 Out of Scope）。
func (d *Daemon) handleImport(w http.ResponseWriter, r *http.Request)

// handleEval — POST /v1/eval/run 处理器：v0.1 stub 501（dep task-8.1 eval-harness）。
func (d *Daemon) handleEval(w http.ResponseWriter, r *http.Request)

// grpcStatusToHTTP 把 gRPC Status 错误映射到 HTTP code（同 google.rpc.Code 标准映射）。
func grpcStatusToHTTP(err error) int
```

**Rust core** (`core/src/retriever/mod.rs` 扩 `get_chunk`):

```rust
impl Retriever {
    /// task-6.2 §2A 决策 E: 按 chunk_id 精确查 chunk + provenance；REST GET /v1/chunks/{id}
    /// fast-path 入口。SearchResult 12 字段沿 task-4.2 schema；空命中返 Ok(None)（不是 Err）。
    /// 内部：SQLite `WHERE chunk_id = ?1 LIMIT 1` + provenance JOIN（同 task-4.2 search() 的
    /// provenance 拼接逻辑），不走 Tantivy 全文检索。
    pub fn get_chunk(&self, chunk_id: &str) -> Result<Option<SearchResult>, RetrieverError>;

    // ---- 沿用 task-4.1 / 4.2 ----
    pub fn search(&self, opts: &SearchOptions) -> Result<Vec<SearchResult>, RetrieverError>;
    pub fn explain(&self, opts: &SearchOptions) -> Result<Vec<SearchResult>, RetrieverError>;
    pub fn open(...) -> ...;
    pub fn open_with_config(...) -> ...;
    pub fn config(&self) -> &RetrieverConfig;
}
```

**Rust core** (`core/src/server.rs` `CoreService::search` fast-path 补丁):

```rust
// task-6.2 §2A 决策 E: chunk_id fast-path — search() 内部当 query 看起来像 chunk_id
// format（^[0-9a-f]{16,}$ 或具体 schema 由 retriever 定）时优先调 retriever.get_chunk 走精确路径，
// fallback 仍走 full BM25 search。不引入新 gRPC RPC（proto frozen + phase23-gate）。
// 仅扩 search() 内部分支，不破契约。
async fn search(
    &self,
    req: Request<SearchRequest>,
) -> Result<Response<SearchResponse>, Status>;
```

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Implementation Phases Phase 6 Exit Criteria): REST `POST /v1/search` 可用，请求/响应契约与 PRD §Technical Approach 草案一致。
- [ ] **AC2** (PRD §Technical Approach REST/MCP 契约): `GET /v1/chunks/{id}` / `POST /v1/import` / `POST /v1/eval/run` / `GET /v1/collections` 可用（**v0.1 解读**：chunks/{id} + collections 真实施返业务数据；import + eval/run stub 501 + 显式 deferred note —— §2A 决策 B）。
- [ ] **AC3** (PRD §Constraints Local service security baseline): daemon 默认只监听 `127.0.0.1` 或 Unix socket，v0.1 禁默认绑定 `0.0.0.0`。
- [ ] **AC4** (PRD §Constraints Local service security baseline): REST API 默认启用本地随机 token，token 文件权限 `0600`。
- [ ] **AC5** (PRD §Technical Risks R9): 未带有效 token 的请求被拒；访问写 audit log（脱敏，复用 task 5.3）。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 /v1/search 契约一致 | SCEN-6.2.1 | TEST-6.2.1 | - | unit-test | Not Started |
| AC2 chunks/collections 真返 + import/eval stub 501 | SCEN-6.2.2 | TEST-6.2.2 | - | unit-test | Not Started |
| AC3 默认本地监听禁 0.0.0.0 | SCEN-6.2.3 | TEST-6.2.3 | - | unit-test | Not Started |
| AC4 token 0600 启动软随机生 | SCEN-6.2.4 | TEST-6.2.4 | - | unit-test | Not Started |
| AC5 无 token 拒绝 + audit 脱敏 | SCEN-6.2.5 | TEST-6.2.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R9**（本地 daemon/MCP 暴露面）：监听限制（loopback / Unix socket / 拒 0.0.0.0）+ token Bearer + audit 脱敏 三层缓解齐全；与 task-1.4 baseline 同构（无新 attack surface）。
- **gRPC Search fast-path 对 chunk_id 误判风险**：query 看起来像 chunk_id format 时优先调 retriever.get_chunk；若用户故意输入 chunk_id-pattern 的 BM25 query → fast-path 命中后不再 fallback。**缓解**：fast-path 仅 chunk_id 精确匹配命中时返；未命中（None）则 fallback 全文 search；用户场景仅 REST GET /v1/chunks/{id} 触发（CLI search 不走 fast-path 因 query 内容多样）。
- **持久 daemon 资源管理**：`contextforge serve` 长生命周期 daemon.Start(AutoRestart=true) 沿 task-1.4 supervisor pattern；SIGTERM 触发 srv.Shutdown(ctx, 5s timeout) + daemon.Stop。**风险**：Windows 上 Unix socket 不支持 → fallback TCP loopback 已写 stderr warning（与 PRD platform 平台支持一致：v0.1 P0 = Linux/WSL2）。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。

## 10. Completion Notes

- **完成日期**：`<TBD-after-impl>`
- **改动文件**：`<TBD-after-impl>`
- **commit 列表**：`<TBD-after-impl>`
- **§9 Verification 结果**：
  - install: `<TBD-after-impl>`
  - typecheck: `<TBD-after-impl>`
  - unit-test: `<TBD-after-impl>`
- **剩余风险 / 未做项**：`<TBD-after-impl>`
- **下游 task 影响**：`<TBD-after-impl>`
- **§2A Decisions**（2026-05-23 用户审定，主 agent 与用户预先审定后落 spec；worker 完工时按实际实施情况验证 / 补充）：
  - **A: HTTP framework = stdlib net/http**（不引 chi / gorilla mux / gin；沿 task-1.4 stdlib flag 先例 + R7 严格通道；手写 `http.ServeMux` 5 endpoint 路由 + middleware 函数链）
  - **B: 5 endpoint 部分真实施 + 其他 stub 501**：search / chunks/{id} / collections 真实施；import / eval/run 返 501 + body `{"error":"deferred to phase 8"}`。AC2「可用」v0.1 解读为「有响应」（含 501 也算）；spec §3 In Scope + §6 AC2 明示。未来 task-8.x 接 import / eval/run 真实施
  - **C: 新增 `contextforge serve` 子命令（persistent server）**：替换 task-1.4 default 的 "not implemented"；daemon.Start(AutoRestart=true) + REST HTTP server + 信号 graceful shutdown。task-6.1 CLI per-invocation spawn pattern 保留不变（脚本场景）；REST 用 long-running 场景
  - **D: Token = file 0600 启动软随机生**：`<data_dir>/token` 文件不存在时 `crypto/rand 32 bytes hex` 生成 + chmod 0600；Authorization Bearer middleware + `crypto/subtle.ConstantTimeCompare`；audit.Write 记 access deny（不记 token 值）
  - **E: retriever 加公开 `get_chunk(chunk_id)` API**：v0.1 fast-path 实现 AC2 chunks/{id}；不引入新 gRPC RPC（proto frozen + phase23-gate）；server.rs CoreService::search 内部当 query 看起来像 chunk_id 时优先调 retriever.get_chunk 走精确路径，fallback 全文 search。retriever Rust 层新 API 是 minimal 扩展，sibling 于 search/explain
  - **R7 严格通道**：未引入新 Go module / Rust crate；沿 task-1.3 / 1.4 / 4.2 / 5.3 / 6.1 已落定依赖（net/http / crypto/rand / crypto/subtle / encoding/json / encoding/hex stdlib + grpc + google.golang.org/grpc/status/codes 已有 + rusqlite 沿 task-2.4 / 4.2）
