// task-16.2 (Phase 16 P4 #11): tests for GET /v1/observability/events real
// long-poll behavior. The fakes in this file drive `handleEvents` through
// `httptest` recorders so we can assert wait/no-event vs. early-return-on-
// event semantics + MemStore fallback sleep without spinning up the daemon.
//
// Daemon-level (gRPC stream-backed) coverage lives in e2e_grpc_test.go Step 11b.

package consoleapi

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync"
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
}

func (s *sleepingEventsClient) Recent(
	_ int,
	wait time.Duration,
) ([]contractv1.ObservabilityEvent, error) {
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
	if !strings.Contains(w.Body.String(), "indexing.progress") {
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

// =====================================================================
// task-26.2 (Phase 26 / ADR-031 D3/D4) — SSE push + audit replay contract.
// =====================================================================

// fakeStreamer is a deterministic EventsStreamer test double: it emits the
// configured events in order then closes the channel (unless blockForever, in
// which case it waits on ctx after emitting so disconnect can be exercised).
// It records the StreamOptions it received and whether ctx was observed done.
type fakeStreamer struct {
	events       []contractv1.ObservabilityEvent
	blockForever bool
	mu           sync.Mutex
	gotCtxDone   bool
	lastOpts     StreamOptions
}

func (f *fakeStreamer) Stream(ctx context.Context, opts StreamOptions) (<-chan contractv1.ObservabilityEvent, error) {
	f.mu.Lock()
	f.lastOpts = opts
	f.mu.Unlock()
	ch := make(chan contractv1.ObservabilityEvent)
	go func() {
		defer close(ch)
		for _, e := range f.events {
			select {
			case ch <- e:
			case <-ctx.Done():
				f.mu.Lock()
				f.gotCtxDone = true
				f.mu.Unlock()
				return
			}
		}
		if f.blockForever {
			<-ctx.Done()
			f.mu.Lock()
			f.gotCtxDone = true
			f.mu.Unlock()
		}
	}()
	return ch, nil
}

func evt(id, etype string) contractv1.ObservabilityEvent {
	return contractv1.ObservabilityEvent{
		EventID:   id,
		EventType: etype,
		Severity:  "info",
		Source:    "contextforge-core",
		Message:   "memory " + id,
		Timestamp: time.Unix(1_700_000_000, 0).UTC(),
	}
}

func newStreamTestRouter(t *testing.T, es EventsStreamer) http.Handler {
	t.Helper()
	store := NewMemStore()
	deps := Deps{
		Workspace:    WorkspaceAdapter{S: store},
		Job:          JobAdapter{S: store},
		Search:       store,
		Events:       store,
		EventsStream: es,
	}
	return NewRouter(deps)
}

// parseSSEFrames splits an SSE response body into frames (split on blank line)
// and returns each frame's id/event/data line values.
func parseSSEFrames(body string) []map[string]string {
	var frames []map[string]string
	for _, block := range strings.Split(strings.TrimRight(body, "\n"), "\n\n") {
		if strings.TrimSpace(block) == "" {
			continue
		}
		f := map[string]string{}
		for _, line := range strings.Split(block, "\n") {
			if k, v, ok := strings.Cut(line, ": "); ok {
				f[k] = v
			}
		}
		frames = append(frames, f)
	}
	return frames
}

// TEST-26.2.1 / AC1: SSE endpoint encodes each event as an id/event/data frame
// in order; data is a valid JSON ObservabilityEvent.
func TestEventsStream_SSEFrameEncodingAndOrder(t *testing.T) {
	router := newStreamTestRouter(t, &fakeStreamer{events: []contractv1.ObservabilityEvent{
		evt("e1", "memory.pin"),
		evt("e2", "memory.deprecate"),
		evt("e3", "memory.soft_delete"),
	}})
	req := httptest.NewRequest("GET", "/v1/observability/events/stream", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("status: got %d want 200; body=%s", w.Code, w.Body.String())
	}
	if ct := w.Header().Get("Content-Type"); !strings.HasPrefix(ct, "text/event-stream") {
		t.Errorf("Content-Type: got %q want text/event-stream", ct)
	}
	frames := parseSSEFrames(w.Body.String())
	if len(frames) != 3 {
		t.Fatalf("got %d frames want 3; body=%q", len(frames), w.Body.String())
	}
	wantIDs := []string{"e1", "e2", "e3"}
	wantTypes := []string{"memory.pin", "memory.deprecate", "memory.soft_delete"}
	for i, f := range frames {
		if f["id"] != wantIDs[i] {
			t.Errorf("frame %d id: got %q want %q", i, f["id"], wantIDs[i])
		}
		if f["event"] != wantTypes[i] {
			t.Errorf("frame %d event: got %q want %q", i, f["event"], wantTypes[i])
		}
		var got contractv1.ObservabilityEvent
		if err := json.Unmarshal([]byte(f["data"]), &got); err != nil {
			t.Errorf("frame %d data not valid JSON ObservabilityEvent: %v (%q)", i, err, f["data"])
			continue
		}
		if got.EventID != wantIDs[i] {
			t.Errorf("frame %d data event_id: got %q want %q", i, got.EventID, wantIDs[i])
		}
	}
}

// TEST-26.2.2 / AC2: SSE is add-only — the existing long-poll endpoint still
// returns 200 + [], and the stream endpoint is nil-safe (503 without a streamer).
func TestEventsStream_AddOnly_LongPollUnchanged_NilSafe(t *testing.T) {
	// Long-poll endpoint unchanged (Events client present, EventsStream nil).
	router := newStreamTestRouter(t, nil)
	req := httptest.NewRequest("GET", "/v1/observability/events?wait=1s", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("long-poll status: got %d want 200", w.Code)
	}
	if b := strings.TrimSpace(w.Body.String()); b != "[]" {
		t.Errorf("long-poll body: got %q want []", b)
	}
	// Stream endpoint with no streamer → 503 (preserves long-poll-only contract).
	req2 := httptest.NewRequest("GET", "/v1/observability/events/stream", nil)
	w2 := httptest.NewRecorder()
	router.ServeHTTP(w2, req2)
	if w2.Code != http.StatusServiceUnavailable {
		t.Errorf("stream nil-streamer status: got %d want 503", w2.Code)
	}
}

// TEST-26.2.3 / AC3: replay-then-live splice dedups by event_id at the boundary
// (a duplicate id is emitted once); ?since_ts= is forwarded to the streamer.
func TestEventsStream_ReplayThenLive_DedupAndSinceTS(t *testing.T) {
	fs := &fakeStreamer{events: []contractv1.ObservabilityEvent{
		evt("evt-audit-1", "memory.pin"),       // replay
		evt("evt-audit-2", "memory.deprecate"), // replay
		evt("evt-audit-2", "memory.deprecate"), // boundary duplicate → skipped
		evt("evt-live-9", "memory.pin"),        // live
	}}
	router := newStreamTestRouter(t, fs)
	req := httptest.NewRequest("GET", "/v1/observability/events/stream?since_ts=1700000000", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	frames := parseSSEFrames(w.Body.String())
	ids := make([]string, len(frames))
	for i, f := range frames {
		ids[i] = f["id"]
	}
	want := []string{"evt-audit-1", "evt-audit-2", "evt-live-9"}
	if strings.Join(ids, ",") != strings.Join(want, ",") {
		t.Errorf("frame ids: got %v want %v (boundary dup must be deduped)", ids, want)
	}
	fs.mu.Lock()
	gotTS := fs.lastOpts.SinceTS
	fs.mu.Unlock()
	if gotTS != 1_700_000_000 {
		t.Errorf("since_ts not forwarded: got %d want 1700000000", gotTS)
	}
}

// TEST-26.2.4 / AC4: client disconnect (ctx cancel) ends the handler promptly
// and the streamer observes ctx.Done() (no goroutine leak / subscription release).
func TestEventsStream_ClientDisconnect_ReleasesStream(t *testing.T) {
	fs := &fakeStreamer{
		events:       []contractv1.ObservabilityEvent{evt("e1", "memory.pin")},
		blockForever: true,
	}
	router := newStreamTestRouter(t, fs)
	ctx, cancel := context.WithCancel(context.Background())
	req := httptest.NewRequest("GET", "/v1/observability/events/stream", nil).WithContext(ctx)
	w := httptest.NewRecorder()

	done := make(chan struct{})
	go func() {
		router.ServeHTTP(w, req)
		close(done)
	}()
	// Give the handler a moment to emit the first frame + block, then disconnect.
	time.Sleep(50 * time.Millisecond)
	cancel()
	select {
	case <-done:
		// handler returned on ctx cancel.
	case <-time.After(2 * time.Second):
		t.Fatal("handler did not return after client disconnect (goroutine leak)")
	}
	// Streamer observed cancellation (released the subscription).
	deadline := time.After(time.Second)
	for {
		fs.mu.Lock()
		ok := fs.gotCtxDone
		fs.mu.Unlock()
		if ok {
			break
		}
		select {
		case <-deadline:
			t.Fatal("streamer did not observe ctx.Done() (subscription not released)")
		case <-time.After(10 * time.Millisecond):
		}
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
