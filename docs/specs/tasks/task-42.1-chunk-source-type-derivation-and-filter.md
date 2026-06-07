# Task `42.1`: `chunk-source-type-derivation-and-filter — core/src/retriever/mod.rs 加 classify_source_type(file_path) 纯函数（扩展名 → code/doc/config/other 确定性桶，镜像 indexer::lang_hint_from_path）+ 三处 SearchResult 构造点（search() BM25 / get_chunk / search_semantic）source_type 由 DEFAULT_SOURCE_TYPE 改真实派生 + search() BM25 加 source_type post-filter（镜像 language post-filter，空 filter byte-equiv）+ agent_scope 续 documented no-op（窄化既有 WARN 块仅 agent_scope）；0 schema migration（chunks/files/provenance §5.3 FROZEN，source_type 由 file_path 派生不存储）；据真契约改写 TEST-32.3.2`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 42 (chunk-source-type-filter)
**Dependencies**: 既有 `core/src/retriever/mod.rs`（`SearchFilters.source_type` :137 / `SearchResult.source_type` :156 / `DEFAULT_SOURCE_TYPE=""` :42 / `search()` :314 + task-32.3 no-op 块 :321-336 / language post-filter :364-388 范式 / 三构造点 :463-476·:555-568·:803-816 / `TEST-32.3.2` :903-940，task-4.2/32.3 已交付）/ 既有 `core/src/indexer/mod.rs`（`lang_hint_from_path` :483 扩展名派生范式 + SQL_SCHEMA :115-147 §5.3 FROZEN）/ 既有 `core/src/server.rs:440-453`（v1 search filter mapping，proto `filters.source_type` → `RetrieverFilters.source_type` 已就绪）/ ADR-047（chunk-source-type-filter，本 task 即其 D1/D2 原文实现）/ ADR-037（vector-backend-config-plumbing-and-completeness，source_type no-op 被本 task 真实过滤 supersede add-only Amendment @ task-42.3）/ ADR-004（空 filter byte-equiv + source_type value 填补 v0.1 schema gap）/ ADR-008（dep add-only，Phase 42 = 0 新 dep，`classify_source_type` 纯 std）/ ADR-013（禁伪造红线——source_type 由 file_path 确定性派生非合成、真实过滤行为实测非预填、agent_scope 据实 honest-defer 不伪造）/ ADR-012 / ADR-014 D1-D5（第三十三次激活）

## 1. Background

`SearchFilters.source_type` / `agent_scope`（`retriever/mod.rs:137/139`）自 task-4.2 起就有字段，v1 proto `SearchFilters.source_type=1`（`search.proto:13`）+ `RetrievalResult.source_type=3` 也早有契约，但 Phase 32（task-32.3 / ADR-037）经核据实记为 documented no-op：

- **B1 source_type 恒空（真实）**：三处 `SearchResult` 构造点（`search()` BM25 :466 / `get_chunk` :558 / `search_semantic` :806）`source_type: DEFAULT_SOURCE_TYPE.to_string()`（`DEFAULT_SOURCE_TYPE=""` :42）→ source_type **value** 恒空；`search()` 在 `:321-336` 把 source_type / agent_scope filter 据实记为 documented no-op（非空 filter → 与空 filter byte-identical），`TEST-32.3.2`（:903-940）守护该 no-op。
- **B2 source_type 可由 file_path 确定性派生（真实，决定方案）**：`indexer/mod.rs:483 lang_hint_from_path(path) -> &'static str` 已有「扩展名 → 语言」纯函数范式；source_type 是其**粗粒度桶**（code/doc/config/other）——故可在 query 时由 `file_path` 确定性派生，**无须存储、无须 schema migration**（chunks/files/provenance §5.3 保持 FROZEN）。确定性派生 == 存储值（同一 file_path 恒得同一 source_type），等价正确且更 surgical。
- **B3 读路径已就绪（真实）**：v1 `server.rs:440-453` 已把 proto `filters.source_type` → `RetrieverFilters.source_type`（只是 retriever no-op）；`search_result_to_proto`（:491-…）已映射 `source_type`。故 retriever 真实派生 + 过滤后，v1 gRPC / v1 REST body（`rest.go:137` 解码完整 proto SearchRequest 含 `filters`）路径**立即生效**。
- **B4 language post-filter 是镜像源（真实）**：`search()` 在 `:364-388` 已有 language post-filter（`want_lang = !filters.language.is_empty()`；`want_lang && !filters.language.iter().any(|l| l == &language) → continue`）——source_type post-filter 镜像此（空 filter → 不过滤 → byte-equiv）。

