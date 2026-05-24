package grpcclient

import (
	"context"
	"errors"
	"net"
	"net/http"
	"strings"
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
