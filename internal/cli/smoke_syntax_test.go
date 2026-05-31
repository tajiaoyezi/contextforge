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
	for _, marker := range []string{"[29/32]", "[30/32]"} {
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

// TEST-22.4.1 / AC1: smoke v12 adds step 31 (task-22.4 closeout) — `contextforge init` emits the
// add-only [embedding] config section (task-22.1). The step greps the generated config.toml for the
// [embedding] header + the `dim` key (unique to that section) + an intact [remote] header. ADR-013:
// no real network — cache/remote are unit/contract-verified, not console-hot-path-wired in v0.15.
func TestTask224_SmokeV12EmbeddingConfigStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v12") {
		t.Fatalf("console_smoke.sh missing v12 header (task-22.4 closeout)")
	}
	for _, marker := range []string{"[31/32]", "init --root", `grep -q '^\[embedding\]'`} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v12 step 31 must assert init emits [embedding] config (missing %q)", marker)
		}
	}
	// No regression of the v9/v10/v11 steps (renumbered to /31).
	for _, marker := range []string{"[29/32]", "[30/32]", "vector-bruteforce", "--hybrid", "--rerank"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v12 must not regress existing step marker %q", marker)
		}
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

// TEST-23.3.1 / AC1: smoke v13 adds step 32 (task-23.3 closeout) — documents that Phase 23 vector
// persistence (hnsw save/load, task-23.1) + cross-platform (sqlite-vec Windows MSVC, task-23.2) are
// verified at the Rust feature layer (TEST-23.1.* / TEST-23.2.*), not the console-api hot path. The
// step asserts the default build is intact (init scaffold). ADR-013: feature-layer verification, no
// faked console persistence path.
func TestTask233_SmokeV13VectorPersistenceStatusStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v13") {
		t.Fatalf("console_smoke.sh missing v13 header (task-23.3 closeout)")
	}
	for _, marker := range []string{"[32/32]", "vector persistence", "TEST-23.1.", "TEST-23.2."} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v13 step 32 must document vector persistence / cross-platform status (missing %q)", marker)
		}
	}
	// No regression of the v9-v12 steps (renumbered to /32).
	for _, marker := range []string{"[29/32]", "[30/32]", "[31/32]", "vector-bruteforce", "--hybrid", `grep -q '^\[embedding\]'`} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v13 must not regress existing step marker %q", marker)
		}
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

// TEST-24.3.1 / AC1: smoke v14 adds step 33 (task-24.3 closeout) — documents that the opt-in code/CJK
// tokenizer (task-24.1) + the eval golden-dataset validator + code/CJK golden 扩充 (task-24.2) are verified
// at the Rust indexer + Go eval layers (TEST-24.1.* / TEST-24.2.*), not the console-api hot path. The step
// asserts the default build is intact. ADR-013: feature/config-layer verification, no faked console path.
func TestTask243_SmokeV14TokenizerEvalHardeningStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v14") {
		t.Fatalf("console_smoke.sh missing v14 header (task-24.3 closeout)")
	}
	for _, marker := range []string{"[33/33]", "code/CJK tokenizer", "TEST-24.1.", "TEST-24.2."} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v14 step 33 must document tokenizer / eval hardening status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (v13 block intact, denominators untouched per ADR-014 D5).
	for _, marker := range []string{"[32/32]", "[31/32]", "[30/32]", "vector persistence", "--hybrid"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v14 must not regress existing step marker %q", marker)
		}
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

// TEST-21.3.2 / AC2: smoke v11 upgrades step 30 from `eval run --semantic` to
// `eval run --semantic --hybrid --rerank`, asserting the add-only hybrid (req.Hybrid → daemon
// search_hybrid, task-21.1) + reranked (eval-layer deterministic IdentityReranker, ADR-026 D2) eval
// passes engage end-to-end. ADR-013: report shape + gate only (the transient eval index is empty;
// real hybrid/rerank recall is docs/spikes/phase-21-hybrid-recall.md). Existing steps unchanged; the
// per-result retrieval_method="hybrid" + hybrid_score provenance is asserted by the Rust dispatch test
// (core/src/server.rs test_21_1_hybrid_dispatches_fusion_path); console-api ?hybrid/?rerank REST
// forward stays [SPEC-DEFER:phase-future.console-api-hybrid-forward].
func TestTask213_SmokeV11HybridRerankAssertion(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v11") {
		t.Fatalf("console_smoke.sh missing v11 header (task-21.3 closeout)")
	}
	for _, marker := range []string{"--hybrid", "--rerank", "hybrid_recall_at_10=", "reranked_recall_at_10="} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v11 step 30 must assert the hybrid/rerank eval path (missing %q)", marker)
		}
	}
	// No regression of the v9/v10 steps.
	for _, marker := range []string{"[29/32]", "[30/32]", "vector-bruteforce"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v11 must not regress existing step marker %q", marker)
		}
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
