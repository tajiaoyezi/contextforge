// Package cli (sub-file search.go) — task-6.1 cli-search implementation.
//
// Contract: task-6.1 §5.3. `contextforge search "<query>" [flags]` parses
// flags, calls the injected gRPC `Search` backend (per-invocation spawn,
// §2A 决策 B), and renders the SearchResponse to stdout (text by default;
// --json for structured output).
//
// `internal/cli` deliberately does NOT import `internal/daemon` — that
// would form an import cycle with daemon_test.go (which import cli for the
// task-1.4 §6 end-to-end smoke). The real daemon-spawning backend lives
// in `cmd/contextforge/main.go` and is wired in via `SetSearchBackend`.
package cli

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"strings"
	"time"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// defaultTopK matches the Rust retriever default (`SearchOptions.top_k=0 ⇒ 10`)
// so a missing / non-positive `--top-k` returns 10 results.
const defaultTopK int32 = 10

// SearchBackend is the injectable gRPC `Search` callable that
// `cmd/contextforge/main.go` wires to a real `daemon.Search` call and
// tests substitute with a fake `SearchResponse` to skip cargo build +
// loopback bind.
type SearchBackend func(
	ctx context.Context,
	req *contextforgev1.SearchRequest,
) (*contextforgev1.SearchResponse, error)

// fetchSearchResults is the active backend; nil means runSearch will fail
// with a clear `backend not wired` message rather than panicking. Tests
// in `search_test.go` overwrite this variable directly.
var fetchSearchResults SearchBackend

// SetSearchBackend wires the production gRPC `Search` backend. Called
// once from `cmd/contextforge/main.go` before `Execute`. Passing a nil
// `b` is treated as a programmer error and panics at startup.
func SetSearchBackend(b SearchBackend) {
	if b == nil {
		panic("cli.SetSearchBackend: nil backend")
	}
	fetchSearchResults = b
}

// searchOpts holds the parsed flag state (§5.3 internal-only).
type searchOpts struct {
	Query       string
	Collections []string
	AgentScope  []string
	TopK        int32
	SourceType  []string
	Language    []string
	Explain     bool
	JSON        bool
}

// runSearch is the `contextforge search` subcommand entry point dispatched
// from Execute. Returns the process exit code: 0 = ok, 2 = usage error,
// 1 = runtime error.
func runSearch(args []string, stdout, stderr io.Writer) int {
	opts, err := parseSearchOpts(args, stderr)
	if err != nil {
		return 2
	}
	req := optsToProtoRequest(opts)

	if fetchSearchResults == nil {
		fmt.Fprintln(stderr,
			"contextforge search: search backend not wired "+
				"(cmd/contextforge/main.go must call cli.SetSearchBackend)")
		return 1
	}

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	resp, err := fetchSearchResults(ctx, req)
	if err != nil {
		fmt.Fprintf(stderr, "contextforge search: %v\n", err)
		return 1
	}

	var renderErr error
	if opts.JSON {
		renderErr = renderJSON(resp, stdout)
	} else {
		renderErr = renderText(resp, stdout)
	}
	if renderErr != nil {
		fmt.Fprintf(stderr, "contextforge search: render: %v\n", renderErr)
		return 1
	}
	return 0
}

