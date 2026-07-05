package eval

import (
	"os"
	"path/filepath"
	"reflect"
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

// TEST-18.8.1 / AC1: SemanticRecall@K counts strong hits within the top K only.
func TestTask188_AC1_SemanticRecallAtK(t *testing.T) {
	results := []Result{
		{Outcome: OutcomeStrong, MatchedRank: 1}, // strong @1  → counts for @5 and @10
		{Outcome: OutcomeStrong, MatchedRank: 5}, // strong @5  → counts for @5 and @10
		{Outcome: OutcomeStrong, MatchedRank: 8}, // strong @8  → counts for @10 only
		{Outcome: OutcomeWeak, MatchedRank: 2},   // weak       → never counts toward recall
		{Outcome: OutcomeMiss, MatchedRank: -1},  // miss
	}
	if got, want := SemanticRecallAtK(results, 5), 2.0/5.0; got != want {
		t.Fatalf("SemanticRecall@5 = %v, want %v", got, want)
	}
	if got, want := SemanticRecallAtK(results, 10), 3.0/5.0; got != want {
		t.Fatalf("SemanticRecall@10 = %v, want %v", got, want)
	}
	if got := SemanticRecallAtK(nil, 10); got != 0 {
		t.Fatalf("SemanticRecall@10 of empty = %v, want 0", got)
	}
}

// TEST-18.8.2 / AC2: SummarizeHybrid fills BOTH the BM25 and the semantic (vector-path) fields.
func TestTask188_AC2_SummarizeHybridBothPaths(t *testing.T) {
	bm25 := []Result{
		{Outcome: OutcomeStrong, StrongTop5: true, StrongTop10: true, MatchedRank: 1, Latency: 5 * time.Millisecond},
		{Outcome: OutcomeWeak, MatchedRank: 3, Latency: 7 * time.Millisecond},
	}
	semantic := []Result{
		{Outcome: OutcomeStrong, MatchedRank: 2},
		{Outcome: OutcomeStrong, MatchedRank: 9},
	}
	report := SummarizeHybrid(bm25, semantic)
	if report.Total != 2 || report.Top5StrongHits != 1 || report.WeakHits != 1 {
		t.Fatalf("bm25 fields wrong: %+v", report)
	}
	if !report.SemanticEvaluated {
		t.Fatalf("SemanticEvaluated should be true when semantic results supplied")
	}
	if report.SemanticStrongHits5 != 1 || report.SemanticStrongHits10 != 2 {
		t.Fatalf("semantic strong hits wrong: top5=%d top10=%d", report.SemanticStrongHits5, report.SemanticStrongHits10)
	}
	if report.SemanticRecallAt5 != 1.0/2.0 || report.SemanticRecallAt10 != 2.0/2.0 {
		t.Fatalf("semantic recall wrong: @5=%v @10=%v", report.SemanticRecallAt5, report.SemanticRecallAt10)
	}
}

// TEST-18.8.3 / AC3: with no vector results SummarizeHybrid is BM25-only (SemanticEvaluated false),
// and the gate does not require the semantic threshold — the production fallback until a vector
// backend + embedding provider are wired in.
func TestTask188_AC3_EmptySemanticBM25Only(t *testing.T) {
	bm25 := []Result{{Outcome: OutcomeStrong, StrongTop5: true, StrongTop10: true, MatchedRank: 1}}
	report := SummarizeHybrid(bm25, nil)
	if report.SemanticEvaluated {
		t.Fatalf("SemanticEvaluated should be false with no semantic results")
	}
	if report.SemanticRecallAt10 != 0 {
		t.Fatalf("semantic recall should be 0 in BM25-only, got %v", report.SemanticRecallAt10)
	}
	pass, failures := MeetsRecallGate(Report{Top5StrongRate: 0.8, Top10StrongRate: 0.9, SemanticEvaluated: false})
	if !pass {
		t.Fatalf("BM25-only gate should pass at 0.8/0.9: %v", failures)
	}
}

// TEST-18.8.4 / AC4: the recall gate enforces the BM25 thresholds always and SemanticRecall@10 only
// when the semantic path was evaluated.
func TestTask188_AC4_RecallGate(t *testing.T) {
	if pass, failures := MeetsRecallGate(Report{Top5StrongRate: 0.8, Top10StrongRate: 0.9, SemanticEvaluated: true, SemanticRecallAt10: 0.75}); !pass {
		t.Fatalf("gate should pass at bm25 0.8/0.9 + semantic 0.75: %v", failures)
	}
	if pass, failures := MeetsRecallGate(Report{Top5StrongRate: 0.8, Top10StrongRate: 0.9, SemanticEvaluated: true, SemanticRecallAt10: 0.5}); pass || len(failures) != 1 {
		t.Fatalf("gate should fail only on semantic recall: pass=%v failures=%v", pass, failures)
	}
	if pass, _ := MeetsRecallGate(Report{Top5StrongRate: 0.5, Top10StrongRate: 0.6, SemanticEvaluated: false}); pass {
		t.Fatalf("gate should fail on bm25 thresholds below 0.75/0.85")
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

// TEST-21.3.1 / AC1: SummarizePasses fills the add-only hybrid + reranked recall columns alongside
// the BM25 + semantic columns (task-21.3, mirroring the task-18.8 SemanticRecall@K add-only pattern).
func TestTask213_AC1_SummarizePassesHybridRerankedColumns(t *testing.T) {
	bm25 := []Result{
		{Outcome: OutcomeStrong, StrongTop5: true, StrongTop10: true, MatchedRank: 1, Latency: 5 * time.Millisecond},
		{Outcome: OutcomeWeak, MatchedRank: 3, Latency: 7 * time.Millisecond},
	}
	hybrid := []Result{
		{Outcome: OutcomeStrong, MatchedRank: 2},
		{Outcome: OutcomeStrong, MatchedRank: 7},
	}
	reranked := []Result{
		{Outcome: OutcomeStrong, MatchedRank: 1},
		{Outcome: OutcomeMiss, MatchedRank: -1},
	}
	report := SummarizePasses(bm25, Passes{Hybrid: hybrid, Reranked: reranked})

	if !report.HybridEvaluated {
		t.Fatal("HybridEvaluated should be true when hybrid results supplied")
	}
	if report.HybridStrongHits5 != 1 || report.HybridStrongHits10 != 2 {
		t.Fatalf("hybrid strong hits wrong: top5=%d top10=%d", report.HybridStrongHits5, report.HybridStrongHits10)
	}
	if report.HybridRecallAt5 != 1.0/2.0 || report.HybridRecallAt10 != 2.0/2.0 {
		t.Fatalf("hybrid recall wrong: @5=%v @10=%v", report.HybridRecallAt5, report.HybridRecallAt10)
	}
	if !report.RerankedEvaluated {
		t.Fatal("RerankedEvaluated should be true when reranked results supplied")
	}
	if report.RerankedStrongHits5 != 1 || report.RerankedRecallAt10 != 1.0/2.0 {
		t.Fatalf("reranked columns wrong: strong5=%d recall@10=%v", report.RerankedStrongHits5, report.RerankedRecallAt10)
	}
	// BM25 columns still correct; semantic stays unevaluated (no semantic pass supplied).
	if report.Total != 2 || report.Top5StrongHits != 1 || report.WeakHits != 1 {
		t.Fatalf("bm25 columns wrong: %+v", report)
	}
	if report.SemanticEvaluated {
		t.Fatal("SemanticEvaluated should be false when no semantic pass supplied")
	}
}

// TEST-21.3.1 / AC1 (byte-equivalent): SummarizeHybrid delegates to SummarizePasses and stays
// byte-equivalent to the legacy BM25/semantic-only output — no hybrid/reranked columns set (add-only).
func TestTask213_AC1_SummarizeHybridByteEquivalent(t *testing.T) {
	bm25 := []Result{{Outcome: OutcomeStrong, StrongTop5: true, StrongTop10: true, MatchedRank: 1}}
	semantic := []Result{{Outcome: OutcomeStrong, MatchedRank: 2}}

	legacy := SummarizeHybrid(bm25, semantic)
	viaPasses := SummarizePasses(bm25, Passes{Semantic: semantic})
	if !reflect.DeepEqual(legacy, viaPasses) {
		t.Fatalf("SummarizeHybrid must equal SummarizePasses{Semantic}:\n legacy=%+v\n passes=%+v", legacy, viaPasses)
	}
	// No hybrid/reranked pass → those columns stay zero/false (add-only, byte-equivalent).
	if legacy.HybridEvaluated || legacy.RerankedEvaluated {
		t.Fatalf("hybrid/reranked must be unset for a semantic-only run: %+v", legacy)
	}
	if legacy.HybridRecallAt10 != 0 || legacy.RerankedRecallAt10 != 0 {
		t.Fatalf("hybrid/reranked recall must be zero when not evaluated: %+v", legacy)
	}
}

// TEST-21.3.1 / AC1: the recall gate adds HybridRecall@10 / RerankedRecall@10 checks ONLY when those
// passes were evaluated (mirrors the SemanticRecall@10 gate; BM25/semantic-only reports unaffected).
func TestTask213_AC1_RecallGateHybridReranked(t *testing.T) {
	// hybrid + reranked evaluated, both below 0.70 → two extra failures (BM25 thresholds ok).
	pass, failures := MeetsRecallGate(Report{
		Top5StrongRate: 0.8, Top10StrongRate: 0.9,
		HybridEvaluated: true, HybridRecallAt10: 0.5,
		RerankedEvaluated: true, RerankedRecallAt10: 0.4,
	})
	if pass || len(failures) != 2 {
		t.Fatalf("gate should fail on hybrid+reranked recall: pass=%v failures=%v", pass, failures)
	}
	// not evaluated → no extra checks (byte-equivalent to the BM25-only gate).
	if ok, _ := MeetsRecallGate(Report{Top5StrongRate: 0.8, Top10StrongRate: 0.9}); !ok {
		t.Fatal("gate should pass when hybrid/reranked not evaluated")
	}
}

// ---- task-24.2: golden dataset 校验器 + 代码/CJK golden 扩充 ----

func validBaseQuestions() []Question {
	return []Question{
		{Query: "build_tantivy_schema", ExpectedFilePath: "core/src/indexer/mod.rs", Category: "code-symbol"},
		{Query: "RetrieverConfig", ExpectedFilePath: "core/src/retriever/mod.rs", Category: "code-symbol"},
		{Query: "单驱动", ExpectedFilePath: "AGENTS.md", Category: "cjk"},
	}
}

// TEST-24.2.1 / AC1: 校验器 schema 良构 + 覆盖（良构过；不良 schema + 悬空 expected 被拒）。
func TestTask242_AC1_ValidatorSchemaAndCoverage(t *testing.T) {
	if err := ValidateGoldenSemantic(BuiltinGoldenQuestions()); err != nil {
		t.Fatalf("builtin should pass ValidateGoldenSemantic: %v", err)
	}
	if err := ValidateGoldenSemantic(validBaseQuestions()); err != nil {
		t.Fatalf("valid base should pass: %v", err)
	}
	// 未知 category 被拒
	bad := validBaseQuestions()
	bad[0].Category = "nonexistent-category"
	if err := ValidateGoldenSemantic(bad); err == nil {
		t.Fatal("unknown category should be rejected")
	}
	// line_range start>end 被拒
	bad = validBaseQuestions()
	bad[0].ExpectedLineRange = LineRange{Start: 40, End: 10}
	if err := ValidateGoldenSemantic(bad); err == nil {
		t.Fatal("line_range start>end should be rejected")
	}
	// 悬空 expected（file 与 chunk 皆空）被拒
	bad = validBaseQuestions()
	bad[0].ExpectedFilePath = ""
	bad[0].ExpectedChunkID = ""
	if err := ValidateGoldenSemantic(bad); err == nil {
		t.Fatal("dangling expected (no file / no chunk) should be rejected")
	}
	// 空 query 被拒
	bad = validBaseQuestions()
	bad[0].Query = "   "
	if err := ValidateGoldenSemantic(bad); err == nil {
		t.Fatal("empty query should be rejected")
	}
}

// TEST-24.2.2 / AC2: 重复检测（同 query / 同 (query,expected) 对被拒）+ 既有 ValidateDataset/roundtrip 不退化。
func TestTask242_AC2_DuplicateDetectionAndNoRegression(t *testing.T) {
	// 同 query 文本重复被拒
	dupQuery := append(validBaseQuestions(), Question{
		Query: "build_tantivy_schema", ExpectedFilePath: "core/src/parser/mod.rs", Category: "code-symbol",
	})
	if err := ValidateGoldenSemantic(dupQuery); err == nil {
		t.Fatal("duplicate query text should be rejected")
	}
	// 同 (query, expected) 对重复被拒（chunk 维度）
	dupPair := []Question{
		{Query: "x", ExpectedChunkID: "chunk-a", Category: "code-symbol"},
		{Query: "x", ExpectedChunkID: "chunk-a", Category: "code-symbol"},
	}
	if err := ValidateGoldenSemantic(dupPair); err == nil {
		t.Fatal("duplicate (query, expected) pair should be rejected")
	}
	// 既有 ValidateDataset + 30 题 builtin 不退化
	if err := ValidateDataset(BuiltinGoldenQuestions()); err != nil {
		t.Fatalf("ValidateDataset(builtin) regressed: %v", err)
	}
	// JSONL roundtrip 不退化
	path := filepath.Join(t.TempDir(), "rt.jsonl")
	if err := WriteJSONL(path, BuiltinGoldenQuestions()); err != nil {
		t.Fatalf("WriteJSONL: %v", err)
	}
	loaded, err := LoadJSONL(path)
	if err != nil || len(loaded) != 30 {
		t.Fatalf("roundtrip regressed: err=%v len=%d", err, len(loaded))
	}
}

// TEST-24.2.3 / AC3: golden-semantic.jsonl 含代码符号 + CJK case，路径真实，过校验器。
func TestTask242_AC3_GoldenSemanticDatasetCodeAndCJK(t *testing.T) {
	const fixture = "../../test/fixtures/eval/golden-semantic.jsonl"
	qs, err := LoadJSONL(fixture)
	if err != nil {
		t.Fatalf("LoadJSONL(%s): %v", fixture, err)
	}
	if len(qs) == 0 {
		t.Fatal("golden-semantic.jsonl is empty")
	}
	if err := ValidateGoldenSemantic(qs); err != nil {
		t.Fatalf("golden-semantic should pass ValidateGoldenSemantic: %v", err)
	}
	cats := map[string]int{}
	for _, q := range qs {
		cats[q.Category]++
	}
	if cats["code-symbol"] == 0 {
		t.Fatal("golden-semantic must contain code-symbol query case")
	}
	if cats["cjk"] == 0 {
		t.Fatal("golden-semantic must contain cjk query case")
	}
	// query/expected 指向真实源码（路径经核实存在，ADR-013 grounded）
	for _, q := range qs {
		if q.ExpectedFilePath == "" {
			continue
		}
		p := filepath.Join("../..", q.ExpectedFilePath)
		if _, err := os.Stat(p); err != nil {
			t.Fatalf("expected_file_path %q does not exist (query %q): %v", q.ExpectedFilePath, q.Query, err)
		}
	}
}

// TEST-24.2.4 / AC4: ADR-006 gate 阈值不变（本 task 加固标尺不改阈值）。
func TestTask242_AC4_GateThresholdsUnchanged(t *testing.T) {
	if GateTop5StrongMin != 0.75 || GateTop10StrongMin != 0.85 || GateSemanticRecall10Min != 0.70 {
		t.Fatalf("gate thresholds changed: %v / %v / %v", GateTop5StrongMin, GateTop10StrongMin, GateSemanticRecall10Min)
	}
}

// TEST-49.1.1 / AC1: golden-retrieval.jsonl ~120 题 / 6 categories / 每类 ≥5 → 过 ValidateDataset
func TestTask491_AC1_GoldenRetrievalPassesValidateDataset(t *testing.T) {
	const fixture = "../../test/fixtures/eval/golden-retrieval.jsonl"
	qs, err := LoadJSONL(fixture)
	if err != nil {
		t.Fatalf("LoadJSONL(%s): %v", fixture, err)
	}
	if len(qs) < 120 {
		t.Fatalf("golden-retrieval.jsonl len=%d want >=120", len(qs))
	}
	if err := ValidateDataset(qs); err != nil {
		t.Fatalf("golden-retrieval should pass ValidateDataset: %v", err)
	}
}

// TEST-49.1.2 / AC2: 所有 category ∈ knownCategories（6 builtin）
func TestTask491_AC2_CategoriesInKnownSet(t *testing.T) {
	const fixture = "../../test/fixtures/eval/golden-retrieval.jsonl"
	qs, err := LoadJSONL(fixture)
	if err != nil {
		t.Fatalf("LoadJSONL(%s): %v", fixture, err)
	}
	wantCats := map[string]bool{
		"config-location":     true,
		"error-reproduction":  true,
		"historical-decision": true,
		"log-troubleshooting": true,
		"agent-memory-rule":   true,
		"code-location":       true,
	}
	seen := map[string]int{}
	for _, q := range qs {
		if !wantCats[q.Category] {
			t.Fatalf("category %q not in 6 builtin known set (query %q)", q.Category, q.Query)
		}
		seen[q.Category]++
	}
	if len(seen) != 6 {
		t.Fatalf("want 6 distinct categories, got %d: %v", len(seen), seen)
	}
	for cat, n := range seen {
		if n < 5 {
			t.Fatalf("category %q has %d questions, want >=5", cat, n)
		}
	}
}

// TEST-49.1.3 / AC3: 无 duplicate query + expected_file_path 真实存在
func TestTask491_AC3_NoDupQueriesAndFilesExist(t *testing.T) {
	const fixture = "../../test/fixtures/eval/golden-retrieval.jsonl"
	qs, err := LoadJSONL(fixture)
	if err != nil {
		t.Fatalf("LoadJSONL(%s): %v", fixture, err)
	}
	seen := map[string]int{}
	for i, q := range qs {
		if q.Query == "" {
			t.Fatalf("question %d has empty query", i)
		}
		if q.ExpectedFilePath == "" {
			t.Fatalf("question %d (%q) has empty expected_file_path", i, q.Query)
		}
		if prev, ok := seen[q.Query]; ok {
			t.Fatalf("duplicate query %q (questions %d and %d)", q.Query, prev, i)
		}
		seen[q.Query] = i
		// expected_file_path must be a real file in the repo (grounded, ADR-013)
		p := filepath.Join("../..", q.ExpectedFilePath)
		if _, err := os.Stat(p); err != nil {
			t.Fatalf("expected_file_path %q does not exist (query %q): %v", q.ExpectedFilePath, q.Query, err)
		}
	}
}
