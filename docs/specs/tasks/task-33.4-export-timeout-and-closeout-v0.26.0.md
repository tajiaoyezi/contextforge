# Task `33.4`: `export-timeout-and-closeout-v0.26.0 — export --timeout add-only flag（默认 60s byte-equiv，ADR-004）+ v0.26.0 closeout（smoke v23 step [42/42] + TestTask334 + release docs + ADR-038 据 D1-D5 ratify + ADR-031/027 add-only Amendment + roadmap §3.15+§4 + adapter）；§范围外诚实记录三处 grounding 校正（%v→%w non-bug / tracestore-fts already-fixed / datadir env→Options 🟡 honest-defer），不实现`

**Status**: Draft

**Priority**: P2
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 33 (governance-debt-cleanup-2)
**Dependencies**: task-33.1（l2-embedding-cache-bound — L2 SQLite cache rowid-FIFO bound）/ task-33.2（memstore-lru-and-harddelete-invariant — console-api memstore access-order LRU + memory hard-delete no-dangling-ref invariant）/ task-33.3（observability-indexing-replay-and-trace-isolation — indexing event 持久化 migration 0019 + replay mapper + TraceStore workspace isolation add-only proto + drain-timeout verify-only）全 Done / ADR-038（governance-debt-cleanup-2，本 task ratify）/ ADR-031（observability-hardening，本 task add-only Amendment：indexing event replay 新增 + drain-timeout verify-only 校正）/ ADR-027（embedding-provider-completion，本 task add-only Amendment：L2 cache bound）/ ADR-004（默认行为 / proto 既有字段 / 既有契约不变——export 默认 60s byte-equivalent）/ ADR-008（dep add-only，Phase 33 = 0 new dep）/ ADR-012（tag/release outward-facing 须用户显式授权；本轮已授权 v0.26.0）/ ADR-013（禁伪造红线——真实 tag/run/digest 不预填）/ ADR-014 D1-D5（第二十四次激活）

## 1. Background

Phase 33 三个实现 task 全 Done：33.1（L2 SQLite embedding cache bound——`core/src/embedding/cache.rs` L1 已是 `BoundedCache` FIFO（Phase 31），L2 `sqlite_put`（`:153-161`，`INSERT OR REPLACE INTO embedding_cache` `:155`）无上限 → 行只增；本 task 加 row-count cap + rowid-FIFO eviction；0 新 dep / 0 schema migration（用 implicit rowid）；HONEST CAVEAT：`with_sqlite` 无生产调用点（test-only，`cache.rs:331/337`），shipped daemon 走 memory-only L1，故为 opt-in-path defense-in-depth 非确认的 live leak）/ 33.2（console-api memstore FIFO → access-order LRU——`internal/consoleapi/memstore.go` `cacheChunkUnlocked`（`:76-91`）/ `cacheTraceUnlocked`（`:96-111`）为 FIFO，read paths 不 move-to-front；本 task 升级 read-hit + existing-key overwrite move-to-front，修订既有 `TestMemStore_CacheEviction_FIFO`（`memstore_test.go:209-243`）→ LRU；memory hard-delete cascade = HONEST-DEFER 非问题（全 schema audit 0010-0018：`memory_id` 仅 `memory_items` PK，无其他表持有，无 memory-vector/embedding 表 → 无孤儿，写 cascade 代码即 speculative；交付 INVARIANT 测试令未来 FK 失败 `[SPEC-DEFER:phase-future.memory-harddelete-cascade]`）；`handleMemoryPin` strict-400 = DROPPED 刻意契约 不改（ADR-022 D2 lenient 保持）/ 33.3（observability——indexing.* event 持久化 add-only migration 0019 + replay mapper（REAL gap：indexing events 仅 broadcast 到 in-memory EventBus，`replay_events_from_audit` 仅处理 `memory_*`）+ TraceStore strict multi-workspace isolation add-only proto field + SQL WHERE workspace_id filter（empty=aggregate-all 保形）+ drain-timeout VERIFY-ONLY（`CONTEXTFORGE_EVENTS_DRAIN_TIMEOUT` 已交付 Phase 26，cite 既有 `TestDrainTimeoutFromEnv`，不重实现））。

本 task 兼两职：(A) 一处 GENUINE nit——`export --timeout` add-only flag；(B) 收口 v0.26.0：smoke v23 + release docs + ADR-038 据真实结果 ratify + ADR-031/027 add-only Amendment + roadmap §3.15 推进记录 + §4 add-only backlog + phase §6 闭合 + adapter + feature。

