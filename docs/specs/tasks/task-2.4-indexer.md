# Task `2.4`: `indexer — Tantivy 全文索引 + SQLite metadata/chunk 存储 + 增量更新 + contextforge index`

> ✅ 已过 `/s2v-implement` §2A 前置审核（2026-05-22）：§3/§4/§5.2/§5.3 `<TBD-by-user>` 已清零、§6 AC 经用户审定接受。实时状态以下方 `**Status**` 字段为准；状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: In Progress

**Priority**: P0
**Owner**: tajiaoyezi
**Related Phase**: Phase 2 (index-core)
**Dependencies**: 2.1 (scanner), 2.3 (chunker)

## 1. Background

Phase 2 收口 task：把 scanner→parser→chunker 产物写入 Tantivy 全文索引 + SQLite metadata/chunk 存储，并支持基础增量（PRD §Decisions Log D2 / §Implementation Phases Phase 2）。完整长任务恢复在 Phase 8 硬化。本 task 是 Phase 2 最后一个 task（team §4 Gate 3 phase smoke gate 触发）。

## 2. Goal

`contextforge index ./project` 端到端建立本地 Tantivy 索引 + SQLite（metadata/chunk/provenance）；denylist/allowlist + secret redaction 在索引链路生效；单文件变更触发基础增量更新（< 5s 工程目标）。

## 3. Scope

### In Scope

- 实现 AC1–AC5：≥1000 文件索引 / SQLite + Tantivy 分层查询 / denylist + redaction 链路守住 / 单文件增量 / Phase 2 端到端 smoke
- 消费上游：`scanner::scan_path` 产出 `ScanReport`（含 `ScannedFile.redacted_content` / `redaction_status` / `skipped`）→ `parser::parse_content` → `chunker::chunk_units` → 索引器写双存储
- **分层存储（按 ADR-002）**：
  - SQLite（**真值源**）：3 表 schema —— `chunks`（chunk_id PK / file_path / line_start / line_end / language / content / content_hash / kind / collection_id / indexed_at）/ `files`（file_path PK / content_hash / mtime_unix / indexed_at — 增量追踪 AC4 锚点）/ `provenance`（chunk_id FK / importer / original_path / imported_at / source_modified_at）
  - Tantivy（**全文倒排**）：5 字段 schema —— `chunk_id` (STRING, STORED, INDEXED PK) / `content` (TEXT, STORED, INDEXED) / `file_path` (STRING, STORED, INDEXED) / `language` (STRING, STORED, INDEXED) / `line_start` / `line_end` (i64, STORED)
- **同步策略**：先写 SQLite（事务）后写 Tantivy（best-effort）；Tantivy 失败时记 log warning + 留 SQLite truth，可后续从 SQLite 重建（ADR-002 已规定）
- **增量（AC4）**：indexer 在 `files` 表查 `content_hash` —— 与 chunker 算出的新 `content_hash` 不同 → 删该 file_path 所有旧 chunks + 重插；相同 → skip。单文件路径 partial reindex 目标 < 5s（工程基线，不硬测）
- **denylist + redaction 链路（AC3）**：indexer **只消费** `ScannedFile.redacted_content`（scanner 已做 redact + denylist 跳过）；indexer 不应触碰原始 secret 内容；如 `ScannedFile.content` 为 None 或 `redaction_status` 异常 → skip + log
- **AC5 smoke**：`core/tests/phase2_smoke.rs` 含 `#[test] fn phase_2_end_to_end_smoke()`（用户 §2A 决策）；被 `cargo test --workspace` 自动收纳；主 agent §4 Gate 3 可 `cargo test --test phase2_smoke` 精准抓
- **数据目录布局**：`<data_dir>/collections/<collection_id>/{metadata.sqlite, tantivy/}`（PRD §Local data directory v0.1 已规定；本 task 实施落地）
- 文件锚点：`core/src/indexer/mod.rs`（在 task-1.3 placeholder 上实现）+ `core/tests/phase2_smoke.rs`（新增集成测试，AC5）

