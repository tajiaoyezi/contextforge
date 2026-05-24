package consoleapi

import (
	"errors"
	"fmt"
	"net/http"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/contractv1"
)

// handleHealth — GET /v1/health.
// Returns CoreHealth with contract_version "v1" (Console HTTPAdapter expects
// this constant in a must-have field).
//
// task-11.2 (ADR-016 §D4): BackendKind drives degraded reporting —
//   - "grpc" / "" (default): 200 + status="healthy"
//   - "inmem-fallback": 200 + status="degraded" + ErrorReason mentions inmem fallback
//   - "degraded": 503 + status="degraded" + MissingMustHaveFields=[{Object:"core",Missing:["data_plane"]}]
func handleHealth(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, _ *http.Request) {
		now := time.Now().UTC()
		switch deps.BackendKind {
		case "inmem-fallback":
			reason := "console-api: in-memory fallback store active (data plane bypassed; ADR-016 §D4)"
			writeJSON(w, http.StatusOK, contractv1.CoreHealth{
				Status:          "degraded",
				ContractVersion: contractv1.ContractVersion,
				LastConnectedAt: nil,
				ErrorReason:     &reason,
				MissingMustHaveFields: []contractv1.FieldAvailability{
					{Object: "core", Missing: []string{"data_plane_persistence"}},
				},
			})
		case "degraded":
			reason := "console-api: data plane gRPC unreachable; set CONSOLE_API_FALLBACK_INMEM=1 OR start contextforge-core daemon (ADR-016 §D4)"
			writeJSON(w, http.StatusServiceUnavailable, contractv1.CoreHealth{
				Status:          "degraded",
				ContractVersion: contractv1.ContractVersion,
				LastConnectedAt: nil,
				ErrorReason:     &reason,
				MissingMustHaveFields: []contractv1.FieldAvailability{
					{Object: "core", Missing: []string{"data_plane"}},
				},
			})
		default: // "grpc" or unset
			writeJSON(w, http.StatusOK, contractv1.CoreHealth{
				Status:                "healthy",
				ContractVersion:       contractv1.ContractVersion,
				LastConnectedAt:       &now,
				ErrorReason:           nil,
				MissingMustHaveFields: nil,
			})
		}
	}
}

// handleCreateWorkspace — POST /v1/workspaces.
func handleCreateWorkspace(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var body contractv1.WorkspaceCreate
		if !readJSONBody(w, r, &body) {
			return
		}
		ws, err := deps.Workspace.Create(body)
		if err != nil {
			mapStorageError(w, err)
			return
		}
		writeJSON(w, http.StatusOK, ws)
	}
}

// handleListWorkspaces — GET /v1/workspaces.
func handleListWorkspaces(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, _ *http.Request) {
		list, err := deps.Workspace.List()
		if err != nil {
			mapStorageError(w, err)
			return
		}
		if list == nil {
			list = []contractv1.Workspace{}
		}
		writeJSON(w, http.StatusOK, list)
	}
}

// handleGetWorkspace — GET /v1/workspaces/{id}.
func handleGetWorkspace(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id := trimID(r)
		if id == "" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST", "missing id")
			return
		}
		ws, err := deps.Workspace.Get(id)
		if err != nil {
			mapStorageError(w, err)
			return
		}
		if ws == nil {
			writeError(w, http.StatusNotFound, "NOT_FOUND", "workspace not found: "+id)
			return
		}
		writeJSON(w, http.StatusOK, *ws)
	}
}

// handlePatchWorkspaceConfig — PATCH /v1/workspaces/{id}/config.
// Body shape: {"allowlist": [...], "denylist": [...]}. Both fields required
// (覆盖式更新)。X-Confirm/?confirm=true enforced upstream by confirmMiddleware.
//
// task-12.1 (ADR-017 D1 Wave 1) — calls deps.Workspace.Update; returns 200 +
// updated Workspace on success; ErrNotFound → 404; ErrInvalidRequest → 400.
func handlePatchWorkspaceConfig(deps Deps) http.HandlerFunc {
	type patchBody struct {
		Allowlist []string `json:"allowlist"`
		Denylist  []string `json:"denylist"`
	}
	return func(w http.ResponseWriter, r *http.Request) {
		id := trimID(r)
		if id == "" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST", "missing id")
			return
		}
		var body patchBody
		if !readJSONBody(w, r, &body) {
			return
		}
		allow := body.Allowlist
		deny := body.Denylist
		if allow == nil {
			allow = []string{}
		}
		if deny == nil {
			deny = []string{}
		}
		ws, err := deps.Workspace.Update(id, allow, deny)
		if err != nil {
			mapStorageError(w, err)
			return
		}
		writeJSON(w, http.StatusOK, ws)
	}
}

// handleEnqueueJob — POST /v1/index-jobs.
// Body shape: {"workspace_id": "..."} (Console HTTPAdapter convention).
func handleEnqueueJob(deps Deps) http.HandlerFunc {
	type enqueueBody struct {
		WorkspaceID   string `json:"workspace_id"`
		TriggerSource string `json:"trigger_source,omitempty"`
	}
	return func(w http.ResponseWriter, r *http.Request) {
		var body enqueueBody
		if !readJSONBody(w, r, &body) {
			return
		}
		trigger := body.TriggerSource
		if trigger == "" {
			trigger = "rest"
		}
		job, err := deps.Job.Enqueue(body.WorkspaceID, trigger)
		if err != nil {
			mapStorageError(w, err)
			return
		}
		writeJSON(w, http.StatusOK, job)
	}
}

