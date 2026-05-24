# Phase 9 · cli-pipeline

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 是 v0.2.0 minor release 收口 phase — 补齐 v0.1 spec drift（CLI 数据通路 / import 实施 / release smoke 真端到端）。本 phase 最后一个 task 完工/合并前必须执行 §6 端到端 smoke（`s2v_preflight_phase` C1）。
>
> 治理基线：本 phase 按 ADR-011 单驱动 + ADR-012 主 agent 自治；§2A Ready review 由主 agent 自审（带用户复核选项 — 本 phase 涉及 PRD §Implementation Phases 修改，建议保留用户审）。详见 [ADR-013](../../decisions/adr-013-cli-data-plane-grpc-bridge.md)。

## 1. 阶段目标

补齐 v0.1 spec drift：`contextforge init → import hermes|openclaw|agent-rules → index → search → eval run` CLI 端到端真实可跑；proto add-only 扩展 `rpc Index(IndexRequest) returns (stream IndexProgress)`；Rust `CoreService::index` wire 到 `IndexSession::index_path`；task-8.3 release smoke 用真集成测试取代假证据测试；README quick start 基于 `examples/quickstart/` fixture 可复现。来源：[ADR-013](../../decisions/adr-013-cli-data-plane-grpc-bridge.md) / PRD §User Flow 主流程步 2-3 / PRD §Implementation Phases v0.2 新增（见 PRD §Implementation Phases Phase 9 行）。

## 2. 业务价值

实现 PRD §User Flow 主流程字面承诺 — `contextforge import` / `contextforge index` 真实可跑；直接支撑成功指标「上下文重建时间 ≤ 3-5 分钟」（v0.1 因 CLI 不通无法测量）。修复 ADR-013 §Context 列出的击鼓传花 spec drift（Phase 2 / 3 / 6 / 8 互相推诿 CLI 数据通路实施责任）。v0.2.0 release 后 README quick start 对外部用户首次诚实可用。

## 3. 涉及模块

- `proto/contextforge/v1/service.proto`（add-only：`rpc Index` + `IndexRequest` / `IndexProgress` messages）
- `core/src/server.rs`（Rust gRPC `CoreService::index` 流式 handler）
- `core/src/indexer/mod.rs`（如需 scan_path_with_progress hook，保持 `index_path` 兼容）
- `internal/daemon/index.go`（Go gRPC client wrapper `Daemon.Index`）
- `internal/cli/index.go`（重写：调真实 gRPC 替代 manifest 存根）
- `internal/cli/import.go`（新增：三子命令 hermes/openclaw/agent-rules，调 `internal/importer/<src>`）
- `internal/release/release_test.go`（删 fake-evidence 测试，加真集成）
- `scripts/release_smoke.sh`（增 CLI 端到端段）
- `scripts/quickstart_smoke.sh`（新增 CI 可跑的 README quick start 校验）
- `examples/quickstart/`（新增 fixture：sample 项目 + Hermes MEMORY.md / USER.md）
- `README.md` / `RELEASE_NOTES.md` / `docs/releases/v0.2.0-*.md`（v0.2.0 发布文档）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 9.1 | proto | `../tasks/task-9.1-proto-index-rpc.md` |
| 9.2 | core/server | `../tasks/task-9.2-rust-grpc-index.md` |
| 9.3 | cli/index | `../tasks/task-9.3-go-cli-index.md` |
| 9.4 | cli/import | `../tasks/task-9.4-go-cli-import.md` |
| 9.5 | release | `../tasks/task-9.5-release-smoke-real.md` |
| 9.6 | release/readme | `../tasks/task-9.6-readme-quickstart-verified.md` |

## 5. 依赖关系

- **依赖**：Phase 8（eval-and-reliability）— 复用 task-8.2 reliability manifest schema；Phase 6（cli-api-export）— 复用 task-6.1 daemon spawn + gRPC client 模式。
- **可并行**：否（v0.2 收口 phase）。Phase 内顺序：9.1 → 9.2 → {9.3 ∥ 9.4} → 9.5 → 9.6。
- **Phase 内并行机会**：task-9.3 (go-cli-index) ∥ task-9.4 (go-cli-import) 在 task-9.2 完成后可并行 — 9.3 改 `internal/cli/index.go`，9.4 新增 `internal/cli/import.go`，无源文件写冲突。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（任务 9.1-9.6 全 Done，实测验证）**：

