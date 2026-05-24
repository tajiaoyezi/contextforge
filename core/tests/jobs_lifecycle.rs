//! task-10.3 §6 AC4 / AC5 integration — IndexJob lifecycle + heartbeat updates
//! end-to-end against a real temp data_dir.

use contextforge_core::jobs::{
    status, IndexerBackend, JobOutcome, JobProgressEvent, JobRunner, JobStore, ProgressDecision,
    SqliteJobStore,
};
use contextforge_core::workspace::{SqliteWorkspaceStore, WorkspaceCreate, WorkspaceStore};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn unique_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    env::temp_dir().join(format!(
        "cfg-jobs-itest-{}-{}-{}",
        label,
        std::process::id(),
        nanos
    ))
}

fn setup_workspace(label: &str) -> (PathBuf, Arc<dyn JobStore>) {
    let dir = unique_dir(label);
    let _ = fs::remove_dir_all(&dir);
    let ws_store = SqliteWorkspaceStore::open(&dir).expect("ws open");
    let root_path = env::temp_dir()
        .join(format!("cfg-jobs-itest-root-{}", label))
        .to_string_lossy()
        .into_owned();
    ws_store
        .create(&WorkspaceCreate {
            workspace_id: "demo".to_string(),
            name: "demo".to_string(),
            root_path,
            ..Default::default()
        })
        .expect("ws create");
    let job_store = SqliteJobStore::open(&dir).expect("job open");
    let store_arc: Arc<dyn JobStore> = Arc::new(job_store);
    (dir, store_arc)
}

struct CountingIndexer {
    total: i64,
    delay_ms: u64,
}

impl IndexerBackend for CountingIndexer {
    fn index(
        &self,
        _source: &Path,
        _data: &Path,
        _workspace_id: &str,
        on_progress: &mut dyn FnMut(&JobProgressEvent) -> ProgressDecision,
    ) -> Result<JobOutcome, String> {
        let mut processed = 0;
        let mut cancelled = false;
        for i in 1..=self.total {
            processed = i;
            let evt = JobProgressEvent {
                processed_files: i,
                total_files: self.total,
                failed_files: 0,
                skipped_files: 0,
                current_file: format!("/fake/{i}.md"),
                stage: "index".to_string(),
            };
            if matches!(on_progress(&evt), ProgressDecision::Cancel) {
                cancelled = true;
                break;
            }
            if self.delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(self.delay_ms));
            }
        }
        Ok(JobOutcome {
            processed_files: processed,
            total_files: self.total,
            failed_files: 0,
            skipped_files: 0,
            cancelled,
        })
    }
}

/// AC4 — full lifecycle queued → running → succeeded.
#[tokio::test]
async fn lifecycle_queued_running_succeeded() {
    let (dir, store_arc) = setup_workspace("ac4-ok");
    let indexer = Arc::new(CountingIndexer { total: 10, delay_ms: 0 });
    let mut runner = JobRunner::new(store_arc.clone(), indexer);
    runner.heartbeat_every_n_files = 2;
    runner.heartbeat_interval_secs = 0;
    let j = store_arc.enqueue("demo", "rest").expect("enqueue");
    assert_eq!(j.status, status::QUEUED);
    runner
        .run_one(&j.job_id, &env::temp_dir(), &env::temp_dir())
        .await
        .expect("run");
    let final_job = store_arc.get(&j.job_id).expect("get").expect("present");
    assert_eq!(final_job.status, status::SUCCEEDED);
    assert_eq!(final_job.processed_files, 10);
    assert!(final_job.started_at_unix.is_some());
    assert!(final_job.finished_at_unix.is_some());
    let _ = fs::remove_dir_all(&dir);
}

/// AC4 — mid-run cancel within 2 seconds.
#[tokio::test]
async fn lifecycle_cancel_within_2s() {
    let (dir, store_arc) = setup_workspace("ac4-cancel");
    let indexer = Arc::new(CountingIndexer { total: 200, delay_ms: 15 });
    let mut runner = JobRunner::new(store_arc.clone(), indexer);
    runner.heartbeat_every_n_files = 3;
    runner.heartbeat_interval_secs = 0;
    let j = store_arc.enqueue("demo", "rest").expect("enqueue");
    let store_for_cancel = store_arc.clone();
    let job_id_for_cancel = j.job_id.clone();
    let cancel_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(80)).await;
        let _ = store_for_cancel.request_cancel(&job_id_for_cancel);
    });
    let start = std::time::Instant::now();
    runner
        .run_one(&j.job_id, &env::temp_dir(), &env::temp_dir())
        .await
        .expect("run");
    let elapsed = start.elapsed();
    let _ = cancel_task.await;
    let final_job = store_arc.get(&j.job_id).expect("get").expect("present");
    assert_eq!(final_job.status, status::CANCELLED);
    assert!(
        elapsed < Duration::from_secs(2),
        "cancel must take <2s; got {elapsed:?}"
    );
    assert!(
        final_job.processed_files < 200,
        "cancel must stop before full 200; got {}",
        final_job.processed_files
    );
    let _ = fs::remove_dir_all(&dir);
}

/// AC5 — heartbeat updates: long-ish job updates last_heartbeat_at across the run.
#[tokio::test]
async fn heartbeat_updates() {
    let (dir, store_arc) = setup_workspace("ac5-heartbeat");
    let indexer = Arc::new(CountingIndexer { total: 30, delay_ms: 25 });
    let mut runner = JobRunner::new(store_arc.clone(), indexer);
    // heartbeat every 5 files so we get multiple writes across the 30-file run
    runner.heartbeat_every_n_files = 5;
    runner.heartbeat_interval_secs = 0;
    let j = store_arc.enqueue("demo", "rest").expect("enqueue");
    let initial_hb = store_arc
        .get(&j.job_id)
        .expect("get")
        .expect("present")
        .last_heartbeat_at_unix;
    runner
        .run_one(&j.job_id, &env::temp_dir(), &env::temp_dir())
        .await
        .expect("run");
    let final_job = store_arc.get(&j.job_id).expect("get").expect("present");
    assert_eq!(final_job.status, status::SUCCEEDED);
    // heartbeat written during run (mark_running sets initial; update_progress
    // writes more; processed_files crossed multiple 5-boundary thresholds).
    assert!(final_job.last_heartbeat_at_unix.is_some(), "heartbeat must be set");
    assert_ne!(
        initial_hb, final_job.last_heartbeat_at_unix,
        "heartbeat must advance across the run"
    );
    // processed_files written by heartbeat callback should reflect mid-run
    // progress; final update_progress in run_one sets it to outcome.
    assert!(final_job.processed_files >= 30);
    let _ = fs::remove_dir_all(&dir);
}
