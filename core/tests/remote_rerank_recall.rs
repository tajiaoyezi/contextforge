// remote cross-encoder rerank live quality harness.
//
// Measures whether a REAL remote cross-encoder reranker (e.g. Qwen/Qwen3-VL-Reranker-8B via
// SiliconFlow's /v1/rerank) produces semantically correct top-1 / MRR rankings on a small
// AUTHOR-LABELED query×candidate set, and compares it against the model-free `IdentityReranker`
// baseline on the SAME set. The baseline is a NO-SEMANTIC-SIGNAL control: candidates are fed with a
// uniform (0.0) relevance prior, so IdentityReranker's deterministic `score desc, chunk_id asc`
// tie-break is independent of query relevance (≈ chance). The set is constructed so the labeled doc
// is never alphabetically first among its candidates — identity therefore scores recall@1 = 0. A real
// cross-encoder must use joint query×doc meaning to lift the relevant doc to #1. The delta is what
// real remote reranking actually buys.
//
// This is the first real redemption of [SPEC-DEFER:phase-future.embedding-remote-reranker-live]
// (real network 联调 + real rerank quality numbers; in-repo only ever had the model-free
// IdentityReranker + a feature-gated local CrossEncoderReranker, never a measured remote rerank, and
// the reranker was never wired into the production data plane — task-38.2 closes the wiring gap).
//
// HONEST SCOPE (ADR-013): this is a small hand-labeled set proving a real cross-encoder ranks obvious
// paraphrase / cross-lingual pairs above deliberate near-distractors — it is NOT a large standardized
// rerank benchmark. The MRR floor is a regression guard, not a quality ceiling. Large-corpus /
// standard-benchmark rerank quality stays [SPEC-DEFER:phase-future.reranker-large-corpus-quality].
//
// env-gated: the live test runs ONLY when CONTEXTFORGE_RERANKER_API_KEY is set (the factory also
// reads CONTEXTFORGE_RERANKER_ENDPOINT / _MODEL / _PROVIDER from env; the api key is never logged).
// Unset → SKIP cleanly (honest-defer; CI has no credentials — remote rerank is a paid external API
// with no free service container, unlike qdrant — so no fabricated pass, no fabricated numbers). Only
// compiled under `--features reranker-remote`; the default `cargo test --workspace` never sees it.
#![cfg(feature = "reranker-remote")]

use std::sync::Arc;

use contextforge_core::rerank::remote_reranker::{build_rerank_request_body, parse_rerank_response};
use contextforge_core::rerank::{select_reranker, Reranker};
use contextforge_core::retriever::SearchResult;

/// One labeled case: a query whose single relevant document is `relevant`, drawn from `candidates`
/// (which always includes at least one near-distractor that shares vocabulary with the query).
struct Case {
    query: &'static str,
    relevant: &'static str,
    candidates: Vec<&'static str>,
}

/// Corpus: (id, text). Includes near-distractor pairs (config save vs load; bm25 vs hybrid; cosine vs
/// vector_backend; cjk index vs cjk vector; cache vs chunk) so ranking the labeled doc #1 is not
/// lexically trivial — it needs joint query×doc meaning.
fn docs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("audit", "Every memory pin and delete operation is recorded in an append-only audit log."),
        ("bm25", "Lexical ranking scores a document by term frequency and inverse document frequency of the query words."),
        ("cache", "Computed embeddings are cached by the content hash of the input so identical text is never re-embedded."),
        ("chunk", "Source files are split into overlapping chunks before each chunk is embedded into a vector."),
        ("cjk_index", "中文文本先用分词器切分为词语，再建立倒排索引以便检索。"),
        ("cjk_vector", "向量检索在嵌入空间里用余弦相似度寻找最近邻文档。"),
        ("config_load", "The configuration loader reads settings from config.toml when the daemon starts up."),
        ("config_save", "Persisting user preferences writes the modified settings back to disk on shutdown."),
        ("cosine", "The similarity between two embeddings is the cosine of the angle between the vectors."),
        ("eval", "Recall at k measures how often the relevant document appears within the top k retrieved results."),
        ("grpc", "The Go control plane communicates with the Rust core engine over a gRPC bridge."),
        ("health", "A readiness probe reports whether the data plane and its backing dependencies are reachable."),
        ("hybrid", "Fusing keyword relevance scores together with vector similarity into a single combined ranking."),
        ("reranker", "A cross-encoder re-orders the top candidate documents by jointly reading the query and each document together."),
        ("tokenizer", "A custom analyzer splits camelCase and snake_case identifiers such as RetrieverConfig into separate searchable tokens."),
        ("vector_backend", "Pick which approximate nearest-neighbor vector store to use — qdrant or lancedb — through the configuration file."),
    ]
}

