# Task `30.2`: `tokenizer-default-on-and-cjk-recall-delta — tokenizer 默认开启评估 + 既有索引 reindex/migration 工具 + RetrieverConfig.tokenizer 路由接线（或 schema-driven 对称文档化）+ 扩 CJK golden 真实 recall delta（default vs bigram vs 真分词，ADR-013 不预填）`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 30 (cjk-true-segmenter)
**Dependencies**: task-30.1（cjk-true-segmenter，真分词 analyzer + parallel 名 + 双站点注册，本 task 评估其默认开启面）/ Phase 24（retrieval-tokenizer-and-eval-hardening，Done——bigram analyzer + phase24 recall harness + golden-semantic.jsonl + Go `ValidateGoldenSemantic` 起源）/ `core/src/retriever/mod.rs:99`（`RetrieverConfig.tokenizer` 现 vestigial）+ `:325-328`（search 路径 `QueryParser::for_index` 据 schema 绑定派生 analyzer，非读 config）/ `core/src/indexer/mod.rs:155/162/442`（schema 分支 + index 站点注册）/ `test/fixtures/eval/golden-semantic.jsonl`（11 行：6 code-symbol + 5 cjk）/ `internal/eval/eval.go:231-280`（`ValidateGoldenSemantic`）/ ADR-035 D3+D4（tokenizer-default-on 评估 + 扩 CJK golden recall delta）/ ADR-029（code-and-cjk-tokenizer-and-eval-hardening，add-only Amendment 由 task-30.3）/ ADR-013（禁伪造召回 / perf 数字红线）/ ADR-004（local-first-privacy-baseline，默认构建 0-dep 不变）/ ADR-014 D1-D5（第二十一次激活）

## 1. Background

Phase 24 落地了 opt-in 的 code/CJK analyzer（`CodeCjkTokenizer`，代码符号拆分 + CJK **overlapping bigram**：`配置加载 → 配置/置加/加载`），但它**不是默认开启**——`build_tantivy_schema` 仅在 `tokenizer_name == CODE_CJK_TOKENIZER` 分支（`core/src/indexer/mod.rs:155`）把 `content` 绑自定义 analyzer，默认分支（`:162`）维持 `TEXT | STORED`。task-30.1 在 bigram 之上加了 feature-gated 真分词器（`cjk-segmenter` feature，parallel analyzer 名，默认 0-dep）。本 task 处理**两件遗留**：

1. **tokenizer-default-on 评估**（ADR-029:54 Follow-ups + phase-24 spec:42 + task-24.3:39/40 的开放 marker）：把 tokenizer 从 opt-in 翻成**默认开启**意味着改默认 analyzer 绑定，而绑定持久化在 tantivy `meta.json`（schema-binding），**既有索引必须 re-index** 才能享新 analyzer——否则旧 index 仍按旧绑定检索。故默认开启**必须**配套既有索引 reindex/migration 工具。
2. **`RetrieverConfig.tokenizer` 路由真接线 或 schema-driven 对称文档化**：`core/src/retriever/mod.rs:99` 的 `RetrieverConfig.tokenizer`（Default `:110`）**当前 vestigial**——search 路径在 `:325-328` 用 `QueryParser::for_index(&self.tantivy_index, vec![f_content, f_file_path])` 据 **schema 字段绑定**派生 analyzer，**从不读 config.tokenizer**。若 Phase 30 想要 config 驱动选择，必须真把 `config.tokenizer` 接线到「选哪个 register fn / 哪个 analyzer 名」；否则须**文档化 schema-driven 对称**（绑定唯一真相源是 `meta.json`，config 字段保留为 no-op 并注明）。

同时，现 golden 仅 5 个 CJK case、Phase 24 实测 delta `+0.0909` **由单个 cjk case 驱动**（小语料、单点）——要测出真分词 vs bigram 有意义的 delta，须**扩充 CJK case**（经 Go `ValidateGoldenSemantic` 守门），并经 phase24-style harness 跑出 **default vs bigram vs 真分词**的真实 before/after delta。数字**实测回填、绝不预填**（ADR-013），且**小语料如实标注、不外推**。

