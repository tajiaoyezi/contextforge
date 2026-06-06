# Task `39.2`: `console-api-hybrid-forward-and-rerank-visibility — (A) internal/contractv1/contractv1.go add-only SearchRequest.Hybrid bool（json hybrid，镜像 Semantic :125）+ SearchResult.HybridScore float32（json hybrid_score，镜像 VectorScore :153）。(B) internal/consoleapi/handlers.go handleSearch 加 ?hybrid=true OR-merge（镜像 ?semantic :452-454）。(C) internal/consoleapi/grpcclient/grpcclient.go Search 转发 Hybrid: req.Hybrid（镜像 Semantic :372）+ protoToSearchResult 映射 HybridScore: p.HybridScore（镜像 VectorScore :623）。对外 POST /v1/search（body {"hybrid":true} 或 ?hybrid=true）贯通到 core hybrid 路径；rerank reason provenance 在对外 REST 响应可见（reranker 保持 env 驱动、不做 per-request，?rerank=true 据 ADR-044 D3 superseded）；默认 hybrid=false 字节等价；0 新 dep / 0 proto 再改`

**Status**: Done（v0.32.0；PR #253 合入 master @ a9cc6bc；§9 真实验证完成，AC1-AC3 全达成，见 §10）

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 39 (console-api-retrieval-signal-forward)
**Dependencies**: task-39.1（console-dataplane-hybrid-proto-and-dispatch，本 phase 同批；加 `console_data_plane.proto SearchRequest.hybrid=8` + `SearchResultItem.hybrid_score=17` + `buf generate` 重生 Go 生成代码（`pb.SearchRequest.Hybrid` / `pb.SearchResultItem.HybridScore` 可用）+ 数据面 hybrid dispatch——本 task 的 Go 转发消费 task-39.1 的 proto 字段 + 数据面分支）/ 既有 `internal/contractv1/contractv1.go`（`SearchRequest` `:114-127`——`Semantic bool` `:125`（task-20.1 add-only opt-in 范本）；`SearchResult` `:134-157`——`VectorScore float32` `:153`（task-32.3 cross-repo add-only provenance 范本）+ `Reason string` `:154`（rerank provenance 承载字段））/ 既有 `internal/consoleapi/handlers.go` `handleSearch`（`:443-462`——`?semantic` OR-merge `:452-454`，task-20.1 范本）/ 既有 `internal/consoleapi/grpcclient/grpcclient.go` `searchClient.Search`（`:364-390`——`Semantic: req.Semantic` `:372` 转发）+ `protoToSearchResult`（`:609-643`——`VectorScore: p.VectorScore` `:623` + `Reason: p.Reason` `:624` 映射）/ 既有 `internal/consoleapi/search_semantic_test.go`（`TestTask201_HandleSearchSemanticORMerge` `:39-68`——`?hybrid` OR-merge 测试范本）/ ADR-044（console-api-retrieval-signal-forward；本 task = 其 D2 + D3 落点）/ ADR-025（hybrid-scoring-fusion；本 task 经 Go 转发兑现其 `[SPEC-DEFER:phase-future.console-api-hybrid-forward]` 的 Go 半，add-only Phase-39 Amendment 落点 @ task-39.3）/ ADR-043（embedding-remote-reranker-live；本 task = 其 `console-api-rerank-forward` 重界定落点——reranker 保持 env 驱动 D3、provenance 可见性兑现、per-request superseded，add-only Amendment @ task-39.3）/ ADR-024（console-api-semantic-forward；`?hybrid` 镜像其 `?semantic` Phase 20 贯通范式）/ ADR-014 D4（cross-repo add-only signal——`Hybrid` / `HybridScore` ContextForge-Console contractv1 镜像）/ ADR-004（默认 hybrid=false 字节等价 + 既有契约不变）/ ADR-008（dep add-only，0 新 Go dep）/ ADR-013（rerank-forward 重界定据实记录、不夸大为 per-request 实现）/ ADR-012（main-agent-governance-autonomy）/ ADR-014 D1-D5（第三十次激活）

## 1. Background

Phase 20（task-20.1，ADR-024）确立了「对外 REST `?semantic=true` → console 数据面 opt-in」贯通范式：`contractv1.SearchRequest.Semantic`（`:125`）add-only + `handleSearch` 的 `?semantic` OR-merge（`:452-454`）+ `grpcclient.Search` 转发 `Semantic: req.Semantic`（`:372`）。task-39.1（本 phase 同批）加 `console_data_plane.proto SearchRequest.hybrid=8` + `SearchResultItem.hybrid_score=17` + 数据面 hybrid dispatch。但**对外 console-api 仍无法请求 hybrid**——Go console-api 三处缺转发：

