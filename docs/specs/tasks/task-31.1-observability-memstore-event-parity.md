# Task `31.1`: `observability-memstore-event-parity — Go fallback MemMemoryStore memory ops 发 memory.* 事件入 fallback ring（与 workspace/job + Rust 路径对齐）+ event-bus partition/capacity verify-only 校正`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 31 (governance-debt-cleanup)
**Dependencies**: Phase 26（observability-hardening：event-bus partition/capacity 经核已交付，`core/src/data_plane/events.rs` `EventBusConfig`/`Partition`/`from_config` + `server.rs:602-603` 生产接线 + `TEST-26.3.1a/b/c`）/ Phase 27（ADR-032 D2：`Unpin`/`HardDelete` 物理删除路径）/ ADR-021（event-bus / memory-bus-bridge：Rust `MemoryServer` 已发 `memory.*` 事件，本 task 补 Go fallback parity + event-bus partition/capacity 已交付的 add-only 更正记录）/ ADR-031（event-bus 容量·分区 D5）/ ADR-004（默认行为 + 既有契约不变）/ ADR-013（禁伪造红线）/ ADR-012（main-agent-governance-autonomy）/ ADR-014 D1-D5（第二十二次激活）

## 1. Background

ContextForge 观测事件有两条路径：(1) Rust 数据面 `MemoryServer`（`core/src/data_plane/memory.rs:52-106` `emit_audit_and_event` + `build_memory_event`）在 memory 状态变更时发 `memory.pin` / `memory.deprecate` / `memory.soft_delete` / `memory.unpin`（与 `memory.pin` 共 event_type，payload `op` 区分）/ `memory.hard_delete` 到 `EventBus`；(2) Go console-api fallback `internal/consoleapi/memstore.go` 的内存态 `emitEvent` helper（`:100-115`）向上限 1000 的环形缓冲追加 `ObservabilityEvent`，供 `GET /v1/observability/events` 在无 Rust 数据面接线时兜底返回。

债项核查（ADR-013 诚实）发现两类性质截然不同的项，合并在本 task 处理：

- **A2 memstore-event-emit（真实债，Go fallback 侧）**：Go fallback 的 `emitEvent` 已在 workspace / job 变更被调用（`CreateWorkspace:183` / `UpdateWorkspaceConfig:232` / `EnqueueJob:260` / `CancelJob:308`），但在 memory 变更路径从未被调用——`MemMemoryStore.Pin`（`:590-603`）/ `Deprecate`（`:605-616`）/ `SoftDelete`（`:618-629`）/ `Unpin`（`:631-645`）/ `HardDelete`（`:649-657`）均不发事件。结果是：在 Go fallback 模式下，memory 变更对 `GET /v1/observability/events` 不可见，与 workspace/job 行为不一致、也与 Rust 路径不对齐。这是本 task 的真实修复。
- **A1 event-bus partition/capacity（经核 Phase 26 已交付，非债）**：roadmap §4 backlog（约 line 230 / 236）仍把 `event-bus-partition` 与 `event-bus-capacity` 列为待办，但经核查二者已于 Phase 26 / ADR-031 D5 交付——`core/src/data_plane/events.rs:24-203`（`EventBusConfig` 容量/分区 + `Partition` + `from_config`）生产接线于 `server.rs:602-603`，并有 `TEST-26.3.1a/b/c`（`events.rs:549-605`）覆盖。本 task 对其为 **verify-only**（确认既有测试常绿）+ 在 roadmap §4 做 add-only 更正记录其经核 Phase 26 已交付，不重复实现（ADR-013 禁伪造完成、禁重复造轮子）。

## 2. Goal

让 Go fallback `MemMemoryStore` 的五个 memory 写操作（Pin / Deprecate / SoftDelete / Unpin / HardDelete）在成功变更后向 fallback ring 发对应 `memory.*` 事件，达到 (a) 与同进程 workspace/job 路径一致、(b) 与 Rust 数据面 `memory.*` event_type 语义对齐；同时对 event-bus partition/capacity 做 verify-only 确认并在 roadmap §4 做 add-only 更正。

