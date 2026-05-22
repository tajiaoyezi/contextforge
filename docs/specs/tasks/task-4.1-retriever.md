# Task `4.1`: `retriever — BM25 / metadata / filter 检索`

> ✅ 已过 `/s2v-implement` §2A 前置审核（2026-05-22）：§3/§4/§5.2/§5.3 `<TBD-by-user>` 已清零、§6 AC 经用户审定接受。实时状态以下方 `**Status**` 字段为准；状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: In Progress

**Priority**: P0
**Owner**: tajiaoyezi
**Related Phase**: Phase 4 (retrieval-explain)
**Dependencies**: Phase 2 (index-core)

## 1. Background

可解释检索的检索内核：在 Phase 2 的 Tantivy + SQLite 索引上做 BM25 全文 + metadata + filter 检索（PRD §Decisions Log D2 P0 = 可解释 BM25/metadata baseline，不依赖向量）。

## 2. Goal

`retriever` 支持 BM25 全文检索 + metadata 检索 + filter（source_type / language / collection / agent_scope / time），返回 Top-K；空/错误 query 返回空结果不 panic；满足 PRD §Constraints 性能（10 万 chunk P95 < 500ms，不含 embedding/远程）。

## 3. Scope

### In Scope

- 新增 `Retriever` 模块（独立 read-only struct；与 indexer 解耦）：自己 open SQLite + Tantivy 句柄（meta.json 路径与 task-2.4 IndexSession 一致：`[data_dir]/collections/[id]/{metadata.sqlite, tantivy/}`）
- 实现 AC1–AC5：
  - BM25 全文 + Tantivy term query 元数据过滤 → Top-K 命中 → SQLite JOIN 取完整字段
  - filter 协议（SearchFilters）接受 PRD §search 契约全 5 字段：`language` / `source_type` / `collection` / `agent_scope` / `time_range`
  - 空 / 错误 query → 返回空 `Vec<SearchResult>` 不 panic（QueryParserError 转 Ok(vec![]) + 写 debug log）
  - field boost（默认 `file_path: 2.0` / `content: 1.0`）via QueryParser::set_field_boost
  - exact phrase via Tantivy QueryParser 原生 `"..."` 语法
  - tokenizer 可配置（`RetrieverConfig::tokenizer: String`，v0.1 仅支持 default；CJK 留接入点）
- `SearchResult` 字段集对齐 PRD §REST/MCP search response（chunk_id / file_path / line_start / line_end / language / score / retrieval_method / reason / agent_scope / redaction_status / content）
- 模块入口：`core/src/retriever/mod.rs`（在 task-1.3 placeholder 上实现）

### Out Of Scope

- **AC2 filter 中 `source_type` / `agent_scope` 实际生效**（§2A 用户决策选项 A）：v0.1 接受 protocol 但 indexer SQLite chunks 表 / Tantivy schema 不含这两列 → no-op + §10 schema gap 留档；indexer 扩 schema 走未来 SPEC-DRIFT-task-2.4 chore-spec PR
- **AC4 性能 P95 < 500ms 硬测**：本 task 不跑大规模 benchmark；架构支持即可（task-8.1 eval-harness 回归）
- **CJK-aware / n-gram tokenizer 实测**（PRD §O11 R8）：留 `RetrieverConfig::tokenizer` 接入点；默认 English tokenizer
- **retrieval explain trace 完整字段**（task-4.2 接力丰富 `reason` / matched_terms 字段；本 task 仅 placeholder）
- **跨 collection 联邦查询**（v0.1 每个 Retriever 单 collection；多 collection 联邦留 Phase 6 daemon 编排）
- **hybrid embedding / reranker / vector**（P1 — Phase 5+ ；ADR-002 已抽象 provider）
- **写操作 / 索引更新**（read-only 模块；任何写都走 task-2.4 IndexSession）
- **gRPC / REST / MCP 暴露**（task-6.2 / 7.1）
- **explain JSON / CLI 调试入口**（task-4.2）

## 4. Users / Actors

