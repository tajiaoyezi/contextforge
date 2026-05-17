# Task `2.1`: `scanner — 文件扫描 + denylist/allowlist 过滤 + secret 扫描`

> ✅ 已过 `/s2v-implement` §2A 前置审核（2026-05-17）：§3/§4/§5.2/§5.3 的待定字段已清零、§6 AC 经用户审定接受。实施硬约束：只读消费 task-1.1 冻结 proto + task-1.2 denylist/allowlist 契约，禁止修改 `proto/`；scanner 先用 Rust stdlib 实现，避免 R7 新依赖冲突。实时状态以下方 `**Status**` 字段为准；状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: Done

**Priority**: P0
**Owner**: tajiaoyezi
**Related Phase**: Phase 2 (index-core)
**Dependencies**: Phase 1（canonical schema + proto）

## 1. Background

数据面入口：扫描本地目录，按 denylist/allowlist 过滤，并做 secret 扫描 + redaction（PRD §Constraints 安全 / §Technical Risks R4）。secret redaction 不改原文件，结果保留 `[REDACTED:<TYPE>]` 类型标签。

## 2. Goal

`scanner` 能遍历指定路径（ignore/walkdir），命中 denylist 路径默认跳过，allowlist 模型可配置；secret pattern 检测命中后产出 redacted 内容 + `redaction_status`，原文件不被修改；超大单文件走流式 + 大小上限保护。

## 3. Scope

### In Scope

- `core/src/scanner/` Rust 模块：本地目录递归扫描、文件发现、路径规范化与确定性排序（供后续 parser/chunker/indexer 消费）。
- 默认 denylist 过滤：等价消费 task-1.2 `DefaultDenylist()` 契约列出的敏感路径/模式；命中路径不读取内容、不进入扫描结果，并在 skip 列表中记录原因。
- allowlist 路径导入模型：支持按 collection 配置 allowlist 前缀；未显式 allow 的路径跳过；覆盖默认 denylist 必须显式确认。
- secret pattern 检测与 redaction：覆盖 API key / Bearer token / private key / AWS / GitHub token / 通用 password / cookie；产出 redacted 内容、`redaction_status` 与命中明细，不修改原文件。
- `dry_run` 扫描模式：返回将被 redact 的命中与跳过/扫描统计，但不产出用于索引的 redacted content。
- 单文件大小上限与流式读保护：默认 100MiB 上限，超限文件跳过且不一次性读入内存；普通文件按 bounded reader 读取。

### Out Of Scope

- `contextforge scan --dry-run` CLI 子命令参数解析与终端输出（Phase 6 CLI / task 6.1；本 task 只提供 Rust scanner `dry_run` 能力）。
- parser/chunker/indexer 写入、SQLite/Tantivy 持久化、增量索引调度（task 2.2/2.3/2.4）。
- audit log 写入与 redaction override 审计（task 5.3；本 task 暴露 override 事实供下游审计）。
- 新增第三方扫描/ignore/regex crate 或修改 `Cargo.toml` / `Cargo.lock`（若后续确认必须引入，按 R7 独立 chore-dep PR）。
- 修改 `proto/contextforge/v1/*` 或 canonical record 契约（若确需改动，立即写 `SPEC-DRIFT-task-2.1.md` 停止实施）。
- 二进制文件语义解析、语言识别、chunk metadata/provenance 细化（后续 parser/chunker/indexer）。

## 4. Users / Actors

- Phase 2 `indexer` / `parser` / `chunker`：消费 scanner 输出的可索引文件、redacted content 与 skip/redaction metadata。
- Go CLI / daemon（后续 task）：通过 gRPC/内部编排触发 Rust scanner 能力，读取 task-1.2 config 产生的 denylist/allowlist 后传入 scanner。
- 本地优先 / 隐私敏感开发者：依赖默认 denylist + secret redaction 避免敏感内容明文进入索引。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Constraints 安全 + §User Flow secret 命中示例）
- `docs/specs/phases/phase-2-index-core.md`
- `docs/specs/tasks/task-1.1-proto.md`
- `docs/specs/tasks/task-1.2-config.md`
- `docs/decisions/adr-004-local-first-privacy-baseline.md`
- `test/features/scanner.feature`

### 5.2 Imports

- Rust 标准库：`std::fs` / `std::io::{BufRead, BufReader, Read}` / `std::path::{Path, PathBuf, Component}` / `std::collections::BTreeSet` / `std::fmt` / `std::error::Error`（递归扫描、bounded read、路径匹配、错误类型）。
- 上游契约（只读）：task-1.2 `DefaultDenylist()` 的 16 项默认 denylist / allowlist 模型语义；Rust 侧复制等价默认值，不 import Go 包。
- 上游契约（只读）：task-1.1 冻结的 `ContextRecord.redaction_status` 取值语义（`none|partial|full`）；本 task 不修改 proto。
- 测试侧：Rust 标准库 `std::fs` + `std::env::temp_dir()` 生成隔离 fixture；不新增 dev-dependency。

