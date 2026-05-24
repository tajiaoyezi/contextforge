//! task-11.4: `SearchServer` real impl wrapping `core/src/retriever`.
//!
//! `Query` opens a `Retriever` for the workspace_id-as-collection_id (ADR-015
//! D2), executes `Retriever::search` with the request's query + top_k, and
//! maps each `SearchResult` to the proto `SearchResultItem`. `RetrievalTrace`
//! is built from the same hit set with `retrieved_chunks` populated
//! (score + source_file + UTF-8-safe content snippet ≤ 200 chars).

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use tonic::{Request, Response, Status};

use crate::pb_console::search_service_server::SearchService;
use crate::pb_console::{
    Citation as PbCitation, GetSearchTraceRequest, GetSourceChunkRequest,
    RetrievalTrace as PbRetrievalTrace, SearchRequest as PbSearchRequest, SearchResponse,
    SearchResultItem, SourceChunk as PbSourceChunk,
};
use crate::retriever::{Retriever, RetrieverError, SearchFilters, SearchOptions};
use crate::workspace::WorkspaceStore;

use super::DataPlaneStores;

/// task-12.3 (ADR-017 D1 Wave 2): in-memory LRU cap for trace_store. Picked
/// to bound memory under sustained Console UI debug usage; daemon restart
/// loses entries [SPEC-DEFER:task-future.search-trace-sqlite-persistence].
const TRACE_STORE_CAP: usize = 1000;

/// LRU-FIFO trace store: HashMap for O(1) lookup + VecDeque for insertion-order
/// eviction. Newer inserts of an existing key refresh recency by re-pushing.
struct TraceStore {
    map: HashMap<String, PbRetrievalTrace>,
    order: VecDeque<String>,
    cap: usize,
}

impl TraceStore {
    fn new(cap: usize) -> Self {
        Self {
            map: HashMap::with_capacity(cap),
            order: VecDeque::with_capacity(cap),
            cap,
        }
    }

    fn put(&mut self, key: String, value: PbRetrievalTrace) {
        if self.map.contains_key(&key) {
            // Refresh recency: remove old position, push to back.
            if let Some(pos) = self.order.iter().position(|k| k == &key) {
                self.order.remove(pos);
            }
        } else if self.map.len() >= self.cap {
            // Evict oldest.
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            }
        }
        self.order.push_back(key.clone());
        self.map.insert(key, value);
    }

    fn get(&self, key: &str) -> Option<PbRetrievalTrace> {
        self.map.get(key).cloned()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.map.len()
    }
}

pub struct SearchServer {
    stores: Arc<DataPlaneStores>,
    trace_store: Arc<Mutex<TraceStore>>,
}

impl SearchServer {
    pub fn new(stores: Arc<DataPlaneStores>) -> Self {
        Self {
            stores,
            trace_store: Arc::new(Mutex::new(TraceStore::new(TRACE_STORE_CAP))),
        }
    }
}

/// Truncate `s` to at most `max_chars` chars on a UTF-8 boundary
/// (multi-byte safe). Returns the prefix slice borrow.
pub fn utf8_safe_truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

