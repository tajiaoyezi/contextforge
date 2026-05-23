// task-6.1: contextforge search 子命令 RED 测试集 (TEST-6.1.1 ~ 6.1.4).
//
// 设计：CLI 单元测用 fake fetcher (var hook) 注入 SearchResponse，
// 跳过真实 daemon spawn + cargo build（端到端真路径走 core/tests/phase6_smoke.rs
// = TEST-6.1.5 / AC5）。本文件 4 测试覆盖 AC1/AC2/AC3/AC4 + flag 契约 + 渲染.

package cli

import (
	"bytes"
	"context"
	"encoding/json"
	"strings"
	"testing"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// fakeResponse — 合成 SearchResponse fixture：12 字段齐 + provenance ≥ 1.
func fakeResponse() *contextforgev1.SearchResponse {
	return &contextforgev1.SearchResponse{
		Results: []*contextforgev1.RetrievalResult{
			{
				ChunkId:         "chunk-1",
				ContextId:       "",
				SourceType:      "",
				FilePath:        "fixtures/readme.md",
				LineStart:       1,
				LineEnd:         5,
				Score:           0.93,
				RetrievalMethod: "bm25",
				Reason:          "bm25 hit on 'fake-trigger'; matched terms: [fake-trigger]",
				AgentScope:      []string{},
				RedactionStatus: "applied",
				Provenance: []*contextforgev1.Provenance{
					{
						Importer:     "scanner",
						OriginalPath: "fixtures/readme.md",
					},
				},
			},
			{
				ChunkId:         "chunk-2",
				FilePath:        "fixtures/notes.md",
				LineStart:       10,
				LineEnd:         12,
				Score:           0.55,
				RetrievalMethod: "bm25",
				RedactionStatus: "applied",
				Provenance: []*contextforgev1.Provenance{
					{Importer: "scanner", OriginalPath: "fixtures/notes.md"},
				},
			},
		},
	}
}

// TEST-6.1.1 / SCEN-6.1.1 / AC1 — runSearch 调用 search backend 并把 Top-K 渲染到 stdout.
//
// 用 fetchSearchResults var hook 注入 fake；不真实 spawn daemon。
func TestTask61_AC1_RunSearchInvokesBackendAndRenders(t *testing.T) {
	orig := fetchSearchResults
	defer func() { fetchSearchResults = orig }()
	var capturedReq *contextforgev1.SearchRequest
	fetchSearchResults = func(ctx context.Context, req *contextforgev1.SearchRequest) (*contextforgev1.SearchResponse, error) {
		capturedReq = req
		return fakeResponse(), nil
	}

	var stdout, stderr bytes.Buffer
	code := runSearch([]string{"--collections=c1", "fake-trigger"}, &stdout, &stderr)
	if code != 0 {
		t.Fatalf("AC1: exit=%d stderr=%q", code, stderr.String())
	}
	if capturedReq == nil {
		t.Fatalf("AC1: backend was not invoked (capturedReq nil)")
	}
	if capturedReq.GetQuery() != "fake-trigger" {
		t.Fatalf("AC1: backend req.Query=%q want fake-trigger", capturedReq.GetQuery())
	}
	// Top-K 渲染到 stdout：text 模式应含两条结果的 chunk_id
	out := stdout.String()
	if !strings.Contains(out, "chunk-1") || !strings.Contains(out, "chunk-2") {
		t.Fatalf("AC1: stdout should render Top-K results (chunk-1 + chunk-2), got:\n%s", out)
	}
}

// TEST-6.1.2 / SCEN-6.1.2 / AC2 — flag 契约：解析 --collections / --agent-scope /
// --top-k / --source-type / --language / --explain 1:1 映射 SearchRequest.
func TestTask61_AC2_FlagContractMapsToProtoRequest(t *testing.T) {
	t.Run("ParseAllFlagsPositionalQuery", func(t *testing.T) {
		args := []string{
			"--collections=c1,c2",
			"--agent-scope=a1,a2",
			"--top-k=5",
			"--source-type=markdown",
			"--language=go,rust",
			"--explain",
			"my-query-text",
		}
		var stderr bytes.Buffer
		opts, err := parseSearchOpts(args, &stderr)
		if err != nil {
			t.Fatalf("AC2 parse: %v stderr=%q", err, stderr.String())
		}
		if opts.Query != "my-query-text" {
			t.Fatalf("AC2: Query=%q want my-query-text", opts.Query)
		}
		if got, want := opts.Collections, []string{"c1", "c2"}; !equalStrs(got, want) {
			t.Fatalf("AC2: Collections=%v want %v", got, want)
		}
		if got, want := opts.AgentScope, []string{"a1", "a2"}; !equalStrs(got, want) {
			t.Fatalf("AC2: AgentScope=%v want %v", got, want)
		}
		if opts.TopK != 5 {
			t.Fatalf("AC2: TopK=%d want 5", opts.TopK)
		}
		if got, want := opts.SourceType, []string{"markdown"}; !equalStrs(got, want) {
			t.Fatalf("AC2: SourceType=%v want %v", got, want)
		}
		if got, want := opts.Language, []string{"go", "rust"}; !equalStrs(got, want) {
			t.Fatalf("AC2: Language=%v want %v", got, want)
		}
		if !opts.Explain {
			t.Fatalf("AC2: Explain=false, want true")
		}

		req := optsToProtoRequest(opts)
		if req.GetQuery() != "my-query-text" {
			t.Fatalf("AC2 proto: Query=%q want my-query-text", req.GetQuery())
		}
		if got, want := req.GetCollections(), []string{"c1", "c2"}; !equalStrs(got, want) {
			t.Fatalf("AC2 proto: Collections=%v want %v", got, want)
		}
		if got, want := req.GetAgentScope(), []string{"a1", "a2"}; !equalStrs(got, want) {
			t.Fatalf("AC2 proto: AgentScope=%v want %v", got, want)
		}
		if req.GetTopK() != 5 {
			t.Fatalf("AC2 proto: TopK=%d want 5", req.GetTopK())
		}
		if !req.GetExplain() {
			t.Fatalf("AC2 proto: Explain=false want true")
		}
		f := req.GetFilters()
		if f == nil {
			t.Fatalf("AC2 proto: Filters nil")
		}
		if got, want := f.GetSourceType(), []string{"markdown"}; !equalStrs(got, want) {
			t.Fatalf("AC2 proto: Filters.SourceType=%v want %v", got, want)
		}
		if got, want := f.GetLanguage(), []string{"go", "rust"}; !equalStrs(got, want) {
			t.Fatalf("AC2 proto: Filters.Language=%v want %v", got, want)
		}
	})

	t.Run("TopKZeroFallsBackToDefault", func(t *testing.T) {
		args := []string{"--top-k=0", "q"}
		var stderr bytes.Buffer
		opts, err := parseSearchOpts(args, &stderr)
		if err != nil {
			t.Fatalf("AC2 fallback parse: %v", err)
		}
		req := optsToProtoRequest(opts)
		if req.GetTopK() != 10 {
			t.Fatalf("AC2: --top-k=0 应回退 default 10, got %d", req.GetTopK())
		}
	})

	t.Run("MissingQueryReturnsError", func(t *testing.T) {
		var stderr bytes.Buffer
		_, err := parseSearchOpts([]string{"--top-k=5"}, &stderr)
		if err == nil {
			t.Fatalf("AC2: 空 positional query 应返错")
		}
	})
}

// TEST-6.1.3 / SCEN-6.1.3 / AC3 — text 默认 / --json 二选一渲染，含全部可解释字段.
func TestTask61_AC3_TextAndJSONRendering(t *testing.T) {
	resp := fakeResponse()

	t.Run("RenderText", func(t *testing.T) {
		var buf bytes.Buffer
		if err := renderText(resp, &buf); err != nil {
			t.Fatalf("AC3 renderText: %v", err)
		}
		out := buf.String()
		// text 模式应含可解释字段（chunk_id / file_path:line / score / redaction_status / reason）
		for _, want := range []string{
			"chunk-1",
			"fixtures/readme.md:1-5",
			"score=",
			"redaction_status=applied",
			"reason=",
		} {
			if !strings.Contains(out, want) {
				t.Fatalf("AC3 text: 缺少 %q, full output:\n%s", want, out)
			}
		}
	})

	t.Run("RenderJSON", func(t *testing.T) {
		var buf bytes.Buffer
		if err := renderJSON(resp, &buf); err != nil {
			t.Fatalf("AC3 renderJSON: %v", err)
		}
		// 验证 JSON 可被反解析回结构 + 关键字段存在
		var roundtrip map[string]any
		if err := json.Unmarshal(buf.Bytes(), &roundtrip); err != nil {
			t.Fatalf("AC3 JSON 不是合法 JSON: %v\nout: %s", err, buf.String())
		}
		results, ok := roundtrip["results"].([]any)
		if !ok || len(results) == 0 {
			t.Fatalf("AC3 JSON: results 字段缺失或空, full:\n%s", buf.String())
		}
		first := results[0].(map[string]any)
		// proto-generated json tag: chunk_id / file_path / redaction_status / line_start ...
		if first["chunk_id"] != "chunk-1" {
			t.Fatalf("AC3 JSON: results[0].chunk_id=%v want chunk-1", first["chunk_id"])
		}
		if first["redaction_status"] != "applied" {
			t.Fatalf("AC3 JSON: results[0].redaction_status=%v want applied", first["redaction_status"])
		}
		prov, ok := first["provenance"].([]any)
		if !ok || len(prov) == 0 {
			t.Fatalf("AC3 JSON: provenance 数组缺失或空, full:\n%s", buf.String())
		}
	})

	t.Run("RunSearchJSONFlagSelectsJSONOutput", func(t *testing.T) {
		orig := fetchSearchResults
		defer func() { fetchSearchResults = orig }()
		fetchSearchResults = func(ctx context.Context, req *contextforgev1.SearchRequest) (*contextforgev1.SearchResponse, error) {
			return fakeResponse(), nil
		}
		var stdout, stderr bytes.Buffer
		code := runSearch([]string{"--json", "q"}, &stdout, &stderr)
		if code != 0 {
			t.Fatalf("AC3 runSearch --json: exit=%d stderr=%q", code, stderr.String())
		}
		// stdout 应为合法 JSON
		var roundtrip map[string]any
		if err := json.Unmarshal(stdout.Bytes(), &roundtrip); err != nil {
			t.Fatalf("AC3 runSearch --json: stdout 不是合法 JSON: %v\nout: %s", err, stdout.String())
		}
	})
}

// TEST-6.1.4 / SCEN-6.1.4 / AC4 — 透传 redaction_status，不二次扫 content.
func TestTask61_AC4_RedactionStatusPassthrough(t *testing.T) {
	resp := fakeResponse()
	// 验证 fake response 设定（基线 sanity）
	if resp.Results[0].GetRedactionStatus() != "applied" {
		t.Fatalf("AC4 fixture sanity: redaction_status should be 'applied'")
	}

	t.Run("TextRenderShowsField", func(t *testing.T) {
		var buf bytes.Buffer
		if err := renderText(resp, &buf); err != nil {
			t.Fatalf("renderText: %v", err)
		}
		// AC4: text 渲染显式打 redaction_status 字段值（不二次扫 content）
		if !strings.Contains(buf.String(), "redaction_status=applied") {
			t.Fatalf("AC4 text: 缺 redaction_status=applied 字段, output:\n%s", buf.String())
		}
	})

	t.Run("JSONRenderShowsField", func(t *testing.T) {
		var buf bytes.Buffer
		if err := renderJSON(resp, &buf); err != nil {
			t.Fatalf("renderJSON: %v", err)
		}
		// AC4: JSON 渲染 redaction_status 字段透传 (proto json tag)
		if !strings.Contains(buf.String(), `"redaction_status":"applied"`) {
			t.Fatalf("AC4 JSON: 缺 redaction_status:applied JSON 字段, output:\n%s", buf.String())
		}
	})

	t.Run("ContentNotReScanned", func(t *testing.T) {
		// 黑盒 sanity：CLI 不对 content 做 secret scan（content 字段在 RetrievalResult 中也不存在，
		// retriever 内部用 redacted_content；CLI 仅消费 proto-defined 12 字段 + redaction_status）.
		// 这里通过验证 RetrievalResult proto 没有 raw_content 字段间接覆盖.
		fields := []string{
			"chunk_id", "context_id", "source_type", "file_path",
			"line_start", "line_end", "score", "retrieval_method",
			"reason", "agent_scope", "redaction_status", "provenance",
		}
		var buf bytes.Buffer
		if err := renderJSON(resp, &buf); err != nil {
			t.Fatalf("renderJSON: %v", err)
		}
		out := buf.String()
		// 12 字段全部 JSON 透传可见
		for _, f := range fields {
			if !strings.Contains(out, `"`+f+`"`) {
				t.Fatalf("AC4/AC5 schema parity: JSON 缺字段 %q, output:\n%s", f, out)
			}
		}
	})
}

// equalStrs — slice 比较 helper（test-only，避免引 reflect.DeepEqual）.
func equalStrs(a, b []string) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}
