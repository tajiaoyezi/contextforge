# Task `16.1`: `tracestore-sqlite-persistence — migration 0015_search_traces.sql + SqliteTracePersist 模块 + TraceStore write-through 改造 + daemon 重启 warm restore`

**Status**: Ready

**Priority**: P4
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 16 (v0.9.0-backlog-completion)
**Dependencies**: Phase 12 task-12.3（既有 TraceStore in-memory LRU 实现）+ Phase 15 task-15.5（既有 TraceRecord wrapper / `TraceStore.put(key, trace, workspace_id, ts_unix)` 签名）

## 1. Background

ContextForge-Console PR #91/#93 backlog 列 P4 #10：

> `GET /v1/queries` 和 `GET /v1/search/{query_id}/trace` 当前走 `core/src/data_plane/search.rs::TraceStore` 内存 HashMap+VecDeque cap=1000；daemon 重启即丢全部历史。生产环境用户体验：每次重启 Dashboard "最近查询"列表清空 + 旧 query_id drill-down 全部 404。期望：SQLite 持久化让历史跨重启保留。

既有 v0.8.0 状态：
- `core/src/data_plane/search.rs:44-115` `TraceStore { map: HashMap, order: VecDeque, cap: 1000 }` — task-12.3 ship
- `core/src/data_plane/search.rs:30-40` `TraceRecord { trace, workspace_id, ts_unix }` wrapper — task-15.5 ship
- 进程内单实例 `Arc<Mutex<TraceStore>>`；`SearchServer.new(stores)` 构造时 `TraceStore::new(TRACE_STORE_CAP)`
- 既有 spec 显式标 `[SPEC-DEFER:task-future.search-trace-sqlite-persistence]` (task-12.3) / `[SPEC-DEFER:phase-16.tracestore-sqlite-persist]` (task-15.5)

**实施策略**：

- 新增 SQLite migration `core/migrations/0015_search_traces.sql`（表 `search_traces` 5 列 + 1 索引 + IF NOT EXISTS 幂等）
- 新增模块 `core/src/data_plane/search_persist.rs`（`SqliteTracePersist` struct + open(data_dir) / put / list / load_warm 方法）
- 改造 `core/src/data_plane/search.rs::TraceStore` 为 write-through 设计 — `put` 双写（先内存 LRU，再 SQLite best-effort）+ `list` 先内存命中 + miss 时落 SQLite 回填 + 新增 `warm_restore(persist) -> Self` 启动时 load
- `SearchServer::new` 签名加 `data_dir: PathBuf` 参；`serve_full` 把 data_dir 传下去
- ADR-014 D2 lint：本 task spec 的 anti-pattern (`stub` / `future` / `[SPEC-DEFER]`) 全部标注

## 2. Goal

新增 SQLite migration `0015_search_traces.sql` 自动建表；`TraceStore.put` 写穿 SQLite；daemon 重启后 `warm_restore` 从 SQLite load 最近 1000 条到内存 LRU；`GET /v1/queries` 和 `GET /v1/search/{query_id}/trace` 跨 daemon 重启返历史；既有 in-memory cap=1000 LRU 行为不破坏；`cargo test --workspace` + `go test ./...` 不退化；≥4 unit test + ≥1 integration test PASS。

## 3. Scope

### In Scope

- **新建 `core/migrations/0015_search_traces.sql`**：
  ```sql
  -- Phase 16 task-16.1 — TraceStore SQLite persistence (P4 #10).
  -- 5 列对齐 TraceRecord wrapper + PbRetrievalTrace JSON 序列化。
  CREATE TABLE IF NOT EXISTS search_traces (
    query_id      TEXT PRIMARY KEY,
    trace_json    TEXT NOT NULL,
    workspace_id  TEXT NOT NULL,
    ts_unix       INTEGER NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
  );
  CREATE INDEX IF NOT EXISTS idx_search_traces_ts_desc ON search_traces (ts_unix DESC);
  ```
  - 复用既有 SQLite migration 机制（`SqliteWorkspaceStore::open` 等同款 execute_batch IF NOT EXISTS 幂等）
  - `trace_json` 序列化 PbRetrievalTrace via `prost::Message::encode_to_vec` + base64 OR `serde_json` — 实施时 grep `prost::Message` 既有使用模式决定；初选 `prost-encoded bytes -> base64::encode -> store as TEXT`（与 PbRetrievalTrace prost-derive 一致）

