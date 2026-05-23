# ADR `010`: `audit-cross-language-unification`

**Status**: Proposed
**Category**: Audit / 跨语言架构
**Date**: 2026-05-23
**Decided By**: tajiaoyezi
**Related**: task-5.3 Rust audit / task-6.2 Go audit / task-7.1 MCP（复用 Go audit）/ PR #45 phase-6 closeout / ADR-002 layered storage

## Context

v0.1 ContextForge 在 audit 路径上跨 Go / Rust 两端形成双轨互补设计，源自 PR #45 phase-6 closeout 对 task-6.2 spec §3 / §5.2 / AC5 的 5 处「Go 互补 Rust」字面修正：

- **Rust 端（task-5.3）— `core/src/memoryops/audit.rs`**：collection-scoped，复用 collection 自带的 `<data_dir>/collections/<id>/metadata.sqlite` SQLite，新增 `audit_log` 表 + 索引（`idx_audit_log_operation` / `idx_audit_log_collection`）。记 memoryops data-plane 操作 — `import` / `search` / `export` / `redact`（task-5.3 §3 / §6 AC2）。schema：`operation` / `collection` / `source` / `result_count` / `redaction_count` / `timestamp` / `query_hash` / `query_length` / `redacted_terms` / `chunk_ids` / `export_total_byte_count`。脱敏规则：不记完整 query content（仅 `sha256` + 长度）/ 不记完整 secret / 不记导出内容。公开 API：`AuditSink::open(data_dir, collection)` / `AuditSink::record(event)` / `AuditSink::list()` / `count_by_operation(op)`。
- **Go 端（task-6.2）— `internal/memoryops/audit/audit.go`**：daemon-scoped，单文件 `<data_dir>/audit-rest.log` JSON-lines（`os.O_APPEND | O_CREATE | O_WRONLY`，mode `0o644`，私密性来自 `0o700` 的 dataDir 自身 — task-1.2 config.Init 落定）。记 REST 控制平面访问 — 含 `401` 拒访问 + `200` 成功。schema 极简：`endpoint` / `status` / `timestamp` / `reason`（可选）。脱敏规则：**永不** 记 Bearer token 值 / **永不** 记完整请求 body（middleware 在 `audit.Write` 之前剥离）。公开 API：`audit.Write(dataDir, Event)`。
- **MCP 端（task-7.1，codex 同期实施中）**：§2A 决策 D 复用 task-6.2 Go `audit.Write` 公开 API + Endpoint 字段 prefix `mcp:`（`mcp:initialize` / `mcp:context_search` / `mcp:context_read` / `mcp:context_explain` / `mcp:context_collections`），与 REST 共写同一 `audit-rest.log`，运维可 grep 分类。

**为什么是双轨而非单轨**（v0.1 trade-off）：

- **cross-language 限制**：Rust 不能直接调 Go 函数 / Go 不能直接调 Rust 函数；要打通必须经 gRPC RPC 或 stdio IPC 之类的进程间通道，引入新 wire + 新 proto + 新错误传播路径。
- **职责天然分层**：data-plane（memoryops 改 SQLite 内容）在 Rust core 进程；control-plane（REST / MCP 访问校验）在 Go daemon 进程。两者天然落在各自语言的 host 进程里，不强行跨语言能让公开 API 简单可测。
- **PR #45 phase-6 closeout** 已把 task-6.2 spec §3 / §5.2 / AC5 / §10 字面定型为「互补不重叠」描述（5 处），且明示「future SPEC-DRIFT 整合（Go REST audit 写入 Rust SQLite 通过 gRPC RPC 或 shared schema）留 task-8+」。本 ADR 是该 backlog 的正式调研记录。

**未来 unification 张力来源**：

- 运维查询场景：「过去 24h 全部 access」要 grep + jq Rust SQLite `audit_log` 表 + Go JSON-lines `audit-rest.log` 两边，再合并时间线。
- task-8.1 eval-harness 可能需要单一审计入口做 recall metrics + access pattern 联动分析。
- PRD §Vision「多 Agent 一致可追溯」深层诉求会在 v0.2+ 重新审视统一性必要程度。

## Decision

**v0.1 维持 Rust SQLite + Go JSON-lines 双轨互补现状不强制统一**；本 ADR 作为调研 + 未来路径明示，Status = `Proposed`（不是 Accepted —— 不强制实施任何 unification 选项，仅为后续决策提供结构化材料）。

具体落地约束在 v0.1：

- 不动 task-5.3 Rust `audit.rs` 公开 API / SQLite schema；
- 不动 task-6.2 Go `audit.Write` 公开 API / JSON-lines schema；
- task-7.1 MCP 继续按其 §2A 决策 D 复用 Go audit；
- 不引入新 proto RPC（如 `WriteAudit`）/ 不引入 Rust ↔ Go IPC 写路径。