## 2. Goal

(1) **评估 tokenizer-default-on**：实现既有索引 **reindex/migration 工具**（默认 analyzer 绑定变更须 re-index，因绑定持久化 `meta.json`），并把 `RetrieverConfig.tokenizer` **真路由接线**（现 `:99` vestigial）到 analyzer 选择 **或** 文档化 schema-driven 对称（绑定唯一真相源是 schema/`meta.json`）；既有默认索引**向后兼容不失效**。(2) **扩 CJK golden + 真实 recall delta**：扩充 `test/fixtures/eval/golden-semantic.jsonl` 的 CJK case（经 Go `ValidateGoldenSemantic` schema/dup/category 守门），经 phase24-style harness 跑 **default vs bigram vs 真分词**真实 recall delta（数字实测回填、不预填、小语料不外推，ADR-013）。

pass bar：reindex/migration 工具可对既有索引重建 + config 路由接线（或 schema-driven 对称文档化）二选一落地且既有默认索引不破；扩充 CJK golden 经 `ValidateGoldenSemantic` 通过；真实 recall delta 跑出（`🟡` feature build / local real run，数字回填）。**若完整 default-on 迁移工程过重**（reindex 工具 + config 路由 + 既有索引兼容三者综合），则**诚实延后 default flip**、保留 opt-in + migration 工具 `[SPEC-DEFER:phase-future.tokenizer-default-on]`（ADR-013 不强推不达项）。D2 lint 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- **reindex/migration 工具**：对既有 collection 索引按目标 analyzer 绑定**重建**（因 analyzer 绑定持久化 `meta.json`，变更须 re-index）；既有默认 index 不破（向后兼容）。形态（CLI 子命令 vs `core` 内 fn）于 §5.2 决策、实施时定。
- **`RetrieverConfig.tokenizer` 路由接线 或 schema-driven 对称文档化**（二选一）：要么把 `config.tokenizer`（`retriever/mod.rs:99`）真接线到 analyzer 选择（决定调哪个 register fn），要么文档化「绑定唯一真相源是 schema/`meta.json`、config 字段保留 no-op」并注明 `:325-328` 不读 config 的事实。
- **扩 `test/fixtures/eval/golden-semantic.jsonl` 的 CJK case**：新增 CJK 标注行（category `cjk`），经 Go `ValidateGoldenSemantic`（`internal/eval/eval.go:231-280`）schema/dup/category 守门；现 11 行（6 code-symbol + 5 cjk）不删改既有行（add-only）。
- **真实 recall delta 度量**：经 phase24-style harness（`core/examples/phase24_tokenizer_recall.rs` 同形）跑扩充后 golden 的 **default vs bigram vs 真分词**真实 before/after delta；数字**实测回填**（§10 will record at impl）、小语料 caveat、不外推。
- **tokenizer-default-on 评估结论**：达项则记录默认开启路径 + migration；过重则诚实 `[SPEC-DEFER:phase-future.tokenizer-default-on]`，保留 opt-in + migration 工具。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **真分词 analyzer 本体 + feature-gating + 双站点注册** [SPEC-OWNER:task-30.1]（本 task 只评估其默认开启面 + 跑 recall delta）。
- **重词典 dep（jieba-rs / lindera）落地** [SPEC-OWNER:task-30.1]（dep add-only 经主 agent R7 chore + ADR-008，规划阶段不加 dep）。
- **closeout（adapter / phase Status→Done / ADR-035 ratify / ADR-029 Amendment / release）** [SPEC-OWNER:task-30.3]。
- **若 default flip 过重则延后默认开启** [SPEC-DEFER:phase-future.tokenizer-default-on]（保留 opt-in + migration 工具，ADR-013 不强推）。
- **大规模 / 跨语言（ko-dic / 日语形态）扩 golden** [SPEC-DEFER:phase-future.cross-lingual-golden]（本 task 限中文 CJK case 扩充）。
- **embedding / 向量召回侧 CJK 评估** [SPEC-DEFER:phase-future.cjk-semantic-recall]（本 task 限 BM25 lexical analyzer 面）。

