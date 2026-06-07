# Task `40.1`: `memory-actor-propagation — PinMemoryRequest add-only actor=3 + MemoryStore.Pin(id,pin,actor) Go 参数链 + handleMemoryPin 读 X-Actor header 透传 + Rust pin() 用 req.actor 非空透传 / 空回落 "console-api"（默认 byte-equiv，ADR-004）；认证身份据实延后；ADR-022 D2 宽松 body 契约不改`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 40 (governance-debt-cleanup-3)
**Dependencies**: 既有 `core/src/data_plane/memory.rs` `pin()`（:215-240，task-27.1 / ADR-032 D1 已把 pin actor 做成 store first-class 字段 `set_pinned_with_actor` + proto `MemoryItem.pinned_by=11`，仅入口→store 透传链缺）/ `PinMemoryRequest`（proto:336-339 已在）/ buf generate（已在）/ `internal/consoleapi/handlers.go` `handleMemoryPin`（:525-549 已在）+ header 读取范式（`r.Header.Get` :815 / router.go:71）/ `internal/consoleapi/grpcclient/grpcclient.go` `memoryClient.Pin`（:724-726 已在）/ `internal/consoleapi/memstore.go` `MemMemoryStore.Pin`（:653 已在）/ ADR-032（memory-ops-hardening，pin actor first-class，本 task actor 透传维度兑现为 add-only Amendment @ task-40.3 closeout）/ ADR-015（console-data-plane proto 契约，add-only field 字段号冻结）/ ADR-022 D2（memory pin lenient body contract 保持）/ ADR-004（默认行为 + 既有契约不变，空 actor 回落 byte-equiv）/ ADR-008（dep add-only，Phase 40 = 0 新 dep）/ ADR-013（禁伪造红线——actor 真实透传非合成、认证身份据实延后不夸大）/ ADR-012（main-agent-governance-autonomy）/ ADR-014 D1-D5（第三十一次激活）

## 1. Background

task-27.1（ADR-032 D1）已把 memory pin 的 actor 做成 store 层 first-class 字段——`set_pinned_with_actor(id, pin, actor)` 写入 `pinned_by`（proto `MemoryItem.pinned_by=11` + migration），但**入口到 store 的 actor 透传链缺失**，故 `pin()` RPC 只能硬编码一个常量 actor：

- **B1 `pin()` 硬编码 actor**：`core/src/data_plane/memory.rs` `pin()`（:215-240）调 `set_pinned_with_actor(&req.memory_id, req.pin, "console-api")`（:229），actor 写死 `"console-api"`；doc-comment（:225-227）明记「console-api source is currently 'console-api'（real per-user actor propagation is `[SPEC-DEFER:phase-future.memory-actor-propagation]`）」。
- **B2 透传链断点**：(a) `PinMemoryRequest`（proto:336-339）只有 `memory_id=1` / `pin=2`，**无 actor field** → Rust `req` 拿不到调用方 actor；(b) Go `MemoryStore.Pin(id,pin)` interface（`memoryClient.Pin` grpcclient.go:724-726 / `MemMemoryStore.Pin` memstore.go:653）**无 actor 参数** → Go 侧无处携带 actor；(c) `handleMemoryPin`（handlers.go:525-549）**不读任何调用方标识** → REST 入口无 actor 来源。
- **B3 header 读取范式已存在**：console-api 已多处读 header——`r.Header.Get("Last-Event-ID")`（handlers.go:815）/ `r.Header.Get("X-Confirm")`（router.go:71）/ `r.Header.Get("Authorization")`（router.go:89）。本 task 用同范式读 `X-Actor`（缺省空串）。
- **B4 认证身份是另一层（据实延后，不夸大）**：本 task 交付**调用方透传**——actor 取自请求 header，**未做认证校验**（把 header 值映射为已验证 auth subject 须 console-api 鉴权层，当前无）。故 actor 是「调用方声明的标识」而非「已认证身份」——认证身份 honest-defer `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`（ADR-013 不夸大为已认证）。

本 task 补齐 B2 三处断点 + B4 据实标注，为 code-local 🟢 可单测，0 新 dep（仅既有 proto field + Go 参数 + header 读取），proto field 与 Go 参数均 add-only、空 actor 回落 `"console-api"` byte-equiv（ADR-004）。

## 2. Goal

