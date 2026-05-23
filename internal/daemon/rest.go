// Package daemon (sub-file rest.go) — task-6.2 REST API server.
// Contract: task-6.2 §5.3. stdlib net/http + http.ServeMux (Go 1.22+
// `r.PathValue`). 5 endpoints: POST /v1/search + GET /v1/chunks/{id} +
// GET /v1/collections (real implementations) + POST /v1/import +
// POST /v1/eval/run (stub 501, §2A 决策 B). Authorization Bearer
// middleware + audit log every access (脱敏 per AC5).

package daemon

import (
	"context"
	"crypto/subtle"
	"encoding/json"
	"errors"
	"fmt"
	"net"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/memoryops/audit"
	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

// RESTSearcher is the gRPC `Search` callable consumed by all five REST
// handlers (POST /v1/search uses it directly; GET /v1/chunks/{id} routes
// through it so the Rust-side `CoreService::search` fast-path
// (task-6.2 §2A 决策 E) can short-circuit to `retriever.get_chunk`).
// The production `*Daemon` satisfies this interface via task-6.1
// `daemon.Search`; tests inject a fake.
type RESTSearcher interface {
	Search(ctx context.Context, req *contextforgev1.SearchRequest) (*contextforgev1.SearchResponse, error)
}

// shutdownGrace is the maximum time `ServeREST` waits for in-flight
// requests to drain after the caller cancels its context.
const shutdownGrace = 5 * time.Second

// NewRESTHandler builds the full REST handler tree (5 endpoints + auth
// middleware + audit). Production `ServeREST` wraps this around `*Daemon`;
// tests wrap it around a fake `RESTSearcher` to skip cargo build + a real
// loopback bind.
func NewRESTHandler(s RESTSearcher, token, dataDir string) http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("POST /v1/search", func(w http.ResponseWriter, r *http.Request) {
		handleSearch(s, w, r)
	})
	mux.HandleFunc("GET /v1/chunks/{id}", func(w http.ResponseWriter, r *http.Request) {
		handleChunk(s, w, r)
	})
	mux.HandleFunc("GET /v1/collections", func(w http.ResponseWriter, r *http.Request) {
		handleCollections(dataDir, w, r)
	})
	mux.HandleFunc("POST /v1/import", handleImport)
	mux.HandleFunc("POST /v1/eval/run", handleEval)
	return authMiddleware(mux, token, dataDir)
}

// ServeREST is the production entry: serve REST on the provided listener
// (loopback TCP or Unix socket — the caller has already validated it),
// until the caller cancels `ctx`. On cancellation a 5-second graceful
// shutdown drains in-flight requests, then the function returns nil.
func (d *Daemon) ServeREST(
	ctx context.Context,
	listener net.Listener,
	token, dataDir string,
) error {
	handler := NewRESTHandler(d, token, dataDir)
	srv := &http.Server{Handler: handler}

	serverDone := make(chan error, 1)
	go func() { serverDone <- srv.Serve(listener) }()

	select {
	case err := <-serverDone:
		if errors.Is(err, http.ErrServerClosed) {
			return nil
		}
		return err
	case <-ctx.Done():
		shutdownCtx, cancel := context.WithTimeout(context.Background(), shutdownGrace)
		defer cancel()
		return srv.Shutdown(shutdownCtx)
	}
}

// authMiddleware enforces Authorization: Bearer + writes one audit-rest.log
// line per access (including 401 denials). The token value and request
// body are NEVER logged — AC5 redaction. Const-time comparison via
// crypto/subtle defends against timing oracles.
func authMiddleware(next http.Handler, token, dataDir string) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		rec := &statusCapture{ResponseWriter: w, status: 200}

		if !checkBearerToken(r, token) {
			writeJSON(rec, 401, map[string]any{"error": "missing or invalid token"})
			if err := audit.Write(dataDir, audit.Event{
				Endpoint:  r.URL.Path,
				Status:    401,
				Timestamp: time.Now().UTC(),
				Reason:    "missing or invalid token",
			}); err != nil {
				// audit chain break is a v0.1 operability signal (disk full /
				// permission error). Stay non-blocking for the response, but
				// surface on stderr so ops can notice (PR #44 review FIX-2).
				fmt.Fprintf(os.Stderr, "contextforge audit: %v\n", err)
			}
			return
		}

		next.ServeHTTP(rec, r)
		if err := audit.Write(dataDir, audit.Event{
			Endpoint:  r.URL.Path,
			Status:    rec.status,
			Timestamp: time.Now().UTC(),
		}); err != nil {
			fmt.Fprintf(os.Stderr, "contextforge audit: %v\n", err)
		}
	})
}

func checkBearerToken(r *http.Request, expected string) bool {
	header := r.Header.Get("Authorization")
	if !strings.HasPrefix(header, "Bearer ") {
		return false
	}
	presented := strings.TrimPrefix(header, "Bearer ")
	return subtle.ConstantTimeCompare([]byte(presented), []byte(expected)) == 1
}

// handleSearch — POST /v1/search.
func handleSearch(s RESTSearcher, w http.ResponseWriter, r *http.Request) {
	var req contextforgev1.SearchRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, 400, map[string]any{"error": "invalid JSON: " + err.Error()})
		return
	}
	resp, err := s.Search(r.Context(), &req)
	if err != nil {
		writeJSON(w, GRPCStatusToHTTP(err), map[string]any{"error": err.Error()})
		return
	}
	writeJSON(w, 200, resp)
}