**(A) export --timeout add-only flag（GENUINE nit）**：`internal/cli/export.go:29` 硬编码 `context.WithTimeout(context.Background(), 60*time.Second)`；`parseExportOpts`（`:58-91`）无 `--timeout` flag。因 task-31.3 export 做**两次顺序 daemon spawn**（`internal/exporter/source.go:91` loadRecords spawn#1 + `:104-109` chunk loader spawn#2），二者共用一个 60s 上限、每次 spawn 最多等 `daemonHealthDeadline=15s`（`cmd/contextforge/main.go:34`）→ 在慢盘 / 冷启动下可能偏紧。本 task 加 add-only `--timeout` flag（默认 60s，default 时与改前 byte-equivalent；ADR-004），parse 单测 🟢。

**(B) v0.26.0 closeout**：smoke v23 step `[42/42]` + `TestTask334`（mirror `TestTask324`，无回归 `[37/37]`..`[41/41]`）+ `docs/releases/v0.26.0-{evidence,artifacts}.md`（`<backfill>` 待回填）+ README v0.26 段 + RELEASE_NOTES v0.26.0 段 + ADR-038 Proposed→Accepted（per-D ratify）+ ADR-031/027 add-only Amendment + roadmap §3.15 + §4 + phase-33 §6 闭合 + adapter + feature。

## 2. Goal

(A) `internal/cli/export.go` 加 add-only `--timeout` flag（`time.Duration` 或秒整数；未设 → 默认 60s）：`parseExportOpts`（`:58-91`）注册该 flag 并填入 `exportOpts`，`runExport`（`:29`）的 `context.WithTimeout` 用解析值代替硬编码 `60*time.Second`——**未设 `--timeout` 时与改前 byte-equivalent**（60s，ADR-004 默认行为不变）；设值则用该值（覆盖 spawn#1 + spawn#2 两次顺序 spawn 总上限）。0 新 dep（`flag` + `time` 标准库，ADR-008）。

(B) 据 33.1/33.2/33.3 **真实 CI / 实测产物**收口 v0.26.0：ADR-038 `Proposed → Accepted`（逐 D 如实——D1 L2 cache rowid-FIFO bound 达成 + opt-in-path caveat / true-LRU honest-defer、D2 memstore access-order LRU 达成 + hard-delete no-dangling-ref invariant 达成 + cascade honest-defer 非问题 + `handleMemoryPin` lenient ADR-022 D2 保持、D3 observability indexing event 持久化+replay（add-only migration 0019）+ TraceStore workspace isolation（add-only proto field）+ drain-timeout verify-only 校正、D4 honest-defer + dropped-nits 诚实、D5 默认行为+proto+migration(add-only)+0-dep 不变）；ADR-031 add-only Amendment（indexing event replay 新增 + drain-timeout verify-only 校正，不溯改正文 ADR-014 D5）；ADR-027 add-only Amendment（L2 cache bound，不溯改正文）；roadmap §3.15（Phase 33 推进记录）+ §4 add-only（新 backlog）；phase-33 §6 AC 置 `[x]` + Status Done；smoke v23 step `[42/42]`；release docs（tag/run/digest 用 `<backfill>`）；adapter（Phase 33 Done + Tasks 4 + ADR-038 Accepted + feature 行）。**真实 v0.26.0 tag/release 须用户显式授权**（本轮用户已授权 v0.26.0；不自行越界 tag，ADR-012）。

