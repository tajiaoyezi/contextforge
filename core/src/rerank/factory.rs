//! task-38.1: reranker selection factory (Phase 38 embedding-remote-reranker-live).
//!
//! `select_reranker` maps a config reranker name to a concrete `Arc<dyn Reranker>`, mirroring the
//! embedding `select_provider` factory (`embedding/factory.rs:27-83`). The default (`""` / `"none"` /
//! `"identity"`) is the model-free `IdentityReranker` — byte-equivalent to the existing default-build
//! reranker (no behavior change). `cross-encoder` / `remote` stay feature-gated (ADR-004 local-first:
//! the default build pulls 0 new dependency); a feature-not-enabled name surfaces an explicit error
//! (no panic, no silent fallback). task-38.2 wires this into the production data plane via
//! `reranker_from_env()`.

use std::sync::Arc;

use crate::rerank::identity::IdentityReranker;
use crate::rerank::traits::{RerankError, Reranker};

/// Select a reranker by config name.
///
/// - `""` / `"none"` / `"identity"` → `IdentityReranker` (model-free deterministic default).
/// - `"cross-encoder"` / `"fastembed"` → `CrossEncoderReranker` behind the `reranker-fastembed`
///   feature; an explicit feature-not-enabled error otherwise (no panic, no silent fallback).
/// - `"remote"` → `RemoteRerankerProvider` behind the `reranker-remote` feature, constructed from
///   `CONTEXTFORGE_RERANKER_ENDPOINT` / `_MODEL` / `_PROVIDER` / `_API_KEY` (api_key read here and
///   never logged — PRD security baseline / ADR-004 opt-in); an explicit error otherwise.
/// - any other name → an explicit unknown-reranker error.
pub fn select_reranker(provider_name: &str) -> Result<Arc<dyn Reranker>, RerankError> {
    let reranker: Arc<dyn Reranker> = match provider_name {
        "" | "none" | "identity" => Arc::new(IdentityReranker::new()),
        "cross-encoder" | "fastembed" => {
            #[cfg(feature = "reranker-fastembed")]
            {
                Arc::new(crate::rerank::cross_encoder::CrossEncoderReranker::new())
            }
            #[cfg(not(feature = "reranker-fastembed"))]
            {
                return Err(RerankError::Other(
                    "reranker 'cross-encoder' requires the reranker-fastembed feature".into(),
                ));
            }
        }
        "remote" => {
            #[cfg(feature = "reranker-remote")]
            {
                // endpoint / model / provider / api_key from env (config plumbing lands in task-38.2;
                // api_key is read here and never logged — PRD security baseline / ADR-004 opt-in).
                let endpoint = std::env::var("CONTEXTFORGE_RERANKER_ENDPOINT").unwrap_or_default();
                let model = std::env::var("CONTEXTFORGE_RERANKER_MODEL").unwrap_or_default();
                let provider = std::env::var("CONTEXTFORGE_RERANKER_PROVIDER")
                    .unwrap_or_else(|_| "openai".to_string());
                let api_key = std::env::var("CONTEXTFORGE_RERANKER_API_KEY").ok();
                Arc::new(crate::rerank::remote_reranker::RemoteRerankerProvider::new(
                    endpoint, model, &provider, api_key,
                ))
            }
            #[cfg(not(feature = "reranker-remote"))]
            {
                return Err(RerankError::Other(
                    "reranker 'remote' requires the reranker-remote feature".into(),
                ));
            }
        }
        other => {
            return Err(RerankError::Other(format!("unknown reranker {other:?}")));
        }
    };
    Ok(reranker)
}

#[cfg(test)]
mod tests {
    use super::*;

    // TEST-38.1 routing (no network): default names → IdentityReranker; unknown → explicit Err.
    #[test]
    fn select_reranker_routes_default_and_unknown() {
        for name in ["", "none", "identity"] {
            let rr = select_reranker(name).expect("default reranker");
            assert_eq!(rr.name(), "identity-rerank", "{name:?} → identity");
        }
        assert!(select_reranker("does-not-exist").is_err(), "unknown reranker → Err");
    }

    // Feature-gated names error explicitly (no panic / no silent fallback) when the feature is off.
    #[cfg(not(feature = "reranker-fastembed"))]
    #[test]
    fn select_reranker_cross_encoder_requires_feature() {
        assert!(
            select_reranker("cross-encoder").is_err(),
            "cross-encoder without reranker-fastembed → Err"
        );
    }

    #[cfg(not(feature = "reranker-remote"))]
    #[test]
    fn select_reranker_remote_requires_feature() {
        assert!(
            select_reranker("remote").is_err(),
            "remote without reranker-remote → Err"
        );
    }

    // Under the feature, "remote" constructs a provider (reads env; no network call on construction).
    #[cfg(feature = "reranker-remote")]
    #[test]
    fn select_reranker_remote_under_feature() {
        let rr = select_reranker("remote").expect("remote selects under feature");
        assert!(
            rr.name().starts_with("remote-"),
            "factory remote reranker carries remote-* provenance, got {:?}",
            rr.name()
        );
    }
}
