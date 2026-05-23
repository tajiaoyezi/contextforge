# Task `7.1`: `mcp-server — MCP server (context_search/read/explain/collections) + client allowlist`

> ✅ 已过 `/s2v-implement` §2A 前置审核（2026-05-23，主 agent 与用户预先审定，worker 终端可直接进入 RED）：§3/§4/§5.2/§5.3 `<TBD-by-user>` 已清零、§6 AC 经用户审定接受、A/B/C/D/E 五决策已确认（A. 手写 MCP JSON-RPC over stdio (R7 严格通道，不引 SDK)、B. stdio subprocess + 新 `contextforge mcp` 子命令、C. `<data_dir>/mcp-allowlist.json` 启动读 + initialize handshake 验证、D. 复用 task-6.2 `internal/memoryops/audit/` 同 audit-rest.log + Endpoint 字段 prefix `mcp:`、E. MCP spec lock 2025-06-18 — 详见 §10 §2A Decisions）。实时状态以下方 `**Status**` 字段为准；状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: Done

**Priority**: P0
**Owner**: tajiaoyezi
**Related Phase**: Phase 7 (mcp-adapter)
**Dependencies**: Phase 6 (cli-api-export) — ✅ done (PR #41/43/44/45 merged cd7df58)

## 1. Background

把 ContextForge 接入真实多 Agent 工作流（OpenClaw/Hermes/Claude Code/Cursor/Zed）经 MCP（PRD §Vision / §Technical Approach MCP tools）。MCP 协议/SDK 版本为 PRD §Open Questions O4 的 TBD（需 Phase 7 启动前锁定 — **§2A 决策 E 锁定 MCP spec 2025-06-18**）。Phase 7 唯一 task（即最后 task，team §4 Gate 3 触发）。

## 2. Goal

MCP server 暴露 `context_search` / `context_read` / `context_explain` / `context_collections`，返回字段与 REST search result 可解释字段一致；MCP client 须显式 allowlist，未授权拒绝；adapter 仅做协议翻译，与核心检索解耦（R7 缓解）；MCP spec lock 2025-06-18。

## 3. Scope

### In Scope

- **新增 `contextforge mcp` 子命令（§2A 决策 B：stdio subprocess + `contextforge mcp` 子命令）**：
  - `internal/cli/cli.go` Execute 内 dispatch case `"mcp"` 改 dispatch 到本 task 新增 `runMCP(args, stdin, stdout, stderr)`（替换 task-1.4 default 的 "not implemented"）
  - flags（stdlib `flag.FlagSet`）：
    - `--data-dir=<path>` data 根目录（default = `config.DefaultRootDir()`）
    - `--allowlist=<path>` MCP client allowlist 文件路径（default = `<data_dir>/mcp-allowlist.json`）
  - 输入 stdio (`os.Stdin` / `os.Stdout`)：JSON-RPC 2.0 over LSP-style framing 或 newline-delimited（MCP spec 2025-06-18 stdio transport）
  - daemon 生命周期：
    - mcp-adapter 启动时 daemon.Start(AutoRestart=true)（同 task-6.2 contextforge serve 模式）
    - 等 daemon SERVING
    - 启动 MCP server 读 stdin / 写 stdout（不监听 HTTP / 不引 REST endpoint）
    - stdin EOF (agent 中止) 或 SIGTERM → graceful shutdown (daemon.Stop + flush audit)
- **`internal/mcpadapter/` 新建包（§2A 决策 A：手写 MCP JSON-RPC over stdio）**：
  - **手写 MCP 2025-06-18 stdio transport 实现**（R7 严格通道；不引 mcp-go / 任何 MCP SDK）
  - 公开 API：`func (s *Server) Serve(ctx context.Context, stdin io.Reader, stdout io.Writer) error`
  - 内部组件：
    - `internal/mcpadapter/server.go` — Server struct + Serve 主循环 + initialize handshake + tools/list + tools/call dispatch
    - `internal/mcpadapter/jsonrpc.go` — JSON-RPC 2.0 message encode/decode (stdlib `encoding/json` + `bufio.Scanner` for newline-delimited 或 Content-Length framing per MCP 2025-06-18 spec)
    - `internal/mcpadapter/tools.go` — 4 个 tool 实现（context_search/read/explain/collections）
    - `internal/mcpadapter/allowlist.go` — `<data_dir>/mcp-allowlist.json` 加载 + initialize handshake 客户端校验
  - **MCP spec 2025-06-18 兼容范围标注（AC4）**：spec §10 §2A Decision E 明示锁定版本；client higher version → 仍 negotiate down to 2025-06-18；client lower version → 拒绝 + audit 记
- **4 个 MCP tool 实施（§2A 决策 A + B：每个 tool 复用 task-6.x 已 wire 的 daemon/exporter/retriever）**：
  - `context_search`：调 task-6.1 `daemon.Search` (per-invocation pattern — mcp-adapter 持久 daemon 内调用，复用 ServeBackend wiring 思路；不再 spawn core 子进程)；映射 result schema 与 PRD §search response 一致 (12 字段)
  - `context_read`：调 task-6.2 §2A 决策 E fast-path（query=chunk_id 走 retriever.get_chunk）；input { chunk_id, collection? }；output 单条 RetrievalResult 12 字段
  - `context_explain`：调 task-6.1 `daemon.Search` with `explain=true`；input { query, ... }；output `{ results, retrieval_trace }` 含 reason / matched_terms / provenance
  - `context_collections`：扫 `<data_dir>/collections/<id>/` 子目录（同 task-6.2 handleCollections 逻辑）；output `{ collections: [{id, chunk_count, last_indexed_at}, ...] }`
- **Client allowlist (§2A 决策 C：`<data_dir>/mcp-allowlist.json` 启动读 + initialize handshake 客户端校验)**：
  - 默认文件路径：`<data_dir>/mcp-allowlist.json`（mode 0600；目录沿 task-1.2 0700 baseline）
  - JSON schema：`[{"name": "claude-desktop", "version": ">=0.7.0"}, {"name": "cursor"}, ...]`
  - mcp-adapter 启动：读 allowlist 文件 → 解析 entries（无文件 → 空 allowlist = 拒绝所有；用户必须显式加 entry）
  - MCP initialize handshake：client 报 `client.name` + `client.version`；mcp-adapter 比对 allowlist
    - 匹配 name + version satisfies range (semver via inline minimal parser 或 stdlib semver — **不引外部 semver dep**；v0.1 仅支持精确匹配 + `>=X.Y.Z` 比较)
    - 不匹配 → JSON-RPC error response code -32000 (server error) + 关闭 stdio + `audit.Write(Endpoint: "mcp:initialize", Status: 403, Reason: "client not allowlisted")`
- **Audit log 集成（§2A 决策 D：复用 task-6.2 `internal/memoryops/audit/` 同 audit-rest.log + Endpoint 字段 prefix `mcp:`）**：
  - 复用 task-6.2 `audit.Write(dataDir, Event)` 公开 API（PR #44 merged 881aadf）
  - Event.Endpoint 字段值：`mcp:initialize` / `mcp:context_search` / `mcp:context_read` / `mcp:context_explain` / `mcp:context_collections`
  - Event.Status：MCP error code 转 HTTP-equivalent (initialize 403 / unauthorized / tool call 200 / error 500 / 等)
  - 与 task-6.2 REST 访问混合在同 audit-rest.log（运维统一查询 + grep `mcp:` 分类）
  - **不记 token / 不记完整 query / 不记 tool args**（AC3 redaction — 同 task-6.2 baseline）
- **AC5 Phase 7 端到端 smoke 由本 task 填实 phase-7 spec §6**（同 task-6.3 phase-6 §6 填实先例）：
  - 本 task 在 phase-7 spec §6 落 shell + JSON-RPC 命令骨架：启 `contextforge mcp` subprocess → stdin/stdout JSON-RPC initialize + tools/list + tools/call ×4 → 校验 4 tool 字段与 REST 一致 + 未 allowlist client 被拒
  - 自动化运行留 task-8.1 eval-harness（同 task-6.3 §2A 决策 E 模式）
- **新增 RED→GREEN 测试**（5 个）：
  - `internal/mcpadapter/server_test.go` (新建) — TEST-7.1.1 (AC1 context_search 字段一致) + TEST-7.1.2 (AC2 4 tool 行为)
  - `internal/mcpadapter/allowlist_test.go` (新建) — TEST-7.1.3 (AC3 未 allowlist 拒 + audit)
  - `internal/mcpadapter/jsonrpc_test.go` (新建) — TEST-7.1.4 (AC4 JSON-RPC 2025-06-18 spec 兼容)
  - `internal/cli/mcp_test.go` (新建) — TEST-7.1.5 (AC5 CLI mcp 子命令 end-to-end + 假 stdin/stdout fake)
- **填实 `test/features/mcp-adapter.feature` SCEN-7.1.1~5** 占位 Given/When/Then
- **填实 `docs/specs/phases/phase-7-mcp-adapter.md` §6** 端到端 smoke 命令骨架

### Out Of Scope

- **MCP HTTP / SSE / WebSocket transport**：v0.1 仅 stdio（MCP 主流；agent 默认 stdio subprocess）；HTTP/SSE transport 留 future v0.2+
- **MCP 1.0 / 2026-XX-XX 等更高 spec 版本支持**：v0.1 §2A 决策 E 锁定 2025-06-18；2025-11-25 (current per modelcontextprotocol.io) 等新版本接入留 future SPEC-DRIFT-task-7.1.spec-bump
- **MCP `resources` / `prompts` / `sampling` 能力**：v0.1 仅 4 个 tool（per AC1/2）；resources/prompts/sampling 留 future（同 PRD Vision 范围内但分阶段）
- **完整 semver parser**：v0.1 allowlist version 仅支持精确匹配 + `>=X.Y.Z`；完整 semver (1.x.x ~ caret) 留 future 或 R7 chore-dep PR 引 `github.com/Masterminds/semver`
- **MCP client UI 帮助** / 安装文档：v0.1 仅命令行 startup；UX 留 docs/onboarding future
- **MCP server 集群 / 负载均衡**：v0.1 单 daemon + 单 mcp subprocess；集群留 future
- **修改 `Cargo.toml` / `go.mod` / `Cargo.lock` / `go.sum`**：R7 严格通道；本 task 全 stdlib (`encoding/json` / `bufio` / `os` / `path/filepath` / `crypto/subtle` / `context` / `time`)
- **修改 `proto/contextforge/v1/*.proto`**：proto frozen + phase23-start-gate
- **修改 task-5.3 audit Rust SQLite / task-6.2 audit Go 公开 API / task-4.x retriever / task-6.x daemon**：仅消费现有公开 API

## 4. Users / Actors

- **PRD §Vision 多 Agent 工作流消费者**（业务消费）：Claude Desktop / Cursor / Zed / OpenClaw / Hermes 等 agent 经 MCP 接入 ContextForge 一致可解释上下文
- **task-6.1 cli-search**（上游 ✅ done）：本 task `context_search` / `context_explain` 复用 `daemon.Search`
- **task-6.2 rest-api**（上游 ✅ done）：本 task 复用 `internal/memoryops/audit/audit.Write` 公开 API（§2A 决策 D）；本 task 复用 `internal/cli/serve.go::loadOrGenerateToken` 等 helper（可能；如不复用就独立 helper）；不 wire REST handler 自身（mcp-adapter 仅 stdio JSON-RPC）
- **task-6.3 exporter**（上游 ✅ done）：本 task 不直调 exporter（MCP 不暴露 export tool；export 仍是 CLI-only per task-6.3）；保留共享 ContextRecord proto 消费模式
- **task-4.2 retriever `Retriever::explain`**（上游 ✅ done）：本 task `context_explain` 内部经 daemon.Search → core gRPC → retriever.explain
- **task-6.2 retriever `Retriever::get_chunk` 公开 API + server.rs fast-path**（上游 ✅ done）：本 task `context_read` 复用 fast-path（query=chunk_id）
- **task-1.4 daemon.Start / HealthCheck / Stop**（上游 ✅ done）：本 task `contextforge mcp` 子命令复用持久 daemon supervisor pattern（AutoRestart=true）
- **task-1.2 config**（上游 ✅ done）：本 task data_dir 解析复用 `config.DefaultRootDir`；mcp-allowlist.json 路径默认 `<data_dir>/mcp-allowlist.json`
- **task-8.1 eval-harness**（下游强依赖 — MCP 自动化 smoke）：本 task 提供 phase-7 spec §6 命令骨架，task-8.1 接手大规模运行 + recall eval via MCP
- **PRD §Success Metrics 主指标消费者**：「上下文重建时间 ≤ 3-5 分钟」由 MCP tool 链端到端达成；agent 经 MCP 重建上下文不再手动 search/导入
- **PRD §Technical Risks R7 消费者**：MCP spec 漂移由 adapter 解耦 + 版本锁定缓解
- **PRD §Technical Risks R9 消费者**：MCP client allowlist + audit 缓解越权读取

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Technical Approach MCP tools / §Constraints Local service security baseline / §Technical Risks R7 R9 / §Open Questions O4 / §Vision 多 Agent 工作流 / §Success Metrics）
- `docs/specs/phases/phase-7-mcp-adapter.md`
- `docs/specs/tasks/task-6.1-cli-search.md`（上游：daemon.Search per-invocation）
- `docs/specs/tasks/task-6.2-rest-api.md`（上游：persistent daemon supervisor + audit.Write + retriever.get_chunk + server.rs fast-path + token mechanism）
- `docs/specs/tasks/task-6.3-exporter.md`（上游：lifecycle Mark/FilterStale 模式 — 可能本 task 也用）
- `docs/specs/tasks/task-4.2-explain.md`（上游：Retriever::explain + SearchResult 12 字段）
- `docs/specs/tasks/task-1.4-cli-init.md`（上游：CLI dispatch + daemon supervisor）
- `docs/specs/tasks/task-1.2-config.md`（上游：DataDir + 0600/0700 baseline）
- `docs/decisions/adr-003-cli-rest-mcp-grpc-interfaces.md`（result schema 单一源）
- `docs/decisions/adr-004-local-first-privacy-baseline.md`
- `test/features/mcp-adapter.feature`
- **MCP spec 2025-06-18** — https://modelcontextprotocol.io/specification/2025-06-18 (§2A 决策 E 锁定版本)

### 5.2 Imports

- **Go stdlib**:
  - `encoding/json`（JSON-RPC 2.0 message encode/decode；MCP frames）
  - `bufio`（stdin Scanner / Content-Length 解析）
  - `os` / `path/filepath` / `os/signal` / `syscall`（signal handling + 文件路径）
  - `io` / `bytes` / `errors` / `fmt` / `strings` / `strconv`
  - `context` / `time`（超时 + graceful shutdown）
  - `crypto/subtle`（如有 token check 复用 task-6.2 mechanism；MCP 不直接需 token，allowlist 是 client.name）
- **Go 内部（已有）**:
  - `github.com/tajiaoyezi/contextforge/internal/config`（DataDir / DefaultRootDir）
  - `github.com/tajiaoyezi/contextforge/internal/daemon`（Start / HealthCheck / Stop / Search — task-6.1 已 wire）
  - `github.com/tajiaoyezi/contextforge/internal/memoryops/audit`（audit.Write — task-6.2 已 wire；§2A 决策 D 复用）
- **Go proto（已有，task-1.1 codegen）**:
  - `contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"`（SearchRequest / SearchResponse / RetrievalResult — 同 task-6.x）
- **R7 严格通道**：不引入新 Go module / Rust crate（**特别**：不引 mcp-go / mcp-sdk / semver/Masterminds 等任一 MCP / semver SDK）；纯 stdlib + 已落定的 internal/proto/grpc

### 5.3 函数签名

**Go CLI** (`internal/cli/mcp.go` 新建；`cli.go` 内 dispatch case `"mcp"` 调 `runMCP`):

```go
package cli

import (
    "context"
    "flag"
    "io"
    "os"
    "os/signal"
    "syscall"

    "github.com/tajiaoyezi/contextforge/internal/config"
    "github.com/tajiaoyezi/contextforge/internal/mcpadapter"
)

// mcpOpts — flag 解析后状态.
type mcpOpts struct {
    DataDir   string // --data-dir (default config.DefaultRootDir())
    Allowlist string // --allowlist (default <DataDir>/mcp-allowlist.json)
}

// runMCP 实现 mcp 子命令 — stdio JSON-RPC MCP 2025-06-18 server.
// 返回 process exit code (0=ok / 1=运行错 / 2=usage 错)
//
// 1. 解析 flag → 加载 allowlist → daemon.Start(AutoRestart=true) → 等 SERVING
// 2. 注册信号处理 (SIGINT/SIGTERM) → graceful shutdown
// 3. mcpadapter.Server.Serve(ctx, stdin, stdout) — 阻塞至 stdin EOF 或 ctx Done
func runMCP(args []string, stdin io.Reader, stdout, stderr io.Writer) int

// parseMCPOpts — flag 解析；空 args = use defaults.
func parseMCPOpts(args []string) (*mcpOpts, error)
```

**Go mcpadapter package** (`internal/mcpadapter/*.go` 新建):

```go
// Package mcpadapter 实现手写 MCP 2025-06-18 stdio transport server.
// Contract: task-7.1 §5.3. R7 严格通道：无外部 MCP SDK / semver dep.
package mcpadapter

import (
    "context"
    "io"

    "github.com/tajiaoyezi/contextforge/internal/daemon"
)

// Server — MCP 2025-06-18 stdio JSON-RPC server.
type Server struct {
    Daemon    *daemon.Daemon    // task-6.1 wire 的 Search 入口
    DataDir   string            // audit.Write 用
    Allowlist []AllowlistEntry  // §2A 决策 C 启动时加载
}

// AllowlistEntry — JSON unmarshal target of <data_dir>/mcp-allowlist.json.
type AllowlistEntry struct {
    Name    string `json:"name"`    // MCP client name (claude-desktop / cursor / ...)
    Version string `json:"version"` // ">=X.Y.Z" 精确匹配 / 空 = 任意版本
}

// LoadAllowlist 读取 path JSON 文件并解析；不存在 → 空 allowlist (=拒绝所有).
// path mode 校验：要求 0600（warn if 不是）.
func LoadAllowlist(path string) ([]AllowlistEntry, error)

// Serve 主循环：读 stdin JSON-RPC 消息 → dispatch → 写 stdout 响应.
// ctx 取消时 graceful shutdown (finish in-flight request 然后返).
// stdin EOF 自然返 nil.
func (s *Server) Serve(ctx context.Context, stdin io.Reader, stdout io.Writer) error

// handleInitialize — MCP initialize handshake.
// 校验 params.clientInfo.name + .version 与 allowlist; 不匹配 → JSON-RPC error code -32000 + audit + close stdio.
// 返回 server.capabilities (tools: { listChanged: false }) + serverInfo.
func (s *Server) handleInitialize(ctx context.Context, params InitializeParams) (InitializeResult, error)

// handleListTools — MCP tools/list method (在 initialize 成功后).
// 返回 4 个 tool 的 schema: context_search / context_read / context_explain / context_collections.
func (s *Server) handleListTools(ctx context.Context) ([]ToolDef, error)

// handleCallTool — MCP tools/call dispatch.
// name → 对应 internal handler (search / read / explain / collections).
// 错误转 JSON-RPC error code.
func (s *Server) handleCallTool(ctx context.Context, name string, args map[string]any) (CallToolResult, error)

// 4 个 tool 的内部 handler:

// callContextSearch — MCP context_search tool → daemon.Search.
// args { query, collections?, agent_scope?, top_k?, filters?, explain? }
// result { results: [RetrievalResult ×N] }（与 REST /v1/search 一致 — ADR-003 单一 schema）
func (s *Server) callContextSearch(ctx context.Context, args map[string]any) (any, error)

// callContextRead — MCP context_read tool → daemon.Search fast-path (query=chunk_id, top_k=1).
// args { chunk_id, collection? }
// result single RetrievalResult or 404 error
func (s *Server) callContextRead(ctx context.Context, args map[string]any) (any, error)

// callContextExplain — MCP context_explain tool → daemon.Search with explain=true.
// args { query, ... } (same as context_search)
// result { results, retrieval_trace } (含 reason / matched_terms / provenance)
func (s *Server) callContextExplain(ctx context.Context, args map[string]any) (any, error)

// callContextCollections — MCP context_collections tool → scan <data_dir>/collections/.
// args {} (no input)
// result { collections: [{id, chunk_count, last_indexed_at}] } (同 task-6.2 handleCollections)
func (s *Server) callContextCollections(ctx context.Context, args map[string]any) (any, error)

// IsAllowlisted 比对 entry 与 allowlist (name 精确 + version >=X.Y.Z 或 空=任意).
// 不引外部 semver dep — inline 简化 parser: 仅支持 ">=X.Y.Z" + 精确匹配 + 空版本.
func IsAllowlisted(entry AllowlistEntry, allowlist []AllowlistEntry) bool

// JSON-RPC 2.0 types (MCP 2025-06-18 framing — newline-delimited per stdio transport):
type JSONRPCRequest struct {
    JSONRPC string         `json:"jsonrpc"` // "2.0"
    ID      any            `json:"id,omitempty"`
    Method  string         `json:"method"`
    Params  map[string]any `json:"params,omitempty"`
}

type JSONRPCResponse struct {
    JSONRPC string          `json:"jsonrpc"`
    ID      any             `json:"id"`
    Result  any             `json:"result,omitempty"`
    Error   *JSONRPCError   `json:"error,omitempty"`
}

type JSONRPCError struct {
    Code    int    `json:"code"`
    Message string `json:"message"`
    Data    any    `json:"data,omitempty"`
}

// MCP 2025-06-18 type definitions (inline; 不引 SDK):
type InitializeParams struct {
    ProtocolVersion string         `json:"protocolVersion"` // 客户端报；mcp-adapter 期望 = "2025-06-18"
    Capabilities    map[string]any `json:"capabilities"`
    ClientInfo      ClientInfo     `json:"clientInfo"`
}

type ClientInfo struct {
    Name    string `json:"name"`
    Version string `json:"version"`
}

type InitializeResult struct {
    ProtocolVersion string                 `json:"protocolVersion"` // = "2025-06-18"
    Capabilities    map[string]any         `json:"capabilities"`
    ServerInfo      ServerInfo             `json:"serverInfo"`
}

type ServerInfo struct {
    Name    string `json:"name"`
    Version string `json:"version"`
}

type ToolDef struct {
    Name        string         `json:"name"`
    Description string         `json:"description"`
    InputSchema map[string]any `json:"inputSchema"` // JSON Schema for tool args
}

type CallToolResult struct {
    Content []ToolContent `json:"content"`
    IsError bool          `json:"isError,omitempty"`
}

type ToolContent struct {
    Type string `json:"type"` // "text" / "image" / etc; v0.1 仅 "text"
    Text string `json:"text"` // JSON-stringified result for tool
}
```

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Implementation Phases Phase 7 Exit Criteria): MCP `context_search` 返回可解释结果，字段与 REST search result 一致。
- [ ] **AC2** (PRD §Implementation Phases Phase 7 Exit Criteria): MCP `context_read` 读取指定 chunk/context；`context_explain` 返回召回理由+provenance；`context_collections` 列出可用 collection。
- [ ] **AC3** (PRD §Constraints Local service security baseline / §Technical Risks R9): MCP client 未被 allowlist 时拒绝访问，访问写 audit log。
- [ ] **AC4** (PRD §Technical Risks R7 / §Open Questions O4): mcp-adapter 与核心检索解耦（仅协议翻译）；锁定一个已发布 MCP spec 版本并在 spec 标注兼容范围（**§2A 决策 E 锁定 MCP spec 2025-06-18**；2025-11-25 等 newer 版本接入留 future SPEC-DRIFT-task-7.1.spec-bump）。
- [ ] **AC5** (本 task 新增): Phase 7 端到端 smoke 可执行（起 MCP server → client 调 4 tool 校验字段与 REST 一致 + 未 allowlist client 被拒），本 task 填实 phase-7 spec §6 端到端 smoke 命令骨架（自动化运行留 task-8.1 eval-harness）。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 context_search 一致字段 | SCEN-7.1.1 | TEST-7.1.1 | - | unit-test | Done |
| AC2 read/explain/collections | SCEN-7.1.2 | TEST-7.1.2 | - | unit-test | Done |
| AC3 client allowlist 拒绝+审计 | SCEN-7.1.3 | TEST-7.1.3 | - | unit-test | Done |
| AC4 adapter 解耦+版本锁定 | SCEN-7.1.4 | TEST-7.1.4 | - | unit-test | Done |
| AC5 Phase7 端到端 smoke 骨架 | SCEN-7.1.5 | TEST-7.1.5 | phase-7 spec §6 | unit-test | Done |

