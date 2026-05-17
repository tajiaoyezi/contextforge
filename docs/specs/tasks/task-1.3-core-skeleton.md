# Task `1.3`: `core-skeleton — contextforge-core Rust 骨架 + gRPC server + health`

> ✅ 已过 `/s2v-implement` §2A 前置审核（2026-05-17）：§3/§4/§5.2/§5.3 `<TBD-by-user>` 已清零、§6 AC（4 条）经用户审定接受、Owner=tajiaoyezi、R7 决策=tokio/serde 直接依赖折入本 task 并透明披露（已在 Cargo.lock 传递依赖图，无新增供应链面）。实时状态以下方 `**Status**` 字段为准；状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: In Progress

**Priority**: P0
**Owner**: tajiaoyezi
**Related Phase**: Phase 1 (foundation)
**Dependencies**: 1.1 (proto)

## 1. Background

数据面二进制 `contextforge-core`（Rust）经 local gRPC 被 Go daemon 拉起与健康检查（PRD §Decisions Log D1 / §Technical Risks R1）。本 task 搭 Rust 侧 tonic gRPC server 骨架 + health，使双进程契约可端到端打通。

## 2. Goal

`contextforge-core` 可独立启动并监听 local gRPC（Unix socket 或 127.0.0.1）；实现 health/SERVING 响应；proto 由 tonic codegen 接入；模块占位（scanner/parser/chunker/indexer/retriever/memoryops）目录就位但不实现。

## 3. Scope

### In Scope

- `core/src/main.rs` + `core/Cargo.toml` `[[bin]] contextforge-core`：二进制可独立 `cargo build` 并启动（AC1）
- tonic gRPC server：监听 local gRPC —— 默认 Unix domain socket，或 `127.0.0.1:<port>`；**显式拒绝默认 `0.0.0.0`**（PRD §Constraints Local service security baseline）
- 实现 task-1.1 冻结的 `ContextService.Health` RPC → 返回 `HealthResponse{ status: "SERVING" }`（AC2）
- `ContextService.Search` 留 `Status::unimplemented` stub（业务属 Phase 2+，本 task 仅骨架，不实现）
- `tokio` 异步运行时（`#[tokio::main]`）+ 最小 `serde` 接入；二者提升为 `core/Cargo.toml` 直接依赖（§2A 用户决策：折入本 task 并透明披露 —— AC1/AC3 强制需要，且 tokio/serde 已在 Cargo.lock 经 tonic 0.12 传递依赖图，无新增供应链面）
- `core/src/{scanner,parser,chunker,indexer,retriever,memoryops}/mod.rs` 空模块占位（编译通过、无逻辑、doc 注释标注 Phase 归属）（AC4）
- 监听地址解析逻辑：默认安全地址 + 拒绝 `0.0.0.0`，可被单测直接调用

### Out Of Scope

- `ContextService.Search` 等业务方法实现（Phase 2+ retriever）
- Go daemon 拉起 core 进程 / Go gRPC client 健康检查编排（task 1.4 端到端）
- 从 task-1.2 `config` 包读监听地址（task 1.4 串联；本 task server 用启动参数 / 内建安全默认，依赖仅 1.1）
- scanner/parser/chunker/indexer/retriever/memoryops 的任何实际逻辑（仅空模块占位，Phase 2+）
- tantivy / tree-sitter / pulldown-cmark / SQLite 等数据面库接入（Phase 2，ADR-008；本 task 不引入）
- 进程崩溃自动重启 / 信号处理 / 长任务硬化（Phase 8 reliability）
- gRPC TLS / 鉴权（v0.1 本地 Unix socket / 127.0.0.1 + 后续 token 由 daemon 层负责）

## 4. Users / Actors

- Go 控制面 `contextforge` daemon（task 1.4 起）：拉起 `contextforge-core` 进程 + 经 local gRPC 调 `ContextService.Health`
- Phase 2+ 数据面实施 agent：在本 task 落位的 `core/src/{scanner,parser,chunker,indexer,retriever,memoryops}` 模块占位上实现扫描/解析/索引/检索
- 本地优先 / 隐私敏感用户（间接受益）：受"禁默认 `0.0.0.0`、默认 Unix socket / 127.0.0.1"本地服务安全基线保护（ADR-004 / PRD Local service security baseline）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Technical Approach 架构风格 / 数据流）
- `docs/specs/phases/phase-1-foundation.md`
- `docs/specs/tasks/task-1.1-proto.md`
- `docs/decisions/adr-001-go-rust-dual-binary-architecture.md`
- `docs/decisions/adr-008-core-library-selection.md`
- `test/features/core.feature`

### 5.2 Imports