pass bar：(A) export `--timeout` add-only flag parse 单测 🟢（默认 60s byte-equiv + 设值生效）；(B) smoke `bash -n` 过 + `go test -run TestTask334` 过 + 文档闭合人工核 + ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `internal/cli/export.go`——`parseExportOpts`（`:58-91`）注册 add-only `--timeout` flag（`fs.Duration("timeout", 60*time.Second, "export timeout (default 60s; covers both sequential daemon spawns)")`），写入 `exportOpts`（add-only `Timeout time.Duration` 字段）；`runExport`（`:29`）`context.WithTimeout(context.Background(), opts.Timeout)` 代替硬编码 `60*time.Second`。default 时 `opts.Timeout==60s` → byte-equivalent。
- `internal/cli/export_test.go`（或同源 test）——`TestParseExportOpts_Timeout`（断言未设 → 60s default byte-equiv；设 `--timeout=120s` → 120s；既有 `--format`/`--output`/`--data-dir`/`--collection`/`--include-stale` parse 不退化）。
- `scripts/console_smoke.sh`——banner v22→v23 + v23 changelog 块 + step `[42/42]`（doc/status 断言 governance-debt-cleanup-2 baseline：L2 cache bound + memstore LRU + indexing replay/trace isolation + export --timeout；default build init baseline 不变 + denominator 不溯改 ADR-014 D5）。当前 live 脚本 v22 `[41/41]`（Phase 32）；故 Phase 33 顺接 `[42/42]`。
- `internal/cli/smoke_syntax_test.go`——新增 `TestTask334_SmokeV23GovernanceDebtCleanup2Step`（mirror `TestTask324`，断言 `v23 (task-33.4)` header + `[42/42]` + 标记 + 无回归既有 `[37/37]`..`[41/41]`，denominator 不溯改 ADR-014 D5 + `bash -n` 语法）。
- 新增 `docs/releases/v0.26.0-{evidence,artifacts}.md`（tag SHA / run id / digest 用 `<backfill>` 待回填）+ `README.md` v0.26 段 + `RELEASE_NOTES.md` v0.26.0 段。
- `docs/decisions/adr-038-governance-debt-cleanup-2.md`——Status Proposed→Accepted（per-D 限定）+ `## Ratification（v0.26.0 / task-33.4）` 节（逐 D 真实依据；L2 cache opt-in-path caveat / true-LRU honest-defer / cascade honest-defer 非问题 / drain-timeout verify-only / indexing-replay-e2e 🟡 据已达维度 ratify + 如实记录）。
- add-only Amendment（不溯改正文，ADR-014 D5）：`docs/decisions/adr-031-*.md`——`## Amendment (Phase 33 / v0.26.0)`（indexing.* event 持久化 + replay mapper 新增能力——承 Phase 26 observability，`replay_events_from_audit` 由仅 `memory_*` 扩展到 indexing.*；drain-timeout VERIFY-ONLY 校正：`CONTEXTFORGE_EVENTS_DRAIN_TIMEOUT` 系 Phase 26 既交付，本 pass 仅 verify 不重实现，不溯改正文）+ `docs/decisions/adr-027-*.md`——`## Amendment (Phase 33 / v0.26.0)`（embedding L2 SQLite cache row-count cap + rowid-FIFO eviction——承 Phase 31 L1 BoundedCache，opt-in-path defense-in-depth caveat + created_at true-LRU honest-defer，不溯改正文）。
- `docs/roadmap.md`——§3 新增 §3.15 Phase 33 推进记录 + §4 add-only（新 backlog 条目：l2-cache-true-lru / memory-harddelete-cascade（only-if-future-FK）/ indexing-replay-e2e / tracestore-multi-workspace-strict-e2e / daemon-options-datadir，add-only 不删旧条目正文）。
- `docs/specs/phases/phase-33-governance-debt-cleanup-2.md`——Status Draft→Done + §6 AC `[x]`（honest per-dim：indexing-replay-e2e / tracestore isolation e2e console 🟡 如实标注）。
- `docs/s2v-adapter.md`——§Phase 33 In Progress→Done + Tasks 3→4；§Task +33.4；§ADR 038 Proposed→Accepted；§BDD +phase-33 行。
- `test/features/phase-33-governance-debt-cleanup-2.feature`（已创建）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER] / DROPPED honest record）

以下三处经 grounding 校正为 **DROPPED / honest-defer，不实现**（survey overstatement 校正即本 task 的 ADR-013 价值，须在 spec 与 ADR-038 D4 如实记录）：

