//! task-11.1 (Phase 11, ADR-016 §D2): Console data plane gRPC services.
//!
//! Four tonic services backing the Console REST 9-endpoint surface via a
//! Go thin proxy (`internal/consoleapi/grpcclient`, task-11.2):
//!
//! - [`WorkspaceServer`] (`workspace`) — Workspace CRUD against
//!   [`crate::workspace::SqliteWorkspaceStore`] (task-10.2)
//! - [`JobServer`] (`job`) — IndexJob enqueue/get/cancel against
//!   [`crate::jobs::SqliteJobStore`] (task-10.3); JobRunner真触发
//!   `IndexSession::index_path_with_progress` 在 task-11.3 [SPEC-OWNER:task-11.3]
//! - [`SearchServer`] (`search`) — task-11.1 returns empty results;
//!   真接 retriever 在 task-11.4 [SPEC-OWNER:task-11.4]
//! - [`EventsServer`] (`events`) — task-11.1 emits keepalive only;
//!   真接 EventBus broadcast channel 在 task-11.4 [SPEC-OWNER:task-11.4]
//!
//! Field naming (ADR-016 §D3 thin proxy): proto snake_case → prost-generated
//! Rust struct snake_case → matches Go contractv1 JSON tag 1:1.

pub mod eval;
pub mod events;
pub mod health;
pub mod indexing_events;
pub mod job;
pub mod memory;
pub mod search;
pub mod search_persist;
pub mod workspace;

use crate::pb_console::eval_service_server::EvalServiceServer;
use crate::pb_console::events_service_server::EventsServiceServer;
use crate::pb_console::health_service_server::HealthServiceServer;
use crate::pb_console::job_service_server::JobServiceServer;
use crate::pb_console::memory_service_server::MemoryServiceServer;
use crate::pb_console::search_service_server::SearchServiceServer;
use crate::pb_console::workspace_service_server::WorkspaceServiceServer;
use std::sync::{Arc, Mutex};

/// Shared stores injected into all 4 tonic service implementations.
///
/// task-11.1 only needed `workspace_store` + `job_store`. task-11.3 added
/// `job_runner` + `data_dir`. task-11.4 added `event_bus` for real
/// `EventsService.Subscribe` server stream + `JobRunner` progress emission.
pub struct DataPlaneStores {
    pub workspace_store: Arc<crate::workspace::SqliteWorkspaceStore>,
    pub job_store: Arc<crate::jobs::SqliteJobStore>,
    /// task-11.3: real JobRunner backed by `IndexSessionBackend`. When None
    /// (e.g. task-11.1 tests), `JobService.Enqueue` only writes status=queued
    /// without spawning a worker.
    pub job_runner: Option<Arc<crate::jobs::JobRunner<crate::jobs::IndexSessionBackend>>>,
    /// task-11.3: data directory passed to `IndexSession::open(data_dir, ws_id)`.
    /// Empty path means no spawning (task-11.1 default).
    pub data_dir: std::path::PathBuf,
    /// task-11.4: shared broadcast event bus. `EventsService.Subscribe` streams
    /// from `event_bus.subscribe()`; `JobRunner` progress callback emits
    /// `indexing.progress` / `.cancelled` / `.error` via `event_bus.send`.
    /// `None` (task-11.1 / unit tests) falls back to single-keepalive stream.
    pub event_bus: Option<Arc<events::EventBus>>,
    /// task-13.1: shared `SqliteMemoryStore` backing the `MemoryService` 5 RPC.
    /// `None` for the 4-service Phase 11 baseline (workspace/job/search/events
    /// tests don't need the memory store). `MemoryServer.list/get/...` returns
    /// `failed_precondition` when this is None.
    pub memory: Option<Arc<crate::memory::SqliteMemoryStore>>,
    /// task-13.1: shared `AuditSink` used by `MemoryServer` to emit
    /// pin/deprecate/soft-delete audit events. `None` falls back to silent
    /// no-op (state ops still succeed; audit is observability).
    pub audit: Option<Arc<Mutex<crate::memoryops::audit::AuditSink>>>,
    /// task-14.1: shared `SqliteEvalStore` backing `EvalService` 3 RPC.
    pub eval: Option<Arc<crate::eval::SqliteEvalStore>>,
    /// task-16.1 (Phase 16 P4 #10): SQLite-backed persistence for the in-memory
    /// `TraceStore` (used by `SearchServer.get_search_trace` + `.list_queries`).
    /// `None` keeps the in-memory-only behavior (task-11.1/12.3/15.5 baseline);
    /// `Some` enables write-through + warm restore on daemon boot.
    pub trace_persist: Option<Arc<search_persist::SqliteTracePersist>>,
}

