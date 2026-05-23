# Task `6.1`: `cli-search — contextforge search 命令`

> ✅ 已过 `/s2v-implement` §2A 前置审核（2026-05-23，主 agent 与用户预先审定，worker 终端可直接进入 RED）：§3/§4/§5.2/§5.3 `<TBD-by-user>` 已清零、§6 AC 经用户审定接受、A/B/C/D/E 五决策已确认（A. AC1 Rust tonic Search wire 本 task 端到端、B. CLI per-invocation spawn、C. AC4 secret 透传 redaction_status、D. AC5 直接用 proto-generated RetrievalResult、E. Provenance 时间字段 v0.1 placeholder — 详见 §10 §2A Decisions）。**PR #37 review 补判定**：原草稿 `RetrieverError::DataDirMissing` 是写错（实际 5 变种 `Io/Sqlite/Tantivy/InvalidConfig/CollectionNotFound`）；chunker::Provenance 时间字段是 `String` 不是 `chrono::DateTime`（→ §2A 决策 E）。实时状态以下方 `**Status**` 字段为准；状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: Done

**Priority**: P0
**Owner**: tajiaoyezi
**Related Phase**: Phase 6 (cli-api-export)
**Dependencies**: Phase 4 (retrieval-explain) + Phase 5 (memoryops)

## 1. Background

把可解释检索对外暴露为用户最常用入口 `contextforge search`（PRD §User Flow 主流程步 3 / §Core Capabilities #2）。Phase 6 首个 task，6.2/6.3 依赖其命令骨架与 Go↔Rust gRPC Search wire。

## 2. Goal

`contextforge search "<query>" [--collections --agent-scope --top-k --filters --explain]` 经 Go CLI → 自启 daemon (内嵌 Rust core) → gRPC `ContextService::Search` → Rust `Retriever::explain` 返回 12-field 可解释 RetrievalResult；CLI 默认人类可读输出，`--json` 输出结构化 `SearchResponse`。继承 task-4.2 explain 单一源 schema、task-1.4 daemon supervise pattern。

## 3. Scope

### In Scope

- **Rust 侧 — `core/src/server.rs` `CoreService::search` wire（§2A 决策 A: 本 task 端到端真走通）**：
  - 替换 task-1.3 写的 `Status::unimplemented` 占位 → 真实业务实现
  - `CoreService` 新增 `data_dir: PathBuf` 字段 + `pub fn new(data_dir: PathBuf) -> Self`（保持 `Default` impl 走 `PathBuf::new()` 兼容现存测试入口）
  - `core/src/main.rs` 启动时把 cmd-arg listen_addr 之后的第 2 个 arg（或 env `CONTEXTFORGE_DATA_DIR`）传入 `CoreService::new(data_dir)`（向后兼容：缺省走 `config::DefaultRootDir` 等价路径解析）
  - search() 实现：
    - 校验 `req.collections` 非空（v0.1 P0 仅消费 `collections[0]`；为空 → `Status::invalid_argument("collections is required (v0.1 single-collection)")`）
    - `Retriever::open(&data_dir, &collections[0])` → 错误映射（按 task-4.1 / 4.2 实际暴露的 `RetrieverError` 5 变种 `Io / Sqlite / Tantivy / InvalidConfig / CollectionNotFound`）：`RetrieverError::Io`（NotFound — 路径不存在）/ `RetrieverError::CollectionNotFound` → `Status::failed_precondition`；`RetrieverError::InvalidConfig` → `Status::invalid_argument`；`RetrieverError::Sqlite` / `RetrieverError::Tantivy` / `RetrieverError::Io`（非 NotFound） → `Status::internal`
    - 映射 `SearchRequest → SearchOptions`：`query`、`top_k`（≤0 → 默认 10）、`agent_scope`、`filters.source_type`、`filters.language`、`explain`
    - explain=true → `retriever.explain(opts)`；否则 `retriever.search(opts)`（同 task-4.2 公开 API）
    - 映射 `Vec<retriever::SearchResult> → Vec<proto::RetrievalResult>`：12 字段 1:1（`provenance` 用 `chunker::Provenance → proto::Provenance` field mapping helper；`google.protobuf.Timestamp` 走 `prost_types::Timestamp`）
