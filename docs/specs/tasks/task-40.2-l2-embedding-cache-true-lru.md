# Task `40.2`: `l2-embedding-cache-true-lru — cache.rs sqlite_get 命中即对命中行 INSERT OR REPLACE bump 隐式 rowid 到表尾（仅有限 l2_cap），使既有 sqlite_put 的 rowid 序驱逐由插入序 FIFO（Phase 33 D1）升为访问序 LRU；复用既有隐式 rowid、0 新 dep / 0 schema migration；据实更正 Phase 33「真 LRU 须加 created_at 列 + ALTER」假设；命中 bump 写放大据实记 + L2 无生产调用点现网零影响（opt-in-path 语义补全非已确认线上问题）`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 40 (governance-debt-cleanup-3)
**Dependencies**: 既有 `core/src/embedding/cache.rs`（task-22.2 `CachingEmbeddingProvider` L2 SQLite 持久化 / task-33.1 L2 row-count cap + rowid-FIFO 驱逐 `sqlite_put` :153-195 + `DEFAULT_L2_EMBEDDING_CACHE_CAP` :117 + `with_sqlite_capacity` ctor，Phase 33 / ADR-038 D1 已交付）/ ADR-038（governance-debt-cleanup-2，D1 rowid-FIFO + 真-LRU 据「须加时间列」延后，本 task 真-LRU 维度兑现 + 假设据实更正为 add-only Amendment @ task-40.3 closeout）/ ADR-027（embedding-provider-completion，L2 cache 有界化前序，本 task add-only Amendment）/ ADR-004（local-first-privacy-baseline，默认行为 + 既有契约不变，cap==0 不 bump byte-equiv）/ ADR-008（dep add-only，Phase 40 = 0 新 dep）/ ADR-013（禁伪造红线——命中 bump 真实重排经单测、写放大据实记、opt-in-path 现网零影响不夸大）/ ADR-012（main-agent-governance-autonomy）/ ADR-014 D1-D5（第三十一次激活）

## 1. Background

task-33.1（ADR-038 D1）已给 L2 SQLite `embedding_cache` 加了 row-count cap + rowid-FIFO 驱逐，但**驱逐序是插入序 FIFO 而非访问序 LRU**——这是 Phase 33 当时据一处假设主动延后的增量：

- **B1 Phase 33 D1 的 rowid-FIFO（插入序）**：`core/src/embedding/cache.rs` `sqlite_put`（:153-195）在 `INSERT OR REPLACE` 后做 row-count cap：超 cap 则 `DELETE FROM embedding_cache WHERE rowid NOT IN (SELECT rowid FROM embedding_cache ORDER BY rowid DESC LIMIT cap)`（:185-187，保最大 cap 个 rowid、删其余）。隐式 rowid 单调递增 = **插入序** → 驱逐最早**插入**的行（FIFO）。
- **B2 sqlite_get 命中不重排**：`sqlite_get`（:140-150）L2 命中只读 vector 返回，**不**对命中行做任何重写/重排 → 命中不改其 rowid → 一个频繁命中的旧行仍因插入早而被 FIFO 驱逐（非访问序 LRU）。
- **B3 Phase 33 的延后假设（本 task 据实更正）**：Phase 33（ADR-038 A2 / D4，marker `[SPEC-DEFER:phase-future.l2-cache-true-lru]`）把真-LRU 据「带 created_at 列的真 LRU 须 ALTER 既有用户文件 → 破 0-migration」判定延后。**本轮 grounding 据实更正该假设**：访问序 LRU **不**须新增时间列——命中时对该行 `INSERT OR REPLACE`（同 `(content_hash,provider,dim,vector)`）使其隐式 rowid 跳到表尾（= 最新），即把隐式 rowid 由「插入序」变「访问序」，既有 `sqlite_put` 的 rowid 序驱逐随之由 FIFO 升为 LRU。**复用既有隐式 rowid、0 新 dep、0 schema migration**，与 Go memstore 命中 move-to-front（task-33.2 / ADR-038 D2）同技法。
- **B4 opt-in-path 现网零影响（据实，不夸大）**：`with_sqlite` 经 Phase 33 D1 已据实标注**无生产调用点**（test-only，出厂 daemon 走 memory-only L1）→ 本 task 的访问序 LRU 是 **opt-in 持久化路径的语义补全、非已确认线上问题**（ADR-013 不夸大）；命中 bump 给 L2 读路径加一次行重写（写放大）是访问序 LRU 的固有代价（同 Go memstore move-to-front），据实记。

