# Phase 32 · vector-backend-config-plumbing-and-completeness

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。本 phase 补齐向量后端的**配置接线**与**工厂覆盖完整性**，并诚实化两处契约：**配置接线**（`core/src/server.rs:340` hybrid 热路径 + `:382` 语义热路径今天硬注入 `select_vector_backend("", 0)` 默认 BruteForce——无任何向量配置经 env/config 接到 server，运维无法在不改源码下切后端）、**工厂后端覆盖补全**（`core/src/retriever/vector/sqlite_vec.rs` 的 `SqliteVecBackend`（feature `vector-sqlite`，task-23.2 已验 MSVC 可构建）impls 全套 VectorStore 且 `name()="sqlite-vec"`、经 `mod.rs:40` re-export，但 `core/src/retriever/vector/factory.rs` 的 `select_vector_backend` 至今**无 sqlite-vec arm**——工厂后端覆盖有缺）、**console provenance 对齐**（控制面 `proto/contextforge/console_data_plane/v1/console_data_plane.proto` 的 `SearchResultItem`（:185-201）有 `retrieval_method=13` 但**无 `vector_score`**，而数据面 `proto/contextforge/v1/search.proto` 的 `RetrievalResult` 已有 `vector_score=13`——控制面 provenance 与数据面不对齐）、以及 **retrieval-filter 契约诚实化**（`core/src/retriever/mod.rs:325` 对 `source_type`/`agent_scope` 非空 filter emit 一条措辞误导的 WARN「not yet implemented」，暗示「将来某 task 即可落地」；但 chunks 表（`core/src/indexer/mod.rs:117`，§5.3 FROZEN）根本**无** `source_type`/`agent_scope` 列——`kind` 是 AST 结构性 `Option<String>`，非来源分类符——`SearchResult.source_type` 硬编码 `DEFAULT_SOURCE_TYPE`（mod.rs:452）、`agent_scope` 硬编码 `Vec::new()`（mod.rs:459），且 `agent_scope` 本属 memory 层概念（`memory_items` 表 / migration 0013）；真正的 chunk filter 须 importer 侧 source_type 打标 + schema migration——是一项 import-path feature，非确定性 nit）。配置接线 + sqlite-vec arm wiring 为 code-local 🟢 可单测；sqlite-vec in-process 选择矩阵的 recall/latency cell 须本机 MSVC feature build → 🟡 据实延后 `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]` 不伪造数值（ADR-013）。**关键诚实校正**：本 phase **不**实现 real chunk source_type/agent_scope filter——它把 mod.rs:325 的误导性 WARN 改为**准确的 no-op 契约**（默认空 filter 结果完全一致）+ 把 real filter feature 记为新 backlog（`[SPEC-DEFER:phase-future.chunk-source-type-filter]` + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`），不伪造为「filter 已实现」。默认行为 / proto / 既有契约不变（ADR-004）；既有 `cargo-test` / `go-test` / `spec-lint` 三门不退化。

> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md §3 + §4 backlog` → 各维度源码锚点（`core/src/retriever/vector/factory.rs` `select_vector_backend` arms + `let _ = dim` / `core/src/server.rs:52` `data_dir: PathBuf` + `:340` hybrid + `:382` semantic 两热路径 + `:504` `resolve_data_dir` env 模式 / `core/src/retriever/vector/sqlite_vec.rs` `SqliteVecBackend` `name()="sqlite-vec"` + `mod.rs:18-40` feature gate/re-export + `Cargo.toml` `vector-sqlite` feature / `proto/contextforge/console_data_plane/v1/console_data_plane.proto:185-201` `SearchResultItem` + `proto/contextforge/v1/search.proto:35-52` `RetrievalResult` vector_score=13 / `core/src/retriever/mod.rs:135` `SearchFilters` + `:325` 误导性 WARN + `:452/:459` source_type/agent_scope 硬编码 + `core/src/indexer/mod.rs:117` chunks 表 FROZEN §5.3 无 source_type/agent_scope 列）→ AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5，第二十三次激活）→ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造红线：sqlite-vec in-process 矩阵 recall/latency cell 须真实 MSVC feature build 才记数、filter 诚实化为 no-op 契约不伪造「filter 已实现」、受阻维度如实 defer 不伪造）。