- **Go 侧 — `internal/cli/search.go` 实现 search 子命令**：
  - 入口：`Execute` 现有 dispatch case `"search"` 不再走 `not implemented` 默认分支，改为调本 task 新增 `runSearch`
  - flags（用 stdlib `flag.FlagSet`，沿 task-1.4 风格不引第三方 cobra/cli）：
    - `query` 取 positional arg 第 1 个；空 → stderr usage + exit 2
    - `--collections=<id1,id2>` 逗号分隔（v0.1 仅取第一个；多 collection 联邦 留 Phase 6+）
    - `--agent-scope=<a,b>` 逗号分隔
    - `--top-k=N` int32（default 10；≤0 → 同 default）
    - `--source-type=<t1,t2>` / `--language=<l1,l2>` 逗号分隔（映射 `SearchFilters`）
    - `--explain` bool（透传 SearchRequest.explain）
    - `--json` bool（输出 marshaled `SearchResponse`；默认走 human-readable text）
  - 渲染：
    - text 模式：每条结果 1 块 `chunk_id <file_path>:<line_start>-<line_end>  score=<score>  redaction_status=<status>` + `reason=<reason>`（仅 explain=true 非空时打印）+ 第二行 truncate 后的 content 一行（≤ 120 chars 后追加 `…`）+ 空行分隔
    - json 模式：`json.Marshal(resp)` 写 stdout（用 stdlib `encoding/json`，不引 proto-json — RetrievalResult 字段是 plain scalar/repeated，标准 encoding/json 足够）
  - daemon 生命周期（§2A 决策 A: per-invocation spawn）：
    - `ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)` — 整体超时
    - `daemon.Start(ctx, daemon.Options{CoreBinPath: "", ListenAddr: "", AutoRestart: false})` — 自动找 core bin + 自动选 loopback 端口；不需 AutoRestart（一次查询）
    - 等 `daemon.HealthCheck` 返 `"SERVING"`（轮询 ≤ 15s；超时 → stderr 报错 + exit 1）
    - 调 `daemon.Search(ctx, req)` 拿响应
    - `defer daemon.Stop()` 收尾
- **Daemon 侧 — `internal/daemon/search.go` 新增 `Daemon.Search`**：
  - `func (d *Daemon) Search(ctx context.Context, req *contextforgev1.SearchRequest) (*contextforgev1.SearchResponse, error)`
  - 复用 `d.clientConn()` 单条 gRPC client（懒初始化 + Stop 时关）
  - 调 `contextforgev1.NewContextServiceClient(conn).Search(ctx, req)` → 直接 forward 响应 / 错（不吞 gRPC `Status`）
- **AC4 secret redaction（§2A 决策 A: 透传）**：CLI text/JSON 渲染时**直接打 `redaction_status` 字段值**（"applied" / "" / etc）；不在 CLI 二次扫描 content；上游 scanner+indexer+retriever 已保证 `content` = `redacted_content`（task-4.2 §10 留档）；本 task 端到端假定上游契约。
- **AC5 共享 result model（§2A 决策 A: 直接用 proto-generated `contextforgev1.RetrievalResult`）**：
  - 不新建 Go wrapper struct；不写 ProtoTo* / FromProto* 适配
  - task-6.3 exporter 直接消费 `*contextforgev1.RetrievalResult` 序列化为 JSONL / Markdown bundle / agent draft
  - 本 task 不需为 task-6.3 暴露任何 helper；shared model = proto package 单一源
- **新增 RED→GREEN 测试**（5 个）：
  - `TEST-6.1.1 ~ TEST-6.1.5` 落在以下文件（按测试关注点拆分，参考 task-4.2 / 5.2 multi-file 测试拆分先例）：
    - `internal/cli/search_test.go`（AC1 / AC2 / AC3 / AC4 — flag parsing + 渲染 + redaction_status 显示）
    - `core/src/server.rs` `#[cfg(test)] mod tests`（CoreService::search wire 单元 — 用 in-memory tempdir Retriever 验真实拿到 12 字段）
    - `core/tests/phase6_smoke.rs`（AC5 端到端 smoke — 索引 fixture + 调 Rust gRPC Search + 验 RetrievalResult 12 字段；pattern 同 phase2_smoke / phase4_smoke）
  - **测试拆分指引**：CLI 用 fakeDaemon mock gRPC（无需实际 spawn core；快）；Rust server wire 用 in-process Retriever（不走 tonic transport；快）；smoke 走完整 Go → Rust gRPC 端到端（慢 1-2s 因 cargo build + core 启动，但每 phase 仅 1 个 smoke 测试可接受）
- **填实 `test/features/cli.feature` SCEN-6.1.1 ~ SCEN-6.1.5** 的占位 Given/When/Then（与 TEST-6.1.X 一一映射，SCEN 名沿现有模板不改）

### Out Of Scope