## 4. Actors

- 主 agent（ADR-012 自治；扩 golden + recall harness + reindex 工具均 inward-facing，无 outward-facing 红线）
- `core/src/indexer/mod.rs`（`build_tantivy_schema:155/162` analyzer 绑定 + `register_*:442` 注册站点；reindex 工具按目标绑定重建）
- `core/src/retriever/mod.rs`（`RetrieverConfig.tokenizer:99` + search 路径 `:250` 注册 + `:325-328` `QueryParser::for_index` schema-driven 派生）
- `test/fixtures/eval/golden-semantic.jsonl`（扩 CJK case 的 golden 真相源）
- `internal/eval/eval.go`（`ValidateGoldenSemantic:231-280` + `knownCategories:214-223` 守门 dirty data）
- phase24-style recall harness（`core/examples/phase24_tokenizer_recall.rs` 同形，default vs bigram vs 真分词 delta）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/retriever/mod.rs:99`（`RetrieverConfig.tokenizer` 字段，Default `:110`——**vestigial，search 路径从不读**）+ `:250`（`register_code_cjk(&tantivy_index)` query 站点）+ `:325-328`（`QueryParser::for_index(&self.tantivy_index, vec![f_content, f_file_path])`——analyzer 由 **schema 字段绑定**派生，非 config）
- `core/src/indexer/mod.rs:155`（opt-in 分支 `content` 绑 `CODE_CJK_TOKENIZER`）+ `:162`（默认分支 `TEXT | STORED`）+ `:442`（`IndexSession::open_with_tokenizer` index 站点 `register_code_cjk`）——analyzer 绑定持久化 `meta.json` ⇒ 变更须 re-index
- `test/fixtures/eval/golden-semantic.jsonl`（现 11 行：6 code-symbol + 5 cjk；扩 CJK case，add-only 不改既有行）
- `docs/spikes/phase-24-tokenizer-recall.md` + `core/examples/phase24_tokenizer_recall.rs`（phase24 recall harness 方法论：现实测 delta `+0.0909` 由单 cjk case 驱动、11 q / 12 file 小语料）
- `internal/eval/eval.go:231-280`（`ValidateGoldenSemantic` schema/dup/category/dangling 守门）+ `:214-223`（`knownCategories`，含 `cjk` / `code-symbol`）
- ADR-035 §D3（tokenizer-default-on 评估 + reindex/migration + config 路由）+ §D4（扩 CJK golden + 真实 recall delta，ADR-013 不预填）
- deferral provenance：ADR-029:54（Follow-ups：cjk-true-segmenter + tokenizer-default-on）+ :66（ratification scope）；phase-24 spec:42（tokenizer-default-on，opt-in 不破既有索引、默认开启 + 迁移属后续）+ :125；task-24.3:39/:40

### 5.2 关键设计 — tokenizer-default-on 评估：reindex 必要性 + config-routing vs schema-driven 对称 + 真实 recall delta（ADR-013 不预填）

**(a) 默认开启为何须 re-index（绑定持久化）**：`build_tantivy_schema`（`indexer/mod.rs:149`）在 schema 构造时把 `content` 字段的 tokenizer 名写入 schema，schema 经 tantivy `meta.json` **持久化**。翻默认 analyzer 绑定**不会**追溯改写既有 index 的 `meta.json`——旧 index 仍按旧绑定（`default`）分词检索。故「默认开启」**等价于**「既有索引须按新绑定 re-index」。本 task 的 reindex/migration 工具即填此缺：按目标 analyzer 绑定**重建** collection 索引；既有默认 index 在工具运行前**不破**（向后兼容读路径 `:240-293` 不依赖新绑定）。

**(b) config-routing vs schema-driven 对称（二选一，§3 In Scope 决策）**：
- **方案 A — config 路由真接线**：把 `RetrieverConfig.tokenizer`（`:99`，现 vestigial）真接线到 analyzer 选择——index 侧据 config 选 `build_tantivy_schema` 分支、query 侧据 config 选调哪个 register fn（task-30.1 的 parallel 名 + 双站点注册前提下）。**代价**：config 须与 schema `meta.json` 绑定**保持一致**，否则 query 解析按 schema 派生（`:325-328`）与 config 期望分叉 → 静默召回退化（task-24.1 R4）。
- **方案 B — schema-driven 对称文档化**：保留 `config.tokenizer` 为 **no-op**（注明 `:325-328` 据 schema 字段绑定派生 analyzer，绑定唯一真相源是 `meta.json`），不引入 config↔schema 一致性负担。**代价**：config 字段名义存在但不可驱动选择，调用方须经 reindex 工具 + index 侧绑定换 analyzer。
- 选择于实施时据「config 路由真带来可观测价值 vs 一致性维护成本」定；**任一方案均须保 index/query 分词对称**（task-24.1 R4：新 analyzer 名须 index `:442` + query `:250` 双站点注册，否则 query 解析静默失败 → 召回退化）。

**(c) 扩 CJK golden + 真实 recall delta（ADR-013 不预填）**：现 golden 仅 5 CJK case、delta 由**单 case** 驱动 → 扩充 CJK 标注行（中文短语 case，含真分词与 bigram 切分边界有别者，如 `配置加载` 真分词 `配置/加载` vs bigram `配置/置加/加载`），经 `ValidateGoldenSemantic` 守门（schema/dup/category/dangling），再经 phase24-style harness 跑 **default vs bigram vs 真分词** 三方真实 recall delta。**数字实测回填**（§10 record at impl）、**小语料 caveat 如实标注**（11+N q / 小语料，不外推到大语料）、**绝不预填**（ADR-013）。

### 5.3 不变量

- 默认构建 0-dep / 0-network baseline 不变（ADR-004）——reindex 工具 + config 接线 + golden 扩充均不引入新代码 dep（真分词 dep 在 task-30.1 经 ADR-008 add-only，本 task 不加）。
- 既有默认 index **向后兼容不失效**——reindex 工具运行前旧 index 读路径（`retriever/mod.rs:240-293`）不依赖新绑定；golden 扩充 add-only（不删改既有 11 行）。
- index/query 分词**对称**——任一新 analyzer 名须 index 站点（`:442`）+ query 站点（`:250`）双注册（task-24.1 R4），config 路由方案下 config↔schema 绑定须一致。
- recall 数字**实测、非合成**——`待实测回填`，绝不预填；小语料 caveat 显式、不外推（ADR-013）。
- ADR-013 no-prefill：本 task 任何 recall / delta / perf 数字均**真实跑出后回填**，不出现伪造 run-id / 合成数值。
- default flip 不强推——迁移过重则诚实 `[SPEC-DEFER:phase-future.tokenizer-default-on]`，保留 opt-in + migration 工具，不为达项伪造「已默认开启」。

## 6. Acceptance Criteria

- [ ] AC1（🟡 真实 recall delta，扩 CJK golden）: 扩充后 golden 经 phase24-style harness 跑出 **default vs bigram vs 真分词**真实 recall delta（数字实测回填、不预填、小语料 caveat 不外推，ADR-013）；真分词相对 bigram 的 CJK 召回变化如实记录（含「无提升 / 单点驱动」诚实结论的可能） — verified by TEST-30.2.1
- [ ] AC2（🟢 reindex/migration + config 路由 或 schema-driven 对称）: 既有索引 reindex/migration 工具按目标 analyzer 绑定重建 + `RetrieverConfig.tokenizer` 路由真接线（或 schema-driven 对称文档化，二选一）；既有默认索引**向后兼容不破**（index/query 分词对称保持，task-24.1 R4） — verified by TEST-30.2.2
- [ ] AC3（🟢 扩 golden 经 Go 守门）: 扩充后 `test/fixtures/eval/golden-semantic.jsonl` 的 CJK case 经 Go `ValidateGoldenSemantic`（schema / dup / category，`internal/eval/eval.go:231-280`）通过；既有 11 行不删改（add-only）；`go test ./...` deterministic 绿 — verified by TEST-30.2.3
- [ ] AC4（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by TEST-30.2.4

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-30.2.1 | 扩 CJK golden 经 phase24-style harness 跑 default vs bigram vs 真分词真实 recall delta（数字回填、不预填、小语料不外推，ADR-013）；SCEN-30.2.1 | `core/examples/phase24_tokenizer_recall.rs`（同形 harness）+ `test/fixtures/eval/golden-semantic.jsonl` + §10 实测记录 | Planned |
| TEST-30.2.2 | reindex/migration 工具按目标绑定重建 + `RetrieverConfig.tokenizer` 路由接线（或 schema-driven 对称文档化）+ 既有默认索引向后兼容 + index/query 对称（:442+:250） | `core/src/indexer/mod.rs` + `core/src/retriever/mod.rs` | Planned |
| TEST-30.2.3 | 扩充后 golden CJK case 经 Go `ValidateGoldenSemantic`（schema/dup/category，eval.go:231-280）通过 + 既有行 add-only 不改 | `internal/eval/eval.go` + `test/fixtures/eval/golden-semantic.jsonl` | Planned |
| TEST-30.2.4 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威） | `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（中）默认开启迁移工程过重 → default flip 受阻**：reindex 工具 + config 路由 + 既有索引兼容三者综合过重。
  - **缓解**：诚实延后 default flip、保留 opt-in + migration 工具 `[SPEC-DEFER:phase-future.tokenizer-default-on]`（ADR-013 不强推不达项）；AC2 仍可经 migration 工具 + schema-driven 对称文档化（方案 B）以较轻路径达项。stop-condition：reindex/兼容不可行则该子项不标 `[x]`、§10 如实记录受阻。
