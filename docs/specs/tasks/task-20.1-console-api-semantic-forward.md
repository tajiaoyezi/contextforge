# Task `20.1`: `console-api-semantic-forward — internal/contractv1 SearchRequest 加 add-only Semantic 字段 + internal/consoleapi/handlers.go handleSearch 转发 ?semantic=true / body semantic 到 gRPC SearchRequest.Semantic + grpcclient 透传`

**Status**: Draft

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 20 (semantic-retrieval-throughline)
**Dependencies**: task-19.3（`SearchRequest.semantic` add-only proto 字段 + `internal/daemon/rest.go` `?semantic=true` 参考转发实现）/ task-19.4（`internal/cli/eval.go` `--semantic` + smoke v9，§10 诚实记录 console-api 未转发 semantic）/ ADR-015（Console Contract v1 add-only 兼容）/ ADR-017（22-endpoint conformance）/ ADR-013（禁伪造凭据）/ ADR-014 D1-D5（第十一次激活）

## 1. Background

Phase 19（v0.12.0）已把 `semantic` 加成 `SearchRequest` 的 add-only proto 字段（field 7），并在 `internal/daemon/rest.go:142-146` 把 `/v1/search?semantic=true` query param OR-merge 到 `req.Semantic` 后转发 gRPC `CoreService.Search` 语义分支。但 **console-api（`internal/consoleapi`）的 `/v1/search`** 未走这条通路：`handleSearch`（`internal/consoleapi/handlers.go:443`）只把请求体解码进 `contractv1.SearchRequest`，而 `contractv1.SearchRequest`（`internal/contractv1/contractv1.go:114`）目前**没有** `Semantic` 字段；`searchClient.Search`（`internal/consoleapi/grpcclient/grpcclient.go:356`）把 `contractv1.SearchRequest` 映射到 `pb.SearchRequest{Query, WorkspaceId, ...}` 时也不带 `Semantic`。

task-19.4 §10「设计取舍」已**诚实记录**这一点：smoke step 29 仅断言 `?semantic=true` add-only query param **不破坏** `{result, trace}` 22-endpoint 合约（保形），**不声称**语义检索经 console-api 真正生效；真正语义通路当时仅经 CLI `eval run --semantic`（`searchViaDaemon` → core gRPC，绕过 console-api）。`docs/releases/v0.12.0-evidence.md` §3b 末同样记录该 follow-up。

本 task 闭合该 caveat：让经 console-api `/v1/search` 的语义请求真正转发到 gRPC `SearchRequest.Semantic`，使 Console（及任何走 console-api 的客户端）可请求语义检索。

## 2. Goal

