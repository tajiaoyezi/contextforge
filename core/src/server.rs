//! task-1.3 / task-6.1: tonic gRPC server + health + search wire.
//!
//! - AC1 (task-1.3): listen on local gRPC — built-in default is loopback
//!   `127.0.0.1`; a wildcard / `0.0.0.0` (or `::`) bind is rejected (PRD
//!   Local service security baseline). `ListenAddr::Unix` is modeled for
//!   the daemon to request later (task-1.4); task-1.3 serves loopback TCP.
//! - AC2 (task-1.3): `ContextService.Health` -> `HealthResponse{status:"SERVING"}`.
//! - AC3 (task-1.3): tonic + tokio + serde wired, proto via tonic codegen.
//! - AC1 (task-6.1): `ContextService.Search` wire — replaces the task-1.3
//!   `Status::unimplemented` placeholder with a real `Retriever::open` →
//!   `retriever.search/explain` → `SearchResponse` pipeline.

use std::net::SocketAddr;
use std::path::PathBuf;

use prost_types::Timestamp;
use serde::{Deserialize, Serialize};
use tonic::{Request, Response, Status};

use crate::chunker::Provenance as RetrieverProvenance;
use crate::pb::context_service_server::{ContextService, ContextServiceServer};
use crate::pb::{
    HealthRequest, HealthResponse, Provenance as PbProvenance, RetrievalResult, SearchRequest,
    SearchResponse,
};
use crate::retriever::{
    Retriever, RetrieverError, SearchFilters as RetrieverFilters, SearchOptions, SearchResult,
};

/// Built-in safe default listen address (loopback only, never `0.0.0.0`).
pub const DEFAULT_LISTEN: &str = "127.0.0.1:50551";

/// gRPC service impl for the data plane.
///
/// task-1.3 = skeleton + health; task-6.1 = `Search` wired through
/// `Retriever`. `data_dir` is the on-disk root the retriever opens
/// (`[data_dir]/collections/[id]/{metadata.sqlite, tantivy/}`), set by
/// `main.rs` via `resolve_data_dir` (cmd-arg / env / `~/.contextforge`).
/// `Default::default()` (used by task-1.3/1.4 tests that only exercise
/// Health) yields an empty path; calling `search()` against it will return
/// `FailedPrecondition` from `Retriever::open`, matching task-6.1 §5.3.
#[derive(Debug, Default, Clone)]
pub struct CoreService {
    pub data_dir: PathBuf,
}

impl CoreService {
    /// task-6.1 §5.3: explicit constructor — `main.rs` injects `data_dir`.
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }
}

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

    /// task-6.1 §5.3 AC1: SearchRequest → Retriever.search/explain → SearchResponse.
    ///
    /// Error mapping (per task-4.1/4.2 `RetrieverError` 5 variants):
    ///   `collections` empty                            → `InvalidArgument`
    ///   `Io(NotFound)` / `CollectionNotFound`          → `FailedPrecondition`
    ///   `InvalidConfig`                                → `InvalidArgument`
    ///   `Sqlite` / `Tantivy` / `Io(non-NotFound)`      → `Internal`
    async fn search(
        &self,
        req: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        let req = req.into_inner();

        if req.collections.is_empty() {
            return Err(Status::invalid_argument(
                "collections is required (v0.1 single-collection)",
            ));
        }
        let collection_id = &req.collections[0];

        let retriever = match Retriever::open(&self.data_dir, collection_id) {
            Ok(r) => r,
            Err(RetrieverError::CollectionNotFound(_)) => {
                return Err(Status::failed_precondition(format!(
                    "collection not found: {}",
                    collection_id
                )));
            }
            Err(RetrieverError::Io(io_err))
                if io_err.kind() == std::io::ErrorKind::NotFound =>
            {
                return Err(Status::failed_precondition(format!(
                    "data dir / collection path missing: {}",
                    io_err
                )));
            }
            Err(RetrieverError::InvalidConfig(s)) => {
                return Err(Status::invalid_argument(s));
            }
            Err(e) => return Err(Status::internal(e.to_string())),
        };

        let filters_pb = req.filters.unwrap_or_default();
        let top_k = if req.top_k <= 0 {
            10
        } else {
            req.top_k as usize
        };
        let opts = SearchOptions {
            query: req.query,
            top_k,
            filters: RetrieverFilters {
                source_type: filters_pb.source_type,
                language: filters_pb.language,
                agent_scope: req.agent_scope,
                ..Default::default()
            },
            explain: req.explain,
        };

        let results = if req.explain {
            retriever.explain(&opts)
        } else {
            retriever.search(&opts)
        }
        .map_err(|e| Status::internal(format!("retrieve: {}", e)))?;

        Ok(Response::new(SearchResponse {
            results: results.iter().map(search_result_to_proto).collect(),
        }))
    }
}

