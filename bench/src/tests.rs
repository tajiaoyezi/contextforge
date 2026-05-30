//! task-18.2 unit tests (TEST-18.2.2 .. TEST-18.2.7).

use std::time::Duration;

use contextforge_core::retriever::vector::{ChunkId, VectorChunk};

use crate::backends::run_named;
use crate::corpus::{gen_queries, gen_synthetic};
use crate::measure::{brute_force_topk, percentile_ms, recall_rate};
use crate::runner::render_evidence_md;

// TEST-18.2.2
#[test]
fn test_corpus_deterministic() {
    let a = gen_synthetic(42, 100, 16);
    let b = gen_synthetic(42, 100, 16);
    assert_eq!(a.len(), 100);
    for (x, y) in a.iter().zip(b.iter()) {
        assert_eq!(x.chunk_id, y.chunk_id);
        assert_eq!(x.embedding, y.embedding);
    }
    let qa = gen_queries(7, &a, 20, 16);
    let qb = gen_queries(7, &b, 20, 16);
    assert_eq!(qa.len(), 20);
    for (x, y) in qa.iter().zip(qb.iter()) {
        assert_eq!(x.vec, y.vec);
        assert_eq!(x.truth, y.truth);
    }
    // different seed → different embeddings
    let c = gen_synthetic(43, 100, 16);
    assert_ne!(a[0].embedding, c[0].embedding);
}

// TEST-18.2.3
#[test]
fn test_recall_rate_math() {
    let t = vec![ChunkId("x".into()), ChunkId("y".into())];
    let full = vec![vec![ChunkId("x".into())], vec![ChunkId("y".into())]];
    assert_eq!(recall_rate(&full, &t, 5), 1.0);
    let none = vec![vec![ChunkId("a".into())], vec![ChunkId("b".into())]];
    assert_eq!(recall_rate(&none, &t, 5), 0.0);
    let half = vec![vec![ChunkId("x".into())], vec![ChunkId("b".into())]];
    assert_eq!(recall_rate(&half, &t, 5), 0.5);
    // empty backend results (Noop) → recall 0
    let empty = vec![vec![], vec![]];
    assert_eq!(recall_rate(&empty, &t, 5), 0.0);
}

// TEST-18.2.4
#[test]
fn test_p95_percentile() {
    let mut d: Vec<Duration> = (1..=100).map(Duration::from_millis).collect();
    let p95 = percentile_ms(&mut d, 0.95);
    assert!((p95 - 95.0).abs() < 1.0, "P95 should be ~95ms, got {p95}");
    let mut empty: Vec<Duration> = vec![];
    assert_eq!(percentile_ms(&mut empty, 0.95), 0.0);
}

// TEST-18.2.5
#[test]
fn test_brute_force_topk() {
    let corpus = vec![
        VectorChunk { chunk_id: ChunkId("near".into()), embedding: vec![1.0, 0.0, 0.0], metadata: None },
        VectorChunk { chunk_id: ChunkId("mid".into()), embedding: vec![0.5, 0.5, 0.0], metadata: None },
        VectorChunk { chunk_id: ChunkId("far".into()), embedding: vec![0.0, 0.0, 1.0], metadata: None },
    ];
    let top = brute_force_topk(&[1.0, 0.0, 0.0], &corpus, 2);
    assert_eq!(top.len(), 2);
    assert_eq!(top[0], ChunkId("near".into()));
}

// TEST-18.2.6
#[test]
fn test_runner_noop_end_to_end() {
    let corpus = gen_synthetic(1, 200, 16);
    let queries = gen_queries(1, &corpus, 30, 16);
    let report = run_named("noop", &corpus, &queries, 16)
        .expect("run ok")
        .expect("noop is a known backend");
    assert_eq!(report.backend_name, "noop");
    assert_eq!(report.n, 200);
    assert_eq!(report.recall_at_5, 0.0, "Noop returns empty hits → recall 0");
    assert_eq!(report.recall_at_10, 0.0);
    assert!(report.p95_latency_ms >= 0.0);
    // unknown backend → Ok(None)
    assert!(run_named("does-not-exist", &corpus, &queries, 16)
        .expect("ok")
        .is_none());
}

// TEST-18.2.7
#[test]
fn test_render_evidence_md() {
    let corpus = gen_synthetic(1, 50, 8);
    let queries = gen_queries(1, &corpus, 10, 8);
    let report = run_named("noop", &corpus, &queries, 8).unwrap().unwrap();
    let md = render_evidence_md(&report);
    assert!(md.contains("recall@5"));
    assert!(md.contains("P95 latency"));
    assert!(md.contains("`noop`"));
}
