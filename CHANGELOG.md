# Changelog

All notable changes to ContextForge are documented in this file. The format is based on [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/).

> For the full per-version release notes (What shipped / ADR / Upgrade path / Rollback path / supply-chain evidence), see [RELEASE_NOTES.md](RELEASE_NOTES.md). This file is the condensed user-facing changelog; RELEASE_NOTES.md is the detailed maintainer-facing record.

## [Unreleased]

### Added
- Phase 46 / v0.39.0 (in progress): README restructure (Features summary + maturity label), CHANGELOG.md (this file), docs/decisions/README.md (ADR visitor index), release.yml GitHub Release object auto-creation.

## [v0.38.0] — 2026-07-01 — v1.0-api-cli-freeze

v1.0 收口冲刺第一步: ADR-050 formally defines v1.0.0 (capability maturity D1 + API/CLI freeze D2 + docs alignment D3 Phase 46 + GitHub Release D4 Phase 46-47).

### Added
- CLI `version` subcommand + top-level `--help`/`-h` (fixes the task-1.4 `-h` → exit 2 hostility).
- `contextforge.example.toml` 补全 4 retrieval sections (`[embedding]`/`[vector]`/`[reranker]`/`[retrieval]`).
- ADR-050 v1.0-definition (Proposed; D1/D2 partially ratified).

### Removed
- **[BREAKING]** daemon REST removed 2 v0.1 §2A 501 未实装 endpoints (`POST /v1/import` + `POST /v1/eval/run`) — console-api covers both. v1.0-pre major-boundary breaking change.

### Changed
- `chunk_count` in `/v1/collections` honest-defer (stays 0 as v1.0 known limitation; Go daemon has no SQLite lib; real counts via console-api `/v1/stats/chunks`).

## [v0.37.0] — 2026-07-01 — memory-unpin-actor-propagation

Closes the pin/unpin actor-propagation asymmetry: `emit_audit_and_event` gains an `actor` param (audit/event source attribution); `unpin` propagates + `pin` 顺带 closes the loop.

### Added
- proto `UnpinMemoryRequest` add-only `actor=2` + Go `Unpin(id, actor)` + `X-Actor` header read on unpin (mirrors pin).
- ADR-049 ratified.

## [v0.36.0] — 2026-07-01 — governance-debt-cleanup-4 / indexing-replay-splice

Fourth governance-debt sweep: splices the indexing-event replay mapper into the live `subscribe` path (4 splice gaps closed). `since_ts>0` reconnecting subscribers now receive missed `indexing.progress`/`.cancelled`/`.error` events.

### Added
- `list_since(limit, since_ts)` + `DataPlaneStores.indexing_event_store` field + `serve_full` wiring + subscribe splice.
- ADR-048 ratified.

## [v0.35.0] — 2026-06-07 — chunk-source-type-filter

Turns the chunk-search `source_type` filter from a documented no-op into a real filter (derived from `file_path`, 0 schema migration).

### Added
- `classify_source_type(file_path)` deterministic derivation + populate on every hit + BM25 post-filter + console-api `?source_type=` forward (proto add-only `source_type=9`).
- ADR-047 ratified.

### Changed
- `agent_scope` chunk filter honest-deferred (memory-layer concept, chunks carry no agent dimension) — stays documented no-op.

## [v0.34.0] — 2026-06-07 — tokenizer-default-on

First deliberate default-behavior change: the code/CJK-aware analyzer `code_cjk` flips from opt-in to the production default for newly created collections.

### Changed
- New collection default tokenizer: `TEXT` → `code_cjk` (NOT byte-equivalent). Real recall delta +0.1250 (0.8750 → 1.0000).
- Existing collections unaffected (schema-driven safety); opt-out via `CONTEXTFORGE_TOKENIZER=default`.
- ADR-046 ratified.

## [v0.33.0] — 2026-06-07 — governance-debt-cleanup-3

Third governance-debt sweep: memory pin actor propagation + L2 embedding cache access-order LRU.

### Added
- `PinMemoryRequest.actor=3` add-only + Go `Pin(id,pin,actor)` + `X-Actor` header → store `pinned_by`.
- L2 SQLite embedding cache hit-bump (access-order LRU, reusing implicit rowid, 0 schema migration).
- ADR-045 ratified.

## [v0.32.0] — 2026-06-06 — console-api-retrieval-signal-forward

