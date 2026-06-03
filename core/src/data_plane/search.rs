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
    ChunksStats as PbChunksStats, Citation as PbCitation, GetChunksStatsRequest,
    GetSearchTraceRequest, GetSourceChunkRequest, ListQueriesRequest, ListQueriesResponse,
    QueryRecord as PbQueryRecord, RetrievalTrace as PbRetrievalTrace,
    SearchRequest as PbSearchRequest, SearchResponse, SearchResultItem, SourceChunk as PbSourceChunk,
};
use crate::embedding::DeterministicEmbeddingProvider;
use crate::retriever::vector::BruteForceVectorBackend;
use crate::retriever::{Retriever, RetrieverError, SearchFilters, SearchOptions};
use crate::workspace::WorkspaceStore;

use super::search_persist::SqliteTracePersist;
use super::DataPlaneStores;

/// task-12.3 (ADR-017 D1 Wave 2): in-memory LRU cap for trace_store. Picked
/// to bound memory under sustained Console UI debug usage; daemon restart
/// loses entries [SPEC-DEFER:task-future.search-trace-sqlite-persistence].
const TRACE_STORE_CAP: usize = 1000;

/// task-15.5 (Phase 15 P1 #5): wrapped trace record. PbRetrievalTrace itself
/// lacks workspace_id / ts_unix per ADR-015 D1 field freeze; we keep those as
/// out-of-band metadata in the trace store so QueryRecord can be built for
/// ListQueries without amending the contract message.
#[derive(Clone)]
struct TraceRecord {
    trace: PbRetrievalTrace,
    workspace_id: String,
    ts_unix: i64,
}

/// LRU-FIFO trace store: HashMap for O(1) lookup + VecDeque for insertion-order
/// eviction. Newer inserts of an existing key refresh recency by re-pushing.
///
/// task-16.1 (Phase 16 P4 #10): optional `persist` field enables write-through
/// to a SQLite-backed `SqliteTracePersist`. Hot cache LRU semantics are
/// unchanged when `persist == None` (task-12.3/15.5 baseline); when `Some`,
/// `put` double-writes (best-effort on SQLite errors), `get` falls back to
/// SQLite on cache miss, and `list` falls back when the hot cache has fewer
/// than `limit` items.
struct TraceStore {
    map: HashMap<String, TraceRecord>,
    order: VecDeque<String>,
    cap: usize,
    persist: Option<Arc<SqliteTracePersist>>,
}

impl TraceStore {
    fn new(cap: usize) -> Self {
        Self {
            map: HashMap::with_capacity(cap),
            order: VecDeque::with_capacity(cap),
            cap,
            persist: None,
        }
    }

    /// task-16.1: build a TraceStore wired to a SQLite persist and warm
    /// restore the hot cache from the most-recent `cap` rows (insertion
    /// order: oldest-first so the newest lands at the back of the VecDeque).
    fn with_persist(cap: usize, persist: Arc<SqliteTracePersist>) -> Self {
        let mut store = Self {
            map: HashMap::with_capacity(cap),
            order: VecDeque::with_capacity(cap),
            cap,
            persist: Some(persist.clone()),
        };
        match persist.load_warm(cap) {
            Ok(warm) => {
                for (key, trace, ws, ts) in warm {
                    store.put_mem_only(key, trace, ws, ts);
                }
            }
            Err(e) => {
                eprintln!(
                    "WARN search_persist warm restore failed (starting with empty hot cache): {e}"
                );
            }
        }
        store
    }

    fn put(
        &mut self,
        key: String,
        trace: PbRetrievalTrace,
        workspace_id: String,
        ts_unix: i64,
    ) {
        // 1) hot cache write (LRU semantics unchanged from v0.8).
        self.put_mem_only(key.clone(), trace.clone(), workspace_id.clone(), ts_unix);
        // 2) write-through to SQLite — best-effort; SQLite error is logged but
        //    does not abort the hot cache update (Console UI keeps working off
        //    the in-memory store).
        if let Some(p) = self.persist.as_ref() {
            if let Err(e) = p.put(&key, &trace, &workspace_id, ts_unix) {
                eprintln!(
                    "WARN search_persist.put failed (key={key}); hot cache still updated: {e}"
                );
            }
        }
    }

