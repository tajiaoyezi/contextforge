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
use std::path::{Path, PathBuf};
use std::sync::Arc;

use prost_types::Timestamp;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use crate::chunker::{ChunkPolicy, Provenance as ChunkerProvenance, Provenance as RetrieverProvenance};
use crate::indexer::{IndexProgressSnapshot, IndexSession};
use crate::pb::context_service_server::{ContextService, ContextServiceServer};
use crate::pb::{
    ChunkContent, HealthRequest, HealthResponse, IndexProgress, IndexRequest,
    ListAllChunksRequest, ListAllChunksResponse, Provenance as PbProvenance, RetrievalResult,
    SearchRequest, SearchResponse,
};
use crate::embedding::{select_provider, DeterministicEmbeddingProvider};
use crate::retriever::vector::select_vector_backend;
use crate::retriever::{
    is_chunk_id_format, Retriever, RetrieverError, SearchFilters as RetrieverFilters,
    SearchOptions, SearchResult,
};
use crate::scanner::{default_denylist, ScanOptions};

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

/// task-9.2 §5.3：Index server-stream associated type — mpsc 包装。
pub type IndexProgressStream = ReceiverStream<Result<IndexProgress, Status>>;

/// task-9.2 §5.3：mpsc channel 容量（per-file progress emit；32 是 tonic stream
/// 推荐默认 — buffer slow-consumer 一小段同时不过度内存占用）。
const INDEX_PROGRESS_CHANNEL_CAPACITY: usize = 32;

#[tonic::async_trait]
impl ContextService for CoreService {
    type IndexStream = IndexProgressStream;