// parseSearchOpts parses `args` (a `flag.FlagSet` plus a positional `query`).
// Comma-separated flag values expand to []string. Missing query writes a
// usage line to `stderr` and returns an error.
func parseSearchOpts(args []string, stderr io.Writer) (*searchOpts, error) {
	fs := flag.NewFlagSet("search", flag.ContinueOnError)
	fs.SetOutput(stderr)

	var (
		collections = fs.String("collections", "", "comma-separated collection IDs (v0.1: only the first is used)")
		agentScope  = fs.String("agent-scope", "", "comma-separated agent-scope filter")
		topK        = fs.Int("top-k", int(defaultTopK), "max results to return (≤0 falls back to default)")
		sourceType  = fs.String("source-type", "", "comma-separated source-type filter")
		language    = fs.String("language", "", "comma-separated language filter")
		explain     = fs.Bool("explain", false, "include reason + matched_terms in each result")
		jsonOut     = fs.Bool("json", false, "emit structured JSON (default: human-readable text)")
	)
	if err := fs.Parse(args); err != nil {
		return nil, err
	}
	positional := fs.Args()
	if len(positional) == 0 {
		fmt.Fprintln(stderr,
			`contextforge search: usage: contextforge search "<query>" [--collections=...] [--agent-scope=...] [--top-k=N] [--source-type=...] [--language=...] [--explain] [--json]`)
		return nil, fmt.Errorf("missing positional <query>")
	}

	return &searchOpts{
		Query:       positional[0],
		Collections: splitCSV(*collections),
		AgentScope:  splitCSV(*agentScope),
		TopK:        int32(*topK),
		SourceType:  splitCSV(*sourceType),
		Language:    splitCSV(*language),
		Explain:     *explain,
		JSON:        *jsonOut,
	}, nil
}

// splitCSV trims and splits a comma-separated flag value. Empty input
// returns nil so the proto field stays unset (proto3 repeated default).
func splitCSV(s string) []string {
	s = strings.TrimSpace(s)
	if s == "" {
		return nil
	}
	parts := strings.Split(s, ",")
	out := make([]string, 0, len(parts))
	for _, p := range parts {
		if t := strings.TrimSpace(p); t != "" {
			out = append(out, t)
		}
	}
	return out
}

// optsToProtoRequest maps `searchOpts` to a `*contextforgev1.SearchRequest`.
// `top_k ≤ 0` falls back to defaultTopK (matches Rust retriever behaviour).
func optsToProtoRequest(o *searchOpts) *contextforgev1.SearchRequest {
	topK := o.TopK
	if topK <= 0 {
		topK = defaultTopK
	}
	return &contextforgev1.SearchRequest{
		Query:       o.Query,
		Collections: o.Collections,
		AgentScope:  o.AgentScope,
		TopK:        topK,
		Filters: &contextforgev1.SearchFilters{
			SourceType: o.SourceType,
			Language:   o.Language,
		},
		Explain: o.Explain,
	}
}

// renderText writes a human-readable block per result to `w`. Each block:
//
//	<chunk_id>  <file_path>:<line_start>-<line_end>  score=<float>  redaction_status=<status>
//	  reason=<reason or empty>
//
// AC3: human-readable mode. AC4: redaction_status field value passes
// through verbatim from the upstream (no second secret scan in CLI).
func renderText(resp *contextforgev1.SearchResponse, w io.Writer) error {
	results := resp.GetResults()
	if len(results) == 0 {
		_, err := fmt.Fprintln(w, "(no results)")
		return err
	}
	for _, r := range results {
		if _, err := fmt.Fprintf(w,
			"%s  %s:%d-%d  score=%.4f  redaction_status=%s\n",
			r.GetChunkId(),
			r.GetFilePath(),
			r.GetLineStart(),
			r.GetLineEnd(),
			r.GetScore(),
			r.GetRedactionStatus(),
		); err != nil {
			return err
		}
		if _, err := fmt.Fprintf(w, "  reason=%s\n\n", r.GetReason()); err != nil {
			return err
		}
	}
	return nil
}

// renderJSON writes the SearchResponse as JSON to `w` (AC3 --json mode).
// stdlib encoding/json is sufficient because RetrievalResult fields are
// plain scalar/repeated types tagged with proto3 json names (`chunk_id`,
// `redaction_status`, etc.); §5.2 R7 strict-channel — no protojson dep.
//
// AC4: `redaction_status` is rendered as-is from the proto field; CLI does
// not scan the content again. AC5 / ADR-003: the marshaled shape IS the
// shared schema task-6.3 exporter will consume.
func renderJSON(resp *contextforgev1.SearchResponse, w io.Writer) error {
	return json.NewEncoder(w).Encode(resp)
}
