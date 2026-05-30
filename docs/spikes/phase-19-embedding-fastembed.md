# Phase 19 spike — `fastembed` (chosen embedding provider)

> task-19.1: real embedding provider via **fastembed-rs 4.9.1** (ort 2.0 / ONNX, `all-MiniLM-L6-v2`,
> dim **384**), behind the `embedding-fastembed` feature. Candidate evaluation:
> `docs/spikes/phase-19-embedding-candidates.md`.

## Build evidence (both P0 platforms)

| platform | command | result |
|---|---|---|
| Linux x86_64 (WSL2 Ubuntu 26.04, rustc 1.96) | `cargo build -p contextforge-core --features embedding-fastembed` | ✅ **30.4s** (standalone probe: 21s) |
| Windows 11 MSVC | `cargo build -p contextforge-core --features embedding-fastembed` | ✅ **1m11s** |
| default (no feature, both platforms) | `cargo build -p contextforge-core` | ✅ **8.6s**, fastembed **not** compiled (0 new dep) |

Built with `default-features = false, features = ["ort-download-binaries", "hf-hub-rustls-tls"]` —
**rustls, not OpenSSL**, so no system `pkg-config`/OpenSSL is needed. Unlike sqlite-vec (Windows MSVC
build-blocked, `docs/spikes/phase-18-sqlite-vec.md`), fastembed builds on **both** Linux and Windows
MSVC — the real provider has dev/prod platform parity.

## Real embedding evidence (in-repo provider)

`FastEmbedProvider` (`core/src/embedding/fastembed_provider.rs`) — model lazy-loaded on first `embed`
via a `OnceLock`-guarded `TextEmbedding::try_new`. In-repo test:

```
cargo test -p contextforge-core --features embedding-fastembed --lib -- --ignored
  test embedding::tests::test_real_fastembed_embed_dim384 ... ok
  test result: ok. 1 passed; 0 failed; ...
```

- `dim()` = **384**; `name()` = `fastembed-all-minilm-l6-v2`.
- `embed(["where is the config loader", "how does the daemon restart"])` → 2 vectors of length 384,
  non-zero. Standalone-probe sample (v0[..4]): `[-0.0578, 0.0225, -0.0749, 0.0250]`.
- Model `all-MiniLM-L6-v2` (~80 MB) downloaded once via `hf-hub` on first use, then cached; the test
  reruns in 0.74s against the cache. ONNX runtime fetched by `ort-download-binaries`.

The test is `#[ignore]` (it needs network + the model on first run), so it is **not** part of the
default `cargo test --workspace` / CI — CI exercises the model-free `DeterministicEmbeddingProvider`.

## Provider design (in-repo)

- `EmbeddingProvider` trait (`embed` / `dim` / `name`) + `EmbeddingError` (`#[non_exhaustive]`),
  mirroring the task-18.1 vector trait style. Object-safe (`Arc<dyn EmbeddingProvider>`) → the
  retriever swaps deterministic ↔ real behind one seam (task-19.2).
- `DeterministicEmbeddingProvider` (default, no model): `Sha256(text)` → splitmix64 → unit-normalized
  dim-384 vector. Same text → byte-identical vector (reproducible). **No semantic structure** — it
  drives wiring/smoke/CI deterministically, *not* real recall (that is task-19.5, real provider).
- `FastEmbedProvider` (feature `embedding-fastembed`): real model, lazy-loaded.

## Verdict

fastembed-rs is **viable on both P0 platforms** with real dim-384 embeddings and a rustls-clean build.
It is the chosen real provider; `embedding-fastembed` ships off by default (BM25-only + deterministic
default unaffected). Real-distribution recall (feeding the ADR-023 ratify) is task-19.5.

## Open questions / follow-ups

- recall on the dogfood corpus with these real embeddings — task-19.5.
- model cache path / offline story [SPEC-DEFER:phase-future.embedding-cache].
- the default retriever wiring of `EmbeddingProvider` — task-19.2.
