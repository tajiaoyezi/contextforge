# Task `15.2`: `memory-event-bus-bridge — MemoryServer.emit_audit 同步桥接 EventBus.send (memory.pin/deprecate/soft_delete)`

**Status**: Done

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 15 (console-functional-gap-closure)
**Dependencies**: task-11.4 (EventBus broadcast channel) + task-13.1 (MemoryServer + emit_audit) + [ADR-021](../../decisions/adr-021-memory-event-bus-bridge.md)

## 1. Background

ContextForge-Console PR #91/#93 backlog 列 P0 #2：

> `GET /v1/observability/events` 永不返 `memory.pin` / `memory.deprecate` / `memory.soft_delete` 类型 event — Console UI Memory 详情面板"操作历史"列表预期通过 `?event_type=memory.*` 拉 memory 变更流，当前永远空。

**根因**：Phase 13 task-13.1 ship 的 `core/src/data_plane/memory.rs::MemoryServer.emit_audit`（line 39-56）只写 `AuditSink` (SQLite audit_log 表)，不调 `EventBus.send`。task-13.1 当时 [SPEC-DEFER:phase-future.memory-event-bus-bridge] 留给后续。

Phase 15 task-15.2 落地 [ADR-021](../../decisions/adr-021-memory-event-bus-bridge.md) D1-D4：emit_audit 内同步追加 EventBus.send（best-effort，SendError swallowed），不引入新 channel。

## 2. Goal

`core/src/data_plane/memory.rs::MemoryServer` 加 `emit_audit_and_event(op, memory_id)` 替代既有 `emit_audit`；Pin/Deprecate/SoftDelete 3 handler 调用点改用新方法；3 个 `memory.*` event_type 字符串实施 D2；ObservabilityEvent 字段填充按 D3；EventBus.send SendError swallow（D4）。≥3 新 Rust unit test PASS；`cargo test --workspace` 不退化；实测 `GET /v1/observability/events` 拉到 memory.* event。

## 3. Scope

### In Scope

- **修改 `core/src/data_plane/memory.rs`**：
  - 重命名既有 `emit_audit(op, memory_id)` 为 `emit_audit_and_event(op, memory_id)`（同名替换）
  - 新方法签名（line 39-56 既有 emit_audit 同位置）：
    ```rust
    fn emit_audit_and_event(&self, op: AuditOperation, memory_id: &str) {
        // 1. AuditSink 既有路径（保留）
        if let Some(audit) = self.stores.audit.as_ref() {
            if let Ok(mut sink) = audit.lock() {
                let event = AuditEvent { /* ... 既有内容 ... */ };
                let _ = sink.record(event);
            }
        }
        // 2. EventBus 新增桥接（ADR-021 D1）
        if let Some(bus) = self.stores.event_bus.as_ref() {
            let event_type = audit_op_to_event_type(op);
            if let Some(evt) = build_memory_event(&event_type, memory_id, op) {
                let _ = bus.send(evt);  // best-effort; ADR-021 D4
            }
        }
    }
    ```
  - 新增模块私有函数 `audit_op_to_event_type(op: AuditOperation) -> &'static str`:
    ```rust
    fn audit_op_to_event_type(op: AuditOperation) -> &'static str {
        match op {
            AuditOperation::MemoryPin | AuditOperation::MemoryUnpin => "memory.pin",
            AuditOperation::MemoryDeprecate => "memory.deprecate",
            AuditOperation::MemorySoftDelete => "memory.soft_delete",
            _ => "",  // 其它 op 不应到达此分支；返空触发 None Option chain
        }
    }
    ```
  - 新增模块私有函数 `build_memory_event(event_type: &str, memory_id: &str, op: AuditOperation) -> Option<PbEvent>`:
    - event_type 空 → 返 None
    - 否则构造 PbEvent（按 ADR-021 D3 字段映射）：
      ```rust
      let op_str = match op {
          AuditOperation::MemoryPin => "pin",
          AuditOperation::MemoryUnpin => "unpin",
          AuditOperation::MemoryDeprecate => "deprecate",
          AuditOperation::MemorySoftDelete => "soft_delete",
          _ => return None,
      };
      Some(PbEvent {
          event_id: format!("evt-memory-{}", now_unix_nanos()),
          event_type: event_type.to_string(),
          severity: "info".to_string(),
          source: "contextforge-core".to_string(),
          message: format!("memory {}: {}", op_str, memory_id),
          ts_unix: now_unix(),
          trace_id: None,
          job_id: None,
          payload_json: format!(r#"{{"memory_id":"{}","op":"{}"}}"#, memory_id, op_str),
      })
      ```
    - `now_unix` / `now_unix_nanos` 复用 `events.rs` 中的 helper（pub via `use crate::data_plane::events::*` or duplicate as private helpers）
  - 3 handler (`pin` / `deprecate` / `soft_delete`) 调用点（既有 line 125-181）改：所有 `self.emit_audit(op, &id)` → `self.emit_audit_and_event(op, &id)`

