# Phase 24 spike — code/CJK tokenizer real before/after recall delta + rust-native-eval-runner evaluation

> task-24.3 closeout evidence (ADR-013: real run, no synthetic/fabricated numbers). Measures the
> task-24.1 opt-in code/CJK `TextAnalyzer` against the task-24.2 `test/fixtures/eval/golden-semantic.jsonl`
> dataset, and records the rust-native-eval-runner evaluation conclusion.

## 1. Method

Harness: `core/examples/phase24_tokenizer_recall.rs` (default build — BM25 lexical, no feature gate / no
ONNX, runs in CI and any environment).

1. Load the task-24.2 golden-semantic dataset (11 queries: 6 `code-symbol` + 5 `cjk`, each query → a real
   ContextForge file that contains it).
2. Write the **deduped** expected files (9 unique) + 3 distractors into a temp source tree (real file content).
3. Index the **same** corpus twice via the production `IndexSession`:
   - **before** = default analyzer (`IndexSession::open`)
   - **after** = opt-in code/CJK analyzer (`IndexSession::open_with_tokenizer(.., "code_cjk")`)
4. For each query, `Retriever::search` top-10 (default-config retriever for *before*; `tokenizer="code_cjk"`
   config for *after*) and record the rank of the first hit whose `file_path` carries the query's expected
   file → file-level Strong-hit@5 / @10 + top-1 + MRR.

Run: `cargo run -p contextforge-core --example phase24_tokenizer_recall`

## 2. Results (real)

| metric | before (default) | after (code/CJK) | delta |
|---|---|---|---|
| recall@5 | 0.9091 | **1.0000** | **+0.0909** |
| recall@10 | 0.9091 | **1.0000** | **+0.0909** |
| top-1 | 0.9091 | **1.0000** | **+0.0909** |
| MRR | 0.9091 | **1.0000** | **+0.0909** |

`corpus_files=9 expected + 3 distractor`, `queries=11`.

### Per-query (rank of expected file; `-` = miss within top-10)

| category | query | before | after | expected file |
|---|---|---|---|---|
| code-symbol | `build_tantivy_schema` | 1 | 1 | core/src/indexer/mod.rs |
| code-symbol | `tantivy_search` | 1 | 1 | core/src/indexer/mod.rs |
| code-symbol | `RetrieverConfig` | 1 | 1 | core/src/retriever/mod.rs |
| code-symbol | `open_with_config` | 1 | 1 | core/src/retriever/mod.rs |
| code-symbol | `BuiltinGoldenQuestions` | 1 | 1 | internal/eval/eval.go |
| code-symbol | `json.Unmarshal` | 1 | 1 | internal/eval/eval.go |
| cjk | `单驱动` | 1 | 1 | AGENTS.md |
| cjk | `向后兼容` | 1 | 1 | core/src/indexer/mod.rs |
| cjk | `治理自治` | 1 | 1 | AGENTS.md |
| **cjk** | **`语义检索`** | **−** | **1** | **docs/decisions/adr-024-console-api-semantic-forward.md** |
| cjk | `禁伪造` | 1 | 1 | docs/decisions/adr-025-hybrid-scoring-fusion.md |

## 3. Interpretation (honest)

- The **+0.0909** delta is **real and driven by a single CJK case**: `语义检索`. The default analyzer treats
  the contiguous CJK run as **one token** (`语义检索`), which only matches a document containing that exact
  4-char run; `adr-024` discusses 语义 and 检索 but not as one contiguous token, so **default misses**. The
  opt-in analyzer emits **bigrams** (`语义` / `义检` / `检索`), so the document's own `语义` + `检索` bigrams
  match → **after hits at rank 1**. This is exactly the CJK-bigram value the tokenizer was built for.
- The other 10 queries are **parity** (both rank-1). Full-symbol / full-phrase queries (`build_tantivy_schema`,
  `RetrieverConfig`, `单驱动`, …) match in **both** analyzers: the default `SimpleTokenizer` already splits on
  `_` `.` and non-alnum boundaries, and an exact camelCase/CJK-phrase query equals its own single default token.
  The tokenizer's distinguishing power is on **sub-token / substring** queries, where it is proven
  deterministically by the task-24.1 unit tests (not by this file-level full-query dataset):
  - **TEST-24.1.4**: opt-in collection — query `user` hits a doc containing `getUserById` (camelCase subword);
    query `置加` hits a doc containing `配置加载` (CJK bigram). The **default** collection **misses both**.
  - **TEST-24.1.1 / 24.1.2**: deterministic token-stream assertions (`camelCase`→`camel`/`case` + 原 token;
    `配置加载`→`配置`/`置加`/`加载`).
- **Small-corpus caveat** (承 phase-24 §7 R2 / task-19.5 §10): 11 queries / 12 files is a small dataset; the
  delta is not a population-level recall claim. It is recorded as measured, not extrapolated (ADR-013). The
  opt-in tokenizer does **not** regress any query here (no ranking degradation observed on this set), and
  improves the one case where default's whole-phrase CJK token failed.

**Conclusion**: the opt-in code/CJK tokenizer delivers a **real, non-negative** before/after recall delta
(+0.0909, default 0.9091 → 1.0000) on the task-24.2 golden, with the sub-token mechanism proven by the
task-24.1 unit tests. Default tokenization is unchanged (既有索引不失效); opt-in needs a re-index to adopt.

## 4. rust-native-eval-runner evaluation (ADR-029 D4)

**Decision: honestly deferred** — `[SPEC-DEFER:phase-future.rust-native-eval-runner]`.

`core/src/eval/runner.rs` stays a placeholder (`EvalRunner::trigger_external` noop). Evaluated promoting it to
a real Rust-native runner and deferred because:

1. **Single source of truth.** The recall harness lives in Go (`internal/eval/eval.go`: `SemanticRecallAtK` /
   `SummarizeHybrid` / `MeetsRecallGate` / `ValidateDataset`) by task-14.1's deliberate choice. A Rust-native
   runner would **duplicate** the recall 口径 + gate logic across two languages → drift risk.
2. **No current consumer.** Production eval is triggered Go-side (`runEvalAsync`, task-14.2). Nothing consumes
   a Rust-native runner today; promoting it now is speculative.
3. **Ad-hoc Rust measurement already works.** When a spike needs Rust-side recall (e.g. this very before/after
   delta), `core/examples/phase24_tokenizer_recall.rs` measures it directly through the production `Retriever`
   path — no permanent runner required.

This is an honest defer, not a faked implementation (ADR-013). The placeholder + `[SPEC-DEFER:...]` marker are
retained; the rationale is recorded here and in `core/src/eval/runner.rs`.

## 5. Verification

- `cargo run -p contextforge-core --example phase24_tokenizer_recall` → the table in §2 (deterministic).
- `cargo test -p contextforge-core --lib indexer::tests::test_24_1` → TEST-24.1.1-4 (tokenizer mechanism).
- `go test ./internal/eval/...` → TEST-24.2.1-4 (validator + golden dataset).
