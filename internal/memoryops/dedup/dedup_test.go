package dedup

import (
	"testing"
	"time"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
	"google.golang.org/protobuf/types/known/timestamppb"
)

// TEST-5.1.1 / SCEN-5.1.1 / AC1: exact duplicate records with the same content_hash are merged.
func TestExactDuplicateRecordsAreDeduped(t *testing.T) {
	hash := "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
	records := []*contextforgev1.ContextRecord{
		record("ctx-hermes", hash, "same fact", "hermes-memory", "/h/MEMORY.md"),
		record("ctx-openclaw", hash, "same fact", "openclaw-workspace", "/o/memory.md"),
	}

	result := Records(records)

	if len(result.Records) != 1 {
		t.Fatalf("expected 1 representative record, got %d", len(result.Records))
	}
	if result.Records[0].Id != "ctx-hermes" {
		t.Errorf("expected first-seen representative ctx-hermes, got %q", result.Records[0].Id)
	}
	if got := result.Records[0].RedactionStatus; got != "pending" {
		t.Errorf("expected RedactionStatus=pending (task-3.1 §10 Waiver BINDING), got %q", got)
	}
	if len(result.Duplicates) != 1 {
		t.Fatalf("expected 1 duplicate report, got %d", len(result.Duplicates))
	}
	dup := result.Duplicates[0]
	if dup.RepresentativeID != "ctx-hermes" || dup.DuplicateID != "ctx-openclaw" || dup.ContentHash != hash {
		t.Errorf("unexpected duplicate report: %#v", dup)
	}
}

// TEST-5.1.2 / SCEN-5.1.2 / AC2: merged representative keeps all distinct provenance entries.
func TestProvenanceChainIsMerged(t *testing.T) {
	hash := "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
	records := []*contextforgev1.ContextRecord{
		record("ctx-a", hash, "same fact", "hermes-memory", "/h/MEMORY.md"),
		record("ctx-b", hash, "same fact", "openclaw-workspace", "/o/memory.md"),
	}

	result := Records(records)

	if len(result.Records) != 1 {
		t.Fatalf("expected 1 representative record, got %d", len(result.Records))
	}
	got := result.Records[0].Provenance
	if len(got) != 2 {
		t.Fatalf("expected 2 provenance entries, got %d: %#v", len(got), got)
	}
	if !hasProvenance(got, "hermes-memory", "/h/MEMORY.md") {
		t.Error("missing hermes provenance")
	}
	if !hasProvenance(got, "openclaw-workspace", "/o/memory.md") {
		t.Error("missing openclaw provenance")
	}
	for _, p := range got {
		if p.SourceModifiedAt == nil {
			t.Errorf("source_modified_at must be preserved for %#v", p)
		}
	}
}

// TEST-5.1.3 / SCEN-5.1.3 / AC3: semantic similarity is out of scope; different hashes are not merged.
func TestSemanticSimilarRecordsAreNotDeduped(t *testing.T) {
	records := []*contextforgev1.ContextRecord{
		record("ctx-a", "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc", "Use QMD reranker", "hermes-memory", "/h/MEMORY.md"),
		record("ctx-b", "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd", "Prefer QMD reranking", "openclaw-workspace", "/o/memory.md"),
	}

	result := Records(records)

	if len(result.Records) != 2 {
		t.Fatalf("expected both semantically similar records to remain, got %d", len(result.Records))
	}
	if len(result.Duplicates) != 0 {
		t.Fatalf("expected no duplicates for different hashes, got %#v", result.Duplicates)
	}
}

// TEST-5.1.4 / SCEN-5.1.4 / AC4: dedup uses the provided chunker content_hash field, not recalculated content.
func TestContentHashFieldIsTheDedupAnchor(t *testing.T) {
	hash := "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
	records := []*contextforgev1.ContextRecord{
		record("ctx-a", hash, "first literal body", "local-fs", "/a.md"),
		record("ctx-b", hash, "different literal body but same upstream hash", "hermes-memory", "/b.md"),
	}

	result := Records(records)

	if len(result.Records) != 1 {
		t.Fatalf("expected dedup to use provided content_hash anchor, got %d records", len(result.Records))
	}
	if result.Records[0].Content != "first literal body" {
		t.Errorf("representative content should remain first-seen, got %q", result.Records[0].Content)
	}
}

func record(id string, hash string, content string, importer string, originalPath string) *contextforgev1.ContextRecord {
	return &contextforgev1.ContextRecord{
		Id:              id,
		SchemaVersion:   "0.1",
		CollectionId:    "project-x",
		SourceType:      "memory",
		SourceProvider:  importer,
		SourceUri:       "file://" + originalPath,
		Content:         content,
		ContentHash:     hash,
		RedactionStatus: "pending",
		AgentScope:      []string{importer},
		Tags:            []string{"memory", importer},
		SecurityLabels:  []string{"local_only"},
		Provenance: []*contextforgev1.Provenance{{
			Importer:         importer,
			OriginalPath:     originalPath,
			ImportedAt:       timestamppb.New(time.Unix(1, 0).UTC()),
			SourceModifiedAt: timestamppb.New(time.Unix(2, 0).UTC()),
		}},
	}
}

func hasProvenance(provenance []*contextforgev1.Provenance, importer string, originalPath string) bool {
	for _, p := range provenance {
		if p.Importer == importer && p.OriginalPath == originalPath {
			return true
		}
	}
	return false
}
