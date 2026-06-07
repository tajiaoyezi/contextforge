# Task `32.3`: `console-provenance-and-retrieval-filter-honesty — console_data_plane SearchResultItem add-only vector_score=16（parity v1 search proto vector_score=13）携带 provenance + retrieval-filter 契约诚实化（mod.rs:325 误导性 WARN → 准确 no-op 契约 + 新 SPEC-DEFER backlog）`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 32 (vector-backend-config-plumbing-and-completeness)
**Dependencies**: 既有 `proto/contextforge/v1/search.proto`（`RetrievalResult.vector_score = 13` + `embedding_provider = 14`，task-19.3 add-only，Phase 19 已交付）/ `proto/contextforge/console_data_plane/v1/console_data_plane.proto`（`SearchResultItem` `:185-201`，字段至 `citation = 15`）/ `internal/consoleapi/grpcclient/grpcclient.go`（`protoToSearchResult` `:609` 现仅映射 `Score` `:620` + `RetrievalMethod` `:622`）/ `core/src/retriever/mod.rs`（`SearchFilters` `:135` + filter WARN `:325` + `SearchResult` `:152` + `source_type`/`agent_scope` hardcoded `:452`/`:459`）/ `core/src/indexer/mod.rs:117`（chunks 表 FROZEN §5.3，无 source_type/agent_scope 列）/ `core/migrations/0013_memory_items.sql`（agent_scope 属 memory 层）/ ADR-037（vector-backend-config-plumbing-and-completeness，D3 console provenance add-only + retrieval-filter 契约诚实化）/ ADR-004（local-first-privacy-baseline，默认行为 + proto + 既有契约不变）/ ADR-013（禁伪造红线——受阻 / 无驱动维度如实记录不伪造）/ ADR-014 D1-D5（第二十三次激活）

## 1. Background

Phase 32 第三块 grounded 缺口聚焦「控制面 provenance 携带」与「检索 filter 契约诚实化」两项——前者补齐数据面/控制面对 vector provenance 的 add-only 透出，后者把一处误导性运行时 WARN 校正为准确契约 + 新 backlog，二者均守 ADR-004 默认行为不变：

- **A：console_data_plane SearchResultItem 无 vector_score（provenance 不可直接携带）**：v1 search proto（`proto/contextforge/v1/search.proto`）的 `RetrievalResult` 自 task-19.3 起携带 `vector_score = 13`（语义相似度，BM25 命中为 0）+ `embedding_provider = 14`（`:33-49`），是 hit 的 retrieval provenance 单源。但 **CONSOLE 数据面 proto** 的 `SearchResultItem`（`console_data_plane.proto:185-201`）只有 `retrieval_method = 13`，**无 `vector_score`**——字段排到 `citation = 15`。控制面 client `internal/consoleapi/grpcclient/grpcclient.go` 的 `protoToSearchResult`（`:609`）只能映射 `Score`（`:620`）+ `RetrievalMethod`（`:622`），向量 provenance 现仅能经 `score` + `retrieval_method` 二者间接推断，无法据实透出每条 hit 的真实 `vector_score`。
- **B：retrieval-filter 一处误导性运行时 WARN**：`core/src/retriever/mod.rs:325` 在 caller 传非空 `source_type` / `agent_scope` filter 时 `eprintln!` 一条 WARN，原文措辞为「source_type/agent_scope filter `not yet implemented`（schema gap; SPEC-DRIFT-task-2.4 pending）, value ignored」（`:326-329`）[SPEC-DEFER:phase-future.chunk-source-type-filter]。该措辞暗示「只待 reverse-fill schema 即可落地」的确定性小项，与真实工程现状不符。

