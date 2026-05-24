// Package cli (sub-file index.go) — task-9.3 cli-index real implementation.
//
// Contract: task-9.3 §5.3. `contextforge index --source <path> --data-dir <root>
// --collection <id> [--resume] [--changed-items N] [--json]` parses flags,
// opens / resumes the reliability manifest (task-8.2), and consumes the
// injected `IndexBackend` (wired by cmd/contextforge/main.go to a daemon
// spawn + Daemon.Index callable) IndexProgress stream. Renders per-message
// progress to stdout (\r-overwrite line by default, --json: JSONL stream)
// and persists manifest ProcessedItems every 10 messages + at end.
//
// `internal/cli` deliberately does NOT import `internal/daemon` — the
// daemon-spawn implementation lives in cmd/contextforge/main.go which
// injects via SetIndexBackend (same pattern as SetSearchBackend / task-6.1).
package cli

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/config"
	"github.com/tajiaoyezi/contextforge/internal/reliability"
	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// indexProgressFlushInterval — manifest ProcessedItems is persisted every N
// IndexProgress messages received (plus once at terminal). Avoids per-message
// disk write; spec §5.3 allows N+10 slop in resume position.
const indexProgressFlushInterval = 10

// indexCommandTimeout caps the index call; 30 min covers very large repos
// (PRD §User Flow 异常流 "long task mode").
const indexCommandTimeout = 30 * time.Minute

// IndexBackend is the injectable gRPC `Index` stream callable wired from
// cmd/contextforge/main.go (production: daemon spawn + Daemon.Index) and
// substituted by tests with a fake that emits canned IndexProgress messages.
// Returning nil error indicates clean stream completion; caller still must
// inspect final IndexProgress.Error (in-band indexer failure per task-9.2
// §5.3) via the onProgress callback.
type IndexBackend func(
	ctx context.Context,
	req *contextforgev1.IndexRequest,
	onProgress func(*contextforgev1.IndexProgress),
) error

// fetchIndexStream is the active backend; nil means runIndex returns a clear
// `backend not wired` message rather than panicking.
var fetchIndexStream IndexBackend

// SetIndexBackend wires the production gRPC `Index` backend. Called once
// from cmd/contextforge/main.go before Execute.
func SetIndexBackend(b IndexBackend) {
	if b == nil {
		panic("cli.SetIndexBackend: nil backend")
	}
	fetchIndexStream = b
}

type indexOpts struct {
	Source       string
	DataDir      string
	Collection   string
	Resume       bool
	ChangedItems int64
	JSON         bool
}

