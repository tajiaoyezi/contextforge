// task-6.2: REST API server RED 测试集 (TEST-6.2.1 / 2 / 5).
//
// 设计：用 newRESTHandler 内部 testable factory 注入 fakeSearcher，跳过
// 真实 daemon spawn (cargo build) + 真实 gRPC transport。AC1/AC2 验响应契约 +
// gRPC Status → HTTP 映射；AC5 验 401 + audit JSON-lines 文件写入。AC3
// (ensureLoopback) 在 internal/cli/serve_test.go 验（addr 解析层）；AC4
// (token 0600) 同样在 serve_test.go。

package daemon_test

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/tajiaoyezi/contextforge/internal/daemon"
	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

// fakeSearcher 实现 daemon.RESTSearcher 接口的测试 fake；用于 newRESTHandler 注入.
type fakeSearcher struct {
	resp *contextforgev1.SearchResponse
	err  error
}

func (f *fakeSearcher) Search(_ context.Context, _ *contextforgev1.SearchRequest) (*contextforgev1.SearchResponse, error) {
	return f.resp, f.err
}

// fakeSearchResponse — 12 字段齐 + provenance ≥1 的合成 SearchResponse fixture.
func fakeSearchResponse(chunkID string) *contextforgev1.SearchResponse {
	return &contextforgev1.SearchResponse{
		Results: []*contextforgev1.RetrievalResult{
			{
				ChunkId:         chunkID,
				FilePath:        "fixtures/readme.md",
				LineStart:       1,
				LineEnd:         5,
				Score:           0.93,
				RetrievalMethod: "bm25",
				Reason:          "bm25 hit",
				RedactionStatus: "applied",
				Provenance: []*contextforgev1.Provenance{
					{Importer: "scanner", OriginalPath: "fixtures/readme.md"},
				},
			},
		},
	}
}

// emptySearchResponse — 模拟 chunks/{id} 未命中（results 空）.
func emptySearchResponse() *contextforgev1.SearchResponse {
	return &contextforgev1.SearchResponse{Results: nil}
}

// capturingSearcher — task-19.3: records the inbound gRPC SearchRequest so a test can assert how
// the REST layer translated query params (e.g. ?semantic=true → req.Semantic).
type capturingSearcher struct {
	resp     *contextforgev1.SearchResponse
	captured *contextforgev1.SearchRequest
}

func (c *capturingSearcher) Search(_ context.Context, req *contextforgev1.SearchRequest) (*contextforgev1.SearchResponse, error) {
	c.captured = req
	return c.resp, nil
}

// TEST-19.3 — POST /v1/search?semantic=true sets req.Semantic on the forwarded gRPC request;
// absence of the param leaves it false.
func TestTask193_SemanticQueryParamSetsFlag(t *testing.T) {
	const token = "test-token-abc123"
	do := func(url string) *contextforgev1.SearchRequest {
		s := &capturingSearcher{resp: fakeSearchResponse("c1")}
		server := httptest.NewServer(daemon.NewRESTHandler(s, token, t.TempDir()))
		defer server.Close()
		body := bytes.NewBufferString(`{"query":"foo","collections":["c1"]}`)
		req, _ := http.NewRequest("POST", server.URL+url, body)
		req.Header.Set("Authorization", "Bearer "+token)
		req.Header.Set("Content-Type", "application/json")
		resp, err := http.DefaultClient.Do(req)
		if err != nil {
			t.Fatalf("http: %v", err)
		}
		resp.Body.Close()
		return s.captured
	}

	if got := do("/v1/search?semantic=true"); got == nil || !got.GetSemantic() {
		t.Fatalf("?semantic=true should set req.Semantic=true, got %+v", got)
	}
	if got := do("/v1/search"); got == nil || got.GetSemantic() {
		t.Fatalf("no param → req.Semantic=false, got %+v", got)
	}
}

