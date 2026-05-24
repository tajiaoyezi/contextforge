Feature: Console Contract Completion 22-endpoint (Phase 12/13/14)
  As a Console UI integrator
  I want ContextForge REST endpoints to cover the full 22-endpoint Contract v1 surface
  So that ContextForge-Console HTTPAdapter conformance suite passes 22/22 and Console UI can run in production HTTP mode (no Mock)

  # =====================================================================
  # Phase 12 — Wave 1 quick win (task-12.1)
  # =====================================================================

  Scenario: PATCH /v1/workspaces/{id}/config 缺 X-Confirm 返 412
    Given contextforge-core daemon running + console-api-serve running
    And a workspace exists with id "ws-1"
    When I send PATCH /v1/workspaces/ws-1/config body '{"allowlist":["*.go"],"denylist":[]}' without X-Confirm header
    Then I receive HTTP status 412 PRECONDITION_FAILED
    And the response body contains code "PRECONDITION_FAILED"
    And the body mentions "X-Confirm: yes header or ?confirm=true query required"

  Scenario: PATCH /v1/workspaces/{id}/config with X-Confirm: yes 头 returns updated Workspace
    Given a workspace exists with id "ws-1"
    When I send PATCH /v1/workspaces/ws-1/config body '{"allowlist":["*.md"],"denylist":[]}' with header X-Confirm: yes
    Then I receive HTTP status 200
    And the response body contains "workspace_id":"ws-1"
    And the response body contains "*.md" in allowlist (via config_snapshot)

  Scenario: PATCH /v1/workspaces/{id}/config with ?confirm=true 也接受
    Given a workspace exists with id "ws-1"
    When I send PATCH /v1/workspaces/ws-1/config?confirm=true body '{"allowlist":["*.rs"],"denylist":[]}' without X-Confirm header
    Then I receive HTTP status 200
    And the response body contains "*.rs" in allowlist

  Scenario: GET /v1/index-jobs?status=active filters out terminal
    Given an active index job exists with id "job-1" status queued
    And a terminal index job exists with id "job-2" status succeeded
    When I send GET /v1/index-jobs?status=active
    Then I receive HTTP status 200
    And the response array contains job_id "job-1"
    And the response array does not contain job_id "job-2"

  Scenario: POST /v1/index-jobs/{id}/cancel returns 204 No Content
    Given an active index job exists with id "job-1" status running
    When I send POST /v1/index-jobs/job-1/cancel
    Then I receive HTTP status 204
    And the response body is empty

  Scenario: POST /v1/index-jobs/{id}/cancel on terminal job returns 409 Conflict
    Given a terminal index job exists with id "job-2" status succeeded
    When I send POST /v1/index-jobs/job-2/cancel
    Then I receive HTTP status 409 CONFLICT

  # =====================================================================
  # Phase 12 — Wave 2 mid scope (task-12.2 + task-12.3)
  # =====================================================================

  Scenario: GET /v1/source-chunks/{id} 真返 SourceChunk
    Given an index job has succeeded for fixture repo and created chunk "chunk-1"
    When I send GET /v1/source-chunks/chunk-1
    Then I receive HTTP status 200
    And the response body contains "chunk_id":"chunk-1"
    And the response body contains "source_file_path"
    And the response body contains "line_start" and "line_end" as integers

  Scenario: GET /v1/source-chunks/{id} for unknown chunk returns 404
    When I send GET /v1/source-chunks/nonexistent-chunk-id
    Then I receive HTTP status 404 NOT_FOUND

  Scenario: GET /v1/search/{query_id}/trace returns RetrievalTrace after POST search
    Given I have executed a POST /v1/search and received result.query_id "q-abc"
    When I send GET /v1/search/q-abc/trace
    Then I receive HTTP status 200
    And the response body contains "trace_id"
    And the response body contains "candidate_generation_steps" as array
    And the response body contains "final_context_count" as integer

  Scenario: GET /v1/search/{query_id}/trace for unknown query returns 404
    When I send GET /v1/search/q-unknown/trace
    Then I receive HTTP status 404 NOT_FOUND

  Scenario: LRU trace store evicts oldest at capacity 1000
    Given the trace store has 1000 entries with query_ids q-0 through q-999
    When I execute POST /v1/search resulting in a new query_id "q-1000"
    And q-0 is evicted (FIFO)
    Then GET /v1/search/q-0/trace returns 404
    And GET /v1/search/q-1000/trace returns 200

  # =====================================================================
  # Phase 13 — Wave 3 memory (task-13.1 + task-13.2)
  # =====================================================================

  Scenario: GET /v1/memory list returns seeded memory items
    Given memory_items table contains 5 fixture items (agent_scope variety)
    When I send GET /v1/memory
    Then I receive HTTP status 200
    And the response array contains 5 MemoryItem objects
    And each item has memory_id, agent_scope, content_preview, source_type, status fields

  Scenario: GET /v1/memory with agent_id filter returns matching items only
    Given memory_items table contains items with agent_scope "agent-1/session" (3 items) and "agent-2/global" (2 items)
    When I send GET /v1/memory?agent_id=agent-1
    Then I receive HTTP status 200
    And the response array contains 3 items
    And all items have agent_scope starting with "agent-1"

  Scenario: GET /v1/memory/{id} returns single MemoryItem
    Given a memory item exists with id "mem-1"
    When I send GET /v1/memory/mem-1
    Then I receive HTTP status 200
    And the response body contains "memory_id":"mem-1"

  Scenario: POST /v1/memory/{id}/pin returns 204 (non-destructive, no X-Confirm needed)
    Given a memory item exists with id "mem-1"
    When I send POST /v1/memory/mem-1/pin without X-Confirm header
    Then I receive HTTP status 204
    And subsequent GET /v1/memory/mem-1 returns is_pinned=true

  Scenario: POST /v1/memory/{id}/deprecate without X-Confirm returns 412
    Given a memory item exists with id "mem-1"
    When I send POST /v1/memory/mem-1/deprecate without X-Confirm header
    Then I receive HTTP status 412 PRECONDITION_FAILED

  Scenario: POST /v1/memory/{id}/deprecate with X-Confirm: yes returns 204 + updates status
    Given a memory item exists with id "mem-1" status active
    When I send POST /v1/memory/mem-1/deprecate with X-Confirm: yes
    Then I receive HTTP status 204
    And subsequent GET /v1/memory/mem-1 returns status="deprecated"
    And an audit log entry is written with op_type="deprecate" for memory_id mem-1

  Scenario: POST /v1/memory/{id}/soft-delete with X-Confirm excludes item from default list
    Given a memory item exists with id "mem-1" status active
    When I send POST /v1/memory/mem-1/soft-delete with X-Confirm: yes
    Then I receive HTTP status 204
    And subsequent GET /v1/memory does not contain mem-1
    And subsequent GET /v1/memory?include_soft_deleted=true contains mem-1 with status="soft_deleted"
    And an audit log entry is written with op_type="soft_delete" for memory_id mem-1

  # =====================================================================
  # Phase 14 — Wave 4 eval (task-14.1 + task-14.2)
  # =====================================================================

  Scenario: POST /v1/eval-runs returns 200 + EvalRun status="running"
    Given a workspace exists with id "ws-1"
    And a golden_questions fixture exists at "test/fixtures/eval-seed/golden_questions.jsonl"
    When I send POST /v1/eval-runs body '{"workspace_id":"ws-1","config_snapshot":{},"dataset_ref":"test/fixtures/eval-seed/golden_questions.jsonl"}'
    Then I receive HTTP status 200
    And the response body contains "eval_run_id"
    And the response body contains "status":"running"
    And the response body contains "started_at" with RFC3339Nano timestamp
    And the response body contains "finished_at":null

  Scenario: GET /v1/eval-runs/{id} returns EvalRun with metrics after recall harness completes
    Given an eval run exists with id "eval-1" and recall harness is spawned
    When I poll GET /v1/eval-runs/eval-1 every 1s until status terminal (max 60s)
    Then within 60s the response body contains "status":"succeeded" (or "failed")
    And the response body contains "metrics" with key "recall@5" as float64
    And the response body contains "case_results" as non-empty array

  Scenario: GET /v1/eval-runs/{id} for unknown id returns 404
    When I send GET /v1/eval-runs/eval-nonexistent
    Then I receive HTTP status 404 NOT_FOUND

  # =====================================================================
  # Phase 14 closeout — Console 22-endpoint conformance全 PASS
  # =====================================================================

  Scenario: Console 22-endpoint conformance suite passes 22/22 on v0.7.0
    Given ContextForge v0.7.0 contextforge-core + console-api-serve are running
    And CONSOLE_REPO env points to ContextForge-Console repo with conformance suite
    When the Console HTTPAdapter conformance test runs against ContextForge
    Then 22/22 endpoint behaviors match the contract (path / shape / status / headers / X-Confirm semantics)
    And the test reports PASS for: Health(1) + Workspace(4) + IndexJob(4) + Search(3) + Memory(5) + Eval(2) + Observability(1) + SourceChunk(1) + SearchTrace(1)
    And no Mock adapter is required for Console UI demo

  # =====================================================================
  # Cross-cutting — RFC3339Nano + long-poll + 4 trade-off (ADR-017 D2/D3/D4/D5)
  # =====================================================================

  Scenario: ContextForge JSON output uses RFC3339Nano for time.Time fields (ADR-017 D5)
    When I send GET /v1/workspaces/ws-1
    Then the response body contains "created_at" matching pattern "\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z"
    And Console Zod schema accepts the format (datetime with precision: 9)

  Scenario: GET /v1/observability/events long-poll (v1.0 lock per ADR-017 D4)
    Given no events have been emitted yet
    When I send GET /v1/observability/events?wait=2s
    Then within ~2s I receive HTTP status 200
    And the response body is an empty JSON array (long-poll timed out without events)
    And the response Content-Type is application/json (not text/event-stream — SSE deferred to v1.x)