本 task 在 `sqlite_get`（:140-150）命中分支补「仅有限 cap 时 bump 命中行 rowid」，为 code-local 🟢 可单测，0 新 dep（仅既有 `rusqlite` `INSERT OR REPLACE`）+ 0 schema migration（复用隐式 rowid）。

## 2. Goal

(1) **B2/B3**：`sqlite_get`（:140-150）命中分支：仅当 `l2_cap > 0`（有限 cap）时对命中行 `INSERT OR REPLACE INTO embedding_cache`（同 `(content_hash,provider,dim,vector)`）bump 其隐式 rowid 到表尾，使既有 `sqlite_put`（:153-195）的 rowid 序驱逐由插入序 FIFO 升访问序 LRU；`cap==0`（不限）不 bump（保插入序、零额外写）。**0 新 dep + 0 schema migration**（复用隐式 rowid，不加列、不 ALTER）。(2) **B3**：据实更正 Phase 33「真 LRU 须加 created_at 列 + ALTER」假设（命中 bump 即得访问序 LRU，与 Go memstore move-to-front 同技法）——记于 spec / ADR-045 D2 + ADR-038/027 add-only Amendment。(3) **B4**：命中 bump 写放大 + L2 无生产调用点现网零影响据实记入 spec / ADR-045 D2（ADR-013 不夸大）。

pass bar：访问序 LRU 经确定性单测验证（cap=2，put a,b → 命中 a（bump）→ put c → 驱逐 b（最久未**用**）而非 a；FIFO 旧行为会驱逐 a）（🟢）；`cap==0` 不 bump 保插入序、零额外写（🟢）；默认 cap 下既有 L2 round-trip（TEST-22.2.3）+ L1 既有 cap（TEST-31.2.1）+ L2 rowid cap（TEST-33.1.1）不退化（🟢）；公共构造源码兼容、0 新 dep（ADR-008）、0 schema migration（ADR-004 既有契约不变）；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `core/src/embedding/cache.rs`——`sqlite_get`（:140-150）命中分支（读到 vector 后、返回前）：仅当 `l2_cap > 0` 时执行 `INSERT OR REPLACE INTO embedding_cache (content_hash, provider, dim, vector) VALUES (?,?,?,?)` 重写命中行以 bump 其隐式 rowid 到表尾（命中行数据原样回写，值不变、仅 rowid 变新）；`l2_cap == 0`（不限）不 bump（保插入序、零额外写）。
- 既有 `sqlite_put`（:153-195）rowid 序驱逐 SQL **不改**——隐式 rowid 由插入序变访问序后，既有 `DELETE ... ORDER BY rowid DESC LIMIT cap` 自动由 FIFO 升 LRU。
- `CREATE TABLE`（:110-120）不改——复用既有隐式 rowid，不加 `created_at` / `last_access` 列、不 ALTER、不新增编号 migration（0 schema migration）。
- 同源测试：`cache.rs` 同源 test（镜像 TEST-33.1.1 形态 + SQLite 文件 round-trip）断言 cap=2 put a,b → 命中 a（bump）→ put c → L2 行数 ≤ 2 + 驱逐 b（最久未用，重读 miss）、保留 a（命中 bump，重读 hit）；对比 FIFO 旧行为（驱逐 a）；cap==0 不 bump、行为同 Phase 33 FIFO 基线。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- L2 缓存 TTL / 时间过期（须时间列）[SPEC-DEFER:phase-future.l2-cache-ttl]——本 task 只补访问序 LRU（命中 bump rowid），时间维度过期延后。
- L1 内存缓存的 LRU（task-31.2 `BoundedCache` 是 FIFO，本 task 范围限 L2 SQLite）[SPEC-DEFER:phase-future.l1-cache-access-order-lru]。
- `with_sqlite` 接入生产 daemon 调用点（当前出厂走 memory-only L1）[SPEC-DEFER:phase-future.l2-cache-production-wire]——本 task 只补 opt-in 路径的访问序 LRU，不改 daemon 装配。
- L2 命中 bump 写放大的优化（如批量 bump / 计数阈值 bump）——本 task 命中即 bump（语义正确优先），优化延后 [SPEC-DEFER:phase-future.l2-lru-bump-batching]。
- 真实 release tag / run-id / digest（v0.33.0）[SPEC-OWNER:task-40.3-closeout]（ADR-012 用户授权后回填）。

