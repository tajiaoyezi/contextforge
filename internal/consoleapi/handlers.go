package consoleapi

import (
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"strconv"
	"strings"
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
	return func(w http.ResponseWriter, r *http.Request) {
		// task-15.6 (Phase 15 P2 #7 / ADR-020): opt-in 5-component detail.
		// When ?detailed=true is present, dispatch to HealthClient (real gRPC
		// in production; nil → synthesize per BackendKind so fallback / degraded
		// modes still report a coherent component map).
		if r.URL.Query().Get("detailed") == "true" {
			writeDetailedHealth(w, deps)
			return
		}
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

// handleGetSourceChunk — GET /v1/source-chunks/{id} (task-12.2 / ADR-017 D1 Wave 2).
// Returns 200 + SourceChunk on hit; 404 when chunk missing; 503 in fallback mode.
// Non-destructive — no confirmMiddleware.
func handleGetSourceChunk(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id := trimID(r)
		if id == "" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST", "missing id")
			return
		}
		chunk, err := deps.Search.GetSourceChunk(id)
		if err != nil {
			mapStorageError(w, err)
			return
		}
		writeJSON(w, http.StatusOK, chunk)
	}
}

// handleGetSearchTrace — GET /v1/search/{query_id}/trace (task-12.3 / ADR-017 D1 Wave 2).
// Returns 200 + RetrievalTrace on hit; 404 when query_id unknown (or evicted
// from the in-memory LRU); 503 in fallback mode. Non-destructive.
func handleGetSearchTrace(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		queryID := strings.TrimSpace(r.PathValue("query_id"))
		if queryID == "" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST", "missing query_id")
			return
		}
		trace, err := deps.Search.GetSearchTrace(queryID)
		if err != nil {
			mapStorageError(w, err)
			return
		}
		writeJSON(w, http.StatusOK, trace)
	}
}

// handleListEvalRuns — GET /v1/eval-runs (task-15.4 / Phase 15 P1 #4).
// Returns 200 + JSON []EvalRun; empty list when no rows match. Optional
// filters: ?workspace_id=, ?status=, ?limit= (default 50, clamped 1..=200).
func handleListEvalRuns(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		filter := contractv1.ListEvalRunsFilter{
			WorkspaceID: strings.TrimSpace(r.URL.Query().Get("workspace_id")),
			Status:      strings.TrimSpace(r.URL.Query().Get("status")),
			Limit:       0,
		}
		if v := r.URL.Query().Get("limit"); v != "" {
			if n, err := strconv.Atoi(v); err == nil && n > 0 {
				if n > 200 {
					n = 200
				}
				filter.Limit = int32(n)
			}
		}
		runs, err := deps.Eval.List(filter)
		if err != nil {
			mapStorageError(w, err)
			return
		}
		if runs == nil {
			runs = []contractv1.EvalRun{}
		}
		writeJSON(w, http.StatusOK, runs)
	}
}

// writeDetailedHealth — task-15.6 (Phase 15 P2 #7 / ADR-020) sub-handler.
// Dispatches to the gRPC HealthService when wired; falls back to a synthetic
// 5-component view when running in MemStore / degraded modes so the Console
// UI's CoreHealthCard always sees a complete shape.
func writeDetailedHealth(w http.ResponseWriter, deps Deps) {
	if deps.Health != nil {
		detailed, err := deps.Health.GetDetailed()
		if err == nil {
			status := http.StatusOK
			if detailed.Status == "unreachable" {
				status = http.StatusServiceUnavailable
			}
			writeJSON(w, status, detailed)
			return
		}
		// Fall through to synthetic on error.
	}
	now := time.Now().UTC()
	mkLat := func(ms int64) *int64 { return &ms }
	mkReason := func(s string) *string { return &s }
	makeComps := func(allHealthy bool, errorReason string) map[string]contractv1.ComponentHealth {
		out := make(map[string]contractv1.ComponentHealth, 5)
		for _, name := range []string{"db", "index", "embed", "retriever", "eval"} {
			c := contractv1.ComponentHealth{Name: name, Status: "healthy", LatencyMs: mkLat(0)}
			if !allHealthy {
				c.Status = "degraded"
				c.ErrorReason = mkReason(errorReason)
			}
			out[name] = c
		}
		return out
	}
	switch deps.BackendKind {
	case "inmem-fallback":
		reason := "console-api: in-memory fallback store active (ADR-016 §D4); component probes not real"
		zero := int64(0)
		writeJSON(w, http.StatusOK, contractv1.CoreHealth{
			Status:          "degraded",
			ContractVersion: contractv1.ContractVersion,
			LastConnectedAt: nil,
			ErrorReason:     &reason,
			MissingMustHaveFields: []contractv1.FieldAvailability{
				{Object: "core", Missing: []string{"data_plane_persistence"}},
			},
			Components:     makeComps(false, "inmem fallback (no real probe)"),
			TotalLatencyMs: &zero,
		})
	case "degraded":
		reason := "console-api: data plane gRPC unreachable (ADR-016 §D4)"
		zero := int64(0)
		writeJSON(w, http.StatusServiceUnavailable, contractv1.CoreHealth{
			Status:          "unreachable",
			ContractVersion: contractv1.ContractVersion,
			LastConnectedAt: nil,
			ErrorReason:     &reason,
			MissingMustHaveFields: []contractv1.FieldAvailability{
				{Object: "core", Missing: []string{"data_plane"}},
			},
			Components:     makeComps(false, "data plane unreachable"),
			TotalLatencyMs: &zero,
		})
	default: // "grpc" or unset but no HealthClient wired — synthesize all-healthy
		zero := int64(0)
		writeJSON(w, http.StatusOK, contractv1.CoreHealth{
			Status:                "healthy",
			ContractVersion:       contractv1.ContractVersion,
			LastConnectedAt:       &now,
			ErrorReason:           nil,
			MissingMustHaveFields: nil,
			Components:            makeComps(true, ""),
			TotalLatencyMs:        &zero,
		})
	}
}

