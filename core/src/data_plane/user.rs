//! task-50.2 (Phase 50 / ADR-051): UserService gRPC handler — per-user identity foundation.
//!
//! Backs the 3 UserService RPCs (Create / GetByToken / List) with `SqliteUserStore`.
//! The store is opened lazily on first use from `stores.data_dir` (the same dir
//! the other SQLite stores live in) — this avoids threading a new field through every
//! `DataPlaneStores` constructor. When `data_dir` is empty (task-11.1 baseline /
//! unit tests that don't set it), RPCs return `failed_precondition`.
//!
//! Closes `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]` on the
//! storage + access path; task-50.3 redeems the marker once Go bearer resolution
//! overrides the declared actor with the verified id.

use tonic::{Request, Response, Status};

use crate::identity::{SqliteUserStore, User as StoreUser, UserStoreError};
use crate::pb_console::user_service_server::UserService;
use crate::pb_console::{
    CreateUserRequest, GetUserByTokenRequest, ListUsersRequest, ListUsersResponse, User,
};

pub struct UserServer {
    data_dir: std::path::PathBuf,
}

impl UserServer {
    pub fn new(data_dir: std::path::PathBuf) -> Self {
        Self { data_dir }
    }

    /// Open the user store on demand and run `f` against it. Per-call open is cheap
    /// (CREATE TABLE IF NOT EXISTS is idempotent on warm boot) and sidesteps the
    /// non-Clone `SqliteUserStore` lifetime issue. When `data_dir` is empty
    /// (task-11.1 baseline / unit tests), returns `failed_precondition`.
    #[allow(clippy::result_large_err)]
    fn with_store<F, R>(&self, f: F) -> Result<R, Status>
    where
        F: FnOnce(&SqliteUserStore) -> Result<R, Status>,
    {
        if self.data_dir.as_os_str().is_empty() {
            return Err(Status::failed_precondition(
                "UserService: data_dir not configured (task-11.1 baseline / unit test)",
            ));
        }
        let store = SqliteUserStore::open(&self.data_dir)
            .map_err(|e| Status::internal(format!("open user store: {e}")))?;
        f(&store)
    }
}

fn store_user_to_pb(u: &StoreUser) -> User {
    User {
        id: u.id.clone(),
        name: u.name.clone(),
        token: u.token.clone(),
        created_at_unix: u.created_at_unix,
    }
}

fn map_store_err(err: UserStoreError) -> Status {
    match err {
        UserStoreError::Duplicate(what) => Status::already_exists(format!("duplicate {what}")),
        other => Status::internal(format!("user store: {other}")),
    }
}

#[tonic::async_trait]
#[allow(clippy::result_large_err)]
impl UserService for UserServer {
    async fn create(&self, req: Request<CreateUserRequest>) -> Result<Response<User>, Status> {
        let req = req.into_inner();
        self.with_store(|store| {
            let user = store
                .create(StoreUser {
                    id: req.id,
                    name: req.name,
                    token: req.token,
                    created_at_unix: 0,
                })
                .map_err(map_store_err)?;
            Ok(Response::new(store_user_to_pb(&user)))
        })
    }

    async fn get_by_token(
        &self,
        req: Request<GetUserByTokenRequest>,
    ) -> Result<Response<User>, Status> {
        let req = req.into_inner();
        self.with_store(|store| {
            match store.get_by_token(&req.token).map_err(map_store_err)? {
                Some(user) => Ok(Response::new(store_user_to_pb(&user))),
                None => Err(Status::not_found("user not found for token")),
            }
        })
    }

    async fn list(&self, req: Request<ListUsersRequest>) -> Result<Response<ListUsersResponse>, Status> {
        let _ = req.into_inner(); // no fields
        self.with_store(|store| {
            let users = store.list().map_err(map_store_err)?;
            Ok(Response::new(ListUsersResponse {
                users: users.iter().map(store_user_to_pb).collect(),
            }))
        })
    }
}

// TEST-50.2.2 / AC2: UserService gRPC round-trip (create → get-by-token)
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
            "cf-user-svc-{}-{}-{nanos}",
            label,
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn test_50_2_2_create_get_by_token_roundtrip() {
        let dir = temp_data_dir("roundtrip");
        let svc = UserServer::new(dir);

        // create
        let created = svc
            .create(Request::new(CreateUserRequest {
                id: "u-rt".into(),
                name: "roundtrip user".into(),
                token: "tok-rt".into(),
            }))
            .await
            .expect("create ok")
            .into_inner();
        assert_eq!(created.id, "u-rt");
        assert_eq!(created.token, "tok-rt");
        assert!(created.created_at_unix > 0);

        // get-by-token
        let got = svc
            .get_by_token(Request::new(GetUserByTokenRequest {
                token: "tok-rt".into(),
            }))
            .await
            .expect("get ok")
            .into_inner();
        assert_eq!(got.id, "u-rt");
        assert_eq!(got.name, "roundtrip user");

        // unknown token → not_found
        let miss = svc
            .get_by_token(Request::new(GetUserByTokenRequest {
                token: "nope".into(),
            }))
            .await;
        assert!(miss.is_err());
        let err = miss.unwrap_err();
        assert_eq!(err.code(), tonic::Code::NotFound);

        // list
        let listed = svc
            .list(Request::new(ListUsersRequest {}))
            .await
            .expect("list ok")
            .into_inner();
        assert_eq!(listed.users.len(), 1);
        assert_eq!(listed.users[0].id, "u-rt");
    }

    #[tokio::test]
    async fn test_50_2_2b_duplicate_token_already_exists() {
        let dir = temp_data_dir("dup");
        let svc = UserServer::new(dir);
        svc.create(Request::new(CreateUserRequest {
            id: "u1".into(),
            name: "a".into(),
            token: "dup".into(),
        }))
        .await
        .expect("first create");
        let dup = svc
            .create(Request::new(CreateUserRequest {
                id: "u2".into(),
                name: "b".into(),
                token: "dup".into(),
            }))
            .await;
        assert!(dup.is_err());
        assert_eq!(dup.unwrap_err().code(), tonic::Code::AlreadyExists);
    }

    #[tokio::test]
    async fn test_50_2_2c_empty_data_dir_failed_precondition() {
        let svc = UserServer::new(PathBuf::new());
        let err = svc
            .list(Request::new(ListUsersRequest {}))
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::FailedPrecondition);
    }
}
