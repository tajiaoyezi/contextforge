//! task-15.6 (Phase 15 P2 #7 / ADR-020): 5-link health probes.
//!
//! Each probe runs synchronously with a soft latency target (≤ 40ms each /
//! ≤ 500ms total). Failures degrade the per-component status to `degraded`
//! or `unreachable`; ADR-020 D4 aggregation rule rolls these up into the
//! overall status.
//!
//! Probes:
//!   - `db`        — SQLite `SELECT 1` on workspaces.db
//!   - `index`     — Tantivy Index::open_in_dir reads at least one collection
//!   - `embed`     — embed provider configuration check (env / file)
//!   - `retriever` — `top_k=1` query exercise on the first openable collection
//!   - `eval`      — SqliteEvalStore::open round-trip
//!
//! The probes are intentionally read-only and do not touch the actual
//! retriever cache / Tantivy reader for normal traffic (ADR-020 §Trade-offs).

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::data_plane::DataPlaneStores;
use crate::workspace::WorkspaceStore;

/// Per-component status aligned with `pb_console::ComponentHealth.status`
/// values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unreachable,
}

impl HealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Degraded => "degraded",
            HealthStatus::Unreachable => "unreachable",
        }
    }
}

/// Single probe result. `latency_ms` is the wall-clock cost of the probe;
/// `error_reason` is None when status=Healthy.
#[derive(Debug, Clone)]
pub struct ComponentResult {
    pub name: &'static str,
    pub status: HealthStatus,
    pub latency_ms: i64,
    pub error_reason: Option<String>,
}

/// Aggregated result for ADR-020 D4 rollup.
#[derive(Debug, Clone)]
pub struct DetailedHealth {
    pub overall: HealthStatus,
    pub components: Vec<ComponentResult>,
    pub total_latency_ms: i64,
}

/// HealthChecker owns the shared DataPlaneStores + the data_dir override so
/// individual probes can open per-collection Tantivy / SqliteEvalStore handles
/// without going through the cached reader (the cached reader is too
/// long-lived for a health snapshot).
pub struct HealthChecker {
    stores: Arc<DataPlaneStores>,
}

impl HealthChecker {
    pub fn new(stores: Arc<DataPlaneStores>) -> Self {
        Self { stores }
    }

    /// Run all 5 probes synchronously. Total wall-clock under ~500ms in
    /// nominal conditions; per-probe failures degrade individual statuses
    /// without aborting the sweep.
    pub fn check_all(&self) -> DetailedHealth {
        let started = Instant::now();
        let mut components = Vec::with_capacity(5);
        components.push(self.probe_db());
        components.push(self.probe_index());
        components.push(self.probe_embed());
        components.push(self.probe_retriever());
        components.push(self.probe_eval());
        let overall = aggregate_status(&components);
        DetailedHealth {
            overall,
            components,
            total_latency_ms: started.elapsed().as_millis() as i64,
        }
    }

    fn probe_db(&self) -> ComponentResult {
        let start = Instant::now();
        // SqliteWorkspaceStore.list() is the cheapest read-only probe we
        // already expose; an Err signals connection failure or schema drift.
        let result = self.stores.workspace_store.list();
        let elapsed = start.elapsed().as_millis() as i64;
        match result {
            Ok(_) => ComponentResult {
                name: "db",
                status: HealthStatus::Healthy,
                latency_ms: elapsed,
                error_reason: None,
            },
            Err(e) => ComponentResult {
                name: "db",
                status: HealthStatus::Degraded,
                latency_ms: elapsed,
                error_reason: Some(format!("sqlite workspace list: {e}")),
            },
        }
    }

