//! task-21.2: real cross-encoder reranker via fastembed-rs `TextRerank` (BGE-reranker-base).
//!
//! Gated behind the `reranker-fastembed` feature — the default build does not compile this module
//! or pull fastembed/ort (0 new crate; it reuses the already-present optional `fastembed` dep). A
//! cross-encoder scores the (query, doc) *pair* jointly — more precise than the dual-encoder cosine
//! `search_semantic` uses — so it reranks the initial top-k to lift top-1 / MRR. The model is
//! lazy-loaded on the first `rerank` call (BGE reranker downloaded on demand). `TextRerank::rerank`
//! needs `&mut self`, so the model lives behind a `Mutex` (the `Reranker` trait takes `&self`).
//!
//! ADR-013: real rerank quality numbers come ONLY from a real model run (task-21.3 dogfood eval),
//! recorded honestly in `docs/spikes/phase-21-reranker.md`; receiving-platform blockers are deferred
//! as `[SPEC-DEFER:phase-future.reranker-real-quality]`, never faked. This module provides the
//! pipeline; the deterministic `IdentityReranker` is the CI-verifiable default.

use std::sync::Mutex;

use fastembed::{RerankInitOptions, RerankerModel, TextRerank};

use crate::rerank::traits::{RerankError, Reranker};
use crate::retriever::SearchResult;

/// Provenance marker written to a reranked result's `reason` (mirrors the identity marker).
pub const CROSS_ENCODER_RERANK_REASON: &str = "reranked:cross-encoder";

/// Real cross-encoder reranker backed by fastembed-rs + ONNX BGE-reranker-base.
pub struct CrossEncoderReranker {
    model: Mutex<Option<TextRerank>>,
}

impl CrossEncoderReranker {
    pub fn new() -> Self {
        Self {
            model: Mutex::new(None),
        }
    }
}

impl Default for CrossEncoderReranker {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CrossEncoderReranker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CrossEncoderReranker")
    }
}

impl Reranker for CrossEncoderReranker {
    fn rerank(
        &self,
        query: &str,
        candidates: &[SearchResult],
    ) -> Result<Vec<SearchResult>, RerankError> {
        if candidates.is_empty() {
            return Ok(vec![]);
        }
        let mut guard = self
            .model
            .lock()
            .map_err(|e| RerankError::Other(format!("rerank model lock poisoned: {e}")))?;
        if guard.is_none() {
            let m = TextRerank::try_new(RerankInitOptions::new(RerankerModel::BGERerankerBase))
                .map_err(|e| RerankError::ModelLoad(e.to_string()))?;
            *guard = Some(m);
        }
        let model = guard.as_mut().expect("model set above");

        let docs: Vec<&str> = candidates.iter().map(|c| c.content.as_str()).collect();
        // return_documents=false: we already hold the candidates; map back by `index`.
        let ranked = model
            .rerank(query, docs, false, None)
            .map_err(|e| RerankError::Other(e.to_string()))?;

        let mut out = Vec::with_capacity(ranked.len());
        for rr in ranked {
            // `rr.index` is the original position in `candidates` (results are sorted desc by score).
            let src = candidates.get(rr.index).ok_or_else(|| {
                RerankError::Other(format!("rerank index {} out of range", rr.index))
            })?;
            let mut c = src.clone();
            c.score = rr.score;
            c.reason = if c.reason.is_empty() {
                format!("{CROSS_ENCODER_RERANK_REASON}:{}", self.name())
            } else {
                format!("{CROSS_ENCODER_RERANK_REASON}:{}; {}", self.name(), c.reason)
            };
            out.push(c);
        }
        Ok(out)
    }

    fn name(&self) -> &'static str {
        "fastembed-bge-reranker-base"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sr(chunk_id: &str, content: &str, score: f32) -> SearchResult {
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
            content: content.into(),
            matched_terms: vec![],
        }
    }

    // TEST-21.2.3 / AC3 — real cross-encoder rerank. Downloads the BGE model, so it runs ONLY under
    // `--features reranker-fastembed` (never in the default CI build, per AC4). Asserts the pipeline:
    // re-orders candidates by cross-encoder relevance (panda-eat doc outranks an unrelated doc for a
    // panda query), drops no candidate, output sorted by score desc, provenance annotated.
    // ADR-013: this exercises the real model on a local run; quality numbers are recorded in
    // docs/spikes/phase-21-reranker.md (real-run / deterministic / blocked, honestly labelled).
    #[test]
    fn test_21_2_3_cross_encoder_reranks_by_relevance() {
        let rr = CrossEncoderReranker::new();
        let cands = vec![
            sr("a", "the giant panda is a bear species endemic to china", 0.10),
            sr("b", "rust is a systems programming language", 0.20),
            sr("c", "pandas eat bamboo shoots and leaves in the wild", 0.30),
        ];
        let out = rr.rerank("what does a panda eat?", &cands).expect("cross-encoder rerank");

        assert_eq!(out.len(), cands.len(), "no candidate dropped");
        // The bamboo-eating doc (c) should outrank the unrelated rust doc (b) for this query.
        let pos = |id: &str| out.iter().position(|r| r.chunk_id == id).unwrap();
        assert!(pos("c") < pos("b"), "panda-eat doc outranks unrelated doc");
        for w in out.windows(2) {
            assert!(w[0].score >= w[1].score, "reranked order is score desc");
        }
        assert!(
            out.iter().all(|r| r.reason.contains(CROSS_ENCODER_RERANK_REASON)),
            "cross-encoder provenance annotated"
        );
    }
}