- **R2（中）小语料 recall delta 单点驱动 / 无显著提升 — ⚠️ 已知 Phase 24 特征**：现 golden 11 q / 12 file，Phase 24 delta `+0.0909` 由单 cjk case 驱动；扩 CJK case 后真分词相对 bigram 仍可能**无显著提升 / 提升仍由少数 case 驱动**。
  - **缓解**：扩充足量 CJK case（含真分词与 bigram 切分边界有别者）；数字**实测回填**、小语料 caveat 显式标注、**不外推**（ADR-013）；若真分词无提升则**如实记录**（诚实结论本身即有效产出，bigram 作 0-dep fallback 仍有价值）——不为达项伪造提升数字。
- **R3（中）config 路由 vs schema 绑定分叉致静默召回退化**：方案 A 下 `config.tokenizer` 与 schema `meta.json` 绑定不一致 → query 据 schema 派生（`:325-328`）与 config 期望分叉。
  - **缓解**：方案 A 须保 config↔schema 一致（index/query 同源决定 analyzer）；或选方案 B（schema-driven 对称，config no-op）规避一致性负担；任一方案保 index `:442` + query `:250` 双站点注册（task-24.1 R4）。
- **R4（低）扩 golden 引入 dirty data 污染 recall 分母**：新增 CJK 行 schema 不良构 / 重复 / 悬空 expected。
  - **缓解**：经 Go `ValidateGoldenSemantic`（schema/dup/category/dangling，eval.go:231-280）守门 + `knownCategories` 限 `cjk`；`go test ./...` deterministic 卡红。