- **新建 `core/src/data_plane/search_persist.rs`**（≥150 行实现 + ≥5 unit test）：
  ```rust
  use rusqlite::{params, Connection, OptionalExtension};
  use std::path::Path;
  use std::sync::Mutex;
  use crate::pb_console::{QueryRecord as PbQueryRecord, RetrievalTrace as PbRetrievalTrace};

  pub struct SqliteTracePersist {
      conn: Mutex<Connection>,
  }

  impl SqliteTracePersist {
      pub fn open(data_dir: &Path) -> Result<Self, SqliteTracePersistError> {
          let path = data_dir.join("search_traces.db");
          let conn = Connection::open(&path)?;
          conn.execute_batch(include_str!("../../migrations/0015_search_traces.sql"))?;
          Ok(Self { conn: Mutex::new(conn) })
      }

      pub fn put(
          &self,
          key: &str,
          trace: &PbRetrievalTrace,
          workspace_id: &str,
          ts_unix: i64,
      ) -> Result<(), SqliteTracePersistError> {
          let trace_json = encode_trace(trace)?;
          let conn = self.conn.lock().map_err(|_| SqliteTracePersistError::Poisoned)?;
          conn.execute(
              "INSERT OR REPLACE INTO search_traces (query_id, trace_json, workspace_id, ts_unix) VALUES (?1, ?2, ?3, ?4)",
              params![key, trace_json, workspace_id, ts_unix],
          )?;
          Ok(())
      }

      pub fn get(&self, key: &str) -> Result<Option<PbRetrievalTrace>, SqliteTracePersistError> {
          let conn = self.conn.lock().map_err(|_| SqliteTracePersistError::Poisoned)?;
          let row = conn.query_row(
              "SELECT trace_json FROM search_traces WHERE query_id = ?1",
              params![key],
              |r| r.get::<_, String>(0),
          ).optional()?;
          row.map(|s| decode_trace(&s)).transpose()
      }

      pub fn list(&self, limit: usize) -> Result<Vec<PbQueryRecord>, SqliteTracePersistError> {
          let conn = self.conn.lock().map_err(|_| SqliteTracePersistError::Poisoned)?;
          let mut stmt = conn.prepare(
              "SELECT query_id, trace_json, workspace_id, ts_unix FROM search_traces ORDER BY ts_unix DESC LIMIT ?1",
          )?;
          let lim = limit.clamp(1, 100) as i64;
          let rows = stmt.query_map(params![lim], |r| {
              let key: String = r.get(0)?;
              let trace_json: String = r.get(1)?;
              let workspace_id: String = r.get(2)?;
              let ts_unix: i64 = r.get(3)?;
              Ok((key, trace_json, workspace_id, ts_unix))
          })?;
          let mut out = Vec::with_capacity(lim as usize);
          for r in rows {
              let (key, trace_json, workspace_id, ts_unix) = r?;
              let trace = decode_trace(&trace_json)?;
              out.push(PbQueryRecord {
                  query_id: key,
                  query: trace.query.clone(),
                  ts_unix,
                  workspace_id,
              });
          }
          Ok(out)
      }

      /// task-16.1: warm restore — load most-recent N traces back into a Vec
      /// (in insertion-order from oldest-to-newest, so caller can re-insert
      /// into the in-memory LRU preserving recency).
      pub fn load_warm(
          &self,
          n: usize,
      ) -> Result<Vec<(String, PbRetrievalTrace, String, i64)>, SqliteTracePersistError> {
          let conn = self.conn.lock().map_err(|_| SqliteTracePersistError::Poisoned)?;
          let mut stmt = conn.prepare(
              "SELECT query_id, trace_json, workspace_id, ts_unix FROM search_traces ORDER BY ts_unix DESC LIMIT ?1",
          )?;
          let lim = n.min(1000) as i64;
          let rows = stmt.query_map(params![lim], |r| {
              Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, i64>(3)?))
          })?;
          let mut out: Vec<(String, PbRetrievalTrace, String, i64)> = Vec::new();
          for r in rows {
              let (key, trace_json, ws, ts) = r?;
              out.push((key, decode_trace(&trace_json)?, ws, ts));
          }
          // Reverse so caller inserts oldest-first → newest ends at back of VecDeque
          out.reverse();
          Ok(out)
      }
  }

  fn encode_trace(t: &PbRetrievalTrace) -> Result<String, SqliteTracePersistError> { ... } // prost encode → base64
  fn decode_trace(s: &str) -> Result<PbRetrievalTrace, SqliteTracePersistError> { ... }   // base64 → prost decode

  #[derive(Debug)]
  pub enum SqliteTracePersistError {
      Sqlite(rusqlite::Error),
      Codec(String),
      Poisoned,
  }
  ```

