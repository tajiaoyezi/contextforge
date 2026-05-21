# NEEDS-DEP — task-3.4-importer-agent-rules

**Date**: 2026-05-21
**Agent**: grok (per dispatch)

## Declared dependencies
- None (R7 compliance)

## Rationale
- Reuses `internal/importer` framework, `buildRecord`, `recordInput` and `Importer` interface from task-3.1 (already Done).
- Uses only frozen proto `contextforge.v1.ContextRecord` + stdlib (`os`, `path/filepath`, `strings`, `log`).
- No changes to `go.mod`, `go.sum`, `Cargo.toml`, `Cargo.lock`.
- Registration via `init()` + blank import pattern (standard Go, no new runtime dep).
- Future CLI wiring (Phase 6) will `_ "github.com/tajiaoyezi/contextforge/internal/importer/agentrules"` to trigger auto-register; no dep impact.

## Alternatives considered
- Put agent-rules logic inside `internal/importer/` flat (rejected: violates "每个 importer 是独立 Go 子包" for parallel 3.2/3.3/3.4 write isolation).
- Add markdown parser lib (rejected: overkill, 3.1 fallback already handles raw markdown content; rules are treated as opaque text + metadata tags).

## Verification
- `go build ./internal/importer/agentrules/...` (will pass with no new require)
- `go test ./internal/importer/...` (after impl)
- No `go mod tidy` delta expected.

If main agent later adds shared helper, this task does not block.