// handleListQueries — GET /v1/queries (task-15.5 / Phase 15 P1 #5).
// Returns 200 + JSON []QueryRecord (most-recent first). ?limit= clamps
// 1..=100; default 20 when missing or invalid.
func handleListQueries(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		limit := 20
		if v := r.URL.Query().Get("limit"); v != "" {
			if n, err := strconv.Atoi(v); err == nil && n > 0 {
				if n > 100 {
					n = 100
				}
				limit = n
			}
		}
		records, err := deps.Search.ListQueries(limit)
		if err != nil {
			mapStorageError(w, err)
			return
		}
		if records == nil {
			records = []contractv1.QueryRecord{}
		}
		writeJSON(w, http.StatusOK, records)
	}
}

// handleGetChunksStats — GET /v1/stats/chunks (task-15.3 / Phase 15 P1 #3).
// Returns 200 + contractv1.ChunksStats; 503 in fallback when SearchBackend
// is unwired (MemStore stub returns zero). Optional ?workspace_id= filters
// to a single collection; default is cross-workspace aggregate.
func handleGetChunksStats(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		workspaceID := strings.TrimSpace(r.URL.Query().Get("workspace_id"))
		stats, err := deps.Search.GetChunksStats(workspaceID)
		if err != nil {
			mapStorageError(w, err)
			return
		}
		writeJSON(w, http.StatusOK, stats)
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

// =====================================================================
// task-13.2 (ADR-017 D1 Wave 3) — 5 memory REST handlers.
// =====================================================================

// handleListMemory — GET /v1/memory[?agent_id=&scope=&namespace=&include_soft_deleted=].
func handleListMemory(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if deps.Memory == nil {
			writeError(w, http.StatusServiceUnavailable, "SERVICE_UNAVAILABLE", ErrDataPlaneUnavailable.Error())
			return
		}
		q := r.URL.Query()
		filter := MemoryListFilter{
			AgentID:            q.Get("agent_id"),
			Scope:              q.Get("scope"),
			Namespace:          q.Get("namespace"),
			IncludeSoftDeleted: q.Get("include_soft_deleted") == "true",
		}
		items, err := deps.Memory.List(filter)
		if err != nil {
			mapStorageError(w, err)
			return
		}
		if items == nil {
			items = []contractv1.MemoryItem{}
		}
		writeJSON(w, http.StatusOK, items)
	}
}

// handleGetMemory — GET /v1/memory/{id}.
func handleGetMemory(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if deps.Memory == nil {
			writeError(w, http.StatusServiceUnavailable, "SERVICE_UNAVAILABLE", ErrDataPlaneUnavailable.Error())
			return
		}
		id := trimID(r)
		if id == "" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST", "missing id")
			return
		}
		item, err := deps.Memory.Get(id)
		if err != nil {
			mapStorageError(w, err)
			return
		}
		if item == nil {
			writeError(w, http.StatusNotFound, "NOT_FOUND", "memory item not found: "+id)
			return
		}
		writeJSON(w, http.StatusOK, *item)
	}
}

