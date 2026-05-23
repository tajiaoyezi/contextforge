# Task `6.3`: `exporter — canonical JSONL / Markdown bundle / agent draft 导出 + 二次 secret scan`

> ✅ 已过 `/s2v-implement` §2A 前置审核（2026-05-23，主 agent 与用户预先审定，worker 终端可直接进入 RED）：§3/§4/§5.2/§5.3 `<TBD-by-user>` 已清零、§6 AC 经用户审定接受、A/B/C/D/E 五决策已确认（A. 数据源=collection-wide + stale 默认过滤、B. 三格式=jsonl + md-bundle(.tar.gz multi-file) + agent-draft 4 .md、C. AC3 secret scan = Go inline sanity hit-count check（cross-language 修正不重做 task-2.1 完整 detection）、D. AC4 fidelity = exporter.CalcFidelity helper + 内部 fixture、E. AC5 Phase 6 端到端 smoke = 本 task 填实 phase-6 spec §6 — 详见 §10 §2A Decisions）。实时状态以下方 `**Status**` 字段为准；状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: Done

**Priority**: P0
**Owner**: tajiaoyezi
**Related Phase**: Phase 6 (cli-api-export)
**Dependencies**: 6.1 (cli-search) — 已 merged 6a80f4c；与 task-6.2 同期并行（claude-work1 跑 6.2 / 你跑 6.3）

## 1. Background

跨 Agent 上下文迁移的导出侧（PRD §Core Capabilities #5 / §Decisions Log D5）。导出一律 draft/bundle 不写回；export 前二次 secret scan（PRD §Technical Risks R4 / §Constraints 安全）。是 Phase 6 末批 task — Phase 6 phase-last 一般由 6.2 / 6.3 中后 merge 那个触发（主 agent 调度域）；本 task AC5 提供 Phase 6 端到端 smoke 落点。

## 2. Goal

`contextforge export --format jsonl|markdown-bundle|agent-draft` 把选定 collection 导出为 canonical JSONL / Markdown bundle / Agent rule draft；导出前执行 sanity hit-count secret scan（cross-language 修正：不重做 task-2.1 完整 detection，作为 defense-in-depth）；迁移后结构化字段保真率经 fixture 计算 ≥ 80%；不写回任何第三方 Agent。

## 3. Scope

### In Scope

- **新增 `contextforge export` 子命令（实施 task-1.4 已 register 但 not-implemented 的 `export` 子命令）**：
  - `internal/cli/cli.go` Execute 内 dispatch case `"export"` 改 dispatch 到本 task 新增 `runExport(args, stdout, stderr)`（替换 task-1.4 default 的 "not implemented"）
  - flags（stdlib `flag.FlagSet`）：
    - `--format=<jsonl|markdown-bundle|agent-draft>` 必填（无 default — usage error if 缺）
    - `--collection=<id>` collection ID（default = "default"）
    - `--data-dir=<path>` data 根目录（default = `config.DefaultRootDir()`）
    - `--output=<path>` 输出路径（jsonl → file path / md-bundle → .tar.gz path / agent-draft → output dir）
    - `--include-stale` bool 默认 false（默认 stale 过滤，复用 task-5.2 `lifecycle.Mark + FilterStale`）
- **新增 Go 子包 `internal/exporter/`（§2A 决策 A：collection-wide pipeline）**：
  - 公开 API：
    - `func Export(ctx context.Context, opts Options) (*Result, error)`
    - `type Options struct { Format Format; Collection string; DataDir string; Output string; IncludeStale bool }`
    - `type Format string` 枚举: `FormatJSONL` / `FormatMarkdownBundle` / `FormatAgentDraft`
    - `type Result struct { RecordsExported int; OutputPath string; FidelityScore float64; SecretHits []SecretHit }`
- **collection 读取（§2A 决策 A：daemon.Search pseudo full-scan，避免 R7 chore-dep）**：
  - `internal/exporter/source.go`：`loadRecords(ctx, dataDir, collection string) ([]*contextforgev1.ContextRecord, error)`
  - 走 `daemon.Search(query="*", top_k=large)` pseudo full-scan（v0.1 P0 选 — 避免引 SQLite Go driver R7 chore-dep；复用 task-6.1 已 wire 的 daemon.Search 入口）
  - **fallback / future**：如 RED 阶段发现 Tantivy BM25 `*` 不全集（standard query parser 不一定接受 wildcard 单 token） → 写 `SPEC-DRIFT-task-6.3.list-chunks.md` 报主 agent 串行加 gRPC RPC `ListAllChunks(collection_id)`（add-only proto 扩展）；SQLite 直读由于 R7 chore-dep PR 成本高仅作 future 备选
  - 复用 retriever 已有的 SearchResult → ContextRecord 反向映射（如未有则 build minimal mapping）
  - 默认 stale 过滤：调 `lifecycle.Mark(records, SystemOracle{})` + `lifecycle.FilterStale(records, marks)`（复用 task-5.2 ✅）；`--include-stale` 透传 bypass