### 5.3 函数签名

> Rust crate `contextforge_core::scanner`，落 `core/src/scanner/mod.rs`（adapter §Source areas `core/`）。

```rust
pub const DEFAULT_MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanOptions {
    pub denylist: Vec<String>,
    pub allowlist: Vec<std::path::PathBuf>,
    pub allow_denylist_override: bool,
    pub dry_run: bool,
    pub max_file_bytes: u64,
}

impl Default for ScanOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanReport {
    pub root: std::path::PathBuf,
    pub files: Vec<ScannedFile>,
    pub skipped: Vec<SkippedPath>,
    pub redaction_hits: Vec<SecretMatch>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFile {
    pub path: std::path::PathBuf,
    pub original_size_bytes: u64,
    pub content: Option<String>,
    pub redacted_content: Option<String>,
    pub redaction_status: RedactionStatus,
    pub redactions: Vec<SecretMatch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionStatus { None, Partial, Full }

impl RedactionStatus {
    pub fn as_str(self) -> &'static str; // "none" | "partial" | "full"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecretKind {
    ApiKey,
    BearerToken,
    PrivateKey,
    AwsAccessKey,
    AwsSecretKey,
    GithubToken,
    Password,
    Cookie,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretMatch {
    pub kind: SecretKind,
    pub path: Option<std::path::PathBuf>,
    pub line: usize,
    pub start: usize,
    pub end: usize,
    pub redaction: &'static str, // e.g. "[REDACTED:GITHUB_TOKEN]"
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedPath {
    pub path: std::path::PathBuf,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    Denylisted(String),
    NotAllowlisted,
    TooLarge { size: u64, max: u64 },
    NotUtf8,
    Symlink,
}

#[derive(Debug)]
pub enum ScanError {
    Io { path: std::path::PathBuf, source: std::io::Error },
    DenylistOverrideRequired,
    NotAllowlisted,
    FileTooLarge { path: std::path::PathBuf, size: u64, max: u64 },
    Symlink { path: std::path::PathBuf },
}

pub fn default_denylist() -> Vec<String>;
pub fn scan_path(root: impl AsRef<std::path::Path>, options: &ScanOptions) -> Result<ScanReport, ScanError>;
pub fn scan_file(path: impl AsRef<std::path::Path>, options: &ScanOptions) -> Result<ScannedFile, ScanError>;
pub fn detect_secrets(content: &str) -> Vec<SecretMatch>;
pub fn redact_content(content: &str) -> (String, RedactionStatus, Vec<SecretMatch>);
pub fn is_denied(path: impl AsRef<std::path::Path>, denylist: &[String]) -> Option<String>;
pub fn is_allowlisted(path: impl AsRef<std::path::Path>, allowlist: &[std::path::PathBuf]) -> bool;
```

- SCEN/TEST-2.1.1 → `scan_path` 默认跳过 `.env` / `.ssh/` / `.git/objects/` / `node_modules/` / `target/`（AC1）
- SCEN/TEST-2.1.2 → `ScanOptions.allowlist` 限定导入路径；denylist override 未显式确认时报 `DenylistOverrideRequired`（AC2）
- SCEN/TEST-2.1.3 → `redact_content` / `scan_file` 检测并 redacts API key / Bearer / private key / AWS / GitHub token / password / cookie，不改原文件（AC3）
- SCEN/TEST-2.1.4 → `ScanOptions.dry_run=true` 时返回 `redaction_hits`，`ScannedFile.redacted_content=None`，不产出索引内容（AC4）
- SCEN/TEST-2.1.5 → `scan_file` 对 `metadata.len() > max_file_bytes` 返回 `TooLarge` skip，不读取内容；普通文件用 bounded reader（AC5）

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [x] **AC1** (PRD §Implementation Phases Phase 2 Exit Criteria): `.env`、`.ssh/`、`.git/objects/`、`node_modules/`、`target/` 等 denylist 路径默认跳过，不进扫描结果。
- [x] **AC2** (PRD §Constraints 安全): allowlist 路径导入模型生效；用户覆盖 denylist 须显式确认。
- [x] **AC3** (PRD §Technical Risks R4 / §User Flow): secret pattern（API key / Bearer token / private key / AWS / GitHub token / 通用 password / cookie）命中后产出 redacted 内容 + `redaction_status`，**原文件不被修改**，保留 `[REDACTED:<TYPE>]` 类型标签。
- [x] **AC4** (PRD §Technical Risks R4): 提供 `scan --dry-run` 预检（列出将被 redact 的命中，不写索引）。
- [x] **AC5** (PRD §User Flow 边界场景): 超大单文件（如 100MB 日志）走流式 + 大小上限保护，内存不爆。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 denylist 默认跳过 | SCEN-2.1.1 | TEST-2.1.1 | - | unit-test | Done |
| AC2 allowlist 模型 | SCEN-2.1.2 | TEST-2.1.2 | - | unit-test | Done |
| AC3 secret redact 不改原文件 | SCEN-2.1.3 | TEST-2.1.3 | - | unit-test | Done |
| AC4 scan --dry-run 预检 | SCEN-2.1.4 | TEST-2.1.4 | - | unit-test | Done |
| AC5 超大文件流式保护 | SCEN-2.1.5 | TEST-2.1.5 | - | unit-test | Done |