// handleMemoryPin — POST /v1/memory/{id}/pin → 204 (non-destructive).
//
// task-17.1 / ADR-022 D2: body shape `{"pin": bool}` toggles state. Empty body
// (v0.7-v0.9 callers) or absent `pin` key falls back to `pin=true` so existing
// callers that POST without a body keep working (backward compat); malformed
// JSON also falls back rather than 400 to preserve the v0.7 lenient contract.
func handleMemoryPin(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if deps.Memory == nil {
			writeError(w, http.StatusServiceUnavailable, "SERVICE_UNAVAILABLE", ErrDataPlaneUnavailable.Error())
			return
		}
		id := trimID(r)
		if id == "" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST", "missing id")
			return
		}
		pin := true
		var body struct {
			Pin *bool `json:"pin"`
		}
		if err := json.NewDecoder(r.Body).Decode(&body); err == nil && body.Pin != nil {
			pin = *body.Pin
		}
		if err := deps.Memory.Pin(id, pin); err != nil {
			mapStorageError(w, err)
			return
		}
		w.WriteHeader(http.StatusNoContent)
	}
}

// handleMemoryDeprecate — POST /v1/memory/{id}/deprecate → 204 (destructive; confirmMiddleware-gated).
func handleMemoryDeprecate(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if deps.Memory == nil {
			writeError(w, http.StatusServiceUnavailable, "SERVICE_UNAVAILABLE", ErrDataPlaneUnavailable.Error())
			return
		}
		id := trimID(r)
		if id == "" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST", "missing id")
			return
		}
		if err := deps.Memory.Deprecate(id); err != nil {
			mapStorageError(w, err)
			return
		}
		w.WriteHeader(http.StatusNoContent)
	}
}

// handleMemorySoftDelete — POST /v1/memory/{id}/soft-delete → 204 (destructive; confirmMiddleware-gated).
func handleMemorySoftDelete(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if deps.Memory == nil {
			writeError(w, http.StatusServiceUnavailable, "SERVICE_UNAVAILABLE", ErrDataPlaneUnavailable.Error())
			return
		}
		id := trimID(r)
		if id == "" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST", "missing id")
			return
		}
		if err := deps.Memory.SoftDelete(id); err != nil {
			mapStorageError(w, err)
			return
		}
		w.WriteHeader(http.StatusNoContent)
	}
}

// =====================================================================
// task-14.2 (ADR-017 D1 Wave 4) — 2 eval REST handlers.
// =====================================================================

// handleCreateEvalRun — POST /v1/eval-runs.
// Body: contractv1.EvalRunCreate. Returns 200 + EvalRun status="running";
// spawns runEvalAsync goroutine that drives recall harness + reverse-updates
// store via EvalService.UpdateProgress when terminal.
func handleCreateEvalRun(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if deps.Eval == nil {
			writeError(w, http.StatusServiceUnavailable, "SERVICE_UNAVAILABLE", ErrDataPlaneUnavailable.Error())
			return
		}
		var body contractv1.EvalRunCreate
		if !readJSONBody(w, r, &body) {
			return
		}
		run, err := deps.Eval.Create(body)
		if err != nil {
			mapStorageError(w, err)
			return
		}
		// Spawn async runner; the goroutine survives the request.
		go runEvalAsync(deps, run.EvalRunID, body)
		writeJSON(w, http.StatusOK, run)
	}
}

// handleGetEvalRun — GET /v1/eval-runs/{id}.
func handleGetEvalRun(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if deps.Eval == nil {
			writeError(w, http.StatusServiceUnavailable, "SERVICE_UNAVAILABLE", ErrDataPlaneUnavailable.Error())
			return
		}
		id := trimID(r)
		if id == "" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST", "missing id")
			return
		}
		run, err := deps.Eval.Get(id)
		if err != nil {
			mapStorageError(w, err)
			return
		}
		if run == nil {
			writeError(w, http.StatusNotFound, "NOT_FOUND", "eval run not found: "+id)
			return
		}
		writeJSON(w, http.StatusOK, *run)
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
		// task-16.2 (Phase 16 P4 #11): pass parsed `wait` down to grpcclient
		// (was previously discarded; the gRPC client used a hardcoded 30s ctx).
		// Recent now drives a two-phase long-poll: phase-1 blocks up to `wait`
		// for the first event; phase-2 drains immediately-available events with
		// a short (~100ms) timeout.
		wait := parseWaitParam(r)
		limit := parseLimitParam(r, defaultLimit)
		evts, err := deps.Events.Recent(limit, wait)
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
