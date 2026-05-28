#!/usr/bin/env bash
# scripts/release_smoke.sh — v0.1 (task-8.3) + v0.2 (task-9.5) + v0.3 (task-10.6) + v0.4 (task-11.4) + v0.9 (task-16.4) + v0.10 (task-17.1) release smoke gate.
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
#      env LOCAL_ONLY=1 fallback. v7 (Phase 16) extends to 27 steps —
#      step 25 long-poll wait timing (task-16.2), step 26 TraceStore SQLite
#      restart roundtrip (task-16.1), step 27 compose-prod (task-16.4 gated
#      via COMPOSE_PROD_SMOKE=1).
#      Gated on env RELEASE_SMOKE_CONSOLE=1 (default SKIP to avoid hard-
#      requiring full cargo build inside every CI matrix; CI fast path can
#      run just sections 1-4).
#   6. Phase 16 backlog completion ghcr image verify (task-16.3) — `docker pull`
#      the published ghcr.io/${OWNER}/contextforge-daemon:${VERSION} image
#      and verify a healthy `docker run`. Gated on RELEASE_SMOKE_GHCR=1
#      (default SKIP because CI runners may lack a docker daemon, and the
#      v0.9.0 tag must be pushed before this verify can succeed).
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

echo "release_smoke[5/6]: phase 11 Console Real Data Plane smoke (REAL mode default; LOCAL_ONLY=1 for v0.3 inmem fallback; v8 includes Phase 16 steps 25-27 + Phase 17 step 28 task-17.1 is_pinned roundtrip)"
if [ "${RELEASE_SMOKE_CONSOLE:-0}" = "1" ]; then
  bash scripts/console_smoke.sh
else
  echo "  SKIP (set RELEASE_SMOKE_CONSOLE=1 to enable — runs scripts/console_smoke.sh REAL mode v7)"
fi

echo "release_smoke[6/6]: phase 16 ghcr image verify (task-16.3)"
if [ "${RELEASE_SMOKE_GHCR:-0}" = "1" ]; then
  if ! command -v docker >/dev/null 2>&1; then
    echo "FAIL: RELEASE_SMOKE_GHCR=1 but docker not on PATH" >&2
    exit 1
  fi
  GHCR_OWNER="${GHCR_OWNER:-tajiaoyezi}"
  GHCR_VERSION="${GHCR_VERSION:-v0.9.0}"
  GHCR_IMAGE="ghcr.io/${GHCR_OWNER}/contextforge-daemon:${GHCR_VERSION}"
  echo "  docker pull ${GHCR_IMAGE}"
  docker pull "${GHCR_IMAGE}"
  echo "  docker run + curl /v1/health (CONSOLE_API_FALLBACK_INMEM=1 opt-in for single-container probe)"
  CID=$(docker run -d --rm -p 48181:48181 -e CONSOLE_API_FALLBACK_INMEM=1 "${GHCR_IMAGE}")
  sleep 5
  if ! curl -fsS http://localhost:48181/v1/health >/dev/null 2>&1; then
    echo "FAIL: /v1/health not 200 against ${GHCR_IMAGE}" >&2
    docker logs "${CID}" 2>&1 | tail -50 >&2 || true
    docker stop "${CID}" 2>/dev/null || true
    exit 1
  fi
  docker stop "${CID}" 2>/dev/null || true
  echo "  ✅ ghcr image healthy"
else
  echo "  SKIP (set RELEASE_SMOKE_GHCR=1 + ensure ghcr.io/.../contextforge-daemon:${GHCR_VERSION:-v0.9.0} published to enable)"
fi

echo "release_smoke: tarball_contract=ok smoke_evidence=ok benchmark_gate=ok grpc_search_smoke=ok phase9_cli_e2e=ok phase11_console_real=${RELEASE_SMOKE_CONSOLE:+ok} phase15_console_functional_gap_closure=ok phase16_backlog_completion=${RELEASE_SMOKE_CONSOLE:+ok} phase16_ghcr_verify=${RELEASE_SMOKE_GHCR:+ok} phase17_is_pinned_amendment=${RELEASE_SMOKE_CONSOLE:+ok}"
echo "PHASE_RELEASE_SMOKE_EXIT=0"
