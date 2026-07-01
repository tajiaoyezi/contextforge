# Phase 44 · memory-unpin-actor-propagation

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。本 phase 闭环 pin/unpin actor 透传不对称——Phase 40 task-40.1（ADR-045 D1）给 `pin` 加了 actor 透传（X-Actor header → PinMemoryRequest.actor → store pinned_by），但对称的 `unpin` 漏了（unpin handler `core/src/data_plane/memory.rs:298` 硬编码 `"console-api"`）。**grounding 发现真实价值在 audit/event**：`set_pinned_with_actor(id, false, actor)` 在 pinned=false 时丢弃 actor（store.rs:192-196 清 pinned_by），故单纯透传到 store 是"空透传"（违 ADR-013）；真实落点是 `emit_audit_and_event`（memory.rs:52 不接受 actor / :59 硬编码 source）——让 actor 进入 audit log + event stream，console 部署在 auth 代理后时 unpin 可归因到真实调用方。本 phase 交付**完整闭环**：unpin handler 透传 + `emit_audit_and_event` 加 actor 参数（audit/event source 归因）+ pin 顺带闭环（add-only，既有 byte-equiv）+ Go 透传链。code-local 🟢 可单测，0 新 dep（ADR-008）+ 0 schema migration + proto add-only（UnpinMemoryRequest actor=2，ADR-015）。**诚实定性（ADR-013）**：认证身份 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`（须 console-api 鉴权层）；deprecate/softdelete/harddelete actor 透传 `[SPEC-DEFER:phase-future.memory-actor-all-rpc]`（Deprecate/SoftDelete 需 7 层+新 migration / HardDelete 须 audit 重设计——本 phase 仅做 emit_audit_and_event actor 参数共用基础，这 3 RPC 未来顺带受益）据实延后。默认行为 / proto（add-only）/ 既有契约不变（ADR-004：空 actor 回落 "console-api" byte-equiv）；既有 `cargo-test` / `go-test` / `spec-lint` 三门不退化。

> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md §3.26 + §4 backlog` → 各锚点（`core/src/data_plane/memory.rs:215-302`（pin :231-237 透传 + unpin :287-302 硬编码 + emit_audit_and_event :52-78 + build_memory_event :103）+ `core/src/memory/store.rs:184-207`（set_pinned_with_actor pinned=false 丢弃 actor）+ `proto/contextforge/console_data_plane/v1/console_data_plane.proto:342-373`（PinMemoryRequest actor=3 / UnpinMemoryRequest 仅 memory_id=1）+ `internal/consoleapi/types.go:120-132`（MemoryClient interface）+ `internal/consoleapi/grpcclient/grpcclient.go:725-752`（Pin/Unpin client）+ `internal/consoleapi/handlers.go:548-626`（handleMemoryPin X-Actor :559 / handleMemoryUnpin 不读 :621）+ `internal/consoleapi/memstore.go:703`（MemMemoryStore.Unpin））→ ADR-014（D1-D5，第三十五次激活）→ ADR-013（禁伪造：unpin actor 真实进 audit/event source 非空透传；认证身份 + 其余 3 RPC 据实延后）。

## 1. 阶段目标

v0.36.0 ship 后，闭环 pin/unpin actor 透传不对称：把 unpin handler 硬编码 `"console-api"` 改为真实透传调用方 actor，核心价值落在 audit/event source 归因（非 store pinned_by——unpin 语义清空 pin 快照）。pin 顺带闭环（audit/event 也归因）。code-local 🟢 可单测，0 新 dep + 0 migration + proto add-only。认证身份 + deprecate/softdelete/harddelete actor 透传据实延后。默认行为 / proto / 既有契约不变（ADR-004）；Phase 44 = 0 新依赖（ADR-008）；既有三门不退化。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. **unpin actor 透传 + audit/event source 归因**：proto `UnpinMemoryRequest` add-only `actor=2` + Rust unpin handler 透传 + `emit_audit_and_event` 加 `actor` 参数（audit source + event source 用 actor）+ pin 顺带传 actor + Go 透传链（interface/grpcclient/handler X-Actor/memstore）；空 actor 回落 "console-api" byte-equiv（AC1）
2. **v0.37.0 closeout + 默认零依赖守线 + honest-defer**：认证身份 + deprecate/softdelete/harddelete 据实延后；默认 byte-equiv + 0 dep + 0 migration + proto add-only；smoke v34[53/53] + release docs + ADR-049 ratify + ADR-032/045 Amendment + roadmap/adapter（AC2）
3. ADR-014 D1-D5（**第三十五次**激活）全通过（AC3）

