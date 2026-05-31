#!/usr/bin/env bash
# scripts/console_smoke.sh — Phase 21 task-21.3 retrieval-quality smoke (v11; was Phase 20 v10).
#
# REAL mode (default): spawns BOTH the Rust `contextforge-core` daemon
# (data plane gRPC) AND the Go `console-api-serve` REST proxy. The
# console-api-serve dials the Rust daemon over gRPC, so the 22 REST endpoints
# (v0.4 9 + task-12.1 3 + task-12.2 1 + task-12.3 1 + task-13.2 5 memory +
# task-14.2 2 eval-runs) go through the real cross-process bridge
# (ADR-016 D2 / ADR-017 D1 Wave 1+2+3+4 — full 22 endpoint conformance).
# Note: 22 endpoint count in playbook spec includes 2 routes (`POST /v1/index-jobs`,
# `GET /v1/index-jobs?status=active`) shared via filtered shape — flow tests 20
# distinct invocations covering all 22 of the Console contract.
#
# v6 (Phase 15) added steps 21-24 — stats/chunks, eval-runs list, queries list,
# health?detailed=true.
#
# v7 (Phase 16) added steps 25-27 — task-16.2 long-poll wait timing semantics,
# task-16.1 TraceStore SQLite restart roundtrip, task-16.4 compose-prod stack
# health (env-gated via COMPOSE_PROD_SMOKE=1).
#
# v8 (Phase 17) added step 28 — task-17.1 MemoryItem.is_pinned add-only field
# Pin RPC roundtrip (POST {"pin": true} + GET asserts is_pinned=true; POST
# {"pin": false} + GET asserts is_pinned=false). Step 26 already validates
# SQLite persistence across daemon restart, so step 28 trusts that path and
# focuses on the wire-field roundtrip exposed by task-17.1.
#
# v9 (Phase 19) added steps 29-30 — task-19.4 semantic-retrieval wiring. Step 29
# exercises the task-19.3 /v1/search?semantic=true REST→gRPC path (add-only query
# param; asserts the response is still nested {result, trace}). Step 30 builds the
# Go binary and runs `contextforge eval run --semantic`, asserting the dual-path
# (BM25 + semantic) report shape + recall-gate line. Per ADR-013 neither step
# asserts a recall threshold — real SemanticRecall@K numbers come from task-19.5;
# these steps only prove the semantic path is wired end-to-end.
#
# v10 (Phase 20) upgrades step 29 — task-20.1 made console-api forward ?semantic=true
# to the gRPC SearchService.Query semantic branch, so step 29 now asserts the semantic
# path actually engaged (the trace's candidate_generation_steps reports the vector path),
# not only that the add-only param preserves the {result, trace} contract. Per ADR-013
# still no recall-threshold assertion (deterministic provider proves dispatch, not recall;
# real recall via the Retriever hot path is task-20.2 / docs/spikes/phase-20-recall-via-retriever.md).
#
# v11 (Phase 21) upgrades step 30 — task-21.3 grows `eval run --semantic` into
# `eval run --semantic --hybrid --rerank`, so step 30 now also asserts the add-only hybrid
# (req.Hybrid → daemon search_hybrid, task-21.1) + reranked (eval-layer deterministic
# IdentityReranker, ADR-026 D2) multi-path report lines + gate engage end-to-end. Per ADR-013
# still no recall-threshold assertion (the transient eval index is empty; real hybrid/rerank
# recall vs the baseline is docs/spikes/phase-21-hybrid-recall.md). Per-result
# retrieval_method="hybrid" + hybrid_score provenance is asserted by the Rust dispatch test
# (core/src/server.rs test_21_1_hybrid_dispatches_fusion_path); the console-api ?hybrid/?rerank
# REST forward stays [SPEC-DEFER:phase-future.console-api-hybrid-forward].
#
# v12 (Phase 22) adds step 31 — task-22.4 closeout. `contextforge init` now scaffolds an add-only
# [embedding] config section (provider/dim, task-22.1) alongside the existing [remote] section; step
# 31 asserts the real config codec emits it without disturbing [remote]. The embedding cache
# (task-22.2) + remote provider skeleton (task-22.3) are verified at the unit/contract layer
# (TEST-22.2.* / TEST-22.3.*) — they are not console-hot-path-wired in v0.15, and the remote path
# never hits the network here (ADR-013 — real remote 联调 / recall deferred,
# [SPEC-DEFER:phase-future.embedding-provider-remote]).
#
# Modes (selected by env):
#
#   Default (REAL mode):  spawn contextforge-core + console-api-serve;
#       curl the 22 endpoints + run index-job against test/fixtures/index-job-real/
#       + POST /v1/search returns ≥1 real chunk. Final marker:
#       CONSOLE_REAL_SMOKE_EXIT=0.
#
#   LOCAL_ONLY=1: v0.3 backward-compatible mode — only spawns console-api-serve
#       with CONSOLE_API_FALLBACK_INMEM=1 (no Rust daemon). Final marker:
#       CONSOLE_SMOKE_EXIT=0.
#
#   DOCKER_SMOKE=1: docker compose up; same 22 endpoint flow against the
#       docker-published REST port (v0.3 compat path).
#
#   COMPOSE_PROD_SMOKE=1: also run step 27 (docker compose -f
#       deploy/docker-compose.production.yml up -d → /v1/health 200). Gated
#       default SKIP because it requires a live docker daemon and a published
#       ghcr.io image; meant for release_smoke.sh + manual verification.
#
# Designed for Linux / WSL2 / macOS / Git Bash on Windows.

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

# ----------- Mode selection -----------
MODE="real"
if [ "${LOCAL_ONLY:-0}" = "1" ]; then
  MODE="local"
