// Package consoleapi serves ContextForge's Console Contract v1 REST surface
// (ADR-015 §D4). 9 endpoints under /v1/* aligned to Console HTTPAdapter
// expectations (see Console console-api/internal/coreadapter/http_adapter.go
// + testhelper/fakehttpserver.go for the single source of truth on URL paths,
// request/response shapes and error codes).
//
// Storage trade-off (task-10.4 §10 #1): v0.3 uses Go-side in-memory stores
// (no shared SQLite between Go REST handlers and Rust workspace/jobs stores
// from task-10.2/10.3). Cross-process Rust↔Go SQLite sharing is deferred to
// task-future.cross-process-sqlite-sharing.
//
// Refs: ADR-015 §D4 / phase-10 §6 AC4 / task-10.4 §6 AC1-5
package consoleapi

import (
	"context"
	"errors"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/contractv1"
)

// Sentinel errors used by handlers and clients for the Console HTTPAdapter
// error-mapping convention (404 → ErrNotFound / 409 → ErrConflict / 503 →
// ErrDataPlaneUnavailable).
//
// task-11.2 (ADR-016 §D3 + §D4): ErrDataPlaneUnavailable lights up the
// degraded mode UI (gRPC unreachable + CONSOLE_API_FALLBACK_INMEM unset).
var (
	ErrNotFound             = errors.New("not found")
	ErrJobTerminal          = errors.New("job already terminal")
	ErrInvalidRequest       = errors.New("invalid request")
	ErrDataPlaneUnavailable = errors.New("data plane unavailable")
	// task-12.1 (ADR-017 D2): X-Confirm: yes header OR ?confirm=true query
	// required for destructive endpoints (PATCH workspace/config + memory
	// deprecate/soft-delete in phase-13). Server-side bottom defense: if
	// Console BFF forgets to inject, ops curl gets 412 not silent succeed.
	ErrPreconditionRequired = errors.New("X-Confirm: yes header or ?confirm=true query required")
)

// WorkspaceClient backs the /v1/workspaces[*] handlers.
type WorkspaceClient interface {
	Create(req contractv1.WorkspaceCreate) (contractv1.Workspace, error)
	List() ([]contractv1.Workspace, error)
	Get(id string) (*contractv1.Workspace, error) // nil if not found
	// task-12.1: Update overwrites allowlist + denylist, bumps updated_at.
	// Returns ErrNotFound when workspace missing.
	Update(workspaceID string, allowlist, denylist []string) (contractv1.Workspace, error)
}

// JobClient backs the /v1/index-jobs[*] handlers.
type JobClient interface {
	Enqueue(workspaceID, triggerSource string) (contractv1.IndexJob, error)
	Get(jobID string) (*contractv1.IndexJob, error) // nil if not found
	Cancel(jobID string) error                       // returns ErrJobTerminal if already terminal; ErrNotFound if missing
	// task-12.1: ListActive returns queued + running jobs (v1.0 scope).
	// Non-active filters (succeeded/failed/cancelled) [SPEC-DEFER:console-list-all-jobs] 留 v1.x.
	ListActive() ([]contractv1.IndexJob, error)
}

// SearchClient backs POST /v1/search + GET /v1/source-chunks/{id} (task-12.2)
// + GET /v1/search/{query_id}/trace (task-12.3) + GET /v1/stats/chunks
// (task-15.3 Phase 15 P1 #3).
type SearchClient interface {
	Search(req contractv1.SearchRequest) (contractv1.SearchResult, contractv1.RetrievalTrace, error)
	GetSourceChunk(chunkID string) (contractv1.SourceChunk, error)
	// task-12.3 (ADR-017 D1 Wave 2): trace-by-query_id lookup. Returns
	// ErrNotFound when the query has not been executed (or was evicted from
	// the in-memory LRU; daemon restart wipes the cache).
	GetSearchTrace(queryID string) (contractv1.RetrievalTrace, error)
	// task-15.3 (Phase 15 P1 #3): chunks stats for Dashboard "已索引块"
	// indicator. workspaceID empty = cross-workspace aggregate.
	GetChunksStats(workspaceID string) (contractv1.ChunksStats, error)
	// task-15.5 (Phase 15 P1 #5): query history for Dashboard "最近查询"
	// panel. limit ≤ 0 → server default 20; max 100.
	ListQueries(limit int) ([]contractv1.QueryRecord, error)
}

// EventsClient backs GET /v1/observability/events.
//
// task-16.2 (Phase 16 P4 #11): Recent takes a `wait` duration so the handler
// can drive real long-poll semantics. Implementations:
//   - grpcclient: phase-1 blocks up to `wait` on the broadcast stream; phase-2
//     drains immediately-available events with a short (~100ms) timeout.
//   - MemStore (fallback): sleeps min(wait, 1s) on empty ring buffer to avoid
//     UI poll-storm; non-empty buffer returns immediately.
type EventsClient interface {
	Recent(limit int, wait time.Duration) ([]contractv1.ObservabilityEvent, error)
}

