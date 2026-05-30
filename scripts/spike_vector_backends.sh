#!/usr/bin/env bash
# scripts/spike_vector_backends.sh — task-18.2: run the spike harness per wired backend and
# write 5-dimension evidence to docs/spikes/phase-18-<backend>.md.
#
# At task-18.2 ship only "noop" is wired; task-18.3-18.6 extend BACKENDS with their real backend.
#
# usage: scripts/spike_vector_backends.sh [N] [DIM] [SEED]

set -euo pipefail

N="${1:-2000}"
DIM="${2:-64}"
SEED="${3:-1}"

mkdir -p docs/spikes

# task-18.3-18.6 extend this list: ("noop" "sqlite-vec" "qdrant" "lancedb" "hnsw")
BACKENDS=("noop")

for b in "${BACKENDS[@]}"; do
  out="docs/spikes/phase-18-${b}.md"
  echo "→ spike backend=${b} n=${N} dim=${DIM} seed=${SEED} → ${out}"
  cargo run -q -p contextforge-bench -- --backend "${b}" --n "${N}" --dim "${DIM}" --seed "${SEED}" --out "${out}"
done

echo "✅ spike evidence written under docs/spikes/"
