# ContextForge

**Status:** **v2.0.0-alpha** (current) — v2.0 身份验证基础（per-user token → verified identity；actor 从 declared 变 verified，关闭冒充风险；ADR-051）+ 承 v1.1.0 eval 硬化 + v1.0.0 API/CLI 冻结。**v2.0 进行中**：身份基础已交付（POST /v1/users + bearer verified identity + actor 覆写），但 RBAC / workspace isolation / OAuth-OIDC 仍延后（Phase 51-54+）。byte-equivalent 默认（trusted-network + 旧 shared token 不变）。详 [ADR-051](docs/decisions/adr-051-identity-foundation.md)。

ContextForge is a local-first context indexing and retrieval tool for agent memory, rules, source files, logs, and project notes.

It ships as two binaries (ADR-001):

- `contextforge`: Go control-plane CLI, REST/MCP adapter, Console Contract v1 REST surface (`console-api-serve`, v0.3+), export and eval entrypoint.
- `contextforge-core`: Rust data-plane daemon for scan, parse, chunk, index, and retrieval.

## Features

ContextForge indexes your project's knowledge sources and retrieves them with a multi-stage pipeline — all local-first, no telemetry, no network by default (ADR-004).

**Indexing & storage**
- Scan → parse → chunk → index pipeline over local files (markdown, code, config, notes, agent memory/rules).
- Layered storage: SQLite for metadata + Tantivy full-text (BM25) + pluggable vector backends (ADR-002).
- Secret redaction + denylist on ingest (AWS keys / `.env` redacted, never indexed as-is).

**Retrieval (three modes)**
- **BM25** (full-text) — default, pure std, zero dependency.
- **Semantic** (vector) — pluggable backends: BruteForce / sqlite-vec / qdrant (live server) / lancedb; remote embedding providers (OpenAI-compatible, SiliconFlow-verified) opt-in via env (ADR-027/042).
- **Hybrid** (BM25 + vector, RRF fusion) — combines lexical + semantic recall (ADR-025); reachable over REST `?hybrid=true` (Phase 39).
- **Reranker** (cross-encoder) — opt-in second-stage re-rank; remote providers (SiliconFlow `Qwen3-VL-Reranker-8B` verified) via env (ADR-026/043).

**Recall quality (real-measured, ADR-013)**
- **Hybrid** (BM25+vector RRF) recall@5/@10 = **1.0** over the 16-question author-curated golden (exceeds PRD north-star 75%/85%). This is the hybrid path (`contextforge eval run --hybrid`); BM25-only recall is lower by design — the vector/hybrid paths are where the recall uplift lives (ADR-025).
- **Large-corpus BM25 baseline** (Phase 49, 121 questions / 58 files): recall@10 = **0.7438** — lower than the small-golden hybrid number, confirming the small author-curated golden overfits BM25 recall. Hybrid/reranked large-corpus data stays deferred (needs ONNX model run). See `docs/spikes/phase49-large-corpus-recall.md` for the honest per-category breakdown + caveat.
- Code/CJK-aware tokenizer (`code_cjk`) is the **production default** for new collections (ADR-046): camelCase/snake_case/dotted.path subword split + CJK bigram; existing collections unaffected (opt-out `CONTEXTFORGE_TOKENIZER=default`).
- `source_type` filter (code/doc/config/other, derived from file extension — Phase 42).

**Memory & provenance**
- Memory ops: pin/unpin (with `X-Actor` caller attribution, Phase 40/44), deprecate/soft-delete/hard-delete, audit trail.
- Provenance on every hit: file path, line range, score, redaction status, retrieval method.
- Event bus: live SSE subscribe + replay-from-audit (reconnect-safe) + indexing-event persistence.

**Interfaces**
- **CLI** (`contextforge`): `init` / `import` / `index` / `search` / `eval` / `export` / `pin` / `version` / `console-api-serve`. Top-level `--help`/`-h` (no more exit-2 hostility, Phase 45).
- **REST** (`console-api-serve`, port 48181): Console Contract v1 — 22 endpoints (search/chunks/collections/memory/stats/eval/trace/events); proto FROZEN add-only (ADR-015).
- **MCP adapter**: Model Context Protocol server for agent integration.