// StreamOptions carries the SSE replay parameters (task-26.2 / ADR-031 D3/D4).
// SinceTS > 0 requests replay of memory state-op events from the persistent
// audit log at/after the cutoff (unix seconds) before splicing the live stream;
// LastEventID is the client's last-seen SSE id for boundary dedup (advisory).
type StreamOptions struct {
	SinceTS     int64
	LastEventID string
}

// EventsStreamer backs GET /v1/observability/events/stream (task-26.2 / ADR-031
// D3): an add-only Server-Sent-Events surface alongside the existing long-poll
// EventsClient. Stream returns a channel of events (audit replay first when
// SinceTS > 0, then the live broadcast). The channel is closed — and the
// underlying gRPC subscription released — when ctx is cancelled (client
// disconnect) or the upstream stream ends.
type EventsStreamer interface {
	Stream(ctx context.Context, opts StreamOptions) (<-chan contractv1.ObservabilityEvent, error)
}

// MemoryListFilter — task-13.2 (ADR-017 D1 Wave 3) filter struct mirroring
// the Rust ListMemoryRequest 4 fields. AgentID prefix matches agent_scope;
// Namespace suffix matches agent_scope; Scope exact-matches agent_scope.
type MemoryListFilter struct {
	AgentID            string
	Scope              string
	Namespace          string
	IncludeSoftDeleted bool
}

// MemoryClient backs the 5 /v1/memory[*] handlers (task-13.2 / ADR-017 D1 Wave 3).
type MemoryClient interface {
	List(filter MemoryListFilter) ([]contractv1.MemoryItem, error)
	Get(memoryID string) (*contractv1.MemoryItem, error) // nil if not found
	Pin(memoryID string, pin bool) error                  // pin=false = unpin
	Deprecate(memoryID string) error
	SoftDelete(memoryID string) error
}

// EvalClient backs POST /v1/eval-runs + GET /v1/eval-runs/{id} (task-14.2 / ADR-017 D1 Wave 4)
// + GET /v1/eval-runs (task-15.4 / Phase 15 P1 #4 list).
// UpdateProgress is the runEvalAsync goroutine callback — not exposed in
// Console contract v1 22-endpoint surface.
type EvalClient interface {
	Create(req contractv1.EvalRunCreate) (contractv1.EvalRun, error)
	Get(evalRunID string) (*contractv1.EvalRun, error) // nil if not found
	UpdateProgress(evalRunID, status string, metrics map[string]float64,
		caseResults []contractv1.CaseResult, errorMessage string) error
	// task-15.4: list eval runs filtered + ORDER BY started_at DESC.
	List(filter contractv1.ListEvalRunsFilter) ([]contractv1.EvalRun, error)
}

// HealthClient backs GET /v1/health?detailed=true (task-15.6 / Phase 15 P2 #7
// / ADR-020). The basic binary /v1/health endpoint stays in handler-side
// switch (BackendKind); only the detailed opt-in goes through this interface.
type HealthClient interface {
	GetDetailed() (contractv1.CoreHealth, error)
}

// Deps bundles all four backends + the bearer auth token for NewRouter.
// AuthToken == "" means "trusted-network" (no Authorization header required —
// aligns with Console CONSOLE_API_CORE_AUTH_MODE=trusted-network default).
//
// task-11.2 (ADR-016 §D4): BackendKind tags how /v1/health reports degraded
// state — "grpc" (default, healthy), "inmem-fallback" (degraded, MemStore
// fallback active), or "degraded" (data plane unreachable, 503).
type Deps struct {
	Workspace   WorkspaceClient
	Job         JobClient
	Search      SearchClient
	Events      EventsClient
	// task-26.2 (ADR-031 D3): optional SSE streaming surface. May be nil —
	// handleEventsStream returns 503 when absent (preserves the long-poll-only
	// contract for backends without a streaming entry).
	EventsStream EventsStreamer
	Memory       MemoryClient
	Eval         EvalClient
	// task-15.6 (Phase 15 P2 #7): optional HealthClient for ?detailed=true.
	// May be nil — handleHealth falls back to a synthetic 5-component
	// response if so (preserves v0.7 contract).
	Health      HealthClient
	AuthToken   string
	BackendKind string
}

// SearchResponse is the Console HTTPAdapter-expected nested JSON envelope
// for POST /v1/search (see Console http_adapter.go).
type SearchResponse struct {
	Result contractv1.SearchResult   `json:"result"`
	Trace  contractv1.RetrievalTrace `json:"trace"`
}

// ErrorBody is the JSON shape we emit for any non-2xx response. Console's
// HTTPAdapter maps 404 → ErrNotFound / 409 → ErrConflict / 5xx →
// ErrCoreUnavailable; we keep the body shape simple + machine-readable.
type ErrorBody struct {
	Error struct {
		Code    string `json:"code"`
		Message string `json:"message"`
	} `json:"error"`
}
