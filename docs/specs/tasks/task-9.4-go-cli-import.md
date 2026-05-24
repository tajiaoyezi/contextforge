# Task `9.4`: `go-cli-import — contextforge import hermes/openclaw/agent-rules 三子命令实现`

> Status=Done；主 agent §2A 自审 + §6 AC 5/5 + §9 verify 全绿（ADR-012 + goal §自决规则 6）。

**Status**: Done

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 9 (cli-pipeline)
**Dependencies**: 9.2 (rust-grpc-index — 间接，因 import 产出物最终用 index 灌入；本 task 自身不调 gRPC)

## 1. Background

v0.1 实测：`internal/cli/cli.go` `import` 子命令直接返回 `not implemented (Phase 2+/6/7/8; task-1.4 registers the skeleton only)`，尽管 `internal/importer/{hermes,openclaw,agentrules,fallback}` Go 包已在 Phase 3 实现并 Done — 仅缺 CLI 入口包装。详见 [ADR-013](../../decisions/adr-013-cli-data-plane-grpc-bridge.md) §Context #1。

ADR-013 §Decision 决定走 D1 两步式：`import` 离线产出 canonical record → 用户跑 `index` 灌入。本 task 实施 import 子命令，把已实现的 `internal/importer/<src>` 包包装为可命令行调用的入口。

**实施层 D1 细化**：import 不直接产 JSONL（避免扩 indexer 支持新输入格式），而是**把每个 ContextRecord 写成单独的 .md 文件**到 `<data_dir>/imports/<source>/<ctx_id>.md`（含 YAML frontmatter 标准 metadata + body=record.content）。用户跑 `contextforge index --source <data_dir>/imports/<source>/` 让现有 `IndexSession::index_path` 把目录当普通 source 扫描。这种"normalize 为 markdown 目录"方案：
- 优点：复用现有 indexer / scanner / parser / chunker 链；不扩 proto；导出的 .md 文件可被用户人工查看 / 编辑 / git track
- 缺点：失去 importer 的 chunk 边界（重新 chunker 分块）；产生 N 个小文件
- v0.2 PoC 接受此 trade-off；如未来需保 importer chunk 边界，走 task-10.X 引入 proto FEED_RECORDS mode（task-9.1 §3 Out Of Scope 留 escape hatch）

## 2. Goal

`contextforge import hermes <path> --collection <id> [--data-dir <root>]` / `contextforge import openclaw <path> ...` / `contextforge import agent-rules <path> ...` 三子命令实现：解析 `<path>` 通过 `internal/importer/<src>.Importer` 接口产出 `[]*ContextRecord` → 把每条 record normalize 为 `.md` 文件 + YAML frontmatter 写到 `<data_dir>/imports/<source>/<ctx_id>.md` → CLI stdout 打印 summary（imported N records to <output_dir>，提示 next step `contextforge index --source <output_dir> --collection <id>`）；幂等可重跑（同源同内容 → 同 ctx_id → 覆盖写）；Go unit + integration test 覆盖三种 importer + fallback。

## 3. Scope

### In Scope

- **新增 `internal/cli/import.go`**：
  - 入口：`Execute` 现 dispatch case `"import"` 不再走 `not implemented`，改为调本 task 新增 `runImport(args []string, stdout, stderr io.Writer) int`
  - sub-dispatch：第一个 positional arg 必须是 importer 名（`hermes` / `openclaw` / `agent-rules`），不识别 → stderr usage + exit 2
  - flags（per sub-importer 共享 flagset）：
    - 第二 positional arg = source path（路径文件 / 目录，importer 自己 detect）
    - `--collection <id>` (required if no default)
    - `--data-dir <root>` (default `config.DefaultRootDir`)
    - `--output <dir>` (optional override；default `<data_dir>/imports/<source-name>/`)
    - `--dry-run` (bool；不写文件，只打 summary)
  - 流程：
    1. 解析 args → importer name + source path + flags
    2. 校验 source path 存在；不存在 → stderr + exit 1
    3. 选 `imp = importer.Get<Source>Importer()`（按 importer name 分发；详见 5.3）— 不用 importer.Resolve(path)（Resolve 是 detect-based，本 task 命令式分发更明确）
    4. `records, err := imp.Import(path, collectionID)` → err → stderr + exit 1
    5. for each record → `recordToMarkdown(record)` → 写文件 `<output_dir>/<record.Id>.md`（mkdir -p output_dir if missing）
    6. `--dry-run` skip 写文件，只输 summary
    7. stdout 打 `imported %d records to %s` + `next: contextforge index --source %s --collection %s`
    8. exit 0