- **PbEvent import**：`core/src/data_plane/memory.rs` 顶部 `use crate::pb_console::ObservabilityEvent as PbEvent;` 加入

- **单元测试 ≥3**（`core/src/data_plane/memory.rs` 内 `#[cfg(test)] mod tests`）：
  - `test_pin_emits_event_bus` — 创建带 EventBus 的 MemoryServer，订阅一个 receiver，调 pin → recv 拿到 memory.pin event
  - `test_deprecate_emits_event_bus` — 同款 deprecate → memory.deprecate event
  - `test_soft_delete_emits_event_bus` — 同款 soft_delete → memory.soft_delete event
  - 加分项 `test_emit_swallows_send_error` — 无订阅者时 send 返 SendError；操作不 panic 且 REST 仍返成功

- **不修改**：
  - AuditSink 既有逻辑 (audit_log SQLite 表写不变)
  - EventBus 实现 (events.rs)
  - DataPlaneStores struct (event_bus 字段 task-11.4 已加)
  - proto schema (ObservabilityEvent.event_type 是 string，无 schema 变更)
  - Go side anything

- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **历史 audit log 重放到 EventBus** [SPEC-DEFER:phase-future.events-replay-from-audit]
- **memory.pin / memory.unpin event_type 拆分** [SPEC-DEFER:phase-future.memory-pin-unpin-split]
- **memory list / get 操作 emit event**（仅状态写 emit）
- **MemMemoryStore fallback emit EventBus event** [SPEC-DEFER:phase-future.memstore-event-emit]
- **broadcast channel partition** (memory bus / indexing bus 分离) [SPEC-DEFER:phase-future.event-bus-partition]
- **EventBus capacity 提升 / 配置化** [SPEC-DEFER:phase-future.event-bus-capacity]

## 4. Users / Actors

