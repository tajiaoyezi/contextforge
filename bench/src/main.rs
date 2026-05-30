//! `spike` CLI — run the vector backend measurement harness and print/write a report.
//!
//! usage: spike --backend <name> --n <N> --dim <D> --seed <S> [--m <M>] [--dogfood <path>] [--out <md>]

use std::path::PathBuf;
use std::process::exit;

use contextforge_bench::backends::{known_backends, run_named};
use contextforge_bench::corpus::{gen_queries, gen_synthetic, load_dogfood};
use contextforge_bench::runner::render_evidence_md;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut backend = "noop".to_string();
    let mut n = 1000usize;
    let mut dim = 32usize;
    let mut seed = 1u64;
    let mut m = 0usize; // 0 → default n/10
    let mut dogfood: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--backend" => {
                i += 1;
                backend = args.get(i).cloned().unwrap_or(backend);
            }
            "--n" => {
                i += 1;
                n = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(n);
            }
            "--dim" => {
                i += 1;
                dim = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(dim);
            }
            "--seed" => {
                i += 1;
                seed = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(seed);
            }
            "--m" => {
                i += 1;
                m = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(m);
            }
            "--dogfood" => {
                i += 1;
                dogfood = args.get(i).map(PathBuf::from);
            }
            "--out" => {
                i += 1;
                out = args.get(i).map(PathBuf::from);
            }
            "--help" | "-h" => {
                eprintln!(
                    "usage: spike --backend <name> --n <N> --dim <D> --seed <S> [--m <M>] [--dogfood <path>] [--out <md>]\nknown backends: {:?}",
                    known_backends()
                );
                return;
            }
            other => {
                eprintln!("unknown arg: {other}");
                exit(2);
            }
        }
        i += 1;
    }
    if m == 0 {
        m = (n / 10).max(1);
    }

    let corpus = match &dogfood {
        Some(p) => match load_dogfood(p) {
            Ok(c) => {
                dim = c.first().map(|x| x.embedding.len()).unwrap_or(dim);
                c
            }
            Err(e) => {
                eprintln!("dogfood load error: {e}");
                exit(1);
            }
        },
        None => gen_synthetic(seed, n, dim),
    };
    let queries = gen_queries(seed, &corpus, m, dim);

    match run_named(&backend, &corpus, &queries, dim) {
        Ok(Some(report)) => {
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
            if let Some(o) = out {
                let md = render_evidence_md(&report);
                if let Err(e) = std::fs::write(&o, md) {
                    eprintln!("write evidence error: {e}");
                    exit(1);
                }
                eprintln!("evidence written: {}", o.display());
            }
        }
        Ok(None) => {
            eprintln!("unknown backend: {backend} (known: {:?})", known_backends());
            exit(1);
        }
        Err(e) => {
            eprintln!("bench error: {e}");
            exit(1);
        }
    }
}
