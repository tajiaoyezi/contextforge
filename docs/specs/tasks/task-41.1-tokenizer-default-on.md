# Task `41.1`: `tokenizer-default-on — core/src/server.rs 加 resolve_tokenizer() env-resolution（读 CONTEXTFORGE_TOKENIZER：unset→code_cjk 翻默认 / "default"→opt-out 回 legacy TEXT / "code_cjk"/"cjk_segmenter"(feature) passthrough / unknown/feature-off→stderr WARN+code_cjk）+ 生产索引两调用点（server.rs:141 CoreService::index + jobs/index_session_backend.rs:151）由 IndexSession::open(..) 改 open_with_tokenizer(.., &resolve_tokenizer())；IndexSession::open/DEFAULT_TOKENIZER 库 API+常量不动（向后兼容库调用方+既有单测）；既有 collection 经 open_in_dir 保持持久化 TEXT 不被静默失效；Phase 24 harness 复测 default vs code_cjk 真实 recall delta +0.0909（首次刻意默认变更非 byte-equiv，由 ADR-046 承接）`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 41 (tokenizer-default-on)
**Dependencies**: 既有 `core/src/indexer/mod.rs`（task-24.1 `code_cjk` analyzer + `CODE_CJK_TOKENIZER` :185 / `DEFAULT_TOKENIZER` :183 / `register_code_cjk` :379 / `open_with_tokenizer` :511 + create-vs-open meta.json 分支 :528-535 / `IndexSession::open` :502 / `reindex_with_tokenizer` :920，task-30.1 `cjk_segmenter` feature + `CJK_SEGMENTER_TOKENIZER` :189，Phase 24/30 已交付）/ 既有 `core/src/server.rs`（`resolve_data_dir` :521-549 + `resolve_vector_backend` :551-560 env-resolution 范式 + `CoreService::index` :141 生产索引入口，task-32.1/34.1 已在）/ 既有 `core/src/jobs/index_session_backend.rs`（:151 生产索引 `IndexSession::open`）/ 既有 `core/examples/phase24_tokenizer_recall.rs` + `docs/spikes/phase-24-tokenizer-recall.md`（recall delta harness）/ ADR-029（code-and-cjk-tokenizer，默认开启维度兑现 add-only Amendment @ task-41.3）/ ADR-035（cjk-true-segmenter-and-tokenizer-default，D3 产品决策兑现 add-only Amendment @ task-41.3）/ ADR-046（tokenizer-default-on，本 task 即其 D1/D3 原文实现）/ ADR-004（刻意默认变更例外由 ADR-046 承接，opt-out 后 legacy byte-equiv + 不自动迁移 safety intent 保持）/ ADR-008（dep add-only，Phase 41 = 0 新 dep，`code_cjk` 纯 std）/ ADR-013（禁伪造红线——recall delta +0.0909 真实实测、刻意默认变更据实定性非夸大、jieba 默认不取据实）/ ADR-012 / ADR-014 D1-D5（第三十二次激活）

## 1. Background

code/CJK tokenizer `code_cjk`（task-24.1 / ADR-029，纯 std bigram + 代码符号拆分）自 v0.17.0 已实存但仅 opt-in；其默认化在 Phase 24（ADR-029 §Negative）+ Phase 30（ADR-035 D3）两度被据「翻默认是产品决策」诚实延后（`[SPEC-DEFER:phase-future.tokenizer-default-on]`）。本 task 做出该产品决策：

