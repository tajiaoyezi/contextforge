# Task `1.4`: `cli-init — Go CLI + daemon 骨架 + gRPC client + contextforge init 端到端`

> ✅ 已过 `/s2v-implement` §2A 前置审核（2026-05-17）：§3/§4/§5.2/§5.3 `<TBD-by-user>` 已清零、§6 AC（5 条）经用户审定接受、Owner=tajiaoyezi、CLI 框架决策=stdlib `flag` 子命令分发（零新依赖，规避 R7，不与并行 task-3.1 改 go.mod 冲突；cobra 待后续依赖 PR 引入）。实时状态以下方 `**Status**` 字段为准；状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: Done

**Priority**: P0
**Owner**: tajiaoyezi
**Related Phase**: Phase 1 (foundation)
**Dependencies**: 1.1 (proto), 1.2 (config), 1.3 (core-skeleton)

## 1. Background

Phase 1 收口 task：把 proto/config/core-skeleton 串成端到端 `contextforge init`，并打通 Go daemon ↔ Rust core 的 local gRPC（PRD §Implementation Phases Phase 1 / §Technical Risks R1）。这是 Phase 1 的最后一个 task（team §4 Gate 3 phase smoke gate 在此触发）。

## 2. Goal

`contextforge init` 端到端跑通：生成本地配置与数据目录、由 daemon 拉起 `contextforge-core`、Go 经 local gRPC health check Rust core 返回 SERVING；CLI 骨架（cobra）含 init/import/index/search/serve/mcp/eval/export 子命令注册（未实现的返回 not-implemented）。

## 3. Scope

### In Scope

- `cmd/contextforge/main.go`：`contextforge` 二进制入口（执行 CLI root，`os.Exit(cli.Execute(...))`）
- `internal/cli/`：CLI root + 8 子命令注册（init/import/index/search/serve/mcp/eval/export），stdlib `flag` 子命令分发；未实现子命令返回明确 not-implemented 错误（写 stderr + 非 0 退出码，**非 panic**）（AC4）
- `internal/cli/init.go`：`contextforge init` → 编排 task-1.2 `config.Init()` 生成 `~/.contextforge/` 配置 + 目录骨架（collections/ logs/ runtime/，不联网），幂等可重跑（AC1）
- `internal/daemon/`：daemon 骨架 —— `os/exec` 拉起 task-1.3 `contextforge-core` 子进程 + 经 local gRPC `ContextService.Health` 返回 `SERVING`（AC2）；core 异常退出基础版自动重启 + 健康检查（AC3）
- Phase 1 端到端 smoke 落点：可执行测试串 init → 拉 core → gRPC health `SERVING`（AC5）。**仅在本 task 提供可执行落点；phase-1 spec §6 端到端 smoke 命令序列由主 agent 在合并前填实，本 task 不编辑 `docs/specs/phases/phase-1-foundation.md`**

### Out Of Scope

- import/index/search/serve/mcp/eval/export 子命令的业务实现（Phase 2+ / 6 / 7 / 8；本 task 仅注册骨架 + not-implemented 提示）
- Rust core 业务方法（scanner/parser/chunker/indexer/retriever/memoryops）实现（Phase 2+；本 task 仅消费 task-1.3 Health 骨架）
- 生产级进程监督硬化（信号转发 / 优雅停机 / 重启退避策略 / systemd 服务化）—— Phase 8 reliability；AC3 仅基础版自动重启 + 健康检查
- Unix domain socket 传输实现（task-1.3 已显式推迟；本 task daemon 走 task-1.3 的 loopback TCP `127.0.0.1` 路径；Unix socket 需新增 Rust `tokio-stream` 走 R7，不在本 task）
- REST API / MCP server 监听（Phase 6 task-6.2 / Phase 7）；gRPC TLS / 鉴权 / 随机 token 生成（v0.1 本地 127.0.0.1 明文；token 策略 Phase 6 daemon 层）
- 修改 proto / config 契约（仅只读消费 task-1.1/1.2/1.3 冻结契约；若发现确需改 → 立即 STOP 写 `SPEC-DRIFT-task-1.4.md` 交主 agent，不私改）

## 4. Users / Actors

