//! task-49.4: large-corpus recall spike — validates whether v1.0's hybrid recall@5/@10=1.0 (measured
//! over the 16-30 question author-curated golden in phase-19/21) holds at scale (~120 questions /
//! ~500-1000 chunks). Fixture-driven: reads `test/fixtures/eval/golden-retrieval.jsonl` (the task-49.1
//! 6-category dataset), NOT hardcoded queries.
//!
//! Corpus = the deduped golden expected files + distractors (real ContextForge source), indexed via
//! the production `IndexSession` with the production-default `code_cjk` tokenizer (ADR-046). Measures
//! file-level recall@5/@10 + top-1 + MRR per category + overall for:
//!   - baseline BM25          (`Retriever::search`)            — default build, always runs
//!   - hybrid RRF fusion       (`Retriever::search_hybrid`)     — needs `embedding-fastembed`
//!   - reranked cross-encoder  (`search_hybrid` + reranker)     — needs `reranker-fastembed` too
//!
//! ADR-013: every number is a real model run — no synthetic/fabricated figures. The BM25 baseline
//! runs in the default build (no ONNX); the semantic/hybrid/reranked passes need the real embedding
//! model. This is an ad-hoc spike, NOT a promoted Rust-native eval runner
//! (`[SPEC-DEFER:phase-future.rust-native-eval-runner]`; [SPEC-OWNER:task-49.4] this example is a
//! compile anchor mirroring phase19/21/24 — not an unimplemented placeholder).
//!
//! Run:
//!   cargo run -p contextforge-core --example phase49_large_corpus_recall                            # BM25 only
//!   cargo run -p contextforge-core --example phase49_large_corpus_recall --features embedding-fastembed,reranker-fastembed
//!
//! Results → docs/spikes/phase49-large-corpus-recall.md (honest report: does recall hold at scale?).

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use contextforge_core::chunker::ChunkPolicy;
use contextforge_core::indexer::IndexSession;
use contextforge_core::retriever::{Retriever, RetrieverConfig, SearchOptions};
use contextforge_core::scanner::{default_denylist, ScanOptions};

#[derive(serde::Deserialize)]
struct GoldenQ {
    query: String,
    expected_file_path: String,
    category: String,
}

