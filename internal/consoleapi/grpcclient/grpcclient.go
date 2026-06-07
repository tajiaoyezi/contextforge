// Package grpcclient is the Go thin proxy bridge from Console REST handlers
// (`internal/consoleapi/`) to the Rust contextforge-core data plane gRPC
// services (`core/src/data_plane/`, task-11.1). It implements the four
// consoleapi.{Workspace,Job,Search,Events}Client interfaces by dispatching
// to tonic-generated gRPC client stubs (ADR-016 D3 thin protocol translator).
//
// Field semantics (ADR-016 D3 thin proxy enforcement):
//   - proto snake_case field names match Go contractv1 JSON tag 1:1; we do
//     NOT introduce any "business" mapping (no status advance, no field
//     defaulting, no timestamp generation) here — those belong in Rust.
//   - The only transformation kept on the Go side is "Unix int64 epoch ↔
//     time.Time" because Go's *time.Time / time.Time are the contractv1
//     types Console UI consumes; proto int64 epoch is the wire form.
//   - gRPC error code → consoleapi sentinel error mapping (NotFound →
//     ErrNotFound; FailedPrecondition → ErrJobTerminal; Unavailable →
//     ErrDataPlaneUnavailable; other → wrapped Internal err).
//
// Refs: ADR-016 §D3 / task-11.2 §6 AC1-3
package grpcclient

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log"
	"os"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/status"

	"github.com/tajiaoyezi/contextforge/internal/consoleapi"
	"github.com/tajiaoyezi/contextforge/internal/contractv1"
	pb "github.com/tajiaoyezi/contextforge/proto/contextforge/console_data_plane/v1"
)

// Client bundles 7 gRPC client wrappers + the underlying conn so Close()
// releases the channel cleanly.
type Client struct {
	conn      *grpc.ClientConn
	workspace consoleapi.WorkspaceClient
	job       consoleapi.JobClient
	search    consoleapi.SearchClient
	events    consoleapi.EventsClient
	memory    consoleapi.MemoryClient
	eval      consoleapi.EvalClient
	// task-15.6 (Phase 15 P2 #7 / ADR-020): detailed health probes.
	health consoleapi.HealthClient
}

// New dials the Rust data plane gRPC server (default 127.0.0.1:50551) and
// returns a *Client whose 4 wrapped interface fields satisfy
// consoleapi.{Workspace,Job,Search,Events}Client. The caller injects these
// into consoleapi.Deps + consoleapi.NewRouter.
//
// `addr` is a host:port string (no scheme); credentials default to
// insecure (loopback only — bearer auth is enforced at the REST layer per
// ADR-016 D3). Pass custom grpc.DialOption(s) via opts.
func New(ctx context.Context, addr string, opts ...grpc.DialOption) (*Client, error) {
	if len(opts) == 0 {
		opts = []grpc.DialOption{
			grpc.WithTransportCredentials(insecure.NewCredentials()),
		}
	}
	conn, err := grpc.DialContext(ctx, addr, opts...)
	if err != nil {
		return nil, fmt.Errorf("grpcclient.New(%s): %w", addr, err)
	}
	return &Client{
		conn:      conn,
		workspace: &workspaceClient{c: pb.NewWorkspaceServiceClient(conn)},
		job:       &jobClient{c: pb.NewJobServiceClient(conn)},
		search:    &searchClient{c: pb.NewSearchServiceClient(conn)},
		events:    &eventsClient{c: pb.NewEventsServiceClient(conn)},
		memory:    &memoryClient{c: pb.NewMemoryServiceClient(conn)},
		eval:      &evalClient{c: pb.NewEvalServiceClient(conn)},
		health:    &healthClient{c: pb.NewHealthServiceClient(conn)},
	}, nil
}

// Close releases the underlying gRPC connection.
func (c *Client) Close() error {
	if c.conn == nil {
		return nil
	}
	return c.conn.Close()
}

// Workspace returns the consoleapi.WorkspaceClient wrapper.
func (c *Client) Workspace() consoleapi.WorkspaceClient { return c.workspace }

// Job returns the consoleapi.JobClient wrapper.
func (c *Client) Job() consoleapi.JobClient { return c.job }

// Search returns the consoleapi.SearchClient wrapper.
func (c *Client) Search() consoleapi.SearchClient { return c.search }

// Events returns the consoleapi.EventsClient wrapper.
func (c *Client) Events() consoleapi.EventsClient { return c.events }

// EventsStream returns the consoleapi.EventsStreamer wrapper (task-26.2 / ADR-031
// D3). The same eventsClient backs both the long-poll Recent and the SSE Stream.
func (c *Client) EventsStream() consoleapi.EventsStreamer {
	es, _ := c.events.(consoleapi.EventsStreamer)
	return es
}

