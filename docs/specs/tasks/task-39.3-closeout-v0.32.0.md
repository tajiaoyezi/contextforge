# Task `39.3`: `closeout-v0.32.0 — smoke v28→v29 banner + 新 step [48/48]（staging dir 顺位 offset，端到端断言 ?hybrid=true 抵达 core → retrieval_method="hybrid" / hybrid_score + CONTEXTFORGE_RERANKER_PROVIDER=identity 时 rerank reason marker 在对外 REST 响应可见）+ smoke_syntax_test.go TestTask393 镜像 TestTask383 断言 [48/48] + no-regression（denominators [37/37]..[47/47] 不溯改 ADR-014 D5）+ v0.32.0 release docs（evidence/artifacts + README v0.32 段替换 :350「in a later release」措辞 + RELEASE_NOTES，tag/run/digest <backfill> markers）+ ADR-044 Proposed→Accepted 逐 D ratify + ADR-025 add-only Phase-39 Amendment（标 console-api-hybrid-forward fulfilled）+ ADR-043 add-only Phase-39 Amendment（标 console-api-rerank-forward 重界定 fulfilled + ?rerank per-request superseded）+ roadmap §3.21/§4 add-only + s2v-adapter add-only + defer marker 更新 + phase §6 闭合`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Dependencies**: task-39.1（console-dataplane-hybrid-proto-and-dispatch，proto add-only + 数据面 hybrid dispatch 全 Done）+ task-39.2（console-api-hybrid-forward-and-rerank-visibility，Go 转发 + rerank provenance 可见性全 Done）/ 既有 `scripts/console_smoke.sh`（banner v28 + step [47/47] `:1108`（task-38.3）+ staging dir `cf-v29-cfg` `:1102` / `cf-v30-cfg` `:1130`——本 task banner v28→v29 + 新 step [48/48] 顺位 offset；`:49-50` `[SPEC-DEFER:phase-future.console-api-hybrid-forward]` 注——本 task 据实更新）/ 既有 `internal/cli/smoke_syntax_test.go`（`TestTask383_SmokeV28RemoteRerankerLiveStep` 范本 + `:705-706` 同 defer 注——本 task `TestTask393` 镜像 + defer 注更新）/ 既有 `docs/releases/v0.31.0-{evidence,artifacts}.md`（release docs 模板）+ `README.md`（`:350` console-api `?hybrid=true` / `?rerank=true`「in a later release」措辞——本 task 替换为 fulfilled / 重界定口径）+ `RELEASE_NOTES.md` / ADR-044（console-api-retrieval-signal-forward；本 task ratify Proposed→Accepted）/ ADR-025（hybrid-scoring-fusion；本 task add-only Phase-39 Amendment 标 `console-api-hybrid-forward` fulfilled）/ ADR-043（embedding-remote-reranker-live；本 task add-only Phase-39 Amendment 标 `console-api-rerank-forward` 重界定 fulfilled + `?rerank` per-request superseded）/ ADR-012（tag/release outward-facing 须用户显式授权——v0.32.0 release 须另行授权）/ ADR-013（禁伪造红线——tag/run/digest `<backfill>` 待回填、rerank-forward 重界定据实记录）/ ADR-014 D1-D5（第三十次激活）

## 1. Background

task-39.1（console_data_plane proto add-only `SearchRequest.hybrid=8` + `SearchResultItem.hybrid_score=17` + `buf generate` + 数据面 hybrid dispatch）+ task-39.2（Go `contractv1.SearchRequest.Hybrid` + `SearchResult.HybridScore` + `handleSearch` `?hybrid` OR-merge + `grpcclient` 转发/映射 + rerank provenance 可见性）合入后，对外 console-api 已贯通 hybrid REST 转发 + rerank `reason` provenance 可见。本 task 收口 v0.32.0：smoke 端到端断言 + release docs + ADR ratify / Amendment + roadmap / adapter + defer marker 更新 + phase Done。