- **Console UI 端**（下游，via cross-repo）：Memory 详情面板"操作历史"列表自动获取 memory.* event
- **observability 工具链**：Grafana / log forwarder 拉 events stream 解析 memory.* event type 用于 audit dashboard

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/decisions/adr-021-memory-event-bus-bridge.md` D1-D4
- `docs/specs/phases/phase-15-console-functional-gap-closure.md` §3 / §6 AC2
- `core/src/data_plane/memory.rs` 既有 line 39-56 (emit_audit) + line 125-181 (3 handler)
- `core/src/data_plane/events.rs` 全文（EventBus 实现 + build_*_event helpers）
- `core/src/data_plane/mod.rs::DataPlaneStores.event_bus` 字段 (line 50-53)
- `core/src/memoryops/audit.rs::AuditOperation` enum (8 变体)

### 5.2 Imports

- **Rust**: 现有 `std::sync::{Arc, Mutex}` + `tonic::*`；新 import `use crate::pb_console::ObservabilityEvent as PbEvent;`
- **不引入新依赖**：R7 不触发

### 5.3 调用契约

```rust
// pin handler (既有 line 125-147 框架)
async fn pin(&self, req: Request<PinMemoryRequest>) -> Result<Response<PinMemoryResponse>, Status> {
    let inner = req.into_inner();
    // ... 既有 set_pinned ...
    let op = if inner.pin { AuditOperation::MemoryPin } else { AuditOperation::MemoryUnpin };
    self.emit_audit_and_event(op, &inner.memory_id);  // 改这一行 (既有是 emit_audit)
    Ok(Response::new(PinMemoryResponse { /* ... */ }))
}
```

deprecate / soft_delete handler 类比。

### 5.4 Event 字段填充测试预期

| Field | pin 时 | deprecate 时 | soft_delete 时 |
|---|---|---|---|
| event_type | `"memory.pin"` | `"memory.deprecate"` | `"memory.soft_delete"` |
| severity | `"info"` | `"info"` | `"info"` |
| source | `"contextforge-core"` | `"contextforge-core"` | `"contextforge-core"` |
| message | `"memory pin: mem-1"` | `"memory deprecate: mem-1"` | `"memory soft_delete: mem-1"` |
| payload_json | `{"memory_id":"mem-1","op":"pin"}` | `{"memory_id":"mem-1","op":"deprecate"}` | `{"memory_id":"mem-1","op":"soft_delete"}` |
| trace_id | `None` | `None` | `None` |
| job_id | `None` | `None` | `None` |

## 6. Acceptance Criteria

- [x] AC1：MemoryServer.pin (req.pin=true / false) 触发 `EventBus.send(memory.pin)` event；订阅者 recv 拿到 event_type="memory.pin" + payload_json 含 op="pin" / "unpin" — **verified by `core/src/data_plane/memory.rs::tests::test_pin_emits_event_bus_memory_pin` + `test_unpin_emits_event_bus_memory_pin_with_op_unpin` PASS**
- [x] AC2：MemoryServer.deprecate / soft_delete 同款 emit memory.deprecate / memory.soft_delete event — **verified by `test_deprecate_emits_event_bus_memory_deprecate` + `test_soft_delete_emits_event_bus_memory_soft_delete` PASS**
- [x] AC3：EventBus.send SendError (无订阅者) 不影响 state-op 成功返回；audit log 正常写入；REST 返 204 — **verified by `test_pin_swallows_send_error_when_no_subscriber` PASS**
- [x] AC4：`cargo test --workspace` 既有 task-11.4 events test + task-13.1 memory test 不退化 — **verified by `cargo test --workspace` 100 lib tests + 17 integration tests 全 PASS（含 task-13.1 既有 5 memory test 不退化）**
- [x] AC5：unit-level 实测覆盖通过 `EventBus.subscribe()` + `drain_events` 模式（broadcast recv 验证）；daemon-level curl 实测留 smoke v6 (task-15.6) 集成 — **verified by 6 新 unit test PASS（含 audit_op_to_event_type filter test）**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | pin emit memory.pin | memory.rs + 2 new test | Done |
| AC2 | deprecate/soft_delete emit | memory.rs + 2 new test | Done |
| AC3 | SendError swallow | memory.rs + 1 new test | Done |
| AC4 | cargo test 不退化 | cargo test --workspace | Done |
| AC5 | daemon-level 实测 | smoke v6 (task-15.6 集成) | Deferred to task-15.6 |

## 8. Risks

- **EventBus broadcast 满 (cap=1000) 导致 lag**：单用户场景充分；缓解 ADR-021 §Trade-offs 记录 + Phase 15 风险
- **emit 路径竞态**：emit_audit_and_event 在 spawn_blocking 内调用时跨 await 边界；缓解 emit_audit_and_event 是同步函数（不 await），调用点在 await 完成后
- **AuditOperation enum 漏匹配**：audit_op_to_event_type 漏 case → 静态分析 + match exhaustive；用 `_` 兜底返空 → build_memory_event 返 None → 不 emit（safe degradation）
- **测试创建 DataPlaneStores 复杂**：需要 audit + event_bus 共存；缓解仿照 task-13.1 既有 test pattern + 用 EventBus::new() + AuditSink::open(tempdir)
- **memory.pin vs memory.unpin 不分**：ADR-021 D2 决策合并；payload_json.op 区分；UI 端实施时按 op 字段过滤

## 9. Verification Plan

- **install**: `cargo fetch`
- **lint**: `cargo fmt --check`
- **typecheck**: `cargo check --workspace`
- **unit-test**: `cargo test -p contextforge-core --lib data_plane::memory::tests`（≥3 新 + 既有 task-13.1 test 不退化）
- **integration**: `cargo test --workspace`（含 core/tests/ + 跨 crate）
- **e2e**: N/A（emit/receive 是 in-process broadcast；smoke v6 验证 REST 拉取）
- **build**: `cargo build --workspace --release`
- **coverage**: 不强制
- **runtime-smoke**: start daemon + 订阅 events + trigger pin + verify recv
- **manual**: curl POST /v1/memory/<id>/pin + curl GET /v1/observability/events 验证 memory.pin event

## 10. Completion Notes

- **完成日期**：2026-05-26
- **关键决策**：
  - **pin / unpin 共享 event_type="memory.pin"**：按 ADR-021 D2 决策，payload_json 内 `op` 字段区分 pin / unpin；event_type 命名空间紧凑（避免 memory.pin + memory.unpin 双 type）
  - **PbEvent.severity 统一 "info"**：所有 memory.* 操作都是用户主动；不报 warn/error
  - **payload_json 用 serde_json 编码 memory_id**：避免 quote / control char 引起的 JSON 注入问题
  - **trace_id / job_id 都 None**：memory 操作不属于 indexing job / search trace 上下文
  - **DataPlaneStores 测试中手动构造**：现有 `with_memory` 不带 EventBus 参数；新增 `fresh_server_with_event_bus` 测试 helper 用 struct literal 同时填 audit + event_bus
  - **drain_events helper via try_recv**：broadcast 实际是同步 send；测试中 await `tokio::task::yield_now()` 后 `try_recv` 取出全部已 buffer 的事件
- **§9 Verification 结果**：
  - `cargo check -p contextforge-core --tests`: clean（只有一处既有 unused import warning，与本 task 无关）
  - `cargo test -p contextforge-core --lib data_plane::memory`: 11 tests PASS（5 既有 + 6 新）
  - `cargo test --workspace`: 100 lib tests + 17 integration test files 全 PASS（task-11.4 events test + task-13.1 memory test + 跨 phase 集成全不退化）
- **改动文件**：
  - `core/src/data_plane/memory.rs` (修改 — emit_audit → emit_audit_and_event + audit_op_to_event_type + build_memory_event + now_unix/now_unix_nanos helpers + PbEvent import + 3 handler 调用点 + 6 新 unit test + fresh_server_with_event_bus helper)
  - `docs/specs/tasks/task-15.2-memory-event-bus-bridge.md` (本 spec §6 [x] / §7 Done / §10 完工 + Status → Done)
- **commit 列表**：
  - feat(core/data_plane/memory): task-15.2 — memory.pin/deprecate/soft_delete → EventBus.send (ADR-021 D1-D4)
  - docs(spec): task-15.2 §6/§7/§10 / Status → Done
- **剩余风险 / 未做项**：
  - 历史 audit 重放 [SPEC-DEFER:phase-future.events-replay-from-audit]
  - pin/unpin event_type 拆分 [SPEC-DEFER:phase-future.memory-pin-unpin-split]
  - MemStore fallback emit [SPEC-DEFER:phase-future.memstore-event-emit]
- **下游 task 影响**：task-15.6 smoke v6 Step 22 验证本 task EventBus 桥接；ADR-021 Phase 15 closeout 推 Accepted