### Out Of Scope

- **embedding / 向量索引**（P1 — Phase 4 retriever 接 hybrid search；本 task 仅 BM25 baseline）
- **Tantivy tokenizer 高级调优 / CJK 分词 / 同义词扩展**（PRD §R8 / Phase 4 retriever 调优）
- **后台长任务恢复 / 中断点恢复 / 进度上报**（Phase 8 性能硬化；本 task 同步阻塞实现）
- **collection lifecycle 全套**（创建 / 删除 / 重命名 — task-5.x memoryops 负责）
- **REST/MCP/gRPC 暴露 indexer**（Phase 6 / 7 — 本 task 仅 Rust API + Rust smoke）
- **跨 collection 联合查询**（Phase 4 retriever）
- **真实大仓库性能压测**（Phase 8 R6 缓解；本 task 测试用 ≥1000 合成 fixture 满足 AC1 基线）
- **schema 演进 / migration 工具**（v0.1 schema 即 frozen for v0.1；v0.2 时另起 migration task）
- **完整 long-running daemon 集成 / file watcher**（Phase 8 / task-6.x daemon 编排）

## 4. Users / Actors

- **scanner** (task-2.1, ✅ done, 上游)：通过 `scan_path` 提供 `ScanReport`（含 redacted_content / skipped denylist）
- **parser** (task-2.2, ✅ done, 上游)：通过 `parse_content` 提供 `Vec<ParsedUnit>`
- **chunker** (task-2.3, ✅ done, 上游)：通过 `chunk_units` 提供 `Vec<Chunk>` 含 sha256 algo-prefixed `content_hash`
- **`contextforge index [path]` CLI 命令**（task-6.x 实现 CLI 编排，本 task 提供 `IndexSession::index_path` 入口）
- **retriever** (Phase 4, 下游)：消费 Tantivy 全文索引 + SQLite metadata 联合查询；本 task 冻结存储 schema
- **memoryops** (Phase 5, 下游)：基于 `chunks.content_hash` 做去重 / 跨 collection 治理；本 task 在 SQLite 索引 content_hash 字段
- **secret redaction 链路** (AC3 桥)：indexer 是最末环 — 不引入新 redaction 逻辑，只 verify `ScannedFile.redaction_status` 后消费 `redacted_content`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Decisions Log D2 / §Constraints 性能 / §Implementation Phases Phase 2 Exit Criteria）
- `docs/specs/phases/phase-2-index-core.md`
- `docs/specs/tasks/task-2.1-scanner.md`
- `docs/specs/tasks/task-2.3-chunker.md`
- `docs/decisions/adr-002-sqlite-tantivy-layered-storage.md`
- `test/features/indexer.feature`

### 5.2 Imports

- **标库**：`std::path::{Path, PathBuf}` / `std::fs` / `std::time::SystemTime` / `std::collections::HashMap` / `std::fmt::Write`
- **内部**：`crate::scanner::{scan_path, ScanOptions, ScanReport, ScannedFile, RedactionStatus}` / `crate::parser::{parse_content, ParsedUnit}` / `crate::chunker::{chunk_units, Chunk, ChunkPolicy, Provenance, content_hash}`
- **错误类型**：复用 `thiserror = "2.0.18"`（task-2.2 chore PR#11，已在 core/Cargo.toml）
- **R7 NEW deps（独立 chore-dep PR 引入，本 task 不 fold-in）**：
  - `tantivy = "0.22"`（全文倒排索引引擎；BM25 默认评分；STRING/TEXT/i64 schema 字段类型；本 task 用 sync API + RAMDirectory tests + MmapDirectory 生产）
  - `rusqlite = "0.32"` with feature `bundled`（SQLite 绑定；`bundled` 编译 sqlite-amalgamation 进 binary，避免系统 libsqlite3-dev 依赖 — PRD §Constraints 本地优先 + 跨平台便携）
