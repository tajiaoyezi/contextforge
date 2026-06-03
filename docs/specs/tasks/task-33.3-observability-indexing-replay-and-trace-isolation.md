# Task `33.3`: `observability-indexing-replay-and-trace-isolation — (a) indexing.* 事件持久化（add-only migration 0019_indexing_events）+ replay mapper 扩展（mapper 🟢 / e2e 🟡）；(b) TraceStore 严格多-workspace 隔离（add-only proto workspace_id + SQL WHERE filter + handler/store 接线，empty workspace_id = aggregate-all 向后兼容 ADR-004）；(c) events-drain-timeout VERIFY-ONLY（Phase 26 已交付，引证既有测试）`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 33 (governance-debt-cleanup-2)
**Dependencies**: 既有 `core/src/jobs/index_session_backend.rs`（`build_progress_event` / `build_error_event` / `build_cancelled_event` → `eb.send`，best-effort 内存广播，Phase 11/26 已交付）/ `core/src/data_plane/events.rs`（`replay_events_from_audit` `:391-420` + `audit_op_str_to_event` `:369-377`，memory_* 专用 replay mapper，task-26.2 / ADR-031 D4 已交付，indexing marker `[SPEC-DEFER:phase-future.indexing-event-persistence]` @ `:389`）/ `core/src/memoryops/audit.rs`（`AuditOperation` enum `:12-25` + `AuditLogEntry` `:56-70`，task-13.1/26.2/27.2 已交付）/ `core/src/data_plane/search_persist.rs`（`SqliteTracePersist` get/list/search_fts/load_warm，task-16.1/26.1 已交付）/ `core/src/data_plane/search.rs`（in-mem `TraceStore` + `get_search_trace` `:460-480` / `list_queries` `:486-502` handler）/ `proto/contextforge/console_data_plane/v1/console_data_plane.proto`（`GetSearchTraceRequest` `:237-239` + `ListQueriesRequest` `:255-257`）/ `internal/consoleapi/grpcclient/grpcclient.go`（`drainTimeoutFromEnv` `:407-419`，task-26.3 / ADR-031 D5 已交付）/ ADR-038（governance-debt-cleanup-2，本 task 即其 D3 原文实现）/ ADR-031（observability-hardening，本 task 以 add-only Amendment 记 indexing replay + drain verify-only）/ ADR-016（trace isolation proto add-only field）/ ADR-021（memory-event 桥接源）/ ADR-004（默认行为 + 既有契约不变）/ ADR-008（dep add-only，Phase 33 = 0 新 dep）/ ADR-013（禁伪造红线——e2e 不可在 CI 默认闭环维度如实 defer，不预填真实数值）/ ADR-012（main-agent-governance-autonomy）/ ADR-014 D1-D5（第二十四次激活）

## 1. Background

Phase 26（ADR-031）建立了 observability 持久化与 replay 骨架，但留下三处缺口/校正点；本 task 据真实源码诚实化范围（三者皆 add-only / 行为不变）：

- **B1 indexing.* 事件仅内存广播、无持久 replay 源（REAL gap）**：`core/src/jobs/index_session_backend.rs` 的三个 emit 点——progress（`:157-168` `build_progress_event` → `eb.send`）、error（`:182-193` `build_error_event` → `eb.send`）、cancelled（`:210-218` `build_cancelled_event` → `eb.send`）——皆 best-effort 内存 `EventBus` 广播（无订阅者时 `SendError` 被吞，`:158` 注释明记 best-effort），**不落任何持久存储**。`core/src/data_plane/events.rs` 的 `replay_events_from_audit`（`:391-420`）+ `audit_op_str_to_event`（`:369-377`）只处理 `memory_*`（`:389` doc 明记「indexing events lack one, `[SPEC-DEFER:phase-future.indexing-event-persistence]`」）。`AuditOperation` enum（`audit.rs:12-25`）**无** indexing variant；`AuditLogEntry`（`audit.rs:56-70`）**缺** `job_id` / `processed` / `total` 字段——audit_log 不是 indexing lifecycle 的合适持久源。守阻：daemon 重启后 indexing 历史 lifecycle 不可 replay（订阅者错过的 progress/cancelled/error 无从重建）。
- **B2 TraceStore 无 workspace 隔离（add-only proto + SQL filter）**：`core/src/data_plane/search_persist.rs` 的 `get`（`:129-142`）/ `list`（`:147-174`）/ `search_fts`（`:219-259`）/ `load_warm`（`:184-213`）**无 `WHERE workspace_id`**（`workspace_id` 列已在 schema `0015_search_traces.sql:9` 且已 SELECT 出，但从不作过滤谓词）；in-mem `TraceStore`（`search.rs`）get/list 同样不过滤；handler `get_search_trace`（`search.rs:460-480`）/ `list_queries`（`search.rs:486-502`）忽略 workspace。proto `GetSearchTraceRequest`（`console_data_plane.proto:237-239`，仅 `query_id=1`）+ `ListQueriesRequest`（`:255-257`，仅 `limit=1`）**无 `workspace_id` 字段**——console 客户端无法按 workspace 隔离 trace 历史（多租户/多 workspace 下交叉可见）。marker `[SPEC-DEFER:phase-future.tracestore-multi-workspace-strict]`（task-16.1:288）。
- **B3 events-drain-timeout = VERIFY-ONLY（Phase 26 已交付）**：`CONSOLE_EVENTS_DRAIN_TIMEOUT` 已是真实可配项——`internal/consoleapi/grpcclient/grpcclient.go:405` `var eventsDrainTimeout = drainTimeoutFromEnv()` + `drainTimeoutFromEnv`（`:407-419`，default 100ms，非法/非正回落 default），且有通过测试 `TestDrainTimeoutFromEnv`（`grpcclient_test.go:867-895`，task-26.3 / ADR-031 D5）。本 task 把该维度从「add」改为「verify-only」（与 Phase 31 event-bus-partition verify-only 校正同形），**不重新实现**，仅引证既有测试断言其仍绿。