// Memory returns the consoleapi.MemoryClient wrapper.
func (c *Client) Memory() consoleapi.MemoryClient { return c.memory }

// Eval returns the consoleapi.EvalClient wrapper.
func (c *Client) Eval() consoleapi.EvalClient { return c.eval }

// Health returns the consoleapi.HealthClient wrapper (task-15.6 / Phase 15 P2 #7).
func (c *Client) Health() consoleapi.HealthClient { return c.health }

// =====================================================================
// Health wrapper (task-15.6 / Phase 15 P2 #7 / ADR-020).
// =====================================================================

type healthClient struct{ c pb.HealthServiceClient }

func (h *healthClient) GetDetailed() (contractv1.CoreHealth, error) {
	resp, err := h.c.GetDetailed(context.Background(), &pb.DetailedHealthRequest{})
	if err != nil {
		return contractv1.CoreHealth{}, mapGrpcErr(err)
	}
	comps := make(map[string]contractv1.ComponentHealth, len(resp.GetComponents()))
	for _, c := range resp.GetComponents() {
		latency := c.GetLatencyMs()
		var reason *string
		if r := c.GetErrorReason(); r != "" {
			reason = &r
		}
		comps[c.GetName()] = contractv1.ComponentHealth{
			Name:        c.GetName(),
			Status:      c.GetStatus(),
			LatencyMs:   &latency,
			ErrorReason: reason,
		}
	}
	total := resp.GetTotalLatencyMs()
	return contractv1.CoreHealth{
		Status:          resp.GetOverallStatus(),
		ContractVersion: contractv1.ContractVersion,
		Components:      comps,
		TotalLatencyMs:  &total,
	}, nil
}

// Ping issues a lightweight RPC to verify the data plane is reachable.
// Used by console-api-serve startup health-check.
func (c *Client) Ping(ctx context.Context) error {
	_, err := pb.NewWorkspaceServiceClient(c.conn).List(ctx, &pb.ListWorkspacesRequest{})
	return mapGrpcErr(err)
}

// mapGrpcErr maps a tonic Status to the consoleapi sentinel error set used by
// REST handlers (router.go translates these to HTTP 404 / 409 / 503 / 500).
func mapGrpcErr(err error) error {
	if err == nil {
		return nil
	}
	st, ok := status.FromError(err)
	if !ok {
		return fmt.Errorf("grpc non-status: %w", err)
	}
	switch st.Code() {
	case codes.NotFound:
		return consoleapi.ErrNotFound
	case codes.FailedPrecondition:
		return consoleapi.ErrJobTerminal
	case codes.Unavailable:
		return consoleapi.ErrDataPlaneUnavailable
	case codes.InvalidArgument:
		return fmt.Errorf("%w: %s", consoleapi.ErrInvalidRequest, st.Message())
	default:
		return fmt.Errorf("grpc %v: %s", st.Code(), st.Message())
	}
}

// =====================================================================
// Workspace wrapper
// =====================================================================

type workspaceClient struct{ c pb.WorkspaceServiceClient }

func (w *workspaceClient) Create(req contractv1.WorkspaceCreate) (contractv1.Workspace, error) {
	resp, err := w.c.Create(context.Background(), &pb.CreateWorkspaceRequest{
		WorkspaceId: req.Name, // workspace_id 与 name 同（v0.3 简化策略 — ADR-015 D2 workspace_id ↔ collection_id 1:1）
		Name:        req.Name,
		RootPath:    req.RootPath,
		Allowlist:   req.Allowlist,
		Denylist:    req.Denylist,
	})
	if err != nil {
		return contractv1.Workspace{}, mapGrpcErr(err)
	}
	return protoToWorkspace(resp), nil
}

func (w *workspaceClient) List() ([]contractv1.Workspace, error) {
	resp, err := w.c.List(context.Background(), &pb.ListWorkspacesRequest{})
	if err != nil {
		return nil, mapGrpcErr(err)
	}
	out := make([]contractv1.Workspace, 0, len(resp.Items))
	for _, item := range resp.Items {
		out = append(out, protoToWorkspace(item))
	}
	return out, nil
}

func (w *workspaceClient) Get(id string) (*contractv1.Workspace, error) {
	resp, err := w.c.Get(context.Background(), &pb.GetWorkspaceRequest{WorkspaceId: id})
	if err != nil {
		mapped := mapGrpcErr(err)
		if errors.Is(mapped, consoleapi.ErrNotFound) {
			return nil, nil // contractv1 convention: nil + nil = not found
		}
		return nil, mapped
	}
	ws := protoToWorkspace(resp)
	return &ws, nil
}

