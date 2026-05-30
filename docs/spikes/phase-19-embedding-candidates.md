# Phase 19 spike — embedding provider candidates

> task-19.1: evaluate real local embedding providers for ContextForge semantic retrieval. Chosen:
> **fastembed-rs**. Measured on the same machine as Phase 18 (WSL2 Ubuntu 26.04 + Windows 11 MSVC).

## Candidate matrix

| candidate | Linux x86_64 build | Windows MSVC build | model acquisition | API shape | runtime | verdict |
|---|---|---|---|---|---|---|
| **fastembed-rs 4.9.1** | ✅ **21s** (rustls) | ✅ **1m11s** | `hf-hub` lazy download of `all-MiniLM-L6-v2` (~80 MB) on first embed; `ort-download-binaries` fetches the ONNX runtime | `TextEmbedding::try_new(InitOptions::new(EmbeddingModel::AllMiniLML6V2))` → `embed(Vec<String>, None)` → `Vec<Vec<f32>>` (dim **384**) | ort 2.0 / ONNX, sync | **CHOSEN** |
| candle 0.x | builds (pure-Rust ML; not adopted) | expected (pure Rust + optional CUDA) | HF hub download (safetensors + tokenizer, manual) | manual: load model + tokenizer + forward pass | candle tensors | not chosen — significantly more wiring than fastembed for the same MiniLM model |
| ort (direct) | ✅ (fastembed wraps it) | ✅ | manual ONNX model + tokenizer wiring | low-level `Session` | ONNX | not chosen — fastembed wraps ort with batteries-included model management |

## The OpenSSL blocker and the fix (key finding)

fastembed's **default** features pull `hf-hub-native-tls` → `openssl-sys`, which needs system
`pkg-config` + OpenSSL. On a box without them (and without passwordless sudo) the build fails:

```
openssl-sys: ... requires the `pkg-config` utility to find OpenSSL ... could not be found
```

Fix: depend with `default-features = false, features = ["ort-download-binaries", "hf-hub-rustls-tls"]`.
This swaps native-tls for **rustls** (pure Rust TLS, no OpenSSL/pkg-config) and keeps the ONNX runtime
auto-download. With that, fastembed builds cleanly on **both** Linux x86_64 and **Windows MSVC**.

## Decision: fastembed-rs (rustls)

- **Cross-platform — both P0 platforms build.** Unlike sqlite-vec (Windows MSVC build-blocked,
  `docs/spikes/phase-18-sqlite-vec.md`), fastembed builds on Linux **and** Windows MSVC. The dev box
  (Windows) and the Linux container both get the real provider — dev/prod parity for embeddings.
- **Purpose-built + minimal wiring.** Batteries-included model management (download, cache, tokenize,
  batch) vs hand-wiring candle/ort tensors + tokenizers.
- **Local-first (PRD §Anti-metrics).** Model runs locally; only the first-use model fetch hits the
  network, then it is cached. No remote inference API.
- **384-dim** (`all-MiniLM-L6-v2`) — a small, fast, widely-used sentence embedding model. Matches the
  `DeterministicEmbeddingProvider` default dim so the retriever can swap deterministic ↔ real without
  a dimension change.

## Wiring in this task

- `embedding-fastembed` feature, **off by default** — the default build compiles only the model-free
  `DeterministicEmbeddingProvider` (0 new dep, see `docs/spikes/phase-19-embedding-fastembed.md`).
- Real model is lazy-loaded on first `embed` (a `OnceLock`-guarded `TextEmbedding::try_new`), so
  constructing `FastEmbedProvider` never downloads anything.

## Open questions / follow-ups

- CI runs the **deterministic** provider (no network/model). The real provider's recall is measured
  on a dev/Linux box with network — that is task-19.5.
- Model cache location + offline/air-gapped story [SPEC-DEFER:phase-future.embedding-cache].
- Remote providers (OpenAI / Cohere) [SPEC-DEFER:phase-future.embedding-provider-remote].
