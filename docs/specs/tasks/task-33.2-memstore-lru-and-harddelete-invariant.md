# Task `33.2`: `memstore-lru-and-harddelete-invariant — console-api memstore 缓存 FIFO→access-order LRU（命中/覆写均 move-to-front）+ memory hard-delete no-dangling-ref 不变量测试（cascade 经全表审计为 non-issue 据实延后）+ handleMemoryPin lenient 契约据实不改（ADR-022 D2 保留）`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 33 (governance-debt-cleanup-2)
**Dependencies**: 既有 `internal/consoleapi/memstore.go`（task-15.1 fallback chunk/trace 缓存 + task-31.2 `resolveCacheCapacity` 可配置 cap，Phase 15 / 31 已交付）/ `internal/consoleapi/memstore_test.go`（`TestMemStore_CacheEviction_FIFO` `:209-243`，task-31.2 FIFO 基线）/ `core/src/memory/store.rs`（task-27.2 `hard_delete` `:235-246`，Phase 27 已交付，ADR-032 D2）/ `internal/consoleapi/handlers.go`（task-17.1 `handleMemoryPin` `:519-549` lenient 契约，Phase 17 已交付，ADR-022 D2）/ ADR-021（memory-event-as-observation，memory 层契约）/ ADR-022 D2（memory pin lenient 契约——本 task 据实保留不改）/ ADR-004（local-first-privacy-baseline，默认行为 + 既有契约不变）/ ADR-016（Go thin proxy + Rust SoT 两进程布局，fallback-mode 不改拓扑）/ ADR-013（禁伪造红线——cascade non-issue 据实记录不伪造为「已实现 cascade」，无伪造测试结果）/ ADR-012（main-agent-governance-autonomy）/ ADR-038 D2（governance-debt-cleanup-2，本 task 即其原文实现）/ ADR-014 D1-D5（第二十四次激活）

## 1. Background

跨 Phase 累积的内存层 / fallback 缓存治理债，本 task 聚焦三项 code-local 加固，其中两项经 grounding 校正为「不变量守护」与「据实不改」而非「新功能」（ADR-013 的诚实价值）：

- **B1 console-api memstore 缓存为 FIFO 非 access-order LRU**：`internal/consoleapi/memstore.go` 的 `cacheChunkUnlocked`（`:76-91`）/ `cacheTraceUnlocked`（`:96-111`）是 FIFO——既有 key 覆写**不**重排序列（`:80-83` / `:100-103` 命中既有 key 即原地写回 return，不动 `*Order`）、驱逐弹出 `*Order[0]`（`:87-89` / `:107-109`）；读路径 `GetSourceChunk`（`:341-352`）/ `GetSearchTrace`（`:357-368`）命中缓存后**不** move-to-front。但 `types.go:69` / `handlers.go:277` 注释已称「in-memory LRU」——术语先行于实现，存在语义漂移。Console UI drill-down 重复访问热 chunk/trace 时，FIFO 仍可能在 cap 满时逐出仍在被频繁访问的热条目（access pattern 与驱逐策略不匹配）。
- **B2 memory hard-delete cascade — 经全表审计为 non-issue（无可级联对象）**：`core/src/memory/store.rs` `hard_delete`（`:235-246`）执行单条 `DELETE FROM memory_items WHERE memory_id = ?`（`:237-240`）。全 schema 审计（6 张表 / migration 0010-0018）：`memory_id` 仅作 `memory_items` 的 PRIMARY KEY（`core/migrations/0013_memory_items.sql:6`），其余表均无 `memory_id` 列（`memory_id` 全仓 `*.sql` 仅 1 处命中 = 0013），无 memory-vector / memory-embedding 表（grep=0），向量存储层与 memory 零耦合。故 hard-delete 后无任何孤儿行 → 写 cascade 代码属「为不可能场景写错误处理」（CLAUDE.md Simplicity-First，speculative/impossible-scenario）。本 task **不**写 cascade 实现，交付一条**不变量测试**（schema 内省断言「`memory_items` 是唯一含 `memory_id` 的表」+ `get(id)` 在 `hard_delete` 后为 `None`），使将来若有人新增 `memory_id` 外键表而不补 cascade，该不变量测试会失败、强制一次真实决策。
- **B3 handleMemoryPin strict-400 — 经契约审计为据实不改（lenient 是 deliberate contract）**：`internal/consoleapi/handlers.go` `handleMemoryPin`（`:525-549`）对 malformed / empty / absent body 故意回落 `pin=true`（`:536` 初值 + `:540-542` 仅当 decode 成功且 `body.Pin != nil` 才覆盖）以保持 v0.7 起的宽松契约（doc-comment `:519-524` 明记）；此为 task-17.1 / ADR-022 D2 的既定决策。改为 400 会违反 ADR-004（默认行为不变）且推翻已 Accepted 的 ADR-022 D2。本 task 记为**诚实 non-change**（无代码改动），仅在 ADR-038 D4 记其 lenient by design。

