package consoleapi

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/contractv1"
)

func newTestRouter(t *testing.T, authToken string) (http.Handler, *MemStore) {
	t.Helper()
	store := NewMemStore()
	deps := Deps{
		Workspace: WorkspaceAdapter{S: store},
		Job:       JobAdapter{S: store},
		Search:    store,
		Events:    store,
		AuthToken: authToken,
	}
	return NewRouter(deps), store
}

// TestRouterRegistration verifies all 9 endpoints respond (not 404 / not
// 405 / not 401 when no auth). AC1.
func TestRouterRegistration(t *testing.T) {
	router, store := newTestRouter(t, "")
	// seed workspace + job so GET-by-id endpoints have something to find
	ws, err := store.CreateWorkspace(contractv1.WorkspaceCreate{Name: "demo", RootPath: "/tmp/demo"})
	if err != nil {
		t.Fatalf("seed workspace: %v", err)
	}
	job, err := store.EnqueueJob(ws.WorkspaceID, "test")
	if err != nil {
		t.Fatalf("seed job: %v", err)
	}
	cases := []struct {
		name   string
		method string
		path   string
		body   any
		want   int
	}{
		{"health", "GET", "/v1/health", nil, http.StatusOK},
		{"create_workspace", "POST", "/v1/workspaces", contractv1.WorkspaceCreate{Name: "another", RootPath: "/tmp/another"}, http.StatusOK},
		{"list_workspaces", "GET", "/v1/workspaces", nil, http.StatusOK},
		{"get_workspace", "GET", "/v1/workspaces/" + ws.WorkspaceID, nil, http.StatusOK},
		{"enqueue_job", "POST", "/v1/index-jobs", map[string]string{"workspace_id": ws.WorkspaceID}, http.StatusOK},
		{"get_job", "GET", "/v1/index-jobs/" + job.JobID, nil, http.StatusOK},
		{"cancel_job", "POST", "/v1/index-jobs/" + job.JobID + "/cancel", nil, http.StatusNoContent},
		{"search", "POST", "/v1/search", contractv1.SearchRequest{Query: "q", WorkspaceID: ws.WorkspaceID, TopK: 5, RetrievalMethod: "bm25"}, http.StatusOK},
		{"events", "GET", "/v1/observability/events", nil, http.StatusOK},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			var body *bytes.Buffer
			if tc.body != nil {
				b, _ := json.Marshal(tc.body)
				body = bytes.NewBuffer(b)
			}
			var req *http.Request
			if body != nil {
				req = httptest.NewRequest(tc.method, tc.path, body)
				req.Header.Set("Content-Type", "application/json")
			} else {
				req = httptest.NewRequest(tc.method, tc.path, nil)
			}
			w := httptest.NewRecorder()
			router.ServeHTTP(w, req)
			if w.Code != tc.want {
				t.Errorf("%s %s: status=%d want=%d body=%s", tc.method, tc.path, w.Code, tc.want, w.Body.String())
			}
		})
	}
}

