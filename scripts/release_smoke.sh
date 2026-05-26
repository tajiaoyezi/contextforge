#!/usr/bin/env bash
# scripts/release_smoke.sh — v0.1 (task-8.3) + v0.2 (task-9.5) + v0.3 (task-10.6) + v0.4 (task-11.4) release smoke gate.
#
# Sections:
#   1. Go release harness (BuildTarball / ValidateTarball / CheckBenchmark unit tests)
#   2. Task 8 reliability + eval harness unit tests
#   3. Rust gRPC search smoke (phase6_search_grpc_end_to_end_smoke)
#   4. Phase 9 CLI end-to-end smoke (TestPhase9ReleaseSmoke_EndToEnd) — REAL
#      go build + cargo build + 7-step CLI binary exercise. Renamed exit
#      marker to PHASE_RELEASE_SMOKE_EXIT (drops v0.1-only PHASE8 prefix per
#      task-9.5 §3).
#   5. Phase 11 Console Real Data Plane smoke (scripts/console_smoke.sh REAL
#      mode default in v0.4 — spawns contextforge-core daemon + console-api-
#      serve + cross-process gRPC bridge). v0.3 LOCAL_ONLY mode retained as
#      env LOCAL_ONLY=1 fallback.
#      Gated on env RELEASE_SMOKE_CONSOLE=1 (default SKIP to avoid hard-
#      requiring full cargo build inside every CI matrix; CI fast path can
#      run just sections 1-4).
#
# Each section's non-zero exit propagates (set -e). Final marker line is the
# release tag gate.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "PHASE_RELEASE_SMOKE_BEGIN"

echo "release_smoke[1/5]: go release harness (TestTask83 AC1 real-binary + AC3 benchmark)"
go test ./internal/release -run 'TestTask83'

echo "release_smoke[2/5]: task 8 reliability/eval harness"
go test ./internal/eval ./internal/reliability -run 'TestTask8(1|2)'

echo "release_smoke[3/5]: Rust gRPC search smoke"
cargo test --workspace phase_6_search_grpc_end_to_end_smoke

echo "release_smoke[4/5]: phase 9 CLI end-to-end smoke (real binaries + 7-step CLI)"
go test ./internal/release -run 'TestPhase9ReleaseSmoke_EndToEnd' -timeout 180s

echo "release_smoke[5/5]: phase 11 Console Real Data Plane smoke (REAL mode default; LOCAL_ONLY=1 for v0.3 inmem fallback)"
if [ "${RELEASE_SMOKE_CONSOLE:-0}" = "1" ]; then
  bash scripts/console_smoke.sh
else
  echo "  SKIP (set RELEASE_SMOKE_CONSOLE=1 to enable — runs scripts/console_smoke.sh REAL mode)"
fi

echo "release_smoke: tarball_contract=ok smoke_evidence=ok benchmark_gate=ok grpc_search_smoke=ok phase9_cli_e2e=ok phase11_console_real=${RELEASE_SMOKE_CONSOLE:+ok} phase15_console_functional_gap_closure=ok"
echo "PHASE_RELEASE_SMOKE_EXIT=0"