(1) **B2a**：`PinMemoryRequest`（proto:336-339）add-only `string actor = 3`（既有 memory_id=1 / pin=2 字段号冻结，ADR-015 D1）+ buf generate 重生 Go/Rust binding。(2) **B2b**：Go `MemoryStore.Pin` interface 加 add-only `actor string` 参数（`memoryClient.Pin` / `MemMemoryStore.Pin` 两实现 + 调用点同步）+ `grpcclient` 填 `pb.PinMemoryRequest{MemoryId:id, Pin:pin, Actor:actor}`。(3) **B2c**：`handleMemoryPin`（handlers.go:525-549）读 `r.Header.Get("X-Actor")`（缺省空串）传入 `Pin(id,pin,actor)`；宽松 body 契约（ADR-022 D2）不改。(4) **B1**：Rust `pin()`（memory.rs:225-229）`set_pinned_with_actor` 第三参由硬编码 `"console-api"` 改为 `if req.actor.is_empty() { "console-api" } else { &req.actor }`（空回落 byte-equiv，ADR-004）+ 更新 :227 marker 措辞。(5) **B4**：认证身份据实延后 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`（ADR-013 不夸大）。

pass bar：`PinMemoryRequest{actor}` wire-tag 字段号 3 经 in-crate prost 断言（🟢）；Rust `pin()` 空 actor 回落 `"console-api"`、非空透传写入 `pinned_by`（🟢）；Go `handleMemoryPin` 读 `X-Actor` header 透传到 `Pin(actor)` + 缺省空串（🟢）；grpcclient 填 `pb.PinMemoryRequest.Actor`（🟢）；既有 client（不传 actor / 无 X-Actor header）行为与改前 byte-equiv（ADR-004）；公共 proto 既有字段号不动（ADR-015）、0 新 dep（ADR-008）；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`——`PinMemoryRequest`（:336-339）add-only `string actor = 3`（既有 `memory_id=1` / `pin=2` 不动）；`buf generate proto` 重生 Go（`.pb.go`）+ Rust（build.rs tonic-build 编译期）binding。
- 改 `internal/consoleapi/grpcclient/grpcclient.go`——`MemoryStore` interface `Pin(id string, pin bool)` → `Pin(id string, pin bool, actor string)`；`memoryClient.Pin`（:724-726）填 `pb.PinMemoryRequest{MemoryId:id, Pin:pin, Actor:actor}`。
- 改 `internal/consoleapi/memstore.go`——`MemMemoryStore.Pin`（:653）签名加 `actor string`（fallback 实现，签名对齐 interface；actor 在 in-mem fallback 据实处理或忽略，不破既有 pin 行为）。
- 改 `internal/consoleapi/handlers.go`——`handleMemoryPin`（:525-549）读 `actor := r.Header.Get("X-Actor")`（缺省空串，镜像 :815 header 范式）+ 调 `Pin(id, pin, actor)`；宽松 body 契约（ADR-022 D2，:519-524 doc-comment 范式）不改。
- 改 `core/src/data_plane/memory.rs`——`pin()`（:225-229）`set_pinned_with_actor` 第三参 `"console-api"` → `if req.actor.is_empty() { "console-api" } else { req.actor.as_str() }`（空回落 byte-equiv）；更新 :227 marker 措辞为「调用方透传已落地；认证身份 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`」。
- 同源测试：(a) Rust in-crate `#[cfg(test)]` prost wire-tag 断言 `PinMemoryRequest{actor:"x"}` 编码字段号 3（tag=(3<<3)|2=0x1A）；(b) Rust `pin()` 空 actor 回落 `"console-api"` / 非空透传写 `pinned_by`；(c) Go `handleMemoryPin` 读 `X-Actor` 透传 + 缺省空串；(d) Go grpcclient 填 `pb.PinMemoryRequest.Actor`。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- memory pin actor 认证身份（把 `X-Actor` header 值校验映射为已认证 auth subject，须 console-api 鉴权层）[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]——本 task 只交付调用方透传，认证校验延后。
- 其它 memory RPC（deprecate / hard-delete 等）的 actor 透传 [SPEC-DEFER:phase-future.memory-actor-all-rpc]——本 task 范围限 `pin`（actor first-class 落点 ADR-032 D1 即 pin），其它 RPC 的 actor 透传延后。
- `handleMemoryPin` 宽松 body 契约改 strict-400（ADR-022 D2 刻意保持，不在本 task 改）。
- 真实 release tag / run-id / digest（v0.33.0）[SPEC-OWNER:task-40.3-closeout]（ADR-012 用户授权后回填）。

## 4. Actors

