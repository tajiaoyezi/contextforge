// Package mcpadapter implements the MCP 2025-06-18 stdio JSON-RPC server.
//
// R7 strict channel: this package intentionally uses only the Go standard
// library plus existing ContextForge internal/proto packages. No MCP SDK is
// imported.
package mcpadapter

import (
	"bufio"
	"encoding/json"
	"fmt"
	"io"
)

// SupportedProtocolVersion is the MCP spec version locked by task-7.1.
const SupportedProtocolVersion = "2025-06-18"

const (
	codeParseError     = -32700
	codeInvalidRequest = -32600
	codeMethodNotFound = -32601
	codeInvalidParams  = -32602
	codeInternalError  = -32603
	codeServerError    = -32000
)

// JSONRPCRequest is a JSON-RPC 2.0 request / notification. ID is nil for
// notifications.
type JSONRPCRequest struct {
	JSONRPC string         `json:"jsonrpc"`
	ID      any            `json:"id,omitempty"`
	Method  string         `json:"method"`
	Params  map[string]any `json:"params,omitempty"`
}

// JSONRPCResponse is a JSON-RPC 2.0 response.
type JSONRPCResponse struct {
	JSONRPC string        `json:"jsonrpc"`
	ID      any           `json:"id"`
	Result  any           `json:"result,omitempty"`
	Error   *JSONRPCError `json:"error,omitempty"`
}

// JSONRPCError is a JSON-RPC 2.0 error object.
type JSONRPCError struct {
	Code    int    `json:"code"`
	Message string `json:"message"`
	Data    any    `json:"data,omitempty"`
}

type rpcError struct {
	code    int
	message string
	data    any
	close   bool
}

func (e *rpcError) Error() string {
	return e.message
}

func newRPCError(code int, message string, data any) *rpcError {
	return &rpcError{code: code, message: message, data: data}
}

func newClosingRPCError(code int, message string, data any) *rpcError {
	return &rpcError{code: code, message: message, data: data, close: true}
}

func successResponse(id any, result any) JSONRPCResponse {
	return JSONRPCResponse{JSONRPC: "2.0", ID: id, Result: result}
}

func errorResponse(id any, err *rpcError) JSONRPCResponse {
	return JSONRPCResponse{
		JSONRPC: "2.0",
		ID:      id,
		Error: &JSONRPCError{
			Code:    err.code,
			Message: err.message,
			Data:    err.data,
		},
	}
}

func writeJSONRPCLine(w *bufio.Writer, resp JSONRPCResponse) error {
	if err := json.NewEncoder(w).Encode(resp); err != nil {
		return err
	}
	return w.Flush()
}

func paramsInto(params map[string]any, out any) error {
	if params == nil {
		params = map[string]any{}
	}
	b, err := json.Marshal(params)
	if err != nil {
		return err
	}
	if err := json.Unmarshal(b, out); err != nil {
		return err
	}
	return nil
}

func readJSONRPCLines(r io.Reader) *bufio.Scanner {
	scanner := bufio.NewScanner(r)
	scanner.Buffer(make([]byte, 0, 64*1024), 16*1024*1024)
	return scanner
}

func validateRequest(req JSONRPCRequest) *rpcError {
	if req.JSONRPC != "2.0" {
		return newRPCError(codeInvalidRequest, "invalid JSON-RPC version", nil)
	}
	if req.Method == "" {
		return newRPCError(codeInvalidRequest, "missing method", nil)
	}
	return nil
}

func methodNotFound(method string) *rpcError {
	return newRPCError(codeMethodNotFound, fmt.Sprintf("method not found: %s", method), nil)
}
