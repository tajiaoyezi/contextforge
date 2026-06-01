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

use crate::memoryops::audit::AuditLogEntry;
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
        req: Request<SubscribeEventsRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let req = req.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(EVENTS_STREAM_CAPACITY);

        // task-11.4 §6 AC3/AC4: real broadcast subscription.
        if let Some(event_bus) = self.stores.event_bus.as_ref() {
            // Subscribe to the live broadcast FIRST so no event emitted after
            // this point is lost while we build the replay batch below.
            let mut sub = event_bus.subscribe();
            // task-26.2 / ADR-031 D4: when `since_ts > 0`, replay the memory
            // state-op events the subscriber missed from the persistent audit
            // log (id ASC) before splicing the live stream. Replay event_ids
            // are `evt-audit-{id}` so the SSE client can dedup the splice
            // boundary. Best-effort: audit lock / list failure → no replay.
            let replay: Vec<PbEvent> = if req.since_ts > 0 {
                self.stores
                    .audit
                    .as_ref()
                    .and_then(|a| a.lock().ok().and_then(|s| s.list().ok()))
                    .map(|entries| replay_events_from_audit(&entries, req.since_ts))
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            tokio::spawn(async move {
                // 1. replay historical (audit) events first, in id ASC order.
                for evt in replay {
                    if tx.send(Ok(evt)).await.is_err() {
                        return;
                    }
                }
                // 2. then forward the live broadcast.
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

/// task-26.2 / ADR-031 D4: map a persisted audit-log operation string to the
/// `(event_type, op)` pair used when replaying it as an `ObservabilityEvent`.
/// Mirrors `data_plane::memory::audit_op_to_event_type` (Pin/Unpin share the
/// `memory.pin` event_type; `op` in payload disambiguates). Non-memory audit
/// operations return `None` (no persistent observability replay source).
fn audit_op_str_to_event(op: &str) -> Option<(&'static str, &'static str)> {
    match op {
        "memory_pin" => Some(("memory.pin", "pin")),
        "memory_unpin" => Some(("memory.pin", "unpin")),
        "memory_deprecate" => Some(("memory.deprecate", "deprecate")),
        "memory_soft_delete" => Some(("memory.soft_delete", "soft_delete")),
        _ => None,
    }
}

/// task-26.2 / ADR-031 D4: rebuild the `ObservabilityEvent` sequence for memory
/// state-op events from the persistent `audit_log` (ADR-021 D1 桥接源), so a
/// subscriber can replay events it missed before subscribing (兑现 ADR-021
/// `[SPEC-DEFER:phase-future.events-replay-from-audit]`).
///
/// `entries` MUST be `id ASC` (as returned by `AuditSink::list()`); the output
/// preserves that order. `since_ts > 0` filters to entries at/after the cutoff
/// (unix seconds); `since_ts == 0` replays all. Non-memory operations
/// (`import` / `search` / ...) are skipped — only memory state-op events have a
/// persistent replay source (indexing events lack one,
/// `[SPEC-DEFER:phase-future.indexing-event-persistence]`). Each event_id is the
/// deterministic `evt-audit-{audit_id}` so the replay→live splice can dedup.
pub fn replay_events_from_audit(entries: &[AuditLogEntry], since_ts: i64) -> Vec<PbEvent> {
    let mut out = Vec::new();
    for entry in entries {
        let Some((event_type, op_str)) = audit_op_str_to_event(&entry.operation) else {
            continue;
        };
        let ts: i64 = entry.timestamp.parse().unwrap_or(0);
        if since_ts > 0 && ts < since_ts {
            continue;
        }
        let memory_id = entry.chunk_ids.first().cloned().unwrap_or_default();
        let payload_json = format!(
            r#"{{"memory_id":{},"op":"{}"}}"#,
            serde_json::to_string(&memory_id).unwrap_or_else(|_| String::from("\"\"")),
            op_str,
        );
        out.push(PbEvent {
            event_id: format!("evt-audit-{}", entry.id),
            event_type: event_type.to_string(),
            severity: "info".to_string(),
            source: "contextforge-core".to_string(),
            message: format!("memory {op_str}: {memory_id}"),
            ts_unix: ts,
            trace_id: None,
            job_id: None,
            payload_json,
        });
    }
    out
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

    // =====================================================================
    // task-26.2 (Phase 26 / ADR-031 D4) — audit-log replay reconstruction.
    // =====================================================================

    fn audit_entry(id: i64, op: &str, memory_id: &str, ts: &str) -> AuditLogEntry {
        AuditLogEntry {
            id,
            operation: op.to_string(),
            collection: "memory".to_string(),
            source: "console-api".to_string(),
            result_count: 1,
            redaction_count: 0,
            timestamp: ts.to_string(),
            query_hash: None,
            query_length: None,
            redacted_terms: vec![],
            chunk_ids: if memory_id.is_empty() {
                vec![]
            } else {
                vec![memory_id.to_string()]
            },
            export_total_byte_count: None,
        }
    }

    /// TEST-26.2.3 (Rust): replay rebuilds memory state-op events in audit
    /// `id ASC` order with the ADR-021 D3 field mapping; non-memory ops are
    /// skipped; `since_ts` filters by cutoff.
    #[test]
    fn test_replay_events_from_audit_id_asc_mapping_and_since_ts() {
        let entries = vec![
            audit_entry(1, "memory_pin", "m1", "100"),
            audit_entry(2, "search", "", "150"), // non-memory → skipped
            audit_entry(3, "memory_unpin", "m1", "200"),
            audit_entry(4, "memory_soft_delete", "m2", "300"),
        ];
        let evs = replay_events_from_audit(&entries, 0);
        assert_eq!(evs.len(), 3, "search op skipped (no replay source)");
        // id ASC order preserved.
        assert_eq!(evs[0].event_id, "evt-audit-1");
        assert_eq!(evs[0].event_type, "memory.pin");
        assert_eq!(evs[0].source, "contextforge-core");
        assert_eq!(evs[0].severity, "info");
        assert_eq!(evs[0].ts_unix, 100);
        assert!(evs[0].payload_json.contains("\"op\":\"pin\""));
        assert!(evs[0].payload_json.contains("\"memory_id\":\"m1\""));
        // Unpin shares memory.pin event_type (ADR-021 D2); op disambiguates.
        assert_eq!(evs[1].event_type, "memory.pin");
        assert!(evs[1].payload_json.contains("\"op\":\"unpin\""));
        assert_eq!(evs[2].event_type, "memory.soft_delete");
        assert_eq!(evs[2].ts_unix, 300);
        // since_ts cutoff: only ts >= 200.
        let recent = replay_events_from_audit(&entries, 200);
        assert_eq!(recent.len(), 2, "only ts>=200 (unpin@200 + soft_delete@300)");
        assert_eq!(recent[0].event_id, "evt-audit-3");
        assert_eq!(recent[1].event_id, "evt-audit-4");
    }

    #[tokio::test]
    async fn test_events_server_keepalive() {
        let server = fresh_server();
        let resp = server
            .subscribe(Request::new(SubscribeEventsRequest {
                job_id: None,
                workspace_id: None,
                since_ts: 0,
                last_event_id: String::new(),
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
