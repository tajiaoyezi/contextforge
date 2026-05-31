// Package eval implements the v0.1 recall-eval harness.
package eval

import (
	"bufio"
	"encoding/json"
	"fmt"
	"math"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

const (
	OutcomeStrong = "strong"
	OutcomeWeak   = "weak"
	OutcomeMiss   = "miss"
)

type LineRange struct {
	Start int64 `json:"start"`
	End   int64 `json:"end"`
}

type Question struct {
	Query             string    `json:"query"`
	ExpectedSources   []string  `json:"expected_sources,omitempty"`
	ExpectedFilePath  string    `json:"expected_file_path"`
	ExpectedLineRange LineRange `json:"expected_line_range"`
	ExpectedChunkID   string    `json:"expected_chunk_id,omitempty"`
	Category          string    `json:"category"`
	Notes             string    `json:"notes,omitempty"`
}

type Result struct {
	Question    Question
	Outcome     string
	StrongTop5  bool
	StrongTop10 bool
	Weak        bool
	Latency     time.Duration
	MatchedRank int
}

type MissCase struct {
	Query    string `json:"query"`
	Category string `json:"category"`
	Expected string `json:"expected"`
}

type Report struct {
	Total            int        `json:"total"`
	Top5StrongHits   int        `json:"top5_strong_hits"`
	Top10StrongHits  int        `json:"top10_strong_hits"`
	WeakHits         int        `json:"weak_hits"`
	Misses           int        `json:"misses"`
	Top5StrongRate   float64    `json:"top5_strong_rate"`
	Top10StrongRate  float64    `json:"top10_strong_rate"`
	LatencyP95Millis int64      `json:"latency_p95_ms"`
	MissCases        []MissCase `json:"miss_cases"`

	// task-18.8: semantic (vector-path) recall, computed alongside the BM25 path when vector-backend
	// results are supplied. SemanticEvaluated is false for a BM25-only run (no vector results) — the
	// live semantic path requires the deferred vector-retrieval integration + an embedding provider,
	// so production runs are BM25-only until then.
	SemanticEvaluated    bool    `json:"semantic_evaluated"`
	SemanticStrongHits5  int     `json:"semantic_strong_hits_top5"`
	SemanticStrongHits10 int     `json:"semantic_strong_hits_top10"`
	SemanticWeakHits     int     `json:"semantic_weak_hits"`
	SemanticMisses       int     `json:"semantic_misses"`
	SemanticRecallAt5    float64 `json:"semantic_recall_at_5"`
	SemanticRecallAt10   float64 `json:"semantic_recall_at_10"`

	// task-21.3: hybrid (RRF-fused BM25+vector, ADR-025) recall, computed alongside the BM25 path when
	// a hybrid pass is supplied (add-only, mirrors the task-18.8 SemanticRecall@K fields).
	// HybridEvaluated is false for a run without a hybrid pass — byte-equivalent to the legacy report.
	HybridEvaluated    bool    `json:"hybrid_evaluated"`
	HybridStrongHits5  int     `json:"hybrid_strong_hits_top5"`
	HybridStrongHits10 int     `json:"hybrid_strong_hits_top10"`
	HybridWeakHits     int     `json:"hybrid_weak_hits"`
	HybridMisses       int     `json:"hybrid_misses"`
	HybridRecallAt5    float64 `json:"hybrid_recall_at_5"`
	HybridRecallAt10   float64 `json:"hybrid_recall_at_10"`

	// task-21.3: reranked (top-k re-ordered by the wired Reranker — the deterministic IdentityReranker
	// default, ADR-026 D2) recall, add-only mirror of the hybrid/semantic columns. RerankedEvaluated is
	// false without a rerank pass. Real cross-encoder uplift is recorded in the dogfood spike
	// (docs/spikes/phase-21-hybrid-recall.md), not asserted here (ADR-013 — no synthetic quality).
	RerankedEvaluated    bool    `json:"reranked_evaluated"`
	RerankedStrongHits5  int     `json:"reranked_strong_hits_top5"`
	RerankedStrongHits10 int     `json:"reranked_strong_hits_top10"`
	RerankedWeakHits     int     `json:"reranked_weak_hits"`
	RerankedMisses       int     `json:"reranked_misses"`
	RerankedRecallAt5    float64 `json:"reranked_recall_at_5"`
	RerankedRecallAt10   float64 `json:"reranked_recall_at_10"`
}

// Recall gate thresholds (ADR-006 acceptance gate + Phase 18 task-18.8 amendment).
const (
	GateTop5StrongMin       = 0.75 // BM25 Top-5 strong-hit rate (ADR-006)
	GateTop10StrongMin      = 0.85 // BM25 Top-10 strong-hit rate (ADR-006)
	GateSemanticRecall10Min = 0.70 // SemanticRecall@10 (Phase 18 task-18.8 amendment)
	GateHybridRecall10Min   = 0.70 // HybridRecall@10 (Phase 21 task-21.3; ADR-006 A1 parity)
	GateRerankedRecall10Min = 0.70 // RerankedRecall@10 (Phase 21 task-21.3; ADR-006 A1 parity)
)

func BuiltinGoldenQuestions() []Question {
	cats := []struct {
		name  string
		file  string
		chunk string
		qs    []string
	}{
		{"config-location", "internal/config/config.go", "builtin-config-location", []string{
			"where is the config loader",
			"which file initializes config permissions",
			"where is default data dir resolved",
			"find the schema version config constant",
			"where are config directory modes enforced",
		}},
		{"error-reproduction", "internal/daemon/daemon.go", "builtin-error-reproduction", []string{
			"how does daemon restart after crash",
			"where is daemon health timeout handled",
			"how is loopback bind validated",
			"where does core binary lookup fail",
			"how is daemon stop made idempotent",
		}},
		{"historical-decision", "docs/decisions/adr-007-minimal-tarball-distribution.md", "builtin-historical-decision", []string{
			"why is v0.1 a minimal tarball",
			"which ADR rejects single language package distribution",
			"where is Docker compose scoped for release",
			"what is the rollback plan for release tarball",
			"which ADR covers v0.1 distribution",
		}},
		{"log-troubleshooting", "internal/memoryops/audit/audit.go", "builtin-log-troubleshooting", []string{
			"where are audit events written",
			"how is search audit metadata recorded",
			"where is export content kept out of audit",
			"which audit code records unauthorized access",
			"where is audit log append implemented",
		}},
		{"agent-memory-rule", "docs/s2v-adapter.md", "builtin-agent-memory-rule", []string{
			"where are subagent rules documented",
			"what rule prevents subagent lockfile edits",
			"where is review subagent protocol described",
			"which adapter section lists task worktrees",
			"where is ADR-012 governance autonomy referenced",
		}},
		{"code-location", "core/src/retriever/mod.rs", "builtin-code-location", []string{
			"where is BM25 search implemented",
			"which code builds explainable search results",
			"where is get chunk fast path implemented",
			"which retriever code synthesizes provenance",
			"where are search filters applied",
		}},
	}

	out := make([]Question, 0, 30)
	for _, cat := range cats {
		for i, query := range cat.qs {
			n := i + 1
			out = append(out, Question{
				Query:            query,
				ExpectedSources:  []string{cat.file},
				ExpectedFilePath: cat.file,
				ExpectedLineRange: LineRange{
					Start: 1,
					End:   120,
				},
				ExpectedChunkID: fmt.Sprintf("%s-%d", cat.chunk, n),
				Category:        cat.name,
				Notes:           "v0.1 built-in golden question",
			})
		}
	}
	return out
}

func ValidateDataset(questions []Question) error {
	if len(questions) < 30 {
		return fmt.Errorf("eval dataset has %d questions, want >=30", len(questions))
	}
	counts := map[string]int{}
	for i, q := range questions {
		if strings.TrimSpace(q.Query) == "" {
			return fmt.Errorf("question %d: query is required", i)
		}
		if strings.TrimSpace(q.ExpectedFilePath) == "" && strings.TrimSpace(q.ExpectedChunkID) == "" {
			return fmt.Errorf("question %d: expected_file_path or expected_chunk_id is required", i)
		}
		if strings.TrimSpace(q.Category) == "" {
			return fmt.Errorf("question %d: category is required", i)
		}
		counts[q.Category]++
	}
	for cat, n := range counts {
		if n < 5 {
			return fmt.Errorf("category %q has %d questions, want >=5", cat, n)
		}
	}
	if len(counts) < 6 {
		return fmt.Errorf("dataset has %d categories, want >=6", len(counts))
	}
	return nil
}

func LoadJSONL(path string) ([]Question, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()

	var out []Question
	scanner := bufio.NewScanner(f)
	for lineNo := 1; scanner.Scan(); lineNo++ {
		line := strings.TrimSpace(scanner.Text())
		if line == "" {
			continue
		}
		var q Question
		if err := json.Unmarshal([]byte(line), &q); err != nil {
			return nil, fmt.Errorf("%s:%d: %w", path, lineNo, err)
		}
		out = append(out, q)
	}
	if err := scanner.Err(); err != nil {
		return nil, err
	}
	return out, nil
}

func WriteJSONL(path string, questions []Question) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o700); err != nil {
		return err
	}
	f, err := os.OpenFile(path, os.O_CREATE|os.O_TRUNC|os.O_WRONLY, 0o600)
	if err != nil {
		return err
	}
	defer f.Close()
	enc := json.NewEncoder(f)
	for _, q := range questions {
		if err := enc.Encode(q); err != nil {
			return err
		}
	}
	return nil
}

