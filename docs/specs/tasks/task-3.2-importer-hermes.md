# Task `3.2`: `importer-hermes — Hermes MEMORY.md / USER.md 导入`

> ✅ 已过 `/s2v-implement` §2A 前置审核（2026-05-21）：§3/§4/§5.2/§5.3 `<TBD-by-user>` 已清零、§6 AC 经用户审定接受。实时状态以下方 `**Status**` 字段为准；状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: Done

**Priority**: P0
**Owner**: tajiaoyezi
**Related Phase**: Phase 3 (agent-importers)
**Dependencies**: 3.1 (importer-core)

## 1. Background

Hermes 是 PRD 列出的 P0 导入源之一。本 task 实现 Hermes `MEMORY.md` / `USER.md` → canonical ContextRecord 的只读导入（PRD §Constraints 兼容性 / §Decisions Log D5）。

## 2. Goal

`contextforge import hermes <path>` 把 Hermes `MEMORY.md` / `USER.md` 转为 ContextRecord（source_provider=hermes，agent_scope 含 hermes），保留 provenance（original_path / source_modified_at）；不写回 Hermes memory。

## 3. Scope

### In Scope

- 实现 AC1–AC4：Hermes `MEMORY.md` / `USER.md` → canonical ContextRecord + provider/scope/provenance 三联 + 只读 + AC4 fallback 复用 task-3.1 框架
- **Detect 策略**：文件名 `MEMORY.md` / `USER.md` 大小写不敏感匹配 → confidence 0.9, ok=true；不依赖路径上下文 / 不依赖内容标记（v0.1 §2A 决策；PRD §O3 实测后可加版本 marker）
- **canonical record 关键字段**（task-3.1 已冻结 BINDING + AC2 必填）：
  - `source_provider="hermes"` / `agent_scope=["hermes"]` / `provenance.importer="hermes-memory"`
  - `source_type="memory"`（PRD §Canonical Record example）
  - `redaction_status="pending"`（task-3.1 §10 Waiver BINDING — 下游 scanner/indexer 脱敏）
  - `language="markdown"`（Hermes MEMORY.md/USER.md 均为 markdown）
  - `provenance.original_path` / `provenance.source_modified_at` 保留 file mtime
- **AC4 fallback 触发**（v0.1 §2A 决策）：仅 `strings.TrimSpace(content) == ""` → 调 `importer.NewFileFallbackImporter().Import(...)` + 显式 `[warning]` log
- **内容保留原 markdown 不结构化**（headings / sections / code blocks → Phase 2 chunker 接力）
- 模块入口：`internal/importer/hermes/hermes.go`（新子包；与 task-3.3/3.4 物理隔离）
- 显式 Register 入口：暴露 `hermes.New()` 返 `importer.Importer`，由 CLI/daemon 启动期注册（不在 importer 包加全局 init() 副作用 — task-3.1 重构精神，§3.1 refactor commit）

### Out Of Scope

- 写回 Hermes `MEMORY.md` / `USER.md`（AC3 + ADR-005 + PRD §Decisions Log D5 三重禁止）
- markdown 结构解析（headings / sections / code blocks / frontmatter — 留 Phase 2 chunker）
- 真实 Hermes fixture 样本回归（PRD §O3 待 v0.2 实测后补；v0.1 用合成 markdown fixture 覆盖 AC1-4）
- secret redaction（task-3.1 §10 Waiver BINDING + SPEC-DRIFT-task-3.1 选项 A — 由下游 scanner/indexer task-2.1/2.4 负责）
- 跨 importer 共享 record 构造 helper 抽象（task-3.1 `buildRecord` 未导出 — 本子包内重新实现 ~30 行；后续如 3 个 importer 都重复可由主 agent 走独立 refactor PR 抽取，本 task 不 fold-in）
- Hermes-specific frontmatter / metadata 抽取（v0.1 不识别；放 ContextRecord.Content 由 chunker 处理）
- 多文件目录递归（v0.1 `import hermes <single-file-path>`；批量导入由 CLI/daemon 编排，task-6.x）

## 4. Users / Actors

- **`contextforge import hermes <path>` CLI 命令**（调用方，task-6.x CLI 编排实现）：通过 `importer.Resolve` 或显式 `hermes.New()` 触发
- **task-3.1 `importer.Resolve`**（注册表派发方）：本 task 注册 hermes importer 后被 Resolve 按 confidence 选中
- **task-3.1 `importer.NewFileFallbackImporter`**（AC4 fallback 委托方）：v0.1 仅在 content 为空时复用其降级路径
- **chunker (task-2.3, downstream)**：消费 `ContextRecord.Content` 切片
- **indexer (task-2.4, downstream)**：写 SQLite metadata + Tantivy 全文索引
- **scanner (task-2.1) / indexer (task-2.4)**：执行 secret redaction（redaction_status="pending" → 实际脱敏）
- **memoryops (Phase 5)**：基于 `content_hash` 跨来源去重（task-2.3 已统一 sha256；本 importer 用同算法直接 hex 无 algo-prefix —— 与 task-3.1 一致）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Constraints 兼容性 / §Core Capabilities #5）
- `docs/specs/phases/phase-3-agent-importers.md`
- `docs/specs/tasks/task-3.1-importer-core.md`
- `docs/decisions/adr-005-readonly-import-draft-export.md`
- `test/features/importer.feature`