Plumbs the hybrid (BM25+vector RRF) retrieval signal out to the public REST surface: `POST /v1/search?hybrid=true`.

### Added
- console_data_plane proto add-only (`SearchRequest.hybrid=8` + `SearchResultItem.hybrid_score=17`) + data-plane hybrid dispatch + Go forwarding.
- Rerank provenance visibility (reranker stays env-driven; `?rerank` per-request superseded).
- ADR-044 ratified.

## [v0.31.0] — 2026-06-06 — embedding-remote-reranker-live

First end-to-end against a REAL remote cross-encoder reranker + real rerank quality measurement + Go `[reranker]` config bridge + data-plane `with_reranker` opt-in wiring.

### Added
- `RemoteRerankerProvider` (ureq-backed) + `select_reranker` factory + `reranker-remote` feature + env-gated live-quality harness.
- Real measured: MRR=1.0000 / recall@1=1.0000 (small author-labeled 14-case set).
- ADR-043 ratified.

## [v0.30.0] — 2026-06-06 — embedding-provider-remote-live

First end-to-end against a REAL remote embedding endpoint + real semantic recall measurement + Go `[remote]` config bridge.

### Added
- Remote `EmbeddingProvider` env-gated live-recall harness + Go `[remote]` env-bridge.
- Real measured: recall@3=1.0000 (small author-curated 15-case/16-doc set).
- ADR-042 ratified.

## [v0.29.0] — 2026-06-04 — qdrant-live-vector-recall

Env-gated live qdrant KNN recall harness + CI service-container permanent guard (closes the CI-no-server defer).

### Added
- `core/tests/qdrant_live_recall.rs` + qdrant-recall CI job.
- Real CI-measured: recall@10=1.0000 vs BruteForce exact KNN ground truth.
- ADR-041 ratified.

## [v0.28.0] — 2026-06-04 — observability-hardening

Surfaces genuinely-swallowed hot-path errors via stderr (best-effort stays best-effort, not fail-fast).

### Changed
- `index_session_backend` store.append ×4 + retriever Tantivy/SQLite desync → `eprintln!` WARN.
- `setVectorEnv` config errors → `fmt.Fprintf(os.Stderr)`.
- 7→3-4 grounding correction.
- ADR-040 ratified.

## [v0.27.0] — 2026-06-03 — vector-config-completeness

### Added
- Vector dim auto-negotiation `negotiate_vector_dim` + `[vector]` config Go→core env bridge + `get_source_chunk` workspace isolation.
- ADR-039 ratified.

## [v0.26.0] — 2026-06-03 — governance-debt-cleanup-2

### Added
- L2 SQLite cache rowid-FIFO bounding + console memstore access-order LRU + memory hard-delete non-issue grounding + indexing.* event persistence/replay (migration 0019) + TraceStore workspace isolation + export `--timeout`.
- ADR-038 ratified.

## [v0.25.0] — 2026-06-03 — vector-backend-config-plumbing-and-completeness

### Added
- `server.rs` hot-path env-config injection + sqlite-vec factory arm + console `vector_score` provenance + retrieval-filter contract honesty.
- ADR-037 ratified.

## [v0.24.0] — 2026-06-03 — governance-debt-cleanup

### Added
- memstore-event parity + event-bus config (verify-only) + cache LRU/cap + compose hardening + eval per-case subtable + exporter full-content + 3 MCP nits.
- ADR-036 ratified.

## [v0.23.0] — 2026-06-03 — cjk-true-segmenter

### Added
- jieba true-word CJK analyzer (feature-gated) + dual-site registration + reindex migration tool + real CJK recall delta.
- ADR-035 ratified.

## [v0.22.0] — 2026-06-03 — live-vector-recall

### Added
- Vector backend factory + `server.rs` hot-path injection + qdrant live KNN (honest-defer without server) + lancedb real IVF_PQ/IVF_HNSW_SQ index + compaction + selection matrix.
- ADR-034 ratified.

## [v0.21.0] — 2026-06-02 — release-ci-hardening

### Added
- anonymous-pull guard + cosign keyless signing/SBOM/provenance + CI strict-lint (clippy + gofmt + go vet, blocking).
- ADR-033 ratified.

## [v0.20.0] — 2026-06-01 — memory-ops-hardening

### Added
- pin-actor + pinned-at-timestamp + Pin/Unpin RPC split + hard-delete with X-Confirm + is_pinned audit backfill.
- ADR-032 ratified.