- 主 agent（ADR-012 自治）
- `pin()` RPC（`core/src/data_plane/memory.rs:215-240`，本 task 把硬编码 actor 改为 `req.actor` 空回落）
- `PinMemoryRequest`（proto:336-339，本 task add-only `actor=3`）
- `MemoryStore.Pin` Go interface + `memoryClient.Pin`（grpcclient.go:724-726）/ `MemMemoryStore.Pin`（memstore.go:653）（本 task 加 `actor` 参数）
- `handleMemoryPin`（handlers.go:525-549，本 task 读 `X-Actor` header 透传）
- console 部署在 auth 代理后的运维 / 多租户场景（设置 `X-Actor` / `X-Forwarded-User` → pin 操作归因到真实调用方）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/data_plane/memory.rs:215-240`（`pin()` RPC——:229 `set_pinned_with_actor(&req.memory_id, req.pin, "console-api")` 硬编码 actor + :225-227 doc-comment / marker，本 task 改为 `req.actor` 空回落）
- `proto/contextforge/console_data_plane/v1/console_data_plane.proto:303-319`（`MemoryItem`——:317 `pinned_by=11` actor 落点，task-27.1）+ `:336-339`（`PinMemoryRequest` 仅 memory_id=1 / pin=2，本 task add-only `actor=3`）
- `internal/consoleapi/handlers.go:525-549`（`handleMemoryPin` REST 入口）+ `:815`（`r.Header.Get("Last-Event-ID")` header 范式）+ `internal/consoleapi/router.go:71`（`r.Header.Get("X-Confirm")` header 范式）
- `internal/consoleapi/grpcclient/grpcclient.go:724-726`（`memoryClient.Pin(id,pin)` → `pb.PinMemoryRequest{MemoryId,Pin}`，本 task 加 `actor` + `Actor`）+ `internal/consoleapi/memstore.go:653`（`MemMemoryStore.Pin` fallback 实现）
- `docs/decisions/adr-032-*.md §D1`（pin actor first-class，本 task actor 透传维度兑现为 add-only Amendment @ task-40.3）+ `docs/decisions/adr-015-*.md`（proto add-only field 字段号冻结）+ `docs/decisions/adr-022-*.md §D2`（memory pin lenient body contract 保持）+ `docs/decisions/adr-045-governance-debt-cleanup-3.md §D1`（本 task 即其原文实现）

### 5.2 关键设计 — actor add-only 透传链（0 dep / proto add-only / 默认 byte-equiv）

- **B2a proto add-only field**：`PinMemoryRequest` add-only `string actor = 3`（既有 `memory_id=1` / `pin=2` 字段号冻结，ADR-015 D1）。`buf generate proto` 重生 Go `.pb.go`（`Actor string` + getter）+ Rust binding（build.rs tonic-build 编译期，`pub actor: String`）。in-crate prost wire-tag 断言：`PinMemoryRequest{actor:"x", ..}` 编码含字段号 3 的 tag（`(3<<3)|2 = 0x1A`，length-delimited）。
- **B2b Go 参数链 add-only**：`MemoryStore.Pin(id,pin)` → `Pin(id,pin,actor)`（interface + `memoryClient` + `MemMemoryStore` 两实现 + 调用点同步）；`memoryClient.Pin` 填 `pb.PinMemoryRequest{MemoryId:id, Pin:pin, Actor:actor}`。
- **B2c REST header 透传**：`handleMemoryPin`（:525-549）`actor := r.Header.Get("X-Actor")`（缺省空串，镜像 :815 范式）+ 调 `Pin(id, pin, actor)`；宽松 body 契约（ADR-022 D2）不改——仅在入口加 header 读取，不改 body 解析回落 `pin=true` 行为。
- **B1 Rust pin() 空回落 byte-equiv**：`pin()`（:229）`set_pinned_with_actor(&req.memory_id, req.pin, if req.actor.is_empty() { "console-api" } else { req.actor.as_str() })`——空 actor（既有 client / 无 X-Actor header）= 既有硬编码值 `"console-api"`，byte-equiv（ADR-004）；非空 actor 透传写入 `pinned_by`。
- **B4 认证身份据实延后**：actor 取自 header（调用方声明），未做认证校验 → spec / ADR-045 D1 据实记「调用方透传已落地、认证身份 honest-defer `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`」（ADR-013 不夸大为已认证身份）。

### 5.3 不变量

- 默认行为不变（ADR-004）：既有 client（不传 actor）/ 无 `X-Actor` header → 空 actor → Rust 回落 `"console-api"` → 与改前 byte-equiv（`pinned_by` 写入值不变）；`handleMemoryPin` 宽松 body 契约（ADR-022 D2）不变；`PinMemoryRequest` 既有 `memory_id=1` / `pin=2` 字段号 / 语义不变。
- 既有契约不变：proto add-only `actor=3`（既有字段号不动，ADR-015 D1）；Go `Pin` 加参数是源码内部 interface（非对外 proto），两实现 + 调用点同步编译通过；Rust `pin()` 行为仅在 actor 非空时变（透传），空时不变。
- 0 新代码依赖（ADR-008）：仅既有 proto field + Go 参数 + `r.Header.Get`，无 Cargo / go.mod 依赖增量。
- 认证边界诚实（ADR-013）：actor = 调用方声明的标识（header），非已认证身份；据实记 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`，不夸大。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [ ] **AC1**（proto add-only `actor=3` + wire-tag 🟢）: `PinMemoryRequest`（proto:336-339）add-only `string actor = 3`（既有 memory_id=1 / pin=2 字段号冻结，ADR-015 D1）+ buf generate 重生 Go/Rust binding；in-crate prost 断言 `PinMemoryRequest{actor}` 编码字段号 3（tag 0x1A）；0 新 dep — verified by **TEST-40.1.1**
- [ ] **AC2**（Rust `pin()` actor 空回落 / 非空透传 🟢）: `pin()`（:225-229）`set_pinned_with_actor` 第三参 `if req.actor.is_empty() { "console-api" } else { req.actor.as_str() }`——空 actor 回落 `"console-api"`（默认 byte-equiv，ADR-004）、非空 actor 透传写入 `pinned_by`；marker 措辞更新为认证身份延后 — verified by **TEST-40.1.2**
- [ ] **AC3**（Go `handleMemoryPin` 读 `X-Actor` 透传 🟢）: `handleMemoryPin`（:525-549）读 `r.Header.Get("X-Actor")`（缺省空串）传入 `Pin(id,pin,actor)`；`MemoryStore.Pin` interface + 两实现加 `actor` 参数；ADR-022 D2 宽松 body 契约不改 — verified by **TEST-40.1.3**
- [ ] **AC4**（grpcclient 填 `pb.PinMemoryRequest.Actor` 🟢）: `memoryClient.Pin`（:724-726）填 `pb.PinMemoryRequest{MemoryId:id, Pin:pin, Actor:actor}` — verified by **TEST-40.1.4**
- [ ] **AC5**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-40.1.5**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-40.1.1 | proto add-only `PinMemoryRequest.actor=3`：in-crate prost wire-tag 断言 `{actor:"x"}` 编码含字段号 3 的 tag（0x1A）；既有 memory_id=1 / pin=2 字段号不动；buf generate 重生 binding；0 新 dep | `core/src/data_plane/memory.rs`（in-crate `#[cfg(test)]`） | Planned |
| TEST-40.1.2 | Rust `pin()` actor 空回落 / 非空透传：空 `req.actor` → `set_pinned_with_actor(.., "console-api")`（byte-equiv，`pinned_by`=console-api）；非空 → 透传写 `pinned_by` | `core/src/data_plane/memory.rs`（in-crate test 或 tests/ 集成） | Planned |
| TEST-40.1.3 | Go `handleMemoryPin` 读 `X-Actor` 透传：设 `X-Actor: alice` → `Pin(id,pin,"alice")`；无 header → `Pin(id,pin,"")`（缺省空串）；ADR-022 D2 宽松 body 契约不变 | `internal/consoleapi/handlers_test.go`（或同源 test） | Planned |
| TEST-40.1.4 | Go grpcclient 填 `pb.PinMemoryRequest.Actor`：`memoryClient.Pin(id,pin,"alice")` → fake server 收到 `Actor=="alice"` | `internal/consoleapi/grpcclient/grpcclient_test.go` | Planned |
| TEST-40.1.5 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（中）proto add-only `actor` 字段号冲突 / 既有 client 破**：`PinMemoryRequest` 加 `actor` 若用了既有字段号或非 add-only，会破既有控制面 client。
  - **缓解**：用字段号 3（既有 memory_id=1 / pin=2 冻结，ADR-015 D1）；buf generate 重生 binding + in-crate prost wire-tag 断言字段号 3；既有 client（不传 actor）= proto3 默认空串 → Rust 回落 `"console-api"` byte-equiv。stop-condition：字段号冲突 / 既有 client 破 / 空 actor 非回落则 AC1/AC2 不标 `[x]`。
