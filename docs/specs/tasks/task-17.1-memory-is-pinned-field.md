# Task `17.1`: `memory-is-pinned-field — proto MemoryItem.is_pinned + migration 0017_memory_items_add_is_pinned.sql + SqliteMemoryStore.set_pinned + Go contractv1.MemoryItem.IsPinned + memstore fallback + smoke v8`

**Status**: Done

**Priority**: P2
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 17 (is-pinned-amendment)
**Dependencies**: Phase 13 task-13.1 (`SqliteMemoryStore` 既有结构 + Pin RPC handler) / Phase 13 task-13.2 (`MemMemoryStore` fallback + REST handler) / Phase 15 task-15.2 ([ADR-021](../../decisions/adr-021-memory-event-bus-bridge.md) `emit_audit_and_event` 共享路径) / [ADR-022](../../decisions/adr-022-memory-is-pinned-field-amendment.md) D1-D5 主决策依据 + **cross-repo signal**: Console 主仓 PR ship `internal/contractv1/contractv1.go::MemoryItem.IsPinned` merged

**Pending → Ready trigger** (resolved 2026-05-28): Console master @ `415ee30fcd8effd7929806d196458ec6e60fb49f` (PR [tajiaoyezi/ContextForge-Console#101](https://github.com/tajiaoyezi/ContextForge-Console/pull/101), merged 2026-05-28T12:16:57Z) ships `MemoryItem.IsPinned bool` add-only per ADR-022 D1. ContextForge main agent verified Console master HEAD contains the field via gh API + base64 fetch → Status flipped `Pending → Ready → Done` within this implementation PR.

**Spec drift discovered during recon (2026-05-28)** — captured here so future readers don't follow the original §3 verbatim:

- **Migration 0017 unnecessary**: `core/migrations/0013_memory_items.sql` already declares `is_pinned INTEGER NOT NULL DEFAULT 0` (line 16, forward-added by task-13.1 with the comment "9 columns 1:1 mirror contractv1.MemoryItem + orthogonal is_pinned flag"). Creating migration 0017 would error with `duplicate column name` on existing v0.6+ DBs. **Action**: skip migration 0017 + PRAGMA gate prescribed in original §3; document tautology in §6 AC4 verification.
- **Rust SqliteMemoryStore already complete**: `set_pinned(memory_id, pin)`, `MemoryItem.is_pinned: bool`, `SELECT ... is_pinned` in `list/get`, `MemoryServer.Pin` calling `set_pinned` — all from task-13.1 (Phase 13 ship). Only `memory_to_pb` mapper (lines 140-152) needed `is_pinned: m.is_pinned` to propagate the column onto the proto wire.
- **Real residual work**: (a) proto `bool is_pinned = 10`, (b) `memory_to_pb` mapper, (c) Go `contractv1.MemoryItem.IsPinned`, (d) `grpcclient.protoToMemoryItem` copy, (e) `MemMemoryStore` track + fixture preset + (f) `handleMemoryPin` JSON body parsing (which previously hardcoded `Pin(id, true)` and discarded body — task-17.1 spec §3 missed this gap), (g) tests + smoke v8.

## 1. Background

ContextForge-Console PR #91/#93 backlog 最后剩余项（11/11 中最后 1 项）：

> **P2 #6 — `MemoryItem.is_pinned` 字段缺失**：Console UI Memory 列表 / 详情面板期望按 `is_pinned` 排序（pinned 项排前）+ 显示 pin 状态图标；当前 schema 没有 `is_pinned` 字段；Console UI 只能通过查 `MemoryOperation.op_type=pin` 历史推断（unpin 后历史仍有 pin 记录，逻辑脆弱）。

既有 v0.9.0 状态：
- `internal/contractv1/contractv1.go:194-207` `MemoryItem` 10 字段（含 `Status` / `Availability`），**不含** `is_pinned`
- `core/proto/console_data_plane.proto` MemoryItem message 字段集合与 Go 一致（不含 `is_pinned`）
- SQLite `memory_items` 表（migration 0013）不含 `is_pinned` 列
- `MemoryService.Pin` RPC handler（task-13.1）调 `emit_audit` 写 audit_log + ADR-021 D1 emit EventBus.send(`memory.pin/unpin`) — **但 pin 状态本身无持久化**
- 既有 spec 显式标 `[SPEC-OWNER:phase-17.is-pinned-amendment]` (Phase 16 spec) / `[SPEC-DEFER:phase-16.memoryitem-is-pinned]` (v0.8 ship)

**实施策略**（ADR-022 D1-D5 落地）：

- 新增 SQLite migration `core/migrations/0017_memory_items_add_is_pinned.sql`（PRAGMA `table_info` 预检 + `ALTER TABLE memory_items ADD COLUMN is_pinned INTEGER NOT NULL DEFAULT 0`；幂等）
- 修改 proto `MemoryItem` add `bool is_pinned = N`（序号在实施时 grep 当前最大 + 1）
- 修改 `SqliteMemoryStore.list` / `SqliteMemoryStore.get_by_id` SELECT 加 `is_pinned` 列；新方法 `set_pinned(memory_id, pin) -> Result<()>` 写入
- 修改 `MemoryServer.Pin` RPC handler 在 `emit_audit_and_event` 路径前 / 后调 `store.set_pinned(memory_id, req.pin)` （写穿）
- 修改 Go `internal/contractv1/contractv1.go::MemoryItem` 加 `IsPinned bool` 字段（JSON tag `is_pinned`）
- 修改 `MemMemoryStore` fallback 加 `is_pinned map[string]bool`；Pin / Get / List 同步更新
- 修改 `scripts/console_smoke.sh` v7 27-step → v8 28-step；加 step 28 Pin RPC roundtrip + restart 验证
- ADR-014 D2 lint：本 task spec 的延后行为关键词全部用 [SPEC-DEFER:&lt;name&gt;] 或 [SPEC-OWNER:&lt;task&gt;] 标注（详 §Out of Scope）

## 2. Goal

新增 SQLite migration `0017_memory_items_add_is_pinned.sql` 自动应用；proto MemoryItem add-only `is_pinned` 字段；`MemoryService.Pin` 调用同步写穿 SQLite `is_pinned` 列；`GET /v1/memory/{id}` 和 `GET /v1/memory` 返字段；daemon 重启后 `is_pinned` 仍正确（SQLite 持久化生效）；既有 `cargo test --workspace` + `go test ./...` + 22-endpoint conformance 不退化；≥3 Rust + ≥2 Go unit test + ≥1 migration integration test PASS；ADR-022 Status Proposed → Accepted 在 closeout PR 内同步。

## 3. Scope

### In Scope

- **新建 `core/migrations/0017_memory_items_add_is_pinned.sql`**：
  ```sql
  -- Phase 17 task-17.1 — MemoryItem.is_pinned add-only field (ADR-022 D1).
  -- SQLite 不支持 ALTER TABLE ... ADD COLUMN IF NOT EXISTS；必须 PRAGMA 预检幂等。
  -- 实施层用 SqliteMemoryStore::open 内 PRAGMA table_info 检 + 条件 ALTER。
  -- 本文件作为 reference SQL（实际通过 Rust 代码动态执行 PRAGMA + ALTER）：
  ALTER TABLE memory_items ADD COLUMN is_pinned INTEGER NOT NULL DEFAULT 0;
  ```
  注：因 SQLite 不支持 IF NOT EXISTS for ADD COLUMN，实施时在 `SqliteMemoryStore::open` 内：
  ```rust
  let has_col = conn.query_row(
      "SELECT 1 FROM pragma_table_info('memory_items') WHERE name = 'is_pinned'",
      [],
      |r| r.get::<_, i32>(0),
  ).optional()?.is_some();
  if !has_col {
      conn.execute("ALTER TABLE memory_items ADD COLUMN is_pinned INTEGER NOT NULL DEFAULT 0", [])?;
  }
  ```

- **修改 `core/proto/console_data_plane.proto::MemoryItem`**：add-only 字段
  ```proto
  message MemoryItem {
    string memory_id = 1;
    string agent_scope = 2;
    string content_preview = 3;
    string source_type = 4;
    string source_ref = 5;
    int64 created_at_unix = 6;
    int64 updated_at_unix = 7;
    int64 hit_count = 8;
    string status = 9;
    bool is_pinned = 10;  // ADR-022 D1 — 序号在实施时 grep proto 当前最大 + 1 (示例标 10，实际按 grep 决定)
    // 既有 field_availability 等字段保留
  }
  ```

- **修改 `core/src/memory/store.rs::SqliteMemoryStore`**（≥3 unit test）：
  - `open(data_dir)` 内加 PRAGMA 预检 + 条件 ALTER（幂等）
  - `list(filter) -> Vec<MemoryItem>` SELECT SQL 加 `is_pinned` 列
  - `get_by_id(id) -> Option<MemoryItem>` SELECT SQL 加 `is_pinned` 列
  - 新方法 `set_pinned(memory_id: &str, pin: bool) -> Result<()>`:
    ```rust
    pub fn set_pinned(&self, memory_id: &str, pin: bool) -> Result<(), MemoryStoreError> {
        let conn = self.conn.lock()?;
        conn.execute(
            "UPDATE memory_items SET is_pinned = ?1, updated_at = ?2 WHERE memory_id = ?3",
            params![pin as i32, now_iso(), memory_id],
        )?;
        Ok(())
    }
    ```
  - 新增 unit test：`test_sqlite_set_pinned_true_persists_get` / `test_sqlite_set_pinned_false_reverses` / `test_list_returns_is_pinned_column`

- **修改 `core/src/memory/types.rs` 或同源 `MemoryItem` struct**：加 `is_pinned: bool` 字段；prost-derive 自动覆盖 proto wire format

- **修改 `core/src/data_plane/memory.rs::MemoryServer.Pin`**：在 `emit_audit_and_event` 调用前先调 `store.set_pinned(memory_id, req.pin)`
  - 既有 audit + EventBus 路径（ADR-021 D1）不变
  - 写穿失败时 Pin RPC 返 Error（与 task-13.1 既有错误传播一致）

- **修改 `internal/contractv1/contractv1.go::MemoryItem`**：加字段
  ```go
  type MemoryItem struct {
      MemoryID       string            `json:"memory_id"`
      AgentScope     string            `json:"agent_scope"`
      ContentPreview string            `json:"content_preview"`
      SourceType     string            `json:"source_type"`
      SourceRef      string            `json:"source_ref"`
      CreatedAt      time.Time         `json:"created_at"`
      UpdatedAt      time.Time         `json:"updated_at"`
      HitCount       int               `json:"hit_count"`
      Status         string            `json:"status"`
      IsPinned       bool              `json:"is_pinned"`           // ADR-022 D1
      Availability   FieldAvailability `json:"field_availability"`
  }
  ```

- **修改 `internal/consoleapi/memstore.go::MemMemoryStore`**：
  - 加 `is_pinned map[string]bool`（默认 false）
  - `Pin(id, pin bool) error`：同步更新 map
  - `Get(id) (MemoryItem, error)` / `List(filter) ([]MemoryItem, error)`：返字段
  - `SeedFixtures()`：保留 ≥1 fixture `is_pinned: true` 作 UI 渲染验证
  - 新增 ≥2 unit test：`TestMemMemoryStore_Pin_TogglesIsPinned` + `TestMemMemoryStore_List_ReturnsIsPinned`

- **修改 `scripts/console_smoke.sh` v7 → v8**：27-step → 28-step；加 step 28
  ```bash
  echo "Step 28: Pin RPC roundtrip + restart verify is_pinned persists"
  # (a) POST pin true
  curl -X POST -H "X-Confirm: yes" http://localhost:48181/v1/memory/mem-1/pin -d '{"pin": true}'
  # (b) GET verify is_pinned=true
  curl -s http://localhost:48181/v1/memory/mem-1 | jq -e '.is_pinned == true'
  # (c) POST pin false
  curl -X POST -H "X-Confirm: yes" http://localhost:48181/v1/memory/mem-1/pin -d '{"pin": false}'
  # (d) GET verify is_pinned=false
  curl -s http://localhost:48181/v1/memory/mem-1 | jq -e '.is_pinned == false'
  # (e) kill daemon + restart + GET verify state preserved
  kill -9 $DAEMON_PID && sleep 2 && spawn_daemon && sleep 3
  curl -s http://localhost:48181/v1/memory/mem-1 | jq -e '.is_pinned == false'
  ```

- **修改 `scripts/release_smoke.sh`**：加 `phase17_is_pinned_amendment=ok` 子段（gated by 实测 `curl Memory.IsPinned`）

- **修改 `docs/decisions/adr-022-memory-is-pinned-field-amendment.md`**：closeout PR 内 Status `Proposed` → `Accepted (YYYY-MM-DD, via Phase 17 closeout PR)`

### Out of Scope（[SPEC-DEFER]）

- `pinned_at` timestamp 字段 [SPEC-DEFER:phase-future.memory-pinned-at-timestamp]
- `pin_actor` 字段 [SPEC-DEFER:phase-future.memory-pin-actor]
- 历史 audit log backfill `is_pinned` 当前态 [SPEC-DEFER:phase-future.is-pinned-backfill-from-audit]
- Memory `tags` / `priority` 字段 [SPEC-DEFER:phase-future.memory-tags] [SPEC-DEFER:phase-future.memory-priority]
- proto contract v1 → v2 bump（amendment 不构成 break；contract v1 维持）[SPEC-DEFER:phase-future.contract-v2]
- ADR-015 D5 amendment self-amendment 路径（recursive amendment 协议）[SPEC-DEFER:phase-future.adr-015-d5-amendment-self]
- Console UI 端"按 pin 排序"feature flag visual closure（cross-repo Console 主仓领域）

## 4. Actors

- **主 agent**：本 task 实施 + closeout PR 主理
- **Rust SoT**：`core/src/memory/store.rs` 持久化层 + `core/src/data_plane/memory.rs` Pin RPC handler 写穿
- **Go thin proxy**：`internal/contractv1/contractv1.go` 字段 + `internal/consoleapi/handlers.go` Pin handler 透传 + `internal/consoleapi/memstore.go` fallback
- **Console UI 客户端**：消费 `is_pinned` 字段渲染 pin 状态（cross-repo Console 主仓领域，本 task 外）

## 5. Behavior Contract

### 5.1 Required Reading

- [ADR-022](../../decisions/adr-022-memory-is-pinned-field-amendment.md) D1-D5（主决策依据）
- [ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md) D1 add-only + D5 字段冻结 amendment 路径首次激活
- [ADR-021](../../decisions/adr-021-memory-event-bus-bridge.md) D1（`emit_audit_and_event` 路径不破坏）
- `docs/specs/phases/phase-17-is-pinned-amendment.md`（本 task 父 phase）
- `docs/specs/tasks/task-13.1-rust-memory-grpc-service.md`（既有 SqliteMemoryStore 结构）
- `docs/specs/tasks/task-13.2-go-memory-rest-handlers.md`（既有 MemMemoryStore + REST handler）

### 5.2 Imports

- Rust: `rusqlite::{params, Connection, OptionalExtension}` (既有) / `crate::pb_console::MemoryItem as PbMemoryItem` (proto-generated) / `crate::memory::store::{MemoryStoreError, SqliteMemoryStore}`
- Go: `time` (既有 contractv1) / `encoding/json` (既有)
- Proto: `bool is_pinned = N` (序号实施时确定)

### 5.3 Function Signatures

- `SqliteMemoryStore::set_pinned(&self, memory_id: &str, pin: bool) -> Result<(), MemoryStoreError>` (新)
- `SqliteMemoryStore::list(&self, filter: ListFilter) -> Result<Vec<MemoryItem>, _>` (既有；SELECT 加 `is_pinned`)
- `SqliteMemoryStore::get_by_id(&self, id: &str) -> Result<Option<MemoryItem>, _>` (既有；SELECT 加 `is_pinned`)
- `MemMemoryStore.Pin(id string, pin bool) error` (既有；新增 is_pinned map 写入)
- `MemMemoryStore.Get(id string) (contractv1.MemoryItem, error)` (既有；返字段)
- `MemMemoryStore.List(filter ListFilter) ([]contractv1.MemoryItem, error)` (既有；返字段)

## 6. Acceptance Criteria

- [x] AC1: SqliteMemoryStore `set_pinned(memory_id, true)` 后 `get(memory_id)` 返 `is_pinned: true`；`set_pinned(memory_id, false)` 后返 `false` — **verified by `core/src/memory/store.rs::tests::test_set_pinned_persists` PASS (Phase 13 既有；本 task 不退化；covers both true 与 false toggle in 一 test)**
- [x] AC2: MemMemoryStore fallback (`CONSOLE_API_FALLBACK_INMEM=1` 显式 opt-in) Pin(id, true) 后 Get(id).IsPinned = true；Pin(id, false) 后 = false — **verified by `internal/consoleapi/memstore_test.go::TestMemMemoryStore_Pin_TogglesIsPinned` PASS (本 task 新增)**
- [x] AC3: SqliteMemoryStore.list / MemMemoryStore.List 返字段 — list 响应每项 MemoryItem 含 `is_pinned: bool` — **verified by `core/src/memory/store.rs::tests::test_list_returns_is_pinned_column` (本 task 新增) + `internal/consoleapi/memstore_test.go::TestMemMemoryStore_List_ReturnsIsPinned` (本 task 新增) PASS**
- [x] AC4: Migration 幂等 — **tautologically satisfied** — `is_pinned` 列已存在 `core/migrations/0013_memory_items.sql` (task-13.1 ship 时 forward-added, line 16: `is_pinned INTEGER NOT NULL DEFAULT 0`); fresh install + v0.6+ 升级用户 daemon 重启自动 OK; no migration 0017 needed (would conflict with existing column on real DBs). Spec drift documented at top of this file.
- [x] AC5: contractv1 forward/backward compat — Console v0.7-v0.9 (pre-amend) client 解析 v0.10 response 不破坏；v0.10+ client 解析 v0.9 response `is_pinned` 默认 `false` — **verified by `internal/contractv1/types_test.go::TestMemoryItemForwardBackwardCompat` PASS (本 task 新增；filename drift from spec — same package, same semantic)**
- [x] AC6: `Pin RPC` 端到端 — `POST /v1/memory/{id}/pin {"pin":true}` → 204 No Content + audit_log 记录 + EventBus emit `memory.pin` event + SqliteMemoryStore.is_pinned 列 = 1；`GET /v1/memory/{id}` 返 `is_pinned: true` — **verified by smoke v8 Step 28 (daemon-level full roundtrip 含 empty-body backward-compat path) bash -n PASS; runtime PASS gated REAL mode + sqlite3 (see PR body §10). Per-RPC propagation also verified by `core/tests/memory_integration.rs::test_is_pinned_propagates_via_grpc_list_and_get` + `::test_pin_rpc_unpin_reverses_state` PASS (本 task 新增)**. X-Confirm header dropped from smoke (POST /v1/memory/{id}/pin not destructive — confirmMiddleware not wired per router.go:42).
- [x] AC7: 既有 22-endpoint conformance 不退化（contract v1 不 bump；仅 MemoryItem 字段 add-only）；`cargo test --workspace` 全 PASS（含 5 既有 memory_integration tests + 2 新增 + lib `test_list_returns_is_pinned_column` 新增）；`go test ./...` 21 packages 全 PASS — **verified by 本 PR 跑 cargo + go + bash -n 实测 (see §10)**
- [x] AC8: ADR-014 D2 lint — `scripts/spec_drift_lint.sh --touched origin/master` 0 unannotated hits — **verified by 本 PR body D2 输出段**

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-17.1.1 | SqliteMemoryStore::set_pinned true+false toggle 持久化 | `core/src/memory/store.rs::tests::test_set_pinned_persists` (既有；不退化) | Done |
| TEST-17.1.2 | SqliteMemoryStore::set_pinned NotFound | `core/src/memory/store.rs::tests::test_set_pinned_not_found` (既有；不退化) | Done |
| TEST-17.1.3 | SqliteMemoryStore::list 返 is_pinned | `core/src/memory/store.rs::tests::test_list_returns_is_pinned_column` (本 task 新增) | Done |
| TEST-17.1.4 | MemMemoryStore.Pin 切换 IsPinned | `internal/consoleapi/memstore_test.go::TestMemMemoryStore_Pin_TogglesIsPinned` (本 task 新增) | Done |
| TEST-17.1.5 | MemMemoryStore.List 返 IsPinned | `internal/consoleapi/memstore_test.go::TestMemMemoryStore_List_ReturnsIsPinned` (本 task 新增) | Done |
| TEST-17.1.6 | Migration 幂等 + 升级 backfill | **tautologically satisfied** — 列已在 `core/migrations/0013_memory_items.sql:16` (task-13.1 ship); no migration 0017 needed | Done (no-op) |
| TEST-17.1.7 | contractv1 forward/backward compat | `internal/contractv1/types_test.go::TestMemoryItemForwardBackwardCompat` (本 task 新增；filename drift from spec — types_test.go, not contractv1_test.go) | Done |
| TEST-17.1.8 | gRPC wire propagates is_pinned via List + Get | `core/tests/memory_integration.rs::test_is_pinned_propagates_via_grpc_list_and_get` (本 task 新增) | Done |
| TEST-17.1.9 | Pin RPC pin=false reverses state | `core/tests/memory_integration.rs::test_pin_rpc_unpin_reverses_state` (本 task 新增) | Done |
| SCEN-17.1.1 | Pin RPC + GET + restart full roundtrip + empty-body backward compat | `scripts/console_smoke.sh` v8 Step 28 (本 task 新增) | Done (bash -n PASS; REAL runtime PASS gated MODE=real + sqlite3) |

## 8. Risks

- **SQLite ALTER TABLE ADD COLUMN 不支持 IF NOT EXISTS**：必须 PRAGMA `table_info` 预检；漏检会导致升级 daemon 第二次启动报 `duplicate column name` error → mitigated by 实施层条件判断（见 §3 In Scope 第 1 项）
- **proto 字段序号冲突**：与并行 PR 共用 MemoryItem message 时序号可能撞；实施时 grep proto 当前最大序号 + 1 → 冲突时通知主 agent 重新分配
- **cross-repo amend 顺序错配**：ADR-022 D4 约定 Console 先 ship 后 ContextForge 后 ship；反序也工作但 Console UI 临时显示"全部未 pin"fallback；非 P0 故障但 cross-repo coord 必须用户人工 bridge（ContextForge 不主动跨仓）
- **MemMemoryStore fallback 字段同步漏**：fallback 模式（`CONSOLE_API_FALLBACK_INMEM=1` 显式 opt-in；ADR-018 deny 默认下不触发）下 MemMemoryStore.Pin / Get / List 必须真同步 is_pinned 字段，否则 Console UI fallback 测试无法验证 pin 功能 → AC2 覆盖
- **既有 v0.9 daemon 升级风险**：用户 v0.9 → v0.10 daemon 重启后 `is_pinned` 列自动 ADD 且既有数据 backfill `false`；首次 Pin RPC 调用后正常写穿；不破坏既有 Pin RPC 行为
- **ADR-021 EventBus emit 路径不破坏**：task-15.2 ship 的 `emit_audit_and_event` 路径必须保留；本 task 仅在前 / 后追加 `store.set_pinned` 调用 [SPEC-OWNER:task-17.1]，不动 audit + EventBus 顺序

## 9. Verification Plan

```bash
# install
go mod download && cargo fetch

# lint (项目当前 Lint 槽位待填，按 §s2v-adapter)

# typecheck
go vet ./...
cargo check --workspace

# unit-test
go test ./internal/consoleapi/... -run 'TestMemMemoryStore_Pin_TogglesIsPinned|TestMemMemoryStore_List_ReturnsIsPinned'
go test ./internal/contractv1/... -run TestMemoryItemForwardBackwardCompat
cargo test -p contextforge-core --lib memory::store::tests::test_sqlite_set_pinned_true_persists_get
cargo test -p contextforge-core --lib memory::store::tests::test_sqlite_set_pinned_false_reverses
cargo test -p contextforge-core --lib memory::store::tests::test_list_returns_is_pinned_column

# integration
cargo test -p contextforge-core --test memory_persist_integration test_is_pinned_migration_applies_idempotently

# regression — 既有不退化
cargo test --workspace
go test ./...

# e2e smoke (v8)
bash scripts/console_smoke.sh
# expect: CONSOLE_REAL_SMOKE_EXIT=0 含 Step 28 Pin roundtrip + restart 验证

# release smoke
bash scripts/release_smoke.sh
# expect: PHASE_RELEASE_SMOKE_EXIT=0; phase17_is_pinned_amendment=ok 段

# ADR-014 D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
# expect: 0 unannotated hits

# manual: cross-repo client compatibility
# (a) Console v0.7-v0.9 client 解析 v0.10 response (含 is_pinned) → ignore unknown field
# (b) Console v0.10+ client 解析 v0.9 response (无 is_pinned) → default false
```

## 10. Completion Notes (s2v 6 项标准)

- [x] commit SHA / PR #: see PR body — branch `feat/task-17.1-memory-is-pinned-field`. Cross-repo trigger: Console master @ `415ee30fcd8effd7929806d196458ec6e60fb49f` (PR [tajiaoyezi/ContextForge-Console#101](https://github.com/tajiaoyezi/ContextForge-Console/pull/101), merged 2026-05-28).
- [x] cargo test --workspace 实测输出: 全 PASS — proto regen via `buf generate --template buf.gen.yaml proto` + build.rs auto-regen; tests include 5 既有 memory_integration tests + 2 新增 (`test_is_pinned_propagates_via_grpc_list_and_get`, `test_pin_rpc_unpin_reverses_state`) + lib `test_list_returns_is_pinned_column` 新增 + `test_set_pinned_persists` 既有.
- [x] go test ./... 实测输出: 21 packages 全 PASS — 含 `internal/consoleapi` (2 新增 `TestMemMemoryStore_Pin_TogglesIsPinned` + `TestMemMemoryStore_List_ReturnsIsPinned`) + `internal/contractv1` (新增 `TestMemoryItemForwardBackwardCompat` + extended `TestJSONRoundtrip MemoryItem_pinned` case) + `test/conformance` 22-endpoint 不退化.
- [x] smoke v8 bash -n PASS — step 28 体内有 4 子断言 (post-restart from step-26 verify pinned + explicit pin=false + explicit pin=true + empty-body backward compat); runtime PASS gated MODE=real + sqlite3 (CI-equivalent via release_smoke.sh `RELEASE_SMOKE_CONSOLE=1`).
- [x] ADR-014 D2 lint 0 unannotated hits — `bash scripts/spec_drift_lint.sh --touched origin/master` confirmed at impl commit + will re-run after this spec sync edit lands.
- [x] ADR-022 Status Proposed → Accepted: 2026-05-28 via Phase 17 closeout PR (this PR). ADR-022 header now reads `**Status**: Accepted (2026-05-28, via Phase 17 closeout PR — implementation shipped via PR #118 task-17.1; cross-repo trigger ContextForge-Console PR #101 master @ 415ee30)`.
