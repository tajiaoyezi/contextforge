# Phase 42 · chunk-source-type-filter

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase **把 chunk 检索的 `source_type` 过滤从「诚实 no-op 契约」落地为真实过滤**，并据 grounding 诚实校正 `agent_scope` 的归属。背景：`SearchFilters` 自 task-4.2 起就有 `source_type` / `agent_scope` 两字段，v1 search proto（`proto/contextforge/v1/search.proto:12-14` `SearchFilters{source_type, language}` + `RetrievalResult.source_type=3`）也早有契约，但 **Phase 32（task-32.3 / ADR-037）经核 chunks 表无该列、`SearchResult.source_type` 恒为 `DEFAULT_SOURCE_TYPE=""`、`agent_scope` 恒空，遂据实把二者定为「documented no-op」**（`core/src/retriever/mod.rs:321-336` + `TEST-32.3.2`），并开 `[SPEC-DEFER:phase-future.chunk-source-type-filter]` + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`。本 phase 兑现其中 **source_type 维度**。grounding 真实状态（决定方案）：(a) **source_type 可由 `file_path` 确定性派生**——`core/src/indexer/mod.rs:483` 已有纯函数 `lang_hint_from_path(path) -> &'static str`（扩展名 → 语言）的范式，source_type 是其**粗粒度桶**（code/doc/config/other），**故无须存储、无须 schema migration**（chunks/files/provenance 三表 §5.3 **保持 FROZEN**）。(b) **读路径已就绪**——v1 `server.rs:440-453` 已把 proto `filters.source_type` → `RetrieverFilters.source_type`（只是 retriever 当前 no-op）；console 数据面 `data_plane/search.rs:378` 已把 `source_file_type: h.source_type` 写入响应（只是 `h.source_type` 恒为 `""`）。故只要 retriever 真实派生 + 过滤，v1 gRPC / v1 REST body 路径**立即生效**，console 响应**立即显示**真实 source_type。(c) **agent_scope 是 memory 层概念**——`agent_scope` 真实归属 memory（`memory_items` migration 0013 / `MemoryListFilter` / `ListMemory` scope filter / `internal/consoleapi/memstore.go:629-635`）；chunks 无 agent 关联、无可派生维度。**关键诚实校正（ADR-013，本 phase 核心）**：`chunk-source-type-filter` 与 `chunk-agent-scope-filter` 经 grounding 并**不对称**——前者可派生、可真实落地（0 migration）；后者须 ingest-path schema 工程（为 chunks 引入 agent 维度）且价值不明，**本 phase 不伪造**，agent_scope 续为 documented no-op、`[SPEC-DEFER:phase-future.chunk-agent-scope-filter]` 据实保持（镜像 Phase 32/34/35 的 grounding 校正手法）。

> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md §3.24 + §4 backlog` → 源码锚点（`core/src/retriever/mod.rs:41-43`（`DEFAULT_SOURCE_TYPE=""` 等常量）+ `:132-142`（`SearchFilters{source_type, language, agent_scope, ...}`）+ `:144-166`（`SearchResult` 12-field，`source_type` v0.1 schema-gap default）+ `:314-336`（`search()` 入口 + task-32.3 no-op 块）+ `:364-388`（language post-filter 范式，**source_type 镜像源**）+ `:463-476`（BM25 `SearchResult` 构造，`source_type: DEFAULT_SOURCE_TYPE`）+ `:555-568`（`get_chunk` 构造）+ `:803-816`（`search_semantic` 构造）+ `:903-940`（`TEST-32.3.2` no-op 守护，本 phase 据真契约改写）/ `core/src/indexer/mod.rs:483-…`（`lang_hint_from_path` 纯函数派生范式 + `:115-147` SQL_SCHEMA §5.3 FROZEN）/ `core/src/server.rs:440-453`（v1 search filter mapping，proto `filters.source_type`/`agent_scope` → `RetrieverFilters` 已就绪）+ `:491-…`（`search_result_to_proto`，`source_type` 已映射）/ `core/src/data_plane/search.rs:337-342`（console BM25 分支 `SearchFilters::default()`）+ `:374-382`（`SearchResultItem.source_file_type: h.source_type` 响应已写）/ `proto/contextforge/v1/search.proto:11-15/35-46`（v1 `SearchFilters`/`RetrievalResult.source_type` 既有契约）/ `proto/contextforge/console_data_plane/v1/console_data_plane.proto:151-167`（console `SearchRequest`，字段 1-8 已用、**下一空号 9**）+ `:190-212`（`SearchResultItem.source_file_type=5` 响应侧已在）/ `internal/consoleapi/handlers.go`（`handleSearch` `?semantic`/`?hybrid` OR-merge forward 范式，**source_type 镜像源**）+ `internal/contractv1/contractv1.go:112-128`（`SearchRequest` add-only 字段范式）/ `internal/daemon/rest.go:135-150`（v1 REST `handleSearch` 解码 proto SearchRequest body，**filters.source_type 已可经 body 透传**）/ `internal/consoleapi/memstore.go:629-635`（agent_scope memory 层 filter 实证）） → AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5，**第三十三次**激活）→ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造红线：source_type 由 file_path 确定性派生非合成 / agent_scope 据实 honest-defer 不伪造为 chunk 维度 / 真实过滤行为实测非预填）。

> **ADR 影响面（已识别）**：
> - **ADR-047 chunk-source-type-filter（新，Proposed）**：记 source_type 由 file_path 确定性派生（`classify_source_type` 纯函数，0 migration，§5.3 FROZEN，D1）+ 真实过滤 + 三构造点 populate（v1 retriever `search()` BM25 post-filter 镜像 language，D2）+ console 数据面 source_type 请求侧 forward（proto add-only `source_type=9` + `data_plane` post-filter + Go `?source_type=` 转发，D3）+ agent_scope 据实 honest-defer（memory 层概念、chunks 无该维度，续 documented no-op + SPEC-DEFER，D4）。Status: Proposed（Draft 阶段不 ratify；ratify 在 task-42.3 closeout）。
> - 触及 **ADR-037（vector-backend-config-plumbing-and-completeness）**：其 task-32.3 把 source_type / agent_scope 定为 documented no-op + 开 `[SPEC-DEFER:phase-future.chunk-source-type-filter]` / `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`——本 phase 经 add-only Amendment 记其 **source_type no-op 被本 phase 真实过滤 supersede**（agent_scope no-op 据实保持），不溯改 ADR-037 正文（ADR-014 D5）。
> - 触及 **ADR-015（proto-evolution-add-only）→ 守线**：console_data_plane `SearchRequest` add-only `source_type=9`（既有字段 1-8 号冻结，默认空 → 不过滤，backward-compatible）；v1 search.proto `SearchFilters.source_type=1` 既有不动。
> - 触及 **ADR-016（contract-field-availability）/ ADR-024（console-api-semantic-forward）/ ADR-044（console-api-retrieval-signal-forward）→ 范式承接**：console `?source_type=` 请求侧 forward 镜像 `?semantic`（ADR-024）/ `?hybrid`（ADR-044）的 OR-merge + grpcclient 透传范式。
> - 触及 **ADR-004（默认行为 + 既有契约不变）→ 守线（byte-equiv）**：空 source_type filter → 结果与既有 byte-for-byte 一致（过滤仅在非空时生效）；source_type **value** 由空串变真实派生值是「填补 v0.1 schema gap」（task-4.2 §2A 早记 `source_type` 为 schema-gap default ""，本 phase 据 file_path 真实填充），非破坏性默认变更——但属可观测响应字段变化，由 ADR-047 据实记。
> - 触及 **ADR-008（dep add-only）→ 守线**：本 phase = **0 新依赖**（`classify_source_type` 纯 std；proto add-only 无新 dep）+ 0 网络 + 0 schema migration。

## 1. 阶段目标

v0.34.0 ship 后，ContextForge 把 chunk 检索的 `source_type` 过滤从 Phase 32 据实记录的「documented no-op 契约」落地为**真实过滤**，让用户可按来源类型（code / doc / config / other）筛选检索结果，并据 grounding 诚实校正 `agent_scope` 的归属（memory 层、非 chunk 维度，续 honest-defer）。具体：(1) `core/src/retriever/mod.rs` 加 `classify_source_type(file_path) -> &'static str` 纯函数（镜像 `indexer::lang_hint_from_path`，扩展名 → {code, doc, config, other} 确定性桶）；三处 `SearchResult` 构造点（`search()` BM25 / `get_chunk` / `search_semantic`）把 `source_type` 由 `DEFAULT_SOURCE_TYPE=""` 改为 `classify_source_type(&file_path)` 真实派生值；`search()` 加 source_type post-filter（`!filters.source_type.is_empty()` 时仅留 `classify_source_type(file_path) ∈ filters.source_type` 的 hit，镜像 language post-filter）；`agent_scope` 续 documented no-op（窄化既有 WARN 块仅覆盖 agent_scope）。**0 schema migration**（source_type 由 file_path 派生、§5.3 chunks/files/provenance 保持 FROZEN）；v1 gRPC（`server.rs:440-453` 已映射）+ v1 REST body 路径立即生效。(2) console 数据面请求侧 forward：`console_data_plane.proto` `SearchRequest` add-only `repeated string source_type = 9`（既有字段号冻结）+ `data_plane/search.rs` 按 `req.source_type` 对已 populate 的 hit 做 post-filter（覆盖 BM25/semantic/hybrid 一致）+ Go `internal/consoleapi` `handleSearch` 解析 `?source_type=` 查询参数（+ body 字段）→ `contractv1.SearchRequest` add-only `SourceType []string` → grpcclient 透传到 console_data_plane（镜像 `?semantic`/`?hybrid` forward 范式）。(3) **诚实校正（ADR-013）**：`agent_scope` 经 grounding 为 memory 层概念（`memory_items` 0013 / `ListMemory` scope）、chunks 无 agent 关联，**本 phase 不伪造** chunk-level agent_scope filter，续 documented no-op + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`（须 ingest-path schema 工程、价值不明）。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. **source_type 派生 + 真实过滤 + populate（🟢）**：`core/src/retriever/mod.rs` add `classify_source_type(file_path) -> &'static str`（扩展名确定性桶 code/doc/config/other，镜像 `lang_hint_from_path`）；三处构造点 `source_type` 由 `DEFAULT_SOURCE_TYPE` 改真实派生；`search()` BM25 加 source_type post-filter（镜像 language post-filter，空 filter → byte-equiv）；`agent_scope` 续 documented no-op（WARN 窄化仅 agent_scope）；0 schema migration（§5.3 FROZEN）；v1 path（`server.rs:440-453`）立即生效（AC1）
2. **console-api source_type 请求侧 forward（🟢）**：`console_data_plane.proto` `SearchRequest` add-only `repeated string source_type = 9`（既有字段 1-8 冻结，buf generate）+ `data_plane/search.rs` 按 `req.source_type` 对 populate 后的 hit post-filter（空 → 不过滤 byte-equiv）+ Go `handleSearch` 解析 `?source_type=`（+ body）→ `contractv1.SearchRequest` add-only `SourceType []string` → grpcclient 透转；console 响应 `source_file_type` 显示真实派生值（AC2）
3. **agent_scope honest-defer + 0-dep/0-migration 守线 + v0.35.0 closeout**：`agent_scope` 据 grounding 为 memory 层概念续 documented no-op + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`（不伪造，ADR-013）；0 新依赖（`classify_source_type` 纯 std）+ 0 网络 + 0 schema migration（§5.3 FROZEN）；空 filter byte-equiv（ADR-004）；v0.35.0 release docs + `scripts/console_smoke.sh` v32[51/51] + ADR-047 据真实测试 ratify + ADR-037 add-only Amendment（source_type no-op superseded、agent_scope no-op reaffirmed）+ roadmap §3.24/§4 add-only + phase §6 闭合（AC3）
4. ADR-014 D1-D5（**第三十三次**激活）全通过（AC4）

