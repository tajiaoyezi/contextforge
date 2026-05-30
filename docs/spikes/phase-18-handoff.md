# Phase 18 (vector-backend-selection) — autonomous run handoff

**Date**: 2026-05-30 · **Stop reason**: stop-condition (a) generalized — the remaining vector
backends cannot be built/run on this `x86_64-pc-windows-msvc` machine; the 4-backend comparison +
task-18.7 selection requires a **Linux runner** (phase-18 §7 R1 "Linux-first"). Documented, not
fabricated/spun.

## Merged this run

| PR | task | result |
|---|---|---|
| #129 | spec-fix-task-18.1 | review remediation (VectorError `#[non_exhaustive]`, AC3 test rigor, spec integrity) |
| #130 | task-18.2 spike harness | `bench/` crate (deterministic corpus + 5-dim measure + trait runner), 202 tests, Noop smoke |
| #131 | task-18.6 hnsw + task-18.3 deferral | first **real backend recall data**; sqlite-vec build-block evidence |

## Backend status (4 candidates)

| backend | task | status | data |
|---|---|---|---|
| **hnsw** (instant-distance) | 18.6 | ✅ **done, real data** | release n=5000/dim=64: recall@5/10 = **1.0**, P95 **0.23ms**, cold-start 641ms, reindex 664ms (idle/index RSS = n/a on Windows) |
| sqlite-vec | 18.3 | ⏸️ deferred — build-blocked | `cl.exe` exit 2 on `sqlite-vec.c` (MSVC); `docs/spikes/phase-18-sqlite-vec.md` |
| qdrant-embedded | 18.4 | ⏸️ deferred — needs server | `qdrant-client` requires a running Qdrant server / embedded segment; cannot spike unattended on Windows |
| lancedb | 18.5 | ⏸️ deferred — heavy native | Arrow/Lance build is heavy; dep resolution also hit a transient schannel SSL error this run |

hnsw passes the PRD thresholds it can be measured against on Windows: P95 0.23ms < 500ms ✅,
recall@10 1.0 ≥ 70% ✅ (RSS not measurable on Windows). recall=1.0 is near-ideal on well-separated
synthetic vectors — representative ranking needs the dogfood corpus + larger n.

## task-18.7 (ADR-023 selection) — UNDECIDED

A data-driven default-backend choice needs the full 4-way 5-dim comparison. Only hnsw has data on
this platform, so **no ADR-023 selection is made**. hnsw is a strong candidate (pure-Rust, builds
everywhere, sub-ms P95, no native deps) but the decision must follow the Linux comparison.

## Recommended next steps (on a Linux x86_64 runner / CI)

1. sqlite-vec (18.3): build with gcc/clang (where the extension is tested) → implement
   `SqliteVecBackend`, register in `bench/src/backends.rs`, run the harness.
2. qdrant (18.4): stand up Qdrant via docker-compose → `qdrant-client` backend → harness.
3. lancedb (18.5): build `lancedb` (embedded Lance files) → backend → harness.
4. Extend `BACKENDS` in `scripts/spike_vector_backends.sh` and run all four at the 100k synthetic +
   dogfood corpora (release).
5. task-18.7: compare the four `docs/spikes/phase-18-<backend>.md` evidence files, write
   `docs/decisions/adr-023-<chosen>-default.md` (Proposed → Accepted), wire the default backend.
6. task-18.8 (eval SemanticRecall@K) → task-18.9 (v0.11.0 closeout).

The harness (task-18.2) + the hnsw backend (task-18.6) are reusable as-is; the Linux work is
additive (each backend is a new cfg-gated file + a registry arm).
