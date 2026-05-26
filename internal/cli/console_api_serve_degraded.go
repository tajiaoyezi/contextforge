// task-11.2 (ADR-016 §D4): degraded Deps stand-in when the contextforge-core
// data plane is unreachable and CONSOLE_API_FALLBACK_INMEM is not set.
// Every business RPC returns consoleapi.ErrDataPlaneUnavailable; the router
// maps this sentinel to HTTP 503 + degraded:true + missing:["data_plane"].
//
// Keep these wrappers minimal (no business logic) — they exist solely to
// preserve the Deps interface boundary so the router still serves /v1/health
// (which can report "degraded" from outside the per-RPC error path).

package cli

import (
	"github.com/tajiaoyezi/contextforge/internal/consoleapi"
	"github.com/tajiaoyezi/contextforge/internal/contractv1"
)

type degradedWorkspace struct{}

func (degradedWorkspace) Create(_ contractv1.WorkspaceCreate) (contractv1.Workspace, error) {
	return contractv1.Workspace{}, consoleapi.ErrDataPlaneUnavailable
}
func (degradedWorkspace) List() ([]contractv1.Workspace, error) {
	return nil, consoleapi.ErrDataPlaneUnavailable
}
func (degradedWorkspace) Get(_ string) (*contractv1.Workspace, error) {
	return nil, consoleapi.ErrDataPlaneUnavailable
}
func (degradedWorkspace) Update(_ string, _, _ []string) (contractv1.Workspace, error) {
	return contractv1.Workspace{}, consoleapi.ErrDataPlaneUnavailable
}

type degradedJob struct{}

func (degradedJob) Enqueue(_, _ string) (contractv1.IndexJob, error) {
	return contractv1.IndexJob{}, consoleapi.ErrDataPlaneUnavailable
}
func (degradedJob) Get(_ string) (*contractv1.IndexJob, error) {
	return nil, consoleapi.ErrDataPlaneUnavailable
}
func (degradedJob) Cancel(_ string) error {
	return consoleapi.ErrDataPlaneUnavailable
}
func (degradedJob) ListActive() ([]contractv1.IndexJob, error) {
	return nil, consoleapi.ErrDataPlaneUnavailable
}

type degradedSearch struct{}

func (degradedSearch) Search(_ contractv1.SearchRequest) (contractv1.SearchResult, contractv1.RetrievalTrace, error) {
	return contractv1.SearchResult{}, contractv1.RetrievalTrace{}, consoleapi.ErrDataPlaneUnavailable
}
func (degradedSearch) GetSourceChunk(_ string) (contractv1.SourceChunk, error) {
	return contractv1.SourceChunk{}, consoleapi.ErrDataPlaneUnavailable
}
func (degradedSearch) GetSearchTrace(_ string) (contractv1.RetrievalTrace, error) {
	return contractv1.RetrievalTrace{}, consoleapi.ErrDataPlaneUnavailable
}
func (degradedSearch) GetChunksStats(_ string) (contractv1.ChunksStats, error) {
	return contractv1.ChunksStats{}, consoleapi.ErrDataPlaneUnavailable
}
func (degradedSearch) ListQueries(_ int) ([]contractv1.QueryRecord, error) {
	return nil, consoleapi.ErrDataPlaneUnavailable
}

// task-15.4: degraded EvalClient also implements List → 503.

type degradedEvents struct{}

func (degradedEvents) Recent(_ int) ([]contractv1.ObservabilityEvent, error) {
	return nil, consoleapi.ErrDataPlaneUnavailable
}

type degradedMemory struct{}

func (degradedMemory) List(_ consoleapi.MemoryListFilter) ([]contractv1.MemoryItem, error) {
	return nil, consoleapi.ErrDataPlaneUnavailable
}
func (degradedMemory) Get(_ string) (*contractv1.MemoryItem, error) {
	return nil, consoleapi.ErrDataPlaneUnavailable
}
func (degradedMemory) Pin(_ string, _ bool) error    { return consoleapi.ErrDataPlaneUnavailable }
func (degradedMemory) Deprecate(_ string) error      { return consoleapi.ErrDataPlaneUnavailable }
func (degradedMemory) SoftDelete(_ string) error     { return consoleapi.ErrDataPlaneUnavailable }

type degradedEval struct{}

func (degradedEval) Create(_ contractv1.EvalRunCreate) (contractv1.EvalRun, error) {
	return contractv1.EvalRun{}, consoleapi.ErrDataPlaneUnavailable
}
func (degradedEval) Get(_ string) (*contractv1.EvalRun, error) {
	return nil, consoleapi.ErrDataPlaneUnavailable
}
func (degradedEval) UpdateProgress(_, _ string, _ map[string]float64,
	_ []contractv1.CaseResult, _ string) error {
	return consoleapi.ErrDataPlaneUnavailable
}
func (degradedEval) List(_ contractv1.ListEvalRunsFilter) ([]contractv1.EvalRun, error) {
	return nil, consoleapi.ErrDataPlaneUnavailable
}