经核：B1 replay mapper（add-only 持久行 → 重建 indexing.* PbEvent，id/ts ASC）为纯函数 🟢 可单测（镜像 TEST-26.2.3 `events.rs:493`）；端到端 restart-then-replay 须 running daemon / job runner，CI 默认不闭环 → 🟡 `[SPEC-DEFER:phase-future.indexing-replay-e2e]`，不伪造数值（ADR-013）。B2 SQL `WHERE workspace_id` filter + handler 接线 🟢 可单测（empty workspace_id 必须保持 aggregate-all 既有行为，ADR-004），e2e console 多-workspace 隔离 🟡。B3 verify-only 0 改动。**0 新 dep**（migration 经 `include_str!` 编译入二进制，rusqlite bundled；proto add-only field 经 buf generate）；新 migration = `0019_indexing_events`（最新为 0018）。

## 2. Goal

(1) **B1 indexing 事件持久化 + replay mapper**：add-only migration `0019_indexing_events`（专用表，比复用 audit_log 更干净——audit_log 无 job_id/processed/total，强行复用须改 `AuditLogEntry` 全链路 + enum），在三个 emit 点（`index_session_backend.rs:157-168`/`:182-193`/`:210-218`）就地**额外**持久写一行 indexing lifecycle（`job_id` / `stage` / `processed` / `total` / `ts`），并扩展 replay mapper（新 `replay_indexing_events_from_store` 或扩展现有）以 id/ts ASC 重建 indexing.* `PbEvent`（真实 `job_id` / `processed` / `total` 取自持久行，非合成，ADR-013）。mapper 单测 🟢；端到端 restart-then-replay 🟡 `[SPEC-DEFER:phase-future.indexing-replay-e2e]`。(2) **B2 TraceStore 多-workspace 隔离**：add-only `string workspace_id` 字段 on `GetSearchTraceRequest`（`:237-239`，下一 tag `=2`）+ `ListQueriesRequest`（`:255-257`，下一 tag `=2`），buf generate 重生 Go/Rust binding；接线到 `SqliteTracePersist` get/list/search_fts + in-mem `TraceStore` + handler，加 `WHERE workspace_id = ?`（in-mem 同义谓词）；**empty `workspace_id` 必须保持当前 aggregate-all 行为**（ADR-004 向后兼容——既有 client 不传字段 → 空 → 全聚合，结果与改前一致）。SQL/handler 单测 🟢；e2e console 隔离 🟡。(3) **B3 drain-timeout verify-only**：不改代码，引证 `TestDrainTimeoutFromEnv`（`grpcclient_test.go:867-895`）断言既有行为；ADR-031 以 add-only Amendment 记 verify-only 校正。

