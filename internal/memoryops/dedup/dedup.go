// Package dedup implements v0.1 MemoryOps exact duplicate detection.
package dedup

import (
	"sort"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// Result is the output of exact duplicate deduplication.
type Result struct {
	Records    []*contextforgev1.ContextRecord
	Duplicates []Duplicate
}

// Duplicate describes a record merged into a first-seen representative.
type Duplicate struct {
	RepresentativeID string
	DuplicateID      string
	ContentHash      string
}

// Records merges exact duplicate ContextRecords by ContentHash.
func Records(records []*contextforgev1.ContextRecord) Result {
	result := Result{
		Records:    make([]*contextforgev1.ContextRecord, 0, len(records)),
		Duplicates: []Duplicate{},
	}
	representativeByHash := make(map[string]int, len(records))

	for _, record := range records {
		if record == nil {
			continue
		}

		hash := record.GetContentHash()
		if hash == "" {
			result.Records = append(result.Records, cloneRecord(record))
			continue
		}

		if representativeIndex, ok := representativeByHash[hash]; ok {
			representative := result.Records[representativeIndex]
			mergeInto(representative, record)
			result.addDuplicate(representative, record, hash)
			continue
		}

		representativeByHash[hash] = len(result.Records)
		result.Records = append(result.Records, cloneRecord(record))
	}

	return result
}

func (r *Result) addDuplicate(representative *contextforgev1.ContextRecord, duplicate *contextforgev1.ContextRecord, hash string) {
	r.Duplicates = append(r.Duplicates, Duplicate{
		RepresentativeID: representative.GetId(),
		DuplicateID:      duplicate.GetId(),
		ContentHash:      hash,
	})
}

func cloneRecord(record *contextforgev1.ContextRecord) *contextforgev1.ContextRecord {
	return &contextforgev1.ContextRecord{
		Id:              record.Id,
		SchemaVersion:   record.SchemaVersion,
		CollectionId:    record.CollectionId,
		SourceType:      record.SourceType,
		SourceProvider:  record.SourceProvider,
		SourceUri:       record.SourceUri,
		AgentScope:      append([]string(nil), record.AgentScope...),
		Title:           record.Title,
		Content:         record.Content,
		ContentHash:     record.ContentHash,
		RedactionStatus: record.RedactionStatus,
		Language:        record.Language,
		FilePath:        record.FilePath,
		LineStart:       record.LineStart,
		LineEnd:         record.LineEnd,
		Tags:            append([]string(nil), record.Tags...),
		Provenance:      cloneProvenance(record.Provenance),
		SecurityLabels:  append([]string(nil), record.SecurityLabels...),
		CreatedAt:       record.CreatedAt,
		UpdatedAt:       record.UpdatedAt,
		ExpiresAt:       record.ExpiresAt,
		Version:         record.Version,
		Metadata:        record.Metadata,
	}
}

func cloneProvenance(provenance []*contextforgev1.Provenance) []*contextforgev1.Provenance {
	cloned := make([]*contextforgev1.Provenance, 0, len(provenance))
	for _, entry := range provenance {
		if entry == nil {
			continue
		}
		cloned = append(cloned, cloneProvenanceEntry(entry))
	}
	return cloned
}

func mergeInto(representative *contextforgev1.ContextRecord, duplicate *contextforgev1.ContextRecord) {
	representative.Provenance = mergeProvenance(representative.Provenance, duplicate.Provenance)
	representative.AgentScope = mergeStrings(representative.AgentScope, duplicate.AgentScope)
	representative.Tags = mergeStrings(representative.Tags, duplicate.Tags)
	representative.SecurityLabels = mergeStrings(representative.SecurityLabels, duplicate.SecurityLabels)
}

func mergeProvenance(left []*contextforgev1.Provenance, right []*contextforgev1.Provenance) []*contextforgev1.Provenance {
	merged := cloneProvenance(left)
	seen := make(map[string]struct{}, len(merged)+len(right))
	for _, entry := range merged {
		seen[provenanceKey(entry)] = struct{}{}
	}
	for _, entry := range right {
		if entry == nil {
			continue
		}
		key := provenanceKey(entry)
		if _, ok := seen[key]; ok {
			continue
		}
		merged = append(merged, cloneProvenanceEntry(entry))
		seen[key] = struct{}{}
	}
	return merged
}

func cloneProvenanceEntry(entry *contextforgev1.Provenance) *contextforgev1.Provenance {
	return &contextforgev1.Provenance{
		Importer:         entry.Importer,
		OriginalPath:     entry.OriginalPath,
		ImportedAt:       entry.ImportedAt,
		SourceModifiedAt: entry.SourceModifiedAt,
	}
}

func provenanceKey(entry *contextforgev1.Provenance) string {
	modified := ""
	if entry.GetSourceModifiedAt() != nil {
		modified = entry.GetSourceModifiedAt().String()
	}
	return entry.GetImporter() + "\x00" + entry.GetOriginalPath() + "\x00" + modified
}

func mergeStrings(left []string, right []string) []string {
	seen := make(map[string]struct{}, len(left)+len(right))
	for _, value := range left {
		if value != "" {
			seen[value] = struct{}{}
		}
	}
	for _, value := range right {
		if value != "" {
			seen[value] = struct{}{}
		}
	}

	merged := make([]string, 0, len(seen))
	for value := range seen {
		merged = append(merged, value)
	}
	sort.Strings(merged)
	return merged
}