**Supply chain**
- Every `v*` tag pushes a cosign keyless-signed image to GHCR + SPDX SBOM + SLSA provenance (ADR-033).
- GitHub Release object auto-created on tag push (Phase 46).

## Latest

The current release is **`v1.0.0`** (v1.0 收口终点 — ADR-050 完整 ratify Accepted + maturity label flip Pre-1.0→v1.0.0). For the full per-version changelog see [RELEASE_NOTES.md](RELEASE_NOTES.md) (detailed) or [CHANGELOG.md](CHANGELOG.md) (condensed). All historical version summaries (v0.3.0→v1.0.0) live in RELEASE_NOTES.md.

## Run the released image

Prebuilt, signed images are published to GHCR on every `v*` tag — no build
required. The current stable tag is **`v1.0.0`** (`linux/amd64`). The image
bundles both binaries (`contextforge-core` Rust data-plane + `contextforge` Go
control-plane); its default command is `console-api-serve` on port `48181`.

```bash
# Production: two-process stack (Rust core + Go console-api), data persisted.
# Defaults to v1.0.0 — override with CONTEXTFORGE_VERSION. See docs/deploy/production.md.
docker compose -f deploy/docker-compose.production.yml up -d
curl -fsS http://localhost:48181/v1/health | jq .   # -> {"status":"healthy",...}

# Dev / PoC: single container, in-memory fallback — serves immediately, NOT persistent.
docker run --rm -p 48181:48181 \
  -e CONSOLE_API_FALLBACK_INMEM=1 \
  ghcr.io/tajiaoyezi/contextforge-daemon:v1.0.0
```

Without `CONSOLE_API_FALLBACK_INMEM=1` (and no reachable `contextforge-core`),
`/v1/health` honestly reports `503 degraded` with an actionable `error_reason` —
start the core daemon or use the production compose stack.

Each release image is cosign keyless-signed and ships an SPDX SBOM + SLSA
provenance attestation; verify the exact digest before deploying (command + the
per-release digest live in [`docs/deploy/production.md`](docs/deploy/production.md)
§2 and `docs/releases/v1.0.0-evidence.md`).

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

## Releases & supply chain

- **GitHub Releases**: each `v*` tag auto-creates a GitHub Release object (Phase 46) with the version's release notes + signed image references. See the [Releases page](https://github.com/tajiaoyezi/contextforge/releases).
- **GHCR image**: `ghcr.io/tajiaoyezi/contextforge-daemon:vX.Y.Z` (`linux/amd64`), cosign keyless-signed + SPDX SBOM + SLSA provenance (ADR-033).
- **Verify the image** before deploying: see [`docs/deploy/production.md`](docs/deploy/production.md) §2 for the cosign verify command + per-release digest.
- **Changelog**: [CHANGELOG.md](CHANGELOG.md) (condensed) / [RELEASE_NOTES.md](RELEASE_NOTES.md) (detailed).

## Platform support & license

- **Official target**: Linux x86_64 / WSL2; macOS should work; Windows is best-effort via Git Bash.
- **`LICENSE`**: dual-licensed under MIT OR Apache-2.0 (`SPDX: (MIT OR Apache-2.0)`). Contributions are dual-licensed without additional terms.
- Release / quickstart smoke gates (`scripts/release_smoke.sh`, `scripts/quickstart_smoke.sh`) run as part of the CI pipeline.

## Where to go next

- [`contextforge.example.toml`](contextforge.example.toml) — starting point for collection allowlists, retrieval/tokenizer settings, and local-only provider config (`[embedding]` / `[vector]` / `[reranker]` / `[retrieval]`).
- [CHANGELOG.md](CHANGELOG.md) / [RELEASE_NOTES.md](RELEASE_NOTES.md) — per-version history.
- [docs/decisions/README.md](docs/decisions/README.md) — ADR index (architecture / storage / retrieval / release / governance decisions).
- [docs/roadmap.md](docs/roadmap.md) — version roadmap + the v1.0 收口 anchor (ADR-050).
- [docs/specs/phases/](docs/specs/phases/) — phase specs (SDD/BDD/TDD traceability).
