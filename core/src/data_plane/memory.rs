//! task-13.1 (ADR-017 D1 Wave 3): `MemoryServer` impl `MemoryService` trait.
//!
//! 5 RPC: List / Get / Pin / Deprecate / SoftDelete → 真走 `SqliteMemoryStore`
//! (task-13.1) + Pin / Deprecate / SoftDelete each emit one audit event via
//! the shared `AuditSink` so Console UI's audit log panel surfaces the trace.
//!
//! task-15.2 (Phase 15 P0 #2 / ADR-021): each state op also emits an
//! `ObservabilityEvent` (`memory.pin` / `memory.deprecate` / `memory.soft_delete`)
//! to the shared `EventBus` so Console UI's `/v1/observability/events` stream
//! surfaces memory state changes alongside `indexing.*` events. Both audit
//! and event paths are best-effort and decoupled from state success.
//!
//! Error mapping (ADR-016 §D3 thin proxy):
//!   - `MemoryStoreError::NotFound` → `tonic::Status::not_found`
//!   - `MemoryStoreError::Invalid` → `tonic::Status::invalid_argument`
//!   - others → `tonic::Status::internal`
//!
//! Audit failures are logged but do NOT fail the state-op (REST 204 still
//! returned) — audit is observability, not authority.

use std::sync::{Arc, Mutex};

use tonic::{Request, Response, Status};

use crate::memory::{MemoryItem as RustMemoryItem, MemoryListFilter, MemoryStoreError};
use crate::memoryops::audit::{AuditEvent, AuditOperation, AuditSink};
use crate::pb_console::memory_service_server::MemoryService;
use crate::pb_console::{
    DeprecateMemoryRequest, DeprecateMemoryResponse, GetMemoryRequest, HardDeleteMemoryRequest,
    HardDeleteMemoryResponse, ListMemoryRequest, ListMemoryResponse, MemoryItem as PbMemoryItem,
    ObservabilityEvent as PbEvent, PinMemoryRequest, PinMemoryResponse, SoftDeleteMemoryRequest,
    SoftDeleteMemoryResponse, UnpinMemoryRequest, UnpinMemoryResponse,
};

use super::DataPlaneStores;

pub struct MemoryServer {
    stores: Arc<DataPlaneStores>,
}

impl MemoryServer {
    pub fn new(stores: Arc<DataPlaneStores>) -> Self {
        Self { stores }
    }

    /// task-15.2 (Phase 15 P0 #2 / ADR-021): emit audit AND broadcast a
    /// matching `ObservabilityEvent` to the shared `EventBus` so the Console UI
    /// `/v1/observability/events` stream surfaces memory state changes. Both
    /// paths are best-effort: `AuditSink.record` failure or `EventBus.send`
    /// no-subscriber `SendError` is swallowed and the state-op return is
    /// unaffected (observability != authority).
    fn emit_audit_and_event(&self, op: AuditOperation, memory_id: &str) {
        // 1. AuditSink (既有路径 — Phase 13 task-13.1 ship)
        if let Some(audit) = self.stores.audit.as_ref() {
            if let Ok(mut sink) = audit.lock() {
                let event = AuditEvent {
                    operation: op,
                    collection: "memory".to_string(),
                    source: "console-api".to_string(),
                    result_count: 1,
                    redaction_count: 0,
                    query: None,
                    redacted_terms: vec![],
                    chunk_ids: vec![memory_id.to_string()],
                    export_total_byte_count: None,
                };
                let _ = sink.record(event);
            }
        }
        // 2. EventBus broadcast (task-15.2 / ADR-021 D1 新增桥接)
        if let Some(bus) = self.stores.event_bus.as_ref() {
            if let Some(evt) = build_memory_event(op, memory_id) {
                // ADR-021 D4: SendError swallowed (no subscriber is fine; events
                // are observability, not durable state).
                let _ = bus.send(evt);
            }
        }
    }
}

/// task-15.2 / ADR-021 D2: map AuditOperation → ObservabilityEvent.event_type
/// string. Pin and Unpin share `memory.pin` (payload_json `op` distinguishes)
/// per ADR-021 D2 to keep the event_type namespace compact.
fn audit_op_to_event_type(op: AuditOperation) -> Option<&'static str> {
    match op {
        AuditOperation::MemoryPin | AuditOperation::MemoryUnpin => Some("memory.pin"),
        AuditOperation::MemoryDeprecate => Some("memory.deprecate"),
        AuditOperation::MemorySoftDelete => Some("memory.soft_delete"),
        // task-27.2 (ADR-032 D2): physical delete gets its own event_type to
        // distinguish from soft_delete (status flip vs row removal).
        AuditOperation::MemoryHardDelete => Some("memory.hard_delete"),
        // Non-memory ops should never reach this bridge; returning None
        // causes `emit_audit_and_event` to skip EventBus emission without
        // panicking.
        _ => None,
    }
}