- **B1 生产默认仍 `TEXT`（真实）**：生产索引全走 `IndexSession::open(..)`（`core/src/server.rs:141` `CoreService::index` RPC + `core/src/jobs/index_session_backend.rs:151`）→ `open_with_tokenizer(.., DEFAULT_TOKENIZER="default")`（`indexer/mod.rs:502/183`）→ 新建 collection `content` 字段绑 Tantivy 默认 `TEXT` analyzer。今天 0 tokenizer env/config 接线（不像 vector/reranker 有 `CONTEXTFORGE_*`）。
- **B2 翻默认对既有 collection 自动安全（真实，schema-driven）**：tokenizer 绑定真相源是 Tantivy `meta.json`——`open_with_tokenizer` 仅 create 时（`meta.json` 不存在）用传入 tokenizer 建 schema，**open 既有 collection 走 `Index::open_in_dir` 读回持久化 schema、忽略传入值**（`indexer/mod.rs:528-535`）；query 侧 `Retriever::open_with_config` 据 schema 派生 analyzer（`register_code_cjk` 无条件注册，task-24.1 R4 对称）。故翻默认对既有 `TEXT` collection 零影响（不被静默失效），仅新建 collection 绑 `code_cjk`。
- **B3 Phase 24 实测真实收益（非合成）**：`docs/spikes/phase-24-tokenizer-recall.md` 实测 `code_cjk` over default `TEXT` recall delta **+0.0909**（default 0.9091 → code/CJK 1.0000，over task-24.2 golden）。Phase 30 另测 jieba `cjk_segmenter` vs bigram delta=+0.0000（小语料无增益）→ 默认取 0-dep `code_cjk`、不取重 jieba。
- **B4 首次刻意默认变更（据实，非 byte-equiv）**：翻默认令**新建** collection 倒排词项 `TEXT`→`code_cjk`——**非 byte-equivalent**，是项目首次刻意默认行为变更，由 ADR-046 D1/D4 显式承接（既有 collection 安全 + `CONTEXTFORGE_TOKENIZER=default` opt-out + 不自动迁移 + Phase 24 +0.0909 justify）。spec / ADR 据实定性、不夸大为 byte-equiv（ADR-013）。

本 task 在 `core/src/server.rs` 加 `resolve_tokenizer()` env-resolution + 生产两调用点接线，为 code-local 🟢 可单测 / 🟡 本地 real recall delta，0 新 dep（`code_cjk` 纯 std）。

## 2. Goal

(1) **B1/B2**：`core/src/server.rs` add `resolve_tokenizer() -> String`（镜像 `resolve_data_dir`/`resolve_vector_backend`）读 `CONTEXTFORGE_TOKENIZER`：unset/"" → `CODE_CJK_TOKENIZER`（翻默认）；`"default"` → `DEFAULT_TOKENIZER`（opt-out 回 `TEXT`）；`"code_cjk"` → `CODE_CJK_TOKENIZER`；`"cjk_segmenter"` → feature `cjk-segmenter` 在则 `CJK_SEGMENTER_TOKENIZER`、缺则 stderr WARN + `CODE_CJK_TOKENIZER`；其余未知值 → stderr WARN + `CODE_CJK_TOKENIZER`（best-effort 不静默落 `TEXT`）。生产索引两调用点（`server.rs:141` + `jobs/index_session_backend.rs:151`）改 `open_with_tokenizer(.., &resolve_tokenizer())`。`IndexSession::open` / `DEFAULT_TOKENIZER` **不动**。既有 collection 经 `open_in_dir` 保持持久化 `TEXT`（不被静默失效）。(2) **B3**：经 `phase24_tokenizer_recall` harness 复测 default `TEXT` vs `code_cjk` 真实 recall delta（+0.0909 已 Phase 24 实测，本 task 复确认默认化后此增益成出厂基线，真实数不预填 ADR-013）。(3) **B4**：刻意默认变更（新建 collection 非 byte-equiv）由 ADR-046 D1/D4 承接、spec 据实定性，安全边界（既有 collection 安全 + opt-out + 不自动迁移）据实记。

