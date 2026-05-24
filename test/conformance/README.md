# test/conformance/ — cross-repo Contract v1 conformance

This directory holds tests that verify ContextForge satisfies cross-repo
contracts driven by *other* ContextForge-family repositories. Today: only
the **Console** Contract v1 conformance suite (task-10.5 / ADR-015 §D5).

## Running the conformance suite

```bash
export CONSOLE_REPO=/path/to/ContextForge-Console
go test ./test/conformance/... -run TestConsoleContractV1Conformance -v -timeout 60s
```

When `CONSOLE_REPO` is **not** set, the test SKIPs (exit 0). This is the
D5 historical-skip default — CI environments that have not yet vendored
the Console repo are not penalised, but a developer who wants to verify
cross-repo alignment can opt in with one env var.

`CONSOLE_REPO` must point at a clone of **ContextForge-Console** where
`console-api/internal/coreadapter/contractv1/contractv1.go` exists. The
test reads that file to verify the `ContractVersion = "v1"` anchor and
will t.Fatalf if the path resolves but the file is missing.

## Why an embedded mini-client (not the live Console HTTPAdapter)

The test embeds a tiny client (`minimalConsoleHTTPClient` in
`console_contractv1_test.go`) that mirrors the Console
HTTPAdapter's calling pattern (path / verb / body / error-code mapping)
without importing Console's Go module.

Reason: pulling Console as a Go module dep would add a cross-repo build
coupling, and Console is not published on Go module proxies. v0.4 may
revisit via `go.mod replace` once the cross-process SQLite sharing
[`SPEC-DEFER:task-future.cross-process-sqlite-sharing`] story lands.

## What the suite covers (task-10.5 §6 AC)

| AC | Verification |
|----|--------------|
| AC1 | Console-style 9 endpoint flow runs end-to-end |
| AC2 | env `CONSOLE_REPO` set → test PASSes |
| AC3 | env `CONSOLE_REPO` unset → test SKIPs (no fail) |
| AC4 | Every returned contractv1.* object has `FieldAvailability.Complete() == true` |
| AC5 | 404 → Console ErrNotFound / 409 → Console ErrConflict mapping verified |

## What is NOT covered (v0.4+)

- Memory / Eval / Source-chunks endpoints (`SPEC-DEFER:task-future.consoleapi-extension`)
- Long-poll / SSE for `/v1/observability/events`
  (`SPEC-DEFER:task-future.consoleapi-sse`)
- Rust workspace/jobs SQLite sharing with Go REST handlers
  (`SPEC-DEFER:task-future.cross-process-sqlite-sharing`)
- Performance / latency conformance (P95 < 200ms etc.)