**v0.x 版本号决策**：v0.35.0（Phase 42，承 v0.34.0；roadmap §1.1 Phase N→v0.(N-7).0），theme chunk-source-type-filter。minor release（兑现 Phase 32 据实延后的 chunk source_type 真实过滤——0 schema migration（source_type 由 file_path 派生、§5.3 FROZEN）+ proto add-only console forward + 空 filter byte-equiv；agent_scope 据 grounding honest-defer 续 no-op；默认构建 0 新依赖（ADR-008）+ 0 网络）。

## 2. 业务价值

把 v1 / console 检索 API 早有契约但 Phase 32 据实记为 no-op 的 `source_type` 过滤落地为真实能力，让用户可按来源类型筛选检索结果，**关闭一个诚实缺口**（API 一直收 `source_type` 参数却忽略它），且空 filter byte-equiv、0 schema migration：

### 42.1 chunk source_type 派生 + 真实过滤 + populate（chunk-source-type-derivation-and-filter，🟢）

- grounding 真实状态：`SearchFilters.source_type: Vec<String>`（`retriever/mod.rs:137`）+ v1 proto `SearchFilters.source_type=1`（`search.proto:13`）+ `RetrievalResult.source_type=3`（`search.proto:38`）自 task-4.2 / task-6.1 起就有契约；`server.rs:440-453` 已把 proto `filters.source_type` → `RetrieverFilters.source_type`。但 retriever `search()` 在 `:321-336` 把 source_type / agent_scope 据实记为 documented no-op（chunks 表无该列、`SearchResult.source_type` 恒 `DEFAULT_SOURCE_TYPE=""`），`TEST-32.3.2` 守护该 no-op 契约（Phase 32 / ADR-037）。
- 本 phase 加 `classify_source_type(file_path: &str) -> &'static str` 纯函数（镜像 `indexer/mod.rs:483 lang_hint_from_path`：`match path.extension().to_ascii_lowercase()` → 确定性桶）：`code`（rs/go/py/js/ts/jsx/tsx/java/c/h/cpp/hpp/cc/cs/rb/php/swift/kt/scala/sh/sql/… 源码扩展名）/ `doc`（md/markdown/mdx/txt/rst/adoc/org/tex）/ `config`（toml/yaml/yml/json/ini/cfg/conf/env/xml/properties）/ `other`（其余 / 无扩展名）。**确定性、纯 std、0-dep**——具体扩展名表在 task-42.1 spec §2 固化、单测穷举。
- 三处 `SearchResult` 构造点（`:463-476` BM25 `search()` / `:555-568` `get_chunk` / `:803-816` `search_semantic`）`source_type: DEFAULT_SOURCE_TYPE.to_string()` → `source_type: classify_source_type(&file_path).to_string()`——source_type **value** 在所有路径真实可见（填补 task-4.2 §2A 记录的 v0.1 schema gap）。
- `search()`（BM25）加 source_type post-filter：`if !opts.filters.source_type.is_empty() && !opts.filters.source_type.iter().any(|s| s == derived_source_type) { continue; }`（镜像 `:386` language post-filter；空 source_type filter → 不过滤 → byte-equiv）。
- `agent_scope` 续 **documented no-op**：窄化 `:321-336` 既有 no-op 块——仅 `!opts.filters.agent_scope.is_empty()` 时 stderr note「agent_scope 是 memory 层 filter、非 chunk 检索维度」（source_type 不再 no-op）。
- **0 schema migration**：source_type 由 file_path 在 query 时确定性派生（与 `language` 同源信号），chunks/files/provenance 三表 §5.3 **保持 FROZEN**——比加列 + importer 打标 + 既有 chunk backfill 更 surgical 且等价正确（确定性派生 == 存储值）。
- **真实行为（ADR-013，不预填）**：source_type 过滤的真实命中行为（混合扩展名 fixture：`?source_type=doc` 仅返 .md / `?source_type=code` 仅返 .rs / 空 filter 返全部）由 task-42.1 单测 + task-42.3 smoke 实测断言、真实跑出后记录（非预填）。