- **新增 `internal/cli/import.go` 内 `recordToMarkdown` helper**：
  ```go
  // recordToMarkdown serialises a ContextRecord as a Markdown file with YAML
  // frontmatter. The frontmatter preserves importer / source_provider /
  // source_type / agent_scope / language / file_path / line_start / line_end /
  // content_hash / created_at metadata; the body is record.content. This format
  // is round-trippable for human inspection and consumable by IndexSession via
  // its existing scanner→parser→chunker→indexer pipeline (D1 two-step flow).
  func recordToMarkdown(rec *contextforgev1.ContextRecord) (string, error)
  ```
  - Frontmatter schema：
    ```yaml
    ---
    schema_version: "0.1"
    id: ctx_abc123def
    collection_id: silijian
    source_type: memory
    source_provider: hermes
    source_uri: file:///path/to/original
    agent_scope: [hermes]
    language: markdown
    file_path: /path/to/original
    line_start: 1
    line_end: 42
    content_hash: sha256:abcd...
    importer: hermes-memory
    created_at: "2026-05-24T12:00:00Z"
    ---
    
    <record.content here>
    ```
  - 注意：YAML frontmatter 是给人类读 + git track，**不被 indexer 解析为 metadata** — indexer 当成 markdown 内容看；schema 同步 task-5.x memoryops 未来回填 provenance 时可能用到
- **新增 importer name 分发 helper**：
  - `internal/cli/import.go` 内 `selectImporter(name string) (importer.Importer, error)`：
    - "hermes" → `hermes.NewHermesImporter()` 等价；具体 constructor 名按 internal/importer/hermes/ 实际暴露
    - "openclaw" → `openclaw.NewOpenclawImporter()` 等价
    - "agent-rules" → `agentrules.NewAgentRulesImporter()` 等价
    - other → error `"unknown importer: %s; want one of [hermes, openclaw, agent-rules]"`
  - **task §2A Ready 前主 agent 必须 verify** internal/importer/hermes / openclaw / agentrules 三包实际 constructor / Register API 名；如不存在显式 New<X> 函数需复用 `importer.Register` + 全局 registry → 详细签名按实际包 export 修正本 §5.3
- **修改 `internal/cli/cli.go`**：
  - dispatch case `"import"` 从 `fmt.Fprintln(stderr, "contextforge import: not implemented...")` 改为 `return runImport(args[1:], stdout, stderr)`
  - 同 task-6.1 添加 case `"search"` 的 wire pattern
- **新增 `internal/cli/import_test.go`**：
  - TEST-9.4.1: `runImport hermes <fixture>` 写出预期数量 .md 文件 + frontmatter 正确（unit）
  - TEST-9.4.2: `runImport openclaw <fixture>` 同上
  - TEST-9.4.3: `runImport agent-rules <fixture>` 同上
  - TEST-9.4.4: `runImport <unknown-importer>` → exit 2 + stderr 含 usage
  - TEST-9.4.5: `--dry-run` 不写文件但 stdout 打 summary（包含 next-step 提示）
- **新增 fixture**：`test/fixtures/import-cli/hermes/MEMORY.md` + `.../USER.md`（覆盖 Hermes importer 期望的输入）；同理 openclaw / agent-rules 各一个 minimal fixture
- 文件锚点：`internal/cli/import.go`（新增）+ `internal/cli/import_test.go`（新增）+ `internal/cli/cli.go`（修改 1 行 dispatch）+ `test/fixtures/import-cli/<source>/...`（新增）

### Out Of Scope

- **import 调 gRPC FEED_RECORDS 灌入 (D1 选项 B)**：v0.2 走两步式；如未来需要单步式，走 future task
- **canonical JSONL 导出**（v0.2 把 record 序列化为 .md 不是 JSONL）：如需 JSONL，task-6.3 exporter 已实现 `contextforge export --format jsonl`，与本 task `import` 用途不同
- **修改 `internal/importer/`**（本 task 不动 importer 实现，仅消费）：如 importer 自身有 bug 需独立 chore PR
- **修改 `core/src/` 或 proto**（本 task 纯 Go 离线）
- **修改 `Cargo.toml` / `go.mod`**（R7）；YAML frontmatter 用 stdlib 字符串拼接（不引 yaml lib — 简单场景不值得 R7 chore）
- **schema 进化 / migration**（v0.2 schema 即 frozen for v0.2；v0.3 时另起）

