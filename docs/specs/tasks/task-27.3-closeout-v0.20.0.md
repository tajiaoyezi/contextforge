# Task `27.3`: `closeout-v0.20.0 — is-pinned-backfill-from-audit（按 memory_pin/memory_unpin audit 事件时序重放重建 legacy item 的 is_pinned）+ scripts/console_smoke.sh v17 memory ops 硬化 smoke + v0.20.0 release docs（README/RELEASE_NOTES/evidence/artifacts）+ ADR-032 据真实结果 ratify + ADR-022 add-only Amendment（推进 §Trade-offs 三条 marker，不溯改正文 D5）+ phase-27 §6 闭合 + adapter`

**Status**: Done

**Priority**: P2
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 27 (memory-ops-hardening)
**Dependencies**: task-27.1（pin-actor + pinned-at-timestamp add-only 字段 + 写穿）/ task-27.2（Pin/Unpin 拆分 + hard-delete + X-Confirm gated）/ task-19.7（closeout 模板 + tag/backfill pattern）/ task-23.3（前一 closeout pattern + smoke 版本递进）/ ADR-032（memory-ops-hardening，本 phase 新 Proposed）/ ADR-022（is-pinned-field-amendment，本 phase 推进其 §Trade-offs marker）/ ADR-013（禁伪造）/ ADR-014 D1-D5（第十八次激活）

## 1. Background

task-27.1 已让 `MemoryItem` 有 `pinned_by` + `pinned_at_unix` add-only 字段且 pin 写穿；task-27.2 已拆分显式 Pin/Unpin + 加 hard-delete（X-Confirm gated）。本 task 收口 Phase 27：(1) 落 **is_pinned 从 audit log 回填**——按 `memory_pin`/`memory_unpin` audit 事件时序重放、以末次事件重建 legacy item（v0.10 前 `is_pinned` 恒 false）的当前 `is_pinned`，opt-in 一次性 reconcile（非热路径，承 ADR-022 `[SPEC-DEFER:phase-future.is-pinned-backfill-from-audit]`）；(2) 把 smoke 升 v17，加 memory ops 硬化相关断言；(3) 产出 v0.20.0 release docs；(4) 据真实非合成结果 ratify ADR-032 + ADR-022 add-only Amendment 记录推进三条 marker（不溯改正文 D5）；(5) 闭合 phase-27 §6 AC；(6) 更新 s2v-adapter。

承 v0.12.0 / v0.16.0 收口模式：closeout = smoke final + release evidence/artifacts + README/RELEASE_NOTES + ADR 状态 + adapter；tag push 据用户授权 / 主 agent 自治（ADR-012）后由 release.yml 触发 + post-tag-push backfill。

## 2. Goal