> **ADR 影响面（已识别）**：
> - **ADR-037 vector-backend-config-plumbing-and-completeness（新，Proposed）**：记 backend config plumbing（env→server.rs 两热路径，default 保形，D1）+ sqlite-vec factory arm 补全工厂后端覆盖（in-process 矩阵 cell 据实延后 `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`，D2）+ console provenance add-only `vector_score` + retrieval-filter 契约诚实化（real chunk filter 须 importer-side source_type tagging + schema migration → 据实延后新 backlog `[SPEC-DEFER:phase-future.chunk-source-type-filter]` + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`，D3）+ 据实延后边界（D4）+ 默认 0-vector-dep baseline / 默认行为 / 既有契约不变（D5）。Status: Proposed（Draft 阶段不 ratify；ratify 在 task-32.4 closeout）。
> - 触及 **ADR-034（production-vector-live-recall）**：`select_vector_backend` 工厂补全 sqlite-vec arm（既有 brute/qdrant/lancedb arms 之外补齐 sqlite-vec 后端覆盖）——以 add-only Amendment 记录（不溯改正文，@ task-32.4）。
> - 触及 **ADR-023（vector-backend）**：默认 0-vector-dep baseline 守线（默认 `""`/"brute" → BruteForce、sqlite-vec/qdrant/lancedb 全 feature-gated、未启用 feature 显式 Err 不静默回退）——以 add-only Amendment / 守线引用记录。
> - 触及 **ADR-004（默认行为 + 既有契约不变）**：console proto `vector_score` 为 add-only field、factory sqlite-vec arm 为 add-only、filter no-op 契约不破既有空 filter 结果——默认行为 / proto / 既有契约均不变（守线，非推翻）。
> - 触及 **ADR-013（禁伪造红线）**：sqlite-vec in-process 矩阵 recall/latency cell 须本机 MSVC feature build 才记数、real chunk source_type/agent_scope filter feature 受阻如实 defer——不伪造完成 / 不伪造数值。

## 1. 阶段目标

v0.24.0 ship 后，ContextForge 补齐向量后端的配置接线与工厂覆盖完整性、并诚实化两处契约：**配置可接线**（`server.rs` hybrid + semantic 两热路径经 env/config 选 backend，而非硬注入默认；未设/"" 时仍 byte-equivalent 到 BruteForce）、**工厂后端覆盖完整**（`select_vector_backend` 补 sqlite-vec arm——feature on → `SqliteVecBackend`、feature off → 命名 `vector-sqlite` 的显式 Err，与既有 qdrant/lancedb arm 同形 honest gating）、**console provenance 对齐**（控制面 `SearchResultItem` add-only `vector_score` 字段，与数据面 `RetrievalResult.vector_score` 对齐携带 provenance）、**filter 契约诚实**（mod.rs:325 误导性 WARN → 准确 no-op 契约 + real chunk filter feature 记为新 backlog）。配置接线 + sqlite-vec arm wiring 为 code-local 🟢 可单测；sqlite-vec in-process 选择矩阵 recall/latency cell 须本机 MSVC feature build → 🟡 据实延后 `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]` 不伪造数值（ADR-013）。**关键诚实校正**：本 phase **不**实现 real chunk source_type/agent_scope filter（须 importer 侧 source_type 打标 + schema migration `[SPEC-DEFER:phase-future.chunk-source-type-filter]` + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`），仅把契约改诚实（准确 no-op + 新 backlog tag）。默认行为 / proto / 既有契约不变（ADR-004）；既有 `cargo-test` / `go-test` / `spec-lint` 三门不退化。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. **config plumbing 两热路径 + default 保形**：`core/src/server.rs:340` hybrid 路径 + `:382` semantic 路径经 env/config（承 `resolve_data_dir` 的 `CONTEXTFORGE_*` env 模式，server.rs:504）选 backend name 传入 `select_vector_backend`；未设 / "" → BruteForce，与既有 `select_vector_backend("", 0)` 硬注入 byte-equivalent（既有语义/hybrid 行为不变，TEST-29.1.3 守线）（AC1）
2. **sqlite-vec factory arm（双半 feature gating）**：`core/src/retriever/vector/factory.rs` `select_vector_backend` 加 `"sqlite-vec"` arm——feature `vector-sqlite` on → `Arc::new(SqliteVecBackend::new()?)`（`name()="sqlite-vec"`）、off → 显式 `VectorError`（错误命名 `vector-sqlite` feature，不静默回退、不伪造成功，与既有 qdrant/lancedb arm 同形）；in-process 选择矩阵（default/brute + sqlite-vec on）wiring 🟢；矩阵 recall/latency CELL 须本机 MSVC feature build → 🟡 honest-defer `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`，不伪造数值（ADR-013）（AC2）
3. **console provenance + filter 契约诚实**：`proto/contextforge/console_data_plane/v1/console_data_plane.proto` 的 `SearchResultItem`（:185-201，现至 `citation=15`）add-only `vector_score = 16`（parity 数据面 `RetrievalResult.vector_score=13`）经数据面/控制面携带 provenance；`core/src/retriever/mod.rs:325` 误导性 WARN（「not yet implemented」）→ 准确 no-op 契约（default 空 filter 结果完全一致）+ real chunk source_type/agent_scope filter feature 记为新 backlog（`[SPEC-DEFER:phase-future.chunk-source-type-filter]` + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`），不伪造为「filter 已实现」（AC3）
4. **默认不变 + v0.25.0 closeout**：默认行为 / proto / 既有契约不变（ADR-004——console proto add-only field、factory sqlite-vec arm add-only、filter no-op 不破契约）；v0.25.0 release docs + `scripts/console_smoke.sh` [41/41] + ADR-037 据真实测试 ratify（sqlite-vec 矩阵 cell 据实延后部分 ratify）+ ADR-034 add-only Amendment（sqlite-vec arm 补全工厂）+ roadmap/adapter add-only + phase §6 闭合（AC4）
5. ADR-014 D1-D5（第二十三次激活）全通过（AC5）

**v0.x 版本号决策**：v0.25.0（Phase 32，承 v0.24.0；roadmap §1.1 Phase N→v0.(N-7).0）。minor release（向量后端配置接线 + 工厂覆盖补全 + 契约诚实化；多为 code-local，console proto `vector_score` 为 add-only field、factory sqlite-vec arm 为 add-only、filter no-op 不破契约、config plumbing default 保形，默认行为 / proto / 既有契约 / 默认构建 0 新依赖（ADR-008，Phase 32 不增 dep）+ 0 网络不变）。

## 2. 业务价值

补齐向量后端「能选、覆盖全、provenance 对齐、契约诚实」四个缺口，且经核诚实校正一处易被误判为「可直接落地的 nit」的真实 import-path feature：

- **backend config plumbing（config-plumbing-two-hotpaths）**：`select_vector_backend` 工厂（task-29.1）已把后端选择集中化，但 `core/src/server.rs:340` hybrid 与 `:382` semantic 两热路径今天仍硬注入 `select_vector_backend("", 0)`（注释明记「No vector config is plumbed to the server yet」）——运维无法在不改源码、不重编译下把生产从 BruteForce 切到 qdrant/lancedb/sqlite-vec。本 phase 经 env/config（承 server.rs:504 `resolve_data_dir` 的 `CONTEXTFORGE_*` env 模式）把 backend name 接到两热路径；未设 / "" 时仍 byte-equivalent 到 BruteForce（默认行为不变）。
- **factory 后端覆盖补全（sqlite-vec-factory-arm）**：`core/src/retriever/vector/sqlite_vec.rs` 的 `SqliteVecBackend`（feature `vector-sqlite`，`Cargo.toml` `sqlite-vec = "=0.1.9"` optional）impls 全套 `VectorBackend`/`VectorIndexer`/`VectorSearcher` → `VectorStore`、`name()="sqlite-vec"`（sqlite_vec.rs:68-69）、经 `mod.rs:40` re-export，且 task-23.2（Phase 23）已确证 MSVC 可构建可运行；但 `factory.rs` `select_vector_backend` 仅有 brute/qdrant/lancedb arms，**无 sqlite-vec arm**——一个已实现、已 re-export、已验构建的后端却无法经工厂选用，是工厂后端覆盖的真实缺口。本 phase 补 sqlite-vec arm（与 qdrant/lancedb arm 同形 feature 双半 gating）。
- **console provenance 对齐（console-vector-score-parity）**：数据面 `proto/contextforge/v1/search.proto` 的 `RetrievalResult` 已有 `vector_score=13`（search.proto:48）+ `retrieval_method=8`（携带语义 provenance）；但控制面 `proto/contextforge/console_data_plane/v1/console_data_plane.proto` 的 `SearchResultItem`（:185-201）有 `retrieval_method=13` 却**无 `vector_score`**——控制面 console 客户端看不到向量得分 provenance，与数据面不对齐。本 phase add-only `vector_score = 16`（现字段至 `citation=15`，下一可用 tag 为 16），parity 数据面。
- **retrieval-filter 契约诚实化（filter-contract-honesty，关键诚实校正）**：`core/src/retriever/mod.rs:325` 对 `source_type`/`agent_scope` 非空 filter emit 一条 WARN「source_type/agent_scope filter not yet implemented (schema gap; SPEC-DRIFT-task-2.4 pending), value ignored」——措辞暗示「某 task 即可落地」。但复核源码：chunks 表（`core/src/indexer/mod.rs:117`，§5.3 FROZEN）只有 `chunk_id/file_path/line_start/line_end/language/content/content_hash/kind/collection_id/indexed_at`——**无** `source_type`/`agent_scope` 列（`kind` 是 AST 结构性 `Option<String>`，非来源分类符）；`SearchResult.source_type` 硬编码 `DEFAULT_SOURCE_TYPE`（mod.rs:452）、`agent_scope` 硬编码 `Vec::new()`（mod.rs:459）；且 `agent_scope` 本属 memory 层概念（`memory_items` 表 / migration 0013），非 chunk 检索维度。所以「real chunk filter」须 importer 侧 source_type 打标 + schema migration——是一项 import-path feature，**非确定性 nit**。本 phase **不**实现该 feature，仅把契约改诚实：mod.rs:325 误导性 WARN → 准确 no-op 契约（default 空 filter 结果完全一致）+ real filter feature 记为新 backlog `[SPEC-DEFER:phase-future.chunk-source-type-filter]` + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`，如实不伪造为「filter 已实现」（ADR-013）。