- **三格式实现（§2A 决策 B）**：
  - **JSONL（`internal/exporter/jsonl.go`）**：
    - 一行一个 `*contextforgev1.ContextRecord` 用 stdlib `encoding/json` marshal
    - 写到 `opts.Output` (file path)；目录不存在 → 创建（0700）；file mode 0600
  - **Markdown bundle（`internal/exporter/markdown.go`）**：
    - 每 record / chunk 生成一个 `.md` 文件（按 source_uri 或 file_path 作为分组 key 决定文件路径；分组后聚合多 chunk 到同一 .md 用 H2 / H3 分级）
    - 文件内容 markdown frontmatter（YAML）含 id / collection_id / source_type / source_uri / language / agent_scope / tags 等结构化字段（per ContextRecord 23 字段 23 个 YAML key）
    - body 含 `content`（已 redacted）+ provenance 段
    - 多个 .md 文件 + manifest.json（meta）打包为 `.tar.gz`（stdlib `archive/tar` + `compress/gzip`）
    - 写到 `opts.Output` (.tar.gz path)；file mode 0600
  - **Agent draft（`internal/exporter/agentdraft.go`）**：
    - 生成 4 个 `.md` draft 到 `opts.Output` 目录（mode 0700）：
      - `MEMORY.md` — agent_scope=memory 的 records；Hermes-style top-level memories
      - `USER.md` — agent_scope=user 的 records；user-level preferences / context
      - `AGENTS.md` — agent_scope=agents 或所有 agent 配置；含 Hermes-style 团队约定 / multi-agent topology（如有）
      - `CLAUDE.md` — Claude-specific 配置（agent_scope=claude）
    - 每文件 markdown 格式 + bash code block 示例（如 Hermes 风格 `# Project memories\n- rule 1`）
    - 不写回第三方 Agent 目录（AC2 硬约束 — Output 必须是用户给的 dir，禁写到 `~/.cursor/` / `~/.claude/` 等保护路径）；启动 check Output 路径是否在保护列表 → 拒绝
- **AC3 二次 secret scan（§2A 决策 C：cross-language 修正为 Go inline sanity hit-count check）**：
  - **背景说明**：task-2.1 scanner 是 Rust 实现（`core/src/scanner/`），Go 不能直接调；且 ContextRecord.content 已是 scanner+indexer redacted 后的内容（理论上 0 secret 残留）。本 task **不重做 task-2.1 完整 detection**，仅作 defense-in-depth sanity check
  - **实现**：`internal/exporter/secretscan.go`：
    - 公开 API：`func ScanForSecrets(content []byte) []SecretHit`
    - 5-7 个常见 pattern regex（Go 直写）：API key (sk-_xxx / token-_xxx)、Bearer token (`Bearer [A-Za-z0-9+/=]{20+}`)、PEM private key block (`-----BEGIN [A-Z ]+PRIVATE KEY-----`)、AWS access key (`AKIA[0-9A-Z]{16}`)、GitHub token (`ghp_[A-Za-z0-9]{36}` / `gho_...`)、密码字符串 (`password\s*[:=]\s*[^\s]+` non-redacted heuristic)
    - 复用 task-2.1 Rust scanner 在测试 fixture 中已用的 pattern（同源知识 — `test/fixtures/scanner/with-secret.env` 等）
  - **export 前调用**：在每个 format 写出前，扫 final bytes（serialized output 全文）；命中 → 拒 export + Result.SecretHits 非空 + 错误返回；CLI 显示 hits 列表 + 退出码 1
  - **不修原 ContextRecord.content**（export 是只读）：仅做 detection 不做 redaction（已 redacted 上游）；命中即报告