/// task-15.2 / ADR-021 D3: build the `PbEvent` populating the fixed field
/// contract for memory events. `trace_id` and `job_id` are `None` (memory ops
/// have no trace / job context); `payload_json` carries `memory_id` + `op`
/// so subscribers can disambiguate pin vs unpin without parsing the message.
fn build_memory_event(op: AuditOperation, memory_id: &str) -> Option<PbEvent> {
    let event_type = audit_op_to_event_type(op)?;
    let op_str = match op {
        AuditOperation::MemoryPin => "pin",
        AuditOperation::MemoryUnpin => "unpin",
        AuditOperation::MemoryDeprecate => "deprecate",
        AuditOperation::MemorySoftDelete => "soft_delete",
        AuditOperation::MemoryHardDelete => "hard_delete",
        _ => return None,
    };
    let payload_json = format!(
        r#"{{"memory_id":{},"op":"{}"}}"#,
        serde_json::to_string(memory_id).unwrap_or_else(|_| String::from("\"\"")),
        op_str,
    );
    Some(PbEvent {
        event_id: format!("evt-memory-{}", now_unix_nanos()),
        event_type: event_type.to_string(),
        severity: "info".to_string(),
        source: "contextforge-core".to_string(),
        message: format!("memory {op_str}: {memory_id}"),
        ts_unix: now_unix(),
        trace_id: None,
        job_id: None,
        payload_json,
    })
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn now_unix_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn memory_to_pb(m: RustMemoryItem) -> PbMemoryItem {
    PbMemoryItem {
        memory_id: m.memory_id,
        agent_scope: m.agent_scope,
        content_preview: m.content_preview,
        source_type: m.source_type,
        source_ref: m.source_ref,
        created_at_unix: m.created_at_unix,
        updated_at_unix: m.updated_at_unix,
        hit_count: m.hit_count,
        status: m.status,
        is_pinned: m.is_pinned,
        // task-27.1 (ADR-032 D1): project add-only pin-actor + pinned-at-timestamp.
        pinned_by: m.pinned_by,
        pinned_at_unix: m.pinned_at_unix,
    }
}

fn mem_err_to_status(e: MemoryStoreError) -> Status {
    match e {
        MemoryStoreError::NotFound => Status::not_found("memory item not found"),
        MemoryStoreError::Invalid(msg) => Status::invalid_argument(msg),
        MemoryStoreError::Sqlite(msg) => Status::internal(format!("sqlite: {msg}")),
        MemoryStoreError::Io(err) => Status::internal(format!("io: {err}")),
    }
}

#[tonic::async_trait]
impl MemoryService for MemoryServer {
    async fn list(
        &self,
        req: Request<ListMemoryRequest>,
    ) -> Result<Response<ListMemoryResponse>, Status> {
        let req = req.into_inner();
        let memory = self
            .stores
            .memory
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("memory store not configured"))?;
        let filter = MemoryListFilter {
            agent_id: opt_str(req.agent_id),
            scope: opt_str(req.scope),
            namespace: opt_str(req.namespace),
            include_soft_deleted: req.include_soft_deleted,
        };
        let items = memory.list(filter).map_err(mem_err_to_status)?;
        Ok(Response::new(ListMemoryResponse {
            items: items.into_iter().map(memory_to_pb).collect(),
        }))
    }

    async fn get(
        &self,
        req: Request<GetMemoryRequest>,
    ) -> Result<Response<PbMemoryItem>, Status> {
        let id = req.into_inner().memory_id;
        if id.is_empty() {
            return Err(Status::invalid_argument("memory_id must not be empty"));
        }
        let memory = self
            .stores
            .memory
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("memory store not configured"))?;
        match memory.get(&id).map_err(mem_err_to_status)? {
            Some(m) => Ok(Response::new(memory_to_pb(m))),
            None => Err(Status::not_found(format!("memory item not found: {id}"))),
        }
    }

    async fn pin(
        &self,
        req: Request<PinMemoryRequest>,
    ) -> Result<Response<PinMemoryResponse>, Status> {
        let req = req.into_inner();
        let memory = self
            .stores
            .memory
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("memory store not configured"))?;
        // task-27.1 (ADR-032 D1): write the calling actor through to the store.
        // task-40.1 (ADR-045 D1): the calling actor is now propagated from console-api
        // (X-Actor header → PinMemoryRequest.actor → here). Empty actor falls back to
        // "console-api" (byte-equivalent default for callers that send no header, ADR-004).
        // Caller-supplied actor is a *declared* identity; verifying it against an
        // authenticated subject is [SPEC-DEFER:phase-future.memory-actor-authenticated-identity].
        let actor = if req.actor.is_empty() {
            "console-api"
        } else {
            req.actor.as_str()
        };
        memory
            .set_pinned_with_actor(&req.memory_id, req.pin, actor)
            .map_err(mem_err_to_status)?;
        self.emit_audit_and_event(
            if req.pin {
                AuditOperation::MemoryPin
            } else {
                AuditOperation::MemoryUnpin
            },
            &req.memory_id,
        );
        Ok(Response::new(PinMemoryResponse {}))
    }

    async fn deprecate(
        &self,
        req: Request<DeprecateMemoryRequest>,
    ) -> Result<Response<DeprecateMemoryResponse>, Status> {
        let id = req.into_inner().memory_id;
        let memory = self
            .stores
            .memory
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("memory store not configured"))?;
        memory
            .set_status(&id, "deprecated")
            .map_err(mem_err_to_status)?;
        self.emit_audit_and_event(AuditOperation::MemoryDeprecate, &id);
        Ok(Response::new(DeprecateMemoryResponse {}))
    }

    async fn soft_delete(
        &self,
        req: Request<SoftDeleteMemoryRequest>,
    ) -> Result<Response<SoftDeleteMemoryResponse>, Status> {
        let id = req.into_inner().memory_id;
        let memory = self
            .stores
            .memory
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("memory store not configured"))?;
        memory
            .set_status(&id, "soft_deleted")
            .map_err(mem_err_to_status)?;
        self.emit_audit_and_event(AuditOperation::MemorySoftDelete, &id);
        Ok(Response::new(SoftDeleteMemoryResponse {}))
    }

    /// task-27.2 (ADR-032 D2): explicit Unpin = `set_pinned(id, false)` made
    /// semantically explicit + idempotent (unpin already-unpinned item is Ok).
    /// Emits `MemoryUnpin` (既有 audit op 复用). The Pin{bool pin} toggle stays.
    async fn unpin(
        &self,
        req: Request<UnpinMemoryRequest>,
    ) -> Result<Response<UnpinMemoryResponse>, Status> {
        let id = req.into_inner().memory_id;
        let memory = self
            .stores
            .memory
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("memory store not configured"))?;
        memory
            .set_pinned_with_actor(&id, false, "console-api")
            .map_err(mem_err_to_status)?;
        self.emit_audit_and_event(AuditOperation::MemoryUnpin, &id);
        Ok(Response::new(UnpinMemoryResponse {}))
    }

    /// task-27.2 (ADR-032 D2): hard-delete physically removes the row (vs
    /// soft-delete's status flip). Emits `MemoryHardDelete`. console-api gates
    /// this behind `confirmMiddleware` (X-Confirm, ADR-017 D2).
    async fn hard_delete(
        &self,
        req: Request<HardDeleteMemoryRequest>,
    ) -> Result<Response<HardDeleteMemoryResponse>, Status> {
        let id = req.into_inner().memory_id;
        let memory = self
            .stores
            .memory
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("memory store not configured"))?;
        memory.hard_delete(&id).map_err(mem_err_to_status)?;
        self.emit_audit_and_event(AuditOperation::MemoryHardDelete, &id);
        Ok(Response::new(HardDeleteMemoryResponse {}))
    }
}