落 is_pinned audit backfill：对 legacy memory items（`is_pinned=false` 但 audit log 含 `memory_pin`/`memory_unpin` 事件），按 audit 事件时序重放、以末次 pin/unpin 事件重建当前 `is_pinned`，opt-in 一次性 reconcile（非热路径）+ deterministic 单测可断言（构造 audit 序 → backfill → is_pinned 与重放末态一致）。`scripts/console_smoke.sh` 升 v17：既有 step 不退化 + 新增 memory ops 硬化 smoke（actor+timestamp 字段经 console-api pin round-trip 投影 / 显式 unpin 路由 → 204 / hard-delete 缺 X-Confirm → 412 + 带 confirm → 物理删除）。新增 `docs/releases/v0.20.0-{evidence,artifacts}.md` + `README.md` v0.20 段 + `RELEASE_NOTES.md` v0.20.0 段。`docs/decisions/adr-032-memory-ops-hardening.md` 据 task-27.1/27.2 真实结果 Status `Proposed → Accepted`（§Ratification 回填，或受阻维度记录维持）+ ADR-022 add-only Amendment 记推进 §Trade-offs 三条 marker（`pin_actor` / `memory-pinned-at-timestamp` / `is-pinned-backfill-from-audit`），不溯改 ADR-022 正文 D1-D5（D5）。`docs/specs/phases/phase-27-*.md` §6 AC1-5 全 `[x]` + Status `Draft → Done`。`docs/s2v-adapter.md` Phase 27 `Draft → Done` + Tasks `0 → 3` + ADR-032 索引 + ADR-022 Trade-offs marker 推进记录。ADR-014 D1-D5 第十八次激活 closeout PR body。D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **新增 is_pinned audit backfill 逻辑（`core/src/memory/` 或 `core/src/memoryops/`，§5.2 据归属定）**：读 audit log 中某 memory_id 的 `memory_pin`/`memory_unpin` 事件序（`AuditSink.list` + filter），按时序重放、末次 pin → `is_pinned=true`、末次 unpin → `false`，对 `is_pinned=false` 的 legacy item 执行 `set_pinned`（或直接 UPDATE）reconcile；opt-in 一次性（如 `reconcile_is_pinned_from_audit`），非热路径自动触发；无 audit 事件的 item 保持原态（不臆造）。
- **修改 `scripts/console_smoke.sh`**：v17 注释段 + 新增 memory ops 硬化 smoke 断言（pin round-trip 投影 `pinned_by`/`pinned_at_unix` / `POST /v1/memory/{id}/unpin` → 204 / `POST /v1/memory/{id}/hard-delete` 缺 X-Confirm → 412 + 带 `X-Confirm: yes` → 物理删除后 GET → 404）；既有 step 标号 / 断言不动语义；终态 marker 保留。
- **修改 `internal/cli/smoke_syntax_test.go`**：既有 step markers 同步（步号递进）+ 新 memory ops 硬化 step 断言。
- **新增 `docs/releases/v0.20.0-evidence.md` + `docs/releases/v0.20.0-artifacts.md`**：承 v0.16.0/v0.12.0 模板（合入记录 / S2V 状态 / 验证证据 / CI / AC 达成 / 平台矩阵 / upgrade-rollback / ADR-014 record / tag+镜像 SHA backfill 段）。
- **修改 `README.md`**：v0.20 段——memory ops 硬化（pin-actor/timestamp + 显式 Pin/Unpin + hard-delete X-Confirm + is_pinned audit backfill）。
- **修改 `RELEASE_NOTES.md`**：v0.20.0 段（task 表 + pin-actor/timestamp / Pin·Unpin 拆分 / hard-delete / is_pinned backfill + upgrade/rollback）。
- **修改 `docs/decisions/adr-032-memory-ops-hardening.md`**：据 task-27.1/27.2 真实结果 Status `Proposed → Accepted`（§Ratification 回填，或受阻维度记录维持）；ADR-022 §Trade-offs 三条 marker 推进以 add-only Amendment 记录（不溯改 ADR-022 正文 D1-D5，D5）；若实施确需新 dep 则 ADR-008 add-only Amendment。
- **修改 `docs/decisions/adr-022-memory-is-pinned-field-amendment.md`**：append add-only Amendment 段（记 `pin_actor`→`pinned_by` 落地 / `memory-pinned-at-timestamp`→`pinned_at_unix` 落地 / `is-pinned-backfill-from-audit` 落地；不溯改既有 D1-D5 + §Trade-offs 正文，D5）。
- **修改 `docs/specs/phases/phase-27-memory-ops-hardening.md`**：§6 AC1-5 全 `[x]` + Status `Draft → Done` + §8 DoD 勾选。
- **修改 `docs/s2v-adapter.md`**：Phase 27 行 `Draft → Done` + `Tasks 0 → 3` + Task 索引 27.1-27.3 Done + ADR-032 索引行 + BDD phase-27 feature 行 + ADR-022 Trade-offs marker 推进注。
- **新增 `test/features/phase-27-memory-ops-hardening.feature`**（≥3 scenario）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **pin-actor + pinned-at-timestamp 字段实现** [SPEC-OWNER:task-27.1-memory-pin-actor-and-timestamp]：本 task 在 smoke / release docs 引用它，不实现。
- **Pin/Unpin 拆分 + hard-delete 实现** [SPEC-OWNER:task-27.2-memory-pin-unpin-split-and-hard-delete]：本 task 引用其 RPC / 路由，不重做。
- **is_pinned backfill 覆盖 audit 缺失 / 被裁剪的 legacy item** [SPEC-DEFER:phase-future.is-pinned-backfill-from-audit]：本 task backfill 仅处理有 audit 记录的 item；无事件的 item 保持原态（audit 缺失的完整覆盖属后续）。
- **真实 per-user actor 透传** [SPEC-DEFER:phase-future.memory-actor-propagation] + **hard-delete 级联清理** [SPEC-DEFER:phase-future.memory-hard-delete-cascade]：发布后 backlog。
- **v0.20.0 tag push 实际执行**：closeout PR 合入后据用户授权 / 主 agent 自治（ADR-012）push `v0.20.0` annotated tag 触发 release.yml（沿用历史 release 流）；post-tag-push backfill 填实 tag SHA / run ID / 镜像 digest 由独立 backfill PR 承接（仿 v0.12.0/v0.16.0 pattern）。
- **multi-arch 镜像 / 签名 / SBOM** [SPEC-DEFER:phase-future.multi-arch-image] / [SPEC-DEFER:phase-future.image-signing-and-sbom]：发布硬化项，独立推进。

