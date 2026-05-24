# Task `10.1`: `contractv1-types — internal/contractv1/ Go 镜像 Console Contract v1 must-have 字段`

**Status**: Ready

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 10 (console-contract-v1)
**Dependencies**: 无（基于已 ship 的 Console `console-api/internal/coreadapter/contractv1/contractv1.go` 镜像）

## 1. Background

ContextForge v0.2 内部业务类型分散在 `internal/` 各包；Console v1.0 已定义 Contract v1 must-have 字段 single source of truth (Console PRD §Technical Approach + `contractv1.go`)。task-10.4 9 REST endpoint handler 需要单一 Go 类型层与 Console 对齐 — 不引入镜像 + handler 直接写 map[string]any 会 cross-repo drift。详 [ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md) §D1。

本 task 是 Phase 10 解锁项：先建 Go types 镜像，task-10.4 才能在镜像基础上实现 REST handler。

## 2. Goal

`internal/contractv1/` Go 包包含 17 个 Contract v1 类型（Workspace / IndexJob / SearchRequest / SearchResult / RetrievalTrace / SourceChunk / Citation / MemoryItem / EvalRun / EvalRunCreate / ObservabilityEvent / FieldAvailability / CoreHealth / WorkspaceCreate / CaseResult / AgentScope / MemoryOperation），字段 / json tag / type 与 Console `contractv1.go` 完全一致；`ContractVersion = "v1"` 常量 + FieldAvailability helper (Complete / IsMissing)；`types_test.go` 跑 JSON marshal/unmarshal roundtrip 验证字段 tag + nullable 表达 (`*time.Time` / `*string` / `*int`) 一致；`go vet ./...` + `go test ./internal/contractv1/...` 全绿。

## 3. Scope

### In Scope

- **新增 `internal/contractv1/contractv1.go`**：
  - 1:1 镜像 Console `console-api/internal/coreadapter/contractv1/contractv1.go` 17 个类型 + `ContractVersion = "v1"` 常量 + FieldAvailability struct + Complete() / IsMissing() helper
  - 字段 / json tag / type 严格一致（含 `json.RawMessage` for ConfigSnapshot / `*time.Time` for finished_at 等 nullable 字段 / `*string` for ErrorMessage / `*int` for RankAfterRerank）
  - 包注释引用 Console 仓库路径 + ADR-015 D1 + ContractVersion 演进规则 (`contractv2` add-only freeze)
- **新增 `internal/contractv1/types_test.go`**：
  - 17 个类型每个跑 JSON marshal/unmarshal roundtrip 测试，断言字段 tag 与 Console 镜像一致
  - FieldAvailability Complete() / IsMissing() helper 单测
  - 至少一个 nullable 字段缺失 case 验证 `*T` 解码为 nil 而非零值
- **包约束**：
  - `internal/contractv1/` 只依赖 stdlib `encoding/json` + `time`（同 Console contractv1.go 约束）
  - **不**导入 ContextForge 内部业务包（避免循环依赖）
- 文件锚点：`internal/contractv1/contractv1.go` + `internal/contractv1/types_test.go`

### Out Of Scope

- **Console 端 contractv1.go 任何修改** [SPEC-OWNER:console-team]：cross-repo 只读镜像，Console 字段变更由 Console 端 PR 驱动；本 task 任何时刻 Console 镜像与 ContextForge 镜像不一致 → ADR-014 D4 + playbook §自决规则 #8 转 §8 STOP
- **should-have / optional 字段镜像** [SPEC-DEFER:task-future.contractv1-should-have]：Phase 10 仅 must-have；should-have / optional 字段在 v0.4+ 增量
- **v0.2 现有 internal/ 类型重构** [SPEC-DEFER:task-future.contractv1-internal-refactor]：v0.2 现有业务类型保持原样；task-10.4 REST handler 内部做 contractv1.X ↔ 现有业务类型转换
- **REST handler 实现** [SPEC-OWNER:task-10.4]：本 task 仅提供 types；不写任何 handler
- **Workspace / IndexJob 资源 CRUD** [SPEC-OWNER:task-10.2]/[SPEC-OWNER:task-10.3]：本 task 仅 type 镜像，CRUD 行为在 Rust 侧
- **conformance test**（task-10.5）：本 task types_test.go 仅验证 Go 镜像自身 marshal/unmarshal；与 Console fakehttpserver 对齐的 conformance test 在 task-10.5

## 4. Users / Actors

- **task-10.4 rest-endpoints 实施 agent**（下游）：消费本 task 产出的 contractv1.Workspace / IndexJob / SearchRequest 等类型作为 REST handler 输入输出契约
- **task-10.5 conformance-test 实施 agent**（下游）：消费本 task 产出的镜像作为 conformance 对齐基线
- **Console HTTPAdapter 维护者**（cross-repo 接收方）：Console 端 contractv1.go 字段变更后，需要 PR 同步 ContextForge 镜像（cross-repo amendment）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md` §Open Questions O13
- `docs/decisions/adr-015-console-contract-v1-compatibility.md` §D1
- `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md` §D2 §D3
- `docs/specs/phases/phase-10-console-contract-v1.md`
- `H:/devlopment/code/ContextForge-Console/docs/prds/contextforge-console.prd.md` §Technical Approach「Contract v1 must-have 字段」段
- `H:/devlopment/code/ContextForge-Console/console-api/internal/coreadapter/contractv1/contractv1.go`（single source of truth）

### 5.2 Imports

- **Go**: 仅 stdlib `encoding/json` + `time`
- **不引入新依赖**：R7 不触发；`go.mod` 不动

### 5.3 函数签名

```go
package contractv1

