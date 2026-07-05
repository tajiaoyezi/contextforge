# Phase 49 · eval-hardening (B4 v1.1)

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 是 **v2.0 前的 eval 硬化**——用更大语料（~500-1000 chunks）+ 更多 golden questions（~120 retrieval + ~80 semantic）实测，确认 v1.0 宣称的 hybrid recall@5/@10=1.0 是真本事还是 16-30 题小语料过拟合，并把 CJK golden 从 10 题扩到 ~40 题（验证 bigram vs true-segmenter 在更大语料是否仍 delta=0）。
>
> **方向锚点**（用户已定）：(1) 扩自建语料（非标准 benchmark BEIR/MS MARCO/MTEB）；(2) 保持软门 + 诚实报告（gate 不绑 CI exit code）；(3) CJK 扩到 ~40 题但不做日韩跨语言。
>
> **诚实核心（ADR-013）**：本 phase 的**预期价值就是可能暴露 recall<1.0**——这不是失败而是验证。所有实测数字如实记录，README/RELEASE_NOTES 据实更新，必要时把 "recall@1.0" 改为更保守措辞。
>
> **入读顺序**：本 phase spec → 5 个 task spec（49.1-49.5）→ 源码锚点（`internal/eval/eval.go` ValidateDataset/ValidateGoldenSemantic/knownCategories + `internal/cli/eval.go:42` dispatch 点 + `test/fixtures/eval/golden-semantic.jsonl` 现状 + `core/examples/phase21_hybrid_rerank_recall.rs` 可复用骨架）→ ADR-006（recall-eval-acceptance-gate）/ ADR-013（禁伪造）/ ADR-014（D1-D5，第四十一次激活）/ ADR-029（code-and-cjk-tokenizer）/ ADR-035（cjk-true-segmenter）/ ADR-046（tokenizer-default-on）。

## 1. 阶段目标

v1.0 的所有 recall 数字都在 ≤30 题作者策展 golden 上测的（phase-19/21 spike：40-180 chunks / 30 queries）。本 phase 用扩到 ~500-1000 chunks + ~200 golden questions 的更大语料实测，回答两个核心问题：

1. **recall@5/@10=1.0 是真本事还是小语料过拟合？** —— 用 ~120 题 retrieval golden（6 builtin categories × ~20）跑 BM25/semantic/hybrid/reranked，看大语料下是否退化、哪些 category 退化
2. **CJK bigram vs true-segmenter 在更大语料是否仍 delta=0？** —— 把 CJK golden 从 10 题扩到 ~40 题（含多词短语/CJK+ASCII 混合/自然语言长句），跑 phase24 tokenizer recall 对比

**具体 exit criteria（§6 AC）**：
1. **task-49.1** `golden-retrieval.jsonl`：~120 题 / 6 categories / 每类 ≥5 → 过 `ValidateDataset`（CLI 可跑）
2. **task-49.2** `golden-semantic.jsonl` 扩展：~80 题（~40 code-symbol + ~40 cjk）→ 过 `ValidateGoldenSemantic`
3. **task-49.3** CLI dispatch + `--strict` flag：`--dataset` 文件 ValidateDataset 失败降级 ValidateGoldenSemantic；`--strict` 强制前者
4. **task-49.4** 大语料 recall spike：fixture-driven（从 golden-retrieval.jsonl 读 queries），跑四 pass，结果写入 `docs/spikes/phase49-large-corpus-recall.md`（诚实报告）
5. **task-49.5** README/RELEASE_NOTES recall 声明更新 + defer marker 清理 + phase closeout
6. ADR-014 D1-D5（第四十一次激活）全通过

**版本号**：v1.1.0（Phase 49），theme eval-hardening。**minor release**（eval 体系硬化 + golden 扩展；gate 仍软门不 breaking；CLI 加 `--strict` flag 是 add-only）。

## 2. 业务价值

**核心价值：诚实验证**。当前 README 宣称 recall@1.0 超 PRD 北极星，但只在 16-30 题上测。如果大语料实测 recall 退化（如降到 0.85），现在的 README 声明就是**虚标**（违反 ADR-013）。本 phase 要么**确认** recall 是真本事（强化 v1.0 价值主张），要么**暴露**真实天花板并据实更新声明（消除虚标风险）。两种结果都是净正价值。

**次要价值**：CJK 扩展验证 bigram 默认（ADR-046）是否在大语料仍合理——如果 true-segmenter 在大语料有显著 recall 提升，可能需要重新评估默认选择。

### 49.1 golden-retrieval.jsonl（🟢 数据）
~120 题 / 6 builtin categories（config-location/error-reproduction/historical-decision/log-troubleshooting/agent-memory-rule/code-location），每类 ~20 题。查询基于真实 contextforge 源码设计，覆盖更多文件/更细粒度。

### 49.2 golden-semantic.jsonl 扩展（🟢 数据）
~80 题（~40 code-symbol + ~40 cjk）。CJK 扩展重点：多词短语 / CJK+ASCII 混合（`向量检索backend`）/ 自然语言长句 / code-CJK 标识符。code-symbol 扩展：更多 snake_case/PascalCase/dotted.path/kebab-case 模式。

### 49.3 CLI dispatch + --strict flag（🟢 代码）
`internal/cli/eval.go` 加 dispatch：`--dataset` 文件先过 `ValidateDataset`，失败降级 `ValidateGoldenSemantic`。新增 `--strict` flag 强制前者（CI/benchmark 场景）。

### 49.4 大语料 recall spike（🟢 refactor + 实测）
`core/examples/phase49_large_corpus_recall.rs`：fixture-driven（读 golden-retrieval.jsonl）+ refactor 出共享 corpus helper + 复用 phase21 production Retriever 路径。feature-gated，手动跑，结果入 spike doc。