- **R2（中）Go `Pin` interface 加参数漏改实现 / 调用点**：`MemoryStore.Pin` 加 `actor` 须 `memoryClient.Pin` / `MemMemoryStore.Pin` 两实现 + `handleMemoryPin` 调用点同步，漏改则编译失败。
  - **缓解**：一并改 interface + 两实现 + 调用点；`go build ./...` + `go vet` 守编译；TEST-40.1.3/40.1.4 断言透传。stop-condition：编译失败 / 任一实现漏改则 AC3/AC4 不标 `[x]`。
- **R3（低）认证身份被误读为已实现**：actor 取自 header（调用方声明）易被夸大为已认证身份。
  - **缓解**：spec §1 B4 / §5.2 B4 / §5.3 + ADR-045 D1 据实记「调用方透传、认证身份 honest-defer `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`」（ADR-013 不夸大）。stop-condition：若把调用方透传夸大为已认证身份则越界。
- **R4（低）`handleMemoryPin` 宽松 body 契约被误改**：在入口加 header 读取时易顺手改 body 解析。
  - **缓解**：本 task 只在入口加 `r.Header.Get("X-Actor")`，不动 body 解析回落 `pin=true`（ADR-022 D2）；既有 pin body 测维持绿。stop-condition：若改宽松 body 契约则违 ADR-022 D2。