pass bar：`resolve_tokenizer` env 矩阵经确定性单测验证（unset→`code_cjk` / `"default"`→`default` / `"code_cjk"`→`code_cjk` / unknown→WARN+`code_cjk`）（🟢）；生产路径新建 collection 默认绑 `code_cjk`（`content_tokenizer_name`）、opt-out env → 绑 `default`、既有 `TEXT` collection 经生产路径 open 保持 `default`（不被静默改）（🟢）；Phase 24 harness 真实 recall delta +0.0909 复测（🟡 本地，小 golden caveat，不预填）；`IndexSession::open` / `DEFAULT_TOKENIZER` / 既有 indexer/retriever 单测不退化；0 新 dep（ADR-008）；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `core/src/server.rs`——add `fn resolve_tokenizer() -> String`（读 `std::env::var("CONTEXTFORGE_TOKENIZER")`，trim；空/unset → `CODE_CJK_TOKENIZER.to_string()`；`"default"` → `DEFAULT_TOKENIZER.to_string()`；`"code_cjk"` → `CODE_CJK_TOKENIZER`；`"cjk_segmenter"` → `#[cfg(feature="cjk-segmenter")]` 时 `CJK_SEGMENTER_TOKENIZER`、否则 `eprintln!("WARN ... cjk-segmenter feature off, using code_cjk")` + `CODE_CJK_TOKENIZER`；其余 → `eprintln!("WARN ... unknown tokenizer X, using code_cjk")` + `CODE_CJK_TOKENIZER`），import `CODE_CJK_TOKENIZER`/`DEFAULT_TOKENIZER`（+ feature 下 `CJK_SEGMENTER_TOKENIZER`）from `crate::indexer`
- 改 `core/src/server.rs:141`——`CoreService::index` RPC `IndexSession::open(&data_dir, &collection_id)` → `IndexSession::open_with_tokenizer(&data_dir, &collection_id, &resolve_tokenizer())`
- 改 `core/src/jobs/index_session_backend.rs:151`——`IndexSession::open(data, workspace_id)` → `IndexSession::open_with_tokenizer(data, workspace_id, &resolve_tokenizer())`（复用 `crate::server::resolve_tokenizer` 或同 crate 共享 helper；二选一按真实可见性，spec 不预设私有性，实施按编译期 grounding 定）
- **不改**：`core/src/indexer/mod.rs` `IndexSession::open`（:502 库便捷入口续 `DEFAULT_TOKENIZER`）/ `DEFAULT_TOKENIZER` 常量（:183）/ `open_with_tokenizer`（:511，create-vs-open 已安全）/ `reindex_with_tokenizer`（:920，迁移工具已备）/ `RetrieverConfig.tokenizer`（vestigial，ADR-035 D3 schema-driven 对称不改）
- 复测（不改源）`core/examples/phase24_tokenizer_recall.rs`——确认 default `TEXT` vs `code_cjk` recall delta +0.0909（真实数 task §10 + evidence 回填）
- 同源测试：`server.rs` 同源 test 断言 `resolve_tokenizer` env 矩阵 + 生产路径绑定（新建 collection 绑 `code_cjk` / opt-out env 绑 `default` / 既有 `TEXT` collection open 保持 `default`），用 `content_tokenizer_name`（`indexer/mod.rs:1184` 既有 helper）断言

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- jieba `cjk_segmenter` 默认开启（重词典 dep 破 0-dep + Phase 30 实测 delta=0）[SPEC-DEFER:phase-future.cjk-segmenter-default-on]——本 task 默认翻 `code_cjk` 纯 std，jieba 续 feature opt-in
- 既有 collection 升级时自动 reindex 到 `code_cjk`（不自动改用户数据）[SPEC-DEFER:phase-future.tokenizer-auto-reindex-on-upgrade]——用户经既有 `reindex_with_tokenizer` 主动触发
- 大语料 tokenizer recall 基准 [SPEC-DEFER:phase-future.tokenizer-large-corpus-recall]——本 task 复测小 golden +0.0909
- `RetrieverConfig.tokenizer` vestigial 字段真路由 [SPEC-DEFER:phase-future.retriever-config-tokenizer-routing]——ADR-035 D3 已定性 schema-driven 对称，本 task 不改
- Go `[retrieval] tokenizer` config 桥（task-41.2 交付）
- 真实 release tag / run-id / digest（v0.34.0）[SPEC-OWNER:task-41.3-closeout]（ADR-012 用户授权后回填）

## 4. Actors

