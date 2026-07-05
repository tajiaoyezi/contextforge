package cli

import (
	"context"
	"flag"
	"fmt"
	"io"
	"sort"
	"time"

	evalpkg "github.com/tajiaoyezi/contextforge/internal/eval"
	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

type evalRunOpts struct {
	Dataset     string
	Collection  string
	TopK        int32
	ExportJSONL string
	Semantic    bool
	Hybrid      bool
	Rerank      bool
	Strict      bool
}

func runEval(args []string, stdout, stderr io.Writer) int {
	if len(args) == 0 || args[0] != "run" {
		fmt.Fprintln(stderr, "contextforge eval: usage: contextforge eval run [--dataset=golden.jsonl] [--collection=default] [--top-k=10] [--export-jsonl=path] [--semantic] [--hybrid] [--rerank] [--strict]")
		return 2
	}
	opts, err := parseEvalRunOpts(args[1:], stderr)
	if err != nil {
		return 2
	}
	questions := evalpkg.BuiltinGoldenQuestions()
	if opts.Dataset != "" {
		questions, err = evalpkg.LoadJSONL(opts.Dataset)
		if err != nil {
			fmt.Fprintf(stderr, "contextforge eval run: load dataset: %v\n", err)
			return 1
		}
	}
	// task-49.3: dispatch validation. --strict forces ValidateDataset (>=30/>=6cat/>=5each, the
	// builtin-shape contract for CI/benchmark). Default (soft) tries ValidateDataset first, then
	// falls back to ValidateGoldenSemantic (count-agnostic, for the smaller code-symbol/cjk golden)
	// so `--dataset=golden-semantic.jsonl` is CLI-runnable. Both must pass schema well-formedness.
	if err := evalpkg.ValidateDataset(questions); err != nil {
		if opts.Strict {
			fmt.Fprintf(stderr, "contextforge eval run: invalid dataset (--strict: ValidateDataset required): %v\n", err)
			return 1
		}
		// soft fallback: try the count-agnostic validator
		if err2 := evalpkg.ValidateGoldenSemantic(questions); err2 != nil {
			fmt.Fprintf(stderr, "contextforge eval run: invalid dataset (failed both ValidateDataset and ValidateGoldenSemantic):\n  ValidateDataset: %v\n  ValidateGoldenSemantic: %v\n", err, err2)
			return 1
		}
	}
	if opts.ExportJSONL != "" {
		if err := evalpkg.WriteJSONL(opts.ExportJSONL, questions); err != nil {
			fmt.Fprintf(stderr, "contextforge eval run: export jsonl: %v\n", err)
			return 1
		}
	}
	if fetchSearchResults == nil {
		fmt.Fprintln(stderr, "contextforge eval run: search backend not wired")
		return 1
	}

	bm25Results, err := evalSearchPass(questions, opts, passMode{}, stderr)
	if err != nil {
		return 1
	}
	// Optional passes (all opt-in, off → byte-equivalent legacy BM25 output via SummarizePasses):
	//   --semantic: vector path (SearchRequest.Semantic=true).
	//   --hybrid:   RRF fusion path (SearchRequest.Hybrid=true → daemon search_hybrid, task-21.1).
	//   --rerank:   re-order the best available base pass (hybrid > semantic > BM25) by the
	//               deterministic IdentityReranker contract at the eval layer (ADR-026 D2).
	var passes evalpkg.Passes
	if opts.Semantic {
		passes.Semantic, err = evalSearchPass(questions, opts, passMode{semantic: true}, stderr)
		if err != nil {
			return 1
		}
	}
	if opts.Hybrid {
		passes.Hybrid, err = evalSearchPass(questions, opts, passMode{hybrid: true}, stderr)
		if err != nil {
			return 1
		}
	}
	if opts.Rerank {
		base := passMode{rerank: true}
		switch {
		case opts.Hybrid:
			base.hybrid = true
		case opts.Semantic:
			base.semantic = true
		}
		passes.Reranked, err = evalSearchPass(questions, opts, base, stderr)
		if err != nil {
			return 1
		}
	}

	report := evalpkg.SummarizePasses(bm25Results, passes)
	fmt.Fprintf(stdout, "total=%d\n", report.Total)
	fmt.Fprintf(stdout, "top5_strong_hits=%d top5_strong_rate=%.4f\n", report.Top5StrongHits, report.Top5StrongRate)
	fmt.Fprintf(stdout, "top10_strong_hits=%d top10_strong_rate=%.4f\n", report.Top10StrongHits, report.Top10StrongRate)
	fmt.Fprintf(stdout, "weak_hits=%d misses=%d\n", report.WeakHits, report.Misses)
	fmt.Fprintf(stdout, "latency_p95_ms=%d\n", report.LatencyP95Millis)
	if report.SemanticEvaluated {
		fmt.Fprintf(stdout, "semantic_strong_hits_top5=%d semantic_recall_at_5=%.4f\n", report.SemanticStrongHits5, report.SemanticRecallAt5)
		fmt.Fprintf(stdout, "semantic_strong_hits_top10=%d semantic_recall_at_10=%.4f\n", report.SemanticStrongHits10, report.SemanticRecallAt10)
		fmt.Fprintf(stdout, "semantic_weak_hits=%d semantic_misses=%d\n", report.SemanticWeakHits, report.SemanticMisses)
	}
	if report.HybridEvaluated {
		fmt.Fprintf(stdout, "hybrid_strong_hits_top5=%d hybrid_recall_at_5=%.4f\n", report.HybridStrongHits5, report.HybridRecallAt5)
		fmt.Fprintf(stdout, "hybrid_strong_hits_top10=%d hybrid_recall_at_10=%.4f\n", report.HybridStrongHits10, report.HybridRecallAt10)
		fmt.Fprintf(stdout, "hybrid_weak_hits=%d hybrid_misses=%d\n", report.HybridWeakHits, report.HybridMisses)
	}
	if report.RerankedEvaluated {
		fmt.Fprintf(stdout, "reranked_strong_hits_top5=%d reranked_recall_at_5=%.4f\n", report.RerankedStrongHits5, report.RerankedRecallAt5)
		fmt.Fprintf(stdout, "reranked_strong_hits_top10=%d reranked_recall_at_10=%.4f\n", report.RerankedStrongHits10, report.RerankedRecallAt10)
		fmt.Fprintf(stdout, "reranked_weak_hits=%d reranked_misses=%d\n", report.RerankedWeakHits, report.RerankedMisses)
	}
	if len(report.MissCases) > 0 {
		fmt.Fprintln(stdout, "miss_cases:")
		for _, miss := range report.MissCases {
			fmt.Fprintf(stdout, "- category=%s query=%q expected=%s\n", miss.Category, miss.Query, miss.Expected)
		}
	}
	// Recall gate (ADR-006 A1 + task-18.8 + task-21.3): BM25 thresholds always; SemanticRecall@10 /
	// HybridRecall@10 / RerankedRecall@10 only when that pass ran. ADR-013: the gate is printed for
	// human judgement — it does NOT bind the CLI exit code; real hybrid/rerank recall vs the baseline
	// comes from the dogfood eval (docs/spikes/phase-21-hybrid-recall.md), this is wiring not a verdict.
	ok, failures := evalpkg.MeetsRecallGate(report)
	if ok {
		fmt.Fprintln(stdout, "gate=pass")
	} else {
		fmt.Fprintln(stdout, "gate=fail")
		for _, f := range failures {
			fmt.Fprintf(stdout, "- gate_failure=%s\n", f)
		}
	}
	return 0
}

// passMode selects which retrieval method one eval pass exercises (task-21.3, add-only; the zero
// value is the BM25 baseline). semantic/hybrid set the matching add-only SearchRequest fields; rerank
// re-orders the fetched top-k at the eval layer (see rerankIdentity).
type passMode struct {
	semantic bool
	hybrid   bool
	rerank   bool
}

// evalSearchPass runs one retrieval pass over the whole question set, returning the per-question
// outcomes. The zero passMode is the BM25 baseline; semantic/hybrid set the add-only SearchRequest
// fields; rerank applies rerankIdentity to each response's top-k before scoring. A nil error means
// every query succeeded; on the first failure it reports to stderr and returns the error.
func evalSearchPass(questions []evalpkg.Question, opts *evalRunOpts, mode passMode, stderr io.Writer) ([]evalpkg.Result, error) {
	results := make([]evalpkg.Result, 0, len(questions))
	for _, q := range questions {
		ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
		started := time.Now()
		resp, err := fetchSearchResults(ctx, &contextforgev1.SearchRequest{
			Query:       q.Query,
			Collections: []string{opts.Collection},
			TopK:        opts.TopK,
			Explain:     true,
			Semantic:    mode.semantic,
			Hybrid:      mode.hybrid,
		})
		latency := time.Since(started)
		cancel()
		if err != nil {
			fmt.Fprintf(stderr, "contextforge eval run: search %q: %v\n", q.Query, err)
			return nil, err
		}
		items := resp.GetResults()
		if mode.rerank {
			items = rerankIdentity(items)
		}
		results = append(results, evalpkg.EvaluateQuestion(q, items, latency))
	}
	return results, nil
}

// rerankIdentity re-orders the fetched top-k by the deterministic IdentityReranker contract (ADR-026
// D2: relevance score desc, chunk_id asc stable tie-break, no candidate dropped — see
// core/src/rerank/identity.rs). The daemon Query RPC does not expose the reranker seam (console-api
// rerank forward is server-side env-driven per ADR-043 D3, with provenance visibility fulfilled in
// Phase 39 / ADR-044 — the per-request ?rerank control was superseded, not deferred-open), so the
// eval layer applies the deterministic default here to populate the reranked-recall column. Real
// cross-encoder uplift comes from the Rust dogfood eval (docs/spikes/phase-21-hybrid-recall.md),
// never faked here (ADR-013).
func rerankIdentity(in []*contextforgev1.RetrievalResult) []*contextforgev1.RetrievalResult {
	out := make([]*contextforgev1.RetrievalResult, len(in))
	copy(out, in)
	sort.SliceStable(out, func(i, j int) bool {
		if si, sj := out[i].GetScore(), out[j].GetScore(); si != sj {
			return si > sj
		}
		return out[i].GetChunkId() < out[j].GetChunkId()
	})
	return out
}

func parseEvalRunOpts(args []string, stderr io.Writer) (*evalRunOpts, error) {
	fs := flag.NewFlagSet("eval run", flag.ContinueOnError)
	fs.SetOutput(stderr)
	dataset := fs.String("dataset", "", "optional golden questions JSONL path; built-in dataset is used when omitted")
	collection := fs.String("collection", "default", "collection ID")
	topK := fs.Int("top-k", 10, "results per query; values <=0 fall back to 10")
	exportJSONL := fs.String("export-jsonl", "", "write the eval dataset JSONL to this path")
	semantic := fs.Bool("semantic", false, "also run the semantic (vector) retrieval path and report SemanticRecall@K + recall gate")
	hybrid := fs.Bool("hybrid", false, "also run the hybrid (RRF BM25+vector fusion) path and report HybridRecall@K + recall gate")
	rerank := fs.Bool("rerank", false, "also run a reranked pass (deterministic IdentityReranker over the best base pass) and report RerankedRecall@K + recall gate")
	strict := fs.Bool("strict", false, "force ValidateDataset (>=30 questions / >=6 categories / >=5 per category); without this flag, ValidateDataset failure falls back to ValidateGoldenSemantic")
	if err := fs.Parse(args); err != nil {
		return nil, err
	}
	k := int32(*topK)
	if k <= 0 {
		k = 10
	}
	return &evalRunOpts{
		Dataset:     *dataset,
		Collection:  *collection,
		TopK:        k,
		ExportJSONL: *exportJSONL,
		Semantic:    *semantic,
		Hybrid:      *hybrid,
		Rerank:      *rerank,
		Strict:      *strict,
	}, nil
}
