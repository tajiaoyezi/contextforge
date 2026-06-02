# Task `30.1`: `cjk-true-segmenter — feature-gated 真分词器 analyzer（cjk-segmenter，默认 0-dep）+ 并行 analyzer 名 + 双站点注册（open_with_tokenizer:442 / open_with_config:250）+ deterministic 分词单测（配置加载 → 配置/加载，区别于 bigram 配置/置加/加载）`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 30 (cjk-true-segmenter)
**Dependencies**: Phase 24（retrieval-tokenizer-and-eval-hardening，Done——`CodeCjkTokenizer` 代码符号拆分 + CJK overlapping bigram seam）/ `core/src/indexer/mod.rs`（`build_code_cjk_analyzer:364-369` / `register_code_cjk:373-377` / `open_with_tokenizer:442`）/ `core/src/retriever/mod.rs`（`open_with_config:250` 注册站点 + `RetrieverConfig.tokenizer:99` vestigial）/ `core/Cargo.toml`（feature-gating pattern :115-132）/ ADR-035（cjk-true-segmenter-and-tokenizer-default，§D1+§D2——本 task 实现即其原文意图）/ ADR-029（code-and-cjk-tokenizer-and-eval-hardening，bigram seam 起源，add-only Amendment 属 task-30.3）/ ADR-004（local-first-privacy-baseline，默认构建仍 0 新 dep）/ ADR-008（dependency-governance，新 optional dep jieba-rs/lindera 经 main-agent R7 chore add-only——本 task 为规划契约层标注，dep chore 由 main agent 执行、不自改 Cargo.toml）/ ADR-012（main-agent-governance-autonomy）/ ADR-013（禁伪造红线，分词单测确定性、不预填 recall）/ ADR-014 D1-D5（第二十一次激活）

## 1. Background

Phase 24（ADR-029 Accepted）为 `content` 字段引入 opt-in 自定义 `TextAnalyzer`：`CodeCjkTokenizer`（`core/src/indexer/mod.rs`）做代码符号拆分（保留原 token + 拆 `_ . -` + camelCase）+ **CJK overlapping bigram**——`tokenize_code_cjk:282-322` 的 bigram 循环（:290-308）把「配置加载」切成 `配置/置加/加载`（相邻字滑窗双字），**不是真分词器**。真分词器（jieba-rs / lindera）会按词典切出词边界 `配置/加载`（无 `置加` 噪声 token），召回精度更高、phrase 命中更干净。

Phase 24 为守 ADR-004 0-dep 基线、不引词典依赖，**务实选用 std-only bigram 起步**，并把「真分词器升级」与「tokenizer 默认开启」两项显式延后（provenance：`docs/decisions/adr-029-...:54`/`:66` + `docs/specs/phases/phase-24-...:41`/`:42`/`:125` + `task-24.1:35`/`:36` + `task-24.3:39`/`:40`）。本 task 兑现其中第一项——在 **feature gate 后**引入真分词器 analyzer，默认仍 0-dep。

## 2. Goal

在新 core feature `cjk-segmenter`（默认 off → 0 新 dep，gating 镜像 `core/Cargo.toml` 的 `vector-lancedb:120`）后，新增一个**真 CJK 词分词 analyzer**，采**并行 analyzer 名**策略（保留 bigram 作 0-dep fallback，真分词作 feature 升级）：

- 新增 `CJK_SEGMENTER_TOKENIZER = "cjk_segmenter"` 常量（与 `CODE_CJK_TOKENIZER = "code_cjk":183` 并列）+ 一个 `#[cfg(feature = "cjk-segmenter")]` 构建 analyzer 函数（与 `build_code_cjk_analyzer:364-369` 并列）。
- 该 analyzer 名**必须在两站点同时注册**：index 站点 `IndexSession::open_with_tokenizer`（`core/src/indexer/mod.rs:442`，现注册 `register_code_cjk`）+ query 站点 `Retriever::open_with_config`（`core/src/retriever/mod.rs:250`，同样注册 `register_code_cjk`）。新 analyzer 名漏注册任一站点 → `QueryParser::for_index` 据 schema 字段绑定解析 analyzer 时**静默失败 → 召回退化**（task-24.1 R4）。
- 分词库（jieba-rs vs lindera）选型 + optional dep 经 **main-agent R7 chore + ADR-008 add-only**——本 task **仅规划标注，不自改 `core/Cargo.toml`**。
- deterministic 分词单测断言**真词边界**：`配置加载 → 配置/加载`（与 bigram 的 `配置/置加/加载` 可区分）。