- **改造 `core/src/data_plane/search.rs::TraceStore`** 为 write-through：
  ```rust
  struct TraceStore {
      // 既有 hot cache
      map: HashMap<String, TraceRecord>,
      order: VecDeque<String>,
      cap: usize,
      // task-16.1: SQLite SoT (optional — None in tests for backward compat)
      persist: Option<Arc<SqliteTracePersist>>,
  }

  impl TraceStore {
      // 既有 new 保留作 in-memory-only 构造（test convenience）
      fn new(cap: usize) -> Self { ... }

      // task-16.1: new with persist + warm restore
      fn with_persist(cap: usize, persist: Arc<SqliteTracePersist>) -> Self {
          let mut store = Self {
              map: HashMap::with_capacity(cap),
              order: VecDeque::with_capacity(cap),
              cap,
              persist: Some(persist.clone()),
          };
          if let Ok(warm) = persist.load_warm(cap) {
              for (key, trace, ws, ts) in warm {
                  store.put_mem_only(key, trace, ws, ts);
              }
          }
          store
      }

      fn put(&mut self, key: String, trace: PbRetrievalTrace, workspace_id: String, ts_unix: i64) {
          // 1. write hot cache (既有 LRU 行为不变)
          self.put_mem_only(key.clone(), trace.clone(), workspace_id.clone(), ts_unix);
          // 2. write-through to SQLite (best-effort; SQLite error logged, swallowed)
          if let Some(p) = self.persist.as_ref() {
              if let Err(e) = p.put(&key, &trace, &workspace_id, ts_unix) {
                  eprintln!("WARN search_persist.put failed (key={key}): {e:?}; hot cache still updated");
              }
          }
      }

      fn put_mem_only(&mut self, ...) { ... } // 既有 put body 抽出

      fn get(&self, key: &str) -> Option<PbRetrievalTrace> {
          // 1. hot cache 命中 → 返
          if let Some(r) = self.map.get(key) { return Some(r.trace.clone()); }
          // 2. miss → SQLite fallback (read-only; 不回填 LRU 避免污染 recency)
          self.persist.as_ref().and_then(|p| p.get(key).ok().flatten())
      }

      fn list(&self, limit: usize) -> Vec<PbQueryRecord> {
          // task-16.1 design: hot cache 是最近 N；多数 Console UI list 请求 limit ≤ 20 → 内存覆盖
          // 内存返足量直接返；不足 fallback SQLite 补
          let mem = self.list_mem(limit);
          if mem.len() >= limit || self.persist.is_none() { return mem; }
          // Persist fallback
          self.persist.as_ref().unwrap().list(limit).unwrap_or(mem)
      }

      fn list_mem(&self, limit: usize) -> Vec<PbQueryRecord> { ... } // 既有 list body 抽出
  }
  ```

- **修改 `core/src/data_plane/search.rs::SearchServer`** 构造签名：
  ```rust
  pub struct SearchServer {
      stores: Arc<DataPlaneStores>,
      trace_store: Arc<Mutex<TraceStore>>,
  }

  impl SearchServer {
      pub fn new(stores: Arc<DataPlaneStores>) -> Self {
          // 既有签名保留作 test convenience (in-memory only)
          Self {
              stores,
              trace_store: Arc::new(Mutex::new(TraceStore::new(TRACE_STORE_CAP))),
          }
      }

      // task-16.1: new_with_persist
      pub fn new_with_persist(stores: Arc<DataPlaneStores>, persist: Arc<SqliteTracePersist>) -> Self {
          Self {
              stores,
              trace_store: Arc::new(Mutex::new(TraceStore::with_persist(TRACE_STORE_CAP, persist))),
          }
      }
  }
  ```

