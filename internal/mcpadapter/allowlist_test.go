package mcpadapter

import (
	"context"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TEST-7.1.3 / SCEN-7.1.3 / AC3:
// allowlist is explicit; empty allowlist rejects every client, and simplified
// version matching supports exact versions and >=X.Y.Z.
func TestTask71_AC3_AllowlistMatching(t *testing.T) {
	allowlist := []AllowlistEntry{
		{Name: "claude-desktop", Version: ">=0.7.0"},
		{Name: "cursor"},
		{Name: "zed", Version: "1.2.3"},
	}
	cases := []struct {
		name  string
		entry AllowlistEntry
		want  bool
	}{
		{"empty allowlist rejects", AllowlistEntry{Name: "claude-desktop", Version: "0.8.0"}, false},
		{"range accepts newer", AllowlistEntry{Name: "claude-desktop", Version: "0.8.0"}, true},
		{"range rejects older", AllowlistEntry{Name: "claude-desktop", Version: "0.6.9"}, false},
		{"blank rule accepts any version", AllowlistEntry{Name: "cursor", Version: "0.1.0"}, true},
		{"exact accepts equal", AllowlistEntry{Name: "zed", Version: "1.2.3"}, true},
		{"exact rejects different", AllowlistEntry{Name: "zed", Version: "1.2.4"}, false},
		{"unknown client rejects", AllowlistEntry{Name: "unknown", Version: "9.9.9"}, false},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			list := allowlist
			if tc.name == "empty allowlist rejects" {
				list = nil
			}
			if got := IsAllowlisted(tc.entry, list); got != tc.want {
				t.Fatalf("IsAllowlisted(%+v)=%v want %v", tc.entry, got, tc.want)
			}
		})
	}
}

func TestTask71_AC3_LoadAllowlistMissingMeansEmpty(t *testing.T) {
	entries, err := LoadAllowlist(filepath.Join(t.TempDir(), "mcp-allowlist.json"))
	if err != nil {
		t.Fatalf("LoadAllowlist missing: %v", err)
	}
	if len(entries) != 0 {
		t.Fatalf("missing allowlist should be empty (= reject all), got %+v", entries)
	}
}

// TEST-7.1.3 / SCEN-7.1.3 / AC3:
// initialize from a non-allowlisted client returns JSON-RPC -32000, closes
// stdio after the response, and writes audit-rest.log with mcp:initialize.
func TestTask71_AC3_InitializeRejectsAndAudits(t *testing.T) {
	dataDir := t.TempDir()
	s := &Server{
		Searcher:  &fakeSearcher{},
		DataDir:   dataDir,
		Allowlist: nil, // default empty allowlist = reject every client
	}
	input := strings.Join([]string{
		`{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"cursor","version":"0.1.0"}}}`,
		`{"jsonrpc":"2.0","id":2,"method":"tools/list"}`,
		"",
	}, "\n")
	var stdout strings.Builder
	if err := s.Serve(context.Background(), strings.NewReader(input), &stdout); err != nil {
		t.Fatalf("Serve unauthorized: %v", err)
	}
	lines := nonEmptyLines(stdout.String())
	if len(lines) != 1 {
		t.Fatalf("unauthorized initialize should close after one response, got %d lines: %q", len(lines), stdout.String())
	}
	var resp JSONRPCResponse
	if err := json.Unmarshal([]byte(lines[0]), &resp); err != nil {
		t.Fatalf("decode response: %v", err)
	}
	if resp.Error == nil || resp.Error.Code != -32000 {
		t.Fatalf("unauthorized initialize error=%+v want code -32000", resp.Error)
	}

	auditBytes, err := os.ReadFile(filepath.Join(dataDir, "audit-rest.log"))
	if err != nil {
		t.Fatalf("audit log missing: %v", err)
	}
	audit := string(auditBytes)
	if !strings.Contains(audit, `"endpoint":"mcp:initialize"`) || !strings.Contains(audit, `"status":403`) {
		t.Fatalf("audit missing endpoint/status: %s", audit)
	}
	if strings.Contains(audit, `"query"`) || strings.Contains(audit, `"arguments"`) {
		t.Fatalf("audit must not include request/query/tool args: %s", audit)
	}
}
