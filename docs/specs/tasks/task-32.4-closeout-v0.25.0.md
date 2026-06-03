# Task `32.4`: `closeout-v0.25.0 — smoke v22 step [41/41] + v0.25.0 release docs + ADR-037 据 D1-D5 真实 ratify（D2 sqlite-vec 矩阵 cell / D4 real chunk source_type·agent_scope filter 受阻维度 honest-defer 部分 ratify）+ ADR-034 add-only Amendment（sqlite-vec arm 补全工厂后端覆盖）+ ADR-023 守线引用 + roadmap §3.14 + §4 add-only + phase-32 §6 闭合`

**Status**: Draft

**Priority**: P2
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 32 (vector-backend-config-plumbing-and-completeness)
**Dependencies**: task-32.1（vector backend config plumbing — server.rs hybrid + semantic 两热路径经 env/config 选 backend，未设/"" → BruteForce byte-equivalent）/ task-32.2（factory sqlite-vec arm — feat on→SqliteVecBackend / feat off→honest Err naming vector-sqlite + in-process 选择矩阵 wiring；矩阵 recall/latency cell 须 MSVC feature build honest-defer）/ task-32.3（console provenance add-only vector_score=16 + retrieval-filter 契约诚实化）全 Done / ADR-037（vector-backend-config-plumbing-and-completeness，本 task ratify）/ ADR-034（production-vector-live-recall，本 task add-only Amendment：sqlite-vec arm 补全工厂后端覆盖）/ ADR-023（vector-backend-default，0-dep baseline 守线引用）/ ADR-004（默认行为 / proto 既有字段 / 既有契约不变）/ ADR-012（tag/release outward-facing 须用户显式授权）/ ADR-013（禁伪造凭据红线）/ ADR-014 D1-D5（第二十三次激活）

## 1. Background

