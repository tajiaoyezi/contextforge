//! task-11.3 (Phase 11): real `IndexerBackend` impl wrapping
//! `IndexSession::index_path_cancellable` (task-11.3 §10 trade-off T1 added
//! method) — JobRunner spawns this for every `JobService.Enqueue` to truly
//! trigger the Rust indexer (Tantivy + SQLite chunks) instead of the
//! task-10.3 in-memory stub.
//!
//! Refs: ADR-016 §D1/§D3 / task-11.3 §6 AC1/AC2/AC3/AC5

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::chunker::{ChunkPolicy, Provenance};
use crate::indexer::{IndexProgressSnapshot, IndexSession};
use crate::scanner::{default_denylist, ScanOptions};

use super::{IndexerBackend, JobOutcome, JobProgressEvent, ProgressDecision};

/// Real indexer-backend used by `JobRunner` in production: opens a fresh
/// `IndexSession` for the workspace's `root_path`, drives
/// `index_path_cancellable`, and translates `IndexProgressSnapshot` →
/// `JobProgressEvent` so the `on_progress` callback can plumb heartbeat +
/// cancel-check through `SqliteJobStore`.
///
/// task-11.4: optionally holds an `Arc<EventBus>` so heartbeat boundaries emit
/// `indexing.progress` events to `EventsService.Subscribe` server stream.
/// Cancel observed (closure returns `ProgressDecision::Cancel`) emits an
/// `indexing.cancelled` event before returning.
///
/// Cancel is co-operative at file granularity (between scanned files);
/// see `index_path_cancellable` doc for trade-off detail.
pub struct IndexSessionBackend {
    /// Override scan options (e.g. test fixtures may want different denylist
    /// or max_file_bytes). None = use default_denylist() + 10 MB max.
    pub scan_options_override: Option<ScanOptions>,
    /// Override chunk policy. None = ChunkPolicy::default().
    pub chunk_policy_override: Option<ChunkPolicy>,
    /// task-11.4: optional broadcast event bus. When Some, progress callback
    /// emits `indexing.progress` at every heartbeat boundary + final
    /// `indexing.cancelled` / `indexing.error` event on terminal.
    pub event_bus: Option<Arc<crate::data_plane::events::EventBus>>,
    /// task-11.4: job_id used for event_bus event payload tagging. Set by
    /// `with_job_context` before each index() call.
    pub job_id_context: parking_lot_like::Mutex<String>,
    /// task-33.3 (ADR-038 D3): optional persistent indexing-event sink. When
    /// Some, each emit point ALSO best-effort persists a lifecycle row (replay
    /// source) in addition to the in-memory `event_bus` broadcast. None (tests
    /// / task-11.4 baseline) → broadcast-only, behavior unchanged.
    pub indexing_event_store:
        Option<Arc<crate::data_plane::indexing_events::SqliteIndexingEventStore>>,
}

/// Tiny parking_lot-like Mutex shim built on std (we already depend on std::
/// sync::Mutex; named for self-documentation only).
mod parking_lot_like {
    pub use std::sync::Mutex;
}

impl Default for IndexSessionBackend {
    fn default() -> Self {
        Self {
            scan_options_override: None,
            chunk_policy_override: None,
            event_bus: None,
            job_id_context: parking_lot_like::Mutex::new(String::new()),
            indexing_event_store: None,
        }
    }
}

impl IndexSessionBackend {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// task-11.4: build with EventBus wired so progress events flow to
    /// `EventsService.Subscribe` subscribers.
    pub fn with_event_bus(event_bus: Arc<crate::data_plane::events::EventBus>) -> Arc<Self> {
        Arc::new(Self {
            scan_options_override: None,
            chunk_policy_override: None,
            event_bus: Some(event_bus),
            job_id_context: parking_lot_like::Mutex::new(String::new()),
            indexing_event_store: None,
        })
    }

    /// task-33.3 (ADR-038 D3): build with both the broadcast EventBus and a
    /// persistent indexing-event sink (replay source). Add-only superset of
    /// `with_event_bus`; emit points additionally persist a lifecycle row.
    pub fn with_event_bus_and_indexing_store(
        event_bus: Arc<crate::data_plane::events::EventBus>,
        indexing_event_store: Arc<crate::data_plane::indexing_events::SqliteIndexingEventStore>,
    ) -> Arc<Self> {
        Arc::new(Self {
            scan_options_override: None,
            chunk_policy_override: None,
            event_bus: Some(event_bus),
            job_id_context: parking_lot_like::Mutex::new(String::new()),
            indexing_event_store: Some(indexing_event_store),
        })
    }

