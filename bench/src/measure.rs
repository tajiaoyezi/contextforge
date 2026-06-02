//! Pure measurement math: cosine brute-force ground truth, recall@k, P95 percentile.

use std::time::Duration;

use contextforge_core::retriever::vector::{ChunkId, VectorChunk};

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..len {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

/// Exact top-k nearest neighbours by cosine similarity (ground truth for recall).
/// Ties broken by chunk_id for determinism.
pub fn brute_force_topk(query: &[f32], corpus: &[VectorChunk], k: usize) -> Vec<ChunkId> {
    let mut scored: Vec<(f32, &ChunkId)> = corpus
        .iter()
        .map(|c| (cosine(query, &c.embedding), &c.chunk_id))
        .collect();
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1 .0.cmp(&b.1 .0))
    });
    scored.into_iter().take(k).map(|(_, id)| id.clone()).collect()
}

/// recall@k = fraction of queries whose ground-truth top-1 appears in the backend's top-k.
pub fn recall_rate(got: &[Vec<ChunkId>], truths: &[ChunkId], k: usize) -> f64 {
    if got.is_empty() {
        return 0.0;
    }
    let mut hit = 0usize;
    for (g, t) in got.iter().zip(truths.iter()) {
        if g.iter().take(k).any(|id| id == t) {
            hit += 1;
        }
    }
    hit as f64 / got.len() as f64
}

/// p-th percentile of latencies in milliseconds (nearest-rank). `p` in [0.0, 1.0].
pub fn percentile_ms(durations: &mut [Duration], p: f64) -> f64 {
    if durations.is_empty() {
        return 0.0;
    }
    durations.sort();
    let rank = ((durations.len() as f64) * p).ceil() as usize;
    let idx = rank.saturating_sub(1).min(durations.len() - 1);
    durations[idx].as_secs_f64() * 1000.0
}