## 4. Actors

- **主 agent**：实施 + PR 主理 + closeout 决策（is_pinned backfill 实现 + ADR-032 ratify vs 受阻维度记录维持）。
- **`core/src/memory/` / `core/src/memoryops/`**：is_pinned audit backfill 落点。
- **`core/src/memoryops/audit.rs::AuditSink`**：audit 事件源（`memory_pin`/`memory_unpin` 事件），本 task backfill 读它重放。
- **`scripts/console_smoke.sh`**：端到端 C1 兜底 smoke，本 task 升 v17。
- **`docs/releases/` + `README.md` + `RELEASE_NOTES.md`**：v0.20.0 release 文档面。
- **`docs/decisions/adr-032-*.md` + `adr-022-*.md`**：本 phase 新 ADR ratify + ADR-022 add-only Amendment。
- **`docs/s2v-adapter.md`**：Phase/task/ADR/BDD 索引。
- **用户**：v0.20.0 tag push 授权（或主 agent 自治 ADR-012）。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/tasks/task-19.7-closeout-v0.12.0.md`（closeout 模板 + tag/backfill pattern）+ `docs/specs/tasks/task-23.3-closeout-v0.16.0.md`（前一 closeout pattern + smoke 版本递进 + ADR ratify/Amendment pattern）
- `docs/releases/v0.16.0-evidence.md` + `docs/releases/v0.12.0-evidence.md`（release 文档结构 + 平台矩阵 + backfill 段）
- `scripts/console_smoke.sh`（既有 step + 终态 marker + 版本注释段 v13 当前）+ `internal/cli/smoke_syntax_test.go`（step marker 同步 pattern）
- `docs/specs/tasks/task-27.1-memory-pin-actor-and-timestamp.md` + `task-27.2-memory-pin-unpin-split-and-hard-delete.md`（本 phase 上游交付）
- `core/src/memoryops/audit.rs:11-37`（`AuditOperation::MemoryPin/MemoryUnpin` + `as_str`）+ `:195-217`（`AuditSink.list` + `count_by_operation` 读 audit 事件 pattern）
- `core/src/memory/store.rs:153-165`（`set_pinned` reconcile 写入点）
- `docs/decisions/adr-032-memory-ops-hardening.md`（D3 backfill + §Ratification 回填）+ `docs/decisions/adr-022-memory-is-pinned-field-amendment.md`（§Trade-offs 三条 marker + Amendment append 锚点）+ `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5）
- `docs/s2v-adapter.md` §Phase / §Task / §ADR / §BDD 索引

### 5.2 关键设计 — is_pinned backfill + smoke v17 + ADR ratify

