# Phase 27 · memory-ops-hardening

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。本 phase 硬化 Phase 13 / Phase 17 落地的 Memory 生命周期 / pin 语义：**记录 pin-actor + pinned-at-timestamp**（ADR-022 §Trade-offs 缩范围延后的 `pin_actor` + `[SPEC-DEFER:phase-future.memory-pinned-at-timestamp]`）、**Pin/Unpin 显式拆分**（vs 既有 `bool pin` toggle）、**hard-delete 策略**（vs 仅 soft-delete；X-Confirm gated）、**is_pinned 审计回填**（ADR-022 `[SPEC-DEFER:phase-future.is-pinned-backfill-from-audit]`）。proto 改动全 add-only（不破冻结契约，proto-freeze guard 须过）。全部本地（ADR-004，0 网络 / 默认构建 0 新 dep）。v0.20.0 收口。对应 `docs/roadmap.md`（memory-ops 段）。
>
> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md`（memory-ops 段）→ `docs/decisions/adr-022-memory-is-pinned-field-amendment.md`（§Trade-offs / Conscious limitations 三条 marker：`pin_actor` 不引入 / `memory-pinned-at-timestamp` 延后 / `is-pinned-backfill-from-audit` 延后 + D2 Pin RPC 写穿语义）→ `proto/contextforge/console_data_plane/v1/console_data_plane.proto:283-336`（`MemoryItem` field 1-10 含 `is_pinned=10` + 5 RPC `MemoryService` + `PinMemoryRequest{memory_id, bool pin}`）→ `core/src/memory/store.rs`（`SqliteMemoryStore::set_pinned` `:153` toggle + `set_status` `:170` 三态 + `MemoryItem` struct）+ `core/migrations/0013_memory_items.sql`（`is_pinned INTEGER DEFAULT 0` + `status CHECK active/deprecated/soft_deleted`）→ `core/src/data_plane/memory.rs`（`MemoryServer` thin proxy + `emit_audit_and_event` `:51` + `AuditOperation::MemoryPin/MemoryUnpin` 据 `req.pin` 分流 `:220`）→ `core/src/memoryops/audit.rs:19-37`（`AuditOperation` enum + `MemoryPin/MemoryUnpin/MemoryDeprecate/MemorySoftDelete`）→ `internal/consoleapi/router.go:38-44`（5 memory routes，deprecate/soft-delete 经 `confirmMiddleware` `:62`）+ `internal/consoleapi/handlers.go:519-588`（`handleMemoryPin` / `handleMemoryDeprecate` / `handleMemorySoftDelete`）→ `core/tests/proto_contract.rs`（FROZEN proto 契约 + freeze 规则）→ `internal/contractv1/contractv1.go:206-232`（Go `MemoryItem.IsPinned` + `MemoryOperation{OpType, Actor}`）→ AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5，第十八次激活）→ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造凭据红线）→ `docs/decisions/adr-017-console-contract-completion-22-endpoint.md`（D2 destructive X-Confirm 兜底）→ `docs/decisions/adr-008-core-library-selection.md`（依赖选型 add-only）。
>
> **ADR 影响面（已识别）**：
> - **ADR-032 memory-ops-hardening（新，Proposed）**：记 pin-actor + pinned-at-timestamp add-only 字段（D1）+ Pin/Unpin 拆分 + hard-delete 策略（D2）+ is_pinned 审计回填（D3）+ 全 add-only / 默认构建不变 / X-Confirm 复用（D4）。落地后据真实非合成往返 / 真实物理删除 + 412 兜底结果 ratify（ADR-013）。
> - 触及 **ADR-022（memory-is-pinned-field-amendment）**：§Trade-offs 三条 marker（`pin_actor` / `memory-pinned-at-timestamp` / `is-pinned-backfill-from-audit`）由本 phase 推进——以 add-only Amendment 记录推进结果，不溯改 ADR-022 正文 D1-D5（D5）。
> - 触及 **ADR-017（console-contract-completion-22-endpoint）**：D2 destructive X-Confirm 兜底被 hard-delete 复用（不改 `confirmMiddleware` 语义，新 route 经既有中间件 gated）。
> - 触及 **ADR-008（core-library-selection）**：actor / timestamp / backfill 全用既有 `rusqlite` / `serde`，预期 0 新依赖；若实施时确需新 dep 则按 add-only Amendment 记录（不溯改既有 D 段）。

## 1. 阶段目标

v0.20.0 ship 后，ContextForge 的 Memory 生命周期 / pin 语义具备**可审计的 pin 归属**（`pinned_by` + `pinned_at_unix` add-only 字段，pin 操作写穿、proto-freeze guard 过）、**显式 Pin/Unpin RPC**（与既有 `bool pin` toggle 并存、add-only 不破坏既有调用）、**hard-delete 策略**（物理删除、经既有 X-Confirm destructive 兜底 gated）、以及**从 audit log 回填 is_pinned**（按 `memory_pin`/`memory_unpin` 事件时序重建 legacy item 状态）。proto 改动全 add-only（不动既有 tag）、SQLite migration add-only column with default、默认构建 0 新依赖 + 0 网络（ADR-004）、既有 5 RPC 行为不变。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. `MemoryItem` 新增 add-only proto 字段 `pinned_by` + `pinned_at_unix`（序号在既有 field 10 后追加，不动既有 tag）+ `memory_items` 表 add-only migration（缺省值 backfill）；pin=true 写穿 actor + timestamp、pin=false 归缺省，round-trip 在 deterministic 单测可断言；proto-freeze guard（`core/tests/proto_contract.rs`）过、既有字段不退化（AC1）
2. 显式 `Unpin` RPC（add-only，与既有 `Pin` 并存、不动其签名）+ `HardDelete` RPC（add-only，物理删除行）落地；console-api `POST /v1/memory/{id}/hard-delete` 经既有 `confirmMiddleware` gated（缺 X-Confirm → 412），deterministic 单测可断言拆分语义 + hard-delete 物理删除 + 412 兜底（AC2）
3. is_pinned 从 audit log 回填——按 `memory_pin`/`memory_unpin` 事件时序重放、以末次事件重建当前 `is_pinned`，opt-in 一次性 reconcile（非热路径），deterministic 单测可断言（构造 audit 序 → backfill → is_pinned 与重放末态一致）（AC3）
4. v0.20.0 release docs + `scripts/console_smoke.sh` v17（memory ops 硬化相关 smoke 断言 + 既有 step 不退化）+ phase §6 闭合 + ADR-032 据真实非合成结果 ratify 或记录维持 + ADR-022 add-only Amendment（AC4）
5. ADR-014 D1-D5（第十八次激活）全通过（AC5）

**v0.x 版本号决策**：v0.20.0 minor release（Memory ops 硬化收口；全 add-only proto 演进 + 新 RPC，不破坏既有 v0.6-v0.19 client；默认构建 0 新依赖 + 0 网络）。

## 2. 业务价值

直接推进 ADR-022 §Trade-offs / Conscious limitations 中三条刻意缩范围延后的 marker，并补齐 Memory 生命周期缺口：

- **pin-actor + pinned-at-timestamp**：ADR-022 决定「不引入 `pin_actor`（留 audit log 查）」+「不需要 `pinned_at`（留 audit 溯源）」并标 `[SPEC-DEFER:phase-future.memory-pinned-at-timestamp]`。但 audit log 当前不记 actor（`core/src/data_plane/memory.rs:62` 只记 `chunk_ids=[memory_id]`），`set_pinned` 只 bump `updated_at_unix`（`store.rs:157`，被任何 update 覆盖）——「谁 / 何时置顶」事实上不可查。本 phase 让二者成为一手字段（add-only），使 Console UI 详情面板可显示 pin 归属。
- **Pin/Unpin 拆分 + hard-delete**：既有 `Pin` 是 `bool pin` toggle（`proto:311`，console-api 空 body 兜底 `pin=true`，`handlers.go:536`），语义不显式；Memory 生命周期只有 soft-delete（`set_status(id, "soft_deleted")`，行仍在表、get-by-id 仍可取，`store.rs:135`），无物理删除路径。本 phase 显式拆分 Pin/Unpin（add-only），并加 hard-delete（隐私基线 ADR-004「不可恢复清除」诉求，经既有 X-Confirm destructive 兜底 gated）。
- **is_pinned backfill from audit**：ADR-022 接受「字段 backfill 不溯源」trade-off 并标 `[SPEC-DEFER:phase-future.is-pinned-backfill-from-audit]`——legacy item（v0.10 前）`is_pinned` 恒 `false` 即使有 pin 历史。本 phase 提供按 audit 事件重放重建 is_pinned 的 opt-in reconcile。
- **PRD §Privacy / §Memory（本地优先 + 可审计生命周期）**：Memory 的「谁动过 / 何时动 / 能否彻底清除」在隐私基线下成为可观测、可审计、可清除的一手能力。

**不在本 phase scope**：

- per-user actor 透传（console-api source 当前写死 `"console-api"`，`memory.rs:64`，真实用户身份取决于上游链路）[SPEC-DEFER:phase-future.memory-actor-propagation]
- hard-delete 的级联清理（清向量索引 / 引用该 memory 的 trace）[SPEC-DEFER:phase-future.memory-hard-delete-cascade]
- Memory 内容全文检索 / 语义召回 [SPEC-DEFER:phase-future.memory-semantic-search]
- Memory item 版本历史 / 软删除回收站 UI [SPEC-DEFER:phase-future.memory-restore-recycle-bin]
- 跨 agent_scope 的 memory 迁移 / 合并 [SPEC-DEFER:phase-future.memory-scope-merge]

## 3. 涉及模块

### 27.1 pin-actor + pinned-at-timestamp（task-27.1）

- 修改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`——`MemoryItem` 加 add-only `string pinned_by` + `int64 pinned_at_unix`（序号在既有 field 10 后追加，不动既有 tag）
- 修改 `core/migrations/0013_memory_items.sql`（或新增 `0016_memory_items_add_pin_actor.sql`，task-27.1 §3 据 migration 习惯定）——add-only `pinned_by TEXT NOT NULL DEFAULT ''` + `pinned_at_unix INTEGER NOT NULL DEFAULT 0`（既有行缺省 backfill，同 `is_pinned` pattern）
- 修改 `core/src/memory/store.rs`——`MemoryItem` struct 加二字段 + `set_pinned` 携带 actor（或新增 `set_pinned_with_actor`，§5.2 定语义）：pin=true 写 `pinned_by` + `pinned_at_unix=now`、pin=false 归缺省；`row_to_item` / `seed_for_tests` / SELECT 投影同步
- 修改 `core/src/data_plane/memory.rs`——`memory_to_pb` 投影二字段 + `pin` RPC 传 actor（source 当前 `"console-api"`）
- 同源 Rust tests（≥3，默认构建可跑：pin 写穿 actor+timestamp / unpin 归缺省 / round-trip get+list 投影）
- proto-freeze guard 复核（`core/tests/proto_contract.rs` FROZEN 契约不退化 + 新字段为 superset 追加）

