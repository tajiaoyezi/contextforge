//! task-1.3: tonic gRPC server + health.
//!
//! - AC1: listen on local gRPC — built-in default is loopback `127.0.0.1`;
//!   a wildcard / `0.0.0.0` (or `::`) bind is rejected (PRD Local service
//!   security baseline). `ListenAddr::Unix` is modeled for the daemon to
//!   request later (task-1.4); task-1.3 serves loopback TCP.
//! - AC2: `ContextService.Health` -> `HealthResponse{status:"SERVING"}`.
//! - AC3: tonic + tokio + serde wired, proto via tonic codegen, no FFI.
//! - `Search` returns `Status::unimplemented` (Phase 2+; out of scope here).

use std::net::SocketAddr;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tonic::{Request, Response, Status};

use crate::pb::context_service_server::{ContextService, ContextServiceServer};
use crate::pb::{HealthRequest, HealthResponse, SearchRequest, SearchResponse};

/// Built-in safe default listen address (loopback only, never `0.0.0.0`).
pub const DEFAULT_LISTEN: &str = "127.0.0.1:50551";

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
        // AC2: core is up -> SERVING (Go daemon health-checks this in task-1.4).
        Ok(Response::new(HealthResponse {
            status: "SERVING".to_string(),
        }))
    }

    async fn search(
        &self,
        _req: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        // Out of scope for task-1.3 (retrieval is Phase 2+/Phase 4).
        Err(Status::unimplemented(
            "Search is Phase 2+ (task-1.3 ships the gRPC skeleton only)",
        ))
    }
}

/// AC3: assemble the tonic code-generated server for `CoreService`.
pub fn context_service() -> ContextServiceServer<CoreService> {
    ContextServiceServer::new(CoreService)
}

/// AC1: resolve a *safe* listen address.
///
/// - `None` -> built-in loopback default (`DEFAULT_LISTEN`).
/// - `"unix:/path"` -> `ListenAddr::Unix`.
/// - `"<ip>:<port>"` -> `ListenAddr::Tcp`, **unless** the ip is unspecified
///   (`0.0.0.0` / `::`), which is rejected.
pub fn resolve_listen_addr(arg: Option<&str>) -> Result<ListenAddr, AddrError> {
    let s = arg.unwrap_or(DEFAULT_LISTEN);

    if let Some(path) = s.strip_prefix("unix:") {
        if path.is_empty() {
            return Err(AddrError("empty unix socket path".to_string()));
        }
        return Ok(ListenAddr::Unix(PathBuf::from(path)));
    }

    let sock: SocketAddr = s
        .parse()
        .map_err(|_| AddrError(format!("not a valid socket address: {s}")))?;
    if sock.ip().is_unspecified() {
        return Err(AddrError(format!(
            "refusing wildcard bind {s}: 0.0.0.0/:: is forbidden \
             (use 127.0.0.1, ::1, or unix:<path>)"
        )));
    }
    Ok(ListenAddr::Tcp(sock))
}

/// AC1/AC2: bind `addr` and serve `ContextService`.
///
/// task-1.3 serves loopback TCP; `ListenAddr::Unix` is intentionally deferred
/// to task-1.4 daemon wiring (AC1 is satisfied via the 127.0.0.1 path).
pub async fn serve(addr: ListenAddr) -> Result<(), Box<dyn std::error::Error>> {
    match addr {
        ListenAddr::Tcp(sock) => {
            tonic::transport::Server::builder()
                .add_service(context_service())
                .serve(sock)
                .await?;
            Ok(())
        }
        ListenAddr::Unix(path) => Err(Box::new(AddrError(format!(
            "unix socket serving ({}) is deferred to task-1.4 daemon wiring; \
             task-1.3 serves loopback TCP",
            path.display()
        )))),
    }
}
