Feature: Console Real Data Plane (Phase 11)
  As a Console UI integrator
  I want ContextForge REST endpoints backed by real Rust data plane
  So that workspace + index job + search + events are persistent and functional

  Background:
    Given a clean ContextForge install with empty data_dir
    And contextforge-core daemon listens on 127.0.0.1:48180 gRPC
    And console-api-serve listens on 127.0.0.1:48181 REST + proxies to gRPC

  Scenario: Workspace 持久化跨 daemon 重启 (Phase §6 AC3)
    Given contextforge-core daemon running + console-api-serve running
    When I POST /v1/workspaces with name="persist-test" and root_path="/tmp/cf-fixture"
    And the daemon is killed and restarted
    Then GET /v1/workspaces returns the workspace
    And the workspace has the same workspace_id

  Scenario: IndexJob 真触发 Rust JobRunner (Phase §6 AC4 + task-11.3 §6 AC1/AC2)
    Given a fixture repo at test/fixtures/index-job-real with 5 markdown files
    And a workspace pointing to the fixture exists
    When I POST /v1/index-jobs with workspace_id and trigger_source="console-ui"
    Then within 1s the IndexJob status transitions queued -> running
    And within 30s the IndexJob status transitions running -> succeeded
    And processed_files == 5
    And total_files == 5
    And error_message is null

  Scenario: Search 真返回 indexed 分块 (Phase §6 AC4 + task-11.4 §6 AC1)
    Given an IndexJob has succeeded for the fixture repo
    When I POST /v1/search with q="contextforge" and top_k=5
    Then the SearchResult contains at least 1 SourceChunk
    And each chunk has score > 0
    And each chunk has a source_file path matching the fixture repo
    And the RetrievalTrace.retrieved_chunks contains at least 1 entry with chunk_id + score + source_file + content_snippet

  Scenario: Events long-poll 30s 真接 JobRunner progress (Phase §6 AC5 + task-11.4 §6 AC3/AC4)
    Given an IndexJob is running on the fixture repo
    When I GET /v1/observability/events with wait=30s
    Then I receive at least 1 indexing.progress event within 5s
    And each event has job_id matching the running IndexJob
    And each event has processed_files and total_files numeric fields

  Scenario: Cancel in-flight job 真停 (Phase §6 AC5 + task-11.3 §6 AC3)
    Given a large fixture repo (≥20 files) is being indexed
    And the IndexJob status is "running"
    When I POST /v1/index-jobs/<id>/cancel
    Then within 5s the IndexJob status becomes "cancelled"
    And processed_files < total_files (cancel mid-progress)
    And the CancelToken atomic flag was observed as true by the IndexSession callback

  Scenario: gRPC 不可达 + fallback 未启用 = degraded 503 (Phase §6 AC2 + task-11.2 §6 AC4)
    Given contextforge-core daemon is NOT running
    And console-api-serve is running WITHOUT CONSOLE_API_FALLBACK_INMEM
    When I GET /v1/health
    Then the HTTP status code is 503
    And the response body has degraded=true
    And the response body has missing=["data_plane"]

  Scenario: gRPC 不可达 + fallback 启用 = degraded but functional (ADR-016 D4)
    Given contextforge-core daemon is NOT running
    And console-api-serve is running WITH CONSOLE_API_FALLBACK_INMEM=1
    When I POST /v1/workspaces with name="fallback-test"
    Then the HTTP status code is 201 (created in in-memory MemStore)
    And GET /v1/health returns degraded=true and store="inmem-fallback"
    And a log warning "console-api: using in-memory fallback store" was emitted

  Scenario: Daemon kill mid-index = orphan reaper marks failed (task-11.3 §6 AC4)
    Given an IndexJob is running on a large fixture
    When the contextforge-core daemon receives SIGKILL mid-progress
    And the daemon is restarted with the same data_dir
    Then the orphan reaper marks the in-progress job status=failed
    And the error_message contains "job lost: daemon restart"

  Scenario: Rust SoT - Go does not write SQLite (ADR-016 D1)
    Given a fresh ContextForge install
    When console-api-serve receives POST /v1/workspaces
    Then no SQLite file is opened by the console-api-serve process
    And the workspace row exists in core/data/workspaces.db (Rust-managed)
    And the workspace row owner process is contextforge-core

  Scenario: Conformance test 仍 PASS (v0.3 不回归) (task-11.2 §6 AC5)
    Given CONSOLE_REPO=H:/devlopment/code/ContextForge-Console is set
    And CONSOLE_API_FALLBACK_INMEM=1 is set (conformance test mode)
    When I run go test ./test/conformance/... -run TestConsoleContractV1Conformance
    Then the test PASSes
    And the 9 endpoint flow + FieldAvailability.Complete() + sentinel mapping are verified

  Scenario: gRPC 字段命名 snake_case 与 Go contractv1 1:1 (ADR-016 D3 + task-11.1 §6 AC1)
    Given the proto file core/proto/console_data_plane.proto compiled
    When I run grpcurl -plaintext 127.0.0.1:48180 describe contextforge.console_data_plane.v1.Workspace
    Then all field names are snake_case
    And every field name matches a JSON tag in internal/contractv1/contractv1.go

  Scenario: EventBus broadcast 容量 1000 + Lagged 不破坏 stream (task-11.4 §6 AC4)
    Given an EventBus with broadcast capacity 1000 is running
    And a slow subscriber falls behind by 1500 events
    When the subscriber recv resumes
    Then the broadcast::RecvError::Lagged is logged as warning
    And the stream continues without break
    And new events flow normally