- 多 Agent 重度个人 / 独立开发者：终端跑 `contextforge init` 一键生成本地配置 / 数据目录并由 daemon 拉起数据面（PRD §User Flow 主流程步 1）
- `contextforge` CLI 自身：CLI root 编排各子命令骨架（Phase 6 在此骨架上落 search/export 实现）
- `internal/daemon`（Go 控制面）：作 `contextforge-core`（Rust 数据面）进程父级 + local gRPC client（ADR-001 双二进制 / ADR-003 内部 gRPC）
- Phase 2+ / 6 / 7 / 8 实施 agent：在本 task 注册的 init/import/index/search/serve/mcp/eval/export 子命令骨架上填充业务逻辑
- 本地优先 / 隐私敏感开发者（间接受益）：受 task-1.3 “禁默认 0.0.0.0、本地 127.0.0.1” 安全基线保护（ADR-004 / PRD Local service security baseline）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§User Flow 主流程步 1 / §Technical Approach）
- `docs/specs/phases/phase-1-foundation.md`
- `docs/specs/tasks/task-1.1-proto.md`
- `docs/specs/tasks/task-1.2-config.md`
- `docs/specs/tasks/task-1.3-core-skeleton.md`
- `docs/decisions/adr-001-go-rust-dual-binary-architecture.md`
- `docs/decisions/adr-003-cli-rest-mcp-grpc-interfaces.md`
- `test/features/cli.feature`

### 5.2 Imports

- Go 标准库：`os` / `os/exec`（拉起 `contextforge-core` 子进程）/ `context` / `time` / `fmt` / `errors` / `io` / `flag`（子命令分发）/ `path/filepath` / `sync`（重启监督编排）
- 项目内包：`github.com/tajiaoyezi/contextforge/internal/config`（task-1.2：`config.Init` / `config.Config` / `config.DefaultRootDir`）；`github.com/tajiaoyezi/contextforge/proto/contextforge/v1`（task-1.1 冻结 gRPC：`NewContextServiceClient` / `HealthRequest` / `HealthResponse`）
- gRPC client：`google.golang.org/grpc` + `google.golang.org/grpc/credentials/insecure`（**均已在 go.mod**，task-1.1 引入；本地 127.0.0.1 明文，v0.1 安全基线允许 loopback；本 task **不改 go.mod / go.sum**）
- 跨进程消费 task-1.3 `contextforge-core` 二进制（`cargo build` 产物）及其 `server::resolve_listen_addr` / `serve` / Health=SERVING（仅 proto 契约耦合，无 FFI / cgo）
- CLI 框架决策（§2A）：**stdlib `flag` 子命令分发**，**零新第三方依赖** → 不触发 R7、不与并行 task-3.1 改 go.mod 冲突（PRD §Technical Approach / D8 的 cobra 待后续独立依赖 PR 引入；v0.1 骨架 stdlib 足够）
- 测试侧：`testing` / `os` / `os/exec` / `net` / `context` / `time`（temp `HOME` 隔离 + 端口探测 + 子进程生命周期断言；`TestMain` 内 `cargo build -p contextforge-core` 一次构建被测二进制）

### 5.3 函数签名

> Go 包 `cli` 落 `internal/cli/`、`daemon` 落 `internal/daemon/`、二进制入口 `cmd/contextforge/main.go`（adapter §Source areas `cmd/contextforge/` + `internal/`）。仅消费 task-1.1/1.2/1.3 冻结契约，不新增 §6 AC 未覆盖的方法 / 字段。

```go
// internal/cli  (AC4)  — stdlib flag 子命令分发
package cli

// Execute 解析 args 分发子命令；未知/未实现子命令 → 写 stderr "<name>: not implemented"
// 并返回非 0 退出码（绝不 panic）。已实现：init。返回进程退出码。
func Execute(args []string, stdout, stderr io.Writer) int

// SubcommandNames 返回注册的 8 个子命令名（AC4 可断言注册齐全，稳定顺序）。
func SubcommandNames() []string // {"init","import","index","search","serve","mcp","eval","export"}

// runInit 编排 config.Init 生成默认配置 + 目录骨架；root=="" → config.DefaultRootDir()；
// 已存在则不覆盖（config.Init 幂等语义）→ 可重跑（AC1）。
func runInit(root string, stdout io.Writer) error

// cmd/contextforge/main.go  (AC4)
func main() // os.Exit(cli.Execute(os.Args[1:], os.Stdout, os.Stderr))
```

