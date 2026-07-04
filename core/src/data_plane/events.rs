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

use crate::data_plane::indexing_events::IndexingEventRow;
use crate::memoryops::audit::AuditLogEntry;
use crate::pb_console::events_service_server::EventsService;
use crate::pb_console::{ObservabilityEvent as PbEvent, SubscribeEventsRequest};

use super::DataPlaneStores;

/// task-26.3 (ADR-031 D5): EventBus configuration. Conservative defaults keep
/// the existing behavior unchanged (capacity 1000, single un-partitioned
/// channel — equivalent to task-11.4 `broadcast::channel(1000)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventBusConfig {
    /// Broadcast ring capacity per channel (replaces the hardcoded 1000).
    pub capacity: usize,
    /// When true, `memory.*` and `indexing.*` events get independent broadcast
    /// channels so a high-volume namespace cannot evict the other's events
    /// (ADR-021 D4 / Rollback path `adr-021:153`). Default false (single channel).
    pub partitioned: bool,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            capacity: 1000,
            partitioned: false,
        }
    }
}

impl EventBusConfig {
    /// Read config from the environment with conservative defaults:
    /// `CF_EVENT_BUS_CAPACITY` (positive int; default 1000) +
    /// `CF_EVENT_BUS_PARTITION` (`1`/`true` → partitioned; default off).
    pub fn from_env() -> Self {
        let capacity = std::env::var("CF_EVENT_BUS_CAPACITY")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .filter(|n| *n > 0)
            .unwrap_or(1000);
        let partitioned = matches!(
            std::env::var("CF_EVENT_BUS_PARTITION").ok().as_deref(),
            Some("1") | Some("true") | Some("TRUE")
        );
        Self {
            capacity,
            partitioned,
        }
    }
}

/// task-26.3 (ADR-031 D5): coarse event-type namespace partition. `memory.*` and
/// `indexing.*` are the two high-traffic namespaces ADR-021 D4 calls out; all
/// other event types share the default channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Partition {
    Memory,
    Indexing,
    Other,
}

/// Map an event_type string to its coarse partition.
pub fn partition_of(event_type: &str) -> Partition {
    if event_type.starts_with("memory.") {
        Partition::Memory
    } else if event_type.starts_with("indexing.") {
        Partition::Indexing
    } else {
        Partition::Other
    }
}

/// task-11.4: EventBus — broadcast::Sender wrapper sized at 1000 events.
/// Slow subscribers that fall behind get `RecvError::Lagged(skipped)` and the
/// stream loop logs a warning + continues (rather than breaking the stream).
///
/// task-26.3 (ADR-031 D5): capacity is now configurable and the bus can
/// optionally partition `memory.*` / `indexing.*` onto independent channels
/// (default: single channel, capacity 1000 — identical to task-11.4).
pub struct EventBus {
    /// Default channel — carries everything in single mode, and `Other` events
    /// (plus memory/indexing when un-partitioned) otherwise.
    tx: broadcast::Sender<PbEvent>,
    /// Partition channels (Some only when `config.partitioned`).
    memory_tx: Option<broadcast::Sender<PbEvent>>,
    indexing_tx: Option<broadcast::Sender<PbEvent>>,
    config: EventBusConfig,
}

impl EventBus {
    /// Default capacity = 1000 (task-11.4 §3: matches v0.3 internal evt
    /// convention; Kafka/NATS replacement is OOS per ADR-004 local-first).
    pub fn new() -> Arc<Self> {
        Self::from_config(EventBusConfig::default())
    }

    pub fn with_capacity(cap: usize) -> Arc<Self> {
        Self::from_config(EventBusConfig {
            capacity: cap,
            partitioned: false,
        })
    }