`internal/contractv1/contractv1.go` 的 `SearchRequest` 加 add-only `Semantic bool`（`json:"semantic"`，缺省 false → 既有 BM25 行为不变）。`internal/consoleapi/handlers.go::handleSearch` 在解码 body 后，读 `?semantic=true` query param 并 OR-merge 到 `body.Semantic`（仿 `internal/daemon/rest.go:142-146`：query param 或 body 任一为真即语义）。`internal/consoleapi/grpcclient/grpcclient.go::searchClient.Search` 把 `req.Semantic` 透传到 `pb.SearchRequest.Semantic`。既有 `{result, trace}` 响应 shape 与 22-endpoint conformance 不破坏（add-only 请求字段，响应 shape 不变）。≥3 Go 测试全 PASS；默认 `go test ./...` 不退化；D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **修改 `internal/contractv1/contractv1.go`**：`SearchRequest` 结构体加 `Semantic bool \`json:"semantic"\`` 字段（add-only，置于既有字段后；缺省 false）。
- **修改 `internal/consoleapi/handlers.go`**：`handleSearch` 在 `readJSONBody` 成功后、调 grpcclient 前，插入 `if r.URL.Query().Get("semantic") == "true" { body.Semantic = true }`（query param 与 body 字段 OR-merge，与 `internal/daemon/rest.go` 语义一致）。
- **修改 `internal/consoleapi/grpcclient/grpcclient.go`**：`searchClient.Search` 构造 `pb.SearchRequest{...}` 时加 `Semantic: req.Semantic`（透传到 gRPC）。
- **修改 `internal/consoleapi/handlers_test.go`（或同包测试文件）**：新增 Go 测试断言（a）`contractv1.SearchRequest` JSON round-trip 含 `semantic`；（b）`handleSearch` 在 `?semantic=true` / body `"semantic":true` 任一时使下游 grpcclient 收到 `Semantic=true`，两者皆缺省时为 false；（c）既有 BM25 请求（无 semantic）响应 shape `{result, trace}` 不变。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **proto `SearchRequest.semantic` 字段 + Rust gRPC semantic 路径 + `BruteForceVectorBackend`** [SPEC-OWNER:task-19.3-semantic-search-api]：本 task 复用 Phase 19 已落地的 gRPC 语义分支，不实现它。
- **`internal/daemon/rest.go ?semantic=true` 转发**（另一条 REST surface）[SPEC-OWNER:task-19.3-semantic-search-api]：已由 Phase 19 落地，本 task 以其为参考实现，不改它。
- **真实 `SemanticRecall@K` 经 Retriever 热路径数值** [SPEC-OWNER:task-20.2-real-recall-via-retriever]：本 task 是 console-api 通路 wiring，不产出召回数值。
- **smoke v10 console-api 语义真实断言 + v0.13.0 release docs** [SPEC-OWNER:task-20.3-closeout-v0.13.0]：本 task 落 console-api 转发；smoke 真实断言 + closeout 在收口 task。
- **Console UI 语义 explain 面板** [SPEC-OWNER:phase-future.console-semantic-explain]：跨仓库 Console 领域，本 task 仅就绪其数据通路。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`internal/contractv1.SearchRequest`**：Console Contract v1 请求类型，本 task add-only 加 `Semantic`。
- **`internal/consoleapi/handlers.go::handleSearch`**：`/v1/search` 入口，本 task 加 query-param OR-merge。
- **`internal/consoleapi/grpcclient.searchClient.Search`**：console-api→core gRPC 桥，本 task 透传 `Semantic`。
- **`internal/daemon/rest.go`（参考）**：Phase 19 已实现 `?semantic=true` 转发的另一条 REST surface，本 task 对齐其语义。
- **下游 task-20.2 / 20.3**：20.2 经 Retriever 热路径跑真实召回；20.3 smoke v10 在本通路上做真实语义断言 + closeout。

## 5. Behavior Contract

### 5.1 Required Reading

- `internal/daemon/rest.go:140-147`（`?semantic=true` OR-merge 参考实现）
- `internal/consoleapi/handlers.go::handleSearch`（`/v1/search` body 解码 + grpcclient 调用 + `{result, trace}` 响应封装）
- `internal/consoleapi/grpcclient/grpcclient.go::searchClient.Search`（`contractv1.SearchRequest` → `pb.SearchRequest` 映射）
- `internal/contractv1/contractv1.go::SearchRequest`（既有字段：Query/WorkspaceID/AgentScope/RetrievalMethod/TopK/ConfigSnapshot）
- `proto/contextforge/v1/search.pb.go`（`SearchRequest.Semantic` field 7，task-19.3 已加）
- `docs/specs/tasks/task-19.3-semantic-search-api.md` + `docs/specs/tasks/task-19.4-smoke-v9.md` §10（console-api 未转发的诚实记录）
- `docs/decisions/adr-015-console-contract-v1-compatibility.md`（add-only 兼容）+ `docs/decisions/adr-017-console-contract-completion-22-endpoint.md`（conformance）

### 5.2 关键设计 — query-param 与 body OR-merge

- `handleSearch` 解码 body 后：`if r.URL.Query().Get("semantic") == "true" { body.Semantic = true }`——query param **或** body 字段任一为真即语义路径（与 `internal/daemon/rest.go` 一致，便于 Console 用 query param 或 JSON body 任一方式请求）。
- `searchClient.Search` 映射加 `Semantic: req.Semantic`——`pb.SearchRequest.Semantic` 透传到 core gRPC，由 Phase 19 的语义分支处理。
- `Semantic` 缺省 false：既有不带 semantic 的请求逐字节等价于现状（BM25），向后兼容（ADR-015 add-only）。

