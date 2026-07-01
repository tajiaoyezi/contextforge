# Phase 43 · governance-debt-cleanup-4

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。本 phase 是继 Phase 31（governance-debt-cleanup, Done）+ Phase 33（governance-debt-cleanup-2, Done）+ Phase 40（governance-debt-cleanup-3, Done）后的**第四轮治理债清扫**（镜像 ADR-036 / ADR-038 / ADR-045 的「核实-诚实化-补全」打法），**单聚焦 `indexing-replay-e2e` 拼接缺口**——承 Phase 33 task-33.3（indexing event 持久化 + replay mapper，ADR-038 D3）血脉的"最后一公里"：mapper `indexing_rows_to_pb_events`（`core/src/data_plane/events.rs:438`）已写好并经 `test_33_3_2` 验证，但**未接进 live subscribe 路径**（`events.rs:241-250` 的 replay splice 只接了 memory audit replay，漏了 indexing）。4 个拼接缺口（grounding 已亲自核实）：(1) `SqliteIndexingEventStore::list(limit)`（`indexing_events.rs:111`）缺 `since_ts` 参数；(2) `DataPlaneStores`（`mod.rs:43-74`）无 `indexing_event_store` 字段；(3) `serve_full`（`server.rs:788-798`）`DataPlaneStores::full()` 未传入已构造的 store；(4) `EventsServer::subscribe`（`events.rs:241-250`）replay splice 只接 audit 未接 indexing。code-local 🟢 可单测，0 新 dep（ADR-008）+ 0 schema migration（复用 Phase 33 migration 0019）+ 0 proto 改动。**关键诚实定性（ADR-013，本 phase 的核心价值）**：本 phase 交付 splice **拼接**（接进 live subscribe 路径 + since_ts 时序对齐 audit + 默认 byte-equiv），🟢 纯本地单测守护时序 + 拼接 + 退化 byte-equiv；live daemon restart-then-replay 端到端 e2e（真起进程 + 跨 restart 双窗口）须 running daemon（须 console 跨进程）→ 🟡 honest-defer `[SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]` 不预填；memory-actor-all-rpc（Deprecate/SoftDelete 7 层改动 + 新 schema migration / HardDelete 须 audit 层重设计）据实 honest-defer 留独立 phase，不在本 phase 强行扩面。默认行为 / proto / 既有契约不变（ADR-004：`since_ts<=0` / `store=None` 两条退化路径 byte-equiv）；Phase 43 = **0 新依赖**（ADR-008）；既有 `cargo-test` / `go-test` / `spec-lint` 三门不退化。

> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md §3.25 + §4 backlog` → 各债项源码锚点（`core/src/data_plane/indexing_events.rs:111`（`list(limit)` 缺 since_ts）+ `:79-107`（`append` 写路径，已在）+ `:172 test_33_3_2`（store round-trip 镜像源）/ `core/src/data_plane/mod.rs:43-74`（`DataPlaneStores` 字段列表，缺 indexing_event_store）+ `:156-178`（`full()` constructor）/ `core/src/server.rs:756-762`（`indexing_event_store` 局部构造 + 传 `IndexSessionBackend`）+ `:788-798`（`DataPlaneStores::full()` 9 参数未传 store）/ `core/src/data_plane/events.rs:223-308`（`subscribe` replay 段 `:241-250` 只接 audit）+ `:394-423`（`replay_events_from_audit` since_ts 范式镜像源）+ `:438-474`（`indexing_rows_to_pb_events` mapper 已写好））→ AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5，**第三十四次**激活）→ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造红线：splice 真实接入非合成、since_ts 时序单测守护；live daemon e2e 🟡 据实延后不预填；memory-actor 据实延后不强行扩面）。

> **ADR 影响面（已识别）**：
> - **ADR-048 indexing-replay-splice（新，Proposed）**：记 indexing replay splice 接进 live subscribe（list_since + DataPlaneStores 字段 + serve_full 接线 + subscribe splice；live daemon e2e 据实延后，D1-D4）+ 默认行为 byte-equiv + 0 新 dep / 0 migration / 0 proto。Status: Proposed（Draft 阶段不 ratify；ratify 在 task-43.3 closeout）。
> - 触及 **ADR-038（governance-debt-cleanup-2 — D3 indexing event 持久化 + replay mapper）**：Phase 33 D3 标 `[SPEC-DEFER:phase-future.indexing-replay-e2e]`（mapper 🟢 已达 / e2e 🟡 未跑）——本 phase 以 add-only Amendment 记其 splice 维度兑现（mapper 接进 live subscribe + since_ts 时序），不溯改 ADR-038 正文（ADR-014 D5）。
> - 触及 **ADR-031（observability-hardening — replay 范式源 task-26.2）**：本 phase 复用 task-26.2 建立的 since_ts 守护 + best-effort `unwrap_or_default()` + subscribe-first 模式——以引用而非 Amendment（ADR-031 正文不动）。
> - 触及 **ADR-021（memory-event-bridge — audit replay splice 镜像源）**：本 phase 把 indexing replay 对称接进同一 subscribe 路径——以引用而非 Amendment。
> - 触及 **ADR-004（默认行为 + 既有契约不变）**：`since_ts<=0` / `store=None` 两条退化路径 byte-equiv——默认行为 / 既有契约不变（守线，非推翻）。

## 1. 阶段目标

v0.35.0 ship 后，ContextForge 进行第四轮治理债清扫，**单聚焦**把 Phase 33 task-33.3 交付但未接进 live 路径的 indexing replay mapper 拼接进 `EventsServer::subscribe`，使 `since_ts > 0` 的订阅者能收到 missed 的 `indexing.progress`/`.cancelled`/`.error` 生命周期事件（与既有 memory audit replay 对称）。code-local 🟢 可单测，0 新 dep + 0 schema migration + 0 proto 改动。**关键诚实定性**：交付 splice 拼接 + unit 级时序单测；live daemon restart-then-replay 端到端 e2e 🟡 据实延后 `[SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]`；memory-actor-all-rpc（非小债）据实延后留独立 phase。默认行为 / proto / 既有契约不变（ADR-004）；Phase 43 = 0 新依赖（ADR-008）；既有三门不退化。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. **indexing replay splice 接进 live subscribe**：`SqliteIndexingEventStore` add `list_since(limit, since_ts)`（since_ts 时序过滤镜像 audit）+ `DataPlaneStores` add `indexing_event_store: Option<Arc<...>>` 字段 + `full()` 加参数 + `serve_full` 传入已构造 store + `subscribe` replay 段 splice indexing replay（since_ts>0 时 list_since + mapper，在 audit replay 后、live forward 前）；`since_ts>0` 订阅者收到 indexing replay 事件序列（AC1）
2. **默认 byte-equiv + 时序正确**：`since_ts<=0` → 无 indexing replay（byte-equiv 现状）；`store=None` → 无 indexing replay（退化现状）；replay batch 内 indexing/audit 两类各 id ASC / ts ASC，客户端按 event_id dedup（AC1）
3. **honest-defer 边界 + v0.36.0 closeout + 默认零依赖守线**：live daemon restart-then-replay e2e `[SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]` 据实延后；memory-actor-all-rpc 据实延后留独立 phase；默认行为 / proto / 既有契约不变 + 0 网络（ADR-004）+ 0 新依赖（ADR-008）+ 0 schema migration（复用 0019）；v0.36.0 release docs + `scripts/console_smoke.sh` v33[52/52] + ADR-048 据真实测试 ratify + ADR-038 add-only Amendment + roadmap §3.25/§4 + phase §6 闭合（AC2）
4. ADR-014 D1-D5（**第三十四次**激活）全通过（AC3）

**v.x 版本号决策**：v0.36.0（Phase 43，承 v0.35.0；roadmap §1.1 Phase N→v0.(N-7).0），theme governance-debt-cleanup-4。minor release（第四轮治理债清扫，单聚焦 indexing-replay splice 拼接；0 新 dep / 0 schema migration / 0 proto / 默认 byte-equiv 不变）。

## 2. 业务价值

第四轮治理债清扫——补齐「indexing replay mapper 接进 live subscribe 路径」这一"最后一公里"拼接缺口，使 Phase 33 投入的 mapper 不再是死代码：

### 43.1 indexing-replay-splice（indexing-replay-e2e splice 维度，🟢）

- Phase 33 task-33.3（ADR-038 D3）交付了 indexing event 的**持久化**（migration 0019 + `SqliteIndexingEventStore` + 4 emit 点）+ **replay mapper**（`indexing_rows_to_pb_events`，真实 job_id/processed/total 取持久行不合成，`test_33_3_2` 守护）。但 mapper **从未在 live subscribe 路径被调用**——`EventsServer::subscribe`（`events.rs:223-308`）的 replay 段（`:241-250`）只 splice 了 memory audit replay（`self.stores.audit`），漏了 indexing。结果：`since_ts > 0` 的订阅者能收到 missed 的 memory 事件，但**收不到** missed 的 indexing 事件。
- **根因（4 缺口，grounding 已亲自核实）**：(1) `list(limit)` 缺 since_ts 参数；(2) `DataPlaneStores` 无 `indexing_event_store` 字段；(3) `serve_full` `DataPlaneStores::full()` 未传入已构造 store（store 在 `:756` 局部构造传给了 `IndexSessionBackend` 写路径，但没 clone 进 DataPlaneStores 读路径）；(4) `subscribe` replay splice 只接 audit。
- 本 phase 补 4 缺口：`list_since(limit, since_ts)`（since_ts 过滤镜像 audit `ts < since_ts → skip`）+ `DataPlaneStores` 加字段 + `full()` 加参数（既有 constructor 补 None byte-equiv）+ `serve_full` 传入 + `subscribe` splice indexing replay（since_ts>0 时，audit replay 后、live forward 前）。
- **HONEST CAVEAT（不夸大，ADR-013）**：本 phase 交付 splice **拼接** + unit 级时序单测（🟢）；live daemon restart-then-replay 端到端 e2e（真起进程 + 跨 restart 双窗口断言）须 running daemon（须 console 跨进程）→ 🟡 honest-defer `[SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]` 不预填。

**不在本 phase 范围**：

- live daemon restart-then-replay 端到端 e2e（真起进程 + 跨 restart 双窗口断言）[SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]
- memory-actor-all-rpc 四 RPC（Deprecate/SoftDelete 7 层改动 + 新 schema migration / HardDelete 须 audit 层重设计；Unpin 单 RPC store 层已有 slot 但本 phase 单聚焦不扩面）[SPEC-DEFER:phase-future.memory-actor-all-rpc]
- memory actor 认证身份（须 console-api 鉴权层）[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]
- 其余 roadmap §4 backlog 项（vector-dim-feature-enforce / tracestore-multi-workspace-strict 余下读路径 / 等）据实保持延后

## 3. 涉及模块

### 43.1 indexing-replay-splice（task-43.1）

- 修改 `core/src/data_plane/indexing_events.rs`——add `pub fn list_since(&self, limit: usize, since_ts: i64) -> Result<Vec<IndexingEventRow>, IndexingEventStoreError>`（`since_ts > 0` 时 `WHERE ts_unix >= ? ORDER BY id ASC LIMIT ?`，镜像 `replay_events_from_audit` 的 since_ts 语义；`since_ts <= 0` 时不过滤返全量 limit，与既有 `list()` 一致）；既有 `list(limit)` 保留不动；同源测试 TEST-43.1.1（list_since 时序过滤：append 3 行不同 ts → since_ts 过滤 + id ASC）
- 修改 `core/src/data_plane/mod.rs`——`DataPlaneStores` add `pub indexing_event_store: Option<Arc<crate::data_plane::indexing_events::SqliteIndexingEventStore>>` 字段；`full()` constructor 加第 10 参数；所有既有 constructor（`new`/`with_eval`/`with_memory`/`with_runner`/`with_runner_and_bus`）补 `indexing_event_store: None`（byte-equiv）
- 修改 `core/src/server.rs`——`serve_full` `DataPlaneStores::full(...)` 第 10 参数传 `Some(indexing_event_store.clone())`（store 已在 `:756` 构造，clone 进 DataPlaneStores 读路径）
- 修改 `core/src/data_plane/events.rs`——`subscribe` replay 段（`:241-250`）after audit replay 加 indexing replay：`since_ts > 0` 时 `self.stores.indexing_event_store.as_ref().and_then(|s| s.list_since(REPLAY_LIMIT, req.since_ts).ok()).map(|rows| indexing_rows_to_pb_events(&rows)).unwrap_or_default()`；合并进 `replay: Vec<PbEvent>`；TEST-43.1.2（subscribe 带 since_ts → 先 indexing replay 再 audit replay 再 live；since_ts<=0 无 replay byte-equiv；store None 无 indexing replay）
- 同源验证（≥2，🟢：list_since 时序过滤（TEST-43.1.1）+ subscribe splice 时序 + 默认 byte-equiv（TEST-43.1.2））

### 43.2 closeout（task-43.3）

- 修改 `scripts/console_smoke.sh`——banner v32→v33 + v33 changelog block + 新 step [52/52]（indexing replay splice 可达则断言、否则 doc/status；current Phase 42 [51/51] → Phase 43 顺位 [52/52]）
- 修改 `internal/cli/smoke_syntax_test.go`——新 `TestTask433`（镜像 `TestTask423`）断言 [52/52] + no-regression（denominators [37/37]..[51/51] 不溯改，ADR-014 D5）
- 新增 `docs/releases/v0.36.0-evidence.md` + `v0.36.0-artifacts.md`（tag SHA / run id / digest 为 angle-bracket backfill marker）+ `README.md` v0.36 段 + `RELEASE_NOTES.md` v0.36.0 段
- 修改 `docs/decisions/adr-048-indexing-replay-splice.md`——Status Proposed→Accepted（逐 D 如实）+ 新 `## Ratification（v0.36.0 / task-43.3）`
- add-only Amendment（非正文改，ADR-014 D5）：`adr-038`（governance-debt-cleanup-2，indexing-replay-e2e splice 维度兑现 add-only Amendment）；`docs/roadmap.md §3.25/§4` add-only（Phase 43 行 + indexing-replay-daemon-e2e 新 backlog 条目）
- 修改 `docs/specs/phases/phase-43-governance-debt-cleanup-4.md`——Status Draft→Done + §6 AC 勾选（逐维如实）
- 修改 `docs/s2v-adapter.md`——Phase 43 行 + Task 行 + ADR-048 行 + BDD 行