- **AC4 fidelity 计算（§2A 决策 D：exporter.CalcFidelity helper + 内部 fixture）**：
  - `internal/exporter/fidelity.go`：
    - 公开 API：`func CalcFidelity(original []*contextforgev1.ContextRecord, exported []byte, format Format) (float64, error)`
    - 实现：reparse exported bytes → 反向构造 record list → 字段对比
    - 23 ContextRecord 字段（id, schema_version, collection_id, source_type, source_provider, source_uri, agent_scope, title, content, content_hash, redaction_status, language, file_path, line_start, line_end, tags, provenance, security_labels, created_at, updated_at, expires_at, version, metadata）
    - 每字段二元（PRESENT / ABSENT 或 MATCH / MISMATCH）；fidelity = Σ matched / (records × 23)
    - 部分字段允许 v0.1 schema-gap 默认值（context_id / source_type / agent_scope / redaction_status — task-4.2 §10 留档）— 这些字段如双方都为 default 算 MATCH
  - **AC4 内部 fixture 验证**：`internal/exporter/fidelity_test.go`：
    - 创 10 个 mock records → JSONL export → CalcFidelity → assert ≥ 0.8
    - 同 10 records → markdown-bundle → CalcFidelity → assert ≥ 0.8
    - 同 10 records → agent-draft → CalcFidelity → assert ≥ 0.6（agent-draft 是 lossy format，YAML frontmatter 不覆盖全 23 字段，预期 fidelity 较低，特殊阈值）
- **AC5 Phase 6 端到端 smoke（§2A 决策 E）**：
  - **本 task 填实 `docs/specs/phases/phase-6-cli-api-export.md` §6 端到端 smoke**（currently `<TBD-by-user>`）：
    ```bash
    # Phase 6 端到端 smoke（task-6.3 AC5 落点）
    contextforge init --root /tmp/cf-phase6
    contextforge import /path/to/fixture --collection default
    contextforge serve --data-dir /tmp/cf-phase6 &
    SERVE_PID=$!
    sleep 2
    contextforge search "fixture query"  # CLI 入口
    curl -H "Authorization: Bearer $(cat /tmp/cf-phase6/token)" \
      -X POST http://127.0.0.1:<port>/v1/search \
      -d '{"query":"fixture query"}'  # REST 入口与 CLI 一致
    contextforge export --collection default --format jsonl --output /tmp/cf-out.jsonl
    contextforge export --collection default --format markdown-bundle --output /tmp/cf-out.tar.gz
    contextforge export --collection default --format agent-draft --output /tmp/cf-out-draft/
    # 验 secret scan 0 hits + fidelity >= 0.8
    kill $SERVE_PID
    ```
  - `core/tests/phase6_smoke.rs`（task-6.1 已 ✅ 1 测试 + 本 task 不扩 — 已是 Rust gRPC 端到端 smoke）；本 task 的 AC5 phase smoke 是 **shell-level Go-side**（端到端 CLI/REST/export 完整链）
  - 本 task **不实现 phase smoke 命令的自动化测试**（留 task-8.1 eval-harness）；spec 中给 shell 命令骨架供主 agent §4 Gate 3 手动验证 + future task-8.1 接手
- **新增 RED→GREEN 测试**（5 个，落在以下文件）：
  - `internal/exporter/jsonl_test.go` — TEST-6.3.1 AC1 jsonl 格式 + record count
  - `internal/exporter/markdown_test.go` — TEST-6.3.1 (extends) md-bundle .tar.gz 结构验证 + manifest.json
  - `internal/exporter/agentdraft_test.go` — TEST-6.3.2 AC2 agent-draft 4 .md 生成 + 保护路径拒绝
  - `internal/exporter/secretscan_test.go` — TEST-6.3.3 AC3 sanity scan 命中 / 0 hits 行为
  - `internal/exporter/fidelity_test.go` — TEST-6.3.4 AC4 三格式各自 fidelity ≥ 0.8 / 0.6
  - `internal/cli/export_test.go` — TEST-6.3.5 AC5 CLI export 子命令 end-to-end + 三 format flag
- **填实 `test/features/exporter.feature` SCEN-6.3.1~5** 占位 Given/When/Then
- **填实 phase-6 spec §6 端到端 smoke**（移走 `<TBD-by-user>`；§2A 决策 E）

### Out Of Scope

- **导出 search result subset**（`--query` flag 过滤）：v0.1 P0 仅 collection-wide；future enhancement
- **完整 task-2.1 scanner Go mirror**（§2A 决策 C 已澄清）：本 task 仅 sanity hit-count check，不重做完整 detection
- **import / write-back 功能**：导出 read-only；不写回第三方 Agent（PRD ADR-005 + AC2 硬约束）
- **跨 collection 导出**（多 collection union）：v0.1 单 collection；future
- **加密 export bundle**：v0.1 plain output（用户负责 transport encryption）
- **真实大规模 fidelity 测试（10000+ records）**：本 task 仅内部 fixture（10 records）；大规模留 task-8.1 eval-harness
- **持续 export（incremental / delta export）**：v0.1 全量 export；增量留 future
- **修改 task-2.1 scanner / task-5.2 lifecycle / task-4.x retriever 公开 API**：仅消费现有公开 API；不扩
- **修改 `Cargo.toml` / `go.mod` / `Cargo.lock` / `go.sum`**：R7 严格通道（stdlib archive/tar + compress/gzip + regexp + encoding/json + encoding/yaml? — yaml 见下）
  - **YAML for markdown frontmatter**：Go stdlib 无 yaml；本 task 选**手写简单 YAML emit**（subset：scalar + repeated string lists；不引入 gopkg.in/yaml.v3）— 23 字段都是 scalar / string list，手写 emit ~40 行 stdlib 代码足够；解析端（fidelity reparse）也手写 minimal YAML parse。**坚决不引 yaml dep**（R7 严格通道）
