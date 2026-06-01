# Task `27.2`: `memory-pin-unpin-split-and-hard-delete — 把 Pin toggle 拆成显式 Pin / Unpin RPC（add-only Unpin，既有 Pin 签名不动）+ 新增 hard-delete 策略（SqliteMemoryStore::hard_delete 物理删除 + MemoryService.HardDelete RPC + console-api POST /v1/memory/{id}/hard-delete 经 confirmMiddleware X-Confirm gated）+ AuditOperation::MemoryHardDelete`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 27 (memory-ops-hardening)
**Dependencies**: task-13.2（5 memory REST handler + `confirmMiddleware` deprecate/soft-delete gated 已落地）/ task-13.1（`SqliteMemoryStore` + `MemoryServer` 5 RPC + audit emit 已落地）/ task-17.1（`Pin{bool pin}` toggle + `set_pinned`）/ ADR-032（memory-ops-hardening，本 phase 新 Proposed，D2）/ ADR-017 D2（destructive op X-Confirm 服务端兜底，`confirmMiddleware`）/ ADR-021（`memory.pin`/`memory.unpin` event_type）/ ADR-013（禁伪造凭据）/ ADR-014 D1-D5（第十八次激活）

## 1. Background

Phase 13 / Phase 17 落地的 Memory pin / 生命周期语义有两处可硬化：

1. **Pin 是 toggle 而非显式 Pin/Unpin**：`PinMemoryRequest{ memory_id, bool pin }`（`proto:311`）一个 RPC 经 `pin` bool 兼任 pin 与 unpin——`core/src/data_plane/memory.rs:220` 据 `req.pin` 分流 `AuditOperation::MemoryPin` / `MemoryUnpin`；console-api `handleMemoryPin`（`internal/consoleapi/handlers.go:525`）POST 空 body 兜底 `pin=true`。toggle 形态对调用方语义不显式，无法表达「幂等 unpin」意图。

2. **Memory 生命周期只有 soft-delete**：`set_status(id, "soft_deleted")`（`store.rs:170`，CHECK 约束 `active/deprecated/soft_deleted`，`migration 0013:14`）只翻转状态——行仍在表中、get-by-id 仍可取（`store.rs:135` 注释「soft_deleted rows are still gettable by ID」）。无「物理删除 / 不可恢复清除」路径；隐私基线（ADR-004）下「彻底忘掉某条 memory」的诉求无法满足。

既有 destructive 操作（deprecate / soft-delete）已有服务端兜底：`internal/consoleapi/router.go:43-44` 经 `confirmMiddleware`（ADR-017 D2，`router.go:62`：`X-Confirm: yes` header 或 `?confirm=true` query，缺则 412）gated。

本 task 据 ADR-032 D2：(1) 新增显式 `Unpin` RPC（add-only，与既有 `Pin` 并存、不动其签名）让 unpin 语义显式且幂等；(2) 新增 `HardDelete` RPC（add-only，物理删除行），console-api `POST /v1/memory/{id}/hard-delete` 复用既有 `confirmMiddleware` X-Confirm 兜底（与 deprecate/soft-delete 同 destructive 确认 pattern）。

## 2. Goal

