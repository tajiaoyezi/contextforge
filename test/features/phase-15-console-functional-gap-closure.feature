# language: en
# Maps to:
#   - docs/specs/phases/phase-15-console-functional-gap-closure.md
#   - docs/specs/tasks/task-15.1-memstore-chunk-trace-cache.md
#   - docs/specs/tasks/task-15.2-memory-event-bus-bridge.md
#   - docs/specs/tasks/task-15.3-chunks-stats-endpoint.md
#   - docs/specs/tasks/task-15.4-list-eval-runs-endpoint.md
#   - docs/specs/tasks/task-15.5-query-history-endpoint.md
#   - docs/specs/tasks/task-15.6-health-component-detail.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: phase-15 console-functional-gap-closure
  In order to close ContextForge-Console PR #91/#93 backlog 6/11 items in v0.8.0
  As a ContextForge release operator
  I want MemStore cache fallback / memory EventBus bridge / chunks stats / list eval-runs / query history / health 5-component detail

  # ---
  # Maps to: docs/specs/tasks/task-15.1-memstore-chunk-trace-cache.md
  Scenario: SCEN-15.1.1 — 对应 AC1 (chunk cache hit)
    Given a MemStore in fallback mode with no SearchBackend
    When a client first calls POST /v1/search and then GET /v1/source-chunks/{chunk_id}
    Then the chunk request returns 200 with the cached SourceChunk instead of 503

  Scenario: SCEN-15.1.2 — 对应 AC2 (trace cache hit)
    Given a MemStore in fallback mode with no SearchBackend
    When a client first calls POST /v1/search and then GET /v1/search/{query_id}/trace
    Then the trace request returns 200 with the cached RetrievalTrace instead of 503

  Scenario: SCEN-15.1.3 — 对应 AC3 (FIFO eviction at cap=256)
    Given a MemStore with chunk cache capacity 256
    When more than 256 distinct searches populate the cache
    Then the oldest chunk entry is evicted in FIFO order

  # ---
  # Maps to: docs/specs/tasks/task-15.2-memory-event-bus-bridge.md
  Scenario: SCEN-15.2.1 — 对应 AC1 (pin emits memory.pin event)
    Given a MemoryServer wired with EventBus and a subscriber on the events channel
    When a client invokes Pin RPC with pin=true on memory_id mem-1
    Then the subscriber receives an ObservabilityEvent with event_type "memory.pin" and payload_json containing op=pin

  Scenario: SCEN-15.2.2 — 对应 AC2 (deprecate and soft_delete emit corresponding events)
    Given a MemoryServer with EventBus subscriber
    When the client invokes Deprecate RPC and SoftDelete RPC on a memory item
    Then the subscriber receives ObservabilityEvent entries with event_type "memory.deprecate" and "memory.soft_delete"

  Scenario: SCEN-15.2.3 — 对应 AC3 (SendError swallowed when no subscriber)
    Given a MemoryServer with EventBus but no active subscribers
    When the client invokes Pin RPC
    Then the RPC returns 204 successfully and the audit log is written despite the SendError

  # ---
  # Maps to: docs/specs/tasks/task-15.3-chunks-stats-endpoint.md
  Scenario: SCEN-15.3.1 — 对应 AC2 (Tantivy num_docs + SQLite today COUNT)
    Given a Rust SearchService backed by a Tantivy index with N documents
    When a client calls GET /v1/stats/chunks via gRPC SearchService.GetChunksStats
    Then the response includes total = N and today_delta = chunks indexed since today_start_unix

  Scenario: SCEN-15.3.2 — 对应 AC3 (REST endpoint)
    Given a running console-api-serve daemon
    When a client calls HTTP GET /v1/stats/chunks
    Then the response is 200 with JSON {total: int64, today_delta: int64}

  # ---
  # Maps to: docs/specs/tasks/task-15.4-list-eval-runs-endpoint.md
  Scenario: SCEN-15.4.1 — 对应 AC2 (Rust list filter + ORDER BY started_at DESC)
    Given a SqliteEvalStore with 3 eval runs at different started_at timestamps
    When the client calls list with workspace_id=A and limit=10
    Then the result returns rows ordered by started_at DESC where workspace_id matches

  Scenario: SCEN-15.4.2 — 对应 AC3 (REST GET list with filter)
    Given a running console-api-serve daemon backed by SqliteEvalStore with 3 eval runs
    When a client calls HTTP GET /v1/eval-runs?workspace_id=A&status=succeeded&limit=10
    Then the response is 200 with JSON array of EvalRun filtered by query params

  # ---
  # Maps to: docs/specs/tasks/task-15.5-query-history-endpoint.md
  Scenario: SCEN-15.5.1 — 对应 AC2 (TraceStore.list recent-first ordering)
    Given a TraceStore with 5 inserted traces in order T1, T2, T3, T4, T5
    When the client calls list with limit=3
    Then the result returns T5, T4, T3 in that order (most recent first)

  Scenario: SCEN-15.5.2 — 对应 AC4 (REST default limit 20)
    Given a running console-api-serve daemon with 50 trace records in TraceStore
    When a client calls HTTP GET /v1/queries without limit query param
    Then the response is 200 with a JSON array of exactly 20 QueryRecord entries

  # ---
  # Maps to: docs/specs/tasks/task-15.6-health-component-detail.md
  Scenario: SCEN-15.6.1 — 对应 AC2 (5 probes + aggregate)
    Given a Rust HealthChecker wired to a valid data_dir with db / index / eval stores opened
    When check_all is invoked
    Then it returns a DetailedHealth with 5 components named db, index, embed, retriever, eval and an aggregated overall_status

  Scenario: SCEN-15.6.2 — 对应 AC3 (REST ?detailed=true returns components)
    Given a running console-api-serve daemon
    When a client calls HTTP GET /v1/health?detailed=true
    Then the response is 200 with JSON containing a "components" map with 5 keys

  Scenario: SCEN-15.6.3 — 对应 AC3 (REST default GET stays binary)
    Given a running console-api-serve daemon
    When a client calls HTTP GET /v1/health (no detailed query)
    Then the response sticks to the v0.7 binary CoreHealth schema (no components field)

  Scenario: SCEN-15.6.4 — 对应 AC6 (5 probes total latency under 500ms)
    Given a HealthChecker wired to a populated data_dir
    When check_all is invoked under a wall-clock timer
    Then the total elapsed time is under 500ms even when retriever exercise is included
