# ADR `032`: `memory-ops-hardening`

**Status**: Accepted (2026-06-01。v0.20.0 closeout（task-27.3）据 task-27.1/27.2/27.3 真实非合成验证 ratify Proposed→Accepted，ADR-013。见 §Ratification。)
**Category**: 数据平面 / Memory 生命周期 / 协议契约演进（add-only）
**Date**: 2026-05-31
**Decided By**: 主 agent（ADR-012 自治）；tajiaoyezi ratification at v0.20.0 closeout
**Related**: ADR-022 (memory-is-pinned-field-amendment — 本 ADR 据其 §Trade-offs 三条 marker 推进：`pin_actor` / `[SPEC-DEFER:phase-future.memory-pinned-at-timestamp]` / `[SPEC-DEFER:phase-future.is-pinned-backfill-from-audit]`) / ADR-021 (memory-event-bus-bridge — `memory.pin` / `memory.unpin` event_type) / ADR-017 (console-contract-completion-22-endpoint — D2 destructive op X-Confirm 服务端兜底) / ADR-015 (console-contract-v1-compatibility — D5 字段冻结 amendment 路径) / ADR-016 (cross-process-rust-go-via-grpc-bridge) / ADR-008 (core-library-selection) / ADR-004 (local-first-privacy-baseline) / ADR-013 (禁伪造凭据红线) / ADR-014 (D1-D5，第十八次激活) / Phase 13 (memory-rest-surface — task-13.1/13.2) / Phase 17 (is-pinned-amendment — task-17.1)

## Context

Phase 13（v0.6 ship）落地 Memory 持久层 + 5 个 gRPC RPC（List / Get / Pin / Deprecate / SoftDelete）：`core/src/memory/store.rs::SqliteMemoryStore`（table `memory_items`，migration `core/migrations/0013_memory_items.sql`）+ `core/src/data_plane/memory.rs::MemoryServer`（thin proxy，Pin/Deprecate/SoftDelete 各 emit 一条 `AuditSink` event + 经 ADR-021 桥到 `EventBus`）。Phase 17（v0.10 ship）经 ADR-022 给 `MemoryItem` 加 `is_pinned bool`（proto field 10，`proto/contextforge/console_data_plane/v1/console_data_plane.proto:293`）+ `SqliteMemoryStore::set_pinned`（`store.rs:153`）。

ADR-022 §Trade-offs / Conscious limitations 显式记录了三条本可在 v0.10 引入、但当时刻意缩范围延后的硬化项：

1. **`pin_actor` 字段缺失**：ADR-022 决定「不引入 `pin_actor` 字段——谁 pin 的留 `MemoryOperation.actor` audit log 查（不污染 `MemoryItem` schema）」。但 audit log 当前只记 `chunk_ids=[memory_id]`（`core/src/data_plane/memory.rs:62`），并不记录调用 actor，因此「谁 pin 的」事实上**无处可查**；Console UI 详情面板若要显示「由谁置顶」必须有一手字段。

2. **`pinned_at` timestamp 缺失**：ADR-022 决定「不需要 `pinned_at` timestamp（如需历史溯源，查 `MemoryOperation.op_type=pin/unpin` audit log）」并标 `[SPEC-DEFER:phase-future.memory-pinned-at-timestamp]`。但 `set_pinned` 只 bump `updated_at_unix`（`store.rs:157`），任何非 pin 的 update（deprecate / soft_delete）都会覆盖它，故「何时置顶」同样不可恢复。

3. **Pin 是 toggle 而非显式 Pin/Unpin**：`PinMemoryRequest{ memory_id, bool pin }`（proto `:311`）一个 RPC 经 `pin` bool 兼任 pin 与 unpin（`memory.rs:207` 据 `req.pin` 分流 `MemoryPin` / `MemoryUnpin` audit op）。toggle 形态对调用方语义不显式（POST 空 body 在 console-api 兜底为 `pin=true`，`internal/consoleapi/handlers.go:536`），且无法表达「幂等 unpin」意图。

