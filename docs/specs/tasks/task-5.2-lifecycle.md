# Task `5.2`: `lifecycle — stale 标记 + 基础冲突检测`

> ✅ 已过 `/s2v-implement` §2A 前置审核（2026-05-23）：§3/§4/§5.2/§5.3 `<TBD-by-user>` 已清零、§6 AC 经用户审定接受、AC1/AC2/AC4 三决策已确认（详见 §10 §2A Decisions）。实时状态以下方 `**Status**` 字段为准；状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: In Progress

**Priority**: P0
**Owner**: tajiaoyezi
**Related Phase**: Phase 5 (memoryops)
**Dependencies**: 5.1 (dedup)

## 1. Background

MemoryOps 生命周期治理：过期标记 + 基础冲突检测，避免过期/冲突上下文污染 Agent（PRD §Core Capabilities #3 / §Problem Statement 痛点 4）。能力边界按 PRD「v0.1 MemoryOps 能力边界」：stale = expires_at / source deleted / source modified；冲突仅检测同 key/path/tag 明显冲突，不做 LLM 语义判断。

## 2. Goal

`memoryops` 支持 stale 标记（`expires_at` 到期 / source deleted / source modified）可被设置与检索；基础冲突检测（同一 key / path / tag 下明显冲突给出提示），不做语义冲突判断。

## 3. Scope

### In Scope

- 新增 Go 子包 `internal/memoryops/lifecycle/`，与 task-5.1 `internal/memoryops/dedup/` 同 Go 控制面边界（task-5.1 §10 已确立 — Go 控制面消费 canonical `ContextRecord`；Rust `core/src/memoryops/` 仍为 placeholder）。
- **AC1 stale 三触发**（§2A 决策选项 A — 纯 Go transform，不持久化）：
  - `expires_at` 到期：`record.expires_at != nil && record.expires_at <= oracle.Now()`
  - source deleted：`record.provenance[i].original_path` 不存在（`!oracle.Exists(path)`）任一 importer 行命中即触发
  - source modified：当前 fs mtime > `record.provenance[i].source_modified_at` 任一 importer 行命中即触发
  - 三者 OR；返 `StaleMark{RecordID, Reason, MarkedAt}` 列表 — 不写 SQLite 不改 proto
- **AC2 基础冲突检测**（§2A 决策选项 A — `source_uri OR file_path` 任一重叠 + `content_hash` 不同）：
  - 分组 key：以 `source_uri`（主）和 `file_path`（辅）分别 group；任一 group 内 ≥2 条且 `content_hash` 不全相同 → 报 `ConflictReport{Key, KeyType, RecordIDs}`
  - tags 字段 v0.1 不参与冲突判定（§2A 决策：噪音过大）
  - 不做语义判断（AC3 反指标硬约束）
- **AC3 边界负向测试**：显式 `TestMark_DoesNotPerformSemanticAnalysis` — 2 record 含语义相同但 `content_hash` 不同 + `source_uri/file_path` 不重叠 → Mark 不报 conflict（证明无 LLM 调用，仅按 oracle + 字面字段判断）。
- **AC4 stale 排除 pre-filter**（§2A 决策选项 A — memoryops 提 API，retriever 不改）：
  - 公开 `FilterStale(records, marks)` 函数 → Phase 6 CLI/REST/MCP caller 显式 wrap（`r := lifecycle.Mark(results, oracle); clean := lifecycle.FilterStale(r.Records, r.StaleMarks)`）
  - Rust retriever crate 不改（保持 task-4.1/4.2 已 merge 的边界）
- 新增 4 RED→GREEN 测试 `TEST-5.2.1 ~ TEST-5.2.4` in `internal/memoryops/lifecycle/lifecycle_test.go`，全部 unit test。
- 填实 `test/features/memoryops.feature` 中 SCEN-5.2.1 ~ SCEN-5.2.4 占位 Given/When/Then。

### Out Of Scope