    /// task-16.1: extracted from the original `put` body so `with_persist`
    /// can warm-restore the LRU without re-writing to SQLite.
    fn put_mem_only(
        &mut self,
        key: String,
        trace: PbRetrievalTrace,
        workspace_id: String,
        ts_unix: i64,
    ) {
        if self.map.contains_key(&key) {
            if let Some(pos) = self.order.iter().position(|k| k == &key) {
                self.order.remove(pos);
            }
        } else if self.map.len() >= self.cap {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            }
        }
        self.order.push_back(key.clone());
        self.map.insert(
            key,
            TraceRecord {
                trace,
                workspace_id,
                ts_unix,
            },
        );
    }

    fn get(&self, key: &str) -> Option<PbRetrievalTrace> {
        if let Some(rec) = self.map.get(key) {
            return Some(rec.trace.clone());
        }
        // task-16.1: hot cache miss → SQLite fallback (read-only; deliberately
        // does not back-fill the LRU to avoid polluting recency with old rows).
        self.persist
            .as_ref()
            .and_then(|p| p.get(key).ok().flatten())
    }

    /// task-15.5 / task-16.1: list the most-recent N query records (DESC by
    /// insertion order via VecDeque reverse iteration). `limit` is clamped
    /// 1..=100. If a SQLite persist is configured AND the hot cache returns
    /// fewer items than requested, fall back to SQLite ORDER BY ts_unix DESC.
    fn list(&self, limit: usize) -> Vec<PbQueryRecord> {
        let lim = limit.clamp(1, 100);
        let mem: Vec<PbQueryRecord> = self
            .order
            .iter()
            .rev()
            .take(lim)
            .filter_map(|key| {
                self.map.get(key).map(|rec| PbQueryRecord {
                    query_id: key.clone(),
                    query: rec.trace.query.clone(),
                    ts_unix: rec.ts_unix,
                    workspace_id: rec.workspace_id.clone(),
                })
            })
            .collect();
        if mem.len() >= lim || self.persist.is_none() {
            return mem;
        }
        // task-16.1: hot cache short → SQLite supplements. After warm restore
        // this rarely triggers (LRU holds up to 1000); fresh boot before first
        // `put` is the typical hit.
        match self.persist.as_ref().unwrap().list(lim) {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!("WARN search_persist.list failed; returning hot cache subset: {e}");
                mem
            }
        }
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
    /// task-11.4 / task-16.1: read `stores.trace_persist` to decide whether
    /// the in-memory `TraceStore` is wired to a SQLite-backed persist
    /// (production via `serve_full`) or stays in-memory-only (Phase 11/12/15
    /// tests via `DataPlaneStores::new` / `with_eval` / `with_memory`).
    pub fn new(stores: Arc<DataPlaneStores>) -> Self {
        let trace_store = match stores.trace_persist.as_ref() {
            Some(persist) => TraceStore::with_persist(TRACE_STORE_CAP, persist.clone()),
            None => TraceStore::new(TRACE_STORE_CAP),
        };
        Self {
            stores,
            trace_store: Arc::new(Mutex::new(trace_store)),
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

        // task-20.1 (Phase 20): opt-in semantic path. Mirrors core CoreService.search
        // (server.rs, task-19.3): wire the model-free DeterministicEmbeddingProvider +
        // the 0-dep BruteForceVectorBackend, build an on-demand in-memory index from this
        // collection's chunks (no persistence — [SPEC-DEFER:phase-future.hnsw-graph-persistence]),
        // and run the vector path. Hits carry retrieval_method "vector". Deterministic
        // embeddings prove the wiring, not recall (real recall is task-19.5/20.2; ADR-013).
        // Default (semantic == false) keeps the BM25 path byte-for-byte unchanged.
        let hits = if req.semantic {
            let embedder = Arc::new(DeterministicEmbeddingProvider::default());
            let backend = Arc::new(BruteForceVectorBackend::new());
            let wired = retriever
                .with_embedder(embedder)
                .with_vector_searcher(backend.clone());
            let items = wired
                .enumerate_chunks()
                .map_err(|e| Status::internal(format!("semantic enumerate: {e}")))?;
            wired
                .index_chunks_semantic(backend.as_ref(), &items)
                .map_err(|e| Status::internal(format!("semantic index: {e}")))?;
            wired
                .search_semantic(&req.query, top_k)
                .map_err(|e| Status::internal(format!("semantic search: {e}")))?
        } else {
            let opts = SearchOptions {
                query: req.query.clone(),
                top_k,
                filters: SearchFilters::default(),
                explain: false,
            };
            retriever
                .search(&opts)
                .map_err(|e| Status::internal(format!("retriever search: {e}")))?
        };

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
                // task-32.3 add-only: vector_score provenance, mirroring the v1 search path
                // (server.rs:407) — the cosine similarity for semantic ("vector") hits, 0 for BM25
                // (no fabricated score; ADR-013). Parity with v1 search proto vector_score=13.
                vector_score: if h.retrieval_method == "vector" {
                    h.score
                } else {
                    0.0
                },
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
            candidate_generation_steps: vec![if req.semantic {
                "vector-bruteforce".to_string()
            } else {
                "tantivy-bm25".to_string()
            }],
            lexical_candidates_count: if req.semantic { 0 } else { final_context_count },
            vector_candidates_count: if req.semantic { final_context_count } else { 0 },
            rerank_steps: vec![],
            scope_filter_result: "no-op".to_string(),
            final_context_count,
            retrieved_chunks: chunks,
        };

        // task-12.3 / task-15.5: persist trace + metadata by query_id for later
        // GetSearchTrace lookup and ListQueries history listing.
        if let Ok(mut store) = self.trace_store.lock() {
            store.put(
                query_id.clone(),
                trace.clone(),
                req.workspace_id.clone(),
                now_unix(),
            );
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

    /// task-15.5 (Phase 15 P1 #5): query history list. Returns most-recent N
    /// `QueryRecord` entries from the in-memory trace store. Limit default 20,
    /// clamped 1..=100 server-side. Daemon restart wipes the store — same
    /// trade-off as get_search_trace ([SPEC-DEFER:task-future.search-trace-sqlite-persistence]).
    async fn list_queries(
        &self,
        req: Request<ListQueriesRequest>,
    ) -> Result<Response<ListQueriesResponse>, Status> {
        let inner = req.into_inner();
        let limit = if inner.limit <= 0 {
            20usize
        } else {
            inner.limit as usize
        };
        let store = self
            .trace_store
            .lock()
            .map_err(|_| Status::internal("trace_store poisoned"))?;
        let records = store.list(limit);
        Ok(Response::new(ListQueriesResponse { records }))
    }

    /// task-15.3 (Phase 15 P1 #3): cross-workspace chunks stats.
    ///
    /// - `total` aggregates `Retriever::num_docs()` (Tantivy live segment doc
    ///   count) across every opened workspace collection
    /// - `today_delta` aggregates `Retriever::count_indexed_since(today_start)`
    ///   over the same set (chunks.indexed_at TEXT lexicographic compare)
    ///
    /// `req.workspace_id` is honored when set; empty value falls back to
    /// iterating all registered workspaces (consistent with `get_source_chunk`
    /// open-set probe behavior). Collections that fail to open are skipped
    /// silently — health probing lives in task-15.6, not stats. [SPEC-OWNER:task-15.3]
    async fn get_chunks_stats(
        &self,
        req: Request<GetChunksStatsRequest>,
    ) -> Result<Response<PbChunksStats>, Status> {
        let inner = req.into_inner();
        if self.stores.data_dir.as_os_str().is_empty() {
            // No data plane → return zero stats (UI renders "no data" rather
            // than 503; aligns with fallback semantics).
            return Ok(Response::new(PbChunksStats {
                total: 0,
                today_delta: 0,
            }));
        }
        let candidates: Vec<String> = if !inner.workspace_id.is_empty() {
            vec![inner.workspace_id]
        } else {
            self.stores
                .workspace_store
                .list()
                .map_err(|e| Status::internal(format!("workspace list: {e}")))?
                .into_iter()
                .map(|w| w.workspace_id)
                .collect()
        };
        let today_iso = today_start_iso();
        let mut total: i64 = 0;
        let mut today_delta: i64 = 0;
        for ws_id in candidates {
            let retriever = match Retriever::open(&self.stores.data_dir, &ws_id) {
                Ok(r) => r,
                Err(_) => continue, // skip unopenable collections per [SPEC-OWNER:task-15.3]
            };
            total = total.saturating_add(retriever.num_docs() as i64);
            today_delta =
                today_delta.saturating_add(retriever.count_indexed_since(&today_iso));
        }
        Ok(Response::new(PbChunksStats { total, today_delta }))
    }
}

/// task-15.3 helper: compute the start-of-today (UTC) as an ISO-ish string
/// compatible with the indexer's `indexed_at_now_str` format ("YYYY-MM-DD
/// HH:MM:SS"). Lexicographic compare against indexed_at column yields the
/// correct ">= today" set without parsing.
fn today_start_iso() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let day = now / 86_400 * 86_400;
    seconds_to_iso(day)
}

