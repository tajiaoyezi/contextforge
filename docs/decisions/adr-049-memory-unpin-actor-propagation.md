# ADR `049`: `memory-unpin-actor-propagation`

**Status**: Accepted（v0.37.0 / task-44.3 closeout 据真实 CI 逐 D ratify；D1/D2/D3 unit 🟢 Accepted，D4 默认 byte-equiv 🟢 + 认证身份/其余 3 RPC 🔴 honest-defer 据实记录——见 §Ratification）

**Category**: 治理债清理 / memory actor 透传闭环 / observability 归因
**Date**: 2026-07-01
**Decided By**: 主 agent（ADR-012 自治，用户全权授权）
**Related**: ADR-045（governance-debt-cleanup-3 — D1 memory pin actor 透传 task-40.1，本 ADR 兑现其 `memory-actor-all-rpc` backlog 的 Unpin 子项 + audit/event source 归因，add-only Amendment）/ ADR-032（memory-ops-hardening — D1 pin actor/timestamp first-class store 字段，本 ADR 以 add-only Amendment 记 unpin 透传维度兑现）/ ADR-021（memory-event-bridge — emit_audit_and_event + build_memory_event 镜像源，本 ADR 加 actor 参数）/ ADR-022（D2 — memory pin lenient body 契约保持不改）/ ADR-015（console-data-plane proto 契约 — UnpinMemoryRequest add-only actor=2）/ ADR-004（local-first-privacy-baseline — 默认行为 byte-equiv + 空 actor 回落 "console-api"）/ ADR-008（dep add-only — Phase 44 = 0 新依赖）/ ADR-013（禁伪造红线 — actor 真实透传 + audit/event source 真实归因非合成；认证身份据实延后；deprecate/softdelete/harddelete 据实延后不强行扩面）/ ADR-012（main-agent-governance-autonomy — 用户全权授权 tag/release）/ ADR-014（D1-D5，第三十五次激活）/ roadmap §3.26 + §4

## Context

ContextForge 截至 Phase 43（governance-debt-cleanup-4, Done / v0.36.0）。Phase 40 task-40.1（ADR-045 D1）给 `pin` RPC 加了 actor 透传（`X-Actor` header → `PinMemoryRequest.actor` → `set_pinned_with_actor` 写 `pinned_by`），但对称的 `unpin` RPC 漏了——unpin handler（`core/src/data_plane/memory.rs:298`）硬编码 `"console-api"`。roadmap 行 556（Phase 40 closeout）把 `memory-actor-all-rpc`（其它 memory RPC 的 actor 透传）列为新增 backlog。

**grounding 发现的真实价值点（改变范围设计，ADR-013）**：`set_pinned_with_actor(memory_id, pinned=false, actor)` 在 `pinned=false` 时**丢弃 actor**（`core/src/memory/store.rs:192-196` 把 `pinned_by` 清空为 `""`、`pinned_at_unix` 清 0——unpin 语义是清除 pin 快照）。故单纯让 unpin handler 透传 actor 给 store 是"空透传"（接口对称但 actor 无落点）——违 ADR-013（honest over padding，不为凑数做无落点的透传）。

**真实价值在 audit/event 归因**：unpin handler 调 `emit_audit_and_event(AuditOperation::MemoryUnpin, &id)`（memory.rs:300），而 `emit_audit_and_event`（:52）当前**不接受 actor**、audit `source` 硬编码 `"console-api"`（:59）、`build_memory_event`（:103）也不携 actor。让 actor 进入 audit log + event stream，console 部署在 auth 代理后时 unpin 操作可归因到真实调用方——这是真实可观测价值，非空透传。

**关键不对称（核实）**：pin handler（:231-237）虽透传 actor 到 store `pinned_by`，但其 `emit_audit_and_event`（:239-246）同样不携 actor（audit/event source 仍硬编码 "console-api"）。故 pin 的 audit/event 也未归因——本 phase 顺带闭环（add-only，pin handler 传 actor 给升级后的 emit_audit_and_event）。

## Decision

unpin actor 透传采用 **「完整闭环：unpin handler 透传 + emit_audit_and_event 加 actor 参数（audit/event source 归因）+ pin 顺带闭环 + Go 透传链 + 默认 byte-equiv」** 策略，分 4 个决策点：

### D1 — proto add-only + Rust unpin handler 透传（task-44.1）🟢

(a) `UnpinMemoryRequest`（proto:369-371）add-only `string actor = 2`（既有 `memory_id = 1` 字段号冻结，ADR-015）；buf generate 重生 Go/Rust binding。
(b) Rust `unpin()` handler（memory.rs:287-302）：`req.into_inner()` 保留 actor；`let actor = if req.actor.is_empty() { "console-api" } else { req.actor.as_str() };`（镜像 pin :231-235）；传 `set_pinned_with_actor(&id, false, actor)`（store 在 pinned=false 时丢弃 actor，接口对称）+ 传 `emit_audit_and_event(MemoryUnpin, &id, actor)`（新签名，**核心价值**）。