Phase 32 三个实现 task 全 Done：32.1（vector backend config plumbing——`server.rs` 之前两热路径仅注入 DEFAULT `select_vector_backend("", 0)`，hybrid 路径（约 `server.rs:340`）+ semantic 路径（约 `server.rs:367+`），无 config 经 env 经 `CoreService.data_dir` 模式 plumb；本 task 把 backend 名经 env/config plumb 进两热路径，未设/"" 回落 BruteForce byte-equivalent，默认行为不变）/ 32.2（factory sqlite-vec arm——`factory.rs::select_vector_backend` 之前无 sqlite-vec 臂，仅 ""/"brute"/"qdrant"/"lancedb"/unknown→Err；本 task 加 `"sqlite-vec"` 臂，feature `vector-sqlite` on→`SqliteVecBackend`（`sqlite_vec.rs`，name()="sqlite-vec"）/ off→honest Err 命名 `vector-sqlite` feature；in-process 选择矩阵 wiring 🟢，矩阵 recall/latency cell 须本机 MSVC feature build → honest-defer `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`，不伪造数值 ADR-013）/ 32.3（console provenance + filter 契约诚实——console_data_plane `SearchResultItem` add-only `vector_score=16`（v1 search proto 已有 `vector_score=13` + `retrieval_method=8`，console proto 仅有 `retrieval_method=13` 缺 vector_score，字段到 `citation=15` 故 add-only field 16）携带 provenance parity；retrieval-filter 契约诚实化——`mod.rs:325` 误导性 WARN（"source_type/agent_scope filter not yet implemented"）→ 准确 no-op 契约 + `[SPEC-DEFER:phase-future.chunk-source-type-filter]` + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`，默认空 filter 结果完全一致）。本 task 收口 v0.25.0：smoke v22 + release docs + ADR-037 据真实结果 ratify + ADR-034 add-only Amendment（sqlite-vec arm 补全工厂后端覆盖）+ ADR-023 守线引用 + roadmap §3.14 推进记录 + §4 add-only backlog + phase §6 闭合 + adapter + feature。

## 2. Goal

据 32.1/32.2/32.3 **真实 CI / 实测产物**收口 v0.25.0：ADR-037 `Proposed → Accepted`（逐 D 项如实——D1 backend config plumbing 两热路径达成 + default 保形、D2 sqlite-vec factory arm feature 双半 gating 🟢 达成 / in-process 矩阵 recall·latency cell 须 MSVC feature build 🟡 honest-defer、D3 console provenance add-only + retrieval-filter 契约诚实化达成 + real chunk filter feature honest-defer 新 backlog、D4 honest-defer 边界重申、D5 默认 0-vector-dep baseline + 既有契约不变）；ADR-034 add-only Amendment（sqlite-vec arm 补全工厂后端覆盖，承 Phase 29 D4 选择矩阵——sqlite-vec 本 pass 未跑 in-process 测量的格——不溯改正文 ADR-014 D5）；ADR-023 0-dep baseline 守线引用；roadmap §3.14（Phase 32 推进记录）+ §4 add-only（新 backlog：chunk-source-type-filter / chunk-agent-scope-filter / sqlite-vec-inprocess-matrix）；phase-32 §6 AC 置 `[x]` + Status Done；smoke v22 step `[41/41]`（doc/status 断言 vector backend config-selectable baseline + factory sqlite-vec arm 可达则断言，default build baseline intact）；release docs（evidence/artifacts/README/RELEASE_NOTES，tag/run/digest 用 `<backfill>` 待回填）；adapter（Phase 32 Done + Tasks 4 + ADR-037 Accepted + feature 行）。**真实 v0.25.0 tag/release 须用户显式授权**（本轮用户已授权 v0.25.0；不自行越界 tag，ADR-012）。

## 3. Scope

### In Scope（计划交付）

- `scripts/console_smoke.sh`——banner v21→v22 + v22 changelog 块 + step `[41/41]`（doc/status 断言 vector backend config-selectable baseline + factory sqlite-vec arm 可达则断言 + default build init baseline 不变；既有 step 不退化 + denominator 不溯改 ADR-014 D5）。当前 live 脚本为 `[40/40]`（v21 Phase 31）；故 Phase 32 顺接 `[41/41]`。step 为文档/状态步：断言 default-build init baseline + 有运行时面的 config-plumbing 状态（如 `select_vector_backend("", 0)` 默认臂仍 BruteForce byte-equivalent 若可达，否则文档/状态）。
- `internal/cli/smoke_syntax_test.go`——新增 `TestTask324_SmokeV22VectorBackendConfigStep`（断言 `[41/41]` + 标记 + 无回归既有 `[37/37]`..`[40/40]`，denominator 不溯改 ADR-014 D5）。
- 新增 `docs/releases/v0.25.0-{evidence,artifacts}.md`（tag SHA / run id / digest 用 `<backfill>` 待回填）+ `README.md` v0.25 段 + `RELEASE_NOTES.md` v0.25.0 段。
- `docs/decisions/adr-037-vector-backend-config-plumbing-and-completeness.md`——Status Proposed→Accepted（per-D 限定）+ `## Ratification（v0.25.0 / task-32.4）` 节（逐 D 真实依据；sqlite-vec in-process 矩阵 cell / real chunk source_type·agent_scope filter feature 受阻 / 无驱动维度据已达维度 ratify + 如实记录）。
- add-only Amendment（不溯改正文，ADR-014 D5）：`docs/decisions/adr-034-production-vector-live-recall.md`——`## Amendment (Phase 32 / v0.25.0)`（sqlite-vec arm 补全工厂后端覆盖——Phase 29 D4 选择矩阵 sqlite-vec 本 pass 未跑 in-process 测量的格，现 factory 加 `"sqlite-vec"` 臂使其可经工厂选择；in-process 矩阵 recall·latency cell 仍须 MSVC feature build honest-defer，不溯改 D1-D5 正文）。
- `docs/decisions/adr-023-*.md` 守线引用（0-vector-dep baseline + BruteForce 默认 byte-equivalent，本 task 不改其正文，仅 ADR-037 D5 引用守线）。
- `docs/roadmap.md`——§3 新增 §3.14 Phase 32 推进记录 + §4 add-only（新 backlog 条目 chunk-source-type-filter / chunk-agent-scope-filter / sqlite-vec-inprocess-matrix，add-only 不删旧条目正文）。
- `docs/specs/phases/phase-32-vector-backend-config-plumbing-and-completeness.md`——Status Draft→Done + §6 AC `[x]`（honest per-dim：sqlite-vec in-process 矩阵 cell / real chunk filter feature 维度如实标注）。
- `docs/s2v-adapter.md`——§Phase 32 In Progress→Done + Tasks 3→4；§Task +32.4；§ADR 037 Proposed→Accepted；§BDD +phase-32 行。
- `test/features/phase-32-vector-backend-config-plumbing-and-completeness.feature`（已创建）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- 真实 v0.25.0 tag push + release run（cosign 真签 + GHCR 推送）[SPEC-OWNER:user-authorized-release]——outward-facing 不可逆已获本轮用户授权（ADR-012）；post-tag-push backfill 填实 tag SHA / run id / digest，本 task body 不预填真实凭据。
- sqlite-vec in-process 选择矩阵 recall/latency cell（须本机 MSVC `vector-sqlite` feature build）[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]——factory arm wiring 🟢 已达（feature 双半 gating），矩阵真实测量 cell 🟡 须 MSVC feature build，据 ADR-013 不伪造数值，真实跑出后回填。
- real chunk source_type filter feature（须 importer-side source_type tagging + schema migration）[SPEC-DEFER:phase-future.chunk-source-type-filter] / real chunk agent_scope filter feature（agent_scope 系 memory-layer 概念 memory_items table）[SPEC-DEFER:phase-future.chunk-agent-scope-filter]——chunks table（FROZEN §5.3）无 source_type/agent_scope 列，`SearchResult.source_type` 为 hardcoded DEFAULT、`agent_scope` 为 hardcoded `Vec::new()`，真实 chunk filter 系 import-path feature 非确定性 nit；Phase 32 仅令契约诚实（准确 no-op），真实 feature 据 ADR-013 honest-defer。

