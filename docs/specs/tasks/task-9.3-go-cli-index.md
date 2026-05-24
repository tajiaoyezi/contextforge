# Task `9.3`: `go-cli-index — internal/cli/index.go 改写调真实 gRPC + 进度条 + 保留 resume manifest`

> Status=Ready；主 agent §2A 自审通过（ADR-012 + goal §自决规则 6）。本 task 依赖 task-9.2 提供的 Rust gRPC Index handler。可与 task-9.4 并行（不同文件）。

**Status**: Ready

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 9 (cli-pipeline)
**Dependencies**: 9.2 (rust-grpc-index)

## 1. Background

v0.1 实测：`internal/cli/index.go` 接收 `--source` / `--data-dir` / `--collection` / `--resume` 参数后，**仅写一份 `runtime/index-<collection>.resume.json` 占位 manifest**（含 source_path / data_dir / collection / total_items=0 / processed_items=0 / completed=false），输出 `processed=0 total=0` 后退出。**完全不调** Rust core scanner/indexer。`contextforge index --source ./project` 跑完后 SQLite chunks 表仍为空、Tantivy 索引为空。详见 [ADR-013](../../decisions/adr-013-cli-data-plane-grpc-bridge.md) §Context #1。

task-9.2 完成后 Rust 侧 `CoreService::index` 已 wire；本 task 把 Go CLI 接到该 gRPC stream，让 `contextforge index --source ./project --data-dir <root> --collection X` 真实索引。

## 2. Goal

`internal/cli/index.go` 改写：解析 args → 调 `internal/daemon.Start(...)` 起 core 子进程 → 调 `daemon.Index(ctx, &IndexRequest{...})` 消费 stream → 终端进度上报（人类可读 `\r` 行覆盖 + `--json` mode 结构化 JSONL stream） → 收 final IndexProgress 后退出（exit 0 if error=="" else 1） → `defer daemon.Stop()`；`--resume` flag 保留 task-8.2 reliability manifest 行为但叠加从 gRPC stream 收的 file-level progress 更新 manifest；新增 `internal/daemon/index.go::Daemon.Index` client wrapper 返回 `<-chan *IndexProgress` + error chan；Go unit test (fake gRPC server) + Go integration test (`cargo build -p contextforge-core` + 真扫描 sample fixture)。

## 3. Scope

### In Scope

- **新增 `internal/daemon/index.go`**（类似 `internal/daemon/search.go` pattern）：
  ```go
  // Index streams IndexProgress from contextforge-core's ContextService.Index.
  // Caller consumes progress chan until it closes (final progress has done=true).
  // The error chan emits at most one error (gRPC transport / Status) or nil on
  // clean completion; both chans close together.
  func (d *Daemon) Index(
      ctx context.Context,
      req *contextforgev1.IndexRequest,
  ) (<-chan *contextforgev1.IndexProgress, <-chan error)
  ```
  - 内部：`clientConn() → NewContextServiceClient(conn).Index(ctx, req)` → spawn goroutine `for { Recv() }` → 收到消息 push 到 progress chan → 收 EOF / err push 到 err chan + 关两个 chan
  - ctx cancel → goroutine 退出（gRPC stream 自动 cancel server-side）
- **重写 `internal/cli/index.go`**：
  - 保留现有 flag parsing（`--source` / `--data-dir` / `--collection` / `--resume` / `--changed-items`）+ 新增 `--json` flag（输出 JSONL stream 而非进度条）
  - 删除当前"只写 manifest 存根 + 返回 0"的逻辑
  - 新流程：
    1. 解析 flags（同现有），校验 `--source` 非空且路径存在
    2. 算出 `manifestPath = <data_dir>/runtime/index-<collection>.resume.json`
    3. `reliability.StartOrResumeManifest(manifestPath, ...)` — 如 resumed=true 且 `--resume` 启用 → CLI 输出 `resuming long-task mode`；否则输出 `safe rebuild mode` / `long-task mode`（保留现有人类可读输出兼容）
    4. `ctx, cancel := context.WithTimeout(context.Background(), 30*time.Minute)` — 长任务超时
    5. `d, err := daemon.Start(ctx, daemon.Options{CoreBinPath: "", ListenAddr: "", AutoRestart: false})` — 一次性 index 不需要 AutoRestart
    6. 等 `daemon.HealthCheck` SERVING（轮询 ≤ 15s）
    7. `progressCh, errCh := d.Index(ctx, &contextforgev1.IndexRequest{SourcePath: opts.Source, DataDir: opts.DataDir, CollectionId: opts.Collection})`
    8. for-select 循环 consume progressCh：
       - human mode：`fmt.Fprintf(stdout, "\rindexing %s (files=%d, chunks=%d)", p.CurrentFile, p.FilesProcessed, p.ChunksWritten)`；终末 `\n` + summary
       - json mode：`json.Marshal(p)` per line 写 stdout
       - 每 N 条（N=10）更新 manifest 文件 ProcessedItems = FilesProcessed
    9. 收 errCh 终态 → if err != nil → stderr 报错 + exit 1
    10. final progress.error != "" → stderr 报错 + exit 1
    11. final progress.done=true && error=="" → manifest 标 completed=true → exit 0
    12. `defer d.Stop()`