/// path → unique alnum stem carried in the temp file name (and thus in file_path for matching).
fn sanitize(path: &str) -> String {
    path.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn ext(rel: &str) -> String {
    PathBuf::from(rel)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("txt")
        .to_string()
}

#[derive(Default, Clone)]
struct CatMetrics {
    n: usize,
    h5: usize,
    h10: usize,
    top1: usize,
    mrr: f64,
}

impl CatMetrics {
    fn record(&mut self, rank: Option<usize>) {
        self.n += 1;
        if let Some(r) = rank {
            if r == 1 {
                self.top1 += 1;
            }
            if r <= 5 {
                self.h5 += 1;
            }
            if r <= 10 {
                self.h10 += 1;
            }
            self.mrr += 1.0 / r as f64;
        }
    }
}

/// Measure file-level recall@5/@10 + top-1 + MRR over all queries, grouped by category.
fn measure<F>(queries: &[GoldenQ], mut search: F) -> std::collections::BTreeMap<String, CatMetrics>
where
    F: FnMut(&GoldenQ) -> Result<Vec<contextforge_core::retriever::SearchResult>, Box<dyn std::error::Error>>,
{
    let mut by_cat: std::collections::BTreeMap<String, CatMetrics> = std::collections::BTreeMap::new();
    for q in queries {
        let stem = sanitize(&q.expected_file_path);
        let hits = match search(q) {
            Ok(h) => h,
            Err(_) => {
                by_cat.entry(q.category.clone()).or_default().record(None);
                continue;
            }
        };
        let mut matched: Option<usize> = None;
        for (rank, h) in hits.iter().enumerate() {
            if h.file_path.contains(&stem) {
                matched = Some(rank + 1);
                break;
            }
        }
        by_cat.entry(q.category.clone()).or_default().record(matched);
    }
    by_cat
}

fn print_report(label: &str, by_cat: &std::collections::BTreeMap<String, CatMetrics>) {
    let mut overall = CatMetrics::default();
    for (cat, m) in by_cat {
        let nf = m.n as f64;
        if nf > 0.0 {
            println!(
                "  {label:22} [{:<20}] n={:>2} recall@5={:.4} recall@10={:.4} top1={:.4} mrr={:.4}",
                cat,
                m.n,
                m.h5 as f64 / nf,
                m.h10 as f64 / nf,
                m.top1 as f64 / nf,
                m.mrr / nf
            );
        }
        overall.n += m.n;
        overall.h5 += m.h5;
        overall.h10 += m.h10;
        overall.top1 += m.top1;
        overall.mrr += m.mrr;
    }
    let nf = overall.n as f64;
    if nf > 0.0 {
        let r10 = overall.h10 as f64 / nf;
        println!(
            "  {label:22} [OVERALL             ] n={:>2} recall@5={:.4} recall@10={:.4} top1={:.4} mrr={:.4} gate@10(>=0.85)={}",
            overall.n,
            overall.h5 as f64 / nf,
            r10,
            overall.top1 as f64 / nf,
            overall.mrr / nf,
            if r10 >= 0.85 { "pass" } else { "fail" }
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");

    // Fixture-driven: load golden-retrieval.jsonl (task-49.1, ~120 questions / 6 categories).
    let golden_path = repo_root.join("test/fixtures/eval/golden-retrieval.jsonl");
    let raw = fs::read_to_string(&golden_path)
        .map_err(|e| format!("read {}: {e}", golden_path.display()))?;
    let mut queries: Vec<GoldenQ> = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        queries.push(serde_json::from_str(line)?);
    }
    if queries.is_empty() {
        return Err("golden-retrieval.jsonl has no questions".into());
    }

    // Build corpus from deduped expected files + distractors (real ContextForge content).
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let work = std::env::temp_dir().join(format!("cf-phase49-recall-{}-{nanos}", std::process::id()));
    let src = work.join("src");
    let data = work.join("data");
    fs::create_dir_all(&src)?;
    let mut expected_files: BTreeSet<String> = BTreeSet::new();
    for q in &queries {
        if expected_files.insert(q.expected_file_path.clone()) {
            let body = fs::read_to_string(repo_root.join(&q.expected_file_path))
                .map_err(|e| format!("read expected {}: {e}", q.expected_file_path))?;
            fs::write(
                src.join(format!("{}.{}", sanitize(&q.expected_file_path), ext(&q.expected_file_path))),
                body,
            )?;
        }
    }
    let distractors = [
        "core/src/server.rs",
        "internal/daemon/daemon.go",
        "internal/cli/eval.go",
        "core/src/retriever/vector/brute_force.rs",
        "internal/eval/eval.go",
        "core/src/health.rs",
        "internal/consoleapi/memstore.go",
        "core/src/chunker/mod.rs",
    ];
    for (i, rel) in distractors.iter().enumerate() {
        if let Ok(body) = fs::read_to_string(repo_root.join(rel)) {
            let _ = fs::write(src.join(format!("distractor_{i}.{}", ext(rel))), body);
        }
    }

    let scan_opts = ScanOptions {
        denylist: default_denylist(),
        allowlist: Vec::new(),
        allow_denylist_override: false,
        dry_run: false,
        max_file_bytes: 10 * 1024 * 1024,
    };

    // Index with the production-default code_cjk tokenizer (ADR-046).
    let coll = "phase49-large-corpus";
    {
        let mut s = IndexSession::open_with_tokenizer(&data, coll, "code_cjk")?;
        s.index_path(&src, &scan_opts, &ChunkPolicy::default(), vec![])?;
        s.commit()?;
    }

    println!("=== task-49.4 large-corpus recall spike (ADR-013 real run) ===");
    println!(
        "corpus_files={} queries={} tokenizer=code_cjk(production-default)",
        expected_files.len() + distractors.len(),
        queries.len()
    );

    let cfg = RetrieverConfig {
        tokenizer: "code_cjk".to_string(),
        ..Default::default()
    };

    // BM25 baseline (default build, always runs).
    {
        let retr = Retriever::open_with_config(&data, coll, cfg.clone())?;
        let bm25 = measure(&queries, |q| {
            Ok(retr.search(&SearchOptions {
                query: q.query.clone(),
                top_k: 10,
                ..Default::default()
            })?)
        });
        print_report("baseline-bm25", &bm25);
    }

    // Semantic + hybrid + reranked passes (feature-gated; need real embedding model).
    #[cfg(feature = "embedding-fastembed")]
    {
        use std::sync::Arc;
        use contextforge_core::embedding::{EmbeddingProvider, FastEmbedProvider};
        use contextforge_core::retriever::vector::BruteForceVectorBackend;

        let provider = FastEmbedProvider::new();
        let provider_name = provider.name().to_string();
        let provider_dim = provider.dim();
        let embedder: Arc<dyn EmbeddingProvider> = Arc::new(provider);
        let backend = Arc::new(BruteForceVectorBackend::new());
        let retr = Retriever::open_with_config(&data, coll, cfg.clone())?
            .with_embedder(embedder.clone())
            .with_vector_searcher(backend.clone());
        let items = retr.enumerate_chunks()?;
        eprintln!(
            "indexed {} chunks; embedding via {} (dim {})...",
            items.len(),
            provider_name,
            provider_dim
        );
        retr.index_chunks_semantic(backend.as_ref(), &items)?;

        // hybrid RRF fusion (ADR-025).
        let hybrid = measure(&queries, |q| Ok(retr.search_hybrid(&q.query, 10)?));
        print_report("hybrid-rrf", &hybrid);

        // reranked cross-encoder (ADR-026; needs reranker-fastembed feature).
        #[cfg(feature = "reranker-fastembed")]
        {
            use contextforge_core::rerank::CrossEncoderReranker;
            let reranked_retr = Retriever::open_with_config(&data, coll, cfg.clone())?
                .with_embedder(embedder.clone())
                .with_vector_searcher(backend.clone())
                .with_reranker(Arc::new(CrossEncoderReranker::new()));
            let reranked = measure(&queries, |q| Ok(reranked_retr.search_hybrid(&q.query, 10)?));
            print_report("reranked-cross-encoder", &reranked);
            println!("rerank_provider=fastembed-bge-reranker-base (real cross-encoder, ADR-013)");
        }
        #[cfg(not(feature = "reranker-fastembed"))]
        println!("  reranked-cross-encoder   SKIPPED (build with --features embedding-fastembed,reranker-fastembed)");
    }
    #[cfg(not(feature = "embedding-fastembed"))]
    println!("  hybrid-rrf / reranked    SKIPPED (build with --features embedding-fastembed[,reranker-fastembed])");

    let _ = fs::remove_dir_all(&work);
    Ok(())
}
