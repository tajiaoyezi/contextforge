# ADR `047`: `chunk-source-type-filter`

**Status**: Proposed（规划稿；ratify 在 v0.35.0 / task-42.3 closeout 据 task-42.1/42.2 真实 CI + 实测过滤行为逐 D ratify）

**Category**: 检索质量 / chunk 过滤落地 / 诚实 no-op→真实契约 / proto add-only console forward
**Date**: 2026-06-07
**Decided By**: 主 agent（ADR-012 自治；本批为规划稿 Proposed）；tajiaoyezi ratification at v0.35.0 closeout
**Related**: ADR-037（vector-backend-config-plumbing-and-completeness — task-32.3 把 source_type/agent_scope 据实定为 documented no-op + 开 `[SPEC-DEFER:phase-future.chunk-source-type-filter]`/`chunk-agent-scope-filter`，本 ADR 兑现 source_type 维度、supersede 其 source_type no-op、agent_scope no-op 据实保持）/ ADR-015（proto-evolution-add-only — console_data_plane `SearchRequest` add-only `source_type=9` 既有字段号冻结）/ ADR-024（console-api-semantic-forward — `?semantic` 请求侧 forward 范式）/ ADR-044（console-api-retrieval-signal-forward — `?hybrid` 请求侧 forward 范式，本 ADR `?source_type=` 承接）/ ADR-004（local-first-privacy-baseline — 空 source_type filter byte-equiv + source_type value 填补 v0.1 schema gap 由本 ADR 据实记）/ ADR-008（dep add-only — Phase 42 = 0 新依赖，`classify_source_type` 纯 std）/ ADR-013（禁伪造红线 — source_type 由 file_path 确定性派生非合成、agent_scope 据实 honest-defer 不伪造为 chunk 维度、真实过滤行为实测非预填）/ ADR-012（main-agent-governance-autonomy — tag/release outward-facing 须用户显式授权）/ ADR-014（D1-D5，第三十三次激活）/ roadmap §3.24 + §4

## Context

ContextForge 截至 Phase 41（tokenizer-default-on, Done / v0.34.0）的检索 filter 契约状态：`SearchFilters`（`core/src/retriever/mod.rs:135-142`）自 task-4.2 起就有 `source_type: Vec<String>` / `agent_scope: Vec<String>` 两字段，v1 search proto（`proto/contextforge/v1/search.proto:12-14` `SearchFilters{source_type, language}` + `RetrievalResult.source_type=3`）也早有契约，但 **Phase 32（task-32.3 / ADR-037）经 grounding 据实把二者定为「documented no-op」**——非「未实现占位」而是诚实契约（`retriever/mod.rs:321-336` + `TEST-32.3.2`：非空 source_type/agent_scope filter 返与空 filter byte-identical），并开 `[SPEC-DEFER:phase-future.chunk-source-type-filter]` + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`。grounding 逐维度调研结论：

- **source_type 可由 file_path 确定性派生（真实，决定方案）**：`core/src/indexer/mod.rs:483 lang_hint_from_path(path) -> &'static str` 已有「扩展名 → 语言」纯函数范式；source_type 是其**粗粒度桶**（code/doc/config/other）——故可在 query 时由 `file_path` 确定性派生，**无须存储、无须 schema migration**（chunks/files/provenance §5.3 保持 FROZEN）。确定性派生 == 存储值（同一 file_path 恒得同一 source_type），等价正确且比「加列 + importer 打标 + 既有 chunk backfill」更 surgical。

- **读路径已就绪（真实）**：v1 `server.rs:440-453` 已把 proto `filters.source_type` → `RetrieverFilters.source_type`（只是 retriever no-op）；`search_result_to_proto`（:491-…）已映射 `source_type`；v1 REST `rest.go:137` 解码完整 proto SearchRequest（含 `filters`）。console 数据面 `SearchResultItem.source_file_type=5`（`console_data_plane.proto:195`）+ `data_plane/search.rs:378 source_file_type: h.source_type` 响应侧已就绪。故 retriever 真实派生 + 过滤后，v1 gRPC / v1 REST body 立即生效、console 响应立即显示真实 source_type；缺的只是 console **请求侧** source_type 字段。

