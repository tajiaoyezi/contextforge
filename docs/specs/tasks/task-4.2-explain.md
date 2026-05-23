# Task `4.2`: `explain — explainable retrieval trace + result schema`

> ✅ 已过 `/s2v-implement` §2A 前置审核（2026-05-23）：§3/§4/§5.2/§5.3 `<TBD-by-user>` 已清零、§6 AC 经用户审定接受、AC1/AC3/AC4 schema-gap 三决策已确认（详见 §10 §2A Decisions）。实时状态以下方 `**Status**` 字段为准；状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: Done

**Priority**: P0
**Owner**: tajiaoyezi
**Related Phase**: Phase 4 (retrieval-explain)
**Dependencies**: 4.1 (retriever)

## 1. Background

可解释检索是 ContextForge 一等公民与核心差异（PRD §Core Capabilities #2 / §Vision 关键差异）。本 task 在 retriever 之上产出可解释 result：每条结果带来源/位置/打分/召回方式/理由/scope，并产出 retrieval trace。是 Phase 4 最后一个 task（team §4 Gate 3 触发）。

## 2. Goal

检索结果按 PRD §Technical Approach search response 契约带 `chunk_id/context_id/source_type/file_path/line_start/line_end/score/retrieval_method/reason/agent_scope/redaction_status/provenance`；可输出 retrieval trace（为何召回：命中词/方式/分数）；可经内部 gRPC Search API / `contextforge search` 调试入口验证。

## 3. Scope

### In Scope

- 扩展 task-4.1 `SearchResult` 为 12-field 可解释契约（AC1 / PRD §Technical Approach REST/MCP search response / proto `contextforge.v1.RetrievalResult` 单源对齐）：新增 `context_id` / `source_type` / `agent_scope` / `redaction_status` / `provenance` 5 字段；保留 task-4.1 已有 7 字段（`chunk_id` / `file_path` / `line_start` / `line_end` / `score` / `retrieval_method` / `reason`）；保留非 AC1 内部扩展字段（`language` / `content` / `matched_terms` — 下游 CLI/REST 消费方便用）。
- 引入 `retriever::Provenance` 类型（直接 `use crate::chunker::Provenance` 复用 — 与 indexer provenance 表 / proto `Provenance` 字段集一一对应，DRY）。
- **Provenance 合成（AC3 黑盒守护）**：对每条命中结果，若 SQLite `provenance` 表有匹配 chunk_id 行 → 拼全部 importer 行；否则合成 default `[{importer: "scanner", original_path: file_path, imported_at: indexed_at, source_modified_at: ""}]` 保证 `provenance.len() ≥ 1`。
- `reason` 类型从 `Option<String>` 改为 `String`（proto parity，proto3 string 字段非 optional 而是空串默认）；`explain=false` 时 `reason=""`，`explain=true` 时填实「`bm25 hit on '<query>'; matched terms: [<terms>]`」+ 同步 `matched_terms` 非空（task-4.1 已留 placeholder，本 task 接力）。
- 新增 `Retriever::explain(opts)` 公开方法（AC4 v0.1 调试入口 — 强制 `explain=true` 等价 `search(opts.clone() with explain=true)`，让 CLI/REST 调试场景一键拿全可解释字段；CLI/REST/MCP 在 Phase 6/7 wrap 本方法）。
- v0.1 schema-gap 默认值常量（in retriever 模块内）：`context_id=""` / `source_type=""` / `agent_scope=vec![]` / `redaction_status="applied"`（indexer per BINDING comment 仅消费 `redacted_content`，进入索引的内容默认安全 = applied）。
- 新增 5 个 RED→GREEN 测试 `TEST-4.2.1 ~ TEST-4.2.4`（in `core/src/retriever/mod.rs` `#[cfg(test)] mod tests`）+ `TEST-4.2.5`（AC5 Phase 4 端到端 smoke = `core/tests/phase4_smoke.rs` 内 `#[test] fn phase_4_end_to_end_smoke()` — pattern 与 task-2.4 phase2_smoke 一致，主 agent §4 Gate 3 `cargo test --test phase4_smoke` 精确抓）。
- 填实 `test/features/retriever.feature` 中 SCEN-4.2.1 ~ SCEN-4.2.5 占位 Given/When/Then。

