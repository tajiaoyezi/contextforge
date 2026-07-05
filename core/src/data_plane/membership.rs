//! task-52.2 (Phase 52 / ADR-053): MembershipService gRPC handler — 3-role RBAC
//! membership management.
//!
//! Backs the 4 MembershipService RPCs (AddMember / RemoveMember / ListMembers /
//! GetMyRole) with [`crate::membership::SqliteMembershipStore`] (task-52.1). The
//! store is opened lazily on first use from `stores.data_dir` (the same dir the
//! other SQLite stores live in) — this mirrors the [`super::user::UserServer`]
//! pattern and avoids threading a new field through every `DataPlaneStores`
//! constructor. When `data_dir` is empty (task-11.1 baseline / unit tests that
//! don't set it), RPCs return `failed_precondition`.
//!
//! Store error → tonic Status mapping:
//! - [`MembershipStoreError::Duplicate`] → `already_exists` (PK collision)
//! - [`MembershipStoreError::Invalid`] → `invalid_argument` (bad role / CHECK fail)
//! - other → `internal`
//!
//! This is the AuthZ seam: task-52.3 Go `roleMiddleware` calls `GetMyRole` over
//! gRPC and gates destructive ops on `role == "admin"` (ADR-053 D3).

use std::path::PathBuf;
use std::str::FromStr;

use tonic::{Request, Response, Status};

use crate::membership::{
    Member as StoreMember, MembershipStoreError, Role, SqliteMembershipStore,
};
use crate::pb_console::membership_service_server::MembershipService;
use crate::pb_console::{
    AddMemberRequest, GetMyRoleRequest, GetMyRoleResponse, ListMembersRequest,
    ListMembersResponse, Member, RemoveMemberRequest, RemoveMemberResponse,
};

pub struct MembershipServer {
    data_dir: PathBuf,
}

impl MembershipServer {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    /// Open the membership store on demand and run `f` against it. Per-call open
    /// is cheap (CREATE TABLE IF NOT EXISTS is idempotent on warm boot) and
    /// sidesteps the non-Clone `SqliteMembershipStore` lifetime issue. When
    /// `data_dir` is empty (task-11.1 baseline / unit tests), returns
    /// `failed_precondition`. Mirrors [`super::user::UserServer::with_store`].
    #[allow(clippy::result_large_err)]
    fn with_store<F, R>(&self, f: F) -> Result<R, Status>
    where
        F: FnOnce(&SqliteMembershipStore) -> Result<R, Status>,
    {
        if self.data_dir.as_os_str().is_empty() {
            return Err(Status::failed_precondition(
                "MembershipService: data_dir not configured (task-11.1 baseline / unit test)",
            ));
        }
        let store = SqliteMembershipStore::open(&self.data_dir)
            .map_err(|e| Status::internal(format!("open membership store: {e}")))?;
        f(&store)
    }
}

fn store_member_to_pb(m: &StoreMember) -> Member {
    Member {
        workspace_id: m.workspace_id.clone(),
        user_id: m.user_id.clone(),
        role: m.role.as_str().to_string(),
        created_at_unix: m.created_at_unix,
    }
}

/// Map a [`MembershipStoreError`] to a tonic [`Status`]:
/// - `Duplicate` (PK collision) → `already_exists`
/// - `Invalid` (bad role / CHECK fail) → `invalid_argument`
/// - other (sqlite / io) → `internal`
fn map_store_err(err: MembershipStoreError) -> Status {
    match err {
        MembershipStoreError::Duplicate(what) => {
            Status::already_exists(format!("duplicate {what}"))
        }
        MembershipStoreError::Invalid(msg) => Status::invalid_argument(msg),
        other => Status::internal(format!("membership store: {other}")),
    }
}

#[tonic::async_trait]
#[allow(clippy::result_large_err)]
impl MembershipService for MembershipServer {
    async fn add_member(&self, req: Request<AddMemberRequest>) -> Result<Response<Member>, Status> {
        let req = req.into_inner();
        self.with_store(|store| {
            // Parse the role at the API boundary so a bad role surfaces as
            // invalid_argument rather than reaching the DB CHECK.
            let role = Role::from_str(&req.role).map_err(map_store_err)?;
            let member = store
                .add_member(&req.workspace_id, &req.user_id, role)
                .map_err(map_store_err)?;
            Ok(Response::new(store_member_to_pb(&member)))
        })
    }

    async fn remove_member(
        &self,
        req: Request<RemoveMemberRequest>,
    ) -> Result<Response<RemoveMemberResponse>, Status> {
        let req = req.into_inner();
        self.with_store(|store| {
            // Idempotent: Ok(()) whether or not the row existed.
            store
                .remove_member(&req.workspace_id, &req.user_id)
                .map_err(map_store_err)?;
            Ok(Response::new(RemoveMemberResponse {}))
        })
    }

    async fn list_members(
        &self,
        req: Request<ListMembersRequest>,
    ) -> Result<Response<ListMembersResponse>, Status> {
        let req = req.into_inner();
        self.with_store(|store| {
            let members = store
                .list_members(&req.workspace_id)
                .map_err(map_store_err)?;
            Ok(Response::new(ListMembersResponse {
                members: members.iter().map(store_member_to_pb).collect(),
            }))
        })
    }

