package exporter

import (
	"context"
	"errors"
	"os"
	"path/filepath"
	"testing"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// phase-6 closeout PR (post-PR #43 review FIX-1): end-to-end secret-scan
// rejection test through Export() pipeline — previously TEST-6.3.3 only
// tested ScanForSecrets() directly; Export() rejection path lacked coverage.
//
// Scenario: SearchBackend returns RetrievalResults containing AWS-key-shape
// file_path → loadRecords preserves it → render JSONL → ScanForSecrets
// hits aws_access_key → Export returns ErrSecretHits + file NOT written.

func TestTask63_AC3_ExportEndToEnd_SecretInRenderedOutput_RejectsAndNotWriteFile(t *testing.T) {
	// Fake SearchBackend returning record with secret pattern in file_path
	fake := func(_ context.Context, _ string, _ *contextforgev1.SearchRequest) (*contextforgev1.SearchResponse, error) {
		return &contextforgev1.SearchResponse{
			Results: []*contextforgev1.RetrievalResult{
				{
					ChunkId:  "test-chunk-1",
					FilePath: "/tmp/AKIA1234567890ABCDEF-leaked-config.txt", // AWS key in path
				},
			},
		}, nil
	}
	restore := SetSearchBackend(fake)
	defer restore()

	tmpDir := t.TempDir()
	outFile := filepath.Join(tmpDir, "out.jsonl")

	result, err := Export(context.Background(), Options{
		Format:     FormatJSONL,
		Collection: "default",
		DataDir:    tmpDir,
		Output:     outFile,
	})

	// Behavior contract:
	// 1. ErrSecretHits returned (errors.Is matches)
	if !errors.Is(err, ErrSecretHits) {
		t.Fatalf("Export err = %v, want ErrSecretHits", err)
	}
	// 2. Result.SecretHits non-empty (carrier of hit details)
	if result == nil || len(result.SecretHits) == 0 {
		t.Fatalf("Result.SecretHits = %v, want non-empty", result)
	}
	// 3. aws_access_key pattern hit specifically
	found := false
	for _, h := range result.SecretHits {
		if h.PatternName == "aws_access_key" {
			found = true
			break
		}
	}
	if !found {
		t.Fatalf("SecretHits should contain aws_access_key; got %+v", result.SecretHits)
	}
	// 4. Output file NOT written (secret-tainted export must NOT touch disk)
	if _, err := os.Stat(outFile); !os.IsNotExist(err) {
		t.Fatalf("output file should not exist after secret rejection; stat err=%v", err)
	}
}

func TestTask63_AC3_ExportEndToEnd_CleanRecords_WritesFile(t *testing.T) {
	// Sanity: clean records (no secret pattern anywhere) write file successfully
	fake := func(_ context.Context, _ string, _ *contextforgev1.SearchRequest) (*contextforgev1.SearchResponse, error) {
		return &contextforgev1.SearchResponse{
			Results: []*contextforgev1.RetrievalResult{
				{
					ChunkId:  "clean-chunk-1",
					FilePath: "/projects/myapp/main.go",
				},
				{
					ChunkId:  "clean-chunk-2",
					FilePath: "/projects/myapp/README.md",
				},
			},
		}, nil
	}
	restore := SetSearchBackend(fake)
	defer restore()

	tmpDir := t.TempDir()
	outFile := filepath.Join(tmpDir, "out.jsonl")

	result, err := Export(context.Background(), Options{
		Format:     FormatJSONL,
		Collection: "default",
		DataDir:    tmpDir,
		Output:     outFile,
	})

	if err != nil {
		t.Fatalf("Export err = %v, want nil", err)
	}
	if len(result.SecretHits) != 0 {
		t.Fatalf("Result.SecretHits should be empty for clean records; got %+v", result.SecretHits)
	}
	if _, err := os.Stat(outFile); err != nil {
		t.Fatalf("output file should exist after clean export; stat err=%v", err)
	}
	if result.RecordsExported != 2 {
		t.Fatalf("RecordsExported = %d, want 2", result.RecordsExported)
	}
}
