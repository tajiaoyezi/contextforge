//! task-18.1: unit tests for vector trait abstraction + NoopVectorBackend.

use std::sync::Arc;

use super::*;
use super::types::{ChunkId, VectorError, VectorScore};

#[test]
fn trait_object_safety_test() {
    // Verifies `Arc<dyn VectorSearcher>` can be constructed (object safety).
    let _: Arc<dyn VectorSearcher> = Arc::new(NoopVectorBackend);
}

#[test]
fn test_noop_search_returns_empty() {
    let backend = NoopVectorBackend;
    let hits = backend.search(&[0.1, 0.2, 0.3], 10, None).unwrap();
    assert!(hits.is_empty());
}

#[test]
fn test_noop_index_batch_is_noop_ok() {
    let backend = NoopVectorBackend;
    let chunk = super::types::VectorChunk {
        chunk_id: ChunkId("test-chunk".into()),
        embedding: vec![0.1, 0.2],
        metadata: None,
    };
    let count = backend.index_batch(&[chunk]).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_noop_is_indexed_always_false() {
    let backend = NoopVectorBackend;
    assert!(!backend.is_indexed());
}

#[test]
fn test_vector_score_nan_rejected() {
    let result = VectorScore::new(f32::NAN);
    assert!(matches!(result, Err(VectorError::InvalidScore(_))));
}

#[test]
fn test_vector_score_inf_rejected() {
    let result = VectorScore::new(f32::INFINITY);
    assert!(matches!(result, Err(VectorError::InvalidScore(_))));
}

#[test]
fn test_vector_score_valid_accepted() {
    let score = VectorScore::new(0.95).unwrap();
    assert!((score.as_f32() - 0.95).abs() < f32::EPSILON);
}

#[test]
fn test_noop_backend_properties() {
    let backend = NoopVectorBackend;
    assert_eq!(backend.name(), "noop");
    assert_eq!(backend.version(), "0.1.0");
    assert!(backend.is_local());
    assert!(!backend.requires_embedding());
}