### Out Of Scope

- **真实存** `context_id` / `source_type` / `agent_scope` / `redaction_status` 到 indexer schema（task-4.1 §10 已留 schema gap；未来 SPEC-DRIFT-task-2.4 chore-spec PR 扩 SQLite chunks + Tantivy schema + 反向回填 — 完成后 retriever 自动 fill 真实值）。
- **gRPC `ContextService::Search` tonic server 实现**（v0.1 调试入口 = Rust `Retriever::explain` public API；真实 gRPC server 留 task-6.2 REST API 一并 wire tonic wrapper；proto 已 frozen 在 task-1.1 / phase23-start-gate 禁改）。
- **`contextforge search` Go CLI 命令**（留 task-6.1 cli-search — CLI wrap `Retriever::explain` 终端展示）。
- **MCP `context_search` tool 实现**（留 task-7.1 mcp-server — MCP wrap 同 REST 形态）。
- **AC3 populated coverage ≥ 90%（真实数据语义）**：v0.1 量化为 schema coverage 100%（struct 强制 12 字段 PRESENT，远超 90%）+ 反指标 `provenance.len() ≥ 1`（黑盒守护）。真实数据 populated coverage（5 schema-gap 字段非空）留 SPEC-DRIFT-task-2.4 完成后由 task-8.1 eval-harness 回归。
- **跨 collection 联邦查询** / **hybrid embedding / reranker / vector**（v0.1 P0 不依赖；ADR-002 已抽象 provider；P1 Phase 5+）。
- **AC4 性能压测**（本 task 不跑大规模 benchmark；架构支持即可；task-8.1 eval-harness 回归）。

## 4. Users / Actors

- **task-4.1 retriever**（上游，✅ done — SHA 已 merge 到 master）：本 task 扩展 task-4.1 `SearchResult` schema；task-4.1 不需回归（向下兼容 — 7 字段保留 + 5 字段新增 + reason 类型变 `Option<String>`→`String`，仅 task-4.1 自身 5 个 unit test 在同 worktree 同 commit 内同步更新）。
- **task-6.1 CLI `contextforge search`**（下游强依赖）：CLI 调用 `Retriever::explain(opts)` → 终端展示 12 字段可解释结果。
- **task-6.2 REST API `POST /v1/search`**（下游强依赖）：HTTP handler 把 request body 映射到 `SearchOptions` → 调 `Retriever::explain` → 序列化 `SearchResult` 为 PRD §search response JSON / proto `RetrievalResult`（tonic codegen 已 done in task-1.1）。
- **task-7.1 MCP `context_search` tool**（下游强依赖）：MCP tool handler 同 REST 形态。
- **task-8.1 eval-harness**（下游）：跑 recall eval + AC3 schema 覆盖率回归 + 未来 populated coverage 回归。
- **未来 SPEC-DRIFT-task-2.4 chore-spec PR**（前置软依赖）：扩 indexer SQLite chunks 表 + Tantivy schema 让 `context_id` / `source_type` / `agent_scope` / `redaction_status` 真实有效；reverse-fill 后 retriever 不需改即可自动 fill 真实值（仅 v0.1 default 常量逻辑会被 SQLite JOIN 真值覆盖）。
- **PRD §Success Metrics 次指标 + 反指标消费者**（业务消费）：「可解释性覆盖率 ≥ 90%」+「不能为命中率牺牲可解释性」由本 task schema 强制保障。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Core Capabilities #2 / §Technical Approach REST/MCP search response / §Success Metrics 可解释性覆盖率）
- `docs/specs/phases/phase-4-retrieval-explain.md`
- `docs/specs/tasks/task-4.1-retriever.md`
- `docs/specs/tasks/task-1.1-proto.md`
- `docs/decisions/adr-003-cli-rest-mcp-grpc-interfaces.md`
- `test/features/retriever.feature`

### 5.2 Imports

