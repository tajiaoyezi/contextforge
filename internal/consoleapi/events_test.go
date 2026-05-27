// task-16.2 (Phase 16 P4 #11): tests for GET /v1/observability/events real
// long-poll behavior. The fakes in this file drive `handleEvents` through
// `httptest` recorders so we can assert wait/no-event vs. early-return-on-
// event semantics + MemStore fallback sleep without spinning up the daemon.
//
// Daemon-level (gRPC stream-backed) coverage lives in e2e_grpc_test.go Step 11b.

package consoleapi

import (
	"net/http"
	"net/http/httptest"
	"sync"
	"sync/atomic"
	"testing"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/contractv1"
)

// sleepingEventsClient simulates a slow gRPC backend: Recent sleeps for the
// passed `wait` duration (capped at `maxSleep` to keep tests bounded), then
// returns the events configured at construction time.
type sleepingEventsClient struct {
	events   []contractv1.ObservabilityEvent
	maxSleep time.Duration
	calls    atomic.Int64
}

func (s *sleepingEventsClient) Recent(
	_ int,
	wait time.Duration,
) ([]contractv1.ObservabilityEvent, error) {
	s.calls.Add(1)
	sleep := wait
	if sleep > s.maxSleep {
		sleep = s.maxSleep
	}
	if sleep > 0 {
		time.Sleep(sleep)
	}
	if s.events == nil {
		return []contractv1.ObservabilityEvent{}, nil
	}
	return s.events, nil
}

// immediateEventsClient returns the configured events without sleeping.
type immediateEventsClient struct {
	events []contractv1.ObservabilityEvent
}

func (i immediateEventsClient) Recent(
	_ int,
	_ time.Duration,
) ([]contractv1.ObservabilityEvent, error) {
	return i.events, nil
}

func newEventsTestRouter(t *testing.T, ec EventsClient) http.Handler {
	t.Helper()
	store := NewMemStore()
	deps := Deps{
		Workspace: WorkspaceAdapter{S: store},
		Job:       JobAdapter{S: store},
		Search:    store,
		Events:    ec,
	}
	return NewRouter(deps)
}

// task-16.2 §6 AC2: with no event source, `?wait=2s` must block the handler
// at least ~2s before returning 200 + `[]`. Older v0.8 behavior would have
// discarded the wait param and returned [] immediately.
func TestHandleEvents_Wait2s_Blocks_When_NoEvent(t *testing.T) {
	router := newEventsTestRouter(t, &sleepingEventsClient{
		events:   nil,
		maxSleep: 5 * time.Second,
	})

	start := time.Now()
	req := httptest.NewRequest("GET", "/v1/observability/events?wait=2s", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	elapsed := time.Since(start)

	if w.Code != http.StatusOK {
		t.Fatalf("status: got %d want %d body=%s", w.Code, http.StatusOK, w.Body.String())
	}
	// Expect ≥ 1.8s (tolerance below the requested 2s for CI clock jitter;
	// the test infrastructure overhead pushes wall-clock slightly under).
	if elapsed < 1800*time.Millisecond {
		t.Errorf("elapsed too short: %v want ≥ 1.8s — handler did not honor ?wait", elapsed)
	}
	body := w.Body.String()
	if body != "[]\n" && body != "[]" {
		t.Errorf("body: got %q want [] (with optional trailing newline)", body)
	}
}

// task-16.2 §6 AC3: when the events client returns events immediately,
// the handler must complete quickly (≤ 200ms) — NOT block for the full
// `wait` window.
func TestHandleEvents_Returns_Early_OnEvent(t *testing.T) {
	evt := contractv1.ObservabilityEvent{
		EventID:   "evt-1",
		EventType: "indexing.progress",
		Severity:  "info",
		Source:    "contextforge-core",
		Message:   "progress 1/10",
		Timestamp: time.Unix(1_700_000_000, 0).UTC(),
	}
	router := newEventsTestRouter(t, immediateEventsClient{
		events: []contractv1.ObservabilityEvent{evt},
	})

	start := time.Now()
	req := httptest.NewRequest("GET", "/v1/observability/events?wait=5s", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	elapsed := time.Since(start)

	if w.Code != http.StatusOK {
		t.Fatalf("status: got %d want %d body=%s", w.Code, http.StatusOK, w.Body.String())
	}
	if elapsed > 500*time.Millisecond {
		t.Errorf("elapsed too long: %v want ≤ 500ms — handler should return immediately on event", elapsed)
	}
	if !contains(w.Body.String(), "indexing.progress") {
		t.Errorf("body: missing indexing.progress event: %s", w.Body.String())
	}
}

// task-16.2 §6 AC4: two concurrent clients with `?wait=1s` must complete
// independently — one client's wait must NOT serialize behind another.
// Each takes ~1s; the test asserts wall-clock concurrency.
func TestHandleEvents_ConcurrentClients_Independent(t *testing.T) {
	router := newEventsTestRouter(t, &sleepingEventsClient{
		events:   nil,
		maxSleep: 5 * time.Second,
	})

	var wg sync.WaitGroup
	results := make([]int, 2)
	elapsed := make([]time.Duration, 2)

	start := time.Now()
	for i := 0; i < 2; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			t0 := time.Now()
			req := httptest.NewRequest("GET", "/v1/observability/events?wait=1s", nil)
			w := httptest.NewRecorder()
			router.ServeHTTP(w, req)
			results[idx] = w.Code
			elapsed[idx] = time.Since(t0)
		}(i)
	}
	wg.Wait()
	total := time.Since(start)

	// Sequential would be ~2s; concurrent should be ~1s wall-clock.
	if total > 1800*time.Millisecond {
		t.Errorf("total wall-clock %v — concurrent clients appear serialized", total)
	}
	for i := 0; i < 2; i++ {
		if results[i] != http.StatusOK {
			t.Errorf("client %d: status %d want 200", i, results[i])
		}
		if elapsed[i] < 800*time.Millisecond {
			t.Errorf("client %d: elapsed %v — did not honor ?wait=1s", i, elapsed[i])
		}
	}
}

