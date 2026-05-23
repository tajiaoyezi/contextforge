package eval

import (
	"path/filepath"
	"testing"
	"time"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// TEST-8.1.1 / SCEN-8.1.1 / AC1
func TestTask81_AC1_BuiltinDatasetHasThirtyQuestionsAcrossSixCategories(t *testing.T) {
	questions := BuiltinGoldenQuestions()
	if err := ValidateDataset(questions); err != nil {
		t.Fatalf("built-in dataset should validate: %v", err)
	}
	if got, want := len(questions), 30; got != want {
		t.Fatalf("built-in dataset len=%d want %d", got, want)
	}
	counts := map[string]int{}
	for _, q := range questions {
		counts[q.Category]++
		if q.Query == "" || q.ExpectedFilePath == "" || q.Category == "" {
			t.Fatalf("question has required empty field: %+v", q)
		}
	}
	for _, category := range []string{
		"config-location",
		"error-reproduction",
		"historical-decision",
		"log-troubleshooting",
		"agent-memory-rule",
		"code-location",
	} {
		if got := counts[category]; got < 5 {
			t.Fatalf("category %q count=%d want >=5 (all counts=%v)", category, got, counts)
		}
	}
}

// TEST-8.1.2 / SCEN-8.1.2 / AC2
func TestTask81_AC2_StrongWeakMissClassification(t *testing.T) {
	q := Question{
		Query:            "where is the config loader",
		ExpectedFilePath: "internal/config/config.go",
		ExpectedLineRange: LineRange{
			Start: 10,
			End:   40,
		},
		ExpectedChunkID: "chunk-config",
		Category:        "config-location",
	}

	strongTop5 := EvaluateQuestion(q, []*contextforgev1.RetrievalResult{
		result("other", "README.md", 1, 2),
		result("chunk-config", "internal/config/config.go", 20, 30),
	}, 15*time.Millisecond)
	if strongTop5.Outcome != OutcomeStrong || !strongTop5.StrongTop5 || !strongTop5.StrongTop10 {
		t.Fatalf("strong top5 classification wrong: %+v", strongTop5)
	}

	results := make([]*contextforgev1.RetrievalResult, 0, 7)
	for i := 0; i < 6; i++ {
		results = append(results, result("miss", "docs/other.md", 1, 2))
	}
	results = append(results, result("chunk-config", "internal/config/config.go", 20, 30))
	strongTop10 := EvaluateQuestion(q, results, 20*time.Millisecond)
	if strongTop10.Outcome != OutcomeStrong || strongTop10.StrongTop5 || !strongTop10.StrongTop10 {
		t.Fatalf("strong top10 classification wrong: %+v", strongTop10)
	}

	weak := EvaluateQuestion(q, []*contextforgev1.RetrievalResult{
		result("nearby", "internal/config/config.go", 100, 120),
	}, 3*time.Millisecond)
	if weak.Outcome != OutcomeWeak || weak.StrongTop10 {
		t.Fatalf("weak classification wrong: %+v", weak)
	}

	miss := EvaluateQuestion(q, []*contextforgev1.RetrievalResult{
		result("unrelated", "internal/cli/cli.go", 1, 5),
	}, 1*time.Millisecond)
	if miss.Outcome != OutcomeMiss {
		t.Fatalf("miss classification wrong: %+v", miss)
	}
}

// TEST-8.1.3 / SCEN-8.1.3 / AC3 and TEST-8.1.4 / SCEN-8.1.4 / AC4
func TestTask81_AC3_AC4_ReportIncludesHitRatesMissesAndSuppliedLatency(t *testing.T) {
	report := Summarize([]Result{
		{Question: Question{Query: "strong"}, Outcome: OutcomeStrong, StrongTop5: true, StrongTop10: true, Latency: 10 * time.Millisecond},
		{Question: Question{Query: "weak"}, Outcome: OutcomeWeak, Latency: 30 * time.Millisecond},
		{Question: Question{Query: "miss"}, Outcome: OutcomeMiss, Latency: 20 * time.Millisecond},
	})

	if report.Total != 3 || report.Top5StrongHits != 1 || report.Top10StrongHits != 1 ||
		report.WeakHits != 1 || report.Misses != 1 {
		t.Fatalf("summary counts wrong: %+v", report)
	}
	if report.Top5StrongRate != 1.0/3.0 || report.Top10StrongRate != 1.0/3.0 {
		t.Fatalf("summary rates wrong: %+v", report)
	}
	if report.LatencyP95Millis != 30 {
		t.Fatalf("latency p95 should use supplied search durations only, got %+v", report)
	}
	if len(report.MissCases) != 1 || report.MissCases[0].Query != "miss" {
		t.Fatalf("miss cases wrong: %+v", report.MissCases)
	}
}

// TEST-8.1.5 / SCEN-8.1.5 / AC5
func TestTask81_AC5_JSONLRoundTrip(t *testing.T) {
	path := filepath.Join(t.TempDir(), "eval.jsonl")
	questions := BuiltinGoldenQuestions()
	if err := WriteJSONL(path, questions); err != nil {
		t.Fatalf("WriteJSONL: %v", err)
	}
	loaded, err := LoadJSONL(path)
	if err != nil {
		t.Fatalf("LoadJSONL: %v", err)
	}
	if err := ValidateDataset(loaded); err != nil {
		t.Fatalf("loaded dataset should validate: %v", err)
	}
	if got, want := len(loaded), len(questions); got != want {
		t.Fatalf("loaded len=%d want %d", got, want)
	}
	if loaded[0].Query != questions[0].Query || loaded[0].ExpectedFilePath != questions[0].ExpectedFilePath {
		t.Fatalf("first question round trip mismatch: got %+v want %+v", loaded[0], questions[0])
	}
}

func result(chunkID, filePath string, lineStart, lineEnd int64) *contextforgev1.RetrievalResult {
	return &contextforgev1.RetrievalResult{
		ChunkId:   chunkID,
		FilePath:  filePath,
		LineStart: lineStart,
		LineEnd:   lineEnd,
	}
}