- **SQLite stale 表 / proto 加字段**：§2A 决策选项 A 选用纯 transform；持久化层（in-memory cache / SQLite 表）留 Phase 6 daemon 决定（同 task-5.1 dedup 不持久化先例）。
- **修改 task-4.1/4.2 retriever `SearchOptions` / `SearchResult`**：AC4 决策 B 不走 retriever 加 flag 路径；Rust crate 不动（保持 phase 4 merge 边界）。
- **LLM / embedding 语义冲突判断 / 语义相似 stale 推断**（AC3 反指标硬约束，PRD §Core Capabilities v0.1 MemoryOps 能力边界明示）。
- **tags overlap 冲突判定**（§2A 决策：v0.1 噪音过大；可未来 SPEC-DRIFT-task-5.2.tag-overlap 单独 chore PR 加严，需 negative test 套件保护）。
- **完整 event sourcing / audit 写入**（留 task-5.3 audit）。
- **修改 chunker / indexer / importer / proto 契约**（R7 + phase23-start-gate 冻结）。
- **跨 collection 的 stale / conflict 全局视图**（v0.1 单 collection 内处理；联邦留 Phase 6 daemon 编排）。
- **stale 自动清理 / GC**（仅标记 + filter；删 / archive 留 Phase 6 治理 task / 用户手动 export 决定）。

## 4. Users / Actors

- **task-5.1 dedup**（上游，✅ done）：本 task 消费 dedup 后的 `Records` 输入（或直接 importer 原始 records，pipeline 顺序灵活）。
- **task-5.3 audit**（并行，codex 同会话跑）：可消费本 task 的 `StaleMarks` 和 `Conflicts` 写入 audit log（task-5.3 决定如何记）。
- **Phase 6 task-6.1 CLI `contextforge search`**（下游强依赖）：CLI 调 retriever.Search → records → 调 `lifecycle.Mark(records, oracle)` + `lifecycle.FilterStale(records, marks)` → 终端展示 cleaned 结果。
- **Phase 6 task-6.2 REST API `POST /v1/search`**（下游强依赖）：HTTP handler 同 CLI pattern；返回 response 时可附 `stale_marks` / `conflict_reports` 字段（按 PRD §search response 契约 extension 字段）。
- **Phase 6 task-6.3 exporter**（下游软依赖）：export 时可选择跳过 stale records / 附 conflict 警告。
- **task-7.1 MCP `context_search` tool**（下游强依赖）：同 REST 形态。
- **task-8.1 eval-harness**（下游）：可消费 conflict reports 度量 PRD §Success Metrics 次指标「真实接入度 ≥ 20 条 memory/context 治理记录」。
- **PRD §Problem Statement 痛点 4 消费者**（业务消费）：「记忆冲突 / 过期记忆 / Agent 持续学习导致 hallucination」由本 task stale + conflict 守护。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Core Capabilities v0.1 MemoryOps 能力边界 / §Problem Statement 痛点 4）
- `docs/specs/phases/phase-5-memoryops.md`
- `docs/specs/tasks/task-5.1-dedup.md`
- `docs/decisions/adr-005-readonly-import-draft-export.md`
- `test/features/memoryops.feature`

### 5.2 Imports

- 内部：`github.com/tajiaoyezi/contextforge/proto/contextforge/v1`（canonical ContextRecord / Provenance — 同 task-5.1）
- stdlib：`os`（FS 探针 Stat / IsNotExist）、`time`（Now / mtime）、`sort`（输出确定性）
- **不引入新 crate / Go 模块**（R7 严格通道：不修改 `go.mod` / `Cargo.toml`）
- **不 import** task-5.1 `internal/memoryops/dedup`（lifecycle 与 dedup 双向解耦 — caller 决定 pipeline 顺序）

### 5.3 函数签名

