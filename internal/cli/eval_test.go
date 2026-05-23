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