经核 B1 为 code-local 🟢 可单测（access-order 行为确定性可验），B2 不变量测试为 schema 内省 + hard-delete 后 `get`=None 🟢 可单测，B3 无代码改动（仅文档记录）。本 task 0 新 dep（B1 沿用既有 map + slice 序列结构，仅改重排逻辑；B2 用既有连接 introspection），fallback-mode only（生产接真实后端，本 fallback 路径仅 `CONSOLE_API_FALLBACK_INMEM=1` 时启用）。

## 2. Goal

(1) **B1**：把 `memstore.go` 两缓存（chunk / trace）由 FIFO 升级为 access-order LRU——命中读路径（`GetSourceChunk` `:341-352` / `GetSearchTrace` `:357-368`）move-to-front + 既有 key 覆写（`cacheChunkUnlocked` `:80-83` / `cacheTraceUnlocked` `:100-103`）move-to-front；cap 沿用 `resolveCacheCapacity`（task-31.2，env `CONTEXTFORGE_CONSOLEAPI_CACHE_CAP`，默认 256）；驱逐仍弹 `*Order[0]`（现为真正最久未访问者）。须改写既有硬断言 FIFO 的 `TestMemStore_CacheEviction_FIFO`（`:209-243`）为 LRU 语义。(2) **B2**：交付 memory hard-delete no-dangling-ref **不变量测试**——schema 内省断言 `memory_items` 是唯一含 `memory_id` 列的表 + `hard_delete(id)` 后 `get(id)` 为 `None`；cascade 据全表审计为 non-issue 据实延后 [SPEC-DEFER:phase-future.memory-harddelete-cascade]（only-if-future-FK）。(3) **B3**：handleMemoryPin lenient 契约据实保留不改（无代码改动，仅 ADR-038 D4 记录）。