**v.x 版本号决策**：v0.37.0（Phase 44，承 v0.36.0），theme memory-unpin-actor-propagation。minor release（治理债闭环 + audit/event 归因；0 新 dep / 0 migration / proto add-only / 默认 byte-equiv）。

## 2. 业务价值

闭环 pin/unpin actor 透传不对称，让 unpin 的 audit log + event stream 归因到真实调用方（pin 顺带闭环）：

### 44.1 unpin-actor-propagation（🟢）

- Phase 40 task-40.1 给 pin 加了 actor 透传（store pinned_by），unpin 漏了。grounding 发现 store 在 pinned=false 时丢弃 actor，真实价值在 audit/event：emit_audit_and_event 不携 actor + source 硬编码 "console-api"。本 phase 加 actor 参数让 audit/event source 归因（pin 顺带闭环消除残余不对称）。
- **HONEST CAVEAT（ADR-013）**：本 phase 交付**调用方透传**（audit/event source 归因），认证身份（X-Actor → 已认证 auth subject）须 console-api 鉴权层 → honest-defer `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`。

**不在本 phase 范围**：认证身份 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`；deprecate/softdelete/harddelete actor 透传 `[SPEC-DEFER:phase-future.memory-actor-all-rpc]`（本 phase 仅做 emit_audit_and_event actor 参数共用基础）。

## 3. 涉及模块

### 44.1 unpin-actor-propagation（task-44.1）

- 改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`——`UnpinMemoryRequest`（:369-371）add-only `string actor = 2`（既有 memory_id=1 冻结，ADR-015）；buf generate
- 改 `core/src/data_plane/memory.rs`——unpin handler（:287-302）透传 actor + `emit_audit_and_event`（:52）加 `actor: &str` 参数（audit source :59 用 actor）+ pin handler（:239-246）传 actor（顺带闭环）+ deprecate/softdelete/harddelete 传 `"console-api"` byte-equiv + `build_memory_event`（:103）加 actor 参数（PbEvent.source 用 actor）+ TEST-44.1.1/.2/.3
- 改 `internal/consoleapi/types.go`——`MemoryClient.Unpin(id)` → `Unpin(id, actor)`
- 改 `internal/consoleapi/grpcclient/grpcclient.go`——`memoryClient.Unpin(id)` → `Unpin(id, actor)` + `pb.UnpinMemoryRequest{MemoryId, Actor}`
- 改 `internal/consoleapi/handlers.go`——`handleMemoryUnpin` 读 `X-Actor`（:559 范式）+ 传 `Unpin(id, actor)`
- 改 `internal/consoleapi/memstore.go`——`MemMemoryStore.Unpin(id)` → `Unpin(id, actor)`
- 改 degraded fallback（`console_api_serve_degraded.go`，如有 Unpin）签名同步
- 同源验证（🟢：TEST-44.1.1 unpin actor 进 audit source + TEST-44.1.2 pin actor 进 audit source 顺带闭环 + TEST-44.1.3 空 actor byte-equiv + Go TEST-44.1.4 handleMemoryUnpin 读 X-Actor）

### 44.2 closeout（task-44.3）

- smoke v33→v34[53/53]（unpin X-Actor 端到端断言）+ TestTask443（no-regression [37/37]..[52/52]）
- v0.37.0 release docs + ADR-049 ratify + ADR-032/045 add-only Amendment + roadmap §3.26/§4 + adapter + phase §6

### BDD feature

