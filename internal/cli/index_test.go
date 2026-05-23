package cli

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TEST-8.2.4 / SCEN-8.2.4 / AC4
func TestTask82_AC4_RunIndexResumeCreatesLongTaskManifest(t *testing.T) {
	dataDir := filepath.Join(t.TempDir(), "data")
	source := filepath.Join(t.TempDir(), "repo")
	if err := os.MkdirAll(source, 0o700); err != nil {
		t.Fatalf("mkdir source: %v", err)
	}

	var stdout, stderr bytes.Buffer
	code := runIndex([]string{
		"--source=" + source,
		"--data-dir=" + dataDir,
		"--collection=default",
		"--resume",
		"--changed-items=100000",
	}, &stdout, &stderr)
	if code != 0 {
		t.Fatalf("runIndex exit=%d stderr=%q", code, stderr.String())
	}
	out := stdout.String()
	if !strings.Contains(out, "long-task mode") || !strings.Contains(out, "resume_manifest=") {
		t.Fatalf("stdout should announce long task and manifest path:\n%s", out)
	}
	manifest := filepath.Join(dataDir, "runtime", "index-default.resume.json")
	if _, err := os.Stat(manifest); err != nil {
		t.Fatalf("resume manifest missing: %v", err)
	}

	stdout.Reset()
	stderr.Reset()
	code = runIndex([]string{
		"--source=" + source,
		"--data-dir=" + dataDir,
		"--collection=default",
		"--resume",
		"--changed-items=100000",
	}, &stdout, &stderr)
	if code != 0 {
		t.Fatalf("runIndex resume exit=%d stderr=%q", code, stderr.String())
	}
	if !strings.Contains(stdout.String(), "resuming") {
		t.Fatalf("second run should report resuming, got:\n%s", stdout.String())
	}
}
