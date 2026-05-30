# Phase 18 — 4-backend vector spike comparison (task-18.7 input)

> All four candidate backends (task-18.3 sqlite-vec / task-18.4 qdrant / task-18.5 lancedb /
> task-18.6 hnsw) measured on the **same Linux x86_64 host** (WSL2 Ubuntu 26.04, rustc 1.96.0,
> release) by the task-18.2 harness, at two corpus sizes. This is the data-driven input for the
> ADR-023 default-backend decision.

## 5-dimension data — n=5000, dim=64

| backend | model | recall@5/10 | P95 (ms) | idle/index RSS (MB) | cold-start (ms) | reindex (ms) |
|---|---|---|---|---|---|---|
| hnsw | in-mem ANN (pure Rust) | 1.0 / 1.0 | 0.382 | 4.4 / 11.0 | 836 | 798 |
| sqlite-vec | embedded + disk, exact | 1.0 / 1.0 | **0.167** | 6.0 / 8.5 | 36.8 | 54.4 |
| qdrant | external server, ANN | 1.0 / 1.0 | 0.650 | server **~105** | 30.9 | 29.8 |
| lancedb | embedded + disk, flat | 1.0 / 1.0 | 1.551 | 30.5 / 50.9 | **7.4** | 5.8 |

## 5-dimension data — n=100000, dim=64 (PRD scale target)

| backend | recall@5/10 | P95 (ms) | idle/index RSS (MB) | cold-start (ms) | reindex (ms) |
|---|---|---|---|---|---|
| hnsw | 1.0 / 1.0 | **0.871** | 55.3 / **180.0** | **28432** | 28984 |
| sqlite-vec | 1.0 / 1.0 | 3.198 | 56.7 / 90.7 | 760 | 1114 |
| qdrant | 1.0 / 1.0 | 0.947 | client 57.5 / 91.6 · server **~166** | 385 | 355 |
| lancedb | 1.0 / 1.0 | 10.893 | 68.0 / 90.8 | **50.4** | 56.6 |

## What the scale (5k → 100k) reveals

- **recall is non-discriminating here**: all four hold recall@5/10 = 1.0 even at 100k. The synthetic
  seed vectors are too well-separated to differentiate ANN from exact. **The recall ranking that
  matters must come from real-distribution embeddings — that is task-18.8 (eval SemanticRecall@K on
  the dogfood corpus).** This is the single biggest caveat on any selection made from this table.
- **query latency (P95)**: the ANN backends scale best — hnsw 0.87 ms and qdrant 0.95 ms stay
  sub-ms at 100k. The exact/flat backends grow with n: sqlite-vec 0.167 → 3.2 ms (O(n) scan),
  lancedb 1.55 → 10.9 ms (flat scan + DataFusion planning). All four remain far under the PRD P95
  < 500 ms even at 100k.
- **index build / cold-start**: this is where the profiles diverge hardest. hnsw's graph build is
  **28.4 s at 100k** (O(n·log n) — up from 0.84 s at 5k); the others build in well under 1.2 s, with
  lancedb fastest (50 ms columnar append). For a local-first tool that rebuilds on restart, hnsw's
  build cost at scale is a real liability **unless the graph is persisted**.
- **memory (index RSS)**: hnsw is the heaviest in-process at 100k (**180 MB** — the graph), ~2× the
  others (~90 MB). Qdrant adds a separate ~166 MB server process on top of the client.

## Per-backend summary

| backend | strengths | weaknesses | natural fit |
|---|---|---|---|
| **sqlite-vec** | lightest, ADR-002 SQLite-aligned (shares the data-plane store, on-disk, no rebuild), exact recall, fast build | exact O(n) query latency grows with n; **Windows MSVC build-blocked** (Linux/gcc only) | Linux/container production aligned with the existing SQLite layer |
| **hnsw** | **pure Rust, builds everywhere incl. Windows MSVC, 0 native deps**, sub-ms ANN at scale | **28 s graph build at 100k**, in-memory only (no persistence → rebuild on restart), heaviest in-process RSS | cross-platform dev/test + small corpora where build cost is low |
| **qdrant** | best ANN throughput + horizontal scale, replication, filtering, server-managed persistence | **external server process** (breaks single-binary), +166 MB server RSS, gRPC latency | hosted / multi-agent scale-out deployments |
| **lancedb** | **fastest writes** (50 ms), embedded + durable columnar, SQL + metadata filtering, versioned datasets | slowest queries (flat scan), heaviest deps + **protoc build prereq** (~5 min build) | embedded analytics / fast bulk-ingest with on-disk durability |

## Bearing on ContextForge (local-first, single-binary, SQLite-based, cross-platform)

- **single-binary / no external service** rules out qdrant as the *default* (it is the scale-out
  option, not the embedded default).
- **ADR-002 already ships SQLite** (rusqlite bundled) for the data plane → **sqlite-vec** is the most
  architecturally coherent embedded default: the vector index can live in the same on-disk SQLite
  store, no separate format, no rebuild-on-restart. Its only blocker is the **Windows MSVC dev box**.
- **cross-platform dev parity** favors **hnsw** (the only backend that builds on Windows MSVC with
  zero native deps), but its 28 s build + no-persistence + 180 MB at 100k make it a poor *production*
  default at scale.
- **lancedb** is a credible embedded alternative (durable + fastest writes) but carries the heaviest
  build (protoc + Lance/DataFusion) and the highest query latency.

See `docs/decisions/adr-023-vector-backend-default.md` for the decision.
