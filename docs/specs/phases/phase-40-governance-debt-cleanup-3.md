# Phase 40 · governance-debt-cleanup-3

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 是继 Phase 31（governance-debt-cleanup, Done）+ Phase 33（governance-debt-cleanup-2, Done）后的**第三轮治理债清扫**（镜像 ADR-036 / ADR-038 的「核实-诚实化-补全」打法），清理两组真实的跨 Phase 治理债 marker：**memory pin actor 透传**（`core/src/data_plane/memory.rs:225-229` `pin()` 把调用 actor **硬编码** `"console-api"`——因 `PinMemoryRequest`（proto:336-339）无 actor field、`MemoryStore.Pin(id,pin)`（Go interface）无 actor 参数；marker `[SPEC-DEFER:phase-future.memory-actor-propagation]` @ memory.rs:227。补 add-only proto field + Go 调用链透传 + REST `X-Actor` header 读取，使 console 部署在 auth 代理后可把 pin 操作归因到真实调用方）、**L2 embedding 缓存访问序 LRU**（`core/src/embedding/cache.rs` Phase 33 D1 给 L2 SQLite 加了 row-count cap + **rowid-FIFO**（插入序）驱逐，但 `sqlite_get`（:140-150）命中**不**重排——属插入序 FIFO 而非访问序 LRU；marker `[SPEC-DEFER:phase-future.l2-cache-true-lru]`。Phase 33 当时判定真 LRU「须加 created_at 列 + ALTER 既有文件」而延后——本轮 grounding **据实更正该假设**：命中即对该行重写（bump 隐式 rowid 到表尾）即得访问序 LRU，**复用既有隐式 rowid、0 schema migration**，与 Go memstore 命中 move-to-front（task-33.2）同技法）。两项均为 code-local 🟢 可单测。**关键诚实校正（ADR-013，本 phase 的核心价值）**：(1) memory pin actor 本轮交付**调用方透传**（REST header → proto → store），但**认证身份**（把 actor 校验为已认证 auth subject）须 console-api 鉴权层、本轮不做 → honest-defer `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`；(2) L2 true-LRU 的命中 bump 给 L2 读路径加了写放大（命中即重写行），属访问序 LRU 的固有代价，据实记，且因 L2 `with_sqlite` 无生产调用点（Phase 33 D1 已据实标注 opt-in-path），现网零影响——本项是 opt-in 路径的语义补全、非已确认线上问题，不夸大（ADR-013）；(3) 其余治理 marker 据实**保持延后**（`vector-dim-feature-enforce` 须 feature build / `tracestore-multi-workspace-strict` 余下读路径深化 / `chunk-source-type-filter` 须 import-path schema migration），不在本 phase 强行扩面。默认行为 / proto / 既有契约不变（ADR-004，proto field 与 Go 参数均 add-only、空 actor 回落 `"console-api"` byte-equiv、L2 bump 仅在有限 cap 下生效且结果等价）；Phase 40 = **0 新依赖**（ADR-008）；既有 `cargo-test` / `go-test` / `spec-lint` 三门不退化。

> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md §3.22 + §4 backlog` → 各债项源码锚点（`core/src/data_plane/memory.rs:215-240`（`pin()` RPC，:225-229 `set_pinned_with_actor(.., "console-api")` 硬编码 actor + :227 `[SPEC-DEFER:phase-future.memory-actor-propagation]` marker）/ `proto/contextforge/console_data_plane/v1/console_data_plane.proto:336-339`（`PinMemoryRequest` 仅 memory_id=1 / pin=2，缺 actor）/ `internal/consoleapi/handlers.go:525-549`（`handleMemoryPin` REST 入口）+ `:815`（`r.Header.Get("Last-Event-ID")` header 读取范式）/ `internal/consoleapi/router.go:71`（`r.Header.Get("X-Confirm")` header 读取范式）/ `internal/consoleapi/grpcclient/grpcclient.go:724-726`（`memoryClient.Pin(id,pin)` → `pb.PinMemoryRequest{MemoryId,Pin}`）/ `internal/consoleapi/memstore.go:653`（`MemMemoryStore.Pin(id,pin)` fallback 实现）/ `core/src/embedding/cache.rs:140-150`（`sqlite_get` L2 命中读，不重排——本 phase 加 bump）+ `:153-195`（`sqlite_put` Phase 33 rowid-FIFO 驱逐，:185-187 `DELETE ... ORDER BY rowid DESC LIMIT`）+ `:23`（`DEFAULT_EMBEDDING_CACHE_CAP`）+ `:117`（`DEFAULT_L2_EMBEDDING_CACHE_CAP`）+ `:402`（TEST-33.1.1 镜像源））→ AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5，**第三十一次**激活）→ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造红线：pin actor 真实透传非合成 / L2 命中 bump 真实重排经单测；认证身份据实延后 / L2 写放大据实记 / 其余 marker 据实保持延后，不伪造、不夸大、不强行扩面）。

> **ADR 影响面（已识别）**：
> - **ADR-045 governance-debt-cleanup-3（新，Proposed）**：记 memory pin actor add-only 透传（proto field + Go 参数链 + REST header；认证身份据实延后，D1）+ L2 embedding 缓存访问序 LRU（命中 bump 隐式 rowid，0 migration，更正 Phase 33 真-LRU 假设；写放大据实记，D2）+ honest-defer 边界 + 默认行为 / proto（add-only）/ 既有契约不变 + 0-dep / 0-network（D3）。Status: Proposed（Draft 阶段不 ratify；ratify 在 task-40.3 closeout）。
> - 触及 **ADR-032（memory-ops-hardening — pin actor/timestamp first-class）**：`pin()` 当前硬编码 actor `"console-api"`（task-27.1 / ADR-032 D1 明记「real per-user actor propagation 延后」）——本 phase 以 add-only Amendment 记其 actor 透传维度兑现（proto add-only field + Go 参数链 add-only + 空 actor 回落 `"console-api"` byte-equiv；认证身份续延后），不溯改 ADR-032 正文（ADR-014 D5）。
> - 触及 **ADR-038（governance-debt-cleanup-2）+ ADR-027（embedding-provider）**：Phase 33 D1 / ADR-027 给 L2 加 rowid-FIFO（插入序），并把真-LRU 据「须加时间列」判定延后——本 phase 以 add-only Amendment 记其真-LRU 维度经命中 bump 隐式 rowid（0 migration）兑现 + 据实更正该判定，不溯改其正文（ADR-014 D5）。
> - 触及 **ADR-015（console-data-plane proto 契约）**：`PinMemoryRequest` add-only `actor=3`（既有 memory_id=1 / pin=2 字段号冻结 D1 + add-only D2）——以 add-only Amendment 记录。
> - 触及 **ADR-022（memory-pin）**：`handleMemoryPin` malformed/empty/absent body 回落 `pin=true` 的宽松契约（ADR-022 D2）本 phase **保持不变**——仅在其入口加 add-only `X-Actor` header 读取（缺省空 → 回落 `"console-api"`），不改其宽松 body 解析行为。
> - 触及 **ADR-004（默认行为 + 既有契约不变）**：pin actor proto field add-only、Go `Pin` 参数 add-only（空 actor 回落 `"console-api"` byte-equiv）、L2 命中 bump 仅在有限 cap 下生效且驱逐结果等价（默认 cap 充足、单文本 round-trip 不变）——默认行为 / proto / 既有契约均不变（守线，非推翻）。

## 1. 阶段目标

v0.32.0 ship 后，ContextForge 进行第三轮治理债清扫，把两组真实的跨 Phase 治理 marker 据 grounding 真实状态补全或诚实化：**memory pin actor 透传**（`pin()` 当前硬编码 `"console-api"`，补 add-only proto field + Go 调用链 actor 参数 + REST `X-Actor` header 读取，使 console 部署在 auth 代理后可把 pin 操作归因到真实调用方；认证身份据实延后）、**L2 embedding 缓存访问序 LRU**（Phase 33 给 L2 加了 rowid-FIFO 插入序驱逐但命中不重排，本轮把命中即 bump 隐式 rowid 接入，使驱逐由插入序 FIFO 升为访问序 LRU，复用隐式 rowid、0 schema migration——据实更正 Phase 33「真 LRU 须加时间列」的假设）。两项均 code-local 🟢 可单测。**关键诚实校正**：pin actor 本轮交付调用方透传、认证身份据实延后 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`；L2 命中 bump 给读路径加写放大（访问序 LRU 固有代价）据实记、且 L2 无生产调用点（Phase 33 已标 opt-in-path）现网零影响、本项是 opt-in 路径语义补全非已确认线上问题；其余治理 marker（`vector-dim-feature-enforce` / `tracestore-multi-workspace-strict` 余下读路径 / `chunk-source-type-filter`）据实保持延后不强行扩面。默认行为 / proto（add-only）/ 既有契约不变（ADR-004）；Phase 40 = 0 新依赖（ADR-008）；既有三门不退化。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. **memory pin actor add-only 透传**：`PinMemoryRequest`（proto:336-339）add-only `string actor = 3`（既有 memory_id=1 / pin=2 字段号冻结，ADR-015）+ buf generate 重生 Go/Rust binding；`MemoryStore.Pin` Go interface 加 add-only `actor string` 参数（两实现 `memoryClient.Pin` / `MemMemoryStore.Pin` 同步）+ `grpcclient` 把 actor 填入 `pb.PinMemoryRequest{..,Actor}`；`handleMemoryPin`（handlers.go:525-549）读 `r.Header.Get("X-Actor")` 透传（缺省空串）；Rust `pin()`（memory.rs:225-229）用 `req.actor` 非空时透传、空时回落 `"console-api"`（默认 byte-equiv，ADR-004）；认证身份 honest-defer `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`（AC1）
2. **L2 embedding 缓存访问序 LRU**：`core/src/embedding/cache.rs` `sqlite_get`（:140-150）L2 命中时（仅在有限 L2 cap 下）对命中行重写 bump 其隐式 rowid 到表尾，使 `sqlite_put`（:153-195）既有 rowid 序驱逐由插入序 FIFO 升为访问序 LRU（命中后被驱逐的是最久未**用**而非最早**插入**的行）；复用既有隐式 rowid、**0 新 dep / 0 schema migration**；据实更正 Phase 33「真 LRU 须加 created_at 列 + ALTER」的假设（命中 bump 即得，与 Go memstore move-to-front 同技法）；命中 bump 写放大据实记、L2 无生产调用点现网零影响（AC2）
3. **honest-defer 边界 + v0.33.0 closeout + 默认零依赖守线**：pin actor 认证身份 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]` / `vector-dim-feature-enforce`（须 feature build）/ `tracestore-multi-workspace-strict`（余下读路径深化）/ `chunk-source-type-filter`（须 import-path schema migration）据实保持延后；默认行为 / proto（add-only field）/ 既有契约不变 + 0 网络（ADR-004）+ 0 新依赖（ADR-008）；v0.33.0 release docs + `scripts/console_smoke.sh` v30[49/49] + ADR-045 据真实测试 ratify + ADR-032/038/027/015 add-only Amendment + roadmap §3.22/§4 add-only + phase §6 闭合（AC3）
4. ADR-014 D1-D5（**第三十一次**激活）全通过（AC4）

**v0.x 版本号决策**：v0.33.0（Phase 40，承 v0.32.0；roadmap §1.1 Phase N→v0.(N-7).0），theme governance-debt-cleanup-3。minor release（第三轮治理债清扫，两组 code-local 真实 marker 补全 + 诚实化；pin actor proto field 与 Go 参数 add-only、空 actor 回落 byte-equiv、L2 命中 bump 复用隐式 rowid 0 schema migration、默认行为不变；默认行为 / proto / 既有契约 / 默认构建 0 新依赖（ADR-008，Phase 40 不增 dep）+ 0 网络不变）。

## 2. 业务价值

第三轮治理债清扫——补齐「memory pin actor 透传、L2 embedding 缓存访问序 LRU」两组真实 marker，且对 Phase 33 一处延后假设据 grounding 诚实更正（这是本 phase 的 ADR-013 核心价值）：

### 40.1 memory pin actor 透传（memory-actor-propagation，🟢）

- `core/src/data_plane/memory.rs` `pin()`（:215-240）今把调用 actor **硬编码** `"console-api"`（:229 `set_pinned_with_actor(&req.memory_id, req.pin, "console-api")`，doc-comment :225-227 明记「console-api source is currently 'console-api'（real per-user actor propagation is `[SPEC-DEFER:phase-future.memory-actor-propagation]`）」）。根因：`PinMemoryRequest`（proto:336-339）只有 `memory_id=1` / `pin=2`，无 actor field；Go `MemoryStore.Pin(id,pin)` interface 无 actor 参数；`handleMemoryPin`（handlers.go:525-549）不读任何调用方标识。`set_pinned_with_actor`（store 层）本就接受 actor 参数（task-27.1 / ADR-032 D1 已把 pin actor 做成 first-class store 字段 + proto `MemoryItem.pinned_by=11`），仅**入口到 store 的透传链缺失**。
- 本 phase 补透传链：`PinMemoryRequest` add-only `string actor = 3`（既有字段号冻结，ADR-015）→ buf generate → Go `Pin(id,pin,actor)` 参数 add-only（两实现同步）+ `grpcclient` 填 `pb.PinMemoryRequest{..,Actor:actor}` → `handleMemoryPin` 读 `X-Actor` header 透传 → Rust `pin()` 用 `req.actor`（非空透传、空回落 `"console-api"`）。console 部署在设置 `X-Actor` / `X-Forwarded-User` 的 auth 代理后即可把 pin/unpin 归因到真实调用方（写入既有 `pinned_by` 字段 + audit）。
- **HONEST CAVEAT（不夸大，ADR-013）**：本轮交付**调用方透传**——actor 取自请求 header，未做认证校验（把 header 值映射为已验证 auth subject 须 console-api 鉴权层）。认证身份 honest-defer `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`。默认（无 `X-Actor` header）→ 空 actor → Rust 回落 `"console-api"` → 与今 byte-equiv（ADR-004）。

### 40.2 L2 embedding 缓存访问序 LRU（l2-embedding-cache-true-lru，🟢）

- `core/src/embedding/cache.rs` Phase 33 D1（ADR-038 D1）给 L2 SQLite `embedding_cache` 加了 row-count cap + **rowid-FIFO** 驱逐（`sqlite_put` :153-195，超 cap 时 `DELETE ... WHERE rowid NOT IN (SELECT rowid ... ORDER BY rowid DESC LIMIT cap)`，隐式 rowid = 插入序）。但 `sqlite_get`（:140-150）L2 命中**不**对命中行做任何重排——故驱逐是**插入序 FIFO** 而非**访问序 LRU**：一个频繁命中的旧行仍会因插入早而被驱逐。Phase 33 当时把真-LRU 据「带 created_at 列的真 LRU 须 ALTER 既有用户文件」判定延后（ADR-038 A2 / D4，marker `[SPEC-DEFER:phase-future.l2-cache-true-lru]`）。
- **本轮 grounding 据实更正该假设**：访问序 LRU **不**须新增时间列——命中时对该行重写（`INSERT OR REPLACE` 同 `(content_hash,provider,dim,vector)`，使其隐式 rowid 跳到表尾 = 最新）即把隐式 rowid 由「插入序」变为「访问序」，既有 `sqlite_put` 的 rowid 序驱逐随之由 FIFO 升为 LRU。**复用既有隐式 rowid、0 新 dep、0 schema migration**，与 Go memstore 命中 move-to-front（task-33.2，ADR-038 D2）同技法。
- 本 phase 在 `sqlite_get`（:140-150）命中分支加：仅当 L2 cap 有限（`l2_cap > 0`）时，对命中行 `INSERT OR REPLACE` 重写以 bump rowid（cap==0 不限时不 bump，保插入序、零额外写）。pass bar：cap=2，put a,b → 命中 a（bump）→ put c → 驱逐的是 b（最久未**用**）而非 a（最早**插入**）；FIFO 旧行为会驱逐 a，LRU 新行为驱逐 b。
- **HONEST CAVEAT（不夸大，ADR-013）**：命中 bump 给 L2 读路径加了一次行重写（写放大），属访问序 LRU 的固有代价（同 Go memstore 命中 move-to-front 的内部代价）；且 L2 `with_sqlite` 无生产调用点（Phase 33 D1 已据实标注 opt-in-path，出厂 daemon 走 memory-only L1）→ 现网零影响，本项是 **opt-in 路径的语义补全、非已确认线上问题**（据实声明，不夸大）。

**不在本 phase 范围**：

- memory pin actor 认证身份（把 header 值校验映射为已认证 auth subject，须 console-api 鉴权层）[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]
- L2 缓存 TTL / 时间过期（须时间列）[SPEC-DEFER:phase-future.l2-cache-ttl]
- `with_sqlite` 接入生产 daemon 调用点（当前出厂走 memory-only L1）[SPEC-DEFER:phase-future.l2-cache-production-wire]
- vector backend dim 强校验（须 feature build：qdrant/lancedb/sqlite-vec 声明 collection dim）[SPEC-DEFER:phase-future.vector-dim-feature-enforce]
- TraceStore 多 workspace 严格隔离余下读路径深化（load_warm / 内存路径之外的 RPC）[SPEC-DEFER:phase-future.tracestore-multi-workspace-strict]
- chunk source_type / agent_scope 真实过滤（须 import-path schema migration + importer 侧标注）[SPEC-DEFER:phase-future.chunk-source-type-filter] / [SPEC-DEFER:phase-future.chunk-agent-scope-filter]

## 3. 涉及模块

### 40.1 memory-actor-propagation（task-40.1）

- 修改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`——`PinMemoryRequest`（:336-339）add-only `string actor = 3`（既有 memory_id=1 / pin=2 字段号冻结，ADR-015 D1）；buf generate 重生 Go/Rust binding
- 修改 `internal/consoleapi/grpcclient/grpcclient.go`——`MemoryStore.Pin` interface（含 `memoryClient.Pin` :724-726）加 add-only `actor string` 参数 + `pb.PinMemoryRequest{MemoryId:id, Pin:pin, Actor:actor}`
- 修改 `internal/consoleapi/memstore.go`——`MemMemoryStore.Pin`（:653）同步加 `actor string` 参数（fallback 实现，记录 actor 或据实忽略，签名对齐 interface）
- 修改 `internal/consoleapi/handlers.go`——`handleMemoryPin`（:525-549）读 `r.Header.Get("X-Actor")`（缺省空串，镜像 :815 / router.go:71 header 读取范式）+ 传入 `Pin(id,pin,actor)`；宽松 body 契约（ADR-022 D2）不改
- 修改 `core/src/data_plane/memory.rs`——`pin()`（:225-229）`set_pinned_with_actor` 第三参由硬编码 `"console-api"` 改为 `if req.actor.is_empty() { "console-api" } else { &req.actor }`（空回落 byte-equiv，ADR-004）；更新 :227 marker 措辞为「调用方透传已落地；认证身份 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`」
- 同源验证（≥2，🟢：proto `PinMemoryRequest{actor}` wire-tag 字段号 3（in-crate prost 断言）+ Rust `pin()` 空 actor 回落 `"console-api"` / 非空透传（TEST-40.1.1 / TEST-40.1.2）/ Go `handleMemoryPin` 读 `X-Actor` header 透传到 `Pin(actor)` + 缺省空串（TEST-40.1.3）/ grpcclient 填 `pb.PinMemoryRequest.Actor`（TEST-40.1.4））

### 40.2 l2-embedding-cache-true-lru（task-40.2）

- 修改 `core/src/embedding/cache.rs`——`sqlite_get`（:140-150）命中分支：仅当 `l2_cap > 0` 时对命中行 `INSERT OR REPLACE INTO embedding_cache`（同 `(content_hash,provider,dim,vector)`）bump 其隐式 rowid 到表尾；`cap==0`（不限）不 bump（保插入序、零额外写）。既有 `sqlite_put`（:153-195）rowid 序驱逐不改（隐式 rowid 由插入序变访问序后自动升 LRU）
- `CREATE TABLE`（:110-120）不改——复用既有隐式 rowid，不加列、不 ALTER、不新增编号 migration（0 schema migration）
- 同源测试：`cache.rs` 同源 test（镜像 TEST-33.1.1 形态 + SQLite 文件 round-trip）断言 cap=2 put a,b → 命中 a（bump）→ put c → 驱逐 b（最久未用）而非 a（FIFO 会驱逐 a）；cap==0 不 bump、行为同 Phase 33 FIFO 基线
- 同源验证（≥2，🟢：L2 命中 bump → 访问序 LRU 驱逐最久未用（TEST-40.2.1，对比 FIFO 旧行为）+ cap==0 不 bump 保插入序 + 默认行为不变 + 既有 33.1.* / 22.2.* 维持绿（TEST-40.2.2））

### 40.3 closeout（task-40.3）

- 修改 `scripts/console_smoke.sh`——banner v29→v30 + v30 changelog block + 新 step [49/49]（memory pin actor 透传 + L2 访问序 LRU 可达则断言、否则 doc/status；current Phase 39 [48/48] → Phase 40 顺位 [49/49]）
- 修改 `internal/cli/smoke_syntax_test.go`——新 `TestTask403`（镜像 `TestTask393`）断言 [49/49] + no-regression（denominators [37/37]..[48/48] 不溯改，ADR-014 D5）
- 新增 `docs/releases/v0.33.0-evidence.md` + `v0.33.0-artifacts.md`（tag SHA / run id / digest 为 angle-bracket backfill marker）+ `README.md` v0.33 段 + `RELEASE_NOTES.md` v0.33.0 段
- 修改 `docs/decisions/adr-045-governance-debt-cleanup-3.md`——Status Proposed→Accepted（逐 D 如实）+ 新 `## Ratification（v0.33.0 / task-40.3）`
- add-only Amendment（非正文改，ADR-014 D5）：`adr-032`（memory-ops，pin actor 透传维度兑现 add-only）/ `adr-038`+`adr-027`（L2 true-LRU 维度兑现 + 真-LRU 假设据实更正 add-only）/ `adr-015`（proto add-only field）；`docs/roadmap.md §3.22/§4` add-only（Phase 40 行 + memory-actor-authenticated-identity 新 backlog 条目）
- 修改 `docs/specs/phases/phase-40-governance-debt-cleanup-3.md`——Status Draft→Done + §6 AC 勾选（逐维如实）
- 修改 `docs/s2v-adapter.md`——Phase 40 行 + Task 行 + ADR-045 行 + BDD 行