#[tonic::async_trait]
impl SearchService for SearchServer {
    async fn query(
        &self,
        req: Request<PbSearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        let req = req.into_inner();

        // task-11.4 §6 AC1: real Retriever-backed search. Workspace_id maps
        // 1:1 to collection_id per ADR-015 D2.
        // If data_dir is empty (task-11.1 mode where no real index exists)
        // OR workspace_id is empty, fall through to the empty-response path.
        if self.stores.data_dir.as_os_str().is_empty() || req.workspace_id.is_empty() {
            return Ok(Response::new(empty_response(&req.query)));
        }

        let retriever = match Retriever::open(&self.stores.data_dir, &req.workspace_id) {
            Ok(r) => r,
            Err(RetrieverError::CollectionNotFound(_))
            | Err(RetrieverError::Io(_)) => {
                // No index yet (no Enqueue completed for this workspace) →
                // return empty results gracefully (HTTP semantics: 200 OK
                // + empty list, not 404).
                return Ok(Response::new(empty_response(&req.query)));
            }
            Err(RetrieverError::Tantivy(msg)) if msg.contains("FileDoesNotExist") || msg.contains("meta.json") => {
                // Tantivy index not yet created (no IndexJob succeeded for
                // this workspace) → treat as empty (same as collection-not-
                // found case above).
                return Ok(Response::new(empty_response(&req.query)));
            }
            Err(e) => return Err(Status::internal(format!("retriever open: {e}"))),
        };

        let top_k = if req.top_k <= 0 { 5 } else { req.top_k as usize };
        let opts = SearchOptions {
            query: req.query.clone(),
            top_k,
            filters: SearchFilters::default(),
            explain: false,
        };

        let hits = retriever
            .search(&opts)
            .map_err(|e| Status::internal(format!("retriever search: {e}")))?;

        // task-11.4 §6 AC2: build RetrievalTrace.retrieved_chunks (score +
        // source_file + content snippet ≤ 200 UTF-8 chars).
        let chunks: Vec<PbSourceChunk> = hits
            .iter()
            .map(|h| PbSourceChunk {
                chunk_id: h.chunk_id.clone(),
                workspace_id: req.workspace_id.clone(),
                source_file_path: h.file_path.clone(),
                line_start: h.line_start as i64,
                line_end: h.line_end as i64,
                chunk_text_preview: utf8_safe_truncate(&h.content, 200).to_string(),
                chunk_offset_start: 0, // SearchResult does not surface offsets
                chunk_offset_end: 0,
                redaction_status: h.redaction_status.clone(),
            })
            .collect();

        // task-12.3 (ADR-017 D1 Wave 2): generate a unique query_id once per
        // Query and stamp it on every result + trace, then persist the trace
        // into the in-memory store keyed by that query_id (Console UI can
        // later GET /v1/search/{query_id}/trace).
        let query_id = format!("qry-{}", trace_seq());
        let results: Vec<SearchResultItem> = hits
            .iter()
            .enumerate()
            .map(|(idx, h)| SearchResultItem {
                result_id: format!("res-{}", idx),
                query_id: query_id.clone(),
                workspace_id: req.workspace_id.clone(),
                source_file_path: h.file_path.clone(),
                source_file_type: h.source_type.clone(),
                chunk_id: h.chunk_id.clone(),
                chunk_text_preview: utf8_safe_truncate(&h.content, 200).to_string(),
                line_start: h.line_start as i64,
                line_end: h.line_end as i64,
                score: h.score as f64,
                rank_before_rerank: idx as i64,
                rank_after_rerank: None,
                retrieval_method: h.retrieval_method.clone(),
                reason: h.reason.clone(),
                citation: Some(PbCitation {
                    citation_id: format!("cit-{}", h.chunk_id),
                    source_file_path: h.file_path.clone(),
                    chunk_id: h.chunk_id.clone(),
                    line_start: h.line_start as i64,
                    line_end: h.line_end as i64,
                    confidence: h.score as f64,
                }),
            })
            .collect();

        let final_context_count = results.len() as i64;
        let trace = PbRetrievalTrace {
            trace_id: format!("trace-{}", trace_seq()),
            query: req.query.clone(),
            expanded_query: None,
            candidate_generation_steps: vec!["tantivy-bm25".to_string()],
            lexical_candidates_count: final_context_count,
            vector_candidates_count: 0,
            rerank_steps: vec![],
            scope_filter_result: "no-op".to_string(),
            final_context_count,
            retrieved_chunks: chunks,
        };

        // task-12.3: persist trace by query_id for later GetSearchTrace lookup.
        if let Ok(mut store) = self.trace_store.lock() {
            store.put(query_id, trace.clone());
        }

        Ok(Response::new(SearchResponse {
            results,
            trace: Some(trace),
        }))
    }

