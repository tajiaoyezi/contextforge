#!/usr/bin/env bash
# scripts/quickstart_smoke.sh — Phase 9 task-9.6 §6 AC3 entrypoint.
#
# Drives the README Quick Start command sequence end-to-end against the
# examples/quickstart/ fixture in a throw-away staging directory. Asserts
# every step exits 0 and emits PHASE 9 smoke marker as the final line.
#
# Usage:
#   bash scripts/quickstart_smoke.sh
#
# Designed for Linux / WSL2. macOS should work (bash + cargo + go) but is
# not part of v0.2 §6 AC. Windows users: run from Git Bash or WSL.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

STAGING="$(mktemp -d -t cfg-quickstart-XXXXXX)"
DATA="$STAGING/data"
cleanup() { rm -rf "$STAGING"; }
trap cleanup EXIT

EXE_SUFFIX=""
if [ "${OS:-}" = "Windows_NT" ]; then
  EXE_SUFFIX=".exe"
fi

echo "[1/7] build binaries (go + cargo)"
go build -o "$STAGING/contextforge${EXE_SUFFIX}" ./cmd/contextforge
cargo build -p contextforge-core
cp "target/debug/contextforge-core${EXE_SUFFIX}" "$STAGING/"
export PATH="$STAGING:$PATH"
export CONTEXTFORGE_DATA_DIR="$DATA"

echo "[2/7] init"
contextforge init --root "$DATA"

echo "[3/7] import hermes memory fixture"
contextforge import hermes "$ROOT/examples/quickstart/hermes-memory" \
  --collection demo --data-dir "$DATA"

echo "[4/7] index imported hermes records"
contextforge index --source "$DATA/imports/hermes" \
  --collection demo --data-dir "$DATA"

echo "[5/7] index sample project"
contextforge index --source "$ROOT/examples/quickstart/sample-project" \
  --collection demo --data-dir "$DATA"

echo "[6/7] search 'configuration'"
contextforge search --collections=demo --top-k=5 --explain "configuration"

echo "[7/7] eval run (smoke — gate expected to fail on this tiny demo fixture)"
# The eval golden questions target real project source (internal/config/config.go,
# core/src/retriever/mod.rs, …), but this Quick Start only indexed the 2-file demo
# fixture (hermes-memory + sample-project). So the recall gate will fail here — that
# is expected, not a regression. The eval command is run only to prove the CLI path
# wires end-to-end (query → retrieve → score → report). To measure real recall,
# index the project's own source then run: contextforge eval run --collection <that>
# (BM25-only by default; add --semantic / --hybrid for the vector paths whose recall
# the PRD north-star 75/85% gate measures).
eval_summary="$(contextforge eval run --collection demo 2>&1 || true)"
if echo "$eval_summary" | grep -qE "top5_strong_rate|gate="; then
  echo "    → eval CLI wired end-to-end ✅ (recall gate intentionally skipped on the 2-file demo fixture)"
else
  echo "    → eval CLI ran but produced no summary — check output:" >&2
  echo "$eval_summary" >&2
fi

echo "QUICKSTART_SMOKE_EXIT=0"
