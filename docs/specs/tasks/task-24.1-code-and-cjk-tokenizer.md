# Task `24.1`: `code-and-cjk-tokenizer — core/src/indexer/mod.rs 注册自定义 code/CJK TextAnalyzer，opt-in（config）时 content 字段按 camelCase/snake_case/dotted.path/kebab-case 拆分（保留原 token）+ CJK bigram 分词；默认 tokenization 不变（向后兼容，既有索引不失效）+ deterministic 分词单测`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 24 (retrieval-tokenizer-and-eval-hardening)
**Dependencies**: task-2.4（`build_tantivy_schema` + `content` 字段 `TEXT | STORED` + `IndexSession` + `tantivy_search`）/ task-4.1（`RetrieverConfig.tokenizer` 接入点字段，当前恒 `"default"`）/ ADR-029 D1（tokenizer opt-in + 向后兼容）/ ADR-004（local-first：默认 0 新 dep / opt-in）/ ADR-008（core-library-selection，依赖变更 add-only）/ ADR-013（禁伪造凭据）/ ADR-014 D1-D5（第十五次激活）

## 1. Background

`core/src/indexer/mod.rs:145-157` 的 `build_tantivy_schema` 把 Tantivy 5 字段 schema 的 `content` 字段建为 `sb.add_text_field("content", TEXT | STORED)`——`TEXT` 用 Tantivy 默认 analyzer（`SimpleTokenizer` + lowercase filter，按非字母数字边界切词）。该默认切分对代码符号偏弱：`camelCase` 整体当一个 token（不拆 `camel`+`case`），`snake_case` / `dotted.path` / `kebab-case` 的子词无法独立命中，CJK 文本（无空格分隔）被整段当一个 token。`core/src/indexer/mod.rs:447` 的 `tantivy_search` 查询侧用 `QueryParser::for_index(&self.tantivy_index, vec![self.f_content])`，同样走 `content` 字段的 analyzer——index 与 query 分词须对称才能命中。

`core/src/retriever/mod.rs:96-115` 的 `RetrieverConfig` 已有 `tokenizer: String` 字段（task-4.1 §5 留的接入点，`Default` 恒 `"default"`），`mod.rs:1127-1131` 测试注「open_with_config 接入点：自定义 tokenizer 名 ... CJK 留接入点（PRD §O11）」——接入点已留但分词逻辑未落地。`docs/roadmap.md` §4 把该项列为 backlog marker `检索 tokenizer: cjk-and-code-tokenizer`（承 `phase-19` §2）。

本 task 在 `core/src/indexer/mod.rs` 注册一个自定义 code/CJK 感知的 Tantivy `TextAnalyzer`，对 `content` 字段在 **opt-in（config）时** 生效（代码符号拆分 + 保留原 token + CJK bigram），默认时维持 `TEXT` 默认 analyzer（向后兼容，既有 collection 索引不被静默失效）。tokenizer 改进的真实 before/after recall delta 在 closeout（task-24.3）据扩充 golden 数据集实测，不在本 task 内预判跨语料召回数值（ADR-013）。

## 2. Goal