pass bar：`cargo test --features cjk-segmenter` 下真分词单测绿（确定性、无 live dep）；默认构建（无 `cjk-segmenter`）默认分词 + 6 字段 schema + 0 新 dep 不变；index/query 双站点注册对称、opt-in collection round-trip 命中无静默退化；D2 lint 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- `core/src/indexer/mod.rs`——(a) 新增常量 `CJK_SEGMENTER_TOKENIZER = "cjk_segmenter"`（与 `:183` 并列）；(b) `#[cfg(feature = "cjk-segmenter")]` 构建真分词 `TextAnalyzer` 函数（真分词 tokenizer + `RemoveLongFilter(40)` + `LowerCaser`，filter 链与 `build_code_cjk_analyzer:364-369` 一致，仅底层 tokenizer 换真分词）；(c) 一个 `register_cjk_segmenter(index)`（feature-gated；默认构建为 no-op 或不存在符号——不引用即无副作用，镜像 `register_code_cjk:373-377`）。
- `core/src/retriever/mod.rs`——query 站点 `open_with_config:250` 同步注册真分词 analyzer（feature-gated），与 index 站点 `open_with_tokenizer:442` 对称。
- deterministic 分词单测（`#[cfg(feature = "cjk-segmenter")]`）——断言真词边界 token stream（`配置加载 → 配置/加载`，distinct from bigram `配置/置加/加载`）+ index/query 双站点注册对称 round-trip。
- 默认构建无回归单测——默认分词 + 6 字段 schema + `cargo test --workspace` 不受影响。
- **规划标注（不实施）**：`cjk-segmenter = ["dep:jieba-rs"]`（或 lindera）feature 行 + optional dep——经 main-agent R7 chore + ADR-008 add-only，**本 task 不自改 `core/Cargo.toml`**。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- optional dep（jieba-rs / lindera）的实际加入 `core/Cargo.toml` + `Cargo.lock` pin——经 main-agent R7 chore + ADR-008 add-only [SPEC-OWNER:task-30.1-r7-dep-chore]（本 task 仅规划契约层，dep chore 由 main agent 执行）。
- tokenizer 默认开启评估 + 既有索引 reindex/migration 工具 + `RetrieverConfig.tokenizer:99` 路由接线 [SPEC-OWNER:task-30.2-tokenizer-default-on-and-cjk-recall-delta]。
- 扩展 CJK golden + 真实 recall delta 实测 [SPEC-OWNER:task-30.2-tokenizer-default-on-and-cjk-recall-delta]。
- v0.23.0 closeout（smoke / docs / ADR ratify / ADR-029 Amendment / phase Status）[SPEC-OWNER:task-30.3-closeout-v0.23.0]。
- 韩文/日文专用词典精调、用户自定义词典加载 [SPEC-DEFER:phase-future.cjk-custom-dict]。

## 4. Actors

