#!/usr/bin/env bash
# scripts/spike_vector_backends.sh — task-18.2: run the spike harness per wired backend and
# write 5-dimension evidence to docs/spikes/phase-18-<backend>.md.
#
# task-18.3 (sqlite-vec) + task-18.6 (hnsw) are wired; task-18.4-18.5 extend BACKENDS further.
# Real backends are feature-gated, so FEATURES selects which cargo features to enable.
#
# usage: scripts/spike_vector_backends.sh [N] [DIM] [SEED] [FEATURES]
#   e.g. scripts/spike_vector_backends.sh 5000 64 1 "vector-hnsw,vector-sqlite"

set -euo pipefail

N="${1:-2000}"
DIM="${2:-64}"
SEED="${3:-1}"
FEATURES="${4:-vector-hnsw,vector-sqlite}"

mkdir -p docs/spikes

# "qdrant" (task-18.4) + "lancedb" (task-18.5) are intentionally omitted from the default list:
#   - qdrant needs a running Qdrant server (gRPC 6334) + the vector-qdrant feature
#   - lancedb needs protoc at build time + the vector-lancedb feature
# run them on demand, e.g.:
#   cargo run --release -p contextforge-bench --features vector-qdrant  -- --backend qdrant  ...
#   PROTOC=/path/to/protoc cargo run --release -p contextforge-bench --features vector-lancedb -- --backend lancedb ...
BACKENDS=("noop" "sqlite-vec" "hnsw")

for b in "${BACKENDS[@]}"; do
  out="docs/spikes/phase-18-${b}.md"
  echo "→ spike backend=${b} n=${N} dim=${DIM} seed=${SEED} → ${out}"
  cargo run -q -p contextforge-bench --features "${FEATURES}" -- --backend "${b}" --n "${N}" --dim "${DIM}" --seed "${SEED}" --out "${out}"
done

echo "✅ spike evidence written under docs/spikes/"