### 5.2 Imports

- **标库**：`crypto/sha256` / `encoding/hex` / `fmt` / `log` / `os` / `path/filepath` / `strings` / `time` / `testing`
- **proto/canonical record**：`github.com/tajiaoyezi/contextforge/proto/contextforge/v1`（`ContextRecord`、`Provenance` — task-1.1 frozen contract）
- **importer 框架**：`github.com/tajiaoyezi/contextforge/internal/importer`（公共 API：`Importer` 接口、`NewFileFallbackImporter()` — task-3.1 已就绪）
- **第三方**：`google.golang.org/protobuf/types/known/timestamppb`（已通过 task-1.1 引入 go.mod，本 task 不增 dep；与 task-3.1 一致）
- **R7 严格处理**：本 task **不引入新 crate / go module**（task agent 不修改 `go.mod` / `go.sum` / `Cargo.toml` / `Cargo.lock`）；所有依赖均为现有可消费符号

### 5.3 函数签名

```go
// internal/importer/hermes/hermes.go

package hermes

import (
    "github.com/tajiaoyezi/contextforge/internal/importer"
    contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// New 创建 Hermes-aware importer（MEMORY.md / USER.md）。
// 调用方：CLI/daemon 启动期 `importer.Register(hermes.New())`；或测试直接调用。
func New() importer.Importer

// hermesImporter 实现 importer.Importer 接口。私有，仅暴露 New()。
type hermesImporter struct{}

// Name 返回 "hermes-memory"（与 PRD §Canonical Record provenance.importer 例值一致）。
func (h *hermesImporter) Name() string

// Detect：文件名 MEMORY.md / USER.md（大小写不敏感）→ confidence 0.9, ok=true。
// 否则 (0, false)。不读文件 / 不查路径上下文（v0.1 §2A 决策）。
func (h *hermesImporter) Detect(path string) (confidence float64, ok bool)

// Import：读文件 → 内容判空 → recognized 路径调 buildHermesRecord / unrecognized
// 路径调 task-3.1 NewFileFallbackImporter + 显式 [warning] log（AC4）。
// 失败：os.ReadFile 真错（不存在 / 权限）→ 返回 error；其他场景 return nil 不发生。
func (h *hermesImporter) Import(path, collectionID string) ([]*contextforgev1.ContextRecord, error)
```

子包内私有 helper（仅本子包可见；§3 Out-of-Scope 「不抽取跨 importer 共享 helper」对应）：
- `buildHermesRecord(path, content, collectionID string) []*contextforgev1.ContextRecord` — 构造 ContextRecord，硬编码 `source_provider="hermes"` / `agent_scope=["hermes"]` / `provenance.importer="hermes-memory"` / `source_type="memory"` / `language="markdown"` / `redaction_status="pending"`
- `makeID(path, content string) string` — sha256(path:content) 前 16 hex，前缀 `ctx_hermes_`
- `sourceURI(abs string) string` — `file://` 前缀
- `contentHash(content string) string` — sha256 64-hex 裸（与 task-3.1 importer 一致，无 algo-prefix）
- `fallbackImport(path, collectionID string) ([]*contextforgev1.ContextRecord, error)` — 内部委托给 `importer.NewFileFallbackImporter().Import(...)`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [x] **AC1** (PRD §Implementation Phases Phase 3 Exit Criteria): Hermes `MEMORY.md` / `USER.md` 能导入为 canonical ContextRecord。
- [x] **AC2** (PRD §Technical Approach Canonical Record v0.1): source_provider=`hermes`、agent_scope 含 `hermes`、provenance.importer=`hermes-memory`、保留 original_path / source_modified_at。
- [x] **AC3** (PRD §Decisions Log D5): 只读导入，不修改/写回 Hermes `MEMORY.md` / `USER.md`。
- [x] **AC4** (PRD §Technical Risks R5): Hermes schema 不识别/版本差异时降级通用 markdown 导入 + warning（复用 3.1 fallback），不中断。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 Hermes 导入为 record | SCEN-3.2.1 | TEST-3.2.1 | - | unit-test | Done |
| AC2 provider/scope/provenance | SCEN-3.2.2 | TEST-3.2.2 | - | unit-test | Done |
| AC3 只读不写回 | SCEN-3.2.3 | TEST-3.2.3 | - | unit-test | Done |
| AC4 schema 差异降级 | SCEN-3.2.4 | TEST-3.2.4 | - | unit-test | Done |

## 8. Risks

- 关联 PRD §Technical Risks **R5**：Hermes schema 漂移 → fixture 回归 + fallback。关联 PRD §Open Questions **O3**（需实测 Hermes 版本与样本）。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。

## 10. Completion Notes