// Update wraps WorkspaceService.UpdateConfig (task-12.1 / ADR-017 D1 Wave 1).
func (w *workspaceClient) Update(id string, allowlist, denylist []string) (contractv1.Workspace, error) {
	if allowlist == nil {
		allowlist = []string{}
	}
	if denylist == nil {
		denylist = []string{}
	}
	resp, err := w.c.UpdateConfig(context.Background(), &pb.UpdateWorkspaceConfigRequest{
		WorkspaceId: id,
		Allowlist:   allowlist,
		Denylist:    denylist,
	})
	if err != nil {
		return contractv1.Workspace{}, mapGrpcErr(err)
	}
	return protoToWorkspace(resp), nil
}

// =====================================================================
// Job wrapper
// =====================================================================

type jobClient struct{ c pb.JobServiceClient }

func (j *jobClient) Enqueue(workspaceID, triggerSource string) (contractv1.IndexJob, error) {
	resp, err := j.c.Enqueue(context.Background(), &pb.EnqueueJobRequest{
		WorkspaceId:   workspaceID,
		TriggerSource: triggerSource,
	})
	if err != nil {
		return contractv1.IndexJob{}, mapGrpcErr(err)
	}
	return protoToIndexJob(resp), nil
}

func (j *jobClient) Get(jobID string) (*contractv1.IndexJob, error) {
	resp, err := j.c.Get(context.Background(), &pb.GetJobRequest{JobId: jobID})
	if err != nil {
		mapped := mapGrpcErr(err)
		if errors.Is(mapped, consoleapi.ErrNotFound) {
			return nil, nil
		}
		return nil, mapped
	}
	ij := protoToIndexJob(resp)
	return &ij, nil
}

func (j *jobClient) Cancel(jobID string) error {
	_, err := j.c.Cancel(context.Background(), &pb.CancelJobRequest{JobId: jobID})
	return mapGrpcErr(err)
}

// ListActive wraps JobService.List with status_filter = ["queued","running"]
// (task-12.1 / ADR-017 D1 Wave 1). Server-side filter is the Rust authority;
// Go side does not post-filter (ADR-016 D3 thin proxy).
func (j *jobClient) ListActive() ([]contractv1.IndexJob, error) {
	resp, err := j.c.List(context.Background(), &pb.ListJobsRequest{
		StatusFilter: []string{"queued", "running"},
	})
	if err != nil {
		return nil, mapGrpcErr(err)
	}
	out := make([]contractv1.IndexJob, 0, len(resp.Items))
	for _, item := range resp.Items {
		out = append(out, protoToIndexJob(item))
	}
	return out, nil
}

// =====================================================================
// Search wrapper
// =====================================================================

type searchClient struct{ c pb.SearchServiceClient }

// GetSourceChunk wraps SearchService.GetSourceChunk (task-12.2 / ADR-017 D1 Wave 2).
func (s *searchClient) GetSourceChunk(chunkID string) (contractv1.SourceChunk, error) {
	resp, err := s.c.GetSourceChunk(context.Background(), &pb.GetSourceChunkRequest{
		ChunkId: chunkID,
	})
	if err != nil {
		return contractv1.SourceChunk{}, mapGrpcErr(err)
	}
	return protoToSourceChunk(resp), nil
}

// GetSearchTrace wraps SearchService.GetSearchTrace (task-12.3 / ADR-017 D1 Wave 2).
func (s *searchClient) GetSearchTrace(queryID string) (contractv1.RetrievalTrace, error) {
	resp, err := s.c.GetSearchTrace(context.Background(), &pb.GetSearchTraceRequest{
		QueryId: queryID,
	})
	if err != nil {
		return contractv1.RetrievalTrace{}, mapGrpcErr(err)
	}
	return protoToRetrievalTrace(resp), nil
}

// ListQueries wraps SearchService.ListQueries (task-15.5 / Phase 15 P1 #5).
func (s *searchClient) ListQueries(limit int) ([]contractv1.QueryRecord, error) {
	resp, err := s.c.ListQueries(context.Background(), &pb.ListQueriesRequest{
		Limit: int32(limit),
	})
	if err != nil {
		return nil, mapGrpcErr(err)
	}
	out := make([]contractv1.QueryRecord, 0, len(resp.GetRecords()))
	for _, r := range resp.GetRecords() {
		out = append(out, contractv1.QueryRecord{
			QueryID:     r.GetQueryId(),
			Query:       r.GetQuery(),
			TsUnix:      r.GetTsUnix(),
			WorkspaceID: r.GetWorkspaceId(),
		})
	}
	return out, nil
}