## 9. Verification Plan

```bash
# 1. AC1 — proto add-only actor=3 wire-tag（in-crate prost 断言）
cargo test -p contextforge-core data_plane::memory

# 2. AC2 — Rust pin() actor 空回落 / 非空透传
cargo test -p contextforge-core pin

# 3. AC3/AC4 — Go handleMemoryPin 读 X-Actor + grpcclient 填 Actor
go test ./internal/consoleapi/... -run 'Pin|Actor'

# 4. 不退化（全量）
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
go build ./... && go vet ./... && go test ./...

# 5. AC5 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.memory-actor-propagation-defer-note]：本 task 交付 memory pin 的**调用方 actor 透传**（REST `X-Actor` header → proto `actor=3` → Go 参数链 → Rust `pin()` 空回落 `"console-api"`），🟢 可单测，0 新 dep / proto add-only / 默认 byte-equiv。**认证身份**（把 header 值校验映射为已认证 auth subject）须 console-api 鉴权层 [SPEC-DEFER:phase-future.memory-actor-authenticated-identity]、其它 memory RPC 的 actor 透传 [SPEC-DEFER:phase-future.memory-actor-all-rpc] 均不在本 task 范围。actor = 调用方声明的标识（header），非已认证身份（据实声明，ADR-013 不夸大）；实测产物（v0.33.0）真实跑出后回填。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Draft

**§9 Verification 计划** (will record real evidence at impl)：
- AC1：`cargo test -p contextforge-core data_plane::memory` —— `PinMemoryRequest{actor:"x"}` in-crate prost wire-tag 断言字段号 3（tag 0x1A）；既有 memory_id=1 / pin=2 字段号不动；buf generate 重生 binding；0 新 dep（真实结果待实施回填，ADR-013 不伪造）。
- AC2：`cargo test -p contextforge-core pin` —— `pin()` 空 actor 回落 `"console-api"`（byte-equiv）/ 非空透传写 `pinned_by`（真实结果待实施回填）。
- AC3/AC4：`go test ./internal/consoleapi/...` —— `handleMemoryPin` 读 `X-Actor` header 透传 `Pin(actor)` + 缺省空串 / grpcclient 填 `pb.PinMemoryRequest.Actor`（真实结果待实施回填）。
- AC5：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威）。
- 0 新 dep / proto add-only / 默认 byte-equiv / 认证身份据实延后 真实结果待实施回填（ADR-013 不预填）。

**实际改动文件**（计划，待实施回填）：
- `proto/contextforge/console_data_plane/v1/console_data_plane.proto`——`PinMemoryRequest`（:336-339）add-only `string actor = 3` + buf generate 重生 `.pb.go` + Rust binding。
- `internal/consoleapi/grpcclient/grpcclient.go`——`MemoryStore.Pin` 加 `actor string` + `memoryClient.Pin` 填 `pb.PinMemoryRequest.Actor`。
- `internal/consoleapi/memstore.go`——`MemMemoryStore.Pin` 加 `actor string`（fallback 对齐签名）。
- `internal/consoleapi/handlers.go`——`handleMemoryPin`（:525-549）读 `X-Actor` header 透传。
- `core/src/data_plane/memory.rs`——`pin()`（:225-229）`set_pinned_with_actor` 空 actor 回落 `"console-api"` + marker 措辞更新。+ in-crate test。
- `docs/decisions/adr-032-*.md` actor 透传维度兑现 add-only Amendment 落点在 task-40.3 closeout（非本 task body）。