### 42.2 console-api source_type 请求侧 forward（console-api-source-type-forward，🟢）

- grounding 真实状态：console 数据面 `SearchResultItem.source_file_type=5`（`console_data_plane.proto:195`）+ `data_plane/search.rs:378 source_file_type: h.source_type.clone()` 响应侧**已就绪**（task-42.1 populate 后立即显示真实值）；但 console `SearchRequest`（`:151-167`）**无 source_type 字段**（仅 `agent_scope=3`），且 `data_plane/search.rs:337-342` BM25 分支用 `SearchFilters::default()`——故 console **请求侧无法传 source_type filter**。
- 本 phase add-only `console_data_plane.proto` `SearchRequest` `repeated string source_type = 9`（既有字段 1-8 号冻结，ADR-015 add-only，buf generate）+ `data_plane/search.rs` 按 `req.source_type` 对 populate 后的 hit 做 post-filter（在 BM25 / semantic / hybrid 三分支汇总后统一 post-filter，利用 task-42.1 已 populate 的 `h.source_type`；空 source_type → 不过滤 byte-equiv）。
- Go `internal/consoleapi` `handleSearch`：解析 `?source_type=`（repeated query param + body `source_type` 字段，并集合并，镜像 `?semantic`/`?hybrid` OR-merge）→ `contractv1.SearchRequest` add-only `SourceType []string`（json `source_type`）→ grpcclient 映射到 console_data_plane `SearchRequest.source_type`。
- **设计定性**：与 `?semantic`（ADR-024）/ `?hybrid`（ADR-044）请求侧 forward 同构（query param + body 并集 → grpcclient 透传）；source_type 是 repeated（非 bool），合并语义为并集（任一来源给的 source_type 都纳入）；空 → 不过滤（backward-compatible）。

**不在本 phase 范围**：

- chunk-level `agent_scope` 真实过滤（agent_scope 经 grounding 为 memory 层概念 `memory_items` 0013 / `ListMemory` scope，chunks 无 agent 关联、无可派生维度；真实落地须 ingest-path schema 工程为 chunks 引入 agent 维度且价值不明，本 phase 不伪造）[SPEC-DEFER:phase-future.chunk-agent-scope-filter]
- 用户自定义 / importer 显式打标的 source_type（本 phase 由 file_path 确定性派生粗粒度桶；importer 侧显式 source_type 打标 + chunks schema 列须解冻 §5.3）[SPEC-DEFER:phase-future.chunk-importer-source-type-tagging]
- v1 / console **semantic 路径**的 retriever-内 source_type 过滤（v1 `search()` BM25 路径内过滤镜像 language 当前 scope；console 经 data_plane post-filter 已覆盖 semantic/hybrid；v1 semantic 路径若需 retriever-内过滤则另行）[SPEC-DEFER:phase-future.semantic-path-source-type-filter]
- 其余治理 / 检索 marker 据实保持延后（`vector-dim-feature-enforce` 须 feature build / `tracestore-multi-workspace-strict` 须 console e2e）