## 8. Risks

- 关联 PRD §Technical Risks **R7**（MCP 协议/SDK 漂移）：§2A 决策 A 手写 MCP（不引 SDK 即 SDK 漂移风险归零）+ §2A 决策 E 锁定 spec 2025-06-18 + adapter 解耦核心 → R7 全程缓解
- 关联 PRD §Technical Risks **R9**（MCP client 越权读取）：§2A 决策 C `<data_dir>/mcp-allowlist.json` 文件 0600 + initialize handshake 校验 + 默认空 allowlist=拒绝所有（用户必须显式加 entry）+ audit 记拒访问
- 关联 PRD §Open Questions **O4**（Phase 7 启动前锁定 MCP 目标版本）：§2A 决策 E 锁定 2025-06-18 → O4 resolved
- **MCP 2025-06-18 vs 2025-11-25 版本差距**：current per modelcontextprotocol.io 是 2025-11-25（5+ 月新发布）；本 task 选 2025-06-18 是「交接面更熟成」（5+ 月市场适配）trade-off；2025-11-25 newer features 留 future SPEC-DRIFT-task-7.1.spec-bump；用户 / Claude Desktop / Cursor 等 client 若强制 2025-11-25 → fallback negotiate down to 2025-06-18 (MCP initialize handshake 原生 supports)
- **手写 MCP 风险**：~300-500 行 Go 实现 MCP 2025-06-18 stdio transport + JSON-RPC 2.0 + initialize / tools/list / tools/call。**缓解**：(a) MCP 2025-06-18 spec stable + 文档完整；(b) v0.1 仅 4 tool / 仅 tools 能力（不实施 resources/prompts/sampling）；(c) JSON-RPC 2.0 stdlib `encoding/json` 直接 marshal；(d) 后续 v0.2 引 mcp-go SDK 留 SPEC-DRIFT-task-7.1.adopt-sdk
- **allowlist 默认空=拒绝所有的可用性 friction**：新用户启动 `contextforge mcp` 立刻报 client not allowlisted。**缓解**：spec §10 + CLI startup 输出 stderr 含 onboarding hint（"请编辑 `<data_dir>/mcp-allowlist.json` 添加 client; 示例 `[{\"name\":\"claude-desktop\"}]`"）
- **inline semver parser 简化（仅 `>=X.Y.Z` + 精确）**：实际 semver 含 `^X.Y.Z` / `~X.Y.Z` / `X.Y.x` 等。v0.1 不支持，spec §3 Out of Scope 明示；用户配 allowlist 时只能用支持的格式；不支持的格式 → warn + 拒接受 entry
- **`internal/cli/cli.go` dispatch case `"mcp"` 修改 rebase 风险**（本 task 单独修改，无并行同 case worker — 无 rebase 风险）

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。⚠️ 本 task 为 Phase 7 唯一 / phase-last task：完工/合并前 phase-7 spec §6 端到端 smoke 必须填实（C1 / team §4 Gate 3）。本 task §3 In Scope 已含填实 phase-7 spec §6 — 完工时主 agent §4 Gate 3 phase-7 §6 端到端 smoke 触发（syntactic + 命令骨架完整即可；自动化运行留 task-8.1）。