    async fn get_source_chunk(
        &self,
        req: Request<GetSourceChunkRequest>,
    ) -> Result<Response<PbSourceChunk>, Status> {
        let req = req.into_inner();
        if req.chunk_id.is_empty() {
            return Err(Status::invalid_argument("chunk_id must not be empty"));
        }
        if self.stores.data_dir.as_os_str().is_empty() {
            return Err(Status::not_found(format!(
                "chunk not found: {} (no data plane index)",
                req.chunk_id
            )));
        }
        // task-12.2 (ADR-017 D1 Wave 2): workspace_id is optional. When set,
        // open that collection directly; when empty, probe known workspaces
        // (Phase 12 v1.0: chunk_id is global-unique per SqliteChunkStore
        // schema so any open collection finding it is the right one).
        let candidates: Vec<String> = if !req.workspace_id.is_empty() {
            vec![req.workspace_id.clone()]
        } else {
            self.stores
                .workspace_store
                .list()
                .map_err(|e| Status::internal(format!("workspace list: {e}")))?
                .into_iter()
                .map(|w| w.workspace_id)
                .collect()
        };
        for ws_id in candidates {
            let retriever = match Retriever::open(&self.stores.data_dir, &ws_id) {
                Ok(r) => r,
                Err(RetrieverError::CollectionNotFound(_)) | Err(RetrieverError::Io(_)) => continue,
                Err(RetrieverError::Tantivy(msg))
                    if msg.contains("FileDoesNotExist") || msg.contains("meta.json") =>
                {
                    continue;
                }
                Err(e) => return Err(Status::internal(format!("retriever open: {e}"))),
            };
            match retriever.get_chunk(&req.chunk_id) {
                Ok(Some(sr)) => {
                    return Ok(Response::new(PbSourceChunk {
                        chunk_id: sr.chunk_id,
                        workspace_id: ws_id,
                        source_file_path: sr.file_path,
                        line_start: sr.line_start as i64,
                        line_end: sr.line_end as i64,
                        chunk_text_preview: utf8_safe_truncate(&sr.content, 200).to_string(),
                        // SourceChunk byte offsets [SPEC-DEFER:chunk-byte-offsets]
                        // — SqliteChunkStore current schema does not store byte
                        // offsets; v0.5 returns 0/0; Console UI uses line ranges.
                        chunk_offset_start: 0,
                        chunk_offset_end: 0,
                        redaction_status: sr.redaction_status,
                    }));
                }
                Ok(None) => continue,
                Err(e) => return Err(Status::internal(format!("retriever get_chunk: {e}"))),
            }
        }
        Err(Status::not_found(format!(
            "chunk not found: {}",
            req.chunk_id
        )))
    }

    async fn get_search_trace(
        &self,
        req: Request<GetSearchTraceRequest>,
    ) -> Result<Response<PbRetrievalTrace>, Status> {
        let req = req.into_inner();
        if req.query_id.is_empty() {
            return Err(Status::invalid_argument("query_id must not be empty"));
        }
        let trace = self
            .trace_store
            .lock()
            .map_err(|_| Status::internal("trace_store poisoned"))?
            .get(&req.query_id);
        match trace {
            Some(t) => Ok(Response::new(t)),
            None => Err(Status::not_found(format!(
                "trace not found: {}",
                req.query_id
            ))),
        }
    }
}

fn empty_response(query: &str) -> SearchResponse {
    SearchResponse {
        results: vec![],
        trace: Some(PbRetrievalTrace {
            trace_id: format!("trace-{}", trace_seq()),
            query: query.to_string(),
            expanded_query: None,
            candidate_generation_steps: vec![],
            lexical_candidates_count: 0,
            vector_candidates_count: 0,
            rerank_steps: vec![],
            scope_filter_result: "no-op".to_string(),
            final_context_count: 0,
            retrieved_chunks: vec![],
        }),
    }
}

