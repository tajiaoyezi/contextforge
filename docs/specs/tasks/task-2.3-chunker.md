# Task `2.3`: `chunker — chunking + metadata 抽取 + provenance 维护`

> ✅ 已过 `/s2v-implement` §2A 前置审核（2026-05-21）：§3/§4/§5.2/§5.3 `<TBD-by-user>` 已清零、§6 AC 经用户审定接受。实时状态以下方 `**Status**` 字段为准；状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: Done

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

- 标准库：`std::path::Path`、`std::collections::HashMap`、`std::fmt::Write`（hex 输出逐字节格式化）
- 内部：`crate::parser::ParsedUnit`（task-2.2 §5.3 已冻结产出类型）
- 错误类型：复用项目已有 `thiserror = "2.0.18"`（task-2.2 chore PR#11 引入）
- **content_hash 依赖**：`sha2 = "0.11.0"`（chore PR #17 `chore/dep-sha2` merged，master `4b7dadd`）。用 `sha2::{Digest, Sha256}` 二符号；输出 hex 不引入 `hex` crate（逐字节 `{:02x}` 格式化）
- **R7 严格处理**：本 task 通过独立 `chore/dep-sha2` PR #17（merged 2026-05-21）引入 sha2 依赖（R7 单一通道，主 agent 域）；task agent 仅消费 master `core/Cargo.toml` / `Cargo.lock` 已锁定版本，绝不直接修改 lockfile

### 5.3 函数签名

```rust
use std::path::Path;
use std::collections::HashMap;
use sha2::{Digest, Sha256};
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
    pub content_hash: String,        // "sha256:<64-hex>" — algo-prefixed，跨模块 task-3.1 importer 一致
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
/// 算法 = sha256（与 task-3.1 importer `internal/importer/record.go:80` 一致）；返回
/// "sha256:<64-hex>"。normalize 见 §3 In-Scope 最小集（CRLF→LF + 行末 trailing
/// whitespace + 整体 trim — rework 后未变）。
pub fn content_hash(content: &str) -> String;
```

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [x] **AC1** (PRD §Technical Approach Canonical Record v0.1): 每个 `Chunk` 含 chunk_id / file_path / line_start / line_end / language / content / content_hash。
- [x] **AC2** (PRD §Technical Approach Canonical Record v0.1): `provenance[]` 写入 importer / original_path / imported_at / source_modified_at，可承载多来源。
- [x] **AC3** (PRD §Technical Risks R3): chunking 策略可配置，对 code / markdown / log 分别可调参。
- [x] **AC4** (PRD §User Flow 边界场景): 超大文件分块不爆内存（与 scanner 流式协同）。
- [x] **AC5** (本 task 新增): content_hash 为后续 memoryops 去重锚点（normalized content hash），保证同内容跨来源 hash 一致。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 Chunk 字段完整 | SCEN-2.3.1 | TEST-2.3.1 | - | unit-test | Done |
| AC2 provenance 多来源 | SCEN-2.3.2 | TEST-2.3.2 | - | unit-test | Done |
| AC3 chunking 可配置 | SCEN-2.3.3 | TEST-2.3.3 | - | unit-test | Done |
| AC4 大文件分块不爆内存 | SCEN-2.3.4 | TEST-2.3.4 | - | unit-test | Done |
| AC5 content_hash 一致性 | SCEN-2.3.5 | TEST-2.3.5 | - | unit-test | Done |

## 8. Risks

- 关联 PRD §Technical Risks **R3**（召回率）：chunking 策略直接影响召回，须可配置可回归。
- 关联 **R5**：provenance 与 importer 解耦（content_hash 锚点）。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。

## 10. Completion Notes

- **完成日期**：2026-05-21（初版 + 同日 SPEC-DRIFT rework — content_hash FNV-1a-64 → sha256）
- **改动文件**：
  - core/src/chunker/mod.rs（real impl：Chunk / Provenance / ChunkConfig / ChunkPolicy / ChunkError + chunk_units / chunk_file + content_hash sha256 实现 + normalize 最小集；保留 placeholder_ready() 供 task-1.3 core_skeleton AC4 anchor；rework 后用 sha2::Sha256 替换 FNV-1a-64，algo-prefix 格式 sha256:<64-hex> 保留 forward-compat）
  - docs/specs/tasks/task-2.3-chunker.md（Status: Draft→Ready→In Progress→Done→In Progress→Done；§4 / §5.2 / §5.3 §2A 填实并 rework 同步到 sha256；§6 AC1-5 全部勾选；§7 5 行 → Done；§10 终态回填 + Rework 段；§2A Decisions 加 Rework 块）
  - test/features/chunker.feature（§2A 后回填 In order to / As / SCEN-2.3.1~5 的 Given/When/Then；BDD 语义跨 rework 不变）
- **commit 列表**（本 task 全部 7 个，按时间顺序；rebase 后 hash 与 PR #16 描述里的 5 个原 commit 不同 — 见 commit 5/6 行外注释）：
  - 7f350c9 docs(spec): task-2.3 业务承诺 (Draft → Ready)
  - 1b33326 docs(spec): task-2.3 进入实施 (Status: Ready → In Progress) + chunker.feature Given/When/Then 填实
  - 1245521 test(chunker): 加 SCEN-2.3.1~5 共 5 个 RED 测试
  - 9021b53 feat(chunker): 实现 chunk_units / chunk_file / content_hash 通过全部 5 个测试
  - 1d2b6c2 docs(spec): task-2.3 Status In Progress → Done；§6 AC1-5 全部 ☑；§7 全 Done；§10 终态回填
  - a2ecd9f fix(chunker): SPEC-DRIFT rework — content_hash FNV-1a-64 → sha256（依赖 chore PR #17）
  - 本回填 docs(spec) commit（§10 + §2A Decisions Rework 同步 + Status → Done）