## 8. Risks

- 关联 PRD §Technical Risks **R4**（secret redaction 漏检或误报）：denylist 第一道防线；pattern 可扩展；dry-run 预检；override 写 audit log（audit 在 task 5.3）。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。

## 10. Completion Notes

- **完成日期**：2026-05-17
- **改动文件**：
  - `docs/specs/tasks/task-2.1-scanner.md`（修改 — §2A 审核填 §3/§4/§5.2/§5.3、§6 勾选、§7 Done、§10 回填、Status Done）
  - `test/features/scanner.feature`（修改 — 补齐 SCEN-2.1.1~2.1.5 Given/When/Then）
  - `core/src/scanner/mod.rs`（修改 — stdlib scanner API + 默认 denylist、allowlist/override、bounded read、secret detection/redaction、dry-run、大小上限 skip、symlink skip）
  - `core/tests/scanner.rs`（新增/修改 — TEST-2.1.1~2.1.5 + review feedback 回归测试）
- **commit 列表**：
  - `9e2cfda` docs(spec): task-2.1 Draft → Ready（§2A 前置审核通过，5 AC accepted）
  - `c4bb16a` docs(spec): task-2.1 进入实施 (Status: Ready → In Progress)
  - `9a78e98` test(scanner): 加 SCEN-2.1.1~2.1.5 共 5 个 RED 测试
  - `4b91b5a` feat(scanner): 实现 stdlib 文件扫描 + 过滤 + secret redaction 通过全部 5 个测试
  - `2aaba63` test(scanner): 加 review feedback RED 覆盖 scan_file 安全与边界场景
  - `c2adf8a` feat(scanner): 修复 review feedback 的 scan_file 安全与边界行为
  - 本回填 docs(spec) commit 见步 11.A（§10 回填 + §7 Done + Status Done）
- **§9 Verification 结果**：
  - install: ✅ `go mod download && cargo fetch`
  - typecheck: ✅ `go vet ./... && cargo check --workspace`
  - unit-test: ✅ `go test ./... && cargo test --workspace`；Rust 21 passed / 0 failed（scanner 12 + core_skeleton 4 + proto_contract 5），Go packages passed
- **剩余风险 / 未做项**：
  - secret detector 为 v0.1 stdlib 启发式实现，覆盖本 task AC3 枚举类型并已补 GitHub token 常见前缀、Bearer 空白、`x-api-key` 与 key 边界；base64 credential / URL 内嵌 credential 的更高召回率需后续按 R7 评估 regex/secret-scanner crate 或规则库。
  - `scan_file` 直接调用已 fail-closed：denylist 需显式 override、allowlist 外拒绝、超限文件返回 `ScanError::FileTooLarge`、symlink 返回 `ScanError::Symlink`。`scan_path` 仍以 `SkippedPath` 聚合这些 skip 语义。
  - `contextforge scan --dry-run` CLI 输出、indexer 写入、audit log 均按 §3 Out Of Scope 留给 task 6.1 / 2.4 / 5.3。
- **下游 task 影响**：
  - task 2.4 indexer 可消费 `ScanReport.files` / `SkippedPath` / `redaction_hits`，并把 `redaction_status.as_str()` 写入 canonical `ContextRecord.redaction_status`。
  - task 2.2 parser / task 2.3 chunker 可从 `ScannedFile.content` 或 `redacted_content` 获取可索引文本；dry-run 模式不会产出 indexable content。
  - task 5.3 audit 可基于 `ScanError::DenylistOverrideRequired` 和显式 override 路径补审计事件。
  - task 6.1 CLI 可把 `ScanOptions.dry_run=true` 包装成 `contextforge scan --dry-run`。
  - 无 proto / Cargo.toml / Cargo.lock 改动；未触发 R7 依赖 PR。
