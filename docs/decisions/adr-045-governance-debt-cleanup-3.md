# ADR `045`: `governance-debt-cleanup-3`

**Status**: Accepted（v0.33.0 / task-40.3 closeout 据真实 CI 逐 D ratify；D1/D2/D3 Accepted——见 §Ratification）

**Category**: 治理债清理（第三轮）/ actor 透传 / 缓存访问序 LRU / 契约诚实化
**Date**: 2026-06-07
**Decided By**: 主 agent（ADR-012 自治）；tajiaoyezi ratification at v0.33.0 closeout
**Related**: ADR-036（governance-debt-cleanup — 第一轮）/ ADR-038（governance-debt-cleanup-2 — 第二轮；本 ADR 为第三轮，镜像其「逐项据实分级 🟢/🟡，受阻 / 非问题 / 已交付项据 ADR-013 honest-defer，不溯改正文」方法；其 D1 给 L2 加 rowid-FIFO + 据「真 LRU 须加时间列」延后，本 ADR D2 兑现真-LRU 维度 + 据实更正该假设）/ ADR-032（memory-ops-hardening — pin actor first-class store 字段，本 ADR D1 兑现 actor 入口透传维度）/ ADR-027（embedding-provider-selection — L2 cache 有界化前序，本 ADR D2 add-only Amendment）/ ADR-015（console-data-plane proto 契约 — `PinMemoryRequest` add-only `actor=3` 既有字段号冻结）/ ADR-022（D2 — memory pin lenient body contract，本 ADR D1 据实保持，不改）/ ADR-004（local-first-privacy-baseline — 默认行为 / proto（add-only）/ 既有契约不变 + 0 网络）/ ADR-008（dep add-only — Phase 40 = 0 新依赖）/ ADR-013（禁伪造红线 — pin actor 真实透传非合成、L2 命中 bump 真实重排经单测；认证身份据实延后 / L2 写放大据实记 / opt-in-path 现网零影响不夸大）/ ADR-012（main-agent-governance-autonomy — tag/release outward-facing 须用户显式授权，v0.33.0 本轮已授权）/ ADR-014（D1-D5，第三十一次激活）/ roadmap §3.22 + §4

## Context

ContextForge 截至 Phase 39（console-api-retrieval-signal-forward, Done / v0.32.0）已把对外检索信号（hybrid + rerank provenance）贯通到 console-api REST。本 Phase 40 是**第三轮治理债清扫**（镜像 Phase 31 / ADR-036 + Phase 33 / ADR-038），清理两组在 grounding 中确认为**真实且 code-local 可单测**的跨 Phase 治理 marker，并对 Phase 33 一处延后假设据实更正。逐维度调研结论：

- **memory pin actor 硬编码（真实，透传链断点）**：`core/src/data_plane/memory.rs` `pin()`（:215-240）调 `set_pinned_with_actor(&req.memory_id, req.pin, "console-api")`（:229），actor 写死 `"console-api"`（doc-comment :225-227 明记 `[SPEC-DEFER:phase-future.memory-actor-propagation]`）。根因：`set_pinned_with_actor`（store 层）本就接受 actor（task-27.1 / ADR-032 D1 已把 pin actor 做成 first-class store 字段 + proto `MemoryItem.pinned_by=11`），但**入口到 store 的透传链缺失**——`PinMemoryRequest`（proto:336-339）无 actor field、Go `MemoryStore.Pin(id,pin)` 无 actor 参数、`handleMemoryPin`（handlers.go:525-549）不读调用方标识。console-api 已多处用 `r.Header.Get`（:815 / router.go:71/89）的 header 读取范式。