- **`%v→%w` 校正 = NON-BUG（DROPPED，不改）**：survey 称 `internal/daemon/search.go` 有 `%v` 应改 `%w` 丢 grpc Status——经核**该文件无 `fmt`**，不存在该行。真正的 `%v` 在 `internal/cli/search.go:88` `fmt.Fprintf(stderr, "contextforge search: %v\n", err)`——这是**终端 `Fprintf` 输出**，`%w` 在 `Fprintf` 中**非法（vet-error，仅 `fmt.Errorf` 接受 `%w`）**；且 `err.Error()` 已携带完整 grpc Status 文本 → **grpc Status 未丢失**。故改 `%w` 是引入 vet 错误的伪修复，DROPPED。
- **tracestore-fts cross-version migration = ALREADY-FIXED（no-op，不改）**：survey 称 tracestore FTS 跨版本迁移缺失——经核 `core/src/data_plane/search_persist.rs:84-90`（`open` 内 `MIGRATION_FTS_SQL` idempotent + `:90` `backfill_fts_if_empty`）+ `:304` `backfill_fts_if_empty`（0015-only 旧库一次性 backfill）已实现并由 TEST-26.1.4/.4b 守护 → 无 gap，no-op。
- **datadir env-global → daemon.Options.DataDir = REAL 但 🟡 honest-defer**：`cmd/contextforge/main.go:254` `setDataDirEnv` 是**刻意的 Go→Rust 跨进程 datadir transport**（经 `CONTEXTFORGE_DATA_DIR` env，`:255-267`）；改为 `daemon.Options.DataDir` 字段须 thread 进 child `cmd.Env` 且改 spawn 契约 → 非确定性、改进程边界，`[SPEC-DEFER:phase-future.daemon-options-datadir]`，本 task 不实现。

其余范围外：
- 真实 v0.26.0 tag push + release run（cosign 真签 + GHCR 推送）[SPEC-OWNER:user-authorized-release]——outward-facing 不可逆已获本轮用户授权（ADR-012）；post-tag-push backfill 填实 tag SHA / run id / digest，本 task body 不预填真实凭据。
- L2 cache created_at-column true-LRU（须对既有用户库 ALTER）[SPEC-DEFER:phase-future.l2-cache-true-lru]——task-33.1 用 implicit rowid FIFO，true-LRU honest-defer。
- memory hard-delete cascade real code [SPEC-DEFER:phase-future.memory-harddelete-cascade]——task-33.2 audit 证无孤儿（非问题），仅交付 invariant 测试（only-if-future-FK 才需）。
- indexing replay end-to-end restart（须 running daemon / job runner）[SPEC-DEFER:phase-future.indexing-replay-e2e] / TraceStore multi-workspace strict e2e console [SPEC-DEFER:phase-future.tracestore-multi-workspace-strict]——task-33.3 交付 mapper/SQL/handler 🟢，e2e 🟡 honest-defer。

## 4. Actors

- 主 agent（ADR-012 自治；真实 release 本轮已获用户授权）
- `runExport` / `parseExportOpts`（`internal/cli/export.go:24-91`，本 task add-only `--timeout` flag）
- export 两次顺序 daemon spawn（`internal/exporter/source.go:91` spawn#1 + `:104-109` spawn#2，共用 export ctx 上限）
- closeout 文档集（smoke / release docs / ADR-038 ratify / ADR-031/027 add-only Amendment / roadmap §3.15+§4 / phase spec / adapter / feature）

## 5. Behavior Contract

### 5.1 Required Reading

- `internal/cli/export.go:24-91`（`runExport` `:29` 硬编码 `context.WithTimeout(..., 60*time.Second)` + `parseExportOpts` `:58-91` flag 注册——本 task add-only `--timeout` 锚点 + `exportOpts` struct `:15-21`）
- `internal/exporter/source.go:83-109`（`loadRecords` spawn#1 `:91` `backend(ctx, ...)` + chunk loader spawn#2 `:104-109`——两次顺序 spawn 共用 ctx 上限的论据）+ `cmd/contextforge/main.go:31-34`（`daemonHealthDeadline = 15 * time.Second` 每次 spawn 等待上限）
- `internal/cli/search.go:88`（`fmt.Fprintf(stderr, "contextforge search: %v\n", err)`——`%v→%w` non-bug 锚点：终端 `Fprintf` `%w` vet-error + `err.Error()` 已含 grpc Status）
- `core/src/data_plane/search_persist.rs:80-94`（`open` 内 `MIGRATION_FTS_SQL` `:89` + `backfill_fts_if_empty` `:90`）+ `:300-311`（`backfill_fts_if_empty` doc——tracestore-fts already-fixed 锚点，TEST-26.1.4/.4b 守护）
- `cmd/contextforge/main.go:254-268`（`setDataDirEnv`——datadir 跨进程 transport honest-defer 锚点 [SPEC-DEFER:phase-future.daemon-options-datadir]）
- `docs/specs/tasks/task-33.1-l2-embedding-cache-bound.md §10` + `task-33.2-memstore-lru-and-harddelete-invariant.md §10` + `task-33.3-observability-indexing-replay-and-trace-isolation.md §10`（真实测试结果 + 结论——ADR-038 ratify 依据）
- `docs/decisions/adr-038-governance-debt-cleanup-2.md`（§D1-D5 + Consequences Ratification 条款）
- `docs/decisions/adr-031-*.md §Amendment`（observability——本 task add-only Phase 33 Amendment 落点：indexing replay 新增 + drain-timeout verify-only）+ `docs/decisions/adr-027-*.md §Amendment`（embedding-provider——本 task add-only Phase 33 Amendment 落点：L2 cache bound）
- `internal/cli/smoke_syntax_test.go:344-375`（`TestTask324_SmokeV22VectorBackendConfigStep`——本 task `TestTask334` mirror 源）+ `scripts/console_smoke.sh`（v22 `[41/41]` 块 `:983-984`，banner `:2`）
- `docs/releases/v0.25.0-{evidence,artifacts}.md`（release docs 模板）

