//! task-15.6 (Phase 15 P2 #7 / ADR-020): HealthService gRPC handler.
//!
//! Wraps [`crate::health::HealthChecker`] in a tonic service. opt-in:
//! Console UI's `/v1/health?detailed=true` REST handler dispatches here via
//! the Go grpcclient; the basic binary `/v1/health` path stays untouched.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::health::HealthChecker;
use crate::pb_console::health_service_server::HealthService;
use crate::pb_console::{
    ComponentHealth as PbComponentHealth, DetailedHealthRequest, DetailedHealthResponse,
};

use super::DataPlaneStores;

pub struct HealthCheckServer {
    checker: HealthChecker,
}

impl HealthCheckServer {
    pub fn new(stores: Arc<DataPlaneStores>) -> Self {
        Self {
            checker: HealthChecker::new(stores),
        }
    }
}

#[tonic::async_trait]
impl HealthService for HealthCheckServer {
    async fn get_detailed(
        &self,
        _req: Request<DetailedHealthRequest>,
    ) -> Result<Response<DetailedHealthResponse>, Status> {
        let detailed = self.checker.check_all();
        let components: Vec<PbComponentHealth> = detailed
            .components
            .into_iter()
            .map(|c| PbComponentHealth {
                name: c.name.to_string(),
                status: c.status.as_str().to_string(),
                latency_ms: c.latency_ms,
                error_reason: c.error_reason.unwrap_or_default(),
            })
            .collect();
        Ok(Response::new(DetailedHealthResponse {
            overall_status: detailed.overall.as_str().to_string(),
            components,
            total_latency_ms: detailed.total_latency_ms,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::SqliteJobStore;
    use crate::workspace::SqliteWorkspaceStore;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(name: &str) -> PathBuf {
        let seq = SEQ.fetch_add(1, Ordering::SeqCst);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "cf-health-server-{name}-{}-{nanos}-{seq}",
            std::process::id()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn fresh_server() -> HealthCheckServer {
        let dir = temp_dir("base");
        let ws = Arc::new(SqliteWorkspaceStore::open(&dir).unwrap());
        let js = Arc::new(SqliteJobStore::open(&dir).unwrap());
        HealthCheckServer::new(DataPlaneStores::new(ws, js))
    }

    #[tokio::test]
    async fn test_get_detailed_returns_5_components() {
        let server = fresh_server();
        let resp = server
            .get_detailed(Request::new(DetailedHealthRequest {}))
            .await
            .expect("get_detailed ok");
        let inner = resp.into_inner();
        assert_eq!(inner.components.len(), 5);
        let names: Vec<&str> = inner.components.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["db", "index", "embed", "retriever", "eval"]);
        // overall_status must be one of the 3 known values.
        assert!(matches!(
            inner.overall_status.as_str(),
            "healthy" | "degraded" | "unreachable"
        ));
    }
}
