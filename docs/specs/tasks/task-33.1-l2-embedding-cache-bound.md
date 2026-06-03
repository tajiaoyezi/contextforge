# Task `33.1`: `l2-embedding-cache-bound — cache.rs L2 SQLite embedding_cache 行数上界 + rowid-FIFO 驱逐（替代无上界 INSERT OR REPLACE 只增长）；0 新 dep / 0 schema migration（用隐式 rowid）；默认行为不变（ADR-004） + opt-in-path 诚实 caveat（with_sqlite 无生产调用点）`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 33 (governance-debt-cleanup-2)
**Dependencies**: 既有 `core/src/embedding/cache.rs`（task-22.2 `CachingEmbeddingProvider` L2 SQLite 持久化，Phase 22 已交付；task-31.2 L1 `BoundedCache` FIFO 上界，Phase 31 已交付，ADR-036 D2）/ ADR-027（embedding-provider-completion，L2 cache bound 为 add-only Amendment @ task-33.4 closeout）/ ADR-004（local-first-privacy-baseline，默认行为 + 既有契约 + 公共构造签名不变）/ ADR-002（layered storage，`embedding_cache` 表为 add-only L2 持久化，本 task 不改 schema）/ ADR-008（dep add-only，Phase 33 = 0 新 dep）/ ADR-013（禁伪造红线——opt-in-path caveat 据实声明，不夸大为已确认线上泄漏）/ ADR-012（main-agent-governance-autonomy）/ ADR-014 D1-D5（第二十四次激活）

## 1. Background

Phase 31（ADR-036 D2）已为 L1 内存缓存加容量上界（`BoundedCache`，map + `VecDeque` 插入序 FIFO 驱逐，0 新 dep），但 **L2 SQLite 持久化层仍无上界**——本 task 聚焦把 L2 行数 bound 起来，与 L1 对齐：

- **B1 L2 SQLite 无行数上界**：`core/src/embedding/cache.rs` 的 `sqlite_put`（`:153-161`）以 `INSERT OR REPLACE INTO embedding_cache`（`:155`）写直通，无任何行数 cap——同一 `(content_hash, provider)` 主键复写（更新）不增行，但**每个互异文本 → 新行**，行数随唯一文本数线性只增长。长跑 daemon 经 L2 持久化路径对大量不同文本 embed 会令 SQLite 文件无界增大（与 Phase 31 修前 L1 同性质，但落在持久层）。
- **B2 表为 CREATE TABLE IF NOT EXISTS，非编号 migration**：`embedding_cache` 表经 `with_sqlite` 构造期 `CREATE TABLE IF NOT EXISTS`（`:110-120`）建立，列为 `(content_hash, provider, dim, vector)`，主键 `PRIMARY KEY (content_hash, provider)`（`:116`），**非** WITHOUT ROWID 表 → 持有隐式 `rowid`（单调递增 = 插入序）。因此本 task 可直接用隐式 `rowid` 做 FIFO 驱逐键，**不需新增 `created_at` 列、不需 schema migration**（加显式时间列须对既有用户文件 `ALTER`，true-LRU 据访问时间排序须该列 → 该路径 honest-defer [SPEC-DEFER:phase-future.l2-cache-true-lru]）。
- **B3 L1 已有 cap 常量 + 对齐基线**：L1 默认 cap 常量 `DEFAULT_EMBEDDING_CACHE_CAP`（`:23` = 50_000），驱逐策略 FIFO（`BoundedCache::insert` 超 cap pop `order` front）。L2 cap 镜像该风格——L2 默认 cap 常量与 `DEFAULT_EMBEDDING_CACHE_CAP` 并列声明，驱逐策略 rowid-FIFO（与 L1 FIFO 同语义，确定性强）。
- **B4 opt-in-path 诚实 caveat（核实后据实声明，非夸大）**：经核 `with_sqlite`（`:105-126`）**在生产代码无任何调用点**——仅测试调用（`cache.rs:331` / `:337`，TEST-22.2.3 round-trip）；出厂 daemon 走 memory-only L1（`new` → `with_capacity`，`store: None`）。故本 task 的 L2 bound 是 **opt-in 持久化路径的纵深防御（defense-in-depth）**，**不是已确认的线上内存/磁盘泄漏**（据 ADR-013 据实声明，不夸大）。

