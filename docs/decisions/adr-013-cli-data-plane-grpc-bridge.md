# ADR `013`: `cli-data-plane-grpc-bridge`

**Status**: Accepted
**Category**: 协议接口 / 架构
**Date**: 2026-05-24
**Decided By**: tajiaoyezi objective + main agent execution
**Related**: ADR-001 (Go/Rust 双二进制) / ADR-002 (SQLite + Tantivy 分层存储) / ADR-003 (CLI/REST/MCP/gRPC) / ADR-007 (minimal tarball) / Phase 2 / Phase 3 / Phase 6 / Phase 8 / task-1.4 §3 / task-2.4 / task-6.1 / task-8.3 / PRD §User Flow / PRD §Implementation Phases / proto/contextforge/v1/service.proto

## Context

v0.1.0 release (tag `v0.1.0`, master `ce47d17`, 2026-05-23) 在 spec / 治理 / release contract 层面声明完成，但 2026-05-24 主 agent 端到端实测发现以下 spec drift：

1. **CLI 数据通路断**：
   - `internal/cli/index.go` 只写 `runtime/index-<collection>.resume.json` 占位 manifest，**不调用** Rust core scanner/parser/chunker/indexer 任何 API；处理结果永远 `processed=0 total=0`。
   - `internal/cli/cli.go` `import` 子命令直接返回 `not implemented (Phase 2+/6/7/8; task-1.4 registers the skeleton only)`，尽管 `internal/importer/{hermes,openclaw,agentrules,fallback}` Go 包已在 Phase 3 实现并 Done。
   - `proto/contextforge/v1/service.proto` 仅暴露 `rpc Search` + `rpc Health`，**没有 `rpc Index`**。`core/src/indexer/mod.rs::IndexSession::index_path` 实现完整但只能在 Rust 单元测试 (`core/tests/phase2_smoke.rs` / `phase6_smoke.rs`) 内通过 `IndexSession::open` 直接调用。

2. **task-8.3 release smoke 用假证据通过**：
   - `internal/release/release_test.go::TestTask83_AC2` 构造 `[]StepResult{Status: StepPassed, Evidence: "ok"}` 喂给 `ValidateSmokeEvidence`，验证"validator 接受全 passed 输入"，**不实际执行**任何 init / import / index / search CLI 命令。
   - `TestTask83_AC1` 用 `name+"\n"` 作为 fake binary content 构造 tarball 验证文件结构，**不构建** real binary。
   - `scripts/release_smoke.sh` 跑 `go test internal/release` + `cargo test phase_6_search_grpc_end_to_end_smoke`，**没有任何步骤跑 `./contextforge init` 等真实 CLI binary**。

3. **PRD 字面承诺与实际背离**：
   - PRD §User Flow 主流程步 2 字面：`contextforge import openclaw <ws>` / `contextforge index ./project` → scanner→chunker→indexer 流水线。实际：`import` 报 not-implemented，`index` 静默存根。
   - PRD §Implementation Phases Phase 2 Exit Criteria：`contextforge index ./sample_project` 能索引 ≥1000 文件。实际：CLI `index` 不索引任何文件；Phase 2 ≥1000 测试通过靠 Rust `core/tests/phase2_smoke.rs` 内 `IndexSession` 直接调用，绕过 CLI 路径。
   - PRD §Implementation Phases Phase 3 Exit Criteria：Hermes `MEMORY.md` 能导入为 canonical record。实际：importer Go 包能做，但无 CLI 入口。

**Spec drift 形成原因（retrospective）**：

- task-1.4 §3 Out Of Scope 明确把所有非 init 子命令业务实现推到 Phase 2+/6/7/8，仅注册 not-implemented 骨架。
- task-2.4 (indexer Phase 2 收口) §3 Out Of Scope: "REST/MCP/gRPC 暴露 indexer (Phase 6/7 — 本 task 仅 Rust API + Rust smoke)"。Phase 2 §6 注："CLI `contextforge index` 端到端在 Phase 6 task-6.1 实现后由 Phase 8 task-8.3 release smoke 接管。"
- task-6.1 (Phase 6 cli-search) §3 In Scope 只覆盖 CLI `search` + Rust `CoreService::search` wire，**未规划** CLI `index` / `import` wire 或 proto `rpc Index` 扩展。
- task-8.3 §3 Out Of Scope 自承："修复所有历史产品 gap（如完整 import CLI 体验）；本 task 只建立 release smoke 可判定门"，但同 task §6 AC2 仍标 `[x]` 通过"解包→init→import→index→search/MCP→export→eval run 端到端"。AC2 字面承诺 ⊃ §3 Out Of Scope，且 §6 AC2 通过靠 §3 自承未做的功能 — **AC2 是假 Done**。

