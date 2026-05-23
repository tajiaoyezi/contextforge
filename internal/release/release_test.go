package release

import (
	"archive/tar"
	"compress/gzip"
	"os"
	"path/filepath"
	"testing"
)

// TEST-8.3.1 / SCEN-8.3.1 / AC1
func TestTask83_AC1_TarballContainsRequiredAssets(t *testing.T) {
	tarball := filepath.Join(t.TempDir(), "contextforge-linux-amd64.tar.gz")
	writeTarball(t, tarball, map[string]int64{
		"contextforge":               0o755,
		"contextforge-core":          0o755,
		"contextforge.example.toml":  0o644,
		"README.md":                  0o644,
		"LICENSE":                    0o644,
		"extra/release-metadata.txt": 0o644,
	})

	report, err := ValidateTarball(tarball)
	if err != nil {
		t.Fatalf("ValidateTarball: %v", err)
	}
	if report.Name != "contextforge-linux-amd64.tar.gz" {
		t.Fatalf("report name=%q", report.Name)
	}
	if len(report.Entries) < len(RequiredTarballEntries()) {
		t.Fatalf("entries=%v, want required entries", report.Entries)
	}

	bad := filepath.Join(t.TempDir(), "contextforge-linux-amd64.tar.gz")
	writeTarball(t, bad, map[string]int64{
		"contextforge":              0o644,
		"contextforge-core":         0o755,
		"contextforge.example.toml": 0o644,
		"README.md":                 0o644,
		"LICENSE":                   0o644,
	})
	if _, err := ValidateTarball(bad); err == nil {
		t.Fatal("ValidateTarball should reject non-executable contextforge")
	}
}

// TEST-8.3.2 / SCEN-8.3.2 / AC2
func TestTask83_AC2_ReleaseSmokeEvidenceRequiresOrderedPassingSteps(t *testing.T) {
	var evidence []StepResult
	for _, step := range RequiredSteps() {
		evidence = append(evidence, StepResult{Name: step, Status: StepPassed, Evidence: step + " ok"})
	}
	if err := ValidateSmokeEvidence(evidence); err != nil {
		t.Fatalf("ValidateSmokeEvidence valid sequence: %v", err)
	}

	missingExport := append([]StepResult(nil), evidence...)
	missingExport = append(missingExport[:6], missingExport[7:]...)
	if err := ValidateSmokeEvidence(missingExport); err == nil {
		t.Fatal("ValidateSmokeEvidence should reject missing export step")
	}

	failed := append([]StepResult(nil), evidence...)
	failed[3].Status = StepFailed
	if err := ValidateSmokeEvidence(failed); err == nil {
		t.Fatal("ValidateSmokeEvidence should reject failed index step")
	}
}

// TEST-8.3.3 / SCEN-8.3.3 / AC3
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

// TEST-8.3.4 / SCEN-8.3.4 / AC4
func TestTask83_AC4_V01ClosureRequiresSevenTechnicalAreas(t *testing.T) {
	evidence := []StepResult{
		{Name: StepImport, Status: StepPassed, Evidence: "import source"},
		{Name: StepIndex, Status: StepPassed, Evidence: "index source"},
		{Name: StepSearch, Status: StepPassed, Evidence: "CLI/API search"},
		{Name: StepMCP, Status: StepPassed, Evidence: "MCP tools/list"},
		{Name: StepExplain, Status: StepPassed, Evidence: "explainable retrieval fields"},
		{Name: StepEval, Status: StepPassed, Evidence: "eval run"},
		{Name: StepReliability, Status: StepPassed, Evidence: "resume/resource guard"},
	}
	if err := ValidateV01Closure(evidence); err != nil {
		t.Fatalf("ValidateV01Closure valid evidence: %v", err)
	}

	evidence[4].Status = StepFailed
	if err := ValidateV01Closure(evidence); err == nil {
		t.Fatal("ValidateV01Closure should reject failed explainability evidence")
	}
}

// TEST-8.3.5 / SCEN-8.3.5 / AC5
func TestTask83_AC5_PhaseSmokeReportCombinesTarballSmokeAndBenchmark(t *testing.T) {
	report := PhaseSmokeReport{
		Tarball: TarballReport{Entries: RequiredTarballEntries()},
		Smoke: []StepResult{
			{Name: StepUnpack, Status: StepPassed, Evidence: "untar ok"},
			{Name: StepInit, Status: StepPassed, Evidence: "init ok"},
			{Name: StepImport, Status: StepPassed, Evidence: "import ok"},
			{Name: StepIndex, Status: StepPassed, Evidence: "index ok"},
			{Name: StepSearch, Status: StepPassed, Evidence: "search ok"},
			{Name: StepMCP, Status: StepPassed, Evidence: "mcp ok"},
			{Name: StepExport, Status: StepPassed, Evidence: "export ok"},
			{Name: StepEval, Status: StepPassed, Evidence: "eval ok"},
		},
		Closure: []StepResult{
			{Name: StepImport, Status: StepPassed, Evidence: "import source"},
			{Name: StepIndex, Status: StepPassed, Evidence: "index source"},
			{Name: StepSearch, Status: StepPassed, Evidence: "CLI/API search"},
			{Name: StepMCP, Status: StepPassed, Evidence: "MCP tools/list"},
			{Name: StepExplain, Status: StepPassed, Evidence: "explainable retrieval"},
			{Name: StepEval, Status: StepPassed, Evidence: "eval run"},
			{Name: StepReliability, Status: StepPassed, Evidence: "resume/resource"},
		},
		Benchmark: BenchmarkReport{ChunkCount: 100000, BM25P95MS: 250, MetadataP95MS: 70, FilterP95MS: 120},
	}
	if err := ValidatePhaseSmoke(report); err != nil {
		t.Fatalf("ValidatePhaseSmoke: %v", err)
	}
}

func writeTarball(t *testing.T, path string, entries map[string]int64) {
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