## 9. Verification Plan

```bash
# 1. AC3 — 扩充后 golden 经 Go ValidateGoldenSemantic 守门（schema/dup/category，deterministic）
go test ./internal/eval/...
go test ./...

# 2. AC2 — reindex/migration 工具 + config 路由（或 schema-driven 对称）+ 既有索引向后兼容 + index/query 对称
cargo test --workspace
#    （含 reindex 工具按目标绑定重建 + 既有默认 index 不破 + 新 analyzer 名 :442+:250 双注册回归）

# 3. AC1 — 真实 recall delta（🟡 feature build / local real run；default vs bigram vs 真分词）
#    扩充后 golden 经 phase24-style harness 跑三方 recall delta；数字实测回填、小语料不外推（ADR-013）
cargo run -p contextforge-core --example phase24_tokenizer_recall            # default vs bigram（0-dep）
cargo run -p contextforge-core --features cjk-segmenter --example phase24_tokenizer_recall  # + 真分词（task-30.1 feature）
#    实测 default / bigram / 真分词 recall@k + delta —— 待实测回填（NOT prefilled，ADR-013）

# 4. AC4 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **inward-facing**：本 task 全部 inward-facing（golden 扩充 / reindex 工具 / config 接线 / recall harness），无 outward-facing 红线。真分词 recall delta 经 `--features cjk-segmenter`（task-30.1 落地后）本地真实跑出；数字实测回填、绝不预填（ADR-013）。tokenizer-default-on 默认翻转**不在本 task 自行 outward 触发**——达项则记录默认开启路径、过重则诚实延后。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Draft（待实施）

**计划改动文件**：
- `test/fixtures/eval/golden-semantic.jsonl`——add-only 扩充 CJK 标注 case（含真分词与 bigram 切分边界有别者）；既有 11 行（6 code-symbol + 5 cjk）不删改。
- `internal/eval/eval.go`——`ValidateGoldenSemantic`（`:231-280`）守扩充后 golden；`knownCategories`（`:214-223`）已含 `cjk`，预期不改（如需扩类别 add-only）。
- `core/src/indexer/mod.rs`——reindex/migration 工具（按目标 analyzer 绑定重建既有索引；绑定持久化 `meta.json` ⇒ 须 re-index）；保 index 站点注册（`:442`）。
- `core/src/retriever/mod.rs`——`RetrieverConfig.tokenizer`（`:99`）路由真接线（方案 A）**或** schema-driven 对称文档化（方案 B，注明 `:325-328` 据 schema 派生、config no-op）；保 query 站点注册（`:250`）。
- `core/examples/phase24_tokenizer_recall.rs`（同形 harness）——跑扩充后 golden 的 default vs bigram vs 真分词真实 recall delta。
- `docs/specs/tasks/task-30.2-tokenizer-default-on-and-cjk-recall-delta.md`——§10 实施后回填实测结果。

**§9 Verification 计划** (will record real evidence at impl)：
- AC1（🟡）：`待实测回填` — 扩充后 golden 经 phase24-style harness 的 default / bigram / 真分词 recall@k + delta（真实跑出后回填，小语料 caveat、不外推，ADR-013；含「真分词无显著提升 / 单点驱动」诚实结论的可能）。
- AC2（🟢）：`待实测回填` — `cargo test --workspace` reindex 工具按目标绑定重建 + config 路由（或 schema-driven 对称文档化）+ 既有默认索引向后兼容 + index/query `:442`+`:250` 双注册回归（真实跑出后回填）。
- AC3（🟢）：`待实测回填` — `go test ./internal/eval/...` + `go test ./...` 扩充后 golden CJK case 经 `ValidateGoldenSemantic` 通过、既有行 add-only（真实跑出后回填）。
- AC4（🟢）：`待实测回填` — `bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威，真实跑出后回填）。
- **tokenizer-default-on 评估结论**：`待实测回填` — 默认开启达项则记录默认开启路径 + migration；迁移过重则诚实 `[SPEC-DEFER:phase-future.tokenizer-default-on]`、保留 opt-in + migration 工具（ADR-013 不强推不达项）。