- **task-2.4 indexer**（上游，✅ done）：写入数据，Retriever 只读消费同一 `[data_dir]/collections/[id]/` 数据
- **task-4.2 explain**（下游，强依赖）：基于本 task 的 `SearchResult` 加 reason / matched_terms 详细解释
- **task-6.1 CLI `contextforge search`**（下游）：CLI 调用 Retriever::search → 终端展示 Top-K
- **task-6.2 REST API `POST /v1/search`**（下游）：HTTP handler 把请求 body 映射到 `SearchOptions` → 调 Retriever → 序列化 `SearchResult` 到 PRD §search response 契约
- **task-7.1 MCP server `context_search`** tool（下游）：MCP tool handler 同 REST 形态
- **task-8.1 eval-harness**（下游）：跑 recall eval 用 Retriever 作为黑盒 + 测 P95 性能（AC4 回归）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Decisions Log D2 / §Constraints 性能 / §Technical Approach REST/MCP search 契约）
- `docs/specs/phases/phase-4-retrieval-explain.md`
- `docs/specs/tasks/task-2.4-indexer.md`
- `docs/decisions/adr-002-sqlite-tantivy-layered-storage.md`
- `test/features/retriever.feature`

### 5.2 Imports

- **标库**：`std::path::{Path, PathBuf}` / `std::collections::HashMap` / `std::fmt`
- **内部**：本 crate 不直接 `use` task-2.4 indexer 的内部类型；通过 Tantivy / rusqlite 同名 path 打开（schema 由 Tantivy `meta.json` 自携，无须重定义）
- **第三方（已有，主 agent chore PR #23 引入）**：
  - `tantivy = "0.26.1"`（QueryParser + TopDocs::order_by_score / TermQuery / Schema 字段读取）
  - `rusqlite = "0.39.0"` features=`["bundled"]`（read-only Connection + 参数化查询）
- **错误**：`thiserror = "2.0.18"`（已有）
- **R7 严格处理**：本 task **不引入新 crate**（task agent 不修改 `core/Cargo.toml` / `Cargo.lock`）

### 5.3 函数签名

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// 检索会话：read-only 句柄，单 collection 单数据目录。线程安全（rusqlite Connection
/// 非 Send/Sync 故 Retriever 也非；多线程并发各 Retriever 实例）。
pub struct Retriever {
    // 私有字段：sqlite Connection + tantivy::Index + IndexReader + 6 Field handles + config
}

#[derive(Error, Debug)]
pub enum RetrieverError {
    #[error("io: {0}")] Io(#[from] std::io::Error),
    #[error("sqlite: {0}")] Sqlite(String),
    #[error("tantivy: {0}")] Tantivy(String),
    #[error("invalid config: {0}")] InvalidConfig(String),
    #[error("collection not found: {0}")] CollectionNotFound(String),
}

/// 检索配置（AC5 tokenizer / boost / exact phrase）.
#[derive(Debug, Clone)]
pub struct RetrieverConfig {
    /// Tokenizer 名（v0.1 仅 "default"；CJK / n-gram 留 PRD §O11 接入点）
    pub tokenizer: String,
    /// 字段 boost — 默认 {file_path: 2.0, content: 1.0}
    pub field_boosts: HashMap<String, f32>,
    /// 是否启用 exact phrase（QueryParser `"..."` 语法；v0.1 默认 true）
    pub enable_exact_phrase: bool,
}

impl Default for RetrieverConfig { fn default() -> Self; }

/// 检索请求（与 PRD §REST/MCP search 请求契约对齐）.
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub query: String,
    pub top_k: usize,                  // 默认 10
    pub filters: SearchFilters,
    pub explain: bool,                 // true = 填 reason / matched_terms (task-4.2 接力)
}

/// 过滤契约（与 PRD §search 请求 filters 字段一致）.
/// v0.1 实现：language / collection / time_range；source_type / agent_scope no-op (§10 schema gap)
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub language: Vec<String>,         // ✅ v0.1 生效（Tantivy STRING field）
    pub source_type: Vec<String>,      // ⚠️ v0.1 no-op (indexer 未存；§10 schema gap)
    pub collection: Vec<String>,       // ✅ v0.1 生效但当前 Retriever 单 collection (跨 coll 联邦留 Phase 6)
    pub agent_scope: Vec<String>,      // ⚠️ v0.1 no-op (indexer 未存)
    pub time_after_unix: Option<i64>,  // ✅ v0.1 生效（SQLite chunks.indexed_at 联表）
    pub time_before_unix: Option<i64>, // ✅ v0.1 生效
}

