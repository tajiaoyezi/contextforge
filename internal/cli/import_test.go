// task-9.4 import_test — TEST-9.4.1 .. 9.4.5 covering AC1-AC5.

package cli

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

const fixtureRoot = "../../test/fixtures/import-cli"

// hermes fixture absolute path.
func hermesFixtureDir(t *testing.T) string {
	t.Helper()
	p := filepath.Join(fixtureRoot, "hermes")
	if _, err := os.Stat(p); err != nil {
		t.Skipf("hermes fixture %s missing: %v", p, err)
	}
	abs, _ := filepath.Abs(p)
	return abs
}

func openclawFixtureDir(t *testing.T) string {
	t.Helper()
	p := filepath.Join(fixtureRoot, "openclaw", "sample")
	if _, err := os.Stat(p); err != nil {
		t.Skipf("openclaw fixture %s missing: %v", p, err)
	}
	abs, _ := filepath.Abs(p)
	return abs
}

func agentRulesFixtureFile(t *testing.T) string {
	t.Helper()
	p := filepath.Join(fixtureRoot, "agent-rules", "AGENTS.md")
	if _, err := os.Stat(p); err != nil {
		t.Skipf("agent-rules fixture %s missing: %v", p, err)
	}
	abs, _ := filepath.Abs(p)
	return abs
}

// TEST-9.4.1 / AC1 — hermes import writes 2 .md files (MEMORY + USER) with
// frontmatter `source_provider: hermes` + body content.
func TestTask94_AC1_HermesImport(t *testing.T) {
	dataDir := t.TempDir()
	source := hermesFixtureDir(t)

	var stdout, stderr bytes.Buffer
	code := runImport([]string{"hermes", source,
		"--collection=demo",
		"--data-dir=" + dataDir,
	}, &stdout, &stderr)
	if code != 0 {
		t.Fatalf("AC1: exit=%d stderr=%q", code, stderr.String())
	}
	out := stdout.String()
	if !strings.Contains(out, "imported") || !strings.Contains(out, "records to") {
		t.Fatalf("AC1: summary missing in stdout:\n%s", out)
	}
	if !strings.Contains(out, "next: contextforge index --source") {
		t.Fatalf("AC1: next-step hint missing:\n%s", out)
	}

	outDir := filepath.Join(dataDir, "imports", "hermes")
	files, err := os.ReadDir(outDir)
	if err != nil {
		t.Fatalf("AC1: read output dir: %v", err)
	}
	if len(files) < 2 {
		t.Fatalf("AC1: expected ≥2 .md (MEMORY + USER), got %d", len(files))
	}
	var anyHadProvider bool
	for _, f := range files {
		body, err := os.ReadFile(filepath.Join(outDir, f.Name()))
		if err != nil {
			t.Fatalf("read %s: %v", f.Name(), err)
		}
		if !strings.HasPrefix(string(body), "---\n") {
			t.Fatalf("AC1: %s missing YAML frontmatter prefix", f.Name())
		}
		if strings.Contains(string(body), "source_provider: hermes") {
			anyHadProvider = true
		}
	}
	if !anyHadProvider {
		t.Fatalf("AC1: no output file had 'source_provider: hermes' frontmatter")
	}
}

// TEST-9.4.2 / AC2 — openclaw import writes ≥1 .md with `source_provider: openclaw`.
func TestTask94_AC2_OpenClawImport(t *testing.T) {
	dataDir := t.TempDir()
	source := openclawFixtureDir(t)

	var stdout, stderr bytes.Buffer
	code := runImport([]string{"openclaw", source,
		"--collection=demo",
		"--data-dir=" + dataDir,
	}, &stdout, &stderr)
	if code != 0 {
		t.Fatalf("AC2: exit=%d stderr=%q", code, stderr.String())
	}

	outDir := filepath.Join(dataDir, "imports", "openclaw")
	files, err := os.ReadDir(outDir)
	if err != nil {
		t.Fatalf("AC2: read output dir: %v", err)
	}
	if len(files) < 1 {
		t.Fatalf("AC2: expected ≥1 .md, got %d", len(files))
	}
	var anyHadProvider bool
	for _, f := range files {
		body, _ := os.ReadFile(filepath.Join(outDir, f.Name()))
		if strings.Contains(string(body), "source_provider: openclaw") {
			anyHadProvider = true
		}
	}
	if !anyHadProvider {
		t.Fatalf("AC2: no output file had 'source_provider: openclaw' frontmatter")
	}
}