**不在本 phase 范围**：

- real chunk source_type filter（须 importer 侧 source_type 打标 + schema migration，chunks 表 §5.3 FROZEN 无该列）[SPEC-DEFER:phase-future.chunk-source-type-filter]
- real chunk agent_scope filter（agent_scope 属 memory 层概念 / `memory_items` 表，chunk 检索路径无该维度）[SPEC-DEFER:phase-future.chunk-agent-scope-filter]
- sqlite-vec in-process 选择矩阵 recall/latency CELL（须本机 MSVC feature build 真实跑出，CI 默认不构建该 feature）[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]
- qdrant/lancedb live recall 矩阵端到端（须 live 后端服务）[SPEC-DEFER:phase-future.vector-live-recall-matrix]

## 3. 涉及模块

### 32.1 backend config plumbing（task-32.1）

- 修改 `core/src/server.rs`——`:340` hybrid 路径 + `:382` semantic 路径今天硬注入 `select_vector_backend("", 0)`（注释「No vector config is plumbed to the server yet」），改为经 env/config（承 `resolve_data_dir` 的 `CONTEXTFORGE_*` env 模式，server.rs:504-521；`CoreService.data_dir: PathBuf` 在 server.rs:52）解析 backend name（如 `CONTEXTFORGE_VECTOR_BACKEND`）传入 `select_vector_backend`
- 未设 / "" → BruteForce，与既有 `select_vector_backend("", 0)` 硬注入 **byte-equivalent**（既有语义/hybrid 行为不变；既有 `select_vector_backend` 默认 arm `"" | "brute" → BruteForceVectorBackend::new()` 不动，factory.rs:39）
- `dim` 仍按既有约定（factory.rs:37 `let _ = dim` reserved for later embedder-dim negotiation）——本 task 不改 dim 语义
- 同源验证（≥2，🟢：config plumbing 单测断言 env 设值经两热路径选对 backend / default 未设 → BruteForce byte-equiv + 既有 TEST-29.1.3 守线维持绿）