4. **hard-delete 缺失**：Memory 生命周期当前只有 soft-delete（`set_status(id, "soft_deleted")`，CHECK 约束 `active/deprecated/soft_deleted`，`migration 0013:14`）——行仍在表中、get-by-id 仍可取（`store.rs:135` 注释）。无「物理删除 / 不可恢复清除」路径；隐私基线（ADR-004）下用户「彻底忘掉某条 memory」的诉求无法满足。

5. **is_pinned backfill 不溯源**：ADR-022 记「不重写历史 audit log 推 is_pinned 当前态……接受作为字段 backfill 不溯源 trade-off `[SPEC-DEFER:phase-future.is-pinned-backfill-from-audit]`」。legacy memory items（v0.10 前）`is_pinned` 恒 `false`，即使 audit log 里有 `memory_pin` 事件也不反映。

本 ADR 记录上述五块 Memory ops 硬化的处理策略：actor + timestamp 记录、Pin/Unpin 显式拆分、hard-delete 策略、is_pinned 审计回填。**全部 add-only**（proto 新字段不动既有 tag、SQLite migration add-only column with default、新 RPC 不改既有 RPC 签名），不破坏 ADR-015 D5 冻结契约，且全部本地（ADR-004，0 网络 / 默认构建 0 新 dep）。

## Decision

Memory ops 硬化采用 **add-only 协议演进、显式生命周期 RPC、destructive 操作 X-Confirm 兜底、审计回填重建状态** 的策略：

### D1 — pin-actor + pinned-at-timestamp：add-only proto 字段 + 存储写穿（task-27.1）

`MemoryItem` 加两个 add-only proto 字段（序号在既有 field 10 之后追加，task-27.1 实施时按 proto 当前最大序号 +1 确定）：

- `string pinned_by`（pin-actor）——记录最近一次 pin 操作的调用 actor；unpin 时清空或保留末次（task-27.1 §5.2 定语义）。
- `int64 pinned_at_unix`（pinned-at-timestamp）——记录最近一次 pin 置真的 unix 秒；unpin 归 0。

`memory_items` 表经 add-only migration 加 `pinned_by TEXT NOT NULL DEFAULT ''` + `pinned_at_unix INTEGER NOT NULL DEFAULT 0`（与 `is_pinned` 列同 pattern，既有行 backfill 为缺省）。`set_pinned` 签名扩展为携带 actor（或新增 `set_pinned_with_actor`，task-27.1 §5.2 定）：pin=true 写 `pinned_by=actor` + `pinned_at_unix=now`；pin=false 归缺省。proto add-only 经 proto-freeze guard（`core/tests/proto_contract.rs` FROZEN 契约：只增字段、不删不改 tag）守护。

**理由**：actor / timestamp 是一手状态字段，比「查 audit log 推断」可靠（audit 当前不记 actor，§Context 1）；add-only `int64` / `string` 与既有 `created_at_unix` / `status` 风格一致；解除 ADR-022 §Trade-offs 的 `pin_actor` 与 `memory-pinned-at-timestamp` 两条 marker。

### D2 — Pin/Unpin 显式拆分 + hard-delete 策略（task-27.2）

- **Pin/Unpin 拆分**：新增显式 `Unpin(UnpinMemoryRequest)` RPC（add-only，与既有 `Pin` 并存）。既有 `Pin(PinMemoryRequest{memory_id, bool pin})` 签名**不动**（向后兼容；`pin=true` 仍有效）；新 `Unpin` 是 `pin=false` 的语义显式化 + 幂等。console-api 既有 `POST /v1/memory/{id}/pin` 不破坏；显式 unpin 路由 add-only。
- **hard-delete 策略**：新增 `HardDelete(HardDeleteMemoryRequest)` RPC（add-only）——物理删除行（`DELETE FROM memory_items WHERE memory_id=?`），不同于 soft-delete 的状态翻转。console-api `POST /v1/memory/{id}/hard-delete` 经既有 `confirmMiddleware`（ADR-017 D2：`X-Confirm: yes` header 或 `?confirm=true`，缺则 412）gated，与 deprecate / soft-delete 的 destructive 确认 pattern 一致。hard-delete emit 一条新 `MemoryHardDelete` audit op。

