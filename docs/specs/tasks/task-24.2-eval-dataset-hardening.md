# Task `24.2`: `eval-dataset-hardening — internal/eval/eval.go golden 数据集独立校验器（schema 良构 + 重复检测 + query/answer 覆盖）+ test/fixtures/eval/golden-semantic.jsonl 扩充 annotated query（含代码符号 + CJK query case，exercise task-24.1 tokenizer）+ deterministic 校验单测`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 24 (retrieval-tokenizer-and-eval-hardening)
**Dependencies**: task-14.1（eval 模块框架 + `core/src/eval` + EvalRun schema）/ task-19.5（recall eval + `test/fixtures/eval/dogfood-embeddings.jsonl` real 语料 + golden 数据集口径 + `Question` JSON 形状）/ task-8.1（`internal/eval/eval.go` `ValidateDataset` + `Question` + `BuiltinGoldenQuestions` 30 题 + `LoadJSONL`/`WriteJSONL`）/ ADR-029 D2 + D3（数据集校验器 + golden 扩充）/ ADR-013（禁伪造 recall）/ ADR-014 D1-D5（第十五次激活）

## 1. Background

召回声明全靠 eval 度量背书。recall harness 在 Go（`internal/eval/eval.go`）：`ValidateDataset`（`eval.go:183-209`）仅校验 ≥30 题 / ≥6 类 / 每类 ≥5 题 / 必填字段（`Query` 非空 + `ExpectedFilePath`/`ExpectedChunkID` 至少一项 + `Category` 非空）；golden 数据集为 `BuiltinGoldenQuestions()`（`eval.go:111-181` 硬编码 30 题，6 类各 5 题）+ `test/fixtures/eval/dogfood-embeddings.jsonl`（task-19.5 real embedding 语料 40 行 dim-384）。`Question` 形状（`eval.go:29-37`）：`query` / `expected_sources` / `expected_file_path` / `expected_line_range` / `expected_chunk_id` / `category` / `notes`。

现状三处局限（`docs/roadmap.md` §4 marker `eval-dataset-validation` / `semantic-golden-dataset`）：(a) `ValidateDataset` 不查 schema 良构细节（category 是否在已知集 / line_range 是否合理）、不查 **重复**（同一 `query` 文本重复 / 同一 `(query, expected)` 对重复会膨胀题数却不增覆盖）、不查 **query/answer 覆盖**（声明的 expected 文件 / chunk_id 是否有悬空）——脏数据会静默喂入召回口径污染数字；(b) golden 数据集 30 题为 BM25 口径（task-18.8 §3 + task-19.5 §10 复用 file-level），**无代码符号 / CJK query case**，无法度量 task-24.1 tokenizer 改进；(c) `test/fixtures/eval/` 下无独立的 annotated semantic golden 数据集文件（task-19.5 §10 记 golden-semantic.jsonl 「未需要」，本 phase 因 tokenizer 度量需要而新增）。

本 task 加 eval golden 数据集独立校验器（schema 良构 + 重复检测 + query/answer 覆盖）+ 扩充 `test/fixtures/eval/golden-semantic.jsonl`（含代码符号 + CJK annotated query，exercise task-24.1 tokenizer）。本 task **不产真实 recall 数字**——真实 before/after recall delta 在 closeout（task-24.3）据 task-24.1 tokenizer over 本扩充数据集实测（ADR-013）。

## 2. Goal

