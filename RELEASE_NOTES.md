# ContextForge Release Notes

## v0.20.0 (2026-06-01) — memory-ops-hardening (pin-actor + pinned-at-timestamp + Pin/Unpin split + hard-delete + is_pinned audit backfill + ADR-032 ratified)

Phase 27 硬化 Phase 13 / Phase 17 落地的 Memory pin / 生命周期语义，兑现 ADR-022 §Trade-offs 三条刻意缩范围延后的 marker。proto 全 add-only（既有 field 1-10 + 5 RPC + `Pin{bool pin}` 不动，proto-freeze guard 过）；默认构建恒 0 新依赖 / 0 network（ADR-004）；既有 5 Memory RPC + `confirmMiddleware` 不退化。

| task | 交付 |
|---|---|
| 27.1 (#181) | `MemoryItem` add-only proto `pinned_by`(field 11) + `pinned_at_unix`(field 12) + migration `0017`（`ensure_pin_actor_columns` 守护幂等 ALTER）+ `set_pinned_with_actor`（pin 写 actor+now / unpin 归 ''+0，`pinned_at` 独立于 `updated_at`，`set_pinned` 委托向后兼容）+ pin RPC 传 `"console-api"` + `memory_to_pb`/Go 投影；`console_message_fields` freeze guard；TEST-27.1.1-5（store 15/15 + data_plane 14/14 + proto_contract） |
| 27.2 (#183) | proto add-only `MemoryService.Unpin`/`HardDelete` RPC + 4 message + `store.hard_delete`（物理删除，0 行 NotFound）+ `AuditOperation::MemoryHardDelete`（event_type `memory.hard_delete`）+ console-api `POST /v1/memory/{id}/unpin`（204）+ `.../hard-delete`（confirmMiddleware gated 412→204→404）；TEST-27.2.1-5 |
| 27.3 (this) | is_pinned audit backfill `reconcile_is_pinned_from_audit`（last `memory_pin`/`memory_unpin` 事件胜，opt-in 一次性，仅修正 is_pinned 不臆造 actor/timestamp）+ smoke v17 step 36（REAL mode live round-trip）+ v0.20.0 release docs + ADR-032 ratify + ADR-022 add-only Amendment + phase-27 §6 闭合；TEST-27.3.1 |

**ADR**：ADR-032 (memory-ops-hardening) 据 D1-D4 真实非合成验证 `Proposed → Accepted`（actor 真实来源 + backfill 覆盖率据真实受限如实记录，不伪造，ADR-013）；ADR-022 add-only Amendment 兑现 §Trade-offs 三条 marker（`pin_actor`→`pinned_by` / `memory-pinned-at-timestamp`→`pinned_at_unix` / `is-pinned-backfill-from-audit`→`reconcile_is_pinned_from_audit`，不溯改正文 D1-D5）；ADR-017 X-Confirm 复用；ADR-015 全 add-only。

**Upgrade / Rollback**：默认行为不变，无强制迁移。`memory.db` boot 时经 `ensure_pin_actor_columns` 幂等 ALTER 加 `pinned_by`/`pinned_at_unix`（既有行缺省 backfill，add-only）。新 `Unpin`/`HardDelete` RPC + unpin/hard-delete 路由 add-only；is_pinned backfill 是 opt-in。Rollback：`git tag -d v0.20.0` + 删 Release / ghcr tag；与 v0.19.0 行为兼容。**hard-delete 物理删除不可恢复**（隐私基线设计意图，X-Confirm 兜底防误触）。

## v0.19.0 (2026-06-01) — observability-hardening (TraceStore FTS5 + VACUUM + events SSE/重放 + event-bus 配置 + ADR-031 ratified)

Phase 26 硬化 Phase 16 落地的两条可观测性信号路径（TraceStore 持久化 + events 实时面）。默认构建恒 0 新依赖 / 0 network（FTS5/VACUUM 复用 rusqlite bundled，SSE 用 Go stdlib `http.Flusher`，重放查既有 `audit_log`，event-bus 配置复用 `with_capacity` seam）；既有 long-poll endpoint + 22-endpoint 契约 + `put`/`get`/`list`/`load_warm` 签名不退化（add-only，ADR-015 D1）。

| task | 交付 |
|---|---|
| 26.1 (#178) | `SqliteTracePersist` FTS5 内容检索 `search_fts`（quoted-phrase MATCH，limit clamp 1..=100）+ 周期 `vacuum()` / `prune_older_than(cutoff)` + `open()` 旧 0015-only 库 boot 回填（`backfill_fts_if_empty`）+ `put()` FTS 同步；migration `0016_search_traces_fts.sql`（FTS5 影子虚表，`IF NOT EXISTS` 幂等）；既有签名语义不变；0 新依赖（rusqlite bundled）；TEST-26.1.1-5（10/10） |
| 26.2 (#179) | events SSE 实时推送 `GET /v1/observability/events/stream`（`text/event-stream` + `http.Flusher`，add-only 旁挂 long-poll）+ 从 audit log 重放漏失 memory state-op 事件（proto add-only `since_ts`/`last_event_id`，`replay_events_from_audit` id ASC + ADR-021 D3 映射，`evt-audit-{id}` 拼接边界去重）；SSE 帧契约 + 重放顺序 deterministic（不依赖墙钟）；真实 daemon SSE e2e 诚实延后 `[SPEC-DEFER:phase-future.sse-live-server-e2e]`；TEST-26.2.1-5（Rust 2/2 + Go SSE 4 契约） |
| 26.3 (this) | event-bus 容量/分区/drain 配置（`EventBus::from_config` + `CF_EVENT_BUS_CAPACITY`/`CF_EVENT_BUS_PARTITION` + `CONSOLE_EVENTS_DRAIN_TIMEOUT`，保守默认 1000/不分区/100ms 行为不变）+ smoke v16 step 35 + v0.19.0 release docs + ADR-031 ratify + ADR-021/015 add-only Amendment + phase-26 §6 闭合；TEST-26.3.1（events 6/6 + drain 5/5） |

**ADR**：ADR-031 (observability-hardening) 据 D1-D6 真实非合成验证 `Proposed → Accepted`（SSE live-server e2e 维度据 CI 无 running daemon **记录维持**，不强 ratify、不伪造，ADR-013）；ADR-021 add-only Amendment 兑现 events-replay-from-audit（`adr-021:115`）+ event-bus 容量/分区（Rollback path `adr-021:153`）；ADR-015 SSE endpoint add-only。

**Upgrade / Rollback**：默认行为不变，无强制迁移。旧 `search_traces.db`（0015-only）boot 时幂等创建 0016 FTS 表 + 回填（add-only，既有数据不损）。SSE endpoint + `?since_ts=` 重放 + event-bus 配置均 opt-in。Rollback：`git tag -d v0.19.0` + 删 Release / ghcr tag；与 v0.18.0 行为兼容（add-only / opt-in），旧库 0016 FTS 表对 v0.18.0 代码无害。

## v0.18.0 (2026-06-01) — production-vector-backend (qdrant 生命周期层 + lancedb 可构建性 🟢 + 生产 backend 选择矩阵 + ADR-030 ratified)

### 摘要

v0.18.0 minor release (Phase 25): pushes the two production-scale ANN backends that ADR-023 tiered — **qdrant** (hosted/scale-out) and **lancedb** (embedded-columnar) — from the Phase-18 spike state toward production. **qdrant** gains a server lifecycle layer (`QdrantConnConfig` validate + `health()` probe + `decide_ensure` collection ensure-create) that is contract-testable **without a live server**, replacing the spike's blind drop+create `open()` (task-25.1). **lancedb** gets a real dev-box buildability investigation — 🟢 `cargo build --features vector-lancedb` passes on `x86_64-pc-windows-msvc` (protoc supplied via the in-repo vendored binary, 0 new dependency) — plus an index-tuning parameter-validation layer (`LanceIndexTuning::validate`, IVF_PQ/HNSW + compaction threshold) (task-25.2). A **production backend selection matrix** (corpus-size × deployment-shape → hnsw / sqlite-vec / lancedb / qdrant + per-tier caveat) ships in the evidence doc (task-25.3).

**Honest scope (read this)**: the **default build is unchanged** — 0 vector dependency, BM25-only baseline (qdrant/lancedb are feature-gated, default unchanged). This is a **backend lifecycle / buildability-layer release with NO recall numbers**. qdrant's lifecycle layer is verified at the **contract layer without a live server** (config validation + health-probe unreachable shape + ensure-create decision); **real KNN over a live qdrant server is honestly deferred** (`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`, CI has no qdrant server). lancedb 🟢 builds on the Windows MSVC dev box, but the credential is **single-box** (protoc is still a hard prerequisite that must be explicitly provided; CI does not build the feature by default), and a broad `cargo test --features vector-lancedb` (compiling all integration-test targets) hits a rustc 1.95.0 ICE in vector-unrelated test targets — a toolchain limitation, not a regression (the `cargo build` + `--lib` tests pass). **lancedb real ANN index perf is deferred** (`[SPEC-DEFER:phase-future.lancedb-index-tuning]`). All recorded per ADR-013 (no faked live-server / cross-platform credentials).

### What shipped (Phase 25, tasks 25.1–25.3)

| task | delivery | PR |
|---|---|---|
| 25.1 | `core/src/retriever/vector/qdrant.rs` qdrant server lifecycle layer: `QdrantConnConfig` (url/timeout/api_key/tls + `from_env` + `validate`) + `QdrantHealth` + `CollectionDesc` + `EnsureAction` + `decide_ensure` pure fn + `QdrantBackend::connect`/`health()`; `open()` rewritten to ensure-create (reuse-if-matching, no silent drop) — TEST-25.1.1-4 contract-tested without a live server, 0 new dep | (merged) |
| 25.2 | `core/src/retriever/vector/lance_db.rs` `LanceAnnIndex` (IvfPq/Hnsw) + `LanceIndexTuning::validate(dim)` index-tuning param contract + `docs/spikes/phase-25-lancedb-buildability.md` real dev-box buildability (🟢 `cargo build --features vector-lancedb` exit 0 on x86_64-pc-windows-msvc, 1097 rlib hard evidence, 0 new dep) — TEST-25.2.1-5 | (merged) |
| 25.3 | Phase 25 closeout: production backend selection matrix (corpus-size × deployment-shape → hnsw/sqlite-vec/lancedb/qdrant + caveat) + smoke v15 step 34 + v0.18.0 release docs + ADR-030 ratify + ADR-023 D3/D4 add-only Amendment | this PR |

### ADR-030 ratified (Proposed → Accepted) + ADR-023 Amendment

**ADR-030 production-vector-backend** ratified on the **real non-synthetic** verification of D1–D4: D1 qdrant lifecycle contract layer (TEST-25.1.1-4, no live server), D2 lancedb 🟢 real dev-box build (`cargo build --features vector-lancedb` exit 0 + index-tuning param validation TEST-25.2.3-4), D3 production backend selection matrix, D4 default 0-vector-dep unchanged. **qdrant live-server KNN** (`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`) and **lancedb real ANN index perf** (`[SPEC-DEFER:phase-future.lancedb-index-tuning]`) are honestly deferred — not faked. **ADR-023** gets an add-only Amendment advancing the D3 (qdrant) + D4 (lancedb) tiers without rewriting D1-D6 (ADR-014 D5). **ADR-008** needs **no amendment** (qdrant-client/lancedb/arrow-array/futures are all pre-existing optional deps; 0 new direct dep). **ADR-014 — 16th activation.**

### Upgrade path

- Drop-in: default build behavior unchanged (0 vector dependency + BM25-only baseline). No forced migration. The production-scale backends are feature-gated: `--features vector-qdrant` (needs a live qdrant server) / `--features vector-lancedb` (needs a `protoc` prerequisite for the lance `build.rs`).
- qdrant `open()` upgraded from spike drop+create to **ensure-create** (reuse if the collection exists with matching dim/metric; error on mismatch instead of silently dropping data) — a safer semantic for existing collections under `--features vector-qdrant`.

### Rollback path

`git tag -d v0.18.0` + delete the GitHub Release/ghcr tag. The default-build image is behavior-compatible with v0.17.0 (0 vector dependency + BM25 baseline), so a rollback is non-breaking (the qdrant/lancedb lifecycle/buildability work is all feature-gated and not enabled by default).

## v0.17.0 (2026-05-31) — code-and-cjk-tokenizer-and-eval-hardening (opt-in tokenizer + eval 校验器 + ADR-029 ratified)

### 摘要

v0.17.0 minor release (Phase 24): adds an **opt-in code/CJK tokenizer** and a **hardened eval ruler**. The Tantivy `content` field can now split code symbols (`camelCase`→`camel`+`case` keeping the original token, `snake_case`/`dotted.path`/`kebab-case`) and tokenize CJK text into bigrams (`配置加载`→`配置`/`置加`/`加载`) — opt-in via `RetrieverConfig.tokenizer="code_cjk"`. The eval golden dataset gains an independent validator (`ValidateGoldenSemantic`: schema well-formedness + duplicate detection + answer coverage) and a code/CJK-annotated `golden-semantic.jsonl`. A **real before/after recall delta** (default **0.9091 → code/CJK 1.0000**, +0.0909) is measured through the production `Retriever` BM25 path.

**Honest scope (read this)**: the **default build is unchanged** — 0 new dependency (pure std tokenizer), default tokenization unchanged (existing collections are not silently invalidated), eval gate thresholds unchanged. The tokenizer is **opt-in via config (not a feature flag)**; **adopting it requires a re-index** (it changes the inverted terms). The +0.0909 delta is on a **small dataset (11 queries / 12 files)**, driven by one real CJK-bigram case (`语义检索`); the other 10 queries are parity (full-symbol/full-phrase queries match in both analyzers). The tokenizer's sub-token discrimination is proven deterministically by the task-24.1 unit tests (not extrapolated). The **rust-native-eval-runner is honestly deferred** after a real evaluation (the Go harness stays the single source of truth). All recorded per ADR-013 (no faked numbers).

### What shipped (Phase 24, tasks 24.1–24.3)

| task | delivery | PR |
|---|---|---|
| 24.1 | `core/src/indexer/mod.rs` opt-in code/CJK `TextAnalyzer` (`CodeCjkTokenizer`: camelCase/snake_case/dotted.path/kebab-case split + 保留原 token + CJK bigram, pure std) + `build_tantivy_schema(tokenizer)` opt-in branch + `open_with_tokenizer` + retriever symmetric registration — 0 new dep, default tokenization unchanged | #173 |
| 24.2 | `internal/eval/eval.go` `ValidateGoldenSemantic` (add-only: schema + duplicate + coverage; `knownCategories` += code-symbol/cjk) + `test/fixtures/eval/golden-semantic.jsonl` (11 questions, code-symbol + CJK → real files) — zero Rust delta, gate thresholds unchanged | #174 |
| 24.3 | Phase 24 closeout: real before/after recall delta (`phase24_tokenizer_recall` example, +0.0909) + rust-native-eval-runner eval (honestly deferred) + smoke v14 step 33 + v0.17.0 release docs + ADR-029 ratify | this PR |

### ADR-029 ratified (Proposed → Accepted)

**ADR-029 code-and-cjk-tokenizer-and-eval-hardening** ratified on the **real non-synthetic** verification of D1–D3/D5: D1 code/CJK tokenizer (TEST-24.1.1-4 + real recall delta +0.0909), D2 eval validator (TEST-24.2.1-2), D3 code/CJK golden 扩充 (TEST-24.2.3), D5 default unchanged (0 new dep + default tokenization + gate thresholds). **D4 rust-native-eval-runner honestly deferred** (`[SPEC-DEFER:phase-future.rust-native-eval-runner]`) — not faked as implemented. **ADR-006** (recall gate thresholds) and **ADR-008** (library selection) need **no amendment** (gate unchanged; tokenizer is std-only, 0 new dep). **ADR-014 — 15th activation.**

### Upgrade path

- Drop-in: default build behavior unchanged (default tokenization + BM25 baseline + eval gate thresholds). No forced migration. The eval `ValidateGoldenSemantic` is add-only (existing `ValidateDataset` callers unaffected).
- The code/CJK tokenizer is opt-in via `RetrieverConfig.tokenizer="code_cjk"`. **Adopting opt-in requires a re-index of existing collections** (it changes the `content` inverted terms; the old index still works with the default analyzer but does not get code/CJK sub-token hits).

### Rollback path

`git tag -d v0.17.0` + delete the GitHub Release/ghcr tag. The default-build image is behavior-compatible with v0.16.0 (default tokenization + BM25 + 0-dep), so a rollback is non-breaking (the opt-in tokenizer is not enabled by default).

## v0.16.0 (2026-05-31) — vector-persistence-and-cross-platform (hnsw 持久化 + sqlite-vec MSVC + ADR-028 ratified)

### 摘要

v0.16.0 minor release (Phase 23): makes the feature-gated vector backends **persistent + cross-platform** — **hnsw graph persistence** (`HnswBackend::save`/`load` to `VectorIndexConfig.persistence_path` + rebuild-on-load fallback, `vector-hnsw`), a **sqlite-vec Windows MSVC** investigation that **resolved the Phase 18 MSVC-build-blocked stop-condition** (real `cargo build` + run on `x86_64-pc-windows-msvc`), and a **vector incremental-index evaluation** (brute-force / sqlite-vec support row-level append; hnsw full-rebuild deferred).

**Honest scope (read this)**: the **default build stays local, 0-vector-dependency, and BM25-baseline-unchanged** — all persistence/cross-platform capability is behind the `vector-hnsw` / `vector-sqlite` features (ADR-023 D5). **This is a backend-layer release with no recall numbers.** The persisted graph is **not** wired into the `server.rs` semantic hot path yet (still rebuilds on demand — a future release). sqlite-vec MSVC evidence is from a single dev box (rustc 1.95.0); CI does not build the feature by default — honestly recorded (ADR-013), not faked.

### What shipped (Phase 23, tasks 23.1–23.3)

| task | delivery | PR |
|---|---|---|
| 23.1 | `core/src/retriever/vector/hnsw.rs` `HnswBackend::save`/`load` (path B: serialize `(normalized embedding, chunk_id)` inputs + load-rebuild; absent/corrupt/version-mismatch → rebuild-on-load) + `open` wires `persistence_path` — 0 new dep | #168 |
| 23.2 | sqlite-vec Windows MSVC investigation → 🟢 path (a) bundled amalgamation builds + runs on `x86_64-pc-windows-msvc` (resolves Phase 18 MSVC-blocked) + contract tests + `docs/spikes/phase-23-sqlite-vec-cross-platform.md` — 0 source/Cargo.toml change | #169 |
| 23.3 | Phase 23 closeout: incremental-index eval (brute-force/sqlite-vec row-level append; hnsw deferred) + smoke v13 step 32 + v0.16.0 release docs + ADR-028 ratify + ADR-023 add-only Amendment | this PR |

### ADR-028 ratified (Proposed → Accepted) + ADR-023 Amendment

**ADR-028 vector-persistence-strategy** ratified on the **real non-synthetic** verification of D1–D4: D1 hnsw persistence (path B roundtrip 3/3 PASS), D2 sqlite-vec MSVC (real build + run, resolves Phase 18 stop-condition), D3 incremental index (brute-force/sqlite-vec append; hnsw deferred), D4 default 0-vector-dep unchanged. **ADR-023** gets an add-only Amendment: its "hnsw rebuild-on-restart" disqualifier is resolved (task-23.1) and "sqlite-vec MSVC-blocked / dev-prod parity imperfect" is narrowed (task-23.2) — D1–D6 正文 not retro-edited (ADR-014 D5).

### Upgrade path

- Drop-in: default build behavior unchanged (BM25 + 0-vector-dep). No migration. `VectorIndexConfig.persistence_path` (existing field) is first consumed by `HnswBackend::open` (`None` → in-memory, byte-equivalent).
- Vector persistence (hnsw `save`/`load`) + sqlite-vec cross-platform are feature-gated (`vector-hnsw` / `vector-sqlite`) + explicit opt-in (not in the default image).

### Rollback path

`git tag -d v0.16.0` + delete the GitHub Release/ghcr tag. The default-build image is behavior-compatible with v0.15.0 (BM25 + 0-dep deterministic semantic/hybrid path), so a rollback is non-breaking.

## v0.15.0 (2026-05-31) — embedding-provider-completion (provider 配置选择 + 缓存 + 远程骨架 + ADR-027 ratified)

### 摘要

v0.15.0 minor release (Phase 22): grows the Phase 19 embedding layer ("hardcoded `DeterministicEmbeddingProvider` default + a single feature-gated `FastEmbedProvider`") into a **configurable provider layer** — a runtime `select_provider` factory (deterministic / fastembed / remote) with **dim negotiation** (`DimMismatch`, no silent truncate/pad), a **content-hash embedding cache** (`CachingEmbeddingProvider`, memory L1 + optional SQLite L2), a **feature-gated remote provider skeleton** (`RemoteEmbeddingProvider`, OpenAI/Cohere HTTP via ureq rustls), and an **opt-in remote-reachability health probe**.

**Honest scope (read this)**: the **default build stays local, model-free, and 0-network-dep** — the deterministic identity provider is the default; fastembed (`embedding-fastembed`) and remote (`embedding-remote`) are **feature-gated + explicit opt-in** (ADR-004 local-first, the non-negotiable red line). The embedding cache + remote skeleton are verified at the **unit / contract layer** (no network in tests). **This is a provider-layer release with no recall numbers** — real remote-network 联调 / API keys / recall quality + the real remote health probe are honestly **deferred** (ADR-013 — CI has no credentials/network; not faked). `[embedding]` config is add-only — **no breaking contract bump**.

### What shipped (Phase 22, tasks 22.1–22.4)

| task | delivery | PR |
|---|---|---|
| 22.1 | `internal/config` add-only `[embedding]`(provider/dim) codec + `core/src/embedding/factory.rs` `select_provider` + `negotiate_dim`→`DimMismatch` + `server.rs` semantic path via factory (byte-equivalent default) | #163 |
| 22.2 | `core/src/embedding/cache.rs` `CachingEmbeddingProvider` (Sha256(text)→embedding; memory L1 + optional SQLite L2, ADR-002; f32 LE BLOB round-trip) — 0 new dep | #164 |
| 22.3 | `core/src/embedding/remote_provider.rs` `RemoteEmbeddingProvider` (`embedding-remote` feature, ureq rustls) + pure `build_request_body`/`parse_response` + contract tests (fixtures, no network); `ureq 2.12.1` R7 chore | #165 |
| 22.4 | Phase 22 closeout: `health.rs probe_embed` feature-gated opt-in remote probe (config-only default unchanged) + smoke v12 step 31 (`init` emits `[embedding]`) + v0.15.0 release docs + ADR-027 ratify | this PR |

### ADR-027 ratified (Proposed → Accepted)

**ADR-027 embedding-provider-abstraction** is ratified on the **real non-synthetic** verification of D1–D5: D1 config+factory (Go round-trip + Rust factory tests), D2 dim negotiation (`negotiate_dim`→`DimMismatch` + feature fastembed 384-mismatch, network-free), D3 cache (`CachingEmbeddingProvider` 4/4 deterministic), D4 remote skeleton (contract 4/4 fixtures, no network), D5 local-first (default build 0 network dep — `cargo tree | grep ureq` empty). The ratify scope is the provider **abstraction layer**; remote real-network integration quality is honestly deferred (ADR-013).

### Upgrade path

- Drop-in: default build behavior unchanged (deterministic default + 0 model/0 network dep). No migration. `[embedding]` is add-only (absent / `Provider=""` → deterministic; existing `[remote]`/`[[collections]]` unaffected).
- Embedding cache: a library decorator (`CachingEmbeddingProvider`) wrapping any provider. Remote provider: needs `--features embedding-remote` + explicit opt-in config + env API key (not in the default image; key never logged).

### Rollback path

`git tag -d v0.15.0` + delete the GitHub Release/ghcr tag. The default-build image is behavior-compatible with v0.14.0 (BM25 + 0-dep deterministic semantic/hybrid path + config-only health probe), so a rollback is non-breaking.

## v0.14.0 (2026-05-31) — retrieval-quality (hybrid scoring + reranker + ADR-025/026 ratified)

### 摘要

v0.14.0 minor release (Phase 21): on top of the Phase 19/20 BM25 + semantic dual paths, it adds two **opt-in** ranking-quality enhancements — **hybrid scoring** (RRF fusion of the BM25 word-level + vector semantic scores, `retrieval_method = "hybrid"` + add-only `hybrid_score`) and a **reranker pipeline** (`Reranker` trait + deterministic `IdentityReranker` default + feature-gated real cross-encoder `CrossEncoderReranker`, wired via `Retriever::with_reranker`).

**Honest scope (read this)**: hybrid + rerank are both **opt-in** — default retrieval stays BM25. The **default build is unchanged and dependency-free** (0 new crate): its hybrid path uses the 0-dep `DeterministicEmbeddingProvider` + `BruteForceVectorBackend`, and the default reranker is the deterministic model-free `IdentityReranker`. The **real** embedding model (`FastEmbedProvider`, `all-MiniLM-L6-v2`) and **real** cross-encoder (`BGE-reranker-base`) are behind the `embedding-fastembed` / `reranker-fastembed` features; the real recall numbers below were measured with them. `SearchRequest.hybrid` is add-only — **no breaking contract bump**.

### What shipped (Phase 21, tasks 21.1–21.3)

| task | delivery | PR |
|---|---|---|
| 21.1 | `core/src/retriever/fusion.rs` RRF fusion (k=60) + `Retriever::search_hybrid` + proto `SearchRequest.hybrid=8` / `RetrievalResult.hybrid_score=15` (add-only, buf regen) + `server.rs` `req.hybrid` dispatch + `test_21_1×4` | #159 |
| 21.2 | `core/src/rerank/{mod,traits,identity,cross_encoder}.rs` `Reranker` trait + deterministic `IdentityReranker` (0 model dep) + feature-gated `CrossEncoderReranker` + `Retriever::with_reranker` seam + `docs/spikes/phase-21-reranker.md` | #160 |
| 21.3 | Phase 21 closeout: `internal/eval` Report hybrid/reranked columns + `internal/cli/eval.go --hybrid/--rerank` + smoke v11 step 30 multi-path assertion + real dogfood eval + v0.14.0 release docs + ADR-025/026 ratify | this PR |

### Real hybrid / reranked recall vs the BM25 baseline (dogfood, ADR-013 real run)

Real `FastEmbedProvider` (`all-MiniLM-L6-v2`, dim 384) + real `CrossEncoderReranker` (`BGE-reranker-base`), through the production `Retriever` over real ContextForge text (180 production chunks / 30 golden queries; `docs/spikes/phase-21-hybrid-recall.md`):

| method | recall@5 | recall@10 | top-1 | MRR | ADR-006 gate (≥0.70) |
|---|---|---|---|---|---|
| baseline BM25 | 0.9000 | 0.9667 | 0.0333 | 0.4095 | PASS |
| **hybrid RRF** | 0.9333 | 0.9667 | **0.6667** | **0.7881** | PASS |
| reranked cross-encoder | **0.9667** | 0.9667 | 0.3333 | 0.6306 | PASS |

- **Hybrid RRF is the decisive win**: top-1 +0.6334 / MRR +0.3786 over BM25 (BM25 finds the right file in top-10 but rarely ranks it first; fusing the vector signal fixes that) → ratifies **ADR-025 Accepted**.
- **Real cross-encoder ran** (ADR-026 D5 stop-condition not triggered): beats BM25 baseline (top-1 +0.30, MRR +0.22) + best recall@5. **Honest caveat (ADR-013)**: on this small code+ADR corpus, reranking the already-strong hybrid top-k does **not** beat hybrid on top-1/MRR (general-text BGE is weaker on code chunks; rerank only re-orders an already-good fusion) → rerank is recommended as a **domain-fit opt-in**, never a default. → ratifies **ADR-026 Accepted (with caveat)**.

### Upgrade path

- Drop-in: default build behavior unchanged (BM25-only, 0 new dependency). No migration.
- Hybrid: send `SearchRequest.hybrid=true` (or `contextforge eval run --hybrid`); unset → BM25. Reranking: wire `Retriever::with_reranker` (default `None` → no rerank). Real-model hybrid vector component needs `--features embedding-fastembed`; real cross-encoder rerank needs `--features reranker-fastembed` (not in the default image).
- Proto: `SearchRequest.hybrid` (8) + `RetrievalResult.hybrid_score` (15) are **add-only** — existing clients unaffected (Console Contract v1 shape unchanged, 22-endpoint conformance + proto-freeze guard PASS, unset → BM25). **No breaking contract bump.**

### Rollback path

`git tag -d v0.14.0` + delete the GitHub Release/ghcr tag. The default-build image is behavior-compatible with v0.13.0 (BM25-only + 0-dep deterministic semantic/hybrid path + identity reranker seam), so a rollback is non-breaking.

### Contract

Console Contract v1 unchanged in shape; the new `SearchRequest.hybrid` / `RetrievalResult.hybrid_score` fields are add-only and default to BM25 behavior when unset. The reranker adds no proto field (builder seam).

### ADR

**ADR-025 hybrid-scoring-fusion → Accepted** + **ADR-026 reranker-provider → Accepted** — both ratified on real dogfood eval data (ADR-013: real `FastEmbedProvider` + `CrossEncoderReranker` run, no synthetic figures). ADR-025 confirms the default RRF strategy (decisive top-1/MRR uplift, resolving the "indistinguishable on synthetic data" open point). ADR-026 ratifies the reranker architecture (real model ran; uplift over baseline + best recall@5) with the honest caveat that it underperforms hybrid on this corpus → opt-in domain-fit enhancement. **ADR-014 cross-validation gate — 12th activation**.

详 [Phase 21 spec](docs/specs/phases/phase-21-retrieval-quality.md) + [ADR-025](docs/decisions/adr-025-hybrid-scoring-fusion.md) + [ADR-026](docs/decisions/adr-026-reranker-provider.md) + [v0.14.0 evidence](docs/releases/v0.14.0-evidence.md) + [v0.14.0 artifacts](docs/releases/v0.14.0-artifacts.md) + [hybrid/reranked recall evidence](docs/spikes/phase-21-hybrid-recall.md)。

## v0.13.0 (2026-05-31) — semantic-retrieval-throughline (语义检索贯通 console-api + 经 Retriever 真实召回 + ADR-024 ratified)

### 摘要

v0.13.0 minor release: carries the Phase 19 (v0.12.0) semantic path the last mile (Phase 20). It now engages **end-to-end through console-api** — `POST /v1/search?semantic=true` (or body `semantic`) routes through Go `handleSearch` → `grpcclient` → `console_data_plane` gRPC `SearchService.Query` semantic dispatch (Rust `SearchServer::query`, mirroring the core `CoreService` `server.rs`) → ranked vector hits — and real recall is now measured **through the production `Retriever::search_semantic` hot path** instead of the v0.12.0 standalone example. This closes the two caveats v0.12.0 honestly recorded (`docs/releases/v0.12.0-evidence.md` §3b / task-19.4 §10).

**Honest scope (read this)**: semantic retrieval is still **opt-in** — default retrieval stays BM25. The **default build is unchanged and dependency-free**: its semantic path (incl. via console-api) uses the **0-dependency `DeterministicEmbeddingProvider` + `BruteForceVectorBackend`** (proves wiring, not model quality). The **real** embedding provider (`FastEmbedProvider`, `all-MiniLM-L6-v2`) is behind the `embedding-fastembed` feature; the real recall numbers below were measured with it. The new `console_data_plane SearchRequest.semantic` field is **add-only** — no breaking contract bump.

### What shipped (Phase 20, tasks 20.1–20.3)

| task | delivery | PR |
|---|---|---|
| 20.1 | `console_data_plane.proto` `SearchRequest` add-only `bool semantic = 7` (buf regen) + Rust `SearchServer::query` semantic dispatch (mirrors core `CoreService` `server.rs`, `DeterministicEmbeddingProvider` + 0-dep `BruteForceVectorBackend`) + Go `contractv1.SearchRequest.Semantic` + `handleSearch` `?semantic=true` OR-merge + `grpcclient` passthrough | #155 |
| 20.2 | real recall through the production `Retriever::search_semantic` hot path (real fastembed) + deterministic `test_20_2` hot-path wiring + `docs/spikes/phase-20-recall-via-retriever.md` | #156 |
| 20.3 | Phase 20 closeout + smoke v10 step 29 console-api real semantic assertion + v0.13.0 release docs + ADR-024 ratify | this PR |

### Real-embedding recall, through the production Retriever (resolves the v0.12.0 example caveat)

Real `FastEmbedProvider` (`all-MiniLM-L6-v2`, dim 384) over real ContextForge text, exact cosine, routed through the production `Retriever::search_semantic` hot path (real scanner + chunker → **175 production chunks**; `docs/spikes/phase-20-recall-via-retriever.md`):

| metric | task-20.2 (production Retriever, 175 chunks) | task-19.5 (standalone, 40 capped chunks) baseline |
|---|---|---|
| SemanticRecall@5 | **0.9667** (29/30) | 0.8333 |
| SemanticRecall@10 | **1.0000** (30/30) | 0.9333 |
| top-1 accuracy | **0.7333** | 0.60 |
| MRR | **0.8367** | 0.70 |
| ADR-006 A1 gate (≥ 0.70) | **PASS** | PASS |

**Honest inflation caveat (ADR-013)**: @10 = 1.0 is **partly file-level chunk-count inflation** — the uncapped production chunker yields many chunks per file (175 across 11 files), making "any chunk from the expected file in top-K" mechanically easier; this is the same artifact task-19.5 deliberately suppressed with `MAX_CHUNKS_PER_FILE`. But the discriminating metrics rule out pure inflation: top-1 (0.7333) and MRR (0.8367) are not chunk-count-inflated and are **higher** than task-19.5's 0.60 / 0.70 — the production path genuinely ranks the right file first more often. The two numbers are **not directly comparable** (different corpora + chunking); both clear the gate. task-20.2 is the **representative** measurement (production pipeline), task-19.5 the **controlled** discrimination floor. Deterministic embeddings prove plumbing, not recall — every real number here is a real fastembed run, no synthetic/fabricated figures.

### Upgrade path

- Drop-in: default build behavior is unchanged (BM25-only retrieval, 0 new dependency). No migration.
- To use the semantic path via console-api: send `?semantic=true` on `POST /v1/search` (now forwarded end-to-end), or run `contextforge eval run --semantic`. The default-build semantic path uses the deterministic provider; for real-model semantic search build/deploy with `--features embedding-fastembed`.
- Proto: `console_data_plane SearchRequest.semantic` (7) is **add-only** — existing clients are unaffected (Console Contract v1 shape unchanged, 22-endpoint conformance + proto-freeze guard PASS, unset → BM25). **No breaking contract bump.**

### Rollback path

`git tag -d v0.13.0` + delete the GitHub Release/ghcr tag. The default-build image is behavior-compatible with v0.12.0 (BM25-only + 0-dep deterministic semantic path), so a rollback is non-breaking.

### Contract

Console Contract v1 unchanged in shape; the new `console_data_plane SearchRequest.semantic` field is add-only and defaults to BM25 behavior when unset.

### ADR

**ADR-024 console-api-semantic-forward → Accepted** — ratified on task-20.1's real landing (Go contractv1/handleSearch/grpcclient tests + Rust `SearchServer::query` dispatch test `test_20_1` prove it; not synthetic, ADR-013). It records the `console_data_plane` add-only `semantic` field + the console-api↔daemon semantic-alignment口径 (mirroring the ADR-015/022 add-only pattern). **ADR-014 cross-validation gate — 11th activation**.

详 [Phase 20 spec](docs/specs/phases/phase-20-semantic-retrieval-throughline.md) + [ADR-024](docs/decisions/adr-024-console-api-semantic-forward.md) + [v0.13.0 evidence](docs/releases/v0.13.0-evidence.md) + [v0.13.0 artifacts](docs/releases/v0.13.0-artifacts.md) + [real recall evidence](docs/spikes/phase-20-recall-via-retriever.md)。

## v0.12.0 (2026-05-30) — vector-retrieval-integration (end-to-end semantic search + ADR-023 ratified)

### 摘要

v0.12.0 minor release: turns the Phase 18 vector-backend **infrastructure** into a **live, end-to-end semantic retrieval path** (Phase 19) and **ratifies ADR-023** on **real** embedding recall. A request can take the vector path through the whole stack (`POST /v1/search?semantic=true` → Go → Rust gRPC → `EmbeddingProvider` → vector backend → ranked hits), the eval CLI gains `contextforge eval run --semantic`, and the Phase 18 "synthetic recall is non-discriminating" caveat is resolved with measured real-embedding recall.

**Honest scope (read this)**: semantic retrieval is **opt-in** — default retrieval stays BM25. The **default build is unchanged and dependency-free**: its semantic path uses the **0-dependency `DeterministicEmbeddingProvider` + `BruteForceVectorBackend`** (proves wiring correctness, not model quality). The **real** embedding provider (`FastEmbedProvider`, `all-MiniLM-L6-v2`) is behind the `embedding-fastembed` feature; the real recall numbers below were measured with it. No model or vector dependency is compiled by default (ADR-023 D5).

### What shipped (Phase 19, tasks 19.1–19.7)

| task | delivery | PR |
|---|---|---|
| 19.1 | `EmbeddingProvider` trait + `DeterministicEmbeddingProvider` (0-dep default) + `FastEmbedProvider` (real, feature-gated) + spike evidence | #142 |
| 19.2 | default backend wired into `Retriever` (`with_embedder` + `with_vector_searcher` + `search_semantic`) | #143 |
| 19.3 | `/v1/search?semantic=true` Go→Rust gRPC semantic path + proto add-only (`semantic`, `vector_score`, `embedding_provider`) + 0-dep `BruteForceVectorBackend` | #144 |
| 19.4 | smoke v9 30-step (step 29 semantic REST + step 30 eval `--semantic`) + `contextforge eval run --semantic` dual-path CLI | #145 |
| 19.5 | **real** dogfood embedding `SemanticRecall@K` (fastembed) + `docs/spikes/phase-19-real-recall.md` | #146 |
| 19.6 | ADR-023 Proposed→**Accepted** + ADR-006 A1→**Active** + ADR-008 embedding-crate amendment | #147 |
| 19.7 | Phase 19 closeout (end-to-end semantic search) + v0.12.0 release docs | this PR |

### Real-embedding recall (resolves the Phase 18 non-discriminating caveat)

Real `FastEmbedProvider` (`all-MiniLM-L6-v2`, dim 384) over real ContextForge text, exact cosine (`docs/spikes/phase-19-real-recall.md`):

| metric | Phase 18 synthetic | Phase 19 real |
|---|---|---|
| SemanticRecall@5 | 1.0 (non-discriminating) | **0.8333** (25/30) |
| SemanticRecall@10 | 1.0 (non-discriminating) | **0.9333** (28/30) |
| top-1 / MRR | — | 0.60 / 0.70 |
| ADR-006 A1 gate (≥ 0.70) | aspirational | **PASS** |

ADR-013: every real number is a real fastembed run — no synthetic / deterministic / fabricated figures. The recall is exact-cosine (representative of any exact backend incl. the D1 `sqlite-vec` pick; upper bound for ANN).

### Upgrade path

- Drop-in: default build behavior is unchanged (BM25-only retrieval, 0 new dependency). No migration.
- To use the semantic path: send `?semantic=true` on `POST /v1/search`, or run `contextforge eval run --semantic`. The default-build semantic path uses the deterministic provider; for real-model semantic search build/deploy with `--features embedding-fastembed`.
- Proto: `SearchRequest.semantic` (7), `RetrievalResult.vector_score` (13) + `embedding_provider` (14) are **add-only** — existing clients are unaffected (22-endpoint conformance + proto-freeze guard PASS).

### Rollback path

`git tag -d v0.12.0` + delete the GitHub Release/ghcr tag. The default-build image is behavior-compatible with v0.11.0 (BM25-only), so a rollback is non-breaking.

### Contract

Console Contract v1 unchanged in shape; the three new proto fields are add-only and default to BM25 behavior when unset.

详 [Phase 19 spec](docs/specs/phases/phase-19-vector-retrieval-integration.md) + [ADR-023](docs/decisions/adr-023-vector-backend-default.md) + [v0.12.0 evidence](docs/releases/v0.12.0-evidence.md) + [v0.12.0 artifacts](docs/releases/v0.12.0-artifacts.md)。

## v0.11.0 (2026-05-30) — vector-backend-selection (infra + spike + ADR-023 Proposed)

### 摘要

v0.11.0 minor release: ships the **vector retrieval backend infrastructure + a data-driven backend selection** (Phase 18). It delivers the vector trait abstraction, a deterministic spike harness, **four real-data backend spikes measured on one Linux host**, the ADR-023 default-backend decision (**Proposed**), and the `SemanticRecall@K` eval metric + gate.

**Honest scope (read this)**: this is an **infrastructure + selection milestone**, *not* live semantic search. Production semantic retrieval and the ADR-023 ratification are **deferred** — the Phase 18 spike deliberately used deterministic seed vectors to avoid an ONNX/embedding dependency, so there is no real-distribution recall yet (all four backends score recall 1.0 on synthetic data — non-discriminating). Wiring a chosen backend into the production retriever + an embedding provider is a follow-on phase (`[SPEC-OWNER:phase-future.vector-retrieval-integration]`, ADR-023 D6). The `vector-*` features ship **off by default** — the default build is BM25-only and dependency-free.

### What shipped (Phase 18, tasks 18.1–18.9)

| task | delivery | PR |
|---|---|---|
| 18.1 | `Vector{Backend,Indexer,Searcher}` trait abstraction + `NoopVectorBackend` + retriever seam | #128 (+#129 review) |
| 18.2 | `bench/` spike harness — deterministic corpus + 5-dim measure + runner | #130 |
| 18.3 | **sqlite-vec** backend (`vec0`, stable 0.1.9) | #133 |
| 18.4 | **qdrant** backend (qdrant-client gRPC → local server) | #134 |
| 18.5 | **lancedb** backend (embedded Lance + Arrow) | #135 |
| 18.6 | **hnsw** backend (instant-distance, pure Rust) | #131 |
| 18.7 | **ADR-023 (Proposed)** default-backend decision + 4-backend comparison | #136 |
| 18.8 | `internal/eval` **SemanticRecall@K** metric + recall gate + ADR-006 Amendment A1 | #137 |
| 18.9 | Phase 18 closeout (honest scope) + v0.11.0 release docs | this PR |

### 4-backend comparison (real Linux data, n=100000 / dim=64)

| backend | recall@5/10 | P95 (ms) | index RSS (MB) | cold-start | model |
|---|---|---|---|---|---|
| sqlite-vec | 1.0 / 1.0 | 3.198 | 90.7 | 760 ms | embedded + disk, exact |
| hnsw | 1.0 / 1.0 | 0.871 | 180.0 | 28.4 s | in-mem ANN, pure Rust |
| qdrant | 1.0 / 1.0 | 0.947 | 91.6 (+~166 server) | 385 ms | external server ANN |
| lancedb | 1.0 / 1.0 | 10.893 | 90.8 | 50 ms | embedded + disk, flat |

recall is non-discriminating on synthetic vectors → the selection is driven by ContextForge's architecture (local-first, single-binary, SQLite-based per ADR-002, cross-platform incl. Windows MSVC), not recall. Full analysis: `docs/spikes/phase-18-comparison.md`.

### ADR-023 (Proposed) — tiered, feature-gated, default build BM25-only

- **D1** sqlite-vec = recommended embedded default (Linux prod, ADR-002-aligned) — **provisional** pending real-embedding recall.
- **D2** hnsw = cross-platform/dev fallback (pure Rust; but 28 s build + 180 MB at 100k → not the prod default at scale).
- **D3** qdrant = hosted/scale-out. **D4** lancedb = embedded-columnar alternative. **D5** default build ships no backend.

### Deferred to a follow-on phase (`[SPEC-OWNER:phase-future.vector-retrieval-integration]`)

- ADR-023 `Proposed → Accepted` ratification (needs real-embedding recall).
- Default backend wired into the production retriever hot path + smoke v9 `/v1/search?semantic=true`.
- An embedding provider (`[SPEC-DEFER:phase-future.embedding-provider-full]`).

### ADR-014 cross-validation gate — 9th activation

D1 mapping table + D2 spec-drift lint 0 hits + D3 verified-by + D4 main-agent autonomy + D5 no Phase 1–17 spec edits, across PRs #133/#134/#135/#136/#137 + this closeout. Each PR: default `cargo test --workspace` 0 failed + `go test ./...` ok + CI three-gate green before autonomous merge.

### Upgrade path (v0.10.0 → v0.11.0)

No migration, no breaking change. The `vector-*` features are off by default; the default build is byte-for-byte BM25-only behavior (NoopVectorBackend). Enabling a backend is a build-time feature choice (sqlite-vec needs Linux/gcc; lancedb needs protoc; qdrant needs a running server). `docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.11.0` after the tag push.

### Rollback path

`git tag -d v0.11.0` + delete the GitHub release; the closeout is documentation + feature-gated code, so reverting is a no-op for the default build. No DB or schema change to undo.

## v0.10.0 (2026-05-28) — is-pinned-amendment (Console PR #91/#93 backlog 11/11 = 100% closed)

### 摘要

v0.10.0 minor release: closes the **final** ContextForge-Console PR #91/#93 backlog item (P2 #6 `MemoryItem.is_pinned`). Backlog is now **11/11 = 100% closed** — the full review feedback raised by the Console team in v0.7.x is addressed. **First successful activation of the ADR-015 D5 字段冻结 amendment path** via ADR-022 (`memory-is-pinned-field-amendment`, Proposed → Accepted in this closeout PR).

### Backlog item closed (1 final)

| Item | Backlog signal | Solution | PR |
|---|---|---|---|
| **P2 #6** | `MemoryItem.is_pinned` field missing — Console UI Memory list/detail could only infer pin state from `MemoryOperation.op_type=pin` history (fragile: unpin still leaves a pin record) | task-17.1 — proto field 10 + Rust `memory_to_pb` mapper + Go `contractv1.MemoryItem.IsPinned` + `grpcclient.protoToMemoryItem` + `MemMemoryStore.Pin(id, pin)` (no longer discards `_`) + fixture-1 preset `IsPinned: true` + `handleMemoryPin` JSON body parser (backward-compat empty-body default true) + 5 new tests + smoke v8 step 28 (4 sub-assertions: post-restart survive + explicit pin=false + explicit pin=true + empty-body backward-compat) | [#118](https://github.com/tajiaoyezi/contextforge/pull/118) |

Additional Phase 17 ship:
- **Phase 17 E1 scaffolding** (PR #116, post-v0.9.0): Phase 17 spec + task-17.1 spec + ADR-022 (Proposed) + adapter index — Status: Pending awaiting Console cross-repo amend trigger
- **Phase 17 closeout** (this PR): ADR-022 Status Proposed → Accepted + Phase 17 + task-17.1 spec final §10 fills + v0.10.0 release docs (README + RELEASE_NOTES + evidence + artifacts)

### Cross-repo coordination — first end-to-end exercise of ADR-022 D4/D5

Phase 17 is the first phase to use the ADR-022 D5 cross-repo `Pending → Ready → Done` protocol:

1. **2026-05-28 (Phase 17 scaffolding ship via PR #116)**: ContextForge ships Phase 17 spec + ADR-022 Proposed + task-17.1 with Status: Pending awaiting Console signal.
2. **2026-05-28T12:16:57Z (Console-first ship)**: ContextForge-Console PR [#101](https://github.com/tajiaoyezi/ContextForge-Console/pull/101) merges to Console master @ `415ee30fcd8effd7929806d196458ec6e60fb49f` — `MemoryItem.IsPinned bool` add-only field in `console-api/internal/coreadapter/contractv1/contractv1.go` (between `Status` and `Availability`, JSON tag `is_pinned`).
3. **2026-05-28 (User forwards SHA)**: User forwards Console PR #101 merge SHA `415ee30` to ContextForge main agent.
4. **2026-05-28 (Verification)**: ContextForge main agent verifies via `gh api repos/tajiaoyezi/ContextForge-Console/contents/console-api/internal/coreadapter/contractv1/contractv1.go?ref=415ee30` returns the expected field block; flips Phase 17 + task-17.1 Status: `Pending → Ready → Done` within PR #118 implementation PR.
5. **2026-05-28 (ContextForge ship via PR #118)**: PR #118 ships the proto + Rust + Go end-to-end + tests + smoke v8.
6. **2026-05-28 (this closeout PR)**: ADR-022 Status Proposed → Accepted; v0.10.0 release docs.

This pattern is now reusable for any future cross-repo schema evolution (`tags`, `pinned_at`, etc. — all `[SPEC-DEFER:phase-future.*]`).

### Spec drift discovery

The original task-17.1 §3 prescribed migration `0017_memory_items_add_is_pinned.sql` + PRAGMA gate + Rust `SqliteMemoryStore::set_pinned` implementation. Recon during PR #118 revealed task-13.1 (Phase 13) already shipped most of it forward-looking:

- **Migration 0017 NOT needed** — `is_pinned INTEGER NOT NULL DEFAULT 0` was already added in `core/migrations/0013_memory_items.sql:16` at task-13.1 ship (Phase 13). The comment in 0013 even read "9 columns 1:1 mirror contractv1.MemoryItem + orthogonal is_pinned flag". Creating 0017 would have errored with `duplicate column name` on existing v0.6+ DBs.
- **Rust `SqliteMemoryStore::set_pinned` + `MemoryServer.Pin` write-through wiring** already shipped at Phase 13. Only the proto wire propagation (via `memory_to_pb` mapper) and the Go-side surface needed update.
- **`handleMemoryPin` body parsing gap** — the original handler at `internal/consoleapi/handlers.go:524` hardcoded `deps.Memory.Pin(id, true)` and never read the request body. Task-17.1 spec §3 missed this gap; the new handler now parses `{"pin": bool}` with empty/malformed body defaulting to `true` (preserving v0.7-v0.9 backward-compat contract).

PR #118 commit body + task-17.1 §3 + this release notes capture the discovery for future readers.

### Schema additions (add-only per ADR-015 D1 + first ADR-015 D5 amendment via ADR-022)

- `proto/contextforge/console_data_plane/v1/console_data_plane.proto`: `MemoryItem.bool is_pinned = 10` (add-only field 10; next available after `string status = 9`)
- `internal/contractv1/contractv1.go::MemoryItem.IsPinned bool` (json tag `is_pinned`, position between `Status` and `Availability` — mirrors Console master @ `415ee30` exactly)
- No SQLite migration needed (column already at `0013:16`)
- 22-endpoint Console contract conformance unaffected (contract v1 not bumped)
- Forward/backward compat: legacy v0.7-v0.9 daemon responses lacking `is_pinned` key unmarshal to Go bool zero value (`false`) — Console v0.10+ client treats this as "memory item not currently pinned" fallback. New v0.10+ daemon responses carry the real state.

### 关键设计取舍

- **`bool` type, not `*bool`**: pin state is always defined (never "not applicable" — Memory items are either pinned or not). Pointer + `omitempty` would let Console UI render ambiguously. ADR-022 D1 locks this.
- **`handleMemoryPin` empty-body defaults to `pin=true`**: preserves v0.7-v0.9 callers that POST without body. Pointer-typed body (`*bool`) cleanly distinguishes "absent" (default true) from "explicit false". Malformed JSON also falls back to `true` rather than 400 — lenient contract preserved.
- **No `pinned_at` / `pin_actor` / `tags` / `priority` fields in this amendment**: explicitly `[SPEC-DEFER:phase-future.*]`. ADR-022 §Trade-offs locks this — future amendments can follow the same D4/D5 protocol established here.
- **MemMemoryStore fixture-1 preset `IsPinned: true`**: ADR-022 D3 stipulates at least one pinned fixture so Console UI fallback mode (`CONSOLE_API_FALLBACK_INMEM=1`) renders a pinned row when verifying the new field. ADR-018 deny default keeps this off in production.
- **Smoke v8 step 28 gated on `MODE=real && sqlite3`**: the runtime end-to-end check needs both the Rust daemon (for SQLite persistence) and `sqlite3` CLI (for fixture seeding via `test/fixtures/memory-seed/seed.sql`). LOCAL_ONLY/docker modes verify via `internal/consoleapi/memstore_test.go` unit tests instead.

### ADR-014 cross-validation gate 第八次激活

- D1 mapping table: PR #118 body contains the Phase §6 ↔ task-17.1 §6 AC mapping (7-row table including the deferred AC7 → resolved in this closeout PR)
- D2 lint `--touched origin/master`: 0 unannotated hits across PR #118 + this closeout PR
- D3 verified-by: every Phase 17 §6 AC and task-17.1 §6 AC carries an explicit `verified by <test>` clause
- D4 governance: 主 agent 自治 §2A Ready review + R6 merge decision; user as single driver forwards the Console SHA but does not edit ContextForge code
- D5 历史不溯改: Phase 1-16 specs untouched (verified via `git diff origin/master` scoping)

### Tests (cumulative Phase 17)

- `cargo test --workspace`: 41 tests across crates (lib + integration); PR #118 adds 1 new lib test (`test_list_returns_is_pinned_column`) + 2 new gRPC integration tests (`test_is_pinned_propagates_via_grpc_list_and_get`, `test_pin_rpc_unpin_reverses_state`). Existing `test_set_pinned_persists` from Phase 13 covers the SqliteMemoryStore toggle path.
- `go test ./...`: 21 packages all PASS. PR #118 adds 2 new unit tests in `internal/consoleapi/memstore_test.go` (`TestMemMemoryStore_Pin_TogglesIsPinned`, `TestMemMemoryStore_List_ReturnsIsPinned`) + 1 new test in `internal/contractv1/types_test.go` (`TestMemoryItemForwardBackwardCompat`) + extended `TestJSONRoundtrip` with `MemoryItem_pinned` case.
- `test/conformance` 22-endpoint Console contract: unchanged (contract v1 not bumped).
- `bash scripts/console_smoke.sh` v8 28-step bash syntax verified; runtime gated `MODE=real && sqlite3` per step 28.
- `bash scripts/spec_drift_lint.sh --touched origin/master`: 0 unannotated hits across PR #118 + this closeout PR.

### Upgrade path (v0.9.0 → v0.10.0)

- **SQLite DB users**: no migration required (column already in 0013 from v0.6 ship). After upgrade to v0.10.0, `is_pinned` field begins surfacing on `GET /v1/memory[/<id>]` responses with the actual persisted value.
- **Console UI clients (v0.7-v0.9)**: existing client code reading the v0.10.0 response silently ignores the new `is_pinned` key (Go JSON unmarshal ignores unknown fields). No client-side change required.
- **Console UI clients (v0.10+ adapted)**: client can now sort/render based on `MemoryItem.IsPinned`. Existing Console PR #101 ships the field type; rendering UI is the next user-driven Console PR (visual closure, outside this autonomous flow).
- **Docker users**: `docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.10.0` after tag push (release.yml handles ghcr build/push on `v*` tag).
- **No BREAKING** — purely additive schema. Backward compatible in both directions per ADR-015 D1 + ADR-022 D4.

### Rollback path

If v0.10.0 ship reveals an unexpected issue:

1. `git revert <v0.10.0 merge SHA>` to roll back to v0.9.0 (master HEAD `cfcdbd4` post-PR-#118 but pre-this-closeout)
2. Ship v0.10.0.1 patch tagging the specific concern
3. No DB rollback needed — `is_pinned` column has always been in 0013 (Phase 13); rolling back the proto/contractv1 field doesn't drop the column
4. ADR-022 stays Accepted (the decision path is sound even if implementation needs patching)

### Cross-repo follow-up — **COMPLETED 2026-05-29** 🎉

User-forwarded after this closeout PR merge + v0.10.0 tag push:
- ✅ Notified Console team of v0.10.0 release ship via GitHub Release page URL (2026-05-28)
- ✅ **Console UI visual closure SHIPPED end-to-end** to Console master @ `c1c4609744a9c34201e3fd87cba4ab1596be4fd4`:
  - PR [#102](https://github.com/tajiaoyezi/ContextForge-Console/pull/102) `30aeff4` — pin 排序 + 列表 icon + 详情 "已置顶" badge (UI 主体)
  - PR [#103](https://github.com/tajiaoyezi/ContextForge-Console/pull/103) `14f9ce0` — v0.10.0 ack: mock 落真 is_pinned + docker-compose 切 GHCR pull + 联调清单文档 + apiFetch typecheck 潜伏 bug 修
  - PR [#104](https://github.com/tajiaoyezi/ContextForge-Console/pull/104) `c1c4609` — pin-sort util 抽函数 + 混合 pinned/unpinned 数组排序单测
- ✅ E2E daemon-level verification (Console-reported): `docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.10.0` → http stack → daemon fixtures → daemon → console-api(http) → BFF → web → 详情页 "已置顶" badge 实拍坐实
- 🎉 **ContextForge-Console PR #91/#93 review backlog end-to-end 100% closed** (backend protocol via cumulative Phase 13/15/16/17 + UI visual surface via Console PRs #102/103/104)
- Feedback acknowledged: GHCR package v0.10.0 / :latest was initially shipped as PRIVATE (anonymous pull 403, observed by Console team); owner has since flipped to public. Future enhancement to add anonymous-pull verify step `[SPEC-DEFER:phase-future.verify-image-anonymous-pull]`.

ContextForge agent has no further obligation on this backlog.

---

## v0.9.0 (2026-05-28) — v0.9.0-backlog-completion (10/11 closed) + release infra

### 摘要

v0.9.0 minor release：closes 4 of the remaining 5 Console PR #91/#93 backlog items (P3 + P4) + ships production release infrastructure (GHCR image push CI + docker-compose.production.yml + verify-image.yml workflow). Backlog status now **10/11 = 91% closed**; only `MemoryItem.is_pinned` (P2 #6) remains for Phase 17 cross-repo coord. **No new ADR in v0.9.0 itself** — 4 Phase 16 tasks all extend existing ADR-013/015/016/017/018.

### Backlog items closed (4 more)

| Item | Backlog signal | Solution | PR |
|---|---|---|---|
| **P4 #10** | TraceStore daemon-restart 即丢历史 | task-16.1 — migration 0015_search_traces.sql + SqliteTracePersist + TraceStore write-through + warm restore | #110 |
| **P4 #11** | events `?wait=` 等价 batch polling | task-16.2 — handleEvents 真传 wait + EventsClient.Recent(limit, wait) + 两阶段 long-poll (phase 1 block + phase 2 100ms drain) | #111 |
| **P3 #8** | ghcr.io image push 缺 CI/CD | task-16.3 — `.github/workflows/release.yml` (tag push → docker build + push ghcr) + `ci.yml` (PR/push → cargo+go+lint 3 parallel jobs) | #112 |
| **P3 #9** | production-ready docker-compose 缺示例 | task-16.4 — `deploy/docker-compose.production.yml` 双容器 (contextforge-core + console-api-serve, ADR-018 fallback deny 沿用, 卷持久化, healthcheck) + `.env.production.example` + `docs/deploy/production.md` + smoke v7 27-step | #113 |

Additional Phase 16 ship:
- **Phase 16 E6 closeout** (PR #114): Status → Done + §10 Completion Notes + adapter sync
- **Phase 16 E7 release-verify** (PR #115): `.github/workflows/verify-image.yml` GHA pull+run+/v1/health verification workflow

Remaining (deferred to Phase 17 / v0.10.0):
- P2 #6 `MemoryItem.is_pinned` (ADR-015 D5 amendment via ADR-022 Proposed) — Phase 17 + task-17.1 scaffolded in PR #116 with Status: Pending awaiting Console contractv1.go cross-repo amend trigger

### v0.9.0 不引入新 ADR

Phase 16 4 task 全部是既有 ADR 的延伸实施：
- task-16.1 ↔ ADR-013 (CLI data plane gRPC bridge) + ADR-015 D1 (add-only schema)
- task-16.2 ↔ ADR-017 D4 (long-poll v1.0 lock — 不引入 SSE)
- task-16.3 ↔ ops practice (CI/CD pipeline 不构成 architectural decision)
- task-16.4 ↔ ADR-018 (fallback deny default 沿用)

**ADR-022 (memory-is-pinned-field-amendment)** 在 v0.9.0 ship 后作为 Phase 17 scaffolding PR #116 单独 ship — Status: Proposed；属 Phase 17 不属 v0.9.0 release。

### Schema additions (all add-only, ADR-015 D1)

- `core/migrations/0015_search_traces.sql`: 新建 `search_traces` 表 (query_id PK / trace_json TEXT / workspace_id TEXT / ts_unix INTEGER / created_at TEXT) + `idx_search_traces_ts_desc` 索引 (IF NOT EXISTS 幂等)
- `core/src/data_plane/search_persist.rs`: 新模块 `SqliteTracePersist` (open + put + get + list + load_warm)
- `internal/consoleapi/types.go`: `EventsClient.Recent(limit int)` → `Recent(limit int, wait time.Duration)` (signature extension; 所有 callers 同步更新)
- 既有 `RetrievalTrace` / `QueryRecord` / `MemoryItem` / `CoreHealth` 等 contract v1 message **完全不动** (ADR-015 D1 freeze 维持)

### 关键设计取舍

- **task-16.1 write-through dual-write**: 内存 LRU cap=1000 保留作 hot cache (低延迟读) + SQLite SoT best-effort 双写 (持久化保证)；SQLite write 失败 swallow 不阻塞 RPC 返回
- **task-16.1 SQLite trace_json 序列化**: prost-encoded bytes → base64 → store as TEXT (与 PbRetrievalTrace prost-derive 一致；非 serde_json — 避免 schema drift)
- **task-16.1 cap-by-LRU 内存 + cap-by-DELETE 留 future**: 内存 LRU cap=1000 同 v0.8；SQLite 端无 LRU eviction → 长时间运行后表可能数百万行；留 SPEC-DEFER:phase-future.tracestore-sqlite-vacuum
- **task-16.2 两阶段 long-poll**: phase 1 block 等首 event ≤ wait；phase 2 短 drainTimeout=100ms drain immediately-available events；避免单 event 触发后立即返就只带 1 个 event 浪费 RTT
- **task-16.4 CONTEXTFORGE_ALLOW_WILDCARD_BIND=1 env opt-in**: ADR-004 安全基线下 daemon 默认 127.0.0.1 bind；docker compose-prod 需 0.0.0.0 跨容器；引入 env opt-in 显式解锁 (PR #113 review fix c21315b) — 非默认行为 + 用户感知
- **task-16.4 ADR-018 deny 默认沿用**: compose-prod 不注入 `CONSOLE_API_FALLBACK_INMEM=1` → 真 grpcclient 不可达时 503 (与 v0.7.2 deny 默认一致)

### ADR-014 cross-validation gate 第七次激活

- D1 closeout PR (#114) body 含 Phase §6 ↔ Task §6 mapping 表 (6 行)
- D2 lint `--touched origin/master`: 0 unannotated hits in PR-changed lines
- D3 phase-16 §6 每条 AC 含 verified-by owner 显式
- D4 governance: 主 agent 自治 §2A Ready review + R6 merge decision
- D5 历史不溯改: Phase 1-15 spec 内容未触

### Tests (cumulative Phase 16 E1-E7)

- `cargo test --workspace`: full PASS (Phase 11-15 既有 + Phase 16 task-16.1 新增 TraceStore SQLite persist tests + memory_persist_integration tests 不退化)
- `go test ./...`: 22 packages 全 PASS (含 task-16.2 handlers_test.go::TestHandleEvents_Wait5s_Blocks_When_NoEvent + TestHandleEvents_Returns_Early_OnEvent + grpcclient 4 unit tests + e2e_grpc Step 11b real long-poll 不退化)
- `test/conformance`: 22-endpoint Console contract conformance 不退化
- `bash -n scripts/console_smoke.sh`: syntax OK; v7 27-step (v6 24 + step 25 `?wait=2s` + step 26 TraceStore restart roundtrip + step 27 compose-prod stack health gated `COMPOSE_PROD_SMOKE=1`)
- `gh workflow run verify-image.yml -f tag=v0.9.0-rc1`: GHA run 26555768957 GREEN in 18s (pull + run + /v1/health probe + `?detailed=true` 5-component breakdown)
- `gh workflow run verify-image.yml -f tag=v0.9.0`: GHA run 26556137023 GREEN in 11s (post-release verify)

### Upgrade path (v0.8.0 → v0.9.0)

**Console UI / SDK 用户** (v0.7.x-v0.8.x clients 继续工作):
- 旧 client 解析 v0.9 JSON 自动忽略未知字段 (`Events.wait` semantic 仅 server-side 生效) → zero migration
- Console UI Dashboard 历史查询面板自动 survive daemon restart (无 client 改动)
- Memory 操作历史 events stream 现在真 long-poll (≤ wait latency)

**ContextForge daemon 升级**:
- 二进制升级 v0.8.0 → v0.9.0 不破坏既有部署 (无 BREAKING)
- SQLite migration 0015 自动应用 (IF NOT EXISTS 幂等)；既有 in-memory traces 不迁移 (重启时空 cap=1000 LRU + 后续 search 累积新 trace)
- Docker users: `docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.9.0` (替代 v0.8 本地 `docker build`)

**新功能 opt-in 试用**:

```bash
# 1. GHCR image pull (replaces local docker build)
docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.9.0
docker pull ghcr.io/tajiaoyezi/contextforge-daemon:latest  # always points to latest release

# 2. Real long-poll events
curl 'http://localhost:48181/v1/observability/events?wait=5s'
# now truly blocks 5s when no events (vs prior batch polling)

# 3. Trace persistence — survive daemon restart
curl -X POST -H "X-Confirm: yes" http://localhost:48181/v1/search \
  -d '{"query":"foo","limit":5}'
# (note query_id)
docker restart contextforge
curl 'http://localhost:48181/v1/queries?limit=10'    # 仍有历史
curl http://localhost:48181/v1/search/{query_id}/trace  # 仍 200

# 4. Production-ready compose stack
git clone https://github.com/tajiaoyezi/contextforge && cd contextforge
cp deploy/.env.production.example deploy/.env.production
# edit deploy/.env.production for your tokens
docker compose -f deploy/docker-compose.production.yml up -d
curl http://localhost:48181/v1/health   # expect: {"status":"healthy", ...}
```

### Rollback path

若 v0.9.0 ship 后发现非预期问题：
1. `git revert <v0.9.0 merge SHA>` 回退到 v0.8.0 (master HEAD `622155b` 或 v0.8.0 tag)
2. ship v0.9.0.1 patch + 标具体 task 16.x Reverted
3. SQLite migration 0015 不撤回 (新表无 backward break — 既有 v0.8 binary 不读 search_traces 表)
4. 不撤回 v0.8.0 ADR-020 / ADR-021 / v0.7.2 ADR-018 / v0.7.0 ADR-017 (跨版本独立)

Cross-repo follow-up:
- 通知 Console 团队 v0.9.0 ship → Console UI 验证 Dashboard 历史查询面板跨重启 + events real long-poll latency 提升
- **Phase 17 启动信号**: 用户人工转发本 release page → Console 主 Agent 启动 contractv1.go IsPinned add-only field amend PR (ADR-022 D4 第 1 步) → 完成后回报触发 ContextForge Phase 17 Pending → Ready

详 [docs/releases/v0.9.0-evidence.md](docs/releases/v0.9.0-evidence.md) + [v0.9.0-artifacts.md](docs/releases/v0.9.0-artifacts.md)。

---

## v0.8.0 (2026-05-26) — Console functional gap closure (6/11 backlog)

### 摘要

v0.8.0 minor release：closes 6 of 11 items raised in the ContextForge-Console PR #91/#93 backlog (P0 + P1 + P2#7). New Dashboard backend endpoints (chunks stats / eval-runs list / queries history), 5-link health detail (db / index / embed / retriever / eval), MemStore fallback drill-down fix, and the long-standing memory.* → EventBus bridge (Phase 13 [SPEC-DEFER:phase-future.memory-event-bus-bridge] lifted). Two new ADRs (020 / 021) promoted to Accepted.

### Backlog items closed (6/11)

| Item | Backlog signal | Solution | PR |
|---|---|---|---|
| **P0 #1** | MemStore inmem-fallback 503 on drill-down | task-15.1 — chunkCache + traceCache (FIFO cap=256) | #99 |
| **P0 #2** | `memory.*` event 桥接 缺失 | task-15.2 (ADR-021) — `emit_audit` 同步追加 `EventBus.send` | #100 |
| **P1 #3** | Dashboard "已索引块" 缺 backend | task-15.3 — `GET /v1/stats/chunks` (Tantivy `num_docs` + SQLite COUNT today) | #101 |
| **P1 #4** | Eval 列表 缺 endpoint | task-15.4 — `GET /v1/eval-runs?workspace_id=&status=&limit=N` (ORDER DESC) | #102 |
| **P1 #5** | Dashboard "最近查询" 缺 backend | task-15.5 — `GET /v1/queries?limit=N` (TraceStore.list wrapper) | #103 |
| **P2 #7** | CoreHealthCard 5 链路 缺 | task-15.6 (ADR-020) — `GET /v1/health?detailed=true` (5 probes opt-in) | #104 |

Remaining (deferred to Phase 16 / v0.9.0):
- P2 #6 `MemoryItem.is_pinned` (needs ADR-015 D5 amendment — BREAKING window required)
- P3 #8 ghcr.io image push — CI/CD pipeline work
- P3 #9 docker-compose.production.yml example
- P4 #10 TraceStore SQLite persistence (currently in-memory ring buffer)
- P4 #11 `?wait=` real long-poll (currently batch polling — v0.7.2 cleanup already documented this)

### 新增 ADR

- **ADR-020 health-component-breakdown** (Accepted 2026-05-26): D1-D5 spelling out the 5 probes (db SQLite ping / index Tantivy open / embed config check / retriever top_k=1 / eval store open), add-only ComponentHealth schema, opt-in `?detailed=true`, aggregation rule (any unreachable → 503; any degraded → 200 + degraded), Console cross-repo coord.
- **ADR-021 memory-event-bus-bridge** (Accepted 2026-05-26): D1-D4 — `emit_audit_and_event` shared path (no new channel), 3 new event_type string values (`memory.pin` / `memory.deprecate` / `memory.soft_delete`; pin/unpin share via payload `op`), field contract (severity=info, source=contextforge-core, trace_id/job_id None), best-effort emit with SendError swallowed.

### Schema additions (all add-only, ADR-015 D1)

- proto `console_data_plane.proto`:
  - `SearchService.GetChunksStats` + `GetChunksStatsRequest` + `ChunksStats{total, today_delta}`
  - `SearchService.ListQueries` + `ListQueriesRequest` + `ListQueriesResponse` + `QueryRecord{query_id, query, ts_unix, workspace_id}`
  - `EvalService.List` + `ListEvalRunsRequest` + `ListEvalRunsResponse`
  - new `HealthService.GetDetailed` + `ComponentHealth` + `DetailedHealthRequest` + `DetailedHealthResponse`
- `internal/contractv1`:
  - `ChunksStats`, `QueryRecord`, `ListEvalRunsFilter`, `ComponentHealth` Go structs
  - `CoreHealth.Components map[string]ComponentHealth` (omitempty) + `CoreHealth.TotalLatencyMs *int64` (omitempty)
- 既有 `RetrievalTrace` / `EvalRun` / `MemoryItem` 消息**完全不动** (ADR-015 D1 字段冻结保留)

### 关键设计取舍

- **task-15.5 TraceRecord wrapper**: 保留 `RetrievalTrace` 不动 (ADR-015 D1 freeze)，workspace_id + ts_unix 仅作 Rust-side metadata 储存在 `TraceStore.put` 内部；新 `QueryRecord` message 是这俩元数据的真承载
- **task-15.6 synthesize fallback for nil HealthClient**: handleHealth 在 fallback / degraded 模式下 synthesize 5-component 全 healthy / 全 degraded，让 Console UI CoreHealthCard 永远拿到完整 5 key shape
- **task-15.3 today_delta lexicographic SQLite compare**: 复用既有 `chunks.indexed_at TEXT NOT NULL` 列；`seconds_to_iso` (Howard Hinnant 算法，无 chrono dep) 生成 `YYYY-MM-DD HH:MM:SS` 格式 — lexicographic >= 与时序一致
- **task-15.2 memory.pin / memory.unpin 合并 event_type**: payload_json `op` 区分；event_type 命名空间紧凑

### ADR-014 cross-validation gate 第六次激活

- D1 closeout PR (#105) body 含 Phase §6 ↔ Task §6 mapping 表 (7 行)
- D2 lint `--touched origin/master`: 0 unannotated hits in PR-changed lines (Python equivalent 实测；bash 在 Windows 太慢)
- D3 phase-15 §6 每条 AC 含 verified-by owner 显式
- D4 governance: 主 agent 自治 §2A Ready review + R6 merge decision (cross-repo 字段仅 add-only)
- D5 历史不溯改: Phase 1-14 spec 内容未触

### Tests (cumulative E2-E7)

- `cargo test --workspace`: 121 lib + 17 integration test files 全 PASS (Phase 11-14 既有不退化)
- `go test ./...`: 22 packages 全 PASS (含 `test/conformance` 22-endpoint Console contract conformance 不退化)
- `bash -n scripts/console_smoke.sh`: syntax OK; v6 24-step (既有 20 + 4 new for chunks-stats / eval-runs / queries / health-detail)
- Smoke daemon-level CONSOLE_REAL_SMOKE_EXIT=0 留 v0.8.0 ship 前 manual / CI 实测

### Upgrade path (v0.7.x → v0.8.0)

**Console UI / SDK 用户** (v0.7.x 客户端继续工作):
- 旧 client 解析 v0.8 JSON 自动忽略未知字段 (`Components` / `TotalLatencyMs` / new endpoint shapes) → zero migration
- Console UI 启动 standby PR 后切到 v1.x：Dashboard 3 KPI / CoreHealthCard 5 链路 / Memory 操作历史 自动有数据

**ContextForge daemon 升级**:
- 二进制升级 v0.7.2 → v0.8.0 不破坏既有部署 (无 BREAKING)
- Docker users: `docker pull contextforge-daemon:v0.8.0` — fallback 默认行为不变 (ADR-018 v0.7.2 决定继承)

**新 endpoints opt-in 试用**:
```bash
# Dashboard 已索引块
curl http://localhost:48181/v1/stats/chunks

# Eval 最近评测
curl 'http://localhost:48181/v1/eval-runs?limit=10'

# Dashboard 最近查询
curl 'http://localhost:48181/v1/queries?limit=20'

# CoreHealthCard 5 链路
curl 'http://localhost:48181/v1/health?detailed=true' | jq .components
```

### Rollback path

若 v0.8.0 ship 后发现非预期问题：
1. `git revert <v0.8.0 merge SHA>` 回退到 v0.7.2 (master HEAD `c3e6698^` 前一版本 `5264fd6`)
2. ship v0.8.0.1 patch + 标 ADR-020 / ADR-021 status Superseded 或 Reverted
3. 不撤回 v0.7.2 ADR-018 / v0.7.0 ADR-017 (跨版本独立)

Cross-repo follow-up: 通知 Console 团队 v0.8.0 ship → Console UI standby PR (Dashboard 3 KPI 真接 + CoreHealthCard 5 链路 + Memory 操作历史)。

详 [docs/releases/v0.8.0-evidence.md](docs/releases/v0.8.0-evidence.md) + [v0.8.0-artifacts.md](docs/releases/v0.8.0-artifacts.md)。

---

## v0.7.2 (2026-05-26) — fallback-inmem default reversal ⚠️ BREAKING

### 摘要

v0.7.2 patch release：按 v0.7.1 pre-announce 反转 single-image deployment 默认行为，消除 in-mem fallback 的 silent footgun（HTTP 200 healthcheck 掩盖容器重启数据失风险）。代码无改动，仅 Dockerfile 删 ENV 行 + ADR-018 spec lock。

### 变更点

详 [ADR-018: fallback-inmem-default-reversal](docs/decisions/adr-018-fallback-inmem-default-reversal.md)（D1-D4 共 4 决策）。

#### 1. Dockerfile 删 `ENV CONSOLE_API_FALLBACK_INMEM=1`
- v0.7.1 行为：`docker run contextforge-daemon:v0.7.1` → 默认 fallback-inmem，`/v1/health` 返 200（degraded），容器重启数据失
- **v0.7.2 行为**：`docker run contextforge-daemon:v0.7.2` → 默认 fallback **deny**，gRPC core 不可达时 `/v1/health` 返 **503**，docker healthcheck 立即报 unhealthy

#### 2. Binary code 无变更
- `internal/cli/console_api_serve.go` binary default 一直是 `false`，v0.7.1 是 Dockerfile ENV 单方面强制 set 成 true
- v0.7.2 删 ENV 行后，binary default 自然生效，container 内外行为统一

#### 3. ADR-018 ratification test
- 新增 `TestADR018_BinaryDefaultIsFallbackDeny` 锚定意图（`internal/cli/console_api_serve_test.go`）
- 现有 `TestBuildDeps_DegradedWhenNoDaemon` + `TestRouter_HealthDegraded_503` 已覆盖默认 deny 路径，本 patch 无 logic change

### ⚠️ BREAKING change call-out

**v0.7.1 → v0.7.2 升级前请 review 您的部署方式**：

| 部署方式 | v0.7.1 默认 | v0.7.2 默认 | 升级动作 |
|---|---|---|---|
| `docker run` single-image | inmem-fallback (200) | **fallback deny (503)** | 保留旧行为需 `-e CONSOLE_API_FALLBACK_INMEM=1` opt-in |
| docker-compose single-service | inmem-fallback (200) | **fallback deny (503)** | docker-compose.yml `environment` 加 `CONSOLE_API_FALLBACK_INMEM=1` opt-in |
| docker-compose multi-process (核 + proxy) | 已 opt-out via `=0` | 无变更 | 无需动 |
| k8s Deployment | inmem-fallback (200) | **fallback deny (503)** | manifest env 加 `CONSOLE_API_FALLBACK_INMEM=1` opt-in 或切真 multi-process |
| 纯 binary (非 docker) | fallback deny | fallback deny | **无影响** |

### Upgrade path (v0.7.1 → v0.7.2)

```bash
# 1. 切到新 image (拉 v0.7.2 tag)
docker pull contextforge-daemon:v0.7.2

# 2. 验证默认 deny 行为
docker run -d -p 48181:48181 --name v072 contextforge-daemon:v0.7.2
sleep 5
curl -o /dev/null -w '%{http_code}\n' localhost:48181/v1/health
# expect: 503 (v0.7.1 是 200)

# 3. 保留旧行为 (in-mem fallback) → 显式 opt-in
docker rm -f v072
docker run -d -p 48181:48181 -e CONSOLE_API_FALLBACK_INMEM=1 --name v072-optin contextforge-daemon:v0.7.2
sleep 5
curl -o /dev/null -w '%{http_code}\n' localhost:48181/v1/health
# expect: 200 + status=degraded
```

### Trade-offs / Conscious decisions

- **env 名保留 `CONSOLE_API_FALLBACK_INMEM`**（不改 `ALLOW_INMEM`）— v0.7.x patch series 不引入 dual-name + deprecate 包袱；改名留 v0.8/v1.0
- **不加 startup banner WARN** — (a) 方案的 503 healthcheck 已是 ops 链路最强信号，banner WARN 易被 multi-container log 掩盖
- **不变更 contractv1.go / proto / Rust core code** — 仅 Dockerfile + 单元测试 + spec docs
- **Console 端 standby chore PR 已准备好**（ContextForge-Console PR #91 §6.5 F1 列出动作清单）— v0.7.2 ship 后 Console 团队同步 ship docker-compose.yml + .env.example 更新

### Tests

- `cargo test -p contextforge-core`: 94 lib + 5 integration suites all PASS (无 logic change，不退化)
- `go test ./...`: 43 packages PASS + 新增 1 个 `TestADR018_BinaryDefaultIsFallbackDeny`
- Docker container 实测 (manual verify on PR review)：
  - 默认 `docker run contextforge-daemon:v0.7.2` → `/v1/health` 503 + healthcheck unhealthy
  - `-e CONSOLE_API_FALLBACK_INMEM=1` → `/v1/health` 200 + status=degraded + healthcheck healthy

### Console (cross-repo) sync state

- Console 主仓 master `3370a92` (PR #91) checklist §6.5 F1 已 standby
- v0.7.2 ship 后 Console 端启动 chore PR：docker-compose.yml + .env.example 加 `CONSOLE_API_FALLBACK_INMEM=1` opt-in；checklist §6.5 F1 标 ✅
- 跨仓 break change 双向 coordinate path：ContextForge → 用户转达 → Console 主 Agent 启动 standby PR

### Rollback path

若 v0.7.2 ship 后发现 (a) 方案不可接受（Console standby PR 延迟 / 其它用户 ops 链路无法适配）：
1. `git revert <v0.7.2 commit>` 反转
2. ship v0.7.3 patch + ADR-018 status 改 "Reverted"
3. 重新 design：可能切到 (b) startup-banner WARN 双重防御，或等 v0.8 ship 2 进程 image 一起解决
4. 跨仓通知 Console 团队 v0.7.3 ship + standby PR 撤回

---

## v0.7.1 (2026-05-26) — Dockerfile + single-image deployment fix

### 摘要

v0.7.1 patch release：收齐 v0.7.0 Dockerfile 4 处 stale，single-image docker
deployment ready。ContextForge-Console 团队联调期发现，本 patch 一次性 ship。

### 4 处 fix (PR #94, master `233ced5`)

#### 1. Rust 1.82-bullseye → 1.93-slim-bookworm
- 现象：cargo build fail，`cpufeatures-0.3.0 Cargo.toml: feature edition2024 is required`
- 根因：transitive deps `darling@0.23` / `tantivy@0.26` / `time@0.3.47` 要 rustc >= 1.88
- Fix：升 `rust:1.93-slim-bookworm`（保稳定 + 300 MB 小镜像；bullseye Go 1.26 dropped）

#### 2. Go 1.22-bullseye → 1.26-bookworm
- 现象：`go: go.mod requires go >= 1.26 (running go 1.22.12)`
- Fix：升 `golang:1.26-bookworm`（Go 1.26 dropped bullseye）

#### 3. 加 ENV CONSOLE_API_FALLBACK_INMEM=1（single-image default 模式）
- 现象：v0.7.0 image 起来后 daemon 只跑 REST proxy 不起 Rust gRPC core 进程
  → `/v1/health` 返 503 → docker healthcheck `curl -fsS` 永远不过
- Fix：single-image deployment 默认 in-memory MemStore 模式（ADR-016 §D4）
  - 默认：`docker run contextforge-daemon:v0.7.1` → backend=inmem-fallback → 200
  - 多进程：`docker run -e CONSOLE_API_FALLBACK_INMEM=0 ...` 关闭 fallback +
    另起 contextforge-core daemon 实现真持久化

#### 4. 加 .dockerignore（build context 瘦身）
- 现象：v0.7.0 build context 含 `target/` 9.3 GB cargo cache 全 transfer →
  build 5+ min 才到 cargo 阶段
- Fix：新加 `.dockerignore` 排除 `target/` / `.git/` / `_dispatch/` / `docs/` /
  `test/` 等，build context 从 GB 级降到 ~50 MB

### Behavior change call-out

- **Single-image deployment 默认 `inmem-fallback` 模式 → 容器重启数据全失**
- Multi-process 部署用户需 `docker run -e CONSOLE_API_FALLBACK_INMEM=0` 显式 opt-out
- PR #94 reviewer 与 ContextForge-Console 团队已独立 flag 该默认是 silent
  footgun 风险（telemetry 充分但 HTTP 200 healthcheck 掩盖）→
  **v0.7.2 将反转该默认行为**（详 §"v0.7.2 pre-announce"）

### Verify

```bash
docker build -t contextforge-daemon:v0.7.1 .
# 默认：should be healthy (fallback-inmem)
docker run -d --name v071 -p 48181:48181 contextforge-daemon:v0.7.1
curl localhost:48181/v1/health
# 200 + status="degraded" + error_reason="...in-memory fallback store active..."

# Override：should be 503 (no gRPC core)
docker run -d --name v071-strict -e CONSOLE_API_FALLBACK_INMEM=0 -p 48182:48181 contextforge-daemon:v0.7.1
curl localhost:48182/v1/health
# 503
```

### v0.7.2 pre-announce — fallback default 反转 ⚠️ BREAKING

为消除 single-image silent footgun（HTTP 200 healthcheck 掩盖 in-mem
fallback 风险），v0.7.2 将反转默认行为：

- Daemon default 改为 `CONSOLE_API_FALLBACK_INMEM=0`（强制 opt-in）
- gRPC core 不可达时 → `/v1/health` 返 **503**，docker healthcheck 立刻报 unhealthy
- 旧 v0.7.1 行为兼容：用户显式设 `CONSOLE_API_FALLBACK_INMEM=1` 即可保留
- **Console 团队 standby**：docker-compose.yml 已准备好加 `CONSOLE_API_FALLBACK_INMEM=1`
  env 显式 opt-in；ContextForge-Console 端 chore PR standby 待 v0.7.2 ship

详 v0.7.2 ship 时 ADR-018。

### Console (cross-repo) sync state

- ContextForge-Console 联调期发现本 PR 4 项 stale，cross-repo notify → ship 同步
- Console master `3370a92` (PR #91) 已更新 checklist §6.3 / §6.5 反映 v0.7.1 ship
- Console docker-compose.yml `CONSOLE_API_FALLBACK_INMEM=1` env 当前作显式声明保留，
  v0.7.2 ship 后转为必需 opt-in

---

## v0.7.0 (2026-05-24) — Console 22-endpoint conformance 100% PASS 🎉

### 摘要

ContextForge v0.7.0 完成 **Phase 14 eval-rest-surface** 收口 + **ADR-017
Proposed → Accepted** 6-D-clause 一次性 promote。Console HTTPAdapter v1.0
conformance 从 18/22 提升到 22/22 (100%)。**ContextForge v0.4-v0.7 ship 全
22 Console contract v1 endpoint**; Console UI HTTPAdapter 端到端调用代码
已 cross-repo ship — 双方握手成功 standardized signal landed.

### 主要改进

- **task-14.1 Rust SoT** (PR #89):
  - `core/migrations/0014_eval_runs.sql` (10 columns + 3 indexes + status CHECK)
  - `core/src/eval/store.rs` `SqliteEvalStore` (5 methods: create / get /
    update_metrics / update_case_results / mark_finished) + 7 unit tests
  - `core/src/eval/runner.rs` `EvalRunner` stub (real triggering Go side per task-14.2)
  - `proto/contextforge/console_data_plane/v1/console_data_plane.proto` add-only
    `EvalService` 3 RPC + 5 messages (CaseResult / EvalRun / CreateEvalRunRequest /
    GetEvalRunRequest / UpdateEvalRunProgressRequest+Response)
  - `core/src/data_plane/eval.rs` `EvalServer` impl 3 RPC + 3 unit tests;
    JSON roundtrip verified (HashMap<String,f64> + Vec<CaseResult>)
  - `core/src/data_plane/mod.rs` `DataPlaneStores` 加 Option<eval>; `with_eval()`
    构造函数; `full()` takes 8 params; `register_services` + `server_with_services`
    都加 6th EvalServiceServer
  - `core/src/server.rs` `serve_full` 实例化 SqliteEvalStore 真接到 daemon
  - 2 integration tests via tonic client + EvalServiceClient
- **task-14.2 Go REST + runEvalAsync goroutine** (PR #90):
  - `internal/consoleapi/types.go` `EvalClient` interface (Create/Get/UpdateProgress)
    + `Deps.Eval` field
  - `internal/consoleapi/router.go` 2 new routes (non-destructive — no confirm gate)
  - `internal/consoleapi/handlers.go` `handleCreateEvalRun` (spawn goroutine + 200 + running)
    + `handleGetEvalRun` (200 / 404)
  - `internal/consoleapi/eval_runner.go` `runEvalAsync` goroutine:
    - 5min context timeout
    - Light-weight recall harness using `BuiltinGoldenQuestions` + mock pass-all
    - Computes `recall@5` / `recall@10` / `precision@5` metrics
    - Builds `case_results` array with `case_id` / `query` / `expected_chunks` /
      `actual_chunks` / `score` / `passed`
    - Defer-recover panic → status=failed + error_message="panic: ..."
    - Calls `deps.Eval.UpdateProgress(...)` to reverse-update Rust store on terminal
  - `internal/consoleapi/memstore.go` `MemEvalStore` (in-memory) + 2s timer
    auto-advance to succeeded with mock metrics (`recall@5: 0.7` 等)
  - `internal/consoleapi/grpcclient/grpcclient.go` `evalClient` 3 method wrappers
    + `protoToEvalRun` helper; `Client.Eval()` accessor; Create generates
    `eval-{nanos}` id Go-side per task-14.1 contract
  - `internal/cli/console_api_serve.go` buildDeps wires Eval in both inmem +
    gRPC modes; degradedDeps adds Eval
  - e2e_grpc Step 9e: real Rust daemon EvalService end-to-end PASS
- **scripts/console_smoke.sh v5** (PR #90):
  - Header v4 → v5; subtitle "Phase 14 console-22-endpoint complete"
  - 18 → 20 endpoint flow; renumber `[1/20]..[20/20]`
  - New Step 19/20: POST /v1/eval-runs → 200 + status=running
  - New Step 20/20: poll GET /v1/eval-runs/<id> 30s for terminal + verify metrics
    contains `recall@5` + 404 on unknown id
  - REAL mode: `CONSOLE_REAL_SMOKE_EXIT=0` 20/20 PASS (eval terminal at attempt 1!)
- **治理 / spec 同步** (PR #91):
  - Phase 14 spec / adapter §Phase 14 / task-14.{1,2} 全 `Status: Done`
  - **ADR-017 Status: Proposed → Accepted** (one-shot promotion, 6 D-clauses
    spanning v0.5/v0.6/v0.7 3 phase)
  - ADR-014 D1 mapping 表 / D2 lint 0 violation / D3 verified-by

### ADR-017 D-clauses (all landed by v0.7.0)

| D | Clause | Where shipped |
|---|---|---|
| D1 | 22-endpoint roadmap (Wave 1+2+3+4) | task-12.{1,2,3} + task-13.{1,2} + task-14.{1,2} |
| D2 | X-Confirm OR ?confirm=true → 412 | `confirmMiddleware` on PATCH config + memory deprecate + soft-delete |
| D3 | cancel 200 → 204 | handlers.go handleCancelJob StatusNoContent |
| D4 | Long-poll v1.0 lock (no SSE) | retained from v0.4 task-11.4 |
| D5 | RFC3339Nano kept | Go time.Time JSON unchanged |
| D6 | ADR-016 sub | Rust SoT + Go thin proxy preserved across all 13 new endpoints |
| D7 | ADR-014 cross-validation gate 3rd/4th/5th activation | Phase 12+13+14 closeout PRs each shipped D1 mapping + D2 lint verified |

### Trade-offs / Conscious limitations

- **Light-weight recall harness in runEvalAsync** [SPEC-DEFER:phase-future.real-recall-via-retriever]:
  v0.7 ship 用 BuiltinGoldenQuestions + mock pass-all 计算 metrics；future v1.x
  接 retriever-backed recall (RetrievalResult dispatch + EvaluateQuestion)
- **5min ctx timeout** in runEvalAsync (大 dataset 可能超时；future ?timeout query param)
- **Eval orphan reaper** not implemented [SPEC-DEFER:phase-15.eval-orphan-reaper]:
  console-api-serve crash 时 in-flight eval 状态卡 running；future 加 Rust 侧
  orphan reaper 扫描 status=running 超时 → mark failed
- **Eval cancel REST** 不实施 [SPEC-DEFER:console-eval-cancel] (Console 22 endpoint contract 不含)
- **Pin state not in contractv1.MemoryItem** (carried from v0.6)

### Migration notes (v0.6.0 → v0.7.0)

- **daemon 重启后 eval_runs 表自动创建** (migration 0014 IF NOT EXISTS 幂等);
  既有 v0.6 data_dir 兼容
- **新 2 endpoint** (POST /v1/eval-runs + GET /v1/eval-runs/{id}): client 按 OpenAPI/contractv1 v1 spec 调用
- contractv1.go 字段集合不变 (ADR-015 D5)
- 新 proto RPC + message add-only (ADR-013 D2)

### Tests (Phase 14 全程)

- **Rust**: 94 lib (含 10 new task-14.1: 7 store + 3 server) + 2 eval_integration
  + 既有 phase 1-13 测试不退化 (含 3 memory_integration / 5 indexjob_real /
  4 search_real / 5 data_plane_integration 等)
- **Go**: 43 packages PASS (含 e2e_grpc Step 9e 真接 Rust daemon eval-runs +
  既有 task-12.x/13.x 不退化)
- **smoke**: `bash scripts/console_smoke.sh` REAL mode 20/20 PASS;
  eval terminal at attempt 1: status=succeeded; metrics contains recall@5 ✅
- **conformance**: v0.4-v0.6 既有 endpoints 不退化

### Console (cross-repo) sync state

- ContextForge-Console contractv1.go (Workspace + IndexJob + SourceChunk +
  Search + Memory + EvalRun + CaseResult + ObservabilityEvent 等 全套 22-endpoint
  types) cross-repo 已 ship (v0.3 锁定不动)
- Console UI HTTPAdapter v1.0 端到端 22-endpoint 调用代码已 cross-repo ship
- ContextForge v0.7 ship 后 Console UI 可切到 production HTTPAdapter mode
  (关闭 MockAdapter)

### Verification commands

```bash
cargo test -p contextforge-core   # expect all PASS (94 lib + integration tests)
go test ./...                     # expect 43 packages PASS
bash scripts/console_smoke.sh     # expects CONSOLE_REAL_SMOKE_EXIT=0 20/20
RELEASE_SMOKE_CONSOLE=1 bash scripts/release_smoke.sh   # PHASE_RELEASE_SMOKE_EXIT=0
```

---

## v0.6.0 (2026-05-24)

### 摘要

ContextForge v0.6.0 完成 **Phase 13 memory-rest-surface** 收口：ADR-017 D1
Wave 3 共 5 个 memory REST endpoint 落地，把 Console HTTPAdapter conformance
从 13/22 提升到 18/22（82% coverage）。新增 SQLite 表 + `MemoryService` 5 gRPC
RPC + 4 个 AuditOperation 变体 + Go REST 5 handler。ADR-014 cross-validation
gate **第四次完整激活** 跨 4 phase 验证制度稳定性。

### 主要改进

- **task-13.1 Rust SoT** (PR #84):
  - `core/migrations/0013_memory_items.sql` (10 columns + 3 indexes + status CHECK constraint)
  - `core/src/memory/store.rs` `SqliteMemoryStore` (5 methods + 9 unit tests)
  - `proto/contextforge/console_data_plane/v1/console_data_plane.proto` add-only
    `MemoryItem` + 5 request/response messages + `MemoryService` 5 RPC
  - `core/src/data_plane/memory.rs` `MemoryServer` impl (5 RPC + 5 unit tests)
  - `core/src/memoryops/audit.rs` `AuditOperation` 加 4 variants
    (MemoryPin / MemoryUnpin / MemoryDeprecate / MemorySoftDelete)
  - Pin / Deprecate / SoftDelete 各 emit 一条 audit event
  - `core/src/data_plane/mod.rs` `DataPlaneStores` 加 Option<memory> + Option<audit>;
    新 `with_memory()` + `full()` 构造函数; `register_services` 加 5th MemoryServiceServer
  - `core/src/server.rs` `serve_full` 实例化 SqliteMemoryStore + AuditSink 真接到 daemon
  - 3 integration tests via tonic client + MemoryServiceClient
- **task-13.2 Go REST** (PR #85):
  - `internal/consoleapi/types.go` `MemoryClient` interface + `MemoryListFilter` + `Deps.Memory`
  - `internal/consoleapi/router.go` 5 new routes; deprecate + soft-delete
    confirmMiddleware-gated (ADR-017 D2 OR-semantics)
  - `internal/consoleapi/handlers.go` 5 new handlers (Pin/Deprecate/SoftDelete
    each return 204 No Content); `deps.Memory == nil → 503` graceful degrade
  - `internal/consoleapi/memstore.go` `MemMemoryStore` + `SeedFixtures()` (5 hard-coded)
    for `CONSOLE_API_FALLBACK_INMEM=1` mode
  - `internal/consoleapi/grpcclient/grpcclient.go` `memoryClient` 5 wrappers +
    `protoToMemoryItem` helper; `Client.Memory()` accessor
  - `internal/cli/console_api_serve.go` `buildDeps` wires Memory in both modes;
    `degradedDeps()` adds `degradedMemory{}`
  - 7 new router_test + e2e_grpc Step 9d (real Rust daemon 404/412 invariants)
- **scripts/console_smoke.sh v4** (PR #85):
  - Header v3 → v4; subtitle "Phase 13 memory-rest-surface"
  - 13 → 18 endpoint flow; renumber [1/18]..[18/18]
  - 新 Step 13/18: sqlite3 seed (gracefully skips if sqlite3 unavailable)
  - 新 Step 14-18/18: memory list / get / pin 204 / deprecate 412+204 / soft-delete 412+204
  - REAL mode: `CONSOLE_REAL_SMOKE_EXIT=0` 18/18 PASS
- **test/fixtures/memory-seed/seed.sql** (新增): 5 rows + agent_scope 分布
- **治理 / spec 同步** (PR #86):
  - Phase 13 spec / adapter §Phase 13 / task-13.{1,2} 全 `Status: Done`
  - ADR-017 Status: Proposed (full Accepted 推到 Phase 14 closeout 一次性)
  - ADR-014 D1 mapping 表 / D2 lint 0 violation

### Trade-offs / Conscious limitations

- **is_pinned 列设计**：选 `is_pinned bool` 列 + `status` 三态独立；pin state
  存在 Rust SqliteMemoryStore 但**不在 contractv1.MemoryItem 暴露** (ADR-015 D5
  字段锁定)；Console UI 显示 Pin 按钮但 pinned visual indicator 需通过
  future contractv1 amendment 或 inferred via 单独 Get-by-id 调用
- **importer 写入 memory_items 路径** `[SPEC-DEFER:phase-15.import-to-memory-items]`
  留 v0.6.x；v0.6.0 ship 后 Console UI 看 0 条 memory items（fresh install）→
  Console UI 端 graceful degrade
- **memory hard delete** 不实施（Console PRD 显式只支持 soft-delete）
- **POST /unpin separate endpoint** 不实施（Console v1.0 contract 只有 `/pin`；
  `Pin(id, false)` API 端已支持 unpin 语义；如 Console 需要 separate route →
  cross-repo amendment `[SPEC-DEFER:console-memory-unpin]`)

### Migration notes (v0.5.0 → v0.6.0)

- **daemon 重启后 memory_items 表自动创建**（schema migration 0013_memory_items.sql
  在 SqliteMemoryStore.open 内 execute_batch IF NOT EXISTS）；v0.5 用户重启
  daemon 后 `<data_dir>/memory.db` 自动 ready
- **新 5 endpoint**（Memory CRUD + Pin/Deprecate/SoftDelete）— 无 v0.5 baseline;
  client 按 OpenAPI/contractv1 v1 spec 调用
- **destructive endpoints** (deprecate + soft-delete) 需要 X-Confirm: yes header
  或 ?confirm=true query；Console BFF 自动注入；ops curl 用户须显式加
- contractv1.go 字段集合不变 (ADR-015 D5)
- 新 proto RPC + message add-only (ADR-013 D2)

### Tests (Phase 13 全程)

- **Rust**: 84 lib tests (含 14 new memory: 9 store + 5 server) + 3 memory_integration
  + 既有 phase 1-12 测试不退化 = 17 test groups all PASS
- **Go**: 43 packages PASS (含 7 new memory router_test + e2e_grpc Step 9d
  real Rust daemon + grpcclient_test 不退化)
- **conformance**: v0.4/v0.5 既有 endpoints 不退化
- **smoke**: `bash scripts/console_smoke.sh` REAL mode 18/18 PASS

### Verification commands

```bash
cargo test -p contextforge-core   # expect all PASS (17 test groups)
go test ./...                     # expect 43 packages PASS
bash scripts/console_smoke.sh     # expects CONSOLE_REAL_SMOKE_EXIT=0
RELEASE_SMOKE_CONSOLE=1 bash scripts/release_smoke.sh   # PHASE_RELEASE_SMOKE_EXIT=0
```

---

## v0.5.0 (2026-05-24)

### 摘要

ContextForge v0.5.0 完成 **Phase 12 console-contract-completion** 收口：把
ADR-017 D1 Wave 1（quick win 4 个 endpoint）+ Wave 2（mid scope 2 个 endpoint）
共 5 个新 endpoint + 1 个 behavior 切换（cancel 200→204）一次性 ship，把 Console
HTTPAdapter conformance 从 9/22 提升到 13/22（route inventory 9→14 含 PATCH
config）。ADR-014 cross-validation gate **第三次完整激活** 验证制度稳定性。

### 主要改进

- **task-12.1 Wave 1 quick win** (PR #78):
  - `PATCH /v1/workspaces/{id}/config` 走 gRPC `WorkspaceService.UpdateConfig`
    (proto add-only `UpdateWorkspaceConfigRequest`)；body `{allowlist, denylist}`
    覆盖式更新；SqliteWorkspaceStore.update_config 真持久化 + updated_at_unix 推进
  - `GET /v1/index-jobs?status=active` 走 gRPC `JobService.List` + status_filter
    (proto add-only `ListJobsRequest{status_filter, workspace_id}` + `ListJobsResponse`)；
    Rust 端 `list_active()` 包装 + Go 端 missing-filter → 400
  - `POST /v1/index-jobs/{id}/cancel` 返 **204 No Content** (ADR-017 D3)
  - `confirmMiddleware` 服务端 X-Confirm 兜底 (ADR-017 D2): 破坏性 endpoint
    必须 `X-Confirm: yes` header **或** `?confirm=true` query (OR-semantics);
    缺失 → 412 PRECONDITION_FAILED + ErrorBody `{code:"PRECONDITION_FAILED",...}`
- **task-12.2 source-chunk-by-id** (PR #79):
  - `GET /v1/source-chunks/{id}` 走 gRPC `SearchService.GetSourceChunk` (proto
    add-only `GetSourceChunkRequest{chunk_id, workspace_id(optional)}`)
  - Rust impl 复用既存 `Retriever::get_chunk(chunk_id)` (task-6.2 ship 的 SQL
    fast-path)；workspace_id 缺失时枚举 SqliteWorkspaceStore.list() 真试每个
    workspace 寻 chunk (chunk_id 全局唯一 SqliteChunkStore 假设
    `[SPEC-DEFER:phase-15.multi-workspace-strict]`)
  - chunk_offset_start/end = 0 占位 `[SPEC-DEFER:chunk-byte-offsets]` (current
    schema 不存 byte offsets; Console UI 用 line_start/end)
- **task-12.3 search-trace-by-query-id** (PR #80):
  - `GET /v1/search/{query_id}/trace` 走 gRPC `SearchService.GetSearchTrace`
    (proto add-only `GetSearchTraceRequest{query_id}`)
  - 自研 `TraceStore { HashMap, VecDeque, cap=1000 }` ~30 行 LRU/FIFO eviction
    (避免 `lru` crate R7 风险)；`std::sync::Mutex` 包裹 read-heavy 场景足够
  - `SearchService.Query` 内统一生成 `qry-{nanos}` 唯一 query_id 字段
    (task-11.4 既存返 empty query_id 字段被替换)；每次 Query 自动 put trace
    到 trace_store
- **scripts/console_smoke.sh v3** (PR #80):
  - Header bump v2 → v3；subtitle "Phase 12 console-contract-completion"
  - 9 → 13 endpoint flow；renumber [1/13]..[13/13]
  - 新 Step 9/13: task-12.1 PATCH workspace/config (412→200×2)
  - 新 Step 10/13: task-12.1 GET active jobs + missing-status 400
  - 新 Step 11/13: task-12.2 GET source-chunks/{id} (uses chunk_id from search)
  - 新 Step 12/13: task-12.3 GET search/{query_id}/trace + unknown 404
  - REAL mode 真接 daemon: `CONSOLE_REAL_SMOKE_EXIT=0` 13/13 PASS
- **治理 / spec 同步** (PR #81):
  - Phase 12 spec / adapter §Phase 12 / task-12.{1,2,3} 全 `Status: Done`
  - ADR-017 Status: Proposed (full Accepted 推到 Phase 14 closeout 一次性)
  - ADR-014 D1 mapping 表 / D2 lint 0 violation / D3 verified-by 显式

### Trade-offs / Conscious limitations

- **task-12.2 §10**: chunk_offset_start/end = 0 占位
  `[SPEC-DEFER:chunk-byte-offsets]` — current SqliteChunkStore schema 不存
  byte offsets; Console UI 用 line_start/end 显示足够；future schema migration
  填充字节偏移留 v0.5.x
- **task-12.2 §10**: workspace_id 全局唯一假设
  `[SPEC-DEFER:phase-15.multi-workspace-strict]` — multi-workspace strict
  isolation 留 v1.x
- **task-12.3 §10**: trace_store 重启即丢 `[SPEC-DEFER:task-future.search-trace-sqlite-persistence]`
  — SQLite 持久化跨 daemon 重启留 v0.5.x；Console UI 端 graceful degrade 承接
- **task-12.3 §10**: trace_store cap=1000 硬编码 — env var 参数化留 v0.5.x

### Migration notes (v0.4.0 → v0.5.0)

- **`POST /v1/index-jobs/{id}/cancel` 改 204 No Content** — Console HTTPAdapter
  v1.0 已 200/204 双 check (cross-repo 验证)，应不出现 break；如发现 strict
  200 only 的旧 client → rollback path 是把 handlers.go handleCancelJob 回退
  到 `StatusOK`
- **PATCH /v1/workspaces/{id}/config + 新破坏性 endpoint** 现在强制
  X-Confirm/?confirm=true — Console BFF 自动注入；ops curl 用户须显式加
- **新 4 endpoint (PATCH config + active filter + source-chunks + trace)**
  无 v0.4 baseline; client 端按 OpenAPI/contractv1 v1 spec 调用
- contractv1.go 字段集合不变 (ADR-015 D5 字段镜像约束沿用)
- 新 RPC 全 proto add-only (ADR-013 D2)，既有 RPC 字段编号不动

### Tests (Phase 12 全程)

- **Rust**: 70 lib tests (含 4 new task-12.1 workspace UpdateConfig/job List + 3
  new task-12.2 GetSourceChunk + 4 new task-12.3 GetSearchTrace+TraceStore +
  既有 phase 1-11 测试不退化)
- **Go**: 43 packages PASS (含 task-12.1 7 new router_test + 4 new grpcclient_test
  + task-12.2 2 new + task-12.3 1 new + degraded fallback impls + e2e_grpc with
  real Rust daemon Step 8a/8b/9/9b/9c PASS)
- **conformance**: `test/conformance/console_contractv1_test.go` v0.4 9 endpoint
  不退化
- **smoke**: `bash scripts/console_smoke.sh` REAL mode 13/13 endpoint PASS
  with `CONSOLE_REAL_SMOKE_EXIT=0` final marker

### Verification commands

```bash
# Rust workspace
cargo test -p contextforge-core --lib   # expect 70/70 PASS

# Go full
go test ./...   # expect 43 packages PASS

# Phase 12 console real smoke v3 (default REAL mode)
bash scripts/console_smoke.sh   # expects CONSOLE_REAL_SMOKE_EXIT=0

# Release smoke (§5 enables console smoke via env)
RELEASE_SMOKE_CONSOLE=1 bash scripts/release_smoke.sh   # PHASE_RELEASE_SMOKE_EXIT=0

# ADR-014 D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master   # 0 violation
```

---

## v0.4.0 (2026-05-25)

### 摘要

ContextForge v0.4.0 完成 **Phase 11 console-real-data-plane** 收口：把 Phase 10
task-10.4 §10 显式记录的两个 Trade-off (`[SPEC-DEFER:task-future.cross-process-
sqlite-sharing]` 与 JobRunner 不真索引) 一次性 resolve。通过新 ADR-016
**cross-process-rust-go-via-grpc-bridge** 实施 4 个新 Rust gRPC service
(Workspace / Job / Search / Events)，Go console-api-serve 重构为 **thin REST→gRPC
translator**；console UI 期望的 Workspace 持久化跨 daemon 重启 + IndexJob 真触发
Rust 索引 + Search 真返回 indexed chunks + Events 真接 JobRunner progress 全部
端到端落地。ADR-014 cross-validation gate **第二次完整激活** 验制度稳定性。

### 主要改进

- **ADR-016 cross-process Rust ↔ Go gRPC bridge** (Proposed → Accepted): 6 个 D
  条款落地。D1 Rust 持 SoT (Go 不写 SQLite); D2 4 gRPC service in
  `proto/contextforge/console_data_plane/v1/console_data_plane.proto` (snake_case
  1:1 镜像 Go contractv1 JSON tag); D3 Go console-api-serve thin proxy
  (`internal/consoleapi/grpcclient/`); D4 in-memory MemStore 降级为 env-gated
  fallback (`CONSOLE_API_FALLBACK_INMEM=1`); D5 schema 单 owner = Rust; D6 沿用
  ADR-014 cross-validation gate.
- **Rust data plane gRPC services** (`core/src/data_plane/`): 4 tonic service
  trait impls (`WorkspaceServer` / `JobServer` / `SearchServer` / `EventsServer`)
  + `register_services` helper + `serve_full(addr, svc, data_dir)` 把 Phase 9
  ContextService + Phase 11 4 service 注册到同一 tonic Server.
- **Real JobRunner wiring** (task-11.3): `IndexSessionBackend` impl
  `IndexerBackend` 包 `IndexSession::index_path_cancellable` (add-only API
  extension; cancel_token at file boundaries); `JobService.Enqueue` 真
  `tokio::spawn(JobRunner.run_one)`; `orphan_reaper` 在 `serve_full` 启动早期
  清理上一 boot 留下的 running 行 (mark failed + error_message="job lost: daemon
  restart"); JobRunner.run_one 改 per-file cancel-check (heartbeat 仍 throttled
  100files/5s) 让小 fixture 也能在 5s 内观察 cancel.
- **Real SearchService + EventBus** (task-11.4): `SearchService.Query` 真接
  `core/src/retriever/Retriever::search` (Tantivy + SQLite chunks);
  `RetrievalTrace.retrieved_chunks` 真填 (chunk_id + score + source_file +
  `chunk_text_preview` ≤200 chars via `utf8_safe_truncate` UTF-8 boundary safe);
  `EventBus` (broadcast::Sender 容量 1000) 接 `EventsService.Subscribe` server
  stream; `JobRunner` progress callback emit `indexing.progress` /
  `indexing.cancelled` / `indexing.error` events.
- **Go grpcclient** (`internal/consoleapi/grpcclient/`): `Client.Workspace/Job/
  Search/Events()` 4 wrapper impl `consoleapi.{Workspace,Job,Search,Events}Client`;
  `mapGrpcErr` maps gRPC status → consoleapi sentinel (NotFound → ErrNotFound /
  FailedPrecondition → ErrJobTerminal / Unavailable → ErrDataPlaneUnavailable).
- **console-api-serve 新 flags**: `--grpc-addr 127.0.0.1:50551` (default; alias
  to Rust DEFAULT_LISTEN) + `--fallback-inmem` (alias env
  `CONSOLE_API_FALLBACK_INMEM=1`). `BackendKind`-aware `/v1/health`: grpc → 200
  healthy; inmem-fallback → 200 degraded + ErrorReason; degraded → 503 + missing=
  ["data_plane"].
- **Long-poll wait/limit** (`/v1/observability/events`): `?wait=<duration>`
  (default 30s, clamped [1s, 60s]) + `?limit=<int>` (default 100, clamped [1, 500])
  query params; grpcclient.eventsClient.Recent uses ctx 30s timeout to drive
  long-poll behaviour at the gRPC layer.
- **scripts/console_smoke.sh v2** (REAL mode default): spawns both contextforge-
  core daemon and console-api-serve, drives the 9 endpoint flow + real index
  job against `test/fixtures/index-job-real/` (5 markdown files). Final marker:
  `CONSOLE_REAL_SMOKE_EXIT=0`. v0.3 inmem mode retained as `LOCAL_ONLY=1`.
- **release_smoke.sh §5 updated** for REAL mode; final
  `phase11_console_real=ok` marker.
- **ADR-014 D1-D5 second activation pass**: D1 mapping (in closeout PR body);
  D2 lint `bash scripts/spec_drift_lint.sh --touched <base>` 0 violation (with
  proper [SPEC-OWNER]/[SPEC-DEFER] tags throughout phase-11 + 4 task spec);
  D3 each phase §6 AC verified by explicit owner; D4 main-agent self-merge
  via /goal autonomy; D5 historical Phase 1-10 unchanged.
- **治理 / spec 同步**: ADR-016 Proposed → Accepted; Phase 11 / Task 11.1-11.4
  全 Done; PRD §Implementation Phases Phase 11 + §Open Questions O14 partially
  resolved by ADR-016 (business plane wiring; endpoint expansion [SPEC-DEFER:
  console-endpoint-expansion]); adapter §Phase / §Tasks / §ADRs / §BDD synced.

### Trade-offs / Conscious limitations

- **task-11.2 §10 T2** `--grpc-addr` default `127.0.0.1:50551` (与 Rust
  `DEFAULT_LISTEN` 对齐); playbook 文档曾写 `:48180` 是 ADR-013 概念预留, 实施
  按 Rust 既有 default 落地 (无 spec drift — gRPC 字段集合才是契约, 端口可配).
- **task-11.3 §10 T1** cancel co-operative only (file-boundary granularity);
  hard kill cancel [SPEC-DEFER:task-future.hard-cancel].
- **task-11.4 §10 T1** EventBus volatile broadcast (daemon 重启即丢历史
  events); persistent event ring buffer [SPEC-DEFER:task-future.event-persistence].
- **task-11.2 §10 T1** v0.3 in-memory MemStore retained as env-gated fallback
  (not deleted) for conformance test backward compat + degraded mode demo.
- Multi-instance daemon leader election [SPEC-DEFER:task-future.multi-daemon-leader-election].

### Migration notes (v0.3.0 → v0.4.0)

- `console-api-serve` 默认 backend 从 in-memory MemStore 切到 gRPC. v0.3 用户
  若需 inmem 行为, 设 `CONSOLE_API_FALLBACK_INMEM=1` (CLI flag `--fallback-inmem`).
- v0.3 console_smoke.sh 默认 local mode → v0.4 默认 REAL mode (需 cargo build
  Rust binary). 兼容 v0.3 行为: `LOCAL_ONLY=1 bash scripts/console_smoke.sh`.
- Console contract v1 字段集合不变 (ADR-015 D5 字段镜像约束沿用); Console UI
  端无任何改动 — v0.4 仅 ContextForge 单仓内业务面真接通.
- 新 deploy 形态: `contextforge-core <listen> <data_dir> &` 后 `contextforge
  console-api-serve --addr ... --grpc-addr ...`. 双进程 deploy 可用 systemd /
  docker compose / 脚本管理.

### Tests (Phase 11 全程)

- Rust: 60 lib + 5 indexjob_real_runner + 4 search_real_retriever + 5
  data_plane_integration + 既有 phase 1-10 测试不退化.
- Go: 9 grpcclient + 6 cli + 1 e2e gRPC backed E2E (TestRESTEndpoints_E2E_
  GrpcBacked spawns Rust daemon + 9 endpoint flow + workspace 持久化跨 daemon
  restart) + 既有 consoleapi v0.3 + conformance test 不退化.

### Verification commands

```bash
# Rust full workspace
cargo test --workspace

# Go full
go test ./...

# Phase 11 console real smoke (default REAL mode)
bash scripts/console_smoke.sh   # expects CONSOLE_REAL_SMOKE_EXIT=0

# Release smoke (§5 enables console smoke via env)
RELEASE_SMOKE_CONSOLE=1 bash scripts/release_smoke.sh   # PHASE_RELEASE_SMOKE_EXIT=0

# ADR-014 D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master   # 0 violation
```

---

## v0.3.0 (2026-05-24)

### 摘要

ContextForge v0.3.0 完成 **Phase 10 console-contract-v1** 收口：实现 ContextForge ↔
**ContextForge-Console** v1.0 (已 ship) **Contract v1 兼容层** —— 17 个 Go 类型
1:1 镜像 Console `contractv1.go` + Rust workspace/jobs 资源模型 + 9 个对齐 Console
HTTPAdapter 期望的 REST 端点 + cross-repo conformance test + docker compose 集成
smoke。同时 ADR-014 cross-validation gate (D1 mapping / D2 lint / D3 verified-by /
D4 自治补丁 / D5 历史不溯改) 首次完整激活。

### 主要改进

- **internal/contractv1/ Go 类型镜像**：1:1 复刻 Console
  `console-api/internal/coreadapter/contractv1/contractv1.go` 17 个类型 +
  `ContractVersion = "v1"` 常量 + `FieldAvailability` helper；env
  `CONSOLE_REPO=$path` 设时 reflect 反射跑 Console parity 校验。
- **Rust Workspace + IndexJob 资源**：`core/src/workspace/` (CRUD + 1:1
  collection 映射) + `core/src/jobs/` (异步 lifecycle queued/running/
  succeeded/failed/cancelled + heartbeat + co-operative cancel) +
  SQLite migration `0010_workspaces.sql` + `0011_index_jobs.sql`。
- **9 Console Contract v1 REST endpoint** (新增 `internal/consoleapi/`)：
  `GET /v1/health` + `POST/GET/GET /v1/workspaces*` +
  `POST/GET/POST /v1/index-jobs*[/cancel]` + `POST /v1/search` (nested
  `{result, trace}`) + `GET /v1/observability/events` (long-poll, 非 SSE)；
  路径 / shape / 错误码 严格对齐 Console HTTPAdapter；bearer auth +
  OpenAPI 3.0 yaml (`docs/consoleapi/openapi.yaml`)。
- **新 CLI 子命令** `contextforge console-api-serve --addr ...` 启动
  consoleapi router (in-memory MemStore v0.3；cross-process SQLite 共享留
  v0.4 task-future)。
- **Cross-repo conformance test** (`test/conformance/`)：env-based skip
  机制 + Console-style 9 endpoint flow + FieldAvailability.Complete() +
  Console sentinel error mapping (404→ErrNotFound / 409→ErrConflict)。
- **Docker compose stack**：`deploy/console-stack.yml` 含 5 service
  (postgres + redis + contextforge + console-api + console-web)；profile
  `console` gates the optional Console UI services。
- **多阶段 `Dockerfile`**：rust:1.82 + golang:1.22 → debian:bookworm-slim，
  CMD `contextforge console-api-serve --addr 0.0.0.0:48181`。
- **新 smoke**：`scripts/console_smoke.sh` 默认本地 mode (build + spawn
  + 9 endpoint curl); env DOCKER_SMOKE=1 触发 docker compose 模式。
- **release_smoke.sh 第 5 段**：env `RELEASE_SMOKE_CONSOLE=1` 启用 (默认 SKIP
  避 CI 强依赖 docker)。
- **ADR-014 cross-validation gate 全程激活**：D2 lint `scripts/spec_drift_lint.sh
  --touched origin/master` 0 violation；D3 每条 phase §6 AC + task §6 AC 含
  `verified by ...` 显式 owner；D1 closeout PR body mapping 表。
- **治理 / spec 同步**：ADR-015 Proposed → Accepted；Phase 10 / Task
  10.1-10.6 全 Done；PRD §Implementation Phases Phase 10 + §Open Questions
  O12 (Resolved by ADR-014) + O13 (新增 Console 集成)；adapter §Phase /
  Task / ADR / BDD 索引同步。

### v0.3 trade-offs (§Implementation Notes)

- **Cross-process SQLite 共享 Rust ↔ Go (task-10.4 §10 #1)**：v0.3 Go 端 REST
  用 in-memory MemStore；Rust 端 workspace/jobs 用 SQLite。两者各自独立，
  Console UI POST 创建的 workspace 不进 Rust JobRunner。**Why**：保守
  优先级 backward compat > spec literal > minimal change；避新增 sqlite Go
  driver (mattn/go-sqlite3 CGO 或 modernc/sqlite 纯 Go) — playbook v0.3 不
  预期新 dep。**v0.4 follow-up**：[SPEC-DEFER:task-future.cross-process-sqlite-sharing]。
- **时间字段 Unix epoch i64 (workspace/jobs)**：避新增 chrono dep；Go REST
  序列化时 `time.Unix(sec, 0).UTC()` 转 RFC3339 喂 Console wire。
- **Console UI integration smoke 在 docker compose 默认 SKIP**：Console v1.0
  docker image 公网未发布；console_smoke.sh 默认 local mode (ContextForge
  daemon only)；DOCKER_SMOKE=1 + CONSOLE_API_IMAGE / CONSOLE_WEB_IMAGE 三
  env 同时设才跑 full Console UI 集成。

### 限制（继承 v0.1 + v0.2 + Phase 10 新增）

- v0.3 Console 集成是 spec/REST 契约层 conformance；Console UI 真返回
  workspace 列表（非 Mock）已通过 console_smoke.sh 在 ContextForge daemon
  端验证。**Console docker image 公网拉取 + UI 真渲染**留 v0.4 (依赖 Console
  仓库发布 image)。
- v0.3 in-memory MemStore 不持久化 — daemon 重启后数据丢失。Cross-process
  SQLite 共享 / 持久化 IndexJob 留 v0.4。
- 其它 10+ Console endpoint (`/v1/memory*` / `/v1/eval-runs*` /
  `/v1/source-chunks/:id` / `/v1/search/:query_id/trace` /
  `/v1/workspaces/:id/config` PATCH) — Console Mock Adapter 覆盖到 v0.4。

### Migration notes (from v0.2.0)

- `internal/cli` 新增 `console-api-serve` 子命令 — 现有子命令行为不变。
- `internal/daemon/rest.go` v0.2 既有 5 endpoint (`/v1/search`, `/v1/chunks/{id}`,
  `/v1/collections`, `/v1/import`, `/v1/eval/run`) 不变；Console Contract v1
  9 endpoint 在独立 `internal/consoleapi/` 包内，通过 `console-api-serve` 子
  命令暴露 (不与 `serve` 子命令的 daemon REST 冲突)。
- `scripts/release_smoke.sh` 增第 5 段 (env RELEASE_SMOKE_CONSOLE=1 启用)；
  `PHASE_RELEASE_SMOKE_EXIT` 退出码兼容 v0.2。

---

## v0.2.0 (2026-05-24)

### 摘要

ContextForge v0.2.0 完成 Phase 9 cli-pipeline 收口：补齐 v0.1 ship 后实测的
CLI 数据通路 spec drift —— `contextforge index` / `contextforge import` 在
v0.1 是 stub，v0.2 通过 ADR-013 add-only 扩 `rpc Index` server-stream 真接通
Go↔Rust gRPC + 真扫描 + 真写 SQLite/Tantivy。README Quick Start 现可复制粘贴
跑通。

### 主要改进

- **CLI 数据通路打通**：`proto/contextforge/v1/service.proto` 新增 `rpc Index(IndexRequest) returns (stream IndexProgress)`；Rust `CoreService::index`
  wire `IndexSession::index_path_with_progress` 按文件粒度上报进度；Go
  `Daemon.Index` + `internal/cli/index.go` 真实 stream consume + human/JSONL render。
- **`contextforge import` 三子命令真实**：hermes / openclaw / agent-rules 现产
  YAML-frontmatter Markdown 到 `<data-dir>/imports/<source>/`；`contextforge index --source <output_dir>` 把它灌入。
- **README Quick Start 可复制粘贴**：新增 `examples/quickstart/` fixture +
  `scripts/quickstart_smoke.sh` 一键 7 步端到端；README 重写 manual steps + 注释 flag 顺序陷阱。
- **Release smoke 真端到端**：删除 `internal/release/release_test.go` 三个
  fake-evidence 测试（`TestTask83_AC2/AC4/AC5`），重写 `TestTask83_AC1` 用真
  `go build` + `cargo build`，新增 `TestPhase9ReleaseSmoke_EndToEnd` 7-step
  CLI binary 真跑；`scripts/release_smoke.sh` 加 phase 9 段 + 重命名
  `PHASE_RELEASE_SMOKE_EXIT`（去 v0.1-only PHASE8 前缀）。
- **治理 / spec 同步**：ADR-013 Proposed → Accepted；Phase 9 / Task 9.1-9.6 全
  Done；PRD §Implementation Phases Phase 9 + §Open Questions O12 同步；
  adapter §Phase 状态索引 / Task 索引 / ADR 索引 / BDD 索引同步。

### 验证证据

最终 `master` 上执行：

```bash
bash -lc 'source docs/s2v/scripts/lib/preflight.sh; source docs/s2v/scripts/lib/verify.sh; s2v_baseline_green "cmd/contextforge internal core/src core/tests"'
```

结果：`FINAL_HEAD_BASELINE_EXIT=0`。

```bash
bash scripts/release_smoke.sh
```

结果：`PHASE_RELEASE_SMOKE_EXIT=0`（4 段：go release harness / task-8 reliability/eval / Rust gRPC search smoke / phase 9 CLI e2e）。

```bash
bash scripts/quickstart_smoke.sh
```

结果：`QUICKSTART_SMOKE_EXIT=0`（7 步：build / init / import hermes / index records / index source / search / eval）。

完整证据见 [`docs/releases/v0.2.0-evidence.md`](docs/releases/v0.2.0-evidence.md)；产物清单见 [`docs/releases/v0.2.0-artifacts.md`](docs/releases/v0.2.0-artifacts.md)。

### 发布边界

- 继承 v0.1 限制：Linux x86_64 / WSL2 官方目标；macOS 应能跑（bash + cargo + go）；Windows 走 Git Bash / WSL；macOS / Windows 官方 tarball 仍延后。
- `LICENSE` 继续 all-rights-reserved（占位于明确 OSI 许可证前）。
- 真实 GitHub Release 上传、checksum / signing、CI release job 仍需外部发布流水线执行。

### v0.1.0 → v0.2.0 迁移

无 schema 变更（schema_version 仍 `0.1`，proto add-only `rpc Index` 不破坏现有 wire 兼容）。脚本端：`PHASE8_RELEASE_SMOKE_EXIT` 重命名为 `PHASE_RELEASE_SMOKE_EXIT` — 任何依赖此标记的外部 CI 步骤需相应更新。

---

## v0.1.0 (2026-05-23)

### 摘要

ContextForge v0.1.0 完成本地优先的双二进制基础闭环：Go 控制面 `contextforge` + Rust 数据面 `contextforge-core`，覆盖初始化、索引核心、检索解释、REST/MCP/export、recall eval、可靠性 guard 与 release smoke gate。

### 主要能力

- S2V 治理：ADR-012 放宽主 agent 自治决策，同时保留 R3 分支校验、R6 PR-only、worktree 隔离和合入 gate。
- Eval：`contextforge eval run` 具备 30 条内置 golden questions、Top-5/Top-10 strong hit rate、miss cases 与 latency p95 输出。
- Reliability：长任务 resume manifest、资源预算 gate、secret/export/audit safety regression guard。
- Release：新增 `internal/release` tarball contract、七步 smoke evidence、10 万 chunk P95 benchmark gate，以及 `scripts/release_smoke.sh` Phase 8 smoke 入口。
- Distribution docs：新增 `README.md`、`LICENSE`、`contextforge.example.toml` 和 ADR-007 产物清单。

### 验证

最终 `master` 上通过：

```bash
bash -lc 'source docs/s2v/scripts/lib/preflight.sh; source docs/s2v/scripts/lib/verify.sh; s2v_baseline_green "cmd/contextforge internal core/src core/tests"'
```

结果：`FINAL_HEAD_BASELINE_EXIT=0`。

最终 `master` 上通过：

```bash
bash scripts/release_smoke.sh
```

结果：`PHASE8_RELEASE_SMOKE_EXIT=0`（v0.1 版本；v0.2 已重命名为 PHASE_RELEASE_SMOKE_EXIT）。

完整证据见 `docs/releases/v0.1-evidence.md`。

### 发布边界

- 本 tag 提供 release contract gate 与产物清单；真实 GitHub Release 上传、checksum/signing 与 CI release job 仍需在发布流水线中执行。
- v0.1 官方目标平台为 Linux x86_64 / WSL2；macOS / Windows 官方 tarball 延后。
- `LICENSE` 当前为 all-rights-reserved，占位于明确开源许可证之前。