// handleListJobs — GET /v1/index-jobs?status=active.
//
// task-12.1 (ADR-017 D1 Wave 1) — v1.0 only supports the ?status=active filter
// (queued + running). Missing or other status returns 400 [SPEC-DEFER:console-list-all-jobs]
// 留 v1.x. Empty active set returns 200 + [] (not 204).
func handleListJobs(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		status := r.URL.Query().Get("status")
		if status != "active" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST",
				"?status=active required (v1 only supports active filter)")
			return
		}
		jobs, err := deps.Job.ListActive()
		if err != nil {
			mapStorageError(w, err)
			return
		}
		if jobs == nil {
			jobs = []contractv1.IndexJob{}
		}
		writeJSON(w, http.StatusOK, jobs)
	}
}

// handleGetJob — GET /v1/index-jobs/{id}.
func handleGetJob(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id := trimID(r)
		if id == "" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST", "missing id")
			return
		}
		job, err := deps.Job.Get(id)
		if err != nil {
			mapStorageError(w, err)
			return
		}
		if job == nil {
			writeError(w, http.StatusNotFound, "NOT_FOUND", "index job not found: "+id)
			return
		}
		writeJSON(w, http.StatusOK, *job)
	}
}

// handleCancelJob — POST /v1/index-jobs/{id}/cancel.
//
// task-12.1 (ADR-017 D3): 204 No Content on accepted cancel (was 200 in v0.4).
// Console HTTPAdapter accepts both per cross-repo v1.0 dual-check; ops scripts
// should treat 2xx as success. 409 Conflict if terminal; 404 if not found.
func handleCancelJob(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id := trimID(r)
		if id == "" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST", "missing id")
			return
		}
		err := deps.Job.Cancel(id)
		switch {
		case err == nil:
			w.WriteHeader(http.StatusNoContent)
		case errors.Is(err, ErrNotFound):
			writeError(w, http.StatusNotFound, "NOT_FOUND", "index job not found: "+id)
		case errors.Is(err, ErrJobTerminal):
			writeError(w, http.StatusConflict, "CONFLICT", err.Error())
		default:
			mapStorageError(w, err)
		}
	}
}

// handleSearch — POST /v1/search.
// Body shape: contractv1.SearchRequest. Response: nested {"result":...,"trace":...}.
func handleSearch(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var body contractv1.SearchRequest
		if !readJSONBody(w, r, &body) {
			return
		}
		result, trace, err := deps.Search.Search(body)
		if err != nil {
			mapStorageError(w, err)
			return
		}
		writeJSON(w, http.StatusOK, SearchResponse{Result: result, Trace: trace})
	}
}

// handleEvents — GET /v1/observability/events (task-11.4 long-poll wrap).
//
// Query params:
//   - `wait=<duration>` (optional; default 30s; max 60s) — how long the
//     handler is allowed to block waiting for ≥1 event before returning
//     200 + []. Parsed via time.ParseDuration ("30s" / "1m" forms).
//   - `limit=<int>` (optional; default 100) — max events per batch.
//
// Returns 200 + JSON array of ObservabilityEvent (possibly empty if no
// events arrive within the timeout). Console HTTPAdapter v1.0 expects
// 200 + maybe-empty array (NOT 204) [SPEC-DEFER:task-future.consoleapi-sse].
func handleEvents(deps Deps) http.HandlerFunc {
	const defaultLimit = 100
	return func(w http.ResponseWriter, r *http.Request) {
		// Parse optional wait + limit query params (long-poll knobs).
		_ = parseWaitParam(r) // task-11.4: currently passed to gRPC via grpcclient ctx timeout
		limit := parseLimitParam(r, defaultLimit)
		evts, err := deps.Events.Recent(limit)
		if err != nil {
			mapStorageError(w, err)
			return
		}
		if evts == nil {
			evts = []contractv1.ObservabilityEvent{}
		}
		writeJSON(w, http.StatusOK, evts)
	}
}

// parseWaitParam reads ?wait=30s; default 30s; clamped to [1s, 60s].
func parseWaitParam(r *http.Request) time.Duration {
	raw := r.URL.Query().Get("wait")
	if raw == "" {
		return 30 * time.Second
	}
	d, err := time.ParseDuration(raw)
	if err != nil {
		return 30 * time.Second
	}
	if d < time.Second {
		return time.Second
	}
	if d > 60*time.Second {
		return 60 * time.Second
	}
	return d
}

// parseLimitParam reads ?limit=N; defaults to fallback when missing / invalid.
// Clamps to [1, 500] to bound memory.
func parseLimitParam(r *http.Request, fallback int) int {
	raw := r.URL.Query().Get("limit")
	if raw == "" {
		return fallback
	}
	var n int
	if _, err := fmtSscanf(raw, "%d", &n); err != nil {
		return fallback
	}
	if n < 1 {
		return 1
	}
	if n > 500 {
		return 500
	}
	return n
}

// fmtSscanf wraps fmt.Sscanf for a tiny helper boundary (avoids adding fmt
// to the package-level import set when we already use stdlib net/http /
// encoding/json / time).
func fmtSscanf(s, format string, a ...any) (int, error) {
	return fmt.Sscanf(s, format, a...)
}
