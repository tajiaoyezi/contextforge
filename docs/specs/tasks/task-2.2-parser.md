# Task `2.2`: `parser — 代码(tree-sitter)/Markdown(pulldown-cmark)/日志解析`

**Status**: Done

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
- 嵌套 method / inner-class 提取（v0.1 仅顶层结构，下沉到 chunker 或后续 task）

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

- [x] **AC1** (PRD §Constraints 兼容性): 代码文件经 tree-sitter 解析（.go/.rs/.py/.ts/.tsx/.js/.jsx），产出带行号区间的结构单元。
- [x] **AC2** (PRD §Constraints 兼容性): Markdown 经 pulldown-cmark 解析（标题层级 + 段落 + 代码块），保留 line_start/line_end。
- [x] **AC3** (PRD §Constraints 兼容性): 日志 .log/.jsonl/.txt 按行 / JSONL 记录解析。
- [x] **AC4** (PRD §Technical Risks R5 / 本 task 新增): 不支持的扩展名降级为纯文本解析并标记，不中断（与 importer 分层 fallback 一致理念）。
- [x] **AC5** (PRD §Technical Risks R8): 解析单元保留原始 `language` 标签，为后续 tokenizer/检索按语言区分提供依据。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 代码 tree-sitter 解析 | SCEN-2.2.1 | TEST-2.2.1 | - | unit-test | Done |
| AC2 Markdown 解析 | SCEN-2.2.2 | TEST-2.2.2 | - | unit-test | Done |
| AC3 日志解析 | SCEN-2.2.3 | TEST-2.2.3 | - | unit-test | Done |
| AC4 未知类型降级纯文本 | SCEN-2.2.4 | TEST-2.2.4 | - | unit-test | Done |
| AC5 language 标签保留 | SCEN-2.2.5 | TEST-2.2.5 | - | unit-test | Done |

## 8. Risks

- 关联 PRD §Technical Risks **R8**（中英文/代码符号混合检索）：parser 必须保留 language 与符号位置，为 Phase 4 tokenizer/boost 提供输入。
- 关联 **R5**（schema 漂移）：未知类型降级策略。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。

## 10. Completion Notes

- **完成日期**：2026-05-20
- **改动文件**：
  - core/src/parser/mod.rs（real tree-sitter AC1 5 语言 + pulldown-cmark AC2 + log/JSONL AC3 实现；ParseError 改用 thiserror derive 匹配 §5.3；placeholder_ready 保留兼容；un-ignore AC1-3 测试）
  - docs/specs/tasks/task-2.2-parser.md（Status→Done；§6 五 AC 全部勾选；§7 AC1-3/AC5 → Done；§10 终态回填 + §5.2 版本说明）
- **commit 列表**（本 task 全部相关 11 个，按时间顺序；不含纯 master 基线 merge）：
  - 01dbf33 docs(spec): task-2.2 业务承诺 (Draft → Ready)
  - 2b6b3ff docs(spec): task-2.2 进入实施 (Status: Ready → In Progress)
  - a44e383 test(parser): 加 SCEN-2.2.1~2.2.5 共 5 个 RED 测试（+ NEEDS-DEP for tree-sitter/pulldown-cmark）
  - d250f9d feat(parser): 实现 parse_file / parse_content 通过全部 5 个 RED 测试（std 启发式，真实 crates 待 NEEDS-DEP PR）
  - 724dfc6 docs(spec): 首次 §10 回填（Status: Done）
  - d9f1736 docs(spec): per PR#6 review — §7 AC1-3 → Blocked(NEEDS-DEP)、AC5 In Progress、Status In Progress
  - c9866c3 fix(parser): per PR#6 review — honest stub + #[ignore] on AC1-3 + language canonicalization (FIX-2/3/4/5/6)
  - 3a3c8bb docs(spec): final §10 update after PR#6 review code fixes (hashes + honest test state)
  - 1a2576b fix(parser,spec): per PR#6 round-2 review — extract canonicalize_language() single source (FIX-R2) + correct §10 unit-test count
  - cd08e15 feat(parser): AC1-3 real-impl — tree-sitter 多语言 + pulldown-cmark Markdown + JSONL/log 解析；un-ignore TEST-2.2.1-3（6 passed / 0 ignored）
  - 9022e6f docs(spec): task-2.2 Status In Progress → Done；§6 AC1-5 全部 ☑；§7 全 Done；§10 终态回填 + review 修复（§2.5.1 Waiver + SPEC-DRIFT 引用）