func EvaluateQuestion(q Question, results []*contextforgev1.RetrievalResult, latency time.Duration) Result {
	out := Result{Question: q, Outcome: OutcomeMiss, Latency: latency, MatchedRank: -1}
	limit := len(results)
	if limit > 10 {
		limit = 10
	}
	for i := 0; i < limit; i++ {
		if isStrong(q, results[i]) {
			out.Outcome = OutcomeStrong
			out.StrongTop10 = true
			out.StrongTop5 = i < 5
			out.MatchedRank = i + 1
			return out
		}
	}
	for i := 0; i < limit; i++ {
		if isWeak(q, results[i]) {
			out.Outcome = OutcomeWeak
			out.Weak = true
			out.MatchedRank = i + 1
			return out
		}
	}
	return out
}

func Summarize(results []Result) Report {
	report := Report{Total: len(results)}
	latencies := make([]int64, 0, len(results))
	for _, r := range results {
		latencies = append(latencies, r.Latency.Milliseconds())
		switch r.Outcome {
		case OutcomeStrong:
			if r.StrongTop5 {
				report.Top5StrongHits++
			}
			if r.StrongTop10 {
				report.Top10StrongHits++
			}
		case OutcomeWeak:
			report.WeakHits++
		default:
			report.Misses++
			report.MissCases = append(report.MissCases, MissCase{
				Query:    r.Question.Query,
				Category: r.Question.Category,
				Expected: expectedLabel(r.Question),
			})
		}
	}
	if report.Total > 0 {
		report.Top5StrongRate = float64(report.Top5StrongHits) / float64(report.Total)
		report.Top10StrongRate = float64(report.Top10StrongHits) / float64(report.Total)
		report.LatencyP95Millis = percentile95(latencies)
	}
	return report
}