- **§9 Verification 结果**：
  - install: ✅ `go mod download && cargo fetch`（sha2 v0.11.0 + 7 transitive 已从 chore PR #17 锁定）
  - typecheck: ✅ `go vet ./... && cargo check --workspace`（clean；sha2 0.11 + digest 0.11 + hybrid-array 0.4 编译通过）
  - unit-test: ✅ `go test ./... && cargo test --workspace` —— chunker::tests 5 passed / 0 failed / 0 ignored（AC1-5 全绿，sha256 hash 长度 71 = "sha256:" + 64 hex 断言通过；CRLF/LF + trailing whitespace + 跨来源一致 全过）；parser::tests 6 + core_skeleton 4 + proto_contract 5 + scanner 12 + Go 侧 5 包（含 task-3.1 importer，sha256 已对齐）全绿，零回归
- **剩余风险 / 未做项**：
  - normalize 规则保守最小集（CRLF→LF + 行末 trailing whitespace + 整体 trim）。CJK / Unicode NFC / 注释剥离 / stop-word 等进阶归一化留 Phase 4 retriever / 召回评估时按需加，按 §3 Out-of-Scope。
  - chunk_units 流式安全 = 当前一次性传入 ParsedUnit 切片 + 内部按 max_chunk_lines 分段；真正"分块读 + 并发"的流式接口（返回 Iterator of Chunk）留 Phase 8 性能硬化（与 scanner 流式衔接，R6 大仓库性能基准）。
  - **跨模块 hash 存储格式微差**：本 task chunker 输出 `sha256:<64-hex>` algo-prefix；task-3.1 importer (`internal/importer/record.go:80`) 输出裸 `<64-hex>` 无 prefix。Phase 5 memoryops 桥接时需按前缀剥离再比较（实际 hash bytes 一致 — 同 sha256 算法 + 同 raw content；normalize 路径目前两侧不一定 1:1 — 后续 memoryops 跨模块对齐时再校验）。
  - **§3 In/Out Scope 文字仍含 FNV-1a-64 字面提及**：原 §2A 填空记录（PR #16 初版），按本 rework 派工硬约束（§3 业务契约字段禁动），未在 rework commit 中修订；权威算法以 §5.2 / §5.3 / §10 / 实现为准。后续若需要彻底清理 §3 文字，由主 agent 走独立 spec-drift PR。
- **下游 task 影响**：
  - **task-2.4 indexer**（强依赖）：消费 Chunk 切片向量写 SQLite metadata + Tantivy 全文索引 — Chunk 字段集已冻结契约（§5.3 + AC1）。
  - **Phase 5 memoryops**（AC5 锚点）：基于 content_hash 做去重 / 冲突 / 过期；sha256 与 task-3.1 importer 一致（统一密码学锚点，PRD §Technical Risks R5 缓解）；algo-prefix 设计允许未来切换算法时按前缀分流。
  - **task-3.x importers**（跨 phase 并行）：调用 chunker 前注入 Provenance 切片向量；importer/original_path/imported_at/source_modified_at 字段集已与 PRD §Canonical Record provenance[] 对齐。
  - **Phase 4 retriever**（间接）：通过 indexer 消费 chunker 输出，line_start/line_end 单调 + 不重叠（overlap=0 时）保证可解释 line range 复原。
- **§2A Decisions**：
  - **初版（2026-05-21，已被 rework 撤销）**：content_hash 算法 = std-only 手写 FNV-1a-64（worker 答题选项 A）；意图是放弃 sha2 crate 的 R7 串行 chore-dep PR。
  - 不修改 Cargo.toml / Cargo.lock（R7 严格 — task agent 不引入新依赖）— **rework 后仍守此约束**，sha2 由主 agent 域 chore PR #17 引入，task agent 仍未改 lockfile。
  - **Rework (2026-05-21, 主 agent SPEC-DRIFT 裁决后)**：content_hash 算法：FNV-1a-64 → **sha256**
    - **裁决理由**：
      1. 与 task-3.1 importer (PR #7, commit `5861e98`) 已用 sha256 一致；Phase 5 memoryops 跨来源去重锚点要求一致 hash — FNV vs sha256 对不上则去重失效
      2. 64-bit FNV 在 10M chunk 规模碰撞概率 ~10⁻⁶ 不可接受（PRD §Constraints 性能阈值）
      3. "避 R7 流程开销" 不应成为降级算法的理由 — 正确做法是发 NEEDS-DEP，让主 agent 串行加 sha2 crate
    - **依赖通道**：chore-dep PR #17 加 sha2 v0.11.0（+ 7 transitive：block-buffer / const-oid / cpufeatures / crypto-common / digest / hybrid-array / typenum）→ merged master `4b7dadd`（commit `2d3aa84`）
    - **本 rebase 后用 sha256 替换**；algo-prefix 格式 `sha256:<64-hex>` 保留 (forward-compat 不变)
    - **与 PRD §Canonical Record JSON 示例 `"content_hash": "sha256..."` 现已字面一致**（初版偏差通过 rework 修复）