pass bar：indexing replay mapper 纯函数单测（add-only migration round-trip + id/ts ASC 重建 indexing.* PbEvent，真实 job_id/processed/total）🟢；TraceStore add-only proto field + SQL WHERE filter，empty=aggregate-all byte-equiv（既有 get/list/search_fts 行为不变）🟢；drain-timeout verify-only 引证既有测试绿 🟢；e2e restart-replay + e2e console 多-workspace 隔离 🟡 据实延后不伪造（ADR-013）；默认行为 / proto（add-only field）/ 既有契约不变（ADR-004）+ 0 新 dep（ADR-008）；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- **B1（a）** 新增 `core/migrations/0019_indexing_events.sql`——专用表 `indexing_events`（`id INTEGER PRIMARY KEY` 自增 / `job_id TEXT NOT NULL` / `stage TEXT NOT NULL`（`indexing` / `cancelled` / `error`）/ `processed INTEGER NOT NULL DEFAULT 0` / `total INTEGER NOT NULL DEFAULT 0` / `ts_unix INTEGER NOT NULL` / 可选 `message TEXT`），`CREATE TABLE IF NOT EXISTS` 幂等，加 `idx_indexing_events_id_asc`（或依 PK 自然序）；经 `include_str!` 编译入二进制（镜像 `0015_search_traces.sql` 既有 pattern）。
- **B1（b）** 在 `index_session_backend.rs` 三个 emit 点（progress `:157-168` / error `:182-193` / cancelled `:210-218`）就地**额外**持久写一行（best-effort 持久——与既有 best-effort `eb.send` 同语义，写失败不阻断 indexing 主流程，但写成功才作为 replay 源）。须把持久 sink（`SqliteIndexingEventStore` 或既有 store handle）注入到 backend（add-only 字段 / Option，既有构造兼容）。
- **B1（c）** `core/src/data_plane/events.rs` 加 `replay_indexing_events_from_store`（或同形纯 mapper `indexing_rows_to_pb_events(rows) -> Vec<PbEvent>`）——输入 `Vec<IndexingEventRow>`（id ASC），输出 indexing.* `PbEvent`（`event_type` = `indexing.progress` / `indexing.cancelled` / `indexing.error`，`job_id` / payload 含 `processed` / `total` 取自行，`event_id` = 确定性 `evt-idx-{id}` 以便 replay→live splice dedup）；保留 `:389` doc 的 `[SPEC-DEFER:phase-future.indexing-event-persistence]` marker 措辞更新为「已落地持久源 0019，端到端 restart-replay `[SPEC-DEFER:phase-future.indexing-replay-e2e]`」。
- **B2（a）** `console_data_plane.proto`——`GetSearchTraceRequest`（`:237-239`）add-only `string workspace_id = 2`；`ListQueriesRequest`（`:255-257`）add-only `string workspace_id = 2`（既有 tag 1 不动，ADR-004 add-only）；buf generate 重生 Go（`internal/consoleapi/.../*.pb.go`）/ Rust（`core/src/.../*.rs`）binding。
- **B2（b）** `core/src/data_plane/search_persist.rs`——`get` / `list` / `search_fts` 加 `workspace_id: &str` 形参，非空 → `WHERE workspace_id = ?`（`get`：`AND workspace_id = ?`；`list` / `search_fts`：`WHERE`/`AND workspace_id = ?`），**空 → 不加谓词（aggregate-all 既有行为）**；in-mem `TraceStore`（`search.rs`）get/list 加同义 workspace 谓词（空 → 全返回）。
- **B2（c）** handler `get_search_trace`（`search.rs:460-480`）/ `list_queries`（`search.rs:486-502`）从 request 读 `workspace_id` 透传到 store；empty workspace_id 保持现 aggregate-all 路径。
- **B3** events-drain-timeout VERIFY-ONLY——0 代码改动；§9 引证 `TestDrainTimeoutFromEnv`（`grpcclient_test.go:867-895`）+ `drainTimeoutFromEnv`（`grpcclient.go:407-419`）断言既有行为绿；ADR-031 add-only Amendment 记 verify-only 校正（@ ADR-038 D3，不溯改 ADR-031 正文）。
- 同源测试：indexing replay mapper 纯函数单测（🟢 mirror TEST-26.2.3）+ persist round-trip（🟢 写 0019 行 → 读回 → mapper 重建）；TraceStore SQL workspace filter 单测（🟢 空=aggregate-all / 非空=隔离）+ handler workspace 透传单测（🟢）；drain-timeout verify-only 引证既有测试（🟢）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- 端到端 indexing restart-then-replay（须 running daemon + job runner 真实跑一轮 indexing 后重启 replay）[SPEC-DEFER:phase-future.indexing-replay-e2e]——本 task 仅交付持久源 0019 + mapper 纯函数（🟢），e2e 须 live daemon CI 默认不闭环，据真实跑出回填（ADR-013 不伪造）。
- 端到端 console 多-workspace trace 隔离（须 console-api 真实双 workspace 跨进程隔离验证）[SPEC-DEFER:phase-future.tracestore-multi-workspace-strict]——本 task 交付 add-only proto field + SQL/handler 接线（🟢），e2e console 隔离 🟡 据实延后。
- audit_log 复用承载 indexing lifecycle（须改 `AuditOperation` enum + `AuditLogEntry` 加 job_id/processed/total 全链路）——经核 audit_log 无这些列，强行复用是更脏的方案；本 task 采专用表 0019（更干净），不动 audit 链路。
- indexing.* event SSE live-tail / 过滤 UI（console-api 表单按 stage 过滤）[SPEC-DEFER:phase-future.indexing-event-ui]——本 task 仅持久 + replay mapper，不含 UI。
- 真实 release tag / run-id / digest（v0.26.0）[SPEC-OWNER:task-33.4-closeout]（ADR-012 用户授权后回填）。

