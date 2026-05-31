//! task-21.2: deterministic, model-free reranker (the default-build `Reranker`).
//!
//! `IdentityReranker` re-orders candidates by their existing relevance score (descending), with a
//! `chunk_id` ascending tie-break for stability. It introduces no model and changes no candidate
//! content — it only re-orders and annotates `reason` with its provenance (ADR-026 D2). Like the
//! 0-dep `BruteForceVectorBackend` (ADR-023), it is a *real, runnable* deterministic implementation
//! (not a placeholder): it lets CI/tests verify the rerank-pipeline wiring with no model dependency.
//! NOTE: it asserts pipeline correctness + order determinism, NOT real rerank quality — that needs a
//! real cross-encoder (`CrossEncoderReranker`, task-21.3 dogfood eval; ADR-013).

use crate::rerank::traits::{RerankError, Reranker};
use crate::retriever::SearchResult;

/// Provenance marker written to a reranked result's `reason` (ADR-026 D2 — annotate rerank source).
pub const IDENTITY_RERANK_REASON: &str = "reranked:identity";

/// Deterministic, model-free reranker: score desc, `chunk_id` asc tie-break.
#[derive(Debug, Clone, Default)]
pub struct IdentityReranker;

impl IdentityReranker {
    pub fn new() -> Self {
        Self
    }
}

impl Reranker for IdentityReranker {
    fn rerank(
        &self,
        _query: &str,
        candidates: &[SearchResult],
    ) -> Result<Vec<SearchResult>, RerankError> {
        let mut out: Vec<SearchResult> = candidates.to_vec();
        // Annotate provenance (ADR-026 D2) — does not change candidate content/score/identity.
        for r in out.iter_mut() {
            r.reason = if r.reason.is_empty() {
                IDENTITY_RERANK_REASON.to_string()
            } else {
                format!("{IDENTITY_RERANK_REASON}; {}", r.reason)
            };
        }
        // Deterministic order: existing relevance score desc, chunk_id asc as a stable tie-break
        // (matches the fusion.rs convention). No model, no candidate dropped.
        out.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.chunk_id.cmp(&b.chunk_id))
        });
        Ok(out)
    }

    fn name(&self) -> &'static str {
        "identity-rerank"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sr(chunk_id: &str, score: f32) -> SearchResult {
        SearchResult {
            chunk_id: chunk_id.into(),
            context_id: String::new(),
            source_type: String::new(),
            file_path: format!("{chunk_id}.md"),
            line_start: 0,
            line_end: 0,
            score,
            retrieval_method: "vector".into(),
            reason: String::new(),
            agent_scope: vec![],
            redaction_status: "applied".into(),
            provenance: vec![],
            language: String::new(),
            content: format!("content of {chunk_id}"),
            matched_terms: vec![],
        }
    }

    // TEST-21.2.1 / AC1 — IdentityReranker deterministically re-orders a fixed candidate set by
    // score desc (chunk_id asc tie-break), drops no candidate, mutates no candidate content, and
    // annotates `reason` with its provenance. Re-running yields a byte-identical order (determinism).
    #[test]
    fn test_21_2_1_identity_rerank_deterministic_order_no_drop_no_content_change() {
        let rr = IdentityReranker::new();
        // Unsorted input with a score tie (b, c both 0.5) to exercise the chunk_id tie-break.
        let input = vec![sr("b", 0.5), sr("a", 0.9), sr("c", 0.5), sr("d", 0.1)];

        let out = rr.rerank("any query", &input).expect("identity rerank");

        // Order: a(0.9) > tie{b,c}=0.5 (chunk_id asc → b before c) > d(0.1).
        let order: Vec<&str> = out.iter().map(|r| r.chunk_id.as_str()).collect();
        assert_eq!(order, vec!["a", "b", "c", "d"], "score desc, chunk_id asc tie-break");

        // No candidate dropped (same multiset of chunk_ids, same length).
        assert_eq!(out.len(), input.len(), "no candidate dropped");
        let mut in_ids: Vec<&str> = input.iter().map(|r| r.chunk_id.as_str()).collect();
        let mut out_ids: Vec<&str> = out.iter().map(|r| r.chunk_id.as_str()).collect();
        in_ids.sort();
        out_ids.sort();
        assert_eq!(in_ids, out_ids, "same candidate set (no fabrication / drop)");

        // Content not changed: per chunk_id, content + score + file_path preserved.
        for r in &out {
            let orig = input.iter().find(|o| o.chunk_id == r.chunk_id).unwrap();
            assert_eq!(r.content, orig.content, "content unchanged for {}", r.chunk_id);
            assert_eq!(r.score, orig.score, "score unchanged for {}", r.chunk_id);
            assert_eq!(r.file_path, orig.file_path, "file_path unchanged for {}", r.chunk_id);
            // reason annotated with the identity-rerank provenance marker (ADR-026 D2).
            assert!(
                r.reason.contains(IDENTITY_RERANK_REASON),
                "reason annotated with rerank provenance for {}",
                r.chunk_id
            );
        }

        // Determinism: re-running on the same input yields the identical order.
        let out2 = rr.rerank("any query", &input).expect("identity rerank #2");
        let order2: Vec<&str> = out2.iter().map(|r| r.chunk_id.as_str()).collect();
        assert_eq!(order, order2, "deterministic across runs");

        // Empty input → empty output (no panic).
        assert!(rr.rerank("q", &[]).expect("empty").is_empty());
    }
}