### BDD feature

- 新增 `test/features/phase-43-governance-debt-cleanup-4.feature`（≥2 scenario：indexing replay splice 接进 live subscribe（since_ts>0 订阅者收到 indexing replay 事件序列 + 时序 indexing→audit→live）/ 默认 byte-equiv + honest-defer（since_ts<=0 无 replay byte-equiv / store=None 退化 / live daemon e2e 🟡 延后 / memory-actor 据实延后））

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 43.1 | `indexing_events.rs` add `list_since(limit, since_ts)` + `DataPlaneStores` add `indexing_event_store` 字段 + `full()` 加参数 + `server.rs` serve_full 传入 + `events.rs` subscribe splice indexing replay + TEST-43.1.1/.2 | `../tasks/task-43.1-indexing-replay-splice.md` |
| 43.3 | smoke v33[52/52] + v0.36.0 closeout + ADR-048 ratify + ADR-038 add-only Amendment + roadmap §3.25/§4 add-only + s2v-adapter add-only | `../tasks/task-43.3-closeout-v0.36.0.md` |

> Phase 43 实际为 2 task（43.1 实现 + 43.3 closeout），承 Phase 33/40 等小 phase 惯例（实现 1 + closeout 1）。无 43.2（规划时单聚焦不拆第二实现 task）。

