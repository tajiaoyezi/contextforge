# language: en
# Maps to:
#   - docs/specs/tasks/task-9.1-proto-index-rpc.md
#   - docs/specs/tasks/task-9.2-rust-grpc-index.md
#   - docs/specs/tasks/task-9.3-go-cli-index.md
#   - docs/specs/tasks/task-9.4-go-cli-import.md
#   - docs/specs/tasks/task-9.5-release-smoke-real.md
#   - docs/specs/tasks/task-9.6-readme-quickstart-verified.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then 实测细节。

Feature: cli-pipeline
  In order to close v0.1 CLI data-plane spec drift (ADR-013) and ship a real end-to-end CLI flow
  As a v0.2 release owner
  I want contextforge init → import → index → search → eval to work as a copy-pasteable command sequence

  # ---
  # Maps to: docs/specs/tasks/task-9.1-proto-index-rpc.md
  Scenario: SCEN-9.1.1 — 对应 AC1（service.proto 含 rpc Index）
    Given the proto/contextforge/v1/service.proto file
    When the ContextService block is inspected
    Then it contains "rpc Index(IndexRequest) returns (stream IndexProgress);"

  Scenario: SCEN-9.1.2 — 对应 AC2（index.proto 7 字段）
    Given the new proto/contextforge/v1/index.proto file
    When the IndexRequest and IndexProgress messages are inspected
    Then IndexRequest has 3 fields and IndexProgress has 7 fields with the names listed in task-9.1 §5.3

  Scenario: SCEN-9.1.3 — 对应 AC3（Go codegen 全绿）
    Given the regenerated Go protobuf files
    When go vet ./... and go test ./proto/... are run
    Then both commands exit 0 and the new Index method appears on ContextServiceClient and ContextServiceServer

  Scenario: SCEN-9.1.4 — 对应 AC4（Rust codegen 全绿）
    Given the regenerated Rust prost / tonic bindings via core/build.rs
    When cargo check --workspace is run
    Then it exits 0 and the new Index trait method is present on the ContextService trait

  Scenario: SCEN-9.1.5 — 对应 AC5（baseline 不回归）
    Given the unchanged existing CLI/MCP/eval call sites that use rpc Search and rpc Health
    When go test ./internal/cli/... ./internal/daemon/... and cargo test --workspace are run
    Then no existing test regresses (baseline green preserved)

  # ---
  # Maps to: docs/specs/tasks/task-9.2-rust-grpc-index.md
  Scenario: SCEN-9.2.1 — 对应 AC1（index_path_with_progress 回调按文件粒度触发）
    Given an IndexSession opened on a temp data_dir
    When index_path_with_progress is called with a fixture of 3 normal files
    Then the on_progress callback is invoked at least 3 times with files_processed incrementing per file

  Scenario: SCEN-9.2.2 — 对应 AC2（index_path thin wrapper 兼容）
    Given the existing core/tests/phase2_smoke.rs and phase6_smoke.rs callers of index_path
    When cargo test --test phase2_smoke and --test phase6_smoke are run unchanged
    Then both still pass (binary compatible)

  Scenario: SCEN-9.2.3 — 对应 AC3（校验阶段错误映射）
    Given a tonic in-process server with CoreService::index wired
    When the client calls index with source_path = "/nonexistent/path/should/not/exist"
    Then the stream is never established and the client receives tonic::Status::InvalidArgument

  Scenario: SCEN-9.2.4 — 对应 AC4（stream 进度上报）
    Given a tonic in-process server with CoreService::index wired
    When the client calls index with a fixture of N normal files
    Then the client receives at least N+1 IndexProgress messages with the final one having done=true and error==""

  Scenario: SCEN-9.2.5 — 对应 AC5（phase9_index_smoke 端到端）
    Given a temp data_dir + a temp source_path with 3 .md + 1 .env (denied) + 1 secret-redacted .yaml
    When cargo test --test phase9_index_smoke is run
    Then the smoke passes: SQLite chunks > 0, Tantivy hits the fixture marker, .env is skipped, and the .yaml secret is redacted

  # ---
  # Maps to: docs/specs/tasks/task-9.3-go-cli-index.md
  Scenario: SCEN-9.3.1 — 对应 AC1（Daemon.Index 包装）
    Given a fake gRPC server that streams 3 IndexProgress messages then closes
    When daemon.Index(ctx, req) is called and the channels are consumed
    Then 3 progress messages are received, progressCh closes, and errCh emits nil then closes

  Scenario: SCEN-9.3.2 — 对应 AC2（CLI 真实索引 + 人类输出）
    Given a fake daemon stub that yields N progress messages with current_file populated
    When runIndex is called in human mode
    Then stdout contains \r-overwrite progress lines plus a final summary, and the exit code is 0

  Scenario: SCEN-9.3.3 — 对应 AC3（CLI --json mode）
    Given the same fake daemon stub
    When runIndex is called with --json
    Then each stdout line is a valid JSON object containing files_processed / chunks_written / current_file / done / error

  Scenario: SCEN-9.3.4 — 对应 AC4（集成端到端）
    Given a fresh temp data_dir + temp source with 3 .md + 1 .env + 1 secret-redacted .yaml + a freshly cargo-built core binary
    When the real contextforge CLI runs index against that source
    Then exit code is 0, SQLite chunks > 0, Tantivy hits the fixture marker, .env is skipped, .yaml secret is redacted

  Scenario: SCEN-9.3.5 — 对应 AC5（--resume 行为）
    Given the integration setup from SCEN-9.3.4
    When runIndex is called twice with --resume (second call after the first completes)
    Then the second call prints "resuming long-task mode" and the manifest ProcessedItems was updated during the first run

  # ---
  # Maps to: docs/specs/tasks/task-9.4-go-cli-import.md
  Scenario: SCEN-9.4.1 — 对应 AC1（hermes import）
    Given a temp data_dir + a Hermes MEMORY.md / USER.md fixture
    When runImport hermes <fixture> --collection demo is called
    Then ≥1 .md file is written under data_dir/imports/hermes/ with YAML frontmatter source_provider=hermes, and stdout contains the next-step "contextforge index --source ..." hint

  Scenario: SCEN-9.4.2 — 对应 AC2（openclaw import）
    Given a temp data_dir + an OpenClaw workspace fixture
    When runImport openclaw <fixture> --collection demo is called
    Then ≥1 .md file is written under data_dir/imports/openclaw/ with YAML frontmatter source_provider=openclaw

  Scenario: SCEN-9.4.3 — 对应 AC3（agent-rules import）
    Given a temp data_dir + an agent-rules fixture (AGENTS.md / CLAUDE.md style)
    When runImport agent-rules <fixture> --collection demo is called
    Then ≥1 .md file is written under data_dir/imports/agent-rules/ with YAML frontmatter source_type=agent_rule

  Scenario: SCEN-9.4.4 — 对应 AC4（unknown importer error）
    Given any runImport invocation
    When the first positional arg is not hermes / openclaw / agent-rules
    Then exit code is 2 and stderr lists the three valid importer names

  Scenario: SCEN-9.4.5 — 对应 AC5（--dry-run）
    Given a Hermes fixture and a --dry-run flag
    When runImport hermes <fixture> --dry-run is called
    Then no files are written to the output dir but stdout still prints the imported count and next-step hint

  # ---
  # Maps to: docs/specs/tasks/task-9.5-release-smoke-real.md
  Scenario: SCEN-9.5.1 — 对应 AC1（真 binary tarball）
    Given the rewritten TestTask83_AC1
    When go test runs that test
    Then the test actually invokes go build and cargo build before constructing the tarball, and asserts the contextforge binary entry has executable mode bits

  Scenario: SCEN-9.5.2 — 对应 AC2（fake-evidence 删除）
    Given the current internal/release/release_test.go after this task
    When grep is run for the pattern Status: StepPassed, Evidence: "ok"
    Then there are zero matches and the AC2 / AC4 fake-evidence test functions are gone

  Scenario: SCEN-9.5.3 — 对应 AC3（真 CLI 端到端）
    Given a fresh temp staging dir with freshly built contextforge + contextforge-core binaries
    When TestPhase9ReleaseSmoke_EndToEnd runs all seven CLI steps
    Then every step exits 0, ValidateSmokeEvidence accepts the real evidence sequence, and the test passes

  Scenario: SCEN-9.5.4 — 对应 AC4（script 集成）
    Given the updated scripts/release_smoke.sh
    When the script runs end-to-end
    Then it includes the phase 9 CLI segment, every segment exits 0, and the script prints PHASE_RELEASE_SMOKE_EXIT=0

  Scenario: SCEN-9.5.5 — 对应 AC5（benchmark 边界）
    Given the retained TestTask83_AC3_BenchmarkRequires100kChunksAndSub500msP95
    When the unit test runs
    Then the validator gate still accepts a passing synthetic BenchmarkReport and rejects an undersized one — real 100k benchmark is documented out-of-scope in task-9.5 §3

  # ---
  # Maps to: docs/specs/tasks/task-9.6-readme-quickstart-verified.md
  Scenario: SCEN-9.6.1 — 对应 AC1（sample-project fixture）
    Given the examples/quickstart/sample-project/ directory
    When its contents are listed
    Then it contains at least 5 .md files plus 1 .env, 1 secret-redacted .yaml, 1 .go, and 1 .log file, all ≤100KB total

  Scenario: SCEN-9.6.2 — 对应 AC2（hermes-memory fixture）
    Given the examples/quickstart/hermes-memory/ directory
    When the Hermes importer Detect() is called on its MEMORY.md and USER.md
    Then both files return ok=true

  Scenario: SCEN-9.6.3 — 对应 AC3（quickstart_smoke.sh 端到端）
    Given a fresh clone of the repo (or a clean working tree)
    When bash scripts/quickstart_smoke.sh is invoked
    Then it exits 0 and prints QUICKSTART_SMOKE_EXIT=0 after seven sequential CLI steps succeed

  Scenario: SCEN-9.6.4 — 对应 AC4（README rewrite）
    Given the updated README.md
    When the Quick Start section is inspected
    Then it references examples/quickstart/ and lists build → init → import → index → search → eval as copy-pasteable commands with real flags

  Scenario: SCEN-9.6.5 — 对应 AC5（v0.2.0 release docs）
    Given the updated RELEASE_NOTES.md and new docs/releases/v0.2.0-{evidence,artifacts}.md
    When the v0.2.0 section is read
    Then it follows the v0.1.0 template format and references ADR-013 + Phase 9 closure