### 5.2 关键设计 — export --timeout add-only + 诚实 per-D ratify + backfill 待回填

- **export `--timeout` add-only（默认 60s byte-equiv，ADR-004）**：`parseExportOpts` 加 `fs.Duration("timeout", 60*time.Second, ...)`，`exportOpts` add-only `Timeout time.Duration`；`runExport` `context.WithTimeout(ctx, opts.Timeout)`。**未设 flag → `opts.Timeout==60s` → 与改前 `context.WithTimeout(..., 60*time.Second)` byte-equivalent**（default 行为不变）。该 timeout 覆盖 `loadRecords` spawn#1 + chunk loader spawn#2 两次顺序 spawn 的总上限（每次 spawn 内部仍受 `daemonHealthDeadline=15s` 约束，本 task 不改 spawn 内部契约）。pass bar 单测：未设 → 60s；设 `--timeout=120s` → 120s；既有 flag parse 不退化。0 新 dep（`flag.FlagSet.Duration` + `time`，ADR-008）。
- ADR-038 ratify **逐 D 项据真实结果**：D1（L2 cache rowid-FIFO bound 达成 + opt-in-path caveat：`with_sqlite` 无生产调用点 / true-LRU honest-defer）/ D2（memstore access-order LRU 双 cache move-to-front-on-hit 达成 + hard-delete no-dangling-ref invariant 达成；cascade honest-defer 非问题（schema audit 无孤儿）；`handleMemoryPin` lenient ADR-022 D2 保持，无代码改动）/ D3（observability——indexing.* event 持久化 add-only migration 0019 + replay mapper 🟢 mapper unit / 🟡 e2e restart honest-defer；TraceStore workspace isolation add-only proto field + SQL filter，empty=aggregate-all 保形 🟢 SQL/handler / 🟡 e2e console honest-defer；drain-timeout verify-only cite 既有 `TestDrainTimeoutFromEnv`）/ D4（honest-defer + dropped-nits 诚实：datadir env→Options 🟡 defer / `%v→%w` non-bug（grpc Status 未丢 + `%w` 在 `Fprintf` 非法）/ tracestore-fts already-fixed TEST-26.1.4）/ D5（默认行为 + proto（add-only field）+ migration（add-only 0019）+ 0-dep 不变）。各 D 真实测试 / 实测结果待 33.1-33.3 实施后跑出再回填，不为「全 Accepted」伪造 e2e restart 重放或 e2e console isolation 已验（ADR-013）。
- ADR-031 add-only Amendment 为 **add-only 注记**（不删/不改 ADR-031 D 正文 + 既有 Amendment 正文）：indexing.* event 持久化 + replay mapper 新增能力（`replay_events_from_audit` 由仅 `memory_*` 扩展，add-only migration 0019）+ drain-timeout VERIFY-ONLY 校正（`CONTEXTFORGE_EVENTS_DRAIN_TIMEOUT` Phase 26 既交付，本 pass verify 不重实现，cite `grpcclient_test.go` 既有 `TestDrainTimeoutFromEnv`）。ADR-027 add-only Amendment 为 add-only 注记：L2 SQLite cache row-count cap + rowid-FIFO（承 Phase 31 L1 BoundedCache）+ opt-in-path caveat。
- tag SHA / release run id / 镜像 digest 在 release docs 用 `<backfill: ...>` 待回填——真实 v0.26.0 tag/release 是 closeout 合入后的**用户授权步**（本轮已授权），post-tag-push backfill PR 填实（承 v0.8–v0.25 pattern）。
- smoke step `[42/42]` 为文档/状态步：验 default build init baseline 不变（ADR-004）+ 文档化四 task 状态（L2 cache bound + memstore LRU/hard-delete invariant + indexing replay/trace isolation + export --timeout）。