## 4. Actors

- 主 agent（ADR-012 自治）
- `IndexSessionBackend`（`core/src/jobs/index_session_backend.rs`，三个 emit 点，本 task 加持久写）
- `SqliteIndexingEventStore`（新，`core/migrations/0019_indexing_events.sql` 持久源 + read API）
- indexing replay mapper（`core/src/data_plane/events.rs`，新 `indexing_rows_to_pb_events` / `replay_indexing_events_from_store`）
- `SqliteTracePersist` + in-mem `TraceStore`（`core/src/data_plane/search_persist.rs` + `search.rs`，本 task 加 workspace filter）
- handler `get_search_trace` / `list_queries`（`core/src/data_plane/search.rs:460-502`，本 task 透传 workspace_id）
- `GetSearchTraceRequest` / `ListQueriesRequest`（`console_data_plane.proto:237-239` / `:255-257`，本 task add-only workspace_id field）
- `drainTimeoutFromEnv`（`internal/consoleapi/grpcclient/grpcclient.go:407-419`，verify-only 引证源）
- console 客户端 / 运维（多-workspace 隔离消费方）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/jobs/index_session_backend.rs:157-168`（progress emit `build_progress_event` → `eb.send` best-effort，`:158` 注释「SendError swallowed」）+ `:182-193`（error emit `build_error_event`）+ `:210-218`（cancelled emit `build_cancelled_event`）——三个持久写落点
- `core/src/data_plane/events.rs:369-377`（`audit_op_str_to_event` memory_* mapper）+ `:391-420`（`replay_events_from_audit` 纯函数 + id ASC + `evt-audit-{id}` dedup id pattern）+ `:389`（indexing marker `[SPEC-DEFER:phase-future.indexing-event-persistence]`，本 task 兑现持久源）+ `:493`（TEST-26.2.3，mapper 单测镜像源）
- `core/src/memoryops/audit.rs:12-25`（`AuditOperation` enum 无 indexing variant）+ `:56-70`（`AuditLogEntry` 缺 job_id/processed/total——audit_log 不适合承载 indexing，故采专用表 0019）
- `core/src/data_plane/search_persist.rs:129-142`（`get` 无 WHERE workspace_id）+ `:147-174`（`list` 无 WHERE workspace_id，但已 SELECT workspace_id）+ `:184-213`（`load_warm`）+ `:219-259`（`search_fts` 无 WHERE workspace_id）+ `core/migrations/0015_search_traces.sql:9`（`workspace_id TEXT NOT NULL` 列已在）
- `core/src/data_plane/search.rs:460-480`（`get_search_trace` handler 忽略 workspace）+ `:486-502`（`list_queries` handler 忽略 workspace）
- `proto/contextforge/console_data_plane/v1/console_data_plane.proto:237-239`（`GetSearchTraceRequest` 仅 query_id=1）+ `:255-257`（`ListQueriesRequest` 仅 limit=1）——add-only workspace_id=2 落点
- `internal/consoleapi/grpcclient/grpcclient.go:405`（`var eventsDrainTimeout = drainTimeoutFromEnv()`）+ `:407-419`（`drainTimeoutFromEnv` default 100ms）+ `grpcclient_test.go:867-895`（`TestDrainTimeoutFromEnv`，verify-only 引证）
- `core/migrations/0015_search_traces.sql`（migration pattern：CREATE TABLE IF NOT EXISTS + index + `include_str!` 编译——0019 镜像源）+ `core/src/data_plane/search_persist.rs:28`（`include_str!("../../migrations/0015_search_traces.sql")` pattern）
- `docs/decisions/adr-038-governance-debt-cleanup-2.md §D3`（本 task 即其原文实现）+ `docs/decisions/adr-031-observability-hardening.md`（add-only Amendment：indexing replay 持久源 + drain verify-only 校正）+ `docs/decisions/adr-016-*.md`（trace isolation proto add-only field）+ `docs/decisions/adr-021-*.md`（memory-event 桥接源 pattern）

### 5.2 关键设计

- **B1 专用表优于复用 audit_log**：audit_log（`AuditLogEntry` `:56-70`）无 `job_id` / `processed` / `total` 列，且 `AuditOperation` enum（`:12-25`）无 indexing variant——强行复用须改 enum + 全链路加列，是更脏方案；故采 add-only 专用表 `0019_indexing_events`（`job_id` / `stage` / `processed` / `total` / `ts_unix`），与 indexing lifecycle 形状 1:1。pass bar：persist round-trip 单测——三个 emit 点写行 → 读回 → 字段（job_id/stage/processed/total/ts）与写入一致。
- **B1 replay mapper 纯函数 + 真实字段**：`indexing_rows_to_pb_events(rows)` 输入 `Vec<IndexingEventRow>`（id ASC，由 store 的 `list()` 保证），逐行 map：`stage="indexing"` → `event_type="indexing.progress"`、`"cancelled"` → `"indexing.cancelled"`、`"error"` → `"indexing.error"`；`PbEvent.job_id = row.job_id`，`payload_json` 含 `processed` / `total` 取自行（**非合成**，ADR-013——真实值来自持久行）；`event_id = format!("evt-idx-{}", row.id)`（确定性，replay→live splice dedup，镜像 `evt-audit-{id}` pattern `:408`）；输出保 id ASC。pass bar：mapper 单测（mirror TEST-26.2.3 `:493`）——给定 rows → 期望 indexing.* PbEvent 序列（id ASC + 真实字段 + 确定性 event_id）。
- **B2 empty workspace_id = aggregate-all（ADR-004 向后兼容关键不变量）**：proto add-only `workspace_id=2`（既有 client 不传 → proto default 空串）；store 层 `if workspace_id.is_empty() { /* 不加谓词，既有 SQL 不变 */ } else { /* 加 WHERE/AND workspace_id = ? */ }`——空路径与改前 byte-equivalent（既有 get/list/search_fts SQL 完全一致），非空路径加隔离谓词。pass bar：(a) 空 workspace_id → 结果与改前 aggregate-all 完全一致（byte-equiv 单测）；(b) 非空 workspace_id=A → 只返回 workspace A 的 trace（隔离单测，跨 workspace 不可见）。
- **B2 add-only proto field 不破既有 client（ADR-004）**：`workspace_id` 用下一可用 tag `=2`（两 message 现各仅 tag 1）；既有字段 tag/类型不动；buf generate 重生 binding 后既有 client 不传该字段 → 空 → aggregate-all（行为不变）。
- **B3 drain-timeout verify-only（不重新实现）**：`CONSOLE_EVENTS_DRAIN_TIMEOUT` 已真实可配（`grpcclient.go:405/:407-419`）+ 有通过测试（`grpcclient_test.go:867-895`）；本 task **0 代码改动**，仅 §9 引证既有测试断言绿（与 Phase 31 event-bus-partition verify-only 校正同形）。ADR-031 add-only Amendment 记此为「已交付，verify-only」，不伪造为新交付（ADR-013）。
- **honest-defer 边界（ADR-013）**：indexing 持久源 0019 + mapper 纯函数 🟢 本 task 交付；端到端 restart-then-replay（live daemon + job runner）🟡 `[SPEC-DEFER:phase-future.indexing-replay-e2e]`；TraceStore SQL/handler workspace filter 🟢 本 task 交付，e2e console 多-workspace 隔离 🟡 `[SPEC-DEFER:phase-future.tracestore-multi-workspace-strict]`——🟡 维度据真实跑出回填，不预填数值。

### 5.3 不变量

- 默认行为不变（ADR-004）：empty workspace_id → store get/list/search_fts 与改前 byte-equivalent（aggregate-all 既有结果）；既有 indexing `eb.send` best-effort 广播路径不变（持久写为**额外**写，不替换广播）；proto add-only field 既有 client（不传 workspace_id）行为不变。
- 既有契约不变：proto add-only `workspace_id=2`（既有 tag 1 不动，ADR-016 add-only）；store API 加 `workspace_id` 形参为 add-only（既有内部调用点同步传入，空串保旧行为）；migration 0019 为 add-only 表（不改既有 0010-0018 schema，ADR-014 D5 不溯改）；两进程拓扑不变（ADR-016）。
- 0 新代码依赖（ADR-008）：migration 0019 经 `include_str!` 编译（rusqlite bundled，无新 Cargo dep）；proto add-only field 经既有 buf generate；drain-timeout verify-only 0 改动；默认构建 0 新 dep / 0 network（ADR-004）。
- honest 守线（ADR-013）：indexing replay 真实 job_id/processed/total 取自持久行（非合成）；e2e restart-replay + e2e console 隔离受阻维度如实 `[SPEC-DEFER]`，不伪造完成 / 不预填数值；drain-timeout verify-only 引证既有测试不伪造为新交付。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [ ] **AC1**（indexing 事件持久化 + replay mapper 🟢 / e2e 🟡）: add-only migration `0019_indexing_events`（`job_id`/`stage`/`processed`/`total`/`ts_unix`，`include_str!` 编译）+ `index_session_backend.rs` 三 emit 点（`:157-168`/`:182-193`/`:210-218`）就地额外持久写 + `events.rs` 新 indexing replay mapper（`indexing_rows_to_pb_events`，id/ts ASC 重建 indexing.* `PbEvent`，真实 job_id/processed/total 取自持久行、确定性 `evt-idx-{id}`，非合成 ADR-013）；端到端 restart-then-replay 🟡 `[SPEC-DEFER:phase-future.indexing-replay-e2e]` — verified by **TEST-33.3.1**（mapper 纯函数，mirror TEST-26.2.3）+ **TEST-33.3.2**（persist round-trip：emit → 0019 行 → 读回 → mapper 重建）
- [ ] **AC2**（TraceStore 多-workspace 隔离 add-only proto + SQL filter 🟢 / e2e 🟡）: `GetSearchTraceRequest`（`:237-239`）+ `ListQueriesRequest`（`:255-257`）add-only `string workspace_id = 2`（buf generate 重生 binding）+ `search_persist.rs` get/list/search_fts + in-mem `TraceStore` 加 `WHERE workspace_id` filter + handler（`search.rs:460-502`）透传；**empty workspace_id = aggregate-all 既有行为 byte-equiv**（ADR-004 向后兼容）；e2e console 隔离 🟡 `[SPEC-DEFER:phase-future.tracestore-multi-workspace-strict]` — verified by **TEST-33.3.3**（SQL：空=aggregate-all byte-equiv / 非空=workspace 隔离）+ **TEST-33.3.4**（handler：request workspace_id 透传到 store，空保旧路径）
- [ ] **AC3**（events-drain-timeout VERIFY-ONLY 🟢）: `CONSOLE_EVENTS_DRAIN_TIMEOUT` 已于 Phase 26 交付（`grpcclient.go:405`/`:407-419` `drainTimeoutFromEnv` default 100ms），本 task 0 代码改动，引证既有 `TestDrainTimeoutFromEnv`（`grpcclient_test.go:867-895`）断言绿（reframe add→verify，同 Phase 31 event-bus-partition 校正）；ADR-031 add-only Amendment 记 verify-only — verified by **TEST-33.3.5**（引证既有 `TestDrainTimeoutFromEnv` 绿）
- [ ] **AC4**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-33.3.6**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-33.3.1 | indexing replay mapper 纯函数：给定 `Vec<IndexingEventRow>`（id ASC）→ 重建 indexing.* `PbEvent`（`indexing.progress`/`.cancelled`/`.error` + 真实 job_id/processed/total + 确定性 `evt-idx-{id}` + id/ts ASC），mirror TEST-26.2.3 | `core/src/data_plane/events.rs`（单测 mod） | Planned |
| TEST-33.3.2 | indexing persist round-trip：emit 点写 0019_indexing_events 行 → store 读回 → mapper 重建，字段（job_id/stage/processed/total/ts）与写入一致；migration `IF NOT EXISTS` 幂等 | `core/src/jobs/index_session_backend.rs` + 新 `SqliteIndexingEventStore` + `core/migrations/0019_indexing_events.sql` | Planned |
| TEST-33.3.3 | TraceStore SQL workspace filter：空 workspace_id → get/list/search_fts 与改前 aggregate-all byte-equiv；非空 workspace_id=A → 只返回 workspace A trace（跨 workspace 隔离） | `core/src/data_plane/search_persist.rs` + `core/src/data_plane/search.rs`（in-mem TraceStore） | Planned |
| TEST-33.3.4 | handler workspace 透传：`GetSearchTraceRequest`/`ListQueriesRequest` add-only `workspace_id=2` → handler 读并透传 store；既有 client 不传（空）保 aggregate-all 旧路径 | `core/src/data_plane/search.rs:460-502` + `proto/.../console_data_plane.proto:237-239/:255-257`（buf generate binding） | Planned |
| TEST-33.3.5 | drain-timeout VERIFY-ONLY：引证既有 `TestDrainTimeoutFromEnv`（default 100ms / env override / 非法回落）绿，本 task 0 改动（Phase 26 已交付） | `internal/consoleapi/grpcclient/grpcclient_test.go:867-895`（既有） | Planned |
| TEST-33.3.6 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（中）indexing 持久写不得阻断 indexing 主流程**：在三个 emit 点加持久写，若同步写 SQLite 失败/阻塞会拖慢或中断 indexing。
  - **缓解**：持久写 best-effort（与既有 `eb.send` best-effort `:158` 同语义——写失败 log 但不 `?` 上抛中断主流程）；写成功才作 replay 源（写失败 → 该行不可 replay，与「无订阅者时事件丢失」同级 trade-off，如实 doc）。stop-condition：persist round-trip 单测（TEST-33.3.2）不过 / indexing 主流程被持久写中断则 AC1 不标 `[x]`。
- **R2（中）empty workspace_id 必须严格 aggregate-all（默认行为回归）**：workspace filter 若把空串也当谓词（`WHERE workspace_id = ''`）会破既有 aggregate-all（多数既有行 workspace_id 非空 → 空查询返回 0 行）。
  - **缓解**：store 层显式 `if workspace_id.is_empty()` 分支——空 → 不加谓词（既有 SQL byte-equiv）、非空 → 加 `WHERE/AND workspace_id = ?`；TEST-33.3.3 断言空 workspace_id 结果与改前完全一致。stop-condition：空 workspace_id byte-equiv 单测不过则 AC2 不标 `[x]`。
- **R3（低）proto add-only field 不破既有 client**：`GetSearchTraceRequest`/`ListQueriesRequest` 加 `workspace_id` 须 add-only（既有 tag 不动），否则破既有 console client。
  - **缓解**：用下一可用 tag `=2`（两 message 现各仅 tag 1）；既有字段 1 tag/类型不动（ADR-016 add-only）；buf generate 重生 binding，既有 client 不传 → 空 → aggregate-all（ADR-004）。stop-condition：add-only field + 既有 client 不破 + 空=aggregate-all 方标 AC2。
- **R4（低）indexing replay 字段须真实非合成**：mapper 若把 processed/total/job_id 填默认/合成值会伪造 lifecycle（ADR-013）。
  - **缓解**：mapper 输入 `IndexingEventRow`（持久行）真实字段直传 `PbEvent`，无合成；TEST-33.3.1 断言重建字段 == 持久行字段。stop-condition：mapper 合成字段则违 ADR-013，AC1 不标 `[x]`。
- **R5（中→🟡）e2e restart-replay + e2e console 隔离 CI 默认不闭环**：indexing restart-then-replay 须 live daemon + job runner、console 多-workspace 隔离须跨进程双 workspace，CI 默认不闭环。
  - **缓解**：本 task 以「持久源 0019 + mapper 纯函数 🟢 + SQL/handler workspace filter 🟢」满足 AC1/AC2 code-local 维度；e2e 维度 🟡 `[SPEC-DEFER:phase-future.indexing-replay-e2e]` / `[SPEC-DEFER:phase-future.tracestore-multi-workspace-strict]` 据真实跑出回填，不伪造数值（ADR-013）。stop-condition：e2e 维度数值未真实跑出则该维度不标达成、仅记 honest-defer。

## 9. Verification Plan

```bash
# 1. AC1 — indexing replay mapper 纯函数 + persist round-trip
cargo test -p contextforge-core data_plane::events::
cargo test -p contextforge-core jobs::index_session_backend

