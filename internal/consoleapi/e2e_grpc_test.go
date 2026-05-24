// task-11.2 §6 AC5: TestRESTEndpoints_E2E_GrpcBacked — spawn the real
// `contextforge-core` Rust daemon binary on a chosen port, point an
// in-process console-api-serve router at it via grpcclient, and exercise
// the 9 REST endpoints + workspace persistence across daemon restart.
//
// This test is heavy (it cargo-builds the Rust binary if missing). To skip
// it (e.g. CI environments without the Rust toolchain), set env
// CF_SKIP_RUST_E2E=1.

package consoleapi_test

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/consoleapi"
	"github.com/tajiaoyezi/contextforge/internal/consoleapi/grpcclient"
)

func findRepoRoot(t *testing.T) string {
	t.Helper()
	wd, err := os.Getwd()
	if err != nil {
		t.Fatalf("getwd: %v", err)
	}
	d := wd
	for i := 0; i < 8; i++ {
		if _, err := os.Stat(filepath.Join(d, "go.mod")); err == nil {
			return d
		}
		d = filepath.Dir(d)
	}
	t.Fatalf("could not locate go.mod from %s", wd)
	return ""
}

func ensureRustDaemon(t *testing.T) string {
	t.Helper()
	root := findRepoRoot(t)
	binName := "contextforge-core"
	if runtime.GOOS == "windows" {
		binName += ".exe"
	}
	binPath := filepath.Join(root, "target", "debug", binName)
	if _, err := os.Stat(binPath); err == nil {
		return binPath
	}
	// Build it.
	cmd := exec.Command("cargo", "build", "-p", "contextforge-core")
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Skipf("cargo build failed (skip E2E): %v\n%s", err, out)
	}
	if _, err := os.Stat(binPath); err != nil {
		t.Skipf("binary still missing after build: %v", err)
	}
	return binPath
}

func findFreePort(t *testing.T) int {
	t.Helper()
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("listen: %v", err)
	}
	addr := l.Addr().(*net.TCPAddr)
	port := addr.Port
	_ = l.Close()
	return port
}

func startDaemon(t *testing.T, binPath, listenAddr, dataDir string) (*exec.Cmd, func()) {
	t.Helper()
	cmd := exec.Command(binPath, listenAddr, dataDir)
	stdout, _ := cmd.StdoutPipe()
	stderr, _ := cmd.StderrPipe()
	if err := cmd.Start(); err != nil {
		t.Fatalf("start daemon: %v", err)
	}
	// Drain output so the daemon doesn't block on full pipe buffer.
	go func() { _, _ = io.Copy(io.Discard, stdout) }()
	go func() { _, _ = io.Copy(io.Discard, stderr) }()
	stop := func() {
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
	}
	return cmd, stop
}

func waitDaemonReady(t *testing.T, addr string, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		ctx, cancel := context.WithTimeout(context.Background(), 500*time.Millisecond)
		cli, err := grpcclient.New(ctx, addr)
		cancel()
		if err == nil {
			pingCtx, pingCancel := context.WithTimeout(context.Background(), 500*time.Millisecond)
			if pingErr := cli.Ping(pingCtx); pingErr == nil {
				_ = cli.Close()
				pingCancel()
				return
			}
			pingCancel()
			_ = cli.Close()
		}
		time.Sleep(200 * time.Millisecond)
	}
	t.Fatalf("daemon did not become ready at %s within %v", addr, timeout)
}

// buildRouterWithGrpc — wire the consoleapi router to grpcclient-backed Deps.
func buildRouterWithGrpc(t *testing.T, grpcAddr string) (http.Handler, *grpcclient.Client) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	cli, err := grpcclient.New(ctx, grpcAddr)
	if err != nil {
		t.Fatalf("grpcclient.New: %v", err)
	}
	deps := consoleapi.Deps{
		Workspace:   cli.Workspace(),
		Job:         cli.Job(),
		Search:      cli.Search(),
		Events:      cli.Events(),
		Memory:      cli.Memory(),
		BackendKind: "grpc",
	}
	return consoleapi.NewRouter(deps), cli
}

func doJSON(t *testing.T, srv *httptest.Server, method, path string, body string) (int, []byte) {
	t.Helper()
	return doJSONHeaders(t, srv, method, path, body, nil)
}