## 5. 依赖关系

- **task-43.1**（indexing-replay-splice）dep 既有 `core/src/data_plane/indexing_events.rs` `SqliteIndexingEventStore`（task-33.3 已在）+ `list(limit)`（task-33.3 已在，本 task 加 list_since 不改 list）+ `indexing_rows_to_pb_events` mapper（`events.rs:438`，task-33.3 已在 + `test_33_3_2` 守护）+ `DataPlaneStores`（task-11.1 起的 store 注入点）+ `serve_full` `indexing_event_store` 局部构造（task-33.3 `:756` 已在）+ `subscribe` replay 段（task-26.2 audit replay `:241-250` 已在，本 task splice 对称项）+ `replay_events_from_audit` since_ts 范式（task-26.2 `:394` 镜像源）；无外部 dep。
- **task-43.3**（closeout）dep 43.1 Done；release docs / smoke v33[52/52] / ADR-048 ratify 据 task-43.1 真实测试产物。
- 外部：ADR-048（本 phase 新 Proposed）/ ADR-038（indexing-replay-e2e splice 维度兑现 add-only Amendment）/ ADR-031（replay 范式源引用）/ ADR-021（audit replay splice 镜像源引用）/ ADR-004（默认行为 byte-equiv + 既有契约不变）/ ADR-008（dep add-only，Phase 43 不增 dep）/ ADR-012（tag/release outward-facing 须用户显式授权）/ ADR-014 **第三十四次**激活 / ADR-013（禁伪造红线，splice 真实接入非合成、since_ts 时序单测守护；live daemon e2e 🟡 据实延后不预填；memory-actor 据实延后不强行扩面）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（每条 AC 含 ADR-014 D3 verified by 显式 owner；Draft 阶段未勾选，实施后逐条置 `[x]`）**：