本 task 在 `core/src/retriever/mod.rs` 加 `classify_source_type` 纯函数 + 三构造点 populate + `search()` BM25 source_type post-filter，code-local 🟢 可单测，0 新 dep（纯 std）+ 0 schema migration。`agent_scope` 据 grounding 为 memory 层概念续 no-op（见 §5.2 B5）。

## 2. Goal

(1) **B2/B4**：`core/src/retriever/mod.rs` add `pub(crate) fn classify_source_type(file_path: &str) -> &'static str`（镜像 `indexer::lang_hint_from_path`：`match path.extension().to_ascii_lowercase()` → 确定性桶）：
   - `"code"`：rs go py js ts jsx tsx mjs cjs java kt kts scala c h cc cpp cxx hpp hh cs rb php swift m mm sh bash zsh fish ps1 sql lua r jl dart ex exs erl hs clj pl pm vue svelte（源码扩展名）
   - `"doc"`：md markdown mdx txt rst adoc asciidoc org tex（文档扩展名）
   - `"config"`：toml yaml yml json jsonc ini cfg conf env xml properties（配置扩展名）
   - `"other"`：其余 / 无扩展名 / 未知扩展名
   （确定性、纯 std、0-dep；具体表本 spec §2 固化、TEST-42.1.1 穷举断言。无扩展名（如 `Makefile`/`Dockerfile`/`LICENSE`）→ `other`，不做 basename 特例——确定性优先）
(2) **B1 populate**：三处构造点（`:466`/`:558`/`:806`）`source_type: DEFAULT_SOURCE_TYPE.to_string()` → `source_type: classify_source_type(&file_path).to_string()`——source_type value 在 BM25 / get_chunk / search_semantic 三路径真实可见（填补 task-4.2 §2A v0.1 schema gap）。
(3) **B4 filter**：`search()` BM25 加 source_type post-filter：`!opts.filters.source_type.is_empty()` 时仅留 `classify_source_type(file_path) ∈ opts.filters.source_type` 的 hit（镜像 :386 language post-filter；空 source_type → 不过滤 → byte-equiv）。
(4) **B5 agent_scope no-op**：窄化 `:321-336` 既有 no-op 块——仅 `!opts.filters.agent_scope.is_empty()` 时 stderr note；source_type 不再 no-op。`agent_scope` 据 grounding 为 memory 层概念（`memory_items` 0013 / `ListMemory` scope / `memstore.go:629-635`），chunks 无 agent 关联续 documented no-op（不伪造，ADR-013，见 ADR-047 D4）。
(5) **据真契约改写 `TEST-32.3.2`**：原断言 source_type + agent_scope 双 no-op → 拆为 agent_scope-only no-op 守护 + source_type 真实过滤断言（ADR-037 source_type no-op 被 supersede，记入 ADR-047 / ADR-037 Amendment；当前测试码随契约演进，非溯改闭合 spec）。