### BDD feature

- 新增 `test/features/phase-40-governance-debt-cleanup-3.feature`（≥3 scenario：memory pin actor add-only 透传（REST X-Actor header → proto actor=3 → store；空回落 console-api byte-equiv；认证身份据实延后）/ L2 访问序 LRU（命中 bump 隐式 rowid → 驱逐最久未用；cap==0 不 bump 保插入序；0 schema migration 更正 Phase 33 假设）/ v0.33.0 收口 + 默认零依赖守线）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 40.1 | `console_data_plane.proto` `PinMemoryRequest` add-only `actor=3` + `MemoryStore.Pin(id,pin,actor)` Go 参数链 + `handleMemoryPin` 读 `X-Actor` header + Rust `pin()` 用 `req.actor` 空回落 `"console-api"`（认证身份据实延后 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`；ADR-022 D2 宽松 body 契约不改） | `../tasks/task-40.1-memory-actor-propagation.md` |
| 40.2 | `core/src/embedding/cache.rs` `sqlite_get` 命中 bump 隐式 rowid（仅有限 cap）→ L2 驱逐由 rowid-FIFO 升访问序 LRU（0 新 dep / 0 schema migration，复用隐式 rowid，更正 Phase 33 真-LRU 假设；写放大据实记） | `../tasks/task-40.2-l2-embedding-cache-true-lru.md` |
| 40.3 | smoke v30[49/49] + v0.33.0 closeout + ADR-045 ratify + ADR-032/038/027/015 add-only Amendment + roadmap §3.22/§4 add-only + s2v-adapter add-only | `../tasks/task-40.3-closeout-v0.33.0.md` |

## 5. 依赖关系

- **task-40.1**（memory-actor-propagation）dep 既有 `core/src/data_plane/memory.rs` `pin()`（:215-240 已在）+ `set_pinned_with_actor`（store 层 actor 参数，task-27.1 / ADR-032 D1 已在）+ `PinMemoryRequest`（proto:336-339 已在）+ buf generate（已在）+ `internal/consoleapi/handlers.go` `handleMemoryPin`（:525-549 已在）+ header 读取范式（:815 / router.go:71 已在）+ `grpcclient.go` `memoryClient.Pin`（:724-726 已在）+ `memstore.go` `MemMemoryStore.Pin`（:653 已在）；可独立先行（不依赖 40.2）。
- **task-40.2**（l2-embedding-cache-true-lru）dep 既有 `core/src/embedding/cache.rs` `sqlite_get`（:140-150 已在）+ `sqlite_put` rowid-FIFO（:153-195，task-33.1 / ADR-038 D1 已在）+ `DEFAULT_L2_EMBEDDING_CACHE_CAP`（:117 已在）+ `with_sqlite_capacity`（task-33.1 已在）+ TEST-33.1.1（:402 镜像源，已在）；与 40.1 并行无依赖。
- **task-40.3**（closeout）dep 40.1 + 40.2 全 Done；release docs / smoke v30[49/49] / ADR-045 ratify 据两 task 真实测试 / 实测产物。
- 外部：ADR-045（本 phase 新 Proposed）/ ADR-032（memory-ops，pin actor 透传维度兑现 add-only Amendment）/ ADR-038 + ADR-027（embedding，L2 true-LRU 维度兑现 + 真-LRU 假设据实更正 add-only Amendment）/ ADR-015（proto add-only field Amendment）/ ADR-022 D2（memory pin 宽松 body 契约保持）/ ADR-004（默认行为 + 既有契约不变）/ ADR-008（dep add-only，Phase 40 不增 dep）/ ADR-012（tag/release outward-facing 须用户显式授权，本轮已授权 v0.33.0）/ ADR-014 **第三十一次**激活 / ADR-013（禁伪造红线，pin actor 真实透传非合成、L2 命中 bump 真实重排经单测；认证身份据实延后 / L2 写放大据实记 / 其余 marker 据实保持延后，不伪造、不夸大、不强行扩面）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（每条 AC 含 ADR-014 D3 verified by 显式 owner；Draft 阶段未勾选，实施后逐条置 `[x]`）**：

- [x] **AC1**（memory pin actor add-only 透传 🟢）: `PinMemoryRequest`（proto:336-339）add-only `string actor = 3`（既有 memory_id=1 / pin=2 字段号冻结，ADR-015）+ buf generate 重生 binding；`MemoryStore.Pin` Go interface 加 add-only `actor string`（`memoryClient.Pin` / `MemMemoryStore.Pin` 同步）+ `grpcclient` 填 `pb.PinMemoryRequest.Actor`；`handleMemoryPin`（:525-549）读 `X-Actor` header（缺省空串）透传；Rust `pin()`（:225-229）用 `req.actor` 非空透传、空回落 `"console-api"`（默认 byte-equiv，ADR-004）；认证身份 honest-defer `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`；ADR-022 D2 宽松 body 契约不改 — verified by **TEST-40.1.1**（`PinMemoryRequest{actor}` wire-tag 字段号 3，in-crate prost 断言）+ **TEST-40.1.2**（Rust `pin()` 空 actor 回落 `"console-api"` / 非空透传写入 `pinned_by`）+ **TEST-40.1.3**（Go `handleMemoryPin` 读 `X-Actor` 透传 `Pin(actor)` + 缺省空串）+ **TEST-40.1.4**（grpcclient 填 `pb.PinMemoryRequest.Actor`）+ phase-smoke step 1
- [x] **AC2**（L2 embedding 缓存访问序 LRU 🟢）: `sqlite_get`（:140-150）命中时（仅 `l2_cap > 0`）对命中行 `INSERT OR REPLACE` bump 隐式 rowid 到表尾，使 `sqlite_put`（:153-195）rowid 序驱逐由插入序 FIFO 升访问序 LRU；cap==0 不 bump（保插入序、零额外写）；复用既有隐式 rowid、**0 新 dep / 0 schema migration**；据实更正 Phase 33「真 LRU 须加 created_at 列 + ALTER」假设；命中 bump 写放大据实记、L2 无生产调用点现网零影响（opt-in-path 语义补全非已确认线上问题，ADR-013） — verified by **TEST-40.2.1**（cap=2 put a,b → 命中 a bump → put c → 驱逐 b 最久未用而非 a，对比 FIFO 旧行为驱逐 a）+ **TEST-40.2.2**（cap==0 不 bump 保插入序 + 默认行为不变 + 既有 33.1.* / 22.2.* 维持绿）+ phase-smoke step 2
- [x] **AC3**（honest-defer 边界 + v0.33.0 closeout + 默认零依赖守线）: pin actor 认证身份 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]` / `vector-dim-feature-enforce`（须 feature build）/ `tracestore-multi-workspace-strict`（余下读路径）/ `chunk-source-type-filter`（须 import-path schema migration）据实保持延后；默认行为 / proto（add-only field）/ 既有契约不变 + 0 网络（ADR-004）+ 0 新依赖（ADR-008）；v0.33.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ `scripts/console_smoke.sh` v30[49/49] + `internal/cli/smoke_syntax_test.go` `TestTask403` markers 同步（no-regression [37/37]..[48/48]）+ ADR-045 据真实测试 ratify + ADR-032/038/027/015 add-only Amendment + roadmap §3.22/§4 add-only + phase §6 闭合 — verified by **TEST-40.3.1**（smoke v30[49/49] + smoke_syntax_test + ADR-045 ratify + roadmap/adapter add-only + phase §6 闭合）
- [x] **AC4**（ADR-014 cross-validation gate）: ADR-014 D1-D5（**第三十一次**激活）全通过 — D1 mapping + D2 lint `bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-39 不溯改（ADR 改动 add-only Amendment）— verified by task-40.3 closeout PR body + 各 task LAST TEST（TEST-40.1.5 / TEST-40.2.3 / TEST-40.3.2）

**端到端 smoke（C1 集成兜底）**：(1) `PinMemoryRequest` add-only `actor=3` + Go `Pin(id,pin,actor)` 参数链 + `handleMemoryPin` 读 `X-Actor` header + Rust `pin()` 空 actor 回落 `"console-api"`（默认 byte-equiv）/ 非空透传写 `pinned_by` 全 PASS（认证身份据实延后如实标注）；(2) `core/src/embedding/cache.rs` `sqlite_get` 命中 bump 隐式 rowid → L2 驱逐最久未用（访问序 LRU）+ cap==0 不 bump 保插入序 + 默认行为不变 + 既有 33.1.* 维持绿全 PASS（写放大 / opt-in-path 现网零影响如实标注，0 schema migration）；(3) v0.33.0 收口 + 默认零依赖守线全 PASS。

## 7. 阶段级风险

- **R1（中）proto add-only `actor` 不破既有 client + 空 actor 回落 byte-equiv**：`PinMemoryRequest` 加 `actor` 须 add-only（既有 memory_id=1 / pin=2 字段号不动）且空值保回落 `"console-api"`，否则破既有控制面 client 或默认行为。
  - **缓解**：task-40.1 用字段号 3（既有 1/2 冻结，ADR-015 D1）；Rust `pin()` `if req.actor.is_empty() { "console-api" } else { &req.actor }`（空 actor = 既有硬编码值，byte-equiv）；buf generate 重生 binding + in-crate prost wire-tag 断言字段号 3 + 单测断言空 actor 结果与改前一致。stop-condition：add-only field 破既有 client / 空 actor 非回落 `"console-api"` 则 AC1 不标 `[x]`。
- **R2（中）Go `Pin` interface 加参数须两实现 + 调用点同步**：`MemoryStore.Pin` 加 `actor` 参数须 `memoryClient.Pin` / `MemMemoryStore.Pin` 两实现 + `handleMemoryPin` 调用点同步，漏改则编译失败或行为不一致。
  - **缓解**：task-40.1 一并改 interface + 两实现 + 调用点；`go build ./...` + `go vet` 守编译；TEST-40.1.3 断言 `handleMemoryPin` 读 header 透传、TEST-40.1.4 断言 grpcclient 填 proto Actor。stop-condition：编译失败 / 任一实现漏改则 AC1 不标 `[x]`。
- **R3（中）L2 命中 bump 误改驱逐语义 / 默认行为回归**：`sqlite_get` 命中加 `INSERT OR REPLACE` bump rowid 若条件或 SQL 有偏，会误驱逐或破默认行为；cap==0 时若仍 bump 会引入无谓写放大。
  - **缓解**：task-40.2 仅在 `l2_cap > 0` 时 bump（cap==0 不限 → 不 bump，保插入序、零额外写）；既有 `sqlite_put` rowid 序驱逐 SQL 不改（仅 rowid 含义由插入序变访问序）；TEST-40.2.1 断言命中 bump 后驱逐最久未用（对比 FIFO 旧行为）、TEST-40.2.2 断言 cap==0 不 bump + 既有 33.1.* / 22.2.* 维持绿。stop-condition：LRU 命中重排不生效 / cap==0 引入写放大 / 默认行为回归则 AC2 不标 `[x]`。
- **R4（低）L2 命中 bump 写放大被误读为线上回归**：命中即重写行给读路径加写 I/O，易被夸大为性能回归。
  - **缓解**：spec §2 40.2 + ADR-045 D2 据实记「访问序 LRU 固有代价（同 Go memstore move-to-front）+ L2 无生产调用点（Phase 33 已标 opt-in-path）现网零影响」（ADR-013 不夸大）；本项价值在 opt-in 路径被启用时的访问序 LRU 正确性。stop-condition：若把 opt-in-path 语义补全夸大为线上修复则越界。
- **R5（低）真-LRU 假设更正被误读为否定 Phase 33**：本 phase 据实更正 Phase 33「真 LRU 须加时间列」的假设，易被误读为 Phase 33 D1 有错。
  - **缓解**：task-40.2 spec / ADR-045 D2 明记 Phase 33 D1 的 rowid-FIFO 是正确且必要的前序（row-count cap 本身），本 phase 仅补「命中 bump 使 rowid 序由插入序变访问序」这一增量——以 add-only Amendment 记于 ADR-038/027，不溯改其正文（ADR-014 D5）。stop-condition：若溯改 Phase 33 ADR 正文则违 ADR-014 D5。

## 8. Definition of Done

- 3 task spec（40.1-40.3）顶部 `**Status**: Done`
- §6 阶段级 AC1-4 全 `[x]`（受阻 / 延后态按 ADR-013 如实记录，不伪造——如 pin actor 认证身份据实延后 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`，其余治理 marker 据实保持延后）
- 端到端 smoke 3 step 全 PASS（含受阻 / 延后态如实标注）
- **ADR**：ADR-045 `Proposed → Accepted`（据真实测试 / 实测产物逐 D 项 ratify）；ADR-032 经 add-only Amendment 记录（pin actor 透传维度兑现 + 认证身份续延后，不溯改正文，ADR-014 D5）；ADR-038 + ADR-027（L2 true-LRU 维度兑现 + 真-LRU 假设据实更正 add-only）/ ADR-015（proto add-only field）Amendment；ADR-022 D2（memory pin 宽松 body 契约保持）守线引用；`docs/roadmap.md §3.22/§4` add-only（Phase 40 行 + memory-actor-authenticated-identity 新 backlog 条目）
- **adapter**：§Phase 索引 Phase 40 `Draft → Done` + `Tasks 0 → 3`；§ADR 索引 ADR-045；§BDD 追加 phase-40 feature 行；ADR-032/038/027/015 Amendment 记录
- **release**：`docs/releases/v0.33.0-{evidence,artifacts}.md` + `RELEASE_NOTES.md` v0.33 段 + README v0.33 段
- **smoke**：`scripts/console_smoke.sh` v30[49/49]（memory pin actor 透传 + L2 访问序 LRU smoke + 既有 step 不退化，denominators [37/37]..[48/48] 不溯改）+ `internal/cli/smoke_syntax_test.go` `TestTask403` markers 同步
- **follow-up**：pin actor 认证身份 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]` + L2 TTL `[SPEC-DEFER:phase-future.l2-cache-ttl]` + `with_sqlite` 生产接线 `[SPEC-DEFER:phase-future.l2-cache-production-wire]` + vector dim 强校验 `[SPEC-DEFER:phase-future.vector-dim-feature-enforce]` + TraceStore 多 workspace 余下读路径 `[SPEC-DEFER:phase-future.tracestore-multi-workspace-strict]` + chunk 过滤 `[SPEC-DEFER:phase-future.chunk-source-type-filter]` / `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]` 留 backlog
