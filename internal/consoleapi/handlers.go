package consoleapi

import (
	"errors"
	"net/http"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/contractv1"
)

// handleHealth — GET /v1/health.
// Returns CoreHealth with contract_version "v1" (Console HTTPAdapter expects
// this constant in a must-have field).
func handleHealth(_ Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, _ *http.Request) {
		now := time.Now().UTC()
		writeJSON(w, http.StatusOK, contractv1.CoreHealth{
			Status:                "healthy",
			ContractVersion:       contractv1.ContractVersion,
			LastConnectedAt:       &now,
			ErrorReason:           nil,
			MissingMustHaveFields: nil,
		})
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
// 200 OK on accepted cancel; 409 Conflict if terminal; 404 if not found.
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
			w.WriteHeader(http.StatusOK)
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

// handleEvents — GET /v1/observability/events (long-poll-style; returns
// most-recent events list, Console HTTPAdapter v1.0 does not consume SSE
// [SPEC-DEFER:task-future.consoleapi-sse]).
func handleEvents(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, _ *http.Request) {
		const defaultLimit = 100
		evts, err := deps.Events.Recent(defaultLimit)
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
