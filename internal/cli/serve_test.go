// task-6.2: contextforge serve RED 测试集 (TEST-6.2.3 / 6.2.4).
//
// TEST-6.2.3 (AC3) — parseServeOpts 拒 0.0.0.0 / 校验 ensureLoopback.
// TEST-6.2.4 (AC4) — loadOrGenerateToken 软随机生成 + 0600 + 重启复用.

package cli

import (
	"bytes"
	"encoding/hex"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

// TEST-6.2.3 / SCEN-6.2.3 / AC3 — parseServeOpts 拒非 loopback 监听地址 (0.0.0.0 / :: / 非 IP host).
func TestTask62_AC3_ServeRejectsWildcardAddr(t *testing.T) {
	cases := []struct {
		name    string
		addr    string
		wantErr bool
	}{
		{"WildcardIPv4", "0.0.0.0:8080", true},
		{"WildcardIPv6", "[::]:8080", true},
		{"LoopbackIPv4", "127.0.0.1:0", false},
		{"LoopbackIPv6", "[::1]:0", false},
	}
	for _, tc := range cases {
		tc := tc
		t.Run(tc.name, func(t *testing.T) {
			var stderr bytes.Buffer
			_, err := parseServeOpts([]string{"--addr=" + tc.addr}, &stderr)
			if tc.wantErr {
				if err == nil {
					t.Fatalf("AC3 %s: 期待 parseServeOpts 拒 %q, 实际通过 stderr=%q", tc.name, tc.addr, stderr.String())
				}
				if !strings.Contains(stderr.String(), "loopback") && !strings.Contains(stderr.String(), "0.0.0.0") && !strings.Contains(stderr.String(), "wildcard") {
					t.Logf("AC3 %s: stderr=%q (hint should mention loopback/wildcard)", tc.name, stderr.String())
				}
			} else {
				if err != nil {
					t.Fatalf("AC3 %s: 期待 loopback %q 通过, 实际 err=%v stderr=%q", tc.name, tc.addr, err, stderr.String())
				}
			}
		})
	}
}

// TEST-6.2.3b — 默认无 flag 应自动选 loopback (无 wildcard 风险).
func TestTask62_AC3_ServeDefaultsToLoopback(t *testing.T) {
	var stderr bytes.Buffer
	opts, err := parseServeOpts([]string{}, &stderr)
	if err != nil {
		t.Fatalf("AC3 default: parseServeOpts: %v stderr=%q", err, stderr.String())
	}
	if opts == nil {
		t.Fatalf("AC3 default: opts nil")
	}
	// 默认 addr 应为空（让 runServe 走 freeLoopbackAddr 选端口）或为 127.0.0.1
	if opts.Addr != "" && !strings.HasPrefix(opts.Addr, "127.0.0.1") && !strings.HasPrefix(opts.Addr, "[::1]") {
		t.Fatalf("AC3 default: opts.Addr=%q 应为空 (走 freeLoopbackAddr) 或显式 loopback", opts.Addr)
	}
}

// TEST-6.2.4 / SCEN-6.2.4 / AC4 — token file 0600 + 软随机生成 + 重启复用.
func TestTask62_AC4_TokenFileMode0600(t *testing.T) {
	dataDir := t.TempDir()

	t.Run("GenerateOnFirstCall", func(t *testing.T) {
		token, tokenPath, err := loadOrGenerateToken(dataDir)
		if err != nil {
			t.Fatalf("AC4 first call: %v", err)
		}
		// 32-byte hex = 64 字符
		if len(token) != 64 {
			t.Fatalf("AC4: token len=%d want 64 (32 bytes hex)", len(token))
		}
		if _, err := hex.DecodeString(token); err != nil {
			t.Fatalf("AC4: token 非合法 hex: %v", err)
		}
		// tokenPath 应在 dataDir 下
		if filepath.Dir(tokenPath) != dataDir {
			t.Fatalf("AC4: tokenPath=%s 应在 dataDir=%s 下", tokenPath, dataDir)
		}
		info, err := os.Stat(tokenPath)
		if err != nil {
			t.Fatalf("AC4: stat token file: %v", err)
		}
		// POSIX: 0600 严格校验；Windows ACL 留 Phase 8（同 task-1.4 决策）
		if runtime.GOOS != "windows" {
			if mode := info.Mode().Perm(); mode != 0o600 {
				t.Fatalf("AC4: token file mode=%o want 0600", mode)
			}
		} else {
			t.Logf("AC4: Windows ACL bit check 跳过 (Go stdlib 报 0666/0777; 留 Phase 8 / v0.3)")
		}
	})

	t.Run("ReuseOnSecondCall", func(t *testing.T) {
		token1, path1, err := loadOrGenerateToken(dataDir)
		if err != nil {
			t.Fatalf("AC4 reuse first: %v", err)
		}
		token2, path2, err := loadOrGenerateToken(dataDir)
		if err != nil {
			t.Fatalf("AC4 reuse second: %v", err)
		}
		if token1 != token2 {
			t.Fatalf("AC4: 重启应复用同 token，第一次=%q 第二次=%q", token1, token2)
		}
		if path1 != path2 {
			t.Fatalf("AC4: tokenPath 应一致 %q != %q", path1, path2)
		}
	})

	t.Run("EmptyDataDirError", func(t *testing.T) {
		_, _, err := loadOrGenerateToken("")
		if err == nil {
			t.Fatalf("AC4: 空 dataDir 应返错")
		}
	})
}

// TEST-6.2.4b — runServe 在 backend 未注入时返清晰 error，不 panic.
func TestTask62_AC4_RunServeWithoutBackendReturnsError(t *testing.T) {
	// 保险：保存并清除 backend hook
	orig := serveBackend
	serveBackend = nil
	defer func() { serveBackend = orig }()

	var stdout, stderr bytes.Buffer
	code := runServe([]string{}, &stdout, &stderr)
	if code == 0 {
		t.Fatalf("AC4 sanity: backend 未 wire 不应返 exit 0 (stderr=%q)", stderr.String())
	}
	if !strings.Contains(stderr.String(), "backend") {
		t.Logf("AC4 sanity hint: stderr=%q should mention 'backend not wired'", stderr.String())
	}
}

// (Test-only sanity: avoid the unused-import warning for io if no other test uses it.)
var _ = io.Discard
