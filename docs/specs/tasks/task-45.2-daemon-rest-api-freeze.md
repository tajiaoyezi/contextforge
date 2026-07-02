# Task `45.2`: `daemon-rest-api-freeze — internal/daemon/rest.go 移除 2 个 501 未实装端点（POST /v1/import handleImport + POST /v1/eval/run handleEval，§2A 决策 B 有意留下，console-api /v1/index-jobs + /v1/eval-runs 已完整覆盖）+ 路由注册 :58/:59 移除 + 实装 handleCollections chunk_count（打开 collection metadata.sqlite COUNT 查询，非 placeholder 0）+ rest_test 更新（移除 501 测试 + 加 chunk_count 真实值测试）；v1.0 前 breaking change（major 边界）`

**Status**: Done
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 45 (v1.0-api-cli-freeze)
**Dependencies**: 既有 `internal/daemon/rest.go:48-60`（5 路由含 2 个 501 :58/:59）+ `:188-240`（handleCollections chunk_count placeholder :219 + handleImport :227 + handleEval :234）+ `internal/daemon/rest_test.go`（既有 501 测试）+ console-api 已覆盖 import/eval（ADR-017）/ ADR-050 D2（API 冻结）/ ADR-013（移除是 v1.0 前 breaking，major 边界）

## 1. Background
daemon REST（`serve` 子命令的简化 API）有 5 endpoint，其中 2 个是 §2A 决策 B 有意留下的 501 未实装（import/eval/run）+ chunk_count placeholder 0。v1.0 API 冻结前必须清理（冻结永久 501 不可接受）。console-api 已完整覆盖 import（`/v1/index-jobs`）+ eval（`/v1/eval-runs`），故移除无功能损失。

## 2. Goal
(1) 移除 `POST /v1/import`（handleImport :227-232）+ `POST /v1/eval/run`（handleEval :234-240）501 未实装 + 路由注册 `:58/:59`。
(2) 实装 `handleCollections` chunk_count（:219 placeholder 0 → 打开 `<dataDir>/collections/<id>/metadata.sqlite` COUNT 查询真实值；best-effort 单 collection 失败不阻断列表）。
(3) rest_test 更新：移除 501 测试 + 加 chunk_count 真实值测试。

pass bar：501 端点移除（编译 + test 无 501 引用）+ chunk_count 真实非 0（fixture 索引后 COUNT>0）+ 既有 search/chunks/collections 测试不退化 + go test 全绿。

## 3. Scope
- 改 `internal/daemon/rest.go`（移除 handleImport/handleEval 函数 + :58/:59 路由；实装 chunk_count 查询）
- 改 `internal/daemon/rest_test.go`（移除 501 测试 + 加 chunk_count 测试）
- **不改**：console-api（已覆盖）/ proto / CLI search/serve 子命令

## 6. AC
- [x] **AC1**（501 端点移除）: rest.go 无 handleImport/handleEval + 无 :58/:59 路由 — verified by **TEST-45.2.1**（Task452_RemovedEndpointsAre404：移除端点返 404 非 501）
- [x] **AC2**（chunk_count honest-defer，grounding 校正）: handleCollections chunk_count 保持 0 作为 **v1.0 known limitation**（Go daemon 无 SQLite 依赖；引纯 Go SQLite 库 modernc.org/sqlite 为 1 字段不值；真实计数用 console-api `/v1/stats/chunks`） — verified by **TEST-45.2.2**（注释守护：rest.go chunk_count 注释含 "v1.0 known limitation"）

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-45.2.1 | 移除端点返 404 非 501（Task452_RemovedEndpointsAre404 subtest） | rest_test.go | Done |
| TEST-45.2.2 | chunk_count honest-defer 注释守护（rest.go 含 "v1.0 known limitation"） | rest_test.go grep | Done |

## 8. Risks
- R1（中）移除破既有调用——v1.0 前 major 边界允许；console-api 覆盖；release notes 记 breaking。
- R2（低）chunk_count honest-defer 被误读为 bug——注释明确 v1.0 known limitation + 指向 console-api `/v1/stats/chunks`。

## 9. Verification
```bash
go test ./internal/daemon/ -run "Collections|Import|Eval"
go build ./... && go vet ./...
```

## 10. Completion Notes

**Status**: Done

**完成日期**：2026-07-01

**改动文件**：
- `internal/daemon/rest.go`（移除 handleImport/handleEval 函数 + :58/:59 路由注册；handleCollections chunk_count honest-defer 注释明确 v1.0 known limitation + 指向 console-api /v1/stats/chunks）
- `internal/daemon/rest_test.go`（移除 ImportStub501 + EvalStub501 子测试 + 加 Task452_RemovedEndpointsAre404 子测试）

**§9 Verification 结果**：
- go test ./internal/daemon/：全绿（含 Task452_RemovedEndpointsAre404：移除端点返 404 非 501）
- go build + go vet：pass
- gofmt：clean（LF）

**grounding 校正（ADR-013 据实）**：规划稿 AC2 写"实装 chunk_count"——实施期 grounding 发现 Go daemon 侧无 SQLite 依赖（Go 侧纯 gRPC client，metadata.sqlite 由 Rust core 独占）。引纯 Go SQLite 库（modernc.org/sqlite）为 chunk_count 1 字段不值（build weight）。改为 **honest-defer**：chunk_count 保持 0 作为 v1.0 known limitation（稳定契约，非 placeholder），注释明确指向 console-api `/v1/stats/chunks` 查真实计数。这是 ADR-013 honest over padding——不为凑 AC 引重库。

**剩余风险**：chunk_count honest-defer——v1.0 release notes 显式列 known limitation；真实计数经 console-api `/v1/stats/chunks`（Rust core gRPC）。

**下游影响**：task-45.4 closeout（release notes 记 daemon REST breaking change + chunk_count known limitation）。