```go
// internal/daemon  (AC2/AC3)
package daemon

type Options struct {
    CoreBinPath string // contextforge-core 二进制路径（默认 exec.LookPath("contextforge-core") 退化到约定 target 路径）
    ListenAddr  string // 传给 core 的安全监听地址；默认 "127.0.0.1:<port>"，禁 0.0.0.0（对齐 task-1.3 resolve_listen_addr）
    AutoRestart bool   // AC3：core 异常退出基础版自动重启
}

type Daemon struct { /* unexported: cmd / opts / mu / restarts / cancel ... */ }

// Start 拉起 contextforge-core 子进程（AC2）；AutoRestart 时启动重启监督 goroutine（AC3）。
func Start(ctx context.Context, opts Options) (*Daemon, error)

// HealthCheck 经 local gRPC ContextService.Health 探测，返回 status（期望 "SERVING"）（AC2）。
func (d *Daemon) HealthCheck(ctx context.Context) (string, error)

// Restarts 返回累计自动重启次数（AC3：测试断言 core 被杀后 >=1 且 health 恢复 SERVING）。
func (d *Daemon) Restarts() int

// Stop 终止 core 子进程并停止重启监督（幂等）。
func (d *Daemon) Stop() error
```

- SCEN/TEST-1.4.1 → `runInit` 在临时 `HOME` 生成 `config.toml` + `collections/`·`logs/`·`runtime/`（文件 0600 / 目录 0700），二次调用幂等不报错（AC1）
- SCEN/TEST-1.4.2 → `daemon.Start` 拉起 core，`HealthCheck` 返回 `"SERVING"`（AC2）
- SCEN/TEST-1.4.3 → core 子进程被杀后 `AutoRestart` 使 `Restarts() >= 1` 且 `HealthCheck` 恢复 `"SERVING"`（AC3）
- SCEN/TEST-1.4.4 → `SubcommandNames()` 含全部 8 子命令；未实现子命令经 `Execute` 返回非 0 + stderr 含 `not implemented`，不 panic（AC4）
- SCEN/TEST-1.4.5 → 端到端：`runInit` → `daemon.Start` → `HealthCheck()=="SERVING"` 串通（AC5，为 phase-1 §6 提供可执行落点）

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [x] **AC1** (PRD §User Flow 主流程步 1): `contextforge init` 生成 `~/.contextforge/` 配置与数据目录（不联网），幂等可重跑。
- [x] **AC2** (PRD §Implementation Phases Phase 1 Exit Criteria): daemon 能启动 `contextforge-core` 并经 local gRPC health check 返回 SERVING。
- [x] **AC3** (PRD §Technical Risks R1): core 异常退出时 daemon 能自动重启 + 健康检查（基础版）。
- [x] **AC4** (PRD §Technical Approach / §Decisions Log D3): CLI（v0.1 用 stdlib `flag` 子命令分发；§2A 决策 cobra 待后续独立依赖 PR 引入）注册 init/import/index/search/serve/mcp/eval/export 子命令；未实现子命令返回明确 not-implemented 提示（非 panic）。
- [x] **AC5** (本 task 新增): Phase 1 端到端 smoke 可执行（init → core 拉起 → gRPC health SERVING），为 phase spec §6 端到端 smoke 提供落点。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 init 生成配置/目录 | SCEN-1.4.1 | TEST-1.4.1 | - | unit-test | Done |
| AC2 daemon 拉起 core+health | SCEN-1.4.2 | TEST-1.4.2 | - | unit-test | Done |
| AC3 core 崩溃自动重启 | SCEN-1.4.3 | TEST-1.4.3 | - | unit-test | Done |
| AC4 CLI 子命令注册 | SCEN-1.4.4 | TEST-1.4.4 | - | unit-test | Done |
| AC5 Phase1 端到端 smoke | SCEN-1.4.5 | TEST-1.4.5 | - | unit-test | Done |

## 8. Risks

- 关联 PRD §Technical Risks **R1**（Go↔Rust gRPC 边界 / 进程生命周期）：本 task 端到端验证 R1 缓解；daemon 自动重启 + 健康检查在此落地。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。⚠️ 本 task 为 Phase 1 最后 task：完工/合并前 phase spec §6 端到端 smoke 必须填实（`s2v_preflight_phase` C1 / team §4 Gate 3）。

## 10. Completion Notes

- **完成日期**：2026-05-17
- **改动文件**：
  - `cmd/contextforge/main.go`（新增 — `contextforge` 二进制入口，委托 `internal/cli`）
  - `internal/cli/cli.go`（新增 — stdlib `flag` 子命令分发 + `Execute`/`SubcommandNames`/`runInit`，编排 task-1.2 `config.Init`）
  - `internal/cli/cli_test.go`（新增 — TEST-1.4.1 init 幂等 / TEST-1.4.4 子命令注册+未实现非 panic）
  - `internal/daemon/daemon.go`（新增 — `Options`/`Daemon` + `Start`/`HealthCheck`/`Restarts`/`Stop` + 重启监督 + `ensureLoopback` 禁 0.0.0.0）
  - `internal/daemon/daemon_test.go`（新增 — `TestMain` cargo build core + TEST-1.4.2/1.4.3/1.4.5）
  - `docs/specs/tasks/task-1.4-cli-init.md`（修改 — §2A 审核填 §3/§4/§5.2/§5.3 + AC4 措辞、§6 勾选、§7→Done、§10 回填、Status）
