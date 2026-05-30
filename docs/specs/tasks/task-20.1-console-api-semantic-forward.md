# Task `20.1`: `console-api-semantic-forward — console_data_plane SearchRequest add-only semantic 字段（buf 重生成）+ Rust SearchServer::query 语义分派（仿 core CoreService task-19.3）+ internal/contractv1 SearchRequest.Semantic + handleSearch ?semantic=true OR-merge + grpcclient 透传`

**Status**: Done

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 20 (semantic-retrieval-throughline)
**Dependencies**: task-19.3（**core** `contextforge/v1 SearchRequest.semantic` + `core/src/server.rs` CoreService 语义分派参考实现 + `internal/daemon/rest.go` `?semantic=true`）/ task-19.2（`Retriever::with_embedder` / `with_vector_searcher` / `enumerate_chunks` / `index_chunks_semantic` / `search_semantic` 生产热路径）/ task-19.1（`DeterministicEmbeddingProvider` + `BruteForceVectorBackend`）/ ADR-015（Console Contract v1 add-only）/ ADR-016（cross-process Rust↔Go gRPC bridge）/ ADR-017（22-endpoint conformance）/ ADR-013（禁伪造）/ ADR-014 D1-D5（第十一次激活）

## 1. Background

Phase 19（v0.12.0）让 **core** `contextforge/v1` proto 的 `SearchRequest` 加了 `semantic`（field 7），并在 `internal/daemon/rest.go` 把 `?semantic=true` 转发到 core `CoreService.Search` 的语义分派（`core/src/server.rs:299-328`：`DeterministicEmbeddingProvider` + 0-dep `BruteForceVectorBackend` 按需建索 + `search_semantic`）。

但 **console-api（`internal/consoleapi`）走的是另一条 gRPC 服务**：`console_data_plane/v1` 的 `SearchService.Query`（ADR-016 cross-process bridge）。**实施期核实发现**（spec drift，§10 记录）：

- `console_data_plane/v1 SearchRequest` **没有** `semantic` 字段（task-19.3 只改了 core `contextforge/v1` proto，未改 console_data_plane proto）。
- Rust `core/src/data_plane/search.rs::SearchServer::query`（console 端 handler）**只走 BM25**（`Retriever::open` + `retriever.search`），无语义分派。
- `internal/contractv1.SearchRequest` 无 `Semantic` 字段；`grpcclient.searchClient.Search` 映射不带 `Semantic`；`handleSearch` 不读 `?semantic=true`。

故 v0.12.0 evidence §3b / task-19.4 §10 记录的「console-api 语义未生效」caveat，真实修复 scope = **console_data_plane proto add-only 字段 + Rust SearchServer.query 语义分派（仿 server.rs）+ Go 三处 wiring**（**非**初版 Draft 误写的「0 Rust/proto delta」）。

## 2. Goal