fn seconds_to_iso(unix_secs: i64) -> String {
    // Civil-date arithmetic (no chrono dep). Days since 1970-01-01.
    let days = unix_secs.div_euclid(86_400);
    let secs_of_day = unix_secs.rem_euclid(86_400);
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;
    // Convert days since epoch to civil date (Howard Hinnant algorithm).
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as i64;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as i64;
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02} {hour:02}:{minute:02}:{second:02}")
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

/// task-15.5: seconds-since-epoch helper for QueryRecord.ts_unix.
fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
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
            .put(
                "qry-test-1".into(),
                synthetic.clone(),
                "ws-test".into(),
                1_700_000_000,
            );
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

    // task-15.3 (Phase 15 P1 #3): chunks stats RPC tests.
    #[tokio::test]
    async fn test_get_chunks_stats_empty_data_dir_returns_zero() {
        // fresh_server uses DataPlaneStores::new which leaves data_dir empty;
        // get_chunks_stats short-circuits to {0, 0} for the fallback path.
        let server = fresh_server();
        let resp = server
            .get_chunks_stats(Request::new(GetChunksStatsRequest {
                workspace_id: String::new(),
            }))
            .await
            .expect("get_chunks_stats ok");
        let stats = resp.into_inner();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.today_delta, 0);
    }

    #[tokio::test]
    async fn test_get_chunks_stats_with_workspace_id_filter_returns_zero_when_empty() {
        // Workspace ID is honored but no collection has been indexed → 0/0.
        let server = fresh_server();
        let resp = server
            .get_chunks_stats(Request::new(GetChunksStatsRequest {
                workspace_id: "ws-test".into(),
            }))
            .await
            .expect("get_chunks_stats ok");
        let stats = resp.into_inner();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.today_delta, 0);
    }

    #[test]
    fn test_today_start_iso_format_is_lexicographic_sortable() {
        // The string must be lexicographically ordered same as chronologically
        // (year-month-day HH:MM:SS pad zeros) to ensure SQLite >= compare works.
        let s = today_start_iso();
        // Format check: "YYYY-MM-DD HH:MM:SS"
        assert_eq!(s.len(), 19);
        assert_eq!(&s[4..5], "-");
        assert_eq!(&s[7..8], "-");
        assert_eq!(&s[10..11], " ");
        // today_start has HH:MM:SS = 00:00:00
        assert!(s.ends_with(" 00:00:00"), "today_start should be midnight: {s}");
    }

    #[test]
    fn test_seconds_to_iso_known_value() {
        // 1700000000 = 2023-11-14 22:13:20 UTC
        let s = seconds_to_iso(1_700_000_000);
        assert_eq!(s, "2023-11-14 22:13:20");
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
                "ws-test".into(),
                1_700_000_000 + i as i64,
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

    // task-15.5 (Phase 15 P1 #5): TraceStore.list + SearchServer.list_queries tests.
    #[test]
    fn test_trace_store_list_returns_recent_first() {
        let mut store = TraceStore::new(10);
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
                "ws".into(),
                1_700_000_000 + i as i64,
            );
        }
        let recs = store.list(3);
        assert_eq!(recs.len(), 3);
        // Most recent (qry-4) first.
        assert_eq!(recs[0].query_id, "qry-4");
        assert_eq!(recs[1].query_id, "qry-3");
        assert_eq!(recs[2].query_id, "qry-2");
        // ts_unix carries over.
        assert_eq!(recs[0].ts_unix, 1_700_000_004);
    }

    #[test]
    fn test_trace_store_list_clamps_limit() {
        let mut store = TraceStore::new(10);
        store.put(
            "q1".into(),
            PbRetrievalTrace {
                trace_id: "t1".into(),
                query: "hi".into(),
                expanded_query: None,
                candidate_generation_steps: vec![],
                lexical_candidates_count: 0,
                vector_candidates_count: 0,
                rerank_steps: vec![],
                scope_filter_result: "".into(),
                final_context_count: 0,
                retrieved_chunks: vec![],
            },
            "ws".into(),
            1_700_000_000,
        );
        // 0 → clamp to at least 1; 500 → clamp to 100 (but store only has 1).
        let one = store.list(0);
        assert_eq!(one.len(), 1);
        let big = store.list(500);
        assert_eq!(big.len(), 1);
    }

    #[tokio::test]
    async fn test_list_queries_rpc_default_limit_returns_empty() {
        let server = fresh_server();
        let resp = server
            .list_queries(Request::new(ListQueriesRequest { limit: 0 }))
            .await
            .expect("list_queries ok");
        assert!(resp.into_inner().records.is_empty());
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
                semantic: false,
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

    // ---- TEST-20.1 — console SearchService.Query semantic branch dispatches the
    // vector path (mirrors core CoreService TEST-19.3). Deterministic embeddings
    // prove the dispatch/plumbing, not recall quality (real recall is task-19.5/
    // 20.2; ADR-013). semantic=false keeps the BM25 baseline. ----
    #[tokio::test]
    async fn test_20_1_query_semantic_dispatches_vector_path() {
        use crate::chunker::ChunkPolicy;
        use crate::indexer::IndexSession;
        use crate::scanner::{default_denylist, ScanOptions};

        let src = temp_data_dir("sem-src");
        let data = temp_data_dir("sem-data");
        let coll = "ws-sem".to_string();
        std::fs::write(
            src.join("a.md"),
            "where is the config loader and default data dir",
        )
        .unwrap();
        std::fs::write(src.join("b.md"), "how the daemon restarts after a crash").unwrap();
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
        drop(sess); // release the index writer lock before the server opens a reader

        let ws = Arc::new(SqliteWorkspaceStore::open(&data).unwrap());
        let js = Arc::new(SqliteJobStore::open(&data).unwrap());
        let mut stores = DataPlaneStores::new(ws, js);
        // DataPlaneStores::new returns Arc<Self> with an empty data_dir; set the
        // real index dir in place while the Arc is still unique (refcount 1).
        Arc::get_mut(&mut stores)
            .expect("stores Arc is unique here")
            .data_dir = data.clone();
        let server = SearchServer::new(stores);

        // semantic=true → vector path.
        let inner = server
            .query(Request::new(PbSearchRequest {
                query: "where is the config loader and default data dir".into(),
                workspace_id: coll.clone(),
                agent_scope: String::new(),
                retrieval_method: String::new(),
                top_k: 5,
                config_snapshot: String::new(),
                semantic: true,
            }))
            .await
            .expect("semantic query ok")
            .into_inner();
        assert!(!inner.results.is_empty(), "semantic path should return hits");
        assert_eq!(
            inner.results[0].retrieval_method, "vector",
            "semantic hits must report the vector retrieval_method"
        );

        // semantic=false → BM25 baseline (not the vector method); unchanged behavior.
        let bm25 = server
            .query(Request::new(PbSearchRequest {
                query: "config loader".into(),
                workspace_id: coll.clone(),
                agent_scope: String::new(),
                retrieval_method: String::new(),
                top_k: 5,
                config_snapshot: String::new(),
                semantic: false,
            }))
            .await
            .expect("bm25 query ok")
            .into_inner();
        if let Some(top) = bm25.results.first() {
            assert_ne!(
                top.retrieval_method, "vector",
                "bm25 path must not report the vector method"
            );
        }
    }

    // ----------------------------------------------------------------------
    // task-16.1 (Phase 16 P4 #10) review follow-up: TraceStore↔Persist wiring.
    // These tests cover the AC3/AC4/AC5 seams that the search_persist
    // module's unit tests didn't reach (those exercised SqliteTracePersist
    // in isolation; here we exercise TraceStore::with_persist + write-through
    // + cache-miss fallback + list-supplement paths).
    // ----------------------------------------------------------------------

    fn wiring_trace(query: &str) -> PbRetrievalTrace {
        PbRetrievalTrace {
            trace_id: format!("trace-{query}"),
            query: query.to_string(),
            expanded_query: None,
            candidate_generation_steps: vec!["bm25".into()],
            lexical_candidates_count: 0,
            vector_candidates_count: 0,
            rerank_steps: vec![],
            scope_filter_result: "no-op".into(),
            final_context_count: 0,
            retrieved_chunks: vec![],
        }
    }

    /// AC3 wiring: warm restore populates the hot cache from SQLite contents.
    /// Pre-populate the persist directly, then construct TraceStore::with_
    /// persist and verify the hot cache holds the expected rows.
    #[test]
    fn test_trace_store_with_persist_warm_restore_populates_hot_cache() {
        let dir = temp_data_dir("wiring-warm");
        let persist = Arc::new(SqliteTracePersist::open(&dir).expect("open ok"));
        // Pre-populate persist directly (bypasses TraceStore.put).
        persist.put("k1", &wiring_trace("q1"), "ws", 100).unwrap();
        persist.put("k2", &wiring_trace("q2"), "ws", 200).unwrap();

        let store = TraceStore::with_persist(10, persist);

        assert_eq!(store.len(), 2, "warm restore populates hot cache");
        // Hot cache hit (not SQLite fallback) — get reads from map first.
        assert!(store.get("k1").is_some());
        assert_eq!(store.get("k1").unwrap().query, "q1");
        assert!(store.get("k2").is_some());

        // list returns insertion-order DESC from VecDeque; warm restore
        // inserted oldest-first (k1, k2), so reverse-iteration yields k2, k1.
        let listed = store.list(10);
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].query_id, "k2");
        assert_eq!(listed[1].query_id, "k1");
    }

    /// AC2 + AC5 wiring: TraceStore::put writes through to SQLite. Verify
    /// the row reaches persist by reading directly from a clone of the
    /// persist Arc.
    #[test]
    fn test_trace_store_put_writes_through_to_persist() {
        let dir = temp_data_dir("wiring-wt");
        let persist = Arc::new(SqliteTracePersist::open(&dir).expect("open ok"));
        let mut store = TraceStore::with_persist(10, persist.clone());

        store.put(
            "k-wt".into(),
            wiring_trace("hello"),
            "ws-wt".into(),
            1_700_000_000,
        );

        // Hot cache contains it.
        assert_eq!(store.len(), 1);
        assert_eq!(store.get("k-wt").unwrap().query, "hello");

        // Persist also contains it (write-through path verified).
        let from_persist = persist.get("k-wt").expect("persist get ok");
        assert!(from_persist.is_some());
        assert_eq!(from_persist.unwrap().query, "hello");
        assert_eq!(persist.row_count().unwrap(), 1);
    }

    /// AC4 invariant: TraceStore::put updates the hot cache FIRST, then
    /// best-effort SQLite. The ordering guarantees the hot cache reflects
    /// every put even if the persist layer is unreliable. This test verifies
    /// the invariant by inspecting both hot cache state and confirming
    /// `put` does not panic even after the underlying persist file is
    /// deleted (best-effort on Linux; Windows file lock may keep file open
    /// — either way the hot cache invariant must hold).
    #[test]
    fn test_trace_store_put_hot_cache_intact_even_after_persist_failure() {
        let dir = temp_data_dir("wiring-err");
        let persist = Arc::new(SqliteTracePersist::open(&dir).expect("open ok"));
        let mut store = TraceStore::with_persist(10, persist);

        // Sabotage: remove the data dir. SQLite Connection may still have an
        // open FD on Linux (so subsequent put might still succeed); on Windows
        // file lock typically blocks removal. The test asserts the INVARIANT
        // (hot cache updated) regardless of which branch persist.put takes.
        let _ = std::fs::remove_dir_all(&dir);

        // put — may or may not log a WARN, but must NOT panic and MUST update
        // the hot cache (per the write-order guarantee in TraceStore::put).
        store.put(
            "k-after-sabotage".into(),
            wiring_trace("survived"),
            "ws".into(),
            1,
        );

        // Invariant: hot cache reflects the put.
        assert_eq!(store.len(), 1);
        let got = store.get("k-after-sabotage");
        assert!(got.is_some(), "AC4 invariant: hot cache intact after persist failure");
        assert_eq!(got.unwrap().query, "survived");
    }

    /// AC5 wiring: TraceStore::get falls back to SQLite on cache miss.
    /// Force eviction by using a tiny cap so warm restore evicts the older
    /// row; then verify get() still finds it via the SQLite fallback path.
    #[test]
    fn test_trace_store_get_falls_back_to_persist_on_cache_miss() {
        let dir = temp_data_dir("wiring-getfb");
        let persist = Arc::new(SqliteTracePersist::open(&dir).expect("open ok"));
        // 2 rows in persist before TraceStore construction.
        persist.put("k1", &wiring_trace("q1"), "ws", 100).unwrap();
        persist.put("k2", &wiring_trace("q2"), "ws", 200).unwrap();

        // cap=1 → warm restore loads oldest-first (k1, then k2 which evicts k1).
        let store = TraceStore::with_persist(1, persist);

        assert_eq!(store.len(), 1, "cap=1 enforced after warm restore");
        // k2 should be the survivor (newest, last inserted).
        assert!(store.get("k2").is_some());

        // k1 missed the hot cache → must fall back to SQLite via the
        // persist.get path inside TraceStore::get.
        let got_k1 = store.get("k1");
        assert!(
            got_k1.is_some(),
            "AC5 wiring: k1 served via SQLite fallback after cache eviction"
        );
        assert_eq!(got_k1.unwrap().query, "q1");
    }

    /// AC5 wiring: TraceStore::list supplements from SQLite when the hot
    /// cache has fewer items than `limit`. Use cap=2 so warm restore keeps
    /// only the newest 2; ask for limit=5; expect SQLite to return all 5.
    #[test]
    fn test_trace_store_list_supplements_from_persist_when_cache_short() {
        let dir = temp_data_dir("wiring-listfb");
        let persist = Arc::new(SqliteTracePersist::open(&dir).expect("open ok"));
        // 5 rows ts 100..500.
        for i in 1..=5i64 {
            persist
                .put(
                    &format!("k{i}"),
                    &wiring_trace(&format!("q{i}")),
                    "ws",
                    i * 100,
                )
                .unwrap();
        }

        // cap=2 → warm restore retains the 2 newest after eviction (k4, k5).
        let store = TraceStore::with_persist(2, persist);
        assert_eq!(store.len(), 2, "cap=2 enforced after warm restore of 5 rows");

        // limit=5 but hot cache has only 2 → fallback to SQLite for all 5.
        let listed = store.list(5);
        assert_eq!(listed.len(), 5, "AC5 wiring: SQLite supplements when cache short");

        // Order from SQLite is ts_unix DESC: 500, 400, 300, 200, 100.
        assert_eq!(listed[0].ts_unix, 500);
        assert_eq!(listed[0].query_id, "k5");
        assert_eq!(listed[4].ts_unix, 100);
        assert_eq!(listed[4].query_id, "k1");
    }
}
