# ContextForge

ContextForge is a local-first context indexing and retrieval tool for agent memory, rules, source files, logs, and project notes.

It ships as two binaries (ADR-001):

- `contextforge`: Go control-plane CLI, REST/MCP adapter, Console Contract v1 REST surface (`console-api-serve`, v0.3+), export and eval entrypoint.
- `contextforge-core`: Rust data-plane daemon for scan, parse, chunk, index, and retrieval.

## What's new in v0.33.0

🧹 **v0.33.0 governance-debt-cleanup-3** — a third wave of cross-phase governance-debt cleanup (mirrors Phase 31 / 33), clearing two real, code-local governance markers. (1) **memory-actor-propagation** (ADR-032 defer): `pin()` hardcoded the actor `"console-api"` because the input-to-store propagation chain was missing — `PinMemoryRequest` had no actor field, Go `MemoryClient.Pin` no actor param, and `handleMemoryPin` read no caller identity (the `set_pinned_with_actor` store field has existed since task-27.1). v0.33.0 adds `PinMemoryRequest.actor=3` (add-only, existing `memory_id=1`/`pin=2` frozen, ADR-015 D1) + threads `Pin(id,pin,actor)` through the Go interface + 3 impls + fills `pb.PinMemoryRequest.Actor` + `handleMemoryPin` reads the `X-Actor` header, so a console deployment behind an auth proxy (`X-Actor` / `X-Forwarded-User`) can attribute pins to the real caller (written to the existing `pinned_by`). (2) **l2-cache-true-lru** (ADR-038 defer): Phase 33 bounded the L2 SQLite embedding cache with rowid-FIFO (insert order), but `sqlite_get` did not re-order on hit. v0.33.0 makes `sqlite_get`, on a hit and only when bounded, re-write the row to bump its implicit rowid to the tail — turning the eviction into **access-order LRU**, reusing the implicit rowid (**0 schema migration**, correcting Phase 33's assumption that true-LRU needs a `created_at` column; mirrors the Go memstore move-to-front). **0 new dependency, 0 schema migration, default byte-equivalent** (ADR-004/008/015): no `X-Actor` header → empty actor → server falls back to `"console-api"`; L2 hit-bump only fires under a finite cap and re-writes identical bytes (return value unchanged). Honest scope (ADR-013): caller-supplied actor is a *declared* identity — **authenticated** identity stays deferred (`[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`); the L2 hit-bump's read-path write-amplification is the inherent cost of access-order LRU, and `with_sqlite` has no production call site (opt-in path) so there is zero live impact. **ADR-045 → Accepted**; ADR-032/038/027/015 add-only Phase-40 Amendments; ADR-014 — 31st activation.

- **memory-actor-propagation** (task-40.1, #257): `PinMemoryRequest.actor=3` add-only + Go `Pin(id,pin,actor)` across interface/3-impls + `grpcclient` fills `Actor` + `handleMemoryPin` reads `r.Header.Get("X-Actor")` (empty → server falls back to `"console-api"`, byte-equivalent) + Rust `pin()` writes `req.actor` to `pinned_by` when non-empty. ADR-022 D2 lenient body contract unchanged; authenticated identity honest-deferred. 0 new dep / proto add-only. (TEST-40.1.1 prost wire-tag actor=3 / TEST-40.1.2 Rust propagate+fallback / TEST-40.1.3 Go X-Actor / TEST-40.1.4 grpcclient Actor.)
- **l2-embedding-cache-true-lru** (task-40.2, #258): `sqlite_get` hit-bump (only when `l2_cap>0`) re-writes the hit row to bump its implicit rowid, flipping `sqlite_put`'s rowid-ordered eviction from insert-order FIFO to access-order LRU; `cap==0` skips the bump. Reuses the implicit rowid (0 schema migration); corrects Phase 33's true-LRU assumption. (TEST-40.2.1 LRU evicts LRU not FIFO / TEST-40.2.2 cap gates the bump + results unchanged.)
- **closeout** (task-40.3): **ADR-045 governance-debt-cleanup-3 → Accepted** (per-D). **ADR-032/038/027/015** add-only Phase-40 Amendments (memory-actor-propagation / l2-cache-true-lru fulfilled + true-LRU assumption corrected; proto add-only field), NOT retro-editing their D-body (ADR-014 D5). **ADR-014 cross-validation gate — 31st activation**. Smoke v30 [49/49] (`TestTask403`). Multi-agent adversarial review (4 dimensions × 3 skeptics) confirmed 0 real defects.

```bash
# memory pin actor propagation: proto wire-tag + Rust propagate/fallback
cargo test -p contextforge-core --lib data_plane::memory
# Go X-Actor header → Pin(actor) + grpcclient fills pb.PinMemoryRequest.Actor
go test ./internal/consoleapi/... -run TestTask401
# L2 access-order LRU: hit-bump evicts LRU not FIFO + cap gates the bump
cargo test -p contextforge-core --lib embedding::cache
```

详 `RELEASE_NOTES.md` v0.33.0 段 + [Phase 40 spec](docs/specs/phases/phase-40-governance-debt-cleanup-3.md) + [ADR-045](docs/decisions/adr-045-governance-debt-cleanup-3.md)。

## What's new in v0.32.0

🔗 **v0.32.0 console-api-retrieval-signal-forward** — carries the **hybrid** (BM25+vector RRF) retrieval signal the last mile: it has lived in the retrieval core since Phase 21 (`server.rs` hybrid path + `search_hybrid` + `hybrid_score`) but was **unreachable over the public REST surface**. v0.32.0 plumbs it out to console-api `POST /v1/search` — mirroring the Phase 20 `?semantic` forward — so Console / REST clients can request `?hybrid=true` (or `{"hybrid":true}`) and get back `retrieval_method="hybrid"` + the `hybrid_score` fusion-score provenance. **0 backend algorithm change ("forward, not rewrite")**: console_data_plane proto gains two add-only fields (`SearchRequest.hybrid=8` + `SearchResultItem.hybrid_score=17`, existing field numbers frozen, ADR-015 D1), the console data-plane `query()` gains a hybrid dispatch branch reusing the existing `search_hybrid` + `reranker_from_env`, and Go console-api forwards `Hybrid` / maps `HybridScore` (mirroring `Semantic` / `VectorScore`). Reranking stays **server-side env-driven** (`CONTEXTFORGE_RERANKER_PROVIDER`, ADR-043 D3) — its `reason` provenance is now visible end-to-end in the REST response, while a per-request `?rerank` flag is recorded **superseded** by the env-driven model (honest re-scope, ADR-013 / ADR-044 D3). **0 new dependency, 0 migration, default `hybrid=false` byte-equivalent** (ADR-004/008). Fulfills ADR-025's `console-api-hybrid-forward` defer and re-scopes ADR-043's `console-api-rerank-forward` to rerank-provenance visibility.

详 `RELEASE_NOTES.md` v0.32.0 段 + [Phase 39 spec](docs/specs/phases/phase-39-console-api-retrieval-signal-forward.md) + [ADR-044](docs/decisions/adr-044-console-api-retrieval-signal-forward.md)。

## What's new in v0.31.0

📌 **v0.31.0 embedding-remote-reranker-live** — runs the **FIRST end-to-end against a REAL remote cross-encoder reranker** + measures **REAL rerank quality** + bridges the Go `[reranker]` config section to the core daemon's env + adds the **first data-plane `with_reranker` opt-in wiring**. A `RemoteRerankerProvider` (`ureq`-backed, mirroring `RemoteEmbeddingProvider`) calls SiliconFlow's `https://api.siliconflow.cn/v1/rerank` with `Qwen/Qwen3-VL-Reranker-8B` (same URL + key as embedding, different model). v0.31.0 adds the provider + `select_reranker` factory + a `reranker-remote` feature + an env-gated live-quality harness + a Go `[reranker]` env-bridge + data-plane `with_reranker` wiring — **0 backend dep, 0 new dependency** (`ureq` optional since task-22.3), **0 proto, 0 migration, 0 default-behavior change** (ADR-004/008). The **default build stays 0-network, 0-remote-dep**: the harness skips unless `CONTEXTFORGE_RERANKER_API_KEY` is set, an unset `[reranker]` config → `None` → byte-equivalent no-rerank (backward compatible), and the API key is **env-only — it never enters `config.toml`**. Honest caveat (ADR-013): `MRR = 1.0000 / recall@1 = 1.0000` on a **small author-labeled 14-case** set with deliberately-planted near-distractors proves a real cross-encoder ranks the obvious relevant doc above its near-distractor, **NOT a large-benchmark quality claim**; large-corpus / standard-benchmark rerank quality stays deferred (`[SPEC-DEFER:phase-future.reranker-large-corpus-quality]`).

- **remote-reranker-live-quality harness** (task-38.1, #247): a new env-gated `core/tests/remote_rerank_recall.rs` (`#![cfg(feature = "reranker-remote")]`) reranks 14 author-labeled query×candidate cases with deliberate near-distractors (`config_save`↔`config_load`, `bm25`↔`hybrid`, `cjk_index`↔`cjk_vector`, `cosine`↔`vector_backend`, `cache`↔`chunk`) via a **live remote reranker** and measures **rerank MRR + recall@1** (NOT embedding recall@3) against an `IdentityReranker` no-semantic baseline. **Measured (real local run, SiliconFlow `https://api.siliconflow.cn/v1/rerank` + `Qwen/Qwen3-VL-Reranker-8B`, 3 runs, ALL STABLE)**: remote **MRR = 1.0000 / recall@1 = 1.0000** (14/14 relevant ranked #1, all 3 runs); identity baseline **MRR = 0.4762 / recall@1 = 0.0000** (uniform score → tie-break by `chunk_id`, relevant never alphabetically first); **delta_MRR = +0.5238 / delta_recall@1 = +1.0000**. Guardrail floors (`MRR_remote >= 0.70` AND `MRR_remote > MRR_identity`) pass every run. A separate de-risk probe (feasibility evidence, NOT the harness metric): query "how to save config to file" → `config_save relevance_score=0.7356` ranked #1 vs near-distractor `config_load=0.0158` (~46x). The harness skips unless `CONTEXTFORGE_RERANKER_API_KEY` is set, so the default `cargo test` build stays 0-network / 0-remote-dep (ADR-004/008). New `RemoteRerankerProvider` + `select_reranker` factory + `reranker-remote` feature (0 new dep — `ureq` optional since task-22.3, `Debug` never logs `api_key`).
- **remote-reranker-config-bridge + first data-plane opt-in** (task-38.2, #248): a Go `RerankerConfig` (`[reranker]` section, **no api-key field**) + a `setRerankerEnv` cross-process env-bridge (mirroring `setRemoteEnv`, **env-wins**, **API key NEVER bridged**) + a Rust `reranker_from_env()` + the **FIRST data-plane `with_reranker` opt-in wiring** in `server.rs` (hybrid + semantic) + `data_plane/search.rs` (semantic). Default unset → `None` → byte-equivalent no rerank (backward compatible, ADR-004); feature-off / unknown → explicit `Status::internal` (no silent fallback, ADR-013). The Rust core keeps its **0-toml-dep** rule (config parsing stays Go-side).
- **CI honest-defer (key honest difference, ADR-013)**: unlike qdrant — which has a free OSS service container guarding live recall on **every CI run** — the remote reranker is a **paid external API with no free service container**. When no key is present in CI, the harness cleanly **honest-defer skips**; the real MRR/recall@1 numbers above are measured by an **already-authenticated local run**, **NOT guarded on every CI run** (reuses `[SPEC-DEFER:phase-future.embedding-remote-ci-credential]`). Unlike Phase 37 embedding (recall@1 fluctuated 0.8667–0.9333 cross-run), this rerank is **STABLE across all 3 runs** (cross-encoder joint scoring is more decisive on this small set). Recorded as-is, not overstated as "CI-guarded each run".
- **closeout** (task-38.3): **ADR-043 embedding-remote-reranker-live → Accepted** (per-D). **ADR-026 + ADR-042** add-only Phase-38 Amendment mark their reranker / remote-provider honest-defer **fulfilled** (real reranker round-trip + live quality measured), NOT retro-editing their D-body (ADR-014 D5). **ADR-014 cross-validation gate — 29th activation**. Smoke v28 [47/47] (`TestTask383`).

```bash
# live remote reranker MRR/recall@1 vs IdentityReranker baseline (skipped unless CONTEXTFORGE_RERANKER_API_KEY set)
CONTEXTFORGE_RERANKER_API_KEY=… CONTEXTFORGE_RERANKER_URL=https://api.siliconflow.cn/v1/rerank \
  CONTEXTFORGE_RERANKER_MODEL=Qwen/Qwen3-VL-Reranker-8B \
  cargo test -p contextforge-core --features reranker-remote --test remote_rerank_recall -- --nocapture
```

发版凭据（post-tag-push backfill，ADR-013）：tag SHA `<backfill: tag-sha>` / tag object `<backfill: tag-object>` / release run `<backfill: run-id>` / ghcr digest `<backfill: ghcr-digest>` / cosign tlog `<backfill: tlog-sign / tlog-attest>`。

详 `RELEASE_NOTES.md` v0.31.0 段 + [Phase 38 spec](docs/specs/phases/phase-38-embedding-remote-reranker-live.md) + [ADR-043](docs/decisions/adr-043-embedding-remote-reranker-live.md)。

## What's new in v0.30.0

📌 **v0.30.0 embedding-provider-remote-live** — closes the ADR-027 honest-defer `[SPEC-DEFER:phase-future.embedding-provider-remote]` by running the **FIRST end-to-end against a REAL remote embedding endpoint** + measuring **REAL semantic recall** + bridging the Go `[remote]` config section to the core daemon's env. The remote `EmbeddingProvider` (`ureq`-backed) was already implemented since Phase 22; v0.30.0 adds an env-gated live-recall harness + a Go `[remote]` env-bridge — **0 backend change, 0 new dependency** (`ureq` optional since task-22.3), **0 migration, 0 default-behavior change** (ADR-004/008). The **default build stays 0-network, 0-remote-dep**: the harness skips unless `CONTEXTFORGE_REMOTE_API_KEY` is set, and the API key is **env-only — it never enters `config.toml`**. Honest caveat (ADR-013): `recall@3 = 1.0000` on a **small author-curated 15-case / 16-doc** labeled set with deliberately-planted near-synonym distractors proves the remote model ranks the obvious semantic match above its near-synonym, **NOT a large-benchmark quality claim**; large-corpus semantic quality stays deferred (`[SPEC-DEFER:phase-future.embedding-large-corpus-recall]`).

- **remote-embedding-live-recall harness** (task-37.1): a new env-gated `core/tests/remote_embedding_recall.rs` embeds a 16-doc corpus via a **live remote endpoint** and a model-free `deterministic` baseline through the **same** `BruteForceVectorBackend` exact-cosine path, then measures recall@1/@3 over a 15-case author-curated labeled set with planted near-synonym distractors. **Measured (real local run, SiliconFlow `https://api.siliconflow.cn/v1/embeddings` + `Qwen/Qwen3-Embedding-8B`, dim=1024, 3 runs)**: remote **recall@1 = 0.8667–0.9333** (13–14/15, **fluctuates across runs** — the remote model/service is not fully deterministic) / **recall@3 = 1.0000** (15/15, **stable across all 3 runs**); deterministic baseline **recall@1 = 0.0000 / recall@3 = 0.0667** (stable); **delta@3 = +0.9333**. The one-or-two @1 misses are exactly the planted hard near-synonym distractors (`config_save`↔`config_load`, `hybrid`↔`bm25`) yielding the top-1 slot. The harness guardrails (floor `recall@3 >= 0.70` + `remote@1 > deterministic@1`) pass on every run. Skipped unless `CONTEXTFORGE_REMOTE_API_KEY` is set, so the default `cargo test` build stays 0-network / 0-remote-dep (ADR-004/008). 0 backend change, 0 new dep (`ureq` optional since task-22.3).
- **remote-embedding-config-bridge** (task-37.2): the Go `RemoteProviderConfig` gains an add-only `Model` field, and a new `setRemoteEnv` bridges the `[remote]` config section to the core daemon's env across process boundary (mirroring `setVectorEnv`, **env-wins**). The **API key is env-only — it is never read from / written to `config.toml`** (secret hygiene). The Rust core keeps its **0-toml-dep** rule (config parsing stays Go-side).
- **CI honest-defer (key honest difference, ADR-013)**: unlike qdrant — which has a free OSS service container guarding live KNN recall on **every CI run** — the remote endpoint is a **paid external API with no free service container**. When no key is present in CI, the harness cleanly **honest-defer skips**; the real recall numbers above are measured by an **already-authenticated local run**, **NOT guarded on every CI run**. Recorded as-is, not overstated as "CI-guarded each run".
- **closeout**: **ADR-042 embedding-provider-remote-live → Accepted** (per-D, D1–D4). **ADR-027** add-only Phase-37 Amendment marks its remote-provider honest-defer **fulfilled** (real endpoint round-trip + live recall measured), NOT retro-editing the D-body (ADR-014 D5). **ADR-014 cross-validation gate — 28th activation**.

```bash
# live remote embedding recall@1/@3 vs deterministic baseline (skipped unless CONTEXTFORGE_REMOTE_API_KEY set)
CONTEXTFORGE_REMOTE_API_KEY=… CONTEXTFORGE_REMOTE_URL=https://api.siliconflow.cn/v1/embeddings \
  CONTEXTFORGE_REMOTE_MODEL=Qwen/Qwen3-Embedding-8B \
  cargo test -p contextforge-core --features embedding-remote --test remote_embedding_recall -- --nocapture
```

发版凭据（post-tag-push backfill，ADR-013）：tag SHA `b49f28803e73338997f04bc3ffad85e7d386edf5` / tag object `38bf3c2f86241ba25be8f64456c8258d3c5d12ff` / release run `27050883547` / ghcr digest `sha256:ff1306bf088452df8cdc78d5f5f0c35bcda0e654258bcbfc0cbba5a4992fb95c` / cosign tlog `1738028951 (sign) / 1738031045 (attest)`。

详 `RELEASE_NOTES.md` v0.30.0 段 + [Phase 37 spec](docs/specs/phases/phase-37-embedding-provider-remote-live.md) + [ADR-042](docs/decisions/adr-042-embedding-provider-remote-live.md)。

## What's new in v0.29.0

📌 **v0.29.0 qdrant-live-vector-recall** — closes the ADR-034 D2 honest-defer `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` by measuring **REAL live qdrant KNN recall** and guarding it permanently in CI via a qdrant service container. The qdrant backend (connect/health/ensure-create/upsert/KNN/delete) was already fully implemented since Phase 25/29; v0.29.0 adds an env-gated harness + a `qdrant-recall` CI job — **0 backend change, 0 new dependency** (`qdrant-client` optional since task-18.4). The **default build stays 0-vector-dep, 0-network** (ADR-004/008): the harness skips unless `QDRANT_URL` is set, and the CI job is the only place a live qdrant is spun up. Honest caveat (ADR-013): `recall@10 = 1.0000` because at N=2000 — below qdrant's HNSW `indexing_threshold` (default ~10000) — qdrant serves **EXACT** KNN, so this is a live-KNN **correctness** proof (qdrant == brute-force exact ground truth) replacing the synthetic `eval_integration.rs` fixture; stressing the HNSW **approximation** regime (large corpus > `indexing_threshold` + optimizer-built index) is honestly deferred (`[SPEC-DEFER:phase-future.vector-large-corpus-perf]`).

- **qdrant-live-recall harness** (task-36.1, #236): a new env-gated `core/tests/qdrant_live_recall.rs` builds an N=2000 / dim=64 corpus, upserts into a live qdrant via the existing backend, runs M=50 KNN queries, and asserts qdrant's top-10 against a `BruteForce` **exact** KNN ground truth — measuring **recall@10 = 1.0000**. The harness is skipped unless `QDRANT_URL` is set, so the default `cargo test` build stays 0-vector-dep / 0-network (ADR-004/008). 0 backend change, 0 new dep (`qdrant-client` optional since task-18.4).
- **qdrant-recall CI job** (task-36.2, #237): a new `qdrant-recall` CI job runs the harness against a **qdrant/qdrant service container**, guarding live KNN recall permanently. CI run 26961084355 reports "qdrant ready after 1 attempt(s)" and "test result: ok. 2 passed; 0 failed" with **recall@10 = 1.0000** (N=2000, dim=64, M=50); also reproduced locally against a `qdrant/qdrant` docker container. This is a live-KNN correctness proof (exact-below-threshold, see caveat), not an HNSW-approximation stress test.
- **closeout** (task-36.3): **ADR-041 qdrant-live-vector-recall → Accepted** (per-D). **ADR-034** add-only Phase-36 Amendment marks its **D2 qdrant-server-lifecycle fulfilled** (live KNN recall measured + CI-guarded), NOT retro-editing the D-body (ADR-014 D5). **ADR-014 cross-validation gate — 27th activation**. Smoke v26 [45/45] (banner v25→v26, staging `cf-v28-cfg`, `TestTask363`).

```bash
# live qdrant KNN recall@10 vs BruteForce exact ground truth (skipped unless QDRANT_URL set)
QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture
```

详 `RELEASE_NOTES.md` v0.29.0 段 + [Phase 36 spec](docs/specs/phases/phase-36-qdrant-live-vector-recall.md) + [ADR-041](docs/decisions/adr-041-qdrant-live-vector-recall.md)。

## What's new in v0.28.0

📌 **v0.28.0 observability-hardening** — a focused, small version (third debt-cleanup, diminishing returns; honest over padding, ADR-013) that surfaces genuinely-swallowed errors in the hot paths, mirroring the repo's existing stderr conventions (Rust `eprintln!` / Go `fmt.Fprintf(os.Stderr)`). It is **observability-only**: best-effort contracts stay best-effort (indexing not blocked, query keeps skipping, daemon not blocked) and are **never** turned into fail-fast (ADR-004). The **default build stays 0-new-dependency, 0-network**; no new logging/metrics framework is introduced.

- **rust-silent-failure-surfacing** (task-35.1, #229): `index_session_backend`'s **four** `store.append` emit points (progress/index-error/commit-error/cancelled) change `let _ =` → `if let Err(persist_err) { eprintln!("WARN indexing-event persist failed …: {persist_err}") }` (SQLite persist failures — disk-full/lock — no longer swallowed; best-effort, indexing not blocked). `retriever/mod.rs:415`'s `Err(_) => continue` (Tantivy/SQLite desync) → `Err(e) => { eprintln!("WARN retriever: … desync …"); continue }` (skip preserved). `eb.send` stays as-is (no-subscribers is a normal broadcast condition, intentional). Mirrors `search.rs:108-113`. 0 new dep.
- **go-silent-failure-surfacing** (task-35.2, #230): `setVectorEnv`'s `config.Load` + `os.Setenv` failures now `fmt.Fprintf(os.Stderr, "contextforge: …")` (mirrors `daemon/rest.go:110`), guarded by `errors.Is(err, os.ErrNotExist)` so a MISSING config.toml — the normal default — stays silent and only a malformed/unreadable config warns. Best-effort preserved (env-only path unchanged on failure, daemon not blocked). 0 new dep.
- **7→3-4 grounding correction + closeout** (task-35.3): the survey's 7 candidates collapse to 3-4 genuinely-silent sites; four are dropped/left as-is with no code change (a grounding correction, the ADR-013 value): `search.rs:109` already WARNs (and core has no metrics facility, so a counter would be over-engineering) / `mcpadapter/server.go:298` already surfaced in task-31.3 / `allowlist.go:31` intentional POSIX-only platform caveat / `eb.send:193` intentional no-subscribers. `memstore.go:579` nil-sink is an honest non-issue (its only production wiring always calls `SetEventSink`). **No new metrics facility** is introduced. **ADR-040 → Accepted** (per-D). **ADR-031** add-only Phase-35 Amendment. **ADR-014 cross-validation gate — 26th activation**.

```bash
# rust: indexing-event persist best-effort guard + retriever desync skip guard
cargo test -p contextforge-core test_35_1
# go: setVectorEnv malformed→WARN (stderr-capture) / missing→no WARN / valid→no WARN
go test ./cmd/contextforge/ -run TestSetVectorEnv
```

详 `RELEASE_NOTES.md` v0.28.0 段 + [Phase 35 spec](docs/specs/phases/phase-35-observability-hardening.md) + [ADR-040](docs/decisions/adr-040-observability-hardening.md)。

## What's new in v0.27.0

📌 **v0.27.0 vector-config-completeness** — a focused, small version that completes the vector-backend config story opened by Phase 32: the factory now honors `CONTEXTFORGE_VECTOR_DIM` via dim auto-negotiation, the Go `[vector]` config section bridges to the core daemon's env, and `get_source_chunk` workspace isolation is re-grounded as already-present. The **default build stays 0-new-dependency, 0-network**; every change is add-only / default-preserving / opt-in, so existing v0.6–v0.26 clients + data are unaffected (ADR-004). Honest scope: the default `BruteForce` backend is dim-agnostic, so the default build accepts any dim and stays byte-equivalent — real dim enforcement bites only for dim-declaring feature backends (`[SPEC-DEFER:phase-future.vector-dim-feature-enforce]`); the Rust core keeps its 0-toml-dep rule (config parsing stays Go-side).

- **vector-dim auto-negotiation** (task-34.1, #224): `select_vector_backend` no longer silently discards `CONTEXTFORGE_VECTOR_DIM` — after constructing the backend it calls a new pure `negotiate_vector_dim(requested, declared)` (mirrors `embedding::factory::negotiate_dim`) reusing the existing `VectorError::DimMismatch`. The `VectorBackend` trait gains an add-only default `expected_dim() -> Option<usize>` returning `None` (dim-agnostic); `BruteForce` keeps `None`, so the default build accepts any dim byte-equivalently (ADR-004). Real enforcement is honest-deferred to dim-declaring feature backends (`[SPEC-DEFER:phase-future.vector-dim-feature-enforce]`). 0 new dep.
- **vector-backend config file** (task-34.2, #225): Go `config.Config` gains an add-only `[vector]` section (`Backend`/`Dim`); a `setVectorEnv(dataDir)` helper best-effort loads config and bridges `[vector]` → `CONTEXTFORGE_VECTOR_BACKEND`/`_DIM` for the spawned core daemon (wired into `doServe` + `doMCP`). **ENV WINS** (explicit env overrides config); no `[vector]` section → nothing exported → unset → `BruteForce` byte-equivalent (default unchanged, ADR-004). The Rust core keeps its 0-toml-dep rule (parsing stays Go-side). 0 new dep.
- **get_source_chunk workspace isolation + closeout** (task-34.3): a verify-only grounding correction — `get_source_chunk` already scopes candidates to `req.workspace_id` (shipped task-12.2, `search.rs`); TEST-34.3.1 builds a real 2-state index and asserts `workspace_id` set → that workspace only / cross-workspace → not-found / empty → aggregate. **ADR-039 → Accepted** (per-D; D1 dim-negotiation Accepted + feature-enforce honest-deferred). **ADR-037** add-only Phase-34 Amendment (completes its env-plumbing follow-up). **ADR-014 cross-validation gate — 25th activation**.

```bash
# vector-dim negotiate (four paths) + BruteForce any-dim byte-equiv
cargo test -p contextforge-core test_34_1
# Go [vector] config round-trip + setVectorEnv export / env-wins
go test ./internal/config/ -run TestTask342
go test ./cmd/contextforge/ -run TestSetVectorEnv
# get_source_chunk workspace isolation (set / cross / empty)
cargo test -p contextforge-core test_34_3
```

详 `RELEASE_NOTES.md` v0.27.0 段 + [Phase 34 spec](docs/specs/phases/phase-34-vector-config-completeness.md) + [ADR-039](docs/decisions/adr-039-vector-config-completeness.md)。

## What's new in v0.26.0

📌 **v0.26.0 governance-debt-cleanup-2** — a second wave of cross-phase governance-debt cleanup (mirrors Phase 31), tightening cache bounds, cache eviction policy, indexing-event durability, and trace-store workspace isolation. The **default build stays 0-new-dependency, 0-network**; every change is add-only / default-preserving / opt-in, so existing v0.6–v0.25 clients + data are unaffected (ADR-004). Honest scope: the L2 SQLite cache cap is an opt-in defense-in-depth ctor (no production call site yet), and indexing-replay e2e / strict multi-workspace trace isolation / true-LRU L2 / memory hard-delete cascade stay honestly deferred (ADR-013).

- **L2 + memstore cache bounding** (task-33.1/33.2, #218/#219): the embedding L2 SQLite cache gains a row-count cap + rowid-FIFO eviction (`with_sqlite_capacity`, `DEFAULT_L2_EMBEDDING_CACHE_CAP=50_000`, 0 schema migration via implicit rowid); the console-api memstore chunk/trace caches move from FIFO to access-order LRU (read-hit + existing-key overwrite both move-to-front). 0 new dep; the L2 cap is an opt-in ctor (defense-in-depth, no live leak), true-LRU L2 honest-deferred (`[SPEC-DEFER:phase-future.l2-cache-true-lru]`).
- **indexing-event persistence + trace workspace isolation** (task-33.3, #220): `indexing.*` events now persist add-only (migration `0019_indexing_events` + `SqliteIndexingEventStore`, best-effort persist in ADDITION to the unchanged `eb.send` broadcast) with a pure `indexing_rows_to_pb_events` replay mapper; the trace store carries add-only `workspace_id` on `GetSearchTrace`/`ListQueries` (empty `workspace_id` = aggregate-all, byte-equivalent, ADR-004). drain-timeout was already delivered in Phase 26 → verify-only. indexing-replay e2e + strict multi-workspace isolation honest-deferred (`[SPEC-DEFER:phase-future.indexing-replay-e2e]` / `[SPEC-DEFER:phase-future.tracestore-multi-workspace-strict]`).
- **export --timeout + closeout** (task-33.4): `export` gains an add-only `--timeout` flag (default `60s`, byte-equivalent to the old hardcoded `context.WithTimeout(60s)`, ADR-004). **ADR-038 → Accepted** (per-D). **ADR-031/027** add-only Phase-33 Amendments. **ADR-014 cross-validation gate — 24th activation**.

```bash
# L2 SQLite cap + rowid-FIFO eviction
cargo test -p contextforge-core test_33_1
# memstore chunk/trace access-order LRU
go test ./internal/consoleapi/ -run TestMemStore_CacheEviction
# indexing.* persistence round-trip + replay mapper
cargo test -p contextforge-core test_33_3
# export --timeout add-only flag (default 60s byte-equiv)
go test ./internal/cli/ -run TestParseExportOpts_Timeout
```

详 `RELEASE_NOTES.md` v0.26.0 段 + [Phase 33 spec](docs/specs/phases/phase-33-governance-debt-cleanup-2.md) + [ADR-038](docs/decisions/adr-038-governance-debt-cleanup-2.md)。

## What's new in v0.25.0

📌 **v0.25.0 vector-backend-config-plumbing-and-completeness** — completes the vector-backend story end-to-end: the two production hot paths (`server.rs` hybrid `:340` / semantic `:382`) now select a backend from env (`CONTEXTFORGE_VECTOR_BACKEND` + optional `CONTEXTFORGE_VECTOR_DIM`, mirroring `resolve_data_dir`), the factory gains a `"sqlite-vec"` arm, and the console search surface carries `vector_score` provenance. The **default build stays 0-new-dependency, 0-network**; unset/blank backend → `BruteForce` byte-equivalent (default behavior unchanged), and every change is add-only / default-preserving (ADR-004). Honest scope: an unknown / feature-off backend surfaces the factory's honest `Err` (no silent fallback, ADR-013); the sqlite-vec in-process recall/latency matrix cell, real chunk `source_type`/`agent_scope` filtering, a config-file backend source, and dim auto-negotiation stay honestly deferred (ADR-013).

- **backend config plumbing** (task-32.1, #212): `server.rs` hybrid (`:340`) + semantic (`:382`) resolve the backend via `resolve_vector_backend`/`parse_vector_backend` (reads `CONTEXTFORGE_VECTOR_BACKEND` + optional `CONTEXTFORGE_VECTOR_DIM`); unset/`""` → `BruteForce` byte-equivalent (default unchanged); unknown / feature-off → factory honest `Err` surfaced as `Status::internal` (no silent fallback, ADR-013). 0 new dep.
- **sqlite-vec factory arm** (task-32.2, #213): `select_vector_backend` gains a `"sqlite-vec"` arm (feature `vector-sqlite` double-half cfg gating, mirroring qdrant/lancedb) — feat on → `SqliteVecBackend::new()` / feat off → honest `Err` naming sqlite-vec + vector-sqlite. The default build verifies feat-off honest-Err + the selection-matrix wiring (factory 6/6); a **real x86_64-pc-windows-msvc `cargo test --features vector-sqlite` build PASSED** (arm wiring genuinely verified, not just structural). The in-process recall/latency cell is honest-deferred (`[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`). 0 new dep (sqlite-vec already optional).
- **console vector_score provenance + filter contract honesty** (task-32.3, #214): `console_data_plane.proto` `SearchResultItem` gains add-only `vector_score=16` (parity with v1 `RetrievalResult.vector_score=13`); the Rust producer sets it (cosine for vector hits, 0 for BM25), Go `grpcclient` maps it through to add-only `contractv1.SearchResult.VectorScore` (ADR-015 add-only). The misleading `retriever/mod.rs:325` WARN becomes an accurate no-op contract: the FROZEN `chunks` table has no `source_type`/`agent_scope` columns, so a real chunk filter is an import-path feature — honest-deferred (`[SPEC-DEFER:phase-future.chunk-source-type-filter]` / `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`). **ADR-037 → Accepted** (per-D; D2 sqlite-vec matrix cell honest-deferred). **ADR-034** add-only Phase-32 Amendment. **ADR-014 cross-validation gate — 23rd activation**.

```bash
# backend config plumbing: env name+dim parse + unset → brute-force byte-equiv
cargo test -p contextforge-core resolve_vector_backend
# sqlite-vec factory arm: feat-off honest-Err + selection matrix (factory 6/6)
cargo test -p contextforge-core select_vector_backend
cargo test -p contextforge-core --features vector-sqlite   # real MSVC feat-on build
# console vector_score carried Rust producer → Go grpcclient → contractv1
go test ./internal/grpcclient/ -run TestTask323
```

详 `RELEASE_NOTES.md` v0.25.0 段 + [Phase 32 spec](docs/specs/phases/phase-32-vector-backend-config-plumbing-and-completeness.md) + [ADR-037](docs/decisions/adr-037-vector-backend-config-plumbing-and-completeness.md)。

## What's new in v0.24.0

📌 **v0.24.0 governance-debt-cleanup** — clears cross-phase debt across observability, caching, deployment, eval, and export. The **default build stays 0-new-dependency, 0-network**; every change is add-only / default-preserving / opt-in, so existing v0.6–v0.23 clients + data are unaffected (ADR-004). Honest scope: real TLS cert issuance needs a domain (`docker compose config` parse verified; cert deferred), and multi-arch arm64 / GitHub-native attestation / a Rust-native eval runner stay honestly deferred (ADR-013).

- **observability + memstore parity** (task-31.1, #206): the Go fallback `MemMemoryStore` now emits `memory.*` events into the observability ring (parity with the workspace/job fallback + the Rust data plane). event-bus partition/capacity was already delivered in Phase 26 → verify-only + a roadmap correction (not re-implemented).
- **cache + deploy hardening** (task-31.2, #207): the embedding L1 cache is now LRU/cap-bounded (`BoundedCache`, 0 new dep) and the Go memstore cache cap is env-configurable (`CONTEXTFORGE_CONSOLEAPI_CACHE_CAP`); the production compose gains `mem_limit`/`cpus` + an optional `tls` profile (Caddy reverse proxy). `docker compose config` + `--profile tls config` parse verified.
- **eval subtable + exporter full-content + MCP nits** (task-31.3, #208): per-case eval results become a queryable subtable (`eval_case_results`, migration 0018); the exporter fills real content + a real `ContentHash` via the new add-only `ListAllChunks` RPC (was content=""); 3 MCP nits fixed (protocol-version date parsing, audit-write error surfacing, allowlist file-mode warning). **ADR-036 → Accepted** (per-D). **ADR-021/027/029/033** add-only Phase-31 Amendments. **ADR-014 cross-validation gate — 22nd activation**.

```bash
# memstore event parity + event-bus verify-only
go test ./internal/consoleapi/ -run TestMemMemoryStore_EventParity
cargo test -p contextforge-core data_plane::events   # 6 passed (verify-only, Phase 26)
# eval per-case subtable + exporter full-content RPC
cargo test -p contextforge-core eval::store          # 12 passed
go test ./internal/exporter/ ./internal/mcpadapter/
```

详 `RELEASE_NOTES.md` v0.24.0 段 + [Phase 31 spec](docs/specs/phases/phase-31-governance-debt-cleanup.md) + [ADR-036](docs/decisions/adr-036-governance-debt-cleanup.md)。

## What's new in v0.23.0

📌 **v0.23.0 cjk-true-segmenter** — upgrades the 0-dep overlapping-bigram CJK analyzer (`配置加载`→`配置`/`置加`/`加载`) to a **feature-gated true-word segmenter** (`cjk-segmenter`, jieba-rs: `配置加载`→`配置`/`加载`), keeping bigram as the 0-dep fallback. The **default build stays 0-new-dependency** — jieba is not compiled at default features (ADR-004); default `content` tokenization + 6-field schema are unchanged. Honest scope: on this small golden corpus the true segmenter shows **no file-level recall gain over bigram** (+0.0000), recorded truthfully (ADR-013); the value is cleaner tokens, not measurable recall at this scale.

- **jieba true-word analyzer + dual-site register** (task-30.1, #202): `cjk-segmenter` feature (jieba-rs 0.7, optional, default off) adds a parallel `cjk_segmenter` analyzer registered at both the index site (`IndexSession::open_with_tokenizer`) and the query site (`Retriever::open_with_config`) — asymmetry would silently degrade recall (task-24.1 R4). Bigram `code_cjk` stays as the 0-dep fallback.
- **reindex migration tool + tokenizer-default-on eval** (task-30.2, #203): `IndexSession::reindex_with_tokenizer` rebuilds a collection's Tantivy index under a new analyzer binding (binding persists in `meta.json`, so switching requires re-index), reading SQLite chunk content as source of truth. `RetrieverConfig.tokenizer` is documented as vestigial (schema-driven, 方案 B). The full default-on flip is honest-deferred (`[SPEC-DEFER:phase-future.tokenizer-default-on]`) — the migration tool is ready, but flipping the default is a product decision.
- **real recall delta** (task-30.2): extended golden (11→16 CJK cases) measured default/bigram/segmenter — **segmenter vs bigram delta = +0.0000** (both fully recall CJK cases at file level), both +0.125 over default. **ADR-035 → Accepted** (per-D; D3 default-flip honest-defer). **ADR-029** add-only Phase-30 Amendment. **ADR-014 cross-validation gate — 21st activation**.

```bash
# jieba true-word CJK analyzer (配置加载 → 配置/加载, vs bigram 配置/置加/加载)
cargo test -p contextforge-core --features cjk-segmenter --lib test_30_1      # 2 passed
# real recall delta: default vs bigram vs true segmenter
cargo run -p contextforge-core --features cjk-segmenter --example phase24_tokenizer_recall
# reindex an existing index to a new analyzer binding (default build, 0-dep)
cargo test -p contextforge-core --lib test_30_2_2
```

详 `RELEASE_NOTES.md` v0.23.0 段 + [Phase 30 spec](docs/specs/phases/phase-30-cjk-true-segmenter.md) + [ADR-035](docs/decisions/adr-035-cjk-true-segmenter-and-tokenizer-default.md)。

## What's new in v0.22.0

📌 **v0.22.0 live-vector-recall** — redeems Phase 25's qdrant/lancedb contract & parameter layers into **real live vector recall**, and factory-injects the real backend into the production hot path (`core/src/server.rs` previously hardcoded `BruteForceVectorBackend` at the hybrid `:302` / semantic `:341` paths). The **default build stays 0-new-dependency, 0-network** — the default semantic+hybrid path still runs the 0-dep `BruteForceVectorBackend` (ADR-004 / ADR-023 D5). Honest scope: **qdrant live KNN honest-defers when no server is running** (`health()==Unreachable` → exit 0, no fabricated recall, ADR-013); lancedb real ANN indexes are feature-gated and verified `--lib` scoped.

- **vector backend factory + hot-path injection** (task-29.1, #197): `select_vector_backend(name, dim) -> Result<Arc<dyn VectorStore>, VectorError>` mirrors `embedding::factory::select_provider` — `""`/`"brute"` → BruteForce (0-dep, byte-equivalent), `"qdrant"`/`"lancedb"` feature-gated (honest `Err` otherwise). An add-only combined trait `VectorStore: VectorIndexer + VectorSearcher` lets one handle both index and search; the three base trait signatures are unchanged. `server.rs:302/341` now inject via the factory. factory 4/4 + `cargo test --workspace` 191 lib + integration 0 failed.
- **qdrant live KNN harness + honest-defer** (task-29.2, #198): `core/examples/phase29_recall_via_qdrant.rs` (feature `vector-qdrant`+`embedding-fastembed`) runs connect→ensure-create→upsert→KNN through the production `Retriever::search_semantic` path against a real single-node qdrant; no server → `health()==Unreachable` → eprintln + exit 0 with zero fabricated recall (real recall backfilled from a dev-box server).
- **lancedb real ANN index + compaction + backend matrix** (task-29.3, #199): `LanceDbBackend::create_ann_index` builds real `Index::IvfPq` / `Index::IvfHnswSq` via Lance `create_index`; `compact()` runs real `OptimizeAction::All`. Measured (n=1024, dim=384): **IVF_HNSW_SQ recall@10≈0.90 (~0.25 s build, ~3.5 ms/q)**, IVF_PQ≈0.44, brute-force exact fastest at modest n. **ADR-034 → Accepted** (per-D; D2 live-server honest-defer partial). **ADR-030 / ADR-023** add-only Phase-29 Amendment (real matrix). **ADR-014 cross-validation gate — 20th activation**.

```bash
# vector backend factory contract (deterministic, no server)
cargo test -p contextforge-core --lib retriever::vector::factory      # 4 passed
# qdrant live KNN harness (no server → honest-defer exit 0, no fabricated recall)
cargo run -p contextforge-core --example phase29_recall_via_qdrant --features vector-qdrant,embedding-fastembed
# lancedb real IVF_PQ/IVF_HNSW_SQ index + compaction + recall matrix (--lib scoped)
cargo test -p contextforge-core --features vector-lancedb --lib retriever::vector::lance_db -- --nocapture
```

详 `RELEASE_NOTES.md` v0.22.0 段 + [Phase 29 spec](docs/specs/phases/phase-29-live-vector-recall.md) + [ADR-034](docs/decisions/adr-034-production-vector-live-recall.md)。

## What's new in v0.21.0

📌 **v0.21.0 release-ci-hardening** — hardens the release / CI pipeline. **All changes are CI/release config** (plus surgical clippy/gofmt fixes); the **image runtime + default 0-network / 0-dependency baseline are unchanged** (ADR-004). Adds an **anonymous-pull guard** (regression protection for public GHCR pullability), **supply-chain proof** (cosign keyless signature + SPDX SBOM attestation + SLSA provenance), and a **strict CI lint gate** (clippy + gofmt + go vet, blocking). Honest scope: **multi-arch arm64 is DEFERRED** (QEMU emulation infeasible — build timed out), and the **real GHCR signing happens at the v0.21.0 release run** (the mechanism is CI-verified end-to-end).

- **anonymous-pull guard + multi-arch (deferred)** (task-28.1): `verify-image.yml` adds an unauthenticated (logged-out) `docker pull` step asserting the GHCR package is publicly pullable — guarding the v0.10.0 `PRIVATE → 403` regression (run 26788773926 ✅). multi-arch (linux/arm64) was attempted but **deferred**: arm64 QEMU emulation build is infeasible (run 26757640892 cancelled at 45 min, still compiling Rust deps); `release.yml` stays single-arch `linux/amd64`. arm64 → `[SPEC-DEFER:phase-future.multi-arch-native-runner]` (native runner / cross-compile).
- **supply-chain proof: cosign + SBOM + provenance** (task-28.2): `release.yml` keyless-signs the image digest (`cosign sign`), attests an SPDX SBOM (syft → `cosign attest`), and attaches SLSA provenance (`provenance: mode=max`); `verify-image.yml` runs `cosign verify` + `cosign verify-attestation`. GitHub-native attestation (`actions/attest-*`) is unavailable on user-owned private repos, so cosign (public Sigstore + GHCR OCI artifacts, repo-visibility-independent) is used. Mechanism verified end-to-end against a local registry (run 26799480280 ✅); real GHCR signing lands at the v0.21.0 release run.
- **strict CI lint gate** (task-28.3): `ci.yml` adds a `lint` job — `cargo clippy --workspace --all-targets -- -D warnings` + `gofmt` check + `go vet`, all blocking. Backlog measured first (CI/LF authoritative: gofmt 15 / go vet 0 / clippy ~33) then fixed (clippy `-D warnings` clean, `cargo test` 187 passed). **ADR-033 → Accepted** (D1 arm64 deferred / D2 cosign mechanism-verified, real sign at release / D3 lint gate green). **ADR-007** add-only Amendment (distribution surface extended). **ADR-014 cross-validation gate — 19th activation**.

```bash
# CI strict-lint gate (clippy + gofmt + go vet, all blocking)
cargo clippy --workspace --all-targets -- -D warnings
gofmt -l . && go vet ./...
# supply-chain verify (against a real v0.21.0+ signed image)
cosign verify ghcr.io/tajiaoyezi/contextforge-daemon:v0.21.0 \
  --certificate-identity-regexp '^https://github.com/tajiaoyezi/contextforge/.github/workflows/release.yml@.*$' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
```

详 `RELEASE_NOTES.md` v0.21.0 段 + [Phase 28 spec](docs/specs/phases/phase-28-release-ci-hardening.md) + [ADR-033](docs/decisions/adr-033-release-ci-hardening.md)。

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

(`hybrid` is opted into via `SearchRequest.hybrid` / `eval run --hybrid`; reranking is wired via `Retriever::with_reranker`. As of **v0.32.0 (Phase 39)** the console-api `?hybrid=true` REST forward is live — `POST /v1/search?hybrid=true` reaches the core hybrid path and the response carries `retrieval_method="hybrid"` + `hybrid_score`. Reranking stays server-side env-driven (`CONTEXTFORGE_RERANKER_PROVIDER`); its `reason` provenance is visible in the REST response, while a per-request `?rerank` flag is superseded by the env-driven model — see ADR-044.)

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

## Run the released image

Prebuilt, signed images are published to GHCR on every `v*` tag — no build
required. The current stable tag is **`v0.28.0`** (`linux/amd64`). The image
bundles both binaries (`contextforge-core` Rust data-plane + `contextforge` Go
control-plane); its default command is `console-api-serve` on port `48181`.

```bash
# Production: two-process stack (Rust core + Go console-api), data persisted.
# Defaults to v0.28.0 — override with CONTEXTFORGE_VERSION. See docs/deploy/production.md.
docker compose -f deploy/docker-compose.production.yml up -d
curl -fsS http://localhost:48181/v1/health | jq .   # -> {"status":"healthy",...}

# Dev / PoC: single container, in-memory fallback — serves immediately, NOT persistent.
docker run --rm -p 48181:48181 \
  -e CONSOLE_API_FALLBACK_INMEM=1 \
  ghcr.io/tajiaoyezi/contextforge-daemon:v0.28.0
```

Without `CONSOLE_API_FALLBACK_INMEM=1` (and no reachable `contextforge-core`),
`/v1/health` honestly reports `503 degraded` with an actionable `error_reason` —
start the core daemon or use the production compose stack.

Each release image is cosign keyless-signed and ships an SPDX SBOM + SLSA
provenance attestation; verify the exact digest before deploying (command + the
per-release digest live in [`docs/deploy/production.md`](docs/deploy/production.md)
§2 and `docs/releases/v0.28.0-evidence.md`). The release artifact **is** the signed
GHCR image — this repo does not publish a GitHub Release object or source tarball.

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
# 1. Build (Go 1.26+, Rust stable).
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
- The published release artifact is the **signed GHCR image**
  (`ghcr.io/tajiaoyezi/contextforge-daemon:vX.Y.Z`, cosign keyless + SBOM +
  provenance) built by `.github/workflows/release.yml` on each `v*` tag — this
  repo does **not** publish a GitHub Release object or source tarball. Release /
  quickstart smoke gates (`scripts/release_smoke.sh`,
  `scripts/quickstart_smoke.sh`) run as part of the pipeline.

## Where to go next

- `contextforge.example.toml` — starting point for collection allowlists and
  local-only provider settings.
- [`docs/specs/phases/phase-9-cli-pipeline.md`](docs/specs/phases/phase-9-cli-pipeline.md) — the v0.2 CLI data-plane phase that re-enabled the manual
  Quick Start sequence above.
- [`docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`](docs/decisions/adr-013-cli-data-plane-grpc-bridge.md) — context on why
  v0.1's `contextforge index` / `contextforge import` were stubs and what
  Phase 9 changed.