**理由**：proto add-only（field 2，既有 1 冻结）+ 空 actor 回落 `"console-api"` byte-equiv（ADR-004）。store 丢弃 actor 是 unpin 语义（清 pin 快照），接口对称无害；真实落点在 audit/event（D2）。

### D2 — emit_audit_and_event 加 actor 参数 + pin 顺带闭环（task-44.1）🟢

(a) `emit_audit_and_event(&self, op, memory_id, actor: &str)`（memory.rs:52 加第 4 参）：audit `source`（:59）由硬编码 `"console-api"` 改 `actor`；`build_memory_event(op, memory_id, actor)`（:103 加第 3 参）的 `PbEvent.source` 由 `"contextforge-core"` 改 `actor`（注：既有 audit source 与 event source 不同——audit 是 "console-api"，event 是 "contextforge-core"；本 phase 统一为 actor，空回落各自保持原值 byte-equiv）。
(b) pin handler（:239-246）传 `actor`（顺带闭环——pin 的 audit/event 也归因）。
(c) deprecate/softdelete/harddelete handler 传 `"console-api"`（byte-equiv，这三 RPC 的真实 actor 透传统续延后 D4）。

**理由**：emit_audit_and_event 是所有 memory RPC 共用的 observability 桥（task-15.2 / ADR-021 D1）。加 actor 参数是 add-only（既有调用传固定值 byte-equiv）。pin 顺带闭环消除"pin 透传到 store 但 audit/event 不归因"的残余不对称。空 actor → audit source "console-api" / event source "contextforge-core"（各自 byte-equiv）。

### D3 — Go 透传链（task-44.1）🟢

(a) `MemoryClient` interface（types.go:130）`Unpin(memoryID string) error` → `Unpin(memoryID, actor string) error`。
(b) `memoryClient.Unpin`（grpcclient.go:743）加 `actor string` 形参 + `pb.UnpinMemoryRequest{MemoryId: id, Actor: actor}`。
(c) `handleMemoryUnpin`（handlers.go:610-626）读 `r.Header.Get("X-Actor")`（:559 pin 范式）+ 传 `Unpin(id, actor)`。
(d) `MemMemoryStore.Unpin`（memstore.go:703）签名同步加 `actor string`（fallback 实现，actor 可忽略——fallback 无 audit/event）。
(e) degraded stub（console_api_serve_degraded.go，如有 Unpin）同步签名。

**理由**：Go 透传链镜像 task-40.1 pin 的 4 处改动（interface/grpcclient/handler/memstore）。空 actor（无 X-Actor header）→ 空串 → Rust 回落 "console-api" byte-equiv。

### D4 — 默认 byte-equiv + honest-defer 边界（all tasks）🟢 / 🔴

- 空 actor → audit source `"console-api"` / event source `"contextforge-core"`（各自 byte-identical 现状，ADR-004）。
- deprecate/softdelete/harddelete 传 `"console-api"`（byte-equiv）。
- 认证身份（把 X-Actor 校验为已认证 auth subject）🔴 honest-defer `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`（须 console-api 鉴权层）。
- deprecate/softdelete/harddelete 真实 actor 透传🔴 honest-defer `[SPEC-DEFER:phase-future.memory-actor-all-rpc]`（Deprecate/SoftDelete 需 7 层+新 migration；HardDelete 须 audit 层重设计——本 phase 仅做 emit_audit_and_event actor 参数共用基础，这三 RPC 未来可顺带受益）。

**理由**：ADR-013 禁伪造——本 phase 交付真实有落点的 unpin actor 透传（audit/event source）+ pin 顺带闭环，非空透传；认证身份 + 其余 3 RPC 据实延后不强行扩面（roadmap §3.17/§3.22 "据实排小不凑数"）。

## Consequences

- **Positive**: unpin 操作的 audit log + event stream 现归因到真实调用方（console 部署在 auth 代理后时，unpin 不再恒记 "console-api"）；pin 的 audit/event 顺带闭环（消除 pin 透传到 store 但 audit/event 不归因的残余不对称）；`emit_audit_and_event` 加 actor 参数为 deprecate/softdelete/harddelete 未来透传铺路（共用基础）；默认 byte-equiv（空 actor 各自回落原值）；0 新 dep / 0 migration / proto add-only（ADR-004/008/015）。
- **Negative / open**：认证身份 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`（须 console-api 鉴权层）；deprecate/softdelete/harddelete actor 透传 `[SPEC-DEFER:phase-future.memory-actor-all-rpc]`（须 7 层+migration / audit 重设计）。
- **Ratification**: 本 ADR **Proposed**。task-44.1 通过后于 v0.37.0 closeout（task-44.3）据真实 CI 逐 D ratify Proposed→Accepted。
- **Follow-ups**: 认证身份 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`；deprecate/softdelete/harddelete actor 透传 `[SPEC-DEFER:phase-future.memory-actor-all-rpc]`。

