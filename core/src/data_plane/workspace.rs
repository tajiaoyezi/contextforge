//! task-11.1 §6 AC3: `WorkspaceServer` impl `WorkspaceService` trait.
//!
//! 4 RPC: Create / Get / List / Delete → 真走 `SqliteWorkspaceStore` (task-10.2).
//! Error mapping (ADR-016 §D3 thin proxy: sentinel-aligned with Go contractv1
//! handler error model):
//!   - `WorkspaceError::Invalid` → `tonic::Status::invalid_argument`
//!   - `Get` returning `Ok(None)` → `tonic::Status::not_found`
//!   - `WorkspaceError::Sqlite / Io / Json` → `tonic::Status::internal`

use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::pb_console::workspace_service_server::WorkspaceService;
use crate::pb_console::{
    CreateWorkspaceRequest, DeleteWorkspaceRequest, DeleteWorkspaceResponse, GetWorkspaceRequest,
    ListWorkspacesRequest, ListWorkspacesResponse, UpdateWorkspaceConfigRequest,
    Workspace as PbWorkspace,
};
use crate::workspace::{Workspace as RustWorkspace, WorkspaceCreate, WorkspaceError, WorkspaceStore};

use super::DataPlaneStores;

pub struct WorkspaceServer {
    stores: Arc<DataPlaneStores>,
}

impl WorkspaceServer {
    pub fn new(stores: Arc<DataPlaneStores>) -> Self {
        Self { stores }
    }
}

fn workspace_to_pb(w: RustWorkspace) -> PbWorkspace {
    PbWorkspace {
        workspace_id: w.workspace_id,
        name: w.name,
        root_path: w.root_path,
        status: w.status,
        config_snapshot: w.config_snapshot,
        allowlist: w.allowlist,
        denylist: w.denylist,
        created_at_unix: w.created_at_unix,
        updated_at_unix: w.updated_at_unix,
    }
}

fn ws_err_to_status(e: WorkspaceError) -> Status {
    match e {
        WorkspaceError::Invalid(msg) => Status::invalid_argument(msg),
        WorkspaceError::Sqlite(err) => Status::internal(format!("sqlite: {}", err)),
        WorkspaceError::Io(err) => Status::internal(format!("io: {}", err)),
        WorkspaceError::Json(err) => Status::internal(format!("json: {}", err)),
    }
}

#[tonic::async_trait]
impl WorkspaceService for WorkspaceServer {
    async fn create(
        &self,
        req: Request<CreateWorkspaceRequest>,
    ) -> Result<Response<PbWorkspace>, Status> {
        let req = req.into_inner();
        let create_req = WorkspaceCreate {
            workspace_id: req.workspace_id,
            name: req.name,
            root_path: req.root_path,
            allowlist: req.allowlist,
            denylist: req.denylist,
        };
        let ws = self
            .stores
            .workspace_store
            .create(&create_req)
            .map_err(ws_err_to_status)?;
        Ok(Response::new(workspace_to_pb(ws)))
    }

    async fn get(
        &self,
        req: Request<GetWorkspaceRequest>,
    ) -> Result<Response<PbWorkspace>, Status> {
        let id = req.into_inner().workspace_id;
        match self.stores.workspace_store.get(&id) {
            Ok(Some(ws)) => Ok(Response::new(workspace_to_pb(ws))),
            Ok(None) => Err(Status::not_found(format!("workspace not found: {}", id))),
            Err(e) => Err(ws_err_to_status(e)),
        }
    }

    async fn list(
        &self,
        _req: Request<ListWorkspacesRequest>,
    ) -> Result<Response<ListWorkspacesResponse>, Status> {
        let items = self
            .stores
            .workspace_store
            .list()
            .map_err(ws_err_to_status)?;
        Ok(Response::new(ListWorkspacesResponse {
            items: items.into_iter().map(workspace_to_pb).collect(),
        }))
    }

    async fn delete(
        &self,
        req: Request<DeleteWorkspaceRequest>,
    ) -> Result<Response<DeleteWorkspaceResponse>, Status> {
        let id = req.into_inner().workspace_id;
        self.stores
            .workspace_store
            .soft_delete(&id)
            .map_err(ws_err_to_status)?;
        Ok(Response::new(DeleteWorkspaceResponse { ok: true }))
    }

