// Package exporter implements contextforge export formats, secret sanity
// scanning, and migration fidelity checks.
//
// Contract: docs/specs/tasks/task-6.3-exporter.md.
package exporter

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"os"
	"path/filepath"

	"github.com/tajiaoyezi/contextforge/internal/config"
	"github.com/tajiaoyezi/contextforge/internal/memoryops/lifecycle"
	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// Format is the supported export format enum.
type Format string

const (
	FormatJSONL          Format = "jsonl"
	FormatMarkdownBundle Format = "markdown-bundle"
	FormatAgentDraft     Format = "agent-draft"
)

// Options configures Export.
type Options struct {
	Format       Format
	Collection   string
	DataDir      string
	Output       string
	IncludeStale bool
}

// Result is returned from Export.
type Result struct {
	RecordsExported int
	OutputPath      string
	FidelityScore   float64
	SecretHits      []SecretHit
}

// ErrSecretHits indicates that the final serialized export bytes matched the
// task-6.3 sanity secret scanner and were not written.
var ErrSecretHits = errors.New("exporter: secret hits detected")

// Export loads records, renders the requested format, runs a final sanity
// secret scan, and writes the artifact to opts.Output.
func Export(ctx context.Context, opts Options) (*Result, error) {
	opts, err := normalizeOptions(opts)
	if err != nil {
		return nil, err
	}

	records, err := loadRecords(ctx, opts.DataDir, opts.Collection)
	if err != nil {
		return nil, err
	}
	if !opts.IncludeStale {
		marked := lifecycle.Mark(records, lifecycle.SystemOracle{})
		records = lifecycle.FilterStale(marked.Records, marked.StaleMarks)
	}

	rendered, draftFiles, err := render(records, opts.Format)
	if err != nil {
		return nil, err
	}

	result := &Result{
		RecordsExported: len(records),
		OutputPath:      opts.Output,
	}

	if hits := ScanForSecrets(rendered); len(hits) > 0 {
		result.SecretHits = hits
		return result, ErrSecretHits
	}

	switch opts.Format {
	case FormatJSONL, FormatMarkdownBundle:
		if err := writeFile0600(opts.Output, rendered); err != nil {
			return result, err
		}
	case FormatAgentDraft:
		if err := writeAgentDraftFiles(opts.Output, draftFiles); err != nil {
			return result, err
		}
	default:
		return result, fmt.Errorf("unsupported export format %q", opts.Format)
	}

	score, err := CalcFidelity(records, rendered, opts.Format)
	if err != nil {
		return result, err
	}
	result.FidelityScore = score
	return result, nil
}

func normalizeOptions(opts Options) (Options, error) {
	if opts.Format == "" {
		return opts, fmt.Errorf("format is required")
	}
	if !validFormat(opts.Format) {
		return opts, fmt.Errorf("unsupported format %q", opts.Format)
	}
	if opts.Collection == "" {
		opts.Collection = "default"
	}
	if opts.DataDir == "" {
		root, err := config.DefaultRootDir()
		if err != nil {
			return opts, err
		}
		opts.DataDir = root
	}
	if opts.Output == "" {
		return opts, fmt.Errorf("output is required")
	}
	return opts, nil
}

func validFormat(f Format) bool {
	switch f {
	case FormatJSONL, FormatMarkdownBundle, FormatAgentDraft:
		return true
	default:
		return false
	}
}

func render(records []*contextforgev1.ContextRecord, format Format) ([]byte, map[string][]byte, error) {
	var buf bytes.Buffer
	switch format {
	case FormatJSONL:
		if err := writeJSONL(records, &buf); err != nil {
			return nil, nil, err
		}
		return buf.Bytes(), nil, nil
	case FormatMarkdownBundle:
		if err := writeMarkdownBundle(records, &buf); err != nil {
			return nil, nil, err
		}
		return buf.Bytes(), nil, nil
	case FormatAgentDraft:
		files, err := renderAgentDraft(records)
		if err != nil {
			return nil, nil, err
		}
		return concatDraftFiles(files), files, nil
	default:
		return nil, nil, fmt.Errorf("unsupported export format %q", format)
	}
}

func writeFile0600(path string, body []byte) error {
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0o700); err != nil {
		return fmt.Errorf("create output dir %s: %w", dir, err)
	}
	return os.WriteFile(path, body, 0o600)
}