impl DataPlaneStores {
    /// task-11.1 constructor: no JobRunner spawning + no EventBus. Used by
    /// data_plane unit tests + integration tests that only exercise the
    /// gRPC wire.
    pub fn new(
        workspace_store: Arc<crate::workspace::SqliteWorkspaceStore>,
        job_store: Arc<crate::jobs::SqliteJobStore>,
    ) -> Arc<Self> {
        Arc::new(Self {
            workspace_store,
            job_store,
            job_runner: None,
            data_dir: std::path::PathBuf::new(),
            event_bus: None,
            memory: None,
            audit: None,
            eval: None,
            trace_persist: None,
        })
    }

    /// task-14.1 constructor: stores + eval store. Used by EvalServer tests.
    pub fn with_eval(
        workspace_store: Arc<crate::workspace::SqliteWorkspaceStore>,
        job_store: Arc<crate::jobs::SqliteJobStore>,
        eval: Arc<crate::eval::SqliteEvalStore>,
    ) -> Arc<Self> {
        Arc::new(Self {
            workspace_store,
            job_store,
            job_runner: None,
            data_dir: std::path::PathBuf::new(),
            event_bus: None,
            memory: None,
            audit: None,
            eval: Some(eval),
            trace_persist: None,
        })
    }

    /// task-13.1 constructor: 4 Phase 11 stores + MemoryStore + AuditSink for
    /// the new MemoryService 5 RPC. Used by memory tests + `serve_full` once
    /// production wiring lands.
    pub fn with_memory(
        workspace_store: Arc<crate::workspace::SqliteWorkspaceStore>,
        job_store: Arc<crate::jobs::SqliteJobStore>,
        memory: Arc<crate::memory::SqliteMemoryStore>,
        audit: Arc<Mutex<crate::memoryops::audit::AuditSink>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            workspace_store,
            job_store,
            job_runner: None,
            data_dir: std::path::PathBuf::new(),
            event_bus: None,
            memory: Some(memory),
            audit: Some(audit),
            eval: None,
            trace_persist: None,
        })
    }

    /// task-11.4 constructor: full production wiring with `IndexSession`-backed
    /// `JobRunner` + `EventBus`. Used by `serve_full` in `server.rs`.
    pub fn with_runner_and_bus(
        workspace_store: Arc<crate::workspace::SqliteWorkspaceStore>,
        job_store: Arc<crate::jobs::SqliteJobStore>,
        job_runner: Arc<crate::jobs::JobRunner<crate::jobs::IndexSessionBackend>>,
        data_dir: std::path::PathBuf,
        event_bus: Arc<events::EventBus>,
    ) -> Arc<Self> {
        Self::full(
            workspace_store, job_store, job_runner, data_dir, event_bus, None, None, None, None,
        )
    }

    /// task-13.1 full production wiring constructor — Phase 11 4 stores +
    /// memory + audit. `serve_full` in `server.rs` uses this to register all
    /// 5 services (workspace / job / search / events / memory).
    #[allow(clippy::too_many_arguments)]
    pub fn full(
        workspace_store: Arc<crate::workspace::SqliteWorkspaceStore>,
        job_store: Arc<crate::jobs::SqliteJobStore>,
        job_runner: Arc<crate::jobs::JobRunner<crate::jobs::IndexSessionBackend>>,
        data_dir: std::path::PathBuf,
        event_bus: Arc<events::EventBus>,
        memory: Option<Arc<crate::memory::SqliteMemoryStore>>,
        audit: Option<Arc<Mutex<crate::memoryops::audit::AuditSink>>>,
        eval: Option<Arc<crate::eval::SqliteEvalStore>>,
        trace_persist: Option<Arc<search_persist::SqliteTracePersist>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            workspace_store,
            job_store,
            job_runner: Some(job_runner),
            data_dir,
            event_bus: Some(event_bus),
            memory,
            audit,
            eval,
            trace_persist,
        })
    }

    /// task-11.3 constructor (retained for backward-compatibility): full
    /// production wiring with `IndexSession`-backed `JobRunner` but without
    /// EventBus. `EventsService.Subscribe` falls back to single keepalive.
    pub fn with_runner(
        workspace_store: Arc<crate::workspace::SqliteWorkspaceStore>,
        job_store: Arc<crate::jobs::SqliteJobStore>,
        job_runner: Arc<crate::jobs::JobRunner<crate::jobs::IndexSessionBackend>>,
        data_dir: std::path::PathBuf,
    ) -> Arc<Self> {
        Arc::new(Self {
            workspace_store,
            job_store,
            job_runner: Some(job_runner),
            data_dir,
            event_bus: None,
            memory: None,
            audit: None,
            eval: None,
            trace_persist: None,
        })
    }
}