    /// task-9.2 §5.3：Index 真实现。SCAN_PATH 模式：
    ///   1. 校验 source_path 非空 + 路径存在 → 否则 `InvalidArgument` 流建立前抛出
    ///   2. fallback data_dir (空 → self.data_dir) + collection_id (空 → "default")
    ///   3. 创建 mpsc(32) channel
    ///   4. spawn_blocking 跑同步 `IndexSession::index_path_with_progress`，回调内
    ///      `tx.blocking_send` 喂 `IndexProgress` 进 stream
    ///   5. 完成 / 错误：发最后一条 `done=true` (error 字段空 / 含 indexer err str)
    ///   6. return `Response(ReceiverStream(rx))`
    /// 错误映射：校验阶段 → `Status::invalid_argument`；indexer 内部错误 → in-band
    /// 经 final IndexProgress.error 字段传递（不中断 stream — client 看 done=true
    /// && error != "" 即知失败）
    async fn index(
        &self,
        request: Request<IndexRequest>,
    ) -> Result<Response<Self::IndexStream>, Status> {
        let req = request.into_inner();

        let source_path = req.source_path.trim().to_string();
        if source_path.is_empty() {
            return Err(Status::invalid_argument("source_path is required"));
        }
        if !Path::new(&source_path).exists() {
            return Err(Status::invalid_argument(format!(
                "source_path does not exist: {}",
                source_path
            )));
        }

        let data_dir: PathBuf = if req.data_dir.trim().is_empty() {
            self.data_dir.clone()
        } else {
            PathBuf::from(req.data_dir.trim())
        };
        if data_dir.as_os_str().is_empty() {
            return Err(Status::invalid_argument(
                "data_dir must be provided when CoreService has no default",
            ));
        }
        let collection_id: String = if req.collection_id.trim().is_empty() {
            "default".to_string()
        } else {
            req.collection_id.trim().to_string()
        };

        let (tx, rx) =
            tokio::sync::mpsc::channel::<Result<IndexProgress, Status>>(INDEX_PROGRESS_CHANNEL_CAPACITY);

        let src = PathBuf::from(&source_path);
        let tx_indexer = tx.clone();
        tokio::task::spawn_blocking(move || {
            let mut session = match IndexSession::open(&data_dir, &collection_id) {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx_indexer.blocking_send(Ok(IndexProgress {
                        files_processed: 0,
                        files_skipped_denied: 0,
                        files_skipped_redaction: 0,
                        chunks_written: 0,
                        current_file: String::new(),
                        done: true,
                        error: format!("open IndexSession: {}", e),
                    }));
                    return;
                }
            };

            let scan_opts = ScanOptions {
                denylist: default_denylist(),
                allowlist: Vec::new(),
                allow_denylist_override: false,
                dry_run: false,
                max_file_bytes: 10 * 1024 * 1024,
            };
            let policy = ChunkPolicy::default();
            let provenance: Vec<ChunkerProvenance> = Vec::new();

            let result = session.index_path_with_progress(
                &src,
                &scan_opts,
                &policy,
                provenance,
                |snap: &IndexProgressSnapshot<'_>| {
                    let progress = IndexProgress {
                        files_processed: snap.files_processed as i64,
                        files_skipped_denied: snap.files_skipped_denied as i64,
                        files_skipped_redaction: snap.files_skipped_redaction as i64,
                        chunks_written: snap.chunks_written as i64,
                        current_file: snap
                            .current_file
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                        done: false,
                        error: String::new(),
                    };
                    // best-effort: if client dropped, indexer continues to finish
                    // current file (mpsc send failure is logged via final message).
                    let _ = tx_indexer.blocking_send(Ok(progress));
                },
            );

            // Final message: emit done=true with error if any. Always attempt
            // to commit Tantivy writer before reporting done (mirrors task-2.4
            // contract — indexer is only durable after commit).
            let (final_stats, error_str) = match result {
                Ok(stats) => match session.commit() {
                    Ok(()) => (stats, String::new()),
                    Err(e) => (stats, format!("commit: {}", e)),
                },
                Err(e) => (
                    crate::indexer::IndexStats::default(),
                    format!("index_path: {}", e),
                ),
            };

            let _ = tx_indexer.blocking_send(Ok(IndexProgress {
                files_processed: final_stats.files_indexed as i64,
                files_skipped_denied: final_stats.files_skipped_denied as i64,
                files_skipped_redaction: final_stats.files_skipped_redaction as i64,
                chunks_written: final_stats.chunks_written as i64,
                current_file: String::new(),
                done: true,
                error: error_str,
            }));
            drop(tx_indexer);
        });

        drop(tx);
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn health(
        &self,
        _req: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        // AC2: core is up -> SERVING (Go daemon health-checks this in task-1.4).
        Ok(Response::new(HealthResponse {
            status: "SERVING".to_string(),
        }))
    }

    /// task-31.3 (ADR-036 D3): full-text chunk listing for the exporter. Reads every chunk's real
    /// content from the collection's SQLite store (no embedder / vector wiring needed) so the
    /// exporter can fill real `content` + a real `ContentHash` (vs the prior content=""). Add-only —
    /// the Search RPC / SearchResponse are unchanged.
    async fn list_all_chunks(
        &self,
        req: Request<ListAllChunksRequest>,
    ) -> Result<Response<ListAllChunksResponse>, Status> {
        let req = req.into_inner();
        let collection_id = if req.collection_id.trim().is_empty() {
            "default"
        } else {
            req.collection_id.as_str()
        };
        let retriever = match Retriever::open(&self.data_dir, collection_id) {
            Ok(r) => r,
            Err(RetrieverError::CollectionNotFound(_)) => {
                return Err(Status::failed_precondition(format!(
                    "collection not found: {}",
                    collection_id
                )));
            }
            Err(e) => return Err(Status::internal(format!("open collection: {}", e))),
        };
        let items = retriever
            .enumerate_chunks()
            .map_err(|e| Status::internal(format!("enumerate chunks: {}", e)))?;
        let chunks = items
            .into_iter()
            .map(|(chunk_id, content)| ChunkContent { chunk_id, content })
            .collect();
        Ok(Response::new(ListAllChunksResponse { chunks }))
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

        // task-6.2 §2A 决策 E fast-path: when the query shape looks like a
        // chunk_id (chunker §100 = "chk_<8-hex>_<ordinal>"), try the exact
        // SQLite lookup first; hit → single-result SearchResponse, no BM25
        // scoring. Miss → fall through to the original BM25 search (so
        // legitimate full-text queries that happen to look like chunk_ids
        // still get retrieval, and chunk_id-shaped strings that aren't real
        // chunk_ids degrade gracefully to BM25). REST `GET /v1/chunks/{id}`
        // is the primary user of this path; CLI search is mostly unaffected
        // because user queries rarely match the chunker format prefix.
        if is_chunk_id_format(&req.query) {
            match retriever.get_chunk(&req.query) {
                Ok(Some(hit)) => {
                    return Ok(Response::new(SearchResponse {
                        results: vec![search_result_to_proto(&hit)],
                    }));
                }
                Ok(None) => { /* fall through to BM25 */ }
                Err(e) => return Err(Status::internal(format!("get_chunk: {}", e))),
            }
        }

        // task-21.1: hybrid path (opt-in via SearchRequest.hybrid). RRF-fuses the BM25 and vector
        // result lists (core/src/retriever/fusion.rs). Wires the same model-free
        // DeterministicEmbeddingProvider + 0-dep BruteForceVectorBackend as the semantic path, builds
        // the on-demand index, then fuses. Results carry retrieval_method "hybrid" + hybrid_score (the
        // RRF score). Deterministic embeddings prove the fusion wiring, not recall quality (real recall
        // driving the ADR-025 ratify is task-21.3; ADR-013).
        if req.hybrid {
            let top_k = if req.top_k <= 0 { 10 } else { req.top_k as usize };
            let embedder = Arc::new(DeterministicEmbeddingProvider::default());
            // task-29.1: backend now comes from the factory. No vector config is plumbed to the
            // server yet, so default args ("", 0) — byte-equivalent to the hardcoded
            // BruteForceVectorBackend::new() (TEST-29.1.3).
            let backend = select_vector_backend("", 0)
                .map_err(|e| Status::internal(format!("hybrid vector backend: {}", e)))?;
            let wired = retriever
                .with_embedder(embedder)
                .with_vector_searcher(backend.clone());
            let items = wired
                .enumerate_chunks()
                .map_err(|e| Status::internal(format!("hybrid enumerate: {}", e)))?;
            wired
                .index_chunks_semantic(backend.as_ref(), &items)
                .map_err(|e| Status::internal(format!("hybrid index: {}", e)))?;
            let results = wired
                .search_hybrid(&req.query, top_k)
                .map_err(|e| Status::internal(format!("hybrid search: {}", e)))?;
            let proto_results = results
                .iter()
                .map(|r| {
                    let mut pr = search_result_to_proto(r);
                    pr.hybrid_score = r.score;
                    pr
                })
                .collect();
            return Ok(Response::new(SearchResponse {
                results: proto_results,
            }));
        }

        // task-19.3: semantic path (opt-in via SearchRequest.semantic). Wire the model-free
        // DeterministicEmbeddingProvider + the 0-dep BruteForceVectorBackend, build an in-memory
        // index from this collection's chunks on demand (no persistence yet —
        // [SPEC-DEFER:phase-future.hnsw-graph-persistence]), and run the vector path. Results carry
        // retrieval_method "vector" + vector_score + embedding_provider. Deterministic embeddings
        // prove the wiring, not recall (real recall is task-19.5; ADR-013).
        if req.semantic {
            let top_k = if req.top_k <= 0 { 10 } else { req.top_k as usize };
            // task-22.1: provider now comes from the factory. No embedding config is plumbed to the
            // server yet, so default args ("deterministic", 0) — byte-equivalent to the Phase 19
            // hardcoded DeterministicEmbeddingProvider::default() (TEST-22.1.2/22.1.5).
            let embedder = select_provider("deterministic", 0)
                .map_err(|e| Status::internal(format!("semantic embedder: {}", e)))?;
            // task-29.1: backend via factory (default "", 0 → byte-equivalent to the hardcoded
            // BruteForceVectorBackend::new(); TEST-29.1.3).
            let backend = select_vector_backend("", 0)
                .map_err(|e| Status::internal(format!("semantic vector backend: {}", e)))?;
            let wired = retriever
                .with_embedder(embedder.clone())
                .with_vector_searcher(backend.clone());
            let items = wired
                .enumerate_chunks()
                .map_err(|e| Status::internal(format!("semantic enumerate: {}", e)))?;
            wired
                .index_chunks_semantic(backend.as_ref(), &items)
                .map_err(|e| Status::internal(format!("semantic index: {}", e)))?;
            let results = wired
                .search_semantic(&req.query, top_k)
                .map_err(|e| Status::internal(format!("semantic search: {}", e)))?;
            let provider = embedder.name().to_string();
            let proto_results = results
                .iter()
                .map(|r| {
                    let mut pr = search_result_to_proto(r);
                    pr.vector_score = r.score;
                    pr.embedding_provider = provider.clone();
                    pr
                })
                .collect();
            return Ok(Response::new(SearchResponse {
                results: proto_results,
            }));
        }

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
        // task-19.3 add-only: BM25 path leaves these at proto3 defaults; the semantic path overrides.
        vector_score: 0.0,
        embedding_provider: String::new(),
        // task-21.1 add-only: BM25/vector paths leave this 0; the hybrid path overrides with the RRF score.
        hybrid_score: 0.0,
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
///   (`0.0.0.0` / `::`), which is rejected unless explicit opt-in via env
///   `CONTEXTFORGE_ALLOW_WILDCARD_BIND=1` (task-16.4 — docker / k8s
///   deployments where container network isolation makes wildcard bind safe).
pub fn resolve_listen_addr(arg: Option<&str>) -> Result<ListenAddr, AddrError> {
    let allow_wildcard =
        std::env::var("CONTEXTFORGE_ALLOW_WILDCARD_BIND").as_deref() == Ok("1");
    resolve_listen_addr_with_opts(arg, allow_wildcard)
}

/// Pure variant of [`resolve_listen_addr`] without env-var inspection. Used
/// in tests to drive both opt-in / opt-out paths deterministically without
/// racing on a process-global env var.
pub fn resolve_listen_addr_with_opts(
    arg: Option<&str>,
    allow_wildcard: bool,
) -> Result<ListenAddr, AddrError> {
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
    if sock.ip().is_unspecified() && !allow_wildcard {
        return Err(AddrError(format!(
            "refusing wildcard bind {s}: 0.0.0.0/:: is forbidden \
             (set CONTEXTFORGE_ALLOW_WILDCARD_BIND=1 to opt-in for \
              docker/k8s deployment, or use 127.0.0.1, ::1, or unix:<path>)"
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

/// task-11.1 §6 AC5 (ADR-016 §D2): bind `addr` and serve the full Console
/// data plane gRPC contract — Phase 9 `ContextServiceServer` + Phase 11 4
/// new `console_data_plane.v1` services (WorkspaceService / JobService /
/// SearchService / EventsService) on the **same** tonic `Server::builder`
/// chain so they share one port + auth boundary (ADR-013 cli-data-plane
/// pattern extended).
///
/// `data_dir` is used to construct `SqliteWorkspaceStore` (task-10.2) +
/// `SqliteJobStore` (task-10.3) — both share `<data_dir>/workspaces.db`.
/// If either store fails to open, the error is propagated up before any
/// listen-bind happens.
pub async fn serve_full(
    addr: ListenAddr,
    svc: CoreService,
    data_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::data_plane::{register_services, DataPlaneStores};
    use crate::jobs::{orphan_reaper, IndexSessionBackend, JobRunner, JobStore, SqliteJobStore};
    use crate::workspace::SqliteWorkspaceStore;
    use std::sync::Arc;

    match addr {
        ListenAddr::Tcp(sock) => {
            // task-11.1: open data plane stores up-front. Fail loud if SQLite
            // can't open (corrupt schema / permission denied) — daemon should
            // not silently start without business plane.
            let ws_store = Arc::new(SqliteWorkspaceStore::open(data_dir)?);
            let job_store = Arc::new(SqliteJobStore::open(data_dir)?);

            // task-11.3 §6 AC4: reap orphan jobs left over from previous boot
            // BEFORE binding the gRPC listener so no fresh Enqueue can see
            // a stale running row.
            let reaped = orphan_reaper(&job_store)?;
            if reaped > 0 {
                eprintln!("INFO orphan reaper: marked {} stale job(s) terminal at startup", reaped);
            }

            // task-11.4 §6 AC4: shared EventBus for indexing.progress /
            // indexing.cancelled / indexing.error emission + EventsService
            // .Subscribe broadcast stream.
            // task-26.3 (ADR-031 D5): capacity + partition are env-configurable
            // (CF_EVENT_BUS_CAPACITY / CF_EVENT_BUS_PARTITION); conservative
            // defaults (1000 / un-partitioned) keep task-11.4 behavior unchanged.
            let event_bus = crate::data_plane::events::EventBus::from_config(
                crate::data_plane::events::EventBusConfig::from_env(),
            );

            // task-11.3 §6 AC1/AC2 + task-11.4: real JobRunner backed by
            // IndexSessionBackend wired to EventBus.
            let indexer = IndexSessionBackend::with_event_bus(event_bus.clone());
            let job_store_dyn: Arc<dyn JobStore> = job_store.clone();
            let runner = Arc::new(JobRunner::new(job_store_dyn, indexer));

            // task-13.1 (ADR-017 D1 Wave 3): SqliteMemoryStore + AuditSink wired
            // into DataPlaneStores so MemoryService 5 RPC are backed by a real
            // store and Pin/Deprecate/SoftDelete emit audit events.
            let memory_store = std::sync::Arc::new(
                crate::memory::SqliteMemoryStore::open(data_dir)?,
            );
            let audit_sink = std::sync::Arc::new(std::sync::Mutex::new(
                crate::memoryops::audit::AuditSink::open(data_dir, "memory")?,
            ));
            // task-14.1 (ADR-017 D1 Wave 4): SqliteEvalStore opens
            // `<data_dir>/eval.db` and is wired into DataPlaneStores so
            // EvalService 3 RPC are backed by a real store.
            let eval_store = std::sync::Arc::new(
                crate::eval::SqliteEvalStore::open(data_dir)?,
            );
            // task-16.1 (Phase 16 P4 #10): SqliteTracePersist opens
            // `<data_dir>/search_traces.db` and is wired into DataPlaneStores
            // so SearchServer's in-memory TraceStore becomes write-through +
            // warm-restores on every daemon boot.
            let trace_persist = std::sync::Arc::new(
                crate::data_plane::search_persist::SqliteTracePersist::open(data_dir)?,
            );
            let stores = DataPlaneStores::full(
                ws_store,
                job_store,
                runner,
                data_dir.to_path_buf(),
                event_bus,
                Some(memory_store),
                Some(audit_sink),
                Some(eval_store),
                Some(trace_persist),
            );

            let mut builder = tonic::transport::Server::builder();
            let router = builder.add_service(ContextServiceServer::new(svc));
            let router = register_services(router, stores);
            router.serve(sock).await?;
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
            semantic: false,
            hybrid: false,
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

    // ---- TEST-19.3 — semantic=true dispatches the vector path ----
    // The opt-in semantic flag builds an on-demand index (DeterministicEmbeddingProvider +
    // BruteForceVectorBackend) and returns vector hits carrying retrieval_method "vector" +
    // embedding_provider + vector_score. Deterministic embeddings prove the dispatch/plumbing, not
    // recall quality (real recall is task-19.5; ADR-013).
    #[tokio::test]
    async fn test_19_3_semantic_dispatches_vector_path() {
        let (data, coll) = build_fixture(
            "semantic-dispatch",
            &[
                ("a.md", "where is the config loader and default data dir"),
                ("b.md", "how the daemon restarts after a crash"),
            ],
        );
        let svc = CoreService::new(data);
        let req = SearchRequest {
            query: "where is the config loader and default data dir".into(),
            collections: vec![coll],
            agent_scope: vec![],
            top_k: 5,
            filters: None,
            explain: false,
            semantic: true,
            hybrid: false,
        };
        let inner = svc
            .search(Request::new(req))
            .await
            .expect("semantic search ok")
            .into_inner();
        assert!(!inner.results.is_empty(), "semantic path should return hits");
        let top = &inner.results[0];
        assert_eq!(top.retrieval_method, "vector", "semantic hits use the vector method");
        assert_eq!(top.embedding_provider, "deterministic-sha256");
        assert!(
            top.vector_score.is_finite() && top.vector_score <= 1.001 && top.vector_score >= -1.001,
            "vector_score should be a valid cosine (±f32 eps), got {}",
            top.vector_score
        );
        assert!(!top.provenance.is_empty(), "AC3 provenance floor still holds on the vector path");
    }

    // ---- TEST-22.1.5 — AC5: the semantic path now builds its embedder via select_provider with
    // default args ("deterministic", 0). Behavior must not regress — provider stays
    // "deterministic-sha256", the vector method holds, and results stay byte-stable across calls
    // (the factory swap is a pure construction change). Default-arg byte-equivalence to the Phase 19
    // hardcoded default() is additionally proven at the embedding layer by TEST-22.1.2.
    #[tokio::test]
    async fn test_22_1_5_semantic_path_factory_default_no_regression() {
        let (data, coll) = build_fixture(
            "semantic-factory-default",
            &[
                ("a.md", "where is the config loader and default data dir"),
                ("b.md", "how the daemon restarts after a crash"),
            ],
        );
        let svc = CoreService::new(data);
        let mk = || SearchRequest {
            query: "where is the config loader and default data dir".into(),
            collections: vec![coll.clone()],
            agent_scope: vec![],
            top_k: 5,
            filters: None,
            explain: false,
            semantic: true,
            hybrid: false,
        };
        let first = svc
            .search(Request::new(mk()))
            .await
            .expect("factory-driven semantic ok")
            .into_inner();
        assert!(!first.results.is_empty(), "factory-driven semantic path should return hits");
        assert_eq!(first.results[0].retrieval_method, "vector");
        assert_eq!(
            first.results[0].embedding_provider, "deterministic-sha256",
            "default-arg factory must keep the deterministic provider"
        );
        let second = svc
            .search(Request::new(mk()))
            .await
            .expect("factory-driven semantic ok")
            .into_inner();
        let scores1: Vec<f32> = first.results.iter().map(|r| r.vector_score).collect();
        let scores2: Vec<f32> = second.results.iter().map(|r| r.vector_score).collect();
        assert_eq!(scores1, scores2, "factory-driven semantic results must be byte-stable across calls");
    }

    // ---- TEST-32.1.1 — parse_vector_backend maps env values (name + dim) to the factory args,
    // trimming whitespace and defaulting a blank/unparsable dim to 0; an unknown backend name flows
    // to the factory as an honest Err (never a silent BruteForce fallback, ADR-013). Pure parser →
    // no process-global env mutation, so it is race-free under the parallel test runner. ----
    #[test]
    fn test_32_1_1_parse_vector_backend_and_factory_honest_err() {
        assert_eq!(
            parse_vector_backend(Some("qdrant"), Some("384")),
            ("qdrant".to_string(), 384)
        );
        assert_eq!(
            parse_vector_backend(Some("  lancedb  "), Some(" 768 ")),
            ("lancedb".to_string(), 768),
            "name + dim are trimmed"
        );
        assert_eq!(
            parse_vector_backend(Some("sqlite-vec"), Some("")),
            ("sqlite-vec".to_string(), 0),
            "blank dim → 0"
        );
        assert_eq!(
            parse_vector_backend(Some("brute"), Some("notanumber")),
            ("brute".to_string(), 0),
            "unparsable dim → 0"
        );
        // an unknown backend name surfaces the factory's honest Err (no silent fallback)
        let (name, dim) = parse_vector_backend(Some("nope"), None);
        let err = select_vector_backend(&name, dim).unwrap_err();
        assert!(
            err.to_string().contains("nope"),
            "factory should echo the unknown backend name: {err}"
        );
    }

    // ---- TEST-32.1.2 — unset env (None/None) and blank names resolve to ("", 0) → BruteForce,
    // byte-equivalent to the Phase 29 hardcoded select_vector_backend("", 0) default. This is the
    // default-behavior-unchanged guard (ADR-004): an unset CONTEXTFORGE_VECTOR_BACKEND keeps the
    // hybrid/semantic hot paths on BruteForce exactly as before. ----
    #[test]
    fn test_32_1_2_default_unset_is_brute_force_byte_equiv() {
        let (name, dim) = parse_vector_backend(None, None);
        assert_eq!((name.as_str(), dim), ("", 0), "unset env → empty default arm");
        let backend = select_vector_backend(&name, dim).expect("default arm always builds");
        assert_eq!(
            backend.name(),
            "brute-force",
            "unset env → BruteForce (byte-equivalent to the Phase 29 hardcoded default)"
        );
        // a whitespace-only backend name also collapses to the empty default arm
        let (blank, bdim) = parse_vector_backend(Some("   "), None);
        assert_eq!((blank.as_str(), bdim), ("", 0), "blank name → empty default arm");
    }

    // ---- TEST-21.1.2 — hybrid=true dispatches the RRF fusion path ----
    // search_hybrid RRF-fuses BM25 + the (deterministic) vector path; results carry retrieval_method
    // "hybrid" + hybrid_score (the fused RRF score). Deterministic embeddings prove the fusion
    // dispatch/plumbing, not recall quality (real recall driving ADR-025 ratify is task-21.3; ADR-013).
    #[tokio::test]
    async fn test_21_1_hybrid_dispatches_fusion_path() {
        let (data, coll) = build_fixture(
            "hybrid-dispatch",
            &[
                ("a.md", "where is the config loader and default data dir"),
                ("b.md", "how the daemon restarts after a crash"),
            ],
        );
        let svc = CoreService::new(data);
        let req = SearchRequest {
            query: "where is the config loader and default data dir".into(),
            collections: vec![coll],
            agent_scope: vec![],
            top_k: 5,
            filters: None,
            explain: false,
            semantic: false,
            hybrid: true,
        };
        let inner = svc
            .search(Request::new(req))
            .await
            .expect("hybrid search ok")
            .into_inner();
        assert!(!inner.results.is_empty(), "hybrid path should return hits");
        let top = &inner.results[0];
        assert_eq!(top.retrieval_method, "hybrid", "hybrid hits use the hybrid method");
        assert!(
            top.hybrid_score > 0.0,
            "hybrid_score is the positive RRF score, got {}",
            top.hybrid_score
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
            semantic: false,
            hybrid: false,
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
            semantic: false,
            hybrid: false,
        };
        let err = svc.search(Request::new(req)).await.unwrap_err();
        assert_eq!(
            err.code(),
            tonic::Code::FailedPrecondition,
            "AC1 wire: 未知 collection 应 FailedPrecondition, got {:?}",
            err.code()
        );
    }

    // ============================================================================
    // task-6.2 §2A 决策 E — CoreService::search fast-path 走 retriever.get_chunk
    // 当 query 看起来像 chunk_id format (^[0-9a-fA-F]{16,}$) 时优先精确路径，
    // 未命中 fallback 到原有 BM25 全文 search（保留语义一致性）。
    // ============================================================================

    // ---- TEST-6.2.E3 — search fast-path: query=chunk_id 命中 → 单条返 ----
    //
    // chunker §`chunk_id` format = "chk_<8-hex>_<ordinal>" (core/src/chunker/mod.rs §100);
    // server.rs fast-path detector tests this prefix shape, not the spec's
    // illustrative `^[0-9a-f]{16,}$` regex. If retriever.is_chunk_id_format
    // recognises the seed chunk_id, search() takes the get_chunk fast-path;
    // otherwise it falls back to BM25 (TEST-6.2.E4).
    #[tokio::test]
    async fn test_6_2_e3_search_fast_path_on_chunk_id_format() {
        let (data, coll) = build_fixture(
            "ac2e-fastpath",
            &[("readme.md", "# Readme\nunique token fastpathmarker62z\n")],
        );
        // 1. Seed: 拿 fixture 真实 chunk_id（chunker 算出 "chk_<8-hex>_<ord>"）
        let retr = crate::retriever::Retriever::open(&data, &coll).expect("seed open");
        let seed = retr
            .search(&crate::retriever::SearchOptions {
                query: "fastpathmarker62z".into(),
                top_k: 1,
                filters: crate::retriever::SearchFilters::default(),
                explain: false,
            })
            .expect("seed search");
        assert!(!seed.is_empty(), "seed: fixture indexed");
        let chunk_id = seed[0].chunk_id.clone();
        // 2. chunker format sanity: chk_<8hex>_<ord> (chunker/mod.rs §100)
        assert!(
            chunk_id.starts_with("chk_"),
            "seed chunk_id={} 应以 'chk_' 开头 (chunker format)",
            chunk_id
        );
        // 3. detector helper 应识别此 chunk_id (will be added in GREEN)
        assert!(
            crate::retriever::is_chunk_id_format(&chunk_id),
            "seed chunk_id={} 应被 is_chunk_id_format 识别为 fast-path 候选",
            chunk_id
        );

        // 4. server.rs search with query = chunk_id → 走 fast-path 返单条
        let svc = CoreService::new(data);
        let resp = svc
            .search(Request::new(SearchRequest {
                query: chunk_id.clone(),
                collections: vec![coll.clone()],
                agent_scope: vec![],
                top_k: 10,
                filters: None,
                explain: false,
            semantic: false,
            hybrid: false,
            }))
            .await
            .expect("fast-path search ok");
        let inner = resp.into_inner();
        assert_eq!(
            inner.results.len(),
            1,
            "AC2-E fast-path: 应返单条 (target chunk_id), got {}",
            inner.results.len()
        );
        assert_eq!(
            inner.results[0].chunk_id, chunk_id,
            "AC2-E fast-path: 命中 chunk_id 一致"
        );
    }

    // ---- TEST-6.2.E4 — fast-path miss → fallback to BM25 search ----
    //
    // Query 用 chunker format-shape 字面 (`chk_deadbeef_0`) 触发 fast-path
    // detector，但 chunk_id 不在 fixture 中。expected: get_chunk → None → fallback
    // 走原 BM25 search() 路径 (which returns 0 hits for this nonsense query,
    // but the wrap is Ok, not Err).
    #[tokio::test]
    async fn test_6_2_e4_search_fast_path_miss_falls_back_to_search() {
        let (data, coll) = build_fixture(
            "ac2e-fastpath-miss",
            &[("readme.md", "# Readme\nunique token fallbackmarker62z\n")],
        );
        // 形如 chunk_id 但不在 fixture 中
        let nonexistent_chunk_id = "chk_deadbeef_0";
        // detector 应识别为 chunk_id 形（触发 fast-path 尝试）
        assert!(
            crate::retriever::is_chunk_id_format(nonexistent_chunk_id),
            "detector 应识别 'chk_<hex>_<ord>' 形"
        );

        let svc = CoreService::new(data);
        let resp = svc
            .search(Request::new(SearchRequest {
                query: nonexistent_chunk_id.into(),
                collections: vec![coll.clone()],
                agent_scope: vec![],
                top_k: 10,
                filters: None,
                explain: false,
            semantic: false,
            hybrid: false,
            }))
            .await
            .expect("fallback search ok (BM25 won't crash even if 0 hits)");
        let inner = resp.into_inner();
        // BM25 fallback 对 "chk_deadbeef_0" 几乎 0 hits（fixture body 没该串），但路径活.
        // 核心：fast-path miss 后 fallback 走通，返 Ok(SearchResponse) 即使 0 hits.
        assert!(
            inner.results.len() <= 10,
            "AC2-E fallback: BM25 search 路径活，return ≤ top_k results"
        );
    }
}