## 4. Actors

- 主 agent（ADR-012 自治；真实 release 本轮已获用户授权）
- closeout 文档集（smoke / release docs / ADR-037 ratify / ADR-034 add-only Amendment / ADR-023 守线引用 / roadmap §3.14+§4 / phase spec / adapter / feature）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-32-vector-backend-config-plumbing-and-completeness.md §6/§8`（AC + DoD）
- `docs/decisions/adr-037-vector-backend-config-plumbing-and-completeness.md`（§D1-D5 + Consequences Ratification 条款）
- `docs/specs/tasks/task-32.1-vector-backend-config-plumbing.md §10` + `task-32.2-sqlite-vec-factory-arm-and-selection-matrix.md §10` + `task-32.3-console-provenance-and-retrieval-filter-honesty.md §10`（真实测试结果 + 结论）
- `core/src/retriever/vector/factory.rs`（`select_vector_backend(name, dim) -> Result<Arc<dyn VectorStore>, VectorError>`——32.2 sqlite-vec arm 锚点；TEST-29.1.1..3 pattern）+ `core/src/retriever/vector/sqlite_vec.rs`（`SqliteVecBackend`，feature `vector-sqlite`，name()="sqlite-vec"——32.2 选择对象）
- `core/src/server.rs`（hybrid 路径约 `:340` + semantic 路径约 `:367+` 两热路径 `select_vector_backend("", 0)`；`CoreService.data_dir`（`:52`）+ `resolve_data_dir`（`:504-521`，env `CONTEXTFORGE_DATA_DIR` 模式）——32.1 config plumbing 锚点）
- `core/src/retriever/mod.rs:135`（`SearchFilters` source_type/agent_scope）+ `:325`（误导性 WARN——32.3 诚实化锚点）+ `:452`（`source_type` hardcoded DEFAULT）+ `:459`（`agent_scope` hardcoded `Vec::new()`）
- `proto/contextforge/console_data_plane/v1/console_data_plane.proto:185-201`（`SearchResultItem`——32.3 add-only `vector_score=16` 落点）+ `proto/contextforge/v1/search.proto`（`vector_score=13` + `retrieval_method=8` parity 参照）
- `docs/decisions/adr-034-production-vector-live-recall.md §Amendment`（Phase 29/v0.22.0 选择矩阵——本 task add-only Phase 32 Amendment 落点）+ `docs/decisions/adr-023-*.md`（0-dep baseline 守线）
- `docs/releases/v0.24.0-{evidence,artifacts}.md`（模板）

### 5.2 关键设计 — 诚实 per-D ratify + backfill 待回填

- ADR-037 ratify **逐 D 项据真实结果**：D1（backend config plumbing——server.rs 两热路径经 env/config 选 backend + default 未设/"" 保形 byte-equivalent）/ D2（sqlite-vec factory arm——feature on→SqliteVecBackend / off→honest Err naming vector-sqlite + in-process 选择矩阵 wiring 🟢 达成；in-process 矩阵 recall·latency cell 须 MSVC feature build 🟡 honest-defer）/ D3（console provenance add-only vector_score=16 + retrieval-filter 契约诚实化；real chunk source_type+agent_scope filter feature honest-defer 新 backlog）/ D4（honest-defer 边界重申）/ D5（默认 0-vector-dep baseline + 既有契约不变）。各 D 的真实测试 / 实测结果待 32.1-32.3 实施后跑出再回填，不为「全 Accepted」伪造 sqlite-vec in-process 矩阵召回数值或 real chunk filter feature 已实现（ADR-013）。
- ADR-034 add-only Amendment 为 **add-only 注记**（不删/不改 ADR-034 D1-D5 正文 + 既有 Phase 29 Amendment 正文，加 `## Amendment (Phase 32 / v0.25.0)`：sqlite-vec arm 补全工厂后端覆盖，承 Phase 29 D4 选择矩阵 sqlite-vec 本 pass 未跑 in-process 测量的格——现 factory 加 `"sqlite-vec"` 臂使其可经工厂选择；in-process 矩阵 cell 仍 honest-defer），如实记录已达 wiring、矩阵 cell 不伪造（ADR-013）。
- tag SHA / release run id / 镜像 digest 在 release docs 用 `<backfill: ...>` 待回填——真实 v0.25.0 tag/release 是 closeout 合入后的**用户授权步**（本轮已授权），post-tag-push backfill PR 填实（承 v0.8–v0.24 pattern）。
- smoke step `[41/41]` 为文档/状态步：验 default build init baseline 不变（ADR-004）+ 文档化三 task 状态（vector backend config-selectable + factory sqlite-vec arm + console provenance/filter 诚实）；`select_vector_backend("", 0)` 默认臂 byte-equivalent BruteForce 若运行时可达则附加断言，否则退为文档/状态。