### 32.2 sqlite-vec factory arm + 选择矩阵 wiring（task-32.2）

- 修改 `core/src/retriever/vector/factory.rs`——`select_vector_backend` 加 `"sqlite-vec"` arm（与既有 `"qdrant"`（factory.rs:40-51）/ `"lancedb"`（:52-63）arm 同形双半 gating）：feature `vector-sqlite` on → `Arc::new(crate::retriever::vector::SqliteVecBackend::new()?)`（`name()="sqlite-vec"`，sqlite_vec.rs:68-69，经 mod.rs:40 re-export）、off → `VectorError`（错误命名 `vector-sqlite` feature，不静默回退到 BruteForce、不伪造成功，承 ADR-013 / factory.rs 既有 honest gating 风格）
- in-process 选择矩阵 wiring（default/brute + sqlite-vec feature-on）🟢：factory arm + `name()` 选择正确性可经确定性单测验证（feature-on 半 / feature-off 半，承 TEST-29.1.1..3 pattern）
- 矩阵 recall/latency CELL = **honest-defer** 🟡：须本机 MSVC feature build（`cargo test -p contextforge-core --features vector-sqlite`）真实跑出才记数，CI 默认不构建该 feature → `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`，不伪造数值（ADR-013）
- 同源验证（≥2，🟢：feature-off → sqlite-vec honest Err 命名 feature / feature-on → factory 返回 `name()="sqlite-vec"` backend；🟡 矩阵 cell 据实延后 `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`）