`console_data_plane.proto` 的 `SearchRequest` 加 add-only `bool semantic = 7`（buf 重生成 Go pb；Rust pb 由 `core/build.rs` tonic_build 在 cargo build 时自动重生成）。Rust `SearchServer::query` 加 `if req.semantic` 分派分支，仿 `core/src/server.rs:299-328`：wire `DeterministicEmbeddingProvider` + `BruteForceVectorBackend`，`enumerate_chunks` + `index_chunks_semantic` + `search_semantic`，hits 携带 `retrieval_method="vector"`；`semantic==false` 时 BM25 路径逐字节不变。`internal/contractv1.SearchRequest` 加 add-only `Semantic bool`；`handleSearch` 读 `?semantic=true` OR-merge 到 body（仿 `internal/daemon/rest.go`）；`grpcclient.searchClient.Search` 透传 `Semantic` 到 `pb.SearchRequest.Semantic`。响应 `{result, trace}` shape 与 22-endpoint conformance 不破坏（仅请求 add-only）。Go + Rust 测试全 PASS；`go test ./...` + `cargo test --workspace` 不退化；D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **proto**：`proto/contextforge/console_data_plane/v1/console_data_plane.proto` `SearchRequest` add-only `bool semantic = 7`；`buf generate proto`（buf 模块 `proto/buf.yaml`）重生成 Go pb；Rust pb 经 `core/build.rs` 在 cargo build 自动重生成。
- **Rust `core/src/data_plane/search.rs`**：`SearchServer::query` 加 `if req.semantic` 语义分派（仿 server.rs；imports `DeterministicEmbeddingProvider` + `BruteForceVectorBackend`）；`RetrievalTrace.candidate_generation_steps`/`lexical_candidates_count`/`vector_candidates_count` 按路径区分。修既有 console `SearchRequest` 字面量补 `semantic: false`（add-only 字段使 exhaustive 字面量需补；`search.rs` 测试 + `data_plane/mod.rs` 字段测试 + `core/tests/{data_plane_integration,search_real_retriever}.rs`）。
- **Go `internal/contractv1/contractv1.go`**：`SearchRequest` add-only `Semantic bool \`json:"semantic"\``。
- **Go `internal/consoleapi/handlers.go`**：`handleSearch` `?semantic=true` OR-merge 到 body（仿 `internal/daemon/rest.go`）。
- **Go `internal/consoleapi/grpcclient/grpcclient.go`**：`searchClient.Search` 透传 `Semantic: req.Semantic`。
- **测试**：Go（`contractv1` JSON round-trip + `handleSearch` OR-merge 四组合 + `grpcclient` 透传 true/false）+ Rust（`search.rs` 语义分派 retrieval_method=vector + BM25 baseline 不变）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **core `CoreService.Search` 语义分派 + `internal/daemon/rest.go` `?semantic=true`** [SPEC-OWNER:task-19.3-semantic-search-api]：Phase 19 已落地；本 task **仿其模式**在 console_data_plane 端实现等价分派，不改 core 端。
- **`vector_score` provenance 独立 proto 字段（console_data_plane SearchResultItem）** [SPEC-DEFER:phase-future.console-vector-score-provenance]：本 task 语义 hits 经既有 `score`（= 向量分）+ `retrieval_method="vector"` 区分；独立 `vector_score` 字段后续按需 add-only。
- **真实 `SemanticRecall@K` 经 Retriever 数值** [SPEC-OWNER:task-20.2-real-recall-via-retriever]：本 task 是通路 wiring + 分派，deterministic embeddings 证 plumbing 非召回质量（ADR-013）。
- **smoke v10 console-api 真实语义断言 + v0.13.0 release docs** [SPEC-OWNER:task-20.3-closeout-v0.13.0]。
- **Console UI 语义 explain 面板** [SPEC-OWNER:phase-future.console-semantic-explain]：跨仓库 Console 领域。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`console_data_plane.proto SearchRequest`**：console contract gRPC 请求，本 task add-only `semantic`。
- **`core/src/data_plane/search.rs::SearchServer::query`**：console 端 gRPC handler，本 task 加语义分派（仿 `server.rs` CoreService）。
- **`internal/contractv1.SearchRequest` / `handleSearch` / `grpcclient.searchClient.Search`**：Go REST→gRPC 链，本 task 透传 `Semantic`。
- **`internal/daemon/rest.go`（参考）**：core REST surface 的 `?semantic=true` 既有实现。
- **下游 task-20.2 / 20.3**：20.2 经 Retriever 跑真实召回；20.3 smoke v10 真实语义断言 + closeout。

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/server.rs:293-328`（**参考实现**：CoreService 语义分派 + TEST-19.3 `test_19_3_semantic_dispatches_vector_path` build_fixture 模式）
- `core/src/data_plane/search.rs::query`（console handler，本 task 加分派）+ 同文件 `mod tests`（`temp_data_dir` / `fresh_server` 模式）
- `proto/contextforge/console_data_plane/v1/console_data_plane.proto`（`SearchRequest` fields 1-6）+ `core/build.rs`（tonic_build 自动重生成）+ `proto/buf.yaml` + 根 `buf.gen.yaml`（`buf generate proto`）
- `internal/daemon/rest.go:140-147`（`?semantic=true` OR-merge 参考）+ `internal/consoleapi/handlers.go::handleSearch` + `grpcclient.go::searchClient.Search` + `internal/contractv1.SearchRequest`
- `core/src/retriever/mod.rs`（`with_embedder` / `enumerate_chunks` / `index_chunks_semantic` / `search_semantic`）+ `core/src/embedding`（`DeterministicEmbeddingProvider`）+ `core/src/retriever/vector::BruteForceVectorBackend`
- `docs/decisions/adr-015-*.md`（add-only）+ `adr-016-*.md`（bridge）+ `adr-017-*.md`（22-endpoint conformance）+ `adr-013-*.md`（禁伪造）

### 5.2 关键设计 — console 语义分派 + Go OR-merge

- `SearchServer::query`：`let hits = if req.semantic { <wire embedder+brute-force, enumerate+index, search_semantic> } else { retriever.search(&opts) };` 下游 chunks/results/trace mapping 共用 `hits`；语义 hits `retrieval_method=="vector"`（来自 `h.retrieval_method`）。deterministic embeddings 证分派非召回（ADR-013）。
- `handleSearch`：`if r.URL.Query().Get("semantic") == "true" { body.Semantic = true }`（query param 或 body 任一为真即语义，与 `internal/daemon/rest.go` 一致）。
- `grpcclient`：`pb.SearchRequest{ ..., Semantic: req.Semantic }`。

### 5.3 不变量

- `semantic` 缺省 false → console-api / contractv1 / proto 既有行为逐字节不变（ADR-015 add-only）；既有客户端无需改动。
- 响应 `{result, trace}` shape 不变（仅请求 add-only）；22-endpoint conformance + proto-freeze 不破坏。
- 默认 `cargo test --workspace`（无 vector feature）不退化；语义分派用 0-dep `BruteForceVectorBackend`（ADR-023 D5 默认 BM25 baseline 守线）。

## 6. Acceptance Criteria

- [x] **AC1**: `console_data_plane SearchRequest` add-only `bool semantic = 7`，`buf generate proto` 重生成 Go pb（`Semantic` 字段 + `GetSemantic`）；Rust pb 经 build.rs 重生成；既有 console SearchRequest 字面量补 `semantic: false` 后 `cargo build` + `cargo test` 编译通过 — verified by **TEST-20.1.1**（含 cargo build/test 编译）
- [x] **AC2**: Rust `SearchServer::query` `semantic==true` 分派语义路径，结果 `retrieval_method=="vector"`；`semantic==false` 保持 BM25（不报 vector 方法）— verified by **TEST-20.1.2**（`test_20_1_query_semantic_dispatches_vector_path`）
- [x] **AC3**: Go 链透传 — `contractv1.SearchRequest.Semantic` JSON round-trip（缺省 false）+ `handleSearch` `?semantic=true`/body OR-merge + `grpcclient.Search` 透传 `pb.SearchRequest.Semantic`（true/false）— verified by **TEST-20.1.3**（contractv1 + handleSearch + grpcclient 三测）
- [x] **AC4**: 既有不退化 — `go test ./...` + `cargo test --workspace` 全 PASS；`{result, trace}` 响应 shape + 22-endpoint conformance 不破坏 — verified by **TEST-20.1.4** + §10 实测
- [x] **AC5**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-20.1.5** + §10 记录

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-20.1.1 | proto add-only `semantic` + buf 重生成 + 字面量补全后编译 | `console_data_plane.proto` + `*.pb.go` + cargo build | Done |
| TEST-20.1.2 | Rust `query` 语义分派 retrieval_method=vector + BM25 baseline | `core/src/data_plane/search.rs`（`test_20_1_query_semantic_dispatches_vector_path`） | Done |
| TEST-20.1.3 | Go contractv1 round-trip + handleSearch OR-merge + grpcclient 透传 | `internal/contractv1/semantic_field_test.go` + `internal/consoleapi/search_semantic_test.go` + `grpcclient_test.go` | Done |
| TEST-20.1.4 | `go test ./...` + `cargo test --workspace` 0 failed | 全 Go + Rust | Done |
| TEST-20.1.5 | D2 lint `--touched origin/master` 0 未标注命中 | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（中，已触发并处理）spec drift：console_data_plane proto 与 core proto 分离**：初版 Draft 误判 console_data_plane 已有 semantic。
  - **处理**：实施期核实修正 scope（proto + Rust + Go）；spec 据实重写（本文件）；§10 记录。Phase 20 为 Draft（非闭合历史，ADR-014 D5 允许在实施中据实修正）。
- **R2（中）proto add-only 破坏既有 exhaustive 字面量**：Rust 字面量需补 `semantic: false`。
  - **缓解**：grep 全 console SearchRequest 字面量（6 处）逐一补 `semantic: false`；`cargo test` 编译复核（add-only 字段使旧字面量不编译 → 编译器强制发现）。
- **R3（低）真实召回数值受 deterministic provider 影响**（承 phase-20 §7 R2）：本 task 用 deterministic embeddings 证分派 plumbing，不预判召回（ADR-013）；真实数值 [SPEC-OWNER:task-20.2-real-recall-via-retriever]。
- **R4（低）22-endpoint conformance 因 add-only 误判**：仅加请求字段，响应 shape 不变。
  - **缓解**：conformance + proto-freeze 守护复跑（ADR-017）。

## 9. Verification Plan

```bash
# proto 重生成（Go pb；Rust pb 经 cargo build）
buf generate proto