// GetChunksStats wraps SearchService.GetChunksStats (task-15.3 / Phase 15 P1 #3).
func (s *searchClient) GetChunksStats(workspaceID string) (contractv1.ChunksStats, error) {
	resp, err := s.c.GetChunksStats(context.Background(), &pb.GetChunksStatsRequest{
		WorkspaceId: workspaceID,
	})
	if err != nil {
		return contractv1.ChunksStats{}, mapGrpcErr(err)
	}
	return contractv1.ChunksStats{
		Total:      resp.GetTotal(),
		TodayDelta: resp.GetTodayDelta(),
	}, nil
}

func (s *searchClient) Search(req contractv1.SearchRequest) (contractv1.SearchResult, contractv1.RetrievalTrace, error) {
	resp, err := s.c.Query(context.Background(), &pb.SearchRequest{
		Query:           req.Query,
		WorkspaceId:     req.WorkspaceID,
		AgentScope:      req.AgentScope,
		RetrievalMethod: req.RetrievalMethod,
		TopK:            int64(req.TopK),
		ConfigSnapshot:  string(req.ConfigSnapshot),
		Semantic:        req.Semantic, // task-20.1: forward opt-in semantic flag to core
		Hybrid:          req.Hybrid,   // task-39.2: forward opt-in hybrid flag to core
	})
	if err != nil {
		return contractv1.SearchResult{}, contractv1.RetrievalTrace{}, mapGrpcErr(err)
	}
	// task-11.4 [SPEC-OWNER:task-11.4] 真接 retriever 后 results 非空；v0.4 task-11.1
	// 占位返 empty results + minimal trace。Console HTTPAdapter v1.0 期望 {result:
	// SearchResult, trace: RetrievalTrace} 嵌套形态——本 wrapper 选第一个 result
	// (或 empty SearchResult) + non-nil trace 返回。
	var result contractv1.SearchResult
	if len(resp.Results) > 0 {
		result = protoToSearchResult(resp.Results[0])
	}
	var trace contractv1.RetrievalTrace
	if resp.Trace != nil {
		trace = protoToRetrievalTrace(resp.Trace)
	}
	return result, trace, nil
}

// =====================================================================
// Events wrapper (task-11.4 baseline; task-16.2 Phase 16 P4 #11 real long-poll).
// =====================================================================

type eventsClient struct{ c pb.EventsServiceClient }

// task-16.2 (Phase 16 P4 #11): drain timeout for phase-2 — once the first
// event arrives, give immediately-broadcast follow-up events ~drainTimeout
// to land before returning.
//
// task-26.3 (ADR-031 D5): now configurable via CONSOLE_EVENTS_DRAIN_TIMEOUT
// (Go duration string, e.g. "150ms"); conservative default 100ms keeps the
// task-16.2 two-phase long-poll behavior unchanged.
var eventsDrainTimeout = drainTimeoutFromEnv()

// drainTimeoutFromEnv reads CONSOLE_EVENTS_DRAIN_TIMEOUT (default 100ms; invalid
// / non-positive → default).
func drainTimeoutFromEnv() time.Duration {
	raw := os.Getenv("CONSOLE_EVENTS_DRAIN_TIMEOUT")
	if raw == "" {
		return 100 * time.Millisecond
	}
	d, err := time.ParseDuration(raw)
	if err != nil || d <= 0 {
		return 100 * time.Millisecond
	}
	return d
}

// isCtxDeadlineExceeded probes both the native ctx error and the gRPC
// status-code variant. gRPC wraps context.DeadlineExceeded into
// `status.Error(codes.DeadlineExceeded, ...)`, which `errors.Is(err,
// context.DeadlineExceeded)` does NOT match.
func isCtxDeadlineExceeded(err error) bool {
	if errors.Is(err, context.DeadlineExceeded) {
		return true
	}
	if st, ok := status.FromError(err); ok && st.Code() == codes.DeadlineExceeded {
		return true
	}
	return false
}

