package grpcclient

import (
	"context"
	"errors"
	"net"
	"net/http"
	"runtime"
	"strings"
	"sync"
	"testing"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/status"

	"github.com/tajiaoyezi/contextforge/internal/consoleapi"
	"github.com/tajiaoyezi/contextforge/internal/contractv1"
	pb "github.com/tajiaoyezi/contextforge/proto/contextforge/console_data_plane/v1"
)

// TestClientImplementsDeps is a compile-time check: the 4 wrappers from
// grpcclient.New must satisfy the consoleapi.{Workspace,Job,Search,Events}Client
// interfaces (task-11.2 §6 AC1).
func TestClientImplementsDeps(t *testing.T) {
	// Compile-time: each wrapper field type assigns to the consoleapi
	// interface variable. Failure here = signature drift.
	c := &Client{
		workspace: &workspaceClient{},
		job:       &jobClient{},
		search:    &searchClient{},
		events:    &eventsClient{},
	}
	var _ consoleapi.WorkspaceClient = c.workspace
	var _ consoleapi.JobClient = c.job
	var _ consoleapi.SearchClient = c.search
	var _ consoleapi.EventsClient = c.events
	if c.Workspace() == nil || c.Job() == nil || c.Search() == nil || c.Events() == nil {
		t.Fatal("Client accessors returned nil")
	}
}

// TestMapGrpcErr_NotFound: gRPC NotFound → consoleapi.ErrNotFound.
func TestMapGrpcErr_NotFound(t *testing.T) {
	src := status.Error(codes.NotFound, "missing")
	mapped := mapGrpcErr(src)
	if !errors.Is(mapped, consoleapi.ErrNotFound) {
		t.Fatalf("expected ErrNotFound, got %v", mapped)
	}
}

// TestMapGrpcErr_FailedPrecondition: → ErrJobTerminal.
func TestMapGrpcErr_FailedPrecondition(t *testing.T) {
	src := status.Error(codes.FailedPrecondition, "job already terminal")
	mapped := mapGrpcErr(src)
	if !errors.Is(mapped, consoleapi.ErrJobTerminal) {
		t.Fatalf("expected ErrJobTerminal, got %v", mapped)
	}
}

// TestMapGrpcErr_Unavailable: → ErrDataPlaneUnavailable.
func TestMapGrpcErr_Unavailable(t *testing.T) {
	src := status.Error(codes.Unavailable, "down")
	mapped := mapGrpcErr(src)
	if !errors.Is(mapped, consoleapi.ErrDataPlaneUnavailable) {
		t.Fatalf("expected ErrDataPlaneUnavailable, got %v", mapped)
	}
}

