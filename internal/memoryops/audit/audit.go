// Package audit implements task-6.2 daemon-scoped REST API audit log.
//
// Each REST access (including 401 denials) writes one JSON-lines event to
// `<dataDir>/audit-rest.log`. The schema is intentionally minimal to honour
// AC5 redaction: endpoint / status / timestamp only. The Bearer token value
// and full request body MUST NEVER be written here (the middleware excludes
// them before calling `Write`).
//
// Relation to the existing Rust-side `core/src/memoryops/audit.rs`
// (task-5.3): that one is collection-scoped, SQLite-backed, and records
// memoryops operations (import / search / export / redact). This Go
// package is daemon-scoped (one log file per data root), file-based, and
// records REST control-plane access. The two are complementary, not
// duplicates — §10 of task-6.2 documents the split honestly.
package audit

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"
)

// FileName is the daemon-scoped REST audit log file name (under dataDir).
const FileName = "audit-rest.log"

// Event is one REST access record. `Reason` is optional and stays generic
// (never include the offending token / body content — keeps AC5 redaction
// invariant on the caller side too).
type Event struct {
	Endpoint  string    `json:"endpoint"`
	Status    int       `json:"status"`
	Timestamp time.Time `json:"timestamp"`
	Reason    string    `json:"reason,omitempty"`
}

// Write appends `ev` as one JSON line to `<dataDir>/audit-rest.log`. The
// file is created on first write (mode 0o644 — privacy comes from the
// 0o700 dataDir itself, set by task-1.2 config.Init). Empty `dataDir`
// returns an error so callers don't silently lose audit records.
func Write(dataDir string, ev Event) error {
	if dataDir == "" {
		return fmt.Errorf("audit: empty dataDir")
	}
	if ev.Timestamp.IsZero() {
		ev.Timestamp = time.Now().UTC()
	}
	p := filepath.Join(dataDir, FileName)
	f, err := os.OpenFile(p, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o644)
	if err != nil {
		return fmt.Errorf("audit: open %q: %w", p, err)
	}
	defer f.Close()
	line, err := json.Marshal(ev)
	if err != nil {
		return fmt.Errorf("audit: marshal: %w", err)
	}
	line = append(line, '\n')
	if _, err := f.Write(line); err != nil {
		return fmt.Errorf("audit: write %q: %w", p, err)
	}
	return nil
}
