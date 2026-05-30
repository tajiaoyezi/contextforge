package cli

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// TEST-19.4.3 / AC4: scripts/console_smoke.sh v9 passes `bash -n` (syntax check) and carries the
// new step 29 (/v1/search?semantic=true roundtrip) + step 30 (eval run --semantic) headers. Skips
// when bash is unavailable (Windows dev machines without Git Bash); CI (Linux) always runs it.
func TestTask194_AC4_SmokeV9SyntaxAndSteps(t *testing.T) {
	// internal/cli → repo root is two levels up.
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	for _, marker := range []string{"[29/30]", "[30/30]"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("console_smoke.sh missing step marker %q (v9 migrates [N/28]→[N/30] + adds steps 29/30)", marker)
		}
	}
	// No stale /28 step headers should remain after the v9 migration.
	if strings.Contains(body, "/28]") {
		t.Fatalf("console_smoke.sh still has /28] step headers; v9 must migrate all to /30]")
	}

	bash, err := exec.LookPath("bash")
	if err != nil {
		t.Skip("bash not in PATH — skipping `bash -n` syntax check (CI Linux runs it)")
	}
	out, err := exec.Command(bash, "-n", script).CombinedOutput()
	if err != nil {
		t.Fatalf("bash -n %s failed: %v\n%s", script, err, out)
	}
}

// TEST-20.3.1 / AC1: smoke v10 upgrades step 29 from a shape-only ({result, trace}) assertion to a
// real semantic-engagement assertion — after task-20.1 wired console-api ?semantic=true forwarding,
// step 29 now greps the trace for the vector path (candidate_generation_steps=vector-bruteforce),
// proving the semantic branch actually engaged through console-api (not only that the add-only param
// is non-breaking). ADR-013: still no recall-threshold assertion in the smoke.
func TestTask203_SmokeV10SemanticEngagementAssertion(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v10") {
		t.Fatalf("console_smoke.sh missing v10 header (task-20.3 closeout)")
	}
	// The v10 step-29 semantic-engagement assertion greps the trace for the vector path.
	if !strings.Contains(body, "vector-bruteforce") {
		t.Fatalf("smoke v10 step 29 must assert the vector path engaged (grep candidate_generation_steps=vector-bruteforce)")
	}
}