// TestDialFailedReturnsErr: dialing a non-listening address returns a non-nil
// error (note: grpc-go DialContext is lazy by default; we use WithBlock +
// short timeout to force eager connect).
func TestDialFailedReturnsErr(t *testing.T) {
	// 127.0.0.1:1 is conventionally "always closed" on most systems.
	ctx, cancel := context.WithTimeout(context.Background(), 1*time.Second)
	defer cancel()
	cli, err := New(ctx, "127.0.0.1:1",
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err == nil {
		_ = cli.Close()
		t.Fatal("expected dial error on closed port; got nil")
	}
	if !strings.Contains(err.Error(), "127.0.0.1:1") {
		t.Logf("expected addr in error; got %v", err)
	}
}

// TestProtoToWorkspace_NullableTime: int64 unix epoch → time.Time UTC.
func TestProtoToWorkspace_NullableTime(t *testing.T) {
	p := &pb.Workspace{
		WorkspaceId:    "ws-x",
		Name:           "x",
		RootPath:       "/tmp",
		Status:         "ready",
		ConfigSnapshot: "{}",
		CreatedAtUnix:  1700000000,
		UpdatedAtUnix:  1700000001,
	}
	w := protoToWorkspace(p)
	if w.WorkspaceID != "ws-x" {
		t.Errorf("WorkspaceID drift")
	}
	if w.CreatedAt.Unix() != 1700000000 {
		t.Errorf("CreatedAt drift: %d", w.CreatedAt.Unix())
	}
	if w.CreatedAt.Location().String() != "UTC" {
		t.Errorf("CreatedAt not UTC: %s", w.CreatedAt.Location())
	}
}

// TestProtoToIndexJob_NullablePointers: optional int64 → *time.Time;
// optional string → *string; preserves "not set" → nil.
func TestProtoToIndexJob_NullablePointers(t *testing.T) {
	p := &pb.IndexJob{
		JobId:       "j1",
		WorkspaceId: "ws",
		Status:      "queued",
		// 显式不设 ErrorMessage / StartedAtUnix / FinishedAtUnix / LastHeartbeatAtUnix
	}
	j := protoToIndexJob(p)
	if j.ErrorMessage != nil {
		t.Errorf("expected nil ErrorMessage; got %v", j.ErrorMessage)
	}
	if j.StartedAt != nil {
		t.Errorf("expected nil StartedAt")
	}
	if j.FinishedAt != nil {
		t.Errorf("expected nil FinishedAt")
	}
	if j.LastHeartbeatAt != nil {
		t.Errorf("expected nil LastHeartbeatAt")
	}

	// Now with values set
	startTs := int64(1700000000)
	finishTs := int64(1700000050)
	hbTs := int64(1700000025)
	msg := "oops"
	p2 := &pb.IndexJob{
		JobId:               "j2",
		ErrorMessage:        &msg,
		StartedAtUnix:       &startTs,
		FinishedAtUnix:      &finishTs,
		LastHeartbeatAtUnix: &hbTs,
	}
	j2 := protoToIndexJob(p2)
	if j2.ErrorMessage == nil || *j2.ErrorMessage != "oops" {
		t.Errorf("ErrorMessage drift")
	}
	if j2.StartedAt == nil || j2.StartedAt.Unix() != startTs {
		t.Errorf("StartedAt drift")
	}
	if j2.FinishedAt == nil || j2.FinishedAt.Unix() != finishTs {
		t.Errorf("FinishedAt drift")
	}
	if j2.LastHeartbeatAt == nil || j2.LastHeartbeatAt.Unix() != hbTs {
		t.Errorf("LastHeartbeatAt drift")
	}
}

// =====================================================================
// Integration-style tests (spawn an in-process tonic Server via Go gRPC
// fake server stub — Go side does not have access to Rust tonic, so we
// reuse the generated protoc-gen-go-grpc Server interface + a minimal
// fake server that returns canned responses. This exercises the full Go
// wire (grpcclient → gRPC → fake server → grpcclient → consoleapi
// interface) without spawning the actual Rust daemon binary.
// =====================================================================

// fakeWorkspaceServer is a minimal pb.WorkspaceServiceServer that returns
// canned responses to verify grpcclient wire path. (Rust daemon spawn
// integration is task-11.2 §6 AC5 / TestRESTEndpoints_E2E_GrpcBacked in
// internal/consoleapi/e2e_test.go.)
type fakeWorkspaceServer struct {
	pb.UnimplementedWorkspaceServiceServer
	createResp *pb.Workspace
	getResp    *pb.Workspace
	getErr     error
	listResp   *pb.ListWorkspacesResponse
}

func (f *fakeWorkspaceServer) Create(_ context.Context, req *pb.CreateWorkspaceRequest) (*pb.Workspace, error) {
	if f.createResp != nil {
		return f.createResp, nil
	}
	return &pb.Workspace{
		WorkspaceId:   req.WorkspaceId,
		Name:          req.Name,
		RootPath:      req.RootPath,
		Status:        "ready",
		CreatedAtUnix: 1700000000,
		UpdatedAtUnix: 1700000000,
	}, nil
}

func (f *fakeWorkspaceServer) Get(_ context.Context, req *pb.GetWorkspaceRequest) (*pb.Workspace, error) {
	if f.getErr != nil {
		return nil, f.getErr
	}
	if f.getResp != nil {
		return f.getResp, nil
	}
	return nil, status.Error(codes.NotFound, "not found: "+req.WorkspaceId)
}

func (f *fakeWorkspaceServer) List(_ context.Context, _ *pb.ListWorkspacesRequest) (*pb.ListWorkspacesResponse, error) {
	if f.listResp != nil {
		return f.listResp, nil
	}
	return &pb.ListWorkspacesResponse{}, nil
}

// spawnFakeServer starts a gRPC server on 127.0.0.1:0 with the supplied
// service registrations. Returns the bound addr + a stop fn.
func spawnFakeServer(t *testing.T, register func(s *grpc.Server)) (string, func()) {
	t.Helper()
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("net.Listen: %v", err)
	}
	srv := grpc.NewServer()
	register(srv)
	done := make(chan struct{})
	go func() {
		_ = srv.Serve(lis)
		close(done)
	}()
	return lis.Addr().String(), func() {
		srv.GracefulStop()
		select {
		case <-done:
		case <-time.After(3 * time.Second):
		}
	}
}

func TestWorkspaceClient_CreateGetList_ViaGRPC(t *testing.T) {
	fake := &fakeWorkspaceServer{}
	addr, stop := spawnFakeServer(t, func(s *grpc.Server) {
		pb.RegisterWorkspaceServiceServer(s, fake)
	})
	defer stop()

	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	cli, err := New(ctx, addr)
	if err != nil {
		t.Fatalf("New: %v", err)
	}
	defer func() { _ = cli.Close() }()

	// Create
	ws, err := cli.Workspace().Create(contractv1.WorkspaceCreate{
		Name:     "test-ws",
		RootPath: "/tmp/cf",
	})
	if err != nil {
		t.Fatalf("Create: %v", err)
	}
	if ws.WorkspaceID == "" || ws.Status != "ready" {
		t.Errorf("Create resp drift: %+v", ws)
	}

	// Get unknown → ErrNotFound (nil, nil per contractv1 convention)
	got, err := cli.Workspace().Get("ws-missing")
	if err != nil {
		t.Errorf("expected nil err for not-found; got %v", err)
	}
	if got != nil {
		t.Errorf("expected nil workspace for not-found; got %v", got)
	}

	// List
	list, err := cli.Workspace().List()
	if err != nil {
		t.Fatalf("List: %v", err)
	}
	if list == nil {
		t.Errorf("List returned nil slice (want empty slice)")
	}
}

// =====================================================================
// Ping smoke (verifies List dispatch end-to-end).
// =====================================================================

