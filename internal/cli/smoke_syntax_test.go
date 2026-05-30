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
