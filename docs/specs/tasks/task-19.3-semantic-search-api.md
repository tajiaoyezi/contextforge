# Task `19.3`: `semantic-search-api — proto SearchRequest add-only semantic flag + RetrievalResult vector_score/embedding_provider provenance + Rust CoreService::search semantic dispatch + Go /v1/search?semantic=true → gRPC`

**Status**: Pending

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 19 (vector-retrieval-integration)
**Dependencies**: task-19.1（`EmbeddingProvider` trait + deterministic 缺省 provider 落地，semantic 路径需 embedding query 向量）/ task-19.2（默认 backend 接 `Retriever::with_vector_searcher` 生产热路径，semantic dispatch 调用此 seam）/ ADR-015 D1 字段冻结 + add-only 演进 pattern / ADR-022 add-only contract 演进 pattern / ADR-014 D1-D5 第十次激活 / task-1.1 proto 冻结规则（only add fields, never delete/renumber tags）

## 1. Background

Phase 19 §3 19.3 把 Phase 18 的向量基础设施 + task-19.1/19.2 的 embedding provider + 默认 backend wiring 暴露到**对外检索接口**：让调用方能显式请求语义检索路径。

当前 `proto/contextforge/v1/search.proto` 的 `SearchRequest` 占用 tag 1-6（`query`=1 / `collections`=2 / `agent_scope`=3 / `top_k`=4 / `filters`=5 / `explain`=6），`RetrievalResult` 占用 tag 1-12（末位 `provenance`=12）。该文件顶部 `// FROZEN at schema_version "0.1"` + context.proto CONTRACT FREEZE RULE：**只增字段，永不删除或重编号 tag**（task-1.1 §AC5 + `core/tests/proto_contract.rs` TEST-1.1.5 守护）。

对外检索通路现状：
- **Rust 数据面**：`core/src/server.rs` `CoreService::search`（`crate::pb::SearchRequest` → `Retriever::open` → `retriever.search/explain` → `SearchResponse`，含 task-6.2 §2A 决策 E chunk_id fast-path）。`Retriever` 已有 `with_vector_searcher` seam（`core/src/retriever/mod.rs:555`），`search()` 内部 task-18.1 placeholder 以零向量调用 vector searcher（结果未并入），task-19.2 将其升级为真实 embedding query 向量驱动的默认 backend。
- **Go 控制面**：`internal/daemon/rest.go` `handleSearch` 仅 `json.NewDecoder(r.Body).Decode(&req)` 解析 body（POST `/v1/search`），未读 URL query param；`internal/daemon/search.go` `Daemon.Search` 透传 gRPC；`internal/cli/search.go` `optsToProtoRequest` 组 `SearchRequest`。
- **生成代码**：Rust 经 `core/build.rs`（tonic-build）在 `cargo build` 时重生成；Go 经 `buf generate`（`buf.gen.yaml` v2，`protoc-gen-go` + `protoc-gen-go-grpc`，`paths=source_relative`）重生成 `proto/contextforge/v1/search.pb.go`。

本 task 按 ADR-015/022 add-only pattern 给 `SearchRequest` 加 `semantic bool`（tag 7），给 `RetrievalResult` 加 `vector_score float`（tag 13，f32）+ `embedding_provider string`（tag 14，provenance），Rust `CoreService::search` 据 `semantic` 分派到 vector searcher 路径，Go `handleSearch` 解析 `?semantic=true` query param 写入 gRPC flag。22-endpoint conformance（`test/conformance/console_contractv1_test.go` + `core/tests/proto_contract.rs`）不破坏。

## 2. Goal