- **commit 列表**：
  - `f3b56f7` docs(spec): task-1.4 Draft → Ready（§2A 前置审核通过，5 AC accepted）
  - `2bea1a8` docs(spec): task-1.4 进入实施 (Status: Ready → In Progress)
  - `a063b23` test(cli): 加 SCEN-1.4.1~1.4.5 共 5 个 RED 测试（§2.5.1 可编译 panic 骨架）
  - `1be6514` feat(cli): 实现 stdlib flag CLI + contextforge init 编排 + daemon 拉起 core/gRPC Health/基础自动重启 通过全部 5 个测试
  - 本回填 docs(spec) commit 见步 11.A（§10 回填 + §6 勾选 + §7 Done + Status → Done）
- **§9 Verification 结果**：
  - install: ✅ `go mod download && cargo fetch`
  - typecheck: ✅ `go vet ./... && cargo check --workspace`
  - unit-test: 5 passed / 0 failed（本 task TEST-1.4.1~1.4.5：internal/cli 2 + internal/daemon 3；全量 `go test ./...` + `cargo test --workspace` 全绿，task-1.1/1.2/1.3 无回归：Rust core_skeleton 4 + proto_contract 5、Go config/contract 全过）
- **剩余风险 / 未做项**：
  - AC3 自动重启为「基础版」（固定 200ms backoff + launch 失败即停止监督）；生产级监督（信号转发 / 优雅停机 / 指数退避+jitter / systemd 服务化）属 Phase 8 reliability，本 task §3 Out-of-Scope，非缺陷。
  - daemon 走 task-1.3 loopback TCP 127.0.0.1 路径；Unix domain socket 传输 task-1.3 已显式推迟（需新增 Rust tokio-stream，触发 R7），本 task §3 Out-of-Scope。
  - 测试 `freeAddr` 预留端口后关闭再交 core 绑定，存在极小 TOCTOU 窗口；仅本地测试支持代码、非生产路径，可接受。
  - §2A 决策：CLI 用 stdlib `flag`（零新第三方依赖，规避 R7，不与并行 task-3.1 改 go.mod 冲突）；cobra（PRD §Technical Approach / D8）待后续独立依赖 PR 引入 —— 经用户 §2A 审定，非缺陷。
  - import/index/search/serve/mcp/eval/export 子命令返回明确 not-implemented（Phase 2+ / 6 / 7 / 8；本 task §3 Out-of-Scope，非缺陷）。
  - 步 3 基线 helper 因 Unit test areas 含当时未创建的 `cmd/contextforge/` 未按冷启动跳过而**实跑** install/typecheck/unit-test 实证基线真绿（非 skip，未掩盖真红）；本 task 创建 `cmd/contextforge/` 后该误判消失，§9 三项均实跑全绿。
  - 本 task 为 Phase 1 最后 task：phase-1 spec §6 端到端 smoke 仍为占位，属 phase 业务契约（主 agent + 用户域），本 task 不编辑 phase spec；AC5 已提供可执行落点（TEST-1.4.5：init → core 拉起 → gRPC health SERVING）。§6 smoke 填实 + team §4 Gate 3 phase smoke gate 由主 agent 合并前处理。
- **下游 task 影响**：
  - Phase 1 收口：本 task 串通 `contextforge init` + daemon 拉起 `contextforge-core` + gRPC Health SERVING，端到端验证 PRD §Technical Risks R1 缓解。
  - Phase 6 task-6.1 / 6.2：在 `internal/cli` 子命令骨架上落 search / export 实现 + REST；daemon 骨架供 serve 子命令复用。
  - Phase 2+ 实施 agent：cli 子命令骨架（import / index 等）待后续 phase 填充业务。
  - phase-1 spec §6 端到端 smoke 填实 + team §4 Gate 3 由主 agent 合并前处理（本 task 已提供 AC5 可执行落点 TEST-1.4.5）。
  - 无破坏性契约变更：仅只读消费 task-1.1/1.2/1.3 冻结契约（proto / config）；未改 go.mod / go.sum / Cargo.toml / proto（R7 不触发）。