Unification 决策延期到任一【触发条件】被满足时（见下方 Rollback Or Migration Plan）。

## Rationale

- **cross-language 限制让 v0.1 单轨成本高**：把任一端写入路径搬到另一端，都需新增进程间 wire（proto + RPC + 错误传播 + 测试）。v0.1 P0 时序（Phase 6 closeout → Phase 7 MCP → Phase 8 eval）已紧；为统一性投入设计 / 实施 / 迁移会推迟 phase 7 / 8 派工。
- **各语言原生 audit 路径已 production-ready**：task-5.3 Rust SQLite audit_log 有 4 单元测试 + Phase 5 smoke；task-6.2 Go JSON-lines 有 TEST-6.2.5 黑盒覆盖文件写入 + 脱敏 + 401 audit；两端公开 API 已 freeze 且消费者（task-5.x memoryops / task-6.2 REST / task-7.1 MCP）已 wire — 强制统一等于推翻 ≥3 个已 done task 的实施。
- **运维 query 跨双轨在 v0.1 数据规模下可承受**：Rust audit_log 是按 collection 分区的小型 SQLite（每 collection 一文件）；Go audit-rest.log 是单文件 JSON-lines（grep / jq 友好）。`audit-query` 类一次性脚本（bash + sqlite3 + jq）足够覆盖运维场景，v0.1 量级不构成 tool 缺口。
- **统一架构需先回答更深层决策**：「single source of truth」放哪？ data-plane（Rust SQLite）主导 vs control-plane（Go daemon log）主导 vs 二者并列再加 query layer —— 该选择会牵动 ADR-002 layered storage 的写路径设计、collection scope vs daemon scope 的 audit retention 策略、跨 collection 检索语义、PRD §Vision「多 Agent 一致可追溯」的实现层定位。v0.1 直接拍板任一方向都有 lock-in 风险。
- **强制 v0.1 统一会推迟 Phase 7 / 8 派工**：Phase 7 MCP（codex 进行中）和 Phase 8 eval-harness 都在等 audit 现状沉淀稳定；本 ADR-010 提前介入统一会触发 task-5.3 / 6.2 / 7.1 三个已 ready/done task 的 SPEC-DRIFT 链。

## Alternatives

- **选项 A（v0.1 选定）— 维持双轨互补**：Rust SQLite collection-scoped audit_log + Go JSON-lines daemon-scoped audit-rest.log + MCP 复用 Go audit + Endpoint prefix `mcp:`。
  - Pros：零迁移成本；与已 done 实施 1:1；与 PR #45 phase-6 closeout 修正字面一致；可逆（未来仍能选 B / C / D）。
  - Cons：运维跨双轨查询略 friction；task-8.1 eval-harness 需要兼容两种 schema 才能做联动分析；schema 演进需双端同步（如新增字段要同时改 Rust SQLite migration + Go Event struct）。
- **选项 B — 统一到 Rust SQLite（推荐 Rollback 方向）**：Go REST / MCP audit 经 gRPC RPC（新增 `proto/contextforge/v1/audit.proto` 含 `AuditRecord` message + `ContextService::WriteAudit` RPC）写入 Rust 端 daemon-scoped 或 collection-scoped SQLite。Go 端 `audit-rest.log` 文件路径删除 + 迁移历史 JSON-lines 一次性入库。
  - Pros：与 ADR-002「SQLite + Tantivy layered storage」对齐 — audit 沿用同一持久层 + 同一 query 语义；single source of truth；跨 collection / cross-scope 查询 trivial（标准 SQL）；与 task-5.3 Rust audit 现有 schema + 索引复用。
  - Cons：phase23-start-gate「proto frozen，add-only field tag」需解封以加 new message + new RPC（仍 add-only 兼容，但 freeze 通道要走主 agent gate）；Go 端写路径多一次 RPC（延迟 + 错误传播 + 进程死活 — daemon 没起来时 REST middleware 怎么办？）；一次性迁移 audit-rest.log 历史数据脚本 + 失败回滚预案；task-7.1 MCP 也要跟改 audit wire（codex 在跑，要等 PR merge 后 SPEC-DRIFT 重派）。
- **选项 C — 统一到 Go JSON-lines**：Rust memoryops 经 stdin/stdout 或 Unix socket IPC 把 audit 事件转给 Go daemon 写 `audit-rest.log`，删除 Rust SQLite `audit_log` 表。
  - Pros：单文件 + grep 友好；Go 端 query 工具不需额外 wire；daemon-scoped 视图比 collection-scoped 视图更接近运维心智模型。
  - Cons：丢失 ADR-002 SQLite 优势（事务 / 索引 / structured query）；Rust core 进程要被迫依赖 Go daemon 在线才能写 audit（违背 task-5.3 memoryops 独立可测的设计）；IPC pipe 失败时 Rust core 怎么 fallback？file write 失败容错路径变 IPC 失败容错路径，复杂度反而上升；audit 数据规模长期可能让 JSON-lines 检索成本超过 SQLite。