- **C1 `contractv1.SearchRequest` 无 `Hybrid` 字段**：`internal/contractv1/contractv1.go` 的 `SearchRequest`（`:114-127`）有 `Semantic bool`（`:125`，task-20.1）但**无 `Hybrid`**；`SearchResult`（`:134-157`）有 `VectorScore float32`（`:153`，task-32.3）但**无 `HybridScore`**——对外 REST body 无 hybrid 请求落点、响应无融合分 provenance 字段。
- **C2 `handleSearch` 无 `?hybrid` 解析**：`internal/consoleapi/handlers.go` 的 `handleSearch`（`:443-462`）只 OR-merge `?semantic`（`:452-454`）——**无 `?hybrid`**，对外 `?hybrid=true` query param / body `{"hybrid":true}` 无处生效。
- **C3 `grpcclient` 不转发 `Hybrid` / 不映射 `HybridScore`**：`grpcclient.Search`（`:364-390`）转发 `Semantic: req.Semantic`（`:372`）但**不转发 `Hybrid`**；`protoToSearchResult`（`:609-643`）映射 `VectorScore: p.VectorScore`（`:623`）+ `Reason: p.Reason`（`:624`）但**不映射 `HybridScore`**——hybrid flag 断在 Go 侧、融合分不回传。

且 rerank provenance 在对外 REST 的可见性从未被断言：rerank `reason` 标记经既有链路（Rust `SearchResult.reason` → proto `reason=14` → `grpcclient.protoToSearchResult` `Reason: p.Reason` `:624` → `contractv1.SearchResult.Reason` → REST JSON）端到端流通，但无测试 / smoke 断言它在对外 `POST /v1/search` 响应可见。

本 task 关闭 console-api hybrid 转发 + rerank provenance 可见性，是 ADR-044 D2 + D3：

- **B1 contractv1 add-only（镜像 Semantic / VectorScore）**：`SearchRequest` 加 `Hybrid bool`（json `hybrid`，镜像 `Semantic` `:125` add-only opt-in 范式）+ `SearchResult` 加 `HybridScore float32`（json `hybrid_score`，镜像 `VectorScore` `:153` cross-repo add-only provenance 范式，ADR-014 D4）。
- **B2 `handleSearch` `?hybrid` OR-merge（镜像 ?semantic）**：`if r.URL.Query().Get("hybrid") == "true" { body.Hybrid = true }`（紧随 `?semantic` `:452-454`）——`?hybrid=true` query param 或 body `{"hybrid":true}` 均可请求 hybrid。
- **B3 `grpcclient` 转发 + 映射（镜像 Semantic / VectorScore）**：`Search`（`:364-390`）的 `pb.SearchRequest` 构造加 `Hybrid: req.Hybrid`（紧随 `Semantic: req.Semantic` `:372`）；`protoToSearchResult`（`:609-643`）加 `HybridScore: p.HybridScore`（紧随 `VectorScore: p.VectorScore` `:623`）。
- **B4 对外 REST 贯通**：`POST /v1/search` body `{"query":"...","hybrid":true}` 或 `?hybrid=true` ⇒ `handleSearch` OR-merge ⇒ `contractv1.SearchRequest.Hybrid=true` ⇒ `grpcclient` 转发 `pb.SearchRequest.Hybrid=true` ⇒ console 数据面（task-39.1）hybrid dispatch ⇒ 响应 `retrieval_method="hybrid"` + `hybrid_score` + （reranker env opt-in 时）`reason` rerank marker。
- **B5 rerank provenance 可见性（reranker 保持 env 驱动，不做 per-request；ADR-044 D3）**：reranker 由 `CONTEXTFORGE_RERANKER_PROVIDER` env 服务端 opt-in（ADR-043 D3 不变，**本 task 不加 `?rerank` 参数**）；`reason` 经既有链路（`grpcclient.protoToSearchResult` `Reason: p.Reason` `:624`）端到端流通——本 task 加 Go 测试断言 reranker env opt-in 时 `reason` 经 `protoToSearchResult` 映射到 `contractv1.SearchResult.Reason`（对外 REST 可见性 smoke 端到端断言属 task-39.3）。`?rerank=true` per-request 控制据 ADR-044 D3 重界定为被 ADR-043 D3（env 驱动）取代（superseded）、不实现 `[SPEC-DEFER:phase-future.console-api-rerank-forward]`。
- **B6 默认 `hybrid=false` 字节等价（向后兼容，ADR-004）**：不设 `?hybrid` / body `hybrid` ⇒ `contractv1.SearchRequest.Hybrid=false` ⇒ `grpcclient` 转发 `false` ⇒ 数据面走既有 semantic / BM25 ⇒ 响应字节等价；既有 REST client 行为不变。