### 27.2 Pin/Unpin 拆分 + hard-delete（task-27.2）

- 修改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`——add-only `UnpinMemoryRequest`/`UnpinMemoryResponse` + `HardDeleteMemoryRequest`/`HardDeleteMemoryResponse` + `MemoryService` 加 `rpc Unpin` + `rpc HardDelete`（既有 5 RPC + `Pin` 签名不动）
- 修改 `core/src/memory/store.rs`——新增 `hard_delete(memory_id)`（`DELETE FROM memory_items WHERE memory_id=?`，物理删除；NotFound 当行不存在）
- 修改 `core/src/memoryops/audit.rs`——`AuditOperation` add-only `MemoryHardDelete` 变体 + `as_str` 映射（与既有 `MemoryPin` 等并列）
- 修改 `core/src/data_plane/memory.rs`——`MemoryServer` impl `unpin`（语义 = `set_pinned(id, false)` 显式 + emit `MemoryUnpin`）+ `hard_delete`（调 `store.hard_delete` + emit `MemoryHardDelete`）
- 修改 `internal/consoleapi/router.go` + `handlers.go`——add-only `POST /v1/memory/{id}/unpin`（non-destructive）+ `POST /v1/memory/{id}/hard-delete` 经 `confirmMiddleware`（destructive，X-Confirm gated，与 deprecate/soft-delete 同 pattern）
- 同源 Rust + Go tests（≥3：Unpin 幂等 + 显式语义 / hard-delete 物理删除后 get-by-id 返 None / console-api hard-delete 缺 X-Confirm → 412 + 带 confirm → 204）

### 27.3 is_pinned backfill from audit + closeout（task-27.3）

- 新增 backfill 逻辑（`core/src/memory/` 或 `core/src/memoryops/`，§3 据归属定）——按 `memory_pin`/`memory_unpin` audit 事件时序重放、以末次事件重建当前 `is_pinned`，opt-in 一次性 reconcile（非热路径）
- 修改 `scripts/console_smoke.sh`——v17：memory ops 硬化相关 smoke 断言（actor+timestamp 字段 round-trip / Unpin 显式路由 / hard-delete X-Confirm 412 + 物理删除 / 既有 step 不退化）
- 修改 `internal/cli/smoke_syntax_test.go`——既有 step markers 同步 + 新 step 断言
- 新增 `docs/releases/v0.20.0-{evidence,artifacts}.md` + `README.md` v0.20 段 + `RELEASE_NOTES.md` v0.20.0 段
- 修改 `docs/decisions/adr-032-memory-ops-hardening.md`——据真实结果 Proposed→Accepted（§Ratification 回填）+ ADR-022 add-only Amendment（推进三条 marker，不溯改正文 D5）
- 修改 `docs/s2v-adapter.md`（Phase 27 Draft→Done + Tasks 0→3；ADR-032 状态；ADR-022 Trade-offs marker 推进记录）

### BDD feature

- 新增 `test/features/phase-27-memory-ops-hardening.feature`（≥3 scenario：pin-actor+timestamp round-trip / Pin·Unpin 拆分 + hard-delete X-Confirm / is_pinned backfill + v0.20.0 收口）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 27.1 | `console_data_plane.proto` `MemoryItem` + `0013/0016` migration + `core/src/memory/store.rs` `set_pinned` actor/timestamp 写穿 + `data_plane/memory.rs` 投影 | `../tasks/task-27.1-memory-pin-actor-and-timestamp.md` |
| 27.2 | `console_data_plane.proto` `Unpin`/`HardDelete` RPC + `store.rs::hard_delete` + `audit.rs MemoryHardDelete` + `data_plane/memory.rs` + console-api `unpin`/`hard-delete` 路由（X-Confirm gated） | `../tasks/task-27.2-memory-pin-unpin-split-and-hard-delete.md` |
| 27.3 | is_pinned audit backfill + smoke v17 + v0.20.0 closeout + ADR-032 ratify + ADR-022 Amendment | `../tasks/task-27.3-closeout-v0.20.0.md` |

## 5. 依赖关系

- **task-27.1**（pin-actor + timestamp）dep Phase 13 task-13.1（`SqliteMemoryStore` + `memory_items` 表 + 5 RPC 已落地）+ Phase 17 task-17.1（`is_pinned` add-only 字段 + `set_pinned` 已落地）+ ADR-022（§Trade-offs marker）；可与 27.2 部分并行（同改 proto + memory.rs，task 内序列实施避免合并冲突）。
- **task-27.2**（Pin/Unpin 拆分 + hard-delete）dep task-13.1 / task-17.1 + ADR-017 D2（`confirmMiddleware` X-Confirm 兜底已落地）；建议 27.1 先 merge（proto 字段先落，27.2 加 RPC 时 proto 基线稳定）。
- **task-27.3**（closeout）dep 27.1 + 27.2 全 Done；is_pinned backfill 为本 task 子项（依赖 audit log `MemoryPin/MemoryUnpin` 事件已记录，Phase 13 既有）。
- 外部：ADR-032（本 phase 新 Proposed）/ ADR-022（is-pinned-field-amendment，本 phase 推进其 §Trade-offs 三条 marker，add-only Amendment）/ ADR-017（X-Confirm 复用）/ ADR-008（依赖变更 add-only，预期 0 新 dep）/ ADR-014 第十八次激活 / ADR-013（禁伪造凭据）/ ADR-004（本地优先，0 网络 / 默认构建 0 新 dep）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（每条 AC 含 ADR-014 D3 verified by 显式 owner；Draft 阶段未勾选，实施后逐条置 `[x]`）**：

- [ ] **AC1**：`MemoryItem` 新增 add-only `pinned_by` + `pinned_at_unix` proto 字段（序号在既有 field 10 后、不动既有 tag）+ `memory_items` add-only migration（缺省 backfill）；pin=true 写穿 actor+timestamp、pin=false 归缺省，round-trip deterministic 单测可断言；proto-freeze guard 过 + 既有字段不退化 — verified by task-27.1 §6 AC1-4 + phase-smoke step 1
- [ ] **AC2**：显式 `Unpin` RPC（add-only，既有 `Pin` 签名不动）+ `HardDelete` RPC（物理删除行）落地；console-api `POST /v1/memory/{id}/hard-delete` 经既有 `confirmMiddleware` gated（缺 X-Confirm → 412、带 confirm → 204）；拆分语义 + 物理删除 + 412 兜底 deterministic 单测可断言 — verified by task-27.2 §6 AC1-4 + phase-smoke step 2
- [ ] **AC3**：is_pinned 从 audit log 回填——按 `memory_pin`/`memory_unpin` 事件时序重放、以末次事件重建当前 `is_pinned`，opt-in 一次性 reconcile（非热路径）；deterministic 单测可断言（构造 audit 序 → backfill → is_pinned 与重放末态一致）— verified by task-27.3 §6 AC1 + phase-smoke step 3
- [ ] **AC4**：v0.20.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ `scripts/console_smoke.sh` v17（memory ops 硬化 smoke + 既有 step 不退化）+ ADR-032 据真实非合成结果 ratify 或记录维持 + ADR-022 add-only Amendment（推进三条 marker，不溯改正文 D5）+ phase §6 闭合 — verified by task-27.3 §6 AC2-3
- [ ] **AC5**：ADR-014 cross-validation gate 全套通过（第十八次激活）— D1 mapping + D2 lint `--touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-26 不溯改 — verified by task-27.3 closeout PR body

