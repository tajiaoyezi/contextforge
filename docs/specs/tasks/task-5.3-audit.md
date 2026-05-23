# Task `5.3`: `audit — 审计事件 + audit log`

> ✅ 已过 `/s2v-implement` §2A 前置审核（2026-05-23）：§3/§4/§5.2/§5.3 的待定字段已清零；决策为嵌入 collection SQLite `audit_log` 表、默认仅记录脱敏元数据、Phase 5 smoke 落 `core/tests/phase5_smoke.rs`，task-5.2 stale API 合并前 smoke 使用局部 stub 标明衔接点。实施硬约束：不改 `proto/`，不改依赖/lockfile，audit log 不记录完整 query / secret / export content。实时状态以下方 `**Status**` 字段为准；状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: Ready

**Priority**: P0
**Owner**: codex
**Related Phase**: Phase 5 (memoryops)
**Dependencies**: 5.1 (dedup)

## 1. Background

可审计性是 PRD 隐私基线一部分（PRD §Constraints 安全 + Local service security baseline / §Decisions Log D4）。本 task 实现 import/search/export/redact 等关键事件写 audit log，且 audit log 不记录完整 secret/导出内容。是 Phase 5 末批 task（与 5.2 并列）。

## 2. Goal

`memoryops` 能为 import / search / export / redact / delete 关键事件产出审计事件并写入 collection `audit.log`；默认记录 operation/collection/source/result_count/redaction_count/timestamp，**不**记录完整 query content / 完整 secret / 完整导出内容。

## 3. Scope

### In Scope

- 在 Rust data-plane `core/src/memoryops/` 新增 audit 能力，使用 collection 现有 `metadata.sqlite` 内的 `audit_log` 表记录审计事件；不新增依赖、不拆单独 DB。
- 支持 import / search / export / redact 四类事件写入，默认字段为 operation / collection / source / result_count / redaction_count / timestamp。
- 对敏感上下文字段只写脱敏元数据：query 仅写 hash 和长度；secret 仅写 `[REDACTED:TYPE]` 标签；export 仅写 chunk_id 列表和总字节数。
- 暴露 scanner override 审计 helper，供用户显式覆盖 scanner redaction / denylist 保护时写入 redact 事件。
- 新增 `core/tests/phase5_smoke.rs` 作为 Phase 5 Gate 3 精准 smoke 入口；在 task-5.2 merge 前使用测试内 stale stub，后续 rebase 后替换为真实 lifecycle API。

### Out Of Scope

- CLI / REST / MCP 层的 audit 查询、展示、筛选、分页与权限控制。
- 单独 JSONL audit 文件、单独 audit.db、log rotation、retention policy、跨 collection 全局审计汇总。
- exporter 正式导出实现、export 二次 secret scan 的完整产品 wiring（本 task 只验证 audit 记录不泄露导出内容）。
- task-5.2 的生产 stale / conflict API 实现，以及 task-6 之后的 search/export 用户命令 wiring。
- 修改 `proto/contextforge/v1/*`、`Cargo.toml`、`Cargo.lock`、`go.mod`、`go.sum`。

## 4. Users / Actors

- MemoryOps 调度器：在 import / search / export / redact 关键操作后写审计事件。
- Scanner / indexer / retriever / exporter 调用方：通过 audit helper 记录安全相关操作结果。
- 本地优先 / 隐私敏感开发者：依赖 audit log 可追溯关键操作，同时 audit log 本身不泄露敏感内容。
- Phase 5 Gate 3 主 agent：通过 `cargo test --test phase5_smoke` 验证去重、stale 衔接点与 audit 脱敏闭环。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Constraints 安全 + Local service security baseline / §Decisions Log D4）
- `docs/specs/phases/phase-5-memoryops.md`
- `docs/specs/tasks/task-5.1-dedup.md`
- `docs/specs/tasks/task-2.1-scanner.md`
- `docs/decisions/adr-004-local-first-privacy-baseline.md`
- `test/features/memoryops.feature`

### 5.2 Imports

- Rust 标准库：`std::fs` / `std::path::{Path, PathBuf}` / `std::time::{SystemTime, UNIX_EPOCH}` / `std::fmt`。
- 既有直接依赖：`rusqlite::{params, Connection}`、`sha2::{Digest, Sha256}`。
- 上游 Rust 模块（只读消费）：`crate::scanner`（redaction labels / scanner override 场景）、`crate::indexer::IndexSession`、`crate::retriever::Retriever`、`crate::chunker::{ChunkPolicy, Provenance}`（Phase 5 smoke）。
- 上游 Go task-5.1 语义（只读契约）：exact duplicate 去重按 content_hash，provenance 链合并不丢来源；Rust phase smoke 用测试内 fixture/stub 验证该联动，不复制 Go 生产包。