- **is_pinned audit backfill**：audit log 已记 `memory_pin`/`memory_unpin` 事件（`AuditOperation::MemoryPin/MemoryUnpin`，`audit.rs:19-22`，每事件 `chunk_ids=[memory_id]`）。backfill 路径：枚举 memory item → 读其 audit 事件序（按 timestamp / id 升序）→ 末次事件是 pin → `is_pinned=true`、末次是 unpin → `false`、无事件 → 保持原态（不臆造）→ 对需修正的 item 执行 `set_pinned`（或直接 UPDATE）。opt-in 一次性（`reconcile_is_pinned_from_audit` 显式调用），非热路径自动触发（避免每请求重放成本）。**落点**：优先 `core/src/memoryops/`（audit-aware reconcile，与 AuditSink 同模块）或 `core/src/memory/`（store-level reconcile），§10 据归属定。deterministic 单测：构造已知 audit 序（pin, unpin, pin）+ `is_pinned=false` item → backfill → 断言 `is_pinned=true`（末态 = 末次 pin）。
- **smoke v17**：新增 memory ops 硬化 smoke——console-api pin round-trip 投影 `pinned_by`/`pinned_at_unix`（POST pin → GET 断言字段非缺省）；`POST /v1/memory/{id}/unpin` → 204；`POST /v1/memory/{id}/hard-delete` 缺 X-Confirm → 412 + 带 `X-Confirm: yes` → 204 + 后续 GET → 404（物理删除坐实）。既有 step 断言不动；终态 marker 保留。版本注释承 v13（Phase 23）递进到 v17。
- **ADR-032 ratify（ADR-013）**：据 task-27.1 真实 actor+timestamp 写穿往返 + task-27.2 真实 Pin/Unpin 拆分 + hard-delete 物理删除 + X-Confirm 412 + 本 task is_pinned backfill 重放 Proposed→Accepted（§Ratification 回填）；若某维度受阻则据「已达维度 ratify + 受阻维度如实记录」处理，不据合成 / 伪造 ratify。
- **ADR-022 add-only Amendment**：append Amendment 段记三条 §Trade-offs marker 落地（`pin_actor`→`pinned_by` / `memory-pinned-at-timestamp`→`pinned_at_unix` / `is-pinned-backfill-from-audit`→本 task backfill），不溯改 ADR-022 正文 D1-D5 + §Trade-offs（D5）。

### 5.3 不变量

- smoke 既有 step 不退化（仅新增 memory ops 硬化 step + v17 注释）。
- release docs 诚实口径（承 task-19.7 / task-23.3 §10）：deterministic 默认 / 受阻三态如实标；actor 真实来源 + audit backfill 覆盖率 caveat 如实记录，不伪造。
- ADR-032 ratify 仅在 task-27.1/27.2 真实落地后（ADR-013：据真实非合成）；受阻维度不强 ratify。
- is_pinned backfill 不臆造无 audit 记录 item 的状态（保持原态）；非热路径自动触发（opt-in 一次性 reconcile）。
- 默认构建 0 新依赖 + 0 网络（ADR-004）+ 既有 5 memory RPC 行为不变。

## 6. Acceptance Criteria