- **修改 `proto/contextforge/v1/*.proto`**：proto frozen + phase23-start-gate

## 4. Users / Actors

- **PRD §Core Capabilities #5 跨 Agent 上下文迁移消费者**：通过 `contextforge export` 把 ContextForge 治理后的上下文导出为 JSONL / Markdown / Agent draft 供其他 Agent 接入
- **task-6.1 cli-search / daemon.Search**（上游 ✅ done）：本 task **通过 daemon.Search(query="*") pseudo full-scan** 拉 collection 全集（§2A 决策 A 避免引 SQLite driver R7 chore-dep）；复用 task-6.1 落地的 proto-generated `ContextRecord` Go 消费模式
- **task-6.2 rest-api**（同期并行）：与本 task 共享 daemon.Search 入口但不与其 REST handler 写路径冲突（task-6.2 改 `internal/daemon/rest.go` + cli.go dispatch case `"serve"`；本 task 改 `internal/exporter/` + cli.go dispatch case `"export"`；仅 cli.go 同函数体不同 case 分支会触发后 merge rebase — 见 §8 Risks）
- **task-2.4 indexer**（上游 ✅ done）：本 task **通过 daemon.Search 间接消费** indexer SQLite（§2A 决策 A 避免引 Go SQLite driver）；不直读 indexer 文件层 — 由 daemon → core gRPC → retriever → Tantivy/SQLite 走标准 read path
- **task-5.2 lifecycle**（上游 ✅ done）：本 task 默认调 `lifecycle.Mark + FilterStale` 过滤 stale records（`--include-stale` 显式 bypass）
- **task-2.1 scanner**（上游 ✅ done，cross-language 限制）：本 task **不直调** scanner（Rust）— §2A 决策 C 用 Go inline sanity scan；但 secret pattern 知识同源（部分参考 task-2.1 fixture 已用的 pattern）
- **task-1.4 cli-init**（上游 ✅ done）：本 task `contextforge export` 子命令复用 CLI dispatch 框架
- **task-7.1 MCP tool**（下游软依赖）：MCP 可能调 export 输出做迁移；本 task 不直接 wire MCP
- **task-8.1 eval-harness**（下游强依赖 — fidelity 大规模回归 / phase smoke 自动化）：本 task 提供 `CalcFidelity` helper + phase smoke 命令骨架，task-8.1 接手大规模运行
- **PRD §Success Metrics 次指标「跨 Agent 迁移保真 ≥ 80% 结构化字段」消费者**：本 task `CalcFidelity` 内部 fixture 是该指标的 v0.1 落地

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Core Capabilities #5 / §Decisions Log D5 / §Constraints 兼容性导出格式 / §Success Metrics 跨 Agent 迁移保真 / §Technical Risks R4）
- `docs/specs/phases/phase-6-cli-api-export.md`
- `docs/specs/tasks/task-6.1-cli-search.md`（同期 ✅ done）
- `docs/specs/tasks/task-6.2-rest-api.md`（同期并行 — claude-work1 跑；你不依赖其代码，仅 spec 知道 phase-6 同期场景）
- `docs/specs/tasks/task-2.1-scanner.md`（cross-language 知识源：secret patterns）
- `docs/specs/tasks/task-2.4-indexer.md`（上游：SQLite schema）
- `docs/specs/tasks/task-5.2-lifecycle.md`（上游：Mark + FilterStale）
- `docs/specs/tasks/task-1.4-cli-init.md`（上游：CLI dispatch）
- `docs/decisions/adr-005-readonly-import-draft-export.md`
- `test/features/exporter.feature`

### 5.2 Imports