func TestPingSucceedsAgainstFakeServer(t *testing.T) {
	fake := &fakeWorkspaceServer{}
	addr, stop := spawnFakeServer(t, func(s *grpc.Server) {
		pb.RegisterWorkspaceServiceServer(s, fake)
	})
	defer stop()

	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	cli, err := New(ctx, addr)
	if err != nil {
		t.Fatalf("New: %v", err)
	}
	defer func() { _ = cli.Close() }()

	pingCtx, pingCancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer pingCancel()
	if err := cli.Ping(pingCtx); err != nil {
		t.Fatalf("Ping failed: %v", err)
	}
}

// =====================================================================
// task-12.1 (ADR-017 D1 Wave 1) — UpdateConfig + ListActive wire coverage.
// =====================================================================

// fakeWorkspaceServer extension: capture UpdateConfig calls + return canned.
type fakeWorkspaceUpdateServer struct {
	fakeWorkspaceServer
	gotID        string
	gotAllowlist []string
	gotDenylist  []string
	updateErr    error
}

func (f *fakeWorkspaceUpdateServer) UpdateConfig(_ context.Context, req *pb.UpdateWorkspaceConfigRequest) (*pb.Workspace, error) {
	if f.updateErr != nil {
		return nil, f.updateErr
	}
	f.gotID = req.WorkspaceId
	f.gotAllowlist = req.Allowlist
	f.gotDenylist = req.Denylist
	return &pb.Workspace{
		WorkspaceId:   req.WorkspaceId,
		Name:          "updated",
		RootPath:      "/tmp/x",
		Status:        "ready",
		Allowlist:     req.Allowlist,
		Denylist:      req.Denylist,
		CreatedAtUnix: 1700000000,
		UpdatedAtUnix: 1700000100,
	}, nil
}

func TestGrpcClient_WorkspaceUpdate_WiresFields(t *testing.T) {
	fake := &fakeWorkspaceUpdateServer{}
	addr, stop := spawnFakeServer(t, func(s *grpc.Server) {
		pb.RegisterWorkspaceServiceServer(s, fake)
	})
	defer stop()
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	cli, err := New(ctx, addr)
	if err != nil {
		t.Fatalf("New: %v", err)
	}
	defer func() { _ = cli.Close() }()
	ws, err := cli.Workspace().Update("ws-x", []string{"src/**"}, []string{"node_modules/**"})
	if err != nil {
		t.Fatalf("Update: %v", err)
	}
	if ws.WorkspaceID != "ws-x" {
		t.Errorf("WorkspaceID drift: %s", ws.WorkspaceID)
	}
	if fake.gotID != "ws-x" || len(fake.gotAllowlist) != 1 || fake.gotAllowlist[0] != "src/**" {
		t.Errorf("server-side request drift: id=%s allow=%v deny=%v", fake.gotID, fake.gotAllowlist, fake.gotDenylist)
	}
}

func TestGrpcClient_WorkspaceUpdate_Maps_NotFound(t *testing.T) {
	fake := &fakeWorkspaceUpdateServer{updateErr: status.Error(codes.NotFound, "missing")}
	addr, stop := spawnFakeServer(t, func(s *grpc.Server) {
		pb.RegisterWorkspaceServiceServer(s, fake)
	})
	defer stop()
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	cli, _ := New(ctx, addr)
	defer func() { _ = cli.Close() }()
	_, err := cli.Workspace().Update("missing", nil, nil)
	if !errors.Is(err, consoleapi.ErrNotFound) {
		t.Errorf("expected ErrNotFound; got %v", err)
	}
}

// fakeJobListServer captures ListJobs requests + returns canned IndexJobs.
type fakeJobListServer struct {
	pb.UnimplementedJobServiceServer
	gotFilter []string
	listErr   error
}

func (f *fakeJobListServer) List(_ context.Context, req *pb.ListJobsRequest) (*pb.ListJobsResponse, error) {
	if f.listErr != nil {
		return nil, f.listErr
	}
	f.gotFilter = append([]string{}, req.StatusFilter...)
	return &pb.ListJobsResponse{
		Items: []*pb.IndexJob{
			{JobId: "job-a", WorkspaceId: "ws", Status: "queued"},
			{JobId: "job-b", WorkspaceId: "ws", Status: "running"},
		},
	}, nil
}

func TestGrpcClient_JobListActive_FiltersAndMaps(t *testing.T) {
	fake := &fakeJobListServer{}
	addr, stop := spawnFakeServer(t, func(s *grpc.Server) {
		pb.RegisterJobServiceServer(s, fake)
	})
	defer stop()
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	cli, _ := New(ctx, addr)
	defer func() { _ = cli.Close() }()
	jobs, err := cli.Job().ListActive()
	if err != nil {
		t.Fatalf("ListActive: %v", err)
	}
	if len(jobs) != 2 {
		t.Fatalf("expected 2 jobs; got %d", len(jobs))
	}
	wantFilter := map[string]bool{"queued": true, "running": true}
	if len(fake.gotFilter) != 2 || !wantFilter[fake.gotFilter[0]] || !wantFilter[fake.gotFilter[1]] {
		t.Errorf("expected filter ['queued','running']; got %v", fake.gotFilter)
	}
}