- **B1 smoke 端到端断言（v28→v29 + step [48/48]）**：`scripts/console_smoke.sh` 当前 banner v28 + step [47/47]（task-38.3，`:1108`）；本 task banner v28→v29 + 新 step [48/48]（staging dir 顺位 offset，承 `cf-v29-cfg`/`cf-v30-cfg` 序列）——端到端断言对外 `POST /v1/search` `?hybrid=true`（或 body `{"hybrid":true}`）抵达 core hybrid 路径 → 响应 `retrieval_method="hybrid"` + `hybrid_score`（镜像 step [29] 的 trace / 路径断言风格），+ `CONTEXTFORGE_RERANKER_PROVIDER=identity` 时 rerank `reason` marker（`reranked:identity`）在对外 REST 响应可见。
- **B2 smoke_syntax_test.go TestTask393**：新 `TestTask393_SmokeV29ConsoleApiSignalForwardStep`（镜像 `TestTask383`）断言 [48/48] + no-regression（denominators [37/37]..[47/47] 不溯改，ADR-014 D5）+ `bash -n`；`:705-706` 的 `[SPEC-DEFER:phase-future.console-api-hybrid-forward]` 注据实更新。
- **B3 release docs**：`docs/releases/v0.32.0-evidence.md` + `v0.32.0-artifacts.md`（hybrid REST 转发 + rerank provenance 可见性证据；tag SHA / run id / digest / tlog 为 angle-bracket `<backfill>` marker 直至 post-tag-push）+ `README.md` v0.32 段（替换 `:350`「console-api `?hybrid=true` / `?rerank=true` REST forward follows ... in a later release」措辞为 fulfilled / 重界定口径：`?hybrid` 已贯通对外；rerank 保持 env 驱动、provenance 经 `reason` 对外可见、`?rerank` per-request superseded）+ `RELEASE_NOTES.md` v0.32.0 段。
- **B4 ADR ratify + Amendment**：ADR-044 Proposed→Accepted（逐 D 据真实验证 ratify）+ ADR-025 add-only Phase-39 Amendment（标 `console-api-hybrid-forward` fulfilled，不溯改 D-body / `## Ratification`，ADR-014 D5）+ ADR-043 add-only Phase-39 Amendment（标 `console-api-rerank-forward` 重界定为 provenance 可见性 fulfilled + `?rerank` per-request superseded by D3，不溯改 D-body）+ roadmap §3.21/§4 add-only + s2v-adapter rows。
- **B5 defer marker 更新**：`scripts/console_smoke.sh:49-50` + `internal/cli/smoke_syntax_test.go:705-706` + `README.md:350` 的 `[SPEC-DEFER:phase-future.console-api-hybrid-forward]` 注据实更新为 fulfilled（hybrid 维度）+ rerank-forward 重界定口径（provenance 可见、per-request superseded）。

本 task 为 docs + smoke 收口 🟢 可验证（smoke `bash -n` + `TestTask393` + spec-lint）；0 backend 改动；tag/run/digest 真实数据据本机真实发版回填（ADR-013 不预填，发版须 ADR-012 用户授权）。

## 2. Goal

(1) **B1 smoke v29[48/48]**：`scripts/console_smoke.sh` banner v28→v29 + v29 changelog block + 新 step [48/48]（staging dir 顺位 offset；端到端断言 `?hybrid=true` → `retrieval_method="hybrid"` / `hybrid_score`，+ `CONTEXTFORGE_RERANKER_PROVIDER=identity` → rerank `reason` marker 对外 REST 可见）。(2) **B2 TestTask393**：`internal/cli/smoke_syntax_test.go` 新 `TestTask393_SmokeV29ConsoleApiSignalForwardStep`（镜像 `TestTask383`）断言 [48/48] + no-regression（denominators [37/37]..[47/47] 不溯改）+ `bash -n`。(3) **B3 release docs**：`docs/releases/v0.32.0-evidence.md` + `v0.32.0-artifacts.md`（hybrid REST 转发 + rerank provenance 可见证据；tag/run/digest/tlog `<backfill>` marker）+ `README.md` v0.32 段（替换 `:350` 措辞）+ `RELEASE_NOTES.md` v0.32.0 段。(4) **B4 ADR ratify + Amendment**：ADR-044 Proposed→Accepted（逐 D ratify）+ ADR-025 add-only Phase-39 Amendment（标 `console-api-hybrid-forward` fulfilled）+ ADR-043 add-only Phase-39 Amendment（标 `console-api-rerank-forward` 重界定 fulfilled + `?rerank` per-request superseded）+ roadmap §3.21/§4 add-only + s2v-adapter rows。(5) **B5 defer marker 更新**：`console_smoke.sh:49-50` + `smoke_syntax_test.go:705-706` + `README.md:350` defer 注据实更新。(6) **B6 phase 闭合**：`docs/specs/phases/phase-39-*.md` Status Draft→Done + §6 AC 勾选（逐维如实）。