- [x] `proto/contextforge/v1/service.proto` 含 `rpc Index(IndexRequest) returns (stream IndexProgress)`；schema_version 仍 `0.1`；codegen 产物（Go `proto/contextforge/v1/*.pb.go` + Rust `core/src/proto/contextforge.v1.rs` 等）已 regen 并 commit（task-9.1 PR #59 落地）
- [x] `contextforge index --source <path> --collection <id> --data-dir <root>` 真实索引（CLI exit 0 + stdout `\rindexing ...` 进度 + final summary `files=N chunks=M` + SQLite chunks > 0 + Tantivy 可检索）— task-9.3 PR #61 落地，e2e 测试 `TestCliIndex_E2E_RealCore` 真 cargo build 验证
- [x] `contextforge import hermes <path> --collection X --data-dir Y` 真实写 `<data_dir>/imports/hermes/<ctx_id>.md`（D1 两步式：实施细化为写单独 .md 文件而非 jsonl，详 ADR-013 §Decision #5）；openclaw / agent-rules 子命令对称 — task-9.4 PR #62 落地
- [x] `contextforge search` 在已索引 collection 上真实返回 ≥1 结果（不再 `collection not found`）— task-9.6 quickstart_smoke.sh 步骤 6 实测命中 `configuration` 关键字
- [x] `contextforge eval run --collection <id>` 真实跑 30 golden questions 输出 — task-9.6 quickstart_smoke.sh 步骤 7 实测跑完
- [x] `internal/release/release_test.go` fake-evidence pattern 双 gate 命中 0：`grep 'StepPassed, Evidence: "ok"'` + `grep 'Status: StepPassed, Evidence:'` 均 0；新真集成 `TestPhase9ReleaseSmoke_EndToEnd` 真 7-step CLI PASS（force-Windows 36.66s） — task-9.5 PR #63 落地
- [x] `scripts/quickstart_smoke.sh` 真跑 7 步 PASS + `QUICKSTART_SMOKE_EXIT=0`；README Quick Start 段重写（one-shot smoke + manual 7 step + Expected output + v0.2 limitations）— task-9.6 PR #64 落地

**端到端 smoke**：

```bash
bash scripts/quickstart_smoke.sh
```

该脚本是 task-9.6 的 Gate 3 入口：在 `t.TempDir()` 等价的临时目录中跑全套 CLI 命令序列（go build → cargo build → init → import hermes fixture → index → search → eval），验证退出码与 stdout 关键字。Linux / WSL2 runner 可在同一脚本后续扩展为真实 tarball 解包执行；v0.2 gate 至少要求所有 CLI binary 真实启动 + 真实索引 ≥10 个文件 + search 真实返回 ≥1 结果。

**Scope 注**：本 phase smoke 与 task-8.3 release_smoke.sh 互补 — task-8.3 (v0.1) gate tarball 文件结构 + Rust gRPC search smoke + Go unit harness；task-9.6 (v0.2) 新增"真实 CLI binary 端到端"段。两条 smoke 均跑通才允许 v0.2.0 tag。

## 7. 阶段级风险

- **关联 ADR-013 §Rollback Or Migration Plan 三条风险**：
  - proto add-only 撞 schema_version freeze 红线 → 改方案重新 ADR（概率低 — schema_version 表面 `0.1` 仅约束字段 tag 不删不改，新增 service method 不破坏 wire 兼容性）。
  - 两步式 import UX 不可接受 → 切换为方案 D1 选项 B（feed-records gRPC）需新 task 扩 proto stream-records。
  - task-9.5 真集成测试在 cross-platform runner flake → 回退到 unit-level + 单 init smoke。
- **关联 PRD §Technical Risks R1**（Go↔Rust gRPC 边界）：新增 RPC 增加契约面，task-9.1 / 9.2 严格走 R1 缓解（proto add-only + version 化 + core 异常退出 daemon 自动重启已在 task-1.4 落地）。
- **关联 PRD §Technical Risks R6**（大仓库性能）：task-9.3 真实索引 sample fixture 引入性能基线 — 若 examples/quickstart/ fixture 太大导致 quickstart_smoke.sh 跑 > 30s，CI 会被拖慢；缓解：fixture 限定 ≤100 文件 + 总大小 ≤1MB，性能 gate 留 task-9.5 release smoke 单独的 100k chunk benchmark（沿用 task-8.3 BenchmarkReport gate）。
- **关联 ADR-013 §Follow-ups O12**：Phase 1-8 spec drift 形成机制 — 击鼓传花链怎么在治理层提前发现；本 phase 实施完后产 governance retrospective 留 follow-up，不在 phase scope 内。

## 8. Phase Definition of Done

- [x] 本 phase 全部 task spec Status=Done（9.1/9.2/9.3/9.4/9.5/9.6 全 Done — PR #59-64 顺序合）
- [x] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过（quickstart_smoke.sh 真 7 step PASS + release_smoke.sh 4 段 PASS）
- [x] 关联风险 ADR-013 §Rollback 三条 / R1 / R6 缓解措施已落地（proto add-only schema_version 不动 + R1 gRPC 契约 add-only 验证 + R6 examples/quickstart fixture <5KB 远低于 1MB gate）
- [x] adapter §Phase 状态索引该行 Status 同步更新（closeout PR — 本 commit）
- [x] ADR-013 状态推进 Proposed → Accepted（closeout PR — 本 commit）
- [x] PRD §Implementation Phases Phase 9 行 — 表无 Status 列，但 §Implementation Phases Phase 9 段已含 ADR-013 + Exit Criteria 详描；§Open Questions O12 已标记为本 phase follow-up（governance retrospective — 不在 v0.2 scope）
- [x] §4 Gate 3 phase smoke gate 通过（quickstart_smoke.sh + release_smoke.sh 双绿）— v0.2.0 release tag prep ready