### 32.3 console provenance + filter 契约诚实化（task-32.3）

- 修改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`——`SearchResultItem`（:185-201，现字段至 `citation = 15`）add-only `float vector_score = 16`（下一可用 tag）；parity 数据面 `proto/contextforge/v1/search.proto` `RetrievalResult.vector_score=13`（search.proto:48）；经 buf generate 重生 Go/Rust binding，使控制面 console 客户端可见向量得分 provenance（既有字段 tag 不动，add-only 不破既有 client，ADR-004）
- 修改 `core/src/retriever/mod.rs`——`:325` 误导性 WARN「source_type/agent_scope filter not yet implemented」→ 准确 no-op 契约措辞（如实描述 chunks 表 §5.3 无 source_type/agent_scope 列、`SearchResult.source_type` 为 `DEFAULT_SOURCE_TYPE`（mod.rs:452）/ `agent_scope` 为 `Vec::new()`（mod.rs:459）、default 空 filter 结果完全一致），并在同行带 `[SPEC-DEFER:phase-future.chunk-source-type-filter]` + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`；`SearchFilters` struct（mod.rs:135）字段不动（add-only no-op，既有调用方不破）
- **不实现 real chunk filter**：real chunk source_type filter 须 importer 侧 source_type 打标 + schema migration（chunks 表 §5.3 FROZEN）；real agent_scope filter 属 memory 层概念（`memory_items` 表 / migration 0013）——二者记为新 backlog，如实不伪造为「filter 已实现」（ADR-013）
- 同源验证（≥2，🟢：console proto add-only `vector_score=16` buf generate / golden binding 断言 + filter no-op 单测断言「default 空 filter 结果与改前完全一致」+ 非空 filter 走准确 no-op 契约路径）

### 32.4 closeout-v0.25.0（task-32.4）

- 修改 `scripts/console_smoke.sh`——banner v21→v22 + v22 changelog block + 新 step [41/41]（doc/status：default-build init baseline 断言 config plumbing default 保形 + sqlite-vec arm honest gating + console provenance parity + filter 诚实 no-op 契约可达则断言、否则 doc/status；current Phase 31 [40/40] → Phase 32 顺位 [41/41]）
- 修改 `internal/cli/smoke_syntax_test.go`——新 Test 断言 [41/41] + no-regression（denominators 不溯改，ADR-014 D5）
- 新增 `docs/releases/v0.25.0-evidence.md` + `v0.25.0-artifacts.md`（tag SHA / run id / digest 为 angle-bracket backfill marker）+ `README.md` v0.25 段 + `RELEASE_NOTES.md` v0.25.0 段
- 修改 `docs/decisions/adr-037-vector-backend-config-plumbing-and-completeness.md`——Status Proposed→Accepted（逐 D 如实：sqlite-vec 矩阵 cell / real chunk filter feature 受阻维度部分 ratify）+ 新 `## Ratification（v0.25.0 / task-32.4）`
- add-only Amendment（非正文改，ADR-014 D5）：`adr-034`（production-vector-live-recall，sqlite-vec arm 补全工厂后端覆盖）/ `adr-023`（vector-backend，0-vector-dep baseline 守线引用）；`docs/roadmap.md §3/§4` add-only（Phase 32 行 + chunk source_type/agent_scope filter + sqlite-vec in-process matrix 新 backlog 条目）
- 修改 `docs/specs/phases/phase-32-vector-backend-config-plumbing-and-completeness.md`——Status Draft→Done + §6 AC 勾选（逐维如实）
- 修改 `docs/s2v-adapter.md`——Phase 32 行 + Task 行 + ADR-037 行 + BDD 行

### BDD feature