func TestGrpcClient_JobListActive_Maps_Unavailable(t *testing.T) {
	fake := &fakeJobListServer{listErr: status.Error(codes.Unavailable, "down")}
	addr, stop := spawnFakeServer(t, func(s *grpc.Server) {
		pb.RegisterJobServiceServer(s, fake)
	})
	defer stop()
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	cli, _ := New(ctx, addr)
	defer func() { _ = cli.Close() }()
	_, err := cli.Job().ListActive()
	if !errors.Is(err, consoleapi.ErrDataPlaneUnavailable) {
		t.Errorf("expected ErrDataPlaneUnavailable; got %v", err)
	}
}

// =====================================================================
// task-12.2 (ADR-017 D1 Wave 2) — GetSourceChunk wire coverage.
// =====================================================================

type fakeSearchServer struct {
	pb.UnimplementedSearchServiceServer
	chunkResp *pb.SourceChunk
	chunkErr  error
}

func (f *fakeSearchServer) GetSourceChunk(_ context.Context, req *pb.GetSourceChunkRequest) (*pb.SourceChunk, error) {
	if f.chunkErr != nil {
		return nil, f.chunkErr
	}
	if f.chunkResp != nil {
		return f.chunkResp, nil
	}
	return &pb.SourceChunk{
		ChunkId:          req.ChunkId,
		WorkspaceId:      "ws-x",
		SourceFilePath:   "/tmp/foo.md",
		LineStart:        1,
		LineEnd:          10,
		ChunkTextPreview: "preview text",
		RedactionStatus:  "applied",
	}, nil
}

// fakeQueryServer (task-20.1) captures the inbound SearchRequest.Semantic so the
// grpcclient passthrough can be asserted.
type fakeQueryServer struct {
	pb.UnimplementedSearchServiceServer
	mu     sync.Mutex
	gotSem bool
}

func (f *fakeQueryServer) Query(_ context.Context, req *pb.SearchRequest) (*pb.SearchResponse, error) {
	f.mu.Lock()
	f.gotSem = req.Semantic
	f.mu.Unlock()
	return &pb.SearchResponse{
		Results: []*pb.SearchResultItem{},
		Trace:   &pb.RetrievalTrace{TraceId: "t", Query: req.Query},
	}, nil
}

// TestTask201_GrpcClient_Search_ForwardsSemantic — task-20.1 §6 AC3: searchClient
// .Search maps contractv1.SearchRequest.Semantic onto the gRPC pb.SearchRequest
// .Semantic field (both true and false reach the core SearchService.Query).
func TestTask201_GrpcClient_Search_ForwardsSemantic(t *testing.T) {
	for _, sem := range []bool{true, false} {
		fake := &fakeQueryServer{}
		addr, stop := spawnFakeServer(t, func(s *grpc.Server) {
			pb.RegisterSearchServiceServer(s, fake)
		})
		ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
		cli, _ := New(ctx, addr)
		_, _, err := cli.Search().Search(contractv1.SearchRequest{Query: "q", WorkspaceID: "w", Semantic: sem})
		if err != nil {
			t.Fatalf("Search(semantic=%v): %v", sem, err)
		}
		fake.mu.Lock()
		got := fake.gotSem
		fake.mu.Unlock()
		if got != sem {
			t.Errorf("forwarded pb.SearchRequest.Semantic = %v, want %v", got, sem)
		}
		_ = cli.Close()
		cancel()
		stop()
	}
}

func TestGrpcClient_GetSourceChunk_MapsFields(t *testing.T) {
	fake := &fakeSearchServer{}
	addr, stop := spawnFakeServer(t, func(s *grpc.Server) {
		pb.RegisterSearchServiceServer(s, fake)
	})
	defer stop()
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	cli, _ := New(ctx, addr)
	defer func() { _ = cli.Close() }()
	chunk, err := cli.Search().GetSourceChunk("chk_abc_0")
	if err != nil {
		t.Fatalf("GetSourceChunk: %v", err)
	}
	if chunk.ChunkID != "chk_abc_0" || chunk.WorkspaceID != "ws-x" {
		t.Errorf("field drift: %+v", chunk)
	}
	if chunk.LineStart != 1 || chunk.LineEnd != 10 {
		t.Errorf("line range drift: %+v", chunk)
	}
	if chunk.ChunkTextPreview != "preview text" {
		t.Errorf("preview drift: %+v", chunk)
	}
}

func TestGrpcClient_GetSearchTrace_MapsFields(t *testing.T) {
	fake := &fakeSearchTraceServer{}
	addr, stop := spawnFakeServer(t, func(s *grpc.Server) {
		pb.RegisterSearchServiceServer(s, fake)
	})
	defer stop()
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	cli, _ := New(ctx, addr)
	defer func() { _ = cli.Close() }()
	trace, err := cli.Search().GetSearchTrace("qry-abc")
	if err != nil {
		t.Fatalf("GetSearchTrace: %v", err)
	}
	if trace.TraceID == "" || trace.Query != "hello" {
		t.Errorf("field drift: %+v", trace)
	}
}