## 4. Actors

- 主 agent（ADR-012 自治）
- `sqlite_get`（`core/src/embedding/cache.rs:140-150`，L2 命中读，本 task 在命中分支加 rowid bump）
- `sqlite_put`（`core/src/embedding/cache.rs:153-195`，L2 写 + rowid 序驱逐，本 task 不改其 SQL——rowid 由插入序变访问序后自动升 LRU）
- `embedding_cache` SQLite 表（:110-120 CREATE，隐式 rowid 作访问序键）+ `l2_cap` 字段（task-33.1 已存入）
- 运维 / 部署者（启用 L2 持久化路径时受益于访问序 LRU——热行不被插入序 FIFO 误逐）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/embedding/cache.rs:140-150`（`sqlite_get` L2 命中读——本 task 在命中分支加「仅有限 cap 时 bump 命中行 rowid」）
- `core/src/embedding/cache.rs:153-195`（`sqlite_put` L2 写 + rowid 序驱逐——:185-187 `DELETE ... WHERE rowid NOT IN (SELECT rowid ... ORDER BY rowid DESC LIMIT cap)`，本 task 不改；rowid 含义由插入序变访问序后自动升 LRU）+ `:117`（`DEFAULT_L2_EMBEDDING_CACHE_CAP`）+ `l2_cap` 字段（task-33.1 已存）
- `core/src/embedding/cache.rs:110-120`（`CREATE TABLE embedding_cache` 列 `(content_hash,provider,dim,vector)` + PK `(content_hash,provider)`，非 WITHOUT ROWID → 隐式 rowid，本 task 复用，不改）+ `:402`（TEST-33.1.1 L2 cap=2 rowid-FIFO 驱逐——本 task L2 LRU test 镜像形态 + 对比）
- `docs/decisions/adr-038-*.md §D1 + §A2/D4`（Phase 33 rowid-FIFO + 真-LRU 据「须加时间列」延后——本 task 据实更正该假设）+ `docs/decisions/adr-027-*.md`（embedding L2 有界化前序）+ `docs/decisions/adr-045-governance-debt-cleanup-3.md §D2`（本 task 即其原文实现）

### 5.2 关键设计 — 命中 bump 隐式 rowid（0 dep / 0 migration / 默认行为不变）

- **B2/B3 命中 bump 访问序 LRU**：`sqlite_get`（:140-150）命中（读到 vector）后、返回前，于同一 `&Connection`（同锁内）：仅当 `l2_cap > 0` 时 `INSERT OR REPLACE INTO embedding_cache (content_hash, provider, dim, vector) VALUES (?1,?2,?3,?4)` 原样回写命中行（值不变）——`INSERT OR REPLACE` 对既有 PK 行先 DELETE 再 INSERT → 命中行获**新**（最大）rowid → 隐式 rowid 由插入序变访问序。既有 `sqlite_put`（:153-195）的 `DELETE ... ORDER BY rowid DESC LIMIT cap`（保最大 cap 个 rowid）随之由 FIFO 升 LRU（驱逐最久未**用** = 最小 rowid = 最久未 bump 行）。pass bar：cap=2，put a,b → get a（命中 bump，a 获新 rowid > b）→ put c（超 cap，驱逐最小 rowid = b）→ get b miss / get a hit；FIFO 旧行为（命中不 bump）会驱逐 a。
- **cap==0 不 bump**：`l2_cap == 0`（不限，镜像 L1 `BoundedCache` `cap==0` 语义）时 `sqlite_get` 命中不 bump（保插入序、零额外写）——不限容量下无驱逐、LRU 序无意义。
- **B3 据实更正 Phase 33 假设**：Phase 33（ADR-038 A2/D4）把真-LRU 据「须加 created_at 列 + ALTER 既有文件」延后；本 task grounding 更正——命中 bump 隐式 rowid 即得访问序 LRU，**不须时间列、0 schema migration**，与 Go memstore move-to-front（task-33.2）同技法。Phase 33 D1 的 rowid-FIFO（row-count cap 本身）是正确且必要的前序，本 task 只补「命中 bump 使 rowid 序由插入序变访问序」增量——以 add-only Amendment 记于 ADR-038/027，不溯改其正文（ADR-014 D5）。
- **B4 写放大 + opt-in-path 据实**：命中 bump 给 L2 读路径加一次行重写（写放大），是访问序 LRU 的固有代价（同 Go memstore 命中 move-to-front 的内部代价）；且 `with_sqlite` 无生产调用点（Phase 33 D1 已据实标注 opt-in-path）→ 现网零影响，本项是 opt-in 路径语义补全、非已确认线上问题（spec / ADR-045 D2 据实记，ADR-013 不夸大）。
- **同锁内 bump 不破并发约束**：`sqlite_get` 在 `embed` 内持 `store.lock()` 的 `&Connection` 调用，命中 bump 的 `INSERT OR REPLACE` 与既有读共用同一连接 / 同锁，无新增锁、无新增并发面。

### 5.3 不变量

- 默认行为不变（ADR-004）：未触 L2 行数上界（行数 ≤ cap）时，既有 L2 round-trip 命中行为（TEST-22.2.3）的**返回结果**与改前一致（命中 bump 只改 rowid、不改返回 vector）；`cap==0` 不 bump、行为同 Phase 33 FIFO 基线；`embedding_cache` 表 schema（列 / 主键 / 无 WITHOUT ROWID）不变；L1 行为（既有 cap / FIFO）不动。
- 既有契约不变：公共构造（`new` / `with_sqlite` / `with_sqlite_capacity` / `with_capacity`）签名 / 语义不变（本 task 不动构造，仅改 `sqlite_get` 内部）；`EmbeddingProvider` trait `embed` / `dim` / `name` 行为不变；`sqlite_put` rowid 序驱逐 SQL 不改。
- 0 新代码依赖（ADR-008）：仅既有 `rusqlite` `INSERT OR REPLACE`，无 Cargo 依赖增量。
- 0 schema migration（ADR-004 既有契约不变）：复用 `embedding_cache` 隐式 rowid，不加 `created_at` / `last_access` 列、不 ALTER、不新增编号 migration。
- L2 访问序 LRU：有限 cap 下命中行经 bump 获最新 rowid，既有 rowid 序驱逐由 FIFO 升 LRU（驱逐最久未用）；cap==0 不限、不 bump、保插入序。
- opt-in-path / 写放大诚实边界（ADR-013）：命中 bump 写放大是访问序 LRU 固有代价；`with_sqlite` 无生产调用点 → opt-in 路径语义补全、非已确认线上问题，据实声明不夸大。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [x] **AC1**（L2 命中 bump → 访问序 LRU 🟢）: `sqlite_get`（:140-150）命中时（仅 `l2_cap > 0`）`INSERT OR REPLACE` 原样回写命中行 bump 其隐式 rowid 到表尾，使 `sqlite_put`（:153-195）rowid 序驱逐由插入序 FIFO 升访问序 LRU；cap=2 put a,b → get a（bump）→ put c → 驱逐 b（最久未用，重读 miss）、保留 a（重读 hit），对比 FIFO 旧行为驱逐 a；`cap==0` 不 bump（保插入序、零额外写）；复用既有隐式 rowid、**0 新 dep + 0 schema migration** — verified by **TEST-40.2.1**
- [x] **AC2**（默认行为不变 + 既有基线不退化 🟢）: 默认 cap 下既有 L2 round-trip（TEST-22.2.3）返回结果不变（命中 bump 只改 rowid 不改 vector）；L1 cap（TEST-31.2.1）+ L2 rowid cap（TEST-33.1.1）+ 既有 TEST-22.2.* 不退化；公共构造源码兼容；`embedding_cache` schema + L1 行为 + 既有契约不变（ADR-004）+ 0 新 dep（ADR-008）；命中 bump 写放大 + L2 无生产调用点现网零影响据实记（opt-in-path 语义补全，ADR-013） — verified by **TEST-40.2.2**
- [x] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-40.2.3**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-40.2.1 | L2 命中 bump 访问序 LRU：`with_sqlite_capacity` cap=2 put a,b → get a（命中 bump rowid）→ put c → L2 行数 ≤ 2 + 驱逐 b（最久未用，新 provider 同文件重读 = miss，inner 重算）、保留 a（重读 hit，inner 不调）；对比 FIFO（命中不 bump 会驱逐 a）；`cap==0` 不 bump（保插入序、零额外写）；0 新 dep + 0 schema migration（隐式 rowid） | `core/src/embedding/cache.rs`（同源 test） | Done |
| TEST-40.2.2 | 默认行为不变：默认 cap 下既有 L2 round-trip（TEST-22.2.3）返回结果不变 + L1 cap（TEST-31.2.1）+ L2 rowid cap（TEST-33.1.1）+ TEST-22.2.* 不退化；公共构造源码兼容；`embedding_cache` schema 不变 + 0 新 dep | `core/src/embedding/cache.rs` | Done |
| TEST-40.2.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（中）命中 bump 误改驱逐语义 / 默认结果回归**：`sqlite_get` 命中加 `INSERT OR REPLACE` bump 若 SQL 或条件有偏，会误驱逐或改返回结果。
  - **缓解**：命中 bump 原样回写命中行（值不变、仅 rowid 变）；既有 `sqlite_put` 驱逐 SQL 不改；TEST-40.2.1 断言命中 bump 后驱逐最久未用（对比 FIFO）、TEST-40.2.2 断言默认 round-trip 返回结果不变。stop-condition：LRU 命中重排不生效 / 默认返回结果回归则 AC1/AC2 不标 `[x]`。
- **R2（中）cap==0 仍 bump 引入无谓写放大**：不限容量下命中 bump 是纯额外写（无驱逐 → LRU 序无意义）。
  - **缓解**：仅 `l2_cap > 0` 时 bump（cap==0 不 bump，保插入序、零额外写）；TEST-40.2.1 断言 cap==0 不 bump。stop-condition：cap==0 引入写放大则 AC1 不标 `[x]`。
- **R3（中）INSERT OR REPLACE bump 的 rowid 语义正确性**：`INSERT OR REPLACE` 对既有 PK 行先 DELETE 再 INSERT → 命中行获新（最大）rowid，须确保这令 `sqlite_put` 的 `ORDER BY rowid DESC LIMIT cap`（保最大 cap）驱逐的确是最久未 bump 行。
  - **缓解**：测试用互异文本（各独立 rowid 单调递增），命中 a 后 a 获最大 rowid → put c 后最小 rowid 是 b → 驱逐 b；TEST-40.2.1 断言此序。stop-condition：命中 bump 后驱逐序非访问序 LRU 则 AC1 不标 `[x]`。
- **R4（低）写放大被误读为线上回归**：命中即重写给读路径加写 I/O，易被夸大为性能回归。
  - **缓解**：spec §1 B4 / §5.2 B4 / §5.3 + ADR-045 D2 据实记「访问序 LRU 固有代价（同 Go memstore move-to-front）+ L2 无生产调用点（Phase 33 已标 opt-in-path）现网零影响」（ADR-013 不夸大）。stop-condition：若把 opt-in-path 语义补全夸大为线上修复则越界。
- **R5（低）Phase 33 假设更正被误读为否定前序**：本 task 更正 Phase 33「真 LRU 须加时间列」假设，易被误读为 Phase 33 D1 有错。
  - **缓解**：spec §1 B3 / §5.2 B3 + ADR-045 D2 明记 Phase 33 D1 的 rowid-FIFO（row-count cap）是正确且必要前序，本 task 只补命中 bump 增量；以 add-only Amendment 记于 ADR-038/027，不溯改正文（ADR-014 D5）。stop-condition：若溯改 Phase 33 ADR 正文则违 ADR-014 D5。

## 9. Verification Plan

```bash
# 1. AC1 — L2 命中 bump 访问序 LRU（确定性单测：cap=2 put a,b → get a bump → put c → 驱逐 b）
cargo test -p contextforge-core embedding::cache

