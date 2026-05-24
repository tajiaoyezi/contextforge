#!/usr/bin/env bash
# scripts/console_smoke.sh — task-10.6 / Phase 10 console-contract-v1 smoke.
#
# Two modes (selected by env DOCKER_SMOKE):
#
#   Default (DOCKER_SMOKE != "1"): build & run `contextforge console-api-serve`
#   as a local background process; curl the 9 Console Contract v1 endpoints;
#   verify the workspace POST + GET cycle returns real data (not Mock). This
#   is what CI uses — no docker dependency.
#
#   DOCKER_SMOKE=1: docker compose up -d the `contextforge` service (plus
#   `--profile console` extras if CONSOLE_API_IMAGE / CONSOLE_WEB_IMAGE
#   are set); same 9 endpoint curl flow against the docker-published port.
#
# Final stdout marker: CONSOLE_SMOKE_EXIT=0 on success.
#
# Designed for Linux / WSL2. macOS likely works (curl + go + bash all
# available); Windows users should run from Git Bash or WSL.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

STAGING="$(mktemp -d -t cfg-console-smoke-XXXXXX)"
cleanup_local=""
trap 'cleanup' EXIT
cleanup() {
  if [ -n "$cleanup_local" ]; then
    eval "$cleanup_local"
  fi
  rm -rf "$STAGING"
}

EXE_SUFFIX=""
if [ "${OS:-}" = "Windows_NT" ]; then
  EXE_SUFFIX=".exe"
fi

PORT="48181"
BASE="http://localhost:${PORT}"

# ----------- Mode selection -----------
if [ "${DOCKER_SMOKE:-0}" = "1" ]; then
  echo "[mode] docker compose (DOCKER_SMOKE=1)"
  if ! command -v docker >/dev/null 2>&1; then
    echo "ERROR: docker not on PATH; either install docker or unset DOCKER_SMOKE" >&2
    exit 1
  fi
  echo "[docker] building + starting contextforge service..."
  docker compose -f deploy/console-stack.yml up -d --build contextforge
  cleanup_local='docker compose -f deploy/console-stack.yml down -v 2>&1 | tail -3 || true'
else
  echo "[mode] local (DOCKER_SMOKE unset; build go binary + spawn console-api-serve)"
  BIN="$STAGING/contextforge${EXE_SUFFIX}"
  echo "[1/3] go build ./cmd/contextforge"
  go build -o "$BIN" ./cmd/contextforge
  echo "[2/3] spawn console-api-serve on $BASE"
  "$BIN" console-api-serve --addr "127.0.0.1:${PORT}" >"$STAGING/server.log" 2>&1 &
  SERVER_PID=$!
  cleanup_local="kill -TERM $SERVER_PID 2>/dev/null || true; wait $SERVER_PID 2>/dev/null || true"
fi

# ----------- Wait for health -----------
echo "[wait] /v1/health up to 30s"
for i in $(seq 1 30); do
  if curl -sf "$BASE/v1/health" >/dev/null 2>&1; then
    echo "  ✅ health responsive at attempt $i"
    break
  fi
  sleep 1
  if [ "$i" = "30" ]; then
    echo "FAIL: /v1/health did not respond within 30s" >&2
    if [ "${DOCKER_SMOKE:-0}" != "1" ]; then
      echo "---- server.log ----" >&2
      tail -50 "$STAGING/server.log" >&2 || true
    fi
    exit 1
  fi
done

# ----------- 9 endpoint flow -----------
echo "[3/3] 9 endpoint flow"

echo "  [1/9] GET /v1/health (must contain contract_version=v1)"
health_body=$(curl -sf "$BASE/v1/health")
echo "$health_body" | grep -q '"contract_version":"v1"' \
  || { echo "FAIL: /v1/health body missing contract_version=v1: $health_body" >&2; exit 1; }

echo "  [2/9] POST /v1/workspaces (create 'console-smoke')"
ws_body=$(curl -sf -X POST "$BASE/v1/workspaces" \
  -H 'Content-Type: application/json' \
  -d '{"name":"console-smoke","root_path":"/tmp/console-smoke","allowlist":["*.md"],"denylist":[".env"]}')
WS_ID=$(echo "$ws_body" | sed -n 's/.*"workspace_id":"\([^"]*\)".*/\1/p')
[ -z "$WS_ID" ] && { echo "FAIL: workspace_id not parsed from $ws_body" >&2; exit 1; }
echo "    → workspace_id=$WS_ID"

echo "  [3/9] GET /v1/workspaces (list)"
list_body=$(curl -sf "$BASE/v1/workspaces")
echo "$list_body" | grep -q "\"workspace_id\":\"${WS_ID}\"" \
  || { echo "FAIL: list does not contain $WS_ID: $list_body" >&2; exit 1; }

echo "  [4/9] GET /v1/workspaces/$WS_ID (single)"
single_body=$(curl -sf "$BASE/v1/workspaces/${WS_ID}")
echo "$single_body" | grep -q "\"name\":\"console-smoke\"" \
  || { echo "FAIL: single get missing name: $single_body" >&2; exit 1; }

echo "  [5/9] GET /v1/workspaces/non-existent-id (must return 404)"
code=$(curl -s -o /dev/null -w '%{http_code}' "$BASE/v1/workspaces/non-existent-id")
[ "$code" = "404" ] || { echo "FAIL: expected 404; got $code" >&2; exit 1; }

echo "  [6/9] POST /v1/index-jobs (enqueue)"
job_body=$(curl -sf -X POST "$BASE/v1/index-jobs" \
  -H 'Content-Type: application/json' \
  -d "{\"workspace_id\":\"${WS_ID}\",\"trigger_source\":\"smoke\"}")
JOB_ID=$(echo "$job_body" | sed -n 's/.*"job_id":"\([^"]*\)".*/\1/p')
[ -z "$JOB_ID" ] && { echo "FAIL: job_id not parsed from $job_body" >&2; exit 1; }
echo "    → job_id=$JOB_ID"

echo "  [7/9] GET /v1/index-jobs/$JOB_ID + POST /cancel (200 / 200 / 409)"
curl -sf "$BASE/v1/index-jobs/${JOB_ID}" >/dev/null
code=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$BASE/v1/index-jobs/${JOB_ID}/cancel")
[ "$code" = "200" ] || { echo "FAIL: first cancel expected 200; got $code" >&2; exit 1; }
code=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$BASE/v1/index-jobs/${JOB_ID}/cancel")
[ "$code" = "409" ] || { echo "FAIL: re-cancel expected 409; got $code" >&2; exit 1; }

echo "  [8/9] POST /v1/search (nested {result, trace})"
search_body=$(curl -sf -X POST "$BASE/v1/search" \
  -H 'Content-Type: application/json' \
  -d "{\"query\":\"configuration\",\"workspace_id\":\"${WS_ID}\",\"top_k\":5,\"retrieval_method\":\"bm25\",\"agent_scope\":\"session\"}")
echo "$search_body" | grep -q '"result"' \
  && echo "$search_body" | grep -q '"trace"' \
  || { echo "FAIL: search not nested {result, trace}: $search_body" >&2; exit 1; }

echo "  [9/9] GET /v1/observability/events (≥1 event from prior ops)"
events_body=$(curl -sf "$BASE/v1/observability/events")
echo "$events_body" | grep -q '"event_id"' \
  || { echo "FAIL: events empty: $events_body" >&2; exit 1; }

echo
echo "CONSOLE_SMOKE_EXIT=0"