- **REST `/v1/search`**（HTTP wrapper 与 daemon-side REST handler）：留 task-6.2；task-6.2 复用本 task 已 wire 的 Rust tonic Search server + Go daemon.Search 包装
- **`contextforge serve` 持久 daemon 子命令**：§2A 决策 A 走 per-invocation spawn；持久 daemon 留 Phase 6 后续 task（如未来 task-6.x daemon-lifecycle）/ Phase 7 MCP server。本 task **不** 实现 `serve` 子命令
- **MCP `context_search` tool**：留 task-7.1（MCP wrap 同 REST 形态，复用本 task 的 Rust gRPC Search）
- **export 命令（jsonl / markdown-bundle / agent draft）**：留 task-6.3
- **跨 collection 联邦查询**：v0.1 P0 仅消费 `req.collections[0]`；多 collection 联邦留 Phase 6+ / future task
- **认证 / TLS / token**：v0.1 沿 task-1.4 设计，loopback plaintext（127.0.0.1 强制 + 0.0.0.0 拒绝），不引入 TLS / token 认证。本地暴露面缓解走 daemon ensureLoopback（PRD §Technical Risks R9）
- **embeddings / hybrid retrieval / reranker**：v0.1 仅 BM25（task-4.1 retriever 默认）；hybrid 留 Phase 8+ / ADR-002 已留 provider 抽象
- **修改 `Cargo.toml` / `go.mod` / `Cargo.lock` / `go.sum`**：R7 严格通道。无新依赖（CLI 用 stdlib flag/json；daemon/Rust 沿用已有 grpc/tonic/tokio/prost-types）
- **修改 task-4.1/4.2/5.2/5.3 已 merge 的契约 / 内部结构**：retriever / lifecycle / dedup / audit struct 不动。Rust SearchResult schema 已在 task-4.2 freeze；本 task 仅做 `retriever::SearchResult → proto::RetrievalResult` field mapping
- **修改 proto `*.proto` 文件**：proto frozen 在 task-1.1 / phase23-start-gate（仅 add-only field tag）；本 task 不改 proto，所有结构沿 task-1.1 已生成的 contextforgev1
- **改 `internal/cli/cli.go` 现有 `init` 子命令实现 / `daemon.Start/HealthCheck/Stop` 内部行为**：本 task 仅在 dispatch case `"search"` 上叠加，daemon 仅新增 `Search` 方法
- **CLI 输出 stale_marks / conflict_reports**（task-5.2/5.3 lifecycle 输出）：v0.1 search 命令仅返 retriever 结果，不调用 lifecycle.Mark；lifecycle 集成留 task-6.2 REST handler 或 future task-6.4
- **持久缓存检索结果 / pagination cursor**：v0.1 单次 stateless 调用；分页 / 缓存留 Phase 6+

## 4. Users / Actors

- **PRD §User Flow 主流程 step 3 用户**（业务消费）：通过 `contextforge search "<query>"` 在终端拿到 12 字段可解释 Top-K 结果；脚本化场景用 `--json`
- **task-4.2 `Retriever::explain` / `Retriever::search`**（上游 ✅ done）：本 task Rust 侧 `CoreService::search` 调用其公开 API；不改 retriever
- **task-1.4 `daemon` package**（上游 ✅ done）：本 task 复用 `daemon.Start` / `daemon.HealthCheck` / `daemon.Stop`；新增 `daemon.Search` 方法 — 沿用现有 `clientConn` 单连接懒初始化
- **task-1.3 `core/src/server.rs::serve`**（上游 ✅ done）：本 task 替换 `CoreService::search` 内部实现（`Default` impl 保留以不破现存 `core::Default` test 入口）
- **task-2.4 indexer SQLite 数据**（上游软依赖）：retriever 读 indexer 落盘的 chunks + provenance（前置：用户已跑过 `contextforge init` + 未来 `contextforge import` 走 importer 落数据）
- **task-6.2 REST API**（下游强依赖）：复用本 task 已 wire 的 Rust gRPC Search server + Go `daemon.Search` 方法（HTTP handler 包装 `daemon.Search`）
- **task-6.3 exporter**（下游强依赖 AC5）：直接消费 `*contextforgev1.RetrievalResult` 序列化为 JSONL / Markdown bundle / agent draft
- **task-7.1 MCP `context_search` tool**（下游强依赖）：MCP tool handler 复用同 Rust gRPC Search wire
- **task-8.1 eval-harness**（下游）：可调 `contextforge search --json` 跑 recall eval / 可解释字段覆盖率回归
- **PRD §Success Metrics 主指标消费者**：「上下文重建时间 ≤ 3-5 分钟」由本 task 命令骨架 + retrieval pipeline 联合达成
- **PRD §Technical Risks R9 消费者**：本地 daemon 暴露面 — 本 task 不引入新监听口，复用 daemon ensureLoopback / freeLoopbackAddr 全程 127.0.0.1

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§User Flow 主流程 / §Technical Approach REST/MCP search 契约 / §Success Metrics / §Technical Risks R9）
- `docs/specs/phases/phase-6-cli-api-export.md`
- `docs/specs/tasks/task-4.2-explain.md`（上游：Retriever::explain API + SearchResult schema 12 字段）
- `docs/specs/tasks/task-4.1-retriever.md`（上游：Retriever::open + SearchOptions schema）
- `docs/specs/tasks/task-1.4-cli-init.md`（上游：CLI dispatch / daemon.Start / HealthCheck pattern）
- `docs/specs/tasks/task-1.3-core-skeleton.md`（上游：CoreService skeleton + listen_addr 安全基线）
- `docs/specs/tasks/task-1.1-proto.md`（proto contract freeze 规则）
- `docs/decisions/adr-003-cli-rest-mcp-grpc-interfaces.md`
- `docs/decisions/adr-001-go-rust-dual-binary-architecture.md`
- `test/features/cli.feature`

