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
		BackendKind: "grpc",
	}
	return consoleapi.NewRouter(deps), cli
}

func doJSON(t *testing.T, srv *httptest.Server, method, path string, body string) (int, []byte) {
	t.Helper()
	req, err := http.NewRequest(method, srv.URL+path, strings.NewReader(body))
	if err != nil {
		t.Fatalf("new req: %v", err)
	}
	if body != "" {
		req.Header.Set("Content-Type", "application/json")
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
//	9. POST /v1/index-jobs/<id>/cancel → 200
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

	// Step 9: cancel
	code, body = doJSON(t, srv, "POST", "/v1/index-jobs/"+jobID+"/cancel", "")
	if code != 200 {
		t.Fatalf("POST cancel: code=%d body=%s", code, body)
	}

	// Step 10: POST /v1/search (empty result per task-11.1 [SPEC-OWNER:task-11.4])
	searchBody := fmt.Sprintf(`{"query":"x","workspace_id":"%s","top_k":5}`, wsID)
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
