// Package contract exposes the frozen ContextForge proto / canonical-record
// contract (task-1.1). It is the in-repo conformance surface that ties the
// frozen proto SSOT (proto/contextforge/v1/*.proto) to the Go data-plane and
// asserts the grpc-go bindings were code-generated without FFI.
package contract

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"

	"google.golang.org/grpc"
	"google.golang.org/protobuf/proto"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// schemaVersion is the FROZEN canonical-record schema version. Per the
// contract freeze rule (PRD §Technical Risks R1 / proto CONTRACT FREEZE RULE)
// v0.1 may only add fields with new tags — never delete or renumber tags.
const schemaVersion = "0.1"

// SchemaVersion returns the frozen canonical-record schema version ("0.1").
func SchemaVersion() string { return schemaVersion }

// protoDir walks up from the working directory to locate the frozen proto
// SSOT directory proto/contextforge/v1.
func protoDir() (string, error) {
	d, err := os.Getwd()
	if err != nil {
		return "", err
	}
	for {
		p := filepath.Join(d, "proto", "contextforge", "v1")
		if fi, statErr := os.Stat(p); statErr == nil && fi.IsDir() {
			return p, nil
		}
		parent := filepath.Dir(d)
		if parent == d {
			return "", fmt.Errorf("proto/contextforge/v1 not found walking up from cwd")
		}
		d = parent
	}
}

func protoText() (string, error) {
	dir, err := protoDir()
	if err != nil {
		return "", err
	}
	entries, err := os.ReadDir(dir)
	if err != nil {
		return "", err
	}
	var b strings.Builder
	for _, e := range entries {
		if e.IsDir() || !strings.HasSuffix(e.Name(), ".proto") {
			continue
		}
		data, readErr := os.ReadFile(filepath.Join(dir, e.Name()))
		if readErr != nil {
			return "", readErr
		}
		b.Write(data)
		b.WriteByte('\n')
	}
	return b.String(), nil
}

var (
	msgFieldRe = regexp.MustCompile(`(?m)^\s*(?:repeated\s+|optional\s+)?[A-Za-z_][\w.]*\s+([A-Za-z_]\w*)\s*=\s*\d+\s*;`)
)

// messageBlock returns the body between the braces of `message <msg> { ... }`.
// The proto SSOT uses only flat (non-nested) messages, so a brace counter
// from the opening brace is sufficient and unambiguous.
func messageBlock(txt, msg string) (string, bool) {
	re := regexp.MustCompile(`(?m)^\s*message\s+` + regexp.QuoteMeta(msg) + `\s*\{`)
	loc := re.FindStringIndex(txt)
	if loc == nil {
		return "", false
	}
	open := loc[1] - 1 // index of '{'
	depth := 0
	for j := open; j < len(txt); j++ {
		switch txt[j] {
		case '{':
			depth++
		case '}':
			depth--
			if depth == 0 {
				return txt[open+1 : j], true
			}
		}
	}
	return "", false
}

// MessageFields returns the proto field names declared on the given message,
// read from the frozen proto SSOT.
func MessageFields(msg string) []string {
	txt, err := protoText()
	if err != nil {
		return nil
	}
	body, ok := messageBlock(txt, msg)
	if !ok {
		return nil
	}
	var fields []string
	for _, m := range msgFieldRe.FindAllStringSubmatch(body, -1) {
		fields = append(fields, m[1])
	}
	sort.Strings(fields)
	return fields
}

// FreezeRuleDocumented reports whether the proto SSOT documents both the
// schema_version and the versioning freeze rule (only add fields, never
// delete or renumber tags).
func FreezeRuleDocumented() bool {
	txt, err := protoText()
	if err != nil {
		return false
	}
	low := strings.ToLower(txt)
	return strings.Contains(low, "schema_version") &&
		strings.Contains(low, "frozen") &&
		strings.Contains(low, "only add") &&
		strings.Contains(low, "never delete") &&
		strings.Contains(low, "renumber")
}

// GeneratedGoSmoke proves the Go bindings were code-generated (protoc-gen-go
// + protoc-gen-go-grpc, no FFI) by constructing a generated message, using
// it through the protobuf runtime, and binding the generated gRPC client
// constructor to its expected signature.
func GeneratedGoSmoke() error {
	rec := &contextforgev1.ContextRecord{Id: "ctx_smoke", SchemaVersion: SchemaVersion()}
	var _ proto.Message = rec // protoc-gen-go produced a real proto.Message
	if rec.GetSchemaVersion() != SchemaVersion() {
		return fmt.Errorf("generated getter mismatch: got %q", rec.GetSchemaVersion())
	}
	// protoc-gen-go-grpc produced this client constructor (no FFI / cgo).
	var _ func(grpc.ClientConnInterface) contextforgev1.ContextServiceClient = contextforgev1.NewContextServiceClient
	return nil
}