pass bar：B1 经确定性单测验证 access-order LRU（🟢，命中/覆写 move-to-front + 驱逐最久未访问者）；B2 不变量测试 🟢（schema 内省 + hard-delete 后 None）；B3 既有 `handleMemoryPin` 行为 / 既有 pin 测试不退化（无改动）；默认行为 / proto / 既有契约不变（ADR-004）+ 0 新 dep（ADR-008 守线）；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `internal/consoleapi/memstore.go`——`cacheChunkUnlocked`（`:76-91`）/ `cacheTraceUnlocked`（`:96-111`）的既有-key 覆写分支（`:80-83` / `:100-103`）由「原地写回 return」改为 move-to-front（把命中 key 从 `*Order` 移到末尾视为最近使用）；读路径 `GetSourceChunk`（`:341-352`）/ `GetSearchTrace`（`:357-368`）命中缓存后 move-to-front（加私有 helper 如 `touchChunkUnlocked` / `touchTraceUnlocked`，在 `s.mu` 持锁下重排 `*Order`）。`cacheCapacity`（`:43`）+ `resolveCacheCapacity`（`:55-62`，task-31.2）不动；驱逐仍弹 `*Order[0]`（现语义为最久未访问者）。
- 改 `internal/consoleapi/memstore_test.go`——既有 `TestMemStore_CacheEviction_FIFO`（`:209-243`，硬断言 FIFO「首插入即首逐出」）改写为 LRU 语义（重命名为 `TestMemStore_CacheEviction_LRU` 或保名改断言）：在 cap 满后访问最早插入的 key 使其变最近使用，再触发驱逐 → 被逐者应为「最久未访问」而非「最早插入」。
- 新增 memstore LRU 单测：chunk 缓存命中 move-to-front（TEST-33.2.1）+ trace 缓存命中 move-to-front（TEST-33.2.2）——填满 cap → 访问某热 key → 再插入触发驱逐 → 热 key 仍在、最久未访问者被逐。
- 新增 memory hard-delete no-dangling-ref **不变量测试**（`core/src/memory/store.rs` 同源 test，TEST-33.2.3）——(i) schema 内省：枚举 `sqlite_master` / `PRAGMA table_info` 断言含 `memory_id` 列的表仅 `memory_items` 一张（将来若新增带 `memory_id` 的表而无 cascade，此断言失败）；(ii) `hard_delete(id)` 后 `get(id)` 返回 `None`（既有 `hard_delete` 行为守线，不重写 hard_delete 逻辑）。
- B3 无代码改动：`handleMemoryPin`（`:525-549`）lenient 契约据实保留；仅在 ADR-038 D4（@ task-33.4 ratify）记其 lenient by design + ADR-022 D2 引用。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- memory hard-delete real cascade 实现（删除关联向量 / 子表行）[SPEC-DEFER:phase-future.memory-harddelete-cascade]——经全表审计当前无可级联对象（`memory_id` 仅 PK on `memory_items`、无 memory-vector 表），写 cascade 属 impossible-scenario；only-if-future-FK：仅当将来新增带 `memory_id` 外键的表时才需实现，届时本 task 的不变量测试会先失败强制决策。
- handleMemoryPin strict-400 校验（malformed / empty body → 400）——据实**不实现**：lenient 回落 `pin=true` 是 ADR-022 D2 既定 deliberate contract（doc `:519-524`），改 400 违反 ADR-004 + 推翻 ADR-022 D2；本 task 记为诚实 non-change（ADR-038 D4）。
- console-api memstore 缓存跨进程 / 持久化共享 [SPEC-DEFER:phase-future.cross-process-sqlite-sharing]（既有 v0.3 trade-off，memstore.go:18-20 已标，非本 task）。
- 真实 release tag / run-id / digest（v0.26.0）[SPEC-OWNER:task-33.4-closeout]（ADR-012 用户授权后回填）。

## 4. Actors

- 主 agent（ADR-012 自治）
- `MemStore`（`internal/consoleapi/memstore.go`，fallback chunk/trace 缓存 access-order LRU；`cacheChunkUnlocked` `:76-91` / `cacheTraceUnlocked` `:96-111` / `GetSourceChunk` `:341-352` / `GetSearchTrace` `:357-368`）
- `MemoryStore`（`core/src/memory/store.rs`，`hard_delete` `:235-246`——本 task 不变量测试守护其 no-dangling-ref）
- `memory_items` 表（`core/migrations/0013_memory_items.sql`，`memory_id` PK `:6`——唯一含 `memory_id` 的表，schema 内省锚点）
- `handleMemoryPin`（`internal/consoleapi/handlers.go:525-549`，lenient 契约据实保留，无改动）
- Console UI fallback drill-down（`CONSOLE_API_FALLBACK_INMEM=1`，重复访问热 chunk/trace 受益于 LRU）

## 5. Behavior Contract

### 5.1 Required Reading

