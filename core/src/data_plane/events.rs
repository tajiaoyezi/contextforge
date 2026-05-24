//! task-11.1 §3 / §6 AC5: `EventsServer` 占位实现 (real EventBus wiring 在
//! [SPEC-OWNER:task-11.4])。
//!
//! 本 task: `Subscribe` 返 keepalive only — 一条 `core.keepalive` 事件后
//! close stream。task-11.4 替换为 `tokio::sync::broadcast::channel(1000)`
//! backed `EventBus` + 真接 `JobRunner` progress callback emit
//! `indexing.progress`。

use std::sync::Arc;

use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use crate::pb_console::events_service_server::EventsService;
use crate::pb_console::{ObservabilityEvent as PbEvent, SubscribeEventsRequest};

use super::DataPlaneStores;

pub struct EventsServer {
    #[allow(dead_code)] // task-11.4 will use stores.event_bus
    stores: Arc<DataPlaneStores>,
}

impl EventsServer {
    pub fn new(stores: Arc<DataPlaneStores>) -> Self {
        Self { stores }
    }
}

const EVENTS_STREAM_CAPACITY: usize = 8;

#[tonic::async_trait]
impl EventsService for EventsServer {
    type SubscribeStream = ReceiverStream<Result<PbEvent, Status>>;

    async fn subscribe(
        &self,
        _req: Request<SubscribeEventsRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(EVENTS_STREAM_CAPACITY);
        // task-11.4 [SPEC-OWNER:task-11.4]: replace with broadcast::Receiver
        // subscription loop. task-11.1 emits ONE keepalive event then closes,
        // so the gRPC stream wire is exercised by integration tests.
        let evt = PbEvent {
            event_id: format!("evt-keepalive-{}", now_unix_nanos()),
            event_type: "core.keepalive".to_string(),
            severity: "info".to_string(),
            source: "contextforge-core".to_string(),
            message: "task-11.1 placeholder keepalive [SPEC-OWNER:task-11.4]".to_string(),
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