// SemanticRecallAtK returns the fraction of questions whose expected chunk was a strong hit within
// the top K of the semantic (vector) retrieval path — i.e. SemanticRecall@K = strong-hit@K rate.
// (Weak hits do not count toward recall; only an exact-chunk / overlapping-line match does.)
func SemanticRecallAtK(results []Result, k int) float64 {
	if len(results) == 0 {
		return 0
	}
	hits := 0
	for _, r := range results {
		if r.Outcome == OutcomeStrong && r.MatchedRank >= 1 && r.MatchedRank <= k {
			hits++
		}
	}
	return float64(hits) / float64(len(results))
}

// Passes bundles the optional retrieval passes evaluated alongside the BM25 baseline (task-21.3,
// add-only). Each slice holds the per-question outcomes of that retrieval method over the same
// question set; an empty/nil slice leaves that pass's columns at the zero value (it was not run).
type Passes struct {
	Semantic []Result
	Hybrid   []Result
	Reranked []Result
}

// SummarizePasses summarizes the BM25 baseline plus any supplied optional passes (semantic / hybrid /
// reranked, task-21.3). With no optional passes the report is BM25-only — byte-equivalent to the
// legacy Summarize / SummarizeHybrid(bm25, nil) output (every *Evaluated stays false). Each pass uses
// the same strong@K recall definition as the semantic path (SemanticRecallAtK).
func SummarizePasses(bm25 []Result, p Passes) Report {
	report := Summarize(bm25)
	if len(p.Semantic) > 0 {
		report.SemanticEvaluated = true
		report.SemanticStrongHits5, report.SemanticStrongHits10, report.SemanticWeakHits, report.SemanticMisses = tallyPass(p.Semantic)
		report.SemanticRecallAt5 = SemanticRecallAtK(p.Semantic, 5)
		report.SemanticRecallAt10 = SemanticRecallAtK(p.Semantic, 10)
	}
	if len(p.Hybrid) > 0 {
		report.HybridEvaluated = true
		report.HybridStrongHits5, report.HybridStrongHits10, report.HybridWeakHits, report.HybridMisses = tallyPass(p.Hybrid)
		report.HybridRecallAt5 = SemanticRecallAtK(p.Hybrid, 5)
		report.HybridRecallAt10 = SemanticRecallAtK(p.Hybrid, 10)
	}
	if len(p.Reranked) > 0 {
		report.RerankedEvaluated = true
		report.RerankedStrongHits5, report.RerankedStrongHits10, report.RerankedWeakHits, report.RerankedMisses = tallyPass(p.Reranked)
		report.RerankedRecallAt5 = SemanticRecallAtK(p.Reranked, 5)
		report.RerankedRecallAt10 = SemanticRecallAtK(p.Reranked, 10)
	}
	return report
}

