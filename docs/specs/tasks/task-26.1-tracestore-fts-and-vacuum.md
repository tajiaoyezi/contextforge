# Task `26.1`: `tracestore-fts-and-vacuum — core/src/data_plane/search_persist.rs SqliteTracePersist 加 FTS5 全文检索（按内容查 trace）+ 周期 VACUUM（抑制 search_traces.db 无界膨胀）+ core/migrations/0016_*.sql FTS 影子表 + deterministic 测试`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 26 (observability-hardening)
**Dependencies**: task-16.1（`SqliteTracePersist` + `core/migrations/0015_search_traces.sql` 已落地 `put`/`get`/`list`/`load_warm`）/ ADR-031 D1/D2（trace FTS5 + VACUUM）/ ADR-002（sqlite-tantivy-layered-storage — rusqlite bundled SQLite 分层）/ ADR-004（local-first：默认 0 新 dep / 0 network）/ ADR-008（core-library-selection，依赖变更 add-only）/ ADR-013（禁伪造凭据）/ ADR-014 D1-D5（第十七次激活）

## 1. Background

Phase 16 task-16.1 用 `core/src/data_plane/search_persist.rs::SqliteTracePersist` 把每次检索的 `RetrievalTrace`（prost 序列化 → base64 TEXT 写 `trace_json`）持久化到 `<data_dir>/search_traces.db`，schema 在 `core/migrations/0015_search_traces.sql`：`search_traces(query_id TEXT PRIMARY KEY, trace_json TEXT, workspace_id TEXT, ts_unix INTEGER, created_at TEXT)` + `idx_search_traces_ts_desc`。当前查询面（`search_persist.rs`）：

- `put(key, trace, workspace_id, ts_unix)`——`INSERT OR REPLACE`（同 query_id 替换，保 LRU recency 语义）
- `get(key)`——主键命中，miss 返 `Ok(None)`
- `list(limit)`——`ORDER BY ts_unix DESC LIMIT`（clamp 1..=100），投影为 `QueryRecord`
- `load_warm(n)`——暖启动恢复 LRU（oldest-first，cap 1000）

两块缺口：**(1) 无按内容检索**——`RetrievalTrace.query` 等文本只能逐条 `get` 后在内存比对，无法「按内容查命中某关键词的 trace」；**(2) 无清理路径**——`put` 仅同 query_id 替换，不同 query_id 单调增长，`search_traces.db` 无界膨胀，无 VACUUM 回收 page。

ADR-031 D1/D2 记录硬化策略：FTS5 影子表（按内容检索）+ 周期 VACUUM（抑制膨胀）。FTS5 是 `rusqlite = { version = "0.39.0", features = ["bundled"] }`（`core/Cargo.toml:70`）bundled SQLite 自带的全文模块——**0 新依赖、0 network**（ADR-004 满足）。本 task 让 `SqliteTracePersist` 加这两块能力，既有 `put`/`get`/`list`/`load_warm` 签名与语义不变（add-only 方法）。

## 2. Goal

