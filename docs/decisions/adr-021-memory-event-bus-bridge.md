# ADR `021`: `memory-event-bus-bridge`

**Status**: Accepted (2026-05-26, via Phase 15 closeout PR)
**Category**: 协议接口 / 可观测性 / 跨子系统桥接
**Date**: 2026-05-26
**Decided By**: tajiaoyezi objective + main agent execution + ContextForge-Console PR #91/#93 backlog 反馈
**Related**: ADR-010 (audit-cross-language-unification) / ADR-015 (console-contract-v1-compatibility) / ADR-016 (cross-process-rust-go-via-grpc-bridge) / ADR-017 (console-contract-completion-22-endpoint) / Phase 13 (memory-rest-surface) / Phase 15 (console-functional-gap-closure)

## Context

Phase 13（v0.6 ship）落地 5 个 Memory RPC（List / Get / Pin / Deprecate / SoftDelete），每个状态写操作（Pin / Deprecate / SoftDelete）通过 `emit_audit` 写入 `AuditSink`（SQLite `audit_log` 表）—— 但**不**桥接到 `EventBus.send` 广播。

后果：

- `GET /v1/observability/events` 仅返 `indexing.*`（task-11.4）+ `core.keepalive` 类事件，**永不**返 `memory.pin` / `memory.deprecate` / `memory.soft_delete`
- Console UI Memory 详情面板"操作历史"列表预期通过 `/v1/observability/events?event_type=memory.*` 拉取 memory 变更流 — 当前永远空
- Audit log 是持久化历史（SQLite 表查询），EventBus 是实时广播流（broadcast channel）——两条信号路径互补但都需要

**Phase 13 决策记录**：当时把 `memory.* → EventBus` 标 `[SPEC-DEFER:phase-future.memory-event-bus-bridge]`，理由是 v0.6 ship 收口已紧（22-endpoint conformance），桥接非 conformance 阻塞项。Phase 15 是合适窗口推进。

**Console PR #91/#93 backlog 列项 #2**（P0 优先级）：

> Memory pin/deprecate/soft_delete audit 事件没桥接到 ObservabilityEvent stream → Console UI Memory 详情"操作历史"列表空。

## Decision

ContextForge v0.8.0 minor release（Phase 15 task-15.2）通过 **4 个 Decision** 把 memory state-op audit 同步桥接到 `EventBus.send`：不引入新 channel + add-only event_type 字符串 + best-effort emit + zero conformance 退化。

### D1 — 桥接路径：`emit_audit` 内同步 emit `EventBus.send`

`core/src/data_plane/memory.rs::MemoryServer.emit_audit` 在调用 `AuditSink.record(...)` 后同步追加 `EventBus.send(ObservabilityEvent)` 路径：

```rust
// in MemoryServer
fn emit_audit_and_event(&self, op: AuditOperation, memory_id: &str) {
    // 1. AuditSink (既有路径)
    if let Some(audit) = self.stores.audit.as_ref() {
        if let Ok(mut sink) = audit.lock() {
            let _ = sink.record(AuditEvent { ... });
        }
    }
    // 2. EventBus (本 ADR 新增桥接)
    if let Some(bus) = self.stores.event_bus.as_ref() {
        let event_type = audit_op_to_event_type(op);  // "memory.pin" | "memory.deprecate" | ...
        let evt = build_memory_event(&event_type, memory_id);
        let _ = bus.send(evt);  // best-effort; SendError swallowed
    }
}
```

**理由**：

- 同一调用点 emit 两路 — 保证 audit / event 两条信号一致性（不存在 audit 写成功 / event 漏 emit 的不一致状态）
- 不引入新 channel（复用 task-11.4 既有 `EventBus broadcast::channel(1000)`）
- 不引入新 trait / 不解耦（v0.8 minor 不背重构包袱）

### D2 — `EventType` 字符串枚举扩展：3 个新值（add-only）

proto `console_data_plane.proto::ObservabilityEvent.event_type` 字段当前是 `string`（line 361），既有合法值：

- `indexing.progress` / `indexing.cancelled` / `indexing.error`（task-11.4）
- `core.keepalive`（task-11.4 fallback）

本 ADR 加 3 个合法值（add-only，proto 字段类型不变；仅文档约定 + 代码使用）：

- `memory.pin`
- `memory.deprecate`
- `memory.soft_delete`

**理由**：

- proto 字段是 string（不是 enum）→ 无 schema 变更（ADR-015 D1 add-only 满足）
- Console 旧客户端解析 `event_type` 时遇 `memory.*` 不会报错（string 任意值；UI 可忽略未知 event_type）
- 不引入 unpin event（Pin RPC `req.pin = false` 视为 unpin，但归为 `memory.pin` 同一 event_type；payload_json 携带 `pin: bool` 字段区分）
- 不动 `workspace.*` / `indexing.*` 命名空间（隔离）