- **选项 D — 共享 schema 但保留双轨 + 统一查询 layer**：定义 schema 公共子集（如 `endpoint` / `actor` / `operation` / `status` / `timestamp` / `reason`）在 ADR-级 doc 里；新建 `contextforge audit-query` CLI 子命令，内部同时读 Rust SQLite + Go JSON-lines，输出归一化 JSON / Markdown 报表。
  - Pros：不动两端 storage / 不动 proto / 不动两端 audit.Write 公开 API；增量价值（解决运维查询 friction）；最低风险路径。
  - Cons：仍是双 source；audit-query 工具要持续维护两端 schema 同步；如某端 schema 漂移则报表会丢字段；不解决 task-8.1 eval-harness「单一审计入口」诉求。
- **选项 E — PRD §Vision 重审定向 audit 主导方**：把决策升级到 PRD 层 — 先回答「audit 是 data-plane（collection-scoped Rust SQLite）主导，还是 control-plane（daemon-scoped Go file）主导，还是双 actor 并列」，再据此选 B / C / D。
  - Pros：避开 v0.1 ADR 层局部最优陷阱；统一性决策与 PRD §Vision「多 Agent 一致可追溯」同源；后续 unification 实施有 PRD 锚点。
  - Cons：决策周期长（PRD 改动需用户审定 + ADR-002 / ADR-003 / ADR-004 联动审视）；v0.1 P0 不能阻塞等 PRD 重审。

## Consequences

- **正向**：
  - v0.1 维持现状 → 零 storage 层改动；零 proto 改动；零 task-5.3 / 6.2 / 7.1 已 done / ready 实施推翻；
  - audit 数据继续在两端正常 grow，为未来选 B / C / D 提供真实使用模式样本（数据规模 / 查询频率 / 跨双轨 join 频率）；
  - 决策可逆 — 触发条件清单（见 Rollback Plan）任一满足即可启动 SPEC-DRIFT；
  - ADR-010 留档让 task-8.1 eval-harness 实施者能看到 trade-off + 统一路径选项，不会重新发明决策；
  - 与 PR #45 phase-6 closeout 修正字面 1:1 — task-6.2 spec §3 / §5.2 / AC5 / §10 的「Go 互补 Rust」声明现在有完整 ADR 决策溯源（不再是孤立字面）。
- **负向 / 成本**：
  - 运维跨双轨查询 friction（grep + jq + sqlite3 三工具组合）；
  - 未来用户 / 客户 ask「为什么 audit 不统一」时仍需重复解释 — ADR-010 是回应素材，但回应人力成本不可零；
  - task-8.1 eval-harness 若需要单一审计入口 → 触发本 ADR Rollback；
  - schema 演进 friction：新增字段需双端同步（Rust SQLite migration + Go Event struct）；演进过程中如有人忘记同步会埋下 schema 漂移；
  - PRD §Vision「多 Agent 一致可追溯」深层诉求暂未在 audit 层完整兑现。
- **影响面**：
  - 直接：`core/src/memoryops/audit.rs`（Rust）/ `internal/memoryops/audit/audit.go`（Go）；
  - 间接消费者：task-7.1 MCP（复用 Go audit）/ task-6.2 REST 5 个 endpoint / task-5.3 memoryops 4 类 operation；
  - 未来：task-8.1 eval-harness（access pattern 分析）/ PRD §Vision 一致可追溯叙事；
  - 关联 ADR：ADR-002 layered storage（audit unification 若选 B 会走 SQLite 层）/ ADR-003 result schema 单一源（audit 不在 result schema 范围，但 unification 决策风格可借鉴）/ ADR-004 local-first privacy（脱敏规则 v0.1 已对齐，不影响）。

## Rollback Or Migration Plan

**触发条件**（任一即可启动 SPEC-DRIFT）：

1. task-8.1 eval-harness 实施时明确要求单一审计入口（如 recall metrics 需要 join access pattern → schema 不一致让 join 不可能）；
2. 用户 / 客户 ask「audit 不统一」频次达到阻塞反馈级别（如 ≥3 个独立 feedback）；
3. PRD §Vision「多 Agent 一致可追溯」被 v0.2+ 路线深化，需要在 audit 层做实施承诺；
4. cross-language IPC / shared storage 工具链成熟（如 sqlite3 跨语言 client lib stable、proto RPC 错误传播测试已经 Phase 6 / 7 复用充分）→ 实施成本降低；
5. 运维场景跨双轨查询出现实际性能 / 一致性问题（如 audit retention 策略需要 atomic across-scope cleanup，双轨 file + SQLite 无法原子）。