**理由**：拆分让调用方语义显式且 add-only 不破坏既有 toggle 调用；hard-delete 是隐私基线（ADR-004）下「不可恢复清除」的真实诉求，复用既有 X-Confirm destructive 兜底（ADR-017 D2）保证不被误触发，不引入新确认机制。

### D3 — is_pinned backfill from audit log（task-27.3）

提供一个确定性的 backfill 路径：对 legacy memory items（`is_pinned=false` 但 audit log 含 `memory_pin` / `memory_unpin` 事件），按 audit 事件时序重放、以末次 pin/unpin 事件重建当前 `is_pinned` 状态。backfill 是显式 opt-in 的一次性 reconcile（不在热路径自动跑），deterministic 单测可断言（构造 audit 序 → backfill → is_pinned 与重放末态一致）。

**理由**：解除 ADR-022 §Trade-offs `[SPEC-DEFER:phase-future.is-pinned-backfill-from-audit]`；audit log 已记 `memory_pin` / `memory_unpin`（`AuditOperation::MemoryPin/MemoryUnpin`，`core/src/memoryops/audit.rs:19-22`），重放它即可恢复历史；opt-in 一次性避免热路径成本。

### D4 — 默认构建不变 + 全 add-only + X-Confirm 复用

所有改动 **add-only**：proto 只增字段 / 新 RPC（proto-freeze guard 守护），SQLite migration add-only column with default（既有数据零迁移风险），既有 `Pin` / `Deprecate` / `SoftDelete` RPC 签名与行为不变。默认构建 0 新依赖（actor/timestamp/backfill 全用既有 `rusqlite` / `serde`，0 网络 ADR-004）。destructive hard-delete 复用既有 `confirmMiddleware`（ADR-017 D2），不引入新确认机制。本 ADR 不改既有三态 `status` 语义（`active/deprecated/soft_deleted` 仍 CHECK 约束），hard-delete 与 status 正交（物理删除 vs 状态翻转）。

## Consequences

- **Positive**: 「谁 / 何时置顶」成为一手可查字段（解除 ADR-022 两条 marker）；Pin/Unpin 语义显式且幂等；hard-delete 满足隐私基线「不可恢复清除」并经 X-Confirm 兜底防误触；is_pinned 可从 audit 回填（解除第三条 marker）；全 add-only 不破坏 v0.6-v0.19 既有 client（ADR-015 D5 冻结守住）；默认构建 0 新 dep / 0 网络（ADR-004）。
- **Negative / open**: hard-delete 物理删除不可恢复——靠 X-Confirm 兜底，但调用方误传 `confirm=true` 仍会删（属设计意图，与 deprecate/soft-delete 确认语义一致）；audit backfill 仅能重建有 audit 记录的 item（audit log 被裁剪 / 缺失的 legacy item 无法回填——如实记录为 backfill 覆盖率 caveat）；`pinned_by` actor 来源取决于调用链是否携带 actor（console-api 当前 source 写死 `"console-api"`，`memory.rs:64`——真实 per-user actor 取决于上游是否透传，task-27.1 §8 R 记录其 caveat）。
- **Ratification**: 本 ADR **Proposed**。task-27.1（actor+timestamp 真实写穿 round-trip）+ task-27.2（Pin/Unpin 拆分 + hard-delete 真实物理删除 + X-Confirm 412 兜底）通过后，于 v0.20.0 closeout（task-27.3）据真实非合成验证 ratify Proposed→Accepted（ADR-013：禁据合成 / 伪造 ratify）；某维度受阻则据「已达维度 ratify + 受阻维度如实记录」处理，不强 ratify。
- **Follow-ups**: ADR-022 §Trade-offs 三条 marker（`pin_actor` / `memory-pinned-at-timestamp` / `is-pinned-backfill-from-audit`）经本 phase 落地——以 ADR-022 add-only Amendment 记录（task-27.3 D5，不溯改 ADR-022 正文 D1-D5）；per-user actor 透传（console-api source 写死 `"console-api"` → 真实用户身份）`[SPEC-DEFER:phase-future.memory-actor-propagation]`；hard-delete 的级联清理（向量索引 / 引用该 memory 的 trace）`[SPEC-DEFER:phase-future.memory-hard-delete-cascade]`。