- [x] **AC1**: is_pinned audit backfill 落地——按 `memory_pin`/`memory_unpin` 事件时序重放、以末次事件重建 legacy item 当前 `is_pinned`，opt-in 一次性 reconcile（非热路径）；deterministic 单测可断言（构造 audit 序 → backfill → is_pinned 与重放末态一致 + 无 audit 事件 item 保持原态）；`scripts/console_smoke.sh` v17 通过 `bash -n`（exit 0）+ memory ops 硬化 smoke 断言 + 既有 step 不退化 — verified by **TEST-27.3.1**
- [x] **AC2**: v0.20.0 release docs 齐备（`docs/releases/v0.20.0-{evidence,artifacts}.md` + `README.md` v0.20 段 + `RELEASE_NOTES.md` v0.20.0 段）；evidence 含 task 表 / CI / AC 达成 / 平台矩阵 / upgrade-rollback / §tag-backfill 待回填段 — verified by **TEST-27.3.2**
- [x] **AC3**: ADR-032 据 task-27.1/27.2 真实结果 Status `Proposed → Accepted`（或受阻维度记录维持）+ §Ratification 回填；ADR-022 add-only Amendment 记推进 §Trade-offs 三条 marker（不溯改正文 D1-D5）；phase-27 §6 AC1-5 全 `[x]` + Status `Draft → Done`；adapter Phase 27 `Draft → Done` + Tasks `0 → 3` + ADR-032 索引 + ADR-022 Trade-offs marker 推进注 — verified by **TEST-27.3.3**
- [x] **AC4**: 既有不退化 — 默认 `cargo test --workspace`（0 新依赖）+ `go test ./...` 全 PASS；既有 5 memory RPC + `confirmMiddleware` destructive 412 + proto-freeze guard 不退化 — verified by **TEST-27.3.4** + §10
- [x] **AC5**: ADR-014 D1-D5 第十八次激活全通过（D1 phase§6↔task§6 mapping 表 + D2 lint `--touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-26 不溯改）— verified by **TEST-27.3.5** + 本 closeout PR body

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-27.3.1 | is_pinned audit backfill 重放末态一致 + 无事件保持原态 + smoke v17 `bash -n` + memory ops 硬化断言 | `core/src/memoryops/`（或 `core/src/memory/`）+ `internal/cli/smoke_syntax_test.go` + `scripts/console_smoke.sh` | Done |
| TEST-27.3.2 | v0.20.0 release docs 齐备 + 结构校验 | `docs/releases/v0.20.0-*.md` + README + RELEASE_NOTES | Done |
| TEST-27.3.3 | ADR-032 ratify + ADR-022 add-only Amendment + phase-27 闭合 + adapter | `docs/decisions/adr-032-*.md` + `adr-022-*.md` + phase-27 spec + s2v-adapter | Done |
| TEST-27.3.4 | 默认 `cargo test --workspace` + `go test ./...` 0 failed + 既有 memory RPC/confirm/proto-freeze 不退化 | 全 Rust + Go | Done |
| TEST-27.3.5 | ADR-014 D1-D5 record（mapping + D2 lint） | 本 closeout PR body | Done |

## 8. Risks

- **R1（中）is_pinned backfill 覆盖率**（承 phase-27 §7 R4）：audit log 被裁剪 / 缺失的 legacy item 无法回填。
  - **缓解**：backfill 仅处理有 `memory_pin`/`memory_unpin` 事件的 item，无事件的 item 保持原态（不臆造）；backfill 覆盖率 caveat 如实记录在 ADR-032 §Consequences + 本 spec §10；AC1 以「有 audit 记录的 item 重放末态一致 + 单测可断言」满足。
- **R2（中）ADR-032 某维度依赖受阻**：ratify 须真实结果（task-27.1/27.2 真实落地）。
  - **缓解**：ADR-032 据「actor+timestamp 写穿已达 + Pin/Unpin 拆分 + hard-delete + X-Confirm 412 已达 + is_pinned backfill 已达」处理——已达维度 ratify，受阻维度如实记录（ADR-013），不据合成 ratify。
- **R3（低）smoke v17 hard-delete 物理删除污染 smoke fixtures**：物理删除不可恢复，可能影响后续 step。
  - **缓解**：hard-delete smoke 用独立 seed 的临时 memory_id（不删 smoke 主 fixture）；删后 GET 404 断言坐实物理删除；既有 step 用既有 fixture 不受影响。
- **R4（低）v0.20.0 tag 误在授权前 push / smoke 版本号断档**：release stop-condition + smoke 版本承接。
  - **缓解**：closeout PR 仅备齐 release docs；tag push 据用户授权 / 主 agent 自治（ADR-012）后单独执行；smoke 版本注释承前一 closeout（v13 Phase 23）递进到 v17，step 标号同步 `smoke_syntax_test.go`。

## 9. Verification Plan

```bash
# smoke v17 语法 + step 标号
bash -n scripts/console_smoke.sh
go test ./internal/cli/... -run 'TestTask27|Smoke' -v

# is_pinned audit backfill（默认构建 0 新依赖）
cargo test -p contextforge-core memoryops
cargo test -p contextforge-core memory

# 既有不退化
go test ./...
cargo test --workspace