- **L2 embedding 缓存是 rowid-FIFO（插入序）而非访问序 LRU（真实，opt-in-path）**：`core/src/embedding/cache.rs` Phase 33 D1（ADR-038 D1）给 L2 SQLite `embedding_cache` 加了 row-count cap + rowid-FIFO 驱逐（`sqlite_put` :153-195，`DELETE ... WHERE rowid NOT IN (SELECT rowid ... ORDER BY rowid DESC LIMIT cap)`，隐式 rowid = 插入序），但 `sqlite_get`（:140-150）命中**不**重排 → 驱逐是插入序 FIFO（频繁命中的旧行仍因插入早被逐）。Phase 33 当时据「带 created_at 列的真 LRU 须 ALTER 既有用户文件 → 破 0-migration」把真-LRU 延后（ADR-038 A2/D4，marker `[SPEC-DEFER:phase-future.l2-cache-true-lru]`）。**grounding 据实更正该假设**：访问序 LRU 不须时间列——命中时 `INSERT OR REPLACE` 同行使其隐式 rowid 跳表尾即把 rowid 序由插入序变访问序，既有驱逐随之升 LRU，**复用既有隐式 rowid、0 schema migration**，与 Go memstore 命中 move-to-front（task-33.2 / ADR-038 D2）同技法。**诚实 caveat**：`with_sqlite` 经 Phase 33 D1 已据实标注无生产调用点（test-only，出厂 daemon 走 memory-only L1）→ 本项是 opt-in 路径语义补全、非已确认线上问题；命中 bump 给读路径加一次行重写（写放大）是访问序 LRU 固有代价。

本 ADR 把上述「memory pin actor 透传 / L2 访问序 LRU」收敛为一个集中清理 + 诚实化 Phase 的处理策略。改动**两项均 code-local 🟢 可单测**。全部改动遵守 ADR-004 默认行为 / proto（add-only field）/ 既有契约不变 + 0 网络 + ADR-008 0 新依赖 + ADR-013 受阻 / 据实分级不伪造。

## Decision

第三轮治理债清理采用 **「actor 入口透传 + 缓存访问序 LRU + 契约诚实化 + 默认零依赖守线」** 策略，分 3 个决策点：

### D1 — memory pin actor add-only 透传（认证身份 honest-defer）（task-40.1）🟢

补 memory pin 的入口到 store actor 透传链：`PinMemoryRequest`（proto:336-339）add-only `string actor = 3`（既有 `memory_id=1` / `pin=2` 字段号冻结，ADR-015 D1）+ buf generate → Go `MemoryStore.Pin(id,pin)` → `Pin(id,pin,actor)`（interface + `memoryClient.Pin` / `MemMemoryStore.Pin` 两实现 + 调用点同步）+ `grpcclient` 填 `pb.PinMemoryRequest{..,Actor:actor}` → `handleMemoryPin`（handlers.go:525-549）读 `r.Header.Get("X-Actor")`（缺省空串）透传 → Rust `pin()`（:229）`set_pinned_with_actor(.., if req.actor.is_empty() { "console-api" } else { req.actor.as_str() })`。console 部署在设 `X-Actor` / `X-Forwarded-User` 的 auth 代理后可把 pin/unpin 归因到真实调用方（写入既有 `pinned_by` + audit）。

**理由**：task-27.1 / ADR-032 D1 已把 pin actor 做成 store first-class 字段，但入口透传链缺失令 `pin()` 只能硬编码常量——这是 ADR-032 自记的治理债（`[SPEC-DEFER:phase-future.memory-actor-propagation]`）。add-only proto field（字段号冻结）+ Go 参数链 + REST header 是最 surgical 的透传补全：默认（无 `X-Actor` header / 既有 client 不传 actor）→ proto3 空串 → Rust 回落 `"console-api"` → byte-equiv（ADR-004）。**诚实 caveat（ADR-013）**：本轮交付**调用方透传**（actor 取自 header，未做认证校验）；把 header 值校验映射为已认证 auth subject 须 console-api 鉴权层 → honest-defer `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`，spec / ADR 据实记为「调用方声明的标识」非「已认证身份」，不夸大。`handleMemoryPin` 宽松 body 契约（ADR-022 D2）保持不改——仅在入口加 header 读取。备选「把 actor 经 context 隐式传」不如显式参数清晰、不取（见 §A2）。

### D2 — L2 embedding 缓存访问序 LRU（命中 bump 隐式 rowid，0 migration；真-LRU 假设据实更正）（task-40.2）🟢

把 L2 SQLite 缓存的驱逐由插入序 FIFO（Phase 33 D1）升为访问序 LRU：`sqlite_get`（:140-150）命中时，仅当 `l2_cap > 0` 时对命中行 `INSERT OR REPLACE INTO embedding_cache`（同 `(content_hash,provider,dim,vector)`）原样回写以 bump 其隐式 rowid 到表尾（值不变、仅 rowid 变新）；既有 `sqlite_put`（:153-195）的 rowid 序驱逐（`ORDER BY rowid DESC LIMIT cap`）随之由 FIFO 升 LRU（驱逐最久未**用** = 最小 rowid）。`cap==0`（不限）不 bump（保插入序、零额外写）。**0 新依赖、0 schema migration**（复用既有隐式 rowid）。