**端到端 smoke（C1 集成兜底）**：(1) `MemoryItem.pinned_by`/`pinned_at_unix` add-only 字段经 console-api pin round-trip 投影；(2) 显式 `unpin` 路由 + hard-delete X-Confirm 412 兜底 + 带 confirm 物理删除；(3) is_pinned audit backfill reconcile 结论（重放末态一致）全 PASS。

## 7. 阶段级风险

- **R1（中）proto add-only 字段序号 / freeze guard 误触**：`MemoryItem` 已用到 field 10（`is_pinned`），新字段须在其后追加、不重号、不删既有 tag。
  - **缓解**：task-27.1 按 proto 当前最大序号 +1 分配；`core/tests/proto_contract.rs` FROZEN 契约断言新字段为 superset 追加（只增不删不改 tag）；若并行 PR 占用序号则重新分配。stop-condition：proto-freeze guard 不过则 AC1 不标 `[x]`（不伪造）。
- **R2（中）`pinned_by` actor 来源真实性**：console-api 当前 source 写死 `"console-api"`（`core/src/data_plane/memory.rs:64`），并非真实 per-user 身份。
  - **缓解**：task-27.1 落「记录调用链携带的 actor」能力 + 单测（actor 经 RPC 传入并写穿）；真实 per-user 身份透传（console-api → 真实用户）`[SPEC-DEFER:phase-future.memory-actor-propagation]` 如实延后；AC1 以「actor 字段写穿 + 单测可断言」满足，actor 真实来源 caveat 在 spec §8 记录。