# 端到端 smoke（合规环境；memory ops 硬化 round-trip）
bash scripts/console_smoke.sh        # 期望末行 CONSOLE_REAL_SMOKE_EXIT=0

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **Status**: Done（2026-06-01）。
- **完成日期**：2026-06-01。
- **改动文件**：
  - `core/src/memory/store.rs`——`reconcile_is_pinned_from_audit(&[AuditLogEntry])`（按 memory_id 分组 `memory_pin`/`memory_unpin`，last 胜，仅修正不一致的存在行，无事件 item 保持原态）+ 同源测试。
  - `scripts/console_smoke.sh`——v17 注释段 + banner line2 + `[36/36]` step 36（REAL mode pin-actor round-trip + unpin 204 + hard-delete 412→204→404）；`internal/cli/smoke_syntax_test.go`——`TestTask273_*`。
  - `docs/releases/v0.20.0-{evidence,artifacts}.md`（新增）+ `README.md` v0.20 段 + `RELEASE_NOTES.md` v0.20.0 段。
  - `docs/decisions/adr-032-memory-ops-hardening.md`（Proposed→Accepted + §Ratification D1-D4）+ `docs/decisions/adr-022-memory-is-pinned-field-amendment.md`（§Amendment Phase 27 add-only）。
  - `docs/specs/phases/phase-27-memory-ops-hardening.md`（§6 AC1-5 全 `[x]` + Status Done）+ `docs/s2v-adapter.md`（Phase 27 Draft→Done + Tasks 0→3 + 27.1-27.3 Done + ADR-032 Accepted）。
- **commit 列表（RED→GREEN）**：RED `test(memory): TEST-27.3.1 RED`（`reconcile_is_pinned_from_audit` todo!() + 测试）→ GREEN `feat(memory): is_pinned 从 audit log 回填`（last-event-wins 实现）→ `test(smoke): console_smoke v17 step 36` → docs（本回填）。
- **§9 Verification 实测结果（ADR-013 真实非合成）**：`cargo test -p contextforge-core --lib memory::store` **15 passed**（含 backfill last-event-wins / 无事件不变 / 非 pin op 忽略）；`cargo test --workspace` + `go test ./...` 0 failed；`bash -n scripts/console_smoke.sh` exit 0（`TestTask273` marker + 语法门）；上游真实凭据：task-27.1 store 15/15 + data_plane 14/14 + proto_contract（#181）+ task-27.2 store/data_plane 14/14 + go consoleapi 412→204→404（#183）。
- **设计取舍**：(1) **backfill 落点 `core/src/memory/store.rs`**（store-level reconcile，取 `&[AuditLogEntry]` 解耦 AuditSink，可直接单测）；按 memory_id 分组 pin/unpin（id ASC，last 胜），**仅修正 `is_pinned`（+ updated_at）不臆造历史 actor/timestamp**（legacy audit 未记 actor——即缺口本身）；无 pin/unpin 事件的 item 保持原态（不臆造）；opt-in 一次性（非热路径自动跑）。(2) **smoke v17 step 36** REAL mode 活跃断言 console-api memory ops live round-trip（复用 `mem-seed-*` fixtures：pin-actor 投影 + unpin 204 + hard-delete 412→204→404 物理删除坐实）；非 REAL 注 contract-layer 验证。(3) **ADR-032 据 D1-D4 真实非合成验证 Accepted**；actor 真实来源 + backfill 覆盖率据真实受限**如实记录**不伪造。(4) **ADR-022 add-only Amendment** 推进三条 marker（不溯改正文 D1-D5，D5）。(5) **tag push 据 ADR-012 主 agent 自主决断**（无人值守授权）。
- **剩余风险 + 下游影响**：backfill 覆盖率 caveat（仅有 audit 记录的 item，`[SPEC-DEFER:phase-future.is-pinned-backfill-from-audit]` 完整覆盖延后）+ per-user actor 透传 `[SPEC-DEFER:phase-future.memory-actor-propagation]` + hard-delete 级联清理 `[SPEC-DEFER:phase-future.memory-hard-delete-cascade]` 留 backlog；v0.20.0 tag push + post-tag-push backfill 回填 tag SHA / run ID / 镜像 digest。