- [ ] **AC1**（indexing replay splice 接进 live subscribe 🟢）: `SqliteIndexingEventStore` add `list_since(limit, since_ts)`（since_ts>0 时 `WHERE ts_unix >= ?` 镜像 audit，since_ts<=0 不过滤）；`DataPlaneStores` add `indexing_event_store: Option<Arc<...>>` 字段 + `full()` 加参数（既有 constructor 补 None byte-equiv）；`serve_full` 传入 `Some(indexing_event_store.clone())`；`subscribe` replay 段 splice indexing replay（since_ts>0 时 list_since + mapper，audit replay 后、live forward 前）；`since_ts>0` 订阅者收到 indexing replay 事件序列；`since_ts<=0` / `store=None` 两条退化路径 byte-equiv；0 新 dep / 0 schema migration / 0 proto — verified by **TEST-43.1.1**（list_since 时序过滤）+ **TEST-43.1.2**（subscribe splice 时序 indexing→audit→live + since_ts<=0 byte-equiv + store=None 退化）+ phase-smoke step 1
- [ ] **AC2**（honest-defer 边界 + v0.36.0 closeout + 默认零依赖守线）: live daemon restart-then-replay e2e `[SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]` 据实延后；memory-actor-all-rpc 据实延后留独立 phase；默认行为 / proto / 既有契约不变 + 0 网络（ADR-004）+ 0 新依赖（ADR-008）+ 0 schema migration（复用 0019）；v0.36.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ `scripts/console_smoke.sh` v33[52/52] + `internal/cli/smoke_syntax_test.go` `TestTask433` markers 同步（no-regression [37/37]..[51/51]）+ ADR-048 据真实测试 ratify + ADR-038 add-only Amendment + roadmap §3.25/§4 add-only + phase §6 闭合 — verified by **TEST-43.3.1**（smoke v33[52/52] + smoke_syntax_test + ADR-048 ratify + roadmap/adapter add-only + phase §6 闭合）
- [ ] **AC3**（ADR-014 cross-validation gate）: ADR-014 D1-D5（**第三十四次**激活）全通过 — D1 mapping + D2 lint `bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-42 不溯改（ADR 改动 add-only Amendment）— verified by task-43.3 closeout PR body + 各 task LAST TEST（TEST-43.1.3 / TEST-43.3.2）

**端到端 smoke（C1 集成兜底）**：(1) `SqliteIndexingEventStore::list_since` since_ts 过滤 + `DataPlaneStores.indexing_event_store` 接线 + `subscribe` splice indexing replay（since_ts>0 订阅者收到 indexing replay 事件序列 + 时序 indexing→audit→live）+ since_ts<=0 / store=None byte-equiv 全 PASS（live daemon e2e 🟡 据实延后如实标注）；(2) v0.36.0 收口 + 默认零依赖守线全 PASS。

## 7. 阶段级风险

- **R1（中）splice 时序错误（indexing/audit/live 顺序）**：replay batch 内 indexing/audit 两类 + live 三路顺序若错，会导致订阅者看到乱序事件或 dedup 失败。
  - **缓解**：splice 严格在 audit replay 之后、live forward（`:251` spawn）之前（subscribe-first `:235` 保证不丢 live）；两类 replay 各自 id ASC / ts ASC 内部有序；客户端按 `event_id`（`evt-idx-{id}` vs `evt-audit-{id}`）dedup splice 边界（两类命名空间独立）；TEST-43.1.2 断言时序 indexing→audit→live。stop-condition：时序错乱 / dedup 失败则 AC1 不标 `[x]`。
- **R2（中）默认行为回归（since_ts<=0 / store=None 非 byte-equiv）**：splice 逻辑若在 since_ts<=0 或 store=None 时仍发 indexing replay，会破默认行为。
  - **缓解**：splice 仅 `req.since_ts > 0` 时生效（与既有 audit replay `:241` 同守护）；`indexing_event_store.as_ref()` None 时 `unwrap_or_default()` 空切片；TEST-43.1.2 断言 since_ts<=0 无 replay byte-equiv + store=None 退化。stop-condition：退化路径非 byte-equiv 则 AC1 不标 `[x]`。
- **R3（低）live daemon e2e 被误读为已交付**：本 phase 交付 unit 级 splice + 时序单测，live daemon restart-then-replay e2e 未跑，易被误读为完整 e2e 已验。
  - **缓解**：spec §2 43.1 + ADR-048 D4 据实记「splice 拼接 🟢 / live daemon e2e 🟡 honest-defer」；task-43.3 closeout 据 unit 级已达维度 ratify + 如实记录 e2e 受阻（ADR-013 不预填）。stop-condition：若把 unit splice 夸大为 live e2e 已验则越界。
- **R4（低）memory-actor 被误读为本 phase 范围**：grounding 显示 memory-actor-all-rpc 真实范围大（7 层改动 + migration），易被误读为本 phase 应一并做。
  - **缓解**：spec §2 范围外 + ADR-048 A3 据实记「memory-actor 非小债，据实延后留独立 phase」（roadmap §3.17/§3.22 "据实排小不凑数"）；本 phase 单聚焦 indexing-replay 不扩面。stop-condition：若强行扩面 memory-actor 则违 ADR-013 刻意小原则。

## 8. Definition of Done

- 2 task spec（43.1 / 43.3）顶部 `**Status**: Done`
- §6 阶段级 AC1-3 全 `[x]`（受阻 / 延后态按 ADR-013 如实记录，不伪造——如 live daemon e2e 据实延后 `[SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]`，memory-actor 据实延后留独立 phase）
- 端到端 smoke 2 step 全 PASS（含受阻 / 延后态如实标注）
- **ADR**：ADR-048 `Proposed → Accepted`（据真实测试逐 D ratify）；ADR-038 经 add-only Amendment 记录（indexing-replay-e2e splice 维度兑现，不溯改正文，ADR-014 D5）；ADR-031 / ADR-021 引用（不改其正文）；`docs/roadmap.md §3.25/§4` add-only（Phase 43 行 + indexing-replay-daemon-e2e 新 backlog 条目）
- **adapter**：§Phase 索引 Phase 43 `Draft → Done` + `Tasks 0 → 2`；§ADR 索引 ADR-048；§BDD 追加 phase-43 feature 行；ADR-038 Amendment 记录
- **release**：`docs/releases/v0.36.0-{evidence,artifacts}.md` + `RELEASE_NOTES.md` v0.36 段 + README v0.36 段
- **smoke**：`scripts/console_smoke.sh` v33[52/52]（indexing replay splice smoke + 既有 step 不退化，denominators [37/37]..[51/51] 不溯改）+ `internal/cli/smoke_syntax_test.go` `TestTask433` markers 同步
- **follow-up**：live daemon restart-then-replay e2e `[SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]` + memory-actor-all-rpc（独立 phase）+ memory actor 认证身份 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]` + 其余 roadmap §4 backlog 项留 backlog