`core/src/data_plane/search_persist.rs` 的 `SqliteTracePersist` 新增两块能力：(a) `search_fts(query_text, limit)`——经 FTS5 影子虚表按内容检索，返回命中含 `query_text` 的 trace 投影（`QueryRecord` 序，按 FTS rank / `ts_unix` 排序，limit clamp 同 `list`）；(b) `vacuum()`——执行 SQLite `VACUUM` 回收 page + 可选 `prune_older_than(cutoff_ts)`（按 `ts_unix < cutoff` 删行后 VACUUM）。新增 `core/migrations/0016_*.sql` 建 FTS5 影子虚表（`search_traces_fts`，索引 `query_id` + 可读文本如 `RetrievalTrace.query`）+ 同步触发器（或 `put` 时显式同步），`IF NOT EXISTS` 幂等、旧库 boot 时回填。`put` / `get` / `list` / `load_warm` 既有签名与语义逐字节不变。≥3 Rust 测试全 PASS（默认构建可跑）：FTS index→search 命中 / FTS miss 不命中 / VACUUM（含 prune）后数据完好 + `row_count` 一致。默认构建 0 新依赖（FTS5 / VACUUM 复用 rusqlite bundled，无 Cargo.toml 改动）；`cargo test --workspace` 不退化。D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **修改 `core/src/data_plane/search_persist.rs`**：`SqliteTracePersist` 加 `search_fts(query_text, limit) -> Result<Vec<PbQueryRecord>, _>`（FTS5 `MATCH` 查询影子表 join 主表，投影 `QueryRecord`，limit clamp 1..=100 同 `list`）+ `vacuum() -> Result<(), _>`（`conn.execute_batch("VACUUM")`）+ `prune_older_than(cutoff_ts: i64) -> Result<usize, _>`（`DELETE FROM search_traces WHERE ts_unix < ?` 返删除行数，调用方可随后 `vacuum()`）；`put` 时同步写 FTS 影子表（经 0016 触发器自动 / 或 `put` 内显式 `INSERT INTO search_traces_fts`）。
- **新增 `core/migrations/0016_search_traces_fts.sql`**：`CREATE VIRTUAL TABLE IF NOT EXISTS search_traces_fts USING fts5(...)`（内容索引 `query_id` + 可读文本投影）+ `CREATE TRIGGER IF NOT EXISTS` 把 `search_traces` 的 INSERT/REPLACE/DELETE 同步到 FTS 影子表（或采用 FTS5 `content=` external-content 模式同步）；`include_str!` 进 `search_persist.rs`（承 `MIGRATION_SQL` 既有 pattern），`open` 时 `execute_batch` 幂等运行（旧库 boot 回填）。
- **新增同源 Rust 单测（`core/src/data_plane/search_persist.rs` 内 `#[cfg(test)] mod tests`，默认构建可跑）**：(a) FTS 命中——`put` 若干含确定文本的 trace → `search_fts("known-term", k)` 返回含该 term 的 trace 序；(b) FTS miss——`search_fts("absent-term", k)` 返空；(c) VACUUM / prune——插入 N 行 → `prune_older_than(cutoff)` 删旧行 → `vacuum()` → `row_count` 与剩余行一致、`get`/`list` 对保留行仍正确（VACUUM 不破坏数据）。
- **不改既有 `put`/`get`/`list`/`load_warm` 签名与语义**：FTS / VACUUM 为 add-only 方法（ADR-015 D1 思想：add-only）；`put` 内若新增 FTS 同步则保持既有 INSERT OR REPLACE 语义不变 + 返回类型不变。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **`SqliteTracePersist` 既有 `put`/`get`/`list`/`load_warm` 本体** [SPEC-OWNER:task-16.1-tracestore-sqlite-persistence]：本 task 在其上加 FTS / VACUUM，不重写既有方法。
- **events SSE 推送 + 从 audit log 重放** [SPEC-OWNER:task-26.2-events-sse-push-and-replay]：本 task 仅做 trace 持久面硬化，不触 events 实时面。
- **event-bus 分区 / 容量 / drain 配置 + smoke v16 + closeout** [SPEC-OWNER:task-26.3-closeout-v0.19.0]：本 task 落 trace 层能力 + 单测；smoke / release / ratify 在收口 task。
- **FTS5 跨库 schema 迁移 / 索引重建** [SPEC-DEFER:phase-future.tracestore-fts-schema-migration]：本 task 仅做单版本 FTS 影子表建立 + 同步触发器 + `IF NOT EXISTS` 幂等回填。
- **trace 内容脱敏 / 二次审计**（audit log 既有脱敏属 ADR-010）[SPEC-DEFER:phase-future.trace-content-redaction]：本 task FTS 索引 trace 既有持久文本，不引入脱敏层。
- **把 FTS 检索接进 console-api REST endpoint（如 `GET /v1/queries?q=`）** [SPEC-OWNER:task-26.3-closeout-v0.19.0]：本 task 落 Rust 层 `search_fts` 能力 + 单测；REST 暴露 / smoke 在收口 task 据实评估。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`core/src/data_plane/search_persist.rs::SqliteTracePersist`**：task-16.1 trace 持久 store，本 task 加 `search_fts` / `vacuum` / `prune_older_than`。
- **`core/migrations/0016_search_traces_fts.sql`**：本 task 新增的 FTS5 影子表 + 同步触发器 migration（承 `0015_search_traces.sql` 编号序）。
- **bundled SQLite（rusqlite `features=["bundled"]`，`core/Cargo.toml:70`）**：FTS5 + VACUUM 来源——本 task 核实 bundled 含 FTS5（默认含）。
- **下游 task-26.3**：closeout 据本 task FTS / VACUUM 能力评估 smoke v16 断言 + 是否经 console-api 暴露。

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/data_plane/search_persist.rs`（`SqliteTracePersist` / `MIGRATION_SQL include_str!` / `put` `INSERT OR REPLACE` / `get` / `list` clamp 1..=100 / `load_warm` / `encode_trace`/`decode_trace` / `#[cfg(test)] mod tests` + `row_count` testing aid）
- `core/migrations/0015_search_traces.sql`（`search_traces` 5 列 schema + `idx_search_traces_ts_desc`，FTS 影子表索引对象）
- `core/Cargo.toml:70`（`rusqlite = { version = "0.39.0", features = ["bundled"] }` — FTS5 / VACUUM 来源；核实 bundled 含 FTS5）
- `docs/decisions/adr-031-observability-hardening.md`（D1 FTS5 影子表 + D2 VACUUM + D6 默认 0-dep）+ `docs/decisions/adr-002-sqlite-tantivy-layered-storage.md`（rusqlite bundled 分层）
- `docs/decisions/adr-004-local-first-privacy-baseline.md`（默认 0 新 dep / 0 network 红线）+ `docs/decisions/adr-008-core-library-selection.md`（依赖 add-only）+ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造）
- `docs/specs/tasks/task-16.1-tracestore-sqlite-persistence.md`（既有 trace 持久面 + 设计取舍）