经核 grounded 现状（B 项的诚实校正）：chunks 表（`core/src/indexer/mod.rs:117`，§5.3 FROZEN）**无** `source_type` / `agent_scope` 列——其列仅为 `chunk_id` / `file_path` / `line_start` / `line_end` / `language` / `content` / `content_hash` / `kind` / `collection_id` / `indexed_at`（`:118-127`），其中 `kind` 是 AST 结构性 `Option<String>`（`:125`），**非** source 分类器；`SearchResult.source_type` 在热路径被硬编码为 `DEFAULT_SOURCE_TYPE`（`mod.rs:452`），`agent_scope` 硬编码 `Vec::new()`（`mod.rs:459`）；`agent_scope` 本质属 **memory 层** 概念（`core/migrations/0013_memory_items.sql:7` 的 `memory_items.agent_scope` 列 + `:19` 索引）。故对 chunk 检索而言这两个 filter 是**明确 no-op**；要让它们真实生效，须 importer 侧 source_type tagging（导入路径打标）+ chunks 表 schema 迁移——这是一项真实 import-path feature，**非**确定性 nit。本 task **不实现** 该 feature，仅把 WARN / 契约改诚实（准确 no-op 描述 + 默认空 filter 结果完全一致），并就该真实 feature 开新 backlog（honest-defer，不伪造已实现）。

本 task 范围内 A（console proto add-only field + 控制面 wiring）为 code-local 🟢 可单测，B（filter 契约诚实化 + 默认空 filter 结果一致 + 新 SPEC-DEFER backlog）为 code-local 🟢 可单测。

## 2. Goal

(1) **A**：为 console_data_plane proto 的 `SearchResultItem`（`:185-201`）add-only 新增 `vector_score = 16`（与 v1 search proto `RetrievalResult.vector_score = 13` parity；语义相似度，BM25 命中为 0），并在数据面 → 控制面 wiring（`grpcclient.go::protoToSearchResult` `:609`）携带该 provenance（不再仅经 `score` + `retrieval_method` 推断）；既有 client 不破（add-only proto field，ADR-004）。(2) **B**：把 `core/src/retriever/mod.rs:325` 的误导性 WARN 改为准确契约——明确陈述 chunks 表（FROZEN §5.3）无 source_type/agent_scope 列、`SearchResult` 二者为 hardcoded 常量、`agent_scope` 属 memory 层（0013），故对 chunk 检索为明确 no-op；real chunk filter 须 importer 侧 source_type tagging + schema 迁移 → 开新 backlog [SPEC-DEFER:phase-future.chunk-source-type-filter] + [SPEC-DEFER:phase-future.chunk-agent-scope-filter]；默认空 filter 结果与改前完全一致（默认行为不变）。