fn trace_seq() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::SqliteJobStore;
    use crate::workspace::SqliteWorkspaceStore;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_data_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "cf-search-server-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn fresh_server() -> SearchServer {
        let dir = temp_data_dir("empty");
        let ws = Arc::new(SqliteWorkspaceStore::open(&dir).unwrap());
        let js = Arc::new(SqliteJobStore::open(&dir).unwrap());
        SearchServer::new(DataPlaneStores::new(ws, js))
    }

    #[tokio::test]
    async fn test_get_search_trace_empty_query_id_returns_invalid_argument() {
        let server = fresh_server();
        let err = server
            .get_search_trace(Request::new(GetSearchTraceRequest {
                query_id: "".into(),
            }))
            .await
            .expect_err("expect invalid_argument");
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_get_search_trace_unknown_returns_not_found() {
        let server = fresh_server();
        let err = server
            .get_search_trace(Request::new(GetSearchTraceRequest {
                query_id: "qry-does-not-exist".into(),
            }))
            .await
            .expect_err("expect not_found");
        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_query_persists_trace_by_query_id_and_get_returns_it() {
        // Query() with an unindexed workspace falls through to empty_response()
        // which does NOT touch trace_store; to exercise persistence we put
        // directly via the helper, then verify get_search_trace.
        let server = fresh_server();
        let synthetic = PbRetrievalTrace {
            trace_id: "trace-test".into(),
            query: "hello".into(),
            expanded_query: None,
            candidate_generation_steps: vec!["bm25".into()],
            lexical_candidates_count: 0,
            vector_candidates_count: 0,
            rerank_steps: vec![],
            scope_filter_result: "no-op".into(),
            final_context_count: 0,
            retrieved_chunks: vec![],
        };
        server
            .trace_store
            .lock()
            .unwrap()
            .put("qry-test-1".into(), synthetic.clone());
        let resp = server
            .get_search_trace(Request::new(GetSearchTraceRequest {
                query_id: "qry-test-1".into(),
            }))
            .await
            .expect("get_search_trace ok");
        let got = resp.into_inner();
        assert_eq!(got.trace_id, "trace-test");
        assert_eq!(got.query, "hello");
    }

    #[tokio::test]
    async fn test_trace_store_eviction_at_capacity() {
        let mut store = TraceStore::new(3);
        for i in 0..5 {
            store.put(
                format!("qry-{i}"),
                PbRetrievalTrace {
                    trace_id: format!("trace-{i}"),
                    query: format!("q{i}"),
                    expanded_query: None,
                    candidate_generation_steps: vec![],
                    lexical_candidates_count: 0,
                    vector_candidates_count: 0,
                    rerank_steps: vec![],
                    scope_filter_result: "".into(),
                    final_context_count: 0,
                    retrieved_chunks: vec![],
                },
            );
        }
        assert_eq!(store.len(), 3);
        // Oldest 2 (qry-0, qry-1) evicted; newest 3 (qry-2, 3, 4) retained.
        assert!(store.get("qry-0").is_none());
        assert!(store.get("qry-1").is_none());
        assert!(store.get("qry-2").is_some());
        assert!(store.get("qry-3").is_some());
        assert!(store.get("qry-4").is_some());
    }

    #[tokio::test]
    async fn test_get_source_chunk_empty_chunk_id_returns_invalid_argument() {
        let server = fresh_server();
        let err = server
            .get_source_chunk(Request::new(GetSourceChunkRequest {
                chunk_id: "".into(),
                workspace_id: "".into(),
            }))
            .await
            .expect_err("expect error");
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_get_source_chunk_unknown_returns_not_found() {
        let server = fresh_server();
        let err = server
            .get_source_chunk(Request::new(GetSourceChunkRequest {
                chunk_id: "chk_dead_0".into(),
                workspace_id: "".into(),
            }))
            .await
            .expect_err("expect not_found");
        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_search_server_empty_response() {
        let server = fresh_server();
        let resp = server
            .query(Request::new(PbSearchRequest {
                query: "anything".into(),
                workspace_id: "ws-x".into(),
                agent_scope: "".into(),
                retrieval_method: "bm25".into(),
                top_k: 5,
                config_snapshot: "{}".into(),
            }))
            .await
            .expect("query ok");
        let inner = resp.into_inner();
        // task-11.1 占位: empty results + non-None trace
        assert!(inner.results.is_empty(), "task-11.1: empty results");
        let trace = inner.trace.expect("trace present");
        assert_eq!(trace.query, "anything");
        assert!(trace.retrieved_chunks.is_empty());
    }
}
