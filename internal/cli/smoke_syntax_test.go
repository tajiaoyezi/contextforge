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

// TEST-25.3.1 / AC1: smoke v15 adds step 34 (task-25.3 closeout) — documents that the two production-scale
// ANN backends are verified at the Rust feature layer, not the console-api hot path: the qdrant server
// lifecycle layer (task-25.1, TEST-25.1.* — connect-config / health-probe / collection ensure-create
// decision, no live server) and the lancedb dev-box buildability + index-tuning param validation (task-25.2,
// TEST-25.2.* — 🟢 cargo build --features vector-lancedb on x86_64-pc-windows-msvc). The step asserts the
// default build is intact. ADR-013: feature-layer verification, no faked live-server / cross-platform path.
func TestTask253_SmokeV15ProductionVectorBackendStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v15") {
		t.Fatalf("console_smoke.sh missing v15 header (task-25.3 closeout)")
	}
	for _, marker := range []string{"[34/34]", "production vector backend", "TEST-25.1.", "TEST-25.2."} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v15 step 34 must document production vector backend status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (v13/v14 blocks intact; denominators untouched per ADR-014 D5 —
	// only the newest step carries the running total, matching the v14 [33/33] precedent).
	for _, marker := range []string{"[32/32]", "[33/33]", "vector persistence", "code/CJK tokenizer", "--hybrid"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v15 must not regress existing step marker %q", marker)
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

// TEST-26.3.1 / AC1: smoke v16 adds step 35 (task-26.3 closeout) — documents that Phase 26
// observability hardening (TraceStore FTS5 + VACUUM, task-26.1; events SSE push + audit replay,
// task-26.2; event-bus capacity/partition/drain config, task-26.3) is verified at the Rust + Go
// contract layers (TEST-26.1.* / TEST-26.2.* / TEST-26.3.*), not the console-api live path. The step
// asserts the default build is intact. ADR-013: contract-layer verification, real daemon SSE e2e deferred.
func TestTask263_SmokeV16ObservabilityHardeningStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v16") {
		t.Fatalf("console_smoke.sh missing v16 header (task-26.3 closeout)")
	}
	for _, marker := range []string{"[35/35]", "observability hardening", "TEST-26.1.", "TEST-26.2.", "TEST-26.3."} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v16 step 35 must document observability hardening status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (v13-v15 blocks intact; denominators untouched per ADR-014 D5 —
	// only the newest step carries the running total, matching the v14/v15 precedent).
	for _, marker := range []string{"[32/32]", "[33/33]", "[34/34]", "production vector backend", "vector persistence", "--hybrid"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v16 must not regress existing step marker %q", marker)
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

// TEST-27.3.1 / AC1: smoke v17 adds step 36 (task-27.3 closeout) — documents that Phase 27
// memory-ops hardening (pin-actor + pinned-at-timestamp, task-27.1; explicit Unpin + hard-delete,
// task-27.2; is_pinned audit backfill, task-27.3) is verified at the Rust + Go contract layers
// (TEST-27.1.* / TEST-27.2.* / TEST-27.3.1). In REAL mode step 36 exercises the live console-api
// round-trip (pin-actor projection + unpin 204 + hard-delete 412→204→404). ADR-013.
func TestTask273_SmokeV17MemoryOpsHardeningStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v17") {
		t.Fatalf("console_smoke.sh missing v17 header (task-27.3 closeout)")
	}
	for _, marker := range []string{"[36/36]", "memory ops hardening", "TEST-27.1.", "TEST-27.2.", "TEST-27.3.1", "/hard-delete"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v17 step 36 must document memory-ops hardening status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (v13-v16 blocks intact; denominators untouched per ADR-014 D5 —
	// only the newest step carries the running total, matching the v14/v15/v16 precedent).
	for _, marker := range []string{"[33/33]", "[34/34]", "[35/35]", "observability hardening", "production vector backend", "--hybrid"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v17 must not regress existing step marker %q", marker)
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

// TEST-28.4 / AC4: smoke v18 adds step 37 (task-28.4 closeout) — documents that Phase 28
// release-ci-hardening (anon-pull guard + multi-arch arm64 DEFERRED, task-28.1; cosign keyless
// sign + SBOM attest + provenance, task-28.2; CI strict-lint clippy+gofmt+go vet, task-28.3) is
// verified on CI + local registry. step 37 is a documentation/status step (release/CI hardening
// has no console-api runtime surface); it checks the default build still scaffolds (baseline
// unchanged, ADR-004). ADR-013: arm64 honestly DEFERRED, cosign real sign at the v0.21.0 release.
func TestTask284_SmokeV18ReleaseCiHardeningStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v18") {
		t.Fatalf("console_smoke.sh missing v18 header (task-28.4 closeout)")
	}
	for _, marker := range []string{"[37/37]", "release-ci-hardening", "TEST-28.1.", "TEST-28.2.", "TEST-28.3.", "cosign", "anon-pull"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v18 step 37 must document release-ci-hardening status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (v13-v17 blocks intact; denominators untouched per ADR-014 D5 —
	// only the newest step carries the running total [37/37], matching the v14-v17 precedent).
	for _, marker := range []string{"[33/33]", "[34/34]", "[35/35]", "[36/36]", "memory ops hardening"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v18 must not regress existing step marker %q", marker)
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

// TEST-29.4.1 / AC1: smoke v19 adds step 38 — task-29.4 closeout (live-vector-recall). Vector backends
// are feature-gated (no console-api runtime surface), so step 38 is a documentation/status step verifying
// the default build still scaffolds (ADR-004). Asserts the new [38/38] marker + Phase 29 status, and no
// regression of the prior denominators (ADR-014 D5 — only the newest step carries the running total).
func TestTask294_SmokeV19LiveVectorRecallStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v19") {
		t.Fatalf("console_smoke.sh missing v19 header (task-29.4 closeout)")
	}
	for _, marker := range []string{"[38/38]", "live-vector-recall", "TEST-29.1.", "TEST-29.2.", "TEST-29.3.", "honest-defer", "BruteForce"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v19 step 38 must document live-vector-recall status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (v13-v18 blocks intact; denominators untouched per ADR-014 D5).
	for _, marker := range []string{"[34/34]", "[35/35]", "[36/36]", "[37/37]", "release-ci-hardening"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v19 must not regress existing step marker %q", marker)
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

// TEST-34.3.2 / AC2: smoke v24 adds step 43 — task-34.3 closeout (vector-config-completeness). The
// dim-negotiation / config.toml [vector]→env bridge / get_source_chunk isolation guard all preserve
// default behavior, so step 43 is a documentation/status step verifying the default build still
// scaffolds. Asserts the new [43/43] marker + Phase 34 status, and no regression of the prior
// denominators (ADR-014 D5).
func TestTask343_SmokeV24VectorConfigCompletenessStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v24 (task-34.3)") {
		t.Fatalf("console_smoke.sh missing v24 (task-34.3) header block")
	}
	for _, marker := range []string{"[43/43]", "vector-config-completeness", "TEST-34.1.", "TEST-34.2.", "TEST-34.3.", "negotiate_vector_dim", "[vector]", "get_source_chunk"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v24 step 43 must document vector-config-completeness status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (v13-v23 blocks intact; denominators untouched per ADR-014 D5).
	for _, marker := range []string{"[37/37]", "[38/38]", "[39/39]", "[40/40]", "[41/41]", "[42/42]", "governance-debt-cleanup-2"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v24 must not regress existing step marker %q", marker)
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

// TEST-38.3.2 / AC2: smoke v28 adds step 47 — task-38.3 closeout (embedding-remote-reranker-live). Real
// remote cross-encoder rerank (Qwen3-VL-Reranker-8B via SiliconFlow /v1/rerank) MRR/recall@1 vs the
// IdentityReranker no-semantic baseline is measured by a local authenticated run (CI honest-defers —
// remote reranker is a paid external API with no free service container, unlike qdrant), closing
// ADR-026's embedding-remote-reranker-live defer; task-38.2 also adds the [reranker]→setRerankerEnv
// config bridge (API-key-env-only) + the first production data-plane opt-in with_reranker wiring. This
// preserves default behavior (reranker-remote opt-in; default unset = byte-equivalent no rerank), so
// step 47 is a documentation/status step verifying the default build still scaffolds. Asserts the new
// [47/47] marker + Phase 38 status, and no regression of the prior denominators (ADR-014 D5).
func TestTask383_SmokeV28RemoteRerankerLiveStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v28 (task-38.3)") {
		t.Fatalf("console_smoke.sh missing v28 (task-38.3) header block")
	}
	for _, marker := range []string{"[47/47]", "embedding-remote-reranker-live", "TEST-38.1.", "TEST-38.2.", "MRR", "remote_rerank_recall"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v28 step 47 must document embedding-remote-reranker-live status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (denominators untouched per ADR-014 D5).
	for _, marker := range []string{"[37/37]", "[38/38]", "[39/39]", "[40/40]", "[41/41]", "[42/42]", "[43/43]", "[44/44]", "[45/45]", "[46/46]", "embedding-provider-remote-live"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v28 must not regress existing step marker %q", marker)
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

// TEST-39.3.1 / AC1: smoke v29 adds step 48 — task-39.3 closeout (console-api-retrieval-signal-forward).
// In REAL mode POST /v1/search?hybrid=true is forwarded through console-api to the console data-plane
// hybrid dispatch (search_hybrid), and the response carries retrieval_method="hybrid" + hybrid_score;
// rerank `reason` provenance is visible end-to-end (reranker stays server-side env-driven, ?rerank
// superseded by ADR-043 D3). Asserts the new [48/48] marker + Phase 39 status, and no regression of the
// prior denominators (ADR-014 D5).
func TestTask393_SmokeV29ConsoleApiSignalForwardStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v29 (task-39.3)") {
		t.Fatalf("console_smoke.sh missing v29 (task-39.3) header block")
	}
	for _, marker := range []string{"[48/48]", "console-api-retrieval-signal-forward", "TEST-39.1.", "TEST-39.2.", "hybrid_score", "?hybrid=true"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v29 step 48 must document console-api-retrieval-signal-forward status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (denominators untouched per ADR-014 D5).
	for _, marker := range []string{"[37/37]", "[38/38]", "[39/39]", "[40/40]", "[41/41]", "[42/42]", "[43/43]", "[44/44]", "[45/45]", "[46/46]", "[47/47]", "embedding-remote-reranker-live"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v29 must not regress existing step marker %q", marker)
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

// TEST-40.3.1 / AC1: smoke v30 adds step 49 — task-40.3 closeout (governance-debt-cleanup-3). In REAL
// mode POST /v1/memory/{id}/pin with an X-Actor header propagates the caller through to pinned_by
// (caller-propagation; authenticated identity honest-deferred), and the L2 cache access-order LRU is
// verified via TEST-40.2.* in the default test gate. Asserts the new [49/49] marker + Phase 40 status,
// and no regression of the prior denominators (ADR-014 D5).
func TestTask403_SmokeV30GovernanceDebtCleanup3Step(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v30 (task-40.3)") {
		t.Fatalf("console_smoke.sh missing v30 (task-40.3) header block")
	}
	for _, marker := range []string{"[49/49]", "governance-debt-cleanup-3", "memory-actor-propagation", "l2-cache-true-lru", "TEST-40.1.", "TEST-40.2.", "X-Actor"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v30 step 49 must document governance-debt-cleanup-3 status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (denominators untouched per ADR-014 D5).
	for _, marker := range []string{"[37/37]", "[38/38]", "[39/39]", "[40/40]", "[41/41]", "[42/42]", "[43/43]", "[44/44]", "[45/45]", "[46/46]", "[47/47]", "[48/48]", "console-api-retrieval-signal-forward"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v30 must not regress existing step marker %q", marker)
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

// TEST-41.3.1 / AC1: smoke v31 adds step 50 — task-41.3 closeout (tokenizer-default-on). The code/CJK
// analyzer code_cjk flips from opt-in to the production default for newly created collections (the first
// deliberate default-behavior change, owned by ADR-046; existing collections unaffected; opt-out via
// CONTEXTFORGE_TOKENIZER=default / [retrieval] tokenizer). In REAL mode the camelCase subword "runner"
// (of JobRunner) hits via the code_cjk default (legacy TEXT would keep "jobrunner" single token → miss);
// TEST-41.1.* / TEST-41.2.* prove the flip + config bridge in the default cargo/go gate. Asserts the new
// [50/50] marker + Phase 41 status, and no regression of the prior denominators (ADR-014 D5).
func TestTask413_SmokeV31TokenizerDefaultOnStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v31 (task-41.3)") {
		t.Fatalf("console_smoke.sh missing v31 (task-41.3) header block")
	}
	for _, marker := range []string{"[50/50]", "tokenizer-default-on", "code_cjk", "CONTEXTFORGE_TOKENIZER", "TEST-41.1.", "TEST-41.2."} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v31 step 50 must document tokenizer-default-on status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (denominators untouched per ADR-014 D5).
	for _, marker := range []string{"[37/37]", "[38/38]", "[39/39]", "[40/40]", "[41/41]", "[42/42]", "[43/43]", "[44/44]", "[45/45]", "[46/46]", "[47/47]", "[48/48]", "[49/49]", "governance-debt-cleanup-3"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v31 must not regress existing step marker %q", marker)
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

// TEST-42.3.1 / AC1: smoke v32 adds step 51 — task-42.3 closeout (chunk-source-type-filter). The chunk
// source_type filter Phase 32 recorded as a documented no-op becomes a REAL filter: source_type is derived
// from file_path (classify_source_type), populated on every hit, and post-filtered (0 schema migration,
// §5.3 FROZEN); console-api forwards ?source_type= (proto add-only source_type=9). In REAL mode
// source_type=code keeps the JobRunner code hit (source_file_type=code) and source_type=doc filters it out
// (distinguishing — a no-op would still return it); TEST-42.1.* / TEST-42.2.* prove derive+filter+forward in
// the default cargo/go gate. agent_scope stays a documented no-op (memory-layer; honest-deferred). Asserts the
// new [51/51] marker + Phase 42 status, and no regression of the prior denominators (ADR-014 D5).
func TestTask423_SmokeV32ChunkSourceTypeFilterStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v32 (task-42.3)") {
		t.Fatalf("console_smoke.sh missing v32 (task-42.3) header block")
	}
	for _, marker := range []string{"[51/51]", "chunk-source-type-filter", "source_type", "classify_source_type", "TEST-42.1.", "TEST-42.2."} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v32 step 51 must document chunk-source-type-filter status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (denominators untouched per ADR-014 D5).
	for _, marker := range []string{"[37/37]", "[38/38]", "[39/39]", "[40/40]", "[41/41]", "[42/42]", "[43/43]", "[44/44]", "[45/45]", "[46/46]", "[47/47]", "[48/48]", "[49/49]", "[50/50]", "tokenizer-default-on"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v32 must not regress existing step marker %q", marker)
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

// TEST-43.3.1a / AC1: smoke v33 adds step 52 — task-43.3 closeout (governance-debt-cleanup-4 /
// indexing-replay-splice). Phase 33 task-33.3 wrote the indexing replay mapper (indexing_rows_to_pb_events) +
// persistent store (migration 0019) but never wired it into the live subscribe path. task-43.1 closes the 4
// splice gaps (list_since + DataPlaneStores field + serve_full wiring + subscribe splice), so since_ts>0
// subscribers now receive missed indexing.* lifecycle events (the indexing counterpart of the task-26.2 audit
// replay). The splice is unit-verified by TEST-43.1.1/.2a/.2b/.2c in the default cargo test gate (lib 225→229);
// REAL-mode subscribe-stream e2e is honest-deferred [SPEC-DEFER:phase-future.indexing-replay-daemon-e2e] (needs
// a running daemon + cross-restart dual-window assertion, ADR-013). memory-actor-all-rpc (not a small debt) is
// honest-deferred to an independent phase. 0 new dep / 0 migration (reuses 0019) / 0 proto / default byte-equiv.
func TestTask433_SmokeV33IndexingReplaySpliceStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v33 (task-43.3)") {
		t.Fatalf("console_smoke.sh missing v33 (task-43.3) header block")
	}
	for _, marker := range []string{"[52/52]", "indexing-replay-splice", "list_since", "indexing_event_store", "TEST-43.1."} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v33 step 52 must document indexing-replay-splice status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (denominators untouched per ADR-014 D5).
	for _, marker := range []string{"[37/37]", "[38/38]", "[39/39]", "[40/40]", "[41/41]", "[42/42]", "[43/43]", "[44/44]", "[45/45]", "[46/46]", "[47/47]", "[48/48]", "[49/49]", "[50/50]", "[51/51]", "chunk-source-type-filter"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v33 must not regress existing step marker %q", marker)
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

// TEST-37.3.2 / AC2: smoke v27 adds step 46 — task-37.3 closeout (embedding-provider-remote-live). Real
// remote embedding (Qwen3-Embedding-8B via an OpenAI-compatible endpoint) semantic recall@k vs the
// deterministic baseline is measured by a local authenticated run (CI honest-defers — remote is a paid
// external API with no free service container, unlike qdrant), closing ADR-027's embedding-provider-remote
// defer; the [remote]→setRemoteEnv config bridge is API-key-env-only. This preserves default behavior
// (embedding-remote opt-in, default build 0-network), so step 46 is a documentation/status step verifying
// the default build still scaffolds. Asserts the new [46/46] marker + Phase 37 status, and no regression
// of the prior denominators (ADR-014 D5).
func TestTask373_SmokeV27RemoteEmbeddingLiveStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v27 (task-37.3)") {
		t.Fatalf("console_smoke.sh missing v27 (task-37.3) header block")
	}
	for _, marker := range []string{"[46/46]", "embedding-provider-remote-live", "TEST-37.1.", "TEST-37.2.", "recall@3", "remote_embedding_recall"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v27 step 46 must document embedding-provider-remote-live status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (denominators untouched per ADR-014 D5).
	for _, marker := range []string{"[37/37]", "[38/38]", "[39/39]", "[40/40]", "[41/41]", "[42/42]", "[43/43]", "[44/44]", "[45/45]", "qdrant-live-vector-recall"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v27 must not regress existing step marker %q", marker)
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

// TEST-36.3.2 / AC2: smoke v26 adds step 45 — task-36.3 closeout (qdrant-live-vector-recall). The
// qdrant live KNN recall@k vs BruteForce exact KNN is measured on every CI run via a qdrant service-
// container (recall@10=1.0000), closing ADR-034 D2's qdrant-server-lifecycle defer; this preserves
// default behavior (vector-qdrant opt-in, default build 0-vector-dep), so step 45 is a documentation/
// status step verifying the default build still scaffolds. Asserts the new [45/45] marker + Phase 36
// status, and no regression of the prior denominators (ADR-014 D5).
func TestTask363_SmokeV26QdrantLiveRecallStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v26 (task-36.3)") {
		t.Fatalf("console_smoke.sh missing v26 (task-36.3) header block")
	}
	for _, marker := range []string{"[45/45]", "qdrant-live-vector-recall", "TEST-36.1.", "TEST-36.2.", "recall@10", "qdrant_live_recall"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v26 step 45 must document qdrant-live-vector-recall status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (denominators untouched per ADR-014 D5).
	for _, marker := range []string{"[37/37]", "[38/38]", "[39/39]", "[40/40]", "[41/41]", "[42/42]", "[43/43]", "[44/44]", "observability-hardening"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v26 must not regress existing step marker %q", marker)
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

// TEST-35.3.2 / AC2: smoke v25 adds step 44 — task-35.3 closeout (observability-hardening). The
// rust/go silent-failure surfacing (eprintln! / fmt.Fprintf(os.Stderr)) + 7→3-4 grounding correction
// all preserve default behavior (observability-only; best-effort is never turned into fail-fast), so
// step 44 is a documentation/status step verifying the default build still scaffolds. Asserts the new
// [44/44] marker + Phase 35 status, and no regression of the prior denominators (ADR-014 D5).
func TestTask353_SmokeV25ObservabilityHardeningStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v25 (task-35.3)") {
		t.Fatalf("console_smoke.sh missing v25 (task-35.3) header block")
	}
	for _, marker := range []string{"[44/44]", "observability-hardening", "TEST-35.1.", "TEST-35.2.", "TEST-35.3.", "eprintln", "setVectorEnv"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v25 step 44 must document observability-hardening status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (v13-v24 blocks intact; denominators untouched per ADR-014 D5).
	for _, marker := range []string{"[37/37]", "[38/38]", "[39/39]", "[40/40]", "[41/41]", "[42/42]", "[43/43]", "vector-config-completeness"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v25 must not regress existing step marker %q", marker)
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

// TEST-33.4.2 / AC2: smoke v23 adds step 42 — task-33.4 closeout (governance-debt-cleanup-2). The L2
// cache bound / memstore LRU / indexing replay+trace isolation (add-only proto+migration) / export
// --timeout (add-only flag) all preserve default behavior, so step 42 is a documentation/status step
// verifying the default build still scaffolds. Asserts the new [42/42] marker + Phase 33 status, and no
// regression of the prior denominators (ADR-014 D5).
func TestTask334_SmokeV23GovernanceDebtCleanup2Step(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v23 (task-33.4)") {
		t.Fatalf("console_smoke.sh missing v23 (task-33.4) header block")
	}
	for _, marker := range []string{"[42/42]", "governance-debt-cleanup-2", "TEST-33.1.", "TEST-33.2.", "TEST-33.3.", "TEST-33.4.", "export --timeout", "migration 0019", "workspace_id"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v23 step 42 must document governance-debt-cleanup-2 status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (v13-v22 blocks intact; denominators untouched per ADR-014 D5).
	for _, marker := range []string{"[37/37]", "[38/38]", "[39/39]", "[40/40]", "[41/41]", "vector-backend-config-plumbing"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v23 must not regress existing step marker %q", marker)
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

// TEST-32.4.1 / AC1: smoke v22 adds step 41 — task-32.4 closeout (vector-backend-config-plumbing-and-
// completeness). The vector backends + sqlite-vec arm are feature-gated and the console vector_score is
// add-only, so step 41 is a documentation/status step verifying the default build still scaffolds.
// Asserts the new [41/41] marker + Phase 32 status, and no regression of the prior denominators (ADR-014 D5).
func TestTask324_SmokeV22VectorBackendConfigStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v22 (task-32.4)") {
		t.Fatalf("console_smoke.sh missing v22 (task-32.4) header block")
	}
	for _, marker := range []string{"[41/41]", "vector-backend-config-plumbing", "TEST-32.1.", "TEST-32.2.", "TEST-32.3.", "sqlite-vec", "vector_score"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v22 step 41 must document vector-backend-config-plumbing status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (v13-v21 blocks intact; denominators untouched per ADR-014 D5).
	for _, marker := range []string{"[37/37]", "[38/38]", "[39/39]", "[40/40]", "governance-debt-cleanup"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v22 must not regress existing step marker %q", marker)
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

// TEST-31.4.1 / AC1: smoke v21 adds step 40 — task-31.4 closeout (governance-debt-cleanup). A
// documentation/status step verifying the default build still scaffolds. Asserts the new [40/40]
// marker + Phase 31 status, and no regression of the prior denominators (ADR-014 D5).
func TestTask314_SmokeV21GovernanceDebtCleanupStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v21") {
		t.Fatalf("console_smoke.sh missing v21 header (task-31.4 closeout)")
	}
	for _, marker := range []string{"[40/40]", "governance-debt-cleanup", "TEST-31.1.", "TEST-31.2.", "TEST-31.3.", "memstore-event", "ListAllChunks"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v21 step 40 must document governance-debt-cleanup status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (v13-v20 blocks intact; denominators untouched per ADR-014 D5).
	for _, marker := range []string{"[36/36]", "[37/37]", "[38/38]", "[39/39]", "cjk-true-segmenter"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v21 must not regress existing step marker %q", marker)
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

// TEST-30.3.1 / AC1: smoke v20 adds step 39 — task-30.3 closeout (cjk-true-segmenter). The segmenter
// is feature-gated (no console-api runtime surface), so step 39 is a documentation/status step verifying
// the default build still scaffolds with the 0-dep bigram fallback (ADR-004). Asserts the new [39/39]
// marker + Phase 30 status, and no regression of the prior denominators (ADR-014 D5).
func TestTask303_SmokeV20CjkTrueSegmenterStep(t *testing.T) {
	script := filepath.Join("..", "..", "scripts", "console_smoke.sh")
	raw, err := os.ReadFile(script)
	if err != nil {
		t.Fatalf("read %s: %v", script, err)
	}
	body := string(raw)
	if !strings.Contains(body, "v20") {
		t.Fatalf("console_smoke.sh missing v20 header (task-30.3 closeout)")
	}
	for _, marker := range []string{"[39/39]", "cjk-true-segmenter", "TEST-30.1.", "TEST-30.2.", "reindex", "jieba"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v20 step 39 must document cjk-true-segmenter status (missing %q)", marker)
		}
	}
	// No regression of the prior steps (v13-v19 blocks intact; denominators untouched per ADR-014 D5).
	for _, marker := range []string{"[35/35]", "[36/36]", "[37/37]", "[38/38]", "live-vector-recall"} {
		if !strings.Contains(body, marker) {
			t.Fatalf("smoke v20 must not regress existing step marker %q", marker)
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
// (core/src/server.rs test_21_1_hybrid_dispatches_fusion_path); the console-api ?hybrid REST forward is
// now fulfilled in Phase 39 (task-39.2; smoke step 48 / TestTask393), while ?rerank stays server-side
// env-driven [SPEC-DEFER:phase-future.console-api-rerank-forward] (per-request superseded by ADR-043 D3).
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