// TEST-6.2.1 / SCEN-6.2.1 / AC1 — POST /v1/search 契约一致.
func TestTask62_AC1_RESTSearchContract(t *testing.T) {
	const token = "test-token-abc123"
	dataDir := t.TempDir()
	s := &fakeSearcher{resp: fakeSearchResponse("chunk-1")}

	handler := daemon.NewRESTHandler(s, token, dataDir)
	server := httptest.NewServer(handler)
	defer server.Close()

	body := bytes.NewBufferString(`{"query":"foo","collections":["c1"],"top_k":10}`)
	req, _ := http.NewRequest("POST", server.URL+"/v1/search", body)
	req.Header.Set("Authorization", "Bearer "+token)
	req.Header.Set("Content-Type", "application/json")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("AC1: http: %v", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != 200 {
		t.Fatalf("AC1: status=%d want 200", resp.StatusCode)
	}
	if ct := resp.Header.Get("Content-Type"); !strings.Contains(ct, "application/json") {
		t.Fatalf("AC1: Content-Type=%q want application/json", ct)
	}

	var got contextforgev1.SearchResponse
	if err := json.NewDecoder(resp.Body).Decode(&got); err != nil {
		t.Fatalf("AC1: decode: %v", err)
	}
	if len(got.GetResults()) == 0 {
		t.Fatalf("AC1: results empty (fake produced 1)")
	}
	if got.GetResults()[0].GetChunkId() != "chunk-1" {
		t.Fatalf("AC1: chunk_id=%q want chunk-1", got.GetResults()[0].GetChunkId())
	}
	if got.GetResults()[0].GetRedactionStatus() != "applied" {
		t.Fatalf("AC1: redaction_status=%q want applied", got.GetResults()[0].GetRedactionStatus())
	}
}

// TEST-6.2.2 / SCEN-6.2.2 / AC2 — chunks/collections 真返 + import/eval stub 501 + 错误码映射.
func TestTask62_AC2_OtherEndpoints(t *testing.T) {
	const token = "test-token-xyz"

	t.Run("ChunkHit", func(t *testing.T) {
		s := &fakeSearcher{resp: fakeSearchResponse("abc123def456")}
		handler := daemon.NewRESTHandler(s, token, t.TempDir())
		server := httptest.NewServer(handler)
		defer server.Close()
		req, _ := http.NewRequest("GET", server.URL+"/v1/chunks/abc123def456", nil)
		req.Header.Set("Authorization", "Bearer "+token)
		resp, err := http.DefaultClient.Do(req)
		if err != nil {
			t.Fatalf("http: %v", err)
		}
		defer resp.Body.Close()
		if resp.StatusCode != 200 {
			t.Fatalf("AC2 chunk hit: status=%d want 200", resp.StatusCode)
		}
		var rr contextforgev1.RetrievalResult
		if err := json.NewDecoder(resp.Body).Decode(&rr); err != nil {
			t.Fatalf("AC2 chunk hit decode: %v", err)
		}
		if rr.GetChunkId() != "abc123def456" {
			t.Fatalf("AC2 chunk hit: chunk_id=%q", rr.GetChunkId())
		}
	})

	t.Run("ChunkMiss", func(t *testing.T) {
		s := &fakeSearcher{resp: emptySearchResponse()}
		handler := daemon.NewRESTHandler(s, token, t.TempDir())
		server := httptest.NewServer(handler)
		defer server.Close()
		req, _ := http.NewRequest("GET", server.URL+"/v1/chunks/nonexistent", nil)
		req.Header.Set("Authorization", "Bearer "+token)
		resp, err := http.DefaultClient.Do(req)
		if err != nil {
			t.Fatalf("http: %v", err)
		}
		defer resp.Body.Close()
		if resp.StatusCode != 404 {
			t.Fatalf("AC2 chunk miss: status=%d want 404", resp.StatusCode)
		}
	})

	t.Run("Collections", func(t *testing.T) {
		dataDir := t.TempDir()
		// Mkdir tempdir/collections/c1 + tempdir/collections/c2 (each w/ chunks.db file)
		// to simulate 2 indexed collections; chunk_count=0 ok for unit test.
		for _, c := range []string{"c1", "c2"} {
			collDir := filepath.Join(dataDir, "collections", c)
			if err := os.MkdirAll(collDir, 0o755); err != nil {
				t.Fatalf("mkdir collection: %v", err)
			}
			// Touch metadata.sqlite (mirrors task-2.4 layout) — empty file OK for sanity
			if err := os.WriteFile(filepath.Join(collDir, "metadata.sqlite"), []byte{}, 0o600); err != nil {
				t.Fatalf("touch metadata.sqlite: %v", err)
			}
		}
		s := &fakeSearcher{}
		handler := daemon.NewRESTHandler(s, token, dataDir)
		server := httptest.NewServer(handler)
		defer server.Close()
		req, _ := http.NewRequest("GET", server.URL+"/v1/collections", nil)
		req.Header.Set("Authorization", "Bearer "+token)
		resp, err := http.DefaultClient.Do(req)
		if err != nil {
			t.Fatalf("http: %v", err)
		}
		defer resp.Body.Close()
		if resp.StatusCode != 200 {
			t.Fatalf("AC2 collections: status=%d want 200", resp.StatusCode)
		}
		var body struct {
			Collections []struct {
				ID         string `json:"id"`
				ChunkCount int64  `json:"chunk_count"`
			} `json:"collections"`
		}
		if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
			t.Fatalf("AC2 collections decode: %v", err)
		}
		if len(body.Collections) != 2 {
			t.Fatalf("AC2 collections: want 2 entries, got %d (%+v)", len(body.Collections), body.Collections)
		}
	})

	t.Run("ImportStub501", func(t *testing.T) {
		s := &fakeSearcher{}
		handler := daemon.NewRESTHandler(s, token, t.TempDir())
		server := httptest.NewServer(handler)
		defer server.Close()
		req, _ := http.NewRequest("POST", server.URL+"/v1/import", bytes.NewBufferString(`{}`))
		req.Header.Set("Authorization", "Bearer "+token)
		resp, err := http.DefaultClient.Do(req)
		if err != nil {
			t.Fatalf("http: %v", err)
		}
		defer resp.Body.Close()
		if resp.StatusCode != 501 {
			t.Fatalf("AC2 import: status=%d want 501", resp.StatusCode)
		}
		bodyBytes, _ := readAll(resp.Body)
		if !strings.Contains(string(bodyBytes), "deferred to phase 8") {
			t.Fatalf("AC2 import: body missing deferred note: %s", bodyBytes)
		}
	})

	t.Run("EvalStub501", func(t *testing.T) {
		s := &fakeSearcher{}
		handler := daemon.NewRESTHandler(s, token, t.TempDir())
		server := httptest.NewServer(handler)
		defer server.Close()
		req, _ := http.NewRequest("POST", server.URL+"/v1/eval/run", bytes.NewBufferString(`{}`))
		req.Header.Set("Authorization", "Bearer "+token)
		resp, err := http.DefaultClient.Do(req)
		if err != nil {
			t.Fatalf("http: %v", err)
		}
		defer resp.Body.Close()
		if resp.StatusCode != 501 {
			t.Fatalf("AC2 eval: status=%d want 501", resp.StatusCode)
		}
		bodyBytes, _ := readAll(resp.Body)
		if !strings.Contains(string(bodyBytes), "deferred to phase 8") {
			t.Fatalf("AC2 eval: body missing deferred note: %s", bodyBytes)
		}
	})

	t.Run("GRPCStatusMapping", func(t *testing.T) {
		// 直接调 grpcStatusToHTTP（exported-for-test via export_test.go）.
		cases := []struct {
			code     codes.Code
			wantHTTP int
		}{
			{codes.InvalidArgument, 400},
			{codes.FailedPrecondition, 412},
			{codes.NotFound, 404},
			{codes.Unauthenticated, 401},
			{codes.Internal, 500},
			{codes.Unknown, 500},
		}
		for _, tc := range cases {
			err := status.Error(tc.code, "test")
			got := daemon.GRPCStatusToHTTP(err)
			if got != tc.wantHTTP {
				t.Fatalf("AC2 map: code=%s → http=%d, want %d", tc.code, got, tc.wantHTTP)
			}
		}
		// Nil → 200
		if got := daemon.GRPCStatusToHTTP(nil); got != 200 {
			t.Fatalf("AC2 map: nil → %d want 200", got)
		}
	})
}