    /// Set the current job_id context (called by JobServer::Enqueue right
    /// before invoking JobRunner.run_one). The progress callback reads this
    /// for event payload tagging.
    pub fn set_job_context(&self, job_id: &str) {
        if let Ok(mut g) = self.job_id_context.lock() {
            *g = job_id.to_string();
        }
    }

    fn current_job_id(&self) -> String {
        self.job_id_context
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    fn scan_options(&self) -> ScanOptions {
        if let Some(ref s) = self.scan_options_override {
            return s.clone();
        }
        ScanOptions {
            denylist: default_denylist(),
            allowlist: Vec::new(),
            allow_denylist_override: false,
            dry_run: false,
            max_file_bytes: 10 * 1024 * 1024,
        }
    }

    fn chunk_policy(&self) -> ChunkPolicy {
        self.chunk_policy_override.clone().unwrap_or_default()
    }
}

impl IndexerBackend for IndexSessionBackend {
    fn index(
        &self,
        source: &Path,
        data: &Path,
        workspace_id: &str,
        job_id: &str,
        on_progress: &mut dyn FnMut(&JobProgressEvent) -> ProgressDecision,
    ) -> Result<JobOutcome, String> {
        // task-11.4: stash job_id so the emit-progress closure can tag events.
        self.set_job_context(job_id);
        // task-11.3 §6 AC2: open IndexSession for the workspace; workspace_id
        // maps 1:1 to collection_id (ADR-015 D2).
        let mut session = IndexSession::open(data, workspace_id)
            .map_err(|e| format!("IndexSession::open({}): {}", workspace_id, e))?;

        // Cancel token captured by the inner closure — JobRunner's heartbeat
        // logic sets it when SqliteJobStore.cancel_requested=1 is observed.
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_for_closure = cancel_flag.clone();
        let mut total_estimate: i64 = 0;

        let event_bus = self.event_bus.clone();
        // task-33.3: clone the persistent sink handle for the progress closure
        // (terminal emits below use self.indexing_event_store directly).
        let indexing_store = self.indexing_event_store.clone();
        let job_id_context = self.current_job_id();
        let on_inner = |snap: &IndexProgressSnapshot<'_>| {
            // Sum of all seen file outcomes (indexed + denied + redaction skip)
            // is the "files processed" envelope; total = estimate hits final
            // value after scan_path completes (it's the report length).
            let processed = snap.files_processed as i64
                + snap.files_skipped_denied as i64
                + snap.files_skipped_redaction as i64;
            total_estimate = total_estimate.max(processed);
            let evt = JobProgressEvent {
                processed_files: processed,
                total_files: total_estimate,
                failed_files: 0,
                skipped_files: (snap.files_skipped_denied + snap.files_skipped_redaction) as i64,
                current_file: snap
                    .current_file
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                stage: "indexing".to_string(),
            };
            // task-11.4 §6 AC4: emit progress event to EventBus (best-effort;
            // SendError swallowed since no subscribers is acceptable).
            if let Some(eb) = &event_bus {
                if !job_id_context.is_empty() {
                    let pb_evt = crate::data_plane::events::build_progress_event(
                        &job_id_context,
                        evt.processed_files,
                        evt.total_files,
                    );
                    let _ = eb.send(pb_evt);
                }
            }
            // task-33.3 (ADR-038 D3): additionally persist a progress row as a
            // replay source (best-effort — write failure does not block
            // indexing, mirroring the eb.send best-effort above).
            if let Some(store) = &indexing_store {
                if !job_id_context.is_empty() {
                    let _ = store.append(
                        &job_id_context,
                        "indexing",
                        evt.processed_files,
                        evt.total_files,
                        "",
                    );
                }
            }
            if matches!(on_progress(&evt), ProgressDecision::Cancel) {
                cancel_for_closure.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        };

        let scan_opts = self.scan_options();
        let policy = self.chunk_policy();
        let provenance: Vec<Provenance> = Vec::new();

        let job_id_for_terminal = self.current_job_id();
        let (stats, cancelled) = session
            .index_path_cancellable(source, &scan_opts, &policy, provenance, on_inner, &cancel_flag)
            .map_err(|e| {
                // task-11.4 §6 AC4: emit indexing.error event on terminal failure.
                if let Some(eb) = &self.event_bus {
                    if !job_id_for_terminal.is_empty() {
                        let pb_evt = crate::data_plane::events::build_error_event(
                            &job_id_for_terminal,
                            &format!("IndexSession::index_path_cancellable: {e}"),
                        );
                        let _ = eb.send(pb_evt);
                    }
                }
                // task-33.3: persist the error row as a replay source (best-effort).
                if let Some(store) = &self.indexing_event_store {
                    if !job_id_for_terminal.is_empty() {
                        let _ = store.append(
                            &job_id_for_terminal,
                            "error",
                            0,
                            0,
                            &format!("IndexSession::index_path_cancellable: {e}"),
                        );
                    }
                }
                format!("IndexSession::index_path_cancellable: {e}")
            })?;

        // Always attempt to commit Tantivy writer (mirrors server.rs's contract
        // — indexer is only durable after commit).
        session.commit().map_err(|e| {
            if let Some(eb) = &self.event_bus {
                if !job_id_for_terminal.is_empty() {
                    let pb_evt = crate::data_plane::events::build_error_event(
                        &job_id_for_terminal,
                        &format!("commit: {e}"),
                    );
                    let _ = eb.send(pb_evt);
                }
            }
            // task-33.3: persist the commit-error row as a replay source.
            if let Some(store) = &self.indexing_event_store {
                if !job_id_for_terminal.is_empty() {
                    let _ = store.append(&job_id_for_terminal, "error", 0, 0, &format!("commit: {e}"));
                }
            }
            format!("commit: {e}")
        })?;

        // task-11.4 §6 AC4: emit terminal cancel event when cancelled.
        if cancelled {
            if let Some(eb) = &self.event_bus {
                if !job_id_for_terminal.is_empty() {
                    let pb_evt = crate::data_plane::events::build_cancelled_event(&job_id_for_terminal);
                    let _ = eb.send(pb_evt);
                }
            }
            // task-33.3: persist the cancelled row as a replay source.
            if let Some(store) = &self.indexing_event_store {
                if !job_id_for_terminal.is_empty() {
                    let _ = store.append(&job_id_for_terminal, "cancelled", 0, 0, "");
                }
            }
        }

        Ok(JobOutcome {
            processed_files: stats.files_indexed as i64,
            total_files: stats.files_indexed as i64
                + stats.files_skipped_denied as i64
                + stats.files_skipped_redaction as i64,
            failed_files: 0,
            skipped_files: (stats.files_skipped_denied + stats.files_skipped_redaction) as i64,
            cancelled,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::{JobError, SqliteJobStore};
    use crate::workspace::{SqliteWorkspaceStore, WorkspaceCreate, WorkspaceStore};
    use std::path::PathBuf;
    use std::sync::atomic::Ordering;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "cf-isb-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn fixture_dir() -> PathBuf {
        // task-11.3 §6 AC2: real fixture in test/fixtures/index-job-real/
        // (5 markdown files each containing "contextforge" multiple times).
        let manifest = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(manifest)
            .parent()
            .unwrap()
            .join("test")
            .join("fixtures")
            .join("index-job-real")
    }

    fn ensure_workspace(
        data_dir: &Path,
        workspace_id: &str,
    ) -> Result<(SqliteWorkspaceStore, SqliteJobStore), JobError> {
        let ws = SqliteWorkspaceStore::open(data_dir).expect("ws open");
        ws.create(&WorkspaceCreate {
            workspace_id: workspace_id.to_string(),
            name: "fixture".into(),
            root_path: fixture_dir().to_string_lossy().to_string(),
            allowlist: vec![],
            denylist: vec![],
        })
        .expect("create ws");
        let js = SqliteJobStore::open(data_dir)?;
        Ok((ws, js))
    }

    #[test]
    fn test_index_session_backend_index_fixture_repo() {
        let data_dir = temp_dir("fixture-index");
        let workspace_id = "ws-isb-fixture";
        let (_ws, _js) = ensure_workspace(&data_dir, workspace_id).unwrap();

        let backend = IndexSessionBackend::default();
        let mut events = Vec::new();
        let mut on_progress = |evt: &JobProgressEvent| -> ProgressDecision {
            events.push(evt.clone());
            ProgressDecision::Continue
        };
        let outcome = backend
            .index(&fixture_dir(), &data_dir, workspace_id, "test-job-id", &mut on_progress)
            .expect("index ok");
        assert!(outcome.processed_files >= 5, "≥5 files indexed; got {}", outcome.processed_files);
        assert!(!outcome.cancelled);
        assert!(events.len() >= 5, "progress callback called ≥5 times; got {}", events.len());
    }

    #[test]
    fn test_index_session_backend_cancel_stops_iteration() {
        let data_dir = temp_dir("fixture-cancel");
        let workspace_id = "ws-isb-cancel";
        let (_ws, _js) = ensure_workspace(&data_dir, workspace_id).unwrap();

        let backend = IndexSessionBackend::default();
        let mut events = Vec::new();
        let mut on_progress = |evt: &JobProgressEvent| -> ProgressDecision {
            events.push(evt.clone());
            // Cancel after the very first file.
            ProgressDecision::Cancel
        };
        let outcome = backend
            .index(&fixture_dir(), &data_dir, workspace_id, "test-job-id", &mut on_progress)
            .expect("index ok despite cancel");
        // cancel_token was set after the first progress callback; the second
        // iteration check at file boundary should observe it + break.
        assert!(outcome.cancelled, "outcome should be marked cancelled");
        // We expect strictly fewer than 5 files to be fully processed since
        // we cancel after the first progress callback.
        assert!(
            outcome.processed_files < 5,
            "expected partial processing; got {}",
            outcome.processed_files
        );
        // Discard outcome but verify the cancel order didn't break commit.
        // (The IndexSession committed everything indexed before cancel.)
        let _ = outcome;
        let _ = Ordering::Relaxed; // touch for clarity
    }

    /// TEST-33.3.2 (emit half): when an indexing-event store is wired, the
    /// progress emit point persists real lifecycle rows (job_id/processed/total)
    /// as a replay source, and the replay mapper rebuilds indexing.* events
    /// from them. Proves the emit-point persistence is actually exercised by a
    /// real index run (not just the store's direct round-trip).
    #[test]
    fn test_33_3_2_emit_points_persist_on_fixture_index() {
        use crate::data_plane::events::indexing_rows_to_pb_events;
        use crate::data_plane::indexing_events::SqliteIndexingEventStore;

        let data_dir = temp_dir("emit-persist");
        let workspace_id = "ws-isb-emit";
        let (_ws, _js) = ensure_workspace(&data_dir, workspace_id).unwrap();

        let store = Arc::new(SqliteIndexingEventStore::open(&data_dir).expect("store open"));
        // Wire the store (event_bus None — persistence is independent of broadcast).
        let backend = IndexSessionBackend {
            scan_options_override: None,
            chunk_policy_override: None,
            event_bus: None,
            job_id_context: std::sync::Mutex::new(String::new()),
            indexing_event_store: Some(store.clone()),
        };
        let mut on_progress =
            |_evt: &JobProgressEvent| -> ProgressDecision { ProgressDecision::Continue };
        let outcome = backend
            .index(&fixture_dir(), &data_dir, workspace_id, "job-emit-1", &mut on_progress)
            .expect("index ok");
        assert!(outcome.processed_files >= 5);

        let rows = store.list(1000).expect("list ok");
        assert!(!rows.is_empty(), "progress emit point persisted at least one row");
        assert!(
            rows.iter().all(|r| r.job_id == "job-emit-1"),
            "rows tagged with the real job_id"
        );
        assert!(
            rows.iter().any(|r| r.stage == "indexing" && r.total > 0),
            "at least one progress row carries a real total (>0): {rows:?}"
        );
        // Replay mapper rebuilds indexing.progress events from the persisted rows.
        let evs = indexing_rows_to_pb_events(&rows);
        assert!(evs.iter().all(|e| e.event_type == "indexing.progress"));
        assert!(evs.iter().all(|e| e.job_id == Some("job-emit-1".to_string())));
    }
}