pass bar：smoke `bash -n` 通过 + banner v29 + step [48/48]（🟢）；`TestTask393` 断言 [48/48] + no-regression [37/37]..[47/47] 不溯改（🟢）；release docs 真实证据（hybrid 贯通 + rerank provenance 可见）+ tag/run/digest `<backfill>` 待回填（ADR-013 不预填）；ADR-044 逐 D ratify + ADR-025/043 add-only Amendment（不溯改 D-body，ADR-014 D5）；roadmap §3.21/§4 + s2v-adapter add-only；defer marker 据实更新；phase §6 闭合；ADR-014 D2 lint 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `scripts/console_smoke.sh`——banner v28→v29 + v29 changelog block（Phase 39 console-api-retrieval-signal-forward 一句）+ 新 step [48/48]（staging dir 顺位 offset，承 `cf-v29-cfg`/`cf-v30-cfg` 序列）；step 内容：`init` staging config → 对外 `POST /v1/search` `?hybrid=true`（或 body）断言响应 `retrieval_method="hybrid"` + `hybrid_score`（镜像 step [29] 路径 / trace 断言风格）+ `CONTEXTFORGE_RERANKER_PROVIDER=identity` 时断言响应 `reason` 含 rerank marker（`reranked:identity`）；`:49-50` defer 注据实更新。
- 改 `internal/cli/smoke_syntax_test.go`——新 `TestTask393_SmokeV29ConsoleApiSignalForwardStep`（镜像 `TestTask383`）：断言 banner v29 + step [48/48] 存在 + no-regression（denominators [37/37]..[47/47] 字面不溯改，ADR-014 D5）+ `bash -n scripts/console_smoke.sh` 语法通过；`:705-706` defer 注据实更新。
- 新增 `docs/releases/v0.32.0-evidence.md`——hybrid REST 转发证据（`?hybrid=true` → `retrieval_method="hybrid"` + `hybrid_score`，本机 smoke / 测试实证）+ rerank provenance 可见证据（`reason` 对外 REST 可见，reranker env opt-in）+ rerank-forward 重界定记录（per-request superseded by ADR-043 D3）+ tag SHA / run id / digest / tlog `<backfill>` marker。
- 新增 `docs/releases/v0.32.0-artifacts.md`——artifacts 清单（ghcr digest / cosign tlog / SBOM `<backfill>` marker）+ proto add-only 字段记录（`SearchRequest.hybrid=8` / `SearchResultItem.hybrid_score=17`，既有字段号冻结）。
- 改 `README.md`——v0.32 段 + 替换 `:350`「console-api `?hybrid=true` / `?rerank=true` REST forward follows the Phase 20 `?semantic` pattern in a later release」为 fulfilled / 重界定口径（`?hybrid=true` 已贯通对外 REST；rerank 保持 env 驱动、provenance 经 `reason` 对外可见、`?rerank` per-request superseded by env-driven model）。
- 改 `RELEASE_NOTES.md`——v0.32.0 段（console-api-retrieval-signal-forward：hybrid REST 转发 + rerank provenance 可见性 + rerank-forward 重界定）。
- 改 `docs/decisions/adr-044-console-api-retrieval-signal-forward.md`——Status Proposed→Accepted + `## Ratification（v0.32.0 / task-39.3）` 逐 D 据真实验证（D1 proto + 数据面 dispatch / D2 Go 转发 + rerank 可见 / D3 rerank-forward 重界定 / D4 默认字节等价 + 0 dep）。
- add-only Amendment（非正文改，ADR-014 D5）：`docs/decisions/adr-025-hybrid-scoring-fusion.md`（add-only `## Amendment (Phase 39 / v0.32.0)` 标其 `[SPEC-DEFER:phase-future.console-api-hybrid-forward]` fulfilled，不溯改 D-body / `## Ratification`）；`docs/decisions/adr-043-embedding-remote-reranker-live.md`（add-only `## Amendment (Phase 39 / v0.32.0)` 标其 `console-api-rerank-forward` 重界定为 provenance 可见性 fulfilled + `?rerank` per-request superseded by D3，不溯改 D-body）。
- 改 `docs/roadmap.md §3.21/§4`——add-only Phase 39 行 + `console-api-hybrid-forward` progressed→fulfilled + `console-api-rerank-forward` 重界定记录（provenance 可见 / per-request superseded）。
- 改 `docs/s2v-adapter.md`——Phase 39 行 + Task 39.1/39.2/39.3 行 + ADR-044 行 + BDD 行。
- 改 `docs/specs/phases/phase-39-console-api-retrieval-signal-forward.md`——Status Draft→Done + §6 AC 勾选（逐维如实）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- 真实 tag v0.32.0 + ghcr release（push tag / release run / digest / cosign tlog）——须 ADR-012 用户显式授权后执行；本 task 仅备 release docs（tag/run/digest `<backfill>` marker），真实数据发版后回填 [SPEC-OWNER:phase-future.release-backfill]。
- `?rerank=true` per-request 转发实现 [SPEC-DEFER:phase-future.console-api-rerank-forward]——据 ADR-044 D3 superseded（reranker 保持 env 驱动），本 task 仅在 ADR-043 Amendment + roadmap 据实记录其重界定，不实现。
- Console UI hybrid / rerank explain 面板 [SPEC-OWNER:phase-future.console-semantic-explain]——跨仓库 Console 领域。
- 大语料 hybrid / rerank 对外 REST 召回质量基准 [SPEC-DEFER:phase-future.vector-large-corpus-perf]——本 phase 为信号贯通 + provenance 可见性 wiring 断言，非大基准质量。