// Recent implements real long-poll over the gRPC server-stream `Subscribe`.
//
// Two phases:
//  1. Block up to `wait` waiting for the first event. If the deadline fires
//     before any event arrives, return `[]` + nil (Console expects 200 + []
//     on timeout — NOT 408). Non-DeadlineExceeded Recv errors are logged
//     (so operators see real transport failures) but still surface as `[]`
//     to keep the HTTP contract simple.
//  2. After the first event lands, open a SECOND Subscribe with a short
//     `eventsDrainTimeout` and pull events emitted in that follow-up window.
//     Note: a fresh broadcast::Receiver does NOT replay past events — this
//     phase catches events broadcast in the ~drainTimeout window AFTER the
//     phase-2 subscribe completes. Events emitted in the ~5ms gap between
//     phase-1 first-event return and phase-2 subscribe land time are missed
//     by both streams (acceptable for informational observability events;
//     §8 risk note).
//
// Test-path caveat: when the Rust EventsServer is configured WITHOUT an
// event_bus (e.g., data_plane unit tests), each Subscribe call emits a
// single placeholder `core.keepalive` event then closes the stream. Under
// v0.9 two-phase this can yield 2 keepalives where v0.8 yielded 1.
// Production via `serve_full` always has event_bus configured so this
// path is not hit; documented for completeness in task-16.2 §8.
//
// Refs: task-16.2 §3 / task-11.4 (broadcast EventBus baseline).
func (e *eventsClient) Recent(
	limit int,
	wait time.Duration,
) ([]contractv1.ObservabilityEvent, error) {
	if limit <= 0 {
		limit = 100
	}
	if wait <= 0 {
		wait = 30 * time.Second
	}

	// Phase 1: wait up to `wait` for the first event.
	ctx, cancel := context.WithTimeout(context.Background(), wait)
	defer cancel()
	stream, err := e.c.Subscribe(ctx, &pb.SubscribeEventsRequest{})
	if err != nil {
		return nil, mapGrpcErr(err)
	}
	first, err := stream.Recv()
	if err != nil {
		// ctx timeout / EOF / transport error → return empty. Distinguishing
		// DeadlineExceeded vs real error: log non-deadline + non-EOF cases so
		// operators can see actual failures (gRPC core down, etc.) via daemon
		// logs; /v1/health remains the user-visible signal. io.EOF is normal
		// (server closed empty stream) and intentionally silent.
		//
		// gRPC wraps `context.DeadlineExceeded` into `status.Error(codes.
		// DeadlineExceeded, ...)` — `errors.Is(err, context.DeadlineExceeded)`
		// alone misses the wrapped variant, so we also probe the gRPC status
		// code (caught via grpcclient_test.go::TestEventsClient_PhaseOneTimeout
		// where the bare `errors.Is` check produced spurious WARN logs).
		if !isCtxDeadlineExceeded(err) && !errors.Is(err, io.EOF) {
			log.Printf("WARN events Recv (phase-1) error: %v", err)
		}
		return []contractv1.ObservabilityEvent{}, nil
	}

	batch := make([]contractv1.ObservabilityEvent, 0, limit)
	batch = append(batch, protoToObservabilityEvent(first))

	// Phase 2: drain follow-up events emitted in the next ~drainTimeout.
	if len(batch) < limit {
		drainCtx, drainCancel := context.WithTimeout(context.Background(), eventsDrainTimeout)
		defer drainCancel()
		drainStream, dErr := e.c.Subscribe(drainCtx, &pb.SubscribeEventsRequest{})
		if dErr == nil {
			for len(batch) < limit {
				evt, rErr := drainStream.Recv()
				if rErr != nil {
					// Normal exit: ctx done, EOF, etc.
					break
				}
				batch = append(batch, protoToObservabilityEvent(evt))
			}
		}
		// drain Subscribe error is benign — caller already has the phase-1 event.
	}

	return batch, nil
}

// Stream implements the SSE streaming entry (task-26.2 / ADR-031 D3/D4). It
// opens a single `Subscribe` server-stream carrying the replay params
// (since_ts / last_event_id), and forwards each received event onto a buffered
// channel. The forwarding goroutine exits — closing the channel and releasing
// the gRPC stream — when ctx is cancelled (SSE client disconnect) or the
// upstream Recv ends (EOF / error). The Rust EventsServer replays missed
// memory state-op events from the audit log first (when since_ts > 0), then
// splices the live broadcast; live end-to-end verification is deferred
// (see task-26.2 §10 / [SPEC-DEFER:phase-future.sse-live-server-e2e]).
func (e *eventsClient) Stream(
	ctx context.Context,
	opts consoleapi.StreamOptions,
) (<-chan contractv1.ObservabilityEvent, error) {
	stream, err := e.c.Subscribe(ctx, &pb.SubscribeEventsRequest{
		SinceTs:     opts.SinceTS,
		LastEventId: opts.LastEventID,
	})
	if err != nil {
		return nil, mapGrpcErr(err)
	}
	ch := make(chan contractv1.ObservabilityEvent, 64)
	go func() {
		defer close(ch)
		for {
			evt, rErr := stream.Recv()
			if rErr != nil {
				// EOF / ctx cancelled / transport error → end the stream.
				return
			}
			select {
			case ch <- protoToObservabilityEvent(evt):
			case <-ctx.Done():
				return
			}
		}
	}()
	return ch, nil
}

// =====================================================================
// Proto ↔ contractv1 field conversion helpers (no business logic;
// snake_case → time.Time/optional pointer transformation only).
// =====================================================================

