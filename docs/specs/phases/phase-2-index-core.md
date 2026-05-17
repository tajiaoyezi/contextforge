# Phase 2 · index-core

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。`/s2v-init` 生成，Status=Draft。§6 端到端 smoke 留 `<TBD-by-user>`，本 phase 最后一个 task 完工/合并前必须填实（`s2v_preflight_phase` C1）。

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

- `contextforge index ./sample_project` 能索引 ≥ 1000 个文件
- `.env`、`.ssh/`、`.git/objects/`、`node_modules/`、`target/` 默认跳过（denylist 见 PRD §Constraints）
- secret fixture 能被 redacted（保留 `[REDACTED:<TYPE>]` 类型标签，不改原文件）
- SQLite 中可查询 chunk metadata
- Tantivy 中可搜索到基础结果
- 单文件变更能触发基础增量更新

**端到端 smoke**：`<TBD-by-user>`（本 phase 最后一个 task=2.4 完工/合并前填实，例：对 `test/fixtures/shared/golden-*` 跑 `contextforge index` → 校验 SQLite chunk 计数 + Tantivy 命中 + secret fixture 已 redact 的 smoke 序列）

## 7. 阶段级风险

- 关联 PRD §Technical Risks **R4**（secret redaction 漏检或误报）：denylist 路径优先作第一道防线；redaction 保留占位符+类型标签；`scan --dry-run` 预检。
- 关联 **R6**（大仓库索引性能/资源不达标）：本 phase 起以真实大仓库为基准持续测；流式分块 + 单文件大小上限 + 默认排除目录；超阈值降级后台长任务（完整硬化在 Phase 8）。

## 8. Phase Definition of Done

- [ ] 本 phase 全部 task spec Status=Done 或 Waived
- [ ] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过
- [ ] 关联风险 R4 / R6 缓解措施已落地（denylist+redaction 生效、性能基准已建）
- [ ] adapter §Phase 状态索引该行 Status 同步更新
- [ ] team §4 Gate 3 phase smoke gate 通过后方可 merge 最后一个 task