本 task 为 code-local 🟢 可验证（`handleSearch` `?hybrid` OR-merge 单测 + `grpcclient` 转发/映射单测）；Go 侧 0 新 dep（沿用既有 contractv1 + stdlib + task-39.1 重生的 pb 生成代码）；0 proto 再改（消费 task-39.1 字段）/ 0 migration。

## 2. Goal

(1) **B1 contractv1 add-only**：`internal/contractv1/contractv1.go` 的 `SearchRequest`（`:114-127`）加 `Hybrid bool` json tag `hybrid`（紧随 `Semantic` `:125`，doc 注 `task-39.2 (Phase 39) add-only opt-in hybrid flag; OR-merged from ?hybrid=true or body, forwarded to gRPC; 默认 false → BM25 / semantic 向后兼容`）；`SearchResult`（`:134-157`）加 `HybridScore float32` json tag `hybrid_score`（紧随 `VectorScore` `:153`，doc 注镜像 `VectorScore` cross-repo add-only provenance + parity with console_data_plane SearchResultItem.hybrid_score=17）。(2) **B2 `handleSearch` `?hybrid` OR-merge**：`internal/consoleapi/handlers.go` `handleSearch`（`:443-462`）加 `if r.URL.Query().Get("hybrid") == "true" { body.Hybrid = true }`（紧随 `?semantic` `:452-454`）。(3) **B3 `grpcclient` 转发 + 映射**：`internal/consoleapi/grpcclient/grpcclient.go` `Search`（`:364-390`）的 `pb.SearchRequest` 加 `Hybrid: req.Hybrid`（紧随 `Semantic: req.Semantic` `:372`）；`protoToSearchResult`（`:609-643`）加 `HybridScore: p.HybridScore`（紧随 `VectorScore: p.VectorScore` `:623`）。(4) **B4 对外 REST 贯通**：`POST /v1/search`（body `{"hybrid":true}` 或 `?hybrid=true`）→ core hybrid 路径 → 响应携 `retrieval_method="hybrid"` + `hybrid_score`。(5) **B5 rerank provenance 可见性**：reranker 保持 env 驱动（ADR-043 D3，不加 `?rerank` 参数）；本 task 加 Go 测试断言 `reason` 经 `protoToSearchResult` 映射可见；`?rerank=true` per-request 据 ADR-044 D3 superseded `[SPEC-DEFER:phase-future.console-api-rerank-forward]`。(6) **B6 默认 `hybrid=false` 字节等价**：不设 hybrid ⇒ `Hybrid=false` ⇒ 转发 false ⇒ 数据面 semantic / BM25 ⇒ 响应字节等价（ADR-004 向后兼容）。(7) **0 dep**：Go 侧沿用既有 contractv1 + stdlib + task-39.1 重生 pb 生成代码（0 新 Go dep，ADR-008）。