- 新增 `test/features/phase-32-vector-backend-config-plumbing-and-completeness.feature`（≥4 scenario：config plumbing 两热路径 + default 保形 / sqlite-vec factory arm 双半 gating + 矩阵 cell 据实延后 / console `vector_score` add-only provenance + filter 诚实 no-op 契约 + 新 backlog / v0.25.0 收口）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 32.1 | `core/src/server.rs:340` hybrid + `:382` semantic 两热路径经 env/config 选 backend（承 `resolve_data_dir` env 模式）+ default 未设/"" → BruteForce byte-equiv（默认行为不变） | `../tasks/task-32.1-vector-backend-config-plumbing.md` |
| 32.2 | `core/src/retriever/vector/factory.rs` `select_vector_backend` 加 sqlite-vec arm（feature 双半 gating，与 qdrant/lancedb 同形）+ in-process 选择矩阵 wiring 🟢 + 矩阵 cell 据实延后 🟡 | `../tasks/task-32.2-sqlite-vec-factory-arm-and-selection-matrix.md` |
| 32.3 | `console_data_plane.proto` `SearchResultItem` add-only `vector_score=16`（parity v1 search proto）+ `core/src/retriever/mod.rs:325` 误导性 WARN → 准确 no-op 契约 + 新 chunk source_type/agent_scope filter backlog | `../tasks/task-32.3-console-provenance-and-retrieval-filter-honesty.md` |
| 32.4 | smoke [41/41] + v0.25.0 closeout + ADR-037 ratify + ADR-034 add-only Amendment（sqlite-vec arm 补全工厂）+ roadmap §3/§4 add-only + s2v-adapter add-only | `../tasks/task-32.4-closeout-v0.25.0.md` |

## 5. 依赖关系

- **task-32.1**（backend config plumbing）dep 既有 `core/src/retriever/vector/factory.rs` `select_vector_backend`（task-29.1 已在）+ `core/src/server.rs` 两热路径（:340/:382 已在）+ `resolve_data_dir` env 模式（server.rs:504 已在）；可独立先行（不依赖 32.2/32.3）。
- **task-32.2**（sqlite-vec factory arm）dep 既有 `core/src/retriever/vector/sqlite_vec.rs` `SqliteVecBackend`（task-18.3 spike + task-23.2 MSVC build，Phase 18/23 已在）+ `mod.rs:40` re-export（已在）+ `factory.rs` arm pattern（task-29.1 已在）+ `Cargo.toml` `vector-sqlite` feature（已在）；与 32.1/32.3 并行无依赖。
- **task-32.3**（console provenance + filter 诚实）dep 既有 `proto/contextforge/console_data_plane/v1/console_data_plane.proto` `SearchResultItem`（:185-201 已在）+ `proto/contextforge/v1/search.proto` `RetrievalResult.vector_score`（:48 parity 锚点，已在）+ `core/src/retriever/mod.rs:135/:325/:452/:459`（filter struct + WARN + 硬编码，已在）+ buf generate（已在）；与 32.1/32.2 并行无依赖。
- **task-32.4**（closeout）dep 32.1 + 32.2 + 32.3 全 Done；release docs / smoke [41/41] / ADR-037 ratify 据三 task 真实测试 / 实测产物。
- 外部：ADR-037（本 phase 新 Proposed）/ ADR-034（production-vector-live-recall，sqlite-vec arm 补全工厂后端覆盖，add-only Amendment）/ ADR-023（vector-backend，0-vector-dep baseline 守线引用）/ ADR-004（默认行为 + 既有契约不变）/ ADR-008（dep add-only，Phase 32 不增 dep）/ ADR-012（tag/release outward-facing 须用户显式授权，本轮已授权 v0.25.0）/ ADR-014 第二十三次激活 / ADR-013（禁伪造红线，sqlite-vec 矩阵 cell / real chunk filter feature 受阻如实 defer 不伪造）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（每条 AC 含 ADR-014 D3 verified by 显式 owner；Draft 阶段未勾选，实施后逐条置 `[x]`）**：

