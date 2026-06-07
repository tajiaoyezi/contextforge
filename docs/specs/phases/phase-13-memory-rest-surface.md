# Phase 13 · memory-rest-surface

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 是 v0.6.0 minor release 收口 phase — 把 [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D1 Wave 3 共 5 个 Memory endpoint 落地：
>
> - `GET /v1/memory?agent_id=&scope=&namespace=` → `[]MemoryItem`（list with filter）
> - `GET /v1/memory/{id}` → `MemoryItem`
> - `POST /v1/memory/{id}/pin` → 204（非破坏性，不走 confirmMiddleware）
> - `POST /v1/memory/{id}/deprecate` → 204（破坏性，走 confirmMiddleware；缺 X-Confirm 返 412）
> - `POST /v1/memory/{id}/soft-delete` → 204（破坏性，走 confirmMiddleware）
>
> 治理基线：本 phase 按 ADR-011 单驱动 + ADR-012 主 agent 自治 + **ADR-014 cross-validation gate（D1/D2/D3/D4/D5 第四次完整激活）**；§2A Ready review 由主 agent 自审。详见 [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) + [ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) + [ADR-014](../../decisions/adr-014-cross-phase-exit-criteria-validation.md)。

## 1. 阶段目标

实现 ContextForge 内部 MemoryItem 持久化 + Console Memory 5 endpoint REST surface。复用 [ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) D2 Rust 持 SoT + D3 Go thin proxy + D5 schema 单 owner = Rust pattern：

- **新增 SQLite migration**：`core/migrations/0013_memory_items.sql` 表 `memory_items` (memory_id PK / agent_scope / content_preview / source_type / source_ref / created_at / updated_at / hit_count / status enum {active, deprecated, soft_deleted})
- **新增 Rust `SqliteMemoryStore`**：`core/src/memory/` 模块 + CRUD + state ops (`mark_pinned` / `mark_deprecated` / `mark_soft_deleted`)
- **新增 proto MemoryService**：`proto/contextforge/console_data_plane/v1/memory.proto`（或 amend `console_data_plane.proto`）含 5 RPC × 5 message + 复用既存 MemoryItem message (Phase 11 task-11.1 ship 时 11 message 含 MemoryItem)
- **新增 Rust `MemoryServer`**：`core/src/data_plane/memory.rs` impl MemoryService trait + 接 SqliteMemoryStore；pin/deprecate/soft-delete 操作各 emit 一条 audit event 到既存 `core/src/memoryops/audit.rs` 的 `AuditSink`
- **新增 Go `grpcclient.MemoryClient`**：`internal/consoleapi/grpcclient/grpcclient.go` 加 MemoryClient wrapper（5 method）
- **新增 Go REST handlers**：`internal/consoleapi/handlers.go` 加 5 handler；router 注册 5 路由（deprecate + soft-delete 走 confirmMiddleware；list + get + pin 不走）
- **MemStore fallback**：`internal/consoleapi/memstore.go` MemoryAdapter 实现 List/Get 用 in-memory map（demo 模式下有意义）；pin/deprecate/soft-delete 返 `ErrDataPlaneUnavailable`（fallback 不写 audit log）

**重要 scope 决策（§3 in scope）**：本 phase **不实施 import-to-memory_items 写入路径** — Phase 3 既存 importers (Hermes / OpenClaw / agent-rules / `internal/memoryops/dedup`) 当前不写入 memory_items 表；Phase 13 仅建表 + 暴露 read/state ops + 测试 fixture 通过 SQL seed 或 CLI tool 注入。Importer 写入路径 [SPEC-DEFER:phase-15.import-to-memory-items] 留 v0.6.x。Console UI 在 v0.6.0 ship 后 list 端可能返空数组（fresh install）+ Console UI 端 graceful degrade 显示「No memory items yet; import via CLI」。

来源：[ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D1 Wave 3 / D2 X-Confirm / D6 沿用 ADR-016 / D7 ADR-014 第四次激活 / PRD §Implementation Phases v0.6 新增 / PRD §Open Questions O16 新增。

## 2. 业务价值

直接支撑 ContextForge PRD §Core Capabilities #3「MemoryOps 治理 — 避免 memory 从『增强 Agent』变成『污染 Agent』」的 UI 闭环：

- **Memory 治理 v0.1**：Console UI 端 Memory 列表面板 + pin/deprecate/soft-delete 操作 + audit log 反向可查（pin 高价值上下文 / deprecate 过时项 / soft-delete 污染项）—— PRD §Success Metrics 次指标「真实接入度 ≥ 20 条 memory/context 治理记录」的 UI 表层支撑
- **服务端 X-Confirm 412 兜底应用**：deprecate + soft-delete 5 endpoint 中 2 个走 confirmMiddleware（task-12.1 已 ship）= ADR-017 D2 deep defense 在 Memory 场景首次实战
- **memory_items 表 + SqliteMemoryStore 建立基础**：v0.6.x 起 importer 改造可直接写本表 [SPEC-DEFER:phase-15.import-to-memory-items]；Eval / Search / Audit 模块跨链路查询 memory 有了统一 store
- **Audit 反向可见**：所有 state ops 经 `core/src/memoryops/audit.rs::AuditSink::record` 持久化（既存 Phase 5 task-5.3 audit 框架；本 phase 不动 audit schema）

不在本 phase scope：
- Importer 写入 memory_items 路径 [SPEC-DEFER:phase-15.import-to-memory-items]
- Memory item create REST endpoint（Console 22 endpoint 不含 POST /v1/memory，只含 read + state ops）
- Memory item full text edit / version history [SPEC-DEFER:console-endpoint-expansion]
- Hard delete [SPEC-DEFER:phase-future.hard-delete-policy]（Console PRD 显式只支持 soft-delete + audit）

## 3. 涉及模块

- `core/migrations/0013_memory_items.sql`（新增：`memory_items` 表 schema + indexes on agent_scope / status / created_at）
- `core/src/memory/`（新增 module：`mod.rs` + `store.rs` SqliteMemoryStore CRUD + state ops + tests）
- `core/src/memory/store.rs`（新增：`pub struct SqliteMemoryStore { conn: Arc<Mutex<Connection>> }` + `create_table_if_missing` + `list(filter MemoryListFilter)` + `get(id)` + `mark_pinned(id)` + `mark_deprecated(id)` + `mark_soft_deleted(id)`）
- `core/src/data_plane/mod.rs`（修改：`DataPlaneStores` 持有 `Arc<SqliteMemoryStore>`；构造函数加 memory_store 参数）
- `core/src/data_plane/memory.rs`（新增：`MemoryServer` impl MemoryService trait + 5 RPC method + audit event 集成）
- `core/src/server.rs`（修改：`serve_full` 实例化 SqliteMemoryStore 加入 DataPlaneStores + 注册 MemoryServer service）
- `proto/contextforge/console_data_plane/v1/console_data_plane.proto`（修改：加 MemoryService 5 RPC + 5 new message 类型 + 复用既存 MemoryItem message）
- `internal/consoleapi/grpcclient/grpcclient.go`（修改：加 MemoryClient struct + 5 method wrapper）
- `internal/consoleapi/types.go`（修改：加 `MemoryClient` 接口 5 method 签名 + Deps 加 Memory 字段）
- `internal/consoleapi/router.go`（修改：注册 5 新路由 — deprecate + soft-delete 走 `confirmMiddleware`；list + get + pin 不走）
- `internal/consoleapi/handlers.go`（修改：加 5 handler — `handleListMemory` / `handleGetMemory` / `handleMemoryPin` / `handleMemoryDeprecate` / `handleMemorySoftDelete`）
- `internal/consoleapi/memstore.go`（修改：MemStore 加 MemoryAdapter — List/Get 用 in-memory map seed 5 个 fixture；pin/deprecate/soft-delete 返 ErrDataPlaneUnavailable）
- `core/tests/memory_integration.rs`（新增：5+ 集成测试 — `test_memory_crud_via_grpc` + `test_list_filter_by_scope_namespace` + `test_pin_idempotent` + `test_deprecate_emits_audit` + `test_soft_delete_emits_audit`）
- `internal/consoleapi/e2e_grpc_test.go`（修改：加 5+ sub-step — list / get / pin / deprecate-412 / deprecate-confirm / soft-delete-412 / soft-delete-confirm）
- `internal/consoleapi/handlers_test.go`（修改：加 5+ handler unit test）
- `internal/consoleapi/grpcclient/grpcclient_test.go`（修改：加 5+ Memory client wrapper unit test）
- `scripts/console_smoke.sh` v4（修改：15 endpoint flow → 20 endpoint flow；加 step 16-20 list memory / get memory / pin / deprecate w/ X-Confirm / soft-delete w/ X-Confirm）
- `scripts/release_smoke.sh`（不修改：第 5 段 phase11_console_real + 第 6 段 phase12 仍 ok 不退化）
- `docs/s2v-adapter.md`（修改：§Phases 加 Phase 13 行 / §Tasks 加 task-13.1/13.2 / §BDD console-contract-completion.feature 含 phase-13 scenarios）
- `docs/prds/context-forge.prd.md`（修改：§Implementation Phases 加 Phase 13 段 + §Open Questions O16 新增）
- `test/features/console-contract-completion.feature`（修改：加 phase-13 scenarios 5+）
- `test/fixtures/memory-seed/`（可选新增：5 个 fixture memory_items seed SQL 或 JSON for test）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 13.1 | core/migrations + core/src/memory + core/src/data_plane/memory.rs + proto MemoryService | `../tasks/task-13.1-rust-memory-grpc-service.md` |
| 13.2 | internal/consoleapi (router + handlers + grpcclient) + memstore MemoryAdapter | `../tasks/task-13.2-go-memory-rest-handlers.md` |

## 5. 依赖关系

- **依赖**：Phase 12（console-contract-completion）— 复用 task-12.1 `confirmMiddleware` Wave 1 + task-12.2/12.3 SearchService RPC add-only 演进模板；Phase 11（console-real-data-plane）— 复用 [ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) `DataPlaneStores` 共享 stores 链 + tonic Server::builder pattern + Go grpcclient 4 wrapper pattern；Phase 5（memoryops）— 复用 `core/src/memoryops/audit.rs::AuditSink` audit log 框架（不动 schema）
- **可并行**：否（v0.6 收口 phase）。Phase 内顺序：task-13.1（建表 + Rust MemoryService + audit hooks）→ task-13.2（Go REST + grpcclient + MemStore fallback + smoke v4）
- **Phase 内并行机会** [SPEC-OWNER:task-13.1,task-13.2]：task-13.1 + task-13.2 在 task-11.1/task-12.1 各自 pattern 已成熟下可部分并行（task-13.2 用 proto stub 先实施 Go REST handler + grpcclient interface）；但主 agent 单驱动 governance 偏好串行避免 stub/真接 diff 错位，本 phase 串行实施

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（任务 13.1-13.2 全 Done，实测验证；每条 AC 含 ADR-014 D3 verified by 显式 owner）**：

- [x] AC1：Rust 启动后 MemoryService gRPC 注册可用（5 RPC: List / Get / Pin / Deprecate / SoftDelete）；`memory_items` SQLite 表通过 0013_memory_items.sql migration 自动建立；SqliteMemoryStore CRUD + state ops 全工作 — **verified by task-13.1 §6 AC1/AC2 (9 store unit tests + 3 memory_integration via tonic client) PASS**
- [x] AC2：`GET /v1/memory?agent_id=&scope=&namespace=` 走 gRPC MemoryService.List + filter；查询参数任一组合工作；空结果返 200 + `[]` — **verified by task-13.2 §6 AC1 (`TestListMemory_ReturnsFixtures` + `TestListMemory_FilterByScope`) + smoke v4 Step 14 PASS**
- [x] AC3：`GET /v1/memory/{id}` 真返 MemoryItem 9 字段全填；不存在 → 404；`POST /v1/memory/{id}/pin` 选 `is_pinned bool` 列设计（status 三态独立）+ 返 204 — **verified by task-13.2 §6 AC2 (`TestGetMemory_404_when_missing` + `TestMemoryPin_204_no_body`) + smoke v4 Step 15/16 PASS**
- [x] AC4：`POST /v1/memory/{id}/deprecate` 缺 X-Confirm: yes / ?confirm=true → 412 PRECONDITION_FAILED；带任一 → 204 + status="deprecated" 持久化 + audit log entry op="memory_deprecate" 写入 — **verified by task-13.2 §6 AC3 (`TestMemoryDeprecate_{412,header,query}`) + task-13.1 `test_memory_server_deprecate_persists_and_emits_audit` PASS + smoke v4 Step 17 PASS**
- [x] AC5：`POST /v1/memory/{id}/soft-delete` 缺 X-Confirm → 412；带 → 204 + status="soft_deleted" + audit log entry op="memory_soft_delete" 写入；list endpoint 默认不返 soft_deleted 项 — **verified by task-13.2 §6 AC4 (`TestMemorySoftDelete_412_then_204_then_excluded`) + task-13.1 `test_memory_server_soft_delete_persists_and_emits_audit` PASS + smoke v4 Step 18 PASS**
- [x] AC6：ADR-014 cross-validation gate 全套通过：D2 lint 0 violation + D3 phase §6 每条 AC 含 verified by + D1 closeout PR body 含 mapping 表 + v0.5 既有 13 endpoint 不退化 — **verified by closeout PR body (this PR) + D2 targeted grep PASS + go test ./... 43 pkgs PASS + cargo test 84 lib + 3 memory_integration PASS**

**端到端 smoke**：

```bash
# step 1 — Phase 13 主集成 smoke (v4，含 20 endpoint flow)
bash scripts/console_smoke.sh
# 1) spawn contextforge-core daemon
# 2) spawn console-api-serve
# 3) curl 20 endpoint (v0.5 15 个 + v0.6 新 5 memory): 含 memory seed via inline SQL during smoke + list/get/pin/deprecate(X-Confirm)/soft-delete(X-Confirm)
# 4) CONSOLE_REAL_SMOKE_EXIT=0

# step 2 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master

# step 3 — Release smoke (v0.6.0 release prep)
bash scripts/release_smoke.sh
# PHASE_RELEASE_SMOKE_EXIT=0 不退化
```

step 1 是 task-13.2 Gate 3 入口；smoke v4 内含 memory seed SQL 注入测试 fixture（5 条 memory_items via `sqlite3` CLI 写入 data dir / 或 daemon `serve --seed-fixtures` flag [SPEC-DEFER:dev-mode-seed]）。

step 2 是 ADR-014 D2 lint gate。step 3 是 release_smoke.sh hand-off — v0.6 增量不破坏 v0.4/v0.5 既有。

## 7. 阶段级风险

- **关联 [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) §Rollback**：D1 SoT 反向（Go 写 memory_items）→ 立刻 STOP；D3 thin proxy 违反（Go 内 status 校验）→ STOP + 下推 Rust gRPC method
- **关联 task-13.1 SqliteMemoryStore concurrent access**：`Arc<Mutex<Connection>>` 设计与 task-10.2/10.3 一致；高并发 list QPS 时 Mutex 竞争可能成瓶颈 → §10 trade-off 评估 read replica vs single mutex；本 phase 单 Mutex 起步
- **关联 PRD §Technical Risks R4 + R5**（secret redaction + Agent schema 不稳）：本 phase content_preview 字段透传 store 既有值；不二次 redaction（依赖 importer 写入时已 redact）；importer 写入路径 [SPEC-DEFER:phase-15.import-to-memory-items] 故 v0.6.0 ship 时数据可能为空 → Console UI 端 graceful degrade 显示
- **关联 ADR-014 governance 第四次激活风险**：第三次 v0.5 已跑通；第四次再验证；D2 lint 词表稳定性 + D1 mapping 表格式 一致性 跟 v0.4/v0.5 比较
- **memory_items 表 schema 演进路径**：本 phase 0013 migration 一次性建立 9 列；如 Console v1.x 加 must-have 字段 → ADR-014 D5 历史不动 + amend migration 加 ALTER TABLE 或新 migration（add column nullable）；本 phase 不预判 v1.x scope

## 8. Phase Definition of Done

- [ ] 本 phase 全部 task spec Status=Done（13.1/13.2 全 Done — PR 顺序合）
- [ ] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过（console_smoke.sh 20 endpoint flow + spec_drift_lint.sh 0 violation + release_smoke.sh 不退化）
- [ ] 关联风险 ADR-017 §Rollback / R4 / R5 / ADR-014 治理风险缓解措施已落地
- [ ] adapter §Phase 状态索引该行 Status 同步更新（closeout PR）
- [ ] ADR-017 状态保持 Proposed（Phase 14 closeout 时才推 Accepted）
- [ ] PRD §Implementation Phases Phase 13 行新增（含 Status=Done / 描述 / 范围 / 依赖 / 可并行）+ §Open Questions O16 标记 partially resolved (REST 表面 ship; importer 写入路径留 v0.6.x [SPEC-DEFER:phase-15.import-to-memory-items])
- [ ] **ADR-014 D1 mapping 表**：closeout PR body 含 Phase §6 ↔ Task §6 AC 映射（AC1-6 每行 4 字段）
- [ ] **ADR-014 D2 lint 输出**：closeout PR body 含 `bash scripts/spec_drift_lint.sh --touched origin/master` 0 violation 输出
- [ ] v0.6.0 release tag prep ready（README + RELEASE_NOTES + evidence + artifacts 在 release PR 内）
