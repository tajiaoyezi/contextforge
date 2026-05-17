// Package cli is the contextforge control-plane CLI entrypoint: a stdlib
// flag-based subcommand dispatcher (init/import/index/search/serve/mcp/eval/
// export). Unimplemented subcommands return an explicit not-implemented error
// (never panic). task-1.4 (Phase 1 foundation). Contract: task-1.4 §5.3.
//
// §2A decision: v0.1 uses the standard library (no third-party CLI framework)
// — zero new dependency, avoids R7 / a go.mod conflict with the parallel
// task-3.1; cobra (PRD §Technical Approach / D8) is deferred to a later dep PR.
package cli

import "io"

// task-1.4 RED skeleton (§2.5.1 compilable bridge): signatures exist so the
// AC tests compile and fail on *behaviour* (unimplemented) — not on a compile
// error. GREEN replaces these bodies.

// Execute parses args, dispatches the subcommand and returns the process exit
// code. Unknown / unimplemented subcommands write to stderr and return a
// non-zero code — never panic (AC4).
func Execute(args []string, stdout, stderr io.Writer) int {
	panic("unimplemented: cli.Execute (task-1.4 RED skeleton)")
}

// SubcommandNames returns the registered subcommand names in stable order (AC4).
func SubcommandNames() []string {
	panic("unimplemented: cli.SubcommandNames (task-1.4 RED skeleton)")
}

// runInit orchestrates config.Init to generate the default config + data-dir
// scaffold; root=="" resolves to config.DefaultRootDir(); idempotent (AC1).
func runInit(root string, stdout io.Writer) error {
	panic("unimplemented: cli.runInit (task-1.4 RED skeleton)")
}