### 5.2 Imports

- **Go stdlib**:
  - `flag` / `fmt` / `io` / `os` / `strings`（CLI parsing + 渲染）
  - `encoding/json`（--json 模式 marshal SearchResponse；proto-generated struct 走 plain JSON encoding 够用）
  - `context` / `time`（daemon spawn 超时控制）
- **Go 内部（已有）**:
  - `github.com/tajiaoyezi/contextforge/internal/config`（DataDir 解析 — 复用 task-1.4 DefaultRootDir 等价路径）
  - `github.com/tajiaoyezi/contextforge/internal/daemon`（spawn + HealthCheck + 新 Search 方法）
- **Go proto（已有，task-1.1 codegen）**:
  - `contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"`（SearchRequest / SearchResponse / RetrievalResult / SearchFilters / Provenance）
- **Go gRPC（已有）**:
  - `google.golang.org/grpc`（daemon 已引入；本 task 不引新 module）
  - `google.golang.org/grpc/credentials/insecure`（loopback plaintext）
- **Rust 内部**:
  - `use crate::pb::context_service_server::ContextService`（trait 实现位）
  - `use crate::pb::{SearchRequest, SearchResponse, RetrievalResult, Provenance as PbProvenance}`（proto codegen）
  - `use crate::retriever::{Retriever, SearchOptions, SearchResult, RetrieverError}`（task-4.2 公开 API）
  - `use crate::chunker::Provenance`（mapping chunker::Provenance → PbProvenance 用）
- **Rust 第三方（已有）**:
  - `tonic`（codegen + Status + Request/Response）
  - `tokio` / `async-trait`（沿 task-1.3）
  - `prost-types`（`Timestamp` 类型用于 PbProvenance.imported_at / source_modified_at；Cargo.toml 已声明）
  - **时间字段映射（FIX-2 / §2A 决策 E）**：`chunker::Provenance.imported_at` / `source_modified_at` 实际类型是 **`String`**（RFC3339 with Z suffix，indexer SQLite TEXT 直存原样；chrono 不在 Cargo.toml 中，不引入新 dep）。v0.1 P0 选项 X — placeholder：proto.Provenance.imported_at / source_modified_at 一律返 `prost_types::Timestamp::default()`（seconds=0 nanos=0）+ §8 Risks 留档时间字段保真 gap，下游 task-6.3 export 必要时引入 R7 chore-dep 推 `time` / `chrono` crate；CLI text 渲染时直接 print `chunker::Provenance.imported_at` String（不经 proto.Provenance.imported_at），用户终端仍能看到原始 RFC3339 串
- **R7 严格通道**：**不引入新 Go module / Rust crate**；不改 `Cargo.toml` / `Cargo.lock` / `go.mod` / `go.sum`。所有依赖沿 task-1.3 / 1.4 / 4.2 已落定的版本。

### 5.3 函数签名

**Go CLI** (`internal/cli/search.go` 新增；`internal/cli/cli.go` Execute 内 `"search"` case 改为 `return runSearch(rest, stdout, stderr)`):