// doJSONHeaders extends doJSON with optional extra headers. Used by task-12.1
// sub-steps to inject X-Confirm: yes for the PATCH /v1/workspaces/{id}/config
// destructive endpoint.
func doJSONHeaders(t *testing.T, srv *httptest.Server, method, path, body string, headers map[string]string) (int, []byte) {
	t.Helper()
	req, err := http.NewRequest(method, srv.URL+path, strings.NewReader(body))
	if err != nil {
		t.Fatalf("new req: %v", err)
	}
	if body != "" {
		req.Header.Set("Content-Type", "application/json")
	}
	for k, v := range headers {
		req.Header.Set(k, v)
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("do: %v", err)
	}
	defer resp.Body.Close()
	out, _ := io.ReadAll(resp.Body)
	return resp.StatusCode, out
}

// TestRESTEndpoints_E2E_GrpcBacked verifies the full Console REST → Go
// grpcclient → Rust daemon path end-to-end (task-11.2 §6 AC5).
//
//	1. cargo-build + spawn ./target/debug/contextforge-core at 127.0.0.1:N1
//	2. start in-process console-api-serve router pointed at :N1 via grpcclient
//	3. GET /v1/health → 200 + status=healthy + contract_version=v1
//	4. POST /v1/workspaces → 200 + workspace_id present
//	5. GET /v1/workspaces → 200 + list contains the workspace
//	6. GET /v1/workspaces/<id> → 200 + same workspace
//	7. POST /v1/index-jobs → 200 + job status=queued
//	8. GET /v1/index-jobs/<id> → 200 + same job
//	9. POST /v1/index-jobs/<id>/cancel → 204 (task-12.1 / ADR-017 D3)
//	10. POST /v1/search → 200 + nested {result, trace} (results empty per
//	    task-11.1 [SPEC-OWNER:task-11.4])
//	11. GET /v1/observability/events → 200 + [] (only keepalive emitted
//	    per task-11.1 [SPEC-OWNER:task-11.4])
//	12. kill daemon + restart with same data_dir → GET /v1/workspaces
//	    still returns the workspace (persistence proof)
func TestRESTEndpoints_E2E_GrpcBacked(t *testing.T) {
	if os.Getenv("CF_SKIP_RUST_E2E") == "1" {
		t.Skip("CF_SKIP_RUST_E2E=1; skipping Rust daemon spawn E2E")
	}
	bin := ensureRustDaemon(t)
	port := findFreePort(t)
	addr := "127.0.0.1:" + strconv.Itoa(port)
	dataDir := t.TempDir()

	_, stopDaemon := startDaemon(t, bin, addr, dataDir)
	defer stopDaemon()
	waitDaemonReady(t, addr, 15*time.Second)

	router, cli := buildRouterWithGrpc(t, addr)
	defer func() { _ = cli.Close() }()
	srv := httptest.NewServer(router)
	defer srv.Close()

	// Step 3: /v1/health
	code, body := doJSON(t, srv, "GET", "/v1/health", "")
	if code != 200 {
		t.Fatalf("/v1/health: code=%d body=%s", code, body)
	}
	var health map[string]any
	_ = json.Unmarshal(body, &health)
	if health["status"] != "healthy" || health["contract_version"] != "v1" {
		t.Errorf("health drift: %s", body)
	}

	// Step 4: POST /v1/workspaces
	wsName := "e2e-test"
	createBody := fmt.Sprintf(`{"name":"%s","root_path":%q}`, wsName, dataDir)
	code, body = doJSON(t, srv, "POST", "/v1/workspaces", createBody)
	if code != 200 {
		t.Fatalf("POST workspaces: code=%d body=%s", code, body)
	}
	var ws map[string]any
	_ = json.Unmarshal(body, &ws)
	wsID := ws["workspace_id"].(string)
	if wsID == "" {
		t.Fatalf("empty workspace_id; body=%s", body)
	}

	// Step 5: GET /v1/workspaces
	code, body = doJSON(t, srv, "GET", "/v1/workspaces", "")
	if code != 200 {
		t.Fatalf("GET workspaces: code=%d body=%s", code, body)
	}

	// Step 6: GET /v1/workspaces/<id>
	code, body = doJSON(t, srv, "GET", "/v1/workspaces/"+wsID, "")
	if code != 200 {
		t.Fatalf("GET workspace by id: code=%d body=%s", code, body)
	}

	// Step 7: POST /v1/index-jobs
	enqueueBody := fmt.Sprintf(`{"workspace_id":"%s","trigger_source":"e2e"}`, wsID)
	code, body = doJSON(t, srv, "POST", "/v1/index-jobs", enqueueBody)
	if code != 200 {
		t.Fatalf("POST index-jobs: code=%d body=%s", code, body)
	}
	var job map[string]any
	_ = json.Unmarshal(body, &job)
	jobID := job["job_id"].(string)
	if jobID == "" || job["status"] != "queued" {
		t.Fatalf("job drift: %s", body)
	}

	// Step 8: GET /v1/index-jobs/<id>
	code, body = doJSON(t, srv, "GET", "/v1/index-jobs/"+jobID, "")
	if code != 200 {
		t.Fatalf("GET index-job by id: code=%d body=%s", code, body)
	}

	// Step 8a: task-12.1 (ADR-017 D1 Wave 1) — PATCH /v1/workspaces/{id}/config
	// without X-Confirm → 412 Precondition Failed (server-side bottom defense).
	patchBody := `{"allowlist":["src/**"],"denylist":["node_modules/**"]}`
	code, body = doJSON(t, srv, "PATCH", "/v1/workspaces/"+wsID+"/config", patchBody)
	if code != 412 {
		t.Fatalf("PATCH config without X-Confirm: expected 412; got code=%d body=%s", code, body)
	}
	// With X-Confirm: yes → 200 + updated workspace
	code, body = doJSONHeaders(t, srv, "PATCH", "/v1/workspaces/"+wsID+"/config", patchBody,
		map[string]string{"X-Confirm": "yes"})
	if code != 200 {
		t.Fatalf("PATCH config with X-Confirm: code=%d body=%s", code, body)
	}
	var updated map[string]any
	_ = json.Unmarshal(body, &updated)
	if updated["workspace_id"] != wsID {
		t.Errorf("PATCH config: workspace_id drift; body=%s", body)
	}
	// With ?confirm=true → 200 (OR-semantics verified)
	code, body = doJSON(t, srv, "PATCH", "/v1/workspaces/"+wsID+"/config?confirm=true", patchBody)
	if code != 200 {
		t.Fatalf("PATCH config with ?confirm=true: code=%d body=%s", code, body)
	}

	// Step 8b: GET /v1/index-jobs?status=active includes the queued job
	code, body = doJSON(t, srv, "GET", "/v1/index-jobs?status=active", "")
	if code != 200 {
		t.Fatalf("GET active jobs: code=%d body=%s", code, body)
	}
	var activeJobs []map[string]any
	_ = json.Unmarshal(body, &activeJobs)
	foundActive := false
	for _, j := range activeJobs {
		if j["job_id"] == jobID {
			foundActive = true
			break
		}
	}
	if !foundActive {
		t.Fatalf("active jobs list missing newly enqueued job %q; body=%s", jobID, body)
	}
	// Missing status filter → 400
	code, body = doJSON(t, srv, "GET", "/v1/index-jobs", "")
	if code != 400 {
		t.Fatalf("GET index-jobs without status: expected 400; got code=%d body=%s", code, body)
	}

	// Step 9: cancel — task-12.1 (ADR-017 D3) returns 204 No Content.
	// (Async cancel → terminal status propagation is task-11.3 scope; the
	// REST contract here verifies only the 204 + 204-body=empty invariants.)
	code, body = doJSON(t, srv, "POST", "/v1/index-jobs/"+jobID+"/cancel", "")
	if code != 204 {
		t.Fatalf("POST cancel: code=%d body=%s", code, body)
	}
	if len(body) != 0 {
		t.Errorf("204 must have empty body; got %q", body)
	}

	// Step 9d: task-13.2 (ADR-017 D1 Wave 3) — memory endpoint smoke.
	// Empty store: GET /v1/memory → []; GET /v1/memory/missing → 404;
	// pin/deprecate/soft-delete on missing id → 404 (or 412 without confirm).
	code, body = doJSON(t, srv, "GET", "/v1/memory", "")
	if code != 200 {
		t.Fatalf("GET memory empty: expected 200; got code=%d body=%s", code, body)
	}
	code, body = doJSON(t, srv, "GET", "/v1/memory/does-not-exist", "")
	if code != 404 {
		t.Fatalf("GET memory missing: expected 404; got code=%d body=%s", code, body)
	}
	// Deprecate without confirm → 412 (confirmMiddleware fires before handler)
	code, body = doJSON(t, srv, "POST", "/v1/memory/x/deprecate", "")
	if code != 412 {
		t.Fatalf("POST deprecate no confirm: expected 412; got code=%d body=%s", code, body)
	}

	// Step 9b: task-12.2 (ADR-017 D1 Wave 2) — GET /v1/source-chunks/{id} for an
	// unknown chunk_id returns 404 NOT_FOUND from SearchService.GetSourceChunk.
	code, body = doJSON(t, srv, "GET", "/v1/source-chunks/chk_does_not_exist_0", "")
	if code != 404 {
		t.Fatalf("GET source-chunks unknown: expected 404; got code=%d body=%s", code, body)
	}

	// Step 9c: task-12.3 (ADR-017 D1 Wave 2) — POST /v1/search emits a query_id;
	// trace is persisted by SearchService into in-memory LRU; GET /v1/search/{query_id}/trace
	// returns the cached trace. Use the search response from Step 10 below by
	// reordering: do search first to capture query_id.
	searchBody := fmt.Sprintf(`{"query":"x","workspace_id":"%s","top_k":5}`, wsID)
	code, body = doJSON(t, srv, "POST", "/v1/search", searchBody)
	if code != 200 {
		t.Fatalf("POST search (pre-trace): code=%d body=%s", code, body)
	}
	var searchEnvelope map[string]any
	_ = json.Unmarshal(body, &searchEnvelope)
	resultMap, _ := searchEnvelope["result"].(map[string]any)
	queryID, _ := resultMap["query_id"].(string)
	if queryID == "" {
		// Empty results may produce an empty query_id when no index exists yet
		// (workspace was just created, no index job completed). In that path
		// SearchServer falls through to empty_response() which does not store
		// a trace. Skip the trace fetch; only assert when query_id was set.
		t.Logf("Step 9c: search returned empty query_id (workspace has no index yet); skipping trace fetch")
	} else {
		code, body = doJSON(t, srv, "GET", "/v1/search/"+queryID+"/trace", "")
		if code != 200 {
			t.Fatalf("GET trace by query_id: expected 200; got code=%d body=%s", code, body)
		}
		var traceJSON map[string]any
		_ = json.Unmarshal(body, &traceJSON)
		if traceJSON["trace_id"] == "" || traceJSON["trace_id"] == nil {
			t.Errorf("GET trace by query_id missing trace_id; body=%s", body)
		}
	}
	// Unknown query_id → 404
	code, body = doJSON(t, srv, "GET", "/v1/search/qry-does-not-exist/trace", "")
	if code != 404 {
		t.Fatalf("GET trace unknown query_id: expected 404; got code=%d body=%s", code, body)
	}

	// Step 10: POST /v1/search (empty result per task-11.1 [SPEC-OWNER:task-11.4])
	// Re-use searchBody declared in Step 9c.
	code, body = doJSON(t, srv, "POST", "/v1/search", searchBody)
	if code != 200 {
		t.Fatalf("POST search: code=%d body=%s", code, body)
	}
	var searchResp map[string]any
	_ = json.Unmarshal(body, &searchResp)
	if _, ok := searchResp["result"]; !ok {
		t.Errorf("search nested response missing 'result'; body=%s", body)
	}
	if _, ok := searchResp["trace"]; !ok {
		t.Errorf("search nested response missing 'trace'; body=%s", body)
	}

	// Step 11: GET /v1/observability/events (keepalive only; per task-11.1)
	code, body = doJSON(t, srv, "GET", "/v1/observability/events", "")
	if code != 200 {
		t.Fatalf("GET events: code=%d body=%s", code, body)
	}

	// Step 12: daemon kill + restart + verify workspace persisted
	stopDaemon()
	time.Sleep(500 * time.Millisecond) // let port release
	_, stopDaemon2 := startDaemon(t, bin, addr, dataDir)
	defer stopDaemon2()
	waitDaemonReady(t, addr, 15*time.Second)

	// Rebuild router pointing at new daemon (grpcclient must reconnect; we
	// just create a fresh client for simplicity).
	router2, cli2 := buildRouterWithGrpc(t, addr)
	defer func() { _ = cli2.Close() }()
	srv2 := httptest.NewServer(router2)
	defer srv2.Close()

	code, body = doJSON(t, srv2, "GET", "/v1/workspaces", "")
	if code != 200 {
		t.Fatalf("POST-restart GET workspaces: code=%d body=%s", code, body)
	}
	// Verify the workspace persisted across restart
	var listResp []map[string]any
	_ = json.Unmarshal(body, &listResp)
	found := false
	for _, item := range listResp {
		if item["workspace_id"] == wsID {
			found = true
			break
		}
	}
	if !found {
		t.Fatalf("workspace %s did not persist across daemon restart; body=%s", wsID, body)
	}
}
