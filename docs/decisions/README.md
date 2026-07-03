# ADR Index — Architecture Decision Records

This directory holds ContextForge's Architecture Decision Records (ADRs) — immutable records of significant technical decisions, their context, and consequences. Each ADR follows the s2v full-standard §16.2 template.

> **How to add an ADR**: run `/s2v-add adr <title>` (creates `adr-NNN-<title>.md` with the next free number). Never retro-edit a ratified ADR's decision body — use add-only Amendments (ADR-014 D5).
>
> **Status values**: `Proposed` (draft, not yet ratified) → `Accepted` (ratified, in force) → `Deprecated` / `Superseded`. Several ADRs carry add-only Amendments from later phases (noted inline); the ADR's own Status reflects its latest ratification.

49 ADRs total (001–050; adr-019 was skipped). Grouped by category below.

---

## Architecture

| # | Title | Status | Summary |
|---|---|---|---|
| [001](adr-001-go-rust-dual-binary-architecture.md) | go-rust-dual-binary-architecture | Accepted | Split into Go control-plane CLI + Rust data-plane daemon, communicating via local gRPC. |
| [008](adr-008-core-library-selection.md) | core-library-selection | Accepted | Rust = tantivy/tree-sitter/pulldown-cmark/tokio/tonic/rusqlite; Go = cobra/chi/grpc-go/slog. |
| [016](adr-016-cross-process-rust-go-via-grpc-bridge.md) | cross-process-rust-go-via-grpc-bridge | Accepted | v0.4 reuses the ADR-013 gRPC bridge pattern + 4 new services, making console-api-serve a thin REST→gRPC translator. |

## Storage & Retrieval

| # | Title | Status | Summary |
|---|---|---|---|
| [002](adr-002-sqlite-tantivy-layered-storage.md) | sqlite-tantivy-layered-storage | Accepted | Layered local storage: SQLite (metadata/chunk/provenance) + Tantivy full-text + optional vector backend. |
| [023](adr-023-vector-backend-default.md) | vector-backend-default | Accepted | sqlite-vec default + hnsw fallback + feature-gated qdrant/lancedb, ratified on real embedding recall. |
| [026](adr-026-reranker-provider.md) | reranker-provider | Accepted | Reranker trait + deterministic IdentityReranker default (0-dep) + feature-gated CrossEncoderReranker. |
| [027](adr-027-embedding-provider-abstraction.md) | embedding-provider-abstraction | Accepted | Configurable embedding provider abstraction: config selection + dim negotiation + cache + remote skeleton. |
| [028](adr-028-vector-persistence-strategy.md) | vector-persistence-strategy | Accepted | hnsw graph serialization (rebuild-on-load fallback), sqlite-vec Windows MSVC, incremental-index evaluation. |
| [029](adr-029-code-and-cjk-tokenizer-and-eval-hardening.md) | code-and-cjk-tokenizer-and-eval-hardening | Accepted | Custom code/CJK tokenizer (camelCase/snake_case + CJK bigram) opt-in + eval dataset validator + golden expansion. |
| [030](adr-030-production-vector-backend.md) | production-vector-backend | Accepted | qdrant server lifecycle layer + lancedb buildability/index tuning + selection matrix. |
| [034](adr-034-production-vector-live-recall.md) | production-vector-live-recall | Accepted | Vector backend factory + server.rs hot-path; qdrant live KNN; lancedb real ANN; selection matrix. |
| [035](adr-035-cjk-true-segmenter-and-tokenizer-default.md) | cjk-true-segmenter-and-tokenizer-default | Accepted | CJK true segmenter behind feature gate (jieba/lindera) + 0-dep bigram fallback + tokenizer-default-on evaluation. |
| [037](adr-037-vector-backend-config-plumbing-and-completeness.md) | vector-backend-config-plumbing-and-completeness | Accepted | Vector backend config plumbing (env→server.rs) + sqlite-vec factory arm + console vector_score provenance. |
| [039](adr-039-vector-config-completeness.md) | vector-config-completeness | Accepted | Factory dim-negotiation + Go→env config-file bridge + get_source_chunk workspace isolation grounding-correction. |
| [041](adr-041-qdrant-live-vector-recall.md) | qdrant-live-vector-recall | Accepted | Qdrant live vector recall: env-gated live recall harness + CI service-container permanent guard. |
| [042](adr-042-embedding-provider-remote-live.md) | embedding-provider-remote-live | Accepted | Remote embedding live: env-gated live semantic recall harness + Go [remote] config bridge; first real e2e. |
| [043](adr-043-embedding-remote-reranker-live.md) | embedding-remote-reranker-live | Accepted | Remote reranker live: RemoteRerankerProvider + select_reranker factory + Go [reranker] bridge + data-plane wiring. |
| [046](adr-046-tokenizer-default-on.md) | tokenizer-default-on | Accepted | First intentional default-behavior change: production tokenizer default flipped to code_cjk + opt-out + existing safe. |
| [047](adr-047-chunk-source-type-filter.md) | chunk-source-type-filter | Accepted | chunk source_type filter: deterministic file_path derivation (0 migration) + v1 post-filter + console forward. |

