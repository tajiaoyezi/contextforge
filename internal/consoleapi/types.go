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
	"errors"

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
)

// WorkspaceClient backs the /v1/workspaces[*] handlers.
type WorkspaceClient interface {
	Create(req contractv1.WorkspaceCreate) (contractv1.Workspace, error)
	List() ([]contractv1.Workspace, error)
	Get(id string) (*contractv1.Workspace, error) // nil if not found
}

// JobClient backs the /v1/index-jobs[*] handlers.
type JobClient interface {
	Enqueue(workspaceID, triggerSource string) (contractv1.IndexJob, error)
	Get(jobID string) (*contractv1.IndexJob, error) // nil if not found
	Cancel(jobID string) error                       // returns ErrJobTerminal if already terminal; ErrNotFound if missing
}

// SearchClient backs POST /v1/search.
type SearchClient interface {
	Search(req contractv1.SearchRequest) (contractv1.SearchResult, contractv1.RetrievalTrace, error)
}

// EventsClient backs GET /v1/observability/events.
type EventsClient interface {
	Recent(limit int) ([]contractv1.ObservabilityEvent, error)
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