- **标库**：`std::collections::HashMap` / `std::path::{Path, PathBuf}` （沿用 task-4.1）
- **内部**：`use crate::chunker::Provenance`（DRY — 与 indexer / chunker 已用同一类型；4 字段集 `importer` / `original_path` / `imported_at` / `source_modified_at` 与 proto `Provenance` 一一对应）
- **第三方（已有）**：`tantivy = "0.26.1"` / `rusqlite = "0.39.0"` features=`["bundled"]` / `thiserror = "2.0.18"` — **不引入新 crate**（R7 严格通道；task agent 不修改 `core/Cargo.toml` / `Cargo.lock`）

### 5.3 函数签名

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::chunker::Provenance; // DRY — 与 indexer / chunker / proto 同一类型

/// SearchResult — explainable retrieval result（task-4.2 升级版）.
///
/// 12-field explainable contract per AC1 + PRD §Technical Approach REST/MCP search response
/// + proto `contextforge.v1.RetrievalResult`（ADR-003 单源 schema unity）.
///
/// v0.1 schema gap（继承 task-4.1 §10）：`context_id` / `source_type` / `agent_scope` /
/// `redaction_status` 不在 task-2.4 indexer SQLite/Tantivy schema → 返回 v0.1 default 常量；
/// `provenance` 优先 JOIN indexer provenance 表，缺失则合成 scanner-default 保证 ≥1 entry
/// (AC3 黑盒守护)。SPEC-DRIFT-task-2.4 chore-spec PR 未来 reverse-fill 后真实生效。
#[derive(Debug, Clone)]
pub struct SearchResult {
    // ---- 12 explainable fields (AC1) ----
    pub chunk_id: String,
    pub context_id: String,           // v0.1 default ""（schema gap）
    pub source_type: String,          // v0.1 default ""（schema gap）
    pub file_path: String,
    pub line_start: u64,
    pub line_end: u64,
    pub score: f32,
    pub retrieval_method: String,     // v0.1 = "bm25"（future: "bm25+embedding"）
    pub reason: String,               // explain=false → ""；explain=true → "bm25 hit on '<q>'; matched terms: [...]"
    pub agent_scope: Vec<String>,     // v0.1 default vec![]（schema gap）
    pub redaction_status: String,     // v0.1 default "applied"（indexer per BINDING 仅消费 redacted_content）
    pub provenance: Vec<Provenance>,  // AC3 硬底：每条 result.provenance.len() ≥ 1（合成兜底）
    // ---- 非 AC1 内部扩展（下游消费方便）----
    pub language: String,             // 沿用 task-4.1（CLI/REST 终端展示用）
    pub content: String,              // 沿用 task-4.1（CLI/REST 终端展示 / snippet 用）
    pub matched_terms: Vec<String>,   // 沿用 task-4.1 placeholder；本 task explain=true 时 enrich
}

impl Retriever {
    /// 主检索入口（兼容 task-4.1 + 升级 SearchResult schema 12 fields + AC3 黑盒守护）.
    ///
    /// AC3 invariant: 返回的每条 result.provenance.len() ≥ 1（合成 scanner-default 若无真实 row）.
    pub fn search(&self, opts: &SearchOptions) -> Result<Vec<SearchResult>, RetrieverError>;

    /// AC4 v0.1 调试入口 — 等价 search(opts) 但强制 explain=true（reason / matched_terms 填实）.
    /// CLI (task-6.1) / REST (task-6.2) / MCP (task-7.1) 在 Phase 6/7 wrap 本方法.
    pub fn explain(&self, opts: &SearchOptions) -> Result<Vec<SearchResult>, RetrieverError>;

