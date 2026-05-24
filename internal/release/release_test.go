// task-9.5 §6 AC1-AC5 release tests.
//
// History: v0.1 task-8.3 introduced AC1/AC2/AC4/AC5 tests that fed fake
// stub-evidence StepResult literals into ValidateSmokeEvidence /
// ValidateV01Closure / ValidatePhaseSmoke — passing the release-smoke gate
// without ever executing the CLI. Phase 9 task-9.5 (ADR-013 §Decision #4)
// removes that fake-evidence surface:
//
//   - AC1 (TestTask83_AC1) rewritten to REAL `go build` + `cargo build`
//     binaries before BuildTarball / ValidateTarball.
//   - AC2 (TestTask83_AC2 "ReleaseSmokeEvidenceRequiresOrderedPassingSteps")
//     DELETED — validator-self-test with stub evidence; superseded by the new
//     end-to-end `TestPhase9ReleaseSmoke_EndToEnd` in
//     release_smoke_e2e_test.go which exercises the validator against real
//     CLI evidence.
//   - AC3 (TestTask83_AC3 benchmark validator gate) RETAINED — unit-level
//     synthetic benchmark threshold check; real 100k benchmark out of scope
//     (task-9.5 §3 OOS — nightly task).
//   - AC4 (TestTask83_AC4 "V01ClosureRequiresSevenTechnicalAreas") DELETED —
//     same fake-evidence pattern as AC2; superseded by real e2e closure
//     coverage emerging from the end-to-end suite.
//   - AC5 (TestTask83_AC5 PhaseSmokeReport aggregator) DELETED — its
//     structural aggregation behaviour is now validated indirectly by
//     TestPhase9ReleaseSmoke_EndToEnd which builds a PhaseSmokeReport from
//     real evidence + calls ValidatePhaseSmoke. Removing it eliminates the
//     last stub-evidence literal in this package.

package release

import (
	"archive/tar"
	"compress/gzip"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"testing"
)

// TEST-9.5.1 / SCEN-9.5.1 / AC1 — REAL `go build` + `cargo build` + real
// binary tarball + ValidateTarball pass + executable-bit assertion. Replaces
// task-8.3 AC1's fake `name+"\n"` binary content (task-9.5 §3 + ADR-013
// §Decision #4 fake-evidence取代).
func TestTask83_AC1_TarballContainsRequiredAssets(t *testing.T) {
	if testing.Short() {
		t.Skip("AC1: -short skips real go build + cargo build (~30-60s cold cache)")
	}
	root := findRepoRoot(t)
	staging := t.TempDir()

	// Build real binaries into staging dir (mirrors release pipeline behaviour).
	goExe := filepath.Join(staging, exeName("contextforge"))
	if out, err := runIn(root, "go", "build", "-o", goExe, "./cmd/contextforge"); err != nil {
		t.Fatalf("AC1: go build: %v\n%s", err, out)
	}
	if out, err := runIn(root, "cargo", "build", "-p", "contextforge-core"); err != nil {
		t.Fatalf("AC1: cargo build: %v\n%s", err, out)
	}
	coreSrc := filepath.Join(root, "target", "debug", exeName("contextforge-core"))
	coreDst := filepath.Join(staging, exeName("contextforge-core"))
	if err := copyFile(coreSrc, coreDst); err != nil {
		t.Fatalf("AC1: copy cargo binary: %v", err)
	}

	// Sidecar required assets (real content from repo).
	for src, dst := range map[string]string{
		filepath.Join(root, "contextforge.example.toml"): filepath.Join(staging, "contextforge.example.toml"),
		filepath.Join(root, "README.md"):                 filepath.Join(staging, "README.md"),
		filepath.Join(root, "LICENSE"):                   filepath.Join(staging, "LICENSE"),
	} {
		if err := copyFile(src, dst); err != nil {
			t.Fatalf("AC1: copy %s: %v", src, err)
		}
	}

	tarball := filepath.Join(staging, "contextforge-linux-amd64-test.tar.gz")
	if err := BuildTarball(tarball, []Asset{
		{Name: "contextforge", Path: goExe, Mode: 0o755},
		{Name: "contextforge-core", Path: coreDst, Mode: 0o755},
		{Name: "contextforge.example.toml", Path: filepath.Join(staging, "contextforge.example.toml"), Mode: 0o644},
		{Name: "README.md", Path: filepath.Join(staging, "README.md"), Mode: 0o644},
		{Name: "LICENSE", Path: filepath.Join(staging, "LICENSE"), Mode: 0o644},
	}); err != nil {
		t.Fatalf("AC1: BuildTarball: %v", err)
	}

	report, err := ValidateTarball(tarball)
	if err != nil {
		t.Fatalf("AC1: ValidateTarball real binary tarball: %v", err)
	}
	if len(report.Entries) < len(RequiredTarballEntries()) {
		t.Fatalf("AC1: entries=%v, want required entries %v", report.Entries, RequiredTarballEntries())
	}
	if report.Modes["contextforge"]&0o111 == 0 {
		t.Fatalf("AC1: contextforge mode=%o missing executable bit", report.Modes["contextforge"])
	}
	if report.Modes["contextforge-core"]&0o111 == 0 {
		t.Fatalf("AC1: contextforge-core mode=%o missing executable bit", report.Modes["contextforge-core"])
	}

	// Defensive: ValidateTarball must still reject a tarball with the
	// contextforge binary lacking the executable bit (preserve task-8.3
	// validator behaviour without re-introducing fake-binary stubs).
	bad := filepath.Join(t.TempDir(), "contextforge-linux-amd64.tar.gz")
	writeNonexecutableTarball(t, bad)
	if _, err := ValidateTarball(bad); err == nil {
		t.Fatal("AC1: ValidateTarball should reject non-executable contextforge")
	}
}