pass bar：Go 单测断言 fallback 模式下 `Pin`（及其余四个写操作）后 ring 增长且 event_type 命名与 Rust 对齐；Rust 侧 `TEST-26.3.1a/b/c` 保持常绿（本 task 不改 Rust 源码）；roadmap §4 add-only 更正记录落地；默认行为 / proto / 既有契约不变；ADR-014 D2 lint 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 修改 `internal/consoleapi/memstore.go`——使 `MemMemoryStore.Pin` / `Deprecate` / `SoftDelete` / `Unpin` / `HardDelete` 在成功变更后调用 fallback 的事件发射，向 ring 追加 `memory.pin` / `memory.deprecate` / `memory.soft_delete` / `memory.unpin` / `memory.hard_delete`（event_type 与 Rust `audit_op_to_event_type` 对齐：`Unpin` 与 `Pin` 共 `memory.pin`、payload 以 `op` 区分；`HardDelete` 独立 `memory.hard_delete`）。发射为 best-effort，不改变各写操作的返回契约（观测 != 权威）。
- 新增 Go 单测（`internal/consoleapi/` 下既有测试文件或新增 `_test.go`）——断言 fallback 模式下 `Pin` 后 ring 增长，五个写操作各发对应 event_type，未命中项不发。
- event-bus partition/capacity **verify-only**：确认 `core/src/data_plane/events.rs` 既有 `TEST-26.3.1a/b/c` 常绿（`cargo test`），不改 Rust 源码。
- `docs/roadmap.md` §4 add-only 更正记录：注明 `event-bus-partition` / `event-bus-capacity` 经核 Phase 26 / ADR-031 D5 已交付（add-only 行，不改既有正文，ADR-014 D5）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- Rust 数据面 `MemoryServer` 的 `memory.*` 发射——已交付，本 task 不触碰 Rust 侧 `core/src/data_plane/memory.rs` 源码 [SPEC-OWNER:phase-13.memory-bus-bridge]
- event-bus partition/capacity 的重新实现——经核 Phase 26 已交付，本 task 为 verify-only + roadmap add-only 更正，不重复实现 [SPEC-OWNER:task-26.3-event-bus-partition-capacity]
- fallback ring 持久化 / 跨进程共享 / SSE 实时推送 fallback——Rust 数据面已是权威观测路径，fallback ring 为兜底 [SPEC-DEFER:phase-future.fallback-ring-persistence]
- 真实 outward-facing tag / release——由 task-31.4 closeout 经用户授权处理 [SPEC-OWNER:user-authorized-release]

## 4. Actors

- 主 agent（ADR-012 自治）
- `internal/consoleapi/memstore.go`（Go console-api fallback：`MemStore.emitEvent` ring + `MemMemoryStore` 五个 memory 写操作）
- `GET /v1/observability/events`（fallback 模式下从 ring 读事件的消费方）
- `core/src/data_plane/events.rs`（Rust event-bus：partition/capacity，verify-only 对象）
- `core/src/data_plane/memory.rs`（Rust `MemoryServer`，已发 `memory.*`，对齐参照，本 task 不改）

## 5. Behavior Contract

### 5.1 Required Reading

- `internal/consoleapi/memstore.go:100-115`（`emitEvent` helper：追加 `ObservabilityEvent` 入 ring + 1000 上限裁剪）
- `internal/consoleapi/memstore.go:183`（`CreateWorkspace` 已调 `emitEvent("workspace.created", …)`——workspace 侧 parity 参照）
- `internal/consoleapi/memstore.go:590-657`（`MemMemoryStore` 的 `Pin:590-603` / `Deprecate:605-616` / `SoftDelete:618-629` / `Unpin:631-645` / `HardDelete:649-657`——均**未**发事件，本 task 的修复点）
- `core/src/data_plane/memory.rs:52-106`（Rust `emit_audit_and_event` + `audit_op_to_event_type` + `build_memory_event`：event_type 命名权威——`MemoryPin|MemoryUnpin → memory.pin`、`MemoryDeprecate → memory.deprecate`、`MemorySoftDelete → memory.soft_delete`、`MemoryHardDelete → memory.hard_delete`；**Rust 已交付，本 task 不改**）
- `core/src/data_plane/events.rs:24-203`（`EventBusConfig` 容量/分区 + `Partition` + `from_config`——Phase 26 交付，verify-only）+ `:549-605`（`TEST-26.3.1a/b/c`——保持常绿）
- `docs/roadmap.md` §3.13 + §4 backlog（event-bus-partition / event-bus-capacity 约 line 230 / 236 的待办项——add-only 更正对象）+ ADR-021 / ADR-031

### 5.2 关键设计 — Go fallback memory ops 发 memory.* 入 ring（与 workspace/job + Rust 对齐）