- **agent_scope 是 memory 层概念（真实，honest-defer 依据）**：`agent_scope` 真实归属 memory（`memory_items` migration 0013 / `MemoryListFilter` / `ListMemory` scope filter / `internal/consoleapi/memstore.go:629-635` `item.AgentScope` 过滤）；chunks 无 agent 关联、无可派生维度。一个真实的 chunk-level agent_scope filter 须 ingest-path schema 工程（为 chunks 引入 agent 维度）且价值不明。

本 ADR 把「chunk source_type 过滤落地 + console 请求侧 forward + agent_scope 据实 honest-defer」收敛为处理策略。**关键诚实校正（ADR-013，本 phase 核心）**：`chunk-source-type-filter` 与 `chunk-agent-scope-filter` 经 grounding 并**不对称**——前者可派生、可真实落地（0 migration）；后者须 ingest-path schema 工程且价值不明，**本 phase 不伪造**（镜像 Phase 32/34/35 的 grounding 校正手法）。改动 🟢 可单测（classify 矩阵 + 真实过滤 + populate + console forward + post-filter）+ proto add-only。0 新依赖（ADR-008）+ 0 网络 + 0 schema migration（§5.3 FROZEN）。

## Decision

chunk source_type 过滤采用 **「file_path 确定性派生（0 migration）+ v1 retriever 真实过滤 + populate 三路径 + console proto add-only 请求侧 forward + agent_scope 据实 honest-defer」** 策略，分 4 个决策点：

### D1 — source_type 由 file_path 确定性派生（`classify_source_type` 纯函数，0 schema migration，§5.3 FROZEN）（task-42.1）🟢

`core/src/retriever/mod.rs` add `pub(crate) fn classify_source_type(file_path: &str) -> &'static str`（镜像 `indexer::lang_hint_from_path`：`match path.extension().to_ascii_lowercase()` → 确定性桶 code/doc/config/other，§task-42.1 §2 固化扩展名表）；纯 std `Path::extension`，无扩展名 / 未知 → `other`（确定性优先）。三处 `SearchResult` 构造点（`search()` BM25 :466 / `get_chunk` :558 / `search_semantic` :806）`source_type: DEFAULT_SOURCE_TYPE.to_string()` → `classify_source_type(&file_path).to_string()`——source_type **value** 三路径真实可见。

**理由**：ADR-037 task-32.3 据实记 chunks 表无 source_type 列、`SearchResult.source_type` 恒 `DEFAULT_SOURCE_TYPE=""`，定为 documented no-op + `[SPEC-DEFER:phase-future.chunk-source-type-filter]`。grounding 显 source_type 可由 file_path **确定性派生**（与 `language` 同源信号，`lang_hint_from_path` 范式已在）——确定性派生 == 存储值，无须解冻 §5.3 加列 + importer 打标 + 既有 chunk backfill（更 surgical、等价正确、0 migration 风险，simplicity-first）。source_type value 由 `""` 变真实派生值是填补 task-4.2 §2A 记录的「v0.1 schema gap default ""」（契约本意是真实 source_type、非永久空）——ADR-047 据实记该可观测字段变化（非破坏性默认变更：空 source_type filter 下过滤行为 byte-equiv，仅 value 字段变化）。备选「加 chunks source_type 列 + importer 打标」破 §5.3 FROZEN + 须 migration + backfill，不取（见 §A2）；importer 显式打标续 `[SPEC-DEFER:phase-future.chunk-importer-source-type-tagging]`。

### D2 — v1 retriever 真实 source_type 过滤（`search()` BM25 post-filter 镜像 language；空 filter byte-equiv）（task-42.1）🟢