// handleChunk — GET /v1/chunks/{id}.
//
// v0.1 fast-path: route through daemon.Search with query=id, top_k=1, so
// the Rust-side `CoreService::search` fast-path (task-6.2 §2A 决策 E) can
// short-circuit to `retriever.get_chunk` when the id matches the chunker
// format. Empty results → 404. The collection is taken from
// `?collection=<id>` (default `"default"` per PRD §default collection).
func handleChunk(s RESTSearcher, w http.ResponseWriter, r *http.Request) {
	chunkID := r.PathValue("id")
	if chunkID == "" {
		writeJSON(w, 400, map[string]any{"error": "missing chunk id"})
		return
	}
	coll := r.URL.Query().Get("collection")
	if coll == "" {
		coll = "default"
	}
	resp, err := s.Search(r.Context(), &contextforgev1.SearchRequest{
		Query:       chunkID,
		Collections: []string{coll},
		TopK:        1,
	})
	if err != nil {
		writeJSON(w, GRPCStatusToHTTP(err), map[string]any{"error": err.Error()})
		return
	}
	if len(resp.GetResults()) == 0 {
		writeJSON(w, 404, map[string]any{"error": "chunk not found"})
		return
	}
	writeJSON(w, 200, resp.GetResults()[0])
}

// handleCollections — GET /v1/collections. Lists every `<dataDir>/collections/<id>/`
// subdirectory; one collection per subdirectory (matches Rust indexer layout).
// `chunk_count` is left at 0 in v0.1 — a real SQLite COUNT(*) would require
// opening every collection's DB; deferred to a future task / endpoint.
func handleCollections(dataDir string, w http.ResponseWriter, _ *http.Request) {
	collDir := filepath.Join(dataDir, "collections")
	entries, err := os.ReadDir(collDir)
	if err != nil {
		if os.IsNotExist(err) {
			writeJSON(w, 200, map[string]any{"collections": []any{}})
			return
		}
		writeJSON(w, 500, map[string]any{"error": "read collections: " + err.Error()})
		return
	}
	type collInfo struct {
		ID            string `json:"id"`
		ChunkCount    int64  `json:"chunk_count"`
		LastIndexedAt string `json:"last_indexed_at"`
	}
	out := []collInfo{}
	for _, e := range entries {
		if !e.IsDir() {
			continue
		}
		var lastIndexedAt string
		if info, _ := e.Info(); info != nil {
			lastIndexedAt = info.ModTime().UTC().Format(time.RFC3339)
		}
		out = append(out, collInfo{
			ID:            e.Name(),
			ChunkCount:    0, // v0.1 placeholder — real COUNT(*) deferred
			LastIndexedAt: lastIndexedAt,
		})
	}
	writeJSON(w, 200, map[string]any{"collections": out})
}

// handleImport — POST /v1/import: v0.1 stub 501 (§2A 决策 B).
func handleImport(w http.ResponseWriter, _ *http.Request) {
	writeJSON(w, 501, map[string]any{
		"error": "deferred to phase 8",
		"note":  "see task-8.x backlog (importer pipeline)",
	})
}

// handleEval — POST /v1/eval/run: v0.1 stub 501 (§2A 决策 B).
func handleEval(w http.ResponseWriter, _ *http.Request) {
	writeJSON(w, 501, map[string]any{
		"error": "deferred to phase 8 (eval-harness)",
		"note":  "see task-8.1",
	})
}

// GRPCStatusToHTTP maps a gRPC error to an HTTP status code following the
// google.rpc.Code → HTTP standard map. `nil` → 200 (caller hasn't errored).
// Unknown / wrapped errors → 500. §5.3 contract; reused by handleSearch +
// handleChunk error paths. Exported for unit tests.
func GRPCStatusToHTTP(err error) int {
	if err == nil {
		return 200
	}
	s, ok := status.FromError(err)
	if !ok {
		return 500
	}
	switch s.Code() {
	case codes.OK:
		return 200
	case codes.InvalidArgument:
		return 400
	case codes.FailedPrecondition:
		return 412
	case codes.NotFound:
		return 404
	case codes.Unauthenticated:
		return 401
	case codes.PermissionDenied:
		return 403
	case codes.Internal:
		return 500
	default:
		return 500
	}
}

// statusCapture wraps an http.ResponseWriter so the auth middleware can
// audit the final status code (auto-200 if the handler never calls
// WriteHeader explicitly).
type statusCapture struct {
	http.ResponseWriter
	status      int
	wroteHeader bool
}

func (sc *statusCapture) WriteHeader(code int) {
	if !sc.wroteHeader {
		sc.status = code
		sc.wroteHeader = true
	}
	sc.ResponseWriter.WriteHeader(code)
}

func (sc *statusCapture) Write(b []byte) (int, error) {
	if !sc.wroteHeader {
		sc.status = 200
		sc.wroteHeader = true
	}
	return sc.ResponseWriter.Write(b)
}

// writeJSON marshals `v` and writes a JSON response with the given status.
// Encoding errors are silently dropped (no double-write); the alternative
// (panic / 500-after-WriteHeader) would corrupt the response stream worse.
func writeJSON(w http.ResponseWriter, code int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(code)
	_ = json.NewEncoder(w).Encode(v)
}