func protoToWorkspace(p *pb.Workspace) contractv1.Workspace {
	return contractv1.Workspace{
		WorkspaceID:    p.WorkspaceId,
		Name:           p.Name,
		RootPath:       p.RootPath,
		Status:         p.Status,
		ConfigSnapshot: json.RawMessage(p.ConfigSnapshot),
		CreatedAt:      time.Unix(p.CreatedAtUnix, 0).UTC(),
		UpdatedAt:      time.Unix(p.UpdatedAtUnix, 0).UTC(),
		Availability:   contractv1.FieldAvailability{Object: "Workspace"},
	}
}

func protoToIndexJob(p *pb.IndexJob) contractv1.IndexJob {
	out := contractv1.IndexJob{
		JobID:          p.JobId,
		WorkspaceID:    p.WorkspaceId,
		TriggerSource:  p.TriggerSource,
		Status:         p.Status,
		Stage:          p.Stage,
		ProcessedFiles: int(p.ProcessedFiles),
		TotalFiles:     int(p.TotalFiles),
		FailedFiles:    int(p.FailedFiles),
		SkippedFiles:   int(p.SkippedFiles),
		Availability:   contractv1.FieldAvailability{Object: "IndexJob"},
	}
	if p.ErrorMessage != nil {
		out.ErrorMessage = p.ErrorMessage
	}
	if p.StartedAtUnix != nil {
		t := time.Unix(*p.StartedAtUnix, 0).UTC()
		out.StartedAt = &t
	}
	if p.FinishedAtUnix != nil {
		t := time.Unix(*p.FinishedAtUnix, 0).UTC()
		out.FinishedAt = &t
	}
	if p.LastHeartbeatAtUnix != nil {
		t := time.Unix(*p.LastHeartbeatAtUnix, 0).UTC()
		out.LastHeartbeatAt = &t
	}
	return out
}

func protoToSearchResult(p *pb.SearchResultItem) contractv1.SearchResult {
	out := contractv1.SearchResult{
		ResultID:         p.ResultId,
		QueryID:          p.QueryId,
		WorkspaceID:      p.WorkspaceId,
		SourceFilePath:   p.SourceFilePath,
		SourceFileType:   p.SourceFileType,
		ChunkID:          p.ChunkId,
		ChunkTextPreview: p.ChunkTextPreview,
		LineStart:        int(p.LineStart),
		LineEnd:          int(p.LineEnd),
		Score:            p.Score,
		RankBeforeRerank: int(p.RankBeforeRerank),
		RetrievalMethod:  p.RetrievalMethod,
		VectorScore:      p.VectorScore,
		HybridScore:      p.HybridScore, // task-39.2: carry RRF fused score provenance
		Reason:           p.Reason,
		Availability:     contractv1.FieldAvailability{Object: "SearchResult"},
	}
	if p.RankAfterRerank != nil {
		r := int(*p.RankAfterRerank)
		out.RankAfterRerank = &r
	}
	if p.Citation != nil {
		out.Citation = contractv1.Citation{
			CitationID:     p.Citation.CitationId,
			SourceFilePath: p.Citation.SourceFilePath,
			ChunkID:        p.Citation.ChunkId,
			LineStart:      int(p.Citation.LineStart),
			LineEnd:        int(p.Citation.LineEnd),
			Confidence:     p.Citation.Confidence,
			Availability:   contractv1.FieldAvailability{Object: "Citation"},
		}
	}
	return out
}

// protoToSourceChunk maps proto.SourceChunk → contractv1.SourceChunk (task-12.2).
// Field shapes match 1:1 (snake_case proto ↔ Go json tag); int64 line / offset
// fields downcast to int. Availability marker is set so FieldAvailability.Complete()
// downstream returns true.
func protoToSourceChunk(p *pb.SourceChunk) contractv1.SourceChunk {
	return contractv1.SourceChunk{
		ChunkID:          p.ChunkId,
		WorkspaceID:      p.WorkspaceId,
		SourceFilePath:   p.SourceFilePath,
		LineStart:        int(p.LineStart),
		LineEnd:          int(p.LineEnd),
		ChunkTextPreview: p.ChunkTextPreview,
		ChunkOffsetStart: int(p.ChunkOffsetStart),
		ChunkOffsetEnd:   int(p.ChunkOffsetEnd),
		RedactionStatus:  p.RedactionStatus,
		Availability:     contractv1.FieldAvailability{Object: "SourceChunk"},
	}
}