pass bar：`classify_source_type` 扩展名 → 桶矩阵经穷举单测（code/doc/config/other + 无扩展名 + 大小写不敏感）（🟢）；真实过滤行为（混合扩展名 fixture：`source_type=[doc]` 仅返 doc / `[code]` 仅返 code / 空 filter byte-equiv 返全部）+ source_type value 三路径 populate 非空 + agent_scope 仍 no-op（🟢）；0 schema migration（§5.3 FROZEN）；0 新 dep（ADR-008）；既有 retriever/server 单测不退化；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `core/src/retriever/mod.rs`——add `pub(crate) fn classify_source_type(file_path: &str) -> &'static str`（扩展名确定性桶 code/doc/config/other，纯 std `Path::extension`，镜像 `indexer::lang_hint_from_path`）
- 改三处构造点（`:466` BM25 `search()` / `:558` `get_chunk` / `:806` `search_semantic`）`source_type: DEFAULT_SOURCE_TYPE.to_string()` → `classify_source_type(&file_path).to_string()`
- 改 `search()`（BM25）加 source_type post-filter：`if !opts.filters.source_type.is_empty()` 时 `let st = classify_source_type(&file_path); if !opts.filters.source_type.iter().any(|s| s == st) { continue; }`（紧随 language post-filter，镜像 :386）
- 窄化 `:321-336` no-op 块——仅 `!opts.filters.agent_scope.is_empty()` 时 stderr note（source_type 从 no-op 措辞移除；agent_scope 据实续 no-op）
- 据真契约改写 `TEST-32.3.2`（:903-940）——拆为 agent_scope-only no-op 守护 + source_type 真实过滤断言（非新增第三测试，而是把原双 no-op 测试随契约演进改写；新 source_type 行为另由 TEST-42.1.2 全面断言）
- **不改**：chunks/files/provenance SQL_SCHEMA（`indexer/mod.rs:115-147` §5.3 FROZEN）/ v1 `server.rs:440-453` filter mapping（已就绪）/ `lang_hint_from_path`（:483，不改，仅作镜像范式）
- **grounding 校正（实施编译期发现，ADR-013 据实）**：`DEFAULT_SOURCE_TYPE` 常量（`:42`）在三构造点全改用 `classify_source_type` 后成为**孤儿**（仅 retriever 模块私有、无其他消费方），`-D warnings` 下 dead_code 卡红 → 据 CLAUDE.md「移除本次改动造成的孤儿」**删除该常量**（替为注释）。规划稿「不删常量」是 plan 假设、编译期 grounding 覆盖；`DEFAULT_CONTEXT_ID`/`DEFAULT_REDACTION_STATUS` 仍在用、不动。
- 同源测试：`classify_source_type` 扩展名 → 桶穷举（TEST-42.1.1）+ 真实过滤 + populate + agent_scope no-op（TEST-42.1.2）；populate（source_type value 由 "" 变派生）连带更新断言旧「v0.1 schema gap source_type==""」的既有测试（`test_4_2_1` / `test_6_2_e1` / `server.rs` search RPC wire test / `core/tests/phase4_smoke.rs` / `core/tests/phase6_smoke.rs` 改断言「有效桶 ∈ {code,doc,config,other}」）——契约演进、当前测试码随之更新（非溯改闭合 spec 文档，ADR-014 D5）

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- chunk-level `agent_scope` 真实过滤（memory 层概念、chunks 无 agent 关联、须 ingest-path schema 工程）[SPEC-DEFER:phase-future.chunk-agent-scope-filter]——本 task agent_scope 续 documented no-op
- importer 显式 source_type 打标（须 §5.3 解冻加 chunks 列）[SPEC-DEFER:phase-future.chunk-importer-source-type-tagging]——本 task 由 file_path 确定性派生粗粒度桶
- v1 / console **semantic 路径** retriever-内 source_type 过滤（v1 `search()` BM25 内过滤镜像 language 当前 scope）[SPEC-DEFER:phase-future.semantic-path-source-type-filter]——console 经 task-42.2 data_plane post-filter 覆盖 semantic/hybrid
- console-api source_type 请求侧 forward（task-42.2 交付）
- 真实 release tag / run-id / digest（v0.35.0）[SPEC-OWNER:task-42.3-closeout]（ADR-012 用户授权后回填）

## 4. Actors