/// Labeled cases. Each `relevant` is a paraphrase / cross-lingual / concept restatement of one doc;
/// `candidates` always carries the vocabulary-sharing near-distractor, and is arranged so `relevant`
/// is never alphabetically first (the no-semantic IdentityReranker baseline therefore misses @1).
fn cases() -> Vec<Case> {
    vec![
        Case { query: "persist the user's preferences back to disk when shutting down", relevant: "config_save", candidates: vec!["config_load", "config_save"] },
        Case { query: "blend lexical keyword scores with semantic vector scores into one ranked list", relevant: "hybrid", candidates: vec!["bm25", "hybrid"] },
        Case { query: "在嵌入空间里用余弦距离寻找最相近的文档", relevant: "cjk_vector", candidates: vec!["cjk_index", "cjk_vector"] },
        Case { query: "choose which database stores the vectors for nearest neighbor search", relevant: "vector_backend", candidates: vec!["cosine", "hybrid", "vector_backend"] },
        Case { query: "cut source files into overlapping pieces before each is embedded", relevant: "chunk", candidates: vec!["cache", "tokenizer", "chunk"] },
        Case { query: "a routine that breaks identifiers like getUserName apart into separate words for searching", relevant: "tokenizer", candidates: vec!["chunk", "reranker", "tokenizer"] },
        Case { query: "re-order the shortlisted results by reading the query and each document jointly", relevant: "reranker", candidates: vec!["hybrid", "reranker"] },
        Case { query: "how does the application read its settings when it first launches", relevant: "config_load", candidates: vec!["cache", "config_load", "config_save"] },
        Case { query: "avoid recomputing vectors by keeping them keyed on a hash of the text", relevant: "cache", candidates: vec!["audit", "cache", "chunk"] },
        Case { query: "the metric for how frequently the correct answer lands in the top results", relevant: "eval", candidates: vec!["bm25", "eval", "health"] },
        Case { query: "how the go side talks to the rust engine", relevant: "grpc", candidates: vec!["audit", "grpc", "health"] },
        Case { query: "term frequency inverse document frequency keyword scoring of a document", relevant: "bm25", candidates: vec!["audit", "bm25", "hybrid"] },
        Case { query: "what does cosine similarity compute for a pair of embedding vectors", relevant: "cosine", candidates: vec!["cjk_vector", "cosine", "vector_backend"] },
        Case { query: "中文如何先切词再建立倒排索引", relevant: "cjk_index", candidates: vec!["audit", "cjk_index", "cjk_vector"] },
    ]
}