- **Go stdlib**:
  - `os` / `path/filepath` / `io` / `io/fs`（文件 I/O + permission）
  - `encoding/json`（JSONL marshaling — sequence of records as one JSON per line）
  - `archive/tar` + `compress/gzip`（markdown-bundle .tar.gz packaging — §2A 决策 B）
  - `regexp`（secret scan patterns — §2A 决策 C）
  - `strings` / `bytes` / `fmt` / `errors`
  - `crypto/sha256`（content_hash 重算用于 fidelity check — 与 task-2.3 chunker 一致 sha256）
  - `context`
- **Go 内部（已有）**:
  - `github.com/tajiaoyezi/contextforge/internal/config`（DataDir / DefaultRootDir）
  - `github.com/tajiaoyezi/contextforge/internal/memoryops/lifecycle`（Mark + FilterStale — task-5.2 ✅）
- **Go proto（已有，task-1.1 codegen）**:
  - `contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"`（ContextRecord / Provenance / Chunk — 全 23 字段）
- **Go SQLite**: **不引入新 module**！本 task 直读 SQLite 需要 driver。**关键约束**：Go 端目前**没有** SQLite driver dep（`go.mod` 现状），如要直读 indexer SQLite 需要 R7 chore-dep PR 引 `modernc.org/sqlite` (CGO-free) 或 `mattn/go-sqlite3` (CGO)。**v0.1 P0 决策**：避免 R7 chore-dep PR，改走 **替代路径**：
  - **路径 A（v0.1 选定 — §2A 决策 2026-05-23）**：通过 `daemon.Start` + 多次 `daemon.Search(query="*", top_k=large)` pseudo full-scan — 不需 SQLite driver，但 BM25 `*` 查询语义不一定全集（取决于 Tantivy `*` 是否 wildcard 全命中）
  - **路径 B（fallback）**：扩 Rust core 加新 gRPC RPC `ListAllChunks(collection_id)` — 走 SPEC-DRIFT + phase23-gate add-only；主 agent 串行处理（见 §8 SPEC-DRIFT-task-6.3.list-chunks 名牌）
  - **路径 C（future）**：扩 Rust core 加 CLI 子命令 `contextforge-core dump --collection <id>` 输出 JSONL → Go 端读 stdin — 简单但需要 binary spawn，v0.1 不引入
  - **选定 A（§2A 决策 2026-05-23）**：如 RED 阶段发现 BM25 `*` 不全集 → 写 `SPEC-DRIFT-task-6.3.list-chunks.md` 报主 agent 串行升级到路径 B
- **R7 严格通道**：不引入新 Go module / Rust crate；本 task 选 SQLite **替代路径 A**（daemon.Search 全集 pseudo-scan）避免引 SQLite driver；YAML 也手写 minimal emit/parse 避免引 yaml dep

### 5.3 函数签名

**Go CLI** (`internal/cli/export.go` 新建；`cli.go` 内 dispatch case `"export"` 调 `runExport`):

```go
package cli

import (
    "context"
    "flag"
    "fmt"
    "io"
    "os"
    "time"

    "github.com/tajiaoyezi/contextforge/internal/config"
    "github.com/tajiaoyezi/contextforge/internal/exporter"
)

// exportOpts — flag 解析后状态。
type exportOpts struct {
    Format       string  // --format jsonl|markdown-bundle|agent-draft
    Collection   string  // --collection (default "default")
    DataDir      string  // --data-dir (default config.DefaultRootDir())
    Output       string  // --output (file path or dir per format)
    IncludeStale bool    // --include-stale (default false)
}

// runExport 实现 export 子命令（AC1-5 + AC5 phase smoke 落点）。
// 返回 process exit code (0=ok / 1=运行错 / 2=usage 错 / 3=secret hit 拒)
func runExport(args []string, stdout, stderr io.Writer) int
```

**Go exporter package** (`internal/exporter/*.go` 新建):

