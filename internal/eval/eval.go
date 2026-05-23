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
}

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