- **不引入**：r2d2（连接池，v0.1 单进程单连接即可）/ serde_rusqlite（手动 row 映射 ~20 行，避免反序列化开销）
- 详细版本评估见 `NEEDS-DEP-task-2.4.md`（主 agent chore PR 域）

### 5.3 函数签名

```rust
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::scanner::{ScanOptions, ScanReport};
use crate::chunker::{Chunk, ChunkPolicy, Provenance};

/// 索引会话：单 collection 单数据目录。生命周期管理 SQLite 连接 + Tantivy IndexWriter。
pub struct IndexSession {
    // 内部：SQLite 连接 + Tantivy Index + IndexWriter + collection_id + data_dir
    // 字段全部私有，仅通过下列公开方法访问
}

#[derive(Error, Debug)]
pub enum IndexError {
    #[error("io: {0}")] Io(#[from] std::io::Error),
    #[error("sqlite: {0}")] Sqlite(String),     // 包装 rusqlite::Error
    #[error("tantivy: {0}")] Tantivy(String),   // 包装 tantivy::TantivyError
    #[error("scan: {0}")] Scan(String),         // 包装 ScanError.to_string()
    #[error("parse: {0}")] Parse(String),
    #[error("chunk: {0}")] Chunk(String),
    #[error("redaction status unsafe for indexing: {0:?}")] UnsafeRedaction(crate::scanner::RedactionStatus),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexStats {
    pub files_indexed: usize,
    pub files_skipped_denied: usize,
    pub files_skipped_redaction: usize,
    pub chunks_written: usize,
    pub chunks_updated: usize,    // 增量
    pub chunks_deleted: usize,    // 增量
}

impl IndexSession {
    /// 打开（或创建）索引会话；自动 ensure `<data_dir>/collections/<collection_id>/` +
    /// 创建 SQLite schema（3 表 + 索引）+ 打开 / 创建 Tantivy index。
    pub fn open(data_dir: &Path, collection_id: &str) -> Result<Self, IndexError>;

    /// 全量索引：scan root → for each ScannedFile（已 redact + 跳 denylist）→ parse → chunk → write SQLite+Tantivy。
    /// 返回累计统计；遇致命错（SQLite 写失败）回滚未提交事务并返回 Err。
    pub fn index_path(&mut self, root: &Path, scan_options: &ScanOptions, policy: &ChunkPolicy, provenance: Vec<Provenance>) -> Result<IndexStats, IndexError>;

    /// 增量：对单文件 partial reindex（AC4）— 比对 files.content_hash → 不同则删旧 chunks 重插；相同 skip。
    /// 工程目标 < 5s/file（非硬测）。
    pub fn reindex_file(&mut self, path: &Path, scan_options: &ScanOptions, policy: &ChunkPolicy, provenance: Vec<Provenance>) -> Result<IndexStats, IndexError>;

    /// 提交 Tantivy IndexWriter pending writes（commit）；SQLite 已在事务内 commit。
    pub fn commit(&mut self) -> Result<(), IndexError>;

    /// 查询 SQLite chunks 表行数（AC2 testing helper）。
    pub fn sqlite_chunk_count(&self) -> Result<u64, IndexError>;

    /// 用 Tantivy 跑全文查询（AC2 testing helper）；返回命中 chunk_id 列表。
    pub fn tantivy_search(&self, query: &str, limit: usize) -> Result<Vec<String>, IndexError>;
}
```

**SQLite schema（`open` 时通过 `CREATE TABLE IF NOT EXISTS` 落地）**：