pass bar：A 经 proto add-only 编译 + 控制面 wiring 单测验证（🟢，`vector_score` 透出真实值 / BM25 命中为 0）；B 经契约诚实化单测验证（🟢，默认空 filter 结果与改前一致 + 非空 source_type/agent_scope filter 为准确 no-op）；默认行为 / proto 既有字段 / 既有契约不变（ADR-004）；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`——`SearchResultItem`（`:185-201`，现至 `citation = 15`）add-only 新增 `vector_score = 16`（float，语义 parity v1 search proto `RetrievalResult.vector_score = 13`）；既有字段编号 / 类型一律不动（ADR-004 既有契约不变）。`buf generate` 重生成对应 Go binding。
- 改 `internal/consoleapi/grpcclient/grpcclient.go`——`protoToSearchResult`（`:609`）add-only 映射 `VectorScore: p.VectorScore`（现仅 `Score` `:620` + `RetrievalMethod` `:622`），使控制面 contract 携带真实 vector provenance（不再仅经 score + retrieval_method 推断）；`contractv1.SearchResult` add-only `VectorScore` 字段（若缺）。
- 改 `core/src/retriever/mod.rs:325`——把误导性 WARN（`:326-329` 含「`not yet implemented`」措辞）改为准确契约：陈明 chunks 表（FROZEN §5.3，`indexer/mod.rs:117`）无 source_type/agent_scope 列、`SearchResult.source_type`/`agent_scope` 为 hardcoded 常量（`:452`/`:459`）、`agent_scope` 属 memory 层（0013）→ 对 chunk 检索为明确 no-op；real chunk filter 须 importer 侧 source_type tagging + schema 迁移。WARN 措辞不再暗示「待 reverse-fill 即落地」。`SearchFilters`（`:135`）struct shape 不动（既有契约不变）。
- 改 `core/src/retriever/mod.rs`（doc comment `:132-133` / `:147-150`）——把 `SPEC-DRIFT-task-2.4 pending` 等暗示「确定性待补」的散文校正为「real import-path feature → honest-defer backlog」准确描述，并就近带 [SPEC-DEFER:phase-future.chunk-source-type-filter] + [SPEC-DEFER:phase-future.chunk-agent-scope-filter] 标注。
- 新增同源测试——A：控制面 wiring 单测（`vector_score` 真实透出 / BM25 命中为 0）；B：契约诚实化单测（默认空 filter 结果与改前一致 + 非空 source_type/agent_scope filter 为准确 no-op + WARN 措辞不含「待落地」暗示）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- real chunk source_type filter（importer 侧 source_type tagging + chunks 表 schema 迁移使 `source_type` filter 真实生效）[SPEC-DEFER:phase-future.chunk-source-type-filter]——这是真实 import-path feature（须导入路径打标 + 冻结 §5.3 schema 迁移），非本 task 的契约诚实化，honest-defer 不伪造已实现。
- real chunk agent_scope filter（chunk 维度按 agent_scope 过滤；agent_scope 现属 memory 层 `memory_items` 0013，非 chunk 维度）[SPEC-DEFER:phase-future.chunk-agent-scope-filter]——须设计 chunk↔agent_scope 关联模型 + schema，honest-defer 不伪造已实现。
- v1 search proto `RetrievalResult` 字段变更（本 task 仅消费既有 `vector_score = 13`，不改 v1 proto）——v1 已携带，无须改 [SPEC-DEFER:phase-future.v1-search-proto-evolution]。
- sqlite-vec in-process 选择矩阵 cell（recall/latency）由 task-32.2 honest-defer [SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]
- 真实 release tag / run-id / digest（v0.25.0）[SPEC-OWNER:task-32.4-closeout]（ADR-012 用户授权后回填）

## 4. Actors

- 主 agent（ADR-012 自治）
- `SearchResultItem`（`console_data_plane.proto:185-201`，add-only `vector_score = 16` 携带 vector provenance）
- `protoToSearchResult`（`internal/consoleapi/grpcclient/grpcclient.go:609`，数据面 → 控制面 wiring 映射 `VectorScore`）
- `Retriever::search`（`core/src/retriever/mod.rs:325`，retrieval-filter 契约诚实化 — 准确 no-op）
- chunks 表（`core/src/indexer/mod.rs:117`，FROZEN §5.3，无 source_type/agent_scope 列 — no-op 根因锚点）
- `memory_items`（`core/migrations/0013_memory_items.sql:7`，agent_scope 属 memory 层 — 诚实校正锚点）
- 控制面 / 数据面消费者（据真实 `vector_score` 解释 hit provenance；据准确 filter 契约知情 no-op）

## 5. Behavior Contract

### 5.1 Required Reading

- `proto/contextforge/console_data_plane/v1/console_data_plane.proto:185-201`（`SearchResultItem`——`retrieval_method = 13`，字段至 `citation = 15`，无 `vector_score`；add-only 落点 = `vector_score = 16`）
- `proto/contextforge/v1/search.proto:33-49`（`RetrievalResult.vector_score = 13` + `embedding_provider = 14`，task-19.3 add-only——vector provenance parity 单源）
- `internal/consoleapi/grpcclient/grpcclient.go:609-625`（`protoToSearchResult` 现仅映射 `Score` `:620` + `RetrievalMethod` `:622`，无 `VectorScore`）
- `core/src/retriever/mod.rs:135`（`SearchFilters` struct——`source_type` / `agent_scope` 字段在 struct，shape 不动）+ `:325-330`（误导性 WARN「`not yet implemented` ... SPEC-DRIFT-task-2.4 pending」原文）+ `:452`（`source_type: DEFAULT_SOURCE_TYPE` hardcoded）+ `:459`（`agent_scope: Vec::new()` hardcoded）
- `core/src/indexer/mod.rs:117-128`（chunks 表 FROZEN §5.3——无 source_type/agent_scope 列，`kind` 为 AST 结构性非 source 分类器）
- `core/migrations/0013_memory_items.sql:7`（`memory_items.agent_scope` 列——agent_scope 属 memory 层，非 chunk 维度）
- `docs/decisions/adr-037-vector-backend-config-plumbing-and-completeness.md §D3`（console provenance add-only + retrieval-filter 契约诚实化——本 task 即其原文实现）+ `§D4`（honest-defer 边界）

### 5.2 关键设计 — A console provenance add-only + B filter 契约诚实化（默认行为不变）

- **A vector_score add-only（parity v1 search proto）**：console_data_plane `SearchResultItem`（`:185-201`）现至 `citation = 15` → add-only `vector_score = 16`（float；语义 = v1 search proto `RetrievalResult.vector_score = 13`：语义相似度，BM25-only 命中为 0）。proto add-only field 不破既有 client（旧 client 忽略未知 field，新 field default 0；ADR-004 既有契约不变）。数据面 → 控制面 wiring：`grpcclient.go::protoToSearchResult`（`:609`）add-only 映射 `VectorScore: p.VectorScore`，使控制面 contract（`contractv1.SearchResult`）携带真实 vector provenance（现仅 `Score` `:620` + `RetrievalMethod` `:622` → 间接推断）。pass bar 测试：构造一条带非 0 `vector_score` 的 `SearchResultItem`（语义命中）+ 一条 `vector_score = 0` 的 BM25 命中 → `protoToSearchResult` 后控制面 contract `VectorScore` 分别为真实值 / 0（透出而非推断）。
- **B filter 契约诚实化（准确 no-op，默认空 filter 结果一致）**：`mod.rs:325` 的 WARN 措辞从「`not yet implemented`（schema gap; SPEC-DRIFT-task-2.4 pending）」改为准确陈述——chunks 表（FROZEN §5.3）无 source_type/agent_scope 列；`SearchResult` 二者为 hardcoded 常量（`:452`/`:459`）；agent_scope 属 memory 层（0013）→ 对 chunk 检索为**明确 no-op**；real chunk filter 须 importer 侧 source_type tagging + schema 迁移（honest-defer backlog）。`SearchFilters` struct（`:135`）shape 不动；非空 source_type/agent_scope filter 仍被忽略（与改前同语义，仅 WARN 措辞诚实化）；**默认空 filter 路径完全不触 WARN 分支、结果与改前逐条一致**（ADR-004 默认行为不变）。pass bar 测试：默认空 filter search 结果与改前一致（同 query 同 hits）；传非空 source_type/agent_scope filter → 结果与不传时一致（准确 no-op，filter 被忽略）+ WARN 措辞不含「待落地 / 待 reverse-fill」暗示。
- **新 SPEC-DEFER backlog（honest-defer，不伪造）**：real chunk source_type/agent_scope filter 作为真实 import-path feature 入 backlog——`mod.rs` 触及行就近带 [SPEC-DEFER:phase-future.chunk-source-type-filter] + [SPEC-DEFER:phase-future.chunk-agent-scope-filter]；ADR-037 D3 / D4 + §范围外同步记录。0 新 dep（仅 proto add-only field + Go wiring + Rust 措辞改）。

### 5.3 不变量

- 默认行为不变（ADR-004）：默认空 filter 的 search 结果（hits / 顺序 / 各字段）与改前逐条一致；console proto 既有字段编号 / 类型不动，旧 client 对 `vector_score = 16` 未知 field 兼容忽略。
- 既有契约不变：v1 search proto `RetrievalResult`（`:35-52`）一字段不改（本 task 复用既有 `vector_score = 13`，不新增 v1 字段）；`SearchFilters`（`:135`）struct shape 不动；console proto 以 add-only 方式新增 `vector_score = 16`（不重排、不改既有 field）。
- provenance 据实透出：`vector_score` 由真实检索值映射（语义命中真实相似度 / BM25 命中为 0），非合成 / 伪造（ADR-013）。
- filter 契约诚实：非空 source_type/agent_scope filter 对 chunk 检索为准确 no-op（与改前同行为，措辞诚实化）；real chunk filter feature honest-defer 新 backlog，不伪造已实现（ADR-013）。
- 0 新代码依赖（ADR-008 add-only baseline 守线）：proto add-only field + Go wiring + Rust 措辞改，无 Cargo / go.mod 依赖增量。

## 6. Acceptance Criteria

- [ ] **AC1**（console proto add-only vector_score + provenance carried 🟢）: `console_data_plane.proto` `SearchResultItem`（`:185-201`）add-only 新增 `vector_score = 16`（float，parity v1 search proto `RetrievalResult.vector_score = 13`），既有字段编号 / 类型不动；数据面 → 控制面 wiring（`grpcclient.go::protoToSearchResult` `:609`）携带 `VectorScore`（现仅 `Score` `:620` + `RetrievalMethod` `:622`），控制面 contract 透出真实 vector provenance（语义命中真实值 / BM25 命中为 0，非推断）；旧 client 对新 field 兼容忽略（ADR-004 既有契约不变）— verified by **TEST-32.3.1**
- [ ] **AC2**（filter 契约诚实化 no-op + default 一致 + 新 SPEC-DEFER backlog 🟢）: `core/src/retriever/mod.rs:325` 误导性 WARN（含「`not yet implemented` ... SPEC-DRIFT-task-2.4 pending」）改为准确契约——chunks 表（FROZEN §5.3）无 source_type/agent_scope 列、`SearchResult` 二者 hardcoded 常量（`:452`/`:459`）、agent_scope 属 memory 层（0013）→ 对 chunk 检索为明确 no-op，real chunk filter 须 importer 侧 source_type tagging + schema 迁移；默认空 filter search 结果与改前逐条一致（默认行为不变）；非空 source_type/agent_scope filter 准确 no-op（与不传一致）；新 backlog [SPEC-DEFER:phase-future.chunk-source-type-filter] + [SPEC-DEFER:phase-future.chunk-agent-scope-filter]（honest-defer 不伪造已实现，ADR-013）— verified by **TEST-32.3.2**
- [ ] **AC3**（ADR-014 D2 lint，LAST）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中（CI spec-lint 权威）— verified by **TEST-32.3.3**（LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-32.3.1 | console_data_plane `SearchResultItem` add-only `vector_score = 16`（parity v1 `RetrievalResult.vector_score = 13`）+ `protoToSearchResult` 携带 `VectorScore`：语义命中透出真实值、BM25 命中为 0、既有字段不动、旧 client 兼容 | `proto/contextforge/console_data_plane/v1/console_data_plane.proto` + `internal/consoleapi/grpcclient/grpcclient.go`（同源 test `internal/consoleapi/grpcclient/grpcclient_test.go::TestTask323_ProtoToSearchResult_CarriesVectorScore`） | Done |
| TEST-32.3.2 | retrieval-filter 契约诚实化：默认空 filter 结果与改前逐条一致；非空 source_type/agent_scope filter 准确 no-op（与不传一致）；WARN 措辞诚实（不含「待落地 / 待 reverse-fill」暗示）+ 新 backlog [SPEC-DEFER:phase-future.chunk-source-type-filter] + [SPEC-DEFER:phase-future.chunk-agent-scope-filter] | `core/src/retriever/mod.rs`（同源 test `test_32_3_2_source_type_agent_scope_filter_is_noop`） | Done |
| TEST-32.3.3 (LAST) | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威） | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（中）console proto add-only `vector_score = 16` 字段编号冲突 / 既有 client 兼容**：误改既有字段编号或重排会破契约。
  - **缓解**：仅在 `SearchResultItem`（`:185-201`，现至 `citation = 15`）尾部 add-only `vector_score = 16`，既有 field 编号 / 类型一律不动；`buf generate` 后既有 binding diff 仅新增字段；单测断言旧 client 路径（未读 `vector_score`）行为不变。stop-condition：既有字段 diff 非空 / 旧 client 路径退化则 AC1 不标 `[x]`。
- **R2（中）filter 诚实化误改为「真实实现」越界**：B 项易被误解为「顺手实现 source_type filter」，但真实 filter 须 importer 侧打标 + 冻结 schema 迁移（越界且破 FROZEN §5.3）。
  - **缓解**：本 task **仅** 校正契约措辞 + 保 no-op 语义 + 开 backlog（[SPEC-DEFER:phase-future.chunk-source-type-filter] + [SPEC-DEFER:phase-future.chunk-agent-scope-filter]）；不触 chunks 表 schema、不改 `SearchFilters` struct shape、不实现真实过滤。stop-condition：若改动触及 chunks 表 schema 或改变 filter 行为语义则越界回退。
- **R3（低）默认行为回归**：filter 诚实化误改默认空 filter 路径致结果漂移。
  - **缓解**：WARN 分支仅在 `!source_type.is_empty() || !agent_scope.is_empty()`（`:325`）触发，默认空 filter 路径不触该分支；单测断言默认空 filter 结果与改前逐条一致。stop-condition：默认空 filter 结果漂移则 AC2 不标 `[x]`。
- **R4（低）WARN 措辞仍含 anti-pattern 词**：诚实化后措辞或仍含「not yet implemented」之类宽词表词，命中 spec-lint / 误导未除。
  - **缓解**：WARN 改用准确陈述（「对 chunk 检索为 no-op；real filter 须 importer 侧 tagging + schema 迁移」），代码内 `eprintln!` 字面不再含「not yet implemented」；docs/spec 触及行就近带 [SPEC-DEFER] 标注（lint 对 docs/ 权威，代码注释 lint 不扫但措辞仍诚实化）。stop-condition：D2 lint 命中未标注则 AC3 不标 `[x]`。

## 9. Verification Plan

```bash
# 1. AC1 — console proto add-only vector_score + 控制面 wiring（buf 重生成 + 控制面映射单测）
buf generate
go test ./internal/consoleapi/grpcclient/ -run TestProtoToSearchResult