`internal/eval/eval.go` 加独立校验器（add-only，不改既有 `ValidateDataset` 现有断言语义）：(a) **schema 良构**——每条 question 字段类型 / 必填项 / category 在已知集（config-location / error-reproduction / historical-decision / log-troubleshooting / agent-memory-rule / code-location + 本 task 新增的 code-symbol / cjk 类）/ line_range 合理（start≤end）；(b) **重复检测**——同一 `query` 文本重复 / 同一 `(query, expected_file_path/expected_chunk_id)` 对重复被识别报错；(c) **query/answer 覆盖**——每条 question 声明 expected_file_path 或 expected_chunk_id（无悬空 expected：两者皆空被拒）。新增 `test/fixtures/eval/golden-semantic.jsonl`（每行一个 `eval.Question`），含代码符号 query case（camelCase / snake_case / dotted.path 标识符查询）+ CJK query case，exercise task-24.1 tokenizer；扩充数据集过新校验器。≥3 Go 测试全 PASS：校验器对良构数据集过 / 脏数据（重复 / 悬空 / schema 不良）被拒 / 扩充 golden 含代码+CJK case 且过校验。既有 `ValidateDataset` + 30 题 builtin + JSONL roundtrip（`TestTask81_*`）不退化；`go test ./...` 全 PASS；D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **修改 `internal/eval/eval.go`**：加独立校验器函数（如 `ValidateDatasetStrict` / `ValidateGoldenSemantic` 或等价命名）——schema 良构（字段类型 + category 在已知集 + line_range start≤end）+ 重复检测（同 `query` / 同 `(query, expected)` 对）+ query/answer 覆盖（expected_file_path 或 expected_chunk_id 至少一项非空，无悬空）。**add-only**：既有 `ValidateDataset` 的现有断言（≥30 题 / ≥6 类 / 每类 ≥5 题 / 必填字段）语义不动；新校验器独立函数（或 `ValidateDataset` 内部 add-only 补强且向后兼容现有调用方 `TestTask81_*`）。
- **新增 `test/fixtures/eval/golden-semantic.jsonl`**：每行一个 `eval.Question`（JSON tag 沿用 `eval.go:29-37`：`query` / `expected_file_path` / `expected_chunk_id` / `expected_line_range` / `category` / `notes`），含 **代码符号 query case**（如 query=`getUserById`/`user_id`/`pkg.module.func` 等代码标识符 + expected 指向含该符号的真实源码文件）+ **CJK query case**（如 query=中文检索短语 + expected 指向含该 CJK 文本的真实文件/文档）；query 文本设计为 exercise task-24.1 tokenizer 的代码符号拆分 + CJK bigram。数据集过新校验器（无重复 / 无悬空 / schema 良构）。
- **新增同源 Go 单测（`internal/eval/eval_test.go`）**：(a) 校验器对良构数据集（含 `BuiltinGoldenQuestions` + 扩充 golden）过；(b) 校验器对脏数据被拒——重复 query / 重复 (query,expected) 对 / 悬空 expected（两者皆空）/ category 不在已知集 / line_range start>end，逐项断言报错；(c) 扩充 `golden-semantic.jsonl` 经 `LoadJSONL` 载入 + 含代码符号 + CJK query case + 过新校验器。
- **可能修改 `internal/eval/eval.go::BuiltinGoldenQuestions` 或已知 category 集**：仅当扩充 golden 引入新 category（如 code-symbol / cjk）需校验器识别时——add-only 扩已知 category 集，不改既有 6 类语义。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **task-24.1 自定义 code/CJK tokenizer 实现** [SPEC-OWNER:task-24.1-code-and-cjk-tokenizer]：本 task 扩充能 exercise 它的 golden 数据集，不实现 tokenizer。
- **tokenizer 真实 before/after recall delta 实测** [SPEC-OWNER:task-24.3-closeout-v0.17.0]：本 task 产「可校验、可 exercise tokenizer」的扩充数据集 + 校验器，**不产真实 recall 数字**；真实 delta 在 closeout 据 task-24.1 tokenizer over 本数据集实测（ADR-013 不伪造 recall）。
- **rust-native-eval-runner promote** [SPEC-DEFER:phase-future.rust-native-eval-runner]：runner 评估在 task-24.3。
- **golden 数据集 case_results 子表持久化** [SPEC-DEFER:phase-future.case-results-subtable]：`docs/roadmap.md` §4 长尾。
- **CJK 真正分词器对应的 golden 标注口径** [SPEC-DEFER:phase-future.cjk-true-segmenter]：本 task 用 bigram 口径的 CJK case；真正分词器口径后续。

## 4. Actors

- **主 agent**：实施 + PR 主理。
- **`internal/eval/eval.go::ValidateDataset` / `Question` / `BuiltinGoldenQuestions` / `LoadJSONL`**：task-8.1 eval harness，本 task 加独立校验器 + 扩 golden。
- **`test/fixtures/eval/golden-semantic.jsonl`**：本 task 新增 annotated query 数据集（代码符号 + CJK case）。
- **`internal/eval/eval_test.go`**：既有 eval 单测（`TestTask81_*` / `TestTask188_*`），本 task add-only 加校验器单测且不退化既有。
- **task-24.1 tokenizer**：本 task 扩充数据集的 query case 设计为 exercise 它。
- **下游 task-24.3**：据 task-24.1 tokenizer + 本扩充数据集实测真实 before/after recall delta。