```go
// Package cli (sub-file search.go) — task-6.1 cli-search 实现。
// Contract: task-6.1 §5.3.

package cli

import (
    "context"
    "encoding/json"
    "flag"
    "fmt"
    "io"
    "strings"
    "time"

    "github.com/tajiaoyezi/contextforge/internal/daemon"
    contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// searchOpts 内部 flag 解析后的状态容器（仅本包用；映射到 proto SearchRequest）。
type searchOpts struct {
    Query       string
    Collections []string
    AgentScope  []string
    TopK        int32
    SourceType  []string
    Language    []string
    Explain     bool
    JSON        bool
}

// runSearch 实现 search 子命令（AC1-5 统一入口）。
//
// AC1: 自启 core + 调 gRPC Search → 返 Top-K 可解释结果
// AC2: --collections / --agent-scope / --top-k / --filters / --explain flag 与 SearchRequest 契约 1:1
// AC3: text 默认（人类可读）/ --json（structured SearchResponse JSON）二选一
// AC4: 渲染只读 RetrievalResult.RedactionStatus；不二次扫 content
// AC5: 返回值直接是 *contextforgev1.SearchResponse / RetrievalResult（与 task-6.3 共享）
//
// 返回 process exit code（同 cli.Execute 约定）：0=ok / 2=usage 错 / 1=运行错
func runSearch(args []string, stdout, stderr io.Writer) int

// parseSearchOpts 把 args 解析为 searchOpts；query 取 positional arg；逗号分隔展开为 []string。
// 错误时 fs.Output 已写 usage，调用方只需返 exit 2。
func parseSearchOpts(args []string, stderr io.Writer) (*searchOpts, error)

// optsToProtoRequest 把 searchOpts 映射为 *contextforgev1.SearchRequest（含 Filters 嵌套）。
func optsToProtoRequest(o *searchOpts) *contextforgev1.SearchRequest

// renderText 把 SearchResponse 写人类可读文本到 stdout（每结果块状）。
// AC3 text 模式 + AC4 透传 redaction_status 字段值。
func renderText(resp *contextforgev1.SearchResponse, w io.Writer) error

// renderJSON 把 SearchResponse 写 structured JSON 到 stdout（AC3 --json）。
// 用 stdlib encoding/json（不引 protojson — proto 字段是 plain scalar/repeated 够用）。
func renderJSON(resp *contextforgev1.SearchResponse, w io.Writer) error
```

**Go daemon** (`internal/daemon/search.go` 新增；`daemon.go` 不动):

```go
// Package daemon (sub-file search.go) — task-6.1 daemon.Search 包装。
// Contract: task-6.1 §5.3.

package daemon

import (
    "context"
    "fmt"

    contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// Search forwards a SearchRequest to contextforge-core via the reused
// gRPC client conn (lazy-init by clientConn). Errors and gRPC Status
// codes are forwarded as-is — caller decides on retry / exit code.
//
// task-6.1 §5.3 contract: caller is internal/cli/search.go (per-invocation
// spawn pattern; CLI calls Start, polls Health, calls Search, then Stop).
// task-6.2 REST handler also uses this method.
func (d *Daemon) Search(ctx context.Context, req *contextforgev1.SearchRequest) (*contextforgev1.SearchResponse, error)
```

**Rust core** (`core/src/server.rs` 修改 `CoreService` 字段 + `new` + `search`):

```rust
//! task-6.1: CoreService::search wire 升级 — 替换 task-1.3 unimplemented 占位.

use std::path::PathBuf;

use prost_types::Timestamp;
use tonic::{Request, Response, Status};

use crate::chunker::Provenance as RetrieverProvenance;
use crate::pb::{
    Provenance as PbProvenance, RetrievalResult, SearchRequest, SearchResponse,
};
use crate::pb::context_service_server::ContextService;
use crate::retriever::{Retriever, RetrieverError, SearchOptions, SearchResult};

/// gRPC service impl for the data plane.
/// task-6.1 §5.3: 新增 `data_dir` 字段，让 search() 能 open Retriever；
/// `Default` impl 保留以不破 task-1.3 / 1.4 / phase 1-5 现存测试入口（默认 PathBuf::new() = 空 path）.
#[derive(Debug, Default, Clone)]
pub struct CoreService {
    pub data_dir: PathBuf,
}

impl CoreService {
    /// task-6.1: 显式构造 — main.rs 启动时把 data_dir 注入.
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }
}

#[tonic::async_trait]
impl ContextService for CoreService {
    // ---- task-1.3 已实现 ----
    async fn health(/* 不动 */) -> Result<Response<HealthResponse>, Status>;

    // ---- task-6.1 升级 ----

    /// task-6.1 §5.3: SearchRequest → Retriever.search/explain → SearchResponse.
    ///
    /// 错误映射（按 task-4.1/4.2 暴露的 RetrieverError 5 变种）:
    ///   collections 为空 → InvalidArgument
    ///   RetrieverError::Io (NotFound) / CollectionNotFound → FailedPrecondition
    ///   RetrieverError::InvalidConfig → InvalidArgument
    ///   RetrieverError::Sqlite / Tantivy / Io (非 NotFound) → Internal
    async fn search(
        &self,
        req: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status>;
}

/// task-6.1 §5.3: chunker::Provenance → proto::Provenance field mapping.
///
/// **时间字段映射（§2A 决策 E）**：chunker::Provenance.imported_at /
/// source_modified_at 实际类型是 `String`（RFC3339 with Z；indexer SQLite TEXT
/// 直存）。v0.1 P0 placeholder：
///   imported_at:        prost_types::Timestamp::default() (seconds=0, nanos=0)
///   source_modified_at: prost_types::Timestamp::default()
/// 不引入 chrono / time crate（R7 严格通道）。CLI text 渲染时直接 print
/// chunker::Provenance 原 String 字段（不走 proto.Provenance.imported_at），
/// 用户终端仍能看 RFC3339 原值；--json 输出会显示 placeholder timestamps —
/// §8 Risks 留档，下游 task-6.3 export 触发 SPEC-DRIFT 走 R7 chore-dep 修复.
fn provenance_to_proto(p: &RetrieverProvenance) -> PbProvenance;

/// task-6.1 §5.3: retriever::SearchResult → proto::RetrievalResult.
/// 12 字段 1:1 + Provenance 列表映射.
fn search_result_to_proto(r: &SearchResult) -> RetrievalResult;
```