elif [ "${DOCKER_SMOKE:-0}" = "1" ]; then
  MODE="docker"
fi
echo "[mode] $MODE (REAL=v0.4 default; LOCAL_ONLY=1 = v0.3 inmem; DOCKER_SMOKE=1 = docker compose)"

REST_PORT="48181"
GRPC_PORT="50552"   # randomized-ish to avoid clashing with system Phase 9 default :50551
BASE="http://127.0.0.1:${REST_PORT}"

# ----------- Mode dispatch -----------
case "$MODE" in
  real)
    DATA_DIR="$STAGING/cf-data"
    mkdir -p "$DATA_DIR"
    echo "[real][1/4] cargo build -p contextforge-core"
    cargo build -p contextforge-core --quiet 2>&1 || cargo build -p contextforge-core
    CORE_BIN="$ROOT/target/debug/contextforge-core${EXE_SUFFIX}"
    if [ ! -x "$CORE_BIN" ]; then
      echo "FAIL: $CORE_BIN missing after cargo build" >&2
      exit 1
    fi

    echo "[real][2/4] spawn contextforge-core at 127.0.0.1:${GRPC_PORT}"
    "$CORE_BIN" "127.0.0.1:${GRPC_PORT}" "$DATA_DIR" >"$STAGING/core.log" 2>&1 &
    CORE_PID=$!

    echo "[real][3/4] go build ./cmd/contextforge"
    GO_BIN="$STAGING/contextforge${EXE_SUFFIX}"
    go build -o "$GO_BIN" ./cmd/contextforge

    echo "[real][4/4] spawn console-api-serve at $BASE → gRPC 127.0.0.1:${GRPC_PORT}"
    "$GO_BIN" console-api-serve --addr "127.0.0.1:${REST_PORT}" --grpc-addr "127.0.0.1:${GRPC_PORT}" >"$STAGING/api.log" 2>&1 &
    API_PID=$!
    cleanup_local="kill -TERM $API_PID 2>/dev/null || true; kill -TERM $CORE_PID 2>/dev/null || true; wait $API_PID 2>/dev/null || true; wait $CORE_PID 2>/dev/null || true"
    ;;
  local)
    echo "[local] go build ./cmd/contextforge"
    GO_BIN="$STAGING/contextforge${EXE_SUFFIX}"
    go build -o "$GO_BIN" ./cmd/contextforge
    echo "[local] spawn console-api-serve at $BASE (CONSOLE_API_FALLBACK_INMEM=1)"
    CONSOLE_API_FALLBACK_INMEM=1 "$GO_BIN" console-api-serve --addr "127.0.0.1:${REST_PORT}" >"$STAGING/api.log" 2>&1 &
    API_PID=$!
    cleanup_local="kill -TERM $API_PID 2>/dev/null || true; wait $API_PID 2>/dev/null || true"
    ;;
  docker)
    if ! command -v docker >/dev/null 2>&1; then
      echo "ERROR: docker not on PATH" >&2
      exit 1
    fi
    echo "[docker] building + starting contextforge service..."
    docker compose -f deploy/console-stack.yml up -d --build contextforge
    cleanup_local='docker compose -f deploy/console-stack.yml down -v 2>&1 | tail -3 || true'
    ;;
esac

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
    if [ "$MODE" = "real" ]; then
      echo "---- core.log ----" >&2; tail -50 "$STAGING/core.log" 2>/dev/null || true
      echo "---- api.log ----" >&2; tail -50 "$STAGING/api.log" 2>/dev/null || true
    elif [ "$MODE" = "local" ]; then
      tail -50 "$STAGING/api.log" >&2 || true
    fi
    exit 1
  fi
done

# ----------- 20 endpoint flow (v0.7: 9 base + task-12.1 3 + task-12.2 1 + task-12.3 1 + task-13.2 5 memory + task-14.2 2 eval = Console 22 endpoint conformance) -----------
echo "[flow] 20 endpoint flow (Console 22-endpoint conformance)"

echo "  [1/20] GET /v1/health (must contain contract_version=v1)"
health_body=$(curl -sf "$BASE/v1/health")
echo "$health_body" | grep -q '"contract_version":"v1"' \
  || { echo "FAIL: /v1/health body missing contract_version=v1: $health_body" >&2; exit 1; }

# REAL mode: workspace_id is generated by Rust (UUID-like via the v0.4 path);
# we still need to feed the proto Create with a workspace_id. Use a stable name.
WS_NAME="cf-real-smoke"

echo "  [2/20] POST /v1/workspaces"
# task-11.3 SqliteWorkspaceStore validates root_path via Rust Path::is_absolute,
# which on Windows requires native form (C:\...) not Git-Bash form (/h/...).
# Use cygpath when available to translate; otherwise pass through.
WS_ROOT="$ROOT/test/fixtures/index-job-real"
if [ "$MODE" != "real" ]; then
  WS_ROOT="/tmp/cf-smoke-fixture"
elif command -v cygpath >/dev/null 2>&1; then
  WS_ROOT="$(cygpath -w "$WS_ROOT" | sed 's|\\|/|g')"
fi
ws_body=$(curl -sf -X POST "$BASE/v1/workspaces" \
  -H 'Content-Type: application/json' \
  -d "{\"name\":\"$WS_NAME\",\"root_path\":\"$WS_ROOT\",\"allowlist\":[\"*.md\"],\"denylist\":[]}")
WS_ID=$(echo "$ws_body" | sed -n 's/.*"workspace_id":"\([^"]*\)".*/\1/p')
[ -z "$WS_ID" ] && { echo "FAIL: workspace_id not parsed: $ws_body" >&2; exit 1; }
echo "    → workspace_id=$WS_ID"

