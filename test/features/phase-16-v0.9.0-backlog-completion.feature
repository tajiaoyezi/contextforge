# language: en
# Maps to:
#   - docs/specs/phases/phase-16-v0.9.0-backlog-completion.md
#   - docs/specs/tasks/task-16.1-tracestore-sqlite-persistence.md
#   - docs/specs/tasks/task-16.2-events-real-long-poll.md
#   - docs/specs/tasks/task-16.3-ghcr-image-push-ci.md
#   - docs/specs/tasks/task-16.4-compose-production-example.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: phase-16 v0.9.0-backlog-completion
  In order to close ContextForge-Console PR #91/#93 backlog P3+P4 items in v0.9.0
  As a ContextForge release operator
  I want TraceStore SQLite persistence / events real long-poll / ghcr image push CI / docker-compose.production.yml

  # ---
  # Maps to: docs/specs/tasks/task-16.1-tracestore-sqlite-persistence.md
  Scenario: SCEN-16.1.1 — 对应 AC1 (migration creates search_traces table)
    Given a fresh data_dir without a search_traces.db file
    When SqliteTracePersist::open(data_dir) is called
    Then the search_traces.db file is created with the search_traces table and idx_search_traces_ts_desc index

  Scenario: SCEN-16.1.2 — 对应 AC2 (put + list SQLite roundtrip recent-first)
    Given a SqliteTracePersist with 5 traces persisted at different ts_unix
    When list(limit=3) is invoked
    Then the result returns 3 PbQueryRecord entries ordered by ts_unix DESC

  Scenario: SCEN-16.1.3 — 对应 AC3 (warm restore across daemon restart)
    Given a TraceStore with persist where 3 traces have been put before
    When the daemon process is killed and a new TraceStore::with_persist is instantiated against the same data_dir
    Then the new TraceStore returns the 3 historical PbQueryRecord entries via list

  Scenario: SCEN-16.1.4 — 对应 AC4 (write-through best-effort on SQLite error)
    Given a TraceStore whose persist SqliteTracePersist.put returns an error
    When TraceStore.put is invoked with a new key
    Then the in-memory LRU still contains the new entry and a warning is logged but the call does not panic

  # ---
  # Maps to: docs/specs/tasks/task-16.2-events-real-long-poll.md
  Scenario: SCEN-16.2.1 — 对应 AC2 (wait 5s blocks on no event then returns [])
    Given a running console-api-serve daemon with no observability events being emitted
    When a client calls HTTP GET /v1/observability/events?wait=5s
    Then the response arrives after at least 4.5 seconds with status 200 and body []

  Scenario: SCEN-16.2.2 — 对应 AC3 (wait 5s returns early on event arrival)
    Given a running console-api-serve daemon with an indexing.progress event arriving within 1 second
    When a client calls HTTP GET /v1/observability/events?wait=5s
    Then the response arrives within 200ms after the event with status 200 and body containing at least 1 event

  Scenario: SCEN-16.2.3 — 对应 AC4 (concurrent clients independent)
    Given a running console-api-serve daemon
    When two clients in parallel call HTTP GET /v1/observability/events?wait=2s simultaneously
    Then both receive their independent timeout-driven 200 + [] responses without one blocking the other

  Scenario: SCEN-16.2.4 — 对应 AC5 (MemStore fallback sleep then empty)
    Given a MemStore in fallback mode with an empty in-memory event ring buffer
    When Recent(limit=10, wait=2s) is invoked
    Then the call sleeps min(wait, 1s) and returns an empty slice without error

  # ---
  # Maps to: docs/specs/tasks/task-16.3-ghcr-image-push-ci.md
  Scenario: SCEN-16.3.1 — 对应 AC2 (tag push triggers workflow)
    Given the .github/workflows/release.yml workflow registered with on.push.tags ['v*']
    When an annotated tag v0.9.0-rc1 is pushed to origin
    Then GitHub Actions starts the build-and-push job and completes within 10 minutes

  Scenario: SCEN-16.3.2 — 对应 AC3 (docker pull tag is healthy)
    Given the ghcr.io/${owner}/contextforge-daemon:v0.9.0-rc1 image has been pushed
    When the user runs docker pull ghcr.io/${owner}/contextforge-daemon:v0.9.0-rc1 and docker run with CONSOLE_API_FALLBACK_INMEM=1 opt-in
    Then the container becomes healthy and HTTP GET /v1/health returns 200

  Scenario: SCEN-16.3.3 — 对应 AC4 (latest tag tracks newest)
    Given the build-and-push job has just pushed v0.9.0-rc1 with two tags v0.9.0-rc1 and latest
    When the user runs docker pull ghcr.io/${owner}/contextforge-daemon:latest
    Then the resulting image digest matches the v0.9.0-rc1 tag digest

  Scenario: SCEN-16.3.4 — 对应 AC5 (ci.yml PR gate)
    Given a pull request opened against master branch
    When .github/workflows/ci.yml triggers cargo-test, go-test, spec-lint as three parallel jobs
    Then all three jobs report success on the PR check before the PR can be merged

  # ---
  # Maps to: docs/specs/tasks/task-16.4-compose-production-example.md
  Scenario: SCEN-16.4.1 — 对应 AC1 + AC2 (compose up two-service stack healthy + /v1/health 200 healthy)
    Given a Docker host with docker compose v2 and the ghcr image v0.9.0 available
    When the operator runs docker compose -f deploy/docker-compose.production.yml up -d
    Then within 30 seconds both contextforge-core and console-api-serve become healthy and HTTP GET /v1/health returns 200 with status "healthy"

  Scenario: SCEN-16.4.2 — 对应 AC3 (named volume persistence across restart)
    Given a running compose-prod stack with one workspace created via POST /v1/workspaces
    When the operator runs docker compose -f deploy/docker-compose.production.yml restart and waits for healthy
    Then HTTP GET /v1/workspaces returns the previously-created workspace via the persisted named volume

  Scenario: SCEN-16.4.3 — 对应 AC4 (down -v destroys volume; fresh start)
    Given a running compose-prod stack with data in the contextforge-data named volume
    When the operator runs docker compose -f deploy/docker-compose.production.yml down -v and then up -d
    Then the new stack starts with empty workspace and index-job tables (fresh data_dir)

  Scenario: SCEN-16.4.4 — 对应 AC8 (legacy dev/PoC console-stack.yml unchanged)
    Given the existing deploy/console-stack.yml from v0.7 dev/PoC stack
    When docker compose -f deploy/console-stack.yml config is validated after Phase 16 ship
    Then the legacy yml continues to parse successfully and its semantics remain unchanged
