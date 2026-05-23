package cli

import (
	"bytes"
	"context"
	"io"
	"path/filepath"
	"strings"
	"testing"
)

// TEST-7.1.5 / SCEN-7.1.5 / AC5:
// `contextforge mcp` dispatches to an injected backend with fake stdin/stdout,
// so the CLI path is end-to-end testable without starting a real daemon.
func TestTask71_AC5_CLIMCPSubcommandEndToEndWithFakeStdio(t *testing.T) {
	orig := mcpBackend
	defer func() { mcpBackend = orig }()

	dataDir := t.TempDir()
	allowlist := filepath.Join(dataDir, "mcp-allowlist.json")
	var backendInput string
	var backendOpts MCPOpts
	SetMCPBackend(func(_ context.Context, opts MCPOpts, stdin io.Reader, stdout, _ io.Writer) error {
		backendOpts = opts
		b, err := io.ReadAll(stdin)
		if err != nil {
			return err
		}
		backendInput = string(b)
		_, err = io.WriteString(stdout, `{"jsonrpc":"2.0","id":1,"result":{"ok":true}}`+"\n")
		return err
	})

	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader(`{"jsonrpc":"2.0","id":1,"method":"initialize"}` + "\n")
	code := ExecuteWithIO([]string{"mcp", "--data-dir", dataDir, "--allowlist", allowlist}, stdin, &stdout, &stderr)
	if code != 0 {
		t.Fatalf("ExecuteWithIO mcp exit=%d stderr=%q", code, stderr.String())
	}
	if backendOpts.DataDir != dataDir || backendOpts.Allowlist != allowlist {
		t.Fatalf("backend opts=%+v want dataDir=%s allowlist=%s", backendOpts, dataDir, allowlist)
	}
	if !strings.Contains(backendInput, `"initialize"`) {
		t.Fatalf("backend did not receive fake stdin: %q", backendInput)
	}
	if !strings.Contains(stdout.String(), `"ok":true`) {
		t.Fatalf("stdout missing fake JSON-RPC response: %q", stdout.String())
	}
}

func TestTask71_AC5_ParseMCPOptsDefaultsAllowlistUnderDataDir(t *testing.T) {
	dataDir := t.TempDir()
	var stderr bytes.Buffer
	opts, err := parseMCPOpts([]string{"--data-dir", dataDir}, &stderr)
	if err != nil {
		t.Fatalf("parseMCPOpts: %v stderr=%q", err, stderr.String())
	}
	want := filepath.Join(dataDir, "mcp-allowlist.json")
	if opts.Allowlist != want {
		t.Fatalf("allowlist=%q want %q", opts.Allowlist, want)
	}
}