# 2. AC2 — TraceStore workspace filter（空=aggregate-all / 非空=隔离）+ handler 透传
cargo test -p contextforge-core data_plane::search_persist
cargo test -p contextforge-core data_plane::search

# 3. AC3 — drain-timeout VERIFY-ONLY（引证既有测试，0 改动）
go test ./internal/consoleapi/grpcclient/ -run TestDrainTimeoutFromEnv

# 4. 不退化（全量）
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
go test ./...

# 5. proto add-only field 重生 binding（buf generate）
buf generate

# 6. AC4 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界**：本 task 交付 indexing 持久源 `0019_indexing_events` + replay mapper 纯函数（🟢）+ TraceStore add-only proto field + SQL WHERE workspace filter + handler 透传（🟢，empty=aggregate-all byte-equiv）+ drain-timeout verify-only 引证既有测试（🟢）；端到端 indexing restart-then-replay `[SPEC-DEFER:phase-future.indexing-replay-e2e]`（须 live daemon + job runner）、e2e console 多-workspace 隔离 `[SPEC-DEFER:phase-future.tracestore-multi-workspace-strict]`（须跨进程双 workspace）均 🟡 不在本 task 闭环；据 ADR-013 不预填真实数值（真实跑出后回填）。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Draft

**§9 Verification 计划** (will record real evidence at impl)：
- AC1：`cargo test -p contextforge-core data_plane::events:: / jobs::index_session_backend` —— indexing replay mapper 纯函数（id/ts ASC 重建 indexing.* PbEvent，真实 job_id/processed/total，确定性 `evt-idx-{id}`，mirror TEST-26.2.3）+ persist round-trip（emit → 0019 行 → 读回 → mapper 重建一致；migration `IF NOT EXISTS` 幂等）；端到端 restart-replay 🟡 `[SPEC-DEFER:phase-future.indexing-replay-e2e]`（真实测试结果待实施回填，ADR-013 不伪造）。
- AC2：`cargo test -p contextforge-core data_plane::search_persist / data_plane::search` —— add-only proto `workspace_id=2`（buf generate）+ get/list/search_fts + in-mem TraceStore WHERE workspace filter + handler 透传；空 workspace_id → aggregate-all byte-equiv（既有行为不变）/ 非空 → workspace 隔离；e2e console 隔离 🟡 `[SPEC-DEFER:phase-future.tracestore-multi-workspace-strict]`。真实结果待实施回填。
- AC3：`go test ./internal/consoleapi/grpcclient/ -run TestDrainTimeoutFromEnv` —— VERIFY-ONLY（Phase 26 已交付 `drainTimeoutFromEnv` `grpcclient.go:407-419`，0 代码改动，引证既有测试绿）；ADR-031 add-only Amendment 记 verify-only 校正。真实结果待实施回填。
- AC4：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威）。
- 0 新 dep / 默认行为不变（empty=aggregate-all byte-equiv）/ 既有契约不变（proto add-only field、migration 0019 add-only）/ honest（真实 job_id/processed/total 非合成、e2e 维度据实 defer）真实结果待实施回填（ADR-013 受阻 / 数值不预填，真实跑出才记数）。