`proto/contextforge/v1/search.proto` add-only 加 `SearchRequest.semantic`（tag 7）+ `RetrievalResult.vector_score`（tag 13）/ `RetrievalResult.embedding_provider`（tag 14）；重生成 Rust（`cargo build`）+ Go（`buf generate`）绑定。`core/src/server.rs` `CoreService::search` 在 `req.semantic == true` 时分派语义路径（经 task-19.2 wiring 的默认 vector searcher + task-19.1 embedding query 向量），结果填 `vector_score` + `embedding_provider`；`semantic == false`（proto3 默认）保持既有 BM25 路径完全不变（向后兼容）。`internal/daemon/rest.go` `handleSearch` 读 `r.URL.Query().Get("semantic")`，`"true"` → `req.Semantic = true` 透传 gRPC。≥3 测试：Rust gRPC semantic roundtrip + Go param parse + contract conformance（22-endpoint + proto freeze 守护）。默认 `cargo test --workspace` + `go test ./...` 不退化；ADR-014 D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **修改 `proto/contextforge/v1/search.proto`** — add-only 三字段（FREEZE RULE 合规，tag 严格递增不复用）：
  - `SearchRequest`：`bool semantic = 7;`（缺省 false → BM25，向后兼容）
  - `RetrievalResult`：`float vector_score = 13;`（f32 语义相似度，BM25 命中为 0）+ `string embedding_provider = 14;`（provenance：产出该结果的 provider name，BM25 命中为空串）
- **重生成绑定** — Rust 经 `cargo build`（`core/build.rs` tonic-build 自动）+ Go 经 `buf generate`（生成 `proto/contextforge/v1/search.pb.go` 新增 `Semantic` / `VectorScore` / `EmbeddingProvider` 字段 + getter）。
- **修改 `core/src/server.rs`** — `CoreService::search`：
  - `req.semantic == true` 分支：经 task-19.2 wiring 的默认 vector searcher + task-19.1 `EmbeddingProvider` 对 `req.query` 生成 query 向量，走语义检索；`search_result_to_proto` 填 `vector_score`（语义路径真实分数）+ `embedding_provider`（provider `name()`）。
  - `req.semantic == false`：完全保持既有 BM25 + chunk_id fast-path 路径；`vector_score = 0.0` / `embedding_provider = ""`（proto3 默认）。
- **修改 `internal/daemon/rest.go`** — `handleSearch`：JSON body decode 后，`if r.URL.Query().Get("semantic") == "true" { req.Semantic = true }`（query param 与 body 字段 OR 合并，query param 优先 set-true，便于 `/v1/search?semantic=true` 直连）。
- **同源 Rust `mod tests`（`core/src/server.rs`）** — semantic roundtrip 单元（`CoreService::search` with `semantic=true` 经 fixture index 拿到结果且 `vector_score`/`embedding_provider` 字段存在）+ semantic=false 既有 BM25 不变守护。
- **同源 Go test（`internal/daemon/rest_test.go`）** — `?semantic=true` query param → 注入 fake `RESTSearcher` 捕获到 `req.Semantic == true`；无 param → `false`。
- **修改 `core/tests/proto_contract.rs`** — TEST-1.1.3 `assert_superset` want 列表追加 `semantic`（SearchRequest）+ `vector_score` / `embedding_provider`（RetrievalResult），守护 add-only 落地且不破坏既有字段集（superset 语义天然向后兼容）。

### Out of Scope（[SPEC-DEFER] / [SPEC-OWNER]）