`MemMemoryStore` 在变更成功（取得锁、命中 item、写回 `s.items`）后，向 fallback ring 发对应 `memory.*` 事件。event_type 命名取 Rust `audit_op_to_event_type` 的权威映射，保持两路径语义一致：

- `Pin(id, true/false)` → `memory.pin`（payload `op` 区分 pin/unpin，与 Rust `build_memory_event` 一致）
- `Deprecate` → `memory.deprecate`
- `SoftDelete` → `memory.soft_delete`
- `Unpin` → `memory.pin`（与 `Pin` 共 event_type，payload `op=unpin` 区分；对齐 Rust `MemoryPin | MemoryUnpin → memory.pin`）
- `HardDelete` → `memory.hard_delete`（独立 event_type，区分物理删除 vs `soft_delete` 状态翻转，对齐 ADR-032 D2）

发射为 best-effort 且在写回成功后进行：错误路径（item 未命中返回 `ErrNotFound`）不发事件；事件发射本身不改变各写操作的返回值或错误契约（观测 != 权威，对齐 Rust `emit_audit_and_event` 的 SendError 吞掉语义）。`emitEvent` 既有 1000 上限裁剪不变。

event-bus partition/capacity 为 verify-only：经核 Phase 26 / ADR-031 D5 已交付（`events.rs` `from_config` + `server.rs:602-603` 接线 + `TEST-26.3.1a/b/c`），本 task 确认其常绿即可，不改 Rust 源码；roadmap §4 以 add-only 行记录其经核 Phase 26 已交付的更正。

### 5.3 不变量

- 0 新代码依赖（纯 Go `internal/consoleapi` 改动 + Rust verify-only；无 Cargo / go.mod 新增 direct dep；ADR-008 无 Amendment）。
- 五个 memory 写操作的返回值 / 错误契约不变（ADR-004；事件发射 best-effort，不影响 `Pin`/`Deprecate`/`SoftDelete`/`Unpin`/`HardDelete` 的成功 / 失败语义）。
- Rust 数据面 `core/src/data_plane/memory.rs` + `events.rs` 源码不改（已交付路径不退化）。
- fallback ring 1000 上限裁剪行为不变（`emitEvent:112-114`）。
- event_type 命名与 Rust `audit_op_to_event_type` 一致（两路径观测语义对齐）。
- 默认构建 0-network / 0-dep baseline 不变（ADR-004）。

## 6. Acceptance Criteria

- [x] AC1（memstore-event-emit Go fallback parity）: Go fallback `MemMemoryStore` 的 `Pin`/`Deprecate`/`SoftDelete`/`Unpin`/`HardDelete` 经 `SetEventSink`(wired to `MemStore.EmitEvent` in `console_api_serve.go`) 向 fallback ring 发 `memory.pin`/`memory.deprecate`/`memory.soft_delete`/`memory.pin`(op=unpin)/`memory.hard_delete` 事件；event_type 命名与 Rust `audit_op_to_event_type` 对齐；五写操作返回 / 错误契约不变（error path 不发事件、无 sink 不 panic）（🟢 deterministic）— verified by TEST-31.1.1（`TestMemMemoryStore_EventParity` PASS）
- [x] AC2（event-bus partition/capacity verify-only + roadmap add-only 更正）: `cargo test -p contextforge-core data_plane::events` → **6 passed**（含 `TEST-26.3.1a/b/c`，经核 Phase 26 / ADR-031 D5 已交付，本 task 不改 Rust 源码、不重复实现）；`docs/roadmap.md` §4 add-only 更正记录已于规划阶段（PR #196，roadmap line 294）落地——本实现 task verify-only 确认其 verify-only 结论成立（🟢 deterministic）— verified by TEST-31.1.2
- [x] AC3（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by TEST-31.1.3（PASS）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-31.1.1 | Go fallback `MemMemoryStore` Pin/Deprecate/SoftDelete/Unpin/HardDelete 发 `memory.*` 入 fallback ring（断言 5 写操作各发对应 event_type + Pin/Unpin 共 memory.pin + error path 不发 + 无 sink 不 panic + 返回契约不变） | `internal/consoleapi/memstore.go` + `internal/consoleapi/memstore_test.go` | Done (PASS) |
| TEST-31.1.2 | event-bus partition/capacity verify-only（`cargo test data_plane::events` 6 passed 含 `TEST-26.3.1a/b/c`）+ roadmap §4 add-only 更正（规划阶段 PR #196 line 294 已落地，经核 Phase 26 已交付，不重复实现） | `core/src/data_plane/events.rs`（verify-only）+ `docs/roadmap.md` | Done (PASS) |
| TEST-31.1.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威） | `scripts/spec_drift_lint.sh` | Done (PASS) |

