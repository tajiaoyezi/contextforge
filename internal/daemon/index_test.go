// Package daemon (sub-file index_test.go) — task-9.3 Daemon.Index E2E tests.
//
// AC4 (task-9.3 §6): "TestCliIndex_E2E_RealCore" lives here in the daemon
// package so it can reuse TestMain's cargo build pipeline + spawn a real
// contextforge-core child. (`internal/cli` cannot import `internal/daemon`
// — it would form an import cycle with daemon_test.go.) The test runs the
// production Daemon.Index path end-to-end: real daemon spawn → gRPC stream
// → real Rust IndexSession → SQLite + Tantivy persistence → secret
// redaction assertions. This is the Go-side counterpart to
// core/tests/phase9_index_smoke.rs (Rust-side in-process server smoke).
package daemon

import (
	"context"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

const (
	phase93Marker = "phase9smokemarker93go"
	awsFakeKey    = "AKIAIOSFODNN7EXAMPLE"
)

// TEST-9.3.4 / SCEN-9.3.4 / AC4 — real daemon spawn + Daemon.Index stream
// consume + real Rust indexer + real SQLite/Tantivy. Validates the
// production path that cli.SetIndexBackend wires from cmd/contextforge/main.go.
//
// Skipped under `go test -short` (cargo build cold cache adds ~30-60s).
func TestCliIndex_E2E_RealCore(t *testing.T) {
	if testing.Short() {
		t.Skip("AC4 e2e: -short skips cargo build + real daemon spawn")
	}

	dataDir := filepath.Join(t.TempDir(), "data")
	srcRoot := filepath.Join(t.TempDir(), "src")
	collectionID := "phase9-e2e"

	// CONTEXTFORGE_DATA_DIR signals core's resolve_data_dir so that the
	// post-index Search RPC opens the same data root that Index wrote into.
	// Without this, core defaults to $HOME/.contextforge and Search returns
	// "collection not found" (server.rs::CoreService.search uses self.data_dir
	// captured at startup, while Index respects the per-request data_dir).
	origEnv, hadOrig := os.LookupEnv("CONTEXTFORGE_DATA_DIR")
	if err := os.Setenv("CONTEXTFORGE_DATA_DIR", dataDir); err != nil {
		t.Fatalf("setenv: %v", err)
	}
	t.Cleanup(func() {
		if hadOrig {
			_ = os.Setenv("CONTEXTFORGE_DATA_DIR", origEnv)
		} else {
			_ = os.Unsetenv("CONTEXTFORGE_DATA_DIR")
		}
	})

	addr := freeAddr(t)
	ctx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
	defer cancel()

	d, err := Start(ctx, Options{CoreBinPath: coreBin, ListenAddr: addr})
	if err != nil {
		t.Fatalf("daemon Start: %v", err)
	}
	t.Cleanup(func() { d.Stop() })

	if got := pollHealth(t, d, 15*time.Second); got != "SERVING" {
		t.Fatalf("AC4: daemon health = %q, want SERVING", got)
	}

	// Fixture: 3 normal .md (containing the marker) + 1 .env (denylist) + 1
	// .yaml containing a fake AWS key (secret-redaction). Mirror the Rust
	// phase9_index_smoke.rs fixture so secret + denylist + happy-path
	// assertions all run here.
	writeFile(t, filepath.Join(srcRoot, "README.md"),
		"# README\n\nUnique marker: "+phase93Marker+" body line.\n")
	writeFile(t, filepath.Join(srcRoot, "docs", "guide.md"),
		"# Guide\n\nAlso "+phase93Marker+" multi-hit fixture.\n")
	writeFile(t, filepath.Join(srcRoot, "notes", "log.md"),
		"# Notes\n\nRegular content; no marker here.\n")
	writeFile(t, filepath.Join(srcRoot, ".env"),
		"TOKEN=should-not-be-indexed-and-is-denylisted\n")
	writeFile(t, filepath.Join(srcRoot, "config.yaml"),
		"aws_key: "+awsFakeKey+"\nendpoint: https://api.example.invalid\n")

	var messages []*contextforgev1.IndexProgress
	err = d.Index(ctx, &contextforgev1.IndexRequest{
		SourcePath:   srcRoot,
		DataDir:      dataDir,
		CollectionId: collectionID,
	}, func(p *contextforgev1.IndexProgress) {
		messages = append(messages, p)
	})
	if err != nil {
		t.Fatalf("AC4: Daemon.Index transport error: %v", err)
	}

	if len(messages) < 4 {
		t.Fatalf("AC4: expected ≥4 IndexProgress (3 normal + final done), got %d", len(messages))
	}
	final := messages[len(messages)-1]
	if !final.GetDone() {
		t.Fatalf("AC4: final message done=false: %+v", final)
	}
	if final.GetError() != "" {
		t.Fatalf("AC4: final.Error = %q, expected empty", final.GetError())
	}
	if final.GetFilesProcessed() < 3 {
		t.Fatalf("AC4: files_processed=%d want ≥3", final.GetFilesProcessed())
	}
	if final.GetChunksWritten() <= 0 {
		t.Fatalf("AC4: chunks_written=%d want >0", final.GetChunksWritten())
	}
	if final.GetFilesSkippedDenied() < 1 {
		t.Fatalf("AC4: files_skipped_denied=%d want ≥1 (the .env)", final.GetFilesSkippedDenied())
	}

	// AC4: SQLite has rows + Tantivy can locate the marker via Search RPC +
	// the raw secret literal is NOT indexed.
	searchResp, err := d.Search(ctx, &contextforgev1.SearchRequest{
		Query:       phase93Marker,
		Collections: []string{collectionID},
		TopK:        10,
	})
	if err != nil {
		t.Fatalf("AC4: post-index Search RPC: %v", err)
	}
	if len(searchResp.GetResults()) == 0 {
		t.Fatalf("AC4: post-index Search for marker %q returned 0 results", phase93Marker)
	}

	secretResp, err := d.Search(ctx, &contextforgev1.SearchRequest{
		Query:       awsFakeKey,
		Collections: []string{collectionID},
		TopK:        10,
	})
	if err != nil {
		t.Fatalf("AC4: post-index Search RPC (secret): %v", err)
	}
	if len(secretResp.GetResults()) > 0 {
		// Some hits may surface if Tantivy stems/normalises the literal; assert
		// no result's content matches the raw secret string by inspecting file
		// path metadata (the secret only appeared in config.yaml).
		for _, r := range secretResp.GetResults() {
			if strings.Contains(r.GetFilePath(), "config.yaml") {
				t.Fatalf("AC4/R4 redaction regression: config.yaml indexed via secret query: %+v", r)
			}
		}
	}
}

// TEST-9.3.x — invalid source_path → InvalidArgument propagates through
// Daemon.Index transport error path.
func TestDaemonIndex_InvalidSourcePath(t *testing.T) {
	if testing.Short() {
		t.Skip("-short skips real daemon spawn")
	}

	addr := freeAddr(t)
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	d, err := Start(ctx, Options{CoreBinPath: coreBin, ListenAddr: addr})
	if err != nil {
		t.Fatalf("daemon Start: %v", err)
	}
	t.Cleanup(func() { d.Stop() })

	if got := pollHealth(t, d, 15*time.Second); got != "SERVING" {
		t.Fatalf("daemon health = %q, want SERVING", got)
	}

	err = d.Index(ctx, &contextforgev1.IndexRequest{
		SourcePath:   filepath.Join(os.TempDir(), "nonexistent-phase9-3-go"),
		DataDir:      filepath.Join(t.TempDir(), "data"),
		CollectionId: "x",
	}, func(*contextforgev1.IndexProgress) {})
	if err == nil {
		t.Fatalf("expected InvalidArgument-style error for nonexistent source_path")
	}
	if !strings.Contains(err.Error(), "InvalidArgument") && !strings.Contains(err.Error(), "does not exist") {
		t.Fatalf("expected InvalidArgument / does-not-exist error, got: %v", err)
	}
}

func writeFile(t *testing.T, path, content string) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatalf("mkdir: %v", err)
	}
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write %s: %v", path, err)
	}
}
