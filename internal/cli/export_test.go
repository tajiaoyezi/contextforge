package cli

import (
	"bytes"
	"context"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/exporter"
	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// TEST-33.4.1 (task-33.4 AC1): export --timeout is an add-only flag that
// defaults to 60s (byte-equivalent to the pre-33.4 hardcoded value, ADR-004)
// and accepts an override; the existing flags keep parsing unchanged.
func TestParseExportOpts_Timeout(t *testing.T) {
	base := []string{"--format", "jsonl", "--output", "/tmp/out.jsonl", "--data-dir", "/tmp/d"}

	// Unset → 60s default (byte-equivalent to the old hardcoded WithTimeout).
	opts, err := parseExportOpts(base, &bytes.Buffer{})
	if err != nil {
		t.Fatalf("parse (default timeout): %v", err)
	}
	if opts.Timeout != 60*time.Second {
		t.Errorf("default timeout: got %v want 60s", opts.Timeout)
	}
	// Existing flags still parse.
	if opts.Format != "jsonl" || opts.Output != "/tmp/out.jsonl" || opts.DataDir != "/tmp/d" {
		t.Errorf("existing flags regressed: %+v", opts)
	}

	// Explicit override is honored.
	withTo, err := parseExportOpts(append(base, "--timeout", "120s"), &bytes.Buffer{})
	if err != nil {
		t.Fatalf("parse (--timeout=120s): %v", err)
	}
	if withTo.Timeout != 120*time.Second {
		t.Errorf("override timeout: got %v want 120s", withTo.Timeout)
	}
}

// TEST-6.3.5 / SCEN-6.3.5 / AC5
func TestTask63_AC5_RunExportSubcommandEndToEndAndFormatFlags(t *testing.T) {
	var seenQueries []string
	restore := exporter.SetSearchBackend(func(
		ctx context.Context,
		dataDir string,
		req *contextforgev1.SearchRequest,
	) (*contextforgev1.SearchResponse, error) {
		seenQueries = append(seenQueries, req.GetQuery()+"|"+dataDir)
		return &contextforgev1.SearchResponse{
			Results: []*contextforgev1.RetrievalResult{
				{
					ChunkId:         "chunk-1",
					ContextId:       "ctx-1",
					FilePath:        "fixture/readme.md",
					LineStart:       1,
					LineEnd:         3,
					RetrievalMethod: "bm25",
					RedactionStatus: "applied",
					Provenance: []*contextforgev1.Provenance{
						{Importer: "scanner", OriginalPath: "fixture/readme.md"},
					},
				},
			},
		}, nil
	})
	defer restore()

	dataDir := filepath.Join(t.TempDir(), "data")
	cases := []struct {
		format string
		output string
		check  func(t *testing.T, output string)
	}{
		{
			format: "jsonl",
			output: filepath.Join(t.TempDir(), "out.jsonl"),
			check: func(t *testing.T, output string) {
				t.Helper()
				body, err := os.ReadFile(output)
				if err != nil {
					t.Fatalf("jsonl output missing: %v", err)
				}
				if !bytes.Contains(body, []byte(`"id":"ctx-1"`)) {
					t.Fatalf("jsonl output missing exported record id:\n%s", body)
				}
			},
		},
		{
			format: "markdown-bundle",
			output: filepath.Join(t.TempDir(), "out.tar.gz"),
			check: func(t *testing.T, output string) {
				t.Helper()
				if info, err := os.Stat(output); err != nil || info.Size() == 0 {
					t.Fatalf("markdown bundle missing or empty: info=%v err=%v", info, err)
				}
			},
		},
		{
			format: "agent-draft",
			output: filepath.Join(t.TempDir(), "draft"),
			check: func(t *testing.T, output string) {
				t.Helper()
				if _, err := os.Stat(filepath.Join(output, "MEMORY.md")); err != nil {
					t.Fatalf("agent draft MEMORY.md missing: %v", err)
				}
			},
		},
	}

	for _, tc := range cases {
		t.Run(tc.format, func(t *testing.T) {
			var stdout, stderr bytes.Buffer
			code := runExport([]string{
				"--format=" + tc.format,
				"--collection=default",
				"--data-dir=" + dataDir,
				"--output=" + tc.output,
				"--include-stale",
			}, &stdout, &stderr)
			if code != 0 {
				t.Fatalf("runExport(%s) exit=%d stderr=%q", tc.format, code, stderr.String())
			}
			if !strings.Contains(stdout.String(), "exported 1 records") {
				t.Fatalf("stdout missing summary, got %q", stdout.String())
			}
			tc.check(t, tc.output)
		})
	}

	if got, want := len(seenQueries), len(cases); got != want {
		t.Fatalf("search backend calls=%d want %d", got, want)
	}
	for _, q := range seenQueries {
		if !strings.HasPrefix(q, "*|"+dataDir) {
			t.Fatalf("export should pseudo full-scan with query=* and data-dir %s, got %q", dataDir, q)
		}
	}

	var stdout, stderr bytes.Buffer
	if code := runExport([]string{"--output=x"}, &stdout, &stderr); code != 2 {
		t.Fatalf("missing --format should be usage exit 2, got %d stderr=%q", code, stderr.String())
	}
}