func protoToRetrievalTrace(p *pb.RetrievalTrace) contractv1.RetrievalTrace {
	out := contractv1.RetrievalTrace{
		TraceID:                  p.TraceId,
		Query:                    p.Query,
		ExpandedQuery:            p.ExpandedQuery,
		CandidateGenerationSteps: p.CandidateGenerationSteps,
		LexicalCandidatesCount:   int(p.LexicalCandidatesCount),
		VectorCandidatesCount:    int(p.VectorCandidatesCount),
		RerankSteps:              p.RerankSteps,
		ScopeFilterResult:        p.ScopeFilterResult,
		FinalContextCount:        int(p.FinalContextCount),
		Availability:             contractv1.FieldAvailability{Object: "RetrievalTrace"},
	}
	if out.CandidateGenerationSteps == nil {
		out.CandidateGenerationSteps = []string{}
	}
	if out.RerankSteps == nil {
		out.RerankSteps = []string{}
	}
	return out
}

// =====================================================================
// Memory wrapper (task-13.2 / ADR-017 D1 Wave 3)
// =====================================================================

type memoryClient struct{ c pb.MemoryServiceClient }

func (m *memoryClient) List(filter consoleapi.MemoryListFilter) ([]contractv1.MemoryItem, error) {
	resp, err := m.c.List(context.Background(), &pb.ListMemoryRequest{
		AgentId:            filter.AgentID,
		Scope:              filter.Scope,
		Namespace:          filter.Namespace,
		IncludeSoftDeleted: filter.IncludeSoftDeleted,
	})
	if err != nil {
		return nil, mapGrpcErr(err)
	}
	out := make([]contractv1.MemoryItem, 0, len(resp.Items))
	for _, item := range resp.Items {
		out = append(out, protoToMemoryItem(item))
	}
	return out, nil
}

func (m *memoryClient) Get(id string) (*contractv1.MemoryItem, error) {
	resp, err := m.c.Get(context.Background(), &pb.GetMemoryRequest{MemoryId: id})
	if err != nil {
		mapped := mapGrpcErr(err)
		if errors.Is(mapped, consoleapi.ErrNotFound) {
			return nil, nil
		}
		return nil, mapped
	}
	item := protoToMemoryItem(resp)
	return &item, nil
}

func (m *memoryClient) Pin(id string, pin bool, actor string) error {
	// task-40.1: forward the calling actor (X-Actor header → handler → here) to the data plane.
	// Empty actor → server falls back to "console-api" (byte-equivalent default, ADR-004).
	_, err := m.c.Pin(context.Background(), &pb.PinMemoryRequest{MemoryId: id, Pin: pin, Actor: actor})
	return mapGrpcErr(err)
}

func (m *memoryClient) Deprecate(id string) error {
	_, err := m.c.Deprecate(context.Background(), &pb.DeprecateMemoryRequest{MemoryId: id})
	return mapGrpcErr(err)
}

func (m *memoryClient) SoftDelete(id string) error {
	_, err := m.c.SoftDelete(context.Background(), &pb.SoftDeleteMemoryRequest{MemoryId: id})
	return mapGrpcErr(err)
}

// task-27.2 (ADR-032 D2): explicit Unpin RPC (non-destructive).
func (m *memoryClient) Unpin(id string) error {
	_, err := m.c.Unpin(context.Background(), &pb.UnpinMemoryRequest{MemoryId: id})
	return mapGrpcErr(err)
}

// task-27.2 (ADR-032 D2): hard-delete RPC (physical row removal; destructive).
func (m *memoryClient) HardDelete(id string) error {
	_, err := m.c.HardDelete(context.Background(), &pb.HardDeleteMemoryRequest{MemoryId: id})
	return mapGrpcErr(err)
}

func protoToMemoryItem(p *pb.MemoryItem) contractv1.MemoryItem {
	return contractv1.MemoryItem{
		MemoryID:       p.MemoryId,
		AgentScope:     p.AgentScope,
		ContentPreview: p.ContentPreview,
		SourceType:     p.SourceType,
		SourceRef:      p.SourceRef,
		CreatedAt:      time.Unix(p.CreatedAtUnix, 0).UTC(),
		UpdatedAt:      time.Unix(p.UpdatedAtUnix, 0).UTC(),
		HitCount:       int(p.HitCount),
		Status:         p.Status,
		IsPinned:       p.IsPinned,
		PinnedBy:       p.PinnedBy,     // task-27.1 (ADR-032 D1) add-only
		PinnedAtUnix:   p.PinnedAtUnix, // task-27.1 (ADR-032 D1) add-only
		Availability:   contractv1.FieldAvailability{Object: "MemoryItem"},
	}
}

// =====================================================================
// Eval wrapper (task-14.2 / ADR-017 D1 Wave 4)
// =====================================================================

type evalClient struct{ c pb.EvalServiceClient }