- 主 agent（ADR-012 自治）
- `resolve_tokenizer`（`core/src/server.rs`，本 task 新增 env-resolution，镜像 `resolve_data_dir` :521-549 / `resolve_vector_backend` :551-560）
- `CoreService::index`（`core/src/server.rs:141`，生产索引 RPC 入口，本 task 改 `open` → `open_with_tokenizer(.., &resolve_tokenizer())`）
- `index_session_backend`（`core/src/jobs/index_session_backend.rs:151`，session backend 生产索引，本 task 同改）
- `open_with_tokenizer`（`core/src/indexer/mod.rs:511`，create 绑新 tokenizer / open 既有走 `open_in_dir` 忽略传入值——本 task 不改，复用其 create-vs-open 安全语义）
- 用户 / 运维（新建 collection 默认获 code/CJK recall 增益；可经 `CONTEXTFORGE_TOKENIZER=default` opt-out；既有 collection 经 `reindex_with_tokenizer` 主动升级）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/server.rs:521-549`（`resolve_data_dir` env-resolution 范式）+ `:551-560`（`resolve_vector_backend` env 范式镜像源）+ `:141`（`CoreService::index` `IndexSession::open` 生产索引入口——本 task 改 `open_with_tokenizer(.., &resolve_tokenizer())`）
- `core/src/jobs/index_session_backend.rs:151`（session backend `IndexSession::open`——本 task 同改）
- `core/src/indexer/mod.rs:183`（`DEFAULT_TOKENIZER = "default"`，不改）+ `:185/:189`（`CODE_CJK_TOKENIZER = "code_cjk"` / `CJK_SEGMENTER_TOKENIZER = "cjk_segmenter"`）+ `:499-556`（`open` → `open_with_tokenizer`，:528-535 create-vs-open meta.json 分支——既有 collection 经 `open_in_dir` 忽略传入 tokenizer，本 task 复用此安全语义）+ `:379`（`register_code_cjk` 无条件注册）+ `:1184`（`content_tokenizer_name` 测试 helper）+ `:920-981`（`reindex_with_tokenizer` 迁移工具，不改）
- `core/examples/phase24_tokenizer_recall.rs` + `docs/spikes/phase-24-tokenizer-recall.md`（default `TEXT` vs `code_cjk` recall delta +0.0909 harness——本 task 复测）
- `docs/decisions/adr-029-*.md §Negative/Follow-ups`（`[SPEC-DEFER:phase-future.tokenizer-default-on]`）+ `adr-035-*.md §D3`（产品决策延后 + `RetrieverConfig.tokenizer` vestigial + 迁移工具已备）+ `adr-046-tokenizer-default-on.md §D1/D3/D4`（本 task 即其原文实现）

### 5.2 关键设计 — env-resolution 翻默认（0 dep / 既有 collection 安全 / 刻意默认变更承接）

- **B1 resolve_tokenizer 翻默认 + opt-out**：`resolve_tokenizer()` 读 `CONTEXTFORGE_TOKENIZER`：unset/"" → `code_cjk`（**翻默认**，新建 collection 默认获 code/CJK recall）；`"default"` → `default`（opt-out 回 legacy `TEXT`，byte-equiv）；`"code_cjk"` → `code_cjk`；`"cjk_segmenter"` → feature 在则 `cjk_segmenter`、缺则 WARN + `code_cjk`；unknown → WARN + `code_cjk`（best-effort 不静默落 `TEXT`，镜像 Phase 35 stderr surfacing）。生产两调用点改 `open_with_tokenizer(.., &resolve_tokenizer())`。
- **B2 既有 collection 安全（复用既有语义）**：`open_with_tokenizer` 对 `meta.json` 存在的 collection 走 `Index::open_in_dir`（读回持久化 schema、忽略传入 tokenizer）→ 既有 `TEXT` collection 仍按 `TEXT` 索引 / 检索（query 侧 schema-driven 对称）；仅 `meta.json` 不存在的新建 collection 绑 `resolve_tokenizer()` 返回值。新建 `code_cjk` collection 的 query 侧 `register_code_cjk` 无条件注册保对称（task-24.1 R4）。
- **B3 recall delta 复测（真实，不预填）**：Phase 24 harness 复测 default `TEXT` vs `code_cjk` recall delta +0.0909（小 golden 实测，本 task 复确认默认化后成出厂基线，真实数 §10 回填、大语料续 `[SPEC-DEFER:phase-future.tokenizer-large-corpus-recall]`）。
- **B4 刻意默认变更承接（据实，非 byte-equiv）**：本 task 使新建 collection 倒排词项 `TEXT`→`code_cjk`——**非 byte-equivalent**，首次刻意默认变更，由 ADR-046 D1/D4 承接；安全边界：既有 collection 安全（B2）+ `CONTEXTFORGE_TOKENIZER=default` opt-out 回 `TEXT`（byte-equiv）+ 既有 collection 不自动迁移（用户经 `reindex_with_tokenizer` 主动）+ Phase 24 +0.0909 justify。spec 据实定性、不夸大（ADR-013）。
- **库 API / 常量不动**：`IndexSession::open`（:502）续走 `DEFAULT_TOKENIZER`（库便捷入口向后兼容）/ `DEFAULT_TOKENIZER` 常量（:183）不改 → 既有 indexer/retriever 单测（`content_tokenizer_name == DEFAULT_TOKENIZER`）不破。

### 5.3 不变量

- 既有 collection 不变（ADR-004 safety intent）：既有 `TEXT` collection 经生产路径 open 后 `content_tokenizer_name` 仍 `default`（`open_in_dir` 读回持久化 schema、忽略传入 tokenizer）；index/query 仍对称（不被静默失效）。
- opt-out → legacy byte-equiv：`CONTEXTFORGE_TOKENIZER=default` → 新建 collection 绑 `TEXT`（与翻默认前一致）。
- 库 API / 常量不变：`IndexSession::open`（:502）/ `DEFAULT_TOKENIZER`（:183）/ `open_with_tokenizer`（:511）/ `reindex_with_tokenizer`（:920）签名 + 语义不变；本 task 只在生产调用点改传入 tokenizer + 新增 `resolve_tokenizer`。
- 0 新代码依赖（ADR-008）：默认翻 `code_cjk` 纯 std；jieba `cjk_segmenter` 仍 feature-gated 不进默认构建；无 Cargo 依赖增量。
- 0 网络：tokenizer 选择是本地索引/检索决策，无网络。
- 首次刻意默认变更（据实，非 byte-equiv）：新建 collection 倒排词项 `TEXT`→`code_cjk`，由 ADR-046 承接 + 三重安全 + Phase 24 实测 justify；spec 据实定性不夸大（ADR-013）。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [x] **AC1**（resolve_tokenizer env 矩阵 + 翻默认 🟢）: `core/src/server.rs` `resolve_tokenizer()` 读 `CONTEXTFORGE_TOKENIZER`：unset/"" → `code_cjk`（翻默认）/ `"default"` → `default`（opt-out）/ `"code_cjk"` → `code_cjk` / `"cjk_segmenter"` → feature 在则 `cjk_segmenter`、缺则 WARN+`code_cjk` / unknown → WARN+`code_cjk`（不静默落 `TEXT`）；生产两调用点（`server.rs:141` + `jobs/index_session_backend.rs:151`）改 `open_with_tokenizer(.., &resolve_tokenizer())` — verified by **TEST-41.1.1**
- [x] **AC2**（生产路径绑定 + 既有 collection 安全 🟢）: 生产路径新建 collection 默认绑 `code_cjk`（`content_tokenizer_name`）；`CONTEXTFORGE_TOKENIZER=default` → 新建 collection 绑 `default`（opt-out legacy）；既有 `TEXT` collection 经生产路径 open 保持 `default`（`open_in_dir` 读回持久化 schema、不被静默改）；`IndexSession::open`/`DEFAULT_TOKENIZER`/既有 indexer/retriever 单测不退化（库 API 不破） — verified by **TEST-41.1.2**
- [x] **AC3**（Phase 24 harness 真实 recall delta 复测 🟡）: `phase24_tokenizer_recall` harness 复测 default `TEXT` vs `code_cjk` recall delta +0.0909（默认化后此增益成出厂基线，小 golden caveat、大语料续 SPEC-DEFER，真实数 §10 回填不预填 ADR-013）；刻意默认变更（非 byte-equiv）由 ADR-046 承接 + 安全边界据实记 — verified by **TEST-41.1.3**
- [x] **AC4**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-41.1.4**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-41.1.1 | `resolve_tokenizer` env 矩阵：unset→`code_cjk`（翻默认）/ `"default"`→`default`（opt-out）/ `"code_cjk"`→`code_cjk` / `"cjk_segmenter"`→feature 在则 `cjk_segmenter`/缺则 WARN+`code_cjk` / unknown→WARN+`code_cjk`（不静默落 `TEXT`）；生产两调用点改 `open_with_tokenizer(.., &resolve_tokenizer())` | `core/src/server.rs`（同源 test） | Done |
| TEST-41.1.2 | 生产路径绑定 + 既有 collection 安全：生产路径新建 collection 绑 `code_cjk` / `CONTEXTFORGE_TOKENIZER=default` → 绑 `default` / 既有 `TEXT` collection open 保持 `default`（`content_tokenizer_name` 断言）；`IndexSession::open`/`DEFAULT_TOKENIZER`/既有单测不退化 | `core/src/server.rs` / `core/src/indexer/mod.rs`（既有 test 回归） | Done |
| TEST-41.1.3 | Phase 24 harness 真实 recall delta：default `TEXT` vs `code_cjk` recall delta +0.0909（复测确认默认化后成出厂基线，小 golden caveat，真实数回填不预填） | `core/examples/phase24_tokenizer_recall.rs`（复测，不改源） | Done |
| TEST-41.1.4 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（高）翻默认破既有 collection / 静默失效**：若翻默认令既有 `TEXT` collection 被改用 `code_cjk` 索引或 query 侧不对称，则既有召回静默退化。
  - **缓解**：复用 `open_with_tokenizer` 既有 create-vs-open 语义（`meta.json` 存在 → `open_in_dir` 读回持久化 schema、忽略传入 tokenizer）；query 侧 `register_code_cjk` 无条件注册保对称；TEST-41.1.2 断言既有 `TEXT` collection open 后 `content_tokenizer_name` 仍 `default`。stop-condition：既有 collection 被静默改 / query 不对称则 AC2 不标 `[x]`。
- **R2（高）刻意默认变更被误记 byte-equiv / 未由 ADR 承接**：本 task 是首次刻意改默认，若沿用「byte-equiv」措辞则不诚实、未由 ADR-046 承接则违治理。
  - **缓解**：spec §1 B4 / §5.2 B4 / §5.3 + ADR-046 D1/D4 据实定性「非 byte-equiv、首次刻意默认变更」+ 显式承接（三重安全 + Phase 24 justify）；ADR-004 opt-out byte-equiv + 不自动迁移 safety intent 保持。stop-condition：夸大为 byte-equiv / 未由 ADR-046 承接则越界（ADR-013）。
- **R3（中）recall delta 预填 / 夸大小语料**：+0.0909 是小 golden 实测，预填 / 夸大违 ADR-013。
  - **缓解**：标 +0.0909 为 Phase 24 小 golden 实测、本 task 复确认；真实数 §10 + evidence 回填；大语料续 `[SPEC-DEFER:phase-future.tokenizer-large-corpus-recall]`。stop-condition：预填 / 夸大为大基准则 AC3 不标 `[x]`。
- **R4（中）默认翻 jieba 破 0-dep**：默认翻 `cjk_segmenter` 引入重 dep 破 ADR-008。
  - **缓解**：默认翻 **`code_cjk`（纯 std）**；jieba 仅 feature 在时 passthrough、缺则 WARN+`code_cjk`；0 新 dep。stop-condition：默认构建引入 jieba dep 则 AC1 不标 `[x]`。
- **R5（中）resolve_tokenizer 未知值静默落 TEXT 抵消翻默认**：未知 env 值若静默落 `TEXT` 则翻默认形同未生效。
  - **缓解**：未知 / feature-off → stderr WARN + 回落 `code_cjk`（不静默落 `TEXT`，镜像 Phase 35 surfacing）；TEST-41.1.1 断言。stop-condition：未知值静默落 `TEXT` 则 AC1 不标 `[x]`。
- **R6（低）两调用点 resolve_tokenizer 共享可见性**：`jobs/index_session_backend.rs` 复用 `server::resolve_tokenizer` 须可见（pub(crate) 或共享 helper）。
  - **缓解**：实施按编译期 grounding 定可见性（`pub(crate) fn resolve_tokenizer` 或移入共享 mod）；`cargo build` 守编译。stop-condition：编译失败则 AC1 不标 `[x]`。

## 9. Verification Plan

```bash
# 1. AC1 — resolve_tokenizer env 矩阵（unset→code_cjk / default→default / unknown→WARN+code_cjk）
cargo test -p contextforge-core server::

