# Phase 2 · index-core

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。**Status=Done（2026-05-22 收口）**；§6 端到端 smoke 已填实并跑过；4 task 全 Done（task-2.1 PR #5 / task-2.2 PR #6 / task-2.3 PR #16 / task-2.4 PR #24 / chore phase-2-closeout）。

## 1. 阶段目标

`contextforge index ./project` 建立本地 Tantivy + SQLite 索引；denylist/allowlist 与 secret redaction 生效；支持基础增量索引，完整长任务恢复在 Phase 8 硬化。来源：PRD §Implementation Phases Phase 2。

## 2. 业务价值

实现 PRD 核心能力 #1（多 Agent 中立的本地上下文统一接入与索引）的数据面地基：把本地代码/文档/日志变成可检索、带 provenance、已脱敏的本地索引。直接支撑成功指标「检索性能 10 万 chunk P95 < 500ms」「真实接入度 ≥ 1000 文件/10000 chunk」。

## 3. 涉及模块

- `scanner`（Rust）：文件扫描 + denylist/allowlist 过滤 + secret 扫描
- `parser`（Rust）：代码(tree-sitter)/Markdown(pulldown-cmark)/日志解析
- `chunker`（Rust）：chunking + metadata 抽取 + provenance 维护
- `indexer`（Rust）：Tantivy 全文索引 + SQLite metadata/chunk 存储 + 增量更新
- 文件锚点：`core/src/scanner/` · `core/src/parser/` · `core/src/chunker/` · `core/src/indexer/` · `core/tests/`

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 2.1 | scanner | `../tasks/task-2.1-scanner.md` |
| 2.2 | parser | `../tasks/task-2.2-parser.md` |
| 2.3 | chunker | `../tasks/task-2.3-chunker.md` |
| 2.4 | indexer | `../tasks/task-2.4-indexer.md` |

## 5. 依赖关系

- **依赖**：Phase 1（canonical record schema + proto 契约）
- **可并行**：是 —— 可与 Phase 3（agent-importers）并行（二者均只依赖 Phase 1 冻结契约，分属 `core/`(Rust) 与 `internal/importer/`(Go)，无源文件写冲突）
- **Phase 内顺序**：2.1 scanner ∥ 2.2 parser → 2.3 chunker（dep 2.2）→ 2.4 indexer（dep 2.1/2.3，含 `contextforge index`）

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（参考 — PRD §Implementation Phases Phase 2 Exit Criteria，用户审定后落实）**：

- [x] `contextforge index ./sample_project` 能索引 ≥ 1000 个文件（task-2.4 / 实测 1010 files 0.37s）
- [x] `.env`、`.ssh/`、`.git/objects/`、`node_modules/`、`target/` 默认跳过（task-2.1 scanner denylist）
- [x] secret fixture 能被 redacted（保留 `[REDACTED:<TYPE>]` 类型标签，不改原文件）（task-2.1 scanner redact + indexer 实证 secret 不入 Tantivy）
- [x] SQLite 中可查询 chunk metadata（task-2.4 indexer / chunks 表 + 2 INDEX）
- [x] Tantivy 中可搜索到基础结果（task-2.4 indexer / 全文倒排 + BM25）
- [x] 单文件变更能触发基础增量更新（task-2.4 indexer / reindex_file 内容 hash 比对）

**端到端 smoke**（2026-05-22 chore PR `chore/phase-2-closeout` 验证）：

`cargo test --test phase2_smoke -- --nocapture` 全绿：

- 入口：`core/tests/phase2_smoke.rs` 含 `#[test] fn phase_2_end_to_end_smoke()`（task-2.4 §2A 选项 A 决策；主 agent §4 Gate 3 精准抓）
- 验证项（按 task-2.4 AC1-5 端到端覆盖）：
  - AC1 合成 fixture ≥3 normal files indexed（denied 跳过）
  - AC2 SQLite chunks > 0 + Tantivy 索引 `phase2smokemarkerz3q1` 命中
  - AC3 `.env` 含 `plain_secret` / `plaintext_smoke_password_should_be_skipped` 不入索引（scanner denylist）
  - AC3 `config.md` 含 `AKIAIOSFODNN7EXAMPLE` AWS key 已 redact（Tantivy 搜不到字面值）
- 完整 §6 AC 覆盖：task-2.4 spec §6 AC1-5 + indexer unit tests 4 + phase2_smoke 1
- 全 workspace verify：`cargo test --workspace` 37 passed (Rust) + `go test ./...` 8 packages 全绿
- 实测性能：AC1 1010 files 0.37s — 远超 PRD §6 阈值

**Scope 注**：phase-2 smoke 用 Rust 集成测试作为 §4 Gate 3 精准抓入口；CLI `contextforge index` 端到端在 Phase 6 task-6.1 实现后由 Phase 8 task-8.3 release smoke 接管。

## 7. 阶段级风险

- 关联 PRD §Technical Risks **R4**（secret redaction 漏检或误报）：denylist 路径优先作第一道防线；redaction 保留占位符+类型标签；`scan --dry-run` 预检。
- 关联 **R6**（大仓库索引性能/资源不达标）：本 phase 起以真实大仓库为基准持续测；流式分块 + 单文件大小上限 + 默认排除目录；超阈值降级后台长任务（完整硬化在 Phase 8）。

## 8. Phase Definition of Done

- [x] 本 phase 全部 task spec Status=Done 或 Waived（2.1/2.2/2.3/2.4 全 Done）
- [x] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过
- [x] 关联风险 R4 / R6 缓解措施已落地（scanner denylist + secret redact 永远先于 indexer / 流式分块 + AC1 1010 files 0.37s 性能基线）
- [x] adapter §Phase 状态索引该行 Status 同步更新（chore PR `chore/phase-2-closeout`）
- [x] team §4 Gate 3 phase smoke gate **forward-looking** 通过（本 chore PR 先填实 §6 smoke → PR #24 merge → Gate 3 自然 pass，按 reviewer PR #24 推荐路径 a）
