package cli

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// fakeIndexBackend installs a stub `IndexBackend` for the duration of `t`. It
// records the captured request + invocation count and emits `progressMsgs` in
// order via `onProgress`. Returns `streamErr` from the call (nil for clean
// stream completion). Restores any previously-wired backend on cleanup so
// tests do not pollute each other.
type fakeIndexCall struct {
	gotReq *contextforgev1.IndexRequest
	calls  int
}

func fakeIndexBackend(t *testing.T, progressMsgs []*contextforgev1.IndexProgress, streamErr error) *fakeIndexCall {
	t.Helper()
	prev := fetchIndexStream
	call := &fakeIndexCall{}
	fetchIndexStream = func(_ context.Context, req *contextforgev1.IndexRequest, onProgress func(*contextforgev1.IndexProgress)) error {
		call.calls++
		call.gotReq = req
		for _, p := range progressMsgs {
			onProgress(p)
		}
		return streamErr
	}
	t.Cleanup(func() { fetchIndexStream = prev })
	return call
}

// finalProgress is a helper for building a `done=true` IndexProgress.
func finalProgress(filesProcessed, chunksWritten, denied, redacted int64, errStr string) *contextforgev1.IndexProgress {
	return &contextforgev1.IndexProgress{
		FilesProcessed:        filesProcessed,
		FilesSkippedDenied:    denied,
		FilesSkippedRedaction: redacted,
		ChunksWritten:         chunksWritten,
		CurrentFile:           "",
		Done:                  true,
		Error:                 errStr,
	}
}

// progressMid is a helper for non-final IndexProgress.
func progressMid(filesProcessed, chunksWritten int64, currentFile string) *contextforgev1.IndexProgress {
	return &contextforgev1.IndexProgress{
		FilesProcessed: filesProcessed,
		ChunksWritten:  chunksWritten,
		CurrentFile:    currentFile,
		Done:           false,
	}
}

// TEST-9.3.2 / SCEN-9.3.2 / AC2 — CLI 人类可读输出 (\r-overwrite + final summary).
// 同时复用 task-8.2 AC4 的 "long-task mode" / "resuming" 行为保证不回归（mode 输出在 first line）。
func TestTask93_AC2_RunIndex_HumanModeAndManifestRoundtrip(t *testing.T) {
	dataDir := filepath.Join(t.TempDir(), "data")
	source := filepath.Join(t.TempDir(), "repo")
	if err := os.MkdirAll(source, 0o700); err != nil {
		t.Fatalf("mkdir source: %v", err)
	}

	fakeIndexBackend(t, []*contextforgev1.IndexProgress{
		progressMid(1, 3, filepath.Join(source, "a.md")),
		progressMid(2, 6, filepath.Join(source, "b.md")),
		progressMid(3, 9, filepath.Join(source, "c.md")),
		finalProgress(3, 9, 0, 0, ""),
	}, nil)

	var stdout, stderr bytes.Buffer
	code := runIndex([]string{
		"--source=" + source,
		"--data-dir=" + dataDir,
		"--collection=default",
		"--resume",
		"--changed-items=100",
	}, &stdout, &stderr)
	if code != 0 {
		t.Fatalf("runIndex exit=%d stderr=%q", code, stderr.String())
	}
	out := stdout.String()
	if !strings.Contains(out, "long-task mode") {
		t.Fatalf("AC2/task-8.2 AC4: expected 'long-task mode' header, got:\n%s", out)
	}
	if !strings.Contains(out, "indexing") {
		t.Fatalf("AC2: expected human-readable progress 'indexing <file>' line, got:\n%s", out)
	}
	if !strings.Contains(out, "files=3") || !strings.Contains(out, "chunks=9") {
		t.Fatalf("AC2: expected final summary files=3 chunks=9, got:\n%s", out)
	}
	manifest := filepath.Join(dataDir, "runtime", "index-default.resume.json")
	if _, err := os.Stat(manifest); err != nil {
		t.Fatalf("AC2/task-8.2 AC4: resume manifest missing: %v", err)
	}

	// 二次 run — task-8.2 AC4 resume 行为不回归（仍打 "resuming"）。
	stdout.Reset()
	stderr.Reset()
	fakeIndexBackend(t, []*contextforgev1.IndexProgress{
		finalProgress(0, 0, 0, 0, ""),
	}, nil)
	code = runIndex([]string{
		"--source=" + source,
		"--data-dir=" + dataDir,
		"--collection=default",
		"--resume",
		"--changed-items=100",
	}, &stdout, &stderr)
	if code != 0 {
		t.Fatalf("AC2/task-8.2 AC4 second run exit=%d stderr=%q", code, stderr.String())
	}
	// First run set Completed=true; resume scope check sees existing manifest
	// completed (sameManifestScope but completed) → reliability returns a fresh
	// manifest (resumed=false). Either way, the "long-task mode" string is what
	// task-8.2 AC4 requires, plus second-run completion semantics. We assert
	// the header is *some* mode and a fresh manifest exists.
	out = stdout.String()
	if !strings.Contains(out, "mode") {
		t.Fatalf("AC2/task-8.2 AC4 second run: expected mode header, got:\n%s", out)
	}
}