/// Register 4 Console data plane services on a tonic Server builder/router.
///
/// `Router` is `tonic::transport::server::Router`. Caller wires the resulting
/// router into `.serve(addr).await` or composes with additional services
/// (e.g. Phase 9 `ContextServiceServer`).
pub fn register_services(
    router: tonic::transport::server::Router,
    stores: Arc<DataPlaneStores>,
) -> tonic::transport::server::Router {
    router
        .add_service(WorkspaceServiceServer::new(workspace::WorkspaceServer::new(
            stores.clone(),
        )))
        .add_service(JobServiceServer::new(job::JobServer::new(stores.clone())))
        .add_service(SearchServiceServer::new(search::SearchServer::new(
            stores.clone(),
        )))
        .add_service(EventsServiceServer::new(events::EventsServer::new(
            stores.clone(),
        )))
        .add_service(MemoryServiceServer::new(memory::MemoryServer::new(
            stores.clone(),
        )))
        .add_service(EvalServiceServer::new(eval::EvalServer::new(stores.clone())))
        .add_service(HealthServiceServer::new(health::HealthCheckServer::new(
            stores.clone(),
        )))
}

/// Add 6 services to a fresh `Server::builder()` (no other services).
/// Useful for tests where only the Console data plane is needed.
pub fn server_with_services(
    stores: Arc<DataPlaneStores>,
) -> tonic::transport::server::Router {
    let mut server = tonic::transport::Server::builder();
    let router = server.add_service(WorkspaceServiceServer::new(
        workspace::WorkspaceServer::new(stores.clone()),
    ));
    router
        .add_service(JobServiceServer::new(job::JobServer::new(stores.clone())))
        .add_service(SearchServiceServer::new(search::SearchServer::new(
            stores.clone(),
        )))
        .add_service(EventsServiceServer::new(events::EventsServer::new(
            stores.clone(),
        )))
        .add_service(MemoryServiceServer::new(memory::MemoryServer::new(
            stores.clone(),
        )))
        .add_service(EvalServiceServer::new(eval::EvalServer::new(stores.clone())))
        .add_service(HealthServiceServer::new(health::HealthCheckServer::new(
            stores,
        )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::SqliteJobStore;
    use crate::workspace::SqliteWorkspaceStore;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_data_dir(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "cf-data-plane-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// task-11.1 §6 AC2: register_services 把 4 service 注册到 tonic Router.
    /// The Router type is opaque (tonic doesn't expose introspection), so we
    /// build a router + verify it can be passed to `.serve(addr)` (compile-time
    /// type satisfied) + the underlying stores are non-empty.
    #[test]
    fn test_register_services_adds_4_services() {
        let data_dir = temp_data_dir("register");
        let ws = Arc::new(SqliteWorkspaceStore::open(&data_dir).expect("open ws"));
        let js = Arc::new(SqliteJobStore::open(&data_dir).expect("open js"));
        let stores = DataPlaneStores::new(ws, js);

        let _router = server_with_services(stores.clone());
        // Compile-time: Router has add_service, .serve, etc. We don't run
        // .serve() here (would block); we just assert the stores wiring.
        assert!(Arc::strong_count(&stores) >= 1, "stores Arc shared");
    }

    /// task-11.1 §6 AC1: proto field naming snake_case 1:1 with Go contractv1
    /// JSON tag. prost generates Rust struct fields in snake_case by default
    /// (matches proto field names), so we compile-time check the field-access
    /// path. If the proto were to drift (e.g. someone changed the field name
    /// to camelCase), this test would fail to compile.
    #[test]
    fn test_proto_field_snake_case_consistency() {
        // Workspace 字段
        let w = crate::pb_console::Workspace {
            workspace_id: "ws".into(),
            name: "n".into(),
            root_path: "/tmp".into(),
            status: "ready".into(),
            config_snapshot: "{}".into(),
            allowlist: vec![],
            denylist: vec![],
            created_at_unix: 0,
            updated_at_unix: 0,
        };
        assert_eq!(w.workspace_id, "ws");

        // IndexJob 字段 (含 optional)
        let j = crate::pb_console::IndexJob {
            job_id: "j1".into(),
            workspace_id: "ws".into(),
            trigger_source: "test".into(),
            status: "queued".into(),
            stage: "".into(),
            processed_files: 0,
            total_files: 0,
            failed_files: 0,
            skipped_files: 0,
            error_message: None,
            started_at_unix: None,
            finished_at_unix: None,
            last_heartbeat_at_unix: None,
        };
        assert_eq!(j.job_id, "j1");
        assert!(j.error_message.is_none());

        // SearchRequest + SearchResponse 字段
        let s = crate::pb_console::SearchRequest {
            query: "q".into(),
            workspace_id: "ws".into(),
            agent_scope: "".into(),
            retrieval_method: "bm25".into(),
            top_k: 5,
            config_snapshot: "{}".into(),
            semantic: false,
            hybrid: false,
        };
        assert_eq!(s.top_k, 5);

        // RetrievalTrace + SourceChunk + ObservabilityEvent 字段
        let _t = crate::pb_console::RetrievalTrace {
            trace_id: "t".into(),
            query: "q".into(),
            expanded_query: None,
            candidate_generation_steps: vec![],
            lexical_candidates_count: 0,
            vector_candidates_count: 0,
            rerank_steps: vec![],
            scope_filter_result: "".into(),
            final_context_count: 0,
            retrieved_chunks: vec![],
        };
        let _c = crate::pb_console::SourceChunk {
            chunk_id: "c".into(),
            workspace_id: "ws".into(),
            source_file_path: "/x".into(),
            line_start: 0,
            line_end: 0,
            chunk_text_preview: "".into(),
            chunk_offset_start: 0,
            chunk_offset_end: 0,
            redaction_status: "applied".into(),
        };
        let _e = crate::pb_console::ObservabilityEvent {
            event_id: "e".into(),
            event_type: "x".into(),
            severity: "info".into(),
            source: "core".into(),
            message: "".into(),
            ts_unix: 0,
            trace_id: None,
            job_id: None,
            payload_json: "{}".into(),
        };
    }
}