pass bar：`contractv1.SearchRequest.Hybrid` + `SearchResult.HybridScore` add-only（既有字段不受影响）（🟢）；`handleSearch` `?hybrid` OR-merge 经单测验证（`?hybrid=true` ⇒ `body.Hybrid=true`；body `{"hybrid":true}` ⇒ `Hybrid=true`；两者皆无 ⇒ `Hybrid=false`）（🟢）；`grpcclient` 转发 `req.Hybrid` → `pb.SearchRequest.Hybrid` + 映射 `p.HybridScore` → `out.HybridScore` + `p.Reason` → `out.Reason`（rerank provenance 承载）经单测验证（🟢）；默认 `hybrid=false` 字节等价（ADR-004）+ 既有契约（`contractv1` 既有字段 / `handleSearch` `?semantic` / `grpcclient` 既有转发映射）不变；reranker 保持 env 驱动（不加 `?rerank` 参数，ADR-043 D3）；0 新 dep；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `internal/contractv1/contractv1.go`——`SearchRequest`（`:114-127`）add-only `Hybrid bool` json tag `hybrid`（紧随 `Semantic` `:125`）；`SearchResult`（`:134-157`）add-only `HybridScore float32` json tag `hybrid_score`（紧随 `VectorScore` `:153`，doc 注镜像 `VectorScore` cross-repo add-only provenance）。
- 改 `internal/consoleapi/handlers.go`——`handleSearch`（`:443-462`）加 `if r.URL.Query().Get("hybrid") == "true" { body.Hybrid = true }`（紧随 `?semantic` `:452-454`）。
- 改 `internal/consoleapi/grpcclient/grpcclient.go`——`Search`（`:364-390`）`pb.SearchRequest` 加 `Hybrid: req.Hybrid`（紧随 `:372`）；`protoToSearchResult`（`:609-643`）加 `HybridScore: p.HybridScore`（紧随 `:623`）。
- 同源测试：`internal/consoleapi` 同包 test（`handleSearch` `?hybrid` OR-merge，镜像 `TestTask201_HandleSearchSemanticORMerge`）+ `internal/consoleapi/grpcclient` 同包 test（`Search` 转发 `Hybrid` + `protoToSearchResult` 映射 `HybridScore` / `Reason`）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- console_data_plane proto `hybrid=8` / `hybrid_score=17` 字段 + 数据面 hybrid dispatch 的**实现**——属 task-39.1（本 phase 同批）；本 task 的 Go 转发**消费** task-39.1 的 proto 字段（`buf generate` 后 `pb.SearchRequest.Hybrid` / `pb.SearchResultItem.HybridScore`）+ 数据面分支（依赖 task-39.1，不在本 task 重复实现）。
- console-api 把 reranker 暴露为 `?rerank=true` per-request 参数 [SPEC-DEFER:phase-future.console-api-rerank-forward]——据 ADR-044 D3 诚实校正：reranker 保持服务端 env 驱动（ADR-043 D3），per-request 转发与 env 驱动模型冲突、被取代（superseded）不实现；本 task 改交付 rerank provenance（`reason`）可见性，per-request 控制项作 superseded 记录而非交付项。
- 对外 REST `POST /v1/search` 端到端 rerank `reason` 可见性 smoke 断言（`CONTEXTFORGE_RERANKER_PROVIDER=identity` ⇒ 响应 `reason` 含 rerank marker）——属 task-39.3（smoke step）；本 task 加 Go 单测断言 `protoToSearchResult` 映射 `Reason`（链路单元），端到端 smoke 在 closeout。
- Console UI hybrid / rerank explain 面板（融合分可视化 / rerank 前后排序对比 UI）[SPEC-OWNER:phase-future.console-semantic-explain]——跨仓库 Console 领域，本 task 限对外 REST 信号转发 + provenance 字段。
- 大语料 hybrid 对外 REST 召回质量基准 [SPEC-DEFER:phase-future.vector-large-corpus-perf]——本 task 为转发 wiring，hybrid 融合质量 ADR-025 已 ratify、rerank 质量 ADR-043 已 ratify，不重测。
- 真实 release tag / run-id / digest（v0.32.0）[SPEC-OWNER:task-39.3-closeout]（ADR-012 用户授权后回填）。

## 4. Actors

- 主 agent（ADR-012 自治）
- `contractv1.SearchRequest`（`internal/contractv1/contractv1.go:114-127`，本 task add-only `Hybrid bool`，镜像 `Semantic` `:125`）
- `contractv1.SearchResult`（`:134-157`，本 task add-only `HybridScore float32`，镜像 `VectorScore` `:153`；`Reason` `:154` rerank provenance 承载字段）
- `handleSearch`（`internal/consoleapi/handlers.go:443-462`，本 task 加 `?hybrid` OR-merge，镜像 `?semantic` `:452-454`）
- `searchClient.Search`（`internal/consoleapi/grpcclient/grpcclient.go:364-390`，本 task 加 `Hybrid: req.Hybrid` 转发，镜像 `Semantic` `:372`）
- `protoToSearchResult`（`:609-643`，本 task 加 `HybridScore: p.HybridScore` 映射，镜像 `VectorScore` `:623`；`Reason: p.Reason` `:624` 既有映射）
- 对外 console-api 调用方（经 `POST /v1/search` body `{"hybrid":true}` 或 `?hybrid=true` 请求 hybrid，看到 `hybrid_score` 融合分 + rerank `reason` provenance）
- task-39.1 的 console 数据面 hybrid dispatch（既有 hybrid 分支接收点，本 task 不改其内部）

## 5. Behavior Contract

### 5.1 Required Reading

