# Task `27.1`: `memory-pin-actor-and-timestamp — MemoryItem 加 add-only proto 字段 pinned_by（pin-actor）+ pinned_at_unix（pinned-at-timestamp）+ memory_items add-only migration + SqliteMemoryStore set_pinned 写穿 actor/timestamp + data_plane/memory.rs 投影 + proto-freeze guard 守护`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 27 (memory-ops-hardening)
**Dependencies**: task-13.2（5 memory REST handler + `MemoryServer` 5 RPC 已落地）/ task-13.1（`SqliteMemoryStore` + `memory_items` 表 migration 0013）/ task-17.1（`is_pinned` add-only 字段 + `set_pinned` toggle 已落地，ADR-022）/ ADR-022（§Trade-offs `pin_actor` 不引入 + `[SPEC-DEFER:phase-future.memory-pinned-at-timestamp]`）/ ADR-032（memory-ops-hardening，本 phase 新 Proposed）/ ADR-008（core-library-selection，依赖变更 add-only）/ ADR-013（禁伪造凭据）/ ADR-014 D1-D5（第十八次激活）

## 1. Background

Phase 13 task-13.1 落地 `core/src/memory/store.rs::SqliteMemoryStore`（table `memory_items`，migration `core/migrations/0013_memory_items.sql`）+ `core/src/data_plane/memory.rs::MemoryServer`（5 RPC thin proxy）。Phase 17 task-17.1 经 ADR-022 加 `is_pinned bool`（proto field 10，`proto/contextforge/console_data_plane/v1/console_data_plane.proto:293`）+ `SqliteMemoryStore::set_pinned`（`store.rs:153`：`UPDATE memory_items SET is_pinned=?, updated_at_unix=? WHERE memory_id=?`）。

ADR-022 §Trade-offs / Conscious limitations 显式记录两条刻意缩范围延后的项：

> - **不引入 `pinned_at` timestamp**：当前 ADR 仅 `bool`；如未来 UI 需"按 pin 时间排序"，留 `[SPEC-DEFER:phase-future.memory-pinned-at-timestamp]` amendment ADR-023+
> - **不引入 `pin_actor` 字段**：谁 pin 的留 `MemoryOperation.actor` audit log 查（不污染 `MemoryItem` schema）

但实际上 audit log 不记 actor——`core/src/data_plane/memory.rs:51` 的 `emit_audit_and_event` 构造的 `AuditEvent` 只填 `chunk_ids=[memory_id]` + `source="console-api"`，没有调用 actor；`set_pinned` 只 bump `updated_at_unix`（`store.rs:157`），任何非 pin 的 update（deprecate / soft_delete）都会覆盖它。因此「谁 pin 的 / 何时 pin 的」事实上无处可查。

本 task 让 `pinned_by`（pin-actor）+ `pinned_at_unix`（pinned-at-timestamp）成为 `MemoryItem` 的一手 add-only 字段：pin=true 时写穿调用 actor + 当前时间戳，pin=false 时归缺省。proto 改动 add-only（序号在既有 field 10 后追加、不动既有 tag），经 proto-freeze guard 守护（`core/tests/proto_contract.rs` FROZEN 契约：只增字段，不删 / 不改 tag）。

## 2. Goal