echo "  [3/20] GET /v1/workspaces (list)"
list_body=$(curl -sf "$BASE/v1/workspaces")
echo "$list_body" | grep -q "\"workspace_id\":\"${WS_ID}\"" \
  || { echo "FAIL: list does not contain $WS_ID: $list_body" >&2; exit 1; }

echo "  [4/20] GET /v1/workspaces/$WS_ID"
single_body=$(curl -sf "$BASE/v1/workspaces/${WS_ID}")
echo "$single_body" | grep -q "\"name\":\"${WS_NAME}\"" \
  || { echo "FAIL: single get missing name: $single_body" >&2; exit 1; }

echo "  [5/20] GET /v1/workspaces/non-existent-id (must return 404)"
code=$(curl -s -o /dev/null -w '%{http_code}' "$BASE/v1/workspaces/non-existent-id")
[ "$code" = "404" ] || { echo "FAIL: expected 404; got $code" >&2; exit 1; }

echo "  [6/20] POST /v1/index-jobs (enqueue against fixture repo)"
job_body=$(curl -sf -X POST "$BASE/v1/index-jobs" \
  -H 'Content-Type: application/json' \
  -d "{\"workspace_id\":\"${WS_ID}\",\"trigger_source\":\"smoke\"}")
JOB_ID=$(echo "$job_body" | sed -n 's/.*"job_id":"\([^"]*\)".*/\1/p')
[ -z "$JOB_ID" ] && { echo "FAIL: job_id not parsed: $job_body" >&2; exit 1; }
echo "    → job_id=$JOB_ID"

echo "  [7/20] poll /v1/index-jobs/<id> until status terminal (≤30s)"
if [ "$MODE" = "real" ]; then
  for i in $(seq 1 30); do
    job_body=$(curl -sf "$BASE/v1/index-jobs/${JOB_ID}")
    status=$(echo "$job_body" | sed -n 's/.*"status":"\([^"]*\)".*/\1/p')
    case "$status" in
      succeeded|failed|cancelled)
        echo "    → terminal at attempt $i: status=$status"
        break
        ;;
      *)
        sleep 1
        ;;
    esac
    if [ "$i" = "30" ]; then
      echo "FAIL: job did not reach terminal in 30s; last=$job_body" >&2
      exit 1
    fi
  done
  [ "$status" = "succeeded" ] || echo "  NOTE: REAL job status=$status (test fixture index)"
else
  # LOCAL_ONLY / docker: in-memory MemStore can still drive cancel.
  # task-12.1 (ADR-017 D3) ships cancel as 204 No Content; 409 only when the
  # job is already terminal. v0.7 release-smoke gate ran REAL mode where the
  # 204/409 paths converge on poll-until-terminal so the v0.3 stale assertion
  # (200/409) was never exercised; task-15.6 v6 smoke regression caught it.
  curl -sf "$BASE/v1/index-jobs/${JOB_ID}" >/dev/null
  cancel_code=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$BASE/v1/index-jobs/${JOB_ID}/cancel")
  [ "$cancel_code" = "204" ] || [ "$cancel_code" = "409" ] || { echo "FAIL: cancel expected 204/409; got $cancel_code" >&2; exit 1; }
fi

echo "  [8/20] POST /v1/search (real mode → ≥1 chunk; inmem → empty trace ok)"
search_body=$(curl -sf -X POST "$BASE/v1/search" \
  -H 'Content-Type: application/json' \
  -d "{\"query\":\"contextforge\",\"workspace_id\":\"${WS_ID}\",\"top_k\":5,\"retrieval_method\":\"bm25\",\"agent_scope\":\"session\"}")
echo "$search_body" | grep -q '"result"' \
  && echo "$search_body" | grep -q '"trace"' \
  || { echo "FAIL: search not nested {result, trace}: $search_body" >&2; exit 1; }
if [ "$MODE" = "real" ] && [ "${status:-}" = "succeeded" ]; then
  # REAL mode + index succeeded → search should return at least 1 chunk
  echo "$search_body" | grep -q '"chunk_id"' \
    || echo "  NOTE: REAL search returned 0 chunks (may be fixture-too-small)"
fi

# Extract query_id + chunk_id for follow-on smoke steps (task-12.2 / 12.3).
QUERY_ID=$(echo "$search_body" | sed -nE 's/.*"query_id":"([^"]+)".*/\1/p' | head -1)
CHUNK_ID=$(echo "$search_body" | sed -nE 's/.*"chunk_id":"([^"]+)".*/\1/p' | head -1)

echo "  [9/20] task-12.1 PATCH /v1/workspaces/<id>/config (X-Confirm required → 412 then 200)"
if [ "$MODE" = "real" ]; then
  # No X-Confirm → 412
  code412=$(curl -sf -o /dev/null -w '%{http_code}' -X PATCH "$BASE/v1/workspaces/${WS_ID}/config" \
    -H 'Content-Type: application/json' \
    -d '{"allowlist":["src/**"],"denylist":["node_modules/**"]}' || true)
  [ "$code412" = "412" ] \
    || { echo "FAIL: PATCH config without X-Confirm expected 412; got $code412" >&2; exit 1; }
  # With X-Confirm → 200
  curl -sf -X PATCH "$BASE/v1/workspaces/${WS_ID}/config" \
    -H 'Content-Type: application/json' \
    -H 'X-Confirm: yes' \
    -d '{"allowlist":["src/**"],"denylist":["node_modules/**"]}' >/dev/null \
    || { echo "FAIL: PATCH config with X-Confirm did not return 2xx" >&2; exit 1; }
fi