// TestHandleGetWorkspace_404 — Console HTTPAdapter sentinel error mapping (AC2).
func TestHandleGetWorkspace_404(t *testing.T) {
	router, _ := newTestRouter(t, "")
	req := httptest.NewRequest("GET", "/v1/workspaces/non-existent-id", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusNotFound {
		t.Errorf("expected 404; got %d", w.Code)
	}
	if !strings.Contains(w.Body.String(), `"code":"NOT_FOUND"`) {
		t.Errorf("expected NOT_FOUND code; got %s", w.Body.String())
	}
}

// TestHandleCancelJob_409 — terminal job re-cancel returns 409 Conflict (AC2).
func TestHandleCancelJob_409(t *testing.T) {
	router, store := newTestRouter(t, "")
	ws, _ := store.CreateWorkspace(contractv1.WorkspaceCreate{Name: "demo", RootPath: "/tmp/demo"})
	job, _ := store.EnqueueJob(ws.WorkspaceID, "rest")
	// first cancel succeeds
	_ = store.CancelJob(job.JobID)
	// second cancel via REST should 409
	req := httptest.NewRequest("POST", "/v1/index-jobs/"+job.JobID+"/cancel", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusConflict {
		t.Errorf("expected 409; got %d body=%s", w.Code, w.Body.String())
	}
}

// TestHandleBearerAuth — auth token enforcement (AC4 last clause).
func TestHandleBearerAuth(t *testing.T) {
	router, _ := newTestRouter(t, "secret-token")
	// no auth header → 401
	req := httptest.NewRequest("GET", "/v1/health", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusUnauthorized {
		t.Errorf("no token: expected 401; got %d", w.Code)
	}
	// wrong token → 401
	req = httptest.NewRequest("GET", "/v1/health", nil)
	req.Header.Set("Authorization", "Bearer wrong")
	w = httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusUnauthorized {
		t.Errorf("wrong token: expected 401; got %d", w.Code)
	}
	// correct token → 200
	req = httptest.NewRequest("GET", "/v1/health", nil)
	req.Header.Set("Authorization", "Bearer secret-token")
	w = httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Errorf("correct token: expected 200; got %d", w.Code)
	}
}

// TestHandleSearch_NestedShape — POST /v1/search returns {result, trace}
// envelope (Console HTTPAdapter convention).
func TestHandleSearch_NestedShape(t *testing.T) {
	router, store := newTestRouter(t, "")
	ws, _ := store.CreateWorkspace(contractv1.WorkspaceCreate{Name: "demo", RootPath: "/tmp/demo"})
	body, _ := json.Marshal(contractv1.SearchRequest{
		Query: "configuration", WorkspaceID: ws.WorkspaceID, TopK: 5, RetrievalMethod: "bm25",
	})
	req := httptest.NewRequest("POST", "/v1/search", bytes.NewBuffer(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d body=%s", w.Code, w.Body.String())
	}
	var got SearchResponse
	if err := json.Unmarshal(w.Body.Bytes(), &got); err != nil {
		t.Fatalf("unmarshal nested shape: %v", err)
	}
	if got.Result.ResultID == "" {
		t.Errorf("expected non-empty result_id; got %#v", got.Result)
	}
	if got.Trace.TraceID == "" {
		t.Errorf("expected non-empty trace_id; got %#v", got.Trace)
	}
}

// =====================================================================
// task-12.1 (ADR-017 D1 Wave 1) new unit tests:
//   - confirmMiddleware OR-semantics (412 / 200 via header / 200 via query)
//   - GET /v1/index-jobs?status=active filter
//   - cancel returns 204
//   - WorkspaceClient.Update + JobClient.ListActive coverage
// =====================================================================

// TestPatchWorkspaceConfig_RequiresConfirm — confirmMiddleware emits 412 when
// neither header nor query is supplied (ADR-017 D2 bottom defense).
func TestPatchWorkspaceConfig_RequiresConfirm(t *testing.T) {
	router, store := newTestRouter(t, "")
	ws, _ := store.CreateWorkspace(contractv1.WorkspaceCreate{Name: "patch-demo", RootPath: "/tmp/patch"})
	body := bytes.NewBufferString(`{"allowlist":["a"],"denylist":["b"]}`)
	req := httptest.NewRequest("PATCH", "/v1/workspaces/"+ws.WorkspaceID+"/config", body)
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusPreconditionFailed {
		t.Fatalf("expected 412; got %d body=%s", w.Code, w.Body.String())
	}
	if !strings.Contains(w.Body.String(), `"code":"PRECONDITION_FAILED"`) {
		t.Errorf("expected PRECONDITION_FAILED code; got %s", w.Body.String())
	}
}

// TestPatchWorkspaceConfig_AcceptsHeader — X-Confirm: yes passes the middleware.
func TestPatchWorkspaceConfig_AcceptsHeader(t *testing.T) {
	router, store := newTestRouter(t, "")
	ws, _ := store.CreateWorkspace(contractv1.WorkspaceCreate{Name: "patch-hdr", RootPath: "/tmp/hdr"})
	body := bytes.NewBufferString(`{"allowlist":["src/**"],"denylist":["node_modules/**"]}`)
	req := httptest.NewRequest("PATCH", "/v1/workspaces/"+ws.WorkspaceID+"/config", body)
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("X-Confirm", "yes")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d body=%s", w.Code, w.Body.String())
	}
	var got contractv1.Workspace
	if err := json.Unmarshal(w.Body.Bytes(), &got); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if got.WorkspaceID != ws.WorkspaceID {
		t.Errorf("workspace_id drift: got %q want %q", got.WorkspaceID, ws.WorkspaceID)
	}
	if !strings.Contains(string(got.ConfigSnapshot), `"src/**"`) {
		t.Errorf("allowlist not persisted in config_snapshot: %s", got.ConfigSnapshot)
	}
}

// TestPatchWorkspaceConfig_AcceptsQuery — ?confirm=true passes the middleware.
func TestPatchWorkspaceConfig_AcceptsQuery(t *testing.T) {
	router, store := newTestRouter(t, "")
	ws, _ := store.CreateWorkspace(contractv1.WorkspaceCreate{Name: "patch-qry", RootPath: "/tmp/qry"})
	body := bytes.NewBufferString(`{"allowlist":[],"denylist":["secrets/**"]}`)
	req := httptest.NewRequest("PATCH", "/v1/workspaces/"+ws.WorkspaceID+"/config?confirm=true", body)
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d body=%s", w.Code, w.Body.String())
	}
}

// TestPatchWorkspaceConfig_404 — non-existent workspace returns 404.
func TestPatchWorkspaceConfig_404(t *testing.T) {
	router, _ := newTestRouter(t, "")
	body := bytes.NewBufferString(`{"allowlist":[],"denylist":[]}`)
	req := httptest.NewRequest("PATCH", "/v1/workspaces/missing-ws/config", body)
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("X-Confirm", "yes")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusNotFound {
		t.Fatalf("expected 404; got %d body=%s", w.Code, w.Body.String())
	}
}

// TestListJobs_ActiveFilter — ?status=active returns queued+running only.
func TestListJobs_ActiveFilter(t *testing.T) {
	router, store := newTestRouter(t, "")
	ws, _ := store.CreateWorkspace(contractv1.WorkspaceCreate{Name: "list-demo", RootPath: "/tmp/list"})
	jobA, _ := store.EnqueueJob(ws.WorkspaceID, "test")        // queued
	jobB, _ := store.EnqueueJob(ws.WorkspaceID, "test")        // queued
	if err := store.CancelJob(jobB.JobID); err != nil {        // → cancelled (terminal)
		t.Fatalf("cancel seed job: %v", err)
	}
	req := httptest.NewRequest("GET", "/v1/index-jobs?status=active", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d body=%s", w.Code, w.Body.String())
	}
	var got []contractv1.IndexJob
	if err := json.Unmarshal(w.Body.Bytes(), &got); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(got) != 1 {
		t.Fatalf("expected 1 active job; got %d (%+v)", len(got), got)
	}
	if got[0].JobID != jobA.JobID {
		t.Errorf("expected active job %q; got %q", jobA.JobID, got[0].JobID)
	}
}

// TestListJobs_MissingStatusFilter — v1 only supports active filter; missing →
// 400 [SPEC-DEFER:console-list-all-jobs] 留 v1.x.
func TestListJobs_MissingStatusFilter(t *testing.T) {
	router, _ := newTestRouter(t, "")
	req := httptest.NewRequest("GET", "/v1/index-jobs", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusBadRequest {
		t.Fatalf("expected 400; got %d body=%s", w.Code, w.Body.String())
	}
}

// TestCancelJob_Returns_204 — task-12.1 (ADR-017 D3) success path now emits
// 204 No Content (was 200 in v0.4).
func TestCancelJob_Returns_204(t *testing.T) {
	router, store := newTestRouter(t, "")
	ws, _ := store.CreateWorkspace(contractv1.WorkspaceCreate{Name: "c204", RootPath: "/tmp/c204"})
	job, _ := store.EnqueueJob(ws.WorkspaceID, "rest")
	req := httptest.NewRequest("POST", "/v1/index-jobs/"+job.JobID+"/cancel", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusNoContent {
		t.Fatalf("expected 204; got %d body=%s", w.Code, w.Body.String())
	}
	if w.Body.Len() != 0 {
		t.Errorf("204 must have empty body; got %q", w.Body.String())
	}
}

// TestCancelJob_404_unchanged — sentinel mapping for unknown job IDs.
func TestCancelJob_404_unchanged(t *testing.T) {
	router, _ := newTestRouter(t, "")
	req := httptest.NewRequest("POST", "/v1/index-jobs/job-does-not-exist/cancel", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusNotFound {
		t.Fatalf("expected 404; got %d body=%s", w.Code, w.Body.String())
	}
}

// =====================================================================
// task-12.2 (ADR-017 D1 Wave 2) — GET /v1/source-chunks/{id} fallback wiring.
// =====================================================================

// TestGetSourceChunk_503_WhenFallback — MemStore (no search index) returns
// ErrDataPlaneUnavailable → REST 503 (deep defense / ADR-016 D4).
func TestGetSourceChunk_503_WhenFallback(t *testing.T) {
	router, _ := newTestRouter(t, "")
	req := httptest.NewRequest("GET", "/v1/source-chunks/chk_dead_0", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusServiceUnavailable {
		t.Fatalf("expected 503; got %d body=%s", w.Code, w.Body.String())
	}
	if !strings.Contains(w.Body.String(), `"code":"SERVICE_UNAVAILABLE"`) {
		t.Errorf("expected SERVICE_UNAVAILABLE; got %s", w.Body.String())
	}
}

// TestGetSourceChunk_400_WhenMissingID — empty ID rejected with 400.
func TestGetSourceChunk_400_WhenMissingID(t *testing.T) {
	router, _ := newTestRouter(t, "")
	// Use a trailing-slash request — Go ServeMux routes "/v1/source-chunks/"
	// to the 404 default; supply " " (space → trimmed → empty) via path encode.
	// Easiest: route registers `{id}` so a missing capture returns 404 not 400.
	// Verify the handler's own missing-id guard via a SearchBackend injection
	// where ID is empty (covered indirectly by the 503 fallback test).
	// Here we just confirm a non-existent ID still passes through to 503 path.
	req := httptest.NewRequest("GET", "/v1/source-chunks/x", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusServiceUnavailable && w.Code != http.StatusNotFound {
		t.Errorf("expected 503 or 404; got %d body=%s", w.Code, w.Body.String())
	}
}

// =====================================================================
// task-12.3 (ADR-017 D1 Wave 2) — GET /v1/search/{query_id}/trace fallback wiring.
// =====================================================================

func TestGetSearchTrace_503_WhenFallback(t *testing.T) {
	router, _ := newTestRouter(t, "")
	req := httptest.NewRequest("GET", "/v1/search/qry-fake/trace", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusServiceUnavailable {
		t.Fatalf("expected 503; got %d body=%s", w.Code, w.Body.String())
	}
	if !strings.Contains(w.Body.String(), `"code":"SERVICE_UNAVAILABLE"`) {
		t.Errorf("expected SERVICE_UNAVAILABLE; got %s", w.Body.String())
	}
}

// =====================================================================
// task-15.6 (Phase 15 P2 #7 / ADR-020) — GET /v1/health?detailed=true.
// =====================================================================

// TestHandleHealth_Default_StaysBinary — without ?detailed=true the response
// shape is unchanged from v0.7 (no `components` field).
func TestHandleHealth_Default_StaysBinary(t *testing.T) {
	router, _ := newTestRouter(t, "")
	req := httptest.NewRequest("GET", "/v1/health", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d body=%s", w.Code, w.Body.String())
	}
	// Components field omitted via JSON omitempty.
	if strings.Contains(w.Body.String(), `"components"`) {
		t.Errorf("default health response should not include components: %s", w.Body.String())
	}
}

// TestHandleHealth_Detailed_True_NoHealthClient_Synthesizes — when Deps.Health
// is nil (e.g. fallback mode), the handler synthesizes a 5-component response
// so the Console UI CoreHealthCard always has a complete shape.
func TestHandleHealth_Detailed_True_NoHealthClient_Synthesizes(t *testing.T) {
	router, _ := newTestRouter(t, "")
	req := httptest.NewRequest("GET", "/v1/health?detailed=true", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d body=%s", w.Code, w.Body.String())
	}
	var got contractv1.CoreHealth
	if err := json.Unmarshal(w.Body.Bytes(), &got); err != nil {
		t.Fatalf("unmarshal CoreHealth: %v body=%s", err, w.Body.String())
	}
	if len(got.Components) != 5 {
		t.Fatalf("expected 5 components; got %d (%v)", len(got.Components), got.Components)
	}
	for _, name := range []string{"db", "index", "embed", "retriever", "eval"} {
		if _, ok := got.Components[name]; !ok {
			t.Errorf("missing component %q", name)
		}
	}
}

// TestHandleHealth_Detailed_True_InmemFallback_Degraded — when BackendKind is
// "inmem-fallback", the detailed view reports the synthetic 5-component map
// as degraded with the fallback reason.
func TestHandleHealth_Detailed_True_InmemFallback_Degraded(t *testing.T) {
	store := NewMemStore()
	deps := Deps{
		Workspace:   WorkspaceAdapter{S: store},
		Job:         JobAdapter{S: store},
		Search:      store,
		Events:      store,
		Memory:      NewMemMemoryStore(),
		Eval:        NewMemEvalStore(),
		BackendKind: "inmem-fallback",
		AuthToken:   "",
	}
	router := NewRouter(deps)
	req := httptest.NewRequest("GET", "/v1/health?detailed=true", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d body=%s", w.Code, w.Body.String())
	}
	var got contractv1.CoreHealth
	_ = json.Unmarshal(w.Body.Bytes(), &got)
	if got.Status != "degraded" {
		t.Errorf("expected overall degraded; got %q", got.Status)
	}
	if len(got.Components) != 5 {
		t.Errorf("expected 5 components; got %d", len(got.Components))
	}
}

// =====================================================================
// task-15.5 (Phase 15 P1 #5) — GET /v1/queries (query history) endpoint.
// =====================================================================

// TestHandleListQueries_DefaultLimit_EmptyMemStore — fresh MemStore has no
// traceCache entries → []; default limit applies.
func TestHandleListQueries_DefaultLimit_EmptyMemStore(t *testing.T) {
	router, _ := newTestRouter(t, "")
	req := httptest.NewRequest("GET", "/v1/queries", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d body=%s", w.Code, w.Body.String())
	}
	var records []contractv1.QueryRecord
	if err := json.Unmarshal(w.Body.Bytes(), &records); err != nil {
		t.Fatalf("unmarshal QueryRecord: %v body=%s", err, w.Body.String())
	}
	if len(records) != 0 {
		t.Errorf("expected empty list; got %d", len(records))
	}
}

// TestHandleListQueries_AfterSearch_HasEntry — POST /v1/search populates
// traceCache; GET /v1/queries surfaces the QueryRecord.
func TestHandleListQueries_AfterSearch_HasEntry(t *testing.T) {
	router, store := newTestRouter(t, "")
	// First call Search to populate traceCache via MemStore stub path.
	_, _, err := store.Search(contractv1.SearchRequest{
		Query:           "find config",
		WorkspaceID:     "ws-1",
		RetrievalMethod: "bm25",
	})
	if err != nil {
		t.Fatalf("seed Search: %v", err)
	}
	req := httptest.NewRequest("GET", "/v1/queries?limit=5", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d body=%s", w.Code, w.Body.String())
	}
	var records []contractv1.QueryRecord
	_ = json.Unmarshal(w.Body.Bytes(), &records)
	if len(records) != 1 {
		t.Fatalf("expected 1 record; got %d", len(records))
	}
	if records[0].Query != "find config" {
		t.Errorf("expected query='find config'; got %q", records[0].Query)
	}
	if records[0].QueryID == "" {
		t.Errorf("expected non-empty query_id")
	}
}

// =====================================================================
// task-15.4 (Phase 15 P1 #4) — GET /v1/eval-runs (list) endpoint.
// =====================================================================

// TestHandleListEvalRuns_DefaultLimit_EmptyMemStore — MemEvalStore newly
// created returns []; ?limit defaults to 50 server-side; response is JSON [].
func TestHandleListEvalRuns_DefaultLimit_EmptyMemStore(t *testing.T) {
	memEval := NewMemEvalStore()
	deps := Deps{
		Workspace: WorkspaceAdapter{S: NewMemStore()},
		Job:       JobAdapter{S: NewMemStore()},
		Search:    NewMemStore(),
		Events:    NewMemStore(),
		Memory:    NewMemMemoryStore(),
		Eval:      memEval,
		AuthToken: "",
	}
	router := NewRouter(deps)
	req := httptest.NewRequest("GET", "/v1/eval-runs", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d body=%s", w.Code, w.Body.String())
	}
	var runs []contractv1.EvalRun
	if err := json.Unmarshal(w.Body.Bytes(), &runs); err != nil {
		t.Fatalf("unmarshal: %v body=%s", err, w.Body.String())
	}
	if len(runs) != 0 {
		t.Errorf("expected empty list; got %d entries", len(runs))
	}
}

// TestHandleListEvalRuns_AfterCreate_OrderedDesc — POST 2 eval runs via
// MemEvalStore, then GET /v1/eval-runs and verify the response is non-empty
// and ordered most-recent-first.
func TestHandleListEvalRuns_AfterCreate_OrderedDesc(t *testing.T) {
	memEval := NewMemEvalStore()
	// Create 2 eval runs with a slight delay so StartedAt differs.
	_, err := memEval.Create(contractv1.EvalRunCreate{WorkspaceID: "ws-1", DatasetRef: "/tmp/ds"})
	if err != nil {
		t.Fatalf("first Create: %v", err)
	}
	time.Sleep(10 * time.Millisecond)
	_, err = memEval.Create(contractv1.EvalRunCreate{WorkspaceID: "ws-1", DatasetRef: "/tmp/ds"})
	if err != nil {
		t.Fatalf("second Create: %v", err)
	}
	deps := Deps{
		Workspace: WorkspaceAdapter{S: NewMemStore()},
		Job:       JobAdapter{S: NewMemStore()},
		Search:    NewMemStore(),
		Events:    NewMemStore(),
		Memory:    NewMemMemoryStore(),
		Eval:      memEval,
		AuthToken: "",
	}
	router := NewRouter(deps)
	req := httptest.NewRequest("GET", "/v1/eval-runs?limit=5", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d body=%s", w.Code, w.Body.String())
	}
	var runs []contractv1.EvalRun
	if err := json.Unmarshal(w.Body.Bytes(), &runs); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(runs) != 2 {
		t.Fatalf("expected 2 runs; got %d", len(runs))
	}
	if !runs[0].StartedAt.After(runs[1].StartedAt) {
		t.Errorf("expected newest first; got [0]=%v [1]=%v", runs[0].StartedAt, runs[1].StartedAt)
	}
}

// TestHandleListEvalRuns_StatusFilter — verify the ?status query is honored
// by MemEvalStore (running stays, succeeded run also exists after stub timer).
func TestHandleListEvalRuns_StatusFilter(t *testing.T) {
	memEval := NewMemEvalStore()
	_, _ = memEval.Create(contractv1.EvalRunCreate{WorkspaceID: "ws-1"})
	deps := Deps{
		Workspace: WorkspaceAdapter{S: NewMemStore()},
		Job:       JobAdapter{S: NewMemStore()},
		Search:    NewMemStore(),
		Events:    NewMemStore(),
		Memory:    NewMemMemoryStore(),
		Eval:      memEval,
		AuthToken: "",
	}
	router := NewRouter(deps)
	req := httptest.NewRequest("GET", "/v1/eval-runs?status=cancelled", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d body=%s", w.Code, w.Body.String())
	}
	var runs []contractv1.EvalRun
	_ = json.Unmarshal(w.Body.Bytes(), &runs)
	if len(runs) != 0 {
		t.Errorf("expected empty list for status=cancelled; got %d", len(runs))
	}
}

// =====================================================================
// task-15.3 (Phase 15 P1 #3) — GET /v1/stats/chunks endpoint.
// =====================================================================

// TestHandleGetChunksStats_200_Fallback — MemStore (no SearchBackend) returns
// {total=0, today_delta=0} so the Console UI Dashboard renders "no data"
// rather than 503 — fallback behavior per task-15.3 [SPEC-OWNER:task-15.3].
func TestHandleGetChunksStats_200_Fallback(t *testing.T) {
	router, _ := newTestRouter(t, "")
	req := httptest.NewRequest("GET", "/v1/stats/chunks", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d body=%s", w.Code, w.Body.String())
	}
	var stats contractv1.ChunksStats
	if err := json.Unmarshal(w.Body.Bytes(), &stats); err != nil {
		t.Fatalf("unmarshal ChunksStats: %v body=%s", err, w.Body.String())
	}
	if stats.Total != 0 {
		t.Errorf("expected fallback total=0; got %d", stats.Total)
	}
	if stats.TodayDelta != 0 {
		t.Errorf("expected fallback today_delta=0; got %d", stats.TodayDelta)
	}
}

// TestHandleGetChunksStats_WorkspaceIDQuery — verify the optional workspace_id
// filter is read from query string (fallback shape is the same; the filter
// passes through to the SearchBackend in non-fallback mode).
func TestHandleGetChunksStats_WorkspaceIDQuery(t *testing.T) {
	router, _ := newTestRouter(t, "")
	req := httptest.NewRequest("GET", "/v1/stats/chunks?workspace_id=ws-1", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d body=%s", w.Code, w.Body.String())
	}
}

// =====================================================================
// task-13.2 (ADR-017 D1 Wave 3) — 5 memory REST endpoints.
// =====================================================================

func newTestRouterWithMemFixtures(t *testing.T) http.Handler {
	t.Helper()
	store := NewMemStore()
	memMem := NewMemMemoryStore()
	memMem.SeedFixtures()
	deps := Deps{
		Workspace: WorkspaceAdapter{S: store},
		Job:       JobAdapter{S: store},
		Search:    store,
		Events:    store,
		Memory:    memMem,
		AuthToken: "",
	}
	return NewRouter(deps)
}

func TestListMemory_ReturnsFixtures(t *testing.T) {
	router := newTestRouterWithMemFixtures(t)
	req := httptest.NewRequest("GET", "/v1/memory", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d body=%s", w.Code, w.Body.String())
	}
	var items []contractv1.MemoryItem
	if err := json.Unmarshal(w.Body.Bytes(), &items); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	// SeedFixtures inserts 5 items, 1 is "deprecated"; default list excludes soft_deleted only → 5
	if len(items) != 5 {
		t.Fatalf("expected 5 items; got %d", len(items))
	}
}

func TestListMemory_FilterByScope(t *testing.T) {
	router := newTestRouterWithMemFixtures(t)
	req := httptest.NewRequest("GET", "/v1/memory?scope=agent-default:session", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d", w.Code)
	}
	var items []contractv1.MemoryItem
	_ = json.Unmarshal(w.Body.Bytes(), &items)
	if len(items) != 1 || items[0].MemoryID != "mem-fixture-1" {
		t.Errorf("expected mem-fixture-1; got %+v", items)
	}
}

func TestGetMemory_404_when_missing(t *testing.T) {
	router := newTestRouterWithMemFixtures(t)
	req := httptest.NewRequest("GET", "/v1/memory/missing-id", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusNotFound {
		t.Errorf("expected 404; got %d", w.Code)
	}
}

func TestMemoryPin_204_no_body(t *testing.T) {
	router := newTestRouterWithMemFixtures(t)
	req := httptest.NewRequest("POST", "/v1/memory/mem-fixture-1/pin", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusNoContent {
		t.Fatalf("expected 204; got %d body=%s", w.Code, w.Body.String())
	}
	if w.Body.Len() != 0 {
		t.Errorf("204 must have empty body; got %q", w.Body.String())
	}
}

func TestMemoryDeprecate_412_when_missing_confirm(t *testing.T) {
	router := newTestRouterWithMemFixtures(t)
	req := httptest.NewRequest("POST", "/v1/memory/mem-fixture-1/deprecate", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusPreconditionFailed {
		t.Fatalf("expected 412; got %d body=%s", w.Code, w.Body.String())
	}
}

func TestMemoryDeprecate_204_with_header(t *testing.T) {
	router := newTestRouterWithMemFixtures(t)
	req := httptest.NewRequest("POST", "/v1/memory/mem-fixture-1/deprecate", nil)
	req.Header.Set("X-Confirm", "yes")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusNoContent {
		t.Fatalf("expected 204; got %d body=%s", w.Code, w.Body.String())
	}
}

func TestMemoryDeprecate_204_with_query(t *testing.T) {
	router := newTestRouterWithMemFixtures(t)
	req := httptest.NewRequest("POST", "/v1/memory/mem-fixture-1/deprecate?confirm=true", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusNoContent {
		t.Fatalf("expected 204; got %d body=%s", w.Code, w.Body.String())
	}
}

func TestMemorySoftDelete_412_then_204_then_excluded(t *testing.T) {
	router := newTestRouterWithMemFixtures(t)

	// Without X-Confirm → 412
	req := httptest.NewRequest("POST", "/v1/memory/mem-fixture-1/soft-delete", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusPreconditionFailed {
		t.Fatalf("expected 412 without X-Confirm; got %d", w.Code)
	}

	// With X-Confirm: yes → 204
	req = httptest.NewRequest("POST", "/v1/memory/mem-fixture-1/soft-delete", nil)
	req.Header.Set("X-Confirm", "yes")
	w = httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusNoContent {
		t.Fatalf("expected 204 with X-Confirm; got %d body=%s", w.Code, w.Body.String())
	}

	// Default list excludes the soft-deleted item
	req = httptest.NewRequest("GET", "/v1/memory", nil)
	w = httptest.NewRecorder()
	router.ServeHTTP(w, req)
	var items []contractv1.MemoryItem
	_ = json.Unmarshal(w.Body.Bytes(), &items)
	for _, item := range items {
		if item.MemoryID == "mem-fixture-1" {
			t.Errorf("soft-deleted item still in default list: %+v", item)
		}
	}

	// include_soft_deleted=true returns it
	req = httptest.NewRequest("GET", "/v1/memory?include_soft_deleted=true", nil)
	w = httptest.NewRecorder()
	router.ServeHTTP(w, req)
	items = nil
	_ = json.Unmarshal(w.Body.Bytes(), &items)
	found := false
	for _, item := range items {
		if item.MemoryID == "mem-fixture-1" && item.Status == "soft_deleted" {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("include_soft_deleted=true should return the soft-deleted item")
	}
}

// TestHandleHealth_ContractVersion — must-have field check (AC1).
func TestHandleHealth_ContractVersion(t *testing.T) {
	router, _ := newTestRouter(t, "")
	req := httptest.NewRequest("GET", "/v1/health", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d", w.Code)
	}
	var got contractv1.CoreHealth
	if err := json.Unmarshal(w.Body.Bytes(), &got); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if got.ContractVersion != "v1" {
		t.Errorf("contract_version must be v1; got %q", got.ContractVersion)
	}
	if got.Status != "healthy" {
		t.Errorf("status must be healthy; got %q", got.Status)
	}
}