# 2. AC2 — retrieval-filter 契约诚实化（默认空 filter 结果一致 + 非空 filter 准确 no-op）
cargo test -p contextforge-core retriever::

# 3. 不退化（全量）
cargo test --workspace
go test ./...

# 4. AC3 — D2 lint（LAST）
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界**：real chunk source_type/agent_scope filter 是真实 import-path feature（须 importer 侧 source_type tagging + chunks 表 FROZEN §5.3 schema 迁移），本 task **不实现**，仅诚实化契约 + 开 backlog [SPEC-DEFER:phase-future.chunk-source-type-filter] + [SPEC-DEFER:phase-future.chunk-agent-scope-filter]；据 ADR-013 不伪造该 feature 已实现，受阻 / 未驱动维度如实记录。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done

**§9 Verification（v0.25.0 / impl PR #214，squash commit `eaa37bd`，真实证据）**：
- AC1：`buf generate` + `go test ./...`（grpcclient）PASS —— `console_data_plane.proto` `SearchResultItem` add-only `vector_score = 16`（parity v1 `RetrievalResult.vector_score = 13`）→ buf generate 重生 Go binding（真实 rawDesc descriptor 位移 0xdc→0xff，非 EOL churn；grpc/v1 文件回退保持 surgical）→ Rust 生产端 `core/src/data_plane/search.rs` 填 `vector_score`（"vector" 命中=cosine / BM25=0，镜像 v1 `server.rs:407`，ADR-013 不伪造）→ Go 消费端 `grpcclient::protoToSearchResult` 映射 `VectorScore` + `contractv1.SearchResult` add-only `VectorScore`（ADR-015 add-only，承 task-20.1 Semantic 先例，旧 client 兼容）；**TEST-32.3.1** = `internal/consoleapi/grpcclient/grpcclient_test.go::TestTask323_ProtoToSearchResult_CarriesVectorScore`（语义命中真实值 / BM25 命中为 0）PASS。
- AC2：`cargo test -p contextforge-core`（retriever）PASS —— `retriever/mod.rs:325` 误导性 WARN（含「not yet implemented … SPEC-DRIFT-task-2.4 pending」）改为准确 no-op 契约（chunks 表 SQL_SCHEMA §5.3 FROZEN 无 source_type/agent_scope 列、`SearchResult.source_type` 为 `DEFAULT_SOURCE_TYPE` 常量、`agent_scope` 恒空属 memory 层概念 memory_items/migration 0013）+ 新 backlog `[SPEC-DEFER:phase-future.chunk-source-type-filter]` + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`；**TEST-32.3.2** = `core/src/retriever/mod.rs::test_32_3_2_source_type_agent_scope_filter_is_noop`（非空 source_type/agent_scope filter 结果与空 filter **逐字节相同**，默认空 filter 路径不触 WARN 分支、结果与改前一致）PASS。
- AC3：**TEST-32.3.3**（LAST）`bash scripts/spec_drift_lint.sh --touched origin/master` —— PR #214 触及行 0 未标注命中（CI spec-lint 权威，四门绿）。
- 不退化：`cargo test --workspace` 199 passed / 0 failed + `go test ./...` 全过 + `cargo clippy --workspace --all-targets -- -D warnings` 0 warning + `go vet ./...` clean（v0.25.0 dev box Windows MSVC rustc 1.95.0；CI 四门 cargo-test / go-test / spec-lint / lint 全 PASS）。
- 0 新 dep（proto add-only field + Go wiring + Rust 措辞改，无 Cargo / go.mod 依赖增量）/ 默认行为不变 / proto 既有字段不变 / 既有契约不变（ADR-004 / ADR-008 守线，旧 client 对 `vector_score = 16` 未知 field 兼容忽略）。
- honest-defer：real chunk source_type/agent_scope filter 是真实 import-path feature（importer 侧 source_type tagging + chunks 表 FROZEN §5.3 schema 迁移），本 task **不实现** [SPEC-DEFER:phase-future.chunk-source-type-filter] + [SPEC-DEFER:phase-future.chunk-agent-scope-filter]——受阻 / 未驱动维度据 ADR-013 如实记录，不伪造已实现（filter no-op 行为本就如此，本 task 仅契约诚实化）。

**实际改动文件（PR #214 / `eaa37bd`）**：`proto/contextforge/console_data_plane/v1/console_data_plane.proto`（add-only `vector_score = 16`）+ buf 重生 Go binding + `core/src/data_plane/search.rs`（Rust 生产端填 vector_score）+ `internal/consoleapi/grpcclient/grpcclient.go` + `internal/contractv1/contractv1.go`（add-only `VectorScore`）+ `internal/consoleapi/grpcclient/grpcclient_test.go`（`TestTask323_ProtoToSearchResult_CarriesVectorScore`）+ `core/src/retriever/mod.rs`（WARN 诚实化 + `test_32_3_2_source_type_agent_scope_filter_is_noop`）。
