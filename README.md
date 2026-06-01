# ContextForge

ContextForge is a local-first context indexing and retrieval tool for agent memory, rules, source files, logs, and project notes.

It ships as two binaries (ADR-001):

- `contextforge`: Go control-plane CLI, REST/MCP adapter, Console Contract v1 REST surface (`console-api-serve`, v0.3+), export and eval entrypoint.
- `contextforge-core`: Rust data-plane daemon for scan, parse, chunk, index, and retrieval.

## What's new in v0.20.0

📌 **v0.20.0 memory-ops-hardening** — hardens the Memory pin / lifecycle semantics from Phase 13 / 17, delivering the three ADR-022 deferred markers. **pin-actor + pinned-at-timestamp** become first-class `MemoryItem` fields; **Pin/Unpin** is split explicitly (vs the `Pin{bool pin}` toggle) and a **hard-delete** strategy (physical removal, X-Confirm gated) is added; **is_pinned** can be **backfilled from the audit log**. The **default build stays 0-new-dependency, 0-network**; all proto changes are add-only and the existing 5 Memory RPC + `Pin` toggle do not regress.

- **pin-actor + pinned-at-timestamp** (task-27.1): add-only `MemoryItem.pinned_by` (proto field 11) + `pinned_at_unix` (field 12) + migration `0017` (guarded `ALTER ADD COLUMN`). `set_pinned_with_actor` writes the actor + timestamp on pin, clears on unpin; `pinned_at_unix` is independent of `updated_at_unix`. Audit never recorded the calling actor, so these become first-class fields. proto-freeze guard passes. TEST-27.1.* (store 15/15 + data_plane 14/14).
- **Pin/Unpin split + hard-delete** (task-27.2): add-only `Unpin` RPC (explicit + idempotent, beside the `Pin{bool pin}` toggle) + `HardDelete` RPC (`DELETE FROM memory_items` — physical removal, get-by-id returns None afterwards, vs soft-delete's status flip). console-api `POST /v1/memory/{id}/unpin` (204) + `POST /v1/memory/{id}/hard-delete` (confirmMiddleware-gated: 412 without X-Confirm, 204 with, then GET → 404). New `memory.hard_delete` event_type + `MemoryHardDelete` audit op. TEST-27.2.*.
- **is_pinned audit backfill** (task-27.3): `reconcile_is_pinned_from_audit` replays `memory_pin`/`memory_unpin` audit events (last wins) to rebuild legacy items' `is_pinned`, opt-in one-time reconcile (corrects only `is_pinned`, does not fabricate actor/timestamp). **ADR-032 → Accepted**; **ADR-022** add-only Amendment (three markers). **ADR-014 cross-validation gate — 18th activation**.

```bash
# pin-actor/timestamp + hard-delete + is_pinned backfill (default build, 0 new dep)
cargo test -p contextforge-core --lib memory::store
# console-api unpin / hard-delete X-Confirm
go test ./internal/consoleapi/... -run 'Memory'
```

详 `RELEASE_NOTES.md` v0.20.0 段 + [Phase 27 spec](docs/specs/phases/phase-27-memory-ops-hardening.md) + [ADR-032](docs/decisions/adr-032-memory-ops-hardening.md)。

## What's new in v0.19.0

🔭 **v0.19.0 observability-hardening** — hardens the two observability signal paths landed in Phase 16. **TraceStore** gains FTS5 content search + periodic VACUUM/prune; **events** gain an SSE real-time push endpoint (add-only beside the long-poll) + replay of missed memory state-op events from the persistent audit log; the **EventBus** gains capacity / partition / drain-timeout config. The **default build stays 0-new-dependency, 0-network** (FTS5/VACUUM reuse rusqlite bundled, SSE uses Go stdlib `http.Flusher`, replay reads the existing `audit_log`).

- **TraceStore FTS5 + VACUUM** (task-26.1): `search_fts(query_text, limit)` content-searches persisted traces (FTS5 shadow table, quoted-phrase MATCH); `vacuum()` + `prune_older_than(cutoff)` reclaim space so `search_traces.db` no longer grows unbounded. Old 0015-only DBs get the FTS table created + backfilled on boot. Existing `put`/`get`/`list`/`load_warm` signatures unchanged; **0 new dependency** (rusqlite bundled). TEST-26.1.1-5 (10/10).
- **events SSE push + audit replay** (task-26.2): `GET /v1/observability/events/stream` (`text/event-stream` + `http.Flusher`) pushes events in real time, add-only beside the existing long-poll endpoint. `?since_ts=` replays missed memory state-op events from the audit log (`id ASC`, ADR-021 D3 mapping) then splices the live stream, deduping the boundary by event_id. SSE frame contract + replay order are **deterministic** (no wall-clock). Real daemon-served SSE end-to-end **deferred** (`[SPEC-DEFER:phase-future.sse-live-server-e2e]`, CI has no running daemon).
- **event-bus config** (task-26.3): `CF_EVENT_BUS_CAPACITY` (default 1000) + `CF_EVENT_BUS_PARTITION` (default off — `memory.*` / `indexing.*` on independent channels) + `CONSOLE_EVENTS_DRAIN_TIMEOUT` (default 100ms). Conservative defaults keep the task-11.4 behavior unchanged. **ADR-031 → Accepted**; **ADR-021** add-only Amendment (events-replay + event-bus config). **ADR-014 cross-validation gate — 17th activation**.

```bash
# trace FTS5 content search + VACUUM/prune (default build, 0 new dep)
cargo test -p contextforge-core --lib data_plane::search_persist
# events SSE replay query face + event-bus config
cargo test -p contextforge-core --lib data_plane::events
go test ./internal/consoleapi/... -run 'EventsStream|DrainTimeout'
```

详 `RELEASE_NOTES.md` v0.19.0 段 + [Phase 26 spec](docs/specs/phases/phase-26-observability-hardening.md) + [ADR-031](docs/decisions/adr-031-observability-hardening.md)。

## What's new in v0.18.0

🧭 **v0.18.0 production-vector-backend** — pushes the two production-scale ANN backends ADR-023 tiered (**qdrant** for hosted/scale-out, **lancedb** for embedded-columnar) from the Phase-18 spike state toward production, and ships a **production backend selection matrix**. The **default build stays 0-vector-dependency, BM25-only baseline** (qdrant/lancedb are feature-gated, default unchanged).

- **Feature-gated, default unchanged.** Both production backends are off by default; the default build is 0-vector-dep BM25 baseline. This is a backend **lifecycle / buildability-layer** release with **no recall numbers** — qdrant's lifecycle is contract-verified without a live server, lancedb is buildability-verified on the dev box.
- **qdrant server lifecycle** (task-25.1): `QdrantConnConfig` (url/timeout/api-key/TLS) + `validate()` + `health()` probe (unreachable when no server, no panic) + `decide_ensure` collection ensure-create (reuse-if-matching / create / error-on-mismatch, replacing the spike's blind drop+create). Contract-testable **without a live server** (TEST-25.1.1-4); real KNN over live qdrant **deferred** (`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`, CI has no server).
- **lancedb buildability** 🟢 (task-25.2): `cargo build --features vector-lancedb` **builds on `x86_64-pc-windows-msvc`** (protoc via the in-repo `protoc-bin-vendored` binary, **0 new dependency**), narrowing the Phase-18 protoc-prerequisite concern (protoc must still be explicitly provided). Adds an index-tuning param-validation layer (`LanceIndexTuning::validate`: IVF_PQ/HNSW + compaction threshold). Real ANN index perf **deferred** (`[SPEC-DEFER:phase-future.lancedb-index-tuning]`).
- **Production backend selection matrix** (task-25.3): corpus-size × deployment-shape → hnsw (dev/small) / sqlite-vec (single-box embedded) / lancedb (large-corpus columnar) / qdrant (hosted scale-out), each with its caveat (live-server dependency / protoc prerequisite / platform limit). **ADR-030 → Accepted**. **ADR-014 cross-validation gate — 16th activation**.

```bash
# qdrant lifecycle contract layer (feature; no live server needed)
cargo test -p contextforge-core --features vector-qdrant retriever::vector::qdrant
# lancedb real buildability 🟢 + index-tuning param validation (needs protoc via PROTOC env)
cargo build --features vector-lancedb -p contextforge-core
```

详 `RELEASE_NOTES.md` v0.18.0 段 + [Phase 25 spec](docs/specs/phases/phase-25-production-vector-backend.md) + [ADR-030](docs/decisions/adr-030-production-vector-backend.md) + [lancedb buildability spike](docs/spikes/phase-25-lancedb-buildability.md)。

## What's new in v0.17.0

🔤 **v0.17.0 code-and-cjk-tokenizer-and-eval-hardening** — adds an **opt-in code/CJK tokenizer** and a **hardened eval ruler**. The `content` field can split `camelCase`/`snake_case`/`dotted.path`/`kebab-case` into sub-tokens (keeping the original token) and tokenize CJK text into bigrams; the eval golden dataset gains an independent validator + code/CJK query cases. The **default build stays 0-new-dep, default-tokenization-unchanged, eval-gate-threshold-unchanged**.

- **Opt-in, default unchanged.** The tokenizer is opt-in via `RetrieverConfig.tokenizer="code_cjk"` (config, **not a feature flag** — pure std, in the default build). Default tokenization is unchanged so existing collections are not silently invalidated; **adopting opt-in requires a re-index** (it changes the inverted terms). Index/query stay symmetric (both register the same analyzer).
- **code/CJK tokenizer** (task-24.1): `camelCase`→`camel`+`case` (+ original), `snake_case`/`dotted.path`/`kebab-case` sub-words, CJK bigram (`配置加载`→`配置`/`置加`/`加载`). **0 new dependency.**
- **eval hardening** (task-24.2): `ValidateGoldenSemantic` (schema well-formedness + duplicate detection + answer coverage, add-only) + `test/fixtures/eval/golden-semantic.jsonl` (code-symbol + CJK queries → real files).
- **Real before/after recall delta** (task-24.3): default **0.9091 → code/CJK 1.0000 (+0.0909)** over the task-24.2 golden (BM25 file-level), driven by a real CJK-bigram win; rust-native-eval-runner **honestly deferred**. **ADR-029 → Accepted**. **ADR-014 cross-validation gate — 15th activation**.

```bash
# opt-in code/CJK tokenizer split + CJK bigram (default build)
cargo test -p contextforge-core --lib indexer::tests::test_24_1
# real before/after recall delta over the golden-semantic dataset
cargo run -p contextforge-core --example phase24_tokenizer_recall
```

详 `RELEASE_NOTES.md` v0.17.0 段 + [Phase 24 spec](docs/specs/phases/phase-24-retrieval-tokenizer-and-eval-hardening.md) + [ADR-029](docs/decisions/adr-029-code-and-cjk-tokenizer-and-eval-hardening.md) + [tokenizer recall spike](docs/spikes/phase-24-tokenizer-recall.md)。

## What's new in v0.16.0

🗄️ **v0.16.0 vector-persistence-and-cross-platform** — makes the feature-gated vector backends **persistent + cross-platform**: **hnsw graph persistence** (`save`/`load` + rebuild-on-load fallback), a **sqlite-vec Windows MSVC** investigation that **resolved the Phase 18 MSVC-build-blocked stop-condition** (real build + run on `x86_64-pc-windows-msvc`), and a **vector incremental-index evaluation** (brute-force / sqlite-vec row-level append). The **default build stays 0-vector-dependency, BM25-baseline**.

- **Local-first, default unchanged.** Persistence/cross-platform live behind `vector-hnsw` / `vector-sqlite` features (ADR-023 D5); the default build is 0-vector-dep BM25 baseline. **No proto change** — `VectorIndexConfig.persistence_path` (existing field) is first consumed this release.
- **hnsw graph persistence** (task-23.1): `HnswBackend::save`/`load` round-trips the index (path B: persist inputs + rebuild-on-load), eliminating the cold-start re-embed. **0 new dependency.**
- **sqlite-vec now builds on Windows MSVC** (task-23.2): real `cargo build --features vector-sqlite` + contract tests pass on `x86_64-pc-windows-msvc` (rustc 1.95.0), resolving the Phase 18 stop-condition. Honest caveat: single dev box, CI doesn't build the feature by default (ADR-013).
- **This is a backend-layer release with no recall numbers**; persisted graphs are not yet wired into the semantic hot path (future release). **ADR-028 vector-persistence-strategy → Accepted** + **ADR-023 add-only Amendment**. **ADR-014 cross-validation gate — 14th activation**.

```bash
# hnsw graph persistence round-trip (feature-gated)
cargo test --features vector-hnsw -p contextforge-core retriever::vector::hnsw
# sqlite-vec — now builds + runs on Windows MSVC (and Linux gcc)
cargo test --features vector-sqlite -p contextforge-core retriever::vector::sqlite_vec
```

详 `RELEASE_NOTES.md` v0.16.0 段 + [Phase 23 spec](docs/specs/phases/phase-23-vector-persistence-and-cross-platform.md) + [ADR-028](docs/decisions/adr-028-vector-persistence-strategy.md) + [sqlite-vec cross-platform spike](docs/spikes/phase-23-sqlite-vec-cross-platform.md)。

## What's new in v0.15.0

🧩 **v0.15.0 embedding-provider-completion** — grows the embedding layer from "hardcoded deterministic default + a single feature-gated fastembed provider" into a **configurable provider layer**: a `select_provider` factory (deterministic / fastembed / remote) with **dim negotiation** (`DimMismatch`, no silent resize), a **content-hash embedding cache** (memory L1 + optional SQLite L2), and a **feature-gated remote provider skeleton** (OpenAI/Cohere HTTP). The **default build stays local, model-free, and 0-network-dep**.

- **Local-first, non-negotiable.** Default is the deterministic identity provider — 0 model / 0 network dependency. `fastembed` (real model) and `remote` (OpenAI/Cohere) are **feature-gated + explicit opt-in** (ADR-004); API keys are read from the environment and never logged.
- **Add-only `[embedding]` config, no breaking bump.** `internal/config` gains an add-only `[embedding]`(provider/dim) section (existing `[remote]`/`[[collections]]` unaffected); `contextforge init` now emits it. No proto change.
- **Embedding cache + remote skeleton verified at the unit/contract layer** (no network in tests). **This is a provider-layer release with no recall numbers** — real remote-network 联调 / keys / recall quality + the real remote health probe are honestly **deferred** (ADR-013; CI has no credentials).
- **ADR-027 embedding-provider-abstraction → Accepted**, ratified on the real non-synthetic verification of D1–D5 (config/factory/dim/cache/contract/local-first). **ADR-014 cross-validation gate — 13th activation**.

```bash
# init now scaffolds an add-only [embedding] section (provider/dim) alongside [remote]
contextforge init --root ~/.contextforge

# remote provider skeleton (feature-gated; contract-tested with fixtures, no real network)
cargo test --features embedding-remote -p contextforge-core embedding::remote_provider
```

(The remote provider is opted into via `[embedding] provider="remote"` + the `embedding-remote` feature + env API key; the default build never pulls a network client or hits the network.)

详 `RELEASE_NOTES.md` v0.15.0 段 + [Phase 22 spec](docs/specs/phases/phase-22-embedding-provider-completion.md) + [ADR-027](docs/decisions/adr-027-embedding-provider-abstraction.md) + [v0.15.0 evidence](docs/releases/v0.15.0-evidence.md)。

## What's new in v0.14.0

🎯 **v0.14.0 retrieval-quality** — adds two **opt-in** ranking-quality enhancements on top of the BM25 + semantic dual paths: **hybrid scoring** (RRF fusion of the word-level + vector scores) and a **reranker pipeline** (deterministic default + feature-gated real cross-encoder). The **default build is unchanged and dependency-free** — still BM25 baseline, 0 new crate.

- **Hybrid scoring (RRF) — opt-in, real dogfood win.** `Retriever::search_hybrid` reciprocal-rank-fuses the BM25 and vector result lists (`retrieval_method = "hybrid"`, add-only `hybrid_score`). On the dogfood corpus with the real `FastEmbedProvider`, hybrid lifts **top-1 accuracy 0.0333 → 0.6667** and **MRR 0.4095 → 0.7881** over the BM25 baseline (recall@10 0.9667 unchanged, recall@5 0.9000 → 0.9333). BM25 alone usually finds the right *file* in the top-10 but rarely ranks it first; fusing the vector signal fixes that. Evidence: `docs/spikes/phase-21-hybrid-recall.md` (ADR-013: real run, no synthetic figures).
- **Reranker pipeline (cross-encoder) — opt-in, deterministic default.** A `Reranker` trait + the deterministic, model-free `IdentityReranker` (default build, 0 model dep) + a feature-gated real `CrossEncoderReranker` (`BGE-reranker-base`), wired via `Retriever::with_reranker`. The real cross-encoder beats the BM25 baseline (top-1 +0.30, MRR +0.22) and gives the best recall@5 (0.9667). **Honest caveat**: on this small code-centric corpus it does *not* beat hybrid RRF on top-1/MRR (the general-text reranker is weaker on code chunks than the in-domain fusion), so rerank is a domain-fit-dependent opt-in, never a default.
- **Add-only contract, no breaking bump.** `SearchRequest.hybrid` (field 8) is an add-only request field (proto-freeze guard PASS); the reranker is a builder seam (no proto field, default `None`). Existing clients are unaffected (unset → BM25), 22-endpoint conformance intact.
- **eval CLI multi-path.** `contextforge eval run --semantic --hybrid --rerank` reports BM25 + semantic + hybrid + reranked recall/gate side by side (`SummarizePasses`, add-only — byte-equivalent to the legacy output with no flags).
- **ADR-025 hybrid-scoring-fusion → Accepted** + **ADR-026 reranker-provider → Accepted** (with the honest reranker caveat above), both ratified on real dogfood eval data (ADR-013). **ADR-014 cross-validation gate — 12th activation**.

Quick start (retrieval quality):

```bash
# eval CLI: BM25 baseline + semantic + hybrid + reranked, multi-path report + recall gate (default is BM25-only)
contextforge eval run --semantic --hybrid --rerank --collection=default

# real hybrid/reranked recall vs the BM25 baseline over the dogfood corpus (downloads ONNX models)
cargo run -p contextforge-core --example phase21_hybrid_rerank_recall --features embedding-fastembed,reranker-fastembed
```

(`hybrid` is opted into via `SearchRequest.hybrid` / `eval run --hybrid`; reranking is wired via `Retriever::with_reranker`. The console-api `?hybrid=true` / `?rerank=true` REST forward follows the Phase 20 `?semantic` pattern in a later release.)

详 `RELEASE_NOTES.md` v0.14.0 段 + [Phase 21 spec](docs/specs/phases/phase-21-retrieval-quality.md) + [ADR-025](docs/decisions/adr-025-hybrid-scoring-fusion.md) + [ADR-026](docs/decisions/adr-026-reranker-provider.md) + [hybrid/reranked recall evidence](docs/spikes/phase-21-hybrid-recall.md)。

## What's new in v0.13.0

🔗 **v0.13.0 semantic-retrieval-throughline** — carries the Phase 19 (v0.12.0) semantic path the last mile: it now engages **end-to-end through console-api** (Phase 20), and real recall is measured **through the production `Retriever` hot path**. This closes the two caveats v0.12.0 honestly recorded.

- **console-api `/v1/search?semantic=true` now engages the semantic path end-to-end.** In v0.12.0 the vector path was opt-in only via the CLI (`eval run --semantic`) and the `internal/daemon/rest.go` REST surface — console-api (`internal/consoleapi`) did **not** forward `semantic`. v0.13.0 adds it across the whole console-api stack: `?semantic=true` (or body `semantic`) → Go `handleSearch` OR-merge → `grpcclient` → `console_data_plane` gRPC `SearchService.Query` semantic dispatch (Rust `SearchServer::query`, mirroring the core `CoreService` `server.rs`) → ranked hits whose trace carries `candidate_generation_steps=vector-bruteforce`. **Default retrieval stays BM25** — semantic is still opt-in.
- **Real recall, now through the production hot path (resolves the v0.12.0 example caveat).** v0.12.0 measured real recall with a standalone example; v0.13.0 routes it through the production `Retriever::search_semantic` path (real scanner + chunker → 175 production chunks). With the real `FastEmbedProvider` (`all-MiniLM-L6-v2`, dim 384): **SemanticRecall@5 = 0.9667, @10 = 1.0000** (top-1 **0.7333**, MRR **0.8367**) — clearing the ADR-006 A1 gate (≥ 0.70). **Honest caveat**: @10 = 1.0 is partly file-level chunk-count inflation from the uncapped production chunker (the artifact task-19.5 suppressed with `MAX_CHUNKS_PER_FILE`); the discriminating top-1/MRR (0.7333/0.8367, *higher* than task-19.5's 0.60/0.70) prove genuine real-path performance, not pure inflation. The two are not directly comparable (different chunking); both pass the gate. Evidence: `docs/spikes/phase-20-recall-via-retriever.md` (ADR-013: real run, no synthetic/fabricated figures).
- **Add-only contract, no breaking bump.** `console_data_plane SearchRequest.semantic` (field 7) is an **add-only** request field (`buf` regen; Rust pb auto-regenerated by `core/build.rs`). Console Contract v1 shape is unchanged, the 22-endpoint conformance is intact, and existing clients are unaffected (unset → BM25).
- **Default build unchanged & dependency-free.** No vector backend and no embedding model are compiled by default — the default semantic path (incl. via console-api) uses the 0-dep deterministic provider + brute-force searcher. Real-model semantic search is a feature/deploy choice (`embedding-fastembed`).
- **ADR-024 console-api-semantic-forward → Accepted**, ratified on real landing (Go tests + Rust dispatch test, not synthetic). **ADR-014 cross-validation gate — 11th activation** across PRs #155–#156 + this closeout.

Quick start (semantic path):

```bash
# REST via console-api: opt into the vector path with ?semantic=true (now works end-to-end; default is BM25)
curl -X POST "$BASE/v1/search?semantic=true" -H 'Content-Type: application/json' \
  -d '{"query":"where is the config loader","workspace_id":"<ws>","top_k":5}'

# eval CLI: BM25 + semantic dual-path report + recall gate
contextforge eval run --semantic --collection=default
```

(There is no `contextforge search --semantic` flag — semantic is opted into via the REST `?semantic=true` query param / body field or via `eval run --semantic`.)

详 `RELEASE_NOTES.md` v0.13.0 段 + [Phase 20 spec](docs/specs/phases/phase-20-semantic-retrieval-throughline.md) + [ADR-024](docs/decisions/adr-024-console-api-semantic-forward.md) + [real recall evidence](docs/spikes/phase-20-recall-via-retriever.md)。

## What's new in v0.12.0

🔎 **v0.12.0 vector-retrieval-integration** — turns the Phase 18 vector-backend *infrastructure* into a **live, end-to-end semantic retrieval path** (Phase 19) and ratifies **ADR-023** on **real** embedding recall.

- **End-to-end semantic path is live (opt-in).** A request can now take the vector path through the full stack: `POST /v1/search?semantic=true` (REST) → Go → Rust gRPC → `EmbeddingProvider` → vector backend → ranked hits carrying `retrieval_method="vector"` + `vector_score` + `embedding_provider`. The eval CLI gains `contextforge eval run --semantic` (BM25 + semantic dual-path report + recall gate). **Default retrieval stays BM25** — semantic is opt-in.
- **Two embedding providers (ADR-008 amendment).** `DeterministicEmbeddingProvider` (Sha256→dim-384, **0 dep**) is the default-build provider — it proves the wiring end-to-end with zero new dependency. The **real** `FastEmbedProvider` (`all-MiniLM-L6-v2`, ONNX, dim 384) is behind the `embedding-fastembed` feature (rustls; builds on Linux + Windows MSVC).
- **Real recall, measured (resolves the Phase 18 caveat).** With the real provider over real ContextForge text, **SemanticRecall@5 = 0.8333, @10 = 0.9333** (top-1 0.60, MRR 0.70) — clearing the ADR-006 A1 gate (≥ 0.70). The Phase 18 synthetic 1.0/1.0 was non-discriminating; real embeddings are. Evidence: `docs/spikes/phase-19-real-recall.md` (ADR-013: real run, no synthetic/fabricated figures).
- **ADR-023 ratified → Accepted; ADR-006 Amendment A1 → Active.** The default-backend decision is ratified on real recall. The *implemented* default semantic searcher is the **0-dep `BruteForceVectorBackend`** (exact cosine, honoring D5); sqlite-vec (D1) / hnsw (D2) / qdrant (D3) / lancedb (D4) remain the feature-gated tiers.
- **Default build unchanged & dependency-free.** No vector backend and no embedding model are compiled by default — the default semantic path uses the 0-dep deterministic provider + brute-force searcher. Real-model semantic search is a feature/deploy choice (`embedding-fastembed`).
- **ADR-014 cross-validation gate — 10th activation** across PRs #141–#147 + this closeout.

Quick start (semantic path):

```bash
# REST: opt into the vector path with ?semantic=true (default is BM25)
curl -X POST "$BASE/v1/search?semantic=true" -H 'Content-Type: application/json' \
  -d '{"query":"where is the config loader","workspace_id":"<ws>","top_k":5}'

# eval CLI: BM25 + semantic dual-path report + recall gate
contextforge eval run --semantic --collection=default
```

详 `RELEASE_NOTES.md` v0.12.0 段 + [Phase 19 spec](docs/specs/phases/phase-19-vector-retrieval-integration.md) + [ADR-023](docs/decisions/adr-023-vector-backend-default.md) + [real recall evidence](docs/spikes/phase-19-real-recall.md)。

## What's new in v0.11.0

🧭 **v0.11.0 vector-backend-selection** — ships the **vector retrieval backend infrastructure + a data-driven backend selection** (Phase 18): the `Vector{Backend,Indexer,Searcher}` trait abstraction, a deterministic spike harness, **four real-data backend spikes** (sqlite-vec / qdrant / lancedb / hnsw) measured on one Linux host, the **ADR-023** default-backend decision (**Proposed**), and a `SemanticRecall@K` eval metric + gate.

- **Infrastructure + selection milestone — not live semantic search.** Production semantic retrieval and the ADR-023 ratification are **deferred to a follow-on phase**: the spike deliberately used deterministic seed vectors to avoid an ONNX/embedding dependency, so there is no real-distribution recall yet (all four backends score 1.0 on synthetic data — non-discriminating). The default backend wiring + an embedding provider are tracked under `[SPEC-OWNER:phase-future.vector-retrieval-integration]` (ADR-023 D6).
- **Default build is unchanged & dependency-free.** The `vector-*` features ship **off by default** — the default build is BM25-only (`NoopVectorBackend`), 0 new dependencies. Enabling a backend is a build-time feature choice.
- **Four backends, real Linux 5-dim data** (n=100k): sqlite-vec (lightest, ADR-002 SQLite-aligned, exact), hnsw (pure-Rust, builds everywhere, but 28 s graph build + 180 MB at scale), qdrant (external server ANN), lancedb (embedded columnar, fastest writes). Comparison: `docs/spikes/phase-18-comparison.md`.
- **ADR-023 (Proposed)** — tiered, feature-gated: D1 sqlite-vec recommended embedded default (provisional), D2 hnsw cross-platform fallback, D3 qdrant scale-out, D4 lancedb embedded-columnar, D5 default build ships none.
- **`SemanticRecall@K` eval gate** (ADR-006 Amendment A1) — metric + `MeetsRecallGate` (BM25 Top5≥0.75 / Top10≥0.85 always; SemanticRecall@10≥0.70 when the vector path is evaluated). Live values await the embedding provider.
- **ADR-014 cross-validation gate — 9th activation** across PRs #133/#134/#135/#136/#137 + this closeout.

详 `RELEASE_NOTES.md` v0.11.0 段 + [Phase 18 spec](docs/specs/phases/phase-18-vector-backend-selection.md) + [ADR-023](docs/decisions/adr-023-vector-backend-default.md) + [comparison](docs/spikes/phase-18-comparison.md)。

## What's new in v0.10.0

🎉 **v0.10.0 is-pinned-amendment** — closes the final ContextForge-Console PR #91/#93 backlog item (P2 #6 `MemoryItem.is_pinned`). Backlog is now **11/11 = 100% closed** 🎊. First successful activation of the ADR-015 D5 字段冻结 amendment path via ADR-022.

- **Phase 17 is-pinned-amendment** (ADR-022 Proposed → Accepted) — single task, single closeout PR:
  - **task-17.1 (P2 #6)** `MemoryItem.is_pinned` add-only wire field — proto field 10 + Rust `memory_to_pb` mapper + Go `contractv1.MemoryItem.IsPinned bool` + `grpcclient.protoToMemoryItem` + `MemMemoryStore.Pin(id, pin)` actually writing IsPinned + fixture-1 preset `IsPinned: true` + `handleMemoryPin` JSON body parser (`{"pin": bool}` with empty-body backward-compat default `true`) + 5 new tests + smoke v8 step 28 (4 sub-assertions covering pin/unpin/empty-body across REAL daemon + sqlite3). PR [#118](https://github.com/tajiaoyezi/contextforge/pull/118).
  - **Cross-repo coordination** — ContextForge-Console PR [#101](https://github.com/tajiaoyezi/ContextForge-Console/pull/101) shipped `MemoryItem.IsPinned bool` add-only on Console master @ `415ee30` first (ADR-022 D4 第 1 步, merged 2026-05-28T12:16:57Z); ContextForge backend caught up the same day. Verified via `gh api` round-trip before flipping Phase 17 Status `Pending → Ready → Done` in PR #118.
  - **Spec drift discovery** — `is_pinned INTEGER NOT NULL DEFAULT 0` was already added in migration `0013_memory_items.sql` at task-13.1 ship (Phase 13 forward-added). Migration `0017` prescribed by task-17.1 §3 was **not needed** — would conflict with `duplicate column name` on existing DBs. PR #118 commit body + task-17.1 §3 document this discovery.
- **ADR-022 Accepted** — first amendment ADR activating the ADR-015 D5 字段冻结 amendment path. Documents cross-repo coordination protocol (D4 Console-first / ContextForge-second ship order; D5 `Pending → Ready → Done` trigger via user-forwarded merge SHA) for future schema evolutions.
- **ADR-014 cross-validation gate 8th activation** — D1 mapping table + D2 lint 0 hits + D3 verified-by + D4/D5 governance — see PR #118 + this closeout PR body.
- **Console PR #91/#93 backlog formally 11/11 = 100% closed 🎊** — the backlog raised by the Console team in v0.7 review is now entirely addressed across Phase 13 / 15 / 16 / 17. Mapping table in `docs/releases/v0.10.0-evidence.md` §9.
- **Cross-repo end-to-end closure (2026-05-29)** 🎉 — ContextForge-Console UI visual closure shipped to Console master @ `c1c4609` (PRs [#102](https://github.com/tajiaoyezi/ContextForge-Console/pull/102) pin sort + list icon + detail badge / [#103](https://github.com/tajiaoyezi/ContextForge-Console/pull/103) mock + GHCR pull + docs / [#104](https://github.com/tajiaoyezi/ContextForge-Console/pull/104) sort util + 单测). Backlog end-to-end fully closed: backend protocol via cumulative Phase 13/15/16/17 + UI visual surface via Console PRs #102/103/104. E2E daemon-level verified via `docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.10.0` → web 详情页 "已置顶" badge 实拍坐实.

详 `RELEASE_NOTES.md` v0.10.0 段 + [Phase 17 spec](docs/specs/phases/phase-17-is-pinned-amendment.md) + [ADR-022](docs/decisions/adr-022-memory-is-pinned-field-amendment.md)。

## What's new in v0.9.0

🚀 **v0.9.0-backlog-completion** — closes 4/5 remaining Console backlog items (P3 + P4) plus production release infrastructure. ContextForge-Console PR #91/#93 backlog now **10/11 = 91% closed**; only `MemoryItem.is_pinned` (P2 #6, ADR-015 D5 amendment) remains for Phase 17 cross-repo coord.

- **Phase 16 v0.9.0-backlog-completion** (no new ADR — 4 tasks all extend existing ADR-013/015/016/017/018):
  - **task-16.1 (P4 #10)** TraceStore SQLite persistence — migration `0015_search_traces.sql` (5 cols + 1 index, IF NOT EXISTS idempotent) + new `core/src/data_plane/search_persist.rs::SqliteTracePersist` + `TraceStore` write-through redesign (hot cache LRU cap=1000 unchanged + SQLite SoT best-effort dual-write) + daemon warm restore on startup. `GET /v1/queries` and `GET /v1/search/{query_id}/trace` now survive daemon restart.
  - **task-16.2 (P4 #11)** events `?wait=` real long-poll — `handleEvents` now propagates `parseWaitParam` to `deps.Events.Recent`; `EventsClient.Recent` signature gains `wait time.Duration`; `grpcclient.eventsClient.Recent` implements two-phase wait (phase 1 block ≤ wait for first event; phase 2 short `drainTimeout=100ms` drain). `?wait=5s` GET now truly blocks 5s when no events vs prior batch polling.
  - **task-16.3 (P3 #8)** GHCR image push CI — new `.github/workflows/release.yml` (`v*` tag push triggers docker build + push `ghcr.io/${owner}/contextforge-daemon:{tag}` + `:latest`; linux/amd64 only for v0.9; multi-arch deferred) + new `.github/workflows/ci.yml` (PR + push master → cargo-test + go-test + spec-lint 3 parallel jobs). Users can now `docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.9.0`.
  - **task-16.4 (P3 #9)** docker-compose.production.yml — new `deploy/docker-compose.production.yml` (dual-container: `contextforge-core` daemon bind 0.0.0.0:50551 + `console-api-serve` REST proxy --grpc-addr=contextforge-core:50551; fallback deny default per ADR-018; named volume `contextforge-data` persistence + healthcheck) + `.env.production.example` + `docs/deploy/production.md` (9 sections: Quick start / image / data / health / auth / upgrade / k8s skeleton / troubleshooting / perf) + new env opt-in `CONTEXTFORGE_ALLOW_WILDCARD_BIND=1` for 0.0.0.0 bind. smoke v7 27-step.
  - **release verify workflow (E7)** — new `.github/workflows/verify-image.yml` (workflow_dispatch with `tag` input → pull + run + `/v1/health` health probe; opt-in `--detailed=true` 5-component breakdown via ADR-020). Verified green on v0.9.0-rc1 and v0.9.0.
- **ADR-014 cross-validation gate 7th activation** — D1 mapping table + D2 lint 0 hits + D3 verified-by + D4/D5 governance — see PR #114 closeout body.
- **No new ADR in v0.9.0 itself** — Phase 17 + ADR-022 (`memory-is-pinned-field-amendment`) scaffolded as separate post-release PR (#116, Status: Pending awaiting Console contractv1.go cross-repo amend trigger).

Remaining Console backlog (deferred to Phase 17):
- P2 #6 `MemoryItem.is_pinned` (ADR-015 D5 amendment via ADR-022 Proposed → needs Console cross-repo trigger).

详 `RELEASE_NOTES.md` v0.9.0 段 + [Phase 16 spec](docs/specs/phases/phase-16-v0.9.0-backlog-completion.md) + [docs/deploy/production.md](docs/deploy/production.md)。

## What's new in v0.8.0

🎯 **Console functional gap closure** — closes 6/11 backlog items raised by the Console team (Console PR #91/#93) covering P0 fallback / memory event bridge + P1 Dashboard backend endpoints + P2 5-link health detail. Console UI Dashboard 3 KPI cards and CoreHealthCard now have backend data; Memory 详情面板 "操作历史" 列表 auto-populates via the new memory.* event stream.

- **Phase 15 console-functional-gap-closure** (ADR-020 / ADR-021) — 6 task PRs (#99-#104) + closeout PR (#105). Includes:
  - **task-15.1** MemStore chunk/trace cache — `CONSOLE_API_FALLBACK_INMEM=1` mode no longer 503s on drill-down `GET /v1/source-chunks/<id>` / `GET /v1/search/<query_id>/trace` (cache the stub Search emits)
  - **task-15.2 (ADR-021)** memory.* → EventBus bridge — `memory.pin` / `memory.deprecate` / `memory.soft_delete` events broadcast to `/v1/observability/events` stream
  - **task-15.3** `GET /v1/stats/chunks` — `{total, today_delta}` for Dashboard "已索引块" KPI
  - **task-15.4** `GET /v1/eval-runs` (list) — filter by `?workspace_id=&status=&limit=N`, ORDER BY started_at DESC
  - **task-15.5** `GET /v1/queries` — query history from in-memory trace store, default limit 20
  - **task-15.6 (ADR-020)** `GET /v1/health?detailed=true` — opt-in 5-component breakdown (db / index / embed / retriever / eval)
- **ADR-015 D1 add-only**: All proto / Go schema changes purely additive — Console v0.7.x clients reading v0.8 responses silently ignore the new fields; existing 22-endpoint conformance test PASS (no regression).
- **ADR-014 cross-validation gate 6th activation** — D1 mapping table + D2 lint 0 hits + D3 verified-by + D4/D5 governance — see PR #105 body.
- **ADR-020 / ADR-021 Accepted** (2026-05-26, Phase 15 closeout PR #105).

Remaining Console backlog (deferred to Phase 16 / v0.9.0):
- P2 #6 `MemoryItem.is_pinned` (ADR-015 D5 amendment) · P3 #8 ghcr.io image push · P3 #9 docker-compose.production.yml example · P4 #10 TraceStore SQLite persist · P4 #11 `?wait=` real long-poll.

详 `RELEASE_NOTES.md` v0.8.0 段 + [ADR-020](docs/decisions/adr-020-health-component-breakdown.md) + [ADR-021](docs/decisions/adr-021-memory-event-bus-bridge.md)。

## What's new in v0.7.2

⚠️ **BREAKING — fallback-inmem default reversal** (ADR-018):

- 删 Dockerfile `ENV CONSOLE_API_FALLBACK_INMEM=1` → daemon 默认 fallback **deny**
- `docker run contextforge-daemon:v0.7.2` 不显式 opt-in → `/v1/health` 返 **503** + docker healthcheck unhealthy
- 保留 v0.7.1 行为需 `docker run -e CONSOLE_API_FALLBACK_INMEM=1 ...` 显式 opt-in
- 修复 v0.7.1 silent footgun：HTTP 200 healthcheck 掩盖容器重启数据失风险
- 代码无改动；仅 Dockerfile 删 ENV 行 + ADR-018 spec lock + ratification test

详 `RELEASE_NOTES.md` v0.7.2 段 + [ADR-018](docs/decisions/adr-018-fallback-inmem-default-reversal.md)。

## What's new in v0.7.1

🐳 **Dockerfile + single-image deployment fix** — v0.7.0 Dockerfile 4 处 stale
(rust 1.82 → 1.93 / go 1.22 → 1.26 / ENV `CONSOLE_API_FALLBACK_INMEM=1` 占位 /
新 `.dockerignore` 排 9.3 GB cargo cache) 一次性收齐。

注：v0.7.1 的 ENV 行已在 v0.7.2 移除（fallback 默认从 enable 反转为 deny，
silent footgun fix）。详 v0.7.2 段。

## What's new in v0.7.0

🎉 **Console 22-endpoint conformance 100% PASS** — ContextForge ships all 22
Console contract v1 REST endpoints; Console UI HTTPAdapter v1.0 双方握手成功.

- **Phase 14 eval-rest-surface** (ADR-017 Wave 4) — Console Contract v1
  endpoint coverage 18 → 20 distinct routes (covering all 22 contract
  endpoints, 2 shared via filter shape). 22-endpoint conformance now 100%.
- **task-14.1 Rust SoT**: SQLite `eval_runs` table (migration 0014) +
  `SqliteEvalStore` (5 methods, JSON-roundtrip metrics + case_results) +
  `EvalService` 3 gRPC RPCs (Create / Get / UpdateProgress) — Create returns
  `EvalRun{status:"running", started_at:now}`; UpdateProgress is the Go-side
  runner callback channel that persists status terminal + metrics + case_results.
- **task-14.2 Go REST**: 2 routes (`POST /v1/eval-runs` + `GET /v1/eval-runs/{id}`)
  + `runEvalAsync` goroutine that drives a light-weight recall harness against
  BuiltinGoldenQuestions and reverse-updates the Rust store via UpdateProgress
  when terminal. `MemEvalStore` fallback (2s timer auto-advance to succeeded
  with mock metrics).
- **`console_smoke.sh` v5**: 18 → 20 endpoint REAL flow; new Steps 19/20 cover
  POST eval-runs (200 + status=running) + poll GET until terminal + verify
  metrics contains `recall@5`. Final marker `CONSOLE_REAL_SMOKE_EXIT=0`.
- **ADR-017 Status: Proposed → Accepted** — 6 D-clauses spanning v0.5/v0.6/v0.7
  3 phase one-shot promoted. ADR-014 cross-validation gate **5th activation**
  pass — 制度稳定性跨 5 phase (v0.3-v0.7) 验证.

## What's new in v0.6.0

- **Phase 13 memory-rest-surface** (ADR-017 Wave 3) — Console Contract v1
  endpoint coverage 13 → 18 (the 5 memory endpoints). 22-endpoint conformance
  bumps 64% → 82%.
- **task-13.1 Rust SoT**: SQLite `memory_items` table (migration 0013) + 自研
  `SqliteMemoryStore` (10 columns; status CHECK constraint; 3 indexes) +
  `MemoryService` 5 gRPC RPCs (List / Get / Pin / Deprecate / SoftDelete) +
  Pin/Deprecate/SoftDelete each emit `AuditOperation::Memory*` events via
  shared `AuditSink` (4 new enum variants).
- **task-13.2 Go REST**: 5 routes (`GET /v1/memory[?agent_id=&scope=&namespace=&include_soft_deleted=]`
  + `GET /v1/memory/{id}` + `POST /v1/memory/{id}/pin` non-destructive 204 +
  `POST /v1/memory/{id}/deprecate` + `POST /v1/memory/{id}/soft-delete` —
  the last two confirmMiddleware-gated, returning 412 PRECONDITION_FAILED
  without X-Confirm/?confirm=true). `MemMemoryStore.SeedFixtures()` provides
  5 in-memory items for `CONSOLE_API_FALLBACK_INMEM=1` demo.
- **`console_smoke.sh` v4**: 13 → 18 endpoint REAL flow; new Steps 13-18 cover
  memory seed (sqlite3 CLI) + list + get + pin 204 + deprecate 412/204 +
  soft-delete 412/204 with default-list exclusion. Final marker
  `CONSOLE_REAL_SMOKE_EXIT=0`.
- ADR-014 cross-validation gate **fourth activation** pass — 制度稳定性
  跨 4 phase 验证.
- ADR-017 Status still **Proposed** (full Accepted promotion deferred to
  Phase 14 closeout).

## What's new in v0.5.0

- **Phase 12 console-contract-completion** (ADR-017 Wave 1+2) — Console Contract
  v1 endpoint coverage 9 → 13 (route inventory 9 → 14 including the new GET
  `/v1/source-chunks/{id}` and GET `/v1/search/{query_id}/trace`). 22-endpoint
  conformance bumps from 41% → 64%.
- **task-12.1 quick-win Wave 1**: `PATCH /v1/workspaces/{id}/config` overwrites
  allowlist + denylist via gRPC `WorkspaceService.UpdateConfig`; `GET /v1/index-jobs?status=active`
  filters queued + running via `JobService.List`; `POST /v1/index-jobs/{id}/cancel`
  switches **200 → 204 No Content** (ADR-017 D3); `confirmMiddleware` enforces
  `X-Confirm: yes` header **OR** `?confirm=true` query for destructive ops →
  412 Precondition Failed (ADR-017 D2 server-side bottom defense).
- **task-12.2 source-chunk-by-id**: new `SearchService.GetSourceChunk` RPC +
  Go REST handler reuse existing `Retriever::get_chunk` (task-6.2 SQL fast-path);
  workspace_id optional with workspace enumeration fallback.
- **task-12.3 search-trace-by-query-id**: new `SearchService.GetSearchTrace`
  RPC + in-memory `TraceStore` (HashMap + VecDeque LRU cap 1000); every
  `SearchService.Query` generates a unique `qry-{nanos}` query_id and persists
  its `RetrievalTrace` for later GET by query_id (daemon restart wipes cache —
  SQLite persistence `[SPEC-DEFER:task-future.search-trace-sqlite-persistence]`).
- **`console_smoke.sh` v3**: 9 → 13 endpoint REAL flow; new Steps 9-12 cover
  PATCH config 412→200, active filter + 400, source-chunks lookup, trace fetch.
  Final marker `CONSOLE_REAL_SMOKE_EXIT=0`.
- ADR-014 cross-validation gate **third activation** pass — 制度稳定性 verified.
- ADR-017 Status remains **Proposed** (full Accepted promotion deferred to Phase
  14 closeout where 6 D-clauses across v0.5/v0.6/v0.7 land together).

## What's new in v0.4.0

- **Phase 11 console-real-data-plane** (ADR-016) — `console-api-serve` 默认行为
  从 v0.3 in-memory MemStore 切到 **cross-process gRPC bridge**: Go REST handler
  → `internal/consoleapi/grpcclient` → Rust `core/src/data_plane/` 4 gRPC service
  (Workspace / Job / Search / Events) → SqliteWorkspaceStore + SqliteJobStore.
- **真索引 + 真搜索**: `POST /v1/index-jobs` 真触发 `JobRunner.spawn_blocking(
  IndexSession::index_path_with_progress)`; `POST /v1/search` 真接 retriever
  (Tantivy + SQLite chunks). v0.3 占位 stub 完全 retired.
- **EventBus broadcast** + `/v1/observability/events` batch polling (`?wait=<duration>`
  + `?limit=<int>` 参数 reserved；v0.7.x REST tier 实现是 immediate batch return，
  不真 block — long-poll honoring 留 [SPEC-DEFER:task-future.consoleapi-sse-or-long-poll-honoring]；
  Console 端应自管轮询频率，详 `docs/releases/v0.7.0-integration.md` §7).
  JobRunner heartbeat 真 emit `indexing.progress` / `indexing.cancelled` /
  `indexing.error` 事件.
- **`console_smoke.sh` v2 REAL mode default**: spawns both daemons + drives
  fixture index + verifies real chunks. Final marker
  `CONSOLE_REAL_SMOKE_EXIT=0`. `LOCAL_ONLY=1` retains v0.3 inmem fallback.
- **`--fallback-inmem` env-gated**: `CONSOLE_API_FALLBACK_INMEM=1` keeps v0.3
  behavior available for demo / fallback / conformance test.
- ADR-014 cross-validation gate **second activation** pass —制度稳定性验证.

## What's new in v0.3.0

- **ContextForge ↔ ContextForge-Console Contract v1 兼容层** (ADR-015) — 17 Go types
  mirror Console `contractv1.go` + Rust workspace/jobs resource models +
  9 REST endpoints under `/v1/*`.
- New CLI subcommand: `contextforge console-api-serve --addr 127.0.0.1:48181`
  exposes the 9 Console Contract v1 endpoints (in-memory store for v0.3; cross-
  process Rust ↔ Go SQLite sharing is v0.4 follow-up).
- New smoke: `bash scripts/console_smoke.sh` exercises the 9 endpoint flow end-to-end.
- Docker compose stack: `deploy/console-stack.yml` + multi-stage `Dockerfile`.
- ADR-014 cross-validation gate fully activated for Phase 10 (D1 mapping table +
  D2 lint + D3 verified-by + D4/D5 in `scripts/spec_drift_lint.sh`).

## Quick Start

### One-shot smoke (Linux / WSL2 / Git Bash on Windows)

```bash
bash scripts/quickstart_smoke.sh
```

Builds both binaries, drives the seven-step CLI walkthrough end-to-end against
the [examples/quickstart/](examples/quickstart/) fixture, and prints
`QUICKSTART_SMOKE_EXIT=0` on success.

### Manual steps

```bash
# 1. Build (Go 1.22+, Rust stable).
go build -o contextforge ./cmd/contextforge
cargo build -p contextforge-core
export PATH="$(pwd):$(pwd)/target/debug:$PATH"
export CONTEXTFORGE_DATA_DIR="$HOME/.contextforge-demo"

# 2. Initialise the data root.
contextforge init --root "$CONTEXTFORGE_DATA_DIR"

# 3. Import the Hermes memory fixture (writes canonical .md to <data-dir>/imports/hermes/).
contextforge import hermes examples/quickstart/hermes-memory \
  --collection demo --data-dir "$CONTEXTFORGE_DATA_DIR"

# 4. Index the imported records.
contextforge index --source "$CONTEXTFORGE_DATA_DIR/imports/hermes" \
  --collection demo --data-dir "$CONTEXTFORGE_DATA_DIR"

# 5. Index the sample project (denylist skips .env; secret-redaction rewrites config.yaml AWS key).
contextforge index --source examples/quickstart/sample-project \
  --collection demo --data-dir "$CONTEXTFORGE_DATA_DIR"

# 6. Search — the `configuration` keyword lives in docs/config.md.
contextforge search --collections=demo --top-k=5 --explain "configuration"

# 7. Eval (builtin 30 golden questions).
contextforge eval run --collection demo
```

Notes on flag order: `--collections=demo` (and any other flag) MUST precede the
positional query when invoking `contextforge search` — the stdlib `flag` parser
stops at the first non-flag argument.

### Expected output

- Step 2 prints `contextforge: initialized <data-dir> (schema_version 0.1)`.
- Step 4–5 stream `\rindexing <file> (files=N, chunks=M)` lines, then a final
  `done collection=demo files=… chunks=… denied=… redacted=…` summary.
- Step 6 emits one block per hit: `<chunk_id>  <file>:<start>-<end>  score=…  redaction_status=…` plus a `reason=…` line.
- Step 7 prints the eval report (`Top-5`, `Top-10`, `latency`, optional miss
  list).

## v0.2 limitations

- Official target: Linux x86_64 / WSL2; macOS should work, Windows is best
  effort via Git Bash.
- `LICENSE` remains all-rights-reserved (occupies the slot until an OSI
  license is chosen).
- No GitHub Release tarball is published from this repo yet — release
  contracts and a release-smoke gate are in place
  (`scripts/release_smoke.sh`, `scripts/quickstart_smoke.sh`), but the
  actual `gh release create` step is performed by an external release job.

## Where to go next

- `contextforge.example.toml` — starting point for collection allowlists and
  local-only provider settings.
- [`docs/specs/phases/phase-9-cli-pipeline.md`](docs/specs/phases/phase-9-cli-pipeline.md) — the v0.2 CLI data-plane phase that re-enabled the manual
  Quick Start sequence above.
- [`docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`](docs/decisions/adr-013-cli-data-plane-grpc-bridge.md) — context on why
  v0.1's `contextforge index` / `contextforge import` were stubs and what
  Phase 9 changed.