击鼓传花链：Phase 1 → Phase 2 → Phase 6 → Phase 8 每一站都把 CLI 数据通路推给下一站；task-8.3 终点宣布"out of scope"，但同 task AC 仍勾选通过。

## Decision

设立 **Phase 9 cli-pipeline** 作为 v0.2.0 minor release 收口 phase，通过以下措施补齐 spec drift：

1. **proto add-only 扩展**（task-9.1）：
   - `service ContextService` 增 `rpc Index(IndexRequest) returns (stream IndexProgress)`。
   - 新增 messages：`IndexRequest`（source_path / data_dir / collection_id）+ `IndexProgress`（files_processed / chunks_written / current_file / done / error）。
   - `schema_version` 仍 `0.1`（PRD §Decisions Log D1 R1：proto 仅加字段、不删不改 tag）。
   - **不引入** `rpc Import` —— 见 D1 决策（采两步式：import 产出 canonical JSONL → index 灌入）。

2. **Rust gRPC handler 桥接**（task-9.2）：
   - `core/src/server.rs::CoreService::index` 流式实现 wrap `IndexSession::index_path`；错误映射到 gRPC `Status`。
   - 新增 `IndexSession::scan_path_with_progress` 或类似 hook 用于流式 progress 上报（保持 `index_path` 兼容）。

3. **Go CLI 真实接通**（task-9.3 / 9.4）：
   - `internal/cli/index.go` 改写：调 `daemon.Index()` 流式 → 进度条；`--resume` 仍走 task-8.2 reliability manifest 但叠加 file-level checkpoint。
   - `internal/cli/import.go` 实现：`contextforge import hermes|openclaw|agent-rules <path> --collection X --data-dir Y` → 调 `internal/importer/<src>` 解析 → canonical JSONL 写到 `data_dir/imports/<source>-<timestamp>.jsonl`（**不调 daemon，本步纯 Go 离线**）→ 提示用户跑 `contextforge index --source <jsonl>` 灌入。

4. **task-8.3 假证据测试取代**（task-9.5）：
   - 删除 `TestTask83_AC2_ReleaseSmokeEvidenceRequiresOrderedPassingSteps` / `TestTask83_AC3_BenchmarkRequires100kChunksAndSub500msP95` / `TestTask83_AC4_V01ClosureRequiresSevenTechnicalAreas` 的 fake-evidence 版本。
   - 重写为真集成测试：`TestPhase9ReleaseSmoke_EndToEnd` 在 `t.TempDir()` 中真实跑：`go build` contextforge → `cargo build` contextforge-core → 跑 init → import hermes fixture → index → search → eval run，验证退出码 + 输出包含期望关键字。
   - `scripts/release_smoke.sh` 增 CLI 端到端段，独立于 unit-level contract test。

5. **README quick start 可复现**（task-9.6）：
   - `examples/quickstart/` fixture 目录（伪造 sample 项目 + Hermes MEMORY.md）。
   - `scripts/quickstart_smoke.sh` 一键跑全流程 + 比对输出（CI 可跑）。
   - README 改成基于该 fixture 的可复制命令序列。

6. **v0.2.0 release**：
   - `git tag v0.2.0` + `RELEASE_NOTES.md` v0.2.0 章节。
   - `docs/releases/v0.2.0-evidence.md` + `v0.2.0-artifacts.md`（按 ADR-007 产物清单）。
   - v0.1.0 tag 保留为里程碑（"内部模块 ready / CLI 数据通路待 v0.2"），不撤回。

## Rationale

