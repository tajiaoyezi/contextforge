# Task `2.3`: `chunker — chunking + metadata 抽取 + provenance 维护`

> ✅ 已过 `/s2v-implement` §2A 前置审核（2026-05-21）：§3/§4/§5.2/§5.3 `<TBD-by-user>` 已清零、§6 AC 经用户审定接受。实时状态以下方 `**Status**` 字段为准；状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: In Progress

**Priority**: P0
**Owner**: tajiaoyezi
**Related Phase**: Phase 2 (index-core)
**Dependencies**: 2.2 (parser)

## 1. Background

把 parser 产出的解析单元切成检索用 `Chunk`，抽取 metadata，并维护 provenance（来源链）。chunking 策略需可配置以支撑 PRD §Technical Risks R3（不达标时按 code/markdown/log 分别调参）。

## 2. Goal

`chunker` 产出 `Chunk`（含 chunk_id / file_path / line_start / line_end / language / content / content_hash），并写入 `provenance`（importer/original_path/imported_at/source_modified_at）；chunking 策略可配置（按 code/markdown/log 分别策略）。

## 3. Scope

### In Scope

- 实现 AC1–AC5：Chunk 字段集完整 + provenance 多来源 + chunking 策略按 language 分组可配 + 流式安全 + content_hash 跨来源一致
- 消费 task-2.2 `parser::ParsedUnit`，产出 `Vec<Chunk>`（Chunk 携 chunk_id / file_path / line_start / line_end / language / content / content_hash / kind / provenance / metadata）
- chunking 策略按 language 分组独立可配（code / markdown / log / text fallback）：含 `max_chunk_lines` 行数上限 + `overlap_lines` 重叠 + `respect_parsed_units` 是否尽量按解析边界（heading / function / log_entry）切（直接缓解 PRD §Technical Risks R3）
- 维护 `Vec<Provenance>`：单 Chunk 可承载多来源（importer / original_path / imported_at / source_modified_at）
- content_hash 算法 v0.1 = **std-only FNV-1a-64**（手写，无新依赖），存储格式 `fnv1a64:<16-hex>`；算法名作前缀使未来升级 sha256/blake3 时旧 hash 仍可识别
- normalize 规则（最小集，AC5 跨来源一致需要）：CRLF→LF + 去除整体首尾空白 + 行末 trailing whitespace 折叠
- 文件锚点：`core/src/chunker/mod.rs`（在 task-1.3 placeholder `placeholder_ready()` 上实现，编译通过）

### Out Of Scope

- 实际 SHA-256 / BLAKE3（v0.1 用 FNV-1a-64 stub；真正密码学 hash 升级走未来 ADR + 独立 chore-dep PR；§10 下游影响记 memoryops）
- 写回 SQLite / Tantivy（task-2.4 indexer 负责，本 task 只产 in-memory Chunk）
- gRPC `Chunk` proto wire 表示与 in-memory Chunk struct 的 1:1 映射（本 task 富于 wire — 多 content_hash / provenance / metadata / kind 字段，wire encoder 留 indexer / exporter）
- 全文检索 tokenizer / embedding / boost / 召回评估（Phase 4 retriever / Phase 8 eval）
- normalize 算法进阶（CJK / Unicode NFC、注释剥离、stop-word — 留 Phase 4 检索调优）
- 嵌套 method / inner-class 切分（v0.1 沿用 parser 给的 ParsedUnit 边界，不二次细化）
- 二进制 / 超大单文件（>scanner 大小上限）的特殊处理（scanner 已拦截，chunker 只承诺合理大文件下不爆内存）

## 4. Users / Actors

- **parser**（task-2.2，上游强依赖）：提供 `Vec<ParsedUnit>` 与原始文件路径
- **indexer**（task-2.4，下游强依赖）：消费 `Vec<Chunk>` 写入 SQLite metadata + Tantivy 全文索引
- **memoryops**（Phase 5）：基于 `content_hash` 做去重 / 冲突 / 过期锚点（AC5 跨来源一致是必要前提）
- **importer**（task-3.x，跨 phase 并行）：在调用 chunker 前注入 `Vec<Provenance>` —— importer / original_path / imported_at / source_modified_at
- **retriever / eval**（Phase 4 / 8）：通过 indexer 间接消费 chunker 输出；本 task 不直接对接

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Technical Approach Canonical Record schema / §Technical Risks R3）
- `docs/specs/phases/phase-2-index-core.md`
- `docs/specs/tasks/task-1.1-proto.md`
- `docs/specs/tasks/task-2.2-parser.md`
- `docs/decisions/adr-002-sqlite-tantivy-layered-storage.md`
- `test/features/chunker.feature`

### 5.2 Imports

- 标准库：`std::path::{Path, PathBuf}`、`std::collections::HashMap`、`std::hash::{Hash, Hasher}`（其实手写 FNV，未用 std::hash trait，但保留以备 §10 注释参考）
- 内部：`crate::parser::ParsedUnit`（task-2.2 §5.3 已冻结产出类型）
- 错误类型：复用项目已有 `thiserror = "2.0.18"`（task-2.2 chore PR#11 引入，本 task 不引入新依赖）
- **R7 严格处理**：本 task **不引入新 crate**（content_hash 用 std-only 手写 FNV-1a-64；§2A 决策见 §10 §2A Decisions）；task agent 不修改 `core/Cargo.toml` / `Cargo.lock`

### 5.3 函数签名