`MemoryItem`（proto + Rust struct + SQLite 列）新增两个 add-only 字段：`pinned_by`（pin-actor，`string` / `TEXT NOT NULL DEFAULT ''`）+ `pinned_at_unix`（pinned-at-timestamp，`int64` / `INTEGER NOT NULL DEFAULT 0`）。`proto/contextforge/console_data_plane/v1/console_data_plane.proto::MemoryItem` 加二字段，序号在既有最大 tag（10）后追加（不动既有 tag）。`memory_items` 表经 add-only migration 加二列（既有行缺省 backfill，同 `is_pinned` pattern）。`SqliteMemoryStore::set_pinned` 扩展为携带 actor（或新增 `set_pinned_with_actor`，§5.2 定）：pin=true 写 `pinned_by=actor` + `pinned_at_unix=now`；pin=false 归 `''` / `0`。`row_to_item` / SELECT 投影 / `seed_for_tests` 同步二字段；`core/src/data_plane/memory.rs::memory_to_pb` 投影二字段、`pin` RPC 把 actor（source）传入 store。≥3 Rust 测试（默认构建可跑）全 PASS：pin 写穿 actor+timestamp / unpin 归缺省 / round-trip get+list 投影一致。proto-freeze guard（`core/tests/proto_contract.rs`）过 + 既有字段不退化；`cargo test --workspace` + `go test ./...` 不退化。默认构建 0 新依赖、0 网络（ADR-004）。D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **修改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`**：`MemoryItem` message 加 add-only `string pinned_by = N` + `int64 pinned_at_unix = N+1`（N = 当前最大 tag 10 + 1，按 proto 文件实际最大序号确定）；既有 field 1-10 不动 tag、不删、不改类型。加 add-only 注释（ADR-032 D1 + ADR-022 marker 推进锚点）。
- **add-only SQLite migration**：`memory_items` 表加 `pinned_by TEXT NOT NULL DEFAULT ''` + `pinned_at_unix INTEGER NOT NULL DEFAULT 0`——按 migration 习惯在 `core/migrations/0013_memory_items.sql`（idempotent CREATE TABLE IF NOT EXISTS）追加列定义，或新增 `core/migrations/0016_memory_items_add_pin_actor.sql`（ALTER TABLE ADD COLUMN）；§5.2 据既有 migration 风格（0013 是单文件 include_str! 应用）确定落点，既有行缺省 backfill。
- **修改 `core/src/memory/store.rs`**：`MemoryItem` struct 加 `pinned_by: String` + `pinned_at_unix: i64`；`set_pinned` 扩展携带 actor（或新增 `set_pinned_with_actor(memory_id, pin, actor)`，§5.2 二选一）——pin=true 写 `pinned_by=actor` + `pinned_at_unix=now_unix()`、pin=false 归 `''` / `0`；`row_to_item` + List/Get 的 SELECT 投影 + `seed_for_tests` INSERT 同步二列。
- **修改 `core/src/data_plane/memory.rs`**：`memory_to_pb` 投影 `pinned_by` + `pinned_at_unix`；`pin` RPC 把 actor（当前 source 语义 `"console-api"`，§5.2 定 actor 来源）传给 store 的 actor-aware set_pinned。
- **新增同源 Rust 单测（`core/src/memory/store.rs` 内 `#[cfg(test)] mod tests` + `core/src/data_plane/memory.rs` 内）**：(a) `set_pinned(true, actor)` 后 get 返 `pinned_by=actor` + `pinned_at_unix>0`；(b) `set_pinned(false, ..)` 后 get 返 `pinned_by=''` + `pinned_at_unix=0`；(c) List/Get round-trip 投影 + `MemoryServer.pin` RPC 写穿 actor。
- **proto-freeze guard 复核**：`core/tests/proto_contract.rs` FROZEN 契约——`MemoryItem` 新字段为既有字段集合的 superset 追加（只增不删不改 tag）；若该测试用显式字段表则同步加 `pinned_by` / `pinned_at_unix` 到期望集合。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **Pin/Unpin RPC 显式拆分 + hard-delete** [SPEC-OWNER:task-27.2-memory-pin-unpin-split-and-hard-delete]：本 task 在既有 `Pin` toggle 上加 actor/timestamp 写穿，不拆分 RPC、不加 hard-delete。
- **is_pinned 从 audit log 回填** [SPEC-OWNER:task-27.3-closeout-v0.20.0]：本 task 落 actor/timestamp 字段，backfill 在收口 task。
- **真实 per-user actor 透传（console-api source 写死 `"console-api"` → 真实用户身份）** [SPEC-DEFER:phase-future.memory-actor-propagation]：本 task 落「actor 经 RPC 传入并写穿」能力 + 单测；真实用户身份从 console-api 上游链路透传属后续。
- **Console UI 渲染 pin 归属（详情面板显示「由谁 / 何时置顶」）** [SPEC-OWNER:task-27.3-closeout-v0.20.0]：本 task 落 wire 字段；UI 消费经 cross-repo 协调（ADR-022 D4 pattern）。
- **`MemoryItem.is_pinned` 字段本体** [SPEC-OWNER:task-17.1]：task-17.1 已落地，本 task 在其上加 actor/timestamp。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`proto/contextforge/console_data_plane/v1/console_data_plane.proto::MemoryItem`**：契约 message，本 task 加 add-only 二字段。
- **`core/src/memory/store.rs::SqliteMemoryStore`**：task-13.1 持久层，本 task 加 actor/timestamp 写穿。
- **`core/src/data_plane/memory.rs::MemoryServer`**：task-13.1 thin proxy，本 task 投影二字段 + pin RPC 传 actor。
- **`core/tests/proto_contract.rs`**：FROZEN proto 契约 guard，本 task 复核 add-only superset。
- **下游 task-27.2 / task-27.3**：27.2 加 Unpin/hard-delete RPC（proto 基线据本 task 稳定）；27.3 backfill + UI 消费 + closeout。