## 3. 涉及模块

### 42.1 chunk-source-type-derivation-and-filter（task-42.1）

- 修改 `core/src/retriever/mod.rs`——add `pub(crate) fn classify_source_type(file_path: &str) -> &'static str`（扩展名确定性桶 code/doc/config/other，镜像 `indexer::lang_hint_from_path`）+ 三处构造点（`:466` BM25 / `:558` get_chunk / `:806` search_semantic）`source_type: DEFAULT_SOURCE_TYPE.to_string()` → `classify_source_type(&file_path).to_string()` + `search()` BM25 加 source_type post-filter（镜像 `:386` language）+ 窄化 `:321-336` no-op 块仅覆盖 agent_scope
- **不改** chunks/files/provenance SQL_SCHEMA（`indexer/mod.rs:115-147` §5.3 FROZEN，source_type 由 file_path 派生不存储）/ `DEFAULT_SOURCE_TYPE` 常量（`:42`，库其他消费方 / 历史语义保留）/ v1 `server.rs:440-453` filter mapping（已就绪）
- **据真契约改写** `TEST-32.3.2`（`:903-940`，原断言 source_type + agent_scope 双 no-op）→ 拆为 agent_scope-only no-op 守护 + source_type 真实过滤断言（ADR-037 source_type no-op 被 supersede，agent_scope no-op 保持；非溯改闭合 spec，而是当前测试码随契约演进，记入 ADR-047 / ADR-037 Amendment）
- 同源验证（≥2，🟢）：`classify_source_type` 扩展名 → 桶穷举矩阵（code/doc/config/other + 无扩展名 + 大小写）（TEST-42.1.1）+ 真实过滤行为（混合扩展名 fixture：`source_type=[doc]` 仅返 doc / `[code]` 仅返 code / 空 filter byte-equiv 返全部 + `source_type` value 三路径 populate 非空 + agent_scope 仍 no-op）（TEST-42.1.2）

### 42.2 console-api-source-type-forward（task-42.2）

- 修改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`——`SearchRequest` add-only `repeated string source_type = 9`（既有字段 1-8 冻结，注释记 ADR-015 add-only + 空 → 不过滤 backward-compat）+ `buf generate`（Rust prost + Go pb.go）
- 修改 `core/src/data_plane/search.rs`——按 `req.source_type` 对 BM25/semantic/hybrid 三分支汇总后的 hit 做 post-filter（利用 task-42.1 已 populate 的 `h.source_type`；空 → 不过滤 byte-equiv）
- 修改 `internal/contractv1/contractv1.go`——`SearchRequest` add-only `SourceType []string` json `source_type`（镜像 Semantic/Hybrid add-only 字段）
- 修改 `internal/consoleapi/handlers.go`——`handleSearch` 解析 `?source_type=`（repeated query param）+ body `source_type` 并集合并 → `SearchRequest.SourceType`（镜像 `?semantic`/`?hybrid` OR-merge）
- 修改 `internal/consoleapi/grpcclient*.go`（或对应 client 映射点）——`SearchRequest.SourceType` → console_data_plane `SearchRequest.source_type`
- 同源验证（≥2，🟢）：proto `SearchRequest.source_type` round-trip + prost wire-tag 字段号 9 in-crate 断言（TEST-42.2.1）+ `handleSearch` `?source_type=` 解析 + body 并集 → 转发到下游 SearchClient（capturingSearch 断言，镜像 `TestTask201/392`）+ `data_plane` source_type post-filter（TEST-42.2.2）

### 42.3 closeout（task-42.3）

- 修改 `scripts/console_smoke.sh`——banner v31→v32 + v32 changelog block + 新 step [51/51]（REAL 模式：索引含 .rs + .md 混合 fixture、`POST /v1/search?source_type=doc` → 仅返 .md chunk（`source_file_type="doc"`）、`?source_type=code` → 仅返 .rs chunk，证真实过滤；不可达则 doc/status；current Phase 41 [50/50] → Phase 42 顺位 [51/51]）
- 修改 `internal/cli/smoke_syntax_test.go`——新 `TestTask423`（镜像 `TestTask413`）断言 [51/51] + markers（chunk-source-type-filter / source_type / classify / TEST-42.1. / TEST-42.2.）+ no-regression（denominators [37/37]..[50/50] 不溯改，ADR-014 D5）
- 新增 `docs/releases/v0.35.0-evidence.md` + `v0.35.0-artifacts.md`（tag SHA / run id / digest 为 angle-bracket backfill marker）+ `README.md` v0.35 段 + `RELEASE_NOTES.md` v0.35.0 段（含「source_type 过滤落地 + `?source_type=` REST + 空 filter byte-equiv + agent_scope 续 memory 层 no-op」段）
- 修改 `docs/decisions/adr-047-chunk-source-type-filter.md`——Status Proposed→Accepted（逐 D 如实）+ 新 `## Ratification（v0.35.0 / task-42.3）`
- add-only Amendment（非正文改，ADR-014 D5）：`adr-037`（vector-backend-config-plumbing-and-completeness，source_type no-op 被本 phase 真实过滤 supersede / agent_scope no-op 据实保持）；`docs/roadmap.md §3.24/§4` add-only（Phase 42 行 + chunk-source-type-filter fulfilled + chunk-agent-scope-filter 续延后 + 新 backlog 条目）
- 修改 `docs/specs/phases/phase-42-chunk-source-type-filter.md`——Status Draft→Done + §6 AC 勾选（逐维如实）
- 修改 `docs/s2v-adapter.md`——Phase 42 行 + Task 行 + ADR-047 行 + BDD 行