- [ ] **AC1**（config plumbing 两热路径 🟢）: `core/src/server.rs:340` hybrid + `:382` semantic 两热路径经 env/config（承 `resolve_data_dir` 的 `CONTEXTFORGE_*` env 模式，server.rs:504）选 backend；未设 / "" → BruteForce，与既有 `select_vector_backend("", 0)` 硬注入 byte-equivalent（既有语义/hybrid 行为不变，TEST-29.1.3 + 既有语义/hybrid 行为守线）— verified by **TEST-32.1.1**（env 设值经两热路径选对 backend）+ **TEST-32.1.2**（default 未设/"" → BruteForce byte-equiv，默认行为不变）+ phase-smoke step 1
- [ ] **AC2**（sqlite-vec factory arm 🟢/🟡）: `core/src/retriever/vector/factory.rs` `select_vector_backend` 加 sqlite-vec arm（feature `vector-sqlite` on → `SqliteVecBackend`（`name()="sqlite-vec"`）/ off → 命名 `vector-sqlite` 的显式 Err，与 qdrant/lancedb arm 同形双半 gating）+ in-process 选择矩阵（default/brute + sqlite-vec on）wiring 🟢；矩阵 recall/latency CELL 须本机 MSVC feature build → 🟡 honest-defer `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`，不伪造数值（ADR-013）— verified by **TEST-32.2.1**（feature 双半 gating：on → backend / off → honest Err）+ **TEST-32.2.2**（选择矩阵 wiring 🟢 + 矩阵 cell honest-defer 🟡）+ phase-smoke step 2
- [ ] **AC3**（console provenance + filter 契约诚实 🟢）: `proto/contextforge/console_data_plane/v1/console_data_plane.proto` `SearchResultItem`（:185-201）add-only `vector_score = 16`（parity 数据面 `RetrievalResult.vector_score=13`）经数据面/控制面携带 provenance；`core/src/retriever/mod.rs:325` 误导性 WARN → 准确 no-op 契约（default 空 filter 结果完全一致）+ real chunk source_type/agent_scope filter feature 记为新 backlog（`[SPEC-DEFER:phase-future.chunk-source-type-filter]` + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`），不伪造为「filter 已实现」（ADR-013）— verified by **TEST-32.3.1**（console `vector_score` add-only + 经数据面/控制面携带 provenance）+ **TEST-32.3.2**（filter 诚实 no-op + default 空 filter 结果一致 + 新 SPEC-DEFER backlog）+ phase-smoke step 3
- [ ] **AC4**（默认不变 + v0.25.0 closeout）: 默认行为 / proto / 既有契约不变（ADR-004——console proto `vector_score` add-only field、factory sqlite-vec arm add-only、filter no-op 不破契约、config plumbing default 保形）；v0.25.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ `scripts/console_smoke.sh` [41/41] + `internal/cli/smoke_syntax_test.go` markers 同步 + ADR-037 据真实测试 ratify（sqlite-vec 矩阵 cell 据实延后部分 ratify）+ ADR-034 add-only Amendment（sqlite-vec arm 补全工厂）+ ADR-023 守线引用 + roadmap §3/§4 add-only + phase §6 闭合 — verified by **TEST-32.4.1**（smoke [41/41] + smoke_syntax_test + ADR-037 ratify）+ **TEST-32.4.2**（ADR-034 add-only Amendment + roadmap §3/§4 add-only + s2v-adapter add-only + phase §6 闭合）
- [ ] **AC5**（ADR-014 cross-validation gate）: ADR-014 D1-D5（第二十三次激活）全通过 — D1 mapping + D2 lint `bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-31 不溯改（ADR 改动 add-only Amendment）— verified by task-32.4 closeout PR body + 各 task LAST TEST（TEST-32.1.3 / TEST-32.2.3 / TEST-32.3.3 / TEST-32.4.3）

**端到端 smoke（C1 集成兜底）**：(1) `server.rs` hybrid + semantic 两热路径经 env/config 选 backend，未设 / "" → BruteForce byte-equivalent（既有语义/hybrid 行为不变 + TEST-29.1.3 守线维持绿）全 PASS；(2) `select_vector_backend` sqlite-vec arm feature 双半 gating（feature-on → `name()="sqlite-vec"` / feature-off → 命名 feature 的 honest Err）+ in-process 选择矩阵 wiring 全 PASS（矩阵 recall/latency cell 🟡 须本机 MSVC feature build 如实标注据实延后，不伪造数值）；(3) console `SearchResultItem` add-only `vector_score=16` 经数据面/控制面携带 provenance + retrieval-filter 诚实 no-op 契约（default 空 filter 结果完全一致）+ real chunk filter feature 记新 backlog，默认行为 / proto / 既有契约不变全 PASS（受阻 / 延后维度如实标注）。

## 7. 阶段级风险

- **R1（中）server config plumbing 改两热路径潜在默认行为回归**：`server.rs:340/:382` 由硬注入 `select_vector_backend("", 0)` 改为读 env/config，env 解析 / 默认回落若有偏差会破既有语义/hybrid 默认行为。
  - **缓解**：task-32.1 default 未设 / "" 严格回落 BruteForce（既有 factory 默认 arm `"" | "brute"` 不动，factory.rs:39），单测断言「env 未设 → BruteForce byte-equiv + 既有语义/hybrid 行为不变 + TEST-29.1.3 守线维持绿」；env 解析失败亦回落默认（不 panic、不静默换后端）。stop-condition：default 路径 byte-equiv + 既有行为不退化方标 AC1。
- **R2（低）sqlite-vec arm 须保 0-vector-dep baseline（add-only feature-gated）**：补 sqlite-vec arm 不得令默认构建拉入 `sqlite-vec` dep / 改默认行为。
  - **缓解**：task-32.2 arm 与既有 qdrant/lancedb arm 同形双半 gating（`#[cfg(feature = "vector-sqlite")]` on → backend / `#[cfg(not(...))]` → 显式 Err 命名 feature），默认构建（无 feature）走默认 arm 0-vector-dep（ADR-023 / ADR-004 守线）；feature-off 半显式 Err 不静默回退、不伪造成功（ADR-013）。stop-condition：默认构建 0 新 dep + feature 双半 gating 单测全绿方标 AC2。
