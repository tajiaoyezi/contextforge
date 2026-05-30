//! task-19.1: embedding provider unit tests (deterministic provider; default build, no model).

use std::sync::Arc;

use crate::embedding::deterministic::DeterministicEmbeddingProvider;
use crate::embedding::traits::EmbeddingProvider;

// TEST-19.1.2 — AC2: deterministic — same text → byte-identical vector; different text differs.
#[test]
fn test_deterministic_same_text_identical() {
    let p = DeterministicEmbeddingProvider::new(384);
    let t = vec!["where is the config loader".to_string()];
    let a = p.embed(&t).unwrap();
    let b = p.embed(&t).unwrap();
    assert_eq!(a, b, "same text must embed byte-identically");
    let c = p
        .embed(&["how does the daemon restart".to_string()])
        .unwrap();
    assert_ne!(a[0], c[0], "different text should embed differently");
}

// TEST-19.1.3 — AC3: dim consistency across a batch + unit norm.
#[test]
fn test_dim_consistency_and_unit_norm() {
    let p = DeterministicEmbeddingProvider::new(128);
    let texts: Vec<String> = ["a", "bb", "ccc", ""].iter().map(|s| s.to_string()).collect();
    let out = p.embed(&texts).unwrap();
    assert_eq!(out.len(), texts.len());
    for v in &out {
        assert_eq!(v.len(), p.dim(), "every vector has dim()");
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-3, "expected unit norm, got {norm}");
    }
}

// TEST-19.1.1 — AC1: trait object safety + stable name/dim + empty input.
#[test]
fn test_trait_object_and_empty_input() {
    let p: Arc<dyn EmbeddingProvider> = Arc::new(DeterministicEmbeddingProvider::default());
    assert_eq!(p.dim(), 384);
    assert_eq!(p.name(), "deterministic-sha256");
    let empty = p.embed(&[]).unwrap();
    assert!(empty.is_empty(), "empty input → empty output");
}

// TEST-19.1.4 — AC4: real fastembed provider produces dim-384 embeddings. `#[ignore]` because it
// downloads the all-MiniLM-L6-v2 model on first run; exercise manually on a Linux dev box with:
//   cargo test -p contextforge-core --features embedding-fastembed -- --ignored real_embed
#[cfg(feature = "embedding-fastembed")]
#[test]
#[ignore]
fn test_real_fastembed_embed_dim384() {
    use crate::embedding::fastembed_provider::FastEmbedProvider;
    let p = FastEmbedProvider::new();
    assert_eq!(p.dim(), 384);
    assert_eq!(p.name(), "fastembed-all-minilm-l6-v2");
    let out = p
        .embed(&["where is the config loader".to_string(), "how does the daemon restart".to_string()])
        .expect("real embed should succeed on a networked Linux dev box");
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].len(), 384);
    assert!(out[0].iter().any(|x| *x != 0.0), "embedding should be non-zero");
}