### 5.2 关键设计 — FTS5 影子表 + 同步 + VACUUM

- **FTS5 影子虚表（external-content 或 trigger-synced）**：`search_traces_fts` 用 `fts5(...)` 索引 `query_id`（unindexed rowid join key）+ 可读文本（`RetrievalTrace.query` 投影；trace 文本经 `decode_trace` 取 `query` 字段写入 FTS——避免索引 base64 `trace_json` 不可读内容）。两种实现路径任选其稳妥者：
  - **路径 A（trigger-synced 独立内容表）**：FTS 表存自己的内容副本，`search_traces` INSERT/REPLACE/DELETE 经 `CREATE TRIGGER` 同步到 FTS 表（标准 FTS5 trigger pattern）。
  - **路径 B（external-content `content=`）**：FTS 表声明 `content='search_traces'` 复用主表内容，触发器仅同步 FTS 索引（省一份副本，但需主表有可索引文本列——当前 `trace_json` 是 base64，需在 0016 加可索引投影列或在 `put` 内显式写 FTS）。
  - 选路径以「0 新 dep + 最小 schema 改动 + `put` 既有语义不变」为准（task-26.1 实施期核实，§10 回填结论）。
- **`search_fts(query_text, limit)`**：`SELECT query_id, ... FROM search_traces_fts JOIN search_traces USING(query_id) WHERE search_traces_fts MATCH ?1 ORDER BY rank LIMIT ?2`，投影为 `PbQueryRecord`（同 `list` 投影）；limit clamp 1..=100（承 `list` 既有 clamp）；空命中返 `Ok(vec![])`。
- **`vacuum()` / `prune_older_than(cutoff)`**：`vacuum` 执行 `VACUUM`（独占库、重建紧凑文件、回收 page）——不在 hot path 同步调（调用方在维护窗口 / boot 时调）；`prune_older_than` 先 `DELETE FROM search_traces WHERE ts_unix < ?1`（触发器级联清 FTS）返删除行数，调用方可随后 `vacuum()` 真回收空间。
- **ADR-013**：FTS index→search 命中 + VACUUM 后数据完好是 deterministic 单测可验证项（默认构建可跑，无 feature gate）；不预判跨语料检索数值，仅断言「含某确定 term 的 trace 被 FTS 命中 / 不含的不命中」+「VACUUM 后保留行 get/list 仍正确」。

### 5.3 不变量

- 默认构建 0 新依赖（FTS5 / VACUUM 是 rusqlite bundled SQLite 内建，无 Cargo.toml 改动；ADR-004 / ADR-008）。
- 既有 `put`/`get`/`list`/`load_warm` 签名与语义逐字节不变（add-only 方法；`put` 内若加 FTS 同步保持 INSERT OR REPLACE 语义 + 返回类型不变）。
- `0016_*.sql` `IF NOT EXISTS` 幂等：旧 `search_traces.db`（仅 0015 schema）boot 时回填 FTS 表不破坏既有数据；新库一次建全。
- FTS 检索确定性：含某确定 term 的 trace → `search_fts` 命中；不含 → 不命中（FTS5 MATCH 语义稳定）。
- VACUUM / prune 不破坏数据：保留行的 `get`/`list`/`search_fts` 仍正确；`row_count` 与剩余行一致。
- 不索引敏感原文超出既有持久范围：FTS 仅索引 `search_traces` 已持久的 trace 文本（`RetrievalTrace.query` 等），不新引入持久字段。

## 6. Acceptance Criteria