- `internal/consoleapi/memstore.go:76-91`（`cacheChunkUnlocked` FIFO——`:77-79` 空 id 守卫、`:80-83` 既有 key 原地写回 return（**本 task 改 move-to-front**）、`:84-85` 新 key append `*Order`、`:86-90` 超 cap 弹 `*Order[0]` 驱逐）+ `:96-111`（`cacheTraceUnlocked` 同形 FIFO）+ `:341-352`（`GetSourceChunk` 命中 `chunkCache` 即返回，**不** move-to-front——本 task 加 touch）+ `:357-368`（`GetSearchTrace` 同形）+ `:43`（`cacheCapacity` 字段）+ `:55-62`（`resolveCacheCapacity`，task-31.2 env 可配置，本 task 不动）
- `internal/consoleapi/memstore_test.go:209-243`（`TestMemStore_CacheEviction_FIFO` 硬断言 FIFO「首插入即首逐出」`:226-228` + 末位仍在 `:230-233` + size=cap `:235-238` + `*Order` 长度=cap `:239-242`——**本 task 改写为 LRU 语义**）
- `core/src/memory/store.rs:231-246`（`hard_delete` doc `:231-234` + 单条 `DELETE FROM memory_items WHERE memory_id = ?` `:237-240` + `NotFound` when n==0 `:241-245`——本 task 不变量测试守护，不改逻辑）
- `core/migrations/0013_memory_items.sql:6`（`memory_id TEXT PRIMARY KEY NOT NULL`——唯一含 `memory_id` 的表，schema 内省锚点）+ migration 0010-0018 全表（审计确认无其他表含 `memory_id`、无 memory-vector 表）
- `internal/consoleapi/handlers.go:519-549`（`handleMemoryPin` doc-comment lenient 契约 `:519-524` + `pin=true` 初值 `:536` + 仅 decode 成功且 `body.Pin != nil` 才覆盖 `:540-542`——本 task 据实**不改**）+ `internal/consoleapi/types.go:69` / `handlers.go:277`（注释已称「in-memory LRU」——术语先行，本 task 兑现实现）
- `docs/decisions/adr-038-governance-debt-cleanup-2.md §D2`（本 task 即其原文实现）+ `docs/decisions/adr-022-*.md D2`（memory pin lenient 契约——本 task 据实保留引用）+ `docs/decisions/adr-021-*.md`（memory-event-as-observation，memory 层契约守线）

### 5.2 关键设计 — access-order LRU + 不变量守护（默认行为不变）

- **B1 access-order LRU（命中 + 覆写均 move-to-front）**：保留既有 `map + []string Order` 结构（0 新 dep），仅改重排逻辑：(a) 既有 key 覆写（`cacheChunkUnlocked` `:80-83` / `cacheTraceUnlocked` `:100-103`）由「原地写回 return」改为「写回 + 把该 key 从 `*Order` 删除后 append 末尾」（最近使用）；(b) 读命中（`GetSourceChunk` `:343-346` / `GetSearchTrace` `:359-362`）加 move-to-front（持 `s.mu` 下把命中 key 移到 `*Order` 末尾）。驱逐仍弹 `*Order[0]`（现为最久未访问者）。pass bar 测试：填满 cap → 访问（读命中）最早插入的 key → 它变最近使用 → 再插一个新 key 触发驱逐 → 被逐者是「第二早访问」而非「最早插入但刚被访问」者；仍在缓存的热 key 可继续命中。move-to-front 为 O(n) 线性扫 + slice 删除（cap≤256 量级可接受；若性能敏感属 future 优化，本 task 求正确性）。
- **B2 no-dangling-ref 不变量（schema 内省 + hard-delete 后 None）**：不写 cascade 实现（全表审计无可级联对象——`memory_id` 仅 PK on `memory_items`、无 memory-vector 表）。交付不变量测试：(i) 经 `PRAGMA table_info(<table>)` 或查 `sqlite_master` 枚举所有表，断言含 `memory_id` 列的表恰为 `{memory_items}`——若将来新增带 `memory_id` 的表（潜在孤儿源）该断言失败，强制补 cascade 或显式豁免；(ii) 插入一条 memory → `hard_delete(id)` → `get(id)` 为 `None`（既有 hard_delete 物理删除行为守线）。cascade 据实延后 [SPEC-DEFER:phase-future.memory-harddelete-cascade]（only-if-future-FK），不伪造为「cascade 已实现」（ADR-013）。
- **B3 handleMemoryPin 据实不改**：lenient 回落 `pin=true`（`:536` / `:540-542`）是 ADR-022 D2 deliberate contract（doc `:519-524`）。本 task 不改任何代码；仅在 ADR-038 D4 记 lenient by design + ADR-022 D2 引用（兑现「不为通用化而推翻已 Accepted 决策」CLAUDE.md Surgical-Changes）。
- **fallback-mode only**：本 task 改动仅作用于 `CONSOLE_API_FALLBACK_INMEM=1` 的 fallback MemStore（生产接真实 gRPC 后端，不走此缓存路径）；不改 fallback 拓扑（ADR-016）。

### 5.3 不变量