## 10. Completion Notes

- **完成日期**：2026-05-23
- **改动文件**：
  - `internal/mcpadapter/jsonrpc.go`（新增：MCP 2025-06-18 JSON-RPC 2.0 newline stdio framing）
  - `internal/mcpadapter/allowlist.go`（新增：allowlist JSON 加载 + 简化 version matcher）
  - `internal/mcpadapter/server.go`（新增：initialize / tools/list / tools/call 主循环 + audit）
  - `internal/mcpadapter/tools.go`（新增：context_search / context_read / context_explain / context_collections）
  - `internal/cli/mcp.go`（新增：`contextforge mcp` 参数解析 + backend hook）
  - `internal/cli/cli.go`（修改：`mcp` dispatch + stdin-aware `ExecuteWithIO`）
  - `cmd/contextforge/main.go`（修改：生产 MCP backend 注入 + daemon wiring）
  - `internal/cli/cli_test.go`（修改：`mcp` 从 not-implemented 清单移除）
  - `internal/mcpadapter/*_test.go`（新增：TEST-7.1.1~4）
  - `internal/cli/mcp_test.go`（新增：TEST-7.1.5）
  - `test/features/mcp-adapter.feature`（修改：SCEN-7.1.1~5 填实）
  - `docs/specs/phases/phase-7-mcp-adapter.md`（修改：§6 端到端 smoke shell + JSON-RPC pipe 命令骨架）
  - `docs/s2v-adapter.md`（修改：Task 总索引 7.1 状态同步）
  - `docs/specs/tasks/task-7.1-mcp-server.md`（修改：Status / §6 / §7 / §10 回填）