    fn probe_index(&self) -> ComponentResult {
        let start = Instant::now();
        if self.stores.data_dir.as_os_str().is_empty() {
            return ComponentResult {
                name: "index",
                status: HealthStatus::Degraded,
                latency_ms: start.elapsed().as_millis() as i64,
                error_reason: Some("data_dir not configured".into()),
            };
        }
        // Try to enumerate workspaces; if any opens cleanly via Retriever we
        // consider the index probe healthy. Empty workspace set is healthy
        // (a fresh daemon with no data is still operational).
        let workspaces = match self.stores.workspace_store.list() {
            Ok(w) => w,
            Err(e) => {
                return ComponentResult {
                    name: "index",
                    status: HealthStatus::Degraded,
                    latency_ms: start.elapsed().as_millis() as i64,
                    error_reason: Some(format!("workspace list: {e}")),
                };
            }
        };
        if workspaces.is_empty() {
            return ComponentResult {
                name: "index",
                status: HealthStatus::Healthy,
                latency_ms: start.elapsed().as_millis() as i64,
                error_reason: None,
            };
        }
        let mut last_err: Option<String> = None;
        for ws in &workspaces {
            match crate::retriever::Retriever::open(&self.stores.data_dir, &ws.workspace_id) {
                Ok(_) => {
                    return ComponentResult {
                        name: "index",
                        status: HealthStatus::Healthy,
                        latency_ms: start.elapsed().as_millis() as i64,
                        error_reason: None,
                    };
                }
                Err(crate::retriever::RetrieverError::CollectionNotFound(_)) => {
                    last_err = Some(format!(
                        "collection not found for workspace {}",
                        ws.workspace_id
                    ));
                    continue;
                }
                Err(e) => {
                    last_err = Some(format!("retriever open: {e}"));
                    continue;
                }
            }
        }
        ComponentResult {
            name: "index",
            status: HealthStatus::Degraded,
            latency_ms: start.elapsed().as_millis() as i64,
            error_reason: last_err.or_else(|| Some("no openable collection".into())),
        }
    }

    fn probe_embed(&self) -> ComponentResult {
        let start = Instant::now();
        // task-22.4: opt-in remote reachability probe — feature-gated (embedding-remote) AND
        // explicitly enabled (CONTEXTFORGE_EMBED_REMOTE_PROBE). The default build / CI never compiles
        // this branch and never hits the network (ADR-004 local-first; ADR-013 — real remote
        // reachability is deferred, [SPEC-DEFER:phase-future.embed-remote-probe]).
        #[cfg(feature = "embedding-remote")]
        if std::env::var("CONTEXTFORGE_EMBED_REMOTE_PROBE").is_ok() {
            return self.probe_embed_remote(start);
        }
        // ADR-020 D1: config-only check; we do NOT call the remote provider
        // (rate-limit / secret-exposure risk). Sources checked, in priority:
        //   1. CONTEXTFORGE_EMBED_PROVIDER env var
        //   2. config.toml in data_dir (existence of an [embed] section is
        //      enough — full validation belongs in the embed pipeline itself)
        if std::env::var("CONTEXTFORGE_EMBED_PROVIDER").is_ok() {
            return ComponentResult {
                name: "embed",
                status: HealthStatus::Healthy,
                latency_ms: start.elapsed().as_millis() as i64,
                error_reason: None,
            };
        }
        let cfg = self.stores.data_dir.join("config.toml");
        if cfg.exists() && contains_embed_section(&cfg) {
            return ComponentResult {
                name: "embed",
                status: HealthStatus::Healthy,
                latency_ms: start.elapsed().as_millis() as i64,
                error_reason: None,
            };
        }
        ComponentResult {
            name: "embed",
            status: HealthStatus::Degraded,
            latency_ms: start.elapsed().as_millis() as i64,
            error_reason: Some(
                "embed provider not configured (set CONTEXTFORGE_EMBED_PROVIDER or [embed] in config.toml)"
                    .into(),
            ),
        }
    }