**推荐方向：选项 B（统一到 Rust SQLite）**，动作 7 步：

1. 主 agent 新开 `chore/spec-drift-audit-unification` branch → 在 task-5.3 / task-6.2 / task-7.1 spec §10 追加 SPEC-DRIFT 章节，标注 unification 决策与本 ADR-010 status 变更（Proposed → Accepted with Resolution: B）；
2. proto/contextforge/v1/audit.proto 新建（phase23-start-gate 解封 add-only）— 加 `message AuditRecord { string endpoint = 1; string status = 2; google.protobuf.Timestamp timestamp = 3; string actor = 4; string reason = 5; map<string,string> extra = 6; }` + `rpc WriteAudit(AuditRecord) returns (WriteAuditResponse)` 在 `ContextService`；
3. Rust `core/src/memoryops/audit.rs` 扩 `AuditSink` 接收 daemon-scoped event（不仅 collection-scoped），或新建并行 `daemon_audit_log` 表 / 共用 `audit_log` 表（加 `scope` 列区分 `collection` / `daemon`）；新增 gRPC server impl 接收 `WriteAudit` 请求；
4. Go `internal/memoryops/audit/audit.go` `Write` 函数改为 gRPC client：建立 `daemon.audit_client_conn`（懒初始化 + 复用 `clientConn`）；写失败时 fallback 到当前 JSON-lines 路径（degrade 而不丢 audit）；
5. 一次性 migration script `tools/migrate-audit-rest-log.go`：读历史 `<data_dir>/audit-rest.log` → 逐行 unmarshal 成 `AuditRecord` → 通过 gRPC 写 SQLite → migration 完成后保留原文件加 `.migrated` 后缀（不删，留 fallback）；
6. task-7.1 MCP / task-6.2 REST 调 `audit.Write` 公开 API 不变 — 公开 API 兼容（Go 内部实现从 file write 切到 gRPC，对调用方透明）；
7. 运维查询统一为 SQLite（`audit_log` 表 + 跨 collection / daemon-scope 查询），更新 `docs/ops/audit-query.md`（如有）使用 `sqlite3` + 标准 SQL。

**轻量备选方向：选项 D（共享 schema + 查询 layer）**，动作 3 步（如团队判定 B 成本仍高）：

1. 在 `docs/decisions/audit-schema-shared.md` 定义 Rust SQLite + Go JSON-lines 公共字段子集（不改两端 storage）；
2. 新建 `cmd/contextforge/audit-query.go` 子命令（Go 端单一实施，避免跨语言）— 内部用 `database/sql` 读 SQLite + `bufio.Scanner` 读 JSON-lines → 归一化成 `AuditQueryRow` struct → 输出 JSON / Markdown 报表；
3. 更新 `docs/ops/audit-query.md` 使用 `contextforge audit-query` 替代 `grep + jq + sqlite3` 三工具组合。

scope 评估：选项 B 改动跨 proto + Rust audit + Go audit + migration script + ops doc，但消费者公开 API 不动（task-5.3 / 6.2 / 7.1 调用方零改动），紧凑可逆；选项 D 改动仅 Go 新增子命令 + ops doc + 共享 schema doc，零 storage / proto 影响，最小风险路径。

## Follow-ups

- **关联 ADR-002 layered storage（SQLite + Tantivy）**：audit unification 若选 B，audit_log 表会成为 layered storage 的第三个写路径（chunks / metadata / audit），需要在 ADR-002 中追加写路径合规审视。
- **关联 task-8.1 eval-harness（Phase 8）**：单一审计入口诉求 + access pattern 分析依赖 audit 统一性 — eval-harness spec §3 / §6 AC 起草时应显式声明是接受双轨 + audit-query 工具，还是触发本 ADR Rollback 选 B。
- **关联 PRD §Vision「多 Agent 一致可追溯」**：audit 是该 vision 的实施层 — v0.2+ 路线深化时需把本 ADR-010 升级到 Accepted with Resolution（B / C / D 任一）。
- **关联 PR #45 phase-6 closeout 修正**：task-6.2 spec §3 / §5.2 / AC5 / §10 / §10 §2A Decisions 共 5+ 处「Go 互补 Rust」字面 — 本 ADR 是该字面的完整决策溯源，spec 字面与本 ADR Decision 段一致性自检通过。未来 unification 实施 PR 应同时刷新这 5 处字面（task-6.2 spec §3 等）+ 本 ADR Status / Resolution。
- **关联 task-7.1 MCP（codex 同期实施）**：MCP §2A 决策 D「复用 Go audit + Endpoint prefix `mcp:`」是双轨现状下的最佳路径；统一后 prefix 仍可保留为 `endpoint` 字段值，迁移成本零（Endpoint 字段是 string，跨 storage 中立）。
