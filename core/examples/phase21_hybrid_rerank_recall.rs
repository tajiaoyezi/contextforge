//! task-21.3: real hybrid (RRF) + reranked (cross-encoder) recall vs the BM25 baseline, through the
//! PRODUCTION `Retriever`, over the dogfood corpus. Mirrors `phase20_recall_via_retriever`'s harness:
//! index the 6 golden-question expected files + 5 distractors via the production `IndexSession` (real
//! scanner + chunker), wire the real `FastEmbedProvider` (`all-MiniLM-L6-v2`, dim 384) + the 0-dep
//! `BruteForceVectorBackend`, then for each of the 30 golden queries measure file-level recall@5/@10 +
//! top-1 + MRR for three retrieval methods over the SAME corpus + queries:
//!   - baseline BM25          (`Retriever::search`)
//!   - hybrid RRF fusion      (`Retriever::search_hybrid`; ADR-025)
//!   - reranked cross-encoder (`search_hybrid` top-k re-ordered via `with_reranker(CrossEncoderReranker)`; ADR-026)
//!
//! This is the data source for `docs/spikes/phase-21-hybrid-recall.md`, which ratifies ADR-025
//! (hybrid fusion) and ADR-026 (reranker provider). ADR-013: every number is a real model run — no
//! synthetic / deterministic / fabricated figures. The reranked pass needs the cross-encoder model
//! (`reranker-fastembed`); when only `embedding-fastembed` is built it is skipped (hybrid-vs-baseline
//! is still produced, for ADR-025).
//!
//! Run (downloads the ONNX model(s) on first run):
//!   cargo run -p contextforge-core --example phase21_hybrid_rerank_recall --features embedding-fastembed
//!   cargo run -p contextforge-core --example phase21_hybrid_rerank_recall --features embedding-fastembed,reranker-fastembed
//!
//! Default build compiles this as a no-op (no fastembed/ort dependency), so `cargo test --workspace`
//! / `cargo build --workspace` are unaffected. Deterministic hybrid/rerank wiring is CI-covered by the
//! Rust unit tests (`server.rs::test_21_1_hybrid_dispatches_fusion_path`,
//! `rerank::identity::test_21_2_1_*`, `retriever::mod::test_21_2_2_*`).

#[cfg(not(feature = "embedding-fastembed"))]
fn main() {
    eprintln!(
        "phase21_hybrid_rerank_recall: requires --features embedding-fastembed (real all-MiniLM-L6-v2; \
         add reranker-fastembed for the reranked pass). Default build no-op — nothing to do."
    );
}

