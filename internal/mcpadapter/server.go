package mcpadapter

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"strings"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/memoryops/audit"
	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// Searcher is the daemon.Search-compatible boundary consumed by MCP tools.
// Production wires *daemon.Daemon from cmd/contextforge/main.go; tests use a
// fake. Keeping this package interface-based preserves the adapter/core split.
type Searcher interface {
	Search(context.Context, *contextforgev1.SearchRequest) (*contextforgev1.SearchResponse, error)
}

// Server is an MCP 2025-06-18 stdio JSON-RPC server.
type Server struct {
	Searcher  Searcher
	DataDir   string
	Allowlist []AllowlistEntry

	initialized bool
}

// InitializeParams is the MCP initialize request params subset used here.
type InitializeParams struct {
	ProtocolVersion string         `json:"protocolVersion"`
	Capabilities    map[string]any `json:"capabilities,omitempty"`
	ClientInfo      ClientInfo     `json:"clientInfo"`
}

type ClientInfo struct {
	Name    string `json:"name"`
	Version string `json:"version"`
}

type InitializeResult struct {
	ProtocolVersion string         `json:"protocolVersion"`
	Capabilities    map[string]any `json:"capabilities"`
	ServerInfo      ServerInfo     `json:"serverInfo"`
}

type ServerInfo struct {
	Name    string `json:"name"`
	Version string `json:"version"`
}

type ToolDef struct {
	Name        string         `json:"name"`
	Description string         `json:"description"`
	InputSchema map[string]any `json:"inputSchema"`
}

type CallToolResult struct {
	Content           []ToolContent `json:"content"`
	StructuredContent any           `json:"structuredContent,omitempty"`
	IsError           bool          `json:"isError,omitempty"`
}

type ToolContent struct {
	Type string `json:"type"`
	Text string `json:"text"`
}

// Serve reads newline-delimited JSON-RPC messages from stdin and writes only
// valid MCP messages to stdout. stdin EOF returns nil.
func (s *Server) Serve(ctx context.Context, stdin io.Reader, stdout io.Writer) error {
	scanner := readJSONRPCLines(stdin)
	writer := bufioWriter(stdout)
	for scanner.Scan() {
		select {
		case <-ctx.Done():
			return ctx.Err()
		default:
		}
		line := strings.TrimSpace(scanner.Text())
		if line == "" {
			continue
		}
		var req JSONRPCRequest
		if err := json.Unmarshal([]byte(line), &req); err != nil {
			resp := errorResponse(nil, newRPCError(codeParseError, "parse error", err.Error()))
			if werr := writeJSONRPCLine(writer, resp); werr != nil {
				return werr
			}
			continue
		}
		resp, shouldWrite, closeAfter := s.handleRequest(ctx, req)
		if shouldWrite {
			if err := writeJSONRPCLine(writer, resp); err != nil {
				return err
			}
		}
		if closeAfter {
			return nil
		}
	}
	return scanner.Err()
}

func bufioWriter(w io.Writer) *bufio.Writer {
	return bufio.NewWriter(w)
}

func (s *Server) handleRequest(ctx context.Context, req JSONRPCRequest) (JSONRPCResponse, bool, bool) {
	if err := validateRequest(req); err != nil {
		if req.ID == nil {
			return JSONRPCResponse{}, false, false
		}
		return errorResponse(req.ID, err), true, err.close
	}

	switch req.Method {
	case "initialize":
		var params InitializeParams
		if err := paramsInto(req.Params, &params); err != nil {
			rerr := newRPCError(codeInvalidParams, "invalid initialize params", err.Error())
			return errorResponse(req.ID, rerr), req.ID != nil, false
		}
		result, err := s.handleInitialize(ctx, params)
		if err != nil {
			rerr := asRPCError(err)
			return errorResponse(req.ID, rerr), req.ID != nil, rerr.close
		}
		s.initialized = true
		return successResponse(req.ID, result), req.ID != nil, false

	case "notifications/initialized":
		s.initialized = true
		return JSONRPCResponse{}, false, false

	case "tools/list":
		if !s.initialized {
			err := newRPCError(codeServerError, "server not initialized", nil)
			return errorResponse(req.ID, err), req.ID != nil, false
		}
		tools, err := s.handleListTools(ctx)
		if err != nil {
			rerr := asRPCError(err)
			return errorResponse(req.ID, rerr), req.ID != nil, rerr.close
		}
		return successResponse(req.ID, map[string]any{"tools": tools}), req.ID != nil, false

	case "tools/call":
		if !s.initialized {
			err := newRPCError(codeServerError, "server not initialized", nil)
			return errorResponse(req.ID, err), req.ID != nil, false
		}
		var params struct {
			Name      string         `json:"name"`
			Arguments map[string]any `json:"arguments,omitempty"`
		}
		if err := paramsInto(req.Params, &params); err != nil {
			rerr := newRPCError(codeInvalidParams, "invalid tools/call params", err.Error())
			return errorResponse(req.ID, rerr), req.ID != nil, false
		}
		result, err := s.handleCallTool(ctx, params.Name, params.Arguments)
		if err != nil {
			rerr := asRPCError(err)
			return errorResponse(req.ID, rerr), req.ID != nil, rerr.close
		}
		return successResponse(req.ID, result), req.ID != nil, false

	default:
		err := methodNotFound(req.Method)
		return errorResponse(req.ID, err), req.ID != nil, false
	}
}

