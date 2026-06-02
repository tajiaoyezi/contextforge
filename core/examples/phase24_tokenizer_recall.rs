//! task-24.3: real before/after recall delta for the task-24.1 code/CJK tokenizer,
//! measured over the task-24.2 `test/fixtures/eval/golden-semantic.jsonl` dataset through
//! the production `IndexSession` + `Retriever::search` BM25 path (ADR-013 — no fabricated numbers).
//!
//!   before = default analyzer (`IndexSession::open`)
//!   after  = opt-in code/CJK analyzer (`IndexSession::open_with_tokenizer(.., "code_cjk")`)
//!
//! Both index the SAME corpus (the deduped golden expected files + a few distractors). For each
//! golden query we `Retriever::search` top-10 and record the rank of the first hit whose file_path
//! carries the query's expected file → file-level Strong-hit@5/@10 + top-1 + MRR, before vs after.
//!
//! This is a default-build BM25 lexical harness (no feature gate, no ONNX/embedding) so it runs in
//! CI and any environment. It is an ad-hoc spike measurement, NOT a promoted Rust-native eval runner
//! (see docs/spikes/phase-24-tokenizer-recall.md §runner — `[SPEC-DEFER:phase-future.rust-native-eval-runner]`).
//!
//!   cargo run -p contextforge-core --example phase24_tokenizer_recall

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
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

/// path → unique alnum stem carried in the temp file name (and thus in file_path).
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

struct Metrics {
    recall5: f64,
    recall10: f64,
    top1: f64,
    mrr: f64,
    ranks: Vec<Option<usize>>,
}

fn run(
    data_dir: &Path,
    coll: &str,
    cfg: RetrieverConfig,
    queries: &[GoldenQ],
) -> Result<Metrics, Box<dyn std::error::Error>> {
    let retr = Retriever::open_with_config(data_dir, coll, cfg)?;
    let (mut h5, mut h10, mut top1) = (0usize, 0usize, 0usize);
    let mut mrr = 0.0f64;
    let mut ranks = Vec::with_capacity(queries.len());
    for q in queries {
        let stem = sanitize(&q.expected_file_path);
        let res = retr.search(&SearchOptions {
            query: q.query.clone(),
            top_k: 10,
            ..Default::default()
        })?;
        let mut matched: Option<usize> = None;
        for (rank, r) in res.iter().enumerate() {
            if r.file_path.contains(&stem) {
                matched = Some(rank + 1);
                break;
            }
        }
        if let Some(r) = matched {
            if r == 1 {
                top1 += 1;
            }
            if r <= 5 {
                h5 += 1;
            }
            if r <= 10 {
                h10 += 1;
            }
            mrr += 1.0 / r as f64;
        }
        ranks.push(matched);
    }
    let n = queries.len() as f64;
    Ok(Metrics {
        recall5: h5 as f64 / n,
        recall10: h10 as f64 / n,
        top1: top1 as f64 / n,
        mrr: mrr / n,
        ranks,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");

    // Load the task-24.2 golden-semantic dataset (code-symbol + CJK queries).
    let golden_path = repo_root.join("test/fixtures/eval/golden-semantic.jsonl");
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
        return Err("golden-semantic.jsonl has no questions".into());
    }

    // Write deduped expected files + distractors into a temp source tree (real ContextForge content).
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let work = std::env::temp_dir().join(format!("cf-phase24-tokrecall-{}-{nanos}", std::process::id()));
    let src = work.join("src");
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
    let distractors = ["core/src/server.rs", "internal/daemon/daemon.go", "internal/cli/eval.go"];
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

    // before: default analyzer.
    let data_before = work.join("before");
    {
        let mut s = IndexSession::open(&data_before, "tok")?;
        s.index_path(&src, &scan_opts, &ChunkPolicy::default(), vec![])?;
        s.commit()?;
    }
    // after: opt-in code/CJK analyzer (== indexer::CODE_CJK_TOKENIZER).
    let data_after = work.join("after");
    {
        let mut s = IndexSession::open_with_tokenizer(&data_after, "tok", "code_cjk")?;
        s.index_path(&src, &scan_opts, &ChunkPolicy::default(), vec![])?;
        s.commit()?;
    }

    let before = run(&data_before, "tok", RetrieverConfig::default(), &queries)?;
    let cfg_after = RetrieverConfig {
        tokenizer: "code_cjk".to_string(),
        ..Default::default()
    };
    let after = run(&data_after, "tok", cfg_after, &queries)?;

    println!("=== task-24.3 REAL tokenizer before/after recall (BM25 file-level over task-24.2 golden) ===");
    println!(
        "corpus_files={} queries={} (code-symbol + cjk)",
        expected_files.len() + distractors.len(),
        queries.len()
    );
    println!(
        "before(default):  recall@5={:.4} recall@10={:.4} top1={:.4} mrr={:.4}",
        before.recall5, before.recall10, before.top1, before.mrr
    );
    println!(
        "after(code_cjk):  recall@5={:.4} recall@10={:.4} top1={:.4} mrr={:.4}",
        after.recall5, after.recall10, after.top1, after.mrr
    );
    println!(
        "delta:            recall@5={:+.4} recall@10={:+.4} top1={:+.4} mrr={:+.4}",
        after.recall5 - before.recall5,
        after.recall10 - before.recall10,
        after.top1 - before.top1,
        after.mrr - before.mrr
    );
    println!("--- per-query rank of expected file ('-' = miss within top-10) ---");
    let fmt = |r: Option<usize>| r.map(|x| x.to_string()).unwrap_or_else(|| "-".to_string());
    for (i, q) in queries.iter().enumerate() {
        println!(
            "  [{:<11}] {:<24} before={:>2} after={:>2}  ({})",
            q.category,
            q.query,
            fmt(before.ranks[i]),
            fmt(after.ranks[i]),
            q.expected_file_path
        );
    }
    let _ = fs::remove_dir_all(&work);
    Ok(())
}