    async fn get_my_role(
        &self,
        req: Request<GetMyRoleRequest>,
    ) -> Result<Response<GetMyRoleResponse>, Status> {
        let req = req.into_inner();
        self.with_store(|store| {
            // Non-members resolve to "" (not a member) rather than not_found:
            // the Go roleMiddleware treats "no role" as deny (ADR-053 D3), so
            // a 200 with empty role is the cleanest contract.
            let role = store
                .get_role(&req.workspace_id, &req.user_id)
                .map_err(map_store_err)?;
            let role_str = role.map(|r| r.as_str().to_string()).unwrap_or_default();
            Ok(Response::new(GetMyRoleResponse { role: role_str }))
        })
    }
}

// TEST-52.2.2 / AC2: MembershipService gRPC round-trip
// (AddMember → ListMembers → GetMyRole; RemoveMember → GetMyRole returns "")
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_data_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "cf-membership-svc-{}-{}-{nanos}",
            label,
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn test_52_2_2_add_list_get_role_roundtrip() {
        let dir = temp_data_dir("roundtrip");
        let svc = MembershipServer::new(dir);

        // add admin
        let added = svc
            .add_member(Request::new(AddMemberRequest {
                workspace_id: "ws-1".into(),
                user_id: "u-admin".into(),
                role: "admin".into(),
            }))
            .await
            .expect("add admin ok")
            .into_inner();
        assert_eq!(added.workspace_id, "ws-1");
        assert_eq!(added.user_id, "u-admin");
        assert_eq!(added.role, "admin");
        assert!(added.created_at_unix > 0);

        // add a second member
        svc.add_member(Request::new(AddMemberRequest {
            workspace_id: "ws-1".into(),
            user_id: "u-member".into(),
            role: "member".into(),
        }))
        .await
        .expect("add member ok");

        // list
        let listed = svc
            .list_members(Request::new(ListMembersRequest {
                workspace_id: "ws-1".into(),
            }))
            .await
            .expect("list ok")
            .into_inner();
        assert_eq!(listed.members.len(), 2);
        assert_eq!(listed.members[0].user_id, "u-admin");
        assert_eq!(listed.members[0].role, "admin");

        // get_my_role for a member → "member"
        let role = svc
            .get_my_role(Request::new(GetMyRoleRequest {
                workspace_id: "ws-1".into(),
                user_id: "u-member".into(),
            }))
            .await
            .expect("get role ok")
            .into_inner();
        assert_eq!(role.role, "member");

        // get_my_role for a non-member → "" (not not_found)
        let none = svc
            .get_my_role(Request::new(GetMyRoleRequest {
                workspace_id: "ws-1".into(),
                user_id: "u-nobody".into(),
            }))
            .await
            .expect("get role ok for non-member")
            .into_inner();
        assert_eq!(none.role, "", "non-member resolves to empty role");
    }

    #[tokio::test]
    async fn test_52_2_2b_remove_then_get_role_empty() {
        let dir = temp_data_dir("remove");
        let svc = MembershipServer::new(dir);

        svc.add_member(Request::new(AddMemberRequest {
            workspace_id: "ws-2".into(),
            user_id: "u-viewer".into(),
            role: "viewer".into(),
        }))
        .await
        .expect("add viewer ok");

        // remove → get_my_role returns ""
        svc.remove_member(Request::new(RemoveMemberRequest {
            workspace_id: "ws-2".into(),
            user_id: "u-viewer".into(),
        }))
        .await
        .expect("remove ok");

        let role = svc
            .get_my_role(Request::new(GetMyRoleRequest {
                workspace_id: "ws-2".into(),
                user_id: "u-viewer".into(),
            }))
            .await
            .expect("get role after remove")
            .into_inner();
        assert_eq!(role.role, "", "role empty after removal");

        // remove is idempotent: removing again is Ok
        svc.remove_member(Request::new(RemoveMemberRequest {
            workspace_id: "ws-2".into(),
            user_id: "u-viewer".into(),
        }))
        .await
        .expect("remove absent (idempotent)");
    }

    #[tokio::test]
    async fn test_52_2_2c_duplicate_already_exists() {
        let dir = temp_data_dir("dup");
        let svc = MembershipServer::new(dir);
        svc.add_member(Request::new(AddMemberRequest {
            workspace_id: "ws-3".into(),
            user_id: "u1".into(),
            role: "admin".into(),
        }))
        .await
        .expect("first add");
        let dup = svc
            .add_member(Request::new(AddMemberRequest {
                workspace_id: "ws-3".into(),
                user_id: "u1".into(),
                role: "member".into(),
            }))
            .await;
        assert!(dup.is_err());
        assert_eq!(dup.unwrap_err().code(), tonic::Code::AlreadyExists);
    }

    #[tokio::test]
    async fn test_52_2_2d_invalid_role_invalid_argument() {
        let dir = temp_data_dir("badrole");
        let svc = MembershipServer::new(dir);
        let err = svc
            .add_member(Request::new(AddMemberRequest {
                workspace_id: "ws-4".into(),
                user_id: "u1".into(),
                role: "superuser".into(),
            }))
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_52_2_2e_empty_data_dir_failed_precondition() {
        let svc = MembershipServer::new(PathBuf::new());
        let err = svc
            .list_members(Request::new(ListMembersRequest {
                workspace_id: "ws-x".into(),
            }))
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::FailedPrecondition);
    }
}
