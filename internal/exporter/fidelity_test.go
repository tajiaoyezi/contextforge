package exporter

import (
	"bytes"
	"os"
	"path/filepath"
	"sort"
	"testing"
)

// TEST-6.3.4 / SCEN-6.3.4 / AC4
func TestTask63_AC4_CalcFidelityAcrossFormats(t *testing.T) {
	records := sampleRecords(t, 10)

	var jsonl bytes.Buffer
	if err := writeJSONL(records, &jsonl); err != nil {
		t.Fatalf("writeJSONL: %v", err)
	}
	jsonlScore, err := CalcFidelity(records, jsonl.Bytes(), FormatJSONL)
	if err != nil {
		t.Fatalf("CalcFidelity jsonl: %v", err)
	}
	if jsonlScore < 0.80 {
		t.Fatalf("jsonl fidelity=%.3f want >=0.80", jsonlScore)
	}

	var bundle bytes.Buffer
	if err := writeMarkdownBundle(records, &bundle); err != nil {
		t.Fatalf("writeMarkdownBundle: %v", err)
	}
	bundleScore, err := CalcFidelity(records, bundle.Bytes(), FormatMarkdownBundle)
	if err != nil {
		t.Fatalf("CalcFidelity markdown-bundle: %v", err)
	}
	if bundleScore < 0.80 {
		t.Fatalf("markdown-bundle fidelity=%.3f want >=0.80", bundleScore)
	}

	outDir := filepath.Join(t.TempDir(), "draft")
	if err := writeAgentDraft(records, outDir); err != nil {
		t.Fatalf("writeAgentDraft: %v", err)
	}
	draftScore, err := CalcFidelity(records, readDraftBytes(t, outDir), FormatAgentDraft)
	if err != nil {
		t.Fatalf("CalcFidelity agent-draft: %v", err)
	}
	if draftScore < 0.60 {
		t.Fatalf("agent-draft fidelity=%.3f want >=0.60", draftScore)
	}
}

func readDraftBytes(t *testing.T, dir string) []byte {
	t.Helper()
	names := []string{"AGENTS.md", "CLAUDE.md", "MEMORY.md", "USER.md"}
	sort.Strings(names)
	var out bytes.Buffer
	for _, name := range names {
		body, err := os.ReadFile(filepath.Join(dir, name))
		if err != nil {
			t.Fatalf("read draft %s: %v", name, err)
		}
		out.WriteString("\n--- contextforge-draft-file: ")
		out.WriteString(name)
		out.WriteString(" ---\n")
		out.Write(body)
		out.WriteByte('\n')
	}
	return out.Bytes()
}