### D3 — 字段映射（`ObservabilityEvent` 字段填充约定）

memory.* 事件的 `ObservabilityEvent` 字段填充：

| 字段 | memory.pin/deprecate/soft_delete 填值 |
|---|---|
| `event_id` | `evt-memory-{nanos}` |
| `event_type` | `memory.pin` / `memory.deprecate` / `memory.soft_delete` |
| `severity` | `info`（不论 pin/unpin/deprecate/soft_delete） |
| `source` | `contextforge-core`（与既有 indexing.* 一致） |
| `message` | `"memory <op>: <memory_id>"`（人类可读单行） |
| `ts_unix` | `now_unix()` |
| `trace_id` | `None`（memory 操作无 trace 概念） |
| `job_id` | `None`（不是 indexing job） |
| `payload_json` | `{"memory_id": "<id>", "op": "<pin/unpin/deprecate/soft_delete>"}` |

**理由**：

- `trace_id` / `job_id` `None` 是合法（既有 `core.keepalive` 也是 `None` / `None`）
- `payload_json` 携带 `op` 子字段细分 pin / unpin（pin RPC `pin=false` → `op=unpin`）
- `severity` 统一 `info`（用户主动操作，不是 error / warn）

### D4 — Best-effort emit：`SendError` swallow

`EventBus.send` 返 `Result<usize, SendError<PbEvent>>`。本 ADR 约定：

- 失败（无订阅者 `SendError`）→ swallow（不 log error，不 unwind 调用栈）
- 失败不影响 audit / 不影响 state-op 成功返回 REST 204
- 复用 task-11.4 既有 `let _ = bus.send(evt);` pattern

**理由**：

- local-first single-user 场景，无订阅者属于正常情况（Console UI 未打开 events 面板时）
- 事件是观测信号，不是事务一致性数据 — best-effort 适当
- AuditSink 是持久化主路径，EventBus 是实时广播副路径 — 失败语义不对称合理

## Trade-offs / Conscious limitations

- **不重放历史 audit log 到 EventBus**：Console UI 拉 events 是从订阅时刻开始（broadcast channel 不存历史）；想看历史需要查 audit log（不在本 ADR scope）— 留 [SPEC-DEFER:phase-future.events-replay-from-audit] v1.x
- **不区分 pin / unpin event_type**：合并到 `memory.pin`，UI 通过 `payload_json.op` 区分 — 简化 event_type 命名空间（避免 `memory.pin` vs `memory.unpin` 两个 type）
- **不引入 memory list / get 事件**：仅状态写操作（pin/deprecate/soft_delete）emit；读操作不广播（避免噪声）
- **broadcast channel 满（1000 events）时 drop oldest**：lag 是 task-11.4 既有行为；memory 事件高频时可能丢；缓解 1000 容量对单用户充分
- **MemMemoryStore fallback 不 emit event**：MemStore.Pin/Deprecate/SoftDelete 不持有 EventBus 引用（仅 in-memory）；fallback 模式下 `/v1/observability/events` 仍只返 events ring buffer 内容 — 接受作为 fallback 行为限制 [SPEC-DEFER:phase-future.memstore-event-emit]

## Verification (Phase 15 task-15.2 ship 时)

```bash
# 1. Rust unit test
cargo test -p contextforge-core --lib memory_event_bridge::tests
# expect: ≥3 测试 PASS（pin / deprecate / soft_delete each emit 1 event）

# 2. 实测 emit 与拉取
# (a) start daemon
contextforge-daemon &
sleep 2

# (b) subscribe events (在另一 terminal)
curl -N 'http://localhost:48181/v1/observability/events?wait=5' > /tmp/events.jsonl &
EVTPID=$!

# (c) trigger memory pin
curl -X POST -H "X-Confirm: yes" \
  http://localhost:48181/v1/memory/mem-1/pin \
  -d '{"pin": true}'

# (d) check events
sleep 1
kill $EVTPID
grep -c '"event_type":"memory.pin"' /tmp/events.jsonl
# expect: ≥1
```

## Rollback path

如 Phase 15 task-15.2 ship 后发现：

- memory 操作 emit event 导致 broadcast channel 满频繁丢 indexing event → 单独 ship patch 把 capacity 提到 5000 或 partition channel（memory bus / indexing bus 分离）
- Console UI 端收到 `memory.*` event 后渲染 bug → ContextForge 侧不动；Console 端 patch（feature flag 关闭 memory event 渲染）
- 极端：emit 路径有竞态导致 audit 写丢 → revert task-15.2 commit；v0.8.0.1 patch ship 仅含 task-15.1/15.3-15.6

ADR-021 不撤回 default（D1-D4 都是 best-effort + add-only；rollback 路径是 patch fix 而非 ADR superseded）。

## Upgrade path (v0.7.x → v0.8.0)

### Console UI 用户

