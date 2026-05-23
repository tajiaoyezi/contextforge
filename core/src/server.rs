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

// ============================================================================
// task-6.1 RED tests — CoreService::search wire 单元 (TEST-6.1.1 Rust 端).
// 用 in-memory tempdir Retriever 验真实拿到 12 字段（不走 tonic transport；
// 端到端 transport 走 core/tests/phase6_smoke.rs = TEST-6.1.5 / AC5）.
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunker::ChunkPolicy;
    use crate::indexer::IndexSession;
    use crate::pb::context_service_server::ContextService as CSTrait;
    use crate::pb::SearchRequest;
    use crate::scanner::{default_denylist, ScanOptions};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tonic::Request;

    fn temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "contextforge-server-{name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn build_fixture(name: &str, files: &[(&str, &str)]) -> (PathBuf, String) {
        let src = temp_root(&format!("{name}-src"));
        let data = temp_root(&format!("{name}-data"));
        let coll = format!("test-{}", name);
        for (rel, body) in files {
            let p = src.join(rel);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&p, body).unwrap();
        }
        let scan_opts = ScanOptions {
            denylist: default_denylist(),
            allowlist: Vec::new(),
            allow_denylist_override: false,
            dry_run: false,
            max_file_bytes: 10 * 1024 * 1024,
        };
        let mut sess = IndexSession::open(&data, &coll).expect("open indexer");
        sess.index_path(&src, &scan_opts, &ChunkPolicy::default(), vec![])
            .expect("index_path");
        sess.commit().expect("commit");
        (data, coll)
    }

    // ---- TEST-6.1.1 / SCEN-6.1.1 / AC1 — search wire 端到端拿 12 字段 ----
    #[tokio::test]
    async fn test_6_1_1_search_wire_returns_12_field_results() {
        let (data, coll) = build_fixture(
            "ac1-wire",
            &[(
                "readme.md",
                "# Readme\n\nunique token wire6n1zmark in body.\n",
            )],
        );
        let svc = CoreService::new(data);
        let req = SearchRequest {
            query: "wire6n1zmark".into(),
            collections: vec![coll],
            agent_scope: vec![],
            top_k: 10,
            filters: None,
            explain: true,
        };
        let resp = svc.search(Request::new(req)).await.expect("search ok");
        let inner = resp.into_inner();
        assert!(
            !inner.results.is_empty(),
            "AC1 wire: 应有命中（unique token in fixture）"
        );
        let r = &inner.results[0];

        // 12 explainable fields PRESENT + 内容 sanity
        assert!(!r.chunk_id.is_empty(), "AC1: chunk_id non-empty");
        assert_eq!(r.context_id, "", "AC1 §2A v0.1 schema gap default");
        assert_eq!(r.source_type, "", "AC1 §2A v0.1 schema gap default");
        assert!(!r.file_path.is_empty(), "AC1: file_path non-empty");
        assert!(r.line_end >= r.line_start, "AC1: line range valid");
        assert!(r.score > 0.0, "AC1: score > 0, got {}", r.score);
        assert_eq!(r.retrieval_method, "bm25", "AC1: method=bm25");
        assert!(!r.reason.is_empty(), "AC1 explain=true: reason 非空");
        assert!(r.agent_scope.is_empty(), "AC1 §2A v0.1 default empty");
        assert_eq!(
            r.redaction_status, "applied",
            "AC1 §2A v0.1 default 'applied'"
        );
        assert!(
            !r.provenance.is_empty(),
            "AC3 黑盒守护：provenance.len() ≥ 1"
        );
    }

    // ---- TEST-6.1.1b — collections 为空 → InvalidArgument ----
    #[tokio::test]
    async fn test_6_1_1_empty_collections_returns_invalid_argument() {
        let svc = CoreService::default();
        let req = SearchRequest {
            query: "x".into(),
            collections: vec![],
            agent_scope: vec![],
            top_k: 1,
            filters: None,
            explain: false,
        };
        let err = svc.search(Request::new(req)).await.unwrap_err();
        assert_eq!(
            err.code(),
            tonic::Code::InvalidArgument,
            "AC1 wire: 空 collections 应 InvalidArgument, got {:?}",
            err.code()
        );
    }

    // ---- TEST-6.1.1c — 未知 collection → FailedPrecondition ----
    #[tokio::test]
    async fn test_6_1_1_unknown_collection_returns_failed_precondition() {
        let data = temp_root("ac1-unknown");
        let svc = CoreService::new(data);
        let req = SearchRequest {
            query: "x".into(),
            collections: vec!["nonexistent-collection".into()],
            agent_scope: vec![],
            top_k: 1,
            filters: None,
            explain: false,
        };
        let err = svc.search(Request::new(req)).await.unwrap_err();
        assert_eq!(
            err.code(),
            tonic::Code::FailedPrecondition,
            "AC1 wire: 未知 collection 应 FailedPrecondition, got {:?}",
            err.code()
        );
    }
}