### BDD feature

- 新增 `test/features/phase-42-chunk-source-type-filter.feature`（≥3 scenario：source_type 真实过滤（`source_type=[doc]` 仅返 doc / `[code]` 仅返 code / 空 filter byte-equiv）+ source_type value 三路径 populate / console `?source_type=` 请求侧 forward（query param + body 并集 → 下游） / v0.35.0 收口 + agent_scope honest-defer + 0-dep/0-migration 守线）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 42.1 | `core/src/retriever/mod.rs` add `classify_source_type(file_path)` 纯函数（扩展名 → code/doc/config/other 确定性桶）+ 三构造点 populate 真实 source_type + `search()` BM25 source_type post-filter（镜像 language）+ agent_scope 续 no-op；0 schema migration（§5.3 FROZEN）；据真契约改写 TEST-32.3.2 | `../tasks/task-42.1-chunk-source-type-derivation-and-filter.md` |
| 42.2 | `console_data_plane.proto` `SearchRequest` add-only `source_type=9` + `data_plane/search.rs` post-filter + Go `contractv1.SearchRequest.SourceType` + `handleSearch ?source_type=` forward + grpcclient（镜像 `?semantic`/`?hybrid`） | `../tasks/task-42.2-console-api-source-type-forward.md` |
| 42.3 | smoke v32[51/51] + v0.35.0 closeout + ADR-047 ratify + ADR-037 add-only Amendment + roadmap §3.24/§4 add-only + s2v-adapter add-only | `../tasks/task-42.3-closeout-v0.35.0.md` |

## 5. 依赖关系

- **task-42.1**（chunk-source-type-derivation-and-filter）dep 既有 `core/src/retriever/mod.rs` `SearchFilters.source_type`（:137 已在）+ `SearchResult.source_type`（:156 已在）+ language post-filter 范式（:386 已在）+ `core/src/indexer/mod.rs lang_hint_from_path`（:483 派生范式已在）+ `server.rs:440-453` v1 filter mapping（已就绪）；可独立先行（不依赖 42.2）。
- **task-42.2**（console-api-source-type-forward）dep task-42.1 的 `classify_source_type` + populate（console post-filter 用 populate 后的 `h.source_type`）+ 既有 `console_data_plane.proto` `SearchRequest`（字段 1-8 已用，下一空号 9）+ `data_plane/search.rs`（:337-382 已在）+ Go `handleSearch` `?semantic`/`?hybrid` forward 范式（已在）+ `internal/contractv1` add-only 字段范式（已在）+ grpcclient 映射点（已在）；dep 42.1 populate 先行。
- **task-42.3**（closeout）dep 42.1 + 42.2 全 Done；release docs / smoke v32[51/51] / ADR-047 ratify 据两 task 真实测试 / 实测行为。
- 外部：ADR-047（本 phase 新 Proposed）/ ADR-037（vector-backend-config-plumbing-and-completeness，source_type no-op supersede / agent_scope no-op 保持 add-only Amendment）/ ADR-015（proto add-only，console SearchRequest source_type=9）/ ADR-024 / ADR-044（console 请求侧 forward 范式承接）/ ADR-004（空 filter byte-equiv，source_type value 填补 v0.1 schema gap 由 ADR-047 据实记）/ ADR-008（dep add-only，Phase 42 = 0 新依赖）/ ADR-012（tag/release outward-facing 须用户显式授权）/ ADR-014 **第三十三次**激活 / ADR-013（禁伪造红线，source_type 由 file_path 确定性派生非合成、agent_scope 据实 honest-defer 不伪造为 chunk 维度、真实过滤行为实测非预填）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（每条 AC 含 ADR-014 D3 verified by 显式 owner；Draft 阶段未勾选，实施后逐条置 `[x]`）**：