经核 task-31.2 已为 L1 建立 cap 上界测试基线（`cache.rs:345` TEST-31.2.1 cap=2 FIFO 驱逐），本 task L2 bound 镜像该测试形态（带 SQLite 文件 round-trip），为 code-local 🟢 可单测，0 新 dep（仅既有 `rusqlite` `COUNT(*)` + `DELETE`）+ 0 schema migration（用隐式 rowid）。

## 2. Goal

(1) **B1/B2**：为 L2 SQLite `embedding_cache` 加行数上界 + rowid-FIFO 驱逐——`sqlite_put`（`:153-161`）写直通后做 `COUNT(*)`；若超 L2 cap，`DELETE FROM embedding_cache WHERE rowid IN (SELECT rowid FROM embedding_cache ORDER BY rowid ASC LIMIT <overflow>)` 逐出最旧（最小 rowid = 最早插入）行。**0 新 dep + 0 schema migration**（用隐式 rowid，不加 `created_at` 列、不 `ALTER` 既有文件）。(2) **B3**：L2 默认 cap 取合理常量（与 `DEFAULT_EMBEDDING_CACHE_CAP` `:23` 并列声明，命名如 `DEFAULT_L2_EMBEDDING_CACHE_CAP`），不破现有 L2 round-trip 命中行为。(3) **B4**：opt-in-path caveat 据实记入 spec / ADR-038 D1（`with_sqlite` 无生产调用点 → 纵深防御非已确认泄漏，ADR-013 不夸大）。

pass bar：L2 行数上界 + rowid-FIFO 驱逐经确定性单测验证（cap=2，写 3 个互异文本 → L2 行数 ≤ 2 + 最旧文本被逐出 → 重读为 miss）（🟢）；默认 cap 不破既有 L2 round-trip（TEST-22.2.3）+ L1 既有 cap 基线（TEST-31.2.1 / TEST-22.2.*）不退化（🟢）；公共构造（`new` / `with_sqlite` / `with_capacity`）源码兼容、0 新 dep（ADR-008）、0 schema migration（ADR-004 既有契约不变）；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `core/src/embedding/cache.rs`——`sqlite_put`（`:153-161`）在 `INSERT OR REPLACE`（`:155`）成功后执行 rowid-FIFO 驱逐：`SELECT COUNT(*) FROM embedding_cache`（scoped 全表行数）；若 `count > l2_cap` 则 `DELETE FROM embedding_cache WHERE rowid IN (SELECT rowid FROM embedding_cache ORDER BY rowid ASC LIMIT (count - l2_cap))` 逐出 `overflow` 个最旧 rowid 行。`l2_cap` 取 L2 默认 cap 常量（`cap == 0` ⇒ 不限，与 L1 `BoundedCache` `cap==0` 语义一致）。
- 加 L2 默认 cap 常量——与 `DEFAULT_EMBEDDING_CACHE_CAP`（`:23`）并列声明一个 `DEFAULT_L2_EMBEDDING_CACHE_CAP`（合理值，doc 注释记「L2 SQLite 行数上界，rowid-FIFO 驱逐」）；`with_sqlite`（`:105-126`）构造时把该 cap 存入字段（add-only 私有字段，公共签名不变），`sqlite_put` 读之。
- `CREATE TABLE`（`:110-120`）不改——`embedding_cache` 表列 / 主键不动；本 task 用其隐式 `rowid`（非 WITHOUT ROWID 表本就持有），不加 `created_at` 列、不 `ALTER`、不新增编号 migration（0 schema migration）。
- 同源测试：`cache.rs` 同源 test（镜像 TEST-31.2.1 形态 + SQLite 文件 round-trip）断言 L2 行数 ≤ cap + 最旧文本被逐出（重读 miss）+ 仍在 L2 内文本命中（重读 hit）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- L2 真 access-order LRU（据真实访问时间排序驱逐，须加 `created_at` / `last_access` 列 + 对既有用户文件 `ALTER`）[SPEC-DEFER:phase-future.l2-cache-true-lru]——本 task 用隐式 rowid 行 FIFO（= 插入序，0 migration），真 LRU 须 schema 改动延后。
- L2 SQLite 缓存 TTL / 主动时间过期 [SPEC-DEFER:phase-future.l2-cache-ttl]（task-31.2 §3 已记，本 task 仅加行数上界，时间维度过期延后）。
- L1 内存缓存上界（task-31.2 已交付 `BoundedCache` FIFO）——本 task 范围限于补 L2 行数上界，L1 沿用既有 cap，不动。
- embedding-cache 跨进程 / 分布式共享（Redis 等外部缓存）[SPEC-DEFER:phase-future.distributed-embedding-cache]（task-31.2 §3 已记）。
- 把 `with_sqlite` 接入生产 daemon 调用点（当前出厂走 memory-only L1）[SPEC-DEFER:phase-future.l2-cache-production-wire]——本 task 仅 bound opt-in 路径，不改 daemon 装配。
- 真实 release tag / run-id / digest（v0.26.0）[SPEC-OWNER:task-33.4-closeout]（ADR-012 用户授权后回填）。