/// task-6.1 §5.3: `chunker::Provenance` → `proto::Provenance` field mapping.
///
/// **§2A 决策 E placeholder**: `chunker::Provenance.imported_at` /
/// `source_modified_at` are `String` (RFC3339 with Z; indexer SQLite TEXT
/// 直存). v0.1 P0 returns `prost_types::Timestamp::default()` for both proto
/// fields (chrono not in Cargo.toml — R7 strict channel; CLI text mode reads
/// the original `String` directly from `chunker::Provenance` so users still
/// see RFC3339; `--json` shows 1970-01-01 placeholder). task-6.3 may trigger
/// SPEC-DRIFT to add chrono/time crate via main-agent R7 chore-dep PR.
fn provenance_to_proto(p: &RetrieverProvenance) -> PbProvenance {
    PbProvenance {
        importer: p.importer.clone(),
        original_path: p.original_path.clone(),
        imported_at: Some(Timestamp::default()),
        source_modified_at: Some(Timestamp::default()),
    }
}

/// task-6.1 §5.3: `retriever::SearchResult` → `proto::RetrievalResult`
/// (12 explainable fields 1:1 + provenance list).
fn search_result_to_proto(r: &SearchResult) -> RetrievalResult {
    RetrievalResult {
        chunk_id: r.chunk_id.clone(),
        context_id: r.context_id.clone(),
        source_type: r.source_type.clone(),
        file_path: r.file_path.clone(),
        line_start: r.line_start as i64,
        line_end: r.line_end as i64,
        score: r.score as f64,
        retrieval_method: r.retrieval_method.clone(),
        reason: r.reason.clone(),
        agent_scope: r.agent_scope.clone(),
        redaction_status: r.redaction_status.clone(),
        provenance: r.provenance.iter().map(provenance_to_proto).collect(),
    }
}

/// AC3 (task-1.3): assemble the tonic code-generated server for `CoreService`
/// (`Default::default()` — empty `data_dir`; only safe for Health-only callers).
pub fn context_service() -> ContextServiceServer<CoreService> {
    ContextServiceServer::new(CoreService::default())
}

/// task-6.1: assemble the tonic server with an explicit `data_dir`.
/// Phase-6 smoke (`core/tests/phase6_smoke.rs`) uses this to drive end-to-end
/// gRPC Search against a fixture index. `main.rs` calls this on startup.
pub fn context_service_with_data_dir(data_dir: PathBuf) -> ContextServiceServer<CoreService> {
    ContextServiceServer::new(CoreService::new(data_dir))
}

/// task-6.1 §5.3: resolve the on-disk data root for `CoreService::search`.
///
/// Priority (matches Go-side `config::DefaultRootDir` semantics):
///   1. `arg` (the 2nd cmd-line argument to `contextforge-core`, when present)
///   2. `$CONTEXTFORGE_DATA_DIR`
///   3. `$HOME/.contextforge` (Unix) / `%USERPROFILE%\.contextforge` (Windows)
///   4. `./.contextforge` (last-resort fallback if no env / home is set)
pub fn resolve_data_dir(arg: Option<&str>) -> PathBuf {
    if let Some(a) = arg {
        let trimmed = a.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    if let Ok(env) = std::env::var("CONTEXTFORGE_DATA_DIR") {
        if !env.trim().is_empty() {
            return PathBuf::from(env);
        }
    }
    let home_env = if cfg!(target_os = "windows") {
        "USERPROFILE"
    } else {
        "HOME"
    };
    match std::env::var(home_env) {
        Ok(h) if !h.trim().is_empty() => PathBuf::from(h).join(".contextforge"),
        _ => PathBuf::from(".contextforge"),
    }
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

/// AC1/AC2 (task-1.3): bind `addr` and serve `ContextService` with a
/// default-constructed service (Health-only callers). task-6.1 introduces
/// [`serve_with_service`] for callers that need to inject `data_dir` for
/// the search wire.
pub async fn serve(addr: ListenAddr) -> Result<(), Box<dyn std::error::Error>> {
    serve_with_service(addr, CoreService::default()).await
}

/// task-6.1: bind `addr` and serve the provided `CoreService` (with its
/// configured `data_dir`). `main.rs` calls this with a service constructed
/// from `resolve_data_dir`.
pub async fn serve_with_service(
    addr: ListenAddr,
    svc: CoreService,
) -> Result<(), Box<dyn std::error::Error>> {
    match addr {
        ListenAddr::Tcp(sock) => {
            tonic::transport::Server::builder()
                .add_service(ContextServiceServer::new(svc))
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