## 8. Risks

- **R1（低）event_type 命名与 Rust 漂移**：Go fallback 若用了与 Rust `audit_op_to_event_type` 不一致的 event_type，则两路径观测语义分裂。
  - **缓解**：§5.2 明确取 Rust 权威映射（`Unpin` 共 `memory.pin`、`HardDelete` 独立）；TEST-31.1.1 断言具体 event_type 字符串。
- **R2（低）误改 Rust 已交付路径**：A1 event-bus 与 Rust `MemoryServer` 已交付，若误改会致退化。
  - **缓解**：§3 范围外明确 Rust 侧不触碰；A1 verify-only 仅跑 `cargo test` 确认 `TEST-26.3.1a/b/c` 常绿；本 task 改动限 Go `memstore.go` + 测试 + roadmap add-only 行。
- **R3（低）事件发射在锁内 / 锁外位置**：`emitEvent` 与 memory 写操作的锁交互若处理不当可致死锁或竞态。
  - **缓解**：参照 workspace 路径（`CreateWorkspace:183` 在持锁路径内调 `emitEvent`，`emitEvent` 不再取 `s.mu`）；memory 写操作沿用同 pattern；`go test -race` 验证。

## 9. Verification Plan

```bash
# 1. AC1 — Go fallback memory ops 发 memory.* 入 ring（含 -race）
go test ./internal/consoleapi/... -run TestMemMemoryStore -race
go test ./...

# 2. AC2 — event-bus partition/capacity verify-only（Rust 既有测试常绿，不改源码）
cargo test -p contextforge-core data_plane::events
cargo test --workspace
#    roadmap §4 add-only 更正行经人工核 + D2 lint（下一步）

# 3. AC3 — D2 lint（触及行 0 未标注命中，CI spec-lint 权威）
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **真实证据回填**：各 AC 的真实 `go test` / `cargo test` 结果与 race 结论在实施后于 §10 据真实跑出回填（待实测回填，ADR-013 不伪造）。

## 10. Completion Notes (s2v 6 项标准)

- **Status**: Done。
- **实际改动文件**：
  - `internal/consoleapi/memstore.go`——新增 `MemStore.EmitEvent`（thread-safe 公共 sink，取 s.mu）+ `MemMemoryStore.emit` 可选 sink 字段 + `SetEventSink` + `emitMemoryEvent` helper；`Pin`/`Deprecate`/`SoftDelete`/`Unpin`/`HardDelete` 成功变更后发 `memory.pin`/`memory.deprecate`/`memory.soft_delete`/`memory.pin`(op=unpin)/`memory.hard_delete`（event_type 对齐 Rust `audit_op_to_event_type`；best-effort，error path 不发，返回契约不变）。
  - `internal/cli/console_api_serve.go`——`buildDeps` fallback 分支 `memMem.SetEventSink(store.EmitEvent)` 接线（memory ops 入共享 ring）。
  - `internal/consoleapi/memstore_test.go`——`TestMemMemoryStore_EventParity`（5 写操作各发对应 event_type，Pin/Unpin 共 memory.pin、error path 不发、无 sink 不 panic、返回契约不变）。
  - `core/src/data_plane/events.rs` / `memory.rs`——**不改**（verify-only / 已交付参照）。`docs/roadmap.md` §4 更正于规划 PR #196 已落地（line 294），本 task 不重复改。
- **§9 Verification 实测证据**：
  - AC1：`go test ./internal/consoleapi/ -run TestMemMemoryStore_EventParity` PASS；`go test ./...` 不退化（`-race` 需 cgo，本机无 C 编译器跳过，锁序 memMem.mu→store.mu 一致无死锁；CI 权威）。
  - AC2：`cargo test -p contextforge-core data_plane::events` → **6 passed**（含 `TEST-26.3.1a/b/c`，verify-only Phase 26 已交付）；roadmap §4 更正已 PR #196 落地。
  - AC3：`gofmt` 改动文件 staged blob LF（autocrlf=true，0 CR，CI gofmt-clean；本机 CRLF 为已知 false positive）+ `go vet` 0；spec-lint `--touched origin/master` 0 未标注命中。