    // ---- task-4.1 沿用 ----
    pub fn open(data_dir: &Path, collection_id: &str) -> Result<Self, RetrieverError>;
    pub fn open_with_config(data_dir: &Path, collection_id: &str, config: RetrieverConfig) -> Result<Self, RetrieverError>;
    pub fn config(&self) -> &RetrieverConfig;
}
```

**v0.1 default 常量**（in `core/src/retriever/mod.rs`）：

| 常量 | 值 | 来源依据 |
|---|---|---|
| `DEFAULT_CONTEXT_ID` | `""` | task-2.4 indexer 不存 context_id（chunks 表无此列；ContextRecord 在 proto 但 indexer 不写）— schema gap |
| `DEFAULT_SOURCE_TYPE` | `""` | task-2.4 indexer 不存 source_type — schema gap |
| `DEFAULT_AGENT_SCOPE` | `vec![]` | task-2.4 indexer 不存 agent_scope — schema gap |
| `DEFAULT_REDACTION_STATUS` | `"applied"` | task-2.4 indexer per BINDING 仅消费 `ScannedFile.redacted_content`（scanner 已 redact）→ 进入索引内容默认安全；提前对齐 PRD §Constraints 安全基线 |
| `SYNTHESIZED_IMPORTER` | `"scanner"` | 当 provenance 表无 chunk_id 行时（scanner-indexed 而非 importer-imported）合成默认 importer 标识 |

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [x] **AC1** (PRD §Technical Approach REST/MCP search response): 每条结果含 chunk_id/context_id/source_type/file_path/line_start/line_end/score/retrieval_method/reason/agent_scope/redaction_status/provenance。
- [x] **AC2** (PRD §Implementation Phases Phase 4 Exit Criteria): 结果能定位回原始文件和行号（file_path + line_start/line_end 精确）。
- [x] **AC3** (PRD §Success Metrics 次指标 / 反指标): 可解释性覆盖率 ≥ 90% 结果含全部可解释字段；禁止返回无 provenance 的"黑盒高分"结果。
- [x] **AC4** (PRD §Implementation Phases Phase 4 Exit Criteria): 可经内部 gRPC Search API / `contextforge search` 调试入口返回上述可解释结果。
- [x] **AC5** (本 task 新增): Phase 4 端到端 smoke 可执行（索引 fixture → 一组 query 校验每条结果 7+ 可解释字段 + 空 query 不 panic），为 phase spec §6 端到端 smoke 提供落点。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 可解释字段完整 | SCEN-4.2.1 | TEST-4.2.1 | - | unit-test | Done |
| AC2 定位回原文行号 | SCEN-4.2.2 | TEST-4.2.2 | - | unit-test | Done |
| AC3 覆盖率≥90%/禁黑盒 | SCEN-4.2.3 | TEST-4.2.3 | - | unit-test | Done |
| AC4 gRPC/CLI 调试入口 | SCEN-4.2.4 | TEST-4.2.4 | - | unit-test | Done |
| AC5 Phase4 端到端 smoke | SCEN-4.2.5 | TEST-4.2.5 | - | unit-test | Done |

## 8. Risks

- 关联 PRD §Technical Risks **R3**（召回率）：reason/trace 为调参与回归提供依据。反指标硬约束：可解释性不可为命中率牺牲。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。⚠️ 本 task 为 Phase 4 最后 task：完工/合并前 phase spec §6 端到端 smoke 必须填实（C1 / team §4 Gate 3）。

## 10. Completion Notes

- **完成日期**：2026-05-23
- **改动文件**：
  - core/src/retriever/mod.rs（task-4.2 §2A: SearchResult 扩 12-field explainable contract + provenance 合成 helper read_provenance() + matched_terms enrichment helper enrich_matched_terms() + Retriever::explain() v0.1 调试入口 + 4 新 unit tests TEST-4.2.1~4；task-4.1 reason: Option-of-String → String proto parity 向下兼容）
  - core/tests/phase4_smoke.rs（新增 — TEST-4.2.5 phase_4_end_to_end_smoke；pattern 同 core/tests/phase2_smoke.rs；主 agent §4 Gate 3 phase-4 §6 端到端 smoke `cargo test --test phase4_smoke` 触发入口）
  - test/features/retriever.feature（SCEN-4.2.1~5 Given/When/Then 填实）
  - docs/specs/tasks/task-4.2-explain.md（§2A 业务承诺：Status Draft→Ready→In Progress→Done；§3/§4/§5.2/§5.3 §2A 填实；§6 AC1-5 全部勾选；§7 5 行 → Done；§10 终态回填）
- **commit 列表**（本 task 全部 5 个，按时间顺序）：
  - bc4d74b docs(spec): task-4.2 §2A 业务承诺 (Draft → Ready)
  - 08210c2 test(retriever): 加 SCEN-4.2.1~5 共 5 个 RED 测试 + Status: Ready → In Progress
  - 19d9a01 docs(spec): task-4.2 Status Ready → In Progress (RED 已落)
  - 070f736 feat(retriever): 实现 12-field explainable result + provenance 合成 + explain() 通过全部 5 个 task-4.2 测试
  - 本回填 docs(spec) commit（§6/§7/§10 终态 + Status → Done）
- **§9 Verification 结果**：
  - install: ✅ `go mod download && cargo fetch`（无新 deps；沿用 task-4.1 的 tantivy 0.26.1 + rusqlite 0.39.0 bundled + thiserror 2.0.18；R7 严格通道）
  - typecheck: ✅ `go vet ./... && cargo check --workspace`（clean）
  - unit-test: ✅ `go test ./... && cargo test --workspace`
    - retriever 9/9 passed（task-4.1 5 + task-4.2 4 — TEST-4.2.1~4 全绿）
    - phase4_smoke 1/1 passed（TEST-4.2.5 — 主 agent §4 Gate 3 phase-4 §6 端到端 smoke 触发入口已就位）
    - 全 Rust 47 passed：lib 24 (parser 6 + chunker 5 + indexer 4 + retriever 9) + core_skeleton 4 + phase2_smoke 1 + phase4_smoke 1 + proto_contract 5 + scanner 12
    - 全 Go 8 包 ok（cli / config / contract / daemon / importer 3.1 + 3 个 importer 子包 3.2/3.3/3.4 + memoryops/dedup）
    - 零回归（task-4.1 / task-2.4 / phase2_smoke / 全 importer / 全 memoryops 子包全绿；reason: Option-of-String → String 不破任何现有测试）
- **剩余风险 / 未做项**：
  - **5 字段 schema gap 持续**（context_id / source_type / agent_scope / redaction_status — 沿 task-4.1 §10 留档）：v0.1 用 §2A default 常量（"" / "" / vec![] / "applied"）；populated coverage（5 字段真实非空）留 SPEC-DRIFT-task-2.4 chore-spec PR 扩 indexer schema 后由 task-8.1 eval-harness 回归。彻底支持需 SPEC-DRIFT-task-2.4：(a) SQLite chunks 表加 4 列 + (b) Tantivy STRING/STORED 4 字段 + (c) Chunk struct (chunker) 加字段 + (d) 反向 backfill scan 历史数据；retriever 不需改即可自动 fill 真实值。
  - **AC3 schema coverage 量化为 100%（struct 强制 12 字段）+ 反指标 provenance.len() ≥ 1**：v0.1 用 §2A 决策；populated coverage 留 §2.4 schema 扩后回归。当前 4 字段返默认值仍算"含可解释字段"（PRESENT），未跑实测覆盖率脚本（PRD §Success Metrics 90% 阈值 v0.1 自然达 100% 因 struct 强制）。
  - **AC4 gRPC ContextService::Search tonic server / contextforge search Go CLI 未实现**（§2A 决策 — Out of Scope）：v0.1 调试入口 = Rust `Retriever::explain` public API；gRPC server 留 task-6.2 REST API + tonic wrapper；Go CLI 留 task-6.1 cli-search；MCP `context_search` tool 留 task-7.1。proto `contextforge.v1.RetrievalResult` 已 frozen in task-1.1（12 字段 1:1 对应 SearchResult），Phase 6/7 wrap 仅需简单 `SearchResult → RetrievalResult` field mapping。
  - **AC4 reason / matched_terms enrichment 是简单 substring 匹配**（非 BM25-level token 解析）：enrich_matched_terms() split query on whitespace/quotes + trim non-alphanumeric + case-insensitive substring；适合 v0.1 调试可解释性场景。CJK / 复杂查询 / 高级 BM25 score component 拆解留 task-8.1 eval-harness + future enhancement task。
  - **provenance 合成只考虑 scanner 路径**：当 indexer provenance 表无 chunk_id 行（scanner-indexed 而非 importer-imported）时合成 `[{importer:"scanner",...}]`；importer-imported 的 chunk 走真实 JOIN 取多条 importer 行。但 IndexSession::index_path 默认 provenance: vec![] — 即使是 importer 路径，scanner 默认也合成。下游 importer 直接调用 IndexSession::write_chunks 传 provenance 时正常拼真实行。
- **下游 task 影响**：
  - **Phase 4 收口 → Phase 6 task-6.1/6.2/6.3 解除阻塞**：本 task 完工 → Phase 4 全 Done（task-4.1 + 4.2 都 Done）→ 后置 chore PR 补 phase-4 spec §6 端到端 smoke 命令（pattern 同 Phase 2 chore PR #25 / Phase 3 chore PR #22）→ Phase 4 spec Status: Draft → Done → Phase 6 cli-api-export 满足 dep（dep Phase 4 + 5；Phase 5 task-5.1 已 done，5.2/5.3 仍 Draft 但 Phase 6 仅依赖 Phase 4 + 5 整体 Done）。
  - **task-6.1 CLI `contextforge search`**：直接调用 `Retriever::open` + `Retriever::explain` → 终端展示 12 字段；CLI 不再需要后端实现，仅做 stdin/stdout 序列化。
  - **task-6.2 REST API `POST /v1/search`**：HTTP handler 把 body 映射到 `SearchOptions` → 调 `Retriever::explain` → SearchResult → proto `RetrievalResult`（1:1 field mapping，proto 已 frozen）。
  - **task-7.1 MCP `context_search` tool**：同 REST 形态，MCP tool handler 复用 SearchResult → proto.RetrievalResult 映射。
  - **task-8.1 eval-harness**：
    - schema coverage 回归：assert 每条结果 12 字段 PRESENT（compile-enforced，eval-harness 仅 sanity）
    - 真实 populated coverage 回归：要等 SPEC-DRIFT-task-2.4 chore-spec PR 扩 indexer schema 后跑（assert 5 schema-gap 字段非空率 ≥ 90% — PRD §Success Metrics 次指标）
    - 反指标 provenance 覆盖率：assert 每条结果 provenance.len() ≥ 1（黑盒守护）
    - AC4 性能 P95 < 500ms：用 Retriever 做黑盒 + bench
  - **未来 SPEC-DRIFT-task-2.4 chore-spec PR**（软依赖，本 task 不阻塞）：扩 indexer schema 让 5 schema-gap 字段真实存储；retriever 不需改即可自动 fill 真实值（仅 v0.1 default 常量逻辑会被 SQLite JOIN 真值覆盖）。
- **§2A Decisions**（2026-05-23 用户审定）：
  - **AC1 schema gap（选项 A — partial implement + provenance 合成）**：SearchResult 扩 12 字段 per AC1；context_id/source_type/agent_scope/redaction_status 用 §2A v0.1 default 常量（"" / "" / vec![] / "applied"）；provenance 优先 JOIN indexer provenance 表，缺失合成 `[{importer:"scanner", original_path:file_path, imported_at:indexed_at, source_modified_at:""}]` 保证 AC3 黑盒守护 ≥1 entry。schema gap 持续到 SPEC-DRIFT-task-2.4 chore-spec PR 扩 indexer schema。
  - **AC4 调试入口（选项 A — 仅 Rust public API）**：v0.1 实现 `Retriever::explain(opts)` 公开方法（force explain=true delegate search）；gRPC `ContextService::Search` tonic server 留 task-6.2；Go `contextforge search` CLI 留 task-6.1；MCP tool 留 task-7.1。proto 已 frozen in task-1.1。
  - **AC3 覆盖率（选项 A — schema 100% + 反指标 provenance≥1）**：v0.1 量化为 schema coverage 100%（struct 强制 12 字段，远超 90%）+ 反指标 `provenance.len() ≥ 1`（黑盒守护，合成兜底）。真实 populated coverage（5 schema-gap 字段非空率 ≥ 90%）留 SPEC-DRIFT-task-2.4 完成后 task-8.1 eval-harness 回归。
  - **R7 严格通道**：未引入新 crate；沿用 task-4.1 引入的 tantivy 0.26.1 / rusqlite 0.39.0 bundled / thiserror 2.0.18。`use crate::chunker::Provenance` 单 crate 内复用（DRY — 与 indexer 同一类型）。
  - **reason 类型 Option-of-String → String 改动**：proto3 string 默认空串语义；task-4.1 5 测试无 reason 断言 → 向下兼容零回归。explain=false → reason=""，explain=true → enriched。