## Ratification（v0.20.0 / task-27.3，2026-06-01）

v0.20.0 closeout（task-27.3）据 task-27.1/27.2/27.3 的**真实非合成验证** ratify `Proposed → Accepted`（ADR-013：禁据合成 / 伪造 ratify）。逐 D 项真实依据：

- **D1（pin-actor + pinned-at-timestamp）— Accepted**：`MemoryItem` add-only proto field 11 `pinned_by` + field 12 `pinned_at_unix` + migration `0017`（`ensure_pin_actor_columns` 守护幂等 ALTER）+ `set_pinned_with_actor` 写穿 真实落地；`cargo test -p contextforge-core --lib memory::store` **15 passed**——pin=true 写 `pinned_by=actor`+`pinned_at_unix>0`、pin=false 归 `''`/0、get+list round-trip 投影、`pinned_at` 独立于 `updated_at`（TEST-27.1.2）；`data_plane::memory` pin RPC 传 `"console-api"` + `memory_to_pb` 投影（TEST-27.1.3）；`proto_contract` MemoryItem field 11/12 superset freeze（TEST-27.1.1）。actor 来源 = console-api source（真实 per-user 透传 `[SPEC-DEFER:phase-future.memory-actor-propagation]` 如实延后）。
- **D2（Pin/Unpin 拆分 + hard-delete）— Accepted**：proto add-only `rpc Unpin`/`rpc HardDelete` + 4 message + `store.hard_delete`（`DELETE FROM memory_items`，0 行 NotFound）+ `AuditOperation::MemoryHardDelete`（event_type `memory.hard_delete`）真实落地；`data_plane::memory` **14 passed**——unpin 显式 + 幂等 + emit `MemoryUnpin`（TEST-27.2.3）/ hard_delete 物理删除后 get None + emit `MemoryHardDelete`（TEST-27.2.2）；`go test ./internal/consoleapi/...` console-api `hard-delete` 缺 X-Confirm → **412**、带 confirm → 204、后续 GET → **404**（物理删除坐实）+ `unpin` → 204 + 既有 destructive 412 不退化（TEST-27.2.4）；`proto_contract` MemoryService superset freeze（TEST-27.2.1）。复用既有 `confirmMiddleware`（ADR-017 D2），不引入新确认机制。
- **D3（is_pinned backfill from audit）— Accepted**：`SqliteMemoryStore::reconcile_is_pinned_from_audit(&[AuditLogEntry])` 真实落地；`memory::store` 15/15——按 memory_id 分组 `memory_pin`/`memory_unpin` 事件、last 胜、仅修正不一致的存在行（不臆造无事件 item，TEST-27.3.1）。opt-in 一次性 reconcile（非热路径）；仅修正 `is_pinned`（+ updated_at），不伪造历史 actor/timestamp（legacy audit 未记 actor——这正是 §Context 1 的缺口）。backfill 覆盖率 caveat：仅能重建有 audit 记录的 item（被裁剪 / 缺失的 legacy item 无法回填，如实记录）。
- **D4（默认构建不变 + 全 add-only + X-Confirm 复用）— Accepted**：proto 全 add-only（既有 field 1-10 + 5 RPC 不动，proto-freeze guard 过）；SQLite migration add-only column with default（既有行缺省 backfill，0 迁移风险）；`cargo test --workspace` + `go test ./...` 全 PASS、既有 `Pin`/`Deprecate`/`SoftDelete` + `confirmMiddleware` 行为不变；0 新依赖（`rusqlite`/`serde`，无 `Cargo.toml`/`Cargo.lock` 改动）+ 0 网络（ADR-004 / ADR-008 无 Amendment）。