- 默认行为不变（ADR-004）：LRU 升级不改缓存命中 / 未命中的返回值语义（命中仍返回缓存值、未命中仍 fallthrough 到 `SearchBackend` 或 `ErrDataPlaneUnavailable`，`:347-351` / `:363-367` 路径不变）；cap 默认仍 256（`resolveCacheCapacity` 不动）；仅驱逐**选择**由「最早插入」改为「最久未访问」。`handleMemoryPin` 行为完全不变（无改动）；`hard_delete` 逻辑不变（仅加守护测试）。
- 既有契约不变：`MemStore` 公共方法签名（`GetSourceChunk` / `GetSearchTrace` / `Search`）兼容（move-to-front 为内部重排，调用方不破）；`SearchClient` interface（types.go:64-73）不动；`MemoryStore::hard_delete` 公共签名兼容（不改）；两进程拓扑不变（ADR-016）。
- 0 新代码依赖（ADR-008 守线）：B1 沿用既有 `map[string]T` + `[]string` 序列结构（move-to-front 为标准库 slice 操作），无 go.mod 依赖增量；B2 用既有 rusqlite 连接 introspection，无 Cargo 依赖增量。
- honest 守线（ADR-013）：cascade non-issue 据实记录为不变量守护（不伪造为「cascade 已实现」、不写 impossible-scenario 代码）；handleMemoryPin lenient 据实保留（不伪造为「已加 strict 校验」）；无伪造测试结果（真实跑出后回填）。
- ADR-022 D2 守线：memory pin lenient 契约保留——本 task 不推翻已 Accepted 决策。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [x] **AC1**（memstore access-order LRU 两缓存 move-to-front 🟢）: `memstore.go` `cacheChunkUnlocked`（`:76-91`）/ `cacheTraceUnlocked`（`:96-111`）既有-key 覆写 move-to-front + 读路径 `GetSourceChunk`（`:341-352`）/ `GetSearchTrace`（`:357-368`）命中 move-to-front；驱逐弹最久未访问者；cap 沿用 `resolveCacheCapacity`（默认 256）；既有 `TestMemStore_CacheEviction_FIFO`（`:209-243`）改写为 LRU 语义 — verified by **TEST-33.2.1**（chunk 命中 move-to-front + 热 key 不被逐）+ **TEST-33.2.2**（trace 命中 move-to-front + 改写后 LRU 驱逐）。实证：`go test ./internal/consoleapi/ -run TestMemStore_CacheEviction -v` 2 PASS（`TestMemStore_CacheEviction_LRU` + `TestMemStore_CacheEviction_LRU_Trace`）；private helper `moveToMRU`（线性删除既有位置 + append 末尾，保 `*Order` 无重复）。
- [x] **AC2**（hard-delete no-dangling-ref 不变量 + cascade honest-defer + handleMemoryPin lenient 保留 🟢）: `core/src/memory/store.rs` 不变量测试断言「`memory_items` 是唯一含 `memory_id` 列的表」（schema 内省 `sqlite_master` + `PRAGMA table_info`）+ `hard_delete(id)` 后 `get(id)` 为 `None`；cascade 据全表审计为 non-issue 据实延后 [SPEC-DEFER:phase-future.memory-harddelete-cascade]（不写 impossible-scenario 代码）；`handleMemoryPin`（`:525-549`）lenient 契约据实保留无改动（ADR-022 D2）、既有 pin 测试不退化 — verified by **TEST-33.2.3**。实证：`cargo test -p contextforge-core memory::store` 16 PASS（含 `test_33_2_3_hard_delete_no_dangling_refs`，断言 `with_memory_id == ["memory_items"]`）；`go test ./internal/consoleapi/ -run TestMemoryPin -v` PASS（`TestMemoryPin_204_no_body` 守线）。
- [x] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-33.2.4**（= LAST，CI spec-lint 权威）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-33.2.1 | chunk 缓存 access-order LRU：填满 cap → 读命中（`GetSourceChunk`）最早插入 key 使其最近使用 → 再插触发驱逐 → 热 key 仍在、最久未访问者被逐；既有 key 覆写亦 move-to-front | `internal/consoleapi/memstore.go` + `memstore_test.go` | Done |
| TEST-33.2.2 | trace 缓存 access-order LRU（同形 move-to-front on hit/覆写）；既有 `TestMemStore_CacheEviction_FIFO`（`:209-243`）改写为 LRU 语义（被逐者=最久未访问而非最早插入） | `internal/consoleapi/memstore.go` + `memstore_test.go` | Done |
| TEST-33.2.3 | hard-delete no-dangling-ref 不变量：schema 内省断言含 `memory_id` 列的表仅 `memory_items` + `hard_delete(id)` 后 `get(id)`=None；cascade non-issue 据实延后 [SPEC-DEFER:phase-future.memory-harddelete-cascade]；handleMemoryPin lenient 保留无改动 + 既有 pin 测试不退化 | `core/src/memory/store.rs`（同源 test）+ `internal/consoleapi/handlers.go`（无改动，仅守线） | Done |
| TEST-33.2.4 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（中）move-to-front 正确性 + 锁内重排**：access-order LRU 须在持 `s.mu` 下正确把命中 key 从 `*Order` 移到末尾（删除 + append），漏删会致 `*Order` 含重复 key、驱逐逻辑错乱（逐出仍在 map 的 key）。
  - **缓解**：move-to-front helper 严格「先从 `*Order` 线性删除该 key 再 append」（保 `*Order` 与 `map` key 集一一对应，无重复）；TEST-33.2.1/.2 断言驱逐后 `len(*Order) == len(map) == cap` + 被逐者正确（最久未访问）+ 热 key 仍命中。stop-condition：LRU 语义单测不过则 AC1 不标 `[x]`。