`search()`（BM25）加 source_type post-filter：`!opts.filters.source_type.is_empty()` 时计算 `classify_source_type(&file_path)`，不在 `opts.filters.source_type` 集合内则 `continue`（紧随 language post-filter，镜像 `:386`）。空 source_type filter → 不过滤 → 结果 byte-equiv。v1 `server.rs:440-453` 已映射 proto `filters.source_type` → `RetrieverFilters.source_type` → retriever 真实过滤后 v1 gRPC / v1 REST body（`rest.go:137` 解码 `filters`）立即生效。`agent_scope` 续 documented no-op（窄化 `:321-336` no-op 块仅覆盖 agent_scope）。

**理由**：language post-filter（`want_lang = !filters.language.is_empty()`）是既有镜像源——source_type post-filter 同构（空守护 → byte-equiv）。v1 读路径已就绪（server.rs filter mapping + search_result_to_proto），retriever 真实过滤即闭环 v1 path，无须改 v1 server/proto。`search()` BM25 内过滤镜像 language 当前 scope（v1 semantic 路径 retriever-内过滤续 `[SPEC-DEFER:phase-future.semantic-path-source-type-filter]`；console 经 D3 data_plane post-filter 覆盖 semantic/hybrid）。

### D3 — console-api 请求侧 source_type forward（proto add-only `source_type=9` + data_plane post-filter + Go `?source_type=`）（task-42.2）🟢

`console_data_plane.proto` `SearchRequest` add-only `repeated string source_type = 9`（既有字段 1-8 号冻结，ADR-015 add-only，buf generate）+ `core/src/data_plane/search.rs` 按 `req.source_type` 对 BM25/semantic/hybrid 三分支汇总后的 hit 做 post-filter（利用 task-42.1 populate 的 `h.source_type`；空 → 不过滤 byte-equiv）+ Go `internal/contractv1.SearchRequest` add-only `SourceType []string` + `handleSearch` 解析 `?source_type=`（repeated query param + body 并集，镜像 `?semantic`/`?hybrid` OR-merge）+ grpcclient 映射 → console_data_plane `source_type`。

**理由**：console 响应侧已就绪（`SearchResultItem.source_file_type=5` + `data_plane/search.rs:378`，task-42.1 populate 后立即显示真实值），缺的只是请求侧 source_type 字段。proto add-only `source_type=9`（下一空号，ADR-015 字段号冻结）+ data_plane 统一 post-filter（覆盖三检索路径一致，利用 populate 的 `h.source_type`）+ Go `?source_type=` forward 镜像 `?semantic`（ADR-024）/ `?hybrid`（ADR-044）请求侧 forward 范式（差异：source_type 是 repeated、并集合并）。空 source_type → 不过滤 backward-compat。

### D4 — agent_scope 据实 honest-defer（memory 层概念、chunks 无该维度，续 documented no-op + SPEC-DEFER）（all tasks）🟢

`agent_scope` 经 grounding 为 memory 层概念（`memory_items` 0013 / `MemoryListFilter` / `ListMemory` scope / `memstore.go:629-635`）；chunks 无 agent 关联、无可派生维度。本 phase **不伪造** chunk-level agent_scope filter——agent_scope 续 documented no-op（窄化 retriever no-op 块仅覆盖 agent_scope，非空 → byte-equiv），`[SPEC-DEFER:phase-future.chunk-agent-scope-filter]` 据实保持（真实落地须 ingest-path schema 工程为 chunks 引入 agent 维度且价值不明）。

**理由**：`chunk-source-type-filter`（D1-D3 兑现）与 `chunk-agent-scope-filter` 经 grounding **不对称**——前者可由 file_path 确定性派生（0 migration），后者须为 chunks 引入 agent 维度（ingest-path schema 工程）且 agent_scope 本质是 memory 层概念、价值不明。为凑齐「source_type + agent_scope」而强行给 chunks 加 agent_scope 假派生 / 假过滤是伪造（ADR-013 红线）。据实 honest-defer、不强行扩面（honest over padding），镜像 Phase 32（filter 契约诚实化）/ Phase 34（get_source_chunk verify-only 校正）/ Phase 35（7→3-4 silent-failure 校正）的 grounding 校正手法。