## 4. Actors

- 主 agent（ADR-012 自治）
- `CachingEmbeddingProvider`（`core/src/embedding/cache.rs:69-73`，持 `store: Option<Mutex<Connection>>`，本 task 加 L2 cap 字段 + 驱逐）
- `sqlite_put`（`core/src/embedding/cache.rs:153-161`，L2 写直通点，本 task 在其后加 rowid-FIFO 驱逐）
- `with_sqlite`（`core/src/embedding/cache.rs:105-126`，L2 opt-in 构造，本 task 存入 L2 cap；无生产调用点 = opt-in-path caveat）
- `embedding_cache` SQLite 表（`:110-120` CREATE，隐式 rowid 作 FIFO 键）
- 运维 / 部署者（启用 L2 持久化路径时受益于有界磁盘占用）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/embedding/cache.rs:153-161`（`sqlite_put` L2 写直通——`:155` `INSERT OR REPLACE INTO embedding_cache`，本 task 在其后加 rowid-FIFO 驱逐）
- `core/src/embedding/cache.rs:105-126`（`with_sqlite` L2 opt-in 构造——`:110-120` `CREATE TABLE IF NOT EXISTS embedding_cache` 列 `(content_hash,provider,dim,vector)` + `:116` `PRIMARY KEY (content_hash, provider)`，非 WITHOUT ROWID → 隐式 rowid）
- `core/src/embedding/cache.rs:23`（`DEFAULT_EMBEDDING_CACHE_CAP = 50_000` L1 默认 cap 常量——L2 cap 常量并列声明的对齐点）+ `:27-66`（`BoundedCache` FIFO `insert` `:42-60`，`cap == 0` ⇒ 不限 `:48`——L2 cap 语义镜像源）
- `core/src/embedding/cache.rs:140-150`（`sqlite_get` scoped 读，`(provider, dim)` 作用域）+ `:331` / `:337`（`with_sqlite` 仅测试调用点 = opt-in-path caveat 证据）+ `:345-363`（TEST-31.2.1 L1 cap=2 FIFO 驱逐——本 task L2 test 镜像形态）
- `docs/decisions/adr-027-*.md`（embedding-provider-completion；L2 cache bound 为 add-only Amendment 落点 @ task-33.4 closeout）+ `docs/decisions/adr-038-governance-debt-cleanup-2.md §D1`（本 task 即其原文实现）+ `docs/decisions/adr-036-governance-debt-cleanup.md §D2`（L1 cap 前序，本 task L2 对齐）

### 5.2 关键设计 — L2 行数上界 + rowid-FIFO（0 dep / 0 migration / 默认行为不变）

- **B1/B2 rowid-FIFO 驱逐**：`sqlite_put`（`:153-161`）在 `INSERT OR REPLACE`（`:155`）后于同一 `&Connection`（同锁内）执行：(a) `SELECT COUNT(*) FROM embedding_cache`；(b) `if count > l2_cap`，`DELETE FROM embedding_cache WHERE rowid IN (SELECT rowid FROM embedding_cache ORDER BY rowid ASC LIMIT (count - l2_cap))` 逐出 `overflow = count - l2_cap` 个最小 rowid（= 最早插入）行。`l2_cap == 0` ⇒ 跳过驱逐（不限，镜像 L1 `BoundedCache` `cap==0` 语义 `:48`）。pass bar 测试：`with_sqlite` cap=2，连续 embed 3 个互异文本 → L2 行数 ≤ 2 + 最旧文本（最小 rowid）被逐 → 对其重 embed（经新 provider 同文件，inner 计数增 = miss）；对仍在 L2 内文本 embed → 命中（inner 不增）。SQL 经既有 `rusqlite`，0 新 dep；用隐式 rowid，0 schema migration。
- **B3 L2 cap 常量对齐 L1 风格**：与 `DEFAULT_EMBEDDING_CACHE_CAP`（`:23`）并列声明 `DEFAULT_L2_EMBEDDING_CACHE_CAP`（合理值，pub const，doc 记 rowid-FIFO 语义）；`with_sqlite`（`:105-126`）构造存入 add-only 私有字段（如 `l2_cap: usize`），公共构造签名（`new` / `with_sqlite` / `with_capacity`）**不变**（默认值经常量注入，调用方源码兼容）。`sqlite_put` 读该字段做驱逐阈值。
- **B4 opt-in-path caveat 据实**：`with_sqlite` 经核仅测试调用（`:331` / `:337`），生产 daemon 走 memory-only L1（`new` → `store: None`）→ 本 L2 bound 是 opt-in 持久化路径的纵深防御（defense-in-depth），**非已确认线上泄漏**；spec / ADR-038 D1 据实记此 caveat（ADR-013 不夸大为已确认泄漏）。
- **驱逐镜像既有 L1 风格**：rowid-FIFO（最旧插入先逐）与 L1 `BoundedCache` FIFO（`order` front pop `:52`）同语义，确定性强；不引 access-order LRU（须时间列 + `ALTER` → [SPEC-DEFER:phase-future.l2-cache-true-lru]）。
- **同锁内驱逐不破并发约束**：`sqlite_put` 在 `embed` 内持 `store.lock()` 的 `&Connection`（`:226-229`）调用，`COUNT(*)` + `DELETE` 与 `INSERT OR REPLACE` 共用同一连接 / 同锁，无新增锁、无新增并发面。

### 5.3 不变量

- 默认行为不变（ADR-004）：未触 L2 行数上界（行数 ≤ L2 cap）时，既有 L2 round-trip 命中行为（TEST-22.2.3）与改前一致；`embedding_cache` 表 schema（列 / 主键 / 无 WITHOUT ROWID）不变；L1 行为（既有 cap / FIFO）不动。
- 既有契约不变：公共构造（`new` `:89` / `with_sqlite` `:105` / `with_capacity` `:95`）签名兼容（L2 cap 经默认常量注入 add-only 私有字段，调用方源码不破）；`EmbeddingProvider` trait `embed` / `dim` / `name` 行为不变；`sqlite_get`（`:140-150`）scoped 读语义不动。
- 0 新代码依赖（ADR-008）：仅既有 `rusqlite` `COUNT(*)` + `DELETE` SQL，无 Cargo 依赖增量。
- 0 schema migration（ADR-004 既有契约不变）：用 `embedding_cache` 隐式 rowid（非 WITHOUT ROWID 表本有），不加列、不 `ALTER`、不新增编号 migration；真 access-order LRU（须时间列）→ [SPEC-DEFER:phase-future.l2-cache-true-lru]。
- L2 磁盘有界：bound 后任意 embed 序列下 L2 行数 ≤ L2 cap（`cap == 0` 除外，显式不限）；实测磁盘上界数值不预填（ADR-013，真实跑出才记）。
- opt-in-path 诚实边界（ADR-013）：`with_sqlite` 无生产调用点 → 本 task 为纵深防御，不夸大为已确认线上泄漏；据实声明于 spec / ADR-038 D1。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [ ] **AC1**（L2 SQLite rowid-FIFO 行数上界 🟢）: `sqlite_put`（`:153-161`）在 `INSERT OR REPLACE`（`:155`）后做 `COUNT(*)` + 超 cap 时 `DELETE ... WHERE rowid IN (SELECT rowid ORDER BY rowid ASC LIMIT overflow)` 逐出最旧 rowid 行；`with_sqlite` cap=2 写 3 个互异文本 → L2 行数 ≤ 2 + 最旧文本被逐（新 provider 同文件重读 = miss，inner 重算）、仍在 L2 内文本命中（inner 不调）；`cap==0` ⇒ 不限。L2 cap 常量与 `DEFAULT_EMBEDDING_CACHE_CAP`（`:23`）并列声明；**0 新 dep + 0 schema migration**（隐式 rowid，不加列 / 不 `ALTER`） — verified by **TEST-33.1.1**
- [ ] **AC2**（默认行为不变 + 既有基线不退化 🟢）: 默认 L2 cap 不破既有 L2 round-trip（TEST-22.2.3 全绿）；L1 既有 cap 基线（TEST-31.2.1）+ 既有 TEST-22.2.* 不退化；公共构造（`new` / `with_sqlite` / `with_capacity`）源码兼容；`embedding_cache` schema + L1 行为 + proto / 既有契约不变（ADR-004） + 0 新 dep（ADR-008） — verified by **TEST-33.1.2**
- [ ] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-33.1.3**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-33.1.1 | L2 rowid-FIFO bound：`with_sqlite` cap=2 写 3 互异文本 → L2 `COUNT(*)` ≤ 2 + 最旧 rowid 行被逐（新 provider 同文件重读 = miss，inner 重算），仍在 L2 内文本命中（inner 不调）；`cap==0` ⇒ 不限；0 新 dep + 0 schema migration（隐式 rowid） | `core/src/embedding/cache.rs`（同源 test） | Planned |
| TEST-33.1.2 | 默认行为不变：默认 L2 cap 下既有 L2 round-trip（TEST-22.2.3）+ L1 cap（TEST-31.2.1）+ TEST-22.2.* 不退化；公共构造源码兼容；`embedding_cache` schema 不变 + 0 新 dep | `core/src/embedding/cache.rs` | Planned |
| TEST-33.1.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（中）rowid 作 FIFO 键的正确性（INSERT OR REPLACE 复写是否改 rowid）**：`INSERT OR REPLACE` 对既有主键行先 DELETE 再 INSERT，复写行会获**新** rowid（移到表尾），这与 FIFO「最旧插入先逐」语义一致（复写 = 重新写入 = 视作新近），但须确保 `ORDER BY rowid ASC` 逐出的确是最早未复写的行。
  - **缓解**：测试用互异文本（无主键冲突 → 各自独立 rowid 单调递增），断言最早写入的互异文本被逐；复写场景（同文本重 embed）本就命中 L1/L2 不触发新行，无 rowid 漂移影响 cap。stop-condition：cap=2 写 3 互异 → 最旧被逐单测不过则 AC1 不标 `[x]`。
- **R2（中）COUNT(*) + DELETE 每次 put 的开销**：每次 L2 写后 `COUNT(*)` 全表扫 + 条件 DELETE，高频 put 下有开销。
  - **缓解**：`COUNT(*)` 在有 rowid 的小表上 O(行数) 但常量小；仅超 cap 才 DELETE（稳态下 count==cap 时每 put 逐 1 行）；与 L1 FIFO 每插一次维护 `order` 同量级。若实测开销显著可改 `changes()` / 计数缓存优化（属优化，非本 task 范围）。stop-condition：功能正确性优先，开销实测数值不预填（ADR-013）。
- **R3（低）默认 L2 cap 取值破既有命中**：L2 默认 cap 过小致原本 round-trip 命中的文本被提前逐出。
  - **缓解**：L2 默认 cap 取合理值（与 L1 `DEFAULT_EMBEDDING_CACHE_CAP=50_000` 同量级或据持久层特性定，不破 TEST-22.2.3 单文本 round-trip）；AC2 断言既有 round-trip 全绿。
- **R4（低）opt-in-path caveat 被误读为已确认泄漏**：`with_sqlite` 无生产调用点，易被夸大为线上 bug。
  - **缓解**：spec §1 B4 / §5.2 B4 / §5.3 + ADR-038 D1 据实记「opt-in 纵深防御，非已确认泄漏」（ADR-013 不夸大）；本 task 价值在 opt-in 路径被启用时的有界保证。
- **R5（低）true-LRU 期望与 FIFO 实现一致性**：本 task 是 rowid-FIFO（插入序）非访问序 LRU，易被误读为已实现真 LRU。
  - **缓解**：spec / ADR 明记 rowid-FIFO（插入序），真 access-order LRU 须时间列 + `ALTER` → [SPEC-DEFER:phase-future.l2-cache-true-lru]；本 task 范围内不引时间列、0 schema migration。

## 9. Verification Plan

```bash
# 1. AC1 — L2 rowid-FIFO bound（确定性单测：cap=2 写 3 互异 → 行数 ≤ 2 + 最旧逐出 miss 重算）
cargo test -p contextforge-core embedding::cache