- 主 agent（ADR-012 自治）
- `classify_source_type`（`core/src/retriever/mod.rs`，本 task 新增纯函数，镜像 `indexer::lang_hint_from_path` :483）
- `Retriever::search`（`core/src/retriever/mod.rs:314`，BM25 检索 + post-filter——本 task 加 source_type post-filter + populate）
- `Retriever::get_chunk` / `Retriever::search_semantic`（:540/:692，结果构造——本 task populate source_type）
- v1 `CoreService::search`（`core/src/server.rs:440-453`，已映射 proto `filters.source_type` → `RetrieverFilters`——本 task 不改，retriever 真实过滤后立即生效）
- 用户 / Agent 调用方（可按 source_type 筛选检索结果；响应显示真实 source_type）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/retriever/mod.rs:41-43`（`DEFAULT_SOURCE_TYPE=""` 等常量，不删）+ `:132-142`（`SearchFilters{source_type, language, agent_scope, ...}`）+ `:144-166`（`SearchResult.source_type` v0.1 schema-gap default）+ `:314-336`（`search()` 入口 + task-32.3 no-op 块——本 task 窄化为 agent_scope-only）+ `:364-388`（language post-filter，**source_type 镜像源**）+ `:463-476`/`:555-568`/`:803-816`（三构造点 `source_type: DEFAULT_SOURCE_TYPE`——本 task 改派生）+ `:903-940`（`TEST-32.3.2` no-op 守护——本 task 据真契约改写）
- `core/src/indexer/mod.rs:483-…`（`lang_hint_from_path` 扩展名 → 语言纯函数，**classify_source_type 镜像范式**）+ `:115-147`（SQL_SCHEMA chunks/files/provenance §5.3 FROZEN——本 task 不改）
- `core/src/server.rs:440-453`（v1 search filter mapping，proto `filters.source_type`/`agent_scope` → `RetrieverFilters` 已就绪，本 task 不改）
- `docs/decisions/adr-037-*.md`（task-32.3 source_type/agent_scope documented no-op + `[SPEC-DEFER:phase-future.chunk-source-type-filter]`/`chunk-agent-scope-filter`，本 task supersede source_type no-op add-only Amendment @ task-42.3）+ `adr-047-chunk-source-type-filter.md §D1/D2/D4`（本 task 即其原文实现）

### 5.2 关键设计 — source_type 确定性派生 + 真实过滤（0 migration / 空 filter byte-equiv / agent_scope honest no-op）

- **B2 classify_source_type 派生（纯函数、0-dep、确定性）**：`classify_source_type(file_path) -> &'static str` 据扩展名（小写）映射 code/doc/config/other（§2 固化表）；纯 std `Path::extension`，镜像 `lang_hint_from_path`；无扩展名 / 未知 → `other`（确定性优先、不做 basename 特例）。同一 file_path 恒得同一 source_type（确定性 == 存储值，0 schema migration）。
- **B1 populate（三路径一致）**：BM25 `search()` / `get_chunk` / `search_semantic` 三构造点 `source_type` 改 `classify_source_type(&file_path).to_string()`——source_type value 真实可见（填补 task-4.2 §2A v0.1 schema gap）。
- **B4 source_type post-filter（镜像 language，空 byte-equiv）**：`search()` BM25 在 language post-filter 后加：`if !opts.filters.source_type.is_empty()` 时计算 `classify_source_type(&file_path)`，不在 `opts.filters.source_type` 集合内则 `continue`。空 source_type filter → 不过滤 → 结果 byte-equiv（仅 source_type value 由 "" 变派生值）。
- **B5 agent_scope honest no-op（据实，不伪造）**：`agent_scope` 经 grounding 为 memory 层概念（`memory_items` 0013 / `MemoryListFilter` / `ListMemory` scope / `memstore.go:629-635`）；chunks 无 agent 关联、无可派生维度。本 task 窄化 `:321-336` no-op 块仅 `!opts.filters.agent_scope.is_empty()` 时 stderr note（「agent_scope 是 memory 层 filter、非 chunk 检索维度」）；agent_scope 续 documented no-op（非空 → byte-equiv），不伪造 chunk-level agent_scope 过滤（ADR-047 D4 / ADR-013）。
- **TEST-32.3.2 据真契约改写（非溯改闭合 spec）**：原 `test_32_3_2_source_type_agent_scope_filter_is_noop` 断言二者双 no-op；本 task 改写为 agent_scope-only no-op 守护（source_type 部分被 supersede——契约演进，记入 ADR-037 Amendment + ADR-047）。当前测试码随契约演进合法（ADR-014 D5 约束的是闭合 phase **spec 文档**与 ADR 正文不溯改，非当前测试码）。

### 5.3 不变量