## 4. Users / Actors

- **README quick start 用户**（PRD §User Flow 主流程步 2 - import）：本 task 后 `contextforge import hermes <path>` 真实工作
- **task-9.3 go-cli-index**（下游消费）：用户跑 `contextforge import` 后产生的 `<output_dir>` 作为 task-9.3 `--source` 输入；本 task 必须保证 output 目录布局满足 task-9.3 `--source` 期望（directory of .md files）
- **task-9.5 release-smoke-real 实施 agent**（下游）：复用本 task CLI 作为 release smoke "import" 步
- **PRD §User Flow 异常流"导入源 schema 不识别"用户**（间接）：`internal/importer/fallback.go` 已实现降级为通用 file processing；本 task `selectImporter` 未来可加 `auto` mode 用 `importer.Resolve(path)` detect-based 分发；v0.2 命令式分发优先

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§User Flow 主流程步 2 + 异常流"导入源 schema 不识别" / §Core Capabilities #5 跨 Agent 上下文迁移 / §Implementation Phases Phase 3 Exit Criteria）
- `docs/specs/phases/phase-9-cli-pipeline.md`
- `docs/specs/phases/phase-3-agent-importers.md`
- `docs/specs/tasks/task-3.1-importer-core.md`
- `docs/specs/tasks/task-3.2-importer-hermes.md`
- `docs/specs/tasks/task-3.3-importer-openclaw.md`
- `docs/specs/tasks/task-3.4-importer-agent-rules.md`
- `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`
- `internal/importer/importer.go`（Importer interface）
- `internal/importer/record.go`（ContextRecord build helpers）
- `internal/cli/cli.go`（dispatch pattern）
- `internal/cli/search.go`（CLI flag parsing pattern 参考）
- `test/features/cli-pipeline.feature`

### 5.2 Imports

- **stdlib**：`flag` / `fmt` / `io` / `os` / `path/filepath` / `strings` / `time`
- **proto**：`contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"`（ContextRecord 类型）
- **内部**：
  - `internal/config`（DefaultRootDir）
  - `internal/importer`（Importer interface）
  - `internal/importer/hermes`（NewHermesImporter or registered importer — task §2A 时 verify 实际 export）
  - `internal/importer/openclaw`（同上）
  - `internal/importer/agentrules`（同上）
- **测试侧**：`testing` / `os` / `path/filepath` / `strings` / `bytes`
- **不引入**：R7 严格；YAML frontmatter 不引 yaml lib（stdlib 字符串拼接，简单到不值得加 dep）

### 5.3 函数签名

```go
// internal/cli/import.go ----
package cli

// runImport implements `contextforge import <source> <path> [flags]`.
//   - first positional: importer name (hermes|openclaw|agent-rules)
//   - second positional: source path
//   - flags: --collection (required), --data-dir, --output, --dry-run
// Returns process exit code: 0 success / 1 import error / 2 bad args.
func runImport(args []string, stdout, stderr io.Writer) int

// selectImporter returns the concrete Importer for the given name. Unknown
// name returns nil + error with usage hint. The task §2A reviewer must
// verify each importer package's actual constructor names before merging
// (internal/importer/hermes / openclaw / agentrules).
func selectImporter(name string) (importer.Importer, error)

// recordToMarkdown serialises a ContextRecord as a Markdown file with a YAML
// frontmatter prefix. Used by runImport to produce <output_dir>/<id>.md files
// that the existing IndexSession scanner→parser→chunker→indexer pipeline can
// consume via `contextforge index --source <output_dir>` (D1 two-step flow).
func recordToMarkdown(rec *contextforgev1.ContextRecord) (string, error)

// writeRecordsAsMarkdown writes each record to <outputDir>/<record.Id>.md.
// Returns the count of files written and any error encountered. Mkdir -p
// outputDir if missing. Existing files are overwritten (idempotent on same input).
func writeRecordsAsMarkdown(outputDir string, records []*contextforgev1.ContextRecord) (int, error)
```