```go
// Package exporter 实现 contextforge export 三格式 + 二次 secret scan + fidelity.
// Contract: task-6.3 §5.3.

package exporter

import (
    "context"
    "io"

    contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// Format — 三格式枚举（§2A 决策 B）.
type Format string

const (
    FormatJSONL          Format = "jsonl"
    FormatMarkdownBundle Format = "markdown-bundle"
    FormatAgentDraft     Format = "agent-draft"
)

// Options — Export 公开 API 入参。
type Options struct {
    Format       Format
    Collection   string
    DataDir      string
    Output       string // file path (jsonl / md-bundle) or dir (agent-draft)
    IncludeStale bool
}

// Result — Export 出参。
type Result struct {
    RecordsExported int           // 写出 record 数（stale 过滤后）
    OutputPath      string        // 最终写入路径
    FidelityScore   float64       // 自检 fidelity（仅 jsonl 路径自动 calc；其他 0.0 留 task-8.1 大规模）
    SecretHits      []SecretHit   // export 前 sanity scan 命中（如有 → 拒 export + 返 error）
}

// SecretHit — sanity scan 命中条目（§2A 决策 C）.
type SecretHit struct {
    PatternName string // e.g. "aws_access_key" / "bearer_token"
    Match       string // 命中片段（首 20 chars truncated for safety）
    Offset      int    // 在 final bytes 内偏移
}

// Export — 主入口（AC1 + AC2 + AC3 + AC4 + AC5 落点）.
//
// 流程:
//   1. loadRecords(ctx, opts.DataDir, opts.Collection)
//   2. 默认 stale 过滤 (除非 opts.IncludeStale): lifecycle.Mark + FilterStale
//   3. format-specific writer 写到 buffer
//   4. ScanForSecrets(buffer) — 命中 → 拒 + Result.SecretHits + error
//   5. 写 opts.Output (file mode 0600 / dir mode 0700)
//   6. (jsonl only) CalcFidelity(records, exported, format) → Result.FidelityScore
func Export(ctx context.Context, opts Options) (*Result, error)

// ScanForSecrets — Go inline sanity hit-count check（§2A 决策 C）.
// 5-7 个常见 pattern: aws_access_key / bearer_token / pem_private_key / github_token /
// generic_api_key / password_literal。
// 不重做 task-2.1 完整 detection (cross-language 修正)，作为 defense-in-depth.
func ScanForSecrets(content []byte) []SecretHit

// CalcFidelity — 23 字段对比（§2A 决策 D）.
// reparse exported bytes 反向构造 records → 字段二元对比 (MATCH / MISMATCH) → fidelity.
// agent-draft 是 lossy format，预期较低 (~0.6 baseline)；jsonl/md-bundle 应 ≥ 0.8.
func CalcFidelity(original []*contextforgev1.ContextRecord, exported []byte, format Format) (float64, error)

// 内部 helpers（每个文件 1 个 writer）:

// loadRecords — collection-wide 读全集（§2A 决策 A）.
// 优先路径 A: daemon.Search(query="*", top_k=large) pseudo full-scan；
// 若发现 BM25 "*" 不全集 → SPEC-DRIFT-task-6.3.list-chunks 报主 agent.
func loadRecords(ctx context.Context, dataDir, collection string) ([]*contextforgev1.ContextRecord, error)

// writeJSONL — 一行一个 ContextRecord encoding/json marshal.
func writeJSONL(records []*contextforgev1.ContextRecord, w io.Writer) error

// writeMarkdownBundle — 多 .md + manifest.json 打 .tar.gz.
func writeMarkdownBundle(records []*contextforgev1.ContextRecord, w io.Writer) error

// writeAgentDraft — 生成 4 .md (MEMORY / USER / AGENTS / CLAUDE) 到 dir.
// 启动 check Output 路径是否在保护列表 (~/.cursor/ / ~/.claude/ / etc) → 拒.
func writeAgentDraft(records []*contextforgev1.ContextRecord, dir string) error
```

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [x] **AC1** (PRD §Implementation Phases Phase 6 Exit Criteria): `contextforge export --format jsonl` 导出 canonical JSONL；`--format markdown-bundle` 导出 Markdown bundle（多 .md + manifest 打 .tar.gz）。
- [x] **AC2** (PRD §Constraints 兼容性导出格式): 支持 `--format agent-draft`（Hermes-style MEMORY.md/USER.md/AGENTS.md/CLAUDE.md draft），draft/bundle 一律不写回第三方 Agent（output 在保护路径下 → 拒）。
- [x] **AC3** (PRD §Technical Risks R4 / §Constraints 安全): export 前执行二次 secret scan（**v0.1 解读 §2A 决策 C**：Go inline sanity hit-count check，5-7 常见 pattern；cross-language 限制下不重做 task-2.1 完整 detection；defense-in-depth），命中即拒 export。
- [x] **AC4** (PRD §Success Metrics 跨 Agent 迁移保真): 迁移后结构化字段保真率经 fixture 计算，jsonl/md-bundle ≥ 80% / agent-draft ≥ 60%（agent-draft 是 lossy format 特殊阈值，§2A 决策 D）。
- [x] **AC5** (本 task 新增): Phase 6 端到端 smoke 可执行（init → import → serve → CLI search + REST /v1/search 一致 → export 三格式 + sanity secret scan 0 hits + fidelity ≥ 80% / 60%），本 task 填实 phase-6 spec §6 端到端 smoke 命令骨架（自动化运行留 task-8.1 eval-harness）。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 jsonl/md-bundle 导出 | SCEN-6.3.1 | TEST-6.3.1 | - | unit-test | Done |
| AC2 agent-draft + 不写回 | SCEN-6.3.2 | TEST-6.3.2 | - | unit-test | Done |
| AC3 sanity secret scan | SCEN-6.3.3 | TEST-6.3.3 | - | unit-test | Done |
| AC4 fidelity 三格式 | SCEN-6.3.4 | TEST-6.3.4 | - | unit-test | Done |
| AC5 Phase6 端到端 smoke 骨架 | SCEN-6.3.5 | TEST-6.3.5 | phase-6 spec §6 | unit-test | Done |