- **完成日期**：2026-05-21
- **改动文件**：
  - internal/importer/hermes/hermes.go（新增子包：hermesImporter + Detect/Import + buildHermesRecord/makeID/sourceURI/contentHash/fallbackImport 5 helpers；与 task-3.3/3.4 物理隔离）
  - internal/importer/hermes/hermes_test.go（新增：TEST-3.2.1~4 external test package hermes_test 仅用公开 API；含强断言阻挡 do-nothing stub 假绿）
  - docs/specs/tasks/task-3.2-importer-hermes.md（Status: Draft→Ready→In Progress→Done；§3/§4/§5.2/§5.3 §2A 填实；§6 AC1-4 全部勾选；§7 4 行 → Done；§10 终态回填）
  - test/features/importer.feature（SCEN-3.2.1~4 Given/When/Then 填实）
- **commit 列表**（本 task 全部 4 个，按时间顺序）：
  - 655716b docs(spec): task-3.2 业务承诺 (Draft → Ready)
  - cab703f test(importer): 加 SCEN-3.2.1~4 共 4 个 RED 测试 + Status: Ready → In Progress
  - f880a97 feat(importer): 实现 hermes importer 通过全部 4 个测试
  - 本回填 docs(spec) commit（§6/§7/§10 终态 + Status → Done）
- **§9 Verification 结果**：
  - install: ✅ `go mod download && cargo fetch`
  - typecheck: ✅ `go vet ./... && cargo check --workspace`（clean，新增 hermes 子包编译通过；无新依赖引入）
  - unit-test: ✅ `go test ./... && cargo test --workspace`
    - hermes 4 passed / 0 failed / 0 ignored（AC1-4 全绿；包含 BINDING redaction=pending 断言）
    - 全 Go 6 包绿（cli / config / contract / daemon / importer 3.1 / importer/hermes 3.2）
    - 全 Rust 32 passed（parser 6 + chunker 5 + core_skeleton 4 + proto_contract 5 + scanner 12）— 无回归
- **剩余风险 / 未做项**：
  - **Detect 仅文件名**（v0.1 §2A 决策）：任意 MEMORY.md / USER.md 都会被识别为 Hermes，包括非 Hermes 来源的同名文件。用户显式 `import hermes [path]` 时意图明确；自动 Resolve 派发时如同目录有多个 importer 候选，按 confidence 仲裁（hermes 0.9 > fallback 0.1）。PRD §O3 实测 Hermes 样本后可加内容标记 / 路径上下文检查。
  - **AC4 fallback 触发仅 content 空**（v0.1 §2A 决策）：未来 schema 漂移（Hermes v2 不同 layout）当前会进入正常 buildHermesRecord 路径，由 chunker / retriever 容忍非典型 markdown 结构；PRD §O3 实测后可加 `<!-- hermes-version: vN -->` marker 检测。
  - **真实 Hermes 样本 fixture 待 v0.2 补**：v0.1 用合成 markdown 覆盖 AC1-4（PRD §O3 / phase-3 §6 端到端 smoke 待 task-3.4 完工前由主 agent 决定是否合 golden-hermes-memory/ fixture）。
  - **跨 importer 共享 record 构造 helper 未抽取**（§3 Out-of-Scope）：本 task 在子包内重新实现了 ~30 行 makeID/sourceURI/contentHash/lineCount 等。如 3 个 importer (3.2/3.3/3.4) 完工后重复明显，可由主 agent 走独立 refactor PR 抽取 `internal/importer.BuildRecord` 公共 API。
- **下游 task 影响**：
  - **task-3.3 importer-openclaw / task-3.4 importer-agent-rules**（3-way 并行同 phase）：可参考本 task 的子包模式 + AC4 fallback 委托 task-3.1 NewFileFallbackImporter 的实现模板。
  - **task-6.x CLI**（`contextforge import hermes [path]`）：调用方需在启动期 `importer.Register(hermes.New())`，或直接调用 `hermes.New().Import(path, collectionID)`。
  - **chunker (task-2.3, ✅ done)**：消费 `ContextRecord.Content` 切片；本 task 输出 markdown 类型 + redaction_status=pending 状态符合 chunker 输入契约。
  - **indexer (task-2.4)**：写 SQLite metadata + Tantivy；本 task 输出 ContextRecord 字段集与 task-1.1 frozen contract 完全对齐。
  - **Phase 5 memoryops**：content_hash 与 task-2.3 chunker (algo-prefix `sha256:<64-hex>`) / task-3.1 importer (裸 `<64-hex>`) 同 sha256；本 task 沿用 task-3.1 裸 hex 格式 — Phase 5 桥接时按格式分流即可。
- **§2A Decisions**（2026-05-21 用户答题）：
  - **Detect 策略 = 仅文件名（选项 A）**：v0.1 最小依赖；PRD §O3 实测 Hermes 样本前不引入路径/内容启发式检查
  - **AC4 fallback 触发 = 仅内容空 / 读不到（选项 A）**：v0.1 保守；未来 schema 版本 marker 留 §O3 实测后引入
  - **不引入新依赖**（R7 严格 — task agent 不改 go.mod / go.sum / Cargo.toml / Cargo.lock）
  - **新子包 internal/importer/hermes/ 物理隔离**：3-way 并行写路径不冲突；与 task-3.3/3.4 互不影响