// TEST-9.5.x / SCEN-9.5.5 / AC5 (spec §6 AC5) — benchmark validator gate
// retained as unit-level synthetic check; real 100k benchmark stays out of
// scope (task-9.5 §3 OOS, nightly).
func TestTask83_AC3_BenchmarkRequires100kChunksAndSub500msP95(t *testing.T) {
	ok := BenchmarkReport{
		ChunkCount:    100000,
		BM25P95MS:     320,
		MetadataP95MS: 80,
		FilterP95MS:   140,
	}
	if err := CheckBenchmark(ok); err != nil {
		t.Fatalf("CheckBenchmark valid report: %v", err)
	}

	tooSmall := ok
	tooSmall.ChunkCount = 99999
	if err := CheckBenchmark(tooSmall); err == nil {
		t.Fatal("CheckBenchmark should reject reports below 100000 chunks")
	}

	tooSlow := ok
	tooSlow.FilterP95MS = 500
	if err := CheckBenchmark(tooSlow); err == nil {
		t.Fatal("CheckBenchmark should reject p95 >= 500ms")
	}
}

// ----------------------------------------------------------------------------
// Helpers shared with release_smoke_e2e_test.go (same package).
// ----------------------------------------------------------------------------

func findRepoRoot(t *testing.T) string {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatalf("getwd: %v", err)
	}
	for {
		if _, err := os.Stat(filepath.Join(dir, "go.mod")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatalf("repo root with go.mod not found from cwd")
		}
		dir = parent
	}
}

func exeName(base string) string {
	if runtime.GOOS == "windows" {
		return base + ".exe"
	}
	return base
}

func runIn(dir, name string, args ...string) ([]byte, error) {
	cmd := exec.Command(name, args...)
	cmd.Dir = dir
	return cmd.CombinedOutput()
}

func copyFile(src, dst string) error {
	data, err := os.ReadFile(src)
	if err != nil {
		return fmt.Errorf("read %s: %w", src, err)
	}
	if err := os.MkdirAll(filepath.Dir(dst), 0o755); err != nil {
		return err
	}
	return os.WriteFile(dst, data, 0o755)
}

func writeNonexecutableTarball(t *testing.T, path string) {
	t.Helper()
	f, err := os.Create(path)
	if err != nil {
		t.Fatalf("create tarball: %v", err)
	}
	defer f.Close()
	gz := gzip.NewWriter(f)
	defer gz.Close()
	tw := tar.NewWriter(gz)
	defer tw.Close()
	entries := map[string]int64{
		"contextforge":              0o644, // missing executable bit on purpose
		"contextforge-core":         0o755,
		"contextforge.example.toml": 0o644,
		"README.md":                 0o644,
		"LICENSE":                   0o644,
	}
	for name, mode := range entries {
		body := []byte(name + "\n")
		if err := tw.WriteHeader(&tar.Header{
			Name: name,
			Mode: mode,
			Size: int64(len(body)),
		}); err != nil {
			t.Fatalf("write header %s: %v", name, err)
		}
		if _, err := tw.Write(body); err != nil {
			t.Fatalf("write body %s: %v", name, err)
		}
	}
}
