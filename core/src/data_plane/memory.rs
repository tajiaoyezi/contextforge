//! task-13.1 (ADR-017 D1 Wave 3): `MemoryServer` impl `MemoryService` trait.
//!
//! 5 RPC: List / Get / Pin / Deprecate / SoftDelete → 真走 `SqliteMemoryStore`
//! (task-13.1) + Pin / Deprecate / SoftDelete each emit one audit event via
//! the shared `AuditSink` so Console UI's audit log panel surfaces the trace.
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
    DeprecateMemoryRequest, DeprecateMemoryResponse, GetMemoryRequest, ListMemoryRequest,
    ListMemoryResponse, MemoryItem as PbMemoryItem, PinMemoryRequest, PinMemoryResponse,
    SoftDeleteMemoryRequest, SoftDeleteMemoryResponse,
};

use super::DataPlaneStores;

pub struct MemoryServer {
    stores: Arc<DataPlaneStores>,
}

impl MemoryServer {
    pub fn new(stores: Arc<DataPlaneStores>) -> Self {
        Self { stores }
    }

    fn emit_audit(&self, op: AuditOperation, memory_id: &str) {
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
    }
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
        memory
            .set_pinned(&req.memory_id, req.pin)
            .map_err(mem_err_to_status)?;
        self.emit_audit(
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
        self.emit_audit(AuditOperation::MemoryDeprecate, &id);
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
        self.emit_audit(AuditOperation::MemorySoftDelete, &id);
        Ok(Response::new(SoftDeleteMemoryResponse {}))
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
}
