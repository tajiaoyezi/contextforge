# contextforge-bench (task-18.2)

Vector backend **spike measurement harness** for Phase 18 (vector-backend-selection).

Measures the Phase 18 §2A 5 dimensions for any backend implementing the frozen
`VectorIndexer + VectorSearcher` traits (task-18.1):

| dimension | meaning | tier |
|---|---|---|
| recall@5 / recall@10 | top-k contains the exact (brute-force) nearest neighbour | must |
| P95 latency | 95th-percentile single-query search time | must |
| RSS (idle / index) | resident set size, Linux only (`/proc/self/statm`) | must |
| cold-start | open + first full index build time | nice-to-have |
| reindex | delete-all + rebuild-from-scratch time | nice-to-have |

The corpus is generated from a deterministic seed (inline splitmix64, no embedding model and
no `rand` dep) so the four backends measured by task-18.3-18.6 are directly comparable.

## Usage

```bash
# synthetic corpus
cargo run -p contextforge-bench -- --backend noop --n 2000 --dim 64 --seed 1

# write a Phase 18 evidence markdown
cargo run -p contextforge-bench -- --backend noop --n 2000 --dim 64 --seed 1 --out docs/spikes/phase-18-noop.md

# dogfood corpus (jsonl of {"chunk_id","embedding"})
cargo run -p contextforge-bench -- --backend noop --dogfood test/fixtures/spike/dogfood-contextforge.jsonl

# batch all wired backends → docs/spikes/
bash scripts/spike_vector_backends.sh 2000 64 1
```

At task-18.2 the only wired backend is `noop` (returns empty hits → recall 0), which exercises
the harness machinery end-to-end. Real backends (sqlite-vec / qdrant / lancedb / hnsw) are added
by task-18.3-18.6, each behind its own `vector-<backend>` feature.
