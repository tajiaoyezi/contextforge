//! task-11.4: real `EventsServer` impl backed by `tokio::sync::broadcast::
//! channel(1000)` `EventBus`. `JobRunner` progress callback emits
//! `indexing.progress` / `indexing.cancelled` / `indexing.error` events via
//! `EventBus.send`, and gRPC `Subscribe` streams them out to subscribers.
//!
//! `RecvError::Lagged` (subscriber slower than producer) → log warning +
//! continue; `RecvError::Closed` → end stream gracefully.

use std::sync::Arc;

use tokio::sync::broadcast;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use crate::pb_console::events_service_server::EventsService;
use crate::pb_console::{ObservabilityEvent as PbEvent, SubscribeEventsRequest};

use super::DataPlaneStores;

/// task-11.4: EventBus — broadcast::Sender wrapper sized at 1000 events.
/// Slow subscribers that fall behind get `RecvError::Lagged(skipped)` and the
/// stream loop logs a warning + continues (rather than breaking the stream).
pub struct EventBus {
    tx: broadcast::Sender<PbEvent>,
}

impl EventBus {
    /// Default capacity = 1000 (task-11.4 §3: matches v0.3 internal evt
    /// convention; Kafka/NATS replacement is OOS per ADR-004 local-first).
    pub fn new() -> Arc<Self> {
        let (tx, _) = broadcast::channel(1000);
        Arc::new(Self { tx })
    }

    pub fn with_capacity(cap: usize) -> Arc<Self> {
        let (tx, _) = broadcast::channel(cap);
        Arc::new(Self { tx })
    }

    /// Best-effort emit. Returns `usize` subscriber count on success, or
    /// `SendError` if no active subscribers. Caller should swallow SendError
    /// (event lost is acceptable when no one listens — local-first single
    /// user; the event is informational, not durable state).
    pub fn send(&self, evt: PbEvent) -> Result<usize, broadcast::error::SendError<PbEvent>> {
        self.tx.send(evt)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<PbEvent> {
        self.tx.subscribe()
    }
}

pub struct EventsServer {
    stores: Arc<DataPlaneStores>,
}

impl EventsServer {
    pub fn new(stores: Arc<DataPlaneStores>) -> Self {
        Self { stores }
    }
}

const EVENTS_STREAM_CAPACITY: usize = 64;

#[tonic::async_trait]
impl EventsService for EventsServer {
    type SubscribeStream = ReceiverStream<Result<PbEvent, Status>>;

    async fn subscribe(
        &self,
        _req: Request<SubscribeEventsRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(EVENTS_STREAM_CAPACITY);

        // task-11.4 §6 AC3/AC4: real broadcast subscription.
        if let Some(event_bus) = self.stores.event_bus.as_ref() {
            let mut sub = event_bus.subscribe();
            tokio::spawn(async move {
                loop {
                    match sub.recv().await {
                        Ok(evt) => {
                            if tx.send(Ok(evt)).await.is_err() {
                                // Subscriber gRPC stream dropped — exit loop.
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            eprintln!(
                                "WARN events subscriber lagged by {n} events; continuing"
                            );
                            continue;
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            });
            return Ok(Response::new(ReceiverStream::new(rx)));
        }

        // Fallback (no EventBus configured, e.g. task-11.1 tests): emit a
        // single keepalive event then close stream.
        let evt = PbEvent {
            event_id: format!("evt-keepalive-{}", now_unix_nanos()),
            event_type: "core.keepalive".to_string(),
            severity: "info".to_string(),
            source: "contextforge-core".to_string(),
            message: "EventBus not configured; placeholder keepalive".to_string(),
            ts_unix: now_unix(),
            trace_id: None,
            job_id: None,
            payload_json: "{}".to_string(),
        };
        let _ = tx.send(Ok(evt)).await;
        drop(tx);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

/// Build an `indexing.progress` event for emission by JobRunner heartbeat.
pub fn build_progress_event(
    job_id: &str,
    processed_files: i64,
    total_files: i64,
) -> PbEvent {
    PbEvent {
        event_id: format!("evt-progress-{}", now_unix_nanos()),
        event_type: "indexing.progress".to_string(),
        severity: "info".to_string(),
        source: "contextforge-core".to_string(),
        message: format!(
            "indexing progress: {processed_files}/{total_files} files"
        ),
        ts_unix: now_unix(),
        trace_id: None,
        job_id: Some(job_id.to_string()),
        payload_json: format!(
            r#"{{"job_id":"{job_id}","processed_files":{processed_files},"total_files":{total_files}}}"#
        ),
    }
}

pub fn build_cancelled_event(job_id: &str) -> PbEvent {
    PbEvent {
        event_id: format!("evt-cancelled-{}", now_unix_nanos()),
        event_type: "indexing.cancelled".to_string(),
        severity: "info".to_string(),
        source: "contextforge-core".to_string(),
        message: format!("indexing cancelled by user request"),
        ts_unix: now_unix(),
        trace_id: None,
        job_id: Some(job_id.to_string()),
        payload_json: format!(r#"{{"job_id":"{job_id}"}}"#),
    }
}

pub fn build_error_event(job_id: &str, error: &str) -> PbEvent {
    PbEvent {
        event_id: format!("evt-error-{}", now_unix_nanos()),
        event_type: "indexing.error".to_string(),
        severity: "error".to_string(),
        source: "contextforge-core".to_string(),
        message: format!("indexing failed: {error}"),
        ts_unix: now_unix(),
        trace_id: None,
        job_id: Some(job_id.to_string()),
        payload_json: format!(
            r#"{{"job_id":"{job_id}","error":{}}}"#,
            serde_json::to_string(error).unwrap_or_else(|_| String::from("\"\""))
        ),
    }
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn now_unix_nanos() -> u128 {
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
    use tokio_stream::StreamExt;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_data_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "cf-events-server-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn fresh_server() -> EventsServer {
        let dir = temp_data_dir("keepalive");
        let ws = Arc::new(SqliteWorkspaceStore::open(&dir).unwrap());
        let js = Arc::new(SqliteJobStore::open(&dir).unwrap());
        EventsServer::new(DataPlaneStores::new(ws, js))
    }

    #[tokio::test]
    async fn test_events_server_keepalive() {
        let server = fresh_server();
        let resp = server
            .subscribe(Request::new(SubscribeEventsRequest {
                job_id: None,
                workspace_id: None,
            }))
            .await
            .expect("subscribe ok");
        let mut stream = resp.into_inner();
        // First emit: keepalive
        let first = stream.next().await.expect("at least one event");
        let evt = first.expect("event Ok");
        assert_eq!(evt.event_type, "core.keepalive");
        assert_eq!(evt.source, "contextforge-core");
        assert!(evt.ts_unix > 0);
        // After drop(tx), stream returns None
        let second = stream.next().await;
        assert!(second.is_none(), "stream should close after keepalive");
    }
}