### 5.3 不变量

- export 默认行为不变（ADR-004）：未设 `--timeout` → `opts.Timeout==60s` → `context.WithTimeout(..., 60*time.Second)` byte-equivalent；既有 `--format`/`--output`/`--data-dir`/`--collection`/`--include-stale` flag 语义 + export 输出 / exit code 不变。
- closeout 0 行为变更 / 0 新依赖（除 export `--timeout` add-only flag 用标准库；Phase 33 = 0 new dep，ADR-008；smoke 既有 step + denominator 不溯改 ADR-014 D5）。
- ADR-014 D5：历史 Phase 1-32 spec 不溯改；ADR-031/027 add-only Amendment 不改 D 正文 + 既有 Amendment 正文；roadmap §4 新 backlog 为 add-only 条目不删旧条目正文。
- add-only proto（task-33.3 `workspace_id` field on `GetSearchTraceRequest`/`ListQueriesRequest`，empty=aggregate-all）+ add-only migration（0019_indexing_events）+ export `--timeout` add-only flag（default byte-equiv）不破既有契约（ADR-004）。
- honest 守线（ADR-013）：dropped-nits（`%v→%w` non-bug / tracestore-fts already-fixed / datadir honest-defer）如实记录于 §范围外 + ADR-038 D4，**不实现**；e2e restart 重放 / e2e console isolation 🟡 honest-defer，不伪造已验。
- 真实 tag/release 经用户授权后执行（本轮已授权，ADR-012）；release docs tag/run/digest backfill 待回填，不预填伪造凭据。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [ ] **AC1**（export --timeout add-only flag default 60s 🟢）: `internal/cli/export.go` `parseExportOpts`（`:58-91`）注册 add-only `--timeout` flag（default `60*time.Second`）写入 `exportOpts`，`runExport`（`:29`）`context.WithTimeout(ctx, opts.Timeout)` 代替硬编码 `60*time.Second`；未设 → 60s default byte-equivalent（ADR-004 默认行为不变）；设 `--timeout=120s` → 120s；既有 flag parse 不退化；0 新 dep（标准库 `flag`/`time`，ADR-008）— verified by **TEST-33.4.1**
- [ ] **AC2**（v0.26.0 closeout 🟢🟡）: smoke banner v22→v23 + step `[42/42]`（doc/status 断言 governance-debt-cleanup-2 baseline + default build baseline intact）+ `TestTask334_SmokeV23GovernanceDebtCleanup2Step`（含无回归既有 `[37/37]`..`[41/41]`，denominator 不溯改）；v0.26.0 release docs（`v0.26.0-{evidence,artifacts}.md` `<backfill>` + README v0.26 段 + RELEASE_NOTES v0.26.0 段）+ ADR-038 per-D ratify `Proposed→Accepted`（D1/D2/D5 Accepted；D3 indexing-replay-e2e + tracestore-isolation-e2e 🟡 + D4 dropped-nits/datadir honest-defer PARTIAL）+ ADR-031 add-only Amendment（indexing replay + drain-timeout verify-only）+ ADR-027 add-only Amendment（L2 cache bound）+ roadmap §3.15 + §4 add-only 新 backlog + phase-33 §6 AC `[x]` + Status Done + adapter（Phase 33 Done/Tasks 4/ADR-038 Accepted）+ feature — verified by **TEST-33.4.2**
- [ ] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-33.4.3**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-33.4.1 | export `--timeout` add-only flag：未设 → 60s default byte-equiv（ADR-004）；设 `--timeout=120s` → 120s；`runExport` `context.WithTimeout(ctx, opts.Timeout)`；既有 flag parse 不退化；0 新 dep | `internal/cli/export.go` + `internal/cli/export_test.go`（同源 test） | Planned |
| TEST-33.4.2 | smoke v23 step `[42/42]`（governance-debt-cleanup-2 baseline + L2-cache/memstore-LRU/indexing-replay/export-timeout 标记 + 无回归既有 denominator）+ `bash -n` 过 + `go test -run TestTask334` 过 + v0.26.0 release docs + ADR-038 per-D ratify Accepted（D3 e2e / D4 dropped-nits honest-defer 如实）+ ADR-031/027 add-only Amendment + roadmap §3.15+§4 + phase-33 §6 闭合 + adapter + feature | `scripts/console_smoke.sh` + `internal/cli/smoke_syntax_test.go` + release/ADR-038/ADR-031/ADR-027/roadmap/phase/adapter/feature | Planned |
| TEST-33.4.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（低）export `--timeout` 默认行为回归**：add-only flag 若 default 非 60s 或 `runExport` 误漏 wire `opts.Timeout`，破 byte-equivalence。
  - **缓解**：`fs.Duration("timeout", 60*time.Second, ...)` default 显式 60s + `runExport` 用 `opts.Timeout`；TEST-33.4.1 断言未设 → 60s（与改前一致）+ 既有 flag parse 不退化。stop-condition：default byte-equiv 单测不过则 AC1 不标 `[x]`。