## 8. Risks

- 关联 PRD §Technical Risks **R4**（export 二次扫描漏检）：§2A 决策 C cross-language 修正 — 本 task 的 sanity check 是 5-7 pattern 子集，**不替代** task-2.1 完整 detection；理论上 ContextRecord.content 已 redacted（上游 scanner+indexer 保证），sanity 命中即上游漏 redact 的边缘情况 → 拒 export + 报告促修。**剩余漏检风险**：sanity pattern 未覆盖的新 secret 类型（如 OpenAI key 新 format）→ task-8.1 eval-harness 跑大规模 corpus 暴露 + 主 agent 增 pattern。
- 关联 PRD §Technical Risks **R5**（agent-draft 格式随上游漂移）：Hermes / Cursor / Claude / OpenClaw 各 Agent 配置 schema 实际变更可能让 v0.1 agent-draft 不再被它们识别。**缓解**：v0.1 agent-draft 仅作为 ContextForge 自治模板（YAML frontmatter ContextRecord 字段完整）；用户人工 cp 到 Agent dir 时根据当时 Agent 实际 schema 适配 — **draft/bundle 不写回**（AC2 硬约束已限制）。task-7.1 MCP wrap 或 future SPEC-DRIFT 跟进具体 Agent schema。
- 关联 PRD §Open Questions **O5**（schema 无损承载边界）：23 字段全集 fidelity 假设各 Agent target schema 容纳；agent-draft 是 lossy 已专门阈值化（≥0.6 vs ≥0.8）。
- **§2A 决策 A SQLite 替代路径风险**：v0.1 选定路径 A（daemon.Search(query="*", top_k=large) pseudo full-scan）；若 RED 阶段发现 BM25 `*` 不全集（Tantivy 标准 query parser 不一定接受 wildcard 单 token）→ 写 **`SPEC-DRIFT-task-6.3.list-chunks.md`**（名牌）让主 agent 串行加 gRPC RPC `ListAllChunks(collection_id)` 走 add-only proto 扩展（路径 B）。
- **`internal/cli/cli.go` dispatch case 并行修改 rebase 风险**：本 task 改 dispatch case `"export"`（task-1.4 默认 not-implemented）；同期 task-6.2 改 case `"serve"`（同函数体不同 case 分支）。后 merge 一侧需 rebase 解决（trivial）— §4 Gate 1 必须验证。两 task worker 之间不直接协调，主 agent 在 §4 Gate 1 切到第二个 PR 时执行 rebase。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。⚠️ 本 task 与 6.2 为 Phase 6 末批：Phase 6 最后合并的 task 完工前 phase spec §6 端到端 smoke 必须填实（C1 / team §4 Gate 3）。本 task §3 In Scope 已含填实 phase-6 spec §6 — 当本 task 是 phase-last 时主 agent §4 Gate 3 触发该 smoke。

## 10. Completion Notes

- **完成日期**：2026-05-23
- **改动文件**：
  - `internal/exporter/`（新增）— Export 主入口、daemon.Search pseudo full-scan backend、JSONL / Markdown bundle / agent-draft writers、sanity secret scan、CalcFidelity
  - `internal/cli/export.go`（新增）— `contextforge export` flag parsing + exit-code handling
  - `internal/cli/cli.go`（修改）— dispatch case `"export"` 改为 `runExport`
  - `cmd/contextforge/main.go`（修改）— production exporter backend wire 到 `daemon.Search`，通过 `CONTEXTFORGE_DATA_DIR` 传 `--data-dir`
  - `internal/cli/cli_test.go` / `internal/cli/export_test.go`（修改/新增）— task-6.3 export CLI tests + task-1.4 placeholder test 更新
  - `internal/exporter/*_test.go`（新增）— TEST-6.3.1~5 RED→GREEN 覆盖
  - `test/features/exporter.feature`（修改）— SCEN-6.3.1~5 Given/When/Then 填实
  - `docs/specs/phases/phase-6-cli-api-export.md`（修改）— §6 端到端 smoke shell 命令骨架填实
  - `docs/s2v-adapter.md`（修改）— task-6.3 Status Ready → Done
  - `docs/specs/tasks/task-6.3-exporter.md`（修改）— Status / AC / §7 / §10 终态回填