- [x] **AC1**（source_type 派生 + 真实过滤 + populate 🟢）: `core/src/retriever/mod.rs` add `classify_source_type(file_path) -> &'static str`（扩展名确定性桶 code/doc/config/other，镜像 `lang_hint_from_path`）+ 三构造点（`:466`/`:558`/`:806`）`source_type` 由 `DEFAULT_SOURCE_TYPE` 改真实派生 + `search()` BM25 加 source_type post-filter（镜像 `:386` language，空 filter byte-equiv）+ `agent_scope` 续 documented no-op（WARN 窄化仅 agent_scope）；0 schema migration（chunks/files/provenance §5.3 FROZEN）；v1 path（`server.rs:440-453`）立即生效 — verified by **TEST-42.1.1**（`classify_source_type` 扩展名 → 桶矩阵穷举）+ **TEST-42.1.2**（真实过滤 source_type=[doc]/[code] + 空 filter byte-equiv + source_type value 三路径 populate + agent_scope 仍 no-op）+ phase-smoke step 1
- [x] **AC2**（console-api source_type 请求侧 forward 🟢）: `console_data_plane.proto` `SearchRequest` add-only `repeated string source_type = 9`（既有字段 1-8 冻结，buf generate）+ `data_plane/search.rs` 按 `req.source_type` post-filter（空 → 不过滤 byte-equiv）+ `internal/contractv1.SearchRequest` add-only `SourceType []string` + `handleSearch` `?source_type=`（query + body 并集）→ grpcclient → console_data_plane；console 响应 `source_file_type` 显示真实派生值 — verified by **TEST-42.2.1**（proto `source_type` round-trip + prost wire-tag 字段号 9 in-crate）+ **TEST-42.2.2**（`handleSearch ?source_type=` 解析 + body 并集 → 下游 SearchClient + `data_plane` post-filter）+ phase-smoke step 2
- [x] **AC3**（agent_scope honest-defer + 0-dep/0-migration 守线 + v0.35.0 closeout）: `agent_scope` 据 grounding 为 memory 层概念（`memory_items` 0013 / `ListMemory` scope）续 documented no-op + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`（不伪造，ADR-013）；0 新依赖（`classify_source_type` 纯 std）+ 0 网络 + 0 schema migration（§5.3 FROZEN）；空 source_type filter byte-equiv（ADR-004）；honest-defer：`chunk-agent-scope-filter`（须 ingest-path schema）/ `chunk-importer-source-type-tagging`（须 §5.3 解冻）/ `semantic-path-source-type-filter` 据实保持延后；v0.35.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ `scripts/console_smoke.sh` v32[51/51] + `internal/cli/smoke_syntax_test.go` `TestTask423`（no-regression [37/37]..[50/50]）+ ADR-047 据真实测试 ratify + ADR-037 add-only Amendment + roadmap §3.24/§4 add-only + phase §6 闭合 — verified by **TEST-42.3.1**（smoke v32[51/51] + smoke_syntax_test + ADR-047 ratify + roadmap/adapter add-only + phase §6 闭合）
- [x] **AC4**（ADR-014 cross-validation gate）: ADR-014 D1-D5（**第三十三次**激活）全通过 — D1 mapping + D2 lint `bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-41 不溯改（ADR 改动 add-only Amendment）— verified by task-42.3 closeout PR body + 各 task LAST TEST（TEST-42.1.3 / TEST-42.2.3 / TEST-42.3.2）

**端到端 smoke（C1 集成兜底）**：(1) 索引 .rs + .md 混合 fixture → `POST /v1/search?source_type=doc` 仅返 .md chunk（`source_file_type="doc"`）/ `?source_type=code` 仅返 .rs / 空 filter 返全部（byte-equiv）全 PASS（真实过滤据实标注）；(2) console `?source_type=` 请求侧 forward（query param + body 并集 → 下游 + 响应 source_file_type 真实值）全 PASS；(3) v0.35.0 收口 + agent_scope honest-defer（memory 层据实标注）+ 0-dep/0-migration 守线全 PASS。

## 7. 阶段级风险

- **R1（高）source_type value 由空串变真实值破既有契约 / 客户端**：三构造点 `source_type` 由 `DEFAULT_SOURCE_TYPE=""` 改真实派生值是可观测响应字段变化，若客户端依赖空串则可能受影响。
  - **缓解**：task-4.2 §2A 早把 `source_type` 记为「v0.1 schema gap default ""」、契约本意是真实 source_type（非永久空），本 phase 据 file_path 填补该 gap（非新增破坏性字段）；空 source_type filter → 过滤行为 byte-equiv（仅 value 字段由 "" 变派生值）；ADR-047 D1 据实记该 value 变化。stop-condition：若把 value 变化夸大为 byte-equiv 或未据实记则越界（ADR-013）。
