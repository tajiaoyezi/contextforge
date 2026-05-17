# Task `1.3`: `core-skeleton — contextforge-core Rust 骨架 + gRPC server + health`

> ✅ 已过 `/s2v-implement` §2A 前置审核（2026-05-17）：§3/§4/§5.2/§5.3 `<TBD-by-user>` 已清零、§6 AC（4 条）经用户审定接受、Owner=tajiaoyezi、R7 决策=tokio/serde 直接依赖折入本 task 并透明披露（已在 Cargo.lock 传递依赖图，无新增供应链面）。实时状态以下方 `**Status**` 字段为准；状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: Done

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

- [x] **AC1** (PRD §Decisions Log D1): `contextforge-core` 二进制可构建并独立启动，监听 local gRPC（Unix socket 或 127.0.0.1，禁默认 0.0.0.0，PRD §Constraints Local service security baseline）。
- [x] **AC2** (PRD §Implementation Phases Phase 1 Exit Criteria): gRPC health 返回 SERVING；可被 Go daemon health check（task 1.4 端到端验证）。
- [x] **AC3** (PRD §Decisions Log D8): tonic + tokio + serde 接入，proto 由 tonic codegen，无 FFI/cgo。
- [x] **AC4** (本 task 新增): scanner/parser/chunker/indexer/retriever/memoryops 在 `core/src/` 建模块占位（编译通过，不实现逻辑），供 Phase 2+ 落地。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 core 可启动监听 | SCEN-1.3.1 | TEST-1.3.1 | - | unit-test | Done |
| AC2 gRPC health SERVING | SCEN-1.3.2 | TEST-1.3.2 | - | unit-test | Done |
| AC3 tonic codegen 无 FFI | SCEN-1.3.3 | TEST-1.3.3 | - | unit-test / typecheck | Done |
| AC4 模块占位编译通过 | SCEN-1.3.4 | TEST-1.3.4 | - | unit-test | Done |

## 8. Risks

- 关联 PRD §Technical Risks **R1**（进程生命周期 / core 崩溃恢复）：health 必须可靠，为 task 1.4 daemon 自动重启 + 健康检查提供基础。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。

## 10. Completion Notes

- **完成日期**：2026-05-17
- **改动文件**：
  - `core/Cargo.toml`（修改 — 新增 `[[bin]] contextforge-core` + `tokio`/`serde` 直接依赖 + dev tokio；§2A R7 决策折入本 task）
  - `Cargo.lock`（修改 — 随直接依赖/feature 更新；tokio/serde 已在传递依赖图，无新增 crate）
  - `core/src/lib.rs`（修改 — 新增 `pub mod server` + 6 个占位模块声明）
  - `core/src/server.rs`（新增 — `CoreService`/`ListenAddr`/`AddrError` + `resolve_listen_addr`/`serve`/`context_service` + `ContextService::{health→SERVING, search→unimplemented}`）
  - `core/src/main.rs`（新增 — `#[tokio::main]` `contextforge-core` 二进制入口）
  - `core/src/{scanner,parser,chunker,indexer,retriever,memoryops}/mod.rs`（新增 — Phase 2+ 占位模块）
  - `core/tests/core_skeleton.rs`（新增 — TEST-1.3.1~1.3.4 集成测试）
  - `docs/specs/tasks/task-1.3-core-skeleton.md`（修改 — §2A 审核填 §3/§4/§5.2/§5.3、§6 勾选、§7→Done、§10 回填、Status）
- **commit 列表**：
  - `8e9cb94` docs(spec): task-1.3 Draft → Ready（§2A 前置审核通过，4 AC accepted）
  - `f1b32c0` docs(spec): task-1.3 进入实施 (Status: Ready → In Progress)
  - `1a955e9` test(core): 加 SCEN-1.3.1~1.3.4 共 4 个 RED 测试（§2.5.1 可编译 unimplemented! 骨架 + tokio/serde 直接依赖[§2A R7]）
  - `6f8185e` feat(core): 实现 contextforge-core tonic gRPC server + Health(SERVING) + 拒 0.0.0.0 + 6 模块占位 通过全部 4 个测试
  - 本回填 docs(spec) commit 见步 11.A（§10 回填 + §7 Done + Status → Done）
- **§9 Verification 结果**：
  - install: ✅ `go mod download && cargo fetch`
  - typecheck: ✅ `go vet ./... && cargo check --workspace`
  - unit-test: 4 passed / 0 failed（本 task core_skeleton TEST-1.3.1~1.3.4；全量 `go test ./...` + `cargo test --workspace` 全绿：proto_contract 5 + Go config/contract 无回归）
- **剩余风险 / 未做项**：
  - `ListenAddr::Unix` 已建模 + `resolve_listen_addr` 可解析 `unix:` 前缀，但 `serve` 的 Unix 传输实现**刻意推迟**到 task-1.4 daemon wiring（完整实现需 tokio-stream/UnixListenerStream，超出 §2A 授权的 tokio+serde）。AC1 为「Unix socket **或** 127.0.0.1」，已由 loopback TCP 路径满足，`0.0.0.0`/`::` 在 `resolve_listen_addr` 显式拒绝。task-1.4 若需 Unix socket 按 R7 评估加 tokio-stream。
  - §2A R7 决策：`tokio`/`serde` 提升为 `core/Cargo.toml` 直接依赖（AC1/AC3 强制，gRPC server 不可回避），二者已在 Cargo.lock 经 tonic 0.12 传递依赖图 → 无新增供应链面；折入本 task 并此处如实披露（非 R7 中途偷加）。
  - 步3 基线 helper 仍误判 greenfield（areas 列表含未创建的 `cmd/contextforge/`）→ 已独立实跑 install/typecheck/unit-test 实证基线真绿（task-1.1/1.2 无回归），未掩盖真红。
  - `Search` RPC 返回 `Status::unimplemented`（Phase 2+ retriever；本 task §3 Out-of-Scope，非缺陷）；gRPC 无 TLS/鉴权（v0.1 本地 127.0.0.1，token/鉴权由 Phase 6 daemon 层负责）。
- **下游 task 影响**：
  - task 1.4 (cli-init) 依赖：daemon 拉起 `contextforge-core` 二进制 + 经 local gRPC 调 `ContextService.Health`（本 task 提供 `serve`/`resolve_listen_addr`/Health=SERVING）；phase-1 §6 端到端 smoke 在 1.4 落地（init + 拉 core + gRPC health SERVING）。
  - Phase 2 (scanner/parser/chunker/indexer) / Phase 4 (retriever) / Phase 5 (memoryops)：在本 task 落位的 `core/src/` 各占位模块（scanner·parser·chunker·indexer·retriever·memoryops）上实现真实逻辑。
  - 无破坏性契约变更（消费 task-1.1 冻结 proto，未改/删 proto 字段）；新增 `tokio`/`serde` 直接依赖（§2A 披露）。
