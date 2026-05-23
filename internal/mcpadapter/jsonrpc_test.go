package mcpadapter

import (
	"context"
	"encoding/json"
	"strings"
	"testing"
)

// TEST-7.1.4 / SCEN-7.1.4 / AC4:
// MCP 2025-06-18 stdio transport is newline-delimited JSON-RPC; initialize
// negotiates newer client protocol versions down to the locked server version.
func TestTask71_AC4_JSONRPCFramingAndVersionNegotiation(t *testing.T) {
	s := &Server{
		Searcher: &fakeSearcher{},
		DataDir:  t.TempDir(),
		Allowlist: []AllowlistEntry{
			{Name: "claude-desktop", Version: ">=0.7.0"},
		},
	}
	input := strings.Join([]string{
		`{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"claude-desktop","version":"0.8.0"}}}`,
		`{"jsonrpc":"2.0","method":"notifications/initialized"}`,
		`{"jsonrpc":"2.0","id":2,"method":"tools/list"}`,
		"",
	}, "\n")
	var stdout strings.Builder
	if err := s.Serve(context.Background(), strings.NewReader(input), &stdout); err != nil {
		t.Fatalf("Serve: %v", err)
	}
	lines := nonEmptyLines(stdout.String())
	if len(lines) != 2 {
		t.Fatalf("expected initialize + tools/list responses, got %d lines: %q", len(lines), stdout.String())
	}

	var initResp JSONRPCResponse
	if err := json.Unmarshal([]byte(lines[0]), &initResp); err != nil {
		t.Fatalf("decode initialize response: %v", err)
	}
	if initResp.Error != nil {
		t.Fatalf("initialize error: %+v", initResp.Error)
	}
	initResult := structuredMap(t, initResp.Result)
	if got := initResult["protocolVersion"]; got != SupportedProtocolVersion {
		t.Fatalf("protocolVersion=%v want %s", got, SupportedProtocolVersion)
	}
	caps, ok := initResult["capabilities"].(map[string]any)
	if !ok {
		t.Fatalf("capabilities missing/wrong type: %#v", initResult["capabilities"])
	}
	if _, ok := caps["tools"]; !ok {
		t.Fatalf("server capabilities missing tools: %#v", caps)
	}

	var listResp JSONRPCResponse
	if err := json.Unmarshal([]byte(lines[1]), &listResp); err != nil {
		t.Fatalf("decode tools/list response: %v", err)
	}
	listResult := structuredMap(t, listResp.Result)
	tools, ok := listResult["tools"].([]any)
	if !ok || len(tools) != 4 {
		t.Fatalf("tools/list=%#v want 4 tools", listResult["tools"])
	}
}

func nonEmptyLines(s string) []string {
	raw := strings.Split(s, "\n")
	out := make([]string, 0, len(raw))
	for _, line := range raw {
		if strings.TrimSpace(line) != "" {
			out = append(out, line)
		}
	}
	return out
}
