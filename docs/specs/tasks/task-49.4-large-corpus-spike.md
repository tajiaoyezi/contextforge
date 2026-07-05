# Task `49.4`: `large-corpus-spike — fixture-driven 大语料 recall spike（诚实实测）`

**Status**: Done
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 49 (eval-hardening)
**Dependencies**: task-49.1（golden-retrieval.jsonl）/ task-49.2（golden-semantic.jsonl）/ task-49.3（CLI dispatch）/ ADR-006 / ADR-013 / ADR-025 / ADR-026 / ADR-027 / ADR-035 / ADR-046
**Required Reading**: phase-49-eval-hardening.md / core/examples/phase21_hybrid_rerank_recall.rs（production Retriever 骨架）/ core/examples/phase19_real_recall.rs（embedding dump 范式）/ docs/spikes/phase-19-real-recall.md + phase-21-hybrid-recall.md（小语料基线 + caveat 范式）

## 1. Background
phase-19/21 spike 在 40-180 chunks / 30 queries 上测了 recall（hybrid recall@5/@10=1.0）。但 examples 的语料/queries 全是硬编码 literal，不是 fixture-driven。v1.1 要在 ~500-1000 chunks / ~120 queries 上实测，必须 refactor 出 fixture-driven 路径。

## 2. Goal
新增 `core/examples/phase49_large_corpus_recall.rs`：fixture-driven（从 `golden-retrieval.jsonl` 读 queries）+ refactor 出共享 corpus helper（写真实源码 tree 到 temp）+ 复用 phase21 production Retriever 路径。跑 BM25/semantic/hybrid/reranked 四 pass，测 recall@5/@10/top-1/MRR。结果写入 `docs/spikes/phase49-large-corpus-recall.md`（诚实报告）。

## 3. Scope
- 新增 `core/examples/phase49_large_corpus_recall.rs`：
  - default features = no-op stub（`#[cfg(not(feature = "embedding-fastembed"))]`，[SPEC-OWNER:task-49.4] 编译锚点同 phase19/21 example 范式，非未实现占位）
  - 真实跑需 `--features embedding-fastembed,reranker-fastembed`
  - **fixture-driven**：`LoadJSONL("test/fixtures/eval/golden-retrieval.jsonl")` 读 queries（不再硬编码 cats 数组）
  - **corpus**：从真实 contextforge 源码 tree 写入 temp（复用 phase21 的 distractor + expected 文件写入，扩到 ~500-1000 chunks）
  - 复用 production `IndexSession` + `Retriever::search/search_hybrid/with_reranker`
  - 输出：println 结构化报告（recall@5/@10/top-1/MRR per category + overall）+ 可选写 json
- 可能 refactor 出 `core/examples/common/mod.rs` 或 `core/examples/common_corpus.rs` 共享 helper（corpus 写入 + query 加载）—— 评估是否值得抽（若 phase19/21/49 三处重复则抽）
- 新增 `docs/spikes/phase49-large-corpus-recall.md`：实测结果 + 诚实 caveat + 与 phase-21 小语料对比
- **不进 CI**（feature-gated + 需 ONNX model；ADR-013 禁伪造——CI 只跑 wiring 不跑真实质量）

## 4.1 诚实报告要求（ADR-013）
spike doc 必须包含：
- **实测 corpus 规模**：X files / Y chunks / Z queries（具体数字）
- **四 pass recall@5/@10/top-1/MRR**：overall + per category
- **与 phase-21 对比**：小语料（40-180 chunks）vs 大语料（~500-1000）的 recall 变化
- **诚实 caveat**：是否退化 / 哪些 category 退化 / 可能原因（chunker 粒度 / distractor 密度 / embedding 模型域适配）
- **gate 结果**：gate=pass/fail（软门，仅 print）+ 各 pass 是否过阈值
- **CJK 对比**（如跑）：bigram vs true-segmenter 在扩展 CJK golden 上的 recall delta

## 6. AC
- [x] **AC1**: example 编译；default features 下跑 BM25 baseline（无 ONNX 依赖）；`--features embedding-fastembed,reranker-fastembed` 下真实 harness 编译 — verified by **TEST-49.4.1**（cargo build 两路径均 ✅）
- [x] **AC2**: 真实跑（手动）产出数据填入 spike doc；spike doc 含 corpus 规模 + recall + 与 phase-21 对比 + caveat — verified by **TEST-49.4.2**（BM25 实测数据已入 doc；hybrid/reranked 诚实延后——本环境无 ONNX model）
- [x] **AC3**: fixture-driven（从 golden-retrieval.jsonl 读 queries，非硬编码）— verified by **TEST-49.4.3**（serde_json::from_str 读 JSONL fixture）

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-49.4.1 | example 双路径编译（default BM25 + features 真实 harness） | cargo build | Done |
| TEST-49.4.2 | spike doc 含实测数字 + 对比 + caveat | doc 字段检查 | Done |
| TEST-49.4.3 | fixture-driven（serde_json 读 golden-retrieval.jsonl） | grep | Done |

## 9. Verification
```bash
# default features 编译（no-op stub, [SPEC-OWNER:task-49.4] 编译锚点）
cargo build --example phase49_large_corpus_recall
# feature-gated 编译（真实 harness）
cargo build --example phase49_large_corpus_recall --features embedding-fastembed,reranker-fastembed
# 真实跑（手动，需 ONNX model 下载）
# cargo run --example phase49_large_corpus_recall --features embedding-fastembed,reranker-fastembed
# spike doc 字段检查
grep -qE 'corpus.*chunks|recall@5|recall@10|phase-21.*对比|caveat' docs/spikes/phase49-large-corpus-recall.md
# fixture-driven 验证
grep -q 'LoadJSONL\|golden-retrieval' core/examples/phase49_large_corpus_recall.rs
```

## 10. Completion Notes
**Status**: Done

1. **完成日期**：2026-07-05
2. **改动文件**：
   - core/examples/phase49_large_corpus_recall.rs（新增，fixture-driven BM25 + feature-gated hybrid/reranked）
   - docs/spikes/phase49-large-corpus-recall.md（新增，诚实报告）
3. **commit 列表**：
   - <GREEN> feat(eval): task-49.4 large-corpus recall spike + BM25 实测数据
4. **§9 Verification 结果**：
   - build: ✅（default features 编译 + 跑 BM25；feature-gated 编译 ✅）
   - cargo check --features embedding-fastembed,reranker-fastembed: ✅
   - BM25 实测：corpus 58 files / 121 queries / recall@5=0.6364 / recall@10=0.7438 / top1=0.2479 / gate=fail
   - hybrid/reranked: 诚实延后（本环境无 ONNX model；需 `--features embedding-fastembed,reranker-fastembed` + model 下载）
5. **剩余风险**：**核心发现——大语料 BM25 recall 显著退化**（recall@10 从 phase-21 的 0.9667 降到 0.7438，gate fail）。这是预期价值（暴露小语料过拟合），不是 harness bug。task-49.5 必须据实把 README 的 "recall@1.0" 降级为更保守措辞。hybrid/reranked 大语料数据仍延后（需 ONNX model）。
6. **下游影响**：task-49.5（README/RELEASE_NOTES recall 声明据本 task 实测 BM25=0.74 更新；明确区分 BM25 vs hybrid + 标 corpus 规模）