- **commit 列表**：
  - `4623926` test(mcp-server): 加 SCEN-7.1.1~5 共 5 个 RED 测试 + Status: Ready → In Progress
  - `82f825e` feat(mcp-server): contextforge mcp 端到端实现 — 手写 MCP 2025-06-18 stdio JSON-RPC + 4 tool + allowlist + audit 通过全部 5 个测试 + phase-7 §6 端到端 smoke 命令骨架填实
  - `本 docs commit` docs(spec): 回填 task-7.1 Completion Notes + Status → Done
- **§9 Verification 结果**：
  - install: ✅ `go mod download && cargo fetch`
  - typecheck: ✅ `go vet ./... && cargo check --workspace`
  - unit-test: ✅ `go test ./...`（111 passed / 0 failed；13 package pass，含 `internal/mcpadapter` 13 passed）+ `cargo test --workspace`（59 passed / 0 failed：35 core lib + 4 skeleton + phase2/4/5/6 smokes + 5 proto + 11 scanner）
- **剩余风险 / 未做项**：Phase 7 §6 为可执行命令骨架；自动化 fixture seeding / MCP smoke 编排按 spec 留 task-8.1 eval-harness。
- **下游 task 影响**：task-8.1 eval-harness 可消费 `contextforge mcp` 与 phase-7 §6 smoke 骨架；无 proto / dependency / lockfile 影响。
- **§2A Decisions**（2026-05-23 用户审定，主 agent 与用户预先审定后落 spec；worker 完工时按实际实施情况验证 / 补充）：
  - **A: 手写 MCP JSON-RPC over stdio（R7 严格通道，不引 SDK）**：v0.1 选 stdlib `encoding/json` + `bufio` 手写 MCP 2025-06-18 stdio transport + JSON-RPC 2.0 实现（initialize / tools/list / tools/call）。不引 mcp-go / mcp-sdk / 任一 MCP SDK；零 supply chain surface；漂移隐藏在 adapter 层；与 ContextForge minimal 主义一致。代价：~300-500 行 Go 实现量；benefit：完全可控
  - **B: stdio subprocess + 新 `contextforge mcp` 子命令**：MCP 原生 mode；agent (Claude Desktop / Cursor / Zed) 启动 `contextforge mcp` 为 subprocess + stdio JSON-RPC 通信。task-1.4 default not-implemented 的 `mcp` 子命令本 task 接上（同 task-6.1 search / 6.2 serve / 6.3 export pattern）。HTTP/SSE transport 留 future v0.2+
  - **C: `<data_dir>/mcp-allowlist.json` 启动读 + initialize handshake 验证**：文件 0600；JSON `[{"name":..., "version":">=X.Y.Z"}, ...]`；默认空 allowlist = 拒绝所有（用户必须显式加 entry）；不匹配 → JSON-RPC error code -32000 + close stdio + audit.Write 记。inline 简化 semver parser (仅 `>=X.Y.Z` + 精确)，完整 semver 留 future
  - **D: 复用 task-6.2 `internal/memoryops/audit/` 同 audit-rest.log + Endpoint 字段 prefix `mcp:`**：MCP 访问写同 `<dataDir>/audit-rest.log`；Event.Endpoint = `mcp:initialize` / `mcp:context_search` / etc 与 REST endpoint `/v1/search` 区分；运维统一查询 + grep `mcp:` 分类；零新代码 / 零新文件 / 完美对齐 PR #45 phase-6 closeout 修正的 audit Go vs Rust 互补设计
  - **E: MCP spec lock 2025-06-18（保守 / 交接面熟成）**：modelcontextprotocol.io 今天 (2026-05-23) 显示 current = **2025-11-25**（5+ 月新发布）；本 task 选 2025-06-18 是熟成度优先 trade-off — 5+ 月市场适配（Claude Desktop / Cursor / Zed / 等 agent）；2025-11-25 newer features (含 minor protocol changes) 留 future SPEC-DRIFT-task-7.1.spec-bump；client 报高版本 → MCP initialize handshake 原生 supports 协议降级 negotiate to 2025-06-18
  - **R7 严格通道**：未引入新 Go module / Rust crate；纯 stdlib (`encoding/json` / `bufio` / `crypto/subtle` / `context` / `os` / `os/signal` / `syscall` / `path/filepath` / `time` / `strconv`) + 已落定 internal (config / daemon / memoryops/audit) + proto (contextforgev1)；不引 mcp-go / mcp-sdk / semver/Masterminds / 等任一第三方