## 5. Behavior Contract

### 5.1 Required Reading

- `internal/eval/eval.go:24-37`（`LineRange` / `Question` 形状 + JSON tag）+ `eval.go:111-181`（`BuiltinGoldenQuestions` 30 题 6 类）+ `eval.go:183-209`（`ValidateDataset` 既有断言）+ `eval.go:211-253`（`LoadJSONL` / `WriteJSONL` JSONL 行格式）
- `internal/eval/eval_test.go:13-131`（`TestTask81_*` — `ValidateDataset` + JSONL roundtrip 既有口径，本 task 不退化它）+ `eval_test.go:133-208`（`TestTask188_*` — `SemanticRecallAtK` / gate 口径，本 task 不改度量）
- `test/fixtures/eval/dogfood-embeddings.jsonl`（task-19.5 real 语料行格式 `{"chunk_id", "embedding"}`，本 task 的 golden-semantic.jsonl 是 `Question` 形状，与之不同）
- `docs/specs/tasks/task-19.5-real-recall-eval.md`（golden 数据集口径 + §10 golden-semantic.jsonl 「未需要」结论，本 phase 因 tokenizer 度量需要而新增）+ `docs/specs/tasks/task-18.8-eval-semantic-recall.md`（30 题 BM25 口径 + `semantic-golden-dataset` forward-ref）
- `docs/specs/tasks/task-24.1-code-and-cjk-tokenizer.md`（tokenizer 分词规则——本 task 的 query case 设计为 exercise camelCase/snake_case/dotted.path + CJK bigram）
- `docs/decisions/adr-029-code-and-cjk-tokenizer-and-eval-hardening.md` D2 + D3 + `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造 recall）+ `docs/decisions/adr-006-recall-eval-acceptance-gate.md`（gate 阈值不变）

### 5.2 关键设计 — 数据集校验器 + 代码/CJK golden 扩充

- **校验器 add-only**：新校验器独立于 `ValidateDataset`（既有 30 题口径 + JSONL roundtrip 单测不退化）。三类检查：
  - **schema 良构**：字段类型正确 + category 在已知集（既有 6 类 + 本 task 新增 code-symbol / cjk）+ `expected_line_range` start≤end（0/0 视作整文件，沿用 `lineOverlaps` 语义）。
  - **重复检测**：同一 `query` 文本出现多次报错；同一 `(query, expected_file_path)` 或 `(query, expected_chunk_id)` 对重复报错（重复膨胀题数不增覆盖，污染召回分母）。
  - **query/answer 覆盖**：每条 question 须 expected_file_path 或 expected_chunk_id 至少一项非空（无悬空 expected）；可选检查 expected 在数据集 / 语料口径内被引用（覆盖完整性）。
- **代码/CJK golden 扩充**：`golden-semantic.jsonl` 的 query case **设计为 exercise task-24.1 tokenizer**——代码符号 query（`getUserById` / `user_id` / `pkg.module.func` 等真实代码标识符，expected 指向含该符号的真实源码文件）触发代码符号拆分；CJK query（真实中文检索短语，expected 指向含该 CJK 文本的真实文件/文档）触发 CJK bigram。query/expected 指向真实 ContextForge 源码（非手编虚构路径），expected 路径经核实存在（ADR-013 grounded）。
- **不产 recall 数字（ADR-013）**：本 task 只产「可校验、可 exercise tokenizer」的扩充数据集 + 校验器单测；真实 before/after recall delta 在 task-24.3 据 task-24.1 tokenizer over 本数据集实测，本 task **不**预跑/预填 recall 数。
- **gate 阈值不变（ADR-006）**：本 task 加固标尺但不改 `GateTop5StrongMin` / `GateTop10StrongMin` / `GateSemanticRecall10Min`（`eval.go:103-108`），不改 `SemanticRecallAtK` / `MeetsRecallGate` 度量函数签名。

### 5.3 不变量

- 既有 `ValidateDataset` + `BuiltinGoldenQuestions` 30 题 + `LoadJSONL`/`WriteJSONL` roundtrip（`TestTask81_*`）行为不退化（add-only 校验器独立函数 / 向后兼容补强）。
- recall 度量函数（`SemanticRecallAtK` / `SummarizeHybrid` / `SummarizePasses` / `MeetsRecallGate`）签名 + 语义不变；gate 阈值不变（ADR-006）。
- 扩充 `golden-semantic.jsonl` 的 query/expected 指向真实 ContextForge 源码（expected 路径经核实存在），非虚构；数据集过新校验器（无重复 / 无悬空 / schema 良构）。
- 本 task 零真实 recall 数字产出（ADR-013：真实 delta 在 task-24.3）。
- 本 PR 零 Rust delta（tokenizer 在 task-24.1）。

## 6. Acceptance Criteria

- [x] **AC1**: 数据集校验器 schema 良构 + 覆盖 — 校验器对良构数据集（`BuiltinGoldenQuestions` + 扩充 golden）过；对 schema 不良（category 不在已知集 / line_range start>end）+ 悬空 expected（expected_file_path 与 expected_chunk_id 皆空）被拒报错 — verified by **TEST-24.2.1**
- [x] **AC2**: 重复检测 — 校验器对同一 `query` 文本重复 + 同一 `(query, expected)` 对重复识别报错；既有 `ValidateDataset` + 30 题 builtin + JSONL roundtrip（`TestTask81_*`）不退化（add-only）— verified by **TEST-24.2.2**
- [x] **AC3**: 代码/CJK golden 扩充 — `test/fixtures/eval/golden-semantic.jsonl` 经 `LoadJSONL` 载入含代码符号 query case（camelCase/snake_case/dotted.path 标识符，exercise task-24.1 tokenizer 代码符号拆分）+ CJK query case（exercise CJK bigram）；query/expected 指向真实源码（路径经核实）；过新校验器 — verified by **TEST-24.2.3**
- [x] **AC4**: 既有不退化 — `go test ./internal/eval/...` 全 PASS（含既有 `TestTask81_*` / `TestTask188_*` / `TestTask213_*`）；`go test ./...` 全 PASS；recall 度量函数签名 + gate 阈值不变（ADR-006）；本 PR 零 Rust delta（`cargo test --workspace` 不受影响）— verified by **TEST-24.2.4** + §10 实测
- [x] **AC5**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-24.2.5** + §10 记录

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-24.2.1 | 校验器 schema 良构 + 覆盖（良构过 / 不良 + 悬空被拒） | `internal/eval/eval.go` + `internal/eval/eval_test.go` | Done |
| TEST-24.2.2 | 重复检测（同 query / 同 (query,expected) 对被拒）+ 既有 ValidateDataset 不退化 | `internal/eval/eval.go` + `internal/eval/eval_test.go` | Done |
| TEST-24.2.3 | golden-semantic.jsonl 含代码符号 + CJK query case + 路径真实 + 过校验 | `test/fixtures/eval/golden-semantic.jsonl` + `internal/eval/eval_test.go` | Done |
| TEST-24.2.4 | `go test ./...` 全 PASS + 度量签名/gate 阈值不变 + 零 Rust delta | 全 Go | Done |
| TEST-24.2.5 | D2 lint `--touched origin/master` 0 未标注命中 | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（中）扩充 golden 的代码/CJK query case 在真实源码无清晰 expected**：代码符号 / CJK 短语在仓内可能多文件含 → expected 不唯一。
  - **缓解**：query case 选 expected 文件唯一性较高的真实符号（如 ContextForge 仓内独特标识符 / 文档独特 CJK 短语），expected 路径经核实存在；多文件含时用 `expected_sources` 列多源或选最强单源（沿用 `Question` 既有字段语义）。
- **R2（中）校验器与既有 `ValidateDataset` 语义冲突**：若改 `ValidateDataset` 内部断言可能退化 `TestTask81_*`。
  - **缓解**：校验器优先独立函数（add-only），`ValidateDataset` 内部仅 add-only 向后兼容补强；AC2 显式断言既有 `TestTask81_*` 不退化。
- **R3（低）误把 recall 数字产在本 task**：本 task scope 是数据集 + 校验器，非 recall。
  - **缓解**：真实 before/after recall delta 显式 [SPEC-OWNER:task-24.3-closeout-v0.17.0]；本 task §5.2 明确不预跑/预填 recall 数（ADR-013）。
- **R4（低）category 集扩充破坏既有 6 类校验**：新增 code-symbol/cjk 类可能与既有 `ValidateDataset` ≥6 类断言交互。
  - **缓解**：已知 category 集 add-only 扩（既有 6 类保留），扩充 golden 作独立数据集（不混入 `BuiltinGoldenQuestions` 的 30 题口径），AC4 覆盖既有不退化。

## 9. Verification Plan

```bash
# Go：eval 校验器 + 扩充 golden 数据集校验 + 既有不退化
go test ./internal/eval/... -v
go test ./...