- `internal/contractv1/contractv1.go:114-127`（`SearchRequest`——`Semantic bool` `:125` doc 注「task-20.1 add-only opt-in ... OR-merged ... forwarded to gRPC ... 默认 false → BM25」——`Hybrid` 镜像之）+ `:134-157`（`SearchResult`——`VectorScore float32` `:153` doc 注「task-32.3 add-only ... cross-repo add-only signal, ADR-014 D4」——`HybridScore` 镜像之；`Reason string` `:154` rerank provenance 承载）
- `internal/consoleapi/handlers.go:443-462`（`handleSearch`——`?semantic` OR-merge `:452-454`「`if r.URL.Query().Get("semantic") == "true" { body.Semantic = true }`」——`?hybrid` 镜像之）
- `internal/consoleapi/grpcclient/grpcclient.go:364-390`（`searchClient.Search`——`pb.SearchRequest` 构造 `Semantic: req.Semantic` `:372`——`Hybrid: req.Hybrid` 镜像之）+ `:609-643`（`protoToSearchResult`——`VectorScore: p.VectorScore` `:623` + `Reason: p.Reason` `:624`——`HybridScore: p.HybridScore` 镜像之）
- `internal/consoleapi/search_semantic_test.go:39-68`（`TestTask201_HandleSearchSemanticORMerge`——`?semantic` OR-merge 测试范本，`?hybrid` 测试镜像之）
- `proto/contextforge/console_data_plane/v1/console_data_plane.proto`（task-39.1 加 `SearchRequest.hybrid=8` + `SearchResultItem.hybrid_score=17`——`buf generate` 后 Go `pb.SearchRequest.Hybrid` / `pb.SearchResultItem.HybridScore` 可用，本 task 消费）
- `core/src/data_plane/search.rs`（task-39.1 加数据面 hybrid dispatch——本 task 转发的 `pb.SearchRequest.Hybrid=true` 在此分派 `search_hybrid`）
- `docs/decisions/adr-044-console-api-retrieval-signal-forward.md §D2/§D3`（本 task 即其原文实现）+ `docs/decisions/adr-025-hybrid-scoring-fusion.md`（hybrid 母 ADR，本 task = 其 `console-api-hybrid-forward` Go 半兑现）+ `docs/decisions/adr-043-embedding-remote-reranker-live.md §D3`（reranker env 驱动——本 task 据其不加 `?rerank` 参数，`console-api-rerank-forward` 重界定为 provenance 可见性）+ ADR-024（`?semantic` Phase 20 贯通范式）/ ADR-014 D4（cross-repo add-only signal）/ ADR-004（默认 hybrid=false 字节等价）/ ADR-008（0 新 Go dep）/ ADR-013（rerank-forward 重界定据实记录）

### 5.2 关键设计 — Go console-api hybrid 转发 + rerank provenance 可见性（镜像 ?semantic 范式 / reranker 保持 env 驱动 / 默认 hybrid=false 字节等价）

- **B1 contractv1 add-only（镜像 Semantic / VectorScore）**：`SearchRequest` 加 `Hybrid bool`（json `hybrid`）——镜像 `Semantic` `:125` 的 add-only opt-in flag（默认 `false` → BM25 / semantic 向后兼容）；`SearchResult` 加 `HybridScore float32`（json `hybrid_score`）——镜像 `VectorScore` `:153` 的 cross-repo add-only provenance（ContextForge-Console contractv1 镜像同字段，ADR-014 D4；parity with console_data_plane `SearchResultItem.hybrid_score=17`）。既有字段不受影响（add-only）。
- **B2 `handleSearch` `?hybrid` OR-merge（镜像 ?semantic）**：紧随 `?semantic`（`:452-454`）加 `if r.URL.Query().Get("hybrid") == "true" { body.Hybrid = true }`——`?hybrid=true` query param OR-merge 进 body flag（body `{"hybrid":true}` 亦可），与 `?semantic` 同语义。**`?hybrid` / `?semantic` 独立**（用户可单独请求 hybrid 或 semantic；core 数据面 dispatch 优先级 `if req.hybrid {..} else if req.semantic {..}`，task-39.1，hybrid 优先）。
- **B3 `grpcclient` 转发 + 映射（镜像 Semantic / VectorScore）**：`Search`（`:364-390`）的 `pb.SearchRequest` 构造紧随 `Semantic: req.Semantic`（`:372`）加 `Hybrid: req.Hybrid`——hybrid flag 1:1 转发到 core gRPC；`protoToSearchResult`（`:609-643`）紧随 `VectorScore: p.VectorScore`（`:623`）加 `HybridScore: p.HybridScore`——融合分 1:1 映射回 contractv1。`Reason: p.Reason`（`:624`）既有映射不变（rerank provenance 承载）。
- **B4 对外 REST 贯通**：`POST /v1/search` body `{"query":"...","hybrid":true}` 或 `?hybrid=true` ⇒ `handleSearch` OR-merge `body.Hybrid=true` ⇒ `deps.Search.Search(body)` ⇒ `grpcclient.Search` 转发 `pb.SearchRequest.Hybrid=true` ⇒ console 数据面（task-39.1）hybrid dispatch `search_hybrid` ⇒ `SearchResultItem.retrieval_method="hybrid"` + `hybrid_score` ⇒ `protoToSearchResult` 映射 `HybridScore` + `Reason` ⇒ REST JSON `{"result":{...,"retrieval_method":"hybrid","hybrid_score":...,"reason":...},"trace":...}`。
- **B5 rerank provenance 可见性（reranker 保持 env 驱动，不做 per-request；ADR-044 D3 / ADR-043 D3）**：reranker 由 `CONTEXTFORGE_RERANKER_PROVIDER` env 服务端 opt-in（**本 task 不加 `?rerank` 参数**——per-request 与 env 驱动冲突）；rerank `reason`（如 `IDENTITY_RERANK_REASON`）经既有链路（Rust → proto `reason=14` → `protoToSearchResult` `Reason: p.Reason` `:624` → `contractv1.SearchResult.Reason` → REST JSON）端到端流通；本 task 加 Go 单测断言 `protoToSearchResult` 映射 `Reason`（链路单元正确），对外 REST 端到端可见性 smoke 在 task-39.3。`?rerank=true` per-request 据 ADR-044 D3 记为被 ADR-043 D3（env 驱动）取代（superseded）、不实现 `[SPEC-DEFER:phase-future.console-api-rerank-forward]`，据实声明不夸大为「实现了 per-request rerank 转发」（ADR-013）。
- **B6 默认 `hybrid=false` 字节等价（ADR-004 向后兼容）**：不设 `?hybrid` query param 且 body 无 `hybrid` ⇒ `body.Hybrid=false`（Go bool 默认）⇒ `grpcclient` 转发 `pb.SearchRequest.Hybrid=false` ⇒ 数据面走既有 `else if req.semantic` / BM25 ⇒ 响应字节等价；既有 REST client（不设 `hybrid`）行为不变；`HybridScore` 对非 hybrid 命中为 `0.0`（task-39.1 数据面填 `0.0`）。

