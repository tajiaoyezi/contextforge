//! task-20.2: real SemanticRecall@K through the PRODUCTION `Retriever::search_semantic` hot path.
//!
//! task-19.5 (`phase19_real_recall`) measured real recall against a standalone `BruteForceVectorBackend`
//! with a controlled 40-line-window corpus. This harness instead routes recall through the **real
//! production pipeline** (ADR-013: no fabricated numbers):
//!   1. Write the 6 golden-question expected files + 5 distractor files into a temp source tree.
//!   2. Index them with the production `IndexSession` (real scanner + chunker) → a real collection.
//!   3. Open a `Retriever`, wire the real `FastEmbedProvider` (all-MiniLM-L6-v2, dim 384) + the
//!      0-dep default `BruteForceVectorBackend`, and build the on-demand semantic index from the
//!      collection's own chunks via `enumerate_chunks` + `index_chunks_semantic` — exactly what
//!      `core/src/server.rs` (CoreService) and `core/src/data_plane/search.rs` (console-api, task-20.1)
//!      do at request time.
//!   4. For each of the 30 golden queries, `Retriever::search_semantic` top-10 and record the rank of
//!      the first hit whose file_path belongs to the query's expected file → file-level SemanticRecall@5/
//!      @10 + top-1 + MRR. The production chunker is uncapped (unlike task-19.5's MAX_CHUNKS_PER_FILE),
//!      so top-1/MRR are the discriminating metrics; differences from task-19.5 are reported honestly.
//!
//! Run (downloads the ONNX model on first run):
//!   cargo run -p contextforge-core --example phase20_recall_via_retriever --features embedding-fastembed
//!
//! Default build compiles this as a no-op (no fastembed/ort dependency), so `cargo test --workspace`
//! / `cargo build --workspace` are unaffected. Deterministic-embedding hot-path wiring is CI-covered by
//! `core/src/retriever/mod.rs::test_20_2_recall_via_retriever_brute_force_default_build`.

#[cfg(not(feature = "embedding-fastembed"))]
fn main() {
    eprintln!(
        "phase20_recall_via_retriever: requires --features embedding-fastembed (real all-MiniLM-L6-v2). \
         Default build no-op — nothing to do."
    );
}

#[cfg(feature = "embedding-fastembed")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use contextforge_core::chunker::ChunkPolicy;
    use contextforge_core::embedding::{EmbeddingProvider, FastEmbedProvider};
    use contextforge_core::indexer::IndexSession;
    use contextforge_core::retriever::vector::BruteForceVectorBackend;
    use contextforge_core::retriever::Retriever;
    use contextforge_core::scanner::{default_denylist, ScanOptions};

    // 6 golden categories — same expected files as internal/eval BuiltinGoldenQuestions / task-19.5.
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
    let work = std::env::temp_dir().join(format!("cf-phase20-recall-{}-{nanos}", std::process::id()));
    let src = work.join("src");
    let data = work.join("data");
    fs::create_dir_all(&src)?;

    // Write each real file into the temp source tree under a unique stem (so file_path carries the
    // category, avoiding basename collisions like the two eval.go files).
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

    // Index via the production pipeline (real scanner + chunker).
    let coll = "phase20-recall-via-retriever";
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
    // on-demand semantic index from the collection's own chunks, then run search_semantic.
    let provider = FastEmbedProvider::new();
    let provider_name = provider.name().to_string();
    let provider_dim = provider.dim();
    let embedder: Arc<dyn EmbeddingProvider> = Arc::new(provider);
    let backend = Arc::new(BruteForceVectorBackend::new());
    let retr = Retriever::open(&data, coll)?
        .with_embedder(embedder)
        .with_vector_searcher(backend.clone());
    let items = retr.enumerate_chunks()?;
    eprintln!("indexed {} production chunks; embedding via {provider_name} (dim {provider_dim})...", items.len());
    retr.index_chunks_semantic(backend.as_ref(), &items)?;

    let mut per_cat: BTreeMap<&str, (usize, usize, usize)> = BTreeMap::new();
    let (mut tot5, mut tot10, mut top1) = (0usize, 0usize, 0usize);
    let mut mrr_sum = 0.0f64;
    let mut nq = 0usize;
    for cat in &cats {
        for q in &cat.queries {
            nq += 1;
            let hits = retr.search_semantic(q, 10)?;
            let mut matched: Option<usize> = None;
            for (rank, h) in hits.iter().enumerate() {
                // file_path carries the temp src path; a hit belongs to this category iff its path
                // contains the unique category stem.
                if h.file_path.contains(cat.stem) {
                    matched = Some(rank + 1);
                    break;
                }
            }
            let e = per_cat.entry(cat.stem).or_insert((0, 0, 0));
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
    }
    let recall5 = tot5 as f64 / nq as f64;
    let recall10 = tot10 as f64 / nq as f64;
    let top1_acc = top1 as f64 / nq as f64;
    let mrr = mrr_sum / nq as f64;

    println!("=== task-20.2 REAL SemanticRecall via Retriever::search_semantic (provider={provider_name}, dim={provider_dim}) ===");
    println!("production_chunks={} queries={nq} backend=BruteForceVectorBackend(exact-cosine)", items.len());
    println!("overall semantic_recall_at_5={recall5:.4} semantic_recall_at_10={recall10:.4}");
    println!("discriminating: top1_accuracy={top1_acc:.4} mrr={mrr:.4}");
    for (cat, (h5, h10, t)) in &per_cat {
        println!(
            "  category={cat} recall@5={:.4} recall@10={:.4} n={t}",
            *h5 as f64 / *t as f64,
            *h10 as f64 / *t as f64
        );
    }
    println!("gate(SemanticRecall@10>=0.70)={}", if recall10 >= 0.70 { "pass" } else { "fail" });
    let _ = fs::remove_dir_all(&work);
    Ok(())
}
