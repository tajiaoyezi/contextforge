// remote embedding live semantic recall harness.
//
// Measures whether REAL embeddings (an OpenAI-compatible remote provider, e.g.
// Qwen/Qwen3-Embedding-8B via SiliconFlow) produce semantically correct top-k
// rankings on a small AUTHOR-LABELED set, and compares them against the model-free
// `DeterministicEmbeddingProvider` baseline on the SAME set + SAME exact cosine
// backend. The delta is what real embeddings actually buy: the deterministic
// provider has no semantics (hash-derived vectors) so it ranks near chance, while
// a real model should rank the labeled-relevant doc into the top-k.
//
// This is the first real redemption of [SPEC-DEFER:phase-future.embedding-provider-remote]
// (real network 联调 + real recall numbers; in-repo only ever had the deterministic
// default + a feature-gated fastembed path, never a measured remote recall).
//
// HONEST SCOPE (ADR-013): this is a small hand-labeled semantic set proving a real
// model ranks obvious paraphrase / cross-lingual / code-concept pairs correctly
// against deliberate near-distractors — it is NOT a large standardized benchmark.
// recall floors are regression guards, not a quality ceiling. Large-corpus semantic
// quality stays [SPEC-DEFER:phase-future.embedding-large-corpus-recall].
//
// env-gated: runs ONLY when CONTEXTFORGE_REMOTE_API_KEY is set (the factory also
// reads CONTEXTFORGE_REMOTE_ENDPOINT / _MODEL / _PROVIDER from env; the api key is
// never logged). Unset → SKIP cleanly (honest-defer; CI has no credentials — no
// fabricated pass, no fabricated numbers). Only compiled under `--features
// embedding-remote`; the default `cargo test --workspace` build never sees it.
#![cfg(feature = "embedding-remote")]

use contextforge_core::embedding::{select_provider, EmbeddingProvider};
use contextforge_core::retriever::vector::brute_force::BruteForceVectorBackend;
use contextforge_core::retriever::vector::traits::{VectorIndexer, VectorSearcher};
use contextforge_core::retriever::vector::types::{
    ChunkId, VectorChunk, VectorIndexConfig, VectorMetric,
};
use std::sync::Arc;

// Qwen3-Embedding-8B native dim is 4096; we request 1024 via the OpenAI-style
// `dimensions` param (Matryoshka) — lighter payload + faster brute-force, quality
// preserved. The deterministic baseline uses the same dim for an apples-to-apples
// cosine comparison.
const DIM: usize = 1024;

/// One labeled case: a query whose single SEMANTICALLY relevant document is `relevant`.
struct Case {
    query: &'static str,
    relevant: &'static str,
    category: &'static str,
}

/// Corpus: (id, text). Includes near-distractors that share vocabulary with several
/// queries (config save vs load; bm25 vs hybrid; cjk index vs cjk vector) so that
/// landing the labeled doc in the top-k is not lexically trivial.
fn docs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("config_load", "The configuration loader reads settings from config.toml when the daemon starts up."),
        ("config_save", "Persisting user preferences writes the modified settings back to disk on shutdown."),
        ("tokenizer", "A custom analyzer splits camelCase and snake_case identifiers such as RetrieverConfig into separate searchable tokens."),
        ("vector_backend", "Pick which approximate nearest-neighbor vector store to use — qdrant or lancedb — through the configuration file."),
        ("cosine", "The similarity between two embeddings is the cosine of the angle between the vectors."),
        ("bm25", "Lexical ranking scores a document by term frequency and inverse document frequency of the query words."),
        ("hybrid", "Fusing keyword relevance scores together with vector similarity into a single combined ranking."),
        ("reranker", "A cross-encoder re-orders the top candidate documents by jointly reading the query and each document together."),
        ("cjk_index", "中文文本先用分词器切分为词语，再建立倒排索引以便检索。"),
        ("cjk_vector", "向量检索在嵌入空间里用余弦相似度寻找最近邻文档。"),
        ("cache", "Computed embeddings are cached by the content hash of the input so identical text is never re-embedded."),
        ("health", "A readiness probe reports whether the data plane and its backing dependencies are reachable."),
        ("audit", "Every memory pin and delete operation is recorded in an append-only audit log."),
        ("eval", "Recall at k measures how often the relevant document appears within the top k retrieved results."),
        ("grpc", "The Go control plane communicates with the Rust core engine over a gRPC bridge."),
        ("chunk", "Source files are split into overlapping chunks before each chunk is embedded into a vector."),
    ]
}

/// Labeled queries. Each is a paraphrase / cross-lingual / concept restatement of
/// exactly one doc, chosen so a real model must rely on meaning (not shared words).
fn cases() -> Vec<Case> {
    vec![
        Case { query: "how does the application read its settings when it launches", relevant: "config_load", category: "en-paraphrase" },
        Case { query: "a routine that breaks identifiers like getUserName apart into words for searching", relevant: "tokenizer", category: "code-concept" },
        Case { query: "what does cosine similarity compute for a pair of vectors", relevant: "cosine", category: "en-paraphrase" },
        Case { query: "choose the database that stores vectors for nearest neighbor search", relevant: "vector_backend", category: "en-paraphrase" },
        Case { query: "在嵌入空间用余弦距离找最相近的文档", relevant: "cjk_vector", category: "cjk" },
        Case { query: "segment chinese text into words and then build a search index", relevant: "cjk_index", category: "cross-lingual" },
        Case { query: "blend lexical keyword scores with semantic vector scores into one list", relevant: "hybrid", category: "en-paraphrase" },
        Case { query: "re-order the shortlisted results by reading the query and document jointly", relevant: "reranker", category: "en-paraphrase" },
        Case { query: "avoid recomputing vectors by keeping them keyed on a hash of the text", relevant: "cache", category: "en-paraphrase" },
        Case { query: "the metric for how frequently the correct answer is in the top results", relevant: "eval", category: "en-paraphrase" },
        Case { query: "how the go side talks to the rust engine", relevant: "grpc", category: "en-paraphrase" },
        Case { query: "cut documents into overlapping pieces prior to embedding", relevant: "chunk", category: "en-paraphrase" },
        Case { query: "term frequency inverse document frequency keyword scoring", relevant: "bm25", category: "code-concept" },
        Case { query: "中文如何切词后建立索引", relevant: "cjk_index", category: "cjk" },
        Case { query: "check that the service and the systems it depends on are up", relevant: "health", category: "en-paraphrase" },
    ]
}