```sql
CREATE TABLE IF NOT EXISTS chunks (
    chunk_id      TEXT PRIMARY KEY,
    file_path     TEXT NOT NULL,
    line_start    INTEGER NOT NULL,
    line_end      INTEGER NOT NULL,
    language      TEXT,
    content       TEXT NOT NULL,
    content_hash  TEXT NOT NULL,
    kind          TEXT,
    collection_id TEXT NOT NULL,
    indexed_at    TEXT NOT NULL                 -- RFC3339
);
CREATE INDEX IF NOT EXISTS idx_chunks_file_path     ON chunks(file_path);
CREATE INDEX IF NOT EXISTS idx_chunks_content_hash  ON chunks(content_hash);

CREATE TABLE IF NOT EXISTS files (
    file_path     TEXT PRIMARY KEY,
    content_hash  TEXT NOT NULL,                 -- 文件级 sha256 (整体 vs chunker chunk-level hash)
    mtime_unix    INTEGER NOT NULL,              -- AC4 增量锚点
    indexed_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS provenance (
    chunk_id           TEXT NOT NULL,
    importer           TEXT NOT NULL,
    original_path      TEXT NOT NULL,
    imported_at        TEXT NOT NULL,
    source_modified_at TEXT,
    FOREIGN KEY (chunk_id) REFERENCES chunks(chunk_id) ON DELETE CASCADE
);
```

**Tantivy schema（`open` 时构造）**：

| 字段 | 类型 | flags | 用途 |
|---|---|---|---|
| chunk_id | STRING | STORED + INDEXED | PK / 删除锚点 / 与 SQLite 联表 |
| content | TEXT | STORED + INDEXED | 全文搜（BM25 默认 tokenizer）|
| file_path | STRING | STORED + INDEXED | 按文件过滤 / 增量删除 |
| language | STRING | STORED + INDEXED | 按语言过滤 |
| line_start | I64 | STORED | 结果回放原文位置 |
| line_end | I64 | STORED | 结果回放原文位置 |

**集成测试**：`core/tests/phase2_smoke.rs` 含 `#[test] fn phase_2_end_to_end_smoke()`（用户 §2A 决策；AC5 入口）— 主 agent §4 Gate 3 可 `cargo test --test phase2_smoke` 精准抓。

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Implementation Phases Phase 2 Exit Criteria): `contextforge index ./sample_project` 能索引 ≥ 1000 个文件。
- [ ] **AC2** (PRD §Decisions Log D2): SQLite 存 metadata/chunk/provenance 可查询；Tantivy 全文可搜索到基础结果。
- [ ] **AC3** (PRD §Implementation Phases Phase 2 Exit Criteria): 索引链路尊重 denylist + secret redaction（denylist 路径不入索引、secret 已 redact）。
- [ ] **AC4** (PRD §Constraints 性能 / Phase 2 Exit Criteria): 单文件变更触发基础增量更新（工程目标 < 5s；不重建全量）。
- [ ] **AC5** (本 task 新增): Phase 2 端到端 smoke 可执行（index fixture → SQLite chunk 计数 + Tantivy 命中 + secret fixture 已 redact），为 phase spec §6 端到端 smoke 提供落点。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 索引 ≥1000 文件 | SCEN-2.4.1 | TEST-2.4.1 | - | unit-test | Test Red |
| AC2 SQLite+Tantivy 可查 | SCEN-2.4.2 | TEST-2.4.2 | - | unit-test | Test Red |
| AC3 denylist+redaction 生效 | SCEN-2.4.3 | TEST-2.4.3 | - | unit-test | Test Red |
| AC4 基础增量更新 | SCEN-2.4.4 | TEST-2.4.4 | - | unit-test | Test Red |
| AC5 Phase2 端到端 smoke | SCEN-2.4.5 | TEST-2.4.5 | core/tests/phase2_smoke.rs | unit-test | Test Red |

## 8. Risks

- 关联 PRD §Technical Risks **R6**（大仓库索引性能/资源）：以真实大仓库基准持续测；超阈值降级后台长任务（完整硬化 Phase 8）。
- 关联 **R4**（redaction 在索引链路不可被绕过）。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。⚠️ 本 task 为 Phase 2 最后 task：完工/合并前 phase spec §6 端到端 smoke 必须填实（C1 / team §4 Gate 3）。

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