## 4. Actors

- 主 agent（ADR-012 自治）；tajiaoyezi（v0.32.0 release 授权方，ADR-012）
- `scripts/console_smoke.sh`（banner v28→v29 + step [48/48] + `:49-50` defer 注更新）
- `internal/cli/smoke_syntax_test.go`（`TestTask393` + `:705-706` defer 注更新）
- `docs/releases/v0.32.0-{evidence,artifacts}.md`（新增，release docs）
- `README.md`（v0.32 段 + `:350` 措辞替换）+ `RELEASE_NOTES.md`（v0.32.0 段）
- `docs/decisions/adr-044-*.md`（Proposed→Accepted）+ `adr-025-*.md` / `adr-043-*.md`（add-only Phase-39 Amendment）
- `docs/roadmap.md`（§3.21/§4 add-only）+ `docs/s2v-adapter.md`（Phase 39 rows）+ `docs/specs/phases/phase-39-*.md`（Status Done）

## 5. Behavior Contract

### 5.1 Required Reading

- `scripts/console_smoke.sh:40-60`（banner + v28 changelog block + `:49-50` `[SPEC-DEFER:phase-future.console-api-hybrid-forward]` 注）+ `:1100-1135`（step [47/47] `:1108`（task-38.3）+ staging dir `cf-v29-cfg` `:1102` / `cf-v30-cfg` `:1130` 序列——step [48/48] 顺位 offset 范本）+ 既有 step [29]（semantic dispatch trace 断言风格——hybrid step 镜像）
- `internal/cli/smoke_syntax_test.go`（`TestTask383_SmokeV28RemoteRerankerLiveStep` 范本——`TestTask393` 镜像；`:705-706` 同 defer 注）
- `docs/releases/v0.31.0-evidence.md` + `v0.31.0-artifacts.md`（release docs 模板——v0.32.0 镜像，真实数 + `<backfill>` marker）
- `README.md:350`（console-api `?hybrid=true` / `?rerank=true`「in a later release」措辞——本 task 替换为 fulfilled / 重界定口径）
- `docs/decisions/adr-044-console-api-retrieval-signal-forward.md §Ratification`（骨架——本 task 逐 D ratify）+ `adr-025-hybrid-scoring-fusion.md`（`console-api-hybrid-forward` defer 记录于 §Follow-ups `:56`——add-only Amendment 标 fulfilled）+ `adr-043-embedding-remote-reranker-live.md §D3`（reranker env 驱动——add-only Amendment 标 rerank-forward 重界定 + per-request superseded）
- `docs/roadmap.md §3.21`（Phase 39——本 task add-only 行）+ `§4`（backlog——`console-api-hybrid-forward` fulfilled / `console-api-rerank-forward` 重界定）+ `docs/s2v-adapter.md`（Phase / Task / ADR / BDD rows）
- `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D5 不溯改历史——denominators [37/37]..[47/47] 字面不动、ADR-025/043 D-body 不溯改；第三十次激活）+ `adr-013`（tag/run/digest `<backfill>` 不预填 / rerank-forward 重界定据实记录）

### 5.2 关键设计 — v0.32.0 收口（smoke 端到端断言 / release docs 真实数 + backfill / ADR ratify + add-only Amendment / defer marker 更新 / no-regression）

- **B1 smoke v29[48/48]（端到端断言，镜像 step [29] 风格）**：banner v28→v29 + v29 changelog block；新 step [48/48] staging dir 顺位 offset（承 `cf-v29-cfg`/`cf-v30-cfg` 序列，下一顺位）；step 内容——`init` staging config（含 `[reranker]` provider=identity 或经 env 设 `CONTEXTFORGE_RERANKER_PROVIDER=identity`）→ 对外 `POST /v1/search` `?hybrid=true`（或 body `{"hybrid":true}`）→ 断言响应 `result.retrieval_method == "hybrid"`（或 trace `candidate_generation_steps` 含 hybrid/RRF 标记，依实际响应 shape，镜像 step [29] 的断言方式）+ `hybrid_score` 字段存在 → `CONTEXTFORGE_RERANKER_PROVIDER=identity` 时断言 `result.reason` 含 `reranked` marker（rerank provenance 对外 REST 可见）。**transient index 为空时**据实记 wiring shape 断言（镜像 step [30] 对空 index 的处理，ADR-013 不预判召回阈值）。
- **B2 TestTask393（no-regression）**：`internal/cli/smoke_syntax_test.go` 新 `TestTask393_SmokeV29ConsoleApiSignalForwardStep`（镜像 `TestTask383`）——断言 banner `v29` + step `[48/48]` 字面存在 + no-regression（denominators `[37/37]`..`[47/47]` 字面不溯改，ADR-014 D5）+ `bash -n scripts/console_smoke.sh` 语法通过。
- **B3 release docs（真实数 + backfill）**：`docs/releases/v0.32.0-evidence.md` + `v0.32.0-artifacts.md`——hybrid REST 转发证据（`?hybrid=true` → `retrieval_method="hybrid"` + `hybrid_score`，本机 smoke / `go test` 实证）+ rerank provenance 可见证据（`reason` 对外 REST 可见）+ rerank-forward 重界定记录（`?rerank` per-request superseded by ADR-043 D3）；tag SHA / run id / ghcr digest / cosign tlog 为 angle-bracket `<backfill>` marker（ADR-013 不预填，发版后回填）。`README.md` v0.32 段 + `:350` 措辞替换；`RELEASE_NOTES.md` v0.32.0 段。
- **B4 ADR-044 ratify + ADR-025/043 add-only Amendment**：ADR-044 Status Proposed→Accepted + `## Ratification（v0.32.0 / task-39.3）` 逐 D 据真实验证（D1 proto + 数据面 dispatch 落地、D2 Go 转发 + rerank 可见落地、D3 rerank-forward 重界定据实、D4 默认字节等价 + 0 dep）；`adr-025` add-only `## Amendment (Phase 39 / v0.32.0)`（标 `[SPEC-DEFER:phase-future.console-api-hybrid-forward]` fulfilled：proto + 数据面 dispatch + Go 转发贯通对外 REST，**不溯改 ADR-025 D-body / `## Ratification`**）；`adr-043` add-only `## Amendment (Phase 39 / v0.32.0)`（标 `console-api-rerank-forward` 按 provenance-visibility 口径 fulfilled + `?rerank` per-request superseded by D3 env-driven，**不溯改 ADR-043 D-body**）。
- **B5 defer marker 更新（据实）**：`scripts/console_smoke.sh:49-50` + `internal/cli/smoke_syntax_test.go:705-706` + `README.md:350` 的 `[SPEC-DEFER:phase-future.console-api-hybrid-forward]` 注据实更新——hybrid 维度 fulfilled（对外 REST 已贯通）；rerank-forward 重界定口径（rerank provenance 经 `reason` 对外可见、`?rerank` per-request superseded by env-driven model）。
- **B6 roadmap / adapter / phase 闭合**：`docs/roadmap.md §3.21`（Phase 39 行 add-only）+ `§4`（`console-api-hybrid-forward` progressed→fulfilled + `console-api-rerank-forward` 重界定记录）；`docs/s2v-adapter.md`（Phase 39 + Task 39.1/39.2/39.3 + ADR-044 + BDD rows）；`docs/specs/phases/phase-39-*.md` Status Draft→Done + §6 AC 逐维勾选（如实）。
- **no-regression（ADR-014 D5）**：既有 smoke step [1]..[47] + denominators [37/37]..[47/47] 字面不溯改；ADR-025 / ADR-043 D-body / `## Ratification` 不溯改（仅 add-only Amendment）；既有 release docs（v0.13.0..v0.31.0）不动。