- SCEN/TEST-9.4.1 → `runImport hermes test/fixtures/import-cli/hermes/` 在临时 outputDir 写出 ≥1 .md 文件含 frontmatter `source_provider: hermes` + body 含 fixture 内容（AC1）
- SCEN/TEST-9.4.2 → `runImport openclaw ...` 同上 + `source_provider: openclaw`（AC2）
- SCEN/TEST-9.4.3 → `runImport agent-rules ...` 同上 + `source_provider: ` 按 importer 实际产出 → `source_type: agent_rule`（AC3）
- SCEN/TEST-9.4.4 → `runImport unknown-source ...` → exit 2 + stderr 含 `unknown importer: unknown-source; want one of [hermes, openclaw, agent-rules]`（AC4）
- SCEN/TEST-9.4.5 → `runImport hermes ... --dry-run` 不写文件（outputDir 不存在 / 存在但无新文件）+ stdout 含 `next: contextforge index --source ... --collection ...`（AC5）

## 6. Acceptance Criteria

- [x] **AC1** (PRD §User Flow 主流程步 2 / §Implementation Phases Phase 3 Exit Criteria): `contextforge import hermes <path> --collection X --data-dir Y` 把 `internal/importer/hermes.Importer.Import` 产出的 ContextRecord 写为 `<data_dir>/imports/hermes/<ctx_id>.md` 文件（含 YAML frontmatter + body）；CLI exit 0 + stdout 含 `imported N records to <output_dir>` + `next: contextforge index --source <output_dir> --collection X`
- [x] **AC2** (同 AC1 / openclaw): `contextforge import openclaw <path>` 行为对称（产 openclaw provider 的 .md 文件）
- [x] **AC3** (同 AC1 / agent-rules): `contextforge import agent-rules <path>` 行为对称（产 agent_rule source_type 的 .md 文件）
- [x] **AC4** (本 task 新增 / CLI usability): 未知 importer 名 → exit 2 + stderr 含明确 usage + 列出三个合法 importer 名；位置 args 不足 → 同 exit 2 + usage
- [x] **AC5** (本 task 新增 / `--dry-run`): `--dry-run` flag 不写文件 + stdout 仍打 summary 含 next-step 提示 + `(--dry-run: no files were written)` 标识

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 hermes import | SCEN-9.4.1 | TEST-9.4.1 | - | unit-test | - |
| AC2 openclaw import | SCEN-9.4.2 | TEST-9.4.2 | - | unit-test | - |
| AC3 agent-rules import | SCEN-9.4.3 | TEST-9.4.3 | - | unit-test | - |
| AC4 unknown importer error | SCEN-9.4.4 | TEST-9.4.4 | - | unit-test | - |
| AC5 --dry-run | SCEN-9.4.5 | TEST-9.4.5 | - | unit-test | - |

## 8. Risks

- 关联 PRD §Technical Risks **R5**（外部 Agent schema 不稳定 / OpenClaw / Hermes 版本漂移）：本 task 不引入新 schema 解析逻辑，纯 wrap 已 verified 的 `internal/importer/<src>` 包；如上游 importer 有 schema bug 不在本 task scope。
- 关联 **R4**（secret redaction 漏检）：本 task 写出的 .md 文件保留 importer 已设的 `redaction_status` frontmatter，但由 task-9.3 的 indexer 路径再做一次 scanner redaction（双层保护）；不引入新 secret 处理逻辑。
- 风险次：YAML frontmatter 用 stdlib 字符串拼接 — 如 content_hash 等字段含特殊字符需 escape；本 task §5.3 `recordToMarkdown` 实现时必须 verify 所有字段值都是 yaml-safe 字符串（用 ctx_id 含 hex 字符 + sha256 hex + ISO 8601 timestamps + 已知 enum 字符串如 "hermes" / "markdown"，全部 ASCII safe）；如未来字段含任意用户输入需引 yaml lib（独立 chore PR）。
- 风险次：内部 importer 包的实际 constructor / Register 模式 task §2A 时 verify — 如三包用 `init()` 自动 register 到全局 registry 而无显式 `New<X>` constructor，本 §5.3 `selectImporter` 改为按 name 从 registry 查找。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit。本 task 不调 Rust 路径不需 cargo build；测试纯 Go fast unit。