# 2. AC2 — 生产路径绑定 + 既有 collection 安全（新建绑 code_cjk / opt-out 绑 default / 既有 TEXT 保持）
cargo test -p contextforge-core server::
cargo test -p contextforge-core indexer::

# 3. AC3 — Phase 24 harness real recall delta +0.0909（默认化后成出厂基线）
cargo run -p contextforge-core --example phase24_tokenizer_recall

# 4. 不退化（全量）
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# 5. AC4 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.tokenizer-default-on-defer-note]：本 task 交付 production tokenizer 默认翻 `code_cjk`（resolve_tokenizer env-resolution + 生产两调用点接线 + env opt-out），🟢 可单测 / 🟡 本地 real recall delta，0 新 dep（`code_cjk` 纯 std）。jieba `cjk_segmenter` 默认开启 `[SPEC-DEFER:phase-future.cjk-segmenter-default-on]`（重 dep + Phase 30 delta=0）、既有 collection 自动迁移 `[SPEC-DEFER:phase-future.tokenizer-auto-reindex-on-upgrade]`（不自动改用户数据）、大语料 recall `[SPEC-DEFER:phase-future.tokenizer-large-corpus-recall]`、`RetrieverConfig.tokenizer` 真路由 `[SPEC-DEFER:phase-future.retriever-config-tokenizer-routing]`（ADR-035 D3 schema-driven 对称）、Go `[retrieval]` config 桥（task-41.2）均不在本 task 范围。本 task 是首次刻意默认变更（新建 collection 非 byte-equiv），由 ADR-046 承接 + 既有 collection 安全 + opt-out + 不自动迁移；recall delta +0.0909 系 Phase 24 小 golden 实测（据实声明，ADR-013 不夸大、不预填）；实测产物（v0.34.0）真实跑出后回填。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done