**理由**：Phase 33 D1 的 rowid-FIFO（row-count cap 本身）是正确且必要的前序，但命中不重排令驱逐是插入序而非访问序——一个热的旧行会被误逐。**grounding 据实更正 Phase 33「真 LRU 须加 created_at 列 + ALTER」的假设**：命中 bump 隐式 rowid 即得访问序 LRU，不须时间列、0 schema migration，与 Go memstore 命中 move-to-front（task-33.2 / ADR-038 D2）同技法——同一代码库已有此范式。仅在 `l2_cap > 0` 时 bump（不限容量下无驱逐 → LRU 序无意义、避免无谓写放大）。**诚实 caveat（ADR-013）**：命中 bump 给 L2 读路径加一次行重写（写放大），是访问序 LRU 的固有代价（同 Go memstore move-to-front）；且 `with_sqlite` 无生产调用点（Phase 33 D1 已据实标注 opt-in-path，出厂 daemon 走 memory-only L1）→ 现网零影响，本项是 **opt-in 路径语义补全、非已确认线上问题**，不夸大。本 ADR 以 add-only Amendment 记于 ADR-038/027（兑现真-LRU 维度 + 据实更正假设），**不溯改其正文**（ADR-014 D5）。备选「带 created_at 列的真 LRU」须 ALTER 既有文件（破 0-migration），不取（见 §A3）。

### D3 — 默认行为 + proto（add-only field）+ 0-dep / 0-network 不变 + honest-defer 边界（all tasks）🟢