- 主 agent（ADR-012 自治；optional dep R7 chore 由 main agent 执行，非 subagent 自改）
- `core/src/indexer/mod.rs`（新 `cjk_segmenter` analyzer 构建 + index 站点 `:442` 注册）
- `core/src/retriever/mod.rs`（query 站点 `:250` 对称注册 + `RetrieverConfig.tokenizer:99` 现 vestigial）
- Tantivy `TextAnalyzer` / `TokenizerManager`（analyzer 名按 schema 字段绑定解析；双站点注册对称是召回正确性前提）
- 真分词库 jieba-rs / lindera（feature-gated；默认构建不编译）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/indexer/mod.rs:364-369`（`build_code_cjk_analyzer`——新 `build_cjk_segmenter_analyzer` 镜像此 filter 链：tokenizer + `RemoveLongFilter::limit(40)` + `LowerCaser`）+ `:373-377`（`register_code_cjk`——新 `register_cjk_segmenter` 镜像此注册形）+ `:442`（`open_with_tokenizer` 的 index 站点注册调用）
- `core/src/retriever/mod.rs:250`（`open_with_config` 的 query 站点注册调用——与 `:442` 对称；`QueryParser::for_index` 据 schema 字段绑定解析 analyzer，漏注册即静默失败 → task-24.1 R4 召回退化）
- `core/Cargo.toml:115-132`（feature 块；`default = []` :116；`vector-lancedb = [dep:lancedb,...]` :120；`embedding-remote = [dep:ureq]` :131；optional dep 模式 ——新 `cjk-segmenter` feature + optional dep 镜像此 recipe，默认构建仍 `default = []` → 0 新 dep）
- 对照参考：`tokenize_code_cjk:282-322`（bigram 循环 :290-308：`配置加载 → 配置/置加/加载`）+ `is_cjk:186-196`（CJK 区段判定，真分词器复用同范围识别 CJK run）+ `CODE_CJK_TOKENIZER = "code_cjk":183` / `DEFAULT_TOKENIZER = "default":181`（常量并列处）+ `build_tantivy_schema:155`（opt-in 分支）/`:162`（默认分支）

### 5.2 关键设计 — 并行 analyzer 名 + 双站点注册对称（schema-binding 驱动）

- **并行 analyzer 名（ADR-035 §D2）vs in-place 替换**：采**新增** `CJK_SEGMENTER_TOKENIZER = "cjk_segmenter"` 并列名 + 新构建/注册函数，**保留** bigram `code_cjk` 作默认 0-dep fallback、真分词作 feature 升级。备选「在 `build_code_cjk_analyzer:364-369` 内 in-place 把 bigram 换真分词」会**丢失 0-dep fallback**（默认构建无真分词库时 `code_cjk` analyzer 不可构建）——故不采。
- **双站点注册对称（召回正确性前提）**：analyzer 名由 schema 字段绑定（meta.json 持久化）+ tokenizer manager 注册名解析。新 `cjk_segmenter` 名**必须**在 index 站点 `open_with_tokenizer:442` 与 query 站点 `open_with_config:250` **同时注册**；漏注册 query 站点 → `QueryParser::for_index`（`core/src/retriever/mod.rs:325-328`）解析时找不到 analyzer → 查询分词与索引分词不对称 → **静默召回退化**（task-24.1 R4）。单测须断言双站点对称 round-trip 命中。
- **feature-gating（ADR-035 §D1 + §D5；镜像 vector-lancedb）**：新 `cjk-segmenter` feature + optional dep（jieba-rs 较轻 pure-Rust ~🟡；lindera 内嵌 IPADIC/ko-dic 较重 ~🔴）。默认 `default = []` 不含此 feature → 默认构建不编译真分词库 → **0 新 dep**（ADR-004）。dep 实际加入经 main-agent R7 chore + ADR-008 add-only——**本 task 不自改 Cargo.toml**。
- **真分词 vs bigram 可判定性**：`配置加载` 经真分词 → `[配置, 加载]`（2 token，词边界）；经 bigram → `[配置, 置加, 加载]`（3 token，含 `置加` 滑窗噪声）。单测以 token 文本集合差异断言「真分词」而非 bigram（确定性，不依赖 recall 数值）。

> **stop-condition（ADR-013，不伪造）**：若 jieba-rs / lindera 在本工作区目标三元组（含 Windows MSVC）构建受阻 → 如实记录受阻维度（🔴），对应 AC 不标 `[x]`；不伪造分词输出 / 不伪造构建成功。重词典 dep 维度受阻据已达维度如实 ratify（task-30.3）。

### 5.3 不变量

- 默认构建（无 `cjk-segmenter` feature）**0 新 dep**——默认分词（`default` analyzer）+ `code_cjk` bigram opt-in + 6 字段 schema 不变；`cargo test --workspace`（无 feature）不受影响（ADR-004 / ADR-035 §D5）。
- 并行名：`code_cjk` bigram analyzer **保留**作 0-dep fallback，**不删除、不 in-place 替换**（ADR-035 §D2）。
- index/query 双站点注册对称——新 analyzer 名在 `open_with_tokenizer:442` 与 `open_with_config:250` 同时注册，否则召回静默退化（task-24.1 R4）。
- 既有 opt-in `code_cjk` collection 行为不变（bigram round-trip 仍成立）；既有默认索引不失效。
- optional dep 经 main-agent R7 + ADR-008 add-only——**subagent / 本 task 不自改 `core/Cargo.toml` / `Cargo.lock`**（ADR-008 / ADR-012）。
- 分词单测确定性、CI-verifiable、无 live network dep（ADR-013）。

## 6. Acceptance Criteria

- [ ] AC1（真分词 token stream，🟢 under `--features cjk-segmenter`）: 新 `cjk_segmenter` analyzer 对多字 CJK 短语切出**真词边界**——`配置加载 → 配置/加载`（token 集合区别于 bigram 的 `配置/置加/加载`，无 `置加` 滑窗噪声 token） — verified by TEST-30.1.1
- [ ] AC2（index/query 双站点注册对称，🟢）: 新 analyzer 名在 index 站点 `open_with_tokenizer:442` 与 query 站点 `open_with_config:250` **同时注册**；一个 opt-in collection round-trip 查询能解析 analyzer 并命中（无静默召回退化，task-24.1 R4） — verified by TEST-30.1.2
- [ ] AC3（默认构建不变 + 0 新 dep，🟢）: 默认构建（无 `cjk-segmenter` feature）默认分词 + 6 字段 schema 不变、0 新 dep 完好；`cargo test --workspace`（无 feature）不受影响；`code_cjk` bigram fallback 保留 — verified by TEST-30.1.3
- [ ] AC4（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by TEST-30.1.4

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-30.1.1 | `--features cjk-segmenter` 下真分词 token stream 单测：`配置加载 → 配置/加载`（真词边界，区别于 bigram `配置/置加/加载`） | `core/src/indexer/mod.rs`（`#[cfg(feature = "cjk-segmenter")]` test） | Planned |
| TEST-30.1.2 | index/query 双站点注册对称单测：新 analyzer 名在 `:442` + `:250` 同时注册；opt-in collection round-trip 解析 + 命中无静默退化 | `core/src/indexer/mod.rs` + `core/src/retriever/mod.rs`（feature-gated test） | Planned |
| TEST-30.1.3 | 默认构建（无 feature）无回归：默认分词 + 6 字段 schema + 0 新 dep；`cargo test --workspace` 不受影响；`code_cjk` fallback 保留 | `core/src/indexer/mod.rs`（默认 test） | Planned |
| TEST-30.1.4 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威） | `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（中）双站点注册不对称致召回静默退化**：新 analyzer 名漏注册 query 站点 `:250`（或 index 站点 `:442`）→ `QueryParser::for_index` 解析失败 → 查询/索引分词不对称 → 召回退化无报错（task-24.1 R4 复现风险）。
  - **缓解**：TEST-30.1.2 显式断言双站点注册对称 + round-trip 命中；契约层把注册抽到单一 `register_cjk_segmenter` 函数，两站点同调（镜像 `register_code_cjk` 现状）。
- **R2（🟡-🔴）真分词库构建 / 体积受阻**：lindera 内嵌 IPADIC/ko-dic 词典体积大（~🔴）；jieba-rs 较轻但仍引词典 dep（~🟡）；目标三元组（含 Windows MSVC）可能构建受阻。
  - **缓解**：feature-gated，默认构建不编译（0 新 dep）；选型倾向 jieba-rs（较轻 pure-Rust）。受阻则如实记录维度（ADR-013）、对应 AC 不标 `[x]`、据已达维度 ratify（task-30.3）。dep 经 main-agent R7 + ADR-008 add-only。
- **R3（低）`RetrieverConfig.tokenizer:99` vestigial 误用**：该字段现**从不在 search 热路径读取**（search 经 `QueryParser::for_index:325-328` 据 schema 字段绑定派生 analyzer，非 config）。本 task 不接线该字段（属 task-30.2）；若误以为改 config 即切换 analyzer 会无效。
  - **缓解**：§5.2/§5.3 明确 schema-driven 对称是真实机制；config 路由接线显式划归 task-30.2 [SPEC-OWNER:task-30.2-tokenizer-default-on-and-cjk-recall-delta]。
- **R4（低）默认构建受 feature 改动污染**：feature gate 写错（如误把 `cjk-segmenter` 并入 `default`）会破 0-dep 基线。
  - **缓解**：TEST-30.1.3 断言默认构建 0 新 dep + schema 不变；Cargo.toml dep 行由 main agent R7 审入（非本 task 自改）。

## 9. Verification Plan

```bash
# 1. AC1 — 真分词 token stream（feature build；需先经 main-agent R7 加入 optional dep）
#    断言：cjk_segmenter analyzer 对「配置加载」切出 [配置, 加载]（真词边界），
#    token 集合区别于 bigram 的 [配置, 置加, 加载]
cargo test -p contextforge-core --features cjk-segmenter cjk_segmenter