## 5. Behavior Contract

### 5.1 Required Reading

- `proto/contextforge/console_data_plane/v1/console_data_plane.proto:275-336`（`MemoryItem` field 1-10 含 `is_pinned=10` + `PinMemoryRequest{memory_id, bool pin}` + 5 RPC `MemoryService` + add-only 注释 pattern `:275-281`）
- `core/src/memory/store.rs`（`MemoryItem` struct `:20-32` + `set_pinned` `:153` + `set_status` `:170` + `row_to_item` `:215` + `list`/`get` SELECT 投影 `:97-149` + `seed_for_tests` `:190` + `now_unix` `:231`）
- `core/migrations/0013_memory_items.sql`（`is_pinned INTEGER NOT NULL DEFAULT 0` `:16` + idempotent CREATE TABLE IF NOT EXISTS + include_str! 应用方式）
- `core/src/data_plane/memory.rs`（`memory_to_pb` `:140` + `pin` RPC `:207` + `emit_audit_and_event` `:51` source `"console-api"` `:64`）
- `core/tests/proto_contract.rs`（FROZEN 契约 + `assert_superset` + `message_fields` add-only 验证 pattern + freeze 规则 `:78-90`）
- `docs/decisions/adr-022-memory-is-pinned-field-amendment.md`（§Trade-offs `pin_actor` 不引入 + `memory-pinned-at-timestamp` 延后 + D1 add-only 字段 pattern + D2 Pin RPC 写穿）+ `docs/decisions/adr-032-memory-ops-hardening.md`（D1）+ `docs/decisions/adr-008-core-library-selection.md`（依赖 add-only）+ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造）
- `internal/contractv1/contractv1.go:206-232`（Go `MemoryItem.IsPinned` + `MemoryOperation{OpType, Actor}` — actor 在 Go 契约已有先例）

### 5.2 关键设计 — actor/timestamp 写穿 + add-only proto + migration 落点