- **commit 列表**：
  - `0042671` test(exporter): 加 SCEN-6.3.1~5 共 5 个 RED 测试 + Status: Ready → In Progress
  - `200190b` feat(exporter): contextforge export 端到端实现 (三格式 + sanity secret scan + CalcFidelity 通过全部 5 个测试) + phase-6 §6 端到端 smoke 命令骨架填实
  - `25eeeb0` test(exporter): 避免 ContextRecord fixture 值拷贝触发 go vet
  - 本 docs(spec) commit（§10 回填 + Status → Done）
- **§9 Verification 结果**：
  - install: ✅ `go mod download && cargo fetch`
  - typecheck: ✅ `go vet ./... && cargo check --workspace`
  - unit-test: ✅ `go test ./... && cargo test --workspace`
    - Go: 12 tested packages ok + 2 no-test packages；新 `internal/exporter` 包 5 tests passed
    - Rust: 55 tests passed / 0 failed（core unit 31 + skeleton 4 + phase smokes 4 + proto contract 5 + scanner 11）
- **剩余风险 / 未做项**：Phase 6 §6 shell smoke 已填实但自动化执行留 task-8.1；export 数据源按 §2A 路径 A 走 `daemon.Search(query="*")` pseudo full-scan，若后续实测发现 Tantivy `*` 不能全集召回，按既定 `SPEC-DRIFT-task-6.3.list-chunks.md` 升级到 `ListAllChunks`。
- **下游 task 影响**：task-6.2（phase-last merge 时 Gate 3 复用 phase-6 §6 smoke）；task-7.1（MCP 可复用 `contextforge export` / exporter API）；task-8.1（接手 phase smoke 自动化 + 大规模 fidelity 回归）。
- **§2A Decisions**（2026-05-23 用户审定，主 agent 与用户预先审定后落 spec；worker 完工时按实际实施情况验证 / 补充）：
  - **A: 数据源 = collection-wide 读全集 + stale 默认过滤**：`contextforge export --collection <id>` 直读 indexer 全集；默认调 `lifecycle.Mark + FilterStale` 跳过 stale records，`--include-stale` 显式 bypass。SQLite 读取走 daemon.Search(query="*", top_k=large) pseudo full-scan 替代路径（避免引 SQLite Go driver R7 chore-dep）；BM25 `*` 不全集 → SPEC-DRIFT-task-6.3.list-chunks 串行
  - **B: 三格式 = jsonl + md-bundle(.tar.gz multi-file) + agent-draft (4 .md)**：jsonl 一行一 record；md-bundle 多 .md + manifest.json 打 .tar.gz；agent-draft 生 MEMORY/USER/AGENTS/CLAUDE.md 到 output dir + 启动 check Output 在保护路径拒
  - **C: AC3 secret scan = Go inline sanity hit-count check（cross-language 修正）**：task-2.1 scanner 是 Rust + Go 不能直接调；ContextRecord.content 已上游 redacted；本 task `internal/exporter/secretscan.go` 写 5-7 常见 pattern regex（aws_access_key / bearer_token / pem_private_key / github_token / generic_api_key / password_literal），sanity hit-count check，**不重做** task-2.1 完整 detection
  - **D: AC4 fidelity = exporter.CalcFidelity + 内部 fixture**：23 字段二元对比；jsonl/md-bundle 阈值 ≥ 0.8；agent-draft lossy format 特殊阈值 ≥ 0.6；10 mock records 内部 fixture 跑 unit test；大规模回归留 task-8.1 eval-harness
  - **E: AC5 Phase 6 端到端 smoke 由本 task 填实 phase-6 spec §6**：本 task 在 phase-6 spec §6 落 shell 命令骨架（init/import/serve/search/curl /v1/search/export ×3/secret scan/fidelity）；自动化运行留 task-8.1 eval-harness；core/tests/phase6_smoke.rs 已由 task-6.1 ✅ 占 Rust gRPC 端到端 smoke 落点（本 task 不重做 Rust 端 smoke）
  - **R7 严格通道**：未引入新 Go module / Rust crate；YAML 也手写 minimal emit/parse 避免引 yaml dep；SQLite 走 daemon.Search 替代路径避免 SQLite driver；stdlib archive/tar + compress/gzip + regexp + encoding/json + crypto/sha256 + context 全用 stdlib