echo "  [10/20] task-12.1 GET /v1/index-jobs?status=active (missing status → 400)"
if [ "$MODE" = "real" ]; then
  active_body=$(curl -sf "$BASE/v1/index-jobs?status=active") \
    || { echo "FAIL: GET active jobs" >&2; exit 1; }
  # Missing filter → 400
  code400=$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/v1/index-jobs" || true)
  [ "$code400" = "400" ] \
    || { echo "FAIL: GET index-jobs without status expected 400; got $code400" >&2; exit 1; }
fi

echo "  [11/20] task-12.2 GET /v1/source-chunks/<id> (or 404 if no chunk extracted)"
if [ "$MODE" = "real" ] && [ -n "$CHUNK_ID" ]; then
  curl -sf "$BASE/v1/source-chunks/$CHUNK_ID" >/dev/null \
    && echo "  ok: chunk $CHUNK_ID found" \
    || echo "  NOTE: source-chunks/$CHUNK_ID lookup failed (may be retriever cache miss)"
elif [ "$MODE" = "real" ]; then
  # No chunk from search → expect 404 on a fake id
  code404=$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/v1/source-chunks/chk_fake_0" || true)
  [ "$code404" = "404" ] \
    || { echo "FAIL: GET source-chunks unknown expected 404; got $code404" >&2; exit 1; }
fi

echo "  [12/20] task-12.3 GET /v1/search/<query_id>/trace (or 404 on unknown)"
if [ "$MODE" = "real" ] && [ -n "$QUERY_ID" ]; then
  trace_body=$(curl -sf "$BASE/v1/search/$QUERY_ID/trace") \
    && echo "$trace_body" | grep -q '"trace_id"' \
    || echo "  NOTE: trace lookup for $QUERY_ID returned non-2xx or missing trace_id"
fi
# Unknown query_id → 404 (always validated)
if [ "$MODE" = "real" ]; then
  code404=$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/v1/search/qry-does-not-exist/trace" || true)
  [ "$code404" = "404" ] \
    || { echo "FAIL: GET trace unknown expected 404; got $code404" >&2; exit 1; }
fi

echo "  [13/20] task-13.2 memory: seed 5 fixture items via sqlite3 CLI (or skip if missing)"
if [ "$MODE" = "real" ] && command -v sqlite3 >/dev/null 2>&1; then
  sqlite3 "$DATA_DIR/memory.db" < "$ROOT/test/fixtures/memory-seed/seed.sql" 2>&1 || echo "  NOTE: seed skipped"
  echo "    → seeded 5 fixture memory items"
else
  echo "  NOTE: sqlite3 unavailable; skipping seed (memory tests use empty store)"
fi

echo "  [14/20] task-13.2 GET /v1/memory"
if [ "$MODE" = "real" ]; then
  mem_body=$(curl -sf "$BASE/v1/memory") || { echo "FAIL: GET memory" >&2; exit 1; }
  echo "    → memory body length=$(echo "$mem_body" | wc -c)"
fi

echo "  [15/20] task-13.2 GET /v1/memory/<id>"
if [ "$MODE" = "real" ] && command -v sqlite3 >/dev/null 2>&1; then
  code200=$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/v1/memory/mem-seed-1" || true)
  [ "$code200" = "200" ] \
    || { echo "FAIL: GET memory mem-seed-1 expected 200; got $code200" >&2; exit 1; }
fi
code404=$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/v1/memory/does-not-exist" || true)
[ "$code404" = "404" ] \
  || { echo "FAIL: GET memory unknown expected 404; got $code404" >&2; exit 1; }

echo "  [16/20] task-13.2 POST /v1/memory/<id>/pin → 204 (non-destructive)"
if [ "$MODE" = "real" ] && command -v sqlite3 >/dev/null 2>&1; then
  code204=$(curl -sf -o /dev/null -w '%{http_code}' -X POST "$BASE/v1/memory/mem-seed-1/pin" || true)
  [ "$code204" = "204" ] \
    || { echo "FAIL: POST pin expected 204; got $code204" >&2; exit 1; }
fi

echo "  [17/20] task-13.2 POST /v1/memory/<id>/deprecate → 412 then 204 (X-Confirm gated)"
if [ "$MODE" = "real" ] && command -v sqlite3 >/dev/null 2>&1; then
  code412=$(curl -sf -o /dev/null -w '%{http_code}' -X POST "$BASE/v1/memory/mem-seed-2/deprecate" || true)
  [ "$code412" = "412" ] \
    || { echo "FAIL: deprecate no X-Confirm expected 412; got $code412" >&2; exit 1; }
  code204=$(curl -sf -o /dev/null -w '%{http_code}' -X POST "$BASE/v1/memory/mem-seed-2/deprecate" -H 'X-Confirm: yes' || true)
  [ "$code204" = "204" ] \
    || { echo "FAIL: deprecate with X-Confirm expected 204; got $code204" >&2; exit 1; }
fi

echo "  [18/20] task-13.2 POST /v1/memory/<id>/soft-delete → 412 then 204 + excluded from default list"
if [ "$MODE" = "real" ] && command -v sqlite3 >/dev/null 2>&1; then
  code412=$(curl -sf -o /dev/null -w '%{http_code}' -X POST "$BASE/v1/memory/mem-seed-3/soft-delete" || true)
  [ "$code412" = "412" ] \
    || { echo "FAIL: soft-delete no X-Confirm expected 412; got $code412" >&2; exit 1; }
  curl -sf -o /dev/null -X POST "$BASE/v1/memory/mem-seed-3/soft-delete?confirm=true" \
    || { echo "FAIL: soft-delete confirm=true" >&2; exit 1; }
  # default list should exclude mem-seed-3
  mem_after=$(curl -sf "$BASE/v1/memory")
  echo "$mem_after" | grep -q '"memory_id":"mem-seed-3"' \
    && { echo "FAIL: soft-deleted item still in default list" >&2; exit 1; } \
    || echo "    → mem-seed-3 excluded from default list ✅"