- **R2（高）agent_scope 被伪造为 chunk 维度**：若为凑齐「source_type + agent_scope」而强行给 chunks 加 agent_scope 派生 / 假过滤，则违 ADR-013。
  - **缓解**：grounding 据实——agent_scope 是 memory 层概念（`memory_items` 0013 / `ListMemory` scope / `memstore.go:629-635`），chunks 无 agent 关联、无可派生维度；本 phase **不伪造**，agent_scope 续 documented no-op + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`（须 ingest-path schema 工程）；spec §1/§2 + ADR-047 D4 据实定性。stop-condition：若实现 chunk-level agent_scope 假过滤则 AC3 不标 `[x]`（ADR-013）。
- **R3（中）改 §5.3 FROZEN schema / 加 source_type 列**：若为存 source_type 而解冻 §5.3 加列 + importer 打标 + 既有 chunk backfill，则越 surgical 边界且引入 migration 风险。
  - **缓解**：source_type 由 file_path 确定性派生（query 时计算，与 `language` 同源信号）——确定性派生 == 存储值，0 schema migration、§5.3 chunks/files/provenance 保持 FROZEN；importer 显式 source_type 打标续 `[SPEC-DEFER:phase-future.chunk-importer-source-type-tagging]`。stop-condition：若本 phase 加 chunks 列 / migration 则越界（与 simplicity-first 冲突）。
- **R4（中）source_type post-filter 破空 filter byte-equiv**：若过滤逻辑在 source_type 为空时仍生效则破 backward-compat。
  - **缓解**：post-filter 仅 `!filters.source_type.is_empty()` 时生效（镜像 language post-filter `want_lang` 守护）；空 source_type → 不过滤 → 结果 byte-equiv；TEST-42.1.2 / TEST-42.2.2 断言空 filter byte-equiv。stop-condition：空 filter 改变结果则 AC1/AC2 不标 `[x]`。
- **R5（中）console proto 字段号冲突 / 非 add-only**：console_data_plane `SearchRequest` 加 source_type 若复用既有字段号或改既有字段则破 wire 兼容（ADR-015）。
  - **缓解**：既有字段 1-8 号冻结，source_type 取**下一空号 9**（add-only）；TEST-42.2.1 prost wire-tag in-crate 断言字段号 9；空 → 不过滤 backward-compat。stop-condition：复用 / 改既有字段号则 AC2 不标 `[x]`（ADR-015）。

## 8. Definition of Done

- 3 task spec（42.1-42.3）顶部 `**Status**: Done`
- §6 阶段级 AC1-4 全 `[x]`（受阻 / 延后态按 ADR-013 如实记录，不伪造——如 agent_scope 据实延后 `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`、importer 打标据实延后）
- 端到端 smoke 3 step 全 PASS（含受阻 / 延后态如实标注）
- **ADR**：ADR-047 `Proposed → Accepted`（据真实测试 / 实测行为逐 D 项 ratify）；ADR-037 经 add-only Amendment 记录（source_type no-op 被本 phase 真实过滤 supersede / agent_scope no-op 据实保持，不溯改正文，ADR-014 D5）；ADR-015（proto add-only）/ ADR-024 / ADR-044（console 请求侧 forward 范式）/ ADR-004（空 filter byte-equiv + source_type value 填补 v0.1 schema gap）/ ADR-008（0 新依赖）守线引用；`docs/roadmap.md §3.24/§4` add-only（Phase 42 行 + 新 backlog 条目）
- **adapter**：§Phase 索引 Phase 42 `Draft → Done` + `Tasks 0 → 3`；§ADR 索引 ADR-047；§BDD 追加 phase-42 feature 行；ADR-037 Amendment 记录
- **release**：`docs/releases/v0.35.0-{evidence,artifacts}.md` + `RELEASE_NOTES.md` v0.35 段 + README v0.35 段
- **smoke**：`scripts/console_smoke.sh` v32[51/51]（source_type 真实过滤端到端 + 既有 step 不退化，denominators [37/37]..[50/50] 不溯改）+ `internal/cli/smoke_syntax_test.go` `TestTask423` markers 同步
- **follow-up**：chunk-level agent_scope 过滤 `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]` + importer source_type 打标 `[SPEC-DEFER:phase-future.chunk-importer-source-type-tagging]` + semantic 路径 source_type 过滤 `[SPEC-DEFER:phase-future.semantic-path-source-type-filter]` 留 backlog
