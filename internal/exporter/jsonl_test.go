package exporter

import (
	"bufio"
	"bytes"
	"encoding/json"
	"fmt"
	"testing"
	"time"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
	"google.golang.org/protobuf/types/known/structpb"
	"google.golang.org/protobuf/types/known/timestamppb"
)

// sampleRecords returns canonical ContextRecord fixtures with all 23 fields
// populated so fidelity tests can catch lossy format changes.
func sampleRecords(t *testing.T, n int) []*contextforgev1.ContextRecord {
	t.Helper()
	out := make([]*contextforgev1.ContextRecord, 0, n)
	scopes := []string{"memory", "user", "agents", "claude"}
	for i := 0; i < n; i++ {
		ts := timestamppb.New(time.Unix(1_700_000_000+int64(i), 0).UTC())
		meta, err := structpb.NewStruct(map[string]any{
			"fixture_index": float64(i),
			"fixture_kind":  "task-6.3",
		})
		if err != nil {
			t.Fatalf("metadata fixture: %v", err)
		}
		scope := scopes[i%len(scopes)]
		out = append(out, &contextforgev1.ContextRecord{
			Id:              fmt.Sprintf("ctx-%02d", i),
			SchemaVersion:   "0.1",
			CollectionId:    "default",
			SourceType:      "markdown",
			SourceProvider:  "fixture",
			SourceUri:       fmt.Sprintf("file:///fixture/source-%02d.md", i),
			AgentScope:      []string{scope},
			Title:           fmt.Sprintf("Fixture %02d", i),
			Content:         fmt.Sprintf("fixture content %02d for %s scope", i, scope),
			ContentHash:     fmt.Sprintf("sha256:%064x", i+1),
			RedactionStatus: "applied",
			Language:        "markdown",
			FilePath:        fmt.Sprintf("fixture/source-%02d.md", i),
			LineStart:       int64(i*10 + 1),
			LineEnd:         int64(i*10 + 5),
			Tags:            []string{"fixture", fmt.Sprintf("tag-%02d", i)},
			Provenance: []*contextforgev1.Provenance{
				{
					Importer:         "scanner",
					OriginalPath:     fmt.Sprintf("fixture/source-%02d.md", i),
					ImportedAt:       ts,
					SourceModifiedAt: ts,
				},
			},
			SecurityLabels: []string{"local-only"},
			CreatedAt:      ts,
			UpdatedAt:      ts,
			ExpiresAt:      timestamppb.New(ts.AsTime().Add(24 * time.Hour)),
			Version:        int64(i + 1),
			Metadata:       meta,
		})
	}
	return out
}

// TEST-6.3.1 / SCEN-6.3.1 / AC1
func TestTask63_AC1_JSONLFormatAndRecordCount(t *testing.T) {
	records := sampleRecords(t, 3)

	var buf bytes.Buffer
	if err := writeJSONL(records, &buf); err != nil {
		t.Fatalf("writeJSONL: %v", err)
	}

	scanner := bufio.NewScanner(bytes.NewReader(buf.Bytes()))
	decoded := make([]contextforgev1.ContextRecord, 0, len(records))
	for scanner.Scan() {
		line := scanner.Bytes()
		if len(bytes.TrimSpace(line)) == 0 {
			t.Fatalf("jsonl contains a blank line")
		}
		var rec contextforgev1.ContextRecord
		if err := json.Unmarshal(line, &rec); err != nil {
			t.Fatalf("jsonl line is not a ContextRecord JSON object: %v\nline=%s", err, line)
		}
		decoded = append(decoded, rec)
	}
	if err := scanner.Err(); err != nil {
		t.Fatalf("scan jsonl: %v", err)
	}
	if got, want := len(decoded), len(records); got != want {
		t.Fatalf("jsonl record count=%d want %d", got, want)
	}
	if decoded[0].GetId() != records[0].GetId() {
		t.Fatalf("first record id=%q want %q", decoded[0].GetId(), records[0].GetId())
	}
	if decoded[0].GetCollectionId() != "default" {
		t.Fatalf("collection_id=%q want default", decoded[0].GetCollectionId())
	}
}