所有改动保持默认行为 / proto（add-only field）/ 既有契约不变 + 0 网络（ADR-004）+ 0 新依赖（ADR-008，Phase 40 = 0 dep）：pin actor proto field add-only（既有 memory_id=1 / pin=2 字段号不动）+ Go `Pin` 参数 add-only（空 actor 回落 `"console-api"` byte-equiv）；L2 命中 bump 仅在有限 cap 下生效（cap==0 不 bump byte+perf-equiv）且驱逐结果在默认 cap 下与既有 round-trip 返回结果一致。受阻 / 另一层项据 ADR-013 honest-defer：pin actor 认证身份 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`；L2 TTL `[SPEC-DEFER:phase-future.l2-cache-ttl]` / `with_sqlite` 生产接线 `[SPEC-DEFER:phase-future.l2-cache-production-wire]`；其余治理 marker 据实保持延后不强行扩面——`vector-dim-feature-enforce`（须 feature build）/ `tracestore-multi-workspace-strict`（余下读路径深化）/ `chunk-source-type-filter` / `chunk-agent-scope-filter`（须 import-path schema migration）。既有 `cargo-test` / `go-test` / `spec-lint` 三门不退化。

**理由**：ADR-004 local-first + ADR-008 dep add-only——默认行为 / proto（add-only）/ 既有契约不变 + 0 网络 + 0 新依赖是不可让渡 baseline。本 phase 为治理债清理 + 诚实化、非默认行为演进。add-only proto field（空回落 byte-equiv）/ 命中 bump（仅有限 cap、结果等价）使既有用户与既有契约零感知。受阻 / 另一层 / 须更大改面的 marker 据 ADR-013 逐项据实分级、honest-defer，不强行扩面（焦点小版本，honest over padding）。

## Consequences

- **Positive**: memory pin actor 入口透传补全（proto add-only `actor=3` + Go 参数链 + REST `X-Actor` header + Rust 空回落 `"console-api"`，console 在 auth 代理后可把 pin 归因真实调用方，写入既有 `pinned_by`）；L2 SQLite embedding 缓存由插入序 FIFO 升访问序 LRU（命中 bump 隐式 rowid，0 新 dep / 0 schema migration，与 Go memstore move-to-front 同技法，热行不被误逐）；据实更正 Phase 33「真 LRU 须加时间列」假设；全部 0-dep / 0-network / default 保形 / add-only proto field，默认行为 / proto / 既有契约不变（ADR-004 / ADR-008），既有三门不退化。
- **Negative / open**（受阻 / 另一层项如实，不伪造、不夸大）：pin actor 认证身份（把 header 值校验为已认证 auth subject）→ honest-defer `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`（D1 仅交付调用方透传）；L2 命中 bump 写放大 = 访问序 LRU 固有代价（同 Go memstore），且 L2 无生产调用点 → 现网零影响（opt-in 路径语义补全非已确认线上问题）；L2 TTL `[SPEC-DEFER:phase-future.l2-cache-ttl]` / `with_sqlite` 生产接线 `[SPEC-DEFER:phase-future.l2-cache-production-wire]`；其余治理 marker（`vector-dim-feature-enforce` 须 feature build / `tracestore-multi-workspace-strict` 余下读路径 / `chunk-source-type-filter` 须 import-path migration）据 ADR-013 据实保持延后、不强行扩面。
- **Ratification**: 本 ADR **Proposed**。task-40.1/40.2 通过后于 v0.33.0 closeout（task-40.3）据真实 CI / 实测产物逐 D ratify Proposed→Accepted（ADR-013：禁据合成 / 伪造 ratify）。
- **Follow-ups**: pin actor 认证身份（console-api 鉴权层后）`[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`；其它 memory RPC actor 透传 `[SPEC-DEFER:phase-future.memory-actor-all-rpc]`；L2 TTL `[SPEC-DEFER:phase-future.l2-cache-ttl]`；`with_sqlite` 生产接线 `[SPEC-DEFER:phase-future.l2-cache-production-wire]`；L2 命中 bump 写放大优化 `[SPEC-DEFER:phase-future.l2-lru-bump-batching]`。ADR-032（pin actor 透传维度兑现）/ ADR-038 + ADR-027（L2 true-LRU 维度兑现 + 真-LRU 假设据实更正）/ ADR-015（proto add-only field）以 add-only Amendment 于 task-40.3 记录（不溯改正文，ADR-014 D5）；ADR-022（D2 lenient 保持）/ ADR-004 / ADR-008 / ADR-013 引用均不溯改其正文。

## Ratification（v0.33.0 / task-40.3）

本 ADR 于 v0.33.0 closeout（task-40.3）据 task-40.1/40.2 真实 CI（cargo-test / go-test / lint / spec-lint 四门绿）逐 D ratify Proposed→Accepted。各 D 真实依据：

- **D1（memory pin actor add-only 透传）→ Accepted 🟢**：task-40.1（PR #257，master @ `68046c3`）落 `PinMemoryRequest` add-only `actor=3`（既有 `memory_id=1` / `pin=2` 字段号冻结）+ Go `MemoryClient.Pin(id,pin)` → `Pin(id,pin,actor)`（interface + `memoryClient` / `MemMemoryStore` / `degradedMemory` 三实现）+ `grpcclient` 填 `pb.PinMemoryRequest.Actor` + `handleMemoryPin` 读 `r.Header.Get("X-Actor")` + Rust `pin()` `if req.actor.is_empty() { "console-api" } else { req.actor.as_str() }`。`test_pin_actor_proto_field_number`（TEST-40.1.1，prost wire-tag actor=3 = `[0x1A,0x01,0x78]`）+ `test_memory_server_pin_propagates_actor`（TEST-40.1.2，非空透传 `pinned_by`）+ `test_memory_server_pin_writes_actor_and_projects`（守护空回落 `"console-api"` byte-equiv）+ `TestTask401_HandleMemoryPin_ForwardsXActorHeader`（TEST-40.1.3）+ `TestTask401_GrpcClient_Pin_ForwardsActor`（TEST-40.1.4）全绿。认证身份 honest-defer `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`；ADR-022 D2 宽松 body 契约不改。
- **D2（L2 embedding 缓存访问序 LRU）→ Accepted 🟢**：task-40.2（PR #258，master @ `08e8db6`）`sqlite_get` 命中分支（仅 `l2_cap > 0`）`INSERT OR REPLACE` 原样回写命中行 bump 隐式 rowid → 既有 `sqlite_put` rowid 序驱逐由插入序 FIFO 升访问序 LRU；cap==0 不 bump。`test_40_2_1_l2_hit_bump_evicts_lru`（TEST-40.2.1，cap=2 命中 a → 驱逐最久未用 b 而非 a，对比 FIFO 旧行为驱逐 a）+ `test_40_2_2_cap_gates_hit_bump_and_keeps_results`（TEST-40.2.2，cap>0 命中 bump rowid `[a,b]`→`[b,a]` / cap==0 不 bump 保插入序 + 返回结果不变）全绿（含 TEST-33.1.* / 22.2.* 回归）。真-LRU 假设据实更正（命中 bump 隐式 rowid，0 schema migration，与 Go memstore move-to-front 同技法）；写放大 + `with_sqlite` 无生产调用点现网零影响据实记。
- **D3（默认行为 + proto add-only + 0-dep / 0-network + honest-defer 边界）→ Accepted 🟢**：全 phase 0 新 dep；proto `actor=3` add-only（既有字段号 1-2 不动）；默认（无 `X-Actor` header / 既有 client）→ 空 actor 回落 `"console-api"` byte-equiv + L2 cap==0 / 命中 bump 结果不变；既有 `cargo test --workspace` + `go test ./...` + `cargo clippy` 三门不退化。其余治理 marker（`vector-dim-feature-enforce` / `tracestore-multi-workspace-strict` / `chunk-source-type-filter`）据实保持延后、不强行扩面。多 agent 对抗审查（4 维度 review + 每 finding 3 独立 skeptic 对抗核实，承 Phase 38/39 实践）核实 0 真实缺陷后方 ratify。

真实 v0.33.0 tag/run/digest 经用户授权后由 post-tag-push backfill 填实（release docs `<backfill>`，ADR-013 不预填）。

## Alternatives

- **A1（pin actor 不透传，保持硬编码 "console-api"）**：保留 `pin()` 硬编码 actor。否决：ADR-032 D1 已把 pin actor 做成 store first-class 字段并自记透传债（`[SPEC-DEFER:phase-future.memory-actor-propagation]`）；入口透传链是真实治理缺口，add-only proto field + Go 参数 + header 0 新 dep 即补，且默认空回落 byte-equiv 不破既有。
- **A2（pin actor 经 context 隐式传而非显式参数）**：把 actor 塞进 request context / metadata 隐式传。否决：显式 proto field + Go 参数链更清晰、可测、与既有 `set_pinned_with_actor` 显式 actor 参数一致；隐式传增理解成本且难单测。
- **A3（L2 带 created_at 列的真 LRU）**：给 `embedding_cache` 加 `created_at` / `last_access` 列 + 按时间驱逐。否决：须对既有用户 SQLite 文件 ALTER（破 0-migration）；grounding 更正——命中 bump 隐式 rowid 即得访问序 LRU（0 schema migration，与 Go memstore move-to-front 同技法），无须时间列。
- **A4（L2 不补访问序 LRU，保持 rowid-FIFO）**：保留 Phase 33 的插入序 FIFO。否决：插入序 FIFO 会误逐热的旧行（命中不延寿），访问序 LRU 是缓存的正确语义；命中 bump 0 新 dep / 0 migration 即升，opt-in 路径语义补全有价值（据实标注现网零影响，不夸大）。
- **A5（L2 命中 bump 不论 cap 都执行）**：`sqlite_get` 命中无条件 bump。否决：`cap==0`（不限）下无驱逐 → LRU 序无意义，无条件 bump 引入无谓写放大；据 D2 仅 `l2_cap > 0` 时 bump。
- **A6（本轮强行扩面做认证身份 / vector-dim-enforce / tracestore 余下读路径 / chunk filter）**：一并实现所有相关 marker。否决：认证身份须 console-api 鉴权层、vector-dim-enforce 须 feature build、tracestore 余下读路径 / chunk filter 须更大改面（import-path schema migration）——据 ADR-013 逐项据实分级、honest-defer，焦点小版本不强行扩面（honest over padding）。

## 触及 ADR 关系

- **ADR-032（memory-ops-hardening）→ add-only Amendment @ task-40.3**：D1 把 pin actor 做成 store first-class 字段并自记入口透传债（`[SPEC-DEFER:phase-future.memory-actor-propagation]`）；本 phase D1 兑现 actor 入口透传维度（proto add-only field + Go 参数链 + REST header + 空回落 byte-equiv；认证身份续延后）。以 `## Amendment (Phase 40 / v0.33.0)` add-only 记，**不溯改 ADR-032 正文**（ADR-014 D5）。
- **ADR-038（governance-debt-cleanup-2）→ add-only Amendment @ task-40.3**：其 D1 给 L2 加 rowid-FIFO（row-count cap）+ 据「真 LRU 须加时间列」把真-LRU 延后（A2/D4）；本 phase D2 兑现真-LRU 维度（命中 bump 隐式 rowid，0 migration）+ 据实更正该假设。以 add-only Amendment 记，**不溯改 ADR-038 正文**（ADR-014 D5）。
- **ADR-027（embedding-provider-selection）→ add-only Amendment @ task-40.3**：L2 cache 有界化前序（Phase 33 D1 / ADR-027）；本 phase D2 在其上补访问序 LRU。以 add-only Amendment 记，**不溯改 ADR-027 正文**（ADR-014 D5）。
- **ADR-015（console-data-plane proto 契约）→ add-only Amendment @ task-40.3**：`PinMemoryRequest` add-only `actor=3`（既有 memory_id=1 / pin=2 字段号冻结 D1 + add-only D2）。以 add-only Amendment 记。
- **ADR-022（D2 — memory pin lenient body contract）→ 守线引用（不溯改）**：`handleMemoryPin` 宽松 body 契约是 ADR-022 D2 刻意决策，本 phase D1 仅在入口加 `X-Actor` header 读取、不改 body 解析行为，不溯改其正文。
- **ADR-036（governance-debt-cleanup）+ ADR-038（governance-debt-cleanup-2）→ 方法镜像（不溯改）**：本 ADR 为第三轮治理债清扫，镜像其逐项据实分级 + honest-defer 方法，不溯改其正文。
- **ADR-004（local-first-privacy-baseline）→ 守线**：默认行为 / proto（add-only field）/ 既有契约不变 + 0 网络（D3）守 ADR-004 baseline。
- **ADR-008（dep add-only）→ 守线**：本 phase 加 **0 新依赖**（actor 透传 / L2 命中 bump 均 0-dep）。
- **ADR-013（禁伪造红线）→ 守线**：pin actor 真实透传非合成、L2 命中 bump 真实重排经单测；认证身份据实延后 / L2 写放大据实记 / opt-in-path 现网零影响不夸大 / 其余 marker 据实保持延后（D1 / D2 / D3）。
- **ADR-014（cross-phase-exit-criteria-validation）→ 第三十一次激活**：D1-D5 mapping + 各 task LAST D2 lint（touched 行 0 未标注命中）+ D3 verified-by + D4 自治 + D5 历史 Phase 1-39 不溯改（ADR 改动 add-only Amendment）；本 ADR ratify 在 task-40.3 closeout，Draft 阶段不 ratify。

## Amendment (Phase 44 / v0.37.0) — memory-actor-all-rpc Unpin 子项 + emit_audit_and_event 共用基础兑现 (add-only)

> add-only Amendment（不溯改本 ADR D-body / Ratification (v0.33.0)，ADR-014 D5）。承本 ADR §D1（memory pin actor 透传）+ roadmap 行 556 新增 backlog `memory-actor-all-rpc`（其它 memory RPC 的 actor 透传）的 Unpin 子项。

Phase 44 / v0.37.0（ADR-049）兑现 `memory-actor-all-rpc` 的 **Unpin 子项** + `emit_audit_and_event` actor 参数共用基础——本 ADR §D1 task-40.1 给 pin 加了 actor 透传，roadmap 行 556 把 `memory-actor-all-rpc`（其它 memory RPC 的 actor 透传）列为新增 backlog。Phase 44 grounding 发现真实价值在 audit/event（store pinned=false 丢弃 actor）→ `emit_audit_and_event` 加 actor 参数让 audit/event source 归因（unpin + pin 顺带闭环）。task-44.1（PR #280）闭环 unpin 透传 + Go 透传链 + proto add-only。

**`memory-actor-all-rpc` 部分兑现（Unpin 子项 + 共用基础）**：Unpin actor 透传 ✅（本 phase）；`emit_audit_and_event` actor 参数共用基础 ✅（deprecate/softdelete/harddelete 未来顺带受益）。**仍延后**：Deprecate/SoftDelete actor 透传（须 7 层 + 新 schema migration——`set_status` 无 actor 参数 + 无列记录 actor）；HardDelete actor 透传（须 audit 层重设计——hard_delete 物理删行无落点）→ 续 `[SPEC-DEFER:phase-future.memory-actor-all-rpc]`。不溯改本 ADR D-body（ADR-014 D5）。验证 TEST-44.1.1/.2/.3/.4。详见 ADR-049 Ratification (v0.37.0)。