# Rust 不受影响（本 PR 零 Rust delta）
cargo test --workspace

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**: 2026-05-31
- **改动文件**:
  - `internal/eval/eval.go`（修改）— 新增 `ValidateGoldenSemantic`（add-only 独立校验器：schema 良构 + 重复检测 + 覆盖）+ `knownCategories`（既有 6 类 add-only 加 `code-symbol` / `cjk`）；既有 `ValidateDataset` 语义未动
  - `internal/eval/eval_test.go`（修改）— 新增 TEST-24.2.1~24.2.4（校验器良构过 / 脏数据被拒 / golden-semantic 代码+CJK 路径真实 / gate 阈值不变）+ `validBaseQuestions` helper
  - `test/fixtures/eval/golden-semantic.jsonl`（新增）— 11 题（6 code-symbol：`build_tantivy_schema`/`tantivy_search`/`RetrieverConfig`/`open_with_config`/`BuiltinGoldenQuestions`/`json.Unmarshal` + 5 cjk：单驱动/向后兼容/治理自治/语义检索/禁伪造），每条 query→真实源码文件（路径经核实存在）
- **commit 列表**:
  - `55c07c1` test(eval): TEST-24.2.1~24.2.4 RED + golden-semantic.jsonl 代码/CJK 扩充
  - `500592f` feat(eval): ValidateGoldenSemantic 校验器（schema 良构 + 重复检测 + 覆盖），通过 TEST-24.2.1~24.2.4
  - （本 commit）docs(spec): 回填 task-24.2 §10 + Status → Done