// TEST-9.4.3 / AC3 — agent-rules import writes .md with `source_type: agent_rule`.
func TestTask94_AC3_AgentRulesImport(t *testing.T) {
	dataDir := t.TempDir()
	source := agentRulesFixtureFile(t)

	var stdout, stderr bytes.Buffer
	code := runImport([]string{"agent-rules", source,
		"--collection=demo",
		"--data-dir=" + dataDir,
	}, &stdout, &stderr)
	if code != 0 {
		t.Fatalf("AC3: exit=%d stderr=%q", code, stderr.String())
	}

	outDir := filepath.Join(dataDir, "imports", "agent-rules")
	files, err := os.ReadDir(outDir)
	if err != nil {
		t.Fatalf("AC3: read output dir: %v", err)
	}
	if len(files) < 1 {
		t.Fatalf("AC3: expected ≥1 .md, got %d", len(files))
	}
	var anyHadType bool
	for _, f := range files {
		body, _ := os.ReadFile(filepath.Join(outDir, f.Name()))
		if strings.Contains(string(body), "source_type: agent_rule") {
			anyHadType = true
		}
	}
	if !anyHadType {
		t.Fatalf("AC3: no output file had 'source_type: agent_rule' frontmatter")
	}
}

// TEST-9.4.4 / AC4 — unknown importer → exit 2 + stderr usage hint.
func TestTask94_AC4_UnknownImporter(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runImport([]string{"unknown-source", "/tmp/whatever",
		"--collection=demo",
	}, &stdout, &stderr)
	if code != 2 {
		t.Fatalf("AC4: expected exit 2, got %d", code)
	}
	stderrStr := stderr.String()
	if !strings.Contains(stderrStr, "unknown importer: unknown-source") {
		t.Fatalf("AC4: expected 'unknown importer' error, got: %s", stderrStr)
	}
	if !strings.Contains(stderrStr, "hermes") || !strings.Contains(stderrStr, "openclaw") || !strings.Contains(stderrStr, "agent-rules") {
		t.Fatalf("AC4: expected usage to list valid names, got: %s", stderrStr)
	}
}

// TEST-9.4.4b — missing positional path → exit 2.
func TestTask94_AC4_MissingPathArg(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runImport([]string{"hermes",
		"--collection=demo",
	}, &stdout, &stderr)
	if code != 2 {
		t.Fatalf("AC4: expected exit 2, got %d stderr=%q", code, stderr.String())
	}
}

// TEST-9.4.5 / AC5 — --dry-run doesn't write files but stdout still has summary + next.
func TestTask94_AC5_DryRun(t *testing.T) {
	dataDir := t.TempDir()
	source := hermesFixtureDir(t)

	var stdout, stderr bytes.Buffer
	code := runImport([]string{"hermes", source,
		"--collection=demo",
		"--data-dir=" + dataDir,
		"--dry-run",
	}, &stdout, &stderr)
	if code != 0 {
		t.Fatalf("AC5: exit=%d stderr=%q", code, stderr.String())
	}
	out := stdout.String()
	if !strings.Contains(out, "next: contextforge index --source") {
		t.Fatalf("AC5: --dry-run should still print next-step hint, got:\n%s", out)
	}
	if !strings.Contains(out, "no files were written") {
		t.Fatalf("AC5: --dry-run should advertise itself, got:\n%s", out)
	}
	outDir := filepath.Join(dataDir, "imports", "hermes")
	if _, err := os.Stat(outDir); err == nil {
		// Output dir may exist if some other run created it; assert no .md files in it.
		files, _ := os.ReadDir(outDir)
		for _, f := range files {
			if strings.HasSuffix(f.Name(), ".md") {
				t.Fatalf("AC5: --dry-run wrote %s, expected none", f.Name())
			}
		}
	}
}

// Dispatch wire — `contextforge import` reaches runImport (not "not implemented").
func TestImportSubcommand_DispatchWired(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := Execute([]string{"import"}, &stdout, &stderr)
	// Empty args → exit 2 usage, not "not implemented" placeholder.
	if code != 2 {
		t.Fatalf("dispatch: expected exit 2, got %d", code)
	}
	stderrStr := stderr.String()
	if strings.Contains(stderrStr, "not implemented") {
		t.Fatalf("dispatch: import should be wired (no 'not implemented'), got: %s", stderrStr)
	}
}
