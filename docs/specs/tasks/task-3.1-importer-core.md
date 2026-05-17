# Task `3.1`: `importer-core — canonical record 映射 + importer 框架 + fallback`

> ✅ **已过 §2A 前置审核** — 用户确认选项 A 推进（Owner/Scope/Actors/Imports/函数签名由用户派工确认），Status 经用户确认后推进至 Done。详见 commit `6377d3d` 与 `fe4ff94` 留痕。

**Status**: Done

**Priority**: P0
**Owner**: tajiaoyezi
**Related Phase**: Phase 3 (agent-importers)
**Dependencies**: Phase 1（canonical schema + proto）

## 1. Background

Phase 3 框架 task：定义 importer 抽象 + canonical record 映射 + 通用 file/markdown fallback，使 hermes/openclaw/agent-rules importer 共享一致映射并对未识别 schema 安全降级（PRD §Technical Risks R5 / §Decisions Log D5）。

## 2. Goal

`agent-importer` 框架就位：定义 `Importer` 抽象（探测/解析/映射为 ContextRecord），通用 file/markdown/config/log fallback 永远可用；不识别 schema → 降级 fallback + warning，不中断；canonical record 与 importer 解耦。

## 3. Scope

### In Scope

- `Importer` 接口抽象（`Detect` / `Import`）及注册表
- `ContextRecord` 映射器（将原始来源映射为 `contextforge.v1.ContextRecord`）
- 通用 `FileFallbackImporter`（file / markdown / config / log 的保底导入）
- Importer 版本探测钩子框架
- 未识别 schema 的安全降级机制（fallback + warning）

### Out Of Scope

- Hermes `MEMORY.md` / `USER.md` 具体适配器（task-3.2）
- OpenClaw workspace 具体适配器（task-3.3）
- Agent-rules（`AGENTS.md` / `CLAUDE.md`）具体适配器（task-3.4）
- 写回第三方 Agent memory（PRD Out of Scope）
- Embedding 生成与向量索引（Phase 2 / Phase 4）

## 4. Users / Actors

- `contextforge import` CLI 命令（调用方）
- Go daemon 导入调度器（调用方）
- 下游具体 importer：hermes-importer、openclaw-importer、agent-rules-importer（实现方）
- `FileFallbackImporter`（保底实现方）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Decisions Log D5 / §Technical Approach Canonical Record schema / §Technical Risks R5）
- `docs/specs/phases/phase-3-agent-importers.md`
- `docs/specs/tasks/task-1.1-proto.md`
- `docs/decisions/adr-005-readonly-import-draft-export.md`
- `test/features/importer.feature`

### 5.2 Imports

- `proto/contextforge/v1`（`ContextRecord`、`Provenance` 等生成类型）
- stdlib: `fmt`, `io`, `os`, `path/filepath`, `strings`, `errors`, `sync`, `hash/crc32`, `time`

### 5.3 函数签名

```go
// Importer 是 Agent 导入器的抽象接口。
type Importer interface {
    Name() string
    Detect(path string) (confidence float64, ok bool)
    Import(path string, collectionID string) ([]*contextforgev1.ContextRecord, error)
}

// Register 注册一个 importer 实现到全局注册表。
func Register(importer Importer)

// Resolve 按路径探测并返回最佳匹配的 importer；若无匹配则返回 FileFallbackImporter。
func Resolve(path string) (Importer, error)

// NewFileFallbackImporter 创建通用文件保底导入器。
func NewFileFallbackImporter() Importer
```

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [x] **AC1** (PRD §Decisions Log D5): 定义 `Importer` 抽象（探测 → 解析 → 映射为 `ContextRecord`），只读导入，不写回任何第三方 Agent memory。
- [x] **AC2** (PRD §Technical Risks R5): 通用 file/markdown/config/log fallback 永远可用，作为分层 importer 的保底层。
- [x] **AC3** (PRD §Implementation Phases Phase 3 Exit Criteria): 不识别 schema → 降级为通用文件导入 + 显式 warning，不中断整个导入。
- [x] **AC4** (PRD §Technical Approach Canonical Record v0.1): 映射产出的 ContextRecord 含 source_type/source_provider/source_uri/agent_scope/provenance 等核心字段，未识别字段进 metadata.extra。
- [x] **AC5** (PRD §Technical Risks R5 / 本 task 新增): 每个 importer 可声明版本探测钩子，canonical record 与 importer 解耦（更换 importer 不动 record schema）。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 Importer 抽象只读 | SCEN-3.1.1 | TEST-3.1.1 | - | unit-test | Done |
| AC2 通用 fallback 保底 | SCEN-3.1.2 | TEST-3.1.2 | - | unit-test | Done |
| AC3 未识别降级+warning | SCEN-3.1.3 | TEST-3.1.3 | - | unit-test | Done |
| AC4 映射核心字段完整 | SCEN-3.1.4 | TEST-3.1.4 | - | unit-test | Done |
| AC5 importer/record 解耦 | SCEN-3.1.5 | TEST-3.1.5 | - | unit-test | Done |