### 5.3 不变量

- 默认行为不变（ADR-004）：不设 `?hybrid` / body `hybrid` ⇒ `body.Hybrid=false` ⇒ 转发 `false` ⇒ 数据面 semantic / BM25 ⇒ 响应字节等价；既有 REST client 行为不变；`?semantic` 路径不受影响（`?hybrid` 独立 add-only）。
- 既有契约不变：`contractv1.SearchRequest` 既有字段（`Query` / `WorkspaceID` / `AgentScope` / `RetrievalMethod` / `TopK` / `ConfigSnapshot` / `Semantic`）+ `SearchResult` 既有字段（含 `VectorScore` / `Reason`）不变（add-only `Hybrid` / `HybridScore`）；`handleSearch` `?semantic` OR-merge 不变（add-only `?hybrid`）；`grpcclient.Search` 既有转发 + `protoToSearchResult` 既有映射不变（add-only `Hybrid` 转发 / `HybridScore` 映射）。
- reranker 仍 env 驱动（ADR-043 D3 / ADR-044 D3）：本 task **不加** `?rerank` per-request 参数；reranker 由 `CONTEXTFORGE_RERANKER_PROVIDER` env 服务端 opt-in；`reason` 经既有链路端到端流通（本 task 加测试断言可见性，不改链路）。`?rerank=true` per-request 记为 superseded（据实，不矛盾实现）。
- 0 新代码依赖（ADR-008）：Go 侧沿用既有 contractv1 + `net/http` + task-39.1 重生 pb 生成代码；无第三方依赖增量。
- cross-repo add-only signal（ADR-014 D4）：`Hybrid` / `HybridScore` 是 ContextForge contractv1 add-only 字段，ContextForge-Console contractv1 镜像同字段（与 `Semantic` / `VectorScore` 同范式），非破坏性变更。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [x] **AC1**（contractv1 add-only + `handleSearch` `?hybrid` OR-merge 🟢）: `SearchRequest.Hybrid bool`（json `hybrid`，镜像 `Semantic`）+ `SearchResult.HybridScore float32`（json `hybrid_score`，镜像 `VectorScore`）add-only（既有字段不受影响）；`handleSearch` `?hybrid=true` ⇒ `body.Hybrid=true`；body `{"hybrid":true}` ⇒ `Hybrid=true`；两者皆无 ⇒ `Hybrid=false`（向后兼容，镜像 `?semantic` :452-454）— verified by **TEST-39.2.1**（`handleSearch` `?hybrid` OR-merge）
- [x] **AC2**（`grpcclient` 转发 + 映射 + rerank provenance 可见 🟢）: `Search` 转发 `Hybrid: req.Hybrid` 到 `pb.SearchRequest.Hybrid`（镜像 `Semantic` :372）；`protoToSearchResult` 映射 `HybridScore: p.HybridScore`（镜像 `VectorScore` :623）+ `Reason: p.Reason`（rerank provenance 承载，既有 :624）；reranker 保持 env 驱动（不加 `?rerank` 参数，ADR-043 D3 / ADR-044 D3）；默认 `hybrid=false` 转发 `false` 字节等价；0 新 dep — verified by **TEST-39.2.2**（`grpcclient` 转发 + 映射）
- [x] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-39.2.3**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-39.2.1 | `handleSearch` `?hybrid` OR-merge（镜像 `TestTask201_HandleSearchSemanticORMerge`）：`?hybrid=true` query param ⇒ `body.Hybrid=true`；body `{"hybrid":true}` ⇒ `Hybrid=true`；两者皆无 ⇒ `Hybrid=false`（默认向后兼容）；`?hybrid` / `?semantic` 独立（设 `?hybrid` 不影响 `Semantic`）；`SearchRequest.Hybrid` / `SearchResult.HybridScore` json round-trip（add-only，既有字段不受影响） | `internal/consoleapi/search_semantic_test.go` 或同包新 test | Draft |
| TEST-39.2.2 | `grpcclient` 转发 + 映射：`Search` 把 `req.Hybrid` 转发到 `pb.SearchRequest.Hybrid`（镜像 `Semantic` :372）；`protoToSearchResult` 把 `p.HybridScore` 映射到 `out.HybridScore`（镜像 `VectorScore` :623）+ `p.Reason` 映射到 `out.Reason`（rerank provenance 承载，既有 :624）；默认 `Hybrid=false` 转发 `false`（字节等价）；reranker 不经 `grpcclient` 转发（env 驱动，无 `?rerank` 字段） | `internal/consoleapi/grpcclient` 同包 test | Draft |
| TEST-39.2.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Draft |

