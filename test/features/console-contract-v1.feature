Feature: console-contract-v1 — Phase 10 console-contract-v1 cross-repo integration
  As a ContextForge maintainer
  I want ContextForge to expose Console Contract v1 compatible REST API + resources
  So that ContextForge-Console v1.0 (already shipped) can call ContextForge in HTTPAdapter mode (not just Mock)

  Background:
    Given Console Contract v1 must-have fields are aligned to Console PRD §Technical Approach
    And Console contractv1.go is the single source of truth (cross-repo mirror)
    And ADR-015 governs Console Contract v1 compatibility decisions
    And ADR-014 cross-validation gate applies to all Phase 10 spec PRs

  # task-10.1 — contractv1-types
  Scenario: SCEN-10.1.1 internal/contractv1 mirrors Console contractv1.go 17 types
    Given Console contractv1.go defines 17 Contract v1 must-have types
    When I open internal/contractv1/contractv1.go
    Then it contains 17 corresponding Go types with identical json tags
    And it contains ContractVersion = "v1" constant
    And it contains FieldAvailability struct with Complete() / IsMissing() helpers

  Scenario: SCEN-10.1.2 JSON roundtrip preserves nullable field semantics
    Given an IndexJob with finished_at = nil and error_message = nil
    When I marshal it to JSON and unmarshal back
    Then finished_at deserializes as nil pointer (not zero time.Time)
    And error_message deserializes as nil pointer (not empty string)

  # task-10.2 — workspace-resource
  Scenario: SCEN-10.2.1 workspace_id ↔ collection_id 1:1 mapping
    Given a fresh SqliteWorkspaceStore opened at <data_dir>
    When I create a workspace named "demo"
    Then a row exists in workspaces table with workspace_id = collection_id
    And the underlying collection dir is physically created at <data_dir>/collections/<workspace_id>/

  Scenario: SCEN-10.2.2 invalid input returns WorkspaceError::Invalid
    When I call workspace create with empty name
    Then it returns WorkspaceError::Invalid (not panic)

  # task-10.3 — indexjob-resource
  Scenario: SCEN-10.3.1 IndexJob lifecycle queued → running → succeeded
    Given a workspace exists with workspace_id "demo"
    When I enqueue an IndexJob via JobStore.enqueue
    Then the IndexJob status is "queued" initially
    When JobRunner.run_one is invoked
    Then the IndexJob status transitions queued → running → succeeded within reasonable time

  Scenario: SCEN-10.3.2 IndexJob co-operative cancel
    Given an IndexJob is in "running" state
    When I call JobStore.request_cancel(job_id)
    Then within 2 seconds the IndexJob status transitions to "cancelled"
    And processed_files at cancel-time is preserved

  # task-10.4 — rest-endpoints
  Scenario: SCEN-10.4.1 GET /v1/health returns contract_version "v1"
    Given the ContextForge daemon is running
    When I send GET /v1/health
    Then the response is 200 OK
    And the JSON body contains "contract_version":"v1"
    And the JSON body contains "status":"healthy"

  Scenario: SCEN-10.4.2 POST /v1/index-jobs with body {workspace_id} returns queued IndexJob
    Given a workspace "demo" exists
    When I send POST /v1/index-jobs with body {"workspace_id":"demo"}
    Then the response is 200 OK
    And the JSON body contains IndexJob with status "queued"

  Scenario: SCEN-10.4.3 POST /v1/search returns {result, trace} nested response
    Given workspace "demo" has indexed content
    When I send POST /v1/search with a query
    Then the response is 200 OK
    And the JSON body has top-level "result" and "trace" keys (Console HTTPAdapter convention)

  Scenario: SCEN-10.4.4 GET /v1/workspaces/:non-existent returns 404
    When I send GET /v1/workspaces/non-existent-id
    Then the response is 404 Not Found
    And the response body contains an error object

  Scenario: SCEN-10.4.5 bearer auth enforced when token env set
    Given CONTEXTFORGE_CONSOLEAPI_AUTH_TOKEN is set
    When I send any request without Authorization header
    Then the response is 401 Unauthorized

  # task-10.5 — conformance-test
  Scenario: SCEN-10.5.1 conformance test against Console fakehttpserver oracle
    Given env CONSOLE_REPO is set to Console repo path
    When I run go test ./test/conformance/... -run TestConsoleContractV1Conformance
    Then ContextForge daemon starts and 9 endpoint flow PASSes
    And all returned contractv1 types have FieldAvailability.Complete() == true

  Scenario: SCEN-10.5.2 conformance test skips gracefully when env not set
    Given env CONSOLE_REPO is not set
    When I run the conformance test
    Then it skips (t.Skip) and exits 0

  # task-10.6 — console-integration-smoke
  Scenario: SCEN-10.6.1 docker compose starts full Console + ContextForge stack
    Given docker compose v2 is available
    When I run bash scripts/console_smoke.sh
    Then ContextForge daemon starts on port 48181 and reports contract_version "v1"
    And Console UI starts on port 3000
    And POST /api/workspaces via Console BFF creates a workspace in ContextForge SQLite
    And GET /api/workspaces returns the created workspace (not Mock data)
    And the script ends with CONSOLE_SMOKE_EXIT=0