- **proto add-only 字段序号**：`MemoryItem` 既有最大 tag = 10（`is_pinned`）；新字段 `pinned_by = 11` + `pinned_at_unix = 12`（实施时按 proto 文件实际最大序号确认，不重号）。既有 field 1-10 tag / 类型 / 名不动（ADR-015 D5 冻结；proto-freeze guard 守护）。
- **migration 落点（二选一，§3 据习惯定）**：(A) 在 `0013_memory_items.sql` 的 CREATE TABLE IF NOT EXISTS 列定义里追加二列——因 0013 经 `include_str!` 在 `open` 时 execute_batch（`store.rs:86`），对全新 DB 直接生效；对既有 DB 需配合 `ALTER TABLE ADD COLUMN`（CREATE IF NOT EXISTS 不改既有表）。(B) 新增 `0016_memory_items_add_pin_actor.sql`（`ALTER TABLE memory_items ADD COLUMN pinned_by TEXT NOT NULL DEFAULT ''` + `pinned_at_unix`）并在 `open` 中追加 execute——对既有 DB 幂等加列。**实施时优先 (B)**（既有数据安全 add-only，ALTER ADD COLUMN with DEFAULT 不重写既有行），§10 回填实际落点。
- **set_pinned actor 语义（二选一）**：(A) `set_pinned(memory_id, pin)` 签名扩展为 `set_pinned(memory_id, pin, actor: &str)`——改既有调用点（`data_plane/memory.rs:217`）；(B) 新增 `set_pinned_with_actor(memory_id, pin, actor)`，`set_pinned` 转调它传默认 actor——保既有签名向后兼容。**实施时优先 (B)**（既有 `set_pinned` 单测不改、向后兼容），§10 回填。pin=true → `pinned_by=actor, pinned_at_unix=now_unix()`；pin=false → `pinned_by='', pinned_at_unix=0`（unpin 归缺省，语义：当前未 pin 则无归属）。
- **actor 来源**：本 task 落「actor 经 RPC / store API 传入并写穿」能力；console-api 当前 source 语义 `"console-api"`（`memory.rs:64`），故 `pin` RPC 传 `"console-api"` 作 actor（真实 per-user 身份透传 `[SPEC-DEFER:phase-future.memory-actor-propagation]`）。单测可显式传任意 actor 断言写穿。
- **ADR-013**：actor/timestamp 写穿 round-trip 是 deterministic 默认构建单测可验证项（🟢 默认构建真实往返）；不预判 UI 渲染。

### 5.3 不变量

- 默认构建 0 新依赖（actor=`String` / timestamp=`i64` 用既有 `rusqlite` / `serde`，0 网络 ADR-004）。
- proto add-only：既有 `MemoryItem` field 1-10 tag / 类型 / 名不变（proto-freeze guard FROZEN 契约守护）；新字段为 superset 追加。
- migration add-only：既有 `memory_items` 行经缺省 backfill（`pinned_by=''` / `pinned_at_unix=0`），既有数据零迁移风险（同 `is_pinned DEFAULT 0` pattern）。
- 既有 `Pin` / `Deprecate` / `SoftDelete` RPC 行为不变；既有 `set_pinned` / `set_status` 调用点语义不退化（若选 set_pinned_with_actor 方案则 `set_pinned` 签名不动）。
- pin=true 写 actor+timestamp、pin=false 归缺省（确定性，相同输入相同结果）。
- 不破坏既有 5 memory 单测（`store.rs` + `data_plane/memory.rs` 既有 test）。

## 6. Acceptance Criteria

