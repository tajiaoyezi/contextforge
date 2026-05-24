# ContextForge

ContextForge is a local-first context indexing and retrieval tool for agent memory, rules, source files, logs, and project notes.

It ships as two binaries (ADR-001):

- `contextforge`: Go control-plane CLI, REST/MCP adapter, Console Contract v1 REST surface (`console-api-serve`, v0.3+), export and eval entrypoint.
- `contextforge-core`: Rust data-plane daemon for scan, parse, chunk, index, and retrieval.

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
- **EventBus broadcast** + `/v1/observability/events` long-poll (`?wait=<duration>`
  default 30s, max 60s; `?limit=<int>` default 100): JobRunner heartbeat 真 emit
  `indexing.progress` / `indexing.cancelled` / `indexing.error` 事件.
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