- `test/features/phase-44-memory-unpin-actor-propagation.feature`（≥2 scenario）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 44.1 | proto actor=2 + Rust unpin/pin handler + emit_audit_and_event actor 参数 + Go 透传链 + TEST-44.1.* | `../tasks/task-44.1-unpin-actor-propagation.md` |
| 44.3 | smoke v34[53/53] + v0.37.0 closeout + ADR-049 ratify + ADR-032/045 Amendment + roadmap/adapter | `../tasks/task-44.3-closeout-v0.37.0.md` |

## 5. 依赖关系

- task-44.1 dep 既有 pin actor 透传（task-40.1 / ADR-045 D1）+ set_pinned_with_actor（task-27.1）+ emit_audit_and_event（task-15.2）+ X-Actor header 范式（handlers.go:559）；无外部 dep。
- task-44.3 dep 44.1 Done。
- ADR-049（新 Proposed）/ ADR-045 + ADR-032（add-only Amendment）/ ADR-021（emit_audit_and_event 镜像源）/ ADR-022 D2（lenient body 保持）/ ADR-015（proto add-only）/ ADR-004 / ADR-008 / ADR-013 / ADR-012（全权授权）/ ADR-014 第三十五次激活。

## 6. 阶段级验收标准 + 端到端 smoke

- [ ] **AC1**（unpin actor 透传 + audit/event source 归因 🟢）: proto add-only actor=2 + Rust unpin 透传 + emit_audit_and_event 加 actor 参数（audit/event source 归因）+ pin 顺带闭环 + Go 透传链；空 actor 回落 byte-equiv — verified by **TEST-44.1.1**（unpin actor 进 audit source）+ **TEST-44.1.2**（pin 顺带闭环）+ **TEST-44.1.3**（空 actor byte-equiv）+ **TEST-44.1.4**（Go handleMemoryUnpin 读 X-Actor）+ phase-smoke step 1
- [ ] **AC2**（v0.37.0 closeout + 默认零依赖守线）: 认证身份 + deprecate/softdelete/harddelete 据实延后；默认 byte-equiv + 0 dep + 0 migration + proto add-only；smoke v34[53/53] + release docs + ADR-049 ratify + ADR-032/045 Amendment + roadmap/adapter — verified by **TEST-44.3.1**
- [ ] **AC3**（ADR-014 cross-validation gate）: D1-D5（第三十五次激活）全通过 — verified by task-44.3 closeout PR body + LAST TEST

**端到端 smoke**：(1) unpin X-Actor 端到端（REAL 模式 POST /v1/memory/{id}/unpin 带 X-Actor → audit source 归因，不可达归因 unit）；(2) v0.37.0 收口 + 默认零依赖守线。

## 7. 阶段级风险

- **R1（中）emit_audit_and_event 签名变更影响所有 memory RPC**：加 actor 参数是 add-only，但所有调用点（pin/unpin/deprecate/softdelete/harddelete）须同步传值，漏改则编译失败。
  - **缓解**：一并改所有调用点；`cargo check` 守编译；空 actor / "console-api" byte-equiv。stop-condition：编译失败 / 既有行为回归则 AC1 不标 `[x]`。
- **R2（低）audit source 与 event source 既有值不同被误改**：audit source 既有 "console-api"，event source 既有 "contextforge-core"——空 actor 回落须各自保持原值 byte-equiv。
  - **缓解**：emit_audit_and_event 空 actor → audit source "console-api" / event source "contextforge-core"（各自 byte-equiv）；TEST-44.1.3 断言。stop-condition：空 actor 改变既有 source 则 AC1 不标 `[x]`。

## 8. Definition of Done

- 2 task spec 顶部 `**Status**: Done`；§6 AC1-3 全 `[x]`；端到端 smoke 2 step 全 PASS。
- ADR-049 Proposed→Accepted（逐 D ratify）；ADR-032/045 add-only Amendment；roadmap §3.26/§4 + adapter。
- release：`docs/releases/v0.37.0-{evidence,artifacts}.md` + RELEASE_NOTES + README v0.37 段。
- smoke：v34[53/53] + TestTask443（no-regression [37/37]..[52/52]）。
- follow-up：认证身份 + deprecate/softdelete/harddelete actor 透传留 backlog。