- **smoke v9 30-step + eval `--semantic` CLI** [SPEC-OWNER:task-19.4-smoke-v9]：端到端 smoke step 29/30 在 task-19.4。
- **真实 dogfood embedding SemanticRecall@K 实测** [SPEC-OWNER:task-19.5-real-recall-eval]：semantic 路径的真实召回数据由 task-19.5 产出，本 task 验通路与契约，不预先 claim 召回数值（ADR-013）。
- **Hybrid scoring（BM25 + Vector fusion）** [SPEC-DEFER:phase-future.hybrid-scoring]：本 task semantic 路径单独，承 Phase 19 §不在 scope。
- **Console UI 端 vector_score explain panel** [SPEC-DEFER:phase-future.console-semantic-explain-panel]：cross-repo Console 领域；本 task 仅评估 add-only 字段是否需通知 Console（task-19.6/19.7 §10 follow-up）。
- **Console 数据面 `pb_console::SearchService`（`core/src/data_plane/search.rs`）的 semantic 暴露** [SPEC-DEFER:phase-future.console-data-plane-semantic]：本 task 限对外 `contextforge.v1` 公共检索通路；Console 数据面 SearchRequest 单独契约（ADR-015 D1 字段冻结），其 semantic 暴露后置。
- **CLI `--semantic` flag** [SPEC-OWNER:task-19.4-smoke-v9]：`internal/cli/search.go` 的 `--semantic` flag 与 eval `--semantic` 一并在 task-19.4。
- **proto `top_k`/filters 对 semantic 路径的语义微调** [SPEC-DEFER:phase-future.semantic-param-tuning]：本 task 复用既有 top_k 语义，参数精调后置。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`SearchRequest.semantic` / `RetrievalResult.vector_score` / `RetrievalResult.embedding_provider`**：proto add-only 新字段，tag 7 / 13 / 14。
- **`CoreService::search`（Rust）**：据 `semantic` flag 分派 BM25 vs 语义路径。
- **`handleSearch`（Go）**：解析 `?semantic=true` → gRPC `Semantic` flag。
- **上游 task-19.1/19.2**：本 task semantic 路径消费其 `EmbeddingProvider` + 默认 vector searcher wiring。
- **下游 task-19.4**：在本 task 通路上加 smoke v9 step 29/30 + eval `--semantic`。
- **下游 task-19.5**：在本 task semantic 路径上跑真实 SemanticRecall@K。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-19-vector-retrieval-integration.md`（§3 19.3 模块拆解 / §5 依赖 / §6 AC3 / §7 R3 proto 演进风险）
- `docs/specs/tasks/task-19.1-spike-embedding-provider.md`（`EmbeddingProvider` trait API：`embed(texts) -> Vec<Vec<f32>>` + `dim()` + `name()`）+ `docs/specs/tasks/task-19.2-default-backend-wiring.md`（`Retriever::with_vector_searcher` 生产接入 + index/query embedding seam）
- `docs/decisions/adr-015-*.md`（D1 字段冻结）+ `docs/decisions/adr-022-*.md`（add-only contract 演进 pattern）
- `proto/contextforge/v1/search.proto`（当前 `SearchRequest` tag 1-6 / `RetrievalResult` tag 1-12 + FREEZE RULE）+ `proto/contextforge/v1/context.proto`（CONTRACT FREEZE RULE 原文）
- `core/src/server.rs`（`CoreService::search` 既有分支 + `search_result_to_proto` 12 字段映射）+ `core/src/retriever/mod.rs`（`search()` task-18.1 vector searcher placeholder + `with_vector_searcher`）
- `internal/daemon/rest.go`（`handleSearch` JSON body decode）+ `internal/daemon/search.go`（`Daemon.Search` 透传）
- `core/tests/proto_contract.rs`（TEST-1.1.3 search 契约 superset + TEST-1.1.5 freeze 守护）+ `test/conformance/console_contractv1_test.go`（22-endpoint conformance 流程）
- `docs/decisions/adr-013-*.md`（无伪造证据：semantic 路径召回数值不在本 task 预先 claim）+ `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5）

### 5.2 Imports / 字段映射（add-only，0 既有破坏）

proto add-only（FREEZE RULE 合规，注释标注承前 add-only 演进）：

```proto
message SearchRequest {
  string query = 1;
  repeated string collections = 2;
  repeated string agent_scope = 3;
  int32 top_k = 4;
  SearchFilters filters = 5;
  bool explain = 6;
  bool semantic = 7;  // Phase 19 task-19.3 add-only: 请求语义检索路径（缺省 false → BM25）
}

message RetrievalResult {
  // ... tag 1-12 unchanged ...
  float vector_score = 13;        // Phase 19 task-19.3 add-only: 语义相似度（BM25 命中为 0）
  string embedding_provider = 14; // Phase 19 task-19.3 add-only: 产出该结果的 embedding provider name（BM25 命中为空）
}
```

Rust `search_result_to_proto`（`core/src/server.rs`）追加两字段映射；Go 经 `buf generate` 自动产出 `SearchRequest.Semantic` / `RetrievalResult.VectorScore` / `RetrievalResult.EmbeddingProvider` + getter。

### 5.3 关键设计