// ContractVersion is the explicit version anchor (ADR-015 §D1).
const ContractVersion = "v1"

// FieldAvailability — Core 实际提供了哪些 must-have 字段，缺失的 must-have
// 字段显式列出，调用方据此优雅降级（PRD §字段分级 / ADR-015 §D1）。
type FieldAvailability struct {
    Object  string   `json:"object"`
    Missing []string `json:"missing_must_have_fields"`
}
func (fa FieldAvailability) Complete() bool { return len(fa.Missing) == 0 }
func (fa FieldAvailability) IsMissing(field string) bool

// 17 个类型：Workspace / WorkspaceCreate / IndexJob / SearchRequest /
// SearchResult / RetrievalTrace / SourceChunk / Citation / MemoryItem /
// MemoryOperation / EvalRun / EvalRunCreate / CaseResult /
// ObservabilityEvent / AgentScope / CoreHealth
//
// 字段 / json tag / 类型与 Console contractv1.go 一致（含 *time.Time /
// *string / *int / json.RawMessage 等 nullable / opaque 表达）。
```

## 6. Acceptance Criteria

- [ ] AC1：`internal/contractv1/contractv1.go` 含 17 Contract v1 类型 + `ContractVersion = "v1"` 常量 + FieldAvailability struct + Complete() / IsMissing() helper；字段 / json tag / type 与 Console `contractv1.go` 1:1 一致（diff 验证）— **verified by unit-test step `go test ./internal/contractv1/... -run TestContractMirrorParity`**
- [ ] AC2：`internal/contractv1/types_test.go` 跑 17 个类型 JSON marshal/unmarshal roundtrip 全过；至少一个 nullable 字段缺失 case 验证 `*T` 解码为 nil — **verified by unit-test step `go test ./internal/contractv1/... -run TestJSONRoundtrip`**
- [ ] AC3：FieldAvailability.Complete() / IsMissing(field) helper 单测全过 — **verified by unit-test step `go test ./internal/contractv1/... -run TestFieldAvailability`**
- [ ] AC4：包注释引用 Console 仓库路径 + ADR-015 D1 + ContractVersion 演进规则；`internal/contractv1/` 只导入 stdlib `encoding/json` + `time`（grep 验证）— **verified by `go list -deps ./internal/contractv1/... | grep -v -E '^(encoding/|time$|internal/contractv1)' = empty`**
- [ ] AC5：`go vet ./...` + `go test ./internal/contractv1/...` 全绿；现有 `go test ./...` 不退化 — **verified by typecheck + unit-test phase smoke**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | 17 类型镜像 + ContractVersion + FieldAvailability | internal/contractv1/contractv1.go | Not Started |
| AC2 | JSON roundtrip 17 类型 + nullable 字段验证 | internal/contractv1/types_test.go::TestJSONRoundtrip | Not Started |
| AC3 | FieldAvailability helper 单测 | internal/contractv1/types_test.go::TestFieldAvailability | Not Started |
| AC4 | 镜像 parity + import 限定 | internal/contractv1/types_test.go::TestContractMirrorParity + `go list -deps` grep | Not Started |
| AC5 | typecheck + unit-test 全绿 | go vet + go test ./... | Not Started |

## 8. Risks

- **Console contractv1.go 字段变更未同步**：cross-repo drift 风险；缓解：types_test.go::TestContractMirrorParity 通过读 Console 仓库源文件 (env `$CONSOLE_REPO`) 跑反射对齐校验；env 未设时 SKIP 不 fail（CI 环境支持）
- **`*time.Time` / `*string` 等 nullable 表达 unmarshal 行为分歧**：Go 标准 json 库 nil pointer 解码 = JSON null；测试验证两个方向
- **Console 端使用 `json.RawMessage` for ConfigSnapshot 在我们镜像也用 RawMessage 但 Go 包不能验证 schema 一致性**：本 task scope 内不验证 schema content；conformance test (task-10.5) 走 fakehttpserver oracle 验证 wire-level shape

## 9. Verification Plan

- **install**: `go mod download`
- **lint**: `gofmt -l internal/contractv1/` (empty output)
- **typecheck**: `go vet ./...`
- **unit-test**: `go test ./internal/contractv1/... -v` (全过 + TestContractMirrorParity + TestJSONRoundtrip + TestFieldAvailability)
- **integration**: N/A (无 integration test)
- **e2e**: N/A
- **build**: `go build ./...`
- **coverage**: ≥85% （contractv1 是纯类型 + helper 包，覆盖率应高）
- **runtime-smoke**: N/A
- **manual**: 主 agent diff 检查 internal/contractv1/contractv1.go vs Console contractv1.go 字段完全一致

## 10. Completion Notes

<!-- 完工时按 standard.md §8.3 6 项 schema 回填 -->

- **完成日期**：<TBD-after-impl>
- **改动文件**：<TBD-after-impl>
- **commit 列表**：<TBD-after-impl>
- **§9 Verification 结果**：
  - install: <TBD-after-impl>
  - lint: <TBD-after-impl>
  - typecheck: <TBD-after-impl>
  - unit-test: <TBD-after-impl>
  - build: <TBD-after-impl>
  - coverage: <TBD-after-impl>
  - manual: <TBD-after-impl>
- **剩余风险 / 未做项**：<TBD-after-impl>
- **下游 task 影响**：task-10.4 / task-10.5 消费本 task 镜像