- **R3（中）hard-delete 不可恢复 + 误触发**：物理删除无回收站；调用方误传 `confirm=true` 会真删。
  - **缓解**：task-27.2 复用既有 `confirmMiddleware`（ADR-017 D2，X-Confirm gated，与 deprecate/soft-delete 同 destructive 确认 pattern），单测断言缺 X-Confirm → 412；级联清理（向量 / trace 引用）`[SPEC-DEFER:phase-future.memory-hard-delete-cascade]` 如实延后；hard-delete 不可恢复属设计意图（隐私基线 ADR-004），spec §8 记录。
- **R4（低）audit backfill 覆盖率**：仅能重建有 audit 记录的 item；audit log 被裁剪 / 缺失的 legacy item 无法回填。
  - **缓解**：task-27.3 backfill 仅处理有 `memory_pin`/`memory_unpin` 事件的 item，无事件的 item 保持原态（不臆造）；backfill 覆盖率 caveat 如实记录在 ADR-032 §Consequences + spec §8；AC3 以「有 audit 记录的 item 重放末态一致 + 单测可断言」满足。

## 8. Definition of Done

- 3 task spec（27.1-27.3）顶部 `**Status**: Done`
- §6 阶段级 AC1-5 全 `[x]`（受阻态按 ADR-013 如实记录，不伪造）
- 端到端 smoke 3 step 全 PASS（含受阻态如实标注）
- **ADR**：ADR-032 `Proposed → Accepted`（据真实非合成 actor+timestamp 往返 / Pin·Unpin 拆分 + hard-delete + X-Confirm 412 / is_pinned backfill 重放）或据实测记录维持 + 文档化；ADR-022 §Trade-offs 三条 marker 经 add-only Amendment 记录推进结果（不溯改正文 D1-D5，D5）；若实施确需新 dep 则 ADR-008 add-only Amendment
- **adapter**：§Phase 索引 Phase 27 `Draft → Done` + `Tasks 0 → 3`；§ADR 索引 ADR-032；§BDD 追加 phase-27 feature 行；ADR-022 Trade-offs marker 推进记录
- **release**：`docs/releases/v0.20.0-{evidence,artifacts}.md` + `RELEASE_NOTES.md` v0.20 段 + README v0.20 段
- **smoke**：`scripts/console_smoke.sh` v17（memory ops 硬化 smoke + 既有 step 不退化）+ `internal/cli/smoke_syntax_test.go` markers 同步
- **follow-up**：per-user actor 透传 `[SPEC-DEFER:phase-future.memory-actor-propagation]` + hard-delete 级联清理 `[SPEC-DEFER:phase-future.memory-hard-delete-cascade]` 留 backlog