- [ ] **AC1**: `search_fts(query_text, limit)` FTS5 按内容命中——`put` 若干含确定 term 的 trace → `search_fts("known-term", k)` 返回含该 term 的 trace 投影序（`QueryRecord`），limit clamp 1..=100 — verified by **TEST-26.1.1**
- [ ] **AC2**: FTS miss 不误命中——`search_fts("absent-term", k)` 对不含该 term 的库返 `Ok(vec![])`（不报错、不误命中） — verified by **TEST-26.1.2**
- [ ] **AC3**: `vacuum()` + `prune_older_than(cutoff)` 回收空间且数据完好——插入 N 行 → `prune_older_than` 删旧行（返删除行数）→ `vacuum()` → `row_count` 与剩余行一致 + 保留行 `get`/`list` 仍正确（VACUUM 不破坏数据，不 panic） — verified by **TEST-26.1.3**
- [ ] **AC4**: 既有 `put`/`get`/`list`/`load_warm` 签名与语义不变 + `0016_*.sql` 幂等回填——旧库（仅 0015）`open` 后回填 FTS 表不破坏既有行；既有 task-16.1 单测不退化 — verified by **TEST-26.1.4**
- [ ] **AC5**: 既有不退化 + 0 新依赖 — 默认 `cargo test --workspace` 全 PASS + 无 Cargo.toml 改动（FTS5 / VACUUM 复用 rusqlite bundled）；`go test ./...` 不受影响（本 PR 零 Go delta）；D2 lint `--touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-26.1.5** + §10 实测

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-26.1.1 | `search_fts` FTS5 按内容命中含 term 的 trace + limit clamp | `core/src/data_plane/search_persist.rs`（`mod tests`） | Planned |
| TEST-26.1.2 | `search_fts` miss 返 `Ok(vec![])` 不误命中不报错 | `core/src/data_plane/search_persist.rs`（`mod tests`） | Planned |
| TEST-26.1.3 | `prune_older_than` + `vacuum` 回收空间 + 数据完好 + row_count 一致 | `core/src/data_plane/search_persist.rs`（`mod tests`） | Planned |
| TEST-26.1.4 | 既有 put/get/list/load_warm 不变 + 0016 幂等回填旧库 | `core/src/data_plane/search_persist.rs`（`mod tests`）+ `core/migrations/0016_*.sql` | Planned |
| TEST-26.1.5 | 默认 `cargo test --workspace` 0 failed + 0 新依赖 + D2 lint 0 未标注 | 全 Rust + `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（中）FTS5 影子表 + 触发器使 `put` 写放大**（承 phase-26 §7 R1）：每次 `put` 多写一份 FTS 倒排索引。
  - **缓解**：trace `put` 是检索后写、非高频热路径（单用户 local-first）；FTS5 是 bundled SQLite 内建（无外部成本）；写放大限于 trace 持久面，不触检索热路径。stop-condition：若核实 bundled SQLite 未启用 FTS5（极少见——rusqlite bundled 默认含 FTS5），记录受阻态，AC1/AC2 不标 `[x]` 并按 ADR-013 如实记录（不伪造 FTS 通过）。
- **R2（低）VACUUM 独占库阻塞并发访问**：`VACUUM` 需独占；并发 `put`/`get` 时阻塞。
  - **缓解**：`vacuum()` 不在 hot path 同步调（调用方在维护窗口 / boot 时调）；`SqliteTracePersist` 已是 `Mutex<Connection>` 串行化（`search_persist.rs:36`），VACUUM 在持锁期独占，符合既有串行语义。AC3 仅断言「VACUUM 后数据完好 + 不 panic」，不断言并发性能。
- **R3（低）0016 migration 与旧库不兼容**：旧 `search_traces.db` 仅含 0015 schema。
  - **缓解**：0016 全用 `IF NOT EXISTS`（承 0015 pattern）+ `open` 时 `execute_batch` 幂等运行；旧库回填 FTS 表（trigger-synced 路径需对存量行做一次性回填，在 0016 内 `INSERT INTO ..._fts SELECT ... FROM search_traces` 一次性填）；TEST-26.1.4 覆盖旧库回填路径。

## 9. Verification Plan

```bash
# Rust：默认构建（0 新依赖，无 feature gate）FTS + VACUUM + 既有不退化
cargo test -p contextforge-core data_plane::search_persist

# 全 workspace 默认不退化 + 0 新依赖（无 Cargo.toml 改动）
cargo test --workspace

# Go 不退化（本 PR 零 Go delta）
go test ./...

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **Status**: 待实施（Draft）。实施完成后按 6 项回填：完成日期 / 改动文件（含 `0016_*.sql` 路径选型 A vs B 结论）/ commit 列表（RED→GREEN）/ §9 Verification 实测结果（ADR-013 真实非合成）/ 设计取舍（FTS5 同步路径 + VACUUM 触发口径核实结论）/ 剩余风险 + 下游影响（FTS 经 console-api 暴露由 task-26.3 据实评估 + indexing 事件 FTS 留 backlog）。
