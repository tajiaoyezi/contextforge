package cli

import (
	"context"
	"flag"
	"fmt"
	"io"
	"time"

	evalpkg "github.com/tajiaoyezi/contextforge/internal/eval"
	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

type evalRunOpts struct {
	Dataset     string
	Collection  string
	TopK        int32
	ExportJSONL string
}

func runEval(args []string, stdout, stderr io.Writer) int {
	if len(args) == 0 || args[0] != "run" {
		fmt.Fprintln(stderr, "contextforge eval: usage: contextforge eval run [--dataset=golden.jsonl] [--collection=default] [--top-k=10] [--export-jsonl=path]")
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
	if err := evalpkg.ValidateDataset(questions); err != nil {
		fmt.Fprintf(stderr, "contextforge eval run: invalid dataset: %v\n", err)
		return 1
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

	results := make([]evalpkg.Result, 0, len(questions))
	for _, q := range questions {
		ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
		started := time.Now()
		resp, err := fetchSearchResults(ctx, &contextforgev1.SearchRequest{
			Query:       q.Query,
			Collections: []string{opts.Collection},
			TopK:        opts.TopK,
			Explain:     true,
		})
		latency := time.Since(started)
		cancel()
		if err != nil {
			fmt.Fprintf(stderr, "contextforge eval run: search %q: %v\n", q.Query, err)
			return 1
		}
		results = append(results, evalpkg.EvaluateQuestion(q, resp.GetResults(), latency))
	}

	report := evalpkg.Summarize(results)
	fmt.Fprintf(stdout, "total=%d\n", report.Total)
	fmt.Fprintf(stdout, "top5_strong_hits=%d top5_strong_rate=%.4f\n", report.Top5StrongHits, report.Top5StrongRate)
	fmt.Fprintf(stdout, "top10_strong_hits=%d top10_strong_rate=%.4f\n", report.Top10StrongHits, report.Top10StrongRate)
	fmt.Fprintf(stdout, "weak_hits=%d misses=%d\n", report.WeakHits, report.Misses)
	fmt.Fprintf(stdout, "latency_p95_ms=%d\n", report.LatencyP95Millis)
	if len(report.MissCases) > 0 {
		fmt.Fprintln(stdout, "miss_cases:")
		for _, miss := range report.MissCases {
			fmt.Fprintf(stdout, "- category=%s query=%q expected=%s\n", miss.Category, miss.Query, miss.Expected)
		}
	}
	return 0
}

func parseEvalRunOpts(args []string, stderr io.Writer) (*evalRunOpts, error) {
	fs := flag.NewFlagSet("eval run", flag.ContinueOnError)
	fs.SetOutput(stderr)
	dataset := fs.String("dataset", "", "optional golden questions JSONL path; built-in dataset is used when omitted")
	collection := fs.String("collection", "default", "collection ID")
	topK := fs.Int("top-k", 10, "results per query; values <=0 fall back to 10")
	exportJSONL := fs.String("export-jsonl", "", "write the eval dataset JSONL to this path")
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
	}, nil
}
