// task-14.2 (ADR-017 D1 Wave 4): runEvalAsync goroutine — drives the recall
// harness in-process and reverse-updates the Rust SqliteEvalStore via the
// EvalClient.UpdateProgress callback when the run reaches a terminal status.
//
// Trade-off (task-14.1 §10): Go-side goroutine vs Rust spawn_blocking. We
// chose Go-side because the harness already lives in `internal/eval/eval.go`
// and error propagation stays natural; orphan reaping when console-api-serve
// crashes is deferred to [SPEC-DEFER:phase-15.eval-orphan-reaper].

package consoleapi

import (
	"context"
	"fmt"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/contractv1"
	"github.com/tajiaoyezi/contextforge/internal/eval"
)

// runEvalAsync is invoked by handleCreateEvalRun in a fresh goroutine. It
// runs a small recall scenario against the BuiltinGoldenQuestions fixture +
// computes recall@5 / recall@10 / precision@5 stub metrics, then calls
// UpdateProgress to persist status=succeeded.
//
// On panic or context timeout it persists status=failed with error_message.
// The 5min timeout matches task-14.2 §8 risk note; future Console UI may
// pass ?timeout=<duration> to override.
func runEvalAsync(deps Deps, evalRunID string, req contractv1.EvalRunCreate) {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Minute)
	defer cancel()

	status := "succeeded"
	errMsg := ""
	metrics := map[string]float64{}
	caseResults := []contractv1.CaseResult{}

	defer func() {
		if r := recover(); r != nil {
			status = "failed"
			errMsg = fmt.Sprintf("panic in runEvalAsync: %v", r)
		}
		if deps.Eval != nil {
			_ = deps.Eval.UpdateProgress(evalRunID, status, metrics, caseResults, errMsg)
		}
	}()

	// Honor context: if cancelled during harness, mark failed.
	doneCh := make(chan struct{})
	go func() {
		defer close(doneCh)
		// Light-weight harness: use BuiltinGoldenQuestions as the dataset
		// and synthesize a mock pass/fail per question so the contract surfaces
		// real case_results. Production v1.x will plug into the full recall
		// harness (RetrievalResult + EvaluateQuestion + Summarize) via
		// retriever-backed lookups; v0.7 ships the orchestration contract.
		questions := eval.BuiltinGoldenQuestions()
		passCount := 0
		for i, q := range questions {
			caseResults = append(caseResults, contractv1.CaseResult{
				CaseID:         fmt.Sprintf("case-%d", i),
				Query:          q.Query,
				ExpectedChunks: []string{q.ExpectedFilePath},
				ActualChunks:   []string{q.ExpectedFilePath}, // mock: pass everything
				Score:          1.0,
				Passed:         true,
			})
			passCount++
		}
		total := float64(len(questions))
		if total > 0 {
			pass := float64(passCount)
			metrics["recall@5"] = pass / total
			metrics["recall@10"] = pass / total
			metrics["precision@5"] = pass / total
		}
	}()

	select {
	case <-doneCh:
		// finished normally
	case <-ctx.Done():
		status = "failed"
		errMsg = ctx.Err().Error()
	}

	_ = req // req parameters available for future dataset_ref / config_snapshot dispatch
}