- **R2（低）closeout 误报 dropped-nits 为已修 / 误报 e2e 维度为已验**：诚实风险。
  - **缓解**：§范围外 + ADR-038 D4 逐项如实——`%v→%w` non-bug（grpc Status 未丢 + `%w` 在 `Fprintf` vet-error）DROPPED / tracestore-fts already-fixed（TEST-26.1.4）no-op / datadir env→Options 🟡 honest-defer；indexing-replay-e2e + tracestore-isolation-e2e console 🟡 据已达 mapper/SQL/handler 维度 ratify，不伪造 e2e 已验（ADR-013）。stop-condition：任何「dropped-nit 已修」/「e2e 已验」表述须有真实凭据，否则标受阻维度 / backfill。
- **R3（低）smoke denominator 误溯改**：新 step 须 `[42/42]`，既有 `[37/37]`..`[41/41]` 不动。
  - **缓解**：`TestTask334` 无回归断言守护（mirror `TestTask324`）；ADR-014 D5。
- **R4（低）ADR-031/027 Amendment 误溯改 D 正文 / 既有 Amendment 正文**：须 add-only 追加 `## Amendment (Phase 33 / v0.26.0)` 不删既有正文（D5）。
  - **缓解**：仅追加 Phase 33 Amendment 段（ADR-031：indexing replay 新增 + drain-timeout verify-only / ADR-027：L2 cache bound），不改 ADR-031/027 D 正文 + 既有 Amendment 正文。

## 9. Verification Plan

```bash
# AC1 — export --timeout add-only flag（默认 60s byte-equiv + 设值生效 + 既有 flag 不退化）
go test ./internal/cli/ -run TestParseExportOpts

# AC2 — smoke 语法 + syntax test
bash -n scripts/console_smoke.sh
go test ./internal/cli/ -run TestTask334

# AC2 — 文档闭合人工核（ADR-038 Accepted + per-D / ADR-031/027 add-only Phase 33 Amendment /
#        roadmap §3.15 + §4 新 backlog / phase §6 [x] / adapter Done / feature 存在）
# AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master

# 既有不退化（closeout 文档+smoke 不影响 workspace；export flag add-only）
cargo test --workspace && go test ./...
```

> **outward-facing 红线**：真实 v0.26.0 tag push + release run（cosign 真签 + GHCR 推送）是 closeout 合入后的**用户授权步**（本轮已授权，ADR-012）；本 task body 不预填真实凭据，release docs 的 tag/run/digest 用 `<backfill>` 待 post-tag-push backfill 填实。
>
> **honest-defer 边界**：本 task 仅交付 export `--timeout` add-only flag（🟢 可单测）+ v0.26.0 closeout 文档/smoke；§范围外三处 grounding 校正（`%v→%w` non-bug / tracestore-fts already-fixed / datadir env→Options 🟡 [SPEC-DEFER:phase-future.daemon-options-datadir]）**不实现**，据 ADR-013 如实记录于 §范围外 + ADR-038 D4。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Draft

