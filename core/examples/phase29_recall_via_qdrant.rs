//! task-29.2: real end-to-end SemanticRecall@K over a LIVE qdrant server, through the PRODUCTION
//! `Retriever::search_semantic` hot path.
//!
//! This is a clone of `core/examples/phase20_recall_via_retriever.rs` (task-20.2) with the single
//! backend swap `BruteForceVectorBackend` → `QdrantBackend::connect(QdrantConnConfig::from_env())`.
//! It is the FIRST real redemption of `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` (task-25.1
//! §3): task-25.1 froze the qdrant lifecycle CONTRACT (connect / health / decide_ensure / ensure-
//! create open) and unit-tested it without a server; the live KNN read path (`qdrant.rs` search) had
//! never been exercised end-to-end against a real server. Here recall is measured over real qdrant
//! KNN — connect → ensure-create → upsert → search — through the same wiring `core/src/server.rs`
//! uses (`index_chunks_semantic` calls `open` then `index_batch`, then `search_semantic` runs KNN).
//!
//! ## Honest-defer (ADR-013, the core invariant)
//! The first thing after `connect` is `backend.health()`. CI has no running qdrant server, so health
//! is `Unreachable` → the harness prints an explanation and `exit 0` WITHOUT fabricating any recall
//! number or pretending the live path passed. Real recall is produced ONLY on the `Ready` branch
//! against a real single-node server (manual / dev-box) and is then backfilled into the task §10 +
//! `docs/releases/v0.22.0-evidence.md`. The feature-on-but-no-server path proves the wiring compiles
//! and defers honestly; it is NOT a measured-recall result.
//!
//! ## Single-node deployment baseline (documented; cluster/replication deferred)
//! Start ONE single-node qdrant (the spike's `qdrant-x86_64-unknown-linux-musl` static binary on
//! WSL2, a local `qdrant/qdrant` container, or a dev-box), point `QDRANT_URL` at its gRPC port
//! (default `http://localhost:6334`; optional `QDRANT_API_KEY`), then run:
//!   QDRANT_URL=http://localhost:6334 cargo run -p contextforge-core \
//!     --example phase29_recall_via_qdrant --features vector-qdrant,embedding-fastembed
//! The collection dim MUST equal the FastEmbed all-MiniLM-L6-v2 dim (384), metric Cosine — the
//! ensure-create decision (`decide_ensure`) preserves dim/metric or errors visibly (never silently
//! drops data). Cluster / replication factor / sharding / deployment topology are deferred:
//! `[SPEC-DEFER:phase-future.qdrant-deployment-topology]`.
//!
//! Default build (no `vector-qdrant` or no `embedding-fastembed`) compiles this as a no-op (no new
//! vector dependency), so `cargo build --workspace` / `cargo test --workspace` are unaffected.

#[cfg(not(all(feature = "vector-qdrant", feature = "embedding-fastembed")))]
fn main() {
    eprintln!(
        "phase29_recall_via_qdrant: requires --features vector-qdrant,embedding-fastembed \
         (live qdrant KNN + real all-MiniLM-L6-v2). Default build no-op — nothing to do."
    );
}

#[cfg(all(feature = "vector-qdrant", feature = "embedding-fastembed"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use contextforge_core::chunker::ChunkPolicy;
    use contextforge_core::embedding::{EmbeddingProvider, FastEmbedProvider};
    use contextforge_core::indexer::IndexSession;
    use contextforge_core::retriever::vector::qdrant::{QdrantBackend, QdrantConnConfig, QdrantHealth};
    use contextforge_core::retriever::Retriever;
    use contextforge_core::scanner::{default_denylist, ScanOptions};

    // Honest-defer guard (ADR-013): construct the backend from the environment, then probe health.
    // No live server → explain + exit 0, never a fabricated recall number.
    let conn = QdrantConnConfig::from_env();
    let backend = Arc::new(QdrantBackend::connect(&conn)?);
    if backend.health() == QdrantHealth::Unreachable {
        eprintln!(
            "phase29_recall_via_qdrant: no live qdrant at {} (health=Unreachable); honest-defer per \
             ADR-013 — start a single-node qdrant and set QDRANT_URL to measure real recall. Exiting 0.",
            conn.url
        );
        return Ok(());
    }
    eprintln!("phase29_recall_via_qdrant: qdrant health=Ready at {} — running live KNN recall.", conn.url);

    // 6 golden categories — same expected files as task-19.5 / task-20.2 (production-pipeline parity).
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
    let work = std::env::temp_dir().join(format!("cf-phase29-recall-{}-{nanos}", std::process::id()));
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

    // Index via the production pipeline (real scanner + chunker). Use a unique qdrant collection name
    // so ensure-create reuses/creates a dim-384 Cosine collection without colliding with spike data.
    let coll = "phase29-recall-via-qdrant";
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

    // Wire the real embedder + the LIVE qdrant backend onto a production Retriever, build the
    // on-demand semantic index from the collection's own chunks (ensure-create + upsert happen inside
    // index_chunks_semantic), then run search_semantic — exactly what core/src/server.rs does.
    let provider = FastEmbedProvider::new();
    let provider_name = provider.name().to_string();
    let provider_dim = provider.dim();
    let embedder: Arc<dyn EmbeddingProvider> = Arc::new(provider);
    let retr = Retriever::open(&data, coll)?
        .with_embedder(embedder)
        .with_vector_searcher(backend.clone());
    let items = retr.enumerate_chunks()?;
    eprintln!(
        "indexed {} production chunks; embedding via {provider_name} (dim {provider_dim}); upserting to live qdrant...",
        items.len()
    );
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

    println!("=== task-29.2 REAL SemanticRecall via LIVE qdrant KNN (provider={provider_name}, dim={provider_dim}) ===");
    println!("production_chunks={} queries={nq} backend=QdrantBackend(live, url={})", items.len(), conn.url);
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