    /// task-22.4: opt-in remote reachability probe (feature `embedding-remote`). Any HTTP response —
    /// including 4xx, since the endpoint answered — counts as reachable; only a transport error is
    /// Degraded. Never compiled in the default build (ADR-004). Real reachability against a live
    /// provider is deferred — CI has no endpoint/keys ([SPEC-DEFER:phase-future.embed-remote-probe],
    /// ADR-013).
    #[cfg(feature = "embedding-remote")]
    fn probe_embed_remote(&self, start: Instant) -> ComponentResult {
        let endpoint = std::env::var("CONTEXTFORGE_REMOTE_ENDPOINT").unwrap_or_default();
        if endpoint.is_empty() {
            return ComponentResult {
                name: "embed",
                status: HealthStatus::Degraded,
                latency_ms: start.elapsed().as_millis() as i64,
                error_reason: Some(
                    "remote embed probe opt-in but CONTEXTFORGE_REMOTE_ENDPOINT unset".into(),
                ),
            };
        }
        let reachable = match ureq::head(&endpoint).timeout(Duration::from_secs(3)).call() {
            Ok(_) => true,
            Err(ureq::Error::Status(_, _)) => true, // endpoint answered (e.g. 401/404) → reachable
            Err(_) => false,
        };
        ComponentResult {
            name: "embed",
            status: if reachable {
                HealthStatus::Healthy
            } else {
                HealthStatus::Degraded
            },
            latency_ms: start.elapsed().as_millis() as i64,
            error_reason: if reachable {
                None
            } else {
                Some("remote embed endpoint unreachable".into())
            },
        }
    }

    fn probe_retriever(&self) -> ComponentResult {
        let start = Instant::now();
        if self.stores.data_dir.as_os_str().is_empty() {
            return ComponentResult {
                name: "retriever",
                status: HealthStatus::Degraded,
                latency_ms: start.elapsed().as_millis() as i64,
                error_reason: Some("data_dir not configured".into()),
            };
        }
        let workspaces = match self.stores.workspace_store.list() {
            Ok(w) => w,
            Err(e) => {
                return ComponentResult {
                    name: "retriever",
                    status: HealthStatus::Degraded,
                    latency_ms: start.elapsed().as_millis() as i64,
                    error_reason: Some(format!("workspace list: {e}")),
                };
            }
        };
        if workspaces.is_empty() {
            return ComponentResult {
                name: "retriever",
                status: HealthStatus::Healthy,
                latency_ms: start.elapsed().as_millis() as i64,
                error_reason: None,
            };
        }
        for ws in &workspaces {
            let retriever =
                match crate::retriever::Retriever::open(&self.stores.data_dir, &ws.workspace_id) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
            let opts = crate::retriever::SearchOptions {
                query: "health".into(),
                top_k: 1,
                filters: crate::retriever::SearchFilters::default(),
                explain: false,
            };
            match retriever.search(&opts) {
                Ok(_) => {
                    return ComponentResult {
                        name: "retriever",
                        status: HealthStatus::Healthy,
                        latency_ms: start.elapsed().as_millis() as i64,
                        error_reason: None,
                    };
                }
                Err(e) => {
                    return ComponentResult {
                        name: "retriever",
                        status: HealthStatus::Degraded,
                        latency_ms: start.elapsed().as_millis() as i64,
                        error_reason: Some(format!("retriever search: {e}")),
                    };
                }
            }
        }
        // None opened cleanly — index probe is the better signal here, so we
        // stay Healthy (treat as "no traffic yet").
        ComponentResult {
            name: "retriever",
            status: HealthStatus::Healthy,
            latency_ms: start.elapsed().as_millis() as i64,
            error_reason: None,
        }
    }