## [v0.19.0] — 2026-06-01 — observability-hardening

### Added
- TraceStore FTS5 full-text search + VACUUM/prune + events SSE real-time push + replay from audit + event-bus config.
- ADR-031 ratified.

## [v0.18.0] — 2026-06-01 — production-vector-backend

### Added
- qdrant server lifecycle layer + lancedb buildability + production backend selection matrix.
- ADR-030 ratified.

## [v0.17.0] — 2026-05-31 — code-and-cjk-tokenizer-and-eval-hardening

### Added
- opt-in code/CJK tokenizer (camelCase/snake_case split + CJK bigram) + eval dataset validator + golden dataset expansion.
- ADR-029 ratified.

## [v0.16.0] — 2026-05-31 — vector-persistence-and-cross-platform

### Added
- hnsw graph persistence round-trip (feature-gated) + sqlite-vec Windows MSVC build.
- ADR-028 ratified.

## [v0.15.0] — 2026-05-31 — embedding-provider-completion

### Added
- embedding provider config selection + cache wrapping + remote skeleton (feature-gated) + init scaffolds `[embedding]` section.
- ADR-027 ratified.

## [v0.14.0] — 2026-05-31 — retrieval-quality

### Added
- hybrid scoring (BM25 + semantic RRF fusion) + reranker provider (IdentityReranker default + CrossEncoderReranker feature-gated) + real dogfood eval.
- ADR-025 / ADR-026 ratified.

## [v0.13.0] — 2026-05-31 — semantic-retrieval-throughline

### Added
- semantic retrieval throughline to console-api (`?semantic=true`) + real recall via production Retriever.
- ADR-024 ratified.

## [v0.12.0] — 2026-05-30 — vector-retrieval-integration

### Added
- end-to-end semantic search (vector retrieval integration).
- ADR-023 ratified.

## [v0.11.0] — 2026-05-30 — vector-backend-selection

### Added
- vector backend infrastructure + spike (ADR-023 Proposed).

## [v0.10.0] — 2026-05-28 — is-pinned-amendment

### Added
- Console is-pinned field (backlog 11/11 = 100% closed).

## [v0.9.0] — 2026-05-28 — v0.9.0-backlog-completion

### Added
- 10/11 Console backlog closed + release infra.

## [v0.8.0] — 2026-05-26 — Console functional gap closure

### Added
- Console functional gap closure (6/11 backlog) + health component breakdown (ADR-020) + memory event bus bridge (ADR-021).

## [v0.7.2] — 2026-05-26 — fallback-inmem default reversal ⚠️ BREAKING

### Changed
- **[BREAKING]** Silent in-memory fallback default reversed to 503 (forces opt-in via `CONSOLE_API_FALLBACK_INMEM=1`); degraded state now exposed, not masked by HTTP 200.

## [v0.7.1] — 2026-05-26 — Dockerfile + single-image deployment fix

### Fixed
- Dockerfile + single-image deployment.

## [v0.7.0] — 2026-05-24 — Console 22-endpoint conformance 100% PASS

### Added
- Console 22-endpoint conformance 100% PASS.

## [v0.6.0] — 2026-05-24

Console contract v1 expansion (search/chunks/collections/memory waves).

## [v0.5.0] — 2026-05-24

Console contract v1 initial implementation.

## [v0.4.0] — 2026-05-25 — cross-process-rust-go-via-grpc-bridge

### Added
- console-api-serve as thin REST→gRPC translator (reuses ADR-013 gRPC bridge + 4 new services).
- ADR-016 ratified.

## [v0.3.0] — 2026-05-24 — console-contract-v1-compatibility

### Added
- Console Contract v1 compatibility layer (9 REST endpoints + Go contractv1 17 types).
- ADR-015 ratified.

## [v0.2.0] — 2026-05-24

### Added
- CLI data-plane pipeline (Phase 9: import/index/search re-enabled; fixed v0.1 spec drift).
- ADR-013 ratified.

## [v0.1.0] — 2026-05-23

### Added
- Initial release: Go control-plane CLI + Rust data-plane daemon (ADR-001), SQLite+Tantivy layered storage (ADR-002), three interfaces CLI/REST/MCP (ADR-003), local-first privacy baseline (ADR-004), read-only import + draft export (ADR-005), recall eval acceptance gate (ADR-006), minimal tarball distribution (ADR-007).
