# Task `45.3`: `cli-version-help — CLI 加 version 子命令（打印版本，从 cmd/contextforge/main.go 注入版本常量）+ 顶层 --help/-h 处理（修复 cli.go:119-127 -h 落 unknown subcommand exit 2）+ contextforge.example.toml 补全 4 个检索 section（[embedding]/[vector]/[reranker]/[retrieval]）+ 头部 v0.1→v0.38；TEST-45.3.*`

**Status**: Done
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 45 (v1.0-api-cli-freeze)
**Dependencies**: 既有 `internal/cli/cli.go:23-54`（subcommands slice + Execute）+ `:119-127`（unknown subcommand exit 2）+ `cmd/contextforge/main.go`（注入点）+ `contextforge.example.toml`（16 行缺 4 section）+ ADR-050 D2（CLI 冻结）

## 1. Background
CLI 无 `--version`/`version`（v1.0 产品无版本可查 = 硬伤）+ 顶层 `-h`/`--help` 落 unknown subcommand exit 2（cli.go:119-127）+ example.toml 仅 16 行缺 `[embedding]`/`[vector]`/`[reranker]`/`[retrieval]` 4 个 task-22/34/38/41 引入的核心检索 section。

## 2. Goal
(1) `version` 子命令：打印 `contextforge <version>`（版本从 main.go 常量注入，或 build ldflags；初版硬编码 v0.38.0 + 后续 release 前更新或 ldflags）。
(2) 顶层 `--help`/`-h`：在 Execute 入口检测 args[0]=="-h"/"--help"/"help" → 打印子命令清单 + 用法（不 exit 2）。
(3) example.toml 补全 4 section（镜像 config.go 的 section + 注释说明各 env var）+ 头部 v0.1→v0.38.0。

## 3. Scope
- 改 `internal/cli/cli.go`（version 子命令注册 + Execute 入口 -h/--help 处理）
- 改 `cmd/contextforge/main.go`（版本常量 或 ldflags 注入）
- 改 `contextforge.example.toml`（补全 4 section + 头部）
- 改 `internal/cli/cli_test.go`（TEST-45.3.1 version 输出 + TEST-45.3.2 --help 不 exit 2 + TEST-45.3.3 example.toml grep）

## 6. AC
- [x] **AC1**（version 子命令）: `contextforge version` 打印版本 — verified by **TEST-45.3.1**（TestTask453_VersionPrintsVersion）
- [x] **AC2**（顶层 --help）: `contextforge --help`/`-h` 不 exit 2，打印子命令清单 — verified by **TEST-45.3.2**（TestTask453_TopLevelHelpDoesNotExit2）
- [x] **AC3**（example.toml 补全）: 4 个检索 section 在场 — verified by **TEST-45.3.3**（TestTask453_ExampleTomlCoversRetrievalSections）

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-45.3.1 | `contextforge version` 打印版本 | cli_test.go | Done |
| TEST-45.3.2 | `contextforge --help`/`-h` 不 exit 2 + 打印子命令清单 | cli_test.go | Done |
| TEST-45.3.3 | example.toml 含 [embedding]/[vector]/[reranker]/[retrieval] 4 section | cli_test.go grep | Done |

## 9. Verification
```bash
go test ./internal/cli/ -run "Version|Help|Example"
go build ./... && go vet ./...
```

## 10. Completion Notes

**Status**: Done

**完成日期**：2026-07-01

**改动文件**：
- `internal/cli/cli.go`（add `Version` 变量 + subcommands 加 "version" + ExecuteWithIO 入口 -h/--help/help 处理 + switch `case "version":`）
- `internal/cli/cli_test.go`（TEST-45.3.1/.2/.3 + TestTask14_AC4 want 清单更新加 "version"）
- `contextforge.example.toml`（补全 `[embedding]`/`[vector]`/`[reranker]`/`[retrieval]` 4 section + 头部 v0.1→v0.38.0 + 各字段注释）

**§9 Verification**：go test ./internal/cli/ 全绿（含 TestTask453_* + TestTask14_AC4 更新）+ go build + vet + gofmt clean。

**grounding 校正**：TestTask14_AC4 want 清单需同步加 "version"（subcommands 从 9→10，task-45.3 有意新子命令）——当前测试码随契约演进合法更新（ADR-014 D5 约束闭合 phase spec，非当前测试码）。

**下游影响**：task-45.4 closeout（v1.0 CLI 冻结维度 ratify）。