fn opt_str(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[allow(dead_code)]
type MutexSink = Arc<Mutex<AuditSink>>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::SqliteJobStore;
    use crate::memory::{MemoryItem, SqliteMemoryStore};
    use crate::workspace::SqliteWorkspaceStore;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let seq = TEST_SEQ.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!(
            "cf-memory-server-{name}-{}-{nanos}-{seq}",
            std::process::id()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn fresh_server() -> (MemoryServer, Arc<SqliteMemoryStore>) {
        let dir = temp_dir("base");
        let ws = Arc::new(SqliteWorkspaceStore::open(&dir).unwrap());
        let js = Arc::new(SqliteJobStore::open(&dir).unwrap());
        let mem = Arc::new(SqliteMemoryStore::open(&dir).unwrap());
        let audit = Arc::new(Mutex::new(
            AuditSink::open(dir.as_path(), "memory").expect("audit open"),
        ));
        let stores = DataPlaneStores::with_memory(ws, js, mem.clone(), audit);
        (MemoryServer::new(stores), mem)
    }

    /// task-15.2 test helper: like `fresh_server` but also wires a shared
    /// EventBus so the new audit→event bridge path is exercised. Returns the
    /// EventBus too so callers can subscribe and assert emitted events.
    fn fresh_server_with_event_bus(
    ) -> (MemoryServer, Arc<SqliteMemoryStore>, Arc<crate::data_plane::events::EventBus>) {
        let dir = temp_dir("evt");
        let ws = Arc::new(SqliteWorkspaceStore::open(&dir).unwrap());
        let js = Arc::new(SqliteJobStore::open(&dir).unwrap());
        let mem = Arc::new(SqliteMemoryStore::open(&dir).unwrap());
        let audit = Arc::new(Mutex::new(
            AuditSink::open(dir.as_path(), "memory").expect("audit open"),
        ));
        let event_bus = crate::data_plane::events::EventBus::new();
        let stores = Arc::new(DataPlaneStores {
            workspace_store: ws,
            job_store: js,
            job_runner: None,
            data_dir: std::path::PathBuf::new(),
            event_bus: Some(event_bus.clone()),
            memory: Some(mem.clone()),
            audit: Some(audit),
            eval: None,
            trace_persist: None,
        });
        (MemoryServer::new(stores), mem, event_bus)
    }

    fn mem(id: &str, scope: &str, status: &str) -> MemoryItem {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        MemoryItem {
            memory_id: id.into(),
            agent_scope: scope.into(),
            content_preview: format!("preview for {id}"),
            source_type: "test".into(),
            source_ref: format!("file:{id}.md"),
            created_at_unix: now,
            updated_at_unix: now,
            hit_count: 0,
            status: status.into(),
            is_pinned: false,
            pinned_by: String::new(),
            pinned_at_unix: 0,
        }
    }

    #[tokio::test]
    async fn test_memory_server_get_404() {
        let (server, _) = fresh_server();
        let err = server
            .get(Request::new(GetMemoryRequest {
                memory_id: "does-not-exist".into(),
            }))
            .await
            .expect_err("expect not_found");
        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_memory_server_list_returns_seeded() {
        let (server, mem_store) = fresh_server();
        mem_store
            .seed_for_tests(vec![
                mem("a", "scope", "active"),
                mem("b", "scope", "active"),
            ])
            .unwrap();
        let resp = server
            .list(Request::new(ListMemoryRequest {
                agent_id: "".into(),
                scope: "".into(),
                namespace: "".into(),
                include_soft_deleted: false,
            }))
            .await
            .unwrap();
        assert_eq!(resp.into_inner().items.len(), 2);
    }

    #[tokio::test]
    async fn test_memory_server_pin_persists_and_emits_audit() {
        let (server, mem_store) = fresh_server();
        mem_store.seed_for_tests(vec![mem("p", "s", "active")]).unwrap();
        server
            .pin(Request::new(PinMemoryRequest {
                memory_id: "p".into(),
                pin: true,
                actor: String::new(),
            }))
            .await
            .expect("pin ok");
        assert!(mem_store.get("p").unwrap().unwrap().is_pinned);
        // Verify audit emitted at least one MemoryPin event
        let audit = server.stores.audit.as_ref().unwrap().lock().unwrap();
        let count = audit
            .count_by_operation(AuditOperation::MemoryPin)
            .expect("count ok");
        assert!(count >= 1, "MemoryPin audit event expected");
    }

    /// TEST-27.1.3: pin RPC writes the actor ("console-api") through to the
    /// store + memory_to_pb projects pinned_by / pinned_at_unix on get.
    #[tokio::test]
    async fn test_memory_server_pin_writes_actor_and_projects() {
        let (server, mem_store) = fresh_server();
        mem_store.seed_for_tests(vec![mem("pa", "s", "active")]).unwrap();
        server
            .pin(Request::new(PinMemoryRequest {
                memory_id: "pa".into(),
                pin: true,
                actor: String::new(),
            }))
            .await
            .expect("pin ok");
        // Store wrote the actor + timestamp.
        let stored = mem_store.get("pa").unwrap().unwrap();
        assert_eq!(stored.pinned_by, "console-api");
        assert!(stored.pinned_at_unix > 0);
        // RPC get projects the add-only fields onto the wire MemoryItem.
        let pb = server
            .get(Request::new(GetMemoryRequest {
                memory_id: "pa".into(),
            }))
            .await
            .expect("get ok")
            .into_inner();
        assert_eq!(pb.pinned_by, "console-api");
        assert!(pb.pinned_at_unix > 0);
        // unpin clears.
        server
            .pin(Request::new(PinMemoryRequest {
                memory_id: "pa".into(),
                pin: false,
                actor: String::new(),
            }))
            .await
            .expect("unpin ok");
        let cleared = mem_store.get("pa").unwrap().unwrap();
        assert_eq!(cleared.pinned_by, "");
        assert_eq!(cleared.pinned_at_unix, 0);
    }

    // TEST-40.1.1 (task-40.1): proto add-only PinMemoryRequest.actor = field 3. prost wire encoding:
    // tag = (field_number << 3) | wire_type. actor is field 3, string (wire type 2, length-delimited)
    // → tag (3<<3)|2 = 26 = 0x1A, then length 1, then 'x' (0x78). Existing memory_id=1 / pin=2 field
    // numbers are unchanged (add-only, ADR-015 D1).
    #[test]
    fn test_pin_actor_proto_field_number() {
        use prost::Message;
        let req = PinMemoryRequest {
            actor: "x".into(),
            ..Default::default()
        };
        assert_eq!(
            req.encode_to_vec(),
            vec![0x1A, 0x01, 0x78],
            "actor must be field 3 (length-delimited string)"
        );
    }

    // TEST-40.1.2 (task-40.1 / ADR-045 D1): pin RPC propagates a non-empty actor through to the
    // store's pinned_by. An empty actor falls back to "console-api" (byte-equivalent default,
    // covered by TEST-27.1.3 above).
    #[tokio::test]
    async fn test_memory_server_pin_propagates_actor() {
        let (server, mem_store) = fresh_server();
        mem_store
            .seed_for_tests(vec![mem("pb", "s", "active")])
            .unwrap();
        server
            .pin(Request::new(PinMemoryRequest {
                memory_id: "pb".into(),
                pin: true,
                actor: "alice".into(),
            }))
            .await
            .expect("pin ok");
        let stored = mem_store.get("pb").unwrap().unwrap();
        assert_eq!(
            stored.pinned_by, "alice",
            "non-empty actor propagated to pinned_by"
        );
    }

    #[tokio::test]
    async fn test_memory_server_deprecate_persists_and_emits_audit() {
        let (server, mem_store) = fresh_server();
        mem_store.seed_for_tests(vec![mem("d", "s", "active")]).unwrap();
        server
            .deprecate(Request::new(DeprecateMemoryRequest {
                memory_id: "d".into(),
            }))
            .await
            .unwrap();
        assert_eq!(mem_store.get("d").unwrap().unwrap().status, "deprecated");
        let audit = server.stores.audit.as_ref().unwrap().lock().unwrap();
        assert!(audit.count_by_operation(AuditOperation::MemoryDeprecate).unwrap() >= 1);
    }

    #[tokio::test]
    async fn test_memory_server_soft_delete_persists_and_emits_audit() {
        let (server, mem_store) = fresh_server();
        mem_store.seed_for_tests(vec![mem("x", "s", "active")]).unwrap();
        server
            .soft_delete(Request::new(SoftDeleteMemoryRequest {
                memory_id: "x".into(),
            }))
            .await
            .unwrap();
        assert_eq!(mem_store.get("x").unwrap().unwrap().status, "soft_deleted");
        let audit = server.stores.audit.as_ref().unwrap().lock().unwrap();
        assert!(audit
            .count_by_operation(AuditOperation::MemorySoftDelete)
            .unwrap()
            >= 1);
    }

    // =====================================================================
    // task-15.2 (Phase 15 P0 #2 / ADR-021) — memory.* → EventBus bridge tests.
    // =====================================================================

    /// Helper: drain EventBus broadcast receiver into a Vec (best-effort,
    /// returns whatever is already buffered when `try_recv` exhausts).
    fn drain_events(
        rx: &mut tokio::sync::broadcast::Receiver<PbEvent>,
    ) -> Vec<PbEvent> {
        let mut out = Vec::new();
        while let Ok(evt) = rx.try_recv() {
            out.push(evt);
        }
        out
    }

    #[tokio::test]
    async fn test_pin_emits_event_bus_memory_pin() {
        let (server, mem_store, bus) = fresh_server_with_event_bus();
        mem_store.seed_for_tests(vec![mem("p", "s", "active")]).unwrap();
        // Subscribe BEFORE invoking pin so we don't miss the broadcast.
        let mut rx = bus.subscribe();
        server
            .pin(Request::new(PinMemoryRequest {
                memory_id: "p".into(),
                pin: true,
                actor: String::new(),
            }))
            .await
            .expect("pin ok");
        // Allow the broadcast to settle (sync; no spawn here, but be defensive).
        tokio::task::yield_now().await;
        let events = drain_events(&mut rx);
        assert_eq!(events.len(), 1, "exactly one memory.pin event expected");
        let evt = &events[0];
        assert_eq!(evt.event_type, "memory.pin");
        assert_eq!(evt.severity, "info");
        assert_eq!(evt.source, "contextforge-core");
        assert!(evt.message.contains("memory pin: p"), "message: {}", evt.message);
        assert!(evt.payload_json.contains("\"op\":\"pin\""), "payload: {}", evt.payload_json);
        assert!(evt.payload_json.contains("\"memory_id\":\"p\""), "payload: {}", evt.payload_json);
        assert!(evt.trace_id.is_none());
        assert!(evt.job_id.is_none());
    }

    #[tokio::test]
    async fn test_unpin_emits_event_bus_memory_pin_with_op_unpin() {
        let (server, mem_store, bus) = fresh_server_with_event_bus();
        mem_store
            .seed_for_tests(vec![mem("q", "s", "active")])
            .unwrap();
        let mut rx = bus.subscribe();
        // Pin first so unpin has a target — fire both ops, then assert second
        // event has op=unpin.
        server
            .pin(Request::new(PinMemoryRequest {
                memory_id: "q".into(),
                pin: true,
                actor: String::new(),
            }))
            .await
            .unwrap();
        server
            .pin(Request::new(PinMemoryRequest {
                memory_id: "q".into(),
                pin: false,
                actor: String::new(),
            }))
            .await
            .unwrap();
        tokio::task::yield_now().await;
        let events = drain_events(&mut rx);
        assert_eq!(events.len(), 2);
        assert!(events[0].payload_json.contains("\"op\":\"pin\""));
        assert!(events[1].payload_json.contains("\"op\":\"unpin\""));
        // Both share event_type=memory.pin per ADR-021 D2.
        assert_eq!(events[0].event_type, "memory.pin");
        assert_eq!(events[1].event_type, "memory.pin");
    }

    #[tokio::test]
    async fn test_deprecate_emits_event_bus_memory_deprecate() {
        let (server, mem_store, bus) = fresh_server_with_event_bus();
        mem_store.seed_for_tests(vec![mem("d", "s", "active")]).unwrap();
        let mut rx = bus.subscribe();
        server
            .deprecate(Request::new(DeprecateMemoryRequest {
                memory_id: "d".into(),
            }))
            .await
            .unwrap();
        tokio::task::yield_now().await;
        let events = drain_events(&mut rx);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "memory.deprecate");
        assert!(events[0].payload_json.contains("\"op\":\"deprecate\""));
        assert!(events[0].payload_json.contains("\"memory_id\":\"d\""));
    }

    #[tokio::test]
    async fn test_soft_delete_emits_event_bus_memory_soft_delete() {
        let (server, mem_store, bus) = fresh_server_with_event_bus();
        mem_store.seed_for_tests(vec![mem("x", "s", "active")]).unwrap();
        let mut rx = bus.subscribe();
        server
            .soft_delete(Request::new(SoftDeleteMemoryRequest {
                memory_id: "x".into(),
            }))
            .await
            .unwrap();
        tokio::task::yield_now().await;
        let events = drain_events(&mut rx);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "memory.soft_delete");
        assert!(events[0].payload_json.contains("\"op\":\"soft_delete\""));
    }

    /// AC3: when EventBus has no subscribers, SendError is swallowed and the
    /// state-op return is unaffected. Audit still emitted.
    #[tokio::test]
    async fn test_pin_swallows_send_error_when_no_subscriber() {
        let (server, mem_store, _bus) = fresh_server_with_event_bus();
        mem_store.seed_for_tests(vec![mem("ns", "s", "active")]).unwrap();
        // Do NOT subscribe → bus.send returns SendError.
        let resp = server
            .pin(Request::new(PinMemoryRequest {
                memory_id: "ns".into(),
                pin: true,
                actor: String::new(),
            }))
            .await;
        assert!(resp.is_ok(), "pin should succeed despite SendError");
        // Audit still recorded.
        let audit = server.stores.audit.as_ref().unwrap().lock().unwrap();
        let count = audit
            .count_by_operation(AuditOperation::MemoryPin)
            .expect("count ok");
        assert!(count >= 1, "MemoryPin audit event expected");
    }

    /// Sanity-check the helper mapping: non-memory ops (e.g. Import) return
    /// None → emit_audit_and_event would skip the EventBus path entirely.
    #[test]
    fn test_audit_op_to_event_type_filters_non_memory() {
        assert_eq!(audit_op_to_event_type(AuditOperation::MemoryPin), Some("memory.pin"));
        assert_eq!(audit_op_to_event_type(AuditOperation::MemoryUnpin), Some("memory.pin"));
        assert_eq!(
            audit_op_to_event_type(AuditOperation::MemoryDeprecate),
            Some("memory.deprecate")
        );
        assert_eq!(
            audit_op_to_event_type(AuditOperation::MemorySoftDelete),
            Some("memory.soft_delete")
        );
        assert_eq!(
            audit_op_to_event_type(AuditOperation::MemoryHardDelete),
            Some("memory.hard_delete")
        );
        assert_eq!(audit_op_to_event_type(AuditOperation::Import), None);
        assert_eq!(audit_op_to_event_type(AuditOperation::Search), None);
    }

    /// TEST-27.2.3: explicit unpin RPC = set_pinned(false) + emit MemoryUnpin;
    /// idempotent (unpin already-unpinned item Ok); pin toggle unaffected.
    #[tokio::test]
    async fn test_memory_server_unpin_explicit_and_idempotent() {
        let (server, mem_store) = fresh_server();
        mem_store.seed_for_tests(vec![mem("u", "s", "active")]).unwrap();
        // pin then explicit unpin.
        server
            .pin(Request::new(PinMemoryRequest { memory_id: "u".into(), pin: true, actor: String::new() }))
            .await
            .unwrap();
        server
            .unpin(Request::new(UnpinMemoryRequest { memory_id: "u".into() }))
            .await
            .expect("unpin ok");
        let got = mem_store.get("u").unwrap().unwrap();
        assert!(!got.is_pinned);
        assert_eq!(got.pinned_by, "");
        // idempotent: unpin already-unpinned → still Ok.
        server
            .unpin(Request::new(UnpinMemoryRequest { memory_id: "u".into() }))
            .await
            .expect("idempotent unpin ok");
        let audit = server.stores.audit.as_ref().unwrap().lock().unwrap();
        assert!(
            audit.count_by_operation(AuditOperation::MemoryUnpin).unwrap() >= 2,
            "two unpin audit events expected"
        );
    }

    /// TEST-27.2.2: hard_delete RPC physically removes the row (get-by-id None
    /// afterwards) + emits MemoryHardDelete audit.
    #[tokio::test]
    async fn test_memory_server_hard_delete_physical_and_audit() {
        let (server, mem_store) = fresh_server();
        mem_store.seed_for_tests(vec![mem("h", "s", "active")]).unwrap();
        server
            .hard_delete(Request::new(HardDeleteMemoryRequest { memory_id: "h".into() }))
            .await
            .expect("hard_delete ok");
        // physically gone.
        assert!(mem_store.get("h").unwrap().is_none(), "row physically removed");
        // RPC get → NotFound.
        let err = server
            .get(Request::new(GetMemoryRequest { memory_id: "h".into() }))
            .await
            .expect_err("expect not_found");
        assert_eq!(err.code(), tonic::Code::NotFound);
        // audit emitted.
        let audit = server.stores.audit.as_ref().unwrap().lock().unwrap();
        assert!(
            audit.count_by_operation(AuditOperation::MemoryHardDelete).unwrap() >= 1,
            "MemoryHardDelete audit event expected"
        );
    }
}