- [x] **AC1**: `MemoryItem` 加 add-only `pinned_by`（string）+ `pinned_at_unix`（int64）proto 字段（序号在既有 field 10 后、不动既有 tag）；`core/tests/proto_contract.rs` FROZEN 契约断言新字段为 superset 追加（只增不删不改 tag），既有字段不退化 — verified by **TEST-27.1.1**
- [x] **AC2**: `SqliteMemoryStore` pin=true（actor-aware set_pinned）写穿 `pinned_by=actor` + `pinned_at_unix>0`；pin=false 归 `pinned_by=''` + `pinned_at_unix=0`；get + list round-trip 投影二字段一致 — verified by **TEST-27.1.2**
- [x] **AC3**: `MemoryServer.pin` RPC 把 actor（source `"console-api"`）写穿 store + `memory_to_pb` 投影二字段；add-only migration 既有行缺省 backfill（`pinned_by=''` / `pinned_at_unix=0`）不破坏既有 5 memory 单测 — verified by **TEST-27.1.3**
- [x] **AC4**: 既有不退化 — 默认 `cargo test --workspace`（0 新依赖）全 PASS + `go test ./...`（contractv1 / consoleapi 不退化）全 PASS；既有 `Pin`/`Deprecate`/`SoftDelete` 行为不变 — verified by **TEST-27.1.4** + §10 实测
- [x] **AC5**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-27.1.5** + §10 记录

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-27.1.1 | proto add-only `pinned_by`/`pinned_at_unix`（序号在既有 tag 后）+ FROZEN 契约 superset 追加不退化 | `proto/contextforge/console_data_plane/v1/console_data_plane.proto` + `core/tests/proto_contract.rs` | Done |
| TEST-27.1.2 | pin=true 写穿 actor+timestamp / pin=false 归缺省 / get+list round-trip 投影 / pinned_at 独立 updated_at | `core/src/memory/store.rs`（`mod tests`） | Done |
| TEST-27.1.3 | `MemoryServer.pin` RPC 写穿 actor + `memory_to_pb` 投影 + migration 缺省 backfill 不破坏既有单测 | `core/src/data_plane/memory.rs`（`mod tests`） | Done |
| TEST-27.1.4 | 默认 `cargo test --workspace` 0 failed + 0 新依赖 + `go test ./...` 不退化 | 全 Rust + Go | Done |
| TEST-27.1.5 | D2 lint `--touched origin/master` 0 未标注命中 | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（中）proto add-only 字段序号 / freeze guard 误触**（承 phase-27 §7 R1）：`MemoryItem` 已用到 field 10。
  - **缓解**：按 proto 实际最大序号 +1 分配（`pinned_by=11` / `pinned_at_unix=12`）；`core/tests/proto_contract.rs` FROZEN 契约断言 superset 追加；若该测试用显式字段集合则同步加新字段。stop-condition：freeze guard 不过则 AC1 不标 `[x]`（ADR-013 不伪造 add-only 通过）。
- **R2（中）`pinned_by` actor 来源真实性**（承 phase-27 §7 R2）：console-api source 写死 `"console-api"`（`memory.rs:64`），非真实 per-user 身份。
  - **缓解**：本 task 落「actor 经 RPC / API 传入并写穿」能力 + 单测显式传 actor 断言；真实 per-user 透传 `[SPEC-DEFER:phase-future.memory-actor-propagation]` 如实延后；AC2/AC3 以「actor 字段写穿 + 单测可断言」满足，actor 真实来源 caveat 在 §10 / ADR-032 §Consequences 记录。
- **R3（低）migration 落点对既有 DB 的安全性**：CREATE TABLE IF NOT EXISTS 不改既有表，须 ALTER ADD COLUMN 才能给既有 DB 加列。
  - **缓解**：优先新增 `0016` migration 用 `ALTER TABLE ADD COLUMN ... DEFAULT`（既有行不重写、缺省 backfill），在 `open` 中追加 execute；§10 回填实际落点 + 既有 DB 加列验证。
- **R4（低）`updated_at_unix` 与 `pinned_at_unix` 语义混淆**：既有 `set_pinned` bump `updated_at_unix`。
  - **缓解**：`pinned_at_unix` 独立记 pin 置真时刻、不被 deprecate/soft_delete 的 update 覆盖；`updated_at_unix` 保既有 bump 语义；单测断言二者独立（pin 后 deprecate → `pinned_at_unix` 不变、`updated_at_unix` 变）。

## 9. Verification Plan

