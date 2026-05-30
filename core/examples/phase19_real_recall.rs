//! task-19.5: real SemanticRecall@K harness (Phase 19 vector-retrieval-integration).
//!
//! Produces REAL recall evidence (ADR-013: no fabricated numbers) by running the real
//! `FastEmbedProvider` (all-MiniLM-L6-v2, dim 384) over real ContextForge source/doc text:
//!   1. Chunk the 6 golden-question expected files (internal/eval BuiltinGoldenQuestions) +
//!      a few extra real files as distractors into line windows → retrieval corpus.
//!   2. Embed every chunk and every golden query with the real model.
//!   3. Index the chunks into the default-available `BruteForceVectorBackend` (exact cosine).
//!   4. For each of the 30 golden queries, search top-10 and record the rank of the first hit
//!      from the query's expected file → SemanticRecall@5 / @10 (file-level strong hit @ K).
//!   5. Write the real embeddings to test/fixtures/eval/dogfood-embeddings.jsonl and print a
//!      structured report (overall + per-category recall + gate verdict) for the spike doc.
//!
//! Run (WSL/Linux or Windows MSVC; downloads the ONNX model on first run):
//!   cargo run -p contextforge-core --example phase19_real_recall --features embedding-fastembed
//!
//! The default build compiles this as a no-op stub (no fastembed/ort dependency), so
//! `cargo test --workspace` / `cargo build --workspace` are unaffected (AC5).

#[cfg(not(feature = "embedding-fastembed"))]
fn main() {
    eprintln!(
        "phase19_real_recall: requires --features embedding-fastembed (real all-MiniLM-L6-v2). \
         Default build stub — nothing to do."
    );
}

