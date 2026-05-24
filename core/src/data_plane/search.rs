//! task-11.4: `SearchServer` real impl wrapping `core/src/retriever`.
//!
//! `Query` opens a `Retriever` for the workspace_id-as-collection_id (ADR-015
//! D2), executes `Retriever::search` with the request's query + top_k, and
//! maps each `SearchResult` to the proto `SearchResultItem`. `RetrievalTrace`
//! is built from the same hit set with `retrieved_chunks` populated
//! (score + source_file + UTF-8-safe content snippet ≤ 200 chars).

use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::pb_console::search_service_server::SearchService;
use crate::pb_console::{
    Citation as PbCitation, RetrievalTrace as PbRetrievalTrace, SearchRequest as PbSearchRequest,
    SearchResponse, SearchResultItem, SourceChunk as PbSourceChunk,
};
use crate::retriever::{Retriever, RetrieverError, SearchFilters, SearchOptions};

use super::DataPlaneStores;

pub struct SearchServer {
    stores: Arc<DataPlaneStores>,
}

impl SearchServer {
    pub fn new(stores: Arc<DataPlaneStores>) -> Self {
        Self { stores }
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

        let results: Vec<SearchResultItem> = hits
            .iter()
            .enumerate()
            .map(|(idx, h)| SearchResultItem {
                result_id: format!("res-{}", idx),
                query_id: String::new(),
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

        Ok(Response::new(SearchResponse {
            results,
            trace: Some(trace),
        }))
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