```bash
# Rust：默认构建（0 新依赖）actor/timestamp 写穿 round-trip + proto-freeze guard
cargo test -p contextforge-core memory::store
cargo test -p contextforge-core data_plane::memory
cargo test -p contextforge-core --test proto_contract

# 默认构建不退化 + 0 新依赖
cargo test --workspace

# Go 契约 / console-api 不退化（本 PR 投影字段 add-only）
go test ./internal/contractv1/... ./internal/consoleapi/...
go test ./...

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **Status**: Done（2026-06-01）。
- **完成日期**：2026-06-01。
- **改动文件**：
  - `proto/contextforge/console_data_plane/v1/console_data_plane.proto`——`MemoryItem` add-only `string pinned_by = 11` + `int64 pinned_at_unix = 12`（既有 field 1-10 不动；`buf generate proto` 重生成 Go pb.go + Rust prost）。
  - `core/migrations/0017_memory_items_add_pin_actor.sql`（新增）——`ALTER TABLE memory_items ADD COLUMN pinned_by/pinned_at_unix DEFAULT`。
  - `core/src/memory/store.rs`——`MemoryItem` struct + 2 字段；`set_pinned_with_actor`（写穿/清空）；`set_pinned` 委托（向后兼容）；`ensure_pin_actor_columns`（PRAGMA 守护幂等 ALTER）；`row_to_item`/list/get SELECT/`seed_for_tests` 投影；3 同源测试。
  - `core/src/data_plane/memory.rs`——`pin` RPC 调 `set_pinned_with_actor(id, pin, "console-api")` + `memory_to_pb` 投影 + RPC actor 写穿/投影/清空测试。
  - `core/src/contract.rs`——`console_proto_dir`/`console_proto_text`/`console_message_fields`（strip 行注释稳健解析）/`console_service_methods`/`service_block`。
  - `core/tests/proto_contract.rs`——`test_27_1_memory_item_pin_actor_superset`（FROZEN superset guard）。
  - `internal/contractv1/contractv1.go`——`MemoryItem` + `PinnedBy`/`PinnedAtUnix`；`internal/consoleapi/grpcclient/grpcclient.go`——`protoToMemoryItem` 投影。
  - `core/tests/memory_integration.rs`——`mem()` helper 补字段。0 新依赖（rusqlite/serde）。
- **commit 列表（RED→GREEN）**：RED `test(memory): TEST-27.1.2 RED`（proto+regen + struct + migration + `set_pinned_with_actor` todo!() + 测试）→ GREEN `feat(memory): MemoryItem pin-actor + pinned-at-timestamp 写穿`（实现 + pin RPC actor + contract 助手 + freeze guard + Go 投影）。
- **§9 Verification 实测结果（ADR-013 真实非合成）**：`cargo test -p contextforge-core --lib memory::store` **13 passed**；`data_plane::memory` **12 passed**（含 RPC actor 写穿/投影/清空）；`--test proto_contract` **6 passed**（含 MemoryItem superset freeze）；`cargo test --workspace` 0 failed；`go test ./...` PASS（contractv1/consoleapi/grpcclient 不退化）。
- **设计取舍**：(1) **migration 落点 (B)**——新增 `0017`（0016 已被 task-26.1 FTS 占用）ALTER ADD COLUMN，因 ALTER 非幂等 → `open` 内 `ensure_pin_actor_columns` 经 `PRAGMA table_info` 守护（缺列才 ALTER），既有 + 全新 DB 均经此路径加列、缺省 backfill。(2) **set_pinned_with_actor + set_pinned 委托**（保既有签名向后兼容，task-17.1 单测不改）；pin=true 写 actor+now、pin=false 归 ''/0；`pinned_at_unix` 独立于 `updated_at_unix`（deprecate/soft_delete 不动 pin 字段，TEST-27.1.2b）。(3) **actor 来源**：console-api source `"console-api"`（pin RPC 传入并写穿）；真实 per-user 身份透传 `[SPEC-DEFER:phase-future.memory-actor-propagation]` 如实延后，单测可显式传任意 actor 断言。(4) proto-freeze guard：既有 `message_fields` 仅读 core `contextforge/v1`，故新增 `console_message_fields`（读 console_data_plane proto + strip 行注释，规避 `//` 注释/注释内 `;` 破坏 `;`-split）。
- **剩余风险 + 下游影响**：真实 per-user actor 透传 `[SPEC-DEFER:phase-future.memory-actor-propagation]`；Console UI 渲染 pin 归属经 cross-repo 协调；task-27.2 加 Unpin/HardDelete RPC（proto 基线据本 task 稳定）+ task-27.3 is_pinned audit backfill + closeout 衔接。
