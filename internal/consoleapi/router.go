package consoleapi

import (
	"crypto/subtle"
	"encoding/json"
	"errors"
	"net/http"
	"strings"
)

// NewRouter returns the http.Handler tree for the Console Contract v1
// endpoints + bearer auth middleware + JSON error mapping.
//
// task-12.1 (ADR-017 D1 Wave 1) extends v0.4 9 endpoints with:
//   - PATCH /v1/workspaces/{id}/config (confirmMiddleware-guarded)
//   - GET /v1/index-jobs?status=active (active-only list)
//   - POST /v1/index-jobs/{id}/cancel returns 204 No Content
func NewRouter(deps Deps) http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /v1/health", handleHealth(deps))
	mux.HandleFunc("POST /v1/workspaces", handleCreateWorkspace(deps))
	mux.HandleFunc("GET /v1/workspaces", handleListWorkspaces(deps))
	mux.HandleFunc("GET /v1/workspaces/{id}", handleGetWorkspace(deps))
	mux.HandleFunc("PATCH /v1/workspaces/{id}/config", confirmMiddleware(handlePatchWorkspaceConfig(deps)))
	mux.HandleFunc("POST /v1/index-jobs", handleEnqueueJob(deps))
	mux.HandleFunc("GET /v1/index-jobs", handleListJobs(deps))
	mux.HandleFunc("GET /v1/index-jobs/{id}", handleGetJob(deps))
	mux.HandleFunc("POST /v1/index-jobs/{id}/cancel", handleCancelJob(deps))
	mux.HandleFunc("POST /v1/search", handleSearch(deps))
	mux.HandleFunc("GET /v1/source-chunks/{id}", handleGetSourceChunk(deps))
	mux.HandleFunc("GET /v1/search/{query_id}/trace", handleGetSearchTrace(deps))
	mux.HandleFunc("GET /v1/observability/events", handleEvents(deps))
	// task-26.2 (ADR-031 D3): SSE real-time push, add-only alongside the
	// existing long-poll endpoint above (既有路由 / 22-endpoint 契约不动).
	mux.HandleFunc("GET /v1/observability/events/stream", handleEventsStream(deps))
	// task-15.3 (Phase 15 P1 #3): Dashboard "已索引块" stats endpoint.
	// Non-destructive — no confirmMiddleware. Optional ?workspace_id= query.
	mux.HandleFunc("GET /v1/stats/chunks", handleGetChunksStats(deps))
	// task-15.5 (Phase 15 P1 #5): query history. ?limit= default 20, max 100.
	mux.HandleFunc("GET /v1/queries", handleListQueries(deps))
	// task-13.2 (ADR-017 D1 Wave 3): 5 memory endpoints; deprecate + soft-delete
	// gated by confirmMiddleware (destructive ops).
	mux.HandleFunc("GET /v1/memory", handleListMemory(deps))
	mux.HandleFunc("GET /v1/memory/{id}", handleGetMemory(deps))
	mux.HandleFunc("POST /v1/memory/{id}/pin", handleMemoryPin(deps))
	mux.HandleFunc("POST /v1/memory/{id}/deprecate", confirmMiddleware(handleMemoryDeprecate(deps)))
	mux.HandleFunc("POST /v1/memory/{id}/soft-delete", confirmMiddleware(handleMemorySoftDelete(deps)))
	// task-27.2 (ADR-032 D2): explicit unpin (non-destructive) + hard-delete
	// (destructive — confirmMiddleware-gated, X-Confirm per ADR-017 D2). add-only.
	mux.HandleFunc("POST /v1/memory/{id}/unpin", handleMemoryUnpin(deps))
	mux.HandleFunc("POST /v1/memory/{id}/hard-delete", confirmMiddleware(handleMemoryHardDelete(deps)))
	// task-14.2 (ADR-017 D1 Wave 4): eval 2 endpoints (POST is non-destructive
	// — trigger async run, return 200 with running; GET reads state).
	mux.HandleFunc("POST /v1/eval-runs", handleCreateEvalRun(deps))
	mux.HandleFunc("GET /v1/eval-runs/{id}", handleGetEvalRun(deps))
	// task-15.4 (Phase 15 P1 #4): list eval runs for "最近评测" panel.
	// Non-destructive read; ?workspace_id= ?status= ?limit= optional.
	mux.HandleFunc("GET /v1/eval-runs", handleListEvalRuns(deps))
	return bearerAuthMiddleware(mux, deps.AuthToken)
}

