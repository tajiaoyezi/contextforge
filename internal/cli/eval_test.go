package cli

import (
	"bytes"
	"context"
	"os"
	"path/filepath"
	"strings"
	"testing"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// TEST-8.1.3 / SCEN-8.1.3 / AC3 and TEST-8.1.5 / SCEN-8.1.5 / AC5
func TestTask81_AC3_AC5_RunEvalUsesSearchBackendAndExportsJSONL(t *testing.T) {
	orig := fetchSearchResults
	defer func() { fetchSearchResults = orig }()

	var calls int
	fetchSearchResults = func(ctx context.Context, req *contextforgev1.SearchRequest) (*contextforgev1.SearchResponse, error) {
		calls++
		return &contextforgev1.SearchResponse{
			Results: []*contextforgev1.RetrievalResult{
				{
					ChunkId:   "builtin-config-location-1",
					FilePath:  "internal/config/config.go",
					LineStart: 1,
					LineEnd:   80,
				},
			},
		}, nil
	}

	exportPath := filepath.Join(t.TempDir(), "golden.jsonl")
	var stdout, stderr bytes.Buffer
	code := runEval([]string{"run", "--collection=default", "--export-jsonl=" + exportPath}, &stdout, &stderr)
	if code != 0 {
		t.Fatalf("runEval exit=%d stderr=%q", code, stderr.String())
	}
	if calls != 30 {
		t.Fatalf("eval run should query once per built-in golden question, calls=%d want 30", calls)
	}
	out := stdout.String()
	for _, want := range []string{"total=30", "top5_strong_rate=", "top10_strong_rate=", "latency_p95_ms=", "misses="} {
		if !strings.Contains(out, want) {
			t.Fatalf("eval output missing %q:\n%s", want, out)
		}
	}
	body, err := os.ReadFile(exportPath)
	if err != nil {
		t.Fatalf("export JSONL missing: %v", err)
	}
	if !bytes.Contains(body, []byte(`"query"`)) || !bytes.Contains(body, []byte(`"expected_file_path"`)) {
		t.Fatalf("export JSONL missing required fields:\n%s", body)
	}
}

// TEST-19.4.1 / AC1: parseEvalRunOpts parses --semantic (default false, given → true).
func TestTask194_AC1_ParseSemanticFlag(t *testing.T) {
	var stderr bytes.Buffer
	def, err := parseEvalRunOpts([]string{"--collection=default"}, &stderr)
	if err != nil {
		t.Fatalf("parse without --semantic: %v (stderr=%q)", err, stderr.String())
	}
	if def.Semantic {
		t.Fatalf("Semantic should default to false, got true")
	}
	on, err := parseEvalRunOpts([]string{"--collection=default", "--semantic"}, &stderr)
	if err != nil {
		t.Fatalf("parse with --semantic: %v (stderr=%q)", err, stderr.String())
	}
	if !on.Semantic {
		t.Fatalf("--semantic should set Semantic=true, got false")
	}
}

// TEST-19.4.2 / AC2+AC3: runEval --semantic issues BM25 + semantic passes (60 calls = 30 × 2),
// summarizes via SummarizeHybrid (semantic_recall_at_10= line) and prints the recall gate verdict;
// a gate failure still exits 0 (ADR-013). BM25-only (no --semantic) stays at 30 calls.
func TestTask194_AC2_AC3_RunEvalSemanticDualPath(t *testing.T) {
	orig := fetchSearchResults
	defer func() { fetchSearchResults = orig }()

	// Backend that returns a strong hit for the expected chunk only on the BM25 pass, and a miss on
	// the semantic pass — so the SemanticRecall@10 gate (0.70) fails and we can assert exit 0 + gate=fail.
	var bm25Calls, semanticCalls int
	fetchSearchResults = func(ctx context.Context, req *contextforgev1.SearchRequest) (*contextforgev1.SearchResponse, error) {
		if req.GetSemantic() {
			semanticCalls++
			return &contextforgev1.SearchResponse{Results: []*contextforgev1.RetrievalResult{
				{ChunkId: "unrelated-chunk", FilePath: "no/such/file.go", LineStart: 1, LineEnd: 1},
			}}, nil
		}
		bm25Calls++
		return &contextforgev1.SearchResponse{Results: []*contextforgev1.RetrievalResult{
			{ChunkId: "builtin-config-location-1", FilePath: "internal/config/config.go", LineStart: 1, LineEnd: 80},
		}}, nil
	}

	var stdout, stderr bytes.Buffer
	code := runEval([]string{"run", "--collection=default", "--semantic"}, &stdout, &stderr)
	if code != 0 {
		t.Fatalf("runEval --semantic exit=%d stderr=%q", code, stderr.String())
	}
	if bm25Calls != 30 || semanticCalls != 30 {
		t.Fatalf("expected 30 BM25 + 30 semantic calls (60 total), got bm25=%d semantic=%d", bm25Calls, semanticCalls)
	}
	out := stdout.String()
	for _, want := range []string{"total=30", "semantic_recall_at_5=", "semantic_recall_at_10=", "gate="} {
		if !strings.Contains(out, want) {
			t.Fatalf("eval --semantic output missing %q:\n%s", want, out)
		}
	}
	// Semantic pass returns only misses → SemanticRecall@10 = 0 < 0.70 → gate=fail, but exit still 0.
	if !strings.Contains(out, "gate=fail") {
		t.Fatalf("expected gate=fail (semantic recall 0 < 0.70), output:\n%s", out)
	}

	// Backward compat: without --semantic the semantic pass is skipped (30 calls, no semantic lines).
	bm25Calls, semanticCalls = 0, 0
	stdout.Reset()
	stderr.Reset()
	code = runEval([]string{"run", "--collection=default"}, &stdout, &stderr)
	if code != 0 {
		t.Fatalf("runEval (BM25-only) exit=%d stderr=%q", code, stderr.String())
	}
	if bm25Calls != 30 || semanticCalls != 0 {
		t.Fatalf("BM25-only should issue 30 calls and no semantic pass, got bm25=%d semantic=%d", bm25Calls, semanticCalls)
	}
	if strings.Contains(stdout.String(), "semantic_recall_at_10=") {
		t.Fatalf("BM25-only output should not include semantic recall lines:\n%s", stdout.String())
	}
}

// TEST-21.3.1 / AC1: parseEvalRunOpts parses --hybrid and --rerank (default false, given → true),
// mirroring the task-19.4 --semantic flag.
func TestTask213_AC1_ParseHybridRerankFlags(t *testing.T) {
	var stderr bytes.Buffer
	def, err := parseEvalRunOpts([]string{"--collection=default"}, &stderr)
	if err != nil {
		t.Fatalf("parse without flags: %v (stderr=%q)", err, stderr.String())
	}
	if def.Hybrid || def.Rerank {
		t.Fatalf("Hybrid/Rerank should default false, got hybrid=%v rerank=%v", def.Hybrid, def.Rerank)
	}
	on, err := parseEvalRunOpts([]string{"--collection=default", "--hybrid", "--rerank"}, &stderr)
	if err != nil {
		t.Fatalf("parse --hybrid --rerank: %v (stderr=%q)", err, stderr.String())
	}
	if !on.Hybrid || !on.Rerank {
		t.Fatalf("--hybrid/--rerank should set true, got hybrid=%v rerank=%v", on.Hybrid, on.Rerank)
	}
}

// TEST-21.3.1 / AC1: runEval --hybrid --rerank issues the BM25 baseline + a hybrid pass (req.Hybrid=true
// → daemon search_hybrid, task-21.1) + a reranked pass (the deterministic IdentityReranker contract
// applied to the hybrid top-k at the eval layer, ADR-026 D2), summarizes via SummarizePasses and prints
// the add-only hybrid_recall_at_10= / reranked_recall_at_10= lines + the gate, exiting 0 (ADR-013 — a
// gate failure does not bind the exit code). Plain `eval run` stays BM25-only (no extra passes/lines).
func TestTask213_AC1_RunEvalHybridRerankMultiPath(t *testing.T) {
	orig := fetchSearchResults
	defer func() { fetchSearchResults = orig }()

	var bm25Calls, hybridCalls int
	fetchSearchResults = func(ctx context.Context, req *contextforgev1.SearchRequest) (*contextforgev1.SearchResponse, error) {
		if req.GetHybrid() {
			hybridCalls++
		} else {
			bm25Calls++
		}
		return &contextforgev1.SearchResponse{Results: []*contextforgev1.RetrievalResult{
			{ChunkId: "builtin-config-location-1", FilePath: "internal/config/config.go", LineStart: 1, LineEnd: 80},
		}}, nil
	}

	var stdout, stderr bytes.Buffer
	code := runEval([]string{"run", "--collection=default", "--hybrid", "--rerank"}, &stdout, &stderr)
	if code != 0 {
		t.Fatalf("runEval --hybrid --rerank exit=%d stderr=%q", code, stderr.String())
	}
	// BM25 baseline (30) + hybrid pass (30) + reranked-on-hybrid pass (30 more, req.Hybrid=true).
	if bm25Calls != 30 || hybridCalls != 60 {
		t.Fatalf("expected 30 BM25 + 60 hybrid-flag calls (hybrid + reranked-on-hybrid), got bm25=%d hybrid=%d", bm25Calls, hybridCalls)
	}
	out := stdout.String()
	for _, want := range []string{"total=30", "hybrid_recall_at_10=", "reranked_recall_at_10=", "gate="} {
		if !strings.Contains(out, want) {
			t.Fatalf("eval --hybrid --rerank output missing %q:\n%s", want, out)
		}
	}

	// Backward compat: plain `eval run` → 30 BM25 calls only, no hybrid/reranked lines.
	bm25Calls, hybridCalls = 0, 0
	stdout.Reset()
	stderr.Reset()
	if code := runEval([]string{"run", "--collection=default"}, &stdout, &stderr); code != 0 {
		t.Fatalf("runEval BM25-only exit=%d stderr=%q", code, stderr.String())
	}
	if bm25Calls != 30 || hybridCalls != 0 {
		t.Fatalf("BM25-only should issue 30 calls and no hybrid pass, got bm25=%d hybrid=%d", bm25Calls, hybridCalls)
	}
	if strings.Contains(stdout.String(), "hybrid_recall_at_10=") || strings.Contains(stdout.String(), "reranked_recall_at_10=") {
		t.Fatalf("BM25-only output must not include hybrid/reranked lines:\n%s", stdout.String())
	}
}
