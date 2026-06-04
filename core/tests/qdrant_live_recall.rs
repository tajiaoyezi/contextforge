// task-36.1: qdrant live vector recall harness.
//
// Measures the recall@k of a LIVE qdrant server's KNN search against the exact
// top-k computed by the in-process BruteForceVectorBackend (ground truth), over
// a deterministic, reproducible embedded corpus. This closes ADR-034 D2's
// long-standing honest-defer [SPEC-DEFER:phase-future.qdrant-server-lifecycle]:
// "real live-server KNN recall numbers were never measured (CI had no server)".
//
// env-gated: runs ONLY when QDRANT_URL is set (the qdrant-recall CI job sets it
// to a service container; locally point it at a `docker run qdrant/qdrant`). When
// QDRANT_URL is unset OR the server is unreachable, the test SKIPS cleanly (never
// fails) — honest-defer, ADR-013 (no fabricated pass, no fabricated numbers).
//
// Only compiled under `--features vector-qdrant`; the default `cargo test
// --workspace` build (0-vector-dep) never sees this file.
#![cfg(feature = "vector-qdrant")]

use contextforge_core::retriever::vector::brute_force::BruteForceVectorBackend;
use contextforge_core::retriever::vector::qdrant::{QdrantBackend, QdrantConnConfig, QdrantHealth};
use contextforge_core::retriever::vector::traits::{VectorIndexer, VectorSearcher};
use contextforge_core::retriever::vector::types::{
    ChunkId, VectorChunk, VectorIndexConfig, VectorMetric,
};
use std::collections::HashSet;

const DIM: usize = 64;
const CORPUS_N: usize = 2000;
const QUERY_M: usize = 50;
const K: usize = 10;
const RECALL_FLOOR: f32 = 0.90;
const QUERY_SEED_BASE: u64 = 1_000_000;

/// splitmix64 — deterministic, reproducible PRNG (NO `rand` crate, NO clock), so
/// the corpus/queries are byte-identical across runs (ADR-013 reproducibility).
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Deterministic unit vector of length `dim` seeded by `seed` (same seed => same
/// vector). Components are mapped to (-1, 1) then L2-normalised to unit length so
/// cosine == dot product.
fn det_unit_vec(seed: u64, dim: usize) -> Vec<f32> {
    let mut state = seed ^ 0xD1B5_4A32_D192_ED03;
    let mut v: Vec<f32> = (0..dim)
        .map(|_| {
            let r = splitmix64(&mut state);
            // u64 -> (-1.0, 1.0)
            (r as f64 / u64::MAX as f64) as f32 * 2.0 - 1.0
        })
        .collect();
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
    v
}

fn corpus() -> Vec<VectorChunk> {
    (0..CORPUS_N)
        .map(|i| VectorChunk {
            chunk_id: ChunkId(format!("c{i}")),
            embedding: det_unit_vec(i as u64, DIM),
            metadata: None,
        })
        .collect()
}

fn top_k_ids(hits: &[contextforge_core::retriever::vector::types::VectorHit]) -> HashSet<String> {
    hits.iter().map(|h| h.chunk_id.0.clone()).collect()
}

// TEST-36.1.2 — deterministic corpus generator reproducibility. Runs WITHOUT a
// server (the reproducible foundation the recall harness rests on, ADR-013).
#[test]
fn test_36_1_2_deterministic_corpus_reproducible() {
    let a = det_unit_vec(42, DIM);
    let b = det_unit_vec(42, DIM);
    assert_eq!(a, b, "same seed must yield byte-identical vector");
    let c = det_unit_vec(43, DIM);
    assert_ne!(a, c, "different seed must yield a different vector");
    let norm: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 1e-4, "vector must be unit length, got {norm}");
    assert_eq!(a.len(), DIM);
}

// TEST-36.1.1 — qdrant LIVE recall@k vs BruteForce exact KNN. env-gated on
// QDRANT_URL; honest-defer skip when unset/unreachable (never fail).
#[test]
fn test_36_1_1_qdrant_live_recall_at_k() {
    if std::env::var("QDRANT_URL").is_err() {
        eprintln!(
            "SKIP test_36_1_1_qdrant_live_recall_at_k: QDRANT_URL unset (honest-defer; \
             set QDRANT_URL=http://localhost:6334 with a live qdrant to run)"
        );
        return;
    }
    let conn = QdrantConnConfig::from_env();
    let qdrant = match QdrantBackend::connect(&conn) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("SKIP test_36_1_1: qdrant connect failed (honest-defer): {e}");
            return;
        }
    };
    if qdrant.health() != QdrantHealth::Ready {
        eprintln!(
            "SKIP test_36_1_1: qdrant health != Ready at {} (honest-defer; no live server)",
            conn.url
        );
        return;
    }

    let chunks = corpus();
    let cfg = VectorIndexConfig {
        dim: DIM,
        metric: VectorMetric::Cosine,
        persistence_path: None,
        collection_id: "phase36_live_recall".to_string(),
    };

    // Index the SAME corpus into qdrant (live) and brute-force (exact ground truth).
    qdrant.open(cfg.clone()).expect("qdrant open/ensure-create");
    let n_q = qdrant.index_batch(&chunks).expect("qdrant index_batch");
    assert_eq!(n_q, CORPUS_N, "qdrant must index the whole corpus");

    let brute = BruteForceVectorBackend::new();
    brute.open(cfg).expect("brute open");
    let n_b = brute.index_batch(&chunks).expect("brute index_batch");
    assert_eq!(n_b, CORPUS_N);

    // recall@k = mean over M queries of |qdrant_topk ∩ exact_topk| / k.
    let mut recall_sum = 0.0f32;
    for j in 0..QUERY_M {
        let q = det_unit_vec(QUERY_SEED_BASE + j as u64, DIM);
        let q_hits = qdrant.search(&q, K, None).expect("qdrant search");
        let b_hits = brute.search(&q, K, None).expect("brute search");
        assert_eq!(b_hits.len(), K, "brute ground truth must return k hits");
        let inter = top_k_ids(&q_hits)
            .intersection(&top_k_ids(&b_hits))
            .count();
        recall_sum += inter as f32 / K as f32;
    }
    let recall = recall_sum / QUERY_M as f32;

    eprintln!(
        "PHASE36 qdrant LIVE recall@{K} vs brute-force exact KNN | N={CORPUS_N} dim={DIM} M={QUERY_M} @ {} => recall@{K}={recall:.4}",
        conn.url
    );
    assert!(
        recall >= RECALL_FLOOR,
        "qdrant live recall@{K}={recall:.4} below floor {RECALL_FLOOR} (N={CORPUS_N} dim={DIM} M={QUERY_M})"
    );
}