func (e *evalClient) Create(req contractv1.EvalRunCreate) (contractv1.EvalRun, error) {
	cfg, _ := json.Marshal(req.ConfigSnapshot)
	// Generate unique id Go-side (matches task-14.1 contract: caller-provided id).
	id := fmt.Sprintf("eval-%d", time.Now().UnixNano())
	resp, err := e.c.Create(context.Background(), &pb.CreateEvalRunRequest{
		EvalRunId:          id,
		WorkspaceId:        req.WorkspaceID,
		ConfigSnapshotJson: string(cfg),
		DatasetRef:         req.DatasetRef,
	})
	if err != nil {
		return contractv1.EvalRun{}, mapGrpcErr(err)
	}
	return protoToEvalRun(resp), nil
}

func (e *evalClient) Get(id string) (*contractv1.EvalRun, error) {
	resp, err := e.c.Get(context.Background(), &pb.GetEvalRunRequest{EvalRunId: id})
	if err != nil {
		mapped := mapGrpcErr(err)
		if errors.Is(mapped, consoleapi.ErrNotFound) {
			return nil, nil
		}
		return nil, mapped
	}
	run := protoToEvalRun(resp)
	return &run, nil
}

// List wraps EvalService.List (task-15.4 / Phase 15 P1 #4).
func (e *evalClient) List(filter contractv1.ListEvalRunsFilter) ([]contractv1.EvalRun, error) {
	resp, err := e.c.List(context.Background(), &pb.ListEvalRunsRequest{
		WorkspaceId: filter.WorkspaceID,
		Status:      filter.Status,
		Limit:       filter.Limit,
	})
	if err != nil {
		return nil, mapGrpcErr(err)
	}
	out := make([]contractv1.EvalRun, 0, len(resp.GetRuns()))
	for _, r := range resp.GetRuns() {
		out = append(out, protoToEvalRun(r))
	}
	return out, nil
}

func (e *evalClient) UpdateProgress(id, status string, metrics map[string]float64,
	caseResults []contractv1.CaseResult, errorMessage string) error {
	metricsJSON := "{}"
	if metrics != nil {
		if b, err := json.Marshal(metrics); err == nil {
			metricsJSON = string(b)
		}
	}
	cases := make([]*pb.CaseResult, 0, len(caseResults))
	for _, c := range caseResults {
		cases = append(cases, &pb.CaseResult{
			CaseId:         c.CaseID,
			Query:          c.Query,
			ExpectedChunks: c.ExpectedChunks,
			ActualChunks:   c.ActualChunks,
			Score:          c.Score,
			Passed:         c.Passed,
		})
	}
	_, err := e.c.UpdateProgress(context.Background(), &pb.UpdateEvalRunProgressRequest{
		EvalRunId:    id,
		Status:       status,
		MetricsJson:  metricsJSON,
		CaseResults:  cases,
		ErrorMessage: errorMessage,
	})
	return mapGrpcErr(err)
}

func protoToEvalRun(p *pb.EvalRun) contractv1.EvalRun {
	metrics := map[string]float64{}
	if p.MetricsJson != "" {
		_ = json.Unmarshal([]byte(p.MetricsJson), &metrics)
	}
	cases := make([]contractv1.CaseResult, 0, len(p.CaseResults))
	for _, c := range p.CaseResults {
		cases = append(cases, contractv1.CaseResult{
			CaseID:         c.CaseId,
			Query:          c.Query,
			ExpectedChunks: c.ExpectedChunks,
			ActualChunks:   c.ActualChunks,
			Score:          c.Score,
			Passed:         c.Passed,
		})
	}
	out := contractv1.EvalRun{
		EvalRunID:      p.EvalRunId,
		WorkspaceID:    p.WorkspaceId,
		Status:         p.Status,
		ConfigSnapshot: json.RawMessage(p.ConfigSnapshotJson),
		StartedAt:      time.Unix(p.StartedAtUnix, 0).UTC(),
		Metrics:        metrics,
		CaseResults:    cases,
		SchemaVersion:  p.SchemaVersion,
		Availability:   contractv1.FieldAvailability{Object: "EvalRun"},
	}
	if p.FinishedAtUnix != nil {
		t := time.Unix(*p.FinishedAtUnix, 0).UTC()
		out.FinishedAt = &t
	}
	return out
}

func protoToObservabilityEvent(p *pb.ObservabilityEvent) contractv1.ObservabilityEvent {
	out := contractv1.ObservabilityEvent{
		EventID:      p.EventId,
		EventType:    p.EventType,
		Severity:     p.Severity,
		Source:       p.Source,
		Message:      p.Message,
		Timestamp:    time.Unix(p.TsUnix, 0).UTC(),
		TraceID:      p.TraceId,
		JobID:        p.JobId,
		Availability: contractv1.FieldAvailability{Object: "ObservabilityEvent"},
	}
	return out
}