    /// task-26.3 (ADR-031 D5): build from config. Single mode → one channel
    /// (capacity from config). Partitioned → independent `memory` / `indexing`
    /// channels (each at the configured capacity) beside the default channel,
    /// so a high-volume namespace cannot lag-evict the other (ADR-021 D4).
    pub fn from_config(config: EventBusConfig) -> Arc<Self> {
        let (tx, _) = broadcast::channel(config.capacity);
        let (memory_tx, indexing_tx) = if config.partitioned {
            let (m, _) = broadcast::channel(config.capacity);
            let (i, _) = broadcast::channel(config.capacity);
            (Some(m), Some(i))
        } else {
            (None, None)
        };
        Arc::new(Self {
            tx,
            memory_tx,
            indexing_tx,
            config,
        })
    }

    /// task-26.3: configured ring capacity per channel.
    pub fn capacity(&self) -> usize {
        self.config.capacity
    }

    /// task-26.3: whether memory/indexing are on independent channels.
    pub fn partitioned(&self) -> bool {
        self.config.partitioned
    }

    /// Best-effort emit. Returns `usize` subscriber count on success, or
    /// `SendError` if no active subscribers. Caller should swallow SendError
    /// (event lost is acceptable when no one listens — local-first single
    /// user; the event is informational, not durable state).
    ///
    /// task-26.3 (ADR-031 D5): when partitioned, route `memory.*` / `indexing.*`
    /// to their channel; everything else to the default channel.
    // clippy: error type intentionally not boxed — mirrors tokio's broadcast::SendError public API; boxing would ripple to every caller.
    #[allow(clippy::result_large_err)]
    pub fn send(&self, evt: PbEvent) -> Result<usize, broadcast::error::SendError<PbEvent>> {
        if self.config.partitioned {
            match partition_of(&evt.event_type) {
                Partition::Memory => {
                    if let Some(tx) = &self.memory_tx {
                        return tx.send(evt);
                    }
                }
                Partition::Indexing => {
                    if let Some(tx) = &self.indexing_tx {
                        return tx.send(evt);
                    }
                }
                Partition::Other => {}
            }
        }
        self.tx.send(evt)
    }

    /// Subscribe to the default channel (un-partitioned: every event). Kept for
    /// the task-11.4 single-channel contract + existing tests.
    pub fn subscribe(&self) -> broadcast::Receiver<PbEvent> {
        self.tx.subscribe()
    }

    /// task-26.3 (ADR-031 D5): subscribe to all underlying channels. Single mode
    /// → 1 receiver (default); partitioned → default + memory + indexing. The
    /// `EventsServer` forwards every returned receiver into one subscriber stream.
    pub fn subscribe_all(&self) -> Vec<broadcast::Receiver<PbEvent>> {
        let mut v = vec![self.tx.subscribe()];
        if let Some(m) = &self.memory_tx {
            v.push(m.subscribe());
        }
        if let Some(i) = &self.indexing_tx {
            v.push(i.subscribe());
        }
        v
    }

