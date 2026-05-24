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
/// Cancel is co-operative at file granularity (between scanned files);
/// see `index_path_cancellable` doc for trade-off detail.
pub struct IndexSessionBackend {
    /// Override scan options (e.g. test fixtures may want different denylist
    /// or max_file_bytes). None = use default_denylist() + 10 MB max.
    pub scan_options_override: Option<ScanOptions>,
    /// Override chunk policy. None = ChunkPolicy::default().
    pub chunk_policy_override: Option<ChunkPolicy>,
}

impl Default for IndexSessionBackend {
    fn default() -> Self {
        Self {
            scan_options_override: None,
            chunk_policy_override: None,
        }
    }
}

impl IndexSessionBackend {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
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
        on_progress: &mut dyn FnMut(&JobProgressEvent) -> ProgressDecision,
    ) -> Result<JobOutcome, String> {
        // task-11.3 §6 AC2: open IndexSession for the workspace; workspace_id
        // maps 1:1 to collection_id (ADR-015 D2).
        let mut session = IndexSession::open(data, workspace_id)
            .map_err(|e| format!("IndexSession::open({}): {}", workspace_id, e))?;

        // Cancel token captured by the inner closure — JobRunner's heartbeat
        // logic sets it when SqliteJobStore.cancel_requested=1 is observed.
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_for_closure = cancel_flag.clone();
        let mut total_estimate: i64 = 0;

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
            if matches!(on_progress(&evt), ProgressDecision::Cancel) {
                cancel_for_closure.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        };

        let scan_opts = self.scan_options();
        let policy = self.chunk_policy();
        let provenance: Vec<Provenance> = Vec::new();

        let (stats, cancelled) = session
            .index_path_cancellable(source, &scan_opts, &policy, provenance, on_inner, &cancel_flag)
            .map_err(|e| format!("IndexSession::index_path_cancellable: {e}"))?;

        // Always attempt to commit Tantivy writer (mirrors server.rs's contract
        // — indexer is only durable after commit).
        session.commit().map_err(|e| format!("commit: {e}"))?;

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
    use crate::jobs::{JobError, JobStore, SqliteJobStore};
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
        data_dir: &PathBuf,
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
            .index(&fixture_dir(), &data_dir, workspace_id, &mut on_progress)
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
            .index(&fixture_dir(), &data_dir, workspace_id, &mut on_progress)
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
}