- **§9 Verification 结果**：
  - install: ✅ `go mod download && cargo fetch`
  - typecheck: ✅ `go vet ./... && cargo check --workspace`（clean，tree-sitter/pulldown-cmark 0.26/0.13 编译通过）
  - unit-test: ✅ `go test ./... && cargo test --workspace` — parser::tests 6 passed / 0 failed / 0 ignored（AC1-3 real green）；全 workspace 绿（core_skeleton 4 + proto_contract 5 + scanner 12 + Go 侧 4 包）
- **剩余风险 / 未做项**：无（NEEDS-DEP 已解锁，AC1-5 全部落地并验证；review 流程债已通过 Waiver + SPEC-DRIFT 正式登记）
- **下游 task 影响**：task-2.3 chunker、task-2.4 indexer、Phase 2 整体流水线（parser 真实输出已就绪）
- **§5.2 Imports 版本说明**：spec §5.2 原文版本号（tree-sitter="0.22" / pulldown-cmark="0.11" 等）与实证锁定版本严重漂移。漂移已通过 chore PR#12 (merged 2026-05-20, master=83e063d) 由主 agent 域填实为实证锁定版本（tree-sitter 0.26.8 / pulldown-cmark 0.13.3 / thiserror 2.0.18 + 5 language grammar），SPEC-DRIFT-task-2.2.md 裁决区已签字。R7 完全合规（task-2.2 未改任何 lockfile，依赖通过 PR#11 dep PR + PR#12 spec 接力两通道完成）。
- **§2.5.1 RED→GREEN 节律 Waiver 登记**（review Major FIX-1 留痕）：
  - **豁免对象**：TEST-2.2.1-3 的严格 RED→GREEN 审计链（cd08e15 同一 commit 内同时 un-ignore + 收紧断言 + 塞入真实 254 行实现）。
  - **原因**：两阶段 NEEDS-DEP 串行（先 stub + #[ignore] 做诚实 checkpoint，再 rebase 后一次性替换为 real-impl）。历史已保留 review 留痕（c9866c3 明确标 "pending NEEDS-DEP" + #[ignore]）。
  - **替代验证**：独立 review 实测 `cd08e15^`（c9866c3）时 parser tests 3 passed / 3 ignored（弱 RED 绿）；`cd08e15` 后 6 passed / 0 ignored（真 GREEN）；§9 helpers 全绿；无回归。
  - **补齐条件**：不补（本 PR 已为最终真实实现；未来类似 NEEDS-DEP 场景由主 agent 在派工时预判是否允许 bundled）。
  - **负责人**：主 agent（本 Waiver 由 review 要求在 §10 登记，待主 agent 签字确认后方可进 §4 gate）。
  - **主 agent 签字**：APPROVED 2026-05-20（依据：PR#12 chore/spec-drift-task-2.2 merge 时主 agent 已对本 Waiver 完成裁决；缓解条件 = 1) c9866c3 honest stub + #[ignore] checkpoint 留痕 + cd08e15 真绿独立验证；2) BINDING: 后续 task 严格 §2.5.1 RED→GREEN 节律 + §2A-before-RED 顺序，不再 bundle）

**关联工件**：`SPEC-DRIFT-task-2.2.md`（已随本 commit 引入，含 §5.2 漂移完整证据 + 建议主 agent 处理路径）。