    /// task-26.3 (ADR-031 D5): subscribe to one partition's channel. In single
    /// mode (no partition channel) falls back to the default channel.
    pub fn subscribe_partition(&self, p: Partition) -> broadcast::Receiver<PbEvent> {
        match p {
            Partition::Memory => self.memory_tx.as_ref().unwrap_or(&self.tx).subscribe(),
            Partition::Indexing => self.indexing_tx.as_ref().unwrap_or(&self.tx).subscribe(),
            Partition::Other => self.tx.subscribe(),
        }
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
            // Subscribe to ALL live channels FIRST (task-26.3: default + memory +
            // indexing when partitioned; just default otherwise) so no event
            // emitted after this point is lost while we build the replay batch.
            let subs = event_bus.subscribe_all();
            // task-26.2 / ADR-031 D4: when `since_ts > 0`, replay the memory
            // state-op events the subscriber missed from the persistent audit
            // log (id ASC) before splicing the live stream. Replay event_ids
            // are `evt-audit-{id}` so the SSE client can dedup the splice
            // boundary. Best-effort: audit lock / list failure → no replay.
            //
            // task-43.1 (ADR-048 D3): also replay the indexing lifecycle events
            // (`indexing.progress`/`.cancelled`/`.error`) the subscriber missed
            // from the persistent `indexing_events` store (id ASC), the indexing
            // counterpart of the audit replay. Replay event_ids are
            // `evt-idx-{id}` (independent namespace from `evt-audit-{id}`, so the
            // client dedup's the splice boundary across both). Best-effort: store
            // absent / lock failure → no indexing replay (byte-equivalent to
            // pre-task-43.1). Both replays are sent before the live forward
            // (subscribe-first at line above guarantees no live event is lost).
            let replay: Vec<PbEvent> = if req.since_ts > 0 {
                let mut out: Vec<PbEvent> = self
                    .stores
                    .indexing_event_store
                    .as_ref()
                    .and_then(|s| s.list_since(1000, req.since_ts).ok())
                    .map(|rows| indexing_rows_to_pb_events(&rows))
                    .unwrap_or_default();
                let audit: Vec<PbEvent> = self
                    .stores
                    .audit
                    .as_ref()
                    .and_then(|a| a.lock().ok().and_then(|s| s.list().ok()))
                    .map(|entries| replay_events_from_audit(&entries, req.since_ts))
                    .unwrap_or_default();
                out.extend(audit);
                out
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
                // 2. forward every live channel concurrently into the one stream.
                let mut handles = Vec::with_capacity(subs.len());
                for mut sub in subs {
                    let txc = tx.clone();
                    handles.push(tokio::spawn(async move {
                        loop {
                            match sub.recv().await {
                                Ok(evt) => {
                                    if txc.send(Ok(evt)).await.is_err() {
                                        // Subscriber gRPC stream dropped — exit.
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
                    }));
                }
                // Drop the original sender so the stream closes once every
                // forwarder has ended (all channels closed / subscriber gone).
                drop(tx);
                for h in handles {
                    let _ = h.await;
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
        message: "indexing cancelled by user request".to_string(),
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
/// subscriber can replay events it missed before subscribing (this fulfills the
/// ADR-021 replay-from-audit goal that was once tracked as a future defer).
///
/// `entries` MUST be `id ASC` (as returned by `AuditSink::list()`); the output
/// preserves that order. `since_ts > 0` filters to entries at/after the cutoff
/// (unix seconds); `since_ts == 0` replays all. Non-memory operations
/// (`import` / `search` / ...) are skipped — memory state-op events replay from
/// `audit_log`; `indexing.*` events now have their own persistent source
/// (`indexing_events`, migration 0019) rebuilt by `indexing_rows_to_pb_events`
/// (end-to-end restart-then-replay `[SPEC-DEFER:phase-future.indexing-replay-e2e]`).
/// Each event_id is the deterministic `evt-audit-{audit_id}` so the replay→live
/// splice can dedup.
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

/// task-33.3 (ADR-038 D3): rebuild the `indexing.*` event sequence from the
/// persisted `indexing_events` rows (migration 0019), so a subscriber can
/// replay indexing lifecycle it missed before subscribing — the indexing
/// counterpart of `replay_events_from_audit`.
///
/// `rows` MUST be `id ASC` (as returned by `SqliteIndexingEventStore::list()`);
/// the output preserves that order. `job_id` / `processed` / `total` are taken
/// verbatim from the persisted row (NOT synthesized, ADR-013). `stage` maps to
/// the event_type: `"indexing"` → `indexing.progress`, `"cancelled"` →
/// `indexing.cancelled`, `"error"` → `indexing.error`. Each event_id is the
/// deterministic `evt-idx-{id}` so the replay→live splice can dedup (mirrors
/// the `evt-audit-{id}` pattern). End-to-end restart-then-replay is
/// `[SPEC-DEFER:phase-future.indexing-replay-e2e]`.
pub fn indexing_rows_to_pb_events(rows: &[IndexingEventRow]) -> Vec<PbEvent> {
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let (event_type, severity) = match row.stage.as_str() {
            "cancelled" => ("indexing.cancelled", "info"),
            "error" => ("indexing.error", "error"),
            // "indexing" (progress) is the default; unknown stages fold here so
            // a forward-compat row is never dropped silently.
            _ => ("indexing.progress", "info"),
        };
        let message = match event_type {
            "indexing.cancelled" => "indexing cancelled by user request".to_string(),
            "indexing.error" => format!("indexing failed: {}", row.message),
            _ => format!("indexing progress: {}/{} files", row.processed, row.total),
        };
        // Mirror build_progress_event's payload shape (job_id/processed_files/
        // total_files) so replayed and live events are byte-compatible.
        let payload_json = format!(
            r#"{{"job_id":{},"processed_files":{},"total_files":{}}}"#,
            serde_json::to_string(&row.job_id).unwrap_or_else(|_| String::from("\"\"")),
            row.processed,
            row.total,
        );
        out.push(PbEvent {
            event_id: format!("evt-idx-{}", row.id),
            event_type: event_type.to_string(),
            severity: severity.to_string(),
            source: "contextforge-core".to_string(),
            message,
            ts_unix: row.ts_unix,
            trace_id: None,
            job_id: Some(row.job_id.clone()),
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

    fn idx_row(id: i64, job_id: &str, stage: &str, processed: i64, total: i64, ts: i64, msg: &str) -> IndexingEventRow {
        IndexingEventRow {
            id,
            job_id: job_id.to_string(),
            stage: stage.to_string(),
            processed,
            total,
            message: msg.to_string(),
            ts_unix: ts,
        }
    }

    /// TEST-33.3.1 (Rust): the indexing replay mapper rebuilds `indexing.*`
    /// events in `id ASC` order with real job_id/processed/total taken verbatim
    /// from the persisted rows (not synthesized, ADR-013); stage maps to the
    /// right event_type + severity; event_id is the deterministic `evt-idx-{id}`.
    /// Mirrors TEST-26.2.3.
    #[test]
    fn test_33_3_1_indexing_rows_to_pb_events_id_asc_and_real_fields() {
        let rows = vec![
            idx_row(1, "job-1", "indexing", 2, 5, 100, ""),
            idx_row(2, "job-1", "indexing", 5, 5, 200, ""),
            idx_row(3, "job-1", "cancelled", 0, 0, 300, ""),
            idx_row(4, "job-2", "error", 0, 0, 400, "commit: disk full"),
        ];
        let evs = indexing_rows_to_pb_events(&rows);
        assert_eq!(evs.len(), 4, "every row maps (no drop)");
        // id ASC + deterministic event_id + real fields.
        assert_eq!(evs[0].event_id, "evt-idx-1");
        assert_eq!(evs[0].event_type, "indexing.progress");
        assert_eq!(evs[0].severity, "info");
        assert_eq!(evs[0].source, "contextforge-core");
        assert_eq!(evs[0].job_id, Some("job-1".to_string()));
        assert_eq!(evs[0].ts_unix, 100);
        assert!(evs[0].payload_json.contains("\"job_id\":\"job-1\""));
        assert!(evs[0].payload_json.contains("\"processed_files\":2"));
        assert!(evs[0].payload_json.contains("\"total_files\":5"));
        // progress carries the real counts in its message too.
        assert!(evs[1].message.contains("5/5 files"));
        // cancelled.
        assert_eq!(evs[2].event_id, "evt-idx-3");
        assert_eq!(evs[2].event_type, "indexing.cancelled");
        assert_eq!(evs[2].severity, "info");
        // error: severity error + message carries the persisted detail.
        assert_eq!(evs[3].event_id, "evt-idx-4");
        assert_eq!(evs[3].event_type, "indexing.error");
        assert_eq!(evs[3].severity, "error");
        assert_eq!(evs[3].job_id, Some("job-2".to_string()));
        assert!(evs[3].message.contains("disk full"));
        // empty input → empty output.
        assert!(indexing_rows_to_pb_events(&[]).is_empty());
    }

    // =====================================================================
    // task-26.3 (Phase 26 / ADR-031 D5) — event-bus capacity + partition config.
    // =====================================================================

    fn mk_event(event_type: &str) -> PbEvent {
        PbEvent {
            event_id: format!("evt-{event_type}"),
            event_type: event_type.to_string(),
            severity: "info".to_string(),
            source: "contextforge-core".to_string(),
            message: "test".to_string(),
            ts_unix: 1,
            trace_id: None,
            job_id: None,
            payload_json: "{}".to_string(),
        }
    }

    fn drain(rx: &mut broadcast::Receiver<PbEvent>) -> Vec<String> {
        let mut out = Vec::new();
        while let Ok(evt) = rx.try_recv() {
            out.push(evt.event_type);
        }
        out
    }

    #[test]
    fn test_partition_of_namespaces() {
        assert_eq!(partition_of("memory.pin"), Partition::Memory);
        assert_eq!(partition_of("indexing.progress"), Partition::Indexing);
        assert_eq!(partition_of("core.keepalive"), Partition::Other);
    }

    /// TEST-26.3.1a: default config = capacity 1000, single (un-partitioned)
    /// channel — behaviorally identical to task-11.4 `broadcast::channel(1000)`.
    #[test]
    fn test_event_bus_default_config_single_channel_equiv() {
        let bus = EventBus::from_config(EventBusConfig::default());
        assert_eq!(bus.capacity(), 1000, "default capacity 1000");
        assert!(!bus.partitioned(), "default un-partitioned");
        // subscribe_all returns exactly one receiver in single mode.
        let all = bus.subscribe_all();
        assert_eq!(all.len(), 1, "single channel → 1 receiver");
        // A single subscribe() receiver gets memory + indexing + other events.
        let mut rx = bus.subscribe();
        bus.send(mk_event("memory.pin")).ok();
        bus.send(mk_event("indexing.progress")).ok();
        bus.send(mk_event("core.keepalive")).ok();
        let got = drain(&mut rx);
        assert_eq!(got.len(), 3, "single channel carries all namespaces: {got:?}");
    }

    /// TEST-26.3.1b: capacity is configurable.
    #[test]
    fn test_event_bus_capacity_configurable() {
        let bus = EventBus::from_config(EventBusConfig { capacity: 42, partitioned: false });
        assert_eq!(bus.capacity(), 42);
        assert!(!bus.partitioned());
    }

    /// TEST-26.3.1c: partitioned mode routes memory.* / indexing.* onto
    /// independent channels (a high-volume namespace cannot evict the other).
    #[test]
    fn test_event_bus_partition_routes_by_namespace() {
        let bus = EventBus::from_config(EventBusConfig { capacity: 16, partitioned: true });
        assert!(bus.partitioned());
        // Subscribe to each partition BEFORE sending (broadcast = from-now).
        let mut mem_rx = bus.subscribe_partition(Partition::Memory);
        let mut idx_rx = bus.subscribe_partition(Partition::Indexing);
        let mut other_rx = bus.subscribe_partition(Partition::Other);
        bus.send(mk_event("memory.pin")).ok();
        bus.send(mk_event("memory.soft_delete")).ok();
        bus.send(mk_event("indexing.progress")).ok();
        bus.send(mk_event("core.keepalive")).ok();
        let mem = drain(&mut mem_rx);
        let idx = drain(&mut idx_rx);
        let other = drain(&mut other_rx);
        assert_eq!(mem, vec!["memory.pin", "memory.soft_delete"], "memory channel isolated");
        assert_eq!(idx, vec!["indexing.progress"], "indexing channel isolated");
        assert_eq!(other, vec!["core.keepalive"], "other → default channel");
        // subscribe_all (partitioned) returns default + memory + indexing.
        assert_eq!(bus.subscribe_all().len(), 3, "partitioned → 3 receivers");
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

    /// task-43.1 helper: build an `EventsServer` whose `DataPlaneStores` carry
    /// a real `EventBus` + `audit` (memory state-op entries via `record`, ts
    /// stamped now) + `indexing_event_store` (indexing lifecycle rows via
    /// `append`, ts stamped now), so `subscribe(since_ts > 0)` exercises the
    /// indexing + audit replay splice (ADR-048 D3). `n_audit`/`n_indexing` count
    /// the entries to seed; a small `since_ts` (e.g. 1) keeps all (now >> 1).
    fn server_with_replay_sources(
        n_audit: usize,
        n_indexing: usize,
    ) -> (EventsServer, std::path::PathBuf) {
        use crate::data_plane::indexing_events::SqliteIndexingEventStore;
        use crate::memoryops::audit::{AuditEvent, AuditOperation, AuditSink};
        let dir = temp_data_dir("replay-splice");
        let ws = Arc::new(SqliteWorkspaceStore::open(&dir).unwrap());
        let js = Arc::new(SqliteJobStore::open(&dir).unwrap());
        let audit_sink = Arc::new(std::sync::Mutex::new(
            AuditSink::open(&dir, "memory").unwrap(),
        ));
        {
            let mut sink = audit_sink.lock().unwrap();
            for i in 0..n_audit {
                sink.record(AuditEvent {
                    operation: if i % 2 == 0 {
                        AuditOperation::MemoryPin
                    } else {
                        AuditOperation::MemorySoftDelete
                    },
                    collection: "memory".to_string(),
                    source: "console-api".to_string(),
                    result_count: 1,
                    redaction_count: 0,
                    query: None,
                    redacted_terms: vec![],
                    chunk_ids: vec![format!("m{i}")],
                    export_total_byte_count: None,
                })
                .unwrap();
            }
        }
        let idx_store = Arc::new(SqliteIndexingEventStore::open(&dir).unwrap());
        for i in 0..n_indexing {
            idx_store
                .append("job-1", if i % 2 == 0 { "indexing" } else { "cancelled" }, 1, 1, "")
                .unwrap();
        }
        let event_bus = EventBus::new();
        let stores = Arc::new(DataPlaneStores {
            workspace_store: ws,
            job_store: js,
            job_runner: None,
            data_dir: std::path::PathBuf::new(),
            event_bus: Some(event_bus),
            memory: None,
            audit: Some(audit_sink),
            eval: None,
            trace_persist: None,
            indexing_event_store: Some(idx_store),
        });
        (EventsServer::new(stores), dir)
    }

    /// TEST-43.1.2 (task-43.1 / ADR-048 D2/D3): `subscribe(since_ts > 0)`
    /// splices indexing replay (evt-idx-*, id ASC) BEFORE audit replay
    /// (evt-audit-*, id ASC) BEFORE the live stream — the indexing counterpart
    /// of the task-26.2 audit replay, now wired into the live subscribe path.
    #[tokio::test]
    async fn test_43_1_2_subscribe_splices_indexing_then_audit_then_live() {
        let (server, _dir) = server_with_replay_sources(2, 2);
        // since_ts=1 keeps all (indexing/audit ts = now >> 1).
        let resp = server
            .subscribe(Request::new(SubscribeEventsRequest {
                job_id: None,
                workspace_id: None,
                since_ts: 1,
                last_event_id: String::new(),
            }))
            .await
            .expect("subscribe ok");
        let mut stream = resp.into_inner();
        // Order: indexing replay (2) → audit replay (2). Then live forwarder
        // blocks on broadcast (no sender here) — we read exactly 4 and stop.
        let mut got = Vec::new();
        for _ in 0..4 {
            let evt = stream.next().await.expect("event").expect("ok");
            got.push(evt.event_id);
        }
        // Indexing replay first (evt-idx-*), id ASC.
        assert!(got[0].starts_with("evt-idx-"), "first is indexing replay: {}", got[0]);
        assert!(got[1].starts_with("evt-idx-"), "second is indexing replay: {}", got[1]);
        let idx0: i64 = got[0].trim_start_matches("evt-idx-").parse().unwrap();
        let idx1: i64 = got[1].trim_start_matches("evt-idx-").parse().unwrap();
        assert!(idx0 < idx1, "indexing replay id ASC: {idx0} < {idx1}");
        // Audit replay after (evt-audit-*), id ASC.
        assert!(got[2].starts_with("evt-audit-"), "third is audit replay: {}", got[2]);
        assert!(got[3].starts_with("evt-audit-"), "fourth is audit replay: {}", got[3]);
        let aud0: i64 = got[2].trim_start_matches("evt-audit-").parse().unwrap();
        let aud1: i64 = got[3].trim_start_matches("evt-audit-").parse().unwrap();
        assert!(aud0 < aud1, "audit replay id ASC: {aud0} < {aud1}");
    }

    /// TEST-43.1.2b: since_ts <= 0 → no replay (byte-equivalent to pre-task-43.1).
    /// The subscribe path takes the `else { Vec::new() }` branch, so the stream
    /// goes straight to the live forwarder (which blocks with no sender).
    #[tokio::test]
    async fn test_43_1_2b_since_ts_zero_no_replay_byte_equiv() {
        let (server, _dir) = server_with_replay_sources(1, 1);
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
        // No replay emitted — the live forwarder has no sender, so it blocks.
        // Timeout confirms NO event arrives quickly (replay batch was empty).
        let none = tokio::time::timeout(std::time::Duration::from_millis(150), stream.next()).await;
        assert!(none.is_err(), "since_ts=0 → no replay (timeout = no early event)");
    }

    /// TEST-43.1.2c: indexing_event_store=None → no indexing replay, but audit
    /// replay still splices (the store-absent退化 path, byte-equivalent to
    /// pre-task-43.1 which only had audit replay).
    #[tokio::test]
    async fn test_43_1_2c_store_none_no_indexing_replay() {
        use crate::memoryops::audit::{AuditEvent, AuditOperation, AuditSink};
        let dir = temp_data_dir("store-none");
        let ws = Arc::new(SqliteWorkspaceStore::open(&dir).unwrap());
        let js = Arc::new(SqliteJobStore::open(&dir).unwrap());
        let audit_sink = Arc::new(std::sync::Mutex::new(
            AuditSink::open(&dir, "memory").unwrap(),
        ));
        audit_sink.lock().unwrap().record(AuditEvent {
            operation: AuditOperation::MemoryPin,
            collection: "memory".to_string(),
            source: "console-api".to_string(),
            result_count: 1,
            redaction_count: 0,
            query: None,
            redacted_terms: vec![],
            chunk_ids: vec!["m9".to_string()],
            export_total_byte_count: None,
        }).unwrap();
        let event_bus = EventBus::new();
        let stores = Arc::new(DataPlaneStores {
            workspace_store: ws,
            job_store: js,
            job_runner: None,
            data_dir: std::path::PathBuf::new(),
            event_bus: Some(event_bus),
            memory: None,
            audit: Some(audit_sink),
            eval: None,
            trace_persist: None,
            indexing_event_store: None, // <- the退化 path under test
        });
        let server = EventsServer::new(stores);
        let resp = server
            .subscribe(Request::new(SubscribeEventsRequest {
                job_id: None,
                workspace_id: None,
                since_ts: 1,
                last_event_id: String::new(),
            }))
            .await
            .expect("subscribe ok");
        let mut stream = resp.into_inner();
        let first = stream.next().await.expect("event").expect("ok");
        assert!(
            first.event_id.starts_with("evt-audit-"),
            "store=None → only audit replay, no evt-idx-*: {}",
            first.event_id
        );
    }
}
