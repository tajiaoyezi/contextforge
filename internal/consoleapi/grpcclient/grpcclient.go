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
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/status"

	"github.com/tajiaoyezi/contextforge/internal/consoleapi"
	"github.com/tajiaoyezi/contextforge/internal/contractv1"
	pb "github.com/tajiaoyezi/contextforge/proto/contextforge/console_data_plane/v1"
)

// Client bundles 4 gRPC client wrappers + the underlying conn so Close()
// releases the channel cleanly.
type Client struct {
	conn      *grpc.ClientConn
	workspace consoleapi.WorkspaceClient
	job       consoleapi.JobClient
	search    consoleapi.SearchClient
	events    consoleapi.EventsClient
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

func (s *searchClient) Search(req contractv1.SearchRequest) (contractv1.SearchResult, contractv1.RetrievalTrace, error) {
	resp, err := s.c.Query(context.Background(), &pb.SearchRequest{
		Query:           req.Query,
		WorkspaceId:     req.WorkspaceID,
		AgentScope:      req.AgentScope,
		RetrievalMethod: req.RetrievalMethod,
		TopK:            int64(req.TopK),
		ConfigSnapshot:  string(req.ConfigSnapshot),
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
// Events wrapper (task-11.4 [SPEC-OWNER:task-11.4] long-poll wrap 30s/100 evt;
// task-11.2 only impl simple Recent(limit) using server-stream Subscribe).
// =====================================================================

type eventsClient struct{ c pb.EventsServiceClient }

// Recent dispatches to gRPC Subscribe stream, collects up to `limit` events
// within a short timeout (default 30s for long-poll behavior), then closes
// the stream. Real long-poll wrap (selectable wait param + ctx cancel
// propagation) lives in task-11.4 [SPEC-OWNER:task-11.4].
func (e *eventsClient) Recent(limit int) ([]contractv1.ObservabilityEvent, error) {
	if limit <= 0 {
		limit = 100
	}
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	stream, err := e.c.Subscribe(ctx, &pb.SubscribeEventsRequest{})
	if err != nil {
		return nil, mapGrpcErr(err)
	}
	batch := make([]contractv1.ObservabilityEvent, 0, limit)
	for len(batch) < limit {
		evt, err := stream.Recv()
		if err != nil {
			// Stream ended normally (server closed via drop(tx)) or ctx canceled
			// or transport error: return what we have. err == io.EOF expected on
			// normal close.
			break
		}
		batch = append(batch, protoToObservabilityEvent(evt))
	}
	return batch, nil
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