### 5.3 不变量

- no-regression（ADR-014 D5）：smoke 既有 step [1]..[47] 不退化、denominators [37/37]..[47/47] 字面不溯改；ADR-025 / ADR-043 D-body + `## Ratification` 不溯改（仅 add-only Phase-39 Amendment）；历史 Phase 1-38 spec / ADR / release docs 不动。
- 真实数据据实（ADR-013）：tag SHA / run id / ghcr digest / cosign tlog 为 `<backfill>` marker 直至 post-tag-push 真实回填（不预填）；hybrid 贯通 + rerank provenance 可见证据据本机真实 smoke / `go test` 实证；rerank-forward 重界定据实记录（per-request superseded、不夸大为已实现 per-request）。
- 发版授权（ADR-012）：v0.32.0 tag / ghcr release 须 tajiaoyezi 显式授权后执行；本 task 只备 release docs（`<backfill>` marker），不自行发版。
- 0 backend 改动：本 task 限 smoke / docs / ADR / roadmap / adapter；不改 proto / 数据面 / Go console-api 代码（那些属 task-39.1 / 39.2）。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [ ] **AC1**（smoke v29[48/48] + TestTask393 no-regression 🟢）: `scripts/console_smoke.sh` banner v28→v29 + 新 step [48/48]（staging 顺位 offset；端到端断言 `?hybrid=true` → `retrieval_method="hybrid"` / `hybrid_score` + `CONTEXTFORGE_RERANKER_PROVIDER=identity` → rerank `reason` marker 对外 REST 可见）；`TestTask393_SmokeV29ConsoleApiSignalForwardStep` 断言 [48/48] + no-regression（denominators [37/37]..[47/47] 不溯改）+ `bash -n` 通过 — verified by **TEST-39.3.1**（smoke v29[48/48] + TestTask393）
- [ ] **AC2**（release docs + ADR ratify + Amendment + roadmap/adapter + defer marker + phase 闭合 🟢）: `docs/releases/v0.32.0-{evidence,artifacts}.md`（hybrid 贯通 + rerank provenance 可见证据 + tag/run/digest `<backfill>` marker）+ README v0.32 段（`:350` 措辞替换）+ RELEASE_NOTES v0.32.0 段；ADR-044 Proposed→Accepted（逐 D ratify）+ ADR-025 add-only Phase-39 Amendment（标 console-api-hybrid-forward fulfilled，不溯改 D-body）+ ADR-043 add-only Phase-39 Amendment（标 console-api-rerank-forward 重界定 fulfilled + `?rerank` per-request superseded，不溯改 D-body）+ roadmap §3.21/§4 add-only + s2v-adapter rows + defer marker（console_smoke.sh:49-50 / smoke_syntax_test.go:705-706 / README:350）据实更新 + phase §6 闭合 — verified by **TEST-39.3.1**（docs/ADR/roadmap/adapter 一致性人工核 + grep）
- [ ] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-39.3.2**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-39.3.1 | smoke v29[48/48] + TestTask393 + release docs/ADR/roadmap/adapter 一致性：`bash -n scripts/console_smoke.sh` 通过 + banner v29 + step [48/48]（端到端断言 `?hybrid=true` → `retrieval_method="hybrid"` / `hybrid_score` + rerank `reason` 对外可见）+ `TestTask393` 断言 [48/48] + no-regression [37/37]..[47/47] 不溯改；ADR-044 Accepted + ADR-025/043 add-only Phase-39 Amendment（不溯改 D-body）+ roadmap §3.21/§4 + s2v-adapter + defer marker + phase §6 闭合（一致性人工核 + grep） | `scripts/console_smoke.sh` / `internal/cli/smoke_syntax_test.go` / `docs/releases/v0.32.0-*.md` / `docs/decisions/adr-044-*.md` / `adr-025-*.md` / `adr-043-*.md` / `docs/roadmap.md` / `docs/s2v-adapter.md` / `docs/specs/phases/phase-39-*.md` | Draft |
| TEST-39.3.2 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Draft |