## 8. Risks

- 关联 PRD §Technical Risks **R5**（外部 Agent schema 漂移，概率高）：分层 importer + fallback 是核心缓解；本 task 奠定框架。关联 PRD §Open Questions **O3 / O5**。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。

## 10. Completion Notes

- **完成日期**：2026-05-17
- **改动文件**：
  - `internal/importer/importer.go`（新增：Importer 接口、Register/Resolve 注册表）
  - `internal/importer/fallback.go`（新增：FileFallbackImporter 保底实现）
  - `internal/importer/record.go`（新增：buildRecord 映射辅助 + language/content-hash 探测）
  - `internal/importer/importer_test.go`（新增：6 个单元测试对应 5 个 AC）
  - `test/features/importer.feature`（更新：SCEN-3.1.1~3.1.5 Given/When/Then）
  - `docs/specs/tasks/task-3.1-importer-core.md`（Status 推进、§7 追踪表推进、§10 回填）
- **commit 列表**：
  - `a9e58ca` test(importer): 加 SCEN-3.1.1~3.1.5 共 5 个 RED 测试
  - `3b3ae46` feat(importer): 实现 importer 框架 + fallback + record 映射通过全部 5 个测试
  - `12d7b0d` refactor(importer): 去掉 registerOnce 副作用，Resolve 无匹配时直接返回 fallback 实例
  - `08c7240` fix(importer): Id/content_hash 改用 sha256 避免 100k chunk 生日碰撞
  - `fe4ff94` test(importer): AC5 改用真实 buildRecord 断言 schema 不变性 + §6 勾 [x]
  - `26eecee` test(importer): TEST-3.1.3 补 warning 断言（RED：Resolve 未输出显式 warning）
  - `4b03d1b` feat(importer): Resolve 对未识别 schema 输出显式 warning 后降级 fallback（AC3）
- **§9 Verification 结果**：
  - install: ✅
  - typecheck: ✅
  - unit-test: 16 passed / 0 failed（Go: internal/config 5 + internal/contract 5 + internal/importer 6）/ 9 passed / 0 failed（Rust: core skeleton 4 + proto contract 5）
- **剩余风险 / 未做项**：
  - registry 全局可变，多测试并行时可能交叉干扰；当前测试通过路径名隔离 mock 规避，后续如需高度并行可引入 registry reset hook。
- **下游 task 影响**：
  - task-3.2 importer-hermes（依赖 Importer 接口 + fallback 机制）
  - task-3.3 importer-openclaw（依赖 Importer 接口 + fallback 机制）
  - task-3.4 importer-agent-rules（依赖 Importer 接口 + fallback 机制）
- **Waiver 登记**：
  - **§2.5.1 RED 编译失败 Waived**：
    - 豁免对象：RED commit `a9e58ca` 为编译失败（undefined: Register/Resolve/NewFileFallbackImporter）而非功能性红测试
    - 原因：已 push 到 origin 的 feat 分支禁止 force-push 改历史（R6）；重做 RED 会要求重写已发布 commit 历史，违反 AGENTS.md §2 R6「禁止 git push --force 到任何已发布分支」。评审 Blocker 要求「重做 RED 或 Waive」，在不可改历史约束下选择 Waive。
    - 替代验证：GREEN commit `3b3ae46` 已包含完整实现并一次性通过全部 5 个功能性测试；后续 fix commit `08c7240`/`fe4ff94`/`4b03d1b` 均遵循先 RED 后 GREEN 节律（warning 断言先红后绿）。
    - 补齐条件：后续 task（3.2/3.3/3.4 及后续 phase）严格执行 §2.5.1「RED 为可编译+功能失败」
    - 负责人：主 agent / 评审 Agent（用户确认修复方案）