- proto 生成产物：`contextforge_core::pb`（task-1.1 冻结契约，`core/build.rs` tonic-build 构建期生成；`pb::context_service_server::{ContextService, ContextServiceServer}` + `HealthRequest` / `HealthResponse` / `SearchRequest` / `SearchResponse`）
- Rust crate（`core/Cargo.toml`）：`tonic`（已有，server transport）、`tokio`（**新增直接依赖** — 异步运行时，features `rt-multi-thread`/`macros`/`net`）、`serde`（**新增直接依赖** — 最小，`derive`；为监听配置 / Phase 2+ 序列化预留）、`prost`/`prost-types`（已有）
- Rust 标准库：`std::net::SocketAddr`、`std::path::PathBuf`、`std::error::Error`（监听地址 / Unix socket 路径）
- 本 task 不 import 任何 Go `internal/*` 或 task-1.2 `config` 包（跨进程，仅 proto 契约耦合）
- ⚠️ R7：`tokio`/`serde` 提升为直接依赖 → §2A 用户决策"折入本 task + 透明披露"（二者已在 Cargo.lock 传递依赖图，无新增供应链面；§10 如实记录）

### 5.3 函数签名

> Rust crate `contextforge-core`：新增 `core/src/main.rs`（bin）+ `core/src/server.rs`（lib 模块）+ 6 个空占位模块；`core/src/lib.rs` 追加 `pub mod`。

```rust
// core/src/server.rs
use crate::pb::context_service_server::{ContextService, ContextServiceServer};
use crate::pb::{HealthRequest, HealthResponse, SearchRequest, SearchResponse};
use tonic::{Request, Response, Status};

#[derive(Debug, Default)]
pub struct CoreService;

#[tonic::async_trait]
impl ContextService for CoreService {
    // AC2：health 返回 SERVING
    async fn health(&self, _req: Request<HealthRequest>)
        -> Result<Response<HealthResponse>, Status>;
    // 本 task Out-of-Scope：Search 业务属 Phase 2+
    async fn search(&self, _req: Request<SearchRequest>)
        -> Result<Response<SearchResponse>, Status>; // Err(Status::unimplemented(..))
}

/// AC1：监听地址；默认安全（Unix socket 或 127.0.0.1），拒绝 0.0.0.0。
#[derive(Debug, Clone, PartialEq)]
pub enum ListenAddr {
    Unix(std::path::PathBuf),
    Tcp(std::net::SocketAddr),
}

/// 解析监听地址：None → 默认 127.0.0.1:50? (内建安全默认)；
/// "unix:/path" → Unix；"127.0.0.1:p"/"[::1]:p" → Tcp；
/// "0.0.0.0[:p]" / 任意全零绑定 → Err（禁默认 0.0.0.0）。
pub fn resolve_listen_addr(arg: Option<&str>) -> Result<ListenAddr, AddrError>;

/// AC1/AC2：在 addr 上起 tonic server 提供 ContextService（可被集成测试调用）。
pub async fn serve(addr: ListenAddr) -> Result<(), Box<dyn std::error::Error>>;

pub fn context_service() -> ContextServiceServer<CoreService>; // AC3：codegen server 装配
```

```rust
// core/src/main.rs  (AC1：[[bin]] contextforge-core)
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>; // resolve_listen_addr → serve

// core/src/{scanner,parser,chunker,indexer,retriever,memoryops}/mod.rs  (AC4)
//! Phase 2+ 占位模块（编译通过、无逻辑）。
```

- SCEN/TEST-1.3.1 → `resolve_listen_addr`：默认非 0.0.0.0；`"0.0.0.0:50051"` → Err；`serve` 能在临时地址绑定（AC1）
- SCEN/TEST-1.3.2 → 起 `serve`，gRPC client 调 `Health` → `HealthResponse.status == "SERVING"`（AC2）
- SCEN/TEST-1.3.3 → `context_service()` 可构造 + `Search` 返回 `Status::unimplemented`（tonic codegen 接入、无 FFI）（AC3）
- SCEN/TEST-1.3.4 → `contextforge_core::{scanner,parser,chunker,indexer,retriever,memoryops}` 路径均可引用（占位模块编译通过）（AC4）

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：
     - 完整写出 AC；每条 `- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`
     - review 改内容不删注释；严禁 `<TBD-by-user> AC<N>` 混合写法
-->

- [ ] **AC1** (PRD §Decisions Log D1): `contextforge-core` 二进制可构建并独立启动，监听 local gRPC（Unix socket 或 127.0.0.1，禁默认 0.0.0.0，PRD §Constraints Local service security baseline）。
- [ ] **AC2** (PRD §Implementation Phases Phase 1 Exit Criteria): gRPC health 返回 SERVING；可被 Go daemon health check（task 1.4 端到端验证）。
- [ ] **AC3** (PRD §Decisions Log D8): tonic + tokio + serde 接入，proto 由 tonic codegen，无 FFI/cgo。
- [ ] **AC4** (本 task 新增): scanner/parser/chunker/indexer/retriever/memoryops 在 `core/src/` 建模块占位（编译通过，不实现逻辑），供 Phase 2+ 落地。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 core 可启动监听 | SCEN-1.3.1 | TEST-1.3.1 | - | unit-test | Not Started |
| AC2 gRPC health SERVING | SCEN-1.3.2 | TEST-1.3.2 | - | unit-test | Not Started |
| AC3 tonic codegen 无 FFI | SCEN-1.3.3 | TEST-1.3.3 | - | unit-test / typecheck | Not Started |
| AC4 模块占位编译通过 | SCEN-1.3.4 | TEST-1.3.4 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R1**（进程生命周期 / core 崩溃恢复）：health 必须可靠，为 task 1.4 daemon 自动重启 + 健康检查提供基础。

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