证据见 `docs/releases/v0.20.0-evidence.md`。

### ADR-022 add-only Amendment（推进 §Trade-offs 三条 marker，不溯改正文 D1-D5，ADR-014 D5）

ADR-022 §Trade-offs / Conscious limitations 三条刻意缩范围延后的 marker 经本 phase 落地，以 ADR-022 add-only Amendment 记录推进结果（不溯改 ADR-022 正文 D1-D5 + §Trade-offs）：

- `pin_actor` 不引入 → task-27.1 `pinned_by`（field 11）落地。
- `[SPEC-DEFER:phase-future.memory-pinned-at-timestamp]` → task-27.1 `pinned_at_unix`（field 12）落地。
- `[SPEC-DEFER:phase-future.is-pinned-backfill-from-audit]` → task-27.3 `reconcile_is_pinned_from_audit` 落地。

详见 `docs/decisions/adr-022-memory-is-pinned-field-amendment.md` §Amendment (Phase 27 / v0.20.0)。

## Amendment (Phase 40 / v0.33.0) — memory-actor-propagation 入口透传维度兑现 (add-only)

> add-only Amendment（不溯改本 ADR D-body / Ratification (v0.20.0)，ADR-014 D5）。承本 ADR §D1 — pin actor 做成 store first-class 字段（`set_pinned_with_actor` + `MemoryItem.pinned_by=11`）时自记的入口透传债 `[SPEC-DEFER:phase-future.memory-actor-propagation]`。

Phase 40 / v0.33.0（ADR-045 D1）兑现 pin actor 的**入口到 store 透传链**：此前 `pin()` RPC 只能硬编码 actor `"console-api"`（因 `PinMemoryRequest` 无 actor field、Go `MemoryClient.Pin(id,pin)` 无 actor 参数、`handleMemoryPin` 不读调用方标识）。task-40.1（PR #257）补：

- `PinMemoryRequest` add-only `string actor = 3`（既有 `memory_id=1` / `pin=2` 字段号冻结，ADR-015 D1）+ `buf generate`；
- Go `MemoryClient.Pin(id,pin)` → `Pin(id,pin,actor)`（interface + `memoryClient` / `MemMemoryStore` / `degradedMemory` 三实现）+ `grpcclient` 填 `pb.PinMemoryRequest.Actor` + `handleMemoryPin` 读 `r.Header.Get("X-Actor")`（缺省空串）；
- Rust `pin()` `set_pinned_with_actor(.., if req.actor.is_empty() { "console-api" } else { req.actor.as_str() })`（空回落 byte-equiv）。

console 部署在设 `X-Actor` / `X-Forwarded-User` 的 auth 代理后可把 pin/unpin 归因真实调用方（写入既有 `pinned_by`）。**诚实校正（ADR-013）**：本轮交付**调用方透传**（actor 取自 header、未做认证校验）；**认证身份**（校验为已认证 auth subject）须 console-api 鉴权层 → 续延后 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`；其它 memory RPC 的 actor 透传续 `[SPEC-DEFER:phase-future.memory-actor-all-rpc]`。`handleMemoryPin` 宽松 body 契约（ADR-022 D2）保持不变。验证 TEST-40.1.1/40.1.2/40.1.3/40.1.4。详见 ADR-045 Ratification (v0.33.0) + `docs/releases/v0.33.0-evidence.md`。