**实际改动文件**（计划，待实施回填）：
- `core/migrations/0019_indexing_events.sql`——新增专用表 `indexing_events`（`job_id`/`stage`/`processed`/`total`/`ts_unix`，CREATE TABLE IF NOT EXISTS + index），`include_str!` 编译入二进制（镜像 0015 pattern）。
- `core/src/jobs/index_session_backend.rs`——三 emit 点（`:157-168`/`:182-193`/`:210-218`）就地额外 best-effort 持久写 indexing lifecycle 行 + 注入持久 sink（add-only 字段 / Option，既有构造兼容）。
- `core/src/data_plane/events.rs`——新 `indexing_rows_to_pb_events`（或 `replay_indexing_events_from_store`）纯 mapper（id/ts ASC 重建 indexing.* PbEvent，真实字段 + 确定性 `evt-idx-{id}`）；`:389` marker 措辞更新（持久源已落地 0019，e2e `[SPEC-DEFER:phase-future.indexing-replay-e2e]`）+ 同源单测（mirror TEST-26.2.3）。
- `proto/contextforge/console_data_plane/v1/console_data_plane.proto`——`GetSearchTraceRequest`（`:237-239`）+ `ListQueriesRequest`（`:255-257`）add-only `string workspace_id = 2`（buf generate 重生 Go/Rust binding）。
- `core/src/data_plane/search_persist.rs`——`get`/`list`/`search_fts` 加 `workspace_id` 形参 + `WHERE workspace_id` filter（空=不加谓词 aggregate-all byte-equiv）。
- `core/src/data_plane/search.rs`——in-mem `TraceStore` get/list 加 workspace 谓词 + handler `get_search_trace`（`:460-480`）/ `list_queries`（`:486-502`）从 request 读并透传 workspace_id。
- `internal/consoleapi/grpcclient/grpcclient.go`——**0 改动**（drain-timeout verify-only，引证既有 `TestDrainTimeoutFromEnv` `grpcclient_test.go:867-895`）。
- ADR-031 add-only Amendment（indexing replay 持久源 + drain-timeout verify-only 校正）+ ADR-016 trace isolation proto add-only field 引用——落点在 ADR-038 D3 / task-33.4 closeout（非本 task body）。
