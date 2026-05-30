//! task-19.1: deterministic, model-free embedding provider.
//!
//! Derives a fixed-dimension unit vector from `Sha256(text)` via a splitmix64 PRNG. Same text →
//! byte-identical vector (reproducible recall, no non-determinism in CI/smoke); different text →
//! near-certainly different vector. No model, no network, no new dependency — enabled in the
//! default build. NOTE: these vectors carry **no semantic structure** — they exist to drive the
//! wiring / smoke / tests deterministically, NOT to measure real recall (that needs a real model,
//! task-19.5). The unit-normalization matches the vector backends' cosine convention.

use sha2::{Digest, Sha256};

use crate::embedding::traits::{EmbeddingError, EmbeddingProvider};

/// Default dimension — matches the fastembed `all-MiniLM-L6-v2` real provider (384), so the
/// retriever can swap deterministic ↔ real without a dimension change.
pub const DEFAULT_DIM: usize = 384;

/// Model-free deterministic embedding provider (Sha256-seeded splitmix64 → unit vector).
#[derive(Debug, Clone)]
pub struct DeterministicEmbeddingProvider {
    dim: usize,
}

impl DeterministicEmbeddingProvider {
    pub fn new(dim: usize) -> Self {
        Self { dim: dim.max(1) }
    }
}

impl Default for DeterministicEmbeddingProvider {
    fn default() -> Self {
        Self::new(DEFAULT_DIM)
    }
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn embed_one(text: &str, dim: usize) -> Vec<f32> {
    let digest = Sha256::digest(text.as_bytes());
    let mut seed = u64::from_le_bytes(digest[0..8].try_into().unwrap());
    // seed 0 would make splitmix64 still progress, but avoid the degenerate all-from-0 case.
    if seed == 0 {
        seed = 0xDEAD_BEEF_CAFE_F00D;
    }
    let mut v: Vec<f32> = Vec::with_capacity(dim);
    for _ in 0..dim {
        let r = splitmix64(&mut seed);
        // map u64 → f32 in [-1, 1]
        let unit = (r >> 11) as f32 / (1u64 << 53) as f32; // [0, 1)
        v.push(unit * 2.0 - 1.0);
    }
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
    v
}

impl EmbeddingProvider for DeterministicEmbeddingProvider {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        Ok(texts.iter().map(|t| embed_one(t, self.dim)).collect())
    }

    fn dim(&self) -> usize {
        self.dim
    }

    fn name(&self) -> &'static str {
        "deterministic-sha256"
    }
}