### 5.3 函数签名

> Rust crate `contextforge_core::memoryops::audit`，落 `core/src/memoryops/audit.rs`。Phase 5 smoke 落 `core/tests/phase5_smoke.rs`。

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOperation {
    Import,
    Search,
    Export,
    Redact,
}

impl AuditOperation {
    pub fn as_str(self) -> &'static str;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    pub operation: AuditOperation,
    pub collection: String,
    pub source: String,
    pub result_count: u64,
    pub redaction_count: u64,
    pub query: Option<String>,
    pub redacted_terms: Vec<String>,
    pub chunk_ids: Vec<String>,
    pub export_total_byte_count: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditLogEntry {
    pub id: i64,
    pub operation: String,
    pub collection: String,
    pub source: String,
    pub result_count: u64,
    pub redaction_count: u64,
    pub timestamp: String,
    pub query_hash: Option<String>,
    pub query_length: Option<u64>,
    pub redacted_terms: Vec<String>,
    pub chunk_ids: Vec<String>,
    pub export_total_byte_count: Option<u64>,
}

#[derive(Debug)]
pub enum AuditError {
    Io(std::io::Error),
    Sqlite(String),
    InvalidEvent(String),
}

pub struct AuditSink;

impl AuditSink {
    pub fn open(data_dir: impl AsRef<std::path::Path>, collection: &str) -> Result<Self, AuditError>;
    pub fn record(&mut self, event: AuditEvent) -> Result<AuditLogEntry, AuditError>;
    pub fn list(&self) -> Result<Vec<AuditLogEntry>, AuditError>;
    pub fn count_by_operation(&self, operation: AuditOperation) -> Result<u64, AuditError>;
}

pub fn import_event(collection: &str, source: &str, result_count: u64, redaction_count: u64) -> AuditEvent;
pub fn search_event(collection: &str, source: &str, query: &str, result_count: u64, redaction_count: u64) -> AuditEvent;
pub fn export_event(collection: &str, source: &str, chunk_ids: Vec<String>, total_byte_count: u64, redaction_count: u64) -> AuditEvent;
pub fn redact_event(collection: &str, source: &str, redacted_terms: Vec<String>, redaction_count: u64) -> AuditEvent;
pub fn scanner_override_event(collection: &str, source: &str, redacted_terms: Vec<String>, redaction_count: u64) -> AuditEvent;
```

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Implementation Phases Phase 5 Exit Criteria): import / search / export / redact 事件能写入 collection `audit.log`。
- [ ] **AC2** (PRD §Constraints Local service security baseline): audit log 默认记录 operation/collection/source/result_count/redaction_count/timestamp，**不**默认记录完整 query content。
- [ ] **AC3** (PRD §Constraints 安全): audit log **不**记录完整 secret、**不**记录完整导出内容。
- [ ] **AC4** (PRD §Technical Risks R4): scanner secret override（task 2.1 AC4 关联）发生时必须写 audit log（可追溯）。
- [ ] **AC5** (本 task 新增): Phase 5 端到端 smoke 可执行（导入含重复事实 fixture → 去重+provenance 合并 + stale 可标记可检索 + audit.log 含四类事件且无完整 secret），为 phase spec §6 端到端 smoke 提供落点。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 四类事件写 audit.log | SCEN-5.3.1 | TEST-5.3.1 | - | unit-test | Not Started |
| AC2 默认字段不含 query 全文 | SCEN-5.3.2 | TEST-5.3.2 | - | unit-test | Not Started |
| AC3 不记录完整 secret/导出 | SCEN-5.3.3 | TEST-5.3.3 | - | unit-test | Not Started |
| AC4 secret override 写 audit | SCEN-5.3.4 | TEST-5.3.4 | - | unit-test | Not Started |
| AC5 Phase5 端到端 smoke | SCEN-5.3.5 | TEST-5.3.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R4**（redaction 漏检/误报）：audit log 提供可追溯性但本身不得泄露 secret。关联 PRD §Open Questions **O7 / O10**（威胁模型 / API 安全边界）。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。⚠️ 本 task 与 5.2 为 Phase 5 末批：Phase 5 最后合并的 task 完工前 phase spec §6 端到端 smoke 必须填实（C1 / team §4 Gate 3）。

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