`proto/contextforge/console_data_plane/v1/console_data_plane.proto` 加 add-only `UnpinMemoryRequest`/`UnpinMemoryResponse` + `HardDeleteMemoryRequest`/`HardDeleteMemoryResponse` + `MemoryService` 加 `rpc Unpin` + `rpc HardDelete`（既有 5 RPC + `Pin{bool pin}` 签名不动）。`core/src/memory/store.rs` 新增 `hard_delete(memory_id)`（`DELETE FROM memory_items WHERE memory_id=?`，物理删除；行不存在返 NotFound）。`core/src/memoryops/audit.rs::AuditOperation` 加 add-only `MemoryHardDelete` 变体 + `as_str` 映射。`core/src/data_plane/memory.rs::MemoryServer` impl `unpin`（= `set_pinned(id, false)` 显式 + emit `MemoryUnpin`，幂等）+ `hard_delete`（调 `store.hard_delete` + emit `MemoryHardDelete`）。`internal/consoleapi/router.go` + `handlers.go` 加 add-only `POST /v1/memory/{id}/unpin`（non-destructive，对齐 pin）+ `POST /v1/memory/{id}/hard-delete` 经 `confirmMiddleware`（destructive，X-Confirm gated）。≥3 Rust + Go 测试全 PASS：Unpin 幂等 + 显式语义 / hard-delete 物理删除后 get-by-id 返 None / console-api hard-delete 缺 X-Confirm → 412 + 带 confirm → 204。默认构建 0 新依赖、0 网络（ADR-004）；既有 `Pin`/`Deprecate`/`SoftDelete` + `confirmMiddleware` 语义不变；`cargo test --workspace` + `go test ./...` 不退化。D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **修改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`**：add-only `message UnpinMemoryRequest{ string memory_id = 1; }` + `message UnpinMemoryResponse{}` + `message HardDeleteMemoryRequest{ string memory_id = 1; }` + `message HardDeleteMemoryResponse{}`；`service MemoryService` 加 `rpc Unpin(UnpinMemoryRequest) returns (UnpinMemoryResponse);` + `rpc HardDelete(HardDeleteMemoryRequest) returns (HardDeleteMemoryResponse);`。既有 5 RPC + `PinMemoryRequest{bool pin}` 不动。
- **修改 `core/src/memory/store.rs`**：新增 `hard_delete(&self, memory_id: &str) -> Result<(), MemoryStoreError>`——`DELETE FROM memory_items WHERE memory_id = ?`；`execute` 返回受影响行数 0 → `MemoryStoreError::NotFound`，>0 → `Ok(())`（物理删除，行从表中移除，后续 get-by-id 返 None）。
- **修改 `core/src/memoryops/audit.rs`**：`AuditOperation` enum 加 add-only `MemoryHardDelete` 变体（在既有 `MemorySoftDelete` 后）+ `as_str` 映射 `"memory_hard_delete"`（与既有 `memory_pin` 等并列）。
- **修改 `core/src/data_plane/memory.rs`**：`MemoryServer` impl `unpin`（语义 = `set_pinned(req.memory_id, false)` 显式 + `emit_audit_and_event(MemoryUnpin, id)`；幂等：unpin 已 unpin 的 item 仍 Ok）+ `hard_delete`（调 `store.hard_delete` + `emit_audit_and_event(MemoryHardDelete, id)`）；`build_memory_event` / `audit_op_to_event_type` 据 ADR-021 处理 `MemoryHardDelete`（新 event_type `memory.hard_delete` 或归既有，§5.2 定）。既有 `pin`（toggle）保留不动（向后兼容）。
- **修改 `internal/consoleapi/router.go`**：add-only `mux.HandleFunc("POST /v1/memory/{id}/unpin", handleMemoryUnpin(deps))`（non-destructive，对齐 pin 无 confirmMiddleware）+ `mux.HandleFunc("POST /v1/memory/{id}/hard-delete", confirmMiddleware(handleMemoryHardDelete(deps)))`（destructive，X-Confirm gated）。既有 5 memory route 不动。
- **修改 `internal/consoleapi/handlers.go`**：`handleMemoryUnpin`（trimID → `deps.Memory.Unpin(id)` 或 `Pin(id,false)` → 204）+ `handleMemoryHardDelete`（trimID → `deps.Memory.HardDelete(id)` → 204；destructive 经 confirmMiddleware 上游 gated）；Deps Memory 接口加 `Unpin` / `HardDelete` 方法（add-only，对齐既有 `Pin`/`Deprecate`/`SoftDelete`）。
- **新增同源测试**：Rust（`store.rs` hard_delete 物理删除后 get None / `data_plane/memory.rs` unpin 幂等 + hard_delete RPC emit audit）+ Go（`internal/consoleapi/router_test.go` hard-delete 缺 X-Confirm → 412 + 带 `X-Confirm: yes` → 204；unpin → 204）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **pin-actor + pinned-at-timestamp 字段** [SPEC-OWNER:task-27.1-memory-pin-actor-and-timestamp]：本 task 拆分 RPC + 加 hard-delete，actor/timestamp 字段在 27.1。
- **is_pinned 从 audit log 回填** [SPEC-OWNER:task-27.3-closeout-v0.20.0]：本 task 不做 backfill。
- **hard-delete 级联清理（清向量索引 / 引用该 memory 的 trace）** [SPEC-DEFER:phase-future.memory-hard-delete-cascade]：本 task 只物理删 `memory_items` 行；级联清理属后续。
- **soft-delete 回收站 / restore RPC** [SPEC-DEFER:phase-future.memory-restore-recycle-bin]：本 task 加 hard-delete（不可恢复），restore 路径不在此。
- **既有 `Pin{bool pin}` toggle 弃用 / 移除** [SPEC-DEFER:phase-future.memory-pin-toggle-deprecation]：本 task add-only 加 `Unpin`，既有 toggle 保留向后兼容；toggle 弃用属后续协议演进（ADR-015 D5 路径）。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`proto/.../console_data_plane.proto::MemoryService`**：契约 service，本 task add-only 加 `Unpin` / `HardDelete` RPC。
- **`core/src/memory/store.rs::SqliteMemoryStore`**：持久层，本 task 加 `hard_delete` 物理删除。
- **`core/src/memoryops/audit.rs::AuditOperation`**：审计 op enum，本 task 加 `MemoryHardDelete`。
- **`core/src/data_plane/memory.rs::MemoryServer`**：thin proxy，本 task impl `unpin` / `hard_delete`。
- **`internal/consoleapi/router.go::confirmMiddleware`**：ADR-017 D2 X-Confirm 兜底，本 task 复用 gate hard-delete。
- **下游 task-27.3**：closeout 把本 task 的 unpin/hard-delete 接进 smoke v17 + release docs。

## 5. Behavior Contract

### 5.1 Required Reading

- `proto/contextforge/console_data_plane/v1/console_data_plane.proto:296-336`（`PinMemoryRequest{memory_id, bool pin}` + `DeprecateMemoryRequest` / `SoftDeleteMemoryRequest` empty-response pattern + `MemoryService` 5 RPC）
- `core/src/memory/store.rs:153-187`（`set_pinned` toggle + `set_status` 三态 + `MemoryStoreError::NotFound` 当 execute 0 行 pattern `:160`）
- `core/src/memoryops/audit.rs:11-37`（`AuditOperation` enum + `MemoryPin/MemoryUnpin/MemoryDeprecate/MemorySoftDelete` + `as_str`）
- `core/src/data_plane/memory.rs:51-124`（`emit_audit_and_event` + `audit_op_to_event_type` `:83` + `build_memory_event` `:99` op_str + ADR-021 D2 event_type 注释）+ `:207-263`（`pin`/`deprecate`/`soft_delete` impl + emit pattern）
- `internal/consoleapi/router.go:38-71`（5 memory routes + deprecate/soft-delete 经 `confirmMiddleware` `:43-44` + `confirmMiddleware` impl `:62`：X-Confirm/confirm 兜底 → 412）
- `internal/consoleapi/handlers.go:519-588`（`handleMemoryPin` toggle 兜底 `:536` + `handleMemoryDeprecate` `:552` + `handleMemorySoftDelete` `:572` destructive pattern）+ `internal/consoleapi/router_test.go`（confirmMiddleware 412 既有测试 pattern）
- `docs/decisions/adr-032-memory-ops-hardening.md`（D2）+ `docs/decisions/adr-017-console-contract-completion-22-endpoint.md`（D2 destructive X-Confirm 兜底）+ `docs/decisions/adr-021-memory-event-bus-bridge.md`（event_type namespace）+ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造）

### 5.2 关键设计 — Pin/Unpin 拆分 + hard-delete + X-Confirm 复用

- **Unpin add-only**：新 `Unpin(UnpinMemoryRequest{memory_id})` RPC 语义 = `set_pinned(id, false)` 显式调用 + emit `MemoryUnpin`（既有 audit op，复用）。既有 `Pin{bool pin}` toggle **不动**（`pin=true` 仍 pin、`pin=false` 仍 unpin，向后兼容）；`Unpin` 是 `pin=false` 路径的语义显式化 + 幂等（unpin 已 unpin 的 item：`set_pinned(id,false)` UPDATE 0/1 行均 Ok，幂等成立——但若 item 不存在仍返 NotFound）。console-api `POST /v1/memory/{id}/unpin` non-destructive（对齐 pin 无 confirmMiddleware）。
- **hard-delete 策略**：`store.hard_delete(id)` = `DELETE FROM memory_items WHERE memory_id=?`——物理删除（不同于 soft-delete 的 `status='soft_deleted'` 状态翻转）；execute 受影响 0 行 → `NotFound`、>0 → Ok。删除后 get-by-id 返 None（vs soft-delete 后 get-by-id 仍返行）。`MemoryHardDelete` audit op 记录操作。
- **X-Confirm 复用（ADR-017 D2）**：console-api `POST /v1/memory/{id}/hard-delete` 经既有 `confirmMiddleware` gated——缺 `X-Confirm: yes` header 且无 `?confirm=true` → 412 Precondition Failed；带任一 → 进 handler → 204。与 deprecate/soft-delete 同 destructive 确认 pattern（`router.go:43-44`），不引入新确认机制。
- **event_type（ADR-021）**：`MemoryHardDelete` 的 `audit_op_to_event_type` 映射——新 event_type `"memory.hard_delete"`（与既有 `memory.deprecate`/`memory.soft_delete` 并列）或归既有 namespace，§10 据 ADR-021 D2 event_type namespace 紧凑原则定（优先新 `memory.hard_delete`，与 soft_delete 区分物理 vs 软删）。
- **ADR-013**：hard-delete 物理删除后 get None + console-api 缺 X-Confirm → 412 是 deterministic 默认构建可验证项（🟢 默认构建真实往返）。

### 5.3 不变量

- 默认构建 0 新依赖（`DELETE` 用既有 `rusqlite`，0 网络 ADR-004）。
- proto add-only：既有 5 RPC + `PinMemoryRequest{bool pin}` 签名 / message tag 不变；新 RPC / message 为追加（proto-freeze guard 守护）。
- 既有 `Pin`（toggle）/ `Deprecate` / `SoftDelete` RPC 行为不变；`confirmMiddleware` 语义不退化（既有 deprecate/soft-delete 412 测试不破坏）。
- hard-delete 与 status 正交：物理删除（行移除）vs soft-delete（status 翻转、行保留）；hard-delete 删任意 status 的行（active/deprecated/soft_deleted 均可物理删）。
- Unpin 幂等：unpin 已 unpin 的 item 返 Ok（不报错）；item 不存在返 NotFound。
- 不破坏既有 5 memory 单测（Rust `data_plane/memory.rs` + Go `consoleapi`）。

## 6. Acceptance Criteria

- [x] **AC1**: proto add-only `Unpin`/`HardDelete` RPC + 4 个 request/response message（既有 5 RPC + `Pin{bool pin}` 签名不动）；proto-freeze guard（`core/tests/proto_contract.rs`）服务/message superset 追加不退化 — verified by **TEST-27.2.1**
- [x] **AC2**: `SqliteMemoryStore::hard_delete` 物理删除——删除后 get-by-id 返 None（vs soft-delete 仍返行）；行不存在返 NotFound；`MemoryServer.hard_delete` RPC emit `MemoryHardDelete` audit — verified by **TEST-27.2.2**
- [x] **AC3**: `MemoryServer.unpin` 显式语义 = `set_pinned(id,false)` + emit `MemoryUnpin`、幂等（unpin 已 unpin item 返 Ok）；既有 `pin` toggle 行为不变 — verified by **TEST-27.2.3**
- [x] **AC4**: console-api `POST /v1/memory/{id}/hard-delete` 缺 X-Confirm → 412（confirmMiddleware gated）+ 带 `X-Confirm: yes`（或 `?confirm=true`）→ 204；`POST /v1/memory/{id}/unpin` non-destructive → 204；既有 deprecate/soft-delete 412 不退化 — verified by **TEST-27.2.4**
- [x] **AC5**: 既有不退化 + D2 lint — 默认 `cargo test --workspace`（0 新依赖）+ `go test ./...` 全 PASS；`bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-27.2.5** + §10

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-27.2.1 | proto add-only `Unpin`/`HardDelete` RPC + message + FROZEN 契约 superset 追加不退化 | `proto/contextforge/console_data_plane/v1/console_data_plane.proto` + `core/tests/proto_contract.rs` | Done |
| TEST-27.2.2 | `hard_delete` 物理删除后 get None / 行不存在 NotFound / RPC emit `MemoryHardDelete` audit | `core/src/memory/store.rs` + `core/src/data_plane/memory.rs`（`mod tests`） | Done |
| TEST-27.2.3 | `unpin` 显式语义 = set_pinned(false) + emit MemoryUnpin + 幂等 / 既有 pin toggle 不变 | `core/src/data_plane/memory.rs`（`mod tests`） | Done |
| TEST-27.2.4 | console-api hard-delete 缺 X-Confirm → 412 / 带 confirm → 204 → 404 / unpin → 204 / 既有 destructive 412 不退化 | `internal/consoleapi/router_test.go` | Done |
| TEST-27.2.5 | 默认 `cargo test --workspace` + `go test ./...` 0 failed + D2 lint 0 未标注命中 | 全 Rust + Go + `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（中）hard-delete 不可恢复 + 误触发**（承 phase-27 §7 R3）：物理删除无回收站；误传 `confirm=true` 会真删。
  - **缓解**：复用既有 `confirmMiddleware`（ADR-017 D2，X-Confirm gated）；单测断言缺 X-Confirm → 412；级联清理 `[SPEC-DEFER:phase-future.memory-hard-delete-cascade]` 如实延后；不可恢复属设计意图（隐私基线 ADR-004），§10 记录。
- **R2（中）proto add-only RPC 影响生成代码 / freeze guard**：新 RPC 改 `*_grpc.pb.go` / Rust tonic 生成。
  - **缓解**：add-only RPC 不动既有 RPC 签名；proto-freeze guard 断言 service / message superset 追加；codegen（`core/build.rs` + go proto gen）按既有流程重生成。stop-condition：freeze guard 不过则 AC1 不标 `[x]`（不伪造）。
- **R3（低）`MemoryHardDelete` event_type namespace 抉择**（ADR-021 D2 紧凑原则）：新 `memory.hard_delete` vs 归既有。
  - **缓解**：优先新 `memory.hard_delete`（物理 vs 软删可区分）；§10 据 ADR-021 D2 定 + 单测断言 emit 的 event_type；不破坏既有 `memory.pin`/`memory.deprecate`/`memory.soft_delete` 集合。
- **R4（低）console-api Memory 接口扩展破坏既有实现**：Deps Memory 接口加 `Unpin`/`HardDelete` 方法。
  - **缓解**：接口 add-only 加方法（对齐既有 `Pin`/`Deprecate`/`SoftDelete`）；既有实现（gRPC proxy + fallback inmem）同步 impl 新方法；既有 5 memory 测试不破坏。

## 9. Verification Plan

```bash
# Rust：hard_delete 物理删除 + unpin 幂等 + audit emit（默认构建 0 新依赖）
cargo test -p contextforge-core memory::store
cargo test -p contextforge-core data_plane::memory
cargo test -p contextforge-core --test proto_contract

