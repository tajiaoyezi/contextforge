package audit

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// phase-6 closeout PR (post-PR #44 review FIX-3): direct unit tests for
// `internal/memoryops/audit` package — previously the package had no
// direct unit tests; only rest_test.go TEST-6.2.5 黑盒间接 covered it.
// 这些 unit tests 让 audit.Write 行为有最小 baseline + reason omitempty 校验。

func TestAuditWrite_EmptyDataDirError(t *testing.T) {
	if err := Write("", Event{Endpoint: "/v1/search", Status: 200}); err == nil {
		t.Fatal("Write(empty dataDir) returned nil error; want error")
	}
}

func TestAuditWrite_ValidEvent_AppendsJSONLine(t *testing.T) {
	dir := t.TempDir()
	ev := Event{
		Endpoint:  "/v1/search",
		Status:    200,
		Timestamp: time.Date(2026, 5, 23, 12, 0, 0, 0, time.UTC),
	}
	if err := Write(dir, ev); err != nil {
		t.Fatalf("Write: %v", err)
	}
	body, err := os.ReadFile(filepath.Join(dir, FileName))
	if err != nil {
		t.Fatalf("read audit log: %v", err)
	}
	if !bytes.HasSuffix(body, []byte("\n")) {
		t.Fatalf("audit log line should end with \\n: %q", body)
	}
	var got Event
	if err := json.Unmarshal(bytes.TrimSuffix(body, []byte("\n")), &got); err != nil {
		t.Fatalf("unmarshal audit line: %v (line=%q)", err, body)
	}
	if got.Endpoint != ev.Endpoint || got.Status != ev.Status {
		t.Fatalf("audit event mismatch: got=%+v want=%+v", got, ev)
	}
	if !got.Timestamp.Equal(ev.Timestamp) {
		t.Fatalf("audit timestamp mismatch: got=%v want=%v", got.Timestamp, ev.Timestamp)
	}
}

func TestAuditWrite_ReasonOmittedWhenEmpty(t *testing.T) {
	dir := t.TempDir()
	if err := Write(dir, Event{Endpoint: "/v1/search", Status: 200}); err != nil {
		t.Fatalf("Write: %v", err)
	}
	body, err := os.ReadFile(filepath.Join(dir, FileName))
	if err != nil {
		t.Fatalf("read: %v", err)
	}
	if strings.Contains(string(body), `"reason"`) {
		t.Fatalf("empty Reason should be omitted (omitempty); got %q", body)
	}
}

func TestAuditWrite_ReasonIncludedWhenSet(t *testing.T) {
	dir := t.TempDir()
	if err := Write(dir, Event{Endpoint: "/v1/search", Status: 401, Reason: "missing or invalid token"}); err != nil {
		t.Fatalf("Write: %v", err)
	}
	body, err := os.ReadFile(filepath.Join(dir, FileName))
	if err != nil {
		t.Fatalf("read: %v", err)
	}
	if !strings.Contains(string(body), `"reason":"missing or invalid token"`) {
		t.Fatalf("Reason should be in JSON when set; got %q", body)
	}
}

func TestAuditWrite_AppendsMultipleEvents(t *testing.T) {
	dir := t.TempDir()
	events := []Event{
		{Endpoint: "/v1/search", Status: 200},
		{Endpoint: "/v1/chunks/abc", Status: 404},
		{Endpoint: "/v1/import", Status: 501},
	}
	for _, ev := range events {
		if err := Write(dir, ev); err != nil {
			t.Fatalf("Write %+v: %v", ev, err)
		}
	}
	body, err := os.ReadFile(filepath.Join(dir, FileName))
	if err != nil {
		t.Fatalf("read: %v", err)
	}
	lines := bytes.Split(bytes.TrimSuffix(body, []byte("\n")), []byte("\n"))
	if len(lines) != len(events) {
		t.Fatalf("audit log lines = %d, want %d (body=%q)", len(lines), len(events), body)
	}
}

func TestAuditWrite_AutoTimestampWhenZero(t *testing.T) {
	dir := t.TempDir()
	before := time.Now().UTC().Truncate(time.Second)
	if err := Write(dir, Event{Endpoint: "/v1/search", Status: 200}); err != nil {
		t.Fatalf("Write: %v", err)
	}
	after := time.Now().UTC().Add(time.Second)
	body, _ := os.ReadFile(filepath.Join(dir, FileName))
	var got Event
	_ = json.Unmarshal(bytes.TrimSuffix(body, []byte("\n")), &got)
	if got.Timestamp.Before(before) || got.Timestamp.After(after) {
		t.Fatalf("auto Timestamp %v out of [%v, %v]", got.Timestamp, before, after)
	}
}

// AC5 隐含约束: audit.Write 不应记 token / body 全文 — middleware 端责任，
// 但 Write 接口本身不消费 ev.Endpoint 之外的字段；只要 caller 不传 token /
// body 即安全。本 test 黑盒守护：传一个 Event with reason="some token=abc..."
// 时只会 JSON marshal Reason 字符串 — caller 不传 token 就不会被记录。
func TestAuditWrite_DoesNotInferOrLogToken(t *testing.T) {
	dir := t.TempDir()
	if err := Write(dir, Event{Endpoint: "/v1/search", Status: 401, Reason: "missing or invalid token"}); err != nil {
		t.Fatalf("Write: %v", err)
	}
	body, _ := os.ReadFile(filepath.Join(dir, FileName))
	// Specifically: no fields like "token", "authorization", "bearer", "body"
	// — those are caller-omitted by spec
	forbidden := []string{"token=", "Bearer ", "authorization", `"body"`, `"request"`, `"payload"`}
	for _, fb := range forbidden {
		if bytes.Contains(body, []byte(fb)) {
			t.Fatalf("audit log should not contain %q; got %q", fb, body)
		}
	}
}