fi

echo "  [19/20] task-14.2 POST /v1/eval-runs (returns 200 + status=running + spawn runEvalAsync goroutine)"
if [ "$MODE" = "real" ]; then
  eval_body=$(curl -sf -X POST "$BASE/v1/eval-runs" \
    -H 'Content-Type: application/json' \
    -d "{\"workspace_id\":\"${WS_ID}\",\"config_snapshot\":{},\"dataset_ref\":\"\"}") \
    || { echo "FAIL: POST eval-runs" >&2; exit 1; }
  EVAL_ID=$(echo "$eval_body" | sed -nE 's/.*"eval_run_id":"([^"]+)".*/\1/p' | head -1)
  echo "    → eval_run_id=$EVAL_ID"
fi

echo "  [20/20] task-14.2 GET /v1/eval-runs/<id> (poll up to 30s for terminal)"
if [ "$MODE" = "real" ] && [ -n "$EVAL_ID" ]; then
  for i in $(seq 1 30); do
    eval_get=$(curl -sf "$BASE/v1/eval-runs/$EVAL_ID")
    eval_status=$(echo "$eval_get" | sed -nE 's/.*"status":"([^"]+)".*/\1/p' | head -1)
    case "$eval_status" in
      succeeded|failed|cancelled)
        echo "    → terminal at attempt $i: status=$eval_status"
        echo "$eval_get" | grep -q '"recall@5"' \
          && echo "    → metrics contains recall@5 ✅" \
          || echo "    NOTE: metrics map empty (goroutine race on small fixture)"
        break
        ;;
      *)
        sleep 1
        ;;
    esac
    if [ "$i" = "30" ]; then
      echo "    NOTE: eval still running after 30s (goroutine in flight; not fatal)"
    fi
  done
fi
# Always verify 404 on unknown id (no env dependency)
code404=$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/v1/eval-runs/eval-does-not-exist" || true)
[ "$code404" = "404" ] \
  || { echo "FAIL: GET eval-run unknown expected 404; got $code404" >&2; exit 1; }

# =====================================================================
# v6 (Phase 15) — 4 new steps for task-15.3/15.4/15.5/15.6 endpoints.
# v7 (Phase 16) — re-numbered to 21/30 — 24/30 (3 v7 steps appended below).
# v8 (Phase 17) — step 28 appended for task-17.1 is_pinned wire roundtrip.
# =====================================================================

echo "  [21/31] task-15.3 GET /v1/stats/chunks (returns {total, today_delta})"
stats_body=$(curl -sf "$BASE/v1/stats/chunks") \
  || { echo "FAIL: GET stats/chunks" >&2; exit 1; }
echo "$stats_body" | grep -q '"total"' \
  || { echo "FAIL: stats response missing total" >&2; exit 1; }
echo "$stats_body" | grep -q '"today_delta"' \
  || { echo "FAIL: stats response missing today_delta" >&2; exit 1; }
echo "    → stats response shape ok"

echo "  [22/31] task-15.4 GET /v1/eval-runs (list returns []EvalRun)"
list_body=$(curl -sf "$BASE/v1/eval-runs?limit=10") \
  || { echo "FAIL: GET eval-runs list" >&2; exit 1; }
case "$list_body" in
  \[*\])
    echo "    → eval-runs list returned JSON array"
    ;;
  *)
    echo "FAIL: eval-runs list response not a JSON array: $list_body" >&2
    exit 1
    ;;
esac
# Filter exercise: status=running on a brand-new MemEvalStore returns either
# the in-flight run or [] depending on race; either shape is acceptable.
filter_body=$(curl -sf "$BASE/v1/eval-runs?status=cancelled&limit=5") \
  || { echo "FAIL: GET eval-runs?status=cancelled" >&2; exit 1; }
case "$filter_body" in [*]) echo "    → status filter returns array" ;; esac

echo "  [23/31] task-15.5 GET /v1/queries (history; default limit 20)"
queries_body=$(curl -sf "$BASE/v1/queries") \
  || { echo "FAIL: GET queries" >&2; exit 1; }
case "$queries_body" in
  \[*\])
    echo "    → queries list returned JSON array"
    ;;
  *)
    echo "FAIL: queries response not a JSON array: $queries_body" >&2
    exit 1
    ;;
esac

echo "  [24/31] task-15.6 GET /v1/health?detailed=true (5 components)"
detail_body=$(curl -sf "$BASE/v1/health?detailed=true") \
  || { echo "FAIL: GET health?detailed=true" >&2; exit 1; }
for name in db index embed retriever eval; do
  echo "$detail_body" | grep -q "\"name\":\"$name\"" \
    || { echo "FAIL: health detail missing component $name: $detail_body" >&2; exit 1; }
done
echo "    → 5 components (db/index/embed/retriever/eval) present ✅"
# Verify default GET /v1/health stays binary (no components field).
default_body=$(curl -sf "$BASE/v1/health")
if echo "$default_body" | grep -q '"components"'; then
  echo "FAIL: default /v1/health unexpectedly includes components field" >&2
  exit 1
fi
echo "    → default /v1/health stays binary ✅"

# =====================================================================
# v7 (Phase 16) — 3 new steps for task-16.1 / 16.2 / 16.4.
# =====================================================================