### 49.5 closeout（🟢 文档）
README/RELEASE_NOTES recall 声明据 task-49.4 实测更新；redeem/继续 defer SPEC-DEFER；phase closeout + smoke gate。

**不在本 phase 范围**（诚实 OOS 清单，均已登记 SPEC-DEFER）：标准公共 benchmark（BEIR/MS MARCO/MTEB `[SPEC-DEFER:phase-future.embedding-large-corpus-recall]` 部分 / `[SPEC-DEFER:phase-future.reranker-large-corpus-quality]`，本 phase 用自建语料不接标准基准）/ gate 做 CI 硬门（用户定保持软门，ADR-013）/ CJK 跨语言日韩（`[SPEC-DEFER:phase-future.cross-lingual-golden]` + `[SPEC-DEFER:phase-future.multilang-segmenter]`，本 phase 仅扩中文）/ Rust-native eval runner（`[SPEC-DEFER:phase-future.rust-native-eval-runner]` 继续 defer，Go 为 source of truth）/ NDCG 新指标（本 phase 聚焦 recall@K + top-1/MRR）/ multi-user/auth（v2.0 B1，本 phase 无关）。

## 3. 涉及模块

- **49.1**: `test/fixtures/eval/golden-retrieval.jsonl`（新增）+ `internal/eval/eval_test.go`（TestTask491_*）
- **49.2**: `test/fixtures/eval/golden-semantic.jsonl`（扩展 16→~80）+ `internal/eval/eval_test.go`（TestTask492_*）
- **49.3**: `internal/cli/eval.go`（dispatch + --strict）+ `internal/cli/eval_test.go` 或 smoke_syntax_test.go（TestTask493_*）
- **49.4**: `core/examples/phase49_large_corpus_recall.rs`（新增）+ 可能 `core/examples/common/` 共享 helper + `docs/spikes/phase49-large-corpus-recall.md`（新增）
- **49.5**: `README.md` + `RELEASE_NOTES.md` + `docs/roadmap.md` + `docs/s2v-adapter.md` + `CHANGELOG.md` + redeem/defer marker

## 4.1 PRs
- 规划 PR（本 phase spec + 5 task spec + adapter/roadmap 索引）：chore/phase-49-specs → master
- task-49.1：feat/task-49.1-golden-retrieval-jsonl → master
- task-49.2：feat/task-49.2-golden-semantic-expand → master
- task-49.3：feat/task-49.3-cli-dispatch-strict → master
- task-49.4：feat/task-49.4-large-corpus-spike → master
- task-49.5（closeout）：feat/task-49.5-eval-hardening-closeout → master

## 5.1 Required Reading
- ADR-006（recall-eval-acceptance-gate）/ ADR-013（禁伪造红线）/ ADR-014（D1-D5）/ ADR-029（code-and-cjk-tokenizer）/ ADR-035（cjk-true-segmenter）/ ADR-046（tokenizer-default-on）
- `internal/eval/eval.go`（ValidateDataset / ValidateGoldenSemantic / knownCategories / Report / MeetsRecallGate）
- `internal/cli/eval.go`（runEval / parseEvalRunOpts / dispatch 点 :42）
- `test/fixtures/eval/golden-semantic.jsonl`（现状 16 题）
- `core/examples/phase21_hybrid_rerank_recall.rs`（可复用 production Retriever 骨架）
- `docs/spikes/phase-19-real-recall.md` + `docs/spikes/phase-21-hybrid-recall.md`（小语料基线 + 诚实 caveat 范式）

## 5. Behavior Contract
- `golden-retrieval.jsonl` 过 `ValidateDataset`（≥30 / ≥6 cat / ≥5 每 cat）
- `golden-semantic.jsonl` 扩展后过 `ValidateGoldenSemantic`（count-agnostic + knownCategories 闭集）
- CLI dispatch：`--dataset=X` 若 ValidateDataset 失败则降级 ValidateGoldenSemantic；`--strict` 强制 ValidateDataset
- spike：fixture-driven，feature-gated no-op stub（default features，[SPEC-OWNER:task-49.4] 占位编译锚点同 phase19/21 example 范式）+ 真实跑（手动）
- gate 仍软门（runEval exit 0 不变；ADR-013）

## 6. AC（Phase 级，每个 task §6 细化）
- [x] AC1: golden-retrieval.jsonl ~120 题 / 6 cat / 过 ValidateDataset — verified by task-49.1 §6
- [x] AC2: golden-semantic.jsonl ~80 题 / 过 ValidateGoldenSemantic — verified by task-49.2 §6
- [x] AC3: CLI dispatch + --strict flag 三路径覆盖 — verified by task-49.3 §6
- [x] AC4: 大语料 recall spike 诚实报告（实测数字 + caveat）— verified by task-49.4 §6
- [x] AC5: README/RELEASE_NOTES recall 声明与实测一致 + defer marker 清理 — verified by task-49.5 §6
- [x] AC6: ADR-014 D1-D5（第四十一次激活）全通过

## 8. Risks
- **recall<1.0（预期价值）**：大语料实测可能退化 → task-49.5 据实更新 README，不伪造
- **CJK delta 仍=0**：说明 bigram 默认正确，诚实记录
- **expected_file_path 主观判定**：用 top-10 命中作客观锚点，争议 case 标 notes
- **ONNX model 依赖**：spike 需手动跑，不进 CI（ADR-013）

## 9. Phase smoke gate
task-49.5（最后 task）跑 phase smoke：`go test ./internal/eval/ ./internal/cli/` + `cargo test -p contextforge-core` + spec_drift_lint + console_smoke（如触及）。