### 5.3 不变量

- `/v1/search` 响应 shape 恒为 `{result, trace}`（add-only 仅加请求字段，不改响应结构）；22-endpoint conformance 不破坏。
- proto `SearchRequest` 由 task-19.3 守 add-only；本 task 不新增 proto 字段（仅消费 field 7 + 加 contractv1 Go 字段）。
- semantic 缺省关闭 → 既有 console-api 客户端无需改动。

## 6. Acceptance Criteria

- [ ] **AC1**: `internal/contractv1.SearchRequest` 加 add-only `Semantic bool \`json:"semantic"\``，JSON marshal/unmarshal round-trip 正确（含 / 不含 `semantic` 字段均合法，缺省 false）— verified by **TEST-20.1.1**
- [ ] **AC2**: `handleSearch` 在 `?semantic=true` query param **或** body `"semantic":true` 任一时，使下游 grpcclient 收到 `Semantic=true`；两者皆缺省时为 false（OR-merge 语义与 `internal/daemon/rest.go` 一致）— verified by **TEST-20.1.2**
- [ ] **AC3**: `searchClient.Search` 把 `req.Semantic` 透传到 `pb.SearchRequest.Semantic`；既有 BM25 请求（无 semantic）`{result, trace}` 响应 shape 不变 — verified by **TEST-20.1.3**
- [ ] **AC4**: 既有不退化 — `go test ./...` 全 PASS（含既有 consoleapi / conformance 测试）；`cargo test --workspace` 不受影响（本 PR 零 Rust delta）— verified by **TEST-20.1.4** + §10 实测
- [ ] **AC5**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-20.1.5** + §10 记录

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-20.1.1 | `contractv1.SearchRequest.Semantic` JSON round-trip（含/缺省） | `internal/contractv1/contractv1_test.go` | Planned |
| TEST-20.1.2 | `handleSearch` query-param / body OR-merge → grpcclient 收到 `Semantic` | `internal/consoleapi/handlers_test.go` | Planned |
| TEST-20.1.3 | `searchClient.Search` 透传 `Semantic` + 响应 shape 不变 | `internal/consoleapi/grpcclient/grpcclient_test.go` | Planned |
| TEST-20.1.4 | `go test ./...` 0 failed（含既有 conformance 不退化） | 全 Go | Planned |
| TEST-20.1.5 | D2 lint `--touched origin/master` 0 未标注命中 | `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（中）grpcclient 测试需 fake gRPC server / client 注入**：透传断言需观察下游 `pb.SearchRequest.Semantic`。
  - **缓解**：复用既有 consoleapi 测试的 fake search client 模式（如 task-19.4 / 既有 handlers_test 注入 hook）；断言映射后的 proto 字段，不打真实 core。
- **R2（低）query-param 与 body 双来源语义歧义**：两者冲突时取或值。
  - **缓解**：OR-merge（任一为真即语义），与 `internal/daemon/rest.go` 既有语义一致；测试覆盖四组合（param×body）。
- **R3（低）conformance 因 add-only 字段误判**：22-endpoint conformance 守响应 shape。
  - **缓解**：仅加请求字段，响应 `{result, trace}` 不变；conformance test 复跑确认不破坏（ADR-017）。

## 9. Verification Plan

```bash
# Go：contractv1 字段 + handleSearch 转发 + grpcclient 透传 + 既有不退化
go vet ./internal/...
go test ./internal/contractv1/... ./internal/consoleapi/... -run 'TestTask201' -v
go test ./...

# Rust 不退化（本 PR 零 Rust delta，CI cargo-test gate 复核）
cargo test --workspace

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **Status**: 待实施（Draft）。实施完成后按以下 6 项回填：完成日期 / 改动文件 / commit 列表 / §9 Verification 结果 / 设计取舍 / 剩余风险 + 下游影响。