- **add-only 不破坏**：`semantic`/`vector_score`/`embedding_provider` 均新 tag（7 / 13 / 14），proto3 默认值（false / 0.0 / ""）使老 client 与未带 param 的请求行为不变；`proto_contract.rs` superset 断言天然向后兼容。
- **semantic 分派（Rust）**：`CoreService::search` 在 `req.semantic == true` 时走 task-19.2 wiring 的默认 vector searcher（经 task-19.1 `EmbeddingProvider` 对 query 生成向量）；`semantic == false` 走既有 BM25 + chunk_id fast-path，零行为差异。
- **provenance 填充**：语义路径结果 `vector_score` = vector searcher 返回的相似度、`embedding_provider` = provider `name()`；BM25 路径两字段保持 proto3 默认（0.0 / ""），保留 PRD §可解释性 provenance。
- **Go param 合并**：`handleSearch` 在 body decode 后读 `?semantic=true`，query param 为 set-true 旁路（`/v1/search?semantic=true` 直连无需 body 字段），与 body 内 `semantic` OR 合并。
- **生成代码同步**：Rust `cargo build`（`core/build.rs` tonic-build）+ Go `buf generate` 必须在同一 PR 重跑，确保 `.pb.go` 与 `.proto` 不漂移。
- **诚实边界（ADR-013）**：本 task 验「通路通 + 契约 add-only + 字段填充存在」；semantic 路径的真实召回数值由 task-19.5 用 real provider + dogfood 语料产出，本 task 不预先 claim recall 数字。

## 6. Acceptance Criteria

- [ ] **AC1**: proto add-only — `SearchRequest.semantic`（tag 7）+ `RetrievalResult.vector_score`（tag 13）+ `RetrievalResult.embedding_provider`（tag 14）落地，FREEZE RULE 合规（既有 tag 1-12 未删除/未重编号），Rust（`cargo build`）+ Go（`buf generate`）绑定重生成成功 — verified by **TEST-19.3.1**（`proto_contract.rs` superset 含新字段 + freeze 守护 PASS + `buf generate` 后 `.pb.go` 含 `Semantic`/`VectorScore`/`EmbeddingProvider`）
- [ ] **AC2**: Rust gRPC semantic roundtrip — `CoreService::search` with `semantic=true` 经 fixture index 经默认 vector searcher 返回结果，`vector_score`/`embedding_provider` 字段在响应中存在；`semantic=false` 保持既有 BM25 路径不变 — verified by **TEST-19.3.2**（semantic=true roundtrip + semantic=false BM25 守护）
- [ ] **AC3**: Go param parse — `handleSearch` 读 `?semantic=true` → 透传 gRPC `req.Semantic == true`；无 param → `false`（向后兼容） — verified by **TEST-19.3.3**（fake RESTSearcher 捕获 Semantic flag）
- [ ] **AC4**: contract conformance 不破坏 — 22-endpoint conformance（`test/conformance/console_contractv1_test.go` 在 `CONSOLE_REPO` set 时）+ `proto_contract.rs` TEST-1.1.3/1.1.5 全 PASS，add-only 未破坏既有契约 — verified by **TEST-19.3.4**（`go test ./test/conformance/...` + `cargo test -p contextforge-core --test proto_contract` 0 failed）
- [ ] **AC5**: 既有不退化 — 默认 `cargo test --workspace` 全 PASS + `go test ./...` 全 PASS（add-only proto 字段不影响既有 BM25 检索 / REST / CLI 路径） — verified by **TEST-19.3.5**（workspace + go 全测 0 failed）+ §10 实测
- [ ] **AC6**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched master` PR 触及行 0 未标注命中 — verified by **TEST-19.3.6**（§10 记录的 D2 lint 实跑输出）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-19.3.1 | proto add-only 字段集 + freeze 守护 + Go/Rust 绑定重生成 | `core/tests/proto_contract.rs` + `proto/contextforge/v1/search.pb.go` | Pending |
| TEST-19.3.2 | Rust gRPC semantic=true roundtrip + semantic=false BM25 守护 | `core/src/server.rs`（`mod tests`） | Pending |
| TEST-19.3.3 | Go `?semantic=true` param parse → gRPC Semantic flag | `internal/daemon/rest_test.go` | Pending |
| TEST-19.3.4 | 22-endpoint conformance + proto contract 不破坏 | `test/conformance/console_contractv1_test.go` + `core/tests/proto_contract.rs` | Pending |
| TEST-19.3.5 | 默认 cargo test --workspace + go test ./... 0 failed | 全 workspace + 全 Go module | Pending |
| TEST-19.3.6 | D2 lint --touched master 0 未标注命中 | `scripts/spec_drift_lint.sh` | Pending |

## 8. Risks

- **R1（中）proto/contract 演进破坏 conformance**：新增字段若误改既有 tag / 删字段 → 破坏 22-endpoint conformance + 老 client。
  - **缓解**：严格 add-only（tag 7 / 13 / 14 仅递增，1-12 不动）；`proto_contract.rs` superset + freeze 守护；`buf generate` 与 `cargo build` 同 PR 重跑防 `.pb.go` 漂移；conformance test 守护。
- **R2（中）semantic 路径依赖未就绪**：task-19.1 embedding provider / task-19.2 默认 backend wiring 若未 Done，semantic 分支无可用 searcher。
  - **缓解**：本 task dep 19.1 + 19.2（§Dependencies 显式）；semantic=false 默认路径不依赖任何向量基础设施，先确保向后兼容通路；19.1/19.2 就绪后接 semantic 分支。stop-condition：若 19.1 real provider 受阻则 semantic 路径用 deterministic 缺省 provider 跑通通路 + 契约（标注），真实召回 defer task-19.5（ADR-013）。
- **R3（低）vector_score/embedding_provider provenance 语义未定型**：BM25 命中下两字段的缺省语义需文档化。
  - **缓解**：BM25 路径 `vector_score=0.0` / `embedding_provider=""`（proto3 默认）；§5.3 文档化语义；Console 协同评估 add-only 通知（task-19.6/19.7 §10 follow-up）。
- **R4（低）Go query param 与 body 字段冲突**：`?semantic=true` 与 body `semantic:false` 同时出现时的优先级。
  - **缓解**：query param set-true 旁路 + body OR 合并（任一为 true 即 semantic），语义明确且便于 URL 直连；test 守护两路径。

## 9. Verification Plan

```bash
# proto 重生成（Go 端 buf；Rust 端 cargo build 触发 core/build.rs tonic-build）
buf generate
cargo build -p contextforge-core