/// 检索结果（与 PRD §REST/MCP search response 契约对齐）.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub chunk_id: String,
    pub file_path: String,
    pub line_start: u64,
    pub line_end: u64,
    pub language: String,
    pub content: String,
    pub score: f32,
    pub retrieval_method: String,      // v0.1 = "bm25" (future: "bm25+embedding")
    pub reason: Option<String>,        // v0.1 placeholder, task-4.2 enriches
    pub matched_terms: Vec<String>,    // v0.1 placeholder, task-4.2 enriches
}

impl Retriever {
    /// 打开（read-only）：连同一 task-2.4 数据目录 `[data_dir]/collections/[id]/{metadata.sqlite, tantivy/}`.
    /// 错误：路径不存在 → CollectionNotFound；SQLite/Tantivy open 失败 → 对应 enum.
    pub fn open(data_dir: &Path, collection_id: &str) -> Result<Self, RetrieverError>;
    pub fn open_with_config(data_dir: &Path, collection_id: &str, config: RetrieverConfig) -> Result<Self, RetrieverError>;

    /// 主检索入口（AC1/AC2/AC3/AC5）.
    /// AC3 防御：空 query / TrimSpace empty / QueryParserError → Ok(vec![]) (不 panic).
    pub fn search(&self, opts: &SearchOptions) -> Result<Vec<SearchResult>, RetrieverError>;

    /// 暴露配置（用于诊断）
    pub fn config(&self) -> &RetrieverConfig;
}
```

**字段映射约定**（Tantivy `meta.json` schema 由 task-2.4 frozen）：

| Tantivy field | 类型 | retriever 用途 |
|---|---|---|
| `chunk_id` | STRING (STORED+INDEXED) | 命中后回查 SQLite metadata |
| `content` | TEXT (STORED+INDEXED) | BM25 全文主字段，默认 boost=1.0 |
| `file_path` | STRING (STORED+INDEXED) | path/filename boost=2.0 (AC5) + filter 锚点 |
| `language` | STRING (STORED+INDEXED) | AC2 filter Term query |
| `line_start` | I64 (STORED+INDEXED+FAST) | 结果字段 |
| `line_end` | I64 (STORED+INDEXED+FAST) | 结果字段 |

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Decisions Log D2): BM25 全文检索 + metadata 检索在 Tantivy+SQLite 索引上可返回 Top-K（v0.1 P0，不依赖向量/embedding）。
- [ ] **AC2** (PRD §Technical Approach REST/MCP 契约): filter 支持 source_type / language / collection / agent_scope / time，与 search 请求契约一致。
- [ ] **AC3** (PRD §Implementation Phases Phase 4 Exit Criteria): 错误/空 query 返回空结果，不 panic。
- [ ] **AC4** (PRD §Constraints 性能 / §Success Metrics 次指标): 已索引、未调 embedding/reranker/远程 时 10 万 chunk 内 BM25/metadata/filter P95 < 500ms（基准在 Phase 8 回归）。
- [ ] **AC5** (PRD §Technical Risks R8): 支持 configurable tokenizer + path/filename/symbol 单独 field 并 boost + exact phrase/symbol search 接口（CJK-aware/n-gram fallback 接入点）。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 BM25+metadata Top-K | SCEN-4.1.1 | TEST-4.1.1 | - | unit-test | Test Red |
| AC2 filter 契约一致 | SCEN-4.1.2 | TEST-4.1.2 | - | unit-test | Test Red |
| AC3 空/错误 query 不 panic | SCEN-4.1.3 | TEST-4.1.3 | - | unit-test | Test Red |
| AC4 性能 P95<500ms | SCEN-4.1.4 | TEST-4.1.4 | - | unit-test | Test Red |
| AC5 tokenizer/boost/exact | SCEN-4.1.5 | TEST-4.1.5 | - | unit-test | Test Red |

## 8. Risks

- 关联 PRD §Technical Risks **R3**（召回率）+ **R8**（中英文/代码符号检索）：tokenizer 可配置、symbol field boost；分场景 recall eval 在 Phase 8。关联 PRD §Open Questions **O11**。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。

## 10. Completion Notes

- **完成日期**：`<TBD-after-impl>`
- **改动文件**：`<TBD-after-impl>`
- **commit 列表**：`<TBD-after-impl>`
- **§9 Verification 结果**：
  - install: `<TBD-after-impl>`
  - typecheck: `<TBD-after-impl>`
  - unit-test: `<TBD-after-impl>`
- **剩余风险 / 未做项**：`<TBD-after-impl>`
- **下游 task 影响**：`<TBD-after-impl>`