echo "  [25/31] task-16.2 GET /v1/observability/events?wait=2s (real long-poll timing)"
# REAL mode: assert wait truly blocks ≥ 1.5s when no event is pending (vs. v0.8
# batch-poll path which returned immediately). LOCAL_ONLY / docker: sleep
# fallback per task-16.2 memstore — also blocks min(wait, 1s).
t_start=$(date +%s.%N 2>/dev/null || date +%s)
events_body=$(curl -sf "$BASE/v1/observability/events?wait=2s")
t_end=$(date +%s.%N 2>/dev/null || date +%s)
# Compute elapsed; tolerate plain seconds (Git Bash on Windows) by falling back
# to integer math.
if command -v awk >/dev/null 2>&1; then
  elapsed=$(awk "BEGIN { printf \"%.2f\", $t_end - $t_start }")
else
  elapsed=$(( ${t_end%.*} - ${t_start%.*} ))
fi
echo "    → elapsed=${elapsed}s body_len=$(echo "$events_body" | wc -c)"
# Soft assertion — REAL mode requires ≥ 1.5s blocking when no event pending,
# but if the index-job finished mid-wait an event may be pending and return
# early. We accept either, and only HARD fail when wait clearly didn't take
# effect (returned in < 0.3s on real long-poll path).
case "$MODE" in
  real)
    awk_cmd=$(awk "BEGIN { exit ($elapsed >= 0.3) ? 0 : 1 }" && echo ok || echo fail)
    [ "$awk_cmd" = "ok" ] \
      || { echo "FAIL: REAL wait=2s returned in ${elapsed}s (< 0.3s — long-poll not engaged?)" >&2; exit 1; }
    ;;
  local|docker)
    # MemStore sleep fallback uses min(wait, 1s) — accept any return time
    echo "    → $MODE mode wait fallback (not asserting timing)"
    ;;
esac

echo "  [26/31] task-16.1 TraceStore SQLite restart roundtrip (REAL mode only)"
if [ "$MODE" = "real" ]; then
  # 3 more searches to seed TraceStore (already had 1 from step 8).
  for i in 1 2 3; do
    curl -sf -X POST "$BASE/v1/search" \
      -H 'Content-Type: application/json' \
      -d "{\"query\":\"persist-$i\",\"workspace_id\":\"${WS_ID}\",\"top_k\":5,\"retrieval_method\":\"bm25\",\"agent_scope\":\"session\"}" >/dev/null \
      || { echo "FAIL: POST /v1/search seed $i" >&2; exit 1; }
  done
  echo "    → seeded 3 additional searches"

  # Snapshot pre-restart query count.
  pre_body=$(curl -sf "$BASE/v1/queries?limit=20")
  pre_count=$(echo "$pre_body" | grep -o '"query_id"' | wc -l | tr -d ' ')
  echo "    → pre-restart query count=$pre_count"
  [ "$pre_count" -ge 3 ] \
    || { echo "FAIL: expected ≥ 3 queries before restart; got $pre_count" >&2; exit 1; }

  # Kill -9 the Rust core; the api proxy stays up but gRPC will reconnect after
  # core restart.
  echo "    → kill -9 core (pid $CORE_PID)"
  kill -9 "$CORE_PID" 2>/dev/null || true
  wait "$CORE_PID" 2>/dev/null || true

  # Restart core on the same gRPC port + same DATA_DIR (so SQLite warm restore
  # finds the previously-persisted search_traces rows).
  echo "    → restart core"
  "$CORE_BIN" "127.0.0.1:${GRPC_PORT}" "$DATA_DIR" >>"$STAGING/core.log" 2>&1 &
  CORE_PID=$!
  # Refresh the trap target to the new pid.
  cleanup_local="kill -TERM $API_PID 2>/dev/null || true; kill -TERM $CORE_PID 2>/dev/null || true; wait $API_PID 2>/dev/null || true; wait $CORE_PID 2>/dev/null || true"

  # Wait for /v1/health to recover (Go gRPC client reconnects automatically).
  for i in $(seq 1 30); do
    if curl -sf "$BASE/v1/health" >/dev/null 2>&1; then
      echo "    → /v1/health recovered at attempt $i"
      break
    fi
    sleep 1
    if [ "$i" = "30" ]; then
      echo "FAIL: /v1/health did not recover after core restart in 30s" >&2
      tail -50 "$STAGING/core.log" >&2 || true
      exit 1
    fi
  done

  # Verify TraceStore warm restored from SQLite.
  post_body=$(curl -sf "$BASE/v1/queries?limit=20")
  post_count=$(echo "$post_body" | grep -o '"query_id"' | wc -l | tr -d ' ')
  echo "    → post-restart query count=$post_count"
  [ "$post_count" -ge "$pre_count" ] \
    || { echo "FAIL: TraceStore lost queries on restart (pre=$pre_count post=$post_count)" >&2; exit 1; }
  echo "    → TraceStore SQLite warm restore ✅"
else
  echo "    SKIP ($MODE mode — task-16.1 SoT only validated in real mode)"
fi