- 空 source_type filter byte-equiv（ADR-004）：`opts.filters.source_type.is_empty()` → 过滤行为与改动前 byte-identical（仅 source_type value 字段由 "" 变派生值，结果集 / 序不变）。
- 0 schema migration（§5.3 FROZEN）：chunks/files/provenance 三表 schema 不变；source_type 由 file_path query 时确定性派生不存储。
- 0 新代码依赖（ADR-008）：`classify_source_type` 纯 std；无 Cargo 依赖增量。
- 0 网络：source_type 派生 / 过滤是本地检索决策。
- agent_scope 据实 no-op（ADR-013）：agent_scope 续 documented no-op（memory 层概念），不伪造 chunk-level 过滤；spec 据实定性。
- source_type value 填补 v0.1 schema gap（据实）：三构造点 source_type 由 `DEFAULT_SOURCE_TYPE=""` 变真实派生值是填补 task-4.2 §2A 记录的 schema gap（契约本意），由 ADR-047 D1 据实记（可观测字段变化、非永久空契约破坏）。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [x] **AC1**（classify_source_type 扩展名 → 桶矩阵 🟢）: `core/src/retriever/mod.rs` `classify_source_type(file_path) -> &'static str` 据扩展名（小写）确定性映射 code/doc/config/other（§2 固化表）；无扩展名 / 未知 / dotfile → `other`；纯 std 0-dep — verified by **TEST-42.1.1**
- [x] **AC2**（真实过滤 + populate + agent_scope no-op 🟢）: 三构造点（`search()` / `get_chunk` / `assemble_vector_result`）`source_type` 由 `DEFAULT_SOURCE_TYPE` 改 `classify_source_type(&file_path)` 真实派生；`search()` BM25 source_type post-filter（`source_type=[doc]` 仅返 doc / `[code]` 仅返 code / `[code,config]` 并集 / 空 filter byte-equiv 返全部，镜像 language）；`agent_scope` 续 documented no-op（非空 → byte-equiv）；0 schema migration（§5.3 FROZEN） — verified by **TEST-42.1.2**
- [x] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-42.1.3**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-42.1.1 | `classify_source_type` 扩展名 → 桶矩阵穷举：code（.rs/.go/.py/.ts/.jsx/.java/.sh/.sql/.cpp）/ doc（.md/.txt/.rst/.adoc/.mdx）/ config（.toml/.yaml/.json/.ini/prod.env/.xml）/ other（.bin/.png/Makefile/LICENSE/noext/.env dotfile）+ 大小写不敏感（MAIN.RS→code / READ.MD→doc）；纯 std 0-dep | `core/src/retriever/mod.rs`（同源 test） | Done |
| TEST-42.1.2 | 真实过滤 + populate + agent_scope no-op：混合扩展名 fixture（.rs + .md + .toml）baseline 3 hit + source_type value ∈ {code,doc,config} / `source_type=[doc]` 仅返 .md / `[code]` 仅返 .rs / `[code,config]` 并集 2 hit 非 doc / 非空 agent_scope filter 返与空 byte-identical | `core/src/retriever/mod.rs`（同源 test，含据真契约改写的原 TEST-32.3.2） | Done |
| TEST-42.1.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（高）source_type value 由空串变真实值破客户端**：三构造点 source_type 由 `""` 改真实派生是可观测响应字段变化。
  - **缓解**：task-4.2 §2A 早把 source_type 记为「v0.1 schema gap default ""」（契约本意是真实值），本 task 据 file_path 填补该 gap（非新增破坏字段）；空 source_type filter → 过滤行为 byte-equiv；ADR-047 D1 据实记 value 变化。stop-condition：把 value 变化夸大为 byte-equiv / 未据实记则越界（ADR-013）。
- **R2（高）agent_scope 被伪造为 chunk 维度**：为凑「source_type + agent_scope」强行给 chunks 加 agent_scope 派生 / 假过滤违 ADR-013。
  - **缓解**：grounding 据实——agent_scope memory 层概念（`memory_items` 0013 / `ListMemory` scope），chunks 无 agent 关联；本 task 不伪造，agent_scope 续 documented no-op + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`；spec §5.2 B5 + ADR-047 D4 据实定性。stop-condition：实现 chunk-level agent_scope 假过滤则 AC2 不标 `[x]`。
- **R3（中）改 §5.3 FROZEN schema 加 source_type 列**：解冻 §5.3 加列 + importer 打标 + backfill 越 surgical 边界。
  - **缓解**：source_type 由 file_path 确定性派生（query 时计算）== 存储值，0 schema migration、§5.3 chunks/files/provenance FROZEN；importer 显式打标续 `[SPEC-DEFER:phase-future.chunk-importer-source-type-tagging]`。stop-condition：本 task 加 chunks 列 / migration 则越界。
- **R4（中）source_type post-filter 破空 filter byte-equiv**：过滤逻辑在 source_type 空时仍生效破 backward-compat。
  - **缓解**：post-filter 仅 `!filters.source_type.is_empty()` 时生效（镜像 language `want_lang` 守护）；空 → 不过滤 byte-equiv；TEST-42.1.2 断言空 filter byte-equiv。stop-condition：空 filter 改变结果则 AC2 不标 `[x]`。
- **R5（中）classify_source_type 表分类争议 / 漂移**：扩展名 → 桶映射主观、可能与用户预期不符。
  - **缓解**：§2 固化确定性表（覆盖常见扩展名）+ TEST-42.1.1 穷举断言锁定；未知 → `other`（确定性兜底）；用户自定义 / importer 打标续 `[SPEC-DEFER:phase-future.chunk-importer-source-type-tagging]`。stop-condition：表非确定性 / 单测不穷举则 AC1 不标 `[x]`。

## 9. Verification Plan

```bash
# 1. AC1 — classify_source_type 扩展名 → 桶矩阵穷举
cargo test -p contextforge-core retriever::