- **修改 `core/src/data_plane/mod.rs`**：
  - 注册 `pub mod search_persist;`
  - 导出 `pub use search_persist::{SqliteTracePersist, SqliteTracePersistError};`

- **修改 `core/src/server.rs::serve_full`**：
  - 在 `DataPlaneStores` 构造后 + `SearchServer` 实例化前加：
    ```rust
    let trace_persist = Arc::new(SqliteTracePersist::open(&data_dir)?);
    let search_server = SearchServer::new_with_persist(stores.clone(), trace_persist);
    ```

- **依赖管理**：
  - `core/Cargo.toml` 检查 `rusqlite` + `base64` 是否已 in workspace（多半已有；如缺 base64 → 加 `base64 = "0.22"` 在 R7 dep gate 走 add-only — `base64` 0.22 stable + 无 transitive 重；如已有 trace 不需）
  - 如已有 `prost::Message::encode_to_vec` 路径 → 用现有；否则 fallback `serde_json::to_string(&trace_json_shape)` 自定义 shape（推荐 prost encode）

- **单元测试 ≥5**（在 `core/src/data_plane/search_persist.rs::tests`）：
  - `test_open_creates_search_traces_table`
  - `test_put_then_get_roundtrip`
  - `test_put_then_list_returns_desc_by_ts`
  - `test_load_warm_returns_recent_n_in_oldest_first_order`
  - `test_list_clamps_to_100`

- **集成测试 ≥1**（`core/tests/search_persist_integration.rs` 新建）：
  - `test_tracestore_persists_across_restart`：
    1. 起 `SqliteTracePersist::open(tmpdir)`
    2. `TraceStore::with_persist(3, persist.clone())` + put 3 个 trace
    3. drop store
    4. 重新 `TraceStore::with_persist(3, persist)` — assert warm restore；assert `list(3)` 返 3 条且顺序按 ts_unix DESC
    5. tmpdir 文件 `search_traces.db` 存在 + 行数 = 3

- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **TraceStore FULLTEXT 检索（按 query 文本搜历史）** [SPEC-DEFER:phase-future.tracestore-fts]：v0.9 仅 list by recency + get by id；FTS 留 v1.x
- **TraceStore 跨 workspace 严格 isolation** [SPEC-DEFER:phase-future.tracestore-multi-workspace-strict]：v0.9 沿用 task-15.5 顺带字段；按 workspace_id WHERE filter 留 v1.x
- **search_traces 表 vacuum / 自动 LRU 截断** [SPEC-DEFER:phase-future.tracestore-sqlite-vacuum]：v0.9 仅 INSERT OR REPLACE；表大小无收敛策略；长时间运行可能数百万行
- **SQLite WAL 模式调优** [SPEC-DEFER:phase-future.tracestore-sqlite-wal-tune]：v0.9 沿用 rusqlite 默认；高写入并发调 WAL + busy_timeout 留 v1.x
- **持久化批量异步写（write batching）** [SPEC-DEFER:phase-future.tracestore-batch-write]：v0.9 每 put 同步 INSERT；批量 + tokio::spawn 异步留 v1.x
- **trace_json 压缩存储** [SPEC-DEFER:phase-future.trace-json-compression]：v0.9 prost-encoded base64 TEXT；gzip / zstd 压缩留 v1.x
- **跨 trace_persist 实例并发**：v0.9 单 daemon 单实例 + Mutex 包裹 Connection；多 daemon 共享 SQLite 文件留 [SPEC-DEFER:phase-future.multi-daemon-shared-tracestore]
- **既有 v0.8 用户升级数据迁移**：v0.9 fresh start — v0.8 in-memory 历史本来就重启即丢，v0.9 ship 后从空 SQLite 开始累积，**不**做"v0.8 内存数据导出 → v0.9 SQLite 导入"工具 [SPEC-DEFER:phase-future.tracestore-v0.8-data-import]

## 4. Users / Actors