- **新增 `internal/daemon/index_test.go`**：
  - 用 fake gRPC server (`testserver` package or in-process) 测 `Daemon.Index` 包装：spawn 多条 progress message → consumer 正确收到顺序 + chan close 行为 + ctx cancel mid-stream 不 deadlock
- **新增 `internal/cli/index_test.go` 扩充**（现有文件已有）：
  - 现有测试覆盖 manifest stub 行为 → 改造 / 删除不再 applicable 的断言
  - 新增 fake-daemon mock：替 `daemon.Start` + `Daemon.Index` 为 stub → 验 CLI 收 progress chan + 输出 + exit code 正确
  - 集成测试（独立 `TestMain` `cargo build -p contextforge-core` once）：建临时 source dir（3 .md）+ 临时 data_dir + 真跑 CLI binary → assert exit 0 + stdout 含 `files=3` + SQLite chunks > 0
- 文件锚点：`internal/daemon/index.go`（新增）+ `internal/daemon/index_test.go`（新增）+ `internal/cli/index.go`（重写）+ `internal/cli/index_test.go`（扩充）

### Out Of Scope

- **`contextforge import` 实现**（task-9.4 并行）
- **REST `/v1/index` HTTP wrapper**（v0.2 不实施；如未来需要走 future task-9.X / task-10.X）
- **task-8.2 reliability manifest 跨进程 server-side resume**（v0.2 仍只 client-side manifest 更新；server-side resume 留 future）
- **修改 `proto/` / `internal/daemon/daemon.go` / `internal/daemon/search.go`**（本 task 不动现有 daemon 模块）
- **修改 `Cargo.toml` / `go.mod` / `Cargo.lock` / `go.sum`**（R7）
- **Tantivy commit 频率优化**（task-9.2 Out Of Scope 同款）

## 4. Users / Actors

- **README quick start 用户**（PRD §User Flow 主流程步 2 - 索引）：本 task 后 `contextforge index --source <path>` 真实工作
- **task-9.4 go-cli-import 实施 agent**（间接消费）：task-9.4 产 canonical JSONL 到 `data_dir/imports/`，用户跑 `contextforge index --source <jsonl>` 灌入；本 task 必须支持 `--source` 指向单文件（JSONL）— **本 task §6 AC4 含此要求**
- **task-9.5 release-smoke-real 实施 agent**（下游）：复用本 task CLI 路径作为 release smoke 的"index"步
- **task-9.6 readme-quickstart-verified 实施 agent**（下游）：在 `scripts/quickstart_smoke.sh` 内调本 task CLI

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§User Flow 主流程步 2 + 异常流"索引中断" / §Decisions Log D3）
- `docs/specs/phases/phase-9-cli-pipeline.md`
- `docs/specs/tasks/task-9.1-proto-index-rpc.md`
- `docs/specs/tasks/task-9.2-rust-grpc-index.md`
- `docs/specs/tasks/task-6.1-cli-search.md`（CLI per-invocation spawn 模式参考）
- `docs/specs/tasks/task-8.2-reliability.md`（resume manifest schema）
- `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`
- `internal/cli/index.go`（当前 stub 实现）
- `internal/cli/search.go`（CLI gRPC client pattern 参考）
- `internal/daemon/search.go`（daemon client wrapper pattern 参考）
- `internal/daemon/daemon.go`（Start / HealthCheck / Stop API）
- `internal/reliability/reliability.go`（manifest schema）
- `test/features/cli-pipeline.feature`

### 5.2 Imports

- **stdlib**：`context` / `encoding/json` / `flag` / `fmt` / `io` / `os` / `path/filepath` / `time`
- **proto**：`contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"`（task-9.1 codegen 产）
- **内部**：`internal/config`（DefaultRootDir）+ `internal/daemon`（Start / Stop / HealthCheck / Index）+ `internal/reliability`（StartOrResumeManifest）
- **测试侧**：`testing` / `os` / `os/exec`（集成测试 spawn CLI binary）/ `path/filepath` / `time` / `google.golang.org/grpc/test/bufconn`（fake gRPC server 选项）
- **不引入**：R7 严格；`google.golang.org/grpc` 已在 go.mod（task-1.1）；`bufconn` 是 grpc-go 自带（test/bufconn package）不算新 dep

### 5.3 函数签名

```go
// internal/daemon/index.go ----
package daemon

// Index streams IndexProgress from contextforge-core's ContextService.Index.
// Caller consumes progressCh until it closes (final progress message has done=true,
// possibly with error != ""). errCh emits at most one error (gRPC transport or
// Status) and then closes; on clean completion errCh emits nil then closes.
func (d *Daemon) Index(
    ctx context.Context,
    req *contextforgev1.IndexRequest,
) (progressCh <-chan *contextforgev1.IndexProgress, errCh <-chan error)

// internal/cli/index.go ----  (rewrite)
package cli

// runIndex (rewrite). Behaviour:
//   - parses --source (required) / --data-dir (default config.DefaultRootDir) /
//     --collection (default "default") / --resume (bool) / --changed-items (int64) /
//     --json (bool, new)
//   - opens / resumes manifest via reliability.StartOrResumeManifest
//   - spawns daemon (per-invocation; no AutoRestart) + HealthCheck
//   - calls daemon.Index(ctx, &IndexRequest{...}) and consumes the progress stream
//   - human-readable progress: \r-overwrite line per IndexProgress in stdout
//   - --json: marshals each IndexProgress to a JSONL line in stdout
//   - persists ProcessedItems back to manifest every 10 progress messages + at end
//   - returns process exit code (0 on success / 1 on error / 2 on bad args)
func runIndex(args []string, stdout, stderr io.Writer) int
```

