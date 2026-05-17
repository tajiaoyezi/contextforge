package contract_test

import (
	"sort"
	"testing"

	"github.com/tajiaoyezi/contextforge/internal/contract"
)

// SCEN/TEST-1.1.x map to docs/specs/tasks/task-1.1-proto.md §7 追踪表 and
// test/features/proto.feature. These tests assert the FROZEN proto /
// canonical-record contract (PRD §Technical Approach Canonical Record v0.1).

// AC1 ContextRecord minimal field set (PRD §Technical Approach + task §6 AC1).
var contextRecordRequired = []string{
	"id", "schema_version", "collection_id", "source_type", "source_provider",
	"source_uri", "agent_scope", "content", "content_hash", "redaction_status",
	"language", "file_path", "line_start", "line_end", "tags", "provenance",
	"security_labels", "created_at", "updated_at", "expires_at", "version", "metadata",
}

func assertSuperset(t *testing.T, have, want []string, msg string) {
	t.Helper()
	set := make(map[string]bool, len(have))
	for _, h := range have {
		set[h] = true
	}
	var missing []string
	for _, w := range want {
		if !set[w] {
			missing = append(missing, w)
		}
	}
	if len(missing) > 0 {
		sort.Strings(missing)
		t.Fatalf("%s: missing proto fields %v (have %v)", msg, missing, have)
	}
}

// TEST-1.1.1 / SCEN-1.1.1 / AC1 — ContextRecord 含 PRD 列出的全部最小字段。
func TestContextRecordMinimalFields(t *testing.T) {
	assertSuperset(t, contract.MessageFields("ContextRecord"), contextRecordRequired,
		"AC1 ContextRecord minimal schema")
}

// TEST-1.1.2 / SCEN-1.1.2 / AC2 — 额外定义 SourceRecord / Chunk / RetrievalResult。
func TestFourCanonicalObjects(t *testing.T) {
	for _, m := range []string{"SourceRecord", "Chunk", "RetrievalResult"} {
		if got := contract.MessageFields(m); len(got) == 0 {
			t.Fatalf("AC2: proto message %q not defined / has no fields", m)
		}
	}
}

// TEST-1.1.3 / SCEN-1.1.3 / AC3 — search 请求/响应字段与 PRD 草案一致。
func TestSearchContract(t *testing.T) {
	assertSuperset(t, contract.MessageFields("SearchRequest"),
		[]string{"query", "collections", "agent_scope", "top_k", "filters", "explain"},
		"AC3 SearchRequest")
	assertSuperset(t, contract.MessageFields("RetrievalResult"),
		[]string{
			"chunk_id", "context_id", "source_type", "file_path", "line_start",
			"line_end", "score", "retrieval_method", "reason", "agent_scope",
			"redaction_status", "provenance",
		},
		"AC3 RetrievalResult")
}

// TEST-1.1.4 / SCEN-1.1.4 / AC4 — Go 侧 grpc-go codegen 成功（无 FFI）。
func TestGoCodegenSucceeds(t *testing.T) {
	if err := contract.GeneratedGoSmoke(); err != nil {
		t.Fatalf("AC4 Go (grpc-go) codegen smoke failed: %v", err)
	}
}

// TEST-1.1.5 / SCEN-1.1.5 / AC5 — schema_version="0.1" + 冻结规则文档化。
func TestSchemaVersionFrozen(t *testing.T) {
	if got := contract.SchemaVersion(); got != "0.1" {
		t.Fatalf("AC5: schema_version = %q, want \"0.1\"", got)
	}
	if !contract.FreezeRuleDocumented() {
		t.Fatalf("AC5: proto must document the freeze rule (only add fields, never delete/renumber tags)")
	}
}
