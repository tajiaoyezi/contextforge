// task-38.2: data-plane reranker opt-in wiring via CONTEXTFORGE_RERANKER_PROVIDER.
//
// reranker_from_env() is what server.rs (hybrid + semantic) and data_plane/search.rs (semantic) call
// to opt-in `with_reranker` on their wired retriever. This integration test runs in its OWN process
// (separate from the lib unittest binary), so setting the process-global CONTEXTFORGE_RERANKER_PROVIDER
// here cannot contaminate the server.rs / data_plane search-handler unit tests (which assert the
// backward-compatible no-rerank path when the var is unset).
//
// Coverage:
//   - unset / "" / "none" (any case) -> Ok(None): the default no-rerank path (byte-equivalent, ADR-004).
//   - "identity" -> Ok(Some(IdentityReranker)), and that reranker annotates IDENTITY_RERANK_REASON on
//     the candidates it reorders — i.e. wiring it via with_reranker makes search results carry the
//     rerank provenance marker (the search_semantic seam itself is proven by retriever/mod.rs
//     test_21_2_2_with_reranker_seam_applies_and_none_unchanged).
//   - unknown provider -> Err: no silent fallback (ADR-013).
//
// All env mutation lives in one #[test] fn (sequential) with save/restore, so no intra-file race.

use contextforge_core::rerank::{reranker_from_env, IDENTITY_RERANK_REASON};
use contextforge_core::retriever::SearchResult;

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

#[test]
fn reranker_from_env_routes_and_applies_marker() {
    const KEY: &str = "CONTEXTFORGE_RERANKER_PROVIDER";
    let saved = std::env::var(KEY).ok();

    // unset → None (default no-rerank, backward compatible).
    std::env::remove_var(KEY);
    assert!(
        reranker_from_env().expect("ok").is_none(),
        "unset CONTEXTFORGE_RERANKER_PROVIDER → None (no rerank, ADR-004)"
    );

    // empty / "none" (any case, surrounding whitespace) → None.
    for v in ["", "none", "NONE", "  none  "] {
        std::env::set_var(KEY, v);
        assert!(
            reranker_from_env().expect("ok").is_none(),
            "CONTEXTFORGE_RERANKER_PROVIDER={v:?} → None"
        );
    }

    // "identity" → Some(IdentityReranker), and it annotates the rerank provenance marker.
    std::env::set_var(KEY, "identity");
    let rr = reranker_from_env()
        .expect("identity routes ok")
        .expect("identity → Some");
    assert_eq!(rr.name(), "identity-rerank");
    let cands = vec![sr("a", 0.10), sr("b", 0.90)];
    let out = rr.rerank("any query", &cands).expect("rerank");
    assert_eq!(out.len(), cands.len(), "rerank drops no candidate");
    assert!(
        out.iter().all(|r| r.reason.contains(IDENTITY_RERANK_REASON)),
        "env=identity → wired reranker annotates the rerank provenance marker (data-plane opt-in)"
    );

    // unknown provider → explicit Err (no silent fallback, ADR-013).
    std::env::set_var(KEY, "bogus-provider");
    assert!(
        reranker_from_env().is_err(),
        "unknown CONTEXTFORGE_RERANKER_PROVIDER → Err (no silent fallback)"
    );

    // restore the original environment.
    match saved {
        Some(v) => std::env::set_var(KEY, v),
        None => std::env::remove_var(KEY),
    }
}
