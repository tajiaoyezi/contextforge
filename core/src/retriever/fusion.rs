//! task-21.1: hybrid scoring — Reciprocal Rank Fusion (RRF) of the BM25 and vector result lists.
//!
//! RRF is rank-based (not score-based), so it needs no per-path score normalization and is
//! deterministic: `score(chunk) = Σ_path 1/(k + rank_path)` over the paths that returned the chunk
//! (rank 1-based). A chunk hit by both paths accumulates both contributions, so agreement ranks it
//! higher. ADR-025 selects RRF (over min-max weighted fusion) for its parameter-light determinism;
//! the constant `k` damps the influence of low ranks. Real recall comparison driving the ADR-025
//! ratify is task-21.3 (ADR-013: this module's tests assert fusion *correctness*, not recall quality).

use super::SearchResult;
use std::collections::HashMap;

/// RRF damping constant (standard default 60). Larger k flattens rank influence.
pub const RRF_K: f64 = 60.0;

/// Fuse the BM25 and vector result lists into a single hybrid ranking via RRF.
///
/// Output `SearchResult`s carry the fused RRF score in `score` and `retrieval_method = "hybrid"`.
/// Ranking is deterministic: fused score descending, ties broken by `chunk_id` ascending. A chunk
/// present in both inputs keeps the BM25-side `SearchResult` as its representative (BM25 is iterated
/// first); a chunk only in the vector list keeps the vector-side representative.
pub fn fuse(bm25: &[SearchResult], vector: &[SearchResult], top_k: usize) -> Vec<SearchResult> {
    // chunk_id -> (accumulated RRF score, representative result)
    let mut acc: HashMap<String, (f64, SearchResult)> = HashMap::new();

    for (rank, r) in bm25.iter().enumerate() {
        let contrib = 1.0 / (RRF_K + (rank + 1) as f64);
        acc.entry(r.chunk_id.clone())
            .or_insert_with(|| (0.0, r.clone()))
            .0 += contrib;
    }
    for (rank, r) in vector.iter().enumerate() {
        let contrib = 1.0 / (RRF_K + (rank + 1) as f64);
        match acc.get_mut(&r.chunk_id) {
            Some(e) => e.0 += contrib,
            None => {
                acc.insert(r.chunk_id.clone(), (contrib, r.clone()));
            }
        }
    }

    let mut fused: Vec<SearchResult> = acc
        .into_values()
        .map(|(score, mut r)| {
            r.score = score as f32;
            r.retrieval_method = "hybrid".to_string();
            r
        })
        .collect();
    // Deterministic order: fused score desc, then chunk_id asc as a stable tie-break.
    fused.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.chunk_id.cmp(&b.chunk_id))
    });
    fused.truncate(top_k);
    fused
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sr(chunk_id: &str, method: &str) -> SearchResult {
        SearchResult {
            chunk_id: chunk_id.into(),
            context_id: String::new(),
            source_type: String::new(),
            file_path: format!("{chunk_id}.md"),
            line_start: 0,
            line_end: 0,
            score: 0.0,
            retrieval_method: method.into(),
            reason: String::new(),
            agent_scope: vec![],
            redaction_status: "applied".into(),
            provenance: vec![],
            language: String::new(),
            content: String::new(),
            matched_terms: vec![],
        }
    }

    // TEST-21.1.1 — fixed BM25/vector rank lists → deterministic RRF order; a chunk in BOTH paths
    // accumulates both contributions and outranks single-path chunks.
    #[test]
    fn test_21_1_rrf_fuse_deterministic_order_and_dual_path_boost() {
        // BM25 ranks: A(1), B(2)  |  vector ranks: B(1), C(2)
        let bm25 = vec![sr("A", "bm25"), sr("B", "bm25")];
        let vector = vec![sr("B", "vector"), sr("C", "vector")];

        // RRF (k=60): A = 1/61; B = 1/62 (bm25) + 1/61 (vector); C = 1/62.
        // B is the only dual-path hit → highest. A (1/61) > C (1/62). Expected order: B, A, C.
        let fused = fuse(&bm25, &vector, 10);
        let order: Vec<&str> = fused.iter().map(|r| r.chunk_id.as_str()).collect();
        assert_eq!(order, vec!["B", "A", "C"], "RRF order: dual-path B first, then A>C");
        assert!(fused.iter().all(|r| r.retrieval_method == "hybrid"));

        // B's fused score == both contributions; strictly greater than A's single contribution.
        let b = &fused[0];
        let expected_b = (1.0 / 62.0 + 1.0 / 61.0) as f32;
        assert!((b.score - expected_b).abs() < 1e-6, "B fused score = 1/62 + 1/61");
        assert!(fused[0].score > fused[1].score && fused[1].score > fused[2].score);
    }

    // top_k truncates after the deterministic sort.
    #[test]
    fn test_21_1_rrf_respects_top_k() {
        let bm25 = vec![sr("A", "bm25"), sr("B", "bm25"), sr("C", "bm25")];
        let fused = fuse(&bm25, &[], 2);
        assert_eq!(fused.len(), 2);
        assert_eq!(fused[0].chunk_id, "A"); // rank 1 > rank 2 > rank 3
        assert_eq!(fused[1].chunk_id, "B");
    }

    // Empty inputs → empty output (no panic); single-path degrades to that path's ranking.
    #[test]
    fn test_21_1_rrf_single_path_and_empty() {
        assert!(fuse(&[], &[], 5).is_empty());
        let only_vec = vec![sr("X", "vector"), sr("Y", "vector")];
        let fused = fuse(&[], &only_vec, 5);
        assert_eq!(fused.iter().map(|r| r.chunk_id.clone()).collect::<Vec<_>>(), vec!["X", "Y"]);
        assert!(fused.iter().all(|r| r.retrieval_method == "hybrid"));
    }
}