**Rust core** (`core/src/main.rs` 改 — 新增 data_dir 解析 + 传入 CoreService::new):

```rust
// task-6.1 §5.3: 启动时把 data_dir 注入 CoreService.
// 接受形式（向后兼容 task-1.3 的单 arg listen_addr）：
//   contextforge-core [listen_addr] [data_dir]
// 缺省 data_dir = env CONTEXTFORGE_DATA_DIR / ~/.contextforge / 等价 DefaultRootDir
fn resolve_data_dir(arg: Option<&str>) -> PathBuf;
```

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [x] **AC1** (PRD §Implementation Phases Phase 6 Exit Criteria): `contextforge search "<query>"` 可用并返回 Top-K 可解释结果。
- [x] **AC2** (PRD §Technical Approach REST/MCP 契约): 支持 `--collections / --agent-scope / --top-k / --filters / --explain`，语义与 search 请求契约一致。
- [x] **AC3** (PRD §Core Capabilities #2): 结果含全部可解释字段，CLI 人类可读输出 + `--json` 结构化输出二选一。
- [x] **AC4** (PRD §Constraints 安全): 结果默认不展示完整 secret（redaction_status 透传，复用 scanner/explain 行为）。
- [x] **AC5** (PRD §User Flow 主流程 5 步): search 与后续 export 命令共享检索结果模型，为 6.3 export search-result 提供接口。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 search 返回 Top-K | SCEN-6.1.1 | TEST-6.1.1 | - | unit-test | Done |
| AC2 flags 契约一致 | SCEN-6.1.2 | TEST-6.1.2 | - | unit-test | Done |
| AC3 可解释字段+--json | SCEN-6.1.3 | TEST-6.1.3 | - | unit-test | Done |
| AC4 不展示完整 secret | SCEN-6.1.4 | TEST-6.1.4 | - | unit-test | Done |
| AC5 与 export 共享结果模型 | SCEN-6.1.5 | TEST-6.1.5 | core/tests/phase6_smoke.rs | unit-test | Done |
## 8. Risks

- 关联 PRD §Technical Risks **R9**（本地暴露面）：CLI 经 daemon 走本地 gRPC；daemon.Start 复用 task-1.4 `ensureLoopback` / `freeLoopbackAddr`，全程 127.0.0.1，不引入新监听口。无新 attack surface（task-1.4 缓解措施延续）。
- **CLI 冷启动延迟（§2A per-invocation spawn 后果）**：每次 `contextforge search` 都要 spawn `contextforge-core` + 等 gRPC SERVING，单次延迟约 0.5-2s（cargo target/release 启动 + tonic listener bind + Tantivy index open）。v0.1 P0 用户脚本化场景可接受；持续 / 高频检索场景留 Phase 6+ daemon-lifecycle task 切持久 daemon。本 task 不优化。
- **5 schema-gap 字段返默认值**（context_id / source_type / agent_scope / redaction_status — task-4.2 §10 留档）：本 task 透传 retriever 输出；用户 CLI 看到 `redaction_status=applied`（v0.1 default）/ 4 字段为空。SPEC-DRIFT-task-2.4 chore-spec PR 扩 indexer schema 后自动转真实值（retriever 不需改即自动 fill；本 task CLI 不需改）。
- **Provenance 时间字段保真 gap（§2A 决策 E）**：proto.Provenance.imported_at / source_modified_at 是 `google.protobuf.Timestamp`，但 chunker::Provenance 同名字段实际类型是 `String`（RFC3339 with Z；indexer SQLite TEXT 直存）。chrono / time crate 不在 Cargo.toml（R7 严格通道，不引入新 dep）→ v0.1 P0 placeholder `Timestamp::default()`（seconds=0 nanos=0）；CLI text 渲染走 `chunker::Provenance` String 字段保留 RFC3339 原值，--json 输出会显示 1970-01-01 placeholder。下游 task-6.3 export 触发 SPEC-DRIFT 时由主 agent 走 R7 chore-dep PR 引 `time` / `chrono` crate 写 `parse_rfc3339_utc` helper 补齐保真。PRD §Success Metrics 次指标「跨 Agent 迁移保真 ≥ 80% 结构化字段」用 23 字段全集统计，本 gap（2 字段 placeholder）落在 17% 容差内可吸收。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。

## 10. Completion Notes

- **完成日期**：2026-05-23
- **改动文件**：
  - core/src/server.rs（修改）— CoreService 加 data_dir 字段 + new 构造器；search 端到端 wire（错误映射 5 变种）；新增 provenance_to_proto / search_result_to_proto helpers；新增 serve_with_service / context_service_with_data_dir / resolve_data_dir；保留 Default 与 serve 旧入口
  - core/src/main.rs（修改）— 接受第 2 个 cmd-arg + env CONTEXTFORGE_DATA_DIR / 全角 HOME 等价 fallback + 用 CoreService::new 注入 + 调 serve_with_service
  - core/tests/core_skeleton.rs（修改）— test_1_3_3_search_unimplemented 断言从 Unimplemented 更新到 InvalidArgument，注释说明 §2A 决策 A 替换占位的历史
  - core/tests/phase6_smoke.rs（新增）— TEST-6.1.5 / AC5 端到端 tonic transport smoke
  - internal/cli/search.go（新增）— runSearch / parseSearchOpts / optsToProtoRequest / renderText / renderJSON + SearchBackend 类型 + SetSearchBackend 注入点（cli 不 import daemon 避免与 daemon_test → cli 循环）
  - internal/cli/search_test.go（新增）— TEST-6.1.1 ~ 6.1.4 4 个 Go RED→GREEN 测试
  - internal/cli/cli.go（修改）— dispatch case "search" 改 runSearch，取代 task-1.4 default not-implemented 分支
  - internal/cli/cli_test.go（修改）— TestTask14_AC4 not-implemented 循环把 search 移出（共 6 个剩余 placeholder）
  - internal/daemon/search.go（新增）— Daemon.Search 包装，复用 daemon.go lazy clientConn
  - cmd/contextforge/main.go（修改）— 注入 cli.SetSearchBackend(searchViaDaemon)；per-invocation spawn daemon → 等 Health SERVING ≤ 15s → daemon.Search → Stop
  - test/features/cli.feature（修改）— SCEN-6.1.1 ~ 6.1.5 五个占位 Given/When/Then 填实
  - docs/specs/tasks/task-6.1-cli-search.md（修改）— Status Ready → In Progress → Done；§10 Completion Notes 回填
- **commit 列表**：
  - 1c1e154 test(cli-search): 加 SCEN-6.1.1~5 共 5 个 RED 测试 + Status: Ready → In Progress
  - 3d83479 feat(cli-search): contextforge search 端到端实现 — Rust tonic Search wire + Go CLI/daemon
  - （本 commit）docs(spec): 回填 task-6.1 §10 Completion Notes + Status → Done
- **§9 Verification 结果**：
  - install: ✅ skipped（无新依赖；Cargo.toml / go.mod / lockfile 均不动 — R7 严格通道 §2A 决策 E 兑现）
  - typecheck: ✅ cargo check --workspace 干净 / go vet ./... 干净
  - unit-test: Rust 55 passed / 0 failed（31 lib unit + 4 core_skeleton + 5 proto_contract + 11 scanner + 1 phase2_smoke + 1 phase3_importer_smoke + 1 phase4_smoke + 1 phase6_smoke=TEST-6.1.5）；Go 8 packages all pass（cli 6 含 TEST-6.1.1 ~ 6.1.4 含子 case / config / contract / daemon 3 含 TEST-1.4.2/3/5 / importer + 3 sub / memoryops + 2 sub），零回归
- **剩余风险 / 未做项**：
  - §2A 决策 E placeholder Timestamp::default()：proto.Provenance.imported_at / source_modified_at 在 v0.1 P0 一律返默认零值（1970-01-01）；CLI text 渲染目前不渲染 chunker::Provenance 原 RFC3339 字符串字段（proto v0.1 RetrievalResult schema 不暴露 chunker::Provenance 完整结构，只暴露映射后的 PbProvenance）。下游 task-6.3 export 触发 SPEC-DRIFT 时主 agent 走 R7 chore-dep PR 引 time / chrono crate 写 parse_rfc3339_utc helper 补齐保真。详见 §8 Risks 第 3 条。
  - v0.1 schema gap 默认值（context_id / source_type / agent_scope / redaction_status）：本 task CLI 透传 retriever 输出，用户在 text/JSON 里见 v0.1 default 常量；SPEC-DRIFT-task-2.4 chore-spec PR 扩 indexer schema 后自动转真实值（本 task CLI 不需改）。详见 §8 Risks 第 3 条。
- **下游 task 影响**：
  - task-6.2 rest-api（强依赖）：直接复用本 task 已 wire 的 Rust gRPC Search server + Go daemon.Search 方法（HTTP handler 包装 daemon.Search）；REST `/v1/search` 仅需在已落实的 tonic 上加 HTTP wrapper
  - task-6.3 exporter（强依赖 AC5）：直接消费 *contextforgev1.SearchResponse / RetrievalResult 序列化为 JSONL / Markdown bundle / agent draft；ADR-003 单一源 schema 已落地
  - task-7.1 MCP context_search tool（下游）：MCP tool handler 复用同 Rust gRPC Search wire
  - task-8.1 eval-harness（下游）：可调 `contextforge search --json` 跑 recall eval / 可解释字段覆盖率回归
- **§2A Decisions**（2026-05-23 用户审定，主 agent 与用户预先审定后落 spec；worker 完工时按实际实施情况验证 / 补充）：
  - **AC1 Rust tonic Search server wire 位置（选项 A — 本 task wire 端到端真走通）**：替换 `core/src/server.rs` `CoreService::search` 的 `Status::unimplemented` 占位，task-6.1 实施时由 worker 完成 Rust 端 wire + Go 端 CLI/daemon Search 包装。task-4.2 §10「gRPC server 留 task-6.2」改解读为：task-4.2 自己不做 wire；本 task 接力完成。task-6.2 REST API 仅需在已 wire tonic 上加 HTTP wrapper。
  - **CLI 调用模式（选项 A — per-invocation spawn）**：每次 `contextforge search` 自启 daemon (内嵌 core 子进程)、HealthCheck、调 Search、Stop。v0.1 不引入 `contextforge serve` 持久 daemon；持续 / 高频检索 / Phase 7 MCP server 场景留未来 task。冷启动延迟 0.5-2s 在 v0.1 P0 可接受。
  - **AC4 secret redaction（选项 A — 透传 redaction_status）**：CLI 渲染只读 `RetrievalResult.RedactionStatus` 字段值（"applied" / 等）；不在 CLI 二次扫描 content。AC4「结果默认不展示完整 secret」由上游 scanner+indexer+retriever 已 redact content 保证；task-6.3 exporter 会在 export 前再跑一次 secret scan（其 §3 责任）。
  - **AC5 共享 result model（选项 A — 直接用 proto-generated `contextforgev1.RetrievalResult`）**：不新建 Go wrapper struct；不写 ProtoTo/FromProto helper。task-6.3 exporter 直接消费 `*contextforgev1.RetrievalResult` 序列化为 JSONL / Markdown bundle / agent draft。ADR-003「result schema 单一源」原则即 proto。
  - **R7 严格通道**：未引入新 Go module / Rust crate；沿用 task-1.3 / 1.4 / 4.2 已落定依赖（tonic / tokio / prost-types / google.golang.org/grpc / encoding/json stdlib）。Go CLI flags 沿 task-1.4 §2A 决策（stdlib `flag`，不引 cobra）。
  - **Provenance 时间字段映射（选项 X — placeholder Timestamp::default()）**（FIX-2 / 主 agent 与用户 PR #37 review 跟进补判定）：chunker::Provenance.imported_at / source_modified_at 是 `String`（RFC3339 with Z；indexer SQLite TEXT 直存），与 proto.Provenance.imported_at / source_modified_at（`google.protobuf.Timestamp`）类型不匹配。chrono 不在 Cargo.toml，引入需 R7 chore-dep PR，会延后 task-6.1 派工 → 选 placeholder 路径：v0.1 P0 proto.Provenance.imported_at / source_modified_at 一律返 `Timestamp::default()`（seconds=0 nanos=0）；CLI text 渲染直接 print `chunker::Provenance.imported_at` String，用户终端仍能看 RFC3339 原值；--json 输出 placeholder 时间清晰可辨（1970-01-01）。task-6.3 export 触发 SPEC-DRIFT 时主 agent 走 R7 chore-dep PR 修复（§8 Risks 留档）。
  - **RetrieverError 变种 mapping（FIX-1）**（主 agent 与用户 PR #37 review 跟进补判定）：原 spec 草稿引用 `RetrieverError::DataDirMissing` 是写错（实际 `RetrieverError` 5 变种 = `Io / Sqlite / Tantivy / InvalidConfig / CollectionNotFound`，无 DataDirMissing）。正确映射写入 §3 In Scope + §5.3 search() doc comment：`Io`（NotFound）/ `CollectionNotFound` → `FailedPrecondition`；`InvalidConfig` → `InvalidArgument`；`Sqlite` / `Tantivy` / `Io`（非 NotFound）→ `Internal`。