#[cfg(feature = "embedding-fastembed")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use contextforge_core::chunker::ChunkPolicy;
    use contextforge_core::embedding::{EmbeddingProvider, FastEmbedProvider};
    use contextforge_core::indexer::IndexSession;
    use contextforge_core::retriever::vector::BruteForceVectorBackend;
    use contextforge_core::retriever::{Retriever, SearchFilters, SearchOptions, SearchResult};
    use contextforge_core::scanner::{default_denylist, ScanOptions};

    // 6 golden categories — same expected files as internal/eval BuiltinGoldenQuestions / task-20.2.
    struct Cat {
        stem: &'static str,
        file: &'static str,
        queries: [&'static str; 5],
    }
    let cats = [
        Cat {
            stem: "config-location",
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
            stem: "error-reproduction",
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
            stem: "historical-decision",
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
            stem: "log-troubleshooting",
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
            stem: "agent-memory-rule",
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
            stem: "code-location",
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
    let distractors = [
        "core/src/server.rs",
        "internal/cli/eval.go",
        "internal/daemon/rest.go",
        "core/src/retriever/vector/brute_force.rs",
        "internal/eval/eval.go",
    ];

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let work = std::env::temp_dir().join(format!("cf-phase21-recall-{}-{nanos}", std::process::id()));
    let src = work.join("src");
    let data = work.join("data");
    fs::create_dir_all(&src)?;

    let ext = |rel: &str| -> String {
        PathBuf::from(rel)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("txt")
            .to_string()
    };
    for cat in &cats {
        let body = fs::read_to_string(repo_root.join(cat.file)).map_err(|e| format!("read {}: {e}", cat.file))?;
        fs::write(src.join(format!("{}.{}", cat.stem, ext(cat.file))), body)?;
    }
    for (i, rel) in distractors.iter().enumerate() {
        let body = fs::read_to_string(repo_root.join(rel)).map_err(|e| format!("read {rel}: {e}"))?;
        fs::write(src.join(format!("distractor-{i}.{}", ext(rel))), body)?;
    }

    // Index via the production pipeline (real scanner + chunker), like task-20.2.
    let coll = "phase21-hybrid-rerank-recall";
    let scan_opts = ScanOptions {
        denylist: default_denylist(),
        allowlist: Vec::new(),
        allow_denylist_override: false,
        dry_run: false,
        max_file_bytes: 10 * 1024 * 1024,
    };
    let mut sess = IndexSession::open(&data, coll)?;
    sess.index_path(&src, &scan_opts, &ChunkPolicy::default(), vec![])?;
    sess.commit()?;
    drop(sess);

    // Wire the real embedder + the 0-dep default backend onto a production Retriever, build the
    // on-demand semantic index from the collection's own chunks (shared via the backend Arc).
    let provider = FastEmbedProvider::new();
    let provider_name = provider.name().to_string();
    let provider_dim = provider.dim();
    let embedder: Arc<dyn EmbeddingProvider> = Arc::new(provider);
    let backend = Arc::new(BruteForceVectorBackend::new());
    let base = Retriever::open(&data, coll)?
        .with_embedder(embedder.clone())
        .with_vector_searcher(backend.clone());
    let items = base.enumerate_chunks()?;
    eprintln!("indexed {} production chunks; embedding via {provider_name} (dim {provider_dim})...", items.len());
    base.index_chunks_semantic(backend.as_ref(), &items)?;

    // report measures file-level recall@5/@10 + top-1 + MRR for one retrieval method over the 30
    // golden queries (a hit = first result whose file_path carries the query's unique category stem).
    let report = |label: &str,
                  search: &mut dyn FnMut(&str) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>>|
     -> Result<(), Box<dyn std::error::Error>> {
        let (mut t5, mut t10, mut top1, mut nq) = (0usize, 0usize, 0usize, 0usize);
        let mut mrr = 0.0f64;
        for cat in &cats {
            for q in &cat.queries {
                nq += 1;
                let hits = search(q)?;
                let mut matched: Option<usize> = None;
                for (rank, h) in hits.iter().enumerate() {
                    if h.file_path.contains(cat.stem) {
                        matched = Some(rank + 1);
                        break;
                    }
                }
                if let Some(r) = matched {
                    if r == 1 {
                        top1 += 1;
                    }
                    if r <= 5 {
                        t5 += 1;
                    }
                    if r <= 10 {
                        t10 += 1;
                    }
                    mrr += 1.0 / r as f64;
                }
            }
        }
        let nqf = nq as f64;
        let r10 = t10 as f64 / nqf;
        println!(
            "  {label:24} recall@5={:.4} recall@10={:.4} top1={:.4} mrr={:.4} gate@10(>=0.70)={}",
            t5 as f64 / nqf,
            r10,
            top1 as f64 / nqf,
            mrr / nqf,
            if r10 >= 0.70 { "pass" } else { "fail" }
        );
        Ok(())
    };

    println!("=== task-21.3 dogfood recall: BM25 baseline vs hybrid (RRF) vs reranked (cross-encoder) — real run (ADR-013) ===");
    println!(
        "provider={provider_name} dim={provider_dim} production_chunks={} queries=30 backend=BruteForceVectorBackend(exact-cosine)",
        items.len()
    );
    report("baseline-bm25", &mut |q| {
        let opts = SearchOptions {
            query: q.to_string(),
            top_k: 10,
            filters: SearchFilters::default(),
            explain: false,
        };
        Ok(base.search(&opts)?)
    })?;
    report("hybrid-rrf", &mut |q| Ok(base.search_hybrid(q, 10)?))?;

    #[cfg(feature = "reranker-fastembed")]
    {
        use contextforge_core::rerank::CrossEncoderReranker;
        let reranked_retr = Retriever::open(&data, coll)?
            .with_embedder(embedder.clone())
            .with_vector_searcher(backend.clone())
            .with_reranker(Arc::new(CrossEncoderReranker::new()));
        report("reranked-cross-encoder", &mut |q| Ok(reranked_retr.search_hybrid(q, 10)?))?;
        println!("rerank_provider=fastembed-bge-reranker-base (real cross-encoder model run, ADR-013)");
    }
    #[cfg(not(feature = "reranker-fastembed"))]
    println!("  reranked-cross-encoder   SKIPPED (build with --features embedding-fastembed,reranker-fastembed for ADR-026 uplift data)");

    let _ = fs::remove_dir_all(&work);
    Ok(())
}