`core/src/indexer/mod.rs` 注册自定义 code/CJK `TextAnalyzer` 并在 opt-in 时绑到 `content` 字段：(a) **代码符号拆分**——`camelCase` → `camel` + `case`（+ 保留原 token `camelCase`）、`snake_case` → `snake` + `case`（+ 原 token）、`dotted.path` → `dotted` + `path`（+ 原 token）、`kebab-case` → `kebab` + `case`（+ 原 token）；(b) **CJK bigram**——CJK 字符段切 bigram（如 `配置加载` → `配置` / `置加` / `加载`），让无空格 CJK 文本可子串命中；(c) **opt-in 向后兼容**——自定义 analyzer 经 config 选用初始启用，默认时 `content` 仍走 `TEXT` 默认 analyzer，既有索引不失效；opt-in 切换改倒排词项，需 re-index 才生效，该含义在本 spec §5.2 + closeout release docs 文档化；(d) **index/query 对称**——index 侧 analyzer 名与 query 侧 tokenizer 名（`RetrieverConfig.tokenizer` 接入点）一致。≥3 Rust 测试全 PASS：代码符号拆分（camelCase/snake_case/dotted.path/kebab-case + 保留原 token）+ CJK bigram + 默认 tokenization 不变。默认构建 0 新 dep（优先 std-only CJK bigram + Tantivy 自带 filter 组合）；`cargo test --workspace` 不退化。D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **修改 `core/src/indexer/mod.rs`**：注册自定义 `TextAnalyzer`（如经 `Index::tokenizers().register(name, analyzer)`）；自定义 analyzer 由「code-symbol splitter（保留原 token + 拆 camelCase/snake_case/dotted.path/kebab-case 子词）+ CJK bigram + lowercase」组合而成；`build_tantivy_schema` / `IndexSession::open` 在 opt-in（config 标志 / tokenizer 名）时把 `content` 字段索引选项绑到自定义 analyzer 名，默认时维持 `TEXT` 默认 analyzer（向后兼容）。
- **接通 query 侧对称分词**：`tantivy_search`（或检索热路径）查询侧 tokenizer 名与 index 侧 analyzer 名一致（消费 `RetrieverConfig.tokenizer` 接入点）——opt-in 时 query 也走自定义 analyzer，保 index/query 分词对称。
- **新增同源 Rust 单测（`core/src/indexer/mod.rs` 内 `#[cfg(test)] mod tests` 或 `core/tests/`）**：(a) 代码符号拆分——`camelCase` / `getUserById` / `user_id` / `pkg.module.func` / `kebab-case-name` 经自定义 analyzer 拆出预期子 token 集 + 保留原 token（直接断言 token stream，确定性）；(b) CJK bigram——`配置加载` 等 CJK 输入拆出预期 bigram 序；(c) 默认 tokenization 不变——未 opt-in 时 `content` 索引/检索行为与现状等价（既有 `tantivy_search` 命中不退化）。
- **可选修改 `core/Cargo.toml`**：仅当 CJK 分词确证 std-only 不可行需分词依赖时——按 add-only 评估，依赖变更经主 agent R7 chore（subagent 不自改 Cargo.toml），优先 std-only / Tantivy 自带 analyzer 组合（ADR-004 / ADR-008）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **eval 数据集校验器 + golden 数据集代码/CJK 扩充** [SPEC-OWNER:task-24.2-eval-dataset-hardening]：本 task 落 tokenizer，扩充能 exercise 它的 golden 数据集在 task-24.2。
- **tokenizer 真实 before/after recall delta 实测** [SPEC-OWNER:task-24.3-closeout-v0.17.0]：本 task 落分词正确性单测；真实跨语料召回 delta 在 closeout 据 task-24.2 扩充 golden 实测（不在本 task 预判召回数，ADR-013）。
- **CJK 真正分词器（词典 / 统计分词替 bigram）** [SPEC-DEFER:phase-future.cjk-true-segmenter]：本 task 用 bigram 务实起步。
- **tokenizer 从 opt-in 转默认开启 + 既有索引迁移工具** [SPEC-DEFER:phase-future.tokenizer-default-on]：本 task opt-in 不破坏既有索引；默认开启 + 迁移属后续。
- **rust-native-eval-runner promote** [SPEC-DEFER:phase-future.rust-native-eval-runner]：与本 task 分词无关，runner 评估在 task-24.3。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`core/src/indexer/mod.rs::build_tantivy_schema` / `IndexSession`**：task-2.4 indexer，本 task 注册自定义 analyzer + opt-in 绑 `content` 字段。
- **`core/src/indexer/mod.rs::tantivy_search`**：查询侧分词，本 task 接 opt-in 对称 analyzer。
- **`core/src/retriever/mod.rs::RetrieverConfig::tokenizer`**：task-4.1 预留的 tokenizer 名接入点，本 task 首次据其选 analyzer。
- **Tantivy `TextAnalyzer` / `TokenFilter` / `Tokenizer`（0.26.1）**：分词框架，本 task 核实其自定义 analyzer 注册 + token filter 组合面。
- **下游 task-24.2 / task-24.3**：task-24.2 扩充 exercise 本 tokenizer 的 golden 数据集；task-24.3 据本 tokenizer + task-24.2 数据集实测真实 recall delta。

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/indexer/mod.rs:145-157`（`build_tantivy_schema` — `content` 字段 `TEXT | STORED` 默认 analyzer）+ `core/src/indexer/mod.rs:184-222`（`IndexSession::open` — `Index::create_in_dir` / `open_in_dir` + writer 生命周期）+ `core/src/indexer/mod.rs:440-462`（`tantivy_search` — `QueryParser::for_index(&index, vec![content])` 查询侧分词）+ `core/src/indexer/mod.rs:487-563`（`write_chunks` — `content` 字段 `add_document` 写入点）
- `core/src/indexer/mod.rs:617-720`（既有 `tantivy_search` 命中单测 TEST-2.4.2/2.4.3 — 默认 analyzer 行为基线，本 task「默认不变」单测对照）
- `core/src/retriever/mod.rs:96-115`（`RetrieverConfig.tokenizer` 字段 + `Default` 恒 `"default"`）+ `core/src/retriever/mod.rs:1059-1131`（task-4.1 tokenizer 接入点测试 + 「CJK 留接入点 PRD §O11」注）
- `core/Cargo.toml:69`（`tantivy = "0.26.1"`）+ Tantivy 0.26 `TextAnalyzer` / `Tokenizer` / `TokenFilter` / `Index::tokenizers().register` API（核实自定义 analyzer 注册 + token filter 链 + lowercase / CJK 自带支持）
- `docs/decisions/adr-029-code-and-cjk-tokenizer-and-eval-hardening.md` D1 + D5 + `docs/decisions/adr-004-local-first-privacy-baseline.md`（默认 0 新 dep / opt-in）+ `docs/decisions/adr-008-core-library-selection.md`（依赖选型 add-only）+ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造）

### 5.2 关键设计 — 自定义 code/CJK analyzer + opt-in 向后兼容

- **analyzer 组合**：自定义 `TextAnalyzer` = code-symbol splitter（保留原 token + 拆边界子词）+ CJK bigram + lowercase。
  - **代码符号拆分**：识别 camelCase 大小写边界（`camelCase`→`camel`/`case`）+ `_` / `.` / `-` 分隔符（`snake_case`→`snake`/`case`、`dotted.path`→`dotted`/`path`、`kebab-case`→`kebab`/`case`），**同时保留原 token**（`camelCase` / `snake_case` 原样也入倒排），让原样查询与子词查询都命中。
  - **CJK bigram**：识别 CJK Unicode 区段（CJK Unified Ideographs 等）的连续字符段，切 bigram（`配置加载`→`配置`/`置加`/`加载`），非 CJK 段走代码符号 + 空白切分。优先 std-only（Rust `char` 迭代 + Unicode 区段判定 + 滑窗 bigram，0 新 dep）。
- **opt-in 向后兼容（ADR-004 D1）**：自定义 analyzer 经 config（tokenizer 名非 `"default"` / opt-in 标志）选用初始启用；默认时 `content` 字段维持 `TEXT` 默认 analyzer——**既有 collection 索引不被静默失效**。opt-in 切换改 `content` 倒排词项，**需 re-index** 既有 collection 才生效（旧索引仍可用默认 analyzer 检索，但不享受代码/CJK 子词命中）；该 re-index 含义在本 spec + closeout release docs 明确文档化。
- **index/query 对称**：index 侧绑 `content` 的 analyzer 名 = query 侧 `QueryParser` 用的 tokenizer 名（消费 `RetrieverConfig.tokenizer` 接入点）；opt-in 时二者同为自定义 analyzer 名，保分词对称（否则 query token 与倒排 token 不匹配 → 召回退化）。
- **ADR-013**：分词正确性（代码符号拆分 + 保留原 token + CJK bigram）是 deterministic 单测可验证项（直接断言 token stream）；不预判跨语料召回数值（真实 recall delta 在 task-24.3 据扩充 golden 实测）。

### 5.3 不变量

- 默认构建 0 新 dep（优先 std-only CJK bigram + Tantivy 自带 filter 组合）；若引入分词依赖须经主 agent R7 chore + ADR-008 add-only（subagent 不自改 Cargo.toml）。
- 未 opt-in（默认 tokenizer）时 `content` 索引/检索与现状逐字节等价（既有 `tantivy_search` 命中不退化，TEST-2.4.2/2.4.3 行为不变）。
- opt-in 时代码符号 analyzer 保留原 token：`camelCase` 既拆 `camel`/`case` 又保留 `camelCase`，原样查询不退化。
- index 侧与 query 侧 tokenizer 名一致（分词对称），opt-in 不破坏 index/query 匹配。
- 不改 task-2.4 SQLite 3 表 schema + Tantivy 5 字段 schema 结构（`chunk_id`/`content`/`file_path`/`language`/`line_start`/`line_end`）；仅改 `content` 字段的 analyzer 绑定（opt-in），不改字段集。

## 6. Acceptance Criteria

- [ ] **AC1**: opt-in 下自定义 analyzer 代码符号拆分 — `camelCase` / `getUserById` / `user_id` / `pkg.module.func` / `kebab-case-name` 经自定义 analyzer 拆出预期子 token 集（camel→camel+case 等）**且保留原 token**（确定性 token stream 断言）— verified by **TEST-24.1.1**
- [ ] **AC2**: opt-in 下 CJK bigram — `配置加载` 等 CJK 输入经自定义 analyzer 拆出预期 bigram 序（`配置`/`置加`/`加载`）；非 CJK 段走代码符号/空白切分（混合输入正确）— verified by **TEST-24.1.2**
- [ ] **AC3**: 默认 tokenization 不变（向后兼容）— 未 opt-in（默认 tokenizer）时 `content` 索引/检索与现状等价（既有 `tantivy_search` 命中不退化）；自定义 analyzer 不改 task-2.4 5 字段 schema 结构 — verified by **TEST-24.1.3**
- [ ] **AC4**: index/query 对称 + opt-in 命中 — opt-in 时 index 侧 analyzer 名 = query 侧 tokenizer 名（`RetrieverConfig.tokenizer` 接入点）；opt-in 索引后代码符号子词查询（如 `getuserbyid` 查 `getUserById` 内容）命中（确定性，单 doc 索引→查询）— verified by **TEST-24.1.4**
- [ ] **AC5**: 既有不退化 — 默认 `cargo test --workspace` 全 PASS + 0 新依赖（std-only / Tantivy 自带组合，无 Cargo.toml dep 变更则无 R7）；`go test ./...` 不受影响（本 PR 零 Go delta）— verified by **TEST-24.1.5** + §10 实测
- [ ] **AC6**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-24.1.6** + §10 记录

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-24.1.1 | 代码符号拆分 camelCase/snake_case/dotted.path/kebab-case + 保留原 token | `core/src/indexer/mod.rs`（`mod tests`）或 `core/tests/` | Planned |
| TEST-24.1.2 | CJK bigram 分词（`配置加载`→bigram 序）+ 混合输入正确 | `core/src/indexer/mod.rs`（`mod tests`）或 `core/tests/` | Planned |
| TEST-24.1.3 | 默认 tokenization 不变（未 opt-in 既有 tantivy_search 命中不退化）+ schema 结构不变 | `core/src/indexer/mod.rs`（`mod tests`） | Planned |
| TEST-24.1.4 | index/query 对称 + opt-in 代码符号子词查询命中（单 doc roundtrip） | `core/src/indexer/mod.rs`（`mod tests`）或 `core/tests/` | Planned |
| TEST-24.1.5 | 默认 `cargo test --workspace` 0 failed + 0 新依赖 + 零 Go delta | 全 Rust | Planned |
| TEST-24.1.6 | D2 lint `--touched origin/master` 0 未标注命中 | `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（中）Tantivy 0.26 自定义 analyzer / CJK 无现成 std 路径**（承 phase-24 §7 R1）：Tantivy 自带 analyzer 未必含 CJK bigram；纯 std bigram 需自写。
  - **缓解**：先核实 Tantivy 0.26 `TextAnalyzer` 注册 + `TokenFilter` 链面；CJK 优先自写 std-only bigram（`char` 迭代识别 CJK Unicode 区段 + 滑窗 bigram，0 新 dep）+ 组合 Tantivy 自带 lowercase filter。stop-condition：若自定义 analyzer 注册与分词单测均不可行则记录受阻态，AC1/AC2 不标 `[x]`（ADR-013 不伪造分词通过）。