### 5.3 不变量

- 0 行为变更 / 0 新依赖（closeout 纯文档 + smoke step；Phase 32 不增 dep，ADR-008；smoke 既有 step + denominator 不溯改 ADR-014 D5）。
- ADR-014 D5：历史 Phase 1-31 spec 不溯改；ADR-034 add-only Amendment 不改 D1-D5 正文 + 不改既有 Phase 29 Amendment 正文；ADR-023 守线仅引用不改正文；roadmap §4 新 backlog 为 add-only 条目不删旧条目正文。
- console proto add-only field（`vector_score=16`）+ factory arm add-only（`"sqlite-vec"` 臂）+ filter no-op（默认空 filter 结果完全一致）不破既有契约（ADR-004）。
- 真实 tag/release 经用户授权后执行（本轮已授权，ADR-012）；release docs tag/run/digest backfill 待回填，不预填伪造凭据。

## 6. Acceptance Criteria

- [ ] AC1（smoke v22 step + release docs + ADR-037 ratify）: smoke banner v21→v22 + step `[41/41]`（doc/status 断言 vector backend config-selectable baseline + factory sqlite-vec arm 可达则断言 + default build baseline intact）+ `TestTask324_SmokeV22VectorBackendConfigStep`（含无回归既有 `[37/37]`..`[40/40]`，denominator 不溯改）；v0.25.0 release docs（`v0.25.0-{evidence,artifacts}.md` `<backfill>` 待回填 + README v0.25 段 + RELEASE_NOTES v0.25.0 段）+ ADR-037 per-D ratify `Proposed→Accepted`（D1/D3/D5 Accepted；D2 in-process 矩阵 cell + D4 real chunk filter feature honest-defer PARTIAL）— verified by TEST-32.4.1
- [ ] AC2（ADR-034 Amendment + roadmap + adapter + phase 闭合）: ADR-034 add-only `## Amendment (Phase 32 / v0.25.0)`（sqlite-vec arm 补全工厂后端覆盖，不溯改 D1-D5 + 既有 Phase 29 Amendment 正文）+ ADR-023 守线引用 + roadmap §3 新增 §3.14 Phase 32 推进记录 + §4 add-only 新 backlog（chunk-source-type-filter / chunk-agent-scope-filter / sqlite-vec-inprocess-matrix）+ phase-32 §6 AC1-5 `[x]` + Status Done + adapter 闭合（Phase 32 Done/Tasks 4/ADR-037 Accepted）+ feature — verified by TEST-32.4.2
- [ ] AC3（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by TEST-32.4.3（LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-32.4.1 | smoke v22 step `[41/41]`（vector backend config-selectable baseline + factory sqlite-vec arm 标记 + 无回归既有 denominator）+ `bash -n` 过 + `go test -run TestTask324` 过 + v0.25.0 release docs + ADR-037 per-D ratify Accepted（D2 in-process 矩阵 cell / D4 real chunk filter feature honest-defer 如实） | `scripts/console_smoke.sh` + `internal/cli/smoke_syntax_test.go` + release/ADR-037 | Planned |
| TEST-32.4.2 | ADR-034 add-only Phase 32 Amendment（sqlite-vec arm 补全工厂后端覆盖，不溯改正文）+ ADR-023 守线引用 + roadmap §3.14 + §4 add-only 新 backlog + phase-32 §6 闭合 + adapter + feature | ADR-034/roadmap/phase/adapter/feature | Planned |
| TEST-32.4.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（LAST） | `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（低）closeout 误报 sqlite-vec in-process 矩阵 cell / real chunk source_type·agent_scope filter feature 为已达成**：诚实风险。
  - **缓解**：ADR-037 ratify + release docs + smoke + phase §6 全逐维如实——factory sqlite-vec arm feature 双半 gating wiring 🟢 达成、in-process 矩阵 recall·latency cell 🟡 须 MSVC feature build 待实测回填、real chunk filter feature（须 importer-side source_type tagging + schema migration）据 32.3 范围外 honest-defer；不伪造（ADR-013）。stop-condition：任何「sqlite-vec 矩阵召回已测」/「real chunk filter 已实现」表述须有真实凭据，否则标受阻维度 / backfill。
- **R2（低）smoke denominator 误溯改**：新 step 须 `[41/41]`，既有 `[37/37]`..`[40/40]` 不动。
  - **缓解**：`TestTask324` 无回归断言守护；ADR-014 D5。
- **R3（低）ADR-034 Amendment 误溯改 D1-D5 / 既有 Phase 29 Amendment 正文**：须 add-only 追加 `## Amendment (Phase 32 / v0.25.0)` 不删既有正文（D5）。
  - **缓解**：仅追加 Phase 32 Amendment 段（sqlite-vec arm 补全工厂后端覆盖 + in-process 矩阵 cell honest-defer），不改 ADR-034 D 正文 + Phase 29 Amendment 正文 + ADR-023 正文（仅守线引用）。