/// Build a candidate `SearchResult` carrying a uniform (no-relevance-prior) score so the
/// IdentityReranker baseline ranks ≈ chance (score desc → all tied → chunk_id asc tie-break).
fn cand(id: &str, content: &str) -> SearchResult {
    SearchResult {
        chunk_id: id.into(),
        context_id: String::new(),
        source_type: String::new(),
        file_path: format!("{id}.md"),
        line_start: 0,
        line_end: 0,
        score: 0.0,
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

/// Returns (MRR, recall@1) of `reranker` over the labeled cases.
fn measure(label: &str, reranker: &Arc<dyn Reranker>) -> (f32, f32) {
    let corpus = docs();
    let text_of = |id: &str| -> String {
        corpus
            .iter()
            .find(|(d, _)| *d == id)
            .map(|(_, t)| (*t).to_string())
            .unwrap_or_else(|| panic!("unknown doc id {id}"))
    };
    let cs = cases();
    let (mut mrr_sum, mut hit1) = (0f32, 0u32);
    for c in &cs {
        let candidates: Vec<SearchResult> =
            c.candidates.iter().map(|id| cand(id, &text_of(id))).collect();
        let out = reranker.rerank(c.query, &candidates).expect("rerank");
        let rank = out
            .iter()
            .position(|r| r.chunk_id == c.relevant)
            .map(|p| p + 1)
            .expect("relevant doc must appear in rerank output (rerank is a permutation)");
        if rank == 1 {
            hit1 += 1;
        }
        mrr_sum += 1.0 / rank as f32;
        eprintln!(
            "  [{label}] relevant={:<14} rank={} q={:?}",
            c.relevant, rank, c.query
        );
    }
    let n = cs.len() as f32;
    (mrr_sum / n, hit1 as f32 / n)
}

// Non-network contract + routing + well-formed guard. Runs ALWAYS (no key needed) so the harness has
// a deterministic logic check even when the live test honest-defers (ADR-013).
#[test]
fn test_rerank_contract_and_routing() {
    // build_rerank_request_body contract (pure, no network).
    let body =
        build_rerank_request_body("Qwen/Qwen3-VL-Reranker-8B", "save config", &["a".to_string(), "b".to_string()], 2);
    assert_eq!(body["model"], "Qwen/Qwen3-VL-Reranker-8B");
    assert_eq!(body["query"], "save config");
    assert_eq!(body["documents"][0], "a");
    assert_eq!(body["documents"][1], "b");
    assert_eq!(body["top_n"], 2);
    assert_eq!(body["return_documents"], false);

    // parse_rerank_response contract: sorted by score desc; malformed / empty / missing → Err.
    let parsed = parse_rerank_response(
        r#"{"results":[{"index":1,"relevance_score":0.20},{"index":0,"relevance_score":0.90}],"meta":{}}"#,
    )
    .expect("fixture parses");
    assert_eq!(parsed, vec![(0usize, 0.90f32), (1, 0.20)]);
    assert!(parse_rerank_response("{not json").is_err(), "malformed JSON → Err");
    assert!(parse_rerank_response(r#"{"results":[]}"#).is_err(), "empty results → Err");
    assert!(
        parse_rerank_response(r#"{"results":[{"relevance_score":0.5}]}"#).is_err(),
        "missing index → Err"
    );
    assert!(
        parse_rerank_response(r#"{"results":[{"index":0}]}"#).is_err(),
        "missing relevance_score → Err"
    );

    // select_reranker routing (no network): defaults → IdentityReranker; unknown → Err.
    assert_eq!(select_reranker("").expect("default").name(), "identity-rerank");
    assert_eq!(select_reranker("identity").expect("identity").name(), "identity-rerank");
    assert!(select_reranker("does-not-exist").is_err(), "unknown reranker → Err");

    // labeled set well-formed: unique doc ids; every relevant exists + is in its candidates; every
    // candidate exists; non-trivial case count.
    let corpus = docs();
    let mut ids: Vec<&str> = corpus.iter().map(|(id, _)| *id).collect();
    let n = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), n, "doc ids must be unique");
    for c in cases() {
        assert!(
            corpus.iter().any(|(id, _)| *id == c.relevant),
            "case relevant id {:?} must exist in the corpus",
            c.relevant
        );
        assert!(
            c.candidates.contains(&c.relevant),
            "case relevant id {:?} must be among its candidates",
            c.relevant
        );
        for cid in &c.candidates {
            assert!(
                corpus.iter().any(|(id, _)| id == cid),
                "candidate id {cid:?} must exist in the corpus"
            );
        }
    }
    assert!(cases().len() >= 12, "need a non-trivial number of labeled cases");
}

// Live rerank quality: real remote cross-encoder vs no-semantic IdentityReranker baseline on the
// SAME labeled set. env-gated on CONTEXTFORGE_RERANKER_API_KEY; honest-defer skip when unset.
#[test]
fn test_remote_rerank_quality() {
    if std::env::var("CONTEXTFORGE_RERANKER_API_KEY").is_err() {
        eprintln!(
            "SKIP test_remote_rerank_quality: CONTEXTFORGE_RERANKER_API_KEY unset \
             (honest-defer, ADR-013; set CONTEXTFORGE_RERANKER_ENDPOINT/_MODEL/_API_KEY with a \
             real rerank endpoint to run)"
        );
        return;
    }

    let identity = select_reranker("identity").expect("identity reranker");
    let (i_mrr, i_r1) = measure("identity", &identity);

    let remote = match select_reranker("remote") {
        Ok(r) => r,
        Err(e) => {
            eprintln!("SKIP test_remote_rerank_quality: remote select failed (honest-defer): {e}");
            return;
        }
    };
    let (r_mrr, r_r1) = measure("remote", &remote);

    let n = cases().len();
    eprintln!(
        "REMOTE-RERANK quality over {n} labeled cases | identity: MRR={i_mrr:.4} recall@1={i_r1:.4} | \
         remote: MRR={r_mrr:.4} recall@1={r_r1:.4} | delta_MRR={:+.4} delta_r1={:+.4}",
        r_mrr - i_mrr,
        r_r1 - i_r1
    );

    // Guards: a real cross-encoder must clear an MRR floor AND beat the no-semantic baseline (the
    // whole point of remote reranking). Floors are regression guards on this small set, not a quality
    // claim (ADR-013). recall@1 is reported, not floored (rerank can vary run-to-run across a paid
    // remote model — see task-37.1's cross-run variance note).
    assert!(
        r_mrr >= 0.70,
        "remote MRR={r_mrr:.4} below floor 0.70 — a real cross-encoder should rank obvious relevant docs near the top"
    );
    assert!(
        r_mrr > i_mrr,
        "remote MRR={r_mrr:.4} must beat no-semantic identity baseline MRR={i_mrr:.4} (else remote rerank buys nothing)"
    );
}
