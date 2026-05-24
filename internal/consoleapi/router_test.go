package consoleapi

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

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
		{"cancel_job", "POST", "/v1/index-jobs/" + job.JobID + "/cancel", nil, http.StatusOK},
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
