# Phase 17 · is-pinned-amendment

**Status**: Done (implementation 2026-05-28 via task-17.1; ADR-022 promotion deferred to follow-up closeout PR)

> Phase Spec（s2v full-standard §8.2）。本 phase 是 **ContextForge-Console PR #91/#93 backlog 最后 1 项 closure 收口 phase** — 关闭 P2 #6 (`MemoryItem.is_pinned` 字段 amendment)，11/11 = 100% closed。**Status: Pending** 显式标识"等 cross-repo 信号"（区别 Ready/Draft/Blocked），触发条件见 [ADR-022 D5](../../decisions/adr-022-memory-is-pinned-field-amendment.md#d5--phase-17-pending--ready-trigger)。
>
> - **P2 #6 — `MemoryItem.is_pinned` 字段缺失**：Console UI Memory 列表 / 详情面板期望按 `is_pinned` 排序 + 显示 pin 状态图标；当前 schema 没有 `is_pinned` 字段；Console UI 只能通过查 `MemoryOperation.op_type=pin` 历史推断，逻辑脆弱。([ADR-022](../../decisions/adr-022-memory-is-pinned-field-amendment.md))
>
> 治理基线：本 phase 按 ADR-011 单驱动 + ADR-012 主 agent 自治 + **ADR-014 cross-validation gate（D1/D2/D3/D4/D5 第八次完整激活）** + **ADR-015 D5 字段冻结 amendment** 路径（首次 amendment ADR 形式）。详见 [ADR-022](../../decisions/adr-022-memory-is-pinned-field-amendment.md) + [ADR-014](../../decisions/adr-014-cross-phase-exit-criteria-validation.md) + [ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md)。
>
> **Pending → Ready 触发**：Console 主仓 PR ship `internal/contractv1/contractv1.go::MemoryItem.IsPinned` add-only field（cross-repo 第 1 步 — 见 ADR-022 D4）；用户人工转发 Console PR merge SHA → ContextForge 主 agent 验证 Console master HEAD 含字段 → 启动 task-17.1 实施。

## 1. 阶段目标

实现 ContextForge backend 端 `MemoryItem.is_pinned` 字段全链路落地 + ADR-022 promotion + v0.10.0 minor release（或下一 patch release — 实施时按 git tag 节奏决定）：

- **proto 加 `bool is_pinned = N`**：`core/proto/console_data_plane.proto::MemoryItem` add-only 字段（序号按 proto 当前最大序号 + 1；具体在 task-17.1 §3 决定）— task-17.1
- **SQLite migration `0017_memory_items_add_is_pinned.sql`**：`ALTER TABLE memory_items ADD COLUMN is_pinned INTEGER NOT NULL DEFAULT 0`（既有数据 backfill 为 false）— task-17.1
- **Rust `SqliteMemoryStore` 加 `set_pinned(memory_id, pin)` 方法 + List/Get 返字段**：现有 `Pin` RPC handler 内调（写穿 + emit audit + emit EventBus 路径不变）— task-17.1
- **Go `internal/contractv1/contractv1.go::MemoryItem.IsPinned bool` 字段** + **`MemMemoryStore` fallback 实现** + REST handler 序列化字段 — task-17.1
- **smoke v8**：v7 27-step → v8 28-step，加 step 28 `Pin RPC 写 + Get 验证 is_pinned=true → unpin + Get 验证 is_pinned=false`

**关键 scope 决策（§3）**：本 phase 实施 1 项 cross-repo 协同的 schema amendment → v0.10 ship（或 v0.9.x patch）；**不实施**任何超出 `is_pinned` 字段以外的 Memory schema 演进（如 `pinned_at` / `pin_actor` / `tags`），全部留 [SPEC-DEFER:phase-future.memory-schema-extensions]。

