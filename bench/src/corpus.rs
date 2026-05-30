//! Deterministic corpus + query generation for the spike harness.
//!
//! Uses an inline splitmix64 PRNG (no `rand` dep) so a given `(seed, n, dim)` yields a
//! byte-identical corpus — the precondition for the four backends being comparable.

use std::path::Path;

use serde::Deserialize;

use contextforge_core::retriever::vector::{ChunkId, VectorChunk};

use crate::measure::brute_force_topk;

/// splitmix64 — small, deterministic, dependency-free PRNG.
#[inline]
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Deterministic f32 in [-1.0, 1.0).
#[inline]
fn next_f32_unit(state: &mut u64) -> f32 {
    let u = splitmix64(state);
    let v = (u >> 40) as f32 / (1u32 << 24) as f32; // [0, 1)
    v * 2.0 - 1.0
}

/// A query vector paired with its exact nearest-neighbour chunk id (ground truth).
pub struct Query {
    pub vec: Vec<f32>,
    pub truth: ChunkId,
}

/// Generate `n` chunks of `dim`-d deterministic embeddings. Same `(seed, n, dim)` is
/// byte-identical across runs.
pub fn gen_synthetic(seed: u64, n: usize, dim: usize) -> Vec<VectorChunk> {
    let mut state = seed;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let mut emb = Vec::with_capacity(dim);
        for _ in 0..dim {
            emb.push(next_f32_unit(&mut state));
        }
        out.push(VectorChunk {
            chunk_id: ChunkId(format!("syn-{i:08}")),
            embedding: emb,
            metadata: None,
        });
    }
    out
}

/// Generate `m` queries: each is a deterministically-chosen corpus chunk perturbed by small
/// noise, with `truth` set to the exact top-1 nearest neighbour (brute force, deterministic).
pub fn gen_queries(seed: u64, corpus: &[VectorChunk], m: usize, dim: usize) -> Vec<Query> {
    let mut state = seed ^ 0xD1B5_4A32_D192_ED03;
    let mut out = Vec::with_capacity(m);
    if corpus.is_empty() {
        return out;
    }
    for _ in 0..m {
        let idx = (splitmix64(&mut state) as usize) % corpus.len();
        let base = &corpus[idx];
        let mut q = Vec::with_capacity(dim);
        for d in 0..dim {
            let noise = next_f32_unit(&mut state) * 0.01;
            let b = base.embedding.get(d).copied().unwrap_or(0.0);
            q.push(b + noise);
        }
        let truth = brute_force_topk(&q, corpus, 1)
            .into_iter()
            .next()
            .unwrap_or_else(|| base.chunk_id.clone());
        out.push(Query { vec: q, truth });
    }
    out
}

#[derive(Deserialize)]
struct DogfoodLine {
    chunk_id: String,
    embedding: Vec<f32>,
}

/// Load a dogfood corpus from JSONL: one `{"chunk_id": "...", "embedding": [...]}` per line.
pub fn load_dogfood(path: &Path) -> std::io::Result<Vec<VectorChunk>> {
    let content = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let d: DogfoodLine = serde_json::from_str(line)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        out.push(VectorChunk {
            chunk_id: ChunkId(d.chunk_id),
            embedding: d.embedding,
            metadata: None,
        });
    }
    Ok(out)
}