func TestGrpcClient_GetSearchTrace_Maps_NotFound(t *testing.T) {
	fake := &fakeSearchTraceServer{traceErr: status.Error(codes.NotFound, "missing")}
	addr, stop := spawnFakeServer(t, func(s *grpc.Server) {
		pb.RegisterSearchServiceServer(s, fake)
	})
	defer stop()
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	cli, _ := New(ctx, addr)
	defer func() { _ = cli.Close() }()
	_, err := cli.Search().GetSearchTrace("qry-missing")
	if !errors.Is(err, consoleapi.ErrNotFound) {
		t.Errorf("expected ErrNotFound; got %v", err)
	}
}

type fakeSearchTraceServer struct {
	pb.UnimplementedSearchServiceServer
	traceErr error
}

func (f *fakeSearchTraceServer) GetSearchTrace(_ context.Context, req *pb.GetSearchTraceRequest) (*pb.RetrievalTrace, error) {
	if f.traceErr != nil {
		return nil, f.traceErr
	}
	return &pb.RetrievalTrace{
		TraceId:                  "trace-xyz",
		Query:                    "hello",
		CandidateGenerationSteps: []string{"bm25"},
		LexicalCandidatesCount:   0,
		ScopeFilterResult:        "no-op",
		FinalContextCount:        0,
	}, nil
}

// TEST-32.3.1 — protoToSearchResult carries the add-only vector_score provenance: a semantic hit's
// real vector_score maps through to the contract; a BM25 hit's vector_score stays 0. Proves the
// console data-plane SearchResultItem.vector_score (add-only field 16) is plumbed end-to-end (the
// real value, not inferred from score + retrieval_method).
func TestTask323_ProtoToSearchResult_CarriesVectorScore(t *testing.T) {
	sem := protoToSearchResult(&pb.SearchResultItem{
		ChunkId:         "chk_sem_0",
		RetrievalMethod: "vector",
		VectorScore:     0.83,
	})
	if sem.VectorScore != 0.83 {
		t.Errorf("semantic hit VectorScore = %v, want 0.83 (provenance carried, not inferred)", sem.VectorScore)
	}
	bm25 := protoToSearchResult(&pb.SearchResultItem{
		ChunkId:         "chk_bm25_0",
		RetrievalMethod: "bm25",
		VectorScore:     0,
	})
	if bm25.VectorScore != 0 {
		t.Errorf("BM25 hit VectorScore = %v, want 0", bm25.VectorScore)
	}
}

// fakeHybridQueryServer (task-39.2) captures the inbound SearchRequest.Hybrid so the
// grpcclient passthrough can be asserted.
type fakeHybridQueryServer struct {
	pb.UnimplementedSearchServiceServer
	mu        sync.Mutex
	gotHybrid bool
}

func (f *fakeHybridQueryServer) Query(_ context.Context, req *pb.SearchRequest) (*pb.SearchResponse, error) {
	f.mu.Lock()
	f.gotHybrid = req.Hybrid
	f.mu.Unlock()
	return &pb.SearchResponse{
		Results: []*pb.SearchResultItem{},
		Trace:   &pb.RetrievalTrace{TraceId: "t", Query: req.Query},
	}, nil
}

// TestTask392_GrpcClient_Search_ForwardsHybrid — task-39.2 §6 AC2: searchClient.Search maps
// contractv1.SearchRequest.Hybrid onto the gRPC pb.SearchRequest.Hybrid field (both true and
// false reach the core SearchService.Query); mirrors the task-20.1 Semantic precedent.
func TestTask392_GrpcClient_Search_ForwardsHybrid(t *testing.T) {
	for _, hyb := range []bool{true, false} {
		fake := &fakeHybridQueryServer{}
		addr, stop := spawnFakeServer(t, func(s *grpc.Server) {
			pb.RegisterSearchServiceServer(s, fake)
		})
		ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
		cli, _ := New(ctx, addr)
		_, _, err := cli.Search().Search(contractv1.SearchRequest{Query: "q", WorkspaceID: "w", Hybrid: hyb})
		if err != nil {
			t.Fatalf("Search(hybrid=%v): %v", hyb, err)
		}
		fake.mu.Lock()
		got := fake.gotHybrid
		fake.mu.Unlock()
		if got != hyb {
			t.Errorf("forwarded pb.SearchRequest.Hybrid = %v, want %v", got, hyb)
		}
		_ = cli.Close()
		cancel()
		stop()
	}
}

// TestTask392_ProtoToSearchResult_CarriesHybridScoreAndReason — task-39.2 §6 AC2:
// protoToSearchResult carries the add-only hybrid_score provenance (a hybrid hit's real
// fused score maps through; a non-hybrid hit's stays 0) AND the rerank reason marker
// (rerank provenance is visible end-to-end in the REST response; reranker stays env-driven
// per ADR-043 D3 / ADR-044 D3 — no per-request ?rerank param).
func TestTask392_ProtoToSearchResult_CarriesHybridScoreAndReason(t *testing.T) {
	hyb := protoToSearchResult(&pb.SearchResultItem{
		ChunkId:         "chk_hyb_0",
		RetrievalMethod: "hybrid",
		HybridScore:     0.91,
		Reason:          "reranked:identity",
	})
	if hyb.HybridScore != 0.91 {
		t.Errorf("hybrid hit HybridScore = %v, want 0.91 (provenance carried, not inferred)", hyb.HybridScore)
	}
	if hyb.Reason != "reranked:identity" {
		t.Errorf("rerank reason = %q, want %q (rerank provenance visible in REST, ADR-044 D3)", hyb.Reason, "reranked:identity")
	}
	bm25 := protoToSearchResult(&pb.SearchResultItem{
		ChunkId:         "chk_bm25_0",
		RetrievalMethod: "bm25",
		HybridScore:     0,
	})
	if bm25.HybridScore != 0 {
		t.Errorf("BM25 hit HybridScore = %v, want 0", bm25.HybridScore)
	}
}