# 2. AC2 — 默认行为不变（既有 L2 round-trip TEST-22.2.3 + L1 cap TEST-31.2.1 + L2 rowid cap TEST-33.1.1 全绿）
cargo test -p contextforge-core embedding::cache

# 3. 不退化（全量）
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# 4. AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.l2-cache-true-lru-defer-note]：本 task 交付 L2 SQLite 访问序 LRU（命中即 bump 隐式 rowid，仅有限 cap），🟢 可单测，0 新 dep + 0 schema migration（复用隐式 rowid——据实更正 Phase 33「真 LRU 须加时间列」假设）；L2 TTL 时间过期 [SPEC-DEFER:phase-future.l2-cache-ttl]、L1 访问序 LRU [SPEC-DEFER:phase-future.l1-cache-access-order-lru]、`with_sqlite` 接入生产 daemon [SPEC-DEFER:phase-future.l2-cache-production-wire]、命中 bump 写放大优化 [SPEC-DEFER:phase-future.l2-lru-bump-batching] 均不在本 task 范围。命中 bump 写放大是访问序 LRU 固有代价；`with_sqlite` 无生产调用点 → 本 task 为 opt-in 路径语义补全、**非已确认线上问题**（据实声明，ADR-013 不夸大）；实测产物（v0.33.0）真实跑出后回填。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done

