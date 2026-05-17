# Task `2.2`: `parser — 代码(tree-sitter)/Markdown(pulldown-cmark)/日志解析`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Ready

**Priority**: P0
**Owner**: tajiaoyezi
**Related Phase**: Phase 2 (index-core)
**Dependencies**: Phase 1（canonical schema）

## 1. Background

把扫描出的文件解析为结构化单元，供 chunker 切片。代码用 tree-sitter、Markdown 用 pulldown-cmark、日志按行/JSONL（PRD §Decisions Log D8 / §Constraints 兼容性 P0 导入源）。

## 2. Goal

`parser` 能解析 PRD §Constraints 列出的 P0 代码扩展名（.go/.rs/.py/.ts/.tsx/.js/.jsx/.md/.txt/.json/.yaml/.yml/.toml）与日志（.log/.jsonl/.txt），产出带 `language` 与位置信息（行号区间）的解析单元；不支持的类型降级为纯文本解析（不中断）。

## 3. Scope

### In Scope

- 实现 AC1–AC5：tree-sitter 解析 P0 代码（.go/.rs/.py/.ts/.tsx/.js/.jsx），pulldown-cmark 解析 Markdown（标题层级/段落/代码块），日志按行 + JSONL 解析
- 产出 `ParsedUnit` 结构（language + line_start/line_end + content + kind + metadata），与 PRD canonical ContextRecord 字段（language、file_path、line_*）对齐
- 未知扩展名降级为纯文本 + `language: "text"` 标记，不中断解析
- 所有解析保留原始位置信息，供 chunker（task-2.3）消费
- 模块入口：`core/src/parser/mod.rs`（在 task-1.3 占位上实现，编译通过）

### Out Of Scope

- embedding / 向量检索 / hybrid search（P1，Phase 4）
- chunking 策略与切片逻辑（task-2.3 负责）
- 写回源文件或任何第三方 Agent memory（只读导入 + draft 导出）
- 二进制 / 图片 / 超大单文件（>100MB）的特殊流式处理（基础降级 + 大小保护即可）
- 完整 symbol 提取 / CJK tokenizer 调优（R8 仅要求 language + 位置保留，boost 留 Phase 4）

## 4. Users / Actors

- scanner（task-2.1 并行）：提供文件路径 + 原始内容 + 初步 lang 猜测
- chunker（task-2.3，强依赖）：消费 ParsedUnit 流，执行切片 + provenance 合并
- core 集成测试 / 未来 indexer（2.4）/ eval harness（8.1）
- CLI / daemon / MCP 经内部 gRPC 间接调用 parser 能力

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Constraints 兼容性 P0 导入源 / §Technical Risks R8）
- `docs/specs/phases/phase-2-index-core.md`
- `docs/specs/tasks/task-1.1-proto.md`
- `docs/decisions/adr-008-core-library-selection.md`
- `test/features/parser.feature`

### 5.2 Imports

- `tree-sitter = "0.26.8"` + 对应 language crate（`tree-sitter-go = "0.25.0"` / `tree-sitter-rust = "0.24.2"` / `tree-sitter-python = "0.25.0"` / `tree-sitter-typescript = "0.23.2"` / `tree-sitter-javascript = "0.25.0"`）
- `pulldown-cmark = "0.13.3"`
- `thiserror = "2.0.18"`（错误定义）
- 标准库：`std::path::Path`, `std::fs`, `std::collections::HashMap`
- **R7 严格处理**：本 task 通过独立 `chore/dep-parser-crates` PR#11（merged 2026-05-19）引入依赖（R7 单一通道，主 agent 域），task agent 仅消费 master `core/Cargo.toml` / `Cargo.lock` 已锁定版本（实证 cargo add 解析为当前互兼容集，pulldown-cmark 0.13 与 0.11 API 不兼容 — `Tag::Heading`/`Tag::CodeBlock` 由 tuple struct 改为 named-field struct，代码须按 0.13 编写）；task agent 绝不直接修改 lockfile。: task-2.2 业务承诺 (Draft → Ready))

### 5.3 函数签名

```rust
use std::path::Path;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUnit {
    pub language: String,                    // "go" | "rust" | "markdown" | "log" | "json" | "yaml" | "text"
    pub line_start: usize,
    pub line_end: usize,
    pub content: String,
    pub kind: Option<String>,                // "heading" | "code_block" | "function" | "log_entry" | "text" ...
    pub metadata: HashMap<String, String>,
}

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("io: {0}")] Io(#[from] std::io::Error),
    #[error("unsupported language for {path:?}: {lang}")] Unsupported { path: std::path::PathBuf, lang: String },
    #[error("parse failed: {0}")] Other(String),
}

/// 主入口：根据扩展名自动选择解析策略（tree-sitter / pulldown-cmark / log / text fallback）
pub fn parse_file(path: &Path) -> Result<Vec<ParsedUnit>, ParseError>;

/// 显式指定 language 的解析（便于测试与特殊场景）
pub fn parse_content(path: &Path, source: &str, language_hint: &str) -> Result<Vec<ParsedUnit>, ParseError>;
```

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Constraints 兼容性): 代码文件经 tree-sitter 解析（.go/.rs/.py/.ts/.tsx/.js/.jsx），产出带行号区间的结构单元。
- [ ] **AC2** (PRD §Constraints 兼容性): Markdown 经 pulldown-cmark 解析（标题层级 + 段落 + 代码块），保留 line_start/line_end。
- [ ] **AC3** (PRD §Constraints 兼容性): 日志 .log/.jsonl/.txt 按行 / JSONL 记录解析。
- [ ] **AC4** (PRD §Technical Risks R5 / 本 task 新增): 不支持的扩展名降级为纯文本解析并标记，不中断（与 importer 分层 fallback 一致理念）。
- [ ] **AC5** (PRD §Technical Risks R8): 解析单元保留原始 `language` 标签，为后续 tokenizer/检索按语言区分提供依据。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 代码 tree-sitter 解析 | SCEN-2.2.1 | TEST-2.2.1 | - | unit-test | Not Started |
| AC2 Markdown 解析 | SCEN-2.2.2 | TEST-2.2.2 | - | unit-test | Not Started |
| AC3 日志解析 | SCEN-2.2.3 | TEST-2.2.3 | - | unit-test | Not Started |
| AC4 未知类型降级纯文本 | SCEN-2.2.4 | TEST-2.2.4 | - | unit-test | Not Started |
| AC5 language 标签保留 | SCEN-2.2.5 | TEST-2.2.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R8**（中英文/代码符号混合检索）：parser 必须保留 language 与符号位置，为 Phase 4 tokenizer/boost 提供输入。
- 关联 **R5**（schema 漂移）：未知类型降级策略。

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