## 9. Verification Plan

```bash
# AC1 — smoke 语法 + syntax test
bash -n scripts/console_smoke.sh
go test ./internal/cli/ -run TestTask324

# AC2 — 文档闭合人工核（ADR-037 Accepted + per-D / ADR-034 add-only Phase 32 Amendment /
#        ADR-023 守线引用 / roadmap §3.14 + §4 新 backlog / phase §6 [x] / adapter Done / feature 存在）
# AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master

# 既有不退化（closeout 文档+smoke 不影响 workspace）
cargo test --workspace && go test ./...
```

> **outward-facing 红线**：真实 v0.25.0 tag push + release run（cosign 真签 + GHCR 推送）是 closeout 合入后的**用户授权步**（本轮已授权，ADR-012）；本 task body 不预填真实凭据，release docs 的 tag/run/digest 用 `<backfill>` 待 post-tag-push backfill 填实。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Draft

**§9 Verification 计划** (will record real evidence at impl)：
- `bash -n scripts/console_smoke.sh` + `go test ./internal/cli/ -run TestTask324`（smoke 语法 + syntax test，`[41/41]` + vector-backend-config + factory sqlite-vec arm + TEST-32.1/32.2/32.3 标记 + 无回归 `[37/37]`..`[40/40]`）——真实跑出后回填。
- `cargo test --workspace` + `go test ./...`（既有不退化）——真实跑出后回填。
- `bash scripts/spec_drift_lint.sh --touched origin/master`（D2 lint，CI spec-lint 权威）——真实跑出后回填。
- ADR-037 ratify 逐 D 据 32.1-32.3 真实测试 / 实测结果——待实测回填；sqlite-vec in-process 矩阵 cell（须 MSVC `vector-sqlite` feature build）/ real chunk source_type·agent_scope filter feature（须 importer-side tagging + schema migration）受阻 / 无驱动维度据已达维度 ratify + 如实记录，不强 ratify（ADR-013）。
- ADR-034 add-only `## Amendment (Phase 32 / v0.25.0)`（sqlite-vec arm 补全工厂后端覆盖；in-process 矩阵 cell honest-defer）——据真实 factory arm wiring 落地后回填，不溯改正文 + Phase 29 Amendment 正文（ADR-014 D5）；ADR-023 守线引用。
- roadmap §3.14 Phase 32 推进记录 + §4 add-only 新 backlog（chunk-source-type-filter / chunk-agent-scope-filter / sqlite-vec-inprocess-matrix）——add-only 落地后回填。
- 真实 v0.25.0 tag/release（cosign 真签 + GHCR 推送）经用户授权（本轮已授权）→ post-tag-push backfill 填实 evidence/artifacts 待回填（tag SHA / run id / digest，承 v0.8–v0.24 pattern，不预填伪造凭据 ADR-013）。