# 2. AC2 — 真实过滤 + populate + agent_scope no-op
cargo test -p contextforge-core retriever::

# 3. 不退化（全量）
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# 4. AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.chunk-source-type-filter-defer-note]：本 task 交付 chunk source_type 由 file_path 确定性派生 + 真实过滤（v1 BM25 path + populate 三路径），🟢 可单测，0 新 dep（纯 std）+ 0 schema migration（§5.3 FROZEN）。chunk-level `agent_scope` 真实过滤 `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`（memory 层概念、须 ingest-path schema）、importer 显式 source_type 打标 `[SPEC-DEFER:phase-future.chunk-importer-source-type-tagging]`（须 §5.3 解冻）、semantic 路径 retriever-内过滤 `[SPEC-DEFER:phase-future.semantic-path-source-type-filter]`、console 请求侧 forward（task-42.2）均不在本 task 范围。source_type value 由空串变真实派生值系填补 task-4.2 §2A v0.1 schema gap（契约本意），由 ADR-047 据实记非夸大；agent_scope 据实 honest-defer 不伪造（ADR-013）。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done

**§9 Verification（feat/task-42.1-source-type-filter，真实证据）**：
- AC1：`cargo test -p contextforge-core --lib retriever::test_42_1_1` —— `test_42_1_1_classify_source_type_buckets` PASS（扩展名 → 桶矩阵穷举 code/doc/config/other + 大小写不敏感 MAIN.RS→code / READ.MD→doc + dotfile `.env` → other（`Path::extension` 对 leading-dot 文件返 None））。
- AC2：`cargo test -p contextforge-core --lib retriever::test_42_1_2` —— `test_42_1_2_source_type_filter_populate_and_agent_scope_noop` PASS（混合 fixture .rs+.md+.toml：baseline 3 hit 且每条 source_type ∈ {code,doc,config} 真实派生 / `source_type=[doc]` 仅返 b.md / `[code]` 仅返 a.rs / `[code,config]` 并集 2 hit 非 doc / 非空 agent_scope filter byte-identical 于 baseline）；`cargo test --workspace` 全绿（retriever lib 37→39，含连带更新的 `test_4_2_1`/`test_6_2_e1`/`server.rs` search wire test/`phase4_smoke`/`phase6_smoke` 改断言「有效桶」）。
- AC3：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威）。
- 0 新 dep（`classify_source_type` 纯 std）/ 0 网络 / 0 schema migration（chunks/files/provenance §5.3 FROZEN，source_type 由 file_path 派生不存储）/ 空 source_type filter byte-equiv / agent_scope 续 documented no-op；`cargo clippy --workspace --all-targets -- -D warnings` clean。

**实际改动文件**：
- `core/src/retriever/mod.rs`——add `pub(crate) fn classify_source_type(file_path) -> &'static str`（扩展名确定性桶，镜像 `indexer::lang_hint_from_path`）+ `search()` BM25 source_type 派生 + post-filter（time filter 后、provenance 前）+ 三构造点（`search()`/`get_chunk`/`assemble_vector_result`）`source_type` 真实派生 + 窄化 no-op 块仅 agent_scope + **删除孤儿 `DEFAULT_SOURCE_TYPE` 常量**（grounding 校正，替注释）+ TEST-42.1.1/.2（含据真契约改写的原 TEST-32.3.2）。
- `core/src/server.rs` / `core/tests/phase4_smoke.rs` / `core/tests/phase6_smoke.rs`——既有 source_type=="" schema-gap 断言改「有效桶 ∈ {code,doc,config,other}」（populate 契约演进连带更新）。

**grounding 校正（ADR-013 据实）**：(1) `DEFAULT_SOURCE_TYPE` 在三构造点改派生后成孤儿（`-D warnings` dead_code 卡红）→ 删除（规划稿「不删常量」plan 假设被编译期 grounding 覆盖）；(2) populate（source_type value 由 "" 变派生值）连带需更新 4 处既有断言旧「v0.1 schema gap """ 的测试（非仅 TEST-32.3.2）——契约演进、当前测试码随之更新（非溯改闭合 spec 文档，ADR-014 D5）；(3) source_type 由 file_path 确定性派生 == 存储值，0 schema migration（§5.3 FROZEN）；(4) agent_scope 据实续 documented no-op（memory 层概念，不伪造）。