- **R3（中）console proto add-only field 不破既有 client**：`SearchResultItem` 加 `vector_score=16` 须 add-only（既有字段 tag 不动），否则破既有控制面 client 兼容。
  - **缓解**：task-32.3 用下一可用 tag `16`（现字段至 `citation=15`），既有字段 1-15 tag/类型不动（ADR-004 既有契约不变）；buf generate 重生 binding + golden 断言既有字段不变。stop-condition：add-only field + 既有 client 不破方标 AC3。
- **R4（中）filter 契约诚实化误判为「实现 real filter」**：mod.rs:325 误导性 WARN 易诱导把 source_type/agent_scope filter 当作可直接落地的 nit 去「实现」，但 chunks 表 §5.3 FROZEN 无该列、agent_scope 属 memory 层——real filter 是 import-path feature。
  - **缓解**：task-32.3 仅把契约改诚实（准确 no-op 措辞 + default 空 filter 结果完全一致单测 + 同行 `[SPEC-DEFER:phase-future.chunk-source-type-filter]` / `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`），**不写任何 real chunk filter 实现**（须 importer-side source_type tagging + schema migration，记新 backlog，ADR-013 诚实）。stop-condition：若误实现 real filter 则越界——本 task 仅诚实化契约，real filter 据实 defer 不伪造完成。
- **R5（中→🟡）sqlite-vec in-process 矩阵 cell 须本机 MSVC feature build — 部分维度不可在 CI 默认闭环**：factory arm wiring 可经 feature 双半单测验，但矩阵 recall/latency 数值须 `--features vector-sqlite` 真实跑出，CI 默认不构建该 feature。
  - **缓解**：task-32.2 以「factory arm 双半 gating + 选择矩阵 wiring 🟢（feature-on/off 单测）+ 矩阵 recall/latency cell 🟡 honest-defer `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`」拆分；AC2 以「arm gating + wiring PASS」满足，矩阵数值维度据真实 MSVC feature build 跑出后回填，不伪造数值（ADR-013）。stop-condition：矩阵 cell 数值未本机真实跑出则该维度不标达成、仅记 honest-defer。

## 8. Definition of Done

- 4 task spec（32.1-32.4）顶部 `**Status**: Done`
- §6 阶段级 AC1-5 全 `[x]`（受阻 / 延后态按 ADR-013 如实记录，不伪造——如 sqlite-vec 矩阵 cell 🟡 据实延后 / real chunk source_type+agent_scope filter feature 据实延后新 backlog）
- 端到端 smoke 3 step 全 PASS（含受阻 / 延后态如实标注）
- **ADR**：ADR-037 `Proposed → Accepted`（据真实测试 / 实测产物逐 D 项 ratify，sqlite-vec 矩阵 cell / real chunk filter feature 受阻维度据已达维度部分 ratify + 如实记录，不强 ratify）；ADR-034 经 add-only Amendment 记录（sqlite-vec arm 补全工厂后端覆盖，不溯改正文，ADR-014 D5）；ADR-023 0-vector-dep baseline 守线引用；`docs/roadmap.md §3/§4` add-only（Phase 32 行 + chunk source_type/agent_scope filter + sqlite-vec in-process matrix 新 backlog 条目）
- **adapter**：§Phase 索引 Phase 32 `Draft → Done` + `Tasks 0 → 4`；§ADR 索引 ADR-037；§BDD 追加 phase-32 feature 行；ADR-034 Amendment 记录
- **release**：`docs/releases/v0.25.0-{evidence,artifacts}.md` + `RELEASE_NOTES.md` v0.25 段 + README v0.25 段
- **smoke**：`scripts/console_smoke.sh` [41/41]（config plumbing default 保形 + sqlite-vec arm honest gating + console provenance parity + filter 诚实 no-op smoke + 既有 step 不退化，denominators 不溯改）+ `internal/cli/smoke_syntax_test.go` markers 同步
- **follow-up**：chunk source_type filter `[SPEC-DEFER:phase-future.chunk-source-type-filter]` + chunk agent_scope filter `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]` + sqlite-vec in-process matrix `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]` + vector live recall matrix `[SPEC-DEFER:phase-future.vector-live-recall-matrix]` 留 backlog