- **Console UI 端**（下游，via cross-repo）：Dashboard "最近查询" 面板 + trace drill-down 视图跨 daemon 重启不丢
- **debug session**：开发者 daemon 重启后仍能调旧 query_id 查 trace
- **production ops**：daemon crash / 计划重启不再清空历史

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-16-v0.9.0-backlog-completion.md` §3 / §6 AC1
- `docs/specs/tasks/task-12.3-search-trace-by-query-id.md` (TraceStore 既有 in-memory 实现 + LRU cap)
- `docs/specs/tasks/task-15.5-query-history-endpoint.md` (TraceRecord wrapper / `put(key, trace, ws_id, ts)` 签名 / list clamp 1..=100)
- `core/src/data_plane/search.rs` 既有 TraceStore (line 44-115) + SearchServer (line 117-130)
- `core/src/workspace/store.rs` (既有 SQLite migration 模式参考 — execute_batch IF NOT EXISTS 幂等)
- `core/src/eval/store.rs` (既有 SQLite store with Mutex<Connection> 模式参考)

### 5.2 Imports

- **Rust**: 既有 `rusqlite` + `prost` + `std::sync::{Arc, Mutex}` + `std::collections::{HashMap, VecDeque}`
- 可能新增：`base64` v0.22（如不在 workspace；R7 dep gate add-only）OR `serde_json`（既有）— 实施时 grep `base64` 是否已用决定
- **不引入新 schema language**：纯 SQL DDL

### 5.3 Migration 时序

- daemon 启动 → `serve_full` 调 `SqliteTracePersist::open(data_dir)` → `Connection::open` + `execute_batch(include_str!("0015_search_traces.sql"))` 幂等建表
- migration 0015 与既有 0010-0014 独立 — 不依赖既有表；不依赖既有 store schema
- 升级用户（v0.8 → v0.9）：daemon 重启后 `search_traces.db` 文件首次创建 + 空表；从下次 search 开始累积；不破坏既有 `workspaces.db` / `memory.db` / `eval.db` / `chunks.db`

## 6. Acceptance Criteria

- [ ] AC1：`0015_search_traces.sql` migration 成功执行（5 列 + 1 索引 + IF NOT EXISTS 幂等）；daemon 启动后 `search_traces.db` 文件存在 + 表结构正确 — **verified by `data_plane::search_persist::tests::test_open_creates_search_traces_table` PASS + integration `core/tests/search_persist_integration.rs::test_tracestore_persists_across_restart` `search_traces.db` 文件 + 3 rows 实测**
- [ ] AC2：`TraceStore::with_persist(cap, persist)` put N 个 trace 后 + `persist.list(N)` 返 N 条按 ts_unix DESC；既有 `TraceStore::new(cap)` in-memory-only 构造不破坏（cap LRU 行为既有 test 全过）— **verified by `data_plane::search_persist::tests::test_put_then_list_returns_desc_by_ts` + 既有 `data_plane::search::tests::test_trace_store_eviction_at_capacity` + `test_trace_store_list_returns_recent_first` 不退化**
- [ ] AC3：daemon 重启 warm restore — kill -9 daemon + restart + 内存 LRU 从 SQLite load 最近 1000 条；`GET /v1/queries?limit=10` 返历史；`GET /v1/search/{query_id}/trace` 任一历史 id 返 200 trace — **verified by `core/tests/search_persist_integration.rs::test_tracestore_persists_across_restart` PASS**
- [ ] AC4：write-through 双写 — `TraceStore.put` 写 SQLite 失败（如磁盘满 simulate）不破坏内存 LRU 行为；warn 日志输出；下次 daemon 重启不丢内存数据（仍 in-memory，但 SQLite 缺失记录） — **verified by `data_plane::search_persist::tests::test_put_sqlite_error_logged_swallow` 实现 OR via inject Result::Err in test PASS**
- [ ] AC5：`SearchServer::new_with_persist` 集成既有 `serve_full` 调用链；既有 `SearchServer::new(stores)` 不破坏（test convenience 保留）— **verified by `cargo build --release -p contextforge-core --bin contextforge-core` clean + `core/tests/search_real_retriever.rs` 既有 test 不退化**
- [ ] AC6：既有 22-endpoint conformance + Phase 15 v6 smoke 不退化；`cargo test --workspace` 121 lib + 17 integration files 全 PASS；新增 ≥5 unit + ≥1 integration — **verified by closeout PR body 跑 cargo + go test + bash -n scripts/console_smoke.sh 实测**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | migration + 表结构 | 0015_search_traces.sql + search_persist.rs::open + unit test | Ready |
| AC2 | put + list SQLite roundtrip | search_persist.rs + 2 unit test | Ready |
| AC3 | warm restore 跨重启 | search.rs::TraceStore::with_persist + integration test | Ready |
| AC4 | write-through best-effort 错误吞 | search.rs::TraceStore::put + unit test | Ready |
| AC5 | SearchServer 构造集成 | search.rs::new_with_persist + server.rs serve_full | Ready |
| AC6 | regression 不退化 | closeout PR cargo+go+bash 实测 | Ready |

## 8. Risks

- **prost encode/decode round-trip 字段丢失**：PbRetrievalTrace optional 字段 prost3 默认值序列化可能丢；缓解 — 在 `test_put_then_get_roundtrip` 中显式 assert 全字段对等（trace.query / retrieved_chunks.len() / retrieved_chunks[0].chunk_id 等）
- **rusqlite Connection Mutex 锁竞争**：write-heavy 时多个 search 请求阻塞；缓解 — 每个 put 锁 < 5ms（INSERT OR REPLACE 单行），可接受；高并发调优留 [SPEC-DEFER:phase-future.tracestore-sqlite-wal-tune]
- **base64 dep 引入**：如 base64 不在 workspace → R7 dep gate 走 add-only；base64 v0.22 是 stable + 无 transitive 重；可降级用 `hex` 或 `serde_json` 字符串 escape 替代 — 实施时 grep 既有依赖决定
- **search_traces.db 文件锁定 / Windows file lock 风险**：rusqlite 默认 mode 跨 Windows 测试可能撞 file in-use；缓解 — `Connection::open` 用 default mode + tmpdir 测试用唯一 nanos 名（既有 eval_integration.rs 同款）
- **trace_json 列表大小**：每个 trace prost-encoded ≤ 4KB；100万行 ~ 4GB —— v0.9 接受；vacuum 留 [SPEC-DEFER:phase-future.tracestore-sqlite-vacuum]
- **既有 `TraceStore::new` callers**：测试中既有 `TraceStore::new(3)` / `TraceStore::new(10)` 调用 — 保留 in-memory-only 行为；新构造 `with_persist` 不影响既有 path
- **warm restore 排序 vs LRU 插入顺序边界**：`load_warm` 按 `ts_unix DESC` SELECT 后 reverse 成 oldest-first 给 LRU 插入 — 假设 put 顺序对齐 ts_unix 递增（实时 search 场景成立）。如 future 引入 backfill / 旧数据导入 → ts_unix 序与插入序背离时 LRU recency 不再等价 SQLite ORDER BY ts_unix DESC；v0.9 实时使用 case 不撞此问题 [SPEC-DEFER:phase-future.tracestore-backfill-ordering]
- **关联 [ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md) D1 add-only**：本 task 不动 contractv1.go 字段集合 + 不动 proto wire format（仅内部 SQLite schema）；零跨仓影响

## 9. Verification Plan

- **install**: 已有 `cargo fetch`
- **lint**: `cargo fmt --check -p contextforge-core` + `cargo clippy -p contextforge-core --lib -- -D warnings`
- **typecheck**: `cargo check -p contextforge-core --lib --tests`
- **unit-test**: `cargo test -p contextforge-core --lib data_plane::search_persist::tests` + `cargo test -p contextforge-core --lib data_plane::search::tests`
- **integration**: `cargo test --test search_persist_integration`
- **e2e**: smoke v7 Step 26（task-16.4 collect）
- **build**: `cargo build --release -p contextforge-core --bin contextforge-core`
- **runtime-smoke**: start daemon + 3 次 POST /v1/search + kill daemon + restart + `GET /v1/queries?limit=10` 返历史
- **manual**: 见 §9 runtime-smoke
- **coverage**: 不强制；search_persist.rs ≥ 5 unit + 1 integration 已覆盖核心路径

## 10. Completion Notes

(待 Done 时回填 — standard.md §8.3 6 项 schema)