```go
// Package lifecycle implements v0.1 MemoryOps stale 三触发 + 基础冲突检测.
// 同 task-5.1 dedup pattern：纯 transform（input → Result），不持久化；
// Phase 6 daemon 决定 in-memory cache / SQLite 持久化层归宿。
package lifecycle

import (
    "os"
    "time"

    contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// Result — Mark 一次过的全部输出。Records 为入参原样透传（不去重 / 不删 stale），
// 由 caller 决定后续步骤（FilterStale / 直接展示 / 写 audit log）。
type Result struct {
    Records    []*contextforgev1.ContextRecord
    StaleMarks []StaleMark
    Conflicts  []ConflictReport
}

// StaleMark — AC1 三触发任一命中即标记一条。
type StaleMark struct {
    RecordID string
    Reason   StaleReason
    MarkedAt time.Time
}

// StaleReason 枚举（v0.1 三种触发）。
type StaleReason string

const (
    StaleReasonExpired        StaleReason = "expired"         // expires_at 到期
    StaleReasonSourceDeleted  StaleReason = "source-deleted"  // provenance.original_path 不存在
    StaleReasonSourceModified StaleReason = "source-modified" // 当前 fs mtime > provenance.source_modified_at
)

// ConflictReport — AC2 同 source_uri 或同 file_path 但 content_hash 不全相同。
type ConflictReport struct {
    Key       string          // group key 实值（source_uri / file_path 字面量）
    KeyType   ConflictKeyType // "source_uri" | "file_path"
    RecordIDs []string        // 参与冲突的 record id（≥2，去重 + sort 后确定性输出）
}

// ConflictKeyType 枚举（v0.1 两种 group key）。
type ConflictKeyType string

const (
    ConflictKeySourceURI ConflictKeyType = "source_uri"
    ConflictKeyFilePath  ConflictKeyType = "file_path"
)

// Oracle 抽象环境依赖（Clock + FS），让测试可注入确定性 fake — AC1 三触发依赖.
type Oracle interface {
    Now() time.Time
    Exists(path string) bool
    ModTime(path string) (time.Time, bool)
}

// SystemOracle — 生产默认 Oracle（time.Now / os.Stat）.
type SystemOracle struct{}

func (SystemOracle) Now() time.Time
func (SystemOracle) Exists(path string) bool
func (SystemOracle) ModTime(path string) (time.Time, bool)

// Mark — 主入口（AC1 + AC2 + AC3）.
//
// AC1: 对每条 record 跑 expires_at / source-deleted / source-modified 三触发；
//      任一命中即追加 StaleMark（同一 record 多触发各加一条 — 让 audit/debug 看清原因）.
// AC2: 跨全集按 source_uri 与 file_path 两种 group key 分组；
//      group 内 ≥2 条且 content_hash 不全相同 → ConflictReport 追加.
// AC3: 不调用任何 LLM / embedding API（仅 oracle + 字面字段比较；
//      显式 TestMark_DoesNotPerformSemanticAnalysis 守护）.
func Mark(records []*contextforgev1.ContextRecord, oracle Oracle) Result

// FilterStale — AC4 pre-filter for retriever consumers.
//
// 不修改入参；返回 records 中不在 marks RecordID 集合内的子集.
// Phase 6 CLI / REST / MCP caller 调用模式：
//   results := retriever.Search(opts)
//   r := lifecycle.Mark(results, oracle)
//   clean := lifecycle.FilterStale(r.Records, r.StaleMarks)
//   render(clean)
func FilterStale(records []*contextforgev1.ContextRecord, marks []StaleMark) []*contextforgev1.ContextRecord
```

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Implementation Phases Phase 5 Exit Criteria): stale 标记可被设置和检索（`expires_at` 到期 / source deleted / source modified 三种触发）。
- [ ] **AC2** (PRD §Core Capabilities v0.1 MemoryOps 能力边界): 基础冲突检测仅覆盖同一 key / path / tag 下明显冲突并给提示。
- [ ] **AC3** (PRD §Core Capabilities v0.1 MemoryOps 能力边界): **不做** LLM 语义冲突判断（边界外）。
- [ ] **AC4** (PRD §Problem Statement 痛点 4 / 本 task 新增): 检索可选择排除/标注 stale 记录，避免过期上下文污染召回。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 stale 三触发可设/检索 | SCEN-5.2.1 | TEST-5.2.1 | - | unit-test | Not Started |
| AC2 基础冲突检测提示 | SCEN-5.2.2 | TEST-5.2.2 | - | unit-test | Not Started |
| AC3 不做语义冲突(边界) | SCEN-5.2.3 | TEST-5.2.3 | - | unit-test | Not Started |
| AC4 检索可排除 stale | SCEN-5.2.4 | TEST-5.2.4 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R5**：source modified/deleted 判定依赖 provenance.source_modified_at 准确性。关联 PRD §Open Questions **O5**。

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