- SCEN/TEST-9.3.1 → `Daemon.Index` 收 ≥3 条 progress + 最终 done=true → progressCh close + errCh emit nil + close（AC1）
- SCEN/TEST-9.3.2 → `runIndex` human mode 输出 `\r`-overwrite 行 + final summary 行 + exit 0（AC2）
- SCEN/TEST-9.3.3 → `runIndex --json` 模式 stdout 每行一个合法 JSON 含期望字段（AC3）
- SCEN/TEST-9.3.4 → `runIndex` 集成测试：真 cargo build core + 临时 source 3 .md fixture + 真扫描 → exit 0 + SQLite chunks > 0（AC4）
- SCEN/TEST-9.3.5 → `runIndex --resume` 第二次跑：resumed=true 路径走通 + manifest ProcessedItems 从 0 → N → completed=true（AC5）

## 6. Acceptance Criteria

- [ ] **AC1** (本 task 新增 / ADR-013 §Decision #3): `internal/daemon/index.go::Daemon.Index` 实现 §5.3 签名；按 task-9.2 stream 协议 consume → push 到 progressCh；ctx cancel 时 goroutine 退出不 deadlock；progressCh + errCh 同时关闭
- [ ] **AC2** (PRD §User Flow 主流程步 2): `contextforge index --source <path> --data-dir <root> --collection X` 真实索引：CLI exit 0 + stdout 含进度行（`indexing <file> (files=N, chunks=M)`）+ final summary 行
- [ ] **AC3** (PRD §Decisions Log D3 协议接口 / 本 task 新增): `--json` flag 输出 JSONL stream（每 IndexProgress 一行 JSON 含 files_processed / chunks_written / current_file / done / error），用于程序化消费
- [ ] **AC4** (PRD §Implementation Phases Phase 2 Exit Criteria 补): 集成测试 `TestCliIndex_E2E_RealCore` — `t.TempDir()` 中 cargo build core + 临时 source（3 .md + 1 .env denied + 1 secret-redacted .yaml）+ 真跑 `runIndex` CLI → exit 0 + SQLite chunks > 0 + Tantivy 搜索 fixture marker 命中 + .env skipped + .yaml redacted
- [ ] **AC5** (PRD §User Flow 异常流"索引中断"): `--resume` 行为 — 首次 indexer 完成后 manifest `completed=true`；第二次跑 `--resume` 走 resumed 路径输出 `resuming long-task mode`；ProcessedItems 字段在 indexer 过程中按每 10 messages 更新一次（不必精确，allow 后续不超 N+10 偏差）

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 Daemon.Index 包装 | SCEN-9.3.1 | TEST-9.3.1 (internal/daemon/index_test.go fake gRPC server) | - | unit-test | - |
| AC2 CLI 真实索引 + 人类输出 | SCEN-9.3.2 | TEST-9.3.2 (internal/cli/index_test.go fake daemon) | - | unit-test | - |
| AC3 CLI --json mode | SCEN-9.3.3 | TEST-9.3.3 | - | unit-test | - |
| AC4 集成端到端 | SCEN-9.3.4 | - | TEST-9.3.4 (internal/cli/index_test.go E2E with cargo build) | unit-test (TestMain cargo build) | - |
| AC5 --resume 行为 | SCEN-9.3.5 | TEST-9.3.5 | - | unit-test | - |

## 8. Risks

- 关联 PRD §Technical Risks **R1**（Go↔Rust gRPC 边界）：本 task 是 CLI 真实端到端，引入 gRPC stream 在 Go client 侧；ctx cancel / EOF / Status err 三种终态行为需 fake gRPC server 覆盖测试。
- 关联 **R6**（大仓库性能）：本 task 集成测试 fixture 限 ≤5 文件（保 CI 快）；100k chunk 性能 gate 留 task-9.5。
- 关联 **R9**（本地 daemon 暴露面）：本 task 沿用 task-6.1 per-invocation spawn 模式（默认 loopback + 自动选端口 + AutoRestart=false），不引入新暴露面。
- 风险次：集成测试 `cargo build -p contextforge-core` 在 cold cache 慢（首次 30-60s）；缓解：`TestMain` 一次 build 全包共享（task-1.4 daemon_test.go pattern）；CI runner 上预 build core binary 缓存。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit。本 task 集成测试在 `internal/cli/index_test.go` 内 `TestMain` 触发 cargo build；CI 上需先 cargo build core，否则首次 test 慢。

## 10. Completion Notes

> 待 task 完成后回填。
