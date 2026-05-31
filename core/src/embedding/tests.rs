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

// ---- task-22.1: provider-selection factory + dim negotiation ----

use crate::embedding::deterministic::DEFAULT_DIM;
use crate::embedding::factory::{negotiate_dim, select_provider};
use crate::embedding::traits::EmbeddingError;

// TEST-22.1.2 — AC2: select_provider("deterministic") and ("") return the deterministic provider
// (name "deterministic-sha256", dim DEFAULT_DIM), byte-equivalent to the Phase 19 hardcoded
// `DeterministicEmbeddingProvider::default()` (so the server.rs default-arg swap is safe).
#[test]
fn test_22_1_2_factory_deterministic_default() {
    for name in ["deterministic", ""] {
        let p = select_provider(name, 0).expect("deterministic/empty must select a provider");
        assert_eq!(p.name(), "deterministic-sha256", "name for {name:?}");
        assert_eq!(p.dim(), DEFAULT_DIM, "dim for {name:?}");
    }
    let factory = select_provider("deterministic", 0).unwrap();
    let baseline = DeterministicEmbeddingProvider::default();
    let texts = vec![
        "where is the config loader".to_string(),
        "how the daemon restarts after a crash".to_string(),
    ];
    assert_eq!(
        factory.embed(&texts).unwrap(),
        baseline.embed(&texts).unwrap(),
        "factory deterministic must embed byte-identically to default()"
    );
}

// TEST-22.1.3 — AC3: dim negotiation. Requested dim is honored for the deterministic provider;
// dim=0 uses DEFAULT_DIM; a provider dim != a non-zero requested dim returns DimMismatch (the
// factory never silently truncates/pads, which would corrupt the existing 384-dim index).
#[test]
fn test_22_1_3_dim_negotiation() {
    let p = select_provider("deterministic", 128).expect("deterministic+128");
    assert_eq!(p.dim(), 128, "a non-zero requested dim must be honored");
    let p0 = select_provider("deterministic", 0).expect("deterministic+0");
    assert_eq!(p0.dim(), DEFAULT_DIM, "dim=0 uses DEFAULT_DIM");

    assert!(negotiate_dim(384, 0).is_ok(), "dim=0 must never mismatch");
    assert!(negotiate_dim(128, 128).is_ok(), "equal dims must not mismatch");
    match negotiate_dim(384, 128) {
        Err(EmbeddingError::DimMismatch { expected, got }) => {
            assert_eq!(expected, 128, "expected = the requested dim");
            assert_eq!(got, 384, "got = the provider's actual dim");
        }
        other => panic!("expected DimMismatch, got {other:?}"),
    }
}

// TEST-22.1.4 — AC4: "remote" and unknown names return an explicit error (no panic, no silent
// fallback); "fastembed" without the embedding-fastembed feature returns a feature-not-enabled error.
#[test]
fn test_22_1_4_factory_error_paths() {
    assert!(
        matches!(select_provider("remote", 0), Err(EmbeddingError::Other(_))),
        "remote must return an explicit error until task-22.3 lands the skeleton"
    );
    assert!(
        matches!(select_provider("totally-unknown", 0), Err(EmbeddingError::Other(_))),
        "unknown provider must return an explicit error"
    );
    #[cfg(not(feature = "embedding-fastembed"))]
    match select_provider("fastembed", 0) {
        Err(EmbeddingError::Other(msg)) => assert!(
            msg.contains("embedding-fastembed"),
            "error must name the missing feature, got {msg:?}"
        ),
        other => panic!("fastembed without feature must error, got {other:?}"),
    }
}

// TEST-22.1.4 (feature build) — under embedding-fastembed: "fastembed" selects (lazy load — no
// network on select), and dim negotiation against its fixed 384 dim returns DimMismatch for a
// conflicting request. Network-free: only dim() is read (no model download).
#[cfg(feature = "embedding-fastembed")]
#[test]
fn test_22_1_4_fastembed_feature_select_and_mismatch() {
    let p = select_provider("fastembed", 0).expect("fastembed selects under feature (lazy load)");
    assert_eq!(p.name(), "fastembed-all-minilm-l6-v2");
    assert_eq!(p.dim(), 384);
    match select_provider("fastembed", 128) {
        Err(EmbeddingError::DimMismatch { expected, got }) => {
            assert_eq!(expected, 128);
            assert_eq!(got, 384);
        }
        other => panic!("fastembed+128 must DimMismatch (384 != 128), got {other:?}"),
    }
}
