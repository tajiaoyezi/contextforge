//! task-11.1 §3 / §6 AC5: `SearchServer` 占位实现 (real retriever wiring 在
//! [SPEC-OWNER:task-11.4])。
//!
//! 本 task: `Query` 返 empty `SearchResponse { results: [], trace: ... }`。
//! task-11.4 替换为真调 `core/src/retriever/Retriever::search` + 真填
//! `RetrievalTrace.retrieved_chunks` (score + source_file + content snippet)。

use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::pb_console::search_service_server::SearchService;
use crate::pb_console::{
    RetrievalTrace as PbRetrievalTrace, SearchRequest as PbSearchRequest, SearchResponse,
};

use super::DataPlaneStores;

pub struct SearchServer {
    #[allow(dead_code)] // task-11.4 will use stores.retriever
    stores: Arc<DataPlaneStores>,
}

impl SearchServer {
    pub fn new(stores: Arc<DataPlaneStores>) -> Self {
        Self { stores }
    }
}

#[tonic::async_trait]
impl SearchService for SearchServer {
    async fn query(
        &self,
        req: Request<PbSearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        let req = req.into_inner();
        // task-11.4 [SPEC-OWNER:task-11.4]: replace with real Retriever::search
        // + RetrievalTrace 真填. task-11.1 returns empty + minimal trace so
        // the gRPC wire is exercised by integration tests.
        let trace = PbRetrievalTrace {
            trace_id: format!("trace-task111-{}", trace_seq()),
            query: req.query.clone(),
            expanded_query: None,
            candidate_generation_steps: vec!["task-11.1-placeholder".to_string()],
            lexical_candidates_count: 0,
            vector_candidates_count: 0,
            rerank_steps: vec![],
            scope_filter_result: "no-op".to_string(),
            final_context_count: 0,
            retrieved_chunks: vec![],
        };
        Ok(Response::new(SearchResponse {
            results: vec![],
            trace: Some(trace),
        }))
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
