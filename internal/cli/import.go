// Package cli (sub-file import.go) — task-9.4 cli-import real implementation.
//
// Contract: task-9.4 §5.3. `contextforge import <name> <path> --collection ID
// [--data-dir D] [--output O] [--dry-run]` parses args, selects the named
// importer (hermes | openclaw | agent-rules), invokes its Import(path,
// collectionID) → []*ContextRecord, then serialises each record as a YAML-
// frontmatter Markdown file under <output_dir>/<id>.md. The output dir
// is consumable by `contextforge index --source <output_dir>` (D1 two-step
// flow per ADR-013).
package cli

import (
	"flag"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/config"
	"github.com/tajiaoyezi/contextforge/internal/importer"
	"github.com/tajiaoyezi/contextforge/internal/importer/agentrules"
	"github.com/tajiaoyezi/contextforge/internal/importer/hermes"
	"github.com/tajiaoyezi/contextforge/internal/importer/openclaw"
	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

const (
	importerHermes     = "hermes"
	importerOpenClaw   = "openclaw"
	importerAgentRules = "agent-rules"
)

var validImporters = []string{importerHermes, importerOpenClaw, importerAgentRules}

type importOpts struct {
	Name       string
	SourcePath string
	Collection string
	DataDir    string
	Output     string
	DryRun     bool
}

// runImport implements `contextforge import <name> <path> [flags]`. Returns
// exit code: 0 ok / 1 import error / 2 bad args.
func runImport(args []string, stdout, stderr io.Writer) int {
	if len(args) < 1 {
		printImportUsage(stderr)
		return 2
	}
	name := args[0]
	rest := args[1:]

	// Validate importer name early (before stat or flag parse) so unknown
	// names give a usage hint instead of a "source path does not exist"
	// distraction (AC4).
	imp, err := selectImporter(name)
	if err != nil {
		fmt.Fprintf(stderr, "contextforge import: %v\n", err)
		printImportUsage(stderr)
		return 2
	}

	opts, err := parseImportOpts(name, rest, stderr)
	if err != nil {
		return 2
	}
	if opts.SourcePath == "" {
		fmt.Fprintln(stderr, "contextforge import: <path> positional argument is required")
		printImportUsage(stderr)
		return 2
	}
	if _, err := os.Stat(opts.SourcePath); err != nil {
		fmt.Fprintf(stderr, "contextforge import: source path %q: %v\n", opts.SourcePath, err)
		return 1
	}

	records, err := importRecords(imp, opts.Name, opts.SourcePath, opts.Collection)
	if err != nil {
		fmt.Fprintf(stderr, "contextforge import: %v\n", err)
		return 1
	}

	outputDir := opts.Output
	if outputDir == "" {
		outputDir = filepath.Join(opts.DataDir, "imports", opts.Name)
	}

	count := len(records)
	if !opts.DryRun {
		written, werr := writeRecordsAsMarkdown(outputDir, records)
		if werr != nil {
			fmt.Fprintf(stderr, "contextforge import: write records: %v\n", werr)
			return 1
		}
		count = written
	}

	fmt.Fprintf(stdout, "contextforge import: imported %d records to %s\n", count, outputDir)
	fmt.Fprintf(stdout, "next: contextforge index --source %s --collection %s --data-dir %s\n",
		outputDir, opts.Collection, opts.DataDir)
	if opts.DryRun {
		fmt.Fprintln(stdout, "(--dry-run: no files were written)")
	}
	return 0
}

// parseImportOpts extracts the first non-flag positional arg as SourcePath
// then forwards the remaining args (now flag-only) to flag.Parse. stdlib
// flag.Parse stops at the first non-flag arg, so we do a manual split first
// to support both `import hermes <path> --flag=v` and `import hermes --flag=v
// <path>` orderings.
func parseImportOpts(name string, rest []string, stderr io.Writer) (*importOpts, error) {
	var sourcePath string
	flagArgs := make([]string, 0, len(rest))
	for _, a := range rest {
		if sourcePath == "" && !strings.HasPrefix(a, "-") {
			sourcePath = a
			continue
		}
		flagArgs = append(flagArgs, a)
	}

	fs := flag.NewFlagSet("import "+name, flag.ContinueOnError)
	fs.SetOutput(stderr)
	collection := fs.String("collection", "default", "collection ID (defaults to \"default\")")
	dataDir := fs.String("data-dir", "", "data root (default ~/.contextforge)")
	output := fs.String("output", "", "override output directory (default: <data-dir>/imports/<name>/)")
	dryRun := fs.Bool("dry-run", false, "skip writing files; print summary only")
	if err := fs.Parse(flagArgs); err != nil {
		return nil, err
	}
	if *dataDir == "" {
		root, err := config.DefaultRootDir()
		if err != nil {
			return nil, err
		}
		*dataDir = root
	}
	return &importOpts{
		Name:       name,
		SourcePath: sourcePath,
		Collection: *collection,
		DataDir:    *dataDir,
		Output:     *output,
		DryRun:     *dryRun,
	}, nil
}

// selectImporter returns the concrete Importer for the given name. Unknown
// names return nil + a usage-hint error. §2A verified each importer package's
// actual constructor (hermes.New / openclaw.NewImporter(agent) /
// agentrules.NewAgentRulesImporter).
func selectImporter(name string) (importer.Importer, error) {
	switch name {
	case importerHermes:
		return hermes.New(), nil
	case importerOpenClaw:
		return openclaw.NewImporter("openclaw"), nil
	case importerAgentRules:
		return agentrules.NewAgentRulesImporter(), nil
	default:
		return nil, fmt.Errorf("unknown importer: %s; want one of %v", name, validImporters)
	}
}

// importRecords invokes imp.Import(path, collection) — but for hermes the
// Import method only accepts a single MEMORY.md / USER.md file (task-3.2),
// while the v0.2 quickstart fixture is a directory of such files. We walk
// the directory and fan out per-file imports, merging records. openclaw +
// agent-rules accept paths as-is (openclaw walks the dir itself; agent-rules
// expects a single file).
func importRecords(imp importer.Importer, name, path, collection string) ([]*contextforgev1.ContextRecord, error) {
	if name != importerHermes {
		return imp.Import(path, collection)
	}
	info, err := os.Stat(path)
	if err != nil {
		return nil, err
	}
	if !info.IsDir() {
		return imp.Import(path, collection)
	}

	var files []string
	walkErr := filepath.WalkDir(path, func(p string, d os.DirEntry, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		if d.IsDir() {
			return nil
		}
		base := strings.ToUpper(filepath.Base(p))
		if base == "MEMORY.MD" || base == "USER.MD" {
			files = append(files, p)
		}
		return nil
	})
	if walkErr != nil {
		return nil, walkErr
	}
	if len(files) == 0 {
		return nil, fmt.Errorf("hermes: no MEMORY.md / USER.md found under %s", path)
	}
	sort.Strings(files)
	var out []*contextforgev1.ContextRecord
	for _, f := range files {
		recs, err := imp.Import(f, collection)
		if err != nil {
			return nil, fmt.Errorf("hermes import %s: %w", f, err)
		}
		out = append(out, recs...)
	}
	return out, nil
}

// recordToMarkdown serialises a ContextRecord as a YAML-frontmatter Markdown
// file body. The body is rec.Content; the frontmatter preserves importer /
// source_provider / source_type / agent_scope / language / file_path /
// line_start / line_end / content_hash / created_at metadata. Round-trippable
// for human inspection + consumable by IndexSession through the existing
// scanner→parser→chunker→indexer pipeline (D1 two-step flow per ADR-013).
//
// All field values are known to be yaml-safe ASCII (hex / enum strings /
// ISO 8601 / file paths). Future user-input fields must verify escape — see
// §10 trade-off.
func recordToMarkdown(rec *contextforgev1.ContextRecord) (string, error) {
	if rec == nil {
		return "", fmt.Errorf("nil record")
	}
	importerName := ""
	originalPath := ""
	if len(rec.GetProvenance()) > 0 {
		importerName = rec.GetProvenance()[0].GetImporter()
		originalPath = rec.GetProvenance()[0].GetOriginalPath()
	}
	createdAt := ""
	if t := rec.GetCreatedAt(); t != nil {
		createdAt = t.AsTime().UTC().Format(time.RFC3339)
	}

	var sb strings.Builder
	sb.WriteString("---\n")
	sb.WriteString(fmt.Sprintf("schema_version: %q\n", rec.GetSchemaVersion()))
	sb.WriteString(fmt.Sprintf("id: %s\n", rec.GetId()))
	sb.WriteString(fmt.Sprintf("collection_id: %s\n", rec.GetCollectionId()))
	sb.WriteString(fmt.Sprintf("source_type: %s\n", rec.GetSourceType()))
	sb.WriteString(fmt.Sprintf("source_provider: %s\n", rec.GetSourceProvider()))
	sb.WriteString(fmt.Sprintf("source_uri: %s\n", rec.GetSourceUri()))
	sb.WriteString("agent_scope: [")
	for i, s := range rec.GetAgentScope() {
		if i > 0 {
			sb.WriteString(", ")
		}
		sb.WriteString(s)
	}
	sb.WriteString("]\n")
	sb.WriteString(fmt.Sprintf("language: %s\n", rec.GetLanguage()))
	sb.WriteString(fmt.Sprintf("file_path: %s\n", rec.GetFilePath()))
	sb.WriteString(fmt.Sprintf("line_start: %d\n", rec.GetLineStart()))
	sb.WriteString(fmt.Sprintf("line_end: %d\n", rec.GetLineEnd()))
	sb.WriteString(fmt.Sprintf("content_hash: %s\n", rec.GetContentHash()))
	sb.WriteString(fmt.Sprintf("importer: %s\n", importerName))
	sb.WriteString(fmt.Sprintf("original_path: %s\n", originalPath))
	if createdAt != "" {
		sb.WriteString(fmt.Sprintf("created_at: %q\n", createdAt))
	}
	sb.WriteString("---\n\n")
	sb.WriteString(rec.GetContent())
	if !strings.HasSuffix(rec.GetContent(), "\n") {
		sb.WriteString("\n")
	}
	return sb.String(), nil
}

// writeRecordsAsMarkdown writes each record to <outputDir>/<record.Id>.md.
// Mkdir -p outputDir if missing. Existing files are overwritten (idempotent
// on same input). Returns count of files written + first error encountered.
func writeRecordsAsMarkdown(outputDir string, records []*contextforgev1.ContextRecord) (int, error) {
	if err := os.MkdirAll(outputDir, 0o755); err != nil {
		return 0, fmt.Errorf("mkdir %s: %w", outputDir, err)
	}
	written := 0
	for _, rec := range records {
		body, err := recordToMarkdown(rec)
		if err != nil {
			return written, fmt.Errorf("serialise %s: %w", rec.GetId(), err)
		}
		fname := filepath.Join(outputDir, rec.GetId()+".md")
		if err := os.WriteFile(fname, []byte(body), 0o644); err != nil {
			return written, fmt.Errorf("write %s: %w", fname, err)
		}
		written++
	}
	return written, nil
}

func printImportUsage(w io.Writer) {
	fmt.Fprintf(w, "usage: contextforge import <%s|%s|%s> <path> [--collection ID] [--data-dir DIR] [--output DIR] [--dry-run]\n",
		importerHermes, importerOpenClaw, importerAgentRules)
}