## Ratification（v0.37.0 / task-44.3）

本 ADR 于 v0.37.0 closeout（task-44.3）据 task-44.1 真实 CI（4 门绿：cargo-test / go-test / lint / spec-lint）逐 D ratify Proposed→Accepted。各 D 真实依据：

- **D1（proto add-only + Rust unpin 透传）→ Accepted 🟢**：task-44.1（PR #280，master @ `8f6e94f`）`UnpinMemoryRequest` add-only `actor=2`（既有 memory_id=1 冻结）+ buf generate；Rust unpin handler（memory.rs）透传 actor（`let actor = if req.actor.is_empty() { "console-api" } else { req.actor.as_str() };` 镜像 pin）+ `set_pinned_with_actor(id, false, actor)`（store 丢弃，接口对称）+ `emit_audit_and_event(MemoryUnpin, id, &req.actor)`（新签名）。
- **D2（emit_audit_and_event actor 参数 + pin 顺带闭环）→ Accepted 🟢**：task-44.1 `emit_audit_and_event` 加 `actor: &str` 参数（audit source 非空用 actor / 空回落 "console-api"）+ `build_memory_event` 加 actor 参数（event source 非空用 actor / 空回落 "contextforge-core"）；pin handler 顺带传 `&req.actor`（消除 pin audit/event 不归因残余不对称）；deprecate/softdelete/harddelete 传 `""` byte-equiv。`test_44_1_1_unpin_actor_in_event_source`（"bob"）+ `test_44_1_2_pin_actor_in_event_source`（"alice"）+ `test_44_1_3_empty_actor_event_source_byte_equiv`（"contextforge-core"）绿。
- **D3（Go 透传链）→ Accepted 🟢**：task-44.1 `MemoryClient.Unpin(id)` → `Unpin(id, actor)` + grpcclient `pb.UnpinMemoryRequest{MemoryId, Actor}` + `handleMemoryUnpin` 读 `X-Actor`（:559 范式）+ `MemMemoryStore.Unpin(id, _actor)` + degraded fallback 签名同步 + gofmt 对齐。`TestTask441_UnpinActorPropagationWired` 源码 grep 绿。
- **D4（默认 byte-equiv + honest-defer）→ Accepted（byte-equiv 🟢）+ 认证身份/其余 3 RPC 🔴 honest-defer**：空 actor → audit source "console-api" / event source "contextforge-core"（各自 byte-identical 现状，TEST-44.1.3 守护）；0 新 dep / 0 migration / proto add-only（ADR-004/008/015）。认证身份（X-Actor → 已认证 auth subject）须 console-api 鉴权层 → 🔴 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`；deprecate/softdelete/harddelete actor 透传须 7 层+新 migration / audit 重设计 → 🔴 `[SPEC-DEFER:phase-future.memory-actor-all-rpc]`（本 phase 仅做 emit_audit_and_event actor 参数共用基础）。

真实 v0.37.0 tag/run/digest 经用户全权授权 push（ADR-012），post-tag-push 回填（ADR-013 不预填）。

## Alternatives

- **A1（仅 unpin handler 透传到 store，不动 audit/event）**：只改 unpin handler 把 actor 传给 `set_pinned_with_actor`。否决：store 在 pinned=false 时丢弃 actor（store.rs:192-196），这是"空透传"——接口对称但 actor 无落点，违 ADR-013（honest over padding）。本 ADR 选完整闭环（audit/event source 归因）才有真实可观测价值。
- **A2（一并做 deprecate/softdelete/harddelete 四 RPC 全透传）**：把 memory-actor-all-rpc 四 RPC 全做。否决：grounding（Phase 43 + 本 phase）显示 Deprecate/SoftDelete 需 7 层改动 + 新 schema migration（set_status 无 actor 参数 + 无列记录 actor）；HardDelete 物理 DELETE 行后无法在行上存 actor（须 audit 层重设计 emit_audit_and_event——本 phase 做了共用基础，但 HardDelete handler 的完整透传仍须独立决策）——超"治理债小 phase 刻意小"定位。本 ADR 单聚焦 unpin + emit_audit_and_event 共用基础，其余 3 RPC 据实延后（ADR-013 不强行扩面）。