func runIndex(args []string, stdout, stderr io.Writer) int {
	opts, err := parseIndexOpts(args, stderr)
	if err != nil {
		return 2
	}
	if opts.Source == "" {
		fmt.Fprintln(stderr, "contextforge index: --source is required")
		return 2
	}
	if opts.ChangedItems < 0 {
		fmt.Fprintln(stderr, "contextforge index: --changed-items must be >=0")
		return 2
	}

	manifestPath := indexManifestPath(opts.DataDir, opts.Collection)
	manifest, resumed, err := reliability.StartOrResumeManifest(manifestPath, reliability.ManifestOptions{
		SourcePath: opts.Source,
		DataDir:    opts.DataDir,
		Collection: opts.Collection,
		TotalItems: opts.ChangedItems,
	})
	if err != nil {
		fmt.Fprintf(stderr, "contextforge index: resume manifest: %v\n", err)
		return 1
	}

	mode := "long-task mode"
	if resumed {
		mode = "resuming long-task mode"
	}
	if !opts.Resume {
		mode = "safe rebuild mode"
	}
	if !opts.JSON {
		fmt.Fprintf(stdout, "contextforge index: %s collection=%s\n", mode, manifest.Collection)
	}

	if fetchIndexStream == nil {
		fmt.Fprintln(stderr,
			"contextforge index: index backend not wired "+
				"(cmd/contextforge/main.go must call cli.SetIndexBackend)")
		return 1
	}

	ctx, cancel := context.WithTimeout(context.Background(), indexCommandTimeout)
	defer cancel()

	req := &contextforgev1.IndexRequest{
		SourcePath:   opts.Source,
		DataDir:      opts.DataDir,
		CollectionId: opts.Collection,
	}

	var (
		lastFinal     *contextforgev1.IndexProgress
		msgCount      int
		jsonEnc       *json.Encoder
		flushManifest = func(p *contextforgev1.IndexProgress) {
			_ = reliability.MarkProgress(manifestPath, p.GetFilesProcessed())
		}
	)
	if opts.JSON {
		jsonEnc = json.NewEncoder(stdout)
	}

	err = fetchIndexStream(ctx, req, func(p *contextforgev1.IndexProgress) {
		msgCount++
		if opts.JSON {
			_ = jsonEnc.Encode(map[string]any{
				"files_processed":         p.GetFilesProcessed(),
				"files_skipped_denied":    p.GetFilesSkippedDenied(),
				"files_skipped_redaction": p.GetFilesSkippedRedaction(),
				"chunks_written":          p.GetChunksWritten(),
				"current_file":            p.GetCurrentFile(),
				"done":                    p.GetDone(),
				"error":                   p.GetError(),
			})
		} else {
			if !p.GetDone() {
				fmt.Fprintf(stdout, "\rindexing %s (files=%d, chunks=%d)",
					p.GetCurrentFile(), p.GetFilesProcessed(), p.GetChunksWritten())
			}
		}
		if p.GetDone() {
			lastFinal = p
		}
		if msgCount%indexProgressFlushInterval == 0 || p.GetDone() {
			flushManifest(p)
		}
	})
	if err != nil {
		fmt.Fprintf(stderr, "\ncontextforge index: %v\n", err)
		return 1
	}

	if lastFinal == nil {
		fmt.Fprintln(stderr, "\ncontextforge index: stream ended without done=true")
		return 1
	}
	if lastFinal.GetError() != "" {
		fmt.Fprintf(stderr, "\ncontextforge index: indexer error: %s\n", lastFinal.GetError())
		return 1
	}

	if !opts.JSON {
		fmt.Fprintf(stdout,
			"\ncontextforge index: done collection=%s files=%d chunks=%d denied=%d redacted=%d\n",
			opts.Collection,
			lastFinal.GetFilesProcessed(),
			lastFinal.GetChunksWritten(),
			lastFinal.GetFilesSkippedDenied(),
			lastFinal.GetFilesSkippedRedaction(),
		)
	}
	_ = reliability.MarkComplete(manifestPath)
	return 0
}

func parseIndexOpts(args []string, stderr io.Writer) (*indexOpts, error) {
	fs := flag.NewFlagSet("index", flag.ContinueOnError)
	fs.SetOutput(stderr)
	source := fs.String("source", "", "source root to index")
	dataDir := fs.String("data-dir", "", "data root (default ~/.contextforge)")
	collection := fs.String("collection", "default", "collection ID")
	resume := fs.Bool("resume", false, "resume from an incomplete long-task manifest when available")
	changedItems := fs.Int64("changed-items", 0, "estimated changed item count for long-task mode")
	jsonOut := fs.Bool("json", false, "emit IndexProgress as JSONL stream (default: human-readable)")
	if err := fs.Parse(args); err != nil {
		return nil, err
	}
	if *dataDir == "" {
		root, err := config.DefaultRootDir()
		if err != nil {
			return nil, err
		}
		*dataDir = root
	}
	if _, err := os.Stat(*source); *source != "" && err != nil {
		return nil, fmt.Errorf("source %q: %w", *source, err)
	}
	return &indexOpts{
		Source:       *source,
		DataDir:      *dataDir,
		Collection:   *collection,
		Resume:       *resume,
		ChangedItems: *changedItems,
		JSON:         *jsonOut,
	}, nil
}

func indexManifestPath(dataDir, collection string) string {
	if collection == "" {
		collection = "default"
	}
	return filepath.Join(dataDir, "runtime", "index-"+collection+".resume.json")
}