// tallyPass counts strong@5 / strong@10 / weak / miss for one pass's per-question outcomes (task-21.3
// helper shared by every optional pass; matches the original SummarizeHybrid semantic tally exactly).
func tallyPass(results []Result) (strong5, strong10, weak, miss int) {
	for _, r := range results {
		switch r.Outcome {
		case OutcomeStrong:
			if r.MatchedRank >= 1 && r.MatchedRank <= 5 {
				strong5++
			}
			if r.MatchedRank >= 1 && r.MatchedRank <= 10 {
				strong10++
			}
		case OutcomeWeak:
			weak++
		default:
			miss++
		}
	}
	return
}

// SummarizeHybrid summarizes the BM25 path and, when semantic (vector-path) results are supplied, the
// semantic path alongside it. It now delegates to SummarizePasses (task-21.3, add-only) so the
// BM25/semantic output stays byte-equivalent to the task-18.8 behaviour while hybrid/reranked passes
// reuse the same machinery. With no semantic results the report is BM25-only (SemanticEvaluated=false).
func SummarizeHybrid(bm25 []Result, semantic []Result) Report {
	return SummarizePasses(bm25, Passes{Semantic: semantic})
}

// MeetsRecallGate checks a report against the ADR-006 (+ Phase 18 task-18.8) recall thresholds.
// It returns whether the gate passes and the list of failing checks. The SemanticRecall@10 check
// applies only when the report carries semantic (vector-path) results (SemanticEvaluated); a
// BM25-only report is gated on the BM25 thresholds alone, matching production until the
// vector-retrieval integration ships.
func MeetsRecallGate(report Report) (bool, []string) {
	var failures []string
	if report.Top5StrongRate < GateTop5StrongMin {
		failures = append(failures, fmt.Sprintf("Top5StrongRate %.3f < %.2f", report.Top5StrongRate, GateTop5StrongMin))
	}
	if report.Top10StrongRate < GateTop10StrongMin {
		failures = append(failures, fmt.Sprintf("Top10StrongRate %.3f < %.2f", report.Top10StrongRate, GateTop10StrongMin))
	}
	if report.SemanticEvaluated && report.SemanticRecallAt10 < GateSemanticRecall10Min {
		failures = append(failures, fmt.Sprintf("SemanticRecallAt10 %.3f < %.2f", report.SemanticRecallAt10, GateSemanticRecall10Min))
	}
	// task-21.3: hybrid / reranked recall gates apply only when those passes were evaluated (mirrors
	// the semantic gate). ADR-013: the gate is printed for human judgement, not an eval verdict.
	if report.HybridEvaluated && report.HybridRecallAt10 < GateHybridRecall10Min {
		failures = append(failures, fmt.Sprintf("HybridRecallAt10 %.3f < %.2f", report.HybridRecallAt10, GateHybridRecall10Min))
	}
	if report.RerankedEvaluated && report.RerankedRecallAt10 < GateRerankedRecall10Min {
		failures = append(failures, fmt.Sprintf("RerankedRecallAt10 %.3f < %.2f", report.RerankedRecallAt10, GateRerankedRecall10Min))
	}
	return len(failures) == 0, failures
}

func isStrong(q Question, r *contextforgev1.RetrievalResult) bool {
	if r == nil {
		return false
	}
	if q.ExpectedChunkID != "" && r.GetChunkId() == q.ExpectedChunkID {
		return true
	}
	return samePath(q.ExpectedFilePath, r.GetFilePath()) &&
		lineOverlaps(q.ExpectedLineRange, r.GetLineStart(), r.GetLineEnd())
}

func isWeak(q Question, r *contextforgev1.RetrievalResult) bool {
	if r == nil {
		return false
	}
	if samePath(q.ExpectedFilePath, r.GetFilePath()) {
		return true
	}
	for _, src := range q.ExpectedSources {
		if samePath(src, r.GetFilePath()) {
			return true
		}
	}
	return false
}

func samePath(a, b string) bool {
	return filepath.ToSlash(strings.TrimSpace(a)) == filepath.ToSlash(strings.TrimSpace(b)) &&
		strings.TrimSpace(a) != ""
}

func lineOverlaps(expected LineRange, start, end int64) bool {
	if expected.Start == 0 && expected.End == 0 {
		return true
	}
	if end == 0 {
		end = start
	}
	return start <= expected.End && end >= expected.Start
}

func expectedLabel(q Question) string {
	if q.ExpectedChunkID != "" {
		return q.ExpectedChunkID
	}
	return fmt.Sprintf("%s:%d-%d", q.ExpectedFilePath, q.ExpectedLineRange.Start, q.ExpectedLineRange.End)
}

func percentile95(values []int64) int64 {
	if len(values) == 0 {
		return 0
	}
	sort.Slice(values, func(i, j int) bool { return values[i] < values[j] })
	idx := int(math.Ceil(float64(len(values))*0.95)) - 1
	if idx < 0 {
		idx = 0
	}
	if idx >= len(values) {
		idx = len(values) - 1
	}
	return values[idx]
}