# 默认构建不退化
cargo test --workspace

# Go：console-api hard-delete X-Confirm 412/204 + unpin 204 + 既有 destructive 不退化
go test ./internal/consoleapi/...
go test ./...

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **Status**: Done（2026-06-01）。
- **完成日期**：2026-06-01。
- **改动文件**：
  - `proto/.../console_data_plane.proto`——add-only `UnpinMemoryRequest`/`UnpinMemoryResponse` + `HardDeleteMemoryRequest`/`HardDeleteMemoryResponse` + `MemoryService` `rpc Unpin` + `rpc HardDelete`（既有 5 RPC + `PinMemoryRequest{bool pin}` 不动；regen Go pb.go/grpc.pb.go + Rust prost）。
  - `core/src/memory/store.rs`——`hard_delete`（`DELETE FROM memory_items`，0 行 NotFound）+ 物理删除测试。
  - `core/src/memoryops/audit.rs`——`AuditOperation::MemoryHardDelete` + `as_str "memory_hard_delete"`。
  - `core/src/data_plane/memory.rs`——`MemoryServer.unpin`（`set_pinned_with_actor(id,false,"console-api")` + emit `MemoryUnpin`，幂等）+ `hard_delete`（`store.hard_delete` + emit `MemoryHardDelete`）+ `audit_op_to_event_type`/`build_memory_event` 加 `memory.hard_delete` 映射 + RPC 测试。
  - `internal/consoleapi/types.go`——`MemoryClient` + `Unpin`/`HardDelete`；`grpcclient.go`/`memstore.go`/`console_api_serve_degraded.go` 实现；`handlers.go`——`handleMemoryUnpin`（non-destructive 204）/ `handleMemoryHardDelete`（confirmMiddleware-gated）；`router.go`——add-only `POST /v1/memory/{id}/unpin` + `.../hard-delete`（X-Confirm gated）；`router_test.go`——412/204/404 + unpin 204。
  - `core/tests/proto_contract.rs`——`test_27_2_memory_service_unpin_harddelete_superset`（service + message FROZEN guard）。0 新依赖（`DELETE` 用既有 rusqlite）。