/// Build a brute-force exact-cosine index of the corpus embedded by `provider`.
fn index_with(provider: &Arc<dyn EmbeddingProvider>) -> (BruteForceVectorBackend, Vec<&'static str>) {
    let corpus = docs();
    let texts: Vec<String> = corpus.iter().map(|(_, t)| t.to_string()).collect();
    let embs = provider.embed(&texts).expect("embed corpus");
    assert_eq!(embs.len(), corpus.len());
    let chunks: Vec<VectorChunk> = corpus
        .iter()
        .zip(embs)
        .map(|((id, _), e)| VectorChunk {
            chunk_id: ChunkId((*id).to_string()),
            embedding: e,
            metadata: None,
        })
        .collect();
    let backend = BruteForceVectorBackend::new();
    backend
        .open(VectorIndexConfig {
            dim: DIM,
            metric: VectorMetric::Cosine,
            persistence_path: None,
            collection_id: "remote_embed_recall".to_string(),
        })
        .expect("open");
    let n = backend.index_batch(&chunks).expect("index_batch");
    assert_eq!(n, corpus.len());
    (backend, corpus.iter().map(|(id, _)| *id).collect())
}

/// Returns (recall@1, recall@3) of `provider` over the labeled cases.
fn measure(label: &str, provider: Arc<dyn EmbeddingProvider>) -> (f32, f32) {
    assert_eq!(provider.dim(), DIM, "{label} provider dim must be {DIM}");
    let (backend, _ids) = index_with(&provider);
    let cs = cases();
    let q_texts: Vec<String> = cs.iter().map(|c| c.query.to_string()).collect();
    let q_embs = provider.embed(&q_texts).expect("embed queries");

    let (mut hit1, mut hit3) = (0u32, 0u32);
    for (c, q) in cs.iter().zip(q_embs.iter()) {
        let hits = backend.search(q, 3, None).expect("search");
        let ranked: Vec<String> = hits.iter().map(|h| h.chunk_id.0.clone()).collect();
        let in1 = ranked.first().map(|x| x == c.relevant).unwrap_or(false);
        let in3 = ranked.iter().any(|x| x == c.relevant);
        if in1 {
            hit1 += 1;
        }
        if in3 {
            hit3 += 1;
        }
        eprintln!(
            "  [{label}] {:<14} q={:.40?} -> top3={:?} {}",
            c.category,
            c.query,
            ranked,
            if in1 { "HIT@1" } else if in3 { "hit@3" } else { "MISS" }
        );
    }
    let n = cs.len() as f32;
    (hit1 as f32 / n, hit3 as f32 / n)
}

// Non-network guard: the labeled set is well-formed (unique doc ids; every
// relevant id exists; query/doc counts sane). Runs ALWAYS (no key needed) so the
// harness has a deterministic logic check even when the live test honest-defers.
#[test]
fn test_labeled_set_well_formed() {
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
    }
    assert!(cases().len() >= 12, "need a non-trivial number of labeled cases");
}

// Live semantic recall: real remote model vs deterministic baseline on the SAME
// set. env-gated on CONTEXTFORGE_REMOTE_API_KEY; honest-defer skip when unset.
#[test]
fn test_remote_embedding_semantic_recall() {
    if std::env::var("CONTEXTFORGE_REMOTE_API_KEY").is_err() {
        eprintln!(
            "SKIP test_remote_embedding_semantic_recall: CONTEXTFORGE_REMOTE_API_KEY unset \
             (honest-defer, ADR-013; set CONTEXTFORGE_REMOTE_ENDPOINT/_MODEL/_API_KEY with a \
             real OpenAI-compatible embedding endpoint to run)"
        );
        return;
    }

    let det = select_provider("deterministic", DIM).expect("deterministic provider");
    let (d1, d3) = measure("deterministic", det);

    let remote = match select_provider("remote", DIM) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SKIP test_remote_embedding_semantic_recall: remote select failed (honest-defer): {e}");
            return;
        }
    };
    let (r1, r3) = measure("remote", remote);

    let n = cases().len();
    eprintln!(
        "REMOTE-EMBED semantic recall over {n} labeled cases (dim={DIM}) | \
         deterministic: recall@1={d1:.4} recall@3={d3:.4} | remote: recall@1={r1:.4} recall@3={r3:.4} | \
         delta@1={:+.4} delta@3={:+.4}",
        r1 - d1,
        r3 - d3
    );

    // Guards: a real model must clear a recall floor AND beat the model-free baseline
    // (the whole point of plugging in real embeddings). Floors are regression guards
    // on this small set, not a quality claim (ADR-013).
    assert!(
        r3 >= 0.70,
        "remote recall@3={r3:.4} below floor 0.70 — real embeddings should rank obvious semantic pairs into top-3"
    );
    assert!(
        r1 > d1,
        "remote recall@1={r1:.4} must beat deterministic baseline recall@1={d1:.4} (else real embeddings buy nothing)"
    );
}