## 10. Completion Notes

### 实施摘要

- `internal/cli/import.go`（新）：`runImport` + `parseImportOpts` + `selectImporter` + `importRecords`（hermes 目录展开适配）+ `recordToMarkdown` (YAML frontmatter + body) + `writeRecordsAsMarkdown`
- `internal/cli/cli.go`：dispatch case `"import"` 改为 `return runImport(rest, ...)` 取代 v0.1 `"not implemented"`
- `internal/cli/import_test.go`（新）：7 测试覆盖 AC1-AC5 + missing-path-arg + dispatch-wired
- `internal/cli/cli_test.go`：更新 task-1.4 旧 "not implemented" 测试 — 现在 import 已 wire，断言改为"无 'not implemented' 字串 + 仍 non-zero usage exit"
- `test/fixtures/import-cli/{hermes,openclaw,agent-rules}/`（新 5 fixture 文件）

### §2A verify

主 agent §2A 检查 importer 包实际 export（spec §5.3 + §8 风险次要求）：
- `internal/importer/hermes.New() → importer.Importer` ✓
- `internal/importer/openclaw.NewImporter(agentName string) → importer.Importer` ✓（需 agentName 参数；CLI 用 "openclaw" 默认）
- `internal/importer/agentrules.NewAgentRulesImporter() → importer.Importer` ✓

### 6 项 trade-off 记录

1. **stdlib `flag` 不支持 mixed positional/flag 顺序**：原 spec 隐含 `import hermes <path> --collection X`，但 stdlib `flag.Parse` 在第一个非 flag arg 后停止。修复：parseImportOpts 内手动分离 positional（第一个非 `-` 开头的 arg）+ 剩余喂给 `fs.Parse`。这等价于支持 `import hermes <path> --flag X` 和 `import hermes --flag X <path>` 双顺序，无需引 cobra/pflag（R7 严格 — spec §5.2 明确）
2. **unknown importer 检测在 stat 之前**：原 spec 流程图 stat→selectImporter；实测 `import bogus /tmp/foo` 因 stat 失败先返 exit 1 而非 unknown-importer exit 2。修：把 selectImporter 提前到 stat 前，让 AC4 unknown-importer 走 spec 期望的 exit 2 path
3. **hermes 目录展开**：spec `<path>` 隐含单文件 或目录；实测 hermes.Importer.Import 只接受单文件 (MEMORY.md / USER.md)，但 task-9.6 quickstart fixture 是目录。修：runImport 内当 name=hermes 且 path 是目录时，filepath.WalkDir 找 MEMORY.md / USER.md 逐个 Import 合并。openclaw 自己 walk dir 不需修改；agent-rules 期望单文件按原样
4. **YAML frontmatter 字符串拼接 vs yaml lib**：spec §3 §5.2 明确 R7 严格不引 yaml lib。所有字段都是 ASCII-safe（hex id / enum source_type / 路径 / ISO timestamp / 已知 enum 字符串），用 `fmt.Sprintf` 直接拼接。未来如字段含任意用户输入（如 record.Title 含 special char）需引 yaml lib（独立 chore PR）— §10 disclosed 风险
5. **openclaw 默认 agentName="openclaw"**：openclaw.NewImporter(agentName) 要求 agentName 参数；CLI 没 --agent-name flag 故 hard-code "openclaw" 作 default。如未来需要按用户传 --agent-name 重命名 → 加 flag（小 follow-up，本 task §3 OOS）
6. **task-1.4 cli_test.go 旧 "not implemented" 断言更新**：import 现已 wire，原断言 "stderr substring 'not implemented'" 必然 fail。改为正向断言"无 'not implemented' 字串 + non-zero usage exit"。注释里点明 v0.1 → v0.2 进度（v0.1: stub; v0.2 task-9.4: wired）

### 验证证据

```
$ go test ./internal/cli -run "TestTask94|TestImportSubcommand" -v
  7 测试全 PASS (AC1 Hermes / AC2 OpenClaw / AC3 AgentRules / AC4 unknown +
  missing-path / AC5 dry-run / dispatch-wired)
  exit: 0

$ go vet ./... && go test ./...
  17 包全 ok (含 task-1.4 cli_test 更新后不回归)
  exit: 0
```