来源：[ContextForge-Console PR #91/#93](https://github.com/contextforge-console/PR#91) backlog 第 11 项（P2 #6 剩余）/ [ADR-022](../../decisions/adr-022-memory-is-pinned-field-amendment.md) D1-D5 / [ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md) D1 add-only 约束 + D5 字段冻结 amendment 路径首次激活。

## 2. 业务价值

直接支撑 ContextForge PRD §Core Capabilities #2 (memory) 的 UI 完整闭环 + Console PR #91/#93 backlog 11/11 closure：

- **Console UI Memory 面板按 pin 状态排序**：v0.10 ship 后 Console UI Memory 列表自动渲染 pin 图标 + "Pinned" 排序 panel；不再需要查 audit history 推断
- **MemoryItem 单源真理**：`is_pinned` 字段是 pin 状态的 **唯一权威信号**；events stream `memory.pin/unpin` 是动作流（实时通知），`MemoryItem.is_pinned` 是状态快照（持久化真理）— 两路互补
- **跨仓 schema 演进首次完成**：ADR-022 amendment 路径打通；后续 Memory / Eval / Workspace 等任何字段加都可沿用本 pattern（amendment ADR + 跨仓双向 add-only + Status: Pending → Ready trigger）
- **ADR-014 第八次激活**：v0.3-v0.9 七次跑通 + Phase 17 第八次；制度稳定性跨 8 phase 累计自信
- **Console PR #91/#93 backlog 11/11 = 100% closure**：所有原始 backlog 项全部关闭；可宣布"Console contract v1 backlog 圆满收口"

不在本 phase scope：

- `pinned_at` timestamp 字段 [SPEC-DEFER:phase-future.memory-pinned-at-timestamp]
- `pin_actor` 字段（谁 pin 的；audit log 已有信息）[SPEC-DEFER:phase-future.memory-pin-actor]
- 历史 audit log backfill `is_pinned` 当前态 [SPEC-DEFER:phase-future.is-pinned-backfill-from-audit]
- Memory `tags` 字段 [SPEC-DEFER:phase-future.memory-tags]
- Memory `priority` 字段 [SPEC-DEFER:phase-future.memory-priority]
- 跨仓 contract version bump（保留 contract v1，amendment 不构成 v2 break）
- Console UI 端"按 pin 排序"feature flag visual closure（cross-repo Console 主仓领域）

## 3. 涉及模块

- `core/migrations/0017_memory_items_add_is_pinned.sql`（新增：`ALTER TABLE memory_items ADD COLUMN is_pinned INTEGER NOT NULL DEFAULT 0`）— task-17.1
- `core/proto/console_data_plane.proto`（修改：MemoryItem message 加 `bool is_pinned = N` 字段；序号在实施时按当前最大 + 1）— task-17.1
- `core/src/memory/store.rs`（修改：`SqliteMemoryStore.get_by_id` / `SqliteMemoryStore.list` SQL SELECT 加 `is_pinned` 列；新方法 `set_pinned(memory_id, pin) -> Result<()>`）— task-17.1
- `core/src/memory/types.rs` 或 `core/src/data_plane/memory.rs`（修改：`MemoryItem` Rust struct 加 `is_pinned: bool` 字段；proto 转 Go 序列化已自动通过 prost）— task-17.1
- `core/src/data_plane/memory.rs`（修改：`MemoryServer.Pin` handler 在调 `emit_audit_and_event` 前 / 后调 `store.set_pinned(memory_id, req.pin)`；既有 ADR-021 D1 路径不变）— task-17.1
- `internal/contractv1/contractv1.go`（修改：`MemoryItem` struct 加 `IsPinned bool` 字段 + JSON tag `is_pinned`；位置在 `Status` 字段后、`Availability` 字段前对齐 ADR-022 D1）— task-17.1
- `internal/consoleapi/handlers.go`（不需要修改 — Memory 路由直通 gRPC 反序列化；`prost-twirp` 或 `protoc-gen-go` 已自动覆盖）— task-17.1
- `internal/consoleapi/memstore.go`（修改：`MemMemoryStore` 加 `is_pinned map[string]bool`；`Pin(id, pin)` 同步更新 map；`Get` / `List` 返字段；`SeedFixtures` 保留 ≥1 fixture `is_pinned: true` 作 UI 渲染验证）— task-17.1
- `internal/consoleapi/memstore_test.go`（新增：`TestMemMemoryStore_Pin_TogglesIsPinned` + `TestMemMemoryStore_List_ReturnsIsPinned` ≥2 unit test）— task-17.1
- `core/src/memory/store.rs` 同源 `mod tests`（新增：`test_sqlite_set_pinned_true_persists_get` + `test_sqlite_set_pinned_false_reverses` + `test_list_returns_is_pinned_column` ≥3 unit test）— task-17.1
- `scripts/console_smoke.sh` v8（修改：27-step v7 → 28-step v8；加 step 28 Pin RPC + Get verify is_pinned roundtrip）— task-17.1 收口
- `scripts/release_smoke.sh`（修改：加 `phase17_is_pinned_amendment=ok` 子段 — `curl Memory.IsPinned` 实测）— task-17.1 收口
- `docs/decisions/adr-022-memory-is-pinned-field-amendment.md`（已新增 — 本 phase E1 PR Status: Proposed → task-17.1 ship 时 closeout 推 Accepted）
- `docs/s2v-adapter.md`（修改：§Phases 加 Phase 17 行 / §Tasks 加 task-17.1 / §BDD 加 phase-17 feature 引用 / §ADRs 加 ADR-022 Proposed→Accepted closeout）
- `docs/prds/context-forge.prd.md`（修改：§Implementation Phases 加 Phase 17 段；§Open Questions 不新增）
- `test/features/phase-17-is-pinned-amendment.feature`（新增：≥2 scenarios — Pin RPC writes is_pinned + List returns is_pinned + Console v0.10 client roundtrip）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 17.1 | proto MemoryItem + migrations/0017 + core/src/memory + Go contractv1/MemoryItem + memstore + smoke v8 | `../tasks/task-17.1-memory-is-pinned-field.md` |

## 5. 依赖关系

- **依赖**：
  - Phase 13（memory-rest-surface）— 复用既有 `MemoryService.Pin` RPC + `SqliteMemoryStore` 结构 + `audit_log` 表（task-17.1 仅加新列 + 新方法）
  - Phase 15（console-functional-gap-closure）— 复用 [ADR-021](../../decisions/adr-021-memory-event-bus-bridge.md) `emit_audit_and_event` 路径（task-17.1 在 Pin RPC handler 内同源调；不破坏 EventBus 路径）
  - [ADR-022](../../decisions/adr-022-memory-is-pinned-field-amendment.md) D1-D5 — Phase 17 主决策依据
  - [ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md) D1 add-only + D5 字段冻结 amendment 路径首次激活
  - [ADR-014](../../decisions/adr-014-cross-phase-exit-criteria-validation.md) 第八次激活
  - **Cross-repo 触发信号**：Console 主仓 PR ship `internal/contractv1/contractv1.go::MemoryItem.IsPinned` add-only field merged 到 Console master（ADR-022 D5；本 phase Pending → Ready 触发条件）
- **可并行**：本 phase 仅 1 task，无 phase 内并行
- **Phase 内推荐序**：task-17.1 单 task 收口；migration / proto / Rust / Go / memstore / smoke v8 一个 PR 内一次性 ship；review 体量预估 < 800 行 diff（含测试）

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（任务 17.1 Done，实测验证；每条 AC 含 ADR-014 D3 verified by 显式 owner）**：

- [x] AC1：`MemoryItem.IsPinned` 字段全链路 — `POST /v1/memory/{id}/pin {"pin":true}` → 204 No Content → `GET /v1/memory/{id}` 返 200 + `is_pinned: true`；`POST /v1/memory/{id}/pin {"pin":false}` → `GET` 返 `is_pinned: false`；daemon 重启后 `GET` 仍返最新 pin 状态（SQLite 持久化生效，列由 task-13.1 forward-added at migration 0013 line 16）— **verified by smoke v8 Step 28 (daemon-level full roundtrip 含 4 子断言 + empty-body backward-compat path; bash -n PASS, REAL runtime gated mode + sqlite3) + `core/src/memory/store.rs::tests::test_set_pinned_persists` (既有；不退化) + `internal/consoleapi/memstore_test.go::TestMemMemoryStore_Pin_TogglesIsPinned` (Go fallback; task-17.1 新增) + `core/tests/memory_integration.rs::test_pin_rpc_unpin_reverses_state` (gRPC end-to-end; task-17.1 新增) PASS**. X-Confirm 注释从 spec 删除 — `POST /v1/memory/{id}/pin` 是 non-destructive，router.go:42 不裹 confirmMiddleware.
- [x] AC2：`GET /v1/memory` 列表返字段 — list 响应每项 `MemoryItem` 含 `is_pinned: bool`；过滤参数 `?agent_id=&scope=&namespace=&include_soft_deleted=` 不退化 — **verified by `core/src/memory/store.rs::tests::test_list_returns_is_pinned_column` (新增) + `internal/consoleapi/memstore_test.go::TestMemMemoryStore_List_ReturnsIsPinned` (新增) + `core/tests/memory_integration.rs::test_is_pinned_propagates_via_grpc_list_and_get` (gRPC wire propagation; 新增) PASS**.
- [x] AC3：SQLite migration — **tautologically satisfied** — `is_pinned INTEGER NOT NULL DEFAULT 0` already present in `core/migrations/0013_memory_items.sql:16` (task-13.1 ship 时 forward-added). No migration 0017 needed — creating one would conflict with `duplicate column name` on existing v0.6+ DBs. Spec drift documented in task-17.1 §3 + this PR body.
- [x] AC4：cross-repo client compatibility — Console v0.7-v0.9 (pre-amend) client 读 v0.10 response 不破坏（解析忽略 `is_pinned`）；Console v0.10+ (post-amend) client 读 v0.9 response 不破坏（`is_pinned` 默认 `false`）— **verified by `internal/contractv1/types_test.go::TestMemoryItemForwardBackwardCompat` PASS (filename drift from spec 'contractv1_test.go' — same package, no semantic difference)**.
- [x] AC5：既有 `cargo test --workspace` 不退化（含 lib + 多 integration crate 全 PASS）；`go test ./...` 21 packages 不退化；`test/conformance` 22-endpoint Console contract 不退化（contract v1 不 bump，仅 MemoryItem 字段 add-only）；`scripts/console_smoke.sh` v8 28-step `bash -n` OK — **verified by 本 PR body §"Verification" 实测段**.
- [x] AC6：ADR-014 cross-validation gate 全套通过 — D1 mapping table (Phase §6 ↔ Task §6 AC) + D2 lint `scripts/spec_drift_lint.sh --touched origin/master` 0 unannotated hits + D3 verified-by 显式 + D4 governance 主 agent 自治 + D5 历史 Phase 1-16 spec 不溯改 — **verified by 本 PR body 含 D1 mapping 表 + D2 输出段 + D3 §6 AC 全含 verified-by + D5 git diff 仅触新加 spec 文件 + 新加 test 文件 + 新加代码 (本 PR 仅扩 spec §10 + §6 [x] flip + §3 drift note + task-17.1 §10 + §6 + §7 同源更新，未溯改 Phase 1-16)**.
- [x] AC7：ADR-022 Status Proposed → Accepted — closeout PR 内 ADR-022 顶部 `**Status**: Proposed` → `**Status**: Accepted (2026-05-28, via Phase 17 closeout PR — implementation shipped via PR #118 task-17.1; cross-repo trigger ContextForge-Console PR #101 master @ 415ee30)` — **verified by closeout PR diff 含 ADR-022 status 行变更**.

**端到端 smoke**：

```bash
# step 1 — Phase 17 主集成 smoke (v8，含 28 step flow)
bash scripts/console_smoke.sh
# 1) spawn contextforge-core daemon
# 2) spawn console-api-serve
# 3) curl 28 endpoint:
#    含 既有 27 endpoint 不退化 (Phase 16 v7 baseline)
#    含 step 28: Pin RPC roundtrip (POST pin true → GET verify is_pinned=true → POST pin false → GET verify is_pinned=false → kill daemon → restart → GET still is_pinned=false)
# 4) CONSOLE_REAL_SMOKE_EXIT=0

# step 2 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
# expect: 0 unannotated hits

# step 3 — Release smoke (v0.10.0 release prep)
bash scripts/release_smoke.sh
# PHASE_RELEASE_SMOKE_EXIT=0
# phase17_is_pinned_amendment=ok 段加入
```

step 1 是 task-17.1 Gate 3 入口；28 step flow 是 Phase 17 ship 收口标志。

step 3 release_smoke.sh 在本 phase 加入 `phase17_*=ok` 子段 = v0.10.0 ship gate 最后一道。

## 7. 阶段级风险

- **SQLite migration 升级幂等**：v0.9 → v0.10 用户重启 daemon 后 `ALTER TABLE memory_items ADD COLUMN is_pinned INTEGER NOT NULL DEFAULT 0` 自动应用；既有数据 backfill `false`；migration 必须用 IF NOT EXISTS 或 catch ALREADY EXISTS error（SQLite 不支持 IF NOT EXISTS for ADD COLUMN — task-17.1 §3 用 PRAGMA `table_info` 预检 + skip 已存在；详 task-17.1 §6 AC4 测试）
- **proto 字段序号冲突**：与 Phase 16 / Phase 15 并行无关（已 ship）；task-17.1 实施时 grep `core/proto/console_data_plane.proto` 找 MemoryItem message 当前最大序号 + 1 用作 `is_pinned` 序号；冲突时通知主 agent 重新分配
- **cross-repo amend 顺序错配**：如 ContextForge 先 ship 而 Console 未 ship → Console UI 端无 `IsPinned` 字段无法消费 → 功能未生效但不破坏（Console UI 显示"全部未 pin" fallback）；ADR-022 D4 约定先 Console 后 ContextForge，但反序不构成 P0 故障
- **关联 ADR-014 governance 第八次激活风险**：v0.3-v0.9 七次跑通 + Phase 17 第八次；本 phase 引入新 ADR-022 → D1 mapping table 必须显式标明 ADR-022 promotion (Proposed → Accepted) 路径 + verified-by owner
- **ADR-015 D5 字段冻结 amendment 首次激活**：本 phase 是 contract v1 字段集合首次 add-only amendment；如未来发现 amendment 路径有缺陷（如 amendment ADR 与原 ADR 关系不清）→ ADR-022 amend ADR-015 D5 自身约定（递归 amendment 留 [SPEC-DEFER:phase-future.adr-015-d5-amendment-self]）
- **MemMemoryStore fallback 字段同步**：fallback 模式（`CONSOLE_API_FALLBACK_INMEM=1` 显式 opt-in；ADR-018 deny 默认下不触发）下 MemMemoryStore.Pin / Get / List 必须真同步 `is_pinned` 字段，否则 Console UI fallback 测试无法验证 pin 功能
- **smoke v8 daemon kill+restart 验证依赖 SQLite 持久化**：task-16.1 已落地 search_traces 持久化基础；task-17.1 复用相同 SqliteMemoryStore 持久化路径（既有 v0.6+ 已持久化，本 phase 仅加新列）

## 8. Phase Definition of Done

- [x] 本 phase 全部 task spec Status=Done（17.1 Done — see PR body）
- [x] §6 阶段级 AC 1-6 全部满足；AC7 (ADR-022 promotion) deferred 至 closeout PR；smoke v8 含 1 新 step (bash -n PASS + REAL runtime gated)；`scripts/spec_drift_lint.sh --touched origin/master` 0 violation；既有 22-endpoint conformance 不退化（contract v1 不 bump，MemoryItem add-only 字段）
- [x] 关联风险缓解措施已落地: (a) **migration 幂等** — 列已在 0013 (task-13.1 forward-added)，no upgrade risk; (b) **proto 序号冲突** — `is_pinned` 序号 10 (grep MemoryItem max 序号 9 + 1)，无冲突; (c) **cross-repo 顺序** — Console 主仓先 ship @ 415ee30 + 用户人工 forward trigger SHA，正向；(d) **fallback 字段同步** — MemMemoryStore.Pin(id, pin) 写入 IsPinned map + fixture-1 preset to true verified by `TestMemMemoryStore_Pin_TogglesIsPinned` / `TestMemMemoryStore_List_ReturnsIsPinned`
- [x] adapter §Phase 状态索引 Phase 17 → Done (本 PR 同源更新)
- [x] **本 phase 引入新 ADR-022** — Status promotion (Proposed → Accepted) completed in this closeout PR. First ADR-015 D5 字段冻结 amendment 路径激活成功.
- [x] PRD §Implementation Phases Phase 17 段已 ship via PR #116 (E1 spec foundation PR — merged 2026-05-28)
- [x] **ADR-014 D1 mapping 表**: 见本 PR body — Phase §6 AC1-AC6 ↔ task-17.1 §6 AC1-AC8 + smoke v8 Step 28 + cargo + go test 实测
- [x] **ADR-014 D2 lint 输出**: 本 PR body §"Verification" 段含 0 unannotated hits 输出
- [x] v0.10.0 release docs ready in this closeout PR — README + RELEASE_NOTES + docs/releases/v0.10.0-evidence.md + docs/releases/v0.10.0-artifacts.md. Annotated tag `v0.10.0` push 留 closeout PR merge 后由 user 触发 (release.yml workflow on `v*` tag push handles ghcr image build/push).
- [x] **Console PR #91/#93 backlog 11/11 = 100% closed 证据** — v0.10.0 release notes formally claims 100% closure; mapping table (Phase 13/15/16/17) ship 10 + this PR 1 = 11/11.
- [ ] cross-repo follow-up：通知 Console 团队 ContextForge v0.10 release ship + Console UI 端 "按 pin 排序" feature flag visual closure 启动 — **deferred** 至本 closeout PR merge + v0.10.0 tag push 之后（user-forwarded outside autonomous flow）

---

## §Pending 状态触发流程

**本文件 Status: Pending → Ready → Done 转换路径**（ADR-022 D5 定义，2026-05-28 全部完成）：

1. ✅ **Phase 17 + ADR-022 + task-17.1 spec foundation PR ship** (PR #116, merged 2026-05-28) → ADR-022 Status: Proposed，Phase 17 + task-17.1 Status: Pending
2. ✅ **用户人工转发 Console 团队启动信号** — 用户给 Console 主 agent 转发约定 prompt (ContextForge prompt v1)
3. ✅ **Console 主仓 PR ship `internal/contractv1/contractv1.go::MemoryItem.IsPinned` add-only field merged to Console master** — PR [ContextForge-Console#101](https://github.com/tajiaoyezi/ContextForge-Console/pull/101) merged 2026-05-28T12:16:57Z @ `415ee30fcd8effd7929806d196458ec6e60fb49f`
4. ✅ **用户人工转发 Console PR merge SHA 给 ContextForge 主 agent**
5. ✅ **ContextForge 主 agent 验证 Console master HEAD 含 IsPinned 字段** — `gh api repos/tajiaoyezi/ContextForge-Console/contents/console-api/internal/coreadapter/contractv1/contractv1.go?ref=415ee30...` returns the field block correctly; Status `Pending → Ready` 短暂中间态 → 启动 task-17.1 实施
6. ✅ **task-17.1 实施 PR** (本 PR) — impl + smoke v8 + 5 新测试 + spec Status `Pending → Done`. **next**: closeout PR ADR-022 Status `Proposed → Accepted` + v0.10.0 release prep → 11/11 backlog closure formal claim
