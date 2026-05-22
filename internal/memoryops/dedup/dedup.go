// Package dedup implements v0.1 MemoryOps exact duplicate detection.
package dedup

import contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"

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
	return Result{}
}
