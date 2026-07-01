# Task `44.1`: `unpin-actor-propagation — proto UnpinMemoryRequest add-only actor=2（既有 memory_id=1 冻结，ADR-015）+ buf generate + Rust unpin handler 透传 actor（镜像 pin :231-235）+ emit_audit_and_event 加 actor: &str 参数（audit source :59 / event source 用 actor，空回落各自原值 byte-equiv）+ pin handler 顺带传 actor 闭环 + deprecate/softdelete/harddelete 传 "console-api" byte-equiv + Go Unpin(id,actor) interface/grpcclient/handler X-Actor/memstore 4 处 + degraded stub 同步 + TEST-44.1.1/.2/.3/.4；0 新 dep / 0 migration / proto add-only / 默认 byte-equiv`

**Status**: Ready

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治，全权授权）
**Related Phase**: Phase 44 (memory-unpin-actor-propagation)
**Dependencies**: 既有 pin actor 透传（task-40.1 / ADR-045 D1：memory.rs:231-237 + handlers.go:559 X-Actor 范式 + proto PinMemoryRequest.actor=3 + grpcclient.go:725-729）+ 既有 set_pinned_with_actor（task-27.1 / ADR-032 D1：store.rs:184-207，pinned=false 丢弃 actor）+ 既有 emit_audit_and_event（task-15.2 / ADR-021 D1：memory.rs:52-78，source 硬编码）+ 既有 build_memory_event（memory.rs:103，source "contextforge-core"）+ ADR-049（本 task 即其 D1/D2/D3 原文实现）/ ADR-045（add-only Phase-44 Amendment @ task-44.3）/ ADR-032（add-only Phase-44 Amendment）/ ADR-021（emit_audit_and_event 镜像源）/ ADR-022 D2（pin lenient body 保持）/ ADR-015（proto add-only）/ ADR-004（空 actor byte-equiv）/ ADR-008（0 新 dep）/ ADR-013（真实 audit/event 归因非空透传 + 认证身份/其余 3 RPC 据实延后）/ ADR-012 / ADR-014 D1-D5（第三十五次激活）

## 1. Background

Phase 40 task-40.1 给 `pin` 加了 actor 透传（X-Actor → PinMemoryRequest.actor → store pinned_by），但 `unpin` 漏了。

- **B1 unpin handler 硬编码（真实）**：`memory.rs:298` `set_pinned_with_actor(&id, false, "console-api")` 硬编码 actor；pin handler（:231-237）已正确透传 `req.actor`。不对称。
- **B2 store pinned=false 丢弃 actor（真实，决定方案）**：`store.rs:192-196` `set_pinned_with_actor(pinned=false)` 把 pinned_by 清空为 ""。故 unpin handler 透传 actor 给 store 是"空透传"（actor 无落点）——真实价值须在别处。
- **B3 audit/event 不携 actor（真实，核心价值）**：`emit_audit_and_event`（:52）不接受 actor；audit source（:59）硬编码 "console-api"；`build_memory_event`（:103）source "contextforge-core"。让 actor 进入 audit/event source 是 unpin actor 透传的真实落点（console 部署在 auth 代理后时 unpin 可归因）。
- **B4 pin audit/event 也不归因（真实，顺带闭环）**：pin handler 虽透传 actor 到 store pinned_by，但其 emit_audit_and_event（:239-246）同样不携 actor。本 task 顺带让 pin audit/event 也归因（add-only byte-equiv）。
- **B5 pin actor 透传范式是镜像源（真实）**：pin handler :231-235（actor 读取 + 空回落）+ handlers.go:559（X-Actor header）+ proto PinMemoryRequest.actor=3 + grpcclient.go:728（pb.Actor）是 unpin 4 层透传的模板。

## 2. Goal

(1) **B1/B5 proto**：`UnpinMemoryRequest`（proto:369-371）add-only `string actor = 2`（既有 memory_id=1 冻结）+ buf generate。
(2) **B1 unpin handler 透传**：`memory.rs:287-302` unpin handler：`let req = req.into_inner();` + `let actor = if req.actor.is_empty() { "console-api" } else { req.actor.as_str() };`（镜像 :231-235）+ `set_pinned_with_actor(&req.memory_id, false, actor)`（store 丢弃，接口对称）+ `emit_audit_and_event(MemoryUnpin, &req.memory_id, actor)`（新签名）。
(3) **B3 emit_audit_and_event 加 actor**：`:52` 加第 4 参 `actor: &str`；audit source（:59）`"console-api"` → `actor`；`build_memory_event`（:103）加第 3 参 `actor: &str`，PbEvent.source（:103 内）`"contextforge-core"` → `actor`。
(4) **B4 pin 顺带闭环**：pin handler（:239-246）传 `actor`（消除 pin audit/event 不归因残余不对称）。
(5) **byte-equiv**：deprecate/softdelete/harddelete 传 `"console-api"`（audit）+ `"contextforge-core"`（event）byte-equiv（这 3 RPC 的 emit_audit_and_event 调用——audit 与 event source 分别保持原值）。
(6) **B5 Go 透传链**：types.go `Unpin(id)` → `Unpin(id, actor)` + grpcclient.go `pb.UnpinMemoryRequest{MemoryId, Actor}` + handlers.go `handleMemoryUnpin` 读 `X-Actor`（:559）+ memstore.go `Unpin(id, actor)` + degraded stub 同步。

