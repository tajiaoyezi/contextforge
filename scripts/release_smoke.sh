#!/usr/bin/env bash
# scripts/release_smoke.sh — v0.1 (task-8.3) + v0.2 (task-9.5) release smoke gate.
#
# Sections:
#   1. Go release harness (BuildTarball / ValidateTarball / CheckBenchmark unit tests)
#   2. Task 8 reliability + eval harness unit tests
#   3. Rust gRPC search smoke (phase6_search_grpc_end_to_end_smoke)
#   4. Phase 9 CLI end-to-end smoke (TestPhase9ReleaseSmoke_EndToEnd) — REAL
#      go build + cargo build + 7-step CLI binary exercise. Renamed exit
#      marker to PHASE_RELEASE_SMOKE_EXIT (drops v0.1-only PHASE8 prefix per
#      task-9.5 §3).
#
# Each section's non-zero exit propagates (set -e). Final marker line is the
# release tag gate.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "PHASE_RELEASE_SMOKE_BEGIN"

echo "release_smoke[1/4]: go release harness (TestTask83 AC1 real-binary + AC3 benchmark)"
go test ./internal/release -run 'TestTask83'

echo "release_smoke[2/4]: task 8 reliability/eval harness"
go test ./internal/eval ./internal/reliability -run 'TestTask8(1|2)'

echo "release_smoke[3/4]: Rust gRPC search smoke"
cargo test --workspace phase_6_search_grpc_end_to_end_smoke

echo "release_smoke[4/4]: phase 9 CLI end-to-end smoke (real binaries + 7-step CLI)"
go test ./internal/release -run 'TestPhase9ReleaseSmoke_EndToEnd' -timeout 180s

echo "release_smoke: tarball_contract=ok smoke_evidence=ok benchmark_gate=ok grpc_search_smoke=ok phase9_cli_e2e=ok"
echo "PHASE_RELEASE_SMOKE_EXIT=0"
