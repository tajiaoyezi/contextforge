# Task `45.2`: `daemon-rest-api-freeze — internal/daemon/rest.go 移除 2 个 501 未实装端点（POST /v1/import handleImport + POST /v1/eval/run handleEval，§2A 决策 B 有意留下，console-api /v1/index-jobs + /v1/eval-runs 已完整覆盖）+ 路由注册 :58/:59 移除 + 实装 handleCollections chunk_count（打开 collection metadata.sqlite COUNT 查询，非 placeholder 0）+ rest_test 更新（移除 501 测试 + 加 chunk_count 真实值测试）；v1.0 前 breaking change（major 边界）`

**Status**: Ready
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
- [ ] **AC1**（501 端点移除）: rest.go 无 handleImport/handleEval + 无 :58/:59 路由 — verified by **TEST-45.2.1**（grep 无 501 + go build）
- [ ] **AC2**（chunk_count 实装）: handleCollections chunk_count 真实 COUNT（fixture 索引后 >0） — verified by **TEST-45.2.2**

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-45.2.1 | 501 未实装 移除（rest.go 无 handleImport/handleEval + 路由）+ go build pass | rest.go grep + go build | Not Started |
| TEST-45.2.2 | chunk_count 真实值（fixture 索引后 COUNT>0，非 placeholder 0） | rest_test.go | Not Started |

## 8. Risks
- R1（中）移除破既有调用——v1.0 前 major 边界允许；console-api 覆盖；release notes 记 breaking。
- R2（低）chunk_count 性能——COUNT(*) 快 + collection<10 + best-effort。

## 9. Verification
```bash
go test ./internal/daemon/ -run "Collections|Import|Eval"
go build ./... && go vet ./...
```

## 10. Completion Notes
**Status**: Ready（待实施回填）