- **不重定义 PRD**：PRD §User Flow / §Implementation Phases 字面承诺 CLI 端到端可跑；v0.1 实测不通是实现层 spec drift，不是 PRD 错。修代码 > 改 PRD。
- **proto add-only 兼容 R1**：PRD §Technical Risks R1 + ADR-001/003 要求 proto 仅加字段不删 tag。`rpc Index` 是新增 service method（不动 `rpc Search` / `rpc Health`），新增 messages 独立 tag namespace，对现有 client backward compatible。
- **两步式 import 简化耦合**：方案 D1 两步式（import 离线产 JSONL → index 灌入）允许 importer 不依赖 daemon，可在离线 / CI / 数据迁移场景下纯 Go 跑；同时让 `rpc Index` proto 只需 SCAN_PATH 单模式，不需要 FEED_RECORDS 复杂 stream。
- **task-8.3 假证据测试取代而非共存**：fake-evidence 测试在概念上验证的是 validator 健壮性，但当 release smoke 的整个目的是"真端到端 evidence"时，伪证据测试的存在反而误导后续阅读者以为 release smoke 真跑过。直接删除并由真集成测试取代，避免概念污染。
- **Phase 9 ≠ v0.1 bug fix**：spec drift 范围跨 Phase 2 / 3 / 6 / 8 四个 phase 和 5+ task，且需要 proto schema 扩展。归类为 v0.2.0 minor release 而非 v0.1.1 hotfix 反映真实工作量。
- **v0.1.0 tag 不撤回**：v0.1.0 包含完整的 spec / 测试 / 内部模块实现 + 治理基线 (ADR-001..012) + release contract harness。撤回会损失里程碑信息；保留 + v0.2.0 补齐是更诚实的项目历史。

## Consequences

- v0.1.0 release 在 RELEASE_NOTES 中追注 known issue（CLI 数据通路未通）；README quick start 在 v0.2.0 之前不应被外部用户照抄。
- `proto/contextforge/v1/service.proto` 引入新 RPC 后必须跑 `buf generate` 重生成 Go / Rust 绑定；本 phase 不动 `schema_version`（保持 v0.1 契约 freeze 表面 add-only 演进）。
- task-8.3 §10 标记 §3 Out Of Scope "完整 import CLI 体验" 在 Phase 9 被 task-9.4 处理；task-8.3 spec 自身不动（已 Done），但需在 adapter / PRD 标注 Phase 9 桥接关系。
- 新增 6 个 task + 1 phase spec + 本 ADR + adapter / PRD / feature 同步，按 ADR-011/012 单驱动 + 主 agent 自治流程实施：spec PR 主 agent 自决合，每个 task PR 走 §4 Gate 0-5 自决合。
- Phase 9 完成后 ADR-013 状态 Proposed → Accepted；docs/s2v-adapter.md Phase 索引追加 Phase 9 行（Status=Done after 6 task merge）。

## Rollback Or Migration Plan

如 Phase 9 实施中发现：

1. **proto add-only 不可行**（实际撞 schema_version freeze 红线）：放弃 `rpc Index`，改 CLI 直接 fork `contextforge-core` 子进程 + 私有 stdin/stdout JSON-RPC（PRD §Decisions Log D3 已 reject 此方案，需新 ADR superseding D3）。
2. **两步式 import UX 不可接受**（用户反馈坚持单条命令）：改方案 D1 选项 B（import 内调 gRPC FEED_RECORDS），需新 task 扩 proto stream-records 模式。
3. **task-9.5 真集成测试 flake 不可控**（cargo build 在 Windows / WSL2 / Linux runner 不一致）：回退到 unit-level contract test 为主 + 单条最小 smoke (init only) 作为 CI gate；真端到端移到本地 release-time manual smoke。

Rollback 通过新 ADR 取代本 ADR 完成；不允许直接修改本 ADR `Decision` 字段（standard.md §16.2 ADR 不可变性）。

## Follow-ups

- Phase 9 实施完成后回填本 ADR Status: Proposed → Accepted（在 chore phase-9-closeout PR 中）。
- task-8.3 §10 加 cross-link 指向 ADR-013 + task-9.5（标注假证据测试在 v0.2 被取代）。
- PRD §Open Questions 新增 O12："Phase 1-8 spec drift 形成机制 — 击鼓传花链怎么在治理层提前发现？"（governance 改进 follow-up，可能产出新 ADR 关于"Phase 顶层 Exit Criteria 与 task 收口 AC 必须 cross-validation"）。
- 评估是否需要 ADR-014 review 主 agent 自治在 spec-drift 检测层面的能力边界（ADR-012 把 §2A / merge / Waive 交给主 agent；但 spec drift 检测需要跨 phase / 跨 task 视角，单 task 视角的主 agent 容易漏）。
