# Task `5.2`: `lifecycle — stale 标记 + 基础冲突检测`

> ✅ 已过 `/s2v-implement` §2A 前置审核（2026-05-23）：§3/§4/§5.2/§5.3 `<TBD-by-user>` 已清零、§6 AC 经用户审定接受、AC1/AC2/AC4 三决策已确认（详见 §10 §2A Decisions）。实时状态以下方 `**Status**` 字段为准；状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: Done

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

- [x] **AC1** (PRD §Implementation Phases Phase 5 Exit Criteria): stale 标记可被设置和检索（`expires_at` 到期 / source deleted / source modified 三种触发）。
- [x] **AC2** (PRD §Core Capabilities v0.1 MemoryOps 能力边界): 基础冲突检测仅覆盖同一 key / path / tag 下明显冲突并给提示。
- [x] **AC3** (PRD §Core Capabilities v0.1 MemoryOps 能力边界): **不做** LLM 语义冲突判断（边界外）。
- [x] **AC4** (PRD §Problem Statement 痛点 4 / 本 task 新增): 检索可选择排除/标注 stale 记录，避免过期上下文污染召回。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 stale 三触发可设/检索 | SCEN-5.2.1 | TEST-5.2.1 | - | unit-test | Done |
| AC2 基础冲突检测提示 | SCEN-5.2.2 | TEST-5.2.2 | - | unit-test | Done |
| AC3 不做语义冲突(边界) | SCEN-5.2.3 | TEST-5.2.3 | - | unit-test | Done |
| AC4 检索可排除 stale | SCEN-5.2.4 | TEST-5.2.4 | - | unit-test | Done |

## 8. Risks

- 关联 PRD §Technical Risks **R5**：source modified/deleted 判定依赖 provenance.source_modified_at 准确性。关联 PRD §Open Questions **O5**。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。

## 10. Completion Notes

- **完成日期**：2026-05-23
- **改动文件**：
  - internal/memoryops/lifecycle/lifecycle.go（新增 Go 子包：Result / StaleMark / StaleReason / ConflictReport / ConflictKeyType / Oracle / SystemOracle 类型 + Mark 主入口 + FilterStale pre-filter + 私有 detectConflicts / collectConflicts / recordRef helpers + 私有 const 3 个 StaleReason 与 2 个 ConflictKeyType）
  - internal/memoryops/lifecycle/lifecycle_test.go（新增：4 RED→GREEN tests TEST-5.2.1~5.2.4 + fakeOracle test double + newRecord/newProvenance/findStaleMark/findConflict/recordIDs 测试 helpers）
  - test/features/memoryops.feature（SCEN-5.2.1~5.2.4 Given/When/Then 填实）
  - docs/specs/tasks/task-5.2-lifecycle.md（§2A 业务承诺 §3/§4/§5.2/§5.3 填实；§6 AC1-4 全勾选；§7 4 行 → Done；§10 终态回填；Status: Draft → Ready → In Progress → Done）
- **commit 列表**（本 task 全部 4 个，按时间顺序）：
  - 2eaf12e docs(spec): task-5.2 §2A 业务承诺 (Draft → Ready)
  - 53266fc test(memoryops): 加 SCEN-5.2.1~4 共 4 个 RED 测试 + Status: Ready → In Progress
  - eaaabbb feat(memoryops): 实现 stale 三触发 + 基础冲突检测 + FilterStale 通过全部 4 个测试
  - 本回填 docs(spec) commit（§6/§7/§10 终态 + Status → Done）
- **§9 Verification 结果**：
  - install: ✅ `go mod download && cargo fetch`（无新 deps；R7 严格通道：未改 go.mod / Cargo.toml / 任何 lockfile）
  - typecheck: ✅ `go vet ./... && cargo check --workspace`（clean）
  - unit-test: ✅ `go test ./... && cargo test --workspace`
    - lifecycle 4/4 passed（TEST-5.2.1 stale 三触发 / TEST-5.2.2 conflict 检测 / TEST-5.2.3 反指标无语义分析 regression guard / TEST-5.2.4 FilterStale pre-filter）
    - 全 Go 10 包 ok（cli / config / contract / daemon / importer 4 子包 / memoryops/dedup / memoryops/lifecycle 新增）
    - 全 Rust 47 passed（lib 24 + core_skeleton 4 + phase2_smoke 1 + phase4_smoke 1 + proto_contract 5 + scanner 12）— 无 Rust 改动，零回归