func TestGrpcClient_GetSourceChunk_Maps_NotFound(t *testing.T) {
	fake := &fakeSearchServer{chunkErr: status.Error(codes.NotFound, "missing")}
	addr, stop := spawnFakeServer(t, func(s *grpc.Server) {
		pb.RegisterSearchServiceServer(s, fake)
	})
	defer stop()
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	cli, _ := New(ctx, addr)
	defer func() { _ = cli.Close() }()
	_, err := cli.Search().GetSourceChunk("missing")
	if !errors.Is(err, consoleapi.ErrNotFound) {
		t.Errorf("expected ErrNotFound; got %v", err)
	}
}

// Guard: unused imports (net/http) — kept for future fallback-inmem HTTP
// tests in cli pkg.
var _ = http.StatusServiceUnavailable

// ============================================================================
// task-16.2 (Phase 16 P4 #11) review pass 1: grpcclient eventsClient.Recent
// two-phase long-poll unit tests via a fake EventsServiceServer. Spec PR
// shipped only HTTP-handler-level tests using a stub EventsClient interface;
// the actual two-phase logic lived in this file untested.
// ============================================================================

// fakeEventsServer drives controlled gRPC server-stream behavior. Each
// Subscribe call consumes the next entry in `eventsByCall` (or empty if
// callIdx exceeds the slice). If `blockOnLastCall` is true, the last
// configured call blocks on ctx after emitting its events, mimicking a
// long-lived broadcast subscription. If `failWithCode` is set (non-OK),
// Subscribe returns that status code immediately on every call.
type fakeEventsServer struct {
	pb.UnimplementedEventsServiceServer
	eventsByCall    [][]*pb.ObservabilityEvent
	blockOnLastCall bool
	failWithCode    codes.Code
	callCount       int32 // protected via atomic for goroutine-safe test access
	mu              sync.Mutex
}

func (f *fakeEventsServer) Subscribe(
	_ *pb.SubscribeEventsRequest,
	stream grpc.ServerStreamingServer[pb.ObservabilityEvent],
) error {
	f.mu.Lock()
	idx := int(f.callCount)
	f.callCount++
	f.mu.Unlock()

	if f.failWithCode != codes.OK {
		return status.Error(f.failWithCode, "fake server error")
	}
	var events []*pb.ObservabilityEvent
	if idx < len(f.eventsByCall) {
		events = f.eventsByCall[idx]
	}
	for _, evt := range events {
		if err := stream.Send(evt); err != nil {
			return err
		}
	}
	if f.blockOnLastCall && idx >= len(f.eventsByCall)-1 {
		<-stream.Context().Done()
		return stream.Context().Err()
	}
	return nil
}

func fakeEvt(id, eventType string) *pb.ObservabilityEvent {
	return &pb.ObservabilityEvent{
		EventId:     id,
		EventType:   eventType,
		Severity:    "info",
		Source:      "contextforge-core",
		Message:     "fake " + eventType,
		TsUnix:      1_700_000_000,
		PayloadJson: "{}",
	}
}

// task-16.2 §6 AC2 (gRPC-level): phase-1 ctx timeout → return `[]` + nil.
// Distinct from TestHandleEvents_Wait2s_Blocks (which tests HTTP handler via
// stub EventsClient); this exercises the real grpcclient Recv loop ending
// on DeadlineExceeded.
func TestEventsClient_PhaseOneTimeout_ReturnsEmpty(t *testing.T) {
	fake := &fakeEventsServer{
		eventsByCall:    [][]*pb.ObservabilityEvent{{}},
		blockOnLastCall: true,
	}
	addr, stop := spawnFakeServer(t, func(s *grpc.Server) {
		pb.RegisterEventsServiceServer(s, fake)
	})
	defer stop()

	dialCtx, dialCancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer dialCancel()
	cli, err := New(dialCtx, addr)
	if err != nil {
		t.Fatalf("New: %v", err)
	}
	defer func() { _ = cli.Close() }()

	start := time.Now()
	out, err := cli.Events().Recent(10, 300*time.Millisecond)
	elapsed := time.Since(start)
	if err != nil {
		t.Fatalf("Recent: want nil err on phase-1 timeout, got %v", err)
	}
	if len(out) != 0 {
		t.Errorf("Recent: want empty batch on phase-1 timeout, got %d events", len(out))
	}
	if elapsed < 250*time.Millisecond {
		t.Errorf("Recent: elapsed %v — phase-1 did not wait for ctx timeout", elapsed)
	}
	if elapsed > 800*time.Millisecond {
		t.Errorf("Recent: elapsed %v — phase-1 over-ran ctx timeout", elapsed)
	}
}

