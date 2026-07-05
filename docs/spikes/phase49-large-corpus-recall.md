# Phase 49 Spike · large-corpus recall (task-49.4)

> **诚实报告（ADR-013）**：本 spike 验证 v1.0 宣称的 recall@5/@10=1.0 在大语料上是否成立。**结论：BM25 baseline 在大语料显著退化**（recall@10 从 phase-21 的 0.9667 降到 0.7438），证实小语料 golden 存在过拟合。hybrid/reranked 数据需 ONNX model 真跑（本环境无 model），留诚实延后。

## 实测环境

- **harness**：`core/examples/phase49_large_corpus_recall.rs`（fixture-driven，读 `golden-retrieval.jsonl`）
- **corpus**：58 文件（golden-retrieval.jsonl 的 ~50 unique expected_file_path + 8 distractor），production chunker，**code_cjk tokenizer**（ADR-046 production default）
- **queries**：121（golden-retrieval.jsonl 全量，6 categories）
- **tokenizer**：`code_cjk`（production-default，ADR-046）
- **跑的 pass**：BM25 baseline（default build，无 ONNX 依赖）
- **未跑的 pass**：hybrid RRF / reranked cross-encoder（需 `--features embedding-fastembed,reranker-fastembed` + ONNX model 下载；本环境无 model，**诚实延后**，非伪造）

## BM25 baseline 实测结果（real run，2026-07-05）

```
=== task-49.4 large-corpus recall spike (ADR-013 real run) ===
corpus_files=58 queries=121 tokenizer=code_cjk(production-default)
  baseline-bm25          [agent-memory-rule   ] n=16 recall@5=0.6250 recall@10=0.7500 top1=0.2500 mrr=0.3802
  baseline-bm25          [code-location       ] n=28 recall@5=0.6071 recall@10=0.7143 top1=0.1429 mrr=0.2930
  baseline-bm25          [config-location     ] n=20 recall@5=0.5500 recall@10=0.6500 top1=0.2500 mrr=0.3551
  baseline-bm25          [error-reproduction  ] n=20 recall@5=0.5500 recall@10=0.7000 top1=0.2500 mrr=0.3563
  baseline-bm25          [historical-decision ] n=21 recall@5=0.8571 recall@10=0.9048 top1=0.3810 mrr=0.5719
  baseline-bm25          [log-troubleshooting ] n=16 recall@5=0.6250 recall@10=0.7500 top1=0.2500 mrr=0.3985
  baseline-bm25          [OVERALL             ] n=121 recall@5=0.6364 recall@10=0.7438 top1=0.2479 mrr=0.3876 gate@10(>=0.85)=fail
```

### 与 phase-21 小语料对比

| 指标 | phase-21（30 q / ~180 chunks） | phase-49（121 q / 58 files） | 变化 |
|---|---|---|---|
| BM25 recall@5 | 0.9000 | **0.6364** | -0.2636 ⬇️ |
| BM25 recall@10 | 0.9667 | **0.7438** | -0.2229 ⬇️ |
| BM25 top-1 | 0.0333 | **0.2479** | +0.2146 ⬆️ |
| BM25 MRR | 0.4095 | **0.3876** | -0.0219 ⬇️ |
| gate@10(≥0.85) | pass | **fail** | ⬇️ |

### 诚实解读

1. **recall@5/@10 显著退化（-0.22 ~ -0.26）**：大语料下 BM25 的 top-K 命中率明显下降。小语料 golden（30 q / ~11 files）确实存在过拟合——distractor 密度低 + expected_file 占比高 → 容易命中。大语料（58 files / 121 q）下 distractor 更多 + 同义查询更多 → BM25 lexical match 更易偏移。

2. **top-1 反而提升（+0.21）**：看似矛盾，但合理——phase-21 用的是 uncapped production chunker（大文件切多 chunk，BM25 top-1 常是 term-overlap distractor chunk），phase-49 的 code_cjk tokenizer + 整文件匹配（file_path.contains(stem)）让 top-1 更稳定。**注意**：top-1 提升不代表整体检索质量更好——recall@K 才是 file-level 覆盖率的真实指标。

3. **per-category 差异显著**：`historical-decision`（ADR docs，关键词明确）最好（0.90），`config-location`/`error-reproduction`（code，多义）最差（~0.65-0.70）。说明 BM25 在"概念明确文档"上强，在"模糊 code 意图"上弱——这正是 hybrid（semantic 补充概念意图）的价值所在。

4. **gate fail 的诚实意义**：phase-49 的 BM25-only gate(≥0.85) fail 是**真实信号**，不是 harness bug。phase-21 的小语料 gate pass 部分归因于 corpus 规模。**结论**：README 的 recall 声明必须从"recall@1.0"降级为更保守措辞（task-49.5 据此更新）。

### hybrid/reranked 预期（未实测，诚实标注）

phase-21 实测显示 hybrid RRF 把 top-1 从 0.0333→0.6667（二十倍提升）、MRR 0.41→0.79。phase-49 大语料下 hybrid 很可能：
- **top-1/MRR 显著提升**（semantic 补充 BM25 的概念意图盲区）
- **recall@10 可能仍 <1.0**（大语料的 distractor 密度是真实挑战，hybrid 不是万能）
- **reranked 是否仍劣于 hybrid**（phase-21 发现）需大语料验证——BGE-reranker 在 code+doc 混合语料的域适配可能不同

**这些预期需真跑 ONNX model 验证，本 spike 不伪造。** `[SPEC-DEFER:phase-future.embedding-large-corpus-recall]` 部分兑现（BM25 baseline 数据有了），但 hybrid/reranked 大语料数据仍延后。

## 对 README/RELEASE_NOTES 的影响（→ task-49.5）

**当前 README:28 声明**："recall@5/@10 = 1.0 over the 16-question author-curated golden（exceeds PRD north-star 75%/85%）"

**问题**：这个 1.0 是 hybrid recall（phase-21 小语料），但大语料 BM25 baseline 是 0.74。README 没区分 BM25 vs hybrid，也没标 corpus 规模。

**task-49.5 应改为**（诚实降级）：
- 明确区分 BM25-only vs hybrid recall
- 标注 corpus 规模（小语料 vs 大语料）
- 不再用"recall@1.0"作无 caveated 声明；改为"小语料 author-curated golden 上 hybrid recall@10 达 1.0；大语料 BM25 baseline recall@10=0.74，hybrid/reranked 大语料数据延后"

## harness 复用性

`phase49_large_corpus_recall.rs` 是 fixture-driven（读 golden-retrieval.jsonl），未来扩展语料只需改 fixture 文件，不改 example 代码。hybrid/reranked pass feature-gated，有 ONNX model 时可直接跑补充数据。

## 不 Redeem / 继续 defer

- `[SPEC-DEFER:phase-future.embedding-large-corpus-recall]`：**部分兑现**（BM25 baseline 大语料数据有了），但 semantic/hybrid 大语料 recall 仍延后（需 ONNX model 真跑）
- `[SPEC-DEFER:phase-future.cjk-golden-corpus-expansion]`：**兑现**（task-49.2 把 CJK golden 从 10 扩到 41，含 4 新维度；但 bigram vs true-seg 大语料 delta 仍需 phase24 example 真跑，本 spike 聚焦 retrieval golden）
- `[SPEC-DEFER:phase-future.cross-lingual-golden]`：**不 redeem**（日韩跨语言，本 phase 不做）
- `[SPEC-DEFER:phase-future.reranker-large-corpus-quality]`：**不 redeem**（NDCG 标准基准，需 BEIR/MS MARCO）