    fn probe_eval(&self) -> ComponentResult {
        let start = Instant::now();
        if self.stores.eval.is_some() {
            // Eval store wired into DataPlaneStores already passed migrations
            // at open time; consider it healthy.
            return ComponentResult {
                name: "eval",
                status: HealthStatus::Healthy,
                latency_ms: start.elapsed().as_millis() as i64,
                error_reason: None,
            };
        }
        if self.stores.data_dir.as_os_str().is_empty() {
            return ComponentResult {
                name: "eval",
                status: HealthStatus::Degraded,
                latency_ms: start.elapsed().as_millis() as i64,
                error_reason: Some("data_dir not configured".into()),
            };
        }
        // Last-resort probe: try opening eval.db ad-hoc. Failure → degraded.
        match crate::eval::SqliteEvalStore::open(&self.stores.data_dir) {
            Ok(_) => ComponentResult {
                name: "eval",
                status: HealthStatus::Healthy,
                latency_ms: start.elapsed().as_millis() as i64,
                error_reason: None,
            },
            Err(e) => ComponentResult {
                name: "eval",
                status: HealthStatus::Degraded,
                latency_ms: start.elapsed().as_millis() as i64,
                error_reason: Some(format!("eval store open: {e}")),
            },
        }
    }
}

fn contains_embed_section(path: &Path) -> bool {
    // Minimal scan — open the file and look for "[embed]" header. Heavy TOML
    // parsing belongs in the actual embed pipeline.
    let _ = Duration::from_millis(0); // placeholder to keep imports tidy if file IO is later replaced
    match std::fs::read_to_string(path) {
        Ok(s) => s.lines().any(|l| l.trim().starts_with("[embed]")),
        Err(_) => false,
    }
}