```rust
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use crate::parser::ParsedUnit;

/// 检索切片（chunker 产出 → 喂给 indexer）。字段集对应 PRD §Technical Approach
/// Canonical Record v0.1 + AC1 列出的 7 个必含字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub chunk_id: String,            // "chk_<hash-prefix>_<ordinal>"，本 file 内 ordinal 单调
    pub file_path: String,           // 与 ParsedUnit 来源文件一致（绝对/相对路径由调用方决定）
    pub line_start: usize,
    pub line_end: usize,
    pub language: String,            // 沿用 ParsedUnit.language（"go"/"rust"/"markdown"/"log"/...）
    pub content: String,             // 原始内容（未 normalize；normalize 仅用于算 hash）
    pub content_hash: String,        // "fnv1a64:<16-hex>" — algo-prefixed (v0.1 stub, AC5 跨来源一致)
    pub kind: Option<String>,        // 沿用 ParsedUnit.kind（"heading"/"function"/"log_entry"/...）
    pub provenance: Vec<Provenance>, // AC2: 单 chunk 可承载多来源
    pub metadata: HashMap<String, String>,
}

/// 来源链（AC2 多来源）。importer / original_path / imported_at / source_modified_at
/// 与 PRD §Technical Approach Canonical Record `provenance[]` 字段集对齐。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Provenance {
    pub importer: String,            // 例 "hermes-memory" / "openclaw-workspace" / "local-fs"
    pub original_path: String,
    pub imported_at: String,         // RFC3339 / ISO 8601 字符串
    pub source_modified_at: String,
}

/// 单语言 chunking 配置（AC3 可配置 + R3 调优）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkConfig {
    pub max_chunk_lines: usize,      // 每个 chunk 最大行数（AC4 流式安全 + R3 调优）
    pub overlap_lines: usize,        // 邻接 chunk 重叠行数（R3 召回率调优；0 表关闭）
    pub respect_parsed_units: bool,  // true=尽量按 ParsedUnit 边界（heading/function/log_entry）切；false=纯定长
}

/// 按语言分组的策略集（AC3：code/markdown/log 分别可调）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkPolicy {
    pub code: ChunkConfig,
    pub markdown: ChunkConfig,
    pub log: ChunkConfig,
    pub text: ChunkConfig,           // 未知扩展名 / parser 降级为 text 时的兜底
}

impl Default for ChunkPolicy {
    fn default() -> Self { /* 提供合理默认：code 80 行/0 重叠/respect=true；markdown 60/4/true；log 200/0/false；text 100/0/false */ }
}

#[derive(thiserror::Error, Debug)]
pub enum ChunkError {
    #[error("io: {0}")] Io(#[from] std::io::Error),
    #[error("parse: {0}")] Parse(String),        // 透传 ParseError.to_string()
    #[error("invalid chunk config: {0}")] InvalidConfig(String),
}

/// 主入口（AC1/AC2/AC3/AC4/AC5）：把 parser 产出的解析单元切片为 Chunk。
/// - units: 上游 parser 的 ParsedUnit 流（按行号单调递增；同 file_path）
/// - file_path: 用于 Chunk.file_path + chunk_id 派生
/// - policy: 按 language 选择切分配置
/// - provenance: 调用方注入（importer / fs / etc），整段写入每个 Chunk
pub fn chunk_units(
    units: &[ParsedUnit],
    file_path: &Path,
    policy: &ChunkPolicy,
    provenance: Vec<Provenance>,
) -> Result<Vec<Chunk>, ChunkError>;

/// 便利入口：直接读文件 + 调 parser + chunk（集成测试 / CLI 使用）。
pub fn chunk_file(
    path: &Path,
    policy: &ChunkPolicy,
    provenance: Vec<Provenance>,
) -> Result<Vec<Chunk>, ChunkError>;

/// 公开：用同样规则计算 content_hash（memoryops 去重锚点；AC5 跨来源一致）。
/// 算法 v0.1 = FNV-1a-64；返回 "fnv1a64:<16-hex>"。normalize 见 §3 In-Scope 最小集。
pub fn content_hash(content: &str) -> String;
```

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Technical Approach Canonical Record v0.1): 每个 `Chunk` 含 chunk_id / file_path / line_start / line_end / language / content / content_hash。
- [ ] **AC2** (PRD §Technical Approach Canonical Record v0.1): `provenance[]` 写入 importer / original_path / imported_at / source_modified_at，可承载多来源。
- [ ] **AC3** (PRD §Technical Risks R3): chunking 策略可配置，对 code / markdown / log 分别可调参。
- [ ] **AC4** (PRD §User Flow 边界场景): 超大文件分块不爆内存（与 scanner 流式协同）。
- [ ] **AC5** (本 task 新增): content_hash 为后续 memoryops 去重锚点（normalized content hash），保证同内容跨来源 hash 一致。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 Chunk 字段完整 | SCEN-2.3.1 | TEST-2.3.1 | - | unit-test | Verified |
| AC2 provenance 多来源 | SCEN-2.3.2 | TEST-2.3.2 | - | unit-test | Verified |
| AC3 chunking 可配置 | SCEN-2.3.3 | TEST-2.3.3 | - | unit-test | Verified |
| AC4 大文件分块不爆内存 | SCEN-2.3.4 | TEST-2.3.4 | - | unit-test | Verified |
| AC5 content_hash 一致性 | SCEN-2.3.5 | TEST-2.3.5 | - | unit-test | Verified |

## 8. Risks

- 关联 PRD §Technical Risks **R3**（召回率）：chunking 策略直接影响召回，须可配置可回归。
- 关联 **R5**：provenance 与 importer 解耦（content_hash 锚点）。

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