// task-16.2 §6 AC3 (gRPC-level): phase-1 receives 1 event, phase-2 drains
// the follow-up events that the SECOND Subscribe call emits. Validates the
// two-phase batching mechanism end-to-end via a real gRPC server-stream.
func TestEventsClient_PhaseTwoBatchesFollowupEvents(t *testing.T) {
	fake := &fakeEventsServer{
		eventsByCall: [][]*pb.ObservabilityEvent{
			{fakeEvt("evt-1", "indexing.progress")},                                         // phase-1 call → 1 event
			{fakeEvt("evt-2", "indexing.progress"), fakeEvt("evt-3", "indexing.cancelled")}, // phase-2 call → 2 events
		},
		blockOnLastCall: true,
	}
	addr, stop := spawnFakeServer(t, func(s *grpc.Server) {
		pb.RegisterEventsServiceServer(s, fake)
	})
	defer stop()

	dialCtx, dialCancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer dialCancel()
	cli, err := New(dialCtx, addr)
	if err != nil {
		t.Fatalf("New: %v", err)
	}
	defer func() { _ = cli.Close() }()

	out, err := cli.Events().Recent(100, 1*time.Second)
	if err != nil {
		t.Fatalf("Recent: %v", err)
	}
	// Phase-1 yields evt-1; phase-2 yields evt-2 + evt-3. Total ≥ 3.
	if len(out) < 3 {
		t.Errorf("Recent: want ≥3 events (phase-1 + phase-2 drain), got %d: %+v", len(out), out)
	}
	// Verify event IDs are present (order may vary slightly under stream
	// scheduling but all 3 should appear).
	ids := map[string]bool{}
	for _, e := range out {
		ids[e.EventID] = true
	}
	for _, want := range []string{"evt-1", "evt-2", "evt-3"} {
		if !ids[want] {
			t.Errorf("Recent: missing %s in batch %+v", want, out)
		}
	}
}

// task-16.2 §3: phase-1 Recv non-DeadlineExceeded / non-EOF error path
// logs via log.Printf then returns `[]` + nil (Console expects 200 + []).
// Trigger: server returns codes.Internal status; client's stream.Recv
// surfaces it as a non-deadline non-EOF error.
func TestEventsClient_PhaseOne_LogsNonDeadlineErrors(t *testing.T) {
	fake := &fakeEventsServer{
		failWithCode: codes.Internal,
	}
	addr, stop := spawnFakeServer(t, func(s *grpc.Server) {
		pb.RegisterEventsServiceServer(s, fake)
	})
	defer stop()

	dialCtx, dialCancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer dialCancel()
	cli, err := New(dialCtx, addr)
	if err != nil {
		t.Fatalf("New: %v", err)
	}
	defer func() { _ = cli.Close() }()

	// The Subscribe call itself succeeds (gRPC stream setup is OK); the
	// server's Subscribe handler returns codes.Internal which surfaces as
	// a Recv error. Our client swallows it after log.Printf.
	out, err := cli.Events().Recent(10, 500*time.Millisecond)
	if err != nil {
		t.Errorf("Recent: want nil err (codes.Internal silently swallowed), got %v", err)
	}
	if len(out) != 0 {
		t.Errorf("Recent: want empty batch on server error, got %d", len(out))
	}
}

// task-16.2 §6 AC6 (gRPC-level): defer cancel × 2 + ctx propagation must
// not leak goroutines. Smoke test: capture runtime.NumGoroutine baseline
// after a warm-up call, then run N Recent calls with short waits; assert
// final count doesn't grow unboundedly (tolerance allows for gRPC internal
// pool jitter — the bound catches +1-per-call regressions).
func TestEventsClient_NoGoroutineLeakAfterMultipleCalls(t *testing.T) {
	fake := &fakeEventsServer{
		eventsByCall:    [][]*pb.ObservabilityEvent{{}},
		blockOnLastCall: true,
	}
	addr, stop := spawnFakeServer(t, func(s *grpc.Server) {
		pb.RegisterEventsServiceServer(s, fake)
	})
	defer stop()

	dialCtx, dialCancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer dialCancel()
	cli, err := New(dialCtx, addr)
	if err != nil {
		t.Fatalf("New: %v", err)
	}
	defer func() { _ = cli.Close() }()

	// Warm-up: 1 call establishes baseline (gRPC may lazily spawn pool goroutines).
	_, _ = cli.Events().Recent(10, 50*time.Millisecond)
	runtime.GC()
	time.Sleep(50 * time.Millisecond)
	baseline := runtime.NumGoroutine()

	const n = 10
	for i := 0; i < n; i++ {
		_, _ = cli.Events().Recent(10, 50*time.Millisecond)
	}
	runtime.GC()
	time.Sleep(100 * time.Millisecond)
	final := runtime.NumGoroutine()

	// Tolerance bound: +5 absorbs gRPC pool / scheduler noise. A real
	// leak (e.g., +1 goroutine per Recent call) would push final to
	// baseline+10 and trip the bound.
	if final > baseline+5 {
		t.Errorf(
			"goroutine count grew from %d to %d after %d Recent calls — possible leak (tolerance: +5)",
			baseline, final, n,
		)
	}
}