- v0.7 客户端解析 `/v1/observability/events` 仅看到 `indexing.*` / `core.keepalive` — v0.8 ship 后新增 `memory.*` event_type
- Console v1.x ship 时 Memory 详情面板拉 `/v1/observability/events?event_type=memory.*` filter 后即有数据

### 其它 events consumer

- 不影响 — events 是 string event_type，未知 type 默认透传 / 渲染为 raw event

### contractv1.go 客户端用户

- `ObservabilityEvent` struct 字段不变（v0.7 → v0.8 add-only on `event_type` semantic values，not schema）
- 不需要代码更改即可消费 `memory.*` event

## Amendment (Phase 26 / v0.19.0 — add-only, 不溯改 D1-D4)

> ADR-031 (observability-hardening) 在 v0.19.0 推进了本 ADR 的两处预留。以 add-only Amendment 记录推进结果，**不溯改正文 D1-D4 + Trade-off + Rollback path**（ADR-014 D5）。本 ADR 仍 Accepted；D1-D4 best-effort emit + `broadcast::channel(1000)` 默认语义在默认配置下不变（分区默认关闭）。

- **events-replay-from-audit（兑现 §Trade-off `adr-021:115` `[SPEC-DEFER:phase-future.events-replay-from-audit]`）**：task-26.2 落 `core/src/data_plane/events.rs::replay_events_from_audit`——从 `AuditSink::list()`（`audit_log` `id ASC`）重建 memory state-op `ObservabilityEvent` 序（D3 字段映射：`memory.pin`/`memory.deprecate`/`memory.soft_delete`，pin/unpin 共享 `memory.pin`，payload `op` 区分），SSE 订阅经 `?since_ts=` 先回放再接续实时流，event_id `evt-audit-{id}` 供拼接边界去重。重放仍 best-effort 历史补偿（audit 是持久主路径，event 是实时副路径，D1 不变）；`indexing.*` 无 audit 持久源 → 重放 `[SPEC-DEFER:phase-future.indexing-event-persistence]` 如实延后。
- **event-bus 容量 / 分区（兑现 D4 + Rollback path `adr-021:153`「提容量或 partition channel」预见）**：task-26.3 落 `EventBus::from_config(EventBusConfig{capacity, partitioned})`——`event-bus-capacity`（`CF_EVENT_BUS_CAPACITY`，替换硬编码 `broadcast::channel(1000)`，复用 `with_capacity` seam，默认仍 1000）+ `event-bus-partition`（`CF_EVENT_BUS_PARTITION`，`memory.*` / `indexing.*` 分独立 broadcast channel，缓解 memory 高频挤占 indexing 的丢事件场景，默认不分区单 channel）。保守默认使 D1-D4 既有行为默认不变。
- **events-drain-timeout-config**：task-26.3 把 grpcclient phase-2 硬编码 `~100ms` drainTimeout 提为 `CONSOLE_EVENTS_DRAIN_TIMEOUT` 可配（默认 100ms，task-16.2 两阶段 long-poll 默认语义不变）。

详见 `docs/decisions/adr-031-observability-hardening.md`（D3/D4/D5 + §Ratification）+ `docs/releases/v0.19.0-evidence.md`。

## Amendment (Phase 31 / v0.24.0, 2026-06-03 — add-only, 正文不溯改)

Phase 31（ADR-036 governance-debt-cleanup）以 add-only 方式补齐观测一致性 + 更正一处 stale backlog，**不溯改正文**（ADR-014 D5）：

- **Go fallback memstore-event-emit parity（真实债，已修）**：task-31.1（PR #206）`internal/consoleapi/memstore.go` 的 `MemMemoryStore` Pin/Deprecate/SoftDelete/Unpin/HardDelete 经新 `MemStore.EmitEvent` sink 发 `memory.pin`/`memory.deprecate`/`memory.soft_delete`/`memory.pin`(op=unpin)/`memory.hard_delete` 入 fallback ring，与 workspace/job fallback + Rust 数据面 `core/src/data_plane/memory.rs` 对齐（event_type 取 Rust `audit_op_to_event_type` 权威映射）。Rust 侧不动（已交付）。`TestMemMemoryStore_EventParity` PASS。
- **event-bus partition/capacity 经核 Phase 26 已交付（更正，非新债）**：roadmap §4 旧列 `event-bus-partition`/`event-bus-capacity` 为开放 backlog，经核二者已于 **Phase 26 / ADR-031 D5 交付**（`core/src/data_plane/events.rs:24-203` `EventBusConfig`/`Partition`/`from_config` + `server.rs:602-603` 生产接线 + `TEST-26.3.1a/b/c`）。task-31.1 对其 **verify-only**（`cargo test data_plane::events` 6 passed）+ roadmap §4 add-only 更正剔除 stale 条目，**不重复实现**（ADR-013）。

依赖变更：纯 Go `internal/consoleapi` + Rust verify-only，0 新 dep。详见 ADR-036 Ratification + `docs/releases/v0.24.0-evidence.md`。