**§9 Verification（PR #262，master @ `35bb421`，真实证据）**：
- AC1：`cargo test -p contextforge-core --lib test_41_1_1` —— `test_41_1_1_parse_tokenizer_flips_default_with_opt_out` PASS（env 矩阵 unset/""→code_cjk 翻默认 / "default"→DEFAULT_TOKENIZER opt-out / code_cjk→code_cjk / unknown→WARN+code_cjk 不落 TEXT / cjk_segmenter feature 守护）；生产两调用点（`server.rs:141` + `jobs/index_session_backend.rs:151`）改 `open_with_tokenizer(.., &resolve_tokenizer())`。
- AC2：`cargo test -p contextforge-core --lib test_41_1_2` —— `test_41_1_2_production_default_flip_and_existing_collection_safe` PASS（生产路径新建 collection 绑 code_cjk + camel 子词 'user' 命中 / opt-out env 绑 default + 子词 miss / 既有 TEXT collection 经翻默认 open 仍保持 default + 子词仍 miss）；`IndexSession::open`/`DEFAULT_TOKENIZER`/既有 indexer·retriever 单测不退化（`cargo test --lib` 222 passed = 220 baseline + 2）。
- AC3：`cargo run -p contextforge-core --example phase24_tokenizer_recall` —— **实测 default(TEXT) recall@5/@10=0.8750 mrr=0.8750 → after(code_cjk) recall@5/@10=1.0000 mrr=0.9375，delta recall@5/@10=+0.1250 mrr=+0.0625**（over 当前 16-题 golden / 14 file；与 ADR-035 Amendment D4 测量 delta(seg−default)=+0.1250 一致；ADR-029 §Negative 的 +0.0909 系 Phase 24 原始 11-题 golden，golden 自 Phase 24 已增长 → 据实记当前 +0.1250 不沿用旧数，ADR-013）。
- AC4：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威，PR #262 spec-lint pass）。
- 0 新 dep（`code_cjk` 纯 std；jieba `cjk_segmenter` 仍 feature opt-in）/ 0 网络 / 既有 collection 安全 / opt-out byte-equiv / 首次刻意默认变更据实定性非 byte-equiv（由 ADR-046 承接）；`cargo clippy --all-targets -- -D warnings` clean。

**实际改动文件**（PR #262）：
- `core/src/server.rs`——add `resolve_tokenizer()`（pub fn）+ `parse_tokenizer()`（pub(crate) 纯函数）+ import `CODE_CJK_TOKENIZER`/`DEFAULT_TOKENIZER`/`CJK_SEGMENTER_TOKENIZER` + `CoreService::index`（:141）改 `open_with_tokenizer(.., &resolve_tokenizer())` + TEST-41.1.1。
- `core/src/jobs/index_session_backend.rs`——`IndexSession::open`（:151）改 `open_with_tokenizer(.., &crate::server::resolve_tokenizer())`。
- `core/src/indexer/mod.rs`——TEST-41.1.2（生产路径绑定 + 既有 collection 安全，用 `content_tokenizer_name` + `parse_tokenizer`）。
- `core/examples/phase24_tokenizer_recall.rs`——复测（未改源），实测 recall delta +0.1250。
- `docs/decisions/adr-029-*.md` + `adr-035-*.md` 默认开启维度 / D3 产品决策兑现 add-only Amendment 落点在 task-41.3 closeout（非本 task body，已落地）。