**§9 Verification 计划** (will record real evidence at impl)：
- AC1：`cargo test -p contextforge-core embedding::cache` —— cap=2 put a,b → get a（命中 bump rowid）→ put c → L2 行数 ≤ 2 + 驱逐 b（最久未用，重读 miss inner 重算）、保留 a（重读 hit inner 不调），对比 FIFO 旧行为驱逐 a；cap==0 不 bump；复用隐式 rowid、0 新 dep + 0 schema migration（真实结果待实施回填，ADR-013 不伪造）。
- AC2：`cargo test -p contextforge-core embedding::cache` —— 默认 cap 下既有 L2 round-trip（TEST-22.2.3）返回结果不变 + L1 cap（TEST-31.2.1）+ L2 rowid cap（TEST-33.1.1）+ 既有 TEST-22.2.* 不退化；公共构造源码兼容；`embedding_cache` schema 不变；0 新 dep（ADR-008）。真实结果待实施回填。
- AC3：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威）。
- 0 新 dep / 0 schema migration / 默认行为不变 / 既有契约不变 / 写放大 + opt-in-path 据实 真实结果待实施回填（ADR-013 不预填）。

**实际改动文件**（计划，待实施回填）：
- `core/src/embedding/cache.rs`——`sqlite_get`（:140-150）命中分支加「仅 `l2_cap > 0` 时 `INSERT OR REPLACE` 原样回写命中行 bump 隐式 rowid 到表尾」；既有 `sqlite_put`（:153-195）rowid 序驱逐 SQL 不改（rowid 由插入序变访问序后自动升 LRU）。+ 同源 test（镜像 TEST-33.1.1 形态 + SQLite 文件 round-trip：cap=2 put a,b → get a bump → put c → 驱逐 b）。`CREATE TABLE`（:110-120）不改（复用隐式 rowid，0 schema migration）。
- `docs/decisions/adr-038-*.md` + `adr-027-*.md` L2 true-LRU 维度兑现 + 真-LRU 假设据实更正 add-only Amendment 落点在 task-40.3 closeout（非本 task body）。