// TEST-26.3.1d (ADR-031 D5): events drain timeout is env-configurable
// (CONSOLE_EVENTS_DRAIN_TIMEOUT) with a conservative 100ms default; invalid /
// non-positive values fall back to the default (task-16.2 behavior unchanged).
func TestDrainTimeoutFromEnv(t *testing.T) {
	cases := []struct {
		name string
		set  bool
		raw  string
		want time.Duration
	}{
		{"default_unset", false, "", 100 * time.Millisecond},
		{"valid_150ms", true, "150ms", 150 * time.Millisecond},
		{"valid_1s", true, "1s", time.Second},
		{"invalid_garbage", true, "not-a-duration", 100 * time.Millisecond},
		{"non_positive", true, "0s", 100 * time.Millisecond},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			if tc.set {
				t.Setenv("CONSOLE_EVENTS_DRAIN_TIMEOUT", tc.raw)
			} else {
				t.Setenv("CONSOLE_EVENTS_DRAIN_TIMEOUT", "")
			}
			if got := drainTimeoutFromEnv(); got != tc.want {
				t.Errorf("drainTimeoutFromEnv() = %v want %v", got, tc.want)
			}
		})
	}
}

// fakeMemoryServer captures the Pin request so we can assert grpcclient forwards Actor (task-40.1).
type fakeMemoryServer struct {
	pb.UnimplementedMemoryServiceServer
	lastPin *pb.PinMemoryRequest
}

func (f *fakeMemoryServer) Pin(_ context.Context, req *pb.PinMemoryRequest) (*pb.PinMemoryResponse, error) {
	f.lastPin = req
	return &pb.PinMemoryResponse{}, nil
}

// TEST-40.1.4 (task-40.1 / ADR-045 D1): memoryClient.Pin forwards the actor into
// pb.PinMemoryRequest.Actor (add-only field 3). Empty actor → empty field (server-side fallback
// to "console-api"). memory_id / pin are unchanged.
func TestTask401_GrpcClient_Pin_ForwardsActor(t *testing.T) {
	fake := &fakeMemoryServer{}
	addr, stop := spawnFakeServer(t, func(s *grpc.Server) {
		pb.RegisterMemoryServiceServer(s, fake)
	})
	defer stop()

	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	cli, err := New(ctx, addr)
	if err != nil {
		t.Fatalf("New: %v", err)
	}
	defer func() { _ = cli.Close() }()

	if err := cli.Memory().Pin("m1", true, "alice"); err != nil {
		t.Fatalf("Pin: %v", err)
	}
	if fake.lastPin == nil {
		t.Fatal("server did not receive Pin")
	}
	if got := fake.lastPin.GetActor(); got != "alice" {
		t.Errorf("expected forwarded Actor %q; got %q", "alice", got)
	}
	if fake.lastPin.GetMemoryId() != "m1" || !fake.lastPin.GetPin() {
		t.Errorf("memory_id/pin drift: %+v", fake.lastPin)
	}

	// Empty actor → empty proto field (server falls back to "console-api").
	if err := cli.Memory().Pin("m1", false, ""); err != nil {
		t.Fatalf("Pin(empty actor): %v", err)
	}
	if got := fake.lastPin.GetActor(); got != "" {
		t.Errorf("expected empty Actor; got %q", got)
	}
}

// fakeSourceTypeQueryServer (task-42.2) captures the inbound SearchRequest.SourceType so the
// grpcclient passthrough can be asserted.
type fakeSourceTypeQueryServer struct {
	pb.UnimplementedSearchServiceServer
	mu    sync.Mutex
	gotST []string
}

func (f *fakeSourceTypeQueryServer) Query(_ context.Context, req *pb.SearchRequest) (*pb.SearchResponse, error) {
	f.mu.Lock()
	f.gotST = req.SourceType
	f.mu.Unlock()
	return &pb.SearchResponse{
		Results: []*pb.SearchResultItem{},
		Trace:   &pb.RetrievalTrace{TraceId: "t", Query: req.Query},
	}, nil
}

// TestTask422_GrpcClient_Search_ForwardsSourceType — task-42.2 §6 AC2: searchClient.Search maps
// contractv1.SearchRequest.SourceType onto the gRPC pb.SearchRequest.source_type field; mirrors
// the Semantic/Hybrid precedent. Empty → nil (no filter, backward-compatible).
func TestTask422_GrpcClient_Search_ForwardsSourceType(t *testing.T) {
	cases := [][]string{{"code", "doc"}, nil}
	for _, want := range cases {
		fake := &fakeSourceTypeQueryServer{}
		addr, stop := spawnFakeServer(t, func(s *grpc.Server) {
			pb.RegisterSearchServiceServer(s, fake)
		})
		ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
		cli, _ := New(ctx, addr)
		_, _, err := cli.Search().Search(contractv1.SearchRequest{Query: "q", WorkspaceID: "w", SourceType: want})
		if err != nil {
			t.Fatalf("Search(source_type=%v): %v", want, err)
		}
		fake.mu.Lock()
		got := fake.gotST
		fake.mu.Unlock()
		if strings.Join(got, ",") != strings.Join(want, ",") {
			t.Errorf("forwarded pb.SearchRequest.SourceType = %v, want %v", got, want)
		}
		_ = cli.Close()
		cancel()
		stop()
	}
}