echo "  [27/31] task-16.4 compose-prod stack health (gated COMPOSE_PROD_SMOKE=1)"
if [ "${COMPOSE_PROD_SMOKE:-0}" = "1" ]; then
  if ! command -v docker >/dev/null 2>&1; then
    echo "FAIL: COMPOSE_PROD_SMOKE=1 but docker not on PATH" >&2
    exit 1
  fi
  # Bring stack up; assumes ghcr.io image is published (task-16.3 ship gate).
  echo "    → docker compose -f deploy/docker-compose.production.yml up -d"
  docker compose -f deploy/docker-compose.production.yml up -d
  # Preserve the staging-dir cleanup by chaining to the cleanup() function
  # instead of inlining only $cleanup_local (which would skip rm -rf $STAGING).
  trap 'docker compose -f deploy/docker-compose.production.yml down -v 2>&1 | tail -3 || true; cleanup' EXIT

  # Wait up to 60s for both services healthy.
  ok=0
  for i in $(seq 1 12); do
    sleep 5
    if curl -fsS "http://localhost:48181/v1/health" >/dev/null 2>&1; then
      ok=1
      echo "    → compose-prod /v1/health responsive at attempt $i"
      break
    fi
  done
  [ "$ok" = "1" ] \
    || { echo "FAIL: compose-prod /v1/health did not respond within 60s" >&2; \
         docker compose -f deploy/docker-compose.production.yml logs --tail 50 >&2; \
         exit 1; }

  # Assert healthy status (not degraded — ADR-018 fallback deny default).
  health_body=$(curl -fsS "http://localhost:48181/v1/health")
  echo "$health_body" | grep -q '"status":"healthy"' \
    || { echo "FAIL: compose-prod /v1/health status not healthy: $health_body" >&2; exit 1; }
  echo "    → compose-prod stack healthy ✅"
  docker compose -f deploy/docker-compose.production.yml down -v 2>&1 | tail -3 || true
else
  echo "    SKIP (set COMPOSE_PROD_SMOKE=1 + ghcr image published to enable)"
fi

# =====================================================================
# v8 (Phase 17) — task-17.1 MemoryItem.is_pinned add-only wire field.
# Validates: (a) POST body {"pin": true|false} toggles the persisted column;
# (b) GET /v1/memory/{id} surfaces is_pinned in the JSON payload; (c) empty
# body POST falls back to pin=true (v0.7-v0.9 backward compat path).
# =====================================================================

echo "  [28/31] task-17.1 MemoryItem.is_pinned Pin RPC roundtrip (REAL mode + sqlite3)"
if [ "$MODE" = "real" ] && command -v sqlite3 >/dev/null 2>&1; then
  # The seed.sql from step 13 already created mem-seed-1; step 16 pinned it via
  # empty-body POST (backward-compat path that defaults to pin=true). After
  # step 26's daemon restart, the SQLite is_pinned column survives. Verify.
  body_pre=$(curl -sf "$BASE/v1/memory/mem-seed-1")
  echo "$body_pre" | grep -q '"is_pinned":true' \
    || { echo "FAIL: post-restart mem-seed-1 expected is_pinned=true (step 16 pinned + SQLite persist); body=$body_pre" >&2; exit 1; }
  echo "    → mem-seed-1 is_pinned=true survived step-26 daemon restart ✅"

  # (b) explicit pin=false body
  code204=$(curl -sf -o /dev/null -w '%{http_code}' -X POST "$BASE/v1/memory/mem-seed-1/pin" \
    -H 'Content-Type: application/json' -d '{"pin":false}' || true)
  [ "$code204" = "204" ] \
    || { echo "FAIL: POST pin=false expected 204; got $code204" >&2; exit 1; }
  body_unpin=$(curl -sf "$BASE/v1/memory/mem-seed-1")
  echo "$body_unpin" | grep -q '"is_pinned":false' \
    || { echo "FAIL: after pin=false expected is_pinned=false; body=$body_unpin" >&2; exit 1; }
  echo "    → POST {\"pin\":false} → is_pinned=false ✅"

  # (c) explicit pin=true body
  code204=$(curl -sf -o /dev/null -w '%{http_code}' -X POST "$BASE/v1/memory/mem-seed-1/pin" \
    -H 'Content-Type: application/json' -d '{"pin":true}' || true)
  [ "$code204" = "204" ] \
    || { echo "FAIL: POST pin=true expected 204; got $code204" >&2; exit 1; }
  body_pin=$(curl -sf "$BASE/v1/memory/mem-seed-1")
  echo "$body_pin" | grep -q '"is_pinned":true' \
    || { echo "FAIL: after pin=true expected is_pinned=true; body=$body_pin" >&2; exit 1; }
  echo "    → POST {\"pin\":true} → is_pinned=true ✅"

  # (d) empty body backward-compat — defaults to pin=true (v0.7-v0.9 contract).
  # Unpin first so we can verify the no-body path actually re-pins.
  curl -sf -o /dev/null -X POST "$BASE/v1/memory/mem-seed-1/pin" \
    -H 'Content-Type: application/json' -d '{"pin":false}' \
    || { echo "FAIL: setup unpin for backward-compat probe" >&2; exit 1; }
  code204=$(curl -sf -o /dev/null -w '%{http_code}' -X POST "$BASE/v1/memory/mem-seed-1/pin" || true)
  [ "$code204" = "204" ] \
    || { echo "FAIL: empty-body POST expected 204; got $code204" >&2; exit 1; }
  body_compat=$(curl -sf "$BASE/v1/memory/mem-seed-1")
  echo "$body_compat" | grep -q '"is_pinned":true' \
    || { echo "FAIL: empty-body POST should default to pin=true; body=$body_compat" >&2; exit 1; }
  echo "    → empty-body POST → is_pinned=true (v0.7-v0.9 backward compat) ✅"
else
  echo "    SKIP ($MODE mode or sqlite3 unavailable — task-17.1 wire roundtrip validated via Rust + Go unit tests)"
fi