pass bar：unpin actor 进 audit/event source（TEST-44.1.1 🟢）+ pin 顺带闭环（TEST-44.1.2 🟢）+ 空 actor byte-equiv（TEST-44.1.3 🟢）+ Go handleMemoryUnpin 读 X-Actor（TEST-44.1.4 🟢）；0 新 dep / 0 migration / proto add-only；既有测试不退化；ADR-014 D2 lint 0 未标注命中。

## 3. Scope

### In Scope
- proto `UnpinMemoryRequest` add-only `actor=2` + buf generate
- Rust `unpin` handler 透传 + `emit_audit_and_event` 加 actor 参数（audit source + build_memory_event source）+ pin 顺带传 actor + deprecate/softdelete/harddelete 传固定值 byte-equiv
- Go 4 处透传（types/grpcclient/handlers X-Actor/memstore）+ degraded stub 同步
- TEST-44.1.1（unpin actor 进 audit source：unpin(actor="bob") → audit source "bob"）/ TEST-44.1.2（pin 顺带闭环：pin(actor="alice") → audit source "alice"）/ TEST-44.1.3（空 actor byte-equiv：unpin(actor="") → audit source "console-api" / event source "contextforge-core"）/ TEST-44.1.4（Go handleMemoryUnpin 读 X-Actor 透传 Unpin(id, actor)）

### 范围外
- 认证身份 [SPEC-DEFER:phase-future.memory-actor-authenticated-identity]
- deprecate/softdelete/harddelete 真实 actor 透传 [SPEC-DEFER:phase-future.memory-actor-all-rpc]（本 task 仅做 emit_audit_and_event actor 参数共用基础）
- 真实 v0.37.0 tag [SPEC-OWNER:task-44.3-closeout]

## 4-5. Actors / Behavior Contract（同 phase §3-4，emit_audit_and_event actor 参数是核心：空 actor audit source "console-api" / event source "contextforge-core" 各自 byte-equiv；非空 → 各自 source 用 actor）

## 6. Acceptance Criteria

- [ ] **AC1**（unpin actor 进 audit/event source 🟢）: unpin(actor) → audit source = actor / event source = actor — verified by **TEST-44.1.1**
- [ ] **AC2**（pin 顺带闭环 🟢）: pin(actor) → audit source = actor（消除 pin audit/event 不归因残余不对称） — verified by **TEST-44.1.2**
- [ ] **AC3**（空 actor byte-equiv 🟢）: 空 actor → audit source "console-api" / event source "contextforge-core"（各自 byte-identical 现状） — verified by **TEST-44.1.3**
- [ ] **AC4**（Go handleMemoryUnpin 读 X-Actor 🟢）: handler 读 X-Actor 透传 Unpin(id, actor) — verified by **TEST-44.1.4**
- [ ] **AC5**（ADR-014 D2 lint）: PR 触及行 0 未标注命中 — verified by **TEST-44.1.5**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-44.1.1 | unpin(actor="bob") → audit source "bob" / event source "bob"（audit/event source 归因真实调用方） | `core/src/data_plane/memory.rs`（同源 test） | Not Started |
| TEST-44.1.2 | pin(actor="alice") → audit source "alice" / event source "alice"（pin 顺带闭环，消除 audit/event 不归因残余不对称） | `core/src/data_plane/memory.rs`（同源 test） | Not Started |
| TEST-44.1.3 | 空 actor unpin/pin → audit source "console-api" / event source "contextforge-core"（各自 byte-equiv 现状） | `core/src/data_plane/memory.rs`（同源 test） | Not Started |
| TEST-44.1.4 | Go handleMemoryUnpin 读 X-Actor header 透传 Unpin(id, actor) + grpcclient 填 pb.UnpinMemoryRequest.Actor | `internal/consoleapi/`（Go test） | Not Started |
| TEST-44.1.5 | D2 lint `--touched origin/master` 0 未标注命中（= LAST） | `scripts/spec_drift_lint.sh` | Not Started |

## 8. Risks
- R1（中）emit_audit_and_event 签名变更影响所有 memory RPC 调用点（须全同步）。缓解：一并改 + cargo check 守编译；空 actor / 固定值 byte-equiv。
- R2（低）audit source（"console-api"）与 event source（"contextforge-core"）既有值不同，空 actor 须各自回落原值。缓解：TEST-44.1.3 断言各自 byte-equiv。

## 9. Verification Plan
```bash
cargo test -p contextforge-core --lib data_plane::memory::test_44_1
go test ./internal/consoleapi/ -run TestTask441
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
go test ./... && go vet ./...
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

**Status**: Ready（待实施回填）