#[cfg(feature = "embedding-fastembed")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::BTreeMap;
    use std::fmt::Write as _;
    use std::fs;
    use std::path::PathBuf;

    use contextforge_core::embedding::{EmbeddingProvider, FastEmbedProvider};
    use contextforge_core::retriever::vector::{
        BruteForceVectorBackend, ChunkId, VectorChunk, VectorIndexConfig, VectorIndexer,
        VectorMetric, VectorSearcher,
    };

    // 6 golden categories — transcribed verbatim from internal/eval/eval.go BuiltinGoldenQuestions.
    struct Cat {
        name: &'static str,
        file: &'static str,
        queries: [&'static str; 5],
    }
    let cats = [
        Cat {
            name: "config-location",
            file: "internal/config/config.go",
            queries: [
                "where is the config loader",
                "which file initializes config permissions",
                "where is default data dir resolved",
                "find the schema version config constant",
                "where are config directory modes enforced",
            ],
        },
        Cat {
            name: "error-reproduction",
            file: "internal/daemon/daemon.go",
            queries: [
                "how does daemon restart after crash",
                "where is daemon health timeout handled",
                "how is loopback bind validated",
                "where does core binary lookup fail",
                "how is daemon stop made idempotent",
            ],
        },
        Cat {
            name: "historical-decision",
            file: "docs/decisions/adr-007-minimal-tarball-distribution.md",
            queries: [
                "why is v0.1 a minimal tarball",
                "which ADR rejects single language package distribution",
                "where is Docker compose scoped for release",
                "what is the rollback plan for release tarball",
                "which ADR covers v0.1 distribution",
            ],
        },
        Cat {
            name: "log-troubleshooting",
            file: "internal/memoryops/audit/audit.go",
            queries: [
                "where are audit events written",
                "how is search audit metadata recorded",
                "where is export content kept out of audit",
                "which audit code records unauthorized access",
                "where is audit log append implemented",
            ],
        },
        Cat {
            name: "agent-memory-rule",
            file: "docs/s2v-adapter.md",
            queries: [
                "where are subagent rules documented",
                "what rule prevents subagent lockfile edits",
                "where is review subagent protocol described",
                "which adapter section lists task worktrees",
                "where is ADR-012 governance autonomy referenced",
            ],
        },
        Cat {
            name: "code-location",
            file: "core/src/retriever/mod.rs",
            queries: [
                "where is BM25 search implemented",
                "which code builds explainable search results",
                "where is get chunk fast path implemented",
                "which retriever code synthesizes provenance",
                "where are search filters applied",
            ],
        },
    ];

    // Extra real files indexed as pure distractors (no query targets them) to enlarge the corpus
    // so top-10 is selective rather than trivially covering everything.
    let distractor_files = [
        "core/src/server.rs",
        "internal/cli/eval.go",
        "internal/daemon/rest.go",
        "core/src/retriever/vector/brute_force.rs",
        "internal/eval/eval.go",
    ];

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    const WINDOW: usize = 40; // lines per chunk
    // Cap chunks per file so no single large file (e.g. retriever/mod.rs) dominates the corpus and
    // makes file-level "any chunk in top-K" recall trivially 1.0. Capping at the first N windows also
    // keeps each file's leading region (≈ the golden ExpectedLineRange 1-120) — the relevant span.
    const MAX_CHUNKS_PER_FILE: usize = 4;

    // (chunk_id, text, owning-category-or-"distractor")
    let mut corpus: Vec<(String, String, &str)> = Vec::new();
    let mut per_file_chunks: Vec<(&str, usize)> = Vec::new();

    let chunk_file = |slug: &str, file: &str, owner: &'static str,
                      corpus: &mut Vec<(String, String, &str)>|
     -> Result<usize, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(repo_root.join(file))
            .map_err(|e| format!("read {file}: {e}"))?;
        let lines: Vec<&str> = content.lines().collect();
        let mut n = 0usize;
        let mut i = 0usize;
        while i < lines.len() && n < MAX_CHUNKS_PER_FILE {
            let end = (i + WINDOW).min(lines.len());
            let text = lines[i..end].join("\n");
            if !text.trim().is_empty() {
                corpus.push((format!("{slug}-{n:04}"), text, owner));
                n += 1;
            }
            i = end;
        }
        Ok(n)
    };

    for cat in &cats {
        let n = chunk_file(cat.name, cat.file, cat.name, &mut corpus)?;
        per_file_chunks.push((cat.file, n));
    }
    for (di, file) in distractor_files.iter().enumerate() {
        let slug = match di {
            0 => "distractor-server",
            1 => "distractor-eval",
            2 => "distractor-rest",
            3 => "distractor-bruteforce",
            _ => "distractor-evalpkg",
        };
        let n = chunk_file(slug, file, "distractor", &mut corpus)?;
        per_file_chunks.push((file, n));
    }

    let provider = FastEmbedProvider::new();
    eprintln!(
        "embedding {} corpus chunks via {} (dim {})...",
        corpus.len(),
        provider.name(),
        provider.dim()
    );
    let corpus_texts: Vec<String> = corpus.iter().map(|(_, t, _)| t.clone()).collect();
    let corpus_embs = provider.embed(&corpus_texts)?;

    let backend = BruteForceVectorBackend::new();
    backend.open(VectorIndexConfig {
        dim: provider.dim(),
        metric: VectorMetric::Cosine,
        persistence_path: None,
        collection_id: "phase19-real-recall".into(),
    })?;
    let vchunks: Vec<VectorChunk> = corpus
        .iter()
        .zip(corpus_embs.iter())
        .map(|((id, _, _), e)| VectorChunk {
            chunk_id: ChunkId(id.clone()),
            embedding: e.clone(),
            metadata: None,
        })
        .collect();
    backend.index_batch(&vchunks)?;

    // Embed the 30 golden queries.
    let mut query_texts: Vec<String> = Vec::new();
    let mut query_cat: Vec<&str> = Vec::new();
    for cat in &cats {
        for q in &cat.queries {
            query_texts.push((*q).to_string());
            query_cat.push(cat.name);
        }
    }
    let query_embs = provider.embed(&query_texts)?;

    // Search + file-level strong-hit@K recall (expected = a chunk whose slug prefix is the category).
    // Also track top-1 accuracy (first hit is from the expected file) and MRR of the first expected
    // hit — far more discriminating than "any chunk in top-K" on a balanced corpus.
    let mut per_cat: BTreeMap<&str, (usize, usize, usize)> = BTreeMap::new(); // cat -> (hit@5, hit@10, n)
    let (mut tot5, mut tot10, mut top1) = (0usize, 0usize, 0usize);
    let mut mrr_sum = 0.0f64;
    for (qi, qe) in query_embs.iter().enumerate() {
        let cat = query_cat[qi];
        let want_prefix = format!("{cat}-");
        let hits = backend.search(qe, 10, None)?;
        let mut matched: Option<usize> = None;
        for (rank, h) in hits.iter().enumerate() {
            if h.chunk_id.0.starts_with(&want_prefix) {
                matched = Some(rank + 1);
                break;
            }
        }
        let e = per_cat.entry(cat).or_insert((0, 0, 0));
        e.2 += 1;
        if let Some(r) = matched {
            if r == 1 {
                top1 += 1;
            }
            if r <= 5 {
                e.0 += 1;
                tot5 += 1;
            }
            if r <= 10 {
                e.1 += 1;
                tot10 += 1;
            }
            mrr_sum += 1.0 / r as f64;
        }
    }
    let nq = query_texts.len();
    let recall5 = tot5 as f64 / nq as f64;
    let recall10 = tot10 as f64 / nq as f64;
    let top1_acc = top1 as f64 / nq as f64;
    let mrr = mrr_sum / nq as f64;

    // Write the real-embedding fixture (chunk_id + real vector), matching bench load_dogfood format.
    let fixture_dir = repo_root.join("test/fixtures/eval");
    fs::create_dir_all(&fixture_dir)?;
    let fixture_path = fixture_dir.join("dogfood-embeddings.jsonl");
    let mut out = String::new();
    for ((id, _, _), e) in corpus.iter().zip(corpus_embs.iter()) {
        let emb_json = e
            .iter()
            .map(|x| format!("{x:.6}"))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(out, "{{\"chunk_id\": \"{id}\", \"embedding\": [{emb_json}]}}")?;
    }
    fs::write(&fixture_path, &out)?;

    // Structured report (transcribe into docs/spikes/phase-19-real-recall.md).
    println!("=== task-19.5 REAL SemanticRecall (provider={}, dim={}) ===", provider.name(), provider.dim());
    println!("corpus_chunks={} queries={} window_lines={}", corpus.len(), nq, WINDOW);
    for (f, n) in &per_file_chunks {
        println!("  file={f} chunks={n}");
    }
    println!("overall semantic_recall_at_5={recall5:.4} semantic_recall_at_10={recall10:.4}");
    println!("discriminating: top1_accuracy={top1_acc:.4} mrr={mrr:.4}");
    for (cat, (h5, h10, t)) in &per_cat {
        println!(
            "  category={cat} recall@5={:.4} recall@10={:.4} n={t}",
            *h5 as f64 / *t as f64,
            *h10 as f64 / *t as f64
        );
    }
    let gate = if recall10 >= 0.70 { "pass" } else { "fail" };
    println!("gate(SemanticRecall@10>=0.70)={gate}");
    println!("fixture_written={} ({} lines)", fixture_path.display(), corpus.len());
    Ok(())
}