/// ADR-020 D4: any unreachable → unreachable; any degraded → degraded; else
/// healthy.
pub fn aggregate_status(components: &[ComponentResult]) -> HealthStatus {
    let mut worst = HealthStatus::Healthy;
    for c in components {
        match c.status {
            HealthStatus::Unreachable => return HealthStatus::Unreachable,
            HealthStatus::Degraded if worst != HealthStatus::Unreachable => {
                worst = HealthStatus::Degraded;
            }
            _ => {}
        }
    }
    worst
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
            "cf-health-{name}-{}-{nanos}-{seq}",
            std::process::id()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn fresh_checker() -> HealthChecker {
        let dir = temp_dir("base");
        let ws = Arc::new(SqliteWorkspaceStore::open(&dir).unwrap());
        let js = Arc::new(SqliteJobStore::open(&dir).unwrap());
        let stores = DataPlaneStores::new(ws, js);
        HealthChecker::new(stores)
    }

    #[test]
    fn test_aggregate_status_all_healthy() {
        let comps = vec![
            ComponentResult {
                name: "a",
                status: HealthStatus::Healthy,
                latency_ms: 1,
                error_reason: None,
            };
            5
        ];
        assert_eq!(aggregate_status(&comps), HealthStatus::Healthy);
    }

    #[test]
    fn test_aggregate_status_degraded_wins_over_healthy() {
        let comps = vec![
            ComponentResult {
                name: "ok",
                status: HealthStatus::Healthy,
                latency_ms: 1,
                error_reason: None,
            },
            ComponentResult {
                name: "warn",
                status: HealthStatus::Degraded,
                latency_ms: 1,
                error_reason: Some("x".into()),
            },
        ];
        assert_eq!(aggregate_status(&comps), HealthStatus::Degraded);
    }

    #[test]
    fn test_aggregate_status_unreachable_wins_overall() {
        let comps = vec![
            ComponentResult {
                name: "a",
                status: HealthStatus::Healthy,
                latency_ms: 1,
                error_reason: None,
            },
            ComponentResult {
                name: "b",
                status: HealthStatus::Degraded,
                latency_ms: 1,
                error_reason: Some("x".into()),
            },
            ComponentResult {
                name: "c",
                status: HealthStatus::Unreachable,
                latency_ms: 1,
                error_reason: Some("y".into()),
            },
        ];
        assert_eq!(aggregate_status(&comps), HealthStatus::Unreachable);
    }

    #[test]
    fn test_check_all_returns_5_components_and_under_500ms() {
        // Acquire the env mutex too — other tests touch CONTEXTFORGE_EMBED_PROVIDER
        // and we want a deterministic 5-component sweep.
        let _g = EMBED_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let checker = fresh_checker();
        let detailed = checker.check_all();
        assert_eq!(detailed.components.len(), 5);
        let names: Vec<&str> = detailed.components.iter().map(|c| c.name).collect();
        assert_eq!(names, vec!["db", "index", "embed", "retriever", "eval"]);
        assert!(
            detailed.total_latency_ms < 500,
            "total latency exceeded 500ms: got {}ms",
            detailed.total_latency_ms
        );
    }

    #[test]
    fn test_check_all_db_healthy_on_fresh_store() {
        // db probe is independent of embed env, but acquire the lock for
        // serialization with the env-touching tests above (no probe state
        // is leaked, just keeps cargo test runs deterministic).
        let _g = EMBED_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let checker = fresh_checker();
        let detailed = checker.check_all();
        let db = detailed
            .components
            .iter()
            .find(|c| c.name == "db")
            .expect("db component present");
        assert_eq!(db.status, HealthStatus::Healthy);
        assert!(db.error_reason.is_none());
    }

    // Mutex serializes the two embed env-var tests so they don't race when
    // cargo test runs them in parallel inside the same process. Lock 持续到
    // check_all 调用结束。
    static EMBED_ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_check_all_embed_degraded_when_not_configured() {
        let _g = EMBED_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        std::env::remove_var("CONTEXTFORGE_EMBED_PROVIDER");
        let checker = fresh_checker();
        let detailed = checker.check_all();
        let embed = detailed
            .components
            .iter()
            .find(|c| c.name == "embed")
            .expect("embed component present");
        assert_eq!(embed.status, HealthStatus::Degraded);
        assert!(embed.error_reason.is_some());
    }

    #[test]
    fn test_check_all_embed_healthy_when_env_set() {
        let _g = EMBED_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        std::env::set_var("CONTEXTFORGE_EMBED_PROVIDER", "openai");
        let checker = fresh_checker();
        let detailed = checker.check_all();
        let embed = detailed
            .components
            .iter()
            .find(|c| c.name == "embed")
            .expect("embed component present");
        assert_eq!(embed.status, HealthStatus::Healthy);
        std::env::remove_var("CONTEXTFORGE_EMBED_PROVIDER");
    }

    // TEST-22.4.1 — AC1: in the default build the remote-probe opt-in env var is inert (the
    // feature-gated branch is not compiled), so probe_embed stays config-only — ADR-020 D1 behavior
    // unchanged + ADR-004 default-no-network. Real remote reachability is feature+network-gated and
    // deferred (ADR-013). Gated to the default build: under embedding-remote the opt-in would route
    // to probe_embed_remote (which needs a live endpoint), so this config-only assertion only holds
    // when the feature is off.
    #[cfg(not(feature = "embedding-remote"))]
    #[test]
    fn test_22_4_1_remote_probe_optin_inert_in_default_build() {
        let _g = EMBED_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        std::env::set_var("CONTEXTFORGE_EMBED_REMOTE_PROBE", "1");
        std::env::remove_var("CONTEXTFORGE_EMBED_PROVIDER");
        let degraded = fresh_checker().check_all();
        let e1 = degraded.components.iter().find(|c| c.name == "embed").unwrap();
        assert_eq!(
            e1.status,
            HealthStatus::Degraded,
            "default build must ignore the remote-probe opt-in (config-only)"
        );
        std::env::set_var("CONTEXTFORGE_EMBED_PROVIDER", "openai");
        let healthy = fresh_checker().check_all();
        let e2 = healthy.components.iter().find(|c| c.name == "embed").unwrap();
        assert_eq!(e2.status, HealthStatus::Healthy, "config-only Healthy with opt-in still inert");
        std::env::remove_var("CONTEXTFORGE_EMBED_PROVIDER");
        std::env::remove_var("CONTEXTFORGE_EMBED_REMOTE_PROBE");
    }
}
