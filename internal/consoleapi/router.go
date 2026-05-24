package consoleapi

import (
	"crypto/subtle"
	"encoding/json"
	"errors"
	"net/http"
	"strings"
)

// NewRouter returns the http.Handler tree for the 9 Console Contract v1
// endpoints + bearer auth middleware + JSON error mapping.
func NewRouter(deps Deps) http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /v1/health", handleHealth(deps))
	mux.HandleFunc("POST /v1/workspaces", handleCreateWorkspace(deps))
	mux.HandleFunc("GET /v1/workspaces", handleListWorkspaces(deps))
	mux.HandleFunc("GET /v1/workspaces/{id}", handleGetWorkspace(deps))
	mux.HandleFunc("POST /v1/index-jobs", handleEnqueueJob(deps))
	mux.HandleFunc("GET /v1/index-jobs/{id}", handleGetJob(deps))
	mux.HandleFunc("POST /v1/index-jobs/{id}/cancel", handleCancelJob(deps))
	mux.HandleFunc("POST /v1/search", handleSearch(deps))
	mux.HandleFunc("GET /v1/observability/events", handleEvents(deps))
	return bearerAuthMiddleware(mux, deps.AuthToken)
}

// bearerAuthMiddleware enforces `Authorization: Bearer <token>` when
// `token != ""`. Empty token = trusted-network mode (no header required).
// Constant-time compare avoids timing-side-channel leaks.
func bearerAuthMiddleware(inner http.Handler, token string) http.Handler {
	if token == "" {
		return inner
	}
	expected := "Bearer " + token
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		got := r.Header.Get("Authorization")
		if subtle.ConstantTimeCompare([]byte(got), []byte(expected)) != 1 {
			writeError(w, http.StatusUnauthorized, "UNAUTHORIZED", "missing or invalid bearer token")
			return
		}
		inner.ServeHTTP(w, r)
	})
}

// writeJSON marshals v as JSON with Content-Type application/json + status.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

// writeError emits the ErrorBody shape with the supplied status code, code
// label, and message. Code labels align loosely with Console HTTPAdapter
// sentinel mapping (NOT_FOUND / CONFLICT / UNAUTHORIZED / BAD_REQUEST).
func writeError(w http.ResponseWriter, status int, code, message string) {
	var body ErrorBody
	body.Error.Code = code
	body.Error.Message = message
	writeJSON(w, status, body)
}

// mapStorageError translates a backend error into a writeError + return so
// handlers stay tiny.
func mapStorageError(w http.ResponseWriter, err error) {
	switch {
	case errors.Is(err, ErrNotFound):
		writeError(w, http.StatusNotFound, "NOT_FOUND", err.Error())
	case errors.Is(err, ErrJobTerminal):
		writeError(w, http.StatusConflict, "CONFLICT", err.Error())
	case errors.Is(err, ErrInvalidRequest):
		writeError(w, http.StatusBadRequest, "BAD_REQUEST", err.Error())
	default:
		writeError(w, http.StatusInternalServerError, "INTERNAL", err.Error())
	}
}

// readJSONBody decodes the request body into out. Returns false (and writes
// 400) on failure; handlers should `return` immediately when false.
func readJSONBody(w http.ResponseWriter, r *http.Request, out any) bool {
	dec := json.NewDecoder(r.Body)
	dec.DisallowUnknownFields()
	if err := dec.Decode(out); err != nil {
		writeError(w, http.StatusBadRequest, "BAD_REQUEST", "invalid JSON body: "+err.Error())
		return false
	}
	return true
}

// trimID is a tiny helper extracting and trimming PathValue("id").
func trimID(r *http.Request) string {
	return strings.TrimSpace(r.PathValue("id"))
}