# 2. AC2 — index/query 双站点注册对称 + round-trip
#    断言：新 analyzer 名在 open_with_tokenizer:442 与 open_with_config:250 同时注册；
#    opt-in collection 写入 + 查询解析 + 命中（无静默退化）
cargo test -p contextforge-core --features cjk-segmenter dual_site_register

# 3. AC3 — 默认构建无回归 + 0 新 dep（无 feature）
cargo test --workspace
#    （Cargo.lock 不应因默认构建新增 jieba-rs/lindera 条目——dep 仅 feature-gated）

# 4. AC4 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **outward-facing 说明**：本 task 无 outward-facing 面（无 tag / release / GHCR）。optional dep（jieba-rs / lindera）实际加入 `core/Cargo.toml` + `Cargo.lock` pin 经 **main-agent R7 chore + ADR-008 add-only**——subagent / 本 task spec 仅规划契约层，不自改 manifest（ADR-008 / ADR-012）。真实 recall delta 属 task-30.2，本 task 不产出 recall 数值（ADR-013）。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Draft（待实施）

- **计划改动文件**（will record real diff at impl）:
  - `core/src/indexer/mod.rs`——新增 `CJK_SEGMENTER_TOKENIZER = "cjk_segmenter"` 常量（与 `:183` 并列）+ `#[cfg(feature = "cjk-segmenter")]` `build_cjk_segmenter_analyzer`（镜像 `:364-369` filter 链）+ `register_cjk_segmenter`（镜像 `:373-377`）+ index 站点 `open_with_tokenizer:442` 注册调用 + 真分词/默认构建单测。
  - `core/src/retriever/mod.rs`——query 站点 `open_with_config:250` 对称注册 `register_cjk_segmenter`（feature-gated）+ 双站点对称 round-trip 单测。
  - **规划标注（不在本 task 自改）**：`core/Cargo.toml` 新 `cjk-segmenter = ["dep:jieba-rs"]`（或 lindera）feature 行 + optional dep + `Cargo.lock` pin——经 main-agent R7 chore + ADR-008 add-only [SPEC-OWNER:task-30.1-r7-dep-chore]。
- **§9 Verification 计划**（will record real evidence at impl）:
  - AC1：`cargo test -p contextforge-core --features cjk-segmenter` 真分词单测——真实跑出后回填 token stream 断言结果。
  - AC2：双站点注册对称 + opt-in round-trip 命中——真实跑出后回填。
  - AC3：`cargo test --workspace`（无 feature）默认构建无回归 + Cargo.lock 0 新条目——真实跑出后回填。
  - AC4：D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）——真实跑出后回填。
  - 分词库构建受阻维度（含 Windows MSVC，🔴）——待实测回填（受阻则如实记录，不伪造，ADR-013）。