- **R2（低）分词依赖引入新供应链表面**（如 CJK 分词 crate）：default build 须 0 新依赖。
  - **缓解**：优先 std-only + Tantivy 自带组合（默认 0 新 dep，ADR-004）；确证不可行才经主 agent R7 chore + ADR-008 add-only，subagent 不自改 Cargo.toml。
- **R3（中）opt-in 切换静默失效既有索引**：opt-in 改倒排词项，既有索引未 re-index 时新旧分词不一致。
  - **缓解**：默认 tokenization 不变（既有索引默认走默认 analyzer，不被动失效）；opt-in 的 re-index 含义在 §5.2 + closeout release docs 文档化（ADR-004 向后兼容），AC3 含「默认不变」单测覆盖。
- **R4（低）index/query 分词不对称导致召回退化**：opt-in 仅 index 侧绑自定义 analyzer，query 侧仍默认 → token 不匹配。
  - **缓解**：AC4 显式断言 index/query tokenizer 名一致 + opt-in 子词查询命中（单 doc roundtrip 覆盖对称性）。

## 9. Verification Plan

```bash
# Rust：默认构建（未 opt-in）0 新依赖 + 不退化
cargo test --workspace

# 自定义 analyzer 分词单测（代码符号 + CJK bigram + 默认不变 + opt-in 命中）
cargo test -p contextforge-core indexer

# Go 不退化（本 PR 零 Go delta）
go test ./...

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **Status**: 待实施（Draft）。实施完成后按 6 项回填：完成日期 / 改动文件 / commit 列表 / §9 Verification 实测结果（ADR-013 真实非合成）/ 设计取舍（analyzer 组合 + std-only vs 依赖 + opt-in 向后兼容 + Tantivy 0.26 自定义 analyzer 注册面核实结论）/ 剩余风险 + 下游影响（task-24.2 扩充 golden exercise 本 tokenizer / task-24.3 据本 tokenizer 实测 recall delta + CJK 真正分词器与 tokenizer 默认开启延后项）。