- **剩余风险 / 未做项**：
  - **持久化未做（§2A 决策）**：StaleMarks / Conflicts 仅在内存中，不写 SQLite 不改 proto；同 task-5.1 dedup 先例。Phase 6 daemon 决定是否引入 in-memory cache 或独立 SQLite stale 表；当前调用方需在每次 query/render 前调一次 Mark + FilterStale，对 small dataset (v0.1 单 collection) 性能可接受。大规模时（10 万 chunk 级）需 Phase 6 cache layer。
  - **AC2 tags overlap 不参与冲突判定（§2A 决策）**：v0.1 仅 source_uri 与 file_path 两个 group key；tags overlap 噪音过大（同 tag 不同事实常见）。如未来确需 tag 维度冲突，开 SPEC-DRIFT-task-5.2.tag-overlap 独立 chore PR + 配套 negative test 套件防止误报。
  - **AC1 source-modified 依赖 provenance.source_modified_at 准确性**（PRD §Technical Risks R5 / §Open Questions O5）：若上游 importer 没填 source_modified_at，trigger 3 跳过（不报 stale，也不报 false positive）；若 importer 填错（如总填 unix epoch 0），可能导致所有文件被误报 source-modified。已在测试 helper newProvenance 中演示正确填法；importer 端正确性由 Phase 3 importer task 套件保证（task-3.1/3.2/3.3/3.4 已 done）。
  - **未与 task-5.3 audit 直接 wire**：本 task 输出的 StaleMarks / Conflicts 可被 task-5.3 audit 写 audit.log 消费，但本 task 不直接调 audit 写入（layer separation）；task-5.3 spec 自决如何消费（codex 并行实施中）。
  - **未实现 stale 记录自动清理 / GC / archive**：仅标记 + filter；删 / 迁移留 Phase 6 治理 task / 用户手动 export 决定。
  - **SystemOracle 在测试中未覆盖（仅生产路径）**：所有 4 tests 用 fakeOracle 注入确定性 clock/FS；SystemOracle 方法 (Now / Exists / ModTime) 是 stdlib 直接调用包装（time.Now / os.Stat / info.ModTime），无 testable 逻辑分支。
- **下游 task 影响**：
  - **task-5.3 audit**（并行，codex 同会话跑）：可消费 lifecycle.Result.StaleMarks 与 Conflicts 在 audit.log 中记录 stale / conflict 事件类别（task-5.3 自决事件 schema）。无 import 反向依赖（lifecycle 不 import audit；audit 自由消费 lifecycle 输出）。
  - **Phase 5 合并顺序（主 agent 域）**：task-5.2 先 merge → 主 agent chore PR pre-closeout 填实 phase-5 spec §6 端到端 smoke → task-5.3 是 last task 触发 §4 Gate 3 → merge 后 Phase 5 全 Done（同 Phase 4 pattern）。本 task 不实现 phase-5 smoke（task-5.3 AC5 负责）。
  - **Phase 6 task-6.1 CLI `contextforge search`**：终端调用模式 results := retriever.Search(opts); r := lifecycle.Mark(results, SystemOracle{}); clean := lifecycle.FilterStale(r.Records, r.StaleMarks); 把 r.StaleMarks 与 r.Conflicts 可选打印到 stderr 供 debug。
  - **Phase 6 task-6.2 REST API**：HTTP handler 同 CLI pattern；response JSON 可附加 stale_marks 与 conflict_reports 字段（按 PRD §search response 契约 extension 字段）。
  - **Phase 6 task-6.3 exporter**：可调 lifecycle.Mark 后选择跳过 stale records / 在 export bundle 中附 ConflictReport 警告。
  - **task-7.1 MCP `context_search` tool**：同 REST 形态，MCP tool handler 复用 lifecycle pipeline。
  - **task-8.1 eval-harness**：可消费 ConflictReport 统计度量 PRD §Success Metrics 次指标「真实接入度 ≥ 20 条 memory/context 治理记录」。
- **§2A Decisions**（2026-05-23 用户审定）：
  - **AC1 stale 存储（选项 A — 纯 Go transform，不持久化）**：复用 task-5.1 dedup pattern；Mark 输入 records + Oracle，输出 Result 含 StaleMarks 列表；不写 SQLite 不改 proto。持久化层留 Phase 6 daemon 决定（in-memory cache 或独立 stale 表）。Oracle 接口可注入 fake 实现确定性测试，生产用 SystemOracle (time.Now / os.Stat)。
  - **AC2 冲突 key 语义（选项 A — source_uri OR file_path 任一重叠 + content_hash 不同）**：分组 key 以 source_uri 为主 + file_path 为辅；两套 group 各独立检测；group 内 >=2 且 content_hash 不全相同 → 一条 ConflictReport。tags overlap v0.1 不参与（噪音过大，§2A 决策）。
  - **AC4 stale 排除（选项 A — memoryops 提 FilterStale pre-filter）**：lifecycle 公开 FilterStale(records, marks) 函数；Phase 6 CLI / REST / MCP caller 显式 wrap；Rust retriever 不改（保 phase 4 merge 边界 + 避跨语言耦合）。
  - **R7 严格通道**：未引入新 Go module / Rust crate；仅用 stdlib + 既有 proto/contextforge/v1 / protobuf timestamppb。`internal/memoryops/lifecycle/` 与 `internal/memoryops/dedup/` 兄弟子包（同 task-5.1 边界），互不 import — caller 决定 pipeline 顺序。