## 8. Risks

- **R1（中）smoke step [48/48] denominator 溯改既有 step**：新 step 改 denominators 时误改既有 [37/37]..[47/47]（破 ADR-014 D5 no-regression）。
  - **缓解**：新 step 用顺位 [48/48]，既有 step denominators 字面不动；`TestTask393` 断言 no-regression（[37/37]..[47/47] 字面存在）；TEST-39.3.1 含 no-regression 断言。stop-condition：既有 denominator 变动则 AC1 不标 `[x]`。
- **R2（高）release docs 预填未发版的真实数据（违 ADR-013）**：tag/run/digest/tlog 在发版前填真实值。
  - **缓解**：tag SHA / run id / ghcr digest / cosign tlog 为 angle-bracket `<backfill>` marker 直至 post-tag-push；hybrid 贯通 + rerank provenance 证据据本机真实 smoke / `go test`（已实证可填）；发版（ADR-012 授权）后回填真实 tag/digest/tlog。stop-condition：`<backfill>` marker 在发版前被填真实值则回退。
- **R3（中）ADR-025/043 D-body 被溯改（破 ADR-014 D5）**：标 defer fulfilled / 重界定时误改 ADR-025/043 正文 / `## Ratification`。
  - **缓解**：ADR-025/043 仅 add-only `## Amendment (Phase 39 / v0.32.0)`（追加段，不改 D-body / `## Ratification`）；TEST-39.3.1 含「不溯改 D-body」人工核。