# 2. AC2 — 默认行为不变（既有 L2 round-trip TEST-22.2.3 + L1 cap TEST-31.2.1 + TEST-22.2.* 全绿）
cargo test -p contextforge-core embedding::cache

# 3. 不退化（全量）
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# 4. AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.l2-cache-bound-defer-note]：本 task 仅交付 L2 SQLite 行数上界 + rowid-FIFO 驱逐（🟢 可单测，0 新 dep + 0 schema migration，用隐式 rowid）；L2 真 access-order LRU（须时间列 + `ALTER` 既有文件）[SPEC-DEFER:phase-future.l2-cache-true-lru]、L2 TTL 时间过期 [SPEC-DEFER:phase-future.l2-cache-ttl]、`with_sqlite` 接入生产 daemon [SPEC-DEFER:phase-future.l2-cache-production-wire] 均不在本 task 范围。`with_sqlite` 无生产调用点 → 本 task 为 opt-in 路径纵深防御，**非已确认线上泄漏**（据实声明，ADR-013 不夸大）；实测磁盘上界数值不预填，真实跑出后回填。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Draft

**§9 Verification 计划** (will record real evidence at impl)：
- AC1：`cargo test -p contextforge-core embedding::cache` —— `with_sqlite` cap=2 写 3 个互异文本 → L2 `COUNT(*)` ≤ 2 + 最旧 rowid 行被逐（新 provider 同文件重读 = miss，inner 重算），仍在 L2 内文本命中（inner 不调）；`cap==0` ⇒ 不限；rowid-FIFO 用隐式 rowid，0 新 dep + 0 schema migration（真实测试结果待实施回填，ADR-013 不伪造）。
- AC2：`cargo test -p contextforge-core embedding::cache` —— 默认 L2 cap 下既有 L2 round-trip（TEST-22.2.3）+ L1 cap（TEST-31.2.1）+ 既有 TEST-22.2.* 不退化；公共构造（`new` / `with_sqlite` / `with_capacity`）源码兼容；`embedding_cache` schema + L1 行为不变；0 新 dep（ADR-008）。真实结果待实施回填。
- AC3：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威）。
- 0 新 dep / 0 schema migration / 默认行为不变 / 既有契约不变 / opt-in-path caveat 据实 真实结果待实施回填（ADR-013 数值不预填，真实跑出才记数）。

**实际改动文件**（计划，待实施回填）：
- `core/src/embedding/cache.rs`——`sqlite_put`（`:153-161`）`INSERT OR REPLACE`（`:155`）后加 `COUNT(*)` + 超 cap 时 rowid-FIFO `DELETE`（`ORDER BY rowid ASC LIMIT overflow`）；加 L2 默认 cap 常量（与 `DEFAULT_EMBEDDING_CACHE_CAP` `:23` 并列）+ `with_sqlite`（`:105-126`）存入 add-only 私有字段（公共构造签名不变）。+ 同源 test（镜像 TEST-31.2.1 形态 + SQLite 文件 round-trip：cap=2 写 3 互异 → 行数 ≤ 2 + 最旧逐出）。`CREATE TABLE`（`:110-120`）不改（用隐式 rowid，0 schema migration）。
- `docs/decisions/adr-027-*.md` L2 cache bound add-only Amendment 落点在 task-33.4 closeout（非本 task body）。