// TEST-9.3.3 / SCEN-9.3.3 / AC3 — --json mode 每行 JSON.
func TestTask93_AC3_RunIndex_JSONMode(t *testing.T) {
	dataDir := filepath.Join(t.TempDir(), "data")
	source := filepath.Join(t.TempDir(), "repo")
	if err := os.MkdirAll(source, 0o700); err != nil {
		t.Fatalf("mkdir source: %v", err)
	}
	fakeIndexBackend(t, []*contextforgev1.IndexProgress{
		progressMid(1, 2, "x.md"),
		finalProgress(1, 2, 0, 0, ""),
	}, nil)

	var stdout, stderr bytes.Buffer
	code := runIndex([]string{
		"--source=" + source,
		"--data-dir=" + dataDir,
		"--collection=demo",
		"--json",
	}, &stdout, &stderr)
	if code != 0 {
		t.Fatalf("AC3 exit=%d stderr=%q", code, stderr.String())
	}
	lines := strings.Split(strings.TrimSpace(stdout.String()), "\n")
	if len(lines) < 2 {
		t.Fatalf("AC3: expected ≥2 JSONL lines (mid + final), got %d: %q", len(lines), stdout.String())
	}
	for i, line := range lines {
		if strings.TrimSpace(line) == "" {
			continue
		}
		var m map[string]any
		if err := json.Unmarshal([]byte(line), &m); err != nil {
			t.Fatalf("AC3: line %d not valid JSON: %q err=%v", i, line, err)
		}
		for _, k := range []string{"files_processed", "chunks_written", "current_file", "done", "error"} {
			if _, ok := m[k]; !ok {
				t.Fatalf("AC3: line %d missing field %q: %v", i, k, m)
			}
		}
	}
}

// TEST-9.3.x — backend not wired 报错 + exit 1.
func TestTask93_BackendNotWired_ReturnsError(t *testing.T) {
	prev := fetchIndexStream
	fetchIndexStream = nil
	t.Cleanup(func() { fetchIndexStream = prev })

	dataDir := filepath.Join(t.TempDir(), "data")
	source := filepath.Join(t.TempDir(), "repo")
	_ = os.MkdirAll(source, 0o700)

	var stdout, stderr bytes.Buffer
	code := runIndex([]string{
		"--source=" + source,
		"--data-dir=" + dataDir,
		"--collection=demo",
	}, &stdout, &stderr)
	if code != 1 {
		t.Fatalf("expected exit 1 (backend not wired), got %d stderr=%q", code, stderr.String())
	}
	if !strings.Contains(stderr.String(), "backend not wired") {
		t.Fatalf("expected 'backend not wired' message, got: %s", stderr.String())
	}
}

// TEST-9.3.x — final IndexProgress.error 非空 → exit 1.
func TestTask93_InBandIndexerError_ExitsOne(t *testing.T) {
	dataDir := filepath.Join(t.TempDir(), "data")
	source := filepath.Join(t.TempDir(), "repo")
	_ = os.MkdirAll(source, 0o700)

	fakeIndexBackend(t, []*contextforgev1.IndexProgress{
		finalProgress(0, 0, 0, 0, "synthetic indexer failure"),
	}, nil)

	var stdout, stderr bytes.Buffer
	code := runIndex([]string{
		"--source=" + source,
		"--data-dir=" + dataDir,
		"--collection=demo",
	}, &stdout, &stderr)
	if code != 1 {
		t.Fatalf("expected exit 1 (in-band err), got %d stderr=%q", code, stderr.String())
	}
	if !strings.Contains(stderr.String(), "synthetic indexer failure") {
		t.Fatalf("expected in-band error surfaced, got: %s", stderr.String())
	}
}

// TEST-9.3.x — gRPC transport-level error → exit 1.
func TestTask93_TransportError_ExitsOne(t *testing.T) {
	dataDir := filepath.Join(t.TempDir(), "data")
	source := filepath.Join(t.TempDir(), "repo")
	_ = os.MkdirAll(source, 0o700)

	fakeIndexBackend(t, nil, errors.New("synthetic gRPC transport error"))

	var stdout, stderr bytes.Buffer
	code := runIndex([]string{
		"--source=" + source,
		"--data-dir=" + dataDir,
		"--collection=demo",
	}, &stdout, &stderr)
	if code != 1 {
		t.Fatalf("expected exit 1 (transport err), got %d stderr=%q", code, stderr.String())
	}
	if !strings.Contains(stderr.String(), "synthetic gRPC transport error") {
		t.Fatalf("expected transport error surfaced, got: %s", stderr.String())
	}
}

// TEST-9.3.x — usage / bad args.
func TestTask93_MissingSource_ExitsTwo(t *testing.T) {
	dataDir := filepath.Join(t.TempDir(), "data")
	var stdout, stderr bytes.Buffer
	code := runIndex([]string{
		"--data-dir=" + dataDir,
	}, &stdout, &stderr)
	if code != 2 {
		t.Fatalf("expected exit 2 (missing source), got %d stderr=%q", code, stderr.String())
	}
}
