# ContextForge

ContextForge is a local-first context indexing and retrieval tool for agent memory, rules, source files, logs, and project notes.

It ships as two binaries (ADR-001):

- `contextforge`: Go control-plane CLI, REST/MCP adapter, Console Contract v1 REST surface (`console-api-serve`, v0.3+), export and eval entrypoint.
- `contextforge-core`: Rust data-plane daemon for scan, parse, chunk, index, and retrieval.

## What's new in v0.8.0

🎯 **Console functional gap closure** — closes 6/11 backlog items raised by the Console team (Console PR #91/#93) covering P0 fallback / memory event bridge + P1 Dashboard backend endpoints + P2 5-link health detail. Console UI Dashboard 3 KPI cards and CoreHealthCard now have backend data; Memory 详情面板 "操作历史" 列表 auto-populates via the new memory.* event stream.

- **Phase 15 console-functional-gap-closure** (ADR-020 / ADR-021) — 6 task PRs (#99-#104) + closeout PR (#105). Includes:
  - **task-15.1** MemStore chunk/trace cache — `CONSOLE_API_FALLBACK_INMEM=1` mode no longer 503s on drill-down `GET /v1/source-chunks/<id>` / `GET /v1/search/<query_id>/trace` (cache the stub Search emits)
  - **task-15.2 (ADR-021)** memory.* → EventBus bridge — `memory.pin` / `memory.deprecate` / `memory.soft_delete` events broadcast to `/v1/observability/events` stream
  - **task-15.3** `GET /v1/stats/chunks` — `{total, today_delta}` for Dashboard "已索引块" KPI
  - **task-15.4** `GET /v1/eval-runs` (list) — filter by `?workspace_id=&status=&limit=N`, ORDER BY started_at DESC
  - **task-15.5** `GET /v1/queries` — query history from in-memory trace store, default limit 20
  - **task-15.6 (ADR-020)** `GET /v1/health?detailed=true` — opt-in 5-component breakdown (db / index / embed / retriever / eval)
- **ADR-015 D1 add-only**: All proto / Go schema changes purely additive — Console v0.7.x clients reading v0.8 responses silently ignore the new fields; existing 22-endpoint conformance test PASS (no regression).
- **ADR-014 cross-validation gate 6th activation** — D1 mapping table + D2 lint 0 hits + D3 verified-by + D4/D5 governance — see PR #105 body.
- **ADR-020 / ADR-021 Accepted** (2026-05-26, Phase 15 closeout PR #105).

Remaining Console backlog (deferred to Phase 16 / v0.9.0):
- P2 #6 `MemoryItem.is_pinned` (ADR-015 D5 amendment) · P3 #8 ghcr.io image push · P3 #9 docker-compose.production.yml example · P4 #10 TraceStore SQLite persist · P4 #11 `?wait=` real long-poll.

详 `RELEASE_NOTES.md` v0.8.0 段 + [ADR-020](docs/decisions/adr-020-health-component-breakdown.md) + [ADR-021](docs/decisions/adr-021-memory-event-bus-bridge.md)。

## What's new in v0.7.2

⚠️ **BREAKING — fallback-inmem default reversal** (ADR-018):

- 删 Dockerfile `ENV CONSOLE_API_FALLBACK_INMEM=1` → daemon 默认 fallback **deny**
- `docker run contextforge-daemon:v0.7.2` 不显式 opt-in → `/v1/health` 返 **503** + docker healthcheck unhealthy
- 保留 v0.7.1 行为需 `docker run -e CONSOLE_API_FALLBACK_INMEM=1 ...` 显式 opt-in
- 修复 v0.7.1 silent footgun：HTTP 200 healthcheck 掩盖容器重启数据失风险
- 代码无改动；仅 Dockerfile 删 ENV 行 + ADR-018 spec lock + ratification test

详 `RELEASE_NOTES.md` v0.7.2 段 + [ADR-018](docs/decisions/adr-018-fallback-inmem-default-reversal.md)。

## What's new in v0.7.1

🐳 **Dockerfile + single-image deployment fix** — v0.7.0 Dockerfile 4 处 stale
(rust 1.82 → 1.93 / go 1.22 → 1.26 / ENV `CONSOLE_API_FALLBACK_INMEM=1` 占位 /
新 `.dockerignore` 排 9.3 GB cargo cache) 一次性收齐。

注：v0.7.1 的 ENV 行已在 v0.7.2 移除（fallback 默认从 enable 反转为 deny，
silent footgun fix）。详 v0.7.2 段。

## What's new in v0.7.0

🎉 **Console 22-endpoint conformance 100% PASS** — ContextForge ships all 22
Console contract v1 REST endpoints; Console UI HTTPAdapter v1.0 双方握手成功.

- **Phase 14 eval-rest-surface** (ADR-017 Wave 4) — Console Contract v1
  endpoint coverage 18 → 20 distinct routes (covering all 22 contract
  endpoints, 2 shared via filter shape). 22-endpoint conformance now 100%.
- **task-14.1 Rust SoT**: SQLite `eval_runs` table (migration 0014) +
  `SqliteEvalStore` (5 methods, JSON-roundtrip metrics + case_results) +
  `EvalService` 3 gRPC RPCs (Create / Get / UpdateProgress) — Create returns
  `EvalRun{status:"running", started_at:now}`; UpdateProgress is the Go-side
  runner callback channel that persists status terminal + metrics + case_results.
- **task-14.2 Go REST**: 2 routes (`POST /v1/eval-runs` + `GET /v1/eval-runs/{id}`)
  + `runEvalAsync` goroutine that drives a light-weight recall harness against
  BuiltinGoldenQuestions and reverse-updates the Rust store via UpdateProgress
  when terminal. `MemEvalStore` fallback (2s timer auto-advance to succeeded
  with mock metrics).
- **`console_smoke.sh` v5**: 18 → 20 endpoint REAL flow; new Steps 19/20 cover
  POST eval-runs (200 + status=running) + poll GET until terminal + verify
  metrics contains `recall@5`. Final marker `CONSOLE_REAL_SMOKE_EXIT=0`.
- **ADR-017 Status: Proposed → Accepted** — 6 D-clauses spanning v0.5/v0.6/v0.7
  3 phase one-shot promoted. ADR-014 cross-validation gate **5th activation**
  pass — 制度稳定性跨 5 phase (v0.3-v0.7) 验证.

## What's new in v0.6.0

- **Phase 13 memory-rest-surface** (ADR-017 Wave 3) — Console Contract v1
  endpoint coverage 13 → 18 (the 5 memory endpoints). 22-endpoint conformance
  bumps 64% → 82%.
- **task-13.1 Rust SoT**: SQLite `memory_items` table (migration 0013) + 自研
  `SqliteMemoryStore` (10 columns; status CHECK constraint; 3 indexes) +
  `MemoryService` 5 gRPC RPCs (List / Get / Pin / Deprecate / SoftDelete) +
  Pin/Deprecate/SoftDelete each emit `AuditOperation::Memory*` events via
  shared `AuditSink` (4 new enum variants).
- **task-13.2 Go REST**: 5 routes (`GET /v1/memory[?agent_id=&scope=&namespace=&include_soft_deleted=]`
  + `GET /v1/memory/{id}` + `POST /v1/memory/{id}/pin` non-destructive 204 +
  `POST /v1/memory/{id}/deprecate` + `POST /v1/memory/{id}/soft-delete` —
  the last two confirmMiddleware-gated, returning 412 PRECONDITION_FAILED
  without X-Confirm/?confirm=true). `MemMemoryStore.SeedFixtures()` provides
  5 in-memory items for `CONSOLE_API_FALLBACK_INMEM=1` demo.
- **`console_smoke.sh` v4**: 13 → 18 endpoint REAL flow; new Steps 13-18 cover
  memory seed (sqlite3 CLI) + list + get + pin 204 + deprecate 412/204 +
  soft-delete 412/204 with default-list exclusion. Final marker
  `CONSOLE_REAL_SMOKE_EXIT=0`.
- ADR-014 cross-validation gate **fourth activation** pass — 制度稳定性
  跨 4 phase 验证.
- ADR-017 Status still **Proposed** (full Accepted promotion deferred to
  Phase 14 closeout).

## What's new in v0.5.0

- **Phase 12 console-contract-completion** (ADR-017 Wave 1+2) — Console Contract
  v1 endpoint coverage 9 → 13 (route inventory 9 → 14 including the new GET
  `/v1/source-chunks/{id}` and GET `/v1/search/{query_id}/trace`). 22-endpoint
  conformance bumps from 41% → 64%.
- **task-12.1 quick-win Wave 1**: `PATCH /v1/workspaces/{id}/config` overwrites
  allowlist + denylist via gRPC `WorkspaceService.UpdateConfig`; `GET /v1/index-jobs?status=active`
  filters queued + running via `JobService.List`; `POST /v1/index-jobs/{id}/cancel`
  switches **200 → 204 No Content** (ADR-017 D3); `confirmMiddleware` enforces
  `X-Confirm: yes` header **OR** `?confirm=true` query for destructive ops →
  412 Precondition Failed (ADR-017 D2 server-side bottom defense).
- **task-12.2 source-chunk-by-id**: new `SearchService.GetSourceChunk` RPC +
  Go REST handler reuse existing `Retriever::get_chunk` (task-6.2 SQL fast-path);
  workspace_id optional with workspace enumeration fallback.
- **task-12.3 search-trace-by-query-id**: new `SearchService.GetSearchTrace`
  RPC + in-memory `TraceStore` (HashMap + VecDeque LRU cap 1000); every
  `SearchService.Query` generates a unique `qry-{nanos}` query_id and persists
  its `RetrievalTrace` for later GET by query_id (daemon restart wipes cache —
  SQLite persistence `[SPEC-DEFER:task-future.search-trace-sqlite-persistence]`).
- **`console_smoke.sh` v3**: 9 → 13 endpoint REAL flow; new Steps 9-12 cover
  PATCH config 412→200, active filter + 400, source-chunks lookup, trace fetch.
  Final marker `CONSOLE_REAL_SMOKE_EXIT=0`.
- ADR-014 cross-validation gate **third activation** pass — 制度稳定性 verified.
- ADR-017 Status remains **Proposed** (full Accepted promotion deferred to Phase
  14 closeout where 6 D-clauses across v0.5/v0.6/v0.7 land together).

## What's new in v0.4.0

- **Phase 11 console-real-data-plane** (ADR-016) — `console-api-serve` 默认行为
  从 v0.3 in-memory MemStore 切到 **cross-process gRPC bridge**: Go REST handler
  → `internal/consoleapi/grpcclient` → Rust `core/src/data_plane/` 4 gRPC service
  (Workspace / Job / Search / Events) → SqliteWorkspaceStore + SqliteJobStore.
- **真索引 + 真搜索**: `POST /v1/index-jobs` 真触发 `JobRunner.spawn_blocking(
  IndexSession::index_path_with_progress)`; `POST /v1/search` 真接 retriever
  (Tantivy + SQLite chunks). v0.3 占位 stub 完全 retired.
- **EventBus broadcast** + `/v1/observability/events` batch polling (`?wait=<duration>`
  + `?limit=<int>` 参数 reserved；v0.7.x REST tier 实现是 immediate batch return，
  不真 block — long-poll honoring 留 [SPEC-DEFER:task-future.consoleapi-sse-or-long-poll-honoring]；
  Console 端应自管轮询频率，详 `docs/releases/v0.7.0-integration.md` §7).
  JobRunner heartbeat 真 emit `indexing.progress` / `indexing.cancelled` /
  `indexing.error` 事件.
- **`console_smoke.sh` v2 REAL mode default**: spawns both daemons + drives
  fixture index + verifies real chunks. Final marker
  `CONSOLE_REAL_SMOKE_EXIT=0`. `LOCAL_ONLY=1` retains v0.3 inmem fallback.
- **`--fallback-inmem` env-gated**: `CONSOLE_API_FALLBACK_INMEM=1` keeps v0.3
  behavior available for demo / fallback / conformance test.
- ADR-014 cross-validation gate **second activation** pass —制度稳定性验证.

## What's new in v0.3.0

- **ContextForge ↔ ContextForge-Console Contract v1 兼容层** (ADR-015) — 17 Go types
  mirror Console `contractv1.go` + Rust workspace/jobs resource models +
  9 REST endpoints under `/v1/*`.
- New CLI subcommand: `contextforge console-api-serve --addr 127.0.0.1:48181`
  exposes the 9 Console Contract v1 endpoints (in-memory store for v0.3; cross-
  process Rust ↔ Go SQLite sharing is v0.4 follow-up).
- New smoke: `bash scripts/console_smoke.sh` exercises the 9 endpoint flow end-to-end.
- Docker compose stack: `deploy/console-stack.yml` + multi-stage `Dockerfile`.
- ADR-014 cross-validation gate fully activated for Phase 10 (D1 mapping table +
  D2 lint + D3 verified-by + D4/D5 in `scripts/spec_drift_lint.sh`).

## Quick Start

### One-shot smoke (Linux / WSL2 / Git Bash on Windows)

```bash
bash scripts/quickstart_smoke.sh
```

Builds both binaries, drives the seven-step CLI walkthrough end-to-end against
the [examples/quickstart/](examples/quickstart/) fixture, and prints
`QUICKSTART_SMOKE_EXIT=0` on success.

### Manual steps

```bash
# 1. Build (Go 1.22+, Rust stable).
go build -o contextforge ./cmd/contextforge
cargo build -p contextforge-core
export PATH="$(pwd):$(pwd)/target/debug:$PATH"
export CONTEXTFORGE_DATA_DIR="$HOME/.contextforge-demo"

# 2. Initialise the data root.
contextforge init --root "$CONTEXTFORGE_DATA_DIR"

# 3. Import the Hermes memory fixture (writes canonical .md to <data-dir>/imports/hermes/).
contextforge import hermes examples/quickstart/hermes-memory \
  --collection demo --data-dir "$CONTEXTFORGE_DATA_DIR"

# 4. Index the imported records.
contextforge index --source "$CONTEXTFORGE_DATA_DIR/imports/hermes" \
  --collection demo --data-dir "$CONTEXTFORGE_DATA_DIR"

# 5. Index the sample project (denylist skips .env; secret-redaction rewrites config.yaml AWS key).
contextforge index --source examples/quickstart/sample-project \
  --collection demo --data-dir "$CONTEXTFORGE_DATA_DIR"

# 6. Search — the `configuration` keyword lives in docs/config.md.
contextforge search --collections=demo --top-k=5 --explain "configuration"

# 7. Eval (builtin 30 golden questions).
contextforge eval run --collection demo
```

Notes on flag order: `--collections=demo` (and any other flag) MUST precede the
positional query when invoking `contextforge search` — the stdlib `flag` parser
stops at the first non-flag argument.

### Expected output

- Step 2 prints `contextforge: initialized <data-dir> (schema_version 0.1)`.
- Step 4–5 stream `\rindexing <file> (files=N, chunks=M)` lines, then a final
  `done collection=demo files=… chunks=… denied=… redacted=…` summary.
- Step 6 emits one block per hit: `<chunk_id>  <file>:<start>-<end>  score=…  redaction_status=…` plus a `reason=…` line.
- Step 7 prints the eval report (`Top-5`, `Top-10`, `latency`, optional miss
  list).

## v0.2 limitations

- Official target: Linux x86_64 / WSL2; macOS should work, Windows is best
  effort via Git Bash.
- `LICENSE` remains all-rights-reserved (occupies the slot until an OSI
  license is chosen).
- No GitHub Release tarball is published from this repo yet — release
  contracts and a release-smoke gate are in place
  (`scripts/release_smoke.sh`, `scripts/quickstart_smoke.sh`), but the
  actual `gh release create` step is performed by an external release job.

## Where to go next

- `contextforge.example.toml` — starting point for collection allowlists and
  local-only provider settings.
- [`docs/specs/phases/phase-9-cli-pipeline.md`](docs/specs/phases/phase-9-cli-pipeline.md) — the v0.2 CLI data-plane phase that re-enabled the manual
  Quick Start sequence above.
- [`docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`](docs/decisions/adr-013-cli-data-plane-grpc-bridge.md) — context on why
  v0.1's `contextforge index` / `contextforge import` were stubs and what
  Phase 9 changed.