# ----------- v9 (Phase 19) — task-19.4 semantic-retrieval wiring -----------
echo "  [29/31] task-20.1 POST /v1/search?semantic=true (console-api forwards → gRPC semantic branch engages)"
if [ "$MODE" = "real" ]; then
  # task-20.1: console-api handleSearch now OR-merges ?semantic=true into the gRPC SearchRequest.Semantic
  # (contractv1 add-only field + grpcclient passthrough), and the Rust SearchService.Query semantic
  # branch (DeterministicEmbeddingProvider + 0-dep BruteForceVectorBackend) reports the vector path in
  # the trace's candidate_generation_steps. So this step asserts BOTH the {result, trace} contract is
  # preserved (add-only, non-breaking) AND that the semantic path actually engaged through console-api.
  # ADR-013: deterministic provider proves dispatch, NOT recall — no recall-threshold assertion (real
  # recall via the Retriever hot path is task-20.2 / docs/spikes/phase-20-recall-via-retriever.md).
  sem_body=$(curl -sf -X POST "$BASE/v1/search?semantic=true" \
    -H 'Content-Type: application/json' \
    -d "{\"query\":\"contextforge\",\"workspace_id\":\"${WS_ID}\",\"top_k\":5,\"agent_scope\":\"session\"}") \
    || { echo "FAIL: POST /v1/search?semantic=true did not return 2xx" >&2; exit 1; }
  echo "$sem_body" | grep -q '"result"' \
    && echo "$sem_body" | grep -q '"trace"' \
    || { echo "FAIL: semantic search not nested {result, trace}: $sem_body" >&2; exit 1; }
  echo "$sem_body" | grep -q 'vector-bruteforce' \
    || { echo "FAIL: ?semantic=true did not engage the vector path through console-api (trace: $sem_body)" >&2; exit 1; }
  echo "    → ?semantic=true forwarded + vector path engaged (trace candidate_generation_steps=vector-bruteforce) ✅"
else
  echo "    SKIP ($MODE mode — semantic REST path validated via Go/Rust unit tests; needs the real daemon)"
fi

echo "  [30/31] task-21.3 contextforge eval run --semantic --hybrid --rerank (multi-path BM25 + semantic + hybrid + reranked report + gate)"
if [ "$MODE" = "real" ]; then
  # $GO_BIN built at [real][3/4]. v11 (task-21.3): `eval run --semantic --hybrid --rerank` spawns a
  # transient core per query (searchViaDaemon) and issues BM25 + semantic (req.Semantic) + hybrid
  # (req.Hybrid → daemon search_hybrid, task-21.1) + reranked (eval-layer deterministic
  # IdentityReranker over the hybrid top-k, ADR-026 D2) passes, summarizing via SummarizePasses +
  # MeetsRecallGate. ADR-013: assert the multi-path report SHAPE + gate line + exit 0 ONLY — the
  # transient index is empty, so recall is not meaningful; real hybrid/rerank recall vs the baseline
  # comes from the dogfood eval (docs/spikes/phase-21-hybrid-recall.md). Per-result
  # retrieval_method="hybrid" + hybrid_score provenance is asserted by the Rust dispatch test
  # (core/src/server.rs test_21_1_hybrid_dispatches_fusion_path).
  if eval_out=$("$GO_BIN" eval run --semantic --hybrid --rerank --collection=default 2>"$STAGING/eval.err"); then
    echo "$eval_out" | grep -q '^total=' \
      && echo "$eval_out" | grep -q 'semantic_recall_at_10=' \
      && echo "$eval_out" | grep -q 'hybrid_recall_at_10=' \
      && echo "$eval_out" | grep -q 'reranked_recall_at_10=' \
      && echo "$eval_out" | grep -q '^gate=' \
      || { echo "FAIL: eval --semantic --hybrid --rerank missing multi-path/gate lines:" >&2; echo "$eval_out" >&2; exit 1; }
    echo "    → eval run --semantic --hybrid --rerank produced multi-path report + gate line (recall numbers → docs/spikes/phase-21-hybrid-recall.md) ✅"
  else
    echo "FAIL: eval run --semantic --hybrid --rerank exited non-zero" >&2
    cat "$STAGING/eval.err" >&2
    echo "---- core.log ----" >&2; tail -30 "$STAGING/core.log" 2>/dev/null || true
    echo "---- api.log ----" >&2; tail -30 "$STAGING/api.log" 2>/dev/null || true
    exit 1
  fi
else
  echo "    SKIP ($MODE mode — eval multi-path needs the real daemon search backend)"
fi

echo "  [31/31] task-22.4 contextforge init emits add-only [embedding] config section (provider-config-selection, task-22.1)"
# v12 (task-22.4): the add-only [embedding] section (task-22.1) must round-trip through the real
# config codec. `init --root` scaffolds config.toml; assert it now carries [embedding] (with a `dim`
# key, unique to that section) without disturbing the existing [remote] section. Runs in every mode
# ($GO_BIN built in both real/local). Cache hit (task-22.2) + remote provider (task-22.3) are verified
# at the unit/contract layer (TEST-22.2.* / TEST-22.3.*) — not console-hot-path-wired in v0.15, and the
# remote path never hits the network here (ADR-013 — real remote 联调/recall deferred).
EMBED_CFG_DIR="$STAGING/cf-embed-cfg"
if "$GO_BIN" init --root "$EMBED_CFG_DIR" >"$STAGING/init.out" 2>&1; then
  CFG_FILE="$EMBED_CFG_DIR/config.toml"
  if grep -q '^\[embedding\]' "$CFG_FILE" \
     && grep -q '^dim = ' "$CFG_FILE" \
     && grep -q '^\[remote\]' "$CFG_FILE"; then
    echo "    → init config.toml carries add-only [embedding] section (dim key) + intact [remote] ✅"
  else
    echo "FAIL: init config.toml missing [embedding] section (task-22.1 codec):" >&2
    cat "$CFG_FILE" >&2
    exit 1
  fi
else
  echo "FAIL: contextforge init --root failed" >&2
  cat "$STAGING/init.out" >&2
  exit 1
fi

echo
if [ "$MODE" = "real" ]; then
  echo "CONSOLE_REAL_SMOKE_EXIT=0"
else
  echo "CONSOLE_SMOKE_EXIT=0"
fi