func (s *Server) handleInitialize(_ context.Context, params InitializeParams) (InitializeResult, error) {
	client := AllowlistEntry{Name: params.ClientInfo.Name, Version: params.ClientInfo.Version}
	if !IsAllowlisted(client, s.Allowlist) {
		s.writeAudit("mcp:initialize", 403, "client not allowlisted")
		return InitializeResult{}, newClosingRPCError(codeServerError, "client not allowlisted", nil)
	}
	if strings.TrimSpace(params.ProtocolVersion) == "" {
		s.writeAudit("mcp:initialize", 400, "missing protocol version")
		return InitializeResult{}, newRPCError(codeInvalidParams, "missing protocolVersion", nil)
	}
	if params.ProtocolVersion < SupportedProtocolVersion {
		s.writeAudit("mcp:initialize", 400, "unsupported protocol version")
		return InitializeResult{}, newRPCError(codeInvalidParams, "unsupported protocol version", map[string]any{
			"supported": []string{SupportedProtocolVersion},
			"requested": params.ProtocolVersion,
		})
	}
	s.writeAudit("mcp:initialize", 200, "")
	return InitializeResult{
		ProtocolVersion: SupportedProtocolVersion,
		Capabilities: map[string]any{
			"tools": map[string]any{"listChanged": false},
		},
		ServerInfo: ServerInfo{Name: "contextforge", Version: "0.1.0"},
	}, nil
}

func (s *Server) handleListTools(_ context.Context) ([]ToolDef, error) {
	return []ToolDef{
		toolDef("context_search", "Search governed ContextForge context"),
		toolDef("context_read", "Read one context chunk by chunk_id"),
		toolDef("context_explain", "Search with retrieval reasons and provenance"),
		toolDef("context_collections", "List available ContextForge collections"),
	}, nil
}

func (s *Server) handleCallTool(ctx context.Context, name string, args map[string]any) (CallToolResult, error) {
	if args == nil {
		args = map[string]any{}
	}
	var (
		payload any
		err     error
	)
	switch name {
	case "context_search":
		payload, err = s.callContextSearch(ctx, args)
	case "context_read":
		payload, err = s.callContextRead(ctx, args)
	case "context_explain":
		payload, err = s.callContextExplain(ctx, args)
	case "context_collections":
		payload, err = s.callContextCollections(ctx, args)
	default:
		return CallToolResult{}, methodNotFound(name)
	}
	endpoint := "mcp:" + name
	if err != nil {
		s.writeAudit(endpoint, 500, "tool call failed")
		return CallToolResult{}, asRPCError(err)
	}
	s.writeAudit(endpoint, 200, "")
	return callToolResult(payload)
}

func toolDef(name, desc string) ToolDef {
	return ToolDef{
		Name:        name,
		Description: desc,
		InputSchema: map[string]any{
			"type":                 "object",
			"additionalProperties": true,
		},
	}
}

func callToolResult(payload any) (CallToolResult, error) {
	b, err := json.Marshal(payload)
	if err != nil {
		return CallToolResult{}, err
	}
	return CallToolResult{
		Content: []ToolContent{
			{Type: "text", Text: string(b)},
		},
		StructuredContent: payload,
	}, nil
}

func (s *Server) writeAudit(endpoint string, status int, reason string) {
	if s.DataDir == "" {
		return
	}
	_ = audit.Write(s.DataDir, audit.Event{
		Endpoint:  endpoint,
		Status:    status,
		Timestamp: time.Now().UTC(),
		Reason:    reason,
	})
}

func asRPCError(err error) *rpcError {
	if err == nil {
		return nil
	}
	if rerr, ok := err.(*rpcError); ok {
		return rerr
	}
	return newRPCError(codeServerError, fmt.Sprintf("%v", err), nil)
}