# Go：链路透传 + 既有不退化
go vet ./internal/...
go test ./internal/contractv1/... ./internal/consoleapi/... -run 'TestTask201' -v
go test ./...

# Rust：语义分派 + 既有不退化（WSL）
cargo test --workspace
cargo test -p contextforge-core data_plane::search::tests::test_20_1 -- --nocapture

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**：2026-05-31
- **spec-drift 记录（ADR-013 诚实）**：初版 Draft 误称「0 Rust/proto delta」。实施期核实 console-api 走 `console_data_plane/v1`（与 task-19.3 改的 core `contextforge/v1` proto **分离**）——该 proto 无 `semantic` 字段、Rust `SearchServer::query` 仅 BM25。故真实 scope 扩为 proto add-only + buf 重生成 + Rust 语义分派（仿 `server.rs:299-328` CoreService）+ Go wiring。spec 据实重写（本文件，Phase 20 Draft 阶段，ADR-014 D5 允许实施中修正）。
- **改动文件**：`proto/contextforge/console_data_plane/v1/console_data_plane.proto`（`SearchRequest` add-only `bool semantic = 7`）+ `console_data_plane.pb.go`（buf generate proto 重生成）、`core/src/data_plane/search.rs`（`query` 语义分派分支 + 2 imports + trace 字段按路径区分 + `test_20_1_query_semantic_dispatches_vector_path` + 既有 `test_search_server_empty_response` 字面量补 `semantic: false`）、`core/src/data_plane/mod.rs` + `core/tests/{data_plane_integration,search_real_retriever}.rs`（既有 console SearchRequest 字面量补 `semantic: false`，共 6 处）、`internal/contractv1/contractv1.go`（`SearchRequest.Semantic`）+ `internal/contractv1/semantic_field_test.go`（TEST-20.1.3a）、`internal/consoleapi/handlers.go`（`handleSearch` OR-merge）+ `internal/consoleapi/search_semantic_test.go`（TEST-20.1.3b）、`internal/consoleapi/grpcclient/grpcclient.go`（透传）+ `grpcclient_test.go`（TEST-20.1.3c `fakeQueryServer`）、本 spec + `docs/s2v-adapter.md`（20.1 Done）
- **§9 Verification 结果**：`go build ./...` 通过；`go test ./internal/contractv1/... ./internal/consoleapi/...` PASS（含 3 新 Go 测试）；全 `go test ./...` 0 failed；`cargo test --workspace`（WSL2）全 PASS（22 test 二进制全绿 + 新 `test_20_1_query_semantic_dispatches_vector_path` ok；无 phase9 flake）；`buf generate proto` 重生成 Go pb（回退无关 `search.pb.go` CRLF churn）；D2 lint `--touched origin/master` 0 未标注命中（见 commit）。
- **设计取舍（诚实记录）**：Rust 分派用 0-dep `DeterministicEmbeddingProvider` + `BruteForceVectorBackend`（仿 server.rs），deterministic embeddings 证分派 plumbing 非召回质量（真实召回经 Retriever 属 [SPEC-OWNER:task-20.2-real-recall-via-retriever]）。语义 hits 经既有 `score`（向量分）+ `retrieval_method="vector"` 区分，未加独立 `vector_score` proto 字段（[SPEC-DEFER:phase-future.console-vector-score-provenance]，按需后续 add-only）。`DataPlaneStores::new` 返回 `Arc<Self>` → 测试用 `Arc::get_mut` 设 data_dir。
- **剩余风险 / 下游**：真实召回数值 [SPEC-OWNER:task-20.2]；smoke v10 console-api 真实语义断言 + v0.13.0 release docs [SPEC-OWNER:task-20.3]；Console UI explain [SPEC-OWNER:phase-future.console-semantic-explain]（跨仓库）。