// TEST-6.2.5 / SCEN-6.2.5 / AC5 — 无 token 401 + audit.
func TestTask62_AC5_MissingTokenReturns401AndAudit(t *testing.T) {
	const token = "test-token-xyz789"
	dataDir := t.TempDir()
	s := &fakeSearcher{resp: fakeSearchResponse("chunk-x")}
	handler := daemon.NewRESTHandler(s, token, dataDir)
	server := httptest.NewServer(handler)
	defer server.Close()

	// No Authorization header → 401
	req, _ := http.NewRequest("POST", server.URL+"/v1/search",
		bytes.NewBufferString(`{"query":"foo","collections":["c1"]}`))
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("AC5 http: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 401 {
		t.Fatalf("AC5: status=%d want 401", resp.StatusCode)
	}
	bodyBytes, _ := readAll(resp.Body)
	if !strings.Contains(string(bodyBytes), "missing or invalid token") {
		t.Fatalf("AC5: body missing expected error: %s", bodyBytes)
	}

	// Audit log file should exist + have ≥1 line; entries must NOT contain token value
	// or full request body (脱敏 per spec §3 + AC5).
	auditPath := filepath.Join(dataDir, "audit-rest.log")
	auditBytes, err := os.ReadFile(auditPath)
	if err != nil {
		t.Fatalf("AC5 audit file: %v (path=%s)", err, auditPath)
	}
	if len(auditBytes) == 0 {
		t.Fatalf("AC5 audit file empty")
	}
	auditStr := string(auditBytes)
	if strings.Contains(auditStr, token) {
		t.Fatalf("AC5 audit leak: 含 token 值 %q in %s", token, auditStr)
	}
	if strings.Contains(auditStr, `"query":"foo"`) || strings.Contains(auditStr, `"foo"`) {
		t.Fatalf("AC5 audit leak: 含请求 body 内容 in %s", auditStr)
	}
	// Sanity: audit 行应是 JSON {"endpoint":"...","status":401,...}
	if !strings.Contains(auditStr, `"status":401`) {
		t.Fatalf("AC5 audit: 缺 status:401 字段 in %s", auditStr)
	}
	if !strings.Contains(auditStr, `"endpoint"`) {
		t.Fatalf("AC5 audit: 缺 endpoint 字段 in %s", auditStr)
	}
	// 有效 token 请求应通过（不被 401）— 反向校验
	req2, _ := http.NewRequest("POST", server.URL+"/v1/search",
		bytes.NewBufferString(`{"query":"foo","collections":["c1"]}`))
	req2.Header.Set("Authorization", "Bearer "+token)
	resp2, err := http.DefaultClient.Do(req2)
	if err != nil {
		t.Fatalf("AC5 valid-token http: %v", err)
	}
	defer resp2.Body.Close()
	if resp2.StatusCode == 401 {
		t.Fatalf("AC5: 有效 token 不应返 401")
	}
}

// readAll — minimal io.ReadAll-equivalent (avoid importing io just for read).
func readAll(r interface {
	Read(p []byte) (int, error)
}) ([]byte, error) {
	var out []byte
	buf := make([]byte, 1024)
	for {
		n, err := r.Read(buf)
		if n > 0 {
			out = append(out, buf[:n]...)
		}
		if err != nil {
			if err.Error() == "EOF" {
				return out, nil
			}
			return out, err
		}
	}
}