## Consequences

- **Positive**: chunk 检索的 source_type 过滤从 Phase 32 据实记的 documented no-op 落地为真实能力——用户可经 v1 `{"filters":{"source_type":["doc"]}}` / console `POST /v1/search?source_type=doc` 按来源类型（code/doc/config/other）筛选；source_type value 三路径真实可见（填补 v0.1 schema gap）；**关闭一个诚实缺口**（API 一直收 source_type 却忽略它）；**0 schema migration**（source_type 由 file_path 确定性派生、§5.3 chunks/files/provenance FROZEN）+ proto add-only console forward（既有字段号冻结）+ 空 filter byte-equiv（ADR-004）；**0 新依赖**（`classify_source_type` 纯 std）+ 0 网络；既有三门不退化。
- **Negative / open**（受阻 / 另一层项如实，不伪造、不夸大）：source_type value 由空串变真实派生值是可观测响应字段变化（填补 v0.1 schema gap，非永久空契约破坏，由本 ADR D1 据实记）；chunk-level `agent_scope` 真实过滤须 ingest-path schema 工程 + agent_scope 本质 memory 层概念 → `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`（不伪造）；importer 显式 source_type 打标须 §5.3 解冻 → `[SPEC-DEFER:phase-future.chunk-importer-source-type-tagging]`（本 phase 由 file_path 派生粗粒度桶）；v1 semantic 路径 retriever-内 source_type 过滤镜像 language 当前 scope（console 经 data_plane post-filter 覆盖）→ `[SPEC-DEFER:phase-future.semantic-path-source-type-filter]`；classify_source_type 扩展名 → 桶映射是确定性约定（未知 → other），用户自定义续 importer 打标 marker。
- **Ratification**: 本 ADR **Proposed**。task-42.1/42.2 通过后于 v0.35.0 closeout（task-42.3）据真实 CI（cargo-test / go-test / lint / spec-lint）+ 实测过滤行为（classify 矩阵 + 真实过滤 + populate + console forward + post-filter + smoke v32[51/51] distinguishing）逐 D ratify Proposed→Accepted（ADR-013：禁据合成 / 伪造 ratify）。
- **Follow-ups**: chunk-level agent_scope 过滤 `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`；importer 显式 source_type 打标 `[SPEC-DEFER:phase-future.chunk-importer-source-type-tagging]`；semantic 路径 retriever-内 source_type 过滤 `[SPEC-DEFER:phase-future.semantic-path-source-type-filter]`。ADR-037（source_type no-op 被本 phase 真实过滤 supersede / agent_scope no-op 据实保持）以 add-only Amendment 于 task-42.3 记录（不溯改正文，ADR-014 D5）；ADR-015 / ADR-024 / ADR-044 / ADR-004 / ADR-008 / ADR-013 引用均不溯改其正文。

## Alternatives