## Interfaces (CLI / REST / MCP / gRPC)

| # | Title | Status | Summary |
|---|---|---|---|
| [003](adr-003-cli-rest-mcp-grpc-interfaces.md) | cli-rest-mcp-grpc-interfaces | Accepted | Three external interfaces (CLI / local REST /v1/* / MCP tools) + one internal Go↔Rust local gRPC channel. |
| [013](adr-013-cli-data-plane-grpc-bridge.md) | cli-data-plane-grpc-bridge | Accepted | Fixes v0.1 spec drift (import/index not-implemented) by adding rpc Index + wiring CLI→Rust gRPC data plane. |
| [015](adr-015-console-contract-v1-compatibility.md) | console-contract-v1-compatibility | Accepted | Console Contract v1 compatibility layer (9 REST endpoints + Go contractv1 17 types + workspace/index-job). |
| [017](adr-017-console-contract-completion-22-endpoint.md) | console-contract-completion-22-endpoint | Accepted | Completes Console Contract from 9 to 22 endpoints (Memory/Eval/SourceChunk/SearchTrace waves). |
| [024](adr-024-console-api-semantic-forward.md) | console-api-semantic-forward | Accepted | console-api /v1/search adopts add-only semantic forwarding (contractv1 Semantic field + query-param OR-merge). |
| [025](adr-025-hybrid-scoring-fusion.md) | hybrid-scoring-fusion | Accepted | Adds independent search_hybrid using RRF of BM25 + semantic paths as default fusion (top-1 0.0333→0.6667). |
| [044](adr-044-console-api-retrieval-signal-forward.md) | console-api-retrieval-signal-forward | Accepted | console-api retrieval signal forward: hybrid ?hybrid flag + rerank provenance visibility via proto add-only. |

## Release & Distribution

| # | Title | Status | Summary |
|---|---|---|---|
| [007](adr-007-minimal-tarball-distribution.md) | minimal-tarball-distribution | Accepted | v0.1 minimal distribution: GitHub Release tarball + source self-host + Docker Compose. (Amended by ADR-050: v1.0 narrowed to pragmatic closure; GitHub Release object added Phase 46.) |
| [033](adr-033-release-ci-hardening.md) | release-ci-hardening | Accepted | Multi-arch manifest + anonymous-pull guard + keyless supply-chain attestation (cosign/SBOM) + strict lint. |
| [050](adr-050-v1.0-definition.md) | v1.0-definition | Proposed (partial D1/D2 ratified) | v1.0.0 = capability maturity + API/CLI freeze + docs alignment + GitHub Release process; explicit known-limitations list. |

## Governance & Process

| # | Title | Status | Summary |
|---|---|---|---|
| [004](adr-004-local-first-privacy-baseline.md) | local-first-privacy-baseline | Accepted | Local-first privacy: default no upload, denylist sensitive paths, secret redaction, remote opt-in, audit logging. |
| [005](adr-005-readonly-import-draft-export.md) | readonly-import-draft-export | Accepted | Read-only import + draft/bundle export; never writes back to third-party agent memory. |
| [006](adr-006-recall-eval-acceptance-gate.md) | recall-eval-acceptance-gate | Accepted | recall eval as a first-class PRD acceptance gate: tests + `contextforge eval run` on golden questions. |
| [009](adr-009-provenance-timestamp-placeholder.md) | provenance-timestamp-placeholder | Accepted | v0.1 uses epoch placeholder for proto Provenance time fields, avoiding chrono/time dependency. |
| [010](adr-010-audit-cross-language-unification.md) | audit-cross-language-unification | Proposed | v0.1 keeps the Rust SQLite + Go JSON-lines dual-track audit without forcing unification (research record). |
| [011](adr-011-single-driver-with-subagents.md) | single-driver-with-subagents | Proposed | Governance moves to single-driver + internal subagent variant, retiring external worker terminals. |
| [012](adr-012-main-agent-governance-autonomy.md) | main-agent-governance-autonomy | Accepted | Main-agent governance autonomy for bounded execution decisions (preflight/merge/dep/Waive) without separate confirmation. |
| [014](adr-014-cross-phase-exit-criteria-validation.md) | cross-phase-exit-criteria-validation | Accepted | Phase Exit Criteria ↔ Task §6 AC bidirectional cross-check (mapping table + lint) enforced from Phase 10. |
| [018](adr-018-fallback-inmem-default-reversal.md) | fallback-inmem-default-reversal | Proposed | v0.7.2 reverses silent in-memory fallback default to 503 (forces opt-in) so degraded state is exposed. |
| [020](adr-020-health-component-breakdown.md) | health-component-breakdown | Accepted | v0.8 adds 5-link component breakdown (db/index/embed/retriever/eval) to /v1/health, add-only schema. |
| [021](adr-021-memory-event-bus-bridge.md) | memory-event-bus-bridge | Accepted | v0.8 bridges memory state-op audit events (pin/deprecate/soft_delete) to EventBus.send, best-effort. |
| [022](adr-022-memory-is-pinned-field-amendment.md) | memory-is-pinned-field-amendment | Accepted | Amendment adding is_pinned bool to MemoryItem via add-only schema, cross-repo synchronized. |
| [031](adr-031-observability-hardening.md) | observability-hardening | Accepted | TraceStore FTS5 search + VACUUM; events SSE real-time push + replay from audit; event-bus config. |
| [032](adr-032-memory-ops-hardening.md) | memory-ops-hardening | Accepted | pin_actor + pinned_at, Pin/Unpin RPC split, hard-delete with X-Confirm, is_pinned audit backfill. |
| [036](adr-036-governance-debt-cleanup.md) | governance-debt-cleanup | Accepted | First-round debt cleanup: Go fallback memory-event parity + event-bus config + bounded cache + compose resources. |
| [038](adr-038-governance-debt-cleanup-2.md) | governance-debt-cleanup-2 | Accepted | Second-round cleanup: L2 cache bounding + memstore FIFO→LRU + memory hard-delete grounding + indexing event persistence. |
| [040](adr-040-observability-hardening.md) | observability-hardening | Accepted | Surface genuinely-swallowed errors via stderr (best-effort stays best-effort) + 7→3-4 grounding correction. |
| [045](adr-045-governance-debt-cleanup-3.md) | governance-debt-cleanup-3 | Accepted | Third-round cleanup: memory pin actor propagation (X-Actor) + L2 embedding cache access-order LRU (0 migration). |
| [048](adr-048-indexing-replay-splice.md) | indexing-replay-splice | Accepted | Fourth-round cleanup: indexing replay splice into live subscribe path + since_ts timing + default byte-equiv. |
| [049](adr-049-memory-unpin-actor-propagation.md) | memory-unpin-actor-propagation | Accepted | unpin actor propagation: UnpinMemoryRequest actor add-only + emit_audit_and_event actor param + default byte-equiv. |