// confirmMiddleware enforces ADR-017 D2 server-side bottom defense for
// destructive endpoints: caller must pass `X-Confirm: yes` header OR
// `?confirm=true` query (OR semantics — either suffices). Missing both
// → 412 Precondition Failed.
//
// Console BFF auto-injects the header; ops curl callers may use the query.
// Catches the rare BFF regression that silently strips X-Confirm.
func confirmMiddleware(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Header.Get("X-Confirm") == "yes" || r.URL.Query().Get("confirm") == "true" {
			next.ServeHTTP(w, r)
			return
		}
		writeError(w, http.StatusPreconditionFailed, "PRECONDITION_FAILED",
			"X-Confirm: yes header or ?confirm=true query required for destructive op (ADR-017 D2)")
	}
}

// bearerAuthMiddleware enforces `Authorization: Bearer <token>` when
// `token != ""`. Empty token = trusted-network mode (no header required).
// Constant-time compare avoids timing-side-channel leaks.
func bearerAuthMiddleware(inner http.Handler, token string) http.Handler {
	if token == "" {
		return inner
	}
	expected := "Bearer " + token
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		got := r.Header.Get("Authorization")
		if subtle.ConstantTimeCompare([]byte(got), []byte(expected)) != 1 {
			writeError(w, http.StatusUnauthorized, "UNAUTHORIZED", "missing or invalid bearer token")
			return
		}
		inner.ServeHTTP(w, r)
	})
}

// writeJSON marshals v as JSON with Content-Type application/json + status.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

// writeError emits the ErrorBody shape with the supplied status code, code
// label, and message. Code labels align loosely with Console HTTPAdapter
// sentinel mapping (NOT_FOUND / CONFLICT / UNAUTHORIZED / BAD_REQUEST).
func writeError(w http.ResponseWriter, status int, code, message string) {
	var body ErrorBody
	body.Error.Code = code
	body.Error.Message = message
	writeJSON(w, status, body)
}

// mapStorageError translates a backend error into a writeError + return so
// handlers stay tiny.
//
// task-11.2 (ADR-016 §D4): ErrDataPlaneUnavailable → 503 Service Unavailable
// so Console UI can render the "Core unreachable" degraded mode (REST adapter
// treats 503 as transient + retries; Mock Adapter swaps in if configured).
func mapStorageError(w http.ResponseWriter, err error) {
	switch {
	case errors.Is(err, ErrNotFound):
		writeError(w, http.StatusNotFound, "NOT_FOUND", err.Error())
	case errors.Is(err, ErrJobTerminal):
		writeError(w, http.StatusConflict, "CONFLICT", err.Error())
	case errors.Is(err, ErrInvalidRequest):
		writeError(w, http.StatusBadRequest, "BAD_REQUEST", err.Error())
	case errors.Is(err, ErrDataPlaneUnavailable):
		writeError(w, http.StatusServiceUnavailable, "SERVICE_UNAVAILABLE", err.Error())
	case errors.Is(err, ErrPreconditionRequired):
		writeError(w, http.StatusPreconditionFailed, "PRECONDITION_FAILED", err.Error())
	default:
		writeError(w, http.StatusInternalServerError, "INTERNAL", err.Error())
	}
}

// readJSONBody decodes the request body into out. Returns false (and writes
// 400) on failure; handlers should `return` immediately when false.
func readJSONBody(w http.ResponseWriter, r *http.Request, out any) bool {
	dec := json.NewDecoder(r.Body)
	dec.DisallowUnknownFields()
	if err := dec.Decode(out); err != nil {
		writeError(w, http.StatusBadRequest, "BAD_REQUEST", "invalid JSON body: "+err.Error())
		return false
	}
	return true
}

// trimID is a tiny helper extracting and trimming PathValue("id").
func trimID(r *http.Request) string {
	return strings.TrimSpace(r.PathValue("id"))
}