- **R4（中）rerank-forward 重界定被记为「已实现 per-request」（违 ADR-013）**：defer marker / ADR Amendment / roadmap 误把 `?rerank` per-request 记为已交付。
  - **缓解**：据实记 `console-api-rerank-forward` per-request 控制 **superseded by ADR-043 D3（env 驱动）、不实现**，改交付 rerank provenance 可见性；ADR-043 Amendment + roadmap + defer marker 据实分级（fulfilled = provenance 可见、superseded = per-request）；TEST-39.3.1 含据实口径人工核。
- **R5（低）README :350 措辞替换遗漏 / 不一致**：`:350` 旧「in a later release」措辞未替换或与新口径不一致。
  - **缓解**：替换 `:350` 为 fulfilled / 重界定口径（`?hybrid` 已贯通 + rerank env 驱动 provenance 可见 + `?rerank` per-request superseded）；grep `in a later release` / `console-api ?hybrid` 核无残留旧措辞。

## 9. Verification Plan

```bash
# 1. AC1 — smoke 语法 + banner v29 + step [48/48] + no-regression
bash -n scripts/console_smoke.sh
go test ./internal/cli/ -run TestTask393

# 2. AC1 — smoke 全量不退化（既有 step [1]..[47] + denominators 不溯改）
go test ./internal/cli/ -run TestTask

# 3. AC2 — release docs / ADR / roadmap / adapter 一致性（grep backfill marker + defer marker 据实 + 旧措辞无残留）
grep -rn "backfill" docs/releases/v0.32.0-evidence.md docs/releases/v0.32.0-artifacts.md
grep -rn "console-api-hybrid-forward" docs/ scripts/ internal/ README.md   # 据实更新为 fulfilled / 重界定
grep -n "in a later release" README.md   # 期望 0（:350 措辞已替换）

# 4. 不退化（全量 Go + Rust 默认 build；hybrid 贯通端到端经 39.1/39.2 已绿）
go test ./...
cargo test --workspace

# 5. AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.closeout-v0.32.0-defer-note]：本 task 仅交付 v0.32.0 收口（smoke v29[48/48] 端到端断言 + release docs + ADR-044 ratify + ADR-025/043 add-only Phase-39 Amendment + roadmap/adapter + defer marker 更新 + phase 闭合，🟢 docs + smoke 可验证）；真实 tag v0.32.0 + ghcr release（push tag / release run / digest / cosign tlog）须 ADR-012 用户显式授权后执行 [SPEC-OWNER:phase-future.release-backfill]（tag/run/digest `<backfill>` marker 待回填，不预填，ADR-013）；`?rerank=true` per-request 转发 [SPEC-DEFER:phase-future.console-api-rerank-forward]（据 ADR-044 D3 superseded by env-driven、不实现，本 task 仅据实记录重界定）、Console UI hybrid explain 面板 [SPEC-OWNER:phase-future.console-semantic-explain]、大语料 hybrid 召回质量基准 [SPEC-DEFER:phase-future.vector-large-corpus-perf] 均不在本 task 范围。no-regression（denominators [37/37]..[47/47] + ADR-025/043 D-body 不溯改，ADR-014 D5）。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Draft（实施 + §9 真实验证后置 Done，逐条粘 PASS 摘要：`bash -n` / `go test ./internal/cli/ -run TestTask393` / `go test ./...` / `cargo test --workspace` / grep 一致性 / `bash scripts/spec_drift_lint.sh --touched origin/master` 0 命中；tag/run/digest 发版后回填；未跑不勾 AC）

- **§9 Verification 实证**（实施后回填）：本机真实跑 §9 全部命令、逐条粘 PASS 摘要。
- **实际改动文件**（实施后回填）：`scripts/console_smoke.sh`（banner v29 + step [48/48] + :49-50 defer 注）/ `internal/cli/smoke_syntax_test.go`（TestTask393 + :705-706 defer 注）/ `docs/releases/v0.32.0-{evidence,artifacts}.md`（新增）/ `README.md`（v0.32 段 + :350 措辞）/ `RELEASE_NOTES.md`（v0.32.0 段）/ `docs/decisions/adr-044-*.md`（Accepted）/ `adr-025-*.md`（add-only Amendment）/ `adr-043-*.md`（add-only Amendment）/ `docs/roadmap.md`（§3.21/§4）/ `docs/s2v-adapter.md`（Phase 39 rows）/ `docs/specs/phases/phase-39-*.md`（Status Done）。
- **0 backend 改动 / no-regression**：本 task 限 smoke / docs / ADR / roadmap / adapter；既有 step [1]..[47] + denominators [37/37]..[47/47] 不溯改 + ADR-025/043 D-body 不溯改（ADR-014 D5）；tag/run/digest `<backfill>` 待回填（ADR-013）；v0.32.0 发版须 ADR-012 用户授权。
- **ADR**：本 task ratify ADR-044 Proposed→Accepted + ADR-025 add-only Phase-39 Amendment（console-api-hybrid-forward fulfilled）+ ADR-043 add-only Phase-39 Amendment（console-api-rerank-forward 重界定 fulfilled + `?rerank` per-request superseded）；ADR-014 第三十次激活全 D 据真实验证 ratify。
- **rerank-forward 重界定据实（ADR-013）**：`console-api-rerank-forward` per-request 控制 superseded by ADR-043 D3（env 驱动）、不实现；改交付 rerank provenance 可见性——据实记录、不夸大为已实现 per-request。