# AC1 / AC4 — proto add-only 契约 + freeze 守护
cargo test -p contextforge-core --test proto_contract

# AC2 — Rust gRPC semantic roundtrip（server.rs mod tests）
cargo test -p contextforge-core server::tests

# AC3 — Go param parse
go test ./internal/daemon/...

# AC4 — 22-endpoint conformance（CONSOLE_REPO set 时跑，否则 D5 historical-skip）
go test ./test/conformance/...

# AC5 — 既有不退化
cargo test --workspace
go test ./...

# AC6 — D2 spec-drift lint
bash scripts/spec_drift_lint.sh --touched master
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**：（实现后填）
- **改动文件**：`proto/contextforge/v1/search.proto`（add-only semantic + vector_score + embedding_provider）、`proto/contextforge/v1/search.pb.go`（buf 重生成）、`core/src/server.rs`（semantic 分派 + `search_result_to_proto` 两字段映射 + `mod tests`）、`internal/daemon/rest.go`（`handleSearch` query param）、`internal/daemon/rest_test.go`（param parse test）、`core/tests/proto_contract.rs`（superset want 追加）—（实现后据实际 diff 补全）
- **commit 列表**：（实现后填）见本 task PR（分支 `feat/task-19.3-semantic-search-api`）；合入后以 merge commit 为准
- **§9 Verification 结果**：（实现后填）见 PR 描述（`cargo test --workspace` / `go test ./...` / proto_contract / conformance / D2 lint 实测输出）
- **剩余风险 / 未做项**：semantic 路径真实 SemanticRecall@K 见 [SPEC-OWNER:task-19.5-real-recall-eval]；smoke v9 step 29/30 + eval `--semantic` + CLI `--semantic` flag 见 [SPEC-OWNER:task-19.4-smoke-v9]；Console 数据面 semantic 暴露见 [SPEC-DEFER:phase-future.console-data-plane-semantic]；hybrid fusion 见 [SPEC-DEFER:phase-future.hybrid-scoring]
- **下游 task 影响**：task-19.4（在本 task `/v1/search?semantic=true` 通路加 smoke v9 step 29 + eval `--semantic`）；task-19.5（在 semantic 路径跑真实召回）；task-19.6/19.7（评估 add-only 字段是否需通知 Console — cross-repo follow-up）