- **R2（中）既有 FIFO 测试改写的语义正确性**：`TestMemStore_CacheEviction_FIFO`（`:209-243`）硬断言「首插入即首逐出」，LRU 下该断言不再成立——须改为「访问后变最近使用、不被逐」，改错会假绿。
  - **缓解**：改写后测试须显式「填满 cap → 读命中最早 key → 插新 key → 断言最早 key 仍在、第二早（未被访问者）被逐」；保留 size=cap / `*Order` 长度=cap 的 drift 断言（`:235-242`）。stop-condition：改写测试未真实区分 FIFO vs LRU（如仍只断言末位在）则不标 AC1。
- **R3（低）cascade 误判为「应实现」**：B2 易被误读为「hard-delete 漏了 cascade、应补删子表」，但全表审计无可级联对象——写 cascade 属 impossible-scenario。
  - **缓解**：交付**不变量测试**而非 cascade 实现（schema 内省断言唯一表 + hard-delete 后 None）；同行 [SPEC-DEFER:phase-future.memory-harddelete-cascade]（only-if-future-FK）；据 ADR-013 不伪造为「cascade 已实现」、据 CLAUDE.md Simplicity-First 不写不可能场景代码。stop-condition：若误写 cascade 实现则越界——本 task 仅守护不变量。
- **R4（低）handleMemoryPin 误判为「应改 400」**：B3 易被误读为「malformed body 应 400」，但 lenient 是 ADR-022 D2 deliberate contract。
  - **缓解**：本 task **不改** `handleMemoryPin`（无代码改动）；仅 ADR-038 D4 记 lenient by design + ADR-022 D2 引用；既有 pin 测试守线维持绿。stop-condition：若改 handleMemoryPin 为 400 则越界违反 ADR-004 + ADR-022 D2——据实非改。

## 9. Verification Plan

