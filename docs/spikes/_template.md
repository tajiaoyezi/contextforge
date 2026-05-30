# Phase 18 spike — `<backend-name>`

> Evidence template for Phase 18 vector-backend spikes (task-18.2 harness output schema).
> task-18.3-18.6 fill one of these per backend; task-18.7 compares the four to pick a default.

## 5-dimension measurement

| dimension | value | tier |
|---|---|---|
| backend | `<name>` | — |
| corpus size n | `<n>` | — |
| dim | `<dim>` | — |
| recall@5 | `<r5>` | must |
| recall@10 | `<r10>` | must |
| P95 latency (ms) | `<p95>` | must |
| idle RSS (MB) | `<idle_rss>` | must |
| index RSS (MB) | `<index_rss>` | must |
| cold-start (ms) | `<cold>` | nice-to-have |
| reindex (ms) | `<reindex>` | nice-to-have |

Data source: synthetic 100k (P95 / RSS magnitude) + dogfood corpus (recall relative ranking).

## Environment

- platform / arch:
- toolchain (rustc):
- embedding source: deterministic seed vectors (no production embedding model)

## Trade-off discussion

- **build / compile complexity** (cross-platform, native deps):
- **persistence model** (vs ADR-002 SQLite+Tantivy):
- **API stability / ecosystem maturity**:
- **verdict** (selected / excluded, and why):

## Open questions / follow-ups

-