- **commit 列表（RED→GREEN）**：RED `test(memory): TEST-27.2 RED`（proto+regen + `hard_delete` todo!() + audit op + unpin real + Go wiring + 测试）→ GREEN `feat(memory): hard_delete 物理删除 + Pin/Unpin 拆分`（`store.hard_delete` 实现）。
- **§9 Verification 实测结果（ADR-013 真实非合成）**：`cargo test -p contextforge-core --lib memory::store` **14 passed**（hard_delete 物理删除/NotFound/任意 status）；`data_plane::memory` **14 passed**（unpin 幂等 + emit MemoryUnpin / hard_delete 物理删除 + emit MemoryHardDelete + event_type）；`--test proto_contract` MemoryService superset PASS；`go test ./internal/consoleapi/...` PASS（hard-delete 412→204→404 物理删除坐实 / unpin 204 / 既有 destructive 412 不退化）；`cargo test --workspace` + `go test ./...` 0 failed。
- **设计取舍**：(1) **Unpin 显式 + Pin toggle 保留**——`Unpin` RPC = `set_pinned(id,false)` 显式 + 幂等（UPDATE 0/1 行均 Ok；item 不存在 NotFound）+ emit `MemoryUnpin`（既有 audit op 复用）；既有 `Pin{bool pin}` toggle 不动（向后兼容；toggle 弃用属后续协议演进）。(2) **hard-delete event_type** = 新 `memory.hard_delete`（与 `memory.soft_delete` 区分物理 vs 软删，ADR-021 D2 namespace；同步加进 events.rs replay 的 string 映射经 27.x 一致）。(3) **X-Confirm 复用**——console-api `hard-delete` 经既有 `confirmMiddleware`（ADR-017 D2，缺 X-Confirm/`?confirm=true` → 412），不引入新确认机制；`unpin` non-destructive 无 gate。(4) **Deps Memory 接口 add-only 扩展** Unpin/HardDelete（grpcclient/MemMemoryStore/degradedMemory 同步实现）。
- **剩余风险 + 下游影响**：hard-delete 级联清理（向量索引 / 引用该 memory 的 trace）`[SPEC-DEFER:phase-future.memory-hard-delete-cascade]`；`Pin{bool pin}` toggle 弃用 `[SPEC-DEFER:phase-future.memory-pin-toggle-deprecation]`；hard-delete 不可恢复属设计意图（隐私基线 ADR-004）；task-27.3 is_pinned audit backfill + smoke v17 + closeout 衔接。