```bash
# 1. AC1 — memstore access-order LRU（命中/覆写 move-to-front + 驱逐最久未访问 + FIFO 测试改写为 LRU）
go test ./internal/consoleapi/ -run TestMemStore_CacheEviction

# 2. AC2 — hard-delete no-dangling-ref 不变量（schema 内省 + hard-delete 后 None）+ handleMemoryPin lenient 守线
cargo test -p contextforge-core memory::store
go test ./internal/consoleapi/ -run TestMemoryPin

# 3. 不退化（全量）
go test ./...
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# 4. AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界**：本 task 仅交付 memstore access-order LRU（🟢 可单测）+ hard-delete no-dangling-ref 不变量测试（🟢，schema 内省 + hard-delete 后 None）；memory hard-delete real cascade 据全表审计为 non-issue（无可级联对象）据实延后 [SPEC-DEFER:phase-future.memory-harddelete-cascade]（only-if-future-FK），不写 impossible-scenario 代码（CLAUDE.md Simplicity-First）；handleMemoryPin strict-400 据实**不实现**（lenient 是 ADR-022 D2 deliberate contract，改 400 违反 ADR-004，诚实 non-change @ ADR-038 D4）；据 ADR-013 不伪造完成 / 不预填真实数值。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done

**§9 Verification 实证**（real evidence，本地全绿）：
- AC1：`go test ./internal/consoleapi/ -run TestMemStore_CacheEviction -v` → 2 PASS（`TestMemStore_CacheEviction_LRU` + `TestMemStore_CacheEviction_LRU_Trace`）。chunk / trace 两缓存 access-order LRU 经确定性单测验证（cap=3：填满 a/b/c → 读命中 / 覆写 a → 再插 d → 被逐者为最久未访问者而非最早插入者；读命中 `GetSourceChunk` `:344-352` / `GetSearchTrace` `:360-369` move-to-front + 既有 key 覆写 `:83-86` / `:104-107` move-to-front 均经断言）。私有 helper `moveToMRU`（线性删除既有位置 + append 末尾）保 `*Order` 与 map key 集一一对应无重复（`assertNoCacheOrderDup` 守护 R1）。
- AC2：`cargo test -p contextforge-core memory::store` → 16 PASS（含 `test_33_2_3_hard_delete_no_dangling_refs`）；`go test ./internal/consoleapi/ -run TestMemoryPin -v` → PASS（`TestMemoryPin_204_no_body` 守线）。不变量测试经 `sqlite_master` + `PRAGMA table_info` 内省断言 `with_memory_id == ["memory_items"]`（`0013:6` PK 是唯一含 `memory_id` 列的表）+ `hard_delete("ddel")` 后 `get("ddel")`=None；cascade 据全表审计为 non-issue 据实延后 [SPEC-DEFER:phase-future.memory-harddelete-cascade]（未写 impossible-scenario 代码）；handleMemoryPin lenient 契约据实保留无改动（ADR-022 D2）+ 既有 pin 测试不退化。
- 不退化：`go test ./...` 全 PASS（exit 0）；`cargo test --workspace` 全 PASS（lib 202 + 全 integration）；`cargo clippy --workspace --all-targets -- -D warnings` 0 warning。
- AC3：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威）。
- 0 新 dep（B1 沿用 `map`+`[]string`，B2 用既有 rusqlite 连接 introspection）/ 默认行为不变（命中/未命中返回值语义不变，cap 默认仍 256，仅驱逐选择由最早插入→最久未访问）/ 既有契约不变（公共方法签名兼容）/ cascade non-issue 据实记录不伪造（不写 cascade 实现、无伪造测试结果）/ handleMemoryPin 诚实 non-change（无代码改动，ADR-022 D2 保留）。

**实际改动文件**：
- `internal/consoleapi/memstore.go`——新增私有 `moveToMRU(order, key)` helper；`cacheChunkUnlocked` / `cacheTraceUnlocked` 既有 key 覆写分支由「原地写回 return」改为「写回 + `moveToMRU`」；`GetSourceChunk` / `GetSearchTrace` 读命中加 `moveToMRU`（持 `s.mu` 下）；struct 缓存字段 doc 注释 FIFO→access-order LRU；`cacheCapacity` / `resolveCacheCapacity` 不动。
- `internal/consoleapi/memstore_test.go`——既有 `TestMemStore_CacheEviction_FIFO`（`:209-243`）改写为 `TestMemStore_CacheEviction_LRU`（chunk，TEST-33.2.1）+ 新增 `TestMemStore_CacheEviction_LRU_Trace`（trace 覆写+读命中，TEST-33.2.2）+ `assertNoCacheOrderDup` 守护 helper；移除随 FIFO 测试一并废弃的 `fmt` import。
- `core/src/memory/store.rs`——新增 hard-delete no-dangling-ref 不变量测试 `test_33_2_3_hard_delete_no_dangling_refs`（schema 内省含 `memory_id` 列的表仅 `memory_items` + `hard_delete` 后 `get`=None，TEST-33.2.3）；`hard_delete`（`:235-246`）逻辑不改。
- `internal/consoleapi/handlers.go`——`handleMemoryPin`（`:525-549`）**无改动**（lenient 契约据实保留，ADR-022 D2）；仅守线 + ADR-038 D4 记录。
- ADR-038 D2 据真实测试 ratify @ task-33.4 closeout（非本 task body）；cascade non-issue + handleMemoryPin lenient 据实记入 ADR-038 D4 @ task-33.4。