## 8. Risks

- **R1（高）`?hybrid` 转发破默认无 hybrid 字节等价（向后兼容）**：若 `handleSearch` / `grpcclient` 误把 `Hybrid` 默认设 `true` 或无条件转发，改变默认检索结果。
  - **缓解**：`Hybrid bool` 默认 `false`（Go bool 零值）；`handleSearch` 仅 `?hybrid=true` 时 `body.Hybrid=true`（OR-merge，镜像 `?semantic`）；`grpcclient` 转发 `req.Hybrid`（false 时转发 false）；TEST-39.2.1 含「两者皆无 ⇒ `Hybrid=false`」+ TEST-39.2.2 含「默认 `Hybrid=false` 转发 false」断言。stop-condition：默认字节等价单测退化则 AC1/AC2 不标 `[x]`。
- **R2（中）`?rerank=true` per-request 被误实现（与 ADR-043 D3 冲突）**：若本 task 顺手加 `?rerank` 参数转发，破 env 驱动模型（ADR-043 D3）。
  - **缓解**：据 ADR-044 D3——reranker 保持 env 驱动、本 task 不加 `?rerank` 参数；`grpcclient` 无 rerank 字段转发；TEST-39.2.2 含「reranker 不经 `grpcclient` 转发（env 驱动）」断言；`?rerank=true` per-request 记为 superseded `[SPEC-DEFER:phase-future.console-api-rerank-forward]`。stop-condition：若发现 `?rerank` 参数转发则移除（不交付 per-request）。
- **R3（中）`HybridScore` / `Reason` 映射遗漏（provenance 断在 Go 侧）**：若 `protoToSearchResult` 漏映射 `HybridScore` / 漏保留 `Reason`，对外 REST 看不到融合分 / rerank provenance。
  - **缓解**：`protoToSearchResult` 加 `HybridScore: p.HybridScore`（紧随 `VectorScore`）；`Reason: p.Reason` 既有映射（`:624`）保留；TEST-39.2.2 含「`HybridScore` / `Reason` 映射」断言。
- **R4（中）task-39.1 proto 字段未就位（`buf generate` 未跑）**：本 task 消费 `pb.SearchRequest.Hybrid` / `pb.SearchResultItem.HybridScore`，若 task-39.1 的 `buf generate` 未合入，Go 编译失败。
  - **缓解**：本 task dep task-39.1（proto 字段 + `buf generate` 须先合入）；39.1 完成后开工（§依赖关系）；编译失败即提示 task-39.1 未就位。
- **R5（低）`?hybrid` / `?semantic` 同设时语义歧义**：用户同设 `?hybrid=true&?semantic=true` 时 core 行为。
  - **缓解**：core 数据面 dispatch 优先级 `if req.hybrid {..} else if req.semantic {..}`（task-39.1，hybrid 优先）——同设时走 hybrid（融合含 vector 路）；本 task 只透传两 flag，优先级由 core 裁决；TEST-39.2.1 含「`?hybrid` / `?semantic` 独立透传」断言（不在 Go 侧改写优先级）。

## 9. Verification Plan

