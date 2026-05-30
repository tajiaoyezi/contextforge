# ADR `024`: `console-api-semantic-forward`

**Status**: Accepted (2026-05-31; ratified in Phase 20 task-20.3 closeout on task-20.1's real landing — see the **Amendment / Ratification** section below. Originally Proposed 2026-05-30.)
**Category**: 控制面 / Console Contract v1 / 语义检索通路
**Date**: 2026-05-30
**Decided By**: 主 agent (ADR-012 自治)；tajiaoyezi ratification at v0.13.0 closeout
**Related**: ADR-015 (console-contract-v1-compatibility) / ADR-017 (console-contract-completion-22-endpoint) / ADR-016 (cross-process-rust-go-via-grpc-bridge) / ADR-023 (vector-backend-default) / Phase 19 task-19.3 (`SearchRequest.semantic` proto + `internal/daemon/rest.go` 参考转发) / task-19.4 §10 (console-api 未转发的诚实记录) / Phase 20 (semantic-retrieval-throughline)

## Context

Phase 19（v0.12.0）把 `semantic` 加成 `SearchRequest` 的 add-only proto 字段（field 7），并在 **一条** REST surface（`internal/daemon/rest.go`）把 `/v1/search?semantic=true` query param OR-merge 进 `req.Semantic` 后转发 gRPC `CoreService.Search` 语义分支。

但 ContextForge 有 **两条** `/v1/search` REST surface（ADR-016 cross-process bridge 下控制面有两个 HTTP 入口）：

1. `internal/daemon/rest.go` —— daemon 自带 REST（Phase 19 已转发 semantic）。
2. `internal/consoleapi`（`console-api-serve`）—— Console Contract v1 的 22-endpoint REST，`handleSearch` 仅解码 `contractv1.SearchRequest` body、不读 `?semantic=true`，且 `contractv1.SearchRequest` 无 `Semantic` 字段、`grpcclient.searchClient.Search` 映射不带 `Semantic`。

task-19.4 §10 与 `docs/releases/v0.12.0-evidence.md` §3b 已**诚实记录**：经 console-api 的语义检索在 v0.12.0 未真正生效（仅 CLI `eval run --semantic` 与 daemon REST 走通）。这阻断了 Console UI 语义召回（cross-repo `[SPEC-OWNER:phase-future.console-semantic-explain]`）的数据通路——Console 经 console-api 取数。

## Decision

console-api 的 `/v1/search` 采用 **与 `internal/daemon/rest.go` 一致的 add-only 语义转发**，使两条 REST surface 语义对齐：

### D1 — contractv1 add-only `Semantic` 字段

`internal/contractv1.SearchRequest` 加 `Semantic bool`（`json:"semantic"`，缺省 false）。仿 ADR-015 add-only 兼容：缺省即既有 BM25 行为，既有客户端无需改动；不破坏 22-endpoint conformance（ADR-017）。

### D2 — query-param 与 body OR-merge

`handleSearch` 读 `?semantic=true` query param 并 OR-merge 到 body `Semantic`（任一为真即语义），与 `internal/daemon/rest.go` 既有语义一致——Console 可用 query param 或 JSON body 任一方式请求。

### D3 — grpcclient 透传

`grpcclient.searchClient.Search` 把 `req.Semantic` 透传到 `pb.SearchRequest.Semantic`，复用 Phase 19（task-19.3）已落地的 core gRPC 语义分支（`DeterministicEmbeddingProvider` + 0-dep `BruteForceVectorBackend`），不新增 Rust 数据面改动。

### D4 — 响应 shape 不变

`/v1/search` 响应恒为 `{result, trace}`（add-only 仅加**请求**字段）；result item 携带 Phase 19 已加的 `vector_score` / `embedding_provider` provenance，供 Console explain。响应 shape 不变 → 22-endpoint conformance 不破坏。

## Consequences

- **Positive**: 两条 REST surface 语义对齐；Console 语义召回数据通路就绪（cross-repo explain 的前置）；纯 Go 控制面 add-only 改动，0 Rust delta，0 proto delta（复用 field 7）；缺省 false 向后兼容。
- **Negative / open**: 控制面两条 `/v1/search` surface 并存（ADR-016 既有事实），语义转发逻辑需两处保持一致——以「OR-merge 语义与 daemon REST 一致」约束收敛，测试双覆盖。
- **Ratification**: 本 ADR **Proposed**。task-20.1 落地 + task-20.3 smoke v10 console-api `/v1/search?semantic=true` 真实语义断言（response `retrieval_method` 语义标记 + `vector_score` provenance）通过后，于 v0.13.0 closeout 据真实非合成验证 ratify Proposed→Accepted（ADR-013：禁据合成 / 伪造 ratify）。
- **Follow-ups**: Console UI 语义 explain 面板 `[SPEC-OWNER:phase-future.console-semantic-explain]`（跨仓库 Console 领域）；真实召回经 Retriever 热路径 `[SPEC-DEFER:phase-future.real-recall-via-retriever]`（task-20.2）。

## Amendment / Ratification (2026-05-31, Phase 20 task-20.3 closeout)

> Add-only ratification. The Context / Decision (D1–D4) above are **unchanged**; this section records
> the real implementation basis and **corrects one Consequences claim** (add-only, not a rewrite).

**Implementation correction (ADR-013 honesty)**: the original Consequences "Positive" bullet claimed
"0 Rust delta, 0 proto delta（复用 field 7）". That was written before task-20.1 implementation, which
**discovered spec-drift**: console-api rides the **`console_data_plane/v1`** proto, **separate** from the
core `contextforge/v1` proto that task-19.3's `semantic = 7` lives on. The real implementation is therefore:

- **proto**: `console_data_plane SearchRequest` add-only **`bool semantic = 7`** (its own field 7; buf regen).
- **Rust**: `core/src/data_plane/search.rs::SearchServer::query` gained a semantic dispatch branch mirroring
  core `CoreService.search` (`server.rs`) — `DeterministicEmbeddingProvider` + 0-dep `BruteForceVectorBackend`
  + `enumerate_chunks`/`index_chunks_semantic`/`search_semantic`. **Not** 0 Rust/proto delta.
- **Go**: `contractv1.SearchRequest.Semantic` + `handleSearch` OR-merge + `grpcclient` passthrough (the only
  part the original body got right).

D1–D4's intent (add-only field, query/body OR-merge, grpcclient passthrough, unchanged `{result, trace}`
shape + 22-endpoint conformance) all hold; only the "0 delta" footnote was wrong. Full drift record:
`docs/specs/tasks/task-20.1-console-api-semantic-forward.md` §10.

**Ratification basis (real, non-synthetic — ADR-013)**: `Proposed → Accepted`. Verified by task-20.1
(#155, merged green): `core/src/data_plane/search.rs::test_20_1_query_semantic_dispatches_vector_path`
(semantic dispatch returns `retrieval_method="vector"`) + Go `TestTask201_*` (contractv1 round-trip +
`handleSearch` `?semantic=true`/body OR-merge + grpcclient passthrough to `pb.SearchRequest.Semantic`) +
smoke v10 step 29 (task-20.3) asserting the vector path engaged through console-api. Deterministic
embeddings prove the dispatch plumbing; real recall through this path is task-20.2
(`docs/spikes/phase-20-recall-via-retriever.md`, real fastembed). The 22-endpoint conformance + proto-freeze
guards remained green (response shape unchanged; only an add-only request field).