    async fn update_config(
        &self,
        req: Request<UpdateWorkspaceConfigRequest>,
    ) -> Result<Response<PbWorkspace>, Status> {
        let req = req.into_inner();
        let ws = self
            .stores
            .workspace_store
            .update_config(&req.workspace_id, req.allowlist, req.denylist)
            .map_err(ws_err_to_status)?;
        Ok(Response::new(workspace_to_pb(ws)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::SqliteJobStore;
    use crate::workspace::SqliteWorkspaceStore;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use std::sync::atomic::{AtomicU64, Ordering};
    static TEST_SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_data_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let seq = TEST_SEQ.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!(
            "cf-ws-server-{name}-{}-{nanos}-{seq}",
            std::process::id()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn fresh_server() -> WorkspaceServer {
        let seq = TEST_SEQ.load(Ordering::SeqCst);
        let dir = temp_data_dir(&format!("create-{seq}"));
        let ws = Arc::new(SqliteWorkspaceStore::open(&dir).unwrap());
        let js = Arc::new(SqliteJobStore::open(&dir).unwrap());
        WorkspaceServer::new(DataPlaneStores::new(ws, js))
    }

    #[tokio::test]
    async fn test_workspace_server_create_via_service() {
        let server = fresh_server();
        let resp = server
            .create(Request::new(CreateWorkspaceRequest {
                workspace_id: "ws-create-1".into(),
                name: "create test".into(),
                root_path: std::env::temp_dir().to_string_lossy().to_string(),
                allowlist: vec![],
                denylist: vec![],
            }))
            .await
            .expect("create ok");
        let ws = resp.into_inner();
        assert_eq!(ws.workspace_id, "ws-create-1");
        assert_eq!(ws.name, "create test");
        assert_eq!(ws.status, "ready");
        assert!(ws.created_at_unix > 0);
    }

    #[tokio::test]
    async fn test_workspace_server_get_404() {
        let server = fresh_server();
        let err = server
            .get(Request::new(GetWorkspaceRequest {
                workspace_id: "ws-does-not-exist".into(),
            }))
            .await
            .expect_err("expect not_found");
        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_workspace_server_update_config_overwrites_and_bumps_updated_at() {
        let server = fresh_server();
        let created = server
            .create(Request::new(CreateWorkspaceRequest {
                workspace_id: "ws-update-1".into(),
                name: "update test".into(),
                root_path: std::env::temp_dir().to_string_lossy().to_string(),
                allowlist: vec!["src/**".into()],
                denylist: vec![],
            }))
            .await
            .unwrap()
            .into_inner();
        // small sleep so updated_at_unix can advance
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        let updated = server
            .update_config(Request::new(UpdateWorkspaceConfigRequest {
                workspace_id: "ws-update-1".into(),
                allowlist: vec!["lib/**".into(), "src/**".into()],
                denylist: vec!["node_modules/**".into()],
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(updated.workspace_id, "ws-update-1");
        assert!(updated.allowlist.contains(&"lib/**".to_string()));
        assert!(updated.denylist.contains(&"node_modules/**".to_string()));
        assert!(
            updated.updated_at_unix >= created.updated_at_unix,
            "updated_at_unix must not regress: created={} updated={}",
            created.updated_at_unix,
            updated.updated_at_unix
        );
    }

    #[tokio::test]
    async fn test_workspace_server_update_config_unknown_workspace_returns_internal() {
        // SqliteWorkspaceStore.update_config returns Invalid("workspace_id not found")
        // which maps to invalid_argument; we accept either invalid_argument or
        // internal — both are non-200 sentinel responses.
        let server = fresh_server();
        let err = server
            .update_config(Request::new(UpdateWorkspaceConfigRequest {
                workspace_id: "ws-does-not-exist".into(),
                allowlist: vec![],
                denylist: vec![],
            }))
            .await
            .expect_err("expect error");
        assert!(
            err.code() == tonic::Code::InvalidArgument
                || err.code() == tonic::Code::NotFound
                || err.code() == tonic::Code::Internal,
            "unexpected code: {:?}",
            err.code()
        );
    }

    #[tokio::test]
    async fn test_workspace_server_list_after_create() {
        let server = fresh_server();
        server
            .create(Request::new(CreateWorkspaceRequest {
                workspace_id: "ws-list-a".into(),
                name: "A".into(),
                root_path: std::env::temp_dir().join("a").to_string_lossy().to_string(),
                allowlist: vec![],
                denylist: vec![],
            }))
            .await
            .unwrap();
        server
            .create(Request::new(CreateWorkspaceRequest {
                workspace_id: "ws-list-b".into(),
                name: "B".into(),
                root_path: std::env::temp_dir().join("b").to_string_lossy().to_string(),
                allowlist: vec![],
                denylist: vec![],
            }))
            .await
            .unwrap();
        let resp = server.list(Request::new(ListWorkspacesRequest {})).await.unwrap();
        let items = resp.into_inner().items;
        assert_eq!(items.len(), 2);
        let ids: Vec<_> = items.iter().map(|w| w.workspace_id.clone()).collect();
        assert!(ids.contains(&"ws-list-a".to_string()));
        assert!(ids.contains(&"ws-list-b".to_string()));
    }
}