**计划改动文件**：
- `scripts/console_smoke.sh`——banner v21→v22 + v22 changelog 块 + step `[41/41]`（vector backend config-selectable baseline + factory sqlite-vec arm 可达则断言 + default build init baseline 不变）。
- `internal/cli/smoke_syntax_test.go`——`TestTask324_SmokeV22VectorBackendConfigStep`（断言 `[41/41]` + 标记 + 无回归既有 `[37/37]`..`[40/40]`，denominator 不溯改）。
- `docs/releases/v0.25.0-{evidence,artifacts}.md`（新，tag/run/digest `<backfill>` 待回填）+ `README.md` v0.25 段 + `RELEASE_NOTES.md` v0.25.0 段。
- `docs/decisions/adr-037-vector-backend-config-plumbing-and-completeness.md`——Status Proposed→Accepted（per-D 限定）+ `## Ratification（v0.25.0 / task-32.4）` 节。
- add-only Amendment：`docs/decisions/adr-034-production-vector-live-recall.md`——`## Amendment (Phase 32 / v0.25.0)`（sqlite-vec arm 补全工厂后端覆盖，不溯改 D1-D5 + Phase 29 Amendment 正文）+ `docs/decisions/adr-023-*.md` 守线引用（不改正文）。
- `docs/roadmap.md`——§3 新增 §3.14 Phase 32 推进记录 + §4 add-only 新 backlog（chunk-source-type-filter / chunk-agent-scope-filter / sqlite-vec-inprocess-matrix）。
- `docs/specs/phases/phase-32-vector-backend-config-plumbing-and-completeness.md`——Status Draft→Done + §6 AC `[x]`（honest per-dim）。
- `docs/s2v-adapter.md`——Phase 32 Done + Tasks 4 + ADR-037 Accepted + BDD 行。
- `test/features/phase-32-vector-backend-config-plumbing-and-completeness.feature`（已创建）。