- **A1（保持 source_type documented no-op）**：保留 Phase 32 的 source_type no-op 契约。否决：ADR-037 task-32.3 已开 `[SPEC-DEFER:phase-future.chunk-source-type-filter]`（已识别 follow-up）；source_type 可由 file_path 确定性派生（0 migration）真实落地；API 一直收 source_type 却忽略它是诚实缺口——本 phase 关闭之。
- **A2（加 chunks source_type 列 + importer 打标 + backfill）**：解冻 §5.3 给 chunks 加 source_type 列、importer 侧打标、既有 chunk backfill。否决：破 §5.3 FROZEN + 须 schema migration + 既有数据 backfill（migration 风险 + 非 surgical）；source_type 由 file_path **确定性派生** == 存储值（等价正确、0 migration）。importer 显式打标（支持用户自定义 source_type，超 file_path 派生）续 `[SPEC-DEFER:phase-future.chunk-importer-source-type-tagging]`。
- **A3（为对称强行做 chunk-level agent_scope 过滤）**：给 chunks 加 agent_scope 派生 / 假过滤凑齐「source_type + agent_scope」。否决：agent_scope 经 grounding 是 memory 层概念（`memory_items` 0013 / `ListMemory` scope），chunks 无 agent 关联、无可派生维度；强行加 agent_scope 假过滤是伪造（ADR-013）。据实 honest-defer `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`（须 ingest-path schema 工程 + 价值不明）。
- **A4（console 仅响应侧显示 source_type、不加请求侧 forward）**：只靠 task-42.1 populate 让 console 响应显示 source_type、不加 console 请求侧 filter。否决：console 用户无法按 source_type 筛选（请求侧缺字段）；proto add-only `source_type=9` + `?source_type=` forward 镜像 `?semantic`/`?hybrid` 范式，给 console 完整请求-响应闭环。
- **A5（v1 retriever-内过滤 semantic 路径也做）**：`search_semantic` 也加 source_type retriever-内过滤。否决：v1 `search()` BM25 内过滤镜像 language 当前 scope（language 也仅 BM25 path）；console 经 data_plane post-filter 已覆盖 semantic/hybrid（利用 populate 的 h.source_type）；v1 semantic retriever-内过滤续 `[SPEC-DEFER:phase-future.semantic-path-source-type-filter]`（焦点版本不强行扩面）。
- **A6（classify_source_type 用 basename 特例如 Makefile/Dockerfile）**：对无扩展名特殊文件名做 basename 映射。否决：basename 特例增复杂度 + 主观性；本 phase 用确定性扩展名映射（无扩展名 → other），用户自定义续 importer 打标 marker（simplicity-first）。

## 触及 ADR 关系

- **ADR-037（vector-backend-config-plumbing-and-completeness）→ add-only Amendment @ task-42.3**：其 task-32.3 把 source_type/agent_scope 据实定为 documented no-op + 开 `[SPEC-DEFER:phase-future.chunk-source-type-filter]`/`chunk-agent-scope-filter`；本 phase source_type no-op 被真实过滤 supersede（D1-D3），agent_scope no-op 据实保持（D4）。以 `## Amendment (Phase 42 / v0.35.0)` add-only 记，**不溯改 ADR-037 正文**（ADR-014 D5）。
- **ADR-015（proto-evolution-add-only）→ 守线**：console_data_plane `SearchRequest` add-only `source_type=9`（既有字段 1-8 号 + 类型不变，空 → 不过滤 backward-compat）；v1 search.proto `SearchFilters.source_type=1` 既有不动。
- **ADR-024（console-api-semantic-forward）/ ADR-044（console-api-retrieval-signal-forward）→ 范式承接（不溯改）**：console `?source_type=` 请求侧 forward 镜像 `?semantic`（ADR-024）/ `?hybrid`（ADR-044）的 query param + body 合并 → grpcclient 透传范式。
- **ADR-004（local-first-privacy-baseline）→ 守线**：空 source_type filter → 过滤行为 byte-equiv；source_type value 填补 v0.1 schema gap（task-4.2 §2A 记录）由本 ADR D1 据实记（可观测字段变化、非默认行为破坏）；不溯改 ADR-004 正文。
- **ADR-008（dep add-only）→ 守线**：本 phase 加 **0 新依赖**（`classify_source_type` 纯 std；proto add-only 无新 dep）+ 0 网络 + 0 schema migration。
- **ADR-013（禁伪造红线）→ 守线**：source_type 由 file_path 确定性派生非合成；agent_scope 据实 honest-defer 不伪造为 chunk 维度；真实过滤行为实测非预填；source_type value 变化据实记非夸大为 byte-equiv（D1/D4）。
- **ADR-014（cross-phase-exit-criteria-validation）→ 第三十三次激活**：D1-D5 mapping + 各 task LAST D2 lint（touched 行 0 未标注命中）+ D3 verified-by + D4 自治 + D5 历史 Phase 1-41 不溯改（ADR 改动 add-only Amendment）；本 ADR ratify 在 task-42.3 closeout，Draft 阶段不 ratify。