**§9 Verification 计划** (will record real evidence at impl)：
- `go test ./internal/cli/ -run TestParseExportOpts`（export `--timeout` add-only flag：未设 → 60s default byte-equiv / 设 `--timeout=120s` → 120s / 既有 flag 不退化 / 0 新 dep）——真实跑出后回填，不伪造（ADR-013）。
- `bash -n scripts/console_smoke.sh` + `go test ./internal/cli/ -run TestTask334`（smoke 语法 + syntax test，`[42/42]` + governance-debt-cleanup-2 标记 + TEST-33.1/33.2/33.3 标记 + 无回归 `[37/37]`..`[41/41]`）——真实跑出后回填。
- `cargo test --workspace` + `go test ./...`（既有不退化）——真实跑出后回填。
- `bash scripts/spec_drift_lint.sh --touched origin/master`（D2 lint，CI spec-lint 权威）——真实跑出后回填。
- ADR-038 ratify 逐 D 据 33.1-33.3 真实测试 / 实测结果——待实测回填；D1 L2 cache opt-in-path caveat（`with_sqlite` 无生产调用点）/ true-LRU honest-defer、D2 cascade honest-defer 非问题 + `handleMemoryPin` lenient 保持、D3 indexing-replay-e2e + tracestore-isolation-e2e console 🟡 honest-defer + drain-timeout verify-only、D4 dropped-nits（`%v→%w` non-bug / tracestore-fts already-fixed / datadir env→Options 🟡）如实记录，不强 ratify（ADR-013）。
- ADR-031 add-only `## Amendment (Phase 33 / v0.26.0)`（indexing.* event 持久化 + replay mapper 新增 / drain-timeout verify-only 校正）+ ADR-027 add-only `## Amendment (Phase 33 / v0.26.0)`（L2 SQLite cache row-count cap + rowid-FIFO）——据真实落地后回填，不溯改 D 正文 + 既有 Amendment 正文（ADR-014 D5）。
- roadmap §3.15 Phase 33 推进记录 + §4 add-only 新 backlog（l2-cache-true-lru / memory-harddelete-cascade / indexing-replay-e2e / tracestore-multi-workspace-strict-e2e / daemon-options-datadir）——add-only 落地后回填。
- 真实 v0.26.0 tag/release（cosign 真签 + GHCR 推送）经用户授权（本轮已授权）→ post-tag-push backfill 填实 evidence/artifacts 待回填（tag SHA / run id / digest，承 v0.8–v0.25 pattern，不预填伪造凭据 ADR-013）。

**计划改动文件**：
- `internal/cli/export.go`——`parseExportOpts`（`:58-91`）add-only `--timeout` flag（default `60*time.Second`）+ `exportOpts` add-only `Timeout time.Duration`；`runExport`（`:29`）`context.WithTimeout(ctx, opts.Timeout)` 代替硬编码。+ `internal/cli/export_test.go` `TestParseExportOpts_Timeout`（default 60s byte-equiv + 设值生效）。
- `scripts/console_smoke.sh`——banner v22→v23 + v23 changelog 块 + step `[42/42]`（governance-debt-cleanup-2 baseline：L2 cache bound + memstore LRU + indexing replay/trace isolation + export --timeout；default build init baseline 不变）。
- `internal/cli/smoke_syntax_test.go`——`TestTask334_SmokeV23GovernanceDebtCleanup2Step`（mirror `TestTask324`，断言 `[42/42]` + 标记 + 无回归既有 `[37/37]`..`[41/41]`，denominator 不溯改）。
- `docs/releases/v0.26.0-{evidence,artifacts}.md`（新，tag/run/digest `<backfill>` 待回填）+ `README.md` v0.26 段 + `RELEASE_NOTES.md` v0.26.0 段。
- `docs/decisions/adr-038-governance-debt-cleanup-2.md`——Status Proposed→Accepted（per-D 限定）+ `## Ratification（v0.26.0 / task-33.4）` 节。
- add-only Amendment：`docs/decisions/adr-031-*.md`——`## Amendment (Phase 33 / v0.26.0)`（indexing replay 新增 + drain-timeout verify-only）+ `docs/decisions/adr-027-*.md`——`## Amendment (Phase 33 / v0.26.0)`（L2 cache bound），均不溯改 D 正文 + 既有 Amendment 正文。
- `docs/roadmap.md`——§3 新增 §3.15 Phase 33 推进记录 + §4 add-only 新 backlog。
- `docs/specs/phases/phase-33-governance-debt-cleanup-2.md`——Status Draft→Done + §6 AC `[x]`（honest per-dim）。
- `docs/s2v-adapter.md`——Phase 33 Done + Tasks 4 + ADR-038 Accepted + BDD 行。
- `test/features/phase-33-governance-debt-cleanup-2.feature`（已创建）。
