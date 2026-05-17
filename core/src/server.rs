//! task-1.3: tonic gRPC server skeleton + health.
//!
//! - AC1: listen on local gRPC — Unix socket or 127.0.0.1; a wildcard /
//!   `0.0.0.0` bind is rejected (PRD Local service security baseline).
//! - AC2: `ContextService.Health` -> `HealthResponse{status:"SERVING"}`.
//! - AC3: tonic + tokio + serde wired, proto via tonic codegen, no FFI.
//! - `Search` stays `Status::unimplemented` (Phase 2+; out of scope here).
//!
//! NOTE: §2.5.1 RED skeleton — signatures compile, bodies deliberately
//! `unimplemented!()` so TEST-1.3.* fail functionally (not by compile error).

use std::net::SocketAddr;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tonic::{Request, Response, Status};

use crate::pb::context_service_server::{ContextService, ContextServiceServer};
use crate::pb::{HealthRequest, HealthResponse, SearchRequest, SearchResponse};

/// gRPC service impl for the data plane (task-1.3 = skeleton; Search is Phase 2+).
#[derive(Debug, Default, Clone)]
pub struct CoreService;

/// Where `contextforge-core` listens. Never a wildcard / `0.0.0.0` bind (AC1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ListenAddr {
    Unix(PathBuf),
    Tcp(SocketAddr),
}

/// Listen-address resolution error (e.g. a forbidden `0.0.0.0` bind).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddrError(pub String);

impl std::fmt::Display for AddrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid listen address: {}", self.0)
    }
}
impl std::error::Error for AddrError {}

#[tonic::async_trait]
impl ContextService for CoreService {
    async fn health(
        &self,
        _req: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        unimplemented!("task-1.3 GREEN: ContextService.Health -> SERVING")
    }

    async fn search(
        &self,
        _req: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        unimplemented!("task-1.3 GREEN: Search is Phase 2+ -> Status::unimplemented")
    }
}

/// AC3: assemble the tonic code-generated server for `CoreService`.
pub fn context_service() -> ContextServiceServer<CoreService> {
    unimplemented!("task-1.3 GREEN: ContextServiceServer::new(CoreService)")
}

/// AC1: resolve a *safe* listen address. `None` -> built-in loopback default.
/// Any unspecified / `0.0.0.0` bind is rejected.
pub fn resolve_listen_addr(_arg: Option<&str>) -> Result<ListenAddr, AddrError> {
    unimplemented!("task-1.3 GREEN: default loopback / reject 0.0.0.0")
}

/// AC1/AC2: bind `addr` and serve `ContextService` until the task is dropped.
pub async fn serve(_addr: ListenAddr) -> Result<(), Box<dyn std::error::Error>> {
    unimplemented!("task-1.3 GREEN: tonic Server::builder().add_service(..).serve(..)")
}
