#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "PHASE8_RELEASE_SMOKE_BEGIN"
echo "release_smoke: go release harness"
go test ./internal/release -run 'TestTask83'

echo "release_smoke: task 8 reliability/eval harness"
go test ./internal/eval ./internal/reliability -run 'TestTask8(1|2)'

echo "release_smoke: Rust gRPC search smoke"
cargo test --workspace phase_6_search_grpc_end_to_end_smoke

echo "release_smoke: tarball_contract=ok smoke_evidence=ok benchmark_gate=ok grpc_search_smoke=ok"
echo "PHASE8_RELEASE_SMOKE_EXIT=0"