```bash
# 1. AC1 — handleSearch ?hybrid OR-merge（?hybrid=true / body hybrid / 皆无；?hybrid / ?semantic 独立）
go test ./internal/consoleapi/...

# 2. AC2 — grpcclient 转发 Hybrid + 映射 HybridScore / Reason（默认 false 字节等价；reranker 不经 grpcclient 转发）
go test ./internal/consoleapi/grpcclient/...

# 3. contractv1 add-only round-trip（Hybrid / HybridScore json；既有字段不受影响）
go test ./internal/contractv1/...

# 4. 不退化（全量 Go + 默认 hybrid=false 字节等价确认）
go test ./...
go vet ./...

# 5. AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.console-api-hybrid-forward-go-defer-note]：本 task 仅交付 (A) `contractv1.SearchRequest.Hybrid` + `SearchResult.HybridScore` add-only + (B) `handleSearch` `?hybrid` OR-merge + (C) `grpcclient.Search` 转发 `Hybrid` + `protoToSearchResult` 映射 `HybridScore` / `Reason`（对外 `POST /v1/search` 贯通 hybrid + rerank provenance 可见性 Go 半，🟢 可单测）；console_data_plane proto `hybrid=8` / `hybrid_score=17` + 数据面 hybrid dispatch 属 task-39.1（本 task 消费其字段 + `buf generate`）；`?rerank=true` per-request 转发 [SPEC-DEFER:phase-future.console-api-rerank-forward]（据 ADR-044 D3 重界定为 provenance 可见性、reranker 保持 env 驱动 superseded、不实现）、对外 REST 端到端 rerank `reason` 可见性 smoke 断言 [SPEC-OWNER:task-39.3-closeout]、Console UI hybrid explain 面板 [SPEC-OWNER:phase-future.console-semantic-explain]、大语料 hybrid 召回质量基准 [SPEC-DEFER:phase-future.vector-large-corpus-perf] 均不在本 task 范围。`?hybrid` 镜像 Phase 20 `?semantic` 已证范式（据实，非新机制，ADR-013）；真实 release tag / run-id / digest（v0.32.0）[SPEC-OWNER:task-39.3-closeout] 实施授权后回填。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done（v0.32.0；PR #253 合入 master @ a9cc6bc。§9 真实验证完成：`go test ./internal/consoleapi/...` TEST-39.2.1（`handleSearch` `?hybrid` OR-merge 5 case + `?hybrid`/`?semantic` 独立）PASS / `go test ./internal/consoleapi/grpcclient/...` TEST-39.2.2（`Search` 转发 `Hybrid` true+false + `protoToSearchResult` 携 `HybridScore` + rerank `Reason`）PASS / `go test ./internal/contractv1/...` `Hybrid`/`HybridScore` json round-trip + legacy-absent defaults false PASS / `go test ./...` + `go vet ./...` clean + gofmt clean / `bash scripts/spec_drift_lint.sh --touched origin/master` 0 命中；CI 14/14 绿）

- **§9 Verification 实证**（实施后回填）：本机真实跑 §9 全部命令、逐条粘 PASS 摘要。
- **实际改动文件**（实施后回填）：`internal/contractv1/contractv1.go`（add-only `Hybrid` + `HybridScore`）/ `internal/consoleapi/handlers.go`（`handleSearch` `?hybrid` OR-merge）/ `internal/consoleapi/grpcclient/grpcclient.go`（`Search` 转发 `Hybrid` + `protoToSearchResult` 映射 `HybridScore`）/ `internal/consoleapi/search_semantic_test.go` 或同包 test（TEST-39.2.1）/ `internal/consoleapi/grpcclient` 同包 test（TEST-39.2.2）。
- **0 新 dep / 默认行为不变**：不设 `?hybrid` / body `hybrid` = `Hybrid=false` = 转发 false = 数据面 semantic / BM25 = 响应字节等价（ADR-004 向后兼容）/ 既有契约不变（contractv1 / handleSearch / grpcclient 既有字段、转发、映射不变）/ reranker 保持 env 驱动（不加 `?rerank` 参数，ADR-043 D3 / ADR-044 D3）/ `?rerank=true` per-request superseded 据实记录（ADR-013）/ cross-repo add-only signal（`Hybrid` / `HybridScore` Console contractv1 镜像，ADR-014 D4）。
- **ADR**：本 task = ADR-044 §D2（Go console-api hybrid 转发）+ §D3（rerank-forward 重界定为 provenance 可见性）落点；ADR-025（`console-api-hybrid-forward` Go 半兑现）+ ADR-043（`console-api-rerank-forward` 重界定）Phase 39 add-only Amendment 落点在 task-39.3 closeout（非本 task body）。
- **复用既有范式**：`?semantic` 转发（task-20.1 / ADR-024）+ `VectorScore` cross-repo add-only provenance（task-32.3 / ADR-014 D4）。