- **§9 Verification 结果**（ADR-013 真实非合成，本机）:
  - unit-test: `go test ./internal/eval/...` ok（含既有 `TestTask81_*` / `TestTask188_*` / `TestTask213_*` + 新 4 测试，0 failed）；`go test ./...` 0 FAIL / 0 panic（一次 rerun 后稳定绿；首跑一处 known flake 复跑即绿）
  - build: ✅ `go vet ./internal/eval/...` 干净 + `gofmt -l` 0 文件
  - integration: `cargo test --workspace` 0 failed（零 Rust delta — 本 PR 不含 Rust 改动；gate 阈值常量 `GateTop5StrongMin`/`GateTop10StrongMin`/`GateSemanticRecall10Min` 不变，ADR-006）
  - lint: ✅ `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及 docs/specs 行 0 未标注命中（本机 scoped 复核 + CI spec-lint gate）
- **剩余风险 / 未做项**: golden-semantic.jsonl 为小语料（11 题，承 task-19.5 §10 小语料 caveat），真实 before/after recall delta 不在本 task 产出（ADR-013，在 task-24.3 据 task-24.1 tokenizer over 本数据集实测）；校验器为独立函数，未强制接入 recall harness 默认路径（add-only，避免改 `ValidateDataset` 现有调用方）。CJK 真正分词器口径 [SPEC-DEFER:phase-future.cjk-true-segmenter]、case_results 子表 [SPEC-DEFER:phase-future.case-results-subtable] 续 backlog。
- **下游 task 影响**: task-24.3（据 task-24.1 tokenizer over 本扩充 golden 实测真实 before/after recall delta + console_smoke v14 + v0.17.0 closeout + ADR-029 ratify）。本 PR 零 Rust delta，与 task-24.1 写路径不相交（Go vs Rust），二者已分别合入。
