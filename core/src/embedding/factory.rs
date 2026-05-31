//! task-22.1: embedding provider selection factory (Phase 22 embedding-provider-completion).
//!
//! `select_provider` maps a config provider name + requested dim to a concrete
//! `Arc<dyn EmbeddingProvider>`, centralizing the choice the server.rs semantic path used to
//! hardcode. The default (`""` / `"deterministic"`, dim 0) is byte-equivalent to Phase 19's
//! `DeterministicEmbeddingProvider::default()` — the swap is behavior-preserving (ADR-027 D1).
//! `fastembed` / `remote` stay feature-gated (ADR-004 local-first: the default build pulls 0 new
//! dependency). Dim negotiation never silently truncates/pads — a conflicting dim surfaces as
//! `DimMismatch`, so a misconfiguration can't corrupt the existing 384-dim vector index.

use std::sync::Arc;

use crate::embedding::deterministic::{DeterministicEmbeddingProvider, DEFAULT_DIM};
use crate::embedding::traits::{EmbeddingError, EmbeddingProvider};

/// Select an embedding provider by config name + requested output dim.
///
/// - `""` / `"deterministic"` → `DeterministicEmbeddingProvider` (dim = `dim`, or `DEFAULT_DIM`
///   when `dim == 0`).
/// - `"fastembed"` → `FastEmbedProvider` behind the `embedding-fastembed` feature; an explicit
///   feature-not-enabled error otherwise (no panic, no silent fallback).
/// - `"remote"` → an explicit "not yet wired" error; the skeleton lands in task-22.3.
/// - any other name → an explicit unknown-provider error.
///
/// After selection the provider's `dim()` is reconciled with a non-zero requested `dim`
/// (`DimMismatch` on conflict — never a silent resize).
pub fn select_provider(
    provider_name: &str,
    dim: usize,
) -> Result<Arc<dyn EmbeddingProvider>, EmbeddingError> {
    let provider: Arc<dyn EmbeddingProvider> = match provider_name {
        "" | "deterministic" => Arc::new(DeterministicEmbeddingProvider::new(if dim == 0 {
            DEFAULT_DIM
        } else {
            dim
        })),
        "fastembed" => {
            #[cfg(feature = "embedding-fastembed")]
            {
                Arc::new(crate::embedding::fastembed_provider::FastEmbedProvider::new())
            }
            #[cfg(not(feature = "embedding-fastembed"))]
            {
                return Err(EmbeddingError::Other(
                    "embedding provider 'fastembed' requires the embedding-fastembed feature".into(),
                ));
            }
        }
        "remote" => {
            return Err(EmbeddingError::Other(
                "embedding provider 'remote' is not yet wired (skeleton lands in task-22.3)".into(),
            ));
        }
        other => {
            return Err(EmbeddingError::Other(format!(
                "unknown embedding provider {other:?}"
            )));
        }
    };
    negotiate_dim(provider.dim(), dim)?;
    Ok(provider)
}

/// Reconcile a provider's actual dim with a requested one. `requested == 0` means "use the
/// provider default" (never mismatches). A non-zero `requested` that differs from `provider_dim`
/// is a hard `DimMismatch` — the factory never silently truncates or pads.
pub(crate) fn negotiate_dim(provider_dim: usize, requested: usize) -> Result<(), EmbeddingError> {
    if requested != 0 && provider_dim != requested {
        return Err(EmbeddingError::DimMismatch {
            expected: requested,
            got: provider_dim,
        });
    }
    Ok(())
}