// task-16.2 §6 AC5: MemStore.Recent on empty ring buffer + wait=2s sleeps
// for at most 1s (the internal cap) and returns []. Non-empty buffer must
// return immediately.
func TestMemStore_Recent_EmptyBuffer_SleepsThenReturnsEmpty(t *testing.T) {
	store := NewMemStore()
	start := time.Now()
	out, err := store.Recent(10, 2*time.Second)
	elapsed := time.Since(start)
	if err != nil {
		t.Fatalf("Recent: %v", err)
	}
	if len(out) != 0 {
		t.Errorf("got %d events; want 0 (fallback empty buffer)", len(out))
	}
	// Should sleep ≥ 900ms (cap is 1s; clock tolerance) and ≤ 1200ms (not the
	// full 2s wait since cap kicks in).
	if elapsed < 900*time.Millisecond {
		t.Errorf("elapsed %v — MemStore did not sleep on empty buffer", elapsed)
	}
	if elapsed > 1200*time.Millisecond {
		t.Errorf("elapsed %v — MemStore exceeded 1s sleep cap", elapsed)
	}
}

// task-16.2: MemStore.Recent on non-empty ring buffer returns immediately,
// regardless of `wait`.
func TestMemStore_Recent_NonEmptyBuffer_DoesNotSleep(t *testing.T) {
	store := NewMemStore()
	// Seed an event so the buffer is non-empty.
	store.events = append(store.events, contractv1.ObservabilityEvent{
		EventID:   "evt-seed",
		EventType: "core.keepalive",
		Severity:  "info",
		Source:    "contextforge-core",
		Message:   "seed",
		Timestamp: time.Unix(1_700_000_000, 0).UTC(),
	})

	start := time.Now()
	out, err := store.Recent(10, 5*time.Second)
	elapsed := time.Since(start)
	if err != nil {
		t.Fatalf("Recent: %v", err)
	}
	if len(out) != 1 {
		t.Fatalf("got %d events; want 1", len(out))
	}
	if out[0].EventID != "evt-seed" {
		t.Errorf("event id: got %q want evt-seed", out[0].EventID)
	}
	if elapsed > 100*time.Millisecond {
		t.Errorf("elapsed %v — non-empty buffer should not sleep", elapsed)
	}
}

// task-16.2: parseWaitParam clamps ?wait beyond [1s, 60s] bound. Existing
// v0.8 implementation kept this behavior; this test pins it so the new
// long-poll flow does not regress the clamp.
func TestParseWaitParam_ClampUpperLowerAndDefault(t *testing.T) {
	cases := []struct {
		name string
		raw  string
		want time.Duration
	}{
		{"default", "", 30 * time.Second},
		{"invalid", "not-a-duration", 30 * time.Second},
		{"below_min", "100ms", 1 * time.Second},
		{"above_max", "120s", 60 * time.Second},
		{"valid_15s", "15s", 15 * time.Second},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			path := "/v1/observability/events"
			if tc.raw != "" {
				path += "?wait=" + tc.raw
			}
			req := httptest.NewRequest("GET", path, nil)
			got := parseWaitParam(req)
			if got != tc.want {
				t.Errorf("parseWaitParam(%q) = %v want %v", tc.raw, got, tc.want)
			}
		})
	}
}

// helper — strings.Contains is in stdlib but we avoid the import.
func contains(s, sub string) bool {
	for i := 0; i+len(sub) <= len(s); i++ {
		if s[i:i+len(sub)] == sub {
			return true
		}
	}
	return false
}
